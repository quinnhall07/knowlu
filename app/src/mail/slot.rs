//! The slot's `mail` step (email-forwarding plan T11.5; spec §4.11, §4.10, P18, R7, R17). The app's own step,
//! before the engine chain and after the grades capture: for each mailbox that is due, its kept profile in a
//! **hidden** window, the forwarding setting read, and then T11.2's reconcile and recover over the window.
//! It writes nothing in the vault (the vault changes only through the engine's `judge`) and reports only to
//! the service and to the `RunSummary` line the Settings page reads, never to `state/runner-log.md`.
//!
//! **Whether it runs** is the scheduler's `mail_step` decision (a named skip at exit 0, or no row at all
//! with no mailbox connected); [`reconcile_due_with`] is its cadence. **What it does** is [`run_step`], over
//! the [`Host`] window seam, so a test drives it with a fake window and a loopback service.
//!
//! **Per mailbox.** A mailbox the student turned off in Settings is skipped before its profile is opened, now
//! and ever (§13 Q11; `mail.json` is read per mailbox so a Disconnect mid-slot counts). A profile that is
//! missing or does not read as signed in is `needs sign-in`: the window was opened hidden and is closed again,
//! never shown. Forwarding is then read: on to Knowlu's address is nothing to do; **off** is reported `off`,
//! turned back on with setup's script (the address already in the list, nothing replaced), read back after a
//! navigation, a canary sent and `reenabled` reported; **pointing to another address** is reported
//! `elsewhere` and changed in no way (only the student's yes acts, through the progress view). Reconcile and
//! recover follow either way, reaching back to when forwarding was last known on after it was found off.
//!
//! **Privacy.** A line holds fixed words and counts. No address, subject, sender, URL or page text enters one.
use super::backfill::{keep_pace, left_for, note_stall, save_look, seed_routine, stuck_for};
use super::commands::{run_script, wait_signed_in, GONE_WAIT, SILENT_WAIT};
use super::host::Host;
use super::recover::{run_pass, window, Clock, Job, Outcome, Place, Stop, SLOT_CAP};
use crate::mail_client::{MailClient, Phase, State};
use crate::mail_session::{session_dir, valid_address_id, BackfillPace, MailState};
use crate::mail_steps::{backfill_held_for_c3, Provider};
use serde_json::{json, Value};
use std::path::Path;

// ---- the cadence (§13 Q3, R7) ---------------------------------------------------------------------------

/// How often a mailbox's pass is due. `Daily` is the shipped value until S4's revisit, which changes only
/// this constant.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Cadence {
    /// At most once a day for a healthy mailbox, and at once for one that needs repair.
    Daily,
    /// Only for a mailbox that needs repair.
    RepairOnly,
}

pub const RECONCILE_CADENCE: Cadence = Cadence::Daily;

/// "Once a day" is a gap, not a calendar day: slots are twice a day at times that drift by minutes.
const DAILY_GAP: i64 = 20 * 3600;

/// A state the saved session repairs: forwarding went quiet or off, or the account lapsed and mail was
/// dropped. `pending`, `confirmed`, `blocked` and `paused_send` are setup's (a blocked mailbox must not have
/// old mail brought in while new mail never arrives, D16).
pub fn needs_repair(status: &str) -> bool {
    matches!(status, "quiet" | "off" | "lapsed")
}

fn instant(s: &str) -> Option<i64> {
    s.parse::<jiff::Timestamp>().ok().map(|t| t.as_second())
}

/// Whether a mailbox is due at unix second `now`, `last` being its last whole pass (RFC 3339).
pub fn reconcile_due_with(cadence: Cadence, last: Option<&str>, status: &str, now: i64) -> bool {
    if needs_repair(status) {
        return true;
    }
    match cadence {
        Cadence::RepairOnly => false,
        // A last pass dated in the future is a clock that moved: due, never stuck until then.
        Cadence::Daily => status == "active" && last.and_then(instant).is_none_or(|t| t > now || now - t >= DAILY_GAP),
    }
}

