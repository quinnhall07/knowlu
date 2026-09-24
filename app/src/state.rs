//! What the shell holds between commands: the vault path, the per-device "last looked" stamp,
//! settings, sync/backup/run status, and one lock so console operations on the vault are
//! serialised (spec §3.2; Knowlu plan 1, Task 7 adds settings and the status fields Task 8+ fill).
use std::path::{Path, PathBuf};
use std::sync::atomic::AtomicUsize;
use std::sync::Mutex;
use knowlu_engine::backup::BackupStatus;

/// Per-profile settings, round-tripped through `settings.json` in the app data dir. `profile_id`
/// is derived from the vault path, never hand-named (Task 7's rename-stage-1 rule: no new
/// single-user assumptions — a second profile just derives a different id from a different vault).
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct Settings {
    pub profile_id: String,
    pub backup_dir: Option<PathBuf>,
    pub autostart: bool,
    pub quit_at: Option<String>,
}

impl Settings {
    /// A missing file is not an error — that is every profile's first launch. A file that
    /// **exists** but does not parse (hand-edited, truncated, an old/incompatible shape) falls
    /// back to defaults too, but names the path and the parse/IO error in the second element so
    /// `ConsoleState::open` can log it and `diagnostics_text` can surface it — silently discarding
    /// a corrupt settings file left no trace anywhere before this.
    pub fn load(path: &Path, vault: &Path) -> (Settings, Option<String>) {
        let default = Settings {
            profile_id: knowlu_engine::ids::derived_id("profile", &vault.to_string_lossy()),
            backup_dir: None,
            autostart: true,
            quit_at: None,
        };
        if !path.is_file() {
            return (default, None);
        }
        match std::fs::read_to_string(path) {
            Ok(text) => match serde_json::from_str::<Settings>(&text) {
                Ok(s) => (s, None),
                Err(e) => (default, Some(format!("{}: {e}", path.display()))),
            },
            Err(e) => (default, Some(format!("{}: {e}", path.display()))),
        }
    }
    pub fn save(&self, path: &Path) -> Result<(), String> {
        if let Some(p) = path.parent() {
            std::fs::create_dir_all(p).map_err(|e| e.to_string())?;
        }
        let v = serde_json::to_value(self).map_err(|e| e.to_string())?;
        std::fs::write(path, knowlu_engine::ledger::dumps_value(&v)).map_err(|e| e.to_string())
    }
}

pub struct ConsoleState {
    pub vault: PathBuf,
    pub seen_path: PathBuf,
    /// This profile's own app-data folder (`profiles\<id>\` under the root) — the scheduler's logs,
    /// `settings.json` and `seen.txt` all sit inside it, so two profiles on one machine never share
    /// a file (plan 4a, Task 2).
    pub data_dir: PathBuf,
    pub settings_path: PathBuf,
    pub lock: Mutex<()>,
    /// **Vault I/O, serialised (console spec §8 the sync lock, §9 the race).** The engine's `sync`
    /// can write a pulled note or file an amend card under it, while a console write is single-line
    /// surgery on a note it has just read. Interleave the two and the write lands on a file the
    /// pull is about to write. Every IN-PROCESS path that touches the vault takes this one lock:
    /// `mutate` for the duration of a write, `run_sync` for the duration of the engine's own
    /// `sync` call — so a console write, *Sync now* and the quit push all queue behind each other
    /// instead of racing, and housekeeping's own backup takes it too (F11) for the same reason.
    ///
    /// **The slot's own `sync` step is a CHILD PROCESS and takes no `vault_io`** (fix round 1,
    /// review M1, correcting a false claim this doc used to make): it runs like every other slot
    /// step, outside this lock entirely, because `vault_io` is never held across a child-process
    /// wait, which may run for 20 minutes. What keeps it from racing the in-process callers above
    /// is a different, cross-process mechanism: `run_lines_with`'s own exclusive file lock under
    /// `state/` (review I1), taken and released inside every one of the three callers alike,
    /// engine process or not.
    ///
    /// **Ordering, and it is one-way:** `vault_io` is taken BEFORE `lock`, never while `lock` is
    /// held. `lock` alone still guards the read polls, so a `state` poll never waits on a sync's
    /// own network call.
    pub vault_io: Mutex<()>,
    pub settings: Mutex<Settings>,
    /// `Some` when `settings.json` existed but did not parse at open — names the path and the
    /// error. Fixed at open; never mutated afterward, so no `Mutex` is needed.
    pub settings_error: Option<String>,
    pub session: String,
    pub backup: Mutex<BackupStatus>,
    /// What the last sync did, for the page's sync line. Filled by `commands::sync_inner` and by
    /// the slot's own sync step; never computed in `commands.rs` (console spec §3.1). The type is
    /// the ENGINE's, because the engine is what produces it and a second struct in the app would be
    /// a second thing to keep in step with the run that fills it.
    pub sync: Mutex<knowlu_engine::sync::SyncStatus>,
    pub last_write: Mutex<Option<std::time::Instant>>,
    pub pending_edits: AtomicUsize,
    pub startup_missed: AtomicUsize,
    /// **Test-only seam.** `None` (the only value any real caller ever sets) means "today" is
    /// wherever `commands::now_in` finds the real clock, exactly as before this field existed — no
    /// production path, command or wizard step ever writes here. A test that needs the read model
    /// and a `decide`d approval's expiry to agree on the same day sets this once via
    /// `ConsoleState::set_test_today` instead of racing the frozen `vault-full` fixture's fixed
    /// `expires:` dates against whatever day the suite happens to run on (`app/tests/commands.rs`,
    /// `rejecting_and_snoozing_write_the_decision_fields`).
    pub test_today: Mutex<Option<jiff::civil::Date>>,
}

