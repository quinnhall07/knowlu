//! The job queue (plan §4.3, F8a-F8h) and the runners that are pure enough to test: the feed store,
//! `school.courses`, the `config.ingest` hand-over, the link commands that only validate.
//!
//! **F8a** is the queue's core (below). The runners, the notes gate and the link commands further
//! down are still F1 stubs: their types and signatures are the contract `app/tests/onboarding_jobs.rs`
//! writes against, and each panics (`todo!`, naming its task) until that task replaces it.
//! **While a [`Queue`] is live it is the only writer of `onboarding.json`**: the commands (F2) and the
//! window's exits (F4) change the file through the queue's methods, never `save_state` directly. The
//! one exception is the sign-out rewind, which writes after [`Queue::freeze`] (F8f, F2e).
use crate::onboarding_state::{confirm_school, load_answers, proposals_ready, save_state, school_apply_may_run, update_answers, Answers, CourseAnswer, HandoverItem, JobRecord, JobState, OnboardingState, PendingStore};
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
    /// Read once after each slot job ends (`BeforeConfig`, `NoRank` and `Full`); `slot_started` is its
    /// `RunSummary.started`. **The read runs while that job still counts as running** (so a note job
    /// cannot start beside it): a real view reads the two state files only, takes no `vault_io` and
    /// makes no network call.
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
/// The lane jobs that write notes: they and the slots never run at once (§4.3, F8h's gate guards the runners).
const NOTE_JOBS: [&str; 2] = ["school.courses", "registrar.confirm"];
/// The order a resumed queue runs its jobs in: step order, then ask order within a step (§4.2).
/// A name not listed sorts after these, by name.
pub const JOB_ORDER: [&str; 17] = [
    "school.capture", "school.preview", "school.store", "school.apply", "school.courses", "registrar.confirm",
    "mail.setup", "mail.canary", "mail.first_pass", "google.gmail_first_read", "calendar.store", "calendar.read", "google.calendar_read",
    "coursework.discover", "config.ingest", "slot.partial", "slot.first_full",
];
/// Jobs a drain never waits for: waiting for mail writes nothing, so they start again from the top (§4.10).
/// That holds only while their runners write nothing to the vault and hold neither the notes gate
/// (F8h) nor the school session lock; a runner that ever does must come off this list. An abandoned
/// runner keeps its thread: `Drop` still joins it, after setting the stop it is expected to heed.
/// Every other running job is waited for, with no time limit: an abandoned writer would write after
/// Sign out's `on_sign_out` (§14.51 r1, C1; F1 test 30 pins this list).
pub const ABANDON_AT_ONCE: [&str; 2] = ["mail.canary", "mail.first_pass"];
/// The closed-set reason of a job the crash-loop guard paused (found `running` at two starts, the
/// release profile's way, since it has `panic = "abort"`) and of one whose runner panicked under
/// unwinding (tests and dev). The page maps it to its fix, [`JOB_CRASHED_FIX`]. Only
/// [`Queue::retry`] may write over a record that carries it.
pub const REASON_JOB_CRASHED: &str = "job_crashed";
/// Launches that find a job `running` before it is paused with [`REASON_JOB_CRASHED`].
const CRASH_LIMIT: u8 = 2;
/// What the page says on a `job_crashed` job's row, beside its Try again (F11; §4.3's second
/// closed-set fix, §14.51 r1 I5). It blames no step: a Windows shutdown or End task counts too.
pub const JOB_CRASHED_FIX: &str = "Knowlu stopped twice while this step was running, so it has paused it. Press Try again to run it once more.";
/// `school.courses` when the `BeforeConfig` chain ended without the account's copy read to its end
/// (§4.3, Q-B20); "Try again" starts a new chain. **Still a sentence, not a key** (review r1, M5):
/// F1's tests 16 pin these words with `contains`, so it becomes `account_unfinished` only when that
/// test changes. Every other reason in this file is a short key the page maps to text.
pub const REASON_ACCOUNT_UNFINISHED: &str = "couldn't finish reading your account";
/// A `pending` `school.courses` while the account is unsettled (§4.3:827-828). **A sentence, not a
/// key**, for the same reason: F1's test 14 pins these words exactly.
pub const REASON_WAITING_FOR_ACCOUNT: &str = "waiting to reach your account";
/// "First sync" when a fresh run of `sync` reported an error: a closed-set key, never the status's own text (the page maps it).
pub const REASON_FIRST_SYNC_FAILED: &str = "first_sync_failed";
/// A progress indicator the queue keeps in `jobs` beside the real jobs: never picked, never resumed.
pub const FIRST_SYNC: &str = "first.sync";
const INDICATOR_PREFIX: &str = "first.";

/// The `BeforeConfig` chain's memory (§4.3, Q-B20): runs so far, readings in a row that were not a
/// sync, and the cursors at the last reading. Not persisted: a relaunch starts a new chain.
#[derive(Default)]
struct Chain { runs: u32, not_a_sync: u32, prev: (i64, i64) }

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
    /// When the first [`Queue::drain`] began (`journal::now_ts`); what the page counts its elapsed time from.
    drain_began: Option<String>,
    chain: Chain,
    /// A `BeforeConfig` chain has ended in this queue (given up, or a slot failed with none left). Only
    /// then does a `school.courses` pushed onto an unsettled account start a chain itself (C1).
    chain_ended: bool,
    /// A `slot.partial` ask merged into one that may not run `NoRank` (I1): one `NoRank` slot is owed
    /// once `config.ingest` is done. Not persisted, like the chain.
    no_rank_owed: bool,
    /// The last `slot.partial` started as `NoRank` (read only while it is running, r3).
    partial_no_rank: bool,
    /// The profile's app data: where the answers file the config jobs read lives.
    dir: std::path::PathBuf,
    /// F8d2: when a job parked on a reason in [`TIMER_RETRY_REASONS`] is picked again; read against the queue's clock.
    retry_at: BTreeMap<String, Instant>,
    /// F8d2: mail jobs the student skipped while they ran: a `Stopped` or `Waiting` end lands `skipped`.
    skipping: BTreeSet<String>,
    /// F8d2: the running `mail.setup`'s run, once its glue hands it over ([`Queue::attach_setup_run`]).
    setup_run: Option<Arc<dyn SetupRun>>,
}

