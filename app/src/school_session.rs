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
/// Grades keep today's `lms-session` folder until the school folder exists, so an install already
/// signed in is not signed out by an upgrade.
pub fn profile_for(feature: Feature, _row: Option<&Curated>, data_dir: &Path) -> Option<PathBuf> {
    match feature {
        Feature::Grades => {
            let legacy = data_dir.join(LEGACY_GRADES_DIR);
            Some(if !session_dir(data_dir).exists() && legacy.exists() { legacy } else { session_dir(data_dir) })
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

// ---- S3 stubs: the seams the windows on the folder are tested through. S4a and S4b fill them. ----

/// The one arguments string every window on the school folder is built with. **S3 stub**: S4a sets it
/// equal to `mail::MAIL_BROWSER_ARGS` (S-G recorded no change). It never carries a debug port.
pub const SCHOOL_BROWSER_ARGS: &str = "";

/// A school window's browser arguments. The debug port is a parameter so the rule is testable without
/// the environment: a window on the school folder gets [`SCHOOL_BROWSER_ARGS`] whatever port is
/// passed, in every build. **S3 stub.**
pub fn window_args(_debug_port: Option<u16>) -> &'static str {
    SCHOOL_BROWSER_ARGS
}

/// One cookie a window's jar returned, for the fakes of the S3 cookie tests. `domain` is the cookie's own
/// `Domain` attribute, which no reader may send a request to.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct JarCookie {
    pub name: String,
    pub value: String,
    pub domain: String,
}

/// The folder grades' window is built on, and the use that choice records. **S3 stub**: S4a makes it
/// [`profile_for`]'s answer for `Feature::Grades` and, whenever that is [`session_dir`], calls
/// `add_use(Grades)` before returning. It moves off `lms-session` only on a recorded `Use::Grades`.
pub fn grades_profile(data_dir: &Path) -> Result<PathBuf, String> {
    profile_for(Feature::Grades, None, data_dir).ok_or_else(|| "no folder for grades".to_string())
}
