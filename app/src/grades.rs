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
/// runaway `nextPage` loop is impossible.
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
pub fn read_list(host: &str, first: &str, mut fetch: impl FnMut(&str) -> Result<String, String>) -> Result<Vec<Value>, String> {
    if !on_host(first, host) {
        return Err("not on the LMS host".into());
    }
    let mut pages = Vec::new();
    let mut url = first.to_string();
    while pages.len() < MAX_PAGES {
        let page: Value = serde_json::from_str(&fetch(&url)?).map_err(|_| "unreadable".to_string())?;
        let next = next_page(host, &page);
        pages.push(page);
        match next {
            Some(n) => url = n,
            None => break,
        }
    }
    Ok(pages)
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
