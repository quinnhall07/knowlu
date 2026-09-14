//! zyBooks integration — parser and network surface. Port of `engine/zybooks.py`.
//!
//! Wave 6 of the Rust port. Like `ingest` and `calfeed`, this module has two surfaces: a pure,
//! deterministic parser ([`parse_assignments`], [`category_of`]) that turns a zyBooks JSON payload
//! into [`Assignment`]s, and a network layer that needs live credentials. The parser is pinned
//! against a real captured payload — including a byte-for-byte reference of what the **Python**
//! parser produces from it; the network layer is verified at rollout, not in CI.
//!
//! # Three things this module exists to get right
//!
//! - **A `User-Agent` on every request.** zyBooks 403s a request without one, signin included, so
//!   the symptom is a *total outage*, not a degradation — and it is the most likely cause of a
//!   future sudden failure (spec §7.2). No offline test can observe a header on the wire, so
//!   [`USER_AGENT`] is asserted by a test rather than merely set.
//! - **No secret ever reaches a warning line.** `state/runner-log.md` is committed and pushed
//!   within minutes of being written. [`scrub`] runs over every error message, and [`HttpRequest`]
//!   has a hand-written `Debug` — a derived one would put the production password into any `{:?}`
//!   that ever touched a request.
//! - **A 200 with `success: false` is a dead session, not an empty semester.** [`require_success`]
//!   is what stops a revoked token reading as "0 assignments parsed".

use std::fmt;
use std::time::Duration;

use jiff::civil::DateTime;
use jiff::tz::TimeZone;
use serde_json::Value as Json;
use serde_yaml_ng::{Mapping, Value as Yaml};

use crate::coursework::{Assignment, SourceError};
use crate::ingest::{format_due, slugify, Due};
use crate::pystr::{json_int, json_str, json_truthy, yaml_str, yaml_truthy};

pub const SIGNIN_URL: &str = "https://zyserver.zybooks.com/v1/signin";
pub const BASE: &str = "https://zyserver.zybooks.com/v1";

/// Not shared with `vhl`: these modules are siblings and neither should depend on the other.
///
/// Without this header Python's `urllib` sends `Python-urllib/3.14` and zyBooks returns a hard
/// **403 on every request, signin included** — verified live. An honest identifying UA is
/// sufficient; no browser impersonation is needed.
pub const USER_AGENT: &str = "Mozilla/5.0 (Windows NT 10.0; Win64; x64) Knowlu";

pub const DEFAULT_IMPORTANCE: i64 = 3;
pub const DEFAULT_FLOOR_HOURS: f64 = 0.25;

/// Python's `round(x, 2)` — half to EVEN, matching `ranking::round2`.
fn round2(x: f64) -> f64 {
    (x * 100.0).round_ties_even() / 100.0
}

// ---------------------------------------------------------------------------------------------
// the parser
// ---------------------------------------------------------------------------------------------

/// First configured prefix the (lowercased) title starts with, or `None`.
///
/// Iteration order is the config's own order, which `serde_yaml_ng::Mapping` preserves for the
/// same reason Python's `dict` does. It only becomes observable when two prefixes overlap, and
/// then both engines pick the one written first.
pub fn category_of(title: &str, categories: &Mapping) -> Option<String> {
    let lowered = title.to_lowercase();
    for (prefix, kind) in categories {
        if lowered.starts_with(&yaml_str(prefix).to_lowercase()) {
            return Some(yaml_str(kind));
        }
    }
    None
}

/// `datetime.strptime(raw, "%Y-%m-%dT%H:%M:%SZ")` in UTC, converted to `tz`, then made naive.
///
/// Getting this wrong shifts every deadline by a day, which is the whole point of the system:
/// `2026-08-27T04:59:00Z` is `2026-08-26 23:59` in Central.
fn due_central(raw: &Json, tz: &TimeZone) -> Result<DateTime, String> {
    let text = match raw {
        Json::String(s) => s.as_str(),
        // Python: `strptime` raises TypeError on a non-str. The message differs; the control flow
        // does not — either way the item is skipped with an "unreadable due date" warning.
        other => return Err(format!("strptime() argument 1 must be str, not {other}")),
    };
    let naive = DateTime::strptime("%Y-%m-%dT%H:%M:%SZ", text)
        .map_err(|_| format!("time data '{text}' does not match format '%Y-%m-%dT%H:%M:%SZ'"))?;
    let utc = naive.to_zoned(TimeZone::UTC).map_err(|err| format!("{err}"))?;
    Ok(utc.with_time_zone(tz.clone()).datetime())
}

/// `cfg.get(key) or {}`, refusing a truthy non-mapping the way Python's next `.items()` /
/// `.get()` call would.
fn mapping_field(map: &Mapping, key: &str) -> Result<Mapping, SourceError> {
    match crate::yaml::get(map, key) {
        Some(value) if yaml_truthy(value) => match value {
            Yaml::Mapping(inner) => Ok(inner.clone()),
            _ => Err(SourceError::Failed(format!(
                "config key {key:?} is not a mapping"
            ))),
        },
        _ => Ok(Mapping::new()),
    }
}

/// `float(x)`. Python raises on `None` and on a non-numeric string; so does this.
fn python_float(value: &Yaml, what: &str) -> Result<f64, SourceError> {
    crate::yaml::f64_of(value).ok_or_else(|| SourceError::Failed(format!("{what} is not a number")))
}

/// `int(x)` over a YAML value.
fn python_int_yaml(value: &Yaml, what: &str) -> Result<i64, SourceError> {
    crate::yaml::i64_of(value).ok_or_else(|| SourceError::Failed(format!("{what} is not an int")))
}

/// `table.get(kind, default)` — where a **present** key wins even when its value is null, and
/// `int(None)` / `float(None)` then raises. `kind = None` never matches a string key.
fn table_lookup<'a>(table: &'a Mapping, kind: Option<&str>) -> Option<&'a Yaml> {
    kind.and_then(|k| crate::yaml::get(table, k))
}

/// `payload.get(key) or []`, refusing a truthy non-array the way Python's iteration would fail.
fn array_field(map: &serde_json::Map<String, Json>, key: &str) -> Result<Vec<Json>, SourceError> {
    match map.get(key) {
        Some(value) if json_truthy(value) => match value {
            Json::Array(items) => Ok(items.clone()),
            _ => Err(SourceError::Failed(format!("{key} is not a list"))),
        },
        _ => Ok(Vec::new()),
    }
}

