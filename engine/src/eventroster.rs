//! `state/events.md` — the roster, and the audit trail beneath it. Port of `engine/eventroster.py`.
//!
//! Wave 5 of the Rust port.
//!
//! # Why both sections exist
//!
//! - **Coming up** is what reaches `today.md`. Bounded by `roster_window_days` at the top and by
//!   *today* at the bottom — the lower bound matters because on the all-feeds-down path `cli.run`
//!   re-reads this file into itself, and without it a past event would persist under "Coming up"
//!   forever.
//! - **Everything else** is the audit trail. Spec §5 calls it *"the mechanism by which a wrong
//!   drop is recoverable by reading rather than by missing"*, so pre-filter drops land here too,
//!   marked `· filtered`. A drop that reaches no file leaves no trace anywhere.
//!
//! The two bounds are deliberately asymmetric: **unjudged** events get the full roster window
//! because this file is the routine's only judging input, while judged and pre-filtered entries
//! keep the tight `audit_window_days` bound — they are here to be audited, not read, and the
//! 90-day set would run ~700 lines.
//!
//! [`read_roster`] is the other half of the contract and is load-bearing on a bad-network
//! morning. It is lossy by design: only the fields the line carries survive.

use std::collections::BTreeMap;
use std::path::Path;
use std::sync::LazyLock;

use jiff::civil::{Date, DateTime, Time};
use jiff::Span;
use regex::Regex;

use crate::eventledger::{self, LedgerEntry};
use crate::events::{DiscoveredEvent, EventsConfig};
use crate::pystr;

pub const HEADER: &str = "# Event roster — regenerated every run. Verdicts live in \
`state/events-seen.md`; the second section is the audit trail for what the filter dropped.";

pub const RELEVANT_VERDICTS: [&str; 2] = ["obligation", "opportunity"];
pub const DESCRIPTION_CHARS: usize = 200;
/// The digest already uses a six-space continuation; match it so both files read alike.
pub const CONTINUATION: &str = "      ";

// The time separator is an EN DASH (U+2013) and the field separator a MIDDLE DOT (U+00B7).
static LINE: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(
        r"^- (?P<start>\d{2}:\d{2})–(?P<end>\d{2}:\d{2}) · \*\*(?P<title>.+?)\*\* · (?P<rest>.*)`(?P<uid>[^`]+)`$",
    )
    .unwrap()
});
static DAY: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"^### \w+ (?P<day>\d{4}-\d{2}-\d{2})$").unwrap());
static WHITESPACE: LazyLock<Regex> = LazyLock::new(|| Regex::new(r"\s+").unwrap());

/// Events whose recorded verdict makes them worth showing.
///
/// A `declined` line is terminal (spec §5): an opportunity Quinn left unticked is one he decided
/// against, so it must stop occupying a `Coming up` slot. It stays readable in the roster's audit
/// section.
pub fn relevant_events(
    events: &[DiscoveredEvent],
    ledger: &BTreeMap<String, LedgerEntry>,
) -> Vec<DiscoveredEvent> {
    let mut out: Vec<DiscoveredEvent> = events
        .iter()
        .filter(|event| match ledger.get(&event.uid) {
            Some(entry) => {
                entry
                    .verdict
                    .as_deref()
                    .is_some_and(|v| RELEVANT_VERDICTS.contains(&v))
                    && !entry.declined
            }
            None => false,
        })
        .cloned()
        .collect();
    out.sort_by(|a, b| (a.start(), &a.uid).cmp(&(b.start(), &b.uid)));
    out
}

fn detail(event: &DiscoveredEvent) -> String {
    let mut bits: Vec<String> = Vec::new();
    if !event.organizer.is_empty() {
        bits.push(event.organizer.clone());
    }
    if !event.location.is_empty() {
        bits.push(format!("@ {}", event.location));
    }
    if event.benefits.join(" ").to_lowercase().contains("free food") {
        bits.push("free food".to_string());
    }
    bits.iter().map(|b| format!("{b} · ")).collect()
}

/// One line of description, whitespace collapsed and capped.
fn excerpt(text: &str, limit: usize) -> String {
    let collapsed = WHITESPACE.replace_all(text, " ");
    let collapsed = collapsed.trim();
    if collapsed.chars().count() <= limit {
        return collapsed.to_string();
    }
    // Character-wise, like Python's slice — a byte slice would split a codepoint and panic.
    let head: String = collapsed.chars().take(limit).collect();
    format!("{}…", head.trim_end())
}

