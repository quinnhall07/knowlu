//! P1: the saved school password and its two marks (replan §7.4, §8 P1a and P1b).
//!
//! **The credential (P1a).** One generic credential per profile, at [`target`]: built by
//! `credentials::target_for` and never by hand, so two profiles never share one. It is written, tested
//! and deleted through `credentials.rs` and read through the engine's `wincred.rs`; this module edits
//! neither. The secret is read inside the process only, while the profile's `school_session` lock is
//! held, and no Tauri command returns it. **Nothing here formats the secret or the username:** no reply,
//! error, `Debug` or log carries either. A store call that was handed them (`write`, `read`) has its
//! error replaced by fixed words, because a store's own text could echo them.
//!
//! **The lock.** A save and a Forget take `school_session::try_take(profile_id)` themselves and answer
//! [`BUSY`] while a fill holds it, so neither can clear a mark a running fill is about to set. A read
//! is handed the caller's guard and refuses unless this profile's lock is held.
//!
//! **The marks (P1b), [`FileMarks`].** The rejected mark blocks every fill until a new save. The
//! in-flight mark is written and flushed before the first credential-carrying submit and cleared only
//! by `users/me` 200 in that attempt or by a new save. Each step keeps P1t's order: a rejection marks,
//! then deletes; a save writes, then clears; a Forget deletes, then clears only once the credential is
//! gone. A step that fails leaves a mark set rather than cleared, which blocks a fill (fails safe).
#![cfg(windows)]
use crate::school_session::{try_take, SessionGuard};
use serde_json::{json, Value};
use std::path::{Path, PathBuf};

/// A save's or a Forget's answer while this profile's lock is held: nothing was written or cleared.
pub const BUSY: &str = "Knowlu is signing in to your school right now. Try again in a moment.";
const WRITE_FAILED: &str = "Knowlu couldn't save your school password on this device.";
const CLEAR_FAILED: &str = "Your school password is saved, but Knowlu couldn't clear an earlier sign-in that didn't finish, so it won't use the password yet. Save it again.";
const READ_FAILED: &str = "Knowlu couldn't read the saved school password.";
const NOT_HELD: &str = "the school sign-in is not held for this profile";

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

/// The saved password, read inside the process. `Debug` is written by hand and prints neither field.
#[derive(Clone, PartialEq, Eq)]
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

impl std::fmt::Debug for SchoolSecret {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("SchoolSecret(<redacted>)")
    }
}

/// What a fill finds when it takes the lock. The rejected mark outranks everything; then the in-flight
/// mark. Its derived `Debug` is safe because [`SchoolSecret`]'s redacts.
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

/// Credential Manager: `credentials.rs` writes, tests and deletes; the engine's `wincred.rs` reads.
pub struct RealStore;
impl SchoolStore for RealStore {
    fn write(&self, target: &str, user: &str, secret: &str) -> Result<(), String> {
        crate::credentials::write(target, user, secret)
    }
    fn read(&self, target: &str) -> Result<(String, String), String> {
        let cred = knowlu_engine::wincred::read_credential(target).map_err(|e| e.to_string())?;
        Ok((cred.username, cred.password.expose().to_string()))
    }
    fn exists(&self, target: &str) -> bool {
        crate::credentials::exists(target)
    }
    fn delete(&self, target: &str) -> Result<(), String> {
        crate::credentials::delete(target)
    }
}

const REJECTED_MARK: &str = "school-password-rejected";
const IN_FLIGHT_MARK: &str = "school-password-in-flight";

