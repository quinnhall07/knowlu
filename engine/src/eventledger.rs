//! Port of `engine/eventledger.py` — the append-only event verdict ledger.
//!
//! One verdict per uid, forever. The file (`state/events-seen.md`) is written by an LLM
//! following the cloud routine's prompt, so every read is defensive: a line that cannot be
//! parsed is skipped, and — only in the cases the Python singles out — recorded as a warning
//! so it reaches `state/runner-log.md` instead of vanishing.
//!
//! # Documented deviations, stated once
//!
//! 1. **`BTreeMap`, not insertion order.** Python returns a `dict`, whose iteration order is
//!    insertion order. No consumer in the Python codebase iterates the ledger — `cli.py`,
//!    `eventemit.py` and `eventroster.py` only ever call `.get(uid)`, and `emit_digest`
//!    imposes its own sort — so key order is unobservable at every call site. A `BTreeMap`
//!    keeps the type in std and keeps iteration deterministic.
//! 2. **Error strings.** Python's `ValueError` messages are reproduced verbatim by
//!    [`VerdictError`]'s `Display`. The one exception is the `ledger unreadable: {err}` warning,
//!    whose tail is an OS/decoder message and therefore already platform-specific in Python.
//!
//! # Quirks preserved deliberately (see the port report)
//!
//! - **CRLF on Windows.** Python opens this file in *text* mode, so every `\n` it writes becomes
//!   `os.linesep`. The live `state/events-seen.md` is CRLF for exactly this reason, and the read
//!   path undoes it with universal-newline translation. [`NEWLINE`] reproduces both halves.
//! - **`splitlines()` splits on more than `\n`.** U+2028, U+2029, `\x0b`, `\x0c`, `\x1c`-`\x1e`
//!   and U+0085 are all line boundaries to Python, but [`record_verdict`] sanitises only `\n`,
//!   `\r` and `·` out of a title. A scraped title containing U+2028 therefore writes one line
//!   that reads back as two, and the verdict is lost *silently* — neither fragment trips the
//!   warning condition. [`splitlines`] preserves this; it is not fixed here.
//! - **`strength` is unvalidated on write but must match `[a-z]+` on read.** `"Strong"` reads
//!   back as `""` and `"very strong"` reads back as `"very"`, both without a warning.
//! - **A uid containing the substring `verdict:` breaks its own marker lines.** `:` is inside the
//!   uid charclass and [`sanitize_uid`] does not escape it, so `- x:verdict:1 · proposed …`
//!   fails the `"verdict:" not in line` guard, is not recorded as proposed, and then warns
//!   "unreadable verdict line" on every run.
//!
//! # The one exception to "first verdict wins" (F1, engine follow-ups plan)
//!
//! [`load_ledger`]'s rule is "first verdict wins" — once a uid has a verdict, every later verdict
//! line for it is ignored, forever. A human answer line — written by [`record_answer`], shaped
//! `verdict:<obligation|drop> · by:<actor> · jid:<uuid> · answered <date>` — is allowed to break
//! that rule in exactly one case: when the verdict it is replacing is `unsure`. Every other
//! verdict (`obligation`, `opportunity`, `drop`) stays first-verdict-wins even against a human
//! answer line, so a confident machine verdict can never be silently flipped. The exception exists
//! because the whole point of the `unsure` decision card (F2/F3) is that the student's click takes
//! effect; without it, an answer could never settle anything. [`record_verdict`] and
//! [`record_judged_verdict`] write ordinary verdict lines — the latter with an optional `jid:`
//! field of its own, so a machine's `unsure` verdict can be traced back to the judgment that
//! minted it even before any human answers it.
//!
//! **What an older engine does with an answer line (review m-3).** One post-`unsure`, pre-F1
//! engine: `verdict:<obligation|drop>` matches its `VERDICT` regex and the word is valid, but its
//! unmodified first-verdict-wins keeps the earlier `unsure` — the event stays invisible, exactly
//! as it was before the answer, and `by:`/`jid:`/`answered` are ignored because no regex there
//! looks for them. A pre-`unsure` engine (three-word `VALID_VERDICTS`) instead skips the `unsure`
//! line itself with a warning, so the answer line becomes that uid's first *valid* verdict and
//! settles it immediately — harmless, and arguably the better outcome, just not the one either
//! brief anticipated.
//!
//! # The carry's line (PQ3 (b-prime), P15)
//!
//! When the student accepts a series (an executed `event-accept` card, or an executed
//! `event-check` card with `instances:`), the accept carry writes one line for each later instance
//! the card did not list: [`record_carried_answer`], shaped `verdict:<obligation|opportunity> ·
//! by:agent:knowlu.carry · from:<card id> · start:<…> · end:<…> · answered <date>`. The word is
//! the series' real verdict; `from:` names the archived card, which holds who answered, when, and
//! the series; the span is the instance's own, so the all-day lane can draw the date on each day it
//! covers without the roster. The line is the carry's, not the student's: its `by` starts `agent:`
//! ([`crate::provenance::is_agent`]). [`load_ledger`] reads it into [`LedgerEntry::carry`]:
//!
//! - it sets the verdict where there is none, or where an `unsure` stands unanswered, and its `by`
//!   becomes `answered_by`;
//! - it never flips a confident machine verdict, but its card and span are recorded all the same;
//! - it never counts after a human answer, and only a uid's first carry line counts;
//! - a later human answer replaces a verdict a carry line set, and clears `carry` (judge-once).
//!
//! The feed's title sits on the line right after `- <uid> · `, and [`clean_title`] maps only `·`
//! and the two newlines, so a title starting with a field head would forge that field. The carry
//! asks [`carried_answer_reads_back`] first, which reads the line with [`load_ledger`]'s own rules
//! before it is written, and writes no line the reader would misread.
//!
//! A human answer is an answer-shaped line whose `by` is *not* an agent's. Any other answer-shaped
//! line with an agent `by` (no card, no span that parses, or a word outside [`CARRY_VERDICTS`]) is
//! an ordinary verdict line: first verdict wins, it supersedes nothing, and `answered_by` stays
//! empty. No line written before PQ3 has an agent `by` (an `event-check` answer is written as the
//! vault's human actor or `unknown`), so every existing line reads exactly as before.
//!
//! **What an older engine does with a carry line.** It sees an answer line from an unknown actor:
//! as a uid's first line it settles the uid with the carried word; over an `unsure` it settles
//! only with `obligation`; and a later human answer can no longer replace it. Only a downgrade on
//! the same device meets this, because the ledger never syncs (F4).

use std::collections::BTreeMap;
use std::fs;
use std::io::Write as _;
use std::path::{Path, PathBuf};
use std::sync::LazyLock;

use jiff::civil::{Date, DateTime};
use regex::Regex;

/// Written once, when the file is first created. The em dash is part of the byte stream.
pub const HEADER: &str = "# Event verdict ledger — append-only. One verdict per uid, forever; \
delete a line to force a deliberate re-judge.";

/// The complete verdict vocabulary. Other modules match these strings exactly; nothing else is
/// accepted on write, and anything else is skipped with a warning on read.
///
/// **`unsure` (stream J Task T1, CHECKPOINT J-1, ruled 2026-09-22): additive, never a rename.**
/// Whether an event obliges a particular student is usually not in the event's own text; `unsure`
/// lets the judgment say so honestly instead of guessing among the other three. It also closes the
/// re-ask-forever defect for good: once ANY word here is recorded for a uid, `events::judge_roster`
/// never asks about it again (one verdict per uid, forever), so `unsure` for a service reply the
/// device could not otherwise use — below the confidence floor, incomplete, refused, truncated, or
/// a bare model failure — is exactly as terminal as a real `drop`. An OLDER engine reading a vault
/// this word reached would find it outside its own (three-word) `VALID_VERDICTS`, so
/// `load_ledger`'s "not a valid verdict" branch below would leave that entry's `verdict` unset —
/// graceful degradation to "ask again", never a crash and never a misread as some other word.
pub const VALID_VERDICTS: [&str; 4] = ["obligation", "opportunity", "drop", "unsure"];

/// The words a human answer to an `unsure` decision card may carry (F1). A strict subset of
/// [`VALID_VERDICTS`]: a person answers "does this apply to you", never "I'm not sure either" or
/// "this is an opportunity" — those stay machine-only verdicts.
pub const ANSWER_VERDICTS: [&str; 2] = ["obligation", "drop"];

/// The actor of the carry's answer line ([`record_carried_answer`], PQ3 (b-prime), P15). It
/// starts `agent:`, so [`crate::provenance::is_agent`] reads it as non-human: judge-once never
/// mistakes a carried answer for the student's own.
pub const CARRY_ACTOR: &str = "agent:knowlu.carry";

/// The words the carry's line may carry: an accepted series' real verdict. An `event-accept` card
/// accepts as `obligation` or `opportunity`, and an `event-check` series as `obligation`. `drop`
/// and `unsure` are never carried; a rejected series gets `declined` lines instead.
pub const CARRY_VERDICTS: [&str; 2] = ["obligation", "opportunity"];

/// The carry line's span format, on write and on read: the feed's civil time, to the second.
const SPAN_FORMAT: &str = "%Y-%m-%dT%H:%M:%S";

/// The line terminator Python's text-mode writes produce on this platform.
///
/// `Path.write_text` and `open(..., "a")` default to `newline=None`, which translates `\n` to
/// `os.linesep`. That is not cosmetic: the live ledger in this repo is CRLF, so a Rust writer
/// emitting bare LF would change the file's bytes on the first append.
pub use crate::pystr::NEWLINE;

// The six line-format regexes, character for character from the Python. All use `(?P<name>…)`,
// which Rust's `regex` crate accepts verbatim. None needs lookaround or a backreference — the
// only near-miss is the "is this a marker or a verdict line?" test, and the Python does that
// with a plain `in` check on the line rather than in the pattern, which ports directly.
//
// Each is compiled once via `LazyLock`: a bad pattern then panics on first use rather than
// once per ledger line.

/// The head of every line: `- <uid> · `. Note the trailing space after `·` — this is the
/// narrowest charclass in the whole pipeline, and the reason [`sanitize_uid`] exists.
static UID: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"^- (?P<uid>[A-Za-z0-9_.:@+-]+) · ").unwrap());

/// Searched, not matched — the verdict field can sit anywhere on the line.
static VERDICT: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"· verdict:(?P<verdict>[a-z]+)").unwrap());

