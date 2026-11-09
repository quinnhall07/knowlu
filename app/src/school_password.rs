//! P1: the saved school password and its two marks (replan §7.4, §8 P1a and P1b). **Stubs for P1t**:
//! every body is `todo!()` so the tests in `tests/school_password.rs` fail for the right reason.
//! The real bodies route through `credentials.rs` and `wincred.rs` and edit neither.
#![cfg(windows)]
use crate::school_session::SessionGuard;
use serde_json::Value;
use std::path::{Path, PathBuf};

/// The credential seam: targets are built by [`target`], never by hand.
pub trait SchoolStore {
    fn write(&self, target: &str, user: &str, secret: &str) -> Result<(), String>;
    /// `(username, password)`.
    fn read(&self, target: &str) -> Result<(String, String), String>;
    fn exists(&self, target: &str) -> bool;
    fn delete(&self, target: &str) -> Result<(), String>;
}

/// The file seam for the rejected and in-flight marks (the profile's app data, never the vault).
pub trait SchoolMarks {
    fn rejected(&self) -> bool;
    fn in_flight(&self) -> bool;
    fn set_rejected(&self) -> Result<(), String>;
    fn clear_rejected(&self) -> Result<(), String>;
    fn write_in_flight(&self) -> Result<(), String>;
    /// `sync_all` on the in-flight mark's file.
    fn flush_in_flight(&self) -> Result<(), String>;
    fn clear_in_flight(&self) -> Result<(), String>;
}

/// The saved password, read inside the process. The stub derives `Debug` on purpose (it prints the
/// password); the real one redacts it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SchoolSecret {
    pub username: String,
    password: String,
}

impl SchoolSecret {
    pub fn new(username: &str, password: &str) -> Self {
        SchoolSecret { username: username.to_string(), password: password.to_string() }
    }
    pub fn password(&self) -> &str {
        &self.password
    }
}

/// What a fill finds when it takes the lock. The rejected mark outranks everything; then the in-flight mark.
#[derive(Debug)]
pub enum Gate {
    Fill(SchoolSecret),
    /// The rejected mark is set, with or without a password: Knowlu's own form.
    Form,
    /// The in-flight mark is set and a password is saved: `Rejected(Unconfirmed)`, nothing sent.
    Unconfirmed,
    /// No password and no mark: today's path.
    NoPassword,
}

pub struct RealStore;
impl SchoolStore for RealStore {
    fn write(&self, _t: &str, _u: &str, _s: &str) -> Result<(), String> { todo!("P1a") }
    fn read(&self, _t: &str) -> Result<(String, String), String> { todo!("P1a") }
    fn exists(&self, _t: &str) -> bool { todo!("P1a") }
    fn delete(&self, _t: &str) -> Result<(), String> { todo!("P1a") }
}

pub struct FileMarks {
    #[allow(dead_code)]
    dir: PathBuf,
}
impl FileMarks {
    pub fn new(dir: &Path) -> Self {
        FileMarks { dir: dir.to_path_buf() }
    }
}
impl SchoolMarks for FileMarks {
    fn rejected(&self) -> bool { todo!("P1b") }
    fn in_flight(&self) -> bool { todo!("P1b") }
    fn set_rejected(&self) -> Result<(), String> { todo!("P1b") }
    fn clear_rejected(&self) -> Result<(), String> { todo!("P1b") }
    fn write_in_flight(&self) -> Result<(), String> { todo!("P1b") }
    fn flush_in_flight(&self) -> Result<(), String> { todo!("P1b") }
    fn clear_in_flight(&self) -> Result<(), String> { todo!("P1b") }
}

/// `credentials::target_for(profile_id, SCHOOL_PASSWORD_SOURCE)`.
pub fn target(_profile_id: &str) -> String { todo!("P1a") }

/// Takes `school_session::try_take(profile_id)` (`busy` when `None`, nothing written), writes the
/// credential, then clears both marks. Reply: `{"ok": bool, "busy": bool, "error": string|null}`,
/// never a secret.
pub fn save_with(
    _store: &dyn SchoolStore,
    _marks: &dyn SchoolMarks,
    _profile_id: &str,
    _username: &str,
    _password: &str,
) -> Value {
    todo!("P1a")
}

/// The secret, read only while the caller holds the lock (`_lock`). `None` when none is saved.
pub fn read_with(_store: &dyn SchoolStore, _profile_id: &str, _lock: &SessionGuard) -> Result<Option<SchoolSecret>, String> {
    todo!("P1a")
}

/// Both marks and the secret, read only under the lock; checked only before a fill.
pub fn gate_with(
    _store: &dyn SchoolStore,
    _marks: &dyn SchoolMarks,
    _profile_id: &str,
    _lock: &SessionGuard,
) -> Result<Gate, String> {
    todo!("P1b")
}

/// Sets the rejected mark, then deletes the saved password; each runs even if the other fails.
pub fn reject_with(_store: &dyn SchoolStore, _marks: &dyn SchoolMarks, _profile_id: &str) -> Result<(), String> {
    todo!("P1b")
}

/// Deletes the credential first; clears both marks only if it was deleted or already gone.
pub fn forget_with(_store: &dyn SchoolStore, _marks: &dyn SchoolMarks, _profile_id: &str) -> Result<(), String> {
    todo!("P1b")
}

/// A `users/me` 200: clears the in-flight mark and never the rejected one.
pub fn signed_in_with(_marks: &dyn SchoolMarks) -> Result<(), String> {
    todo!("P1b")
}

/// Moves `school` between profile ids: the new target is written before the old is deleted; a password
/// whose rejected or in-flight mark is set (read from `from_marks`) is deleted and not moved. `Ok(1)`
/// moved, `Ok(0)` nothing to move or one dropped.
pub fn retarget_with(
    _store: &dyn SchoolStore,
    _from_marks: &dyn SchoolMarks,
    _from_id: &str,
    _to_id: &str,
) -> Result<usize, String> {
    todo!("P1b")
}

/// The one Tauri command: saves the password. Its reply holds no secret. S6r registers it.
#[tauri::command(async, rename_all = "snake_case")]
#[allow(unused_variables)]
pub fn school_password_save(cs: tauri::State<'_, crate::state::ConsoleState>, username: String, password: String) -> Value {
    todo!("P1a")
}