/// The two marks (P1b): empty files in the profile's own app-data folder (`profiles\<id>\`), beside
/// `school.json` and never inside `school-session`, which a Forget deletes whole. A file there is a set
/// mark. They hold nothing, so no secret, and they never enter the vault. A mark whose state cannot be
/// read reads as set, which blocks the fill (fails safe). Nothing here makes the folder: a mark written
/// into one that is gone fails, and the fill then sends nothing.
pub struct FileMarks {
    dir: PathBuf,
}
impl FileMarks {
    pub fn new(dir: &Path) -> Self {
        FileMarks { dir: dir.to_path_buf() }
    }
    fn is_set(&self, name: &str) -> bool {
        self.dir.join(name).try_exists().unwrap_or(true)
    }
    fn create(&self, name: &str) -> Result<std::fs::File, String> {
        std::fs::File::create(self.dir.join(name)).map_err(|e| format!("{name} could not be written: {e}"))
    }
    fn clear(&self, name: &str) -> Result<(), String> {
        match std::fs::remove_file(self.dir.join(name)) {
            Err(e) if e.kind() != std::io::ErrorKind::NotFound => Err(format!("{name} could not be cleared: {e}")),
            _ => Ok(()),
        }
    }
}
impl SchoolMarks for FileMarks {
    fn rejected(&self) -> bool {
        self.is_set(REJECTED_MARK)
    }
    fn in_flight(&self) -> bool {
        self.is_set(IN_FLIGHT_MARK)
    }
    /// Written and flushed in one step, so no power cut loses a rejection.
    fn set_rejected(&self) -> Result<(), String> {
        self.create(REJECTED_MARK)?.sync_all().map_err(|e| format!("{REJECTED_MARK} could not be flushed: {e}"))
    }
    fn clear_rejected(&self) -> Result<(), String> {
        self.clear(REJECTED_MARK)
    }
    fn write_in_flight(&self) -> Result<(), String> {
        self.create(IN_FLIGHT_MARK).map(drop)
    }
    /// `sync_all` on the written mark; with no mark written there is nothing to flush, and that fails.
    fn flush_in_flight(&self) -> Result<(), String> {
        std::fs::OpenOptions::new()
            .write(true)
            .open(self.dir.join(IN_FLIGHT_MARK))
            .and_then(|f| f.sync_all())
            .map_err(|e| format!("{IN_FLIGHT_MARK} could not be flushed: {e}"))
    }
    fn clear_in_flight(&self) -> Result<(), String> {
        self.clear(IN_FLIGHT_MARK)
    }
}

/// `credentials::target_for(profile_id, SCHOOL_PASSWORD_SOURCE)`: `knowlu/<profile_id>/school`.
pub fn target(profile_id: &str) -> String {
    crate::credentials::target_for(profile_id, crate::account::SCHOOL_PASSWORD_SOURCE)
}

/// Whether `profile_id`'s lock is held right now. A caller's guard proves some lock is held; this
/// proves it is this profile's, so a guard taken under another key reads nothing.
fn held(profile_id: &str) -> bool {
    try_take(profile_id).is_none()
}

/// Takes `school_session::try_take(profile_id)` (`busy` when `None`, nothing written), writes the
/// credential, then clears both marks. Reply: `{"ok": bool, "busy": bool, "error": string|null}`,
/// never a secret. Both clears run; one that fails leaves its old mark, which blocks the new password
/// (fails safe), and the reply is not ok, so the student is asked again.
pub fn save_with(
    store: &dyn SchoolStore,
    marks: &dyn SchoolMarks,
    profile_id: &str,
    username: &str,
    password: &str,
) -> Value {
    let reply = |ok: bool, busy: bool, error: Option<&str>| json!({ "ok": ok, "busy": busy, "error": error });
    let Some(_held) = try_take(profile_id) else { return reply(false, true, Some(BUSY)) };
    if store.write(&target(profile_id), username, password).is_err() {
        return reply(false, false, Some(WRITE_FAILED));
    }
    let (rejected, in_flight) = (marks.clear_rejected(), marks.clear_in_flight());
    if rejected.is_err() || in_flight.is_err() {
        return reply(false, false, Some(CLEAR_FAILED));
    }
    reply(true, false, None)
}

/// The secret, read only while the caller holds this profile's lock (`_lock`). `None` when none is
/// saved. It never looks at a mark: the in-flight mark is checked only before a fill ([`gate_with`]).
pub fn read_with(store: &dyn SchoolStore, profile_id: &str, _lock: &SessionGuard) -> Result<Option<SchoolSecret>, String> {
    if !held(profile_id) {
        return Err(NOT_HELD.to_string());
    }
    let t = target(profile_id);
    if !store.exists(&t) {
        return Ok(None);
    }
    let (username, password) = store.read(&t).map_err(|_| READ_FAILED.to_string())?;
    Ok(Some(SchoolSecret { username, password }))
}

/// Both marks and the secret, read only while this profile's lock is held; checked only before a fill.
/// The rejected mark first (the form, saved password or not), then the password (none: today's path,
/// the in-flight mark alone included, since with no password nothing can be sent), then the in-flight
/// mark (`Unconfirmed`, left set), and only then is the secret read.
pub fn gate_with(store: &dyn SchoolStore, marks: &dyn SchoolMarks, profile_id: &str, lock: &SessionGuard) -> Result<Gate, String> {
    if !held(profile_id) {
        return Err(NOT_HELD.to_string());
    }
    if marks.rejected() {
        return Ok(Gate::Form);
    }
    if !store.exists(&target(profile_id)) {
        return Ok(Gate::NoPassword);
    }
    if marks.in_flight() {
        return Ok(Gate::Unconfirmed);
    }
    Ok(read_with(store, profile_id, lock)?.map_or(Gate::NoPassword, Gate::Fill))
}

