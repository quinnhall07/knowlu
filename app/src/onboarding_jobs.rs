//! The job queue (plan §4.3, F8a-F8h) and the runners that are pure enough to test: the feed store,
//! `school.courses`, the `config.ingest` hand-over, the link commands that only validate.
//!
//! **F1 stub.** Types and signatures are the contract `app/tests/onboarding_jobs.rs` writes
//! against; every function panics (`todo!`) until F8 replaces it.
use crate::onboarding_state::{Answers, CourseAnswer, HandoverItem, OnboardingState, PendingStore};
use crate::scheduler::SlotScope;
use serde::Serialize;
use serde_json::Value;
use std::collections::BTreeMap;
use std::path::Path;
use std::sync::atomic::AtomicBool;
use std::sync::{Arc, Mutex};
use std::time::Duration;

/// The two threads: the serial queue (every job that runs an engine child or a window) and the
/// local lane (`school.store`, `calendar.store`, `school.apply`, `school.courses`,
/// `registrar.confirm`, `config.ingest`). Their names are how a fake tells where it was called from.
pub const QUEUE_THREAD: &str = "onboarding-queue";
pub const LANE_THREAD: &str = "onboarding-lane";

/// What a runner hands back. `Stopped` is a safe-point stop: the job goes back to `pending`.
#[derive(Debug, Clone, PartialEq)]
pub enum Outcome {
    /// The job's proof counts, `None` where its input was missing; copied into [`crate::onboarding_state::JobRecord::proof`].
    Done(BTreeMap<String, Option<i64>>),
    Failed(String),
    /// Not able to run yet; stays `pending` with this reason.
    Waiting(String),
    Stopped,
    /// A slot's end: the queue reads the status record, not the steps (§4.3).
    Slot(SlotRun),
}

#[derive(Debug, Clone, PartialEq)]
pub struct SlotRun {
    /// `RunSummary.started`, `journal::now_ts`.
    pub started: String,
    pub steps: Vec<(String, i32)>,
    /// A refusal (script mode, another device): the job is `failed` with it, never `done`.
    pub reason: Option<String>,
}

/// One runner for every job; the queue dispatches by name. `scope` is `Some` for the two slot jobs.
pub trait Runner: Send + Sync {
    fn run(&self, job: &str, scope: Option<SlotScope>, stop: &AtomicBool) -> Outcome;
}

/// `scheduler::entitlement_state(cs) == Entitled`, refreshed through `account::refresh_entitlement`.
pub trait Entitlement: Send + Sync {
    fn entitled(&self) -> bool;
    fn refresh(&self);
}

/// `state/sync-status.json` as the slot left it (§4.3's three readings).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SyncStatus {
    pub at: String,
    pub ok: bool,
    pub skipped: Option<String>,
    pub last_error: Option<String>,
}

/// `state/sync-cursor.json`, read and never written.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SyncCursor {
    pub pulled_to_end: bool,
    pub record_cursor: i64,
    pub note_cursor: i64,
}

pub trait SyncView: Send + Sync {
    /// Read once after each `BeforeConfig`/`NoRank` slot ends; `slot_started` is its
    /// `RunSummary.started`.
    fn read(&self, slot_started: &str) -> (Option<SyncStatus>, Option<SyncCursor>);
}

pub struct Deps {
    /// The profile's app data (where `onboarding.json` lives).
    pub data_dir: std::path::PathBuf,
    pub runner: Arc<dyn Runner>,
    pub entitlement: Arc<dyn Entitlement>,
    pub sync_view: Arc<dyn SyncView>,
}

/// The queue's `BeforeConfig` chain stops at this many runs (Q-B20).
pub const CHAIN_BACKSTOP: u32 = 50;

pub struct Queue { _inner: Arc<Mutex<()>> }

