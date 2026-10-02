//! The mail setup's step machines (email-forwarding plan T10.1, spec §4.2 to §4.5). Pure data and
//! transitions: no window, no network, no log, no clock of its own (every time is passed in), and
//! no text a student wrote. The window and the scripts (T9.2, T10.2 onward) drive it by asking
//! [`Setup::next_action`] and answering with [`Setup::finished`]; the progress view reads
//! [`Setup::state`].
//!
//! **Steps.** Each provider has its list in order ([`Provider::steps`]). Anything but school
//! Microsoft 365 and Gmail is guided only (§4.4): its steps never run as script.
//!
//! **Falling back.** A scripted step that fails becomes `Failed` and guided, for that step only. The
//! steps after it continue scripted unless they need it (the canary needs forwarding on; Gmail's
//! steps run in a chain). The machine hands each step out once: there is no retry here, only the
//! student's own "I've done this" ([`Setup::guided_checked`]).
//!
//! **An existing forwarding address** is a `NeedsYou`, never replaced: the script gets
//! `replace_forwarding` only after [`Setup::answer_forwarding_elsewhere`] says yes.
//!
//! **The backfill window** is a fixed [`BACKFILL_DAYS`] before the setup day for every school (BW).
//! It is fixed when the mailbox's first backfill opens; a resume passes the stored start in and gets
//! it back ([`backfill_window`]).
//!
//! **The pace** ([`Routine`]) is Microsoft's: at most [`MAX_PER_MINUTE`] a minute and
//! [`MAX_PER_DAY`] a mailbox a UTC day (provisional until S6), and the first send failure stops the
//! routine for the rest of that UTC day with a named outcome ([`Paused::line`]).
use jiff::civil::Date;
use knowlu_engine::scheduling::add_days;
use std::collections::VecDeque;

/// The backfill window, in days before the setup day. One constant for every school (BW).
pub const BACKFILL_DAYS: i64 = 120;
/// Exchange Online allows 30 a minute; the routine keeps to a third of it (provisional, S6).
pub const MAX_PER_MINUTE: usize = 10;
/// Forwards per mailbox per UTC day (provisional, S6).
pub const MAX_PER_DAY: u32 = 600;

const DAY_SECS: i64 = 86_400;

/// The first day of the backfill window for a mailbox first set up on `setup_day`.
pub fn backfill_since(setup_day: Date) -> Date {
    add_days(setup_day, -BACKFILL_DAYS)
}

