//! VHL Central integration — parser and network surface. Port of `engine/vhl.py`.
//!
//! Wave 6 of the Rust port. Same dual-surface shape as `zybooks`: a pure [`parse_dashboard`]
//! pinned against a real captured dashboard, and a [`login_and_fetch_dashboard`] that needs live
//! credentials and is verified at rollout.
//!
//! # The three live-rollout facts this module encodes (coursework spec §7.2)
//!
//! 1. **Login is CAS and needs the one-time `lt` ticket.** The form carries a Rails
//!    `authenticity_token`, a `lt` login ticket and an empty `service`. Omitting `lt` makes the
//!    POST fail **silently — HTTP 200, the login page again.** So
//!    [`parse_user_session_form`] scrapes *every* named input and button rather than a fixed list,
//!    and a future hidden field keeps working with no code change.
//! 2. **The dashboard is on another host and must be discovered, not constructed.** Login happens
//!    on `www.vhlcentral.com`; the dashboard is on `m3a.vhlcentral.com` at a per-enrollment URL
//!    read out of `data-schools-payload` on the page the login POST returns.
//! 3. **One cookie jar spans both hosts.** Session cookies are scoped to `.vhlcentral.com`. That
//!    is why `ureq` is built with its non-default `cookies` feature and why [`default_opener`]
//!    hands back a closure that *owns* one agent — a fresh agent per request would drop the CAS
//!    session between the login POST and the dashboard GET, and the symptom would be an
//!    unauthenticated page returned with HTTP 200.
//!
//! # And the rule the whole module exists to protect
//!
//! **An unauthenticated page is a session failure, never an empty semester.** Every path that
//! cannot prove it is looking at a real dashboard returns [`SourceError::NotLoggedIn`], which
//! `collect` reports as *"session invalid"* and which stops the sync before it can blank German.

use std::sync::LazyLock;
use std::time::Duration;

use jiff::civil::Date;
use jiff::tz::TimeZone;
use regex::{Captures, Regex};
use serde_json::Value as Json;
use serde_yaml_ng::{Mapping, Value as Yaml};

use crate::coursework::{parse_duration_hours, Assignment, SourceError};
use crate::ingest::slugify;
use crate::pystr::{json_int, json_str, json_truthy, yaml_str, yaml_truthy};
use crate::zybooks::scrub;

pub const MOUNT_MARKER: &str = "js-student-dashboard-app";

/// Not imported from `zybooks`: these modules are siblings and neither should depend on the other.
/// The string is the same one, for the same reason.
pub const USER_AGENT: &str = "Mozilla/5.0 (Windows NT 10.0; Win64; x64) Knowlu";

/// The captured dashboard double-quotes the attribute; the spec's own example single-quotes it.
///
/// Python spells this `([\"'])(.*?)\1` — a backreference, which Rust's `regex` does not have. The
/// alternation is exactly equivalent: each branch runs non-greedily to the *next* quote of the
/// same kind, and a mismatched pair matches neither branch.
static SUMMARIES: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new("(?s)data-assignment-summaries=(?:\"(.*?)\"|'(.*?)')").unwrap()
});

static SECTION: LazyLock<Regex> = LazyLock::new(|| Regex::new(r"/sections/(\d+)/").unwrap());

/// Same double/single-quote tolerance as [`SUMMARIES`]: the post-login landing page's mount
/// element carries the discoverable-dashboard payload here.
static SCHOOLS_PAYLOAD: LazyLock<Regex> =
    LazyLock::new(|| Regex::new("(?s)data-schools-payload=(?:\"(.*?)\"|'(.*?)')").unwrap());

static FORM: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new("(?si)<form\\b[^>]*\\bid=(?:\"user_session\"|'user_session')[^>]*>(.*?)</form>")
        .unwrap()
});

static INPUT_TAG: LazyLock<Regex> = LazyLock::new(|| Regex::new(r"(?i)<input\b([^>]*)/?>").unwrap());

/// Distinct from [`INPUT_TAG`]: a `<button>` has no self-closing form, and when it carries no
/// explicit `value=""` attribute its value is its text content (Rails' `button_tag "Login", name:
/// "commit"`).
static BUTTON_TAG: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"(?is)<button\b([^>]*)>(.*?)</button>").unwrap());

static ATTR: LazyLock<Regex> =
    LazyLock::new(|| Regex::new("(?s)([\\w-]+)\\s*=\\s*(?:\"(.*?)\"|'(.*?)')").unwrap());

static TAG: LazyLock<Regex> = LazyLock::new(|| Regex::new("<[^>]*>").unwrap());

/// Python's `round(x, 2)` — half to EVEN, matching `ranking::round2`.
fn round2(x: f64) -> f64 {
    (x * 100.0).round_ties_even() / 100.0
}

/// The captured text from whichever quote branch of a `(?:"(.*?)"|'(.*?)')` pair fired.
fn quoted_group(captures: &Captures, first: usize) -> String {
    captures
        .get(first)
        .or_else(|| captures.get(first + 1))
        .map(|m| m.as_str().to_string())
        .unwrap_or_default()
}

/// `type(x).__name__` for a value that came out of `json.loads`.
fn python_type_name(value: &Json) -> &'static str {
    match value {
        Json::Null => "NoneType",
        Json::Bool(_) => "bool",
        Json::Number(n) => {
            if n.is_f64() {
                "float"
            } else {
                "int"
            }
        }
        Json::String(_) => "str",
        Json::Array(_) => "list",
        Json::Object(_) => "dict",
    }
}

/// `str(source.get(key) or "")`.
fn field_str(source: &Json, key: &str) -> String {
    match source.get(key) {
        Some(value) if json_truthy(value) => json_str(value),
        _ => String::new(),
    }
}

/// `str(mapping.get(key) or "")` for a YAML mapping.
fn yaml_field(map: &Mapping, key: &str) -> String {
    match crate::yaml::get(map, key) {
        Some(value) if yaml_truthy(value) => yaml_str(value),
        _ => String::new(),
    }
}

// ---------------------------------------------------------------------------------------------
// the parser
// ---------------------------------------------------------------------------------------------

