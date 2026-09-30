//! Grades from Blackboard, the app's half (spec `2026-09-29-grades-design.md`).
//!
//! **The gate** (spec §4; cloud design ruling 12). [`availability`] is the one predicate that decides
//! whether grades are offered, and its four callers — `grades_status`, `grades_connect`,
//! `grades_refresh` and the scheduler's grades step — check it and never re-derive it. No build
//! skips it: no `cfg`, feature or environment variable reaches this function.
//!
//! **The pure pieces** (Task 5b): `grades.json`, paging, sign-in detection, the bundle's assembly
//! and the kept session's directory. None opens a window or makes a call; the one network call is
//! injected into [`read_list`]. No grade value is logged anywhere in this module.
//!
//! **The window, the gated seams and the four commands** (Task 5c): `status_for`, `connect_with`,
//! `refresh_with` and `forget_with` take the curated row as input and the window and session as
//! closures; the `#[tauri::command]`s at the end are thin wrappers over them. The kept session is
//! the one exception to the capture window's "keep nothing" rule (ruling G1; cloud design ruling 12).
use crate::scaffold::Curated;
use serde_json::{json, Value};
use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

/// What the predicate answers. `Available` carries the curated row's own `lms_host`, the only host
/// grades are ever fetched from (spec §4: no address entry).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Availability {
    Available { host: &'static str },
    /// The slot's `grades (skipped: not a Blackboard school)`.
    NotBlackboard,
    /// The slot's `grades (skipped: not available at your school yet)`, and the refusal
    /// `grades_connect` and `grades_refresh` return.
    NotAvailableYet,
}

/// **Whether grades are available**, from the vault's curated row (`scaffold::curated` of
/// `config/campus.yaml`'s `unitid`, `None` for an uncurated school or no `unitid`) and that file's
/// `lms`.
///
/// Available only for a curated row whose `lms_kind` is `blackboard` and which carries
/// `policy_read`. A curated row's kind wins over the vault's `lms`; `lms` only tells an uncurated
/// Blackboard school (not available yet) from anything else (not a Blackboard school).
pub fn availability(row: Option<&Curated>, campus_lms: &str) -> Availability {
    match row {
        Some(c) if c.lms_kind != "blackboard" => Availability::NotBlackboard,
        Some(c) => match c.policy_read {
            Some(_) => Availability::Available { host: c.lms_host },
            None => Availability::NotAvailableYet,
        },
        None if campus_lms.trim() == "blackboard" => Availability::NotAvailableYet,
        None => Availability::NotBlackboard,
    }
}

// ---- the pure pieces (Task 5b): no window, no Tauri item, no network ------------------------

/// The one window label grades use, visible for sign-in and hidden for a refresh (spec §4). Like
/// `lms_link::WINDOW` it has **no capability grant**; `app/tests/grades.rs` pins that
/// `app/capabilities/default.json` never names it.
pub const WINDOW: &str = "lms-grades";

/// **Grades preferences: `hidden` only** (spec §3), in `<profile app data>\grades.json`, a file beside
/// `settings.json` and never a field in it — `state::Settings` falls back to defaults on a parse
/// failure, so a new field there would reset a student's backup folder and autostart (the precedent
/// of `account::cache_path` and `onboarding::offer_marker`). No LMS host lives here: the host comes
/// only from the curated campus row.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct GradesPrefs {
    pub hidden: bool,
}

/// `<data_dir>\grades.json`.
pub fn prefs_path(data_dir: &Path) -> PathBuf {
    data_dir.join("grades.json")
}

impl GradesPrefs {
    /// A missing file, or one that does not parse as `{ "hidden": <bool> }`, is `hidden: false`:
    /// grades show unless the student hid them. Nothing here reads or writes `settings.json`.
    pub fn load(data_dir: &Path) -> GradesPrefs {
        std::fs::read_to_string(prefs_path(data_dir))
            .ok()
            .and_then(|text| serde_json::from_str::<GradesPrefs>(&text).ok())
            .unwrap_or_default()
    }

    /// Through `ledger::dumps_value`, like every JSON this app writes; to a sibling temp file first
    /// and renamed over `grades.json`, so a crash never leaves half a file.
    pub fn save(&self, data_dir: &Path) -> Result<(), String> {
        std::fs::create_dir_all(data_dir).map_err(|e| e.to_string())?;
        let v = serde_json::to_value(self).map_err(|e| e.to_string())?;
        let tmp = data_dir.join(format!("grades.json.{}.tmp", std::process::id()));
        std::fs::write(&tmp, knowlu_engine::ledger::dumps_value(&v)).map_err(|e| e.to_string())?;
        std::fs::rename(&tmp, prefs_path(data_dir)).map_err(|e| {
            let _ = std::fs::remove_file(&tmp);
            e.to_string()
        })
    }
}

