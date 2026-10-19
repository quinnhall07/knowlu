//! The mail setup's kept browser profiles and `mail.json` (email-forwarding plan T9.1, spec §5.4
//! and D15). Pure file work: nothing here opens a window, reaches a network or logs.
//!
//! **The profiles.** One kept WebView2 profile per mailbox, `<data_dir>\mail-session-<address_id>`,
//! and with the saved sign-in off one throwaway `<data_dir>\mail-setup`. [`is_session_dir`] is the
//! one guard every delete passes, grades' rule widened to these two shapes: the exact child of an
//! absolute `data_dir` with no `..`, named `mail-setup` or `mail-session-` plus an address id. An
//! address id is the service's opaque mailbox id, never the address: 1 to 64 ASCII lowercase
//! letters, digits or `-` (a UUID fits), so no id can name a separator, a `..` or a second folder.
//! A delete that fails is recorded in `mail.json`'s `pending_deletes` and retried by
//! [`start_up_sweep`], which also removes a `mail-setup` a crash left. A delete that fails and
//! cannot be recorded is an error, never a quiet `false`.
//!
//! **`mail.json`** sits beside `settings.json` and is never a field in it (the `grades.json`
//! precedent). Unlike `grades.json`, an unreadable file is an error, never the default: its
//! `turned_off` only grows (§13 Q11), so reading a corrupt file as "nothing turned off" could turn
//! back on a mailbox the student turned off, and reading it as "nothing pending" would leave a
//! signed-in profile unnamed. Every write refuses rather than overwrite it, and the one write path
//! is [`MailState::update`], which reloads under the lock.
use std::collections::BTreeMap;
use std::path::{Component, Path, PathBuf};
use std::sync::{Mutex, MutexGuard};
use std::time::Duration;

const SESSION_PREFIX: &str = "mail-session-";
const SETUP_DIR: &str = "mail-setup";
const MAIL_JSON: &str = "mail.json";
const MAX_ADDRESS_ID: usize = 64;

/// `grades`'s patience: WebView2 lets go of a profile a moment after its window is gone.
const WIPE_TRIES: u32 = 12;
const WIPE_BACKOFF: Duration = Duration::from_millis(250);

/// Serialises every read-modify-write of `mail.json` in this process.
static MAIL_JSON_LOCK: Mutex<()> = Mutex::new(());

fn lock() -> MutexGuard<'static, ()> {
    MAIL_JSON_LOCK.lock().unwrap_or_else(|p| p.into_inner())
}

/// Whether `id` can be an address id: 1 to 64 of `a-z`, `0-9` and `-`.
pub fn valid_address_id(id: &str) -> bool {
    !id.is_empty()
        && id.len() <= MAX_ADDRESS_ID
        && id.bytes().all(|b| b.is_ascii_lowercase() || b.is_ascii_digit() || b == b'-')
}

/// `<data_dir>\mail-session-<address_id>`, or `None` for an id [`valid_address_id`] refuses.
pub fn session_dir(data_dir: &Path, address_id: &str) -> Option<PathBuf> {
    valid_address_id(address_id).then(|| data_dir.join(format!("{SESSION_PREFIX}{address_id}")))
}

/// `<data_dir>\mail-setup`: setup's profile while the saved sign-in is off.
pub fn setup_dir(data_dir: &Path) -> PathBuf {
    data_dir.join(SETUP_DIR)
}

fn is_profile_name(name: &str) -> bool {
    name == SETUP_DIR || name.strip_prefix(SESSION_PREFIX).is_some_and(valid_address_id)
}

fn sound(data_dir: &Path) -> bool {
    data_dir.is_absolute() && !data_dir.components().any(|c| c == Component::ParentDir)
}

