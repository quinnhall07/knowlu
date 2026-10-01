//! The vault-writing event state outside `approvals.rs`: the ever-written set, the series carry,
//! D9's rebuild and `answered_series`.
//!
//! Spec `2026-09-29-events-design.md` §4.5 and §6.1. [`answered_series`] is the one definition of
//! an answered `event-accept` series. The emitter's never-ask-twice, the carry and `judge_roster`
//! all call it and none restates it, so one `rank` cannot both carry a series and ask about it.
//!
//! [`run`] is the one entry point `rank` calls: D9's rebuild of `declined` lines from archived
//! `rejected` cards, then the carry over every fetched instance its series' card did not list.
//!
//! **A run built from the roster carries nothing, deliberately.** When every feed fails, `rank`
//! passes `eventroster::read_roster`'s events. Their `series_uid` is their own uid and their span
//! is the roster's lossy read-back, so they match no answered series, and the carry skips any
//! `source: "roster"` event besides. The next run that fetches the feeds carries as usual. The
//! carry reads series only from fetched events, never from the roster, the ledger or any other
//! store. This is intended: do not "fix" it.

use std::collections::{BTreeMap, BTreeSet};
use std::path::Path;

use jiff::civil::Date;
use serde_yaml_ng::Mapping;

use crate::commitments::{create_confirmed, Level};
use crate::eventaccept::{commitment_for, Instance};
use crate::eventledger::{record_declined, LedgerEntry};
use crate::events::DiscoveredEvent;
use crate::journal::Journal;
use crate::models::split_frontmatter;
use crate::write::WriteContext;
use crate::{ids, pystr, yaml};

/// The card kind whose answers [`answered_series`] reads.
const EVENT_ACCEPT: &str = "event-accept";

/// The `source` `eventroster::read_roster` gives every event it reads back (spec §2).
const ROSTER_SOURCE: &str = "roster";

/// One series an archived `event-accept` card answered.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SeriesAccept {
    /// `executed` answers accept (`true`); `rejected` answers decline (`false`).
    pub accepted: bool,
    /// The answering card's file name in `archive/`.
    pub file: String,
    /// The answering card's `id:`, or empty when it has none or [`ids::is_id`] refuses it. The
    /// carry's ledger line names the card by it (the plan's P15, `from:`).
    pub id: String,
    /// The card's `verdict:`, stripped: `obligation` or `opportunity` on every card the emitter
    /// files. The accept carry's level reads it.
    pub verdict: String,
    /// The uids the card's `events:` lists. The settlement answered these, so the carry never does.
    pub listed: BTreeSet<String>,
}

/// Every series an `event-accept` card answered, keyed by `series_uid` (spec §6.1).
///
/// The edge rules follow `eventemit::settled_series`:
/// - it reads `archive/` only, through `approvals::sorted_md`, and only `type: approval`,
///   `kind: event-accept` cards: an `event-check` card is not counted (accepted `event-check`
///   series reach the carry through their own reader, never through this one);
/// - `executed` answers accept and `rejected` answers decline; `expired` or any other status
///   answers nothing (ruling G1), so it shadows no other card;
/// - a card with a missing or empty `series_uid` answers no series; the uid rule (§4.1) closes
///   that card's own instances;
/// - if two cards answer one series, the lowest file name wins.
///
/// It reads no ledger. An unreadable card answers nothing. The `id:` is read as `ids::build_index`
/// reads it (unstripped, then [`ids::is_id`]), so the carry never names a card by an id the index
/// would refuse.
pub fn answered_series(vault: &Path) -> BTreeMap<String, SeriesAccept> {
    let mut answered: BTreeMap<String, SeriesAccept> = BTreeMap::new();
    for path in crate::approvals::sorted_md(&vault.join("archive")) {
        let Ok(text) = pystr::read_text(&path) else { continue };
        let Ok((meta, _)) = split_frontmatter(&text) else { continue };
        if card_text(&meta, "type") != "approval" || card_text(&meta, "kind") != EVENT_ACCEPT {
            continue;
        }
        let accepted = match card_text(&meta, "status").as_str() {
            "executed" => true,
            "rejected" => false,
            _ => continue,
        };
        let series = card_text(&meta, "series_uid");
        if series.is_empty() || answered.contains_key(&series) {
            continue;
        }
        let file = path.file_name().map(|n| n.to_string_lossy().into_owned()).unwrap_or_default();
        let id = yaml::opt_text(yaml::get(&meta, "id")).filter(|id| ids::is_id(id)).unwrap_or_default();
        let verdict = card_text(&meta, "verdict");
        let listed = crate::eventemit::card_event_uids(&meta).into_iter().collect();
        answered.insert(series, SeriesAccept { accepted, file, id, verdict, listed });
    }
    answered
}

/// The ever-written set (spec §5.3): the `source_uid` of every note in `commitments/`, `tasks/` and
/// `archive/`. A live note means "already written"; an archived one means "written, then deleted by
/// the student" (`write::delete` moves it there with its frontmatter intact). Both mean "never
/// write again", so a uid (or `register:<uid>`) in this set is never written by the settlement or
/// the carry.
///
/// Notes with `type: approval` are left out, and `approvals/` is not read: an event card carries
/// its primary's uid as `source_uid`, and counting it would stop that instance from ever being
/// written. Unreadable notes are skipped.
pub fn ever_written(vault: &Path) -> BTreeSet<String> {
    let mut written = BTreeSet::new();
    for folder in [crate::commitments::FOLDER, "tasks", "archive"] {
        for path in crate::approvals::sorted_md(&vault.join(folder)) {
            let Ok(text) = pystr::read_text(&path) else { continue };
            let Ok((meta, _)) = split_frontmatter(&text) else { continue };
            if card_text(&meta, "type") == "approval" {
                continue;
            }
            let uid = card_text(&meta, "source_uid");
            if !uid.is_empty() {
                written.insert(uid);
            }
        }
    }
    written
}

