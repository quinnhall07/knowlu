//! P1t, the saved school password (onboarding replan §7.4 and §8 P1t; makes P1a and P1b pass).
//! Every test but one runs over the injected store and file seam in `support/school_fakes.rs`, which
//! record each call in order. The one real-credential test (signed off by the controller for exactly
//! this pattern) holds the file-scoped `CREDMAN_LOCK`, uses a generated profile id and a synthetic
//! secret, and cleans up with a `Drop` guard. Invented usernames, passwords and ids only.
#![cfg(windows)]
use knowlu::school_password::{
    forget_with, gate_with, read_with, reject_with, retarget_with, save_with, signed_in_with, target, DeleteError,
    FileMarks, Gate, RealStore, SchoolMarks, SchoolSecret, SchoolStore, BAD_CHARACTER, BUSY, CLEAR_BOTH_FAILED,
    CLEAR_IN_FLIGHT_FAILED, CLEAR_REJECTED_FAILED, EMPTY_PASSWORD, EMPTY_USERNAME, PASSWORD_TOO_LONG,
    USERNAME_TOO_LONG,
};
use knowlu::school_session::try_take;
use std::path::{Path, PathBuf};

/// `school_session`'s lock is one process-wide set, and every test here takes profile `A`'s or `B`'s
/// (the fake probes it on each call). Test threads run at once, so each such test holds this first:
/// otherwise one test's held lock reads as another's `Busy`.
static SERIAL: std::sync::Mutex<()> = std::sync::Mutex::new(());
fn serial() -> std::sync::MutexGuard<'static, ()> {
    SERIAL.lock().unwrap_or_else(|e| e.into_inner())
}

#[path = "support/school_fakes.rs"]
mod school_fakes;
use school_fakes::{at, count, Fake};

#[path = "support/credman_lock.rs"]
mod credman_lock;
static CREDMAN_LOCK: credman_lock::CredmanLock = credman_lock::CredmanLock::new();

const PW: &str = "hunter2-synthetic-pw";
const USER: &str = "student-one";
const A: &str = "profile_aaa111";
const B: &str = "profile_bbb222";

fn t(id: &str) -> String {
    format!("knowlu/{id}/school")
}

// ---- the target -------------------------------------------------------------------------------

#[test]
fn the_target_is_built_by_target_for_and_two_profiles_never_share_one() {
    let _serial = serial();
    assert_eq!(target(A), knowlu::credentials::target_for(A, knowlu::account::SCHOOL_PASSWORD_SOURCE));
    assert_eq!(target(A), t(A));
    assert_ne!(target(A), target(B));
}

// ---- save -------------------------------------------------------------------------------------

#[test]
fn a_save_writes_through_the_injected_store_under_the_profiles_own_target() {
    let _serial = serial();
    let f = Fake::new(A);
    let reply = save_with(&f, &f, A, USER, PW);
    assert_eq!(reply["ok"], true, "{reply}");
    assert_eq!(reply["busy"], false, "{reply}");
    assert_eq!(f.entries.borrow().get(&t(A)), Some(&(USER.to_string(), PW.to_string())));
    assert_eq!(f.entries.borrow().len(), 1, "only this profile's target is written");
}

#[test]
fn a_save_writes_the_credential_first_then_clears_both_marks() {
    let _serial = serial();
    let f = Fake::new(A);
    f.rejected.set(true);
    f.in_flight.set(true);
    let reply = save_with(&f, &f, A, USER, PW);
    assert_eq!(reply["ok"], true, "{reply}");
    let w = at(&f.log, "write:").expect("credential written");
    assert!(w < at(&f.log, "clear_rejected").expect("rejected mark cleared"), "{:?}", f.log.borrow());
    assert!(w < at(&f.log, "clear_in_flight").expect("in-flight mark cleared"), "{:?}", f.log.borrow());
    assert!(!f.rejected.get() && !f.in_flight.get());
}

#[test]
fn a_failed_credential_write_clears_no_mark() {
    let _serial = serial();
    let f = Fake::new(A).failing(&["write"]);
    f.rejected.set(true);
    f.in_flight.set(true);
    let reply = save_with(&f, &f, A, USER, PW);
    assert_eq!(reply["ok"], false, "{reply}");
    assert!(f.rejected.get() && f.in_flight.get(), "the old marks stay when nothing was saved");
    assert_eq!(count(&f.log, "clear_"), 0, "{:?}", f.log.borrow());
}

#[test]
fn a_failed_mark_clear_keeps_the_old_mark_and_the_save_is_reported_not_ok() {
    let _serial = serial();
    // The old mark then blocks the new password, which fails safe: the student is asked again.
    let f = Fake::new(A).failing(&["clear_rejected"]);
    f.rejected.set(true);
    let reply = save_with(&f, &f, A, USER, PW);
    assert!(at(&f.log, "write:").is_some(), "the credential was written first");
    assert!(f.rejected.get(), "the mark that could not be cleared is still set");
    assert_eq!(reply["ok"], false, "the student must hear it did not take: {reply}");
}