/// [`reconcile_due_with`] at the shipped cadence.
pub fn reconcile_due(last: Option<&str>, status: &str, now: i64) -> bool {
    reconcile_due_with(RECONCILE_CADENCE, last, status, now)
}

// ---- the pass -------------------------------------------------------------------------------------------

/// What one run of the step works with.
pub struct SlotEnv<'a> {
    pub host: &'a dyn Host,
    pub client: &'a MailClient,
    pub clock: &'a dyn Clock,
    /// This profile's app-data folder: the kept profiles and `mail.json`.
    pub data_dir: &'a Path,
}

enum Part {
    NeedsSignIn,
    Line(String),
}

fn label(p: Provider) -> &'static str {
    if p == Provider::Gmail { "Gmail" } else { "school email" }
}

/// The `RunSummary` row for the mailboxes that ran: spec §4.11 step 5's line, or `mail (needs sign-in)`.
/// `None` when nothing ran (every mailbox was one the step leaves alone).
///
/// **A sign-out or a deletion stops it** (spec §4.13): once their close has called `mail::stop_runs`, the step
/// opens no further mailbox's profile, so nothing is open on a profile they are about to delete.
pub fn run_step(env: &SlotEnv, due: &[Value]) -> Option<String> {
    let run = crate::mail::Run::begin();
    let mut parts: Vec<(Provider, Part)> = Vec::new();
    for row in due {
        if run.stopped() {
            break;
        }
        let done = mailbox(env, row, run);
        // The next profile opens, and the step lets go of the mail window, only once this window is gone: after a
        // pass, and after a stop that closed a window the moment it opened (WebView2 destroys a window
        // asynchronously, and a sign-out deletes the profile as soon as it has the hold, spec §4.13).
        let end = env.clock.now() + GONE_WAIT;
        while env.host.window_open() && env.clock.now() < end {
            env.clock.sleep(1);
        }
        if let Some(done) = done {
            parts.push(done);
        }
    }
    match parts.as_slice() {
        [] => None,
        all if all.iter().all(|(_, p)| matches!(p, Part::NeedsSignIn)) => Some("mail (needs sign-in)".to_string()),
        [(_, Part::Line(l))] => Some(format!("mail: {l}")),
        many => {
            let each = |(p, part): &(Provider, Part)| match part {
                Part::NeedsSignIn => format!("{}: needs sign-in", label(*p)),
                Part::Line(l) => format!("{}: {l}", label(*p)),
            };
            Some(format!("mail: {}", many.iter().map(each).collect::<Vec<_>>().join("; ")))
        }
    }
}

/// One mailbox, §4.11 steps 1 to 5. `None` is a mailbox the step leaves alone: turned off, guided, or a row
/// it cannot read. A `mail.json` that cannot be read is also left alone, never guessed at (`turned_off`).
/// A stopped `run` opens nothing; a stop that comes while the window opens closes it again before anything
/// runs in it, and [`run_step`] waits until it is gone before the step lets go of the mail window.
fn mailbox(env: &SlotEnv, row: &Value, run: crate::mail::Run) -> Option<(Provider, Part)> {
    let id = row["address_id"].as_str().filter(|i| valid_address_id(i))?;
    let provider = Provider::from_service(row["provider"].as_str().unwrap_or(""));
    let state = MailState::load(env.data_dir).ok()?;
    if provider == Provider::Guided || state.is_turned_off(id) {
        return None;
    }
    let knowlu = row["address"].as_str().filter(|a| !a.is_empty())?;
    let Some(profile) = session_dir(env.data_dir, id).filter(|p| p.is_dir()) else { return Some((provider, Part::NeedsSignIn)) };
    env.host.use_provider(provider);
    if run.stopped() {
        return None;
    }
    // Hidden, always: a signed-out profile is a row and a Settings repair, never a window on the desktop.
    if env.host.open_window(&profile, false).is_err() {
        return Some((provider, Part::Line("the mail window would not open".to_string())));
    }
    if run.stopped() {
        env.host.close_window();
        return None;
    }
    let part = if wait_signed_in(env.host, env.clock, SILENT_WAIT) {
        Part::Line(pass(env, id, provider, knowlu, row, &state))
    } else {
        Part::NeedsSignIn
    };
    env.host.close_window();
    Some((provider, part))
}