/// D9's rebuild, then the series carry (spec §4.5, D4, D9): the one entry point `rank` calls.
/// Returns `(ledger lines written, warnings)`, and updates `ledger` as `load_ledger` would read
/// every new line.
///
/// 1. **D9.** Each uid a `rejected` `event-accept` card in `archive/` lists gets a `declined` line
///    unless it has one: the archived card is the record of the answer, and it syncs where the
///    ledger does not. An `executed` card needs nothing here.
/// 2. **The carry**, over each fetched instance in `events` whose series a card answered
///    ([`answered_series`]) and whose uid that card did not list, in `(start, uid)` order, each uid
///    once:
///    - a declined series gives the instance a `declined` line unless it has one;
///    - an accepted series books an instance starting today or later, one carried date at a time
///      (`book`): the commitment `eventaccept::commitment_for` gives at the level the card's
///      `verdict:` names, through `commitments::create_confirmed` (actor `agent:commitments`,
///      journal first). A lane-shaped instance (all-day, multi-day, zero-length) has no
///      commitment, and its answer stands with nothing to write. No register task is written (P9):
///      a series has one, for its primary. No ledger line is written for an accepted series.
///      It never books a uid in the [`ever_written`] set, so a note the student deleted stays
///      deleted, nor an instance the student turned down on its own (`turned_down`).
///
/// A `source: "roster"` event is never carried (the module doc says why).
///
/// A write failure is a warning and the next `rank` retries; nothing here panics. Every carried
/// note is a journal `create` in this run, which is what lists it in the read model's delta.
pub fn run(
    vault: &Path,
    events: &[DiscoveredEvent],
    ledger: &mut BTreeMap<String, LedgerEntry>,
    today: Date,
    ctx: &WriteContext,
    journal: &mut Journal,
) -> (usize, Vec<String>) {
    let mut warnings = Vec::new();
    let mut lines = rebuild_declines(vault, ledger, today, &mut warnings);
    let answered = answered_series(vault);
    let accepted = accepted_series(&answered, &mut warnings);
    let written = ever_written(vault);
    let mut ordered: Vec<&DiscoveredEvent> = events.iter().filter(|e| e.source != ROSTER_SOURCE).collect();
    ordered.sort_by(|a, b| (a.start(), &a.uid).cmp(&(b.start(), &b.uid)));
    let mut seen: BTreeSet<&str> = BTreeSet::new();
    for event in ordered {
        if !seen.insert(event.uid.as_str()) {
            continue;
        }
        if let Some(series) = answered.get(&event.series_uid).filter(|s| !s.accepted) {
            if !series.listed.contains(&event.uid) && decline(vault, &event.uid, ledger, today, &mut warnings) {
                lines += 1;
            }
            continue;
        }
        let Some(series) = accepted.get(&event.series_uid) else { continue };
        if series.listed.contains(&event.uid) || event.start().date() < today {
            continue;
        }
        book(vault, event, series.level, &written, ledger, today, ctx, journal, &mut warnings);
    }
    (lines, warnings)
}

/// D9: a `declined` line for each uid a `rejected` `event-accept` card in `archive/` lists, unless
/// the ledger has one. Every rejected card counts, whatever its `series_uid`, since the lines
/// decline the card's own instances: cards in file-name order, uids in `events:` order. Returns
/// the lines written.
fn rebuild_declines(
    vault: &Path,
    ledger: &mut BTreeMap<String, LedgerEntry>,
    today: Date,
    warnings: &mut Vec<String>,
) -> usize {
    let mut lines = 0;
    for path in crate::approvals::sorted_md(&vault.join("archive")) {
        let Ok(text) = pystr::read_text(&path) else { continue };
        let Ok((meta, _)) = split_frontmatter(&text) else { continue };
        let rejected = card_text(&meta, "status") == "rejected";
        if card_text(&meta, "type") != "approval" || card_text(&meta, "kind") != EVENT_ACCEPT || !rejected {
            continue;
        }
        for uid in crate::eventemit::card_event_uids(&meta) {
            if decline(vault, &uid, ledger, today, warnings) {
                lines += 1;
            }
        }
    }
    lines
}

/// A `declined` line for `uid` unless the ledger has one, then the map updated as `load_ledger`
/// reads that line. `true` when a line was written. A uid the ledger cannot read back gets no line
/// and a warning: its line would never read as declined, so it would be written on every `rank`.
fn decline(
    vault: &Path,
    uid: &str,
    ledger: &mut BTreeMap<String, LedgerEntry>,
    today: Date,
    warnings: &mut Vec<String>,
) -> bool {
    if ledger.get(uid).is_some_and(|entry| entry.declined) {
        return false;
    }
    if !ledger_reads(uid) {
        warnings.push(format!("carry: no declined line for {uid:?}: the ledger cannot read it back"));
        return false;
    }
    if let Err(err) = record_declined(vault, uid, today) {
        warnings.push(format!("carry: no declined line for {uid:?} ({err})"));
        return false;
    }
    ledger.entry(uid.to_string()).or_insert_with(|| LedgerEntry::new(uid)).declined = true;
    true
}

/// Does `load_ledger` read a marker line for `uid` back? Its uid class is `[A-Za-z0-9_.:@+-]`, and
/// a uid holding `verdict:` breaks its own marker line (`eventledger`'s module doc).
fn ledger_reads(uid: &str) -> bool {
    !uid.is_empty()
        && uid.chars().all(|c| c.is_ascii_alphanumeric() || "_.:@+-".contains(c))
        && !uid.contains("verdict:")
}

/// One accepted series, as the accept carry books it.
struct Accepted {
    /// `Hard` for an obligation, `Soft` for an opportunity (D1).
    level: Level,
    /// The uids the answering card listed: the settlement's, never the carry's.
    listed: BTreeSet<String>,
}

