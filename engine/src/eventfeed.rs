//! Fetching and parsing the four event source types — port of `engine/eventfeed.py`.
//!
//! Wave 5 of the Rust port. `ics`, `localist`, `engage`, `html`.
//!
//! # Two rules this module exists to enforce
//!
//! - **One dead feed must never fail a run.** Every fetch and every parse is contained to its own
//!   source; a failure warns and the loop continues. Python gets this from `except Exception`
//!   around each source, and Rust has no equivalent — so nothing here unwraps, indexes or panics.
//! - **A real `User-Agent` on every request.** Some UA hosts 403 the default. That is a total
//!   outage dressed as an empty feed, which is why [`EVENT_USER_AGENT`] is asserted by a test
//!   rather than merely set.
//!
//! The unversioned public APIs (Localist, Engage) drift without notice, so their parsers walk
//! `serde_json::Value` rather than deserialising into typed structs: a shape change must degrade
//! to a warning and a dropped event, never to a hard error.

use std::collections::BTreeMap;
use std::path::Path;
use std::sync::LazyLock;

use jiff::civil::{DateTime, Time};
use jiff::tz::TimeZone;
use jiff::{Span, Timestamp};
use regex::Regex;
use serde_json::Value as Json;

use crate::events::{load_events_config, DiscoveredEvent};
use crate::eventledger::sanitize_uid;
use crate::ingest::{parse_dt, parse_property, unescape, unfold, Due};

static TAG: LazyLock<Regex> = LazyLock::new(|| Regex::new(r"<[^>]+>").unwrap());
static WHITESPACE: LazyLock<Regex> = LazyLock::new(|| Regex::new(r"\s+").unwrap());

pub const EVENT_USER_AGENT: &str = "Mozilla/5.0 (Windows NT 10.0; Win64; x64) Knowlu/1.0";

/// Fetch a feed with a real User-Agent — some UA hosts 403 the default.
pub fn fetch_event_source(url: &str) -> Result<String, String> {
    let agent: ureq::Agent = ureq::Agent::config_builder()
        .timeout_global(Some(std::time::Duration::from_secs(30)))
        .build()
        .into();
    let mut response = agent
        .get(url)
        .header("User-Agent", EVENT_USER_AGENT)
        .call()
        .map_err(|e| e.to_string())?;
    let bytes = response
        .body_mut()
        .with_config()
        .limit(u64::MAX)
        .read_to_vec()
        .map_err(|e| e.to_string())?;
    Ok(String::from_utf8_lossy(&bytes).into_owned())
}

/// Python's `str(value)` over a JSON scalar -- the spelling `sanitize_uid` and the `str(...)`
/// coercions below see. Shared with `zybooks` since wave 6, hence its home in `pystr`.
use crate::pystr::json_str as python_str;

fn get<'a>(value: &'a Json, key: &str) -> Option<&'a Json> {
    value.get(key).filter(|v| !v.is_null())
}

/// `str(raw.get(key) or "").strip()`
fn text_field(value: &Json, key: &str) -> String {
    match get(value, key) {
        Some(v) => {
            let s = python_str(v);
            // Python's `or ""` treats an empty string and 0 as falsey.
            if s == "0" && v.is_number() {
                String::new()
            } else {
                s.trim().to_string()
            }
        }
        None => String::new(),
    }
}

/// Everything inside the engine is naive America/Chicago wall time.
///
/// Python takes a `tz` here and this does not, because Python's `_as_naive_local` has a branch
/// this port cannot reach: `if value.tzinfo is not None: return value.astimezone(tz)...`. Its
/// only caller passes the result of `parse_dt`, which already resolved `TZID` and trailing `Z`
/// and returns a NAIVE datetime — so `tzinfo` is always None and the conversion never runs.
/// `Due` makes that structural rather than incidental: there is no offset left to apply.
/// The date branch is live and is the whole remaining job.
fn as_naive_local(due: Due) -> DateTime {
    match due {
        Due::DateTime(dt) => dt,
        Due::Date(d) => d.to_datetime(Time::midnight()),
    }
}

fn plus_one_hour(start: DateTime) -> DateTime {
    start
        .checked_add(Span::new().hours(1))
        .unwrap_or(start)
}

