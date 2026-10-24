//! The two "never wrote there" tests (plan 4a, Task 6; spec §8, S7). Both drive the handle-free
//! cores in `app/src/onboarding.rs`, so they need no Tauri app and no window: adopting and
//! restoring are the two paths where the app touches a folder the USER already owns, and the
//! promise on both is the same — it is read, never written.
use knowlu::onboarding::{
    adopt_vault_in, apply_profile_settings_in, create_vault_in, dest_for, finish_or_roll_back,
    default_folders_in, mail_offer_marker, restore_vault_in, WizardPlan,
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
        ics_validated: false,
        ics_stored: false,
        personal_calendar: None,
        personal_calendar_validated: false,
        personal_calendar_stored: false,
        google_calendar: false,
        timezone: "America/Chicago".to_string(),
        slots: vec!["12:00".to_string(), "18:00".to_string()],
        campus_choice: Default::default(),
        zybooks: false,
        vhl: false,
        autostart: true,
        offer_mail: false,
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
        google_calendar: false,
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
        human_actor: knowlu_engine::journal::HUMAN_ACTOR,
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
///
/// The name is shared by every process too, not only this binary's threads, so the lock is the
/// cross-process one in `support/credman_lock.rs`
/// (`holding_the_credman_lock_holds_the_mutex_other_test_processes_wait_on`).
#[cfg(windows)]
#[path = "support/credman_lock.rs"]
mod credman_lock;
#[cfg(windows)]
static CREDMAN_LOCK: credman_lock::CredmanLock = credman_lock::CredmanLock::new();

#[cfg(windows)]
struct PendingSession {
    _guard: credman_lock::CredmanGuard,
    /// Whatever was already at `PENDING_TARGET` before this guard wrote over it — `(username,
    /// secret)`, read raw rather than through `load_session` so a session this process cannot even
    /// parse is still put back byte for byte. `None` means there was nothing there.
    had_previous: Option<(String, String)>,
    moved_to: Vec<String>,
    /// The process-global `KNOWLU_API_BASE` from before this guard pointed it at its own
    /// [`AccountStub`], restored on drop — the same shape `app/tests/account.rs::ApiBase` uses, and covered by
    /// the same `CREDMAN_LOCK` this struct already holds for its whole life (`_guard` above), so the
    /// two kinds of process-global state never race each other either.
    ///
    /// **C3' Task 9, H11a's consequence.** `create_vault_in` now calls `sync::restore_into(&dest)`
    /// right after `move_session` succeeds, and that reads `config/cloud.yaml`'s `api_base` — which
    /// this file's `VaultPlan` always sets from `account::api_base()`. Every test below that reaches
    /// a live `create_vault_in` would otherwise make a real request to the compiled-in project
    /// (`DEFAULT_API_BASE`) the moment it does.
    ///
    /// **Ruling 11 (T5).** Finish now reads the account's journal before it makes anything, and a
    /// Finish that cannot reach the account is refused (plan Q1) — so the closed loopback port this
    /// used to point at would refuse every Finish below. It points at [`AccountStub`] instead: an
    /// empty account (every `GET …/sync-pull` answers an empty page) whose every other call fails
    /// with 503, so a feed's retried save still fails exactly as the closed port made it fail.
    prev_api_base: Option<std::ffi::OsString>,
    _account: AccountStub,
}

/// A loopback account for the life of a [`PendingSession`]: an empty `/sync-pull` page for every
/// `GET` of it, 503 for anything else. Serves itself on `127.0.0.1:0`; stops and joins on drop.
#[cfg(windows)]
struct AccountStub {
    base: String,
    stop: std::sync::Arc<std::sync::atomic::AtomicBool>,
    handle: Option<std::thread::JoinHandle<()>>,
}
#[cfg(windows)]
impl AccountStub {
    fn start() -> Self {
        use std::io::{Read, Write};
        use std::sync::atomic::Ordering;
        let listener = std::net::TcpListener::bind("127.0.0.1:0").expect("bind loopback");
        listener.set_nonblocking(true).expect("nonblocking listener");
        let port = listener.local_addr().expect("addr").port();
        let stop = std::sync::Arc::new(std::sync::atomic::AtomicBool::new(false));
        let flag = stop.clone();
        let handle = std::thread::spawn(move || {
            while !flag.load(Ordering::SeqCst) {
                let mut stream = match listener.accept() {
                    Ok((s, _)) => s,
                    Err(e) if e.kind() == std::io::ErrorKind::WouldBlock => {
                        std::thread::sleep(std::time::Duration::from_millis(10));
                        continue;
                    }
                    Err(_) => break,
                };
                let _ = stream.set_nonblocking(false);
                let _ = stream.set_read_timeout(Some(std::time::Duration::from_secs(10)));
                let mut buf: Vec<u8> = Vec::new();
                let mut chunk = [0u8; 1024];
                while !buf.windows(4).any(|w| w == b"\r\n\r\n") {
                    match stream.read(&mut chunk) { Ok(0) | Err(_) => break, Ok(n) => buf.extend_from_slice(&chunk[..n]) }
                }
                let head = String::from_utf8_lossy(&buf).to_string();
                let pull = head.starts_with("GET ") && head.contains("/sync-pull");
                let (status, body) = if pull { ("200 OK", EMPTY_PULL_PAGE) } else { ("503 Error", "") };
                let resp = format!(
                    "HTTP/1.1 {status}\r\ncontent-type: application/json\r\ncontent-length: {}\r\nconnection: close\r\n\r\n{body}",
                    body.len()
                );
                let _ = stream.write_all(resp.as_bytes());
                let _ = stream.flush();
            }
        });
        AccountStub { base: format!("http://127.0.0.1:{port}/functions/v1"), stop, handle: Some(handle) }
    }
}
#[cfg(windows)]
impl Drop for AccountStub {
    fn drop(&mut self) {
        self.stop.store(true, std::sync::atomic::Ordering::SeqCst);
        if let Some(h) = self.handle.take() { let _ = h.join(); }
    }
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
        let prev_api_base = std::env::var_os("KNOWLU_API_BASE");
        let account = AccountStub::start();
        unsafe { std::env::set_var("KNOWLU_API_BASE", &account.base) };
        Self { _guard: guard, had_previous, moved_to: Vec::new(), prev_api_base, _account: account }
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
        match self.prev_api_base.take() {
            Some(v) => unsafe { std::env::set_var("KNOWLU_API_BASE", v) },
            None => unsafe { std::env::remove_var("KNOWLU_API_BASE") },
        }
    }
}