/// The accepted set, keyed by `series_uid`: [`answered_series`]' executed entries, at the level the
/// card's `verdict:` names. The one place it is built. A card with any other verdict books nothing
/// and warns, since Knowlu never guesses a level; only a hand edit makes one.
fn accepted_series(
    answered: &BTreeMap<String, SeriesAccept>,
    warnings: &mut Vec<String>,
) -> BTreeMap<String, Accepted> {
    let mut accepted = BTreeMap::new();
    for (series, answer) in answered.iter().filter(|(_, answer)| answer.accepted) {
        let level = match answer.verdict.as_str() {
            "obligation" => Level::Hard,
            "opportunity" => Level::Soft,
            _ => {
                let file = &answer.file;
                warnings.push(format!(
                    "carry: archive/{file} has no verdict the carry reads; its series is not booked"
                ));
                continue;
            }
        };
        accepted.insert(series.clone(), Accepted { level, listed: answer.listed.clone() });
    }
    accepted
}

/// One carried date's booking step: `event`'s commitment at `level`, unless its uid is in the
/// ever-written set (`written`) or the student turned it down on its own. A lane-shaped event has
/// no commitment. A failed create is a warning, and the next `rank` retries.
#[allow(clippy::too_many_arguments)]
fn book(
    vault: &Path,
    event: &DiscoveredEvent,
    level: Level,
    written: &BTreeSet<String>,
    ledger: &BTreeMap<String, LedgerEntry>,
    today: Date,
    ctx: &WriteContext,
    journal: &mut Journal,
    warnings: &mut Vec<String>,
) {
    if written.contains(&event.uid) || ledger.get(&event.uid).is_some_and(turned_down) {
        return;
    }
    let Some(commitment) = commitment_for(&Instance::from_event(event), level) else { return };
    if let Err(err) = create_confirmed(vault, &commitment, &event.uid, today, ctx, journal) {
        warnings.push(format!("carry: {} not booked ({err})", event.uid));
    }
}

/// Did the student turn this instance down on its own: a `declined` line, or a human `drop`
/// answer? The carry never books one (judge once). A machine's `drop` verdict is no answer: the
/// student's answer for the series covers it.
fn turned_down(entry: &LedgerEntry) -> bool {
    let human = !entry.answered_by.is_empty() && !crate::provenance::is_agent(&entry.answered_by);
    entry.declined || (human && entry.verdict.as_deref() == Some("drop"))
}

/// A card field as text, stripped; empty when absent, null or not a scalar.
fn card_text(meta: &Mapping, key: &str) -> String {
    let raw = yaml::opt_text(yaml::get(meta, key)).unwrap_or_default();
    pystr::strip(&raw).to_string()
}

#[cfg(test)]
mod tests {
    //! T2b.1: `answered_series`' edge rules (spec §6.1), each with an answering card beside it so
    //! a reader that answers nothing fails them.

    use super::*;
    use std::fs;
    use std::path::PathBuf;

    const ID_A: &str = "appr_0123456789";
    const ID_B: &str = "appr_abcdef0123";

    fn tmp(name: &str) -> PathBuf {
        let dir = std::env::temp_dir().join(format!("qo-carry-{}-{}", name, std::process::id()));
        let _ = fs::remove_dir_all(&dir);
        fs::create_dir_all(&dir).unwrap();
        dir
    }

    /// A card written by hand into `folder`. `extra` is whole frontmatter lines, each ending `\n`.
    fn card(vault: &Path, folder: &str, name: &str, kind: &str, status: &str, extra: &str) {
        fs::create_dir_all(vault.join(folder)).unwrap();
        let text = format!(
            "---\n{extra}type: approval\nkind: {kind}\ntitle: \"Required · Weekly meeting · Tue 22 Sep 7–9pm\"\n\
             status: {status}\nverdict: obligation\nsource_uid: \"lx:77:1\"\nevents:\n- \"lx:77:1\"\n\
             proposed_at: 2026-09-20\nfirst_proposed_at: 2026-09-20\nexpires: 2026-09-22\n\
             snooze_until: null\ncreated_by: events\n---\n\nbody\n"
        );
        pystr::write_text(&vault.join(folder).join(name), &text).unwrap();
    }

    /// An `event-accept` card in `archive/` answering `series`; `id` is the YAML text of `id:`.
    fn accept(vault: &Path, name: &str, status: &str, series: &str, id: Option<&str>) {
        let id_line = id.map(|i| format!("id: {i}\n")).unwrap_or_default();
        card(vault, "archive", name, EVENT_ACCEPT, status, &format!("{id_line}series_uid: \"{series}\"\n"));
    }

    fn keys(answered: &BTreeMap<String, SeriesAccept>) -> Vec<&str> {
        answered.keys().map(String::as_str).collect()
    }

    #[test]
    fn an_executed_card_answers_accept_and_a_rejected_one_decline() {
        let vault = tmp("answers");
        accept(&vault, "event-weekly-meeting-2026-09-22.md", "executed", "lx:77", Some(ID_A));
        accept(&vault, "event-film-night-2026-09-23.md", "rejected", "lx:78", Some(ID_B));
        accept(&vault, "event-no-id-2026-09-24.md", "executed", "lx:79", None);
        accept(&vault, "event-bad-id-2026-09-25.md", "rejected", "lx:80", Some("not-an-id"));
        // Read as `ids::build_index` reads an id, unstripped: this one names no note.
        accept(&vault, "event-spaced-id-2026-09-26.md", "executed", "lx:81", Some("\" appr_0123456789\""));

        let answered = answered_series(&vault);
        assert_eq!(keys(&answered), ["lx:77", "lx:78", "lx:79", "lx:80", "lx:81"]);
        let weekly = &answered["lx:77"];
        assert!(weekly.accepted, "executed answers accept");
        assert_eq!(weekly.file, "event-weekly-meeting-2026-09-22.md");
        assert_eq!(weekly.id, ID_A);
        let film = &answered["lx:78"];
        assert!(!film.accepted, "rejected answers decline");
        assert_eq!(film.file, "event-film-night-2026-09-23.md");
        assert_eq!(film.id, ID_B);
        for (series, accepted) in [("lx:79", true), ("lx:80", false), ("lx:81", true)] {
            assert_eq!(answered[series].accepted, accepted, "{series}");
            assert_eq!(answered[series].id, "", "{series}: no id, or one ids::is_id refuses");
        }
    }

