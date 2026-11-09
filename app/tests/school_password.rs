//! P1t, the saved school password (onboarding replan §7.4 and §8 P1t; makes P1a and P1b pass).
//! Every test but one runs over the injected store and file seam in `support/school_fakes.rs`, which
//! record each call in order. The one real-credential test (signed off by the controller for exactly
//! this pattern) holds the file-scoped `CREDMAN_LOCK`, uses a generated profile id and a synthetic
//! secret, and cleans up with a `Drop` guard. Invented usernames, passwords and ids only.
#![cfg(windows)]
use knowlu::school_password::{
    forget_with, gate_with, read_with, reject_with, retarget_with, save_with, signed_in_with, target, FileMarks, Gate,
    RealStore, SchoolMarks, SchoolSecret,
};
use knowlu::school_session::try_take;
use std::path::{Path, PathBuf};

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
    assert_eq!(target(A), knowlu::credentials::target_for(A, knowlu::account::SCHOOL_PASSWORD_SOURCE));
    assert_eq!(target(A), t(A));
    assert_ne!(target(A), target(B));
}

// ---- save -------------------------------------------------------------------------------------

#[test]
fn a_save_writes_through_the_injected_store_under_the_profiles_own_target() {
    let f = Fake::new(A);
    let reply = save_with(&f, &f, A, USER, PW);
    assert_eq!(reply["ok"], true, "{reply}");
    assert_eq!(reply["busy"], false, "{reply}");
    assert_eq!(f.entries.borrow().get(&t(A)), Some(&(USER.to_string(), PW.to_string())));
    assert_eq!(f.entries.borrow().len(), 1, "only this profile's target is written");
}

#[test]
fn a_save_writes_the_credential_first_then_clears_both_marks() {
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
    // The old mark then blocks the new password, which fails safe: the student is asked again.
    let f = Fake::new(A).failing(&["clear_rejected"]);
    f.rejected.set(true);
    let reply = save_with(&f, &f, A, USER, PW);
    assert!(at(&f.log, "write:").is_some(), "the credential was written first");
    assert!(f.rejected.get(), "the mark that could not be cleared is still set");
    assert_eq!(reply["ok"], false, "the student must hear it did not take: {reply}");
}

#[test]
fn a_save_reply_holds_no_secret_even_when_the_stores_own_error_does() {
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
    let s = SchoolSecret::new(USER, PW);
    assert!(!format!("{s:?}").contains(PW), "SchoolSecret's Debug must redact: {s:?}");
    assert!(!format!("{s:#?}").contains(PW));
    let g = Gate::Fill(s);
    assert!(!format!("{g:?}").contains(PW), "Gate's Debug must not carry it: {g:?}");
}

// ---- read, and the lock -----------------------------------------------------------------------