/// `[a-z]+` only: an uppercase or multi-word strength silently reads back short or empty.
static STRENGTH: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"· strength:(?P<strength>[a-z]+)").unwrap());

/// The why field is quoted, so [`why_problem`] refuses a `"` on the way in.
static WHY: LazyLock<Regex> = LazyLock::new(|| Regex::new("· why:\"(?P<why>[^\"]*)\"").unwrap());

/// The title of a uid's first verdict line — the second `·`-segment. Anchored on the next ` · `,
/// not on ` · verdict:` (review m-2): an older, Python-written line puts a bare date segment
/// between the title and `verdict:` (`vault-full/state/events-seen.md` has this shape), and this
/// regex reads a title from that shape too, not just the one this crate writes. Read-only state
/// ([`LedgerEntry::title`]): it changes no line anyone writes and exists only so F3 can give each
/// uid its own title on its answer line (review I-6).
static TITLE: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"^- [A-Za-z0-9_.:@+-]+ · (?P<title>[^·]*?) · ").unwrap());

/// Searched, not matched, like [`VERDICT`]. Present on a human answer line and on the carry's line
/// (P15), whose `by` starts `agent:`; no other machine writer emits a `by:` field. Always searched
/// with the quoted `why:` field stripped out first (review I-1): `why` is free text and is
/// otherwise able to forge this field (see [`load_ledger`]).
static BY: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"· by:(?P<by>[A-Za-z0-9_.:@-]+)").unwrap());

/// Present on a judged machine verdict line ([`record_judged_verdict`]) and on a human answer
/// line ([`record_answer`]); absent everywhere else. Also searched with `why:` stripped first —
/// see [`BY`].
static JID: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"· jid:(?P<jid>[0-9a-f]{8}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{12})")
        .unwrap()
});

/// The marker that makes a line answer-shaped (a human answer, or the carry's line, P15) rather
/// than an ordinary verdict line. Checked together with [`BY`], both with `why:` stripped first: a
/// line needs both to count as an answer, and its `by` then says whose.
static ANSWERED: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"· answered \d{4}-\d{2}-\d{2}").unwrap());

/// The carry line's card (P15): the answering card's `id:`. Present on the carry's line and
/// nowhere else; searched with `why:` stripped first, like [`BY`].
static FROM: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"· from:(?P<from>appr_[0-9a-f]{10})").unwrap());

/// The carry line's span (P15), first and last instant, to the second ([`SPAN_FORMAT`]). Searched
/// like [`FROM`]; a value that matches but does not parse makes the line no carry line.
static START: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"· start:(?P<at>\d{4}-\d{2}-\d{2}T\d{2}:\d{2}:\d{2})").unwrap());
static END: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"· end:(?P<at>\d{4}-\d{2}-\d{2}T\d{2}:\d{2}:\d{2})").unwrap());

/// What [`record_answer`]'s `by` argument must look like — the actor sits unquoted on the line,
/// so it can never carry ` · `.
static BY_SAFE: LazyLock<Regex> = LazyLock::new(|| Regex::new(r"^[A-Za-z0-9_.:@-]{1,64}$").unwrap());

/// What a `jid` argument ([`record_judged_verdict`], [`record_answer`]) must look like — a
/// lowercase-hex UUID, unquoted on the line for the same reason as `by`.
static JID_SAFE: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"^[0-9a-f]{8}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{12}$").unwrap()
});

/// A `proposed` marker line. Matches a *verdict* line whose title starts with "proposed " too —
/// the `"verdict:" not in line` guard in [`load_ledger`] is what separates them.
static PROPOSED: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"^- (?P<uid>[A-Za-z0-9_.:@+-]+) · proposed ").unwrap());

/// A `declined` marker line. Same ambiguity, same guard.
static DECLINED: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"^- (?P<uid>[A-Za-z0-9_.:@+-]+) · declined ").unwrap());

/// The characters `_UID` accepts, minus `+`, which is reserved as the escape char.
///
/// Python calls `fullmatch` on a single character; `\A`/`\z` encode that exactly (Rust's `$`
/// would be equivalent here only because the class excludes `\n`, so the explicit anchors are
/// the honest translation).
static UID_SAFE: LazyLock<Regex> = LazyLock::new(|| Regex::new(r"\A[A-Za-z0-9_.:@-]\z").unwrap());

/// Coerce a uid minted from external text into the charclass `_UID` can read back.
///
/// Every other hop in the pipeline accepts arbitrary uids; this ledger's line format is the
/// narrowest. An ICS `UID` containing a space, `/`, `%` or `"` parses everywhere, reaches the
/// roster, gets judged — and its verdict line is then silently unreadable forever: never
/// relevant, never eligible, never warned, and never re-judged, because the routine's own
/// already-judged check is textual and does see the line. Escaping at mint time makes the round
/// trip a property of construction rather than of luck.
///
/// `+` is the escape character and escapes itself, so the mapping is injective: two different
/// uids can never collapse onto one, and nothing is dropped.
///
/// Iterates *characters*, and escapes a rejected one as its UTF-8 bytes in uppercase hex —
/// Python's `f"+{byte:02X}"` over `char.encode("utf-8")`. Python's `str(raw)` accepts any object;
/// the Rust signature narrows that to the only thing callers pass.
pub fn sanitize_uid(raw: &str) -> String {
    let mut out = String::new();
    let mut buf = [0u8; 4];
    for ch in raw.chars() {
        let encoded = ch.encode_utf8(&mut buf);
        if UID_SAFE.is_match(encoded) {
            out.push(ch);
        } else {
            for byte in encoded.as_bytes() {
                out.push_str(&format!("+{byte:02X}"));
            }
        }
    }
    out
}

/// One uid's accumulated ledger state. Python's frozen dataclass, whose defaults are
/// `verdict=None, strength="", why="", proposed=False, declined=False`.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct LedgerEntry {
    pub uid: String,
    /// `None` until a *valid* verdict line is seen — an unknown token leaves this unset, which
    /// is the whole point of skipping rather than accepting it.
    pub verdict: Option<String>,
    pub strength: String,
    pub why: String,
    pub proposed: bool,
    pub declined: bool,
    /// The `by:` of the human answer that settled this uid, once one has (F1). Empty until a
    /// [`record_answer`] line has superseded an `unsure` verdict, or is itself the first verdict
    /// line seen for the uid. A carry line that set the verdict leaves its own agent `by` here
    /// ([`CARRY_ACTOR`], P15), which [`crate::provenance::is_agent`] tells from a human's; a later
    /// human answer replaces it.
    pub answered_by: String,
    /// The `jid:` carried by whichever line won this uid's verdict — a judged machine verdict's
    /// own id, or (once a human answer supersedes an `unsure` one) the answer's id. Empty when
    /// neither line carried one.
    pub judgment_id: String,
    /// The second `·`-segment of the uid's first verdict line, read by [`TITLE`]. Read-only: no
    /// writer consults it, and a superseding answer line never changes it (F1 decision 4).
    pub title: String,
    /// What the uid's first carry line recorded (P15): the answering card and the instance's
    /// span. Set even when the line could not set the verdict (a confident machine word stands);
    /// never set over a human answer; cleared when a later human answer replaces the verdict a
    /// carry line set (judge-once). `None` on every line shape written before PQ3.
    pub carry: Option<Carried>,
}

/// The carry line's own fields (P15): which archived card answered the series, and the carried
/// instance's span exactly as the feed gave it, so the all-day lane can draw it on each day it
/// covers without the roster.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Carried {
    /// The answering card's `id:`, an `appr_` id.
    pub from: String,
    pub start: DateTime,
    pub end: DateTime,
}

impl LedgerEntry {
    /// `LedgerEntry(uid=uid)` — the dataclass defaults, used as the `dict.get` fallback.
    pub fn new(uid: impl Into<String>) -> Self {
        LedgerEntry { uid: uid.into(), ..Default::default() }
    }
}

/// The why string is embedded in a quoted ledger field; keep it single-line.
///
/// Checked in Python's order — a string with both a quote and a newline reports the quote.
pub fn why_problem(text: &str) -> Option<&'static str> {
    if text.contains('"') {
        return Some("why may not contain a double quote");
    }
    if text.contains('\n') || text.contains('\r') {
        return Some("why may not contain a newline");
    }
    if text.contains(" · ") {
        return Some("why may not contain the field separator");
    }
    None
}

/// Python raises `ValueError` from `record_verdict` and lets `OSError` propagate out of
/// `_append`; both become this.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum VerdictError {
    /// Python: `ValueError(f"unknown verdict: {verdict!r}")`.
    UnknownVerdict(String),
    /// Python: `ValueError(problem)`, where `problem` came from [`why_problem`].
    BadWhy(&'static str),
    /// New in F1: [`record_answer`]'s `by` or [`record_judged_verdict`]/[`record_answer`]'s `jid`
    /// failed its charclass check, or (PQ3) [`record_carried_answer`]'s `from` is not an `appr_`
    /// id. The field name (`"by"`, `"jid"` or `"from"`), not the bad value — the bad value may
    /// itself contain ` · ` and does not belong in an error message that could be logged.
    BadField(&'static str),
    /// Python: an uncaught `OSError` from the append.
    Io(String),
}

impl std::fmt::Display for VerdictError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            // `{verdict!r}` on a str is single-quoted.
            VerdictError::UnknownVerdict(v) => write!(f, "unknown verdict: '{v}'"),
            VerdictError::BadWhy(m) => write!(f, "{m}"),
            VerdictError::BadField(field) => write!(f, "invalid {field}"),
            VerdictError::Io(m) => write!(f, "io: {m}"),
        }
    }
}

fn path_for(vault: &Path) -> PathBuf {
    vault.join("state").join("events-seen.md")
}

fn warn(warnings: &mut Option<&mut Vec<String>>, message: String) {
    if let Some(list) = warnings {
        list.push(message);
    }
}

/// A ledger line, short enough to sit inside one runner-log summary.
///
/// Python's `len()` and `line[:limit]` count **code points**. Ledger lines are full of `·`
/// (U+00B7, two bytes in UTF-8), so a byte-based truncation would both cut at the wrong place
/// and risk splitting the separator in half.
fn shown(line: &str) -> String {
    shown_within(line, 90)
}

fn shown_within(line: &str, limit: usize) -> String {
    if line.chars().count() <= limit {
        line.to_string()
    } else {
        let mut out: String = line.chars().take(limit).collect();
        out.push('…');
        out
    }
}

