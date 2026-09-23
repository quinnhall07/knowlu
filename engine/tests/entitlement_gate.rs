//! Ruling 3 of the cloud design's amendment of 2026-09-17, from the outside.
//!
//! No test here reads a real profile: `entitle`'s two `_in` functions take the app-data root, and
//! every case passes a temp directory. No test here touches Credential Manager — the gate is a file
//! and a clock, deliberately, because an entitlement check that needed a credential would be a
//! second thing to fail on a machine that is merely signed out.
use std::path::{Path, PathBuf};

use knowlu_engine::entitle;

fn cache(root: &Path, profile: &str, status: &str, checked_at: &str) {
    let dir = root.join("knowlu").join("profiles").join(profile);
    std::fs::create_dir_all(&dir).expect("mkdir");
    std::fs::write(
        dir.join("entitlement.json"),
        format!(r#"{{"status":"{status}","current_period_end":null,"plan":"monthly","checked_at":"{checked_at}"}}"#),
    ).expect("write");
}

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
fn the_engines_grace_is_the_apps_grace_and_the_path_is_the_apps_path() {
    // **Two constants in two crates, pinned from one side.** The app writes the file; the engine
    // reads it; a grace that drifted would mean the page said "your subscription lapsed" on a
    // different day from the one the slots stopped. Read as text, not linked: `knowlu-engine` does
    // not depend on `knowlu` and must not start.
    //
    // **Two files, not one** (review I2): the grace and the cache filename are `account.rs`'s
    // (`:479`, `:485`), and the app-data root is `state.rs`'s `app_data_root_in` (`:190`). Reading
    // `account.rs` for all three is how the first draft of this test failed on a line that was never
    // there, so each assertion names the file it is about.
    let app = Path::new(env!("CARGO_MANIFEST_DIR")).join("..").join("app").join("src");
    let account = std::fs::read_to_string(app.join("account.rs")).expect("app/src/account.rs");
    let state = std::fs::read_to_string(app.join("state.rs")).expect("app/src/state.rs");
    assert!(account.contains("from_secs(72 * 60 * 60)"), "app/src/account.rs: the app's grace moved; move the engine's with it");
    assert_eq!(entitle::GRACE_SECONDS, 72 * 60 * 60);
    assert!(account.contains(r#"data_dir.join("entitlement.json")"#), "app/src/account.rs: the cache filename moved");
    assert!(state.contains(r#"base.join("knowlu")"#), "app/src/state.rs: app_data_root_in moved");
}

#[test]
fn the_profile_id_comes_out_of_the_credential_target_and_nowhere_else() {
    assert_eq!(entitle::profile_id("knowlu/profile_0a1b2c3d4e/session"), Some("profile_0a1b2c3d4e"));
    for bad in ["", "knowlu/session", "knowlu//session", "profile_1", "knowlu/p/extra/session"] {
        assert_eq!(entitle::profile_id(bad), None, "{bad}");
    }
}

#[test]
fn a_gated_vault_past_the_grace_gets_one_named_line_and_that_line_is_all_the_caller_needs() {
    let root = temp("gated");
    let vault = root.join("vault");
    std::fs::create_dir_all(vault.join("config")).expect("mkdir");
    std::fs::write(
        vault.join("config").join("cloud.yaml"),
        "api_base: 'https://example.invalid/functions/v1'\nanon_key: 'anon'\nsession_credential_target: 'knowlu/profile_0a1b2c3d4e/session'\naccount_id: 'acct-1'\n",
    ).expect("cloud.yaml");
    cache(&root, "profile_0a1b2c3d4e", "canceled", "2026-09-17T06:00:00Z");
    let reason = entitle::gate_in(&root, &vault, "2026-09-17T12:00:00Z".parse().unwrap()).expect("gated");
    assert_eq!(reason, "skipped: no entitlement");
    // And the reason is a word, never a path or a status code from Stripe.
    assert!(!reason.contains("profile_") && !reason.contains("canceled"));
    let _ = std::fs::remove_dir_all(&root);
}

#[test]
fn the_line_a_student_reads_names_the_step_and_the_reason() {
    // **The one student-visible artefact of ruling 3, asserted** (review I3). `entitle::gate` answers
    // the reason; `main.rs`'s `name_of` supplies the word; the composed line is what lands in the
    // Runs view, and it has to read like the two skips the app already prints. Asserted against the
    // hand-off's own source rather than by spawning the binary, because a spawn would need a real
    // `LOCALAPPDATA` profile and this suite deliberately has none.
    let main = std::fs::read_to_string(
        Path::new(env!("CARGO_MANIFEST_DIR")).join("src").join("main.rs"),
    ).expect("engine/src/main.rs");
    assert!(main.contains(r#"println!("{} ({reason})", name_of(&cli.command));"#), "H4b composes the line");
    for word in ["\"coursework\"", "\"ingest\"", "\"judge\"", "\"sync\""] {
        assert!(main.contains(word), "name_of must answer for {word}");
    }
    // And the shape itself, spelled out once so a reader knows what to look for on the page.
    assert_eq!(format!("{} ({})", "sync", "skipped: no entitlement"), "sync (skipped: no entitlement)");
}

fn temp(name: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!("knowlu-entitlement-gate-{name}-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).expect("mkdir");
    dir
}
