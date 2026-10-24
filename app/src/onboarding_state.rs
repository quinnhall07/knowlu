//! `onboarding.json`, `onboarding-answers.json`, resume, skips, the three exits and the account
//! commands of step 2 again and step 4 (plan §4.5, §4.7, §4.10; F2a-F2h).
//!
//! **F1 stub.** The types and signatures are the contract `app/tests/onboarding_state.rs` and
//! `app/tests/onboarding_jobs.rs` write against. Every function panics (`todo!`) until the F2 tasks
//! replace it, except the two path helpers, which are the plan's own words.
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

pub const STATE_FILE: &str = "onboarding.json";
pub const ANSWERS_FILE: &str = "onboarding-answers.json";
/// The two pending-credential names under `knowlu/<profile_id>/` (§4.11).
pub const PENDING_LMS: &str = "pending-lms_ics";
pub const PENDING_CALENDAR: &str = "pending-calendar_ics";

/// Beside `settings.json`, in the profile's app data, never in the vault.
pub fn state_path(data_dir: &Path) -> PathBuf { data_dir.join(STATE_FILE) }
pub fn answers_path(data_dir: &Path) -> PathBuf { data_dir.join(ANSWERS_FILE) }

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Default, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum JobState {
    #[default]
    Pending,
    Running,
    Done,
    Failed,
    Skipped,
    Held,
}