/// Python's universal-newline decoding, which `read_text` applies before anything sees the text.
///
/// Both `\r\n` and a lone `\r` become `\n`. This is why the CRLF the writer produces on Windows
/// is invisible to the parser, and why `splitlines`' own `\r` handling never fires in practice.
fn universal_newlines(text: &str) -> String {
    crate::pystr::universal_newlines(text)
}

/// The characters Python's `str.splitlines()` treats as line boundaries.
///
/// Wider than `\n`: `\x0b`, `\x0c`, `\x1c`, `\x1d`, `\x1e`, U+0085, U+2028 and U+2029 all split.
/// Notably **`\x1f` does not**, even though `str.isspace()` is true for it — hence the separate
/// [`is_python_space`].
fn is_line_boundary(c: char) -> bool {
    matches!(
        c,
        '\n' | '\r' | '\u{b}' | '\u{c}' | '\u{1c}' | '\u{1d}' | '\u{1e}' | '\u{85}' | '\u{2028}'
            | '\u{2029}'
    )
}

/// Python's `str.splitlines()`. Rust's `str::lines()` is not equivalent — it splits on `\n`
/// only — and the difference is observable: a title carrying U+2028 is one line to the writer
/// and two to the reader.
fn splitlines(text: &str) -> Vec<&str> {
    let mut out = Vec::new();
    let mut start = 0usize;
    let mut chars = text.char_indices().peekable();
    while let Some((i, c)) = chars.next() {
        if !is_line_boundary(c) {
            continue;
        }
        out.push(&text[start..i]);
        let mut end = i + c.len_utf8();
        // `\r\n` is one boundary, not two. Unreachable after `universal_newlines`, kept faithful.
        if c == '\r' {
            if let Some(&(j, '\n')) = chars.peek() {
                chars.next();
                end = j + 1;
            }
        }
        start = end;
    }
    if start < text.len() {
        out.push(&text[start..]);
    }
    out
}

/// Python's `str.isspace()`, which is what bare `str.strip()` strips.
///
/// Wider than Rust's `char::is_whitespace` by exactly `\x1c`-`\x1f`.
fn is_python_space(c: char) -> bool {
    crate::pystr::is_python_space(c)
}

/// Read the ledger. First verdict for a uid wins; later ones are ignored.
///
/// `warnings` collects lines that look like ledger entries but cannot be read, so they reach
/// `state/runner-log.md` instead of vanishing. The writer of these lines is an LLM following a
/// prompt — exactly the case the WARN discipline exists for — and an unreadable verdict is
/// permanent: the uid is never relevant, never eligible, and never re-judged, because the
/// routine's already-judged check is textual and does see the line.
///
/// Exactly two failures are recorded and the rest are silent, and the boundary is narrow:
/// a line only warns as unreadable when it starts with `- ` **and** contains `verdict:`. A
/// garbage line, a `- ` line with no `verdict:`, and a fragment left behind by an exotic line
/// boundary all vanish without a trace.
pub fn load_ledger(vault: &Path, mut warnings: Option<&mut Vec<String>>) -> BTreeMap<String, LedgerEntry> {
    let path = path_for(vault);
    if !path.exists() {
        return BTreeMap::new();
    }
    let text = match read_text(&path) {
        Ok(text) => text,
        Err(err) => {
            // Python catches `(OSError, UnicodeDecodeError)` together and returns an empty
            // ledger — a *missing* verdict, not a fatal run.
            warn(&mut warnings, format!("ledger unreadable: {err}"));
            return BTreeMap::new();
        }
    };
    read_lines(splitlines(&text), warnings)
}

/// [`load_ledger`]'s per-line rules, over `lines` in order: the whole reader but the file. Split
/// out (review of T2b.5) so [`carried_answer_reads_back`] reads a line not yet written with exactly
/// these rules; `load_ledger` reads exactly as before.
fn read_lines<'a>(
    lines: impl IntoIterator<Item = &'a str>,
    mut warnings: Option<&mut Vec<String>>,
) -> BTreeMap<String, LedgerEntry> {
    let mut entries: BTreeMap<String, LedgerEntry> = BTreeMap::new();
    for line in lines {
        if let Some(caps) = PROPOSED.captures(line) {
            if !line.contains("verdict:") {
                let uid = caps.name("uid").unwrap().as_str();
                let mut current =
                    entries.get(uid).cloned().unwrap_or_else(|| LedgerEntry::new(uid));
                current.proposed = true;
                entries.insert(uid.to_string(), current);
                continue;
            }
        }
        if let Some(caps) = DECLINED.captures(line) {
            if !line.contains("verdict:") {
                let uid = caps.name("uid").unwrap().as_str();
                let mut current =
                    entries.get(uid).cloned().unwrap_or_else(|| LedgerEntry::new(uid));
                current.declined = true;
                entries.insert(uid.to_string(), current);
                continue;
            }
        }
        let head = UID.captures(line);
        let verdict = VERDICT.captures(line);
        let (head, verdict) = match (head, verdict) {
            (Some(h), Some(v)) => (h, v),
            _ => {
                if line.starts_with("- ") && line.contains("verdict:") {
                    warn(&mut warnings, format!("unreadable verdict line: {}", shown(line)));
                }
                continue;
            }
        };
        let token = verdict.name("verdict").unwrap().as_str();
        if !VALID_VERDICTS.contains(&token) {
            // Skipped, not silently accepted: an unrecognised token would otherwise count as a
            // verdict and settle the uid forever with no tier.
            //
            // This fires *before* the first-verdict-wins check, so a uid that is already settled
            // still warns about a later bad token. Faithful, and it is warning noise by design.
            warn(&mut warnings, format!("unknown verdict '{token}': {}", shown(line)));
            continue;
        }
        let uid = head.name("uid").unwrap().as_str();
        let current = entries.get(uid).cloned().unwrap_or_else(|| LedgerEntry::new(uid));
        // `why` is free text (`why_problem` only refuses `"`, a newline, or ` · ` — a bare `·`
        // survives), so a scraped or model-written why could otherwise forge `by:`, `jid:` or
        // `answered <date>` (review I-1). `bare` is the line with the whole quoted `why:"…"`
        // field removed, and every field below is searched on `bare`, never on `line`.
        let bare = WHY.replace(line, "");
        // Answer-shaped means `answered <date>` and a `by:`, both outside the why. Who answered
        // decides what the line may do (P15):
        // - a `by` that `provenance::is_agent` rejects makes a human answer ([`record_answer`]);
        // - an agent `by`, a word in [`CARRY_VERDICTS`], a card and a span that parses make the
        //   carry's line ([`record_carried_answer`]);
        // - any other agent line is an ordinary verdict line: it supersedes nothing, and
        //   `answered_by` stays empty. No line written before PQ3 has an agent `by`.
        let by = match (ANSWERED.is_match(&bare), BY.captures(&bare)) {
            (true, Some(c)) => Some(c.name("by").unwrap().as_str().to_string()),
            _ => None,
        };
        let agent = by.as_deref().is_some_and(crate::provenance::is_agent);
        let is_human_answer = by.is_some() && !agent;
        let carried =
            if agent && CARRY_VERDICTS.contains(&token) { carried_of(&bare) } else { None };
        if let Some(carried) = carried {
            // Only the first carry line counts, and none after a human answer (judge-once). It
            // sets the verdict where there is none or an `unsure` stands, with the carry as
            // `answered_by`; a confident machine verdict stands (F1's rule), but the card and the
            // span are recorded all the same, so the lane can still draw the date.
            let answered = &current.answered_by;
            let human = !answered.is_empty() && !crate::provenance::is_agent(answered);
            if current.carry.is_some() || human {
                continue;
            }
            let carry_by = by.unwrap_or_default();
            let mut next = match current.verdict.as_deref() {
                None => first_verdict(line, &bare, uid, token, &current, carry_by),
                Some("unsure") => LedgerEntry {
                    verdict: Some(token.to_string()),
                    answered_by: carry_by,
                    ..current.clone()
                },
                Some(_) => current.clone(),
            };
            next.carry = Some(carried);
            entries.insert(uid.to_string(), next);
            continue;
        }
        if let Some(existing) = current.verdict.as_deref() {
            // A human answer line is the other shape allowed to break "first verdict wins": only
            // with an answer word (F1 decision 1; review m-1 rules out a hand-edited or forged
            // `verdict:opportunity`/`verdict:unsure` on an answer-shaped line), and only over an
            // `unsure` (F1) or over a verdict a carry line set (P15, judge-once).
            let carry_set = crate::provenance::is_agent(&current.answered_by);
            if is_human_answer
                && (existing == "unsure" || carry_set)
                && ANSWER_VERDICTS.contains(&token)
            {
                // Supersede: the answer's verdict and `by` win; `why`, `strength` and `title`
                // stay the first line's (F1 decision 4), `judgment_id` becomes the answer's
                // `jid` when the line carries one, and a carry is cleared.
                let mut settled = current.clone();
                settled.verdict = Some(token.to_string());
                settled.answered_by = by.unwrap_or_default();
                if let Some(jid) = JID.captures(&bare) {
                    settled.judgment_id = jid.name("jid").unwrap().as_str().to_string();
                }
                settled.carry = None;
                entries.insert(uid.to_string(), settled);
            }
            continue; // first verdict wins otherwise — the invariant that keeps reads unambiguous
        }
        // `answered_by` is only ever set from a real answer line, never merely because a `by:`
        // substring is findable somewhere on the line (review I-1); an agent's ordinary verdict
        // line answers nothing (P15).
        let answered_by = if is_human_answer { by.unwrap_or_default() } else { String::new() };
        let entry = first_verdict(line, &bare, uid, token, &current, answered_by);
        entries.insert(uid.to_string(), entry);
    }
    entries
}

/// A uid's first verdict line, read in full: its word, `strength`, `why`, `jid` and title, with
/// the markers seen so far. `answered_by` is the caller's to decide (a human answer's or the
/// carry's `by`, else empty), and so is `carry`. `bare` is the line with `why:` stripped.
fn first_verdict(
    line: &str,
    bare: &str,
    uid: &str,
    token: &str,
    current: &LedgerEntry,
    answered_by: String,
) -> LedgerEntry {
    LedgerEntry {
        uid: uid.to_string(),
        verdict: Some(token.to_string()),
        strength: capture(&STRENGTH, line, "strength"),
        why: capture(&WHY, line, "why"),
        proposed: current.proposed,
        declined: current.declined,
        answered_by,
        judgment_id: capture(&JID, bare, "jid"),
        title: capture(&TITLE, line, "title"),
        carry: None,
    }
}