/// The window start to use: the one already stored for the mailbox (`backfill_since` on the
/// service) when there is one, else 120 days before `today`. A resume never recomputes it.
pub fn backfill_window(stored: Option<Date>, today: Date) -> Date {
    stored.unwrap_or_else(|| backfill_since(today))
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Provider {
    M365,
    Gmail,
    /// Every other provider: guided only (§4.4).
    Guided,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum Step {
    SignIn,
    StepAside,
    /// M365: Settings, Mail, Forwarding; set, then read back. Guided: the provider's own setting.
    Forwarding,
    /// Gmail: add the forwarding address.
    AddAddress,
    /// Gmail: Knowlu confirms the address server-side (E3).
    ConfirmAddress,
    /// Gmail: forward a copy of incoming mail to it.
    TurnOn,
    Canary,
    Backfill,
    Finish,
}

use Step::*;

const M365_STEPS: &[Step] = &[SignIn, StepAside, Forwarding, Canary, Backfill, Finish];
const GMAIL_STEPS: &[Step] = &[SignIn, StepAside, AddAddress, ConfirmAddress, TurnOn, Canary, Backfill, Finish];
const GUIDED_STEPS: &[Step] = &[Forwarding, Canary, Backfill];

impl Provider {
    pub fn steps(self) -> &'static [Step] {
        match self {
            Provider::M365 => M365_STEPS,
            Provider::Gmail => GMAIL_STEPS,
            Provider::Guided => GUIDED_STEPS,
        }
    }

    /// The service's `mail_provider` value (`m365`, `google`, `other`); anything else is guided.
    pub fn from_service(name: &str) -> Provider {
        match name {
            "m365" => Provider::M365,
            "google" => Provider::Gmail,
            _ => Provider::Guided,
        }
    }
}

/// The steps a step cannot run without (when they are in the provider's list).
fn requires(step: Step) -> &'static [Step] {
    match step {
        ConfirmAddress => &[AddAddress],
        TurnOn => &[ConfirmAddress],
        Canary => &[Forwarding, AddAddress, ConfirmAddress, TurnOn],
        _ => &[],
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Mode {
    Scripted,
    Guided,
}

/// Why a step needs the student or failed. A name, never a page's text.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Reason {
    /// The step is guided by design (an `other` provider).
    Guided,
    ForwardingElsewhere,
    /// A control the script looks for is not on the page.
    NotFound,
    ScriptFailed,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum State {
    Waiting,
    Working,
    Done,
    NeedsYou(Reason),
    DoneByHand,
    Skipped,
    Failed(Reason),
}

/// What a script (or the window) reports for a step.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Outcome {
    Done,
    /// A missing control: the named `not found`.
    NotFound,
    Failed,
    /// The page already forwards to another address.
    ForwardingElsewhere,
    /// The student closed the window, or sign-in did not complete: setup ends, nothing changed.
    Cancelled,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Action {
    /// Run this scripted step now.
    Run { step: Step, mode: Mode, replace_forwarding: bool },
    /// Show this guided step (the address with Copy, the provider's path, "I've done this").
    Guide { step: Step },
    /// Nothing to run: the student, or a step already running, is next.
    Wait,
    /// Setup has ended: every step is done, done by hand, or skipped, or the student stopped it.
    Finished,
}

/// What a stopped (or finished) setup did, for "Forwarding is on; mail before 12 September was not
/// brought in" (§4.5). The view words it; this holds the facts.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Summary {
    pub done: Vec<Step>,
    pub left: Vec<Step>,
    /// Set when the backfill did not complete: mail before this day was not brought in.
    pub mail_not_brought_in_before: Option<Date>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Setup {
    provider: Provider,
    since: Date,
    steps: Vec<(Step, State, Mode)>,
    replace_ok: Option<Step>,
    stop: bool,
    ended: bool,
}

impl Setup {
    /// A setup on `setup_day`. `stored_since` is the mailbox's `backfill_since` when this is a resume.
    pub fn new(provider: Provider, setup_day: Date, stored_since: Option<Date>) -> Setup {
        let mode = if provider == Provider::Guided { Mode::Guided } else { Mode::Scripted };
        let steps = provider.steps().iter().map(|s| (*s, State::Waiting, mode)).collect();
        Setup { provider, since: backfill_window(stored_since, setup_day), steps, replace_ok: None, stop: false, ended: false }
    }

    pub fn provider(&self) -> Provider {
        self.provider
    }

    pub fn backfill_since(&self) -> Date {
        self.since
    }

    fn at(&self, step: Step) -> Option<usize> {
        self.steps.iter().position(|(s, _, _)| *s == step)
    }

    pub fn state(&self, step: Step) -> State {
        self.at(step).map_or(State::Skipped, |i| self.steps[i].1)
    }

    pub fn mode(&self, step: Step) -> Mode {
        self.at(step).map_or(Mode::Scripted, |i| self.steps[i].2)
    }

    fn set(&mut self, step: Step, state: State) {
        if let Some(i) = self.at(step) {
            self.steps[i].1 = state;
        }
    }

    /// Waiting on a step that failed (directly or through a step it needs), so it cannot run.
    fn held(&self, step: Step) -> bool {
        requires(step).iter().any(|r| match self.state(*r) {
            State::Failed(_) => true,
            State::Waiting => self.held(*r),
            _ => false,
        })
    }

    fn resolved(&self, step: Step) -> bool {
        matches!(self.state(step), State::Done | State::DoneByHand)
    }

    /// An earlier step that keeps later ones waiting: running, asking, or about to run.
    fn blocks(&self, step: Step) -> bool {
        match self.state(step) {
            State::Working | State::NeedsYou(_) => true,
            State::Waiting => !self.held(step),
            _ => false,
        }
    }

    fn ready(&self, index: usize) -> bool {
        let step = self.steps[index].0;
        self.steps[..index].iter().all(|(s, _, _)| !self.blocks(*s))
            && requires(step).iter().filter(|r| self.at(**r).is_some()).all(|r| self.resolved(*r))
    }

    /// The next thing to do. A step is handed out once; asking again after it ran returns the
    /// next one, never the same step (§4.4: nothing is retried in a loop).
    pub fn next_action(&mut self) -> Action {
        if self.ended || (self.stop && !self.steps.iter().any(|(_, s, _)| *s == State::Working)) {
            self.end();
            return Action::Finished;
        }
        if self.stop {
            return Action::Wait;
        }
        for i in 0..self.steps.len() {
            let (step, state, mode) = self.steps[i];
            if state != State::Waiting || self.held(step) {
                continue;
            }
            if !self.ready(i) {
                break;
            }
            if mode == Mode::Guided {
                self.set(step, State::NeedsYou(Reason::Guided));
                return Action::Guide { step };
            }
            return Action::Run { step, mode, replace_forwarding: self.replace_ok == Some(step) };
        }
        let all_settled = self.steps.iter().all(|(_, s, _)| matches!(s, State::Done | State::DoneByHand | State::Skipped));
        if all_settled {
            Action::Finished
        } else {
            Action::Wait
        }
    }

    /// Setup is over: whatever has not run is skipped. Failed steps stay failed, with their guide.
    fn end(&mut self) {
        self.ended = true;
        for (_, state, _) in self.steps.iter_mut() {
            if matches!(state, State::Waiting | State::NeedsYou(_) | State::Working) {
                *state = State::Skipped;
            }
        }
    }

    /// The script for `step` has started.
    pub fn started(&mut self, step: Step) {
        if self.state(step) == State::Waiting {
            self.set(step, State::Working);
        }
    }

    /// The script for `step` reported `outcome`.
    pub fn finished(&mut self, step: Step, outcome: Outcome) {
        if self.state(step) != State::Working {
            return;
        }
        let elsewhere_ok = matches!(step, Forwarding | AddAddress | TurnOn);
        let (state, guided) = match outcome {
            Outcome::Done => (State::Done, false),
            Outcome::ForwardingElsewhere if elsewhere_ok => (State::NeedsYou(Reason::ForwardingElsewhere), false),
            Outcome::NotFound => (State::Failed(Reason::NotFound), true),
            Outcome::Failed | Outcome::ForwardingElsewhere => (State::Failed(Reason::ScriptFailed), true),
            Outcome::Cancelled => {
                self.set(step, State::Skipped);
                self.end();
                return;
            }
        };
        // Sign-in has no guided version: not signed in means setup ends with nothing changed.
        if step == SignIn && state != State::Done {
            self.set(step, State::Skipped);
            self.end();
            return;
        }
        self.set(step, state);
        if guided {
            if let Some(i) = self.at(step) {
                self.steps[i].2 = Mode::Guided;
            }
        }
    }

    /// The student's answer to "this mailbox already forwards elsewhere": `true` forwards to Knowlu
    /// instead (the school inbox keeps every message), `false` stops. The only way a script may
    /// replace an existing forwarding address.
    pub fn answer_forwarding_elsewhere(&mut self, replace: bool) {
        let asking = self.steps.iter().find(|(_, s, _)| *s == State::NeedsYou(Reason::ForwardingElsewhere)).map(|(s, _, _)| *s);
        let Some(step) = asking else { return };
        if replace {
            self.replace_ok = Some(step);
            self.set(step, State::Waiting);
        } else {
            self.set(step, State::Skipped);
            self.request_stop();
        }
    }

    /// "I've done this" ran the step's check (a canary for forwarding, a count for a backfill):
    /// a pass makes it done by hand. A fail leaves it as it was; the machine never retries it.
    pub fn guided_checked(&mut self, step: Step, passed: bool) {
        let open = matches!(self.state(step), State::Failed(_) | State::NeedsYou(Reason::Guided));
        if passed && open {
            self.set(step, State::DoneByHand);
        }
    }

    /// Stop after the current step (§4.5): a running step finishes, nothing later starts.
    pub fn request_stop(&mut self) {
        self.stop = true;
    }

    pub fn summary(&self) -> Summary {
        let done = |s: State| matches!(s, State::Done | State::DoneByHand);
        let backfilled = self.at(Backfill).map_or(true, |_| done(self.state(Backfill)));
        Summary {
            done: self.steps.iter().filter(|(_, s, _)| done(*s)).map(|(st, _, _)| *st).collect(),
            left: self.steps.iter().filter(|(_, s, _)| !done(*s)).map(|(st, _, _)| *st).collect(),
            mail_not_brought_in_before: (!backfilled).then_some(self.since),
        }
    }
}

// ---- The forwarding routine's pace ---------------------------------------------------------------

/// The named outcome of a refused send, for the view, Settings and the run line.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Paused(Provider);

impl Paused {
    pub fn new(provider: Provider) -> Paused {
        Paused(provider)
    }

    pub fn line(self) -> &'static str {
        match self.0 {
            Provider::Gmail => "mail: forwarding paused \u{2014} Gmail refused a send",
            _ => "mail: forwarding paused \u{2014} Microsoft refused a send",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Send {
    Now,
    /// The unix second the next send may go.
    WaitUntil(i64),
    /// 600 are sent for the day; the unix second the next UTC day starts.
    DayLimit { next_day_at: i64 },
    Paused(Paused),
}

/// One mailbox's send pace. Times are unix seconds, the day is the UTC day (`secs / 86400`).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Routine {
    provider: Provider,
    day: i64,
    sent_today: u32,
    recent: VecDeque<i64>,
    paused_on: Option<i64>,
}

impl Routine {
    /// A routine on UTC day `day` that has already sent `sent_today` (a resume carries the count).
    pub fn new(provider: Provider, day: i64, sent_today: u32) -> Routine {
        Routine { provider, day, sent_today, recent: VecDeque::new(), paused_on: None }
    }

    fn roll(&mut self, now: i64) {
        let day = now.div_euclid(DAY_SECS);
        if day > self.day {
            self.day = day;
            self.sent_today = 0;
            self.paused_on = None;
        }
    }

    /// Whether a send may go at `now`.
    pub fn next(&mut self, now: i64) -> Send {
        self.roll(now);
        if self.paused_on.is_some() {
            return Send::Paused(Paused(self.provider));
        }
        if self.sent_today >= MAX_PER_DAY {
            return Send::DayLimit { next_day_at: (self.day + 1) * DAY_SECS };
        }
        while self.recent.front().is_some_and(|t| *t <= now - 60) {
            self.recent.pop_front();
        }
        match self.recent.front() {
            Some(oldest) if self.recent.len() >= MAX_PER_MINUTE => Send::WaitUntil(oldest + 60),
            _ => Send::Now,
        }
    }

    /// A forward went out at `now`.
    pub fn sent(&mut self, now: i64) {
        self.roll(now);
        self.sent_today += 1;
        self.recent.push_back(now);
    }

    /// The first non-delivery report or send failure: nothing sends again until the next UTC day.
    pub fn send_failed(&mut self, now: i64) {
        self.roll(now);
        self.paused_on = Some(self.day);
    }

    pub fn sent_today(&self) -> u32 {
        self.sent_today
    }
}
