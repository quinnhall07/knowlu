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
//! steps after it continue scripted unless they need it (the canary needs forwarding on, the backfill
//! the canary; Gmail's steps run in a chain). Finish waits while an earlier step is still to run, so a
//! forwarding set by hand runs its canary and backfill before the window closes. A failed canary (D16,
//! a school that blocks forwarding) ends the scripted backfill: it becomes `Failed(Blocked)` and guided;
//! Finish only closes the window and still runs.
//! The machine hands each step out once: there is no retry here, only
//! the student's own "I've done this" ([`Setup::guided_checked`]).
//!
//! **An existing forwarding address** is a `NeedsYou`, never replaced: the script gets
//! `replace_forwarding` only after [`Setup::answer_forwarding_elsewhere`] says yes.
//!
//! **The backfill window** is a fixed [`BACKFILL_DAYS`] before the setup day for every school (BW).
//! It is fixed when the mailbox's first backfill opens; a resume passes the stored start in and gets
//! it back ([`backfill_window`]).
//!
//! **The pace** ([`Routine`]) is Microsoft's: at most [`MAX_PER_MINUTE`] a minute and
//! [`MAX_PER_DAY`] a mailbox a UTC day (provisional until S6); Gmail's day is [`GMAIL_MAX_PER_DAY`] originals in at
//! most [`GMAIL_MAX_MESSAGES_PER_DAY`] messages (Quinn, 2026-10-19). The first send failure stops the
//! routine for the rest of that UTC day with a named outcome ([`Paused::line`]).
use jiff::civil::Date;
use knowlu_engine::scheduling::add_days;
use std::collections::VecDeque;

/// The backfill window, in days before the setup day. One constant for every school (BW).
pub const BACKFILL_DAYS: i64 = 120;
/// Exchange Online allows 30 a minute; the routine keeps to a third of it. **Provisional, pending
/// S6** (the M365 test-tenant run, never a real school account): S6 blocks this number, not the
/// code (email plan §6, T17 step 6, §9.5), so T17 merged it unconfirmed on 2026-10-03.
pub const MAX_PER_MINUTE: usize = 10;
/// Microsoft 365's forwards per mailbox per UTC day. **Provisional, pending S6**, as [`MAX_PER_MINUTE`]; until S6
/// confirms it, T18's live proof keeps the school backfill under 600. Gmail's is [`GMAIL_MAX_PER_DAY`].
pub const MAX_PER_DAY: u32 = 600;
/// Gmail's originals forwarded per mailbox per UTC day: 1,500 (Quinn, 2026-10-19, "increase the day cap",
/// superseding "Keep 600" for Gmail), so a 1,485-row 120-day backfill fits in about one day. Gmail sends them
/// as at most [`GMAIL_MAX_MESSAGES_PER_DAY`] messages.
pub const GMAIL_MAX_PER_DAY: u32 = 1_500;
/// Gmail messages sent per mailbox in any 24 hours (2026-10-19, with the day cap): one `forward.js` send is one
/// message whatever its number of originals (up to ten). Gmail's consumer limit is about 500 messages a day
/// counting the student's own mail, and passing it stops the account sending for up to 24 hours, so the
/// backfill keeps to 300 and leaves the rest of the day to the student. Gmail's day is a rolling 24 hours, so
/// the 300 are too (review round 2 of e7ae803): a message counts until it is a day old, never only to UTC midnight
/// (which let a capped backfill send 600 in about 12 hours).
pub const GMAIL_MAX_MESSAGES_PER_DAY: u32 = 300;
/// A backfill the [`GMAIL_MAX_MESSAGES_PER_DAY`] hold resumes once this many messages have freed, never at the
/// first (review of 5f71623..6a8e738, #1): one freed message paid a pass's fresh Inbox, search and walk for about
/// one send, some 60 passes a day. Thirty is a slot's worth ([`crate::mail::recover::SLOT_CAP`]).
pub const GMAIL_MESSAGES_TO_RESUME: u32 = 30;
/// Gmail page loads per mailbox per UTC day: every open of a place (a fresh Inbox is two, the navigation and
/// the reload), every search, every pager press and every conversation opened (the Gmail backfill's page-press
/// budget, design of 2026-10-19). 600 since the day-cap ruling of 2026-10-19 (300 before it): a 1,500-original
/// day makes about twice the looks and restarts of a 600 one. A backstop: the routine plans well inside it, and
/// a pass that reaches it stops until the next day.
pub const MAX_PAGE_LOADS_PER_DAY: u32 = 600;