#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct JobRecord {
    pub state: JobState,
    /// Counts only.
    pub proof: BTreeMap<String, i64>,
    /// A closed-set reason (and for `failed`, its fix is the page's).
    pub reason: Option<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SchoolPath { Shared, Separate, Paste }

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct School {
    /// `None` for a school typed as "isn't listed": a name and no unitid (R-OB-4).
    pub unitid: Option<String>,
    pub label: String,
    pub path: SchoolPath,
    /// True once the student has confirmed this school in this run. Never trusted across a quit.
    pub confirmed: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Skip {
    pub step: u8,
    pub source: String,
    pub cost_key: String,
}

/// One required item not yet `done` at Open Knowlu, with the signal that clears it (§4.10):
/// `slot_end` or `mail_json`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct HandoverItem {
    pub item: String,
    pub signal: String,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct OnboardingState {
    pub version: u32,
    pub step: u8,
    pub finished: bool,
    pub school: Option<School>,
    pub skips: Vec<Skip>,
    pub jobs: BTreeMap<String, JobRecord>,
    pub account_settled: bool,
    pub step10_confirmed_at: Option<String>,
    pub started_at: String,
    pub handover: Vec<HandoverItem>,
}

impl OnboardingState {
    pub fn new(started_at: &str, step: u8) -> OnboardingState {
        OnboardingState {
            version: 1,
            step,
            finished: false,
            school: None,
            skips: Vec::new(),
            jobs: BTreeMap::new(),
            account_settled: false,
            step10_confirmed_at: None,
            started_at: started_at.to_string(),
            handover: Vec::new(),
        }
    }
    /// The first slot after the reveal ended (or the first onboarding slot to end, before it):
    /// clears the `slot_end` handover items and no other (§4.10).
    pub fn slot_ended(&mut self) { todo!("F8f: clear the slot items") }
    /// A mail signal read from `mail.json`: clears only its own item (§4.10).
    pub fn mail_signal(&mut self, _signal: MailSignal) { todo!("F8f: clear the mail items") }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MailSignal { CanaryVerified, FirstPassDelivered, Disconnected }

/// What reading `onboarding.json` found. An unreadable file is never read as finished or absent.
#[derive(Debug, Clone, PartialEq)]
pub enum Loaded {
    Absent,
    Ok(OnboardingState),
    Unreadable(String),
}

/// Temp file and rename, in the profile's app data.
pub fn save_state(_data_dir: &Path, _s: &OnboardingState) -> Result<(), String> { todo!("F2a: save") }
pub fn load_state(_data_dir: &Path) -> Loaded { todo!("F2a: load") }

// ---- the answers file (§4.5) -------------------------------------------------------------------

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CourseAnswer {
    pub code: String,
    pub name: String,
    pub label: String,
    /// The LMS's own course key.
    pub lms_key: String,
    /// Step 3's Remove (R-C1-51): set through `onboarding_answer`, cleared by a re-capture. A
    /// removed row is read by nobody; [`Answers::kept_courses`] is the only way to the list.
    #[serde(default)]
    pub removed: bool,
    /// The course's slug where the LMS gives one; only for [`CourseAnswer::key`] (not the note's slug,
    /// which `school.courses` records in [`Answers::slugs`]).
    #[serde(default)]
    pub slug: String,
    /// The course's pairs as F8b saves them for `course_map` and `course_exclude` (§14.45): its code
    /// fragments and its gradebook uid pins, built as F6b writes `course_map`'s. Empty until
    /// `school.preview` has run; the hand-over then falls back to the course's code.
    #[serde(default)]
    pub fragments: Vec<String>,
}

impl CourseAnswer {
    /// The key `by_course` uses, and so Remove's and a re-capture's identity for a row (§14.45):
    /// the LMS course `code`, then the slug, then the label, each only when the one before is empty.
    pub fn key(&self) -> String { todo!("F2d: the course key") }
}

/// The name of a match-set bucket as the engine's `by_course` writes it (F7a, 904a92c): the sorted
/// course keys joined by `,`; [`UNMATCHED`] for an item no course claims. The app never parses the
/// name: it reads [`Bucket::codes`].
pub fn bucket_key(codes: &[&str]) -> String {
    let mut c: Vec<&str> = codes.to_vec();
    c.sort_unstable();
    if c.is_empty() { UNMATCHED.to_string() } else { c.join(",") }
}
pub const UNMATCHED: &str = "unmatched";

/// One `by_course` bucket: the course keys that claimed its items (empty for [`UNMATCHED`]) and the
/// counts by name (`class_times`, `assignments_upcoming`, `turned_in`, `grades_courses`). A
/// single-course bucket always exists, with zero counts when nothing landed in it.
#[derive(Debug, Clone, PartialEq, Eq, Default, Serialize, Deserialize)]
pub struct Bucket {
    pub codes: Vec<String>,
    #[serde(flatten)]
    pub counts: BTreeMap<String, i64>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RegistrarRow {
    pub code: String,
    pub days: String,
    pub start: String,
    pub end: String,
}

/// A feed's state: never the link (§4.11).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum FeedState { Validated, Stored, Rejected, Failed }

/// A confirmed homework-site row: the vendor's key and the course slug it belongs to.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct BookRow {
    pub source: String,
    pub key: String,
    pub course_slug: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Default, Serialize, Deserialize)]
pub struct Answers {
    pub courses: Vec<CourseAnswer>,
    pub registrar_rows: Vec<RegistrarRow>,
    /// Course code to the slug `school.courses` settled on (a restored note's own where one matched).
    pub slugs: BTreeMap<String, String>,
    pub google_calendar: Option<bool>,
    /// `lms_ics` or `calendar_ics` to its state.
    pub feeds: BTreeMap<String, FeedState>,
    pub rows: Vec<BookRow>,
    pub zybooks_ignore: Vec<String>,
    /// `school.preview`'s `by_course` match-set buckets (§14.45), by [`bucket_key`], each a [`Bucket`]: its `codes` and its
    /// counts by name.
    #[serde(default)]
    pub buckets: BTreeMap<String, Bucket>,
}

impl Answers {
    /// The courses that were not removed, in captured order. `school.courses`, `config.ingest`'s
    /// `course_map` and the step-3 totals read the list only through this.
    pub fn kept_courses(&self) -> Vec<&CourseAnswer> { todo!("F2d: kept_courses") }
    /// A re-capture: replaces the list with `captured`, every row un-removed. A row whose
    /// [`CourseAnswer::key`] matches an old one keeps that row's `fragments` until the next preview.
    pub fn recapture(&mut self, _captured: Vec<CourseAnswer>) { todo!("F2d: recapture") }
}

/// Step 3's proof totals (§14.45): `classes` is the number of kept rows; every other key is the sum,
/// over **every bucket whose `codes` is empty ([`UNMATCHED`]) or contains at least one kept course's
/// key**, of that count. A bucket naming only removed courses is out; one naming a kept and a removed
/// course stays. This is exactly what F6c's ingest imports (`onboarding_status` returns it).
pub fn step3_totals(_a: &Answers) -> BTreeMap<String, i64> { todo!("F2d: the step-3 totals") }

/// The answer-recording inner of `onboarding_answer(step, answer)` (F2b): saves the answer to the
/// answers file **first**, then calls `enqueue` once per job the answer asks for. Shapes and jobs:
/// - 3 `{"capture": {"courses": [CourseAnswer], "registrar_rows": [..] (default empty), "feeds":
///   {kind: state} (default empty)}}` enqueues `school.preview`;
/// - 3 `{"remove": <CourseAnswer::key>}`, `{"buckets": {<bucket_key>: {"codes": [..], name: n, ..}}}` and
///   `{"pairs": {<key>: [fragment, ..]}}` and `{"label": {"key": <key>, "label": <text>}}` (display
///   only: it changes no pairs and no bucket) enqueue nothing;
/// - 6 `{"google_calendar": bool}` enqueues `google.calendar_read` for `true`, nothing for `false`;
/// - 8 `{"row": BookRow}` or `{"zybooks_ignore": <key>}` enqueue nothing (`config.ingest` is asked
///   for on leaving the step).
/// Answers `{ok, error?}`; an unknown shape is `ok: false` and enqueues nothing. Nothing is held in
/// memory, so a quit between two calls loses nothing.
pub fn onboarding_answer_inner(
    _data_dir: &Path,
    _step: u8,
    _answer: &Value,
    _enqueue: &mut dyn FnMut(&str),
) -> Value {
    todo!("F2b: onboarding_answer")
}

/// `Ok(None)` is an absent file; an unreadable one is `Err`, never "no answers".
pub fn load_answers(_data_dir: &Path) -> Result<Option<Answers>, String> { todo!("F2d: load answers") }
pub fn save_answers(_data_dir: &Path, _a: &Answers) -> Result<(), String> { todo!("F2d: save answers") }
/// Load (or start empty), change, save: how each answer is written the moment it is given.
pub fn update_answers(_data_dir: &Path, _change: &dyn Fn(&mut Answers)) -> Result<(), String> {
    todo!("F2d: update answers")
}

// ---- the pending credentials (§4.11) -------------------------------------------------------------

/// The pending feed links. The real store is Credential Manager under `knowlu/<profile_id>/<name>`
/// (`credentials::write`, `exists`, `delete`); tests pass a fake.
pub trait PendingStore {
    fn put(&self, name: &str, link: &str) -> Result<(), String>;
    fn get(&self, name: &str) -> Option<String>;
    /// Deleting what is not there is not an error.
    fn delete(&self, name: &str);
}

pub struct CredPending { pub profile_id: String }

impl PendingStore for CredPending {
    fn put(&self, _name: &str, _link: &str) -> Result<(), String> { todo!("F2d: write the pending credential") }
    fn get(&self, _name: &str) -> Option<String> { todo!("F2d: read the pending credential") }
    fn delete(&self, _name: &str) { todo!("F2d: delete the pending credential") }
}

/// `lms_ics` to [`PENDING_LMS`], `calendar_ics` to [`PENDING_CALENDAR`].
pub fn pending_name(_kind: &str) -> Option<&'static str> { todo!("F2d: the pending name for a feed kind") }

// ---- resume and LaunchMode (§4.5, §4.7) ----------------------------------------------------------

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LaunchMode { Normal, Onboarding }

/// What §4.7's checks prove, each a fake in the tests. Nothing here reads a school: the account
/// cannot name one (T0, 2026-10-24), and [`ResumeChecks::account_school`] exists so a test can
/// count that it is never asked.
pub trait ResumeChecks {
    fn session_present(&self) -> bool;
    fn restored_course_count(&self) -> usize;
    fn entitled(&self) -> bool;
    fn school_mailbox_active(&self) -> bool;
    fn google_granted(&self) -> bool;
    fn homework_login_stored(&self) -> bool;
    fn class_commitments_cover_courses(&self) -> bool;
    fn account_school(&self) -> Option<School>;
    /// The answers file is present and readable. Past step 3 an absent or unreadable one resumes
    /// at step 3: it means the answers were deleted or never saved, never that there were none.
    fn answers_ok(&self) -> bool;
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SchoolOffer {
    /// Step 3 asks for the school, as on a fresh install.
    Ask,
    /// The device's own earlier choice, offered as a confirm. Nothing is assumed.
    Confirm(School),
}

#[derive(Debug, Clone, PartialEq)]
pub struct Resume {
    pub mode: LaunchMode,
    /// What onboarding runs with: `running` jobs back to `pending`; for an unreadable file, a fresh
    /// state at the derived step. `None` for a normal launch.
    pub state: Option<OnboardingState>,
    /// Where the page opens: step 2 whenever the profile holds no session.
    pub step: u8,
    pub school: SchoolOffer,
}

pub fn launch_mode(_loaded: &Loaded) -> LaunchMode { todo!("F2a: LaunchMode") }
pub fn resume(_loaded: Loaded, _checks: &dyn ResumeChecks) -> Resume { todo!("F2a: resume") }
/// Step 3's school: only ever from this device's file, never from the account.
pub fn school_offer(_state: Option<&OnboardingState>, _checks: &dyn ResumeChecks) -> SchoolOffer {
    todo!("F2a: the school offer")
}
/// `school.apply` does not run until the school is confirmed.
pub fn school_apply_may_run(_state: &OnboardingState) -> bool { todo!("F2a: the apply hold") }
pub fn confirm_school(_state: &mut OnboardingState) { todo!("F2a: confirm the school") }

/// Account, payment, the school step and "Your week" refuse a skip; every other step takes one.
pub fn skip_step(_state: &mut OnboardingState, _step: u8, _source: &str) -> Result<(), String> {
    todo!("F2a: skips")
}

/// F2e: deletes the answers file and both pending credentials, then rewinds an unfinished
/// `onboarding.json` to step 2 (jobs pending, `account_settled` and `school` cleared, skips kept).
/// A finished file is left alone. `account::sign_out_clean` calls the real-store wrapper.
pub fn on_sign_out_with(_data_dir: &Path, _pending: &dyn PendingStore) -> Result<(), String> {
    todo!("F2e: the sign-out rewind")
}
pub fn on_sign_out(_data_dir: &Path, _profile_id: &str) -> Result<(), String> { todo!("F2e: the real-store wrapper") }

// ---- the three exits (§4.10, F2b) ----------------------------------------------------------------

/// Everything an exit touches outside the queue's own state, so F1 drives each exit with fakes.
/// `request_stop` sets the stop; `drain` waits for the safe points of the queue and the lane.
pub trait ExitSeams {
    fn request_stop(&self);
    fn drain(&self);
    fn save(&self) -> Result<(), String>;
    /// After this, the queue's saver refuses every save (F8f). F2a's file save has no such flag.
    fn freeze_saves(&self);
    /// `account::sign_out`, whose clean runs `on_sign_out`.
    fn sign_out(&self) -> Result<(), String>;
    /// `onboarding::relaunch(args)`.
    fn relaunch(&self, args: &[&str]) -> Result<(), String>;
    /// `onboarding::relaunch_with(vault)`.
    fn relaunch_with(&self) -> Result<(), String>;
    /// `settings.quit_at`, as the tray's Quit and `switch_profile` stamp it.
    fn stamp_quit_at(&self);
    /// `state::quit_flush`, capped at 10 s.
    fn flush(&self);
    fn exit(&self);
    /// The queue's state as of now, for finish's refusal and its hand-over.
    fn snapshot(&self) -> OnboardingState;
    /// Writes `finished: true` and `handover`, deletes the answers file, applies autostart.
    fn write_finish(&self, handover: Vec<HandoverItem>) -> Result<(), String>;
}

/// One exit at a time: a second exit while one drains answers `{ok: true, exiting: true}`.
#[derive(Default)]
pub struct ExitGuard(pub std::sync::atomic::AtomicBool);

/// Finish later and the window's close: stop, drain, save, stamp `quit_at`, flush, exit.
pub fn onboarding_quit_inner(_s: &dyn ExitSeams, _g: &ExitGuard) -> Value { todo!("F2b: onboarding_quit") }
/// Stop, drain, save, freeze, sign out, relaunch `--pick`, exit; no flush.
pub fn onboarding_sign_out_inner(_s: &dyn ExitSeams, _g: &ExitGuard) -> Value { todo!("F2b: onboarding_sign_out") }
/// Refuse while the gate is closed (`now_secs` against the cap) or a lane job is not `done`; then
/// stop, drain, save, `write_finish`, `relaunch_with`, and today's quit path.
pub fn onboarding_finish_inner(_s: &dyn ExitSeams, _g: &ExitGuard, _now_secs: i64) -> Value {
    todo!("F2b: onboarding_finish")
}
/// F4's close handler: spawns [`onboarding_quit_inner`] on its own thread and returns at once.
pub fn close_requested(_s: std::sync::Arc<dyn ExitSeams + Send + Sync>, _g: std::sync::Arc<ExitGuard>) {
    todo!("F2b: the close never waits on the event loop")
}
/// Every required item of §4.6 not `done`, each with its clearing signal.
pub fn handover_for(_state: &OnboardingState) -> Vec<HandoverItem> { todo!("F8f: the hand-over list") }

/// `onboarding_status`'s body: the file plus the live job states, the gate, and whether step 10's
/// proposals may be asked (only once `config.ingest` is `done`). Keys: `jobs`, `proposals_ready`,
/// `gate`.
pub fn status_inner(_state: &OnboardingState, _now_secs: i64) -> Value { todo!("F2b: onboarding_status") }

// ---- step 2 again, and step 4 (F2g, F2h) ---------------------------------------------------------

/// A local delete of the pending session and nothing else: never a revoke (review r6, I-1).
/// `credentials::delete`'s not-found result is ignored.
pub fn forget_pending_local(_target: &str) { todo!("F2g: the local forget") }

/// Moves the pending session onto the vault's own target only when the pending session, the
/// sign-in's answer and `config/cloud.yaml` name one account. Otherwise `forget_pending` runs and the
/// answer is the named refusal (`wrong_account: true` where it applies). `after_attach` is the
/// best-effort entitlement refresh, a seam so a test makes no request.
pub fn attach_session_in(
    _vault: &Path,
    _pending_target: &str,
    _expected_account_id: &str,
    _forget_pending: &dyn Fn(),
    _after_attach: &dyn Fn(),
) -> Value {
    todo!("F2g: attach the session")
}

/// The command: passes `account::PENDING_TARGET` and the real seams.
#[tauri::command(async)]
pub fn onboarding_attach_session(_cs: tauri::State<'_, crate::state::ConsoleState>, _account_id: String) -> Value {
    todo!("F2g: onboarding_attach_session")
}

/// `account::refresh_entitlement` on the vault's own session, answered as `entitlement_now`'s
/// envelope (`ok`, `error`, `status`, `plan`, `current_period_end`).
pub fn entitlement_in(_vault: &Path, _data_dir: &Path) -> Value { todo!("F2h: onboarding_entitlement") }

/// The consent-then-checkout core on the vault's own session; returns the URL and opens nothing.
pub fn checkout_url_in(_vault: &Path, _plan: &str) -> Result<String, String> { todo!("F2h: onboarding_checkout") }

/// The `--courses` list `school.preview` passes to `knowlu-engine onboarding --preview` (§14.46): one
/// object per captured course (removed rows included, so their items are attributed and bucketed),
/// carrying `code`, `name`, `label`, `lms_key` and **`pairs`**, the row's saved `fragments`, passed
/// through byte for byte. The label is display only: editing it changes no pair.
pub fn preview_courses(_a: &Answers) -> Value { todo!("F8b: the --courses list") }
