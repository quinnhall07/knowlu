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
use std::time::{Duration, Instant};

use jiff::civil::{DateTime, Time};
use jiff::tz::TimeZone;
use jiff::{Span, Timestamp};
use regex::Regex;
use serde_json::Value as Json;

use crate::events::{load_events_config, DiscoveredEvent, EventsSource};
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

/// F9: wall-clock budgets bounding Localist/Engage pagination. `per_source` bounds one source's
/// own paging; `per_run` bounds the whole call. Checked before every fetch, never mid-request.
#[derive(Debug, Clone, Copy)]
pub struct PagingLimits {
    pub per_source: Duration,
    pub per_run: Duration,
}

impl PagingLimits {
    /// Worst case is the budget plus one page already in flight: `180s + 150s` (120s through the
    /// cloud proxy plus 30s for the direct fallback) `= 330s`, 5.5 minutes — leaving `rank` about
    /// 14.5 of its 20-minute `scheduler::CHILD_TIMEOUT` step budget for the calendar fetch and
    /// everything else, the same order of headroom `enrich::BATCH_BUDGET` leaves `judge` (17 of
    /// 20). At a normal 1-3s a page, 90s a source is 30+ pages — well above the ~14 Localist pages
    /// a 90-day window needs — so the budget only bites on a network that is already failing.
    pub const DEFAULT: PagingLimits =
        PagingLimits { per_source: Duration::from_secs(90), per_run: Duration::from_secs(180) };
}

/// One page of Localist or Engage, and the hard cap on how many a source gets in one call — also a
/// cap of 2,000 events.
const PAGE_SIZE: usize = 100;
const MAX_PAGES: usize = 20;

/// The engine's own query keys, in the fixed order they are appended. Any already in the
/// configured URL are stripped first; everything else in the query is kept, in its original order.
const LOCALIST_KEYS: [&str; 3] = ["days", "pp", "page"];
const ENGAGE_KEYS: [&str; 6] =
    ["endsAfter", "orderByField", "orderByDirection", "status", "take", "skip"];

/// Strip `keys` from `base`'s query wherever they already appear, then append `values` in order. A
/// fragment (`#...`) is set aside first and re-appended after the query, so it is never swallowed
/// into it (review M-4). Nothing is percent-decoded or re-encoded.
fn paged_url(base: &str, keys: &[&str], values: &[(&str, String)]) -> String {
    let (base, fragment) = match base.split_once('#') {
        Some((b, f)) => (b, Some(f)),
        None => (base, None),
    };
    let (path, query) = base.split_once('?').unwrap_or((base, ""));
    let mut parts: Vec<String> = query
        .split('&')
        .filter(|pair| !pair.is_empty())
        .filter(|pair| {
            let key = pair.split('=').next().unwrap_or(pair);
            !keys.contains(&key)
        })
        .map(str::to_string)
        .collect();
    parts.extend(values.iter().map(|(k, v)| format!("{k}={v}")));
    let mut url = format!("{path}?{}", parts.join("&"));
    if let Some(f) = fragment {
        url.push('#');
        url.push_str(f);
    }
    url
}

/// `int(value)` over a JSON number — Localist's `page.total` and Engage's `@odata.count`.
fn json_i64(value: Option<&Json>) -> Option<i64> {
    let value = value?;
    value.as_i64().or_else(|| value.as_f64().map(|f| f as i64))
}

/// `None` to proceed. `Some(true)`: the whole call is out of time. `Some(false)`: this source's own
/// clock is up. Page 1 only answers to the per-run budget, because a source's own clock has not
/// started yet.
fn budget_stop(
    page: usize,
    source_start: Duration,
    elapsed: &dyn Fn() -> Duration,
    limits: &PagingLimits,
) -> Option<bool> {
    let now = elapsed();
    if now >= limits.per_run {
        return Some(true);
    }
    if page > 1 && now.saturating_sub(source_start) >= limits.per_source {
        return Some(false);
    }
    None
}

/// What one Localist/Engage page said about continuation. `declared` is whether the page carried a
/// parseable `page.total`/`@odata.count` at all — independent of `keep_going` — so the *next*
/// page's unexpected emptiness can be named instead of silently truncating the roster (review M-2).
/// `raw_empty` is whether the page's own item array had nothing in it.
struct PageInfo {
    keep_going: bool,
    declared: bool,
    raw_empty: bool,
}