// ---- hosts and paging (spec §3) --------------------------------------------------------------

/// Every list follows `paging.nextPage` to the end, and no list reads more than this many pages, so a
/// runaway `nextPage` loop is impossible; a list longer than this is an error, never cut short.
pub const MAX_PAGES: usize = 20;

/// Whether `url` is `https://<host>…` exactly: the scheme `https`, the authority the host itself
/// (case-blind; no user, no port), and no backslash, whitespace or control character anywhere.
pub fn on_host(url: &str, host: &str) -> bool {
    if host.is_empty() || url.chars().any(|c| c == '\\' || c.is_whitespace() || c.is_control()) {
        return false;
    }
    let Some(rest) = url.get(..8).filter(|s| s.eq_ignore_ascii_case("https://")).map(|_| &url[8..]) else {
        return false;
    };
    let end = rest.find(['/', '?', '#']).unwrap_or(rest.len());
    rest[..end].eq_ignore_ascii_case(host)
}

/// The next page's URL from one page's body: Blackboard's `paging.nextPage`, a path on the same host
/// (made absolute) or an absolute URL, followed **only while it stays on `host`**. `None` is the last
/// page, and also a `nextPage` that points anywhere else — the session's cookies go to one host only.
pub fn next_page(host: &str, body: &Value) -> Option<String> {
    let next = body.get("paging")?.get("nextPage")?.as_str()?;
    let url = if next.starts_with('/') && !next.starts_with("//") { format!("https://{host}{next}") } else { next.to_string() };
    on_host(&url, host).then_some(url)
}

/// One list, every page of it up to [`MAX_PAGES`], each page's body verbatim. `fetch` is the one
/// network call, injected, and answers a body or the status or reason the call failed with, which
/// is returned as is; a body that is not JSON is `unreadable`. A `first` URL off `host` is refused
/// before anything is fetched.
///
/// A list is whole or it is an error, never a truncated `Ok`: the engine reads a column missing from
/// a course as removed, and a grade missing from a list as unattempted. So a page whose `nextPage`
/// is set (neither absent, `null` nor empty) but refused by [`next_page`] is `next page off the LMS
/// host`, and a twentieth page that still names a next page is `more than 20 pages`.
pub fn read_list(host: &str, first: &str, mut fetch: impl FnMut(&str) -> Result<String, String>) -> Result<Vec<Value>, String> {
    if !on_host(first, host) {
        return Err("not on the LMS host".into());
    }
    let mut pages = Vec::new();
    let mut url = first.to_string();
    loop {
        let page: Value = serde_json::from_str(&fetch(&url)?).map_err(|_| "unreadable".to_string())?;
        let named = page.pointer("/paging/nextPage").is_some_and(|n| !n.is_null() && n.as_str() != Some(""));
        let next = next_page(host, &page);
        pages.push(page);
        match next {
            Some(_) if pages.len() >= MAX_PAGES => return Err(format!("more than {MAX_PAGES} pages")),
            Some(n) => url = n,
            None if named => return Err("next page off the LMS host".into()),
            None => return Ok(pages),
        }
    }
}

// ---- sign-in detection (spec §15, signed default 1) ------------------------------------------

/// Signed in only when **both** hold: the window's URL is on the LMS host, and `users/me` answered
/// 200 with the window's cookies. The host alone is a login page Blackboard serves itself; a 200
/// alone could be a window still on the SSO host.
pub fn signed_in(window_url: Option<&str>, host: &str, users_me_status: Option<u16>) -> bool {
    users_me_status == Some(200) && window_url.is_some_and(|u| on_host(u, host))
}

// ---- the capture bundle (spec §5) ------------------------------------------------------------

/// The bundle's header: the LMS kind (a parameter, so Canvas is one more arm), the curated row's
/// host and the capture's own timestamp, which the caller reads from the clock.
#[derive(Debug, Clone, Copy)]
pub struct CaptureHead<'a> {
    pub lms: &'a str,
    pub host: &'a str,
    pub fetched_at: &'a str,
}

/// One available course's three lists, each its pages verbatim, or the status or reason its call
/// failed with. `categories` is optional: its failure only drops the grouping.
#[derive(Debug, Clone)]
pub struct CourseCalls {
    pub columns: Result<Vec<Value>, String>,
    pub grades: Result<Vec<Value>, String>,
    pub categories: Result<Vec<Value>, String>,
}

/// Every page's `results`, in order. A page without a `results` list is `unreadable`.
pub fn merge_pages(pages: &[Value]) -> Result<Vec<Value>, String> {
    let mut out = Vec::new();
    for p in pages {
        out.extend(p.get("results").and_then(Value::as_array).ok_or("unreadable")?.iter().cloned());
    }
    Ok(out)
}

