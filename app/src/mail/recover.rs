//! Reconcile and recover (email-forwarding plan T11.2; spec §4.11 steps 3 and 4, §4.2 step 5, test 32's
//! pace half). The routine one mailbox's pass runs, for the saved session's daily step and for a backfill
//! (a backfill is this routine over the term's window, spec §4.11): list what the mailbox received in a
//! window, compare it **on the device** with what the service holds, forward each missing message as an
//! attachment, oldest first, and close the batch with the count sent.
//!
//! **Nothing about an unheld message reaches the service.** `POST /reconcile` carries a window and
//! nothing else; its answer is the account's fingerprint key and the fingerprints the service holds
//! (spec §4.6 step 7: `HMAC_k(["fp", sender, subject])`, each folded to single spaces and lower case, hex).
//! The device computes the same HMACs and a listed message is **held** when a held entry has the same
//! fingerprint and a received minute within 15 minutes. A missing message goes only to Knowlu's address,
//! as an attachment, through the mailbox's own scripts. The key is read for the pass and dropped.
//!
//! **The scripts are the seam.** The routine speaks `app/assets/mail/` scripts' JSON ([`Mailbox`]); the
//! window half ([`WindowMailbox`]) is the one real one. It sends one pass over **at most 30 messages a
//! slot** unless a backfill is open ([`Job::cap`]), always at the pace of [`Routine`] (10 a minute; a UTC day's
//! 600 for Microsoft 365, and for Gmail 1,500 originals in at most 300 messages, Quinn 2026-10-19; the first
//! refusal stops it for the day), and deletes the Sent copies it made.
//!
//! **Microsoft 365** draws a virtualised list: the rows are the ones drawn. The routine narrows the window
//! with `search.js`'s `until` (halving a span that does not draw whole, down to one day) so each span is
//! listed whole; a single day too large to draw is counted as left for the student, never guessed at.
//! Outlook's conversation view (T18c) draws a conversation as one row: `list.js` opens it and reads each
//! message in it, as Gmail's are (Quinn 2026-10-02); each is judged once a pass (the student's own never,
//! a later one under its "Re: " subject too) and a missing one is forwarded alone. A conversation counts
//! once in `expected` and `handled`; one the page could not read whole is left.
//! **Gmail** lists conversations oldest page first ([`Outcome::expected`] and [`Outcome::handled`] are
//! conversations there). A search draws no Oldest button (live 2026-10-19): `search.js` walks "Next
//! results" to count, inside its step's time and [`PRESSES_PER_CALL`]; a walk cut short is not walked on but
//! aimed at the search's first day (below). "Attachment failed" (Gmail could not
//! upload the forward's attachments) ends the pass too: it is the page's, not a row's. A row that shows a message count is **opened** (Quinn 2026-10-02): `thread.js`
//! reads every message in it, each is fingerprinted, and a conversation with a missing message is
//! forwarded alone, expecting one attachment per message. If Gmail attaches fewer, `forward.js` refuses
//! and the conversation is left (T18c's gate "Gmail conversation rows"). A lone row that will not attach is
//! left only while sends show the page works (review of fd702e6): two such rows with no send between them
//! stop the pass, and a lone row still uploading first gets one longer try. A message the mailbox itself sent
//! (`own`, which `thread.js` reads from the account chip) is never judged: the service never holds it.
//!
//! **The Inbox look** (Gmail only; Quinn's ruling 2026-10-19 on the T18c gate "Gmail non-delivery report
//! between sends", option b; spec §4.2 step 5, risk 15). A bounce lands in the Inbox, which the search page
//! the walk stands on does not list, so right after the forward that brings the sends since the last look to
//! [`BOUNCE_EVERY`] or more (mid-page; groups of ten reach it at 30) the pass opens the Inbox and runs
//! `bounce.js`; a report of the batch stops the pass as
//! [`Stop::Paused`], like the first send failure. The look leaves the search, so the walk starts again and
//! skips the rows it decided before; the pass looks once more when it ends, before the batch closes and Sent
//! is cleaned, if anything was sent since the last look.
//!
//! **Gmail's page budget** (the design of 2026-10-19). Every open (a fresh Inbox is two), search, pager press
//! and conversation opened (its open, any "Expand all", its back) is a page load, charged to the routine's UTC day
//! (failures too, and a call with no answer at the bound it was given) up to `MAX_PAGE_LOADS_PER_DAY`, with
//! [`FINISH_RESERVE`] kept for the pass's end; a press the day cannot pay for is "no presses left", which ends the
//! pass as [`Stop::PageLimit`], never as the newest page. A restart (`Pass::aim`) reloads the Inbox (`FreshInbox`),
//! searches the same day narrowed with `before:` a few days past the row the walk goes on from, without a count
//! walk, and presses "older" until the page's oldest row is strictly older than that row, across as many calls as
//! it takes (only the day's pages end it); it is never made once the day's sends are spent. A page whose range
//! cannot be read is never the end ([`RANGE_UNREADABLE`]). A narrowed search's newest page is aimed past with a
//! later bound (at least a day later, its span doubled) until the search is whole: only the whole search's newest
//! page ends the pass. A backfill keeps its pass's frontier in `mail.json` and its next pass searches from there;
//! a row that was left never holds it back.
//!
//! **A lone row that never attaches** (review of c8806b3) stalls a pass when nothing is sent around it. The
//! caller counts the separate UTC days a pass ends stalled on it alone ([`Outcome::stalled_at`], `mail.json`'s
//! `lone_stall` for a backfill's passes and `recover_stall` for a slot's, its received minute only) and from the third
//! passes it in [`Job::stuck`] to be left at once. A row
//! that reached its third day is then kept as left (`lone_left`, [`Job::left`]) until the backfill closes, so the
//! record counts the next stuck row (review round 2 of e7ae803).
//!
//! **Privacy.** No sender, subject, address, nonce or page text is kept past the pass, logged or put in a
//! [`Stop`]; every stop is a fixed word or a service error's name.
use super::{run_in_window, Refusal, WINDOW};
use crate::mail_client::{MailClient, MailError, Phase};
use crate::mail_steps::{Paused, Provider, Routine, Send, BACKFILL_DAYS};
use serde_json::{json, Value};
use sha2::{Digest, Sha256};
use std::collections::{HashMap, HashSet, VecDeque};
use std::time::Duration;

/// At most this many forwards in one slot when no backfill is open (§4.11 step 4).
pub const SLOT_CAP: u32 = 30;
/// The saved session's usual window (§4.11 step 3, §13 Q3).
pub const WINDOW_DAYS: i64 = 3;
/// After a lapse, reconcile reaches back to `lapsed_since`, at most this far (§4.6 step 3).
pub const LAPSE_DAYS: i64 = 30;
/// A listed message is held when a held entry's minute is within this of its own (§4.11 step 3).
pub const MATCH_SECS: i64 = 15 * 60;
/// The window ends this long before the pass: mail that just arrived is still on its way through the
/// live path (which reaches `mail_claims` in minutes), so it is not "missing" yet.
pub const LAG_SECS: i64 = 15 * 60;
/// One step's wait for the page, longer than the runner's polling.
const STEP: Duration = Duration::from_secs(90);
/// `sent_cleanup.js`'s wait, and the most one call deletes: its worst case (one wait for the first row of
/// ours, three a deleted row since Delete is drawn only after the tick, live 2026-10-19, and two for each of
/// the five skips it allows, a row let go after the wait for Delete; 85 s here) stays inside [`STEP`]
/// (Checkpoint C).
const CLEANUP_WAIT_MS: u64 = 5_000;
const CLEANUP_ROWS: u32 = 2;
const _: () = assert!(CLEANUP_WAIT_MS * (3 * CLEANUP_ROWS as u64 + 11) < STEP.as_millis() as u64);
const DAY: i64 = 86_400;

// ---- the window -----------------------------------------------------------------------------------

/// A pass's window, in unix seconds.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Window {
    pub since: i64,
    pub until: i64,
}

/// The saved session's window at `now`: the last three days, reaching back to `lapsed_since` after a lapse
/// (at most 30 days) and, after forwarding was found off, to when it was last known on (at most as far as
/// the backfill window, which the service refuses to reach past). It ends [`LAG_SECS`] before `now`.
pub fn window(now: i64, lapsed_since: Option<i64>, last_known_on: Option<i64>) -> Window {
    let mut since = now - WINDOW_DAYS * DAY;
    if let Some(l) = lapsed_since {
        since = since.min(l.max(now - LAPSE_DAYS * DAY));
    }
    if let Some(k) = last_known_on {
        since = since.min(k.max(now - BACKFILL_DAYS * DAY));
    }
    Window { since, until: now - LAG_SECS }
}

/// A backfill's window: from the start of the UTC day `day_index` (days since 1970-01-01) to [`LAG_SECS`]
/// before `now`.
pub fn backfill_window(day_index: i64, now: i64) -> Window {
    Window { since: day_index * DAY, until: now - LAG_SECS }
}

fn iso(secs: i64) -> String {
    jiff::Timestamp::from_second(secs).map(|t| t.to_string()).unwrap_or_default()
}

/// `YYYY-MM-DD` of the UTC day with this index.
fn day_str(day: i64) -> String {
    iso(day * DAY).chars().take(10).collect()
}

