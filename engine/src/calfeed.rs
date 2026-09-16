//! Google Calendar busy-time ingest — port of `engine/calfeed.py`.
//!
//! Wave 4 of the Rust port. Two jobs: expand an ICS feed's events (including a deliberately
//! small subset of RRULE) into busy blocks inside a 28-day window, and keep `state/calendar.md`
//! as a snapshot so an unreachable feed degrades to yesterday's answer instead of to nothing.
//!
//! # The rules this module is written against
//!
//! - **A feed that fails must never blank the vault.** Every failure path — no `ics_url`, a
//!   fetch error, a 200 response that is not ICS, a malformed feed entry — falls back to the
//!   snapshot and warns. Live-verified 2026-08-26, when Google returned HTTP 429 and the run
//!   carried on.
//! - **An unsupported recurrence must be visible, not silent** — except when the rule's `UNTIL`
//!   already passed, because a rule that can no longer produce busy time is not worth a WARN
//!   twice a day forever. A MONTHLY rule in the live feed is a permanent, deliberate WARN.
//! - **Naive wall-clock arithmetic throughout.** Python does its recurrence maths on naive
//!   datetimes, so `first + timedelta(days=n)` never shifts across a DST boundary. `jiff`'s
//!   `civil::DateTime` has exactly those semantics; using a zoned type here would move every
//!   recurring event by an hour twice a year.
//!
//! # Deviations from the Python, recorded rather than hidden
//!
//! 1. **Tie order in the returned event list.** Python collects into a `set` and sorts by
//!    `(start, end, title)`, so two events differing only in `all_day` come out in set-iteration
//!    order — which is randomised by `PYTHONHASHSEED`, i.e. unspecified. This port collects into
//!    a `BTreeSet` keyed `(start, end, title, all_day)`, which agrees with Python's sort on the
//!    three keys it specifies and is deterministic on the fourth. Strictly narrower, never wider.
//! 2. **Absurd `INTERVAL` values.** Python's `int()` accepts underscores (`1_0` → 10) and this
//!    port does not, and a `timedelta` overflow that Python turns into a `bad feed entry` warning
//!    ends iteration here instead. Both need an INTERVAL no real calendar emits.
//! 3. **Fetch-error text.** `urllib` and `ureq` word their errors differently, so
//!    `"{name}: fetch failed ({err}); using snapshot"` differs in the parenthesised half. That
//!    string reaches `state/runner-log.md`, never `state/today.md`, so the oracle is unaffected.

use std::collections::{BTreeMap, BTreeSet};
use std::path::Path;
use std::sync::LazyLock;

use jiff::civil::{Date, DateTime, Time};
use jiff::tz::TimeZone;
use jiff::Span;
use regex::Regex;

use crate::ingest::{parse_dt, parse_property, unescape, unfold, Due};
use crate::pystr;
use crate::weekcal::CalEvent;

pub const HORIZON_DAYS: i64 = 28;
pub const MAX_OCCURRENCES: usize = 1000;

pub const SNAPSHOT_HEADER: &str = "# Calendar snapshot — rewritten on every successful fetch; \
fallback source when a feed is unreachable.";

// `[^·]+?` is non-greedy so a title containing `·` still parses: the feed and span are the two
// dot-free fields, and everything after the second separator is the title.
static LINE: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"^- (?P<feed>[^·]+?) · (?P<span>[^·]+?) · (?P<title>.+)$").unwrap());
// The separator inside a span is an EN DASH (U+2013), not a hyphen. The field separator above is
// a MIDDLE DOT (U+00B7). Both are multi-byte in UTF-8 — never index into these strings by byte.
static ALL_DAY: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"^(?P<d1>\d{4}-\d{2}-\d{2})–(?P<d2>\d{4}-\d{2}-\d{2}) \(all day\)$").unwrap()
});
static SAME_DAY: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"^(?P<d>\d{4}-\d{2}-\d{2}) (?P<s>\d{2}:\d{2})–(?P<e>\d{2}:\d{2})$").unwrap()
});
static CROSS_DAY: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(
        r"^(?P<d1>\d{4}-\d{2}-\d{2}) (?P<s>\d{2}:\d{2})–(?P<d2>\d{4}-\d{2}-\d{2}) (?P<e>\d{2}:\d{2})$",
    )
    .unwrap()
});

/// Monday is 0, matching Python's `datetime.weekday()`.
fn day_code(code: &str) -> Option<i64> {
    match code {
        "MO" => Some(0),
        "TU" => Some(1),
        "WE" => Some(2),
        "TH" => Some(3),
        "FR" => Some(4),
        "SA" => Some(5),
        "SU" => Some(6),
        _ => None,
    }
}

const SUPPORTED_FREQ: [&str; 2] = ["DAILY", "WEEKLY"];
const RRULE_ALLOWED_KEYS: [&str; 5] = ["FREQ", "INTERVAL", "BYDAY", "UNTIL", "COUNT"];

/// Why a recurrence could not be expanded. The payload is interpolated verbatim into
/// `skipped recurrence ({why}): {title}`, so these strings are part of the contract.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct UnsupportedRule(pub String);

impl std::fmt::Display for UnsupportedRule {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

fn rrule_dict(value: &str) -> BTreeMap<String, String> {
    let mut out = BTreeMap::new();
    for part in value.split(';') {
        if let Some((key, val)) = part.split_once('=') {
            out.insert(key.to_uppercase(), val.to_string());
        }
    }
    out
}

/// Python's `int(text)`: leading/trailing whitespace tolerated, an optional sign, digits.
///
/// Deliberately NOT `str::parse` alone — Python strips surrounding whitespace and this must too,
/// or `INTERVAL= 2` becomes an unsupported rule here and a working one there.
fn python_int(text: &str) -> Option<i64> {
    pystr::strip(text).parse::<i64>().ok()
}

fn weekday_index(date: Date) -> i64 {
    i64::from(date.weekday().to_monday_zero_offset())
}

fn add_days(dt: DateTime, days: i64) -> Option<DateTime> {
    dt.checked_add(Span::new().try_days(days).ok()?).ok()
}

/// Resolve an `UNTIL` value the way the Python does: a bare date becomes that day at 23:59:59.
fn parse_until(raw: &str, tz: &TimeZone) -> Option<DateTime> {
    match parse_dt(raw, &BTreeMap::new(), tz)? {
        Due::DateTime(dt) => Some(dt),
        Due::Date(d) => Some(d.to_datetime(Time::constant(23, 59, 59, 0))),
    }
}

/// A rule whose `UNTIL` predates the window can never produce busy time, so an unsupported one
/// is not worth a permanent WARN on every run.
fn rule_expired(rule: &BTreeMap<String, String>, tz: &TimeZone, window_start: DateTime) -> bool {
    let Some(raw) = rule.get("UNTIL") else {
        return false;
    };
    if raw.is_empty() {
        return false;
    }
    match parse_until(raw, tz) {
        Some(until) => until < window_start,
        None => false,
    }
}

/// Validate the rule eagerly, then produce the ascending occurrence list.
///
/// Python returns a lazy generator; this returns a `Vec` because `MAX_OCCURRENCES` bounds it at
/// 1000 either way and an eager bound is easier to prove terminating than a lazy one.
fn occurrence_starts(
    first: DateTime,
    rule: &BTreeMap<String, String>,
    tz: &TimeZone,
) -> Result<Vec<DateTime>, UnsupportedRule> {
    let freq = rule.get("FREQ").map(|f| f.to_uppercase()).unwrap_or_default();
    if !SUPPORTED_FREQ.contains(&freq.as_str()) {
        // `freq or "unknown"` — an absent or empty FREQ names itself as unknown.
        let named = if freq.is_empty() { "unknown".to_string() } else { freq };
        return Err(UnsupportedRule(named));
    }
    if freq == "DAILY" && rule.contains_key("BYDAY") {
        return Err(UnsupportedRule("DAILY+BYDAY".to_string()));
    }
    let interval_raw = rule.get("INTERVAL").filter(|v| !v.is_empty());
    let interval = match interval_raw {
        None => 1,
        Some(raw) => python_int(raw)
            .ok_or_else(|| UnsupportedRule(format!("INTERVAL={raw}")))?,
    };

    // `extra.discard("WKST")` runs ONLY when interval == 1. That looks like a mistake and is not:
    // WKST changes which day a multi-week interval counts from, so it is genuinely unsupported
    // there while being inert at interval 1. Preserved exactly.
    let mut extra: BTreeSet<&str> = rule
        .keys()
        .map(|k| k.as_str())
        .filter(|k| !RRULE_ALLOWED_KEYS.contains(k))
        .collect();
    if interval == 1 {
        extra.remove("WKST");
    }
    if !extra.is_empty() {
        let joined = extra.into_iter().collect::<Vec<_>>().join(",");
        return Err(UnsupportedRule(format!("unsupported: {joined}")));
    }

    let count = match rule.get("COUNT").filter(|v| !v.is_empty()) {
        None => None,
        Some(raw) => {
            Some(python_int(raw).ok_or_else(|| UnsupportedRule(format!("COUNT={raw}")))?)
        }
    };
    let until = match rule.get("UNTIL").filter(|v| !v.is_empty()) {
        None => None,
        Some(raw) => {
            Some(parse_until(raw, tz).ok_or_else(|| UnsupportedRule(format!("UNTIL={raw}")))?)
        }
    };

    // Build the candidate stream. WEEKLY anchors on the Monday of `first`'s week and steps whole
    // intervals of seven days from there, emitting each BYDAY offset in ascending order; getting
    // that anchor wrong shifts a recurring class by up to six days and still looks plausible.
    let mut candidates: Vec<DateTime> = Vec::new();
    if freq == "WEEKLY" {
        let byday = rule.get("BYDAY").cloned().unwrap_or_default();
        let mut offsets: Vec<i64> = Vec::new();
        if !byday.is_empty() {
            for code in byday.split(',') {
                match day_code(code) {
                    Some(offset) => offsets.push(offset),
                    None => return Err(UnsupportedRule(format!("BYDAY={code}"))),
                }
            }
        }
        offsets.sort_unstable();
        if offsets.is_empty() {
            offsets.push(weekday_index(first.date()));
        }
        let Some(anchor) = add_days(first, -weekday_index(first.date())) else {
            return Ok(Vec::new());
        };
        'weeks: for week in 0..MAX_OCCURRENCES as i64 {
            let Some(base) = add_days(anchor, 7 * interval * week) else {
                break 'weeks;
            };
            for offset in &offsets {
                match add_days(base, *offset) {
                    Some(c) => candidates.push(c),
                    None => break 'weeks,
                }
            }
        }
    } else {
        for i in 0..MAX_OCCURRENCES as i64 {
            match add_days(first, interval * i) {
                Some(c) => candidates.push(c),
                None => break,
            }
        }
    }