fn pass(env: &SlotEnv, id: &str, provider: Provider, knowlu: &str, row: &Value, state: &MailState) -> String {
    let now = env.clock.now();
    let fwd = forwarding(env, id, provider, knowlu);
    // D13 and C3: no Microsoft 365 recover pass while the retired routine is the only one there is. The
    // forwarding check above is not retired; the pass, its reconcile and its "last known on" are not made, and
    // an open backfill stays open in `mail.json` for C3.
    if backfill_held_for_c3(provider) {
        return fwd.text().to_string();
    }
    let last_known_on = state.last_reconcile.get(id).and_then(|t| instant(t)).filter(|_| fwd.was_not_on());
    let look = |at: i64| save_look(env.data_dir, id, at);
    let backfilling = state.open_backfills.iter().any(|i| i == id);
    // The page loads and messages go only into a pace this day already keeps (a backfill's, or a slot's that sent), so a
    // mailbox that only recovers adds nothing to `mail.json` for them.
    let load = |day: i64, loads: u32, messages: u32| {
        let pace = BackfillPace { day, sent_today: 0, paused_until: None, loads_today: loads, messages_today: messages };
        keep_pace(env.data_dir, id, pace, false);
    };
    let job = Job {
        provider,
        address_id: id,
        knowlu_address: knowlu,
        phase: Phase::Recover,
        window: window(now, row["lapsed_since"].as_str().and_then(instant), last_known_on),
        // An open backfill is not bound by the slot (§4.11); any other pass sends at most 30.
        cap: (!backfilling).then_some(SLOT_CAP),
        watch: None,
        on_look: Some(&look),
        on_load: Some(&load),
        frontier: None,
        // A lone row that stalled passes is watched here too, and left from its third day (review of c8806b3).
        stuck: stuck_for(state, id, now.div_euclid(86_400)),
        left: left_for(state, id),
    };
    let mut routine = seed_routine(env.data_dir, provider, id, now);
    let out = run_pass(env.host, env.client, env.clock, &mut routine, &job);
    note_stall(env.data_dir, id, &out, env.clock.now().div_euclid(86_400));
    // The day's sends count for the mailbox whoever made them: a backfill seeds its pace from here, so a slot's
    // sends are kept beside it (they once were not, and a slot and a backfill could pass the day's cap together).
    if out.sent > 0 || backfilling {
        let day = routine.day();
        let pace = BackfillPace {
            day,
            sent_today: routine.sent_today(),
            paused_until: routine.is_paused().then_some((day + 1) * 86_400),
            loads_today: routine.loads_today(),
            messages_today: routine.messages_today(),
        };
        keep_pace(env.data_dir, id, pace, true);
    }
    // Only a whole pass moves the day on: one that stopped early is due again at the next slot. And only one
    // that read the forwarding setting: a failed read is no proof forwarding was on, so it must not move
    // "last known on" forward (§4.11: recovery reaches back to when it last was).
    if out.stop == Stop::Done && !matches!(fwd, Fwd::Unchecked) {
        let at = jiff::Timestamp::from_second(now).map(|t| t.to_string()).unwrap_or_default();
        let _ = MailState::update(env.data_dir, |s| {
            s.last_reconcile.insert(id.to_string(), at);
            true
        });
    }
    format!("{}, {}", fwd.text(), recovered(&out))
}