/// One audit-section entry: the primary line, plus a continuation when unjudged.
///
/// An unjudged line is the routine's judging input — spec §7 lists "the event's own metadata and
/// description" among the judge's inputs, and §7's one-verdict-forever rule makes every
/// under-informed `drop` permanent. A title like "Lunch & Learn" carries almost no signal alone.
/// Judged lines stay terse: they exist to be audited, not read.
fn audit_lines(event: &DiscoveredEvent, judged: bool, filtered: bool) -> Vec<String> {
    let mut line = format!(
        "- {} · {} · {}`{}`",
        event.start().strftime("%a %Y-%m-%d %H:%M"),
        event.title,
        detail(event),
        event.uid
    );
    if !judged && !event.url.is_empty() {
        line.push_str(&format!(" · {}", event.url));
    }
    if filtered {
        line.push_str(" · filtered");
    }
    if judged {
        return vec![line];
    }
    let mut context: Vec<String> = Vec::new();
    let desc = excerpt(&event.description, DESCRIPTION_CHARS);
    if !desc.is_empty() {
        context.push(desc);
    }
    let cats = event.categories.join(", ");
    if !cats.is_empty() {
        context.push(cats);
    }
    if context.is_empty() {
        vec![line]
    } else {
        vec![line, format!("{CONTINUATION}{}", context.join(" · "))]
    }
}

fn add_days(dt: DateTime, days: i64) -> DateTime {
    Span::new()
        .try_days(days)
        .ok()
        .and_then(|s| dt.checked_add(s).ok())
        .unwrap_or(dt)
}

/// Write the roster. `filtered` is what the pre-filter dropped this run.
pub fn write_roster(
    path: &Path,
    events: &[DiscoveredEvent],
    ledger: &BTreeMap<String, LedgerEntry>,
    config: &EventsConfig,
    today: Date,
    filtered: &[DiscoveredEvent],
) -> std::io::Result<()> {
    let window_start = today.to_datetime(Time::midnight());
    let roster_end = add_days(window_start, config.roster_window_days);
    let audit_end = add_days(window_start, config.audit_window_days);

    let relevant: Vec<DiscoveredEvent> = relevant_events(events, ledger)
        .into_iter()
        .filter(|e| window_start <= e.start() && e.start() < roster_end)
        .collect();
    let mut seen: std::collections::HashSet<String> =
        relevant.iter().map(|e| e.uid.clone()).collect();

    let mut audit: Vec<(DiscoveredEvent, bool, bool)> = Vec::new();
    let pairs = events
        .iter()
        .map(|e| (e, false))
        .chain(filtered.iter().map(|e| (e, true)));
    for (event, was_filtered) in pairs {
        if seen.contains(&event.uid) {
            continue;
        }
        seen.insert(event.uid.clone());
        let entry = ledger.get(&event.uid);
        let judged = entry.is_some_and(|e| e.verdict.is_some());
        let end = if judged || was_filtered { audit_end } else { roster_end };
        if window_start <= event.start() && event.start() < end {
            audit.push((event.clone(), judged, was_filtered));
        }
    }
    audit.sort_by(|a, b| (a.0.start(), &a.0.uid).cmp(&(b.0.start(), &b.0.uid)));

    let mut lines = vec![
        HEADER.to_string(),
        String::new(),
        format!(
            "## Coming up — relevant (next {} days)",
            config.roster_window_days
        ),
        String::new(),
    ];
    let mut current_day: Option<Date> = None;
    for event in &relevant {
        let day = event.start().date();
        if current_day != Some(day) {
            current_day = Some(day);
            lines.push(format!(
                "### {} {}",
                day.strftime("%a"),
                day.strftime("%Y-%m-%d")
            ));
            lines.push(String::new());
        }
        lines.push(format!(
            "- {}–{} · **{}** · {}`{}`",
            event.start().strftime("%H:%M"),
            event.end().strftime("%H:%M"),
            event.title,
            detail(event),
            event.uid
        ));
    }
    if relevant.is_empty() {
        lines.push("- Nothing judged relevant yet.".to_string());
    }

    lines.push(String::new());
    lines.push(format!(
        "## Everything else — unjudged to {} days, judged and filtered to {}",
        config.roster_window_days, config.audit_window_days
    ));
    lines.push(String::new());
    for (event, judged, was_filtered) in &audit {
        lines.extend(audit_lines(event, *judged, *was_filtered));
    }
    if audit.is_empty() {
        lines.push("- Nothing else in the window.".to_string());
    }

    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)?;
    }
    pystr::write_text(path, &(lines.join("\n") + "\n"))
}

