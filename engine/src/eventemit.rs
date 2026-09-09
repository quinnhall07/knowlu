//! The events digest — port of `engine/eventemit.py`.
//!
//! Wave 5 of the Rust port. One note per day in `approvals/`, holding N checkbox events that
//! `approvals::expand_digest` turns into calendar-event notes on approval.
//!
//! # Three properties this module exists to hold
//!
//! - **One digest per day.** If today's file already exists, emit nothing — a re-run must never
//!   clobber boxes Quinn has already ticked.
//! - **The budget is a real ceiling.** `emit_digest` is handed the remaining daily approval
//!   budget and sizes itself, which is *why* `approvals::defer_over_budget` excludes digests from
//!   its candidates. Port both halves or neither; half of this is a budget hole.
//! - **Write the digest, THEN record `proposed`.** An interruption between the two leaves events
//!   in a live digest with no ledger line, and tomorrow's digest has a different filename — so
//!   nothing would stop them being re-proposed into a second live digest.
//!   [`pending_digest_uids`] reads the payload back to close that window.
//!
//! **Preserved defect 15 lives at the other end of this module.** `approvals::calendar_note`
//! writes `start`/`end` unquoted, so a payload value containing `: ` strands the digest forever.
//! [`payload_block`] formats both from real datetimes, which is exactly why that defect is
//! unreachable today. Keep formatting them; do not start quoting them, and do not fix the
//! consumer from here.

use std::collections::{BTreeMap, BTreeSet};
use std::path::{Path, PathBuf};

use jiff::civil::{Date, Time};
use jiff::Span;

use crate::eventledger::{record_proposed, LedgerEntry};
use crate::events::{DiscoveredEvent, EventsConfig};
use crate::journal::Journal;
use crate::models::split_frontmatter;
use crate::pystr;
use crate::write::{create, WriteContext};

/// `strong` first, then `mild`, then anything unrated.
fn strength_order(strength: &str) -> i64 {
    match strength {
        "strong" => 0,
        "mild" => 1,
        _ => 2,
    }
}

pub const SETTLED_STATUSES: [&str; 3] = ["executed", "rejected", "expired"];

pub const BUTTONS: &str = "```meta-bind-button\nlabel: Approve\nstyle: primary\naction:\n  type: updateMetadata\n  bindTarget: status\n  evaluate: false\n  value: approved\n```\n\n```meta-bind-button\nlabel: Reject\nstyle: destructive\naction:\n  type: updateMetadata\n  bindTarget: status\n  evaluate: false\n  value: rejected\n```";

fn minus_days(d: Date, days: i64) -> Date {
    Span::new()
        .try_days(days)
        .ok()
        .and_then(|s| d.checked_sub(s).ok())
        .unwrap_or(d)
}

/// The day this event becomes eligible.
fn horizon_start(event: &DiscoveredEvent, config: &EventsConfig) -> Date {
    let mut opens = minus_days(event.start().date(), config.propose_horizon_days);
    if event.registration {
        if let Some(deadline) = event.registration_deadline {
            let three_before = minus_days(deadline, 3);
            if three_before < opens {
                opens = three_before;
            }
        }
    }
    opens
}

/// Uids already sitting in a live digest in `approvals/`.
///
/// Defensive throughout: a malformed digest must never fail the run.
pub fn pending_digest_uids(vault: &Path) -> BTreeSet<String> {
    let folder = vault.join("approvals");
    let mut uids = BTreeSet::new();
    if !folder.is_dir() {
        return uids;
    }
    let Ok(entries) = std::fs::read_dir(&folder) else {
        return uids;
    };
    let mut paths: Vec<PathBuf> = entries
        .filter_map(|e| e.ok())
        .map(|e| e.path())
        .filter(|p| {
            p.file_name()
                .and_then(|n| n.to_str())
                .is_some_and(|n| n.starts_with("events-digest-") && n.ends_with(".md"))
        })
        .collect();
    paths.sort();
    for path in paths {
        let Ok(text) = pystr::read_text(&path) else { continue };
        let Ok((meta, _)) = split_frontmatter(&text) else { continue };
        let status = crate::yaml::opt_text(crate::yaml::get(&meta, "status")).unwrap_or_default();
        if SETTLED_STATUSES.contains(&pystr::strip(&status)) {
            continue;
        }
        let Some(serde_yaml_ng::Value::Sequence(payload)) = crate::yaml::get(&meta, "events")
        else {
            continue;
        };
        for entry in payload {
            let Some(map) = entry.as_mapping() else { continue };
            let uid = crate::yaml::opt_text(crate::yaml::get(map, "uid")).unwrap_or_default();
            let uid = pystr::strip(&uid);
            if !uid.is_empty() {
                uids.insert(uid.to_string());
            }
        }
    }
    uids
}