/// Parse an iCalendar feed into `DiscoveredEvent`s. Never fails.
pub fn parse_ics_events(
    text: &str,
    source: &str,
    tz: &TimeZone,
) -> (Vec<DiscoveredEvent>, Vec<String>) {
    let mut warnings: Vec<String> = Vec::new();
    if !text.contains("BEGIN:VCALENDAR") {
        return (Vec::new(), vec!["not an ICS response".to_string()]);
    }
    let mut events: Vec<DiscoveredEvent> = Vec::new();
    let mut block: Option<Vec<String>> = None;
    for line in unfold(text) {
        if line == "BEGIN:VEVENT" {
            block = Some(Vec::new());
        } else if line == "END:VEVENT" && block.is_some() {
            let b = block.take().unwrap_or_default();
            match event_from_block(&b, source, tz) {
                (_, Some(problem)) => warnings.push(problem),
                (Some(event), None) => events.push(event),
                (None, None) => {}
            }
        } else if let Some(b) = block.as_mut() {
            b.push(line);
        }
    }
    sort_events(&mut events);
    (events, warnings)
}

fn sort_events(events: &mut [DiscoveredEvent]) {
    events.sort_by(|a, b| (a.start(), &a.uid).cmp(&(b.start(), &b.uid)));
}

fn event_from_block(
    block: &[String],
    source: &str,
    tz: &TimeZone,
) -> (Option<DiscoveredEvent>, Option<String>) {
    let mut fields: BTreeMap<String, String> = BTreeMap::new();
    let mut start: Option<DateTime> = None;
    let mut end: Option<DateTime> = None;
    for line in block {
        let Some((name, params, value)) = parse_property(line) else {
            continue;
        };
        match name.as_str() {
            // A DTSTART Python cannot parse raises and is skipped by the `except` — leaving
            // `start` None, which becomes "no start, skipped" below.
            "DTSTART" => start = parse_dt(&value, &params, tz).map(as_naive_local),
            "DTEND" => end = parse_dt(&value, &params, tz).map(as_naive_local),
            // Last occurrence of a repeated property wins, as dict assignment does.
            _ => {
                fields.insert(name, unescape(&value));
            }
        }
    }
    let uid = fields.get("UID").map(|s| s.trim()).unwrap_or("");
    if uid.is_empty() {
        let summary = fields
            .get("SUMMARY")
            .cloned()
            .unwrap_or_else(|| "untitled".to_string());
        return (None, Some(format!("{summary}: no uid, skipped")));
    }
    let Some(start) = start else {
        return (None, Some(format!("{uid}: no start, skipped")));
    };
    let end = match end {
        Some(e) if e > start => e,
        _ => plus_one_hour(start),
    };
    let title = fields.get("SUMMARY").map(|s| s.trim()).unwrap_or("");
    let title = if title.is_empty() { "(untitled)" } else { title };
    (
        Some(
            DiscoveredEvent {
                // Sanitized at mint: an ICS UID may legally contain characters the verdict
                // ledger's line format cannot read back (see `sanitize_uid`).
                uid: format!("ics:{}", sanitize_uid(uid)),
                title: title.to_string(),
                start: Some(start),
                end: Some(end),
                source: source.to_string(),
                location: fields.get("LOCATION").map(|s| s.trim().to_string()).unwrap_or_default(),
                url: fields.get("URL").map(|s| s.trim().to_string()).unwrap_or_default(),
                description: fields
                    .get("DESCRIPTION")
                    .map(|s| s.trim().to_string())
                    .unwrap_or_default(),
                ..Default::default()
            }
            .normalized(),
        ),
        None,
    )
}

/// HTML sources carry no structure worth parsing deterministically.
///
/// By design this yields nothing: `students.eng.ua.edu` and `edge.culverhouse.ua.edu` are
/// hand-maintained pages with no feed. Keeping the adapter dumb means there are no brittle
/// selectors to break when they change, and a page that starts publishing events costs one prompt
/// change rather than a parser rewrite.
pub fn parse_html_events(
    _text: &str,
    _source: &str,
    _url: &str,
) -> (Vec<DiscoveredEvent>, Vec<String>) {
    (Vec::new(), Vec::new())
}

/// Strip tags to plain text so a routine step can read a scraped page.
pub fn html_text_excerpt(text: &str, limit: usize) -> String {
    let stripped = TAG.replace_all(text, " ");
    let collapsed = WHITESPACE.replace_all(&stripped, " ");
    let trimmed = collapsed.trim();
    // Python slices by CHARACTER, so this must too — a byte slice would split a multi-byte
    // codepoint and panic, which is exactly the crash this pass may not have.
    trimmed.chars().take(limit).collect()
}