fn as_object<'a>(
    value: &'a Json,
    what: &str,
) -> Result<&'a serde_json::Map<String, Json>, SourceError> {
    value
        .as_object()
        .ok_or_else(|| SourceError::Failed(format!("{what} is not an object")))
}

/// Turn one zyBooks assignment payload into [`Assignment`]s. Pure: every field comes from the
/// payload plus config, which is what keeps this inside the engine's no-inference rule.
///
/// **This function is the reference the server-side port is measured against** (cloud design
/// §4.3): `cloud/supabase/functions/ingest-coursework/parse_zybooks.ts` is a faithful TypeScript
/// port of it, and both are gated by the same frozen `tests/fixtures/zybooks-parsed-reference.json`.
/// From C2 on, production parses on the server and this half is what the port is checked against —
/// change one and the other's test fails, which is the point.
pub fn parse_assignments(
    payload: &Json,
    course_slug: &str,
    course_label: &str,
    cfg: &Mapping,
    tz: &TimeZone,
    warnings: &mut Vec<String>,
) -> Result<Vec<Assignment>, SourceError> {
    require_success(payload, "assignment payload")?;
    // slugify, not the raw config value: the slug becomes `tasks/<slug>.md`, and a config typo
    // carrying a path separator would otherwise write outside `tasks/`.
    let slug_prefix = slugify(course_slug);
    let categories = mapping_field(cfg, "categories")?;
    let effort_cfg = mapping_field(cfg, "effort")?;
    let per_section = match crate::yaml::get(&effort_cfg, "minutes_per_section") {
        Some(value) => python_float(value, "minutes_per_section")?,
        None => 6.0,
    };
    let floors = mapping_field(&effort_cfg, "floors")?;
    let importance_table = mapping_field(cfg, "importance")?;

    let payload_map = match payload.as_object() {
        Some(map) => map,
        // Python would raise AttributeError on `.get`; either way nothing parses.
        None => return Err(SourceError::Failed("payload is not an object".to_string())),
    };

    let mut out: Vec<Assignment> = Vec::new();
    for raw in array_field(payload_map, "assignments")? {
        let raw = as_object(&raw, "assignment")?;

        let title = match raw.get("title") {
            Some(value) if json_truthy(value) => json_str(value),
            _ => String::new(),
        };
        let title = crate::pystr::strip(&title).to_string();
        if title.is_empty() {
            warnings.push("assignment with no title skipped".to_string());
            continue;
        }
        // `is False`, not falsiness: a missing or null `visible` leaves the assignment visible.
        if raw.get("visible") == Some(&Json::Bool(false)) {
            continue;
        }

        let dues = array_field(raw, "due_dates")?;
        let stamps: Vec<&Json> = dues
            .iter()
            .filter_map(|d| d.as_object())
            .filter_map(|d| d.get("date"))
            .filter(|d| json_truthy(d))
            .collect();
        if stamps.is_empty() {
            warnings.push(format!("{title}: no due date; skipped"));
            continue;
        }

        // zyBooks uses a plural `due_dates[]` for extensions and per-section overrides, and
        // documents no ordering. Earliest is the only defensible pick: it is the deadline that
        // binds first, and deferring one is far cheaper than missing one. More than one entry
        // warns rather than being silently resolved, because there is no real multi-due-date data
        // to design a rule against.
        //
        // Python's `min(...)` over a generator stops at the first raise, so this does too.
        let mut earliest: Option<DateTime> = None;
        let mut unreadable: Option<String> = None;
        for stamp in &stamps {
            match due_central(stamp, tz) {
                Ok(parsed) => {
                    earliest = Some(match earliest {
                        Some(current) if current <= parsed => current,
                        _ => parsed,
                    })
                }
                Err(err) => {
                    unreadable = Some(err);
                    break;
                }
            }
        }
        if let Some(err) = unreadable {
            warnings.push(format!("{title}: unreadable due date ({err}); skipped"));
            continue;
        }
        let due = match earliest {
            Some(due) => due,
            // Unreachable: `stamps` is non-empty and every entry either parsed or broke the loop.
            None => continue,
        };
        if stamps.len() > 1 {
            warnings.push(format!(
                "{title}: {} due dates; using the earliest ({})",
                stamps.len(),
                format_due(Due::DateTime(due))
            ));
        }

        let sections = array_field(raw, "sections")?;
        let mut points: i64 = 0;
        for section in &sections {
            let section = as_object(section, "section")?;
            points += json_int(section.get("total_points"))
                .map_err(|err| SourceError::Failed(format!("{title}: {err}")))?;
        }

        let kind = category_of(&title, &categories);
        if kind.is_none() {
            warnings.push(format!("{title}: uncategorised; using default importance"));
        }
        let importance = match table_lookup(&importance_table, kind.as_deref()) {
            Some(value) => python_int_yaml(value, "importance")?,
            None => DEFAULT_IMPORTANCE,
        };
        let floor = match table_lookup(&floors, kind.as_deref()) {
            Some(value) => python_float(value, "effort floor")?,
            None => DEFAULT_FLOOR_HOURS,
        };
        let effort = round2(f64::max(sections.len() as f64 * per_section / 60.0, floor));

        let assignment_id = raw
            .get("assignment_id")
            .map(json_str)
            .unwrap_or_else(|| "None".to_string());
        let listing: Vec<String> = sections
            .iter()
            .map(|section| {
                let field = |key: &str| {
                    section
                        .get(key)
                        .map(json_str)
                        .unwrap_or_else(|| "None".to_string())
                };
                format!(
                    "- {}.{} {}",
                    field("chapter_number"),
                    field("section_number"),
                    field("title")
                )
            })
            .collect();
        let body = format!(
            "{} zyBooks section(s), {} points.\n\n{}",
            sections.len(),
            points,
            listing.join("\n")
        );

        out.push(Assignment {
            uid: format!("zybooks:{assignment_id}"),
            slug: format!("{slug_prefix}-{}", slugify(&title)),
            title: format!("{course_label} {title}"),
            due,
            course: Some(course_slug.to_string()),
            effort_hours: effort,
            effort_confidence: "low".to_string(),
            effort_source: "inferred".to_string(),
            importance,
            importance_reason: format!(
                "zyBooks {} worth {points} points across {} sections; per-category importance \
                 from config",
                kind.as_deref().filter(|k| !k.is_empty()).unwrap_or("item"),
                sections.len()
            ),
            progress: 0,
            created_by: "zybooks".to_string(),
            body,
        });
    }
    Ok(out)
}