/// A membership's `courseId`, the primary id the course's gradebook calls and `per_course` key on.
pub fn membership_course_id(membership: &Value) -> Option<&str> {
    membership.get("courseId")?.as_str().filter(|s| !s.is_empty())
}

/// The memberships a capture reads (spec §3: only courses whose membership is available): every
/// page merged, then only those whose `availability.available` is `Yes` and which carry a course id.
pub fn available_memberships(pages: &[Value]) -> Result<Vec<Value>, String> {
    let all = merge_pages(pages)?;
    Ok(all
        .into_iter()
        .filter(|m| m.pointer("/availability/available").and_then(Value::as_str) == Some("Yes"))
        .filter(|m| membership_course_id(m).is_some())
        .collect())
}

/// **The bundle, version 1** (spec §5), from `users/me`'s body, the memberships' pages and each
/// available course's calls keyed by its membership `courseId`. Rows stay verbatim — the engine owns
/// every interpretation. A course whose columns or grades failed, or which nobody captured, carries
/// `"error"` and empty lists; the others are untouched. `Err` only when `users/me` has no `id` or the
/// memberships are unreadable: there is then no bundle to write.
pub fn assemble_bundle(head: &CaptureHead, user: &Value, memberships: &[Value], per_course: &BTreeMap<String, CourseCalls>) -> Result<Value, String> {
    let user_id = user.get("id").and_then(Value::as_str).filter(|s| !s.is_empty()).ok_or("users/me carried no id")?;
    let mut courses = Vec::new();
    for m in available_memberships(memberships)? {
        let calls = membership_course_id(&m).and_then(|id| per_course.get(id));
        let lists = calls.ok_or_else(|| "not captured".to_string()).and_then(|c| Ok((list(&c.columns)?, list(&c.grades)?)));
        courses.push(match lists {
            Ok((columns, grades)) => {
                let categories = calls.and_then(|c| list(&c.categories).ok());
                json!({ "membership": m, "columns": columns, "grades": grades, "categories": categories, "error": null })
            }
            Err(e) => json!({ "membership": m, "columns": [], "grades": [], "categories": null, "error": e }),
        });
    }
    Ok(json!({ "schema": 1, "lms": head.lms, "host": head.host, "fetched_at": head.fetched_at,
               "user_id": user_id, "courses": courses }))
}

fn list(call: &Result<Vec<Value>, String>) -> Result<Vec<Value>, String> {
    merge_pages(call.as_ref().map_err(Clone::clone)?)
}

// ---- the kept session's directory (spec §4) --------------------------------------------------

/// The persistent WebView2 profile's folder name inside the profile's app data.
const SESSION_DIR: &str = "lms-session";

/// WebView2 lets go of a profile a moment after its window is gone, so the first delete can lose;
/// twelve tries at 250 ms is `lms_link`'s same three seconds of patience.
const WIPE_TRIES: u32 = 12;
const WIPE_BACKOFF: std::time::Duration = std::time::Duration::from_millis(250);

/// `<data_dir>\lms-session`: the kept Blackboard session, under the profile's app data (itself
/// under `state::app_data_root()`), never the vault and never synced.
pub fn session_dir(data_dir: &Path) -> PathBuf {
    data_dir.join(SESSION_DIR)
}

/// The one directory [`forget`] will delete: exactly `session_dir(data_dir)` for an absolute
/// `data_dir` with no `..` in it. A child, a sibling, another profile's or a relative path is refused.
pub fn is_session_dir(data_dir: &Path, dir: &Path) -> bool {
    data_dir.is_absolute()
        && !data_dir.components().any(|c| c == std::path::Component::ParentDir)
        && dir == session_dir(data_dir)
}

/// **Forget the Blackboard sign-in**: delete `session_dir(data_dir)` and nothing else, patiently.
/// `true` once it is gone (or was never there); `false` when the path is refused or the tries run
/// out. Closing the window first is the caller's; this touches no other file in the profile.
pub fn forget(data_dir: &Path) -> bool {
    let dir = session_dir(data_dir);
    if !is_session_dir(data_dir, &dir) {
        return false;
    }
    // `symlink_metadata`, not `exists`: a dangling link still counts as there.
    let gone = |d: &Path| std::fs::symlink_metadata(d).is_err();
    for attempt in 0..WIPE_TRIES {
        if gone(&dir) || std::fs::remove_dir_all(&dir).is_ok() {
            return true;
        }
        if attempt + 1 < WIPE_TRIES {
            std::thread::sleep(WIPE_BACKOFF);
        }
    }
    gone(&dir)
}

// ---- the gated seams (Task 5c; spec §4, §9) --------------------------------------------------
//
// Each command is a thin wrapper: it reads the vault's `config/campus.yaml` ([`campus_of`]), resolves
// the curated row with `scaffold::curated`, and hands both to one of these. The window and the session
// are closures the wrapper passes in, so a refusal is shown, without a window, to call neither.