/// The whole minute an instant falls in (unix seconds).
fn minute(at: i64) -> i64 {
    at - at.rem_euclid(60)
}

fn parse_instant(s: &str) -> Option<i64> {
    s.parse::<jiff::Timestamp>().ok().map(|t| t.as_second())
}

// ---- the fingerprint (spec §4.6 step 7) -----------------------------------------------------------

/// JavaScript's `\s`: what the service's fold collapses (Rust's `is_whitespace` differs at U+0085 and U+FEFF).
fn js_space(c: char) -> bool {
    matches!(c, '\t' | '\n' | '\u{b}' | '\u{c}' | '\r' | ' ' | '\u{a0}' | '\u{1680}' | '\u{2000}'..='\u{200a}'
        | '\u{2028}' | '\u{2029}' | '\u{202f}' | '\u{205f}' | '\u{3000}' | '\u{feff}')
}

/// `s.replace(/\s+/g, " ").trim().toLowerCase()`.
fn fold(s: &str) -> String {
    s.split(js_space).filter(|p| !p.is_empty()).collect::<Vec<_>>().join(" ").to_lowercase()
}

fn hmac_sha256(key: &[u8], msg: &[u8]) -> [u8; 32] {
    let mut block = [0u8; 64];
    if key.len() > 64 {
        block[..32].copy_from_slice(&Sha256::digest(key));
    } else {
        block[..key.len()].copy_from_slice(key);
    }
    let pad = |b: u8| block.map(|k| k ^ b);
    let mut inner = Sha256::new();
    inner.update(pad(0x36));
    inner.update(msg);
    let mut outer = Sha256::new();
    outer.update(pad(0x5c));
    outer.update(inner.finalize());
    outer.finalize().into()
}

/// `fp_hash`: lower-case hex of HMAC-SHA256 under `key` over `JSON(["fp", sender, subject])`, each folded.
/// `serde_json` writes a string as `JSON.stringify` does, so the bytes are the service's.
pub fn fingerprint(key: &[u8], sender: &str, subject: &str) -> String {
    let text = json!(["fp", fold(sender), fold(subject)]).to_string();
    hmac_sha256(key, text.as_bytes()).iter().map(|b| format!("{b:02x}")).collect()
}

/// Standard base64 (padding optional) to bytes; `None` for anything else.
fn base64(s: &str) -> Option<Vec<u8>> {
    let (mut out, mut acc, mut bits) = (Vec::new(), 0u32, 0u32);
    for c in s.trim().trim_end_matches('=').bytes() {
        let v = match c {
            b'A'..=b'Z' => c - b'A',
            b'a'..=b'z' => c - b'a' + 26,
            b'0'..=b'9' => c - b'0' + 52,
            b'+' => 62,
            b'/' => 63,
            _ => return None,
        };
        acc = (acc << 6) | u32::from(v);
        bits += 6;
        if bits >= 8 {
            bits -= 8;
            out.push((acc >> bits) as u8);
            acc &= (1 << bits) - 1;
        }
    }
    Some(out)
}

/// What `POST /reconcile` answered: the key and the held fingerprints, each with its received minute.
pub struct Held {
    key: Vec<u8>,
    entries: Vec<(String, i64)>,
}

impl Held {
    /// Reads the service's answer; `None` when the key is not 32 bytes of base64 or `held` is not a list.
    pub fn from_answer(v: &Value) -> Option<Held> {
        let key = base64(v.get("key")?.as_str()?)?;
        if key.len() != 32 {
            return None;
        }
        let entries = v.get("held")?.as_array()?.iter().filter_map(|h| {
            Some((h.get("fp")?.as_str()?.to_string(), parse_instant(h.get("minute")?.as_str()?)?))
        });
        Some(Held { key, entries: entries.collect() })
    }

    /// Whether a message from `sender` titled `subject` received at `at` is held (§4.11 step 3).
    pub fn holds(&self, sender: &str, subject: &str, at: i64) -> bool {
        let fp = fingerprint(&self.key, sender, subject);
        self.entries.iter().any(|(f, minute)| *f == fp && (minute - at).abs() <= MATCH_SECS)
    }
}

/// One listed message (or conversation row) as `list.js` read it. `at` is its instant, `None` when the
/// page's time could not be read (such a row cannot be fingerprinted and is left).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Row {
    pub sender: String,
    pub subject: String,
    pub received: String,
    pub at: Option<i64>,
    /// Gmail: the row is a conversation, and how many messages it counts (`None` when it shows no count).
    pub thread: bool,
    pub messages: Option<u32>,
    /// Gmail, inside an opened conversation: the signed-in account sent this message. The service never
    /// receives the student's own mail, so it is never judged missing (spec 4.11 step 3).
    pub own: bool,
    /// Microsoft 365: `list.js` read this message inside the conversation it numbers (T18c, conversation
    /// view); `forward.js` opens conversations to find it.
    pub conversation: Option<u32>,
}

impl Row {
    fn from_json(v: &Value) -> Option<Row> {
        let s = |k: &str| v.get(k).and_then(Value::as_str).map(str::to_string);
        Some(Row {
            sender: s("sender")?,
            subject: s("subject")?,
            received: s("received")?,
            at: v.get("received_at").and_then(Value::as_str).and_then(parse_instant),
            thread: v.get("thread").and_then(Value::as_bool).unwrap_or(false),
            messages: v.get("messages").and_then(Value::as_u64).and_then(|n| u32::try_from(n).ok()),
            own: v.get("own").and_then(Value::as_bool).unwrap_or(false),
            conversation: v.get("conversation").and_then(Value::as_u64).and_then(|n| u32::try_from(n).ok()),
        })
    }

    /// The args `forward.js` takes for this row.
    fn forward_args(&self) -> Value {
        let mut v = json!({ "sender": self.sender, "subject": self.subject, "received": self.received });
        if self.conversation.is_some() {
            v["in_conversation"] = json!(true);
        }
        v
    }
}

// ---- the seams ------------------------------------------------------------------------------------

/// Where a script runs: the place the mailbox's own page is showing.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Place {
    Inbox,
    /// The Inbox in a freshly loaded document, so Gmail forgets the views it cached (live 2026-10-19: a
    /// search set again over a cached one lands where the pass left it, "of many", and walks off the end).
    /// Gmail: navigate, then reload the window. Microsoft 365: as [`Place::Inbox`], whose navigation is
    /// already a full load.
    FreshInbox,
    /// Sent (Sent Items), where `sent_cleanup.js` runs.
    Sent,
    /// Settings, Forwarding (T11.3): Microsoft 365's scripts expect the page already open; Gmail's go to
    /// their section by hash from wherever the window is, so for Gmail this is the Inbox.
    Forwarding,
}

/// The mailbox's window as the routine drives it: a compiled-in script by name, and a place to be.
pub trait Mailbox {
    fn script(&self, path: &str, args: &Value) -> Result<Value, Refusal>;
    fn open(&self, place: Place) -> Result<(), Refusal>;
}

/// Time as the pace sees it: unix seconds, and a wait.
pub trait Clock {
    fn now(&self) -> i64;
    fn sleep(&self, secs: i64);
}

pub struct SystemClock;

impl Clock for SystemClock {
    fn now(&self) -> i64 {
        jiff::Timestamp::now().as_second()
    }
    fn sleep(&self, secs: i64) {
        std::thread::sleep(Duration::from_secs(secs.max(0) as u64));
    }
}

/// The open mail window as a [`Mailbox`]: scripts through [`run_in_window`], places by navigating the
/// window to the provider's own folder and waiting for the document (S1c: the page may redirect).
pub struct WindowMailbox<'a> {
    pub app: &'a tauri::AppHandle,
    pub provider: Provider,
}

impl Mailbox for WindowMailbox<'_> {
    fn script(&self, path: &str, args: &Value) -> Result<Value, Refusal> {
        run_in_window(self.app, self.provider, path, args, STEP)
    }

    fn open(&self, place: Place) -> Result<(), Refusal> {
        use tauri::Manager;
        let url = match (self.provider, place) {
            (Provider::M365, Place::Inbox | Place::FreshInbox) => "https://outlook.office.com/mail/inbox",
            (Provider::M365, Place::Sent) => "https://outlook.office.com/mail/sentitems",
            (Provider::M365, Place::Forwarding) => "https://outlook.office.com/mail/options/mail/forwarding",
            (Provider::Gmail, Place::Inbox | Place::FreshInbox | Place::Forwarding) => "https://mail.google.com/mail/u/0/#inbox",
            (Provider::Gmail, Place::Sent) => "https://mail.google.com/mail/u/0/#sent",
            (Provider::Guided, _) => return Err(Refusal::NotForProvider),
        };
        let w = self.app.get_webview_window(WINDOW).ok_or(Refusal::NoPage)?;
        w.navigate(url.parse().map_err(|_| Refusal::Unreadable)?).map_err(|_| Refusal::Lost)?;
        if place == Place::FreshInbox && self.provider == Provider::Gmail {
            // A hash change in the same document lands first, then the reload makes it a new one; `ready.js`
            // must not be answered by the old document, so it is given a moment to go.
            std::thread::sleep(Duration::from_secs(1));
            w.reload().map_err(|_| Refusal::Lost)?;
            std::thread::sleep(Duration::from_secs(2));
        }
        // The document settles, then the page draws its list: `ready.js` first, then a beat.
        for _ in 0..120 {
            std::thread::sleep(Duration::from_millis(250));
            if self.script("ready.js", &json!({})).ok().as_ref().and_then(Value::as_str) == Some("complete") {
                std::thread::sleep(Duration::from_secs(2));
                return Ok(());
            }
        }
        Err(Refusal::TimedOut)
    }
}

