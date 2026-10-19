//! The seams behind the ten console commands (email-forwarding plan T11.4; spec §6.2, §4.12, D6, D15).
//! `mail.rs` holds the `#[tauri::command]` wrappers; each is a few lines that gather what a seam needs from
//! `ConsoleState` and the live window and call it, so nothing here touches Tauri and a test drives every one
//! with the setup runner's fake window, a loopback service and a fake clock. Nothing computes anything about
//! mail: the status joins the service's rows to what this device keeps, `mail_applied` hands the engine's own
//! JSON through, and the vault is never written (the vault changes only through the engine's `judge`).
//!
//! **Privacy.** A mailbox's address is shown in the Settings row, so it travels in these answers, in memory
//! only. No answer, error or step message holds a token, a hint, a URL or a word a page said: the steps carry
//! fixed sentences, and an error is a name.
use super::host::Host;
use super::recover::{Clock, Place};
use super::setup::{Answer, Kind, Shared};
use super::Refusal;
use crate::mail_client::{MailClient, MailError};
use crate::mail_session::{forget, forget_all, forget_setup, session_dir, setup_dir, turn_off, valid_address_id, MailState};
use crate::mail_steps::Provider;
use serde_json::{json, Map, Value};
use std::path::Path;
use std::sync::Arc;

/// `{ok: false, error}`, the shape every refusal here takes.
fn refused(why: &str) -> Value {
    json!({ "ok": false, "error": why })
}

// ---- mail_status ------------------------------------------------------------------------------------

/// `mail_status`: the mailboxes the service lists (`list`, or the name of why it could not), each joined to
/// what this device keeps (a saved sign-in, an open backfill, turned off in Settings), the saved-sign-in
/// switch, whether the wizard's offer is still pending (D6: its marker, cleared when setup starts, never
/// written for a student who chose Not now) and whether a setup is running. A service that cannot be read
/// still answers the device's own facts.
pub fn status_for(list: Result<Vec<Value>, String>, data_dir: &Path, setup_running: bool) -> Value {
    let (state, local_error) = match MailState::load(data_dir) {
        Ok(s) => (s, None),
        Err(_) => (MailState::default(), Some("mail.json is unreadable")),
    };
    let (ok, error, rows) = match list {
        Ok(rows) => (true, Value::Null, rows),
        Err(name) => (false, json!(name), Vec::new()),
    };
    // An address handed to the console (Settings' Copy, a guided step's line) may be put in the mailbox by
    // hand (review of a7f1ca0). With no setup running, a record a quit setup left that says never placed is the
    // start-up clean's to remove, and this read can come first (the console reads it at every launch): its
    // address is withheld, so Settings offers no Copy. While a setup runs, its guided step needs the address:
    // it is handed out, and the record is marked so the start-up clean never touches it.
    let unplaced = |id: &str| state.pending_setups.iter().any(|p| p.address_id == id && !(p.placed || p.backfilled || p.shown));
    let id_of = |r: &Value| r["address_id"].as_str().unwrap_or("").to_string();
    let rows: Vec<Value> = if setup_running {
        let ids: Vec<String> = rows.iter().filter(|r| r["address"].as_str().is_some_and(|a| !a.is_empty())).map(id_of).collect();
        // Whatever `placed` reads now: a run lowers it again after a script that changed nothing.
        if state.pending_setups.iter().any(|p| !p.shown && ids.contains(&p.address_id)) {
            super::setup::mark_shown(data_dir, ids.iter().map(String::as_str));
        }
        rows
    } else {
        let withhold = |mut r: Value| {
            if unplaced(&id_of(&r)) {
                r["address"] = Value::Null;
            }
            r
        };
        rows.into_iter().map(withhold).collect()
    };
    let mailboxes: Vec<Value> = rows
        .into_iter()
        .map(|row| {
            let mut m: Map<String, Value> = row.as_object().cloned().unwrap_or_default();
            let id = m.get("address_id").and_then(Value::as_str).unwrap_or("").to_string();
            m.insert("state".into(), m.get("status").cloned().unwrap_or(Value::Null));
            m.insert("turned_off".into(), json!(state.is_turned_off(&id)));
            m.insert("open_backfill".into(), json!(state.open_backfills.contains(&id)));
            m.insert("saved_sign_in".into(), json!(session_dir(data_dir, &id).is_some_and(|d| d.is_dir())));
            Value::Object(m)
        })
        .collect();
    json!({
        "ok": ok, "error": error, "mailboxes": mailboxes,
        "keep_session": state.keep_session,
        "offer_pending": crate::onboarding::mail_offer_marker(data_dir).is_file(),
        "setup_running": setup_running,
        "profile_delete_pending": !state.pending_deletes.is_empty(),
        "local_error": local_error,
    })
}

