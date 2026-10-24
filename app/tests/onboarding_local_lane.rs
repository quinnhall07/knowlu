//! O3 F8c: the local lane's runners beside F1's tests 12-14 (`onboarding_jobs.rs`): the feed store's
//! edges (no link, no feed, the calendar kind), `change_school`'s jobs, and `config.ingest` as the lane
//! runs it (login flags from step 8's rows, the pending links settled after the write, a feed still
//! being validated holds the job). No window, no network, no engine child. Invented data only.
use knowlu::onboarding_jobs::*;
use knowlu::onboarding_state::*;
use std::collections::BTreeMap;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Mutex;

const LMS_LINK: &str = "https://lms.example.invalid/feed/tok-8c01.ics";
const CAL_LINK: &str = "https://cal.example.invalid/private/tok-8c02.ics";

fn scratch(tag: &str) -> PathBuf {
    let d = std::env::temp_dir().join(format!("knowlu-o3-lane-{tag}-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&d);
    std::fs::create_dir_all(&d).unwrap();
    d
}

#[derive(Default)]
struct Pending(Mutex<BTreeMap<String, String>>);
impl PendingStore for Pending {
    fn put(&self, name: &str, link: &str) -> Result<(), String> { self.0.lock().unwrap().insert(name.into(), link.into()); Ok(()) }
    fn get(&self, name: &str) -> Option<String> { self.0.lock().unwrap().get(name).cloned() }
    fn delete(&self, name: &str) -> Result<(), String> { self.0.lock().unwrap().remove(name); Ok(()) }
}

#[derive(Default)]
struct Acct { fail_first: usize, calls: AtomicUsize, sent: Mutex<Vec<(String, String)>> }
impl FeedAccount for Acct {
    fn put_source(&self, kind: &str, link: &str) -> Result<(), String> {
        if self.calls.fetch_add(1, Ordering::SeqCst) < self.fail_first { return Err("unreachable".into()); }
        self.sent.lock().unwrap().push((kind.into(), link.into()));
        Ok(())
    }
}
struct NoWindow;
impl SchoolWindows for NoWindow { fn open_school_window(&self) { panic!("a school window was opened"); } }

fn answers_with(dir: &Path, feeds: &[(&str, FeedState)]) {
    let mut a = Answers::default();
    for (k, s) in feeds { a.feeds.insert((*k).into(), *s); }
    save_answers(dir, &a).unwrap();
}
fn feed(dir: &Path, kind: &str) -> Option<FeedState> { load_answers(dir).unwrap().unwrap().feeds.get(kind).copied() }

#[test]
fn the_calendar_feed_is_stored_after_one_failure_from_its_own_pending_credential() {
    let dir = scratch("cal");
    answers_with(&dir, &[("calendar_ics", FeedState::Validated)]);
    let pending = Pending::default();
    pending.put(PENDING_CALENDAR, CAL_LINK).unwrap();
    let acct = Acct { fail_first: 1, ..Default::default() };
    let out = run_feed_store(&dir, "calendar_ics", &pending, &acct, &NoWindow);
    assert!(matches!(out, Outcome::Done(_)), "{out:?}");
    assert_eq!(acct.calls.load(Ordering::SeqCst), 2);
    assert_eq!(*acct.sent.lock().unwrap(), [("calendar_ics".to_string(), CAL_LINK.to_string())]);
    assert_eq!(feed(&dir, "calendar_ics"), Some(FeedState::Stored));
    assert!(pending.get(PENDING_CALENDAR).is_none());
}

#[test]
fn a_failed_store_is_tried_again_and_a_stored_or_absent_feed_sends_nothing() {
    let dir = scratch("again");
    answers_with(&dir, &[("lms_ics", FeedState::Failed), ("calendar_ics", FeedState::Stored)]);
    let pending = Pending::default();
    pending.put(PENDING_LMS, LMS_LINK).unwrap();
    pending.put(PENDING_CALENDAR, CAL_LINK).unwrap();
    let acct = Acct::default();
    assert!(matches!(run_feed_store(&dir, "lms_ics", &pending, &acct, &NoWindow), Outcome::Done(_)), "Try again on a failed store");
    assert_eq!(feed(&dir, "lms_ics"), Some(FeedState::Stored));
    assert!(matches!(run_feed_store(&dir, "calendar_ics", &pending, &acct, &NoWindow), Outcome::Done(_)));
    assert_eq!(acct.sent.lock().unwrap().len(), 1, "a stored feed is never sent twice");
    assert!(pending.get(PENDING_CALENDAR).is_none(), "its leftover credential goes");
    let none = scratch("none");
    answers_with(&none, &[]);
    assert!(matches!(run_feed_store(&none, "lms_ics", &pending, &acct, &NoWindow), Outcome::Done(_)), "no feed, nothing to store");
    assert_eq!(acct.calls.load(Ordering::SeqCst), 1);
}

#[test]
fn a_validated_feed_whose_pending_credential_is_gone_is_failed_by_name_and_never_sent() {
    let dir = scratch("gone");
    answers_with(&dir, &[("lms_ics", FeedState::Validated)]);
    let acct = Acct::default();
    let out = run_feed_store(&dir, "lms_ics", &Pending::default(), &acct, &NoWindow);
    assert_eq!(out, Outcome::Failed(REASON_LMS_LINK_MISSING.into()));
    assert_eq!(acct.calls.load(Ordering::SeqCst), 0);
    assert_eq!(feed(&dir, "lms_ics"), Some(FeedState::Failed), "settled, so config.ingest is not held for ever");
    let out = run_feed_store(&dir, "lms_ics", &Pending::default(), &acct, &NoWindow);
    assert_eq!(out, Outcome::Failed(REASON_LMS_LINK_MISSING.into()), "a retry says the same");
}

#[test]
fn a_failed_store_names_a_closed_key_and_never_the_link_or_the_services_words() {
    let dir = scratch("key");
    answers_with(&dir, &[("lms_ics", FeedState::Validated)]);
    let pending = Pending::default();
    pending.put(PENDING_LMS, LMS_LINK).unwrap();
    let Outcome::Failed(why) = run_feed_store(&dir, "lms_ics", &pending, &Acct { fail_first: 9, ..Default::default() }, &NoWindow) else { panic!() };
    assert_eq!(why, REASON_FEED_KEPT_ON_DEVICE);
}

#[test]
fn back_to_another_school_clears_the_school_steps_jobs_and_leaves_the_rest() {
    let dir = scratch("school");
    answers_with(&dir, &[("lms_ics", FeedState::Stored)]);
    let mut state = OnboardingState::new("2026-10-24T09:00:00Z", 3);
    for j in ["school.capture", "school.preview", "mail.setup"] { state.jobs.insert(j.into(), JobRecord { state: JobState::Done, ..Default::default() }); }
    change_school(&dir, &Pending::default(), &mut state).unwrap();
    assert_eq!(state.jobs.keys().collect::<Vec<_>>(), ["mail.setup"]);
    assert!(load_answers(&dir).unwrap().unwrap().feeds.is_empty());
    // No answers file yet is not an error, and none is made.
    let bare = scratch("school-bare");
    change_school(&bare, &Pending::default(), &mut state).unwrap();
    assert!(load_answers(&bare).unwrap().is_none());
}

#[test]
fn the_login_flags_come_from_step_8s_rows() {
    let mut a = Answers::default();
    assert_eq!((ingest_base_for(&a, "p", "UTC").zybooks, ingest_base_for(&a, "p", "UTC").vhl), (false, false));
    a.rows = vec![BookRow { source: "vhl".into(), key: "4471".into(), course_slug: "knotwork-theory".into() }];
    let b = ingest_base_for(&a, "profile-8c", "America/Chicago");
    assert_eq!((b.zybooks, b.vhl, b.profile_id.as_str(), b.timezone.as_str()), (false, true, "profile-8c", "America/Chicago"));
    a.rows.push(BookRow { source: "zybooks".into(), key: "ZYB-1".into(), course_slug: "quilting-basics".into() });
    assert!(ingest_base_for(&a, "p", "UTC").zybooks);
}

fn ingest_rig(tag: &str, feeds: &[(&str, FeedState)]) -> (PathBuf, PathBuf, Pending) {
    let root = scratch(tag);
    let (vault, data) = (root.join("vault"), root.join("data"));
    std::fs::create_dir_all(vault.join("config")).unwrap();
    std::fs::create_dir_all(&data).unwrap();
    let mut a = Answers::default();
    a.courses = vec![CourseAnswer { code: "QLT 101".into(), name: "Quilting Basics".into(), captured_label: "Quilting".into(), label: "Quilting".into(), lms_key: "_1001_1".into(), removed: false, slug: String::new(), fragments: Vec::new() }];
    a.slugs.insert("_1001_1".into(), "quilting-basics".into());
    for (k, s) in feeds { a.feeds.insert((*k).into(), *s); }
    save_answers(&data, &a).unwrap();
    let pending = Pending::default();
    pending.put(PENDING_LMS, LMS_LINK).unwrap();
    pending.put(PENDING_CALENDAR, CAL_LINK).unwrap();
    (vault, data, pending)
}

#[test]
fn config_ingest_in_the_lane_writes_the_file_and_then_deletes_both_pending_links() {
    let (vault, data, pending) = ingest_rig("settle", &[("lms_ics", FeedState::Stored)]);
    let out = run_config_ingest_in(&vault, &data, "profile-8c", "America/Chicago", &pending);
    assert!(matches!(out, Outcome::Done(_)), "{out:?}");
    assert!(vault.join("config").join("ingest.yaml").exists());
    assert!(pending.get(PENDING_LMS).is_none() && pending.get(PENDING_CALENDAR).is_none());
}

#[test]
fn a_feed_still_being_validated_holds_config_ingest_and_touches_nothing() {
    let (vault, data, pending) = ingest_rig("hold", &[("lms_ics", FeedState::Validated)]);
    let out = run_config_ingest_in(&vault, &data, "profile-8c", "UTC", &pending);
    assert_eq!(out, Outcome::Waiting(REASON_FEEDS_NOT_SETTLED.into()));
    assert!(!vault.join("config").join("ingest.yaml").exists());
    assert!(pending.get(PENDING_LMS).is_some(), "the link is still where it waits");
}

#[test]
fn a_link_missing_failure_leaves_the_other_credential_alone_and_names_the_re_paste() {
    let (vault, data, pending) = ingest_rig("missing", &[("lms_ics", FeedState::Failed)]);
    pending.delete(PENDING_LMS).unwrap();
    let out = run_config_ingest_in(&vault, &data, "profile-8c", "UTC", &pending);
    assert_eq!(out, Outcome::Failed(REASON_LMS_LINK_MISSING.into()));
    assert!(!vault.join("config").join("ingest.yaml").exists());
    assert!(pending.get(PENDING_CALENDAR).is_some(), "nothing was written, so nothing is settled");
}

#[test]
fn an_existing_ingest_file_with_other_bytes_keeps_a_failed_feeds_link() {
    let (vault, data, pending) = ingest_rig("kept", &[("lms_ics", FeedState::Failed)]);
    std::fs::create_dir_all(vault.join("config")).unwrap();
    std::fs::write(vault.join("config").join("ingest.yaml"), "# a restored file\n").unwrap();
    let out = run_config_ingest_in(&vault, &data, "profile-8c", "UTC", &pending);
    assert!(matches!(&out, Outcome::Done(p) if p.contains_key(KEPT_EXISTING_KEY)), "{out:?}");
    assert_eq!(pending.get(PENDING_LMS).as_deref(), Some(LMS_LINK), "the file did not take the link, so it is not thrown away");
}

#[test]
fn registrar_confirm_takes_the_rows_as_saved() {
    let mut a = Answers::default();
    a.registrar_rows = vec![RegistrarRow { code: "QLT 101".into(), days: "MWF".into(), start: "09:00".into(), end: "09:50".into() }];
    assert_eq!(registrar_confirm_rows(&a), a.registrar_rows);
    assert!(registrar_confirm_rows(&Answers::default()).is_empty());
}
