//! The job queue (plan §4.3, F8a-F8h) and the runners that are pure enough to test: the feed store,
//! `school.courses`, the `config.ingest` hand-over, the link commands that only validate.
//!
//! **F8a** is the queue's core (below). The runners, the notes gate and the link commands further
//! down are still F1 stubs: their types and signatures are the contract `app/tests/onboarding_jobs.rs`
//! writes against, and each panics (`todo!`, naming its task) until that task replaces it.
//! **While a [`Queue`] is live it is the only writer of `onboarding.json`**: the commands (F2) and the
//! window's exits (F4) change the file through the queue's methods, never `save_state` directly. The
//! one exception is the sign-out rewind, which writes after [`Queue::freeze`] (F8f, F2e).
use crate::onboarding_state::{save_state, Answers, CourseAnswer, HandoverItem, JobRecord, JobState, OnboardingState, PendingStore};
use crate::scheduler::SlotScope;
use serde::Serialize;
use serde_json::Value;
use std::collections::{BTreeMap, BTreeSet};
use std::path::Path;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Condvar, Mutex, MutexGuard};
use std::thread::JoinHandle;
use std::time::{Duration, Instant};

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

// ---- the core (F8a): states, persistence, the two serial threads, the entitlement hold ----------------

const LANE: [&str; 6] = ["school.store", "calendar.store", "school.apply", "school.courses", "registrar.confirm", "config.ingest"];
/// Device-side before payment; every other job is held until the vault is `Entitled` (§4.3).
const UNGATED: [&str; 2] = ["school.capture", "school.preview"];
/// The order a resumed queue runs its jobs in: step order, then ask order within a step (§4.2).
/// A name not listed sorts after these, by name.
pub const JOB_ORDER: [&str; 17] = [
    "school.capture", "school.preview", "school.store", "school.apply", "school.courses", "registrar.confirm",
    "mail.setup", "mail.canary", "mail.first_pass", "google.gmail_first_read", "calendar.store", "calendar.read", "google.calendar_read",
    "coursework.discover", "config.ingest", "slot.partial", "slot.first_full",
];
/// Jobs a drain never waits for: waiting for mail writes nothing, so they start again from the top (§4.10).
const ABANDON_AT_ONCE: [&str; 2] = ["mail.canary", "mail.first_pass"];
/// The closed-set reason of a job whose runner panicked; the page maps it to its fix. It is reachable
/// only under unwinding (tests and the dev profile): the release profile has `panic = "abort"`.
pub const REASON_JOB_CRASHED: &str = "job_crashed";

struct Core {
    state: OnboardingState,
    /// Jobs in the order their asks were answered, until they finish; a running job stays in it.
    order: Vec<String>,
    /// False until the start thread has read the entitlement once; gated jobs wait for it.
    ready: bool,
    entitled: bool,
    busy: [bool; 2],
    /// `Waiting` and `Stopped` jobs: not picked again until something else changes (no spin).
    parked: BTreeSet<String>,
    shutdown: bool,
    /// Set by [`Queue::freeze`], never cleared: `persist` writes nothing from then on (F2b's sign-out).
    frozen: bool,
    /// Running jobs a drain gave up on (§4.10: waiting for mail writes nothing). Their thread ends
    /// on its own; whatever it returns is read as `Stopped`.
    abandoned: BTreeSet<String>,
}

impl Core {
    fn lane(job: &str) -> usize { LANE.contains(&job) as usize }
    fn pick(&self, lane: usize) -> Option<String> {
        let runnable = |j: &&String| Core::lane(j) == lane && !self.parked.contains(*j) && self.state.jobs.get(*j).is_some_and(|r| r.state == JobState::Pending);
        self.order.iter().find(|j| runnable(j) && (self.ready || UNGATED.contains(&j.as_str()))).cloned()
    }
    /// A gated job is `held` while not entitled and `pending` again once it is.
    fn settle(&mut self) {
        for j in self.order.iter().filter(|j| !UNGATED.contains(&j.as_str())) {
            if let Some(r) = self.state.jobs.get_mut(j) {
                match (r.state, self.entitled) {
                    (JobState::Pending, false) => r.state = JobState::Held,
                    (JobState::Held, true) => r.state = JobState::Pending,
                    _ => {}
                }
            }
        }
    }
    fn finish(&mut self, job: &str, lane: usize, out: Outcome) {
        self.busy[lane] = false;
        let (state, proof, reason) = match out {
            Outcome::Done(p) => (JobState::Done, p, None),
            Outcome::Failed(r) => (JobState::Failed, BTreeMap::new(), Some(r)),
            Outcome::Slot(SlotRun { reason: Some(r), .. }) => (JobState::Failed, BTreeMap::new(), Some(r)),
            Outcome::Slot(_) => (JobState::Done, BTreeMap::new(), None),
            Outcome::Waiting(r) => (JobState::Pending, BTreeMap::new(), Some(r)),
            Outcome::Stopped => (JobState::Pending, BTreeMap::new(), None),
        };
        if state == JobState::Pending { self.parked.insert(job.to_string()); } else { self.order.retain(|j| j != job); self.parked.clear(); }
        self.state.jobs.insert(job.to_string(), JobRecord { state, proof, reason });
    }
}

