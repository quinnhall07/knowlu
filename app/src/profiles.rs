//! Profiles: the registry, the per-profile app-data layout, the one-time move of the flat layout,
//! launch resolution and the per-profile instance guard (plan 4a, Task 2; spec §2, decision 2).
//!
//! A profile is **a vault plus its app data**. `id` is `ids::derived_id("profile", <vault path>)` —
//! the same call `Settings::load` has always made — so a vault opened by `--vault` and the same
//! vault registered by the wizard are one profile.
use std::path::{Path, PathBuf};

#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct Profile {
    pub id: String,
    pub name: String,
    pub vault: PathBuf,
    pub created_at: String,
    pub last_opened_at: Option<String>,
}

pub fn registry_path(root: &Path) -> PathBuf { root.join("profiles.json") }
pub fn profile_dir(root: &Path, id: &str) -> PathBuf { root.join("profiles").join(id) }
/// One folder for the whole install, not per profile: an update is the same bundle whichever
/// profile is open (Knowlu spec §6, "one bundle, one version").
pub fn updates_dir(root: &Path) -> PathBuf { root.join("updates") }

pub fn id_for(vault: &Path) -> String { knowlu_engine::ids::derived_id("profile", &vault.to_string_lossy()) }
/// The picker's default label: the vault folder's own name, never a person's name (R1).
pub fn default_name(vault: &Path) -> String {
    vault.file_name().map(|n| n.to_string_lossy().to_string()).unwrap_or_else(|| "Knowlu".to_string())
}

/// **Absent is not the same as unreadable** (review round 1, IMPORTANT 3). A registry that is not
/// there is `Ok(empty)` — that is every first launch. A registry that IS there and cannot be read
/// or parsed is `Err`, and every caller refuses rather than carrying on with an empty list: mapping
/// a bad read to "no profiles" meant the next `register` — which runs on every console launch —
/// rewrote the file and lost everyone else's profiles.
pub fn load(root: &Path) -> Result<Vec<Profile>, String> {
    let path = registry_path(root);
    let text = match std::fs::read_to_string(&path) {
        Ok(t) => t,
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => return Ok(Vec::new()),
        Err(e) => return Err(format!("{}: cannot be read ({e})", path.display())),
    };
    serde_json::from_str::<Vec<Profile>>(&text).map_err(|e| format!("{}: is not a profile registry ({e})", path.display()))
}

/// Through `ledger::dumps_value`, like every JSON the app writes — Python's `", "`/`": "`
/// separators, so a file this app wrote and a file the engine wrote never differ by whitespace.
///
/// **Written to a sibling temp file and renamed over the registry** (review round 1, IMPORTANT 3):
/// a plain `write` that dies half way leaves a truncated registry, which every later `load` then
/// refuses. `fs::rename` replaces an existing file on Windows, and a `profiles.json.tmp` left by a
/// crash is inert — `load` only ever reads `profiles.json`.
pub fn save(root: &Path, profiles: &[Profile]) -> Result<(), String> {
    std::fs::create_dir_all(root).map_err(|e| e.to_string())?;
    let v = serde_json::to_value(profiles).map_err(|e| e.to_string())?;
    let tmp = root.join("profiles.json.tmp");
    std::fs::write(&tmp, knowlu_engine::ledger::dumps_value(&v)).map_err(|e| e.to_string())?;
    std::fs::rename(&tmp, registry_path(root)).map_err(|e| {
        let _ = std::fs::remove_file(&tmp);
        e.to_string()
    })
}

/// Idempotent: registering a vault already in the registry updates its name and `last_opened_at`
/// and leaves `created_at` alone. Creates the per-profile folder so `ConsoleState::open` has one.
///
/// Refuses outright when the registry is there but unreadable — the `?` on `load` is the whole
/// point of it returning a `Result` (review round 1, IMPORTANT 3).
pub fn register(root: &Path, name: &str, vault: &Path) -> Result<Profile, String> {
    let id = id_for(vault);
    let now = knowlu_engine::journal::now_ts(None);
    let mut all = load(root)?;
    let p = match all.iter_mut().find(|p| p.id == id) {
        Some(e) => { e.name = name.to_string(); e.last_opened_at = Some(now); e.clone() }
        None => {
            let p = Profile { id, name: name.to_string(), vault: vault.to_path_buf(), created_at: now.clone(), last_opened_at: Some(now) };
            all.push(p.clone());
            p
        }
    };
    std::fs::create_dir_all(profile_dir(root, &p.id)).map_err(|e| e.to_string())?;
    save(root, &all)?;
    Ok(p)
}

