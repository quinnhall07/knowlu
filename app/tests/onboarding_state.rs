//! O3 F1: `onboarding.json`, `onboarding-answers.json`, resume, skips, `LaunchMode`, the sign-out
//! rewind, the re-attach (test 21) and the vault-target entitlement and checkout (test 22). Plan
//! `docs/plans/2026-10-23-wave-b-long-onboarding-plan.md`, F1; test numbers are the plan's.
//!
//! Everything here uses fakes and scratch folders except tests 21 and 22, which hold this file's
//! `CREDMAN_LOCK`, use generated targets and clean up with a `Drop` guard (`app/tests/account.rs`'s
//! pattern). Invented data only; nothing leaves the machine.
use knowlu::onboarding_state::*;
use std::cell::Cell;
use std::collections::BTreeMap;
use std::path::{Path, PathBuf};
use std::sync::Mutex;

fn scratch(tag: &str) -> PathBuf {
    let d = std::env::temp_dir().join(format!("knowlu-o3-state-{tag}-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&d);
    std::fs::create_dir_all(&d).unwrap();
    d
}

/// A pending-credential store in memory: no Credential Manager.
#[derive(Default)]
struct FakePending(Mutex<BTreeMap<String, String>>);
impl FakePending {
    fn with(entries: &[(&str, &str)]) -> FakePending {
        FakePending(Mutex::new(entries.iter().map(|(k, v)| (k.to_string(), v.to_string())).collect()))
    }
    fn is_empty(&self) -> bool { self.0.lock().unwrap().is_empty() }
}
impl PendingStore for FakePending {
    fn put(&self, name: &str, link: &str) -> Result<(), String> {
        self.0.lock().unwrap().insert(name.to_string(), link.to_string());
        Ok(())
    }
    fn get(&self, name: &str) -> Option<String> { self.0.lock().unwrap().get(name).cloned() }
    fn delete(&self, name: &str) { self.0.lock().unwrap().remove(name); }
}

const LMS_LINK: &str = "https://lms.example.invalid/feed/tok-7f3a9c.ics";
const CAL_LINK: &str = "webcal://cal.example.invalid/private/tok-2b81d4.ics";

/// Every field of the state set to something, every job state present, no URL anywhere.
fn full_state() -> OnboardingState {
    let mut s = OnboardingState::new("2026-10-24T09:00:00Z", 8);
    s.finished = false;
    s.school = Some(School { unitid: Some("900001".into()), label: "Example Test College".into(), path: SchoolPath::Separate, confirmed: true });
    s.skips = vec![Skip { step: 5, source: "mail".into(), cost_key: "mail_dates_missed".into() }];
    for (name, state) in [
        ("school.capture", JobState::Done), ("school.preview", JobState::Done), ("school.store", JobState::Running),
        ("school.apply", JobState::Pending), ("school.courses", JobState::Held), ("config.ingest", JobState::Failed),
        ("mail.setup", JobState::Skipped),
    ] {
        let mut rec = JobRecord { state, ..Default::default() };
        if state == JobState::Done { rec.proof.insert("classes".into(), 4); rec.proof.insert("assignments_upcoming".into(), 17); }
        if state == JobState::Failed { rec.reason = Some("could_not_write_config".into()); }
        s.jobs.insert(name.into(), rec);
    }
    s.account_settled = true;
    s.step10_confirmed_at = Some("2026-10-24T09:40:00Z".into());
    s.handover = vec![HandoverItem { item: "slot.first_full".into(), signal: "slot_end".into() }];
    s
}

fn full_answers() -> Answers {
    let mut a = Answers::default();
    a.courses = vec![
        // Fragments: the human code, then a gradebook column id in the pin form (never a course id).
        CourseAnswer { code: "QLT 101".into(), name: "Quilting Basics".into(), label: "Quilting".into(), lms_key: "_1001_1".into(), removed: false, slug: "quilting-basics".into(), fragments: vec!["QLT 101".into(), "_71_1".into()] },
        CourseAnswer { code: "KNT 220".into(), name: "Knotwork Theory".into(), label: "Knots".into(), lms_key: "_1002_1".into(), removed: true, slug: String::new(), fragments: vec!["KNT 220".into()] },
    ];
    a.registrar_rows = vec![RegistrarRow { code: "QLT 101".into(), days: "MWF".into(), start: "09:00".into(), end: "09:50".into() }];
    put_bucket(&mut a, &["_1001_1"], &[("class_times", 3)]);
    put_bucket(&mut a, &["_1001_1", "_1002_1"], &[("assignments_upcoming", 2)]);
    put_bucket(&mut a, &[], &[("turned_in", 1)]);
    // A count whose input was missing is `null`, and survives the round trip as `null`.
    a.buckets.get_mut(UNMATCHED).unwrap().counts.insert("class_times".into(), None);
    a.slugs.insert("QLT 101".into(), "quilting-basics".into());
    a.google_calendar = Some(true);
    a.feeds.insert("lms_ics".into(), FeedState::Validated);
    a.feeds.insert("calendar_ics".into(), FeedState::Rejected);
    a.rows = vec![BookRow { source: "zybooks".into(), key: "ZYB-QLT-101".into(), course_slug: "quilting-basics".into() }];
    a.zybooks_ignore = vec!["ZYB-ONBOARDING".into()];
    a
}

#[test]
fn test_01_onboarding_json_round_trips_every_field() {
    let d = scratch("t01-state");
    let s = full_state();
    save_state(&d, &s).expect("save");
    assert_eq!(load_state(&d), Loaded::Ok(s));
    assert_eq!(load_state(&scratch("t01-state-empty")), Loaded::Absent);
}

#[test]
fn test_01_the_answers_file_round_trips_every_field() {
    let d = scratch("t01-answers");
    assert_eq!(load_answers(&d), Ok(None), "no file is no answers yet, and only that");
    let a = full_answers();
    save_answers(&d, &a).expect("save");
    assert_eq!(load_answers(&d), Ok(Some(a)));
}

#[test]
fn test_01_neither_file_holds_a_url_a_cookie_or_a_login_and_the_state_holds_no_course_name() {
    let d = scratch("t01-nourl");
    // A captured feed link is present in the run: it waits in the pending store, and its states are
    // recorded. Neither file may carry it, or any URL-shaped text.
    // The files are written by the code paths that handle a link, not by a hand-built value: the
    // paste command's inner validates and keeps the link, and a fake `school.capture` answers
    // step 3 through the answer-recording inner. Then both files are scanned.
    let pending = FakePending::default();
    let good = "BEGIN:VCALENDAR\r\nBEGIN:VEVENT\r\nSUMMARY:QLT 101 Homework 4\r\nEND:VEVENT\r\nEND:VCALENDAR\r\n";
    let pasted = knowlu::onboarding_jobs::paste_link_inner(&d, &pending, "lms_ics", LMS_LINK, &|_| Ok(good.to_string()));
    assert_eq!(pasted["ok"], true, "{pasted}");
    assert_eq!(pending.get(PENDING_LMS).as_deref(), Some(LMS_LINK));
    pending.put(PENDING_CALENDAR, CAL_LINK).unwrap();
    let capture = serde_json::json!({"capture": {
        "courses": full_answers().courses,
        "registrar_rows": full_answers().registrar_rows,
        "feeds": {"lms_ics": "validated"},
    }});
    let answered = onboarding_answer_inner(&d, 3, &capture, &mut |_| {});
    assert_eq!(answered["ok"], true, "{answered}");
    save_state(&d, &full_state()).expect("save state");
    let state_text = std::fs::read_to_string(state_path(&d)).expect("onboarding.json exists after a save");
    let answers_text = std::fs::read_to_string(answers_path(&d)).expect("the answers file exists after a save");
    for (which, text) in [("onboarding.json", &state_text), ("onboarding-answers.json", &answers_text)] {
        for banned in ["http", "://", "webcal", "knowlu/", "tok-7f3a9c", "tok-2b81d4", "cookie", "password"] {
            assert!(!text.to_lowercase().contains(banned), "{which} holds {banned:?}: {text}");
        }
    }
    // The answers file holds course names (the vault holds them after payment anyway); the state
    // file, which is read by the rail on every poll, holds none.
    assert!(answers_text.contains("Quilting Basics"));
    for name in ["Quilting Basics", "Knotwork Theory", "QLT 101", "KNT 220"] {
        assert!(!state_text.contains(name), "onboarding.json holds a course name: {name}");
    }
}

#[test]
fn test_01_both_files_are_written_beside_settings_json() {
    let root = scratch("t01-beside");
    let vault = root.join("vault");
    std::fs::create_dir_all(&vault).unwrap();
    let cs = knowlu::state::ConsoleState::open(vault, root.join("appdata"));
    let settings_dir = cs.settings_path.parent().expect("settings.json has a folder").to_path_buf();
    assert_eq!(state_path(&cs.data_dir).parent(), Some(settings_dir.as_path()));
    assert_eq!(answers_path(&cs.data_dir).parent(), Some(settings_dir.as_path()));
    save_state(&cs.data_dir, &full_state()).expect("save state");
    save_answers(&cs.data_dir, &full_answers()).expect("save answers");
    assert!(settings_dir.join("onboarding.json").is_file());
    assert!(settings_dir.join("onboarding-answers.json").is_file());
    // Never in the vault.
    assert!(!cs.vault.join("onboarding.json").exists() && !cs.vault.join("onboarding-answers.json").exists());
}

#[test]
fn test_01_an_unreadable_answers_file_is_an_error_never_no_answers() {
    let d = scratch("t01-unreadable");
    for bad in ["{ not json", "", "[1, 2, 3]", "{\"courses\": \"nope\"}"] {
        std::fs::write(answers_path(&d), bad).unwrap();
        assert!(load_answers(&d).is_err(), "{bad:?} must be an error, not Ok(None) or empty answers");
    }
    // And updating an unreadable file does not start over from empty: the bytes stay.
    std::fs::write(answers_path(&d), "{ not json").unwrap();
    assert!(update_answers(&d, &|a| a.google_calendar = Some(true)).is_err());
    assert_eq!(std::fs::read_to_string(answers_path(&d)).unwrap(), "{ not json");
}

// ---- test 2: resume ---------------------------------------------------------------------------------

/// Fakes of §4.7's checks. Counts how often the account is asked for a school: it must be never.
struct Checks {
    session: Cell<bool>,
    courses: usize,
    entitled: bool,
    mailbox: bool,
    google: bool,
    login: bool,
    week: bool,
    answers: Cell<bool>,
    school_asks: Cell<usize>,
}
impl Checks {
    fn at_least_signed_in() -> Checks {
        Checks { session: Cell::new(true), courses: 0, entitled: false, mailbox: false, google: false, login: false, week: false, answers: Cell::new(true), school_asks: Cell::new(0) }
    }
    fn restored(courses: usize) -> Checks { Checks { courses, ..Checks::at_least_signed_in() } }
}
impl ResumeChecks for Checks {
    fn session_present(&self) -> bool { self.session.get() }
    fn restored_course_count(&self) -> usize { self.courses }
    fn entitled(&self) -> bool { self.entitled }
    fn school_mailbox_active(&self) -> bool { self.mailbox }
    fn google_granted(&self) -> bool { self.google }
    fn homework_login_stored(&self) -> bool { self.login }
    fn class_commitments_cover_courses(&self) -> bool { self.week }
    fn account_school(&self) -> Option<School> {
        self.school_asks.set(self.school_asks.get() + 1);
        None
    }
    fn answers_ok(&self) -> bool { self.answers.get() }
}

fn unfinished(step: u8) -> OnboardingState { OnboardingState::new("2026-10-24T09:00:00Z", step) }

#[test]
fn test_02_an_unfinished_file_reopens_at_its_step_and_running_jobs_come_back_pending() {
    let d = scratch("t02-reopen");
    let mut s = unfinished(5);
    for (n, st) in [("school.capture", JobState::Done), ("mail.setup", JobState::Running), ("coursework.discover", JobState::Running), ("school.store", JobState::Failed)] {
        s.jobs.insert(n.into(), JobRecord { state: st, ..Default::default() });
    }
    save_state(&d, &s).unwrap();
    let r = resume(load_state(&d), &Checks::restored(2));
    assert_eq!(r.mode, LaunchMode::Onboarding);
    assert_eq!(r.step, 5);
    let st = r.state.expect("onboarding runs with a state");
    assert_eq!(st.step, 5);
    assert_eq!(st.jobs["mail.setup"].state, JobState::Pending);
    assert_eq!(st.jobs["coursework.discover"].state, JobState::Pending);
    assert_eq!(st.jobs["school.capture"].state, JobState::Done, "a finished job is not re-run");
    assert_eq!(st.jobs["school.store"].state, JobState::Failed);
}

#[test]
fn test_02_an_unreadable_file_resumes_at_the_earliest_step_the_checks_do_not_prove_never_finished() {
    let d = scratch("t02-unreadable");
    std::fs::write(state_path(&d), "{ \"step\": 7, \"finished\": tru").unwrap();
    assert!(matches!(load_state(&d), Loaded::Unreadable(_)));
    let ladder: Vec<(Checks, u8)> = vec![
        (Checks { session: Cell::new(false), ..Checks::restored(3) }, 2),
        (Checks::at_least_signed_in(), 3),
        (Checks::restored(3), 4),
        (Checks { entitled: true, ..Checks::restored(3) }, 5),
        (Checks { entitled: true, mailbox: true, ..Checks::restored(3) }, 6),
        (Checks { entitled: true, mailbox: true, google: true, ..Checks::restored(3) }, 8),
        (Checks { entitled: true, mailbox: true, google: true, login: true, ..Checks::restored(3) }, 10),
        // Everything proven: still not "finished" and not past the last asked step.
        (Checks { entitled: true, mailbox: true, google: true, login: true, week: true, ..Checks::restored(3) }, 10),
    ];
    for (checks, want) in ladder {
        let r = resume(load_state(&d), &checks);
        assert_eq!(r.mode, LaunchMode::Onboarding, "step {want}");
        assert_eq!(r.step, want);
        let st = r.state.expect("rewritten from the derived step");
        assert_eq!((st.step, st.finished), (want, false));
    }
}

#[test]
fn test_02_an_absent_file_on_a_registered_profile_is_a_normal_launch() {
    let r = resume(load_state(&scratch("t02-absent")), &Checks::restored(2));
    assert_eq!(r.mode, LaunchMode::Normal);
    assert!(r.state.is_none());
}

// ---- test 2, T0 review M5: the school at step 3 on a restored install -----------------------------------

fn a_school() -> School { School { unitid: Some("900001".into()), label: "Example Test College".into(), path: SchoolPath::Shared, confirmed: true } }

#[test]
fn test_02_a_new_profile_on_a_restored_account_asks_for_the_school_and_never_asks_the_account() {
    // (a) F3 wrote an unfinished file at step 3 with no school; the restore brought three course notes.
    let d = scratch("t02-school-a");
    save_state(&d, &unfinished(3)).unwrap();
    let checks = Checks::restored(3);
    let r = resume(load_state(&d), &checks);
    assert_eq!(r.step, 3);
    assert_eq!(r.school, SchoolOffer::Ask, "nothing on this device names a school");
    assert_eq!(school_offer(None, &checks), SchoolOffer::Ask);
    assert_eq!(checks.school_asks.get(), 0, "the account cannot say which school: it is never asked");
}

#[test]
fn test_02_a_school_left_by_a_quit_after_step_3_is_offered_as_a_confirm_and_assumes_nothing() {
    // (b) a quit after step 3 saved `school`.
    let d = scratch("t02-school-b");
    let mut s = unfinished(4);
    s.school = Some(a_school());
    save_state(&d, &s).unwrap();
    let checks = Checks::restored(3);
    let r = resume(load_state(&d), &checks);
    let SchoolOffer::Confirm(offered) = r.school.clone() else { panic!("a saved school is offered: {:?}", r.school) };
    assert_eq!((offered.unitid.as_deref().unwrap_or(""), offered.label.as_str()), ("900001", "Example Test College"));
    assert!(!offered.confirmed, "the offer assumes nothing");
    let mut st = r.state.expect("a state");
    assert!(!school_apply_may_run(&st), "school.apply does not run until the school is confirmed");
    confirm_school(&mut st);
    assert!(school_apply_may_run(&st));
    assert_eq!(checks.school_asks.get(), 0);
}

#[test]
fn test_02_a_file_rewound_by_sign_out_has_no_school_and_behaves_as_a_fresh_install() {
    // (c) `on_sign_out` clears `school` with the rest of the rewind.
    let d = scratch("t02-school-c");
    let mut s = unfinished(8);
    s.school = Some(a_school());
    save_state(&d, &s).unwrap();
    save_answers(&d, &full_answers()).unwrap();
    on_sign_out_with(&d, &FakePending::default()).expect("the rewind");
    let checks = Checks::restored(3);
    let Loaded::Ok(rewound) = load_state(&d) else { panic!("the file stays, rewound") };
    assert!(rewound.school.is_none());
    let r = resume(Loaded::Ok(rewound), &checks);
    assert_eq!(r.step, 3, "signed in again, step 2 is proven and step 3 is next");
    assert_eq!(r.school, SchoolOffer::Ask);
    assert_eq!(checks.school_asks.get(), 0);
}

// ---- test 7: skips ----------------------------------------------------------------------------------------

#[test]
fn test_07_account_payment_the_school_step_and_your_week_refuse_a_skip() {
    for step in [2u8, 3, 4, 10] {
        let mut s = unfinished(step);
        assert!(skip_step(&mut s, step, "any").is_err(), "step {step} refuses a skip");
        assert!(s.skips.is_empty(), "a refused skip records nothing (step {step})");
    }
}

#[test]
fn test_07_every_other_step_takes_a_skip_and_records_it_with_its_cost_key() {
    for (step, source) in [(5u8, "mail"), (6, "google"), (7, "calendar"), (8, "zybooks")] {
        let mut s = unfinished(step);
        skip_step(&mut s, step, source).unwrap_or_else(|e| panic!("step {step} takes a skip: {e}"));
        assert_eq!(s.skips.len(), 1);
        assert_eq!((s.skips[0].step, s.skips[0].source.as_str()), (step, source));
        assert!(!s.skips[0].cost_key.is_empty(), "step {step}: the skip names its cost");
    }
}

/// §4.2 gives each skip a cost sentence (step 5 "date changes and new work announced by email are
/// missed"; step 6 "club, job and internship tasks from Gmail are missed"; step 7 "as step 6's calendar
/// half"; step 8 "that site's due dates are missing"), so each step's key is its own: no two steps
/// share one, the same skip gives the same key twice, and step 8 names its site (a per-site skip).
#[test]
fn test_07_each_steps_skip_has_its_own_cost_key_and_step_8_is_per_site() {
    let key = |step: u8, source: &str| {
        let mut s = unfinished(step);
        skip_step(&mut s, step, source).unwrap();
        s.skips[0].cost_key.clone()
    };
    let keys = [key(5, "mail"), key(6, "google"), key(7, "calendar"), key(8, "zybooks")];
    for (i, a) in keys.iter().enumerate() {
        for b in &keys[i + 1..] {
            assert_ne!(a, b, "two steps share one cost key: {keys:?}");
        }
    }
    assert_eq!(key(5, "mail"), keys[0], "deterministic");
    assert_ne!(key(8, "zybooks"), key(8, "vhl"), "step 8's skip is per site, so its cost is");
    let mut s = unfinished(8);
    skip_step(&mut s, 8, "zybooks").unwrap();
    skip_step(&mut s, 8, "vhl").unwrap();
    assert_eq!(s.skips.len(), 2, "each site's skip is recorded on its own");
}

// ---- test 10: LaunchMode, and the no-session rule -------------------------------------------------------------

#[test]
fn test_10_launch_mode_follows_the_file() {
    let d = scratch("t10-mode");
    assert_eq!(launch_mode(&load_state(&d)), LaunchMode::Normal, "absent is a normal launch");
    save_state(&d, &unfinished(6)).unwrap();
    assert_eq!(launch_mode(&load_state(&d)), LaunchMode::Onboarding);
    let mut done = unfinished(11);
    done.finished = true;
    save_state(&d, &done).unwrap();
    assert_eq!(launch_mode(&load_state(&d)), LaunchMode::Normal, "finished, no hand-over");
    done.handover = vec![HandoverItem { item: "slot.first_full".into(), signal: "slot_end".into() }];
    save_state(&d, &done).unwrap();
    assert_eq!(launch_mode(&load_state(&d)), LaunchMode::Normal, "finished, with a hand-over");
    std::fs::write(state_path(&d), "not json at all").unwrap();
    assert_eq!(launch_mode(&load_state(&d)), LaunchMode::Onboarding, "unreadable is onboarding (test 2)");
}

#[test]
fn test_10_a_profile_with_no_session_resumes_at_the_sign_in_whatever_the_files_step() {
    let d = scratch("t10-nosession");
    save_state(&d, &unfinished(8)).unwrap();
    let checks = Checks { session: Cell::new(false), ..Checks::restored(3) };
    let r = resume(load_state(&d), &checks);
    assert_eq!((r.mode, r.step), (LaunchMode::Onboarding, 2), "step 2's sign-in, inside onboarding mode");
    assert_eq!(r.state.as_ref().unwrap().step, 8, "the file's own step is kept for after the sign-in");
    // A refused attach (test 21's mismatch) leaves the session absent: the resume stays at step 2.
    assert_eq!(resume(load_state(&d), &checks).step, 2);
    // After the fake sign-in and an attach that succeeds, it resumes at the file's step.
    checks.session.set(true);
    assert_eq!(resume(load_state(&d), &checks).step, 8);
}

// ---- test 17: the sign-out rewind (the state half; the exits are in onboarding_jobs.rs) ---------------------------

fn ordinal_job_states(s: &OnboardingState) -> Vec<JobState> { s.jobs.values().map(|j| j.state).collect() }

#[test]
fn test_17_on_sign_out_deletes_the_answers_and_the_pending_links_and_rewinds_to_step_2() {
    let d = scratch("t17-rewind");
    let before = full_state();
    save_state(&d, &before).unwrap();
    save_answers(&d, &full_answers()).unwrap();
    let pending = FakePending::with(&[(PENDING_LMS, LMS_LINK), (PENDING_CALENDAR, CAL_LINK)]);
    on_sign_out_with(&d, &pending).expect("sign-out's rewind");
    assert!(!answers_path(&d).exists(), "the answers file is deleted");
    assert!(pending.is_empty(), "both pending links are deleted");
    let Loaded::Ok(after) = load_state(&d) else { panic!("an unfinished file is rewritten, not removed") };
    assert_eq!(after.step, 2);
    assert!(!after.finished);
    assert!(ordinal_job_states(&after).iter().all(|s| *s == JobState::Pending), "every job is pending again: {:?}", after.jobs);
    assert!(!after.account_settled && after.school.is_none());
    assert_eq!(after.skips, before.skips, "the skips are kept");
    assert_eq!(after.started_at, before.started_at);
}

#[test]
fn test_17_a_finished_file_is_not_touched_by_sign_out() {
    let d = scratch("t17-finished");
    let mut s = full_state();
    s.finished = true;
    save_state(&d, &s).unwrap();
    let bytes = std::fs::read(state_path(&d)).unwrap();
    on_sign_out_with(&d, &FakePending::default()).expect("nothing to rewind");
    assert_eq!(std::fs::read(state_path(&d)).unwrap(), bytes);
}

#[test]
fn test_17_a_step_8_file_with_no_answers_resumes_at_step_3_and_no_config_job_runs_without_answers() {
    let d = scratch("t17-noanswers");
    save_state(&d, &unfinished(8)).unwrap();
    assert_eq!(load_answers(&d), Ok(None));
    let checks = Checks { answers: Cell::new(false), ..Checks::restored(3) };
    assert_eq!(resume(load_state(&d), &checks).step, 3, "the answers were deleted or never saved: ask again");
    // An unreadable answers file is the same.
    std::fs::write(answers_path(&d), "{ broken").unwrap();
    assert_eq!(resume(load_state(&d), &checks).step, 3);
    // With the answers present, the file's own step stands.
    checks.answers.set(true);
    assert_eq!(resume(load_state(&d), &checks).step, 8);
    // And the config jobs that read the answers refuse to run on none: nothing is created.
    let none = scratch("t17-noanswers-run");
    let out = knowlu::onboarding_jobs::run_school_courses(&none, &[], &mut |_| panic!("no note is created without answers"));
    assert!(matches!(out, knowlu::onboarding_jobs::Outcome::Failed(_)), "{out:?}");
}

// ======================================================================================================
// Tests 21 and 22: the only tests in this file that touch the real Credential Manager. Each holds
// `CREDMAN_LOCK` for its whole body, uses generated targets (never `knowlu/pending/session`, never a
// fixed name) and cleans up with a `Drop` guard, as `app/tests/account.rs` does.
// ======================================================================================================

#[cfg(windows)]
#[path = "support/credman_lock.rs"]
mod credman_lock;
#[cfg(windows)]
static CREDMAN_LOCK: credman_lock::CredmanLock = credman_lock::CredmanLock::new();

#[cfg(windows)]
mod real {
    use super::*;
    use knowlu::account::{load_session, save_session, Session};
    use std::io::{Read, Write};
    use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};
    use std::sync::Arc;

    /// Removes every named target when the test ends, on any exit path.
    pub struct Cleanup(pub Vec<String>);
    impl Drop for Cleanup {
        fn drop(&mut self) {
            for t in &self.0 {
                let _ = knowlu::credentials::delete(t);
            }
        }
    }

    /// The process-global `KNOWLU_API_BASE`, restored on any exit path.
    pub struct ApiBase(Option<std::ffi::OsString>);
    impl ApiBase {
        pub fn set(value: &str) -> ApiBase {
            let prev = std::env::var_os("KNOWLU_API_BASE");
            unsafe { std::env::set_var("KNOWLU_API_BASE", value) };
            ApiBase(prev)
        }
    }
    impl Drop for ApiBase {
        fn drop(&mut self) {
            match self.0.take() {
                Some(v) => unsafe { std::env::set_var("KNOWLU_API_BASE", v) },
                None => unsafe { std::env::remove_var("KNOWLU_API_BASE") },
            }
        }
    }

    /// A loopback server on `127.0.0.1:0` that answers queued `(status, body)` pairs in order (500
    /// once they run out) and records every raw request. Joined on drop.
    pub struct Server {
        pub base: String,
        pub seen: Arc<Mutex<Vec<String>>>,
        stop: Arc<AtomicBool>,
        join: Option<std::thread::JoinHandle<()>>,
    }
    impl Server {
        pub fn start(replies: Vec<(u16, String)>) -> Server {
            let listener = std::net::TcpListener::bind("127.0.0.1:0").expect("bind loopback");
            listener.set_nonblocking(true).unwrap();
            let base = format!("http://127.0.0.1:{}/functions/v1", listener.local_addr().unwrap().port());
            let (seen, stop) = (Arc::new(Mutex::new(Vec::new())), Arc::new(AtomicBool::new(false)));
            let (seen2, stop2) = (seen.clone(), stop.clone());
            let next = Arc::new(AtomicUsize::new(0));
            let join = std::thread::spawn(move || {
                while !stop2.load(Ordering::SeqCst) {
                    let Ok((mut stream, _)) = listener.accept() else {
                        std::thread::sleep(std::time::Duration::from_millis(5));
                        continue;
                    };
                    stream.set_nonblocking(false).unwrap();
                    stream.set_read_timeout(Some(std::time::Duration::from_secs(5))).unwrap();
                    let mut buf = Vec::new();
                    let mut chunk = [0u8; 2048];
                    let head_end = loop {
                        let n = stream.read(&mut chunk).unwrap_or(0);
                        if n == 0 { break buf.len(); }
                        buf.extend_from_slice(&chunk[..n]);
                        if let Some(i) = buf.windows(4).position(|w| w == b"\r\n\r\n") { break i + 4; }
                    };
                    let head = String::from_utf8_lossy(&buf[..head_end]).to_string();
                    let want: usize = head.lines().find(|l| l.to_ascii_lowercase().starts_with("content-length:"))
                        .and_then(|l| l.split(':').nth(1)?.trim().parse().ok()).unwrap_or(0);
                    while buf.len() < head_end + want {
                        let n = stream.read(&mut chunk).unwrap_or(0);
                        if n == 0 { break; }
                        buf.extend_from_slice(&chunk[..n]);
                    }
                    seen2.lock().unwrap().push(String::from_utf8_lossy(&buf).to_string());
                    let (status, body) = replies.get(next.fetch_add(1, Ordering::SeqCst)).cloned().unwrap_or((500, "{}".into()));
                    let reason = if (200..300).contains(&status) { "OK" } else { "Bad Request" };
                    let _ = write!(stream, "HTTP/1.1 {status} {reason}\r\ncontent-type: application/json\r\ncontent-length: {}\r\nconnection: close\r\n\r\n{body}", body.len());
                }
            });
            Server { base, seen, stop, join: Some(join) }
        }
        pub fn requests(&self) -> Vec<String> { self.seen.lock().unwrap().clone() }
    }
    impl Drop for Server {
        fn drop(&mut self) {
            self.stop.store(true, Ordering::SeqCst);
            if let Some(j) = self.join.take() { let _ = j.join(); }
        }
    }

    pub fn session(tag: &str) -> Session {
        Session { access_token: format!("at-{tag}"), refresh_token: format!("rt-{tag}"), expires_at: 4_000_000_000, email: "student@example.invalid".into() }
    }

    /// A scratch vault whose `config/cloud.yaml` names `api_base` (this build's own `api_base()`,
    /// the only host `cloud_config` accepts), the profile's generated session target and `account_id`.
    pub fn vault_naming(root: &Path, api_base: &str, target: &str, account_id: &str) -> PathBuf {
        let vault = root.join("vault");
        std::fs::create_dir_all(vault.join("config")).unwrap();
        std::fs::write(vault.join("config").join("cloud.yaml"),
            format!("api_base: '{api_base}'\nanon_key: 'anon'\nsession_credential_target: '{target}'\naccount_id: '{account_id}'\n")).unwrap();
        vault
    }

    pub fn tag(line: u32) -> String { format!("knowlu/test-o3-{}-{line}", std::process::id()) }
    pub fn saved(target: &str, account_id: &str, who: &str) -> Session {
        let s = session(who);
        save_session(target, account_id, &s).expect("write a generated test credential");
        s
    }
    pub fn load(target: &str) -> Result<(String, Session), String> { load_session(target) }
}

// ---- test 21: the re-attach (F2g) --------------------------------------------------------------------------

/// `cloud.yaml` names this build's own `api_base()`, so no environment is changed and no request is made.
#[cfg(windows)]
struct Attach {
    // Dropped in declaration order: the credentials are cleaned up before the lock is released.
    _clean: real::Cleanup,
    _lock: credman_lock::CredmanGuard,
    root: PathBuf,
    vault: PathBuf,
    pending: String,
    profile: String,
}
#[cfg(windows)]
fn attach_rig(line: u32, cloud_account: &str) -> Attach {
    let lock = CREDMAN_LOCK.lock().unwrap_or_else(|e| e.into_inner());
    let tag = real::tag(line);
    let (pending, profile) = (format!("{tag}/pending"), format!("{tag}/profile"));
    let clean = real::Cleanup(vec![pending.clone(), profile.clone()]);
    let root = scratch(&format!("t21-{line}"));
    let vault = real::vault_naming(&root, &knowlu::account::api_base(), &profile, cloud_account);
    Attach { _lock: lock, _clean: clean, root, vault, pending, profile }
}
#[cfg(windows)]
impl Attach {
    /// Runs the inner with counting seams; returns the answer, the forget count and the refresh count.
    fn run(&self, answer_id: &str) -> (serde_json::Value, usize, usize) {
        let (forgot, refreshed) = (Cell::new(0usize), Cell::new(0usize));
        let out = attach_session_in(&self.vault, &self.pending, answer_id, &|| forgot.set(forgot.get() + 1), &|| refreshed.set(refreshed.get() + 1));
        (out, forgot.get(), refreshed.get())
    }
}

#[cfg(windows)]
#[test]
fn test_21_a_pending_session_a_sign_in_answer_and_cloud_yaml_naming_one_account_move_onto_the_profile() {
    let r = attach_rig(line!(), "acc-1");
    let s = real::saved(&r.pending, "acc-1", "pending");
    let (out, forgot, refreshed) = r.run("acc-1");
    assert_eq!(out["ok"], true, "{out}");
    let (id, moved) = real::load(&r.profile).expect("the session is at cloud.yaml's target");
    assert_eq!((id.as_str(), moved), ("acc-1", s));
    assert!(real::load(&r.pending).is_err(), "the pending target is empty");
    assert_eq!(forgot, 0, "nothing is forgotten on success");
    assert_eq!(refreshed, 1, "the best-effort entitlement refresh runs once, as attach_account's does");
    let _ = &r.root;
}

#[cfg(windows)]
#[test]
fn test_21_pending_naming_another_account_leaves_the_profile_alone_and_is_forgotten_once() {
    for earlier in [false, true] {
        let r = attach_rig(line!(), "acc-1");
        real::saved(&r.pending, "acc-2", "other");
        let kept = earlier.then(|| real::saved(&r.profile, "acc-1", "earlier"));
        let (out, forgot, refreshed) = r.run("acc-1");
        assert_eq!(out["ok"], false, "{out}");
        assert_eq!(out["wrong_account"], true, "{out}");
        let sentence = out["error"].as_str().unwrap_or_default();
        assert!(!sentence.is_empty(), "a named refusal: {out}");
        assert!(!sentence.contains("acc-") && !sentence.contains("example.invalid"), "no email or id enters the sentence: {sentence}");
        assert_eq!(forgot, 1, "the forget seam is called once");
        assert_eq!(refreshed, 0);
        match kept {
            None => assert!(real::load(&r.profile).is_err(), "the profile's target stays empty"),
            Some(s) => assert_eq!(real::load(&r.profile).unwrap(), ("acc-1".to_string(), s), "the earlier session is byte-equal"),
        }
    }
}

#[cfg(windows)]
#[test]
fn test_21_a_sign_in_answer_naming_an_account_the_pending_session_does_not_is_the_same_refusal() {
    let r = attach_rig(line!(), "acc-1");
    real::saved(&r.pending, "acc-1", "stale");
    let (out, forgot, _) = r.run("acc-9");
    assert_eq!((out["ok"].clone(), out["wrong_account"].clone()), (false.into(), true.into()), "{out}");
    assert_eq!(forgot, 1);
    assert!(real::load(&r.profile).is_err(), "nothing moved");
}

#[cfg(windows)]
#[test]
fn test_21_no_pending_session_or_an_unreadable_cloud_yaml_is_a_named_refusal_with_nothing_moved() {
    // No pending session.
    let r = attach_rig(line!(), "acc-1");
    let (out, _, refreshed) = r.run("acc-1");
    assert_eq!(out["ok"], false, "{out}");
    assert!(!out["error"].as_str().unwrap_or_default().is_empty(), "{out}");
    assert!(real::load(&r.profile).is_err());
    assert_eq!(refreshed, 0);
    drop(r);
    // An unreadable cloud.yaml, with a pending session: refused, nothing moved, and the forget seam is
    // called, so no session is left behind a refusal.
    let r = attach_rig(line!(), "acc-1");
    real::saved(&r.pending, "acc-1", "pending");
    std::fs::write(r.vault.join("config").join("cloud.yaml"), "api_base: [unclosed\n").unwrap();
    let (out, forgot, refreshed) = r.run("acc-1");
    assert_eq!(out["ok"], false, "{out}");
    assert!(!out["error"].as_str().unwrap_or_default().is_empty(), "{out}");
    assert_eq!(forgot, 1, "an unreadable cloud.yaml still forgets the pending session");
    assert_eq!(refreshed, 0);
    assert!(real::load(&r.profile).is_err(), "nothing moved");
}

#[cfg(windows)]
#[test]
fn test_21_the_forget_is_local_the_pending_credential_goes_and_the_profiles_is_untouched() {
    let r = attach_rig(line!(), "acc-1");
    real::saved(&r.pending, "acc-2", "other");
    let earlier = real::saved(&r.profile, "acc-1", "earlier");
    // The command's own seam, over the generated pending target (the command passes the machine's).
    let out = attach_session_in(&r.vault, &r.pending, "acc-1", &|| forget_pending_local(&r.pending), &|| {});
    assert_eq!(out["ok"], false, "{out}");
    assert!(real::load(&r.pending).is_err(), "the refused account's pending credential is gone from this machine");
    assert_eq!(real::load(&r.profile).unwrap(), ("acc-1".to_string(), earlier), "the profile's session is untouched");
    // Nothing there to forget: the local forget ignores not-found, so the refusal still stands.
    forget_pending_local(&r.pending);
    let out = attach_session_in(&r.vault, &r.pending, "acc-1", &|| forget_pending_local(&r.pending), &|| {});
    assert_eq!(out["ok"], false, "{out}");
    assert!(!out["error"].as_str().unwrap_or_default().is_empty(), "the named refusal, never a delete error: {out}");
}

/// Source text with its comment lines removed, so a pin reads code and a comment may say "never X".
fn code_of(src: &str) -> String {
    src.lines().filter(|l| !l.trim_start().starts_with("//")).collect::<Vec<_>>().join("\n")
}

fn src_of(file: &str) -> String {
    let p = Path::new(env!("CARGO_MANIFEST_DIR")).join("src").join(file);
    code_of(&std::fs::read_to_string(&p).unwrap_or_else(|e| panic!("{}: {e}", p.display())))
}

/// The text of one top-level function: from its signature to the first line that is a lone `}`.
fn fn_body<'a>(src: &'a str, signature: &str) -> &'a str {
    let at = src.find(signature).unwrap_or_else(|| panic!("no `{signature}` in the source"));
    let rest = &src[at..];
    let end = rest.find("\n}").map(|i| i + 2).unwrap_or(rest.len());
    &rest[..end]
}