/// `datetime.fromisoformat(str(raw).replace("Z", "+00:00"))`, then naive-local.
fn iso_to_local(raw: Option<&Json>, tz: &TimeZone) -> Option<DateTime> {
    let text = python_str(raw?);
    let text = text.replace('Z', "+00:00");
    // Offset-bearing first: those are real instants and must be converted, not truncated.
    if let Ok(ts) = text.parse::<Timestamp>() {
        return Some(ts.to_zoned(tz.clone()).datetime());
    }
    if let Ok(dt) = text.parse::<DateTime>() {
        return Some(dt);
    }
    if let Ok(d) = text.parse::<jiff::civil::Date>() {
        return Some(d.to_datetime(Time::midnight()));
    }
    None
}

/// Names out of a feed's list field.
///
/// Anything that is not a list is nothing — a bare string would otherwise iterate one character
/// at a time — and a `None` entry is skipped rather than becoming the string `"None"`, which is
/// what an absent Engage `theme` produced.
fn names(values: Option<&Json>) -> Vec<String> {
    let Some(Json::Array(items)) = values else {
        return Vec::new();
    };
    let mut out = Vec::new();
    for item in items {
        if item.is_null() {
            continue;
        }
        let name = if item.is_object() {
            match get(item, "name") {
                Some(v) => python_str(v).trim().to_string(),
                None => String::new(),
            }
        } else {
            python_str(item).trim().to_string()
        };
        if !name.is_empty() {
            out.push(name);
        }
    }
    out
}

/// Parse a Localist `/api/2/events` payload. Never fails.
pub fn parse_localist(
    payload: &Json,
    source: &str,
    tz: &TimeZone,
) -> (Vec<DiscoveredEvent>, Vec<String>) {
    let Some(Json::Array(wrappers)) = payload.get("events") else {
        return (
            Vec::new(),
            vec!["unexpected Localist payload shape".to_string()],
        );
    };
    let mut events: Vec<DiscoveredEvent> = Vec::new();
    let mut warnings: Vec<String> = Vec::new();
    for wrapper in wrappers {
        let raw = wrapper.get("event");
        let Some(raw) = raw.filter(|r| r.is_object()) else {
            warnings.push("unexpected Localist event shape".to_string());
            continue;
        };
        let event_id = raw.get("id");
        let series = format!(
            "localist:{}",
            sanitize_uid(&event_id.map(python_str).unwrap_or_else(|| "None".into()))
        );
        let empty = Vec::new();
        let instances = match raw.get("event_instances") {
            Some(Json::Array(v)) => v,
            _ => &empty,
        };
        let filters = raw.get("filters").filter(|f| f.is_object());
        let mut categories = names(filters.and_then(|f| f.get("event_topic")));
        categories.extend(names(raw.get("tags")));
        let audiences = names(filters.and_then(|f| f.get("event_target_audience")));
        let organizer = names(raw.get("departments")).first().cloned().unwrap_or_default();

        let mut made = 0;
        for holder in instances {
            let Some(inst) = holder.get("event_instance").filter(|i| i.is_object()) else {
                continue;
            };
            let Some(start) = iso_to_local(inst.get("start"), tz) else {
                continue;
            };
            let end = iso_to_local(inst.get("end"), tz).unwrap_or_else(|| plus_one_hour(start));
            let uid = match inst.get("id").filter(|v| !v.is_null()) {
                None => series.clone(),
                Some(id) => format!("{series}:{}", sanitize_uid(&python_str(id))),
            };
            let title = text_field(raw, "title");
            events.push(
                DiscoveredEvent {
                    uid,
                    title: if title.is_empty() { "(untitled)".into() } else { title },
                    start: Some(start),
                    end: Some(end),
                    source: source.to_string(),
                    organizer: organizer.clone(),
                    location: text_field(raw, "location_name"),
                    url: {
                        let u = text_field(raw, "localist_url");
                        if u.is_empty() { text_field(raw, "url") } else { u }
                    },
                    description: text_field(raw, "description_text"),
                    categories: categories.clone(),
                    audiences: audiences.clone(),
                    registration: matches!(raw.get("has_register"), Some(Json::Bool(true))),
                    series_uid: series.clone(),
                    ..Default::default()
                }
                .normalized(),
            );
            made += 1;
        }
        if made == 0 {
            let id = event_id.map(python_str).unwrap_or_else(|| "None".into());
            warnings.push(format!("localist event {id}: no usable instances"));
        }
    }
    sort_events(&mut events);
    (events, warnings)
}