    let mut out = Vec::new();
    let mut made: i64 = 0;
    for candidate in candidates {
        if candidate < first {
            continue;
        }
        if let Some(u) = until {
            if candidate > u {
                break;
            }
        }
        made += 1;
        if let Some(c) = count {
            if made > c {
                break;
            }
        }
        out.push(candidate);
        if made >= MAX_OCCURRENCES as i64 {
            break;
        }
    }
    Ok(out)
}

/// Parse Google Calendar ICS text into busy events within the horizon.
pub fn parse_calendar_ics(
    text: &str,
    tz: &TimeZone,
    window_start: Date,
    horizon_days: i64,
) -> (Vec<CalEvent>, Vec<String>) {
    let ws = window_start.to_datetime(Time::midnight());
    let we = add_days(ws, horizon_days).unwrap_or(ws);
    // Keyed (start, end, title, all_day): dedups like Python's set, and orders by the three keys
    // Python actually sorts on. See deviation 1 in the module docs.
    let mut events: BTreeSet<(DateTime, DateTime, String, bool)> = BTreeSet::new();
    let mut warnings: Vec<String> = Vec::new();
    let mut block: Option<Vec<String>> = None;
    for line in unfold(text) {
        if line == "BEGIN:VEVENT" {
            block = Some(Vec::new());
        } else if line == "END:VEVENT" && block.is_some() {
            add_event(&block.take().unwrap(), tz, ws, we, &mut events, &mut warnings);
        } else if let Some(b) = block.as_mut() {
            b.push(line);
        }
    }
    let out = events
        .into_iter()
        .map(|(start, end, title, all_day)| CalEvent { title, start, end, all_day })
        .collect();
    (out, warnings)
}

fn add_event(
    block: &[String],
    tz: &TimeZone,
    ws: DateTime,
    we: DateTime,
    events: &mut BTreeSet<(DateTime, DateTime, String, bool)>,
    warnings: &mut Vec<String>,
) {
    let mut props: BTreeMap<String, Vec<(BTreeMap<String, String>, String)>> = BTreeMap::new();
    for line in block {
        if let Some((name, params, value)) = parse_property(line) {
            props.entry(name).or_default().push((params, value));
        }
    }
    let title = match props.get("SUMMARY").and_then(|v| v.first()) {
        Some((_, raw)) => {
            let cleaned = pystr::strip(&unescape(raw)).to_string();
            if cleaned.is_empty() { "(untitled)".to_string() } else { cleaned }
        }
        None => "(untitled)".to_string(),
    };
    let Some(dtstart) = props.get("DTSTART").and_then(|v| v.first()) else {
        warnings.push(format!("skipped (no DTSTART): {title}"));
        return;
    };

    // Python wraps this stretch in `except (ValueError, KeyError)`; parse_dt returns Option here,
    // so a None is the same event: "skipped (unparseable date)".
    let Some(start_raw) = parse_dt(&dtstart.1, &dtstart.0, tz) else {
        warnings.push(format!("skipped (unparseable date): {title}"));
        return;
    };
    let all_day = matches!(start_raw, Due::Date(_));
    let (start, end) = if all_day {
        let Due::Date(start_day) = start_raw else { unreachable!() };
        let start = start_day.to_datetime(Time::midnight());
        let end_day = match props.get("DTEND").and_then(|v| v.first()) {
            Some((params, value)) => match parse_dt(value, params, tz) {
                Some(Due::Date(d)) => d,
                Some(Due::DateTime(dt)) => dt.date(),
                None => {
                    warnings.push(format!("skipped (unparseable date): {title}"));
                    return;
                }
            },
            None => match start_day.checked_add(Span::new().days(1)) {
                Ok(d) => d,
                Err(_) => {
                    warnings.push(format!("skipped (unparseable date): {title}"));
                    return;
                }
            },
        };
        (start, end_day.to_datetime(Time::midnight()))
    } else {
        let Due::DateTime(start) = start_raw else { unreachable!() };
        let Some((params, value)) = props.get("DTEND").and_then(|v| v.first()) else {
            warnings.push(format!("skipped (no DTEND): {title}"));
            return;
        };
        match parse_dt(value, params, tz) {
            Some(Due::DateTime(end)) => (start, end),
            // A DATE where a timed event needs a DATETIME is its own message, not "unparseable".
            Some(Due::Date(_)) => {
                warnings.push(format!("skipped (malformed DTEND): {title}"));
                return;
            }
            None => {
                warnings.push(format!("skipped (unparseable date): {title}"));
                return;
            }
        }
    };

    if end <= start {
        warnings.push(format!("skipped (non-positive duration): {title}"));
        return;
    }
    let duration = start.until(end).unwrap_or_else(|_| Span::new());

    let mut exdatetimes: BTreeSet<DateTime> = BTreeSet::new();
    let mut exdays: BTreeSet<Date> = BTreeSet::new();
    for (params, value) in props.get("EXDATE").map(|v| v.as_slice()).unwrap_or(&[]) {
        for chunk in value.split(',') {
            match parse_dt(pystr::strip(chunk), params, tz) {
                Some(Due::DateTime(dt)) => {
                    exdatetimes.insert(dt);
                }
                Some(Due::Date(d)) => {
                    exdays.insert(d);
                }
                None => continue,
            }
        }
    }

    let starts: Vec<DateTime> = match props.get("RRULE").and_then(|v| v.first()) {
        Some((_, value)) => {
            let rule = rrule_dict(value);
            match occurrence_starts(start, &rule, tz) {
                Ok(s) => s,
                Err(why) => {
                    if !rule_expired(&rule, tz, ws) {
                        warnings.push(format!("skipped recurrence ({why}): {title}"));
                    }
                    return;
                }
            }
        }
        None => vec![start],
    };

    for occ_start in starts {
        if occ_start >= we {
            break;
        }
        if exdatetimes.contains(&occ_start) || exdays.contains(&occ_start.date()) {
            continue;
        }
        let Ok(occ_end) = occ_start.checked_add(duration) else {
            continue;
        };
        if occ_end > ws {
            events.insert((occ_start, occ_end, title.clone(), all_day));
        }
    }
}