/// A paged source's read-only fetch context and its accumulators, bundled so `page_json_source`
/// takes one argument for each rather than seven (review M-7).
struct PagingCtx<'a> {
    tz: &'a TimeZone,
    fetch: &'a dyn Fn(&str) -> Result<String, String>,
    limits: &'a PagingLimits,
    elapsed: &'a dyn Fn() -> Duration,
    warnings: &'a mut Vec<String>,
    seen: &'a mut std::collections::HashSet<String>,
    by_uid: &'a mut Vec<DiscoveredEvent>,
}

/// Page one Localist or Engage source until its own stop condition, the page cap or a time budget
/// ends it. Every failure is contained to this source: a dead page warns and paging — or the whole
/// run — continues.
fn page_json_source(
    source: &EventsSource,
    ctx: &mut PagingCtx,
    build_url: impl Fn(usize) -> String,
    should_continue: impl Fn(&Json, usize) -> PageInfo,
    parse: fn(&Json, &str, &TimeZone) -> (Vec<DiscoveredEvent>, Vec<String>),
) {
    let source_start = (ctx.elapsed)();
    let mut kept = 0usize;
    let mut page = 1usize;
    // This source's own raw progress, kept apart from the cross-source `seen`: a server that
    // ignores `page`/`skip` and re-serves earlier content must not look like a second source's
    // legitimate duplicate (review M-1).
    let mut source_seen: std::collections::HashSet<String> = std::collections::HashSet::new();
    // Whether the page just processed declared, via `page.total`/`@odata.count`, that another page
    // should exist — so an unexpectedly empty next page can be named rather than swallowed (M-2).
    let mut expected_more = false;
    loop {
        // Checked before the budget: at the cap, this page was never going to be fetched
        // regardless of the clock, so the cap message — not a budget one — is the true reason
        // (review M-3).
        if page > MAX_PAGES {
            ctx.warnings.push(format!(
                "{}: stopped after {MAX_PAGES} pages; later events wait for the window to move",
                source.name
            ));
            return;
        }
        match budget_stop(page, source_start, ctx.elapsed, ctx.limits) {
            Some(true) if page == 1 => {
                ctx.warnings.push(format!("{}: skipped (run time budget)", source.name));
                return;
            }
            Some(true) => {
                ctx.warnings.push(format!(
                    "{}: stopped at page {page} (run time budget); kept {kept} events",
                    source.name
                ));
                return;
            }
            Some(false) => {
                ctx.warnings.push(format!(
                    "{}: stopped at page {page} (time budget); kept {kept} events",
                    source.name
                ));
                return;
            }
            None => {}
        }
        let url = build_url(page);
        let text = match (ctx.fetch)(&url) {
            Ok(text) => text,
            Err(err) => {
                if page == 1 {
                    ctx.warnings.push(format!("{}: fetch failed ({err})", source.name));
                } else {
                    ctx.warnings.push(format!(
                        "{}: page {page} failed ({err}); kept {kept} events",
                        source.name
                    ));
                }
                return;
            }
        };
        let payload: Json = match serde_json::from_str(&text) {
            Ok(payload) => payload,
            Err(err) => {
                if page == 1 {
                    ctx.warnings.push(format!("{}: parse failed ({err})", source.name));
                } else {
                    ctx.warnings.push(format!(
                        "{}: page {page} failed ({err}); kept {kept} events",
                        source.name
                    ));
                }
                return;
            }
        };
        let (parsed, source_warnings) = parse(&payload, &source.name, ctx.tz);
        for w in source_warnings {
            ctx.warnings.push(format!("{}: {w}", source.name));
        }
        let info = should_continue(&payload, page);

        let mut source_progressed = false;
        for event in &parsed {
            if source_seen.insert(event.uid.clone()) {
                source_progressed = true;
            }
        }
        let had_parsed = !parsed.is_empty();
        for event in parsed {
            if ctx.seen.insert(event.uid.clone()) {
                ctx.by_uid.push(event);
                kept += 1;
            }
        }

        if info.raw_empty {
            if expected_more {
                ctx.warnings.push(format!(
                    "{}: page {page} was empty; kept {kept} events",
                    source.name
                ));
            }
            return;
        }
        if had_parsed && !source_progressed {
            ctx.warnings.push(format!(
                "{}: page {page} repeated an earlier page; kept {kept} events",
                source.name
            ));
            return;
        }
        expected_more = info.keep_going && info.declared;
        if !info.keep_going {
            return;
        }
        page += 1;
    }
}

