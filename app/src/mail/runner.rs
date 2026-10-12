//! The setup runner (email-forwarding plan T11.3; spec §4.1 to §4.5, D5, D6, D14, D16). It drives T10.1's
//! step machine ([`crate::mail_steps::Setup`]) through the mail window ([`Host`]), T10's compiled-in
//! scripts and T11.1's client, writes the progress rows ([`Shared`]) and takes the student's answers. It
//! runs on a background thread and never writes the vault.
//!
//! **The flow.** Which provider (the curated row's `mail_provider`, else one question and the service's MX
//! lookup; §4.1); mint the mailbox's address; sign in (the window is visible only for this); the window
//! steps aside; the provider's forwarding steps; the setup canary (three minutes, D14; once this run has set
//! Outlook's forwarding, Outlook is given two minutes to begin and a second canary, T18c); the backfill over a
//! fixed 120 days (BW; [`super::backfill`]); finish. A scripted step that cannot go on falls to its guided
//! version and the student's "I've done this" runs its check (§4.4); nothing is retried in a loop, except
//! that a Gmail script that reloads its page is run again once with `reloaded: true`, and sign-in reads a
//! page still drawing its folder pane again for up to [`SIGN_IN_READ`].
//!
//! **A canary that does not come back** (D16) leaves the mailbox blocked: the backfill is not run (old mail
//! would arrive while new mail never does), Retry sends another canary, and what remains is offered:
//! personal Gmail beside a school mailbox, and the guided steps. A retry that arrives marks the backfill
//! open for the next launch (§4.11).
//!
//! **A closed sign-in window is "setup cancelled"**: nothing changed, so the address minted for it is
//! revoked and its profile deleted, unless Settings handed the address out while the run went on (`shown`):
//! then no route deletes it, and the end line says it was kept (2026-10-12). **Stop** ends after the step in
//! hand and names what was done. A setup
//! that ends before its address was ever put in the mailbox (no forwarding script that could change it
//! answered anything but `not found` or `elsewhere`, no guided step ever showed the address with Copy, the
//! student never said a step was done, and no backfill began under it) is undone the same way and says so,
//! so a stalled setup never holds the mailbox's hint against the next one or counts toward D3's three live
//! addresses (T18c). A removal the service refuses is said as such, never as done. A sign-in whose address the service refuses names the
//! refusal (`already_connected` when another of the account's mailboxes holds it), and a refused mint shows
//! the service's own sentence (D3's caps).
//!
//! **A setup the app quit mid-run** (Quinn, 2026-10-06): while a run holds an address it minted, `mail.json`
//! records the id and the facts above (`placed` written before a script that could set it runs), cleared
//! when the run ends; the next start removes one never placed ([`super::setup::clean_unfinished`]).
//!
//! **A rotate asks for its new address late.** The service's rotate retires the old address and cannot be
//! undone, so it is called only once the sign-in has taken, right before the forwarding step. A rotate that
//! ends any way but finished after that is `needs_you` naming the old address's 7-day grace, never
//! `cancelled` or `stopped`; the new address is the mailbox's row on the service, which Settings reads.
//!
//! **Privacy.** No address, hint, URL, selector or page text is logged, kept in [`Progress`] or sent
//! anywhere but where its step says: the typed school address contributes only its domain to the MX lookup
//! (and is the mailbox's own address only for a guided provider, which has no page to read it from).
use super::backfill::{drive_backfill, mark_open, seed_routine, Drive, Drove};
use super::host::Host;
use super::recover::{Clock, Place};
use super::setup::{
    clear_pending, mark_shown, note_pending, state_name, step_name, Answer, Ask, Backfill, Env, Kind, Plan, Progress, Row, Shared, Waiting, IDLE,
};
use super::Refusal;
use crate::mail_client::{MailClient, State as SvcState};
use crate::mail_session::{forget, forget_setup, session_dir, setup_dir, valid_address_id};
use crate::mail_session::{MailState, PendingSetup};
use crate::mail_steps::{Action, Mode, Outcome, Provider, Reason, Setup, State, Step};
use serde_json::{json, Value};
use std::path::PathBuf;

/// How long the sign-in may take (the school's SSO, Duo, the phone tap).
const SIGN_IN_WAIT: i64 = 15 * 60;
/// Once the title reads signed in, how long the page has to draw the mailbox's own address (T18c: Outlook
/// draws its folder pane a few seconds after the title changes).
const SIGN_IN_READ: i64 = 45;
/// D14: the setup canary is waited for up to three minutes.
const CANARY_WAIT: i64 = 3 * 60;
/// Once this run has set Outlook's forwarding, how long Outlook is given to begin forwarding before the first
/// canary (T18c, 2026-10-07: a canary sent the second the Save took never came back; one seven minutes later
/// did). A canary that still does not come back is followed by one more ([`Runner::canary_after_set`]).
const FORWARD_SETTLE: i64 = 2 * 60;
/// What the setup says while it gives Outlook that time.
const SETTLING: &str = "waiting for Outlook to start forwarding";
/// The Gmail address confirm is open for 15 minutes (§4.6 step 5).
const CONFIRM_WAIT: i64 = 15 * 60;
/// Seconds between polls of the service.
const POLL: i64 = 5;
/// Google's popup is waited for this long to appear, and this long to close.
const POPUP_APPEAR: i64 = 30;
const POPUP_WAIT: i64 = 5 * 60;

