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
//! slot** unless a backfill is open ([`Job::cap`]), always at the pace of [`Routine`] (10 a minute, 600 a
//! UTC day, the first refusal stops it for the day), and deletes the Sent copies it made.
//!
//! **Microsoft 365** draws a virtualised list: the rows are the ones drawn. The routine narrows the window
//! with `search.js`'s `until` (halving a span that does not draw whole, down to one day) so each span is
//! listed whole; a single day too large to draw is counted as left for the student, never guessed at.
//! Outlook's conversation view (T18c) draws a conversation as one row: `list.js` opens it and reads each
//! message in it, as Gmail's are (Quinn 2026-10-02); each is judged once a pass (the student's own never,
//! a later one under its "Re: " subject too) and a missing one is forwarded alone. A conversation counts
//! once in `expected` and `handled`; one the page could not read whole is left.
//! **Gmail** lists conversations oldest page first ([`Outcome::expected`] and [`Outcome::handled`] are
//! conversations there). A search draws no Oldest button (live 2026-10-19): `list.js` walks "Next
//! results" inside its step's budget and is asked again, its rows unread, until it says it is on the
//! oldest page; one that stops short without moving ends the pass. "Attachment failed" (Gmail could not
//! upload the forward's attachments) ends the pass too: it is the page's, not a row's. A row that shows a message count is **opened** (Quinn 2026-10-02): `thread.js`
//! reads every message in it, each is fingerprinted, and a conversation with a missing message is
//! forwarded alone, expecting one attachment per message. If Gmail attaches fewer, `forward.js` refuses
//! and the conversation is left (T18c's gate "Gmail conversation rows"). A message the mailbox itself sent
//! (`own`, which `thread.js` reads from the account chip) is never judged: the service never holds it.
//!
//! **The Inbox look** (Gmail only; Quinn's ruling 2026-10-19 on the T18c gate "Gmail non-delivery report
//! between sends", option b; spec §4.2 step 5, risk 15). A bounce lands in the Inbox, which the search page
//! the walk stands on does not list, so after the page that brings the sends since the last look to
//! [`BOUNCE_EVERY`] the pass opens the Inbox and runs `bounce.js`; a report of the batch stops the pass as
//! [`Stop::Paused`], like the first send failure. The look leaves the search, so the walk starts again (the
//! same search, the oldest page, then newer) and skips the rows it decided before; the pass looks once more
//! when it ends, before the batch closes and Sent is cleaned, if anything was sent since the last look.
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
/// `sent_cleanup.js`'s wait, and the most one call deletes: its worst case (two waits a deleted row, one
/// for each of the five skips it allows, 65 s here) stays inside [`STEP`] (Checkpoint C).
const CLEANUP_WAIT_MS: u64 = 5_000;
const CLEANUP_ROWS: u32 = 4;
const _: () = assert!(CLEANUP_WAIT_MS * (2 * CLEANUP_ROWS as u64 + 5) < STEP.as_millis() as u64);
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
            (Provider::M365, Place::Inbox) => "https://outlook.office.com/mail/inbox",
            (Provider::M365, Place::Sent) => "https://outlook.office.com/mail/sentitems",
            (Provider::M365, Place::Forwarding) => "https://outlook.office.com/mail/options/mail/forwarding",
            (Provider::Gmail, Place::Inbox | Place::Forwarding) => "https://mail.google.com/mail/u/0/#inbox",
            (Provider::Gmail, Place::Sent) => "https://mail.google.com/mail/u/0/#sent",
            (Provider::Guided, _) => return Err(Refusal::NotForProvider),
        };
        let w = self.app.get_webview_window(WINDOW).ok_or(Refusal::NoPage)?;
        w.navigate(url.parse().map_err(|_| Refusal::Unreadable)?).map_err(|_| Refusal::Lost)?;
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
}

/// Why a pass ended. A fixed word or a service error's name: nothing the page or a message said.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Stop {
    /// Everything in the window was handled.
    Done,
    /// The slot's 30 are sent.
    SlotCap,
    /// 600 are sent for the UTC day; the next day starts at this unix second.
    DayLimit { next_day_at: i64 },
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
}

impl Outcome {
    fn new() -> Outcome {
        Outcome { stop: Stop::Done, expected: 0, handled: 0, held: 0, sent: 0, left: 0, cleaned: 0, closed: false }
    }
}

/// One pass: lists, diffs, forwards, closes. A pass never writes the vault and never fails as a whole:
/// it ends as a [`Stop`] with the counts so far.
pub fn run_pass(mb: &dyn Mailbox, client: &MailClient, clock: &dyn Clock, routine: &mut Routine, job: &Job) -> Outcome {
    let mut pass = Pass { mb, client, clock, routine, job, out: Outcome::new(), batch: None, attempts: 0, judged: HashSet::new(), decided: HashMap::new(), walk: HashMap::new(), since_look: 0, restart: false };
    pass.out.stop = pass.go().err().unwrap_or(Stop::Done);
    pass.finish();
    pass.out
}

