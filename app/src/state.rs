//! What the shell holds between commands: the vault path, the per-device "last looked" stamp,
//! settings, sync/backup/run status, and one lock so console operations on the vault are
//! serialised (spec §3.2; Knowlu plan 1, Task 7 adds settings and the status fields Task 8+ fill).
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, AtomicUsize};
use std::sync::Mutex;
use knowlu_engine::{backup::BackupStatus, history::HistoryStatus};

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
    /// **Vault I/O, serialised (console spec §8 the sync lock, §9 the race).** `history::sync`
    /// rewrites the working tree under everyone's feet — a rebase checks files out, `reconcile`
    /// re-applies journal records — while a console write is single-line surgery on a note it has
    /// just read. Interleave the two and the write lands on a file the rebase is about to replace.
    /// Every path that touches the vault takes this one lock: `mutate` for the duration of a write,
    /// `run_sync` for the duration of `history::sync`, so housekeeping, a scheduler slot's own
    /// pull/push and the `sync` command all queue behind each other instead of racing.
    ///
    /// **Ordering, and it is one-way:** `vault_io` is taken BEFORE `lock`, never while `lock` is
    /// held. `lock` alone still guards the read polls, so a `state` poll never waits on a git
    /// fetch. And `vault_io` is never held across a child-process wait — a scheduler slot takes it
    /// inside each of its own sync steps, never around `run_child`, which may run for 20 minutes.
    pub vault_io: Mutex<()>,
    pub settings: Mutex<Settings>,
    /// `Some` when `settings.json` existed but did not parse at open — names the path and the
    /// error. Fixed at open; never mutated afterward, so no `Mutex` is needed.
    pub settings_error: Option<String>,
    pub session: String,
    pub head_sha: Mutex<Option<String>>,
    pub history: Mutex<HistoryStatus>,
    pub backup: Mutex<BackupStatus>,
    pub last_write: Mutex<Option<std::time::Instant>>,
    pub pending_edits: AtomicUsize,
    pub auto_sync: AtomicBool,
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
            head_sha: Mutex::new(None),
            history: Mutex::new(HistoryStatus::default()),
            backup: Mutex::new(BackupStatus { last_ok: None, behind_days: None, last_error: None, target_reachable: false }),
            last_write: Mutex::new(None),
            pending_edits: AtomicUsize::new(0),
            auto_sync: AtomicBool::new(true),
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
    /// `last_write` for whatever debounces the next sync (Task 8+).
    pub fn note_write(&self) {
        *self.last_write.lock().unwrap() = Some(std::time::Instant::now());
        self.pending_edits.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
    }
}

/// Re-reads history status from the vault into the cache. Cheap (a handful of `git` calls) —
/// called after any command that might have changed the working tree, and stands alone so
/// `commands.rs` never has to know how the cache is refreshed.
pub fn refresh_history(cs: &ConsoleState) {
    let s = knowlu_engine::history::status(&cs.vault);
    *cs.history.lock().unwrap() = s;
}

/// Re-reads the vault's git HEAD into the cache (`knowlu_engine::runs::git_sha`, a subprocess with a
/// 5 s deadline) so `build_state_value` can compare it against `CONSOLE_BUILD` without ever
/// spawning git on the UI thread itself. `None` for a non-git vault — never a false alarm.
pub fn refresh_head(cs: &ConsoleState) {
    let h = knowlu_engine::runs::git_sha(&cs.vault);
    *cs.head_sha.lock().unwrap() = h;
}

/// Commit-by-name, then pull/push as needed (`history::sync`), against a fresh journal — never
/// under `cs.lock` (git and file I/O can take seconds and must not block a poll). Swaps
/// `pending_edits` to 0 so a race that adds an edit mid-sync is not lost (the swap happens before
/// the engine call reads anything from disk). A non-empty `conflicted` list turns auto-sync off:
/// a device that just fought a merge conflict does not get to retry unattended.
pub fn run_sync(cs: &ConsoleState) -> knowlu_engine::history::SyncOutcome {
    // `vault_io`, for the whole of the engine call — a sync rewrites the working tree, and a
    // console write must never land inside a rebase (see the field's own doc). Poison-tolerant:
    // this returns a `SyncOutcome`, not a `Result`, and a panicked writer elsewhere must not turn
    // every later sync into a silent no-op.
    let _io = cs.vault_io.lock().unwrap_or_else(|e| e.into_inner());
    let edits = cs.pending_edits.swap(0, std::sync::atomic::Ordering::SeqCst);
    let mut journal = knowlu_engine::journal::Journal::new(&cs.vault);
    let out = knowlu_engine::history::sync(&cs.vault, &crate::commands::console_ctx(), &mut journal, edits);
    // A sync that never ran — another process holds `state/.sync.lock` — has not spent those
    // edits. Give them back, or the next commit message undercounts what it carries and the
    // housekeeping thread's `pending_edits > 0` gate stops firing for edits still on disk.
    if out.status.last_error.as_deref().map(|e| e.contains("another sync holds")).unwrap_or(false) {
        cs.pending_edits.fetch_add(edits, std::sync::atomic::Ordering::SeqCst);
    }
    if !out.status.conflicted.is_empty() {
        cs.auto_sync.store(false, std::sync::atomic::Ordering::SeqCst);
    }
    *cs.history.lock().unwrap() = out.status.clone();
    out
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
/// never "this was attempted": a vault with no remote quits `synced: false`, and that is not an
/// error — there was nothing to push.
#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize)]
pub struct QuitFlush {
    pub synced: bool,
    pub backed_up: bool,
    pub timed_out: bool,
}

/// Push on close (console spec §8) and back up on quit (Knowlu spec §4), synchronously, bounded by
/// `cap` — F10. The two engine calls run on a scoped worker and the caller waits at most `cap`: a
/// hung remote must never hold the Quit click hostage.
///
/// `then` is called exactly once, on the caller's thread, and **before the scope joins** — with the
/// worker's result when it arrives, or with `timed_out: true` at the cap. That ordering is the
/// whole design: the tray hands in a `then` that writes the quit log and ends the process, so a
/// worker still stuck inside git never delays the quit past the cap (the scope's join is exactly
/// the unbounded wait the cap exists to prevent). Nothing is lost when the cap fires — the edits
/// are on disk and in the journal, and the next launch's housekeeping picks them up.
///
/// `run_sync` is skipped outright on a vault with no remote or with auto-sync off: the first has
/// nothing to push, and the second was turned off by a merge conflict, which is not a state to
/// retry unattended at quit time.
pub fn quit_flush(cs: &ConsoleState, cap: std::time::Duration, then: impl FnOnce(QuitFlush) + Send) -> QuitFlush {
    use std::sync::atomic::Ordering::SeqCst;
    let (tx, rx) = std::sync::mpsc::channel::<QuitFlush>();
    let mut out = QuitFlush { synced: false, backed_up: false, timed_out: false };
    std::thread::scope(|s| {
        s.spawn(move || {
            let mut q = QuitFlush { synced: false, backed_up: false, timed_out: false };
            let has_remote = cs.history.lock().unwrap_or_else(|e| e.into_inner()).has_remote;
            if has_remote && cs.auto_sync.load(SeqCst) {
                q.synced = run_sync(cs).status.last_error.is_none();
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