/// The named group of `re`'s first match in `text`, or `""`.
fn capture(re: &Regex, text: &str, name: &str) -> String {
    re.captures(text).map(|c| c.name(name).unwrap().as_str().to_string()).unwrap_or_default()
}

/// The carry fields of an agent's answer-shaped line (P15): a card, and a span whose two ends
/// both parse. `None` makes the line no carry line. `bare` is the line with `why:` stripped, so a
/// why can never forge either.
fn carried_of(bare: &str) -> Option<Carried> {
    fn at(re: &Regex, bare: &str) -> Option<DateTime> {
        DateTime::strptime(SPAN_FORMAT, re.captures(bare)?.name("at")?.as_str()).ok()
    }
    let from = FROM.captures(bare)?.name("from")?.as_str().to_string();
    Some(Carried { from, start: at(&START, bare)?, end: at(&END, bare)? })
}

/// `Path.read_text(encoding="utf-8")` — bytes, strict UTF-8, then universal newlines.
///
/// The error text differs from Python's `OSError`/`UnicodeDecodeError` messages; only the
/// `ledger unreadable:` prefix is load-bearing, and the tail was already platform-specific.
fn read_text(path: &Path) -> Result<String, String> {
    let bytes = fs::read(path).map_err(|e| e.to_string())?;
    let text = String::from_utf8(bytes).map_err(|e| e.to_string())?;
    Ok(universal_newlines(&text))
}

/// Append one line. Creates `state/` and writes the header only when the file does not exist.
///
/// Never reads the file. That is the point: `read_text` + `write_text` would rewrite every prior
/// line's terminator, and a concurrent writer's bytes with it.
fn append(vault: &Path, line: &str) -> std::io::Result<()> {
    let path = path_for(vault);
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)?;
    }
    if !path.exists() {
        fs::write(&path, format!("{HEADER}{NEWLINE}"))?;
    }
    let mut fh = fs::OpenOptions::new().append(true).create(true).open(&path)?;
    fh.write_all(format!("{line}{NEWLINE}").as_bytes())
}

/// Sanitise a title for embedding as a `·`-delimited ledger field.
///
/// `·` becomes `-` so a title can never forge a field separator; `\n`/`\r` become spaces so a
/// title can never split the line (U+2028 and friends are *not* handled — see the module doc);
/// the result is trimmed, and an empty result becomes `(untitled)`.
///
/// The one place any writer builds a title onto a line (review I-6): [`record_verdict`],
/// [`record_judged_verdict`] and [`record_answer`] all go through this, so none of them can put a
/// raw title on a line.
pub(crate) fn clean_title(title: &str) -> String {
    let cleaned: String = title.replace('·', "-").replace('\n', " ").replace('\r', " ");
    let cleaned = cleaned.trim_matches(is_python_space);
    if cleaned.is_empty() { "(untitled)".to_string() } else { cleaned.to_string() }
}

/// The line builder shared by [`record_verdict`] and [`record_judged_verdict`].
///
/// **Only `title` and `why` are checked.** `uid`, `strength` and `task` go onto the line
/// unexamined, so a uid with a space in it writes a permanently unreadable verdict (which is why
/// `eventfeed` calls [`sanitize_uid`] at mint), an uppercase strength reads back as `""`, and a
/// `task` containing ` · why:"…"` would inject a why field. Ported as found. `judgment_id`, new in
/// F1, is checked because it sits unquoted on the line.
#[allow(clippy::too_many_arguments)]
fn write_verdict_line(
    vault: &Path,
    uid: &str,
    title: &str,
    when: Date,
    verdict: &str,
    strength: &str,
    why: &str,
    task: &str,
    judgment_id: Option<&str>,
) -> Result<(), VerdictError> {
    if !VALID_VERDICTS.contains(&verdict) {
        return Err(VerdictError::UnknownVerdict(verdict.to_string()));
    }
    // `if why:` — an empty why skips validation, which is harmless because it is then omitted.
    if !why.is_empty() {
        if let Some(problem) = why_problem(why) {
            return Err(VerdictError::BadWhy(problem));
        }
    }
    if let Some(jid) = judgment_id {
        if !JID_SAFE.is_match(jid) {
            return Err(VerdictError::BadField("jid"));
        }
    }
    let clean = clean_title(title);

    let mut line = format!("- {uid} · {clean} · verdict:{verdict}");
    if !strength.is_empty() {
        line.push_str(&format!(" · strength:{strength}"));
    }
    if !why.is_empty() {
        line.push_str(&format!(" · why:\"{why}\""));
    }
    if !task.is_empty() {
        line.push_str(&format!(" · task:{task}"));
    }
    if let Some(jid) = judgment_id {
        line.push_str(&format!(" · jid:{jid}"));
    }
    line.push_str(&format!(" · first seen {}", when.strftime("%Y-%m-%d")));
    append(vault, &line).map_err(|e| VerdictError::Io(e.to_string()))
}

/// Append a verdict line for `uid`.
///
/// Python's `strength`, `why` and `task` are keyword arguments defaulting to `""`; pass `""` for
/// each here. An empty field is omitted from the line entirely. Signature and bytes are unchanged
/// by F1: this always writes with no `jid:` field, byte for byte as before.
#[allow(clippy::too_many_arguments)]
pub fn record_verdict(
    vault: &Path,
    uid: &str,
    title: &str,
    when: Date,
    verdict: &str,
    strength: &str,
    why: &str,
    task: &str,
) -> Result<(), VerdictError> {
    write_verdict_line(vault, uid, title, when, verdict, strength, why, task, None)
}

/// Append a machine verdict line that also carries a `· jid:<uuid>` field, so a later human
/// answer ([`record_answer`]) can be traced back to the judgment that minted this verdict — and so
/// F2's `unsure` decision card can name the judgment it is asking about.
///
/// Shares [`write_verdict_line`] with [`record_verdict`]: the five existing `record_verdict`
/// callers (`cli.rs` ×2, `events.rs` ×2, `eventemit.rs` ×1) are unaffected. `strength` and `task`
/// are always empty here — no existing caller of a judged verdict needs them; a future one that
/// does can go through `write_verdict_line` directly.
pub fn record_judged_verdict(
    vault: &Path,
    uid: &str,
    title: &str,
    when: Date,
    verdict: &str,
    why: &str,
    judgment_id: Option<&str>,
) -> Result<(), VerdictError> {
    write_verdict_line(vault, uid, title, when, verdict, "", why, "", judgment_id)
}

/// Append a human answer line: the shape [`load_ledger`] lets supersede an `unsure` verdict (F1
/// decision 1) or one the carry set (P15). It counts as a human answer only while `by` is not an
/// agent's ([`crate::provenance::is_agent`]): `settle_event_check` passes the journal's human
/// actor or `unknown`, and `inherit_series_answers` copies one of those.
///
/// `verdict` must be one of [`ANSWER_VERDICTS`]; anything else — including a word from
/// [`VALID_VERDICTS`] that is not an answer word, such as `"opportunity"` — is refused. `by` must
/// match `^[A-Za-z0-9_.:@-]{1,64}$` and, when given, `judgment_id` must be a lowercase-hex UUID:
/// both sit unquoted on the line and so can never be allowed to carry ` · `. Every check runs
/// before the line is built, so a rejected call never touches the file.
pub fn record_answer(
    vault: &Path,
    uid: &str,
    title: &str,
    when: Date,
    verdict: &str,
    by: &str,
    judgment_id: Option<&str>,
) -> Result<(), VerdictError> {
    if !ANSWER_VERDICTS.contains(&verdict) {
        return Err(VerdictError::UnknownVerdict(verdict.to_string()));
    }
    if !BY_SAFE.is_match(by) {
        return Err(VerdictError::BadField("by"));
    }
    if let Some(jid) = judgment_id {
        if !JID_SAFE.is_match(jid) {
            return Err(VerdictError::BadField("jid"));
        }
    }
    let clean = clean_title(title);
    let mut line = format!("- {uid} · {clean} · verdict:{verdict} · by:{by}");
    if let Some(jid) = judgment_id {
        line.push_str(&format!(" · jid:{jid}"));
    }
    line.push_str(&format!(" · answered {}", when.strftime("%Y-%m-%d")));
    append(vault, &line).map_err(|e| VerdictError::Io(e.to_string()))
}

/// Append the carry's answer line (PQ3 (b-prime), P15), one per carried date:
/// `- <uid> · <title> · verdict:<w> · by:agent:knowlu.carry · from:<card id> · start:<…> ·
/// end:<…> · answered <when>`.
///
/// A carried date is an instance of an accepted series that the answering card did not list.
/// `verdict` is the series' real verdict, one of [`CARRY_VERDICTS`]; `from` is the answering
/// card's `id:`, an `appr_` id; `start` and `end` are the instance's span as the feed gave it,
/// written to the second. Both checks run before the line is built, so a refused call never
/// touches the file. The actor is always [`CARRY_ACTOR`], never an argument, so this line can never
/// be credited to the student. A sibling of [`record_answer`], whose signature, checks and bytes
/// are unchanged: the two share only [`clean_title`] and the append.
#[allow(clippy::too_many_arguments)]
pub fn record_carried_answer(
    vault: &Path,
    uid: &str,
    title: &str,
    when: Date,
    verdict: &str,
    from: &str,
    start: DateTime,
    end: DateTime,
) -> Result<(), VerdictError> {
    if !CARRY_VERDICTS.contains(&verdict) {
        return Err(VerdictError::UnknownVerdict(verdict.to_string()));
    }
    if !(crate::ids::is_id(from) && from.starts_with("appr_")) {
        return Err(VerdictError::BadField("from"));
    }
    let line = carried_line(uid, title, when, verdict, from, start, end);
    append(vault, &line).map_err(|e| VerdictError::Io(e.to_string()))
}

/// The line [`record_carried_answer`] appends, built in this one place so
/// [`carried_answer_reads_back`] reads exactly those bytes.
fn carried_line(
    uid: &str,
    title: &str,
    when: Date,
    verdict: &str,
    from: &str,
    start: DateTime,
    end: DateTime,
) -> String {
    format!(
        "- {uid} · {} · verdict:{verdict} · by:{CARRY_ACTOR} · from:{from} · start:{} · end:{} · answered {}",
        clean_title(title),
        start.strftime(SPAN_FORMAT),
        end.strftime(SPAN_FORMAT),
        when.strftime("%Y-%m-%d"),
    )
}