type Step<T> = Result<T, Stop>;

struct Batch {
    id: String,
    nonce: String,
    /// When the batch opened (unix seconds): a run's first look reads back from [`BOUNCE_LOOKBACK_SECS`] before.
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
/// Gmail: `list.js` could not reach the oldest page of the results.
const SHORT_OF_OLDEST: Stop = Stop::Script("the oldest page could not be reached");
/// Gmail: the most `list.js` walks toward the oldest page (each one inside a step, about 90 pages).
const OLDEST_WALKS: usize = 200;
/// Gmail: the Inbox is looked in for a non-delivery report once this many originals have been sent since the
/// last look (Quinn's ruling 2026-10-19, option b of the T18c gate "Gmail non-delivery report between sends").
const BOUNCE_EVERY: u32 = 25;
/// `bounce.js`'s wait for the Inbox's list (it waits twice at most, well inside a [`STEP`]).
const BOUNCE_WAIT_MS: u64 = 5_000;
/// A run's first look has no earlier look to read from, so it reaches back this far before the batch opened:
/// over the gap since the previous slot. A report older than that is the canary's, not this batch's.
const BOUNCE_LOOKBACK_SECS: i64 = 6 * 3_600;
/// Gmail: the most pages one walk reads before it is called endless (a result of hundreds of thousands of
/// pages is not a mailbox). A walk that starts over after a look counts from its restart.
const WALK_PAGES: usize = 5_000;
const NO_END: Stop = Stop::Script("the results did not end");
/// Gmail: `bounce.js` could not read the Inbox (no list, or an answer that was neither "ok" nor "refused").
const INBOX_UNREADABLE: Stop = Stop::Script("the inbox could not be read");

impl Pass<'_> {
    fn path(&self, name: &str) -> String {
        let dir = if self.job.provider == Provider::Gmail { "gmail" } else { "m365" };
        format!("{dir}/{name}")
    }

    /// A script's `ok` answer; anything else ends the pass with a fixed word.
    fn ok(&self, name: &str, args: Value) -> Step<Value> {
        let v = self.mb.script(&self.path(name), &args).map_err(refused)?;
        match v.get("result").and_then(Value::as_str) {
            Some("ok") => Ok(v),
            _ => Err(UNFINISHED),
        }
    }

    fn go(&mut self) -> Step<()> {
        let j = self.job;
        if j.provider == Provider::Guided {
            return Err(Stop::Guided);
        }
        self.mb.open(Place::Inbox).map_err(refused)?;
        let found = self.search()?;
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
        if j.provider == Provider::Gmail { self.gmail(&held) } else { self.m365(&held, n) }
    }