/// The originals a mailbox of `provider` may forward in a UTC day.
pub fn max_per_day(provider: Provider) -> u32 {
    match provider {
        Provider::Gmail => GMAIL_MAX_PER_DAY,
        _ => MAX_PER_DAY,
    }
}

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

/// **Temporary.** Whether `provider`'s backfill and recover are held while C3 is built: Microsoft 365's
/// retired DOM routine (`m365/search.js`, `list.js`, `forward.js`, `sent_cleanup.js`) forwards mail from the
/// student's mailbox and deletes Sent copies, and C3 ("the mailbox is never sent from", ruled 2026-10-13)
/// replaces it with read-and-upload. D13 (ruled 2026-10-11): the M365 backfill step is shown as the guided
/// step meanwhile, and nothing else (a setup, a launch's resume, a slot's recover, a Retry) drives that
/// routine. Redesign note `docs/specs/2026-10-10-m365-backfill-redesign.md` §4.10: "removed when C3 ships".
/// Gmail is unchanged.
pub fn backfill_held_for_c3(provider: Provider) -> bool {
    provider == Provider::M365
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
        // Quinn, 2026-10-06 (D16): a backfill while forwarding or the canary failed brings in old mail while
        // new mail never arrives. It waits for the canary, done by script or by hand.
        Backfill => &[Canary],
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
    /// The canary did not arrive: the school may be blocking forwarding (D16). Names the steps that
    /// cannot go on as script.
    Blocked,
    /// Sign-in: another of the account's mailboxes already holds this mailbox's address (T18c).
    AlreadyConnected,
    /// Sign-in: the service refused this mailbox's address for another reason.
    Refused,
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
    /// The service refused the mailbox's address because another of the account's mailboxes holds it.
    AlreadyConnected,
    /// The service refused the mailbox's address for any other reason.
    Refused,
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
    /// While C3 is built (D13), a Microsoft 365 setup's Backfill step is the guided step ([`backfill_held_for_c3`]).
    pub fn new(provider: Provider, setup_day: Date, stored_since: Option<Date>) -> Setup {
        let mode = if provider == Provider::Guided { Mode::Guided } else { Mode::Scripted };
        let held = |s: &Step| if *s == Backfill && backfill_held_for_c3(provider) { Mode::Guided } else { mode };
        let steps = provider.steps().iter().map(|s| (*s, State::Waiting, held(s))).collect();
        Setup { provider, since: backfill_window(stored_since, setup_day), steps, replace_ok: None, stop: false, ended: false }
    }

    /// A rotate's machine: it carries no backfill (the runner's Backfill step is a no-op for it), so the D13
    /// hold does not apply and the step stays scripted.
    pub fn new_rotate(provider: Provider, setup_day: Date) -> Setup {
        let mut s = Setup::new(provider, setup_day, None);
        if let (true, Some(i)) = (backfill_held_for_c3(provider), s.at(Backfill)) {
            s.steps[i].2 = Mode::Scripted;
        }
        s
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
        // Finish closes the window (and wipes a throwaway profile): it waits while an earlier step is still to
        // run, held behind a guided one, so a by-hand forwarding's canary and backfill keep the window (D16).
        // A failed step does not hold it.
        if step == Finish && self.steps[..index].iter().any(|(_, s, _)| *s == State::Waiting) {
            return false;
        }
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
            Outcome::AlreadyConnected => (State::Failed(Reason::AlreadyConnected), true),
            Outcome::Refused => (State::Failed(Reason::Refused), true),
            Outcome::Cancelled => {
                self.set(step, State::Skipped);
                self.end();
                return;
            }
        };
        // Sign-in has no guided version: not signed in means setup ends with nothing changed. Its row keeps
        // the failure and its reason (T18c), so a failed sign-in never reads as one that never ran.
        if step == SignIn && state != State::Done {
            self.set(step, state);
            self.end();
            return;
        }
        self.set(step, state);
        if guided {
            if let Some(i) = self.at(step) {
                self.steps[i].2 = Mode::Guided;
            }
        }
        // D16: a canary that fails means the school (or Gmail) is not forwarding, so a scripted
        // backfill would bring in old mail while new mail never arrives. Only the guided step remains.
        if step == Canary && matches!(state, State::Failed(_)) && self.state(Backfill) == State::Waiting {
            self.set(Backfill, State::Failed(Reason::Blocked));
            if let Some(i) = self.at(Backfill) {
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

    /// Retry on a backfill that did not run (the canary was blocked, D16) or did not finish: scripted again
    /// and waiting to run. Only for a scripted provider (never a held one, D13), and only once the canary has arrived. `false`
    /// when there is nothing to re-arm.
    pub fn rearm_backfill(&mut self) -> bool {
        let canary_ok = self.at(Canary).is_none() || self.resolved(Canary);
        let failed = matches!(self.state(Backfill), State::Failed(_));
        if self.provider == Provider::Guided || backfill_held_for_c3(self.provider) || !failed || !canary_ok || self.stop || self.ended {
            return false;
        }
        self.set(Backfill, State::Waiting);
        if let Some(i) = self.at(Backfill) {
            self.steps[i].2 = Mode::Scripted;
        }
        true
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
    /// The day's sends are spent ([`max_per_day`] originals, or Gmail's [`GMAIL_MAX_MESSAGES_PER_DAY`] messages); the unix
    /// second the next send may go: the next UTC day's start for the originals, 24 hours after the oldest of the
    /// last 300 for Gmail's messages, the later when both are spent.
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
    /// In memory only: the instant of the last look at the Inbox that read it (Gmail's non-delivery report
    /// check, `recover.rs`), clean or not, so the next look reads from there and a report is counted once.
    last_look: Option<i64>,
    /// In memory only: the instant of the last forward the mailbox answered ok to (Gmail), so a pass that
    /// reloads the window (`Place::FreshInbox`) waits out Gmail's Undo Send delay after it, even when the send
    /// was the previous pass's.
    last_forward: Option<i64>,
    /// Page loads made on `day` ([`MAX_PAGE_LOADS_PER_DAY`]); a resume carries it (`mail.json`).
    loads_today: u32,
    /// Gmail messages sent (or reserved for a send under way) on `day`; a resume carries it (`mail.json`).
    messages_today: u32,
    /// The instants of Gmail messages sent (or reserved) in the last 24 hours, oldest first: the
    /// [`GMAIL_MAX_MESSAGES_PER_DAY`] cap. Never reset at UTC midnight; a resume carries them (`mail.json`).
    message_times: VecDeque<i64>,
}

impl Routine {
    /// A routine on UTC day `day` that has already sent `sent_today` (a resume carries the count).
    pub fn new(provider: Provider, day: i64, sent_today: u32) -> Routine {
        Routine { provider, day, sent_today, recent: VecDeque::new(), paused_on: None, last_look: None, last_forward: None, loads_today: 0, messages_today: 0, message_times: VecDeque::new() }
    }

    fn roll(&mut self, now: i64) {
        let day = now.div_euclid(DAY_SECS);
        if day > self.day {
            self.day = day;
            self.sent_today = 0;
            self.paused_on = None;
            self.loads_today = 0;
            self.messages_today = 0;
        }
    }

    /// Whether a send may go at `now`.
    pub fn next(&mut self, now: i64) -> Send {
        self.roll(now);
        if self.paused_on.is_some() {
            return Send::Paused(Paused(self.provider));
        }
        let originals = (self.sent_today >= max_per_day(self.provider)).then_some((self.day + 1) * DAY_SECS);
        if let Some(next_day_at) = originals.max(self.messages_free_at(now)) {
            return Send::DayLimit { next_day_at };
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

    /// `n` originals taken on UTC day `day` for rows that were left with nothing sent: given back to that day's
    /// count (a later day's is never touched). The minute's pace keeps them: only the day's count is given back.
    pub fn unsent(&mut self, n: u32, day: i64) {
        if day == self.day {
            self.sent_today = self.sent_today.saturating_sub(n);
        }
    }

    /// The first non-delivery report or send failure: nothing sends again until the next UTC day.
    pub fn send_failed(&mut self, now: i64) {
        self.roll(now);
        self.paused_on = Some(self.day);
    }

    pub fn sent_today(&self) -> u32 {
        self.sent_today
    }

    /// The UTC day the count is for.
    pub fn day(&self) -> i64 {
        self.day
    }

    /// Whether a refused send has paused today's sends.
    pub fn is_paused(&self) -> bool {
        self.paused_on.is_some()
    }

    /// A forward was answered ok at `now`.
    pub fn forwarded(&mut self, now: i64) {
        self.last_forward = Some(now);
    }

    /// When the last forward answered ok, if this routine has seen one since the app started.
    pub fn last_forward(&self) -> Option<i64> {
        self.last_forward
    }

    /// A look at the Inbox read it at `now`.
    pub fn looked(&mut self, now: i64) {
        self.last_look = Some(now);
    }

    /// When the last look that read the Inbox was, if this routine has made one since the app started.
    pub fn last_look(&self) -> Option<i64> {
        self.last_look
    }

    /// This routine having already made `loads` page loads on its day: a resume carries the count.
    pub fn with_loads(mut self, loads: u32) -> Routine {
        self.loads_today = loads;
        self
    }

    /// `n` page loads were made at `now`.
    pub fn loaded(&mut self, now: i64, n: u32) {
        self.roll(now);
        self.loads_today = self.loads_today.saturating_add(n);
    }

    /// The page loads made on the routine's day.
    pub fn loads_today(&self) -> u32 {
        self.loads_today
    }

    /// The page loads left at `now` for its UTC day, never below none.
    pub fn loads_left(&mut self, now: i64) -> u32 {
        self.roll(now);
        MAX_PAGE_LOADS_PER_DAY.saturating_sub(self.loads_today)
    }

    /// While a Gmail mailbox's [`GMAIL_MAX_MESSAGES_PER_DAY`] messages of the last 24 hours are spent at `now`, the
    /// unix second the oldest of them is a day old; `None` when one may go (Microsoft 365 counts none).
    pub fn messages_free_at(&mut self, now: i64) -> Option<i64> {
        while self.message_times.front().is_some_and(|t| t + DAY_SECS <= now) {
            self.message_times.pop_front();
        }
        let spent = self.provider == Provider::Gmail && self.message_times.len() >= GMAIL_MAX_MESSAGES_PER_DAY as usize;
        spent.then(|| self.message_times.front().map_or(now, |t| t + DAY_SECS))
    }

    /// While a Gmail mailbox's messages of the last 24 hours are spent at `now`, the unix second
    /// [`GMAIL_MESSAGES_TO_RESUME`] of them will have freed: when the 30th oldest kept is a day old (the oldest, when
    /// fewer are kept); `None` when one may go. A backfill waits for it ([`crate::mail::backfill::drive_backfill`]).
    pub fn messages_resume_at(&mut self, now: i64) -> Option<i64> {
        let first = self.messages_free_at(now)?;
        let over = self.message_times.len().saturating_sub(GMAIL_MAX_MESSAGES_PER_DAY as usize);
        let nth = over + GMAIL_MESSAGES_TO_RESUME as usize - 1;
        Some(self.message_times.get(nth).map_or(first, |t| t + DAY_SECS))
    }

    /// Reserves one Gmail message for a send about to go at `now`; `false` (nothing counted) while the last 24
    /// hours' [`GMAIL_MAX_MESSAGES_PER_DAY`] are spent. A send that answers ok keeps it; one that did not go gives it
    /// back ([`Routine::release_message`]).
    pub fn reserve_message(&mut self, now: i64) -> bool {
        self.roll(now);
        if self.messages_free_at(now).is_some() {
            return false;
        }
        self.messages_today += 1;
        self.message_times.push_back(now);
        true
    }

    /// A send the message was reserved for did not go.
    pub fn release_message(&mut self) {
        self.messages_today = self.messages_today.saturating_sub(1);
        self.message_times.pop_back();
    }

    /// The instants of the Gmail messages sent (or reserved) in the last 24 hours, oldest first.
    pub fn message_times(&self) -> Vec<i64> {
        self.message_times.iter().copied().collect()
    }

    /// This routine with the Gmail messages a resume carries, by instant (`mail.json`): they replace any
    /// [`Routine::with_messages`] put in.
    pub fn with_message_times(mut self, times: &[i64]) -> Routine {
        let mut times = times.to_vec();
        times.sort_unstable();
        self.message_times = times.into();
        self
    }

    /// The messages sent (or reserved) on the routine's day.
    pub fn messages_today(&self) -> u32 {
        self.messages_today
    }

    /// This routine having already sent `n` messages on its day: a resume carries the count. A count kept without
    /// its instants (a `mail.json` written before they were) reads as sent at the day's start, so it holds until
    /// the next UTC day, as it did then.
    pub fn with_messages(mut self, n: u32) -> Routine {
        self.messages_today = n;
        self.message_times = std::iter::repeat(self.day * DAY_SECS).take(n as usize).collect();
        self
    }

    /// This routine with today's sends paused: a resume of a day a refused send already ended.
    pub fn paused(mut self) -> Routine {
        self.paused_on = Some(self.day);
        self
    }
}