/// Would [`load_ledger`] read the carry's line for these arguments back exactly as written: one
/// entry, for `uid`, with `verdict`, [`CARRY_ACTOR`] as `answered_by`, the cleaned title, card
/// `from` and the span `start`..`end`, and no `strength`, `why` or `jid`? Pure: it builds the line
/// [`record_carried_answer`] would append and reads it with the reader's own rules
/// ([`read_lines`], after the same newline translation and `splitlines`).
///
/// The uid, the span and the feed's title all sit on the line unchecked by the writer (review of
/// T2b.5). A uid outside the head's class, a span year that is no `\d{4}`, a title holding a line
/// boundary, or a title that starts with a field head (`verdict:`, `by:`, `from:`, …, which the
/// reader's leftmost search then takes for the field) all read back wrong. The carry writes no
/// line for any of them, since it would be misread, or re-written on every `rank`.
pub(crate) fn carried_answer_reads_back(
    uid: &str,
    title: &str,
    when: Date,
    verdict: &str,
    from: &str,
    start: DateTime,
    end: DateTime,
) -> bool {
    let text = universal_newlines(&carried_line(uid, title, when, verdict, from, start, end));
    let expected = LedgerEntry {
        uid: uid.to_string(),
        verdict: Some(verdict.to_string()),
        answered_by: CARRY_ACTOR.to_string(),
        title: clean_title(title),
        carry: Some(Carried { from: from.to_string(), start, end }),
        ..Default::default()
    };
    read_lines(splitlines(&text), None) == BTreeMap::from([(uid.to_string(), expected)])
}

/// Append a `proposed` marker. Carries no verdict — the guard in [`load_ledger`] depends on that.
pub fn record_proposed(vault: &Path, uid: &str, when: Date) -> Result<(), VerdictError> {
    append(vault, &format!("- {uid} · proposed {}", when.strftime("%Y-%m-%d")))
        .map_err(|e| VerdictError::Io(e.to_string()))
}

/// Append a `declined` marker.
pub fn record_declined(vault: &Path, uid: &str, when: Date) -> Result<(), VerdictError> {
    append(vault, &format!("- {uid} · declined {}", when.strftime("%Y-%m-%d")))
        .map_err(|e| VerdictError::Io(e.to_string()))
}

#[cfg(test)]
mod tests {
    use super::*;

    /// `date(2026, 8, 20)` — the `WHEN` of `tests/test_event_ledger.py`.
    const WHEN: Date = Date::constant(2026, 8, 20);

    /// pytest's `tmp_path`, one directory per test so they can run in parallel.
    fn tmp_vault(name: &str) -> PathBuf {
        let dir =
            std::env::temp_dir().join(format!("qo-eventledger-{}-{}", name, std::process::id()));
        let _ = fs::remove_dir_all(&dir);
        fs::create_dir_all(&dir).unwrap();
        dir
    }

    /// `record_verdict(vault, uid, title, WHEN, verdict)` with the Python keyword defaults.
    fn verdict(vault: &Path, uid: &str, title: &str, v: &str) -> Result<(), VerdictError> {
        record_verdict(vault, uid, title, WHEN, v, "", "", "")
    }

    fn seed(vault: &Path, text: &str) -> PathBuf {
        let path = path_for(vault);
        fs::create_dir_all(path.parent().unwrap()).unwrap();
        fs::write(&path, text).unwrap();
        path
    }

    fn keys(ledger: &BTreeMap<String, LedgerEntry>) -> Vec<&str> {
        ledger.keys().map(String::as_str).collect()
    }

    #[test]
    fn missing_ledger_is_empty() {
        let vault = tmp_vault("missing");
        assert!(load_ledger(&vault, None).is_empty());
    }

    #[test]
    fn verdict_round_trips() {
        let vault = tmp_vault("roundtrip");
        record_verdict(
            &vault,
            "engage:1",
            "AI Club Kickoff",
            WHEN,
            "opportunity",
            "strong",
            "AI Club is on your joined list",
            "",
        )
        .unwrap();
        let ledger = load_ledger(&vault, None);
        let entry = &ledger["engage:1"];
        assert_eq!(entry.verdict.as_deref(), Some("opportunity"));
        assert_eq!(entry.strength, "strong");
        assert_eq!(entry.why, "AI Club is on your joined list");
        assert!(!entry.proposed);
        assert!(!entry.declined);
    }

    #[test]
    fn proposed_and_declined_are_separate_line_kinds() {
        let vault = tmp_vault("markers");
        verdict(&vault, "engage:1", "T", "opportunity").unwrap();
        record_proposed(&vault, "engage:1", WHEN).unwrap();
        record_declined(&vault, "engage:2", WHEN).unwrap();
        let ledger = load_ledger(&vault, None);
        assert_eq!(ledger["engage:1"].verdict.as_deref(), Some("opportunity"));
        assert!(ledger["engage:1"].proposed);
        assert!(ledger["engage:2"].declined);
        assert_eq!(ledger["engage:2"].verdict, None);
    }

    #[test]
    fn ledger_is_append_only() {
        let vault = tmp_vault("append-only");
        verdict(&vault, "a", "A", "drop").unwrap();
        verdict(&vault, "b", "B", "drop").unwrap();
        let text = fs::read_to_string(path_for(&vault)).unwrap();
        assert_eq!(text.matches("verdict:drop").count(), 2);
    }

    #[test]
    fn first_verdict_wins_so_a_uid_can_never_flip() {
        let vault = tmp_vault("first-wins");
        verdict(&vault, "a", "A", "opportunity").unwrap();
        verdict(&vault, "a", "A", "drop").unwrap();
        assert_eq!(load_ledger(&vault, None)["a"].verdict.as_deref(), Some("opportunity"));
    }

    #[test]
    fn malformed_lines_are_ignored_not_fatal() {
        let vault = tmp_vault("malformed");
        seed(
            &vault,
            "# header\n\
             garbage line with no uid\n\
             - engage:9 · Good · 2026-09-01 · verdict:drop · first seen 2026-08-20\n",
        );
        assert_eq!(keys(&load_ledger(&vault, None)), ["engage:9"]);
    }

    #[test]
    fn why_rejects_quotes_and_newlines() {
        assert_eq!(why_problem("plain reason"), None);
        assert!(why_problem("has \"quotes\"").is_some());
        assert!(why_problem("has\nnewline").is_some());
    }

    #[test]
    fn record_verdict_refuses_bad_why() {
        let vault = tmp_vault("bad-why");
        let err = record_verdict(
            &vault, "a", "A", WHEN, "opportunity", "", "bad \"quote\"", "",
        );
        assert_eq!(err, Err(VerdictError::BadWhy("why may not contain a double quote")));
    }

    #[test]
    fn unknown_verdict_is_refused() {
        let vault = tmp_vault("bad-verdict");
        let err = verdict(&vault, "a", "A", "maybe");
        assert_eq!(err, Err(VerdictError::UnknownVerdict("maybe".into())));
        assert_eq!(err.unwrap_err().to_string(), "unknown verdict: 'maybe'");
    }

    #[test]
    fn append_preserves_prior_bytes() {
        // Regression: `append` must not rewrite prior lines' line terminators. Seed the file with
        // LF-only bytes (as a git checkout or a concurrent writer would), append, and verify the
        // original bytes survive byte-for-byte as a prefix.
        let vault = tmp_vault("prior-bytes");
        let path = path_for(&vault);
        fs::create_dir_all(path.parent().unwrap()).unwrap();
        let seeded = format!("{HEADER}\n- engage:1 · Prior Event · verdict:drop · first seen 2026-08-19\n");
        fs::write(&path, seeded.as_bytes()).unwrap();

        verdict(&vault, "engage:2", "New Event", "opportunity").unwrap();

        let new_bytes = fs::read(&path).unwrap();
        assert!(
            new_bytes.starts_with(seeded.as_bytes()),
            "seeded bytes were not preserved as a prefix"
        );
    }

    #[test]
    fn verdict_title_not_confused_with_marker() {
        // Regression: a verdict line whose title starts "declined "/"proposed " matches the
        // marker regexes; only the `"verdict:" not in line` guard keeps it a verdict line.
        let vault = tmp_vault("marker-title");
        verdict(&vault, "x", "declined applications workshop", "opportunity").unwrap();
        let ledger = load_ledger(&vault, None);
        assert_eq!(ledger["x"].verdict.as_deref(), Some("opportunity"));
        assert!(!ledger["x"].declined);

        verdict(&vault, "y", "proposed new feature", "obligation").unwrap();
        let ledger = load_ledger(&vault, None);
        assert_eq!(ledger["y"].verdict.as_deref(), Some("obligation"));
        assert!(!ledger["y"].proposed);
    }

    const HOSTILE_UID: &str = "weird uid/with%stuff\"and+plus";

    #[test]
    fn sanitize_uid_stays_inside_the_ledger_charclass() {
        let clean = sanitize_uid(HOSTILE_UID);
        assert!(UID.is_match(&format!("- {clean} · x")), "{clean}");
    }

    #[test]
    fn sanitize_uid_is_injective() {
        // Two uids that differ only in disallowed characters must not collapse onto one, or two
        // real events would share a single verdict.
        assert_ne!(sanitize_uid("a b"), sanitize_uid("a/b"));
        assert_ne!(sanitize_uid("a+b"), sanitize_uid("a b"));
        assert_eq!(sanitize_uid("plain:123"), "plain:123");
    }

    #[test]
    fn a_hostile_uid_round_trips_mint_to_ledger() {
        // The Python test drives this through `parse_ics_events` and `write_roster`; neither
        // module is ported yet, so this covers the ledger half — the only hop that constrains
        // the charclass. `eventfeed.py` mints with `sanitize_uid`, which is what is asserted.
        let vault = tmp_vault("hostile");
        let uid = sanitize_uid(HOSTILE_UID);
        record_verdict(&vault, &uid, "Odd Event", WHEN, "opportunity", "mild", "", "").unwrap();
        let mut warnings: Vec<String> = Vec::new();
        let ledger = load_ledger(&vault, Some(&mut warnings));
        assert!(warnings.is_empty(), "{warnings:?}");
        assert_eq!(ledger[&uid].verdict.as_deref(), Some("opportunity"));
        assert_eq!(ledger[&uid].strength, "mild");
    }

