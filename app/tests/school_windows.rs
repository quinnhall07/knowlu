//! S3: the windows on the shared LMS folder (onboarding plan §S3, as the replan's rows S3, S4a and S4b
//! change it). S4a (`grades.rs`) and S4b (`lms_link.rs`) make these pass. Invented hosts only; no window
//! is opened, no network is reached and no Credential Manager is touched; every folder is a scratch
//! directory under `target/`.
//!
//! What the replan took out: the registrar (`registrar.rs` stays on its incognito throwaway window, so
//! it is pinned here only as "never shares the profile"), the school mailbox (the mail lane keeps its
//! own `mail-session-<id>` folder and its own tests), test 4's navigation list and its fallback, and
//! test 7. What it added to test 4: P2's fill, in the LMS folder's hidden window, on a `sign_in_hosts`
//! host only, is the one named exception to "no initialization script outside the mail window".
//!
//! S4a's binding notes (from S2's review, 2026-11-08) are pinned in section 2: grades switch to the
//! school folder only on a recorded `Use::Grades`, and a grades window on the school folder records it.
use knowlu::grades::{capture_hidden_with, forget_with, read_session_with, refresh_with, status_for, CaptureError};
use knowlu::lms_link::{self, read_courses_with, read_feed_with, window_plan};
use knowlu::scaffold::{Curated, CAMPUSES};
use knowlu::school_session::{
    add_use, clear_use, grades_profile, is_session_dir, profile_for, session_dir, try_take, uses, window_args, Feature,
    JarCookie, Use, SCHOOL_BROWSER_ARGS,
};
use serde_json::json;
use std::cell::{Cell, RefCell};
use std::path::{Path, PathBuf};
use std::sync::Mutex;

/// `grades::Capturing` is one process-wide static: the tests that take it take turns.
static CAPTURING_TURN: Mutex<()> = Mutex::new(());