/// Recover the relevant section as a fallback when a feed is unreachable.
///
/// Lossy by design: only the fields the line carries survive. Unjudged events from a dead feed
/// simply wait for the feed to come back.
pub fn read_roster(path: &Path) -> Vec<DiscoveredEvent> {
    if !path.exists() {
        return Vec::new();
    }
    let Ok(text) = pystr::read_text(path) else {
        return Vec::new();
    };
    let mut events = Vec::new();
    let mut day: Option<Date> = None;
    for line in pystr::splitlines(&text) {
        if let Some(m) = DAY.captures(line) {
            day = m
                .name("day")
                .and_then(|d| d.as_str().parse::<Date>().ok());
            continue;
        }
        let Some(m) = LINE.captures(line) else { continue };
        let Some(day) = day else { continue };
        let parse_time = |name: &str| -> Option<Time> {
            m.name(name)
                .and_then(|x| Time::strptime("%H:%M", x.as_str()).ok())
        };
        let (Some(st), Some(et)) = (parse_time("start"), parse_time("end")) else {
            continue;
        };
        let start = day.to_datetime(st);
        let mut end = day.to_datetime(et);
        if end <= start {
            end = start.checked_add(Span::new().hours(1)).unwrap_or(start);
        }
        let rest = m.name("rest").map(|x| x.as_str()).unwrap_or("");
        let mut organizer = String::new();
        let mut location = String::new();
        for bit in rest.split('·') {
            let bit = bit.trim();
            if bit.is_empty() {
                continue;
            }
            if let Some(rest) = bit.strip_prefix("@ ") {
                location = rest.to_string();
            } else if bit != "free food" {
                organizer = bit.to_string();
            }
        }
        events.push(
            DiscoveredEvent {
                uid: m.name("uid").map(|x| x.as_str()).unwrap_or("").to_string(),
                title: m.name("title").map(|x| x.as_str()).unwrap_or("").to_string(),
                start: Some(start),
                end: Some(end),
                source: "roster".to_string(),
                organizer,
                location,
                ..Default::default()
            }
            .normalized(),
        );
    }
    events
}

/// One event the page lists under *Not shown*: dropped, with the one reason it was.
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize)]
pub struct DroppedEvent {
    pub uid: String,
    pub title: String,
    /// `YYYY-MM-DD`, as the audit line carries it.
    pub date: String,
    /// `HH:MM`, as the audit line carries it.
    pub time: String,
    pub reason: String,
}

// The audit line shape `audit_lines` writes: `- Thu 2026-09-03 18:00 · Title · detail`uid``, then
// ` · url` when unjudged and ` · filtered` when the pre-filter dropped it. The title is matched
// lazily, so a title that itself holds ` · ` is cut at its first one; the uid keys the event.
static AUDIT_LINE: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(
        r"^- \w{3} (?P<date>\d{4}-\d{2}-\d{2}) (?P<time>\d{2}:\d{2}) · (?P<title>.+?) · [^`]*`(?P<uid>[^`]+)`(?P<tail>.*)$",
    )
    .unwrap()
});

/// The events the roster's audit section records as dropped, each with the one reason.
///
/// Read-only: `state/events.md` and the event ledger are read, nothing is written. The reason is
/// "filtered by your interests" for a `· filtered` line, else "you declined it" when the ledger
/// marks the uid declined, else "judged not relevant" (with the ledger's `why`, when it has one)
/// for a `drop` verdict only (ruled 2026-09-30, Quinn: an `unsure` verdict is not listed, it may
/// still have an open card asking the student). An unjudged line, an unsure or relevant verdict, a
/// continuation line and everything under *Coming up* are not drops and are left out (PQ2 = no).
/// Date and time stay the text the line carries. An absent or unreadable roster is empty.
pub fn read_dropped(vault: &Path) -> Vec<DroppedEvent> {
    let path = vault.join("state").join("events.md");
    let Ok(text) = pystr::read_text(&path) else {
        return Vec::new();
    };
    let ledger = eventledger::load_ledger(vault, None);
    let mut in_audit = false;
    let mut out = Vec::new();
    for line in pystr::splitlines(&text) {
        if line.starts_with("## ") {
            in_audit = line.starts_with("## Everything else");
            continue;
        }
        if !in_audit {
            continue;
        }
        let Some(m) = AUDIT_LINE.captures(line) else { continue };
        let uid = &m["uid"];
        let entry = ledger.get(uid);
        let reason = if m["tail"].ends_with(" · filtered") {
            "filtered by your interests".to_string()
        } else if entry.is_some_and(|e| e.declined) {
            "you declined it".to_string()
        } else {
            match entry.and_then(|e| e.verdict.as_deref().map(|v| (v, &e.why))) {
                Some(("drop", why)) => {
                    if why.is_empty() {
                        "judged not relevant".to_string()
                    } else {
                        format!("judged not relevant: {why}")
                    }
                }
                _ => continue,
            }
        };
        out.push(DroppedEvent {
            uid: uid.to_string(),
            title: m["title"].to_string(),
            date: m["date"].to_string(),
            time: m["time"].to_string(),
            reason,
        });
    }
    out
}

#[cfg(test)]
mod tests {
    //! Direct port of `tests/test_event_roster.py` — all 22 tests, same names.

    use super::*;
    use jiff::civil::date;
    use std::fs;
    use std::path::PathBuf;

    const TODAY: Date = Date::constant(2026, 8, 20);

    fn config() -> EventsConfig {
        EventsConfig::default()
    }

    /// `event(uid, title, day, hour=18, **kwargs)` from the Python helper.
    fn event(uid: &str, title: &str, day: i8, hour: i8) -> DiscoveredEvent {
        DiscoveredEvent::new(
            uid,
            title,
            date(2026, 9, day).at(hour, 0, 0, 0),
            date(2026, 9, day).at(hour + 1, 0, 0, 0),
            "campus",
        )
    }