struct Inner { deps: Deps, core: Mutex<Core>, cv: Condvar, stop: AtomicBool }

impl Inner {
    fn lock(&self) -> MutexGuard<'_, Core> { self.core.lock().unwrap_or_else(|e| e.into_inner()) }
    /// The one path every change persists through, the queue being the only writer of the file while
    /// it is live. A frozen queue refuses (the freeze lives in `Core`, so a write in flight finishes
    /// before [`Queue::freeze`] returns, and none starts after it).
    fn try_persist(&self, c: &Core) -> Result<(), String> {
        if c.frozen { return Err("the onboarding file is frozen: a save after the freeze is refused".into()); }
        save_state(&self.deps.data_dir, &c.state)
    }
    fn persist(&self, c: &Core) {
        if let Err(e) = self.try_persist(c) { eprintln!("onboarding: could not save the job states: {e}"); }
    }
    /// The next job for `lane`; none once the stop is set (checked under the lock `request_stop` takes).
    fn next(&self, c: &Core, lane: usize) -> Option<String> {
        if self.stop.load(Ordering::SeqCst) { None } else { c.pick(lane) }
    }
    /// Reads the entitlement without holding the lock: a cache short of `Entitled` gets one refresh.
    /// Under unwinding (tests and dev; release is `panic = "abort"`), a panic in either call reads as
    /// "not entitled", so the jobs stay held rather than the thread dying.
    fn read_entitlement(&self, refresh: bool) -> bool {
        let e = &self.deps.entitlement;
        let read = || { if !e.entitled() && refresh { e.refresh(); } e.entitled() };
        std::panic::catch_unwind(std::panic::AssertUnwindSafe(read)).unwrap_or(false)
    }
}

/// `slot.partial` is `BeforeConfig` until `config.ingest` is done, then `NoRank` (F8e refines it).
fn scope_for(job: &str, s: &OnboardingState) -> Option<SlotScope> {
    let ingested = s.jobs.get("config.ingest").is_some_and(|r| r.state == JobState::Done);
    match job {
        "slot.first_full" => Some(SlotScope::Full),
        "slot.partial" => Some(if ingested { SlotScope::NoRank } else { SlotScope::BeforeConfig }),
        _ => None,
    }
}

/// The one startup read (a cache short of `Entitled` gets one refresh), on its own detached thread so
/// that `school.capture` and `school.preview` never wait for it and a drop never waits on a refresh. It leaves a newer `wake()` read alone.
fn startup(inner: Arc<Inner>) {
    let ent = inner.read_entitlement(true);
    let mut g = inner.lock();
    // The queue was dropped while the refresh ran (this thread is detached): write nothing.
    if g.shutdown { return; }
    // A wake's read is newer, so a stale `false` here never undoes it; a fresh `true` is always kept.
    g.entitled |= ent;
    g.ready = true;
    g.settle();
    inner.persist(&g);
    inner.cv.notify_all();
}

fn work(inner: Arc<Inner>, lane: usize) {
    let mut g = inner.lock();
    while !g.shutdown {
        let Some(job) = inner.next(&g, lane) else { g = inner.cv.wait(g).unwrap_or_else(|e| e.into_inner()); continue };
        g.busy[lane] = true;
        g.state.jobs.insert(job.clone(), JobRecord { state: JobState::Running, ..Default::default() });
        inner.persist(&g);
        let scope = scope_for(&job, &g.state);
        drop(g);
        let run = || inner.deps.runner.run(&job, scope, &inner.stop);
        // Holds under unwinding only (tests and dev): in release `panic = "abort"` ends the process.
        let out = std::panic::catch_unwind(std::panic::AssertUnwindSafe(run)).unwrap_or_else(|_| Outcome::Failed(REASON_JOB_CRASHED.into()));
        g = inner.lock();
        // A job the drain abandoned already reads `pending`; whatever it returns late changes nothing.
        let out = if g.abandoned.remove(&job) { Outcome::Stopped } else { out };
        g.finish(&job, lane, out);
        g.settle();
        inner.persist(&g);
        inner.cv.notify_all();
    }
}