fn span_of(event: &CalEvent) -> String {
    if event.all_day {
        format!(
            "{}–{} (all day)",
            event.start.strftime("%Y-%m-%d"),
            event.end.strftime("%Y-%m-%d")
        )
    } else if event.start.date() == event.end.date() {
        format!(
            "{}–{}",
            event.start.strftime("%Y-%m-%d %H:%M"),
            event.end.strftime("%H:%M")
        )
    } else {
        format!(
            "{}–{}",
            event.start.strftime("%Y-%m-%d %H:%M"),
            event.end.strftime("%Y-%m-%d %H:%M")
        )
    }
}

pub fn write_snapshot(
    path: &Path,
    by_feed: &BTreeMap<String, Vec<CalEvent>>,
) -> std::io::Result<()> {
    let mut lines = vec![SNAPSHOT_HEADER.to_string()];
    for (feed, items) in by_feed {
        let ordered: BTreeSet<(DateTime, DateTime, String, bool)> = items
            .iter()
            .map(|e| (e.start, e.end, e.title.clone(), e.all_day))
            .collect();
        for (start, end, title, all_day) in ordered {
            let event = CalEvent { title: title.clone(), start, end, all_day };
            lines.push(format!("- {feed} · {} · {title}", span_of(&event)));
        }
    }
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)?;
    }
    pystr::write_text(path, &(lines.join("\n") + "\n"))
}

fn parse_hm(day: &str, hm: &str) -> Option<DateTime> {
    DateTime::strptime("%Y-%m-%dT%H:%M", format!("{day}T{hm}")).ok()
}

fn event_from_span(span: &str, title: &str) -> Option<CalEvent> {
    if let Some(m) = ALL_DAY.captures(span) {
        let start: Date = m.name("d1")?.as_str().parse().ok()?;
        let end: Date = m.name("d2")?.as_str().parse().ok()?;
        return Some(CalEvent {
            title: title.to_string(),
            start: start.to_datetime(Time::midnight()),
            end: end.to_datetime(Time::midnight()),
            all_day: true,
        });
    }
    if let Some(m) = SAME_DAY.captures(span) {
        let day = m.name("d")?.as_str();
        return Some(CalEvent {
            title: title.to_string(),
            start: parse_hm(day, m.name("s")?.as_str())?,
            end: parse_hm(day, m.name("e")?.as_str())?,
            all_day: false,
        });
    }
    if let Some(m) = CROSS_DAY.captures(span) {
        return Some(CalEvent {
            title: title.to_string(),
            start: parse_hm(m.name("d1")?.as_str(), m.name("s")?.as_str())?,
            end: parse_hm(m.name("d2")?.as_str(), m.name("e")?.as_str())?,
            all_day: false,
        });
    }
    None
}

pub fn read_snapshot(path: &Path) -> BTreeMap<String, Vec<CalEvent>> {
    if !path.exists() {
        return BTreeMap::new();
    }
    let Ok(text) = pystr::read_text(path) else {
        return BTreeMap::new();
    };
    let mut by_feed: BTreeMap<String, BTreeSet<(DateTime, DateTime, String, bool)>> =
        BTreeMap::new();
    for line in pystr::splitlines(&text) {
        let Some(m) = LINE.captures(line) else { continue };
        let span = pystr::strip(m.name("span").map(|x| x.as_str()).unwrap_or(""));
        let title = pystr::strip(m.name("title").map(|x| x.as_str()).unwrap_or(""));
        if let Some(event) = event_from_span(span, title) {
            let feed = pystr::strip(m.name("feed").map(|x| x.as_str()).unwrap_or("")).to_string();
            by_feed
                .entry(feed)
                .or_default()
                .insert((event.start, event.end, event.title, event.all_day));
        }
    }
    by_feed
        .into_iter()
        .map(|(feed, items)| {
            let events = items
                .into_iter()
                .map(|(start, end, title, all_day)| CalEvent { title, start, end, all_day })
                .collect();
            (feed, events)
        })
        .collect()
}

/// Fetch a calendar feed. Decoding is lossy, matching Python's `errors="replace"`.
pub fn fetch_ics(url: &str) -> Result<String, String> {
    let agent: ureq::Agent = ureq::Agent::config_builder()
        .timeout_global(Some(std::time::Duration::from_secs(60)))
        .build()
        .into();
    let mut response = agent.get(url).call().map_err(|e| e.to_string())?;
    let bytes = response
        .body_mut()
        .with_config()
        .limit(u64::MAX)
        .read_to_vec()
        .map_err(|e| e.to_string())?;
    Ok(String::from_utf8_lossy(&bytes).into_owned())
}

/// Every configured calendar feed, with a per-feed snapshot fallback on failure.
///
/// `fetcher` exists so tests never touch the network. Python achieves the same by monkeypatching
/// the module global; Rust needs the seam to be explicit.
/// Python: `feeds = config.get("calendars") or []`, then `for feed in feeds`. So a falsy value
/// (absent, null, empty, false, 0) is silence; a sequence is the feed list; and anything else is
/// iterated as whatever it is — a STRING one character at a time, a mapping one key at a time —
/// each element then failing `.get` inside the per-feed try and warning `bad feed entry`.
/// Preserved defect 20 (`calendars: "https://…"` is one warning per character). A scalar that is
/// not iterable at all (`calendars: 5`) raises TypeError OUTSIDE every try in Python: the run
/// FAILs and no page is written; here that is `Err` with the same message, and the caller's early
/// return leaves the previous snapshot untouched — the same class of divergence as
/// `load_runners_config`, taken because a config typo should name itself in the log, not take the
/// morning's ranking with it.
///
/// The one place `calendars:`'s own value-shape coercion happens — [`load_calendar_events`] and
/// [`calendar_entries`] both call this rather than each parsing the key on their own.
///
/// `Ok(None)` for a falsy or absent value (Python's `or []` never even enters the per-feed loop,
/// so the caller must take the SAME early return it always has — no per-feed warning, and no
/// snapshot write). `Ok(Some(feeds))` for anything with entries. `Err` only for a scalar that is
/// not iterable at all.
fn calendars_feeds(config: &serde_yaml_ng::Value) -> Result<Option<Vec<serde_yaml_ng::Value>>, String> {
    use serde_yaml_ng::Value;
    match config.get("calendars") {
        Some(value) if !pystr::yaml_truthy(value) => Ok(None),
        None => Ok(None),
        Some(Value::Sequence(items)) => Ok(Some(items.clone())),
        Some(Value::String(s)) => Ok(Some(s.chars().map(|c| Value::String(c.to_string())).collect())),
        Some(Value::Mapping(m)) => Ok(Some(m.keys().cloned().collect())),
        Some(other) => Err(format!(
            "calendar: bad calendars value ('{}' object is not iterable)",
            pystr::yaml_type_name(other)
        )),
    }
}

/// This vault's `config/ingest.yaml` `calendars:` entries, as `(name, ics_url)` pairs — a pure
/// config read with no fetch, no snapshot write and no warning: a non-mapping entry (the "bad feed
/// entry" case [`load_calendar_events`] warns about) simply contributes nothing here, since it has
/// no name or url to report. R-C2-E43: this is what [`crate::enrich`]'s Google-calendar predicate
/// reads, through the one parser both it and `load_calendar_events` share ([`calendars_feeds`]),
/// so checking whether a vault has ever connected Google costs a config read and nothing else.
pub fn calendar_entries(vault: &Path) -> Vec<(String, String)> {
    use serde_yaml_ng::Value;
    let config_path = vault.join("config").join("ingest.yaml");
    let Ok(raw) = pystr::read_text(&config_path) else { return Vec::new() };
    let Ok(config) = serde_yaml_ng::from_str::<Value>(&raw) else { return Vec::new() };
    let feeds = calendars_feeds(&config).ok().flatten().unwrap_or_default();
    feeds
        .iter()
        .filter_map(|feed| {
            let mapping = feed.as_mapping()?;
            let name = mapping
                .get(Value::from("name"))
                .and_then(|v| v.as_str())
                .filter(|s| !s.is_empty())
                .unwrap_or("calendar")
                .to_string();
            let url = mapping.get(Value::from("ics_url")).and_then(|v| v.as_str()).unwrap_or("");
            Some((name, pystr::strip(url).to_string()))
        })
        .collect()
}