    fn ev(uid: &str, title: &str, day: i8) -> DiscoveredEvent {
        event(uid, title, day, 18)
    }

    fn ledger_of(pairs: &[(&str, &str)]) -> BTreeMap<String, LedgerEntry> {
        pairs
            .iter()
            .map(|(uid, verdict)| {
                (
                    uid.to_string(),
                    LedgerEntry {
                        uid: uid.to_string(),
                        verdict: Some(verdict.to_string()),
                        ..Default::default()
                    },
                )
            })
            .collect()
    }

    fn tmp(name: &str) -> PathBuf {
        let dir = std::env::temp_dir().join(format!("qo-roster-{}-{}", name, std::process::id()));
        let _ = fs::remove_dir_all(&dir);
        fs::create_dir_all(&dir).unwrap();
        dir.join("events.md")
    }

    fn read(path: &Path) -> String {
        pystr::read_text(path).unwrap()
    }

    fn write(path: &Path, events: &[DiscoveredEvent], ledger: &BTreeMap<String, LedgerEntry>) {
        write_roster(path, events, ledger, &config(), TODAY, &[]).unwrap();
    }

    fn audit_of(text: &str) -> String {
        text.split("## Everything else").nth(1).unwrap_or("").to_string()
    }

    #[test]
    fn relevant_events_uses_the_ledger() {
        let events = [ev("a", "Kept", 3), ev("b", "Dropped", 4), ev("c", "Unjudged", 5)];
        let ledger = ledger_of(&[("a", "opportunity"), ("b", "drop")]);
        let titles: Vec<String> = relevant_events(&events, &ledger)
            .iter()
            .map(|e| e.title.clone())
            .collect();
        assert_eq!(titles, vec!["Kept"]);
    }

    #[test]
    fn a_declined_opportunity_leaves_the_relevant_section() {
        // Unticking a digest line is a terminal decision — the event must stop occupying one of
        // the five `Coming up` slots. It stays visible in the audit section.
        let declined = ev("d", "Declined Thing", 1);
        let mut ledger = BTreeMap::new();
        ledger.insert(
            "d".to_string(),
            LedgerEntry {
                uid: "d".into(),
                verdict: Some("opportunity".into()),
                proposed: true,
                declined: true,
                ..Default::default()
            },
        );
        assert!(relevant_events(&[declined.clone()], &ledger).is_empty());

        let path = tmp("declined");
        write(&path, &[declined], &ledger);
        assert!(audit_of(&read(&path)).contains("Declined Thing"));
    }

    #[test]
    fn roster_has_both_sections() {
        let path = tmp("bothsections");
        let mut a = ev("a", "Relevant Thing", 3);
        a.organizer = "AI Club".into();
        a.location = "Lloyd 38".into();
        let events = [a, ev("b", "Other Thing", 1)];
        write(&path, &events, &ledger_of(&[("a", "opportunity")]));
        let text = read(&path);
        assert!(text.contains("## Coming up — relevant"));
        assert!(text.contains("## Everything else"));
        assert!(text.contains("Relevant Thing"));
        assert!(text.contains("Other Thing"));
        assert!(text.contains("AI Club"));
        assert!(text.contains("Lloyd 38"));
    }

    #[test]
    fn judged_drops_are_bounded_to_the_audit_window() {
        let path = tmp("auditbound");
        let near = ev("near", "Near Thing", 1); // 12 days out
        let far = ev("far", "Far Thing", 30); // 41 days out
        write(&path, &[near, far], &ledger_of(&[("near", "drop"), ("far", "drop")]));
        let text = read(&path);
        assert!(text.contains("Near Thing"));
        assert!(!text.contains("Far Thing"));
    }

    #[test]
    fn unjudged_events_reach_the_full_roster_window() {
        // The roster is the routine's only judging input. Bounding unjudged events to
        // audit_window_days means an obligation a month away cannot become a task until it is
        // two weeks away.
        let path = tmp("unjudgedfull");
        write(&path, &[ev("far", "Far Unjudged", 30)], &BTreeMap::new());
        assert!(read(&path).contains("Far Unjudged"));
    }

    #[test]
    fn prefiltered_events_appear_in_the_audit_section_marked() {
        let path = tmp("prefiltered");
        let kept = ev("k", "Kept Thing", 1);
        let dropped = ev("f", "Filtered Thing", 2);
        write_roster(&path, &[kept], &BTreeMap::new(), &config(), TODAY, &[dropped]).unwrap();
        let audit = audit_of(&read(&path));
        let line = audit.lines().find(|l| l.contains("Filtered Thing")).unwrap();
        assert!(line.contains("· filtered"), "a pre-filter drop must be distinguishable by stage");
        let kept_line = audit.lines().find(|l| l.contains("Kept Thing")).unwrap();
        assert!(!kept_line.contains("· filtered"));
    }