pub struct Queue { inner: Arc<Inner>, threads: Vec<JoinHandle<()>> }

impl Drop for Queue {
    fn drop(&mut self) {
        // A running job sees the stop (never cleared, §4.10) and returns, so the joins end.
        self.inner.stop.store(true, Ordering::SeqCst);
        self.inner.lock().shutdown = true;
        self.inner.cv.notify_all();
        for t in self.threads.drain(..) { let _ = t.join(); }
    }
}

impl Queue {
    /// Spawns the serial thread and the lane thread, over `state` (running jobs already `pending`).
    pub fn start(deps: Deps, mut state: OnboardingState) -> Queue {
        let mut order = Vec::new();
        for (name, r) in state.jobs.iter_mut() {
            if r.state == JobState::Running { r.state = JobState::Pending; }
            if matches!(r.state, JobState::Pending | JobState::Held) { order.push(name.clone()); }
        }
        order.sort_by_key(|j| (JOB_ORDER.iter().position(|c| c == j).unwrap_or(JOB_ORDER.len()), j.clone()));
        let core = Core { state, order, ready: false, entitled: false, busy: [false; 2], parked: BTreeSet::new(), shutdown: false, frozen: false, abandoned: BTreeSet::new() };
        let inner = Arc::new(Inner { deps, core: Mutex::new(core), cv: Condvar::new(), stop: AtomicBool::new(false) });
        let spawn = |lane: usize, name: &str| {
            let i = inner.clone();
            std::thread::Builder::new().name(name.into()).spawn(move || work(i, lane)).expect("the onboarding queue could not start a thread")
        };
        // The start read is detached, not joined: a refresh can take a minute, and `startup` writes
        // nothing once the queue is dropped.
        let i = inner.clone();
        std::thread::Builder::new().name("onboarding-start".into()).spawn(move || startup(i)).expect("the onboarding queue could not start a thread");
        let threads = vec![spawn(0, QUEUE_THREAD), spawn(1, LANE_THREAD)];
        Queue { inner, threads }
    }
    /// An ask was answered: the job is `pending` (or `held`) and runs when its inputs are ready.
    pub fn enqueue(&self, job: &str) {
        let mut g = self.inner.lock();
        g.parked.clear();
        if !g.order.iter().any(|j| j == job) {
            g.order.push(job.to_string());
            g.state.jobs.insert(job.to_string(), JobRecord::default());
        }
        g.settle();
        self.inner.persist(&g);
        self.inner.cv.notify_all();
    }
    /// The entitlement changed (F2h's poll): re-test the hold.
    pub fn wake(&self) {
        let ent = self.inner.read_entitlement(false);
        let mut g = self.inner.lock();
        (g.entitled, g.parked) = (ent, BTreeSet::new());
        g.settle();
        self.inner.persist(&g);
        self.inner.cv.notify_all();
    }
    /// "Try again" on a job (resets the chain count for `school.courses` while unsettled).
    pub fn retry(&self, _job: &str) { todo!("F8e: retry") }
    /// Step 10's confirm: stamps `step10_confirmed_at` and enqueues `slot.first_full`.
    pub fn confirm_step10(&self, at: &str) {
        self.inner.lock().state.step10_confirmed_at = Some(at.to_string());
        self.enqueue("slot.first_full");
    }
    /// The student confirmed the school offered at step 3: marks it confirmed on the live state and
    /// wakes the queue, so a `school.apply` that was waiting on it can start (§4.7).
    pub fn confirm_school(&self) { todo!("F8e: confirm the school") }
    pub fn snapshot(&self) -> OnboardingState { self.inner.lock().state.clone() }
    /// True when nothing is running and nothing runnable is waiting, within `timeout`.
    pub fn wait_idle(&self, timeout: Duration) -> bool {
        let deadline = Instant::now() + timeout;
        let mut g = self.inner.lock();
        while !(g.ready && !g.busy.contains(&true) && self.inner.next(&g, 0).is_none() && self.inner.next(&g, 1).is_none()) {
            let Some(left) = deadline.checked_duration_since(Instant::now()) else { return false };
            g = self.inner.cv.wait_timeout(g, left).unwrap_or_else(|e| e.into_inner()).0;
        }
        true
    }
    /// Sets the stop (§4.10): no job starts after this returns, and each running job sees the flag at
    /// its own safe point. It is never cleared. Set under the lock a worker holds while it picks and
    /// marks a job busy, so a job is either already busy (the drain waits for it) or never starts.
    pub fn request_stop(&self) {
        let _g = self.inner.lock();
        self.inner.stop.store(true, Ordering::SeqCst);
        self.inner.cv.notify_all();
    }
    pub fn stop_requested(&self) -> bool { self.inner.stop.load(Ordering::SeqCst) }
    /// Waits for each lane's running job to reach its safe point. A running `mail.canary` or
    /// `mail.first_pass` is not waited for: it is set back to `pending` and left to end on its own
    /// (waiting for mail writes nothing, §4.10). Call [`Queue::request_stop`] first.
    pub fn drain(&self) {
        let mut g = self.inner.lock();
        let waiting_mail: Vec<String> = g.state.jobs.iter()
            .filter(|(n, r)| r.state == JobState::Running && ABANDON_AT_ONCE.contains(&n.as_str())).map(|(n, _)| n.clone()).collect();
        for job in waiting_mail {
            g.state.jobs.insert(job.clone(), JobRecord::default());
            g.parked.insert(job.clone());
            g.abandoned.insert(job);
        }
        let held = |g: &Core, lane: usize| g.busy[lane] && !g.abandoned.iter().any(|j| Core::lane(j) == lane);
        while held(&g, 0) || held(&g, 1) { g = self.inner.cv.wait(g).unwrap_or_else(|e| e.into_inner()); }
    }
    /// Persists now; refused after [`Queue::freeze`].
    pub fn save(&self) -> Result<(), String> {
        let g = self.inner.lock();
        self.inner.try_persist(&g)
    }
    /// After this no save the queue makes lands (F2b's sign-out sets it after its last save). Never
    /// cleared. F2a's `save_state` has no such flag, so the rewind written after it always lands.
    pub fn freeze(&self) { self.inner.lock().frozen = true; }
    pub fn is_frozen(&self) -> bool { self.inner.lock().frozen }
    /// Finish's write, through the queue's own state: sets `finished` and `handover` on the live
    /// state and saves it, so no later save can write an older, unfinished copy over it.
    pub fn finish(&self, handover: Vec<HandoverItem>) -> Result<(), String> {
        let mut g = self.inner.lock();
        if g.frozen { return Err("the onboarding file is frozen: finish is refused".into()); }
        (g.state.finished, g.state.handover) = (true, handover);
        self.inner.try_persist(&g)
    }
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

#[cfg(test)]
mod stop_tests {
    //! F8f's own tests over the queue alone (F1's test 9 and 17 go through F2b's exits, which are
    //! still stubs): the stop, the drain, the abandoned mail job, the freeze and finish's write.
    use super::*;