/// A 200 with `success: false` is a dead session, not an empty semester.
///
/// Only an explicitly falsy `success` counts: absence means the endpoint does not report one, and
/// treating that as failure would break every payload shape that has never carried the key.
pub fn require_success(payload: &Json, what: &str) -> Result<(), SourceError> {
    let map = match payload.as_object() {
        Some(map) => map,
        None => return Ok(()),
    };
    let success = match map.get("success") {
        Some(value) => value,
        None => return Ok(()),
    };
    if json_truthy(success) {
        return Ok(());
    }
    let detail = ["error", "message"]
        .iter()
        .find_map(|key| map.get(*key).filter(|value| json_truthy(value)))
        .map(json_str)
        .unwrap_or_else(|| "no detail".to_string());
    Err(SourceError::NotLoggedIn(format!(
        "zybooks {what} reported success=false ({detail})"
    )))
}

// ---------------------------------------------------------------------------------------------
// the network layer
// ---------------------------------------------------------------------------------------------

/// One outbound request, kept as data so a test can inspect it without a socket.
///
/// This is the seam that replaces Python's `monkeypatch.setattr(urllib.request, "urlopen", ...)`.
/// It exists as a type rather than a tuple for one reason: the hand-written `Debug` below.
pub struct HttpRequest {
    pub url: String,
    pub headers: Vec<(String, String)>,
    pub body: Option<Vec<u8>>,
}

impl HttpRequest {
    /// Header lookup, case-insensitively — `urllib.request.Request.get_header` normalises names,
    /// which is why the Python tests ask for `"User-agent"` and find `User-Agent`.
    pub fn header(&self, name: &str) -> Option<&str> {
        self.headers
            .iter()
            .find(|(key, _)| key.eq_ignore_ascii_case(name))
            .map(|(_, value)| value.as_str())
    }
}

/// Redacted by hand, and this is not decoration.
///
/// A derived `Debug` would put the auth token (it is in the query string *and* the `Authorization`
/// header) and the plaintext password (it is the signin body) into any `dbg!`, any `{:?}` log
/// line, and any `unwrap()` panic message that touched a request. `state/runner-log.md` is
/// committed and pushed within minutes of being written.
impl fmt::Debug for HttpRequest {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let path = match self.url.split_once('?') {
            Some((before, _)) => format!("{before}?<redacted>"),
            None => self.url.clone(),
        };
        let names: Vec<&str> = self.headers.iter().map(|(key, _)| key.as_str()).collect();
        f.debug_struct("HttpRequest")
            .field("url", &path)
            .field("headers", &names)
            .field(
                "body",
                &self
                    .body
                    .as_ref()
                    .map(|b| format!("<redacted {} bytes>", b.len())),
            )
            .finish()
    }
}

/// The injection seam: `None` means the real network.
pub type Sender<'a> = &'a dyn Fn(&HttpRequest) -> Result<Vec<u8>, String>;

/// The real sender. Never reached from a test — every test supplies its own, or aims at a URL that
/// fails in the parser before a socket opens.
pub fn default_send(request: &HttpRequest) -> Result<Vec<u8>, String> {
    let agent: ureq::Agent = ureq::Agent::config_builder()
        .timeout_global(Some(Duration::from_secs(60)))
        .build()
        .into();
    let mut response = match &request.body {
        Some(body) => {
            let mut builder = agent.post(&request.url);
            for (key, value) in &request.headers {
                builder = builder.header(key, value);
            }
            builder.send(&body[..]).map_err(|err| err.to_string())?
        }
        None => {
            let mut builder = agent.get(&request.url);
            for (key, value) in &request.headers {
                builder = builder.header(key, value);
            }
            builder.call().map_err(|err| err.to_string())?
        }
    };
    response
        .body_mut()
        .with_config()
        .limit(u64::MAX)
        .read_to_vec()
        .map_err(|err| err.to_string())
}

/// Replace each secret — raw, percent-encoded, and JSON-escaped — with a marker.
///
/// The three forms are the three ways a secret reaches an error message: verbatim in a transport
/// error, percent-encoded because it went into a query string, and backslash-escaped because it
/// went into a JSON request body.
pub fn scrub(text: &str, secrets: &[&str]) -> String {
    let mut out = text.to_string();
    for secret in secrets {
        if secret.is_empty() {
            continue;
        }
        for form in [
            (*secret).to_string(),
            quote(secret),
            json_escape_ascii(secret),
        ] {
            if !form.is_empty() {
                out = out.replace(&form, "<redacted>");
            }
        }
    }
    out
}

/// `urllib.parse.quote(value, safe="")`: everything but the unreserved set is percent-encoded.
fn quote(value: &str) -> String {
    let mut out = String::new();
    for byte in value.as_bytes() {
        let c = *byte as char;
        if c.is_ascii_alphanumeric() || matches!(c, '_' | '.' | '-' | '~') {
            out.push(c);
        } else {
            out.push_str(&format!("%{byte:02X}"));
        }
    }
    out
}

/// `json.dumps(value)[1:-1]` — the JSON string body, with Python's default `ensure_ascii=True`.
///
/// The ASCII escaping matters: a secret containing a non-ASCII character reaches a JSON body as
/// `\uXXXX`, which does not contain the raw form and so would survive a raw-only scrub.
fn json_escape_ascii(value: &str) -> String {
    let mut out = String::new();
    for c in value.chars() {
        match c {
            '"' => out.push_str("\\\""),
            '\\' => out.push_str("\\\\"),
            '\n' => out.push_str("\\n"),
            '\r' => out.push_str("\\r"),
            '\t' => out.push_str("\\t"),
            '\u{8}' => out.push_str("\\b"),
            '\u{c}' => out.push_str("\\f"),
            // Python's ESCAPE_ASCII pattern is `[^ -~]`: printable ASCII is kept and everything
            // else — controls, DEL, and all of non-ASCII — becomes `\uXXXX`.
            c if (' '..='~').contains(&c) => out.push(c),
            c => {
                let mut buffer = [0u16; 2];
                for unit in c.encode_utf16(&mut buffer) {
                    out.push_str(&format!("\\u{unit:04x}"));
                }
            }
        }
    }
    out
}