/// Turn a VHL dashboard page into [`Assignment`]s — one per due-date bucket.
///
/// **This function is the reference the server-side port is measured against** (cloud design
/// §4.3): `cloud/supabase/functions/ingest-coursework/parse_vhl.ts` is a faithful TypeScript port
/// of it, and both are gated by the same frozen `tests/fixtures/vhl-parsed-reference.json`. From
/// C2 on, production parses on the server; the fetch half above stays here, because a portal
/// password never leaves the machine (D11).
///
/// `_tz` is in the signature and unused, exactly as in Python: every VHL bucket is due at a
/// hard-coded local `23:59`, so there is no instant to convert. It stays a parameter so `fetch_vhl`
/// reads the same in both engines and so the day VHL starts publishing a real timestamp, the
/// argument is already threaded through.
pub fn parse_dashboard(
    html_text: &str,
    cfg: &Mapping,
    _tz: &TimeZone,
    warnings: &mut Vec<String>,
) -> Result<Vec<Assignment>, SourceError> {
    if !html_text.contains(MOUNT_MARKER) {
        return Err(SourceError::NotLoggedIn(
            "dashboard mount element absent (login page or redirect?)".to_string(),
        ));
    }
    let captures = SUMMARIES.captures(html_text).ok_or_else(|| {
        SourceError::NotLoggedIn("data-assignment-summaries attribute absent".to_string())
    })?;
    let raw = html_escape::decode_html_entities(&quoted_group(&captures, 1)).into_owned();
    let summaries: Json = serde_json::from_str(&raw).map_err(|err| {
        SourceError::NotLoggedIn(format!("data-assignment-summaries not valid JSON ({err})"))
    })?;
    let summaries = match summaries {
        Json::Array(items) => items,
        // A JSON object would raise AttributeError below in Python and a scalar TypeError; neither
        // is classified as a session problem, so both would be reported as "fetch failed" instead
        // of "session invalid".
        other => {
            return Err(SourceError::NotLoggedIn(format!(
                "data-assignment-summaries is {}, not a list",
                python_type_name(&other)
            )))
        }
    };

    let sections = match crate::yaml::get(cfg, "sections") {
        Some(value) if yaml_truthy(value) => match value {
            Yaml::Mapping(inner) => inner.clone(),
            _ => return Err(SourceError::Failed("config key sections is not a mapping".to_string())),
        },
        _ => Mapping::new(),
    };
    let importance = match crate::yaml::get(cfg, "importance") {
        Some(value) => crate::yaml::i64_of(value)
            .ok_or_else(|| SourceError::Failed("importance is not an int".to_string()))?,
        None => 3,
    };
    let importance_reason = yaml_field(cfg, "importance_reason");

    let mut out: Vec<Assignment> = Vec::new();
    for item in &summaries {
        if !item.is_object() {
            // Python would raise AttributeError on `.get` and lose the whole source.
            return Err(SourceError::Failed(
                "a data-assignment-summaries entry is not an object".to_string(),
            ));
        }
        let raw_date = field_str(item, "due_date");
        let detail = field_str(item, "detail_url");
        let section_id = match SECTION.captures(&detail).and_then(|c| c.get(1)) {
            Some(found) => found.as_str().to_string(),
            None => {
                warnings.push(format!("{raw_date}: no section id in detail_url; skipped"));
                continue;
            }
        };
        let mapping = match section_mapping(&sections, &section_id) {
            Some(inner) => inner.clone(),
            None => {
                warnings.push(format!("section {section_id} not in config; skipped"));
                continue;
            }
        };
        // Python calls `date.fromisoformat`, which since 3.11 also accepts the basic form
        // (`20260901`) and ISO week dates. VHL has only ever emitted `YYYY-MM-DD`, and accepting
        // the other spellings here would mean inventing a parser for input no vendor sends; the
        // narrower reading warns and skips where Python would parse.
        let due_date = match Date::strptime("%Y-%m-%d", &raw_date) {
            Ok(parsed) => parsed,
            Err(_) => {
                warnings.push(format!("unreadable due_date '{raw_date}'; skipped"));
                continue;
            }
        };
        let remaining = match parse_duration_hours(&field_str(item, "estimated_time")) {
            Ok(hours) => hours,
            Err(err) => {
                warnings.push(format!("{raw_date}: {err}; skipped"));
                continue;
            }
        };

        let progress = json_int(item.get("percentage_complete"))
            .map_err(|err| SourceError::Failed(format!("{raw_date}: {err}")))?;
        // estimated_time covers what is LEFT. effort_hours is the whole bucket, because
        // `Task.remaining_hours` is `effort_hours * (1 - progress/100)`.
        let effort = if progress < 100 {
            round2(remaining / (1.0 - progress as f64 / 100.0))
        } else {
            remaining
        };
        let count = json_int(item.get("assignment_count"))
            .map_err(|err| SourceError::Failed(format!("{raw_date}: {err}")))?;
        let due = due_date.at(23, 59, 0, 0);
        let label = match yaml_field(&mapping, "label") {
            empty if empty.is_empty() => "Hausaufgaben".to_string(),
            found => found,
        };
        let course = match yaml_field(&mapping, "course") {
            empty if empty.is_empty() => None,
            found => Some(found),
        };

        out.push(Assignment {
            uid: format!("vhl:{section_id}:{raw_date}"),
            // slugify, not the raw config value: the slug becomes `tasks/<slug>.md`, and a config
            // typo carrying a path separator would otherwise write outside `tasks/`.
            slug: format!(
                "{}-hausaufgaben-{raw_date}",
                slugify(course.as_deref().unwrap_or("task"))
            ),
            title: format!(
                "{label} — due {} {} ({count} activities)",
                due_date.strftime("%a"),
                due_date.strftime("%m-%d")
            ),
            due,
            course,
            effort_hours: effort,
            effort_confidence: "high".to_string(),
            effort_source: "vendor".to_string(),
            importance,
            importance_reason: importance_reason.clone(),
            progress,
            created_by: "vhl".to_string(),
            body: format!(
                "{count} VHL activities, {} outstanding.\n\
                 VHL's own estimate for what remains: **{}**.\n\n\
                 Detail: `{detail}`",
                item.get("activities_remaining")
                    .map(json_str)
                    .unwrap_or_else(|| "None".to_string()),
                item.get("estimated_time")
                    .map(json_str)
                    .unwrap_or_else(|| "None".to_string())
            ),
        });
    }
    Ok(out)
}

/// VHL's own `percentage_complete` for every due-date bucket on the dashboard (stream J T8).
/// **A separate function on purpose**: [`parse_dashboard`] is gated byte for byte by the frozen
/// `vhl-parsed-reference.json`, and completion is a different question asked of the same page.
///
/// The uid is [`parse_dashboard`]'s own, `vhl:<section_id>:<due_date>`, built from the same
/// `detail_url` section match and the same raw `due_date` text; a bucket the parser would skip
/// (no section id, an unreadable date) yields no figure here either. The figure is out of `100`,
/// the number the parser already seeds `progress` with once and never again.
///
/// Tolerant by construction — a page that is not a dashboard yields nothing rather than an error:
/// the parse half already reports a dead session, and this must never fail a coursework run.
pub fn completions(html_text: &str) -> Vec<crate::completion::VendorCompletion> {
    let mut out = Vec::new();
    if !html_text.contains(MOUNT_MARKER) {
        return out;
    }
    let Some(captures) = SUMMARIES.captures(html_text) else { return out };
    let raw = html_escape::decode_html_entities(&quoted_group(&captures, 1)).into_owned();
    let Ok(Json::Array(summaries)) = serde_json::from_str::<Json>(&raw) else { return out };
    for item in &summaries {
        if !item.is_object() {
            continue;
        }
        let raw_date = field_str(item, "due_date");
        let detail = field_str(item, "detail_url");
        let Some(section_id) = SECTION.captures(&detail).and_then(|c| c.get(1)) else { continue };
        if Date::strptime("%Y-%m-%d", &raw_date).is_err() {
            continue;
        }
        let Ok(percent) = json_int(item.get("percentage_complete")) else { continue };
        out.push(crate::completion::VendorCompletion {
            source: "vhl".to_string(),
            uid: format!("vhl:{}:{raw_date}", section_id.as_str()),
            earned: percent as f64,
            possible: 100.0,
        });
    }
    out
}

