//! The setup runner's shared surface (email-forwarding plan T11.3; spec §4.1 to §4.5, D6, D15, D16).
//! What a setup is asked ([`Plan`]), what it runs with ([`Env`]), what the progress view reads
//! ([`Progress`]), what a student answers ([`Answer`]) and the one-at-a-time slot ([`Setups`]) the
//! runner's background thread lives in. The flow itself is [`super::runner`]'s and the backfill's pace
//! loop is [`super::backfill`]'s.
//!
//! **Privacy.** A [`Progress`] is fixed words, step names, counts and dates. It never holds a mailbox
//! address, the Knowlu address, a hint, a token or a word a page said: the view asks the service for the
//! address when a guided step shows it (spec §5.4).
use super::host::Host;
use super::recover::Clock;
use super::MailWindow;
use crate::mail_client::{MailClient, MailError};
use crate::mail_session::{forget, MailState, PendingSetup};
use crate::mail_steps::{Reason, State, Step};
use serde::Serialize;
use std::path::{Path, PathBuf};
use std::sync::{Arc, Condvar, Mutex, MutexGuard};
use std::time::{Duration, Instant};

/// How long the runner waits on a student's answer before it gives the setup up (real time).
pub(super) const IDLE: Duration = Duration::from_secs(30 * 60);

/// What is being set up.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Kind {
    /// The school mailbox. `unitid` names the curated campus row (data: its `mail_provider` decides, else
    /// the service's MX lookup does); `address` is the school email address when the student already gave
    /// it, else the setup asks once.
    School { unitid: Option<String>, address: Option<String> },
    /// Personal Gmail, offered beside the school mailbox and skippable.
    Gmail,
    /// Rotate (D3, `mail_rotate`): a new address for a mailbox that is already set up, then the forwarding
    /// step with it. `provider` is the service's own word for the mailbox (`m365`, `google` or `other`).
    /// It signs in again only to reach the page, mints and revokes nothing, and brings in no mail.
    Rotate { address_id: String, provider: String },
}

/// One setup's inputs. The backfill window is fixed from `setup_day` (BW): there is no date to give.
#[derive(Debug, Clone)]
pub struct Plan {
    pub kind: Kind,
    /// This profile's app-data folder: the kept profiles and `mail.json` live in it.
    pub data_dir: PathBuf,
    /// The saved sign-in switch (D15): off means setup runs in `mail-setup`, wiped when the window closes.
    pub keep_session: bool,
    /// The device's own day.
    pub setup_day: jiff::civil::Date,
}

/// What a run uses besides the plan: the window, the service, the clock, and what a closed backfill batch
/// triggers. `guard` is the mail window's one-at-a-time hold, dropped when the run ends.
pub struct Env {
    pub host: Box<dyn Host>,
    pub client: MailClient,
    pub clock: Box<dyn Clock + Send>,
    pub on_closed: Box<dyn Fn() + Send>,
    pub guard: Option<MailWindow>,
}

/// A question the progress view shows, one at a time.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Ask {
    /// An uncurated school: its email address, so the service can decide the provider (§4.1 step 1).
    SchoolAddress,
    /// The mailbox already forwards elsewhere: forward to Knowlu instead, or stop (§4.2 step 3).
    ForwardingElsewhere,
}

impl Ask {
    /// Every question a setup can ask. None is a date: the window is fixed (BW).
    pub const ALL: [Ask; 2] = [Ask::SchoolAddress, Ask::ForwardingElsewhere];

    pub fn name(self) -> &'static str {
        match self {
            Ask::SchoolAddress => "school_address",
            Ask::ForwardingElsewhere => "forwarding_elsewhere",
        }
    }
}

/// A student's answer.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Answer {
    SchoolAddress(String),
    /// `true`: forward to Knowlu instead; `false`: stop.
    Replace(bool),
    /// "I've done this" on a guided step: its check runs.
    Done,
    /// Retry the canary of a blocked mailbox.
    Retry,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum Waiting {
    Ask(Ask),
    /// A guided or failed step is open: "I've done this" or Retry.
    Student,
}

/// One step's row in the progress view.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct Row {
    pub step: &'static str,
    pub state: &'static str,
    pub reason: Option<&'static str>,
    pub guided: bool,
}