    #[test]
    fn an_unreadable_verdict_line_warns() {
        let vault = tmp_vault("unreadable");
        seed(
            &vault,
            "# header\n\
             - weird uid/here · Bad · verdict:drop · first seen 2026-08-20\n\
             - engage:9 · Good · verdict:drop · first seen 2026-08-20\n",
        );
        let mut warnings: Vec<String> = Vec::new();
        let ledger = load_ledger(&vault, Some(&mut warnings));
        assert_eq!(keys(&ledger), ["engage:9"]);
        assert_eq!(warnings.len(), 1);
        assert!(warnings[0].contains("unreadable verdict line"), "{warnings:?}");
    }

    #[test]
    fn an_unknown_verdict_token_is_skipped_with_a_warning() {
        let vault = tmp_vault("unknown-token");
        seed(
            &vault,
            "- engage:9 · Maybe · verdict:maybe · first seen 2026-08-20\n\
             - engage:9 · Maybe · verdict:drop · first seen 2026-08-21\n",
        );
        let mut warnings: Vec<String> = Vec::new();
        let ledger = load_ledger(&vault, Some(&mut warnings));
        assert!(warnings.iter().any(|w| w.contains("unknown verdict")), "{warnings:?}");
        // Skipped, not accepted: the later valid line is the one that settles the uid.
        assert_eq!(ledger["engage:9"].verdict.as_deref(), Some("drop"));
    }

    #[test]
    fn load_ledger_without_a_warnings_list_is_unchanged() {
        let vault = tmp_vault("no-warnings");
        seed(&vault, "- bad uid! · X · verdict:nope · first seen 2026-08-20\n");
        assert!(load_ledger(&vault, None).is_empty());
    }

    #[test]
    fn carriage_return_in_title_is_sanitized() {
        // Regression: a title with `\r` must be sanitized or `splitlines()` corrupts the file.
        let vault = tmp_vault("carriage-return");
        verdict(&vault, "z", "event\rwith carriage return", "drop").unwrap();
        let ledger = load_ledger(&vault, None);
        assert_eq!(ledger["z"].verdict.as_deref(), Some("drop"));
        // Header plus one entry, and no third line.
        let text = read_text(&path_for(&vault)).unwrap();
        assert_eq!(splitlines(&text).len(), 2);
    }

    // --- Quirks the Python has that the port must keep -------------------------------------

    #[test]
    fn a_unicode_line_separator_in_a_title_loses_the_verdict_silently() {
        // GENUINE BUG, PRESERVED: `record_verdict` sanitizes `\n` and `\r` but not U+2028, which
        // `str.splitlines()` also treats as a boundary. The line splits on read, neither half
        // parses, and — because the tail does not start with "- " and the head has no
        // "verdict:" — nothing warns.
        let vault = tmp_vault("u2028");
        verdict(&vault, "u", "split\u{2028}title", "drop").unwrap();
        let mut warnings: Vec<String> = Vec::new();
        let ledger = load_ledger(&vault, Some(&mut warnings));
        assert!(ledger.is_empty(), "{ledger:?}");
        assert!(warnings.is_empty(), "the loss is silent: {warnings:?}");
    }

    #[test]
    fn a_non_lowercase_strength_is_dropped_without_a_warning() {
        // `_STRENGTH` is `[a-z]+` but nothing validates the field on write.
        let vault = tmp_vault("strength");
        record_verdict(&vault, "a", "A", WHEN, "drop", "Strong", "", "").unwrap();
        record_verdict(&vault, "b", "B", WHEN, "drop", "very strong", "", "").unwrap();
        let mut warnings: Vec<String> = Vec::new();
        let ledger = load_ledger(&vault, Some(&mut warnings));
        assert_eq!(ledger["a"].strength, "");
        assert_eq!(ledger["b"].strength, "very");
        assert!(warnings.is_empty());
    }

    #[test]
    fn a_uid_containing_the_substring_verdict_breaks_its_marker_lines() {
        // GENUINE BUG, PRESERVED: `:` is in the uid charclass and `sanitize_uid` keeps it, so a
        // uid like `x:verdict:1` fails the `"verdict:" not in line` guard on its own marker line.
        let vault = tmp_vault("verdict-uid");
        record_proposed(&vault, "x:verdict:1", WHEN).unwrap();
        let mut warnings: Vec<String> = Vec::new();
        let ledger = load_ledger(&vault, Some(&mut warnings));
        assert!(ledger.is_empty(), "the proposed marker is not recorded: {ledger:?}");
        assert_eq!(warnings.len(), 1);
        assert!(warnings[0].contains("unreadable verdict line"), "{warnings:?}");
    }

    #[test]
    fn an_empty_title_becomes_untitled() {
        let vault = tmp_vault("untitled");
        verdict(&vault, "e", "   ", "drop").unwrap();
        let text = fs::read_to_string(path_for(&vault)).unwrap();
        assert!(text.contains("- e · (untitled) · verdict:drop"), "{text}");
    }

    #[test]
    fn a_middle_dot_in_a_title_cannot_forge_a_field() {
        let vault = tmp_vault("dot-title");
        verdict(&vault, "d", "A · B", "drop").unwrap();
        let text = fs::read_to_string(path_for(&vault)).unwrap();
        assert!(text.contains("- d · A - B · verdict:drop"), "{text}");
    }

    #[test]
    fn shown_truncates_by_characters_not_bytes() {
        // Every ledger line is dense with `·` (two bytes). Byte slicing would cut early and could
        // land mid-character.
        let line = "·".repeat(100);
        let out = shown_within(&line, 90);
        assert_eq!(out.chars().count(), 91, "90 code points plus the ellipsis");
        assert_eq!(shown_within("short", 90), "short");
    }

    #[test]
    fn an_unreadable_file_warns_and_returns_empty() {
        let vault = tmp_vault("bad-utf8");
        let path = path_for(&vault);
        fs::create_dir_all(path.parent().unwrap()).unwrap();
        fs::write(&path, [0xffu8, 0xfe, 0x00]).unwrap();
        let mut warnings: Vec<String> = Vec::new();
        assert!(load_ledger(&vault, Some(&mut warnings)).is_empty());
        assert_eq!(warnings.len(), 1);
        assert!(warnings[0].starts_with("ledger unreadable: "), "{warnings:?}");
    }

    #[test]
    fn crlf_written_by_a_prior_run_reads_back_cleanly() {
        // Python's text-mode write produces CRLF on Windows; the live vault's ledger is CRLF.
        // Universal-newline decoding is what makes that invisible to the parser.
        let vault = tmp_vault("crlf");
        seed(&vault, "# header\r\n- engage:9 · Good · verdict:drop · first seen 2026-08-20\r\n");
        let mut warnings: Vec<String> = Vec::new();
        let ledger = load_ledger(&vault, Some(&mut warnings));
        assert_eq!(keys(&ledger), ["engage:9"]);
        assert!(warnings.is_empty(), "{warnings:?}");
    }

    #[test]
    fn the_header_is_written_once_and_only_when_the_file_is_new() {
        let vault = tmp_vault("header");
        verdict(&vault, "a", "A", "drop").unwrap();
        verdict(&vault, "b", "B", "drop").unwrap();
        let text = read_text(&path_for(&vault)).unwrap();
        assert_eq!(text.matches(HEADER).count(), 1);
        assert_eq!(splitlines(&text)[0], HEADER);
    }

    // --- F1: the ledger learns human answers, and `jid:` ------------------------------------

    /// A well-formed judgment id, for tests that need one.
    const J: &str = "12345678-1234-5678-1234-567812345678";

    /// `record_judged_verdict(vault, uid, title, WHEN, verdict, "", judgment_id)`.
    fn judged(
        vault: &Path,
        uid: &str,
        title: &str,
        v: &str,
        judgment_id: Option<&str>,
    ) -> Result<(), VerdictError> {
        record_judged_verdict(vault, uid, title, WHEN, v, "", judgment_id)
    }

    #[test]
    fn a_human_answer_replaces_unsure() {
        let vault = tmp_vault("answer-replaces-unsure");
        judged(&vault, "e", "Career Fair", "unsure", Some(J)).unwrap();
        record_answer(&vault, "e", "Career Fair", WHEN, "obligation", "quinn", Some(J)).unwrap();
        let ledger = load_ledger(&vault, None);
        let entry = &ledger["e"];
        assert_eq!(entry.verdict.as_deref(), Some("obligation"));
        assert_eq!(entry.answered_by, "quinn");
        assert_eq!(entry.judgment_id, J);
    }

    #[test]
    fn a_human_answer_never_replaces_a_confident_verdict() {
        let vault = tmp_vault("answer-does-not-replace-drop");
        verdict(&vault, "e", "Career Fair", "drop").unwrap();
        record_answer(&vault, "e", "Career Fair", WHEN, "obligation", "quinn", None).unwrap();
        let ledger = load_ledger(&vault, None);
        let entry = &ledger["e"];
        assert_eq!(entry.verdict.as_deref(), Some("drop"));
        assert_eq!(entry.answered_by, "");
    }

    #[test]
    fn the_first_human_answer_wins() {
        let vault = tmp_vault("first-answer-wins");
        judged(&vault, "e", "Career Fair", "unsure", None).unwrap();
        record_answer(&vault, "e", "Career Fair", WHEN, "obligation", "quinn", None).unwrap();
        record_answer(&vault, "e", "Career Fair", WHEN, "drop", "quinn", None).unwrap();
        assert_eq!(load_ledger(&vault, None)["e"].verdict.as_deref(), Some("obligation"));
    }

    #[test]
    fn an_answer_before_any_verdict_is_just_a_verdict() {
        let vault = tmp_vault("answer-alone");
        record_answer(&vault, "e", "Career Fair", WHEN, "obligation", "quinn", None).unwrap();
        let ledger = load_ledger(&vault, None);
        let entry = &ledger["e"];
        assert_eq!(entry.verdict.as_deref(), Some("obligation"));
        assert_eq!(entry.answered_by, "quinn");
        assert_eq!(entry.title, "Career Fair");
    }