// ---- a pass ---------------------------------------------------------------------------------------

/// One mailbox's pass.
pub struct Job<'a> {
    pub provider: Provider,
    pub address_id: &'a str,
    /// The mailbox's Knowlu forwarding address (the one recipient a forward may have).
    pub knowlu_address: &'a str,
    /// `Backfill` opens its batch before listing (§4.2 step 5); `Recover` opens one at its first send.
    pub phase: Phase,
    pub window: Window,
    /// `Some(SLOT_CAP)` in a slot with no backfill open; `None` while a backfill is open (§4.11).
    pub cap: Option<u32>,
    /// Told `(expected, handled, sent)` as the pass goes (counts only); `false` stops the pass at its next
    /// send as [`Stop::Stopped`]. The setup runner's progress and its Stop (T11.3).
    pub watch: Option<&'a dyn Fn(u32, u32, u32) -> bool>,
    /// Told the clock at each look at the Inbox that read it, clean or refused (Gmail), so the caller can keep
    /// it where the next pass seeds its routine from (`mail.json`). Best effort: a note that is not kept costs
    /// the next pass a longer reach-back, never a look.
    pub on_look: Option<&'a dyn Fn(i64)>,
    /// Told the routine's UTC day, its page loads that day, its Gmail messages that day and the instants of its
    /// Gmail messages in the last 24 hours at each look, after each send that kept its message and at the pass's end
    /// (Gmail's page budget, [`crate::mail_steps::MAX_PAGE_LOADS_PER_DAY`], and its message cap,
    /// [`crate::mail_steps::GMAIL_MAX_MESSAGES_PER_DAY`]), so the caller can keep them in `mail.json` and a quit does
    /// not reset the budget. Best effort, like `on_look`.
    pub on_load: Option<&'a dyn Fn(i64, u32, u32, &[i64])>,
    /// Gmail, a backfill after its first day: the UTC day (days since 1970-01-01) the search starts from, the
    /// day before the oldest mail a pass found still missing (Quinn 2026-10-19: "Yes, search from there"). The
    /// search starts from the later of it and the window's first day; mail dated before it is not looked at.
    pub frontier: Option<i64>,
    /// Gmail: the lone row `mail.json` names as stalling passes (review of c8806b3), by its received minute, and
    /// whether it is now left at once when it will not attach (from the third separate day).
    pub stuck: Option<Stuck>,
    /// Gmail: the received minutes of lone rows `mail.json` keeps as left (`lone_left`, review round 2 of e7ae803):
    /// each reached its third stalled day, and is left at once when it will not attach, as a leaving `stuck` is.
    pub left: Vec<i64>,
}

/// A lone Gmail row that stalled passes: its received minute (unix seconds), and whether to leave it.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Stuck {
    pub at: i64,
    pub leave: bool,
}

/// Why a pass ended. A fixed word or a service error's name: nothing the page or a message said.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Stop {
    /// Everything in the window was handled.
    Done,
    /// The slot's 30 are sent.
    SlotCap,
    /// The day's sends are spent (Microsoft 365's 600 or Gmail's 1,500 originals a UTC day,
    /// [`crate::mail_steps::max_per_day`], or Gmail's 300 messages in 24 hours); the next send may go at this unix
    /// second ([`crate::mail_steps::Send::DayLimit`]).
    DayLimit { next_day_at: i64 },
    /// Gmail: the UTC day's page loads are spent ([`crate::mail_steps::MAX_PAGE_LOADS_PER_DAY`]); the next day starts at this
    /// unix second.
    PageLimit { next_day_at: i64 },
    /// The first non-delivery report or send failure (the line names the provider).
    Paused(Paused),
    /// A guided mailbox has no scripts to run.
    Guided,
    /// The student stopped the setup (the job's `watch` said so).
    Stopped,
    /// A step could not run or finish (a refusal's wording, or a fixed word).
    Script(&'static str),
    /// A service call failed ([`MailError::name`]).
    Service(&'static str),
}

/// What a pass did, in counts only.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Outcome {
    pub stop: Stop,
    /// The search's count: messages on Microsoft 365, conversations on Gmail.
    pub expected: u32,
    /// Handled so far, in the same unit as `expected`: held, sent or left.
    pub handled: u32,
    /// Listed and already held.
    pub held: u32,
    /// Originals forwarded (a conversation counts each message in it).
    pub sent: u32,
    /// Left for the student: unreadable, not on the page, a day too large to draw, a conversation whose
    /// attachments did not match. Never named, only counted.
    pub left: u32,
    /// Sent copies deleted from Sent.
    pub cleaned: u32,
    /// The batch was closed with the count sent.
    pub closed: bool,
    /// Gmail: the UTC day before the oldest row this pass sent or left waiting (a bounced batch is searched again)
    /// or, when that is older, before the row the walk would go on from. A row that was left is never retried
    /// (plan T11.2), so it never holds the frontier back (review of c8806b3). `None` when the pass met no row. A
    /// backfill keeps it as its next pass's [`Job::frontier`], only ever moving it forward.
    pub frontier: Option<i64>,
    /// Gmail: the pass ended stalled on one lone row that would not attach, received in this minute (unix seconds).
    pub stalled_at: Option<i64>,
    /// Gmail: a row received in the job's [`Job::stuck`] minute was sent or held.
    pub stuck_cleared: bool,
    /// Gmail: the minutes of [`Job::left`] in which a row was sent or held (review of 5f71623..6a8e738, #3).
    pub left_cleared: Vec<i64>,
    /// Gmail: the minutes in which this pass left a row at once, as a leaving [`Job::stuck`] or one of [`Job::left`].
    pub left_at_once: Vec<i64>,
}

impl Outcome {
    fn new() -> Outcome {
        Outcome { stop: Stop::Done, expected: 0, handled: 0, held: 0, sent: 0, left: 0, cleaned: 0, closed: false, frontier: None, stalled_at: None, stuck_cleared: false, left_cleared: Vec::new(), left_at_once: Vec::new() }
    }
}

/// Gmail: whether `routine`'s day has too few page loads left at `now` for a pass to open anything (the fresh
/// Inbox and the search, past the end's reserve): a pass would stop at once as [`Stop::PageLimit`].
pub fn pages_spent(routine: &mut Routine, now: i64) -> bool {
    routine.loads_left(now).saturating_sub(FINISH_RESERVE) < FRESH_LOADS + 1
}

/// One pass: lists, diffs, forwards, closes. A pass never writes the vault and never fails as a whole:
/// it ends as a [`Stop`] with the counts so far.
pub fn run_pass(mb: &dyn Mailbox, client: &MailClient, clock: &dyn Clock, routine: &mut Routine, job: &Job) -> Outcome {
    let mut pass = Pass { mb, client, clock, routine, job, out: Outcome::new(), batch: None, attempts: 0, judged: HashSet::new(), decided: HashMap::new(), walk: HashMap::new(), since_look: 0, restart: false, sent_since_lone: false, lone_seen: false, waiting: 0, since_day: 0, bound: i64::MIN, base_span: MIN_SPAN_DAYS, span: MIN_SPAN_DAYS, aim_at: 0, at_of: HashMap::new(), first_kept: None, waiting_at: None };
    pass.out.stop = pass.go().err().unwrap_or(Stop::Done);
    pass.out.frontier = pass.frontier();
    pass.finish();
    pass.out
}

type Step<T> = Result<T, Stop>;

struct Batch {
    id: String,
    nonce: String,
    /// When the batch opened (unix seconds): with no saved look, the first look reads back from
    /// [`BOUNCE_FIRST_LOOKBACK_SECS`] before.
    opened: i64,
}

/// A Gmail row as `list.js` reads it: sender, subject and time as the page writes them.
type RowKey = (String, String, String);

fn key_of(r: &Row) -> RowKey {
    (r.sender.clone(), r.subject.clone(), r.received.clone())
}