// ---- mail_setup_start, _progress, _answer, _stop ----------------------------------------------------

/// `mail_setup_start(provider)`: `school` (the curated row of the vault's campus, else the setup asks once
/// for the address) or `gmail`. `start` begins the run; only a run that began answers the wizard's offer, so
/// a refused start leaves the offer standing.
pub fn setup_start_with(data_dir: &Path, provider: &str, unitid: Option<String>, start: impl FnOnce(Kind) -> Result<(), &'static str>) -> Value {
    let kind = match provider {
        "school" => Kind::School { unitid, address: None },
        "gmail" => Kind::Gmail,
        _ => return refused("not a mail provider"),
    };
    match start(kind) {
        Ok(()) => {
            let _ = std::fs::remove_file(crate::onboarding::mail_offer_marker(data_dir));
            json!({ "ok": true, "error": Value::Null })
        }
        Err(why) => refused(why),
    }
}

/// `mail_setup_progress`: the run's progress rows (polled), and whether it is still going.
pub fn progress_of(current: Option<Arc<Shared>>) -> Value {
    match current {
        Some(run) => json!({ "ok": true, "active": !run.is_done(), "progress": run.progress() }),
        None => json!({ "ok": true, "active": false, "progress": Value::Null }),
    }
}

/// `mail_setup_answer(choice)`: `"replace"` forwards to Knowlu instead, `"stop"` keeps what the mailbox
/// forwards to now and ends, `"done"` is "I've done this", `"retry"` retries a blocked canary, and
/// `{"school_address": "<address>"}` answers the one question of an uncurated school. There is no date to
/// give: the window is fixed (BW). An answer the open question cannot take is refused.
pub fn answer_with(current: Option<Arc<Shared>>, choice: &Value) -> Value {
    let Some(run) = current else { return refused("no setup is running") };
    let answer = match choice {
        Value::String(s) => match s.as_str() {
            "replace" => Answer::Replace(true),
            "stop" => Answer::Replace(false),
            "done" => Answer::Done,
            "retry" => Answer::Retry,
            _ => return refused("not an answer"),
        },
        Value::Object(o) => match o.get("school_address").and_then(Value::as_str) {
            Some(a) => Answer::SchoolAddress(a.to_string()),
            None => return refused("not an answer"),
        },
        _ => return refused("not an answer"),
    };
    if run.answer(answer) { json!({ "ok": true, "error": Value::Null }) } else { refused("that answer does not fit the question") }
}

/// `mail_setup_stop`: after the step in hand (§4.5).
pub fn stop_with(current: Option<Arc<Shared>>) -> Value {
    match current {
        Some(run) => {
            run.stop();
            json!({ "ok": true, "error": Value::Null })
        }
        None => refused("no setup is running"),
    }
}

// ---- mail_rotate ------------------------------------------------------------------------------------

/// `mail_rotate(address_id)`: `row` is the mailbox's row from the service. A mailbox the student turned off
/// is never set going again, and one the service does not list is not there to rotate.
pub fn rotate_with(data_dir: &Path, address_id: &str, row: Option<&Value>, start: impl FnOnce(Kind) -> Result<(), &'static str>) -> Value {
    if !valid_address_id(address_id) {
        return refused("not a mailbox");
    }
    if MailState::load(data_dir).is_ok_and(|s| s.is_turned_off(address_id)) {
        return refused("email is turned off for this mailbox");
    }
    let Some(provider) = row.and_then(|r| r["provider"].as_str()) else { return refused("no such mailbox") };
    match start(Kind::Rotate { address_id: address_id.to_string(), provider: provider.to_string() }) {
        Ok(()) => json!({ "ok": true, "error": Value::Null }),
        Err(why) => refused(why),
    }
}

// ---- mail_keep_session ------------------------------------------------------------------------------

