//! The updater (plan 4a, Task 8; spec §6, Knowlu spec decisions 14–16). Four rules, each a
//! function a test can hold: **never mid-run** (`update_offer`), **a failed check is quiet**
//! (`record_check` — an unreachable endpoint is the expected state, not an incident), **one bundle,
//! one version** (`begin_check`'s single flight and `stage_bytes`' temp-then-rename), and **a failed
//! signature is refused and shown** (the plugin's own verification, see `check_and_stage`).
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Mutex;
use std::time::Duration;
use tauri::AppHandle;

/// The refusal both install paths give when a slot holds the flag. One string, so the page's
/// wording and the tray's recorded wording cannot drift apart.
pub const SLOT_RUNNING: &str = "a slot is running — the update will be offered when it finishes";
/// The refusal a second concurrent check gets (R-P4a-24). It is a refusal, never a queue: the
/// caller is told, and the check already in flight finishes on its own.
pub const ALREADY_CHECKING: &str = "a check is already running";
/// The manifest round-trip's own budget. Short on purpose: the daily check runs unattended and the
/// endpoint may simply not answer, and a request with no deadline holds a thread and the
/// single-flight latch until the process ends.
pub const CHECK_TIMEOUT: Duration = Duration::from_secs(30);
/// The bundle download's budget, set on the `Update` itself (`Update.timeout` is a public field) so
/// the check's 30 s does not also cap a multi-megabyte installer on a student's hotel wifi.
pub const DOWNLOAD_TIMEOUT: Duration = Duration::from_secs(20 * 60);

#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize)]
pub struct Staged { pub version: String, pub path: PathBuf }

#[derive(Default)]
pub struct Updates {
    pub staged: Mutex<Option<Staged>>,
    pub last_check: Mutex<Option<String>>,
    pub last_error: Mutex<Option<String>>,
    /// **The most recent ACTION's outcome, and `record_check` never touches it** (fix round 1,
    /// IMPORTANT 2). `last_error` belongs to the *check* and is rewritten by every `record_check` —
    /// which is every boot, since `bootConsole` runs one — so a tray refusal or a failed install
    /// written there was overwritten before any reader ever saw it. An action's failure lives here
    /// instead, and survives every check, automatic or manual.
    ///
    /// The rule, in one line: **every explicitly initiated action clears this and then records its
    /// own outcome; automatic background work never touches it.** So *Check now*, *Restart to
    /// update* in the page and *Restart to update* in the tray each clear it on the way in
    /// (`clear_action_error`) and set it if they refuse or fail; the daily housekeeping check
    /// neither clears nor sets it. That gives the reader a message that persists until they do
    /// something about it, and a dismissal that is an action rather than a dialog.
    pub last_action_error: Mutex<Option<String>>,
    /// The tray's *Restart to update* item, stashed by `tray::build` so the menu can be enabled in
    /// place rather than rebuilt (the same pattern as `Scheduler.pause_item`).
    pub item: Mutex<Option<tauri::menu::MenuItem<tauri::Wry>>>,
    /// The single-flight latch (R-P4a-24). Two checks can be asked for at once — *Check now* in the
    /// settings row and the daily housekeeping one — and both would download into the same
    /// `updates\` folder and race for one bundle path. An `AtomicBool` rather than a `Mutex`
    /// deliberately: there is nothing to guard but the decision, and an atomic cannot be poisoned,
    /// so the poison-tolerance `scheduler::lock` has to spell out is structural here.
    pub checking: AtomicBool,
}

/// **Never mid-run** (Knowlu spec decision 16). `running` is `Scheduler.running`: a slot that is
/// part-way through coursework or rank must not have its engine replaced under it.
pub fn update_offer(running: bool, staged: Option<&Staged>) -> Option<String> {
    if running { return None; }
    staged.map(|s| s.version.clone())
}

/// The other half of the mid-run gate (R-P4a-24). `update_offer` withholds the OFFER while a slot
/// runs; this says whether there is nevertheless a verified bundle sitting on disk, so the settings
/// row can say *"0.2.0 staged; installs after the run"* instead of *"up to date"* — which would be a
/// plain lie about a file the user's own machine has already downloaded. `Some` exactly when
/// `update_offer` is `None` *because of the run*, never when there is simply nothing staged.
pub fn stage_note(running: bool, staged: Option<&Staged>) -> Option<String> {
    if !running { return None; }
    staged.map(|s| s.version.clone())
}

