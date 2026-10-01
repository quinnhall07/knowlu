//! The vault-writing event state outside `approvals.rs`: the ever-written set, the series carry,
//! D9's rebuild and `answered_series`.
//!
//! Spec `2026-09-29-events-design.md` §4.5 and §6.1. [`answered_series`] is the one definition of
//! an answered `event-accept` series. The emitter's never-ask-twice, the carry and `judge_roster`
//! all call it and none restates it, so one `rank` cannot both carry a series and ask about it.
//! [`accepted_check_series`] (PQ1 (a), Quinn, 2026-09-30) adds the series an accepted
//! `event-check` card carries, for the accept carry alone.
//!
//! [`run`] is the one entry point `rank` calls: D9's rebuild of `declined` lines from archived
//! `rejected` cards, then the carry over every fetched instance of an answered series that no event
//! card names. For each carried date of an accepted series, timed or lane-shaped, the carry writes
//! one ledger line credited to itself, never to the student (PQ3 (b-prime), the plan's P15):
//! `eventledger::record_carried_answer`, by `agent:knowlu.carry`, `from:` the answering card's id,
//! with the series' real verdict and the date's span as the feed gave it.
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
use crate::eventledger::{load_ledger, record_carried_answer, record_declined, LedgerEntry};
use crate::events::DiscoveredEvent;
use crate::journal::Journal;
use crate::models::split_frontmatter;
use crate::write::WriteContext;
use crate::{ids, pystr, yaml};

/// The card kind whose answers [`answered_series`] reads.
const EVENT_ACCEPT: &str = "event-accept";

/// The other event card kind. [`shown_on_a_card`] reads its uids; [`answered_series`] does not.
const EVENT_CHECK: &str = "event-check";

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

/// One series an archived `executed` `event-check` card with `instances:` claimed and accepted
/// (PQ1 (a)).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CheckAccept {
    /// The answering card's file name in `archive/`.
    pub file: String,
    /// The answering card's `id:`, read as [`SeriesAccept::id`] is: the carry's ledger line names
    /// the card by it (the plan's P15, `from:`).
    pub id: String,
    /// The uids the card's `events:` lists. The settlement answered these, so the carry never does.
    pub listed: BTreeSet<String>,
}