/// Send, decode, and let **no** error carry a secret out.
///
/// Python builds the `Request` inside the same `try` as the send, because the `full_url` setter
/// raises `ValueError("unknown url type: %r" % full_url)` — a message with the whole token-bearing
/// URL in it. Rust's equivalent is that URL parsing happens inside the sender, so it is covered by
/// the same scrub without needing a second try block.
fn open_json(
    request: &HttpRequest,
    secrets: &[&str],
    sender: Option<Sender>,
) -> Result<Json, SourceError> {
    let send: &dyn Fn(&HttpRequest) -> Result<Vec<u8>, String> = sender.unwrap_or(&default_send);
    match send(request).and_then(|bytes| decode_json(&bytes)) {
        Ok(value) => Ok(value),
        Err(message) => Err(SourceError::Failed(scrub(&message, secrets))),
    }
}

/// `json.loads(resp.read().decode("utf-8-sig", errors="replace"))`.
///
/// The BOM strip is load-bearing, not defensive: the captured payload in `tests/fixtures/` carries
/// one. Decoding with plain UTF-8 raises `Unexpected UTF-8 BOM`, which becomes a generic
/// "zybooks: fetch failed" warning and a day-one ingest of nothing.
fn decode_json(bytes: &[u8]) -> Result<Json, String> {
    let body = bytes.strip_prefix(&[0xEF, 0xBB, 0xBF]).unwrap_or(bytes);
    let text = String::from_utf8_lossy(body);
    serde_json::from_str(&text).map_err(|err| format!("JSONDecodeError: {err}"))
}

fn zybooks_headers() -> Vec<(String, String)> {
    [
        ("Accept", "application/json, text/javascript, */*; q=0.01"),
        ("Origin", "https://learn.zybooks.com"),
        ("Referer", "https://learn.zybooks.com/"),
        ("User-Agent", USER_AGENT),
    ]
    .iter()
    .map(|(key, value)| ((*key).to_string(), (*value).to_string()))
    .collect()
}

pub fn post_json(url: &str, payload: &Json, sender: Option<Sender>) -> Result<Json, SourceError> {
    let password = payload
        .get("password")
        .filter(|value| json_truthy(value))
        .map(json_str)
        .unwrap_or_default();
    let mut headers = vec![("Content-Type".to_string(), "application/json".to_string())];
    headers.extend(zybooks_headers());
    let request = HttpRequest {
        url: url.to_string(),
        headers,
        // Every JSON this crate writes goes through `ledger::dumps_value`. Its sorted keys differ
        // from Python's insertion order for a body of more than one key; the server does not care,
        // and no byte of this reaches disk.
        body: Some(crate::ledger::dumps_value(payload).into_bytes()),
    };
    open_json(&request, &[&password], sender)
}

pub fn get_json(url: &str, token: &str, sender: Option<Sender>) -> Result<Json, SourceError> {
    let joiner = if url.contains('?') { "&" } else { "?" };
    // Percent-encoded, not raw: `http.client` rejects a URL containing any character in
    // `[\x00-\x20\x7f]` — a plain space included — by raising `InvalidURL` with the whole URL in
    // the message. Encoding the value means such a token never reaches the URL raw in either
    // engine.
    let quoted = quote(token);
    let mut headers = vec![("Authorization".to_string(), format!("Bearer {token}"))];
    headers.extend(zybooks_headers());
    let request = HttpRequest {
        url: format!("{url}{joiner}auth_token={quoted}"),
        headers,
        body: None,
    };
    open_json(&request, &[token], sender)
}

/// Returns `(auth_token, user_id)`. `Err` on a rejected login.
pub fn signin(
    email: &str,
    password: &str,
    sender: Option<Sender>,
) -> Result<(String, i64), SourceError> {
    let payload = serde_json::json!({ "email": email, "password": password });
    let data = post_json(SIGNIN_URL, &payload, sender)?;
    if !data.get("success").map(json_truthy).unwrap_or(false) {
        return Err(SourceError::Failed(
            "zybooks signin rejected (bad credentials or SSO-only account)".to_string(),
        ));
    }
    // Python indexes: a missing key raises KeyError, which `collect` turns into "fetch failed".
    // Same outcome, a message that names the field instead of the exception type.
    let token = data
        .pointer("/session/auth_token")
        .and_then(Json::as_str)
        .ok_or_else(|| {
            SourceError::Failed("zybooks signin response has no session.auth_token".to_string())
        })?;
    let user_id = data
        .pointer("/user/user_id")
        .and_then(Json::as_i64)
        .ok_or_else(|| {
            SourceError::Failed("zybooks signin response has no user.user_id".to_string())
        })?;
    Ok((token.to_string(), user_id))
}

pub fn fetch_zybook_codes(
    token: &str,
    user_id: i64,
    sender: Option<Sender>,
) -> Result<Vec<String>, SourceError> {
    let url = format!("{BASE}/user/{user_id}/items?items=%5B%22zybooks%22%5D");
    let data = get_json(&url, token, sender)?;
    require_success(&data, "item list")?;
    let books = match data.get("items").and_then(|items| items.get("zybooks")) {
        Some(Json::Array(books)) => books.clone(),
        _ => return Ok(Vec::new()),
    };
    let mut codes = Vec::new();
    for book in &books {
        match book.get("zybook_code") {
            Some(code) => codes.push(json_str(code)),
            None => {
                return Err(SourceError::Failed(
                    "zybooks item list entry has no zybook_code".to_string(),
                ))
            }
        }
    }
    Ok(codes)
}

pub fn fetch_assignments(
    token: &str,
    zybook_code: &str,
    sender: Option<Sender>,
) -> Result<Json, SourceError> {
    // The token goes in BOTH the query string and the Authorization header: the query-param form
    // is reportedly deprecated, and sending both survives the transition in either direction at
    // no cost.
    let data = get_json(
        &format!("{BASE}/zybook/{zybook_code}/assignments"),
        token,
        sender,
    )?;
    require_success(&data, &format!("assignment fetch for {zybook_code}"))?;
    Ok(data)
}

/// The fetch half alone: sign in, list the student's zybooks, and fetch each book's raw payload.
///
/// **No parsing and no routing** — both moved server-side in C2 (§4.3). What stays here is exactly
/// what needs the student's own credentials, which never leave the machine (D11).
#[cfg(windows)]
pub fn fetch_payloads(email: &str, password: &str) -> Result<Vec<(String, Json)>, SourceError> {
    let (token, user_id) = signin(email, password, None)?;
    let mut out = Vec::new();
    for code in fetch_zybook_codes(&token, user_id, None)? {
        let payload = fetch_assignments(&token, &code, None)?;
        out.push((code, payload));
    }
    Ok(out)
}