    /// The window's search; the same words every time it is asked (Gmail asks again after each Inbox look).
    fn search(&self) -> Step<Value> {
        self.ok("search.js", json!({ "since": day_str(self.job.window.since.div_euclid(DAY)) }))
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
    /// `list.js` on the oldest page of the results (a search draws no Oldest, live 2026-10-19): it walks "Next
    /// results" inside its own budget and says `at_oldest: false` when the walk stopped short. It is asked
    /// again, its rows unread, until it is there; a walk that stops short without moving would leave older
    /// pages unread, so it stops.
    fn gmail_oldest(&self) -> Step<Value> {
        let mut first = self.ok("list.js", json!({ "go": "oldest" }))?;
        for _ in 0..OLDEST_WALKS {
            if first.get("at_oldest").and_then(Value::as_bool) != Some(false) {
                break;
            }
            if first.get("moved").and_then(Value::as_bool) != Some(true) {
                return Err(SHORT_OF_OLDEST);
            }
            first = self.ok("list.js", json!({ "go": "oldest" }))?;
        }
        if first.get("at_oldest").and_then(Value::as_bool) == Some(false) {
            return Err(SHORT_OF_OLDEST);
        }
        Ok(first)
    }

    fn gmail(&mut self, held: &Held) -> Step<()> {
        let mut first = Some(self.gmail_oldest()?);
        let mut go = "oldest";
        let mut pages = 0;
        while pages < WALK_PAGES {
            pages += 1;
            let v = match first.take() {
                Some(v) => v,
                None => self.ok("list.js", json!({ "go": go }))?,
            };
            if go == "newer" && v.get("moved").and_then(Value::as_bool) != Some(true) {
                return Ok(());
            }
            let mut rows = Vec::new();
            for r in v.get("rows").and_then(Value::as_array).into_iter().flatten() {
                rows.push(Row::from_json(r).ok_or(Stop::Script("a listed message could not be read"))?);
            }
            let newest_page = v.pointer("/page/from").and_then(Value::as_u64).is_some_and(|from| from <= 1);
            let keys = self.gmail_page(rows, held)?;
            if self.restart {
                self.restart = false;
                // A look left the search. On the newest page with every row decided there is nothing to walk to;
                // otherwise ask the search again and walk from the oldest page: what is decided is skipped, and
                // what the look interrupted (rows judged missing but not yet sent) is handled now.
                if newest_page && self.page_decided(&keys) {
                    return Ok(());
                }
                self.search()?;
                first = Some(self.gmail_oldest()?);
                self.walk.clear();
                go = "oldest";
                pages = 0;
            } else if newest_page {
                return Ok(());
            } else {
                go = "newer";
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

    /// Opens the Inbox and reads it for a non-delivery report of this batch (`bounce.js`). A report ends the
    /// pass as the first send failure does; a list that cannot be read ends it with a fixed word.
    fn look_for_bounce(&mut self) -> Step<()> {
        let Some(batch) = &self.batch else { return Err(INBOX_UNREADABLE) };
        // From the routine's last look, so a report is counted once; the first of a run reaches back over the gap.
        let since = self.routine.last_look().unwrap_or(batch.opened - BOUNCE_LOOKBACK_SECS);
        let args = json!({
            "address": self.job.knowlu_address, "nonce": batch.nonce, "since": iso(since), "wait_ms": BOUNCE_WAIT_MS,
        });
        self.mb.open(Place::Inbox).map_err(refused)?;
        let v = self.mb.script(&self.path("bounce.js"), &args).map_err(refused)?;
        match v.get("result").and_then(Value::as_str) {
            Some("ok") => {
                self.since_look = 0;
                self.routine.looked(self.clock.now());
                Ok(())
            }
            Some("refused") => {
                self.since_look = 0;
                self.routine.looked(self.clock.now());
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
                if !known {
                    cands.push(Cand::One(row.clone()));
                }
                self.out.held += u32::from(known);
                known
            } else {
                match self.read_thread(&row, held)? {
                    Convo::Held => {
                        self.out.held += 1;
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
    /// message reads with the conversation subject, so its "Re: " form is tried too.
    fn read_thread(&mut self, row: &Row, held: &Held) -> Step<Convo> {
        let mut args = row.forward_args();
        args["messages"] = json!(row.messages);
        let v = self.mb.script(&self.path("thread.js"), &args).map_err(refused)?;
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
                    self.forward_gmail(vec![r], Some(n))?;
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
            let k = self.reserve(want)? as usize;
            let rows: Vec<Row> = group.drain(..k).collect();
            self.forward_gmail(rows, None)?;
            if self.restart {
                return Ok(());
            }
        }
        Ok(())
    }

    /// One Send of `rows` (a conversation row carries its message count). An `over 25 MB` send is halved,
    /// as `forward.js` leaves to its caller.
    fn forward_gmail(&mut self, rows: Vec<Row>, messages: Option<u32>) -> Step<()> {
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
        let args = json!({ "address": self.job.knowlu_address, "nonce": self.nonce(), "rows": wire });
        self.attempts += 1;
        let v = self.mb.script(&self.path("forward.js"), &args).map_err(refused)?;
        let reason = v.get("reason").and_then(Value::as_str);
        let control = v.get("control").and_then(Value::as_str);
        match (v.get("result").and_then(Value::as_str), reason, control) {
            (Some("ok"), _, _) => {
                let sent = count(&v, "count").unwrap_or(rows.len() as u32);
                self.out.sent += sent;
                self.since_look += sent;
                self.out.handled += rows.len() as u32;
                for r in &rows {
                    self.decide(key_of(r));
                }
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
            (Some("failed"), Some("attachment still uploading"), _) => {
                return Err(Stop::Script("an attachment was still uploading"))
            }
            // A group of single rows one short (live, ef25 2026-10-19: 6 rows, 5 attached) is halved too, so
            // only the row that will not attach is left; nothing was sent. A conversation is one row.
            (Some("failed"), Some("over 25 MB"), _) | (Some("failed"), Some("attachments do not match the rows"), _)
                if rows.len() > 1 =>
            {
                let (a, b) = rows.split_at(rows.len() / 2);
                self.forward_gmail(a.to_vec(), None)?;
                if self.restart {
                    return Ok(());
                }
                self.forward_gmail(b.to_vec(), None)?;
            }
            // The page would not take these as sent (too big alone, a row gone, a conversation whose
            // attachments did not match its count): nothing was sent, and they are left.
            (Some("failed"), Some("over 25 MB" | "attachments do not match the rows"), _)
            | (Some("not found"), _, Some("message")) => {
                self.out.left += rows.len() as u32;
                self.out.handled += rows.len() as u32;
                for r in &rows {
                    self.decide(key_of(r));
                }
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
        if self.job.provider == Provider::Gmail && self.since_look > 0 && !matches!(self.out.stop, Stop::Paused(_)) {
            if let Err(stop @ Stop::Paused(_)) = self.look_for_bounce() {
                self.out.stop = stop;
            }
        }
        let Some(batch) = self.batch.take() else { return };
        self.out.closed = self.client.batch_sent(&batch.id, self.out.sent).is_ok();
        if self.attempts == 0 || self.mb.open(Place::Sent).is_err() {
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