/// Why a run ended.
#[derive(Debug, Clone, PartialEq, Eq)]
enum End {
    Finished,
    Stopped,
    Cancelled,
    /// A fixed word, or a service error's name.
    Failed(&'static str),
    /// A rotate that got its new address and did not finish moving the mailbox's forwarding to it.
    NeedsYou(&'static str),
    /// Stopped before the Knowlu address was ever put in the mailbox: the address made for it is gone.
    Withdrawn,
    /// As [`End::Withdrawn`], but the service did not take the delete: the address is still there.
    NotWithdrawn,
    /// A cancelled sign-in whose address Settings handed out while it ran: kept, never deleted (2026-10-12).
    KeptCancelled,
    /// A failure (its word) after which the address would have gone, kept because Settings handed it out.
    KeptFailed(&'static str),
}

/// A sign-in whose mailbox another of the account's setups already holds (T18c): the service refused its
/// address as this one's hint.
const ALREADY_LINE: &str = "this mailbox is already set up in Knowlu: open Settings, then Email, to resume it or turn it off";
/// A setup that stopped before its address was ever put in the mailbox (T18c).
const WITHDRAWN_LINE: &str = "setup stopped before forwarding was set, so the Knowlu address made for it was removed";
/// The same stop, when the service did not take the delete (a 5xx, or no network): it is never said as done.
const NOT_WITHDRAWN_LINE: &str =
    "setup stopped before forwarding was set, but the Knowlu address made for it could not be removed: open Settings, then Email, and turn it off";

/// A setup that ended where its address would have been removed, but Settings handed it out while it ran: the
/// student may have put it in the mailbox's forwarding by hand, so it is kept, and the line says so (2026-10-12).
const KEPT_CANCELLED_LINE: &str = "setup cancelled, but the Knowlu address made for it was kept because Settings showed it: if your mailbox does not forward to it, open Settings, then Email, and turn it off";
const KEPT_FAILED_LINE: &str = "setup did not finish, but the Knowlu address made for it was kept because Settings showed it: if your mailbox does not forward to it, open Settings, then Email, and turn it off";

/// What a rotate that did not finish says: the mailbox forwards to the old address, which the service stops
/// accepting after its grace (spec §4.6, a retired token).
const ROTATED_LINE: &str = "forwarding still goes to your old address, which stops working in 7 days: finish the new address";

enum Probe {
    Hint(String),
    NotFound,
    Wait,
}

struct Runner<'a> {
    plan: &'a Plan,
    env: &'a Env,
    sh: &'a Shared,
    provider: Provider,
    machine: Setup,
    id: String,
    /// Knowlu's forwarding address for this mailbox (kept in memory for the run).
    knowlu: String,
    profile: PathBuf,
    /// The window was opened (so it is closed, and a throwaway profile wiped).
    opened: bool,
    canary_ok: bool,
    said_no: bool,
    gave_up: bool,
    /// The profile is the mailbox's kept one (so it stays); `false` is the throwaway `mail-setup`.
    keep: bool,
    /// A rotate has asked the service for the new address: the old one is retired and will stop working.
    rotated: bool,
    /// The Knowlu address may be in the mailbox: a step script that could change the mailbox answered
    /// anything but `not found` or `elsewhere` (both change nothing), a guided step showed the address with
    /// Copy, or the student said a step was done.
    placed: bool,
    /// A backfill began under this address: revoking it would purge the mail brought in (T18c review).
    backfilled: bool,
    /// The address this setup minted has been revoked.
    revoked: bool,
    /// A revoke found the address handed out by Settings (or `mail.json` unreadable) and kept it.
    kept: bool,
    /// The service's sentence when it refused the mailbox's address at sign-in.
    refusal: &'static str,
    /// When this run's `m365/forwarding_set.js` changed the mailbox's forwarding (T18c): Outlook may take
    /// minutes to begin forwarding, so the canary waits for it and has a second try.
    forwarding_set_at: Option<i64>,
}

fn host(env: &Env) -> &dyn Host {
    &*env.host
}

/// One setup, start to end.
pub(super) fn run_setup(plan: &Plan, env: &Env, sh: &Shared) {
    let end = match Runner::begin(plan, env, sh) {
        Ok(mut r) => {
            let end = r.go();
            r.wrap_up(&end);
            // The mailbox still forwards to the old address unless the forwarding step finished: whatever
            // else ended the run, the student is told, never "cancelled" or "stopped" as if nothing changed.
            let end = match end {
                End::Finished => End::Finished,
                _ if r.rotated => End::NeedsYou(ROTATED_LINE),
                // Nothing was put in the mailbox: the address goes, so it neither holds the mailbox's hint
                // against the next setup nor counts as live (T18c).
                end => match (r.withdraw(), end) {
                    (Some(true), End::Stopped) => End::Withdrawn,
                    (Some(false), End::Stopped) => End::NotWithdrawn,
                    (_, End::Cancelled) if r.kept => End::KeptCancelled,
                    (_, End::Failed(word)) if r.kept => End::KeptFailed(word),
                    (_, end) => end,
                },
            };
            // The run handled its address: kept with its facts, or revoked (a revoke the service refused
            // keeps its record, for the next start). A rotate never wrote one.
            if !r.revoked && !r.rotating() {
                clear_pending(&plan.data_dir, &r.id);
            }
            end
        }
        Err(end) => end,
    };
    let (ended, line) = match end {
        End::NeedsYou(line) => ("needs_you", line),
        End::Finished => ("finished", "setup finished"),
        End::Stopped => ("stopped", "setup stopped"),
        End::Withdrawn => ("stopped", WITHDRAWN_LINE),
        End::NotWithdrawn => ("stopped", NOT_WITHDRAWN_LINE),
        End::Cancelled => ("cancelled", "setup cancelled"),
        End::KeptCancelled => ("cancelled", KEPT_CANCELLED_LINE),
        End::Failed(word) => ("failed", word),
        End::KeptFailed(word) => {
            // The failure stays named as the view's sentence; the line says the address was kept.
            sh.update(|p| p.sentence = word);
            ("failed", KEPT_FAILED_LINE)
        }
    };
    sh.end(ended, line);
}

fn provider_name(p: Provider) -> &'static str {
    match p {
        Provider::M365 => "m365",
        Provider::Gmail => "google",
        Provider::Guided => "other",
    }
}

/// A run the service refused: its fixed sentence is the view's sentence and the ended line.
fn refused(sh: &Shared, sentence: &'static str) -> End {
    sh.update(|p| p.sentence = sentence);
    End::Failed(sentence)
}

fn answers(v: &Value, key: &str) -> Option<String> {
    v.get(key).and_then(Value::as_str).map(str::to_string)
}

impl<'a> Runner<'a> {
    /// Which provider, the address minted for it, and the first progress. `Err` is a run that never began.
    fn begin(plan: &'a Plan, env: &'a Env, sh: &'a Shared) -> Result<Runner<'a>, End> {
        let (provider, typed) = Runner::resolve(plan, env, sh)?;
        // A rotate keeps the mailbox it has and asks for its new address later (`rotate_now`, once the
        // sign-in has taken, so a window closed first changes nothing on the service); every other setup
        // mints a mailbox now.
        let ((id, knowlu), keep) = match &plan.kind {
            Kind::Rotate { address_id, .. } => {
                // A rotate puts its address in the mailbox: a record a quit setup left for it is placed now.
                mark_shown(&plan.data_dir, [address_id.as_str()]);
                let kept = session_dir(&plan.data_dir, address_id).is_some_and(|d| d.is_dir());
                ((address_id.clone(), String::new()), kept)
            }
            _ => {
                let v = env.client.mint_address(provider_name(provider)).map_err(|e| refused(sh, e.fixed_sentence()))?;
                let Some(knowlu) = answers(&v, "address") else { return Err(End::Failed("bad-reply")) };
                ((answers(&v, "address_id").unwrap_or_default(), knowlu), plan.keep_session)
            }
        };
        let profile = match keep {
            true => session_dir(&plan.data_dir, &id),
            false => Some(setup_dir(&plan.data_dir)),
        };
        let Some(profile) = profile.filter(|_| valid_address_id(&id)) else { return Err(End::Failed("bad-reply")) };
        env.host.use_provider(provider);
        let machine = Setup::new(provider, plan.setup_day, None);
        let mut r = Runner {
            plan, env, sh, provider, machine, id, knowlu, profile, opened: false, canary_ok: false, said_no: false, gave_up: false, keep,
            rotated: false, placed: false, backfilled: false, revoked: false, kept: false, refusal: "", forwarding_set_at: None,
        };
        // A quit from here on leaves the address this device minted named, for the next start (2026-10-06).
        r.persist(false);
        // A guided provider has no page to read the mailbox's own address from: the one the student typed.
        if let (Provider::Guided, Some(t)) = (provider, typed) {
            if let Err(e) = r.client().set_hint(&r.id, &t) {
                let removed = r.revoke();
                let end = refused(sh, if e.hint_held() { ALREADY_LINE } else { e.fixed_sentence() });
                return Err(match (removed, end) {
                    // Kept: the run handled its address, so its record goes (as `run_setup` clears one).
                    (None, End::Failed(word)) => {
                        clear_pending(&plan.data_dir, &r.id);
                        End::KeptFailed(word)
                    }
                    (_, end) => end,
                });
            }
        }
        r.sh.update(|p| {
            p.provider = provider_name(provider);
            p.address_id = Some(r.id.clone()).filter(|i| !i.is_empty());
        });
        r.publish();
        Ok(r)
    }

    /// §4.1 step 1: the curated row's provider (data), else one question and the service's MX lookup.
    fn resolve(plan: &Plan, env: &Env, sh: &Shared) -> Result<(Provider, Option<String>), End> {
        let (unitid, address) = match &plan.kind {
            Kind::School { unitid, address } => (unitid, address),
            Kind::Rotate { provider, .. } => return Ok((Provider::from_service(provider), None)),
            Kind::Gmail => return Ok((Provider::Gmail, None)),
        };
        let curated = unitid.as_deref().and_then(crate::scaffold::curated).map(|c| Provider::from_service(c.mail_provider));
        if let Some(p) = curated.filter(|p| *p != Provider::Guided) {
            return Ok((p, address.clone()));
        }
        let typed = match address {
            Some(a) => a.clone(),
            None => match sh.wait_answer(Waiting::Ask(Ask::SchoolAddress), IDLE) {
                Some(Answer::SchoolAddress(a)) => a,
                _ => return Err(End::Stopped),
            },
        };
        let typed = typed.trim().to_string();
        let domain = typed.rsplit_once('@').map(|(_, d)| d.trim().to_ascii_lowercase()).filter(|d| !d.is_empty());
        let domain = domain.ok_or(End::Failed("that is not an email address"))?;
        let answer = env.client.provider(&domain).map_err(|e| End::Failed(e.name()))?;
        let provider = Provider::from_service(answer.get("provider").and_then(Value::as_str).unwrap_or("other"));
        Ok((provider, Some(typed)))
    }

    fn client(&self) -> &MailClient {
        &self.env.client
    }

    fn clock(&self) -> &dyn Clock {
        &*self.env.clock
    }

    fn host(&self) -> &dyn Host {
        host(self.env)
    }

    /// Publishes the machine's rows, and the blocked state with what remains (D16).
    fn publish(&self) {
        let rows: Vec<Row> = self
            .provider
            .steps()
            .iter()
            .map(|s| {
                let (state, reason) = state_name(self.machine.state(*s));
                Row { step: step_name(*s), state, reason, guided: self.machine.mode(*s) == Mode::Guided }
            })
            .collect();
        let blocked = self.machine.state(Step::Backfill) == State::Failed(Reason::Blocked);
        let personal = matches!(self.plan.kind, Kind::School { .. });
        let since = self.machine.backfill_since().to_string();
        self.sh.update(|p: &mut Progress| {
            p.rows = rows;
            p.blocked = blocked;
            p.offers = if blocked { ["retry"].into_iter().chain(personal.then_some("personal_gmail")).chain(["guided_steps"]).collect() } else { vec![] };
            if p.backfill.is_none() && !self.rotating() && matches!(self.machine.state(Step::Backfill), State::Working | State::Done | State::DoneByHand) {
                p.backfill = Some(Backfill { since, expected: 0, handled: 0, sent: 0, left: 0, eta_secs: None, resumes_at: None, paused: None });
            }
        });
    }

    fn say(&self, sentence: &'static str) {
        self.sh.update(|p| p.sentence = sentence);
    }

    fn rotating(&self) -> bool {
        matches!(self.plan.kind, Kind::Rotate { .. })
    }

    /// Revokes the address this setup minted and deletes its profile: a setup that changed nothing. Every
    /// delete a setup makes comes through here. `Some(true)` when the service took the delete, `Some(false)`
    /// when it did not, `None` when nothing was asked of it: a rotate minted no mailbox (the mailbox, its kept
    /// profile and its open backfill are not this run's to undo), and an address Settings handed out while
    /// the run went on (`shown`; the student may have pasted it into the mailbox) is kept (`kept`). An
    /// unreadable `mail.json` reads as shown: never a guess toward deleting. The check and the delete are
    /// made under `mail.json`'s lock, so a status read marks the record either before (kept) or after (the
    /// record is gone with the address).
    fn revoke(&mut self) -> Option<bool> {
        let mut deleted = None;
        if !self.rotating() {
            let (client, id) = (self.client(), self.id.as_str());
            let _ = MailState::update(&self.plan.data_dir, |s| {
                if s.pending_setups.iter().any(|p| p.address_id == id && p.shown) {
                    return false;
                }
                let ok = client.delete_address(id).is_ok();
                deleted = Some(ok);
                // A delete the service did not take stays recorded (never placed), so the next start tries again.
                let before = s.pending_setups.len();
                if ok {
                    s.pending_setups.retain(|p| p.address_id != id);
                }
                s.pending_setups.len() != before
            });
        }
        if self.opened {
            self.host().close_window();
        }
        let Some(ok) = deleted else {
            self.kept = !self.rotating();
            if !self.keep {
                let _ = forget_setup(&self.plan.data_dir);
            }
            return None;
        };
        self.revoked = true;
        let _ = if self.keep { forget(&self.plan.data_dir, &self.id) } else { forget_setup(&self.plan.data_dir) };
        let _ = mark_open(&self.plan.data_dir, &self.id, false);
        Some(ok)
    }

    /// A setup that ended any way but finished, whose address was never put in the mailbox, revokes it as a
    /// closed sign-in window does (§4.2 step 1: nothing changed). `Some` when it asked the service now, `true`
    /// when the service took the delete. A rotate's address is the mailbox's own, never this run's to undo,
    /// an address a backfill began under holds mail that a revoke would purge, and one Settings handed out is
    /// [`Runner::revoke`]'s to keep.
    fn withdraw(&mut self) -> Option<bool> {
        if self.rotating() || self.placed || self.backfilled || self.revoked || self.kept {
            return None;
        }
        self.revoke()
    }

    /// The progress view shows a guided step, the Knowlu address with Copy, beside any failed row or a row
    /// that needs the student's own hand (`console.js` `mpGuide`); the forwarding-elsewhere question's
    /// buttons replace it.
    fn guide_shown(&self) -> bool {
        let shown = |s: &Step| matches!(self.machine.state(*s), State::Failed(_) | State::NeedsYou(Reason::Guided));
        self.provider.steps().iter().any(shown)
    }

    /// Notes a run of a script that could put the Knowlu address in the mailbox: only `not found` and
    /// `elsewhere` are answered with nothing changed (each script's contract); anything else, an error
    /// included, may have changed it.
    fn note_placed(&mut self, v: &Option<Value>) {
        self.placed |= !matches!(Runner::result(v), "not found" | "elsewhere");
        self.persist(false);
    }

    /// Writes this setup's record in `mail.json`: its address id, `placed` (or `placing`: a script that could
    /// set it is about to run, so a quit mid-script counts as placed) and `backfilled`. A rotate minted
    /// nothing, and a revoked address has nothing left to clean.
    fn persist(&self, placing: bool) {
        if self.rotating() || self.revoked {
            return;
        }
        let rec = PendingSetup { address_id: self.id.clone(), placed: self.placed || placing, backfilled: self.backfilled, shown: false };
        note_pending(&self.plan.data_dir, rec);
    }

    /// A rotate's call to the service. It cannot be undone (the old address is retired after its grace and
    /// one of the month's mints is used), so it is made only once the sign-in has taken and the forwarding
    /// step is next. Once it has been asked, `rotated` holds even if the reply is unusable.
    fn rotate_now(&mut self) -> Result<(), End> {
        let v = self.client().rotate_address(&self.id).map_err(|e| End::Failed(e.name()))?;
        self.rotated = true;
        self.knowlu = answers(&v, "address").ok_or(End::Failed("bad-reply"))?;
        Ok(())
    }

    fn finished_end(&self) -> End {
        if self.sh.stopping() || self.said_no || self.gave_up { End::Stopped } else { End::Finished }
    }

    /// The loop: the machine hands out a step, the runner runs it, the machine takes its outcome.
    fn go(&mut self) -> End {
        loop {
            if self.sh.stopping() {
                self.machine.request_stop();
            }
            let action = self.machine.next_action();
            // A rotate asks for its new address right before the step that carries it, and not before.
            let carries = matches!(action, Action::Run { step: Step::Forwarding | Step::AddAddress, .. } | Action::Guide { step: Step::Forwarding });
            if carries && self.rotating() && !self.rotated {
                if let Err(end) = self.rotate_now() {
                    return end;
                }
            }
            // From the first publish that shows the address with Copy, the student may have put it in the mailbox
            // by hand without saying so: it is never withdrawn (T18c review).
            if action != Action::Finished && self.guide_shown() && !self.placed {
                self.placed = true;
                self.persist(false);
            }
            self.publish();
            match action {
                Action::Finished => return self.finished_end(),
                Action::Run { step, replace_forwarding, .. } => {
                    self.machine.started(step);
                    self.publish();
                    let out = self.exec(step, replace_forwarding);
                    self.machine.finished(step, out);
                    if step == Step::SignIn && out != Outcome::Done {
                        self.revoke();
                        return match out {
                            Outcome::Cancelled => End::Cancelled,
                            Outcome::AlreadyConnected => End::Failed(ALREADY_LINE),
                            Outcome::Refused => End::Failed(self.refusal),
                            _ => End::Failed("sign-in"),
                        };
                    }
                }
                Action::Guide { step: Step::Canary } if self.canary_ok => self.machine.guided_checked(Step::Canary, true),
                Action::Guide { .. } | Action::Wait => self.wait_student(),
            }
        }
    }

    /// Setup is over: the window closes, a throwaway profile is wiped, and the progress says what was done.
    fn wrap_up(&mut self, end: &End) {
        if self.opened {
            self.host().close_window();
            if !self.keep && !matches!(end, End::Cancelled) {
                let _ = forget_setup(&self.plan.data_dir);
            }
        }
        let summary = self.machine.summary();
        self.publish();
        self.sh.update(|p| {
            p.done = summary.done.iter().map(|s| step_name(*s)).collect();
            p.left = summary.left.iter().map(|s| step_name(*s)).collect();
            p.not_brought_in_before = summary.mail_not_brought_in_before.map(|d| d.to_string());
            p.sentence = "";
        });
    }
}

// ---- the steps -------------------------------------------------------------------------------------

impl Runner<'_> {
    fn exec(&mut self, step: Step, replace: bool) -> Outcome {
        match step {
            Step::SignIn => self.sign_in(),
            Step::StepAside => {
                self.say("the window steps aside");
                if self.host().step_aside().is_ok() { Outcome::Done } else { Outcome::Failed }
            }
            Step::Forwarding => self.forwarding(replace),
            Step::AddAddress => self.add_address(replace),
            Step::ConfirmAddress => self.confirm_address(),
            Step::TurnOn => self.turn_on(replace),
            Step::Canary => {
                self.say("waiting for the test message");
                if self.canary_ok || self.canary_after_set() {
                    self.canary_ok = true;
                    Outcome::Done
                } else {
                    Outcome::Failed
                }
            }
            // A rotate changes where mail goes, not what has been brought in.
            Step::Backfill if self.rotating() => Outcome::Done,
            Step::Backfill => {
                self.backfilled = true;
                self.persist(false);
                self.backfill()
            }
            Step::Finish => {
                self.say("finishing");
                self.host().close_window();
                if !self.keep {
                    let _ = forget_setup(&self.plan.data_dir);
                }
                Outcome::Done
            }
        }
    }