/// The backfill's line: N of M, the time left, and why it is waiting.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct Backfill {
    /// The first day read: a fixed 120 days before setup (BW).
    pub since: String,
    pub expected: u32,
    pub handled: u32,
    pub sent: u32,
    pub left: u32,
    pub eta_secs: Option<i64>,
    /// The unix second the backfill goes on (the day's limit, or Microsoft's refusal, ends then).
    pub resumes_at: Option<i64>,
    pub paused: Option<&'static str>,
}

/// What the progress view reads, polled.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize)]
pub struct Progress {
    /// `running`, `asking` or `ended`.
    pub status: &'static str,
    /// `finished`, `stopped`, `cancelled`, `failed` or `needs_you` once it has ended (`needs_you`: a rotate
    /// got its new address and did not finish moving the forwarding to it; the old one stops in 7 days).
    pub ended: Option<&'static str>,
    /// `m365`, `google` or `other` once known.
    pub provider: &'static str,
    /// The mailbox's opaque id on the service (never the address itself): a guided step's view reads the
    /// Knowlu address from `mail_status`'s row with this id (spec §5.4).
    pub address_id: Option<String>,
    pub rows: Vec<Row>,
    /// One fixed sentence of what is happening now.
    pub sentence: &'static str,
    /// The question open now, if any (an [`Ask`] name).
    pub ask: Option<&'static str>,
    /// The setup canary did not come back (D16): the school may be blocking forwarding.
    pub blocked: bool,
    /// What a blocked mailbox can still do: `retry`, `personal_gmail`, `guided_steps`.
    pub offers: Vec<&'static str>,
    pub backfill: Option<Backfill>,
    /// Steps done (by script or by hand) and not, once it has ended.
    pub done: Vec<&'static str>,
    pub left: Vec<&'static str>,
    /// Set when the backfill did not complete: mail before this day (`YYYY-MM-DD`) was not brought in.
    pub not_brought_in_before: Option<String>,
    /// The ended setup's one line (`setup finished`, `setup stopped`, `setup cancelled`, or a named failure).
    pub line: Option<&'static str>,
}

pub(super) fn step_name(s: Step) -> &'static str {
    match s {
        Step::SignIn => "sign_in",
        Step::StepAside => "step_aside",
        Step::Forwarding => "forwarding",
        Step::AddAddress => "add_address",
        Step::ConfirmAddress => "confirm_address",
        Step::TurnOn => "turn_on",
        Step::Canary => "canary",
        Step::Backfill => "backfill",
        Step::Finish => "finish",
    }
}

pub(super) fn reason_name(r: Reason) -> &'static str {
    match r {
        Reason::Guided => "guided",
        Reason::ForwardingElsewhere => "forwarding_elsewhere",
        Reason::NotFound => "not_found",
        Reason::ScriptFailed => "script_failed",
        Reason::Blocked => "blocked",
        Reason::AlreadyConnected => "already_connected",
        Reason::Refused => "refused",
    }
}

pub(super) fn state_name(s: State) -> (&'static str, Option<&'static str>) {
    match s {
        State::Waiting => ("waiting", None),
        State::Working => ("working", None),
        State::Done => ("done", None),
        State::NeedsYou(r) => ("needs_you", Some(reason_name(r))),
        State::DoneByHand => ("done_by_hand", None),
        State::Skipped => ("skipped", None),
        State::Failed(r) => ("failed", Some(reason_name(r))),
    }
}

// ---- the shared state (the runner's thread and the view's polls) ---------------------------------

#[derive(Default)]
struct Inner {
    progress: Progress,
    /// Every question opened, in order (`Ask` names).
    asked: Vec<&'static str>,
    waiting: Option<Waiting>,
    answer: Option<Answer>,
    stop: bool,
    done: bool,
}

/// One run's state: what the view polls, what the student answers, and Stop.
pub struct Shared {
    inner: Mutex<Inner>,
    cv: Condvar,
}

impl Default for Shared {
    fn default() -> Shared {
        Shared::new()
    }
}

impl Shared {
    pub fn new() -> Shared {
        let mut inner = Inner::default();
        inner.progress.status = "running";
        Shared { inner: Mutex::new(inner), cv: Condvar::new() }
    }

