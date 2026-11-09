//! The LMS's one persistent sign-in folder, its uses and its per-profile lock (onboarding plan §S2;
//! replan RQ4 and rows S1 and S2).
//!
//! **What the folder is.** `<data_dir>\school-session`, a WebView2 profile the LMS windows keep their
//! sign-in in. It belongs to the LMS alone: grades and the course list read through it. The registrar
//! and the school mailbox never do (two folders, RQ4; the mailbox keeps its own `mail-session-<id>`).
//! **When it goes.** [`forget`] deletes it; [`clear_use`] of the last use deletes it. **What it is not.**
//! Never synced, never in the vault, and it holds no password: that is Credential Manager's.
//!
//! `school.json` records which uses are live. It sits in `data_dir` beside the folder, never inside it,
//! because the folder is deleted wholesale. It is read whole and rewritten whole under one process-wide
//! lock, and one it cannot read is an error, never "no uses".
//!
//! The lock is per profile and never blocks: [`try_take`] answers `None` when the profile is busy. The
//! hidden window's guard stays [`Capturing`], process-wide, because window labels are not per profile;
//! [`take_both`] takes the session lock first, then `Capturing`.
use crate::grades::{remove_dir_patiently, CaptureError, Capturing};
use crate::scaffold::Curated;
use serde_json::{json, Value};
use std::collections::HashSet;
use std::path::{Path, PathBuf};
use std::sync::Mutex;

const SESSION_DIR: &str = "school-session";
const USES_FILE: &str = "school.json";
/// Today's grades folder, kept for an install that signed in before the school folder existed.
const LEGACY_GRADES_DIR: &str = "lms-session";

/// What reads through the folder. Replan RQ4: the LMS's alone, so grades and the course list.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum Use {
    Grades,
    CourseList,
}

impl Use {
    fn name(self) -> &'static str {
        match self {
            Use::Grades => "grades",
            Use::CourseList => "course_list",
        }
    }
    fn parse(s: &str) -> Option<Use> {
        [Use::Grades, Use::CourseList].into_iter().find(|u| u.name() == s)
    }
}

/// Who asks for a folder.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Feature {
    Grades,
    LmsCapture,
    Registrar,
    SchoolMail,
}

// ---- the per-profile lock ---------------------------------------------------------------------

/// The profiles whose folder is held right now. The critical section is one set operation, so taking
/// the mutex never waits on anything a caller does.
static HELD: Mutex<Option<HashSet<String>>> = Mutex::new(None);

/// Held while a caller owns one profile's school folder; dropping it frees the profile.
pub struct SessionGuard(String);

impl SessionGuard {
    /// The profile this guard holds, so a callee can check the guard it is handed (read-only).
    pub fn profile_id(&self) -> &str {
        &self.0
    }
}

impl Drop for SessionGuard {
    fn drop(&mut self) {
        let mut held = HELD.lock().unwrap_or_else(|e| e.into_inner());
        if let Some(set) = held.as_mut() {
            set.remove(&self.0);
        }
    }
}

/// `None` when this profile's folder is held; another profile's is never affected.
pub fn try_take(profile_id: &str) -> Option<SessionGuard> {
    let mut held = HELD.lock().unwrap_or_else(|e| e.into_inner());
    held.get_or_insert_with(HashSet::new).insert(profile_id.to_string()).then(|| SessionGuard(profile_id.to_string()))
}

/// Session lock first, then `Capturing`; either busy is `Busy` with the first guard dropped.
pub fn take_both(profile_id: &str) -> Result<(SessionGuard, Capturing), CaptureError> {
    let session = try_take(profile_id).ok_or(CaptureError::Busy)?;
    let capturing = Capturing::try_take().ok_or(CaptureError::Busy)?; // `session` drops on the way out
    Ok((session, capturing))
}

// ---- the folder -------------------------------------------------------------------------------

pub fn session_dir(data_dir: &Path) -> PathBuf {
    data_dir.join(SESSION_DIR)
}

