//! The backfill's driver (email-forwarding plan T11.3; spec §4.2 step 5, §4.11 "an open backfill is not
//! bound by the slot", §4.8 step 1). [`drive_backfill`] is T11.2's recover routine over the fixed window
//! (BW: 120 days before setup, never recomputed), run again as the pace allows: the day's sends spent, or
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
//! **Held for Microsoft 365 (D13, temporary).** While C3 is built, [`resume_one`] returns before any window opens
//! for a Microsoft 365 mailbox and leaves its marker open; the setup shows its Backfill as the guided step
//! and the slot makes no recover pass ([`crate::mail_steps::backfill_held_for_c3`]).
//!
//! **Privacy.** Counts, dates and fixed words only.
use super::recover::{backfill_window, pages_spent, run_pass, Clock, Job, Mailbox, Outcome, Stop, Stuck};
use super::setup::{Backfill, Env, Row, Shared};
use crate::mail_client::{MailClient, Phase};
use crate::mail_session::{session_dir, BackfillPace, LoneStall, MailState};
use crate::mail_steps::{backfill_held_for_c3, Provider, Routine, Send};
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

/// The open backfills a launch resumes: `ids` without the mailboxes whose backfill is held while C3 is built
/// (Microsoft 365, [`backfill_held_for_c3`]). Knowlu stops trying to resume what it cannot run yet (Quinn,
/// 2026-10-19), so a launch with only such markers starts no run: no failed line, no amber. Their markers
/// stay in `mail.json` for C3 to resume. If the service cannot be asked, every id is kept and the resume says
/// what it finds, as before.
pub fn launch_resumable(ids: Vec<String>, client: &MailClient) -> Vec<String> {
    let Ok(rows) = client.list_addresses() else { return ids };
    let held = |id: &String| {
        rows.iter()
            .find(|r| r["address_id"].as_str() == Some(id.as_str()))
            .is_some_and(|r| backfill_held_for_c3(Provider::from_service(r["provider"].as_str().unwrap_or(""))))
    };
    ids.into_iter().filter(|id| !held(id)).collect()
}

/// Marks (or unmarks) a mailbox's backfill open in `mail.json`.
pub(super) fn mark_open(data_dir: &Path, id: &str, open: bool) -> Result<(), String> {
    MailState::update(data_dir, |s| {
        let listed = s.open_backfills.iter().any(|i| i == id);
        match (open, listed) {
            (true, false) => s.open_backfills.push(id.to_string()),
            (false, true) => s.open_backfills.retain(|i| i != id),
            // A closed backfill keeps no frontier and no lone-stall record.
            (false, false) => {
                let frontier = s.backfill_frontier.remove(id).is_some();
                let left = s.lone_left.remove(id).is_some();
                return s.lone_stall.remove(id).is_some() || frontier || left;
            }
            _ => return false,
        }
        if !open {
            s.backfill_frontier.remove(id);
            s.lone_stall.remove(id);
            s.lone_left.remove(id);
        }
        true
    })
    .map(|_| ())
}

/// The send pace a mailbox's backfill starts with at unix second `now`: what `mail.json` kept for today
/// (the count, and a pause a refused send began), else a fresh day. A relaunch or a re-run the same UTC day
/// carries on from it (spec §4.2 step 5); a pace from an earlier day counts for nothing.
pub(super) fn seed_routine(data_dir: &Path, provider: Provider, id: &str, now: i64) -> Routine {
    let day = now.div_euclid(DAY);
    let state = MailState::load(data_dir).ok();
    let kept = state.as_ref().and_then(|s| s.backfill_pace.get(id).cloned()).filter(|p| p.day == day);
    let mut routine = match kept {
        Some(p) => {
            let routine = Routine::new(provider, day, p.sent_today).with_loads(p.loads_today).with_messages(p.messages_today);
            if p.paused_until.is_some_and(|t| t > now) { routine.paused() } else { routine }
        }
        None => Routine::new(provider, day, 0),
    };
    // Gmail's messages are a rolling 24 hours: their instants carry over days (a count with none kept holds the day).
    if let Some(times) = state.as_ref().and_then(|s| s.gmail_messages.get(id)) {
        routine = routine.with_message_times(times);
    }
    // The last look at the Inbox is not a day's fact: it carries over days, so the next look reads from it.
    if let Some(at) = state.and_then(|s| s.last_look.get(id).copied()) {
        routine.looked(at);
    }
    routine
}