    #[test]
    fn the_answer_line_is_byte_exact() {
        let vault = tmp_vault("answer-byte-exact");
        let when = Date::constant(2026, 10, 1);
        record_answer(&vault, "engage:1", "Career Fair", when, "obligation", "quinn", Some(J))
            .unwrap();
        let text = fs::read_to_string(path_for(&vault)).unwrap();
        let expected = format!(
            "{HEADER}{NEWLINE}- engage:1 · Career Fair · verdict:obligation · by:quinn · jid:{J} · answered 2026-10-01{NEWLINE}"
        );
        assert_eq!(text, expected);
    }

    #[test]
    fn a_judged_verdict_line_carries_jid_and_round_trips() {
        let vault = tmp_vault("judged-jid");
        judged(&vault, "e", "Career Fair", "unsure", Some(J)).unwrap();
        let text = fs::read_to_string(path_for(&vault)).unwrap();
        let expected = format!(
            "{HEADER}{NEWLINE}- e · Career Fair · verdict:unsure · jid:{J} · first seen 2026-08-20{NEWLINE}"
        );
        assert_eq!(text, expected);
        let ledger = load_ledger(&vault, None);
        let entry = &ledger["e"];
        assert_eq!(entry.verdict.as_deref(), Some("unsure"));
        assert_eq!(entry.judgment_id, J);
    }

    #[test]
    fn record_verdict_bytes_are_unchanged() {
        let vault = tmp_vault("verdict-bytes-unchanged");
        record_verdict(
            &vault,
            "engage:1",
            "AI Club Kickoff",
            WHEN,
            "opportunity",
            "strong",
            "AI Club is on your joined list",
            "",
        )
        .unwrap();
        let text = fs::read_to_string(path_for(&vault)).unwrap();
        let expected = format!(
            "{HEADER}{NEWLINE}- engage:1 · AI Club Kickoff · verdict:opportunity · strength:strong · why:\"AI Club is on your joined list\" · first seen 2026-08-20{NEWLINE}"
        );
        assert_eq!(text, expected);
    }

    #[test]
    fn record_answer_refuses_a_bad_word_actor_or_jid() {
        let vault = tmp_vault("answer-refuses-bad-fields");
        assert_eq!(
            record_answer(&vault, "e", "T", WHEN, "opportunity", "quinn", None),
            Err(VerdictError::UnknownVerdict("opportunity".into()))
        );
        assert_eq!(
            record_answer(&vault, "e", "T", WHEN, "obligation", "a b", None),
            Err(VerdictError::BadField("by"))
        );
        assert_eq!(
            record_answer(&vault, "e", "T", WHEN, "obligation", "quinn", Some("x")),
            Err(VerdictError::BadField("jid"))
        );
        assert!(!path_for(&vault).exists(), "no line was appended by a refused call");
    }

    #[test]
    fn an_answer_title_is_sanitised_like_a_verdict_title() {
        let vault = tmp_vault("answer-title-sanitised");
        record_answer(
            &vault,
            "e",
            "Career fair · Thu 1 Oct 10am–3pm · +3 more",
            WHEN,
            "obligation",
            "quinn",
            None,
        )
        .unwrap();
        record_answer(&vault, "f", "line\rone\nand two", WHEN, "drop", "quinn", None).unwrap();
        let text = fs::read_to_string(path_for(&vault)).unwrap();
        assert!(
            text.contains(
                "- e · Career fair - Thu 1 Oct 10am–3pm - +3 more · verdict:obligation · by:quinn"
            ),
            "{text}"
        );
        assert!(text.contains("- f · line one and two · verdict:drop · by:quinn"), "{text}");
        let ledger = load_ledger(&vault, None);
        assert_eq!(ledger["e"].verdict.as_deref(), Some("obligation"));
        assert_eq!(ledger["e"].answered_by, "quinn");
        assert_eq!(ledger["f"].verdict.as_deref(), Some("drop"));
        assert_eq!(ledger["f"].answered_by, "quinn");
    }

    #[test]
    fn the_first_verdict_lines_title_is_kept() {
        let vault = tmp_vault("title-kept");
        judged(&vault, "e", "Career Fair", "unsure", None).unwrap();
        record_answer(&vault, "e", "Career Fair (updated)", WHEN, "obligation", "quinn", None)
            .unwrap();
        let ledger = load_ledger(&vault, None);
        assert_eq!(ledger["e"].title, "Career Fair");
    }

    // --- F1 fix round 1 (review) -------------------------------------------------------------

    #[test]
    fn a_why_cannot_forge_an_answer_or_a_jid() {
        // GENUINE FORGERY, CLOSED (review I-1): `why_problem` refuses `"`, a newline and ` · `
        // (space-dot-space), but not a bare `·` with no surrounding space, and `events.rs`
        // sanitises only ` · ` before writing a scraped why. Without stripping `why:"…"` before
        // searching for `by:`/`jid:`/`answered`, this why would forge a human answer onto a
        // machine's own `unsure` verdict.
        let vault = tmp_vault("why-cannot-forge");
        record_judged_verdict(
            &vault,
            "e",
            "Career Fair",
            WHEN,
            "unsure",
            "a· by:x· jid:12345678-1234-5678-1234-567812345678· answered 2026-01-01",
            None,
        )
        .unwrap();
        let ledger = load_ledger(&vault, None);
        let entry = &ledger["e"];
        assert_eq!(entry.verdict.as_deref(), Some("unsure"));
        assert_eq!(entry.answered_by, "", "the why field must not forge by:");
        assert_eq!(entry.judgment_id, "", "the why field must not forge jid:");

        // A real answer still supersedes the (still-`unsure`) verdict afterwards.
        record_answer(&vault, "e", "Career Fair", WHEN, "obligation", "quinn", Some(J)).unwrap();
        let ledger = load_ledger(&vault, None);
        let entry = &ledger["e"];
        assert_eq!(entry.verdict.as_deref(), Some("obligation"));
        assert_eq!(entry.answered_by, "quinn");
        assert_eq!(entry.judgment_id, J);
    }

    #[test]
    fn an_answer_shaped_line_with_a_non_answer_word_does_not_supersede() {
        // Review m-1: a hand-edited or otherwise malformed answer-shaped line carrying a word
        // outside `ANSWER_VERDICTS` (here `opportunity`, a word a person may not give) must not
        // settle the uid — and, critically, must not leave `unsure` open to a *later* line
        // superseding it a second time (which would break "the first human answer wins").
        let vault = tmp_vault("answer-word-checked");
        seed(
            &vault,
            "- e · Career Fair · verdict:unsure · first seen 2026-08-20\n\
             - e · Career Fair · verdict:opportunity · by:quinn · answered 2026-08-21\n",
        );
        let ledger = load_ledger(&vault, None);
        let entry = &ledger["e"];
        assert_eq!(entry.verdict.as_deref(), Some("unsure"), "unsure must stay open");
        assert_eq!(entry.answered_by, "");
    }

    #[test]
    fn title_reads_from_the_older_python_written_line_shape_too() {
        // Review m-2: `vault-full/state/events-seen.md` (frozen, Python-written) puts a bare
        // date segment between the title and `verdict:`. `TITLE` must read a title from that
        // shape too, not only from the shape this crate writes.
        let vault = tmp_vault("title-python-shape");
        seed(
            &vault,
            "- ics:fixture-1 · Undergraduate Research Symposium · 2026-08-26 · \
             verdict:opportunity · strength:strong · first seen 2026-08-20\n",
        );
        let ledger = load_ledger(&vault, None);
        assert_eq!(ledger["ics:fixture-1"].title, "Undergraduate Research Symposium");
    }

    // --- PQ3 (b-prime): the carry's line and its read rules (P15, T2b.4) ----------------------

    use crate::journal::HUMAN_ACTOR;

    /// The answering card the carry's tests name.
    const CARD: &str = "appr_0123456789";

    /// A second judgment id, so a test can tell an answer's id from the `unsure` line's.
    const J2: &str = "87654321-4321-8765-4321-876543218765";

    /// `2026-10-<day>T<hour>:00:00`.
    fn at(day: i8, hour: i8) -> DateTime {
        jiff::civil::date(2026, 10, day).at(hour, 0, 0, 0)
    }

    /// The carry's line for uid `e`, "Career fair", answered 2026-10-01.
    fn carried(
        vault: &Path,
        w: &str,
        from: &str,
        start: DateTime,
        end: DateTime,
    ) -> Result<(), VerdictError> {
        let day = Date::constant(2026, 10, 1);
        record_carried_answer(vault, "e", "Career fair", day, w, from, start, end)
    }

    fn carry_of(from: &str, start: DateTime, end: DateTime) -> Option<Carried> {
        Some(Carried { from: from.to_string(), start, end })
    }

    #[test]
    fn the_carry_line_is_byte_exact() {
        let vault = tmp_vault("carry-byte-exact");
        record_carried_answer(
            &vault,
            "lx:77:3",
            "Career fair",
            Date::constant(2026, 10, 1),
            "opportunity",
            "appr_0123456789",
            jiff::civil::date(2026, 10, 8).at(10, 0, 0, 0),
            jiff::civil::date(2026, 10, 8).at(15, 0, 0, 0),
        )
        .unwrap();
        let text = fs::read_to_string(path_for(&vault)).unwrap();
        let expected = format!(
            "{HEADER}{NEWLINE}- lx:77:3 · Career fair · verdict:opportunity · by:agent:knowlu.carry · from:appr_0123456789 · start:2026-10-08T10:00:00 · end:2026-10-08T15:00:00 · answered 2026-10-01{NEWLINE}"
        );
        assert_eq!(text, expected);
        assert!(crate::provenance::is_agent(CARRY_ACTOR));
    }

    #[test]
    fn the_carry_line_refuses_a_bad_word_or_card_id() {
        let vault = tmp_vault("carry-refuses");
        for w in ["drop", "unsure", "maybe"] {
            assert_eq!(
                carried(&vault, w, CARD, at(8, 10), at(8, 15)),
                Err(VerdictError::UnknownVerdict(w.into())),
                "{w}"
            );
        }
        for from in ["", "x", "task_0123456789"] {
            assert_eq!(
                carried(&vault, "obligation", from, at(8, 10), at(8, 15)),
                Err(VerdictError::BadField("from")),
                "{from:?}"
            );
        }
        assert!(!path_for(&vault).exists(), "no file was created by a refused call");
    }