/// The one directory [`forget`] will delete: exactly `session_dir(data_dir)` for an absolute
/// `data_dir` with no `..` in it.
pub fn is_session_dir(data_dir: &Path, dir: &Path) -> bool {
    data_dir.is_absolute()
        && !data_dir.components().any(|c| c == std::path::Component::ParentDir)
        && dir == session_dir(data_dir)
}

/// The patient delete of the folder alone; `Err` when the path is refused or the tries run out.
fn wipe_folder(data_dir: &Path) -> Result<(), String> {
    let dir = session_dir(data_dir);
    if !is_session_dir(data_dir, &dir) {
        return Err("that is not the school sign-in folder".to_string());
    }
    if remove_dir_patiently(&dir) {
        Ok(())
    } else {
        Err("the school sign-in could not be deleted yet; try again in a moment".to_string())
    }
}

/// Patient delete of the folder and `school.json`'s uses. `Err` when refused or not finished.
pub fn forget(data_dir: &Path) -> Result<(), String> {
    let _file = USES_LOCK.lock().unwrap_or_else(|e| e.into_inner());
    wipe_folder(data_dir)?;
    match std::fs::remove_file(data_dir.join(USES_FILE)) {
        Ok(()) => Ok(()),
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(()),
        Err(e) => Err(format!("the record of the school sign-in's uses could not be cleared: {e}")),
    }
}

// ---- uses (school.json) -----------------------------------------------------------------------

/// Every read-modify-write of `school.json` runs under this, and reloads inside it.
static USES_LOCK: Mutex<()> = Mutex::new(());

/// The recorded uses in the order written; no file is no uses, an unreadable one is an error.
fn read_uses(data_dir: &Path) -> Result<Vec<Use>, String> {
    let text = match std::fs::read_to_string(data_dir.join(USES_FILE)) {
        Ok(t) => t,
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => return Ok(Vec::new()),
        Err(e) => return Err(format!("school.json could not be read: {e}")),
    };
    let bad = |what: &str| format!("school.json is not readable ({what})");
    let v: Value = serde_json::from_str(&text).map_err(|_| bad("not JSON"))?;
    let list = v.get("uses").and_then(Value::as_array).ok_or_else(|| bad("no list of uses"))?;
    let mut out = Vec::new();
    for item in list {
        let u = item.as_str().and_then(Use::parse).ok_or_else(|| bad("an unknown use"))?;
        if !out.contains(&u) {
            out.push(u);
        }
    }
    Ok(out)
}

fn write_uses(data_dir: &Path, uses: &[Use]) -> Result<(), String> {
    let names: Vec<&str> = uses.iter().map(|u| u.name()).collect();
    let text = knowlu_engine::ledger::dumps_value(&json!({ "uses": names }));
    let tmp = data_dir.join(format!("{USES_FILE}.tmp"));
    std::fs::create_dir_all(data_dir).map_err(|e| format!("school.json could not be written: {e}"))?;
    std::fs::write(&tmp, text).map_err(|e| format!("school.json could not be written: {e}"))?;
    std::fs::rename(&tmp, data_dir.join(USES_FILE)).map_err(|e| {
        let _ = std::fs::remove_file(&tmp);
        format!("school.json could not be written: {e}")
    })
}

pub fn uses(data_dir: &Path) -> Result<Vec<Use>, String> {
    let _file = USES_LOCK.lock().unwrap_or_else(|e| e.into_inner());
    read_uses(data_dir)
}

pub fn add_use(data_dir: &Path, u: Use) -> Result<(), String> {
    let _file = USES_LOCK.lock().unwrap_or_else(|e| e.into_inner());
    let mut now = read_uses(data_dir)?;
    if now.contains(&u) {
        return Ok(());
    }
    now.push(u);
    write_uses(data_dir, &now)
}

/// Clearing the last use deletes the folder.
pub fn clear_use(data_dir: &Path, u: Use) -> Result<(), String> {
    let _file = USES_LOCK.lock().unwrap_or_else(|e| e.into_inner());
    let mut now = read_uses(data_dir)?;
    now.retain(|x| *x != u);
    if now.is_empty() {
        // The folder first: a delete that cannot finish leaves the use recorded, so a retry still finds it.
        wipe_folder(data_dir)?;
    }
    write_uses(data_dir, &now)
}