    fn lock(&self) -> MutexGuard<'_, Inner> {
        self.inner.lock().unwrap_or_else(|p| p.into_inner())
    }

    pub fn progress(&self) -> Progress {
        self.lock().progress.clone()
    }

    /// The questions asked so far, in order.
    pub fn asked(&self) -> Vec<&'static str> {
        self.lock().asked.clone()
    }

    pub fn is_done(&self) -> bool {
        self.lock().done
    }

    pub fn stopping(&self) -> bool {
        self.lock().stop
    }

    /// Stop after the current step (§4.5).
    pub fn stop(&self) {
        self.lock().stop = true;
        self.cv.notify_all();
    }

    /// A student's answer. Taken only when it fits the question open now (or, for "I've done this" and
    /// Retry, while a guided or failed step is open); `false` otherwise.
    pub fn answer(&self, a: Answer) -> bool {
        let mut g = self.lock();
        let fits = match (g.waiting, &a) {
            (Some(Waiting::Ask(Ask::SchoolAddress)), Answer::SchoolAddress(_)) => true,
            (Some(Waiting::Ask(Ask::ForwardingElsewhere)), Answer::Replace(_)) => true,
            (Some(Waiting::Student), Answer::Done | Answer::Retry) => true,
            _ => false,
        };
        if fits && g.answer.is_none() {
            g.answer = Some(a);
            self.cv.notify_all();
        }
        fits
    }

    /// Waits for the run to end; `false` if it did not within `timeout`.
    pub fn wait_done(&self, timeout: Duration) -> bool {
        let end = Instant::now() + timeout;
        let mut g = self.lock();
        while !g.done {
            let left = end.saturating_duration_since(Instant::now());
            if left.is_zero() {
                return false;
            }
            g = self.cv.wait_timeout(g, left).unwrap_or_else(|p| p.into_inner()).0;
        }
        true
    }

    /// The runner's side: change the progress.
    pub(super) fn update(&self, f: impl FnOnce(&mut Progress)) {
        f(&mut self.lock().progress);
    }

    /// Opens a question (or waits on the student for a guided step) and waits for the answer, Stop, or
    /// `idle` to pass; `None` on the last two.
    pub(super) fn wait_answer(&self, waiting: Waiting, idle: Duration) -> Option<Answer> {
        let end = Instant::now() + idle;
        let mut g = self.lock();
        g.waiting = Some(waiting);
        g.answer = None;
        if let Waiting::Ask(a) = waiting {
            g.asked.push(a.name());
            g.progress.ask = Some(a.name());
            g.progress.status = "asking";
        }
        let got = loop {
            if let Some(a) = g.answer.take() {
                break Some(a);
            }
            let left = end.saturating_duration_since(Instant::now());
            if g.stop || left.is_zero() {
                break None;
            }
            g = self.cv.wait_timeout(g, left).unwrap_or_else(|p| p.into_inner()).0;
        };
        g.waiting = None;
        g.progress.ask = None;
        if !g.done {
            g.progress.status = "running";
        }
        got
    }

    /// The run is over: its closing words, then done.
    pub(super) fn end(&self, ended: &'static str, line: &'static str) {
        let mut g = self.lock();
        g.progress.ended = Some(ended);
        g.progress.line = Some(line);
        g.progress.status = "ended";
        g.progress.ask = None;
    }

    pub(super) fn close(&self) {
        let mut g = self.lock();
        g.done = true;
        g.progress.status = "ended";
        self.cv.notify_all();
    }
}

/// Marks a run done when its thread ends, however it ends.
struct Closer(Arc<Shared>);

impl Drop for Closer {
    fn drop(&mut self) {
        self.0.close();
    }
}

// ---- a setup the app quit mid-run (Quinn, 2026-10-06) ------------------------------------------

/// Writes (or rewrites) an in-flight setup's record in `mail.json`. Best effort: a record that could not be
/// written costs a quit its clean-up, never the setup.
pub(super) fn note_pending(data_dir: &Path, rec: PendingSetup) {
    let _ = MailState::update(data_dir, |s| match s.pending_setups.iter_mut().find(|p| p.address_id == rec.address_id) {
        Some(p) if *p == rec => false,
        Some(p) => {
            *p = rec;
            true
        }
        None => {
            s.pending_setups.push(rec);
            true
        }
    });
}