impl ConsoleState {
    pub fn open(vault: PathBuf, app_data_dir: PathBuf) -> ConsoleState {
        let _ = std::fs::create_dir_all(&app_data_dir);
        let settings_path = app_data_dir.join("settings.json");
        let (settings, settings_error) = Settings::load(&settings_path, &vault);
        if let Some(e) = &settings_error {
            eprintln!("Knowlu: {e}");
        }
        ConsoleState {
            data_dir: app_data_dir.clone(),
            seen_path: app_data_dir.join("seen.txt"),
            settings_path,
            lock: Mutex::new(()),
            vault_io: Mutex::new(()),
            settings: Mutex::new(settings),
            settings_error,
            session: knowlu_engine::ids::new_id("sess"),
            backup: Mutex::new(BackupStatus { last_ok: None, behind_days: None, last_error: None, target_reachable: false }),
            // Fix round 1, review I4: read back whatever the last run (this launch's or an
            // earlier one's) left in `state/sync-status.json` — a missing or unreadable file is
            // the default, never an error, the same rule `load_status` itself follows. Without
            // this the line reads "not synced yet" on every launch even when the 07:00 slot synced
            // clean, because the slot's own `sync` step is a child process and cannot fill this
            // field itself.
            sync: Mutex::new(knowlu_engine::sync::load_status(&vault)),
            last_write: Mutex::new(None),
            pending_edits: AtomicUsize::new(0),
            startup_missed: AtomicUsize::new(0),
            test_today: Mutex::new(None),
            vault,
        }
    }
    pub fn seen_at(&self) -> Option<String> { std::fs::read_to_string(&self.seen_path).ok().map(|s| s.trim().to_string()).filter(|s| !s.is_empty()) }

    /// Pins "today" for every `commands::now_in` call this state makes from here on — the test-only
    /// seam described on `test_today`. Not `#[cfg(test)]`: `app/tests/**` links this crate as an
    /// ordinary dependency, so the field and this setter always compile, but nothing outside a test
    /// ever calls it.
    pub fn set_test_today(&self, d: Option<jiff::civil::Date>) {
        *self.test_today.lock().unwrap() = d;
    }

    /// Records that a note write just happened: bumps the pending-edit count and stamps
    /// `last_write` for the housekeeping thread's debounced backup (fix round 1, review M2: it no
    /// longer debounces a sync — that cadence is the slot, `Sync now`, and the quit push only).
    pub fn note_write(&self) {
        *self.last_write.lock().unwrap() = Some(std::time::Instant::now());
        self.pending_edits.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
    }
}