    #[test]
    fn an_expired_or_other_status_answers_nothing() {
        let vault = tmp("statuses");
        for (name, status, series) in [
            ("event-a-expired.md", "expired", "lx:1"),
            // A `pending` card moved into `archive/` by hand was never answered.
            ("event-b-pending.md", "pending", "lx:2"),
            ("event-c-approved.md", "approved", "lx:3"),
            ("event-d-failed.md", "failed", "lx:4"),
            ("event-e-empty.md", "\"\"", "lx:5"),
            ("event-f-null.md", "null", "lx:6"),
        ] {
            accept(&vault, name, status, series, Some(ID_A));
        }
        // Ruling G1: an expired card answers nothing, so it does not shadow a later answer.
        accept(&vault, "event-g-expired.md", "expired", "lx:9", Some(ID_A));
        accept(&vault, "event-h-executed.md", "executed", "lx:9", Some(ID_B));
        let broken = "---\ntype: approval\nkind: [event-accept\nstatus: executed\n---\n";
        pystr::write_text(&vault.join("archive").join("event-i-unreadable.md"), broken).unwrap();

        let answered = answered_series(&vault);
        assert_eq!(keys(&answered), ["lx:9"]);
        assert!(answered["lx:9"].accepted);
        assert_eq!(answered["lx:9"].file, "event-h-executed.md");
        assert_eq!(answered["lx:9"].id, ID_B);
    }

    #[test]
    fn a_card_with_a_missing_or_empty_series_uid_answers_no_series() {
        let vault = tmp("no-series");
        card(&vault, "archive", "event-a-missing.md", EVENT_ACCEPT, "executed", &format!("id: {ID_A}\n"));
        let empties = [("event-b-empty.md", "\"\""), ("event-c-blank.md", "\"   \""), ("event-d-null.md", "null")];
        for (name, series) in empties {
            let extra = format!("id: {ID_A}\nseries_uid: {series}\n");
            card(&vault, "archive", name, EVENT_ACCEPT, "rejected", &extra);
        }
        accept(&vault, "event-e.md", "executed", "lx:9", Some(ID_B));

        // No series, and no fallback to the card's `source_uid` (`lx:77:1`): the uid rule (§4.1)
        // closes that card's own instances, not this reader.
        let answered = answered_series(&vault);
        assert_eq!(keys(&answered), ["lx:9"]);
        assert_eq!(answered["lx:9"].file, "event-e.md");
    }

    #[test]
    fn an_event_check_card_is_not_counted() {
        let vault = tmp("event-check");
        // PQ1 (a) leaves this rule unchanged: an executed `event-check` card, even one carrying
        // `instances:`, reaches the carry only through `accepted_check_series` (T2b.2b).
        let instances = "instances:\n- uid: \"lx:1:1\"\n  title: \"Weekly meeting\"\n  \
                         start: \"2026-09-22T19:00\"\n  end: \"2026-09-22T21:00\"\n";
        let carried = format!("series_uid: \"lx:1\"\n{instances}");
        card(&vault, "archive", "event-check-a.md", "event-check", "executed", &carried);
        card(&vault, "archive", "event-check-b.md", "event-check", "rejected", "series_uid: \"lx:2\"\n");
        // A note that is not an approval is no card, whatever its `kind:` says.
        let task = "---\ntype: task\nkind: event-accept\nstatus: executed\nseries_uid: \"lx:3\"\n---\n";
        pystr::write_text(&vault.join("archive").join("event-c-task.md"), task).unwrap();
        accept(&vault, "event-d.md", "executed", "lx:9", Some(ID_A));

        assert_eq!(keys(&answered_series(&vault)), ["lx:9"]);
    }

    #[test]
    fn two_cards_for_one_series_the_lowest_file_name_wins() {
        let vault = tmp("two-cards");
        // Written highest first, so the directory's own order cannot decide.
        accept(&vault, "event-weekly-b.md", "executed", "lx:77", Some(ID_B));
        accept(&vault, "event-weekly-a.md", "rejected", "lx:77", Some(ID_A));
        accept(&vault, "event-film-b.md", "rejected", "lx:78", Some(ID_B));
        accept(&vault, "event-film-a.md", "executed", "lx:78", Some(ID_A));
        // Lowest by bytes: a collision's `-2.md` sorts before `.md`.
        accept(&vault, "event-talk-2026-10-06.md", "executed", "lx:79", Some(ID_A));
        accept(&vault, "event-talk-2026-10-06-2.md", "rejected", "lx:79", Some(ID_B));

        let answered = answered_series(&vault);
        let pick = |series: &str| {
            let a = &answered[series];
            (a.accepted, a.file.clone(), a.id.clone())
        };
        assert_eq!(pick("lx:77"), (false, "event-weekly-a.md".to_string(), ID_A.to_string()));
        assert_eq!(pick("lx:78"), (true, "event-film-a.md".to_string(), ID_A.to_string()));
        assert_eq!(pick("lx:79"), (false, "event-talk-2026-10-06-2.md".to_string(), ID_B.to_string()));
    }

    #[test]
    fn only_archive_is_read() {
        let vault = tmp("archive-only");
        assert!(answered_series(&vault).is_empty(), "a vault with no archive/ answers nothing");
        for (name, status, series) in [("event-a.md", "executed", "lx:1"), ("event-b.md", "rejected", "lx:2")] {
            let extra = format!("id: {ID_A}\nseries_uid: \"{series}\"\n");
            card(&vault, "approvals", name, EVENT_ACCEPT, status, &extra);
        }
        assert!(answered_series(&vault).is_empty(), "a card in approvals/ answers nothing");

        accept(&vault, "event-c.md", "executed", "lx:9", Some(ID_B));
        assert_eq!(keys(&answered_series(&vault)), ["lx:9"]);
    }

