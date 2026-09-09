//! The two "never wrote there" tests (plan 4a, Task 6; spec §8, S7). Both drive the handle-free
//! cores in `app/src/onboarding.rs`, so they need no Tauri app and no window: adopting and
//! restoring are the two paths where the app touches a folder the USER already owns, and the
//! promise on both is the same — it is read, never written.
use knowlu::onboarding::{
    adopt_vault_in, apply_profile_settings_in, create_vault_in, dest_for, finish_or_roll_back,
    offer_marker, restore_vault_in, WizardPlan,
};
use std::path::{Path, PathBuf};

fn tmp(tag: &str) -> PathBuf {
    let d = std::env::temp_dir().join(format!("knowlu-onb-{tag}-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&d);
    std::fs::create_dir_all(&d).unwrap();
    d
}
fn copy(from: &Path, to: &Path) {
    std::fs::create_dir_all(to).unwrap();
    for e in std::fs::read_dir(from).unwrap().flatten() {
        let (p, t) = (e.path(), to.join(e.file_name()));
        if p.is_dir() { copy(&p, &t); } else { std::fs::copy(&p, &t).unwrap(); }
    }
}
/// Every file under `root`, as (vault-relative path, bytes), sorted — a whole-tree fingerprint.
/// Content, not mtimes: a copy that rewrote a file byte-identically is not a write worth failing.
fn fingerprint(root: &Path) -> Vec<(String, Vec<u8>)> {
    let mut out = Vec::new();
    let mut stack = vec![root.to_path_buf()];
    while let Some(dir) = stack.pop() {
        for e in std::fs::read_dir(&dir).unwrap().flatten() {
            let p = e.path();
            if p.is_dir() { stack.push(p); }
            else { out.push((p.strip_prefix(root).unwrap().to_string_lossy().to_string(), std::fs::read(&p).unwrap())); }
        }
    }
    out.sort();
    out
}

/// Spec §8: adopting `tests/fixtures/vault-s1` as a profile **writes nothing into it** — no
/// scaffold, no seed note, no first task, not even a journal record.
#[test]
fn adopting_a_vault_writes_nothing_into_it() {
    let root = tmp("adopt");
    let vault = root.join("vault-s1");
    copy(Path::new("../engine/tests/fixtures/vault-s1"), &vault);
    let before = fingerprint(&vault);
    let out = adopt_vault_in(&root.join("appdata"), vault.to_str().unwrap(), None);
    assert_eq!(out["ok"], true, "{out}");
    assert_eq!(fingerprint(&vault), before, "the adopted vault was modified");
    // …and a folder that is not a vault is refused by name, before anything is registered.
    let bad = adopt_vault_in(&root.join("appdata"), root.to_str().unwrap(), None);
    assert_eq!(bad["ok"], false);
    assert!(bad["error"].as_str().unwrap().contains("config/planning.yaml and tasks/"), "{bad}");
    // I1: the predicate is the CONSOLE's, not a weaker cousin of it. A folder with `config/` and
    // `tasks/` but no `config/planning.yaml` looked like a vault here and was refused by
    // `resolve_vault` on the very next launch — a profile registered into a dead end.
    let shell = root.join("looks-like-one");
    std::fs::create_dir_all(shell.join("config")).unwrap();
    std::fs::create_dir_all(shell.join("tasks")).unwrap();
    let bad = adopt_vault_in(&root.join("appdata"), shell.to_str().unwrap(), None);
    assert_eq!(bad["ok"], false, "config/ without planning.yaml is not a vault: {bad}");
    assert!(bad["error"].as_str().unwrap().contains("config/planning.yaml and tasks/"), "{bad}");
    assert!(knowlu::state::resolve_vault(shell.to_str(), Path::new("C:\\nowhere")).is_err(), "…and the console agrees");
    let _ = std::fs::remove_dir_all(&root);
}

/// Spec §8: restore COPIES the mirror out and never writes into the backup folder — the one place
/// a user's only other copy of their vault lives.
#[test]
fn restoring_copies_the_mirror_and_never_writes_into_the_backup() {
    let root = tmp("restore");
    let backup = root.join("backup").join("profile_1111111111").join("vault");
    copy(Path::new("../engine/tests/fixtures/vault-s1"), &backup);
    let before = fingerprint(&root.join("backup"));
    let parent = root.join("restored-into");
    std::fs::create_dir_all(&parent).unwrap();
    let dest = restore_vault_in(root.join("backup").to_str().unwrap(), parent.to_str().unwrap(), "Fall 2026").expect("restore");
    assert_eq!(dest, parent.join("Fall 2026"));
    assert_eq!(fingerprint(&root.join("backup")), before, "the backup folder was written to");
    assert_eq!(fingerprint(&dest), before.iter().map(|(p, b)| (p.trim_start_matches("profile_1111111111\\").trim_start_matches("vault\\").to_string(), b.clone())).collect::<Vec<_>>(), "the mirror arrived whole");
    // A second restore to the same name is refused, and still writes nothing to the backup.
    let again = restore_vault_in(root.join("backup").to_str().unwrap(), parent.to_str().unwrap(), "Fall 2026");
    assert!(again.unwrap_err().contains("already exists"));
    assert_eq!(fingerprint(&root.join("backup")), before);
    let strays: Vec<_> = std::fs::read_dir(&parent).unwrap().flatten()
        .filter(|e| e.file_name().to_string_lossy().starts_with(".knowlu-restore-")).collect();
    assert!(strays.is_empty(), "a refused restore leaves no staging folder");
    let _ = std::fs::remove_dir_all(&root);
}

/// R-P4a-23(c): the names Windows would rewrite or refuse, caught before a credential can be keyed
/// to a path the OS spells differently. `dest_for` is public for exactly this.
#[test]
fn a_name_windows_would_rewrite_is_refused_by_name() {
    let root = tmp("names");
    let p = root.to_str().unwrap();
    for (name, why) in [
        ("Fall*2026", "no * ? \" < > or |"),
        ("Fall?", "no * ? \" < > or |"),
        ("a<b", "no * ? \" < > or |"),
        ("a|b", "no * ? \" < > or |"),
        ("CON", "reserved by Windows"),
        ("nul.md", "reserved by Windows"),
        // `nul.and.more` IS the null device to Windows: the name before the first dot is what the
        // filesystem matches, whatever follows it.
        ("nul.and.more", "reserved by Windows"),
        ("Lpt9", "reserved by Windows"),
        ("Fall 2026.", "cannot end in a dot or a space"),
        ("Fall 2026 ", "cannot end in a dot or a space"),
        ("Fall/2026", "no slashes or colons"),
        ("", "give the vault a name"),
    ] {
        let e = dest_for(p, name).expect_err(&format!("{name:?} must be refused"));
        assert!(e.contains(why), "{name:?}: {e}");
    }
    // …and the ordinary ones still pass, including one that merely CONTAINS a device name.
    for name in ["Fall 2026", "console", "annul", "v1.0"] {
        assert!(dest_for(p, name).is_ok(), "{name:?} must be allowed");
    }
    let _ = std::fs::remove_dir_all(&root);
}

/// Review round 1, minor: a vault that could not be finished is removed, not left half-made for
/// `dest_for` to refuse next time — and nothing is registered, because the settings file (the only
/// fallible half) is written before the registry entry (IMPORTANT 4).
///
/// **R-P4a-25 is the backup rule this drives** (final fix wave A1): the backup folder may not BE the
/// vault and may not sit INSIDE it — a mirror inside the thing it mirrors copies itself — and a
/// vault inside the backup folder is allowed, because the mirror writes under
/// `<backup>\<profile>\vault`. The allowed case is not academic: the wizard's default parent is
/// `%USERPROFILE%\Documents\Knowlu` and its default vault is `Documents\Knowlu\<name>`, so refusing
/// it made the most obvious folder on the machine unpickable.
#[test]
fn a_vault_that_cannot_be_finished_is_removed_and_nothing_is_registered() {
    let root = tmp("rollback");
    let parent = root.join("vaults");
    std::fs::create_dir_all(&parent).unwrap();
    let app_data = root.join("appdata");
    let plan_with = |bdir: String| WizardPlan {
        ics_url: None,
        timezone: "America/Chicago".to_string(),
        slots: vec!["12:00".to_string(), "18:00".to_string()],
        campus: "none".to_string(),
        zybooks: false,
        vhl: false,
        backup_dir: Some(bdir),
        autostart: true,
        offer_inference: false,
    };
    let vault = parent.join("Fall 2026");
    // (a) The backup folder INSIDE the vault: the one case the rule refuses, checked after the
    // vault has already been scaffolded — which is what makes this the rollback path too.
    let inside = plan_with(vault.join("backups").to_string_lossy().to_string());
    let out = create_vault_in(&app_data, parent.to_str().unwrap(), "Fall 2026", &inside);
    assert_eq!(out["ok"], false, "{out}");
    let err = out["error"].as_str().unwrap();
    assert!(err.contains("cannot be the vault, or inside it"), "{err}");
    assert!(err.contains("was removed"), "the refusal says the vault went with it: {err}");
    assert!(!vault.exists(), "the half-made vault is gone");
    assert!(!knowlu::profiles::registry_path(&app_data).exists(), "and nothing was registered");
    // (b) The backup folder that IS the vault.
    let same = plan_with(vault.to_string_lossy().to_string());
    let out = create_vault_in(&app_data, parent.to_str().unwrap(), "Fall 2026", &same);
    assert_eq!(out["ok"], false, "{out}");
    assert!(out["error"].as_str().unwrap().contains("cannot be the vault, or inside it"), "{out}");
    assert!(!vault.exists());
    // (c) R-P4a-25: the vault INSIDE the backup folder — here its own parent — goes through.
    let above = plan_with(parent.to_string_lossy().to_string());
    let out = create_vault_in(&app_data, parent.to_str().unwrap(), "Fall 2026", &above);
    assert_eq!(out["ok"], true, "a vault inside the backup folder must be allowed: {out}");
    assert!(vault.join("config").join("runners.yaml").is_file());
    // …and so does a backup folder with nothing to do with the vault at all.
    let elsewhere = plan_with(root.join("mirror").to_string_lossy().to_string());
    let out = create_vault_in(&app_data, parent.to_str().unwrap(), "Spring 2027", &elsewhere);
    assert_eq!(out["ok"], true, "{out}");
    assert!(parent.join("Spring 2027").join("config").join("runners.yaml").is_file());
    let _ = std::fs::remove_dir_all(&root);
}

/// Deletes the credential on drop, including on a panicking assertion — Task 3's pattern, for the
/// same reason: a failing test must never leave a login behind in Credential Manager.
#[cfg(windows)]
struct Cleanup(String);
#[cfg(windows)]
impl Drop for Cleanup {
    fn drop(&mut self) {
        let _ = knowlu::credentials::delete(&self.0);
    }
}

/// R-P4a-23(a)(b): the user renamed the vault after panel 5, so the entry moves. The secret is
/// read and rewritten inside the command and never crosses back to the page — this test is the
/// only thing that ever sees both ends of it, and it mints its own.
///
/// The targets are DERIVED from two throwaway paths rather than spelled `knowlu/test/…`: the
/// command computes them with `target_for(id_for(path), source)`, so a literal target would test
/// a target the command never uses. Both are deleted on the way out, whatever happens.
#[cfg(windows)]
#[test]
fn a_rename_after_panel_five_moves_the_login_and_leaves_nothing_behind() {
    use knowlu::credentials::{exists, target_for, write};
    use knowlu::onboarding::retarget_credentials;
    let from = std::env::temp_dir().join(format!("knowlu-retarget-from-{}", std::process::id()));
    let to = std::env::temp_dir().join(format!("knowlu-retarget-to-{}", std::process::id()));
    let old = target_for(&knowlu::profiles::id_for(&from), "zybooks");
    let new = target_for(&knowlu::profiles::id_for(&to), "zybooks");
    let (_c1, _c2) = (Cleanup(old.clone()), Cleanup(new.clone()));

    let user = "knowlu-test@example.invalid";
    let secret = knowlu_engine::ids::new_id("s");
    write(&old, user, &secret).expect("mint the entry panel 5 would have written");

    let out = retarget_credentials(from.to_string_lossy().to_string(), to.to_string_lossy().to_string());
    assert_eq!(out["ok"], true, "{out}");
    assert_eq!(out["moved"], 1);
    assert!(!exists(&old), "the entry keyed to the old path is gone");
    let got = knowlu_engine::wincred::read_credential(&new).expect("the engine reads the moved one");
    assert_eq!(got.username, user);
    assert_eq!(got.password.expose(), secret, "byte for byte, through the move");

    // Idempotent: nothing left to move is a success with nothing moved, not an error.
    let again = retarget_credentials(from.to_string_lossy().to_string(), to.to_string_lossy().to_string());
    assert_eq!(again["ok"], true, "{again}");
    assert_eq!(again["moved"], 0);
    // …and a move to the same path never touches Credential Manager at all.
    let same = retarget_credentials(to.to_string_lossy().to_string(), to.to_string_lossy().to_string());
    assert_eq!(same["moved"], 0);
    assert!(exists(&new), "the entry is still there after a no-op move");
}

/// Knowlu plan 3a Task 10 review, M1: a minimal plan, varying only the checkbox under test. The
/// finish panel's checkbox is `WizardPlan.offer_inference`, and the fix round this covers found
/// that the brief's own code wired it into only ONE of the three paths that share this panel.
fn base_plan(offer_inference: bool) -> WizardPlan {
    WizardPlan {
        ics_url: None,
        timezone: "America/Chicago".to_string(),
        slots: vec!["12:00".to_string(), "18:00".to_string()],
        campus: "none".to_string(),
        zybooks: false,
        vhl: false,
        backup_dir: None,
        autostart: true,
        offer_inference,
    }
}

/// The marker path for one profile — the same join `commands::settings_context` reads and clears.
fn marker_for(app_data: &Path, id: &str) -> PathBuf {
    offer_marker(&knowlu::profiles::profile_dir(app_data, id))
}

/// Task 10 review, M1: the CREATE path (`create_vault_in` → `finish_profile_in`) drops the marker
/// when the finish panel's checkbox was checked, and only then. This is the path the brief's own
/// code actually wired the marker into, so this pair is the regression guard for it.
#[test]
fn create_vault_drops_the_offer_marker_when_the_checkbox_was_checked() {
    let root = tmp("create-offer-on");
    let parent = root.join("vaults");
    std::fs::create_dir_all(&parent).unwrap();
    let app_data = root.join("appdata");
    let out = create_vault_in(&app_data, parent.to_str().unwrap(), "Fall 2026", &base_plan(true));
    assert_eq!(out["ok"], true, "{out}");
    let id = out["profile"]["id"].as_str().expect("a profile id").to_string();
    assert!(marker_for(&app_data, &id).is_file(), "the checkbox was checked");
    let _ = std::fs::remove_dir_all(&root);
}

#[test]
fn create_vault_leaves_no_offer_marker_when_the_checkbox_was_not_checked() {
    let root = tmp("create-offer-off");
    let parent = root.join("vaults");
    std::fs::create_dir_all(&parent).unwrap();
    let app_data = root.join("appdata");
    let out = create_vault_in(&app_data, parent.to_str().unwrap(), "Fall 2026", &base_plan(false));
    assert_eq!(out["ok"], true, "{out}");
    let id = out["profile"]["id"].as_str().expect("a profile id").to_string();
    assert!(!marker_for(&app_data, &id).exists(), "the checkbox was left unchecked");
    let _ = std::fs::remove_dir_all(&root);
}

/// Task 10 review, M1: the RESTORE path. `restore_vault_in` alone only copies the backup mirror
/// out — it takes no `WizardPlan` — so the marker is `finish_or_roll_back`'s doing, exactly as the
/// live `restore_vault` command calls it. This is the pair the brief's own code silently missed:
/// checking the box while restoring a vault would have done nothing.
#[test]
fn restore_vault_drops_the_offer_marker_when_the_checkbox_was_checked() {
    let root = tmp("restore-offer-on");
    let backup = root.join("backup").join("profile_2222222222").join("vault");
    copy(Path::new("../engine/tests/fixtures/vault-s1"), &backup);
    let parent = root.join("restored-into");
    std::fs::create_dir_all(&parent).unwrap();
    let app_data = root.join("appdata");
    let dest = restore_vault_in(root.join("backup").to_str().unwrap(), parent.to_str().unwrap(), "Fall 2026").expect("restore");
    let out = finish_or_roll_back(&app_data, &dest, Some("Fall 2026".to_string()), &base_plan(true));
    assert_eq!(out["ok"], true, "{out}");
    let id = out["profile"]["id"].as_str().expect("a profile id").to_string();
    assert!(marker_for(&app_data, &id).is_file(), "the checkbox was checked");
    let _ = std::fs::remove_dir_all(&root);
}

#[test]
fn restore_vault_leaves_no_offer_marker_when_the_checkbox_was_not_checked() {
    let root = tmp("restore-offer-off");
    let backup = root.join("backup").join("profile_3333333333").join("vault");
    copy(Path::new("../engine/tests/fixtures/vault-s1"), &backup);
    let parent = root.join("restored-into");
    std::fs::create_dir_all(&parent).unwrap();
    let app_data = root.join("appdata");
    let dest = restore_vault_in(root.join("backup").to_str().unwrap(), parent.to_str().unwrap(), "Fall 2026").expect("restore");
    let out = finish_or_roll_back(&app_data, &dest, Some("Fall 2026".to_string()), &base_plan(false));
    assert_eq!(out["ok"], true, "{out}");
    let id = out["profile"]["id"].as_str().expect("a profile id").to_string();
    assert!(!marker_for(&app_data, &id).exists(), "the checkbox was left unchecked");
    let _ = std::fs::remove_dir_all(&root);
}

/// Task 10 review, M1: the ADOPT path. `adopt_vault_in` alone registers the vault and carries no
/// `WizardPlan` either — the marker is `apply_profile_settings_in`'s doing, the wizard's own
/// second half for this path (panel 3's backup folder and panel 6's autostart choice go through
/// the same call).
#[test]
fn adopt_vault_drops_the_offer_marker_when_the_checkbox_was_checked() {
    let root = tmp("adopt-offer-on");
    let vault = root.join("vault-s1");
    copy(Path::new("../engine/tests/fixtures/vault-s1"), &vault);
    let app_data = root.join("appdata");
    let out = adopt_vault_in(&app_data, vault.to_str().unwrap(), None);
    assert_eq!(out["ok"], true, "{out}");
    let id = out["profile"]["id"].as_str().expect("a profile id").to_string();
    let out2 = apply_profile_settings_in(&app_data, &id, &base_plan(true));
    assert_eq!(out2["ok"], true, "{out2}");
    assert!(marker_for(&app_data, &id).is_file(), "the checkbox was checked");
    let _ = std::fs::remove_dir_all(&root);
}

#[test]
fn adopt_vault_leaves_no_offer_marker_when_the_checkbox_was_not_checked() {
    let root = tmp("adopt-offer-off");
    let vault = root.join("vault-s1");
    copy(Path::new("../engine/tests/fixtures/vault-s1"), &vault);
    let app_data = root.join("appdata");
    let out = adopt_vault_in(&app_data, vault.to_str().unwrap(), None);
    assert_eq!(out["ok"], true, "{out}");
    let id = out["profile"]["id"].as_str().expect("a profile id").to_string();
    let out2 = apply_profile_settings_in(&app_data, &id, &base_plan(false));
    assert_eq!(out2["ok"], true, "{out2}");
    assert!(!marker_for(&app_data, &id).exists(), "the checkbox was left unchecked");
    let _ = std::fs::remove_dir_all(&root);
}
