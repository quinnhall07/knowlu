//! Ruling 3 of the cloud design's amendment of 2026-09-17, from the outside.
//!
//! No test here reads a real profile: `entitle`'s two `_in` functions take the app-data root, and
//! every case passes a temp directory. No test here touches Credential Manager — the gate is a file
//! and a clock, deliberately, because an entitlement check that needed a credential would be a
//! second thing to fail on a machine that is merely signed out.
use std::path::{Path, PathBuf};
use std::process::Command;

use knowlu_engine::childproc::NoConsole;
use knowlu_engine::entitle;

fn cache(root: &Path, profile: &str, status: &str, checked_at: &str) {
    let dir = root.join("knowlu").join("profiles").join(profile);
    std::fs::create_dir_all(&dir).expect("mkdir");
    std::fs::write(
        dir.join("entitlement.json"),
        format!(r#"{{"status":"{status}","current_period_end":null,"plan":"monthly","checked_at":"{checked_at}"}}"#),
    ).expect("write");
}

/// A minimal `config/cloud.yaml`, the same four keys every test in this file that gates a vault
/// needs.
fn cloud_yaml(vault: &Path, session_target: &str) {
    std::fs::create_dir_all(vault.join("config")).expect("mkdir");
    std::fs::write(
        vault.join("config").join("cloud.yaml"),
        format!("api_base: 'https://example.invalid/functions/v1'\nanon_key: 'anon'\nsession_credential_target: '{session_target}'\naccount_id: 'acct-1'\n"),
    ).expect("cloud.yaml");
}

fn binary() -> PathBuf { PathBuf::from(env!("CARGO_BIN_EXE_knowlu-engine")) }

#[test]
fn a_vault_with_no_account_is_not_gated_at_all() {
    // "an orphaned binary ranks a hand-made folder and nothing else" — the hand-made folder is the
    // one with no `config/cloud.yaml`, and nothing about it changes.
    let dir = temp("nogate");
    assert_eq!(entitle::gate(&dir), None);
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn a_fresh_check_lets_the_step_run_and_a_stale_one_does_not() {
    let now: jiff::Timestamp = "2026-09-17T12:00:00Z".parse().unwrap();
    let fresh = entitle::Cached { status: "active".into(), checked_at: "2026-09-17T06:00:00Z".into() };
    let stale = entitle::Cached { status: "active".into(), checked_at: "2026-09-13T06:00:00Z".into() };
    assert!(entitle::decide(Some(&fresh), now));
    assert!(!entitle::decide(Some(&stale), now), "past 72 hours the grace is spent");
    assert!(!entitle::decide(None, now), "no cache is not entitlement");
    for status in ["past_due", "canceled", "none", ""] {
        let c = entitle::Cached { status: status.into(), checked_at: "2026-09-17T06:00:00Z".into() };
        assert!(!entitle::decide(Some(&c), now), "{status}");
    }
    assert!(entitle::decide(Some(&entitle::Cached { status: "trialing".into(), checked_at: "2026-09-17T06:00:00Z".into() }), now));
}

#[test]
fn a_clock_an_hour_fast_is_tolerated_and_a_clock_a_year_fast_is_not() {
    // `app/src/account.rs::decide`'s own fix round 1 item 1, copied because copying it is the point:
    // a machine a few minutes ahead must not disentitle itself forever, and a machine a year ahead
    // is a clock that moved, not a licence.
    let now: jiff::Timestamp = "2026-09-17T12:00:00Z".parse().unwrap();
    assert!(entitle::decide(Some(&entitle::Cached { status: "active".into(), checked_at: "2026-09-17T12:30:00Z".into() }), now));
    assert!(!entitle::decide(Some(&entitle::Cached { status: "active".into(), checked_at: "2027-09-17T12:00:00Z".into() }), now));
}

#[test]
fn the_grace_boundary_is_inclusive_and_the_skew_clamp_is_exact() {
    // Fix round 1, I2: these mutants survive the rest of the suite without a case pinned exactly
    // at each edge — `<= GRACE_SECONDS` -> `<`, `<= GRACE_SECONDS + 86_400` (a day of drift), and
    // `-age <= 3600` -> `< 3600` or `<= 7200`. `app/tests/account.rs:360-361` pins the app's own
    // boundary the same way, "to the second".
    let cached = |checked_at: &str| entitle::Cached { status: "active".into(), checked_at: checked_at.into() };
    assert!(entitle::decide(Some(&cached("2026-09-14T12:00:00Z")), "2026-09-17T12:00:00Z".parse().unwrap()), "exactly 72h old is still entitled");
    assert!(!entitle::decide(Some(&cached("2026-09-14T12:00:00Z")), "2026-09-17T12:00:01Z".parse().unwrap()), "72h and one second is not");
    assert!(entitle::decide(Some(&cached("2026-09-17T13:00:00Z")), "2026-09-17T12:00:00Z".parse().unwrap()), "exactly one hour fast is still tolerated");
    assert!(!entitle::decide(Some(&cached("2026-09-17T13:00:01Z")), "2026-09-17T12:00:00Z".parse().unwrap()), "one hour and one second fast is not");
}

#[test]
fn the_engines_grace_is_the_apps_grace_and_the_path_is_the_apps_path() {
    // **Two constants in two crates, pinned from one side.** The app writes the file; the engine
    // reads it; a grace that drifted would mean the page said "your subscription lapsed" on a
    // different day from the one the slots stopped. Read as text, not linked: `knowlu-engine` does
    // not depend on `knowlu` and must not start.
    //
    // **Two files, not one** (review I2): the grace and the cache filename are `account.rs`'s
    // (`GRACE`, `cache_path`), and the app-data root is `state.rs`'s `app_data_root_in`. No line
    // numbers (Task 8 re-review N4): they drifted twice, on every merge from main. Reading
    // `account.rs` for all three is how the first draft of this test failed on a line that was never
    // there, so each assertion names the file it is about.
    //
    // **The comparison itself, not just the constant** (fix round 1, I2): a pin on `72 * 60 * 60`
    // alone never turns red if the app's own `<` became a `<=`, or its `||` became `&&`. Pinning
    // the two comparison lines verbatim closes that gap on the app's side; `decide`'s own boundary
    // test above closes it on the engine's.
    let app = Path::new(env!("CARGO_MANIFEST_DIR")).join("..").join("app").join("src");
    let account = std::fs::read_to_string(app.join("account.rs")).expect("app/src/account.rs");
    let state = std::fs::read_to_string(app.join("state.rs")).expect("app/src/state.rs");
    assert!(account.contains("from_secs(72 * 60 * 60)"), "app/src/account.rs: the app's grace moved; move the engine's with it");
    assert_eq!(entitle::GRACE_SECONDS, 72 * 60 * 60);
    assert!(account.contains(r#"data_dir.join("entitlement.json")"#), "app/src/account.rs: the cache filename moved");
    assert!(state.contains(r#"base.join("knowlu")"#), "app/src/state.rs: app_data_root_in moved");
    assert!(account.contains("-age <= 3600"), "app/src/account.rs: the skew clamp's own comparison moved");
    assert!(account.contains("age < 0 || age > GRACE.as_secs() as i64"), "app/src/account.rs: the grace comparison moved");
}

#[test]
fn the_profile_id_comes_out_of_the_credential_target_and_nowhere_else() {
    assert_eq!(entitle::profile_id("knowlu/profile_0a1b2c3d4e/session"), Some("profile_0a1b2c3d4e"));
    // Fix round 1, M4: `profile_id` used to accept any non-empty middle segment. The app's own
    // `is_profile_id` (account.rs) requires `profile_` plus exactly ten hex characters (either
    // case, through `is_ascii_hexdigit`, on both sides), because a profile id names folders and
    // Credential Manager entries — anything else is not "a different id", it is a path fragment
    // somebody typed.
    for bad in [
        "", "knowlu/session", "knowlu//session", "profile_1", "knowlu/p/extra/session",
        // M4: a `..` id and one carrying a backslash — a path fragment, never a real profile id.
        "knowlu/../session", "knowlu/profile_..\\..\\etc/session",
        // M4: the right prefix, the wrong length (nine hex characters, one short).
        "knowlu/profile_0a1b2c3d4/session",
        // ...and one hex character too many.
        "knowlu/profile_0a1b2c3d4ef/session",
        // A non-hex character in an otherwise right-length id.
        "knowlu/profile_0a1b2c3d4g/session",
    ] {
        assert_eq!(entitle::profile_id(bad), None, "{bad}");
    }
}

#[test]
fn a_canceled_subscriptions_gate_gets_one_named_line_and_that_line_is_all_the_caller_needs() {
    // Fix round 1, M6: this test's name used to claim "past the grace", but the cache below is
    // FRESH (`checked_at` equals `now`) with status `canceled` — it is really testing that a bad
    // status gates exactly like a stale one, not that staleness itself gates. Genuine staleness is
    // `gate_in_runs_on_a_fresh_active_cache_and_refuses_a_stale_one` below (review I3).
    let root = temp("gated");
    let vault = root.join("vault");
    cloud_yaml(&vault, "knowlu/profile_0a1b2c3d4e/session");
    cache(&root, "profile_0a1b2c3d4e", "canceled", "2026-09-17T06:00:00Z");
    let reason = entitle::gate_in(&root, &vault, "2026-09-17T12:00:00Z".parse().unwrap()).expect("gated");
    assert_eq!(reason, "skipped: no entitlement");
    // And the reason is a word, never a path or a status code from Stripe.
    assert!(!reason.contains("profile_") && !reason.contains("canceled"));
    let _ = std::fs::remove_dir_all(&root);
}

#[test]
fn cache_path_in_is_exactly_the_apps_profile_folder_layout() {
    // Review I3: the only existing `gate_in` case used a status that gates whether or not the file
    // is even found at the right path, so a slip in the path itself (a folder missing, a filename
    // typo) would have passed all nine tests fix round 1 started from.
    let base = Path::new("C:\\fake-appdata");
    assert_eq!(
        entitle::cache_path_in(base, "profile_0a1b2c3d4e"),
        base.join("knowlu").join("profiles").join("profile_0a1b2c3d4e").join("entitlement.json")
    );
}

#[test]
fn gate_in_runs_on_a_fresh_active_cache_and_refuses_a_stale_one() {
    // Review I3, the entitled path exercised for the first time: every prior `gate_in` case used a
    // status that gates regardless of the file's contents.
    let root = temp("gate-in-both-sides");
    let vault = root.join("vault");
    cloud_yaml(&vault, "knowlu/profile_0a1b2c3d4e/session");
    cache(&root, "profile_0a1b2c3d4e", "active", "2026-09-17T06:00:00Z");
    assert_eq!(entitle::gate_in(&root, &vault, "2026-09-17T12:00:00Z".parse().unwrap()), None, "a fresh active cache must run");
    cache(&root, "profile_0a1b2c3d4e", "active", "2026-09-10T06:00:00Z");
    assert_eq!(
        entitle::gate_in(&root, &vault, "2026-09-17T12:00:00Z".parse().unwrap()),
        Some("skipped: no entitlement".to_string()),
        "a cache past the grace must gate"
    );
    let _ = std::fs::remove_dir_all(&root);
}

#[test]
fn a_moved_vaults_profile_id_comes_from_the_registry_not_the_stale_cloud_yaml() {
    // Review I1: a vault whose folder moved (a USB stick that came back under a new drive letter,
    // or a restore to a new path) is re-registered by the app under a FRESH profile id
    // (`profiles::id_for` hashes the path string), and every slot after that refreshes that new
    // id's cache — never the id `config/cloud.yaml` still names from onboarding. Reading
    // `cloud.yaml`'s id first would read a cache the app has stopped writing to, forever.
    let root = temp("moved-vault");
    let vault = root.join("vault");
    // `cloud.yaml` still names the OLD (onboarding-time) profile id.
    cloud_yaml(&vault, "knowlu/profile_0000000001/session");
    // `profiles.json` (the app's own registry) now points THIS vault at a NEW id — the picker
    // adopted the moved folder under a fresh one.
    let appdata_root = root.join("knowlu");
    std::fs::create_dir_all(&appdata_root).expect("mkdir");
    let entries = serde_json::json!([{
        "id": "profile_0000000002",
        "name": "vault",
        "vault": vault.to_string_lossy(),
        "created_at": "2026-09-17T00:00:00Z",
        "last_opened_at": serde_json::Value::Null,
    }]);
    std::fs::write(appdata_root.join("profiles.json"), entries.to_string()).expect("profiles.json");
    // The app refreshed the NEW id's cache, fresh and active.
    cache(&root, "profile_0000000002", "active", "2026-09-17T06:00:00Z");
    // The OLD id's cache, if it is ever refreshed again, sits stale and canceled — proving the
    // assertion below reads the registry's id, never `cloud.yaml`'s.
    cache(&root, "profile_0000000001", "canceled", "2026-01-01T00:00:00Z");

    let reason = entitle::gate_in(&root, &vault, "2026-09-17T12:00:00Z".parse().unwrap());
    assert_eq!(reason, None, "the engine must read the registry's current id, not cloud.yaml's onboarding-time one");
    let _ = std::fs::remove_dir_all(&root);
}

#[test]
fn a_vault_with_no_registry_entry_falls_back_to_the_cloud_yaml_id() {
    // The registry lookup is an override, not a replacement: a vault the app has never opened (a
    // hand-typed `knowlu-engine.exe` run against a freshly-onboarded vault before the console's
    // first launch registers it) has no `profiles.json` entry at all, and the gate must still work
    // from `cloud.yaml`'s own id exactly as it always has.
    let root = temp("no-registry-entry");
    let vault = root.join("vault");
    cloud_yaml(&vault, "knowlu/profile_0a1b2c3d4e/session");
    cache(&root, "profile_0a1b2c3d4e", "active", "2026-09-17T06:00:00Z");
    assert_eq!(entitle::gate_in(&root, &vault, "2026-09-17T12:00:00Z".parse().unwrap()), None);
    let _ = std::fs::remove_dir_all(&root);
}

#[test]
fn the_line_a_student_reads_names_the_step_and_the_reason() {
    // **The one student-visible artefact of ruling 3, asserted** (review I3). `entitle::gate` answers
    // the reason; `main.rs`'s `name_of` supplies the word; the composed line is what lands in the
    // Runs view, and it has to read like the two skips the app already prints. Asserted against the
    // hand-off's own source rather than by spawning the binary for THIS case, because a spawn would
    // need a real `LOCALAPPDATA` profile and this one deliberately has none — the spawn test below
    // (fix round 1, I4) exercises the built binary itself.
    let main = std::fs::read_to_string(
        Path::new(env!("CARGO_MANIFEST_DIR")).join("src").join("main.rs"),
    ).expect("engine/src/main.rs");
    assert!(main.contains(r#"println!("{line}");"#), "the gate must print the one composed line");
    for word in ["\"coursework\"", "\"ingest\"", "\"judge\"", "\"sync\""] {
        assert!(main.contains(word), "name_of must answer for {word}");
    }
    // And the shape itself, spelled out once so a reader knows what to look for on the page.
    assert_eq!(format!("{} ({})", "sync", "skipped: no entitlement"), "sync (skipped: no entitlement)");
}

#[test]
fn a_gated_sync_persists_its_status_before_printing_the_line() {
    // Carry-forward from Task 7's review: without this, the console keeps showing a stale "in
    // step" (or whatever a sync run left behind before the subscription lapsed) forever after a
    // lapse, because nothing else ever touches `state/sync-status.json` when the GATE — not
    // `sync.rs` — is what stops the run. Asserted against the source, like the test above, for the
    // same reason: a spawn would need a real `LOCALAPPDATA` profile and this suite has none.
    let main = std::fs::read_to_string(
        Path::new(env!("CARGO_MANIFEST_DIR")).join("src").join("main.rs"),
    ).expect("engine/src/main.rs");
    assert!(
        main.contains("sync::record_gated_skip"),
        "the gate must persist a SyncStatus before it prints the sync skip line"
    );
}

#[test]
fn a_lapsed_vaults_sync_leaves_the_gated_status_on_disk() {
    // Carry-forward from Task 7's review, the mechanics half: `record_gated_skip` writes through
    // the same atomic `save_status` every other outcome uses, and the shape it writes must read
    // distinctly from an ordinary "no account"/"no session" skip (`ok: true`) — a lapsed
    // subscription is `ok: false`, so the page never paints it as an in-step account.
    let vault = temp("gated-sync-status");
    knowlu_engine::sync::record_gated_skip(&vault, "sync (skipped: no entitlement)").expect("status saved");
    let status = knowlu_engine::sync::load_status(&vault);
    assert!(!status.ok, "a lapsed subscription must read distinctly from an ordinary skip");
    assert_eq!(status.skipped.as_deref(), Some("no entitlement"));
    assert_eq!(status.lines, vec!["sync (skipped: no entitlement)".to_string()]);
    let _ = std::fs::remove_dir_all(&vault);
}

#[test]
fn a_gated_sync_composes_no_line_of_its_own() {
    // Fix round 1, M2: the reason string is `main.rs`'s alone to compose ("composed here, and
    // here only" — H4b's own words). Passing a distinctive line through `record_gated_skip` and
    // finding that exact line, rather than a hard-coded one, on disk is what would catch a second
    // copy of the sentence drifting from the first.
    let vault = temp("gated-sync-status-line");
    knowlu_engine::sync::record_gated_skip(&vault, "sync (a made-up line for this test)").expect("status saved");
    let status = knowlu_engine::sync::load_status(&vault);
    assert_eq!(status.lines, vec!["sync (a made-up line for this test)".to_string()]);
    let _ = std::fs::remove_dir_all(&vault);
}

#[test]
fn a_gated_skip_writes_nothing_when_another_sync_holds_the_lock() {
    // Fix round 1, M1: `save_status`'s single fixed temp name is safe only because every other
    // writer holds `RunLock` first (`state/sync.lock`) — *Sync now* runs in-process and holds it
    // for the whole of its own call. Without this, a slot's gated `sync` racing *Sync now* could
    // clobber a result that just succeeded, or lose its own write to a `rename` collision.
    let vault = temp("locked-gate");
    let lock_path = vault.join(knowlu_engine::sync::RUN_LOCK_FILE);
    std::fs::create_dir_all(lock_path.parent().unwrap()).unwrap();
    // This test process holds the lock first — two different `File` handles to the same path,
    // which is what `try_lock` actually serialises against, exactly as `sync_contract.rs`'s own
    // `i1_a_run_that_finds_the_lock_held_skips_and_writes_nothing` proves for `run_lines`.
    let held = std::fs::OpenOptions::new().create(true).write(true).open(&lock_path).expect("open the lock file");
    held.try_lock().expect("this test process holds it first");
    knowlu_engine::sync::record_gated_skip(&vault, "sync (skipped: no entitlement)").expect("must not error just because the lock is held");
    assert!(
        !vault.join(knowlu_engine::sync::STATUS_FILE).exists(),
        "a lock held elsewhere must mean nothing is written, not even the status file"
    );
    drop(held);
    let _ = std::fs::remove_dir_all(&vault);
}

#[test]
fn a_spawned_binary_refuses_each_gated_command_at_exit_0_with_the_named_line() {
    // Fix round 1, I4: the four tests above only ever check that `main.rs`'s SOURCE contains the
    // right substrings — every one of these edits would have survived them all: `ExitCode::SUCCESS`
    // -> `FAILURE`, `record_gated_skip` moved outside the `Sync` guard or after the `println!`, or
    // the whole gate block deleted. Only running the real, built `knowlu-engine.exe` catches those.
    //
    // `.env("LOCALAPPDATA", &root)` points the child at this temp root without needing a real
    // profile — `oracle.rs` already spawns this same binary from this crate's tests, and the gate
    // returns before any Credential Manager or network call, since it precedes every match arm.
    for cmd in ["coursework", "ingest", "judge", "sync"] {
        let root = temp(&format!("spawn-{cmd}"));
        let vault = root.join("vault");
        cloud_yaml(&vault, "knowlu/profile_0a1b2c3d4e/session");
        cache(&root, "profile_0a1b2c3d4e", "canceled", "2026-09-17T06:00:00Z");
        let out = Command::new(binary())
            .no_console()
            .env("LOCALAPPDATA", &root)
            .args([cmd, "--vault", vault.to_str().expect("temp path is UTF-8")])
            .output()
            .expect("run knowlu-engine");
        assert!(out.status.success(), "{cmd}: exit {:?}, stderr {}", out.status.code(), String::from_utf8_lossy(&out.stderr));
        let stdout = String::from_utf8_lossy(&out.stdout);
        assert_eq!(stdout.trim_end(), format!("{cmd} (skipped: no entitlement)"), "{cmd}: {stdout:?}");
        if cmd == "sync" {
            let status = knowlu_engine::sync::load_status(&vault);
            assert_eq!(status.skipped.as_deref(), Some("no entitlement"), "{cmd}");
            assert!(!status.ok, "{cmd}");
        }
        // Fix round 1, M3: the same skip lands in the vault's own runner log, the same way
        // `coursework::main`'s own routine skips do, with a routine `ok` status.
        let log = std::fs::read_to_string(vault.join("state").join("runner-log.md")).expect("state/runner-log.md");
        assert!(log.contains(&format!("{cmd} (skipped: no entitlement)")), "{cmd}: {log}");
        let _ = std::fs::remove_dir_all(&root);
    }
}

fn temp(name: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!("knowlu-entitlement-gate-{name}-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).expect("mkdir");
    dir
}