    struct Gates(Mutex<BTreeSet<String>>);
    impl Gates {
        fn open(&self, job: &str) { self.0.lock().unwrap().insert(job.into()); }
        fn is_open(&self, job: &str) -> bool { self.0.lock().unwrap().contains(job) }
    }
    /// Every job waits for its gate; `heed` jobs also end at the stop, as `Stopped`.
    struct Blocking { gates: Arc<Gates>, ran: Mutex<Vec<String>>, heed: Vec<&'static str> }
    impl Runner for Blocking {
        fn run(&self, job: &str, _scope: Option<SlotScope>, stop: &AtomicBool) -> Outcome {
            self.ran.lock().unwrap().push(job.into());
            let deadline = Instant::now() + Duration::from_secs(20);
            while !self.gates.is_open(job) {
                assert!(Instant::now() < deadline, "a gate was never opened");
                if self.heed.contains(&job) && stop.load(Ordering::SeqCst) { return Outcome::Stopped; }
                std::thread::sleep(Duration::from_millis(2));
            }
            Outcome::Done(BTreeMap::new())
        }
    }
    struct Paid;
    impl Entitlement for Paid { fn entitled(&self) -> bool { true } fn refresh(&self) {} }
    struct NoSync;
    impl SyncView for NoSync { fn read(&self, _s: &str) -> (Option<SyncStatus>, Option<SyncCursor>) { (None, None) } }