    /// §4.2 step 1, §4.3 step 1: the visible window until the page reads as a signed-in mailbox; the
    /// mailbox's own address is its hint. A window the student closes is a cancelled setup. The title
    /// turns before the page has drawn the address (T18c), so the page is read again each second for
    /// [`SIGN_IN_READ`] from the title first reading signed in before the step ends `not found`.
    fn sign_in(&mut self) -> Outcome {
        self.say("sign in to your email");
        if self.host().open_window(&self.profile, true).is_err() {
            return Outcome::Failed;
        }
        self.opened = true;
        let deadline = self.clock().now() + SIGN_IN_WAIT;
        let mut read_by: Option<i64> = None;
        loop {
            if !self.host().window_open() || self.sh.stopping() {
                return Outcome::Cancelled;
            }
            if !self.host().signed_in() {
                read_by = None;
            } else if self.rotating() {
                return Outcome::Done;
            } else {
                // The read's clock starts when the title first reads signed in, so a page that errs while
                // Outlook finishes navigating (`Wait`) near the limit still gets its read (T18c review).
                let by = *read_by.get_or_insert(self.clock().now() + SIGN_IN_READ);
                match self.probe() {
                    // The service's refusal is named by its status and sentence (T18c): another of the
                    // account's mailboxes holding this address is not a sign-in that did not finish.
                    Probe::Hint(h) => {
                        return match self.client().set_hint(&self.id, &h) {
                            Ok(_) => Outcome::Done,
                            Err(e) if e.hint_held() => Outcome::AlreadyConnected,
                            Err(e) => {
                                self.refusal = e.fixed_sentence();
                                Outcome::Refused
                            }
                        };
                    }
                    Probe::NotFound => {
                        if self.clock().now() >= by {
                            return Outcome::NotFound;
                        }
                    }
                    Probe::Wait => {}
                }
            }
            // A read under way finishes even past the sign-in's limit, so its failure is named, never cancelled.
            if self.clock().now() >= read_by.map_or(deadline, |by| by.max(deadline)) {
                return Outcome::Cancelled;
            }
            self.clock().sleep(1);
        }
    }