/// What the move managed: the profile the flat files landed under, and the text of anything that
/// went wrong on the way. **`errors` is never swallowed** (review round 1, IMPORTANT 2) — every
/// caller prints it the way `ConsoleState::open` prints a bad `settings.json`, because a silent
/// failure here loses a user's settings, seen stamp and logs with nothing anywhere to say so.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Migrated {
    pub id: String,
    pub errors: Vec<String>,
}

/// The two FILES the flat layout ever held. `logs\` is handled separately, entry by entry: it is a
/// directory the app shares with other writers (see `migrate_flat_layout`).
const FLAT: [&str; 2] = ["settings.json", "seen.txt"];

/// **The app's own log files, and nothing else** (R-P4a-28). A slot writes `slot-<ts>-<n>-<step>.txt`
/// and the tray writes `quit-<ts>.txt`; `prune_logs` already keys on the first of those prefixes.
/// Anything else in that directory was put there by something that is not this app — on the machine
/// this was written, `scripts/dual-run.ps1` and the live Task Scheduler runner both write there —
/// and moving it would break whatever wrote it.
fn is_app_log(name: &str) -> bool { name.starts_with("slot-") || name.starts_with("quit-") }

/// The app's log files directly inside `logs`, oldest-first order irrelevant. Empty when the
/// directory is missing, which is the usual case.
fn app_logs_in(logs: &Path) -> Vec<PathBuf> {
    let Ok(rd) = std::fs::read_dir(logs) else { return Vec::new() };
    rd.flatten()
        .filter(|e| e.file_type().map(|t| t.is_file()).unwrap_or(false))
        .filter(|e| is_app_log(&e.file_name().to_string_lossy()))
        .map(|e| e.path())
        .collect()
}

/// Is there anything of ours left at `root` to fold? The two flat files, or one app log.
fn has_flat_layout(root: &Path) -> bool {
    FLAT.iter().any(|n| root.join(n).exists()) || !app_logs_in(&root.join("logs")).is_empty()
}

fn is_empty_dir(p: &Path) -> bool {
    std::fs::read_dir(p).map(|mut it| it.next().is_none()).unwrap_or(false)
}

/// One entry moved into the profile folder — or dropped, when the profile folder already holds that
/// name: a half-moved state re-run must keep the folded copy and discard the stale root one.
fn fold_one(src: &Path, dest: &Path, errors: &mut Vec<String>) {
    if dest.exists() {
        if let Err(e) = std::fs::remove_file(src).or_else(|_| std::fs::remove_dir_all(src)) {
            errors.push(format!("could not remove the superseded {} ({e})", src.display()));
        }
    } else if let Err(e) = std::fs::rename(src, dest) {
        errors.push(format!("could not move {} into {} ({e})", src.display(), dest.display()));
    }
}

fn profile_id_in(settings: &Path) -> Option<String> {
    std::fs::read_to_string(settings).ok()
        .and_then(|t| serde_json::from_str::<serde_json::Value>(&t).ok())
        .and_then(|v| v.get("profile_id").and_then(|p| p.as_str()).map(str::to_string))
}