/// The refusal `grades_connect` and `grades_refresh` return, and the slot's named skip.
pub const NOT_AVAILABLE: &str = "not available at your school yet";
/// The same for a school that is not a Blackboard school at all.
pub const NOT_BLACKBOARD: &str = "not a Blackboard school";
/// `grades_refresh` with no saved session; the slot's `grades (skipped: not connected)`.
pub const NOT_CONNECTED: &str = "not connected";

/// The two fields of `config/campus.yaml` the gate reads, blank when the file, the field or the
/// whole file is missing or unreadable (the school is then no school, never a guess).
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Campus {
    pub unitid: String,
    pub lms: String,
}

/// `config/campus.yaml`, read through `pystr` and `serde_yaml_ng` as `account::feeds_in` reads
/// `ingest.yaml`. A `unitid` written unquoted reads as a number and is taken as its digits.
pub fn campus_of(vault: &Path) -> Campus {
    let Ok(text) = knowlu_engine::pystr::read_text(&vault.join("config").join("campus.yaml")) else { return Campus::default() };
    let Ok(v) = serde_yaml_ng::from_str::<serde_yaml_ng::Value>(&text) else { return Campus::default() };
    let field = |k: &str| match v.get(k) {
        Some(serde_yaml_ng::Value::String(s)) => s.trim().to_string(),
        Some(serde_yaml_ng::Value::Number(n)) => n.to_string(),
        _ => String::new(),
    };
    Campus { unitid: field("unitid"), lms: field("lms") }
}

/// The refusal a predicate that says no names; `None` when grades are available.
fn refusal(a: Availability) -> Option<&'static str> {
    match a {
        Availability::Available { .. } => None,
        Availability::NotAvailableYet => Some(NOT_AVAILABLE),
        Availability::NotBlackboard => Some(NOT_BLACKBOARD),
    }
}

fn refused(reason: &str, available: bool) -> Value {
    json!({ "ok": false, "error": reason, "available": available })
}

/// `<data_dir>\grades-signed-out`: present while the last capture found the saved session expired.
/// App data beside `grades.json`, never in it (that file holds `hidden` only, spec §3).
const SIGNED_OUT_MARK: &str = "grades-signed-out";

/// Record (or clear) that the saved session was found signed out. Best effort: a mark that could
/// not be written only means the strip says *connected* until the next capture.
pub fn set_signed_out(data_dir: &Path, signed_out: bool) {
    let mark = data_dir.join(SIGNED_OUT_MARK);
    if signed_out {
        let _ = std::fs::create_dir_all(data_dir);
        let _ = std::fs::write(mark, "");
    } else {
        let _ = std::fs::remove_file(mark);
    }
}

/// Whether a session has been saved at all: the kept profile's directory exists.
pub fn connected(data_dir: &Path) -> bool {
    std::fs::symlink_metadata(session_dir(data_dir)).is_ok()
}

/// **`grades_status`'s seam** (spec §9): available or not and why; and only when available, the
/// session (`connected`, `signed_out`), the row's `host` and `state/grades.json`'s `fetched_at`.
/// Without a date it says *not available* whatever session directory exists. `hidden` is the
/// student's own preference, reported in every case.
pub fn status_for(row: Option<&Curated>, campus_lms: &str, data_dir: &Path, vault: &Path) -> Value {
    let hidden = GradesPrefs::load(data_dir).hidden;
    let Availability::Available { host } = availability(row, campus_lms) else {
        let reason = refusal(availability(row, campus_lms));
        return json!({ "ok": true, "available": false, "reason": reason, "connected": false, "signed_out": false,
                       "host": null, "fetched_at": null, "hidden": hidden });
    };
    let is_connected = connected(data_dir);
    let signed_out = is_connected && data_dir.join(SIGNED_OUT_MARK).is_file();
    json!({ "ok": true, "available": true, "reason": null, "connected": is_connected, "signed_out": signed_out,
            "host": host, "fetched_at": fetched_at(vault), "hidden": hidden })
}

/// `state/grades.json`'s `fetched_at` (the engine's, spec §6), or `None` before the first run.
fn fetched_at(vault: &Path) -> Option<String> {
    let text = std::fs::read_to_string(vault.join(knowlu_engine::grades::STATE_FILE)).ok()?;
    let v: Value = serde_json::from_str(&text).ok()?;
    v.get("fetched_at")?.as_str().map(str::to_string)
}