    #[test]
    fn a_judged_drop_carries_no_filtered_marker() {
        let path = tmp("judgeddrop");
        write(&path, &[ev("d", "Judged Drop", 1)], &ledger_of(&[("d", "drop")]));
        let audit = audit_of(&read(&path));
        let line = audit.lines().find(|l| l.contains("Judged Drop")).unwrap();
        assert!(!line.contains("· filtered"));
    }

    #[test]
    fn prefiltered_events_keep_the_tight_audit_bound() {
        let path = tmp("tightbound");
        let far = ev("far", "Far Filtered", 30);
        write_roster(&path, &[], &BTreeMap::new(), &config(), TODAY, &[far]).unwrap();
        assert!(!read(&path).contains("Far Filtered"));
    }

    #[test]
    fn unjudged_audit_lines_carry_the_metadata_the_judge_needs() {
        // Spec §7: the judge's inputs include the event's own metadata and description, and one
        // verdict per uid forever makes every under-informed drop permanent.
        let path = tmp("richaudit");
        let mut rich = ev("u", "Lunch & Learn", 1);
        rich.url = "https://calendar.ua.edu/event/lunch".into();
        rich.description = "A  session\non resume writing\twith recruiters.".into();
        rich.categories = vec!["Career".into(), "Workshop".into()];
        write(&path, &[rich], &BTreeMap::new());
        let text = read(&path);
        assert!(text.contains("https://calendar.ua.edu/event/lunch"));
        assert!(text.contains("A session on resume writing with recruiters."));
        assert!(text.contains("Career, Workshop"));
    }

    #[test]
    fn description_excerpt_is_one_capped_line() {
        let path = tmp("cappedline");
        let mut rich = ev("u", "Wordy", 1);
        rich.description = "alpha beta\n".repeat(200);
        write(&path, &[rich], &BTreeMap::new());
        let text = read(&path);
        let continuation = text.lines().find(|l| l.starts_with("      ")).unwrap();
        assert!(!continuation.contains('\n'));
        assert!(continuation.trim().chars().count() <= 210);
    }

    #[test]
    fn judged_lines_stay_terse() {
        let path = tmp("terse");
        let mut rich = ev("d", "Judged", 1);
        rich.url = "https://x.example/e".into();
        rich.description = "Long prose.".into();
        rich.categories = vec!["Career".into()];
        write(&path, &[rich], &ledger_of(&[("d", "drop")]));
        let text = read(&path);
        assert!(!text.contains("https://x.example/e"));
        assert!(!text.contains("Long prose."));
    }

    #[test]
    fn read_roster_ignores_continuation_lines() {
        let path = tmp("continuation");
        let mut kept = ev("engage:7", "Kickoff", 3);
        kept.organizer = "AI Club".into();
        kept.location = "Lloyd 38".into();
        let mut unjudged = ev("u", "Other", 1);
        unjudged.description = "Some description.".into();
        unjudged.categories = vec!["Career".into()];
        write(&path, &[kept, unjudged], &ledger_of(&[("engage:7", "opportunity")]));
        let uids: Vec<String> = read_roster(&path).iter().map(|e| e.uid.clone()).collect();
        assert_eq!(uids, vec!["engage:7"]);
    }

    #[test]
    fn relevant_section_reaches_the_full_roster_window() {
        let path = tmp("fullwindow");
        write(&path, &[ev("far", "Far Relevant", 30)], &ledger_of(&[("far", "obligation")]));
        assert!(read(&path).contains("Far Relevant"));
    }

    #[test]
    fn the_relevant_section_has_a_lower_bound() {
        // On the all-feeds-down fallback path the roster is re-read from itself, so without a
        // lower bound a past event persists under "Coming up" forever.
        let path = tmp("lowerbound");
        let past = DiscoveredEvent::new(
            "past",
            "Yesterday Thing",
            date(2026, 8, 19).at(18, 0, 0, 0),
            date(2026, 8, 19).at(19, 0, 0, 0),
            "campus",
        );
        write(&path, &[past], &ledger_of(&[("past", "obligation")]));
        assert!(!read(&path).contains("Yesterday Thing"));
    }

    #[test]
    fn roster_is_stably_ordered() {
        let path = tmp("stable");
        let events = vec![ev("b", "Bee", 3), ev("a", "Ay", 3)];
        write(&path, &events, &BTreeMap::new());
        let first = read(&path);
        let reversed: Vec<DiscoveredEvent> = events.into_iter().rev().collect();
        write(&path, &reversed, &BTreeMap::new());
        assert_eq!(read(&path), first);
    }

    #[test]
    fn roster_round_trips_the_fields_it_carries() {
        let path = tmp("roundtrip");
        let mut original = ev("engage:7", "Kickoff", 3);
        original.organizer = "AI Club".into();
        original.location = "Lloyd 38".into();
        write(&path, &[original.clone()], &ledger_of(&[("engage:7", "opportunity")]));
        let recovered = read_roster(&path);
        assert_eq!(recovered.len(), 1);
        assert_eq!(recovered[0].uid, "engage:7");
        assert_eq!(recovered[0].title, "Kickoff");
        assert_eq!(recovered[0].start(), original.start());
        assert_eq!(recovered[0].end(), original.end());
    }