    /// The page's half of the sign-in check (D5): Microsoft's script reads the folder pane; Gmail's address
    /// is in the title.
    fn probe(&self) -> Probe {
        match self.provider {
            Provider::M365 => match self.host().script("m365/signin.js", &json!({})) {
                Ok(v) => match (v["result"].as_str(), v["signed_in"].as_bool()) {
                    (Some("ok"), Some(true)) => answers(&v, "mailbox_hint").map_or(Probe::NotFound, Probe::Hint),
                    (Some("not found"), _) => Probe::NotFound,
                    _ => Probe::Wait,
                },
                Err(_) => Probe::Wait,
            },
            Provider::Gmail => self.host().title_address().map_or(Probe::NotFound, Probe::Hint),
            Provider::Guided => Probe::NotFound,
        }
    }

    /// The student is needed (an SSO or MFA prompt, a consent page): the window comes back until the page
    /// is the mailbox again, then steps aside. `false` if it never is.
    fn needs_student(&self) -> bool {
        self.say("the site needs you");
        let _ = self.host().come_back();
        let deadline = self.clock().now() + SIGN_IN_WAIT;
        while !self.host().signed_in() {
            if !self.host().window_open() || self.clock().now() >= deadline || self.sh.stopping() {
                return false;
            }
            self.clock().sleep(1);
        }
        self.host().step_aside().is_ok()
    }