/// Re-reads the persisted sync status into the cache (fix round 1, review I4). The slot's own
/// `sync` step is a child process — `run_slot_inner` records only its exit code, and the lines it
/// printed went to a log file, not to `cs.sync` — so this is how a slot's sync reaches the page at
/// all. Called right after the slot's own child steps (git leaves the product, C3' Task 10).
/// `commands::sync_inner` needs no equivalent call: `state::run_sync` already fills `cs.sync`
/// directly, in-process, for *Sync now*.
pub fn refresh_sync(cs: &ConsoleState) {
    let s = knowlu_engine::sync::load_status(&cs.vault);
    *cs.sync.lock().unwrap_or_else(|e| e.into_inner()) = s;
}

/// Runs the engine's sync in-process and records what it did. **Takes `vault_io`, never `lock`**:
/// a pull writes notes and can file a card, so it must not run under the console's read lock, and
/// `commands::sync_inner` takes `lock` only afterwards to rebuild `state`.
pub fn run_sync(cs: &ConsoleState) -> knowlu_engine::sync::SyncStatus {
    let (_, lines, totals) = {
        let _io = cs.vault_io.lock().unwrap_or_else(|e| e.into_inner());
        knowlu_engine::sync::run_lines_with(&cs.vault, knowlu_engine::sync::Direction::Both, "dashboard", None)
    };
    let status = knowlu_engine::sync::SyncStatus::of(&totals, lines);
    *cs.sync.lock().unwrap_or_else(|e| e.into_inner()) = status.clone();
    status
}

/// Mirror + snapshot + prune (`backup::tick`) against `settings.backup_dir`. `Err` when no
/// backup folder is set — that is a settings gap, not a failed backup, so it never reaches
/// `backup::tick` or touches `cs.backup` at all.
pub fn run_backup(cs: &ConsoleState, now: jiff::Timestamp) -> Result<knowlu_engine::backup::BackupStatus, String> {
    let (dir, profile) = {
        let s = cs.settings.lock().unwrap();
        (s.backup_dir.clone(), s.profile_id.clone())
    };
    let dir = dir.ok_or_else(|| "no backup folder set — pick one in settings".to_string())?;
    let today = now.to_zoned(knowlu_engine::cli::vault_zone(&cs.vault)).date();
    let st = knowlu_engine::backup::tick(&cs.vault, &dir, &profile, now, today);
    *cs.backup.lock().unwrap() = st.clone();
    Ok(st)
}

/// The app-data root under an explicit base — the seam the profile tests use. Order still matters:
/// `app_data_root` runs `migrate_flat_layout` before calling this, so the fold sees the flat layout
/// as it was. It is no longer load-bearing that this directory does not exist yet — the fold folds
/// INTO whatever `knowlu` is there and renames nothing (R-P4a-28) — but this is still the one place
/// the path is decided.
pub fn app_data_root_in(base: &Path) -> PathBuf {
    let d = base.join("knowlu");
    let _ = std::fs::create_dir_all(&d);
    d
}

/// `%LOCALAPPDATA%\knowlu` (plan 4a decision 2 — the move plan 2 Task 12 was going to make).
/// `None` only when `LOCALAPPDATA` is unset or empty; `main` then falls back to the temp dir. There
/// is no `app_data_dir()` fallback any more — the shell resolves its root before Tauri exists.
pub fn app_data_root() -> Option<PathBuf> {
    let base = std::env::var("LOCALAPPDATA").ok().filter(|s| !s.is_empty())?;
    let base = PathBuf::from(base);
    // Anything the move could not do is printed, never swallowed (review round 1, IMPORTANT 2).
    if let Some(m) = crate::profiles::migrate_flat_layout(&base, None) {
        for e in &m.errors { eprintln!("Knowlu: {e}"); }
    }
    Some(app_data_root_in(&base))
}

/// What a quit flush actually managed before its cap fired. Every field means "this completed",
/// never "this was attempted": a vault with no account, or one that is signed out, quits
/// `synced: false`, and that is not an error — there was nothing to push, or no session to push it
/// under (fix round 2, review M2: the parenthetical this replaced claimed no vault here ever had a
/// git "remote", which is false — `HistoryStatus::has_remote` existed for exactly that; the point
/// is only that this struct's own vocabulary is the engine's now, not git's).
#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize)]
pub struct QuitFlush {
    pub synced: bool,
    pub backed_up: bool,
    pub timed_out: bool,
}