/// Keeps a Gmail mailbox's messages of the last 24 hours (`times`, unix seconds) in `mail.json` at unix second
/// `now`: merged with what is kept, each instant as often as the more of the two has it (a note never lowers what
/// another kept), and pruned to the 24 hours before `now`. Best effort, like [`keep_pace`].
pub(super) fn keep_messages(data_dir: &Path, id: &str, times: &[i64], now: i64) {
    let _ = MailState::update(data_dir, |s| {
        let old = s.gmail_messages.get(id).cloned().unwrap_or_default();
        let count = |list: &[i64]| {
            let mut c = std::collections::BTreeMap::<i64, usize>::new();
            for t in list.iter().filter(|t| *t + DAY > now) {
                *c.entry(*t).or_default() += 1;
            }
            c
        };
        let mut merged = count(&old);
        for (t, n) in count(times) {
            let m = merged.entry(t).or_default();
            *m = (*m).max(n);
        }
        let merged: Vec<i64> = merged.into_iter().flat_map(|(t, n)| std::iter::repeat(t).take(n)).collect();
        if merged == old {
            return false;
        }
        if merged.is_empty() {
            s.gmail_messages.remove(id);
        } else {
            s.gmail_messages.insert(id.to_string(), merged);
        }
        true
    });
}

/// Keeps a look at the Inbox in `mail.json` for the next pass to be seeded from (see [`seed_routine`]). Best
/// effort, like [`keep_pace`].
pub(super) fn save_look(data_dir: &Path, id: &str, at: i64) {
    let _ = MailState::update(data_dir, |s| s.last_look.insert(id.to_string(), at) != Some(at));
}

/// Keeps a mailbox's day count, page loads, Gmail messages and pause in `mail.json`, and drops every pace whose day is past.
/// A pace for the day already kept is merged, never lowered: each count by its larger, a pause kept (a
/// progress note knows the sends but not the loads, a look's note the loads but not the sends). `create`
/// false only merges into a pace kept for the same day. Best effort: a pace that could not be written costs
/// a relaunch its memory, never the backfill.
pub(super) fn keep_pace(data_dir: &Path, id: &str, pace: BackfillPace, create: bool) {
    let today = pace.day;
    let _ = MailState::update(data_dir, |s| {
        let before = s.backfill_pace.clone();
        s.backfill_pace.retain(|_, p| p.day >= today);
        let merged = match s.backfill_pace.get(id) {
            Some(old) if old.day == pace.day => BackfillPace {
                day: pace.day,
                sent_today: old.sent_today.max(pace.sent_today),
                paused_until: pace.paused_until.or(old.paused_until),
                loads_today: old.loads_today.max(pace.loads_today),
                messages_today: old.messages_today.max(pace.messages_today),
            },
            _ if !create => return s.backfill_pace != before,
            _ => pace,
        };
        s.backfill_pace.insert(id.to_string(), merged);
        s.backfill_pace != before
    });
}

/// The day an open backfill's next pass searches from, if a pass has found one.
fn frontier(data_dir: &Path, id: &str) -> Option<i64> {
    MailState::load(data_dir).ok().and_then(|s| s.backfill_frontier.get(id).copied())
}