/// Judged-relevant, unproposed, undeclined, and inside the horizon.
pub fn eligible_events(
    events: &[DiscoveredEvent],
    ledger: &BTreeMap<String, LedgerEntry>,
    config: &EventsConfig,
    today: Date,
    pending_uids: &BTreeSet<String>,
) -> Vec<DiscoveredEvent> {
    let now = today.to_datetime(Time::midnight());
    let mut out = Vec::new();
    for event in events {
        let Some(entry) = ledger.get(&event.uid) else { continue };
        if entry.verdict.as_deref() != Some("opportunity") {
            continue;
        }
        if entry.proposed || entry.declined {
            continue;
        }
        if pending_uids.contains(&event.uid) {
            continue;
        }
        if event.start() < now {
            continue;
        }
        if horizon_start(event, config) > today {
            continue;
        }
        out.push(event.clone());
    }
    out
}

/// `(strength, registration deadline, start, uid)` — the uid tiebreak is what makes it total.
fn sort_key(
    event: &DiscoveredEvent,
    ledger: &BTreeMap<String, LedgerEntry>,
) -> (i64, Date, jiff::civil::DateTime, String) {
    let strength = ledger
        .get(&event.uid)
        .map(|e| strength_order(&e.strength))
        .unwrap_or(2);
    let deadline = event.registration_deadline.unwrap_or(Date::MAX);
    (strength, deadline, event.start(), event.uid.clone())
}

/// Target first; then urgent overflow only, up to the ceiling.
///
/// `budget` is the global daily approval budget left after Gmail has taken its share. `None` skips
/// the `min(ceiling, budget)` clamp, but the target-vs-ceiling reconciliation and the
/// `ceiling <= 0` early return still apply — so `None` is not simply the old pre-budget behaviour
/// whenever `daily_proposal_ceiling < daily_proposal_target`.
pub fn select_for_digest(
    eligible: &[DiscoveredEvent],
    ledger: &BTreeMap<String, LedgerEntry>,
    config: &EventsConfig,
    today: Date,
    budget: Option<i64>,
) -> Vec<DiscoveredEvent> {
    let mut ceiling = config.daily_proposal_ceiling;
    if let Some(b) = budget {
        ceiling = ceiling.min(b.max(0));
    }
    if ceiling <= 0 {
        return Vec::new();
    }
    let mut ordered = eligible.to_vec();
    ordered.sort_by_key(|e| sort_key(e, ledger));

    // The target is a floor on a quiet day, never a licence to exceed the budget.
    let target = config.daily_proposal_target.min(ceiling).max(0) as usize;
    let mut chosen: Vec<DiscoveredEvent> = ordered.iter().take(target).cloned().collect();
    let urgency_end = today
        .checked_add(
            Span::new()
                .try_days(config.urgency_window_days)
                .unwrap_or_default(),
        )
        .unwrap_or(today)
        .to_datetime(Time::midnight());
    for event in ordered.iter().skip(target) {
        if chosen.len() as i64 >= ceiling {
            break;
        }
        if event.start() < urgency_end {
            chosen.push(event.clone());
        }
    }
    chosen
}

/// Quote a value for a YAML double-quoted scalar, escaping what would break it.
///
/// Feed-sourced strings (event titles, locations) come from external venues and carry whatever
/// punctuation they typed — including literal double quotes. An unescaped one corrupts the
/// frontmatter and silently drops the whole digest from being parsed.
fn yaml_str(value: &str) -> String {
    let text = value
        .replace('\\', "\\\\")
        .replace('"', "\\\"")
        .replace('\n', " ")
        .replace('\r', " ");
    format!("\"{text}\"")
}