/// Record an ACTION's failure — the tray's *Restart to update* arm, which is a menu click with no
/// envelope to return; a refused install; a check refused by the single-flight latch (R-P4a-24, and
/// fix round 1 IMPORTANT 2 / m2).
///
/// It writes `last_action_error` and **nothing else**: not `last_check`, which would claim a check
/// happened when none did; not `last_error`, which belongs to the check and is rewritten by the very
/// next `record_check`; and not `staged`, because a refused install leaves the bundle on disk and
/// still correct. The settings row shows it, and it survives every check until the next explicit
/// action clears it.
pub fn note_error(u: &Updates, msg: &str) {
    *u.last_action_error.lock().unwrap_or_else(|e| e.into_inner()) = Some(msg.to_string());
}

/// The other half of the rule: an explicitly initiated action clears the previous action's outcome
/// on the way in, so what the row shows is always the latest attempt and never a message the user
/// has already acted on. Called by `check_for_updates`, `install_update` and the tray's install arm;
/// never by the housekeeping tick.
pub fn clear_action_error(u: &Updates) {
    *u.last_action_error.lock().unwrap_or_else(|e| e.into_inner()) = None;
}

/// Fold one check's outcome into the state. A failure keeps the previous staged update (a network
/// blip does not un-stage a bundle already on disk); a success that offers nothing means the
/// manifest no longer carries it, so the stage is cleared.
pub fn record_check(u: &Updates, result: Result<Option<Staged>, String>, now: &str) {
    *u.last_check.lock().unwrap_or_else(|e| e.into_inner()) = Some(now.to_string());
    match result {
        Ok(s) => {
            *u.last_error.lock().unwrap_or_else(|e| e.into_inner()) = None;
            *u.staged.lock().unwrap_or_else(|e| e.into_inner()) = s;
        }
        Err(e) => { *u.last_error.lock().unwrap_or_else(|e| e.into_inner()) = Some(e); }
    }
}

/// S11: an install is as exclusive as a slot. Taking `Scheduler.running` for the duration means a
/// tick that comes due mid-install stands down on its own existing "already running" branch —
/// there is no second flag and no new race. `None` when a slot (or another install) already holds
/// it, which the caller reports rather than queues.
pub struct InstallHold<'a>(&'a crate::scheduler::Scheduler);
impl Drop for InstallHold<'_> {
    fn drop(&mut self) { *crate::scheduler::lock(&self.0.running) = false; }
}
pub fn hold_for_install(sch: &crate::scheduler::Scheduler) -> Option<InstallHold<'_>> {
    let mut r = crate::scheduler::lock(&sch.running);
    if *r { return None; }
    *r = true;
    drop(r);
    Some(InstallHold(sch))
}

/// Single flight (R-P4a-24). `None` while another check is in flight — the second caller is
/// **refused, not queued**, because the two would otherwise download into one `updates\` folder and
/// race for one bundle path. The latch is given back on drop, including on every `?` inside
/// `check_and_stage`, which is the reason it is a guard rather than two stores.
pub struct CheckGuard<'a>(&'a Updates);
impl Drop for CheckGuard<'_> {
    fn drop(&mut self) { self.0.checking.store(false, Ordering::SeqCst); }
}
pub fn begin_check(u: &Updates) -> Option<CheckGuard<'_>> {
    u.checking
        .compare_exchange(false, true, Ordering::SeqCst, Ordering::SeqCst)
        .ok()
        .map(|_| CheckGuard(u))
}