#[test]
fn a_saved_password_reads_back_inside_the_process_only_under_the_lock() {
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

#[test]
fn a_save_while_a_fill_holds_the_lock_answers_busy_and_writes_nothing() {
    let f = Fake::new(A);
    f.rejected.set(true);
    f.in_flight.set(true);
    let held = try_take(A).expect("lock free");
    let reply = save_with(&f, &f, A, USER, PW);
    assert_eq!(reply["busy"], true, "{reply}");
    assert_eq!(reply["ok"], false, "{reply}");
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
    let f = Fake::new(A);
    let _ = save_with(&f, &f, A, USER, PW);
    assert!(try_take(A).is_some(), "the save's own lock is dropped");
}

// ---- the gate: marks and secret, read under the lock, checked only before a fill ---------------

#[test]
fn the_gate_reads_both_marks_and_the_secret_only_under_the_lock() {
    let f = Fake::new(A).with_entry(&t(A), USER, PW);
    let guard = try_take(A).expect("lock free");
    let _ = gate_with(&f, &f, A, &guard).expect("gate");
    assert!(!f.log.borrow().is_empty(), "the gate looked at something");
    assert!(f.unlocked.borrow().is_empty(), "{:?}", f.unlocked.borrow());
}

#[test]
fn with_a_password_and_no_mark_the_gate_gives_the_secret_to_fill() {
    let f = Fake::new(A).with_entry(&t(A), USER, PW);
    let guard = try_take(A).unwrap();
    match gate_with(&f, &f, A, &guard).expect("gate") {
        Gate::Fill(s) => assert_eq!((s.username.as_str(), s.password()), (USER, PW)),
        other => panic!("expected Fill, got {other:?}"),
    }
}

#[test]
fn with_nothing_saved_and_no_mark_the_gate_is_todays_path() {
    let f = Fake::new(A);
    let guard = try_take(A).unwrap();
    assert!(matches!(gate_with(&f, &f, A, &guard).expect("gate"), Gate::NoPassword));
}

#[test]
fn the_rejected_mark_with_no_password_saved_gives_the_form_never_todays_path() {
    let f = Fake::new(A);
    f.rejected.set(true);
    let guard = try_take(A).unwrap();
    assert!(matches!(gate_with(&f, &f, A, &guard).expect("gate"), Gate::Form));
}

#[test]
fn the_rejected_mark_blocks_a_saved_password_too() {
    let f = Fake::new(A).with_entry(&t(A), USER, PW);
    f.rejected.set(true);
    let guard = try_take(A).unwrap();
    assert!(matches!(gate_with(&f, &f, A, &guard).expect("gate"), Gate::Form));
    assert_eq!(count(&f.log, "write:"), 0);
}

#[test]
fn an_in_flight_mark_from_an_earlier_attempt_makes_a_fill_unconfirmed_and_stays_set() {
    let f = Fake::new(A).with_entry(&t(A), USER, PW);
    f.in_flight.set(true);
    let guard = try_take(A).unwrap();
    assert!(matches!(gate_with(&f, &f, A, &guard).expect("gate"), Gate::Unconfirmed));
    assert!(f.in_flight.get(), "the gate never clears it");
    assert_eq!(count(&f.log, "clear_"), 0);
}

#[test]
fn reading_the_secret_never_looks_at_the_in_flight_mark() {
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

#[test]
fn a_rejection_sets_the_rejected_mark_first_then_deletes_the_saved_password() {
    let f = Fake::new(A).with_entry(&t(A), USER, PW);
    reject_with(&f, &f, A).expect("both steps ran");
    assert!(at(&f.log, "set_rejected").unwrap() < at(&f.log, "delete:").unwrap(), "{:?}", f.log.borrow());
    assert!(f.rejected.get());
    assert!(f.entries.borrow().is_empty());
}

#[test]
fn if_the_delete_fails_the_mark_still_blocks_every_fill() {
    let f = Fake::new(A).with_entry(&t(A), USER, PW).failing(&["delete"]);
    assert!(reject_with(&f, &f, A).is_err(), "the failed delete is reported");
    assert!(f.rejected.get());
    let guard = try_take(A).unwrap();
    assert!(matches!(gate_with(&f, &f, A, &guard).expect("gate"), Gate::Form));
}

#[test]
fn if_the_mark_cannot_be_written_the_delete_still_runs() {
    let f = Fake::new(A).with_entry(&t(A), USER, PW).failing(&["set_rejected"]);
    assert!(reject_with(&f, &f, A).is_err(), "the failed mark is reported");
    assert!(f.entries.borrow().is_empty(), "the password was deleted anyway");
    assert!(at(&f.log, "delete:").is_some());
}

// ---- only a new save clears the rejected mark -----------------------------------------------------

#[test]
fn a_visible_sign_in_that_reaches_users_me_clears_the_in_flight_mark_and_never_the_rejected_one() {
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
    let f = Fake::new(A).with_entry(&t(A), USER, PW);
    f.rejected.set(true);
    f.in_flight.set(true);
    forget_with(&f, &f, A).expect("forgotten");
    let d = at(&f.log, "delete:").unwrap();
    assert!(d < at(&f.log, "clear_rejected").unwrap() && d < at(&f.log, "clear_in_flight").unwrap());
    assert!(!f.rejected.get() && !f.in_flight.get() && f.entries.borrow().is_empty());
}

#[test]
fn forget_of_a_credential_already_gone_is_ok_and_still_clears_the_marks() {
    let f = Fake::new(A);
    f.rejected.set(true);
    forget_with(&f, &f, A).expect("already gone is success");
    assert!(!f.rejected.get());
}

#[test]
fn a_failed_forget_keeps_the_marks_and_reports_the_failure() {
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

// ---- retarget_credentials ---------------------------------------------------------------------------------

#[test]
fn retarget_writes_the_new_school_target_before_it_deletes_the_old() {
    let f = Fake::new(A).with_entry(&t(A), USER, PW);
    assert_eq!(retarget_with(&f, &f, A, B).expect("moved"), 1);
    assert!(at(&f.log, &format!("write:{}", t(B))).unwrap() < at(&f.log, &format!("delete:{}", t(A))).unwrap());
    assert_eq!(f.entries.borrow().get(&t(B)), Some(&(USER.to_string(), PW.to_string())));
    assert!(!f.entries.borrow().contains_key(&t(A)));
}

#[test]
fn a_failed_new_write_leaves_the_old_credential_in_place() {
    let f = Fake::new(A).with_entry(&t(A), USER, PW).failing(&["write"]);
    assert!(retarget_with(&f, &f, A, B).is_err());
    assert!(f.entries.borrow().contains_key(&t(A)), "never delete before the new one is safe");
}

#[test]
fn retarget_with_nothing_saved_moves_nothing_and_is_not_a_failure() {
    let f = Fake::new(A);
    assert_eq!(retarget_with(&f, &f, A, B).expect("ok"), 0);
    assert_eq!(count(&f.log, "write:"), 0);
}

#[test]
fn a_rejected_password_is_never_moved_and_that_is_not_a_failure() {
    let f = Fake::new(A).with_entry(&t(A), USER, PW);
    f.rejected.set(true);
    assert_eq!(retarget_with(&f, &f, A, B).expect("not a failure"), 0);
    assert!(f.entries.borrow().is_empty(), "the old credential is deleted and the new target gets nothing");
    assert_eq!(count(&f.log, &format!("write:{}", t(B))), 0);
}

#[test]
fn an_in_flight_password_is_never_moved_either() {
    let f = Fake::new(A).with_entry(&t(A), USER, PW);
    f.in_flight.set(true);
    assert_eq!(retarget_with(&f, &f, A, B).expect("not a failure"), 0);
    assert!(f.entries.borrow().is_empty());
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
    let _ = std::fs::remove_dir_all(&marks_dir);
}