/// Clears an in-flight setup's record: its run ended, and handled its address itself.
pub(super) fn clear_pending(data_dir: &Path, address_id: &str) {
    let _ = MailState::update(data_dir, |s| {
        let before = s.pending_setups.len();
        s.pending_setups.retain(|p| p.address_id != address_id);
        s.pending_setups.len() != before
    });
}

/// **At start-up**, under the mail window: each setup the app quit mid-run. One whose facts say its address
/// was never put in the mailbox (no script that could set it began, no guide showed it, no "I've done this"
/// or Retry, no backfill began, none open) and whose row on the service still reads `pending` is removed as
/// `withdraw` would have: `DELETE /addresses/<id>`, then its profile through the `is_session_dir` guard. A row
/// Settings shows as set up (any other status) and a placed one are never touched. A row the service no
/// longer lists is gone already: only its profile goes. A list or delete that fails keeps the record for the
/// next start. Returns the ids still recorded.
pub fn clean_unfinished(data_dir: &Path, client: &MailClient) -> Vec<String> {
    let Ok(state) = MailState::load(data_dir) else { return Vec::new() };
    if state.pending_setups.is_empty() {
        return Vec::new();
    }
    let Ok(rows) = client.list_addresses() else {
        return state.pending_setups.into_iter().map(|p| p.address_id).collect();
    };
    for rec in &state.pending_setups {
        let id = rec.address_id.as_str();
        let placed = rec.placed || rec.backfilled || state.open_backfills.iter().any(|o| o == id);
        let status = rows.iter().find(|r| r["address_id"].as_str() == Some(id)).map(|r| r["status"].as_str().unwrap_or(""));
        let gone = match status {
            _ if placed => false,
            None => true,
            Some("pending") => matches!(client.delete_address(id), Ok(_) | Err(MailError::NotFound)),
            Some(_) => false,
        };
        if gone {
            let _ = forget(data_dir, id);
        } else if !placed && status == Some("pending") {
            continue;
        }
        clear_pending(data_dir, id);
    }
    MailState::load(data_dir).map(|s| s.pending_setups.into_iter().map(|p| p.address_id).collect()).unwrap_or_default()
}

// ---- one at a time -----------------------------------------------------------------------------

/// Another setup (or resume) is running: the mail window is one (spec §5.4).
#[derive(Debug, PartialEq, Eq)]
pub struct Busy;

/// The slot a run's background thread lives in. Starting while one runs is refused.
pub struct Setups {
    cur: Mutex<Option<Arc<Shared>>>,
}

impl Default for Setups {
    fn default() -> Setups {
        Setups::new()
    }
}

/// The app's one.
pub static SETUPS: Setups = Setups::new();

impl Setups {
    pub const fn new() -> Setups {
        Setups { cur: Mutex::new(None) }
    }

    /// The run in progress (or the last one, ended), for the view's polls.
    pub fn current(&self) -> Option<Arc<Shared>> {
        self.cur.lock().unwrap_or_else(|p| p.into_inner()).clone()
    }

    fn spawn(&self, run: impl FnOnce(&Shared) + Send + 'static) -> Result<Arc<Shared>, Busy> {
        let mut cur = self.cur.lock().unwrap_or_else(|p| p.into_inner());
        if cur.as_ref().is_some_and(|s| !s.is_done()) {
            return Err(Busy);
        }
        let shared = Arc::new(Shared::new());
        *cur = Some(shared.clone());
        let closer = Closer(shared.clone());
        std::thread::spawn(move || {
            run(&closer.0);
            drop(closer);
        });
        Ok(shared)
    }

    /// Starts a setup on a background thread.
    pub fn start(&self, plan: Plan, env: Env) -> Result<Arc<Shared>, Busy> {
        self.spawn(move |shared| {
            super::runner::run_setup(&plan, &env, shared);
            drop(env);
        })
    }

    /// Resumes the open backfills `ids` name, one after the other, on a background thread (§4.11).
    pub fn start_resume(&self, data_dir: PathBuf, ids: Vec<String>, env: Env) -> Result<Arc<Shared>, Busy> {
        self.spawn(move |shared| {
            super::backfill::run_resume(&data_dir, &ids, &env, shared);
            drop(env);
        })
    }
}