pub fn load_calendar_events(
    vault: &Path,
    today: Date,
    fetcher: Option<&dyn Fn(&str) -> Result<String, String>>,
) -> (Vec<CalEvent>, Vec<String>) {
    let default_fetch = |url: &str| fetch_ics(url);
    let fetch: &dyn Fn(&str) -> Result<String, String> = fetcher.unwrap_or(&default_fetch);

    let config_path = vault.join("config").join("ingest.yaml");
    if !config_path.exists() {
        return (Vec::new(), Vec::new());
    }
    let raw = match pystr::read_text(&config_path) {
        Ok(text) => text,
        Err(err) => return (Vec::new(), vec![format!("config unreadable: {err}")]),
    };
    let config: serde_yaml_ng::Value = match serde_yaml_ng::from_str(&raw) {
        Ok(value) => value,
        Err(err) => return (Vec::new(), vec![format!("config unreadable: {err}")]),
    };
    use serde_yaml_ng::Value;
    let feeds: Vec<Value> = match calendars_feeds(&config) {
        Ok(Some(feeds)) => feeds,
        Ok(None) => return (Vec::new(), Vec::new()),
        Err(warning) => return (Vec::new(), vec![warning]),
    };

    let mut warnings: Vec<String> = Vec::new();
    let tz_name = config
        .get("timezone")
        .and_then(|v| v.as_str())
        .unwrap_or("America/Chicago")
        .to_string();
    let tz = match TimeZone::get(&tz_name) {
        Ok(tz) => tz,
        Err(err) => {
            warnings.push(format!("config: bad timezone ({err}); using America/Chicago"));
            TimeZone::get("America/Chicago").unwrap_or(TimeZone::UTC)
        }
    };

    let snapshot_path = vault.join("state").join("calendar.md");
    let previous = read_snapshot(&snapshot_path);
    let mut fresh: BTreeMap<String, Vec<CalEvent>> = BTreeMap::new();

    for feed in &feeds {
        // Python sets `name = "calendar"` BEFORE the inner try, so a feed entry that is not a
        // mapping is reported against that placeholder rather than crashing the run.
        let name = "calendar".to_string();
        let Some(mapping) = feed.as_mapping() else {
            warnings.push(format!(
                "{name}: bad feed entry ('{}' object has no attribute 'get')",
                pystr::yaml_type_name(feed)
            ));
            fresh.insert(name.clone(), previous.get(&name).cloned().unwrap_or_default());
            continue;
        };
        let name = mapping
            .get(serde_yaml_ng::Value::from("name"))
            .and_then(|v| v.as_str())
            .filter(|s| !s.is_empty())
            .unwrap_or("calendar")
            .to_string();
        let url = mapping
            .get(serde_yaml_ng::Value::from("ics_url"))
            .and_then(|v| v.as_str())
            .unwrap_or("");
        let url = pystr::strip(url);
        if url.is_empty() {
            warnings.push(format!("{name}: no ics_url"));
            fresh.insert(name.clone(), previous.get(&name).cloned().unwrap_or_default());
            continue;
        }
        // A-6: `url` can be the literal `cloud:<name>` marker (C2 Task 8, §11a) — this module
        // never parses that prefix itself; `cli.rs`'s own `calendar` closure is what strips it and
        // routes to `cloudmodel::fetch_calendar`, so `fetch` here is opaque either way.
        let text = match fetch(url) {
            Ok(text) => text,
            Err(err) => {
                warnings.push(format!("{name}: fetch failed ({err}); using snapshot"));
                fresh.insert(name.clone(), previous.get(&name).cloned().unwrap_or_default());
                continue;
            }
        };
        if !text.contains("BEGIN:VCALENDAR") {
            warnings.push(format!(
                "{name}: fetch failed (not an ICS response); using snapshot"
            ));
            fresh.insert(name.clone(), previous.get(&name).cloned().unwrap_or_default());
            continue;
        }
        let (parsed, feed_warnings) = parse_calendar_ics(&text, &tz, today, HORIZON_DAYS);
        for w in feed_warnings {
            warnings.push(format!("{name}: {w}"));
        }
        fresh.insert(name, parsed);
    }

    write_snapshot(&snapshot_path, &fresh).ok();

    let window_start = today.to_datetime(Time::midnight());
    let window_end = add_days(window_start, HORIZON_DAYS).unwrap_or(window_start);
    let merged: BTreeSet<(DateTime, DateTime, String, bool)> = fresh
        .values()
        .flatten()
        .filter(|e| e.start < window_end && e.end > window_start)
        .map(|e| (e.start, e.end, e.title.clone(), e.all_day))
        .collect();
    let out = merged
        .into_iter()
        .map(|(start, end, title, all_day)| CalEvent { title, start, end, all_day })
        .collect();
    (out, warnings)
}

#[cfg(test)]
mod tests {
    //! Direct port of `tests/test_calfeed.py` — all 33 tests, same names, same assertions.
    //!
    //! The Python file is the specification for this module: every warning string, every
    //! recurrence edge, and the never-crash contract are pinned here rather than described.

    use super::*;
    use jiff::civil::date;
    use std::fs;
    use std::path::PathBuf;

    /// `TZ = ZoneInfo("America/Chicago")`
    fn tz() -> TimeZone {
        TimeZone::get("America/Chicago").unwrap()
    }

    /// `WINDOW = date(2026, 9, 7)`
    const WINDOW: Date = Date::constant(2026, 9, 7);

    fn wrap(vevents: &[String]) -> String {
        format!("BEGIN:VCALENDAR\n{}END:VCALENDAR\n", vevents.concat())
    }

    fn vevent(lines: &[&str]) -> String {
        let body: String = lines.iter().map(|l| format!("{l}\n")).collect();
        format!("BEGIN:VEVENT\n{body}END:VEVENT\n")
    }

    fn parse(text: &str) -> (Vec<CalEvent>, Vec<String>) {
        parse_calendar_ics(text, &tz(), WINDOW, HORIZON_DAYS)
    }

    /// pytest's `tmp_path`, one directory per test so they can run in parallel.
    fn tmp_vault(name: &str) -> PathBuf {
        let dir = std::env::temp_dir().join(format!("qo-calfeed-{}-{}", name, std::process::id()));
        let _ = fs::remove_dir_all(&dir);
        fs::create_dir_all(&dir).unwrap();
        dir
    }

    fn write(path: &Path, text: &str) {
        fs::create_dir_all(path.parent().unwrap()).unwrap();
        pystr::write_text(path, text).unwrap();
    }

    fn titles(events: &[CalEvent]) -> Vec<&str> {
        events.iter().map(|e| e.title.as_str()).collect()
    }

    // --- parsing ------------------------------------------------------------------------

    #[test]
    fn timed_event_utc_converts_to_chicago() {
        let text = wrap(&[vevent(&[
            "UID:a",
            "SUMMARY:Client call",
            "DTSTART:20260914T140000Z",
            "DTEND:20260914T143000Z",
        ])]);
        let (events, warnings) = parse(&text);
        assert!(warnings.is_empty());
        assert_eq!(events.len(), 1);
        assert_eq!(events[0].title, "Client call");
        // 14:00Z -> 09:00 CDT
        assert_eq!(events[0].start, date(2026, 9, 14).at(9, 0, 0, 0));
        assert_eq!(events[0].end, date(2026, 9, 14).at(9, 30, 0, 0));
        assert!(!events[0].all_day);
    }