/// **F14: a move, not a fresh start** — but the move is of OUR FILES, never of the directory
/// (**R-P4a-28**). The flat `settings.json` and `seen.txt` fold into `profiles\<id>\`, the app's own
/// `slot-*` and `quit-*` logs fold into `profiles\<id>\logs\` one file at a time, and a one-entry
/// registry is written. Runs at most once per machine in practice, and **must run before anything
/// creates the new root** — which is why `state::app_data_root()` calls it first. `vault_hint` is
/// the resolved `--vault`: the old `settings.json` carries the profile id but not the vault path.
///
/// **`%LOCALAPPDATA%\quinn-ops` is not ours to rename** (final review, CRITICAL C1). It used to be
/// renamed whole to `knowlu` whenever `knowlu` did not exist yet, which is true on exactly the
/// launch this runs. On the machine this was written that directory ALSO holds `dual\` (the
/// cutover harness's snapshots), `logs\` (written twice a day by the live Task Scheduler runner and
/// by `scripts/dual-run.ps1`), `rehearsal\`, `scratch\` (the scratch vaults `scripts/scratch-vault.ps1`
/// prints `--vault` lines for) and `shots\`. One first launch would have moved all of it mid-cutover
/// week: the scripts would have recreated the old root and split the evidence trail in two, and a
/// `--vault <scratch path>` launch would then have registered a profile at a path the rename had
/// just invalidated — fatal, on every launch after. So there is no rename, only the fold, and
/// anything in `quinn-ops` that is not ours is left exactly where it is.
///
/// The fold still reads from BOTH roots — `knowlu` (half-folded by an earlier crash, or by a build
/// that did rename) and `quinn-ops` — and is idempotent from either side (S5). The old root is
/// removed only if the fold has left it completely empty, and whatever could not be done is carried
/// out in `errors` rather than swallowed (review round 1, IMPORTANT 2).
pub fn migrate_flat_layout(base: &Path, vault_hint: Option<&Path>) -> Option<Migrated> {
    let old = base.join("quinn-ops");
    let new = base.join("knowlu");
    let mut errors = Vec::new();
    let sources: Vec<PathBuf> = [new.clone(), old.clone()].into_iter().filter(|d| d.is_dir()).collect();
    if !sources.iter().any(|d| has_flat_layout(d)) { return None; }
    // The id comes from whichever settings file survives, else from the launch's own vault. If
    // neither names one the files are still MOVED, under a fallback id: leaving them at a root
    // nothing reads any more would silently drop the backup folder the user picked, their autostart
    // choice, their seen stamp and their slot logs (S5). The backups themselves live wherever that
    // setting points and are never touched here.
    let id = sources.iter().find_map(|d| profile_id_in(&d.join("settings.json")))
        .or_else(|| vault_hint.map(id_for))
        .unwrap_or_else(|| "profile_legacy".to_string());
    let dir = profile_dir(&new, &id);
    if let Err(e) = std::fs::create_dir_all(&dir) {
        errors.push(format!("could not create {} ({e}) — the old layout was left untouched", dir.display()));
        return Some(Migrated { id, errors });
    }
    for src_root in &sources {
        for name in FLAT {
            let src = src_root.join(name);
            if !src.exists() { continue; }
            fold_one(&src, &dir.join(name), &mut errors);
        }
        // `logs\` is a SHARED directory: only the files this app writes move, and the directory
        // itself is never renamed. It goes only when our files were all it held.
        let logs = src_root.join("logs");
        let ours = app_logs_in(&logs);
        if !ours.is_empty() {
            let dest_logs = dir.join("logs");
            if let Err(e) = std::fs::create_dir_all(&dest_logs) {
                errors.push(format!("could not create {} ({e}) — the logs were left where they are", dest_logs.display()));
            } else {
                for src in &ours {
                    let Some(name) = src.file_name() else { continue };
                    fold_one(src, &dest_logs.join(name), &mut errors);
                }
            }
        }
        if logs.is_dir() && is_empty_dir(&logs) { let _ = std::fs::remove_dir(&logs); }
    }
    // Only when the fold emptied it — the whole point of C1 is that everything else stays put.
    if old.is_dir() && is_empty_dir(&old) { let _ = std::fs::remove_dir(&old); }
    if let Some(v) = vault_hint {
        if let Err(e) = register(&new, &default_name(v), v) { errors.push(e); }
    }
    Some(Migrated { id, errors })
}