/// `mail_keep_session(on)` (D15): the switch is recorded first, so a delete that fails below still leaves it
/// off. Off closes the mail window (`close`) and deletes every `mail-session-*` profile and `mail-setup`,
/// through `mail_session`'s guard, and nothing else. A `mail.json` that cannot be read or written changes
/// nothing, because the switch could not be kept.
pub fn keep_session_with(data_dir: &Path, on: bool, close: impl FnOnce()) -> Value {
    let saved = MailState::update(data_dir, |s| {
        let changed = s.keep_session != on;
        s.keep_session = on;
        changed
    });
    if saved.is_err() {
        return refused("mail.json is unreadable");
    }
    let left = if on {
        false
    } else {
        close();
        forget_all(data_dir) != Ok(true)
    };
    json!({ "ok": true, "error": Value::Null, "keep_session": on, "profiles_left": left })
}

// ---- mail_applied -----------------------------------------------------------------------------------

/// `knowlu-engine mail-applied --vault <v> [--since <ts>]`.
pub fn applied_argv(vault: &Path, since: Option<&str>) -> Vec<String> {
    let mut argv = vec!["mail-applied".to_string(), "--vault".to_string(), vault.to_string_lossy().into_owned()];
    if let Some(s) = since {
        argv.extend(["--since".to_string(), s.to_string()]);
    }
    argv
}

/// `mail_applied`: the engine's JSON, passed through as it printed it. `run` takes the argv and answers the
/// engine's stdout. A report that is not one JSON object is `{ok: false, error}`, never a guess.
pub fn applied_with(vault: &Path, since: Option<&str>, run: impl FnOnce(&[String]) -> Result<String, String>) -> Value {
    match run(&applied_argv(vault, since)) {
        Ok(out) => match serde_json::from_str::<Value>(out.trim()) {
            Ok(v) if v.is_object() => v,
            _ => refused("mail-applied printed no report"),
        },
        Err(why) => refused(&why),
    }
}

/// Runs the engine for [`applied_with`]. It reads the vault and writes nothing, and it is never gated.
pub fn run_engine(argv: &[String]) -> Result<String, String> {
    use knowlu_engine::childproc::NoConsole;
    let exe = crate::scheduler::engine_exe()?;
    let out = std::process::Command::new(exe).no_console().args(argv).output().map_err(|e| format!("could not run the engine ({})", e.kind()))?;
    if !out.status.success() {
        return Err(format!("mail-applied exited with status {}", out.status.code().map_or("unknown".to_string(), |c| c.to_string())));
    }
    String::from_utf8(out.stdout).map_err(|_| "mail-applied printed no report".to_string())
}

// ---- mail_sign_in -----------------------------------------------------------------------------------

/// How long the sign-in may take (the school's SSO, Duo, the phone tap), as the runner waits.
const SIGN_IN_WAIT: i64 = 15 * 60;
/// How long a hidden window on a saved sign-in is given to read as the mailbox before the student is asked.
pub(super) const SILENT_WAIT: i64 = 30;
/// How long a closed window is waited for to be gone before the next one opens.
pub(super) const GONE_WAIT: i64 = 10;

/// `mail_sign_in(address_id)`, a repair: the visible window on the mailbox's kept profile until the page
/// reads as a signed-in mailbox, then it closes and the sign-in is kept for the next slot. A window the
/// student closes first is a sign-in that did not happen, named; the profile stays either way.
pub fn sign_in_with(host: &dyn Host, clock: &dyn Clock, data_dir: &Path, provider: Provider, address_id: &str) -> Value {
    let not_signed_in = |why: &str| json!({ "ok": false, "signed_in": false, "error": why });
    if provider == Provider::Guided {
        return not_signed_in("this mailbox is set up by hand");
    }
    let Some(profile) = session_dir(data_dir, address_id) else { return not_signed_in("not a mailbox") };
    host.use_provider(provider);
    if host.open_window(&profile, true).is_err() {
        return not_signed_in("the mail window would not open");
    }
    let signed_in = wait_signed_in(host, clock, SIGN_IN_WAIT);
    host.close_window();
    if signed_in {
        json!({ "ok": true, "signed_in": true, "error": Value::Null })
    } else {
        not_signed_in("sign-in was not completed")
    }
}