fn temp(tag: &str) -> PathBuf {
    let d = Path::new(env!("CARGO_TARGET_TMPDIR")).join(format!("school-windows-{tag}-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&d);
    std::fs::create_dir_all(&d).unwrap();
    d
}

/// A kept folder with one file in it, as a signed-in window leaves it.
fn make(dir: &Path) {
    std::fs::create_dir_all(dir.join("Default")).unwrap();
    std::fs::write(dir.join("Default").join("Cookies"), "synthetic").unwrap();
}

fn legacy(d: &Path) -> PathBuf {
    d.join("lms-session")
}

/// A synthetic Blackboard school; never a real one.
fn row() -> Curated {
    Curated { unitid: "999999", lms_host: "lms.example.test", lms_kind: "blackboard", ..CAMPUSES[0] }
}

fn src(file: &str) -> String {
    std::fs::read_to_string(file).unwrap_or_else(|e| panic!("{file}: {e}"))
}

// ---- 1. the one arguments constant (S3 test 2, as the replan keeps it) -----------------------

#[test]
fn the_school_arguments_are_the_mail_windows_arguments_and_carry_no_debug_port() {
    assert_eq!(SCHOOL_BROWSER_ARGS, knowlu::mail::MAIL_BROWSER_ARGS, "S-G recorded no change to the arguments");
    for banned in ["remote-debugging", "remote-allow-origins"] {
        assert!(!SCHOOL_BROWSER_ARGS.contains(banned), "{banned} must never be in the school arguments");
    }
}

#[test]
fn a_window_on_the_school_folder_gets_the_constant_whatever_debug_port_is_passed() {
    assert_eq!(window_args(None), SCHOOL_BROWSER_ARGS);
    assert_eq!(window_args(Some(9222)), SCHOOL_BROWSER_ARGS, "a port is never added to a school window");
    assert!(!window_args(Some(9222)).contains("9222"));
    // The environment variable is not read or set here: it is process-wide (review r2, M3).
    #[cfg(debug_assertions)]
    assert_ne!(window_args(Some(9222)), knowlu::debug_port::browser_args(9222), "a debug build's port is the mail window's alone");
}

#[test]
fn no_window_builder_in_grades_or_lms_link_builds_without_the_constant_or_with_a_port() {
    let grades = src("src/grades.rs");
    let builders = grades.matches(".data_directory(").count();
    assert!(builders >= 2, "grades builds a visible and a hidden window");
    assert_eq!(grades.matches(".additional_browser_args(").count(), builders, "every grades window sets the arguments");
    assert!(!grades.contains("remote-debugging") && !grades.contains("debug_port") && !grades.contains("KNOWLU_MAIL_DEBUG_PORT"));
    for (at, _) in grades.match_indices(".additional_browser_args(") {
        let call = &grades[at..grades[at..].find(')').map_or(grades.len(), |e| at + e)];
        assert!(call.contains("window_args") || call.contains("SCHOOL_BROWSER_ARGS"), "{call}");
    }
    let lms = src("src/lms_link.rs");
    assert!(lms.contains(".additional_browser_args("), "the LMS capture on the school folder sets the arguments");
    assert!(lms.contains("window_args") || lms.contains("SCHOOL_BROWSER_ARGS"));
    assert!(!lms.contains("remote-debugging") && !lms.contains("debug_port") && !lms.contains("KNOWLU_MAIL_DEBUG_PORT"));
}

// ---- 2. grades choose their folder (S3 tests 1 and 6; S4a's binding notes) --------------------

#[test]
fn grades_stay_on_lms_session_while_no_grades_use_is_recorded_even_if_the_school_folder_exists() {
    let d = temp("stay");
    make(&legacy(&d));
    make(&session_dir(&d)); // a capture window opened and closed unsigned left this
    assert_eq!(profile_for(Feature::Grades, Some(&row()), &d), Some(legacy(&d)), "mere existence moves nothing");
    add_use(&d, Use::CourseList).unwrap();
    assert_eq!(profile_for(Feature::Grades, Some(&row()), &d), Some(legacy(&d)), "the course list's use moves nothing");
    assert_eq!(grades_profile(&d), Ok(legacy(&d)));
    assert_eq!(uses(&d).unwrap(), vec![Use::CourseList], "grades on the old folder record no use of the school one");
}

#[test]
fn grades_move_to_the_school_folder_on_a_recorded_grades_use() {
    let d = temp("move");
    make(&legacy(&d));
    make(&session_dir(&d));
    add_use(&d, Use::Grades).unwrap();
    assert_eq!(profile_for(Feature::Grades, Some(&row()), &d), Some(session_dir(&d)));
    assert_eq!(grades_profile(&d), Ok(session_dir(&d)));
}

#[test]
fn a_new_install_reads_through_the_school_folder_and_records_the_grades_use() {
    let d = temp("fresh");
    assert_eq!(grades_profile(&d), Ok(session_dir(&d)));
    assert_eq!(uses(&d).unwrap(), vec![Use::Grades], "resolved to the school folder, so the use is recorded");
    assert_eq!(grades_profile(&d), Ok(session_dir(&d)), "and again, once recorded");
    assert_eq!(uses(&d).unwrap(), vec![Use::Grades], "recorded once");
}

#[test]
fn an_old_install_resolves_to_lms_session_and_records_nothing() {
    let d = temp("old");
    make(&legacy(&d));
    assert_eq!(grades_profile(&d), Ok(legacy(&d)));
    assert_eq!(uses(&d).unwrap(), Vec::<Use>::new());
    assert!(!session_dir(&d).exists(), "resolving a folder never makes the other one");
}

#[test]
fn clearing_the_course_lists_use_never_deletes_the_folder_grades_read_through() {
    let d = temp("keep");
    add_use(&d, Use::CourseList).unwrap(); // the capture window opens on the folder
    make(&session_dir(&d)); // and the student signs in
    assert_eq!(grades_profile(&d), Ok(session_dir(&d)), "no old folder, so grades read through the school's");
    clear_use(&d, Use::CourseList).unwrap(); // the capture window closes
    assert!(session_dir(&d).exists(), "grades' use keeps the folder");
    assert_eq!(uses(&d).unwrap(), vec![Use::Grades]);
}

#[test]
fn grades_windows_are_built_on_the_resolved_profile_and_never_on_the_old_folder_directly() {
    let grades = src("src/grades.rs");
    assert!(!grades.contains(".data_directory(session_dir("), "S4a: the builders take the profile `grades_profile` resolved");
    assert!(grades.contains("grades_profile") || grades.contains("profile_for"), "grades choose through school_session");
}

#[test]
fn once_grades_move_to_the_school_folder_the_abandoned_lms_session_is_forgotten() {
    let d = temp("abandon");
    make(&legacy(&d));
    make(&session_dir(&d));
    // Not moved yet: resolving the folder, either way, forgets nothing.
    assert_eq!(grades_profile(&d), Ok(legacy(&d)));
    assert!(legacy(&d).exists());
    add_use(&d, Use::Grades).unwrap();
    // The pure choice (status, connected) never deletes; only the window's resolution does.
    assert_eq!(profile_for(Feature::Grades, Some(&row()), &d), Some(session_dir(&d)));
    assert!(legacy(&d).exists(), "profile_for deletes nothing (Q-B4)");
    assert_eq!(grades_profile(&d), Ok(session_dir(&d)));
    assert!(!legacy(&d).exists(), "the abandoned lms-session is forgotten once grades moved (Q-B4)");
    assert!(session_dir(&d).exists(), "and the school folder is untouched");
    assert_eq!(uses(&d).unwrap(), vec![Use::Grades]);
}

#[test]
fn the_lock_key_is_the_profile_folders_own_name() {
    assert_eq!(knowlu::school_session::profile_key(Path::new("C:\\synthetic\\profiles\\prof-abc123")), "prof-abc123");
}

// ---- 3. Forget and "needs sign-in" (S3 tests 6 and 8, grades) ---------------------------------

#[test]
fn a_refresh_reads_through_the_recorded_school_folder_and_forget_leaves_it_needing_sign_in() {
    let d = temp("refresh");
    make(&session_dir(&d));
    add_use(&d, Use::Grades).unwrap();
    let r = row();
    let opened = Cell::new(0);
    let out = refresh_with(Some(&r), "blackboard", &d, |_| { opened.set(opened.get() + 1); Ok(()) }, |_| json!({ "ok": true }));
    assert_eq!(opened.get(), 1, "the signed-in school folder is a connected session");
    assert_eq!(out, json!({ "ok": true }));
    let s = status_for(Some(&r), "blackboard", &d, &d.join("no-vault"));
    assert_eq!(s["connected"], json!(true));

    // Forget school sign-in: the folder and its record go, and nothing opens afterwards.
    let gone = forget_with(&d, || {});
    assert_eq!(gone["ok"], json!(true), "{gone}");
    let opened = Cell::new(false);
    let out = refresh_with(Some(&r), "blackboard", &d, |_| { opened.set(true); Ok(()) }, |_| json!({ "ok": true }));
    assert!(!opened.get(), "with the folder gone no hidden window is opened");
    assert_eq!(out["error"], json!(knowlu::grades::NOT_CONNECTED));
}

#[test]
fn forget_at_a_shared_school_is_school_sessions_forget() {
    let d = temp("forget");
    make(&session_dir(&d));
    add_use(&d, Use::Grades).unwrap();
    add_use(&d, Use::CourseList).unwrap();
    let closed = Cell::new(false);
    let out = forget_with(&d, || closed.set(true));
    assert!(closed.get(), "the window closes first, as today");
    assert_eq!(out["ok"], json!(true), "{out}");
    assert!(!session_dir(&d).exists(), "the school folder is deleted");
    assert_eq!(uses(&d).unwrap(), Vec::<Use>::new(), "and its record of uses with it");
}

#[test]
fn forget_at_an_old_install_still_deletes_lms_session_alone() {
    let d = temp("forget-old");
    make(&legacy(&d));
    let out = forget_with(&d, || {});
    assert_eq!(out["ok"], json!(true));
    assert!(!legacy(&d).exists());
}

// ---- 4. one window at a time on the folder (S3 test 5) ----------------------------------------

#[test]
fn a_hidden_read_while_the_profile_is_held_answers_busy_and_opens_nothing() {
    let _turn = CAPTURING_TURN.lock().unwrap_or_else(|e| e.into_inner());
    let held = try_take("s3-profile-a").expect("free");
    let (opened, read) = (Cell::new(false), Cell::new(false));
    let out = capture_hidden_with("s3-profile-a", || { opened.set(true); Ok(()) }, || { read.set(true); Ok(json!({})) });
    assert_eq!(out, Err(CaptureError::Busy));
    assert!(!opened.get() && !read.get(), "nothing opens and nothing reads");
    // Another profile is not blocked by it.
    let other = capture_hidden_with("s3-profile-b", || Ok(()), || Ok(json!({ "ok": true })));
    assert_eq!(other, Ok(json!({ "ok": true })));
    drop(held);
    let again = capture_hidden_with("s3-profile-a", || Ok(()), || Ok(json!({ "ok": true })));
    assert_eq!(again, Ok(json!({ "ok": true })), "the guard is released by the read ending and by the drop");
}

#[test]
fn a_busy_capture_guard_answers_busy_and_frees_the_profile() {
    let _turn = CAPTURING_TURN.lock().unwrap_or_else(|e| e.into_inner());
    let capturing = knowlu::grades::Capturing::try_take().expect("free");
    let opened = Cell::new(false);
    let out = capture_hidden_with("s3-profile-c", || { opened.set(true); Ok(()) }, || Ok(json!({})));
    assert_eq!(out, Err(CaptureError::Busy));
    assert!(!opened.get());
    drop(capturing);
    assert!(try_take("s3-profile-c").is_some(), "the session guard was dropped on the way out");
}

// ---- 5. cookies go only where they were read (S3 test 3) --------------------------------------

fn host_of(url: &str) -> String {
    url.split("://").nth(1).unwrap_or("").split(['/', '?', '#']).next().unwrap_or("").to_ascii_lowercase()
}

/// What the readers' fakes saw: every host the jar was queried for, and each request's host paired with
/// the host of the query that preceded it.
struct Seen {
    queried: Vec<String>,
    pairs: Vec<(String, String)>,
}

/// The canned body for a URL: just enough for a reader to carry on to its next request.
fn body_for(url: &str) -> String {
    if url.ends_with("/users/me") {
        json!({ "id": "_100_1", "userName": "synthetic" }).to_string()
    } else if url.contains("/terms/") {
        json!({ "availability": { "available": "Yes" } }).to_string()
    } else if url.contains("users/me/courses") {
        let course = json!({ "userId": "_100_1", "courseId": "_9_1", "availability": { "available": "Yes" },
            "course": { "id": "_9_1", "courseId": "SYN-9", "name": "Synthetic", "termId": "_t_1",
                        "availability": { "available": "Term" } } });
        json!({ "results": [course] }).to_string()
    } else if url.contains("calendarFeed") {
        "https://lms.example.test/webapps/calendar/calendarFeed/abc/learn.ics".to_string()
    } else {
        json!({ "results": [] }).to_string()
    }
}

/// Run one reader over a fake jar and a fake agent. The jar answers every query with this host's cookie
/// and an identity provider's parent-domain one, whose `Domain` is not the queried host.
fn drive(run: impl FnOnce(&mut dyn FnMut(&str) -> Vec<JarCookie>, &mut dyn FnMut(&str, &str) -> Result<String, String>)) -> Seen {
    let queried = RefCell::new(Vec::<String>::new());
    let pairs = RefCell::new(Vec::<(String, String)>::new());
    let mut jar = |url: &str| {
        queried.borrow_mut().push(host_of(url));
        let ck = |name: &str, domain: &str| JarCookie { name: name.into(), value: "synthetic".into(), domain: domain.into() };
        vec![ck("sid", "lms.example.test"), ck("idp", "example.test")]
    };
    let mut send = |url: &str, _cookie: &str| {
        let q = queried.borrow().last().cloned().unwrap_or_default();
        pairs.borrow_mut().push((q, host_of(url)));
        Ok(body_for(url))
    };
    run(&mut jar, &mut send);
    Seen { queried: queried.into_inner(), pairs: pairs.into_inner() }
}

/// The reader asked the jar for its own host alone and sent every request to exactly that host.
fn assert_cookies_stay(seen: &Seen, host: &str, what: &str) {
    assert!(!seen.queried.is_empty(), "{what}: the jar was never queried");
    assert!(!seen.pairs.is_empty(), "{what}: no request was sent");
    for q in &seen.queried {
        assert_eq!(q, host, "{what}: queried a host that is not on the reader's list");
    }
    for (q, r) in &seen.pairs {
        assert_eq!(q, r, "{what}: a request went to {r} on cookies read for {q}");
        assert_eq!(r, host, "{what}");
    }
}

/// Where the window might be when the reader runs: on the identity provider, on the reader's own host, nowhere.
const WINDOWS: [Option<&str>; 3] =
    [Some("https://idp.example.test/login"), Some("https://lms.example.test/ultra/institution-page"), None];

#[test]
fn grades_read_cookies_for_the_lms_host_only_and_send_every_request_there() {
    for window in WINDOWS {
        let seen = drive(|jar, send| {
            let _ = read_session_with("lms.example.test", window, jar, send);
        });
        assert_cookies_stay(&seen, "lms.example.test", &format!("grades, window at {window:?}"));
        assert!(seen.pairs.len() >= 2, "users/me and the memberships at least: {:?}", seen.pairs);
    }
}

#[test]
fn the_calendar_feed_read_uses_the_feed_host_only() {
    let feed = "https://lms.example.test/webapps/calendar/calendarFeed/url";
    for window in WINDOWS {
        let seen = drive(|jar, send| {
            let _ = read_feed_with(feed, window, jar, send);
        });
        assert_cookies_stay(&seen, "lms.example.test", &format!("feed, window at {window:?}"));
    }
}

#[test]
fn the_course_list_read_and_its_term_requests_use_the_list_host_only() {
    let list = "https://lms.example.test/learn/api/public/v1/users/me/courses?expand=course";
    for window in WINDOWS {
        let seen = drive(|jar, send| {
            let _ = read_courses_with(list, window, jar, send);
        });
        assert_cookies_stay(&seen, "lms.example.test", &format!("courses, window at {window:?}"));
        assert!(seen.pairs.len() >= 2, "the list and its course's term: {:?}", seen.pairs);
    }
}

// ---- 6. the LMS capture window (S3 test 1 and 6; S4b) -----------------------------------------

#[test]
fn the_lms_capture_on_the_school_folder_is_non_incognito_and_records_the_course_list_use() {
    let d = temp("plan");
    let throwaway = lms_link::session_dir();
    let plan = window_plan(Some(&d), &throwaway);
    assert_eq!(plan.data_dir, session_dir(&d), "built on the school folder");
    assert!(!plan.incognito, "non-incognito, so the sign-in is kept");
    assert_eq!(plan.browser_args, Some(SCHOOL_BROWSER_ARGS), "with the one arguments constant");
    assert_eq!(plan.records_use, Some(Use::CourseList));
    assert!(!plan.wipe_on_destroy, "the shared folder is never wiped by the window's close");
}

#[test]
fn without_a_school_folder_the_lms_capture_is_todays_incognito_throwaway() {
    let throwaway = lms_link::session_dir();
    let plan = window_plan(None, &throwaway);
    assert_eq!(plan.data_dir, throwaway);
    assert!(plan.incognito);
    assert_eq!(plan.records_use, None);
    assert!(plan.wipe_on_destroy, "the throwaway, and only it, is wiped");
}

#[test]
fn the_wipe_and_the_sweep_refuse_the_school_folder_and_still_take_the_throwaway() {
    let d = temp("refuse");
    assert!(!lms_link::is_session_dir(&session_dir(&d)));
    assert!(!lms_link::is_session_dir(&std::env::temp_dir().join("school-session")));
    assert!(lms_link::is_session_dir(&lms_link::session_dir()), "today's throwaway is still ours to wipe");
    // And the school guard refuses the throwaway, so neither module can delete the other's folder.
    assert!(!is_session_dir(&d, &lms_link::session_dir()));
}

// ---- 7. scripts and popups stay per window (S3 test 4, as the replan keeps it) ----------------

#[test]
fn no_grades_or_lms_window_sets_a_popup_handler_and_only_the_fill_may_set_a_script() {
    let lms = src("src/lms_link.rs");
    let grades = src("src/grades.rs");
    for (name, text) in [("grades.rs", &grades), ("lms_link.rs", &lms)] {
        assert!(!text.contains(".on_new_window("), "{name} sets a new-window handler");
        assert!(!text.contains("INIT_SCRIPT"), "{name} carries the mail window's script");
    }
    assert!(!lms.contains(".initialization_script("), "the LMS capture window runs no script of ours");
    // The one named exception: P2's fill, in the LMS folder's hidden window, on a `sign_in_hosts` host only.
    let scripts = grades.matches(".initialization_script(").count();
    assert!(scripts <= 1, "grades' hidden window may carry the fill and nothing else");
    if scripts == 1 {
        assert!(grades.contains("sign_in_hosts"), "the fill is gated on the row's sign_in_hosts");
    }
}

#[test]
fn no_capability_names_a_grades_or_lms_window() {
    let caps = src("capabilities/default.json");
    for label in ["lms-grades", "lms-signin"] {
        assert!(!caps.contains(label), "{label} must have no capability grant");
    }
}

#[test]
fn the_registrar_never_shares_the_lms_profile() {
    let registrar = src("src/registrar.rs");
    assert!(!registrar.contains("school_session"), "registrar.rs stays on its incognito throwaway window");
    for feature in [Feature::Registrar, Feature::SchoolMail] {
        assert_ne!(profile_for(feature, Some(&row()), Path::new("C:\\synthetic")), Some(session_dir(Path::new("C:\\synthetic"))));
    }
}


#[test]
fn the_lms_capture_window_holds_the_profile_lock_for_its_life_and_releases_it_with_the_window() {
    let session = lms_link::LmsSession::default();
    let profile = "s4b-capture-profile";
    let mine = session.hold_school(profile).expect("a free profile is taken by the window being built");
    assert!(session.is_school(), "the open window is a school window while it holds the folder");
    assert!(try_take(profile).is_none(), "held while the window lives: grades' windows answer Busy");
    assert!(lms_link::LmsSession::default().hold_school(profile).is_none(), "a second window on the profile is refused, not waited on");
    assert!(try_take("s4b-other-profile").is_some(), "another profile is not blocked");
    // The window's own Destroyed event: the hold goes, and the school folder is never named for a wipe.
    assert!(session.release(mine));
    assert!(!session.is_school());
    assert_eq!(session.end(), None, "no throwaway folder was remembered for a school window");
    assert!(try_take(profile).is_some(), "freed once the window is gone");
}

#[test]
fn a_late_destroyed_event_from_an_old_window_cannot_free_a_new_windows_hold() {
    let session = lms_link::LmsSession::default();
    let profile = "s4b-late-profile";
    let old = session.hold_school(profile).unwrap();
    assert!(session.release(old), "the old window's own event frees its hold");
    let new = session.hold_school(profile).expect("a new window on the freed profile");
    assert_ne!(old, new, "each window has its own generation");
    assert!(!session.release(old), "the old window's late event finds a hold that is not its own");
    assert!(session.is_school() && try_take(profile).is_none(), "the new window still holds the profile");
    assert!(session.release(new));
    assert!(try_take(profile).is_some());
}

#[test]
fn the_exit_frees_whatever_hold_is_left() {
    let session = lms_link::LmsSession::default();
    session.hold_school("s4b-exit-profile").unwrap();
    session.end();
    assert!(try_take("s4b-exit-profile").is_some());
}

#[test]
fn the_registrar_and_the_school_window_never_land_on_each_others_open_window() {
    use lms_link::{reuse_open_window as reuse, Reuse};
    let school = window_plan(Some(Path::new("C:/synthetic")), &lms_link::session_dir());
    let throwaway = window_plan(None, &lms_link::session_dir());
    assert_eq!(reuse(None, &school), Reuse::Build);
    assert_eq!(reuse(None, &throwaway), Reuse::Build);
    assert_eq!(reuse(Some(true), &school), Reuse::Show, "a second Sign in shows the school window");
    assert_eq!(reuse(Some(false), &throwaway), Reuse::Show, "a second registrar click shows the registrar's");
    assert_eq!(reuse(Some(true), &throwaway), Reuse::Refuse, "the registrar never shows the kept school window");
    assert_eq!(reuse(Some(false), &school), Reuse::Refuse, "and the school capture never shows the registrar's");
    // The kind of the open window is whether the session holds the school folder.
    let session = lms_link::LmsSession::default();
    let open = |s: &lms_link::LmsSession| Some(s.is_school());
    assert_eq!(reuse(open(&session), &school), Reuse::Refuse, "an open window with no hold is a throwaway");
    let g = session.hold_school("s4b-kind-profile").unwrap();
    assert_eq!(reuse(open(&session), &throwaway), Reuse::Refuse);
    assert_eq!(reuse(open(&session), &school), Reuse::Show);
    session.release(g);
}

#[test]
fn the_builder_decides_before_it_shows_and_refuses_in_words() {
    let lms = src("src/lms_link.rs");
    let at = lms.find("fn open_window_with").expect("open_window_with");
    let body = &lms[at..at + lms[at..].find("\n}\n").unwrap()];
    let decide = body.find("reuse_open_window(").expect("the open window's kind is checked");
    assert!(decide < body.find(".show()").expect("an open window is shown"), "never shown before its kind is checked");
    assert!(lms.contains("close the other sign-in window first"));
}

#[test]
fn the_throwaway_windows_folder_is_still_handed_back_for_its_wipe() {
    let session = lms_link::LmsSession::default();
    let throwaway = lms_link::session_dir();
    session.remember_dir(&throwaway);
    assert_eq!(session.end(), Some(throwaway), "ending the throwaway window hands its folder to the wipe, once");
    assert_eq!(session.end(), None, "and only once");
}

#[test]
fn the_lms_capture_records_its_use_when_it_opens_and_never_clears_it_when_it_closes() {
    let lms = src("src/lms_link.rs");
    assert!(lms.contains("add_use("), "the school window records Use::CourseList");
    assert!(!lms.contains("clear_use("), "the use is not cleared on close (controller ruling): grades' use and Forget govern the folder");
}

#[test]
fn forget_with_both_folders_and_only_the_course_list_recorded_leaves_neither_and_nothing_opens() {
    let d = temp("forget-both");
    make(&legacy(&d));
    make(&session_dir(&d));
    add_use(&d, Use::CourseList).unwrap();
    assert_eq!(grades_profile(&d), Ok(legacy(&d)), "grades are still on the old folder");
    let out = forget_with(&d, || {});
    assert_eq!(out["ok"], json!(true), "{out}");
    assert!(!legacy(&d).exists() && !session_dir(&d).exists(), "neither folder remains: forgetting one must not leave the other to answer");
    assert!(!knowlu::grades::connected(&d), "so grades are not connected");
    let opened = Cell::new(false);
    let r = row();
    let out = refresh_with(Some(&r), "blackboard", &d, |_| { opened.set(true); Ok(()) }, |_| json!({ "ok": true }));
    assert!(!opened.get() && out["error"] == json!(knowlu::grades::NOT_CONNECTED), "the next capture opens nothing");
    assert_eq!(uses(&d).unwrap(), Vec::<Use>::new(), "and records nothing");
}

#[test]
fn a_busy_profile_held_by_the_sign_in_window_is_named_for_it() {
    use knowlu::grades::busy_kind;
    assert_eq!(busy_kind(false, true), CaptureError::WindowOpen, "the LMS capture window holds the folder");
    assert_eq!(busy_kind(true, false), CaptureError::WindowOpen, "grades' own visible sign-in");
    assert_eq!(busy_kind(false, false), CaptureError::Busy, "another capture running");
    assert_eq!(CaptureError::WindowOpen.as_str(), "sign-in window open");
}

#[test]
fn the_registrar_acts_only_on_a_throwaway_window_and_says_so_in_words() {
    use lms_link::registrar_window_check as check;
    assert_eq!(check(None), Ok(()), "no window: the registrar's own checks answer");
    assert_eq!(check(Some(false)), Ok(()), "the registrar's throwaway window");
    assert_eq!(check(Some(true)), Err("the open sign-in window is not the registrar's"), "the kept school window");
    assert_eq!(lms_link::NOT_THE_REGISTRARS, "the open sign-in window is not the registrar's");
}

#[test]
fn capture_and_close_check_the_window_before_they_read_a_cookie_or_close_anything() {
    let reg = src("src/registrar.rs");
    assert!(!reg.contains("school_session"), "registrar.rs names no school folder, only lms_link's check");
    let body_of = |start: &str| {
        let at = reg.find(start).unwrap_or_else(|| panic!("{start}"));
        let tail = &reg[at..];
        tail[..tail.find("\n}\n").unwrap()].to_string()
    };
    for (name, body) in [("capture", body_of("fn capture(")), ("close_registrar_window", body_of("pub fn close_registrar_window("))] {
        let check = body.find("check_registrar_window(").unwrap_or_else(|| panic!("{name} never checks the open window"));
        for acts in ["get_webview_window(", "cookies_for_url(", "close_and_wipe("] {
            if let Some(at) = body.find(acts) {
                assert!(check < at, "{name}: {acts} comes before the check:\n{body}");
            }
        }
        assert!(body.contains("\"ok\": false") || body.contains("failed("), "{name} refuses with ok:false");
    }
}
