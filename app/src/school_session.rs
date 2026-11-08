//! STUB (task S1): the LMS's one persistent sign-in folder, its uses and its per-profile lock.
//! Task S2 writes the real module; every body here is `todo!()` so `tests/school_session.rs` compiles
//! and fails for the right reason.
use crate::grades::{CaptureError, Capturing};
use crate::scaffold::Curated;
use std::path::{Path, PathBuf};

/// What reads through the folder. Replan RQ4: the LMS's alone, so grades and the course list.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum Use {
    Grades,
    CourseList,
}

/// Who asks for a folder.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Feature {
    Grades,
    LmsCapture,
    Registrar,
    SchoolMail,
}

/// Held while a caller owns one profile's school folder; dropping it frees the profile.
pub struct SessionGuard(());

pub fn try_take(_profile_id: &str) -> Option<SessionGuard> {
    todo!("S2")
}

/// Session lock first, then `Capturing`; either busy is `Busy` with the first guard dropped.
pub fn take_both(_profile_id: &str) -> Result<(SessionGuard, Capturing), CaptureError> {
    todo!("S2")
}

pub fn session_dir(_data_dir: &Path) -> PathBuf {
    todo!("S2")
}

pub fn is_session_dir(_data_dir: &Path, _dir: &Path) -> bool {
    todo!("S2")
}

/// Patient delete of the folder and `school.json`'s uses. `Err` when refused or not finished.
pub fn forget(_data_dir: &Path) -> Result<(), String> {
    todo!("S2")
}

pub fn uses(_data_dir: &Path) -> Result<Vec<Use>, String> {
    todo!("S2")
}

pub fn add_use(_data_dir: &Path, _u: Use) -> Result<(), String> {
    todo!("S2")
}

/// Clearing the last use deletes the folder.
pub fn clear_use(_data_dir: &Path, _u: Use) -> Result<(), String> {
    todo!("S2")
}

/// The folder a feature's window is built on; `None` for a feature with no folder of this module's.
pub fn profile_for(_feature: Feature, _row: Option<&Curated>, _data_dir: &Path) -> Option<PathBuf> {
    todo!("S2")
}

/// A `sign_in_hosts` entry: a bare host name, no scheme, path, port or whitespace.
pub fn sign_in_host_ok(_host: &str) -> bool {
    todo!("S2")
}

/// A `wrong_password_text`: empty, or fixed text with no `@` and no digits-only token.
pub fn wrong_password_text_ok(_text: &str) -> bool {
    todo!("S2")
}