/// One un-paged source (`ics`, `html`): fetched once, exactly as before.
fn fetch_once(
    source: &EventsSource,
    fetch: &dyn Fn(&str) -> Result<String, String>,
    warnings: &mut Vec<String>,
    seen: &mut std::collections::HashSet<String>,
    by_uid: &mut Vec<DiscoveredEvent>,
    parse: impl FnOnce(&str) -> (Vec<DiscoveredEvent>, Vec<String>),
) {
    let text = match fetch(&source.url) {
        Ok(text) => text,
        Err(err) => {
            warnings.push(format!("{}: fetch failed ({err})", source.name));
            return;
        }
    };
    let (parsed, source_warnings) = parse(&text);
    for w in source_warnings {
        warnings.push(format!("{}: {w}", source.name));
    }
    for event in parsed {
        if seen.insert(event.uid.clone()) {
            by_uid.push(event);
        }
    }
}

/// Fetch every enabled source. One dead feed must never fail a run.
pub fn load_discovered_events(
    vault: &Path,
    fetcher: Option<&dyn Fn(&str) -> Result<String, String>>,
) -> (Vec<DiscoveredEvent>, Vec<String>) {
    let start = Instant::now();
    load_discovered_events_at(vault, fetcher, Timestamp::now(), PagingLimits::DEFAULT, &move || {
        start.elapsed()
    })
}

