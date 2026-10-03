//! The backfill's driver (email-forwarding plan T11.3; spec §4.2 step 5, §4.11 "an open backfill is not
//! bound by the slot", §4.8 step 1). [`drive_backfill`] is T11.2's recover routine over the fixed window
//! (BW: 120 days before setup, never recomputed), run again as the pace allows: the day's 600 spent, or
//! Microsoft's refusal, ends a pass, and the driver waits for the next UTC day and runs the window again
//! (what was sent is held by then, so the next pass forwards only what is still missing). Each pass opens
//! a `backfill` batch before it lists and closes it with the count sent; **a closed batch starts a slot at
//! once** (`on_closed`; the app's hook is `scheduler::run_slot`) so `judge` pulls it without waiting.
//!
//! **An open backfill** is a mailbox id in `mail.json`'s `open_backfills`, written when the first pass
//! starts and removed when the window is whole or the student stops. The app marks nothing else, and a
//! quit leaves it, so [`run_resume`] (at launch, H4) opens the mailbox's kept profile **hidden**, checks it
//! is signed in, and drives the same loop on the window the service holds (`backfill_since`). Signed out:
//! no window is shown, nothing is read and the marker stays (§4.11 step 1). A mailbox the student turned
//! off is never resumed ([`open_backfills`]).
//!
//! **Privacy.** Counts, dates and fixed words only.
use super::recover::{backfill_window, run_pass, Clock, Job, Mailbox, Stop};
use super::setup::{Backfill, Env, Row, Shared};
use crate::mail_client::{MailClient, Phase};
use crate::mail_session::{session_dir, MailState};
use crate::mail_steps::{Provider, Routine};
use jiff::civil::Date;
use std::path::Path;

const DAY: i64 = 86_400;
/// How long a resumed mailbox's window gets to show a signed-in page before it is called signed out.
const SIGNED_IN_WAIT: i64 = 60;
/// A backfill that has not finished in this many passes is not a mailbox: stop before it is a loop.
const MAX_PASSES: u32 = 400;

/// What one mailbox's backfill runs with.
pub struct Drive<'a> {
    pub mb: &'a dyn Mailbox,
    pub client: &'a MailClient,
    pub clock: &'a dyn Clock,
    pub provider: Provider,
    pub address_id: &'a str,
    /// The mailbox's Knowlu forwarding address: the one recipient a forward may have.
    pub knowlu_address: &'a str,
    /// The window's first day, fixed when the first backfill opened.
    pub since: Date,
    pub shared: &'a Shared,
    /// Called once per closed batch: starts a slot now.
    pub on_closed: &'a dyn Fn(),
    pub data_dir: &'a Path,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Drove {
    /// The whole window is handled and its batches are closed.
    Done,
    /// The student stopped it.
    Stopped,
    /// A fixed word naming why it could not go on.
    Failed(&'static str),
}

/// The mailboxes with a backfill left open, minus any the student turned off in Knowlu's Settings.
pub fn open_backfills(data_dir: &Path) -> Vec<String> {
    match MailState::load(data_dir) {
        Ok(s) => s.open_backfills.iter().filter(|id| !s.is_turned_off(id)).cloned().collect(),
        Err(_) => Vec::new(),
    }
}

/// Marks (or unmarks) a mailbox's backfill open in `mail.json`.
pub(super) fn mark_open(data_dir: &Path, id: &str, open: bool) -> Result<(), String> {
    MailState::update(data_dir, |s| {
        let listed = s.open_backfills.iter().any(|i| i == id);
        match (open, listed) {
            (true, false) => s.open_backfills.push(id.to_string()),
            (false, true) => s.open_backfills.retain(|i| i != id),
            _ => return false,
        }
        true
    })
    .map(|_| ())
}

/// Waits (a clock's sleeps, a stop between them) until unix second `t`; `false` if the student stopped it.
fn wait_until(d: &Drive, t: i64) -> bool {
    loop {
        let now = d.clock.now();
        if now >= t {
            return true;
        }
        if d.shared.stopping() {
            return false;
        }
        d.clock.sleep((t - now).min(30));
    }
}