/// The only folders this module deletes: exactly `data_dir`'s child `mail-setup` or
/// `mail-session-<address_id>`, for an absolute `data_dir` with no `..`. A grandchild, a sibling,
/// another profile's folder, another case or a relative path is refused.
pub fn is_session_dir(data_dir: &Path, dir: &Path) -> bool {
    sound(data_dir)
        && dir
            .file_name()
            .and_then(|n| n.to_str())
            .is_some_and(|n| is_profile_name(n) && dir == data_dir.join(n))
}

// ---- `mail.json` (§5.4) -----------------------------------------------------------------------

/// `<data_dir>\mail.json`. A field a file written before it existed lacks reads as its default.
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(default)]
pub struct MailState {
    /// The saved sign-in switch, on by default (D15).
    pub keep_session: bool,
    /// Each mailbox's last reconcile, by address id.
    pub last_reconcile: BTreeMap<String, String>,
    /// Profile folder names whose delete failed, retried at start-up.
    pub pending_deletes: Vec<String>,
    /// Mailboxes the student turned off in Knowlu's Settings; only grows (§13 Q11).
    pub turned_off: Vec<String>,
    /// Mailboxes whose backfill began and has not finished: the app resumes each at launch (§4.11, T11.3).
    /// An address id only; the window's start is the service's `backfill_since`. Written only while one
    /// is open, so a file with none is byte-for-byte what spec §5.4 lists.
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub open_backfills: Vec<String>,
    /// Each mailbox's send pace for its UTC day, by address id: the day's count, page loads and any pause, so
    /// a relaunch the same day carries on from them (spec §4.2 step 5: a mailbox's day cap, 600 for Microsoft 365
    /// and 1,500 originals in at most 300 messages for Gmail since 2026-10-19, and
    /// nothing retries the same day after a refused send). Written by a backfill, and by a slot pass that sent
    /// or whose mailbox has a backfill open (a same-day pace is merged, never lowered). Counts and days only;
    /// dropped once its day is past, and not written at all when empty.
    #[serde(skip_serializing_if = "BTreeMap::is_empty")]
    pub backfill_pace: BTreeMap<String, BackfillPace>,
    /// Each Gmail mailbox's last look at the Inbox for a non-delivery report, by address id: a unix second.
    /// Every look that read the Inbox, clean or refused, writes it, and a slot or backfill pass seeds its
    /// `Routine` from it, so the next look reads from there and a report is counted once, however far apart
    /// the slots are. A file written before it existed lacks it and reads as none. Not written when empty.
    #[serde(skip_serializing_if = "BTreeMap::is_empty")]
    pub last_look: BTreeMap<String, i64>,
    /// Each open Gmail backfill's frontier, by address id: the UTC day (days since 1970-01-01) its next pass
    /// searches from, the day before the oldest mail a pass found still missing (Quinn 2026-10-19). It only
    /// moves forward and is removed with the open marker. A file written before it existed lacks it and reads
    /// as none (the whole window). Not written when empty.
    #[serde(skip_serializing_if = "BTreeMap::is_empty")]
    pub backfill_frontier: BTreeMap<String, i64>,
    /// Each Gmail mailbox's lone row that will not attach, by address id (review of c8806b3): its received minute
    /// only (no sender, subject or text), the separate UTC days a pass ended stalled on it alone, and the last of
    /// them. From the third such day the row is left in every pass (`mail::backfill::stuck_for`). Cleared when a
    /// row at that minute is sent or held, when the backfill closes and on disconnect. A file written before it
    /// existed lacks it and reads as none. Not written when empty.
    #[serde(skip_serializing_if = "BTreeMap::is_empty")]
    pub lone_stall: BTreeMap<String, LoneStall>,
    /// Each Gmail mailbox's lone rows that reached their third stalled day, by address id (review round 2 of
    /// e7ae803): received minutes only. Each is left in every pass until the backfill closes or the mailbox is
    /// disconnected, so `lone_stall` is free to count the next row that stalls (one record let two stuck rows
    /// replace each other for good). A file written before it existed lacks it and reads as none. Not written
    /// when empty.
    #[serde(skip_serializing_if = "BTreeMap::is_empty")]
    pub lone_left: BTreeMap<String, Vec<i64>>,
    /// Each Gmail mailbox's messages of the last 24 hours, by address id: unix seconds, oldest first (review round
    /// 2 of e7ae803). Gmail's 300 messages are a rolling 24 hours (`mail_steps::GMAIL_MAX_MESSAGES_PER_DAY`), so they
    /// outlive the UTC day a `backfill_pace` is for; a slot or backfill pass seeds its `Routine` from them. Written
    /// after each send that kept its message, at each look and at a pass's end, merged never lowered, and pruned to
    /// the last 24 hours. Instants only: no sender, subject or text. Keyed by address id, so a mailbox disconnected
    /// and set up again starts from none (the device keeps nothing that names the account). A file written before it
    /// existed lacks it and reads as none (its `messages_today` then holds until the next UTC day). Not written when
    /// empty.
    #[serde(skip_serializing_if = "BTreeMap::is_empty")]
    pub gmail_messages: BTreeMap<String, Vec<i64>>,
    /// Each setup in flight on this device whose address this device minted (Quinn, 2026-10-06): written
    /// once the address exists, its facts written before each act that may set them, cleared when the run
    /// ends. One left at start-up is a setup the app quit mid-run (`mail::setup::clean_unfinished`). Not
    /// written at all when empty.
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub pending_setups: Vec<PendingSetup>,
}