/// Contract review r1 (bf8ab212..807ed978), Minor 7: each mark that could not be cleared names itself.
#[test]
fn each_mark_that_cannot_be_cleared_has_its_own_words() {
    let _serial = serial();
    let cases: [(&[&'static str], &str); 3] = [
        (&["clear_rejected"], CLEAR_REJECTED_FAILED),
        (&["clear_in_flight"], CLEAR_IN_FLIGHT_FAILED),
        (&["clear_rejected", "clear_in_flight"], CLEAR_BOTH_FAILED),
    ];
    for (failing, words) in cases {
        let f = Fake::new(A).failing(failing);
        f.rejected.set(true);
        f.in_flight.set(true);
        let reply = save_with(&f, &f, A, USER, PW);
        assert_eq!((&reply["ok"], &reply["error"]), (&serde_json::json!(false), &serde_json::json!(words)), "{failing:?}");
    }
    assert_ne!(CLEAR_REJECTED_FAILED, CLEAR_IN_FLIGHT_FAILED);
}

// ---- contract review r1, Important 1: a save refuses what the store cannot hold, before the lock -----

/// Refused by name, before the lock: no store call, no mark touched, so no rejection pause ends.
fn refused(reply: &serde_json::Value, reason: &str, f: &Fake) {
    assert_eq!((&reply["ok"], &reply["busy"]), (&serde_json::json!(false), &serde_json::json!(false)), "{reply}");
    assert_eq!(reply["error"], reason, "{reply}");
    assert!(f.log.borrow().is_empty(), "no store call and no mark touched: {:?}", f.log.borrow());
    assert!(f.rejected.get(), "an empty or bad save never ends a rejection pause");
}

fn marked() -> Fake {
    let f = Fake::new(A);
    f.rejected.set(true);
    f
}

#[test]
fn an_empty_or_blank_username_or_password_is_refused_and_ends_no_pause() {
    let _serial = serial();
    for (user, pw, reason) in [("", PW, EMPTY_USERNAME), (" \t", PW, EMPTY_USERNAME), (USER, "", EMPTY_PASSWORD), (USER, "  \r\n", EMPTY_PASSWORD)] {
        let f = marked();
        refused(&save_with(&f, &f, A, user, pw), reason, &f);
    }
}

#[test]
fn a_nul_in_either_field_is_refused() {
    let _serial = serial();
    for (user, pw) in [("student\0one", PW), (USER, "hunter\02"), (USER, "\0")] {
        let f = marked();
        refused(&save_with(&f, &f, A, user, pw), BAD_CHARACTER, &f);
    }
}

#[test]
fn values_over_the_stores_limits_are_too_long_never_a_failed_write() {
    let _serial = serial();
    // CRED_MAX_CREDENTIAL_BLOB_SIZE is 2560 bytes, 1280 UTF-16 units; CRED_MAX_USERNAME_LENGTH is 513.
    // Both are counted in UTF-16 units, as the store counts: one non-BMP character is two.
    let ok = |user: &str, pw: &str| {
        let f = Fake::new(A);
        save_with(&f, &f, A, user, pw)["ok"] == true
    };
    assert!(ok(USER, &"p".repeat(1280)) && ok(USER, &"\u{1F600}".repeat(640)) && ok(&"u".repeat(513), PW));
    let over: [(String, String, &str); 4] = [
        (USER.to_string(), "p".repeat(1281), PASSWORD_TOO_LONG),
        (USER.to_string(), "\u{1F600}".repeat(641), PASSWORD_TOO_LONG),
        ("u".repeat(514), PW.to_string(), USERNAME_TOO_LONG),
        ("\u{1F600}".repeat(257), PW.to_string(), USERNAME_TOO_LONG),
    ];
    for (user, pw, reason) in over {
        let f = marked();
        refused(&save_with(&f, &f, A, &user, &pw), reason, &f);
    }
}

#[test]
fn a_password_is_stored_exactly_as_typed_never_trimmed() {
    let _serial = serial();
    let f = Fake::new(A);
    assert_eq!(save_with(&f, &f, A, USER, "  spaced pw\t")["ok"], true);
    assert_eq!(f.entries.borrow().get(&t(A)).map(|e| e.1.as_str()), Some("  spaced pw\t"));
}

#[test]
fn a_refusal_comes_before_the_lock() {
    let _serial = serial();
    let f = marked();
    let held = try_take(A).expect("lock free");
    refused(&save_with(&f, &f, A, USER, ""), EMPTY_PASSWORD, &f);
    drop(held);
}

#[test]
fn a_save_reply_holds_no_secret_even_when_the_stores_own_error_does() {
    let _serial = serial();
    let f = Fake::new(A).failing(&["write"]);
    f.echo_secret.set(true);
    let reply = save_with(&f, &f, A, USER, PW);
    let text = reply.to_string();
    assert_eq!(reply["ok"], false, "{text}");
    assert!(!text.contains(PW), "the reply must not carry the password: {text}");
    let ok = save_with(&Fake::new(B), &Fake::new(B), B, USER, PW).to_string();
    assert!(!ok.contains(PW), "{ok}");
}

#[test]
fn a_secret_and_a_gate_holding_one_never_print_the_password() {
    let _serial = serial();
    let s = SchoolSecret::new(USER, PW);
    assert!(!format!("{s:?}").contains(PW), "SchoolSecret's Debug must redact: {s:?}");
    assert!(!format!("{s:#?}").contains(PW));
    let g = Gate::Fill(s);
    assert!(!format!("{g:?}").contains(PW), "Gate's Debug must not carry it: {g:?}");
}

// ---- read, and the lock -----------------------------------------------------------------------

#[test]
fn a_saved_password_reads_back_inside_the_process_only_under_the_lock() {
    let _serial = serial();
    let f = Fake::new(A).with_entry(&t(A), USER, PW);
    let guard = try_take(A).expect("lock free");
    let got = read_with(&f, A, &guard).expect("read").expect("a saved password");
    assert_eq!((got.username.as_str(), got.password()), (USER, PW));
    assert!(f.unlocked.borrow().is_empty(), "read while the lock was not held: {:?}", f.unlocked.borrow());
    let none = read_with(&Fake::new(A), A, &guard).expect("read");
    assert!(none.is_none(), "no saved password is None, not an error");
}

#[test]
fn no_tauri_command_returns_the_secret() {
    let src = std::fs::read_to_string(concat!(env!("CARGO_MANIFEST_DIR"), "/src/school_password.rs")).expect("source");
    let parts: Vec<&str> = src.split("#[tauri::command").collect();
    assert_eq!(parts.len(), 2, "exactly one Tauri command, the save, lives in school_password.rs");
    let body = parts[1].split("\n}\n").next().unwrap();
    for banned in ["read_with", "SchoolSecret", ".password()", "gate_with", "wincred"] {
        assert!(!body.contains(banned), "the command must not touch the secret ({banned})");
    }
    assert!(body.contains("save_with"), "the one command saves through save_with");
}

/// The controller's ruling (P1a): the lock, the target and the marks are all the profile folder's,
/// the key `grades` and `lms_link` already take the lock under, so a fill and a save exclude each
/// other; a folder whose name is not a profile id is refused, so sign-out can always delete the save.
#[test]
fn the_save_command_keys_its_lock_target_and_marks_to_the_profiles_own_folder() {
    let src = std::fs::read_to_string(concat!(env!("CARGO_MANIFEST_DIR"), "/src/school_password.rs")).expect("source");
    let body = src.split("#[tauri::command").nth(1).and_then(|b| b.split("\n}\n").next()).expect("the command");
    assert!(body.contains("school_session::profile_key(&cs.data_dir)"), "{body}");
    assert!(body.contains("FileMarks::new(&cs.data_dir)"), "the marks sit beside school.json: {body}");
    assert!(body.contains("is_profile_id("), "{body}");
}

#[test]
fn a_guard_for_another_profile_reads_no_secret() {
    let _serial = serial();
    let f = Fake::new(A).with_entry(&t(A), USER, PW);
    let other = try_take(B).expect("lock free");
    assert!(read_with(&f, A, &other).is_err(), "A's secret is read only while A's lock is held");
    assert!(gate_with(&f, &f, A, &other).is_err(), "and so are A's marks");
    assert!(f.log.borrow().is_empty(), "{:?}", f.log.borrow());
}

/// Contract review r1, Minor 3: the read checks the guard it is handed, never a probe of the lock.
/// With A's lock held by someone else, B's guard still reads nothing of A's.
#[test]
fn a_guard_for_another_profile_reads_nothing_even_while_that_profile_is_held_elsewhere() {
    let _serial = serial();
    let f = Fake::new(A).with_entry(&t(A), USER, PW);
    let _a_elsewhere = try_take(A).expect("lock free");
    let b = try_take(B).expect("lock free");
    assert!(read_with(&f, A, &b).is_err());
    assert!(gate_with(&f, &f, A, &b).is_err());
    assert!(f.log.borrow().is_empty(), "{:?}", f.log.borrow());
}

#[test]
fn a_session_guard_names_the_profile_it_holds() {
    let _serial = serial();
    assert_eq!(try_take(A).expect("lock free").profile_id(), A);
}

#[test]
fn a_save_while_a_fill_holds_the_lock_answers_busy_and_writes_nothing() {
    let _serial = serial();
    let f = Fake::new(A);
    f.rejected.set(true);
    f.in_flight.set(true);
    let held = try_take(A).expect("lock free");
    let reply = save_with(&f, &f, A, USER, PW);
    assert_eq!(reply["busy"], true, "{reply}");
    assert_eq!(reply["ok"], false, "{reply}");
    assert_eq!(reply["error"], BUSY, "{reply}");
    assert!(f.log.borrow().is_empty(), "no store call and no mark touched: {:?}", f.log.borrow());
    assert!(f.rejected.get() && f.in_flight.get(), "no mark is cleared under a running fill");
    // Another profile's lock is its own.
    let other = Fake::new(B);
    assert_eq!(save_with(&other, &other, B, USER, PW)["ok"], true);
    drop(held);
    assert_eq!(save_with(&f, &f, A, USER, PW)["ok"], true, "the lock is free again");
}

#[test]
fn a_save_frees_the_lock_when_it_returns() {
    let _serial = serial();
    let f = Fake::new(A);
    let _ = save_with(&f, &f, A, USER, PW);
    assert!(try_take(A).is_some(), "the save's own lock is dropped");
}

// ---- the gate: marks and secret, read under the lock, checked only before a fill ---------------

#[test]
fn the_gate_reads_both_marks_and_the_secret_only_under_the_lock() {
    let _serial = serial();
    let f = Fake::new(A).with_entry(&t(A), USER, PW);
    let guard = try_take(A).expect("lock free");
    let _ = gate_with(&f, &f, A, &guard).expect("gate");
    assert!(!f.log.borrow().is_empty(), "the gate looked at something");
    assert!(f.unlocked.borrow().is_empty(), "{:?}", f.unlocked.borrow());
}

#[test]
fn with_a_password_and_no_mark_the_gate_gives_the_secret_to_fill() {
    let _serial = serial();
    let f = Fake::new(A).with_entry(&t(A), USER, PW);
    let guard = try_take(A).unwrap();
    match gate_with(&f, &f, A, &guard).expect("gate") {
        Gate::Fill(s) => assert_eq!((s.username.as_str(), s.password()), (USER, PW)),
        other => panic!("expected Fill, got {other:?}"),
    }
}

#[test]
fn with_nothing_saved_and_no_mark_the_gate_is_todays_path() {
    let _serial = serial();
    let f = Fake::new(A);
    let guard = try_take(A).unwrap();
    assert!(matches!(gate_with(&f, &f, A, &guard).expect("gate"), Gate::NoPassword));
}

#[test]
fn the_rejected_mark_with_no_password_saved_gives_the_form_never_todays_path() {
    let _serial = serial();
    let f = Fake::new(A);
    f.rejected.set(true);
    let guard = try_take(A).unwrap();
    assert!(matches!(gate_with(&f, &f, A, &guard).expect("gate"), Gate::Form));
}

#[test]
fn the_rejected_mark_blocks_a_saved_password_too() {
    let _serial = serial();
    let f = Fake::new(A).with_entry(&t(A), USER, PW);
    f.rejected.set(true);
    let guard = try_take(A).unwrap();
    assert!(matches!(gate_with(&f, &f, A, &guard).expect("gate"), Gate::Form));
    assert_eq!(count(&f.log, "write:"), 0);
}

#[test]
fn an_in_flight_mark_from_an_earlier_attempt_makes_a_fill_unconfirmed_and_stays_set() {
    let _serial = serial();
    let f = Fake::new(A).with_entry(&t(A), USER, PW);
    f.in_flight.set(true);
    let guard = try_take(A).unwrap();
    assert!(matches!(gate_with(&f, &f, A, &guard).expect("gate"), Gate::Unconfirmed));
    assert!(f.in_flight.get(), "the gate never clears it");
    assert_eq!(count(&f.log, "clear_"), 0);
}

#[test]
fn reading_the_secret_never_looks_at_the_in_flight_mark() {
    let _serial = serial();
    // The mark is checked only before a fill: a read whose session is live still reads grades, and
    // the mark stays.
    let f = Fake::new(A).with_entry(&t(A), USER, PW);
    f.in_flight.set(true);
    let guard = try_take(A).unwrap();
    let _ = read_with(&f, A, &guard).expect("read");
    assert_eq!(count(&f.log, "in_flight?"), 0, "{:?}", f.log.borrow());
    assert_eq!(count(&f.log, "clear_"), 0);
    assert!(f.in_flight.get());
}

// ---- rejection: mark first, then delete ---------------------------------------------------------
// The controller's ruling (P1b): a rejection happens only inside a fill that already holds the lock,
// so `reject_with` is handed that `&SessionGuard`; these calls are the compile-level pin.

#[test]
fn a_rejection_sets_the_rejected_mark_first_then_deletes_the_saved_password() {
    let _serial = serial();
    let f = Fake::new(A).with_entry(&t(A), USER, PW);
    let guard = try_take(A).expect("lock free");
    reject_with(&f, &f, A, &guard).expect("both steps ran");
    assert!(at(&f.log, "set_rejected").unwrap() < at(&f.log, "delete:").unwrap(), "{:?}", f.log.borrow());
    assert!(f.rejected.get());
    assert!(f.entries.borrow().is_empty());
    assert!(f.unlocked.borrow().is_empty(), "the rejection ran under the fill's lock: {:?}", f.unlocked.borrow());
}

#[test]
fn if_the_delete_fails_the_mark_still_blocks_every_fill() {
    let _serial = serial();
    let f = Fake::new(A).with_entry(&t(A), USER, PW).failing(&["delete"]);
    let guard = try_take(A).unwrap();
    assert!(reject_with(&f, &f, A, &guard).is_err(), "the failed delete is reported");
    assert!(f.rejected.get());
    assert!(matches!(gate_with(&f, &f, A, &guard).expect("gate"), Gate::Form));
}

#[test]
fn if_the_mark_cannot_be_written_the_delete_still_runs() {
    let _serial = serial();
    let f = Fake::new(A).with_entry(&t(A), USER, PW).failing(&["set_rejected"]);
    let guard = try_take(A).unwrap();
    assert!(reject_with(&f, &f, A, &guard).is_err(), "the failed mark is reported");
    assert!(f.entries.borrow().is_empty(), "the password was deleted anyway");
    assert!(at(&f.log, "delete:").is_some());
}

/// Refusing a rejection would leave a rejected password saved, ready to be sent again, so the guard
/// is never a reason to skip it: unlike a read, a rejection still marks and deletes.
#[test]
fn a_rejection_handed_another_profiles_guard_still_marks_and_deletes() {
    let _serial = serial();
    let f = Fake::new(A).with_entry(&t(A), USER, PW);
    let other = try_take(B).expect("lock free");
    reject_with(&f, &f, A, &other).expect("both steps ran");
    assert!(f.rejected.get() && f.entries.borrow().is_empty());
}

#[test]
fn a_rejection_with_no_password_saved_still_sets_the_mark_and_is_ok() {
    // The rejected mark set with no password saved (§7.4's table): nothing is left to delete.
    let _serial = serial();
    let f = Fake::new(A);
    let guard = try_take(A).unwrap();
    reject_with(&f, &f, A, &guard).expect("already gone is not a failure");
    assert!(f.rejected.get());
}

// ---- only a new save clears the rejected mark -----------------------------------------------------

#[test]
fn a_visible_sign_in_that_reaches_users_me_clears_the_in_flight_mark_and_never_the_rejected_one() {
    let _serial = serial();
    let f = Fake::new(A);
    f.rejected.set(true);
    f.in_flight.set(true);
    signed_in_with(&f).expect("clear");
    assert!(f.rejected.get(), "only a new save ends the pause");
    assert!(!f.in_flight.get());
    assert_eq!(count(&f.log, "clear_rejected"), 0);
}

// ---- forget: delete first, clear the marks only if it is gone ---------------------------------------

#[test]
fn forget_deletes_the_credential_first_then_clears_both_marks() {
    let _serial = serial();
    let f = Fake::new(A).with_entry(&t(A), USER, PW);
    f.rejected.set(true);
    f.in_flight.set(true);
    forget_with(&f, &f, A).expect("forgotten");
    let d = at(&f.log, "delete:").unwrap();
    assert!(d < at(&f.log, "clear_rejected").unwrap() && d < at(&f.log, "clear_in_flight").unwrap());
    assert!(!f.rejected.get() && !f.in_flight.get() && f.entries.borrow().is_empty());
}

/// Contract review r1 (bf8ab212..807ed978), Minor 2, as the coordinator revised it: "already gone" is
/// the store's own not-found, never `credentials::exists`, which reads false on any read error (and a
/// spurious not-found happens under load). So a not-found delete clears the in-flight mark only and
/// keeps the rejected one, and the gate then shows the form. Both marks clear only after a delete that
/// itself succeeded (the test above).
#[test]
fn forget_of_a_credential_already_gone_is_ok_clears_the_in_flight_mark_and_keeps_the_rejected_one() {
    let _serial = serial();
    let f = Fake::new(A);
    f.rejected.set(true);
    f.in_flight.set(true);
    forget_with(&f, &f, A).expect("already gone is success");
    assert!(f.rejected.get(), "a not-found delete keeps the rejected mark");
    assert!(!f.in_flight.get());
    assert_eq!(count(&f.log, "exists:"), 0, "never decided from exists: {:?}", f.log.borrow());
}

/// The same review: a delete that failed is never "already gone", whatever `exists` would say.
#[test]
fn a_failed_delete_is_never_read_as_already_gone() {
    let _serial = serial();
    let f = Fake::new(A).failing(&["delete"]); // nothing saved, so `exists` would answer gone
    f.rejected.set(true);
    f.in_flight.set(true);
    assert!(forget_with(&f, &f, A).is_err(), "a failed delete is not ok");
    assert!(f.rejected.get() && f.in_flight.get());
    assert_eq!(count(&f.log, "clear_"), 0, "{:?}", f.log.borrow());
    let r = Fake::new(A).failing(&["delete"]);
    let guard = try_take(A).unwrap();
    assert!(reject_with(&r, &r, A, &guard).is_err(), "a rejection reports it too");
    assert!(r.rejected.get());
}

/// The controller's ruling (P1a): Forget clears the marks, so it takes the lock a fill holds, and a
/// Forget during a fill answers `Busy` and touches nothing (no mark cleared under a running fill).
#[test]
fn forget_while_a_fill_holds_the_lock_answers_busy_and_touches_nothing() {
    let _serial = serial();
    let f = Fake::new(A).with_entry(&t(A), USER, PW);
    f.rejected.set(true);
    f.in_flight.set(true);
    let held = try_take(A).expect("lock free");
    assert_eq!(forget_with(&f, &f, A), Err(BUSY.to_string()));
    assert!(f.log.borrow().is_empty(), "no store call and no mark touched: {:?}", f.log.borrow());
    assert!(f.rejected.get() && f.in_flight.get() && f.entries.borrow().contains_key(&t(A)));
    drop(held);
    forget_with(&f, &f, A).expect("the lock is free again");
    assert!(try_take(A).is_some(), "forget's own lock is dropped");
}

#[test]
fn a_failed_forget_keeps_the_marks_and_reports_the_failure() {
    let _serial = serial();
    let f = Fake::new(A).with_entry(&t(A), USER, PW).failing(&["delete"]);
    f.rejected.set(true);
    f.in_flight.set(true);
    assert!(forget_with(&f, &f, A).is_err());
    assert!(f.rejected.get() && f.in_flight.get(), "the marks stay while the credential does");
    assert_eq!(count(&f.log, "clear_"), 0, "{:?}", f.log.borrow());
}

// ---- the marks on disk (P1b): the profile's app data, never the vault ------------------------------------

fn scratch(tag: &str) -> PathBuf {
    let d = Path::new(env!("CARGO_TARGET_TMPDIR")).join(format!("school-marks-{tag}-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&d);
    std::fs::create_dir_all(&d).unwrap();
    d
}

#[test]
fn the_file_marks_set_and_clear_and_outlive_the_value_that_wrote_them() {
    let d = scratch("roundtrip");
    let m = FileMarks::new(&d);
    assert!(!m.rejected() && !m.in_flight());
    m.set_rejected().unwrap();
    m.write_in_flight().unwrap();
    m.flush_in_flight().unwrap();
    let again = FileMarks::new(&d); // as after an app close
    assert!(again.rejected() && again.in_flight());
    again.clear_in_flight().unwrap();
    assert!(!FileMarks::new(&d).in_flight() && FileMarks::new(&d).rejected());
    again.clear_rejected().unwrap();
    assert!(!FileMarks::new(&d).rejected());
    // Clearing a mark that is not there is not a failure.
    again.clear_rejected().unwrap();
    // Another profile's folder never sees them.
    let other = scratch("roundtrip-other");
    m.set_rejected().unwrap();
    assert!(!FileMarks::new(&other).rejected());
    let _ = std::fs::remove_dir_all(&d);
    let _ = std::fs::remove_dir_all(&other);
}

/// The controller's ruling (P1b): the marks are the profile folder's own files, beside `school.json`
/// and never inside `school-session`, so a Forget of that folder never loses a rejected mark before
/// P1's forget clears it in its own order.
#[test]
fn the_marks_sit_beside_school_json_and_outlive_a_forget_of_the_school_folder() {
    let d = scratch("beside");
    std::fs::create_dir_all(knowlu::school_session::session_dir(&d)).unwrap();
    let m = FileMarks::new(&d);
    m.set_rejected().unwrap();
    m.write_in_flight().unwrap();
    m.flush_in_flight().unwrap();
    knowlu::school_session::forget(&d).expect("the school folder is forgotten");
    assert!(!knowlu::school_session::session_dir(&d).exists());
    assert!(m.rejected() && m.in_flight(), "the marks outlive the folder");
    for entry in std::fs::read_dir(&d).unwrap() {
        assert!(entry.unwrap().file_type().unwrap().is_file(), "each mark is a file directly in the profile folder");
    }
    let _ = std::fs::remove_dir_all(&d);
}

/// A mark is never written into a folder that is not there (one Delete my data removed, say): the
/// write fails, which a fill reads as `CannotRead` and sends nothing, and no folder is made.
#[test]
fn a_mark_into_a_missing_folder_fails_and_makes_no_folder() {
    let d = scratch("missing").join("gone");
    let m = FileMarks::new(&d);
    assert!(m.write_in_flight().is_err() && m.set_rejected().is_err());
    assert!(m.flush_in_flight().is_err(), "a flush with no written mark is a failure, never a success");
    assert!(!d.exists());
    assert!(!m.rejected() && !m.in_flight());
    let _ = std::fs::remove_dir_all(d.parent().unwrap());
}

// ---- retarget_credentials ---------------------------------------------------------------------------------

#[test]
fn retarget_writes_the_new_school_target_before_it_deletes_the_old() {
    let _serial = serial();
    let f = Fake::new(A).with_entry(&t(A), USER, PW);
    assert_eq!(retarget_with(&f, &f, &Fake::new(B), A, B).expect("moved"), 1);
    assert!(at(&f.log, &format!("write:{}", t(B))).unwrap() < at(&f.log, &format!("delete:{}", t(A))).unwrap());
    assert_eq!(f.entries.borrow().get(&t(B)), Some(&(USER.to_string(), PW.to_string())));
    assert!(!f.entries.borrow().contains_key(&t(A)));
}

#[test]
fn a_failed_new_write_leaves_the_old_credential_in_place() {
    let _serial = serial();
    let f = Fake::new(A).with_entry(&t(A), USER, PW).failing(&["write"]);
    assert!(retarget_with(&f, &f, &Fake::new(B), A, B).is_err());
    assert!(f.entries.borrow().contains_key(&t(A)), "never delete before the new one is safe");
}

#[test]
fn retarget_with_nothing_saved_moves_nothing_and_is_not_a_failure() {
    let _serial = serial();
    let f = Fake::new(A);
    assert_eq!(retarget_with(&f, &f, &Fake::new(B), A, B).expect("ok"), 0);
    assert_eq!(count(&f.log, "write:"), 0);
}

#[test]
fn a_rejected_password_is_never_moved_and_that_is_not_a_failure() {
    let _serial = serial();
    let f = Fake::new(A).with_entry(&t(A), USER, PW);
    f.rejected.set(true);
    assert_eq!(retarget_with(&f, &f, &Fake::new(B), A, B).expect("not a failure"), 0);
    assert!(f.entries.borrow().is_empty(), "the old credential is deleted and the new target gets nothing");
    assert_eq!(count(&f.log, &format!("write:{}", t(B))), 0);
}

#[test]
fn an_in_flight_password_is_never_moved_either() {
    let _serial = serial();
    let f = Fake::new(A).with_entry(&t(A), USER, PW);
    f.in_flight.set(true);
    assert_eq!(retarget_with(&f, &f, &Fake::new(B), A, B).expect("not a failure"), 0);
    assert!(f.entries.borrow().is_empty());
}

/// A move reads the old id's marks and secret, so it takes the old id's lock (§7.4: both marks and
/// the secret are read only under it); held, it is `BUSY` and touches nothing.
#[test]
fn a_retarget_while_the_old_ids_lock_is_held_is_busy_and_touches_nothing() {
    let _serial = serial();
    let f = Fake::new(A).with_entry(&t(A), USER, PW);
    let held = try_take(A).expect("lock free");
    assert_eq!(retarget_with(&f, &f, &Fake::new(B), A, B), Err(BUSY.to_string()));
    assert!(f.log.borrow().is_empty(), "{:?}", f.log.borrow());
    drop(held);
    assert!(try_take(A).is_some());
}

#[test]
fn a_retarget_onto_the_same_id_never_deletes_the_only_copy() {
    let _serial = serial();
    let f = Fake::new(A).with_entry(&t(A), USER, PW);
    assert_eq!(retarget_with(&f, &f, &f, A, A).expect("nothing to move"), 0);
    assert!(f.log.borrow().is_empty(), "{:?}", f.log.borrow());
    assert!(f.entries.borrow().contains_key(&t(A)));
}

/// Contract review r1, Minor 5: the rejected mark goes with the move, set at the new id before the old
/// password is deleted, so the new id shows the form; the password itself is never moved.
#[test]
fn a_rejected_passwords_mark_is_carried_to_the_new_id_and_the_password_is_not() {
    let _serial = serial();
    let f = Fake::new(A).with_entry(&t(A), USER, PW);
    f.rejected.set(true);
    let mut to = Fake::new(B);
    to.log = f.log.clone(); // one log, so the order across both is visible
    assert_eq!(retarget_with(&f, &f, &to, A, B).expect("not a failure"), 0);
    assert!(to.rejected.get(), "the new id carries the rejected mark");
    assert!(f.entries.borrow().is_empty() && count(&f.log, &format!("write:{}", t(B))) == 0);
    assert!(at(&f.log, "set_rejected").unwrap() < at(&f.log, &format!("delete:{}", t(A))).unwrap(), "{:?}", f.log.borrow());
}

#[test]
fn an_in_flight_mark_alone_is_not_carried() {
    let _serial = serial();
    let f = Fake::new(A).with_entry(&t(A), USER, PW);
    f.in_flight.set(true);
    let to = Fake::new(B);
    assert_eq!(retarget_with(&f, &f, &to, A, B).expect("not a failure"), 0);
    assert!(!to.rejected.get() && !to.in_flight.get() && to.log.borrow().is_empty());
    assert!(f.entries.borrow().is_empty());
}

#[test]
fn a_mark_that_cannot_be_carried_still_deletes_the_old_password_and_is_reported() {
    let _serial = serial();
    let f = Fake::new(A).with_entry(&t(A), USER, PW);
    f.rejected.set(true);
    let to = Fake::new(B).failing(&["set_rejected"]);
    assert!(retarget_with(&f, &f, &to, A, B).is_err());
    assert!(f.entries.borrow().is_empty(), "a rejected password is never left to move later");
}

/// The wiring: the move goes through `retarget_with`, and the marks it honours are the OLD id's, read
/// from that id's profile folder; a carried mark goes to the NEW id's. Both folders sit under the root
/// `main` launches under (contract review r1, Minor 4), so no unset `LOCALAPPDATA` refuses Finish.
#[test]
fn the_retarget_command_reads_the_old_ids_marks_from_its_profile_folder() {
    let src = std::fs::read_to_string(concat!(env!("CARGO_MANIFEST_DIR"), "/src/onboarding.rs")).expect("source");
    let body = src.split("pub fn retarget_credentials").nth(1).and_then(|b| b.split("\n}\n").next()).expect("the command");
    assert!(body.contains("school_password::retarget_with("), "{body}");
    assert!(body.contains("crate::state::app_data_root_or_temp()"), "{body}");
    assert!(!body.contains("no app data folder"), "{body}");
    assert!(body.contains("FileMarks::new(&profiles::profile_dir(&root, &from_id))"), "{body}");
    assert!(body.contains("profiles::profile_dir(&root, &to_id)"), "{body}");
}

/// Contract review r1, Minor 4: one rule for the root, shared with `main`.
#[test]
fn with_no_localappdata_the_root_is_mains_temp_fallback() {
    assert_eq!(knowlu::state::root_or_temp(None), std::env::temp_dir().join("knowlu"));
    let found = PathBuf::from(r"C:\example\knowlu");
    assert_eq!(knowlu::state::root_or_temp(Some(found.clone())), found);
    let main = std::fs::read_to_string(concat!(env!("CARGO_MANIFEST_DIR"), "/src/main.rs")).expect("main.rs");
    assert!(main.contains("app_data_root_or_temp()"), "main launches under the shared rule");
    assert!(!main.contains("temp_dir().join(\"knowlu\")"), "and keeps no copy of it");
}

#[test]
fn the_retarget_command_walks_school_with_zybooks_and_vhl() {
    let src = std::fs::read_to_string(concat!(env!("CARGO_MANIFEST_DIR"), "/src/onboarding.rs")).expect("source");
    let from = src.find("pub fn retarget_credentials").expect("the command");
    let body = &src[from..];
    let body = &body[..body.find("\n}\n").expect("end of fn")];
    assert!(
        body.contains("SCHOOL_PASSWORD_SOURCE") || body.contains("\"school\"") || body.contains("retarget_with"),
        "retarget_credentials must move the school password, honouring its marks"
    );
}

// ---- the one real-credential test --------------------------------------------------------------------------

/// Deletes the generated profile's real `school` target on every exit path.
struct Cleanup(String);
impl Drop for Cleanup {
    fn drop(&mut self) {
        let _ = knowlu::credentials::delete(&self.0);
    }
}

#[test]
fn save_read_and_forget_through_the_real_credential_manager() {
    let _credman_guard = CREDMAN_LOCK.lock().unwrap_or_else(|e| e.into_inner());
    // A generated, per-process id: never `knowlu/dev/*` and never a real profile's target.
    let id = format!("test-{}-{}", std::process::id(), line!());
    let target = target(&id);
    assert_eq!(target, format!("knowlu/{id}/school"));
    let _cleanup = Cleanup(target.clone());
    let marks_dir = scratch("real");
    let marks = FileMarks::new(&marks_dir);

    let reply = save_with(&RealStore, &marks, &id, USER, PW);
    assert_eq!(reply["ok"], true, "{reply}");
    assert!(!reply.to_string().contains(PW));
    assert!(knowlu::credentials::exists(&target), "saved at the profile's own target");

    let guard = try_take(&id).expect("lock free");
    let got = read_with(&RealStore, &id, &guard).expect("read").expect("saved");
    assert_eq!((got.username.as_str(), got.password()), (USER, PW));
    drop(guard);

    forget_with(&RealStore, &marks, &id).expect("forget");
    assert!(!knowlu::credentials::exists(&target), "gone after forget");
    let guard = try_take(&id).expect("lock free");
    assert!(read_with(&RealStore, &id, &guard).expect("read").is_none());
    // Contract review r1, Minor 2: the store's own not-found, mapped from `credentials::delete`'s
    // answer for a target that is not there, never from `exists`.
    assert_eq!(RealStore.delete(&target), Err(DeleteError::NotFound));
    let _ = std::fs::remove_dir_all(&marks_dir);
}