/// Runs passes of the recover routine over the fixed window until it is whole. `routine` carries the
/// UTC day's pace across passes.
pub fn drive_backfill(d: &Drive, routine: &mut Routine) -> Drove {
    if mark_open(d.data_dir, d.address_id, true).is_err() {
        return Drove::Failed("mail.json could not be written");
    }
    let since_idx = knowlu_engine::scheduling::days_between(d.since, jiff::civil::date(1970, 1, 1));
    let since = d.since.to_string();
    let started = d.clock.now();
    for _ in 0..MAX_PASSES {
        if d.shared.stopping() {
            let _ = mark_open(d.data_dir, d.address_id, false);
            return Drove::Stopped;
        }
        let now = d.clock.now();
        let watch = |expected: u32, handled: u32, sent: u32| -> bool {
            let elapsed = (d.clock.now() - started).max(0);
            let eta = (handled > 0 && expected > handled).then(|| i64::from(expected - handled) * elapsed / i64::from(handled));
            d.shared.update(|p| {
                p.backfill = Some(Backfill { since: since.clone(), expected, handled, sent, left: 0, eta_secs: eta, resumes_at: None, paused: None });
            });
            !d.shared.stopping()
        };
        let job = Job {
            provider: d.provider,
            address_id: d.address_id,
            knowlu_address: d.knowlu_address,
            phase: Phase::Backfill,
            window: backfill_window(since_idx, now),
            cap: None,
            watch: Some(&watch),
        };
        let out = run_pass(d.mb, d.client, d.clock, routine, &job);
        d.shared.update(|p| {
            p.backfill = Some(Backfill {
                since: since.clone(),
                expected: out.expected,
                handled: out.handled,
                sent: out.sent,
                left: out.left,
                eta_secs: None,
                resumes_at: None,
                paused: None,
            });
        });
        if out.closed {
            (d.on_closed)();
        }
        let next_day = (now.div_euclid(DAY) + 1) * DAY;
        let wait = match out.stop {
            Stop::Done => {
                let _ = mark_open(d.data_dir, d.address_id, false);
                return Drove::Done;
            }
            Stop::DayLimit { next_day_at } => (next_day_at, None),
            Stop::Paused(p) => (next_day, Some(p.line())),
            Stop::Stopped => {
                let _ = mark_open(d.data_dir, d.address_id, false);
                return Drove::Stopped;
            }
            Stop::Service(name) => return Drove::Failed(name),
            Stop::Script(word) => return Drove::Failed(word),
            Stop::Guided | Stop::SlotCap => return Drove::Failed("this mailbox is set up by hand"),
        };
        d.shared.update(|p| {
            if let Some(b) = p.backfill.as_mut() {
                b.resumes_at = Some(wait.0);
                b.paused = wait.1;
            }
        });
        if !wait_until(d, wait.0) {
            let _ = mark_open(d.data_dir, d.address_id, false);
            return Drove::Stopped;
        }
    }
    Drove::Failed("the backfill did not finish")
}

// ---- resuming at launch (§4.11) ------------------------------------------------------------------

/// A resumed mailbox: opens its kept profile hidden, checks it is signed in and drives its backfill.
/// `Err` is the line for the run: a fixed word or a service error's name.
fn resume_one(data_dir: &Path, id: &str, env: &Env, sh: &Shared) -> Result<(), &'static str> {
    let rows = env.client.list_addresses().map_err(|e| e.name())?;
    let row = rows.iter().find(|r| r["address_id"].as_str() == Some(id)).ok_or("no such mailbox")?;
    let provider = Provider::from_service(row["provider"].as_str().unwrap_or(""));
    if provider == Provider::Guided {
        return Err("this mailbox is set up by hand");
    }
    if matches!(row["status"].as_str(), Some("lapsed" | "revoked")) {
        return Err("lapsed");
    }
    let knowlu = row["address"].as_str().ok_or("bad-reply")?;
    let today = jiff::Timestamp::from_second(env.clock.now()).map_err(|_| "bad clock")?.to_zoned(jiff::tz::TimeZone::UTC).date();
    let stored = row["backfill_since"].as_str().and_then(|s| s.parse::<Date>().ok());
    let since = crate::mail_steps::backfill_window(stored, today);
    let profile = session_dir(data_dir, id).filter(|p| p.exists()).ok_or("needs sign-in")?;
    env.host.use_provider(provider);
    env.host.open_window(&profile, false).map_err(|_| "the mail window would not open")?;
    let deadline = env.clock.now() + SIGNED_IN_WAIT;
    while !env.host.signed_in() {
        if !env.host.window_open() || env.clock.now() >= deadline {
            env.host.close_window();
            return Err("needs sign-in");
        }
        env.clock.sleep(1);
    }
    let mut routine = Routine::new(provider, env.clock.now().div_euclid(DAY), 0);
    let d = Drive {
        mb: &*env.host,
        client: &env.client,
        clock: &*env.clock,
        provider,
        address_id: id,
        knowlu_address: knowlu,
        since,
        shared: sh,
        on_closed: &*env.on_closed,
        data_dir,
    };
    let end = drive_backfill(&d, &mut routine);
    env.host.close_window();
    match end {
        Drove::Done => Ok(()),
        Drove::Stopped => Err("stopped"),
        Drove::Failed(word) => Err(word),
    }
}

/// The resume's run: each open mailbox in turn, then the run's closing line.
pub(super) fn run_resume(data_dir: &Path, ids: &[String], env: &Env, sh: &Shared) {
    sh.update(|p| {
        p.rows = vec![Row { step: "backfill", state: "working", reason: None, guided: false }];
        p.sentence = "bringing in your mail";
    });
    let mut first: Option<&'static str> = None;
    for id in ids {
        if sh.stopping() {
            first.get_or_insert("stopped");
            break;
        }
        if let Err(line) = resume_one(data_dir, id, env, sh) {
            first.get_or_insert(line);
        }
    }
    sh.update(|p| {
        if let Some(r) = p.rows.first_mut() {
            r.state = if first.is_none() { "done" } else { "failed" };
        }
    });
    match first {
        None => sh.end("finished", "backfill finished"),
        Some("stopped") => sh.end("stopped", "setup stopped"),
        Some(line) => sh.end("failed", line),
    }
}