    // --- T2b.2: the ever-written set, D9's rebuild and the series carry (spec §4.5, §5.3) -------
    //
    // No test here asserts on an accepted series' ledger lines or on `run`'s line count when an
    // accepted series carries: T2b.5 adds the carry's line, and an assertion may not change later.

    use crate::eventledger::{load_ledger, record_answer, record_verdict};
    use jiff::civil::{date, DateTime};

    /// The run's day, Thu 1 Oct 2026. The series below meet on Tuesdays.
    const TODAY: Date = Date::constant(2026, 10, 1);
    const DECLINED_ON: &str = "declined 2026-10-01";

    fn at(month: i8, day: i8, hour: i8, minute: i8) -> DateTime {
        date(2026, month, day).at(hour, minute, 0, 0)
    }

    /// A fetched instance of `series`, as a feed gives it.
    fn instance(uid: &str, series: &str, start: DateTime, end: DateTime) -> DiscoveredEvent {
        let mut event = DiscoveredEvent::new(uid, "Weekly meeting", start, end, "localist");
        event.series_uid = series.to_string();
        event
    }

    /// An answered `event-accept` card in `archive/`, as the settlement leaves it. `events:` lists
    /// `listed`, the first its primary; an empty `verdict` or `series` writes a null.
    fn answered_card(vault: &Path, name: &str, status: &str, verdict: &str, series: &str, listed: &[&str], id: &str) {
        let events: String = listed.iter().map(|uid| format!("- \"{uid}\"\n")).collect();
        let primary = listed.first().copied().unwrap_or_default();
        let series = if series.is_empty() { "null".to_string() } else { format!("\"{series}\"") };
        let text = format!(
            "---\nid: {id}\ntype: approval\nkind: {EVENT_ACCEPT}\ntitle: \"Weekly meeting · Tue 6 Oct 7–9pm\"\n\
             status: {status}\nverdict: {verdict}\nsource_uid: \"{primary}\"\nseries_uid: {series}\n\
             events:\n{events}proposed_at: 2026-09-24\nfirst_proposed_at: 2026-09-24\n\
             expires: 2026-10-06\nsnooze_until: null\ncreated_by: events\n---\n\nbody\n"
        );
        fs::create_dir_all(vault.join("archive")).unwrap();
        pystr::write_text(&vault.join("archive").join(name), &text).unwrap();
    }

    /// The context `rank` passes: an agent, so `write`'s human gate does not apply.
    fn rank_ctx() -> WriteContext {
        WriteContext { actor: "agent:approvals".into(), via: "cli".into(), run_id: Some("run-carry".into()) }
    }

    /// The student's own context, its actor read from the vault (rule 1: no literal).
    fn student(vault: &Path) -> WriteContext {
        WriteContext::new(crate::journal::read_human_actor(vault).unwrap(), "dashboard")
    }

    /// One `run` over the vault's ledger as `rank` loads it: `(lines, warnings, the updated map)`.
    fn carry(vault: &Path, events: &[DiscoveredEvent]) -> (usize, Vec<String>, BTreeMap<String, LedgerEntry>) {
        let mut ledger = load_ledger(vault, None);
        let mut journal = Journal::new(vault);
        let (lines, warnings) = run(vault, events, &mut ledger, TODAY, &rank_ctx(), &mut journal);
        (lines, warnings, ledger)
    }

    fn ledger_text(vault: &Path) -> Option<String> {
        pystr::read_text(&vault.join("state").join("events-seen.md")).ok()
    }

    /// The `declined` lines of the ledger, in file order.
    fn declined_lines(vault: &Path) -> Vec<String> {
        let text = ledger_text(vault).unwrap_or_default();
        text.lines().filter(|l| l.contains(" · declined ")).map(str::to_string).collect()
    }

    /// Every journal file's bytes, in file-name order.
    fn journal_text(vault: &Path) -> String {
        let dir = vault.join("state").join("journal");
        let mut files: Vec<PathBuf> = fs::read_dir(&dir)
            .map(|d| d.filter_map(|e| e.ok()).map(|e| e.path()).collect())
            .unwrap_or_default();
        files.sort();
        files.iter().map(|f| fs::read_to_string(f).unwrap()).collect()
    }

    /// `(file name, frontmatter)` of every note in `folder`, in file-name order.
    fn notes(vault: &Path, folder: &str) -> Vec<(String, Mapping)> {
        crate::approvals::sorted_md(&vault.join(folder))
            .into_iter()
            .map(|p| {
                let (meta, _) = split_frontmatter(&pystr::read_text(&p).unwrap()).unwrap();
                (p.file_name().unwrap().to_string_lossy().into_owned(), meta)
            })
            .collect()
    }