    #[test]
    fn a_carry_line_reads_back_its_verdict_actor_card_and_span() {
        let vault = tmp_vault("carry-reads-back");
        carried(&vault, "opportunity", CARD, at(8, 10), at(8, 15)).unwrap();
        let ledger = load_ledger(&vault, None);
        let entry = &ledger["e"];
        assert_eq!(entry.verdict.as_deref(), Some("opportunity"));
        assert_eq!(entry.answered_by, CARRY_ACTOR);
        assert_eq!(entry.title, "Career fair");
        assert_eq!(entry.carry, carry_of(CARD, at(8, 10), at(8, 15)));
    }

    #[test]
    fn a_carry_line_settles_an_unanswered_unsure_with_either_word() {
        for w in ["obligation", "opportunity"] {
            let vault = tmp_vault(&format!("carry-settles-unsure-{w}"));
            judged(&vault, "e", "Career fair", "unsure", Some(J)).unwrap();
            carried(&vault, w, CARD, at(8, 10), at(8, 15)).unwrap();
            let ledger = load_ledger(&vault, None);
            let entry = &ledger["e"];
            assert_eq!(entry.verdict.as_deref(), Some(w));
            assert_eq!(entry.answered_by, CARRY_ACTOR, "{w}");
            assert_eq!(entry.carry, carry_of(CARD, at(8, 10), at(8, 15)), "{w}");
            assert_eq!(entry.judgment_id, J, "{w}: the unsure line's judgment id stays");
        }
    }

    #[test]
    fn a_carry_line_never_flips_a_confident_verdict_but_keeps_its_span() {
        for machine in ["drop", "opportunity"] {
            let vault = tmp_vault(&format!("carry-keeps-confident-{machine}"));
            verdict(&vault, "e", "Career fair", machine).unwrap();
            carried(&vault, "obligation", CARD, at(8, 10), at(8, 15)).unwrap();
            let ledger = load_ledger(&vault, None);
            let entry = &ledger["e"];
            assert_eq!(entry.verdict.as_deref(), Some(machine));
            assert_eq!(entry.answered_by, "", "{machine}");
            assert_eq!(entry.carry, carry_of(CARD, at(8, 10), at(8, 15)), "{machine}");
        }
    }

    #[test]
    fn a_later_human_answer_wins_over_a_carry_line() {
        // With a `jid` on the answer, the answer's id wins; without one, the unsure line's stays.
        for (answer_jid, kept) in [(Some(J2), J2), (None, J)] {
            let vault = tmp_vault(&format!("human-after-carry-{}", answer_jid.is_some()));
            judged(&vault, "e", "Career fair", "unsure", Some(J)).unwrap();
            carried(&vault, "obligation", CARD, at(8, 10), at(8, 15)).unwrap();
            let day = Date::constant(2026, 10, 2);
            record_answer(&vault, "e", "Career fair", day, "drop", HUMAN_ACTOR, answer_jid)
                .unwrap();
            let ledger = load_ledger(&vault, None);
            let entry = &ledger["e"];
            assert_eq!(entry.verdict.as_deref(), Some("drop"));
            assert_eq!(entry.answered_by, HUMAN_ACTOR);
            assert_eq!(entry.judgment_id, kept);
            assert_eq!(entry.carry, None, "a human answer clears the carry");
        }
    }

    #[test]
    fn a_carry_line_never_overrides_a_human_answer() {
        // The human answer after an `unsure`, and the human answer as the uid's first line.
        for after_unsure in [true, false] {
            let vault = tmp_vault(&format!("carry-under-human-{after_unsure}"));
            if after_unsure {
                judged(&vault, "e", "Career fair", "unsure", Some(J)).unwrap();
            }
            record_answer(&vault, "e", "Career fair", WHEN, "obligation", HUMAN_ACTOR, None)
                .unwrap();
            carried(&vault, "opportunity", CARD, at(8, 10), at(8, 15)).unwrap();
            let ledger = load_ledger(&vault, None);
            let entry = &ledger["e"];
            assert_eq!(entry.verdict.as_deref(), Some("obligation"), "{after_unsure}");
            assert_eq!(entry.answered_by, HUMAN_ACTOR, "{after_unsure}");
            assert_eq!(entry.carry, None, "{after_unsure}");
        }
    }

    #[test]
    fn only_the_first_carry_line_counts() {
        let vault = tmp_vault("first-carry-counts");
        carried(&vault, "opportunity", CARD, at(8, 10), at(8, 15)).unwrap();
        carried(&vault, "obligation", "appr_abcdefabcd", at(9, 0), at(10, 0)).unwrap();
        let ledger = load_ledger(&vault, None);
        let entry = &ledger["e"];
        assert_eq!(entry.verdict.as_deref(), Some("opportunity"));
        assert_eq!(entry.answered_by, CARRY_ACTOR);
        assert_eq!(entry.carry, carry_of(CARD, at(8, 10), at(8, 15)));
    }

    #[test]
    fn an_agent_answer_line_without_a_card_or_span_is_neither_kind() {
        let lines = [
            "- e · Career fair · verdict:obligation · by:agent:x · answered 2026-10-01",
            "- e · Career fair · verdict:obligation · by:agent:x · from:appr_0123456789 · \
             start:2026-10-08T10:00:00 · answered 2026-10-01",
        ];
        for (i, line) in lines.iter().enumerate() {
            let vault = tmp_vault(&format!("agent-line-neither-{i}"));
            judged(&vault, "e", "Career fair", "unsure", None).unwrap();
            append(&vault, line).unwrap();
            let ledger = load_ledger(&vault, None);
            let entry = &ledger["e"];
            assert_eq!(entry.verdict.as_deref(), Some("unsure"), "{line}");
            assert_eq!(entry.answered_by, "", "{line}");
            assert_eq!(entry.carry, None, "{line}");
            // Alone, it is an ordinary verdict line: its word stands and no one answered.
            let alone = tmp_vault(&format!("agent-line-alone-{i}"));
            append(&alone, line).unwrap();
            let ledger = load_ledger(&alone, None);
            let entry = &ledger["e"];
            assert_eq!(entry.verdict.as_deref(), Some("obligation"), "{line}");
            assert_eq!(entry.answered_by, "", "{line}");
            assert_eq!(entry.carry, None, "{line}");
        }
    }

    #[test]
    fn a_why_cannot_forge_a_carry_line() {
        let vault = tmp_vault("why-cannot-forge-carry");
        record_judged_verdict(
            &vault,
            "e",
            "Career fair",
            WHEN,
            "unsure",
            "a· by:agent:knowlu.carry· from:appr_0123456789· start:2026-10-08T00:00:00· \
             end:2026-10-09T00:00:00· answered 2026-10-01",
            None,
        )
        .unwrap();
        let ledger = load_ledger(&vault, None);
        let entry = &ledger["e"];
        assert_eq!(entry.verdict.as_deref(), Some("unsure"));
        assert_eq!(entry.carry, None, "the why field must not forge a carry");
        assert_eq!(entry.answered_by, "", "the why field must not forge by:");
    }

    #[test]
    fn the_carry_line_reads_back_only_as_written() {
        // Review of T2b.5: `carried_answer_reads_back` builds the line `record_carried_answer`
        // writes and reads it with `load_ledger`'s own rules. Each case is also written to a vault
        // and read by `load_ledger`, and the check must agree with that read.
        let day = Date::constant(2026, 10, 1);
        let (start, end) = (at(8, 10), at(8, 15));
        let year = |y: i16| jiff::civil::date(y, 1, 1).at(0, 0, 0, 0);
        let jid_title = format!("jid:{J2} Career fair");
        let cases: [(&str, &str, DateTime, bool); 17] = [
            ("e", "Career fair", end, true),
            ("e", "Career fair · verdict:tbd", end, true), // `·` is cleaned: no field head
            ("e", "proposed Career fair", end, true),
            ("e", "answered 2026-01-01 Career fair", end, true),
            ("e", "task:lab Career fair", end, true), // no reader reads a `task:` field
            ("e", "verdict:tbd Career fair", end, false),
            ("e", "verdict:drop Career fair", end, false),
            ("e", "strength:strong Career fair", end, false),
            ("e", "why:\"x\" Career fair", end, false),
            ("e", "by:appointment Career fair", end, false),
            ("e", &jid_title, end, false),
            ("e", "from:appr_abcdefabcd Career fair", end, false),
            ("e", "start:2026-01-01T00:00:00 Career fair", end, false),
            ("e", "Career\u{2028}fair", end, false),
            ("e", "Career fair", year(-1), false), // `-0001-…` is no `\d{4}` year
            ("lx 77", "Career fair", end, false),
            ("lx:77:verdict:2", "Career fair", end, true), // a uid's own `verdict:` breaks nothing
        ];
        for (i, (uid, title, end, reads)) in cases.into_iter().enumerate() {
            let check = carried_answer_reads_back(uid, title, day, "opportunity", CARD, start, end);
            assert_eq!(check, reads, "{uid:?} {title:?}");
            let vault = tmp_vault(&format!("carry-reads-back-only-{i}"));
            record_carried_answer(&vault, uid, title, day, "opportunity", CARD, start, end).unwrap();
            let entry = load_ledger(&vault, None).remove(uid);
            let as_written = entry.as_ref().is_some_and(|e| {
                e.verdict.as_deref() == Some("opportunity")
                    && (e.strength.as_str(), e.why.as_str(), e.judgment_id.as_str()) == ("", "", "")
                    && e.answered_by == CARRY_ACTOR
                    && e.carry == carry_of(CARD, start, end)
                    && e.title == clean_title(title)
            });
            assert_eq!(as_written, reads, "{uid:?} {title:?}: load_ledger gave {entry:?}");
        }
    }

    #[test]
    fn the_frozen_ledger_reads_as_before() {
        let frozen = Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("tests/fixtures/vault-full/state/events-seen.md");
        let bytes = fs::read(&frozen).unwrap();
        let vault = tmp_vault("frozen-ledger");
        let copy = path_for(&vault);
        fs::create_dir_all(copy.parent().unwrap()).unwrap();
        fs::write(&copy, &bytes).unwrap();
        let ledger = load_ledger(&vault, None);
        assert_eq!(ledger.len(), 3, "the frozen ledger's three verdicts");
        for entry in ledger.values() {
            assert_eq!(entry.carry, None, "{}", entry.uid);
            assert_eq!(entry.answered_by, "", "{}", entry.uid);
        }
        assert_eq!(fs::read(&copy).unwrap(), bytes, "reading never changes the bytes");
    }
}