/// Sets the rejected mark, then deletes the saved password; each runs even if the other fails, and a
/// password already gone is not a failure. Called only inside a fill, which holds the lock (`_lock`,
/// the controller's ruling). It never refuses on the lock: a refused rejection would leave a rejected
/// password saved, ready to be sent again.
pub fn reject_with(store: &dyn SchoolStore, marks: &dyn SchoolMarks, profile_id: &str, _lock: &SessionGuard) -> Result<(), String> {
    let marked = marks.set_rejected();
    let t = target(profile_id);
    let deleted = match store.delete(&t) {
        Err(e) if store.exists(&t) => Err(e),
        _ => Ok(()),
    };
    match (marked, deleted) {
        (Ok(()), Ok(())) => Ok(()),
        (Err(m), Err(d)) => Err(format!("{m}; {d}")),
        (Err(e), Ok(())) | (Ok(()), Err(e)) => Err(e),
    }
}

/// Forget (S6a's `school_forget`): takes the lock (`Err(BUSY)` when held, nothing touched), deletes the
/// credential first, and clears both marks only if it was deleted or already gone. A delete that fails
/// keeps the marks and is the error (`credentials::delete`'s sentence: the target, never the secret).
pub fn forget_with(store: &dyn SchoolStore, marks: &dyn SchoolMarks, profile_id: &str) -> Result<(), String> {
    let Some(_held) = try_take(profile_id) else { return Err(BUSY.to_string()) };
    let t = target(profile_id);
    if let Err(e) = store.delete(&t) {
        if store.exists(&t) {
            return Err(e);
        }
    }
    let (rejected, in_flight) = (marks.clear_rejected(), marks.clear_in_flight());
    rejected.and(in_flight)
}

/// A `users/me` 200: clears the in-flight mark and never the rejected one (only a new save does).
pub fn signed_in_with(marks: &dyn SchoolMarks) -> Result<(), String> {
    marks.clear_in_flight()
}

/// Moves `school` between profile ids: the new target is written before the old is deleted; a password
/// whose rejected or in-flight mark is set (read from `from_marks`) is deleted and not moved. `Ok(1)`
/// moved, `Ok(0)` nothing to move or one dropped. It reads the old id's marks and secret, so it takes
/// that id's lock (`BUSY` when held, nothing touched). The same id twice moves nothing: a write then a
/// delete of one target would delete the only copy.
pub fn retarget_with(store: &dyn SchoolStore, from_marks: &dyn SchoolMarks, from_id: &str, to_id: &str) -> Result<usize, String> {
    if from_id == to_id {
        return Ok(0);
    }
    let Some(_held) = try_take(from_id) else { return Err(BUSY.to_string()) };
    let old = target(from_id);
    if !store.exists(&old) {
        return Ok(0);
    }
    if from_marks.rejected() || from_marks.in_flight() {
        return match store.delete(&old) {
            Err(e) if store.exists(&old) => Err(e),
            _ => Ok(0),
        };
    }
    let (username, password) = store.read(&old).map_err(|_| READ_FAILED.to_string())?;
    store.write(&target(to_id), &username, &password).map_err(|_| WRITE_FAILED.to_string())?;
    // Only after the new one is safely written, as `retarget_credentials` does for the coursework logins.
    store.delete(&old)?;
    Ok(1)
}

/// The one Tauri command here: saves the password typed at step 3 or in Settings, and its reply is
/// [`save_with`]'s, which holds no secret. The lock, the target and the marks are all the profile
/// folder's: its name is the profile id, the key `grades` and `lms_link` take the lock under, and the
/// marks sit in it beside `school.json`. A folder whose name is not a profile id is refused, so
/// sign-out can always delete what was saved. S6r registers it.
#[tauri::command(async, rename_all = "snake_case")]
pub fn school_password_save(cs: tauri::State<'_, crate::state::ConsoleState>, username: String, password: String) -> Value {
    let id = crate::school_session::profile_key(&cs.data_dir);
    if !crate::account::is_profile_id(&id) {
        return json!({ "ok": false, "busy": false, "error": WRITE_FAILED });
    }
    save_with(&RealStore, &FileMarks::new(&cs.data_dir), &id, &username, &password)
}