impl Core {
    fn lane(job: &str) -> usize { LANE.contains(&job) as usize }
    fn pick(&self, lane: usize) -> Option<String> {
        self.order.iter().find(|j| {
            Core::lane(j) == lane && self.waiting(j) && (self.ready || UNGATED.contains(&j.as_str())) && self.inputs_ready(j) && !self.notes_conflict(j)
        }).cloned()
    }
    /// `pending`, not parked by a `Waiting` or `Stopped` outcome, and not holding a retry time: a job
    /// parked on a timer reason waits for its 30 s however many other jobs finish meanwhile (F8d2).
    fn waiting(&self, job: &str) -> bool { !self.parked.contains(job) && !self.retry_at.contains_key(job) && self.job_state(job) == Some(JobState::Pending) }
    /// The note jobs and the slots take turns (§4.3). Decided here, under the lock that marks a job
    /// running, so the two never overlap and no thread has to wait for the other. With the account
    /// settled the lane's config jobs are ready first, so a slot also waits for a note job or
    /// `config.ingest` that is running or ready: it then runs once, with the scope the finished
    /// config earns, rather than as `BeforeConfig` with a `NoRank` run behind it.
    fn notes_conflict(&self, job: &str) -> bool {
        let running = |f: fn(&str) -> bool| self.state.jobs.iter().any(|(n, r)| r.state == JobState::Running && f(n));
        let (note, slot, config): (fn(&str) -> bool, fn(&str) -> bool, fn(&str) -> bool) =
            (|n| NOTE_JOBS.contains(&n), |n| n.starts_with("slot."), |n| NOTE_JOBS.contains(&n) || n == "config.ingest");
        if note(job) { running(slot) }
        else if slot(job) { running(config) || self.order.iter().any(|n| config(n) && self.waiting(n) && self.inputs_ready(n)) }
        else { false }
    }
    fn job_state(&self, job: &str) -> Option<JobState> { self.state.jobs.get(job).map(|r| r.state) }
    /// A job waits only for its own inputs (§4.3): config before its readers, the notes after the
    /// account, a `NoRank` slot (whose `grades` reads the course notes) after `school.apply`. A job
    /// nothing names is always ready.
    fn inputs_ready(&self, job: &str) -> bool {
        let done = |n: &str| self.job_state(n) == Some(JobState::Done);
        let settled = |n: &str| matches!(self.job_state(n), None | Some(JobState::Done | JobState::Failed | JobState::Skipped));
        // An absent or unreadable answers file is never "no answers" (§4.5): no config job runs.
        let answers = || matches!(load_answers(&self.dir), Ok(Some(_)));
        match job {
            "school.apply" => school_apply_may_run(&self.state),
            "school.courses" => self.state.account_settled && answers(),
            "registrar.confirm" => done("school.apply") && done("school.courses") && answers(),
            "config.ingest" => done("school.courses") && settled("school.store") && settled("calendar.store") && answers(),
            "calendar.read" | "google.calendar_read" => done("config.ingest"),
            "slot.first_full" => done("config.ingest") && settled("school.apply"),
            "slot.partial" => !done("config.ingest") || settled("school.apply"),
            _ => true,
        }
    }
    /// F8d2: unparks every job whose retry time has come; true when any did.
    fn unpark_due(&mut self, now: Instant) -> bool {
        let due: Vec<String> = self.retry_at.iter().filter(|(_, at)| **at <= now).map(|(j, _)| j.clone()).collect();
        for j in &due { self.retry_at.remove(j); self.parked.remove(j); }
        !due.is_empty()
    }
    /// `failed` with [`REASON_JOB_CRASHED`]: the crash-loop guard's pause (or a panic's).
    fn is_paused(&self, job: &str) -> bool {
        self.state.jobs.get(job).is_some_and(|r| r.state == JobState::Failed && r.reason.as_deref() == Some(REASON_JOB_CRASHED))
    }
    /// The student's Try again asks for `job`: the one writer allowed over a paused record. A paused job
    /// is put back to `pending` with a count of `CRASH_LIMIT - 1`, so it runs once more and one more
    /// kill pauses it again. Any other job is pushed as [`Core::push_job`] does.
    fn press(&mut self, job: &str) {
        let paused = self.is_paused(job);
        if paused { self.order.retain(|j| j != job); self.state.jobs.remove(job); }
        self.push_job(job);
        if let Some(r) = self.state.jobs.get_mut(job).filter(|_| paused) { r.crashes = CRASH_LIMIT - 1; }
    }
    /// Adds `job` as `pending` unless it is already queued. A progress indicator is never a job.
    /// `school.courses` pushed while the account is unsettled, after a chain has ended and with no
    /// slot queued to settle it, starts a new chain, so it cannot wait for ever (Q-B20). Before any
    /// chain has ended it waits for the slot that payment asks for (C1, test 14).
    fn push_job(&mut self, job: &str) {
        if job.starts_with(INDICATOR_PREFIX) { return; }
        // A paused job stays paused until the student presses Try again ([`Core::press`]).
        if self.is_paused(job) { return; }
        self.parked.clear();
        if job == "school.courses" && !self.state.account_settled && self.chain_ended && !self.order.iter().any(|j| j == "slot.partial") {
            (self.chain, self.chain_ended) = (Chain::default(), false);
            self.push_job("slot.partial");
        }
        if !self.order.iter().any(|j| j == job) {
            self.order.push(job.to_string());
            self.state.jobs.insert(job.to_string(), JobRecord::default());
        }
    }
    /// The chain has ended: a `school.courses` that nothing will settle (no slot is queued to read
    /// the account) stops waiting.
    fn fail_courses(&mut self) {
        self.chain_ended = true;
        if matches!(self.job_state("school.courses"), Some(JobState::Pending | JobState::Held)) {
            self.order.retain(|j| j != "school.courses");
            self.state.jobs.insert("school.courses".into(), JobRecord { state: JobState::Failed, reason: Some(REASON_ACCOUNT_UNFINISHED.into()), ..Default::default() });
        }
    }
    /// After a `slot.partial` ended (not stopped): with the account unsettled and no slot left to
    /// settle it, the chain has ended.
    fn after_partial(&mut self) {
        if !self.state.account_settled && !self.order.iter().any(|j| j == "slot.partial") { self.fail_courses(); }
    }
    /// Pays the owed `NoRank` slot (I1) once it can run as one: `config.ingest` done and no
    /// `slot.partial` already queued (a queued one runs `NoRank` and pays it when it starts).
    fn pay_no_rank(&mut self) {
        if self.no_rank_owed && proposals_ready(&self.state) && !self.order.iter().any(|j| j == "slot.partial") {
            self.no_rank_owed = false;
            self.push_job("slot.partial");
        }
    }
    /// A pending `school.courses` on an unsettled account says why it waits (§4.3:827-828); the
    /// reason goes once the account settles or the job moves on.
    fn mark_account_wait(&mut self) {
        let settled = self.state.account_settled;
        if let Some(r) = self.state.jobs.get_mut("school.courses") {
            let waits = r.state == JobState::Pending && !settled;
            if waits { r.reason = Some(REASON_WAITING_FOR_ACCOUNT.into()); }
            else if matches!(r.state, JobState::Pending | JobState::Held) && r.reason.as_deref() == Some(REASON_WAITING_FOR_ACCOUNT) { r.reason = None; }
        }
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
        self.mark_account_wait();
    }
    fn finish(&mut self, job: &str, lane: usize, out: Outcome) {
        self.busy[lane] = false;
        // F8d2: a mail job the student skipped while it ran lands `skipped` when it stops, parks or fails (a
        // student who chose to skip is never told to finish connecting); one that ended done stays done.
        let skipped = self.skipping.remove(job) && matches!(out, Outcome::Stopped | Outcome::Waiting(_) | Outcome::Failed(_));
        if job == "mail.setup" { self.setup_run = None; }
        let (state, proof, reason) = match out {
            _ if skipped => (JobState::Skipped, BTreeMap::new(), None),
            Outcome::Done(p) => (JobState::Done, p, None),
            Outcome::Failed(r) => (JobState::Failed, BTreeMap::new(), Some(r)),
            Outcome::Slot(SlotRun { reason: Some(r), .. }) => (JobState::Failed, BTreeMap::new(), Some(r)),
            Outcome::Slot(_) => (JobState::Done, BTreeMap::new(), None),
            Outcome::Waiting(r) => (JobState::Pending, BTreeMap::new(), Some(r)),
            Outcome::Stopped => (JobState::Pending, BTreeMap::new(), None),
        };
        if state == JobState::Pending { self.parked.insert(job.to_string()); } else { self.order.retain(|j| j != job); self.parked.clear(); }
        self.state.jobs.insert(job.to_string(), JobRecord { state, proof, reason, ..Default::default() });
        if job == "config.ingest" || job == "slot.partial" { self.pay_no_rank(); }
    }
    /// Reads what a slot that ended left in the sync status and cursor (§4.3's three readings): sets
    /// "first sync" and `account_settled`, and while the account is unsettled keeps the `BeforeConfig`
    /// chain going or ends it (Q-B20). `started` and the status's `at` are both `journal::now_ts`.
    fn after_slot(&mut self, scope: SlotScope, started: &str, counting: bool, status: Option<SyncStatus>, cursor: Option<SyncCursor>) {
        // `Some(ok)` only for a fresh stamp with no `skipped`; anything else is not a sync.
        let ran = status.filter(|s| s.at.as_str() >= started && s.skipped.is_none()).map(|s| s.ok);
        match ran {
            Some(true) => {
                self.state.jobs.insert(FIRST_SYNC.into(), JobRecord { state: JobState::Done, ..Default::default() });
                if cursor.is_some_and(|c| c.pulled_to_end) { self.settle_account(); }
            }
            // A settled account's sync that fails later is a retry's business, not a first sync undone.
            Some(false) if self.state.account_settled && self.job_state(FIRST_SYNC) == Some(JobState::Done) => {}
            Some(false) => { self.state.jobs.insert(FIRST_SYNC.into(), JobRecord { state: JobState::Failed, reason: Some(REASON_FIRST_SYNC_FAILED.into()), ..Default::default() }); }
            None => {}
        }
        // A stopped queue counts nothing: the chain starts over when the queue does.
        if scope != SlotScope::BeforeConfig || self.state.account_settled || !counting { return; }
        self.chain.runs += 1;
        let cursors = cursor.map(|c| (c.record_cursor, c.note_cursor));
        let more = match ran {
            Some(false) => false,
            Some(true) => {
                self.chain.not_a_sync = 0;
                let moved = cursors.is_some_and(|c| c.0 > self.chain.prev.0 || c.1 > self.chain.prev.1);
                self.chain.prev = cursors.unwrap_or(self.chain.prev);
                moved
            }
            None => { self.chain.not_a_sync += 1; self.chain.not_a_sync < 2 }
        };
        if more && self.chain.runs < CHAIN_BACKSTOP { return self.push_job("slot.partial"); }
        self.fail_courses();
    }
    /// The account's copy has been read to its end. A `school.courses` that gave up waiting for it
    /// goes back to `pending` (M3).
    fn settle_account(&mut self) {
        self.state.account_settled = true;
        let gave_up = self.state.jobs.get("school.courses").is_some_and(|r| r.state == JobState::Failed && r.reason.as_deref() == Some(REASON_ACCOUNT_UNFINISHED));
        if gave_up {
            self.state.jobs.remove("school.courses");
            self.push_job("school.courses");
        }
    }
}