struct Pass<'a> {
    mb: &'a dyn Mailbox,
    client: &'a MailClient,
    clock: &'a dyn Clock,
    routine: &'a mut Routine,
    job: &'a Job<'a>,
    out: Outcome,
    batch: Option<Batch>,
    /// Forward scripts run (each can leave a copy in Sent).
    attempts: u32,
    /// Microsoft 365: the conversation messages judged this pass. A conversation is found by any message in
    /// it, so spans may list it twice; each message is judged once.
    judged: HashSet<(String, String, String)>,
    /// Gmail: the rows each page decided (held, left, out of window, read, forwarded), counted by key: two
    /// messages can read alike, and each is decided once.
    decided: HashMap<RowKey, u32>,
    /// Gmail: the rows met so far in the walk now under way, counted by key; a walk starts over after a look.
    walk: HashMap<RowKey, u32>,
    /// Gmail: originals sent since the last look at the Inbox.
    since_look: u32,
    /// Gmail: a look at the Inbox left the search page, so the walk must start over. Set by `forward_gmail`,
    /// returned early on by `flush`/`send_gmail`, and acted on (and cleared) by `gmail`; never a [`Stop`].
    restart: bool,
    /// Gmail: a send went since the last lone row that would not attach (or since the pass began).
    sent_since_lone: bool,
    /// Gmail: a lone row has failed to attach in this pass.
    lone_seen: bool,
    /// Gmail: the pass's first lone failure, when nothing had been sent before it: decided (not tried again this
    /// pass) and counted left only once a later send goes; a second lone failure before that stops the pass.
    waiting: u32,
    /// Gmail: the UTC day the pass's search starts from (the window's first day, or a later frontier).
    since_day: i64,
    /// Gmail: the current search's `until` day ([`UNBOUNDED`] when it has none; `i64::MIN` before the first
    /// restart). It only grows within a pass, strictly at each re-aim from a narrowed search's newest page.
    bound: i64,
    /// Gmail: a narrowed search's span past the row the walk goes on from, in days; `base_span` is the pass's
    /// own, and `span` doubles at each re-aim from a narrowed search's newest page.
    base_span: i64,
    span: i64,
    /// Gmail: the instant the last restart aimed at.
    aim_at: i64,
    /// Gmail: each row met in the walk now under way, by key: its instant.
    at_of: HashMap<RowKey, i64>,
    /// Gmail: the oldest instant of a row this pass sent (never one it left).
    first_kept: Option<i64>,
    /// Gmail: the instant of the lone row `waiting` counts.
    waiting_at: Option<i64>,
}

/// A missing Gmail row: one message, or a conversation of `n` messages sent alone.
enum Cand {
    One(Row),
    Thread(Row, u32),
}

fn svc<T>(r: Result<T, MailError>) -> Step<T> {
    r.map_err(|e| Stop::Service(e.name()))
}

fn refused(r: Refusal) -> Stop {
    Stop::Script(r.as_str())
}

fn count(v: &Value, key: &str) -> Option<u32> {
    v.get(key).and_then(Value::as_u64).and_then(|n| u32::try_from(n).ok())
}

const UNFINISHED: Stop = Stop::Script("a mailbox step did not finish");
/// Gmail: `list.js` was asked to move on a page whose range it could not read and whose pager drew no button that
/// way: nothing proves the page is the end, so it is never read as one (review of c8806b3).
const RANGE_UNREADABLE: Stop = Stop::Script("the page range could not be read");
/// Gmail: a fresh Inbox's page loads, the navigation and the reload (review of c8806b3).
const FRESH_LOADS: u32 = 2;
/// Gmail: the most page loads one `thread.js` call makes, the row's open, three "Expand all" and the back: a
/// conversation is opened only with this much page room, and a call with no answer is charged it.
const THREAD_LOADS: u32 = 5;
/// Gmail: the most pager presses one `search.js` or `list.js` call may make (`max_presses`).
const PRESSES_PER_CALL: u32 = 40;
/// Gmail: a restart's narrowed search reaches about this many rows past where the walk goes on: its span in
/// days is sized from the window's rows a day.
const SPAN_ROWS: i64 = 60;
/// Gmail: a narrowed search reaches at least this many whole days past the day of the row the walk goes on
/// from. `before:B` cuts at the account's local midnight (live 2026-10-19), so two days keep that row inside
/// the results in any zone (UTC-12 to UTC+14).
const MIN_SPAN_DAYS: i64 = 2;
/// Gmail: page loads kept for a pass's end (the last look's Inbox and Sent): the walk never spends them, and
/// the end is never refused.
const FINISH_RESERVE: u32 = 4;
/// Gmail: a narrowed search's bound that is no bound (the search runs without `until`).
const UNBOUNDED: i64 = i64::MAX;
/// Gmail: the Inbox is looked in for a non-delivery report once this many originals have been sent since the
/// last look (Quinn's ruling 2026-10-19, option b of the T18c gate "Gmail non-delivery report between sends").
const BOUNCE_EVERY: u32 = 25;
/// `bounce.js`'s wait for the Inbox's list (it waits twice at most, well inside a [`STEP`]).
const BOUNCE_WAIT_MS: u64 = 5_000;
/// A mailbox with no saved look (a new one, or one saved before the field existed) reaches back this far before
/// its batch opened. Two slots are up to about 13 hours apart (07:00 and 18:00), and a day covers the gap with
/// margin; a report older than that is the canary's, not this batch's.
const BOUNCE_FIRST_LOOKBACK_SECS: i64 = 24 * 3_600;
/// Gmail: the most pages one walk reads before it is called endless (a result of hundreds of thousands of
/// pages is not a mailbox). A walk that starts over after a look counts from its restart.
const WALK_PAGES: usize = 5_000;
/// Gmail holds a send for the student's Undo Send delay (30 seconds at most) after the compose closes, which
/// is when `forward.js` answers ok. Reloading the window inside it could cancel the held send or raise a "Leave
/// site?" dialog nothing can answer, so a fresh open waits until this long after the last ok forward.
const UNDO_SEND_SECS: i64 = 31;
const NO_END: Stop = Stop::Script("the results did not end");
/// Gmail: `bounce.js` could not read the Inbox (no list, or an answer that was neither "ok" nor "refused").
const INBOX_UNREADABLE: Stop = Stop::Script("the inbox could not be read");
/// Gmail: lone rows failed to attach with nothing sent between them (review of fd702e6): the page, not the rows.
/// The pass stops rather than leave every row, so the next pass tries them again.
const STALLED: Stop = Stop::Script("the attachments would not attach");
/// `forward.js`'s waits other than the upload, at their defaults (5 s each): Knowlu's own leftover compose to
/// go, the rows ticked, the menu item, Send, the recipient, the Send's settling and the rows let go (its header).
const FORWARD_OTHER_WAITS_MS: u64 = 35_000;
/// Gmail: the one longer try a lone row still uploading gets (review of fd702e6), as `upload_ms`: the longest
/// that keeps `forward.js`'s worst case (35 s + this, 85 s) inside one [`STEP`] with a margin for the page.
const SLOW_UPLOAD_MS: u64 = 50_000;
const _: () = assert!(FORWARD_OTHER_WAITS_MS + SLOW_UPLOAD_MS + 5_000 <= STEP.as_millis() as u64);