    #[test]
    fn all_day_event_spans_dates_exclusive_end() {
        let text = wrap(&[vevent(&[
            "UID:b",
            "SUMMARY:Career fair",
            "DTSTART;VALUE=DATE:20260915",
            "DTEND;VALUE=DATE:20260917",
        ])]);
        let (events, _) = parse(&text);
        assert!(events[0].all_day);
        assert_eq!(events[0].start, date(2026, 9, 15).at(0, 0, 0, 0));
        assert_eq!(events[0].end, date(2026, 9, 17).at(0, 0, 0, 0));
    }

    #[test]
    fn event_outside_horizon_is_dropped() {
        let text = wrap(&[vevent(&[
            "UID:c",
            "SUMMARY:Far future",
            "DTSTART:20261101T150000Z",
            "DTEND:20261101T160000Z",
        ])]);
        let (events, warnings) = parse(&text);
        assert!(events.is_empty() && warnings.is_empty());
    }

    #[test]
    fn event_straddling_window_start_is_kept() {
        let text = wrap(&[vevent(&[
            "UID:d",
            "SUMMARY:Overnight",
            "DTSTART;TZID=America/Chicago:20260906T230000",
            "DTEND;TZID=America/Chicago:20260907T010000",
        ])]);
        let (events, _) = parse(&text);
        assert_eq!(events.len(), 1);
    }

    #[test]
    fn timed_event_without_dtend_warns_and_skips() {
        let text = wrap(&[vevent(&["UID:e", "SUMMARY:Broken", "DTSTART:20260914T140000Z"])]);
        let (events, warnings) = parse(&text);
        assert!(events.is_empty());
        assert_eq!(warnings, vec!["skipped (no DTEND): Broken"]);
    }

    #[test]
    fn fixture_expands_weekly_rrule_and_warns_on_monthly() {
        let text = pystr::read_text(Path::new("tests/fixtures/gcal.ics")).unwrap();
        let (events, warnings) = parse(&text);
        // Club meeting: TU/TH 17:00 from 9/8 until 10/1 00:00Z (= 9/30 19:00 CDT),
        // minus the 9/17 EXDATE -> 9/8, 9/10, 9/15, 9/22, 9/24, 9/29.
        let club: Vec<&CalEvent> = events.iter().filter(|e| e.title == "Club meeting").collect();
        let days: Vec<String> = club.iter().map(|e| e.start.date().to_string()).collect();
        assert_eq!(
            days,
            vec![
                "2026-09-08",
                "2026-09-10",
                "2026-09-15",
                "2026-09-22",
                "2026-09-24",
                "2026-09-29"
            ]
        );
        assert!(club.iter().all(|e| e.start.hour() == 17 && e.end.hour() == 18));
        // 6 club + Client call + Career fair; Far future out of window.
        assert_eq!(events.len(), 8);
        assert_eq!(warnings, vec!["skipped recurrence (MONTHLY): Advising lunch"]);
    }

    #[test]
    fn daily_rrule_with_count() {
        let text = wrap(&[vevent(&[
            "UID:d",
            "SUMMARY:Standup",
            "DTSTART;TZID=America/Chicago:20260908T080000",
            "DTEND;TZID=America/Chicago:20260908T081500",
            "RRULE:FREQ=DAILY;COUNT=3",
        ])]);
        let (events, warnings) = parse(&text);
        assert!(warnings.is_empty());
        let days: Vec<i8> = events.iter().map(|e| e.start.day()).collect();
        assert_eq!(days, vec![8, 9, 10]);
    }

    #[test]
    fn weekly_interval_two_skips_alternate_weeks() {
        let text = wrap(&[vevent(&[
            "UID:i",
            "SUMMARY:Biweekly",
            "DTSTART;TZID=America/Chicago:20260908T100000",
            "DTEND;TZID=America/Chicago:20260908T110000",
            "RRULE:FREQ=WEEKLY;INTERVAL=2",
        ])]);
        let (events, _) = parse(&text);
        let days: Vec<String> = events.iter().map(|e| e.start.date().to_string()).collect();
        // 10/6 falls outside the 28-day window ending 10/5.
        assert_eq!(days, vec!["2026-09-08", "2026-09-22"]);
    }

    #[test]
    fn ordinal_byday_is_unsupported() {
        let text = wrap(&[vevent(&[
            "UID:o",
            "SUMMARY:Second Monday",
            "DTSTART;TZID=America/Chicago:20260914T100000",
            "DTEND;TZID=America/Chicago:20260914T110000",
            "RRULE:FREQ=WEEKLY;BYDAY=2MO",
        ])]);
        let (events, warnings) = parse(&text);
        assert!(events.is_empty());
        assert_eq!(warnings, vec!["skipped recurrence (BYDAY=2MO): Second Monday"]);
    }

    // --- snapshot -----------------------------------------------------------------------

    #[test]
    fn snapshot_round_trips_all_shapes() {
        let events = vec![
            CalEvent {
                title: "Client call".into(),
                start: date(2026, 9, 7).at(9, 0, 0, 0),
                end: date(2026, 9, 7).at(9, 30, 0, 0),
                all_day: false,
            },
            CalEvent {
                title: "Career fair".into(),
                start: date(2026, 9, 15).at(0, 0, 0, 0),
                end: date(2026, 9, 17).at(0, 0, 0, 0),
                all_day: true,
            },
            CalEvent {
                title: "Overnight".into(),
                start: date(2026, 9, 8).at(22, 0, 0, 0),
                end: date(2026, 9, 9).at(1, 0, 0, 0),
                all_day: false,
            },
        ];
        let dir = tmp_vault("roundtrip");
        let path = dir.join("calendar.md");
        let mut by_feed = BTreeMap::new();
        by_feed.insert("personal".to_string(), events.clone());
        write_snapshot(&path, &by_feed).unwrap();

        let mut expected = events.clone();
        expected.sort_by(|a, b| (a.start, a.end, &a.title).cmp(&(b.start, b.end, &b.title)));
        let restored = read_snapshot(&path);
        assert_eq!(restored.len(), 1);
        assert_eq!(restored["personal"], expected);
    }

    #[test]
    fn snapshot_dedups_union_merge_duplicates() {
        let dir = tmp_vault("dedup");
        let path = dir.join("calendar.md");
        let line = "- personal · 2026-09-07 09:00–09:30 · Client call\n";
        write(&path, &format!("# header\n{line}{line}"));
        assert_eq!(read_snapshot(&path)["personal"].len(), 1);
    }

    #[test]
    fn snapshot_missing_file_is_empty() {
        let dir = tmp_vault("missing");
        assert!(read_snapshot(&dir.join("calendar.md")).is_empty());
    }

    #[test]
    fn snapshot_silently_skips_shape_valid_but_date_invalid_lines() {
        let dir = tmp_vault("baddate");
        let path = dir.join("calendar.md");
        let valid = "- personal · 2026-09-07 09:00–09:30 · Client call\n";
        let invalid = "- personal · 2026-13-99 99:99–99:99 · Bad Event\n";
        write(&path, &format!("# header\n{valid}{invalid}"));
        let restored = read_snapshot(&path);
        assert_eq!(restored["personal"].len(), 1);
        assert_eq!(restored["personal"][0].title, "Client call");
    }

    // --- load_calendar_events -----------------------------------------------------------

    #[test]
    fn load_events_missing_config_is_silent() {
        let dir = tmp_vault("noconfig");
        let (events, warnings) = load_calendar_events(&dir, date(2026, 9, 7), None);
        assert!(events.is_empty() && warnings.is_empty());
    }

    /// R-C2-E43: `calendar_entries` is a pure config read — no fetch, no snapshot write, and it
    /// never even needs a `fetch` seam to prove that with.
    #[test]
    fn calendar_entries_reads_the_calendars_list_with_no_fetch_and_no_write() {
        let dir = tmp_vault("entries");
        assert_eq!(calendar_entries(&dir), Vec::<(String, String)>::new(), "no config file at all");

        write(&dir.join("config").join("ingest.yaml"), "timezone: America/Chicago\ncourse_map: {}\n");
        assert_eq!(calendar_entries(&dir), Vec::<(String, String)>::new(), "no calendars: key");

        write(
            &dir.join("config").join("ingest.yaml"),
            "timezone: America/Chicago\ncalendars:\n  - name: personal\n    ics_url: https://a.test/a.ics\n",
        );
        assert_eq!(
            calendar_entries(&dir),
            vec![("personal".to_string(), "https://a.test/a.ics".to_string())],
        );

        write(
            &dir.join("config").join("ingest.yaml"),
            "timezone: America/Chicago\ncalendars:\n  - name: personal\n    ics_url: https://a.test/a.ics\n  - name: google\n    ics_url: 'cloud:google'\n",
        );
        assert_eq!(
            calendar_entries(&dir),
            vec![
                ("personal".to_string(), "https://a.test/a.ics".to_string()),
                ("google".to_string(), "cloud:google".to_string()),
            ],
        );
        // No fetch was ever possible to make (this function takes no fetcher at all) and no
        // snapshot was written: the state directory this vault never had stays absent.
        assert!(!dir.join("state").join("calendar.md").exists());
    }