    /// Runs a step script with the two repairs a page can ask for: the student's sign-in prompt (once), and
    /// a Gmail page that reloads (once, with `reloaded: true`; a second miss is `not found`).
    fn run(&mut self, script: &str, mut args: Value) -> Option<Value> {
        let (mut reloaded, mut came_back) = (false, false);
        for _ in 0..4 {
            let r = self.host().script(script, &args);
            let reloading = match &r {
                Ok(v) => v["result"].as_str() == Some("reloading"),
                Err(Refusal::Lost) => self.provider == Provider::Gmail,
                _ => false,
            };
            if reloading {
                if reloaded {
                    return Some(json!({ "result": "not found", "control": "section" }));
                }
                reloaded = true;
                args["reloaded"] = json!(true);
                let _ = self.host().open(Place::Forwarding);
                continue;
            }
            match r {
                Ok(v) => return Some(v),
                Err(Refusal::OffOrigin) if !came_back => {
                    came_back = true;
                    if !self.needs_student() {
                        return None;
                    }
                }
                Err(_) => return None,
            }
        }
        None
    }

    fn result(v: &Option<Value>) -> &str {
        v.as_ref().and_then(|v| v["result"].as_str()).unwrap_or("")
    }

    /// The mapping every forwarding script's answer shares.
    fn outcome(v: &Option<Value>) -> Outcome {
        match Runner::result(v) {
            "ok" => Outcome::Done,
            "elsewhere" => Outcome::ForwardingElsewhere,
            "not found" => Outcome::NotFound,
            _ => Outcome::Failed,
        }
    }
}

impl Runner<'_> {
    /// §4.2 step 3: set, then navigate away and back and read the setting again.
    fn forwarding(&mut self, replace: bool) -> Outcome {
        self.say("turning forwarding on");
        if self.host().open(Place::Forwarding).is_err() {
            return Outcome::Failed;
        }
        let args = json!({ "address": self.knowlu, "replace": replace });
        self.persist(true);
        let set = self.run("m365/forwarding_set.js", args);
        self.note_placed(&set);
        match Runner::outcome(&set) {
            Outcome::Done => {}
            other => return other,
        }
        if set.as_ref().is_some_and(|v| v["changed"] == true) {
            self.forwarding_set_at = Some(self.clock().now());
        }
        if self.host().open(Place::Inbox).is_err() || self.host().open(Place::Forwarding).is_err() {
            return Outcome::Failed;
        }
        let read = self.run("m365/forwarding_read.js", json!({ "address": self.knowlu }));
        let ours = read.as_ref().is_some_and(|v| v["forwarding"] == "on" && v["to"] == "ours" && v["keep_copy"] == true);
        if Runner::result(&read) == "ok" && ours { Outcome::Done } else { Outcome::Failed }
    }