/// Keeps a pass's frontier for the next: it only moves forward (Quinn 2026-10-19).
fn advance_frontier(data_dir: &Path, id: &str, day: i64) {
    let _ = MailState::update(data_dir, |s| {
        let old = s.backfill_frontier.get(id).copied();
        if old.is_some_and(|o| o >= day) {
            return false;
        }
        s.backfill_frontier.insert(id.to_string(), day);
        true
    });
}

/// A pass ends stalled on separate UTC days before a lone row is left: from the third, it is left at once.
const STALL_DAYS: u32 = 3;

/// The lone row `state` says stalls `id`'s passes, for a pass on UTC day `today`: left at once when today makes
/// the [`STALL_DAYS`]th separate day (review of c8806b3), else only watched (a send or a hold clears it).
pub(super) fn stuck_for(state: &MailState, id: &str, today: i64) -> Option<Stuck> {
    let r = state.lone_stall.get(id)?;
    Some(Stuck { at: r.at, leave: leaves(r, today) })
}

/// Whether the lone row `r` counts is left at once in a pass on UTC day `today`: today makes its third day.
fn leaves(r: &LoneStall, today: i64) -> bool {
    r.days + u32::from(today > r.last_day) >= STALL_DAYS
}

/// The lone rows `state` keeps as left for `id` ([`Job::left`]): received minutes.
pub(super) fn left_for(state: &MailState, id: &str) -> Vec<i64> {
    state.lone_left.get(id).cloned().unwrap_or_default()
}

/// Keeps what a pass that ended on UTC day `today` says of a lone row in `mail.json`: a row in the watched minute
/// sent or held drops the record; a pass that ended stalled on one lone row alone counts `today` for it once (a
/// row in another minute starts a new record). The stall wins over a row sent or held in its own minute (review
/// round 1 of e7ae803: a held row from the same burst must not reset the count of a pass that stalled there).
/// A record whose row is left at once today moves to `lone_left` first, kept there until the backfill closes, so
/// the next stuck row starts its own count and never undoes it (review round 2 of e7ae803).
/// Best effort, like [`keep_pace`].
pub(super) fn note_stall(data_dir: &Path, id: &str, out: &Outcome, today: i64) {
    let _ = MailState::update(data_dir, |s| {
        let before = (s.lone_stall.get(id).copied(), s.lone_left.get(id).cloned());
        let mut old = before.0;
        if let Some(r) = old.filter(|r| leaves(r, today)) {
            let left = s.lone_left.entry(id.to_string()).or_default();
            if !left.contains(&r.at) {
                left.push(r.at);
            }
            old = None;
        }
        // A pass that began before its row was kept as left may still stall on it: that counts for no record.
        let stalled = out.stalled_at.filter(|at| !s.lone_left.get(id).is_some_and(|l| l.contains(at)));
        let new = match (stalled, old) {
            (Some(at), Some(r)) if r.at == at && today > r.last_day => Some(LoneStall { at, days: r.days + 1, last_day: today }),
            (Some(at), Some(r)) if r.at == at => Some(r),
            (Some(at), _) => Some(LoneStall { at, days: 1, last_day: today }),
            (None, _) if out.stuck_cleared => None,
            (None, old) => old,
        };
        match new {
            Some(r) => s.lone_stall.insert(id.to_string(), r),
            None => s.lone_stall.remove(id),
        };
        (s.lone_stall.get(id).copied(), s.lone_left.get(id).cloned()) != before
    });
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
    let blank = || Backfill { since: since.clone(), expected: 0, handled: 0, sent: 0, left: 0, eta_secs: None, resumes_at: None, paused: None };
    for _ in 0..MAX_PASSES {
        if d.shared.stopping() {
            let _ = mark_open(d.data_dir, d.address_id, false);
            return Drove::Stopped;
        }
        let now = d.clock.now();
        let today = now.div_euclid(DAY);
        let next_day = (today + 1) * DAY;
        // A day already spent, or paused by a refused send (this run's, or a relaunch's carried from
        // `mail.json`): wait for the next UTC day before a pass lists or opens anything.
        let held = match routine.next(now) {
            Send::DayLimit { next_day_at } => Some((next_day_at, None)),
            Send::Paused(p) => Some((next_day, Some(p.line()))),
            // The day's page loads spent: a pass would open nothing.
            _ if d.provider == Provider::Gmail && pages_spent(routine, now) => Some((next_day, None)),
            _ => None,
        };
        let held = match held {
            Some(h) => Some(h),
            None => {
                let out = run_one(d, routine, &since, since_idx, now, started);
                keep_pace(d.data_dir, d.address_id, pace_of(routine), true);
                keep_messages(d.data_dir, d.address_id, &routine.message_times(), d.clock.now());
                if let Some(day) = out.frontier.filter(|_| out.stop != Stop::Done) {
                    advance_frontier(d.data_dir, d.address_id, day);
                }
                note_stall(d.data_dir, d.address_id, &out, d.clock.now().div_euclid(DAY));
                if out.closed {
                    (d.on_closed)();
                }
                match out.stop {
                    Stop::Done => {
                        let _ = mark_open(d.data_dir, d.address_id, false);
                        return Drove::Done;
                    }
                    Stop::DayLimit { next_day_at } | Stop::PageLimit { next_day_at } => Some((next_day_at, None)),
                    Stop::Paused(p) => Some((next_day, Some(p.line()))),
                    Stop::Stopped => {
                        let _ = mark_open(d.data_dir, d.address_id, false);
                        return Drove::Stopped;
                    }
                    Stop::Service(name) => return Drove::Failed(name),
                    Stop::Script(word) => return Drove::Failed(word),
                    Stop::Guided | Stop::SlotCap => return Drove::Failed("this mailbox is set up by hand"),
                }
            }
        };
        let Some(wait) = held else { continue };
        d.shared.update(|p| {
            let b = p.backfill.get_or_insert_with(&blank);
            b.resumes_at = Some(wait.0);
            b.paused = wait.1;
        });
        if !wait_until(d, wait.0) {
            let _ = mark_open(d.data_dir, d.address_id, false);
            return Drove::Stopped;
        }
    }
    Drove::Failed("the backfill did not finish")
}