/// Push on close (console spec §8) and back up on quit (Knowlu spec §4), synchronously, bounded by
/// `cap` — F10. The two engine calls run on a scoped worker and the caller waits at most `cap`: a
/// hung account must never hold the Quit click hostage.
///
/// `then` is called exactly once, on the caller's thread, and **before the scope joins** — with the
/// worker's result when it arrives, or with `timed_out: true` at the cap. That ordering is the
/// whole design: the tray hands in a `then` that writes the quit log and ends the process, so a
/// worker still stuck inside a slow push never delays the quit past the cap (fix round 1, review
/// M2: no git here to be stuck inside of; the scope's join is exactly the unbounded wait the cap
/// exists to prevent). Nothing is lost when the cap fires — the edits are on disk and in the
/// journal, and the next slot, `Sync now`, or another quit picks them up (housekeeping no longer
/// syncs at all).
///
/// The push is skipped outright on a vault with no account: there is nothing to push. A vault WITH
/// one but signed out is a named SKIP inside `run_lines_with` (`totals.skipped`, printed
/// `sync (skipped: no session)`); one without an entitlement is instead an ERROR there
/// (`totals.errors`, printed `sync (no entitlement)`) — fix round 2, review M2, correcting a claim
/// that a 402 was also "a skip on stdout". Either way `synced` reads `false`, never `true`, which
/// is this struct's own contract (above) and what M4's fix (below) makes true of both (fix round 1,
/// review M4).
pub fn quit_flush(cs: &ConsoleState, cap: std::time::Duration, then: impl FnOnce(QuitFlush) + Send) -> QuitFlush {
    let (tx, rx) = std::sync::mpsc::channel::<QuitFlush>();
    let mut out = QuitFlush { synced: false, backed_up: false, timed_out: false };
    std::thread::scope(|s| {
        s.spawn(move || {
            let mut q = QuitFlush { synced: false, backed_up: false, timed_out: false };
            // A quit flush pushes what is already on disk; it never pulls. A pull applies writes and
            // can file cards, and doing that while the window is closing would show the student a
            // deck they never saw change.
            if knowlu_engine::sync::is_configured(&cs.vault) {
                let _io = cs.vault_io.lock().unwrap_or_else(|e| e.into_inner());
                let (_, _, totals) =
                    knowlu_engine::sync::run_lines_with(&cs.vault, knowlu_engine::sync::Direction::Push, "dashboard", None);
                // Fix round 1, review M4: a skip (signed out, no entitlement) is not a completed
                // push either. `QuitFlush`'s own contract is "this completed, never this was
                // attempted" — `errors.is_empty()` alone reads a skip as success, since a skip
                // raises no error.
                q.synced = totals.errors.is_empty() && totals.skipped.is_none();
            }
            let has_backup_dir = cs.settings.lock().unwrap_or_else(|e| e.into_inner()).backup_dir.is_some();
            if has_backup_dir {
                // `vault_io` around the engine call, as everywhere else a backup runs (F11).
                let _io = cs.vault_io.lock().unwrap_or_else(|e| e.into_inner());
                q.backed_up = run_backup(cs, jiff::Timestamp::now()).map(|st| st.last_error.is_none()).unwrap_or(false);
            }
            let _ = tx.send(q);
        });
        // A disconnected channel (the worker panicked) reads the same as the cap: something did not
        // finish, and the quit log says so.
        out = rx.recv_timeout(cap).unwrap_or(QuitFlush { synced: false, backed_up: false, timed_out: true });
        then(out);
    });
    out
}

/// `--vault`, else a working directory that holds `config/planning.yaml`, else an error naming
/// both (spec §3.3). Nothing here knows a path.
pub fn resolve_vault(arg: Option<&str>, cwd: &Path) -> Result<PathBuf, String> {
    if let Some(a) = arg { let p = PathBuf::from(a); return if p.join("config").join("planning.yaml").is_file() { Ok(p) } else { Err(format!("--vault {a}: no config/planning.yaml there")) }; }
    if cwd.join("config").join("planning.yaml").is_file() { return Ok(cwd.to_path_buf()); }
    Err(format!("no vault: pass --vault <path>, or start in a directory holding config/planning.yaml (started in {})", cwd.display()))
}