#[cfg(test)]
mod tests {
    //! Ported from `tests/test_zybooks.py`, one Rust test per Python test, keeping the Python
    //! name wherever it still describes what happens.
    //!
    //! **Nothing here touches the network.** Every test either injects a [`Sender`] — the seam
    //! that replaces `monkeypatch.setattr(urllib.request, "urlopen", ...)` — or aims the real one
    //! at a URL that fails in the parser. The Python file's `https://example.invalid/` would still
    //! cost a DNS lookup, so the ported URLs use an unknown scheme instead.

    use std::cell::RefCell;

    use jiff::civil::date;

    use super::*;

    fn tz() -> TimeZone {
        TimeZone::get("America/Chicago").expect("the tz database has America/Chicago")
    }

    /// `CFG` from the Python test, spelled as the YAML the live config actually holds.
    fn cfg() -> Mapping {
        crate::yaml::mapping_of(concat!(
            "categories:\n  HW: hw\n  Lab: lab\n  Project: project\n",
            "effort:\n",
            "  minutes_per_section: 6\n",
            "  floors:\n    hw: 0.25\n    lab: 0.5\n    project: 1.0\n",
            "importance:\n  hw: 2\n  lab: 2\n  project: 2\n",
        ))
    }

    /// The real captured payload — read through [`decode_json`], which is how its **UTF-8 BOM**
    /// gets stripped in production too.
    fn fixture_payload() -> Json {
        let bytes = std::fs::read("tests/fixtures/zybooks-assignments.json")
            .expect("the captured zyBooks payload is committed");
        assert_eq!(
            &bytes[..3],
            &[0xEF, 0xBB, 0xBF],
            "the fixture is a real capture and carries a BOM; that is the point of it"
        );
        decode_json(&bytes).expect("the fixture is valid JSON")
    }

    fn load() -> (Vec<Assignment>, Vec<String>) {
        let mut warnings = Vec::new();
        let items = parse_assignments(
            &fixture_payload(),
            "cs-100",
            "CS 100",
            &cfg(),
            &tz(),
            &mut warnings,
        )
        .expect("the fixture payload reports success");
        (items, warnings)
    }