    /// §4.3 step 3: the 15-minute confirm window opens first (Google mails the confirmation in seconds),
    /// then the Knowlu address is added; Google's popup is the student's, and the window waits it out.
    fn add_address(&mut self, replace: bool) -> Outcome {
        self.say("adding the forwarding address");
        if self.client().set_state(&self.id, SvcState::Confirming).is_err() {
            return Outcome::Failed;
        }
        self.persist(true);
        let v = self.run("gmail/forwarding_add.js", json!({ "address": self.knowlu, "replace": replace }));
        self.note_placed(&v);
        if Runner::result(&v) != "ok" {
            return Runner::outcome(&v);
        }
        if v.as_ref().and_then(|v| v["stage"].as_str()) == Some("listed") {
            return Outcome::Done;
        }
        self.wait_popup();
        let done = self.run("gmail/forwarding_add_done.js", json!({}));
        Runner::outcome(&done)
    }

    /// Google's "verify it's you" popup: waited for (it opens when the page asks), then out (the phone tap
    /// is the student's), and the window steps aside again.
    fn wait_popup(&self) {
        let start = self.clock().now();
        let mut seen = false;
        loop {
            let waited = self.clock().now() - start;
            if self.host().popup_open() {
                if !seen {
                    seen = true;
                    self.say("Google wants to confirm it is you");
                }
            } else if seen || waited >= POPUP_APPEAR {
                break;
            }
            if waited >= POPUP_WAIT {
                break;
            }
            self.clock().sleep(1);
        }
        if seen {
            let _ = self.host().step_aside();
        }
    }