/// Whether `sections[section_id]` in a `coursework.vhl` config block names a real mapping, and
/// the mapping itself when it does.
///
/// The **one** predicate for "is this section mapped" — [`parse_dashboard`] and discovery both
/// call it, rather than each keeping its own copy of "an empty mapping is falsy" to drift apart
/// later. Mirrors [`crate::coursework::route_zybook`]'s role on the zyBooks side.
pub fn section_mapping<'a>(sections: &'a Mapping, section_id: &str) -> Option<&'a Mapping> {
    match crate::yaml::get(sections, section_id) {
        Some(Yaml::Mapping(inner)) if !inner.is_empty() => Some(inner),
        _ => None,
    }
}

/// `(course_id, section_id)` for every summary the dashboard carries, first-seen order,
/// deduplicated.
///
/// This is the discovery half of [`parse_dashboard`], and it needs no `sections:` config because
/// finding out what that config should say is the point. The summaries name no course **title**
/// — only `/courses/<id>/sections/<id>/` — so onboarding shows the pair and asks the student for
/// the code and the label rather than inventing one.
///
/// The `/courses/<id>/` prefix is required, not just `/sections/<id>/`: the pair is the point,
/// and VHL is known to also emit course-less section URLs elsewhere on the page (the committed
/// fixture's own `data-activities-url="/sections/2102121/programs/192"`) that would otherwise
/// surface a section with no course to pair it with.
pub fn discover_sections(html: &str) -> Vec<(String, String)> {
    static COURSE_SECTION: LazyLock<Regex> =
        LazyLock::new(|| Regex::new(r"/courses/(\d+)/sections/(\d+)/").unwrap());
    let mut out: Vec<(String, String)> = Vec::new();
    for caps in COURSE_SECTION.captures_iter(html) {
        let pair = (caps[1].to_string(), caps[2].to_string());
        if !out.contains(&pair) {
            out.push(pair);
        }
    }
    out
}

// ---------------------------------------------------------------------------------------------
// the network layer
// ---------------------------------------------------------------------------------------------

/// The injection seam, shaped like the Python tests' `FakeOpener.open(url, data)`.
///
/// `None` for the body is a GET; `Some(bytes)` is a form POST. `None` for the whole opener means
/// the real network.
pub type Opener<'a> = &'a dyn Fn(&str, Option<&[u8]>) -> Result<String, String>;

/// An opener that **owns one agent**, and therefore one cookie jar.
///
/// This is the single most important line in the module's network half. VHL's session cookie is
/// set on the login POST to `www.vhlcentral.com` and has to be presented to
/// `m3a.vhlcentral.com` two requests later; the cookie is scoped to `.vhlcentral.com`, so one jar
/// covers both. Building an agent per request would silently return an unauthenticated page with
/// HTTP 200 — which [`parse_dashboard`] would correctly, but confusingly, call a dead session.
///
/// Only the supervised live run (plan Task 13) can prove this end to end; no offline fixture can.
pub fn default_opener() -> impl Fn(&str, Option<&[u8]>) -> Result<String, String> {
    let agent: ureq::Agent = ureq::Agent::config_builder()
        .timeout_global(Some(Duration::from_secs(60)))
        .build()
        .into();
    move |url: &str, body: Option<&[u8]>| -> Result<String, String> {
        let mut response = match body {
            Some(data) => agent
                .post(url)
                .header("User-Agent", USER_AGENT)
                // urllib sets this for you whenever `data` is present; ureq does not.
                .header("Content-Type", "application/x-www-form-urlencoded")
                .send(data)
                .map_err(|err| err.to_string())?,
            None => agent
                .get(url)
                .header("User-Agent", USER_AGENT)
                .call()
                .map_err(|err| err.to_string())?,
        };
        let bytes = response
            .body_mut()
            .with_config()
            .limit(u64::MAX)
            .read_to_vec()
            .map_err(|err| err.to_string())?;
        // `decode("utf-8", errors="replace")`.
        Ok(String::from_utf8_lossy(&bytes).into_owned())
    }
}

/// Ordered `name -> value` pairs, because Python's `dict` is ordered and assigning to an existing
/// key keeps its position. Only the POST body's field order depends on it, and no server cares —
/// but the two engines producing the same bytes is cheaper to keep than to argue about.
fn set_field(fields: &mut Vec<(String, String)>, name: &str, value: &str) {
    match fields.iter_mut().find(|(key, _)| key == name) {
        Some(slot) => slot.1 = value.to_string(),
        None => fields.push((name.to_string(), value.to_string())),
    }
}

fn get_field<'a>(fields: &'a [(String, String)], name: &str) -> Option<&'a str> {
    fields
        .iter()
        .find(|(key, _)| key == name)
        .map(|(_, value)| value.as_str())
}

/// Every `name -> value` pair from the `id="user_session"` form's inputs and buttons.
///
/// Generic on purpose: a future hidden field VHL adds to the form keeps working without a code
/// change here, rather than each field being named explicitly. That is the fix for the silent
/// missing-`lt` login failure.
pub fn parse_user_session_form(html_text: &str) -> Result<Vec<(String, String)>, SourceError> {
    let form_html = FORM
        .captures(html_text)
        .and_then(|c| c.get(1))
        .map(|m| m.as_str().to_string())
        .ok_or_else(|| {
            SourceError::NotLoggedIn("no user_session login form on the login page".to_string())
        })?;

    let attrs_of = |attr_text: &str| -> Vec<(String, String)> {
        let mut attrs: Vec<(String, String)> = Vec::new();
        for caps in ATTR.captures_iter(attr_text) {
            let name = caps
                .get(1)
                .map(|m| m.as_str().to_lowercase())
                .unwrap_or_default();
            let value = html_escape::decode_html_entities(&quoted_group(&caps, 2)).into_owned();
            set_field(&mut attrs, &name, &value);
        }
        attrs
    };

    let mut fields: Vec<(String, String)> = Vec::new();
    for caps in INPUT_TAG.captures_iter(&form_html) {
        let attrs = attrs_of(caps.get(1).map(|m| m.as_str()).unwrap_or(""));
        // `if name:` — an empty name is skipped, not stored under "".
        if let Some(name) = get_field(&attrs, "name").filter(|n| !n.is_empty()) {
            let name = name.to_string();
            let value = get_field(&attrs, "value").unwrap_or("").to_string();
            set_field(&mut fields, &name, &value);
        }
    }
    for caps in BUTTON_TAG.captures_iter(&form_html) {
        let attrs = attrs_of(caps.get(1).map(|m| m.as_str()).unwrap_or(""));
        if let Some(name) = get_field(&attrs, "name").filter(|n| !n.is_empty()) {
            let name = name.to_string();
            let value = match get_field(&attrs, "value") {
                Some(found) => found.to_string(),
                None => {
                    let inner = TAG.replace_all(caps.get(2).map(|m| m.as_str()).unwrap_or(""), "");
                    crate::pystr::strip(&html_escape::decode_html_entities(&inner)).to_string()
                }
            };
            set_field(&mut fields, &name, &value);
        }
    }
    if get_field(&fields, "authenticity_token")
        .filter(|token| !token.is_empty())
        .is_none()
    {
        return Err(SourceError::NotLoggedIn(
            "no authenticity_token in the user_session login form".to_string(),
        ));
    }
    Ok(fields)
}

