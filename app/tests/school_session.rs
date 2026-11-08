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
fn an_existing_install_keeps_its_old_folder_until_the_school_folder_exists() {
    let d = temp("profile-legacy");
    std::fs::create_dir_all(d.join("lms-session")).unwrap(); // today's grades folder, signed in
    assert_eq!(profile_for(Feature::Grades, Some(&row()), &d), Some(d.join("lms-session")), "old folder kept");
    std::fs::create_dir_all(session_dir(&d)).unwrap();
    assert_eq!(profile_for(Feature::Grades, Some(&row()), &d), Some(session_dir(&d)), "the new folder once it exists");
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