    /// §4.3 step 4: Knowlu confirms the address itself; the view waits until the mailbox reads confirmed.
    fn confirm_address(&mut self) -> Outcome {
        self.say("Knowlu is confirming the address");
        let deadline = self.clock().now() + CONFIRM_WAIT;
        loop {
            if self.address_confirmed() {
                return Outcome::Done;
            }
            if self.clock().now() >= deadline {
                return Outcome::Failed;
            }
            self.clock().sleep(POLL);
        }
    }

    fn row(&self) -> Option<Value> {
        let rows = self.client().list_addresses().ok()?;
        rows.into_iter().find(|r| r["address_id"].as_str() == Some(self.id.as_str()))
    }

    fn address_confirmed(&self) -> bool {
        self.row().is_some_and(|r| matches!(r["status"].as_str(), Some("confirmed" | "active")))
    }

    fn turn_on(&mut self, replace: bool) -> Outcome {
        self.say("turning forwarding on");
        self.persist(true);
        let v = self.run("gmail/forwarding_set.js", json!({ "address": self.knowlu, "replace": replace }));
        self.note_placed(&v);
        Runner::outcome(&v)
    }

    /// D14: sends one canary and waits up to three minutes for it to come back through forwarding.
    fn canary_check(&mut self) -> bool {
        match self.send_canary() {
            Some(sent_at) => self.canary_back(sent_at),
            None => false,
        }
    }

    /// The setup's canary step. When this run has just set Outlook's forwarding (T18c), Outlook is first given
    /// [`FORWARD_SETTLE`] to begin forwarding, and a canary that does not come back in its three minutes is
    /// followed by one more, waited for as long: two of the service's three a day (§4.10), so one is left for
    /// the student's Retry. A second the service refuses (its daily limit) fails the step as before. Either
    /// canary coming back is enough.
    fn canary_after_set(&mut self) -> bool {
        let Some(set_at) = self.forwarding_set_at else { return self.canary_check() };
        if self.clock().now() < set_at + FORWARD_SETTLE {
            self.say(SETTLING);
            while self.clock().now() < set_at + FORWARD_SETTLE {
                self.clock().sleep(POLL.min(set_at + FORWARD_SETTLE - self.clock().now()));
            }
            self.say("waiting for the test message");
        }
        let Some(first) = self.send_canary() else { return false };
        if self.canary_back(first) {
            return true;
        }
        self.say(SETTLING);
        self.send_canary().is_some() && self.canary_back(first)
    }

