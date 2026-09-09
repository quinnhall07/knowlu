use knowlu::profiles::{
    default_name, fallback_shell, id_for, load, migrate_flat_layout, profile_dir, register,
    resolve_launch, save, wait_for_pid_gone, Launch, Profile,
};
use knowlu::state::resolve_vault;
use std::path::{Path, PathBuf};

fn temp_root(tag: &str) -> PathBuf {
    let d = std::env::temp_dir().join(format!("knowlu-profiles-{tag}-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&d);
    std::fs::create_dir_all(&d).unwrap();
    d
}

#[test]
fn the_registry_round_trips_and_a_vault_registered_twice_stays_one_profile() {
    let root = temp_root("registry");
    let vault = root.join("vaults").join("Ada");
    std::fs::create_dir_all(&vault).unwrap();
    let p = register(&root, &default_name(&vault), &vault).unwrap();
    assert_eq!(p.id, id_for(&vault));
    assert_eq!(p.name, "Ada", "the folder name is the default label");
    assert!(profile_dir(&root, &p.id).is_dir(), "the per-profile folder exists");
    let again = register(&root, "Renamed", &vault).unwrap();
    assert_eq!(again.id, p.id);
    let all = load(&root).unwrap();
    assert_eq!(all.len(), 1, "registering the same vault twice is one profile");
    assert_eq!(all[0].name, "Renamed");
    assert_eq!(all[0].created_at, p.created_at, "created_at is never rewritten");
    // Written through ledger::dumps_value: Python's separators, not serde's minimal ones.
    let text = std::fs::read_to_string(root.join("profiles.json")).unwrap();
    assert!(text.contains("\", \"") || text.contains("\": \""), "dumps_value separators: {text}");
    // A round trip through save(), and a missing registry that is empty rather than an error.
    let manual = Profile { id: "profile_0123456789".into(), name: "X".into(), vault: PathBuf::from("C:\\x"), created_at: "2026-09-05T00:00:00Z".into(), last_opened_at: None };
    save(&root, std::slice::from_ref(&manual)).unwrap();
    assert_eq!(load(&root).unwrap(), vec![manual]);
    assert!(load(Path::new("C:\\nowhere-at-all")).unwrap().is_empty(), "an absent registry is empty, not an error");
    // Review round 1, IMPORTANT 3: the write is a temp file renamed over the registry, so nothing
    // is left behind for a later `load` to trip over.
    assert!(!root.join("profiles.json.tmp").exists(), "the temp file is renamed, not left");
}

#[test]
fn the_flat_files_fold_into_the_profile_folder_and_nothing_is_lost() {
    let base = temp_root("f14");
    let old = base.join("quinn-ops");
    std::fs::create_dir_all(old.join("logs")).unwrap();
    let vault = base.join("v");
    std::fs::create_dir_all(&vault).unwrap();
    let id = id_for(&vault);
    std::fs::write(old.join("settings.json"), format!("{{\"profile_id\": \"{id}\", \"backup_dir\": null, \"autostart\": true, \"quit_at\": null}}")).unwrap();
    std::fs::write(old.join("seen.txt"), "2026-09-04T12:00:00Z").unwrap();
    std::fs::write(old.join("logs").join("slot-x.txt"), "kept").unwrap();

    let moved = migrate_flat_layout(&base, Some(&vault)).expect("the move happened");
    assert_eq!(moved.id, id);
    assert!(moved.errors.is_empty(), "a clean move reports nothing: {:?}", moved.errors);
    let new = base.join("knowlu");
    // Here — and ONLY here — the old root goes: our three files were all it held, so the fold left
    // an empty directory behind it. Nothing was renamed to get that (R-P4a-28).
    assert!(!old.exists(), "a root the fold emptied is removed, not left as a stub");
    assert_eq!(std::fs::read_to_string(profile_dir(&new, &id).join("seen.txt")).unwrap(), "2026-09-04T12:00:00Z");
    assert!(profile_dir(&new, &id).join("settings.json").is_file());
    assert_eq!(std::fs::read_to_string(profile_dir(&new, &id).join("logs").join("slot-x.txt")).unwrap(), "kept");
    assert_eq!(load(&new).unwrap().len(), 1, "the registry gained the one profile");
    assert!(migrate_flat_layout(&base, Some(&vault)).is_none(), "a second launch moves nothing");
    assert_eq!(std::fs::read_to_string(profile_dir(&new, &id).join("seen.txt")).unwrap(), "2026-09-04T12:00:00Z", "…and nothing was disturbed by the second run");
}

/// **CRITICAL C1 / R-P4a-28: `%LOCALAPPDATA%\quinn-ops` is not this app's directory to move.** It
/// used to be renamed whole to `knowlu` on the first launch of a build with profiles. On the machine
/// this was written that same directory holds `dual\` (the cutover harness's snapshots), a `logs\`
/// the LIVE Task Scheduler runner and `scripts/dual-run.ps1` write to twice a day, `rehearsal\`,
/// `scratch\` (the scratch vaults the console is demoed against) and `shots\` — so one launch would
/// have moved the evidence trail of a cutover week out from under the scripts still writing to it,
/// and invalidated every `--vault <scratch path>` line at the same time.
///
/// Each planted decoy stands for one of those: a foreign directory, a foreign file in the SHARED
/// `logs\`, and a scratch vault. All three must still be at their original paths, byte for byte,
/// with only `slot-*`/`quit-*` and the two flat files folded away.
#[test]
fn nothing_but_the_apps_own_files_is_ever_moved_out_of_the_old_root() {
    let base = temp_root("c1");
    let old = base.join("quinn-ops");
    std::fs::create_dir_all(old.join("logs")).unwrap();
    std::fs::create_dir_all(old.join("dual")).unwrap();
    std::fs::create_dir_all(old.join("scratch").join("v").join("config")).unwrap();
    let vault = base.join("v");
    std::fs::create_dir_all(&vault).unwrap();
    let id = id_for(&vault);
    std::fs::write(old.join("settings.json"), format!("{{\"profile_id\": \"{id}\", \"backup_dir\": null, \"autostart\": true, \"quit_at\": null}}")).unwrap();
    std::fs::write(old.join("seen.txt"), "2026-09-04T12:00:00Z").unwrap();
    std::fs::write(old.join("dual").join("keep.txt"), "the harness's").unwrap();
    std::fs::write(old.join("logs").join("dual-20260903.txt"), "dual-run.ps1's").unwrap();
    std::fs::write(old.join("logs").join("slot-20260903-1200.txt"), "the app's").unwrap();
    std::fs::write(old.join("logs").join("quit-20260903T1201.txt"), "the tray's").unwrap();
    std::fs::write(old.join("scratch").join("v").join("config").join("planning.yaml"), "daily_hours: 4\n").unwrap();

    let moved = migrate_flat_layout(&base, Some(&vault)).expect("the fold happened");
    assert!(moved.errors.is_empty(), "{:?}", moved.errors);
    let new = base.join("knowlu");
    let prof = profile_dir(&new, &id);
    // What is OURS moved.
    assert!(prof.join("settings.json").is_file(), "settings.json folded");
    assert_eq!(std::fs::read_to_string(prof.join("seen.txt")).unwrap(), "2026-09-04T12:00:00Z");
    assert_eq!(std::fs::read_to_string(prof.join("logs").join("slot-20260903-1200.txt")).unwrap(), "the app's");
    assert_eq!(std::fs::read_to_string(prof.join("logs").join("quit-20260903T1201.txt")).unwrap(), "the tray's");
    assert!(!old.join("settings.json").exists() && !old.join("seen.txt").exists());
    // Everything else is still exactly where its own writer put it.
    assert!(old.is_dir(), "the old root survives — it is not ours to remove while it holds someone else's files");
    assert_eq!(std::fs::read_to_string(old.join("dual").join("keep.txt")).unwrap(), "the harness's");
    assert_eq!(std::fs::read_to_string(old.join("logs").join("dual-20260903.txt")).unwrap(), "dual-run.ps1's");
    assert_eq!(std::fs::read_to_string(old.join("scratch").join("v").join("config").join("planning.yaml")).unwrap(), "daily_hours: 4\n");
    assert!(old.join("logs").is_dir(), "a logs directory someone else is still writing to stays");
    assert!(!prof.join("logs").join("dual-20260903.txt").exists(), "a foreign log was carried off");
    assert!(!new.join("dual").exists() && !new.join("scratch").exists(), "nothing foreign reached the new root");
    // And it is done: a second launch finds nothing of ours left, however much else is there.
    assert!(migrate_flat_layout(&base, Some(&vault)).is_none(), "the foreign files alone are not a migration");
}

/// S5: the fold reads both roots. Flat files sitting at `knowlu` itself — left by a build that did
/// rename the old root, or by a crash part-way through an earlier fold — must not be stranded there,
/// and a run with nothing to derive an id from must still move them.
#[test]
fn a_half_moved_layout_is_folded_on_the_next_launch_and_never_stranded() {
    let base = temp_root("halfmoved");
    let new = base.join("knowlu");
    std::fs::create_dir_all(new.join("logs")).unwrap();
    std::fs::write(new.join("seen.txt"), "2026-09-04T12:00:00Z").unwrap();
    std::fs::write(new.join("logs").join("slot-x.txt"), "kept").unwrap();
    // No settings.json and no vault hint: nothing names an id, and the files still move.
    let id = migrate_flat_layout(&base, None).expect("the fold ran on a half-folded root").id;
    assert_eq!(id, "profile_legacy");
    assert!(!new.join("seen.txt").exists(), "nothing is left at the root");
    assert_eq!(std::fs::read_to_string(profile_dir(&new, &id).join("seen.txt")).unwrap(), "2026-09-04T12:00:00Z");
    assert_eq!(std::fs::read_to_string(profile_dir(&new, &id).join("logs").join("slot-x.txt")).unwrap(), "kept");
    assert!(migrate_flat_layout(&base, None).is_none(), "and it is done");
}

#[test]
fn launch_resolution_is_none_one_many_and_the_flag_always_wins() {
    let root = temp_root("launch");
    assert!(matches!(resolve_launch(&root, None), Launch::Onboard));
    let a = root.join("A"); let b = root.join("B");
    std::fs::create_dir_all(&a).unwrap(); std::fs::create_dir_all(&b).unwrap();
    register(&root, "A", &a).unwrap();
    match resolve_launch(&root, None) { Launch::Open(p) => assert_eq!(p.vault, a), other => panic!("{other:?}") }
    register(&root, "B", &b).unwrap();
    match resolve_launch(&root, None) { Launch::Pick(ps) => assert_eq!(ps.len(), 2), other => panic!("{other:?}") }
    // --vault wins over everything, registered or not (the Start-menu shortcut and autostart).
    let c = root.join("C"); std::fs::create_dir_all(&c).unwrap();
    match resolve_launch(&root, Some(&c)) { Launch::Open(p) => { assert_eq!(p.vault, c); assert_eq!(p.id, id_for(&c)); } other => panic!("{other:?}") }
    match resolve_launch(&root, Some(&b)) { Launch::Open(p) => assert_eq!(p.name, "B"), other => panic!("{other:?}") }
}

/// I1 (final review): a registered profile whose vault folder is gone — moved, renamed, or on a USB
/// stick that is not in the machine — must not end the launch at a message box. `run_console`
/// resolves the vault first and, when that fails, asks `fallback_shell` what to show instead: the
/// picker whenever the registry still holds a profile, so the OTHER profiles stay reachable and this
/// one can be fixed. With nothing registered there is nothing to fall back to, and the fatal stands.
#[test]
fn a_profile_whose_vault_is_gone_falls_back_to_the_picker() {
    let root = temp_root("i1");
    let vault = root.join("gone");
    std::fs::create_dir_all(vault.join("config")).unwrap();
    std::fs::create_dir_all(vault.join("tasks")).unwrap();
    std::fs::write(vault.join("config").join("planning.yaml"), "daily_hours: 4\n").unwrap();
    let p = register(&root, "Gone", &vault).unwrap();
    // Nothing to fall back to yet is not this test's case — but an empty registry must answer None,
    // or `run_console` would show an empty picker instead of saying what went wrong.
    assert!(fallback_shell(&root.join("empty")).is_none(), "no registry, no fallback");

    // The folder goes, the way a rename or an unplugged drive takes it.
    std::fs::remove_dir_all(&vault).unwrap();
    assert!(resolve_vault(vault.to_str(), Path::new("C:\\nowhere")).is_err(), "the vault really cannot be opened");
    let ps = fallback_shell(&root).expect("a registered profile is something to fall back to");
    assert_eq!(ps.len(), 1);
    assert_eq!(ps[0].id, p.id, "and it is the profile itself the picker will offer");

    // A registry that is THERE and unreadable is not an empty one: `load` refuses it, so the launch
    // keeps its fatal rather than opening a picker built from a file it could not read.
    std::fs::write(root.join("profiles.json"), "{ not a registry").unwrap();
    assert!(fallback_shell(&root).is_none(), "an unreadable registry is no fallback either");
}

/// R-P4a-1: one Knowlu per machine. Switching profiles is a relaunch, and the child must not
/// register with `tauri-plugin-single-instance` until the parent has actually exited — otherwise
/// the plugin kills the new window and the user is left with nothing.
#[test]
fn the_relaunched_child_waits_for_the_old_process_to_go_and_gives_up_at_the_cap() {
    // A real short-lived process, so a real pid that really disappears. `cmd /c exit` is the
    // cheapest one on this machine and touches nothing.
    let mut child = std::process::Command::new("cmd").args(["/c", "exit"]).spawn().unwrap();
    let pid = child.id();
    child.wait().unwrap();
    let t0 = std::time::Instant::now();
    assert!(wait_for_pid_gone(pid, std::time::Duration::from_secs(10)), "an exited pid is gone");
    assert!(t0.elapsed() < std::time::Duration::from_secs(2), "and it returns at once, not at the cap");

    // Our own pid never goes: the wait is bounded and gives up rather than hanging the launch.
    let t0 = std::time::Instant::now();
    assert!(!wait_for_pid_gone(std::process::id(), std::time::Duration::from_millis(400)));
    assert!(t0.elapsed() >= std::time::Duration::from_millis(400) && t0.elapsed() < std::time::Duration::from_secs(3));
}

/// Review round 1, IMPORTANT 2, restated for the file-by-file fold (R-P4a-28). A whole-directory
/// rename is the thing an open file can block: Windows refuses to rename a directory that holds one,
/// and the fold used to be the fallback for exactly that failure. Now it is the only path, so the
/// held handle below — on a file that is not ours, in the root we no longer touch — must cost
/// nothing at all: every flat file still arrives, and there is nothing to report.
#[test]
fn a_file_someone_else_holds_open_costs_the_fold_nothing() {
    let base = temp_root("busyroot");
    let old = base.join("quinn-ops");
    std::fs::create_dir_all(old.join("logs")).unwrap();
    let vault = base.join("v");
    std::fs::create_dir_all(&vault).unwrap();
    let id = id_for(&vault);
    std::fs::write(old.join("settings.json"), format!("{{\"profile_id\": \"{id}\", \"backup_dir\": null, \"autostart\": true, \"quit_at\": null}}")).unwrap();
    std::fs::write(old.join("seen.txt"), "2026-09-04T12:00:00Z").unwrap();
    std::fs::write(old.join("logs").join("slot-x.txt"), "kept").unwrap();
    let held = std::fs::File::create(old.join("busy.txt")).unwrap();

    let moved = migrate_flat_layout(&base, Some(&vault)).expect("the fold ran");
    let new = base.join("knowlu");
    assert_eq!(moved.id, id, "the id still comes from the old settings file");
    assert!(moved.errors.is_empty(), "nothing was in the way: {:?}", moved.errors);
    assert!(old.is_dir(), "the old root stays — someone else's open file is still in it");
    assert!(old.join("busy.txt").is_file(), "and that file was neither moved nor removed");
    // F14's one job: settings, the seen stamp and the logs all arrive.
    assert!(profile_dir(&new, &id).join("settings.json").is_file());
    assert_eq!(std::fs::read_to_string(profile_dir(&new, &id).join("seen.txt")).unwrap(), "2026-09-04T12:00:00Z");
    assert_eq!(std::fs::read_to_string(profile_dir(&new, &id).join("logs").join("slot-x.txt")).unwrap(), "kept");
    assert_eq!(load(&new).unwrap().len(), 1, "and the registry gained the profile");
    assert!(!old.join("seen.txt").exists() && !old.join("logs").exists(), "nothing of ours is left behind in the old root");
    assert!(migrate_flat_layout(&base, Some(&vault)).is_none(), "and the next launch has nothing left to move");
    drop(held);
}

/// A `knowlu` an earlier `app_data_root()` call created before the fold ever ran is simply the root
/// the fold folds INTO. (It used to be the thing that blocked the rename permanently — review round
/// 1, IMPORTANT 2 — and with the rename gone, R-P4a-28, it cannot block anything.)
#[test]
fn an_empty_knowlu_left_by_an_earlier_launch_does_not_block_the_fold() {
    let base = temp_root("emptynew");
    let old = base.join("quinn-ops");
    std::fs::create_dir_all(&old).unwrap();
    std::fs::write(old.join("seen.txt"), "2026-09-04T12:00:00Z").unwrap();
    let new = base.join("knowlu");
    std::fs::create_dir_all(&new).unwrap();

    let moved = migrate_flat_layout(&base, None).expect("the fold happened anyway");
    assert!(moved.errors.is_empty(), "{:?}", moved.errors);
    assert!(!old.exists(), "the seen stamp was all it held, so the emptied root goes");
    assert_eq!(std::fs::read_to_string(profile_dir(&new, &moved.id).join("seen.txt")).unwrap(), "2026-09-04T12:00:00Z");
}

/// Review round 1, IMPORTANT 3. `load` mapping an unreadable registry to an empty list meant the
/// `register` every console launch performs rewrote the file — one bad read and every profile on
/// the machine was gone. A registry that is THERE and cannot be read is refused, and left alone.
#[test]
fn a_corrupt_registry_is_refused_and_never_rewritten() {
    let root = temp_root("corrupt");
    let vault = root.join("V");
    std::fs::create_dir_all(&vault).unwrap();
    std::fs::write(root.join("profiles.json"), "{ this is not a profile registry").unwrap();
    let before = std::fs::read(root.join("profiles.json")).unwrap();

    let e = load(&root).expect_err("present-but-unparsable is an error, not an empty list");
    assert!(e.contains("profiles.json"), "the message names the file: {e}");
    let e = register(&root, "V", &vault).expect_err("register refuses rather than rebuilding");
    assert!(e.contains("profiles.json"), "{e}");
    assert_eq!(std::fs::read(root.join("profiles.json")).unwrap(), before, "the bad file is left exactly as it was");
    // Both launch paths stop dead — `main` turns Broken into the same message box a bad --vault gets.
    match resolve_launch(&root, None) { Launch::Broken(m) => assert!(m.contains("profiles.json"), "{m}"), other => panic!("{other:?}") }
    match resolve_launch(&root, Some(&vault)) { Launch::Broken(_) => {} other => panic!("{other:?}") }
}

/// The save is a temp file renamed over the registry, so a crash mid-write leaves a stale
/// `profiles.json.tmp` and an intact `profiles.json` — never a truncated registry that every later
/// launch then refuses (review round 1, IMPORTANT 3).
#[test]
fn a_half_written_temp_file_never_shadows_the_good_registry() {
    let root = temp_root("tmpleftover");
    let a = root.join("A");
    std::fs::create_dir_all(&a).unwrap();
    register(&root, "A", &a).unwrap();
    std::fs::write(root.join("profiles.json.tmp"), "{ half written").unwrap();

    assert_eq!(load(&root).expect("the good registry still reads").len(), 1);
    let b = root.join("B");
    std::fs::create_dir_all(&b).unwrap();
    register(&root, "B", &b).unwrap();
    assert_eq!(load(&root).unwrap().len(), 2, "and the next write lands cleanly over it");
    assert!(!root.join("profiles.json.tmp").exists(), "the stale temp is consumed by the rename");
}
