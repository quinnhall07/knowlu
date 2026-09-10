//! The two "never wrote there" tests (plan 4a, Task 6; spec §8, S7). Both drive the handle-free
//! cores in `app/src/onboarding.rs`, so they need no Tauri app and no window: adopting and
//! restoring are the two paths where the app touches a folder the USER already owns, and the
//! promise on both is the same — it is read, never written.
use knowlu::onboarding::{
    adopt_vault_in, apply_profile_settings_in, create_vault_in, dest_for, finish_or_roll_back,
    default_folders_in, offer_marker, restore_vault_in, WizardPlan,
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
///
/// Fix round 1, item 1: restore decides the folder the way create does — `<home>\Knowlu\<name>`,
/// with the mirror at `<home>\Knowlu\Backups` — rather than a page-supplied `parent`. This is also
/// why a restore on a fresh machine (no `%USERPROFILE%\Knowlu` yet) now works: `vault_dest_in`
/// creates the parent, where the old `dest_for(parent, …)` would have refused a missing one.
#[test]
fn restoring_copies_the_mirror_and_never_writes_into_the_backup() {
    let root = tmp("restore");
    let backup = root.join("backup").join("profile_1111111111").join("vault");
    copy(Path::new("../engine/tests/fixtures/vault-s1"), &backup);
    let before = fingerprint(&root.join("backup"));
    let home = root.join("home");
    assert!(!home.join("Knowlu").exists(), "a fresh machine — the parent does not exist yet");
    let dest = restore_vault_in(root.join("backup").to_str().unwrap(), &home, "Fall 2026").expect("restore");
    assert_eq!(dest, home.join("Knowlu").join("Fall 2026"), "restore decides the folder the way create does");
    let (_, backups) = default_folders_in(&home);
    assert_eq!(PathBuf::from(backups), home.join("Knowlu").join("Backups"), "the mirror is the same default sibling a created vault gets");
    assert_eq!(fingerprint(&root.join("backup")), before, "the backup folder was written to");
    assert_eq!(fingerprint(&dest), before.iter().map(|(p, b)| (p.trim_start_matches("profile_1111111111\\").trim_start_matches("vault\\").to_string(), b.clone())).collect::<Vec<_>>(), "the mirror arrived whole");
    // A second restore to the same name is refused, and still writes nothing to the backup.
    let again = restore_vault_in(root.join("backup").to_str().unwrap(), &home, "Fall 2026");
    assert!(again.unwrap_err().contains("already exists"));
    assert_eq!(fingerprint(&root.join("backup")), before);
    let strays: Vec<_> = std::fs::read_dir(home.join("Knowlu")).unwrap().flatten()
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
/// `%USERPROFILE%\Knowlu` and its default vault is `Knowlu\<name>` (since 2026-09-09; it was the
/// OneDrive-redirected `Documents\Knowlu` before), so refusing it made the most obvious folder on
/// the machine unpickable.
#[test]
fn a_vault_that_cannot_be_finished_is_removed_and_nothing_is_registered() {
    let root = tmp("rollback");
    let parent = root.join("vaults");
    std::fs::create_dir_all(&parent).unwrap();
    let app_data = root.join("appdata");
    let plan = WizardPlan {
        ics_url: None,
        personal_calendar: None,
        timezone: "America/Chicago".to_string(),
        slots: vec!["12:00".to_string(), "18:00".to_string()],
        campus_choice: Default::default(),
        zybooks: false,
        vhl: false,
        autostart: true,
        offer_inference: false,
        zybooks_courses: Vec::new(),
        vhl_sections: Vec::new(),
        course_map: Vec::new(),
        courses: Vec::new(),
        zybooks_ignore: Vec::new(),
    };
    // A vault, scaffolded directly through `scaffold::create_vault` — this test is about
    // `finish_or_roll_back`'s own backup-folder check now that the folder is a parameter of
    // `finish_or_roll_back` rather than a field of `WizardPlan` (spec §4.1 removed the panel), so it
    // never needs a signed-in session the way `create_vault_in` does.
    let scaffold_at = |dest: &std::path::Path| knowlu::scaffold::VaultPlan {
        profile_id: knowlu::profiles::id_for(dest),
        ics_url: None,
        personal_calendar: None,
        timezone: "America/Chicago".into(),
        slots: vec!["12:00".into()],
        device: "MACHINE".into(),
        campus: "none".into(),
        campus_choice: Default::default(),
        zybooks: false,
        vhl: false,
        zybooks_courses: Vec::new(),
        vhl_sections: Vec::new(),
        zybooks_ignore: Vec::new(),
        course_map: Vec::new(),
        courses: Vec::new(),
        api_base: "https://example.supabase.co/functions/v1".into(),
        anon_key: "anon".into(),
        account_id: "acc-1".into(),
    };
    let vault = parent.join("Fall 2026");
    // (a) The backup folder INSIDE the vault: the one case the rule refuses, checked after the
    // vault has already been scaffolded — which is what makes this the rollback path too.
    knowlu::scaffold::create_vault(&vault, &scaffold_at(&vault)).unwrap();
    let out = finish_or_roll_back(&app_data, &vault, Some("Fall 2026".to_string()), &plan, Some(vault.join("backups")));
    assert_eq!(out["ok"], false, "{out}");
    let err = out["error"].as_str().unwrap();
    assert!(err.contains("cannot be the vault, or inside it"), "{err}");
    assert!(err.contains("was removed"), "the refusal says the vault went with it: {err}");
    assert!(!vault.exists(), "the half-made vault is gone");
    assert!(!knowlu::profiles::registry_path(&app_data).exists(), "and nothing was registered");
    // (b) The backup folder that IS the vault.
    knowlu::scaffold::create_vault(&vault, &scaffold_at(&vault)).unwrap();
    let out = finish_or_roll_back(&app_data, &vault, Some("Fall 2026".to_string()), &plan, Some(vault.clone()));
    assert_eq!(out["ok"], false, "{out}");
    assert!(out["error"].as_str().unwrap().contains("cannot be the vault, or inside it"), "{out}");
    assert!(!vault.exists());
    // (c) R-P4a-25: the vault INSIDE the backup folder — here its own parent — goes through.
    knowlu::scaffold::create_vault(&vault, &scaffold_at(&vault)).unwrap();
    let out = finish_or_roll_back(&app_data, &vault, Some("Fall 2026".to_string()), &plan, Some(parent.clone()));
    assert_eq!(out["ok"], true, "a vault inside the backup folder must be allowed: {out}");
    assert!(vault.join("config").join("runners.yaml").is_file());
    // …and so does a backup folder with nothing to do with the vault at all.
    let vault2 = parent.join("Spring 2027");
    knowlu::scaffold::create_vault(&vault2, &scaffold_at(&vault2)).unwrap();
    let out = finish_or_roll_back(&app_data, &vault2, Some("Spring 2027".to_string()), &plan, Some(root.join("mirror")));
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

/// Every test below that drives `create_vault_in`/`finish_or_roll_back`'s session move needs a real
/// pending session in Credential Manager under `account::PENDING_TARGET` — a fixed, shared name, so
/// this holds `CREDMAN_LOCK` for its whole life (the same reason `app/tests/account.rs` does) and
/// deletes every credential it touched on drop, including on a panicking assertion: the pending
/// entry itself, and the profile's own copy `move_session` leaves behind on success.
#[cfg(windows)]
static CREDMAN_LOCK: std::sync::Mutex<()> = std::sync::Mutex::new(());

#[cfg(windows)]
struct PendingSession {
    _guard: std::sync::MutexGuard<'static, ()>,
    /// Whatever was already at `PENDING_TARGET` before this guard wrote over it — `(username,
    /// secret)`, read raw rather than through `load_session` so a session this process cannot even
    /// parse is still put back byte for byte. `None` means there was nothing there.
    had_previous: Option<(String, String)>,
    moved_to: Vec<String>,
}
#[cfg(windows)]
impl PendingSession {
    /// Writes a throwaway session under `PENDING_TARGET` for `account_id` — never a real one.
    ///
    /// Fix round 1, item 3: `PENDING_TARGET` is one fixed, shared name — a real wizard signed in on
    /// this same machine, mid-flow, would have its session sitting right there. This reads whatever
    /// is already at the target FIRST, so `drop` can put it back rather than delete a live sign-in
    /// this test process never owned.
    fn new(account_id: &str) -> Self {
        let guard = CREDMAN_LOCK.lock().unwrap_or_else(|e| e.into_inner());
        let had_previous = knowlu_engine::wincred::read_credential(knowlu::account::PENDING_TARGET)
            .ok()
            .map(|c| (c.username, c.password.expose().to_string()));
        let s = knowlu::account::Session {
            access_token: "test-at".into(),
            refresh_token: "test-rt".into(),
            expires_at: jiff::Timestamp::now().as_second() + 3600,
            email: "knowlu-test@example.invalid".into(),
        };
        knowlu::account::save_session(knowlu::account::PENDING_TARGET, account_id, &s)
            .expect("write the pending session this wizard test signs in with");
        Self { _guard: guard, had_previous, moved_to: Vec::new() }
    }
    /// The profile-keyed target the session moves onto once a vault named `profile_id` exists,
    /// tracked for cleanup regardless of whether the move actually happened.
    fn expect_move_to(&mut self, profile_id: &str) {
        self.moved_to.push(knowlu::account::session_target(profile_id));
    }
}
#[cfg(windows)]
impl Drop for PendingSession {
    fn drop(&mut self) {
        match &self.had_previous {
            // Restore it exactly, rather than delete — this run is not the only thing that might
            // have been signed in at `PENDING_TARGET`.
            Some((user, secret)) => { let _ = knowlu::credentials::write(knowlu::account::PENDING_TARGET, user, secret); }
            None => { let _ = knowlu::credentials::delete(knowlu::account::PENDING_TARGET); }
        }
        for t in &self.moved_to {
            let _ = knowlu::credentials::delete(t);
        }
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
    // Fix round 1: this file's other tests now put real, heavy Credential Manager traffic through
    // `PendingSession`/`CREDMAN_LOCK` — the same threshold `app/tests/account.rs` documents crossing
    // (a read for one target spuriously reporting `ERROR_NOT_FOUND` under concurrent CredMan I/O,
    // even for a target no other thread touched). This test predates that traffic and never took the
    // lock; now it does, so every real-credential test in this file is serialized against every
    // other, not only the ones sharing a target.
    let _credman_guard = CREDMAN_LOCK.lock().unwrap_or_else(|e| e.into_inner());
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
        personal_calendar: None,
        timezone: "America/Chicago".to_string(),
        slots: vec!["12:00".to_string(), "18:00".to_string()],
        campus_choice: Default::default(),
        zybooks: false,
        vhl: false,
        zybooks_courses: Vec::new(),
        vhl_sections: Vec::new(),
        course_map: Vec::new(),
        courses: Vec::new(),
        zybooks_ignore: Vec::new(),
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
///
/// `create_vault_in` reads the account it signed in with off `account::PENDING_TARGET` (C1 Task
/// 12), so this needs a real pending session — `PendingSession` mints one and cleans it (and the
/// moved copy) up on drop.
#[cfg(windows)]
#[test]
fn create_vault_drops_the_offer_marker_when_the_checkbox_was_checked() {
    let root = tmp("create-offer-on");
    let home = root.join("home");
    let app_data = root.join("appdata");
    let mut session = PendingSession::new("acc-marker-create-on");
    let out = create_vault_in(&app_data, &home, "Fall 2026", &base_plan(true));
    assert_eq!(out["ok"], true, "{out}");
    let id = out["profile"]["id"].as_str().expect("a profile id").to_string();
    session.expect_move_to(&id);
    assert!(marker_for(&app_data, &id).is_file(), "the checkbox was checked");
    let _ = std::fs::remove_dir_all(&root);
}

#[cfg(windows)]
#[test]
fn create_vault_leaves_no_offer_marker_when_the_checkbox_was_not_checked() {
    let root = tmp("create-offer-off");
    let home = root.join("home");
    let app_data = root.join("appdata");
    let mut session = PendingSession::new("acc-marker-create-off");
    let out = create_vault_in(&app_data, &home, "Fall 2026", &base_plan(false));
    assert_eq!(out["ok"], true, "{out}");
    let id = out["profile"]["id"].as_str().expect("a profile id").to_string();
    session.expect_move_to(&id);
    assert!(!marker_for(&app_data, &id).exists(), "the checkbox was left unchecked");
    let _ = std::fs::remove_dir_all(&root);
}

/// Fix round 1, item 4: with nothing at the pending target, `create_vault_in` refuses — naming the
/// sentence a friend actually sees — and creates no vault directory at all. Not a `PendingSession`
/// test (there is deliberately no session here), but it still touches the real, shared pending
/// target, so it takes `CREDMAN_LOCK` itself and puts back whatever it found there, the same way
/// `PendingSession` does.
#[cfg(windows)]
#[test]
fn create_vault_without_a_pending_session_refuses_and_creates_nothing() {
    let _guard = CREDMAN_LOCK.lock().unwrap_or_else(|e| e.into_inner());
    let had_previous = knowlu_engine::wincred::read_credential(knowlu::account::PENDING_TARGET)
        .ok()
        .map(|c| (c.username, c.password.expose().to_string()));
    let _ = knowlu::credentials::delete(knowlu::account::PENDING_TARGET);

    let root = tmp("no-pending-session");
    let home = root.join("home");
    let app_data = root.join("appdata");
    let out = create_vault_in(&app_data, &home, "Fall 2026", &base_plan(false));
    assert_eq!(out["ok"], false, "{out}");
    assert!(out["error"].as_str().unwrap().contains("sign in again"), "{out}");
    assert!(!home.join("Knowlu").join("Fall 2026").exists(), "no vault directory was created");

    match had_previous {
        Some((user, secret)) => { knowlu::credentials::write(knowlu::account::PENDING_TARGET, &user, &secret).unwrap(); }
        None => {}
    }
    let _ = std::fs::remove_dir_all(&root);
}

/// Fix round 1, item 2: `personal_calendar` is validated on the device before it ever reaches a
/// `VaultPlan` — trimmed, `webcal://` rewritten to `https://` (R-C1-22), anything else that is not
/// `https://` refused. Four cases, each its own test so a failure names exactly one behaviour.
#[cfg(windows)]
#[test]
fn a_blank_personal_calendar_writes_no_calendars_entry() {
    let root = tmp("cal-blank");
    let home = root.join("home");
    let app_data = root.join("appdata");
    let mut session = PendingSession::new("acc-cal-blank");
    let mut plan = base_plan(false);
    plan.personal_calendar = Some("   ".to_string());
    let out = create_vault_in(&app_data, &home, "Fall 2026", &plan);
    assert_eq!(out["ok"], true, "{out}");
    let id = out["profile"]["id"].as_str().expect("a profile id").to_string();
    session.expect_move_to(&id);
    let ingest = knowlu_engine::pystr::read_text(&home.join("Knowlu").join("Fall 2026").join("config").join("ingest.yaml")).unwrap();
    assert!(ingest.contains("calendars: []\n"), "{ingest}");
    let _ = std::fs::remove_dir_all(&root);
}

#[cfg(windows)]
#[test]
fn a_webcal_personal_calendar_is_rewritten_to_https() {
    let root = tmp("cal-webcal");
    let home = root.join("home");
    let app_data = root.join("appdata");
    let mut session = PendingSession::new("acc-cal-webcal");
    let mut plan = base_plan(false);
    plan.personal_calendar = Some("webcal://x.invalid/y.ics".to_string());
    let out = create_vault_in(&app_data, &home, "Fall 2026", &plan);
    assert_eq!(out["ok"], true, "{out}");
    let id = out["profile"]["id"].as_str().expect("a profile id").to_string();
    session.expect_move_to(&id);
    let ingest = knowlu_engine::pystr::read_text(&home.join("Knowlu").join("Fall 2026").join("config").join("ingest.yaml")).unwrap();
    assert!(
        ingest.contains("calendars:\n  - name: personal\n    ics_url: 'https://x.invalid/y.ics'\n"),
        "{ingest}"
    );
    let _ = std::fs::remove_dir_all(&root);
}

#[cfg(windows)]
#[test]
fn a_non_https_personal_calendar_is_refused_and_creates_no_vault() {
    let root = tmp("cal-http");
    let home = root.join("home");
    let app_data = root.join("appdata");
    let _session = PendingSession::new("acc-cal-http");
    let mut plan = base_plan(false);
    plan.personal_calendar = Some("http://x.invalid/y.ics".to_string());
    let out = create_vault_in(&app_data, &home, "Fall 2026", &plan);
    assert_eq!(out["ok"], false, "{out}");
    assert!(out["error"].as_str().unwrap().contains("must start with https://"), "{out}");
    assert!(!home.join("Knowlu").join("Fall 2026").exists(), "no vault was created");
    let _ = std::fs::remove_dir_all(&root);
}

#[cfg(windows)]
#[test]
fn a_padded_personal_calendar_is_trimmed() {
    let root = tmp("cal-padded");
    let home = root.join("home");
    let app_data = root.join("appdata");
    let mut session = PendingSession::new("acc-cal-padded");
    let mut plan = base_plan(false);
    plan.personal_calendar = Some("  https://x.invalid/y.ics  ".to_string());
    let out = create_vault_in(&app_data, &home, "Fall 2026", &plan);
    assert_eq!(out["ok"], true, "{out}");
    let id = out["profile"]["id"].as_str().expect("a profile id").to_string();
    session.expect_move_to(&id);
    let ingest = knowlu_engine::pystr::read_text(&home.join("Knowlu").join("Fall 2026").join("config").join("ingest.yaml")).unwrap();
    assert!(
        ingest.contains("calendars:\n  - name: personal\n    ics_url: 'https://x.invalid/y.ics'\n"),
        "{ingest}"
    );
    let _ = std::fs::remove_dir_all(&root);
}

/// Review round 1, I4: the task's headline sentence, actually executed — discovered rows →
/// `WizardPlan` → `create_vault_in` → the `config/ingest.yaml` the engine reads, read back through
/// the engine's OWN loader (I1) rather than only by substring, so this closes the YAML trace in a
/// test instead of in a review. Folds in the other three Important findings this same code path
/// carries:
/// - **I1**: two rows mapping the SAME zyBook code collapse to one key, and the written file still
///   parses — through `create_vault_in`'s de-duplication, not `build_into`'s defensive parse-back
///   (that is `a_duplicate_zybook_key_is_refused_rather_than_written_unparsable`, `scaffold.rs`).
/// - **I2**: a zyBook the student declined to map lands in `ignore:` alongside the permanent
///   `HowToUseZyBooks2`.
/// - **I3**: the real `GN 103` / `GN 103 Hausaufgaben` pair — the slug comes from the page-sent
///   `course` ("GN 103" → `gn-103`), never from `label` (which would have produced
///   `gn-103-hausaufgaben` and silently missed `courses/gn-103.md`, Task 14b's seed).
#[cfg(windows)]
#[test]
fn discovered_rows_route_through_create_vault_in_to_the_config_the_engine_reads() {
    use knowlu::scaffold::{BookMapping, SectionMapping};
    let root = tmp("coursework-mapping");
    let home = root.join("home");
    let app_data = root.join("appdata");
    let mut session = PendingSession::new("acc-coursework-mapping");
    let mut plan = base_plan(false);
    plan.zybooks = true;
    plan.vhl = true;
    plan.zybooks_courses = vec![
        BookMapping { code: "UACS100Fall2026".into(), course: "CS 100".into(), label: "CS 100".into() },
        // I1: a second row confirming the SAME book — a two-part book, a lecture+lab pair — must
        // collapse to one key, first wins, not two (which `ingest_yaml` would write as a duplicate
        // YAML key and `serde_yaml_ng` would then refuse to parse at all).
        BookMapping { code: "UACS100Fall2026".into(), course: "CS 100 (again)".into(), label: "CS 100, again".into() },
    ];
    // I3: `course` and `label` genuinely differ — Quinn's own real pair. The slug MUST come from
    // `course` ("GN 103" → `gn-103`), never from `label` ("GN 103 Hausaufgaben" → would-be
    // `gn-103-hausaufgaben`).
    plan.vhl_sections = vec![SectionMapping { section: "2102121".into(), course: "GN 103".into(), label: "GN 103 Hausaufgaben".into() }];
    plan.course_map = vec![("CS 100".into(), String::new())];
    // I2: a book the student left unmapped or ticked *ignore*.
    plan.zybooks_ignore = vec!["SomeOtherBook".into()];
    let out = create_vault_in(&app_data, &home, "Fall 2026", &plan);
    assert_eq!(out["ok"], true, "{out}");
    let id = out["profile"]["id"].as_str().expect("a profile id").to_string();
    session.expect_move_to(&id);
    let vault = home.join("Knowlu").join("Fall 2026");
    let text = knowlu_engine::pystr::read_text(&vault.join("config").join("ingest.yaml")).unwrap();

    // The engine's OWN loader reads it back without complaint (I1) — the same door
    // `coursework-discover`/`coursework`/`judge` walk through on the first real slot.
    let (cfg, warnings) = knowlu_engine::coursework::load_coursework_config(&vault).expect("parses");
    assert!(warnings.is_empty(), "{warnings:?}: {text}");
    assert!(cfg.get(serde_yaml_ng::Value::String("coursework".into())).is_some(), "the block is really there: {text}");

    // I1: one key, first wins — the second row's "CS 100 (again)"/"CS 100, again" never appears.
    assert_eq!(text.matches("'UACS100Fall2026':").count(), 1, "{text}");
    assert!(text.contains("      'UACS100Fall2026':\n        course: 'cs-100'\n        label: 'CS 100'\n"), "{text}");
    assert!(!text.contains("again"), "the first mapping won, the second was dropped: {text}");

    // I3: the section's slug is `gn-103`, derived from `course` — never `gn-103-hausaufgaben`.
    assert!(text.contains("      '2102121':\n        course: 'gn-103'\n        label: 'GN 103 Hausaufgaben'\n"), "{text}");
    assert!(!text.contains("gn-103-hausaufgaben"), "{text}");

    // I2: the onboarding book is always ignored, and so is a declined discovery.
    assert!(text.contains("      - 'HowToUseZyBooks2'\n"), "{text}");
    assert!(text.contains("      - 'SomeOtherBook'\n"), "{text}");

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
    let home = root.join("home");
    let app_data = root.join("appdata");
    let dest = restore_vault_in(root.join("backup").to_str().unwrap(), &home, "Fall 2026").expect("restore");
    let out = finish_or_roll_back(&app_data, &dest, Some("Fall 2026".to_string()), &base_plan(true), None);
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
    let home = root.join("home");
    let app_data = root.join("appdata");
    let dest = restore_vault_in(root.join("backup").to_str().unwrap(), &home, "Fall 2026").expect("restore");
    let out = finish_or_roll_back(&app_data, &dest, Some("Fall 2026".to_string()), &base_plan(false), None);
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
    let out2 = apply_profile_settings_in(&app_data, &id, &base_plan(true), None);
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
    let out2 = apply_profile_settings_in(&app_data, &id, &base_plan(false), None);
    assert_eq!(out2["ok"], true, "{out2}");
    assert!(!marker_for(&app_data, &id).exists(), "the checkbox was left unchecked");
    let _ = std::fs::remove_dir_all(&root);
}

/// Cloud design §4.1 and §11a (2026-09-09): the wizard's defaults are `<home>\Knowlu` for the
/// vault's parent and its `Backups` sibling for the mirror — never `Documents`, which OneDrive
/// redirects (that is how a live token reached Microsoft's cloud on 2026-09-09). Siblings, so the
/// backup rule (R-P4a-25: not the vault, not inside it) holds for the defaults by construction.
#[test]
fn the_default_folders_are_knowlu_under_home_and_its_backups_sibling() {
    let (parent, backup) = default_folders_in(Path::new(r"C:\Users\someone"));
    assert_eq!(parent, r"C:\Users\someone\Knowlu");
    assert_eq!(backup, r"C:\Users\someone\Knowlu\Backups");
    assert!(!parent.contains("Documents") && !backup.contains("Documents"), "never the redirected folder");
}

#[test]
fn the_vault_goes_under_knowlu_and_nobody_picks_a_folder() {
    use knowlu::onboarding::{default_folders_in, vault_dest_in};
    let home = tmp("home");
    // The parent does not exist yet — the app makes it, which is the whole of §4.1.
    let (parent, backups) = default_folders_in(&home);
    assert!(!std::path::Path::new(&parent).exists());
    let (vault, backup) = vault_dest_in(&home, "Fall 2026").expect("dest");
    assert_eq!(vault, home.join("Knowlu").join("Fall 2026"));
    assert_eq!(backup, std::path::PathBuf::from(&backups));
    assert!(std::path::Path::new(&parent).is_dir(), "the parent was created");
    assert!(std::path::Path::new(&backups).is_dir(), "the backups root was created");
    assert!(!vault.exists(), "the vault itself is created by create_vault, not by this");
    // The same refusals `dest_for` makes, because this is `dest_for` with the parent decided.
    assert!(vault_dest_in(&home, "CON").is_err());
    assert!(vault_dest_in(&home, "a/b").is_err());
    assert!(vault_dest_in(&home, "  ").is_err());
    // …and a name that is already there is refused, not silently reused.
    std::fs::create_dir_all(home.join("Knowlu").join("Taken")).unwrap();
    assert!(vault_dest_in(&home, "Taken").is_err());
    let _ = std::fs::remove_dir_all(&home);
}

/// The discovery reply the panel renders, parsed and suggested — driven directly, because spawning
/// the engine needs a credential this test must not have.
#[test]
fn discovery_output_becomes_rows_with_a_suggestion_each() {
    use knowlu::onboarding::rows_from_discovery;
    let json = r#"{"errors": [], "vhl": [{"course_id": "1623220", "mapped": false, "section": "2102121"}], "zybooks": [{"code": "UACS100Fall2026", "ignored": false, "mapped": false}, {"code": "HowToUseZyBooks2", "ignored": true, "mapped": false}]}"#;
    let rows = rows_from_discovery(json);
    assert_eq!(rows.len(), 3, "{rows:?}");
    let zy = &rows[0];
    assert_eq!(zy.source, "zybooks");
    assert_eq!(zy.key, "UACS100Fall2026");
    assert_eq!(zy.suggested.as_deref(), Some("CS 100"));
    assert!(!zy.ignored);
    // zyBooks' onboarding book comes back flagged, so the panel can pre-tick "ignore" rather than
    // asking a student what course "HowToUseZyBooks2" is.
    assert!(rows[1].ignored);
    assert_eq!(rows[1].suggested, None);
    let vhl = &rows[2];
    assert_eq!(vhl.source, "vhl");
    assert_eq!(vhl.key, "2102121");
    // The dashboard names no course text at all — only ids — so there is nothing to suggest and the
    // panel asks. Inventing one here would be a guess wearing a suggestion's clothes.
    assert_eq!(vhl.suggested, None);
    assert_eq!(vhl.detail.as_deref(), Some("course 1623220"));

    // A source that could not be reached is rows we do not have, not an error the panel dies on —
    // and the reason survives, because "your VHL password is wrong" and "try again" are different
    // instructions.
    use knowlu::onboarding::errors_from_discovery;
    let payload = r#"{"errors": ["vhl: fetch failed (…)"], "vhl": [], "zybooks": [{"code": "UACS100Fall2026", "ignored": false, "mapped": false}]}"#;
    assert_eq!(rows_from_discovery(payload).len(), 1, "zyBooks still worked");
    assert_eq!(errors_from_discovery(payload), vec!["vhl: fetch failed (…)".to_string()]);
    let failed = rows_from_discovery(r#"{"errors": ["zybooks: fetch failed (…)"], "vhl": [], "zybooks": []}"#);
    assert!(failed.is_empty());
    // …and garbage is empty too: the panel's own copy tells the student to type the mapping.
    assert!(rows_from_discovery("not json").is_empty());
}

/// `KNOWLU_ENGINE_EXE` is process-wide state; this file's own lock (mirroring
/// `app/tests/scheduler.rs`'s `ENGINE_ENV_LOCK`) keeps the tests below from racing each other or
/// any future test in this binary that sets it.
static ENGINE_ENV_LOCK: std::sync::Mutex<()> = std::sync::Mutex::new(());

/// An RAII seam for `KNOWLU_ENGINE_EXE`, the same shape as `scheduler.rs`'s `EnvSeam`: the restore
/// runs on drop, so a panicking assertion never leaves the override set for the rest of the binary.
struct EngineExeSeam(Option<std::ffi::OsString>);
impl EngineExeSeam {
    fn set(exe: &std::ffi::OsStr) -> Self {
        let prev = std::env::var_os("KNOWLU_ENGINE_EXE");
        unsafe { std::env::set_var("KNOWLU_ENGINE_EXE", exe) };
        EngineExeSeam(prev)
    }
}
impl Drop for EngineExeSeam {
    fn drop(&mut self) {
        match self.0.take() {
            Some(p) => unsafe { std::env::set_var("KNOWLU_ENGINE_EXE", p) },
            None => unsafe { std::env::remove_var("KNOWLU_ENGINE_EXE") },
        }
    }
}

/// Review round 1, m4: `discovery_argv` pinned directly — which flags appear when, and that no
/// argument is ever anything but a credential TARGET name.
#[test]
fn discovery_argv_omits_a_missing_source_and_never_carries_a_secret() {
    use knowlu::onboarding::discovery_argv;
    assert_eq!(
        discovery_argv("profile_x", true, true),
        vec![
            "coursework-discover".to_string(),
            "--zybooks-target".to_string(), "knowlu/profile_x/zybooks".to_string(),
            "--vhl-target".to_string(), "knowlu/profile_x/vhl".to_string(),
        ]
    );
    let zybooks_only = discovery_argv("profile_x", true, false);
    assert_eq!(zybooks_only, vec!["coursework-discover".to_string(), "--zybooks-target".to_string(), "knowlu/profile_x/zybooks".to_string()]);
    assert!(!zybooks_only.iter().any(|a| a.contains("vhl")), "a false flag omits its pair entirely: {zybooks_only:?}");
    let vhl_only = discovery_argv("profile_x", false, true);
    assert_eq!(vhl_only, vec!["coursework-discover".to_string(), "--vhl-target".to_string(), "knowlu/profile_x/vhl".to_string()]);
    assert_eq!(discovery_argv("profile_x", false, false), vec!["coursework-discover".to_string()]);
    // The constraint that actually matters: every argument is the subcommand name or a target
    // NAME built by `credentials::target_for` — never a username or a password.
    for a in discovery_argv("profile_x", true, true) {
        assert!(!a.to_lowercase().contains("password"));
    }
}

/// Review round 1, m5 (required this round): the child's exit status and stderr are surfaced in
/// the envelope's `error`, never discarded. A throwaway batch file stands in for a broken engine —
/// exits 7, writes to stderr, never touches Credential Manager or any real vault.
#[test]
fn a_child_that_fails_outright_surfaces_its_exit_status_and_stderr_in_error() {
    let _guard = ENGINE_ENV_LOCK.lock().unwrap_or_else(|e| e.into_inner());
    let root = tmp("discover-fail");
    let fake_exe = root.join("fake-engine.bat");
    std::fs::write(&fake_exe, "@echo off\r\necho boom-from-fake-engine 1>&2\r\nexit /b 7\r\n").unwrap();
    let _seam = EngineExeSeam::set(fake_exe.as_os_str());
    let out = knowlu::onboarding::discover_coursework(root.join("Vault").to_string_lossy().to_string(), true, true);
    assert_eq!(out["ok"], true, "{out}");
    let err = out["error"].as_str().expect("the child's own failure is named, not discarded: {out}");
    assert!(err.contains('7'), "the exit status is in it: {err}");
    assert!(err.contains("boom-from-fake-engine"), "the stderr is in it: {err}");
    // stdout was empty (no valid discovery JSON), so rows are empty and the note is the generic one.
    assert!(out["rows"].as_array().unwrap().is_empty());
    assert_eq!(out["note"].as_str(), Some("we could not reach your coursework sites — fill them in below"));
    let _ = std::fs::remove_dir_all(&root);
}

/// Review round 1, m4 (required this round): `discover_coursework` against the REAL engine binary,
/// with credential targets that have never had a login stored for them —
/// `coursework-discover`'s own contract exercised for real (always exits 0; an unreachable source
/// is an `errors` entry), not only through `rows_from_discovery`'s hand-built JSON. No secret
/// anywhere: the vault path is a throwaway temp folder nothing has ever signed into.
#[test]
fn discover_coursework_against_the_real_engine_names_the_missing_credential() {
    let exe = std::path::Path::new("../target/debug/knowlu-engine.exe");
    assert!(exe.is_file(), "{}: build it first (`cargo build -p knowlu-engine`)", exe.display());
    let _guard = ENGINE_ENV_LOCK.lock().unwrap_or_else(|e| e.into_inner());
    let _seam = EngineExeSeam::set(exe.as_os_str());
    let vault = tmp("discover-real").join("Vault");
    let out = knowlu::onboarding::discover_coursework(vault.to_string_lossy().to_string(), true, true);
    assert_eq!(out["ok"], true, "{out}");
    assert_eq!(out["error"], serde_json::Value::Null, "the real engine exits 0, so there is no child failure to surface: {out}");
    let rows = out["rows"].as_array().expect("rows array");
    assert!(rows.is_empty(), "a target that has never had a login stored finds nothing: {out}");
    let note = out["note"].as_str().expect("a note names why: {out}");
    assert!(note.to_lowercase().contains("zybooks") || note.to_lowercase().contains("vhl"), "{note}");
    // No credential anywhere in the envelope.
    let text = out.to_string();
    assert!(!text.to_lowercase().contains("password"));
}

/// R-OB-4: the school list is a committed asset, because a typeahead that needs a network call to
/// show a school does not work in a dorm on move-in day — which is most first runs.
#[test]
fn the_campus_list_is_bundled_headed_and_small() {
    let raw = std::fs::read_to_string("campuses.json").expect("app/campuses.json");
    // The header is the first line, and it is what makes "is this current?" answerable without
    // re-downloading a federal zip.
    let head = raw.lines().next().expect("a first line");
    assert!(head.starts_with("{\"source\":\"NCES IPEDS HD"), "the header names its source file: {head}");
    assert!(head.contains("\"retrieved\":\"20"), "…and when it was taken: {head}");
    assert!(head.contains("\"count\":"), "…and how many schools it holds: {head}");

    let v: serde_json::Value = serde_json::from_str(&raw).expect("campuses.json is one JSON object");
    let rows = v["campuses"].as_array().expect("campuses is an array");
    assert_eq!(rows.len() as u64, v["count"].as_u64().expect("count is a number"), "the header's count is the array's length");
    // 4,319 active two- and four-year institutions in HD2024 (of 6,072 rows; R-C1-37). A file that suddenly holds 40 of them is
    // a script that half-ran, and a bundle that holds 40,000 is one that stopped filtering.
    assert!(rows.len() > 4_000 && rows.len() < 8_000, "{} schools is not a US institution list", rows.len());

    // `[unitid, name, city, state, host]`, and the host is a HOST: this test is the guard that the
    // bundled list carries no `http(s)://` literal — `static_assets.rs` cannot be, because its
    // `read()` helper resolves against `app/static/` and this asset is `app/campuses.json`.
    let first = rows[0].as_array().expect("a row is an array");
    assert_eq!(first.len(), 5, "a row is [unitid, name, city, state, host]");
    assert!(first[0].is_number() && first[1].is_string() && first[3].is_string());
    assert!(!raw.contains("http://") && !raw.contains("https://"), "the list carries hosts, never URLs");

    // The size guard the installer cares about. H11's script refuses to write past this too.
    let kb = raw.len() / 1024;
    assert!(kb < 600, "campuses.json is {kb} KB");
}

/// Every curated school must be **in** the bundled list, or picking it from the typeahead and then
/// looking it up in `CAMPUSES` would answer with two different schools.
#[test]
fn every_curated_campus_is_in_the_bundled_list() {
    let raw = std::fs::read_to_string("campuses.json").expect("app/campuses.json");
    let v: serde_json::Value = serde_json::from_str(&raw).expect("campuses.json");
    let rows = v["campuses"].as_array().expect("campuses");
    for c in knowlu::scaffold::CAMPUSES {
        let want: u64 = c.unitid.parse().expect("a unitid is a number");
        let found = rows.iter().find(|r| r[0].as_u64() == Some(want)).unwrap_or_else(|| panic!("{} ({}) is not in campuses.json", c.label, c.unitid));
        // …and it is the school we think it is. A curated row that named the wrong unitid would send
        // a student's sign-in window to another university's LMS.
        let name = found[1].as_str().unwrap_or_default();
        assert!(name.to_lowercase().contains(&c.label.to_lowercase()) || c.label.to_lowercase().contains(&name.to_lowercase()),
            "{} is unitid {} in our table and {name:?} in IPEDS", c.label, c.unitid);
    }
}

/// R-OB-4: the search is Rust's, because the page cannot fetch the list (the app's CSP names no
/// `'self'` in `connect-src`) and should not hold six thousand rows to answer a keystroke.
#[test]
fn typing_a_school_name_finds_it_and_typing_one_letter_finds_nothing() {
    use knowlu::onboarding::campus_search;
    let one = campus_search("a".into());
    assert_eq!(one["hits"].as_array().map(Vec::len), Some(0), "one letter is not a search");
    let hits = campus_search("university of alabama".into());
    let rows = hits["hits"].as_array().expect("hits");
    assert!(!rows.is_empty() && rows.len() <= 10, "{} hits", rows.len());
    assert!(rows.iter().any(|r| r[0].as_u64() == Some(100751)), "{rows:?}");
    // A row is [unitid, name, city, state] — the web host stays in the asset, unshown and uncrossed.
    assert_eq!(rows[0].as_array().map(Vec::len), Some(4));
    // City and state match too, or a student who knows where they go and not what it is called is stuck.
    assert!(campus_search("tuscaloosa".into())["hits"].as_array().map(|r| !r.is_empty()).unwrap_or(false));
}

/// R-C1-51 item 1: `plan.course_map` is `(fragment, slug)`. When the page sends a non-empty slug,
/// `create_vault_in` honours it verbatim; when it sends an empty one, the fragment is slugified.
///
/// Needs a real pending session, the same as every other `create_vault_in` test in this file
/// (`PendingSession`'s own doc explains why).
#[cfg(windows)]
#[test]
fn create_vault_in_honours_the_pages_course_map_slug_and_falls_back_when_blank() {
    let root = tmp("course-map-slug");
    let home = root.join("home");
    let mut session = PendingSession::new("acc-course-map-slug");
    let mut plan = base_plan(false);
    plan.course_map = vec![
        ("CS 100".to_string(), "cs-100-custom".to_string()),
        ("GN 103".to_string(), String::new()),
    ];
    let out = create_vault_in(&root, &home, "Fall 2026", &plan);
    assert_eq!(out["ok"], true, "{out}");
    let id = out["profile"]["id"].as_str().expect("a profile id").to_string();
    session.expect_move_to(&id);
    let vault = home.join("Knowlu").join("Fall 2026");
    let ingest = std::fs::read_to_string(vault.join("config").join("ingest.yaml")).expect("ingest.yaml");
    // The page's own slug, unrecomputed.
    assert!(ingest.contains("'CS 100': 'cs-100-custom'"), "{ingest}");
    // An empty slug falls back to `slugify(fragment)`.
    assert!(ingest.contains("'GN 103': 'gn-103'"), "{ingest}");
    let _ = std::fs::remove_dir_all(&root);
}

/// R-C1-51 item 2: a page-supplied `CourseSeed.slug` reaches `courses/<slug>.md` only after
/// `knowlu_engine::ingest::slugify` — a slug carrying a path separator or spaces must not name a
/// file outside `courses/` or with spaces in its stem.
#[cfg(windows)]
#[test]
fn a_page_supplied_course_slug_is_normalised_before_it_names_a_file() {
    let root = tmp("course-slug-normalise");
    let home = root.join("home");
    let mut session = PendingSession::new("acc-course-slug-normalise");
    let mut plan = base_plan(false);
    plan.courses = vec![
        knowlu::scaffold::CourseSeed { code: "CS 100".into(), name: "CS 100".into(), slug: "../../evil".into() },
        knowlu::scaffold::CourseSeed { code: "GN 103".into(), name: "GN 103".into(), slug: "  has spaces  ".into() },
    ];
    let out = create_vault_in(&root, &home, "Fall 2026", &plan);
    assert_eq!(out["ok"], true, "{out}");
    let id = out["profile"]["id"].as_str().expect("a profile id").to_string();
    session.expect_move_to(&id);
    let vault = home.join("Knowlu").join("Fall 2026");
    let courses_dir = vault.join("courses");
    let mut names: Vec<String> = std::fs::read_dir(&courses_dir).unwrap().flatten()
        .map(|e| e.file_name().to_string_lossy().to_string())
        .filter(|n| n != "get-to-know-knowlu.md")
        .collect();
    names.sort();
    // Neither slug escaped `courses/` and neither carries a raw space or slash.
    for n in &names {
        assert!(!n.contains("..") && !n.contains('/') && !n.contains("  "), "{n}");
    }
    assert_eq!(names, vec!["evil.md".to_string(), "has-spaces.md".to_string()]);
    let _ = std::fs::remove_dir_all(&root);
}

/// **R-C1-55 (I2)**: *Use an existing vault* must refuse a backup mirror. `backup::mirror` writes
/// `<backups root>\<profile id>\vault\`, which carries `config/planning.yaml` and `tasks/` and so
/// passes the vault predicate exactly — and adopting it would register the backup AS the live vault,
/// the one thing `restore_vault_in`'s own doc says must never happen: the next tick would then mirror
/// the vault over itself, and the student's only other copy of their work would be gone.
///
/// Detected structurally, never by the word "backup" in a path: a folder named `vault` whose parent
/// holds what a backup tick leaves beside it (`status.json`, `snapshots\`), or whose parent is a
/// `profile_<10 hex>` id under a `Backups` folder — the layout `default_folders_in` hands every
/// wizard-made vault.
#[test]
fn adopting_a_backup_mirror_is_refused_and_registers_nothing() {
    let root = tmp("adopt-mirror");
    let app_data = root.join("appdata");
    let backups = root.join("Knowlu").join("Backups");
    let mirror = backups.join("profile_1111111111").join("vault");
    copy(Path::new("../engine/tests/fixtures/vault-s1"), &mirror);
    // What `backup::tick` leaves beside the mirror on every successful run.
    std::fs::write(backups.join("profile_1111111111").join("status.json"), "{}").unwrap();
    std::fs::create_dir_all(backups.join("profile_1111111111").join("snapshots")).unwrap();
    let before = fingerprint(&backups);

    let out = adopt_vault_in(&app_data, mirror.to_str().unwrap(), None);
    assert_eq!(out["ok"], false, "a backup mirror is not a vault to open: {out}");
    let err = out["error"].as_str().unwrap();
    assert!(err.contains("backup"), "the sentence the picker shows must say what it is: {err}");
    assert!(knowlu::profiles::load(&app_data).unwrap().is_empty(), "nothing may be registered");
    assert_eq!(fingerprint(&backups), before, "the backup folder was written to");

    // …and one that has not been ticked yet — no `status.json`, no `snapshots\` — is refused on the
    // layout alone.
    let bare = root.join("Knowlu").join("Backups").join("profile_2222222222").join("vault");
    copy(Path::new("../engine/tests/fixtures/vault-s1"), &bare);
    let out = adopt_vault_in(&app_data, bare.to_str().unwrap(), None);
    assert_eq!(out["ok"], false, "the default backups layout is refused before a first tick: {out}");

    // The guard is structural, so a real vault that merely lives near a backup folder is still
    // adoptable — including one a student happened to name `vault`.
    let live = root.join("Knowlu").join("vault");
    copy(Path::new("../engine/tests/fixtures/vault-s1"), &live);
    let out = adopt_vault_in(&app_data, live.to_str().unwrap(), None);
    assert_eq!(out["ok"], true, "an ordinary vault must still be adoptable: {out}");
    let _ = std::fs::remove_dir_all(&root);
}