/// Discover the dashboard URL from the post-login landing page.
///
/// The dashboard lives on a different host at a per-enrollment URL that is not known ahead of
/// time — it must be read out of `data-schools-payload` on the page the login POST returns. Each
/// of the three ways this can fail is a different problem for the user (dead session vs. no active
/// enrollment vs. a VHL page-shape change) and gets its own message rather than collapsing into
/// one.
pub fn first_dashboard_link(html_text: &str) -> Result<String, SourceError> {
    let captures = SCHOOLS_PAYLOAD.captures(html_text).ok_or_else(|| {
        SourceError::NotLoggedIn(
            "no data-schools-payload on the post-login page \
             (login rejected, or the form/page shape changed)"
                .to_string(),
        )
    })?;
    let raw = html_escape::decode_html_entities(&quoted_group(&captures, 1)).into_owned();
    let schools: Json = serde_json::from_str(&raw).map_err(|err| {
        SourceError::NotLoggedIn(format!("data-schools-payload not valid JSON ({err})"))
    })?;

    let mut open_enrollments: Vec<&Json> = Vec::new();
    for school in schools.as_array().map(Vec::as_slice).unwrap_or_default() {
        for program in school
            .get("programs")
            .and_then(Json::as_array)
            .map(Vec::as_slice)
            .unwrap_or_default()
        {
            if let Some(open) = program
                .get("enrollments")
                .and_then(|e| e.get("open"))
                .and_then(Json::as_array)
            {
                open_enrollments.extend(open);
            }
        }
    }

    if open_enrollments.is_empty() {
        return Err(SourceError::NotLoggedIn(
            "data-schools-payload has no open enrollment".to_string(),
        ));
    }
    for enrollment in open_enrollments {
        if let Some(link) = enrollment
            .get("dashboard_link")
            .filter(|value| json_truthy(value))
        {
            return Ok(json_str(link));
        }
    }
    Err(SourceError::NotLoggedIn(
        "open enrollment present but no dashboard_link".to_string(),
    ))
}

/// `urllib.parse.quote_plus(value)` — `quote(safe="")` with space rendered as `+`.
fn quote_plus(value: &str) -> String {
    let mut out = String::new();
    for byte in value.as_bytes() {
        let c = *byte as char;
        if c.is_ascii_alphanumeric() || matches!(c, '_' | '.' | '-' | '~') {
            out.push(c);
        } else if c == ' ' {
            out.push('+');
        } else {
            out.push_str(&format!("%{byte:02X}"));
        }
    }
    out
}

/// `urllib.parse.urlencode(fields)`, which quotes with `quote_plus` and `safe=""`.
fn urlencode(fields: &[(String, String)]) -> String {
    fields
        .iter()
        .map(|(key, value)| format!("{}={}", quote_plus(key), quote_plus(value)))
        .collect::<Vec<String>>()
        .join("&")
}

/// Log in with a real form POST and return the dashboard HTML.
///
/// Chrome's cookie store is not readable on this machine (App-Bound Encryption, Chrome 151), so
/// the session is established here rather than borrowed.
///
/// `base` is the *login* host, not the dashboard host — the dashboard is discovered per-enrollment
/// from the login response and fetched directly.
pub fn login_and_fetch_dashboard(
    username: &str,
    password: &str,
    base: &str,
    opener: Option<Opener>,
) -> Result<String, SourceError> {
    let owned;
    let open: &dyn Fn(&str, Option<&[u8]>) -> Result<String, String> = match opener {
        Some(supplied) => supplied,
        None => {
            owned = default_opener();
            &owned
        }
    };
    // Python does not scrub here, and on the evidence it does not need to: the password reaches
    // the POST *body*, which no urllib message quotes. This scrubs anyway, because `ureq`'s error
    // text is a third party's and this tree auto-pushes within minutes of a warning being written.
    let secrets = [password];
    let transport = |err: String| SourceError::Failed(scrub(&err, &secrets));

    let home = open(&format!("{base}/"), None).map_err(transport)?;
    let mut fields = parse_user_session_form(&home)?;
    set_field(&mut fields, "user_session[username]", username);
    set_field(&mut fields, "user_session[password]", password);
    let payload = urlencode(&fields);

    let landing = open(&format!("{base}/user_session"), Some(payload.as_bytes())).map_err(transport)?;
    let dashboard_link = first_dashboard_link(&landing)?;
    open(&dashboard_link, None).map_err(transport)
}

#[cfg(test)]
mod tests {
    //! Ported from `tests/test_vhl.py`, one Rust test per Python test.
    //!
    //! **Nothing here touches the network.** The `FakeOpener` the Python tests install by
    //! monkeypatching `build_opener` becomes an injected [`Opener`] closure that serves a list of
    //! pages in call order and records what it was asked for.

    use std::cell::RefCell;

    use jiff::civil::date;

    use super::*;

    const FIXTURE: &str = "tests/fixtures/vhl-dashboard.html";
    const LOGIN_TICKET: &str = "LT-1787747389rB844A8D064F2F719D0";
    const DASHBOARD_LINK: &str =
        "https://m3a.vhlcentral.com/courses/1623220/sections/2102121/?guids=true";

    fn tz() -> TimeZone {
        TimeZone::get("America/Chicago").expect("the tz database has America/Chicago")
    }

    fn cfg() -> Mapping {
        crate::yaml::mapping_of(concat!(
            "sections:\n",
            "  \"2102121\":\n",
            "    course: gn-103\n",
            "    label: GN 103 Hausaufgaben\n",
            "importance: 3\n",
            "importance_reason: GN 103 accepts no late Hausaufgaben at all\n",
        ))
    }

    fn dashboard_html() -> String {
        std::fs::read_to_string(FIXTURE).expect("the captured VHL dashboard is committed")
    }

    fn load() -> (Vec<Assignment>, Vec<String>) {
        let mut warnings = Vec::new();
        let items = parse_dashboard(&dashboard_html(), &cfg(), &tz(), &mut warnings)
            .expect("the captured dashboard is an authenticated one");
        (items, warnings)
    }