/// Every series an accepted `event-check` card carries, keyed by `series_uid` (PQ1 (a), Quinn,
/// 2026-09-30). D11 makes such a card's Approve an Accept, so its series carries as an accepted
/// `event-accept` series does, at `level: hard` (P7).
///
/// Only the accept carry reads it: never the decline carry, the emitter or `judge_roster`, which
/// keep [`answered_series`] (whose edge rule still leaves `event-check` cards out). The rules:
/// - it reads `archive/` only, through `approvals::sorted_md`, and only `type: approval`,
///   `kind: event-check` cards;
/// - a series is claimed as `eventemit::settled_series` claims it, so the two `event-check`
///   readers never disagree: the lowest-named `executed` or `rejected` card with a non-empty
///   `series_uid` claims it, and an `expired` card or any other status claims nothing (ruling G1);
/// - the series carries only when its claiming card is `executed` and its `instances:` is a
///   sequence. A claiming `rejected` card (its series has `drop` lines through
///   `eventemit::inherit_series_answers`) or a claiming card filed before this lane (no
///   `instances:`: its Approve only answered) carries nothing, and no later card of the series
///   can book it (review finding on T2b.2b).
///
/// It reads no ledger. An unreadable card answers nothing.
pub fn accepted_check_series(vault: &Path) -> BTreeMap<String, CheckAccept> {
    let mut claimed: BTreeSet<String> = BTreeSet::new();
    let mut accepted: BTreeMap<String, CheckAccept> = BTreeMap::new();
    for path in crate::approvals::sorted_md(&vault.join("archive")) {
        let Ok(text) = pystr::read_text(&path) else { continue };
        let Ok((meta, _)) = split_frontmatter(&text) else { continue };
        if card_text(&meta, "type") != "approval" || card_text(&meta, "kind") != EVENT_CHECK {
            continue;
        }
        let executed = match card_text(&meta, "status").as_str() {
            "executed" => true,
            "rejected" => false,
            _ => continue,
        };
        let series = card_text(&meta, "series_uid");
        if series.is_empty() || !claimed.insert(series.clone()) {
            continue;
        }
        if !executed || !matches!(yaml::get(&meta, "instances"), Some(serde_yaml_ng::Value::Sequence(_))) {
            continue;
        }
        let file = path.file_name().map(|n| n.to_string_lossy().into_owned()).unwrap_or_default();
        let id = yaml::opt_text(yaml::get(&meta, "id")).filter(|id| ids::is_id(id)).unwrap_or_default();
        let listed = crate::eventemit::card_event_uids(&meta).into_iter().collect();
        accepted.insert(series, CheckAccept { file, id, listed });
    }
    accepted
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

/// The shown set: every uid an event card of either kind names, as `source_uid` or in `events:`,
/// in `approvals/` or `archive/`, whatever its status. These are the folders and kinds the
/// emitter's never-ask-twice reads. The carry acts on none of them.
///
/// An answer covers the instances its own card lists and the series' instances that no card has
/// shown. An instance another card showed belongs to that card. While the card is live, its own
/// answer is still to come. Once it has expired, the instance stays unanswered: D8 keeps it in
/// Coming up and out of the plan. A later card of the series never lists it (ruling G1), so that
/// card's answer does not cover it either (review finding on T2b.2). Unreadable cards are skipped.
fn shown_on_a_card(vault: &Path) -> BTreeSet<String> {
    let mut shown = BTreeSet::new();
    for folder in ["approvals", "archive"] {
        for path in crate::approvals::sorted_md(&vault.join(folder)) {
            let Ok(text) = pystr::read_text(&path) else { continue };
            let Ok((meta, _)) = split_frontmatter(&text) else { continue };
            let kind = card_text(&meta, "kind");
            if card_text(&meta, "type") != "approval" || (kind != EVENT_ACCEPT && kind != EVENT_CHECK) {
                continue;
            }
            let uid = card_text(&meta, "source_uid");
            if !uid.is_empty() {
                shown.insert(uid);
            }
            shown.extend(crate::eventemit::card_event_uids(&meta));
        }
    }
    shown
}

/// D9's rebuild, then the series carry (spec §4.5, D4, D9): the one entry point `rank` calls.
/// Returns `(ledger lines written, warnings)`, and updates `ledger` as `load_ledger` would read
/// every new line.
///
/// 1. **D9.** Each uid a `rejected` `event-accept` card in `archive/` lists gets a `declined` line
///    unless it has one: the archived card is the record of the answer, and it syncs where the
///    ledger does not. An `executed` card needs nothing here.
/// 2. **The carry**, over each fetched instance in `events` whose series a card answered
///    ([`answered_series`], or for an accept [`accepted_check_series`] too) and whose uid no event
///    card names (`shown_on_a_card`: not the answering card, and not an expired or live one), in
///    `(start, uid)` order, each uid once:
///    - a declined series ([`answered_series`] alone) gives the instance a `declined` line unless
///      it has one;
///    - an accepted series (`accepted_series`: the union, [`answered_series`] winning an overlap)
///      books an instance starting today or later, one carried date at a time (`book`): the
///      commitment `eventaccept::commitment_for` gives at the level the `event-accept` card's
///      `verdict:` names, or `hard` for an `event-check` series (P7), through
///      `commitments::create_confirmed` (actor `agent:commitments`, journal first). A
///      lane-shaped instance (all-day, multi-day, zero-length) has no commitment. No register
///      task is written (P9): a series has one, for its primary. It never books a uid in the
///      [`ever_written`] set, so a note the student deleted stays deleted, nor an instance the
///      student turned down on its own (`turned_down`);
///    - then, whatever the booking's outcome, the date's one carry line (`carry_line`, PQ3, P15),
///      unless the uid already has a carry line, a human answer or a `declined` line. A card with
///      no usable `id:` books its dates and writes no line, with one warning naming it.
///
/// A `source: "roster"` event is never carried (the module doc says why).
///
/// A write failure is a warning and the next `rank` retries; nothing here panics. Every carried
/// note is a journal `create` in this run, which is what lists it in the read model's delta. No
/// ledger line has a journal record, the carry's included.
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
    let accepted = accepted_series(&answered, &accepted_check_series(vault), &mut warnings);
    let written = ever_written(vault);
    let shown = shown_on_a_card(vault);
    let mut ordered: Vec<&DiscoveredEvent> = events.iter().filter(|e| e.source != ROSTER_SOURCE).collect();
    ordered.sort_by(|a, b| (a.start(), &a.uid).cmp(&(b.start(), &b.uid)));
    let mut seen: BTreeSet<&str> = BTreeSet::new();
    let mut carried: Vec<String> = Vec::new();
    let mut unlisted: BTreeSet<String> = BTreeSet::new();
    for event in ordered {
        if !seen.insert(event.uid.as_str()) || shown.contains(&event.uid) {
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
        // PQ3: the date's one line, whatever the booking's outcome (P15's *When*).
        if carry_line(vault, event, series, ledger, today, &mut unlisted, &mut warnings) {
            carried.push(event.uid.clone());
        }
    }
    read_back(vault, &carried, ledger);
    (lines + carried.len(), warnings)
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
    uid_reads(uid) && !uid.contains("verdict:")
}

/// Does `load_ledger` read the head of any line for `uid` (`- <uid> · `) back? Its uid class is
/// `[A-Za-z0-9_.:@+-]`. A `verdict:` inside the uid does not break a verdict or answer line, whose
/// own `· verdict:` field comes first.
fn uid_reads(uid: &str) -> bool {
    !uid.is_empty() && uid.chars().all(|c| c.is_ascii_alphanumeric() || "_.:@+-".contains(c))
}

/// One accepted series, as the accept carry books it and writes its lines.
struct Accepted {
    /// `Hard` for an obligation or an `event-check` series (P7), `Soft` for an opportunity (D1).
    level: Level,
    /// The series' real verdict, the carry line's word (P15): the `event-accept` card's
    /// `verdict:`, or `obligation` for an `event-check` series.
    word: &'static str,
    /// The answering card's `id:` (the carry line's `from:`), empty when it has none.
    id: String,
    /// The answering card's file name in `archive/`, for the warning when `id` is empty.
    file: String,
    /// The uids the answering card listed: the settlement's, never the carry's.
    listed: BTreeSet<String>,
}

/// The accepted set, keyed by `series_uid`: the one place it is built. It is the union of
/// [`answered_series`]' executed entries, at the level the card's `verdict:` names, and
/// [`accepted_check_series`] (PQ1 (a)), at `Hard` (P7).
///
/// A series in both readers, which only a hand edit makes, takes [`answered_series`]' answer
/// whatever it is: a series declined on an `event-accept` card is never booked, and one whose
/// `event-accept` card names no verdict the carry reads is not booked either. An `event-accept`
/// card with any other verdict books nothing and warns, since Knowlu never guesses a level; only a
/// hand edit makes one.
fn accepted_series(
    answered: &BTreeMap<String, SeriesAccept>,
    checks: &BTreeMap<String, CheckAccept>,
    warnings: &mut Vec<String>,
) -> BTreeMap<String, Accepted> {
    let mut accepted = BTreeMap::new();
    for (series, answer) in answered.iter().filter(|(_, answer)| answer.accepted) {
        let (level, word) = match answer.verdict.as_str() {
            "obligation" => (Level::Hard, "obligation"),
            "opportunity" => (Level::Soft, "opportunity"),
            _ => {
                let file = &answer.file;
                warnings.push(format!(
                    "carry: archive/{file} has no verdict the carry reads; its series is not booked"
                ));
                continue;
            }
        };
        let (id, file, listed) = (answer.id.clone(), answer.file.clone(), answer.listed.clone());
        accepted.insert(series.clone(), Accepted { level, word, id, file, listed });
    }
    for (series, check) in checks.iter().filter(|(series, _)| !answered.contains_key(*series)) {
        let (id, file, listed) = (check.id.clone(), check.file.clone(), check.listed.clone());
        accepted.insert(series.clone(), Accepted { level: Level::Hard, word: "obligation", id, file, listed });
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

/// One carried date's ledger line (PQ3 (b-prime), P15), after its booking step whatever that
/// step's outcome: `record_carried_answer` with the series' real verdict, the answering card's id
/// and the event's own span. `true` when a line was written.
///
/// It does not wait on the booking, which retries on its own through the ever-written set: a line
/// held back by a failed booking would let the same run's `inherit_series_answers` (P16) answer the
/// date as the student, and the carry never could after that. No line for a uid that already has a
/// carry line, a human answer or a `declined` line (judge once). A card with no usable `id:` books
/// its dates but cannot name itself on a line: one warning per card (`unlisted` holds the files
/// warned about). A line the ledger could not read back would be written again on every `rank`, so
/// it is not written and a warning says why. The line has no journal record, as no ledger line has
/// one. A write failure is a warning, and the next `rank` retries.
fn carry_line(
    vault: &Path,
    event: &DiscoveredEvent,
    series: &Accepted,
    ledger: &BTreeMap<String, LedgerEntry>,
    today: Date,
    unlisted: &mut BTreeSet<String>,
    warnings: &mut Vec<String>,
) -> bool {
    let answered = |entry: &LedgerEntry| entry.carry.is_some() || entry.declined || answered_by_human(entry);
    if ledger.get(&event.uid).is_some_and(answered) {
        return false;
    }
    if series.id.is_empty() {
        if unlisted.insert(series.file.clone()) {
            let file = &series.file;
            warnings.push(format!("carry: archive/{file} has no id; its dates are booked but not listed"));
        }
        return false;
    }
    let span_reads = [event.start(), event.end()].iter().all(|at| (0..=9999).contains(&at.year()));
    if !uid_reads(&event.uid) || !span_reads {
        warnings.push(format!("carry: no line for {:?}: the ledger cannot read it back", event.uid));
        return false;
    }
    let (start, end) = (event.start(), event.end());
    let written = record_carried_answer(vault, &event.uid, &event.title, today, series.word, &series.id, start, end);
    if let Err(err) = written {
        warnings.push(format!("carry: no line for {} ({err})", event.uid));
        return false;
    }
    true
}

/// The map updated as `load_ledger` reads the carry's new lines (P15's *Read*): each uid given a
/// line in this run takes its entry from the file itself, so the map and the reader cannot
/// disagree. One read, and only when a line was written.
fn read_back(vault: &Path, uids: &[String], ledger: &mut BTreeMap<String, LedgerEntry>) {
    if uids.is_empty() {
        return;
    }
    let fresh = load_ledger(vault, None);
    for uid in uids {
        if let Some(entry) = fresh.get(uid) {
            ledger.insert(uid.clone(), entry.clone());
        }
    }
}

/// Did the student turn this instance down on its own: a `declined` line, or a human `drop`
/// answer? The carry never books one (judge once). A machine's `drop` verdict is no answer: the
/// student's answer for the series covers it.
fn turned_down(entry: &LedgerEntry) -> bool {
    entry.declined || (answered_by_human(entry) && entry.verdict.as_deref() == Some("drop"))
}

/// Was this uid settled by a human answer: an `answered_by` that `provenance::is_agent` rejects?
/// The carry's own line leaves an agent's (`eventledger::CARRY_ACTOR`).
fn answered_by_human(entry: &LedgerEntry) -> bool {
    !entry.answered_by.is_empty() && !crate::provenance::is_agent(&entry.answered_by)
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
    fn an_instance_another_card_showed_is_never_carried() {
        // Review finding on T2b.2 (D8 against D4, §4.5): a later card's answer does not reach an
        // instance an earlier card showed. Expired, that card left it unanswered: it stays in Coming
        // up and out of the plan (D8). The emitter never puts it on the later card (ruling G1).
        let vault = tmp("shown-elsewhere");
        answered_card(&vault, "event-weekly-2026-10-06.md", "expired", "opportunity", "lx:77", &["lx:77:1"], "appr_00000000b2");
        answered_card(&vault, "event-weekly-2026-10-13.md", "executed", "opportunity", "lx:77", &["lx:77:2"], "appr_00000000b3");
        answered_card(&vault, "event-film-2026-10-06.md", "expired", "opportunity", "lx:78", &["lx:78:1"], "appr_00000000b4");
        answered_card(&vault, "event-film-2026-10-13.md", "rejected", "opportunity", "lx:78", &["lx:78:2"], "appr_00000000b5");
        // Either event kind, any status, either folder, named in `events:` or as `source_uid` alone.
        let shown = |folder: &str, name: &str, kind: &str, status: &str, uid: &str, in_events: bool| {
            let events = if in_events { format!("events:\n- \"{uid}\"\n") } else { String::new() };
            let text = format!(
                "---\ntype: approval\nkind: {kind}\nstatus: {status}\nsource_uid: \"{uid}\"\n\
                 series_uid: \"{}\"\n{events}---\n\nbody\n",
                &uid[..5]
            );
            fs::create_dir_all(vault.join(folder)).unwrap();
            pystr::write_text(&vault.join(folder).join(name), &text).unwrap();
        };
        shown("archive", "event-check-a.md", "event-check", "expired", "lx:77:3", true);
        shown("approvals", "event-weekly-b.md", EVENT_ACCEPT, "pending", "lx:77:4", true);
        shown("archive", "event-check-c.md", "event-check", "expired", "lx:78:3", false);
        shown("approvals", "event-check-d.md", "event-check", "pending", "lx:78:4", true);
        // `:5` of each series is on no card: the controls, carried as before.
        let events: Vec<DiscoveredEvent> = ["lx:77", "lx:78"]
            .iter()
            .flat_map(|series| {
                [(1, 10, 6), (2, 10, 13), (3, 10, 20), (4, 10, 27), (5, 11, 3)].map(|(n, month, day)| {
                    instance(&format!("{series}:{n}"), series, at(month, day, 19, 0), at(month, day, 21, 0))
                })
            })
            .collect();

        // No assertion on `run`'s line count: an accepted series carries here (the T2b.2 note above).
        let (_, warnings, ledger) = carry(&vault, &events);
        assert_eq!(warnings, Vec::<String>::new());
        let booked: Vec<String> =
            notes(&vault, crate::commitments::FOLDER).iter().map(|(_, m)| card_text(m, "source_uid")).collect();
        assert_eq!(booked, ["lx:77:5"], "no instance another card showed is booked");
        let expected = [format!("- lx:78:2 · {DECLINED_ON}"), format!("- lx:78:5 · {DECLINED_ON}")];
        assert_eq!(declined_lines(&vault), expected, "D9 for the card's own, the carry for the control");
        assert_eq!(ledger, load_ledger(&vault, None));
        for uid in ["lx:77:1", "lx:78:1", "lx:78:3", "lx:78:4"] {
            assert!(!ledger.contains_key(uid), "{uid}: no line, so it stays in Coming up (D8)");
        }
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

    // --- T2b.2b: PQ1 (a), an accepted `event-check` series carries -----------------------------
    //
    // As in T2b.2, no test here asserts on the accept carry's ledger lines: T2b.5 adds them.

    /// An answered `event-check` card in `archive/`, in the emitter's shape: no `verdict:`,
    /// `events:` lists `listed` (the first its primary), and `instances:` carries them when
    /// `with_instances` (a card filed before this lane carries none). An empty `series` is `''`.
    fn check_card(vault: &Path, name: &str, status: &str, series: &str, listed: &[&DiscoveredEvent], with_instances: bool, id: &str) {
        use crate::yamlemit::Node;
        let mut pairs = vec![
            ("id", Node::text(id)),
            ("type", Node::text("approval")),
            ("kind", Node::text(EVENT_CHECK)),
            ("title", Node::text(&crate::eventemit::what_and_when(listed[0]))),
            ("status", Node::text(status)),
            ("source_uid", Node::text(&listed[0].uid)),
            ("series_uid", Node::text(series)),
            ("events", Node::Seq(listed.iter().map(|e| Node::text(&e.uid)).collect())),
        ];
        if with_instances {
            pairs.push(("instances", Node::Seq(listed.iter().map(|e| Instance::from_event(e).to_node()).collect())));
        }
        pairs.extend([
            ("proposed_at", Node::Date(date(2026, 9, 24))),
            ("first_proposed_at", Node::Date(date(2026, 9, 24))),
            ("expires", Node::Date(listed[0].start().date())),
            ("snooze_until", Node::Null),
            ("created_by", Node::text("events")),
        ]);
        let text = format!("---\n{}---\n\nbody\n", crate::yamlemit::safe_dump_block(&Node::map(pairs)));
        fs::create_dir_all(vault.join("archive")).unwrap();
        pystr::write_text(&vault.join("archive").join(name), &text).unwrap();
    }

    /// `(source_uid, level)` of every commitment, in file-name order.
    fn booked(vault: &Path) -> Vec<(String, String)> {
        let rows = notes(vault, crate::commitments::FOLDER);
        rows.iter().map(|(_, m)| (card_text(m, "source_uid"), card_text(m, "level"))).collect()
    }

    #[test]
    fn an_accepted_event_check_series_books_a_later_instance_once() {
        let vault = tmp("check-series");
        let first = instance("lx:9:1", "lx:9", at(10, 6, 19, 0), at(10, 6, 21, 0));
        let mut later = instance("lx:9:2", "lx:9", at(10, 13, 19, 0), at(10, 13, 21, 0));
        later.registration = true; // P9: the carry still writes no register task
        check_card(&vault, "event-check-weekly-b.md", "executed", "lx:9", &[&first], true, ID_B);
        check_card(&vault, "event-check-weekly-a.md", "executed", "lx:9", &[&first], true, ID_A);
        // The reader: the lowest file name wins; the value is its file, its id and its listed uids.
        let checks = accepted_check_series(&vault);
        let listed = BTreeSet::from(["lx:9:1".to_string()]);
        let file = "event-check-weekly-a.md".to_string();
        assert_eq!(checks, BTreeMap::from([("lx:9".to_string(), CheckAccept { file, id: ID_A.into(), listed })]));
        assert!(answered_series(&vault).is_empty(), "T2b.1's edge rule is unchanged: the card is not counted");

        let events = [first.clone(), later.clone()];
        let (_, warnings, _) = carry(&vault, &events);
        assert_eq!(warnings, Vec::<String>::new());
        let row = (String::from("lx:9:2"), String::from("hard"));
        assert_eq!(booked(&vault), [row.clone()], "P7: hard; nothing for lx:9:1, which the card listed");
        let records = Journal::new(&vault).read(None, None);
        let ops: Vec<(&str, &str)> = records.iter().map(|r| (r["op"].as_str().unwrap(), r["actor"].as_str().unwrap())).collect();
        assert_eq!(ops, [("create", crate::commitments::CARD_ACTOR)], "one create, journal first");
        assert!(!vault.join("tasks").exists(), "P9: the carry writes commitments only");
        // A second call writes nothing, no journal record included.
        let journal = journal_text(&vault);
        let (_, warnings, _) = carry(&vault, &events);
        assert_eq!(warnings, Vec::<String>::new());
        assert_eq!((journal_text(&vault), booked(&vault)), (journal, vec![row]));

        // A hand edit makes an archived `rejected` `event-accept` card answer `lx:9` too:
        // `answered_series`' answer wins, so the series is declined and nothing is booked.
        let edited = tmp("check-series-hand-edit");
        check_card(&edited, "event-check-weekly-a.md", "executed", "lx:9", &[&first], true, ID_A);
        answered_card(&edited, "event-weekly-2026-10-20.md", "rejected", "obligation", "lx:9", &["lx:9:3"], ID_B);
        let (_, warnings, ledger) = carry(&edited, &events);
        assert_eq!(warnings, Vec::<String>::new());
        assert!(!edited.join(crate::commitments::FOLDER).exists(), "a series declined on an event-accept card");
        assert_eq!(journal_text(&edited), "");
        assert!(ledger["lx:9:2"].declined, "the decline carry's line, as answered_series answers it");

        // Built as `read_roster` builds events (series equal to uid): a one-instance `ics:fair`
        // whose executed card lists it writes nothing, from the roster or from a feed.
        let fair_vault = tmp("check-series-roster");
        let fair = DiscoveredEvent::new("ics:fair", "Career fair", at(10, 20, 10, 0), at(10, 20, 14, 0), "ics");
        check_card(&fair_vault, "event-check-career-fair-2026-10-20.md", "executed", "ics:fair", &[&fair], true, ID_A);
        let roster = DiscoveredEvent::new("ics:fair", "Career fair", at(10, 20, 10, 0), at(10, 20, 11, 0), "roster");
        assert_eq!(roster.series_uid, "ics:fair");
        for events in [[roster], [fair]] {
            let (lines, warnings, _) = carry(&fair_vault, &events);
            assert_eq!((lines, warnings), (0, Vec::<String>::new()));
        }
        assert!(!fair_vault.join(crate::commitments::FOLDER).exists(), "no note");
        assert_eq!((journal_text(&fair_vault), ledger_text(&fair_vault)), (String::new(), None), "no record, no line");
    }

    #[test]
    fn an_event_check_card_without_instances_carries_nothing() {
        let vault = tmp("check-without-instances");
        let primary = |n: u8| instance(&format!("lx:{n}:1"), &format!("lx:{n}"), at(10, 6, 19, 0), at(10, 6, 21, 0));
        let later = |n: u8| instance(&format!("lx:{n}:2"), &format!("lx:{n}"), at(10, 13, 19, 0), at(10, 13, 21, 0));
        // As filed before this lane: executed, with no `instances:` key. Its Approve only answered (D11).
        check_card(&vault, "event-check-a.md", "executed", "lx:1", &[&primary(1)], false, ID_A);
        // Rejected and expired, each carrying `instances:`.
        check_card(&vault, "event-check-b.md", "rejected", "lx:2", &[&primary(2)], true, ID_A);
        check_card(&vault, "event-check-c.md", "expired", "lx:3", &[&primary(3)], true, ID_A);
        // A hand edit: `instances:` that is not a sequence.
        card(&vault, "archive", "event-check-d.md", EVENT_CHECK, "executed", "series_uid: \"lx:4\"\ninstances: null\n");
        // Executed with `instances:`, but in `approvals/`, or with an empty `series_uid`.
        check_card(&vault, "event-check-e.md", "executed", "lx:5", &[&primary(5)], true, ID_A);
        fs::create_dir_all(vault.join("approvals")).unwrap();
        fs::rename(vault.join("archive").join("event-check-e.md"), vault.join("approvals").join("event-check-e.md")).unwrap();
        check_card(&vault, "event-check-f.md", "executed", "", &[&primary(6)], true, ID_A);
        assert!(accepted_check_series(&vault).is_empty(), "no card here answers a series for the carry");
        // The control: an executed card with `instances:`, whose later instance is booked.
        check_card(&vault, "event-check-g.md", "executed", "lx:7", &[&primary(7)], true, ID_B);

        let events: Vec<DiscoveredEvent> = (1..=7).map(later).collect();
        // No assertion on `run`'s line count: the control carries (the T2b.2b note above).
        let (_, warnings, _) = carry(&vault, &events);
        assert_eq!(warnings, Vec::<String>::new());
        // A rejected `event-check` series has its `drop` lines through `inherit_series_answers`.
        assert_eq!(declined_lines(&vault), Vec::<String>::new(), "the decline carry reads event-accept cards only");
        assert_eq!(booked(&vault), [(String::from("lx:7:2"), String::from("hard"))], "only the control");
        let records = Journal::new(&vault).read(None, None);
        assert_eq!(records.len(), 1, "one journal record, the control's");
    }

    #[test]
    fn a_rejected_or_pre_lane_check_card_blocks_its_series() {
        // Review finding on T2b.2b: the carry claims a series as `eventemit::settled_series` does.
        // The lowest-named executed or rejected card claims it, and the series carries only if that
        // card is executed with `instances:`. A hand edit, or sync merging two desktops' cards, makes
        // two cards for one series.
        let vault = tmp("check-claims");
        let at_day = |series: &str, n: u8, day: i8| instance(&format!("{series}:{n}"), series, at(10, day, 19, 0), at(10, day, 21, 0));
        // lx:9: the student rejected it first, then a later card was executed.
        check_card(&vault, "event-check-weekly-2026-10-13.md", "executed", "lx:9", &[&at_day("lx:9", 2, 13)], true, "appr_0000000002");
        check_card(&vault, "event-check-weekly-2026-10-06.md", "rejected", "lx:9", &[&at_day("lx:9", 1, 6)], true, "appr_0000000001");
        // lx:8: a card filed before this lane (no `instances:`), then one with them.
        check_card(&vault, "event-check-lab-2026-10-06.md", "executed", "lx:8", &[&at_day("lx:8", 1, 6)], false, "appr_0000000003");
        check_card(&vault, "event-check-lab-2026-10-13.md", "executed", "lx:8", &[&at_day("lx:8", 2, 13)], true, "appr_0000000004");
        // lx:6, the control: executed with `instances:` first, so a later rejected card loses.
        check_card(&vault, "event-check-club-2026-10-06.md", "executed", "lx:6", &[&at_day("lx:6", 1, 6)], true, "appr_0000000005");
        check_card(&vault, "event-check-club-2026-10-13.md", "rejected", "lx:6", &[&at_day("lx:6", 2, 13)], true, "appr_0000000006");

        // Both `event-check` readers give one answer per series.
        let settled = crate::eventemit::settled_series(&vault);
        let verdicts: Vec<(&str, &str)> = settled.iter().map(|(s, a)| (s.as_str(), a.verdict.as_str())).collect();
        assert_eq!(verdicts, [("lx:6", "obligation"), ("lx:8", "obligation"), ("lx:9", "drop")]);
        let checks = accepted_check_series(&vault);
        assert_eq!(checks.keys().map(String::as_str).collect::<Vec<_>>(), ["lx:6"]);
        assert_eq!(checks["lx:6"].file, "event-check-club-2026-10-06.md", "the claiming card's");

        let events = [at_day("lx:9", 3, 20), at_day("lx:8", 3, 20), at_day("lx:6", 3, 20)];
        let (_, warnings, _) = carry(&vault, &events);
        assert_eq!(warnings, Vec::<String>::new());
        assert_eq!(booked(&vault), [(String::from("lx:6:3"), String::from("hard"))], "only the control");
        let records = Journal::new(&vault).read(None, None);
        assert_eq!(records.len(), 1, "one journal record, the control's: none for lx:9:3 or lx:8:3");
    }

    // --- T2b.5: PQ3 (b-prime) with the span, the carry writes its line (P15) --------------------

    use crate::eventledger::{Carried, CARRY_ACTOR, HEADER, NEWLINE};

    /// PQ3's vault: an executed `event-accept` card for `lx:77` (`verdict: opportunity`, id `ID_A`)
    /// listing `lx:77:1`. The events, given latest first so `run`'s own order shows: a zero-length
    /// `:4`, a two-day all-day `:3`, a timed `:2`, the listed `:1`, and `:0`, before today.
    fn pq3_vault(name: &str) -> (PathBuf, Vec<DiscoveredEvent>) {
        let vault = tmp(name);
        answered_card(&vault, "event-weekly-meeting-2026-10-06.md", "executed", "opportunity", "lx:77", &["lx:77:1"], ID_A);
        let events = vec![
            instance("lx:77:4", "lx:77", at(10, 27, 18, 0), at(10, 27, 18, 0)),
            instance("lx:77:3", "lx:77", at(10, 20, 0, 0), at(10, 22, 0, 0)),
            instance("lx:77:2", "lx:77", at(10, 13, 19, 0), at(10, 13, 21, 0)),
            instance("lx:77:1", "lx:77", at(10, 6, 19, 0), at(10, 6, 21, 0)),
            instance("lx:77:0", "lx:77", at(9, 29, 19, 0), at(9, 29, 21, 0)),
        ];
        (vault, events)
    }

    /// T2b.2b's vault: an executed `event-check` card for `lx:9` (id `ID_A`) carrying `instances:`
    /// and listing `lx:9:1`; the events are `lx:9:1` and a later timed `lx:9:2`.
    fn check_vault(name: &str) -> (PathBuf, Vec<DiscoveredEvent>) {
        let vault = tmp(name);
        let first = instance("lx:9:1", "lx:9", at(10, 6, 19, 0), at(10, 6, 21, 0));
        let later = instance("lx:9:2", "lx:9", at(10, 13, 19, 0), at(10, 13, 21, 0));
        check_card(&vault, "event-check-weekly-a.md", "executed", "lx:9", &[&first], true, ID_A);
        (vault, vec![first, later])
    }

    /// The carry's line for `event`, as P15 shapes it, answered on `TODAY`.
    fn carry_line(event: &DiscoveredEvent, word: &str, from: &str) -> String {
        let span = |at: DateTime| at.strftime("%Y-%m-%dT%H:%M:%S").to_string();
        format!(
            "- {} · {} · verdict:{word} · by:{CARRY_ACTOR} · from:{from} · start:{} · end:{} · answered 2026-10-01",
            event.uid,
            event.title,
            span(event.start()),
            span(event.end())
        )
    }

    /// Every ledger line for `uid`, in file order.
    fn lines_for(vault: &Path, uid: &str) -> Vec<String> {
        let head = format!("- {uid} · ");
        ledger_text(vault).unwrap_or_default().lines().filter(|l| l.starts_with(&head)).map(str::to_string).collect()
    }

    /// The ledger's carry lines, in file order.
    fn carry_lines(vault: &Path) -> Vec<String> {
        let by = format!("· by:{CARRY_ACTOR} ·");
        ledger_text(vault).unwrap_or_default().lines().filter(|l| l.contains(&by)).map(str::to_string).collect()
    }

    fn row(uid: &str, level: &str) -> (String, String) {
        (uid.to_string(), level.to_string())
    }

    fn the_carry(from: &str, event: &DiscoveredEvent) -> Option<Carried> {
        Some(Carried { from: from.to_string(), start: event.start(), end: event.end() })
    }

    #[test]
    fn the_carry_writes_one_line_per_carried_date() {
        let (vault, events) = pq3_vault("pq3-lines");
        let (lines, warnings, ledger) = carry(&vault, &events);
        assert_eq!((lines, warnings), (3, Vec::<String>::new()));
        let expected = [
            "- lx:77:2 · Weekly meeting · verdict:opportunity · by:agent:knowlu.carry · from:appr_0123456789 · \
             start:2026-10-13T19:00:00 · end:2026-10-13T21:00:00 · answered 2026-10-01",
            "- lx:77:3 · Weekly meeting · verdict:opportunity · by:agent:knowlu.carry · from:appr_0123456789 · \
             start:2026-10-20T00:00:00 · end:2026-10-22T00:00:00 · answered 2026-10-01",
            "- lx:77:4 · Weekly meeting · verdict:opportunity · by:agent:knowlu.carry · from:appr_0123456789 · \
             start:2026-10-27T18:00:00 · end:2026-10-27T18:00:00 · answered 2026-10-01",
        ];
        let text = fs::read_to_string(vault.join("state").join("events-seen.md")).unwrap();
        let body: String = expected.iter().map(|l| format!("{l}{NEWLINE}")).collect();
        assert_eq!(text, format!("{HEADER}{NEWLINE}{body}"), "one line each, (start, uid) order; none for :1 or :0");
        for event in &events[..3] {
            let entry = &ledger[&event.uid];
            assert_eq!(entry.verdict.as_deref(), Some("opportunity"), "{}", event.uid);
            assert_eq!(entry.answered_by, CARRY_ACTOR, "{}", event.uid);
            assert_eq!(entry.carry, the_carry(ID_A, event), "{}: the card and the event's own span", event.uid);
        }
        assert_eq!(ledger, load_ledger(&vault, None), "the map reads as `load_ledger` reads the file");
        // `:2` also has its commitment, and that commitment's journal `create` record.
        assert_eq!(booked(&vault), [row("lx:77:2", "soft")]);
        let records = Journal::new(&vault).read(None, None);
        let created: Vec<(&str, &str)> = records.iter().map(|r| (r["op"].as_str().unwrap(), r["path"].as_str().unwrap())).collect();
        let note = format!("commitments/{}", notes(&vault, crate::commitments::FOLDER)[0].0);
        assert_eq!(created, [("create", note.as_str())]);
    }

    /// The fault: `state/journal` is a file, so a commitment's journal record cannot be written
    /// and `write::create` writes nothing (journal first). Returns its path, to lift it.
    fn break_the_journal(vault: &Path) -> PathBuf {
        let path = vault.join("state").join("journal");
        fs::create_dir_all(vault.join("state")).unwrap();
        fs::write(&path, "not a folder").unwrap();
        path
    }

    #[test]
    fn a_failed_booking_still_gets_its_line_and_is_rebooked_next_run() {
        let (vault, events) = pq3_vault("pq3-failed-booking");
        let fault = break_the_journal(&vault);
        let (lines, warnings, _) = carry(&vault, &events);
        assert_eq!(warnings.len(), 1, "{warnings:?}");
        assert!(warnings[0].starts_with("carry: lx:77:2 not booked ("), "{warnings:?}");
        assert_eq!(booked(&vault), Vec::<(String, String)>::new(), "no commitment");
        assert_eq!(journal_text(&vault), "", "no journal record");
        let carried: Vec<String> = ["lx:77:2", "lx:77:3", "lx:77:4"].iter().map(|u| u.to_string()).collect();
        let uids: Vec<String> = carry_lines(&vault).iter().map(|l| l[2..9].to_string()).collect();
        assert_eq!((lines, uids), (3, carried), "every carried date still gets its one line");

        // The fault gone: `:2` is booked, journal record first, and no second line is written.
        fs::remove_file(&fault).unwrap();
        let ledger = ledger_text(&vault);
        let (lines, warnings, _) = carry(&vault, &events);
        assert_eq!((lines, warnings), (0, Vec::<String>::new()));
        assert_eq!(booked(&vault), [row("lx:77:2", "soft")]);
        let records = Journal::new(&vault).read(None, None);
        let ops: Vec<(&str, &str)> = records.iter().map(|r| (r["op"].as_str().unwrap(), r["actor"].as_str().unwrap())).collect();
        assert_eq!(ops, [("create", crate::commitments::CARD_ACTOR)]);
        assert_eq!(ledger_text(&vault), ledger, "no second line");

        // P16's order: the same fault, then `inherit_series_answers`, leaves an `event-check`
        // series' carried date with the carry's line only, never the student's.
        let (check, events) = check_vault("pq3-failed-check");
        break_the_journal(&check);
        let (_, warnings, mut ledger) = carry(&check, &events);
        assert_eq!(warnings.len(), 1, "{warnings:?}");
        crate::eventemit::inherit_series_answers(&check, &events, &mut ledger, TODAY);
        assert_eq!(lines_for(&check, "lx:9:2"), [carry_line(&events[1], "obligation", ID_A)]);
        assert_eq!(ledger["lx:9:2"], load_ledger(&check, None)["lx:9:2"]);
        assert_eq!(ledger["lx:9:2"].answered_by, CARRY_ACTOR);
    }

    #[test]
    fn a_second_run_writes_nothing() {
        let (vault, events) = pq3_vault("pq3-second-run");
        let (lines, _, _) = carry(&vault, &events);
        assert_eq!(lines, 3, "the first run's lines");
        let (ledger, journal) = (ledger_text(&vault), journal_text(&vault));
        let (lines, warnings, _) = carry(&vault, &events);
        assert_eq!((lines, warnings), (0, Vec::<String>::new()));
        assert_eq!((ledger_text(&vault), journal_text(&vault)), (ledger.clone(), journal));

        // The student deletes `:2`'s commitment: a third call rebooks nothing and writes no line.
        let note = format!("commitments/{}", notes(&vault, crate::commitments::FOLDER)[0].0);
        crate::write::delete(&vault, &note, &student(&vault), &mut Journal::new(&vault)).unwrap();
        let after_delete = journal_text(&vault);
        let (lines, warnings, _) = carry(&vault, &events);
        assert_eq!((lines, warnings), (0, Vec::<String>::new()));
        assert_eq!(booked(&vault), Vec::<(String, String)>::new(), "nothing is rebooked");
        assert_eq!((ledger_text(&vault), journal_text(&vault)), (ledger, after_delete));
    }

    #[test]
    fn an_accepted_event_check_series_gets_the_carrys_line() {
        let (vault, events) = check_vault("pq3-check-series");
        let (lines, warnings, ledger) = carry(&vault, &events);
        assert_eq!((lines, warnings), (1, Vec::<String>::new()));
        assert_eq!(carry_lines(&vault), [carry_line(&events[1], "obligation", ID_A)], "P7: obligation; from: the card");
        assert_eq!(booked(&vault), [row("lx:9:2", "hard")], "beside its commitment");
        assert_eq!(ledger["lx:9:2"].verdict.as_deref(), Some("obligation"));
        assert_eq!(ledger["lx:9:2"].carry, the_carry(ID_A, &events[1]));
        assert!(!ledger.contains_key("lx:9:1"), "the card listed it: the settlement's, never the carry's");
        assert_eq!(ledger, load_ledger(&vault, None));
    }

    #[test]
    fn the_carry_never_writes_over_a_human_answer_or_a_decline() {
        let vault = tmp("pq3-human-first");
        answered_card(&vault, "event-weekly-meeting-2026-10-06.md", "executed", "opportunity", "lx:77", &["lx:77:1"], ID_A);
        answered_card(&vault, "event-film-night-2026-10-08.md", "rejected", "opportunity", "lx:78", &["lx:78:1"], ID_B);
        let human = crate::journal::read_human_actor(&vault).unwrap();
        let before = date(2026, 9, 30);
        record_answer(&vault, "lx:77:2", "Weekly meeting", before, "obligation", human, None).unwrap();
        record_answer(&vault, "lx:77:3", "Weekly meeting", before, "drop", human, None).unwrap();
        crate::eventledger::record_declined(&vault, "lx:77:4", before).unwrap();
        let on = |uid: &str, series: &str, day: i8| instance(uid, series, at(10, day, 19, 0), at(10, day, 21, 0));
        let events = [
            on("lx:77:2", "lx:77", 13),
            on("lx:77:3", "lx:77", 20),
            on("lx:77:4", "lx:77", 27),
            on("lx:77:5", "lx:77", 28), // the control
            on("lx:78:2", "lx:78", 15),
            on("lx:78:3", "lx:78", 22),
        ];
        let (_, warnings, ledger) = carry(&vault, &events);
        assert_eq!(warnings, Vec::<String>::new());
        assert_eq!(carry_lines(&vault), [carry_line(&events[3], "opportunity", ID_A)], "the control's line alone");
        for (uid, verdict) in [("lx:77:2", "obligation"), ("lx:77:3", "drop")] {
            assert_eq!(ledger[uid].verdict.as_deref(), Some(verdict), "{uid}");
            assert_eq!((ledger[uid].answered_by.as_str(), &ledger[uid].carry), (human, &None), "{uid}: the student's");
        }
        assert_eq!((ledger["lx:77:4"].verdict.clone(), ledger["lx:77:4"].declined), (None, true));
        for uid in ["lx:78:2", "lx:78:3"] {
            assert_eq!(lines_for(&vault, uid), [format!("- {uid} · {DECLINED_ON}")], "{uid}: declined, no answer line");
        }
        assert_eq!(ledger, load_ledger(&vault, None));
    }

    #[test]
    fn a_confidently_judged_carried_date_gets_its_line_and_keeps_its_verdict() {
        let (vault, events) = pq3_vault("pq3-confident");
        record_verdict(&vault, "lx:77:2", "Weekly meeting", date(2026, 9, 30), "obligation", "", "", "").unwrap();
        let (_, warnings, ledger) = carry(&vault, &events);
        assert_eq!(warnings, Vec::<String>::new());
        let lines = lines_for(&vault, "lx:77:2");
        assert_eq!(lines.len(), 2, "the judged line, then the carry's one line: {lines:?}");
        assert_eq!(lines[1], carry_line(&events[2], "opportunity", ID_A));
        let read = load_ledger(&vault, None);
        let entry = &read["lx:77:2"];
        assert_eq!(entry.verdict.as_deref(), Some("obligation"), "a confident machine verdict is never flipped");
        assert_eq!(entry.answered_by, "", "the carry did not set the verdict");
        assert_eq!(entry.carry, the_carry(ID_A, &events[2]), "but its card and span are recorded");
        assert_eq!(ledger, read);
    }

    fn copy_tree(from: &Path, to: &Path) {
        fs::create_dir_all(to).unwrap();
        for entry in fs::read_dir(from).unwrap().map(Result::unwrap) {
            let target = to.join(entry.file_name());
            if entry.file_type().unwrap().is_dir() {
                copy_tree(&entry.path(), &target);
            } else {
                fs::copy(entry.path(), &target).unwrap();
            }
        }
    }

    fn ledger_bytes(vault: &Path) -> Option<Vec<u8>> {
        fs::read(vault.join("state").join("events-seen.md")).ok()
    }

    #[test]
    fn an_existing_vaults_ledger_bytes_are_unchanged() {
        // `vault-full`, copied first and never used in place. Its events as the roster reads them
        // back, and the same events as a feed gives them.
        let fixture = Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/vault-full");
        let full = tmp("pq3-vault-full");
        copy_tree(&fixture, &full);
        let frozen = ledger_bytes(&fixture);
        assert!(frozen.is_some(), "vault-full has a ledger");
        let roster = crate::eventroster::read_roster(&full.join("state").join("events.md"));
        assert_eq!(roster.len(), 3, "vault-full's three events");
        let fetched = roster.iter().map(|e| DiscoveredEvent { source: "localist".into(), ..e.clone() });
        let events: Vec<DiscoveredEvent> = roster.iter().cloned().chain(fetched).collect();
        let (lines, warnings, _) = carry(&full, &events);
        assert_eq!((lines, warnings), (0, Vec::<String>::new()));
        assert_eq!(ledger_bytes(&full), frozen);

        // An accepted series whose every instance is on its card: no ledger, then one with lines.
        let listed = tmp("pq3-all-listed");
        answered_card(&listed, "event-weekly-meeting-2026-10-06.md", "executed", "obligation", "lx:77", &["lx:77:1", "lx:77:2"], ID_A);
        let events = [
            instance("lx:77:1", "lx:77", at(10, 6, 19, 0), at(10, 6, 21, 0)),
            instance("lx:77:2", "lx:77", at(10, 13, 0, 0), at(10, 14, 0, 0)),
        ];
        assert_eq!(carry(&listed, &events).0, 0);
        assert_eq!(ledger_bytes(&listed), None, "still absent");
        for event in &events {
            record_verdict(&listed, &event.uid, &event.title, date(2026, 9, 24), "obligation", "", "", "").unwrap();
            crate::eventledger::record_proposed(&listed, &event.uid, date(2026, 9, 24)).unwrap();
        }
        let seeded = ledger_bytes(&listed);
        assert_eq!(carry(&listed, &events).0, 0);
        assert_eq!(ledger_bytes(&listed), seeded, "byte-identical");
    }

    #[test]
    fn a_card_without_an_id_books_but_writes_no_line() {
        let vault = tmp("pq3-no-id");
        let name = "event-weekly-meeting-2026-10-06.md";
        answered_card(&vault, name, "executed", "obligation", "lx:77", &["lx:77:1"], ID_A);
        let path = vault.join("archive").join(name);
        let text = pystr::read_text(&path).unwrap().replace(&format!("id: {ID_A}\n"), "");
        pystr::write_text(&path, &text).unwrap();
        let on = |uid: &str, day: i8| instance(uid, "lx:77", at(10, day, 19, 0), at(10, day, 21, 0));
        let events = [on("lx:77:1", 6), on("lx:77:2", 13), on("lx:77:3", 20)];
        let (lines, warnings, ledger) = carry(&vault, &events);
        let warned = format!("carry: archive/{name} has no id; its dates are booked but not listed");
        assert_eq!((lines, warnings), (0, vec![warned]), "one warning names the card");
        let mut rows = booked(&vault); // file-name order: `weekly-meeting-2.md` sorts first
        rows.sort();
        assert_eq!(rows, [row("lx:77:2", "hard"), row("lx:77:3", "hard")], "the dates are booked");
        assert_eq!((ledger_text(&vault), ledger.len()), (None, 0), "no line");

        // An `event-check` card whose `id:` `ids::is_id` refuses: the same.
        let check = tmp("pq3-check-bad-id");
        let first = instance("lx:9:1", "lx:9", at(10, 6, 19, 0), at(10, 6, 21, 0));
        check_card(&check, "event-check-weekly-a.md", "executed", "lx:9", &[&first], true, "not-an-id");
        let events = [first.clone(), instance("lx:9:2", "lx:9", at(10, 13, 0, 0), at(10, 14, 0, 0))];
        let (lines, warnings, _) = carry(&check, &events);
        let warned = "carry: archive/event-check-weekly-a.md has no id; its dates are booked but not listed";
        assert_eq!((lines, warnings), (0, vec![warned.to_string()]));
        assert_eq!(ledger_text(&check), None);
    }

    #[test]
    fn the_carrys_lines_are_deterministic() {
        // P14: two runs over copies of one vault, the events given in opposite orders. Two series
        // (one per card kind) share a start, so the uid breaks the tie.
        let (vault, mut events) = pq3_vault("pq3-determinism-a");
        let first = instance("lx:9:1", "lx:9", at(10, 6, 19, 0), at(10, 6, 21, 0));
        check_card(&vault, "event-check-weekly-a.md", "executed", "lx:9", &[&first], true, ID_B);
        events.extend([
            first,
            instance("lx:9:2", "lx:9", at(10, 13, 19, 0), at(10, 13, 21, 0)),
            instance("lx:9:3", "lx:9", at(10, 20, 0, 0), at(10, 21, 0, 0)),
        ]);
        let copy = tmp("pq3-determinism-b");
        copy_tree(&vault, &copy);
        carry(&vault, &events);
        let reversed: Vec<DiscoveredEvent> = events.iter().rev().cloned().collect();
        carry(&copy, &reversed);
        assert_eq!(carry_lines(&vault).len(), 5, "three dates of lx:77 and two of lx:9");
        assert_eq!(ledger_bytes(&vault), ledger_bytes(&copy));
    }

    #[test]
    fn a_uid_the_ledger_cannot_read_back_gets_no_carry_line() {
        // Written anyway, such a line would be written again on every `rank` (as `decline`'s rule):
        // a uid outside the ledger's class, or a span end in a negative year (`-0001-…` is no
        // `\d{4}` year, so the line reads as no carry line). `verdict:` inside a uid does not break
        // an answer line, so that one is carried.
        let vault = tmp("pq3-unreadable-uid");
        answered_card(&vault, "event-weekly-meeting-2026-10-06.md", "executed", "opportunity", "lx:77", &["lx:77:1"], ID_A);
        let all_day = |uid: &str, day: i8| instance(uid, "lx:77", at(10, day, 0, 0), at(10, day + 1, 0, 0));
        let broken_end = instance("lx:77:9", "lx:77", at(10, 27, 19, 0), date(-1, 1, 1).at(0, 0, 0, 0));
        let events = [all_day("lx:77 bad", 13), all_day("lx:77:verdict:2", 20), broken_end];
        let warned: Vec<String> = ["lx:77 bad", "lx:77:9"]
            .iter()
            .map(|uid| format!("carry: no line for {uid:?}: the ledger cannot read it back"))
            .collect();
        let (lines, warnings, ledger) = carry(&vault, &events);
        assert_eq!((lines, &warnings), (1, &warned));
        assert_eq!(carry_lines(&vault), [carry_line(&events[1], "opportunity", ID_A)]);
        assert_eq!(ledger["lx:77:verdict:2"].carry, the_carry(ID_A, &events[1]));
        let before = ledger_text(&vault);
        let (lines, warnings, _) = carry(&vault, &events);
        assert_eq!((lines, &warnings), (0, &warned));
        assert_eq!(ledger_text(&vault), before);
    }
}
