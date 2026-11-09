//! S1: `school_session`, the LMS's one persistent sign-in folder (onboarding plan §S1, replan rows S1
//! and S2). The folder belongs to the LMS alone (RQ4): its uses are grades and the course list. It
//! holds no password (that is Credential Manager, P1). Invented data only; every folder is a scratch
//! directory under `target/`; no window, no Credential Manager, no network.
//!
//! Test 5 (the lock) is per profile and non-blocking; no test here takes `grades::Capturing` except
//! `a_busy_window_guard_answers_busy_and_drops_the_session_guard`, because that guard is one static.
use knowlu::grades::{CaptureError, Capturing};
use knowlu::scaffold::{Curated, CAMPUSES};
use knowlu::school_session::{
    add_use, clear_use, forget, is_session_dir, profile_for, session_dir, sign_in_host_ok, take_both, try_take, uses,
    wrong_password_text_ok, Feature, Use,
};
use std::path::{Path, PathBuf};
use std::sync::mpsc;
use std::time::Duration;

/// A scratch profile folder under `target/` (absolute, so the guard accepts it), one per test tag.
fn temp(tag: &str) -> PathBuf {
    let base = Path::new(env!("CARGO_TARGET_TMPDIR")).join(format!("school-session-{tag}-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&base);
    std::fs::create_dir_all(&base).unwrap();
    base
}

/// Make a kept folder with one file in it, as a signed-in window would leave.
fn make_folder(d: &Path) {
    std::fs::create_dir_all(session_dir(d).join("Default").join("Network")).unwrap();
    std::fs::write(session_dir(d).join("Default").join("Network").join("Cookies"), "synthetic").unwrap();
}

/// A synthetic school, never a real one.
fn row() -> Curated {
    Curated { unitid: "999999", lms_host: "lms.example.test", ..CAMPUSES[0] }
}

/// Run `f` on a thread and fail if it has not answered in two seconds: "neither call blocks".
fn within_two_seconds<T: Send + 'static>(f: impl FnOnce() -> T + Send + 'static) -> T {
    let (tx, rx) = mpsc::channel();
    std::thread::spawn(move || {
        let _ = tx.send(f());
    });
    rx.recv_timeout(Duration::from_secs(2)).expect("the call blocked")
}

// ---- 1. the folder and its guard -------------------------------------------------------------

#[test]
fn session_dir_is_school_session_and_is_session_dir_refuses_anything_else() {
    let d = temp("dir");
    assert_eq!(session_dir(&d), d.join("school-session"));
    assert!(is_session_dir(&d, &session_dir(&d)));
    let other = temp("dir-other");
    for wrong in [
        d.clone(),
        d.parent().unwrap().to_path_buf(),
        d.join("school-session").join("Default"), // a grandchild
        d.join("school-session-2"),               // a sibling
        d.join("lms-session"),
        d.join("SCHOOL-SESSION"),
        d.join("school-session").join("..").join("school-session"),
        other.join("school-session"), // another profile's folder
    ] {
        assert!(!is_session_dir(&d, &wrong), "{}", wrong.display());
    }
    assert!(!is_session_dir(Path::new("relative"), &session_dir(Path::new("relative"))), "a relative profile");
    let dotted = d.join("x").join("..");
    assert!(!is_session_dir(&dotted, &session_dir(&dotted)), "a `..` in the profile path");
}

// ---- 2. forget -------------------------------------------------------------------------------

#[test]
fn forget_deletes_only_the_folder_and_is_done_when_there_is_none() {
    let d = temp("forget");
    make_folder(&d);
    std::fs::write(d.join("settings.json"), "{}").unwrap();
    std::fs::create_dir_all(d.join("school-session-keep")).unwrap();
    assert_eq!(forget(&d), Ok(()));
    assert!(!session_dir(&d).exists());
    for kept in ["settings.json", "school-session-keep"] {
        assert!(d.join(kept).exists(), "{kept} survives forget");
    }
    assert_eq!(forget(&d), Ok(()), "nothing to forget is done, not a failure");
}

#[test]
fn forget_refuses_a_path_its_guard_refuses() {
    assert!(forget(Path::new("relative")).is_err(), "a relative profile");
    let d = temp("forget-dotdot");
    let dotted = d.join("x").join("..");
    std::fs::create_dir_all(d.join("x")).unwrap();
    make_folder(&d);
    assert!(forget(&dotted).is_err(), "a `..` in the profile path");
    assert!(session_dir(&d).exists(), "the refused call deleted nothing");
}

#[cfg(windows)]
fn hold_open(file: &Path) -> std::fs::File {
    use std::os::windows::fs::OpenOptionsExt;
    // share mode 1 is FILE_SHARE_READ: a delete of the file or its folder fails while this lives.
    std::fs::OpenOptions::new().read(true).share_mode(1).open(file).unwrap()
}

#[cfg(windows)]
#[test]
fn forget_is_patient_with_a_file_held_open_for_a_moment() {
    let d = temp("forget-patient");
    make_folder(&d);
    let file = session_dir(&d).join("Default").join("Network").join("Cookies");
    let held = hold_open(&file);
    assert!(std::fs::remove_dir_all(session_dir(&d)).is_err(), "the hold works: a plain delete loses");
    let release = std::thread::spawn(move || {
        std::thread::sleep(Duration::from_millis(400)); // past the first try, well inside twelve
        drop(held);
    });
    assert_eq!(forget(&d), Ok(()), "it survives the failed attempt, then the folder goes");
    release.join().unwrap();
    assert!(!session_dir(&d).exists());
}

#[cfg(windows)]
#[test]
fn forget_reports_a_delete_it_could_not_finish() {
    let d = temp("forget-stuck");
    make_folder(&d);
    let held = hold_open(&session_dir(&d).join("Default").join("Network").join("Cookies"));
    assert!(forget(&d).is_err(), "a folder still held after the last try is an error, never Ok");
    drop(held);
    assert!(session_dir(&d).exists(), "and it is still there");
    assert_eq!(forget(&d), Ok(()), "once released, the next forget finishes");
}

// ---- 3. uses (school.json) -------------------------------------------------------------------

#[test]
fn uses_persist_in_school_json_beside_the_folder_not_inside_it() {
    let d = temp("uses");
    assert_eq!(uses(&d), Ok(vec![]), "no school.json is no uses");
    add_use(&d, Use::Grades).unwrap();
    add_use(&d, Use::CourseList).unwrap();
    add_use(&d, Use::Grades).unwrap(); // twice is once
    assert!(d.join("school.json").exists());
    assert!(!session_dir(&d).join("school.json").exists(), "the folder is deleted wholesale; the uses are not in it");
    let mut got = uses(&d).unwrap();
    got.sort();
    assert_eq!(got, vec![Use::Grades, Use::CourseList]);
    let text = std::fs::read_to_string(d.join("school.json")).unwrap();
    for word in ["password", "token", "cookie"] {
        assert!(!text.to_lowercase().contains(word), "school.json holds uses only, found {word}");
    }
}

#[test]
fn clear_use_of_one_of_two_keeps_the_folder_and_of_the_last_deletes_it() {
    let d = temp("clear");
    make_folder(&d);
    add_use(&d, Use::Grades).unwrap();
    add_use(&d, Use::CourseList).unwrap();
    clear_use(&d, Use::Grades).unwrap();
    assert_eq!(uses(&d), Ok(vec![Use::CourseList]));
    assert!(session_dir(&d).exists(), "one use left keeps the folder");
    clear_use(&d, Use::CourseList).unwrap();
    assert_eq!(uses(&d), Ok(vec![]));
    assert!(!session_dir(&d).exists(), "the last use gone deletes the folder");
}

#[test]
fn forget_clears_every_use_and_deletes_the_folder() {
    let d = temp("forget-uses");
    make_folder(&d);
    add_use(&d, Use::Grades).unwrap();
    add_use(&d, Use::CourseList).unwrap();
    assert_eq!(forget(&d), Ok(()));
    assert_eq!(uses(&d), Ok(vec![]));
    assert!(!session_dir(&d).exists());
}

#[test]
fn an_unreadable_school_json_is_an_error_never_no_uses() {
    for (i, bad) in ["{not json", "", "[1, 2", r#"{"uses": "grades"}"#, r#"{"uses": ["not-a-use"]}"#].iter().enumerate() {
        let d = temp(&format!("unreadable-{i}"));
        make_folder(&d);
        std::fs::write(d.join("school.json"), bad).unwrap();
        assert!(uses(&d).is_err(), "{bad:?}");
        assert!(add_use(&d, Use::Grades).is_err(), "{bad:?}: a write would overwrite what it could not read");
        assert!(clear_use(&d, Use::Grades).is_err(), "{bad:?}");
        assert!(session_dir(&d).exists(), "{bad:?}: an unreadable record deletes nothing");
        assert_eq!(std::fs::read_to_string(d.join("school.json")).unwrap(), *bad, "and is left as found");
    }
}

// ---- 4. profile_for --------------------------------------------------------------------------

#[test]
fn grades_and_the_lms_capture_get_the_school_folder() {
    let d = temp("profile");
    for feature in [Feature::Grades, Feature::LmsCapture] {
        assert_eq!(profile_for(feature, Some(&row()), &d), Some(session_dir(&d)), "{feature:?}");
    }
}

#[test]
fn the_registrar_and_the_school_mailbox_never_get_the_school_folder() {
    let d = temp("profile-others");
    make_folder(&d); // even with the folder there
    for feature in [Feature::Registrar, Feature::SchoolMail] {
        assert_ne!(profile_for(feature, Some(&row()), &d), Some(session_dir(&d)), "{feature:?}");
        assert_ne!(profile_for(feature, None, &d), Some(session_dir(&d)), "{feature:?}");
    }
}

#[test]
fn an_existing_install_keeps_its_old_folder_until_grades_use_of_the_school_folder_is_recorded() {
    let d = temp("profile-legacy");
    std::fs::create_dir_all(d.join("lms-session")).unwrap(); // today's grades folder, signed in
    assert_eq!(profile_for(Feature::Grades, Some(&row()), &d), Some(d.join("lms-session")), "old folder kept");
    std::fs::create_dir_all(session_dir(&d)).unwrap();
    // S2's review: a bare school folder (a capture window opened and closed unsigned) moves nothing.
    assert_eq!(profile_for(Feature::Grades, Some(&row()), &d), Some(d.join("lms-session")), "mere existence moves nothing");
}

// ---- 5. the lock: per profile, non-blocking --------------------------------------------------

#[test]
fn the_lock_is_per_profile_and_never_blocks() {
    let first = within_two_seconds(|| try_take("lock-profile-a")).expect("a free profile is taken");
    assert!(within_two_seconds(|| try_take("lock-profile-a")).is_none(), "the same profile is refused, not waited on");
    let other = within_two_seconds(|| try_take("lock-profile-b")).expect("another profile succeeds beside it");
    assert!(within_two_seconds(|| try_take("lock-profile-b")).is_none(), "and holds its own lock");
    drop(first);
    let again = within_two_seconds(|| try_take("lock-profile-a")).expect("dropping the guard frees the profile");
    assert!(within_two_seconds(|| try_take("lock-profile-b")).is_none(), "dropping one never freed the other");
    drop(other);
    drop(again);
    assert!(within_two_seconds(|| try_take("lock-profile-b")).is_some(), "freed once dropped");
}

#[test]
fn a_busy_window_guard_answers_busy_and_drops_the_session_guard() {
    let profile = "lock-both";
    // Both free: both are held, and the profile's session lock is then busy.
    let both = take_both(profile).ok().expect("both free");
    assert!(try_take(profile).is_none(), "the session guard is held");
    assert!(Capturing::try_take().is_none(), "and so is the window guard");
    drop(both);
    // The window guard busy: Busy, and the session guard is not left held.
    let capturing = Capturing::try_take().expect("the window guard is free");
    assert!(matches!(take_both(profile), Err(CaptureError::Busy)), "the window guard busy answers Busy");
    let session = try_take(profile).expect("the first guard was dropped on the way out");
    drop(session);
    drop(capturing);
    // The session guard busy: Busy, and the window guard is not left held.
    let session = try_take(profile).unwrap();
    assert!(matches!(take_both(profile), Err(CaptureError::Busy)), "the session lock busy answers Busy");
    assert!(Capturing::try_take().is_some(), "the window guard was never taken");
    drop(session);
}

// ---- the campus fields (replan S2) -----------------------------------------------------------

#[test]
fn sign_in_hosts_are_bare_host_names() {
    for good in ["login.example.test", "sso.school.example.test", "idp1.example.test"] {
        assert!(sign_in_host_ok(good), "{good}");
    }
    for bad in [
        "", "https://login.example.test", "http://login.example.test", "login.example.test/path", "login.example.test/",
        "login.example.test:8443", "login.example.test?x=1", "login.example.test#f", "user@login.example.test",
        " login.example.test", "login .example.test", "//login.example.test",
    ] {
        assert!(!sign_in_host_ok(bad), "{bad:?}");
    }
}

#[test]
fn wrong_password_text_holds_no_address_and_no_id() {
    for good in ["", "The username or password you entered is incorrect.", "Sign-in failed. Try again."] {
        assert!(wrong_password_text_ok(good), "{good:?}");
    }
    for bad in [
        "Wrong password for jane@example.test",
        "@",
        "Account 12345678 not found",
        "123456",
        "ID 42",
        "Try again 2",
    ] {
        assert!(!wrong_password_text_ok(bad), "{bad:?}");
    }
    assert!(wrong_password_text_ok("Code A1B2 is not valid"), "a token with letters is not digits-only");
}

#[test]
fn every_curated_row_passes_its_own_rules_and_ships_the_wrong_password_text_empty() {
    for c in CAMPUSES.iter() {
        for h in c.sign_in_hosts {
            assert!(sign_in_host_ok(h), "{}: {h}", c.unitid);
        }
        assert!(wrong_password_text_ok(c.wrong_password_text), "{}", c.unitid);
        assert_eq!(c.wrong_password_text, "", "{}: ships empty unless cited by date (replan S2)", c.unitid);
    }
}

// ---- S6t: Settings' one school sign-in row (`school_status`, `school_forget`) ----------------------
//
// The two Tauri commands are thin wrappers over `status_with` and `school_forget_with`, which take the
// row, the credential store, the marks and the window close as inputs. Fakes only: no window, no
// Credential Manager. Each Forget test uses its own profile id, because the lock is one process-wide set.
use knowlu::account::SCHOOL_WINDOWS;
use knowlu::school_password::{gate_with, target, Gate};
use knowlu::school_session::{school_forget_with, status_with};
use serde_json::Value;

#[path = "support/school_fakes.rs"]
mod school_fakes;
use school_fakes::{at, Fake};

fn named_row() -> Curated {
    Curated { label: "Example State University", ..row() }
}

#[test]
fn school_status_reports_the_uses_the_folder_and_the_schools_label() {
    let d = temp("status-full");
    make_folder(&d);
    add_use(&d, Use::Grades).unwrap();
    add_use(&d, Use::CourseList).unwrap();
    let r = named_row();
    let s = status_with(&d, Some(&r));
    assert_eq!(s["uses"], serde_json::json!(["grades", "course_list"]), "the uses, in the order written: {s}");
    assert_eq!(s["folder"], serde_json::json!(true), "the folder exists: {s}");
    assert_eq!(s["label"], serde_json::json!("Example State University"), "the school label: {s}");
}

#[test]
fn school_status_with_no_folder_no_uses_and_no_row_says_so() {
    let d = temp("status-empty");
    let s = status_with(&d, None);
    assert_eq!(s["uses"], serde_json::json!([]), "{s}");
    assert_eq!(s["folder"], serde_json::json!(false), "{s}");
    assert_eq!(s["label"], Value::Null, "no curated row, no label: {s}");
}

#[test]
fn school_status_never_carries_a_path_a_cookie_or_a_host() {
    let d = temp("status-private");
    make_folder(&d);
    add_use(&d, Use::Grades).unwrap();
    let r = named_row();
    let text = status_with(&d, Some(&r)).to_string();
    assert!(text.contains("grades"), "the check must be reading a real answer: {text}");
    let dir = d.to_string_lossy().to_string();
    for needle in [dir.as_str(), "school-session", "school.json", r.lms_host, "Cookies", "cookie", "synthetic", "://"] {
        assert!(!text.contains(needle), "status must not carry {needle:?}: {text}");
    }
    assert!(!text.contains('\\') && !text.contains('/'), "no path separator at all: {text}");
}

/// A scratch profile with the folder, both uses, a saved password and both marks set.
fn forget_setup(tag: &str, profile: &str) -> (PathBuf, Fake) {
    let d = temp(tag);
    make_folder(&d);
    add_use(&d, Use::Grades).unwrap();
    add_use(&d, Use::CourseList).unwrap();
    let fake = Fake::new(profile).with_entry(&target(profile), "student-one", "hunter2-synthetic-pw");
    fake.rejected.set(true);
    fake.in_flight.set(true);
    (d, fake)
}

#[test]
fn school_forget_closes_both_school_windows_by_label_before_it_deletes_anything() {
    let profile = "profile_s6forget1";
    let (d, fake) = forget_setup("forget-close", profile);
    let mut closed: Vec<String> = Vec::new();
    let log = fake.log.clone();
    let reply = school_forget_with(&d, profile, &fake, &fake, &mut |label| {
        closed.push(label.to_string());
        log.borrow_mut().push(format!("close:{label}"));
    });
    assert_eq!(reply["ok"], serde_json::json!(true), "{reply}");
    closed.sort();
    assert_eq!(closed, vec!["lms-grades".to_string(), "lms-signin".to_string()], "both windows on the school folders");
    assert_eq!(SCHOOL_WINDOWS.len(), 2, "and they are sign-out's two");
    let last_close = log.borrow().iter().rposition(|e| e.starts_with("close:")).expect("a close");
    let first_delete = at(&log, "delete:").expect("the password delete");
    assert!(last_close < first_delete, "windows close before the password is deleted: {:?}", log.borrow());
    assert!(!session_dir(&d).exists(), "the folder is gone");
}

#[test]
fn school_forget_deletes_the_password_and_the_rejected_mark_and_clears_every_use() {
    let profile = "profile_s6forget2";
    let (d, fake) = forget_setup("forget-all", profile);
    let reply = school_forget_with(&d, profile, &fake, &fake, &mut |_| {});
    // Answering ok, and not Busy, proves P1's `forget_with` ran before this call took its own guard.
    assert_eq!(reply["ok"], serde_json::json!(true), "forget_with takes the profile lock itself: {reply}");
    assert!(fake.entries.borrow().is_empty(), "the saved password is deleted");
    assert!(!fake.rejected.get(), "the rejected mark is cleared by a delete that succeeded");
    assert_eq!(uses(&d), Ok(vec![]), "every use is cleared");
    assert!(!session_dir(&d).exists(), "the folder is deleted");
    assert!(!d.join("school.json").exists(), "and the record of uses");
    assert!(try_take(profile).is_some(), "no guard is left held");
}

#[test]
fn school_forget_while_the_profile_is_busy_answers_not_ok_and_touches_nothing() {
    let profile = "profile_s6forget3";
    let (d, fake) = forget_setup("forget-busy", profile);
    let _other = try_take(profile).expect("free");
    let busy = school_forget_with(&d, profile, &fake, &fake, &mut |_| {});
    assert_eq!(busy["ok"], serde_json::json!(false), "{busy}");
    assert!(fake.entries.borrow().contains_key(&target(profile)), "the password is untouched");
    assert!(session_dir(&d).exists(), "the folder is untouched");
}

#[test]
fn school_forget_reports_a_failed_password_delete_without_the_secret() {
    let profile = "profile_s6forget4";
    let (d, fake) = forget_setup("forget-pwfail", profile);
    let fake = fake.failing(&["delete"]);
    let reply = school_forget_with(&d, profile, &fake, &fake, &mut |_| {});
    assert_eq!(reply["ok"], serde_json::json!(false), "a failed delete is reported: {reply}");
    assert!(!reply.to_string().contains("hunter2-synthetic-pw"), "never the secret: {reply}");
    assert!(fake.entries.borrow().contains_key(&target(profile)), "the password is still there, and the reply says so");
    assert!(fake.rejected.get(), "a failed delete clears no mark");
}

#[test]
fn school_forget_reports_a_failed_folder_delete_and_has_already_deleted_the_password() {
    let profile = "profile_s6forget5";
    let fake = Fake::new(profile).with_entry(&target(profile), "student-one", "hunter2-synthetic-pw");
    // A relative data_dir is refused by `is_session_dir`, so the folder delete fails at once and the
    // same way every time, with nothing to wait on.
    let rel = PathBuf::from("s6t-relative-profile-dir");
    assert!(!rel.is_absolute());
    let reply = school_forget_with(&rel, profile, &fake, &fake, &mut |_| {});
    assert_eq!(reply["ok"], serde_json::json!(false), "a failed folder delete is reported: {reply}");
    assert!(fake.entries.borrow().is_empty(), "the password goes first, so a stuck folder never leaves one that signs back in");
}

#[test]
fn after_forget_the_next_grades_slot_finds_no_saved_password_and_fills_nothing() {
    let profile = "profile_s6forget6";
    let (d, fake) = forget_setup("forget-gate", profile);
    // Before: a saved password and no marks is a fill.
    fake.rejected.set(false);
    fake.in_flight.set(false);
    {
        let guard = try_take(profile).unwrap();
        match gate_with(&fake, &fake, profile, &guard).unwrap() {
            Gate::Fill(s) => assert_eq!(s.password(), "hunter2-synthetic-pw"),
            other => panic!("before Forget the gate fills: {other:?}"),
        }
    }
    let reply = school_forget_with(&d, profile, &fake, &fake, &mut |_| {});
    assert_eq!(reply["ok"], serde_json::json!(true), "{reply}");
    let guard = try_take(profile).expect("the slot's own lock is free again");
    match gate_with(&fake, &fake, profile, &guard).unwrap() {
        Gate::NoPassword => {}
        other => panic!("after Forget the slot finds no password and fills nothing, got {other:?}"),
    }
}

// ---- S6a rulings (coordinator, 2026-11-08): the legacy folder counts, and Forget clears the mark -----

#[test]
fn school_status_counts_the_legacy_lms_session_folder_as_a_kept_sign_in() {
    let d = temp("status-legacy");
    std::fs::create_dir_all(d.join("lms-session").join("Default")).unwrap();
    let s = status_with(&d, None);
    assert_eq!(s["folder"], serde_json::json!(true), "an old install signed in through lms-session is offered Forget: {s}");
    assert_eq!(s["uses"], serde_json::json!([]), "{s}");
    let text = s.to_string();
    assert!(!text.contains("lms-session") && !text.contains('\\') && !text.contains('/'), "still no path: {text}");
}

#[test]
fn school_forget_deletes_the_legacy_folder_and_clears_grades_signed_out_mark() {
    let profile = "profile_s6forget7";
    let (d, fake) = forget_setup("forget-legacy-mark", profile);
    std::fs::create_dir_all(d.join("lms-session").join("Default")).unwrap();
    knowlu::grades::set_signed_out(&d, true);
    assert!(d.join("grades-signed-out").is_file(), "the mark is set before Forget");
    let reply = school_forget_with(&d, profile, &fake, &fake, &mut |_| {});
    assert_eq!(reply["ok"], serde_json::json!(true), "{reply}");
    assert!(!d.join("lms-session").exists(), "the legacy folder is gone");
    assert!(!d.join("grades-signed-out").exists(), "grades' signed-out mark is cleared, as grades' own Forget does");
    assert_eq!(status_with(&d, None)["folder"], serde_json::json!(false), "nothing kept afterwards");
}

// ---- review of 2ea88a20..36114ae5: Forget waits a bounded time for a window's lock (Important 2, Minor 3) ----
//
// The course-list window holds the profile lock until its Destroyed event runs on the event loop, so a
// Forget that has just destroyed the windows retries Busy every 250 ms, eight times (two seconds). The
// wait is injected, so no test sleeps.
use knowlu::school_session::{school_forget_retrying, FORGET_RETRIES};
use std::cell::Cell;

#[test]
fn school_forget_retries_a_busy_lock_until_the_window_lets_go() {
    let profile = "profile_s6retry1";
    let (d, fake) = forget_setup("forget-retry", profile);
    let held = std::cell::RefCell::new(Some(try_take(profile).expect("free")));
    let pauses = Cell::new(0u32);
    let mut closed = 0;
    let reply = school_forget_retrying(
        &d,
        profile,
        &fake,
        &fake,
        &mut |_| closed += 1,
        &mut || {
            pauses.set(pauses.get() + 1);
            if pauses.get() == 3 {
                held.borrow_mut().take(); // the Destroyed event ran: the window's guard drops
            }
        },
        &mut |id| try_take(id),
    );
    assert_eq!(reply["ok"], serde_json::json!(true), "the fourth call, after the third pause, finds the lock free: {reply}");
    assert_eq!(pauses.get(), 3, "it waited only as long as it had to");
    assert_eq!(closed, 2, "the windows were closed once, not again on each try");
    assert!(fake.entries.borrow().is_empty() && !session_dir(&d).exists(), "everything was forgotten");
}

#[test]
fn school_forget_gives_up_after_two_seconds_of_busy_with_nothing_deleted() {
    let profile = "profile_s6retry2";
    let (d, fake) = forget_setup("forget-giveup", profile);
    let _held = try_take(profile).expect("free");
    let pauses = Cell::new(0u32);
    let reply = school_forget_retrying(&d, profile, &fake, &fake, &mut |_| {}, &mut || pauses.set(pauses.get() + 1), &mut |id| try_take(id));
    assert_eq!(reply["ok"], serde_json::json!(false), "{reply}");
    assert_eq!(reply["error"], serde_json::json!(knowlu::school_password::BUSY), "plainly Busy: {reply}");
    assert_eq!(pauses.get(), FORGET_RETRIES, "a bounded wait");
    assert_eq!(FORGET_RETRIES, 8, "eight pauses of 250 ms is two seconds");
    assert!(fake.entries.borrow().contains_key(&target(profile)), "the password is untouched");
    assert!(session_dir(&d).exists(), "the folder is untouched");
}

#[test]
fn school_forget_says_the_password_is_forgotten_when_only_the_folders_lock_stays_busy() {
    let profile = "profile_s6retry3";
    let (d, fake) = forget_setup("forget-second-busy", profile);
    let pauses = Cell::new(0u32);
    // P1's forget_with takes its own lock and succeeds; the guard for the folder is then never free.
    let reply = school_forget_retrying(&d, profile, &fake, &fake, &mut |_| {}, &mut || pauses.set(pauses.get() + 1), &mut |_| None);
    assert_eq!(reply["ok"], serde_json::json!(false), "{reply}");
    let error = reply["error"].as_str().expect("an error line").to_lowercase();
    assert!(error.contains("password") && error.contains("forgot"), "it says the password was forgotten: {reply}");
    assert!(error.contains("in use"), "and that the sign-in is in use right now: {reply}");
    assert_ne!(reply["error"], serde_json::json!(knowlu::school_password::BUSY), "not a plain Busy: {reply}");
    assert_eq!(pauses.get(), FORGET_RETRIES, "the same bounded wait");
    assert!(fake.entries.borrow().is_empty(), "the password really is gone");
    assert!(session_dir(&d).exists(), "the folder is still there");
}
