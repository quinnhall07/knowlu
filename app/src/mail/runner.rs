//! The setup runner (email-forwarding plan T11.3; spec §4.1 to §4.5, D5, D6, D14, D16). It drives T10.1's
//! step machine ([`crate::mail_steps::Setup`]) through the mail window ([`Host`]), T10's compiled-in
//! scripts and T11.1's client, writes the progress rows ([`Shared`]) and takes the student's answers. It
//! runs on a background thread and never writes the vault.
//!
//! **The flow.** Which provider (the curated row's `mail_provider`, else one question and the service's MX
//! lookup; §4.1); mint the mailbox's address; sign in (the window is visible only for this); the window
//! steps aside; the provider's forwarding steps; the setup canary (three minutes, D14); the backfill over a
//! fixed 120 days (BW; [`super::backfill`]); finish. A scripted step that cannot go on falls to its guided
//! version and the student's "I've done this" runs its check (§4.4); nothing is retried in a loop, except
//! that a Gmail script that reloads its page is run again once with `reloaded: true`.
//!
//! **A canary that does not come back** (D16) leaves the mailbox blocked: the backfill is not run (old mail
//! would arrive while new mail never does), Retry sends another canary, and what remains is offered:
//! personal Gmail beside a school mailbox, and the guided steps. A retry that arrives marks the backfill
//! open for the next launch (§4.11).
//!
//! **A closed sign-in window is "setup cancelled"**: nothing changed, so the address minted for it is
//! revoked and its profile deleted. **Stop** ends after the step in hand and names what was done.
//!
//! **Privacy.** No address, hint, URL, selector or page text is logged, kept in [`Progress`] or sent
//! anywhere but where its step says: the typed school address contributes only its domain to the MX lookup
//! (and is the mailbox's own address only for a guided provider, which has no page to read it from).
use super::backfill::{drive_backfill, mark_open, seed_routine, Drive, Drove};
use super::host::Host;
use super::recover::{Clock, Place};
use super::setup::{
    state_name, step_name, Answer, Ask, Backfill, Env, Kind, Plan, Progress, Row, Shared, Waiting, IDLE,
};
use super::Refusal;
use crate::mail_client::{MailClient, State as SvcState};
use crate::mail_session::{forget, forget_setup, session_dir, setup_dir, valid_address_id};
use crate::mail_steps::{Action, Mode, Outcome, Provider, Reason, Setup, State, Step};
use serde_json::{json, Value};
use std::path::PathBuf;