fn payload_block(events: &[DiscoveredEvent]) -> String {
    let mut lines = vec!["events:".to_string()];
    for event in events {
        // The uid goes through the same escaper as title and location. It is safe today only
        // because the ledger's charclass rejects the dangerous characters; routing it here means
        // widening that charclass can never corrupt a digest.
        lines.push(format!("  - uid: {}", yaml_str(&event.uid)));
        lines.push(format!("    summary: {}", yaml_str(&event.title)));
        lines.push(format!(
            "    start: {}",
            event.start().strftime("%Y-%m-%dT%H:%M")
        ));
        lines.push(format!("    end: {}", event.end().strftime("%Y-%m-%dT%H:%M")));
        lines.push(format!("    location: {}", yaml_str(&event.location)));
    }
    lines.join("\n")
}

/// Write today's digest, if anything qualifies. Returns `(path, count)`.
pub fn emit_digest(
    vault: &Path,
    events: &[DiscoveredEvent],
    ledger: &BTreeMap<String, LedgerEntry>,
    config: &EventsConfig,
    today: Date,
    budget: Option<i64>,
    ctx: Option<&WriteContext>,
    journal: &mut Journal,
) -> (Option<PathBuf>, usize) {
    let default_ctx = WriteContext::new("agent:events", "cli");
    let ctx = ctx.unwrap_or(&default_ctx);

    let pending = pending_digest_uids(vault);
    let eligible = eligible_events(events, ledger, config, today, &pending);
    let chosen = select_for_digest(&eligible, ledger, config, today, budget);
    if chosen.is_empty() {
        return (None, 0);
    }
    let folder = vault.join("approvals");
    if std::fs::create_dir_all(&folder).is_err() {
        return (None, 0);
    }
    let path = folder.join(format!("events-digest-{}.md", today.strftime("%Y-%m-%d")));
    if path.exists() {
        // One digest per day; a re-run must not clobber ticked boxes.
        return (None, 0);
    }
    let Some(earliest) = chosen.iter().map(|e| e.start().date()).min() else {
        return (None, 0);
    };

    let plural = if chosen.len() != 1 { "s" } else { "" };
    let mut body = vec![
        format!(
            "**{} event{plural} worth a look.** Tick the ones you want on your calendar, then \
             Approve. Anything left unticked is declined.",
            chosen.len()
        ),
        String::new(),
    ];
    for event in &chosen {
        let entry = ledger.get(&event.uid);
        let detail = [event.location.as_str(), event.organizer.as_str()]
            .iter()
            .filter(|b| !b.is_empty())
            .cloned()
            .collect::<Vec<_>>()
            .join(" · ");
        // `%-m/%-d` is not portable to Windows; build the stamp arithmetically.
        let start = event.start();
        let stamp = format!(
            "{} {}/{} {}",
            start.strftime("%a"),
            start.month(),
            start.day(),
            start.strftime("%H:%M")
        );
        let mut line = format!("- [ ] {stamp} · {}", event.title);
        if !detail.is_empty() {
            line.push_str(&format!(" · {detail}"));
        }
        line.push_str(&format!(" · `{}`", event.uid));
        body.push(line);
        if let Some(entry) = entry {
            if !entry.why.is_empty() {
                let strength = if entry.strength.is_empty() { "match" } else { &entry.strength };
                body.push(format!("      {strength}: {}", entry.why));
            }
        }
    }
    body.push(String::new());
    body.push(BUTTONS.to_string());
    body.push(String::new());

    let note = format!(
        "---\ntype: approval\nkind: events-digest\ntitle: {}\nstatus: pending\nproposed_at: {}\nfirst_proposed_at: {}\nexpires: {}\nsnooze_until: null\ncreated_by: events\n{}\n---\n\n{}",
        yaml_str(&format!("Events — {}", today.strftime("%Y-%m-%d"))),
        today.strftime("%Y-%m-%d"),
        today.strftime("%Y-%m-%d"),
        earliest.strftime("%Y-%m-%d"),
        payload_block(&chosen),
        body.join("\n")
    );
    let rel = format!("approvals/events-digest-{}.md", today.strftime("%Y-%m-%d"));
    if create(vault, &rel, &note, ctx, journal, None).is_err() {
        return (None, 0);
    }
    // Digest first, ledger second — see the module docs for why the order is load-bearing.
    for event in &chosen {
        let _ = record_proposed(vault, &event.uid, today);
    }
    (Some(path), chosen.len())
}

#[cfg(test)]
mod tests {
    //! Direct port of `tests/test_event_emission.py` — all 24 tests, same names.