fn recovered(out: &Outcome) -> String {
    let n = format!("{} recovered", out.sent);
    match &out.stop {
        Stop::Done | Stop::Guided | Stop::Stopped => n,
        Stop::SlotCap => format!("{n}, more next slot"),
        Stop::DayLimit { .. } => format!("{n}, daily limit reached"),
        Stop::PageLimit { .. } => format!("{n}, page limit reached"),
        Stop::Paused(_) => format!("{n}, paused: a send was refused"),
        Stop::Script(word) | Stop::Service(word) => format!("{n}, stopped: {word}"),
    }
}

// ---- the forwarding setting (§4.11 step 2) ---------------------------------------------------------------

enum Fwd {
    On,
    TurnedBackOn,
    /// Off, and not turned back on (the page would not let a script, or the read-back did not show it).
    OffStill,
    Elsewhere,
    Unchecked,
}

impl Fwd {
    fn text(&self) -> &'static str {
        match self {
            Fwd::On => "forwarding on",
            Fwd::TurnedBackOn => "forwarding was off, turned back on",
            Fwd::OffStill => "forwarding is off",
            Fwd::Elsewhere => "forwarding goes to another address",
            Fwd::Unchecked => "forwarding not checked",
        }
    }

    /// Mail sent while forwarding was not reaching Knowlu: the window reaches back to when it last did.
    fn was_not_on(&self) -> bool {
        matches!(self, Fwd::TurnedBackOn | Fwd::OffStill | Fwd::Elsewhere)
    }
}

#[derive(PartialEq)]
enum Reading {
    Ours,
    Elsewhere,
    Off,
    Unknown,
}

fn reading(v: &Value) -> Reading {
    if v["result"] != "ok" {
        return Reading::Unknown;
    }
    match (v["forwarding"].as_str(), v["to"].as_str()) {
        // Another address is never overwritten in a hidden step, on or off (§4.2 step 3's rule).
        (Some("on" | "off"), Some("elsewhere")) => Reading::Elsewhere,
        (Some("on"), Some("ours")) => Reading::Ours,
        (Some("on" | "off"), _) => Reading::Off,
        _ => Reading::Unknown,
    }
}

fn read(env: &SlotEnv, provider: Provider, knowlu: &str) -> Option<Value> {
    let script = if provider == Provider::Gmail { "gmail/forwarding_read.js" } else { "m365/forwarding_read.js" };
    env.host.open(Place::Forwarding).ok()?;
    run_script(env.host, provider, script, json!({ "address": knowlu }))
}

fn forwarding(env: &SlotEnv, id: &str, provider: Provider, knowlu: &str) -> Fwd {
    match read(env, provider, knowlu).as_ref().map_or(Reading::Unknown, reading) {
        Reading::Ours => Fwd::On,
        Reading::Elsewhere => {
            let _ = env.client.set_state(id, State::Elsewhere);
            Fwd::Elsewhere
        }
        Reading::Off => turn_back_on(env, id, provider, knowlu),
        Reading::Unknown => Fwd::Unchecked,
    }
}

/// Off: reported `off`, set with setup's script (nothing replaced), read back after leaving the page and
/// returning, then a canary and `reenabled`. The engine's next mail pass opens the one heads-up from the stamp.
fn turn_back_on(env: &SlotEnv, id: &str, provider: Provider, knowlu: &str) -> Fwd {
    let _ = env.client.set_state(id, State::Off);
    let script = if provider == Provider::Gmail { "gmail/forwarding_set.js" } else { "m365/forwarding_set.js" };
    let set = env.host.open(Place::Forwarding).is_ok()
        && run_script(env.host, provider, script, json!({ "address": knowlu, "replace": false })).is_some_and(|v| v["result"] == "ok");
    let back = set
        && env.host.open(Place::Inbox).is_ok()
        && read(env, provider, knowlu).as_ref().map(reading) == Some(Reading::Ours);
    if !back {
        return Fwd::OffStill;
    }
    let _ = env.client.canary(id);
    let _ = env.client.set_state(id, State::Reenabled);
    Fwd::TurnedBackOn
}