/// What `mail.json` keeps of `routine`'s day: its count, and the next day's start while a refused send
/// holds it.
fn pace_of(routine: &Routine) -> BackfillPace {
    let day = routine.day();
    BackfillPace {
        day,
        sent_today: routine.sent_today(),
        paused_until: routine.is_paused().then_some((day + 1) * DAY),
        loads_today: routine.loads_today(),
        messages_today: routine.messages_today(),
    }
}

/// One pass over the fixed window: progress as it goes, the day's count kept as each batch of forwards is
/// taken (so a quit mid-pass still remembers what went), and its final counts published.
fn run_one(d: &Drive, routine: &mut Routine, since: &str, since_idx: i64, now: i64, started: i64) -> Outcome {
    let today = now.div_euclid(DAY);
    // The routine rolls its day lazily: a count from an earlier day is nothing today.
    let base = if routine.day() == today { routine.sent_today() } else { 0 };
    let watch = |expected: u32, handled: u32, sent: u32| -> bool {
        let at = d.clock.now();
        let elapsed = (at - started).max(0);
        let eta = (handled > 0 && expected > handled).then(|| i64::from(expected - handled) * elapsed / i64::from(handled));
        d.shared.update(|p| {
            p.backfill = Some(Backfill { since: since.to_string(), expected, handled, sent, left: 0, eta_secs: eta, resumes_at: None, paused: None });
        });
        // Past midnight mid-pass the day's own count is not known here: the pass's, an upper bound.
        let sent_today = if at.div_euclid(DAY) == today { base + sent } else { sent };
        let pace = BackfillPace { day: at.div_euclid(DAY), sent_today, paused_until: None, loads_today: 0, messages_today: 0 };
        keep_pace(d.data_dir, d.address_id, pace, true);
        !d.shared.stopping()
    };
    let look = |at: i64| save_look(d.data_dir, d.address_id, at);
    let state = MailState::load(d.data_dir).ok();
    let load = |day: i64, loads: u32, messages: u32, times: &[i64]| {
        let pace = BackfillPace { day, sent_today: 0, paused_until: None, loads_today: loads, messages_today: messages };
        keep_pace(d.data_dir, d.address_id, pace, true);
        keep_messages(d.data_dir, d.address_id, times, d.clock.now());
    };
    let job = Job {
        provider: d.provider,
        address_id: d.address_id,
        knowlu_address: d.knowlu_address,
        phase: Phase::Backfill,
        window: backfill_window(since_idx, now),
        cap: None,
        watch: Some(&watch),
        on_look: Some(&look),
        on_load: Some(&load),
        frontier: frontier(d.data_dir, d.address_id),
        stuck: state.as_ref().and_then(|s| stuck_for(s, d.address_id, today)),
        left: state.as_ref().map(|s| left_for(s, d.address_id)).unwrap_or_default(),
    };
    let out = run_pass(d.mb, d.client, d.clock, routine, &job);
    d.shared.update(|p| {
        p.backfill = Some(Backfill { since: since.to_string(), expected: out.expected, handled: out.handled, sent: out.sent, left: out.left, eta_secs: None, resumes_at: None, paused: None });
    });
    out
}