    /// Asks the service for one canary; `None` when it would not send one. The inner value is its sent time.
    fn send_canary(&mut self) -> Option<Option<jiff::Timestamp>> {
        let sent = self.client().canary(&self.id).ok()?;
        Some(sent["sent_at"].as_str().and_then(|s| s.parse::<jiff::Timestamp>().ok()))
    }

    /// Waits up to [`CANARY_WAIT`] for a canary sent at or after `sent_at` to come back.
    fn canary_back(&mut self, sent_at: Option<jiff::Timestamp>) -> bool {
        let deadline = self.clock().now() + CANARY_WAIT;
        loop {
            if let Some(row) = self.row() {
                let back = row["last_canary_ok_at"].as_str().and_then(|s| s.parse::<jiff::Timestamp>().ok());
                let arrived = match (back, sent_at) {
                    (Some(b), Some(s)) => b >= s,
                    (Some(_), None) => true,
                    (None, _) => false,
                };
                if arrived {
                    return true;
                }
            }
            if self.clock().now() >= deadline {
                return false;
            }
            self.clock().sleep(POLL);
        }
    }

    /// §4.2 step 5: the recover routine over the fixed window, as the pace allows ([`drive_backfill`]).
    fn backfill(&mut self) -> Outcome {
        self.say("bringing in your mail");
        let mut routine = seed_routine(&self.plan.data_dir, self.provider, &self.id, self.clock().now());
        let d = Drive {
            mb: self.host(),
            client: self.client(),
            clock: self.clock(),
            provider: self.provider,
            address_id: &self.id,
            knowlu_address: &self.knowlu,
            since: self.machine.backfill_since(),
            shared: self.sh,
            on_closed: &*self.env.on_closed,
            data_dir: &self.plan.data_dir,
        };
        match drive_backfill(&d, &mut routine) {
            Drove::Done => Outcome::Done,
            Drove::Stopped => Outcome::Cancelled,
            Drove::Failed(_) => Outcome::Failed,
        }
    }
}

// ---- the student's turn ------------------------------------------------------------------------------

impl Runner<'_> {
    /// Whether a step is asking if another forwarding address may be replaced.
    fn asking_forwarding(&self) -> bool {
        self.provider.steps().iter().any(|s| self.machine.state(*s) == State::NeedsYou(Reason::ForwardingElsewhere))
    }

    /// The first step that is guided or failed: the one "I've done this" and Retry act on.
    fn open_step(&self) -> Option<Step> {
        let open = |s: &&Step| matches!(self.machine.state(**s), State::Failed(_) | State::NeedsYou(Reason::Guided));
        self.provider.steps().iter().find(open).copied()
    }

    /// Nothing runs: the student's answer, a guided step's "I've done this", or the end.
    fn wait_student(&mut self) {
        if self.asking_forwarding() {
            match self.sh.wait_answer(Waiting::Ask(Ask::ForwardingElsewhere), IDLE) {
                Some(Answer::Replace(yes)) => {
                    self.said_no |= !yes;
                    self.machine.answer_forwarding_elsewhere(yes);
                }
                _ => {
                    self.gave_up = true;
                    self.machine.answer_forwarding_elsewhere(false);
                }
            }
            return;
        }
        if self.open_step().is_none() {
            self.machine.request_stop();
            return;
        }
        self.publish();
        match self.sh.wait_answer(Waiting::Student, IDLE) {
            Some(Answer::Retry) if self.open_step() == Some(Step::Backfill) && self.machine.rearm_backfill() => {}
            Some(Answer::Done | Answer::Retry) => self.check_open(),
            _ => {
                self.gave_up = true;
                self.machine.request_stop();
            }
        }
    }

    /// §4.4: "I've done this" runs the step's check (a canary for forwarding, the server's confirm for the
    /// Gmail address); a pass makes it done by hand, a fail leaves it as it was. A backfill has no check:
    /// no count of a manual forward-as-attachment exists, so a tap cannot make it done. The setup ends
    /// naming the day before which mail was not brought in, and the backfill stays open in `mail.json` if
    /// it ever started. Retry on a backfill that failed is the way to run it again.
    fn check_open(&mut self) {
        let Some(step) = self.open_step() else { return };
        if step == Step::Backfill {
            self.say("mail before the start of the window was not brought in");
            self.gave_up = true;
            self.machine.request_stop();
            return;
        }
        // The student says this step is done: the address may be in the mailbox whatever the check reads.
        self.placed = true;
        self.persist(false);
        self.say("checking");
        let ok = match step {
            Step::AddAddress | Step::ConfirmAddress => self.address_confirmed(),
            _ => {
                let ok = self.canary_ok || self.canary_check();
                self.canary_ok |= ok;
                ok
            }
        };
        self.machine.guided_checked(step, ok);
        // A canary that arrives late (Retry) unblocks the backfill (D16): it runs on this run (§4.2 step 4).
        if ok && step == Step::Canary {
            self.machine.rearm_backfill();
        }
    }
}