    use super::*;
    use crate::eventledger::{load_ledger, record_verdict};
    use jiff::civil::date;
    use std::fs;

    const TODAY: Date = Date::constant(2026, 8, 20);
    const QUOTED_TITLE: &str = "\"The Domestic Liminal\" Exhibition";

    fn config() -> EventsConfig {
        EventsConfig::default()
    }

    /// `event(uid, day, title=None, month=8, hour=18, location="")`
    fn event_at(uid: &str, day: i8, month: i8, title: Option<&str>, location: &str) -> DiscoveredEvent {
        DiscoveredEvent {
            uid: uid.to_string(),
            title: title.map(str::to_string).unwrap_or_else(|| format!("Event {uid}")),
            start: Some(date(2026, month, day).at(18, 0, 0, 0)),
            end: Some(date(2026, month, day).at(19, 0, 0, 0)),
            source: "campus".into(),
            location: location.to_string(),
            ..Default::default()
        }
        .normalized()
    }

    fn event(uid: &str, day: i8) -> DiscoveredEvent {
        event_at(uid, day, 8, None, "")
    }

    /// `opp(uid, strength="mild")`
    fn opp(uid: &str, strength: &str) -> LedgerEntry {
        LedgerEntry {
            uid: uid.to_string(),
            verdict: Some("opportunity".into()),
            strength: strength.to_string(),
            why: format!("because {uid}"),
            ..Default::default()
        }
    }

    fn ledger_of(entries: Vec<LedgerEntry>) -> BTreeMap<String, LedgerEntry> {
        entries.into_iter().map(|e| (e.uid.clone(), e)).collect()
    }

    fn eligible(
        events: &[DiscoveredEvent],
        ledger: &BTreeMap<String, LedgerEntry>,
    ) -> Vec<DiscoveredEvent> {
        eligible_events(events, ledger, &config(), TODAY, &BTreeSet::new())
    }

    fn select(
        events: &[DiscoveredEvent],
        ledger: &BTreeMap<String, LedgerEntry>,
        budget: Option<i64>,
    ) -> Vec<DiscoveredEvent> {
        select_for_digest(&eligible(events, ledger), ledger, &config(), TODAY, budget)
    }

    fn tmp(name: &str) -> PathBuf {
        let dir = std::env::temp_dir().join(format!("qo-emit-{}-{}", name, std::process::id()));
        let _ = fs::remove_dir_all(&dir);
        fs::create_dir_all(&dir).unwrap();
        dir
    }

    fn emit(
        vault: &Path,
        events: &[DiscoveredEvent],
        ledger: &BTreeMap<String, LedgerEntry>,
        budget: Option<i64>,
    ) -> (Option<PathBuf>, usize) {
        let mut journal = Journal::new(vault);
        emit_digest(vault, events, ledger, &config(), TODAY, budget, None, &mut journal)
    }

    fn uids(events: &[DiscoveredEvent]) -> Vec<&str> {
        events.iter().map(|e| e.uid.as_str()).collect()
    }

    // --- eligibility --------------------------------------------------------------------

    #[test]
    fn only_opportunities_inside_the_horizon_are_eligible() {
        let events = [
            event("in", 28),                       // 8 days out — inside the 14-day horizon
            event_at("far", 30, 9, None, ""),      // 41 days out — beyond it
            event("obl", 28),
            event("dropped", 28),
        ];
        let ledger = ledger_of(vec![
            opp("in", "mild"),
            opp("far", "mild"),
            LedgerEntry { uid: "obl".into(), verdict: Some("obligation".into()), ..Default::default() },
            LedgerEntry { uid: "dropped".into(), verdict: Some("drop".into()), ..Default::default() },
        ]);
        assert_eq!(uids(&eligible(&events, &ledger)), vec!["in"]);
    }

    #[test]
    fn already_proposed_or_declined_are_not_eligible() {
        let events = [event("a", 25), event("b", 25)];
        let ledger = ledger_of(vec![
            LedgerEntry { uid: "a".into(), verdict: Some("opportunity".into()), proposed: true, ..Default::default() },
            LedgerEntry { uid: "b".into(), verdict: Some("opportunity".into()), declined: true, ..Default::default() },
        ]);
        assert!(eligible(&events, &ledger).is_empty());
    }