    #[test]
    fn read_roster_of_missing_file_is_empty() {
        let dir = tmp("missing");
        assert!(read_roster(&dir.with_file_name("nope.md")).is_empty());
    }

    #[test]
    fn empty_roster_still_writes_both_headings() {
        let path = tmp("empty");
        write(&path, &[], &BTreeMap::new());
        let text = read(&path);
        assert!(text.contains("## Coming up — relevant"));
        assert!(text.contains("## Everything else"));
    }

    #[test]
    fn relevant_section_is_bounded_by_roster_window_days() {
        // TODAY is 2026-08-20; 90 days later is 2026-11-18.
        let path = tmp("rosterbound");
        let within = DiscoveredEvent::new(
            "within",
            "Within Window",
            date(2026, 10, 20).at(18, 0, 0, 0),
            date(2026, 10, 20).at(19, 0, 0, 0),
            "campus",
        );
        let beyond = DiscoveredEvent::new(
            "beyond",
            "Beyond Window",
            date(2026, 11, 20).at(18, 0, 0, 0),
            date(2026, 11, 20).at(19, 0, 0, 0),
            "campus",
        );
        write(
            &path,
            &[within, beyond],
            &ledger_of(&[("within", "obligation"), ("beyond", "obligation")]),
        );
        let text = read(&path);
        assert!(text.contains("Within Window"));
        assert!(!text.contains("Beyond Window"));
    }

    #[test]
    fn roster_round_trips_all_field_combinations() {
        // All 8 combinations of organizer/location/free-food presence.
        let path = tmp("allcombos");
        let cases: [(&str, &str, bool, &str); 8] = [
            ("", "", false, "no fields"),
            ("AI Club", "", false, "organizer only"),
            ("", "Lloyd 38", false, "location only"),
            ("AI Club", "Lloyd 38", false, "organizer and location"),
            ("", "", true, "food only"),
            ("AI Club", "", true, "organizer and food"),
            ("", "Lloyd 38", true, "location and food"),
            ("AI Club", "Lloyd 38", true, "all fields"),
        ];
        for (organizer, location, food, name) in cases {
            let mut original = ev("test", "Test Event", 3);
            original.organizer = organizer.into();
            original.location = location.into();
            original.benefits = if food { vec!["free food".into()] } else { Vec::new() };
            write(&path, &[original.clone()], &ledger_of(&[("test", "opportunity")]));
            let recovered = read_roster(&path);
            assert_eq!(recovered.len(), 1, "failed for case: {name}");
            assert_eq!(recovered[0].organizer, organizer, "organizer mismatch for {name}");
            assert_eq!(recovered[0].location, location, "location mismatch for {name}");
            assert_eq!(recovered[0].uid, "test");
            assert_eq!(recovered[0].title, "Test Event");
            assert_eq!(recovered[0].start(), original.start());
            assert_eq!(recovered[0].end(), original.end());
        }
    }

    // ---- read_dropped (M2 T6) ----

    /// Close enough to the September events that they fall inside the 14-day audit window.
    const SEPT_1: Date = Date::constant(2026, 9, 1);

    fn scratch_vault(name: &str) -> PathBuf {
        let vault = tmp(name).parent().unwrap().to_path_buf();
        fs::create_dir_all(vault.join("state")).unwrap();
        vault
    }

    fn roster_path(vault: &Path) -> PathBuf {
        vault.join("state").join("events.md")
    }

    #[test]
    fn read_dropped_reads_the_fixtures_audit_section() {
        let fixture = Path::new("tests/fixtures/vault-full/state/events.md");
        let before = fs::read(fixture).unwrap();
        let vault = scratch_vault("dropped-fixture");
        // The frozen roster has an empty audit section; read it through a copy.
        let copy = pystr::read_text(fixture).unwrap();
        assert!(copy.contains("- Nothing else in the window."));
        pystr::write_text(&roster_path(&vault), &copy).unwrap();
        assert!(read_dropped(&vault).is_empty(), "Coming up is never returned");

        // Two filtered lines go into the copy's audit section; Coming up keeps its three.
        let with_drops = copy.replace(
            "- Nothing else in the window.",
            "- Fri 2026-09-04 18:30 · Pep Rally · Spirit Club · @ Gym `localist:pep` · filtered\n\
             - Sat 2026-09-05 09:00 · Yard Sale · `ics:yard` · filtered",
        );
        pystr::write_text(&roster_path(&vault), &with_drops).unwrap();
        let dropped = read_dropped(&vault);
        let got: Vec<(&str, &str, &str, &str)> = dropped
            .iter()
            .map(|d| (d.title.as_str(), d.date.as_str(), d.time.as_str(), d.reason.as_str()))
            .collect();
        assert_eq!(
            got,
            vec![
                ("Pep Rally", "2026-09-04", "18:30", "filtered by your interests"),
                ("Yard Sale", "2026-09-05", "09:00", "filtered by your interests"),
            ]
        );
        assert_eq!(dropped[0].uid, "localist:pep");
        assert_eq!(fs::read(fixture).unwrap(), before, "the frozen reference is untouched");
    }