struct Inner { deps: Deps, core: Mutex<Core>, cv: Condvar, stop: AtomicBool, batch_closed: AtomicBool, clock: Arc<dyn RetryClock> }

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
    /// F8d2: the idle wait. With a retry time pending (and no stop) it wakes at that time on the
    /// queue's clock, at least every [`CLOCK_LOOK`] so a clock the test moves by hand is noticed;
    /// otherwise it waits for a notify, as before. A stop ends the timer's wait: nothing is re-picked.
    fn idle<'a>(&self, g: MutexGuard<'a, Core>) -> MutexGuard<'a, Core> {
        let next = if self.stop.load(Ordering::SeqCst) { None } else { g.retry_at.values().min().copied() };
        match next {
            Some(at) => {
                let left = at.saturating_duration_since(self.clock.now()).min(CLOCK_LOOK);
                self.cv.wait_timeout(g, left).unwrap_or_else(|e| e.into_inner()).0
            }
            None => self.cv.wait(g).unwrap_or_else(|e| e.into_inner()),
        }
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
    let ingested = proposals_ready(s);
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
        if g.unpark_due(inner.clock.now()) { inner.cv.notify_all(); }
        let Some(job) = inner.next(&g, lane) else { g = inner.idle(g); continue };
        g.busy[lane] = true;
        g.retry_at.remove(&job);
        // The running record keeps the count it is counting (the crash-loop guard); `finish` resets it.
        let crashes = g.state.jobs.get(&job).map_or(0, |r| r.crashes);
        g.state.jobs.insert(job.clone(), JobRecord { state: JobState::Running, crashes, ..Default::default() });
        inner.persist(&g);
        let scope = scope_for(&job, &g.state);
        // A `NoRank` slot starting is the owed one, whatever asked for it (so it is never run twice).
        if job == "slot.partial" {
            g.partial_no_rank = scope == Some(SlotScope::NoRank);
            if g.partial_no_rank { g.no_rank_owed = false; }
        }
        drop(g);
        let run = || inner.deps.runner.run(&job, scope, &inner.stop);
        // Holds under unwinding only (tests and dev): in release `panic = "abort"` ends the process.
        let out = std::panic::catch_unwind(std::panic::AssertUnwindSafe(run)).unwrap_or_else(|_| Outcome::Failed(REASON_JOB_CRASHED.into()));
        // The status and cursor are read with no lock held, while the slot still counts as running,
        // so nothing the reading unlocks can start beside it.
        let reading = match (&out, scope) {
            (Outcome::Slot(SlotRun { started, reason: None, .. }), Some(s)) => {
                let read = || inner.deps.sync_view.read(started);
                Some((s, started.clone(), std::panic::catch_unwind(std::panic::AssertUnwindSafe(read)).unwrap_or((None, None))))
            }
            _ => None,
        };
        g = inner.lock();
        // A job the drain abandoned already reads `pending`; whatever it returns late changes nothing.
        let abandoned = g.abandoned.remove(&job);
        let out = if abandoned { Outcome::Stopped } else { out };
        let retry = matches!(&out, Outcome::Waiting(r) if TIMER_RETRY_REASONS.contains(&r.as_str()));
        g.finish(&job, lane, out);
        if retry && g.job_state(&job) == Some(JobState::Pending) { g.retry_at.insert(job.clone(), inner.clock.now() + TIMER_RETRY_AFTER); }
        let counting = !inner.stop.load(Ordering::SeqCst);
        if let Some((s, started, (status, cursor))) = reading.filter(|_| !abandoned) { g.after_slot(s, &started, counting, status, cursor); }
        if job == "slot.partial" && counting && !abandoned { g.after_partial(); }
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
    /// Spawns the serial thread and the lane thread, over `state`. A job found `running` was killed:
    /// it counts one crash and goes back to `pending`, and at its second it is `failed` with
    /// [`REASON_JOB_CRASHED`] and not run (the guard, F8a2). The result is saved once, before any thread.
    pub fn start(deps: Deps, state: OnboardingState) -> Queue { Queue::start_with_clock(deps, state, Arc::new(WallClock)) }
    /// [`Queue::start`] over a clock the retry timer reads (F8d2): a test moves its own by hand.
    pub fn start_with_clock(deps: Deps, mut state: OnboardingState, clock: Arc<dyn RetryClock>) -> Queue {
        let mut order = Vec::new();
        for (name, r) in state.jobs.iter_mut() {
            if r.state == JobState::Running {
                r.crashes = r.crashes.saturating_add(1);
                if r.crashes >= CRASH_LIMIT { (r.state, r.reason) = (JobState::Failed, Some(REASON_JOB_CRASHED.into())); } else { r.state = JobState::Pending; }
            }
            if matches!(r.state, JobState::Pending | JobState::Held) && !name.starts_with(INDICATOR_PREFIX) { order.push(name.clone()); }
        }
        order.sort_by_key(|j| (JOB_ORDER.iter().position(|c| c == j).unwrap_or(JOB_ORDER.len()), j.clone()));
        let mut core = Core { state, order, ready: false, entitled: false, busy: [false; 2], parked: BTreeSet::new(), shutdown: false, frozen: false, abandoned: BTreeSet::new(), drain_began: None, chain: Chain::default(), chain_ended: false, no_rank_owed: false, partial_no_rank: false, dir: deps.data_dir.clone(), retry_at: BTreeMap::new(), skipping: BTreeSet::new(), setup_run: None };
        // A resumed `school.courses` that waits on an unsettled account needs a slot to settle it.
        if core.order.iter().any(|j| j == "school.courses") && !core.state.account_settled { core.push_job("slot.partial"); }
        core.mark_account_wait();
        // Before the start read (which can take a minute) and before the ungated jobs run, so a job that
        // kills the process every time cannot outrun its own count (§14.51 r1, M2).
        if let Err(e) = save_state(&deps.data_dir, &core.state) { eprintln!("onboarding: could not save the job states: {e}"); }
        let inner = Arc::new(Inner { deps, core: Mutex::new(core), cv: Condvar::new(), stop: AtomicBool::new(false), batch_closed: AtomicBool::new(false), clock });
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
    /// A `slot.partial` ask that merges into a queued one owes a `NoRank` slot (I1), unless the queued
    /// one is `pending` with `config.ingest` already done, so it is sure to run as `NoRank` itself, or
    /// is already running as `NoRank`.
    pub fn enqueue(&self, job: &str) { self.enqueue_with(job, false); }
    /// An answer or a proceed (§14.52.3: a repeat changes nothing): like [`Queue::enqueue`], but a job that
    /// already has a `done`, `failed` or `skipped` record is left alone, under the same lock. A failed job
    /// is re-run only by Try again ([`Core::press`]); skipped mail jobs stay skipped. `slot.partial` is
    /// exempt: a second ask is its own merge rule (I1), and step 8 needs a `NoRank` slot after step 3's.
    /// Answers `true` when the job was newly queued (it was not queued, and nothing blocked it), so a proceed
    /// can ask for `slot.partial` only if one of its earlier jobs was.
    pub fn enqueue_once(&self, job: &str) -> bool { self.enqueue_with(job, true) }
    fn enqueue_with(&self, job: &str, once: bool) -> bool {
        let mut g = self.inner.lock();
        if once && job != "slot.partial" && matches!(g.job_state(job), Some(JobState::Done | JobState::Failed | JobState::Skipped)) { return false; }
        if job == "slot.partial" && g.order.iter().any(|j| j == job) {
            let covered = match g.job_state(job) {
                Some(JobState::Pending) => proposals_ready(&g.state),
                Some(JobState::Running) => g.partial_no_rank,
                _ => false,
            };
            if !covered { g.no_rank_owed = true; }
        }
        let queued = g.order.iter().any(|j| j == job);
        g.push_job(job);
        let fresh = !queued && g.order.iter().any(|j| j == job);
        g.settle();
        self.inner.persist(&g);
        self.inner.cv.notify_all();
        fresh
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
    /// "Try again" on a `failed` job puts it back to `pending`; any other state is left alone. For
    /// `school.courses` while the account is unsettled that also starts a new chain: one `BeforeConfig`
    /// slot, the count reset (Q-B20). A progress indicator is never deleted (§4.6, r3): a failed
    /// `first.sync` is reset to `pending` and the slot that re-reads it is queued; any other
    /// indicator (`first.grades`, F8g's) is left as it is, still named with its fix.
    pub fn retry(&self, job: &str) {
        let mut g = self.inner.lock();
        if g.job_state(job) != Some(JobState::Failed) { return; }
        if job.starts_with(INDICATOR_PREFIX) {
            if job != FIRST_SYNC { return; }
            g.state.jobs.insert(job.into(), JobRecord::default());
            if !g.state.account_settled { (g.chain, g.chain_ended) = (Chain::default(), false); }
            g.press("slot.partial");
            g.settle();
            self.inner.persist(&g);
            return self.inner.cv.notify_all();
        }
        if job == "school.courses" && !g.state.account_settled {
            (g.chain, g.chain_ended) = (Chain::default(), false);
            g.press("slot.partial");
        }
        // A failed job is out of the order; a paused one comes back with a count of one (`press`).
        g.press(job);
        g.settle();
        self.inner.persist(&g);
        self.inner.cv.notify_all();
    }
    /// Step 10's confirm: stamps `step10_confirmed_at` and enqueues `slot.first_full` **under one
    /// lock**, so no status read or finish ever sees the confirm without its slot job (F2b r2, m-3).
    pub fn confirm_step10(&self, at: &str) {
        let mut g = self.inner.lock();
        g.state.step10_confirmed_at = Some(at.to_string());
        g.push_job("slot.first_full");
        g.settle();
        self.inner.persist(&g);
        self.inner.cv.notify_all();
    }
    /// The student confirmed the school offered at step 3: marks it confirmed on the live state and
    /// wakes the queue, so a `school.apply` that was waiting on it can start (§4.7).
    pub fn confirm_school(&self) {
        let mut g = self.inner.lock();
        confirm_school(&mut g.state);
        g.parked.clear();
        self.inner.persist(&g);
        self.inner.cv.notify_all();
    }
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
    /// The drain's progress for the page (§4.10): the job it is waiting for, `long` for a slot only (one
    /// engine child can run to `CHILD_TIMEOUT`), and when the drain began. `None` before a drain and
    /// once nothing it waits for is running. A slot is named before a lane job, the longer wait.
    pub fn exiting(&self) -> Option<crate::onboarding_state::ExitingNow> {
        let g = self.inner.lock();
        let since = g.drain_began.clone()?;
        let mut running: Vec<&String> = g.state.jobs.iter().filter(|(_, r)| r.state == JobState::Running).map(|(n, _)| n).collect();
        running.sort_by_key(|n| !n.starts_with("slot."));
        let job = running.first()?.to_string();
        Some(crate::onboarding_state::ExitingNow { long: job.starts_with("slot."), job, since })
    }
    /// Waits for each lane's running job to reach its safe point. A running `mail.canary` or
    /// `mail.first_pass` is not waited for: it is set back to `pending` and left to end on its own
    /// (waiting for mail writes nothing, §4.10). Sets the stop itself (idempotent), so no job can
    /// start after a drain that was reached without [`Queue::request_stop`]. Call it off the main thread.
    pub fn drain(&self) {
        self.request_stop();
        let mut g = self.inner.lock();
        g.drain_began.get_or_insert_with(|| knowlu_engine::journal::now_ts(None));
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
    /// F2b: a change to the live state (a skip) under the queue's lock, saved at once, so the queue
    /// stays the only writer of the file. Refused after the freeze.
    pub fn update_state<T>(&self, change: impl FnOnce(&mut OnboardingState) -> T) -> Result<T, String> {
        let mut g = self.inner.lock();
        if g.frozen { return Err("the onboarding file is frozen: a change is refused".into()); }
        let before = g.state.clone();
        let out = change(&mut g.state);
        // A change that never landed is not kept in memory, so a later save cannot write it.
        if let Err(e) = self.inner.try_persist(&g) { g.state = before; return Err(e); }
        Ok(out)
    }
    /// Finish's write, through the queue's own state: sets `finished` and `handover` on the live
    /// state and saves it, so no later save can write an older, unfinished copy over it.
    pub fn finish(&self, handover: Vec<HandoverItem>) -> Result<(), String> {
        let mut g = self.inner.lock();
        if g.frozen { return Err("the onboarding file is frozen: finish is refused".into()); }
        let before = (g.state.finished, std::mem::replace(&mut g.state.handover, handover));
        g.state.finished = true;
        let saved = self.inner.try_persist(&g);
        // A failed save leaves the live state as it was, so a later save cannot write a finish that never landed.
        if saved.is_err() { (g.state.finished, g.state.handover) = before; }
        saved
    }
    /// F8m: a closed backfill batch tells the queue; nothing is started.
    /// It also unparks every parked job and wakes the threads, so a `mail.first_pass` that looked once
    /// and parked on `mail_not_yet` looks again (F8d2; [`MAIL_FIRST_PASS_LOOKS`]).
    pub fn mail_batch_closed(&self) {
        let mut g = self.inner.lock();
        self.inner.batch_closed.store(true, Ordering::SeqCst);
        g.parked.clear();
        self.inner.cv.notify_all();
    }
    /// F8d2: the student skipped step 5's email. A pending or held `mail.setup`, `mail.canary` and
    /// `mail.first_pass` become `skipped` and leave the order; one that is running is marked, and the
    /// run handed over by [`Queue::attach_setup_run`] is stopped (after its current step), so its
    /// `Stopped` (or a watch's `Waiting`) lands `skipped`, not `pending`. A job with no record gets a `skipped` one (a skip before the answer holds). A job that is done or failed is left
    /// alone, and a skipped mail job is never handed over (`handover_for`).
    pub fn skip_mail(&self) {
        let mut g = self.inner.lock();
        let mut run = None;
        for job in MAIL_JOBS {
            match g.job_state(job) {
                Some(JobState::Pending | JobState::Held) => {
                    g.order.retain(|j| j != job);
                    g.parked.remove(job);
                    g.retry_at.remove(job);
                    g.state.jobs.insert(job.to_string(), JobRecord { state: JobState::Skipped, ..Default::default() });
                }
                Some(JobState::Running) => {
                    g.skipping.insert(job.to_string());
                    if job == "mail.setup" { run = g.setup_run.clone(); }
                }
                // No record yet: the skip is made before the step-5 answer. A `skipped` record keeps
                // `enqueue_once` from queueing the job later.
                None => { g.state.jobs.insert(job.to_string(), JobRecord { state: JobState::Skipped, ..Default::default() }); }
                _ => {}
            }
        }
        self.inner.persist(&g);
        self.inner.cv.notify_all();
        drop(g);
        if let Some(run) = run { run.stop(); }
    }
    /// F8d2: true while `job` is running and the student has skipped it. The `mail.setup` glue checks it on
    /// each window poll and once more right before `start_setup`, since a skip that lands while no run is
    /// attached ([`Queue::attach_setup_run`]) has no run to stop. The marker goes when the job ends.
    pub fn mail_skipped(&self, job: &str) -> bool { self.inner.lock().skipping.contains(job) }
    /// F8d2: the `mail.setup` glue hands over the run it started, so [`Queue::skip_mail`] can stop it.
    /// A skip that came first stops the run at once. The queue forgets it when the job ends.
    pub fn attach_setup_run(&self, run: Arc<dyn SetupRun>) {
        let mut g = self.inner.lock();
        let late = g.skipping.contains("mail.setup");
        if !late { g.setup_run = Some(run.clone()); }
        drop(g);
        if late { run.stop(); }
    }
    /// True once a batch has closed under this queue (the rail's `older_mail` line reads it).
    pub fn batch_closed_noted(&self) -> bool { self.inner.batch_closed.load(Ordering::SeqCst) }
}

// ---- the notes gate and the sync-lock retry (F8h) -------------------------------------------------

pub const RETRY_INTERVAL: Duration = Duration::from_millis(250);
pub const RETRY_BUDGET: Duration = Duration::from_secs(30);

/// Retries `try_once` while it refuses with `week::SYNC_RUNNING` (and only that), every
/// [`RETRY_INTERVAL`] up to [`RETRY_BUDGET`]; holds nothing itself. For `registrar.confirm`, whose
/// call takes `vault_io` and the lock itself.
pub fn retry_on_sync_running<T>(
    sleep: &dyn Fn(Duration),
    try_once: &mut dyn FnMut() -> Result<T, String>,
) -> Result<T, String> {
    let mut waited = Duration::ZERO;
    loop {
        match try_once() {
            // Compared on the constant itself, never a copy of its words (T0 review M2).
            Err(e) if e == crate::week::SYNC_RUNNING && waited < RETRY_BUDGET => {
                sleep(RETRY_INTERVAL);
                waited += RETRY_INTERVAL;
            }
            other => return other,
        }
    }
}

/// The same retry for a lane job that takes the two holds itself around an engine child: each try
/// takes `vault_io` then `take_lock`, and on `SYNC_RUNNING` drops both before it sleeps.
pub fn retry_with_hold<T>(
    vault_io: &Mutex<()>,
    take_lock: &dyn Fn() -> Result<Box<dyn std::any::Any>, String>,
    sleep: &dyn Fn(Duration),
    work: &mut dyn FnMut() -> Result<T, String>,
) -> Result<T, String> {
    retry_on_sync_running(sleep, &mut || {
        // Both holds live only inside one try, in the gate order (`vault_io`, then the lock).
        let _io = vault_io.lock().unwrap_or_else(|e| e.into_inner());
        let _lock = take_lock()?;
        work()
    })
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
    data_dir: &Path,
    kind: &str,
    pending: &dyn PendingStore,
    account: &dyn FeedAccount,
    _windows: &dyn SchoolWindows,
) -> Outcome {
    use crate::onboarding_state::{pending_name, FeedState};
    let answers = match load_answers(data_dir) {
        Ok(Some(a)) => a,
        Ok(None) => return Outcome::Failed("Knowlu has no saved answers to store your calendar link from. Try again.".into()),
        Err(e) => return Outcome::Failed(e),
    };
    let Some(name) = pending_name(kind) else { return Outcome::Failed(format!("{kind} is not a calendar this app stores")) };
    let set = |state: FeedState| update_answers(data_dir, &|a| { a.feeds.insert(kind.to_string(), state); });
    match answers.feeds.get(kind) {
        // Nothing to send: no feed, a rejected link (kept on this computer, never sent) or one already stored.
        None | Some(FeedState::Rejected) => return Outcome::Done(BTreeMap::new()),
        Some(FeedState::Stored) => { let _ = pending.delete(name); return Outcome::Done(BTreeMap::new()); }
        Some(FeedState::Validated | FeedState::Failed) => {}
    }
    let kept = |why: &str| -> Outcome {
        match set(FeedState::Failed) { Ok(()) => Outcome::Failed(why.to_string()), Err(e) => Outcome::Failed(e) }
    };
    // The link comes from the pending credential alone, so a quit after payment needs no school window (§4.11).
    let Some(link) = pending.get(name).filter(|l| !l.trim().is_empty()) else {
        // A retry of a `failed` store whose link `config.ingest` already wrote into ingest.yaml (and so deleted)
        // has nothing to send; a link truly lost is named by `config.ingest`, which checks before it writes.
        if answers.feeds.get(kind) == Some(&FeedState::Failed) { return Outcome::Done(BTreeMap::new()); }
        return kept(if kind == "lms_ics" { REASON_LMS_LINK_MISSING } else { REASON_CALENDAR_LINK_MISSING });
    };
    // One retry, as `create_vault_in` does. The service's words are never kept: they can carry the link.
    if account.put_source(kind, &link).or_else(|_| account.put_source(kind, &link)).is_err() { return kept(REASON_FEED_KEPT_ON_DEVICE); }
    if let Err(e) = set(FeedState::Stored) { return Outcome::Failed(e); }
    let _ = pending.delete(name); // a link that stays is deleted again by `settle_pending_after_ingest`
    Outcome::Done(BTreeMap::new())
}

/// `school.store` / `calendar.store` over the real account: the vault's own session, as `attach_in`
/// stores with the profile's target (`account.rs:1031`), never the wizard's pending one (review r8, M1).
pub struct VaultFeedAccount { pub vault: std::path::PathBuf }
impl FeedAccount for VaultFeedAccount {
    fn put_source(&self, kind: &str, link: &str) -> Result<(), String> { store_feed_in_account(&self.vault, kind, link) }
}

fn store_feed_in_account(vault: &Path, kind: &str, link: &str) -> Result<(), String> {
    let cfg = crate::account::cloud_config(vault)?;
    crate::lms_link::store_source(&cfg.api_base, &cfg.session_credential_target, kind, link)
}

/// A feed the account could not take stays on this computer; the page says so (a closed key).
pub const REASON_FEED_KEPT_ON_DEVICE: &str = "feed_kept_on_device";

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
/// typed rows with no code are two courses unless they would share a note: **one note per slug**
/// (today's seed rule, `seeded_courses`), so a later row whose slug equals an earlier row's reuses
/// that note on the first run, with no create and no error, and both rows record the shared slug.
/// An absent or unreadable answers file is an error (`Outcome::Failed`), never "no courses".
pub fn run_school_courses(
    data_dir: &Path,
    existing: &[ExistingNote],
    create: &mut dyn FnMut(&CourseAnswer) -> Result<String, String>,
) -> Outcome {
    let answers = match load_answers(data_dir) {
        Ok(Some(a)) => a,
        Ok(None) => return Outcome::Failed("Knowlu has no saved course list to make notes from. Try again.".into()),
        Err(e) => return Outcome::Failed(e),
    };
    let nonempty = |t: &str| (!t.trim().is_empty()).then(|| t.trim().to_string());
    let (mut by_code, mut by_slug, mut slugs) = (BTreeMap::<String, String>::new(), BTreeMap::<String, String>::new(), BTreeMap::<String, String>::new());
    for c in answers.kept_courses() {
        let code = nonempty(&c.code);
        let (lms, derived) = (nonempty(&c.lms_key), crate::scaffold::course_note_slug(c));
        // A restored note matches by `source_uid`, then `code` (the LMS key, or a typed human code), then its path.
        let restored = lms.as_deref().and_then(|k| existing.iter().find(|n| n.source_uid.trim() == k))
            .or_else(|| existing.iter().find(|n| lms.as_deref().is_some_and(|k| n.code.trim() == k) || code.as_deref().is_some_and(|k| n.code.trim() == k)))
            .or_else(|| existing.iter().find(|n| n.slug == derived));
        let earlier = code.as_ref().and_then(|k| by_code.get(k)).or_else(|| by_slug.get(&derived));
        let slug = match (earlier, restored) {
            (Some(earlier), _) => earlier.clone(),
            (None, Some(note)) => note.slug.clone(),
            (None, None) => match create(c) { Ok(s) => s, Err(e) => return Outcome::Failed(e) },
        };
        by_slug.insert(derived, slug.clone());
        if let Some(k) = code { by_code.insert(k, slug.clone()); }
        slugs.insert(c.key(), slug);
    }
    match update_answers(data_dir, &|a| a.slugs.extend(slugs.clone())) {
        Ok(()) => Outcome::Done(BTreeMap::new()),
        Err(e) => Outcome::Failed(e),
    }
}

/// What `config.ingest` hands F6b's builder, from the answers file alone. Serialised, it is the
/// text the byte-equality of test 12 compares.
#[derive(Clone, PartialEq, Eq, Serialize)]
pub struct IngestHandoff {
    /// `''` for a stored feed; the link itself for a `failed` or `rejected` one.
    pub ics_url: String,
    pub feed_kept_on_device: bool,
    pub calendar_url: String,
    pub calendar_kept_on_device: bool,
    /// Fragment to slug, over the **kept** courses only: each kept course's saved `fragments` (its
    /// code when none are saved) paired with its slug (a restored note's where one matched).
    /// A course with no recorded slug contributes no pair (`config.ingest` refuses before that).
    pub course_map: Vec<(String, String)>,
    /// The removed courses' fragments (§14.45) that **no kept course also contributes** (§14.46.1), in
    /// no `course_map` entry; empty when none was removed (and then F6b writes no `course_exclude` key).
    pub course_exclude: Vec<String>,
    pub google_calendar: bool,
    /// Step 8's confirmed rows, `(source, key, course slug)` in the order they were saved. Step 8 never
    /// saves a row with a key and a blank slug; `config.ingest` drops one rather than write a mapping the
    /// engine would skip.
    pub books: Vec<(String, String, String)>,
    pub zybooks_ignore: Vec<String>,
}

/// `Debug` never prints a feed link (a log line or a failed assertion must not carry one).
impl std::fmt::Debug for IngestHandoff {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let shown = |link: &str| if link.is_empty() { "" } else { "<redacted>" }.to_string();
        f.debug_struct("IngestHandoff")
            .field("ics_url", &shown(&self.ics_url)).field("feed_kept_on_device", &self.feed_kept_on_device)
            .field("calendar_url", &shown(&self.calendar_url)).field("calendar_kept_on_device", &self.calendar_kept_on_device)
            .field("course_map", &self.course_map).field("course_exclude", &self.course_exclude)
            .field("google_calendar", &self.google_calendar).field("books", &self.books).field("zybooks_ignore", &self.zybooks_ignore)
            .finish()
    }
}

/// A course's pairs for `course_map` / `course_exclude`: its saved `fragments` (code fragments, then
/// uid pins), else its human code alone, and nothing for a row with neither (§14.45, §14.46).
fn course_pairs(c: &CourseAnswer) -> Vec<String> {
    let saved: Vec<String> = c.fragments.iter().filter(|f| !f.trim().is_empty()).cloned().collect();
    if !saved.is_empty() { saved } else { Some(c.code.trim().to_string()).filter(|t| !t.is_empty()).into_iter().collect() }
}

/// The hand-over `config.ingest` builds from: the answers file and the pending links only. A link is
/// read from `pending` for a feed that is `failed` or `rejected` and for no other state (a `validated`
/// feed has not settled: F8c holds `config.ingest` until it has), and is never copied anywhere else.
pub fn ingest_handoff(answers: &Answers, pending: &dyn PendingStore) -> IngestHandoff {
    use crate::onboarding_state::{FeedState, PENDING_CALENDAR, PENDING_LMS};
    let kept_link = |feed: &str, name: &str| -> Option<String> {
        matches!(answers.feeds.get(feed), Some(FeedState::Failed | FeedState::Rejected))
            .then(|| pending.get(name)).flatten().filter(|l| !l.trim().is_empty())
    };
    let (lms, calendar) = (kept_link("lms_ics", PENDING_LMS), kept_link("calendar_ics", PENDING_CALENDAR));
    let mut course_map = Vec::new();
    let mut kept_fragments = BTreeSet::new();
    for c in answers.kept_courses() {
        let slug = answers.slugs.get(&c.key()).map(|s| s.trim().to_string()).filter(|s| !s.is_empty());
        for f in course_pairs(c) {
            kept_fragments.insert(f.clone());
            if let Some(s) = &slug { course_map.push((f, s.clone())); }
        }
    }
    // A fragment a kept course also contributes (a lecture and its lab share `CS 100`) is never excluded.
    let mut course_exclude: Vec<String> = Vec::new();
    for c in answers.courses.iter().filter(|c| c.removed) {
        for f in course_pairs(c) {
            if !kept_fragments.contains(&f) && !course_exclude.contains(&f) { course_exclude.push(f); }
        }
    }
    IngestHandoff {
        feed_kept_on_device: lms.is_some(),
        calendar_kept_on_device: calendar.is_some(),
        ics_url: lms.unwrap_or_default(),
        calendar_url: calendar.unwrap_or_default(),
        course_map,
        course_exclude,
        google_calendar: answers.google_calendar == Some(true),
        books: answers.rows.iter().map(|r| (r.source.clone(), r.key.clone(), r.course_slug.clone())).collect(),
        zybooks_ignore: answers.zybooks_ignore.clone(),
    }
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
pub fn registrar_confirm_rows(answers: &Answers) -> Vec<crate::onboarding_state::RegistrarRow> {
    answers.registrar_rows.clone()
}

/// After `config.ingest` has written the file: deletes both pending credentials. Returns how many
/// could not be deleted (a link that stays is reported, never dropped silently).
pub fn settle_pending_after_ingest(pending: &dyn PendingStore) -> usize {
    [crate::onboarding_state::PENDING_LMS, crate::onboarding_state::PENDING_CALENDAR].iter().filter(|n| pending.delete(n).is_err()).count()
}

/// Back to another school before payment: deletes the pending links, the feed states and the
/// school-step jobs' results, and clears `school` (§4.11).
pub fn change_school(data_dir: &Path, pending: &dyn PendingStore, state: &mut OnboardingState) -> Result<(), String> {
    for name in [crate::onboarding_state::PENDING_LMS, crate::onboarding_state::PENDING_CALENDAR] { pending.delete(name)?; }
    // An absent answers file has no feed states to clear, and none is made; an unreadable one is an error.
    if load_answers(data_dir)?.is_some() { update_answers(data_dir, &|a| a.feeds.clear())?; }
    for job in ["school.capture", "school.preview", "school.store"] { state.jobs.remove(job); }
    state.school = None;
    Ok(())
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

/// `Done`'s proof key: how many files `school.apply` kept because they held other bytes ("kept your
/// existing settings"). Absent or `Some(0)` when nothing was kept.
pub const KEPT_EXISTING_KEY: &str = "kept_existing_settings";

/// `school.apply`: creates `config/campus.yaml` and `config/events.yaml` once (F6a), each through
/// [`crate::scaffold::put_new_file`]. The events preset is `events_preset_for(choice.unitid)`.
pub fn run_school_apply(vault: &Path, choice: &crate::scaffold::CampusChoice) -> Outcome {
    run_school_apply_with(vault, choice, &Default::default())
}

pub fn run_school_apply_with(vault: &Path, choice: &crate::scaffold::CampusChoice, seams: &crate::scaffold::PutSeams) -> Outcome {
    use crate::scaffold::{campus_config_yaml, campus_yaml, events_preset_for, put_new_file_with, Put};
    let preset = events_preset_for(&choice.unitid);
    let files = [
        ("config/events.yaml", campus_yaml(preset).map(str::to_string).ok_or_else(|| format!("unknown campus preset {preset:?}"))),
        ("config/campus.yaml", campus_config_yaml(choice)),
    ];
    let volume = volume_of(vault);
    let mut kept = 0;
    for (rel, text) in files {
        // The bytes `build_into`'s `write_file` writes: the platform's line ending.
        let text = text.map(|t| knowlu_engine::pystr::universal_newlines(&t).replace('\n', knowlu_engine::pystr::NEWLINE));
        match text.and_then(|t| put_new_file_with(vault, rel, &t, seams)) {
            Ok(Put::Existing { same_bytes: false }) => kept += 1,
            Ok(_) => {}
            Err(e) => return Outcome::Failed(format!(
                "Knowlu could not save your school settings on {volume} ({e}). Check that {volume} is a normal drive with free space, then Try again.")),
        }
    }
    Outcome::Done(if kept > 0 { BTreeMap::from([(KEPT_EXISTING_KEY.to_string(), Some(kept))]) } else { BTreeMap::new() })
}

/// `school.courses` against a real vault: finds the notes already in `<vault>/courses/` (a C3' restore),
/// then runs [`run_school_courses`] with [`crate::scaffold::create_course_note`] as `create`.
pub fn run_school_courses_in(data_dir: &Path, vault: &Path) -> Outcome {
    let mut existing = Vec::new();
    for e in std::fs::read_dir(vault.join("courses")).into_iter().flatten().flatten() {
        let path = e.path();
        let Some(slug) = path.file_name().and_then(|n| n.to_str()).and_then(|n| n.strip_suffix(".md")).map(str::to_string) else { continue };
        let front = knowlu_engine::pystr::read_text(&path).ok().and_then(|t| knowlu_engine::models::split_frontmatter(&t).ok()).map(|(m, _)| m);
        let field = |k: &str| front.as_ref().and_then(|m| knowlu_engine::yaml::get(m, k)).and_then(knowlu_engine::yaml::text).unwrap_or_default();
        existing.push(ExistingNote { slug, source_uid: field("source_uid"), code: field("code") });
    }
    existing.sort_by(|a, b| a.slug.cmp(&b.slug));
    run_school_courses(data_dir, &existing, &mut |c| crate::scaffold::create_course_note(vault, c))
}

/// What `config.ingest` needs besides the answers file and the pending links (F6b): the vault's
/// profile id (the coursework credential targets), its time zone, and which homework logins the
/// student stored at step 8 (`zybooks`/`vhl` switch each `coursework:` block on).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct IngestBase {
    pub profile_id: String,
    pub timezone: String,
    pub zybooks: bool,
    pub vhl: bool,
}

/// `config.ingest`: builds `ingest.yaml` from the answers file ([`ingest_handoff`]), then
/// [`put_ingest_text`]. An absent or unreadable answers file is `Failed`, never an empty config.
pub fn run_config_ingest(vault: &Path, data_dir: &Path, base: &IngestBase, pending: &dyn PendingStore) -> Outcome {
    run_config_ingest_with(vault, data_dir, base, pending, &Default::default())
}

pub fn run_config_ingest_with(
    vault: &Path,
    data_dir: &Path,
    base: &IngestBase,
    pending: &dyn PendingStore,
    seams: &crate::scaffold::PutSeams,
) -> Outcome {
    use crate::scaffold::{ingest_yaml_for, patch_capability_url, BookMapping, CampusChoice, SectionMapping, VaultPlan};
    let answers = match load_answers(data_dir) {
        Ok(Some(a)) => a,
        Ok(None) => return Outcome::Failed("Knowlu has no saved answers to build your course settings from. Try again.".into()),
        Err(e) => return Outcome::Failed(e),
    };
    // Defensive: the queue runs this after `school.courses` is done, which records every kept slug.
    if answers.kept_courses().iter().any(|c| answers.slugs.get(&c.key()).is_none_or(|s| s.trim().is_empty())) {
        return Outcome::Failed(REASON_COURSES_NOT_RECORDED.into());
    }
    // A feed still `validated` has not settled (F8c holds the job for it too); the write cannot be undone.
    if ["lms_ics", "calendar_ics"].iter().any(|k| answers.feeds.get(*k) == Some(&crate::onboarding_state::FeedState::Validated)) {
        return Outcome::Waiting(REASON_FEEDS_NOT_SETTLED.into());
    }
    let h = ingest_handoff(&answers, pending);
    // A feed the account did not take must have its link in the pending credential: without it the link
    // would be lost for good once the file is written, so nothing is created and the credential is left alone.
    for (feed, kept, key) in [("lms_ics", h.feed_kept_on_device, REASON_LMS_LINK_MISSING), ("calendar_ics", h.calendar_kept_on_device, REASON_CALENDAR_LINK_MISSING)] {
        let unsettled = matches!(answers.feeds.get(feed), Some(crate::onboarding_state::FeedState::Failed | crate::onboarding_state::FeedState::Rejected));
        if unsettled && !kept { return Outcome::Failed(key.into()); }
    }
    // One mapping per book or section, first wins (a duplicate key is a file the engine refuses whole).
    let (mut zybooks_courses, mut vhl_sections) = (Vec::<BookMapping>::new(), Vec::<SectionMapping>::new());
    for (source, key, slug) in h.books.iter().filter(|(_, k, s)| !k.trim().is_empty() && !s.trim().is_empty()) {
        match source.as_str() {
            "zybooks" if !zybooks_courses.iter().any(|b| &b.code == key) => zybooks_courses.push(BookMapping { code: key.clone(), course: slug.clone(), label: String::new() }),
            "vhl" if !vhl_sections.iter().any(|s| &s.section == key) => vhl_sections.push(SectionMapping { section: key.clone(), course: slug.clone(), label: String::new() }),
            _ => {}
        }
    }
    // Only the fields `ingest_yaml_for` reads are meaningful; the rest are the vault's other files'.
    let plan = VaultPlan {
        profile_id: base.profile_id.clone(), ics_url: None,
        personal_calendar: answers.feeds.contains_key("calendar_ics").then(String::new),
        google_calendar: h.google_calendar, timezone: base.timezone.clone(), slots: Vec::new(), device: String::new(),
        campus: String::new(), campus_choice: CampusChoice::default(), zybooks: base.zybooks, vhl: base.vhl,
        zybooks_courses, vhl_sections, zybooks_ignore: h.zybooks_ignore.clone(), course_map: h.course_map.clone(), courses: Vec::new(),
        api_base: String::new(), anon_key: String::new(), account_id: String::new(), human_actor: knowlu_engine::journal::HUMAN_ACTOR,
    };
    // The account holds the feeds (`ics_url: ''`); a feed it did not take gets its link patched in, as `restore_capability_url` does.
    let built = ingest_yaml_for(&plan, true, &h.course_exclude).and_then(|text| {
        let at = "config/ingest.yaml";
        let text = if h.feed_kept_on_device { patch_capability_url(&text, "lms_ics", &h.ics_url, at)? } else { text };
        if h.calendar_kept_on_device { patch_capability_url(&text, "calendar_ics", &h.calendar_url, at) } else { Ok(text) }
    });
    match built {
        Ok(text) => put_ingest_text(vault, data_dir, &text, seams),
        Err(e) => Outcome::Failed(format!("Knowlu could not build your course settings ({e}). Try again.")),
    }
}

/// `config.ingest`'s base from the answers file: which homework logins the student stored comes from
/// step 8's confirmed rows (a row with a key and a slug, as `config.ingest`'s mappings), the profile id and time zone from the vault.
pub fn ingest_base_for(answers: &Answers, profile_id: &str, timezone: &str) -> IngestBase {
    let has = |source: &str| answers.rows.iter().any(|r| r.source == source && !r.key.trim().is_empty() && !r.course_slug.trim().is_empty());
    IngestBase { profile_id: profile_id.to_string(), timezone: timezone.to_string(), zybooks: has("zybooks"), vhl: has("vhl") }
}

/// The lane's `config.ingest`: [`run_config_ingest`] over a base built from the answers file, and
/// then, once the file is written, [`settle_pending_after_ingest`]. A feed still `validated` holds the
/// job (`Waiting`, nothing touched), and a `*_link_missing` failure settles nothing. A file already
/// there with other bytes did not take a kept link, so a `failed` or `rejected` feed's link stays.
/// The proof names, per feed and never with the link: `feed_kept_on_device` / `calendar_kept_on_device`
/// (the link went into ingest.yaml, not the account), `lms_link_not_placed` / `calendar_link_not_placed`
/// (an existing file kept other bytes: the link is in neither file nor account), and
/// `pending_link_not_deleted` (a pending credential that could not be deleted).
pub fn run_config_ingest_in(vault: &Path, data_dir: &Path, profile_id: &str, timezone: &str, pending: &dyn PendingStore) -> Outcome {
    let answers = match load_answers(data_dir) {
        Ok(Some(a)) => a,
        Ok(None) => return Outcome::Failed("Knowlu has no saved answers to build your course settings from. Try again.".into()),
        Err(e) => return Outcome::Failed(e),
    };
    let base = ingest_base_for(&answers, profile_id, timezone);
    let out = run_config_ingest(vault, data_dir, &base, pending);
    let Outcome::Done(mut proof) = out else { return out };
    let h = ingest_handoff(&answers, pending);
    let existing_kept = proof.contains_key(KEPT_EXISTING_KEY);
    let any_kept = h.feed_kept_on_device || h.calendar_kept_on_device;
    for (kept, placed, unplaced) in [(h.feed_kept_on_device, "feed_kept_on_device", "lms_link_not_placed"), (h.calendar_kept_on_device, "calendar_kept_on_device", "calendar_link_not_placed")] {
        if kept { proof.insert((if existing_kept { unplaced } else { placed }).to_string(), Some(1)); }
    }
    if !(existing_kept && any_kept) && settle_pending_after_ingest(pending) > 0 { proof.insert("pending_link_not_deleted".into(), Some(1)); }
    Outcome::Done(proof)
}

/// The reason key of a `config.ingest` that found a kept course with no recorded slug.
pub const REASON_COURSES_NOT_RECORDED: &str = "courses_not_recorded";
/// `Waiting`'s reason while a feed is still `validated`.
pub const REASON_FEEDS_NOT_SETTLED: &str = "feeds_not_settled";
/// A feed that is `failed` or `rejected` whose pending credential is gone: the link cannot be kept.
pub const REASON_LMS_LINK_MISSING: &str = "lms_link_missing";
pub const REASON_CALENDAR_LINK_MISSING: &str = "calendar_link_missing";

/// Deletes the check's scratch root when dropped, so a panic cannot leave the text (it may hold a feed link) on disk.
struct ScratchRoot(std::path::PathBuf);
impl Drop for ScratchRoot {
    fn drop(&mut self) { let _ = std::fs::remove_dir_all(&self.0); }
}

/// The drive a vault is on (`C:`), for the fix a failed save names.
fn volume_of(vault: &Path) -> String {
    match vault.components().next() {
        Some(std::path::Component::Prefix(p)) => p.as_os_str().to_string_lossy().to_string(),
        _ => vault.display().to_string(),
    }
}

/// The parse check, then the create, for `text` as `build_into` builds it (LF line ends; the file
/// gets the platform's, as [`run_school_apply`]'s do). The check writes the text to
/// `<data_dir>/tmp/ingest-check-<pid>/config/ingest.yaml`, runs `coursework::load_coursework_config`
/// on that root (any warning or error is `Failed`, and nothing is created), and deletes the root on
/// every path. Then [`crate::scaffold::put_new_file_with`] for `config/ingest.yaml`, reporting as
/// [`run_school_apply`] does (`KEPT_EXISTING_KEY`; `Failed` with a fix that names the volume).
/// A failed check names how many problems it found and never quotes them: the text can hold a feed link.
pub fn put_ingest_text(vault: &Path, data_dir: &Path, text: &str, seams: &crate::scaffold::PutSeams) -> Outcome {
    use crate::scaffold::{put_new_file_with, Put};
    let text = knowlu_engine::pystr::universal_newlines(text).replace('\n', knowlu_engine::pystr::NEWLINE);
    let guard = ScratchRoot(data_dir.join("tmp").join(format!("ingest-check-{}", std::process::id())));
    let scratch = guard.0.as_path();
    let _ = std::fs::remove_dir_all(scratch);
    let checked = std::fs::create_dir_all(scratch.join("config"))
        .and_then(|()| std::fs::write(scratch.join("config").join("ingest.yaml"), text.as_bytes()))
        .map_err(|e| format!("could not write the check file ({e})"))
        .and_then(|()| match knowlu_engine::coursework::load_coursework_config(scratch) {
            Ok((_, w)) if w.is_empty() => Ok(()),
            Ok((_, w)) => Err(format!("{} problem(s) reading it back", w.len())),
            Err(_) => Err("it could not be read back".to_string()),
        });
    drop(guard);
    if let Err(e) = checked {
        return Outcome::Failed(format!("Knowlu built a course settings file it could not use ({e}), so it saved nothing. Try again."));
    }
    match put_new_file_with(vault, "config/ingest.yaml", &text, seams) {
        Ok(Put::Existing { same_bytes: false }) => Outcome::Done(BTreeMap::from([(KEPT_EXISTING_KEY.to_string(), Some(1))])),
        Ok(_) => Outcome::Done(BTreeMap::new()),
        Err(e) => {
            let volume = volume_of(vault);
            Outcome::Failed(format!("Knowlu could not save your course settings on {volume} ({e}). Check that {volume} is a normal drive with free space, then Try again."))
        }
    }
}

// ---- the mail, Google and coursework runners (F8d) ---------------------------------------------------------
//
// Each is the pure core over a seam; the job-name dispatch that builds the seams from the app is the
// `Runner` impl's. Every one heeds the queue's stop at its safe point and writes nothing once it has seen it.

/// `mail.setup` ended with the user's window closed: a closed-set reason, never the run's own line.
pub const REASON_MAIL_CANCELLED: &str = "mail_setup_cancelled";
/// `mail.setup` ended failed (or needing the student, which a first setup never does).
pub const REASON_MAIL_SETUP_FAILED: &str = "mail_setup_failed";
/// The canary or first pass did not show in its budget: `pending` again, not failed.
pub const REASON_MAIL_NOT_YET: &str = "mail_not_yet";
/// How often the canary and first-pass watches look, and how long each looks (the setup's `CANARY_WAIT`).
pub const MAIL_WATCH_POLL: Duration = Duration::from_secs(5);
pub const MAIL_WATCH_BUDGET: Duration = Duration::from_secs(180);

/// `mail.setup` found the mail window held by another holder (`on_start`'s sweep or resume) for the
/// whole minute it polled: `pending`, re-picked on a timer ([`TIMER_RETRY_AFTER`]), since the window's
/// release tells the queue nothing (§14.52.1, r1 m2).
pub const REASON_MAIL_WINDOW_BUSY: &str = "mail_window_busy";
/// `Waiting` reasons the queue re-picks on a timer: a closed list, today this one.
pub const TIMER_RETRY_REASONS: [&str; 1] = [REASON_MAIL_WINDOW_BUSY];
/// How long a job parked on a [`TIMER_RETRY_REASONS`] reason waits before it is picked again.
pub const TIMER_RETRY_AFTER: Duration = Duration::from_secs(30);
/// How often `mail.first_pass` looks for a closed batch: once. It sits on the serial queue in front of
/// the Google reads and the slots, so it never waits; [`Queue::mail_batch_closed`] wakes it to look again.
pub const MAIL_FIRST_PASS_LOOKS: u32 = 1;
/// The mail jobs a skip of step 5 takes out.
const MAIL_JOBS: [&str; 3] = ["mail.setup", "mail.canary", "mail.first_pass"];
/// The longest an idle thread sleeps while a retry time is pending, so a moved clock is noticed.
const CLOCK_LOOK: Duration = Duration::from_millis(250);

/// The time the retry timer reads. The queue's is [`WallClock`]; a test moves its own by hand.
pub trait RetryClock: Send + Sync {
    fn now(&self) -> Instant;
}
pub struct WallClock;
impl RetryClock for WallClock {
    fn now(&self) -> Instant { Instant::now() }
}

/// A running mail setup as `mail.setup`'s runner holds it: [`crate::mail::setup::Shared`], or a fake.
pub trait SetupRun: Send + Sync {
    fn stop(&self);
    fn wait_done(&self, timeout: Duration) -> bool;
    fn progress(&self) -> crate::mail::setup::Progress;
}

impl SetupRun for crate::mail::setup::Shared {
    fn stop(&self) { crate::mail::setup::Shared::stop(self) }
    fn wait_done(&self, timeout: Duration) -> bool { crate::mail::setup::Shared::wait_done(self, timeout) }
    fn progress(&self) -> crate::mail::setup::Progress { crate::mail::setup::Shared::progress(self) }
}

/// `mail.setup`: passes the queue's stop to the run ("after the current step") and returns only once the
/// run has ended, so no setup write lands after Sign out's clean. A stopped run is `Stopped` (pending,
/// resumed at the next launch). The setup's provider, the service's own string from the run's progress,
/// goes to `mail_provider` through `update_answers`, stop or not (the drain waits for this runner); an empty provider
/// (the run ended before it was resolved) records nothing.
pub fn run_mail_setup(data_dir: &Path, run: &dyn SetupRun, stop: &AtomicBool) -> Outcome {
    let mut told = false;
    loop {
        if stop.load(Ordering::SeqCst) && !told { run.stop(); told = true; }
        if run.wait_done(Duration::from_millis(50)) { break; }
    }
    let (p, stopping) = (run.progress(), stop.load(Ordering::SeqCst));
    if !p.provider.is_empty() {
        // Written even after a stop: the drain waits for this runner, so the write lands before `on_sign_out`
        // and `write_finish`, and finish needs the provider to leave a never-driven first pass out. It is
        // advisory (F2b keeps the item when it is absent), so a refusal never fails a setup.
        let _ = update_answers(data_dir, &|a| a.mail_provider = Some(p.provider.to_string()));
    }
    match p.ended {
        Some("finished") => Outcome::Done(BTreeMap::new()),
        _ if stopping || p.ended == Some("stopped") => Outcome::Stopped,
        Some("cancelled") => Outcome::Failed(REASON_MAIL_CANCELLED.into()),
        _ => Outcome::Failed(REASON_MAIL_SETUP_FAILED.into()),
    }
}

/// Polls `check` every [`MAIL_WATCH_POLL`] for [`MAIL_WATCH_BUDGET`]: `Done` when it holds, `Stopped` at a
/// poll that finds the stop, else `Waiting`. It writes nothing anywhere (the [`ABANDON_AT_ONCE`] rule).
fn run_mail_watch(check: &dyn Fn() -> bool, stop: &AtomicBool, sleep: &dyn Fn(Duration)) -> Outcome {
    for _ in 0..MAIL_WATCH_BUDGET.as_secs() / MAIL_WATCH_POLL.as_secs() {
        if check() { return Outcome::Done(BTreeMap::new()); }
        if stop.load(Ordering::SeqCst) { return Outcome::Stopped; }
        sleep(MAIL_WATCH_POLL);
    }
    Outcome::Waiting(REASON_MAIL_NOT_YET.into())
}

/// `mail.canary`: `back` is the unfiltered address list's canary reading (F8s2's `CanaryVerified`).
pub fn run_mail_canary(back: &dyn Fn() -> bool, stop: &AtomicBool, sleep: &dyn Fn(Duration)) -> Outcome { run_mail_watch(back, stop, sleep) }

/// `mail.first_pass`: `closed` is F8m's `batch_closed_noted`. The backfill itself is never waited for:
/// it looks [`MAIL_FIRST_PASS_LOOKS`] (once) and never sleeps (`_sleep` stays for the watches' shape). With no
/// batch closed it is `Waiting(mail_not_yet)`, parked until [`Queue::mail_batch_closed`] wakes it (F8d2).
pub fn run_mail_first_pass(closed: &dyn Fn() -> bool, stop: &AtomicBool, _sleep: &dyn Fn(Duration)) -> Outcome {
    for _ in 0..MAIL_FIRST_PASS_LOOKS {
        if closed() { return Outcome::Done(BTreeMap::new()); }
        if stop.load(Ordering::SeqCst) { return Outcome::Stopped; }
    }
    Outcome::Waiting(REASON_MAIL_NOT_YET.into())
}

/// What a read hands back: its proof counts, or a named outcome that is not a failure (no runtime, model,
/// account, session, entitlement or network; `coursework` always exits 0).
#[derive(Debug, Clone, PartialEq)]
pub enum Read {
    Counts(BTreeMap<String, Option<i64>>),
    Named(String),
}

/// One call, then (if the stop has not been seen) `keep` writes its result. The stop is read before the
/// call and after it, before any write; a call in hand is waited for, its answer dropped on a stop.
fn run_checked_read(
    stop: &AtomicBool,
    read: &dyn Fn() -> Result<Read, String>,
    keep: &dyn Fn(&BTreeMap<String, Option<i64>>) -> Result<(), String>,
) -> Outcome {
    if stop.load(Ordering::SeqCst) { return Outcome::Stopped; }
    let got = read();
    if stop.load(Ordering::SeqCst) { return Outcome::Stopped; }
    match got {
        Err(e) => Outcome::Failed(e),
        Ok(Read::Named(reason)) => Outcome::Waiting(reason),
        Ok(Read::Counts(proof)) => match keep(&proof) { Ok(()) => Outcome::Done(proof), Err(e) => Outcome::Failed(e) },
    }
}

/// `google.gmail_first_read` (`judge`'s Gmail pull); `google.calendar_read` (through `ingest`);
/// `coursework.discover`. Each checks the stop after its call and before any write.
pub fn run_gmail_first_read(stop: &AtomicBool, read: &dyn Fn() -> Result<Read, String>, keep: &dyn Fn(&BTreeMap<String, Option<i64>>) -> Result<(), String>) -> Outcome {
    run_checked_read(stop, read, keep)
}
pub fn run_calendar_read(stop: &AtomicBool, read: &dyn Fn() -> Result<Read, String>, keep: &dyn Fn(&BTreeMap<String, Option<i64>>) -> Result<(), String>) -> Outcome {
    run_checked_read(stop, read, keep)
}
pub fn run_coursework_discover(stop: &AtomicBool, read: &dyn Fn() -> Result<Read, String>, keep: &dyn Fn(&BTreeMap<String, Option<i64>>) -> Result<(), String>) -> Outcome {
    run_checked_read(stop, read, keep)
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
        // `school.apply` waits for a confirmed school (F8e), so the stop tests confirm one.
        let mut state = OnboardingState::new("2026-10-24T09:00:00Z", 8);
        state.school = Some(crate::onboarding_state::School { unitid: None, label: "Example Test College".into(), path: crate::onboarding_state::SchoolPath::Shared, confirmed: true });
        (Queue::start(deps, state), gates, runner, dir)
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

    #[test]
    fn an_abandoned_canary_returning_after_the_freeze_leaves_the_file_unchanged() {
        let (q, gates, runner, dir) = rig("canary-freeze", vec![]);
        q.enqueue("mail.canary");
        wait_for("the canary", || !runner.ran.lock().unwrap().is_empty());
        q.drain(); // sets the stop itself (M-5)
        assert!(q.stop_requested());
        q.save().unwrap();
        q.freeze();
        let on_disk = std::fs::read(crate::onboarding_state::state_path(&dir)).unwrap();
        gates.open("mail.canary");
        assert!(q.wait_idle(Duration::from_secs(10)));
        assert_eq!(std::fs::read(crate::onboarding_state::state_path(&dir)).unwrap(), on_disk, "the late return wrote nothing");
        assert_eq!(q.snapshot().jobs["mail.canary"].state, JobState::Pending);
    }

    #[test]
    fn a_failed_finish_save_leaves_the_live_state_unfinished() {
        let (q, _gates, _runner, dir) = rig("finish-fail", vec![]);
        q.request_stop();
        q.save().unwrap();
        // The state file becomes a folder, so no rename can land.
        let path = crate::onboarding_state::state_path(&dir);
        std::fs::remove_file(&path).unwrap();
        std::fs::create_dir(&path).unwrap();
        let handover = vec![HandoverItem { item: "slot.first_full".into(), signal: "slot_end".into() }];
        assert!(q.finish(handover).is_err());
        let s = q.snapshot();
        assert!(!s.finished && s.handover.is_empty(), "a finish that never landed is not kept in memory");
        let _ = std::fs::remove_dir(&path);
    }

    #[test]
    fn a_delivered_first_pass_clears_only_itself_and_only_after_the_canary_items_are_gone() {
        // §14.51 r2, R1 withdrew the old "clears all three": a closed batch must not hide an open setup.
        let mut s = OnboardingState::new("2026-10-24T09:00:00Z", 11);
        let item = |i: &str, sig: &str| HandoverItem { item: i.into(), signal: sig.into() };
        s.handover = vec![item("slot.first_full", "slot_end"), item("mail.setup", "mail_json"), item("mail.canary", "mail_json"), item("mail.first_pass", "mail_json")];
        let held = s.handover.clone();
        s.mail_signal(crate::onboarding_state::MailSignal::FirstPassDelivered);
        assert_eq!(s.handover, held, "the setup and canary are still open: nothing clears");
        s.mail_signal(crate::onboarding_state::MailSignal::CanaryVerified);
        s.mail_signal(crate::onboarding_state::MailSignal::FirstPassDelivered);
        assert_eq!(s.handover, vec![item("slot.first_full", "slot_end")]);
    }
}

#[cfg(test)]
mod order_tests {
    //! F8e's own tests (review r1): the gaps F1's rigs do not reach. A fake runner logs what starts
    //! and the scope each slot got; a scripted view plays the sync status back.
    use super::*;
    use crate::onboarding_state::{save_answers, School, SchoolPath};
    use std::collections::VecDeque;
    use std::sync::atomic::AtomicUsize;

    #[derive(Default)]
    struct Fake { log: Mutex<Vec<String>>, gates: Mutex<BTreeMap<String, Arc<AtomicBool>>>, failing: Mutex<BTreeSet<String>>, clock: AtomicUsize }
    impl Fake {
        fn log(&self) -> Vec<String> { self.log.lock().unwrap().clone() }
        fn count(&self, e: &str) -> usize { self.log().iter().filter(|x| *x == e).count() }
        fn scopes(&self) -> Vec<String> { self.log().iter().filter_map(|e| e.strip_prefix("scope:").map(str::to_string)).collect() }
        fn gate(&self, job: &str) -> Arc<AtomicBool> {
            let g = Arc::new(AtomicBool::new(false));
            self.gates.lock().unwrap().insert(job.into(), g.clone());
            g
        }
    }
    impl Runner for Fake {
        fn run(&self, job: &str, scope: Option<SlotScope>, _stop: &AtomicBool) -> Outcome {
            self.log.lock().unwrap().push(format!("start:{job}"));
            if let Some(s) = scope { self.log.lock().unwrap().push(format!("scope:{s:?}")); }
            let gate = self.gates.lock().unwrap().get(job).cloned();
            if let Some(g) = gate {
                let deadline = Instant::now() + Duration::from_secs(20);
                while !g.load(Ordering::SeqCst) { assert!(Instant::now() < deadline, "a gate was never opened"); std::thread::sleep(Duration::from_millis(2)); }
            }
            if self.failing.lock().unwrap().contains(job) { return Outcome::Failed("boom".into()); }
            let n = self.clock.fetch_add(1, Ordering::SeqCst);
            match scope {
                Some(_) => Outcome::Slot(SlotRun { started: format!("2026-10-24T12:{:02}:{:02}.000Z", n / 60, n % 60), steps: Vec::new(), reason: None }),
                None => Outcome::Done(BTreeMap::new()),
            }
        }
    }
    struct Paid;
    impl Entitlement for Paid { fn entitled(&self) -> bool { true } fn refresh(&self) {} }
    type Reading = (Option<SyncStatus>, Option<SyncCursor>);
    /// Plays one reading per slot; a status whose `at` is empty is stamped with the slot's own start.
    struct Syncs(Mutex<VecDeque<Reading>>);
    impl SyncView for Syncs {
        fn read(&self, started: &str) -> Reading {
            match self.0.lock().unwrap().pop_front() {
                Some((Some(mut s), c)) => { if s.at.is_empty() { s.at = started.to_string(); } (Some(s), c) }
                other => other.unwrap_or((None, None)),
            }
        }
    }
    fn ok(end: bool, n: i64) -> Reading {
        (Some(SyncStatus { at: String::new(), ok: true, skipped: None, last_error: None }), Some(SyncCursor { pulled_to_end: end, record_cursor: n, note_cursor: n }))
    }
    fn failed() -> Reading {
        (Some(SyncStatus { at: String::new(), ok: false, skipped: None, last_error: Some("x".into()) }), Some(SyncCursor { pulled_to_end: false, record_cursor: 0, note_cursor: 0 }))
    }
    fn state(settled: bool) -> OnboardingState {
        let mut s = OnboardingState::new("2026-10-24T09:00:00Z", 8);
        s.account_settled = settled;
        s.school = Some(School { unitid: None, label: "Example Test College".into(), path: SchoolPath::Shared, confirmed: true });
        s
    }
    fn rig(tag: &str, s: OnboardingState, readings: Vec<Reading>, answers: bool) -> (Queue, Arc<Fake>, std::path::PathBuf) {
        let dir = std::env::temp_dir().join(format!("knowlu-f8e-{tag}-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        if answers { save_answers(&dir, &Answers::default()).unwrap(); }
        let fake = Arc::new(Fake::default());
        let deps = Deps { data_dir: dir.clone(), runner: fake.clone(), entitlement: Arc::new(Paid), sync_view: Arc::new(Syncs(Mutex::new(readings.into()))) };
        (Queue::start(deps, s), fake, dir)
    }
    fn wait_for(what: &str, cond: impl Fn() -> bool) {
        let deadline = Instant::now() + Duration::from_secs(20);
        while !cond() { assert!(Instant::now() < deadline, "timed out waiting for {what}"); std::thread::sleep(Duration::from_millis(2)); }
    }
    fn job(q: &Queue, name: &str) -> JobRecord { q.snapshot().jobs.get(name).cloned().unwrap_or_default() }
    fn done_rec() -> JobRecord { JobRecord { state: JobState::Done, ..Default::default() } }
    fn idle(q: &Queue) { assert!(q.wait_idle(Duration::from_secs(20)), "the queue never went idle"); }

    #[test]
    fn a_before_config_slot_still_running_when_config_ingest_ends_is_followed_by_the_no_rank_slot() {
        let mut s = state(true);
        s.jobs.insert("school.courses".into(), done_rec());
        let (q, fake, _d) = rig("i1", s, vec![ok(true, 1), ok(true, 2)], true);
        let hold = fake.gate("slot.partial");
        q.enqueue("slot.partial");
        wait_for("the first slot", || fake.count("start:slot.partial") == 1);
        q.enqueue("config.ingest");
        wait_for("config.ingest", || job(&q, "config.ingest").state == JobState::Done);
        q.enqueue("slot.partial"); // the step-8 ask merges into the running slot
        fake.gates.lock().unwrap().remove("slot.partial");
        hold.store(true, Ordering::SeqCst);
        idle(&q);
        assert_eq!(fake.scopes(), ["BeforeConfig", "NoRank"], "{:?}", fake.log());
    }

    #[test]
    fn i1_a_step_8_ask_merged_into_a_running_chain_slot_runs_exactly_one_no_rank_after_config_ingest() {
        let (q, fake, _d) = rig("i1-chain", state(false), vec![ok(false, 5), ok(true, 9)], true);
        let hold = fake.gate("slot.partial");
        q.enqueue("slot.partial");
        q.enqueue("school.courses");
        wait_for("the chain's first slot", || fake.count("start:slot.partial") == 1);
        q.enqueue("config.ingest");
        q.enqueue("slot.partial"); // the step-8 ask merges into the running chain slot
        hold.store(true, Ordering::SeqCst);
        wait_for("config.ingest", || job(&q, "config.ingest").state == JobState::Done);
        idle(&q);
        assert_eq!(fake.scopes(), ["BeforeConfig", "BeforeConfig", "NoRank"], "{:?}", fake.log());
        q.wake();
        idle(&q);
        assert_eq!(fake.scopes().iter().filter(|s| *s == "NoRank").count(), 1, "the debt is paid once: {:?}", fake.log());
    }

    #[test]
    fn r3_try_again_on_a_failed_first_sync_keeps_its_record_and_queues_one_slot_to_re_read_it() {
        let mut s = state(false);
        s.jobs.insert(FIRST_SYNC.into(), JobRecord { state: JobState::Failed, reason: Some(REASON_FIRST_SYNC_FAILED.into()), ..Default::default() });
        s.jobs.insert("first.grades".into(), JobRecord { state: JobState::Failed, reason: Some("x".into()), ..Default::default() });
        let (q, fake, _d) = rig("r3", s, vec![ok(true, 3)], true);
        let hold = fake.gate("slot.partial");
        q.retry(FIRST_SYNC);
        assert!(q.snapshot().jobs.contains_key(FIRST_SYNC), "the record is kept, never deleted");
        assert_eq!(job(&q, FIRST_SYNC), JobRecord::default(), "kept, pending, no reason");
        assert!(q.snapshot().jobs.contains_key("slot.partial"), "the slot that re-reads it is queued");
        wait_for("the slot", || fake.count("start:slot.partial") == 1);
        q.retry(FIRST_SYNC); // pending now: nothing more is queued
        q.retry("first.grades"); // F8g's: never deleted
        hold.store(true, Ordering::SeqCst);
        idle(&q);
        assert_eq!(fake.count("start:slot.partial"), 1, "{:?}", fake.log());
        assert_eq!(job(&q, FIRST_SYNC).state, JobState::Done);
        assert_eq!(job(&q, "first.grades").state, JobState::Failed, "a failed indicator nothing re-reads stays named");
    }

    #[test]
    fn r3_an_ask_merged_into_a_slot_already_running_as_no_rank_owes_nothing() {
        let mut s = state(true);
        for j in ["school.courses", "config.ingest"] { s.jobs.insert(j.into(), done_rec()); }
        let (q, fake, _d) = rig("r3-norank", s, vec![ok(true, 1), ok(true, 2)], true);
        let hold = fake.gate("slot.partial");
        q.enqueue("slot.partial");
        wait_for("the slot", || fake.count("start:slot.partial") == 1);
        q.enqueue("slot.partial");
        hold.store(true, Ordering::SeqCst);
        idle(&q);
        assert_eq!(fake.scopes(), ["NoRank"], "{:?}", fake.log());
    }

    #[test]
    fn c1_school_courses_waits_with_its_reason_until_a_slot_reads_the_account_and_starts_no_chain_itself() {
        let (q, fake, _d) = rig("c1", state(false), vec![ok(true, 2)], true);
        q.enqueue("school.courses");
        idle(&q);
        let c = job(&q, "school.courses");
        assert_eq!((c.state, c.reason.as_deref()), (JobState::Pending, Some(REASON_WAITING_FOR_ACCOUNT)), "{:?}", fake.log());
        assert!(fake.log().is_empty(), "no chain was ended, so none is started: {:?}", fake.log());
        q.enqueue("slot.partial");
        wait_for("school.courses", || job(&q, "school.courses").state == JobState::Done);
        assert!(q.snapshot().account_settled);
        assert_eq!(job(&q, "school.courses").reason, None);
    }

    #[test]
    fn school_courses_enqueued_after_a_chain_ended_starts_a_chain_and_a_failed_slot_ends_the_wait() {
        // A chain ends (two readings that are not a sync) before school.courses is asked for.
        let (q, fake, _d) = rig("i2", state(false), vec![(None, None), (None, None), ok(true, 1)], true);
        q.enqueue("slot.partial");
        idle(&q);
        assert_eq!(fake.scopes().len(), 2, "{:?}", fake.log());
        q.enqueue("school.courses");
        idle(&q);
        assert_eq!(fake.scopes()[2..], ["BeforeConfig"], "{:?}", fake.log());
        assert_eq!(job(&q, "school.courses").state, JobState::Done);
        // The slot itself fails: nothing is left to settle the account, so the job gives up.
        let (q, fake, _d) = rig("i2-fail", state(false), vec![], true);
        fake.failing.lock().unwrap().insert("slot.partial".into());
        q.enqueue("school.courses");
        q.enqueue("slot.partial");
        idle(&q);
        let c = job(&q, "school.courses");
        assert_eq!((c.state, c.reason.as_deref()), (JobState::Failed, Some(REASON_ACCOUNT_UNFINISHED)));
        assert!(!fake.log().contains(&"start:school.courses".to_string()));
    }

    #[test]
    fn a_resumed_school_courses_on_an_unsettled_account_gets_its_slot() {
        let mut s = state(false);
        s.jobs.insert("school.courses".into(), JobRecord::default());
        let (q, fake, _d) = rig("i2-resume", s, vec![ok(true, 1)], true);
        wait_for("school.courses", || job(&q, "school.courses").state == JobState::Done);
        idle(&q);
        assert_eq!(fake.scopes(), ["BeforeConfig"], "{:?}", fake.log());
    }

    #[test]
    fn a_failed_sync_after_the_account_settled_does_not_undo_first_sync() {
        let mut s = state(true);
        s.jobs.insert(FIRST_SYNC.into(), done_rec());
        let (q, _f, _d) = rig("i3", s, vec![failed()], true);
        q.enqueue("slot.partial");
        idle(&q);
        assert_eq!(job(&q, FIRST_SYNC).state, JobState::Done);
    }

    #[test]
    fn the_full_slot_waits_for_school_apply_as_well_as_config_ingest() {
        let mut s = state(true);
        s.school.as_mut().unwrap().confirmed = false;
        s.jobs.insert("config.ingest".into(), done_rec());
        let (q, fake, _d) = rig("i4", s, vec![ok(true, 1)], true);
        q.enqueue("school.apply");
        q.confirm_step10("2026-10-24T12:00:00Z");
        idle(&q);
        assert!(fake.log().is_empty(), "nothing starts while school.apply is unconfirmed: {:?}", fake.log());
        q.confirm_school();
        wait_for("the full slot", || job(&q, "slot.first_full").state == JobState::Done);
        assert_eq!(fake.log()[..2], ["start:school.apply", "start:slot.first_full"], "{:?}", fake.log());
    }

    #[test]
    fn no_config_job_runs_without_a_readable_answers_file_and_both_run_once_it_is_saved() {
        let mut s = state(true);
        s.jobs.insert("school.courses".into(), done_rec()); // registrar.confirm and config.ingest have their input
        s.jobs.insert("school.apply".into(), done_rec()); // and registrar.confirm waits for this too (F8h)
        let (q, fake, dir) = rig("i5", s, vec![], false);
        for j in ["registrar.confirm", "config.ingest"] { q.enqueue(j); }
        idle(&q);
        assert!(fake.log().is_empty(), "{:?}", fake.log());
        std::fs::write(crate::onboarding_state::answers_path(&dir), "{ broken").unwrap();
        q.wake();
        idle(&q);
        assert!(fake.log().is_empty(), "an unreadable file is not no answers: {:?}", fake.log());
        save_answers(&dir, &Answers::default()).unwrap();
        q.wake();
        wait_for("both jobs", || job(&q, "registrar.confirm").state == JobState::Done && job(&q, "config.ingest").state == JobState::Done);
    }

    #[test]
    fn a_school_courses_that_gave_up_runs_when_a_later_slot_settles_the_account() {
        let (q, fake, _d) = rig("m3", state(false), vec![(None, None), (None, None), ok(true, 5)], true);
        q.enqueue("school.courses");
        q.enqueue("slot.partial");
        idle(&q);
        assert_eq!(job(&q, "school.courses").reason.as_deref(), Some(REASON_ACCOUNT_UNFINISHED), "{:?}", fake.log());
        q.enqueue("slot.partial");
        wait_for("school.courses", || job(&q, "school.courses").state == JobState::Done);
        assert!(q.snapshot().account_settled);
    }

    #[test]
    fn retry_takes_failed_jobs_only_and_an_indicator_is_never_a_job() {
        let mut s = state(true);
        s.jobs.insert("school.apply".into(), done_rec());
        let (q, fake, _d) = rig("m1", s, vec![], true);
        q.retry("school.apply");
        q.enqueue(FIRST_SYNC);
        idle(&q);
        assert!(fake.log().is_empty(), "{:?}", fake.log());
        assert!(!q.snapshot().jobs.contains_key(FIRST_SYNC));
        assert_eq!(job(&q, "school.apply").state, JobState::Done);
    }
}