/// Parse an Engage discovery payload. `startsOn`/`endsOn` are UTC. Never fails.
pub fn parse_engage(
    payload: &Json,
    source: &str,
    tz: &TimeZone,
) -> (Vec<DiscoveredEvent>, Vec<String>) {
    let Some(Json::Array(items)) = payload.get("value") else {
        return (
            Vec::new(),
            vec!["unexpected Engage payload shape".to_string()],
        );
    };
    let mut events: Vec<DiscoveredEvent> = Vec::new();
    let mut warnings: Vec<String> = Vec::new();
    for raw in items {
        if !raw.is_object() {
            warnings.push("unexpected Engage event shape".to_string());
            continue;
        }
        let status = text_field(raw, "status").to_lowercase();
        if !status.is_empty() && status != "approved" {
            continue;
        }
        let Some(start) = iso_to_local(raw.get("startsOn"), tz) else {
            let id = raw.get("id").map(python_str).unwrap_or_else(|| "None".into());
            warnings.push(format!("engage event {id}: unparseable start"));
            continue;
        };
        let end = iso_to_local(raw.get("endsOn"), tz).unwrap_or_else(|| plus_one_hour(start));
        let title = text_field(raw, "name");
        let mut categories = names(raw.get("categoryNames"));
        // `_names([raw.get("theme")])` — wrapped in a list on purpose, so a None theme yields
        // nothing rather than the string "None".
        let theme = Json::Array(vec![raw.get("theme").cloned().unwrap_or(Json::Null)]);
        categories.extend(names(Some(&theme)));
        events.push(
            DiscoveredEvent {
                uid: format!(
                    "engage:{}",
                    sanitize_uid(&raw.get("id").map(python_str).unwrap_or_else(|| "None".into()))
                ),
                title: if title.is_empty() { "(untitled)".into() } else { title },
                start: Some(start),
                end: Some(end),
                source: source.to_string(),
                organizer: text_field(raw, "organizationName"),
                location: text_field(raw, "location"),
                url: text_field(raw, "url"),
                description: html_text_excerpt(&text_field(raw, "description"), 1000),
                categories,
                benefits: names(raw.get("benefitNames")),
                ..Default::default()
            }
            .normalized(),
        );
    }
    sort_events(&mut events);
    (events, warnings)
}

/// Fetch every enabled source. One dead feed must never fail a run.
pub fn load_discovered_events(
    vault: &Path,
    fetcher: Option<&dyn Fn(&str) -> Result<String, String>>,
) -> (Vec<DiscoveredEvent>, Vec<String>) {
    let default_fetch = |url: &str| fetch_event_source(url);
    let fetch: &dyn Fn(&str) -> Result<String, String> = fetcher.unwrap_or(&default_fetch);

    let (config, mut warnings) = load_events_config(&vault.join("config").join("events.yaml"));
    if config.sources.is_empty() {
        return (Vec::new(), warnings);
    }
    let tz = match TimeZone::get(&config.timezone) {
        Ok(tz) => tz,
        Err(err) => {
            warnings.push(format!("config: bad timezone ({err}); using America/Chicago"));
            TimeZone::get("America/Chicago").unwrap_or(TimeZone::UTC)
        }
    };

    // First uid wins, and insertion order is preserved for the tie — the sort below is on
    // `(start, uid)`, so cross-source duplicates collapse before ordering.
    let mut by_uid: Vec<DiscoveredEvent> = Vec::new();
    let mut seen: std::collections::HashSet<String> = std::collections::HashSet::new();

    for source in &config.sources {
        if !source.enabled {
            continue;
        }
        let text = match fetch(&source.url) {
            Ok(text) => text,
            Err(err) => {
                warnings.push(format!("{}: fetch failed ({err})", source.name));
                continue;
            }
        };
        let (parsed, source_warnings) = match source.kind.as_str() {
            "ics" => parse_ics_events(&text, &source.name, &tz),
            "localist" | "engage" => match serde_json::from_str::<Json>(&text) {
                Ok(payload) => {
                    if source.kind == "localist" {
                        parse_localist(&payload, &source.name, &tz)
                    } else {
                        parse_engage(&payload, &source.name, &tz)
                    }
                }
                Err(err) => {
                    // Python's `json.loads` raises inside the same `try` that wraps the adapters,
                    // so a malformed body is a parse failure for that source and nothing else.
                    warnings.push(format!("{}: parse failed ({err})", source.name));
                    continue;
                }
            },
            _ => parse_html_events(&text, &source.name, &source.url),
        };
        for w in source_warnings {
            warnings.push(format!("{}: {w}", source.name));
        }
        for event in parsed {
            if seen.insert(event.uid.clone()) {
                by_uid.push(event);
            }
        }
    }
    sort_events(&mut by_uid);
    (by_uid, warnings)
}