    #[test]
    fn past_events_are_never_eligible() {
        let ledger = ledger_of(vec![opp("old", "mild")]);
        assert!(eligible(&[event("old", 10)], &ledger).is_empty());
    }

    // --- selection ----------------------------------------------------------------------

    fn many(prefix: &str, day: i8, n: usize) -> (Vec<DiscoveredEvent>, BTreeMap<String, LedgerEntry>) {
        let events: Vec<DiscoveredEvent> =
            (0..n).map(|i| event(&format!("{prefix}{i}"), day)).collect();
        let ledger = ledger_of(
            (0..n).map(|i| opp(&format!("{prefix}{i}"), "mild")).collect(),
        );
        (events, ledger)
    }

    #[test]
    fn target_caps_a_quiet_day_at_five() {
        let (events, ledger) = many("e", 30, 9); // 10 days out, none urgent
        assert_eq!(select(&events, &ledger, None).len(), 5);
    }

    #[test]
    fn urgent_events_overflow_past_the_target_up_to_the_ceiling() {
        // 20 events all starting within 7 days: 5 by target, the rest by urgency, capped at 15.
        let (events, ledger) = many("u", 24, 20);
        assert_eq!(select(&events, &ledger, None).len(), 15);
    }

    #[test]
    fn non_urgent_events_never_overflow() {
        let (events, ledger) = many("n", 31, 20); // 11 days out, outside urgency
        assert_eq!(select(&events, &ledger, None).len(), 5);
    }

    #[test]
    fn strong_beats_mild_then_soonest_start() {
        let events = [event("mild-soon", 22), event("strong-late", 29)];
        let ledger = ledger_of(vec![opp("mild-soon", "mild"), opp("strong-late", "strong")]);
        assert_eq!(
            uids(&select(&events, &ledger, None)),
            vec!["strong-late", "mild-soon"]
        );
    }

    // --- emit ---------------------------------------------------------------------------

    #[test]
    fn emit_writes_a_digest_and_records_proposed() {
        let vault = tmp("writes");
        let events = [event_at("engage:1", 25, 8, Some("AI Club Kickoff"), "")];
        let ledger = ledger_of(vec![opp("engage:1", "strong")]);
        let (path, count) = emit(&vault, &events, &ledger, None);
        assert_eq!(count, 1);
        let path = path.unwrap();
        assert_eq!(path.file_name().unwrap(), "events-digest-2026-08-20.md");
        let text = pystr::read_text(&path).unwrap();
        assert!(text.contains("kind: events-digest"));
        assert!(text.contains("- [ ] "), "unchecked by default");
        assert!(text.contains("AI Club Kickoff"));
        assert!(text.contains("engage:1"));
        assert!(text.contains("because engage:1"));
        assert!(load_ledger(&vault, None)["engage:1"].proposed);
    }

    #[test]
    fn emit_digest_is_a_journaled_create_with_id() {
        // Spec §3: every note write goes through engine.write. emit_digest used to write the
        // digest with a raw write_text, so it had no id and no `create` record — detect_external
        // would later attribute all of its fields to Quinn.
        let vault = tmp("journaled");
        let events = [event_at("engage:1", 25, 8, Some("AI Club Kickoff"), "")];
        let ledger = ledger_of(vec![opp("engage:1", "strong")]);
        let (path, count) = emit(&vault, &events, &ledger, None);
        assert_eq!(count, 1);
        let text = pystr::read_text(&path.unwrap()).unwrap();
        let (meta, _) = split_frontmatter(&text).unwrap();
        let id = crate::yaml::opt_text(crate::yaml::get(&meta, "id")).unwrap_or_default();
        assert!(crate::ids::ID_RE.is_match(&id), "id was {id:?}");

        let mut creates = 0;
        let dir = vault.join("state").join("journal");
        for entry in fs::read_dir(&dir).unwrap().filter_map(|e| e.ok()) {
            let text = pystr::read_text(&entry.path()).unwrap();
            for line in pystr::splitlines(&text) {
                if line.trim().is_empty() {
                    continue;
                }
                let rec: serde_json::Value = serde_json::from_str(line).unwrap();
                let path = rec.get("path").and_then(|p| p.as_str()).unwrap_or("");
                if rec.get("op").and_then(|o| o.as_str()) == Some("create")
                    && path.starts_with("approvals/events-digest-")
                {
                    creates += 1;
                    assert_eq!(rec.get("actor").and_then(|a| a.as_str()), Some("agent:events"));
                }
            }
        }
        assert_eq!(creates, 1);
    }