    fn find<'a>(items: &'a [Assignment], suffix: &str) -> &'a Assignment {
        items
            .iter()
            .find(|a| a.uid.ends_with(suffix))
            .unwrap_or_else(|| panic!("no bucket for {suffix}"))
    }

    #[test]
    fn discover_sections_finds_every_course_and_section_once() {
        let pairs = discover_sections(&dashboard_html());
        assert!(pairs.contains(&("1623220".to_string(), "2102121".to_string())), "{pairs:?}");
        let mut sorted = pairs.clone();
        sorted.sort();
        sorted.dedup();
        assert_eq!(sorted.len(), pairs.len(), "a pair was reported twice");
    }

    #[test]
    fn discover_sections_is_first_seen_order() {
        // The fixture carries only one distinct pair, so it can pin dedup but not order.
        assert_eq!(
            discover_sections(
                "/courses/2/sections/9/a /courses/1/sections/3/b /courses/2/sections/9/c"
            ),
            vec![("2".to_string(), "9".to_string()), ("1".to_string(), "3".to_string())]
        );
    }

    /// `html.escape(text, quote=True)` for the characters `json.dumps` can produce.
    fn escape_attr(text: &str) -> String {
        text.replace('&', "&amp;")
            .replace('<', "&lt;")
            .replace('>', "&gt;")
            .replace('"', "&quot;")
            .replace('\'', "&#x27;")
    }

    fn login_page() -> String {
        format!(
            "<html><body>\n\
             <form id=\"user_session\" action=\"/user_session\" method=\"post\">\n\
             <input type=\"hidden\" name=\"authenticity_token\" value=\"AUTHTOKEN123\">\n\
             <input type=\"hidden\" name=\"lt\" value=\"{LOGIN_TICKET}\">\n\
             <input type=\"hidden\" name=\"service\" value=\"\">\n\
             <input type=\"text\" name=\"user_session[username]\" value=\"\">\n\
             <input type=\"password\" name=\"user_session[password]\" value=\"\">\n\
             <button name=\"commit\" type=\"submit\">Login</button>\n\
             </form>\n\
             </body></html>"
        )
    }

    fn landing_page(open_enrollments: Json) -> String {
        let payload = serde_json::json!([{
            "school_id": 10842,
            "school_name": "University of Alabama",
            "programs": [{
                "program_id": 192,
                "title": {"en": "German"},
                "enrollments": {"open": open_enrollments, "closed": []}
            }]
        }]);
        format!(
            "<html><body>\n<div class=\"js-schools-app\" data-schools-payload=\"{}\"></div>\n</body></html>",
            escape_attr(&crate::ledger::dumps_value(&payload))
        )
    }

    /// Serves `pages` in call order and records `(url, body)` for each call — the port of
    /// `FakeOpener`.
    struct Fake {
        pages: RefCell<Vec<String>>,
        calls: RefCell<Vec<(String, Option<Vec<u8>>)>>,
    }

    impl Fake {
        fn new(pages: &[String]) -> Self {
            Fake {
                pages: RefCell::new(pages.to_vec()),
                calls: RefCell::new(Vec::new()),
            }
        }

        fn open(&self, url: &str, body: Option<&[u8]>) -> Result<String, String> {
            self.calls
                .borrow_mut()
                .push((url.to_string(), body.map(<[u8]>::to_vec)));
            let mut pages = self.pages.borrow_mut();
            if pages.is_empty() {
                return Err("the fake opener ran out of pages".to_string());
            }
            Ok(pages.remove(0))
        }
    }

    /// `urllib.parse.parse_qs(body, keep_blank_values=True)`, enough of it for these tests.
    fn parse_qs(body: &[u8]) -> Vec<(String, String)> {
        let unquote = |text: &str| -> String {
            let bytes = text.replace('+', " ").into_bytes();
            let mut out: Vec<u8> = Vec::new();
            let mut index = 0;
            while index < bytes.len() {
                if bytes[index] == b'%' && index + 2 < bytes.len() {
                    let hex = std::str::from_utf8(&bytes[index + 1..index + 3]).unwrap_or("");
                    match u8::from_str_radix(hex, 16) {
                        Ok(byte) => {
                            out.push(byte);
                            index += 3;
                            continue;
                        }
                        Err(_) => {}
                    }
                }
                out.push(bytes[index]);
                index += 1;
            }
            String::from_utf8_lossy(&out).into_owned()
        };
        String::from_utf8_lossy(body)
            .split('&')
            .filter(|pair| !pair.is_empty())
            .map(|pair| match pair.split_once('=') {
                Some((key, value)) => (unquote(key), unquote(value)),
                None => (unquote(pair), String::new()),
            })
            .collect()
    }

    fn qs_get(fields: &[(String, String)], name: &str) -> Option<String> {
        fields
            .iter()
            .find(|(key, _)| key == name)
            .map(|(_, value)| value.clone())
    }

    // --- the parser, against the real dashboard ----------------------------------------------

    #[test]
    fn parses_every_bucket_in_the_real_dashboard() {
        let (items, warnings) = load();
        assert_eq!(items.len(), 13);
        assert!(warnings.is_empty(), "unexpected warnings: {warnings:?}");
    }

    #[test]
    fn bucket_due_is_2359_local() {
        let (items, _) = load();
        assert_eq!(
            find(&items, "2026-08-26").due,
            date(2026, 8, 26).at(23, 59, 0, 0)
        );
    }

    #[test]
    fn uid_carries_section_and_date() {
        let (items, _) = load();
        assert!(items.iter().any(|a| a.uid == "vhl:2102121:2026-08-28"));
    }

    #[test]
    fn slug_is_stable_per_date() {
        let (items, _) = load();
        assert_eq!(
            find(&items, "2026-08-28").slug,
            "gn-103-hausaufgaben-2026-08-28"
        );
    }

    #[test]
    fn title_names_the_day_and_the_count() {
        let (items, _) = load();
        assert_eq!(
            find(&items, "2026-08-28").title,
            "GN 103 Hausaufgaben — due Fri 08-28 (21 activities)"
        );
    }

    #[test]
    fn effort_comes_from_vhls_own_estimate() {
        let (items, _) = load();
        let bucket = find(&items, "2026-08-28");
        assert_eq!(bucket.effort_hours, 2.53); // "2h 32m", 0% complete
        assert_eq!(bucket.effort_source, "vendor");
        assert_eq!(bucket.effort_confidence, "high");
    }

    #[test]
    fn partially_complete_bucket_grosses_the_estimate_up() {
        // 08-26 is 10% complete with 1h22m REMAINING. effort_hours is the whole bucket, because
        // the engine computes remaining as effort * (1 - progress/100).
        let (items, _) = load();
        let bucket = find(&items, "2026-08-26");
        assert_eq!(bucket.progress, 10);
        assert_eq!(bucket.effort_hours, 1.52); // 1.37 / 0.9
        let remaining = bucket.effort_hours * (1.0 - bucket.progress as f64 / 100.0);
        assert_eq!((remaining * 100.0).round_ties_even() / 100.0, 1.37);
    }

    #[test]
    fn importance_comes_from_config() {
        let (items, _) = load();
        assert!(items.iter().all(|a| a.importance == 3));
        assert!(items.iter().all(|a| a.created_by == "vhl"));
    }

    /// The artefact oracle for this module: every field of all 13 buckets, exactly as **Python**
    /// produced them.
    ///
    /// `tests/fixtures/vhl-parsed-reference.json` was written by the Python
    /// `engine.vhl.parse_dashboard` over the committed capture. **Frozen**: if this fails the Rust
    /// engine is wrong. It is also the only reference in the port that carries a non-ASCII
    /// character — the em dash in every title — which makes it the test that would catch a writer
    /// slipping into `ensure_ascii`-style escaping.
    #[test]
    fn the_parsed_dashboard_matches_python_byte_for_byte() {
        let (items, _) = load();
        let rows: Vec<Json> = items
            .iter()
            .map(|a| {
                serde_json::json!([
                    a.uid,
                    a.slug,
                    a.title,
                    crate::ingest::format_due(crate::ingest::Due::DateTime(a.due)),
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
        let expected = std::fs::read_to_string("tests/fixtures/vhl-parsed-reference.json")
            .expect("the frozen Python reference is committed");
        assert_eq!(
            rendered, expected,
            "the Rust parser diverged from Python. The reference is frozen: fix the engine, do \
             not regenerate tests/fixtures/vhl-parsed-reference.json."
        );
    }

    // --- an unauthenticated page is never an empty semester ----------------------------------

    #[test]
    fn login_page_raises_not_logged_in() {
        // A session that has expired serves the marketing/login page with HTTP 200. Treating that
        // as "no assignments" would silently blank German.
        let err = parse_dashboard(
            "<html><body><form id='user_session'></form></body></html>",
            &cfg(),
            &tz(),
            &mut Vec::new(),
        )
        .unwrap_err();
        assert!(matches!(err, SourceError::NotLoggedIn(_)), "{err:?}");
    }

    #[test]
    fn missing_attribute_raises_not_logged_in() {
        let html = "<html><body><div class=\"js-student-dashboard-app\"></div></body></html>";
        let err = parse_dashboard(html, &cfg(), &tz(), &mut Vec::new()).unwrap_err();
        assert_eq!(
            err,
            SourceError::NotLoggedIn("data-assignment-summaries attribute absent".to_string())
        );
    }

    #[test]
    fn invalid_json_in_attribute_raises_not_logged_in() {
        // This is the safety-critical path: an expired VHL session serves the login page with
        // HTTP 200, and if that parsing failure were treated as "no homework", the system would
        // silently blank the user's German coursework.
        let html = "<html><body><div class=\"js-student-dashboard-app\" \
                    data-assignment-summaries=\"not valid json\"></div></body></html>";
        let err = parse_dashboard(html, &cfg(), &tz(), &mut Vec::new()).unwrap_err();
        assert!(matches!(err, SourceError::NotLoggedIn(_)), "{err:?}");
    }

    #[test]
    fn summaries_object_instead_of_list_raises_not_logged_in() {
        let html = "<html><body><div class=\"js-student-dashboard-app\" \
                    data-assignment-summaries='{\"due_date\": \"2026-09-01\"}'></div></body></html>";
        assert_eq!(
            parse_dashboard(html, &cfg(), &tz(), &mut Vec::new()).unwrap_err(),
            SourceError::NotLoggedIn(
                "data-assignment-summaries is dict, not a list".to_string()
            )
        );
    }

    #[test]
    fn summaries_scalar_instead_of_list_raises_not_logged_in() {
        let html = "<html><body><div class=\"js-student-dashboard-app\" \
                    data-assignment-summaries='3'></div></body></html>";
        assert_eq!(
            parse_dashboard(html, &cfg(), &tz(), &mut Vec::new()).unwrap_err(),
            SourceError::NotLoggedIn("data-assignment-summaries is int, not a list".to_string())
        );
    }

    #[test]
    fn not_logged_in_is_the_shared_cross_source_class() {
        // `collect` classifies NotLoggedIn for every source, so it lives beside `Assignment` in
        // `coursework` and both source modules return that one variant. In Python this is an
        // identity check on the exception class; here it is the same enum, which is what makes
        // `collect`'s single match arm cover both sources.
        let from_vhl = parse_dashboard("<html></html>", &cfg(), &tz(), &mut Vec::new()).unwrap_err();
        let from_zybooks = crate::zybooks::require_success(
            &serde_json::json!({"success": false}),
            "assignment payload",
        )
        .unwrap_err();
        assert!(matches!(from_vhl, SourceError::NotLoggedIn(_)));
        assert!(matches!(from_zybooks, SourceError::NotLoggedIn(_)));
    }

    // --- per-item skips ----------------------------------------------------------------------

    #[test]
    fn unknown_section_is_skipped_with_a_warning() {
        let html = "<html><body><div class=\"js-student-dashboard-app\" \
                    data-assignment-summaries=\"[{&quot;due_date&quot;:&quot;2026-09-01&quot;,\
                    &quot;assignment_count&quot;:3,&quot;activities_remaining&quot;:3,\
                    &quot;estimated_time&quot;:&quot;20m&quot;,&quot;percentage_complete&quot;:0,\
                    &quot;detail_url&quot;:&quot;/courses/1/sections/999999/assignments_by_due_date\
                    ?due_date=2026-09-01&quot;}]\"></div></body></html>";
        let mut warnings = Vec::new();
        let items = parse_dashboard(html, &cfg(), &tz(), &mut warnings).unwrap();
        assert!(items.is_empty());
        assert!(warnings.iter().any(|w| w.contains("999999")), "{warnings:?}");
    }

    #[test]
    fn unparseable_estimate_is_skipped_with_a_warning() {
        let html = "<html><body><div class=\"js-student-dashboard-app\" \
                    data-assignment-summaries=\"[{&quot;due_date&quot;:&quot;2026-09-01&quot;,\
                    &quot;assignment_count&quot;:3,&quot;activities_remaining&quot;:3,\
                    &quot;estimated_time&quot;:&quot;ages&quot;,&quot;percentage_complete&quot;:0,\
                    &quot;detail_url&quot;:&quot;/courses/1/sections/2102121/assignments_by_due_date\
                    ?due_date=2026-09-01&quot;}]\"></div></body></html>";
        let mut warnings = Vec::new();
        let items = parse_dashboard(html, &cfg(), &tz(), &mut warnings).unwrap();
        assert!(items.is_empty());
        assert!(warnings.iter().any(|w| w.contains("ages")), "{warnings:?}");
    }

    #[test]
    fn single_quoted_summaries_attribute_is_accepted() {
        // The captured dashboard uses double quotes, but the spec's own example shows the
        // attribute single-quoted. Either must parse — and Rust has no backreference to express
        // "the same quote character", so the alternation that replaces it is what this pins.
        let html = "<html><body><div class=\"js-student-dashboard-app\" \
                    data-assignment-summaries='[{\"due_date\":\"2026-09-01\",\
                    \"assignment_count\":3,\"activities_remaining\":3,\
                    \"estimated_time\":\"20m\",\"percentage_complete\":0,\
                    \"detail_url\":\"/courses/1/sections/2102121/assignments_by_due_date\
                    ?due_date=2026-09-01\"}]'></div></body></html>";
        let mut warnings = Vec::new();
        let items = parse_dashboard(html, &cfg(), &tz(), &mut warnings).unwrap();
        assert!(warnings.is_empty(), "{warnings:?}");
        assert_eq!(items.len(), 1);
        assert_eq!(items[0].uid, "vhl:2102121:2026-09-01");
    }

    #[test]
    fn course_slug_from_config_cannot_carry_a_path_separator() {
        // The slug becomes tasks/<slug>.md. A config typo must not write outside tasks/.
        let evil = crate::yaml::mapping_of(concat!(
            "sections:\n",
            "  \"2102121\":\n",
            "    course: ../../evil\n",
            "    label: X\n",
            "importance: 3\n",
            "importance_reason: \"\"\n",
        ));
        let items = parse_dashboard(&dashboard_html(), &evil, &tz(), &mut Vec::new()).unwrap();
        assert!(!items.is_empty());
        assert!(items.iter().all(|a| a.slug.starts_with("evil-hausaufgaben-")));
    }

    // --- the login chain ---------------------------------------------------------------------

    #[test]
    fn login_and_fetch_dashboard_happy_path() {
        let dashboard = dashboard_html();
        let enrollment = serde_json::json!([{
            "course_name": "GN 103",
            "section_name": "001",
            "instructor_names": "Matthew Feminella",
            "dashboard_link": DASHBOARD_LINK
        }]);
        let fake = Fake::new(&[
            login_page(),
            landing_page(enrollment),
            dashboard.clone(),
        ]);
        let opener = |url: &str, body: Option<&[u8]>| fake.open(url, body);
        let result =
            login_and_fetch_dashboard("quinn", "hunter2", "https://www.vhlcentral.com", Some(&opener))
                .unwrap();
        assert_eq!(result, dashboard);
        assert_eq!(fake.calls.borrow().len(), 3);
        // The third request fetches the DISCOVERED dashboard_link directly, not the login host
        // re-fetched. The host differs from the login host, which is the whole point.
        assert_eq!(fake.calls.borrow()[2].0, DASHBOARD_LINK);
        // The returned page is a real, parseable dashboard — this confirms the whole chain lands
        // somewhere parse_dashboard can use.
        let items = parse_dashboard(&result, &cfg(), &tz(), &mut Vec::new()).unwrap();
        assert_eq!(items.len(), 13);
    }

    #[test]
    fn login_post_body_carries_lt_and_service_from_the_parsed_form() {
        // This is the defect the generic form scrape exists to fix: omitting `lt` makes VHL's
        // CAS-style login fail silently (200, the login page again).
        let enrollment = serde_json::json!([{"dashboard_link": DASHBOARD_LINK}]);
        let fake = Fake::new(&[
            login_page(),
            landing_page(enrollment),
            "<html>dashboard</html>".to_string(),
        ]);
        let opener = |url: &str, body: Option<&[u8]>| fake.open(url, body);
        login_and_fetch_dashboard("quinn", "hunter2", "https://www.vhlcentral.com", Some(&opener))
            .unwrap();
        let calls = fake.calls.borrow();
        let (post_url, post_body) = &calls[1];
        assert_eq!(post_url, "https://www.vhlcentral.com/user_session");
        let fields = parse_qs(post_body.as_deref().expect("the login POST carries a body"));
        assert_eq!(qs_get(&fields, "lt").as_deref(), Some(LOGIN_TICKET));
        assert_eq!(qs_get(&fields, "service").as_deref(), Some(""));
        assert_eq!(
            qs_get(&fields, "authenticity_token").as_deref(),
            Some("AUTHTOKEN123")
        );
        assert_eq!(
            qs_get(&fields, "user_session[username]").as_deref(),
            Some("quinn")
        );
        assert_eq!(
            qs_get(&fields, "user_session[password]").as_deref(),
            Some("hunter2")
        );
        // A <button> with no value="" attribute takes its text content — Rails' button_tag.
        assert_eq!(qs_get(&fields, "commit").as_deref(), Some("Login"));
    }

    #[test]
    fn login_rejected_serves_login_page_again_raises_not_logged_in() {
        // A failed login makes VHL re-serve the login page with HTTP 200 — no
        // data-schools-payload anywhere in the response.
        let fake = Fake::new(&[login_page(), login_page()]);
        let opener = |url: &str, body: Option<&[u8]>| fake.open(url, body);
        let err = login_and_fetch_dashboard(
            "quinn",
            "wrongpass",
            "https://www.vhlcentral.com",
            Some(&opener),
        )
        .unwrap_err();
        assert!(matches!(err, SourceError::NotLoggedIn(_)), "{err:?}");
    }

    #[test]
    fn payload_with_no_open_enrollment_raises_not_logged_in() {
        let fake = Fake::new(&[login_page(), landing_page(serde_json::json!([]))]);
        let opener = |url: &str, body: Option<&[u8]>| fake.open(url, body);
        let err =
            login_and_fetch_dashboard("q", "p", "https://www.vhlcentral.com", Some(&opener))
                .unwrap_err();
        assert_eq!(
            err,
            SourceError::NotLoggedIn("data-schools-payload has no open enrollment".to_string())
        );
    }

    #[test]
    fn open_enrollment_without_dashboard_link_raises_not_logged_in() {
        let enrollment = serde_json::json!([{"course_name": "GN 103", "section_name": "001"}]);
        let fake = Fake::new(&[login_page(), landing_page(enrollment)]);
        let opener = |url: &str, body: Option<&[u8]>| fake.open(url, body);
        let err =
            login_and_fetch_dashboard("q", "p", "https://www.vhlcentral.com", Some(&opener))
                .unwrap_err();
        assert_eq!(
            err,
            SourceError::NotLoggedIn("open enrollment present but no dashboard_link".to_string())
        );
    }

    #[test]
    fn the_three_failure_modes_report_distinct_messages() {
        // "payload attribute absent", "no open enrollment" and "enrollment present but no
        // dashboard_link" are different problems for the user and must not collapse into one.
        let run = |pages: Vec<String>| -> String {
            let fake = Fake::new(&pages);
            let opener = |url: &str, body: Option<&[u8]>| fake.open(url, body);
            login_and_fetch_dashboard("q", "p", "https://www.vhlcentral.com", Some(&opener))
                .unwrap_err()
                .to_string()
        };
        let absent = run(vec![login_page(), login_page()]);
        let no_open = run(vec![login_page(), landing_page(serde_json::json!([]))]);
        let no_link = run(vec![
            login_page(),
            landing_page(serde_json::json!([{"course_name": "GN 103"}])),
        ]);
        let mut messages = vec![absent, no_open, no_link];
        messages.sort();
        messages.dedup();
        assert_eq!(messages.len(), 3);
    }

    #[test]
    fn no_user_session_form_raises_not_logged_in() {
        let fake = Fake::new(&["<html><body>nothing here</body></html>".to_string()]);
        let opener = |url: &str, body: Option<&[u8]>| fake.open(url, body);
        assert_eq!(
            login_and_fetch_dashboard("q", "p", "https://www.vhlcentral.com", Some(&opener))
                .unwrap_err(),
            SourceError::NotLoggedIn("no user_session login form on the login page".to_string())
        );
    }

    #[test]
    fn form_without_authenticity_token_raises_not_logged_in() {
        let page = "<html><body><form id=\"user_session\" action=\"/user_session\">\
                    <input type=\"hidden\" name=\"lt\" value=\"LT-1\">\
                    </form></body></html>"
            .to_string();
        let fake = Fake::new(&[page]);
        let opener = |url: &str, body: Option<&[u8]>| fake.open(url, body);
        assert_eq!(
            login_and_fetch_dashboard("q", "p", "https://www.vhlcentral.com", Some(&opener))
                .unwrap_err(),
            SourceError::NotLoggedIn(
                "no authenticity_token in the user_session login form".to_string()
            )
        );
    }

    #[test]
    fn a_transport_failure_never_carries_the_password() {
        // Python does not scrub here and does not need to — the password is in the POST body,
        // which no urllib message quotes. `ureq`'s error text is a third party's, and this tree
        // auto-pushes, so the port scrubs anyway.
        let fake = Fake::new(&[login_page()]);
        let opener = |url: &str, body: Option<&[u8]>| -> Result<String, String> {
            match fake.open(url, body) {
                Ok(page) if body.is_none() => Ok(page),
                _ => Err(format!(
                    "connection reset while sending {}",
                    String::from_utf8_lossy(body.unwrap_or_default())
                )),
            }
        };
        let err = login_and_fetch_dashboard(
            "quinn",
            "hunter2",
            "https://www.vhlcentral.com",
            Some(&opener),
        )
        .unwrap_err();
        let rendered = format!("{err} / {err:?}");
        assert!(!rendered.contains("hunter2"), "{rendered}");
        assert!(rendered.contains("<redacted>"), "{rendered}");
    }

    // --- the pieces the login chain is built from --------------------------------------------

    #[test]
    fn every_named_field_in_the_form_is_scraped_not_a_fixed_list() {
        // The whole point of the generic scrape: a hidden field VHL adds tomorrow is posted back
        // without a code change here.
        let page = login_page().replace(
            "<input type=\"hidden\" name=\"service\" value=\"\">",
            "<input type=\"hidden\" name=\"service\" value=\"\">\n\
             <input type=\"hidden\" name=\"brand_new_field\" value=\"surprise\">",
        );
        let fields = parse_user_session_form(&page).unwrap();
        assert_eq!(get_field(&fields, "brand_new_field"), Some("surprise"));
        // Order is the document order, which is what keeps the posted body byte-identical.
        let names: Vec<&str> = fields.iter().map(|(key, _)| key.as_str()).collect();
        assert_eq!(
            names,
            vec![
                "authenticity_token",
                "lt",
                "service",
                "brand_new_field",
                "user_session[username]",
                "user_session[password]",
                "commit",
            ]
        );
    }

    #[test]
    fn a_single_quoted_attribute_value_in_the_form_is_read() {
        let page = "<html><form id='user_session'>\
                    <input type='hidden' name='authenticity_token' value='TOK'>\
                    <input type='hidden' name='lt' value='LT-9'>\
                    </form></html>";
        let fields = parse_user_session_form(page).unwrap();
        assert_eq!(get_field(&fields, "authenticity_token"), Some("TOK"));
        assert_eq!(get_field(&fields, "lt"), Some("LT-9"));
    }

    #[test]
    fn an_escaped_attribute_value_is_unescaped_before_it_is_posted() {
        let page = "<html><form id=\"user_session\">\
                    <input name=\"authenticity_token\" value=\"a&amp;b&quot;c\">\
                    </form></html>";
        let fields = parse_user_session_form(page).unwrap();
        assert_eq!(get_field(&fields, "authenticity_token"), Some("a&b\"c"));
    }

    #[test]
    fn urlencode_uses_quote_plus_exactly_as_urllib_does() {
        // urllib.parse.urlencode quotes with quote_plus and safe="": a space is "+", everything
        // outside the unreserved set is percent-encoded.
        let fields = vec![
            ("user_session[username]".to_string(), "a b".to_string()),
            ("service".to_string(), String::new()),
        ];
        assert_eq!(
            urlencode(&fields),
            "user_session%5Busername%5D=a+b&service="
        );
    }

    #[test]
    fn a_button_with_an_explicit_value_keeps_it() {
        let page = "<html><form id=\"user_session\">\
                    <input name=\"authenticity_token\" value=\"T\">\
                    <button name=\"commit\" value=\"go\">Login</button>\
                    </form></html>";
        let fields = parse_user_session_form(page).unwrap();
        assert_eq!(get_field(&fields, "commit"), Some("go"));
    }

    #[test]
    fn the_user_agent_is_an_honest_identifying_string() {
        assert!(USER_AGENT.contains("Knowlu"));
        assert!(!USER_AGENT.to_lowercase().contains("python-urllib"));
    }

    // --- completion (stream J T8) ------------------------------------------------------------

    /// A dashboard page carrying the given summaries, entity-escaped the way the real one is.
    /// Fabricated ids throughout.
    fn dashboard_with(summaries: &Json) -> String {
        let attr = summaries.to_string().replace('&', "&amp;").replace('"', "&quot;");
        format!(
            "<html><body><div class=\"{MOUNT_MARKER}\" data-assignment-summaries=\"{attr}\"></div></body></html>"
        )
    }

    fn bucket(date: &str, percent: i64) -> Json {
        serde_json::json!({
            "due_date": date,
            "detail_url": format!("/courses/1000001/sections/2000001/assignments_by_due_date?due_date={date}"),
            "estimated_time": if percent >= 100 { "0m" } else { "40m" },
            "percentage_complete": percent,
            "assignment_count": 4,
            "activities_remaining": if percent >= 100 { 0 } else { 2 },
        })
    }

    fn fabricated_cfg() -> Mapping {
        crate::yaml::mapping_of("sections:\n  \"2000001\":\n    course: gn-101\n    label: GN 101\n")
    }

    fn scored_dashboard() -> String {
        dashboard_with(&serde_json::json!([
            bucket("2026-09-21", 100),
            bucket("2026-09-23", 99),
            bucket("2026-09-25", 0),
        ]))
    }

    #[test]
    fn a_bucket_at_one_hundred_is_complete_and_nothing_below_is() {
        let all = completions(&scored_dashboard());
        let complete: Vec<&str> =
            all.iter().filter(|c| c.is_complete()).map(|c| c.uid.as_str()).collect();
        assert_eq!(complete, vec!["vhl:2000001:2026-09-21"]);
        let at_99 = all.iter().find(|c| c.uid == "vhl:2000001:2026-09-23").expect("the 99% bucket");
        assert_eq!((at_99.source.as_str(), at_99.earned, at_99.possible), ("vhl", 99.0, 100.0));
        assert_eq!(all.len(), 3);
    }

    #[test]
    fn vhl_completion_uids_are_the_parsers_uids() {
        // `parse_vhl.ts` builds the same `vhl:${sectionId}:${rawDate}`; both parsers are gated by
        // the frozen reference, so matching the Rust parser is matching both.
        let html = scored_dashboard();
        let mut warnings = Vec::new();
        let parsed = parse_dashboard(&html, &fabricated_cfg(), &tz(), &mut warnings).unwrap();
        let parsed_uids: Vec<String> = parsed.iter().map(|a| a.uid.clone()).collect();
        let completion_uids: Vec<String> = completions(&html).into_iter().map(|c| c.uid).collect();
        assert_eq!(completion_uids, parsed_uids);
        // And the number is the one the parser seeds `progress` with.
        let seeded: Vec<i64> = parsed.iter().map(|a| a.progress).collect();
        let reported: Vec<i64> = completions(&html).iter().map(|c| c.earned as i64).collect();
        assert_eq!(reported, seeded);
    }

    #[test]
    fn the_captured_dashboard_has_nothing_complete() {
        let all = completions(&dashboard_html());
        let (items, _) = load();
        assert_eq!(all.len(), items.len());
        assert!(all.iter().all(|c| !c.is_complete()), "{all:?}");
    }

    #[test]
    fn a_page_that_is_not_a_dashboard_yields_no_completions() {
        assert!(completions("<html>login</html>").is_empty());
        assert!(completions(&format!("<div class=\"{MOUNT_MARKER}\"></div>")).is_empty());
        let bad = dashboard_with(&serde_json::json!([{"due_date": "2026-09-21", "detail_url": "/x", "percentage_complete": 100}]));
        assert!(completions(&bad).is_empty(), "a bucket with no section id names no note");
    }
}