#[cfg(test)]
mod tests {
    //! Port of `tests/test_eventfeed_ics.py`, `test_eventfeed_json.py` and
    //! `test_eventfeed_load.py` — 23 tests, same names.

    use super::*;
    use jiff::civil::date;
    use serde_json::json;
    use std::cell::RefCell;
    use std::fs;
    use std::path::PathBuf;

    fn tz() -> TimeZone {
        TimeZone::get("America/Chicago").unwrap()
    }

    const BLOUNT: &str = "BEGIN:VCALENDAR\nVERSION:2.0\nPRODID:-//The Blount Scholars Program - ECPv6.17.2//NONSGML v1.0//EN\nBEGIN:VEVENT\nDTSTART;TZID=America/Chicago:20260819T160000\nDTEND;TZID=America/Chicago:20260819T170000\nUID:10010725-1787155200-1787158800@blount.as.ua.edu\nSUMMARY:Convocation - Introduction to Foundations\nDESCRIPTION:Dr. Deborah Keene will give an overview.\nLOCATION:Lloyd Hall 38\nURL:https://blount.as.ua.edu/event/convocation/\nEND:VEVENT\nEND:VCALENDAR\n";

    // --- ICS ----------------------------------------------------------------------------

    #[test]
    fn parses_a_blount_event() {
        let (events, warnings) = parse_ics_events(BLOUNT, "blount", &tz());
        assert!(warnings.is_empty());
        assert_eq!(events.len(), 1);
        let e = &events[0];
        assert_eq!(e.uid, "ics:10010725-1787155200-1787158800@blount.as.ua.edu");
        assert_eq!(e.title, "Convocation - Introduction to Foundations");
        assert_eq!(e.start(), date(2026, 8, 19).at(16, 0, 0, 0));
        assert_eq!(e.end(), date(2026, 8, 19).at(17, 0, 0, 0));
        assert_eq!(e.location, "Lloyd Hall 38");
        assert_eq!(e.url, "https://blount.as.ua.edu/event/convocation/");
        assert_eq!(e.source, "blount");
    }

    #[test]
    fn utc_timestamps_convert_to_central() {
        let text = "BEGIN:VCALENDAR\nBEGIN:VEVENT\nDTSTART:20260820T133000Z\nDTEND:20260820T190000Z\nUID:utc-1\nSUMMARY:UTC Event\nEND:VEVENT\nEND:VCALENDAR\n";
        let (events, _) = parse_ics_events(text, "s", &tz());
        assert_eq!(events[0].start(), date(2026, 8, 20).at(8, 30, 0, 0));
        assert_eq!(events[0].end(), date(2026, 8, 20).at(14, 0, 0, 0));
    }

    #[test]
    fn event_without_uid_is_skipped_with_warning() {
        let text = "BEGIN:VCALENDAR\nBEGIN:VEVENT\nDTSTART;TZID=America/Chicago:20260901T120000\nSUMMARY:No UID\nEND:VEVENT\nEND:VCALENDAR\n";
        let (events, warnings) = parse_ics_events(text, "s", &tz());
        assert!(events.is_empty());
        assert!(warnings.iter().any(|w| w.to_lowercase().contains("uid")));
    }

    #[test]
    fn all_day_event_gets_a_midnight_to_midnight_span() {
        let text = "BEGIN:VCALENDAR\nBEGIN:VEVENT\nDTSTART;VALUE=DATE:20260905\nDTEND;VALUE=DATE:20260906\nUID:allday-1\nSUMMARY:All Day\nEND:VEVENT\nEND:VCALENDAR\n";
        let (events, _) = parse_ics_events(text, "s", &tz());
        assert_eq!(events[0].start(), date(2026, 9, 5).at(0, 0, 0, 0));
        assert_eq!(events[0].end(), date(2026, 9, 6).at(0, 0, 0, 0));
    }

    #[test]
    fn missing_dtend_defaults_to_one_hour() {
        let text = "BEGIN:VCALENDAR\nBEGIN:VEVENT\nDTSTART;TZID=America/Chicago:20260901T120000\nUID:no-end\nSUMMARY:Open Ended\nEND:VEVENT\nEND:VCALENDAR\n";
        let (events, _) = parse_ics_events(text, "s", &tz());
        assert_eq!(events[0].end(), date(2026, 9, 1).at(13, 0, 0, 0));
    }