    #[test]
    fn the_ever_written_set_reads_commitments_tasks_and_archive() {
        let vault = tmp("ever-written");
        let mut journal = Journal::new(&vault);
        let ctx = rank_ctx();
        // Written as the settlement writes them; two then deleted as the student deletes them.
        let mut book = |uid: &str, day: i8| {
            let event = instance(uid, "lx:1", at(10, day, 19, 0), at(10, day, 21, 0));
            let commitment = commitment_for(&Instance::from_event(&event), Level::Hard).unwrap();
            create_confirmed(&vault, &commitment, uid, TODAY, &ctx, &mut journal).unwrap()
        };
        book("lx:1:1", 6);
        let gone = book("lx:3:1", 13);
        let mut register = |uid: &str, day: i8| {
            let mut event = instance(uid, "lx:2", at(10, day, 19, 0), at(10, day, 21, 0));
            event.registration = true;
            let (stem, text) = crate::eventaccept::register_task(&Instance::from_event(&event)).unwrap();
            crate::write::create(&vault, &format!("tasks/{stem}.md"), &text, &ctx, &mut journal, None).unwrap()
        };
        register("lx:2:1", 6);
        let gone_task = register("lx:4:1", 13);
        for path in [&gone, &gone_task] {
            crate::write::delete(&vault, &crate::ids::rel(&vault, path), &student(&vault), &mut journal).unwrap();
        }
        assert_eq!(notes(&vault, "archive").len(), 2, "write::delete moves each note to archive/");
        // Approval cards carry their primary's uid (`lx:77:1`) as `source_uid`: not a note written.
        accept(&vault, "event-weekly-2026-10-06.md", "executed", "lx:77", Some(ID_A));
        card(&vault, "approvals", "event-pending.md", EVENT_ACCEPT, "pending", "series_uid: \"lx:77\"\n");
        // A folder the set does not read, a null uid, and an unreadable note.
        fs::create_dir_all(vault.join("info")).unwrap();
        pystr::write_text(&vault.join("info").join("x.md"), "---\ntype: info\nsource_uid: \"lx:7:1\"\n---\n").unwrap();
        pystr::write_text(&vault.join("tasks").join("y.md"), "---\ntitle: Y\nsource_uid: null\n---\n").unwrap();
        pystr::write_text(&vault.join("commitments").join("z.md"), "---\nsource_uid: [lx:8:1\n---\n").unwrap();

        let set: Vec<String> = ever_written(&vault).into_iter().collect();
        assert_eq!(set, ["lx:1:1", "lx:3:1", "register:lx:2:1", "register:lx:4:1"]);
    }

    #[test]
    fn d9_rebuilds_declined_lines_from_a_rejected_card() {
        let vault = tmp("d9");
        let film = ["lx:78:1", "lx:78:2"];
        answered_card(&vault, "event-film-night-2026-10-08.md", "rejected", "opportunity", "lx:78", &film, "appr_00000000a1");
        // Every rejected card counts, whatever its series: its lines decline its own instances.
        answered_card(&vault, "event-talk-2026-10-09.md", "rejected", "opportunity", "", &["lx:79:1"], "appr_00000000a2");
        // An executed or expired card, and a rejected `event-check` card, write nothing here.
        answered_card(&vault, "event-weekly-2026-10-06.md", "executed", "obligation", "lx:77", &["lx:77:1"], "appr_00000000a3");
        answered_card(&vault, "event-fair-2026-10-07.md", "expired", "opportunity", "lx:80", &["lx:80:1"], "appr_00000000a4");
        card(&vault, "archive", "event-check-lab.md", "event-check", "rejected", "series_uid: \"lx:81\"\n");
        assert_eq!(ledger_text(&vault), None, "an empty ledger");

        let (lines, warnings, ledger) = carry(&vault, &[]);
        assert_eq!((lines, warnings), (3, Vec::<String>::new()));
        let expected: Vec<String> =
            ["lx:78:1", "lx:78:2", "lx:79:1"].iter().map(|uid| format!("- {uid} · {DECLINED_ON}")).collect();
        assert_eq!(declined_lines(&vault), expected, "cards in file-name order, uids in `events:` order");
        assert_eq!(ledger, load_ledger(&vault, None), "the map reads as `load_ledger` reads the file");
        assert_eq!(journal_text(&vault), "", "a ledger line has no journal record");

        // `relevant_events` then excludes them: each has a verdict that would list it, as a control does.
        let events: Vec<DiscoveredEvent> = ["lx:78:1", "lx:78:2", "lx:79:1", "lx:82:1"]
            .iter()
            .map(|uid| instance(uid, "lx:78", at(10, 8, 19, 0), at(10, 8, 21, 0)))
            .collect();
        for event in &events {
            record_verdict(&vault, &event.uid, "Film night", TODAY, "opportunity", "", "", "").unwrap();
        }
        let shown = crate::eventroster::relevant_events(&events, &load_ledger(&vault, None));
        assert_eq!(shown.iter().map(|e| e.uid.as_str()).collect::<Vec<_>>(), ["lx:82:1"]);

        // A second call writes nothing.
        let before = ledger_text(&vault);
        let (lines, warnings, _) = carry(&vault, &[]);
        assert_eq!((lines, warnings), (0, Vec::<String>::new()));
        assert_eq!(ledger_text(&vault), before);
    }

    #[test]
    fn a_declined_series_declines_its_later_instances() {
        let vault = tmp("declined-series");
        answered_card(&vault, "event-film-night-2026-10-06.md", "rejected", "opportunity", "lx:78", &["lx:78:1"], "appr_00000000b1");
        // The settlement's own line for the instance the card listed (spec §4.3).
        crate::eventledger::record_declined(&vault, "lx:78:1", date(2026, 9, 30)).unwrap();
        let events = [
            instance("lx:78:2", "lx:78", at(10, 13, 19, 0), at(10, 13, 21, 0)),
            instance("lx:78:1", "lx:78", at(10, 6, 19, 0), at(10, 6, 21, 0)),
            // A decline has no date rule (spec §4.5): an instance already started is declined too.
            instance("lx:78:0", "lx:78", at(9, 29, 19, 0), at(9, 29, 21, 0)),
            // A series nobody answered.
            instance("lx:90:1", "lx:90", at(10, 13, 19, 0), at(10, 13, 21, 0)),
        ];
        let (lines, warnings, ledger) = carry(&vault, &events);
        assert_eq!((lines, warnings), (2, Vec::<String>::new()));
        let expected = vec![
            "- lx:78:1 · declined 2026-09-30".to_string(),
            format!("- lx:78:0 · {DECLINED_ON}"),
            format!("- lx:78:2 · {DECLINED_ON}"),
        ];
        assert_eq!(declined_lines(&vault), expected, "one line per new instance, in (start, uid) order");
        assert_eq!(ledger, load_ledger(&vault, None), "the map reads as `load_ledger` reads the file");
        assert!(!ledger.contains_key("lx:90:1"));
        assert!(!vault.join(crate::commitments::FOLDER).exists());
        assert_eq!(journal_text(&vault), "");

        let before = ledger_text(&vault);
        let (lines, warnings, _) = carry(&vault, &events);
        assert_eq!((lines, warnings), (0, Vec::<String>::new()));
        assert_eq!(ledger_text(&vault), before, "a second call writes nothing");
    }