    fn one_feed_config(dir: &Path) {
        write(
            &dir.join("config").join("ingest.yaml"),
            "timezone: America/Chicago\ncalendars:\n  - name: personal\n    ics_url: https://a.test/a.ics\n",
        );
    }

    fn fresh_ics() -> String {
        wrap(&[vevent(&[
            "UID:x",
            "SUMMARY:Fresh",
            "DTSTART;TZID=America/Chicago:20260909T090000",
            "DTEND;TZID=America/Chicago:20260909T100000",
        ])])
    }

    #[test]
    fn load_events_fetches_and_writes_snapshot() {
        let dir = tmp_vault("fetches");
        one_feed_config(&dir);
        let ics = fresh_ics();
        let fetch = |_: &str| Ok(ics.clone());
        let (events, warnings) = load_calendar_events(&dir, date(2026, 9, 7), Some(&fetch));
        assert!(warnings.is_empty());
        assert_eq!(titles(&events), vec!["Fresh"]);
        let snapshot = pystr::read_text(&dir.join("state").join("calendar.md")).unwrap();
        assert!(snapshot.contains("- personal · 2026-09-09 09:00–10:00 · Fresh"));
    }

    #[test]
    fn one_feed_failing_falls_back_and_others_proceed() {
        let dir = tmp_vault("onefail");
        write(
            &dir.join("config").join("ingest.yaml"),
            "timezone: America/Chicago\ncalendars:\n  - name: personal\n    ics_url: https://a.test/a.ics\n  - name: clients\n    ics_url: https://b.test/b.ics\n",
        );
        write(
            &dir.join("state").join("calendar.md"),
            "# snap\n- clients · 2026-09-08 10:00–11:00 · Old booking\n",
        );
        let ok = fresh_ics();
        let fetch = |url: &str| {
            if url.contains("a.test") {
                Ok(ok.clone())
            } else {
                Err("403 Forbidden".to_string())
            }
        };
        let (events, warnings) = load_calendar_events(&dir, date(2026, 9, 7), Some(&fetch));
        assert_eq!(titles(&events), vec!["Old booking", "Fresh"]);
        assert_eq!(
            warnings,
            vec!["clients: fetch failed (403 Forbidden); using snapshot"]
        );
        // The fallback must survive the snapshot rewrite, or the next run loses it too.
        let snapshot = pystr::read_text(&dir.join("state").join("calendar.md")).unwrap();
        assert!(snapshot.contains("Old booking") && snapshot.contains("Fresh"));
    }

    #[test]
    fn load_events_clips_stale_snapshot_to_window() {
        let dir = tmp_vault("clips");
        one_feed_config(&dir);
        write(
            &dir.join("state").join("calendar.md"),
            "# snap\n- personal · 2026-08-01 10:00–11:00 · Ancient\n",
        );
        let fetch = |_: &str| Err("down".to_string());
        let (events, warnings) = load_calendar_events(&dir, date(2026, 9, 7), Some(&fetch));
        assert!(events.is_empty(), "the stale event predates the window");
        assert_eq!(warnings.len(), 1);
    }

    #[test]
    fn load_events_config_unreadable_returns_empty_with_warning() {
        let dir = tmp_vault("badconfig");
        write(&dir.join("config").join("ingest.yaml"), "calendars: [unclosed\n");
        let (events, warnings) = load_calendar_events(&dir, date(2026, 9, 7), None);
        assert!(events.is_empty());
        assert_eq!(warnings.len(), 1);
        assert!(warnings[0].starts_with("config unreadable:"));
    }

    // --- never-crash contract holes -----------------------------------------------------

    #[test]
    fn fetch_incomplete_read_falls_back_with_warning() {
        let dir = tmp_vault("incomplete");
        one_feed_config(&dir);
        write(
            &dir.join("state").join("calendar.md"),
            "# snap\n- personal · 2026-09-08 10:00–11:00 · Old booking\n",
        );
        let fetch = |_: &str| Err("IncompleteRead(0 bytes read)".to_string());
        let (events, warnings) = load_calendar_events(&dir, date(2026, 9, 7), Some(&fetch));
        assert_eq!(titles(&events), vec!["Old booking"]);
        assert_eq!(warnings.len(), 1);
        assert!(warnings[0].starts_with("personal: fetch failed ("));
        assert!(warnings[0].ends_with("); using snapshot"));
        let snapshot = pystr::read_text(&dir.join("state").join("calendar.md")).unwrap();
        assert!(snapshot.contains("Old booking"));
    }

    #[test]
    fn fetch_value_error_from_bad_scheme_falls_back_with_warning() {
        let dir = tmp_vault("badscheme");
        write(
            &dir.join("config").join("ingest.yaml"),
            "timezone: America/Chicago\ncalendars:\n  - name: personal\n    ics_url: htpt://a.test/a.ics\n",
        );
        let fetch = |_: &str| Err("unknown url type".to_string());
        let (events, warnings) = load_calendar_events(&dir, date(2026, 9, 7), Some(&fetch));
        assert!(events.is_empty());
        assert_eq!(warnings.len(), 1);
        assert!(warnings[0].starts_with("personal: fetch failed ("));
        assert!(warnings[0].ends_with("); using snapshot"));
    }

    #[test]
    fn malformed_feed_entry_string_warns_and_others_proceed() {
        let dir = tmp_vault("badentry");
        write(
            &dir.join("config").join("ingest.yaml"),
            "timezone: America/Chicago\ncalendars:\n  - not-a-mapping\n  - name: personal\n    ics_url: https://a.test/a.ics\n",
        );
        let ics = fresh_ics();
        let fetch = |_: &str| Ok(ics.clone());
        let (events, warnings) = load_calendar_events(&dir, date(2026, 9, 7), Some(&fetch));
        assert_eq!(titles(&events), vec!["Fresh"]);
        assert_eq!(warnings.len(), 1);
        assert!(warnings[0].starts_with("calendar: bad feed entry ("));
    }

    #[test]
    fn bad_timezone_falls_back_to_chicago() {
        let dir = tmp_vault("badtz");
        write(
            &dir.join("config").join("ingest.yaml"),
            "timezone: Not/AZone\ncalendars:\n  - name: personal\n    ics_url: https://a.test/a.ics\n",
        );
        let ics = wrap(&[vevent(&[
            "UID:x",
            "SUMMARY:Fresh",
            "DTSTART:20260909T140000Z",
            "DTEND:20260909T150000Z",
        ])]);
        let fetch = |_: &str| Ok(ics.clone());
        let (events, warnings) = load_calendar_events(&dir, date(2026, 9, 7), Some(&fetch));
        // 14:00Z -> 09:00 CDT via the Chicago fallback.
        assert_eq!(events[0].start, date(2026, 9, 9).at(9, 0, 0, 0));
        assert_eq!(warnings.len(), 1);
        assert!(warnings[0].starts_with("config: bad timezone ("));
        assert!(warnings[0].ends_with("); using America/Chicago"));
    }

    #[test]
    fn interval_non_numeric_skips_recurrence_no_crash() {
        let text = wrap(&[vevent(&[
            "UID:n",
            "SUMMARY:Weird interval",
            "DTSTART;TZID=America/Chicago:20260908T100000",
            "DTEND;TZID=America/Chicago:20260908T110000",
            "RRULE:FREQ=DAILY;INTERVAL=abc",
        ])]);
        let (events, warnings) = parse(&text);
        assert!(events.is_empty());
        assert_eq!(warnings.len(), 1);
        assert!(warnings[0].starts_with("skipped recurrence ("));
        assert!(warnings[0].contains("Weird interval"));
    }