    #[test]
    fn garbage_text_warns_rather_than_raising() {
        let (events, warnings) = parse_ics_events("<html>not ics</html>", "s", &tz());
        assert!(events.is_empty());
        assert!(warnings.iter().any(|w| w.contains("not an ICS response")));
    }

    #[test]
    fn html_source_yields_no_events_but_never_raises() {
        let (events, warnings) = parse_html_events("<html><body>Nothing</body></html>", "eng", "u");
        assert!(events.is_empty());
        assert!(warnings.is_empty());
    }

    // --- JSON ---------------------------------------------------------------------------

    fn localist() -> Json {
        json!({"events": [{"event": {
            "id": 52709738382567i64,
            "title": "Co-op Info Session",
            "localist_url": "https://calendar.ua.edu/event/coop",
            "description_text": "Learn about Co-op.",
            "location_name": "H.M. Comer 2004",
            "has_register": true,
            "departments": [{"name": "Career Center"}],
            "keywords": ["co-op"],
            "tags": ["career"],
            "filters": {
                "event_target_audience": [{"name": "Students"}],
                "event_topic": [{"name": "Career"}]
            },
            "event_instances": [
                {"event_instance": {"start": "2026-09-02T12:00:00-05:00",
                                    "end": "2026-09-02T13:00:00-05:00"}}
            ]
        }}]})
    }

    fn engage_event() -> Json {
        json!({
            "id": 12566153,
            "name": "Campus Tour and Welcome Social",
            "organizationId": 63798,
            "organizationName": "Graduate Student Association",
            "location": "UA Student Center",
            "description": "<p>Come along</p>",
            "startsOn": "2026-08-20T13:30:00+00:00",
            "endsOn": "2026-08-20T19:00:00+00:00",
            "categoryNames": ["Party, Social Event", "Networking"],
            "benefitNames": ["Free Food"],
            "theme": "Social",
            "status": "Approved"
        })
    }

    fn engage() -> Json {
        json!({"@odata.count": 1, "value": [engage_event()]})
    }

    #[test]
    fn localist_event_normalizes_to_central() {
        let (events, warnings) = parse_localist(&localist(), "campus", &tz());
        assert!(warnings.is_empty(), "{warnings:?}");
        let e = &events[0];
        assert_eq!(e.uid, "localist:52709738382567");
        assert_eq!(e.start(), date(2026, 9, 2).at(12, 0, 0, 0));
        assert_eq!(e.end(), date(2026, 9, 2).at(13, 0, 0, 0));
        assert_eq!(e.organizer, "Career Center");
        assert_eq!(e.location, "H.M. Comer 2004");
        assert!(e.registration);
        assert!(e.audiences.contains(&"Students".to_string()));
        assert!(e.categories.contains(&"Career".to_string()));
    }

    #[test]
    fn engage_utc_becomes_central_wall_time() {
        let (events, warnings) = parse_engage(&engage(), "clubs", &tz());
        assert!(warnings.is_empty(), "{warnings:?}");
        let e = &events[0];
        assert_eq!(e.uid, "engage:12566153");
        // 13:30Z on 2026-08-20 is 08:30 CDT, not 13:30.
        assert_eq!(e.start(), date(2026, 8, 20).at(8, 30, 0, 0));
        assert_eq!(e.end(), date(2026, 8, 20).at(14, 0, 0, 0));
        assert_eq!(e.organizer, "Graduate Student Association");
        assert_eq!(e.benefits, vec!["Free Food"]);
        assert!(e.categories.contains(&"Networking".to_string()));
        assert_eq!(e.description, "Come along");
    }

    #[test]
    fn engage_skips_unapproved_events() {
        let mut ev = engage_event();
        ev["status"] = json!("Pending");
        ev["id"] = json!(999);
        let (events, _) = parse_engage(&json!({"value": [ev]}), "clubs", &tz());
        assert!(events.is_empty());
    }

    #[test]
    fn localist_recurring_instances_share_a_series_uid() {
        let payload = json!({"events": [{"event": {
            "id": 77,
            "title": "Weekly Meeting",
            "event_instances": [
                {"event_instance": {"id": 1, "start": "2026-09-02T18:00:00-05:00"}},
                {"event_instance": {"id": 2, "start": "2026-09-09T18:00:00-05:00"}}
            ]
        }}]});
        let (events, _) = parse_localist(&payload, "campus", &tz());
        assert_eq!(events.len(), 2);
        assert_ne!(events[0].uid, events[1].uid);
        assert_eq!(events[0].series_uid, "localist:77");
        assert_eq!(events[1].series_uid, "localist:77");
    }

