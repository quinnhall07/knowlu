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

// ---------------------------------------------------------------------------------------------
// The event-check card (F2)
// ---------------------------------------------------------------------------------------------

//
// An event the judgment could only call `unsure` becomes one question to the student: "Does this
// apply to you?". The card rides the ordinary approvals machinery (cap, snooze, expiry, the
// console's Approve/Reject); `approvals::process_approvals` turns the click into a human-answer
// line in the event ledger (F3). Everything here is deterministic and makes no request: it runs
// inside `rank`, which never calls a model.
//
// **Never ask twice.** A uid already named as a card's `source_uid` or listed in a card's
// `events:`, and a `proposed` ledger line, each close that instance's question for good. A series
// is closed while one of its cards is live in `approvals/` and for good once the student answered
// one (archived `executed` or `rejected`). A card closed without an answer (expired, or deleted
// from the app) is not an answer (controller ruling G1): the series' next unanswered instance may
// be asked. Closing the question does not drop the answer: [`inherit_series_answers`] carries a
// settled series card's answer to the series' later instances.

/// The most `kind: event-check` cards first proposed on any one day. They cost real attention and
/// must not crowd task and amend cards out of the day's approval budget.
pub const EVENT_CHECKS_PER_DAY: i64 = 3;

/// The most instances one series card lists.
const SERIES_LIST_CAP: usize = 20;

const EVENT_CHECK: &str = "event-check";

const WHY_PARAGRAPH: &str = "**Does this apply to you?** Knowlu could not tell from the event's \
own listing whether it is meant for you.";

const CLOSING: &str = "Approve if it applies to you: it goes on your schedule for that day. \
Reject and it's dropped. Either way you won't be asked again.";

/// The closing of a card with no `instances:` (a roster read-back, §2): its Approve only records
/// the answer and books nothing (D11), so it promises no schedule entry.
const ANSWER_ONLY_CLOSING: &str = "Approve if it applies to you: it joins Coming up as something you're expected at. Reject and it's dropped. Either way you won't be asked again.";

const EVENT_ACCEPT: &str = "event-accept";

/// The `source` `eventroster::read_roster` gives every event it reads back (a literal there).
const ROSTER_SOURCE: &str = "roster";

/// The most obligation cards first proposed on any one day (D7), beside `event-check`'s own.
pub const OBLIGATION_CARDS_PER_DAY: i64 = 3;

const OBLIGATION_LEAD: &str = "**Knowlu thinks this is required of you.**";

/// The most opportunity cards first proposed on any one day (D7), beside the other two kinds'.
pub const OPPORTUNITY_CARDS_PER_DAY: i64 = 3;

const OPPORTUNITY_LEAD: &str = "**This may interest you.**";

/// An unanswered opportunity card expires this many days after `first_proposed_at`, or at the
/// event, whichever is first (D3, §4.4).
const OPPORTUNITY_EXPIRY_DAYS: i64 = 14;

/// What each answer does on an `event-accept` card, obligation or opportunity alike.
const ACCEPT_CLOSING: &str = "Approve to put it on your schedule for that day. Reject and \
it's dropped. Either way you won't be asked again.";

/// One row of the verdict-to-card table (P10): the ledger verdict that earns a question, the card
/// kind it files, the start window the event must sit in, how the card reads and the cap on
/// cards first proposed in a day. [`select_cards`] reads every row the same way.
struct CardRow {
    verdict: &'static str,
    kind: &'static str,
    /// `(event, config, today, last day)`: is the event inside this kind's window?
    window: fn(&DiscoveredEvent, &EventsConfig, Date, Date) -> bool,
    closing: &'static str,
    cap: i64,
    /// The file name's first words: `event-check` or `event`.
    file_prefix: &'static str,
    /// The title's prefix (`Required · `), or empty.
    title_prefix: &'static str,
    /// The card's first paragraph. A kind that quotes the ledger's why follows it with that why.
    lead: &'static str,
    quotes_why: bool,
    /// Does the body carry the Obsidian Approve and Reject buttons? Only the older card does.
    buttons: bool,
    /// May a card be built from `read_roster`'s read-back (`source: "roster"`)? Such an event is
    /// lossy (§2), so it never earns a card that books its time.
    roster_ok: bool,
    /// An unanswered card expires this many days after it is first proposed if that is before the
    /// event: the opportunity's rule (§4.4). `None`: it expires on its primary's date.
    expiry_days: Option<i64>,
    /// Are the cards ordered by the digest's `sort_key` (D7) rather than by (start, uid)?
    by_strength: bool,
}

/// `unsure` keeps `emit_event_checks`' window: a start in `[today, today + propose_horizon_days]`.
fn start_window(event: &DiscoveredEvent, _: &EventsConfig, today: Date, last_day: Date) -> bool {
    let day = event.start().date();
    day >= today && day <= last_day
}

/// `obligation` uses the digest's rule (P10): a start not before today, and today at or past
/// [`horizon_start`] (the registration deadline can open it earlier).
fn horizon_window(event: &DiscoveredEvent, config: &EventsConfig, today: Date, _: Date) -> bool {
    event.start().date() >= today && horizon_start(event, config) <= today
}

/// `unsure` → `event-check`.
const EVENT_CHECK_ROW: CardRow = CardRow {
    verdict: "unsure",
    kind: EVENT_CHECK,
    window: start_window,
    closing: CLOSING,
    cap: EVENT_CHECKS_PER_DAY,
    file_prefix: "event-check",
    title_prefix: "",
    lead: WHY_PARAGRAPH,
    quotes_why: false,
    buttons: true,
    roster_ok: true,
    expiry_days: None,
    by_strength: false,
};

/// `obligation` → `event-accept`.
const OBLIGATION_ROW: CardRow = CardRow {
    verdict: "obligation",
    kind: EVENT_ACCEPT,
    window: horizon_window,
    closing: ACCEPT_CLOSING,
    cap: OBLIGATION_CARDS_PER_DAY,
    file_prefix: "event",
    title_prefix: "Required · ",
    lead: OBLIGATION_LEAD,
    quotes_why: true,
    buttons: false,
    roster_ok: false,
    expiry_days: None,
    by_strength: false,
};

/// `opportunity` → `event-accept`, live only under `config.event_cards` (D6). The digest's window
/// (P10), its own cap of 3 and the digest's order.
const OPPORTUNITY_ROW: CardRow = CardRow {
    verdict: "opportunity",
    kind: EVENT_ACCEPT,
    window: horizon_window,
    closing: ACCEPT_CLOSING,
    cap: OPPORTUNITY_CARDS_PER_DAY,
    file_prefix: "event",
    title_prefix: "Worth a look · ",
    lead: OPPORTUNITY_LEAD,
    quotes_why: true,
    buttons: false,
    roster_ok: false,
    expiry_days: Some(OPPORTUNITY_EXPIRY_DAYS),
    by_strength: true,
};

/// A settled series card's answer: `verdict` is `obligation` (the card was executed) or `drop`
/// (rejected); `by` is who answered it, read from the ledger's answer line for `source_uid`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SeriesAnswer {
    pub verdict: String,
    pub by: String,
    pub source_uid: String,
}

/// `Thu 1 Oct`.
fn day_label(day: Date) -> String {
    format!("{} {} {}", day.strftime("%a"), day.day(), day.strftime("%b"))
}

/// `10am`, `10:30`, `7pm` — a 12-hour clock, minutes only when non-zero.
pub(crate) fn clock(time: Time, with_meridiem: bool) -> String {
    let hour = match time.hour() % 12 {
        0 => 12,
        h => h,
    };
    let mut out = if time.minute() == 0 {
        hour.to_string()
    } else {
        format!("{hour}:{:02}", time.minute())
    };
    if with_meridiem {
        out.push_str(if time.hour() < 12 { "am" } else { "pm" });
    }
    out
}

/// The date-and-time half of [`what_and_when`]: `Thu 1 Oct 10am–3pm`, `Thu 1 Oct` (all day), or
/// `Thu 1 Oct – Sat 3 Oct` (several days).
fn when_label(event: &DiscoveredEvent) -> String {
    let (start, end) = (event.start(), event.end());
    let midnight = Time::midnight();
    // An end at midnight belongs to the day before it: an all-day event and a 10pm–12am event
    // both end "on" the day they started.
    let last_day = if end.time() == midnight && end.date() > start.date() {
        end.date().yesterday().unwrap_or(start.date())
    } else {
        end.date().max(start.date())
    };
    if last_day > start.date() {
        return format!("{} – {}", day_label(start.date()), day_label(last_day));
    }
    let day = day_label(start.date());
    if start.time() == midnight && end.time() == midnight {
        return day; // all day
    }
    if end <= start {
        return format!("{day} {}", clock(start.time(), true));
    }
    let split_by_noon = (start.hour() < 12) != (end.hour() < 12);
    format!(
        "{day} {}\u{2013}{}",
        clock(start.time(), split_by_noon),
        clock(end.time(), true)
    )
}

/// `{title≤60} · {Ddd} {d} {Mon} {range}` — the card's title, from structured fields only.
///
/// The range is a 12-hour clock with minutes only when non-zero; the end always carries am/pm and
/// the start only when it is on the other side of noon: `10am–3pm`, `7–9pm`, `10:30–11:15am`,
/// `11am–12pm`, `12–1pm` (the dash is U+2013). An all-day event has no range, and an event over
/// several days reads `Thu 1 Oct – Sat 3 Oct`. A series card appends ` · +N more` itself.
pub fn what_and_when(event: &DiscoveredEvent) -> String {
    let title = crate::judge::one_line(&event.title, 60);
    let title = title.trim_end();
    let title = if title.is_empty() { "(untitled)" } else { title };
    format!("{title} · {}", when_label(event))
}

/// `(meta, file name)` of every `kind: event-check` approval in `folder`, in file-name order.
fn event_check_cards(vault: &Path, folder: &str) -> Vec<(serde_yaml_ng::Mapping, String)> {
    cards_of_kind(vault, folder, EVENT_CHECK)
}

/// `(meta, file name)` of every `kind: <kind>` approval in `folder`, in file-name order.
fn cards_of_kind(vault: &Path, folder: &str, kind: &str) -> Vec<(serde_yaml_ng::Mapping, String)> {
    let mut out = Vec::new();
    let dir = vault.join(folder);
    if !dir.is_dir() {
        return out;
    }
    for path in crate::approvals::sorted_md(&dir) {
        let Ok(text) = pystr::read_text(&path) else { continue };
        let Ok((meta, _)) = split_frontmatter(&text) else { continue };
        if card_text(&meta, "type") != "approval" || card_text(&meta, "kind") != kind {
            continue;
        }
        let name = path.file_name().map(|n| n.to_string_lossy().to_string()).unwrap_or_default();
        out.push((meta, name));
    }
    out
}

fn card_text(meta: &serde_yaml_ng::Mapping, key: &str) -> String {
    let raw = crate::yaml::opt_text(crate::yaml::get(meta, key)).unwrap_or_default();
    pystr::strip(&raw).to_string()
}

/// The uids a card's `events:` lists; empty when the key is missing or not a list.
pub(crate) fn card_event_uids(meta: &serde_yaml_ng::Mapping) -> Vec<String> {
    match crate::yaml::get(meta, "events") {
        Some(serde_yaml_ng::Value::Sequence(items)) => items
            .iter()
            .filter_map(crate::yaml::text)
            .map(|uid| pystr::strip(&uid).to_string())
            .filter(|uid| !uid.is_empty())
            .collect(),
        _ => Vec::new(),
    }
}

/// Every series whose `event-check` card was answered, keyed by `series_uid`.
///
/// Reads `archive/` only: an `executed` card answers `obligation`, a `rejected` one `drop`, and an
/// `expired` (unanswered) card answers nothing. `by` is the `answered_by` of the ledger entry for
/// the card's `source_uid`, or `"unknown"`. Two settled cards for one series cannot arise (the
/// emitter never asks about a series twice); if a hand edit makes one, the lowest file name wins.
pub fn settled_series(vault: &Path) -> BTreeMap<String, SeriesAnswer> {
    let mut settled: BTreeMap<String, SeriesAnswer> = BTreeMap::new();
    let mut ledger: Option<BTreeMap<String, LedgerEntry>> = None;
    for (meta, _) in event_check_cards(vault, "archive") {
        let verdict = match card_text(&meta, "status").as_str() {
            "executed" => "obligation",
            "rejected" => "drop",
            _ => continue,
        };
        let series = card_text(&meta, "series_uid");
        if series.is_empty() || settled.contains_key(&series) {
            continue;
        }
        let source_uid = card_text(&meta, "source_uid");
        let ledger = ledger.get_or_insert_with(|| crate::eventledger::load_ledger(vault, None));
        let by = ledger
            .get(&source_uid)
            .map(|e| e.answered_by.clone())
            .filter(|b| !b.is_empty())
            .unwrap_or_else(|| "unknown".to_string());
        settled.insert(series, SeriesAnswer { verdict: verdict.to_string(), by, source_uid });
    }
    settled
}

