#![cfg(windows)]
use knowlu::credentials::{delete, exists, target_for, write};

/// Deletes the credential on drop, including on a panicking assertion — so a failing test never
/// leaves `knowlu/test/<id>` behind in Credential Manager. The delete's own result is ignored:
/// by the time this runs the entry is usually already gone (the happy path deletes it explicitly
/// below), and a missing-target error here must never turn a passing test into a panicking one.
struct Cleanup(String);

impl Drop for Cleanup {
    fn drop(&mut self) {
        let _ = delete(&self.0);
    }
}

/// Round-trips one credential through the app's writer and the ENGINE'S reader — the whole point
/// of the module is that `src/wincred.rs` keeps reading, unchanged, what the app now writes.
///
/// The target and the secret are minted at runtime and the target is deleted at the end: no secret
/// and no fixed target ever appears in this repo (spec §5).
#[test]
fn a_credential_written_by_the_app_is_read_by_the_engine_and_then_deleted() {
    let target = format!("knowlu/test/{}", knowlu_engine::ids::new_id("t"));
    let _cleanup = Cleanup(target.clone());
    let user = "knowlu-test@example.invalid";
    let secret = knowlu_engine::ids::new_id("s");

    assert!(!exists(&target), "the throwaway target must not already exist");
    write(&target, user, &secret).expect("write");
    assert!(exists(&target));

    let got = knowlu_engine::wincred::read_credential(&target).expect("the engine reads it");
    assert_eq!(got.username, user);
    assert_eq!(got.password.expose(), secret, "byte-for-byte — the UTF-16 blob-size trap");

    write(&target, user, "second-value-then-overwritten").expect("rewrite is an update, not a duplicate");
    assert_eq!(knowlu_engine::wincred::read_credential(&target).unwrap().password.expose(), "second-value-then-overwritten");

    delete(&target).expect("delete");
    assert!(!exists(&target));
    assert!(knowlu_engine::wincred::read_credential(&target).is_err(), "gone for the engine too");
    assert!(delete(&target).is_err(), "deleting twice is an error, not a silent success");
}

#[test]
fn targets_are_namespaced_per_profile_so_two_profiles_never_share_a_login() {
    assert_eq!(target_for("profile_0123456789", "zybooks"), "knowlu/profile_0123456789/zybooks");
    assert_ne!(target_for("profile_aaaaaaaaaa", "vhl"), target_for("profile_bbbbbbbbbb", "vhl"));
}