    #[test]
    fn digest_expires_on_the_earliest_event() {
        let vault = tmp("expires");
        let events = [event("a", 25), event("b", 22)];
        let ledger = ledger_of(vec![opp("a", "mild"), opp("b", "mild")]);
        let (path, _) = emit(&vault, &events, &ledger, None);
        let text = pystr::read_text(&path.unwrap()).unwrap();
        assert!(text.contains("expires: 2026-08-22"));
    }

    #[test]
    fn emitting_nothing_writes_no_note() {
        let vault = tmp("nothing");
        let (path, count) = emit(&vault, &[], &BTreeMap::new(), None);
        assert!(path.is_none());
        assert_eq!(count, 0);
        assert!(!vault.join("approvals").exists());
    }

    #[test]
    fn a_second_emit_on_the_same_day_is_a_noop() {
        let vault = tmp("second");
        let events = [event("a", 25)];
        // Persist the verdict for real so it survives a reload — an in-memory-only ledger would
        // make the event drop out on `verdict != "opportunity"` before the once-only guard is
        // ever reached, which would test nothing.
        record_verdict(&vault, "a", "Event a", TODAY, "opportunity", "", "because a", "").unwrap();
        let ledger = load_ledger(&vault, None);
        let (path, count) = emit(&vault, &events, &ledger, None);
        assert_eq!(count, 1);
        assert!(path.is_some());

        let reloaded = load_ledger(&vault, None);
        let (path2, count2) = emit(&vault, &events, &reloaded, None);
        assert!(path2.is_none());
        assert_eq!(count2, 0);

        // Ledger writes are irreversible appends: a broken guard shows up here as two `proposed`
        // lines, not just as a wrong return value.
        let seen = pystr::read_text(&vault.join("state").join("events-seen.md")).unwrap();
        assert_eq!(seen.matches("- a · proposed").count(), 1);
    }

    // --- pending_digest_uids ------------------------------------------------------------

    fn write_digest(vault: &Path, uid: &str, name: &str, status: &str) {
        fs::create_dir_all(vault.join("approvals")).unwrap();
        let text = format!(
            "---\ntype: approval\nkind: events-digest\ntitle: \"Events — 2026-08-19\"\nstatus: {status}\nproposed_at: 2026-08-19\nexpires: 2026-08-25\nsnooze_until: null\ncreated_by: events\nevents:\n  - uid: \"{uid}\"\n    summary: \"Prior\"\n    start: 2026-08-25T18:00\n    end: 2026-08-25T19:00\n    location: \"\"\n---\n\n- [ ] Tue 8/25 18:00 · Prior · `{uid}`\n"
        );
        pystr::write_text(&vault.join("approvals").join(name), &text).unwrap();
    }

    #[test]
    fn a_uid_in_a_live_digest_is_not_eligible() {
        // emit_digest writes the digest and THEN records `proposed`. An interruption between the
        // two must not let tomorrow's run propose the same event again into a second live digest.
        let vault = tmp("livedigest");
        write_digest(&vault, "a", "events-digest-2026-08-19.md", "pending");
        assert_eq!(
            pending_digest_uids(&vault),
            BTreeSet::from(["a".to_string()])
        );
        let events = [event("a", 25)];
        let ledger = ledger_of(vec![opp("a", "mild")]);
        // No `proposed` ledger line exists — exactly the crash window.
        assert_eq!(uids(&eligible(&events, &ledger)), vec!["a"]);
        let (path, count) = emit(&vault, &events, &ledger, None);
        assert!(path.is_none());
        assert_eq!(count, 0);
    }

    #[test]
    fn a_uid_in_a_settled_digest_is_still_eligible() {
        // An executed/rejected/expired digest is archived in the same pass and its uids already
        // carry a `proposed` or `declined` ledger line; it must not block.
        let vault = tmp("settled");
        write_digest(&vault, "a", "events-digest-2026-08-19.md", "executed");
        assert!(pending_digest_uids(&vault).is_empty());
    }