// ---- who gets the folder ----------------------------------------------------------------------

/// The folder a feature's window is built on; `None` for a feature with no folder of this module's.
/// **Grades keep today's `lms-session` folder** while it exists, until a `Use::Grades` is recorded
/// (S2's review): a capture window opened and closed unsigned leaves a bare school folder, and that
/// must not move grades off a working sign-in. With no old folder, grades read through the school's.
/// This never writes or deletes; [`grades_profile`] is the one that records the use.
pub fn profile_for(feature: Feature, _row: Option<&Curated>, data_dir: &Path) -> Option<PathBuf> {
    match feature {
        Feature::Grades => {
            let legacy = data_dir.join(LEGACY_GRADES_DIR);
            // A `school.json` that cannot be read records nothing, so the old folder, if any, stays.
            let moved = uses(data_dir).is_ok_and(|u| u.contains(&Use::Grades));
            Some(if legacy.exists() && !moved { legacy } else { session_dir(data_dir) })
        }
        Feature::LmsCapture => Some(session_dir(data_dir)),
        Feature::Registrar | Feature::SchoolMail => None,
    }
}

// ---- the campus fields' rules -----------------------------------------------------------------

/// A `sign_in_hosts` entry: a bare host name, no scheme, path, port or whitespace.
pub fn sign_in_host_ok(host: &str) -> bool {
    !host.is_empty()
        && host.split('.').all(|label| {
            !label.is_empty()
                && !label.starts_with('-')
                && !label.ends_with('-')
                && label.bytes().all(|b| b.is_ascii_alphanumeric() || b == b'-')
        })
}

/// A `wrong_password_text`: empty, or fixed text with no `@` and no digits-only token.
pub fn wrong_password_text_ok(text: &str) -> bool {
    !text.contains('@')
        && !text.split(|c: char| !c.is_alphanumeric()).any(|t| !t.is_empty() && t.chars().all(char::is_numeric))
}

// ---- the windows on the folder (S3's seams; S4a and S4b fill them) -------------------------------

/// The one arguments string every window on the school folder is built with: the mail window's, since
/// S-G recorded no change. It never carries a debug port.
pub const SCHOOL_BROWSER_ARGS: &str = crate::mail::MAIL_BROWSER_ARGS;

/// A school window's browser arguments. The debug port is a parameter only so the rule is testable
/// without the environment: a window on the school folder gets [`SCHOOL_BROWSER_ARGS`] whatever port is
/// passed, in every build. The mail window's own port rule (`crate::debug_port`) is not this one's.
pub fn window_args(_debug_port: Option<u16>) -> &'static str {
    SCHOOL_BROWSER_ARGS
}

/// One cookie a window's jar returned, for the fakes of the S3 cookie tests. `domain` is the cookie's own
/// `Domain` attribute, which no reader may send a request to: a request goes to the host the reader was
/// given, and the jar is asked for that host alone.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct JarCookie {
    pub name: String,
    pub value: String,
    pub domain: String,
}

/// The cookies as one `cookie` header: `name=value` pairs only, never a `Domain`.
pub fn cookie_header(cookies: &[JarCookie]) -> String {
    cookies.iter().map(|c| format!("{}={}", c.name, c.value)).collect::<Vec<_>>().join("; ")
}

/// A real window's jar for one URL, as the readers' `query_jar` closures take it. A URL that does not
/// parse, or a jar WebView2 will not give, answers no cookies and sets [`WindowJar::failed`], so the
/// caller's request closure can refuse to send and the caller can name the failure.
pub struct WindowJar<'a> {
    window: &'a tauri::WebviewWindow,
    failed: std::cell::Cell<bool>,
}

impl<'a> WindowJar<'a> {
    pub fn new(window: &'a tauri::WebviewWindow) -> Self {
        WindowJar { window, failed: std::cell::Cell::new(false) }
    }
    pub fn query(&self, url: &str) -> Vec<JarCookie> {
        let read = url.parse::<tauri::Url>().ok().and_then(|u| self.window.cookies_for_url(u).ok());
        match read {
            Some(cookies) => cookies
                .iter()
                .map(|c| JarCookie { name: c.name().to_string(), value: c.value().to_string(), domain: c.domain().unwrap_or("").to_string() })
                .collect(),
            None => {
                self.failed.set(true);
                Vec::new()
            }
        }
    }
    /// Whether any read since the last [`WindowJar::reset`] could not be made.
    pub fn failed(&self) -> bool {
        self.failed.get()
    }
    pub fn reset(&self) {
        self.failed.set(false);
    }
}