/// A loopback base **nothing is listening on**: a port is bound only long enough to learn that it is
/// free, then released — the same construction `app/tests/account.rs::closed_loopback_base` uses, so
/// a request to it is refused in microseconds rather than reaching the network or waiting out a
/// timeout.
#[cfg(windows)]
fn closed_loopback_base() -> String {
    let l = std::net::TcpListener::bind("127.0.0.1:0").expect("bind loopback");
    let port = l.local_addr().expect("addr").port();
    drop(l);
    format!("http://127.0.0.1:{port}/functions/v1")
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

/// A minimal plan: every checkbox off, the default campus choice, two slots.
fn base_plan() -> WizardPlan {
    WizardPlan {
        ics_url: None,
        ics_validated: false,
        ics_stored: false,
        personal_calendar: None,
        personal_calendar_validated: false,
        personal_calendar_stored: false,
        google_calendar: false,
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
        offer_mail: false,
    }
}

/// A page cached from an older build still sends `offer_inference`. `WizardPlan` has no
/// `deny_unknown_fields`, so the key is ignored and the plan is not refused. Green while the field
/// exists and green after it is gone: it pins the upgrade path, not the field.
#[test]
fn a_wizard_plan_that_still_sends_offer_inference_is_accepted() {
    let plan: WizardPlan = serde_json::from_value(serde_json::json!({
        "timezone": "America/Chicago",
        "slots": ["12:00", "18:00"],
        "zybooks": false,
        "vhl": false,
        "autostart": true,
        "offer_inference": true,
    }))
    .expect("an older page's key is ignored, not refused");
    assert_eq!(plan.timezone, "America/Chicago");
    assert!(plan.autostart);
}

/// **The cross-process flake (2026-09-29).** `PENDING_TARGET` is one machine-wide name, and two
/// processes running this file at once (two lanes, two worktrees, a workspace run beside a targeted
/// one) used to interleave on it: one process's `create_vault_in` moved the other's pending session
/// away, or its no-session test deleted it, and a `PendingSession` that read the other process's
/// session as "what was here before" restored it after that process had cleaned up — leaving a test
/// session at the real pending target. A `std::sync::Mutex` cannot see another process.
///
/// So holding `CREDMAN_LOCK` must also hold the named, session-wide mutex every real-store test file
/// shares. Probed from a second thread with its own handle: a Windows mutex is owned per thread, so
/// the probe sees exactly what another process would.
#[cfg(windows)]
#[test]
fn holding_the_credman_lock_holds_the_mutex_other_test_processes_wait_on() {
    fn probe(wait_ms: u32) -> bool {
        use windows::Win32::Foundation::{CloseHandle, WAIT_ABANDONED, WAIT_OBJECT_0};
        use windows::Win32::System::Threading::{CreateMutexW, ReleaseMutex, WaitForSingleObject};
        std::thread::spawn(move || unsafe {
            let h = CreateMutexW(None, false, windows::core::w!("Local\\knowlu-tests-credman")).expect("open the named mutex");
            let r = WaitForSingleObject(h, wait_ms);
            let got = r == WAIT_OBJECT_0 || r == WAIT_ABANDONED;
            if got { let _ = ReleaseMutex(h); }
            let _ = CloseHandle(h);
            got
        })
        .join()
        .expect("probe thread")
    }
    {
        let _guard = CREDMAN_LOCK.lock().unwrap_or_else(|e| e.into_inner());
        assert!(!probe(0), "another process could take the Credential Manager lock while this one holds it");
    }
    // Released on drop. Other holders (this process's other tests, another process) come and go, so
    // this waits for its turn rather than demanding the mutex be free this instant.
    assert!(probe(300_000), "the named mutex was never released");
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
    let out = create_vault_in(&app_data, &home, "Fall 2026", &base_plan());
    assert_eq!(out["ok"], false, "{out}");
    assert!(out["error"].as_str().unwrap().contains("sign in again"), "{out}");
    assert!(!home.join("Knowlu").join("Fall 2026").exists(), "no vault directory was created");

    match had_previous {
        Some((user, secret)) => { knowlu::credentials::write(knowlu::account::PENDING_TARGET, &user, &secret).unwrap(); }
        None => {}
    }
    let _ = std::fs::remove_dir_all(&root);
}

/// A loopback server that answers exactly two `GET`s (`/sync-pull`) with `body` verbatim — one page
/// each, which is all a Finish on an account this small sends (`more: false` in `body`): **the
/// token probe's (ruling 11, T5), then H11a's restore's**. Modelled on
/// `engine/tests/sync_contract.rs`'s own `loopback` (same protocol: `cloudmodel::CloudClient` is the
/// client on both sides of it).
///
/// **I6 (fix round 1): an accept deadline**, the same nonblocking-plus-10s-poll shape
/// `app/tests/account.rs`'s own `loopback` already uses — a request that never arrives now fails
/// this test loudly in seconds, where the original blocking `accept()` would have hung until the
/// suite's own outer timeout killed it, with no line saying why.
#[cfg(windows)]
fn restore_loopback(body: &str) -> (String, std::thread::JoinHandle<()>) {
    let listener = std::net::TcpListener::bind("127.0.0.1:0").expect("bind loopback");
    listener.set_nonblocking(true).expect("nonblocking listener");
    let port = listener.local_addr().expect("addr").port();
    let body = body.to_string();
    let handle = std::thread::spawn(move || for _ in 0..2 {
        let deadline = std::time::Instant::now() + std::time::Duration::from_secs(10);
        let mut stream = loop {
            match listener.accept() {
                Ok((s, _)) => break s,
                Err(e) if e.kind() == std::io::ErrorKind::WouldBlock => {
                    if std::time::Instant::now() >= deadline {
                        panic!("restore_loopback: no client connected within 10s");
                    }
                    std::thread::sleep(std::time::Duration::from_millis(10));
                }
                Err(e) => panic!("restore_loopback: accept failed: {e}"),
            }
        };
        stream.set_nonblocking(false).expect("blocking stream");
        stream.set_read_timeout(Some(std::time::Duration::from_secs(10))).expect("read timeout");
        // Drain the request head so a client that flushes only after a full write is never left
        // waiting on us — the request is a `GET` with no body, so a blank line ends it.
        {
            use std::io::BufRead;
            let mut reader = std::io::BufReader::new(stream.try_clone().expect("clone the accepted stream"));
            loop {
                let mut line = String::new();
                if reader.read_line(&mut line).unwrap_or(0) == 0 || line == "\r\n" || line == "\n" {
                    break;
                }
            }
        }
        let response = format!(
            "HTTP/1.1 200 OK\r\ncontent-type: application/json\r\ncontent-length: {}\r\nconnection: close\r\n\r\n{}",
            body.len(),
            body
        );
        use std::io::Write;
        let _ = stream.write_all(response.as_bytes());
        let _ = stream.flush();
    });
    (format!("http://127.0.0.1:{port}/functions/v1"), handle)
}

/// Serves `responses.len()` requests, one per accepted connection, in order — the shape Task 11's
/// fix round I1 needs: `create_vault_in`'s retry (`PUT /account/sources`) and then, if that lands,
/// `sync::restore_into`'s own pull (`GET /sync-pull`), against the same base URL. Modelled on
/// `app/tests/account.rs::loopback` (same nonblocking-accept-plus-10s-deadline and
/// Content-Length-aware read this file's own `restore_loopback` already uses) — copied rather than
/// shared, since each `tests/*.rs` file is its own crate.
#[cfg(windows)]
fn multi_loopback(responses: Vec<(u16, &'static str)>) -> (String, std::thread::JoinHandle<Vec<String>>) {
    let listener = std::net::TcpListener::bind("127.0.0.1:0").expect("bind loopback");
    listener.set_nonblocking(true).expect("nonblocking listener");
    let port = listener.local_addr().expect("addr").port();
    let handle = std::thread::spawn(move || {
        use std::io::{Read, Write};
        let mut seen = Vec::new();
        for (status, body) in responses {
            let deadline = std::time::Instant::now() + std::time::Duration::from_secs(10);
            let mut stream = loop {
                match listener.accept() {
                    Ok((s, _)) => break s,
                    Err(e) if e.kind() == std::io::ErrorKind::WouldBlock => {
                        if std::time::Instant::now() >= deadline {
                            panic!("multi_loopback: no client connected within 10s");
                        }
                        std::thread::sleep(std::time::Duration::from_millis(10));
                    }
                    Err(e) => panic!("multi_loopback: accept failed: {e}"),
                }
            };
            stream.set_nonblocking(false).expect("blocking stream");
            stream.set_read_timeout(Some(std::time::Duration::from_secs(10))).expect("read timeout");
            let mut buf: Vec<u8> = Vec::new();
            let mut chunk = [0u8; 1024];
            let head_end = loop {
                let n = stream.read(&mut chunk).unwrap_or(0);
                if n == 0 { break buf.len(); }
                buf.extend_from_slice(&chunk[..n]);
                if let Some(i) = buf.windows(4).position(|w| w == b"\r\n\r\n") { break i + 4; }
            };
            let head = String::from_utf8_lossy(&buf[..head_end]).to_string();
            let want: usize = head
                .lines()
                .find(|l| l.to_ascii_lowercase().starts_with("content-length:"))
                .and_then(|l| l.split(':').nth(1)?.trim().parse().ok())
                .unwrap_or(0);
            while buf.len() < head_end + want {
                let n = stream.read(&mut chunk).unwrap_or(0);
                if n == 0 { break; }
                buf.extend_from_slice(&chunk[..n]);
            }
            seen.push(String::from_utf8_lossy(&buf).to_string());
            let reason = if (200..300).contains(&status) { "OK" } else { "Error" };
            let resp = format!(
                "HTTP/1.1 {status} {reason}\r\ncontent-type: application/json\r\ncontent-length: {}\r\nconnection: close\r\n\r\n{body}",
                body.len()
            );
            let _ = stream.write_all(resp.as_bytes());
            let _ = stream.flush();
        }
        seen
    });
    (format!("http://127.0.0.1:{port}/functions/v1"), handle)
}

/// The empty, successful `/sync-pull` page `restore_into` reads once a vault has just been made —
/// the same body `restore_loopback`'s own callers pass, kept here as one literal so the three I1
/// tests below do not each spell it out.
const EMPTY_PULL_PAGE: &str = r#"{"records":[],"notes":[],"record_cursor":0,"note_cursor":0,"more":false}"#;

/// C3' Task 9, H11a: Finish now calls `sync::restore_into(&dest)` right after the session moves onto
/// the new profile, and an account that has never pushed answers with a real, successful, empty page
/// — never a network failure. This is that case, end to end: the vault Finish just made is kept
/// (`ok: true`), and the envelope names what the restore found (`restored.empty == true`, nothing
/// counted), the sentence the finish panel reads rather than an error.
///
/// `PendingSession::new` already points `KNOWLU_API_BASE` at its own empty [`AccountStub`] so no
/// OTHER test in this file makes a live request; this one overrides that with a server of its own,
/// for the length of this test only — `PendingSession`'s own drop restores whatever came before
/// `new()`, which is unaffected by this second, later write.
#[cfg(windows)]
#[test]
fn a_finish_whose_account_copy_is_empty_still_keeps_the_vault_and_says_so() {
    let root = tmp("restore-empty");
    let home = root.join("home");
    let app_data = root.join("appdata");
    let mut session = PendingSession::new("acc-restore-empty");
    let (base, handle) = restore_loopback(r#"{"records":[],"notes":[],"record_cursor":0,"note_cursor":0,"more":false}"#);
    unsafe { std::env::set_var("KNOWLU_API_BASE", &base) };
    let out = create_vault_in(&app_data, &home, "Fall 2026", &base_plan());
    assert_eq!(out["ok"], true, "{out}");
    let id = out["profile"]["id"].as_str().expect("a profile id").to_string();
    session.expect_move_to(&id);
    assert_eq!(out["restored"]["empty"], true, "{out}");
    assert_eq!(out["restored"]["notes"], 0, "{out}");
    assert_eq!(out["restored"]["records"], 0, "{out}");
    // N4 (fix round 2): the accept deadline proves a request arrived; it does not by itself prove
    // the reply was ACCEPTED — a parse or status failure folds into the same `empty: true`. Since
    // `warnings` is in the envelope, asserting it is empty is what actually distinguishes "a real,
    // successful, empty pull" from a failure `restore_into` folded to look like one.
    assert_eq!(out["restored"]["warnings"], serde_json::json!([]), "a real empty pull carries no warnings: {out}");
    assert_eq!(out["restored"]["ok"], true, "{out}");
    assert!(home.join("Knowlu").join("Fall 2026").is_dir(), "an empty account copy keeps the vault");
    handle.join().expect("the loopback thread did not panic");
    let _ = std::fs::remove_dir_all(&root);
}

/// I6 (fix round 1): the success-path test above only ever exercises an EMPTY page, which an
/// `empty: true` alone cannot tell apart from a pull that connected and then failed (`restore_into`
/// folds both the same way). This is the other half: a real note and a real record come down, and
/// the assertion is the actual bytes the account sent, landed on the actual disk this call made.
#[cfg(windows)]
#[test]
fn a_finish_whose_account_copy_has_content_writes_it_into_the_new_vault() {
    let root = tmp("restore-content");
    let home = root.join("home");
    let app_data = root.join("appdata");
    let mut session = PendingSession::new("acc-restore-content");
    let device = "aaaaaaaaaaaaaaaa";
    let note_text = "---\nid: task_0000000099\n---\nfrom the account\n";
    let record = serde_json::json!({
        "op": "create", "path": "tasks/from-account.md", "actor": "quinn", "via": "dashboard",
        "device": device, "ts": "2026-08-01T10:00:00.000Z", "id": "task_0000000099",
        "new": { "id": "task_0000000099" }
    });
    let record_body = knowlu_engine::ledger::dumps_value(&record);
    let page = serde_json::json!({
        "records": [{
            "seq": 1, "device": device,
            "record_hash": knowlu_engine::sync::sha256_hex(record_body.as_bytes()),
            "body": record_body
        }],
        "notes": [{ "path": "tasks/from-account.md", "device": device, "deleted": false, "body": note_text }],
        "record_cursor": 1, "note_cursor": 1, "more": false
    });
    let (base, handle) = restore_loopback(&knowlu_engine::ledger::dumps_value(&page));
    unsafe { std::env::set_var("KNOWLU_API_BASE", &base) };
    let out = create_vault_in(&app_data, &home, "Fall 2026", &base_plan());
    assert_eq!(out["ok"], true, "{out}");
    let id = out["profile"]["id"].as_str().expect("a profile id").to_string();
    session.expect_move_to(&id);
    assert_eq!(out["restored"]["empty"], false, "{out}");
    assert_eq!(out["restored"]["notes"], 1, "{out}");
    assert_eq!(out["restored"]["records"], 1, "{out}");
    assert_eq!(out["restored"]["ok"], true, "{out}");
    let restored_note = home.join("Knowlu").join("Fall 2026").join("tasks").join("from-account.md");
    assert_eq!(
        knowlu_engine::pystr::read_text(&restored_note).expect("the restored note"),
        note_text,
        "the account's own note text is what actually landed on disk, not merely a count"
    );
    handle.join().expect("the loopback thread did not panic");
    let _ = std::fs::remove_dir_all(&root);
}

// -- Ruling 11 (T5): Finish decides the vault's token from the account's journal ----------------

/// One journal record as `/sync-pull` carries it — `body` the record's own bytes, `record_hash`
/// their sha256 — so the probe and the restore read exactly what an account would hold.
#[cfg(windows)]
fn pulled_row(actor: &str, id: &str, ts: &str) -> serde_json::Value {
    let record = serde_json::json!({
        "op": "create", "path": format!("tasks/{id}.md"), "actor": actor, "via": "dashboard",
        "device": "invented", "ts": ts, "id": id, "new": { "id": id }
    });
    let body = knowlu_engine::ledger::dumps_value(&record);
    serde_json::json!({ "seq": 1, "device": "aaaaaaaaaaaaaaaa", "record_hash": knowlu_engine::sync::sha256_hex(body.as_bytes()), "body": body })
}

/// One `/sync-pull` page holding `rows` and no notes.
#[cfg(windows)]
fn pull_page(rows: Vec<serde_json::Value>, cursor: i64, more: bool) -> String {
    knowlu_engine::ledger::dumps_value(&serde_json::json!({ "records": rows, "notes": [], "record_cursor": cursor, "note_cursor": 0, "more": more }))
}

/// The new vault's `config/actor.yaml`, its first task and its whole journal, as Finish left them.
#[cfg(windows)]
fn made(home: &Path) -> (String, String, String) {
    let v = home.join("Knowlu").join("Fall 2026");
    let actor = knowlu_engine::pystr::read_text(&v.join("config").join("actor.yaml")).expect("config/actor.yaml");
    let task = knowlu_engine::pystr::read_text(&v.join("tasks").join("get-to-know-knowlu.md")).expect("the first task");
    let journal = std::fs::read_dir(v.join("state").join("journal")).unwrap().flatten()
        .map(|e| std::fs::read_to_string(e.path()).unwrap()).collect::<String>();
    (actor, task, journal)
}

#[cfg(windows)]
#[test]
fn a_restore_into_an_account_with_quinn_records_writes_quinn() {
    let root = tmp("token-quinn");
    let (home, app_data) = (root.join("home"), root.join("appdata"));
    let mut session = PendingSession::new("acc-token-quinn");
    let (base, handle) = restore_loopback(&pull_page(vec![pulled_row("quinn", "task_00000000a1", "2026-08-01T10:00:00.000Z")], 1, false));
    unsafe { std::env::set_var("KNOWLU_API_BASE", &base) };
    let out = create_vault_in(&app_data, &home, "Fall 2026", &base_plan());
    assert_eq!(out["ok"], true, "{out}");
    session.expect_move_to(out["profile"]["id"].as_str().expect("a profile id"));
    assert_eq!(out["restored"]["records"], 1, "{out}");
    let (actor, task, journal) = made(&home);
    assert_eq!(actor, "human_actor: quinn\n");
    assert!(task.contains("created_by: quinn") && task.contains("effort_source: quinn"), "{task}");
    assert!(!journal.contains("\"actor\": \"student\""), "one account, one token: {journal}");
    handle.join().expect("the loopback thread did not panic");
    let _ = std::fs::remove_dir_all(&root);
}

#[cfg(windows)]
#[test]
fn a_new_account_gets_student() {
    let root = tmp("token-new");
    let (home, app_data) = (root.join("home"), root.join("appdata"));
    // `PendingSession`'s own `AccountStub` is an empty account.
    let mut session = PendingSession::new("acc-token-new");
    let out = create_vault_in(&app_data, &home, "Fall 2026", &base_plan());
    assert_eq!(out["ok"], true, "{out}");
    session.expect_move_to(out["profile"]["id"].as_str().expect("a profile id"));
    let (actor, task, journal) = made(&home);
    assert_eq!(actor, "human_actor: student\n");
    assert!(task.contains("created_by: student") && task.contains("effort_source: student"), "{task}");
    assert!(!journal.contains("quinn"), "{journal}");
    let _ = std::fs::remove_dir_all(&root);
}

#[cfg(windows)]
#[test]
fn an_account_with_only_agent_records_gets_student() {
    let root = tmp("token-agents");
    let (home, app_data) = (root.join("home"), root.join("appdata"));
    let mut session = PendingSession::new("acc-token-agents");
    let rows = vec![
        pulled_row("agent:knowlu.enrich", "task_00000000b1", "2026-08-01T10:00:00.000Z"),
        pulled_row("system:migration", "task_00000000b2", "2026-08-01T10:00:01.000Z"),
    ];
    let (base, handle) = restore_loopback(&pull_page(rows, 2, false));
    unsafe { std::env::set_var("KNOWLU_API_BASE", &base) };
    let out = create_vault_in(&app_data, &home, "Fall 2026", &base_plan());
    assert_eq!(out["ok"], true, "{out}");
    session.expect_move_to(out["profile"]["id"].as_str().expect("a profile id"));
    assert_eq!(made(&home).0, "human_actor: student\n", "no human record: student");
    handle.join().expect("the loopback thread did not panic");
    let _ = std::fs::remove_dir_all(&root);
}

/// The probe pages to the first `quinn`, wherever it is — here only on the second page, after a
/// `student` record on the first.
#[cfg(windows)]
#[test]
fn quinn_anywhere_wins_over_earlier_student_records() {
    let root = tmp("token-pages");
    let (home, app_data) = (root.join("home"), root.join("appdata"));
    let mut session = PendingSession::new("acc-token-pages");
    let first: &'static str = Box::leak(pull_page(vec![pulled_row("student", "task_00000000c1", "2026-08-01T10:00:00.000Z")], 1, true).into_boxed_str());
    let second: &'static str = Box::leak(pull_page(vec![pulled_row("quinn", "task_00000000c2", "2026-08-02T10:00:00.000Z")], 2, false).into_boxed_str());
    // The probe's two pages, then the restore's own two.
    let (base, handle) = multi_loopback(vec![(200, first), (200, second), (200, first), (200, second)]);
    unsafe { std::env::set_var("KNOWLU_API_BASE", &base) };
    let out = create_vault_in(&app_data, &home, "Fall 2026", &base_plan());
    assert_eq!(out["ok"], true, "{out}");
    session.expect_move_to(out["profile"]["id"].as_str().expect("a profile id"));
    assert_eq!(made(&home).0, "human_actor: quinn\n");
    let seen = handle.join().expect("the loopback thread did not panic");
    assert!(seen[0].contains("records_after=0") && seen[1].contains("records_after=1"), "the probe paged on: {seen:?}");
    let _ = std::fs::remove_dir_all(&root);
}

/// Plan Q1 (recommended: refuse). A Finish that cannot read the account makes nothing — no vault
/// folder, no `actor.yaml`, no record — keeps the pending session, says so in a retryable line that
/// never carries the bearer, and succeeds once the account answers.
#[cfg(windows)]
#[test]
fn the_token_is_decided_before_anything_is_written() {
    let root = tmp("token-first");
    let (home, app_data) = (root.join("home"), root.join("appdata"));
    let mut session = PendingSession::new("acc-token-first");
    let dest = home.join("Knowlu").join("Fall 2026");
    let (refusing, handle) = multi_loopback(vec![(503, "")]);
    for base in [closed_loopback_base(), refusing] {
        unsafe { std::env::set_var("KNOWLU_API_BASE", &base) };
        let out = create_vault_in(&app_data, &home, "Fall 2026", &base_plan());
        assert_eq!(out["ok"], false, "{base}: {out}");
        let err = out["error"].as_str().unwrap();
        assert!(err.contains("could not reach your account") && err.contains("press Finish again"), "{err}");
        assert!(!err.contains("test-at"), "the bearer never reaches an error line: {err}");
        assert!(!dest.exists(), "{base}: no vault folder, so no actor.yaml and no record");
        assert!(knowlu::credentials::exists(knowlu::account::PENDING_TARGET), "the sign-in stays for the retry");
    }
    handle.join().expect("the loopback thread did not panic");

    let (base, handle) = restore_loopback(EMPTY_PULL_PAGE);
    unsafe { std::env::set_var("KNOWLU_API_BASE", &base) };
    let out = create_vault_in(&app_data, &home, "Fall 2026", &base_plan());
    assert_eq!(out["ok"], true, "the same Finish, retried once the account answers: {out}");
    session.expect_move_to(out["profile"]["id"].as_str().expect("a profile id"));
    assert_eq!(made(&home).0, "human_actor: student\n");
    handle.join().expect("the loopback thread did not panic");
    let _ = std::fs::remove_dir_all(&root);
}

/// The vault's `config/ingest.yaml`, as `create_vault_in` left it.
#[cfg(windows)]
fn read_ingest(home: &Path) -> String {
    knowlu_engine::pystr::read_text(&home.join("Knowlu").join("Fall 2026").join("config").join("ingest.yaml")).unwrap()
}

/// Task 11 review, I1 (`R-C3'-exec-40`), narrowed by the re-review's N1 (`R-C3'-exec-41`): a feed's
/// save at paste or capture time is a `note`, never an error, so a save that failed there must not
/// lose the feed — `ingest_yaml` has already blanked the vault's own copy for an account vault. The
/// page now carries what that save did into the plan (`ics_validated`, `ics_stored`), and Finish
/// retries ONLY a validated feed the account does not hold yet, under the session it has just moved
/// onto the new profile. This is that retry landing: the one `PUT` goes out, and the vault stays blank.
#[cfg(windows)]
#[test]
fn a_feed_that_failed_at_paste_is_saved_by_the_retry_at_finish() {
    let root = tmp("i1-retry-succeeds");
    let home = root.join("home");
    let app_data = root.join("appdata");
    let mut session = PendingSession::new("acc-i1-retry-ok");
    let (base, handle) = multi_loopback(vec![
        (200, EMPTY_PULL_PAGE),           // ruling 11: the token probe, before anything is made
        (200, r#"{"kind":"lms_ics"}"#),   // `create_vault_in`'s retry, and this time it lands
        (200, EMPTY_PULL_PAGE),           // the restore's own pull, once the vault exists
    ]);
    unsafe { std::env::set_var("KNOWLU_API_BASE", &base) };
    let mut plan = base_plan();
    plan.ics_url = Some("https://x.invalid/a.ics".to_string());
    (plan.ics_validated, plan.ics_stored) = (true, false);
    let out = create_vault_in(&app_data, &home, "Fall 2026", &plan);
    assert_eq!(out["ok"], true, "{out}");
    let id = out["profile"]["id"].as_str().expect("a profile id").to_string();
    session.expect_move_to(&id);
    let ingest = read_ingest(&home);
    assert!(ingest.contains("ics_url: ''\n"), "the retry landed, so the link is in the account and the vault stays empty: {ingest}");
    let seen = handle.join().expect("the loopback thread did not panic");
    assert!(seen[0].starts_with("GET /functions/v1/sync-pull"), "the token probe comes first: {}", seen[0]);
    assert!(seen[1].starts_with("PUT /functions/v1/account/sources ") && seen[1].contains("\"kind\":\"lms_ics\""), "{}", seen[1]);
    let _ = std::fs::remove_dir_all(&root);
}

/// The other half: the retry ALSO fails, so the feed is written into the vault instead of being lost
/// — the same shape `ingest_yaml` would have written for a vault with no account at all.
#[cfg(windows)]
#[test]
fn a_feed_that_fails_the_retry_too_is_kept_on_this_machine() {
    let root = tmp("i1-retry-fails");
    let home = root.join("home");
    let app_data = root.join("appdata");
    let mut session = PendingSession::new("acc-i1-retry-fail");
    let (base, handle) = multi_loopback(vec![
        (200, EMPTY_PULL_PAGE),  // ruling 11: the token probe, before anything is made
        (503, ""),               // `create_vault_in`'s retry fails
        (200, EMPTY_PULL_PAGE),  // the restore still runs — one feed's own save is not the account
    ]);
    unsafe { std::env::set_var("KNOWLU_API_BASE", &base) };
    let mut plan = base_plan();
    plan.ics_url = Some("https://x.invalid/b.ics".to_string());
    (plan.ics_validated, plan.ics_stored) = (true, false);
    let out = create_vault_in(&app_data, &home, "Fall 2026", &plan);
    assert_eq!(out["ok"], true, "{out}");
    let id = out["profile"]["id"].as_str().expect("a profile id").to_string();
    session.expect_move_to(&id);
    let ingest = read_ingest(&home);
    assert!(ingest.contains("ics_url: 'https://x.invalid/b.ics'\n"), "the feed is never lost, even when the account never answers: {ingest}");
    handle.join().expect("the loopback thread did not panic");
    let _ = std::fs::remove_dir_all(&root);
}

/// N1 (a) and (c): a feed the paste already saved is NOT sent again at Finish. Before the flag, the
/// retry was unconditional, so a transient failure of that second, needless call wrote a link the
/// account already held into the vault too, and a hung service held Finish for up to a minute. The
/// only request here is the restore's own pull.
#[cfg(windows)]
#[test]
fn a_feed_the_account_already_holds_is_not_sent_again_and_never_touches_the_vault() {
    let root = tmp("n1-stored");
    let home = root.join("home");
    let app_data = root.join("appdata");
    let mut session = PendingSession::new("acc-n1-stored");
    // Ruling 11: the token probe's pull, then the restore's.
    let (base, handle) = multi_loopback(vec![(200, EMPTY_PULL_PAGE), (200, EMPTY_PULL_PAGE)]);
    unsafe { std::env::set_var("KNOWLU_API_BASE", &base) };
    let mut plan = base_plan();
    plan.ics_url = Some("https://x.invalid/c.ics".to_string());
    (plan.ics_validated, plan.ics_stored) = (true, true);
    let out = create_vault_in(&app_data, &home, "Fall 2026", &plan);
    assert_eq!(out["ok"], true, "{out}");
    let id = out["profile"]["id"].as_str().expect("a profile id").to_string();
    session.expect_move_to(&id);
    let seen = handle.join().expect("the loopback thread did not panic");
    assert_eq!(seen.len(), 2, "{seen:?}");
    assert!(seen.iter().all(|s| s.starts_with("GET /functions/v1/sync-pull")), "the only calls are the probe's and the restore's pulls, never a second PUT: {seen:?}");
    assert_eq!(out["restored"]["ok"], true, "{out}");
    let ingest = read_ingest(&home);
    assert!(ingest.contains("ics_url: ''\n"), "a link the account holds never reaches the vault: {ingest}");
    let _ = std::fs::remove_dir_all(&root);
}

/// N1 (b): a link the paste REJECTED — the page lets Next through on the shape of the URL alone — is
/// never sent to the account, where it would have replaced a validated one. It is kept in the vault
/// instead, as a vault with no account keeps any link, so it is not lost either.
#[cfg(windows)]
#[test]
fn a_link_that_failed_validation_never_reaches_the_account() {
    let root = tmp("n1-rejected");
    let home = root.join("home");
    let app_data = root.join("appdata");
    let mut session = PendingSession::new("acc-n1-rejected");
    // Ruling 11: the token probe's pull, then the restore's.
    let (base, handle) = multi_loopback(vec![(200, EMPTY_PULL_PAGE), (200, EMPTY_PULL_PAGE)]);
    unsafe { std::env::set_var("KNOWLU_API_BASE", &base) };
    let mut plan = base_plan();
    plan.ics_url = Some("https://x.invalid/rejected.ics".to_string());
    (plan.ics_validated, plan.ics_stored) = (false, false);
    let out = create_vault_in(&app_data, &home, "Fall 2026", &plan);
    assert_eq!(out["ok"], true, "{out}");
    let id = out["profile"]["id"].as_str().expect("a profile id").to_string();
    session.expect_move_to(&id);
    let seen = handle.join().expect("the loopback thread did not panic");
    assert_eq!(seen.len(), 2, "{seen:?}");
    assert!(seen.iter().all(|s| s.starts_with("GET /functions/v1/sync-pull")), "a rejected link is never PUT: {seen:?}");
    let ingest = read_ingest(&home);
    assert!(ingest.contains("ics_url: 'https://x.invalid/rejected.ics'\n"), "{ingest}");
    let _ = std::fs::remove_dir_all(&root);
}

/// N3: the personal calendar's own retry-success path. A validated address the paste could not save
/// is saved at Finish, and the vault keeps its `cloud:personal` routing entry rather than the URL.
#[cfg(windows)]
#[test]
fn a_personal_calendar_that_failed_at_paste_is_saved_by_the_retry_and_stays_out_of_the_vault() {
    let root = tmp("n3-personal-retry");
    let home = root.join("home");
    let app_data = root.join("appdata");
    let mut session = PendingSession::new("acc-n3-personal");
    let (base, handle) = multi_loopback(vec![
        (200, EMPTY_PULL_PAGE), // ruling 11: the token probe, before anything is made
        (200, r#"{"kind":"calendar_ics"}"#),
        (200, EMPTY_PULL_PAGE),
    ]);
    unsafe { std::env::set_var("KNOWLU_API_BASE", &base) };
    let mut plan = base_plan();
    plan.personal_calendar = Some("https://x.invalid/personal.ics".to_string());
    (plan.personal_calendar_validated, plan.personal_calendar_stored) = (true, false);
    let out = create_vault_in(&app_data, &home, "Fall 2026", &plan);
    assert_eq!(out["ok"], true, "{out}");
    let id = out["profile"]["id"].as_str().expect("a profile id").to_string();
    session.expect_move_to(&id);
    let seen = handle.join().expect("the loopback thread did not panic");
    assert!(seen[0].starts_with("GET /functions/v1/sync-pull"), "the token probe comes first: {}", seen[0]);
    assert!(seen[1].starts_with("PUT /functions/v1/account/sources ") && seen[1].contains("\"kind\":\"calendar_ics\""), "{}", seen[1]);
    let ingest = read_ingest(&home);
    assert!(ingest.contains("- name: personal\n    ics_url: 'cloud:personal'\n"), "the account holds it, so the vault routes to it: {ingest}");
    assert!(!ingest.contains("personal.ics"), "the address itself stays out of the vault: {ingest}");
    let _ = std::fs::remove_dir_all(&root);
}

// M2 (fix round 1): the source-text test that used to stand here,
// `h11a_rolls_the_vault_back_when_the_restore_cannot_read_the_account`, is gone. Its own reasoning
// still holds — `sync::restore_into` computes its allowlist from `note_paths(dest)` in the same
// breath it checks a stray note against it, so the `Err` arm `create_vault_in` guards against is not
// reachable through this call by any input a test can hand it — but pinning that guard's SHAPE by
// matching a byte window of `src/onboarding.rs` was exactly the brittle kind of test the review
// named: a reflow breaks it, and a comment that happens to contain the same words satisfies it
// without the code underneath having to agree. The guard itself is unchanged and stays in
// `create_vault_in`; `engine/tests/sync_replay.rs`'s
// `a_restore_refuses_a_note_it_did_not_put_there_and_tolerates_exactly_the_seeds` (Task 9, Step 2)
// is what actually exercises the refusal this guard exists for, against `sync::restore` with a
// hand-picked allowlist narrower than disk — the one shape that can really happen.

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
    let mut plan = base_plan();
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
    let raw = "webcal://x.invalid/y.ics";
    let mut plan = base_plan();
    plan.personal_calendar = Some(raw.to_string());
    // Validated at paste and not stored there, so Finish retries it (N1, R-C3'-exec-41).
    plan.personal_calendar_validated = true;
    let out = create_vault_in(&app_data, &home, "Fall 2026", &plan);
    assert_eq!(out["ok"], true, "{out}");
    let id = out["profile"]["id"].as_str().expect("a profile id").to_string();
    session.expect_move_to(&id);
    // Task 11 review, M2 (fixed with I1's retry-then-fallback landing): `PendingSession::new` points
    // `KNOWLU_API_BASE` at an `AccountStub` that refuses every save with 503, so `create_vault_in`'s
    // own retry of the account save fails as a failing service would, and the fallback writes the rewritten address
    // into the vault — `create_vault_in` itself is what this test now exercises end to end, not
    // `https_from_webcal` called a second time beside it.
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
    let mut plan = base_plan();
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
    let raw = "  https://x.invalid/y.ics  ";
    let mut plan = base_plan();
    plan.personal_calendar = Some(raw.to_string());
    plan.personal_calendar_validated = true;
    let out = create_vault_in(&app_data, &home, "Fall 2026", &plan);
    assert_eq!(out["ok"], true, "{out}");
    let id = out["profile"]["id"].as_str().expect("a profile id").to_string();
    session.expect_move_to(&id);
    // Task 11 review, M2: same fix as the webcal case above — the stub's 503 fails the retry
    // too, so the fallback writes the trimmed address into the vault and `create_vault_in` is what
    // this test exercises, end to end.
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
    let mut plan = base_plan();
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
    assert_eq!(out["note"].as_str(), Some("We could not reach your coursework sites"));
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
    // A row is [unitid, name, city, state, lms] — the web host stays in the asset, unshown and uncrossed.
    // `lms` is the curated kind (`scaffold::CAMPUSES`) or "" for a school nobody has curated: it lets
    // the wizard pre-answer "which LMS does your school use?" (live proof 2026-10-02: a curated
    // Blackboard school was asked Blackboard or Canvas).
    assert_eq!(rows[0].as_array().map(Vec::len), Some(5));
    for r in rows {
        let want = knowlu::scaffold::curated(&r[0].to_string()).map(|c| c.lms_kind).unwrap_or("");
        assert_eq!(r[4].as_str(), Some(want), "{r:?}");
    }
    for c in knowlu::scaffold::CAMPUSES {
        let found = campus_search(c.label.into());
        let row = found["hits"].as_array().and_then(|h| h.iter().find(|r| r[0].to_string() == c.unitid)).unwrap_or_else(|| panic!("{} not found", c.label));
        assert_eq!(row[4].as_str(), Some(c.lms_kind), "a curated school's hit carries its LMS kind");
    }
    let tusc = campus_search("tuscaloosa".into());
    assert!(tusc["hits"].as_array().unwrap().iter().any(|r| r[4].as_str() == Some("")), "an uncurated school's lms is empty, so the wizard still asks");
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
    let mut plan = base_plan();
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
    let mut plan = base_plan();
    plan.courses = vec![
        knowlu::scaffold::CourseSeed { code: "CS 100".into(), name: "CS 100".into(), slug: "../../evil".into(), label: String::new() },
        knowlu::scaffold::CourseSeed { code: "GN 103".into(), name: "GN 103".into(), slug: "  has spaces  ".into(), label: String::new() },
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
    // R-C1-59 (I2): with no restore panel yet, the refusal is the only route a student has today —
    // it must say what to do instead, not just what is wrong.
    assert!(err.contains("Copy this folder"), "the refusal must say what to do instead: {err}");
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

/// T14 (email spec §4.1, D6): the mail offer is its own marker file, set and cleared by the same
/// three finishing paths, from `offer_mail` alone.
#[test]
fn the_mail_offer_marker_is_set_and_cleared_on_create_restore_and_adopt() {
    let mail_marker = |app_data: &Path, id: &str| mail_offer_marker(&knowlu::profiles::profile_dir(app_data, id));
    let plan = |mail: bool| WizardPlan { offer_mail: mail, ..base_plan() };
    // ADOPT: set, then cleared by a second finish with the box unchecked.
    let root = tmp("mail-offer-adopt");
    let vault = root.join("vault");
    copy(Path::new("../engine/tests/fixtures/vault-s1"), &vault);
    let app_data = root.join("appdata");
    let out = adopt_vault_in(&app_data, vault.to_str().unwrap(), None);
    let id = out["profile"]["id"].as_str().expect("a profile id").to_string();
    assert_eq!(apply_profile_settings_in(&app_data, &id, &plan(true), None)["ok"], true);
    assert!(mail_marker(&app_data, &id).is_file(), "adopt, offered");
    assert_eq!(apply_profile_settings_in(&app_data, &id, &plan(false), None)["ok"], true);
    assert!(!mail_marker(&app_data, &id).exists(), "adopt, not offered");
    let _ = std::fs::remove_dir_all(&root);
    // RESTORE.
    for (tag, on) in [("mail-offer-restore-on", true), ("mail-offer-restore-off", false)] {
        let root = tmp(tag);
        let backup = root.join("backup").join("profile_4444444444").join("vault");
        copy(Path::new("../engine/tests/fixtures/vault-s1"), &backup);
        let app_data = root.join("appdata");
        let dest = restore_vault_in(root.join("backup").to_str().unwrap(), &root.join("home"), "Fall 2026").expect("restore");
        let out = finish_or_roll_back(&app_data, &dest, Some("Fall 2026".to_string()), &plan(on), None);
        assert_eq!(out["ok"], true, "{out}");
        let id = out["profile"]["id"].as_str().expect("a profile id").to_string();
        assert_eq!(mail_marker(&app_data, &id).is_file(), on, "restore, offer_mail={on}");
        let _ = std::fs::remove_dir_all(&root);
    }
}

#[cfg(windows)]
#[test]
fn create_vault_sets_the_mail_offer_marker_only_when_offered() {
    for (tag, on) in [("mail-offer-create-on", true), ("mail-offer-create-off", false)] {
        let root = tmp(tag);
        let app_data = root.join("appdata");
        let mut session = PendingSession::new(&format!("acc-marker-mail-{on}"));
        let out = create_vault_in(&app_data, &root.join("home"), "Fall 2026", &WizardPlan { offer_mail: on, ..base_plan() });
        assert_eq!(out["ok"], true, "{out}");
        let id = out["profile"]["id"].as_str().expect("a profile id").to_string();
        session.expect_move_to(&id);
        let m = mail_offer_marker(&knowlu::profiles::profile_dir(&app_data, &id));
        assert_eq!(m.is_file(), on, "create, offer_mail={on}");
        let _ = std::fs::remove_dir_all(&root);
    }
}

// -- O3 F3: step 2 makes the vault with defaults (plan "F3. Step 2: the vault with defaults") ------
//
// `create_vault_defaults_in` is step 2's create: no name, no plan, no school files. These tests need
// the same real pending session every `create_vault_in` test above needs, so they take the same
// `PendingSession` (CREDMAN_LOCK, restore on drop). Neither old sentence ("press Finish again",
// "pick another name") may be reachable from it.
use knowlu::onboarding::{create_vault_defaults_in, pick_vault_name_in, profiles_with_setup_state};
use knowlu::onboarding_state::{load_state, save_state, Loaded, OnboardingState};

/// The vault step 2 made under `home` for `name`.
fn step2_vault(home: &Path, name: &str) -> PathBuf { home.join("Knowlu").join(name) }

/// The pending session again, after a first create moved it onto its profile. Only the entry is
/// rewritten: the `PendingSession` guard that owns it is still alive and still cleans up.
#[cfg(windows)]
fn refill_pending(account_id: &str) {
    let s = knowlu::account::Session {
        access_token: "test-at".into(),
        refresh_token: "test-rt".into(),
        expires_at: jiff::Timestamp::now().as_second() + 3600,
        email: "knowlu-test@example.invalid".into(),
    };
    knowlu::account::save_session(knowlu::account::PENDING_TARGET, account_id, &s).expect("pending session");
}

/// The three files the school's answer creates, after payment (§4.9).
const SCHOOL_FILES: [&str; 3] = ["config/campus.yaml", "config/events.yaml", "config/ingest.yaml"];

#[cfg(windows)]
fn assert_no_school_files(vault: &Path) {
    for f in SCHOOL_FILES {
        assert!(!vault.join(f).exists(), "{f} must not exist after step 2");
    }
}

/// F3 test 1: the defaults, the session move, the unfinished `onboarding.json` and a runner the
/// app's scheduler accepts; and a second and third profile are "Knowlu 2" and "Knowlu 3".
#[cfg(windows)]
#[test]
fn step_two_makes_a_vault_from_the_defaults_and_names_the_next_ones() {
    let root = tmp("f3-defaults");
    let (home, app_data) = (root.join("home"), root.join("appdata"));
    let mut session = PendingSession::new("acc-f3-defaults");
    for (i, name) in ["Knowlu", "Knowlu 2", "Knowlu 3"].into_iter().enumerate() {
        if i > 0 { refill_pending("acc-f3-defaults"); }
        let out = create_vault_defaults_in(&app_data, &home);
        assert_eq!(out["ok"], true, "{out}");
        assert_eq!(out["profile"]["name"], name, "{out}");
        let id = out["profile"]["id"].as_str().expect("a profile id").to_string();
        session.expect_move_to(&id);
        let vault = step2_vault(&home, name);
        assert!(vault.is_dir(), "{name} is made under the default parent");
        if i > 0 { continue; }

        // The zone is Windows', the slots are noon and six, the device is this machine.
        let runners = knowlu_engine::pystr::read_text(&vault.join("config").join("runners.yaml")).expect("runners.yaml");
        let zone = jiff::tz::TimeZone::system().iana_name().unwrap_or("America/Chicago").to_string();
        assert!(runners.contains(&zone), "the runner's tz is Windows' zone {zone}: {runners}");
        assert!(runners.contains("12:00") && runners.contains("18:00"), "{runners}");
        assert!(runners.contains("name: local"), "{runners}");
        assert!(runners.contains("scheduler: app"), "run_slot_with refuses a runner that is not the app's: {runners}");
        assert!(runners.contains(&knowlu_engine::journal::device_name()), "this device is named: {runners}");
        // The vault names the profile's session target; the session is off the pending target.
        let cloud = knowlu_engine::pystr::read_text(&vault.join("config").join("cloud.yaml")).expect("cloud.yaml");
        assert!(cloud.contains(&knowlu::credentials::target_for(&id, "session")), "{cloud}");
        assert!(knowlu::credentials::exists(&knowlu::account::session_target(&id)), "the session moved onto the profile");
        assert!(!knowlu::credentials::exists(knowlu::account::PENDING_TARGET), "and off the pending target");
        // Settings: autostart on.
        let dir = knowlu::profiles::profile_dir(&app_data, &id);
        let (settings, bad) = knowlu::state::Settings::load(&dir.join("settings.json"), &vault);
        assert!(bad.is_none(), "{bad:?}");
        assert!(settings.autostart && settings.profile_id == id);
        assert!(dir.join("settings.json").is_file(), "settings were written, not defaulted");
        // The unfinished onboarding file, at step 3.
        match load_state(&dir) {
            Loaded::Ok(s) => assert!(s.step == 3 && !s.finished, "unfinished at step 3: {s:?}"),
            other => panic!("onboarding.json: {other:?}"),
        }
    }
    let _ = std::fs::remove_dir_all(&root);
}

/// F3 test 2: nothing about a school exists after step 2 (the files wait for `school.apply`).
#[cfg(windows)]
#[test]
fn step_two_writes_no_campus_events_or_ingest_file() {
    let root = tmp("f3-no-school");
    let (home, app_data) = (root.join("home"), root.join("appdata"));
    let mut session = PendingSession::new("acc-f3-no-school");
    let out = create_vault_defaults_in(&app_data, &home);
    assert_eq!(out["ok"], true, "{out}");
    session.expect_move_to(out["profile"]["id"].as_str().expect("a profile id"));
    let vault = step2_vault(&home, "Knowlu");
    assert_no_school_files(&vault);
    // The files that are all defaults are there.
    for f in ["config/planning.yaml", "config/week_template.yaml", "config/actor.yaml", "config/runners.yaml", "config/cloud.yaml", ".gitignore"] {
        assert!(vault.join(f).is_file(), "{f} is made at step 2");
    }
    let _ = std::fs::remove_dir_all(&root);
}

/// F3 test 3: a restore from the account's copy goes through the same call, writes none of the three
/// school files (config never syncs), and `onboarding.json` lands at the step §4.7 derives: 3, since
/// a restored vault has no `campus.yaml` and the account never says which school.
#[cfg(windows)]
#[test]
fn step_two_restore_writes_no_school_files_and_lands_at_the_derived_step() {
    let root = tmp("f3-restore");
    let (home, app_data) = (root.join("home"), root.join("appdata"));
    let mut session = PendingSession::new("acc-f3-restore");
    let device = "aaaaaaaaaaaaaaaa";
    let note_text = "---\nid: task_0000000098\n---\nfrom the account\n";
    let record = serde_json::json!({
        "op": "create", "path": "tasks/from-account.md", "actor": "student", "via": "dashboard",
        "device": device, "ts": "2026-08-01T10:00:00.000Z", "id": "task_0000000098",
        "new": { "id": "task_0000000098" }
    });
    let record_body = knowlu_engine::ledger::dumps_value(&record);
    let page = serde_json::json!({
        "records": [{ "seq": 1, "device": device, "record_hash": knowlu_engine::sync::sha256_hex(record_body.as_bytes()), "body": record_body }],
        "notes": [{ "path": "tasks/from-account.md", "device": device, "deleted": false, "body": note_text }],
        "record_cursor": 1, "note_cursor": 1, "more": false
    });
    let (base, handle) = restore_loopback(&knowlu_engine::ledger::dumps_value(&page));
    unsafe { std::env::set_var("KNOWLU_API_BASE", &base) };
    let out = create_vault_defaults_in(&app_data, &home);
    assert_eq!(out["ok"], true, "{out}");
    let id = out["profile"]["id"].as_str().expect("a profile id").to_string();
    session.expect_move_to(&id);
    assert_eq!(out["restored"]["notes"], 1, "{out}");
    let vault = step2_vault(&home, "Knowlu");
    assert_eq!(knowlu_engine::pystr::read_text(&vault.join("tasks").join("from-account.md")).expect("the restored note"), note_text);
    assert_no_school_files(&vault);
    match load_state(&knowlu::profiles::profile_dir(&app_data, &id)) {
        Loaded::Ok(s) => assert!(s.step == 3 && !s.finished && s.account_settled, "{s:?}"),
        other => panic!("onboarding.json: {other:?}"),
    }
    handle.join().expect("the loopback thread did not panic");
    let _ = std::fs::remove_dir_all(&root);
}

/// F3 test 4: each profile row carries `setup_unfinished`, from its own `onboarding.json`: true for
/// an unfinished file, false for a finished one and for none.
#[test]
fn launch_state_profiles_carry_setup_unfinished() {
    let root = tmp("f3-unfinished");
    let make = |name: &str| {
        let v = root.join("vaults").join(name);
        std::fs::create_dir_all(&v).unwrap();
        knowlu::profiles::register(&root, name, &v).unwrap()
    };
    let (open, done, none) = (make("open"), make("done"), make("none"));
    let mut st = OnboardingState::new("2026-10-24T00:00:00.000Z", 3);
    save_state(&knowlu::profiles::profile_dir(&root, &open.id), &st).unwrap();
    st.finished = true;
    save_state(&knowlu::profiles::profile_dir(&root, &done.id), &st).unwrap();
    let all = vec![open.clone(), done.clone(), none.clone()];
    let rows = profiles_with_setup_state(&root, &all);
    let rows = rows.as_array().expect("an array of profiles");
    assert_eq!(rows.len(), 3, "{rows:?}");
    let flag = |p: &knowlu::profiles::Profile| {
        let row = rows.iter().find(|r| r["id"] == p.id.as_str()).expect("the profile is listed");
        assert_eq!(row["name"], p.name.as_str(), "the row keeps the profile's own fields");
        row["setup_unfinished"].clone()
    };
    assert_eq!(flag(&open), true);
    assert_eq!(flag(&done), false);
    assert_eq!(flag(&none), false);
    let _ = std::fs::remove_dir_all(&root);
}

/// F3 test 5, reached: an account that answered (here an empty page) settles, whether or not it held
/// anything. The file says so.
#[cfg(windows)]
#[test]
fn account_settled_is_true_when_the_account_was_reached() {
    let root = tmp("f3-settled");
    let (home, app_data) = (root.join("home"), root.join("appdata"));
    let mut session = PendingSession::new("acc-f3-settled");
    let (base, handle) = restore_loopback(EMPTY_PULL_PAGE);
    unsafe { std::env::set_var("KNOWLU_API_BASE", &base) };
    let out = create_vault_defaults_in(&app_data, &home);
    assert_eq!(out["ok"], true, "{out}");
    assert_eq!(out["restored"]["ok"], true, "{out}");
    let id = out["profile"]["id"].as_str().expect("a profile id").to_string();
    session.expect_move_to(&id);
    match load_state(&knowlu::profiles::profile_dir(&app_data, &id)) {
        Loaded::Ok(s) => assert!(s.account_settled, "{s:?}"),
        other => panic!("onboarding.json: {other:?}"),
    }
    handle.join().expect("the loopback thread did not panic");
    let _ = std::fs::remove_dir_all(&root);
}

/// F3 test 5, refused: the token probe is answered, then the restore's connection is refused (the
/// listener is gone). The vault is kept, as today, and `account_settled` stays false so the courses
/// wait. (A base that refuses from the first request would stop at the probe, before any vault
/// exists; that is test 7's case.)
#[cfg(windows)]
#[test]
fn account_settled_is_false_when_the_restore_could_not_reach_the_account() {
    let root = tmp("f3-unsettled");
    let (home, app_data) = (root.join("home"), root.join("appdata"));
    let mut session = PendingSession::new("acc-f3-unsettled");
    let (base, handle) = multi_loopback(vec![(200, EMPTY_PULL_PAGE)]);
    unsafe { std::env::set_var("KNOWLU_API_BASE", &base) };
    let out = create_vault_defaults_in(&app_data, &home);
    handle.join().expect("the probe was served once");
    assert_eq!(out["ok"], true, "the vault is kept: {out}");
    assert_eq!(out["restored"]["ok"], false, "{out}");
    let id = out["profile"]["id"].as_str().expect("a profile id").to_string();
    session.expect_move_to(&id);
    assert!(step2_vault(&home, "Knowlu").is_dir(), "the vault is kept");
    match load_state(&knowlu::profiles::profile_dir(&app_data, &id)) {
        Loaded::Ok(s) => assert!(!s.account_settled && s.step == 3, "{s:?}"),
        other => panic!("onboarding.json: {other:?}"),
    }
    let _ = std::fs::remove_dir_all(&root);
}

/// The folders directly under `<home>\Knowlu` that are not the backups root, sorted.
fn vault_folders(home: &Path) -> Vec<String> {
    let mut v: Vec<String> = std::fs::read_dir(home.join("Knowlu")).map(|rd| {
        rd.flatten().map(|e| e.file_name().to_string_lossy().to_string()).filter(|n| n != "Backups").collect()
    }).unwrap_or_default();
    v.sort();
    v
}

/// F3 test 6: the name is Rust's. It steps past a registered profile's name and past a folder that
/// is on disk with no profile, and it writes into and renames nothing.
#[test]
fn the_vault_name_steps_past_registered_profiles_and_folders_on_disk() {
    let register = |root: &Path, name: &str| {
        let v = root.join("elsewhere").join(name);
        std::fs::create_dir_all(&v).unwrap();
        knowlu::profiles::register(root, name, &v).unwrap();
    };
    let folder = |home: &Path, name: &str| {
        let d = home.join("Knowlu").join(name);
        std::fs::create_dir_all(&d).unwrap();
        std::fs::write(d.join("keep.txt"), b"mine").unwrap();
    };

    // Nothing yet: "Knowlu".
    let root = tmp("f3-name-none");
    assert_eq!(pick_vault_name_in(&root, &root.join("home")).unwrap(), "Knowlu");

    // A registered "Knowlu": "Knowlu 2".
    let root = tmp("f3-name-registered");
    register(&root, "Knowlu");
    assert_eq!(pick_vault_name_in(&root, &root.join("home")).unwrap(), "Knowlu 2");

    // An unregistered Knowlu folder on disk: "Knowlu 2", the folder untouched.
    let root = tmp("f3-name-folder");
    let home = root.join("home");
    folder(&home, "Knowlu");
    let (before, names) = (fingerprint(&home), vault_folders(&home));
    assert_eq!(pick_vault_name_in(&root, &home).unwrap(), "Knowlu 2");
    assert_eq!(fingerprint(&home), before, "nothing is written into the existing folder");
    assert_eq!(vault_folders(&home), names, "no folder is made or renamed");

    // "Knowlu" and a "Knowlu 2" folder: "Knowlu 3".
    let root = tmp("f3-name-two-folders");
    let home = root.join("home");
    folder(&home, "Knowlu");
    folder(&home, "Knowlu 2");
    let (before, names) = (fingerprint(&home), vault_folders(&home));
    assert_eq!(pick_vault_name_in(&root, &home).unwrap(), "Knowlu 3");
    assert_eq!(fingerprint(&home), before);
    assert_eq!(vault_folders(&home), names);

    // A registered "Knowlu" and a leftover "Knowlu 2" folder with no profile: "Knowlu 3".
    let root = tmp("f3-name-mixed");
    let home = root.join("home");
    register(&root, "Knowlu");
    folder(&home, "Knowlu 2");
    assert_eq!(pick_vault_name_in(&root, &home).unwrap(), "Knowlu 3");
    assert_eq!(vault_folders(&home), vec!["Knowlu 2".to_string()]);
}

/// F3 test 6, end to end: a leftover "Knowlu" folder is never the destination, so `dest_for`'s
/// "already exists" refusal is never reached and the leftover keeps its bytes.
#[cfg(windows)]
#[test]
fn step_two_with_a_leftover_folder_makes_the_next_name_and_leaves_it_alone() {
    let root = tmp("f3-leftover");
    let (home, app_data) = (root.join("home"), root.join("appdata"));
    let left = step2_vault(&home, "Knowlu");
    std::fs::create_dir_all(&left).unwrap();
    std::fs::write(left.join("keep.txt"), b"mine").unwrap();
    let before = fingerprint(&left);
    let mut session = PendingSession::new("acc-f3-leftover");
    let out = create_vault_defaults_in(&app_data, &home);
    assert_eq!(out["ok"], true, "{out}");
    session.expect_move_to(out["profile"]["id"].as_str().expect("a profile id"));
    assert_eq!(out["profile"]["name"], "Knowlu 2", "{out}");
    assert!(step2_vault(&home, "Knowlu 2").join("config").join("planning.yaml").is_file());
    assert_eq!(fingerprint(&left), before, "the leftover folder is untouched");
    let _ = std::fs::remove_dir_all(&root);
}

/// The two sentences of the old Finish panel, which have no step to point to on step 2.
#[cfg(windows)]
fn assert_no_old_sentence(out: &serde_json::Value) {
    let e = out["error"].as_str().unwrap_or_default().to_lowercase();
    assert!(!e.contains("press finish again"), "step 2 has no Finish: {e}");
    assert!(!e.contains("pick another name"), "step 2 has no name field: {e}");
}

/// F3 test 7: an account that cannot be reached for the token check refuses step 2 with step 2's own
/// retry ("Try again"), makes nothing, and keeps the sign-in for that retry. Today's create says
/// "press Finish again" for exactly this.
#[cfg(windows)]
#[test]
fn step_two_failures_say_try_again_never_the_old_finish_or_name_sentences() {
    let root = tmp("f3-errors");
    let (home, app_data) = (root.join("home"), root.join("appdata"));
    let session = PendingSession::new("acc-f3-errors");
    unsafe { std::env::set_var("KNOWLU_API_BASE", closed_loopback_base()) };
    let out = create_vault_defaults_in(&app_data, &home);
    assert_eq!(out["ok"], false, "{out}");
    assert_no_old_sentence(&out);
    assert!(out["error"].as_str().unwrap_or_default().contains("Try again"), "a named error with step 2's retry: {out}");
    assert!(!step2_vault(&home, "Knowlu").exists(), "nothing is made");
    assert!(knowlu::credentials::exists(knowlu::account::PENDING_TARGET), "the sign-in stays for the retry");

    // No pending session at all: also a named error, never the old sentences.
    let _ = knowlu::credentials::delete(knowlu::account::PENDING_TARGET);
    let out = create_vault_defaults_in(&app_data, &home);
    assert_eq!(out["ok"], false, "{out}");
    assert_no_old_sentence(&out);
    assert!(!out["error"].as_str().unwrap_or_default().is_empty(), "{out}");
    drop(session);
    let _ = std::fs::remove_dir_all(&root);
}

/// F3 review r1, 1: a profile renamed and its folder moved still owns the id of `<home>\Knowlu\Knowlu`.
/// The name is free and no folder is there, yet the candidate is passed: reusing it would overwrite that
/// profile's session and settings. So is a candidate whose app-data folder is there with no registration.
#[test]
fn the_vault_name_steps_past_a_candidate_that_a_renamed_profile_or_its_folder_still_owns() {
    let root = tmp("f3-name-renamed");
    let home = root.join("home");
    let would_be = home.join("Knowlu").join("Knowlu");
    knowlu::profiles::register(&root, "Biology", &would_be).unwrap();
    assert!(!would_be.exists(), "no folder is there");
    assert_eq!(pick_vault_name_in(&root, &home).unwrap(), "Knowlu 2");

    let root = tmp("f3-name-orphan-profile-dir");
    let home = root.join("home");
    let id = knowlu::profiles::id_for(&home.join("Knowlu").join("Knowlu"));
    std::fs::create_dir_all(knowlu::profiles::profile_dir(&root, &id)).unwrap();
    assert_eq!(pick_vault_name_in(&root, &home).unwrap(), "Knowlu 2");
}

/// What a failed step 2 leaves: no vault, no profile folder, no session at the profile's target, and the
/// sign-in back where a retry finds it. The error is step 2's own.
#[cfg(windows)]
fn assert_step_two_undone(out: &serde_json::Value, home: &Path, app_data: &Path, session: &mut PendingSession) {
    assert_eq!(out["ok"], false, "{out}");
    assert_no_old_sentence(out);
    let error = out["error"].as_str().unwrap_or_default();
    assert!(error.contains("Try again") && !error.contains("could not be"), "named, and nothing left undone: {out}");
    let vault = step2_vault(home, "Knowlu");
    let id = knowlu::profiles::id_for(&vault);
    session.expect_move_to(&id);
    assert!(!vault.exists(), "the vault is gone");
    assert!(!knowlu::profiles::profile_dir(app_data, &id).exists(), "the profile folder this call made is gone");
    assert!(!knowlu::credentials::exists(&knowlu::account::session_target(&id)), "nothing at the profile's session target");
    assert!(knowlu::credentials::exists(knowlu::account::PENDING_TARGET), "the sign-in is back for the retry");
    assert!(knowlu::profiles::load(app_data).unwrap().is_empty(), "no profile is registered");
}

/// F3 review r1, 2: a restore that fails (a seam: a real one refuses only a vault that is not fresh).
#[cfg(windows)]
#[test]
fn step_two_undoes_everything_when_the_restore_fails() {
    let root = tmp("f3-undo-restore");
    let (home, app_data) = (root.join("home"), root.join("appdata"));
    let mut session = PendingSession::new("acc-f3-undo-restore");
    let out = knowlu::onboarding::create_vault_defaults_with(&app_data, &home, &|_| Err("the copy will not read".to_string()));
    assert!(out["error"].as_str().unwrap_or_default().contains("the copy will not read"), "{out}");
    assert_step_two_undone(&out, &home, &app_data, &mut session);
    let _ = std::fs::remove_dir_all(&root);
}

/// F3 review r1, 2: `onboarding.json` cannot be saved (a file stands where the `profiles` folder goes).
#[cfg(windows)]
#[test]
fn step_two_undoes_everything_when_the_setup_file_cannot_be_saved() {
    let root = tmp("f3-undo-save");
    let (home, app_data) = (root.join("home"), root.join("appdata"));
    std::fs::create_dir_all(&app_data).unwrap();
    std::fs::write(app_data.join("profiles"), b"a file, not a folder").unwrap();
    let mut session = PendingSession::new("acc-f3-undo-save");
    let out = create_vault_defaults_in(&app_data, &home);
    assert!(out["error"].as_str().unwrap_or_default().contains("setup progress"), "{out}");
    assert_eq!(out["ok"], false);
    let vault = step2_vault(&home, "Knowlu");
    let id = knowlu::profiles::id_for(&vault);
    session.expect_move_to(&id);
    assert!(!vault.exists() && !knowlu::credentials::exists(&knowlu::account::session_target(&id)));
    assert!(knowlu::credentials::exists(knowlu::account::PENDING_TARGET), "the sign-in is back for the retry");
    assert_no_old_sentence(&out);
    assert!(out["error"].as_str().unwrap_or_default().contains("Try again") && !out["error"].as_str().unwrap_or_default().contains("could not be put back"), "{out}");
    assert!(app_data.join("profiles").is_file(), "the file that was there is not touched");
    let _ = std::fs::remove_dir_all(&root);
}

/// F3 review r1, 2: the finish fails at the registry (a read-only `profiles.json` cannot be replaced).
#[cfg(windows)]
#[test]
fn step_two_undoes_everything_when_the_profile_cannot_be_registered() {
    let root = tmp("f3-undo-finish");
    let (home, app_data) = (root.join("home"), root.join("appdata"));
    std::fs::create_dir_all(&app_data).unwrap();
    let registry = knowlu::profiles::registry_path(&app_data);
    std::fs::write(&registry, b"[]").unwrap();
    let mut perms = std::fs::metadata(&registry).unwrap().permissions();
    perms.set_readonly(true);
    std::fs::set_permissions(&registry, perms).unwrap();
    let mut session = PendingSession::new("acc-f3-undo-finish");
    let out = create_vault_defaults_in(&app_data, &home);
    let mut perms = std::fs::metadata(&registry).unwrap().permissions();
    #[allow(clippy::permissions_set_readonly_false)]
    perms.set_readonly(false);
    std::fs::set_permissions(&registry, perms).unwrap();
    assert!(out["error"].as_str().unwrap_or_default().contains("nothing is half-made"), "it failed at the finish, not before: {out}");
    assert_step_two_undone(&out, &home, &app_data, &mut session);
    let _ = std::fs::remove_dir_all(&root);
}

// ---- O3 F6a: `school.apply` and `school.courses` create each file once (plan F6a; §4.9) ------------
// No Credential Manager: a vault from `create_vault_defaults`, a scratch app-data dir for the answers
// file, invented schools and courses only.
use knowlu::onboarding_jobs::{run_school_apply, run_school_apply_with, run_school_courses_in, Outcome, KEPT_EXISTING_KEY};
use knowlu::onboarding_state::{load_answers, update_answers, CourseAnswer};
use knowlu::scaffold::{campus_yaml, events_preset_for, put_new_file, put_new_file_with, CampusChoice, Put, PutSeams, VaultPlan, TEMP_PREFIX};

fn f6a_plan(dest: &Path, campus: &str, choice: &CampusChoice) -> VaultPlan {
    VaultPlan {
        profile_id: knowlu::profiles::id_for(dest),
        ics_url: None,
        personal_calendar: None,
        google_calendar: false,
        timezone: "America/Chicago".into(),
        slots: vec!["12:00".into(), "18:00".into()],
        device: "M".into(),
        campus: campus.into(),
        campus_choice: choice.clone(),
        zybooks: false,
        vhl: false,
        zybooks_courses: Vec::new(),
        vhl_sections: Vec::new(),
        zybooks_ignore: Vec::new(),
        course_map: Vec::new(),
        courses: Vec::new(),
        api_base: knowlu::account::api_base(),
        anon_key: "anon".into(),
        account_id: "acc-1".into(),
        human_actor: knowlu_engine::journal::HUMAN_ACTOR,
    }
}

/// A step-2 vault (no school files) under a fresh scratch root, and a sibling app-data dir.
fn f6a_vault(tag: &str) -> (PathBuf, PathBuf, PathBuf) {
    let root = tmp(&format!("f6a-{tag}"));
    let vault = root.join("vault");
    knowlu::scaffold::create_vault_defaults(&vault, &f6a_plan(&vault, "none", &CampusChoice::default())).unwrap();
    let data = root.join("appdata");
    std::fs::create_dir_all(&data).unwrap();
    (root, vault, data)
}

fn school(unitid: &str, name: &str, state: &str, lms: &str) -> CampusChoice {
    CampusChoice { unitid: unitid.into(), name: name.into(), state: state.into(), lms: lms.into() }
}

fn f6a_course(code: &str, name: &str, key: &str) -> CourseAnswer {
    let label: String = name.split(' ').next().unwrap().into();
    CourseAnswer { code: code.into(), name: name.into(), captured_label: label.clone(), label, lms_key: key.into(), removed: false, slug: String::new(), fragments: Vec::new() }
}

fn config_bytes(vault: &Path, file: &str) -> Vec<u8> { std::fs::read(vault.join("config").join(file)).unwrap() }

fn strays_in_config(vault: &Path) -> Vec<String> {
    std::fs::read_dir(vault.join("config")).unwrap().flatten()
        .map(|e| e.file_name().to_string_lossy().to_string()).filter(|n| n.starts_with(TEMP_PREFIX)).collect()
}

fn proof_of(out: &Outcome) -> std::collections::BTreeMap<String, Option<i64>> {
    match out { Outcome::Done(p) => p.clone(), other => panic!("expected Done, got {other:?}") }
}

/// F6a test 1: `school.apply`'s two files are byte for byte what `build_into` writes for the same
/// school (the same choice sent through `create_vault` and through the job); an uncurated school and a
/// school typed as "isn't listed" (a name, no unitid) get events preset `none`; the latter's
/// `campus.yaml` says `curated: false`.
#[test]
fn school_apply_writes_the_bytes_build_into_writes_for_the_same_school() {
    let cases = [
        ("a-curated-school", school("100751", "The University of Alabama", "AL", "blackboard")),
        ("an-uncurated-school", school("999001", "Example State College", "OH", "canvas")),
        ("a-school-that-isnt-listed", school("", "St. Mary's Example College", "", "")),
    ];
    for (tag, choice) in cases {
        let (root, vault, _data) = f6a_vault(tag);
        let preset = events_preset_for(&choice.unitid);
        let reference = root.join("reference");
        knowlu::scaffold::create_vault(&reference, &f6a_plan(&reference, preset, &choice)).unwrap();
        let out = run_school_apply(&vault, &choice);
        assert!(matches!(out, Outcome::Done(_)), "{tag}: {out:?}");
        for f in ["campus.yaml", "events.yaml"] {
            assert_eq!(config_bytes(&vault, f), config_bytes(&reference, f), "{tag}: config/{f} differs from build_into's");
        }
        if choice.unitid != "100751" {
            assert_eq!(preset, "none", "{tag}");
            // Vault text is written CRLF on Windows (pystr::write_text), as build_into's is.
            assert_eq!(config_bytes(&vault, "events.yaml"), knowlu_engine::pystr::universal_newlines(campus_yaml("none").unwrap()).replace('\n', knowlu_engine::pystr::NEWLINE).as_bytes(), "{tag}: events preset none");
        }
        if choice.unitid.is_empty() {
            let text = String::from_utf8(config_bytes(&vault, "campus.yaml")).unwrap();
            assert!(text.contains("curated: false"), "{tag}: {text}");
        }
        assert!(strays_in_config(&vault).is_empty(), "{tag}: no temp file is left behind");
        let _ = std::fs::remove_dir_all(&root);
    }
}

/// F6a test 3: a file already there with other bytes is kept unchanged and the job reports "kept your
/// existing settings"; with equal bytes the job is simply `done` (a resume).
#[test]
fn school_apply_keeps_a_file_with_other_bytes_and_a_resume_is_simply_done() {
    let (root, vault, _data) = f6a_vault("keeps");
    let choice = school("999001", "Example State College", "OH", "canvas");
    std::fs::write(vault.join("config").join("campus.yaml"), b"name: my own settings\n").unwrap();
    let out = run_school_apply(&vault, &choice);
    let proof = proof_of(&out);
    assert_eq!(config_bytes(&vault, "campus.yaml"), b"name: my own settings\n", "an existing file is never replaced");
    assert_eq!(proof.get(KEPT_EXISTING_KEY).copied().flatten(), Some(1), "reports it kept your existing settings: {proof:?}");
    assert!(vault.join("config").join("events.yaml").exists(), "the file that was absent is still created");
    // A resume: both files now hold what this school would write, so nothing is "kept".
    std::fs::remove_file(vault.join("config").join("campus.yaml")).unwrap();
    assert!(matches!(run_school_apply(&vault, &choice), Outcome::Done(_)));
    let (before, ev_before) = (config_bytes(&vault, "campus.yaml"), config_bytes(&vault, "events.yaml"));
    let again = proof_of(&run_school_apply(&vault, &choice));
    assert_eq!(again.get(KEPT_EXISTING_KEY).copied().flatten().unwrap_or(0), 0, "equal bytes are a resume, not a kept file: {again:?}");
    assert_eq!((config_bytes(&vault, "campus.yaml"), config_bytes(&vault, "events.yaml")), (before, ev_before));
    let _ = std::fs::remove_dir_all(&root);
}

/// F6a test 4: a file created between the existence check and the put-in-place (the seam creates it
/// once the temp file is written) is kept, the helper reports it existing, and nothing is replaced.
#[test]
fn a_file_created_mid_way_is_kept_and_reported_existing() {
    let (root, vault, _data) = f6a_vault("midway");
    let final_path = vault.join("config").join("campus.yaml");
    let racer = |p: &Path| -> Result<(), String> { std::fs::write(p, b"somebody else's bytes\n").map_err(|e| e.to_string()) };
    let seams = PutSeams { after_temp: Some(&racer), ..Default::default() };
    let put = put_new_file_with(&vault, "config/campus.yaml", "name: ours\n", &seams).unwrap();
    assert_eq!(put, Put::Existing { same_bytes: false });
    assert_eq!(std::fs::read(&final_path).unwrap(), b"somebody else's bytes\n", "never replaced");
    assert!(strays_in_config(&vault).is_empty(), "the temp file is removed");
    // The same race, but the other writer wrote the very bytes we meant to: still existing, same bytes.
    std::fs::remove_file(&final_path).unwrap();
    let twin = |p: &Path| -> Result<(), String> { std::fs::write(p, b"name: ours\n").map_err(|e| e.to_string()) };
    let seams = PutSeams { after_temp: Some(&twin), ..Default::default() };
    assert_eq!(put_new_file_with(&vault, "config/campus.yaml", "name: ours\n", &seams).unwrap(), Put::Existing { same_bytes: true });
    // And with nothing in the way the real helper creates it, once.
    std::fs::remove_file(&final_path).unwrap();
    assert_eq!(put_new_file(&vault, "config/campus.yaml", "name: ours\n").unwrap(), Put::Created);
    assert_eq!(put_new_file(&vault, "config/campus.yaml", "name: other\n").unwrap(), Put::Existing { same_bytes: false });
    assert_eq!(std::fs::read(&final_path).unwrap(), b"name: ours\n");
    let _ = std::fs::remove_dir_all(&root);
}

/// F6a test 5: a failure part-way leaves no partial file at the final name, and a stray temp file (a
/// crash's) is removed on the next run.
#[test]
fn a_failure_part_way_leaves_no_partial_file_and_a_stray_temp_is_swept_next_run() {
    let (root, vault, _data) = f6a_vault("partial");
    let final_path = vault.join("config").join("events.yaml");
    let boom = |_: &Path| -> Result<(), String> { Err("the disk went away".to_string()) };
    let seams = PutSeams { after_temp: Some(&boom), ..Default::default() };
    let err = put_new_file_with(&vault, "config/events.yaml", "preset: none\n", &seams).unwrap_err();
    assert!(err.contains("the disk went away"), "{err}");
    assert!(!final_path.exists(), "no partial file at the final name");
    assert!(strays_in_config(&vault).is_empty(), "the failed run cleans its own temp file");
    // A crash's leftover: the next run removes it, whatever file it was heading for.
    let stray = vault.join("config").join(format!("{TEMP_PREFIX}events.yaml-left-by-a-crash"));
    std::fs::write(&stray, b"half a fi").unwrap();
    assert_eq!(put_new_file(&vault, "config/events.yaml", "preset: none\n").unwrap(), Put::Created);
    assert!(!stray.exists(), "the stray temp file is removed on the next run");
    assert_eq!(std::fs::read(&final_path).unwrap(), b"preset: none\n");
    let _ = std::fs::remove_dir_all(&root);
}

fn not_supported() -> std::io::Error { std::io::Error::new(std::io::ErrorKind::Unsupported, "hard links are not supported here") }

/// F6a test 6, first two bullets: with the hard link forced to fail "not supported", the run-time
/// fallback (a no-replace move) puts the file in place with the same bytes; with a file already at the
/// final name the fallback refuses too, and the file is kept.
#[test]
fn when_hard_links_are_not_supported_the_fallback_puts_the_file_in_place_and_never_replaces() {
    let (root, vault, _data) = f6a_vault("fallback");
    let final_path = vault.join("config").join("campus.yaml");
    let no_link = |_: &Path, _: &Path| -> std::io::Result<()> { Err(not_supported()) };
    let seams = PutSeams { hard_link: Some(&no_link), ..Default::default() };
    assert_eq!(put_new_file_with(&vault, "config/campus.yaml", "name: ours\n", &seams).unwrap(), Put::Created);
    assert_eq!(std::fs::read(&final_path).unwrap(), b"name: ours\n", "same bytes through the fallback");
    assert!(strays_in_config(&vault).is_empty(), "the fallback leaves no temp file");
    // A file at the final name: the fallback must refuse (no MOVEFILE_REPLACE_EXISTING) and keep it.
    std::fs::write(&final_path, b"kept bytes\n").unwrap();
    let put = put_new_file_with(&vault, "config/campus.yaml", "name: ours\n", &seams).unwrap();
    assert_eq!(put, Put::Existing { same_bytes: false });
    assert_eq!(std::fs::read(&final_path).unwrap(), b"kept bytes\n", "the fallback never replaces");
    // …and when the file shows up only after the temp file is written, the fallback refuses then too.
    std::fs::remove_file(&final_path).unwrap();
    let racer = |p: &Path| -> Result<(), String> { std::fs::write(p, b"raced in\n").map_err(|e| e.to_string()) };
    let seams = PutSeams { hard_link: Some(&no_link), after_temp: Some(&racer), ..Default::default() };
    assert_eq!(put_new_file_with(&vault, "config/campus.yaml", "name: ours\n", &seams).unwrap(), Put::Existing { same_bytes: false });
    assert_eq!(std::fs::read(&final_path).unwrap(), b"raced in\n");
    assert!(strays_in_config(&vault).is_empty());
    let _ = std::fs::remove_dir_all(&root);
}

/// F6a test 6, third bullet: both steps forced to fail, `school.apply` is `failed` with a fix that
/// names the volume, and nothing is left at the final name.
#[test]
fn with_both_steps_failing_school_apply_fails_with_a_fix_that_names_the_volume() {
    let (root, vault, _data) = f6a_vault("both-fail");
    let no_link = |_: &Path, _: &Path| -> std::io::Result<()> { Err(not_supported()) };
    let no_move = |_: &Path, _: &Path| -> std::io::Result<()> { Err(std::io::Error::new(std::io::ErrorKind::PermissionDenied, "the volume refused")) };
    let seams = PutSeams { hard_link: Some(&no_link), move_no_replace: Some(&no_move), ..Default::default() };
    let out = run_school_apply_with(&vault, &school("999001", "Example State College", "OH", "canvas"), &seams);
    let Outcome::Failed(fix) = out else { panic!("both steps failing must fail the job, got {out:?}") };
    let volume = match vault.components().next() { Some(std::path::Component::Prefix(p)) => p.as_os_str().to_string_lossy().to_string(), other => panic!("{other:?}") };
    assert!(fix.contains(&volume), "the fix names the volume {volume}: {fix}");
    assert!(!vault.join("config").join("campus.yaml").exists(), "nothing at the final name");
    assert!(strays_in_config(&vault).is_empty());
    let _ = std::fs::remove_dir_all(&root);
}

fn journal_for(vault: &Path) -> Vec<serde_json::Value> {
    let dir = vault.join("state").join("journal");
    let mut files: Vec<PathBuf> = std::fs::read_dir(dir).map(|d| d.flatten().map(|e| e.path()).collect()).unwrap_or_default();
    files.sort();
    files.iter().flat_map(|p| std::fs::read_to_string(p).unwrap().replace("\r\n", "\n").lines()
        .filter(|l| !l.trim().is_empty()).map(|l| serde_json::from_str::<serde_json::Value>(l).unwrap()).collect::<Vec<_>>()).collect()
}

fn course_files(vault: &Path) -> Vec<String> {
    let mut v: Vec<String> = std::fs::read_dir(vault.join("courses")).unwrap().flatten()
        .map(|e| e.file_name().to_string_lossy().to_string()).collect();
    v.sort();
    v
}

/// F6a test 2: `school.courses` makes a note for each kept course and none for a removed row (R-C1-51),
/// as the student through `write`, so the journal holds a `create` record for each, and the answers
/// file records each kept course's slug by its key.
#[test]
fn school_courses_makes_notes_for_kept_courses_only_as_the_student() {
    let (root, vault, data) = f6a_vault("courses");
    let mut removed = f6a_course("WVG 105", "Weaving Lab", "_1003_1");
    removed.removed = true;
    let rows = vec![f6a_course("QLT 101", "Quilting Basics", "_1001_1"), f6a_course("KNT 220", "Knotwork Theory", "_1002_1"), removed];
    update_answers(&data, &|a| a.courses = rows.clone()).unwrap();
    let before = course_files(&vault);
    let out = run_school_courses_in(&data, &vault);
    assert!(matches!(out, Outcome::Done(_)), "{out:?}");
    let slugs = load_answers(&data).unwrap().unwrap().slugs;
    assert_eq!(slugs.len(), 2, "one slug per kept course, none for the removed row: {slugs:?}");
    assert!(!slugs.contains_key("_1003_1"));
    let after = course_files(&vault);
    assert_eq!(after.len(), before.len() + 2, "{after:?}");
    for key in ["_1001_1", "_1002_1"] {
        let slug = &slugs[key];
        let note = vault.join("courses").join(format!("{slug}.md"));
        assert!(note.exists(), "courses/{slug}.md");
        let rel = format!("courses/{slug}.md");
        let rec = journal_for(&vault).into_iter().find(|r| r["op"] == "create" && r["path"].as_str().is_some_and(|p| p.replace(std::path::MAIN_SEPARATOR, "/").ends_with(&rel)))
            .unwrap_or_else(|| panic!("a journal `create` record for {rel}"));
        assert_eq!(rec["actor"], "student", "{rec}");
        assert_eq!(rec["via"], "dashboard", "console_ctx(vault): {rec}");
    }
    let all: String = after.iter().map(|f| std::fs::read_to_string(vault.join("courses").join(f)).unwrap()).collect();
    assert!(!all.contains("Weaving Lab") && !all.contains("_1003_1"), "nothing of the removed row reaches the vault");
    // A rerun makes no second note for a course that has one.
    assert!(matches!(run_school_courses_in(&data, &vault), Outcome::Done(_)));
    assert_eq!(course_files(&vault), after, "no second note on a rerun");
    assert_eq!(load_answers(&data).unwrap().unwrap().slugs, slugs);
    let _ = std::fs::remove_dir_all(&root);
}

/// F6a test 7 (review r2, I5): a course note already present (a C3' restore) is not touched, no second
/// one is made for it, whether it matches by path, `source_uid` or code, and the answers file's slug
/// for that course is the restored note's own.
#[test]
fn a_restored_course_note_is_not_touched_and_its_own_slug_is_recorded() {
    let (root, vault, data) = f6a_vault("restored");
    let note = |extra: &str| format!("---\ntitle: Restored\n{extra}status: active\n---\n\nmy own notes\n");
    let restored = [
        ("path-kept.md", note("")),
        ("by-uid-kept.md", note("source_uid: _1002_1\n")),
        ("by-code-kept.md", note("code: _1003_1\n")),
    ];
    for (f, text) in &restored { std::fs::write(vault.join("courses").join(f), text).unwrap(); }
    let mut by_path = f6a_course("QLT 101", "Quilting Basics", "_1001_1");
    by_path.slug = "path-kept".into();
    let rows = vec![by_path, f6a_course("KNT 220", "Knotwork Theory", "_1002_1"), f6a_course("WVG 105", "Weaving Lab", "_1003_1"), f6a_course("DYE 110", "Dyeing Basics", "_1004_1")];
    update_answers(&data, &|a| a.courses = rows.clone()).unwrap();
    let before = course_files(&vault);
    let out = run_school_courses_in(&data, &vault);
    assert!(matches!(out, Outcome::Done(_)), "{out:?}");
    for (f, text) in &restored {
        assert_eq!(std::fs::read_to_string(vault.join("courses").join(f)).unwrap(), *text, "{f} is untouched");
    }
    let after = course_files(&vault);
    assert_eq!(after.len(), before.len() + 1, "only the course with no note gets one: {after:?}");
    let slugs = load_answers(&data).unwrap().unwrap().slugs;
    assert_eq!(slugs["_1001_1"], "path-kept");
    assert_eq!(slugs["_1002_1"], "by-uid-kept");
    assert_eq!(slugs["_1003_1"], "by-code-kept");
    assert!(!restored.iter().any(|(f, _)| f.trim_end_matches(".md") == slugs["_1004_1"]), "the fourth course has a note of its own");
    let touched = journal_for(&vault).into_iter().filter(|r| r["path"].as_str().is_some_and(|p| restored.iter().any(|(f, _)| p.contains(f)))).count();
    assert_eq!(touched, 0, "no journal record names a restored note");
    let _ = std::fs::remove_dir_all(&root);
}

/// F6a review r1, I-1 (controller's ruling): one note per slug, first row wins. Two code-less rows of
/// the same name make one note on the first run, with no error, and both keys record the shared slug.
#[test]
fn two_codeless_rows_of_one_name_share_one_note_on_the_first_run() {
    let (root, vault, data) = f6a_vault("shared-slug");
    let rows = vec![f6a_course("", "Quilting Basics", "_2001_1"), f6a_course("", "Quilting Basics", "_2002_1")];
    update_answers(&data, &|a| a.courses = rows.clone()).unwrap();
    let before = course_files(&vault);
    let out = run_school_courses_in(&data, &vault);
    assert!(matches!(out, Outcome::Done(_)), "no error on the first run: {out:?}");
    assert_eq!(course_files(&vault).len(), before.len() + 1, "one note for the one slug");
    let slugs = load_answers(&data).unwrap().unwrap().slugs;
    assert_eq!(slugs["_2001_1"], "quilting-basics");
    assert_eq!(slugs["_2002_1"], slugs["_2001_1"], "both rows record the shared slug: {slugs:?}");
    assert!(matches!(run_school_courses_in(&data, &vault), Outcome::Done(_)));
    assert_eq!(load_answers(&data).unwrap().unwrap().slugs, slugs, "a retry changes nothing");
    let _ = std::fs::remove_dir_all(&root);
}

/// F6a review r1, M1: a row's own slug is cleaned before it names a file, so a separator never lets a
/// note land outside `courses/`.
#[test]
fn a_row_slug_with_path_separators_cannot_place_a_note_outside_courses() {
    let (root, vault, data) = f6a_vault("slug-path");
    let mut evil = f6a_course("QLT 101", "Quilting Basics", "_3001_1");
    evil.slug = "../tasks/evil".into();
    update_answers(&data, &|a| a.courses = vec![evil.clone()]).unwrap();
    let tasks_before = std::fs::read_dir(vault.join("tasks")).unwrap().count();
    assert!(matches!(run_school_courses_in(&data, &vault), Outcome::Done(_)));
    assert_eq!(std::fs::read_dir(vault.join("tasks")).unwrap().count(), tasks_before, "nothing under tasks/");
    let slug = load_answers(&data).unwrap().unwrap().slugs["_3001_1"].clone();
    assert!(!slug.contains('/') && !slug.contains('.'), "{slug}");
    assert!(vault.join("courses").join(format!("{slug}.md")).exists());
    let _ = std::fs::remove_dir_all(&root);
}

// ---- O3 F6b: `config.ingest` creates `ingest.yaml` once (plan F6b; §4.9, §4.11, §14.45, §14.46) ------
// The reference is `build_into`'s own text: a full `create_vault` for the same answers (an account, so
// a stored feed is blanked), plus `restore_capability_url` for a link the account did not take. The
// links are fakes (`example.invalid`) and are never printed: feed tests compare with `assert!`.
use knowlu::onboarding_jobs::{put_ingest_text, run_config_ingest, run_config_ingest_with, IngestBase};
use knowlu::onboarding_state::{Answers, BookRow, FeedState, PendingStore, PENDING_CALENDAR, PENDING_LMS};
use knowlu::scaffold::{restore_capability_url, BookMapping, SectionMapping};

const F6B_LMS_LINK: &str = "https://example.invalid/feeds/f6b-lms-token.ics";
const F6B_CAL_LINK: &str = "https://example.invalid/feeds/f6b-cal-token.ics";

#[derive(Default)]
struct F6bPending(std::sync::Mutex<std::collections::BTreeMap<String, String>>);
impl PendingStore for F6bPending {
    fn put(&self, name: &str, link: &str) -> Result<(), String> { self.0.lock().unwrap().insert(name.into(), link.into()); Ok(()) }
    fn get(&self, name: &str) -> Option<String> { self.0.lock().unwrap().get(name).cloned() }
    fn delete(&self, name: &str) -> Result<(), String> { self.0.lock().unwrap().remove(name); Ok(()) }
}

fn f6b_course(code: &str, name: &str, key: &str, fragments: &[&str]) -> CourseAnswer {
    let mut c = f6a_course(code, name, key);
    c.fragments = fragments.iter().map(|f| f.to_string()).collect();
    c
}

fn f6b_pairs(v: &[(&str, &str)]) -> Vec<(String, String)> { v.iter().map(|(a, b)| (a.to_string(), b.to_string())).collect() }

/// One onboarding run's answers, and the reference's inputs that do not come from them.
struct Scn {
    courses: Vec<CourseAnswer>,
    slugs: Vec<(&'static str, &'static str)>,
    lms: Option<FeedState>,
    cal: Option<FeedState>,
    google: Option<bool>,
    /// Step 8 answered: a zyBooks row, a VHL row and two ignored books, with both logins stored.
    step8: bool,
    /// The `course_map` pairs the file must carry, in order.
    pairs: Vec<(&'static str, &'static str)>,
    /// The `course_exclude` fragments, in order; empty means the key is absent.
    exclude: Vec<&'static str>,
}

impl Scn {
    fn quilting() -> Scn {
        Scn {
            courses: vec![
                f6b_course("QLT 101", "Quilting Basics", "_1001_1", &["QLT101Fall2026", "QLT 101"]),
                f6b_course("KNT 220", "Knotwork Theory", "_1002_1", &["KNT220Fall2026", "KNT 220"]),
            ],
            slugs: vec![("_1001_1", "quilting-basics"), ("_1002_1", "knotwork-theory")],
            lms: Some(FeedState::Stored),
            cal: Some(FeedState::Stored),
            google: Some(true),
            step8: true,
            pairs: vec![("QLT101Fall2026", "quilting-basics"), ("QLT 101", "quilting-basics"), ("KNT220Fall2026", "knotwork-theory"), ("KNT 220", "knotwork-theory")],
            exclude: Vec::new(),
        }
    }
}

struct F6b { root: PathBuf, vault: PathBuf, data: PathBuf, base: IngestBase, pending: F6bPending }

fn f6b_rig(tag: &str, s: &Scn) -> F6b {
    let (root, vault, data) = f6a_vault(&format!("f6b-{tag}"));
    let base = IngestBase { profile_id: "profile-f6b".into(), timezone: "America/Chicago".into(), zybooks: s.step8, vhl: s.step8 };
    let pending = F6bPending::default();
    for (kind, state, link) in [(PENDING_LMS, s.lms, F6B_LMS_LINK), (PENDING_CALENDAR, s.cal, F6B_CAL_LINK)] {
        // A stored feed's pending credential is already deleted; the others still wait.
        if matches!(state, Some(FeedState::Failed | FeedState::Rejected)) { pending.put(kind, link).unwrap(); }
    }
    update_answers(&data, &|a: &mut Answers| {
        a.courses = s.courses.clone();
        a.slugs = s.slugs.iter().map(|(k, v)| (k.to_string(), v.to_string())).collect();
        a.google_calendar = s.google;
        if let Some(st) = s.lms { a.feeds.insert("lms_ics".into(), st); }
        if let Some(st) = s.cal { a.feeds.insert("calendar_ics".into(), st); }
        if s.step8 {
            a.rows = vec![
                BookRow { source: "zybooks".into(), key: "ZYB-QLT-101".into(), course_slug: "quilting-basics".into() },
                BookRow { source: "vhl".into(), key: "4471".into(), course_slug: "knotwork-theory".into() },
            ];
            a.zybooks_ignore = vec!["HowToUseZyBooks2".into(), "ZYB-OLD-9".into()];
        }
    }).unwrap();
    F6b { root, vault, data, base, pending }
}

fn f6b_exclude_line(items: &[&str]) -> String {
    let quoted: Vec<String> = items.iter().map(|f| format!("'{}'", f.replace('\'', "''"))).collect();
    format!("course_exclude: [{}]{}", quoted.join(", "), knowlu_engine::pystr::NEWLINE)
}

/// What `build_into` (plus `restore_capability_url` for a kept link) writes for the same answers;
/// `course_exclude` goes right after the `course_map` block, where `calendars:` starts.
fn f6b_expected(r: &F6b, tag: &str, s: &Scn) -> Vec<u8> {
    let reference = r.root.join(format!("reference-{tag}"));
    let mut plan = f6a_plan(&reference, "none", &CampusChoice::default());
    plan.profile_id = r.base.profile_id.clone();
    plan.personal_calendar = s.cal.map(|_| F6B_CAL_LINK.to_string());
    plan.google_calendar = s.google == Some(true);
    plan.course_map = f6b_pairs(&s.pairs);
    if s.step8 {
        plan.zybooks = true;
        plan.vhl = true;
        plan.zybooks_courses = vec![BookMapping { code: "ZYB-QLT-101".into(), course: "quilting-basics".into(), label: String::new() }];
        plan.vhl_sections = vec![SectionMapping { section: "4471".into(), course: "knotwork-theory".into(), label: String::new() }];
        plan.zybooks_ignore = vec!["HowToUseZyBooks2".into(), "ZYB-OLD-9".into()];
    }
    knowlu::scaffold::create_vault(&reference, &plan).unwrap();
    if matches!(s.lms, Some(FeedState::Failed | FeedState::Rejected)) { restore_capability_url(&reference, "lms_ics", F6B_LMS_LINK).unwrap(); }
    if matches!(s.cal, Some(FeedState::Failed | FeedState::Rejected)) { restore_capability_url(&reference, "calendar_ics", F6B_CAL_LINK).unwrap(); }
    let text = String::from_utf8(config_bytes(&reference, "ingest.yaml")).unwrap();
    if s.exclude.is_empty() { return text.into_bytes(); }
    let at = text.find("calendars:").expect("calendars: in the reference");
    format!("{}{}{}", &text[..at], f6b_exclude_line(&s.exclude), &text[at..]).into_bytes()
}

/// Runs `config.ingest` for `s` and returns (what it wrote, what `build_into` would have).
fn f6b_run(tag: &str, s: &Scn) -> (F6b, Outcome, Vec<u8>) {
    let r = f6b_rig(tag, s);
    let expected = f6b_expected(&r, tag, s);
    let out = run_config_ingest(&r.vault, &r.data, &r.base, &r.pending);
    (r, out, expected)
}

fn f6b_written(r: &F6b) -> Vec<u8> {
    std::fs::read(r.vault.join("config").join("ingest.yaml")).unwrap_or_else(|e| panic!("config/ingest.yaml was not created: {e}"))
}

fn f6b_check_root(r: &F6b) -> PathBuf { r.data.join("tmp").join(format!("ingest-check-{}", std::process::id())) }

/// F6b test 1: `ingest.yaml`'s bytes equal `build_into`'s for the same answers; the engine's own loader
/// reads them with no warning; nothing else in `config/` appears or changes, and the check's scratch
/// root is gone.
#[test]
fn config_ingest_writes_the_bytes_build_into_writes_for_the_same_answers() {
    let s = Scn::quilting();
    let r = f6b_rig("bytes", &s);
    let expected = f6b_expected(&r, "bytes", &s);
    let before: Vec<String> = std::fs::read_dir(r.vault.join("config")).unwrap().flatten().map(|e| e.file_name().to_string_lossy().to_string()).collect();
    let out = run_config_ingest(&r.vault, &r.data, &r.base, &r.pending);
    assert!(matches!(out, Outcome::Done(_)), "{out:?}");
    assert!(f6b_written(&r) == expected, "config/ingest.yaml differs from build_into's");
    let (_, warnings) = knowlu_engine::coursework::load_coursework_config(&r.vault).expect("the engine reads it");
    assert!(warnings.is_empty(), "no warning: {warnings:?}");
    let mut after: Vec<String> = std::fs::read_dir(r.vault.join("config")).unwrap().flatten().map(|e| e.file_name().to_string_lossy().to_string()).collect();
    after.sort();
    let mut want = before;
    want.push("ingest.yaml".into());
    want.sort();
    assert_eq!(after, want, "only ingest.yaml is added to config/");
    assert!(!f6b_check_root(&r).exists(), "the scratch root is deleted");
    let text = String::from_utf8(f6b_written(&r)).unwrap();
    assert!(text.contains("ics_url: ''") && !text.contains("example.invalid"), "a stored feed leaves no link in the file");
    assert!(!text.contains("course_exclude"), "nothing was removed, so the key is absent");
    let _ = std::fs::remove_dir_all(&r.root);
}

/// F6b test 2: a skipped step 6, 7 or 8 leaves its section empty, exactly as today's wizard writes it
/// (the same reference, with that answer absent).
#[test]
fn a_skipped_step_leaves_its_section_empty_exactly_as_the_wizard_does() {
    let cases: [(&str, Box<dyn Fn(&mut Scn)>); 4] = [
        ("step6", Box::new(|s| s.google = None)),
        ("step7", Box::new(|s| s.cal = None)),
        ("step8", Box::new(|s| s.step8 = false)),
        ("all", Box::new(|s| { s.google = None; s.cal = None; s.step8 = false; })),
    ];
    for (tag, skip) in cases {
        let mut s = Scn::quilting();
        skip(&mut s);
        let (r, out, expected) = f6b_run(&format!("skip-{tag}"), &s);
        assert!(matches!(out, Outcome::Done(_)), "{tag}: {out:?}");
        assert!(f6b_written(&r) == expected, "{tag}: differs from build_into's");
        let text = String::from_utf8(f6b_written(&r)).unwrap();
        match tag {
            "step6" => assert!(!text.contains("cloud:google") && text.contains("cloud:personal"), "{tag}"),
            "step7" => assert!(text.contains("cloud:google") && !text.contains("cloud:personal"), "{tag}"),
            "step8" => assert!(!text.contains("coursework:"), "{tag}: no coursework block"),
            _ => assert!(text.contains("calendars: []") && !text.contains("coursework:"), "{tag}"),
        }
        let _ = std::fs::remove_dir_all(&r.root);
    }
    // A Google answer of `false` is the same as no answer.
    let mut s = Scn::quilting();
    s.google = Some(false);
    let (r, out, expected) = f6b_run("google-false", &s);
    assert!(matches!(out, Outcome::Done(_)) && f6b_written(&r) == expected, "{out:?}");
    let _ = std::fs::remove_dir_all(&r.root);
}

/// F6b test 3 (§4.11, R-C3'-exec-40/41): for each feed, `stored` writes the empty entry and no link;
/// `failed` and `rejected` write the link itself, and the bytes are `build_into`'s followed by
/// `restore_capability_url`. The two feeds are independent, so every pairing is checked.
#[test]
fn feeds_are_blank_when_stored_and_carry_the_link_when_failed_or_rejected() {
    let states = [FeedState::Stored, FeedState::Failed, FeedState::Rejected];
    for lms in states {
        for cal in states {
            let mut s = Scn::quilting();
            s.lms = Some(lms);
            s.cal = Some(cal);
            let tag = format!("feeds-{lms:?}-{cal:?}").to_lowercase();
            let (r, out, expected) = f6b_run(&tag, &s);
            assert!(matches!(out, Outcome::Done(_)), "{tag}: {out:?}");
            let got = f6b_written(&r);
            assert!(got == expected, "{tag}: differs from build_into plus restore_capability_url");
            let text = String::from_utf8(got).unwrap();
            assert_eq!(text.contains(F6B_LMS_LINK), lms != FeedState::Stored, "{tag}: the LMS link is in the file only when the account did not take it");
            assert_eq!(text.contains(F6B_CAL_LINK), cal != FeedState::Stored, "{tag}: likewise the other calendar's link");
            if lms == FeedState::Stored { assert!(text.contains("ics_url: ''\n") || text.contains("ics_url: ''\r\n"), "{tag}"); }
            let _ = std::fs::remove_dir_all(&r.root);
        }
    }
}

/// F6b test 3, first half again: a feed link kept for a feed whose state is `failed` is read from the
/// pending store and nowhere else (not from the answers file, which never holds a link).
#[test]
fn the_answers_file_never_holds_a_feed_link() {
    let mut s = Scn::quilting();
    s.lms = Some(FeedState::Failed);
    s.cal = Some(FeedState::Rejected);
    let (r, out, _) = f6b_run("no-link-in-answers", &s);
    assert!(matches!(out, Outcome::Done(_)), "{out:?}");
    let answers = std::fs::read_to_string(knowlu::onboarding_state::answers_path(&r.data)).unwrap();
    assert!(!answers.contains("example.invalid"), "the answers file holds states, never links");
    let _ = std::fs::remove_dir_all(&r.root);
}

/// F6b test 4 (the audit's fix, §13.1): `load_coursework_config` reads `<vault>/config/ingest.yaml` and
/// answers empty for an absent file, so a check pointed at the wrong path passes silently. The job
/// writes the text into a scratch root under the profile's app data, checks that root, and deletes it.
/// A malformed text makes the job `failed` with no file created, which proves the check read the text.
#[test]
fn a_text_that_does_not_parse_cleanly_fails_the_job_and_creates_nothing() {
    let (root, vault, data) = f6a_vault("f6b-malformed");
    let scratch = data.join("tmp").join(format!("ingest-check-{}", std::process::id()));
    // A stale, valid file at the scratch path (a crashed run's): the check must overwrite it, not read it.
    std::fs::create_dir_all(scratch.join("config")).unwrap();
    std::fs::write(scratch.join("config").join("ingest.yaml"), b"ics_url: ''\ntimezone: 'America/Chicago'\ncourse_map: {}\ncalendars: []\n").unwrap();
    let reached = std::cell::Cell::new(0);
    let probe = |_: &Path| -> Result<(), String> { reached.set(reached.get() + 1); Ok(()) };
    let seams = PutSeams { after_temp: Some(&probe), ..Default::default() };
    for (what, text) in [
        ("unclosed flow sequence", "ics_url: ''\ncourse_exclude: ['a', 'b'\ncalendars: []\n"),
        ("a duplicate key", "ics_url: ''\ncourse_map:\n  'a': 'x'\n  'a': 'y'\ncalendars: []\n"),
    ] {
        // The premise: the engine's own loader warns about this text when it can see it.
        let premise = root.join(format!("premise-{}", what.len()));
        std::fs::create_dir_all(premise.join("config")).unwrap();
        std::fs::write(premise.join("config").join("ingest.yaml"), text).unwrap();
        let (_, warned) = knowlu_engine::coursework::load_coursework_config(&premise).unwrap_or_default();
        assert!(!warned.is_empty(), "{what}: the loader must warn about this text");
        let out = put_ingest_text(&vault, &data, text, &seams);
        assert!(matches!(out, Outcome::Failed(_)), "{what}: {out:?}");
        assert!(!vault.join("config").join("ingest.yaml").exists(), "{what}: no file is created");
        assert!(strays_in_config(&vault).is_empty(), "{what}");
        assert_eq!(reached.get(), 0, "{what}: the check runs before anything is written to config/");
        assert!(!scratch.exists(), "{what}: the scratch root is deleted on a failed check too");
    }
    // The same job with a good text creates the file and still leaves no scratch root.
    let good = "ics_url: ''\ntimezone: 'America/Chicago'\ncourse_map: {}\ncalendars: []\n";
    let out = put_ingest_text(&vault, &data, good, &PutSeams::default());
    assert!(matches!(out, Outcome::Done(_)), "{out:?}");
    assert_eq!(config_bytes(&vault, "ingest.yaml"), good.replace('\n', knowlu_engine::pystr::NEWLINE).as_bytes());
    assert!(!scratch.exists(), "the scratch root is deleted");
    let _ = std::fs::remove_dir_all(&root);
}

/// F6b test 5: an existing file is kept, unchanged, and the job reports "kept your existing settings";
/// the same bytes are a resume and the job is simply `done`.
#[test]
fn an_existing_ingest_file_is_kept_and_a_resume_is_simply_done() {
    let s = Scn::quilting();
    let r = f6b_rig("keeps", &s);
    std::fs::write(r.vault.join("config").join("ingest.yaml"), b"timezone: 'my own'\n").unwrap();
    let proof = proof_of(&run_config_ingest(&r.vault, &r.data, &r.base, &r.pending));
    assert_eq!(config_bytes(&r.vault, "ingest.yaml"), b"timezone: 'my own'\n", "never replaced");
    assert_eq!(proof.get(KEPT_EXISTING_KEY).copied().flatten(), Some(1), "{proof:?}");
    // A resume: the file holds what this run would write.
    std::fs::remove_file(r.vault.join("config").join("ingest.yaml")).unwrap();
    assert!(matches!(run_config_ingest(&r.vault, &r.data, &r.base, &r.pending), Outcome::Done(_)));
    let first = config_bytes(&r.vault, "ingest.yaml");
    let again = proof_of(&run_config_ingest(&r.vault, &r.data, &r.base, &r.pending));
    assert_eq!(again.get(KEPT_EXISTING_KEY).copied().flatten().unwrap_or(0), 0, "equal bytes are a resume: {again:?}");
    assert_eq!(config_bytes(&r.vault, "ingest.yaml"), first);
    let _ = std::fs::remove_dir_all(&r.root);
}

/// F6b test 5, mid-way: a file created after the temp file is written (a seam) is kept, never replaced.
#[test]
fn an_ingest_file_created_mid_way_is_kept() {
    let s = Scn::quilting();
    let r = f6b_rig("midway", &s);
    let racer = |p: &Path| -> Result<(), String> { std::fs::write(p, b"timezone: 'raced in'\n").map_err(|e| e.to_string()) };
    let seams = PutSeams { after_temp: Some(&racer), ..Default::default() };
    let proof = proof_of(&run_config_ingest_with(&r.vault, &r.data, &r.base, &r.pending, &seams));
    assert_eq!(config_bytes(&r.vault, "ingest.yaml"), b"timezone: 'raced in'\n", "never replaced");
    assert_eq!(proof.get(KEPT_EXISTING_KEY).copied().flatten(), Some(1), "{proof:?}");
    assert!(strays_in_config(&r.vault).is_empty());
    let _ = std::fs::remove_dir_all(&r.root);
}

/// F6b test 6 (F6a's partial-failure test, review r2 M4): a failure part-way leaves no partial file at
/// the final name, the job is `failed`, and a stray temp file is removed on the next run.
#[test]
fn a_failure_part_way_leaves_no_partial_ingest_file_and_the_next_run_sweeps_a_stray() {
    let s = Scn::quilting();
    let r = f6b_rig("partial", &s);
    let boom = |_: &Path| -> Result<(), String> { Err("the disk went away".to_string()) };
    let seams = PutSeams { after_temp: Some(&boom), ..Default::default() };
    let out = run_config_ingest_with(&r.vault, &r.data, &r.base, &r.pending, &seams);
    assert!(matches!(out, Outcome::Failed(_)), "{out:?}");
    assert!(!r.vault.join("config").join("ingest.yaml").exists(), "no partial file at the final name");
    assert!(strays_in_config(&r.vault).is_empty(), "the failed run cleans its own temp file");
    assert!(!f6b_check_root(&r).exists(), "and the scratch root");
    let stray = r.vault.join("config").join(format!("{TEMP_PREFIX}ingest.yaml-left-by-a-crash"));
    std::fs::write(&stray, b"timezone: 'half a fi").unwrap();
    assert!(matches!(run_config_ingest(&r.vault, &r.data, &r.base, &r.pending), Outcome::Done(_)));
    assert!(!stray.exists(), "the stray temp file is removed on the next run");
    assert!(f6b_written(&r) == f6b_expected(&r, "partial", &s), "and the file is whole");
    let _ = std::fs::remove_dir_all(&r.root);
}

/// F6b, the run-time fallback through the same helper (ledger row "§1.9, CLAUDE.md"): with the hard link
/// forced to fail "not supported" the no-replace move puts the same bytes in place; with both forced to
/// fail the job is `failed` with a fix that names the volume and nothing is at the final name.
#[test]
fn the_ingest_file_uses_the_fallback_and_names_the_volume_when_both_steps_fail() {
    let s = Scn::quilting();
    let r = f6b_rig("fallback", &s);
    let expected = f6b_expected(&r, "fallback", &s);
    let no_link = |_: &Path, _: &Path| -> std::io::Result<()> { Err(not_supported()) };
    let seams = PutSeams { hard_link: Some(&no_link), ..Default::default() };
    assert!(matches!(run_config_ingest_with(&r.vault, &r.data, &r.base, &r.pending, &seams), Outcome::Done(_)));
    assert!(f6b_written(&r) == expected, "same bytes through the fallback");
    std::fs::remove_file(r.vault.join("config").join("ingest.yaml")).unwrap();
    let no_move = |_: &Path, _: &Path| -> std::io::Result<()> { Err(std::io::Error::new(std::io::ErrorKind::PermissionDenied, "the volume refused")) };
    let seams = PutSeams { hard_link: Some(&no_link), move_no_replace: Some(&no_move), ..Default::default() };
    let out = run_config_ingest_with(&r.vault, &r.data, &r.base, &r.pending, &seams);
    let Outcome::Failed(fix) = out else { panic!("both steps failing must fail the job, got {out:?}") };
    let volume = match r.vault.components().next() { Some(std::path::Component::Prefix(p)) => p.as_os_str().to_string_lossy().to_string(), other => panic!("{other:?}") };
    assert!(fix.contains(&volume), "the fix names the volume {volume}: {fix}");
    assert!(!r.vault.join("config").join("ingest.yaml").exists() && strays_in_config(&r.vault).is_empty());
    let _ = std::fs::remove_dir_all(&r.root);
}

/// An absent answers file is `failed`, never an empty config (as `school.courses`).
#[test]
fn without_an_answers_file_config_ingest_fails_and_creates_nothing() {
    let (root, vault, data) = f6a_vault("f6b-no-answers");
    let base = IngestBase { profile_id: "profile-f6b".into(), timezone: "America/Chicago".into(), zybooks: false, vhl: false };
    let out = run_config_ingest(&vault, &data, &base, &F6bPending::default());
    assert!(matches!(out, Outcome::Failed(_)), "{out:?}");
    assert!(!vault.join("config").join("ingest.yaml").exists());
    let _ = std::fs::remove_dir_all(&root);
}

/// F6b test 7 (review r2, I5): with two restored course notes (a C3' restore), `school.courses` records
/// the restored notes' own slugs, and `course_map` names exactly those; no second note is made, and the
/// derived slugs do not appear.
#[test]
fn course_map_names_the_restored_notes_slugs() {
    let mut s = Scn::quilting();
    s.slugs = Vec::new();
    s.step8 = false;
    s.pairs = vec![
        ("QLT101Fall2026", "quilting-basics-restored"), ("QLT 101", "quilting-basics-restored"),
        ("KNT220Fall2026", "knotwork-restored"), ("KNT 220", "knotwork-restored"),
    ];
    let r = f6b_rig("restored", &s);
    let note = |extra: &str| format!("---\ntitle: Restored\n{extra}status: active\n---\n\nmy own notes\n");
    std::fs::write(r.vault.join("courses").join("quilting-basics-restored.md"), note("source_uid: _1001_1\n")).unwrap();
    std::fs::write(r.vault.join("courses").join("knotwork-restored.md"), note("code: KNT 220\n")).unwrap();
    let notes_before = course_files(&r.vault);
    assert!(matches!(run_school_courses_in(&r.data, &r.vault), Outcome::Done(_)));
    assert_eq!(course_files(&r.vault), notes_before, "no second note is made for a restored course");
    let slugs = load_answers(&r.data).unwrap().unwrap().slugs;
    assert_eq!((slugs["_1001_1"].as_str(), slugs["_1002_1"].as_str()), ("quilting-basics-restored", "knotwork-restored"));
    let expected = f6b_expected(&r, "restored", &s);
    assert!(matches!(run_config_ingest(&r.vault, &r.data, &r.base, &r.pending), Outcome::Done(_)));
    let got = f6b_written(&r);
    assert!(got == expected, "course_map must carry the restored notes' slugs");
    let text = String::from_utf8(got).unwrap();
    assert!(!text.contains("'quilting-basics'") && !text.contains("'knotwork-theory'"), "the derived slugs are not used");
    let _ = std::fs::remove_dir_all(&r.root);
}

fn f6b_removed(mut c: CourseAnswer) -> CourseAnswer { c.removed = true; c }

/// F6b test 8 (§14.45): a course removed at step 3 has its pairs written as one single-line flow
/// sequence `course_exclude: [...]` through `yaml_scalar` (right after the `course_map` block), in no
/// `course_map` entry; the bytes are otherwise `build_into`'s.
#[test]
fn a_removed_course_goes_in_course_exclude_and_in_no_course_map_entry() {
    let mut s = Scn::quilting();
    s.step8 = false;
    s.courses[1] = f6b_removed(s.courses[1].clone());
    s.slugs = vec![("_1001_1", "quilting-basics")];
    s.pairs = vec![("QLT101Fall2026", "quilting-basics"), ("QLT 101", "quilting-basics")];
    s.exclude = vec!["KNT220Fall2026", "KNT 220"];
    let (r, out, expected) = f6b_run("exclude", &s);
    assert!(matches!(out, Outcome::Done(_)), "{out:?}");
    let got = f6b_written(&r);
    assert!(got == expected, "differs from build_into's text plus the course_exclude line");
    let text = String::from_utf8(got).unwrap();
    assert!(text.contains("course_exclude: ['KNT220Fall2026', 'KNT 220']"), "one single-line flow sequence");
    let map_block = &text[text.find("course_map:").unwrap()..text.find("course_exclude:").unwrap()];
    assert!(!map_block.contains("KNT"), "a removed course is in no course_map entry: {map_block}");
    let (_, warnings) = knowlu_engine::coursework::load_coursework_config(&r.vault).unwrap();
    assert!(warnings.is_empty(), "{warnings:?}");
    let _ = std::fs::remove_dir_all(&r.root);
}

/// F6b test 8, fallback: a course with no saved pairs contributes its human code alone, kept (to
/// `course_map`) or removed (to `course_exclude`), as the hand-over's own fallback says.
#[test]
fn a_course_with_no_saved_pairs_contributes_its_code() {
    let mut s = Scn::quilting();
    s.step8 = false;
    s.courses = vec![f6b_course("QLT 101", "Quilting Basics", "_1001_1", &[]), f6b_removed(f6b_course("KNT 220", "Knotwork Theory", "_1002_1", &[]))];
    s.slugs = vec![("_1001_1", "quilting-basics")];
    s.pairs = vec![("QLT 101", "quilting-basics")];
    s.exclude = vec!["KNT 220"];
    let (r, out, expected) = f6b_run("fallback-code", &s);
    assert!(matches!(out, Outcome::Done(_)), "{out:?}");
    assert!(f6b_written(&r) == expected, "differs from build_into's text plus the course_exclude line");
    let _ = std::fs::remove_dir_all(&r.root);
}

/// F6b test 9 (§14.46, F7c review r2): a lecture and its lab both contribute `CS 100`. With the lab
/// removed and the lecture kept, the shared fragment stays in `course_map` and is absent from
/// `course_exclude`; only the lab's own code goes there. When every removed fragment is shared the key
/// is absent altogether, and the bytes are `build_into`'s.
#[test]
fn a_fragment_a_kept_course_also_contributes_is_never_excluded() {
    let lecture = f6b_course("CS 100", "Intro to Computing", "_2001_1", &["CS100L1", "CS 100"]);
    let mut s = Scn::quilting();
    s.step8 = false;
    s.courses = vec![lecture.clone(), f6b_removed(f6b_course("CS 100", "Intro to Computing Lab", "_2002_1", &["CS100B1", "CS 100"]))];
    s.slugs = vec![("_2001_1", "cs-100")];
    s.pairs = vec![("CS100L1", "cs-100"), ("CS 100", "cs-100")];
    s.exclude = vec!["CS100B1"];
    let (r, out, expected) = f6b_run("shared", &s);
    assert!(matches!(out, Outcome::Done(_)), "{out:?}");
    let got = f6b_written(&r);
    assert!(got == expected, "only the lab's own code is excluded");
    let text = String::from_utf8(got).unwrap();
    assert!(text.contains("course_exclude: ['CS100B1']"), "{text}");
    assert!(text.contains("'CS 100': 'cs-100'"), "the shared fragment stays in course_map");
    let _ = std::fs::remove_dir_all(&r.root);
    // Every fragment the lab has is shared: nothing to exclude, so no key.
    s.courses[1] = f6b_removed(f6b_course("CS 100", "Intro to Computing Lab", "_2002_1", &["CS 100"]));
    s.exclude = Vec::new();
    let (r, out, expected) = f6b_run("shared-all", &s);
    assert!(matches!(out, Outcome::Done(_)), "{out:?}");
    let got = f6b_written(&r);
    assert!(got == expected && !String::from_utf8(got).unwrap().contains("course_exclude"), "no key when nothing is left to exclude");
    let _ = std::fs::remove_dir_all(&r.root);
}

/// F6b test 10 (§14.46): a course's gradebook pins (`_<digits>_<digits>`) follow its code fragments, in
/// the answers file's order: `course_map` entries for a kept course, `course_exclude` items for a
/// removed one. With none removed the bytes are `build_into`'s plus the pin lines.
#[test]
fn uid_pins_follow_their_courses_code_fragments_in_course_map_and_course_exclude() {
    let mut s = Scn::quilting();
    s.step8 = false;
    s.courses[0] = f6b_course("QLT 101", "Quilting Basics", "_1001_1", &["QLT101Fall2026", "QLT 101", "_71_1", "_71_2"]);
    s.courses[1] = f6b_course("KNT 220", "Knotwork Theory", "_1002_1", &["KNT220Fall2026", "KNT 220", "_72_1"]);
    s.pairs = vec![
        ("QLT101Fall2026", "quilting-basics"), ("QLT 101", "quilting-basics"), ("_71_1", "quilting-basics"), ("_71_2", "quilting-basics"),
        ("KNT220Fall2026", "knotwork-theory"), ("KNT 220", "knotwork-theory"), ("_72_1", "knotwork-theory"),
    ];
    let (r, out, expected) = f6b_run("pins-kept", &s);
    assert!(matches!(out, Outcome::Done(_)), "{out:?}");
    let got = f6b_written(&r);
    assert!(got == expected, "build_into's text plus the pin lines, in the answers file's order");
    let text = String::from_utf8(got).unwrap();
    assert!(text.find("'QLT 101'").unwrap() < text.find("'_71_1'").unwrap(), "a pin follows its course's code fragments");
    let _ = std::fs::remove_dir_all(&r.root);
    // The second course removed: its pins are excluded, after its code fragments, and are in no map entry.
    s.courses[1] = f6b_removed(s.courses[1].clone());
    s.slugs = vec![("_1001_1", "quilting-basics")];
    s.pairs.truncate(4);
    s.exclude = vec!["KNT220Fall2026", "KNT 220", "_72_1"];
    let (r, out, expected) = f6b_run("pins-removed", &s);
    assert!(matches!(out, Outcome::Done(_)), "{out:?}");
    let got = f6b_written(&r);
    assert!(got == expected, "a removed course's pins are in course_exclude only");
    let text = String::from_utf8(got).unwrap();
    assert!(text.contains("course_exclude: ['KNT220Fall2026', 'KNT 220', '_72_1']"), "{text}");
    assert!(!text.contains("'_72_1': "), "no map entry for a removed pin");
    let _ = std::fs::remove_dir_all(&r.root);
}

// ---- F6b review r1: the guards on a feed that has not settled, and the redacted Debug --------------------

fn f6b_no_file(r: &F6b) {
    assert!(!r.vault.join("config").join("ingest.yaml").exists(), "nothing is created");
    assert!(strays_in_config(&r.vault).is_empty() && !f6b_check_root(r).exists());
}

/// I-1: a feed that is `failed` or `rejected` whose pending credential is gone (or unreadable) cannot
/// have its link written: the job fails with that feed's key, creates nothing, and touches no credential.
#[test]
fn a_kept_feed_with_no_pending_link_fails_with_its_key_and_creates_nothing() {
    for (lms, cal, want) in [
        (FeedState::Failed, FeedState::Stored, "lms_link_missing"),
        (FeedState::Rejected, FeedState::Stored, "lms_link_missing"),
        (FeedState::Stored, FeedState::Failed, "calendar_link_missing"),
        (FeedState::Stored, FeedState::Rejected, "calendar_link_missing"),
    ] {
        let mut s = Scn::quilting();
        s.lms = Some(lms);
        s.cal = Some(cal);
        let r = f6b_rig(&format!("nolink-{lms:?}-{cal:?}").to_lowercase(), &s);
        // The other feed's link stays where it waits; the missing one is gone.
        let (gone, other) = if want.starts_with("lms") { (PENDING_LMS, PENDING_CALENDAR) } else { (PENDING_CALENDAR, PENDING_LMS) };
        r.pending.delete(gone).unwrap();
        r.pending.put(other, "https://example.invalid/feeds/other-token.ics").unwrap();
        let out = run_config_ingest(&r.vault, &r.data, &r.base, &r.pending);
        assert!(out == Outcome::Failed(want.into()), "{want}: {out:?}");
        f6b_no_file(&r);
        assert!(r.pending.get(gone).is_none() && r.pending.get(other).as_deref() == Some("https://example.invalid/feeds/other-token.ics"), "no credential is touched");
        let _ = std::fs::remove_dir_all(&r.root);
    }
}

/// I-2: a feed still `validated` has not settled, so the job waits (`pending`, with the reason), creates
/// nothing and leaves the waiting link where it is.
#[test]
fn a_validated_feed_makes_config_ingest_wait_and_creates_nothing() {
    for (lms, cal) in [(FeedState::Validated, FeedState::Stored), (FeedState::Stored, FeedState::Validated), (FeedState::Validated, FeedState::Validated)] {
        let mut s = Scn::quilting();
        s.lms = Some(lms);
        s.cal = Some(cal);
        let r = f6b_rig(&format!("validated-{lms:?}-{cal:?}").to_lowercase(), &s);
        r.pending.put(PENDING_LMS, F6B_LMS_LINK).unwrap();
        r.pending.put(PENDING_CALENDAR, F6B_CAL_LINK).unwrap();
        let out = run_config_ingest(&r.vault, &r.data, &r.base, &r.pending);
        assert!(out == Outcome::Waiting("feeds_not_settled".into()), "{out:?}");
        f6b_no_file(&r);
        assert!(r.pending.get(PENDING_LMS).as_deref() == Some(F6B_LMS_LINK) && r.pending.get(PENDING_CALENDAR).as_deref() == Some(F6B_CAL_LINK), "the waiting links stay");
        let _ = std::fs::remove_dir_all(&r.root);
    }
}

/// M4: a `stored` feed whose pending credential is still there (F8c deletes it after the job) is written
/// blank, with no link in the file, and the credential is untouched by this job.
#[test]
fn a_stored_feed_with_its_pending_link_still_present_is_written_blank_and_the_link_stays() {
    let s = Scn::quilting();
    let r = f6b_rig("stored-pending", &s);
    r.pending.put(PENDING_LMS, F6B_LMS_LINK).unwrap();
    r.pending.put(PENDING_CALENDAR, F6B_CAL_LINK).unwrap();
    let expected = f6b_expected(&r, "stored-pending", &s);
    let out = run_config_ingest(&r.vault, &r.data, &r.base, &r.pending);
    assert!(matches!(out, Outcome::Done(_)), "{out:?}");
    let got = f6b_written(&r);
    assert!(got == expected, "the same bytes as a stored feed with no pending link");
    let text = String::from_utf8(got).unwrap();
    assert!(!text.contains("example.invalid"), "a stored feed leaves no link in the file");
    assert!(r.pending.get(PENDING_LMS).as_deref() == Some(F6B_LMS_LINK) && r.pending.get(PENDING_CALENDAR).as_deref() == Some(F6B_CAL_LINK), "this job deletes nothing");
    let _ = std::fs::remove_dir_all(&r.root);
}

/// M5: `Debug` on the hand-over never prints a link (the field still carries it for the builder).
#[test]
fn the_hand_over_never_prints_a_feed_link() {
    let mut s = Scn::quilting();
    s.lms = Some(FeedState::Failed);
    s.cal = Some(FeedState::Rejected);
    let r = f6b_rig("debug", &s);
    let answers = load_answers(&r.data).unwrap().unwrap();
    let h = knowlu::onboarding_jobs::ingest_handoff(&answers, &r.pending);
    assert!(h.ics_url == F6B_LMS_LINK && h.calendar_url == F6B_CAL_LINK, "the builder still gets the links");
    let shown = format!("{h:?} {h:#?}");
    assert!(!shown.contains("example.invalid") && !shown.contains("token"), "Debug redacts the links");
    assert!(shown.contains("feed_kept_on_device: true"), "and still says a link is kept");
    let _ = std::fs::remove_dir_all(&r.root);
}