/// Put verified bytes at `path` **through a temp name in the same folder** (R-P4a-24).
///
/// Two things this buys, both of which a plain `fs::write` loses. A write that dies half-way — the
/// process quit, the disk filled — leaves the partial at `<path>.part<pid>`, never at the name
/// `install_staged` will later hand to the installer; and because the last step is a rename rather
/// than a truncate-and-fill, an `install_staged` that already has the old file open goes on reading
/// the bytes it opened (Rust opens with `FILE_SHARE_DELETE`, so Windows replaces the directory entry
/// and leaves the open handle on the unlinked file) instead of reading half of each version.
///
/// The pid in the temp name means two Knowlu processes staging the same version cannot collide on
/// the partial either; they still converge on one bundle per version, because `rename` replaces.
pub fn stage_bytes(path: &Path, bytes: &[u8]) -> Result<(), String> {
    let mut tmp = path.as_os_str().to_os_string();
    tmp.push(format!(".part{}", std::process::id()));
    let tmp = PathBuf::from(tmp);
    std::fs::write(&tmp, bytes).map_err(|e| format!("{}: {e}", tmp.display()))?;
    match std::fs::rename(&tmp, path) {
        Ok(()) => Ok(()),
        Err(e) => {
            // Never leave the partial behind on the failing path: the next check would find a
            // stale `.part` beside the bundle and nothing would ever remove it.
            let _ = std::fs::remove_file(&tmp);
            Err(format!("{}: {e}", path.display()))
        }
    }
}

/// Is the version the manifest is offering already on disk? (fix round 1, IMPORTANT 3.)
///
/// Without this, every check re-downloaded the whole bundle — the daily one included — so a staged
/// update that sat un-installed for a week cost a week of installer downloads, on a student's data
/// allowance, to arrive at the identical bytes. **Both conditions have to hold**: the recorded
/// `Staged` names this exact version, *and* the file it names is still there. A `Staged` whose file
/// has been deleted (a disk cleanup, a friend tidying `%LOCALAPPDATA%`) must re-download rather than
/// offer a path with nothing behind it, which is what `install_staged` would then fail on.
///
/// Deliberately not a `bool`: it hands back the `Staged` the caller should return, so there is no
/// second place that rebuilds the same value from the same two fields.
pub fn already_staged(staged: Option<&Staged>, remote_version: &str) -> Option<Staged> {
    let s = staged?;
    if s.version != remote_version { return None; }
    if !s.path.is_file() { return None; }
    Some(s.clone())
}

/// Check the manifest and, if there is one, download the bundle into `updates_dir`.
///
/// **`async`, and the two `_blocking` twins below exist for the two std threads** (B4): a
/// `#[tauri::command]` on an `async fn` already runs on the tokio runtime, and
/// `tauri::async_runtime::block_on` inside it panics — "cannot start a runtime from within a
/// runtime". The commands `.await` this; the tray arm and the housekeeping tick, which are plain
/// `std::thread::spawn`s with no runtime under them, call the blocking twin.
///
/// **Where the signature is checked.** `tauri-plugin-updater` 2.11.0 verifies the minisign
/// signature over the whole downloaded buffer *inside* `Update::download`, against
/// `plugins.updater.pubkey`, and returns `Err` rather than bytes when it fails (`updater.rs`, the
/// `verify_signature` call at the end of `download`). So **the only bytes that ever reach
/// `stage_bytes` are already verified** — this function does not re-implement, and cannot skip,
/// that check. A bad signature comes back here as an ordinary `Err`, is recorded by `record_check`
/// and shown in the settings row: refused and reported, never retried silently (Knowlu spec §6).
///
/// `up` is taken so the single-flight latch lives in exactly one place rather than at each of the
/// two call sites.
///
/// **Only ever called under the console builder.** `updater_builder()` resolves the plugin's own
/// managed `UpdaterState`, and `Manager::state` panics when it is not there — so this function on an
/// app that did not register `tauri_plugin_updater` is a panic, not an `Err`. `run_shell` has no
/// updater and no caller of this; `run_console` registers the plugin (`main.rs`).
pub async fn check_and_stage(app: &AppHandle, up: &Updates, updates_dir: &Path) -> Result<Option<Staged>, String> {
    use tauri_plugin_updater::UpdaterExt;
    // Held for the whole check AND the whole download: the point is that a second caller cannot be
    // writing the same bundle path while the first is still filling it.
    let Some(_flight) = begin_check(up) else { return Err(ALREADY_CHECKING.to_string()) };
    // `updater_builder()` rather than `updater()` for one reason: `timeout` lives on
    // `UpdaterBuilder`, not on the plugin's `Builder`, in 2.11.0 (R-P4a-24).
    let updater = app.updater_builder().timeout(CHECK_TIMEOUT).build().map_err(|e| e.to_string())?;
    let Some(mut update) = updater.check().await.map_err(|e| e.to_string())? else { return Ok(None) };
    // IMPORTANT 3: this exact version may already be on disk from an earlier check the user has not
    // installed yet. Read the recorded stage and let go of the lock before any `.await` — a guard
    // held across one is both a deadlock risk and not `Send`.
    let known = { up.staged.lock().unwrap_or_else(|e| e.into_inner()).clone() };
    if let Some(s) = already_staged(known.as_ref(), &update.version) {
        return Ok(Some(s));
    }
    // The check is a small JSON GET; the bundle is tens of megabytes. One timeout cannot serve both.
    update.timeout = Some(DOWNLOAD_TIMEOUT);
    let bytes = update.download(|_, _| {}, || {}).await.map_err(|e| e.to_string())?;
    std::fs::create_dir_all(updates_dir).map_err(|e| format!("{}: {e}", updates_dir.display()))?;
    let path = updates_dir.join(format!("knowlu-{}.bundle", update.version));
    stage_bytes(&path, &bytes)?;
    Ok(Some(Staged { version: update.version.clone(), path }))
}