/// How long the sign-in may take (the school's SSO, Duo, the phone tap).
const SIGN_IN_WAIT: i64 = 15 * 60;
/// D14: the setup canary is waited for up to three minutes.
const CANARY_WAIT: i64 = 3 * 60;
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
}

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
            end
        }
        Err(end) => end,
    };
    let (ended, line) = match end {
        End::Finished => ("finished", "setup finished"),
        End::Stopped => ("stopped", "setup stopped"),
        End::Cancelled => ("cancelled", "setup cancelled"),
        End::Failed(word) => ("failed", word),
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

fn answers(v: &Value, key: &str) -> Option<String> {
    v.get(key).and_then(Value::as_str).map(str::to_string)
}

impl<'a> Runner<'a> {
    /// Which provider, the address minted for it, and the first progress. `Err` is a run that never began.
    fn begin(plan: &'a Plan, env: &'a Env, sh: &'a Shared) -> Result<Runner<'a>, End> {
        let (provider, typed) = Runner::resolve(plan, env, sh)?;
        let minted = env.client.mint_address(provider_name(provider)).map_err(|e| End::Failed(e.name()))?;
        let (Some(id), Some(knowlu)) = (answers(&minted, "address_id"), answers(&minted, "address")) else {
            return Err(End::Failed("bad-reply"));
        };
        let profile = match plan.keep_session {
            true => session_dir(&plan.data_dir, &id),
            false => Some(setup_dir(&plan.data_dir)),
        };
        let Some(profile) = profile.filter(|_| valid_address_id(&id)) else { return Err(End::Failed("bad-reply")) };
        env.host.use_provider(provider);
        let machine = Setup::new(provider, plan.setup_day, None);
        let r = Runner { plan, env, sh, provider, machine, id, knowlu, profile, opened: false, canary_ok: false, said_no: false, gave_up: false };
        // A guided provider has no page to read the mailbox's own address from: the one the student typed.
        if let (Provider::Guided, Some(t)) = (provider, typed) {
            if r.client().set_hint(&r.id, &t).is_err() {
                r.revoke();
                return Err(End::Failed("refused"));
            }
        }
        r.sh.update(|p| p.provider = provider_name(provider));
        r.publish();
        Ok(r)
    }

    /// §4.1 step 1: the curated row's provider (data), else one question and the service's MX lookup.
    fn resolve(plan: &Plan, env: &Env, sh: &Shared) -> Result<(Provider, Option<String>), End> {
        let Kind::School { unitid, address } = &plan.kind else { return Ok((Provider::Gmail, None)) };
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
            if p.backfill.is_none() && matches!(self.machine.state(Step::Backfill), State::Working | State::Done | State::DoneByHand) {
                p.backfill = Some(Backfill { since, expected: 0, handled: 0, sent: 0, left: 0, eta_secs: None, resumes_at: None, paused: None });
            }
        });
    }

    fn say(&self, sentence: &'static str) {
        self.sh.update(|p| p.sentence = sentence);
    }

    /// Revokes the address this setup minted and deletes its profile: a setup that changed nothing.
    fn revoke(&self) {
        let _ = self.client().delete_address(&self.id);
        if self.opened {
            self.host().close_window();
        }
        let _ = if self.plan.keep_session { forget(&self.plan.data_dir, &self.id) } else { forget_setup(&self.plan.data_dir) };
        let _ = mark_open(&self.plan.data_dir, &self.id, false);
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
                        return if out == Outcome::Cancelled { End::Cancelled } else { End::Failed("sign-in") };
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
            if !self.plan.keep_session && !matches!(end, End::Cancelled) {
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
                if self.canary_ok || self.canary_check() {
                    self.canary_ok = true;
                    Outcome::Done
                } else {
                    Outcome::Failed
                }
            }
            Step::Backfill => self.backfill(),
            Step::Finish => {
                self.say("finishing");
                self.host().close_window();
                if !self.plan.keep_session {
                    let _ = forget_setup(&self.plan.data_dir);
                }
                Outcome::Done
            }
        }
    }

    /// §4.2 step 1, §4.3 step 1: the visible window until the page reads as a signed-in mailbox; the
    /// mailbox's own address is its hint. A window the student closes is a cancelled setup.
    fn sign_in(&mut self) -> Outcome {
        self.say("sign in to your email");
        if self.host().open_window(&self.profile, true).is_err() {
            return Outcome::Failed;
        }
        self.opened = true;
        let deadline = self.clock().now() + SIGN_IN_WAIT;
        loop {
            if !self.host().window_open() || self.sh.stopping() {
                return Outcome::Cancelled;
            }
            if self.host().signed_in() {
                match self.probe() {
                    Probe::Hint(h) => {
                        return if self.client().set_hint(&self.id, &h).is_ok() { Outcome::Done } else { Outcome::Failed };
                    }
                    Probe::NotFound => return Outcome::NotFound,
                    Probe::Wait => {}
                }
            }
            if self.clock().now() >= deadline {
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
        let set = self.run("m365/forwarding_set.js", args);
        match Runner::outcome(&set) {
            Outcome::Done => {}
            other => return other,
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
        let v = self.run("gmail/forwarding_add.js", json!({ "address": self.knowlu, "replace": replace }));
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
        let v = self.run("gmail/forwarding_set.js", json!({ "address": self.knowlu, "replace": replace }));
        Runner::outcome(&v)
    }

    /// D14: sends one canary and waits up to three minutes for it to come back through forwarding.
    fn canary_check(&mut self) -> bool {
        let Ok(sent) = self.client().canary(&self.id) else { return false };
        let sent_at = sent["sent_at"].as_str().and_then(|s| s.parse::<jiff::Timestamp>().ok());
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