/// One in-flight setup's address and `withdraw`'s facts (`runner.rs`): ids and flags only.
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct PendingSetup {
    pub address_id: String,
    /// The Knowlu address may be in the mailbox (a script that could set it began, a guide showed it, or
    /// the student said a step was done).
    pub placed: bool,
    /// A backfill began under it.
    pub backfilled: bool,
    /// Outside the run, the address was handed out or used: Settings' status read gave it to the console
    /// (with Copy), or a rotate began for it. The device can no longer tell it was never placed. Only grows:
    /// the run's own writes keep it.
    #[serde(default)]
    pub shown: bool,
}

/// A lone Gmail row that stalled passes (see [`MailState::lone_stall`]): unix seconds and day indexes only.
#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct LoneStall {
    /// The row's received minute, unix seconds (a whole minute).
    pub at: i64,
    /// The separate UTC days a pass ended stalled on this row alone.
    pub days: u32,
    /// The last of them (days since 1970-01-01).
    pub last_day: i64,
}

/// One mailbox's backfill pace on one UTC day (`unix seconds / 86400`).
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct BackfillPace {
    pub day: i64,
    pub sent_today: u32,
    /// Set after a refused send: the unix second the next UTC day starts, before which nothing sends.
    pub paused_until: Option<i64>,
    /// Gmail page loads made on `day` (the page budget, `mail_steps::MAX_PAGE_LOADS_PER_DAY`). A file written
    /// before it existed lacks it and reads as none; not written while none.
    #[serde(default, skip_serializing_if = "is_zero")]
    pub loads_today: u32,
    /// Gmail messages sent on `day` (`mail_steps::GMAIL_MAX_MESSAGES_PER_DAY`, 2026-10-19): one forward is one
    /// message of up to ten originals. A file written before it existed lacks it and reads as none; not written
    /// while none.
    #[serde(default, skip_serializing_if = "is_zero")]
    pub messages_today: u32,
}

fn is_zero(n: &u32) -> bool {
    *n == 0
}

impl Default for MailState {
    fn default() -> MailState {
        MailState {
            keep_session: true,
            last_reconcile: BTreeMap::new(),
            pending_deletes: Vec::new(),
            turned_off: Vec::new(),
            open_backfills: Vec::new(),
            backfill_pace: BTreeMap::new(),
            last_look: BTreeMap::new(),
            backfill_frontier: BTreeMap::new(),
            lone_stall: BTreeMap::new(),
            lone_left: BTreeMap::new(),
            gmail_messages: BTreeMap::new(),
            pending_setups: Vec::new(),
        }
    }
}