impl Queue {
    /// Spawns the serial thread and the lane thread, over `state` (running jobs already `pending`).
    pub fn start(_deps: Deps, _state: OnboardingState) -> Queue { todo!("F8a: start the queue") }
    /// An ask was answered: the job is `pending` (or `held`) and runs when its inputs are ready.
    pub fn enqueue(&self, _job: &str) { todo!("F8a: enqueue") }
    /// The entitlement changed (F2h's poll): re-test the hold.
    pub fn wake(&self) { todo!("F8a: wake") }
    /// "Try again" on a job (resets the chain count for `school.courses` while unsettled).
    pub fn retry(&self, _job: &str) { todo!("F8e: retry") }
    /// Step 10's confirm: stamps `step10_confirmed_at` and enqueues `slot.first_full`.
    pub fn confirm_step10(&self, _at: &str) { todo!("F8e: confirm step 10") }
    /// The student confirmed the school offered at step 3: marks it confirmed on the live state and
    /// wakes the queue, so a `school.apply` that was waiting on it can start (§4.7).
    pub fn confirm_school(&self) { todo!("F8e: confirm the school") }
    pub fn snapshot(&self) -> OnboardingState { todo!("F8a: snapshot") }
    /// True when nothing is running and nothing runnable is waiting, within `timeout`.
    pub fn wait_idle(&self, _timeout: Duration) -> bool { todo!("F8a: wait idle") }
    pub fn request_stop(&self) { todo!("F8f: stop") }
    pub fn stop_requested(&self) -> bool { todo!("F8f: stop requested") }
    /// Waits for each lane's running job to reach its safe point.
    pub fn drain(&self) { todo!("F8f: drain") }
    /// Persists now; refused after [`Queue::freeze`].
    pub fn save(&self) -> Result<(), String> { todo!("F8f: save") }
    pub fn freeze(&self) { todo!("F8f: freeze") }
    pub fn is_frozen(&self) -> bool { todo!("F8f: is frozen") }
    /// Finish's write, through the queue's own state: sets `finished` and `handover` on the live
    /// state and saves it, so no later save can write an older, unfinished copy over it.
    pub fn finish(&self, _handover: Vec<HandoverItem>) -> Result<(), String> { todo!("F8f: finish") }
    /// F8m: a closed backfill batch tells the queue; nothing is started.
    pub fn mail_batch_closed(&self) { todo!("F8m: mail batch closed") }
    pub fn batch_closed_noted(&self) -> bool { todo!("F8m: batch closed noted") }
}

// ---- the notes gate and the sync-lock retry (F8h) -------------------------------------------------

pub const RETRY_INTERVAL: Duration = Duration::from_millis(250);
pub const RETRY_BUDGET: Duration = Duration::from_secs(30);

/// Retries `try_once` while it refuses with `week::SYNC_RUNNING` (and only that), every
/// [`RETRY_INTERVAL`] up to [`RETRY_BUDGET`]; holds nothing itself. For `registrar.confirm`, whose
/// call takes `vault_io` and the lock itself.
pub fn retry_on_sync_running<T>(
    _sleep: &dyn Fn(Duration),
    _try_once: &mut dyn FnMut() -> Result<T, String>,
) -> Result<T, String> {
    todo!("F8h: retry on a running sync")
}

/// The same retry for a lane job that takes the two holds itself around an engine child: each try
/// takes `vault_io` then `take_lock`, and on `SYNC_RUNNING` drops both before it sleeps.
pub fn retry_with_hold<T>(
    _vault_io: &Mutex<()>,
    _take_lock: &dyn Fn() -> Result<Box<dyn std::any::Any>, String>,
    _sleep: &dyn Fn(Duration),
    _work: &mut dyn FnMut() -> Result<T, String>,
) -> Result<T, String> {
    todo!("F8h: retry holding neither between tries")
}

// ---- the feed store, the courses, the hand-over to `config.ingest` --------------------------------

/// `PUT /account/sources` (`lms_link::store_source` over the vault's own session, F8c).
pub trait FeedAccount {
    fn put_source(&self, kind: &str, link: &str) -> Result<(), String>;
}

/// A fake fails if this is ever called: a resumed store needs no school window (§4.11).
pub trait SchoolWindows {
    fn open_school_window(&self);
}

/// `school.store` / `calendar.store` for one feed `kind` (`lms_ics` or `calendar_ics`): sends a
/// **validated** link from its pending credential, retrying once. Stored: the state is `stored` and
/// the pending credential is deleted. Two failures: `failed` (kept on this computer), which settles
/// it for `config.ingest`. A `rejected` link is never sent.
pub fn run_feed_store(
    _data_dir: &Path,
    _kind: &str,
    _pending: &dyn PendingStore,
    _account: &dyn FeedAccount,
    _windows: &dyn SchoolWindows,
) -> Outcome {
    todo!("F8c: the feed store")
}

/// A course note that already exists (a C3' restore), matched by path, `source_uid` or code.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ExistingNote {
    pub slug: String,
    pub source_uid: String,
    pub code: String,
}