/// Give a settled series' answer to each of its instances that is `unsure` or unjudged and
/// unanswered, writing one human-answer line per instance and updating `ledger` exactly as
/// `load_ledger` would read the new line. Returns `(lines written, warnings)`.
///
/// An instance with a confident machine verdict keeps it, and a series whose card expired
/// unanswered gives nothing. The line carries the instance's own `jid`, since it settles that
/// instance's own judgment. A write failure is a warning and the next run retries.
pub fn inherit_series_answers(
    vault: &Path,
    events: &[DiscoveredEvent],
    ledger: &mut BTreeMap<String, LedgerEntry>,
    today: Date,
) -> (usize, Vec<String>) {
    let mut written = 0;
    let mut warnings = Vec::new();
    let settled = settled_series(vault);
    if settled.is_empty() {
        return (written, warnings);
    }
    let mut ordered: Vec<&DiscoveredEvent> = events.iter().collect();
    ordered.sort_by(|a, b| (a.start(), &a.uid).cmp(&(b.start(), &b.uid)));
    for event in ordered {
        let Some(answer) = settled.get(&event.series_uid) else { continue };
        let current = ledger.get(&event.uid).cloned().unwrap_or_else(|| LedgerEntry::new(&event.uid));
        if !matches!(current.verdict.as_deref(), None | Some("unsure")) || !current.answered_by.is_empty() {
            continue;
        }
        let jid = Some(current.judgment_id.as_str()).filter(|j| !j.is_empty());
        let recorded = crate::eventledger::record_answer(
            vault,
            &event.uid,
            &event.title,
            today,
            &answer.verdict,
            &answer.by,
            jid,
        );
        if let Err(err) = recorded {
            warnings.push(format!("series answer not recorded for {} ({err})", event.uid));
            continue;
        }
        let mut entry = current;
        if entry.verdict.is_none() {
            // A first verdict line: `load_ledger` reads its title too.
            entry.title = crate::eventledger::clean_title(&event.title);
        }
        entry.verdict = Some(answer.verdict.clone());
        entry.answered_by = answer.by.clone();
        ledger.insert(event.uid.clone(), entry);
        written += 1;
    }
    (written, warnings)
}

/// Does this event qualify for a question of `row`'s kind today? See [`emit_event_checks`].
#[allow(clippy::too_many_arguments)]
fn needs_check(
    row: &CardRow,
    event: &DiscoveredEvent,
    config: &EventsConfig,
    ledger: &BTreeMap<String, LedgerEntry>,
    first_day: Date,
    last_day: Date,
    asked_uids: &BTreeSet<String>,
    asked_series: &BTreeSet<String>,
) -> bool {
    if event.source == ROSTER_SOURCE && !row.roster_ok {
        return false;
    }
    let Some(entry) = ledger.get(&event.uid) else { return false };
    if entry.verdict.as_deref() != Some(row.verdict)
        || !entry.answered_by.is_empty()
        || entry.declined
        || entry.proposed
    {
        return false;
    }
    if !(row.window)(event, config, first_day, last_day) {
        return false;
    }
    !asked_uids.contains(&event.uid) && !asked_series.contains(&event.series_uid)
}

/// A lowercase `[a-z0-9-]` form of `title`, at most 40 characters.
fn card_slug(title: &str) -> String {
    let slug: String = crate::ingest::slugify(title).chars().take(40).collect();
    slug.trim_end_matches('-').to_string()
}

/// The card's body: the question (the paragraph the console shows as the card's `why`), the
/// facts, the other instances of a series, what each answer does, and the buttons.
fn check_body(
    row: &CardRow,
    primary: &DiscoveredEvent,
    others: &[&DiscoveredEvent],
    ledger: &BTreeMap<String, LedgerEntry>,
) -> String {
    let mut facts = vec![when_label(primary)];
    for extra in [&primary.location, &primary.organizer] {
        let extra = crate::judge::one_line(extra, 120);
        if !extra.is_empty() {
            facts.push(extra);
        }
    }
    let mut lead = row.lead.to_string();
    if row.quotes_why {
        let why = ledger.get(&primary.uid).map(|e| crate::judge::one_line(&e.why, 400)).unwrap_or_default();
        if !why.is_empty() {
            lead.push(' ');
            lead.push_str(&why);
        }
    }
    let mut lines = vec![lead, String::new(), facts.join(" · ")];
    let url = crate::judge::one_line(&primary.url, 500);
    if !url.is_empty() {
        lines.push(url);
    }
    if !others.is_empty() {
        let days: Vec<String> = others.iter().map(|e| when_label(e)).collect();
        lines.push(format!("Also on: {}", days.join(", ")));
    }
    // The new closing promises a booking, so only a card that carries `instances:` (D10) says it.
    let closing = if row.kind == EVENT_CHECK && !writes_instances(primary) {
        ANSWER_ONLY_CLOSING
    } else {
        row.closing
    };
    lines.extend([String::new(), closing.to_string(), String::new()]);
    if row.buttons {
        lines.extend([BUTTONS.to_string(), String::new()]);
    }
    lines.join("\n")
}

/// Does the card for `primary` carry `instances:`? Never when it is read back from the roster.
fn writes_instances(primary: &DiscoveredEvent) -> bool {
    primary.source != ROSTER_SOURCE
}

/// The card's `expires`: the primary's date, or for an opportunity the earlier of that and
/// `first_proposed_at` (today) plus [`CardRow::expiry_days`].
fn expires(row: &CardRow, primary: &DiscoveredEvent, today: Date) -> Date {
    let event_day = primary.start().date();
    match row.expiry_days {
        Some(days) => {
            let cutoff = today
                .checked_add(Span::new().try_days(days).unwrap_or_default())
                .unwrap_or(today);
            event_day.min(cutoff)
        }
        None => event_day,
    }
}

/// Write one card for `primary` and the rest of its series, `others`. `Err` is a create that
/// failed.
fn write_check(
    vault: &Path,
    row: &CardRow,
    primary: &DiscoveredEvent,
    others: &[&DiscoveredEvent],
    ledger: &BTreeMap<String, LedgerEntry>,
    today: Date,
    ctx: &WriteContext,
    journal: &mut Journal,
) -> Result<PathBuf, crate::write::WriteError> {
    use crate::yamlemit::Node;
    let mut title = format!("{}{}", row.title_prefix, what_and_when(primary));
    if !others.is_empty() {
        title.push_str(&format!(" · +{} more", others.len()));
    }
    let jid = ledger.get(&primary.uid).map(|e| e.judgment_id.clone()).unwrap_or_default();
    let mut pairs = vec![
        ("type", Node::text("approval")),
        ("kind", Node::text(row.kind)),
        ("title", Node::text(&title)),
        ("status", Node::text("pending")),
    ];
    if row.kind == EVENT_ACCEPT {
        pairs.push(("verdict", Node::text(row.verdict)));
    }
    pairs.extend([
        ("source_uid", Node::text(&primary.uid)),
        ("series_uid", Node::text(&primary.series_uid)),
        (
            "events",
            Node::Seq(
                std::iter::once(primary)
                    .chain(others.iter().copied())
                    .map(|e| Node::text(&e.uid))
                    .collect(),
            ),
        ),
    ]);
    // The settlement's whole input (D10). Never from `read_roster`'s lossy read-back (§2): such
    // a card answers only, and books nothing.
    if writes_instances(primary) {
        let listed = std::iter::once(primary).chain(others.iter().copied());
        let instances = listed
            .filter(|e| e.source != ROSTER_SOURCE)
            .map(|e| crate::eventaccept::Instance::from_event(e).to_node())
            .collect();
        pairs.push(("instances", Node::Seq(instances)));
    }
    if !jid.is_empty() {
        pairs.push(("judgment_id", Node::text(&jid)));
        pairs.push(("judgment_kind", Node::text("event")));
    }
    pairs.extend([
        // Two distinct dates, never an anchor — see the ruling in `src/yamlemit.rs`.
        ("proposed_at", Node::Date(today)),
        ("first_proposed_at", Node::Date(today)),
        ("expires", Node::Date(expires(row, primary, today))),
        ("snooze_until", Node::Null),
        // The literal the digest and `calendar_note` write; the journal's actor is the ctx's.
        ("created_by", Node::text("events")),
    ]);
    let front = Node::Map(pairs.into_iter().map(|(k, v)| (Node::text(k), v)).collect());
    let text = format!(
        "---\n{}---\n\n{}",
        crate::yamlemit::safe_dump_block(&front),
        check_body(row, primary, others, ledger)
    );

    let folder = vault.join("approvals");
    std::fs::create_dir_all(&folder).map_err(|e| crate::write::WriteError::Io(e.to_string()))?;
    let stem = format!(
        "{}-{}-{}",
        row.file_prefix,
        card_slug(&primary.title),
        primary.start().date().strftime("%Y-%m-%d")
    );
    let mut name = format!("{stem}.md");
    let mut suffix = 2;
    while folder.join(&name).exists() {
        name = format!("{stem}-{suffix}.md");
        suffix += 1;
    }
    create(vault, &format!("approvals/{name}"), &text, ctx, journal, None)
}

/// File the "Does this apply to you?" cards for today. Returns `(paths, count)`.
///
/// An event qualifies when its ledger verdict is `unsure`, nobody answered it, it is neither
/// declined nor proposed, it starts within `[today, today + propose_horizon_days]`, its uid is no
/// approval's `source_uid` and on no event card's `events:` (either kind), and its series has no
/// live event card of either kind in `approvals/` and no answered one (`settled_series` and
/// `eventcarry::answered_series`). An expired or deleted card does not close its series (ruling
/// G1). Qualifying instances of one series share one card: the soonest is its primary, and the
/// card lists up to 20 of them. Cards are filed in `(primary start, primary uid)` order, at most
/// `min(budget, 3 − event-check cards first proposed today)` of them, so there is never overflow
/// for `defer_over_budget` to snooze. After each card, every instance it lists gets a `proposed`
/// ledger line: the second guard against a re-ask. A failed create stops the run of cards.
#[allow(clippy::too_many_arguments)]
pub fn emit_event_checks(
    vault: &Path,
    events: &[DiscoveredEvent],
    ledger: &BTreeMap<String, LedgerEntry>,
    config: &EventsConfig,
    today: Date,
    budget: i64,
    ctx: &WriteContext,
    journal: &mut Journal,
) -> (Vec<PathBuf>, usize) {
    let row = &EVENT_CHECK_ROW;
    let groups = select_cards(vault, row, events, ledger, config, today, budget);
    file_cards(vault, row, &groups, ledger, today, ctx, journal)
}

/// File the `event-accept` cards of one `verdict` for today. Returns `(paths, count)`.
///
/// `obligation` and `opportunity` are the verdicts filed (D2: both are asked, never created);
/// `opportunity` only when `config.event_cards` is on (D6), and any other verdict files nothing.
/// An event qualifies as [`emit_event_checks`]' do, widened to both card
/// kinds: its uid is no approval's `source_uid` and on no event card's `events:` (either kind), and
/// its series has no live event card and no answered one (`settled_series` and
/// `eventcarry::answered_series`, the one definition). Its window is the digest's (P10). An event
/// read back from the roster (`source: "roster"`) never earns a card (§2). At most
/// `min(budget, 3 − cards of this verdict first proposed today)` cards are filed, the soonest
/// first (opportunities in the digest's `sort_key` order, D7), each followed by a `proposed` line
/// for every instance it lists.
#[allow(clippy::too_many_arguments)]
pub fn emit_event_accepts(
    vault: &Path,
    events: &[DiscoveredEvent],
    ledger: &BTreeMap<String, LedgerEntry>,
    config: &EventsConfig,
    today: Date,
    budget: i64,
    verdict: &str,
    ctx: &WriteContext,
    journal: &mut Journal,
) -> (Vec<PathBuf>, usize) {
    let row = match verdict {
        "obligation" => &OBLIGATION_ROW,
        "opportunity" if config.event_cards => &OPPORTUNITY_ROW,
        _ => return (Vec::new(), 0),
    };
    let groups = select_cards(vault, row, events, ledger, config, today, budget);
    file_cards(vault, row, &groups, ledger, today, ctx, journal)
}