    fn find<'a>(items: &'a [Assignment], needle: &str) -> &'a Assignment {
        items
            .iter()
            .find(|a| a.title.contains(needle))
            .unwrap_or_else(|| panic!("no assignment matching {needle:?}"))
    }

    fn parse(payload: &Json, warnings: &mut Vec<String>) -> Vec<Assignment> {
        parse_assignments(payload, "cs-100", "CS 100", &cfg(), &tz(), warnings)
            .expect("payload parses")
    }

    /// Records the request it is handed, then fails without opening a socket — the port of
    /// `fake_urlopen` raising `RuntimeError("stop before any socket")`.
    fn record_into(seen: &RefCell<Vec<String>>, request: &HttpRequest) -> Result<Vec<u8>, String> {
        seen.borrow_mut().push(request.url.clone());
        for (key, value) in &request.headers {
            seen.borrow_mut().push(format!("{key}: {value}"));
        }
        Err("stop before any socket".to_string())
    }

    // --- the parser, against the real payload ------------------------------------------------

    #[test]
    fn parses_every_assignment_in_the_real_payload() {
        let (items, warnings) = load();
        assert_eq!(items.len(), 24);
        assert!(warnings.is_empty(), "unexpected warnings: {warnings:?}");
    }

    #[test]
    fn utc_due_date_converts_to_central_2359() {
        // 2026-08-27T04:59:00Z is 2026-08-26 23:59 CDT. Getting this wrong shifts every deadline
        // a day, which is the whole point of the system.
        let (items, _) = load();
        assert_eq!(find(&items, "HW 01").due, date(2026, 8, 26).at(23, 59, 0, 0));
    }

    #[test]
    fn uid_and_slug_shape() {
        let (items, _) = load();
        let hw01 = find(&items, "HW 01");
        assert_eq!(hw01.uid, "zybooks:1839992");
        assert!(hw01.slug.starts_with("cs-100-"));
        assert_eq!(hw01.course.as_deref(), Some("cs-100"));
        assert_eq!(hw01.created_by, "zybooks");
    }

    #[test]
    fn title_is_prefixed_with_the_course_label() {
        let (items, _) = load();
        assert_eq!(find(&items, "HW 01").title, "CS 100 HW 01 Introduction to C");
    }

    #[test]
    fn effort_scales_with_section_count() {
        let (items, _) = load();
        let hw01 = find(&items, "HW 01");
        assert_eq!(hw01.effort_hours, 2.5); // 25 sections x 6 min
        assert_eq!(hw01.effort_confidence, "low");
        assert_eq!(hw01.effort_source, "inferred");
    }

    #[test]
    fn lab_effort_hits_the_category_floor() {
        // Lab 01 has 2 sections -> 0.2h by formula, which is wrong for a zyLab. The floor is the
        // whole reason the config carries one.
        let (items, _) = load();
        let lab01 = items
            .iter()
            .find(|a| a.title.ends_with("Lab 01"))
            .expect("Lab 01 is in the payload");
        assert_eq!(lab01.effort_hours, 0.5);
    }

    #[test]
    fn importance_comes_from_the_category_table() {
        let (items, _) = load();
        assert!(items.iter().all(|a| a.importance == 2));
    }

    #[test]
    fn body_lists_points_and_section_count() {
        let (items, _) = load();
        let hw01 = find(&items, "HW 01");
        assert!(hw01.body.contains("193"));
        assert!(hw01.body.contains("25"));
    }

    /// The artefact oracle for this module: every field of all 24 parsed assignments, exactly as
    /// **Python** produced them.
    ///
    /// `tests/fixtures/zybooks-parsed-reference.json` was generated by calling the Python
    /// `engine.zybooks.parse_assignments` on the committed capture and `json.dumps`-ing the
    /// result. It is a **frozen reference**: if this fails, the Rust engine is wrong. Do not
    /// regenerate it.
    ///
    /// Rows are arrays, not objects, so the comparison pins field *order* too and does not lean on
    /// `dumps_value`'s key sorting. The file deliberately ends without a newline — it is one line
    /// of JSON, so there is no line terminator for `core.autocrlf` to rewrite and the bytes are
    /// identical on every platform.
    #[test]
    fn the_parsed_payload_matches_python_byte_for_byte() {
        let (items, _) = load();
        let rows: Vec<Json> = items
            .iter()
            .map(|a| {
                serde_json::json!([
                    a.uid,
                    a.slug,
                    a.title,
                    format_due(Due::DateTime(a.due)),
                    a.course,
                    a.effort_hours,
                    a.effort_confidence,
                    a.effort_source,
                    a.importance,
                    a.importance_reason,
                    a.progress,
                    a.created_by,
                    a.body,
                ])
            })
            .collect();
        let rendered = crate::ledger::dumps_value(&Json::Array(rows));
        let expected = std::fs::read_to_string("tests/fixtures/zybooks-parsed-reference.json")
            .expect("the frozen Python reference is committed");
        assert_eq!(
            rendered, expected,
            "the Rust parser diverged from Python. The reference is frozen: fix the engine, do \
             not regenerate tests/fixtures/zybooks-parsed-reference.json."
        );
    }

    // --- the parser, on shapes the capture does not contain ----------------------------------

    #[test]
    fn assignment_without_a_due_date_is_skipped_with_a_warning() {
        let payload = serde_json::json!({"assignments": [
            {"assignment_id": 1, "title": "HW 99", "visible": true,
             "due_dates": [], "sections": []}
        ]});
        let mut warnings = Vec::new();
        assert!(parse(&payload, &mut warnings).is_empty());
        assert_eq!(warnings.len(), 1);
        assert!(warnings[0].contains("no due date"));
    }

    #[test]
    fn invisible_assignment_is_skipped() {
        let payload = serde_json::json!({"assignments": [
            {"assignment_id": 2, "title": "HW 98", "visible": false,
             "due_dates": [{"date": "2026-09-01T04:59:00Z"}], "sections": []}
        ]});
        assert!(parse(&payload, &mut Vec::new()).is_empty());
    }

    #[test]
    fn a_missing_visible_flag_leaves_the_assignment_visible() {
        // Python tests `is False`, not falsiness. Absent and null both stay visible; only the
        // literal `false` hides an assignment.
        let payload = serde_json::json!({"assignments": [
            {"assignment_id": 2, "title": "HW 98",
             "due_dates": [{"date": "2026-09-01T04:59:00Z"}], "sections": []},
            {"assignment_id": 3, "title": "HW 97", "visible": null,
             "due_dates": [{"date": "2026-09-01T04:59:00Z"}], "sections": []}
        ]});
        assert_eq!(parse(&payload, &mut Vec::new()).len(), 2);
    }

    #[test]
    fn unknown_category_falls_back_to_default_importance_and_warns() {
        let payload = serde_json::json!({"assignments": [
            {"assignment_id": 3, "title": "Midterm Reflection", "visible": true,
             "due_dates": [{"date": "2026-09-01T04:59:00Z"}],
             "sections": [{"chapter_number": 1, "section_number": 1,
                           "title": "x", "total_points": 5}]}
        ]});
        let mut warnings = Vec::new();
        let items = parse(&payload, &mut warnings);
        assert_eq!(items.len(), 1);
        assert_eq!(items[0].importance, DEFAULT_IMPORTANCE);
        assert!(warnings.iter().any(|w| w.contains("uncategorised")));
        // `kind or 'item'` — the reason line has to read as English with no category.
        assert!(items[0]
            .importance_reason
            .starts_with("zyBooks item worth 5 points"));
    }

    #[test]
    fn category_of_matches_prefix_case_insensitively() {
        let cats = crate::yaml::mapping_of("HW: hw\nLab: lab\n");
        assert_eq!(category_of("hw 03", &cats).as_deref(), Some("hw"));
        assert_eq!(category_of("Lab 07", &cats).as_deref(), Some("lab"));
        assert_eq!(category_of("Quiz 1", &cats), None);
    }

    #[test]
    fn falsy_success_payload_raises_not_logged_in() {
        // A token revoked between signin and fetch returns 200 with success=false. Silently
        // yielding [] makes that read as "0 assignments parsed" — the wrong diagnosis for a dead
        // session.
        let payload = serde_json::json!({"success": false, "error": "invalid auth token"});
        let err = parse_assignments(&payload, "cs-100", "CS 100", &cfg(), &tz(), &mut Vec::new())
            .unwrap_err();
        assert_eq!(
            err,
            SourceError::NotLoggedIn(
                "zybooks assignment payload reported success=false (invalid auth token)"
                    .to_string()
            )
        );
    }

    #[test]
    fn payload_without_a_success_key_is_still_parsed() {
        // Only an explicitly falsy success is a session signal; absence is not.
        let payload = serde_json::json!({"assignments": [
            {"assignment_id": 5, "title": "HW 05", "visible": true,
             "due_dates": [{"date": "2026-09-01T04:59:00Z"}], "sections": []}
        ]});
        assert_eq!(parse(&payload, &mut Vec::new()).len(), 1);
    }

    #[test]
    fn multiple_due_dates_takes_the_earliest_and_warns() {
        // zyBooks uses a plural due_dates[] for extensions and per-section overrides. Array order
        // is not documented as chronological, and picking the wrong one shifts a deadline.
        let payload = serde_json::json!({"success": true, "assignments": [
            {"assignment_id": 42, "title": "HW 42", "visible": true,
             "due_dates": [{"date": "2026-09-10T04:59:00Z"},
                           {"date": "2026-09-03T04:59:00Z"}],
             "sections": []}
        ]});
        let mut warnings = Vec::new();
        let items = parse(&payload, &mut warnings);
        assert_eq!(items.len(), 1);
        assert_eq!(items[0].due, date(2026, 9, 2).at(23, 59, 0, 0));
        assert!(warnings.iter().any(|w| w.contains("2 due dates")));
        assert!(warnings.iter().any(|w| w.contains("(2026-09-02T23:59)")));
    }

    #[test]
    fn course_slug_from_config_cannot_carry_a_path_separator() {
        // The slug becomes tasks/<slug>.md. A config typo must not write outside tasks/.
        let payload = serde_json::json!({"success": true, "assignments": [
            {"assignment_id": 7, "title": "HW 07", "visible": true,
             "due_dates": [{"date": "2026-09-01T04:59:00Z"}], "sections": []}
        ]});
        let items = parse_assignments(
            &payload,
            "../../evil",
            "CS 100",
            &cfg(),
            &tz(),
            &mut Vec::new(),
        )
        .expect("payload parses");
        assert_eq!(items[0].slug, "evil-hw-07");
    }

    #[test]
    fn an_unreadable_due_date_skips_the_item_rather_than_the_source() {
        let payload = serde_json::json!({"assignments": [
            {"assignment_id": 8, "title": "HW 08", "visible": true,
             "due_dates": [{"date": "next tuesday"}], "sections": []},
            {"assignment_id": 9, "title": "HW 09", "visible": true,
             "due_dates": [{"date": "2026-09-01T04:59:00Z"}], "sections": []}
        ]});
        let mut warnings = Vec::new();
        let items = parse(&payload, &mut warnings);
        assert_eq!(items.len(), 1, "one bad stamp must not lose the whole batch");
        assert!(warnings.iter().any(|w| w.contains("unreadable due date")));
    }

    #[test]
    fn a_seconds_less_stamp_is_not_a_due_date() {
        // Python's strptime is exact: "%Y-%m-%dT%H:%M:%SZ" needs the seconds and the literal Z,
        // and nothing may trail it.
        assert!(due_central(&Json::String("2026-09-01T04:59Z".into()), &tz()).is_err());
        assert!(due_central(&Json::String("2026-09-01T04:59:00".into()), &tz()).is_err());
        assert!(due_central(&Json::String("2026-09-01T04:59:00Z ".into()), &tz()).is_err());
        assert!(due_central(&Json::String("2026-09-01T04:59:00Z".into()), &tz()).is_ok());
    }

    // --- token and password safety -----------------------------------------------------------

    #[test]
    fn get_json_percent_encodes_the_token_into_the_query() {
        // http.client rejects any URL containing a character in [\x00-\x20\x7f] — a plain space
        // included — by raising InvalidURL with the whole URL in the message. Encoding the value
        // means such a token never reaches the URL raw.
        let seen = RefCell::new(Vec::new());
        let sender = |request: &HttpRequest| record_into(&seen, request);
        assert!(get_json("unreachable://host/v1/thing", "tok en", Some(&sender)).is_err());
        let url = seen.borrow()[0].clone();
        assert!(!url.contains(' '), "{url}");
        assert!(url.contains("auth_token=tok%20en"), "{url}");
    }

    #[test]
    fn no_error_escaping_get_json_carries_the_token() {
        // InvalidURL subclasses ValueError, not OSError, so URLError never wraps it. Unscrubbed it
        // reaches collect()'s generic handler -> a WARN line -> state/runner-log.md -> committed
        // and pushed within minutes.
        let token = "tok en with a space";
        let sender = |request: &HttpRequest| -> Result<Vec<u8>, String> {
            Err(format!(
                "InvalidURL: URL can't contain control characters. {:?}",
                request.url
            ))
        };
        let err = get_json("unreachable://host/v1/thing", token, Some(&sender)).unwrap_err();
        let rendered = format!("{err} / {err:?}");
        assert!(!rendered.contains(token), "{rendered}");
        assert!(!rendered.contains(&quote(token)), "{rendered}");
        assert!(rendered.contains("<redacted>"), "{rendered}");
    }

    #[test]
    fn a_malformed_url_does_not_leak_the_token_either() {
        // Python's Request full_url setter raises ValueError("unknown url type: %r") — a second
        // message with the whole token-bearing URL in it, raised before urlopen is ever called.
        // Rust arrives at the same place from the other side: URL parsing happens inside the
        // sender, so `notaurl` fails there. No network either way; the failure precedes any socket.
        //
        // **The engines differ in what there is to redact here, and the difference favours Rust.**
        // Measured: `ureq` 3.4 renders this as `"http: invalid format"` — the URI is not in the
        // message at all, so nothing needs scrubbing and the assertion below holds trivially. That
        // is exactly why `open_json` scrubs *unconditionally* rather than only where a leak is
        // known: the day a transport starts quoting the URI back, this path is already covered.
        // `no_error_escaping_get_json_carries_the_token` is the test that proves the scrub itself
        // works, by handing it a message shaped like Python's.
        let token = "secrettoken123";
        let err = get_json("notaurl", token, None).unwrap_err();
        let rendered = format!("{err} / {err:?}");
        assert!(!rendered.contains(token), "{rendered}");
    }

    #[test]
    fn no_error_escaping_post_json_carries_the_password() {
        // post_json is the signin call: its body carries the plaintext password.
        let password = "hunter2 with a space";
        let sender = |request: &HttpRequest| -> Result<Vec<u8>, String> {
            let body = String::from_utf8_lossy(request.body.as_deref().unwrap_or_default());
            Err(format!("InvalidURL: boom {:?} {body}", request.url))
        };
        let payload = serde_json::json!({"email": "e@x.com", "password": password});
        let err = post_json("unreachable://host/v1/signin", &payload, Some(&sender)).unwrap_err();
        let rendered = format!("{err} / {err:?}");
        assert!(!rendered.contains(password), "{rendered}");
        assert!(!rendered.contains("hunter2"), "{rendered}");
    }

    #[test]
    fn scrub_replaces_the_raw_percent_encoded_and_json_escaped_forms() {
        // The three forms are the three ways a secret reaches a message. The JSON form is the one
        // a raw-only scrub misses, because a body writer escapes before the error is built.
        let secret = "p a\"s\u{e9}";
        assert_eq!(scrub(&format!("raw={secret}"), &[secret]), "raw=<redacted>");
        assert_eq!(
            scrub(&format!("url={}", quote(secret)), &[secret]),
            "url=<redacted>"
        );
        assert_eq!(
            scrub(&format!("body={}", json_escape_ascii(secret)), &[secret]),
            "body=<redacted>"
        );
        // An empty secret must not turn every message into "<redacted>".
        assert_eq!(scrub("nothing secret here", &[""]), "nothing secret here");
    }

    #[test]
    fn quote_matches_python_urllib_quote_with_no_safe_characters() {
        // Verified against the interpreter: quote(safe="") leaves exactly the unreserved set.
        assert_eq!(quote("tok en"), "tok%20en");
        assert_eq!(quote("aZ09_.-~"), "aZ09_.-~");
        assert_eq!(quote("/?&=+!*()"), "%2F%3F%26%3D%2B%21%2A%28%29");
        assert_eq!(quote("\u{e9}"), "%C3%A9");
    }

    #[test]
    fn json_escape_matches_pythons_ensure_ascii_dumps() {
        // json.dumps(x)[1:-1] with the default ensure_ascii=True.
        assert_eq!(json_escape_ascii("a\"b\\c"), "a\\\"b\\\\c");
        assert_eq!(json_escape_ascii("\u{e9}"), "\\u00e9");
        assert_eq!(json_escape_ascii("\n\t\u{7f}"), "\\n\\t\\u007f");
    }

    #[test]
    fn a_debug_print_of_a_request_hides_the_token_and_the_body() {
        // A derived Debug would put the auth token and the plaintext password into any {:?}.
        let request = HttpRequest {
            url: "https://zyserver.zybooks.com/v1/thing?auth_token=secrettoken123".to_string(),
            headers: vec![(
                "Authorization".to_string(),
                "Bearer secrettoken123".to_string(),
            )],
            body: Some(b"{\"password\": \"hunter2\"}".to_vec()),
        };
        let rendered = format!("{request:?}");
        assert!(!rendered.contains("secrettoken123"), "{rendered}");
        assert!(!rendered.contains("hunter2"), "{rendered}");
        assert!(rendered.contains("<redacted>"), "{rendered}");
    }

    // --- transport ---------------------------------------------------------------------------

    #[test]
    fn get_json_decodes_a_bom_prefixed_response() {
        // A capture of a real zyBooks response carries a UTF-8 BOM. Decoding with plain "utf-8"
        // raises JSONDecodeError: Unexpected UTF-8 BOM, which collect() turns into a generic
        // "zybooks: fetch failed" warning and day one ingests nothing.
        let sender = |_: &HttpRequest| -> Result<Vec<u8>, String> {
            let mut body = vec![0xEF, 0xBB, 0xBF];
            body.extend_from_slice(br#"{"success": true, "assignments": []}"#);
            Ok(body)
        };
        let result = get_json("unreachable://host/v1/thing", "tok", Some(&sender)).unwrap();
        assert_eq!(
            result,
            serde_json::json!({"success": true, "assignments": []})
        );
    }

    #[test]
    fn post_json_sends_a_non_default_user_agent() {
        // zyBooks returns a hard 403 on every request — including signin — when the request
        // carries urllib's default "Python-urllib/3.14" UA. Verified live: an identical request
        // with a real User-Agent succeeds. No offline test can watch the wire, so this asserts the
        // header is set rather than that the call works.
        let seen = RefCell::new(Vec::new());
        let sender = |request: &HttpRequest| record_into(&seen, request);
        let payload = serde_json::json!({"email": "e@x.com", "password": "p"});
        assert!(post_json("unreachable://host/v1/signin", &payload, Some(&sender)).is_err());
        let ua = seen
            .borrow()
            .iter()
            .find(|line| line.starts_with("User-Agent: "))
            .cloned()
            .expect("a User-Agent header was sent");
        assert!(!ua.to_lowercase().contains("python-urllib"), "{ua}");
        assert!(ua.contains(USER_AGENT), "{ua}");
    }

    #[test]
    fn get_json_sends_a_non_default_user_agent() {
        let seen = RefCell::new(Vec::new());
        let sender = |request: &HttpRequest| record_into(&seen, request);
        assert!(get_json("unreachable://host/v1/thing", "tok", Some(&sender)).is_err());
        let wanted = format!("User-Agent: {USER_AGENT}");
        assert!(seen.borrow().iter().any(|line| line == &wanted));
    }

    #[test]
    fn the_user_agent_is_an_honest_identifying_string() {
        // Not browser impersonation: the point is only that it is not urllib's default.
        assert!(USER_AGENT.contains("Knowlu"));
        assert!(!USER_AGENT.to_lowercase().contains("python-urllib"));
    }

    #[test]
    fn get_json_appends_with_an_ampersand_when_the_url_already_has_a_query() {
        let seen = RefCell::new(Vec::new());
        let sender = |request: &HttpRequest| record_into(&seen, request);
        let url = "unreachable://host/v1/items?items=%5B%5D";
        assert!(get_json(url, "tok", Some(&sender)).is_err());
        assert!(seen.borrow()[0].ends_with("?items=%5B%5D&auth_token=tok"));
    }

    #[test]
    fn signin_rejects_a_failed_login_without_naming_the_password() {
        let sender =
            |_: &HttpRequest| -> Result<Vec<u8>, String> { Ok(br#"{"success": false}"#.to_vec()) };
        let err = signin("e@x.com", "hunter2", Some(&sender)).unwrap_err();
        assert_eq!(
            err,
            SourceError::Failed(
                "zybooks signin rejected (bad credentials or SSO-only account)".to_string()
            )
        );
        assert!(!format!("{err:?}").contains("hunter2"));
    }

    #[test]
    fn signin_returns_the_token_and_user_id() {
        let sender = |_: &HttpRequest| -> Result<Vec<u8>, String> {
            Ok(
                br#"{"success": true, "session": {"auth_token": "tok"}, "user": {"user_id": 7}}"#
                    .to_vec(),
            )
        };
        assert_eq!(
            signin("e@x.com", "p", Some(&sender)).unwrap(),
            ("tok".to_string(), 7)
        );
    }

    #[test]
    fn fetch_zybook_codes_reads_the_nested_item_list() {
        let sender = |_: &HttpRequest| -> Result<Vec<u8>, String> {
            Ok(br#"{"success": true, "items": {"zybooks": [{"zybook_code": "UACS100Fall2026"}, {"zybook_code": "HowToUseZyBooks2"}]}}"#.to_vec())
        };
        assert_eq!(
            fetch_zybook_codes("tok", 7, Some(&sender)).unwrap(),
            vec!["UACS100Fall2026", "HowToUseZyBooks2"]
        );
    }

    #[test]
    fn a_dead_session_on_the_item_list_is_not_an_empty_shelf() {
        let sender = |_: &HttpRequest| -> Result<Vec<u8>, String> {
            Ok(br#"{"success": false, "error": "invalid auth token"}"#.to_vec())
        };
        assert_eq!(
            fetch_zybook_codes("tok", 7, Some(&sender)).unwrap_err(),
            SourceError::NotLoggedIn(
                "zybooks item list reported success=false (invalid auth token)".to_string()
            )
        );
    }

    #[test]
    fn a_dead_session_on_an_assignment_fetch_names_the_book() {
        let sender =
            |_: &HttpRequest| -> Result<Vec<u8>, String> { Ok(br#"{"success": false}"#.to_vec()) };
        assert_eq!(
            fetch_assignments("tok", "UACS100Fall2026", Some(&sender)).unwrap_err(),
            SourceError::NotLoggedIn(
                "zybooks assignment fetch for UACS100Fall2026 reported success=false (no detail)"
                    .to_string()
            )
        );
    }
}