    fn rig(tag: &str, heed: Vec<&'static str>) -> (Queue, Arc<Gates>, Arc<Blocking>, std::path::PathBuf) {
        let dir = std::env::temp_dir().join(format!("knowlu-f8f-{tag}-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        let gates = Arc::new(Gates(Mutex::new(BTreeSet::new())));
        let runner = Arc::new(Blocking { gates: gates.clone(), ran: Mutex::new(Vec::new()), heed });
        let deps = Deps { data_dir: dir.clone(), runner: runner.clone(), entitlement: Arc::new(Paid), sync_view: Arc::new(NoSync) };
        (Queue::start(deps, OnboardingState::new("2026-10-24T09:00:00Z", 8)), gates, runner, dir)
    }
    fn wait_for(what: &str, cond: impl Fn() -> bool) {
        let deadline = Instant::now() + Duration::from_secs(20);
        while !cond() { assert!(Instant::now() < deadline, "timed out waiting for {what}"); std::thread::sleep(Duration::from_millis(2)); }
    }
    fn state_in(dir: &Path, job: &str) -> JobState {
        match crate::onboarding_state::load_state(dir) {
            crate::onboarding_state::Loaded::Ok(s) => s.jobs[job].state,
            other => panic!("{other:?}"),
        }
    }

    #[test]
    fn the_stop_starts_nothing_new_and_the_drain_waits_for_the_running_job_in_each_lane() {
        let (q, gates, runner, dir) = rig("drain", vec![]);
        for j in ["coursework.discover", "school.apply", "mail.setup", "school.courses"] { q.enqueue(j); }
        wait_for("both lanes busy", || runner.ran.lock().unwrap().len() == 2);
        q.request_stop();
        assert!(q.stop_requested());
        let done = Arc::new(AtomicBool::new(false));
        std::thread::scope(|sc| {
            let t = sc.spawn(|| { q.drain(); done.store(true, Ordering::SeqCst); });
            std::thread::sleep(Duration::from_millis(100));
            assert!(!done.load(Ordering::SeqCst), "the drain returned with two jobs running");
            gates.open("coursework.discover");
            std::thread::sleep(Duration::from_millis(100));
            assert!(!done.load(Ordering::SeqCst), "the drain returned with the lane job running");
            gates.open("school.apply");
            t.join().unwrap();
        });
        assert!(q.wait_idle(Duration::from_secs(5)), "nothing runnable counts once the stop is set");
        assert_eq!(runner.ran.lock().unwrap().len(), 2, "mail.setup and school.courses never started");
        q.save().unwrap();
        assert_eq!(state_in(&dir, "coursework.discover"), JobState::Done);
        assert_eq!(state_in(&dir, "mail.setup"), JobState::Pending);
    }

    #[test]
    fn a_running_mail_canary_is_abandoned_at_once_and_its_late_return_changes_nothing() {
        let (q, gates, runner, dir) = rig("canary", vec![]);
        q.enqueue("mail.canary");
        wait_for("the canary", || !runner.ran.lock().unwrap().is_empty());
        q.request_stop();
        let began = Instant::now();
        q.drain();
        assert!(began.elapsed() < Duration::from_secs(2), "the drain waited for the canary");
        q.save().unwrap();
        assert_eq!(state_in(&dir, "mail.canary"), JobState::Pending);
        gates.open("mail.canary"); // it ends `Done` after the abandon
        assert!(q.wait_idle(Duration::from_secs(10)));
        assert_eq!(q.snapshot().jobs["mail.canary"].state, JobState::Pending, "abandoned, so it runs again from the start");
        q.save().unwrap();
        assert_eq!(state_in(&dir, "mail.canary"), JobState::Pending);
    }

    #[test]
    fn a_frozen_queue_writes_nothing_and_finish_writes_through_the_queues_own_state() {
        let (q, _gates, _runner, dir) = rig("freeze", vec![]);
        q.enqueue("school.capture");
        q.request_stop();
        q.save().unwrap();
        let handover = vec![HandoverItem { item: "slot.first_full".into(), signal: "slot_end".into() }];
        q.finish(handover.clone()).unwrap();
        let on_disk = std::fs::read(crate::onboarding_state::state_path(&dir)).unwrap();
        assert!(q.snapshot().finished && q.snapshot().handover == handover);
        assert!(!q.is_frozen());
        q.freeze();
        assert!(q.is_frozen());
        assert!(q.save().is_err() && q.finish(Vec::new()).is_err());
        q.enqueue("school.preview");
        q.wake();
        assert_eq!(std::fs::read(crate::onboarding_state::state_path(&dir)).unwrap(), on_disk, "no queue write lands after the freeze");
        // The rewind goes through F2a's save, which has no freeze, and lands.
        crate::onboarding_state::save_state(&dir, &OnboardingState::new("2026-10-24T09:00:00Z", 2)).unwrap();
        assert_ne!(std::fs::read(crate::onboarding_state::state_path(&dir)).unwrap(), on_disk);
    }
}