/// **`grades_connect`'s seam.** The predicate first; a refusal returns before either action. Then
/// `open_window` (the visible sign-in window, on the row's host) and, only if it opened,
/// `read_session` (detection, the cookies, the first capture), whose answer is returned as is.
pub fn connect_with(
    row: Option<&Curated>,
    campus_lms: &str,
    open_window: impl FnOnce(&'static str) -> Result<(), String>,
    read_session: impl FnOnce(&'static str) -> Value,
) -> Value {
    let a = availability(row, campus_lms);
    let Availability::Available { host } = a else { return refused(refusal(a).unwrap_or(NOT_AVAILABLE), false) };
    match open_window(host) {
        Ok(()) => read_session(host),
        Err(e) => refused(&e, true),
    }
}

/// **`grades_refresh`'s seam.** The predicate first, then `not connected` when no session was ever
/// saved (a hidden window would otherwise create an empty profile and look connected); only then
/// `open_window` (the hidden window) and `read_session` (settle, cookies, capture).
pub fn refresh_with(
    row: Option<&Curated>,
    campus_lms: &str,
    data_dir: &Path,
    open_window: impl FnOnce(&'static str) -> Result<(), String>,
    read_session: impl FnOnce(&'static str) -> Value,
) -> Value {
    let a = availability(row, campus_lms);
    let Availability::Available { host } = a else { return refused(refusal(a).unwrap_or(NOT_AVAILABLE), false) };
    if !connected(data_dir) {
        return refused(NOT_CONNECTED, true);
    }
    match open_window(host) {
        Ok(()) => read_session(host),
        Err(e) => refused(&e, true),
    }
}

/// **`grades_forget`'s seam: never gated** (spec §4). Close any `lms-grades` window, delete the
/// saved session ([`forget`]) and clear the signed-out mark, whatever the predicate says.
pub fn forget_with(data_dir: &Path, close_window: impl FnOnce()) -> Value {
    close_window();
    let gone = forget(data_dir);
    set_signed_out(data_dir, false);
    if gone {
        json!({ "ok": true, "error": null })
    } else {
        json!({ "ok": false, "error": "the saved Blackboard sign-in could not be deleted yet; try again in a moment" })
    }
}

// ---- the window half (Task 5c; spec §4) -------------------------------------------------------
//
// One label, `lms-grades`, on the persistent profile at `session_dir`: visible for the sign-in,
// hidden for a refresh. The cookies are read from the window for one capture, live on the stack of
// that capture only, and are never written, logged or put in an error; so are the rows it reads.
use knowlu_engine::childproc::NoConsole;
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::{Duration, Instant};
use tauri::{Manager, WebviewUrl, WebviewWindowBuilder};

/// A hidden refresh waits this long for a live SSO session to land back on the LMS host.
pub const SETTLE: Duration = Duration::from_secs(45);
/// A visible sign-in is given this long (SSO and MFA) before the window is closed.
const SIGN_IN_LIMIT: Duration = Duration::from_secs(15 * 60);
const POLL: Duration = Duration::from_secs(1);

/// Why a capture produced no bundle. Each is a named outcome, never an error (spec §10).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CaptureError {
    SignedOut,
    Unreachable,
    WindowOpen,
    /// Another capture (a Connect, a Refresh or the slot's) holds the window right now.
    Busy,
}

impl CaptureError {
    pub fn as_str(self) -> &'static str {
        match self {
            CaptureError::SignedOut => "signed out",
            CaptureError::Unreachable => "Blackboard unreachable",
            CaptureError::WindowOpen => "sign-in window open",
            CaptureError::Busy => "another grades capture is running",
        }
    }
}

/// One capture at a time in this process: the window label is one, and so is the session.
static CAPTURING: AtomicBool = AtomicBool::new(false);

/// Held for a capture's whole run; dropping it frees the next.
pub struct Capturing(());

impl Capturing {
    pub fn try_take() -> Option<Capturing> {
        CAPTURING.compare_exchange(false, true, Ordering::SeqCst, Ordering::SeqCst).ok().map(|_| Capturing(()))
    }
}

impl Drop for Capturing {
    fn drop(&mut self) {
        CAPTURING.store(false, Ordering::SeqCst);
    }
}

/// `lms_link::session_agent`'s shape: one agent, thirty seconds global, no jar of its own.
fn agent() -> ureq::Agent {
    ureq::Agent::config_builder().timeout_global(Some(Duration::from_secs(30))).build().into()
}

/// One GET with the session's cookies: the body, or the status code, or `unreachable`. Only those
/// fixed words leave here; the cookies, the URL and the body never enter an error.
fn get(agent: &ureq::Agent, url: &str, jar: &str) -> Result<String, String> {
    agent
        .get(url)
        .header("cookie", jar)
        .header("accept", "application/json")
        .call()
        .and_then(|mut r| r.body_mut().with_config().limit(1 << 22).read_to_string())
        .map_err(|e| match e {
            ureq::Error::StatusCode(n) => n.to_string(),
            _ => "unreachable".to_string(),
        })
}

/// `users/me`'s body, or its status (`None` when unreachable or not JSON).
fn users_me(agent: &ureq::Agent, host: &str, jar: &str) -> Result<Value, Option<u16>> {
    match get(agent, &format!("https://{host}/learn/api/public/v1/users/me"), jar) {
        Ok(body) => serde_json::from_str(&body).map_err(|_| None),
        Err(code) => Err(code.parse().ok()),
    }
}

/// The window's cookies as one `cookie` header, by `lms_link::capture_courses`'s own rule
/// (`lms_link::cookie_url`): the window's URL while it is on the endpoint's host, else the endpoint.
fn jar_for(w: &tauri::WebviewWindow, host: &str) -> Option<String> {
    let target = format!("https://{host}/learn/api/public/v1/users/me");
    let here = w.url().ok();
    let jar_url = crate::lms_link::cookie_url(here.as_ref().map(|u| u.as_str()), &target).parse::<tauri::Url>().ok()?;
    let cookies = w.cookies_for_url(jar_url).ok()?;
    Some(cookies.iter().map(|c| format!("{}={}", c.name(), c.value())).collect::<Vec<_>>().join("; "))
}

/// A Blackboard primary id that may sit in a URL path as it is (`_4686399_1`).
fn path_safe(id: &str) -> bool {
    !id.is_empty() && id.len() <= 64 && id.chars().all(|c| c.is_ascii_alphanumeric() || matches!(c, '_' | '-' | '.' | ':'))
}

/// **§3's calls** with one jar, into one bundle: the memberships, then per available course its
/// columns, the student's grades and the categories. The memberships refused with 401 or 403 is
/// signed out; any other failure there is unreachable. A course's own failure is its `error`.
fn capture(agent: &ureq::Agent, host: &str, jar: &str, user: &Value) -> Result<Value, CaptureError> {
    let fetch = |url: &str| get(agent, url, jar);
    let api = |path: String| format!("https://{host}/learn/api/public/{path}");
    let memberships = read_list(host, &api("v1/users/me/courses?expand=course".into()), fetch).map_err(|e| match e.as_str() {
        "401" | "403" => CaptureError::SignedOut,
        _ => CaptureError::Unreachable,
    })?;
    let user_id = user.get("id").and_then(Value::as_str).filter(|id| path_safe(id)).ok_or(CaptureError::Unreachable)?;
    let mut per_course = BTreeMap::new();
    for m in available_memberships(&memberships).map_err(|_| CaptureError::Unreachable)? {
        let Some(id) = membership_course_id(&m).filter(|id| path_safe(id)) else { continue };
        let calls = CourseCalls {
            columns: read_list(host, &api(format!("v2/courses/{id}/gradebook/columns")), fetch),
            grades: read_list(host, &api(format!("v2/courses/{id}/gradebook/users/{user_id}")), fetch),
            categories: read_list(host, &api(format!("v1/courses/{id}/gradebook/categories")), fetch),
        };
        per_course.insert(id.to_string(), calls);
    }
    let fetched_at = knowlu_engine::journal::now_ts(None);
    let head = CaptureHead { lms: "blackboard", host, fetched_at: &fetched_at };
    assemble_bundle(&head, user, &memberships, &per_course).map_err(|_| CaptureError::Unreachable)
}

/// **Connect's window** (spec §4): `lms-grades`, visible, at `https://<host>/`, on the kept profile.
pub fn open_visible(app: &tauri::AppHandle, data_dir: &Path, host: &str) -> Result<(), String> {
    if app.get_webview_window(WINDOW).is_some() {
        return Err(CaptureError::WindowOpen.as_str().into());
    }
    let url: tauri::Url = format!("https://{host}/").parse().map_err(|_| "the LMS address is not a URL".to_string())?;
    std::fs::create_dir_all(data_dir).map_err(|e| e.to_string())?;
    WebviewWindowBuilder::new(app, WINDOW, WebviewUrl::External(url))
        .title("Sign in to Blackboard")
        .inner_size(1000.0, 760.0)
        .center()
        .data_directory(session_dir(data_dir))
        .build()
        .map(|_| ())
        .map_err(|e| e.to_string())
}

/// **Detection, then the first capture** (spec §15, signed default 1): poll the window until it is
/// on the LMS host and `users/me` answers 200 with its cookies, close it, and capture with that jar.
/// A window the student closed first, or one still signing in at the limit, is signed out.
pub fn await_sign_in(app: &tauri::AppHandle, host: &str) -> Result<Value, CaptureError> {
    let agent = agent();
    let deadline = Instant::now() + SIGN_IN_LIMIT;
    loop {
        let Some(w) = app.get_webview_window(WINDOW) else { return Err(CaptureError::SignedOut) };
        let here = w.url().ok().map(|u| u.to_string());
        if here.as_deref().is_some_and(|u| on_host(u, host)) {
            if let Some(jar) = jar_for(&w, host) {
                let me = users_me(&agent, host, &jar);
                let status = match &me { Ok(_) => Some(200), Err(code) => *code };
                if let (true, Ok(user)) = (signed_in(here.as_deref(), host, status), me) {
                    let _ = w.destroy();
                    return capture(&agent, host, &jar, &user);
                }
            }
        }
        if Instant::now() >= deadline {
            let _ = w.destroy();
            return Err(CaptureError::SignedOut);
        }
        std::thread::sleep(POLL);
    }
}

/// **Refresh's window**: `lms-grades`, hidden and out of the taskbar, at `https://<host>/ultra/`.
pub fn open_hidden(app: &tauri::AppHandle, data_dir: &Path, host: &str) -> Result<(), CaptureError> {
    if app.get_webview_window(WINDOW).is_some() {
        return Err(CaptureError::WindowOpen);
    }
    let url: tauri::Url = format!("https://{host}/ultra/").parse().map_err(|_| CaptureError::Unreachable)?;
    WebviewWindowBuilder::new(app, WINDOW, WebviewUrl::External(url))
        .title("Blackboard")
        .visible(false)
        .focused(false)
        .skip_taskbar(true)
        .data_directory(session_dir(data_dir))
        .build()
        .map(|_| ())
        .map_err(|_| CaptureError::Unreachable)
}

/// **The hidden refresh's read**: up to [`SETTLE`] for the window to rest on the LMS host with
/// `users/me` answering 200, then capture. The window is destroyed whatever happens. `users/me`
/// answering 401 or 403, or the window never reaching the LMS host, is signed out; anything else
/// (no answer, a 5xx) is unreachable.
pub fn read_hidden(app: &tauri::AppHandle, host: &str) -> Result<Value, CaptureError> {
    let Some(w) = app.get_webview_window(WINDOW) else { return Err(CaptureError::Unreachable) };
    let agent = agent();
    let deadline = Instant::now() + SETTLE;
    let (mut reached, mut last) = (false, None);
    let result = loop {
        let here = w.url().ok().map(|u| u.to_string());
        if here.as_deref().is_some_and(|u| on_host(u, host)) {
            reached = true;
            if let Some(jar) = jar_for(&w, host) {
                match users_me(&agent, host, &jar) {
                    Ok(user) => break capture(&agent, host, &jar, &user),
                    Err(code) => last = code,
                }
            }
        }
        if Instant::now() >= deadline {
            break Err(match last {
                _ if !reached => CaptureError::SignedOut,
                Some(401 | 403) => CaptureError::SignedOut,
                _ => CaptureError::Unreachable,
            });
        }
        std::thread::sleep(POLL);
    };
    let _ = w.destroy();
    result
}

/// The whole hidden capture, for a caller that has already checked [`availability`] (the slot's
/// grades step, Task 6): one capture at a time, then [`open_hidden`] and [`read_hidden`].
pub fn capture_hidden(app: &tauri::AppHandle, data_dir: &Path, host: &str) -> Result<Value, CaptureError> {
    let Some(_capturing) = Capturing::try_take() else { return Err(busy_or_open(app)) };
    open_hidden(app, data_dir, host)?;
    read_hidden(app, host)
}

/// A capture that could not start: the visible sign-in window is open, or another capture runs.
fn busy_or_open(app: &tauri::AppHandle) -> CaptureError {
    match app.get_webview_window(WINDOW) {
        Some(w) if w.is_visible().unwrap_or(false) => CaptureError::WindowOpen,
        _ => CaptureError::Busy,
    }
}

// ---- the bundle handed to the engine (spec §5, §6) ---------------------------------------------

/// `<profile app data>\grades-capture.json`: never the vault, never `%TEMP%`.
pub fn bundle_path(data_dir: &Path) -> PathBuf {
    data_dir.join("grades-capture.json")
}

/// A bundle path private to one slot capture, beside [`bundle_path`] and never that file: the console's
/// Refresh writes and deletes `grades-capture.json` while the slot's engine step is still to run
/// (after sync, coursework and ingest), so sharing it would let either side delete or replace the
/// other's bundle (spec §5, §10). Unique per process and per call.
pub fn slot_bundle_path(data_dir: &Path) -> PathBuf {
    static SEQ: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(0);
    let n = SEQ.fetch_add(1, Ordering::SeqCst);
    data_dir.join(format!("grades-capture-slot-{}-{n}.json", std::process::id()))
}

/// `knowlu-engine grades --vault <v> --input <bundle> --via <via>`.
pub fn grades_argv(vault: &Path, input: &Path, via: &str) -> Vec<String> {
    let s = |p: &Path| p.to_string_lossy().into_owned();
    vec!["grades".into(), "--vault".into(), s(vault), "--input".into(), s(input), "--via".into(), via.into()]
}

/// Write the bundle, run the engine's `grades` step as a child process, and delete the bundle
/// whatever the outcome. No vault lock is held across the child: the engine writes through `write`,
/// and the rows never pass through this process's logs. The answer carries the engine's own lines,
/// which name counts, course codes and error codes only (spec §6, §11).
pub fn run_grades_step(vault: &Path, data_dir: &Path, bundle: &Value, via: &str) -> Value {
    let path = bundle_path(data_dir);
    let ran = std::fs::create_dir_all(data_dir)
        .and_then(|()| std::fs::write(&path, knowlu_engine::ledger::dumps_value(bundle)))
        .map_err(|e| format!("the capture could not be saved ({})", e.kind()))
        .and_then(|()| crate::scheduler::engine_exe())
        .and_then(|exe| {
            std::process::Command::new(exe).no_console().args(grades_argv(vault, &path, via)).output().map_err(|e| format!("could not run the engine ({})", e.kind()))
        });
    let _ = std::fs::remove_file(&path);
    let lines = |b: &[u8]| String::from_utf8_lossy(b).lines().map(str::to_string).filter(|l| !l.is_empty()).collect::<Vec<_>>();
    match ran {
        Ok(out) if out.status.success() => json!({ "ok": true, "error": null, "lines": lines(&out.stdout) }),
        Ok(out) => json!({ "ok": false, "error": lines(&out.stderr).join("; "), "lines": [] }),
        Err(e) => json!({ "ok": false, "error": e, "lines": [] }),
    }
}

/// A capture's outcome as a command's answer: a bundle goes to the engine and clears the signed-out
/// mark; signed out sets it; every other outcome is its name.
fn finish(vault: &Path, data_dir: &Path, captured: Result<Value, CaptureError>) -> Value {
    match captured {
        Ok(bundle) => {
            set_signed_out(data_dir, false);
            run_grades_step(vault, data_dir, &bundle, "dashboard")
        }
        Err(e) => {
            if e == CaptureError::SignedOut {
                set_signed_out(data_dir, true);
            }
            json!({ "ok": false, "error": e.as_str(), "lines": [] })
        }
    }
}

// ---- the four commands (spec §9): thin wrappers over the seams ---------------------------------
//
// Each is `#[tauri::command(async)]`: a synchronous command that builds and waits on a window blocks
// the main thread on Windows. `grades_connect` and `grades_refresh` check the predicate (inside
// their seam) before anything else; `grades_forget` is never gated.
use crate::state::ConsoleState;
use tauri::State;

/// The vault's curated row (`scaffold::curated` of `config/campus.yaml`'s `unitid`) and its `lms`:
/// the predicate's inputs, and the only place the commands resolve them.
fn gate_inputs(vault: &Path) -> (Option<&'static Curated>, String) {
    let campus = campus_of(vault);
    (crate::scaffold::curated(&campus.unitid), campus.lms)
}

#[tauri::command(async)]
pub fn grades_status(cs: State<'_, ConsoleState>) -> Value {
    let (row, lms) = gate_inputs(&cs.vault);
    status_for(row, &lms, &cs.data_dir, &cs.vault)
}

#[tauri::command(async)]
pub fn grades_connect(app: tauri::AppHandle, cs: State<'_, ConsoleState>) -> Value {
    let (row, lms) = gate_inputs(&cs.vault);
    let held = std::cell::RefCell::new(None);
    connect_with(
        row,
        &lms,
        |host| {
            *held.borrow_mut() = Some(Capturing::try_take().ok_or_else(|| busy_or_open(&app).as_str().to_string())?);
            open_visible(&app, &cs.data_dir, host)
        },
        |host| finish(&cs.vault, &cs.data_dir, await_sign_in(&app, host)),
    )
}

#[tauri::command(async)]
pub fn grades_refresh(app: tauri::AppHandle, cs: State<'_, ConsoleState>) -> Value {
    let (row, lms) = gate_inputs(&cs.vault);
    let held = std::cell::RefCell::new(None);
    refresh_with(
        row,
        &lms,
        &cs.data_dir,
        |host| {
            *held.borrow_mut() = Some(Capturing::try_take().ok_or_else(|| busy_or_open(&app).as_str().to_string())?);
            open_hidden(&app, &cs.data_dir, host).map_err(|e| e.as_str().to_string())
        },
        |host| finish(&cs.vault, &cs.data_dir, read_hidden(&app, host)),
    )
}

#[tauri::command(async)]
pub fn grades_forget(app: tauri::AppHandle, cs: State<'_, ConsoleState>) -> Value {
    forget_with(&cs.data_dir, || {
        if let Some(w) = app.get_webview_window(WINDOW) {
            let _ = w.destroy();
        }
    })
}