/// The key the per-profile lock is taken under: the profile folder's own name (`profiles\<id>`), which
/// is the profile's id. A path with no name falls back to the whole path, so two profiles never share one.
pub fn profile_key(data_dir: &Path) -> String {
    data_dir.file_name().map_or_else(|| data_dir.to_string_lossy().into_owned(), |n| n.to_string_lossy().into_owned())
}

/// The folder grades' window is built on, and the use that choice records: [`profile_for`]'s answer for
/// `Feature::Grades` and, whenever that is [`session_dir`], `add_use(Grades)` before returning, so a later
/// `clear_use(CourseList)` can never delete the folder grades read through. It moves off `lms-session`
/// only on a recorded `Use::Grades`; once it has, the abandoned `lms-session` is forgotten (Q-B4), best
/// effort: a delete that cannot finish is tried again at the next window. Window-opening callers only:
/// status and "connected" ask [`profile_for`], which writes and deletes nothing.
pub fn grades_profile(data_dir: &Path) -> Result<PathBuf, String> {
    let profile = profile_for(Feature::Grades, None, data_dir).ok_or_else(|| "no folder for grades".to_string())?;
    if profile == session_dir(data_dir) {
        add_use(data_dir, Use::Grades)?;
        if data_dir.join(LEGACY_GRADES_DIR).exists() {
            let _ = crate::grades::forget(data_dir);
        }
    }
    Ok(profile)
}

// ---- Settings' one school sign-in row (S6a; tests: `app/tests/school_session.rs`, S6t) -------------

/// What `school_status` answers: `{ "uses": [name], "folder": bool, "label": string|null }`. The uses are
/// [`Use`]'s names in the order recorded, `folder` whether a school sign-in is kept in either form
/// (`school-session`, or an old install's `lms-session`), `label` the curated
/// school's own name. Never a path, a cookie or a host. A `school.json` that cannot be read answers no uses
/// (the folder flag still tells the row there is something to Forget).
pub fn status_with(data_dir: &Path, row: Option<&Curated>) -> Value {
    let names: Vec<&str> = uses(data_dir).unwrap_or_default().into_iter().map(Use::name).collect();
    json!({
        "uses": names,
        "folder": session_dir(data_dir).exists() || data_dir.join(LEGACY_GRADES_DIR).exists(),
        "label": row.map(|r| r.label),
    })
}

/// What `school_forget` does, over injected seams (`close_window` is handed each label to close), with no
/// wait: a Busy lock answers Busy at once. The command waits through [`school_forget_retrying`]. In the
/// order the replan binds (§4.5, S6a):
/// 1. close every window on the school folders by label ([`crate::account::SCHOOL_WINDOWS`]). That
///    includes `lms-signin`, which the registrar's throwaway window uses too, so a registrar read in
///    progress is stopped, as sign-out does;
/// 2. P1's `forget_with` (the password, then the marks), **before** this call takes its own guard,
///    because it takes the profile's lock itself. Busy there is `ok: false` and nothing else touched;
///    any other failure is remembered, and the rest still runs (forget as much as it can);
/// 3. take the guard, then forget the school folder and every use ([`forget`]) and the legacy
///    `lms-session` ([`crate::grades::forget`]), each tried whatever the other answered, and clear
///    grades' signed-out mark.
/// Reply `{ "ok": bool, "error": string|null }`; the error names the first thing that failed and never
/// carries the secret.
pub fn school_forget_with(
    data_dir: &Path,
    profile_id: &str,
    store: &dyn crate::school_password::SchoolStore,
    marks: &dyn crate::school_password::SchoolMarks,
    close_window: &mut dyn FnMut(&str),
) -> Value {
    school_forget_retrying(data_dir, profile_id, store, marks, close_window, &mut || {}, &mut |id| try_take(id))
}

