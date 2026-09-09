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

use std::collections::BTreeMap;
use std::fs;
use std::io::Write as _;
use std::path::{Path, PathBuf};
use std::sync::LazyLock;

use jiff::civil::Date;
use regex::Regex;

/// Written once, when the file is first created. The em dash is part of the byte stream.
pub const HEADER: &str = "# Event verdict ledger — append-only. One verdict per uid, forever; \
delete a line to force a deliberate re-judge.";

/// The complete verdict vocabulary. Other modules match these strings exactly; nothing else is
/// accepted on write, and anything else is skipped with a warning on read.
pub const VALID_VERDICTS: [&str; 3] = ["obligation", "opportunity", "drop"];

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
    /// Python: an uncaught `OSError` from the append.
    Io(String),
}

impl std::fmt::Display for VerdictError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            // `{verdict!r}` on a str is single-quoted.
            VerdictError::UnknownVerdict(v) => write!(f, "unknown verdict: '{v}'"),
            VerdictError::BadWhy(m) => write!(f, "{m}"),
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
    let mut entries: BTreeMap<String, LedgerEntry> = BTreeMap::new();
    let text = match read_text(&path) {
        Ok(text) => text,
        Err(err) => {
            // Python catches `(OSError, UnicodeDecodeError)` together and returns an empty
            // ledger — a *missing* verdict, not a fatal run.
            warn(&mut warnings, format!("ledger unreadable: {err}"));
            return BTreeMap::new();
        }
    };
    for line in splitlines(&text) {
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
        if current.verdict.is_some() {
            continue; // first verdict wins — the invariant that keeps reads unambiguous
        }
        let strength = STRENGTH.captures(line);
        let why = WHY.captures(line);
        entries.insert(
            uid.to_string(),
            LedgerEntry {
                uid: uid.to_string(),
                verdict: Some(token.to_string()),
                strength: strength
                    .map(|c| c.name("strength").unwrap().as_str().to_string())
                    .unwrap_or_default(),
                why: why.map(|c| c.name("why").unwrap().as_str().to_string()).unwrap_or_default(),
                proposed: current.proposed,
                declined: current.declined,
            },
        );
    }
    entries
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

/// Append a verdict line for `uid`.
///
/// Python's `strength`, `why` and `task` are keyword arguments defaulting to `""`; pass `""` for
/// each here. An empty field is omitted from the line entirely.
///
/// **Only `title` and `why` are checked.** `uid`, `strength` and `task` go onto the line
/// unexamined, so a uid with a space in it writes a permanently unreadable verdict (which is why
/// `eventfeed` calls [`sanitize_uid`] at mint), an uppercase strength reads back as `""`, and a
/// `task` containing ` · why:"…"` would inject a why field. Ported as found.
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
    if !VALID_VERDICTS.contains(&verdict) {
        return Err(VerdictError::UnknownVerdict(verdict.to_string()));
    }
    // `if why:` — an empty why skips validation, which is harmless because it is then omitted.
    if !why.is_empty() {
        if let Some(problem) = why_problem(why) {
            return Err(VerdictError::BadWhy(problem));
        }
    }
    // `·` becomes `-` so a title can never forge a field separator. `\n`/`\r` become spaces so a
    // title can never split the line. U+2028 and friends are *not* handled — see the module doc.
    let cleaned: String = title.replace('·', "-").replace('\n', " ").replace('\r', " ");
    let cleaned = cleaned.trim_matches(is_python_space);
    let clean_title = if cleaned.is_empty() { "(untitled)" } else { cleaned };

    let mut line = format!("- {uid} · {clean_title} · verdict:{verdict}");
    if !strength.is_empty() {
        line.push_str(&format!(" · strength:{strength}"));
    }
    if !why.is_empty() {
        line.push_str(&format!(" · why:\"{why}\""));
    }
    if !task.is_empty() {
        line.push_str(&format!(" · task:{task}"));
    }
    line.push_str(&format!(" · first seen {}", when.strftime("%Y-%m-%d")));
    append(vault, &line).map_err(|e| VerdictError::Io(e.to_string()))
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
}