fn mail_json(data_dir: &Path) -> PathBuf {
    data_dir.join(MAIL_JSON)
}

impl MailState {
    /// A missing file is the default; a file that is not one JSON object of these fields is an
    /// error, never the default (the module doc says why).
    pub fn load(data_dir: &Path) -> Result<MailState, String> {
        let text = match std::fs::read_to_string(mail_json(data_dir)) {
            Ok(text) => text,
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => return Ok(MailState::default()),
            Err(e) => return Err(format!("mail.json unreadable: {}", e.kind())),
        };
        let bad = |e: serde_json::Error| format!("mail.json unreadable: {:?}", e.classify());
        let value: serde_json::Value = serde_json::from_str(&text).map_err(bad)?;
        if !value.is_object() {
            return Err("mail.json unreadable: not an object".into());
        }
        serde_json::from_value(value).map_err(bad)
    }

    pub fn is_turned_off(&self, address_id: &str) -> bool {
        self.turned_off.iter().any(|id| id == address_id)
    }

    /// Writes this state, keeping every `turned_off` id already on disk, and refuses while the file
    /// on disk is unreadable. Private: [`MailState::update`] is the one write path, so no caller can
    /// write a stale copy over `pending_deletes` or `keep_session` (D15).
    fn write_merged(&self, data_dir: &Path) -> Result<(), String> {
        let mut merged = MailState::load(data_dir)?.turned_off;
        for id in &self.turned_off {
            if !merged.contains(id) {
                merged.push(id.clone());
            }
        }
        let out = MailState { turned_off: merged, ..self.clone() };
        std::fs::create_dir_all(data_dir).map_err(|e| e.to_string())?;
        let v = serde_json::to_value(&out).map_err(|e| e.to_string())?;
        let tmp = data_dir.join(format!("{MAIL_JSON}.{}.tmp", std::process::id()));
        std::fs::write(&tmp, knowlu_engine::ledger::dumps_value(&v)).map_err(|e| e.to_string())?;
        std::fs::rename(&tmp, mail_json(data_dir)).map_err(|e| {
            let _ = std::fs::remove_file(&tmp);
            e.to_string()
        })
    }

    /// Load, change and save under one lock; `change` returns whether it changed anything, and
    /// nothing is written when it did not.
    pub fn update(data_dir: &Path, change: impl FnOnce(&mut MailState) -> bool) -> Result<MailState, String> {
        let _held = lock();
        let mut state = MailState::load(data_dir)?;
        if change(&mut state) {
            state.write_merged(data_dir)?;
        }
        Ok(state)
    }
}

/// Record `address_id` in `turned_off` (Disconnect's step 0, before anything else). An id
/// [`valid_address_id`] refuses, or an unreadable `mail.json`, is an error and writes nothing.
pub fn turn_off(data_dir: &Path, address_id: &str) -> Result<(), String> {
    if !valid_address_id(address_id) {
        return Err("not an address id".into());
    }
    MailState::update(data_dir, |s| {
        let new = !s.is_turned_off(address_id);
        if new {
            s.turned_off.push(address_id.to_string());
        }
        new
    })
    .map(|_| ())
}

// ---- deleting profiles (D15, §4.12, §4.13) ------------------------------------------------------

/// Delete `dir` patiently; `true` once it is gone (or was never there). `symlink_metadata`, not
/// `exists`: a dangling link still counts as there.
fn wipe(dir: &Path) -> bool {
    let gone = |d: &Path| std::fs::symlink_metadata(d).is_err();
    for attempt in 0..WIPE_TRIES {
        if gone(dir) || std::fs::remove_dir_all(dir).is_ok() {
            return true;
        }
        if attempt + 1 < WIPE_TRIES {
            std::thread::sleep(WIPE_BACKOFF);
        }
    }
    gone(dir)
}