impl Pass<'_> {
    fn path(&self, name: &str) -> String {
        let dir = if self.job.provider == Provider::Gmail { "gmail" } else { "m365" };
        format!("{dir}/{name}")
    }

    /// A script's `ok` answer; anything else ends the pass with a fixed word. The pager presses it says it
    /// made are page loads, charged before the answer is read (a failure's too); "no presses left" is the
    /// day's page budget spent ([`Stop::PageLimit`]), never a page the walk reached.
    /// A call that gives no answer (it ran out its step, perhaps after pressing) is charged the presses it was
    /// allowed, `max_presses` (none for a search that does not walk): the bound, since nothing says fewer (review
    /// of c8806b3). "page range unreadable" is a page whose end nothing can prove: the pass stops by name.
    fn ok(&mut self, name: &str, args: Value) -> Step<Value> {
        let v = match self.mb.script(&self.path(name), &args) {
            Ok(v) => v,
            Err(r) => {
                let walks = args.get("walk") != Some(&json!(false));
                self.charge(if walks { count(&args, "max_presses").unwrap_or(0) } else { 0 });
                return Err(refused(r));
            }
        };
        self.charge(count(&v, "presses").unwrap_or(0));
        match (v.get("result").and_then(Value::as_str), v.get("reason").and_then(Value::as_str)) {
            (Some("ok"), _) => Ok(v),
            (Some("failed"), Some("no presses left")) => Err(self.page_limit()),
            (Some("failed"), Some("page range unreadable")) => Err(RANGE_UNREADABLE),
            _ => Err(UNFINISHED),
        }
    }

    /// `n` page loads made now (Gmail's budget; Microsoft 365 has none).
    fn charge(&mut self, n: u32) {
        if n > 0 && self.job.provider == Provider::Gmail {
            self.routine.loaded(self.clock.now(), n);
        }
    }

    /// Opens a place: one page load, and a fresh Inbox two (Gmail navigates, then reloads the window; review of
    /// c8806b3).
    fn open(&mut self, place: Place) -> Result<(), Refusal> {
        self.charge(if place == Place::FreshInbox { FRESH_LOADS } else { 1 });
        self.mb.open(place)
    }

    /// The page loads the walk may still make today, the pass's end kept aside.
    fn page_room(&mut self) -> u32 {
        self.routine.loads_left(self.clock.now()).saturating_sub(FINISH_RESERVE)
    }

    /// The presses one call may make: [`PRESSES_PER_CALL`], or less as the day's page room runs out.
    fn press_room(&mut self) -> u32 {
        self.page_room().min(PRESSES_PER_CALL)
    }

    fn page_limit(&self) -> Stop {
        Stop::PageLimit { next_day_at: (self.clock.now().div_euclid(DAY) + 1) * DAY }
    }

    fn go(&mut self) -> Step<()> {
        let j = self.job;
        if j.provider == Provider::Guided {
            return Err(Stop::Guided);
        }
        // Gmail searches in a freshly loaded document: a window kept across passes (a backfill's) still holds the
        // last pass's cached search view, which a search with the same words would be answered from.
        if j.provider == Provider::Gmail {
            // A day whose pages are spent opens nothing (the fresh Inbox and the search are three loads).
            if self.page_room() < FRESH_LOADS + 1 {
                return Err(self.page_limit());
            }
            self.since_day = j.window.since.div_euclid(DAY).max(j.frontier.unwrap_or(i64::MIN));
            self.fresh_inbox()?;
        } else {
            self.open(Place::Inbox).map_err(refused)?;
        }
        let found = if j.provider == Provider::Gmail {
            self.search(None, true)?
        } else {
            self.ok("search.js", json!({ "since": day_str(j.window.since.div_euclid(DAY)) }))?
        };
        // Gmail's count is a lower bound when its answer says `exact: false` (an "of many" result whose walk
        // to the last page ran out of time): the cloud only records `expected`, and nothing here stops at it.
        let n = count(&found, "count").ok_or(UNFINISHED)?;
        self.out.expected = n;
        if n == 0 {
            return Ok(());
        }
        if j.phase == Phase::Backfill {
            self.open_batch()?;
        }
        let answer = svc(self.client.reconcile(j.address_id, &iso(j.window.since), &iso(j.window.until)))?;
        let held = Held::from_answer(&answer).ok_or(Stop::Service("bad-reply"))?;
        if j.provider != Provider::Gmail {
            return self.m365(&held, n);
        }
        // A restart's narrowed search reaches about SPAN_ROWS rows: the search's rows a day, in whole days.
        let up = |a: i64, b: i64| (a + b - 1) / b;
        let days = up((j.window.until - self.since_day * DAY).max(1), DAY);
        self.base_span = MIN_SPAN_DAYS.max(up(SPAN_ROWS * days, i64::from(n)));
        self.span = self.base_span;
        self.gmail(&held, &found)?;
        // A lone row that would not attach, with no send after it to show the page works: nothing proves it is
        // the row's, so it is not left (the next pass tries it again). Its minute goes back to the caller, which
        // counts the separate days it stalls a pass alone (`mail.json`'s `lone_stall`).
        if self.waiting > 0 {
            self.out.stalled_at = self.waiting_at.map(minute);
            return Err(STALLED);
        }
        Ok(())
    }

    /// Waits until Gmail's Undo Send delay has passed since the last forward this routine saw answered ok, so a
    /// reload, the Inbox look, the batch's close, the Sent cleanup or the window's close cannot touch a send
    /// Gmail still holds.
    fn settle_undo(&self) {
        if let Some(sent) = self.routine.last_forward() {
            let wait = sent + UNDO_SEND_SECS - self.clock.now();
            if wait > 0 {
                self.clock.sleep(wait);
            }
        }
    }

    /// Opens the Inbox in a freshly loaded document (Gmail), once the Undo Send delay has passed.
    fn fresh_inbox(&mut self) -> Step<()> {
        self.settle_undo();
        self.open(Place::FreshInbox).map_err(refused)
    }

    /// Gmail's search from the pass's day, narrowed to before `until` when it has one: one page load, and the
    /// presses of its count walk (none with `walk` false, the page left on page 1).
    fn search(&mut self, until: Option<i64>, walk: bool) -> Step<Value> {
        let presses = self.page_room().saturating_sub(1).min(PRESSES_PER_CALL);
        let mut args = json!({ "since": day_str(self.since_day), "max_presses": presses });
        if let Some(u) = until {
            args["until"] = json!(day_str(u));
        }
        if !walk {
            args["walk"] = json!(false);
        }
        self.charge(1);
        self.ok("search.js", args)
    }

    fn open_batch(&mut self) -> Step<()> {
        if self.batch.is_some() {
            return Ok(());
        }
        let j = self.job;
        let since = match j.phase {
            Phase::Backfill => day_str(j.window.since.div_euclid(DAY)),
            Phase::Recover => iso(j.window.since),
        };
        let opened = self.clock.now();
        let v = svc(self.client.open_batch(j.address_id, j.phase, &since, self.out.expected))?;
        let text = |k: &str| v.get(k).and_then(Value::as_str).map(str::to_string);
        let (Some(id), Some(nonce)) = (text("batch_id"), text("nonce")) else { return Err(Stop::Service("bad-reply")) };
        self.batch = Some(Batch { id, nonce, opened });
        Ok(())
    }

    /// Takes up to `want` send slots from the pace: waits for the first, then takes what is free now.
    fn reserve(&mut self, want: u32) -> Step<u32> {
        let mut taken = 0;
        while taken < want {
            let now = self.clock.now();
            match self.routine.next(now) {
                Send::Now => {
                    self.routine.sent(now);
                    taken += 1;
                }
                Send::WaitUntil(t) if taken == 0 => self.clock.sleep((t - now).max(1)),
                Send::WaitUntil(_) => break,
                Send::DayLimit { .. } | Send::Paused(_) if taken > 0 => break,
                Send::DayLimit { next_day_at } => return Err(Stop::DayLimit { next_day_at }),
                Send::Paused(p) => return Err(Stop::Paused(p)),
            }
        }
        Ok(taken)
    }

    /// Reports the counts so far; `Stopped` when the job's watcher says to stop.
    fn tick(&self) -> Step<()> {
        match self.job.watch {
            Some(w) if !w(self.out.expected, self.out.handled, self.out.sent) => Err(Stop::Stopped),
            _ => Ok(()),
        }
    }

    /// The slot's room: stops the pass at the cap.
    fn room(&self) -> Step<u32> {
        self.tick()?;
        match self.job.cap {
            Some(cap) if self.out.sent >= cap => Err(Stop::SlotCap),
            Some(cap) => Ok(cap - self.out.sent),
            None => Ok(u32::MAX),
        }
    }

    /// The first refusal on the page: nothing sends again until the next UTC day.
    fn refusal(&mut self) -> Stop {
        self.routine.send_failed(self.clock.now());
        Stop::Paused(Paused::new(self.job.provider))
    }

    fn nonce(&self) -> String {
        self.batch.as_ref().map(|b| b.nonce.clone()).unwrap_or_default()
    }

    fn in_window(&self, at: i64) -> bool {
        at >= self.job.window.since && at <= self.job.window.until
    }
}

// ---- Microsoft 365: spans the list can draw whole, one forward at a time ---------------------------

impl Pass<'_> {
    fn m365(&mut self, held: &Held, whole: u32) -> Step<()> {
        let w = self.job.window;
        let mut spans: VecDeque<(i64, i64)> = VecDeque::from([(w.since.div_euclid(DAY), w.until.div_euclid(DAY) + 1)]);
        // The first search is the whole window's, and the page is on its results.
        let mut known = Some(whole);
        while let Some((a, b)) = spans.pop_front() {
            let n = match known.take() {
                Some(n) => n,
                None => {
                    let found = self.ok("search.js", json!({ "since": day_str(a), "until": day_str(b) }))?;
                    count(&found, "count").ok_or(UNFINISHED)?
                }
            };
            if n == 0 {
                continue;
            }
            let list = self.ok("list.js", json!({}))?;
            let total = count(&list, "total").ok_or(UNFINISHED)?;
            let mut rows = Vec::new();
            for r in list.get("rows").and_then(Value::as_array).into_iter().flatten() {
                rows.push(Row::from_json(r).ok_or(Stop::Script("a listed message could not be read"))?);
            }
            // The list's own rows read (a conversation is one, however many messages it holds), and the
            // conversations among them it could not read whole (T18c).
            let drawn = count(&list, "drawn").unwrap_or(rows.len() as u32);
            let unread = count(&list, "left").unwrap_or(0);
            if drawn < total && b - a > 1 {
                let mid = a + (b - a) / 2;
                spans.push_front((mid, b));
                spans.push_front((a, mid));
                continue;
            }
            // One day too large to draw: what the list does not show is left, never guessed at.
            let unseen = total.saturating_sub(drawn) + unread;
            self.out.left += unseen;
            self.out.handled += unseen;
            rows.sort_by_key(|r| r.at.unwrap_or(i64::MAX));
            // `handled` is in the search's unit: a conversation counts once, at its last message judged.
            let mut remaining: HashMap<u32, u32> = HashMap::new();
            for k in rows.iter().filter_map(|r| r.conversation) {
                *remaining.entry(k).or_default() += 1;
            }
            for row in rows {
                let weight = match row.conversation.and_then(|k| remaining.get_mut(&k)) {
                    Some(n) => {
                        *n -= 1;
                        u32::from(*n == 0)
                    }
                    None => 1,
                };
                self.m365_row(&row, held, weight)?;
            }
        }
        Ok(())
    }

    /// Judges one message; `weight` is what it adds to `handled` (0 for a conversation's message that is not
    /// its last). A conversation's message is held under its subject or the "Re: " form of it, as a Gmail
    /// conversation's is; the student's own and one already judged this pass are never judged.
    fn m365_row(&mut self, row: &Row, held: &Held, weight: u32) -> Step<()> {
        let Some(at) = row.at else {
            self.out.left += 1;
            self.out.handled += weight;
            return Ok(());
        };
        let again = row.conversation.is_some()
            && !self.judged.insert((row.sender.clone(), row.subject.clone(), row.received.clone()));
        let holds = |subject: &str| held.holds(&row.sender, subject, at);
        if row.own || again || !self.in_window(at) {
            self.out.handled += weight;
        } else if holds(&row.subject) || (row.conversation.is_some() && holds(&format!("Re: {}", row.subject))) {
            self.out.held += 1;
            self.out.handled += weight;
        } else {
            self.send_m365(row, weight)?;
        }
        self.tick()
    }

    fn send_m365(&mut self, row: &Row, weight: u32) -> Step<()> {
        self.room()?;
        self.reserve(1)?;
        self.open_batch()?;
        let mut args = row.forward_args();
        args["address"] = json!(self.job.knowlu_address);
        args["nonce"] = json!(self.nonce());
        self.attempts += 1;
        let v = self.mb.script(&self.path("forward.js"), &args).map_err(refused)?;
        match v.get("result").and_then(Value::as_str) {
            Some("ok") => self.out.sent += 1,
            Some("refused") => return Err(self.refusal()),
            // The row is not on the page any more: nothing was sent for it.
            Some("not found") => self.out.left += 1,
            _ => return Err(Stop::Script("a forward did not finish")),
        }
        self.out.handled += weight;
        Ok(())
    }
}