/// Eight pauses of 250 ms: the two seconds Forget waits for a window's lock to drop.
pub const FORGET_RETRIES: u32 = 8;
const FORGET_PAUSE: std::time::Duration = std::time::Duration::from_millis(250);

/// [`school_forget_with`] with its two waits injected. A window the windows' close has just destroyed
/// holds the profile lock until its Destroyed event runs on the event loop, so both takes (P1's own, and
/// `take` for the folder's guard) are tried again after each `pause`, up to [`FORGET_RETRIES`] pauses,
/// before they answer Busy. The windows are closed once. A Busy at P1's step touches nothing. A Busy at
/// the folder's step comes after the password was forgotten, and the reply says so.
pub fn school_forget_retrying(
    data_dir: &Path,
    profile_id: &str,
    store: &dyn crate::school_password::SchoolStore,
    marks: &dyn crate::school_password::SchoolMarks,
    close_window: &mut dyn FnMut(&str),
    pause: &mut dyn FnMut(),
    take: &mut dyn FnMut(&str) -> Option<SessionGuard>,
) -> Value {
    use crate::school_password::{forget_with, BUSY};
    let reply = |ok: bool, error: Option<String>| json!({ "ok": ok, "error": error });
    for label in crate::account::SCHOOL_WINDOWS {
        close_window(label);
    }
    let mut first_error: Option<String> = None;
    let mut tries = 0;
    loop {
        match forget_with(store, marks, profile_id) {
            Err(e) if e == BUSY && tries < FORGET_RETRIES => {
                tries += 1;
                pause();
            }
            Err(e) if e == BUSY => return reply(false, Some(e)),
            Err(e) => {
                first_error = Some(e);
                break;
            }
            Ok(()) => break,
        }
    }
    let mut held = take(profile_id);
    for _ in 0..FORGET_RETRIES {
        if held.is_some() {
            break;
        }
        pause();
        held = take(profile_id);
    }
    let Some(_held) = held else {
        let in_use = "the school sign-in itself is in use right now. Press Forget again in a moment to finish.";
        let said = match first_error {
            Some(e) => format!("{e} Also, {in_use}"),
            None => format!("Your saved school password is forgotten, but {in_use}"),
        };
        return reply(false, Some(said));
    };
    if let Err(e) = forget(data_dir) {
        first_error.get_or_insert(e);
    }
    if !crate::grades::forget(data_dir) {
        first_error.get_or_insert_with(|| "the saved Blackboard sign-in could not be deleted yet; try again in a moment".to_string());
    }
    // This row replaces grades' own Forget at a shared school, so it clears the same mark.
    crate::grades::set_signed_out(data_dir, false);
    reply(first_error.is_none(), first_error)
}

/// The row's status for the console profile's campus. Thin: `status_with` over the profile's own folder.
#[tauri::command(async)]
pub fn school_status(cs: tauri::State<'_, crate::state::ConsoleState>) -> Value {
    let campus = crate::grades::campus_of(&cs.vault);
    status_with(&cs.data_dir, crate::scaffold::curated(&campus.unitid))
}

/// The row's Forget: [`school_forget_retrying`] over Credential Manager, the profile's marks, the real
/// windows (destroyed by label as sign-out does, [`crate::account::SCHOOL_WINDOWS`]) and a real 250 ms
/// pause. It runs off the event loop (`async`), which is what lets the windows' Destroyed events run.
#[tauri::command(async)]
pub fn school_forget(app: tauri::AppHandle, cs: tauri::State<'_, crate::state::ConsoleState>) -> Value {
    use tauri::Manager;
    let id = profile_key(&cs.data_dir);
    school_forget_retrying(
        &cs.data_dir,
        &id,
        &crate::school_password::RealStore,
        &crate::school_password::FileMarks::new(&cs.data_dir),
        &mut |label| {
            if let Some(w) = app.get_webview_window(label) {
                let _ = w.destroy();
            }
        },
        &mut || std::thread::sleep(FORGET_PAUSE),
        &mut |id| try_take(id),
    )
}