    #[test]
    fn count_non_numeric_skips_recurrence_no_crash() {
        let text = wrap(&[vevent(&[
            "UID:n2",
            "SUMMARY:Weird count",
            "DTSTART;TZID=America/Chicago:20260908T100000",
            "DTEND;TZID=America/Chicago:20260908T110000",
            "RRULE:FREQ=DAILY;COUNT=abc",
        ])]);
        let (events, warnings) = parse(&text);
        assert!(events.is_empty());
        assert_eq!(warnings.len(), 1);
        assert!(warnings[0].starts_with("skipped recurrence ("));
    }

    #[test]
    fn until_unparseable_skips_recurrence_no_crash() {
        let text = wrap(&[vevent(&[
            "UID:n3",
            "SUMMARY:Weird until",
            "DTSTART;TZID=America/Chicago:20260908T100000",
            "DTEND;TZID=America/Chicago:20260908T110000",
            "RRULE:FREQ=DAILY;UNTIL=notadate",
        ])]);
        let (events, warnings) = parse(&text);
        assert!(events.is_empty());
        assert_eq!(warnings.len(), 1);
        assert!(warnings[0].starts_with("skipped recurrence ("));
    }

    #[test]
    fn non_ics_response_falls_back_to_snapshot() {
        let dir = tmp_vault("nonics");
        one_feed_config(&dir);
        write(
            &dir.join("state").join("calendar.md"),
            "# snap\n- personal · 2026-09-08 10:00–11:00 · Old booking\n",
        );
        let fetch = |_: &str| Ok("<html>login</html>".to_string());
        let (events, warnings) = load_calendar_events(&dir, date(2026, 9, 7), Some(&fetch));
        assert_eq!(titles(&events), vec!["Old booking"]);
        assert_eq!(
            warnings,
            vec!["personal: fetch failed (not an ICS response); using snapshot"]
        );
        let snapshot = pystr::read_text(&dir.join("state").join("calendar.md")).unwrap();
        assert!(snapshot.contains("Old booking"));
    }

    // --- RRULE semantic drift must be visible, not silent -------------------------------

    #[test]
    fn daily_with_byday_is_unsupported() {
        let text = wrap(&[vevent(&[
            "UID:w",
            "SUMMARY:Weekday standup",
            "DTSTART;TZID=America/Chicago:20260908T090000",
            "DTEND;TZID=America/Chicago:20260908T093000",
            "RRULE:FREQ=DAILY;BYDAY=MO,TU,WE,TH,FR",
        ])]);
        let (events, warnings) = parse(&text);
        assert!(events.is_empty());
        assert_eq!(
            warnings,
            vec!["skipped recurrence (DAILY+BYDAY): Weekday standup"]
        );
    }

    #[test]
    fn bysetpos_is_unsupported() {
        let text = wrap(&[vevent(&[
            "UID:z",
            "SUMMARY:Last Friday",
            "DTSTART;TZID=America/Chicago:20260911T090000",
            "DTEND;TZID=America/Chicago:20260911T100000",
            "RRULE:FREQ=WEEKLY;BYSETPOS=2",
        ])]);
        let (events, warnings) = parse(&text);
        assert!(events.is_empty());
        assert_eq!(
            warnings,
            vec!["skipped recurrence (unsupported: BYSETPOS): Last Friday"]
        );
    }

    #[test]
    fn bymonthday_is_unsupported() {
        let text = wrap(&[vevent(&[
            "UID:y",
            "SUMMARY:Monthly billing",
            "DTSTART;TZID=America/Chicago:20260908T090000",
            "DTEND;TZID=America/Chicago:20260908T100000",
            "RRULE:FREQ=WEEKLY;BYMONTHDAY=8",
        ])]);
        let (events, warnings) = parse(&text);
        assert!(events.is_empty());
        assert_eq!(
            warnings,
            vec!["skipped recurrence (unsupported: BYMONTHDAY): Monthly billing"]
        );
    }

    #[test]
    fn wkst_tolerated_without_interval() {
        let text = wrap(&[vevent(&[
            "UID:v",
            "SUMMARY:Tuesday sync",
            "DTSTART;TZID=America/Chicago:20260908T090000",
            "DTEND;TZID=America/Chicago:20260908T100000",
            "RRULE:FREQ=WEEKLY;WKST=SU;BYDAY=TU",
        ])]);
        let (events, warnings) = parse(&text);
        assert!(warnings.is_empty());
        assert!(!events.is_empty());
    }

    #[test]
    fn wkst_not_tolerated_with_interval_above_one() {
        let text = wrap(&[vevent(&[
            "UID:u",
            "SUMMARY:Biweekly with wkst",
            "DTSTART;TZID=America/Chicago:20260908T090000",
            "DTEND;TZID=America/Chicago:20260908T100000",
            "RRULE:FREQ=WEEKLY;INTERVAL=2;WKST=SU;BYDAY=TU",
        ])]);
        let (events, warnings) = parse(&text);
        assert!(events.is_empty());
        assert_eq!(
            warnings,
            vec!["skipped recurrence (unsupported: WKST): Biweekly with wkst"]
        );
    }

    #[test]
    fn expired_unsupported_recurrence_is_silently_skipped() {
        // A biweekly WKST rule whose UNTIL passed long before the window can never produce busy
        // time — it must not generate a permanent WARN either.
        let text = wrap(&[vevent(&[
            "UID:dead",
            "SUMMARY:Old biweekly",
            "DTSTART;TZID=America/Chicago:20250926T153000",
            "DTEND;TZID=America/Chicago:20250926T163000",
            "RRULE:FREQ=WEEKLY;WKST=MO;UNTIL=20251209T055959Z;INTERVAL=2;BYDAY=FR",
        ])]);
        let (events, warnings) = parse(&text);
        assert!(events.is_empty() && warnings.is_empty());
    }

    #[test]
    fn live_unsupported_recurrence_still_warns() {
        let text = wrap(&[vevent(&[
            "UID:live",
            "SUMMARY:Live biweekly",
            "DTSTART;TZID=America/Chicago:20260904T153000",
            "DTEND;TZID=America/Chicago:20260904T163000",
            "RRULE:FREQ=WEEKLY;WKST=MO;UNTIL=20261209T055959Z;INTERVAL=2;BYDAY=FR",
        ])]);
        let (events, warnings) = parse(&text);
        assert!(events.is_empty());
        assert_eq!(
            warnings,
            vec!["skipped recurrence (unsupported: WKST): Live biweekly"]
        );
    }
    // --- the Task 17 defect sweep (2026-09-02); every expectation measured against Python ---

    /// Python iterates whatever `calendars:` holds. A string is 15 characters, so 15 warnings,
    /// each naming `str`; a mapping is its keys, so one per key naming the key's type; a list
    /// element that is not a mapping names ITS type. The snapshot is rewritten from the previous
    /// one in every case, exactly as for a feed that failed to fetch.
    #[test]
    fn preserved_defect_20_a_string_calendars_value_warns_once_per_character() {
        let dir = tmp_vault("calstr");
        write(
            &dir.join("config").join("ingest.yaml"),
            "timezone: America/Chicago\ncalendars: \"https://x/y.ics\"\n",
        );
        write(
            &dir.join("state").join("calendar.md"),
            "# snap\n- calendar \u{b7} 2026-09-08 10:00\u{2013}11:00 \u{b7} Kept\n",
        );
        let fetch = |_: &str| panic!("nothing is ever fetched");
        let (events, warnings) = load_calendar_events(&dir, date(2026, 9, 7), Some(&fetch));
        assert_eq!(warnings.len(), 15, "{warnings:?}");
        assert!(warnings.iter().all(|w| w == "calendar: bad feed entry ('str' object has no attribute 'get')"));
        assert_eq!(titles(&events), vec!["Kept"], "the previous snapshot stands in for the feed");
        let snapshot = pystr::read_text(&dir.join("state").join("calendar.md")).unwrap();
        assert!(snapshot.contains("Kept"), "rewritten from the previous snapshot: {snapshot}");

        for (config, expected) in [
            ("calendars: {name: personal}\n", "calendar: bad feed entry ('str' object has no attribute 'get')"),
            ("calendars: {5: x}\n", "calendar: bad feed entry ('int' object has no attribute 'get')"),
            ("calendars: [[1]]\n", "calendar: bad feed entry ('list' object has no attribute 'get')"),
            ("calendars: [5]\n", "calendar: bad feed entry ('int' object has no attribute 'get')"),
            ("calendars: [null]\n", "calendar: bad feed entry ('NoneType' object has no attribute 'get')"),
        ] {
            write(&dir.join("config").join("ingest.yaml"), &format!("timezone: America/Chicago\n{config}"));
            let (_, warnings) = load_calendar_events(&dir, date(2026, 9, 7), Some(&fetch));
            assert_eq!(warnings, vec![expected.to_string()], "{config}");
        }
    }