    #[test]
    fn read_dropped_names_why() {
        let vault = scratch_vault("dropped-why");
        let mut declined = ev("dec", "Declined Thing", 2);
        declined.organizer = "Chess Club".into();
        let events = [declined, ev("irr", "Irrelevant Thing", 3), ev("rel", "Kept Thing", 4)];
        let filtered = [ev("flt", "Filtered Thing", 5)];
        let when = SEPT_1;
        eventledger::record_verdict(&vault, "dec", "Declined Thing", when, "opportunity", "", "", "")
            .unwrap();
        eventledger::record_proposed(&vault, "dec", when).unwrap();
        eventledger::record_declined(&vault, "dec", when).unwrap();
        eventledger::record_verdict(
            &vault, "irr", "Irrelevant Thing", when, "drop", "", "a sports night", "",
        )
        .unwrap();
        eventledger::record_verdict(&vault, "rel", "Kept Thing", when, "obligation", "", "", "")
            .unwrap();
        let ledger = eventledger::load_ledger(&vault, None);
        write_roster(&roster_path(&vault), &events, &ledger, &config(), SEPT_1, &filtered).unwrap();
        let before = fs::read(roster_path(&vault)).unwrap();
        let ledger_before = fs::read(vault.join("state").join("events-seen.md")).unwrap();

        let dropped = read_dropped(&vault);
        let got: Vec<(&str, &str)> = dropped
            .iter()
            .map(|d| (d.title.as_str(), d.reason.as_str()))
            .collect();
        assert_eq!(
            got,
            vec![
                ("Declined Thing", "you declined it"),
                ("Irrelevant Thing", "judged not relevant: a sports night"),
                ("Filtered Thing", "filtered by your interests"),
            ]
        );
        assert_eq!(dropped[0].date, "2026-09-02");
        assert_eq!(dropped[0].time, "18:00");
        assert_eq!(fs::read(roster_path(&vault)).unwrap(), before, "the read writes nothing");
        assert_eq!(fs::read(vault.join("state").join("events-seen.md")).unwrap(), ledger_before);
    }

    #[test]
    fn read_dropped_skips_unjudged_and_continuation_lines() {
        // PQ2 = no (Quinn, 2026-09-30): an unjudged event is not a drop, so it never appears.
        let vault = scratch_vault("dropped-unjudged");
        let mut unjudged = ev("un", "Unjudged Thing", 2);
        unjudged.description = "A long description that lands on a continuation line".into();
        unjudged.categories = vec!["Social".into()];
        unjudged.url = "https://example.test/un".into();
        let filtered = [ev("flt", "Filtered Thing", 3)];
        write_roster(&roster_path(&vault), &[unjudged], &BTreeMap::new(), &config(), SEPT_1, &filtered)
            .unwrap();
        let text = read(&roster_path(&vault));
        assert!(text.contains("Unjudged Thing") && text.contains("      A long description"));
        let titles: Vec<String> = read_dropped(&vault).into_iter().map(|d| d.title).collect();
        assert_eq!(titles, vec!["Filtered Thing"]);
    }

    #[test]
    fn read_dropped_skips_unsure() {
        // Ruled 2026-09-30 (Quinn): only a "drop" verdict is "judged not relevant"; an "unsure"
        // event may still have an open card asking the student, so it is not listed.
        let vault = scratch_vault("dropped-unsure");
        let events = [ev("uns", "Unsure Thing", 2), ev("irr", "Irrelevant Thing", 3)];
        let when = SEPT_1;
        eventledger::record_verdict(&vault, "uns", "Unsure Thing", when, "unsure", "", "maybe", "")
            .unwrap();
        eventledger::record_verdict(&vault, "irr", "Irrelevant Thing", when, "drop", "", "", "")
            .unwrap();
        let ledger = eventledger::load_ledger(&vault, None);
        write_roster(&roster_path(&vault), &events, &ledger, &config(), SEPT_1, &[]).unwrap();
        assert!(read(&roster_path(&vault)).contains("Unsure Thing"), "the roster still lists it");
        let got: Vec<(String, String)> =
            read_dropped(&vault).into_iter().map(|d| (d.title, d.reason)).collect();
        assert_eq!(got, vec![("Irrelevant Thing".to_string(), "judged not relevant".to_string())]);
    }

    #[test]
    fn read_dropped_of_an_absent_roster_is_empty() {
        let vault = scratch_vault("dropped-absent");
        assert!(!roster_path(&vault).exists());
        assert!(read_dropped(&vault).is_empty());
    }
}