/// F9: the paged, budgeted, clock-injected form. `now` builds Engage's `endsAfter`; `elapsed` is
/// wall-clock time since the call began (production: a closure over one [`Instant`]; tests: a
/// `Cell<Duration>` the fake fetcher advances). `events::judge_roster` (F10) calls this directly,
/// with its own budget capping `per_run` and its own `Instant` feeding `elapsed`.
pub fn load_discovered_events_at(
    vault: &Path,
    fetcher: Option<&dyn Fn(&str) -> Result<String, String>>,
    now: Timestamp,
    limits: PagingLimits,
    elapsed: &dyn Fn() -> Duration,
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

    let days = config.roster_window_days.clamp(1, 365);
    let ends_after = now.to_zoned(TimeZone::UTC).strftime("%Y-%m-%dT%H:%M:%SZ").to_string();

    // First uid wins, and insertion order is preserved for the tie — the sort below is on
    // `(start, uid)`, so cross-source duplicates collapse before ordering.
    let mut by_uid: Vec<DiscoveredEvent> = Vec::new();
    let mut seen: std::collections::HashSet<String> = std::collections::HashSet::new();

    for source in &config.sources {
        if !source.enabled {
            continue;
        }
        // Every kind answers to the run budget, not just the paged ones (review I-1): a JSON
        // source that burns the whole `per_run` budget must not leave a later `ics`/`html` source
        // still to be fetched at up to 150s of its own — past the 330s worst case `PagingLimits`
        // is sized against.
        if elapsed() >= limits.per_run {
            warnings.push(format!("{}: skipped (run time budget)", source.name));
            continue;
        }
        match source.kind.as_str() {
            "localist" => {
                let base = source.url.clone();
                let mut ctx = PagingCtx {
                    tz: &tz,
                    fetch,
                    limits: &limits,
                    elapsed,
                    warnings: &mut warnings,
                    seen: &mut seen,
                    by_uid: &mut by_uid,
                };
                page_json_source(
                    source,
                    &mut ctx,
                    |page| {
                        paged_url(
                            &base,
                            &LOCALIST_KEYS,
                            &[
                                ("days", days.to_string()),
                                ("pp", PAGE_SIZE.to_string()),
                                ("page", page.to_string()),
                            ],
                        )
                    },
                    |payload, page| {
                        let raw_count = payload
                            .get("events")
                            .and_then(|v| v.as_array())
                            .map(|a| a.len())
                            .unwrap_or(0);
                        if raw_count == 0 {
                            return PageInfo { keep_going: false, declared: false, raw_empty: true };
                        }
                        match json_i64(payload.get("page").and_then(|p| p.get("total"))) {
                            Some(total) => PageInfo {
                                keep_going: (page as i64) < total,
                                declared: true,
                                raw_empty: false,
                            },
                            None => PageInfo {
                                keep_going: raw_count >= PAGE_SIZE,
                                declared: false,
                                raw_empty: false,
                            },
                        }
                    },
                    parse_localist,
                );
            }
            "engage" => {
                let base = source.url.clone();
                let ends_after = ends_after.clone();
                let mut ctx = PagingCtx {
                    tz: &tz,
                    fetch,
                    limits: &limits,
                    elapsed,
                    warnings: &mut warnings,
                    seen: &mut seen,
                    by_uid: &mut by_uid,
                };
                page_json_source(
                    source,
                    &mut ctx,
                    move |page| {
                        let skip = (page - 1) * PAGE_SIZE;
                        paged_url(
                            &base,
                            &ENGAGE_KEYS,
                            &[
                                ("endsAfter", ends_after.clone()),
                                ("orderByField", "endsOn".to_string()),
                                ("orderByDirection", "ascending".to_string()),
                                ("status", "Approved".to_string()),
                                ("take", PAGE_SIZE.to_string()),
                                ("skip", skip.to_string()),
                            ],
                        )
                    },
                    |payload, page| {
                        let raw_count = payload
                            .get("value")
                            .and_then(|v| v.as_array())
                            .map(|a| a.len())
                            .unwrap_or(0);
                        if raw_count == 0 {
                            return PageInfo { keep_going: false, declared: false, raw_empty: true };
                        }
                        let skip = (page - 1) * PAGE_SIZE;
                        match json_i64(payload.get("@odata.count")) {
                            Some(count) => PageInfo {
                                keep_going: ((skip + PAGE_SIZE) as i64) < count,
                                declared: true,
                                raw_empty: false,
                            },
                            None => PageInfo {
                                keep_going: raw_count >= PAGE_SIZE,
                                declared: false,
                                raw_empty: false,
                            },
                        }
                    },
                    parse_engage,
                );
            }
            "ics" => {
                fetch_once(source, fetch, &mut warnings, &mut seen, &mut by_uid, |text| {
                    parse_ics_events(text, &source.name, &tz)
                });
            }
            _ => {
                fetch_once(source, fetch, &mut warnings, &mut seen, &mut by_uid, |text| {
                    parse_html_events(text, &source.name, &source.url)
                });
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

    // --- F9: paging Localist and Engage ---------------------------------------------------

    use std::cell::Cell;

    fn fixed_now() -> Timestamp {
        "2026-09-23T12:00:00Z".parse().unwrap()
    }

    fn default_limits() -> PagingLimits {
        PagingLimits::DEFAULT
    }

    fn zero_clock() -> Duration {
        Duration::ZERO
    }

    fn localist_event(id: i64, title: &str) -> Json {
        localist_event_at(id, title, "2026-09-02T12:00:00-05:00")
    }

    fn localist_event_at(id: i64, title: &str, start: &str) -> Json {
        json!({"event": {
            "id": id,
            "title": title,
            "event_instances": [
                {"event_instance": {"start": start}}
            ]
        }})
    }

    fn localist_payload(events: Vec<Json>, total: Option<i64>) -> Json {
        let mut payload = json!({"events": events});
        if let Some(t) = total {
            payload["page"] = json!({"total": t});
        }
        payload
    }

    fn localist_page_of(offset: usize, count: usize) -> Json {
        let events: Vec<Json> = (0..count)
            .map(|i| localist_event((offset + i) as i64, "Bulk"))
            .collect();
        localist_payload(events, None)
    }

    fn engage_payload(id: i64, count_total: i64) -> Json {
        json!({
            "@odata.count": count_total,
            "value": [{
                "id": id,
                "name": format!("Event {id}"),
                "startsOn": "2026-08-20T13:30:00+00:00",
                "endsOn": "2026-08-20T19:00:00+00:00",
                "status": "Approved"
            }]
        })
    }

    fn localist_vault(name: &str, url: &str) -> PathBuf {
        vault_with(
            name,
            &format!("sources:\n  - name: campus\n    type: localist\n    url: {url}\n    enabled: true\n"),
        )
    }

    #[test]
    fn localist_pages_until_page_total() {
        let vault = localist_vault("localist3", "http://x");
        // Page 1's event starts LATEST and page 3's EARLIEST, so a correct final order (earliest
        // first) proves `sort_events` actually re-sorted by start rather than merely preserving
        // fetch/uid order (review M-6).
        let starts = ["2026-09-05T12:00:00-05:00", "2026-09-03T12:00:00-05:00", "2026-09-01T12:00:00-05:00"];
        let urls = RefCell::new(Vec::new());
        let fetch = |url: &str| {
            let mut u = urls.borrow_mut();
            u.push(url.to_string());
            let n = u.len() as i64;
            Ok(localist_payload(
                vec![localist_event_at(n, &format!("Event {n}"), starts[(n - 1) as usize])],
                Some(3),
            )
            .to_string())
        };
        let (events, warnings) =
            load_discovered_events_at(&vault, Some(&fetch), fixed_now(), default_limits(), &zero_clock);
        assert_eq!(
            urls.into_inner(),
            vec![
                "http://x?days=90&pp=100&page=1",
                "http://x?days=90&pp=100&page=2",
                "http://x?days=90&pp=100&page=3",
            ]
        );
        assert_eq!(
            events.iter().map(|e| e.title.as_str()).collect::<Vec<_>>(),
            vec!["Event 3", "Event 2", "Event 1"]
        );
        assert!(warnings.is_empty(), "{warnings:?}");
    }

    #[test]
    fn engage_pages_by_skip_until_the_count() {
        let vault = vault_with(
            "engage3",
            "sources:\n  - name: clubs\n    type: engage\n    url: http://x\n    enabled: true\n",
        );
        let urls = RefCell::new(Vec::new());
        let fetch = |url: &str| {
            let mut u = urls.borrow_mut();
            u.push(url.to_string());
            let n = u.len() as i64;
            Ok(engage_payload(n, 250).to_string())
        };
        let (_, warnings) =
            load_discovered_events_at(&vault, Some(&fetch), fixed_now(), default_limits(), &zero_clock);
        assert!(warnings.is_empty(), "{warnings:?}");
        assert_eq!(
            urls.into_inner(),
            vec![
                "http://x?endsAfter=2026-09-23T12:00:00Z&orderByField=endsOn&orderByDirection=ascending&status=Approved&take=100&skip=0",
                "http://x?endsAfter=2026-09-23T12:00:00Z&orderByField=endsOn&orderByDirection=ascending&status=Approved&take=100&skip=100",
                "http://x?endsAfter=2026-09-23T12:00:00Z&orderByField=endsOn&orderByDirection=ascending&status=Approved&take=100&skip=200",
            ]
        );
    }

    #[test]
    fn paging_stops_at_the_page_cap_and_warns_once() {
        let vault = localist_vault("localistcap", "http://x");
        let urls = RefCell::new(Vec::new());
        let fetch = |url: &str| {
            let mut u = urls.borrow_mut();
            u.push(url.to_string());
            let n = u.len() as i64;
            // A total far past the cap, so only the hard cap can ever end this loop.
            Ok(localist_payload(vec![localist_event(n, "Event")], Some(999)).to_string())
        };
        let (_, warnings) =
            load_discovered_events_at(&vault, Some(&fetch), fixed_now(), default_limits(), &zero_clock);
        assert_eq!(urls.borrow().len(), 20);
        assert_eq!(
            warnings,
            vec!["campus: stopped after 20 pages; later events wait for the window to move".to_string()]
        );
    }

    #[test]
    fn an_existing_query_is_kept_and_the_engines_keys_replaced() {
        let vault = localist_vault("localistquery", "http://x?group=x&pp=10");
        let urls = RefCell::new(Vec::new());
        let fetch = |url: &str| {
            urls.borrow_mut().push(url.to_string());
            Ok(localist_payload(vec![localist_event(1, "Solo")], None).to_string())
        };
        let (events, warnings) =
            load_discovered_events_at(&vault, Some(&fetch), fixed_now(), default_limits(), &zero_clock);
        assert_eq!(urls.into_inner(), vec!["http://x?group=x&days=90&pp=100&page=1"]);
        assert_eq!(events.len(), 1);
        assert!(warnings.is_empty(), "{warnings:?}");
    }

    #[test]
    fn a_failed_later_page_keeps_earlier_pages_and_warns() {
        let vault = localist_vault("localistfail2", "http://x");
        let calls = Cell::new(0usize);
        let fetch = |_: &str| {
            let n = calls.get() + 1;
            calls.set(n);
            if n == 1 {
                Ok(localist_payload(vec![localist_event(1, "Keep")], Some(2)).to_string())
            } else {
                Err("boom".to_string())
            }
        };
        let (events, warnings) =
            load_discovered_events_at(&vault, Some(&fetch), fixed_now(), default_limits(), &zero_clock);
        assert_eq!(events.len(), 1);
        assert_eq!(events[0].title, "Keep");
        assert_eq!(warnings, vec!["campus: page 2 failed (boom); kept 1 events".to_string()]);
    }

    #[test]
    fn a_first_page_failure_warns_exactly_as_before() {
        let vault = localist_vault("localistfail1", "http://x");
        let urls = RefCell::new(Vec::new());
        let fetch = |url: &str| {
            urls.borrow_mut().push(url.to_string());
            Err("boom".to_string())
        };
        let (events, warnings) =
            load_discovered_events_at(&vault, Some(&fetch), fixed_now(), default_limits(), &zero_clock);
        assert!(events.is_empty());
        assert_eq!(warnings, vec!["campus: fetch failed (boom)".to_string()]);
        assert_eq!(urls.borrow().len(), 1);
    }

    #[test]
    fn a_short_page_without_a_total_ends_paging() {
        let vault = localist_vault("localistshort", "http://x");
        let urls = RefCell::new(Vec::new());
        let fetch = |url: &str| {
            urls.borrow_mut().push(url.to_string());
            Ok(localist_payload(vec![localist_event(1, "Solo")], None).to_string())
        };
        let (events, warnings) =
            load_discovered_events_at(&vault, Some(&fetch), fixed_now(), default_limits(), &zero_clock);
        assert_eq!(urls.borrow().len(), 1);
        assert_eq!(events.len(), 1);
        assert!(warnings.is_empty(), "{warnings:?}");
    }

    #[test]
    fn a_duplicate_uid_across_pages_collapses_first_wins() {
        // Page 2 repeats page 1's id (5, "Second") but also adds a genuinely new one (6), so this
        // is dedup — not the whole-page repeat review M-1 detects and stops on separately (see
        // `a_repeated_page_stops_paging_and_names_it`).
        let vault = localist_vault("localistdupe", "http://x");
        let calls = Cell::new(0usize);
        let fetch = |_: &str| {
            let n = calls.get() + 1;
            calls.set(n);
            let body = if n == 1 {
                localist_payload(vec![localist_event(5, "First")], Some(2))
            } else {
                localist_payload(
                    vec![localist_event(5, "Second"), localist_event(6, "New")],
                    Some(2),
                )
            };
            Ok(body.to_string())
        };
        let (events, warnings) =
            load_discovered_events_at(&vault, Some(&fetch), fixed_now(), default_limits(), &zero_clock);
        assert_eq!(events.len(), 2);
        assert_eq!(events.iter().find(|e| e.uid == "localist:5").unwrap().title, "First");
        assert!(events.iter().any(|e| e.uid == "localist:6"));
        assert!(warnings.is_empty(), "{warnings:?}");
    }

    #[test]
    fn a_repeated_page_stops_paging_and_names_it() {
        // A server ignoring `page` and re-serving the same content (review M-1): every page
        // returns the identical event, so page 2 adds nothing new to this source.
        let vault = localist_vault("localistrepeat", "http://x");
        let calls = Cell::new(0usize);
        let fetch = |_: &str| {
            calls.set(calls.get() + 1);
            Ok(localist_payload(vec![localist_event(7, "Same")], Some(999)).to_string())
        };
        let (events, warnings) =
            load_discovered_events_at(&vault, Some(&fetch), fixed_now(), default_limits(), &zero_clock);
        assert_eq!(calls.get(), 2);
        assert_eq!(events.len(), 1);
        assert_eq!(
            warnings,
            vec!["campus: page 2 repeated an earlier page; kept 1 events".to_string()]
        );
    }

    #[test]
    fn an_unexpectedly_empty_page_after_a_declared_total_warns() {
        // Page 1 declares `page.total: 3`; page 2 is shapeless (`{}`) rather than a genuine next
        // page — the promise page 1 made means this is worth naming (review M-2).
        let vault = localist_vault("localistempty", "http://x");
        let calls = Cell::new(0usize);
        let fetch = |_: &str| {
            let n = calls.get() + 1;
            calls.set(n);
            if n == 1 {
                Ok(localist_payload(vec![localist_event(1, "Keep")], Some(3)).to_string())
            } else {
                Ok(json!({}).to_string())
            }
        };
        let (events, warnings) =
            load_discovered_events_at(&vault, Some(&fetch), fixed_now(), default_limits(), &zero_clock);
        assert_eq!(calls.get(), 2);
        assert_eq!(events.len(), 1);
        // `parse_localist` itself also warns on the shapeless page (`{}` has no "events" array) —
        // that warning is real and expected; the M-2 warning about the broken promise is on top.
        assert_eq!(
            warnings,
            vec![
                "campus: unexpected Localist payload shape".to_string(),
                "campus: page 2 was empty; kept 1 events".to_string(),
            ]
        );
    }

    /// Not one of the brief's 12 — a genuinely empty page with no prior declared total ends
    /// silently, same as `a_short_page_without_a_total_ends_paging`. Confirms M-2's fix only fires
    /// when a prior page promised more.
    #[test]
    fn a_genuinely_empty_page_with_no_declared_total_stays_silent() {
        let vault = localist_vault("localistemptysilent", "http://x");
        let urls = RefCell::new(Vec::new());
        let fetch = |url: &str| {
            urls.borrow_mut().push(url.to_string());
            Ok(json!({"events": []}).to_string())
        };
        let (events, warnings) =
            load_discovered_events_at(&vault, Some(&fetch), fixed_now(), default_limits(), &zero_clock);
        assert_eq!(urls.borrow().len(), 1);
        assert!(events.is_empty());
        assert!(warnings.is_empty(), "{warnings:?}");
    }

    #[test]
    fn the_page_cap_wins_over_a_simultaneous_run_time_budget() {
        // Tuned so the run budget and the page cap become true at the SAME check (before the 21st
        // fetch): if the budget were checked first, the message would wrongly read as a run-budget
        // stop instead of the cap (review M-3).
        let vault = localist_vault("localistcapbudget", "http://x");
        let clock = Cell::new(Duration::ZERO);
        let calls = Cell::new(0usize);
        let fetch = |_: &str| {
            let n = calls.get() + 1;
            calls.set(n);
            clock.set(clock.get() + Duration::from_secs(10));
            Ok(localist_payload(vec![localist_event(n as i64, "Event")], Some(999)).to_string())
        };
        let elapsed = || clock.get();
        // 19 calls -> 190s < 195s (every one of pages 2-20's checks passes); 20 calls -> 200s >=
        // 195s, true at exactly the same check where page 21 > MAX_PAGES is also true.
        let limits =
            PagingLimits { per_source: Duration::from_secs(100_000), per_run: Duration::from_secs(195) };
        let (_, warnings) =
            load_discovered_events_at(&vault, Some(&fetch), fixed_now(), limits, &elapsed);
        assert_eq!(calls.get(), 20);
        assert_eq!(
            warnings,
            vec!["campus: stopped after 20 pages; later events wait for the window to move".to_string()]
        );
    }

    #[test]
    fn a_mid_source_run_time_budget_stop_is_named() {
        // No test in round 1 covered the mid-source "(run time budget)" shape, as opposed to the
        // per-source "(time budget)" shape or the pre-page-1 "skipped" shape (review M-6).
        let vault = localist_vault("localistmidrun", "http://x");
        let clock = Cell::new(Duration::ZERO);
        let calls = Cell::new(0usize);
        let fetch = |_: &str| {
            let n = calls.get() + 1;
            calls.set(n);
            clock.set(clock.get() + Duration::from_secs(70));
            Ok(localist_payload(vec![localist_event(n as i64, "Event")], Some(999)).to_string())
        };
        let elapsed = || clock.get();
        // per_source is large enough to never fire; per_run (100s) is crossed by the check before
        // page 3 (elapsed 140s after two 70s fetches), which is `page > 1`.
        let limits =
            PagingLimits { per_source: Duration::from_secs(100_000), per_run: Duration::from_secs(100) };
        let (events, warnings) =
            load_discovered_events_at(&vault, Some(&fetch), fixed_now(), limits, &elapsed);
        assert_eq!(calls.get(), 2);
        assert_eq!(events.len(), 2);
        assert_eq!(
            warnings,
            vec!["campus: stopped at page 3 (run time budget); kept 2 events".to_string()]
        );
    }

    #[test]
    fn a_later_pages_json_parse_failure_keeps_earlier_pages_and_warns() {
        // Round 1 only covered a later page's fetch (transport) failure; this covers a later
        // page's JSON parse failure (review M-6).
        let vault = localist_vault("localistbadjson", "http://x");
        let calls = Cell::new(0usize);
        let fetch = |_: &str| {
            let n = calls.get() + 1;
            calls.set(n);
            if n == 1 {
                Ok(localist_payload(vec![localist_event(1, "Keep")], Some(2)).to_string())
            } else {
                Ok("{ not json".to_string())
            }
        };
        let (events, warnings) =
            load_discovered_events_at(&vault, Some(&fetch), fixed_now(), default_limits(), &zero_clock);
        assert_eq!(events.len(), 1);
        assert_eq!(warnings.len(), 1);
        assert!(warnings[0].starts_with("campus: page 2 failed ("), "{warnings:?}");
        assert!(warnings[0].ends_with("; kept 1 events"), "{warnings:?}");
    }

    #[test]
    fn the_run_time_budget_skips_every_kind_not_just_the_paged_ones() {
        // Review I-1: an `ics` source after a JSON source has burned the run budget must never be
        // fetched either — the 330s worst case `PagingLimits` is sized against assumed this.
        let vault = vault_with(
            "runbudgetics",
            "sources:\n  - name: first\n    type: localist\n    url: http://a\n    enabled: true\n  - name: second\n    type: ics\n    url: http://b\n    enabled: true\n",
        );
        let clock = Cell::new(Duration::ZERO);
        let urls = RefCell::new(Vec::new());
        let fetch = |url: &str| {
            urls.borrow_mut().push(url.to_string());
            clock.set(Duration::from_secs(200));
            Ok(localist_payload(vec![localist_event(1, "First source event")], None).to_string())
        };
        let elapsed = || clock.get();
        let limits =
            PagingLimits { per_source: Duration::from_secs(90), per_run: Duration::from_secs(180) };
        let (events, warnings) =
            load_discovered_events_at(&vault, Some(&fetch), fixed_now(), limits, &elapsed);
        assert_eq!(urls.borrow().len(), 1, "the ics source must never be fetched");
        assert_eq!(events.len(), 1);
        assert_eq!(warnings, vec!["second: skipped (run time budget)".to_string()]);
    }

    #[test]
    fn paged_url_preserves_a_fragment() {
        // Review M-4: a fragment must not swallow the appended query.
        assert_eq!(
            paged_url("http://x/feed#top", &LOCALIST_KEYS, &[("days", "90".to_string())]),
            "http://x/feed?days=90#top"
        );
        assert_eq!(
            paged_url(
                "http://x/feed?group=y&pp=10#top",
                &LOCALIST_KEYS,
                &[("days", "90".to_string()), ("pp", "100".to_string()), ("page", "1".to_string())]
            ),
            "http://x/feed?group=y&days=90&pp=100&page=1#top"
        );
    }

    #[test]
    fn ics_and_html_sources_are_fetched_once_with_the_url_unchanged() {
        let vault = vault_with(
            "icshtml",
            "sources:\n  - name: blount\n    type: ics\n    url: http://a?x=1\n    enabled: true\n  - name: eng\n    type: html\n    url: http://b?y=2\n    enabled: true\n",
        );
        let urls = RefCell::new(Vec::new());
        let fetch = |url: &str| {
            urls.borrow_mut().push(url.to_string());
            if url.starts_with("http://a") {
                Ok(ICS.to_string())
            } else {
                Ok("<html></html>".to_string())
            }
        };
        let (events, warnings) =
            load_discovered_events_at(&vault, Some(&fetch), fixed_now(), default_limits(), &zero_clock);
        assert_eq!(urls.into_inner(), vec!["http://a?x=1", "http://b?y=2"]);
        assert_eq!(titles(&events), vec!["Blount Thing"]);
        assert!(warnings.is_empty(), "{warnings:?}");
    }

    #[test]
    fn paging_stops_at_the_per_source_time_budget_and_keeps_what_it_has() {
        let vault = localist_vault("localistbudget", "http://x");
        let clock = Cell::new(Duration::ZERO);
        let calls = Cell::new(0usize);
        let fetch = |_: &str| {
            let n = calls.get();
            calls.set(n + 1);
            let body = localist_page_of(n * 1000, 100).to_string();
            clock.set(clock.get() + Duration::from_secs(40));
            Ok(body)
        };
        let elapsed = || clock.get();
        let limits = PagingLimits { per_source: Duration::from_secs(90), per_run: Duration::from_secs(180) };
        let (events, warnings) =
            load_discovered_events_at(&vault, Some(&fetch), fixed_now(), limits, &elapsed);
        assert_eq!(calls.get(), 3);
        assert_eq!(events.len(), 300);
        assert_eq!(
            warnings,
            vec!["campus: stopped at page 4 (time budget); kept 300 events".to_string()]
        );
    }

    #[test]
    fn the_run_time_budget_skips_a_later_source_and_names_it() {
        let vault = vault_with(
            "runbudget",
            "sources:\n  - name: first\n    type: localist\n    url: http://a\n    enabled: true\n  - name: second\n    type: localist\n    url: http://b\n    enabled: true\n",
        );
        let clock = Cell::new(Duration::ZERO);
        let urls = RefCell::new(Vec::new());
        let fetch = |url: &str| {
            urls.borrow_mut().push(url.to_string());
            clock.set(Duration::from_secs(200));
            Ok(localist_payload(vec![localist_event(1, "First source event")], None).to_string())
        };
        let elapsed = || clock.get();
        let limits = PagingLimits { per_source: Duration::from_secs(90), per_run: Duration::from_secs(180) };
        let (events, warnings) =
            load_discovered_events_at(&vault, Some(&fetch), fixed_now(), limits, &elapsed);
        assert_eq!(urls.borrow().len(), 1);
        assert_eq!(events.len(), 1);
        assert_eq!(warnings, vec!["second: skipped (run time budget)".to_string()]);
    }

    /// Pins the constants against the `scheduler::CHILD_TIMEOUT` arithmetic (F9, review I-3):
    /// worst case is the budget plus one page already in flight — `180s + 150s = 330s`, 5.5
    /// minutes — leaving `rank` about 14.5 of its 20-minute step budget for the calendar fetch and
    /// everything else, the same order of headroom `enrich::BATCH_BUDGET` leaves `judge` (17 of 20).
    #[test]
    fn the_default_limits_are_90_and_180_seconds() {
        assert_eq!(PagingLimits::DEFAULT.per_source, Duration::from_secs(90));
        assert_eq!(PagingLimits::DEFAULT.per_run, Duration::from_secs(180));
    }
}