/// Delete the profile folder `name` under `data_dir` through the guard, and keep
/// `pending_deletes` true to the result: a failure is added, a success removed.
///
/// `Ok(true)`: gone (a success `mail.json` cannot record still stands; the sweep names the file).
/// `Ok(false)`: still there, and recorded for [`start_up_sweep`]. `Err`: still there and nothing
/// records it, because the path is refused or `mail.json` cannot be read or written.
fn forget_named(data_dir: &Path, name: &str) -> Result<bool, String> {
    let dir = data_dir.join(name);
    if !is_session_dir(data_dir, &dir) {
        return Err("not a mail profile".into());
    }
    let gone = wipe(&dir);
    let recorded = MailState::update(data_dir, |s| {
        let listed = s.pending_deletes.iter().any(|n| n == name);
        if gone && listed {
            s.pending_deletes.retain(|n| n != name);
        } else if !gone && !listed {
            s.pending_deletes.push(name.to_string());
        }
        gone == listed
    });
    match recorded {
        Err(e) if !gone => Err(format!("{name} not deleted and not recorded: {e}")),
        _ => Ok(gone),
    }
}

/// **Forget one mailbox's saved sign-in**: delete `mail-session-<address_id>` and nothing else.
/// Closing the window first is the caller's. `Ok(false)` is retried by [`start_up_sweep`]; an
/// id [`valid_address_id`] refuses is an error.
pub fn forget(data_dir: &Path, address_id: &str) -> Result<bool, String> {
    if !valid_address_id(address_id) {
        return Err("not an address id".into());
    }
    forget_named(data_dir, &format!("{SESSION_PREFIX}{address_id}"))
}

/// Delete `mail-setup` (setup with the saved sign-in off, when its window closes).
pub fn forget_setup(data_dir: &Path) -> Result<bool, String> {
    forget_named(data_dir, SETUP_DIR)
}

/// **Every mail profile of this Knowlu profile** (sign-out, §4.13): each `mail-session-<id>` and
/// `mail-setup`, and nothing else in `data_dir`. Tries every one; `Ok(true)` when all are gone,
/// `Ok(false)` when every leftover is recorded, the first error when one is not.
pub fn forget_all(data_dir: &Path) -> Result<bool, String> {
    if !sound(data_dir) {
        return Err("not a data folder".into());
    }
    let entries = match std::fs::read_dir(data_dir) {
        Ok(entries) => entries,
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => return Ok(true),
        Err(e) => return Err(format!("data folder unreadable: {}", e.kind())),
    };
    let mut names: Vec<String> = entries
        .filter_map(|e| e.ok()?.file_name().into_string().ok())
        .filter(|n| is_profile_name(n))
        .collect();
    names.sort();
    let results: Vec<Result<bool, String>> = names.iter().map(|n| forget_named(data_dir, n)).collect();
    results.into_iter().try_fold(true, |all, r| r.map(|gone| gone && all))
}

/// **At start-up** (H4): remove a `mail-setup` a crash left and retry every delete that failed
/// before. Returns the folder names still pending, or an error when `mail.json` cannot be read or
/// written: that is a named state for Settings, never "nothing pending". A pending name that is not
/// a mail profile's is dropped, never deleted.
pub fn start_up_sweep(data_dir: &Path) -> Result<Vec<String>, String> {
    if !sound(data_dir) {
        return Err("not a data folder".into());
    }
    forget_setup(data_dir)?;
    for name in MailState::load(data_dir)?.pending_deletes {
        if is_profile_name(&name) {
            // Already listed: a failure needs no new record, and the update below reads the result.
            let _ = forget_named(data_dir, &name);
        }
    }
    let state = MailState::update(data_dir, |s| {
        let before = s.pending_deletes.len();
        s.pending_deletes.retain(|n| is_profile_name(n));
        s.pending_deletes.len() != before
    })?;
    Ok(state.pending_deletes)
}