// ---- resuming at launch (§4.11) ------------------------------------------------------------------

/// A resumed mailbox: opens its kept profile hidden, checks it is signed in and drives its backfill.
/// `Ok(false)` is a mailbox skipped because its backfill is held for C3 (nothing ran); `Err` is the line for the
/// run: a fixed word or a service error's name.
fn resume_one(data_dir: &Path, id: &str, env: &Env, sh: &Shared) -> Result<bool, &'static str> {
    let rows = env.client.list_addresses().map_err(|e| e.name())?;
    let row = rows.iter().find(|r| r["address_id"].as_str() == Some(id)).ok_or("no such mailbox")?;
    let provider = Provider::from_service(row["provider"].as_str().unwrap_or(""));
    // D13: Microsoft 365's retired routine is never driven while C3 is built. A launch does not pick such a
    // mailbox ([`launch_resumable`]); one that reaches here is skipped before any window opens, with no failure
    // and its marker left in `mail.json` for C3 to resume. The guided word is the one a mailbox set up by hand
    // gets.
    if backfill_held_for_c3(provider) {
        return Ok(false);
    }
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
    let mut routine = seed_routine(data_dir, provider, id, env.clock.now());
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
        Drove::Done => Ok(true),
        Drove::Stopped => Err("stopped"),
        Drove::Failed(word) => Err(word),
    }
}

/// The resume's run: each open mailbox in turn, then the run's closing line. A run whose every mailbox was
/// skipped as held for C3 ran nothing: it ends with no outcome, no line and no row (review round 2 of e7ae803),
/// as a launch that picks none starts no run ([`launch_resumable`]).
pub(super) fn run_resume(data_dir: &Path, ids: &[String], env: &Env, sh: &Shared) {
    sh.update(|p| {
        p.rows = vec![Row { step: "backfill", state: "working", reason: None, guided: false }];
        p.sentence = "bringing in your mail";
    });
    let mut first: Option<&'static str> = None;
    let mut ran = false;
    for id in ids {
        if sh.stopping() {
            first.get_or_insert("stopped");
            break;
        }
        match resume_one(data_dir, id, env, sh) {
            Ok(r) => ran |= r,
            Err(line) => {
                first.get_or_insert(line);
            }
        }
    }
    if !ran && first.is_none() {
        sh.update(|p| {
            p.rows.clear();
            p.sentence = "";
        });
        return;
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