#[derive(Debug)]
pub enum Launch {
    Onboard,
    Open(Profile),
    Pick(Vec<Profile>),
    /// The registry is THERE and cannot be read. A fourth outcome, not an empty one: carrying on as
    /// if there were no profiles would rewrite the file on the next `register` and lose them all
    /// (review round 1, IMPORTANT 3). `main` turns this into the same `MessageBoxW` a bad `--vault`
    /// gets, and nothing is written.
    Broken(String),
}

/// `--vault` keeps working exactly as today and always wins — it is how the Start-menu shortcut
/// and autostart launch a profile, registered or not. Without it: none → onboarding, one → open
/// it, more → the picker (spec §2).
pub fn resolve_launch(root: &Path, vault_arg: Option<&Path>) -> Launch {
    let all = match load(root) { Ok(v) => v, Err(e) => return Launch::Broken(e) };
    if let Some(v) = vault_arg {
        let id = id_for(v);
        if let Some(p) = all.into_iter().find(|p| p.id == id) { return Launch::Open(p); }
        return Launch::Open(Profile { id, name: default_name(v), vault: v.to_path_buf(), created_at: knowlu_engine::journal::now_ts(None), last_opened_at: None });
    }
    match all.len() {
        0 => Launch::Onboard,
        1 => Launch::Open(all.into_iter().next().expect("len 1")),
        _ => Launch::Pick(all),
    }
}

/// **A vault that cannot be opened is not the end of the launch** (final review, I1). `run_console`
/// resolves the vault before any window exists, and a profile whose folder has since been moved,
/// renamed or deleted fails that check — which used to be a message box and exit 2, with no way
/// left to reach the other profiles on the machine and no way to repair the broken one.
///
/// So the launch falls back to the shell `resolve_launch` would have chosen with no `--vault` at
/// all: `Some(profiles)` — the picker — whenever the registry still holds one. `None` means there is
/// nothing to fall back TO (an empty registry, or one that is there and cannot be read, which
/// `load` refuses on purpose), and the caller keeps its fatal: with no profile registered the
/// message box IS the whole story.
pub fn fallback_shell(root: &Path) -> Option<Vec<Profile>> {
    match load(root) {
        Ok(ps) if !ps.is_empty() => Some(ps),
        _ => None,
    }
}

/// **One Knowlu per machine** (R-P4a-1): `tauri-plugin-single-instance` stays exactly as it is,
/// keyed on the app identifier, and a second launch of any profile is killed by it. Switching
/// profiles is therefore a relaunch, and the child is handed `--after-pid <parent>` so it can wait
/// for the plugin's lock to be free before it registers. Polls every 100 ms up to `cap`; returns
/// `false` if the pid is still there at the cap — the launch goes ahead anyway, and the plugin
/// simply closes the loser, which is a worse outcome than waiting but better than hanging.
///
/// `OpenProcess` failing means the pid is gone (or is not ours to look at, which for our own
/// former parent is the same thing). A live handle is checked with a zero-timeout wait rather than
/// an exit code, so a pid reused by an unrelated process cannot read as "still running forever".
#[cfg(windows)]
pub fn wait_for_pid_gone(pid: u32, cap: std::time::Duration) -> bool {
    use windows::Win32::Foundation::{CloseHandle, WAIT_OBJECT_0};
    use windows::Win32::System::Threading::{
        OpenProcess, WaitForSingleObject, PROCESS_QUERY_LIMITED_INFORMATION, PROCESS_SYNCHRONIZE,
    };
    let started = std::time::Instant::now();
    loop {
        // SAFETY: a plain handle open/close pair; every branch closes what it opened.
        let gone = unsafe {
            match OpenProcess(PROCESS_SYNCHRONIZE | PROCESS_QUERY_LIMITED_INFORMATION, false, pid) {
                Err(_) => true,
                Ok(h) => {
                    let signalled = WaitForSingleObject(h, 0) == WAIT_OBJECT_0;
                    let _ = CloseHandle(h);
                    signalled
                }
            }
        };
        if gone { return true; }
        if started.elapsed() >= cap { return false; }
        std::thread::sleep(std::time::Duration::from_millis(100));
    }
}
#[cfg(not(windows))]
pub fn wait_for_pid_gone(_pid: u32, _cap: std::time::Duration) -> bool { true }