/// Polls until the page reads as the mailbox (`true`), the window is gone or `limit` seconds pass.
pub(super) fn wait_signed_in(host: &dyn Host, clock: &dyn Clock, limit: i64) -> bool {
    let end = clock.now() + limit;
    loop {
        if host.signed_in() {
            return true;
        }
        if !host.window_open() || clock.now() >= end {
            return false;
        }
        clock.sleep(1);
    }
}

// ---- mail_disconnect (§4.12) -------------------------------------------------------------------------

/// What Disconnect works with: the window, the service, the clock and this profile's folder.
pub struct Disconnect<'a> {
    pub host: &'a dyn Host,
    pub client: &'a MailClient,
    pub clock: &'a dyn Clock,
    pub data_dir: &'a Path,
}

const GUIDED: &str = "Turn forwarding off in your mail settings by hand; Knowlu's address is shown below.";
const STILL_IN_GMAIL: &str = "Knowlu's address is still in your Gmail forwarding list; remove it there by hand.";
const LEFT_AS_IS: &str = "Forwarding goes to another address, so it was left as it is.";

fn step(name: &str, state: &str, message: Option<&str>) -> Value {
    json!({ "step": name, "state": state, "message": message })
}

/// **Disconnect, §4.12.** 0: the mailbox is recorded in `turned_off` before anything else, so nothing later
/// (a slot, a retry, a failed step below) ever turns its forwarding back on. 1: forwarding off with the saved
/// sign-in (hidden), the visible window if signed out, and on Gmail the address removed from the list; a
/// declined sign-in or a step that cannot go on is a guided step with the address shown, and a Gmail removal
/// that the live check will not make is a needs-you that leaves the address where it is: not retried, not
/// clicked another way, never "done". 2: `DELETE /mail/addresses/<id>`; a failure leaves the profile so
/// Disconnect can run again. 3: the window closes and `mail-session-<id>` is deleted (a failed delete is
/// retried at start-up and named in Settings).
pub fn disconnect_with(d: &Disconnect, address_id: &str) -> Value {
    if turn_off(d.data_dir, address_id).is_err() {
        return json!({ "ok": false, "error": "could not record the mailbox as turned off; nothing was changed",
                       "steps": [], "needs_you": false, "address": Value::Null });
    }
    let rows = match d.client.list_addresses() {
        Ok(rows) => rows,
        Err(e) => return json!({ "ok": false, "error": e.name(), "steps": [], "needs_you": false, "address": Value::Null }),
    };
    let row = rows.into_iter().find(|r| r["address_id"].as_str() == Some(address_id));
    let provider = row.as_ref().map_or(Provider::Guided, |r| Provider::from_service(r["provider"].as_str().unwrap_or("")));
    let address = row.as_ref().and_then(|r| r["address"].as_str()).filter(|a| !a.is_empty()).map(str::to_string);
    let mut steps = match (&address, provider) {
        (Some(a), p) if p != Provider::Guided => forwarding_steps(d, address_id, p, a),
        // The service no longer lists it: nothing is left to turn off.
        _ if row.is_none() => Vec::new(),
        _ => vec![step("forwarding_off", "guided", Some(GUIDED))],
    };
    let needs_you = steps.iter().any(|s| matches!(s["state"].as_str(), Some("guided" | "needs_you")));
    let shown = address.filter(|_| needs_you);
    match d.client.delete_address(address_id) {
        Ok(_) | Err(MailError::NotFound) => steps.push(step("revoke", "done", None)),
        Err(e) => {
            steps.push(step("revoke", "failed", Some(&e.to_string())));
            return json!({ "ok": false, "error": e.name(), "steps": steps, "needs_you": needs_you, "address": shown });
        }
    }
    d.host.close_window();
    let _ = MailState::update(d.data_dir, |s| {
        // Every per-mailbox entry goes (review of c8806b3: the frontier was once left behind); the day's Gmail
        // message count lives in the pace.
        let before = s.clone();
        s.open_backfills.retain(|i| i != address_id);
        s.backfill_pace.remove(address_id);
        s.last_reconcile.remove(address_id);
        s.last_look.remove(address_id);
        s.backfill_frontier.remove(address_id);
        s.lone_stall.remove(address_id);
        s.lone_left.remove(address_id);
        s.gmail_messages.remove(address_id);
        *s != before
    });
    steps.push(match forget(d.data_dir, address_id) {
        Ok(true) => step("profile", "done", None),
        Ok(false) => step("profile", "pending", Some("The saved sign-in could not be deleted yet; Knowlu tries again when it starts.")),
        Err(_) => step("profile", "failed", None),
    });
    json!({ "ok": true, "error": Value::Null, "steps": steps, "needs_you": needs_you, "address": shown })
}