#[test]
fn test_21_f2gs_code_names_neither_sign_out_with_nor_a_logout_call() {
    let src = src_of("onboarding_state.rs");
    let region = format!(
        "{}\n{}\n{}",
        fn_body(&src, "pub fn forget_pending_local"),
        fn_body(&src, "pub fn attach_session_in"),
        fn_body(&src, "pub fn onboarding_attach_session"),
    );
    for banned in ["sign_out_with", "sign_out(", "/logout", "logout"] {
        assert!(!region.contains(banned), "F2g's code names {banned}: a revoke would sign the other account out everywhere");
    }
    assert!(region.contains("credentials::delete"), "the forget is a local credential delete");
}

// ---- test 22: the vault-target entitlement and checkout (F2h) ----------------------------------------------

/// A session at `cloud.yaml`'s generated target, a loopback listener, `KNOWLU_API_BASE` pointed at it
/// (`cloud_config` refuses a `cloud.yaml` naming another host than `api_base()`).
#[cfg(windows)]
struct Ent {
    // Dropped in declaration order: the credentials, then the environment, then the server, then the lock.
    _clean: real::Cleanup,
    _api: real::ApiBase,
    server: real::Server,
    _lock: credman_lock::CredmanGuard,
    vault: PathBuf,
    data_dir: PathBuf,
    token: String,
}
#[cfg(windows)]
fn ent_rig(line: u32, replies: Vec<(u16, String)>, with_session: bool) -> Ent {
    let lock = CREDMAN_LOCK.lock().unwrap_or_else(|e| e.into_inner());
    let target = format!("{}/profile", real::tag(line));
    let clean = real::Cleanup(vec![target.clone()]);
    let server = real::Server::start(replies);
    let api = real::ApiBase::set(&server.base);
    let root = scratch(&format!("t22-{line}"));
    let vault = real::vault_naming(&root, &knowlu::account::api_base(), &target, "acc-1");
    let token = format!("at-t22-{line}");
    if with_session {
        let s = knowlu::account::Session { access_token: token.clone(), refresh_token: "rt-t22".into(), expires_at: 4_000_000_000, email: "student@example.invalid".into() };
        knowlu::account::save_session(&target, "acc-1", &s).expect("write a generated test credential");
    }
    Ent { _clean: clean, _api: api, server, _lock: lock, vault, data_dir: root.join("appdata"), token }
}
#[cfg(windows)]
fn entitlement_body(status: &str) -> String {
    format!(r#"{{"status":"{status}","current_period_end":null,"plan":"monthly","checked_at":"2026-10-24T09:00:00.000Z"}}"#)
}
#[cfg(windows)]
fn bearer_of(req: &str, token: &str) -> bool {
    req.to_lowercase().contains(&format!("authorization: bearer {}", token.to_lowercase()))
}

#[cfg(windows)]
#[test]
fn test_22_the_poll_reads_the_vaults_own_session_and_an_active_or_trialing_reply_is_entitled() {
    for status in ["active", "trialing"] {
        let e = ent_rig(line!(), vec![(200, entitlement_body(status))], true);
        let out = entitlement_in(&e.vault, &e.data_dir);
        assert_eq!(out["ok"], true, "{out}");
        assert_eq!(out["status"], status);
        let seen = e.server.requests();
        assert_eq!(seen.len(), 1, "{seen:?}");
        assert!(seen[0].starts_with("GET /functions/v1/entitlement "), "{}", seen[0]);
        assert!(bearer_of(&seen[0], &e.token), "the listener sees the vault session's bearer: {}", seen[0]);
        let cs = knowlu::state::ConsoleState::open(e.vault.clone(), e.data_dir.clone());
        assert_eq!(knowlu::scheduler::entitlement_state(&cs), knowlu::account::EntitlementState::Entitled, "{status} saves the cache");
    }
}

#[cfg(windows)]
#[test]
fn test_22_a_refusal_leaves_the_cache_as_it_was_and_answers_with_the_services_sentence() {
    let e = ent_rig(line!(), vec![(403, r#"{"error":"no subscription on this account"}"#.into())], true);
    let before = knowlu::account::EntitlementCache { status: "canceled".into(), current_period_end: None, plan: None, checked_at: "2026-10-01T00:00:00.000Z".into() };
    knowlu::account::save_cache(&e.data_dir, &before).unwrap();
    let bytes = std::fs::read(knowlu::account::cache_path(&e.data_dir)).unwrap();
    let out = entitlement_in(&e.vault, &e.data_dir);
    assert_eq!(out["ok"], false, "{out}");
    assert_eq!(out["error"], "no subscription on this account");
    assert_eq!(std::fs::read(knowlu::account::cache_path(&e.data_dir)).unwrap(), bytes, "the cache is untouched");
}

#[cfg(windows)]
#[test]
fn test_22_no_session_at_the_profiles_target_is_a_named_error_and_no_request_is_made() {
    let e = ent_rig(line!(), vec![], false);
    let out = entitlement_in(&e.vault, &e.data_dir);
    assert_eq!(out["ok"], false, "{out}");
    assert!(!out["error"].as_str().unwrap_or_default().is_empty(), "{out}");
    let checkout = checkout_url_in(&e.vault, "monthly");
    assert!(checkout.is_err(), "{checkout:?}");
    assert!(e.server.requests().is_empty(), "no request: {:?}", e.server.requests());
}

#[cfg(windows)]
#[test]
fn test_22_checkout_records_consent_then_asks_for_the_link_with_the_vault_sessions_bearer_and_opens_nothing() {
    let e = ent_rig(line!(), vec![(200, "{}".into()), (200, r#"{"url":"https://checkout.example.invalid/c/test"}"#.into())], true);
    let url = checkout_url_in(&e.vault, "monthly").expect("the checkout link");
    assert_eq!(url, "https://checkout.example.invalid/c/test");
    let seen = e.server.requests();
    assert_eq!(seen.len(), 2, "{seen:?}");
    assert!(seen[0].starts_with("POST /functions/v1/account/consent "), "{}", seen[0]);
    assert!(seen[1].starts_with("POST /functions/v1/billing-checkout "), "{}", seen[1]);
    assert!(seen.iter().all(|r| bearer_of(r, &e.token)), "both calls carry the vault session's bearer");
    assert!(seen[1].contains("monthly"));
}

#[test]
fn test_22_the_pending_target_is_named_only_inside_f2gs_command_wrapper() {
    let src = src_of("onboarding_state.rs");
    let wrapper = fn_body(&src, "pub fn onboarding_attach_session");
    let start = wrapper.as_ptr() as usize - src.as_ptr() as usize;
    let range = start..start + wrapper.len();
    let hits: Vec<usize> = src.match_indices("PENDING_TARGET").map(|(i, _)| i).collect();
    assert!(!hits.is_empty(), "F2g's command passes account::PENDING_TARGET and a local forget");
    for i in hits {
        assert!(range.contains(&i), "PENDING_TARGET is named outside onboarding_attach_session (at byte {i})");
    }
    // The checkout core returns the URL; the command adds `open_in_browser`.
    let core = fn_body(&src, "pub fn checkout_url_in");
    assert!(core.contains("checkout_url_at"), "the core calls checkout_url_at");
    assert!(!core.contains("open_in_browser"), "the core opens nothing");
}

// ======================================================================================================
// F1 review r1: the removed courses (test 12, R-C1-51), the "isn't listed" school (test 1), the real
// pending store (test 26) and the real-store sign-out wrapper (test 17).
// ======================================================================================================

/// A captured row: `code` is the human code, `key` the LMS course key (the row's identity, F7a). Its
/// one saved fragment is the human code; a test that needs a gradebook pin adds a column id itself.
fn row(code: &str, name: &str, key: &str, removed: bool) -> CourseAnswer {
    CourseAnswer {
        code: code.into(), name: name.into(), label: name.split(' ').next().unwrap().into(), lms_key: key.into(), removed,
        slug: String::new(), fragments: vec![code.to_string()],
    }
}

fn counts(pairs: &[(&str, i64)]) -> BTreeMap<String, Option<i64>> { pairs.iter().map(|(k, v)| (k.to_string(), Some(*v))).collect() }
/// A bucket as the engine writes it: named by its sorted keys, carrying them as `codes`.
fn put_bucket(a: &mut Answers, codes: &[&str], c: &[(&str, i64)]) {
    let mut sorted: Vec<String> = codes.iter().map(|s| s.to_string()).collect();
    sorted.sort();
    a.buckets.insert(bucket_key(codes), Bucket { codes: sorted, counts: counts(c) });
}
/// A total as `step3_totals` gives it: a number, or `None` (absent) / `Some(None)` (null).
fn total(t: &BTreeMap<String, Option<i64>>, name: &str) -> Option<i64> { t.get(name).copied().flatten() }

/// Three captured courses with `school.preview`'s match-set buckets (§14.45), keyed by LMS course key
/// as the engine keys them; the middle (KNT, `_1002_1`) is removed. QLT and KNT share the
/// "_1001_1,_1002_1" bucket (an item both claim), WVG has its own, and one item matched no course.
fn three_with_one_removed() -> Answers {
    let mut a = Answers {
        courses: vec![
            row("QLT 101", "Quilting Basics", "_1001_1", false),
            row("KNT 220", "Knotwork Theory", "_1002_1", true),
            row("WVG 105", "Weaving Lab", "_1003_1", false),
        ],
        ..Default::default()
    };
    put_bucket(&mut a, &["_1001_1"], &[("class_times", 3), ("assignments_upcoming", 7), ("turned_in", 2)]);
    put_bucket(&mut a, &["_1002_1"], &[("class_times", 2), ("assignments_upcoming", 11), ("turned_in", 5)]);
    put_bucket(&mut a, &["_1003_1"], &[("class_times", 1), ("assignments_upcoming", 4), ("turned_in", 0)]);
    put_bucket(&mut a, &["_1002_1", "_1001_1"], &[("assignments_upcoming", 100)]);
    put_bucket(&mut a, &[], &[("assignments_upcoming", 1000)]);
    a
}

#[test]
fn test_01_a_school_typed_as_isnt_listed_has_no_unitid_and_round_trips() {
    let d = scratch("t01-nounitid");
    let mut s = full_state();
    s.school = Some(School { unitid: None, label: "Example Typed College".into(), path: SchoolPath::Paste, confirmed: true });
    save_state(&d, &s).unwrap();
    assert_eq!(load_state(&d), Loaded::Ok(s));
}

#[test]
fn test_12_kept_courses_leaves_out_a_removed_row_and_keeps_the_captured_order() {
    let a = three_with_one_removed();
    let kept: Vec<&str> = a.kept_courses().iter().map(|c| c.code.as_str()).collect();
    assert_eq!(kept, ["QLT 101", "WVG 105"]);
}

#[test]
fn test_12_the_course_key_is_the_lms_key_then_the_slug_then_the_label_like_by_course_and_never_the_human_code() {
    let mut c = row("QLT 101", "Quilting Basics", "_1001_1", false);
    c.slug = "quilting".into();
    assert_eq!(c.key(), "_1001_1", "the LMS course key, as the engine's by_course keys it");
    c.lms_key = String::new();
    assert_eq!(c.key(), "quilting", "no LMS key: the slug, not the human code");
    c.slug = String::new();
    assert_eq!(c.key(), "Quilting", "no LMS key and no slug: the label, not the human code");
}

#[test]
fn test_12_step_3s_totals_add_every_bucket_that_is_unmatched_or_names_a_kept_course() {
    let a = three_with_one_removed();
    let t = step3_totals(&a);
    assert_eq!(total(&t, "classes"), Some(2), "{t:?}");
    assert_eq!(total(&t, "class_times"), Some(4), "QLT 3 + WVG 1; the removed course's own bucket (2) is out: {t:?}");
    // 7 (QLT) + 4 (WVG) + 100 (shared by a kept and a removed course: still imported) + 1000 (unmatched,
    // still imported course-less); the removed course's own 11 is out.
    assert_eq!(total(&t, "assignments_upcoming"), Some(1111), "{t:?}");
    assert_eq!(total(&t, "turned_in"), Some(2), "2 + 0; the removed course's 5 is out: {t:?}");
}

#[test]
fn test_12_a_two_code_bucket_stays_while_one_of_its_courses_is_kept_and_drops_when_both_are_removed() {
    let mut a = three_with_one_removed();
    let shared = |a: &Answers| total(&step3_totals(a), "assignments_upcoming");
    assert_eq!(shared(&a), Some(1111), "KNT removed, QLT kept: the shared bucket stays");
    a.courses[0].removed = true; // QLT removed too: only WVG and unmatched remain
    assert_eq!(shared(&a), Some(1004), "both removed: the shared 100 and both own buckets are out; WVG 4 + unmatched 1000");
    a.courses[1].removed = false; // KNT kept again, QLT removed
    assert_eq!(shared(&a), Some(1115), "KNT 11 + shared 100 + WVG 4 + unmatched 1000");
    let mut none_kept = three_with_one_removed();
    for c in &mut none_kept.courses { c.removed = true; }
    let t = step3_totals(&none_kept);
    assert_eq!((total(&t, "classes"), total(&t, "assignments_upcoming")), (Some(0), Some(1000)), "only the unmatched bucket: {t:?}");
}

#[test]
fn test_12_a_re_capture_clears_the_removed_mark_a_captured_rows_own_fragments_win_and_an_empty_one_keeps_the_old() {
    let mut a = three_with_one_removed();
    a.courses[0].fragments = vec!["QLT 101".into(), "_71_1".into()];
    a.courses[1].fragments = vec!["KNT 220".into(), "_72_1".into()];
    // QLT comes back with a new gradebook column pinned; KNT comes back with no fragments of its own.
    let mut fresh_qlt = row("QLT 101", "Quilting Basics", "_1001_1", false);
    fresh_qlt.fragments = vec!["QLT 101".into(), "_71_1".into(), "_79_1".into()];
    let mut fresh_knt = row("KNT 220", "Knotwork Theory", "_1002_1", false);
    fresh_knt.fragments = Vec::new();
    a.recapture(vec![fresh_qlt, fresh_knt]);
    assert!(a.courses.iter().all(|c| !c.removed), "a re-capture is the student asking again: {:?}", a.courses);
    assert_eq!(a.kept_courses().len(), 2);
    assert_eq!(a.courses[0].fragments, ["QLT 101", "_71_1", "_79_1"], "the re-captured row's own pairs win: the new pin is kept");
    assert_eq!(a.courses[1].fragments, ["KNT 220", "_72_1"], "none of its own, same key: the saved pairs stay");
}

#[test]
fn test_12_remove_and_re_capture_identify_a_row_by_its_key_when_it_has_no_lms_key() {
    let d = scratch("t12-nocode");
    let mut codeless = row("LAB 1", "Lab Section", "", false);
    codeless.slug = "lab-section".into();
    let capture = serde_json::json!({"capture": {"courses": [row("QLT 101", "Quilting Basics", "_1001_1", false), codeless.clone()]}});
    assert_eq!(onboarding_answer_inner(&d, 3, &capture, &mut |_| {})["ok"], true);
    assert_eq!(onboarding_answer_inner(&d, 3, &serde_json::json!({"remove": "lab-section"}), &mut |_| {})["ok"], true, "removed by its slug, since it has no LMS key");
    let a = load_answers(&d).unwrap().unwrap();
    assert!(a.courses[1].removed && !a.courses[0].removed, "{:?}", a.courses);
    assert_eq!(onboarding_answer_inner(&d, 3, &capture, &mut |_| {})["ok"], true);
    assert_eq!(load_answers(&d).unwrap().unwrap().kept_courses().len(), 2, "the re-capture cleared it");
}

#[test]
fn test_12_a_removal_given_through_the_answer_inner_survives_a_reload_and_a_re_capture_clears_it() {
    let d = scratch("t12-removed");
    let courses: Vec<CourseAnswer> = three_with_one_removed().courses.into_iter().map(|c| CourseAnswer { removed: false, ..c }).collect();
    let capture = serde_json::json!({"capture": {"courses": courses}});
    let out = onboarding_answer_inner(&d, 3, &capture, &mut |_| {});
    assert_eq!(out["ok"], true, "{out}");
    let out = onboarding_answer_inner(&d, 3, &serde_json::json!({"remove": "_1002_1"}), &mut |_| {});
    assert_eq!(out["ok"], true, "{out}");
    // Nothing in memory: a fresh read of the file.
    let reloaded = load_answers(&d).unwrap().expect("the answers file");
    let kept: Vec<&str> = reloaded.kept_courses().iter().map(|c| c.code.as_str()).collect();
    assert_eq!(kept, ["QLT 101", "WVG 105"], "the removal was saved the moment it was given");
    assert!(reloaded.courses.iter().any(|c| c.code == "KNT 220" && c.removed), "the row stays, marked: {:?}", reloaded.courses);
    onboarding_answer_inner(&d, 3, &capture, &mut |_| {});
    assert_eq!(load_answers(&d).unwrap().unwrap().kept_courses().len(), 3, "a re-capture clears the mark");
}

fn three_pairs() -> BTreeMap<String, Vec<String>> {
    [("_1001_1", ["QLT 101", "_71_1"]), ("_1002_1", ["KNT 220", "_72_1"]), ("_1003_1", ["WVG 105", "_73_1"])]
        .iter().map(|(k, f)| (k.to_string(), f.iter().map(|s| s.to_string()).collect())).collect()
}

#[test]
fn test_12_buckets_and_pairs_come_only_through_f8bs_seams_and_the_answer_inner_refuses_both_shapes() {
    let d = scratch("t12-buckets");
    let a = three_with_one_removed();
    // The page's capture carries fragments of its own: they are dropped (a page never supplies pairs).
    let forged: Vec<CourseAnswer> = a.courses.iter().cloned().map(|c| CourseAnswer { removed: false, fragments: vec!["FORGED".into()], ..c }).collect();
    let capture = serde_json::json!({"capture": {"courses": forged}});
    assert_eq!(onboarding_answer_inner(&d, 3, &capture, &mut |_| {})["ok"], true);
    assert!(load_answers(&d).unwrap().unwrap().courses.iter().all(|c| c.fragments.is_empty()), "no pair from a page answer");
    // F8b's two seams.
    save_pairs(&d, &three_pairs()).expect("save_pairs");
    save_preview(&d, &serde_json::to_value(&a.buckets).unwrap()).expect("save_preview");
    // The page cannot post either: refused, nothing enqueued, not a byte of the file changed.
    let before = std::fs::read(answers_path(&d)).unwrap();
    let forged_buckets = serde_json::json!({"buckets": {"unmatched": {"codes": [], "assignments_upcoming": 5}}});
    let forged_pairs = serde_json::json!({"pairs": {"_1002_1": ["FORGED"]}});
    for shape in [forged_buckets, forged_pairs] {
        let out = onboarding_answer_inner(&d, 3, &shape, &mut |j| panic!("a refused answer enqueued {j}"));
        assert_eq!(out["ok"], false, "{shape} is F8b's, never a page answer: {out}");
        assert_eq!(std::fs::read(answers_path(&d)).unwrap(), before, "{shape} changed the answers file");
    }
    // A re-capture through the page keeps the saved pairs, not the page's.
    assert_eq!(onboarding_answer_inner(&d, 3, &capture, &mut |_| {})["ok"], true);
    onboarding_answer_inner(&d, 3, &serde_json::json!({"remove": "_1002_1"}), &mut |_| {});
    let saved = load_answers(&d).unwrap().unwrap();
    assert_eq!(saved.buckets, a.buckets);
    let frags: Vec<Vec<String>> = saved.courses.iter().map(|c| c.fragments.clone()).collect();
    assert_eq!(frags, three_pairs().into_values().collect::<Vec<_>>(), "save_pairs' lists, by key");
    assert_eq!(step3_totals(&saved), step3_totals(&a), "the same totals as the in-memory answers");
}

#[test]
fn test_12_an_engine_shaped_by_course_with_nulls_is_saved_and_totals_null_where_its_input_was_missing() {
    // `onboarding --preview` with no grades bundle and no registrar: `turned_in` and `class_times`
    // are `null` in every bucket (F7a), never 0.
    let by_course = serde_json::json!({
        "_1001_1": {"assignments_upcoming": 7, "class_times": null, "codes": ["_1001_1"], "turned_in": null},
        "_1002_1": {"assignments_upcoming": 11, "class_times": null, "codes": ["_1002_1"], "turned_in": null},
        "_1001_1,_1002_1": {"assignments_upcoming": 100, "class_times": null, "codes": ["_1001_1", "_1002_1"], "turned_in": null},
        "unmatched": {"assignments_upcoming": 2, "class_times": null, "codes": [], "turned_in": null},
    });
    let parsed: BTreeMap<String, Bucket> = serde_json::from_value(by_course.clone()).expect("a Bucket holds the engine's nulls");
    assert_eq!(parsed["unmatched"].counts.get("turned_in"), Some(&None), "null, not 0 and not absent");
    let d = scratch("t12-nulls");
    let courses = vec![row("QLT 101", "Quilting Basics", "_1001_1", false), row("KNT 220", "Knotwork Theory", "_1002_1", false)];
    assert_eq!(onboarding_answer_inner(&d, 3, &serde_json::json!({"capture": {"courses": courses}}), &mut |_| {})["ok"], true);
    save_preview(&d, &by_course).expect("the paste fallback's preview is saved, nulls and all");
    onboarding_answer_inner(&d, 3, &serde_json::json!({"remove": "_1002_1"}), &mut |_| {});
    let saved = load_answers(&d).unwrap().unwrap();
    assert_eq!(saved.buckets, parsed, "the file keeps the nulls");
    let t = step3_totals(&saved);
    assert_eq!(t.get("classes"), Some(&Some(1)), "{t:?}");
    assert_eq!(t.get("assignments_upcoming"), Some(&Some(109)), "7 + shared 100 + unmatched 2: {t:?}");
    assert_eq!(t.get("turned_in"), Some(&None), "no grades bundle: null, never 0: {t:?}");
    assert_eq!(t.get("class_times"), Some(&None), "no registrar: null, never 0: {t:?}");
}

#[test]
fn test_12_two_shells_sharing_a_human_code_are_two_keys_and_removing_one_leaves_the_other() {
    // "KNT 220" from spring (`_0990_1`) and from fall (`_1002_1`): one human code, two LMS keys.
    let d = scratch("t12-shells");
    let (spring, fall) = (row("KNT 220", "Knotwork Theory", "_0990_1", false), row("KNT 220", "Knotwork Theory", "_1002_1", false));
    assert_eq!(onboarding_answer_inner(&d, 3, &serde_json::json!({"capture": {"courses": [spring, fall]}}), &mut |_| {})["ok"], true);
    save_preview(&d, &serde_json::json!({
        "_0990_1": {"assignments_upcoming": 3, "class_times": 0, "codes": ["_0990_1"], "turned_in": 0},
        "_1002_1": {"assignments_upcoming": 5, "class_times": 1, "codes": ["_1002_1"], "turned_in": 0},
        "_0990_1,_1002_1": {"assignments_upcoming": 40, "class_times": 0, "codes": ["_0990_1", "_1002_1"], "turned_in": 0},
        "unmatched": {"assignments_upcoming": 0, "class_times": 0, "codes": [], "turned_in": 0},
    })).unwrap();
    assert_eq!(onboarding_answer_inner(&d, 3, &serde_json::json!({"remove": "_0990_1"}), &mut |_| {})["ok"], true);
    let a = load_answers(&d).unwrap().unwrap();
    let kept: Vec<&str> = a.kept_courses().iter().map(|c| c.lms_key.as_str()).collect();
    assert_eq!(kept, ["_1002_1"], "only the spring shell is removed, never both: {:?}", a.courses);
    let t = step3_totals(&a);
    assert_eq!((total(&t, "classes"), total(&t, "assignments_upcoming"), total(&t, "class_times")), (Some(1), Some(45), Some(1)), "fall 5 + shared 40: {t:?}");
    let listed = preview_courses(&a);
    let codes: Vec<&str> = listed.as_array().unwrap().iter().map(|c| c["code"].as_str().unwrap()).collect();
    assert_eq!(codes, ["_0990_1", "_1002_1"], "the engine is given two keys, so it buckets two shells apart");
}

#[test]
fn test_12_the_answer_inner_saves_before_it_enqueues_and_enqueues_exactly_the_jobs_the_answer_asks_for() {
    let d = scratch("t12-order");
    let (enqueued, saw) = (std::cell::RefCell::new(Vec::<String>::new()), std::cell::RefCell::new(Vec::<bool>::new()));
    let courses = vec![row("QLT 101", "Quilting Basics", "_1001_1", false)];
    let answer = |step: u8, a: serde_json::Value, held_by_then: &dyn Fn(&Answers) -> bool| {
        enqueued.borrow_mut().clear();
        saw.borrow_mut().clear();
        let dir = d.clone();
        let out = onboarding_answer_inner(&d, step, &a, &mut |j| {
            enqueued.borrow_mut().push(j.to_string());
            saw.borrow_mut().push(matches!(load_answers(&dir), Ok(Some(ref a)) if held_by_then(a)));
        });
        assert_eq!(out["ok"], true, "{out}");
        assert!(saw.borrow().iter().all(|s| *s), "the file already held the answer when each job was enqueued: {:?}", saw.borrow());
        (enqueued.borrow().clone(), saw.borrow().len())
    };
    let (jobs, checked) = answer(3, serde_json::json!({"capture": {"courses": courses}}), &|a| a.courses.len() == 1);
    assert_eq!(jobs, ["school.preview"], "step 3's capture asks for the preview");
    assert_eq!(checked, 1, "and the save was checked at that enqueue, not vacuously");
    let (jobs, checked) = answer(6, serde_json::json!({"google_calendar": true}), &|a| a.google_calendar == Some(true));
    assert_eq!(jobs, ["google.calendar_read"]);
    assert_eq!(checked, 1);
    let (jobs, _) = answer(6, serde_json::json!({"google_calendar": false}), &|a| a.google_calendar == Some(false));
    assert!(jobs.is_empty(), "a declined calendar asks for nothing: {jobs:?}");
    let (jobs, _) = answer(3, serde_json::json!({"remove": "_1001_1"}), &|a| a.courses[0].removed);
    assert!(jobs.is_empty(), "a Remove asks for no job: {jobs:?}");
    let (jobs, _) = answer(8, serde_json::json!({"zybooks_ignore": "ZYB-ONBOARDING"}), &|a| a.zybooks_ignore == ["ZYB-ONBOARDING"]);
    assert!(jobs.is_empty(), "{jobs:?}");
    let refused = onboarding_answer_inner(&d, 6, &serde_json::json!({"nonsense": 1}), &mut |j| panic!("enqueued {j} for a refused answer"));
    assert_eq!(refused["ok"], false, "{refused}");
}

// ---- the real pending store (plan line 1916) and the real-store sign-out wrapper (test 17) ----------------
// These two are the only additions here that touch the real Credential Manager beyond tests 21 and 22:
// both hold `CREDMAN_LOCK`, use a generated profile id, and clean up with a `Drop` guard.

#[cfg(windows)]
fn pending_targets(profile_id: &str) -> Vec<String> {
    [PENDING_LMS, PENDING_CALENDAR].iter().map(|n| knowlu::credentials::target_for(profile_id, n)).collect()
}

#[cfg(windows)]
#[test]
fn test_26_cred_pending_round_trips_put_get_delete_under_knowlu_profile_pending_names_and_deleting_nothing_is_fine() {
    // Declared lock first, cleanup second: dropped in reverse, so the credentials go before the lock is released.
    let _lock = CREDMAN_LOCK.lock().unwrap_or_else(|e| e.into_inner());
    let id = format!("test-o3-{}-{}", std::process::id(), line!());
    let _clean = real::Cleanup(pending_targets(&id));
    let store = CredPending { profile_id: id.clone() };
    assert_eq!(store.get(PENDING_LMS), None, "nothing there yet");
    store.put(PENDING_LMS, LMS_LINK).expect("put");
    assert_eq!(store.get(PENDING_LMS).as_deref(), Some(LMS_LINK));
    assert!(knowlu::credentials::exists(&format!("knowlu/{id}/{PENDING_LMS}")), "the credential is at knowlu/<profile_id>/<name>");
    assert_eq!(store.get(PENDING_CALENDAR), None, "the other name is its own credential");
    store.put(PENDING_LMS, CAL_LINK).expect("a second put replaces");
    assert_eq!(store.get(PENDING_LMS).as_deref(), Some(CAL_LINK));
    store.delete(PENDING_LMS);
    assert_eq!(store.get(PENDING_LMS), None);
    assert!(!knowlu::credentials::exists(&format!("knowlu/{id}/{PENDING_LMS}")));
    store.delete(PENDING_LMS); // not-found is not an error: this must not panic
    assert_eq!(pending_name("lms_ics"), Some(PENDING_LMS));
    assert_eq!(pending_name("calendar_ics"), Some(PENDING_CALENDAR));
    assert_eq!(pending_name("nonsense"), None);
}

#[cfg(windows)]
#[test]
fn test_17_the_real_store_wrapper_deletes_this_profiles_pending_links_rewinds_and_leaves_another_profiles_alone() {
    let _lock = CREDMAN_LOCK.lock().unwrap_or_else(|e| e.into_inner());
    let (id, other) = (format!("test-o3-{}-{}", std::process::id(), line!()), format!("test-o3-{}-{}-other", std::process::id(), line!()));
    let mut targets = pending_targets(&id);
    targets.extend(pending_targets(&other));
    let _clean = real::Cleanup(targets);
    let (mine, theirs) = (CredPending { profile_id: id.clone() }, CredPending { profile_id: other });
    mine.put(PENDING_LMS, LMS_LINK).unwrap();
    mine.put(PENDING_CALENDAR, CAL_LINK).unwrap();
    theirs.put(PENDING_LMS, LMS_LINK).unwrap();
    let d = scratch("t17-wrapper");
    let before = full_state();
    save_state(&d, &before).unwrap();
    save_answers(&d, &full_answers()).unwrap();
    on_sign_out(&d, &id).expect("the wrapper over the real store");
    assert!(mine.get(PENDING_LMS).is_none() && mine.get(PENDING_CALENDAR).is_none(), "both of this profile's links are gone");
    assert_eq!(theirs.get(PENDING_LMS).as_deref(), Some(LMS_LINK), "another profile's link is untouched");
    assert!(!answers_path(&d).exists());
    let Loaded::Ok(after) = load_state(&d) else { panic!("rewritten, not removed") };
    assert_eq!((after.step, after.finished, after.school.is_none()), (2, false, true));
    on_sign_out(&d, &id).expect("a second sign-out with nothing left to delete is not an error");
}

// ---- F1 review r2 / §14.46: bucket codes, one source of pairs -------------------------------------------------

#[test]
fn test_12_the_app_reads_a_buckets_codes_and_never_parses_its_name() {
    let mut a = Answers { courses: vec![row("WVG 105", "Weaving Lab", "_1003_1", false), row("KNT 220", "Knotwork Theory", "_1002_1", true)], ..Default::default() };
    // Names that parse to nothing sensible: only `codes` counts.
    a.buckets.insert("zz-opaque-1".into(), Bucket { codes: vec!["_1003_1".into()], counts: counts(&[("class_times", 2)]) });
    a.buckets.insert("zz-opaque-2".into(), Bucket { codes: vec!["_1002_1".into()], counts: counts(&[("class_times", 40)]) });
    a.buckets.insert("zz-opaque-3".into(), Bucket { codes: vec![], counts: counts(&[("class_times", 300)]) });
    // A zero single-course bucket (the engine always writes one) changes nothing.
    a.buckets.insert(bucket_key(&["_1003_1"]), Bucket { codes: vec!["_1003_1".into()], counts: counts(&[("class_times", 0)]) });
    assert_eq!(total(&step3_totals(&a), "class_times"), Some(302), "2 + 300; the removed course's 40 is out: {:?}", step3_totals(&a));
}

/// The engine's key for one `--courses` entry (F7a, `onboarding.rs`'s `read_courses`): `code`, then
/// `slug`, then `label`, each trimmed, the first that is not empty.
fn engine_key(entry: &serde_json::Value) -> String {
    ["code", "slug", "label"].iter().map(|k| entry[*k].as_str().unwrap_or("").trim().to_string()).find(|t| !t.is_empty()).unwrap_or_default()
}

#[test]
fn test_12_bucket_names_are_the_sorted_keys_joined_by_a_comma_and_the_engines_key_is_the_apps_for_a_row_with_no_lms_key() {
    assert_eq!(bucket_key(&["_1002_1", "_1001_1"]), "_1001_1,_1002_1");
    assert_eq!(bucket_key(&[]), "unmatched");
    // No LMS key: one row falls back to its slug, one (no slug either) to its label.
    let d = scratch("t12-fallback");
    let mut by_slug = row("LAB 1", "Lab Section", "", false);
    by_slug.slug = "lab-section".into();
    let by_label = row("LAB 2", "Lab", "", false);
    let qlt = row("QLT 101", "Quilting Basics", "_1001_1", false);
    let capture = serde_json::json!({"capture": {"courses": [qlt, by_slug, by_label]}});
    assert_eq!(onboarding_answer_inner(&d, 3, &capture, &mut |_| {})["ok"], true);
    let a = load_answers(&d).unwrap().unwrap();
    let listed = preview_courses(&a);
    let entries = listed.as_array().expect("one entry per row");
    for (entry, course) in entries.iter().zip(&a.courses) {
        assert_eq!(engine_key(entry), course.key(), "the engine buckets this row under the app's key: {entry}");
        assert_eq!(entry["slug"].as_str().unwrap_or(""), course.slug, "the slug is sent, so the engine's fallback is the app's: {entry}");
    }
    assert_eq!(entries.iter().map(engine_key).collect::<Vec<_>>(), ["_1001_1", "lab-section", "Lab"]);
    // The engine's by_course for that list, keyed as it would key it, counts each kept row.
    let mut by_course = serde_json::Map::new();
    for (entry, n) in entries.iter().zip([1, 10, 100]) {
        let k = engine_key(entry);
        by_course.insert(k.clone(), serde_json::json!({"assignments_upcoming": n, "class_times": 0, "codes": [k], "turned_in": 0}));
    }
    by_course.insert("unmatched".into(), serde_json::json!({"assignments_upcoming": 0, "class_times": 0, "codes": [], "turned_in": 0}));
    save_preview(&d, &serde_json::Value::Object(by_course)).unwrap();
    assert_eq!(total(&step3_totals(&load_answers(&d).unwrap().unwrap()), "assignments_upcoming"), Some(111), "no bucket drops out of the totals");
    onboarding_answer_inner(&d, 3, &serde_json::json!({"remove": "Lab"}), &mut |_| {});
    assert_eq!(total(&step3_totals(&load_answers(&d).unwrap().unwrap()), "assignments_upcoming"), Some(11), "the label-keyed row is out");
}

#[test]
fn test_12_a_label_edit_is_display_only_and_the_preview_courses_carry_the_saved_pairs_unchanged() {
    let d = scratch("t12-pairs");
    let give = |step: u8, a: serde_json::Value| {
        let out = onboarding_answer_inner(&d, step, &a, &mut |_| {});
        assert_eq!(out["ok"], true, "{out}");
    };
    let mut qlt = row("QLT 101", "Quilting Basics", "_1001_1", false);
    qlt.fragments = Vec::new();
    give(3, serde_json::json!({"capture": {"courses": [qlt]}}));
    let pairs: BTreeMap<String, Vec<String>> = [("_1001_1".to_string(), vec!["QLT 101".to_string(), "QLT-101".into(), "_71_1".into()])].into();
    save_pairs(&d, &pairs).unwrap();
    let before = preview_courses(&load_answers(&d).unwrap().unwrap());
    give(3, serde_json::json!({"label": {"key": "_1001_1", "label": "Quilting, edited"}}));
    let after = preview_courses(&load_answers(&d).unwrap().unwrap());
    assert_eq!(after[0]["label"], "Quilting, edited");
    assert_eq!(after[0]["code"], "_1001_1", "the engine's code is the row's key");
    assert_eq!(after[0]["pairs"], serde_json::json!(["QLT 101", "QLT-101", "_71_1"]));
    assert_eq!(serde_json::to_string(&before[0]["pairs"]).unwrap(), serde_json::to_string(&after[0]["pairs"]).unwrap(), "byte for byte");
}

#[test]
fn test_12_every_preview_entry_carries_pairs_and_a_row_with_fragments_never_sends_an_empty_list() {
    // Engine 4c9ca48: a present `pairs` array, even `[]`, is the course's whole fragment list, so an
    // empty one would claim nothing and match no class time.
    let mut a = three_with_one_removed();
    a.courses[0].fragments = vec!["QLT 101".into(), "_71_1".into()];
    a.courses[2].fragments = Vec::new(); // none saved yet
    let listed = preview_courses(&a);
    let entries = listed.as_array().expect("one entry per row");
    assert_eq!(entries.len(), 3, "removed rows included");
    for (entry, course) in entries.iter().zip(&a.courses) {
        let pairs = entry.get("pairs").and_then(|p| p.as_array()).unwrap_or_else(|| panic!("no pairs array: {entry}"));
        assert!(!pairs.is_empty(), "never an empty list for {}: {entry}", course.code);
        if !course.fragments.is_empty() {
            assert_eq!(entry["pairs"], serde_json::json!(course.fragments), "exactly the saved fragments");
        }
    }
    assert_eq!(entries[2]["pairs"], serde_json::json!(["WVG 105"]), "none saved: the hand-over's own fallback, the human code");
}