    #[test]
    fn an_accepted_series_books_a_later_instance_once() {
        let vault = tmp("accepted-series");
        answered_card(&vault, "event-weekly-meeting-2026-10-06.md", "executed", "obligation", "lx:77", &["lx:77:1"], "appr_00000000c1");
        answered_card(&vault, "event-film-night-2026-10-08.md", "executed", "opportunity", "lx:88", &["lx:88:1"], "appr_00000000c2");
        let mut later = instance("lx:77:2", "lx:77", at(10, 13, 19, 0), at(10, 13, 21, 0));
        later.registration = true; // P9: the carry still writes no register task
        later.location = "Ferguson Center".into();
        let events = [
            later.clone(),
            instance("lx:77:1", "lx:77", at(10, 6, 19, 0), at(10, 6, 21, 0)), // listed: the settlement's
            instance("lx:77:0", "lx:77", at(9, 30, 19, 0), at(9, 30, 21, 0)), // starts before today
            instance("lx:77:5", "lx:77", at(10, 1, 18, 0), at(10, 1, 19, 0)), // starts today
            instance("lx:77:3", "lx:77", at(10, 20, 0, 0), at(10, 21, 0, 0)), // all day: the answer stands
            instance("lx:88:2", "lx:88", at(10, 15, 17, 0), at(10, 15, 18, 0)), // an opportunity: soft
            later, // the same instance from a second feed: booked once
        ];
        let (_, warnings, _) = carry(&vault, &events);
        assert_eq!(warnings, Vec::<String>::new(), "an all-day instance is not a warning");

        let booked: Vec<(String, String, String, String)> = notes(&vault, crate::commitments::FOLDER)
            .iter()
            .map(|(_, m)| (card_text(m, "source_uid"), card_text(m, "level"), card_text(m, "kind"), card_text(m, "from")))
            .collect();
        let row = |uid: &str, level: &str, from: &str| (uid.to_string(), level.to_string(), "event".to_string(), from.to_string());
        let mut sorted = booked.clone();
        sorted.sort();
        assert_eq!(
            sorted,
            [row("lx:77:2", "hard", "2026-10-13"), row("lx:77:5", "hard", "2026-10-01"), row("lx:88:2", "soft", "2026-10-15")],
            "the level is the card's verdict's; nothing before today, listed, or all day"
        );
        // One journal `create` per note, journal first, as `agent:commitments`, in (start, uid) order.
        let records = Journal::new(&vault).read(None, None);
        let created: Vec<(String, String)> = records
            .iter()
            .map(|r| (r["op"].as_str().unwrap().to_string(), r["actor"].as_str().unwrap().to_string()))
            .collect();
        assert_eq!(created, vec![("create".to_string(), crate::commitments::CARD_ACTOR.to_string()); 3]);
        let order: Vec<String> = records
            .iter()
            .map(|r| {
                let meta = split_frontmatter(&pystr::read_text(&vault.join(r["path"].as_str().unwrap())).unwrap()).unwrap().0;
                card_text(&meta, "source_uid")
            })
            .collect();
        assert_eq!(order, ["lx:77:5", "lx:77:2", "lx:88:2"]);
        assert!(!vault.join("tasks").exists(), "P9: the carry writes commitments only");

        // A second call writes nothing, no journal record included.
        let journal = journal_text(&vault);
        let (_, warnings, _) = carry(&vault, &events);
        assert_eq!(warnings, Vec::<String>::new());
        assert_eq!(journal_text(&vault), journal);
        assert_eq!(notes(&vault, crate::commitments::FOLDER).len(), 3);
    }

    #[test]
    fn a_deleted_carried_note_stays_deleted() {
        let vault = tmp("delete-sticks");
        answered_card(&vault, "event-weekly-meeting-2026-10-06.md", "executed", "obligation", "lx:77", &["lx:77:1"], "appr_00000000d1");
        let mut primary = instance("lx:77:1", "lx:77", at(10, 6, 19, 0), at(10, 6, 21, 0));
        primary.registration = true;
        // The settlement's register task, for the primary only (spec §4.2).
        let (stem, text) = crate::eventaccept::register_task(&Instance::from_event(&primary)).unwrap();
        let mut journal = Journal::new(&vault);
        crate::write::create(&vault, &format!("tasks/{stem}.md"), &text, &rank_ctx(), &mut journal, None).unwrap();
        let events = [primary, instance("lx:77:2", "lx:77", at(10, 13, 19, 0), at(10, 13, 21, 0))];
        let (_, warnings, _) = carry(&vault, &events);
        assert_eq!(warnings, Vec::<String>::new());
        let carried = notes(&vault, crate::commitments::FOLDER);
        assert_eq!(carried.iter().map(|(_, m)| card_text(m, "source_uid")).collect::<Vec<_>>(), ["lx:77:2"]);

        // The student deletes the carried commitment and the register task: each goes to archive/.
        let mut journal = Journal::new(&vault);
        for rel in [format!("commitments/{}", carried[0].0), format!("tasks/{stem}.md")] {
            crate::write::delete(&vault, &rel, &student(&vault), &mut journal).unwrap();
        }
        let after_delete = journal_text(&vault);
        for _ in 0..2 {
            let (_, warnings, _) = carry(&vault, &events);
            assert_eq!(warnings, Vec::<String>::new());
        }
        assert!(notes(&vault, crate::commitments::FOLDER).is_empty(), "the deleted commitment stays deleted");
        assert!(notes(&vault, "tasks").is_empty(), "the deleted register task is never re-created");
        assert_eq!(journal_text(&vault), after_delete, "no journal record for either uid");
    }