// ---- Gmail: conversations, oldest page first -----------------------------------------------------

/// What reading a conversation row came to.
enum Convo {
    /// Every message in the window is held.
    Held,
    /// At least one is missing; the conversation counts this many messages.
    Missing(u32),
    /// It could not be read whole: left for the student.
    Left,
}

impl Pass<'_> {
    /// The search's first page to judge: its oldest. The search's count walk usually left the page there; a
    /// result whose page 1 read an exact total is walked there by `list.js` inside one call's presses; a walk
    /// cut short (its time or its presses) is not walked on but aimed at the search's first day instead.
    fn gmail_first(&mut self, found: &Value) -> Step<Value> {
        if found.get("at_oldest").and_then(Value::as_bool) != Some(false) {
            let max = self.press_room();
            let v = self.ok("list.js", json!({ "go": "oldest", "max_presses": max }))?;
            if v.get("at_oldest").and_then(Value::as_bool) == Some(true) {
                return Ok(v);
            }
        }
        self.aim(self.since_day * DAY, false)
    }

    /// Where a restart's walk goes on from: the oldest row met in this walk that is not decided (two rows that
    /// read alike share an instant, so either is it), else the newest row met, else where the last one aimed.
    fn resume_at(&self) -> i64 {
        let undecided = |k: &RowKey| self.walk.get(k).copied().unwrap_or(0) > self.decided.get(k).copied().unwrap_or(0);
        let open = self.at_of.iter().filter(|(k, _)| undecided(k)).map(|(_, at)| *at).min();
        open.or_else(|| self.at_of.values().copied().max()).unwrap_or(self.aim_at)
    }

    /// The UTC day a backfill's next pass may search from: the day before the oldest row this pass sent or left
    /// waiting, or before where the walk would go on when that is older. `None` when no row was met.
    fn frontier(&self) -> Option<i64> {
        let kept = match (self.first_kept, self.waiting_at) {
            (Some(a), Some(b)) => Some(a.min(b)),
            (a, b) => a.or(b),
        };
        if self.job.provider != Provider::Gmail || (self.at_of.is_empty() && kept.is_none()) {
            return None;
        }
        let at = match kept {
            Some(m) => m.min(self.resume_at()),
            None => self.resume_at(),
        };
        Some(at.div_euclid(DAY) - 1)
    }

    /// A restart (Gmail): a fresh document, the search narrowed to before a few days past `at`, then "older"
    /// pressed until the page's oldest dated row is strictly older than `at` (so every row at `at` is on this
    /// page or a newer one: rows that read alike land together), or the pager ends. Returns the page landed on.
    /// Nothing reloads once the day's sends are spent, paused, or the slot's are, or with no page room left.
    /// `grow`: a re-aim from a narrowed search's newest page, whose bound must pass the last one (a gap in the
    /// mail longer than the span would otherwise repeat the same search for ever).
    fn aim(&mut self, at: i64, grow: bool) -> Step<Value> {
        self.room()?;
        match self.routine.next(self.clock.now()) {
            Send::DayLimit { next_day_at } => return Err(Stop::DayLimit { next_day_at }),
            Send::Paused(p) => return Err(Stop::Paused(p)),
            Send::Now | Send::WaitUntil(_) => {}
        }
        if self.page_room() < FRESH_LOADS + 1 {
            return Err(self.page_limit());
        }
        let floor = if grow { self.bound.saturating_add(1) } else { self.bound };
        let mut until = (at.div_euclid(DAY) + self.span).max(floor);
        // Past the window's end the bound cuts nothing in any zone: the search runs whole.
        if until >= self.job.window.until.div_euclid(DAY) + MIN_SPAN_DAYS {
            until = UNBOUNDED;
        }
        self.bound = until;
        self.aim_at = at;
        self.fresh_inbox()?;
        self.search((until != UNBOUNDED).then_some(until), false)?;
        self.walk.clear();
        self.at_of.clear();
        let mut v = self.ok("list.js", json!({}))?;
        // The seek goes on across calls, each press charged, however far it is (review of c8806b3: one call's 40
        // presses once ended the backfill); the day's page budget ends it as a page limit ("no presses left").
        for _ in 0..WALK_PAGES {
            let rows = v.get("rows").and_then(Value::as_array).into_iter().flatten();
            let oldest = rows.filter_map(|r| r.get("received_at").and_then(Value::as_str).and_then(parse_instant)).min();
            if oldest.is_some_and(|t| t < at) {
                return Ok(v);
            }
            let max = self.press_room();
            let older = self.ok("list.js", json!({ "go": "older", "max_presses": max }))?;
            if older.get("moved").and_then(Value::as_bool) != Some(true) {
                return Ok(older);
            }
            v = older;
        }
        Err(NO_END)
    }

    /// Whether the search now under way is narrowed by an `until` (the first search and an unbounded one are not).
    fn narrowed(&self) -> bool {
        self.bound != UNBOUNDED && self.bound != i64::MIN
    }

    fn gmail(&mut self, held: &Held, found: &Value) -> Step<()> {
        let mut first = Some(self.gmail_first(found)?);
        let mut pages = 0;
        while pages < WALK_PAGES {
            pages += 1;
            let v = match first.take() {
                Some(v) => v,
                None => {
                    let max = self.press_room();
                    let v = self.ok("list.js", json!({ "go": "newer", "max_presses": max }))?;
                    // Nowhere newer: the page before was this search's newest.
                    if v.get("moved").and_then(Value::as_bool) != Some(true) {
                        if !self.narrowed() {
                            return Ok(());
                        }
                        self.span = self.span.saturating_mul(2);
                        first = Some(self.aim(self.resume_at(), true)?);
                        pages = 0;
                        continue;
                    }
                    v
                }
            };
            let mut rows = Vec::new();
            for r in v.get("rows").and_then(Value::as_array).into_iter().flatten() {
                rows.push(Row::from_json(r).ok_or(Stop::Script("a listed message could not be read"))?);
            }
            let newest_page = v.pointer("/page/from").and_then(Value::as_u64).is_some_and(|from| from <= 1);
            let keys = self.gmail_page(rows, held)?;
            if self.restart {
                self.restart = false;
                // A look left the search. On the whole search's newest page with every row decided there is
                // nothing to walk to; otherwise aim again where the walk goes on, in a freshly loaded document (in
                // the old one Gmail restores its cached view of the search, live 2026-10-19, and walks past the
                // real end): what is decided is skipped, and what the look interrupted is handled now.
                if newest_page && !self.narrowed() && self.page_decided(&keys) {
                    return Ok(());
                }
                self.span = self.base_span;
                first = Some(self.aim(self.resume_at(), false)?);
                pages = 0;
            } else if newest_page {
                // Only the whole search's newest page ends the pass; a narrowed one's is aimed past.
                if !self.narrowed() {
                    return Ok(());
                }
                self.span = self.span.saturating_mul(2);
                first = Some(self.aim(self.resume_at(), true)?);
                pages = 0;
            }
        }
        Err(NO_END)
    }

    /// Whether every row met on a page so far in this walk is decided.
    fn page_decided(&self, keys: &[RowKey]) -> bool {
        keys.iter().all(|k| self.decided.get(k).copied().unwrap_or(0) >= self.walk.get(k).copied().unwrap_or(0))
    }

    fn decide(&mut self, key: RowKey) {
        *self.decided.entry(key).or_default() += 1;
    }

    /// A row received at `at` was sent or held: if it is in the job's stuck minute, the record can go, and if it is in
    /// a minute kept as left, so can that.
    fn clears_stuck(&mut self, at: i64) {
        let m = minute(at);
        if self.job.stuck.is_some_and(|s| s.at == m) {
            self.out.stuck_cleared = true;
        }
        if self.job.left.contains(&m) && !self.out.left_cleared.contains(&m) {
            self.out.left_cleared.push(m);
        }
    }

    /// A look read the Inbox: the routine remembers when, and the job's caller is told so it can keep it.
    fn looked(&mut self) {
        let now = self.clock.now();
        self.routine.looked(now);
        if let Some(note) = self.job.on_look {
            note(now);
        }
        self.note_loads();
    }

    /// Tells the job's caller the day's page loads so far, so `mail.json` keeps them.
    fn note_loads(&self) {
        if let Some(note) = self.job.on_load {
            note(self.routine.day(), self.routine.loads_today(), self.routine.messages_today(), &self.routine.message_times());
        }
    }

    /// Opens the Inbox and reads it for a non-delivery report of this batch (`bounce.js`). A report ends the
    /// pass as the first send failure does; a list that cannot be read ends it with a fixed word.
    fn look_for_bounce(&mut self) -> Step<()> {
        let Some(batch) = &self.batch else { return Err(INBOX_UNREADABLE) };
        // From the routine's last look (seeded from `mail.json`), so a report is counted once; with none saved the
        // first look reaches back a day before the batch opened.
        let since = self.routine.last_look().unwrap_or(batch.opened - BOUNCE_FIRST_LOOKBACK_SECS);
        let args = json!({
            "address": self.job.knowlu_address, "nonce": batch.nonce, "since": iso(since), "wait_ms": BOUNCE_WAIT_MS,
        });
        self.open(Place::Inbox).map_err(refused)?;
        let v = self.mb.script(&self.path("bounce.js"), &args).map_err(refused)?;
        match v.get("result").and_then(Value::as_str) {
            Some("ok") => {
                self.since_look = 0;
                self.looked();
                Ok(())
            }
            Some("refused") => {
                self.since_look = 0;
                self.looked();
                Err(self.refusal())
            }
            _ => Err(INBOX_UNREADABLE),
        }
    }

    /// Judges a page's rows and sends the missing ones. A row is decided (and so skipped by a walk that starts
    /// over after an Inbox look) the moment it is held, left, out of the window or sent; a missing row not yet
    /// sent when a look interrupts the page stays undecided. Returns the keys of the page's rows.
    fn gmail_page(&mut self, rows: Vec<Row>, held: &Held) -> Step<Vec<RowKey>> {
        let mut cands = Vec::new();
        let mut keys = Vec::new();
        for row in rows {
            // A row an earlier walk of this pass decided is skipped whole: no count, no conversation read, no
            // forward. Each of two rows that read alike counts once.
            let key = key_of(&row);
            let met = self.walk.entry(key.clone()).or_default();
            *met += 1;
            keys.push(key.clone());
            if let Some(at) = row.at {
                self.at_of.insert(key.clone(), at);
            }
            if *met <= self.decided.get(&key).copied().unwrap_or(0) {
                continue;
            }
            let Some(at) = row.at else {
                self.out.left += 1;
                self.out.handled += 1;
                self.decide(key);
                continue;
            };
            let done = if !self.in_window(at) {
                true
            } else if !row.thread {
                let known = held.holds(&row.sender, &row.subject, at);
                if known {
                    self.clears_stuck(at);
                } else {
                    cands.push(Cand::One(row.clone()));
                }
                self.out.held += u32::from(known);
                known
            } else {
                match self.read_thread(&row, held)? {
                    Convo::Held => {
                        self.out.held += 1;
                        self.clears_stuck(at);
                        true
                    }
                    Convo::Missing(n) => {
                        cands.push(Cand::Thread(row.clone(), n));
                        false
                    }
                    Convo::Left => {
                        self.out.left += 1;
                        true
                    }
                }
            };
            if done {
                self.out.handled += 1;
                self.decide(key);
            }
        }
        self.tick()?;
        self.send_gmail(cands)?;
        Ok(keys)
    }

    /// Opens a conversation row and fingerprints every message in the window (Quinn 2026-10-02). A later
    /// message reads with the conversation subject, so its "Re: " form is tried too. Its page loads (the open, any
    /// "Expand all", the back) are charged to the day's page budget like the pager's (review of c8806b3): one is
    /// opened only with [`THREAD_LOADS`] of page room, and a call with no answer is charged that.
    fn read_thread(&mut self, row: &Row, held: &Held) -> Step<Convo> {
        if self.job.provider == Provider::Gmail && self.page_room() < THREAD_LOADS {
            return Err(self.page_limit());
        }
        let mut args = row.forward_args();
        args["messages"] = json!(row.messages);
        let v = match self.mb.script(&self.path("thread.js"), &args) {
            Ok(v) => v,
            Err(r) => {
                self.charge(THREAD_LOADS);
                return Err(refused(r));
            }
        };
        self.charge(count(&v, "presses").unwrap_or(0));
        let result = v.get("result").and_then(Value::as_str);
        let named = |k: &str| v.get(k).and_then(Value::as_str);
        match (result, named("control"), named("reason")) {
            (Some("ok"), _, _) => {}
            (Some("not found"), Some("message" | "account"), _) => return Ok(Convo::Left),
            (Some("failed"), _, Some("not every message drawn" | "no messages" | "conversation did not open")) => {
                return Ok(Convo::Left);
            }
            _ => return Err(Stop::Script("a conversation could not be read")),
        }
        let mut messages = Vec::new();
        for m in v.get("messages").and_then(Value::as_array).into_iter().flatten() {
            messages.push(Row::from_json(m).ok_or(Stop::Script("a conversation could not be read"))?);
        }
        if messages.iter().any(|m| m.at.is_none()) {
            return Ok(Convo::Left);
        }
        // The student's own replies are never judged: the service never received them, so they would read
        // as missing on every pass (they still count in `messages.len()`, which Gmail attaches).
        let missing = messages
            .iter()
            .filter_map(|m| m.at.map(|at| (m, at)))
            .filter(|(m, at)| !m.own && self.in_window(*at))
            .any(|(m, at)| {
                !held.holds(&m.sender, &m.subject, at) && !held.holds(&m.sender, &format!("Re: {}", m.subject), at)
            });
        Ok(if missing { Convo::Missing(messages.len() as u32) } else { Convo::Held })
    }

    /// Sends a page's missing rows oldest first: consecutive single messages together (at most a minute's
    /// worth at a time), a conversation alone. Returns early, leaving the rest undecided, when a look at the
    /// Inbox set `restart`.
    fn send_gmail(&mut self, cands: Vec<Cand>) -> Step<()> {
        let mut group: Vec<Row> = Vec::new();
        for c in cands {
            match c {
                Cand::One(r) => {
                    group.push(r);
                    if group.len() >= crate::mail_steps::MAX_PER_MINUTE {
                        self.flush(&mut group)?;
                        if self.restart {
                            return Ok(());
                        }
                    }
                }
                Cand::Thread(r, n) => {
                    self.flush(&mut group)?;
                    if self.restart {
                        return Ok(());
                    }
                    self.room()?;
                    let mut got = 0;
                    while got < n {
                        got += self.reserve(n - got)?;
                    }
                    self.forward_taken(vec![r], Some(n), n)?;
                    if self.restart {
                        return Ok(());
                    }
                }
            }
        }
        self.flush(&mut group)
    }

    fn flush(&mut self, group: &mut Vec<Row>) -> Step<()> {
        while !group.is_empty() {
            let want = u32::try_from(group.len()).unwrap_or(u32::MAX).min(self.room()?);
            let k = self.reserve(want)?;
            let rows: Vec<Row> = group.drain(..k as usize).collect();
            self.forward_taken(rows, None, k)?;
            if self.restart {
                return Ok(());
            }
        }
        Ok(())
    }

    /// Forwards `rows` for which `taken` of the day's originals were reserved, and gives back to the day those
    /// that did not go: a row left with nothing sent (a conversation one short under G1, too big alone, gone, or a
    /// lone row that will not attach) used none (review round 2 of e7ae803). A send that stops the pass keeps them:
    /// it may have gone ("send did not settle").
    fn forward_taken(&mut self, rows: Vec<Row>, messages: Option<u32>, taken: u32) -> Step<()> {
        let (before, day) = (self.out.sent, self.routine.day());
        self.forward_gmail(rows, messages)?;
        self.routine.unsent(taken.saturating_sub(self.out.sent - before), day);
        Ok(())
    }

    /// One Send of `rows` (a conversation row carries its message count). An `over 25 MB` send is halved,
    /// as `forward.js` leaves to its caller.
    fn forward_gmail(&mut self, rows: Vec<Row>, messages: Option<u32>) -> Step<()> {
        self.forward_gmail_once(rows, messages, false)
    }

    /// A lone row that would not attach (no chip, still uploading after its longer try, or a conversation one
    /// short): left when a send went since the lone failure before it (or since the pass began). The pass's first
    /// one with nothing sent before it waits for the next send; any other stops the pass, leaving nothing (review
    /// of fd702e6: a Gmail-wide stall must not leave every row).
    /// The lone row `mail.json` says stalled passes on two separate days before today is left at once (review of
    /// c8806b3), and so is one it keeps as left ([`Job::left`]); either touches nothing else: it proves nothing
    /// about the page.
    fn lone_failed(&mut self, rows: &[Row]) -> Step<()> {
        let n = rows.len() as u32;
        let stuck = self.job.stuck.filter(|s| s.leave).map(|s| s.at);
        if let [row] = rows {
            let at = row.at.map(minute);
            if let Some(m) = at.filter(|m| Some(*m) == stuck || self.job.left.contains(m)) {
                if !self.out.left_at_once.contains(&m) {
                    self.out.left_at_once.push(m);
                }
                self.out.left += 1;
                self.out.handled += 1;
                self.decide(key_of(row));
                return Ok(());
            }
        }
        if self.sent_since_lone {
            self.out.left += n;
            self.out.handled += n;
        } else if !self.lone_seen {
            self.waiting += n;
            self.waiting_at = rows.iter().filter_map(|r| r.at).min();
        } else {
            // The breaker: the stall is counted on the first waiting row, as at the pass's end (review round 2 of
            // e7ae803: returning before the minute was set left two stuck rows side by side counted on no day).
            self.out.stalled_at = self.waiting_at.map(minute);
            return Err(STALLED);
        }
        self.sent_since_lone = false;
        self.lone_seen = true;
        for r in rows {
            self.decide(key_of(r));
        }
        Ok(())
    }

    /// `slow`: this is a lone row's one longer try at uploading ([`SLOW_UPLOAD_MS`]).
    fn forward_gmail_once(&mut self, rows: Vec<Row>, messages: Option<u32>, slow: bool) -> Step<()> {
        self.open_batch()?;
        let wire: Vec<Value> = rows
            .iter()
            .map(|r| {
                let mut v = r.forward_args();
                if let Some(n) = messages {
                    v["messages"] = json!(n);
                }
                v
            })
            .collect();
        let mut args = json!({ "address": self.job.knowlu_address, "nonce": self.nonce(), "rows": wire });
        if slow {
            args["upload_ms"] = json!(SLOW_UPLOAD_MS);
        }
        // One send is one Gmail message, whatever its originals (Quinn's day-cap ruling, 2026-10-19): reserved
        // before it goes, kept when it answers ok, given back when it did not go. A call with no answer keeps it,
        // and so does "send did not settle", which comes after Send was pressed: the send may have gone. A kept one
        // is in `mail.json` at once, so a relaunch the same day is never short (review round 1 of e7ae803).
        let now = self.clock.now();
        if !self.routine.reserve_message(now) {
            let next_day_at = self.routine.messages_free_at(now).unwrap_or((now.div_euclid(DAY) + 1) * DAY);
            return Err(Stop::DayLimit { next_day_at });
        }
        self.attempts += 1;
        let v = self.mb.script(&self.path("forward.js"), &args).map_err(refused)?;
        let reason = v.get("reason").and_then(Value::as_str);
        if v.get("result").and_then(Value::as_str) == Some("ok") || reason == Some("send did not settle") {
            self.note_loads();
        } else {
            self.routine.release_message();
        }
        let control = v.get("control").and_then(Value::as_str);
        match (v.get("result").and_then(Value::as_str), reason, control) {
            (Some("ok"), _, _) => {
                let sent = count(&v, "count").unwrap_or(rows.len() as u32);
                self.out.sent += sent;
                self.since_look += sent;
                self.routine.forwarded(self.clock.now());
                self.out.handled += rows.len() as u32;
                for r in &rows {
                    self.decide(key_of(r));
                    if let Some(at) = r.at {
                        self.first_kept = Some(self.first_kept.map_or(at, |k| k.min(at)));
                        self.clears_stuck(at);
                    }
                }
                // The page works: a lone row that was waiting for a send is the row's, and left.
                self.sent_since_lone = true;
                self.out.left += self.waiting;
                self.out.handled += self.waiting;
                self.waiting = 0;
                self.waiting_at = None;
                // Every 25 sends the Inbox is looked in, at once: a report stops the pass, and the walk (which
                // the look left) starts over.
                if self.since_look >= BOUNCE_EVERY {
                    self.look_for_bounce()?;
                    self.restart = true;
                }
            }
            (Some("refused"), _, _) => return Err(self.refusal()),
            // Gmail could not upload the attachments (live 2026-10-19): nothing was sent and the draft is
            // discarded. It is the page's, not these rows', so the pass stops and says so.
            (Some("failed"), Some("attachment failed"), _) => return Err(Stop::Script("an attachment did not upload")),
            // A compose left open by an earlier try is the page's state, not a row's: the pass stops and says so.
            (Some("failed"), Some("compose still open"), _) => return Err(Stop::Script("a draft was still open")),
            // A group of single rows one short (live, ef25 2026-10-19: 6 rows, 5 attached), or one with a chip
            // still uploading when the bound ran out (controller ruling 2026-10-19: one huge message must never
            // stop every pass at the same place), is halved like "over 25 MB", so only the row that will not
            // attach is left; nothing was sent. A conversation is one row.
            (
                Some("failed"),
                Some("over 25 MB" | "attachments do not match the rows" | "attachment still uploading"),
                _,
            ) if rows.len() > 1 => {
                let (a, b) = rows.split_at(rows.len() / 2);
                self.forward_gmail(a.to_vec(), None)?;
                if self.restart {
                    return Ok(());
                }
                self.forward_gmail(b.to_vec(), None)?;
            }
            // Alone, a chip still uploading at forward.js's bound gets one more try with the longest bound a step
            // allows (review of fd702e6: a big message can take longer than 30 s and still go).
            (Some("failed"), Some("attachment still uploading"), _) if !slow => {
                return self.forward_gmail_once(rows, messages, true);
            }
            // Too big alone, or the row gone from the page: nothing was sent, and it is left.
            (Some("failed"), Some("over 25 MB"), _) | (Some("not found"), _, Some("message")) => {
                self.out.left += rows.len() as u32;
                self.out.handled += rows.len() as u32;
                for r in &rows {
                    self.decide(key_of(r));
                }
            }
            // A lone row one short with a chip that drew (a conversation, under G1: Gmail attaches one of its
            // messages) is the row's own: the drawn chip shows the page works, so it is left at once and the
            // breaker below is neither armed nor reset (review round 1 of e7ae803: two side by side stalled every pass).
            (Some("failed"), Some("attachments do not match the rows"), _)
                if rows.len() == 1 && count(&v, "attached").is_some_and(|a| a >= 1) =>
            {
                self.out.left += 1;
                self.out.handled += 1;
                self.decide(key_of(&rows[0]));
            }
            // A lone row that would not attach: a message or a conversation whose chip never draws (forward.js
            // checks that before the count, so the halving ends here), one short with no chip counted, or a chip still
            // uploading after its longer try. Left only while sends show the page works ([`Pass::lone_failed`]).
            // With several rows, no attachment at all is an empty compose, which is the page's: the pass stops.
            (Some("failed"), Some("no attachment" | "attachments do not match the rows" | "attachment still uploading"), _)
                if rows.len() == 1 =>
            {
                self.lone_failed(&rows)?;
            }
            _ => return Err(Stop::Script("a forward did not finish")),
        }
        Ok(())
    }

    /// Closes the batch with the count sent, then deletes the Sent copies this pass made. A Gmail pass that
    /// sent anything since its last look looks in the Inbox first, before either: Gmail may thread a report
    /// with the Sent copy the cleanup trashes. A report there ends a pass that did not stop as `Paused`; a
    /// look that fails leaves the pass's stop as it was.
    fn finish(&mut self) {
        // Gmail holds the last send for its Undo Send delay: nothing that could touch it (the look's open, the
        // batch's close, Sent, or the caller closing the window after the pass) comes before it has gone,
        // whatever stopped the pass.
        if self.job.provider == Provider::Gmail {
            self.settle_undo();
        }
        self.close_and_clean();
        self.note_loads();
    }

    fn close_and_clean(&mut self) {
        if self.job.provider == Provider::Gmail && self.since_look > 0 && !matches!(self.out.stop, Stop::Paused(_)) {
            if let Err(stop @ Stop::Paused(_)) = self.look_for_bounce() {
                self.out.stop = stop;
            }
        }
        let Some(batch) = self.batch.take() else { return };
        self.out.closed = self.client.batch_sent(&batch.id, self.out.sent).is_ok();
        if self.attempts == 0 || self.open(Place::Sent).is_err() {
            return;
        }
        // Each call is asked for no more than it can finish inside one STEP, so none is still running in
        // the page after the runner gives up (Gmail's places share one document); then it is called again
        // while a call deleted all it was asked for, up to the attempts and five.
        let mut left = self.attempts + 5;
        while left > 0 {
            let max = left.min(CLEANUP_ROWS);
            let args = json!({
                "address": self.job.knowlu_address, "nonce": batch.nonce, "max": max, "wait_ms": CLEANUP_WAIT_MS,
            });
            let cleaned = self.mb.script(&self.path("sent_cleanup.js"), &args).ok();
            let deleted = cleaned.as_ref().and_then(|v| count(v, "deleted")).unwrap_or(0);
            self.out.cleaned += deleted;
            let whole = cleaned.as_ref().and_then(|v| v.get("result")).and_then(Value::as_str) == Some("ok");
            if !whole || deleted < max {
                return;
            }
            left -= max;
        }
    }
}