    #[test]
    fn a_malformed_digest_does_not_raise() {
        let vault = tmp("malformed");
        fs::create_dir_all(vault.join("approvals")).unwrap();
        pystr::write_text(
            &vault.join("approvals").join("events-digest-2026-08-19.md"),
            "---\nstatus: pending\nevents: [oops\n  - broken: \"\n---\n\nbody\n",
        )
        .unwrap();
        pystr::write_text(
            &vault.join("approvals").join("events-digest-2026-08-18.md"),
            "---\nstatus: pending\nevents: not-a-list\n---\n\nbody\n",
        )
        .unwrap();
        assert!(pending_digest_uids(&vault).is_empty());
    }

    #[test]
    fn no_approvals_folder_means_no_pending_uids() {
        let vault = tmp("nofolder");
        assert!(pending_digest_uids(&vault).is_empty());
    }

    // --- escaping -----------------------------------------------------------------------

    fn payload_field(vault: &Path, path: &Path, field: &str) -> String {
        let _ = vault;
        let text = pystr::read_text(path).unwrap();
        let (meta, _) = split_frontmatter(&text).unwrap();
        let Some(serde_yaml_ng::Value::Sequence(events)) = crate::yaml::get(&meta, "events") else {
            panic!("no events payload");
        };
        let map = events[0].as_mapping().unwrap();
        crate::yaml::opt_text(crate::yaml::get(map, field)).unwrap_or_default()
    }

    #[test]
    fn titles_with_embedded_quotes_survive_the_round_trip() {
        // Real-world case: the first event returned by the live Localist API during research was
        // titled exactly this — embedded double quotes, straight from the feed.
        let vault = tmp("quotedtitle");
        let events = [event_at("localist:1", 25, 8, Some(QUOTED_TITLE), "")];
        let ledger = ledger_of(vec![opp("localist:1", "mild")]);
        let (path, count) = emit(&vault, &events, &ledger, None);
        assert_eq!(count, 1);
        assert_eq!(payload_field(&vault, &path.unwrap(), "summary"), QUOTED_TITLE);
    }

    #[test]
    fn a_uid_with_a_quote_cannot_corrupt_the_digest() {
        // Defence in depth: uids are sanitized at mint, so this cannot arrive today — but the uid
        // must not be the one scalar written unescaped, or widening the ledger's charclass would
        // silently corrupt the whole digest's frontmatter.
        let vault = tmp("quoteduid");
        let hostile = "engage:\"1\"\\x";
        let events = [event(hostile, 25)];
        let ledger = ledger_of(vec![opp(hostile, "mild")]);
        let (path, count) = emit(&vault, &events, &ledger, None);
        assert_eq!(count, 1);
        assert_eq!(payload_field(&vault, &path.unwrap(), "uid"), hostile);
    }

    #[test]
    fn locations_with_embedded_quotes_survive_the_round_trip() {
        let vault = tmp("quotedloc");
        let events = [event_at("localist:2", 25, 8, None, "Room \"204B\"")];
        let ledger = ledger_of(vec![opp("localist:2", "mild")]);
        let (path, count) = emit(&vault, &events, &ledger, None);
        assert_eq!(count, 1);
        assert_eq!(payload_field(&vault, &path.unwrap(), "location"), "Room \"204B\"");
    }

    // --- budget -------------------------------------------------------------------------

    #[test]
    fn budget_none_keeps_the_existing_ceiling() {
        let (events, ledger) = many("u", 24, 20);
        assert_eq!(select(&events, &ledger, None).len(), 15);
    }

    #[test]
    fn budget_below_the_ceiling_clamps_the_digest() {
        let (events, ledger) = many("u", 24, 20);
        assert_eq!(select(&events, &ledger, Some(4)).len(), 4);
    }

    #[test]
    fn budget_below_the_target_clamps_the_target_too() {
        // A quiet day would normally take 5 by target alone. Budget 2 must beat it.
        let (events, ledger) = many("e", 30, 9);
        assert_eq!(select(&events, &ledger, Some(2)).len(), 2);
    }

    #[test]
    fn exhausted_budget_emits_nothing() {
        let (events, ledger) = many("u", 24, 20);
        assert!(select(&events, &ledger, Some(0)).is_empty());
    }

    #[test]
    fn emit_digest_writes_nothing_when_budget_is_zero() {
        let vault = tmp("zerobudget");
        let (events, ledger) = many("u", 24, 3);
        let (path, emitted) = emit(&vault, &events, &ledger, Some(0));
        assert!(path.is_none());
        assert_eq!(emitted, 0);
    }
}