    #[test]
    fn shape_drift_warns_and_drops_that_event_only() {
        let good = localist()["events"][0]["event"].clone();
        let payload = json!({"events": [{"event": {"id": 1}}, {"event": good}]});
        let (events, warnings) = parse_localist(&payload, "campus", &tz());
        assert_eq!(events.len(), 1);
        assert!(warnings.iter().any(|w| w.contains('1')));
    }

    #[test]
    fn an_absent_engage_theme_does_not_become_the_string_none() {
        let mut ev = engage_event();
        ev["theme"] = Json::Null;
        let (events, _) = parse_engage(&json!({"value": [ev]}), "clubs", &tz());
        assert!(!events[0].categories.contains(&"None".to_string()));
    }

    #[test]
    fn a_scalar_where_a_list_is_expected_is_not_iterated_by_character() {
        // Unversioned public API: a shape change must not turn "Social" into
        // ('S','o','c','i','a','l') and pollute every downstream match.
        let mut ev = engage_event();
        ev["categoryNames"] = json!("Networking");
        ev["benefitNames"] = json!("Free Food");
        let (events, _) = parse_engage(&json!({"value": [ev]}), "clubs", &tz());
        assert_eq!(events[0].categories, vec!["Social"]);
        assert!(events[0].benefits.is_empty());
    }

    #[test]
    fn unexpected_top_level_shape_warns_rather_than_raising() {
        let (events, warnings) = parse_localist(&json!({"nope": true}), "campus", &tz());
        assert!(events.is_empty());
        assert!(warnings.iter().any(|w| w.contains("unexpected")));
        let (events, warnings) = parse_engage(&json!(["not", "a", "dict"]), "clubs", &tz());
        assert!(events.is_empty());
        assert!(warnings.iter().any(|w| w.contains("unexpected")));
    }

    // --- load_discovered_events ---------------------------------------------------------

    const ICS: &str = "BEGIN:VCALENDAR\nBEGIN:VEVENT\nDTSTART;TZID=America/Chicago:20260901T120000\nUID:blount-1\nSUMMARY:Blount Thing\nEND:VEVENT\nEND:VCALENDAR\n";
    const ENGAGE_JSON: &str = r#"{"value": [{"id": 5, "name": "Club Night", "startsOn": "2026-09-02T23:00:00+00:00", "endsOn": "2026-09-03T00:00:00+00:00", "status": "Approved"}]}"#;

    fn tmp_vault(name: &str) -> PathBuf {
        let dir = std::env::temp_dir().join(format!("qo-eventfeed-{}-{}", name, std::process::id()));
        let _ = fs::remove_dir_all(&dir);
        fs::create_dir_all(&dir).unwrap();
        dir
    }

    fn vault_with(name: &str, sources: &str) -> PathBuf {
        let dir = tmp_vault(name);
        fs::create_dir_all(dir.join("config")).unwrap();
        crate::pystr::write_text(&dir.join("config").join("events.yaml"), sources).unwrap();
        dir
    }

    fn titles(events: &[DiscoveredEvent]) -> Vec<&str> {
        events.iter().map(|e| e.title.as_str()).collect()
    }

    #[test]
    fn no_config_means_no_events_and_no_warnings() {
        let dir = tmp_vault("noconfig");
        let fetch = |_: &str| Ok(String::new());
        let (events, warnings) = load_discovered_events(&dir, Some(&fetch));
        assert!(events.is_empty());
        assert!(warnings.is_empty());
    }

    #[test]
    fn disabled_sources_are_not_fetched() {
        let vault = vault_with(
            "disabled",
            "sources:\n  - name: blount\n    type: ics\n    url: http://x\n    enabled: false\n",
        );
        let called = RefCell::new(Vec::new());
        let fetch = |url: &str| {
            called.borrow_mut().push(url.to_string());
            Ok(ICS.to_string())
        };
        load_discovered_events(&vault, Some(&fetch));
        assert!(called.borrow().is_empty());
    }

    #[test]
    fn enabled_source_is_fetched_and_parsed() {
        let vault = vault_with(
            "enabled",
            "sources:\n  - name: blount\n    type: ics\n    url: http://x\n    enabled: true\n",
        );
        let fetch = |_: &str| Ok(ICS.to_string());
        let (events, warnings) = load_discovered_events(&vault, Some(&fetch));
        assert_eq!(titles(&events), vec!["Blount Thing"]);
        assert!(warnings.is_empty());
    }