/// `school.courses`: creates a note only for a course with none (`create` returns the new slug),
/// and records every kept course's slug **by [`CourseAnswer::key`]**, a restored note's own where one
/// matched, in the answers file ([`Answers::slugs`]).
/// **One note per course** (§14.15, I5): the kept rows are walked in captured order, and a kept row
/// whose trimmed, **non-empty** human `code` equals an earlier kept row's reuses that row's slug and
/// does no create (two shells of one course). An empty `code` never matches another row, so two
/// typed rows with no code are two courses, two creates, two slugs.
/// An absent or unreadable answers file is an error (`Outcome::Failed`), never "no courses".
pub fn run_school_courses(
    _data_dir: &Path,
    _existing: &[ExistingNote],
    _create: &mut dyn FnMut(&CourseAnswer) -> Result<String, String>,
) -> Outcome {
    todo!("F6a, F8c: school.courses")
}

/// What `config.ingest` hands F6b's builder, from the answers file alone. Serialised, it is the
/// text the byte-equality of test 12 compares.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct IngestHandoff {
    /// `''` for a stored feed; the link itself for a `failed` or `rejected` one.
    pub ics_url: String,
    pub feed_kept_on_device: bool,
    pub calendar_url: String,
    pub calendar_kept_on_device: bool,
    /// Fragment to slug, over the **kept** courses only: each kept course's saved `fragments` (its
    /// code when none are saved) paired with its slug (a restored note's where one matched).
    pub course_map: Vec<(String, String)>,
    /// The removed courses' fragments (§14.45) that **no kept course also contributes** (§14.46.1), in
    /// no `course_map` entry; empty when none was removed (and then F6b writes no `course_exclude` key).
    pub course_exclude: Vec<String>,
    pub google_calendar: bool,
    pub books: Vec<(String, String, String)>,
    pub zybooks_ignore: Vec<String>,
}

pub fn ingest_handoff(_answers: &Answers, _pending: &dyn PendingStore) -> IngestHandoff {
    todo!("F6b, F8c: the config.ingest hand-over")
}

/// `school.preview`'s core (F8b): loads the answers file, builds the `--courses` list with
/// [`crate::onboarding_state::preview_courses`] and hands it to `engine` (the
/// `knowlu-engine onboarding --preview` call, the feed, grades and registrar files beside it), then
/// saves the output's `by_course` through [`crate::onboarding_state::save_preview`]. An absent or
/// unreadable answers file is `Failed`, never a preview of no courses. An `engine` error, or a
/// `save_preview` error (an output whose `by_course` is `null`, absent or not an object), is `Failed`
/// too, and the saved buckets are left exactly as they were.
pub fn run_school_preview(_data_dir: &Path, _engine: &mut dyn FnMut(&Value) -> Result<Value, String>) -> Outcome {
    todo!("F8b: school.preview")
}

/// What F8c's `registrar.confirm` hands R24's write: **every** saved registrar row, in captured order,
/// a removed course's included (§14.47, G5: removal never touches class times). From the answers
/// file alone.
pub fn registrar_confirm_rows(_answers: &Answers) -> Vec<crate::onboarding_state::RegistrarRow> {
    todo!("F8c: the registrar rows to confirm")
}

/// After `config.ingest` has written the file: deletes both pending credentials.
pub fn settle_pending_after_ingest(_pending: &dyn PendingStore) { todo!("F8c: delete the pending links") }

/// Back to another school before payment: deletes the pending links, the feed states and the
/// school-step jobs' results, and clears `school` (§4.11).
pub fn change_school(_data_dir: &Path, _pending: &dyn PendingStore, _state: &mut OnboardingState) -> Result<(), String> {
    todo!("F8c: back to another school")
}

// ---- the link commands that only validate (F8lb) -----------------------------------------------------

/// `onboarding_paste_link`'s inner: validates through `lms_link::validate_only` (never `finish`),
/// writes the pending credential and the feed's state (`validated`, or `rejected` for a failed
/// paste, whose link is kept too), and sends nothing to any server. Answers `{ok, stored: false,
/// note, …}`. `fetch` is the ICS fetch.
pub fn paste_link_inner(
    _data_dir: &Path,
    _pending: &dyn PendingStore,
    _kind: &str,
    _url: &str,
    _fetch: &dyn Fn(&str) -> Result<String, String>,
) -> Value {
    todo!("F8lb: onboarding_paste_link")
}