/// Step 1 in the mailbox's window: signs in (silently, else visibly), turns forwarding off and, on Gmail,
/// removes the address. The window is closed after, and a throwaway profile deleted.
fn forwarding_steps(d: &Disconnect, id: &str, provider: Provider, address: &str) -> Vec<Value> {
    d.host.use_provider(provider);
    let kept = session_dir(d.data_dir, id).filter(|p| p.is_dir());
    let profile = kept.clone().unwrap_or_else(|| setup_dir(d.data_dir));
    let steps = if signed_in(d, &profile) { run_steps(d, provider, address) } else { vec![step("forwarding_off", "guided", Some(GUIDED))] };
    d.host.close_window();
    if kept.is_none() {
        let _ = forget_setup(d.data_dir);
    }
    steps
}

/// A hidden window on the saved sign-in first; if it does not read as the mailbox, the visible window for
/// the student. `false` when the student declines (closes it, or does not finish).
fn signed_in(d: &Disconnect, profile: &Path) -> bool {
    if d.host.open_window(profile, false).is_ok() {
        if wait_signed_in(d.host, d.clock, SILENT_WAIT) {
            return true;
        }
        d.host.close_window();
        let end = d.clock.now() + GONE_WAIT;
        while d.host.window_open() && d.clock.now() < end {
            d.clock.sleep(1);
        }
    }
    if d.host.open_window(profile, true).is_err() {
        return false;
    }
    if wait_signed_in(d.host, d.clock, SIGN_IN_WAIT) {
        let _ = d.host.step_aside();
        return true;
    }
    false
}

fn result_of(v: &Option<Value>) -> &str {
    v.as_ref().and_then(|v| v["result"].as_str()).unwrap_or("")
}

fn run_steps(d: &Disconnect, provider: Provider, address: &str) -> Vec<Value> {
    let mut steps = Vec::new();
    let off_script = if provider == Provider::M365 { "m365/forwarding_off.js" } else { "gmail/forwarding_off.js" };
    if d.host.open(Place::Forwarding).is_err() {
        return vec![step("forwarding_off", "guided", Some(GUIDED))];
    }
    let off = run_script(d.host, provider, off_script, json!({ "address": address }));
    match result_of(&off) {
        "ok" => steps.push(step("forwarding_off", "done", None)),
        "elsewhere" => steps.push(step("forwarding_off", "done", Some(LEFT_AS_IS))),
        _ => return vec![step("forwarding_off", "guided", Some(GUIDED))],
    }
    if provider == Provider::Gmail {
        let removed = run_script(d.host, provider, "gmail/forwarding_remove_address.js", json!({ "address": address }));
        steps.push(match result_of(&removed) {
            "ok" => step("remove_address", "done", None),
            // The live check will not make the removal (plan T10.3a review, R4): it is left, said, and not tried again.
            "needs you" => step("remove_address", "needs_you", Some(STILL_IN_GMAIL)),
            _ => step("remove_address", "guided", Some(GUIDED)),
        });
    }
    steps
}

/// One step script, run again once with `reloaded: true` when a Gmail page reloads under it (as the setup
/// runner does); `None` when the window refused. A second reload is the page's answer, which is no `ok`.
pub(super) fn run_script(host: &dyn Host, provider: Provider, script: &str, mut args: Value) -> Option<Value> {
    for attempt in 0..2 {
        let r = host.script(script, &args);
        let reloading = match &r {
            Ok(v) => v["result"].as_str() == Some("reloading"),
            Err(Refusal::Lost) => provider == Provider::Gmail,
            Err(_) => false,
        };
        if reloading && attempt == 0 {
            args["reloaded"] = json!(true);
            let _ = host.open(Place::Forwarding);
            continue;
        }
        return r.ok();
    }
    None
}