    #[test]
    fn one_dead_feed_never_kills_the_others() {
        let vault = vault_with(
            "onedead",
            "sources:\n  - name: dead\n    type: ics\n    url: http://dead\n    enabled: true\n  - name: alive\n    type: ics\n    url: http://alive\n    enabled: true\n",
        );
        let fetch = |url: &str| {
            if url.contains("dead") {
                Err("connection refused".to_string())
            } else {
                Ok(ICS.to_string())
            }
        };
        let (events, warnings) = load_discovered_events(&vault, Some(&fetch));
        assert_eq!(titles(&events), vec!["Blount Thing"]);
        assert!(warnings.iter().any(|w| w.starts_with("dead:")));
    }

    #[test]
    fn adapter_exception_is_contained_to_its_source() {
        let vault = vault_with(
            "adapter",
            "sources:\n  - name: clubs\n    type: engage\n    url: http://c\n    enabled: true\n  - name: blount\n    type: ics\n    url: http://b\n    enabled: true\n",
        );
        let fetch = |url: &str| {
            if url.contains("://c") {
                Ok("{ not json".to_string())
            } else {
                Ok(ICS.to_string())
            }
        };
        let (events, warnings) = load_discovered_events(&vault, Some(&fetch));
        assert_eq!(titles(&events), vec!["Blount Thing"]);
        assert!(warnings.iter().any(|w| w.starts_with("clubs:")));
    }

    #[test]
    fn engage_json_is_parsed() {
        let vault = vault_with(
            "engagejson",
            "sources:\n  - name: clubs\n    type: engage\n    url: http://c\n    enabled: true\n",
        );
        let fetch = |_: &str| Ok(ENGAGE_JSON.to_string());
        let (events, _) = load_discovered_events(&vault, Some(&fetch));
        assert_eq!(titles(&events), vec!["Club Night"]);
    }

    #[test]
    fn duplicate_uids_across_sources_are_deduped() {
        let vault = vault_with(
            "dupes",
            "sources:\n  - name: a\n    type: ics\n    url: http://a\n    enabled: true\n  - name: b\n    type: ics\n    url: http://b\n    enabled: true\n",
        );
        let fetch = |_: &str| Ok(ICS.to_string());
        let (events, _) = load_discovered_events(&vault, Some(&fetch));
        assert_eq!(events.len(), 1);
    }

    /// Python monkeypatches the module global to prove the default fetcher is reached. Rust has no
    /// monkeypatching, so this proves it the other way: with no fetcher supplied, the REAL
    /// `fetch_event_source` runs and fails on an unknown URL scheme — which it can only do if it
    /// was actually called. An unknown scheme fails in the URL parser without opening a socket.
    #[test]
    fn the_default_fetcher_is_used_when_none_is_supplied() {
        let vault = vault_with(
            "defaultfetch",
            "sources:\n  - name: blount\n    type: ics\n    url: unreachable://x\n    enabled: true\n",
        );
        let (events, warnings) = load_discovered_events(&vault, None);
        assert!(events.is_empty());
        assert!(
            warnings.iter().any(|w| w.starts_with("blount: fetch failed (")),
            "{warnings:?}"
        );
    }

    #[test]
    fn explicit_fetcher_overrides_the_default() {
        let vault = vault_with(
            "override",
            "sources:\n  - name: blount\n    type: ics\n    url: unreachable://x\n    enabled: true\n",
        );
        // The URL is unfetchable, so a result at all proves the explicit fetcher was used.
        let fetch = |_: &str| Ok(ICS.to_string());
        let (events, warnings) = load_discovered_events(&vault, Some(&fetch));
        assert_eq!(titles(&events), vec!["Blount Thing"]);
        assert!(warnings.is_empty());
    }

    /// Not in the Python suite. A missing `User-Agent` makes some UA hosts 403 every request —
    /// a total outage that looks like an empty feed. Offline tests cannot observe a header, so
    /// the constant is asserted directly; otherwise nothing in the suite would catch its removal.
    #[test]
    fn the_event_user_agent_is_a_real_browser_string() {
        assert!(EVENT_USER_AGENT.starts_with("Mozilla/5.0"));
        assert!(EVENT_USER_AGENT.contains("Knowlu"));
    }

    /// Not in the Python suite. `html_text_excerpt` slices by character in Python; a byte slice
    /// here would split a multi-byte codepoint and panic — in the one pass that may not panic.
    #[test]
    fn html_excerpt_truncates_by_character_not_byte() {
        let text = "<p>é</p>".repeat(10);
        let out = html_text_excerpt(&text, 5);
        assert_eq!(out.chars().count(), 5);
    }
}