#[cfg(test)]
mod reference {
    //! THE ARTEFACT ORACLE for the roster.
    //!
    //! `tests/fixtures/vault-full/state/events.md` was written by the **Python**
    //! `eventroster.write_roster` (see `scripts/build-vault-full.py`) and is frozen with the
    //! fixture. Regenerating it here from the same inputs and comparing bytes covers the whole
    //! writer at once — every separator, the day headings, the audit section, CRLF — the same
    //! argument that made `today.md` a sufficient oracle for waves 0-3.
    //!
    //! **If this fails, fix the Rust.** Never regenerate the fixture.

    use super::*;
    use jiff::civil::date;

    fn fixture_events() -> (Vec<DiscoveredEvent>, BTreeMap<String, LedgerEntry>) {
        let specs: [(&str, &str, DateTime, DateTime, &str, &str, &str, &str, &str, &[&str]); 3] = [
            (
                "ics:fixture-research-symposium",
                "Undergraduate Research Symposium",
                date(2026, 9, 2).at(16, 0, 0, 0),
                date(2026, 9, 2).at(18, 0, 0, 0),
                "fixture-ics",
                "Office of Undergraduate Research",
                "Science and Engineering Complex 1005",
                "Poster session and lightning talks from current UA undergraduate researchers.",
                "strong",
                &["free food"],
            ),
            (
                "localist:fixture-coop-info-session",
                "Co-op and Internship Info Session",
                date(2026, 9, 4).at(12, 0, 0, 0),
                date(2026, 9, 4).at(13, 0, 0, 0),
                "fixture-localist",
                "Career Center",
                "3400 UA Student Center",
                "How the co-op program works, who is hiring this spring, and how to apply.",
                "strong",
                &[],
            ),
            (
                "engage:fixture-ai-club-kickoff",
                "Artificial Intelligence Club Kickoff",
                date(2026, 9, 10).at(18, 0, 0, 0),
                date(2026, 9, 10).at(19, 30, 0, 0),
                "fixture-engage",
                "Artificial Intelligence Club",
                "Shelby Hall 1093",
                "First meeting of the semester. Project teams, competition plans, pizza.",
                "mild",
                &["free food"],
            ),
        ];
        let whys = [
            "Undergraduate research is a strong stated interest",
            "Co-op and internship info sessions are a strong stated interest",
            "Named club, plus free food",
        ];
        let mut events = Vec::new();
        let mut ledger = BTreeMap::new();
        for (i, (uid, title, start, end, source, organizer, location, description, strength, benefits))
            in specs.into_iter().enumerate()
        {
            events.push(
                DiscoveredEvent {
                    uid: uid.into(),
                    title: title.into(),
                    start: Some(start),
                    end: Some(end),
                    source: source.into(),
                    organizer: organizer.into(),
                    location: location.into(),
                    description: description.into(),
                    benefits: benefits.iter().map(|b| b.to_string()).collect(),
                    ..Default::default()
                }
                .normalized(),
            );
            ledger.insert(
                uid.to_string(),
                LedgerEntry {
                    uid: uid.into(),
                    verdict: Some("opportunity".into()),
                    strength: strength.into(),
                    why: whys[i].into(),
                    ..Default::default()
                },
            );
        }
        (events, ledger)
    }

    #[test]
    fn the_vault_full_roster_matches_python_byte_for_byte() {
        let (events, ledger) = fixture_events();
        let dir = std::env::temp_dir().join(format!("qo-roster-ref-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        let path = dir.join("events.md");
        write_roster(
            &path,
            &events,
            &ledger,
            &EventsConfig::default(),
            Date::constant(2026, 8, 28),
            &[],
        )
        .unwrap();

        let actual = std::fs::read(&path).unwrap();
        let expected = std::fs::read("tests/fixtures/vault-full/state/events.md").unwrap();
        if actual != expected {
            let a = String::from_utf8_lossy(&actual);
            let e = String::from_utf8_lossy(&expected);
            for (i, (al, el)) in a.split("\r\n").zip(e.split("\r\n")).enumerate() {
                assert_eq!(al, el, "first difference at line {}", i + 1);
            }
            panic!(
                "roster differs from Python's ({} bytes vs {}). Fix the Rust; do NOT regenerate \
                 the fixture.",
                actual.len(),
                expected.len()
            );
        }
    }

    /// The roster is not only written — `cli.run` reads it back when every feed fails. A format
    /// change that writes fine and parses wrong empties the events section on a bad-network day.
    #[test]
    fn pythons_roster_reads_back_into_the_three_relevant_events() {
        let recovered = read_roster(Path::new("tests/fixtures/vault-full/state/events.md"));
        let titles: Vec<&str> = recovered.iter().map(|e| e.title.as_str()).collect();
        assert_eq!(
            titles,
            vec![
                "Undergraduate Research Symposium",
                "Co-op and Internship Info Session",
                "Artificial Intelligence Club Kickoff"
            ]
        );
        assert_eq!(recovered[0].location, "Science and Engineering Complex 1005");
        assert_eq!(recovered[0].organizer, "Office of Undergraduate Research");
    }
}