    #[test]
    fn roster_read_events_carry_nothing() {
        let vault = tmp("roster-run");
        answered_card(&vault, "event-weekly-meeting-2026-10-06.md", "executed", "obligation", "lx:77", &["lx:77:1"], "appr_00000000e1");
        answered_card(&vault, "event-film-night-2026-10-06.md", "rejected", "opportunity", "lx:78", &["lx:78:1"], "appr_00000000e2");
        crate::eventledger::record_declined(&vault, "lx:78:1", date(2026, 9, 30)).unwrap();
        // The control: a fetched later instance of the declined series is carried.
        let control = [instance("lx:78:2", "lx:78", at(10, 13, 19, 0), at(10, 13, 21, 0))];
        assert_eq!(carry(&vault, &control).0, 1);
        // As `read_roster` builds them: `source: "roster"`, `series_uid` its own uid, one hour.
        let roster = |uid: &str| DiscoveredEvent::new(uid, "Weekly meeting", at(10, 20, 19, 0), at(10, 20, 20, 0), "roster");
        let events = [roster("lx:77:3"), roster("lx:78:3")];
        assert_eq!(events[0].series_uid, "lx:77:3");
        let ledger = ledger_text(&vault);

        let (lines, warnings, _) = carry(&vault, &events);
        assert_eq!((lines, warnings), (0, Vec::<String>::new()));
        // The series rule alone carries nothing either: an event whose series is its own uid
        // reaches no answered series, whatever its source.
        let fetched: Vec<DiscoveredEvent> =
            events.iter().map(|e| DiscoveredEvent { source: "localist".into(), ..e.clone() }).collect();
        let (lines, warnings, _) = carry(&vault, &fetched);
        assert_eq!((lines, warnings), (0, Vec::<String>::new()));

        assert_eq!(ledger_text(&vault), ledger, "no ledger line");
        assert!(!vault.join(crate::commitments::FOLDER).exists(), "no note");
        assert_eq!(journal_text(&vault), "", "no journal record");
    }

    #[test]
    fn a_card_with_an_unknown_verdict_books_nothing_and_warns() {
        let vault = tmp("unknown-verdict");
        // Only a hand edit makes these: the emitter writes `obligation` or `opportunity`.
        answered_card(&vault, "event-weekly-meeting-2026-10-06.md", "executed", "maybe", "lx:77", &["lx:77:1"], "appr_00000000f1");
        answered_card(&vault, "event-lab-2026-10-06.md", "executed", "", "lx:76", &["lx:76:1"], "appr_00000000f2");
        let events = [
            instance("lx:77:2", "lx:77", at(10, 13, 19, 0), at(10, 13, 21, 0)),
            instance("lx:76:2", "lx:76", at(10, 13, 15, 0), at(10, 13, 16, 0)),
        ];
        let (_, warnings, _) = carry(&vault, &events);
        let warned = |file: &str| format!("carry: archive/{file} has no verdict the carry reads; its series is not booked");
        assert_eq!(warnings, [warned("event-lab-2026-10-06.md"), warned("event-weekly-meeting-2026-10-06.md")]);
        assert!(!vault.join(crate::commitments::FOLDER).exists(), "Knowlu never guesses a level");
        assert_eq!(journal_text(&vault), "");
    }

    #[test]
    fn an_instance_turned_down_on_its_own_is_never_booked() {
        let vault = tmp("turned-down");
        answered_card(&vault, "event-weekly-meeting-2026-10-06.md", "executed", "obligation", "lx:77", &["lx:77:1"], "appr_00000000a5");
        let before = date(2026, 9, 30);
        crate::eventledger::record_declined(&vault, "lx:77:2", before).unwrap();
        let human = crate::journal::read_human_actor(&vault).unwrap();
        record_answer(&vault, "lx:77:3", "Weekly meeting", before, "drop", human, None).unwrap();
        // A machine's `drop` is no answer: the student's answer for the series covers the instance.
        record_verdict(&vault, "lx:77:4", "Weekly meeting", before, "drop", "", "", "").unwrap();
        let events: Vec<DiscoveredEvent> = [("lx:77:2", 13), ("lx:77:3", 20), ("lx:77:4", 27)]
            .iter()
            .map(|(uid, day)| instance(uid, "lx:77", at(10, *day, 19, 0), at(10, *day, 21, 0)))
            .collect();
        let (_, warnings, _) = carry(&vault, &events);
        assert_eq!(warnings, Vec::<String>::new());
        let booked: Vec<String> =
            notes(&vault, crate::commitments::FOLDER).iter().map(|(_, m)| card_text(m, "source_uid")).collect();
        assert_eq!(booked, ["lx:77:4"]);
    }

    #[test]
    fn a_uid_the_ledger_cannot_read_back_gets_no_declined_line() {
        let vault = tmp("unreadable-uid");
        let listed = ["lx:78:1", "lx:78 bad", "lx:verdict:2"];
        answered_card(&vault, "event-film-night-2026-10-06.md", "rejected", "opportunity", "lx:78", &listed, "appr_00000000a6");
        let events = [instance("lx:78:verdict:3", "lx:78", at(10, 13, 19, 0), at(10, 13, 21, 0))];
        let warned = |uid: &str| format!("carry: no declined line for {uid:?}: the ledger cannot read it back");
        let expected = [warned("lx:78 bad"), warned("lx:verdict:2"), warned("lx:78:verdict:3")];

        let (lines, warnings, _) = carry(&vault, &events);
        assert_eq!((lines, warnings.as_slice()), (1, expected.as_slice()));
        assert_eq!(declined_lines(&vault), [format!("- lx:78:1 · {DECLINED_ON}")]);
        // Written anyway, such a line would be written again on every `rank`.
        let before = ledger_text(&vault);
        let (lines, warnings, _) = carry(&vault, &events);
        assert_eq!((lines, warnings.as_slice()), (0, expected.as_slice()));
        assert_eq!(ledger_text(&vault), before);
    }
}