    /// Python raises `TypeError: 'int' object is not iterable` out of `load_calendar_events`,
    /// which `cli.run` does not catch below its crash handler: the run FAILs and no page is
    /// written. Here it is one warning, and the previous snapshot is left exactly as it was —
    /// there is no fresh data to rewrite it from. A recorded divergence, not a preserved defect.
    #[test]
    fn a_scalar_calendars_value_is_one_warning_here_where_python_crashes_the_run() {
        let dir = tmp_vault("calscalar");
        let snapshot_path = dir.join("state").join("calendar.md");
        write(&snapshot_path, "# snap\n- personal \u{b7} 2026-09-08 10:00\u{2013}11:00 \u{b7} Kept\n");
        let before = pystr::read_text(&snapshot_path).unwrap();
        let fetch = |_: &str| panic!("nothing is ever fetched");
        for (config, expected) in [
            ("calendars: 5\n", "calendar: bad calendars value ('int' object is not iterable)"),
            ("calendars: 2.5\n", "calendar: bad calendars value ('float' object is not iterable)"),
            ("calendars: true\n", "calendar: bad calendars value ('bool' object is not iterable)"),
        ] {
            write(&dir.join("config").join("ingest.yaml"), &format!("timezone: America/Chicago\n{config}"));
            let (events, warnings) = load_calendar_events(&dir, date(2026, 9, 7), Some(&fetch));
            assert!(events.is_empty());
            assert_eq!(warnings, vec![expected.to_string()], "{config}");
        }
        assert_eq!(pystr::read_text(&snapshot_path).unwrap(), before, "the snapshot is not rewritten");
        // Falsy scalars are `or []` in Python: silence, and no snapshot write either.
        for config in ["calendars: 0\n", "calendars: false\n", "calendars: \"\"\n", "calendars: null\n", "calendars: []\n", "calendars: {}\n"] {
            write(&dir.join("config").join("ingest.yaml"), &format!("timezone: America/Chicago\n{config}"));
            let (events, warnings) = load_calendar_events(&dir, date(2026, 9, 7), Some(&fetch));
            assert!(events.is_empty() && warnings.is_empty(), "{config}: {warnings:?}");
        }
        assert_eq!(pystr::read_text(&snapshot_path).unwrap(), before);
    }

    /// An ICS `SUMMARY` may carry an escaped `\n`, which `unescape` turns into a real newline.
    /// The snapshot writes the title verbatim, so the event occupies two physical lines and
    /// only the first reads back: the busy time survives the round trip, the title's tail does
    /// not. Python does exactly this (measured 2026-09-02); the fallback page shows `Line one`.
    #[test]
    fn preserved_defect_21_a_multi_line_title_is_truncated_by_the_snapshot_round_trip() {
        let ics = wrap(&[vevent(&[
            "UID:x",
            "SUMMARY:Line one\\nLine two",
            "DTSTART;TZID=America/Chicago:20260907T090000",
            "DTEND;TZID=America/Chicago:20260907T093000",
        ])]);
        let (events, warnings) = parse_calendar_ics(&ics, &tz(), date(2026, 9, 7), HORIZON_DAYS);
        assert!(warnings.is_empty());
        assert_eq!(titles(&events), vec!["Line one\nLine two"]);

        let dir = tmp_vault("calnl");
        let path = dir.join("calendar.md");
        let mut by_feed = BTreeMap::new();
        by_feed.insert("personal".to_string(), events.clone());
        write_snapshot(&path, &by_feed).unwrap();
        let back = read_snapshot(&path);
        assert_eq!(titles(&back["personal"]), vec!["Line one"]);
        assert_eq!(back["personal"][0].start, events[0].start);
        assert_eq!(back["personal"][0].end, events[0].end);
    }
}

#[cfg(test)]
mod reference {
    //! THE ARTEFACT ORACLE for wave 4.
    //!
    //! `golden-today-full.md` cannot reach this code: that fixture deliberately carries no
    //! `ics_url`, so `load_calendar_events` takes its snapshot-fallback branch and neither
    //! `parse_calendar_ics` nor the RRULE expansion ever runs inside it (waves 4-7 plan, Task 1
    //! Step 1c). This closes that gap the same way `today.md` closed wave 3's.
    //!
    //! `tests/fixtures/calendar-snapshot-gcal.md` is the **byte-for-byte output of the Python
    //! engine** for `tests/fixtures/gcal.ics` at `window_start = 2026-09-07`, generated by
    //! `engine.calfeed.write_snapshot` and frozen. It is a complete, ordered dump of the parser
    //! and the recurrence generator — every expanded occurrence, in order, with its span
    //! formatted — so one comparison covers the whole module, exactly the argument that made
    //! `today.md` a sufficient oracle for waves 0-3.
    //!
    //! **If this fails, fix the Rust.** Never regenerate the fixture.

    use super::*;
    use jiff::civil::date;
    use std::fs;

    #[test]
    fn snapshot_of_the_gcal_fixture_matches_python_byte_for_byte() {
        let tz = TimeZone::get("America/Chicago").unwrap();
        let text = pystr::read_text(Path::new("tests/fixtures/gcal.ics")).unwrap();
        let (events, warnings) = parse_calendar_ics(&text, &tz, date(2026, 9, 7), HORIZON_DAYS);
        assert_eq!(warnings, vec!["skipped recurrence (MONTHLY): Advising lunch"]);

        let dir = std::env::temp_dir().join(format!("qo-calfeed-ref-{}", std::process::id()));
        let _ = fs::remove_dir_all(&dir);
        fs::create_dir_all(&dir).unwrap();
        let path = dir.join("calendar.md");
        let mut by_feed = BTreeMap::new();
        by_feed.insert("personal".to_string(), events);
        write_snapshot(&path, &by_feed).unwrap();

        let actual = fs::read(&path).unwrap();
        let expected = fs::read("tests/fixtures/calendar-snapshot-gcal.md").unwrap();

        // Bytes, not strings: CRLF and the multi-byte separators are the point. `str::lines()`
        // would strip the `\r` and hide exactly the class of bug this guards.
        if actual != expected {
            let a = String::from_utf8_lossy(&actual);
            let e = String::from_utf8_lossy(&expected);
            for (i, (al, el)) in a.split("\r\n").zip(e.split("\r\n")).enumerate() {
                assert_eq!(al, el, "first difference at line {}", i + 1);
            }
            panic!(
                "calendar.md differs from Python's ({} bytes vs {}). Fix the Rust; do NOT \
                 regenerate tests/fixtures/calendar-snapshot-gcal.md.",
                actual.len(),
                expected.len()
            );
        }
        assert_eq!(actual.len(), 561);
        assert_eq!(actual.windows(2).filter(|w| w == b"\r\n").count(), 9);
    }

    /// The snapshot is not just written — it is read back on the next failed fetch. A format
    /// change that writes fine and parses wrong loses every busy block on a bad-network morning.
    #[test]
    fn pythons_snapshot_reads_back_into_the_same_events() {
        let tz = TimeZone::get("America/Chicago").unwrap();
        let text = pystr::read_text(Path::new("tests/fixtures/gcal.ics")).unwrap();
        let (events, _) = parse_calendar_ics(&text, &tz, date(2026, 9, 7), HORIZON_DAYS);
        let restored = read_snapshot(Path::new("tests/fixtures/calendar-snapshot-gcal.md"));
        assert_eq!(restored["personal"], events);
    }

}