/// Install what was staged.
///
/// The manifest is re-read so the bytes on disk are installed only if the release they were staged
/// from is **still** the one being offered — "one bundle, one version" (R9). A manifest that has
/// moved on, or no longer offers anything, refuses rather than installing a version nobody is
/// offering any more; the next check re-stages whatever is current.
///
/// **A bad signature never reaches here**: the plugin verified these bytes before `check_and_stage`
/// was allowed to write them (see above), so `Update::install` is being handed a buffer this
/// process already saw verified. What is *not* re-verified is the hop through disk in between, and
/// that is a deliberate, documented limit: `verify_signature` is private in 2.11.0, so re-checking
/// would mean vendoring `minisign-verify` and the pubkey into this crate.
///
/// **The premise that argument rests on is pinned in the config, not assumed** (fix round 1, m3):
/// `bundle.windows.nsis.installMode` is `"currentUser"` in `app/tauri.conf.json`, asserted by
/// `the_bundle_config_is_the_one_the_installer_and_the_updater_need`. Knowlu therefore installs
/// under the user's own profile, and the staged bundle sits beside it in that same profile's
/// `%LOCALAPPDATA%\knowlu\updates\`, written temp-then-renamed — so anyone who can rewrite the
/// bundle can equally rewrite `knowlu.exe` itself, and the extra check would buy no real ground.
/// **If that install mode ever becomes `perMachine` or `both`, this reasoning is void**: the exe
/// would then live somewhere only an administrator can write while the bundle stayed
/// user-writable, and a signature check across the disk hop would become load-bearing.
pub async fn install_staged(app: &AppHandle, staged: &Staged) -> Result<(), String> {
    use tauri_plugin_updater::UpdaterExt;
    let bytes = std::fs::read(&staged.path)
        .map_err(|e| format!("the staged bundle could not be read ({}): {e}", staged.path.display()))?;
    let updater = app.updater_builder().timeout(CHECK_TIMEOUT).build().map_err(|e| e.to_string())?;
    let update = updater.check().await.map_err(|e| e.to_string())?
        .ok_or_else(|| "the update is no longer offered".to_string())?;
    if update.version != staged.version {
        return Err(format!("the release moved on: {} is offered now, {} was staged", update.version, staged.version));
    }
    // On Windows this launches the NSIS installer and exits the process, so `Ok(())` is reached
    // only on the platforms that relaunch by hand. The install hold is still correct: it is held
    // right up to the exit.
    update.install(bytes).map_err(|e| e.to_string())
}

/// For callers with no async runtime under them — the tray menu arm and the housekeeping thread.
/// Never call these from inside a command: that is the panic B4 is about.
pub fn check_and_stage_blocking(app: &AppHandle, up: &Updates, updates_dir: &Path) -> Result<Option<Staged>, String> {
    tauri::async_runtime::block_on(check_and_stage(app, up, updates_dir))
}
pub fn install_staged_blocking(app: &AppHandle, staged: &Staged) -> Result<(), String> {
    tauri::async_runtime::block_on(install_staged(app, staged))
}