/// Write each group's card, and after it a `proposed` line for every instance it lists. A failed
/// create stops the run of cards.
fn file_cards(
    vault: &Path,
    row: &CardRow,
    groups: &[Group<'_>],
    ledger: &BTreeMap<String, LedgerEntry>,
    today: Date,
    ctx: &WriteContext,
    journal: &mut Journal,
) -> (Vec<PathBuf>, usize) {
    let mut filed: Vec<PathBuf> = Vec::new();
    for (primary, others) in groups {
        let Ok(path) = write_check(vault, row, primary, others, ledger, today, ctx, journal) else {
            break;
        };
        // Card first, ledger second, as the digest does.
        for event in std::iter::once(primary).chain(others.iter()) {
            let _ = record_proposed(vault, &event.uid, today);
        }
        filed.push(path);
    }
    let count = filed.len();
    (filed, count)
}

/// One card's worth of events: the primary (the soonest) and the rest of its series.
type Group<'a> = (&'a DiscoveredEvent, Vec<&'a DiscoveredEvent>);

/// The selection behind every event card: which groups of `events` get a `row.kind` card today.
///
/// Eligibility (`row.verdict`, nobody answered, neither declined nor proposed), `row.window`,
/// never-ask-twice, the day's cap (`min(budget, row.cap − cards of this kind first proposed
/// today)`), the order (primary start, primary uid) and the series grouping (up to
/// [`SERIES_LIST_CAP`] instances a card). The returned groups are already cut to the allowance.
fn select_cards<'a>(
    vault: &Path,
    row: &CardRow,
    events: &'a [DiscoveredEvent],
    ledger: &BTreeMap<String, LedgerEntry>,
    config: &EventsConfig,
    today: Date,
    budget: i64,
) -> Vec<Group<'a>> {
    let mut asked_series: BTreeSet<String> = BTreeSet::new();
    let mut listed_uids: BTreeSet<String> = BTreeSet::new();
    let mut first_proposed_today: i64 = 0;
    // Never ask twice reads both kinds; each kind counts only its own cards against its cap.
    for kind in [EVENT_CHECK, EVENT_ACCEPT] {
        for folder in ["approvals", "archive"] {
            for (meta, _) in cards_of_kind(vault, folder, kind) {
                // A live card closes its series. One closed without an answer (expired, or
                // deleted from the app) closes only the instances it listed; an answered one
                // closes its series through the union below.
                let series = card_text(&meta, "series_uid");
                if !series.is_empty() && folder == "approvals" {
                    asked_series.insert(series);
                }
                listed_uids.extend(card_event_uids(&meta));
                let own = kind == row.kind
                    && (kind != EVENT_ACCEPT || card_text(&meta, "verdict") == row.verdict);
                let proposed = crate::approvals::as_date(crate::yaml::get(&meta, "first_proposed_at"));
                if own && proposed == Some(today) {
                    first_proposed_today += 1;
                }
            }
        }
    }
    let allowance = budget.min(row.cap - first_proposed_today).max(0) as usize;
    if allowance == 0 {
        return Vec::new();
    }
    // An answered series, of either kind: the one union `judge_roster` skips too.
    asked_series.extend(settled_series(vault).into_keys());
    asked_series.extend(crate::eventcarry::answered_series(vault).into_keys());
    let mut asked_uids = crate::approvals::existing_source_uids(vault);
    asked_uids.extend(listed_uids);
    let last_day = today
        .checked_add(Span::new().try_days(config.propose_horizon_days).unwrap_or_default())
        .unwrap_or(today);

    let mut qualifying: Vec<&DiscoveredEvent> = events
        .iter()
        .filter(|e| needs_check(row, e, config, ledger, today, last_day, &asked_uids, &asked_series))
        .collect();
    qualifying.sort_by(|a, b| (a.start(), &a.uid).cmp(&(b.start(), &b.uid)));
    qualifying.dedup_by(|a, b| a.uid == b.uid);
    // Grouped in (start, uid) order, so each group's first member is its primary and the groups
    // themselves come out in (primary start, primary uid) order.
    let mut groups: Vec<Group<'a>> = Vec::new();
    for event in qualifying {
        match groups.iter_mut().find(|(primary, _)| primary.series_uid == event.series_uid) {
            Some((_, others)) => {
                if others.len() + 1 < SERIES_LIST_CAP {
                    others.push(event);
                }
            }
            None => groups.push((event, Vec::new())),
        }
    }
    if row.by_strength {
        // The soonest instance is still each series' primary; the cards come out in the digest's
        // order of their primaries (strength, registration deadline, start, uid). Stable.
        groups.sort_by_key(|(primary, _)| sort_key(primary, ledger));
    }
    groups.truncate(allowance);
    groups
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

    // --- the event-check card (F2) ------------------------------------------------------

    mod event_checks {
        use super::*;
        use crate::eventledger::{record_answer, record_judged_verdict};
        use jiff::civil::DateTime;
        use serde_yaml_ng::{Mapping, Value};

        /// A Monday; 2026-10-01 is the Thursday after it.
        const DAY: Date = Date::constant(2026, 9, 28);
        const JID_A: &str = "0f0e0d0c-0b0a-4908-8706-050403020100";
        const JID_B: &str = "1a2b3c4d-5e6f-4a1b-8c2d-3e4f5a6b7c8d";
        const WHY: &str = "the listing does not say who it is for";

        fn at(y: i16, m: i8, d: i8, h: i8, mi: i8) -> DateTime {
            date(y, m, d).at(h, mi, 0, 0)
        }

        fn span(uid: &str, title: &str, start: DateTime, end: DateTime) -> DiscoveredEvent {
            DiscoveredEvent {
                uid: uid.to_string(),
                title: title.to_string(),
                start: Some(start),
                end: Some(end),
                source: "campus".into(),
                ..Default::default()
            }
            .normalized()
        }

        /// A same-day event on `(month, day)` from `h:mi` to `eh:emi`.
        fn on(uid: &str, title: &str, m: i8, d: i8, h: i8, mi: i8, eh: i8, emi: i8) -> DiscoveredEvent {
            span(uid, title, at(2026, m, d, h, mi), at(2026, m, d, eh, emi))
        }

        fn in_series(mut event: DiscoveredEvent, series: &str) -> DiscoveredEvent {
            event.series_uid = series.to_string();
            event
        }

        fn vault(name: &str) -> PathBuf {
            tmp(&format!("check-{name}"))
        }

        fn unsure(vault: &Path, uid: &str, title: &str, jid: Option<&str>) {
            record_judged_verdict(vault, uid, title, DAY, "unsure", WHY, jid).unwrap();
        }

        fn check_on(
            vault: &Path,
            events: &[DiscoveredEvent],
            ledger: &BTreeMap<String, LedgerEntry>,
            today: Date,
            budget: i64,
        ) -> (Vec<PathBuf>, usize) {
            let mut journal = Journal::new(vault);
            let ctx = WriteContext::new("agent:events", "cli");
            emit_event_checks(vault, events, ledger, &config(), today, budget, &ctx, &mut journal)
        }

        fn check(vault: &Path, events: &[DiscoveredEvent], budget: i64) -> (Vec<PathBuf>, usize) {
            let ledger = load_ledger(vault, None);
            check_on(vault, events, &ledger, DAY, budget)
        }

        fn cards(vault: &Path, folder: &str) -> Vec<PathBuf> {
            crate::approvals::sorted_md(&vault.join(folder))
                .into_iter()
                .filter(|p| {
                    p.file_name()
                        .and_then(|n| n.to_str())
                        .is_some_and(|n| n.starts_with("event-check-"))
                })
                .collect()
        }

        fn note(path: &Path) -> (Mapping, String) {
            split_frontmatter(&pystr::read_text(path).unwrap()).unwrap()
        }

        fn field(meta: &Mapping, key: &str) -> Option<String> {
            crate::yaml::get(meta, key).and_then(crate::yaml::text)
        }

        fn date_field(meta: &Mapping, key: &str) -> Option<Date> {
            crate::approvals::as_date(crate::yaml::get(meta, key))
        }

        fn event_uids(meta: &Mapping) -> Vec<String> {
            match crate::yaml::get(meta, "events") {
                Some(Value::Sequence(items)) => {
                    items.iter().filter_map(crate::yaml::text).collect()
                }
                _ => Vec::new(),
            }
        }

        fn seen(vault: &Path) -> String {
            pystr::read_text(&vault.join("state").join("events-seen.md")).unwrap_or_default()
        }

        /// A card written by hand into `folder`, as a settled or live card would sit there.
        fn hand_card(vault: &Path, folder: &str, name: &str, status: &str, source: &str, series: &str) {
            fs::create_dir_all(vault.join(folder)).unwrap();
            let text = format!(
                "---\ntype: approval\nkind: event-check\ntitle: \"Earlier card\"\nstatus: {status}\n\
                 source_uid: \"{source}\"\nseries_uid: \"{series}\"\nevents:\n- \"{source}\"\n\
                 proposed_at: 2026-09-20\nfirst_proposed_at: 2026-09-20\nexpires: 2026-09-24\n\
                 snooze_until: null\ncreated_by: events\n---\n\nbody\n"
            );
            pystr::write_text(&vault.join(folder).join(name), &text).unwrap();
        }

        #[test]
        fn what_and_when_formats_every_shape() {
            let cases = [
                (on("a", "Career fair", 10, 1, 10, 0, 15, 0), "Career fair · Thu 1 Oct 10am–3pm"),
                (on("b", "Film night", 10, 1, 19, 0, 21, 0), "Film night · Thu 1 Oct 7–9pm"),
                (on("c", "Office hours", 10, 1, 10, 30, 11, 15), "Office hours · Thu 1 Oct 10:30–11:15am"),
                (on("d", "Brunch", 10, 1, 11, 0, 12, 0), "Brunch · Thu 1 Oct 11am–12pm"),
                (on("e", "Lunch talk", 10, 1, 12, 0, 13, 0), "Lunch talk · Thu 1 Oct 12–1pm"),
                (on("f", "Late lab", 10, 1, 9, 5, 12, 30), "Late lab · Thu 1 Oct 9:05am–12:30pm"),
                (
                    span("g", "Open day", at(2026, 10, 1, 0, 0), at(2026, 10, 2, 0, 0)),
                    "Open day · Thu 1 Oct",
                ),
                (
                    span("h", "Retreat", at(2026, 10, 1, 0, 0), at(2026, 10, 4, 0, 0)),
                    "Retreat · Thu 1 Oct – Sat 3 Oct",
                ),
                (
                    span("i", "Hackathon", at(2026, 10, 1, 18, 0), at(2026, 10, 3, 12, 0)),
                    "Hackathon · Thu 1 Oct – Sat 3 Oct",
                ),
            ];
            for (event, expected) in cases {
                assert_eq!(what_and_when(&event), expected, "{}", event.uid);
            }
            // The title is clipped to 60 characters; the date is never clipped.
            let long = on("j", &"x".repeat(80), 10, 1, 19, 0, 21, 0);
            assert_eq!(what_and_when(&long), format!("{} · Thu 1 Oct 7–9pm", "x".repeat(60)));
        }

        #[test]
        fn an_unsure_event_in_the_horizon_files_one_event_check_card() {
            let vault = vault("one");
            let fair = on("ics:fair-1", "Career fair", 10, 1, 10, 0, 15, 0);
            unsure(&vault, "ics:fair-1", "Career fair", Some(JID_A));
            let (paths, count) = check(&vault, std::slice::from_ref(&fair), 15);
            assert_eq!(count, 1);
            assert_eq!(paths, vec![vault.join("approvals").join("event-check-career-fair-2026-10-01.md")]);

            let (meta, body) = note(&paths[0]);
            assert_eq!(field(&meta, "type").as_deref(), Some("approval"));
            assert_eq!(field(&meta, "kind").as_deref(), Some("event-check"));
            assert_eq!(field(&meta, "title").as_deref(), Some("Career fair · Thu 1 Oct 10am–3pm"));
            assert_eq!(field(&meta, "status").as_deref(), Some("pending"));
            assert_eq!(field(&meta, "source_uid").as_deref(), Some("ics:fair-1"));
            assert_eq!(field(&meta, "series_uid").as_deref(), Some("ics:fair-1"));
            assert_eq!(event_uids(&meta), vec!["ics:fair-1".to_string()]);
            assert_eq!(field(&meta, "judgment_id").as_deref(), Some(JID_A));
            assert_eq!(field(&meta, "judgment_kind").as_deref(), Some("event"));
            assert_eq!(date_field(&meta, "proposed_at"), Some(DAY));
            assert_eq!(date_field(&meta, "first_proposed_at"), Some(DAY));
            assert_eq!(date_field(&meta, "expires"), Some(date(2026, 10, 1)));
            assert!(matches!(crate::yaml::get(&meta, "snooze_until"), Some(Value::Null)));
            let first = body.split("\n\n").map(str::trim).find(|p| !p.is_empty()).unwrap();
            assert_eq!(
                first,
                "**Does this apply to you?** Knowlu could not tell from the event's own listing \
                 whether it is meant for you."
            );
            assert!(body.contains("Either way you won't be asked again."), "{body}");
            assert!(body.contains(BUTTONS), "{body}");
            let id = field(&meta, "id").unwrap_or_default();
            assert!(crate::ids::ID_RE.is_match(&id), "id was {id:?}");

            let creates = journal_creates(&vault);
            assert_eq!(creates, vec![(
                "approvals/event-check-career-fair-2026-10-01.md".to_string(),
                "agent:events".to_string()
            )]);
            // The second guard against a re-ask.
            assert!(load_ledger(&vault, None)["ics:fair-1"].proposed);
        }

        /// `(path, actor)` of every `create` record in the journal.
        fn journal_creates(vault: &Path) -> Vec<(String, String)> {
            let mut out = Vec::new();
            let dir = vault.join("state").join("journal");
            let Ok(entries) = fs::read_dir(&dir) else { return out };
            for entry in entries.filter_map(|e| e.ok()) {
                let text = pystr::read_text(&entry.path()).unwrap();
                for line in pystr::splitlines(&text) {
                    if line.trim().is_empty() {
                        continue;
                    }
                    let rec: serde_json::Value = serde_json::from_str(line).unwrap();
                    if rec.get("op").and_then(|o| o.as_str()) == Some("create") {
                        out.push((
                            rec["path"].as_str().unwrap_or("").to_string(),
                            rec["actor"].as_str().unwrap_or("").to_string(),
                        ));
                    }
                }
            }
            out.sort();
            out
        }

        #[test]
        fn a_second_run_files_nothing() {
            let vault = vault("second");
            let fair = on("ics:fair-1", "Career fair", 10, 1, 10, 0, 15, 0);
            unsure(&vault, "ics:fair-1", "Career fair", Some(JID_A));
            assert_eq!(check(&vault, std::slice::from_ref(&fair), 15).1, 1);
            let (paths, count) = check(&vault, std::slice::from_ref(&fair), 15);
            assert!(paths.is_empty());
            assert_eq!(count, 0);
            assert_eq!(cards(&vault, "approvals").len(), 1);
            assert_eq!(seen(&vault).matches("- ics:fair-1 · proposed").count(), 1);
            // A new day does not reopen the question either.
            let ledger = load_ledger(&vault, None);
            let next = DAY.tomorrow().unwrap();
            assert_eq!(check_on(&vault, std::slice::from_ref(&fair), &ledger, next, 15).1, 0);
        }

        #[test]
        fn nothing_is_asked_after_a_card_is_answered_or_while_one_is_live() {
            let vault = vault("archived");
            // No `proposed` marker for any of these: only the cards stand between them and a
            // second question. (An expired series card closes only its own instances: see
            // `an_expired_or_unanswered_series_card_lets_a_later_instance_be_asked`.)
            let events = [
                on("ics:a", "Asked and rejected", 10, 1, 10, 0, 11, 0),
                on("ics:b", "Asked and expired", 10, 1, 12, 0, 13, 0),
                in_series(on("lx:8:3", "Executed series", 10, 2, 12, 0, 13, 0), "lx:8"),
                in_series(on("lx:7:2", "Live series card", 10, 3, 10, 0, 11, 0), "lx:7"),
                on("ics:d", "Never asked", 10, 4, 10, 0, 11, 0),
            ];
            for e in &events {
                unsure(&vault, &e.uid, &e.title, None);
            }
            hand_card(&vault, "archive", "event-check-a.md", "rejected", "ics:a", "ics:a");
            hand_card(&vault, "archive", "event-check-b.md", "expired", "ics:b", "ics:b");
            hand_card(&vault, "archive", "event-check-eight.md", "executed", "lx:8:1", "lx:8");
            hand_card(&vault, "approvals", "event-check-seven.md", "pending", "lx:7:1", "lx:7");
            let (paths, count) = check(&vault, &events, 15);
            assert_eq!(count, 1, "{paths:?}");
            let (meta, _) = note(&paths[0]);
            assert_eq!(field(&meta, "source_uid").as_deref(), Some("ics:d"));
        }

        #[test]
        fn confident_answered_declined_or_proposed_events_are_never_asked() {
            let vault = vault("never");
            let uids = ["opp", "obl", "drop", "answered", "declined", "proposed", "unjudged"];
            let events: Vec<DiscoveredEvent> = uids
                .iter()
                .enumerate()
                .map(|(i, uid)| on(uid, uid, 10, 1, 8 + i as i8, 0, 9 + i as i8, 0))
                .collect();
            let entry = |uid: &str, verdict: &str| LedgerEntry {
                uid: uid.to_string(),
                verdict: Some(verdict.to_string()),
                ..Default::default()
            };
            let ledger = ledger_of(vec![
                entry("opp", "opportunity"),
                entry("obl", "obligation"),
                entry("drop", "drop"),
                LedgerEntry { answered_by: "quinn".into(), ..entry("answered", "unsure") },
                LedgerEntry { declined: true, ..entry("declined", "unsure") },
                LedgerEntry { proposed: true, ..entry("proposed", "unsure") },
            ]);
            let (paths, count) = check_on(&vault, &events, &ledger, DAY, 15);
            assert!(paths.is_empty());
            assert_eq!(count, 0);
            assert!(!vault.join("approvals").exists());
        }

        #[test]
        fn events_past_or_beyond_the_horizon_are_not_asked() {
            let vault = vault("horizon");
            let events = [
                on("yesterday", "Yesterday", 9, 27, 10, 0, 11, 0),
                on("today", "Today", 9, 28, 8, 0, 9, 0),
                on("edge", "Edge", 10, 12, 10, 0, 11, 0), // today + 14
                on("beyond", "Beyond", 10, 13, 10, 0, 11, 0), // today + 15
            ];
            for e in &events {
                unsure(&vault, &e.uid, &e.title, None);
            }
            let (paths, count) = check(&vault, &events, 15);
            assert_eq!(count, 2);
            let asked: Vec<String> =
                paths.iter().filter_map(|p| field(&note(p).0, "source_uid")).collect();
            assert_eq!(asked, vec!["today".to_string(), "edge".to_string()]);
        }

        #[test]
        fn a_series_gets_one_card_listing_its_unsure_instances() {
            let vault = vault("series");
            let days = [(9, 29), (10, 2), (10, 6), (10, 9)];
            let events: Vec<DiscoveredEvent> = days
                .iter()
                .enumerate()
                .map(|(i, (m, d))| {
                    in_series(
                        on(&format!("lx:77:{}", i + 1), "Weekly meeting", *m, *d, 19, 0, 21, 0),
                        "lx:77",
                    )
                })
                .collect();
            // Fed out of order: the primary is the soonest, not the first listed.
            let mut shuffled = events.clone();
            shuffled.reverse();
            for e in &events {
                unsure(&vault, &e.uid, &e.title, None);
            }
            let (paths, count) = check(&vault, &shuffled, 15);
            assert_eq!(count, 1);
            assert_eq!(paths, vec![vault.join("approvals").join("event-check-weekly-meeting-2026-09-29.md")]);
            let (meta, body) = note(&paths[0]);
            assert_eq!(
                field(&meta, "title").as_deref(),
                Some("Weekly meeting · Tue 29 Sep 7–9pm · +3 more")
            );
            assert_eq!(field(&meta, "source_uid").as_deref(), Some("lx:77:1"));
            assert_eq!(field(&meta, "series_uid").as_deref(), Some("lx:77"));
            assert_eq!(event_uids(&meta), vec!["lx:77:1", "lx:77:2", "lx:77:3", "lx:77:4"]);
            assert!(body.contains("Also on: "), "{body}");
            let ledger = load_ledger(&vault, None);
            for e in &events {
                assert!(ledger[&e.uid].proposed, "{}", e.uid);
            }
        }

        /// T2a.1a's pin: what `emit_event_checks` selects, written green on the code before the
        /// selection moved into the shared function. It pins the selection, not the card bytes, so
        /// it stays true when T2a.1b adds `instances:` and the new closing.
        #[test]
        fn the_selection_refactor_keeps_what_event_checks_select() {
            let vault = vault("pin");
            let events = [
                // Fed out of order: the cards come out in (primary start, primary uid) order.
                on("ics:d", "Dance night", 10, 4, 20, 0, 22, 0), // a fourth group: past the cap of 3
                in_series(on("lx:5:2", "Weekly circle", 10, 9, 19, 0, 21, 0), "lx:5"),
                on("ics:c", "Charity run", 10, 3, 8, 0, 9, 0),
                in_series(on("lx:5:1", "Weekly circle", 10, 2, 19, 0, 21, 0), "lx:5"),
                on("ics:a", "Alpha mixer", 10, 1, 10, 0, 11, 0),
            ];
            for e in &events {
                let jid = (e.uid == "ics:a").then_some(JID_A);
                unsure(&vault, &e.uid, &e.title, jid);
            }
            let (paths, count) = check(&vault, &events, 15);
            assert_eq!(count, 3);
            let names: Vec<String> = paths
                .iter()
                .map(|p| p.file_name().unwrap().to_string_lossy().to_string())
                .collect();
            assert_eq!(
                names,
                [
                    "event-check-alpha-mixer-2026-10-01.md",
                    "event-check-weekly-circle-2026-10-02.md",
                    "event-check-charity-run-2026-10-03.md",
                ]
            );
            // The same cards are what the folder holds, and nothing else.
            let mut by_name = paths.clone();
            by_name.sort();
            assert_eq!(cards(&vault, "approvals"), by_name);

            let expect: [(&str, &str, &str, &[&str], &str); 3] = [
                ("Alpha mixer · Thu 1 Oct 10–11am", "ics:a", "ics:a", &["ics:a"], "2026-10-01"),
                (
                    "Weekly circle · Fri 2 Oct 7–9pm · +1 more",
                    "lx:5:1",
                    "lx:5",
                    &["lx:5:1", "lx:5:2"],
                    "2026-10-02",
                ),
                ("Charity run · Sat 3 Oct 8–9am", "ics:c", "ics:c", &["ics:c"], "2026-10-03"),
            ];
            for (path, (title, source, series, listed, expires)) in paths.iter().zip(expect) {
                let (meta, _) = note(path);
                assert_eq!(field(&meta, "title").as_deref(), Some(title));
                assert_eq!(field(&meta, "source_uid").as_deref(), Some(source));
                assert_eq!(field(&meta, "series_uid").as_deref(), Some(series));
                assert_eq!(event_uids(&meta), listed);
                assert_eq!(field(&meta, "expires").as_deref(), Some(expires));
                assert_eq!(date_field(&meta, "first_proposed_at"), Some(DAY));
                assert_eq!(date_field(&meta, "proposed_at"), Some(DAY));
                assert_eq!(field(&meta, "status").as_deref(), Some("pending"));
            }
            assert_eq!(
                field(&note(&paths[0]).0, "judgment_id").as_deref(),
                Some(JID_A),
                "a card carries its primary's judgment id"
            );
            assert_eq!(field(&note(&paths[1]).0, "judgment_id"), None);

            // The `proposed` lines, in the order the cards were filed; the fourth group has none.
            let proposed: Vec<String> = seen(&vault)
                .lines()
                .filter(|l| l.contains(" · proposed "))
                .map(str::to_string)
                .collect();
            assert_eq!(
                proposed,
                [
                    "- ics:a · proposed 2026-09-28",
                    "- lx:5:1 · proposed 2026-09-28",
                    "- lx:5:2 · proposed 2026-09-28",
                    "- ics:c · proposed 2026-09-28",
                ]
            );
        }

        fn five(vault: &Path, first_day: i8) -> Vec<DiscoveredEvent> {
            let events: Vec<DiscoveredEvent> = (0..5)
                .map(|i| {
                    let uid = format!("ics:d{first_day}-{i}");
                    on(&uid, &format!("Talk {first_day} {i}"), 10, first_day + i as i8, 18, 0, 19, 0)
                })
                .collect();
            for e in &events {
                unsure(vault, &e.uid, &e.title, None);
            }
            events
        }

        #[test]
        fn the_emitter_respects_the_budget_and_the_daily_ceiling() {
            let vault = vault("ceiling");
            let events = five(&vault, 1);
            let (paths, count) = check(&vault, &events, 15);
            assert_eq!(count, 3);
            let asked: Vec<String> =
                paths.iter().filter_map(|p| field(&note(p).0, "source_uid")).collect();
            assert_eq!(asked, vec!["ics:d1-0", "ics:d1-1", "ics:d1-2"]);
            assert_eq!(check(&vault, &events, 15).1, 0, "the same day's second call");

            let small = self::vault("ceiling-budget");
            let events = five(&small, 1);
            assert_eq!(check(&small, &events, 1).1, 1);
            assert_eq!(check(&small, &events, 0).1, 0);
            assert_eq!(check(&small, &events, -4).1, 0);
        }

        #[test]
        fn a_card_without_a_ledger_jid_omits_both_judgment_keys() {
            let vault = vault("nojid");
            let fair = on("ics:fair-1", "Career fair", 10, 1, 10, 0, 15, 0);
            unsure(&vault, "ics:fair-1", "Career fair", None);
            let (paths, _) = check(&vault, std::slice::from_ref(&fair), 15);
            let text = pystr::read_text(&paths[0]).unwrap();
            assert!(!text.contains("judgment_id"), "{text}");
            assert!(!text.contains("judgment_kind"), "{text}");
        }

        #[test]
        fn the_daily_ceiling_counts_first_proposed_at_not_proposed_at() {
            let vault = vault("firstproposed");
            let events = five(&vault, 1);
            let (paths, count) = check(&vault, &events, 15);
            assert_eq!(count, 3);
            // `defer_over_budget` moves `proposed_at` to tomorrow and leaves `first_proposed_at`.
            let tomorrow = DAY.tomorrow().unwrap();
            let rel = crate::ids::rel(&vault, &paths[0]);
            let mut journal = Journal::new(&vault);
            crate::write::write_literals(
                &vault,
                &rel,
                &[
                    ("status".to_string(), "snoozed".to_string()),
                    ("snooze_until".to_string(), tomorrow.to_string()),
                    ("proposed_at".to_string(), tomorrow.to_string()),
                ],
                &WriteContext::new("agent:rank", "cli"),
                &mut journal,
                &crate::write::WriteOpts::default(),
            )
            .unwrap();
            let mut all = events.clone();
            all.extend(five(&vault, 8));
            assert_eq!(check(&vault, &all, 15).1, 0, "the deferred card still counts today");
            let ledger = load_ledger(&vault, None);
            assert_eq!(check_on(&vault, &all, &ledger, tomorrow, 15).1, 3, "tomorrow is a full 3");
        }

        #[test]
        fn the_card_says_created_by_events_and_the_journal_says_agent_events() {
            let vault = vault("createdby");
            let fair = on("ics:fair-1", "Career fair", 10, 1, 10, 0, 15, 0);
            unsure(&vault, "ics:fair-1", "Career fair", Some(JID_A));
            let (paths, _) = check(&vault, std::slice::from_ref(&fair), 15);
            assert_eq!(field(&note(&paths[0]).0, "created_by").as_deref(), Some("events"));
            let actors: Vec<String> = journal_creates(&vault).into_iter().map(|c| c.1).collect();
            assert_eq!(actors, vec!["agent:events".to_string()]);
        }

        // --- series inheritance ------------------------------------------------------------

        fn weekly(n: usize) -> DiscoveredEvent {
            let day = 22 + 7 * (n as i8 - 1); // lx:77:1 on 22 Sep, :2 on 29 Sep, ...
            let (m, d) = if day > 30 { (10, day - 30) } else { (9, day) };
            in_series(on(&format!("lx:77:{n}"), "Weekly meeting", m, d, 19, 0, 21, 0), "lx:77")
        }

        fn inherit(vault: &Path, events: &[DiscoveredEvent], ledger: &mut BTreeMap<String, LedgerEntry>) -> (usize, Vec<String>) {
            inherit_series_answers(vault, events, ledger, DAY)
        }

        #[test]
        fn a_later_instance_of_an_answered_series_inherits_the_answer() {
            for (status, verdict) in [("executed", "obligation"), ("rejected", "drop")] {
                let vault = vault(&format!("inherit-{status}"));
                let (first, later) = (weekly(1), weekly(2));
                unsure(&vault, &first.uid, &first.title, Some(JID_A));
                record_answer(&vault, &first.uid, &first.title, DAY, verdict, "quinn", Some(JID_A)).unwrap();
                unsure(&vault, &later.uid, &later.title, Some(JID_B));
                hand_card(&vault, "archive", "event-check-weekly.md", status, &first.uid, "lx:77");

                let settled = settled_series(&vault);
                assert_eq!(
                    settled.get("lx:77"),
                    Some(&SeriesAnswer {
                        verdict: verdict.to_string(),
                        by: "quinn".to_string(),
                        source_uid: first.uid.clone(),
                    })
                );
                let mut ledger = load_ledger(&vault, None);
                let (count, warnings) = inherit(&vault, &[first.clone(), later.clone()], &mut ledger);
                assert_eq!((count, warnings), (1, Vec::<String>::new()));
                let entry = &ledger[&later.uid];
                assert_eq!(entry.verdict.as_deref(), Some(verdict));
                assert_eq!(entry.answered_by, "quinn");
                assert_eq!(entry.judgment_id, JID_B, "the instance's own judgment");
                assert!(seen(&vault).contains(&format!(
                    "- lx:77:2 · Weekly meeting · verdict:{verdict} · by:quinn · jid:{JID_B} · answered 2026-09-28"
                )));
                assert_eq!(load_ledger(&vault, None)[&later.uid], *entry, "in memory as on disk");
            }
        }

        #[test]
        fn a_series_whose_answer_line_is_missing_is_answered_by_unknown() {
            let vault = vault("inherit-unknown");
            hand_card(&vault, "archive", "event-check-weekly.md", "rejected", "lx:77:1", "lx:77");
            assert_eq!(settled_series(&vault)["lx:77"].by, "unknown");
        }

        #[test]
        fn an_unjudged_instance_of_a_settled_series_inherits_too() {
            let vault = vault("inherit-unjudged");
            let (first, later) = (weekly(1), weekly(3));
            unsure(&vault, &first.uid, &first.title, Some(JID_A));
            record_answer(&vault, &first.uid, &first.title, DAY, "obligation", "quinn", None).unwrap();
            hand_card(&vault, "archive", "event-check-weekly.md", "executed", &first.uid, "lx:77");
            let mut ledger = load_ledger(&vault, None);
            assert!(!ledger.contains_key(&later.uid));
            let (count, _) = inherit(&vault, std::slice::from_ref(&later), &mut ledger);
            assert_eq!(count, 1);
            assert!(seen(&vault).contains("- lx:77:3 · Weekly meeting · verdict:obligation · by:quinn · answered 2026-09-28"));
            assert_eq!(ledger[&later.uid].verdict.as_deref(), Some("obligation"));
            assert_eq!(load_ledger(&vault, None)[&later.uid], ledger[&later.uid]);
        }

        #[test]
        fn a_confident_instance_or_an_expired_series_card_inherits_nothing() {
            let vault = vault("inherit-none");
            let (first, later) = (weekly(1), weekly(2));
            record_verdict(&vault, &later.uid, &later.title, DAY, "opportunity", "", "", "").unwrap();
            hand_card(&vault, "archive", "event-check-weekly.md", "executed", &first.uid, "lx:77");
            let before = seen(&vault);
            let mut ledger = load_ledger(&vault, None);
            assert_eq!(inherit(&vault, std::slice::from_ref(&later), &mut ledger).0, 0);
            assert_eq!(ledger[&later.uid].verdict.as_deref(), Some("opportunity"));
            assert_eq!(seen(&vault), before);

            let expired = self::vault("inherit-expired");
            unsure(&expired, &later.uid, &later.title, None);
            hand_card(&expired, "archive", "event-check-weekly.md", "expired", &first.uid, "lx:77");
            assert!(settled_series(&expired).is_empty());
            let before = seen(&expired);
            let mut ledger = load_ledger(&expired, None);
            assert_eq!(inherit(&expired, std::slice::from_ref(&later), &mut ledger).0, 0);
            assert_eq!(ledger[&later.uid].verdict.as_deref(), Some("unsure"));
            assert_eq!(seen(&expired), before);
        }

        #[test]
        fn inheriting_twice_writes_one_line() {
            let vault = vault("inherit-twice");
            let (first, later) = (weekly(1), weekly(2));
            unsure(&vault, &later.uid, &later.title, None);
            hand_card(&vault, "archive", "event-check-weekly.md", "executed", &first.uid, "lx:77");
            let mut ledger = load_ledger(&vault, None);
            assert_eq!(inherit(&vault, std::slice::from_ref(&later), &mut ledger).0, 1);
            let mut reloaded = load_ledger(&vault, None);
            assert_eq!(inherit(&vault, std::slice::from_ref(&later), &mut reloaded).0, 0);
            assert_eq!(inherit(&vault, std::slice::from_ref(&later), &mut ledger).0, 0, "in memory too");
            assert_eq!(seen(&vault).matches("- lx:77:2 · Weekly meeting · verdict:obligation").count(), 1);
        }

        // --- a card closed without an answer (controller ruling G1) -------------------------

        /// `process_approvals` on `today`, as `rank` (or the app's `decide`) runs it.
        fn settle_on(vault: &Path, today: Date) -> crate::approvals::ApprovalsResult {
            let ctx = crate::approvals::default_ctx();
            let mut journal = Journal::new(vault);
            process_approvals_at(vault, today, &ctx, &mut journal)
        }

        fn process_approvals_at(
            vault: &Path,
            today: Date,
            ctx: &WriteContext,
            journal: &mut Journal,
        ) -> crate::approvals::ApprovalsResult {
            crate::approvals::process_approvals(vault, today, today.at(12, 0, 0, 0), ctx, journal)
        }

        #[test]
        fn an_expired_or_unanswered_series_card_lets_a_later_instance_be_asked() {
            let vault = vault("g1-expired");
            let events = [
                in_series(on("lx:9:1", "Weekly lab", 10, 1, 19, 0, 21, 0), "lx:9"),
                in_series(on("lx:9:1b", "Weekly lab", 10, 5, 19, 0, 21, 0), "lx:9"),
                in_series(on("lx:9:2", "Weekly lab", 10, 15, 19, 0, 21, 0), "lx:9"),
            ];
            for e in &events {
                unsure(&vault, &e.uid, &e.title, None);
            }
            // 28 Sep: the first card lists the two instances inside the horizon.
            let (first, _) = check(&vault, &events, 15);
            assert_eq!(event_uids(&note(&first[0]).0), vec!["lx:9:1", "lx:9:1b"]);
            // 2 Oct: nobody answered, and the card expires with its soonest instance.
            assert_eq!(settle_on(&vault, date(2026, 10, 2)).expired.len(), 1);
            assert_eq!(cards(&vault, "archive").len(), 1);

            // 3 Oct: an expiry is not an answer. The series is asked again about its next
            // instance, but never about one the expired card already listed.
            let ledger = load_ledger(&vault, None);
            let (paths, count) = check_on(&vault, &events, &ledger, date(2026, 10, 3), 15);
            assert_eq!(count, 1, "{paths:?}");
            let (meta, _) = note(&paths[0]);
            assert_eq!(field(&meta, "source_uid").as_deref(), Some("lx:9:2"));
            assert_eq!(field(&meta, "series_uid").as_deref(), Some("lx:9"));
            assert_eq!(event_uids(&meta), vec!["lx:9:2"]);
            assert_eq!(date_field(&meta, "first_proposed_at"), Some(date(2026, 10, 3)));
            // The live card closes the series again, and the same instance is never asked twice.
            let ledger = load_ledger(&vault, None);
            assert_eq!(check_on(&vault, &events, &ledger, date(2026, 10, 3), 15).1, 0);
            assert_eq!(check_on(&vault, &events, &ledger, date(2026, 10, 4), 15).1, 0);
        }

        #[test]
        fn a_card_deleted_unanswered_reopens_its_series_but_never_its_listed_instances() {
            let vault = vault("g1-deleted");
            // `write::delete` from the app archives a card with its status left `pending`. No
            // `proposed` markers here: the card's own `events:` list is what keeps its instances
            // from being asked twice.
            fs::create_dir_all(vault.join("archive")).unwrap();
            let text = "---\ntype: approval\nkind: event-check\ntitle: \"Earlier card\"\n\
                        status: pending\nsource_uid: \"lx:6:1\"\nseries_uid: \"lx:6\"\n\
                        events:\n- \"lx:6:1\"\n- \"lx:6:2\"\n\
                        proposed_at: 2026-09-20\nfirst_proposed_at: 2026-09-20\n\
                        expires: 2026-09-29\nsnooze_until: null\ncreated_by: events\n---\n\nbody\n";
            pystr::write_text(&vault.join("archive").join("event-check-six.md"), text).unwrap();
            let events = [
                in_series(on("lx:6:2", "Reading group", 10, 2, 17, 0, 18, 0), "lx:6"),
                in_series(on("lx:6:3", "Reading group", 10, 3, 17, 0, 18, 0), "lx:6"),
            ];
            for e in &events {
                unsure(&vault, &e.uid, &e.title, None);
            }
            let (paths, count) = check(&vault, &events, 15);
            assert_eq!(count, 1, "{paths:?}");
            let (meta, _) = note(&paths[0]);
            assert_eq!(field(&meta, "source_uid").as_deref(), Some("lx:6:3"));
            assert_eq!(event_uids(&meta), vec!["lx:6:3"]);
        }

        #[test]
        fn an_answered_series_card_is_never_re_asked_and_later_instances_inherit() {
            let vault = vault("g1-answered");
            let events = [
                in_series(on("lx:8:1", "Weekly lab", 10, 1, 19, 0, 21, 0), "lx:8"),
                in_series(on("lx:8:2", "Weekly lab", 10, 15, 19, 0, 21, 0), "lx:8"),
            ];
            unsure(&vault, "lx:8:1", "Weekly lab", Some(JID_A));
            unsure(&vault, "lx:8:2", "Weekly lab", Some(JID_B));
            let (paths, _) = check(&vault, &events, 15);
            assert_eq!(event_uids(&note(&paths[0]).0), vec!["lx:8:1"]);
            // The student approves it in the console; the same pass settles it (F3).
            let rel = format!("approvals/{}", paths[0].file_name().unwrap().to_string_lossy());
            let mut journal = Journal::new(&vault);
            let console = WriteContext::new("quinn", "dashboard");
            let literals = vec![("status".to_string(), "approved".to_string())];
            crate::write::write_literals(&vault, &rel, &literals, &console, &mut journal, &Default::default())
                .unwrap();
            let ctx = crate::approvals::default_ctx();
            let settled = process_approvals_at(&vault, DAY, &ctx, &mut journal);
            assert_eq!(settled.executed.len(), 1, "{settled:?}");

            // 3 Oct: the later instance is in the horizon. Even before inheritance runs, the
            // answered series is closed.
            let today = date(2026, 10, 3);
            let mut ledger = load_ledger(&vault, None);
            assert_eq!(check_on(&vault, &events, &ledger, today, 15).1, 0);
            let (inherited, warnings) = inherit_series_answers(&vault, &events, &mut ledger, today);
            assert_eq!((inherited, warnings), (1, Vec::<String>::new()));
            assert_eq!(ledger["lx:8:2"].verdict.as_deref(), Some("obligation"));
            assert_eq!(ledger["lx:8:2"].answered_by, "quinn");
            assert_eq!(check_on(&vault, &events, &ledger, today, 15).1, 0);
            assert_eq!(cards(&vault, "approvals").len(), 0);
        }

        // --- obligation cards (T2a.1b) ------------------------------------------------------

        const WHY_OBL: &str = "the club's charter lists this meeting as mandatory";

        fn obligation(vault: &Path, uid: &str, title: &str, jid: Option<&str>) {
            record_judged_verdict(vault, uid, title, DAY, "obligation", WHY_OBL, jid).unwrap();
        }

        fn accept_on(
            vault: &Path,
            events: &[DiscoveredEvent],
            ledger: &BTreeMap<String, LedgerEntry>,
            today: Date,
            budget: i64,
        ) -> (Vec<PathBuf>, usize) {
            let mut journal = Journal::new(vault);
            let ctx = WriteContext::new("agent:events", "cli");
            emit_event_accepts(vault, events, ledger, &config(), today, budget, "obligation", &ctx, &mut journal)
        }

        fn accept(vault: &Path, events: &[DiscoveredEvent], budget: i64) -> (Vec<PathBuf>, usize) {
            let ledger = load_ledger(vault, None);
            accept_on(vault, events, &ledger, DAY, budget)
        }

        /// The `event-accept` cards in `folder`: `event-<slug>-<date>.md`, never `event-check-…`.
        fn accept_files(vault: &Path, folder: &str) -> Vec<PathBuf> {
            crate::approvals::sorted_md(&vault.join(folder))
                .into_iter()
                .filter(|p| {
                    p.file_name().and_then(|n| n.to_str()).is_some_and(|n| {
                        n.starts_with("event-") && !n.starts_with("event-check-")
                    })
                })
                .collect()
        }

        fn instances(meta: &Mapping) -> Vec<Mapping> {
            match crate::yaml::get(meta, "instances") {
                Some(Value::Sequence(items)) => {
                    items.iter().filter_map(|v| v.as_mapping().cloned()).collect()
                }
                _ => Vec::new(),
            }
        }

        /// A card written by hand: `extra` is whole frontmatter lines, each ending `\n`.
        fn hand(vault: &Path, folder: &str, name: &str, kind: &str, status: &str, extra: &str) {
            fs::create_dir_all(vault.join(folder)).unwrap();
            let text = format!(
                "---\ntype: approval\nkind: {kind}\ntitle: \"Earlier card\"\nstatus: {status}\n{extra}\
                 proposed_at: 2026-09-20\nfirst_proposed_at: 2026-09-20\nexpires: 2026-09-24\n\
                 snooze_until: null\ncreated_by: events\n---\n\nbody\n"
            );
            pystr::write_text(&vault.join(folder).join(name), &text).unwrap();
        }

        /// A card first proposed on `DAY`, so it counts toward that day's cap.
        fn hand_today(vault: &Path, name: &str, kind: &str, verdict: &str, uid: &str) {
            fs::create_dir_all(vault.join("approvals")).unwrap();
            let text = format!(
                "---\ntype: approval\nkind: {kind}\ntitle: \"Today's card\"\nstatus: pending\n\
                 verdict: {verdict}\nsource_uid: \"{uid}\"\nseries_uid: \"{uid}\"\nevents:\n- \"{uid}\"\n\
                 proposed_at: 2026-09-28\nfirst_proposed_at: 2026-09-28\nexpires: 2026-10-30\n\
                 snooze_until: null\ncreated_by: events\n---\n\nbody\n"
            );
            pystr::write_text(&vault.join("approvals").join(name), &text).unwrap();
        }

        #[test]
        fn an_obligation_files_one_event_accept_card() {
            let vault = vault("oblig");
            let mut first = on("lx:77:1", "Career fair", 10, 1, 10, 0, 15, 0);
            first.location = "Ferguson Center".into();
            first.organizer = "Career Center".into();
            first.url = "https://example.edu/fair".into();
            first.registration = true;
            first.registration_deadline = Some(date(2026, 9, 30));
            let second = on("lx:77:2", "Career fair", 10, 8, 10, 0, 15, 0);
            let events = [in_series(second, "lx:77"), in_series(first, "lx:77")];
            obligation(&vault, "lx:77:1", "Career fair", Some(JID_A));
            obligation(&vault, "lx:77:2", "Career fair", Some(JID_B));

            let (paths, count) = accept(&vault, &events, 15);
            assert_eq!(count, 1);
            assert_eq!(paths, vec![vault.join("approvals").join("event-career-fair-2026-10-01.md")]);
            let (meta, body) = note(&paths[0]);
            assert_eq!(field(&meta, "type").as_deref(), Some("approval"));
            assert_eq!(field(&meta, "kind").as_deref(), Some("event-accept"));
            assert_eq!(field(&meta, "verdict").as_deref(), Some("obligation"));
            assert_eq!(
                field(&meta, "title").as_deref(),
                Some("Required · Career fair · Thu 1 Oct 10am–3pm · +1 more")
            );
            assert_eq!(field(&meta, "status").as_deref(), Some("pending"));
            assert_eq!(field(&meta, "source_uid").as_deref(), Some("lx:77:1"));
            assert_eq!(field(&meta, "series_uid").as_deref(), Some("lx:77"));
            assert_eq!(event_uids(&meta), vec!["lx:77:1", "lx:77:2"]);
            assert_eq!(field(&meta, "judgment_id").as_deref(), Some(JID_A));
            assert_eq!(field(&meta, "judgment_kind").as_deref(), Some("event"));
            assert_eq!(date_field(&meta, "first_proposed_at"), Some(DAY));
            assert_eq!(date_field(&meta, "expires"), Some(date(2026, 10, 1)), "the primary's date");
            assert!(matches!(crate::yaml::get(&meta, "snooze_until"), Some(Value::Null)));
            assert_eq!(field(&meta, "created_by").as_deref(), Some("events"));
            let id = field(&meta, "id").unwrap_or_default();
            assert!(crate::ids::ID_RE.is_match(&id), "id was {id:?}");

            // `instances:` (P5): one mapping per listed uid, the primary first, with the url.
            let listed = instances(&meta);
            let uids: Vec<_> = listed.iter().filter_map(|m| field(m, "uid")).collect();
            assert_eq!(uids, vec!["lx:77:1", "lx:77:2"]);
            assert_eq!(field(&listed[0], "url").as_deref(), Some("https://example.edu/fair"));
            assert_eq!(field(&listed[0], "location").as_deref(), Some("Ferguson Center"));
            assert_eq!(field(&listed[0], "start").as_deref(), Some("2026-10-01T10:00"));
            assert_eq!(field(&listed[0], "end").as_deref(), Some("2026-10-01T15:00"));
            assert_eq!(field(&listed[0], "registration_deadline").as_deref(), Some("2026-09-30"));

            let paragraphs: Vec<&str> = body.split("\n\n").map(str::trim).filter(|p| !p.is_empty()).collect();
            assert_eq!(paragraphs[0], format!("**Knowlu thinks this is required of you.** {WHY_OBL}"));
            assert_eq!(
                paragraphs[1],
                "Thu 1 Oct 10am–3pm · Ferguson Center · Career Center\n\
                 https://example.edu/fair\nAlso on: Thu 8 Oct 10am–3pm"
            );
            assert!(
                body.contains("Reject and it's dropped. Either way you won't be asked again."),
                "{body}"
            );
            assert!(!body.contains("meta-bind-button") && !body.contains(BUTTONS), "{body}");

            // The card is a journaled create by the agent, and the `proposed` lines follow it.
            assert_eq!(
                journal_creates(&vault),
                vec![(
                    "approvals/event-career-fair-2026-10-01.md".to_string(),
                    "agent:events".to_string()
                )]
            );
            let ledger = load_ledger(&vault, None);
            assert!(ledger["lx:77:1"].proposed && ledger["lx:77:2"].proposed);

            // A create that fails leaves no line: `approvals` is a file here.
            let blocked = self::vault("oblig-blocked");
            fs::write(blocked.join("approvals"), "not a folder").unwrap();
            obligation(&blocked, "lx:77:1", "Career fair", None);
            assert_eq!(accept(&blocked, &events, 15).1, 0);
            assert!(!load_ledger(&blocked, None)["lx:77:1"].proposed);
        }

        #[test]
        fn obligation_cards_are_capped_at_three_a_day_beside_the_checks() {
            let vault = vault("oblig-cap");
            let events: Vec<DiscoveredEvent> = (0..4)
                .map(|i| on(&format!("ics:o{i}"), &format!("Meeting {i}"), 10, 1 + i as i8, 18, 0, 19, 0))
                .collect();
            for e in &events {
                obligation(&vault, &e.uid, &e.title, None);
            }
            // Three `event-check` cards first proposed today do not reduce the allowance.
            for i in 0..3 {
                hand_today(&vault, &format!("event-check-x{i}.md"), "event-check", "unsure", &format!("ics:c{i}"));
            }
            let (paths, count) = accept(&vault, &events, 15);
            assert_eq!(count, 3);
            let asked: Vec<String> = paths.iter().filter_map(|p| field(&note(p).0, "source_uid")).collect();
            assert_eq!(asked, vec!["ics:o0", "ics:o1", "ics:o2"], "soonest first");
            assert_eq!(accept(&vault, &events, 15).1, 0, "the same day's second call");

            // The budget bounds it, and a card of today's does not leak into the other kind.
            let small = self::vault("oblig-cap-budget");
            for e in &events {
                obligation(&small, &e.uid, &e.title, None);
            }
            assert_eq!(accept(&small, &events, 2).1, 2);
            assert_eq!(accept(&small, &events, 0).1, 0);
            assert_eq!(accept(&small, &events, -1).1, 0);

            // Three obligation cards today leave `event-check` its own three.
            let other = self::vault("oblig-cap-checks");
            for i in 0..3 {
                hand_today(&other, &format!("event-o{i}.md"), "event-accept", "obligation", &format!("ics:p{i}"));
            }
            let fair = on("ics:fair", "Career fair", 10, 1, 10, 0, 15, 0);
            unsure(&other, "ics:fair", "Career fair", None);
            assert_eq!(check(&other, std::slice::from_ref(&fair), 15).1, 1);
        }

        #[test]
        fn never_ask_twice_across_both_kinds() {
            let vault = vault("never-twice");
            // Each skipped event stands in its own series, so only the named rule can close it.
            let skipped = [
                "ics:src",    // source_uid of an event-accept card in approvals/
                "ics:listed", // in `events:` of an event-check card in archive/
                "ics:prop",   // a `proposed` line
                "ics:decl",   // a `declined` line
                "s1:2",       // a live event-check card for its series
                "s2:2",       // an executed event-accept card for its series
                "s3:2",       // a rejected event-accept card for its series
                "s4:2",       // an executed event-check card for its series
                "s6:2",       // a live event-accept card for its series
            ];
            let mut events: Vec<DiscoveredEvent> = skipped
                .iter()
                .enumerate()
                .map(|(i, uid)| {
                    let event = on(uid, "Meeting", 10, 1 + (i % 10) as i8, 18, 0, 19, 0);
                    match uid.split_once(':') {
                        Some((s, _)) if s.starts_with('s') => in_series(event, s),
                        _ => event,
                    }
                })
                .collect();
            // The expired card closes only the instance it listed.
            events.push(in_series(on("s5:1", "Meeting", 10, 3, 18, 0, 19, 0), "s5"));
            events.push(in_series(on("s5:2", "Meeting", 10, 4, 18, 0, 19, 0), "s5"));
            events.push(on("ics:free", "Meeting", 10, 5, 18, 0, 19, 0));
            for e in &events {
                obligation(&vault, &e.uid, &e.title, None);
            }
            record_proposed(&vault, "ics:prop", DAY).unwrap();
            crate::eventledger::record_declined(&vault, "ics:decl", DAY).unwrap();
            let series = |s: &str| format!("series_uid: \"{s}\"\n");
            hand(&vault, "approvals", "event-a.md", "event-accept", "pending", "source_uid: \"ics:src\"\n");
            hand(&vault, "archive", "event-check-b.md", "event-check", "expired",
                 "source_uid: \"ics:other\"\nevents:\n- \"ics:other\"\n- \"ics:listed\"\n");
            hand(&vault, "approvals", "event-check-c.md", "event-check", "pending", &series("s1"));
            hand(&vault, "archive", "event-d.md", "event-accept", "executed", &series("s2"));
            hand(&vault, "archive", "event-e.md", "event-accept", "rejected", &series("s3"));
            hand(&vault, "archive", "event-check-f.md", "event-check", "executed", &series("s4"));
            hand(&vault, "approvals", "event-g.md", "event-accept", "pending", &series("s6"));
            hand(&vault, "archive", "event-h.md", "event-accept", "expired",
                 &format!("source_uid: \"s5:1\"\nevents:\n- \"s5:1\"\n{}", series("s5")));
            // The answered `event-accept` series are the helper's, not this module's.
            let answered = crate::eventcarry::answered_series(&vault);
            assert_eq!(answered.keys().map(String::as_str).collect::<Vec<_>>(), ["s2", "s3"]);

            let (paths, count) = accept(&vault, &events, 15);
            let asked: Vec<String> = paths.iter().filter_map(|p| field(&note(p).0, "source_uid")).collect();
            assert_eq!(asked, vec!["s5:2", "ics:free"], "{count}");

            // The same rules close an `event-check` question: an answered event-accept series
            // is never asked about by an `unsure` instance of it.
            let unsure_events = [in_series(on("s2:9", "Meeting", 10, 6, 18, 0, 19, 0), "s2")];
            unsure(&vault, "s2:9", "Meeting", None);
            assert_eq!(check(&vault, &unsure_events, 15).1, 0);
        }

        #[test]
        fn a_series_files_one_card_of_at_most_twenty() {
            let vault = vault("twenty");
            let events: Vec<DiscoveredEvent> = (0..25)
                .map(|i| {
                    let (day, hour) = (1 + (i / 10) as i8, 8 + (i % 10) as i8);
                    in_series(on(&format!("lx:5:{i:02}"), "Study hall", 10, day, hour, 0, hour + 1, 0), "lx:5")
                })
                .collect();
            for e in &events {
                obligation(&vault, &e.uid, &e.title, None);
            }
            let (paths, count) = accept(&vault, &events, 15);
            assert_eq!(count, 1);
            let (meta, _) = note(&paths[0]);
            assert_eq!(field(&meta, "title").as_deref(), Some("Required · Study hall · Thu 1 Oct 8–9am · +19 more"));
            assert_eq!(event_uids(&meta).len(), 20);
            assert_eq!(instances(&meta).len(), 20);
            assert_eq!(field(&meta, "source_uid").as_deref(), Some("lx:5:00"));
            let ledger = load_ledger(&vault, None);
            assert_eq!(ledger.values().filter(|e| e.proposed).count(), 20);
            assert!(!ledger["lx:5:24"].proposed, "the 21st instance is not on the card");
        }

        #[test]
        fn new_event_check_cards_carry_instances_and_the_new_closing() {
            let vault = vault("check-instances");
            let fair = on("ics:fair-1", "Career fair", 10, 1, 10, 0, 15, 0);
            let talk = in_series(on("lx:9:2", "Weekly lab", 10, 6, 19, 0, 21, 0), "lx:9");
            let lab = in_series(on("lx:9:1", "Weekly lab", 10, 2, 19, 0, 21, 0), "lx:9");
            for e in [&fair, &talk, &lab] {
                unsure(&vault, &e.uid, &e.title, None);
            }
            let (paths, count) = check(&vault, &[fair, talk, lab], 15);
            assert_eq!(count, 2);
            let (meta, body) = note(&paths[0]);
            let uids: Vec<_> = instances(&meta).iter().filter_map(|m| field(m, "uid")).collect();
            assert_eq!(uids, vec!["ics:fair-1"]);
            let (meta, body2) = note(&paths[1]);
            let uids: Vec<_> = instances(&meta).iter().filter_map(|m| field(m, "uid")).collect();
            assert_eq!(uids, vec!["lx:9:1", "lx:9:2"]);
            for text in [body, body2] {
                assert!(
                    text.contains(
                        "Approve if it applies to you: it goes on your schedule for that day. \
                         Reject and it's dropped. Either way you won't be asked again."
                    ),
                    "{text}"
                );
                assert!(!text.contains("joins Coming up"), "{text}");
            }
        }

        #[test]
        fn event_accept_is_not_a_local_card_kind_and_its_notes_sync() {
            assert!(!crate::commitments::LOCAL_CARD_KINDS.contains(&"event-accept"));
            assert!(!crate::commitments::LOCAL_CARD_KINDS.contains(&"event-check"));
            assert!(crate::ids::NOTE_FOLDERS.contains(&"commitments"));
            assert!(crate::ids::NOTE_FOLDERS.contains(&"tasks"));
        }

        #[test]
        fn a_roster_read_event_files_no_event_accept_card_and_no_instances() {
            let vault = vault("roster");
            // As `read_roster` builds them: `source: "roster"`, a zero-length event read back as
            // 15:00-16:00.
            let roster = |uid: &str, title: &str, day: i8, h: i8, eh: i8| {
                let mut e = on(uid, title, 10, day, h, 0, eh, 0);
                e.source = "roster".into();
                e
            };
            let events = [
                roster("ics:zero", "Info table", 1, 15, 16),
                roster("ics:late", "Chapter meeting", 2, 18, 19),
                roster("ics:ask", "Open lab", 3, 10, 11),
            ];
            obligation(&vault, "ics:zero", "Info table", None);
            obligation(&vault, "ics:late", "Chapter meeting", None);
            unsure(&vault, "ics:ask", "Open lab", None);
            let (paths, count) = accept(&vault, &events, 15);
            assert!(paths.is_empty() && count == 0, "{paths:?}");
            assert!(accept_files(&vault, "approvals").is_empty());
            let ledger = load_ledger(&vault, None);
            assert!(!ledger["ics:zero"].proposed && !ledger["ics:late"].proposed);

            // The unsure roster event still gets its question, with no `instances:` key.
            let (checks, count) = check(&vault, &events, 15);
            assert_eq!(count, 1);
            assert_eq!(field(&note(&checks[0]).0, "source_uid").as_deref(), Some("ics:ask"));
            assert!(crate::yaml::get(&note(&checks[0]).0, "instances").is_none());
            assert!(!pystr::read_text(&checks[0]).unwrap().contains("instances"));
            // Approving it books nothing (D11), so it never promises a schedule entry.
            let body = note(&checks[0]).1;
            assert!(!body.contains("goes on your schedule"), "{body}");
            assert!(
                body.contains(
                    "Approve if it applies to you: it joins Coming up as something you're \
                     expected at. Reject and it's dropped. Either way you won't be asked again."
                ),
                "{body}"
            );

            // The same events from a feed file their cards.
            let fed: Vec<DiscoveredEvent> = events
                .iter()
                .cloned()
                .map(|mut e| {
                    e.source = "campus".into();
                    e
                })
                .collect();
            let other = self::vault("roster-fed");
            obligation(&other, "ics:zero", "Info table", None);
            obligation(&other, "ics:late", "Chapter meeting", None);
            let (paths, count) = accept(&other, &fed, 15);
            assert_eq!(count, 2);
            assert!(paths.iter().all(|p| !instances(&note(p).0).is_empty()));
        }

        // --- opportunity cards (T2a.2) ------------------------------------------------------

        const WHY_OPP: &str = "a talk in your major";

        fn cards_on() -> EventsConfig {
            EventsConfig { event_cards: true, ..EventsConfig::default() }
        }

        /// `emit_event_accepts` for `verdict` under `config`, over `ledger`, today being `DAY`.
        fn accept_for(
            vault: &Path,
            events: &[DiscoveredEvent],
            ledger: &BTreeMap<String, LedgerEntry>,
            config: &EventsConfig,
            budget: i64,
            verdict: &str,
        ) -> (Vec<PathBuf>, usize) {
            let mut journal = Journal::new(vault);
            let ctx = WriteContext::new("agent:events", "cli");
            emit_event_accepts(vault, events, ledger, config, DAY, budget, verdict, &ctx, &mut journal)
        }

        fn opportunity(vault: &Path, uid: &str, title: &str, jid: Option<&str>) {
            record_judged_verdict(vault, uid, title, DAY, "opportunity", WHY_OPP, jid).unwrap();
        }

        fn source_uids(paths: &[PathBuf]) -> Vec<String> {
            paths.iter().filter_map(|p| field(&note(p).0, "source_uid")).collect()
        }

        #[test]
        fn an_opportunity_files_a_card_only_when_event_cards_is_on() {
            let vault = vault("opp-switch");
            let mut talk = on("ics:talk", "Engineering talk", 10, 6, 17, 0, 18, 0);
            talk.location = "Hardaway Hall".into();
            talk.url = "https://example.edu/talk".into();
            let events = [talk];
            opportunity(&vault, "ics:talk", "Engineering talk", Some(JID_A));
            let ledger = load_ledger(&vault, None);

            // Off: nothing is filed and no `proposed` line is written, so the digest still owns it.
            let (paths, count) = accept_for(&vault, &events, &ledger, &config(), 15, "opportunity");
            assert!(paths.is_empty() && count == 0, "{paths:?}");
            assert!(accept_files(&vault, "approvals").is_empty());
            assert!(!load_ledger(&vault, None)["ics:talk"].proposed);

            // On, but asked for obligations: an opportunity is not one.
            assert_eq!(accept_for(&vault, &events, &ledger, &cards_on(), 15, "obligation").1, 0);

            // On: one card.
            let (paths, count) = accept_for(&vault, &events, &ledger, &cards_on(), 15, "opportunity");
            assert_eq!(count, 1);
            assert_eq!(paths, vec![vault.join("approvals").join("event-engineering-talk-2026-10-06.md")]);
            let (meta, body) = note(&paths[0]);
            assert_eq!(field(&meta, "kind").as_deref(), Some("event-accept"));
            assert_eq!(field(&meta, "verdict").as_deref(), Some("opportunity"));
            assert_eq!(
                field(&meta, "title").as_deref(),
                Some("Worth a look · Engineering talk · Tue 6 Oct 5–6pm")
            );
            assert_eq!(field(&meta, "judgment_id").as_deref(), Some(JID_A));
            assert_eq!(field(&meta, "judgment_kind").as_deref(), Some("event"));
            assert_eq!(instances(&meta).len(), 1);
            let paragraphs: Vec<&str> = body.split("\n\n").map(str::trim).filter(|p| !p.is_empty()).collect();
            assert_eq!(paragraphs[0], format!("**This may interest you.** {WHY_OPP}"));
            assert_eq!(paragraphs[1], "Tue 6 Oct 5–6pm · Hardaway Hall\nhttps://example.edu/talk");
            assert!(body.contains("Reject and it's dropped. Either way you won't be asked again."), "{body}");
            assert!(!body.contains("meta-bind-button") && !body.contains(BUTTONS), "{body}");
            let after = load_ledger(&vault, None);
            assert!(after["ics:talk"].proposed, "the `proposed` line follows the card");
            assert_eq!(accept_for(&vault, &events, &after, &cards_on(), 15, "opportunity").1, 0);
        }

        #[test]
        fn opportunity_cards_are_capped_at_three_a_day() {
            let events: Vec<DiscoveredEvent> = (0..5)
                .map(|i| on(&format!("ics:p{i}"), &format!("Talk {i}"), 10, 1 + i as i8, 17, 0, 18, 0))
                .collect();
            let seeded = |name: &str| {
                let vault = vault(name);
                for e in &events {
                    opportunity(&vault, &e.uid, &e.title, None);
                }
                vault
            };

            // Three already first proposed today: none.
            let full = seeded("opp-cap-full");
            for i in 0..3 {
                hand_today(&full, &format!("event-x{i}.md"), "event-accept", "opportunity", &format!("ics:q{i}"));
            }
            let ledger = load_ledger(&full, None);
            assert_eq!(accept_for(&full, &events, &ledger, &cards_on(), 15, "opportunity").1, 0);

            // Two: one more. An obligation card and a check of today leave the allowance alone.
            let two = seeded("opp-cap-two");
            for i in 0..2 {
                hand_today(&two, &format!("event-x{i}.md"), "event-accept", "opportunity", &format!("ics:q{i}"));
            }
            hand_today(&two, "event-o0.md", "event-accept", "obligation", "ics:o0");
            hand_today(&two, "event-check-c0.md", "event-check", "unsure", "ics:c0");
            let ledger = load_ledger(&two, None);
            assert_eq!(accept_for(&two, &events, &ledger, &cards_on(), 15, "opportunity").1, 1);

            // None today: three, and the budget bounds it.
            let none = seeded("opp-cap-none");
            let ledger = load_ledger(&none, None);
            assert_eq!(accept_for(&none, &events, &ledger, &cards_on(), 15, "opportunity").1, 3);
            let small = seeded("opp-cap-budget");
            let ledger = load_ledger(&small, None);
            assert_eq!(accept_for(&small, &events, &ledger, &cards_on(), 2, "opportunity").1, 2);
            assert_eq!(accept_for(&small, &events, &ledger, &cards_on(), 0, "opportunity").1, 0);

            // Three opportunity cards today leave the obligations their own three.
            let other = vault("opp-cap-oblig");
            for i in 0..3 {
                hand_today(&other, &format!("event-x{i}.md"), "event-accept", "opportunity", &format!("ics:q{i}"));
            }
            let fair = on("ics:fair", "Career fair", 10, 1, 10, 0, 15, 0);
            obligation(&other, "ics:fair", "Career fair", None);
            let ledger = load_ledger(&other, None);
            assert_eq!(accept_for(&other, std::slice::from_ref(&fair), &ledger, &cards_on(), 15, "obligation").1, 1);
        }

        /// Every card under `dir`, with its text, in path order, `id:` lines removed.
        fn tree_without_ids(dir: &Path) -> Vec<(String, String)> {
            crate::approvals::sorted_md(dir)
                .into_iter()
                .map(|path| {
                    let text = pystr::read_text(&path).unwrap();
                    let kept: Vec<&str> = text.split('\n').filter(|l| !l.starts_with("id:")).collect();
                    (path.file_name().unwrap().to_string_lossy().into_owned(), kept.join("\n"))
                })
                .collect()
        }

        fn copy_tree(from: &Path, to: &Path) {
            fs::create_dir_all(to).unwrap();
            for entry in fs::read_dir(from).unwrap().filter_map(|e| e.ok()) {
                let target = to.join(entry.file_name());
                if entry.path().is_dir() {
                    copy_tree(&entry.path(), &target);
                } else {
                    fs::copy(entry.path(), target).unwrap();
                }
            }
        }

        #[test]
        fn cards_come_out_in_their_order_and_the_same_input_writes_the_same_bytes() {
            // Obligations and checks come out in (start, uid): two of each share a start, so the
            // uid decides.
            let obligations = [
                on("ics:ob-b", "Meeting B", 10, 2, 18, 0, 19, 0),
                on("ics:ob-c", "Meeting C", 10, 1, 18, 0, 19, 0),
                on("ics:ob-a", "Meeting A", 10, 2, 18, 0, 19, 0),
            ];
            let checks = [
                on("ics:ck-2", "Mixer", 10, 4, 12, 0, 13, 0),
                on("ics:ck-1", "Social", 10, 4, 12, 0, 13, 0),
                on("ics:ck-0", "Fair", 10, 5, 9, 0, 10, 0),
            ];
            // Opportunities by (strength, registration deadline, start, uid): the digest's
            // `sort_key`. Two strong ones carry deadlines; the rest have none.
            let mut d1 = on("ics:op-d1", "Panel D1", 10, 3, 17, 0, 18, 0);
            d1.registration = true;
            d1.registration_deadline = Some(date(2026, 10, 2));
            let mut d2 = on("ics:op-d2", "Panel D2", 10, 2, 17, 0, 18, 0);
            d2.registration = true;
            d2.registration_deadline = Some(date(2026, 10, 3));
            let opps = [
                on("ics:op-m", "Mild early", 10, 1, 17, 0, 18, 0),
                on("ics:op-s2", "Strong late", 10, 9, 17, 0, 18, 0),
                d2,
                on("ics:op-s1", "Strong tie", 10, 9, 17, 0, 18, 0),
                d1,
            ];
            let strengths = [
                ("ics:op-m", "mild"),
                ("ics:op-s2", "strong"),
                ("ics:op-d1", "strong"),
                ("ics:op-d2", "strong"),
                ("ics:op-s1", "strong"),
            ];
            let mut events: Vec<DiscoveredEvent> = Vec::new();
            events.extend(obligations.iter().cloned());
            events.extend(checks.iter().cloned());
            events.extend(opps.iter().cloned());

            let vault_a = vault("order-a");
            for e in &obligations {
                obligation(&vault_a, &e.uid, &e.title, None);
            }
            for e in &checks {
                unsure(&vault_a, &e.uid, &e.title, None);
            }
            for e in &opps {
                opportunity(&vault_a, &e.uid, &e.title, None);
            }
            let vault_b = vault("order-b");
            copy_tree(&vault_a, &vault_b);

            // `rank`'s order: obligations, then checks, then opportunities.
            let run = |vault: &Path| {
                let mut ledger = load_ledger(vault, None);
                for (uid, strength) in strengths {
                    ledger.get_mut(uid).unwrap().strength = strength.to_string();
                }
                vec![
                    source_uids(&accept_for(vault, &events, &ledger, &cards_on(), 15, "obligation").0),
                    source_uids(&check_on(vault, &events, &ledger, DAY, 15).0),
                    source_uids(&accept_for(vault, &events, &ledger, &cards_on(), 15, "opportunity").0),
                ]
            };
            let asked = run(&vault_a);
            assert_eq!(asked[0], vec!["ics:ob-c", "ics:ob-a", "ics:ob-b"], "(start, uid)");
            assert_eq!(asked[1], vec!["ics:ck-1", "ics:ck-2", "ics:ck-0"], "(start, uid)");
            assert_eq!(
                asked[2],
                vec!["ics:op-d1", "ics:op-d2", "ics:op-s1"],
                "strength, registration deadline, start, uid; the cap of three cuts the rest"
            );

            // Two runs over copies of one vault write the same output, as P14 reads it: the same
            // file names in the same order, each card's bytes equal once its `id:` line is
            // removed, and the same `proposed` ledger lines. Journal timestamps and `id:` values
            // are left out on purpose: ids are random and journal times are the wall clock, both
            // contracts. No deterministic id is introduced to make this pass.
            assert_eq!(run(&vault_b), asked);
            let cards_a = tree_without_ids(&vault_a.join("approvals"));
            assert_eq!(cards_a.len(), 9);
            assert_eq!(cards_a, tree_without_ids(&vault_b.join("approvals")));
            assert_eq!(seen(&vault_a), seen(&vault_b));
            assert!(seen(&vault_a).contains("proposed"), "{}", seen(&vault_a));
        }

        #[test]
        fn expiry_is_the_event_date_or_fourteen_days_for_an_opportunity() {
            let vault = vault("opp-expiry");
            // DAY is 28 Sep. A registration deadline opens an event early (`horizon_start`).
            let far = |uid: &str, title: &str| {
                let mut e = on(uid, title, 10, 20, 17, 0, 18, 0);
                e.registration = true;
                e.registration_deadline = Some(date(2026, 10, 1));
                e
            };
            let soon = on("ics:soon", "Soon talk", 10, 5, 17, 0, 18, 0);
            let events = [far("ics:far-opp", "Far talk"), soon.clone(), far("ics:far-obl", "Far meeting")];
            opportunity(&vault, "ics:far-opp", "Far talk", None);
            opportunity(&vault, "ics:soon", "Soon talk", None);
            obligation(&vault, "ics:far-obl", "Far meeting", None);
            let ledger = load_ledger(&vault, None);

            let (far_opp, _) = accept_for(&vault, &events, &ledger, &cards_on(), 15, "opportunity");
            // The deadline sorts the far one first, as the digest's `sort_key` would.
            assert_eq!(source_uids(&far_opp), vec!["ics:far-opp".to_string(), "ics:soon".to_string()]);
            let expires = |p: &PathBuf| date_field(&note(p).0, "expires").unwrap();
            assert_eq!(expires(&far_opp[0]), date(2026, 10, 12), "first_proposed_at + 14 comes first");
            assert_eq!(expires(&far_opp[1]), date(2026, 10, 5), "the event comes first");

            let (obl, _) = accept_for(&vault, &events, &ledger, &cards_on(), 15, "obligation");
            assert_eq!(source_uids(&obl), vec!["ics:far-obl".to_string()]);
            assert_eq!(expires(&obl[0]), date(2026, 10, 20), "an obligation expires on its primary's date");
        }

        #[test]
        fn a_roster_read_opportunity_files_no_card() {
            let vault = vault("opp-roster");
            // As `read_roster` builds it: `source: "roster"`, a past-midnight event read back as
            // 22:00-23:00.
            let mut late = on("ics:late", "Late show", 10, 2, 22, 0, 23, 0);
            late.source = "roster".into();
            opportunity(&vault, "ics:late", "Late show", None);
            let ledger = load_ledger(&vault, None);

            let (paths, count) = accept_for(&vault, std::slice::from_ref(&late), &ledger, &cards_on(), 15, "opportunity");
            assert!(paths.is_empty() && count == 0, "{paths:?}");
            assert!(accept_files(&vault, "approvals").is_empty());
            assert!(!load_ledger(&vault, None)["ics:late"].proposed, "so the next fetched run can still ask");

            // The same event from a feed files one.
            let mut fed = late.clone();
            fed.source = "campus".into();
            let (paths, count) = accept_for(&vault, &[fed], &ledger, &cards_on(), 15, "opportunity");
            assert_eq!(count, 1);
            assert!(!instances(&note(&paths[0]).0).is_empty());
        }
    }
}
