//! Port of `engine/ingest.py` — frontmatter line surgery, the ICS parser, and uid dedup.
//!
//! # The most destructive function in the codebase
//!
//! [`apply_frontmatter_fields_to_text`] edits **one line** of a note and leaves every other byte
//! alone — comments, key order, odd spacing, the body, the trailing newline. `CLAUDE.md`: *never
//! rewrite a vault file wholesale.* A `serde` round-trip here would silently reformat the entire
//! vault on first write, and Obsidian Git would push it within five minutes.
//!
//! There is exactly one implementation, and both the dry run and the real write go through it —
//! a simulation and the writer can disagree, and every disagreement is a corrupted note.
//!
//! # CRLF
//!
//! Every file in this repo is CRLF. Python's `read_text` translates it away before any of this
//! code sees it, and `write_text` puts it back. Reads here go through [`crate::pystr::read_text`]
//! and writes through [`crate::pystr::write_text`] for exactly that reason. Read a note raw and
//! `lines.index("---", 1)` — an **exact** match — never fires, because the line is `"---\r"`.

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};
use std::sync::LazyLock;

use jiff::civil::{Date, DateTime};
use jiff::tz::TimeZone;
use regex::Regex;

use crate::models::split_frontmatter;
use crate::pystr;

/// A due value is a date **or** a datetime, and the difference is observable: [`format_due`]
/// renders them differently, and that string lands in frontmatter.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Due {
    Date(Date),
    DateTime(DateTime),
}

#[derive(Debug, Clone, PartialEq)]
pub struct Event {
    pub uid: String,
    pub title: String,
    pub due: Option<Due>,
    pub description: String,
    pub raw: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum IngestError {
    NoFrontmatter(String),
    Io(String),
}

impl std::fmt::Display for IngestError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            IngestError::NoFrontmatter(label) => write!(f, "{label} has no frontmatter"),
            IngestError::Io(m) => write!(f, "{m}"),
        }
    }
}

// ---------------------------------------------------------------------------
// Frontmatter surgery
// ---------------------------------------------------------------------------

/// Replace or insert frontmatter fields, touching only the lines that change.
///
/// `changes` is an **ordered** slice, not a map: Python iterates a dict in insertion order and the
/// insertion point `end` shifts as keys are appended, so a different order can produce a different
/// file. Order is part of the contract.
///
/// Rules, each of which is observable:
/// - The document must start with `---`, else `ValueError`.
/// - The closing marker is the first line **exactly equal** to `---` at index >= 1.
/// - A key matches on the `"{key}:"` prefix; the trailing colon is what stops `due` matching
///   `due_alt`.
/// - Only the **first** matching line is replaced. A duplicated key keeps its duplicate.
/// - A key with no existing line is inserted immediately **before** the closing `---`.
/// - The value is written raw — callers pre-quote (`source_uid='"zybooks:123"'`).
pub fn apply_frontmatter_fields_to_text(
    text: &str,
    changes: &[(String, String)],
    label: &str,
) -> Result<String, IngestError> {
    if !text.starts_with("---") {
        return Err(IngestError::NoFrontmatter(label.to_string()));
    }
    // split('\n'), NOT lines(): the round trip must be lossless, including a trailing newline,
    // which split leaves as a final empty element and join restores.
    let mut lines: Vec<String> = text.split('\n').map(|s| s.to_string()).collect();
    let Some(mut end) = lines.iter().skip(1).position(|l| l == "---").map(|i| i + 1) else {
        // Python's list.index raises ValueError here; the message differs but every caller
        // treats it as the same "cannot edit this note" failure.
        return Err(IngestError::NoFrontmatter(label.to_string()));
    };

    for (key, value) in changes {
        let replacement = format!("{key}: {value}");
        let prefix = format!("{key}:");
        let mut replaced = false;
        for line in lines.iter_mut().take(end).skip(1) {
            if line.starts_with(&prefix) {
                *line = replacement.clone();
                replaced = true;
                break;
            }
        }
        if !replaced {
            lines.insert(end, replacement);
            end += 1;
        }
    }
    Ok(lines.join("\n"))
}

/// Read, edit, write — through the CRLF-translating helpers, exactly as Python's text mode does.
pub fn update_frontmatter_fields(
    path: &Path,
    changes: &[(String, String)],
) -> Result<(), IngestError> {
    let text = pystr::read_text(path).map_err(|e| IngestError::Io(e.to_string()))?;
    let out = apply_frontmatter_fields_to_text(&text, changes, &path.display().to_string())?;
    pystr::write_text(path, &out).map_err(|e| IngestError::Io(e.to_string()))
}

// ---------------------------------------------------------------------------
// ICS parsing
// ---------------------------------------------------------------------------

/// Unfold RFC 5545 continuation lines: a line beginning with a space or tab continues the previous
/// one, minus that first character.
pub fn unfold(text: &str) -> Vec<String> {
    let normalised = pystr::universal_newlines(text);
    let mut out: Vec<String> = Vec::new();
    for line in normalised.split('\n') {
        let first = line.chars().next();
        if (first == Some(' ') || first == Some('\t')) && !out.is_empty() {
            let rest: String = line.chars().skip(1).collect();
            out.last_mut().unwrap().push_str(&rest);
        } else {
            out.push(line.to_string());
        }
    }
    out
}

/// RFC 5545 text unescaping.
///
/// The NUL sentinel is Python's, and it matters: escaping `\\` first, parking it as `\0`, then
/// restoring it last is what stops `\\n` (an escaped backslash followed by `n`) being read as a
/// newline.
pub fn unescape(value: &str) -> String {
    let staged = value.replace("\\\\", "\0");
    let staged = staged.replace("\\n", "\n").replace("\\N", "\n");
    let staged = staged.replace("\\,", ",").replace("\\;", ";");
    staged.replace('\0', "\\")
}

/// `(NAME, params, value)` from a content line, or `None` if there is no colon.
pub fn parse_property(line: &str) -> Option<(String, BTreeMap<String, String>, String)> {
    let (head, value) = line.split_once(':')?;
    let mut parts = head.split(';');
    let name = parts.next().unwrap_or("").to_uppercase();
    let mut params = BTreeMap::new();
    for part in parts {
        if let Some((key, val)) = part.split_once('=') {
            params.insert(key.to_uppercase(), val.to_string());
        }
    }
    Some((name, params, value.to_string()))
}

static ALL_DIGITS_8: LazyLock<Regex> = LazyLock::new(|| Regex::new(r"^\d{8}$").unwrap());

/// Parse a DATE or DATE-TIME into the feed's local wall time.
///
/// A `Z` value is UTC and is converted into `tz` then stripped of its zone; a `TZID` value is
/// converted the same way. A bare value is already local and is taken as-is. The result is always
/// naive, because that is what the vault stores.
pub fn parse_dt(value: &str, params: &BTreeMap<String, String>, tz: &TimeZone) -> Option<Due> {
    if params.get("VALUE").map(|v| v.to_uppercase()) == Some("DATE".to_string())
        || ALL_DIGITS_8.is_match(value)
    {
        let y: i16 = value.get(0..4)?.parse().ok()?;
        let m: i8 = value.get(4..6)?.parse().ok()?;
        let d: i8 = value.get(6..8)?.parse().ok()?;
        return Date::new(y, m, d).ok().map(Due::Date);
    }
    let trimmed = value.strip_suffix('Z').unwrap_or(value);
    let naive = DateTime::strptime("%Y%m%dT%H%M%S", trimmed).ok()?;

    if value.ends_with('Z') {
        let zoned = naive.to_zoned(TimeZone::UTC).ok()?.with_time_zone(tz.clone());
        return Some(Due::DateTime(zoned.datetime()));
    }
    if let Some(tzid) = params.get("TZID") {
        let from = TimeZone::get(tzid).ok()?;
        let zoned = naive.to_zoned(from).ok()?.with_time_zone(tz.clone());
        return Some(Due::DateTime(zoned.datetime()));
    }
    Some(Due::DateTime(naive))
}

static NON_SLUG: LazyLock<Regex> = LazyLock::new(|| Regex::new(r"[^a-z0-9]+").unwrap());

/// Lowercase, collapse non-alphanumeric runs to `-`, trim, cap at **60 characters** (not bytes),
/// trim again, and fall back to `item` when nothing survives.
pub fn slugify(text: &str) -> String {
    let lowered = text.to_lowercase();
    let dashed = NON_SLUG.replace_all(&lowered, "-");
    let trimmed = dashed.trim_matches('-');
    let capped: String = trimmed.chars().take(60).collect();
    let capped = capped.trim_end_matches('-');
    if capped.is_empty() {
        "item".to_string()
    } else {
        capped.to_string()
    }
}

/// Blackboard's combined "My Calendar" feed mixes real course deliverables with institution-wide
/// noise — registrar deadlines, holidays, other schools' exam periods. Those are well-formed
/// VEVENTs with valid titles and dates; they are recognisable only by UID namespace.
pub const NOISE_UID_MARKERS: [&str; 1] = ["blackboard.data.calendar.calendarentry"];

pub fn is_institutional_noise(uid: &str) -> bool {
    let lowered = uid.to_lowercase();
    NOISE_UID_MARKERS.iter().any(|m| lowered.contains(m))
}

/// Case-insensitive search for `fragment` bounded by non-alphanumerics on both sides.
///
/// This is the port of the codebase's **one and only lookaround**
/// (`(?<![A-Za-z0-9])…(?![A-Za-z0-9])`, `ingest.py:114`). Rust's `regex` has no lookaround, so the
/// boundary test is done in code: find each case-insensitive occurrence, then check the characters
/// either side. `[A-Za-z0-9]` is ASCII-only in the original, so `is_ascii_alphanumeric` is exact.
/// This is what keeps `CS-100` from matching inside `STATISTICS-100-001`.
fn contains_bounded(haystack: &str, fragment: &str) -> bool {
    let pattern = format!("(?i){}", regex::escape(fragment));
    let Ok(re) = Regex::new(&pattern) else { return false };
    for m in re.find_iter(haystack) {
        let before_ok = haystack[..m.start()]
            .chars()
            .next_back()
            .is_none_or(|c| !c.is_ascii_alphanumeric());
        let after_ok = haystack[m.end()..]
            .chars()
            .next()
            .is_none_or(|c| !c.is_ascii_alphanumeric());
        if before_ok && after_ok {
            return true;
        }
    }
    false
}

/// A gradebook uid pin in its one form, `_<digits>_<digits>` exactly (§14.46).
fn is_pin_form(fragment: &str) -> bool {
    let mut parts = fragment.split('_');
    let digits = |p: Option<&str>| p.is_some_and(|p| !p.is_empty() && p.bytes().all(|b| b.is_ascii_digit()));
    parts.next() == Some("") && digits(parts.next()) && digits(parts.next()) && parts.next().is_none()
}

/// Pass (a) for one fragment: a case-sensitive substring of the uid. A fragment in the pin form
/// matches only where no ASCII letter, digit or `_` precedes it and no ASCII digit follows it, so
/// `_71_1` claims neither `…-_71_10` nor `…-_5_71_1`. Every other fragment keeps the substring test.
fn uid_claims(uid: &str, fragment: &str) -> bool {
    if !is_pin_form(fragment) {
        return uid.contains(fragment);
    }
    uid.match_indices(fragment).any(|(start, found)| {
        let before_ok = uid[..start].chars().next_back().is_none_or(|c| !(c.is_ascii_alphanumeric() || c == '_'));
        let after_ok = uid[start + found.len()..].chars().next().is_none_or(|c| !c.is_ascii_digit());
        before_ok && after_ok
    })
}

/// The two passes [`match_course`] runs, over haystacks the caller supplies.
///
/// Split out so `judge`'s tier-1 course heuristic is the same rule this module applies to a feed
/// and not a second one that can drift from it: an enrichment that attributed a course differently
/// from the ingest would make the same assignment land under two slugs depending on which producer
/// saw it first. `match_course` is this function plus the SUMMARY/CATEGORIES extraction; nothing
/// about the ordering, the case sensitivity or the boundary test changed when it moved here.
///
/// **What tier 1 shares with the ingest is this rule, not its haystack.** `match_course` passes
/// SUMMARY and CATEGORIES joined; tier 1 passes only the note's title, and only when the note's
/// own `course` field is already empty — i.e. on exactly the items the ICS ingest did not
/// attribute, plus anything Gmail- or hand-created that never passed through it.
pub fn match_course_fields(
    uid: &str,
    haystack: &str,
    course_map: &[(String, String)],
) -> Option<String> {
    for (fragment, slug) in course_map {
        if uid_claims(uid, fragment) {
            return Some(slug.clone());
        }
    }
    for (fragment, slug) in course_map {
        if contains_bounded(haystack, fragment) {
            return Some(slug.clone());
        }
    }
    None
}

/// Attribute an event to a course.
///
/// Two passes, and the order is load-bearing:
/// **(a) uid pins** — a fragment that is a *case-sensitive* substring of the raw uid always wins.
/// This is how course-less gradebook items get pinned by their Blackboard item id. A fragment in
/// the pin form `_<digits>_<digits>` is bounded (see [`uid_claims`]): `_71_1` does not claim
/// `…-_71_10` or `…-_5_71_1`.
/// **(b) course codes** — matched only against `SUMMARY` and `CATEGORIES` values, never
/// `DESCRIPTION`, which would attribute an event by an incidental mention.
pub fn match_course(event: &Event, course_map: &[(String, String)]) -> Option<String> {
    let mut fields: Vec<String> = Vec::new();
    for line in event.raw.split('\n') {
        if let Some((name, _params, value)) = parse_property(line) {
            if name == "SUMMARY" || name == "CATEGORIES" {
                fields.push(unescape(&value));
            }
        }
    }
    match_course_fields(&event.uid, &fields.join("\n"), course_map)
}

static TRAILING_BRACKET: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"\s*\[[^\]]*\]\s*$").unwrap());

fn event_from(block: &[String], tz: &TimeZone) -> Option<Event> {
    let mut fields: BTreeMap<String, String> = BTreeMap::new();
    let mut params_by: BTreeMap<String, BTreeMap<String, String>> = BTreeMap::new();
    for line in block {
        if let Some((name, params, value)) = parse_property(line) {
            // setdefault: the FIRST occurrence of a property wins.
            fields.entry(name.clone()).or_insert(value);
            params_by.entry(name).or_insert(params);
        }
    }
    let uid = fields.get("UID")?.clone();
    let summary = fields.get("SUMMARY").cloned().unwrap_or_default();
    let title = unescape(&summary);
    let title = pystr::strip(&title);
    let title = TRAILING_BRACKET.replace(title, "");
    let title = pystr::strip(&title).to_string();
    if uid.is_empty() || title.is_empty() {
        return None;
    }

    let mut due = None;
    for key in ["DUE", "DTEND", "DTSTART"] {
        if let Some(raw) = fields.get(key) {
            let empty = BTreeMap::new();
            let params = params_by.get(key).unwrap_or(&empty);
            // An unparseable date makes the whole event malformed — Python returns None here
            // rather than falling through to the next key.
            due = Some(parse_dt(raw, params, tz)?);
            break;
        }
    }

    let description = fields.get("DESCRIPTION").cloned().unwrap_or_default();
    Some(Event {
        uid,
        title,
        due,
        description: pystr::strip(&unescape(&description)).to_string(),
        raw: block.join("\n"),
    })
}

/// Returns `(events, malformed_count)`.
pub fn parse_ics(text: &str, tz: &TimeZone) -> (Vec<Event>, usize) {
    let mut events = Vec::new();
    let mut malformed = 0usize;
    let mut block: Option<Vec<String>> = None;
    for line in unfold(text) {
        if line == "BEGIN:VEVENT" {
            block = Some(Vec::new());
        } else if line == "END:VEVENT" && block.is_some() {
            let finished = block.take().unwrap();
            match event_from(&finished, tz) {
                None => malformed += 1,
                Some(event) => events.push(event),
            }
        } else if let Some(b) = block.as_mut() {
            b.push(line);
        }
    }
    (events, malformed)
}

pub fn format_due(value: Due) -> String {
    match value {
        Due::DateTime(dt) => dt.strftime("%Y-%m-%dT%H:%M").to_string(),
        Due::Date(d) => d.strftime("%Y-%m-%d").to_string(),
    }
}

// ---------------------------------------------------------------------------
// uid dedup and the seen ledger
// ---------------------------------------------------------------------------

static SOURCE_UID_LINE: LazyLock<Regex> = LazyLock::new(|| {
    // (?m) for Python's re.MULTILINE; `[^"\r\n]+?` is non-greedy exactly as in the original.
    Regex::new(r#"(?m)^source_uid:\s*"?([^"\r\n]+?)"?\s*$"#).unwrap()
});

/// Index every note in `tasks/` and `archive/` by its `source_uid`.
///
/// **The fallback is load-bearing.** When the YAML will not parse, Python does *not* give up: it
/// runs a regex over the raw text so the note still registers its uid. Propagating the error
/// instead — Rust's instinct — drops the note out of dedup, and the next sync creates a duplicate
/// task file for it. That is the exact failure S3 exists to prevent.
///
/// First writer wins: `if uid and uid not in found`.
pub fn existing_by_uid(vault: &Path) -> BTreeMap<String, PathBuf> {
    let mut found: BTreeMap<String, PathBuf> = BTreeMap::new();
    for folder in ["tasks", "archive"] {
        let dir = vault.join(folder);
        let mut paths: Vec<PathBuf> = match std::fs::read_dir(&dir) {
            Ok(entries) => entries
                .filter_map(|e| e.ok())
                .map(|e| e.path())
                .filter(|p| p.extension().map(|x| x == "md").unwrap_or(false))
                .collect(),
            Err(_) => continue,
        };
        paths.sort();

        for path in paths {
            let Ok(text) = pystr::read_text(&path) else { continue };
            let mut uid: Option<String> = None;
            let readable = match split_frontmatter(&text) {
                Ok((meta, _)) => {
                    uid = meta
                        .get(serde_yaml_ng::Value::String("source_uid".into()))
                        .and_then(crate::yaml::text);
                    true
                }
                Err(_) => false,
            };
            if !readable {
                match SOURCE_UID_LINE.captures(&text) {
                    Some(caps) => uid = Some(caps[1].to_string()),
                    None => {
                        println!("skipped (unreadable): {}", path.display());
                        continue;
                    }
                }
            }
            if let Some(uid) = uid.filter(|u| !u.is_empty()) {
                found.entry(uid).or_insert(path);
            }
        }
    }
    found
}

pub const SEEN_LEDGER_HEADER: &str =
    "# Blackboard events already imported once. A UID listed here whose note has\n\
     # been deleted will NOT be re-created by ingest. Delete a line to re-import.\n";

/// Every uid ever ingested. A uid here whose note was deleted is **not** re-created.
pub fn load_seen(vault: &Path) -> std::collections::BTreeSet<String> {
    let path = vault.join("state").join("ingest-seen.md");
    let mut seen = std::collections::BTreeSet::new();
    let Ok(text) = pystr::read_text(&path) else { return seen };
    for line in pystr::splitlines(&text) {
        let Some(rest) = line.strip_prefix("- ") else { continue };
        let uid = rest.split(" · ").next().unwrap_or("");
        let uid = pystr::strip(uid);
        if !uid.is_empty() {
            seen.insert(uid.to_string());
        }
    }
    seen
}

pub fn record_seen(vault: &Path, uid: &str, title: &str, date_str: &str) -> std::io::Result<()> {
    let state_dir = vault.join("state");
    std::fs::create_dir_all(&state_dir)?;
    let path = state_dir.join("ingest-seen.md");
    let mut text = if path.exists() {
        pystr::read_text(&path)?
    } else {
        SEEN_LEDGER_HEADER.to_string()
    };
    text.push_str(&format!("- {uid} · {title} · first seen {date_str}\n"));
    pystr::write_text(&path, &text)
}

/// R-OB-3: is this the first ingest this vault has ever had?
///
/// **The absence of `today.md`, and deliberately not the absence of `state/ingest-seen.md`**
/// (ruling R-C2-9). That ledger is not `ingest`'s alone: `coursework` imports
/// `crate::ingest::record_seen` (`coursework.rs:19`) and calls it at `:310` and `:415`, and the
/// slot order is `sync → coursework → grades → ingest → judge → rank`. So on a genuinely fresh vault the
/// seen-ledger already exists by the time `ingest` looks at it, and a flag derived from it is
/// `false` on exactly the run R-OB-3 was written for. `today.md` is written by `rank`, the LAST
/// step of the slot, so the first `ingest` always sees it absent and every later one sees it
/// present.
///
/// It is also the predicate the app already uses to decide a vault needs its first slot at all
/// (`app/src/scheduler.rs::needs_first_run`, main `f8649d5`), so the two halves of "this vault has
/// never been through a slot" now agree by construction rather than by coincidence.
///
/// **The residue, and it is acceptable:** a first slot that dies before `rank` leaves `today.md`
/// absent, so the *next* slot is "first" again and archives whatever has gone past in between.
/// Nothing is double-archived — the uid is already in the seen-ledger — and no page has been
/// rendered for the student to have seen the difference, because `today.md` not existing is
/// precisely the premise.
pub fn is_first_run(vault: &Path) -> bool {
    !vault.join("state").join("today.md").exists()
}

/// Did the due date actually change? An unparseable old value counts as changed.
pub fn due_changed(old: Option<&serde_yaml_ng::Value>, new_due: &str) -> bool {
    let parsed_new = crate::models::coerce_datetime(Some(&serde_yaml_ng::Value::String(
        new_due.to_string(),
    )));
    let parsed_old = crate::models::coerce_datetime(old);
    match (parsed_old, parsed_new) {
        (Ok(a), Ok(b)) => a != b,
        _ => true,
    }
}

// ---------------------------------------------------------------------------
// sync_tasks
// ---------------------------------------------------------------------------

pub const NOTE_TEMPLATE: &str = r#"---
title: {title}
course: {course}
domain: school
due: {due}
effort_hours: 1.0
effort_confidence: low
effort_source: inferred
importance: 3
importance_reason: "pending enrichment"
status: active
progress: 0
created_by: blackboard
source_uid: {uid}
needs_enrichment: true
---

{body}
"#;

/// R-OB-3: why a note went straight to `archive/` on a first ingest. A frontmatter field rather
/// than a naming convention, so a human reading the note in six months can see it, and so a future
/// `surface` view can filter on it without parsing a filename.
pub const IMPORTED_PAST: &str = "imported-past";

/// O1: why a note went straight to `archive/` because its source already says it is turned in.
/// Beside [`IMPORTED_PAST`]; precedence: a past item is `imported-past`, never this.
pub const IMPORTED_DONE: &str = "imported-done";

/// [`IMPORTED_PAST_TEMPLATE`] with `archived_reason: imported-done` ([`IMPORTED_DONE`]) where the
/// past template says `imported-past`: a second template, for the reason the past one gives.
pub const IMPORTED_DONE_TEMPLATE: &str = r#"---
title: {title}
course: {course}
domain: school
due: {due}
effort_hours: 1.0
effort_confidence: low
effort_source: inferred
importance: 3
importance_reason: "pending enrichment"
status: archived
archived_reason: imported-done
progress: 0
created_by: blackboard
source_uid: {uid}
needs_enrichment: false
---

{body}
"#;

/// The prefix of a Blackboard feed task's UID that names a gradebook column; the rest is the
/// column id the grade note keeps as `source_uid` (`_4686399_1`).
const GRADABLE_PREFIX: &str = "_blackboard.platform.gradebook2.GradableItem-";

/// The gradebook column id inside a feed task's UID: the suffix after exactly
/// `_blackboard.platform.gradebook2.GradableItem-`, non-empty, else `None`. No other UID shape
/// (a calendar entry, an assignment-style `BB-ASSIGN-...`) ever joins the gradebook.
pub fn gradable_id(uid: &str) -> Option<&str> {
    uid.strip_prefix(GRADABLE_PREFIX).filter(|id| !id.is_empty())
}

/// Whether a grade note's `status` and `score` say the work is turned in (plan §3, Q-A, spec §6.2):
/// `needs-grading`, `exempt`, and `graded` with a score above 0. Everything else is live -- a graded
/// zero, and a `graded` row with no score (every column type that yields one can hold a zero or a
/// fail), included.
pub fn gradebook_done(status: &str, score: Option<f64>) -> bool {
    match status {
        "needs-grading" | "exempt" => true,
        "graded" => score.is_some_and(|s| s > 0.0),
        _ => false,
    }
}

/// What ingest reads of one Blackboard grade note: its `status` and `score`.
#[derive(Debug, Clone, PartialEq)]
pub struct GradeRow {
    pub status: String,
    pub score: Option<f64>,
}

/// Blackboard grade items by column id (the note's `source_uid`), read from `grades/`.
pub type GradeIndex = BTreeMap<String, GradeRow>;

/// A read-only index of `grades/`: per `kind: item` note with `source: blackboard`, its column id,
/// `status` and `score` -- the fields `grades::desired` writes. Built once per run, in path order
/// (the first note for a column id wins); a note that does not parse is skipped. No `grades/`
/// folder is an empty index, which is today's behaviour exactly.
///
/// **This reads `grades/` without a lock, and a concurrent write can race it.** `ingest` takes no
/// `RunLock`, while the app's grades refresh, grades connect and Sync now write `grades/` under one,
/// and `pystr::write_text` truncates then writes. A read that lands in that window can make
/// `read_meta` miss a note, which only means a turned-in item is created live this run and marked
/// done (with Undo) on the next. `ingest` never writes `grades/`, so nothing is corrupted.
pub fn grade_index(vault: &Path) -> GradeIndex {
    let mut index = GradeIndex::new();
    for path in crate::approvals::sorted_md(&vault.join("grades")) {
        let Some(meta) = crate::ids::read_meta(&path) else { continue };
        let text = |key: &str| crate::yaml::get(&meta, key).and_then(crate::yaml::text).filter(|t| !t.is_empty());
        if text("kind").as_deref().unwrap_or("item") != "item"
            || text("source").as_deref() != Some(crate::grades::SOURCE)
        {
            continue;
        }
        let (Some(id), Some(status)) = (text("source_uid"), text("status")) else { continue };
        let score = crate::yaml::get(&meta, "score").and_then(crate::yaml::f64_of);
        index.entry(id).or_insert(GradeRow { status, score });
    }
    index
}

/// The grade row for a feed task's UID, with its column id, if it joins the gradebook and the row
/// says turned in.
fn done_grade<'a>(grades: &'a GradeIndex, uid: &str) -> Option<(&'a str, &'a GradeRow)> {
    let id = gradable_id(uid)?;
    let (column, row) = grades.get_key_value(id)?;
    gradebook_done(&row.status, row.score).then_some((column.as_str(), row))
}

/// Every ingest, after the sync: each feed event, **in feed order**, whose note is an active task
/// joined to a turned-in grade row is marked done by [`crate::completion::apply_done`] (which also
/// skips a task the student set by hand, or one with a pending or rejected "Mark done" card).
/// Evidence is `{source: "blackboard-gradebook", status, column}`. One line per task marked:
/// `marked done: <stem> (gradebook)`. An event that left the feed is not visited.
fn mark_gradebook_done(
    events: &[Event],
    vault: &Path,
    grades: &GradeIndex,
    ctx: &crate::write::WriteContext,
    journal: &mut crate::journal::Journal,
) -> Vec<String> {
    let mut log = Vec::new();
    if grades.is_empty() {
        return log;
    }
    let known = existing_by_uid(vault);
    for event in events {
        let Some((column, row)) = done_grade(grades, &event.uid) else { continue };
        let Some(path) = known.get(&event.uid) else { continue };
        let mut detail = serde_json::Map::new();
        detail.insert("status".to_string(), serde_json::Value::String(row.status.clone()));
        detail.insert("column".to_string(), serde_json::Value::String(column.to_string()));
        let evidence = crate::completion::Evidence {
            source: "blackboard-gradebook".to_string(),
            summary: "Blackboard's gradebook shows this submitted".to_string(),
            detail,
        };
        let stem = path.file_stem().map(|s| s.to_string_lossy().to_string()).unwrap_or_default();
        match crate::completion::apply_done(vault, path, &evidence, ctx, journal, false) {
            Ok(crate::completion::Applied::Marked) => log.push(format!("marked done: {stem} (gradebook)")),
            Ok(_) => {}
            Err(err) => log.push(format!("gradebook: {stem} not marked done ({err})")),
        }
    }
    log
}

/// The archived twin of `NOTE_TEMPLATE`. Identical but for the two lines that say why it is here —
/// deliberately a second template rather than a substitution on the first, because the two differ
/// in what they MEAN and a reader should not have to diff them to see it.
pub const IMPORTED_PAST_TEMPLATE: &str = r#"---
title: {title}
course: {course}
domain: school
due: {due}
effort_hours: 1.0
effort_confidence: low
effort_source: inferred
importance: 3
importance_reason: "pending enrichment"
status: archived
archived_reason: imported-past
progress: 0
created_by: blackboard
source_uid: {uid}
needs_enrichment: false
---

{body}
"#;

/// `json.dumps(s)` with Python's **default** `ensure_ascii=True`: every non-ASCII character becomes
/// a lowercase `\uXXXX` escape, with a surrogate pair above the BMP.
///
/// Used only by the update branch of [`sync_tasks`], because that is what Python does there — see
/// defect 13 in the preserved-defects report. The create branch uses [`json_dumps_unicode`].
fn json_dumps_ascii(s: &str) -> String {
    let mut out = String::from("\"");
    for c in s.chars() {
        match c {
            '"' => out.push_str("\\\""),
            '\\' => out.push_str("\\\\"),
            '\n' => out.push_str("\\n"),
            '\r' => out.push_str("\\r"),
            '\t' => out.push_str("\\t"),
            '\u{8}' => out.push_str("\\b"),
            '\u{c}' => out.push_str("\\f"),
            c if (c as u32) < 0x20 => out.push_str(&format!("\\u{:04x}", c as u32)),
            c if c.is_ascii() => out.push(c),
            c => {
                let cp = c as u32;
                if cp > 0xFFFF {
                    let v = cp - 0x10000;
                    out.push_str(&format!("\\u{:04x}\\u{:04x}", 0xD800 + (v >> 10), 0xDC00 + (v & 0x3FF)));
                } else {
                    out.push_str(&format!("\\u{cp:04x}"));
                }
            }
        }
    }
    out.push('"');
    out
}

/// `json.dumps(s, ensure_ascii=False)` — an em dash stays an em dash.
fn json_dumps_unicode(s: &str) -> String {
    serde_json::to_string(s).unwrap_or_else(|_| format!("{s:?}"))
}

/// Sync parsed events into `tasks/`, creating and surgically updating notes.
///
/// The order of the guards is the whole logic, and each one exists for a live reason:
/// 0. **Removed course** (`course_exclude`, [`sync_tasks_excluding`]; this entry point passes none):
///    an item only an excluded fragment claims is left out whole, before every guard below.
/// 1. **Institutional noise** with no course match is skipped — registrar deadlines and other
///    schools' exam periods are well-formed VEVENTs that are not assignments.
/// 2. **No date** is skipped: the ranking engine cannot place an undated task.
/// 3. **Known by uid** → update in place. Never falls through to creation, even when the note is
///    unreadable — falling through would produce a duplicate `-2` file, which is the failure S3
///    exists to prevent.
/// 4. **Seen but absent** → the note was deleted deliberately; do not resurrect it.
///
/// `today` is injectable only so tests are deterministic; production passes `None` and gets the
/// local date, exactly as Python's `datetime.now()` does.
pub fn sync_tasks(
    events: &[Event],
    vault: &Path,
    course_map: &[(String, String)],
    ctx: Option<&crate::write::WriteContext>,
    journal: &mut crate::journal::Journal,
    today: Option<Date>,
    // R-OB-3. `true` only on a vault that has never been through a whole slot — `is_first_run`
    // above, which is the absence of `today.md` and **not** of the seen-ledger (R-C2-9).
    // `run_lines` computes it; every caller in the tests passes `false`, which is the behaviour
    // they were written against.
    first_run: bool,
) -> Vec<String> {
    sync_tasks_with_grades(events, vault, course_map, ctx, journal, today, first_run, &GradeIndex::new())
}

/// [`sync_tasks`] plus the gradebook index (O1, spec §6.2): on a first ingest, an item that is not
/// past but joins a turned-in grade row ([`done_grade`]) goes straight to `archive/` as
/// `imported-done`, by the same rule as `imported-past` and below it in precedence.
#[allow(clippy::too_many_arguments)]
pub fn sync_tasks_with_grades(
    events: &[Event],
    vault: &Path,
    course_map: &[(String, String)],
    ctx: Option<&crate::write::WriteContext>,
    journal: &mut crate::journal::Journal,
    today: Option<Date>,
    first_run: bool,
    grades: &GradeIndex,
) -> Vec<String> {
    sync_tasks_excluding(events, vault, course_map, &[], ctx, journal, today, first_run, grades)
}

/// True when no kept `course_map` fragment claims the item and an excluded one does (§14.45).
fn is_left_out(event: &Event, course_map: &[(String, String)], course_exclude: &[(String, String)]) -> bool {
    !course_exclude.is_empty()
        && match_course(event, course_map).is_none()
        && match_course(event, course_exclude).is_some()
}

/// [`sync_tasks_with_grades`] plus `course_exclude` (plan wave B F6c, §14.45): an item that no
/// `course_map` fragment claims but an excluded fragment does belongs to a course the student
/// removed, so it is left out whole: no task, no update, no archive, no journal record, and **no
/// seen-ledger entry** (re-adding the course must import it). A kept course always wins. One log
/// line names the count when it is above zero. An empty `course_exclude` is today's behaviour.
#[allow(clippy::too_many_arguments)]
pub fn sync_tasks_excluding(
    events: &[Event],
    vault: &Path,
    course_map: &[(String, String)],
    course_exclude: &[(String, String)],
    ctx: Option<&crate::write::WriteContext>,
    journal: &mut crate::journal::Journal,
    today: Option<Date>,
    first_run: bool,
    grades: &GradeIndex,
) -> Vec<String> {
    let default_ctx = crate::write::WriteContext::new("agent:ingest.blackboard", "cli");
    let ctx = ctx.unwrap_or(&default_ctx);
    let stamp_date = today.unwrap_or_else(|| jiff::Zoned::now().date());
    let stamp = stamp_date.strftime("%Y-%m-%d").to_string();

    let mut log: Vec<String> = Vec::new();
    let mut known = existing_by_uid(vault);
    let mut seen = load_seen(vault);
    let tasks_dir = vault.join("tasks");
    let _ = std::fs::create_dir_all(&tasks_dir);

    let mut left_out = 0usize;
    for event in events {
        if is_left_out(event, course_map, course_exclude) {
            left_out += 1;
            continue;
        }
        if is_institutional_noise(&event.uid) && match_course(event, course_map).is_none() {
            log.push(format!("skipped (institutional calendar entry): {}", event.title));
            continue;
        }
        let Some(due) = event.due else {
            log.push(format!("skipped (no date): {}", event.title));
            continue;
        };
        let new_due = format_due(due);

        if let Some(path) = known.get(&event.uid).cloned() {
            let stem = path.file_stem().map(|s| s.to_string_lossy().to_string()).unwrap_or_default();
            let Ok(text) = pystr::read_text(&path) else {
                log.push(format!("skipped (unreadable): {stem}"));
                continue;
            };
            let Ok((meta, _)) = split_frontmatter(&text) else {
                // Known by uid (possibly via the regex fallback) but the frontmatter cannot be read
                // to apply an update. Skip outright — never fall through to creation.
                log.push(format!("skipped (unreadable): {stem}"));
                continue;
            };
            let status = crate::yaml::get(&meta, "status")
                .and_then(crate::yaml::text)
                .unwrap_or_else(|| "active".to_string());

            if !seen.contains(&event.uid) {
                // The note exists but its uid never reached the ledger. Backfill so the ledger
                // cannot drift out of sync with what is actually on disk.
                let _ = record_seen(vault, &event.uid, &event.title, &stamp);
                seen.insert(event.uid.clone());
            }
            let in_archive =
                path.parent().and_then(|p| p.file_name()).map(|n| n == "archive") == Some(true);
            if in_archive || status != "active" {
                continue;
            }

            // Ruling 3 and email spec D11 (5) (plan T8b): a field the student edited by hand
            // (`Journal::human_edited`) that the feed contradicts is proposed on its own card and
            // never written; every other field the feed changed is written exactly as before.
            let note_id = crate::yaml::get(&meta, "id").and_then(crate::yaml::text).filter(|id| crate::ids::is_id(id));
            let mut by_hand = |field: &str| note_id.as_deref().is_some_and(|id| journal.human_edited(id, field).is_some());
            let mut changes: Vec<(String, String)> = Vec::new();
            let mut contradicted: Vec<(&str, String)> = Vec::new();
            let current_title = crate::yaml::get(&meta, "title")
                .and_then(crate::yaml::text)
                .unwrap_or_default();
            if current_title != event.title {
                if by_hand("title") {
                    contradicted.push(("title", event.title.clone()));
                } else {
                    // ensure_ascii=True here, unlike the create branch below. Preserved defect 13.
                    changes.push(("title".to_string(), json_dumps_ascii(&event.title)));
                }
            }
            if due_changed(crate::yaml::get(&meta, "due"), &new_due) {
                if by_hand("due") {
                    contradicted.push(("due", new_due.clone()));
                } else {
                    changes.push(("due".to_string(), new_due.clone()));
                }
            }
            let rel_path = crate::ids::rel(vault, &path);
            if !changes.is_empty() {
                if crate::write::write_literals(
                    vault,
                    &rel_path,
                    &changes,
                    ctx,
                    journal,
                    &crate::write::WriteOpts::default(),
                )
                .is_ok()
                {
                    let mut names: Vec<&str> = changes.iter().map(|(n, _)| n.as_str()).collect();
                    names.sort();
                    log.push(format!("updated {stem}: {}", names.join(", ")));
                }
            }
            for (field, to) in contradicted {
                match propose_feed_value(vault, &rel_path, &stem, &meta, field, &to, ctx, journal, stamp_date) {
                    Ok(Some(_)) => log.push(format!("proposed {stem}: {field}")),
                    Ok(None) => {}
                    Err(err) => log.push(format!("skipped (unwritable): {err}")),
                }
            }
            continue;
        }

        if seen.contains(&event.uid) {
            log.push(format!("skipped (deleted earlier): {}", event.title));
            continue;
        }

        let course = match_course(event, course_map);

        // R-OB-3: a feed's window reaches backwards, and a vault born today has no history to
        // reconcile against — Quinn's first slot imported four items already past due, one of them
        // from 2025. On a FIRST ingest such an item is recorded as seen and written straight into
        // `archive/`, so the first page a student ever sees shows the future.
        //
        // **Not skipped** — skipping leaves the uid unseen and the next run creates it. **Not
        // created-then-deleted** — that is two journal records and a note that briefly ranks. One
        // `create` into `archive/`, one `record_seen`, one line.
        //
        // Strictly before TODAY, never before *now*: an item due at 23:59 today is today's work,
        // and the one thing worse than importing a stale task is archiving a live one.
        let past_due = first_run
            && match due {
                Due::Date(d) => d < stamp_date,
                Due::DateTime(dt) => dt.date() < stamp_date,
            };
        // O1: turned in, per the gradebook -- on a first ingest only, and only when not already past.
        let import = if past_due {
            Some((IMPORTED_PAST_TEMPLATE, IMPORTED_PAST))
        } else if first_run && done_grade(grades, &event.uid).is_some() {
            Some((IMPORTED_DONE_TEMPLATE, IMPORTED_DONE))
        } else {
            None
        };
        if let Some((template, reason)) = import {
            let slug = format!("{}-{}", course.clone().unwrap_or_else(|| "task".into()), slugify(&event.title));
            let archive_dir = vault.join("archive");
            let _ = std::fs::create_dir_all(&archive_dir);
            let mut path = archive_dir.join(format!("{slug}.md"));
            let mut suffix = 2;
            while path.exists() {
                path = archive_dir.join(format!("{slug}-{suffix}.md"));
                suffix += 1;
            }
            let body = template
                .replace("{title}", &json_dumps_unicode(&event.title))
                .replace(
                    "{course}",
                    &course.as_deref().map(json_dumps_unicode).unwrap_or_else(|| "null".to_string()),
                )
                .replace("{due}", &new_due)
                .replace("{uid}", &json_dumps_unicode(&event.uid))
                .replace("{body}", &event.description);
            let rel_path = crate::ids::rel(vault, &path);
            match crate::write::create(vault, &rel_path, &body, ctx, journal, None) {
                Ok(created) => {
                    let stem = created.file_stem().map(|s| s.to_string_lossy().to_string()).unwrap_or_default();
                    log.push(format!("archived ({reason}) {stem}"));
                    known.insert(event.uid.clone(), created);
                    let _ = record_seen(vault, &event.uid, &event.title, &stamp);
                }
                Err(err) => log.push(format!("skipped (unwritable): {err}")),
            }
            continue;
        }

        let slug = format!("{}-{}", course.clone().unwrap_or_else(|| "task".into()), slugify(&event.title));
        let mut path = tasks_dir.join(format!("{slug}.md"));
        let mut suffix = 2;
        while path.exists() {
            path = tasks_dir.join(format!("{slug}-{suffix}.md"));
            suffix += 1;
        }

        let body = NOTE_TEMPLATE
            .replace("{title}", &json_dumps_unicode(&event.title))
            .replace(
                "{course}",
                &course.as_deref().map(json_dumps_unicode).unwrap_or_else(|| "null".to_string()),
            )
            .replace("{due}", &new_due)
            .replace("{uid}", &json_dumps_unicode(&event.uid))
            .replace("{body}", &event.description);

        let rel_path = crate::ids::rel(vault, &path);
        match crate::write::create(vault, &rel_path, &body, ctx, journal, None) {
            Ok(created) => {
                let stem = created.file_stem().map(|s| s.to_string_lossy().to_string()).unwrap_or_default();
                log.push(format!("created {stem}"));
                known.insert(event.uid.clone(), created);
                let _ = record_seen(vault, &event.uid, &event.title, &stamp);
            }
            Err(err) => log.push(format!("skipped (unwritable): {err}")),
        }
    }
    if left_out > 0 {
        log.push(format!("{left_out} item(s) from courses you removed were left out"));
    }
    log
}

/// Ruling 3 and email spec D11 (5) (plan T8b, P12, §10 R3 and R14): the feed says `to` for a
/// `field` (`title` or `due`) of the note at `target_rel` that the student edited by hand. Files one
/// `kind: amend` card from the note's value to the feed's, in the shape
/// `approvals::validate_amendment` accepts, through `write::create` as the ingest actor; the note
/// keeps its value. `created_by` is the feed's own name, the one creator whose card may change
/// `title` (PQ5 (c)); the card carries no `source_uid` (an archived card holding the event's uid
/// would shadow the note in [`existing_by_uid`]) and no judgment field. Pending from `today`, it is
/// charged to the 15-a-day budget like any proposal. `Ok(None)` files nothing:
/// - a null or a collection on the note's side, or a value no card holds on one line: such a card
///   could never be applied, so the note keeps its value (R14);
/// - a pending or snoozed `kind: amend` card for the note and exactly `{field}`, whoever filed it,
///   is already the open decision ([`open_amendment`], R14);
/// - the student rejected a card for the note and field proposing this same value (P12).
#[allow(clippy::too_many_arguments)]
fn propose_feed_value(
    vault: &Path,
    target_rel: &str,
    stem: &str,
    meta: &serde_yaml_ng::Mapping,
    field: &str,
    to: &str,
    ctx: &crate::write::WriteContext,
    journal: &mut crate::journal::Journal,
    today: Date,
) -> Result<Option<PathBuf>, crate::write::WriteError> {
    use crate::yamlemit::Node;
    use serde_yaml_ng::Value;
    let Some(from) = crate::yaml::get(meta, field).filter(|v| matches!(v, Value::String(_) | Value::Number(_) | Value::Bool(_))) else {
        return Ok(None);
    };
    let Some(from_text) = crate::yaml::text(from) else { return Ok(None) };
    if [from_text.as_str(), to].into_iter().any(|text| crate::write::single_line_problem(text).is_some()) {
        return Ok(None);
    }
    if open_amendment(vault, target_rel, field).is_some() || rejected_before(vault, target_rel, field, to) {
        return Ok(None);
    }
    let title = crate::yaml::get(meta, "title").and_then(crate::yaml::text).unwrap_or_else(|| stem.to_string());
    let change = Node::map(vec![("from", Node::from_json(&crate::yaml::to_json(from))), ("to", Node::text(to))]);
    let front = Node::map(vec![
        ("type", Node::text("approval")),
        ("kind", Node::text("amend")),
        ("title", Node::text(&format!("Your course feed changed {field} for {title}"))),
        ("status", Node::text("pending")),
        ("target", Node::text(target_rel)),
        ("proposed_at", Node::Date(today)),
        ("first_proposed_at", Node::Date(today)),
        ("expires", Node::Null),
        ("snooze_until", Node::Null),
        ("created_by", Node::text(crate::approvals::TITLE_CARD_CREATOR)),
        ("changes", Node::Map(vec![(Node::text(field), change)])),
    ]);
    let why = format!("Your course feed now says {to}; you set {from_text} by hand, so this is a proposal");
    let text = format!(
        "---\n{}---\n\n**Why proposed:** {why}\n{}",
        crate::yamlemit::safe_dump_block(&front),
        crate::write::AMEND_BUTTONS
    );
    let approvals = vault.join("approvals");
    std::fs::create_dir_all(&approvals).map_err(|e| crate::write::WriteError::Io(e.to_string()))?;
    let mut path = approvals.join(format!("amend-{stem}-{field}.md"));
    let mut suffix = 2;
    while path.exists() {
        path = approvals.join(format!("amend-{stem}-{field}-{suffix}.md"));
        suffix += 1;
    }
    crate::write::create(vault, &crate::ids::rel(vault, &path), &text, ctx, journal, None).map(Some)
}

/// Plan §10 R14's one open decision per field: a `kind: amend` card in `approvals/` for the note at
/// `target_rel` whose `changes` name exactly `{field}`, whoever filed it, either `pending`
/// (`write::find_pending_amendment`) or `snoozed`. A snoozed card is still undecided: it wakes
/// `pending` with its own `to`. `defer_over_budget` snoozes a day's overflow (R3), the student may
/// snooze a card by hand, and the mail pass files its first-day cards snoozed (R16); were any of
/// them invisible here, each slot would file another card for the same field. `write.rs` is not
/// edited (Checkpoint A); the mail card writer (plan T6.2) takes the same rule.
pub(crate) fn open_amendment(vault: &Path, target_rel: &str, field: &str) -> Option<PathBuf> {
    let fields = std::collections::BTreeSet::from([field.to_string()]);
    if let Some(pending) = crate::write::find_pending_amendment(vault, target_rel, &fields) {
        return Some(pending);
    }
    let mut paths: Vec<PathBuf> = std::fs::read_dir(vault.join("approvals"))
        .ok()?
        .flatten()
        .map(|entry| entry.path())
        .filter(|path| path.extension().is_some_and(|x| x == "md"))
        .collect();
    paths.sort();
    paths.into_iter().find(|path| {
        let Some(meta) = crate::ids::read_meta(path) else { return false };
        let word = |key: &str| crate::yaml::get(&meta, key).and_then(crate::yaml::text);
        let card = [("type", "approval"), ("kind", "amend"), ("status", "snoozed"), ("target", target_rel)];
        let named = match crate::yaml::get(&meta, "changes") {
            Some(serde_yaml_ng::Value::Mapping(changes)) => {
                changes.keys().filter_map(crate::yaml::text).collect::<std::collections::BTreeSet<String>>() == fields
            }
            _ => false,
        };
        card.iter().all(|(key, want)| word(key).as_deref() == Some(*want)) && named
    })
}

/// P12's "already rejected": a card in `archive/` with `kind: amend` and `status: rejected` for
/// `target_rel` whose `changes.<field>.to` is the feed's value `to`, whoever filed it. A `due` is
/// compared as a time ([`due_changed`]), so a bare date and its 23:59 are one value.
fn rejected_before(vault: &Path, target_rel: &str, field: &str, to: &str) -> bool {
    let Ok(entries) = std::fs::read_dir(vault.join("archive")) else { return false };
    entries.flatten().map(|entry| entry.path()).filter(|path| path.extension().is_some_and(|x| x == "md")).any(|path| {
        let Some(meta) = pystr::read_text(&path).ok().and_then(|text| split_frontmatter(&text).ok()).map(|(meta, _)| meta) else {
            return false;
        };
        let word = |key: &str| crate::yaml::get(&meta, key).and_then(crate::yaml::text);
        let card = [("type", "approval"), ("kind", "amend"), ("status", "rejected"), ("target", target_rel)];
        let rejected = crate::yaml::get(&meta, "changes").and_then(|changes| changes.get(field)).and_then(|spec| spec.get("to"));
        card.iter().all(|(key, want)| word(key).as_deref() == Some(*want))
            && rejected.is_some_and(|rejected| match field {
                "due" => !due_changed(Some(rejected), to),
                _ => crate::yaml::text(rejected).as_deref() == Some(to),
            })
    })
}

/// Ports `engine/ingest.py:main` (lines 388–431). Exit codes and printed lines are Python's: `1`
/// for an unreadable or missing config, `1` for an empty `ics_url`, `1` for a failed fetch, `0`
/// otherwise. **Python writes no run record and no `runner-log.md` line, and has no `--dry-run`** —
/// neither does this.
///
/// Two documented deviations, both from Python raising where Rust returns: an unknown `timezone`
/// is a `ZoneInfoNotFoundError` traceback in Python and a `1` with a message here, and an empty
/// config file is an `AttributeError` traceback there and `no ics_url configured` here. Both are
/// non-zero on both sides. The text after `config unreadable:` is each language's own parser
/// message — the same class of difference `scripts/diff-engines.ps1` masks for the transport
/// library, and this command is not under the oracle.
pub fn run(vault: &Path, via: &str, run_id: Option<&str>) -> i32 { run_with(vault, via, run_id, None) }

/// [`run`] with the network seam exposed, exactly as `cli::run_with` exposes rank's two. Production
/// passes `None`; tests pass a closure over `tests/fixtures/blackboard.ics`.
pub fn run_with(
    vault: &Path,
    via: &str,
    run_id: Option<&str>,
    fetch: Option<&dyn Fn(&str) -> Result<String, String>>,
) -> i32 {
    let (code, lines) = run_lines(vault, via, run_id, fetch);
    for line in &lines { println!("{line}"); }
    code
}

/// R2-4: the exact `error` text `/ingest-ics` answers when this account has no `lms_ics` source
/// configured (`cloud/supabase/functions/ingest-ics/handler.ts`'s `Response.json({ error: "no
/// lms_ics source for this account" }, { status: 404 })`) — must equal that handler's body byte for
/// byte, the same shape `enrich.rs`'s `pull_rules` settle arm uses for its own 404 (R-C2-E49).
/// `ingest-ics/handler_test.ts` pins this constant from the Deno side: it reads this file, extracts
/// the literal, and asserts it against the handler's own response.
const NO_LMS_ICS_SOURCE_DETAIL: &str = "no lms_ics source for this account";

/// A-1 (H5 fix, CLAUDE.md's "Neither feed is compulsory"): what `ingest` does when `/ingest-ics`
/// failed and there is no local `ics_url` to fall back to — a genuinely dead end either way, but
/// TWO different dead ends. A 404 means this account simply has no `lms_ics` source: a permanent,
/// normal state for a student who left the LMS capture empty, never a failure — exit 0 and name
/// the skip, the same shape as `ingest (skipped: no ics_url)` for a vault with no account at all,
/// never exit 1 every slot forever (`scheduler.rs`'s own invariant: a non-zero step is retry
/// backoff and an amber tray, and nothing here can ever change without the student taking an
/// action the app already offers). Any OTHER failure still means there is no way at all to get a
/// feed — exit 1 stands — but the line says what actually happened: a genuine HTTP status from the
/// service reads differently than the service being unreachable altogether.
///
/// Pulled out as a pure function over `CloudError` (not tested through `run_lines` itself): a
/// cloud vault reaching this arm needs `cloudmodel::resolve` to succeed, which needs a real
/// Windows Credential Manager session — no test in this crate or `cloud_contract.rs` has one, by
/// design (`cloud_contract.rs`'s own doc comment says why), so this is the one place the decision
/// can be exercised directly.
fn cloud_ics_failure_with_no_local_url(e: &crate::cloudmodel::CloudError) -> (i32, Vec<String>) {
    match e {
        crate::cloudmodel::CloudError::Status { code: 404, ref detail } if detail == NO_LMS_ICS_SOURCE_DETAIL => {
            (0, vec!["ingest: no LMS feed on this account — skipped".to_string()])
        }
        crate::cloudmodel::CloudError::Status { code, .. } => (
            1,
            vec![format!(
                "ingest: no feed — the service answered HTTP {code} ({e}) and no ics_url is configured"
            )],
        ),
        _ => (
            1,
            vec![format!("ingest: no feed — the service is unavailable ({e}) and no ics_url is configured")],
        ),
    }
}

/// The testable core: everything `run_with` does, with the output returned instead of printed, in
/// the same order Python prints it. Python's tests assert on `capsys`; Rust has no such capture, so
/// the lines are a value.
pub fn run_lines(
    vault: &Path,
    via: &str,
    run_id: Option<&str>,
    fetch: Option<&dyn Fn(&str) -> Result<String, String>>,
) -> (i32, Vec<String>) {
    run_lines_on(vault, via, run_id, fetch, None)
}

/// [`run_lines`] with `sync_tasks`'s `today` seam exposed. Production passes `None` (the local
/// date). A test over `tests/fixtures/blackboard.ics` must pin it: on a first run R-OB-3 archives
/// everything due before today, so an unpinned run stops creating tasks once the wall clock passes
/// the fixture's dates.
fn run_lines_on(
    vault: &Path,
    via: &str,
    run_id: Option<&str>,
    fetch: Option<&dyn Fn(&str) -> Result<String, String>>,
    today: Option<Date>,
) -> (i32, Vec<String>) {
    let ctx = crate::write::WriteContext {
        actor: "agent:ingest.blackboard".to_string(),
        via: via.to_string(),
        run_id: run_id.map(str::to_string),
    };
    let path = vault.join("config").join("ingest.yaml");
    // NOT `yaml::mapping_from_file`: it folds a missing file and a broken one into an empty
    // mapping, and Python distinguishes both from "no ics_url" by message. Read and parse here so
    // every Python branch survives.
    let text = match crate::pystr::read_text(&path) {
        Ok(t) => t,
        Err(e) => return (1, vec![format!("ingest: config unreadable: {e}")]),
    };
    let config: serde_yaml_ng::Value = match serde_yaml_ng::from_str(&text) {
        Ok(v) => v,
        Err(e) => return (1, vec![format!("ingest: config unreadable: {e}")]),
    };
    let url = config.get("ics_url").and_then(|v| v.as_str()).unwrap_or("").trim().to_string();
    let tz_name = config.get("timezone").and_then(|v| v.as_str()).unwrap_or("America/Chicago").to_string();
    let tz = match TimeZone::get(&tz_name) {
        Ok(tz) => tz,
        Err(e) => return (1, vec![format!("ingest: config unreadable: unknown timezone {tz_name}: {e}")]),
    };
    // C2 (cloud design §3.1): the LMS capability URL lives in the account, encrypted, not in the
    // vault — so when this vault has an account the feed is fetched by `/ingest-ics` with the
    // service role and the URL never leaves the server. `config/ingest.yaml`'s `ics_url` stays the
    // fallback for a vault with no account and for a service that is unreachable: dead hotel
    // Wi-Fi must not stop an ingest that could have run from the URL already on disk.
    //
    // **The empty-URL refusal moved here on purpose.** It used to sit six lines above, before any
    // cloud attempt; a cloud vault with a blank `ics_url` would then exit 1 without ever asking
    // the service, and a non-zero step is retry backoff and an amber tray twice a day forever.
    // Exit 1 now means what it says: there was no way at all to get a feed.
    let cloud = crate::cloudmodel::resolve(vault).ok();
    // R-OB-3 (hand-off H11), by R-C2-9's predicate: this vault has never been through a whole slot
    // if `rank` has never written `today.md`. NOT the seen-ledger — `coursework` runs before
    // `ingest` in the same slot and calls `record_seen`, so that file exists on the very first run.
    let first_run = crate::ingest::is_first_run(vault);
    let fetched = match (fetch, &cloud) {
        (Some(f), _) => {
            if url.is_empty() { return (1, vec!["ingest: no ics_url configured".to_string()]); }
            f(&url)
        }
        // `_past` is bound and unused on purpose: the device archives on its own comparison, which
        // is the guarantee (it knows the vault's timezone), and the service's list is corroboration
        // and what the wizard counts. Binding it here is what makes the next reader ask which half
        // is authoritative; the answer is in `fetch_ics`'s doc comment.
        (None, Some(client)) => match crate::cloudmodel::fetch_ics(client, first_run) {
            Ok((text, _past)) => Ok(text),
            Err(e) if url.is_empty() => return cloud_ics_failure_with_no_local_url(&e),
            Err(e) => {
                println!("ingest: /ingest-ics unavailable ({e}); using the vault's ics_url");
                crate::calfeed::fetch_ics(&url)
            }
        },
        (None, None) => {
            if url.is_empty() { return (1, vec!["ingest: no ics_url configured".to_string()]); }
            crate::calfeed::fetch_ics(&url)
        }
    };
    let feed = match fetched {
        Ok(t) => t,
        Err(e) => return (1, vec![format!("ingest: fetch failed: {e}")]),
    };
    let (events, malformed) = parse_ics(&feed, &tz);
    // `course_map` is a mapping of fragment -> slug, and `match_course` walks it IN ORDER —
    // `serde_yaml_ng::Mapping` preserves the file's order, as Python's dict preserves insertion.
    let course_map: Vec<(String, String)> = config
        .get("course_map")
        .and_then(|v| v.as_mapping())
        .map(|m| m.iter().filter_map(|(k, v)| Some((k.as_str()?.to_string(), v.as_str()?.to_string()))).collect())
        .unwrap_or_default();
    // F6c (§14.45): `course_exclude` is a flow sequence of bare fragments; the slug is unused, so the
    // pairs reuse `match_course` and its one rule. No other reader of `ingest.yaml` looks at it.
    let course_exclude: Vec<(String, String)> = config
        .get("course_exclude")
        .and_then(|v| v.as_sequence())
        // An empty fragment is a substring of every uid and would leave out the whole feed.
        .map(|s| s.iter().filter_map(|f| Some(f.as_str()?.to_string())).filter(|f| !f.trim().is_empty()).map(|f| (f, String::new())).collect())
        .unwrap_or_default();
    let mut journal = crate::journal::Journal::new(vault);
    // O1: the gradebook half. `grades/` is read lockless: a concurrent grades write can make a note
    // be missed this run, which only defers the mark to the next run (see `grade_index`).
    let grades = grade_index(vault);
    let mut log =
        sync_tasks_excluding(&events, vault, &course_map, &course_exclude, Some(&ctx), &mut journal, today, first_run, &grades);
    // A left-out item gets no write of any kind, the gradebook's "done" mark included.
    let kept: Vec<Event> =
        events.iter().filter(|e| !is_left_out(e, &course_map, &course_exclude)).cloned().collect();
    log.extend(mark_gradebook_done(&kept, vault, &grades, &ctx, &mut journal));
    if first_run {
        log.push(format!("ingest: first run — {} item(s) already past were archived",
            log.iter().filter(|l| l.starts_with("archived (imported-past)")).count()));
        let turned_in = log.iter().filter(|l| l.starts_with("archived (imported-done)")).count();
        if turned_in > 0 {
            log.push(format!("ingest: first run — {turned_in} item(s) already turned in were archived"));
        }
    }
    if malformed > 0 { log.push(format!("skipped {malformed} malformed event(s)")); }
    let summary = format!("ingest: {} events, {} action(s)", events.len(), log.len());
    log.push(summary);
    (0, log)
}

#[cfg(test)]
mod tests {
    use super::*;

    // -- A-1: the cloud ingest arm's 404-vs-everything-else decision, tested directly over
    // `CloudError` since reaching it through `run_lines` needs a real Windows session
    // (`cloud_contract.rs`'s doc comment explains why no test here has one).

    #[test]
    fn a_404_with_no_local_url_is_a_named_skip_at_exit_0() {
        let e = crate::cloudmodel::CloudError::Status { code: 404, detail: "no lms_ics source for this account".to_string() };
        let (code, lines) = cloud_ics_failure_with_no_local_url(&e);
        assert_eq!(code, 0);
        assert_eq!(lines, vec!["ingest: no LMS feed on this account — skipped".to_string()]);
    }

    // R2-4: a 404 whose body is NOT the handler's own `no lms_ics source for this account` text —
    // a gateway 404 (the function not deployed, a stale `api_base`, a slug typo) — must fall into
    // the "the service answered HTTP {code}" exit-1 arm, never the named skip. Pre-fix, the arm
    // matched on `code == 404` alone and this case read as a permanent, silent "no feed configured"
    // forever.
    #[test]
    fn a_404_without_the_handlers_detail_keeps_exit_1_and_names_http_404() {
        let e = crate::cloudmodel::CloudError::Status { code: 404, detail: String::new() };
        let (code, lines) = cloud_ics_failure_with_no_local_url(&e);
        assert_eq!(code, 1);
        assert_eq!(lines.len(), 1);
        assert!(lines[0].contains("the service answered HTTP 404"), "{lines:?}");
        assert!(lines[0].contains("no ics_url is configured"), "{lines:?}");
        assert!(!lines[0].contains("skipped"), "a mismatched 404 must never read as the named skip: {lines:?}");
    }

    #[test]
    fn a_503_with_no_local_url_keeps_exit_1_but_names_the_real_status() {
        let e = crate::cloudmodel::CloudError::Status { code: 503, detail: String::new() };
        let (code, lines) = cloud_ics_failure_with_no_local_url(&e);
        assert_eq!(code, 1);
        assert_eq!(lines.len(), 1);
        assert!(lines[0].contains("the service answered HTTP 503"), "{lines:?}");
        assert!(lines[0].contains("no ics_url is configured"), "{lines:?}");
        assert!(!lines[0].contains("unavailable"), "a real status must never read as merely down: {lines:?}");
    }

    #[test]
    fn a_transport_failure_with_no_local_url_reads_as_unavailable_not_a_status() {
        let e = crate::cloudmodel::CloudError::Transport("connection refused".to_string());
        let (code, lines) = cloud_ics_failure_with_no_local_url(&e);
        assert_eq!(code, 1);
        assert!(lines[0].contains("the service is unavailable"), "{lines:?}");
        assert!(!lines[0].contains("HTTP"), "a transport failure carries no status to name: {lines:?}");
    }

    fn changes(pairs: &[(&str, &str)]) -> Vec<(String, String)> {
        pairs.iter().map(|(k, v)| (k.to_string(), v.to_string())).collect()
    }

    const NOTE: &str = "---\ntitle: \"CS 100 HW 01\"\ncourse: cs-100\ndue: 2026-08-26T23:59\neffort_hours: 2.5\nimportance_reason: \"One of 14 homeworks sharing 10% of CS 100.\"\nstatus: active\n---\n\nBody text.\n";

    // -- the byte-preservation test, written first --------------------------

    #[test]
    fn changing_one_field_leaves_every_other_byte_untouched() {
        let out = apply_frontmatter_fields_to_text(NOTE, &changes(&[("due", "2026-09-01T23:59")]), "n").unwrap();
        let before: Vec<&str> = NOTE.split('\n').collect();
        let after: Vec<&str> = out.split('\n').collect();
        assert_eq!(before.len(), after.len(), "line count must not change");
        for (i, (b, a)) in before.iter().zip(after.iter()).enumerate() {
            if i == 3 {
                assert_eq!(*a, "due: 2026-09-01T23:59");
            } else {
                assert_eq!(b, a, "line {i} must be byte-identical");
            }
        }
    }

    #[test]
    fn key_order_comments_and_the_trailing_newline_survive() {
        let out = apply_frontmatter_fields_to_text(NOTE, &changes(&[("status", "archived")]), "n").unwrap();
        assert!(out.contains("importance_reason: \"One of 14 homeworks sharing 10% of CS 100.\""));
        assert!(out.ends_with("Body text.\n"), "trailing newline lost");
        assert!(out.starts_with("---\ntitle: \"CS 100 HW 01\""), "key order changed");
    }

    #[test]
    fn a_missing_key_is_inserted_before_the_closing_marker() {
        let out = apply_frontmatter_fields_to_text(NOTE, &changes(&[("id", "task_abc1234567")]), "n").unwrap();
        let lines: Vec<&str> = out.split('\n').collect();
        let idx = lines.iter().position(|l| l.starts_with("id:")).unwrap();
        let close = lines.iter().skip(1).position(|l| *l == "---").unwrap() + 1;
        assert_eq!(idx, close - 1, "must sit immediately before the closing ---");
    }

    #[test]
    fn a_value_containing_a_colon_and_quotes_is_written_raw() {
        let out = apply_frontmatter_fields_to_text(
            NOTE,
            &changes(&[("source_uid", "\"zybooks:1839992\"")]),
            "n",
        )
        .unwrap();
        assert!(out.contains("source_uid: \"zybooks:1839992\""));
    }

    #[test]
    fn the_key_prefix_match_requires_the_colon() {
        // `due:` must not match `due_alt:`.
        let note = "---\ndue_alt: 2026-01-01\ndue: 2026-08-26T23:59\n---\n\nb\n";
        let out = apply_frontmatter_fields_to_text(&note, &changes(&[("due", "2026-09-09T09:00")]), "n").unwrap();
        assert!(out.contains("due_alt: 2026-01-01"), "due_alt was clobbered");
        assert!(out.contains("due: 2026-09-09T09:00"));
    }

    #[test]
    fn only_the_first_duplicate_key_is_replaced() {
        let note = "---\nstatus: active\nstatus: active\n---\n\nb\n";
        let out = apply_frontmatter_fields_to_text(&note, &changes(&[("status", "done")]), "n").unwrap();
        assert_eq!(out, "---\nstatus: done\nstatus: active\n---\n\nb\n");
    }

    #[test]
    fn a_body_containing_a_horizontal_rule_is_not_treated_as_the_closing_marker() {
        let note = "---\nstatus: active\n---\n\nintro\n\n---\n\nafter\n";
        let out = apply_frontmatter_fields_to_text(&note, &changes(&[("id", "task_0000000000")]), "n").unwrap();
        assert!(out.ends_with("intro\n\n---\n\nafter\n"));
        assert_eq!(out.matches("task_0000000000").count(), 1);
    }

    #[test]
    fn text_without_frontmatter_is_refused() {
        assert!(apply_frontmatter_fields_to_text("no frontmatter\n", &changes(&[("a", "b")]), "n").is_err());
    }

    #[test]
    fn a_crlf_note_read_through_pystr_is_editable() {
        // The trap: read raw, the closing line is "---\r" and the exact match never fires.
        let dir = std::env::temp_dir().join(format!("qo-ingest-crlf-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        let path = dir.join("note.md");
        std::fs::write(&path, NOTE.replace('\n', "\r\n")).unwrap();

        update_frontmatter_fields(&path, &changes(&[("status", "archived")])).unwrap();
        let back = pystr::read_text(&path).unwrap();
        assert!(back.contains("status: archived"));
        let raw = std::fs::read(&path).unwrap();
        if cfg!(windows) {
            assert!(raw.windows(2).any(|w| w == b"\r\n"), "CRLF must be preserved on write");
        }
    }

    // -- ICS ----------------------------------------------------------------

    #[test]
    fn unfold_joins_continuation_lines() {
        // RFC 5545 folding inserts CRLF + one space MID-CONTENT, and unfolding drops exactly that
        // space — it does NOT reinsert a word break. Verified against the interpreter:
        // Python's unfold("SUMMARY:Long\r\n title\nUID:1") is also ["SUMMARY:Longtitle", "UID:1"].
        // A content space has to sit at the end of the previous physical line to survive.
        assert_eq!(unfold("SUMMARY:Long\r\n title\nUID:1"), vec!["SUMMARY:Longtitle", "UID:1"]);
        assert_eq!(unfold("SUMMARY:Long \r\n title"), vec!["SUMMARY:Long title"]);
        assert_eq!(unfold("A:1\n\tcont"), vec!["A:1cont"]);
    }

    #[test]
    fn unescape_handles_the_backslash_sentinel() {
        assert_eq!(unescape(r"a\,b\;c"), "a,b;c");
        assert_eq!(unescape(r"line\nbreak"), "line\nbreak");
        // An escaped backslash followed by n must NOT become a newline.
        assert_eq!(unescape(r"a\\nb"), r"a\nb");
    }

    #[test]
    fn parse_property_splits_name_params_and_value() {
        let (name, params, value) = parse_property("DTSTART;TZID=America/Chicago:20260828T090000").unwrap();
        assert_eq!(name, "DTSTART");
        assert_eq!(params.get("TZID").unwrap(), "America/Chicago");
        assert_eq!(value, "20260828T090000");
        assert!(parse_property("no colon here").is_none());
    }

    #[test]
    fn parse_dt_reads_dates_and_converts_utc_to_local() {
        let tz = TimeZone::get("America/Chicago").unwrap();
        let empty = BTreeMap::new();
        assert_eq!(parse_dt("20260828", &empty, &tz), Some(Due::Date(Date::constant(2026, 8, 28))));
        // 14:00Z is 09:00 CDT.
        assert_eq!(
            parse_dt("20260828T140000Z", &empty, &tz),
            Some(Due::DateTime(DateTime::constant(2026, 8, 28, 9, 0, 0, 0)))
        );
        // No zone marker: already local, taken as-is.
        assert_eq!(
            parse_dt("20260828T090000", &empty, &tz),
            Some(Due::DateTime(DateTime::constant(2026, 8, 28, 9, 0, 0, 0)))
        );
    }

    #[test]
    fn slugify_lowercases_collapses_and_caps_at_sixty_chars() {
        assert_eq!(slugify("CS 100 HW 01: Intro to C!"), "cs-100-hw-01-intro-to-c");
        assert_eq!(slugify("!!!"), "item");
        assert_eq!(slugify(&"a".repeat(80)).chars().count(), 60);
    }

    #[test]
    fn the_course_fragment_matcher_respects_alphanumeric_boundaries() {
        // The one lookaround in the codebase, hand-coded.
        assert!(contains_bounded("CS 100 HW 01", "CS 100"));
        assert!(contains_bounded("summary: cs-100 lab", "CS-100"));
        assert!(!contains_bounded("STATISTICS-100-001", "CS-100"));
        assert!(!contains_bounded("XCS-100Y", "CS-100"));
    }

    #[test]
    fn match_course_never_scans_the_description() {
        let map = vec![("CS 100".to_string(), "cs-100".to_string())];
        let event = Event {
            uid: "u1".into(),
            title: "Registrar deadline".into(),
            due: None,
            description: "mentions CS 100 in passing".into(),
            raw: "UID:u1\nSUMMARY:Registrar deadline\nDESCRIPTION:mentions CS 100 in passing".into(),
        };
        assert_eq!(match_course(&event, &map), None);
    }

    #[test]
    fn a_uid_pin_beats_everything_and_is_case_sensitive() {
        let map = vec![("_4686399_1".to_string(), "engr-101".to_string())];
        let event = Event {
            uid: "_blackboard.platform.gradebook2.GradableItem-_4686399_1".into(),
            title: "Quiz".into(),
            due: None,
            description: String::new(),
            raw: "UID:x\nSUMMARY:Quiz".into(),
        };
        assert_eq!(match_course(&event, &map), Some("engr-101".to_string()));
    }

    #[test]
    fn institutional_noise_is_recognised_by_uid_namespace() {
        assert!(is_institutional_noise("_blackboard.data.calendar.CalendarEntry-_123_1"));
        assert!(!is_institutional_noise("_blackboard.platform.gradebook2.GradableItem-_1_1"));
    }

    /// Reference-output test: every value below was produced by running the **Python** parser over
    /// this fixture, not derived from the Rust. The fixture deliberately contains two malformed
    /// events, so `malformed == 2` is correct behaviour rather than a defect.
    #[test]
    fn parse_ics_reproduces_pythons_output_for_the_blackboard_fixture() {
        let tz = TimeZone::get("America/Chicago").unwrap();
        let text = pystr::read_text(Path::new("tests/fixtures/blackboard.ics")).unwrap();
        let (events, malformed) = parse_ics(&text, &tz);

        let got: Vec<String> = events
            .iter()
            .map(|e| format!("{}|{}|{}", e.uid, e.title, format_due(e.due.unwrap())))
            .collect();

        assert_eq!(
            got,
            vec![
                "BB-ASSIGN-1111@ua.blackboard.com|Homework 2|2026-09-11T23:59",
                "BB-ASSIGN-2222@ua.blackboard.com|Kapitel 2 Vokabeln Quiz|2026-09-15",
                "BB-ASSIGN-3333@ua.blackboard.com|Long folded summary line that continues across two physical lines|2026-09-20T12:00",
                "_blackboard.data.calendar.CalendarEntry-_999001_1|Example Institutional Holiday|2026-11-28T23:59",
                "_blackboard.platform.gradebook2.GradableItem-_999002_1|Example Gradebook Deliverable|2026-10-15T17:00",
            ]
        );
        assert_eq!(malformed, 2, "the fixture carries two deliberately broken events");
    }

    #[test]
    fn the_first_occurrence_of_a_property_wins() {
        let tz = TimeZone::get("America/Chicago").unwrap();
        let ics = "BEGIN:VEVENT\nUID:u1\nSUMMARY:First\nSUMMARY:Second\nDTSTART:20260828\nEND:VEVENT\n";
        let (events, _) = parse_ics(ics, &tz);
        assert_eq!(events[0].title, "First");
    }

    #[test]
    fn a_trailing_bracket_is_stripped_from_the_title() {
        let tz = TimeZone::get("America/Chicago").unwrap();
        let ics = "BEGIN:VEVENT\nUID:u1\nSUMMARY:HW 01 [CS 100 Section 4]\nDTSTART:20260828\nEND:VEVENT\n";
        let (events, _) = parse_ics(ics, &tz);
        assert_eq!(events[0].title, "HW 01");
    }

    #[test]
    fn an_unparseable_date_makes_the_event_malformed() {
        let tz = TimeZone::get("America/Chicago").unwrap();
        let ics = "BEGIN:VEVENT\nUID:u1\nSUMMARY:Broken\nDTSTART:not-a-date\nEND:VEVENT\n";
        let (events, malformed) = parse_ics(ics, &tz);
        assert!(events.is_empty());
        assert_eq!(malformed, 1);
    }

    #[test]
    fn an_event_with_no_uid_or_no_title_is_malformed() {
        let tz = TimeZone::get("America/Chicago").unwrap();
        let (e1, m1) = parse_ics("BEGIN:VEVENT\nSUMMARY:No uid\nEND:VEVENT\n", &tz);
        assert!(e1.is_empty() && m1 == 1);
        let (e2, m2) = parse_ics("BEGIN:VEVENT\nUID:u1\nSUMMARY:\nEND:VEVENT\n", &tz);
        assert!(e2.is_empty() && m2 == 1);
    }

    #[test]
    fn format_due_distinguishes_a_date_from_a_datetime() {
        assert_eq!(format_due(Due::Date(Date::constant(2026, 8, 28))), "2026-08-28");
        assert_eq!(
            format_due(Due::DateTime(DateTime::constant(2026, 8, 28, 23, 59, 0, 0))),
            "2026-08-28T23:59"
        );
    }

    // -- uid dedup ----------------------------------------------------------

    fn uid_vault() -> PathBuf {
        let dir = std::env::temp_dir().join(format!(
            "qo-ingest-uid-{}-{:?}",
            std::process::id(),
            std::thread::current().id()
        ));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(dir.join("tasks")).unwrap();
        std::fs::create_dir_all(dir.join("archive")).unwrap();
        dir
    }

    #[test]
    fn existing_by_uid_indexes_tasks_and_archive() {
        let v = uid_vault();
        pystr::write_text(&v.join("tasks").join("a.md"), "---\nsource_uid: \"zybooks:1\"\n---\n\nb\n").unwrap();
        pystr::write_text(&v.join("archive").join("b.md"), "---\nsource_uid: \"gmail:2\"\n---\n\nb\n").unwrap();
        let found = existing_by_uid(&v);
        assert_eq!(found.len(), 2);
        assert!(found.contains_key("zybooks:1"));
        assert!(found.contains_key("gmail:2"));
    }

    #[test]
    fn an_unreadable_note_still_registers_its_uid_via_the_regex_fallback() {
        // THE trap: propagating the parse error drops the note out of dedup and the next sync
        // creates a duplicate task file for it.
        let v = uid_vault();
        let broken = "---\ntitle: \"x\nsource_uid: \"zybooks:1839992\"\nstatus: [unclosed\n---\n\nbody\n";
        pystr::write_text(&v.join("tasks").join("broken.md"), broken).unwrap();
        assert!(split_frontmatter(broken).is_err(), "fixture must actually be unparseable");

        let found = existing_by_uid(&v);
        assert!(
            found.contains_key("zybooks:1839992"),
            "the regex fallback must still register the uid: {found:?}"
        );
    }

    #[test]
    fn the_seen_ledger_round_trips() {
        let v = uid_vault();
        assert!(load_seen(&v).is_empty());
        record_seen(&v, "zybooks:1", "HW 01", "2026-08-28").unwrap();
        record_seen(&v, "gmail:2", "Email · with a dot", "2026-08-29").unwrap();
        let seen = load_seen(&v);
        assert!(seen.contains("zybooks:1"));
        assert!(seen.contains("gmail:2"), "a title containing the separator must not break the uid");
        assert_eq!(seen.len(), 2);
    }

    /// R-C2-E20 fix 1: the predicate `/ingest-ics`'s `first_run` flag is built from, pinned
    /// directly rather than only through `run_lines`'s cloud arm (which no test machine here can
    /// reach end to end — see `cloud_contract.rs`'s doc comment on the blank-`ics_url` test).
    #[test]
    fn is_first_run_is_true_with_no_today_md_and_false_once_rank_has_written_one() {
        let v = uid_vault();
        assert!(is_first_run(&v), "a vault that has never been ranked has no state/today.md yet");
        std::fs::create_dir_all(v.join("state")).unwrap();
        std::fs::write(v.join("state").join("today.md"), "# Today\n").unwrap();
        assert!(!is_first_run(&v), "today.md now exists — this vault has finished at least one slot");
    }

    #[test]
    fn due_changed_detects_a_real_move_and_ignores_an_equal_one() {
        use serde_yaml_ng::Value;
        let old = Value::String("2026-08-26T23:59".into());
        assert!(!due_changed(Some(&old), "2026-08-26T23:59"));
        assert!(due_changed(Some(&old), "2026-09-01T23:59"));
        // A bare date coerces to 23:59, so these are equal.
        let bare = Value::String("2026-08-26".into());
        assert!(!due_changed(Some(&bare), "2026-08-26T23:59"));
        // Unparseable old value counts as changed.
        let junk = Value::String("nonsense".into());
        assert!(due_changed(Some(&junk), "2026-08-26T23:59"));
    }

    // -- sync_tasks ---------------------------------------------------------

    fn sync_vault() -> PathBuf {
        let dir = std::env::temp_dir().join(format!(
            "qo-sync-{}-{:?}",
            std::process::id(),
            std::thread::current().id()
        ));
        let _ = std::fs::remove_dir_all(&dir);
        for folder in crate::ids::NOTE_FOLDERS {
            std::fs::create_dir_all(dir.join(folder)).unwrap();
        }
        std::fs::create_dir_all(dir.join("state").join("journal")).unwrap();
        dir
    }

    fn ev(uid: &str, title: &str, due: Option<Due>) -> Event {
        Event {
            uid: uid.into(),
            title: title.into(),
            due,
            description: "from blackboard".into(),
            raw: format!("UID:{uid}\nSUMMARY:{title}"),
        }
    }

    fn dt(y: i16, m: i8, d: i8) -> Option<Due> {
        Some(Due::DateTime(DateTime::constant(y, m, d, 23, 59, 0, 0)))
    }

    fn run(vault: &Path, events: &[Event], map: &[(String, String)]) -> Vec<String> {
        let mut j = crate::journal::Journal::new(vault);
        sync_tasks(events, vault, map, None, &mut j, Some(Date::constant(2026, 8, 28)), false)
    }

    #[test]
    fn sync_creates_a_note_and_records_the_uid() {
        let v = sync_vault();
        let map = vec![("CS 100".to_string(), "cs-100".to_string())];
        let log = run(&v, &[ev("bb:1", "CS 100 HW 01", dt(2026, 9, 11))], &map);

        assert_eq!(log, vec!["created cs-100-cs-100-hw-01"]);
        let created = v.join("tasks").join("cs-100-cs-100-hw-01.md");
        let text = pystr::read_text(&created).unwrap();
        assert!(text.contains("source_uid: \"bb:1\""));
        assert!(text.contains("due: 2026-09-11T23:59"));
        assert!(text.contains("created_by: blackboard"));
        assert!(text.contains("from blackboard"), "the description becomes the body");
        assert!(load_seen(&v).contains("bb:1"));
    }

    #[test]
    fn sync_is_idempotent_on_a_second_run() {
        let v = sync_vault();
        let events = [ev("bb:1", "HW 01", dt(2026, 9, 11))];
        assert_eq!(run(&v, &events, &[]).len(), 1);
        assert!(run(&v, &events, &[]).is_empty(), "nothing changed, so nothing is logged");
        let count = std::fs::read_dir(v.join("tasks")).unwrap().count();
        assert_eq!(count, 1, "a second run must not create a twin");
    }

    #[test]
    fn sync_updates_a_moved_due_date_in_place() {
        let v = sync_vault();
        run(&v, &[ev("bb:1", "HW 01", dt(2026, 9, 11))], &[]);
        let log = run(&v, &[ev("bb:1", "HW 01", dt(2026, 9, 18))], &[]);
        assert_eq!(log, vec!["updated task-hw-01: due"]);
        let text = pystr::read_text(&v.join("tasks").join("task-hw-01.md")).unwrap();
        assert!(text.contains("due: 2026-09-18T23:59"));
    }

    #[test]
    fn sync_skips_an_undated_event_and_institutional_noise() {
        let v = sync_vault();
        let log = run(
            &v,
            &[
                ev("bb:1", "No date", None),
                ev("_blackboard.data.calendar.CalendarEntry-_9_1", "Holiday", dt(2026, 11, 28)),
            ],
            &[],
        );
        assert_eq!(
            log,
            vec![
                "skipped (no date): No date",
                "skipped (institutional calendar entry): Holiday",
            ]
        );
        assert_eq!(std::fs::read_dir(v.join("tasks")).unwrap().count(), 0);
    }

    #[test]
    fn an_institutional_entry_that_matches_a_course_is_kept() {
        let v = sync_vault();
        let map = vec![("CS 100".to_string(), "cs-100".to_string())];
        let log = run(
            &v,
            &[ev("_blackboard.data.calendar.CalendarEntry-_9_1", "CS 100 review session", dt(2026, 11, 28))],
            &map,
        );
        assert_eq!(log, vec!["created cs-100-cs-100-review-session"]);
    }

    #[test]
    fn a_deleted_note_is_never_resurrected() {
        // The uid stays in the ledger, so a deliberate delete sticks.
        let v = sync_vault();
        let events = [ev("bb:1", "HW 01", dt(2026, 9, 11))];
        run(&v, &events, &[]);
        std::fs::remove_file(v.join("tasks").join("task-hw-01.md")).unwrap();

        let log = run(&v, &events, &[]);
        assert_eq!(log, vec!["skipped (deleted earlier): HW 01"]);
        assert_eq!(std::fs::read_dir(v.join("tasks")).unwrap().count(), 0);
    }

    #[test]
    fn an_unreadable_known_note_is_skipped_and_never_duplicated() {
        // THE trap. Falling through to creation here produces a "-2" twin, which is exactly the
        // failure S3 exists to prevent.
        let v = sync_vault();
        let broken = "---\ntitle: \"x\nsource_uid: \"bb:1\"\nstatus: [unclosed\n---\n\nbody\n";
        pystr::write_text(&v.join("tasks").join("broken.md"), broken).unwrap();

        let log = run(&v, &[ev("bb:1", "HW 01", dt(2026, 9, 11))], &[]);
        assert_eq!(log, vec!["skipped (unreadable): broken"]);
        assert_eq!(std::fs::read_dir(v.join("tasks")).unwrap().count(), 1, "no twin was created");
    }

    #[test]
    fn an_archived_or_completed_note_is_not_updated() {
        let v = sync_vault();
        run(&v, &[ev("bb:1", "HW 01", dt(2026, 9, 11))], &[]);
        let path = v.join("tasks").join("task-hw-01.md");
        update_frontmatter_fields(&path, &changes(&[("status", "done")])).unwrap();

        let log = run(&v, &[ev("bb:1", "HW 01", dt(2026, 9, 18))], &[]);
        assert!(log.is_empty(), "a completed task must not be reopened by a feed change: {log:?}");
        assert!(pystr::read_text(&path).unwrap().contains("due: 2026-09-11T23:59"));
    }

    #[test]
    fn a_slug_collision_gets_a_numeric_suffix() {
        let v = sync_vault();
        run(&v, &[ev("bb:1", "HW 01", dt(2026, 9, 11))], &[]);
        run(&v, &[ev("bb:2", "HW 01", dt(2026, 9, 12))], &[]);
        assert!(v.join("tasks").join("task-hw-01.md").exists());
        assert!(v.join("tasks").join("task-hw-01-2.md").exists());
    }

    /// Preserved defect 13: create writes the em dash literally, update escapes it.
    #[test]
    fn non_ascii_titles_are_escaped_on_update_but_not_on_create() {
        let v = sync_vault();
        run(&v, &[ev("bb:1", "GN 103 — Hausaufgaben", dt(2026, 9, 11))], &[]);
        let path = v.join("tasks").join("task-gn-103-hausaufgaben.md");
        assert!(
            pystr::read_text(&path).unwrap().contains("title: \"GN 103 — Hausaufgaben\""),
            "create uses ensure_ascii=False"
        );

        run(&v, &[ev("bb:1", "GN 103 — Hausaufgaben (rev)", dt(2026, 9, 11))], &[]);
        let after = pystr::read_text(&path).unwrap();
        assert!(
            after.contains("title: \"GN 103 \\u2014 Hausaufgaben (rev)\""),
            "update uses ensure_ascii=True — preserved, not fixed. Got: {after}"
        );
    }

    #[test]
    fn json_dumps_ascii_matches_pythons_default() {
        // Every expectation here is a string the interpreter actually produced.
        assert_eq!(
            json_dumps_ascii("üben — Übungen"),
            "\"\\u00fcben \\u2014 \\u00dcbungen\""
        );
        assert_eq!(json_dumps_ascii("plain"), "\"plain\"");
        assert_eq!(json_dumps_ascii("quote\"and\\slash"), r#""quote\"and\\slash""#);
        // Above the BMP becomes a surrogate pair, as Python does.
        assert_eq!(json_dumps_ascii("\u{1D11E}"), "\"\\ud834\\udd1e\"");
    }

    // -- `main`: the orchestrator, ported from `engine/ingest.py:388` -------

    fn ingest_vault(tag: &str, config: &str) -> PathBuf {
        let d = std::env::temp_dir().join(format!("qo-ingest-main-{tag}-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&d);
        std::fs::create_dir_all(d.join("config")).unwrap();
        std::fs::create_dir_all(d.join("tasks")).unwrap();
        if !config.is_empty() { pystr::write_text(&d.join("config").join("ingest.yaml"), config).unwrap(); }
        d
    }
    fn feed() -> impl Fn(&str) -> Result<String, String> {
        |_: &str| pystr::read_text(Path::new("tests/fixtures/blackboard.ics")).map_err(|e| e.to_string())
    }
    /// The day the two feed tests below run on. Every course item in `blackboard.ics` is due after
    /// it, so a first run (no `today.md`) creates them; unpinned, the wall clock passed the last of
    /// them (2026-10-15) and R-OB-3 archived every one.
    const FEED_DAY: Date = Date::constant(2026, 8, 29);
    const ONE_COURSE: &str = "ics_url: \"https://example.invalid/learn.ics\"\ntimezone: America/Chicago\ncourse_map:\n  PH-106: ph-106\n  GN-103: gn-103\n  CS-100: cs-100\n";

    fn records_of(vault: &Path) -> Vec<serde_json::Value> {
        let mut out = Vec::new();
        for e in std::fs::read_dir(vault.join("state").join("journal")).unwrap().flatten() {
            let text = pystr::read_text(&e.path()).unwrap();
            for line in text.split('\n').filter(|l| !l.trim().is_empty()) {
                out.push(serde_json::from_str(line).unwrap());
            }
        }
        out
    }

    /// Port of `tests/test_ingest.py:302` — bad YAML is "config unreadable", not "no ics_url", and
    /// the two branches must stay distinguishable by message as well as by exit code.
    #[test]
    fn ingest_main_returns_1_and_says_so_when_the_config_is_unreadable() {
        let v = ingest_vault("badyaml", "ics_url: [unclosed\n");
        let (code, lines) = run_lines(&v, "cli", None, None);
        assert_eq!(code, 1);
        assert!(lines[0].starts_with("ingest: config unreadable:"), "{lines:?}");
        let _ = std::fs::remove_dir_all(&v);
    }

    /// Port of `tests/test_ingest.py:359` — a missing file takes the same branch as a broken one.
    #[test]
    fn ingest_main_returns_1_when_the_config_file_is_missing() {
        let v = ingest_vault("nofile", "");
        let (code, lines) = run_lines(&v, "cli", None, None);
        assert_eq!(code, 1);
        assert!(lines[0].starts_with("ingest: config unreadable:"), "{lines:?}");
        let _ = std::fs::remove_dir_all(&v);
    }

    /// A vault with a config but no feed. Not one of Python's four, but it is the branch the app's
    /// `has_ics_url` skip exists for, so it is pinned on both sides of that decision.
    #[test]
    fn ingest_main_returns_1_when_no_ics_url_is_configured() {
        let v = ingest_vault("nourl", "timezone: America/Chicago\nics_url: \"   \"\n");
        let (code, lines) = run_lines(&v, "cli", None, None);
        assert_eq!(code, 1);
        assert_eq!(lines, vec!["ingest: no ics_url configured".to_string()]);
        let _ = std::fs::remove_dir_all(&v);
    }

    /// Port of `tests/test_ingest.py:312`: under a runner these writes are a RUN's writes. Default
    /// `via: cli, run_id: null` would make every ingested assignment unattributable to the run that
    /// fetched it, and indistinguishable from a hand invocation.
    #[test]
    fn ingest_main_journals_under_the_runners_via_and_run_id() {
        let v = ingest_vault("via", ONE_COURSE);
        let f = feed();
        let (code, lines) = run_lines_on(&v, "local-runner", Some("local-2026-08-29T17:00:00Z"), Some(&f), Some(FEED_DAY));
        assert_eq!(code, 0);
        assert!(lines.last().unwrap().starts_with("ingest: "), "{lines:?}");
        let records = records_of(&v);
        assert!(!records.is_empty(), "ingest wrote notes but journalled nothing");
        for r in &records {
            assert_eq!(r["via"], "local-runner");
            assert_eq!(r["run_id"], "local-2026-08-29T17:00:00Z");
            assert_eq!(r["actor"], "agent:ingest.blackboard");
        }
        let _ = std::fs::remove_dir_all(&v);
    }

    /// Port of `tests/test_ingest.py:341`, plus the happy path Python covers through its other 35
    /// tests: the fixture feed creates real task notes and the summary line counts them.
    #[test]
    fn ingest_main_defaults_to_cli_via_and_creates_tasks_from_the_feed() {
        let v = ingest_vault("happy", ONE_COURSE);
        let f = feed();
        let (code, lines) = run_lines_on(&v, "cli", None, Some(&f), Some(FEED_DAY));
        assert_eq!(code, 0);
        let records = records_of(&v);
        for r in &records { assert_eq!(r["via"], "cli"); assert!(r["run_id"].is_null()); }
        let created = std::fs::read_dir(v.join("tasks")).unwrap().flatten().count();
        assert!(created > 0, "the fixture feed created notes");
        assert!(lines.iter().any(|l| l.starts_with("created ")), "{lines:?}");
        assert!(v.join("state").join("ingest-seen.md").is_file(), "the seen ledger is written");
        // A second run over the same feed is idempotent: the ledger stops every uid.
        let (code2, lines2) = run_lines_on(&v, "cli", None, Some(&f), Some(FEED_DAY));
        assert_eq!(code2, 0);
        assert!(!lines2.iter().any(|l| l.starts_with("created ")), "{lines2:?}");
        let _ = std::fs::remove_dir_all(&v);
    }

    // -- T8b: ruling 3 for `title` and `due` (email spec D11 (5), §13 Q7, test 31b; plan P12) --

    mod feed_cards {
        use super::*;
        use crate::write::{write_literals, WriteContext, WriteOpts};
        use serde_yaml_ng::{Mapping, Value};

        const NOTE: &str = "tasks/task-hw-01.md";
        const CARD: &str = "approvals/amend-task-hw-01-due.md";
        const TODAY: Date = Date::constant(2026, 8, 28);

        /// "HW 01" as the feed says it, due `due`.
        fn hw(due: Option<Due>) -> Event {
            ev("bb:1", "HW 01", due)
        }

        /// A vault where ingest made "HW 01", due 2026-09-11.
        fn made() -> PathBuf {
            let v = sync_vault();
            assert_eq!(run(&v, &[hw(dt(2026, 9, 11))], &[]), ["created task-hw-01"]);
            v
        }

        /// A console write as the vault's human (no `config/actor.yaml`: the legacy token).
        fn by_hand(vault: &Path, rel: &str, field: &str, literal: &str) {
            let ctx = WriteContext::new(crate::journal::LEGACY_HUMAN_ACTOR, "dashboard");
            let literals = [(field.to_string(), literal.to_string())];
            let mut journal = crate::journal::Journal::new(vault);
            write_literals(vault, rel, &literals, &ctx, &mut journal, &WriteOpts::default()).unwrap();
        }

        fn files(vault: &Path, folder: &str) -> Vec<String> {
            let Ok(entries) = std::fs::read_dir(vault.join(folder)) else { return Vec::new() };
            let mut names: Vec<String> = entries.flatten().map(|e| e.file_name().to_string_lossy().to_string()).collect();
            names.sort();
            names
        }

        fn front(vault: &Path, rel: &str) -> Mapping {
            split_frontmatter(&pystr::read_text(&vault.join(rel)).unwrap()).unwrap().0
        }

        fn get(vault: &Path, rel: &str, key: &str) -> Option<String> {
            crate::yaml::get(&front(vault, rel), key).and_then(crate::yaml::text)
        }

        /// A card's one change, `(field, from, to)`.
        fn change(vault: &Path, rel: &str) -> (String, Option<String>, Option<String>) {
            let Some(Value::Mapping(changes)) = crate::yaml::get(&front(vault, rel), "changes").cloned() else { panic!("{rel}") };
            assert_eq!(changes.len(), 1, "one field per card: {changes:?}");
            let (field, spec) = changes.into_iter().next().unwrap();
            let side = |key: &str| spec.get(key).and_then(crate::yaml::text);
            (crate::yaml::text(&field).unwrap(), side("from"), side("to"))
        }

        fn change_of(field: &str, from: &str, to: &str) -> (String, Option<String>, Option<String>) {
            (field.to_string(), Some(from.to_string()), Some(to.to_string()))
        }

        /// Spec test 31b: after the student moved `due` by hand (an approved mail amend writes the
        /// same human `set`, T8), a feed saying otherwise files **one** `kind: amend` card from the
        /// student's value to the feed's, and the note keeps the student's value.
        #[test]
        fn a_hand_set_due_gets_one_amend_card_from_the_feed_and_keeps_its_value() {
            let v = made();
            by_hand(&v, NOTE, "due", "2026-09-18T23:59");
            assert_eq!(run(&v, &[hw(dt(2026, 9, 16))], &[]), ["proposed task-hw-01: due"]);
            assert_eq!(get(&v, NOTE, "due").as_deref(), Some("2026-09-18T23:59"));
            assert_eq!(files(&v, "approvals"), ["amend-task-hw-01-due.md"]);
            assert_eq!(change(&v, CARD), change_of("due", "2026-09-18T23:59", "2026-09-16T23:59"));
            let created: Vec<_> = crate::journal::Journal::new(&v).read(None, None).into_iter()
                .filter(|r| r.get("path").and_then(|p| p.as_str()) == Some(CARD)).collect();
            assert_eq!(created.len(), 1);
            assert_eq!(created[0].get("op").and_then(|o| o.as_str()), Some("create"));
            assert_eq!(created[0].get("actor").and_then(|a| a.as_str()), Some("agent:ingest.blackboard"), "the ingest actor");
        }

        /// Spec test 31b: no second card while the first is pending (`write::find_pending_amendment`).
        #[test]
        fn a_second_ingest_files_no_second_card() {
            let v = made();
            by_hand(&v, NOTE, "due", "2026-09-18T23:59");
            run(&v, &[hw(dt(2026, 9, 16))], &[]);
            assert_eq!(run(&v, &[hw(dt(2026, 9, 16))], &[]), Vec::<String>::new());
            assert_eq!(run(&v, &[hw(dt(2026, 9, 14))], &[]), Vec::<String>::new(), "a moved feed value waits too");
            assert_eq!(files(&v, "approvals"), ["amend-task-hw-01-due.md"]);
            assert_eq!(get(&v, NOTE, "due").as_deref(), Some("2026-09-18T23:59"));
        }

        /// P12: once the student rejects the card, the same feed value is not proposed again; a new
        /// feed value is.
        #[test]
        fn a_rejected_feed_value_is_not_proposed_again_until_the_feed_changes() {
            let v = made();
            by_hand(&v, NOTE, "due", "2026-09-18T23:59");
            run(&v, &[hw(dt(2026, 9, 16))], &[]);
            by_hand(&v, CARD, "status", "rejected");
            let now = DateTime::constant(2026, 8, 28, 12, 0, 0, 0);
            let mut journal = crate::journal::Journal::new(&v);
            let settled = crate::approvals::process_approvals(&v, TODAY, now, &crate::approvals::default_ctx(), &mut journal);
            assert_eq!(settled.rejected, ["amend-task-hw-01-due"]);
            assert_eq!(files(&v, "archive"), ["amend-task-hw-01-due.md"]);
            assert_eq!(run(&v, &[hw(dt(2026, 9, 16))], &[]), Vec::<String>::new());
            assert_eq!(files(&v, "approvals"), Vec::<String>::new());
            assert_eq!(run(&v, &[hw(dt(2026, 9, 14))], &[]), ["proposed task-hw-01: due"]);
            assert_eq!(change(&v, CARD), change_of("due", "2026-09-18T23:59", "2026-09-14T23:59"));
            assert_eq!(get(&v, NOTE, "due").as_deref(), Some("2026-09-18T23:59"));
        }

        /// Spec test 31b: a `title` the student set by hand behaves the same, and its card applies
        /// (PQ5 (c): `title` is amendable on a `created_by: blackboard` card).
        #[test]
        fn a_hand_set_title_behaves_the_same() {
            let v = made();
            by_hand(&v, NOTE, "title", "\"HW 01: my linked lists\"");
            let renamed = ev("bb:1", "HW 01 \u{2014} Linked Lists", dt(2026, 9, 11));
            assert_eq!(run(&v, &[renamed.clone()], &[]), ["proposed task-hw-01: title"]);
            assert_eq!(get(&v, NOTE, "title").as_deref(), Some("HW 01: my linked lists"));
            let card = "approvals/amend-task-hw-01-title.md";
            assert_eq!(change(&v, card), change_of("title", "HW 01: my linked lists", "HW 01 \u{2014} Linked Lists"));
            assert_eq!(run(&v, &[renamed], &[]), Vec::<String>::new(), "no second card");
            by_hand(&v, card, "status", "approved");
            let now = DateTime::constant(2026, 8, 28, 12, 0, 0, 0);
            let mut journal = crate::journal::Journal::new(&v);
            let settled = crate::approvals::process_approvals(&v, TODAY, now, &crate::approvals::default_ctx(), &mut journal);
            assert_eq!(settled.executed, ["amend-task-hw-01-title"], "{:?}", settled.warnings);
            assert_eq!(get(&v, NOTE, "title").as_deref(), Some("HW 01 \u{2014} Linked Lists"));
        }

        /// The feed still owns every field no human edited: an unedited `due`, or one only an agent
        /// set, is overwritten exactly as today, in the same pass that files a card for another.
        #[test]
        fn an_unedited_due_is_overwritten_exactly_as_today() {
            let v = made();
            by_hand(&v, NOTE, "effort_hours", "3.0");
            let agent = WriteContext::new("agent:knowlu.enrich", "cli");
            let mut journal = crate::journal::Journal::new(&v);
            write_literals(&v, NOTE, &[("due".to_string(), "2026-09-12T23:59".to_string())], &agent, &mut journal, &WriteOpts::default()).unwrap();
            assert_eq!(run(&v, &[hw(dt(2026, 9, 18))], &[]), ["updated task-hw-01: due"]);
            assert_eq!(get(&v, NOTE, "due").as_deref(), Some("2026-09-18T23:59"));
            assert_eq!(files(&v, "approvals"), Vec::<String>::new());
            // A hand-set title is a card; the unedited due beside it is still written.
            by_hand(&v, NOTE, "title", "\"HW 01 (mine)\"");
            assert_eq!(run(&v, &[hw(dt(2026, 9, 20))], &[]), ["updated task-hw-01: due", "proposed task-hw-01: title"]);
            assert_eq!(get(&v, NOTE, "due").as_deref(), Some("2026-09-20T23:59"));
            assert_eq!(files(&v, "approvals"), ["amend-task-hw-01-title.md"]);
        }

        /// Pre-PR review finding feed-source (2026-10-20): every ICS feed reaches the card writer,
        /// Canvas feeds included, and the engine does not know which LMS a feed comes from. The
        /// card's text must name no LMS; `created_by` stays the shipped `blackboard` token that
        /// `approvals.rs` accepts a title amendment on.
        #[test]
        fn a_feed_card_names_no_lms_and_keeps_its_creator() {
            let v = made();
            by_hand(&v, NOTE, "due", "2026-09-18T23:59");
            by_hand(&v, NOTE, "title", "\"HW 01 (mine)\"");
            run(&v, &[ev("bb:1", "HW 01 revised", dt(2026, 9, 16))], &[]);
            for card in [CARD, "approvals/amend-task-hw-01-title.md"] {
                let text = pystr::read_text(&v.join(card)).unwrap();
                assert!(!text.replace("created_by: blackboard", "").to_lowercase().contains("blackboard"), "{card}: {text}");
                assert_eq!(get(&v, card, "created_by").as_deref(), Some(crate::approvals::TITLE_CARD_CREATOR));
            }
        }

        /// Spec test 31b and plan P12: the card is in the shape `approvals::validate_amendment`
        /// accepts, `created_by: blackboard` with no `source_uid` and no judgment fields, pending
        /// from the run's day, with its own "Why proposed" line, never the text "re-judged".
        #[test]
        fn the_feed_card_passes_validate_amendment_and_never_says_rejudged() {
            let v = made();
            by_hand(&v, NOTE, "due", "2026-09-18T23:59");
            run(&v, &[hw(dt(2026, 9, 16))], &[]);
            let meta = front(&v, CARD);
            assert_eq!(crate::approvals::validate_amendment(&v, &meta), Ok(crate::ids::resolve_lenient(&v.join(NOTE))));
            let text = pystr::read_text(&v.join(CARD)).unwrap();
            let id = get(&v, CARD, "id").unwrap();
            assert!(crate::ids::is_id(&id) && id.starts_with("appr_"), "{id}");
            let want = format!(
                "---\ntype: approval\nkind: amend\ntitle: Your course feed changed due for HW 01\nstatus: pending\n\
                 target: tasks/task-hw-01.md\nproposed_at: 2026-08-28\nfirst_proposed_at: 2026-08-28\nexpires: null\n\
                 snooze_until: null\ncreated_by: blackboard\nchanges:\n  due:\n    from: 2026-09-18T23:59\n    to: 2026-09-16T23:59\n\
                 id: {id}\n---\n\n**Why proposed:** Your course feed now says 2026-09-16T23:59; you set 2026-09-18T23:59 by hand, \
                 so this is a proposal\n{}",
                crate::write::AMEND_BUTTONS
            );
            assert_eq!(text, want);
            assert!(!text.contains("re-judged"));
        }

        /// Spec test 31b: approving the card moves `due` to the feed's value, and a later feed move
        /// is a card again: the student's hand set still stands in the journal.
        #[test]
        fn approving_the_feed_card_moves_due_and_a_later_feed_move_is_a_card_again() {
            let v = made();
            by_hand(&v, NOTE, "due", "2026-09-18T23:59");
            run(&v, &[hw(dt(2026, 9, 16))], &[]);
            by_hand(&v, CARD, "status", "approved");
            let now = DateTime::constant(2026, 8, 28, 12, 0, 0, 0);
            let mut journal = crate::journal::Journal::new(&v);
            let settled = crate::approvals::process_approvals(&v, TODAY, now, &crate::approvals::default_ctx(), &mut journal);
            assert_eq!(settled.executed, ["amend-task-hw-01-due"], "{:?}", settled.warnings);
            assert_eq!(get(&v, NOTE, "due").as_deref(), Some("2026-09-16T23:59"));
            assert_eq!(run(&v, &[hw(dt(2026, 9, 16))], &[]), Vec::<String>::new(), "the feed and the note agree");
            assert_eq!(run(&v, &[hw(dt(2026, 9, 14))], &[]), ["proposed task-hw-01: due"]);
            assert_eq!(change(&v, CARD), change_of("due", "2026-09-16T23:59", "2026-09-14T23:59"));
            assert_eq!(get(&v, NOTE, "due").as_deref(), Some("2026-09-16T23:59"));
        }

        /// Plan §10 R14: one card per field, so each is its own open decision.
        #[test]
        fn a_hand_set_title_and_due_both_contradicted_file_two_one_field_cards() {
            let v = made();
            by_hand(&v, NOTE, "title", "\"HW 01 (mine)\"");
            by_hand(&v, NOTE, "due", "2026-09-18T23:59");
            let log = run(&v, &[ev("bb:1", "HW 01 revised", dt(2026, 9, 16))], &[]);
            assert_eq!(log, ["proposed task-hw-01: title", "proposed task-hw-01: due"]);
            assert_eq!(files(&v, "approvals"), ["amend-task-hw-01-due.md", "amend-task-hw-01-title.md"]);
            assert_eq!(change(&v, CARD), change_of("due", "2026-09-18T23:59", "2026-09-16T23:59"));
            assert_eq!(change(&v, "approvals/amend-task-hw-01-title.md"), change_of("title", "HW 01 (mine)", "HW 01 revised"));
        }

        /// Plan §10 R14: a pending mail card for `{due}` is the open decision on the note's due, so
        /// the feed files none beside it, and a feed card is what the mail card writer's own
        /// `write::find_pending_amendment` check finds. A card for another field blocks nothing.
        #[test]
        fn a_pending_mail_due_card_blocks_the_feed_due_card_and_the_reverse() {
            let v = made();
            by_hand(&v, NOTE, "title", "\"HW 01 (mine)\"");
            by_hand(&v, NOTE, "due", "2026-09-18T23:59");
            let mail = "---\ntype: approval\nkind: amend\ntitle: Lab moved\nstatus: pending\ntarget: tasks/task-hw-01.md\n\
                        proposed_at: 2026-08-27\nfirst_proposed_at: 2026-08-27\nexpires: null\nsnooze_until: null\n\
                        created_by: mail\nchanges:\n  due:\n    from: 2026-09-18T23:59\n    to: 2026-09-19T23:59\n---\n\nMail.\n";
            pystr::write_text(&v.join("approvals").join("amend-mail.md"), mail).unwrap();
            let log = run(&v, &[ev("bb:1", "HW 01 revised", dt(2026, 9, 16))], &[]);
            assert_eq!(log, ["proposed task-hw-01: title"]);
            assert_eq!(files(&v, "approvals"), ["amend-mail.md", "amend-task-hw-01-title.md"]);
            // The reverse: with no mail card, the feed's `{due}` card is the pending one found.
            let v = made();
            by_hand(&v, NOTE, "due", "2026-09-18T23:59");
            run(&v, &[hw(dt(2026, 9, 16))], &[]);
            let due = std::collections::BTreeSet::from(["due".to_string()]);
            let found = crate::write::find_pending_amendment(&v, NOTE, &due);
            assert_eq!(found.map(|p| crate::ids::rel(&v, &p)).as_deref(), Some(CARD));
        }

        /// Plan §10 R14: `validate_amendment` refuses a null `from` or `to`, so a card for a value the
        /// student cleared by hand could never apply; a value no card can carry on one line could
        /// not either. Each keeps the note's value and files no card. A feed that drops the date is
        /// skipped before any of this, as today.
        #[test]
        fn a_hand_cleared_due_or_a_feed_cleared_due_keeps_the_value_and_files_no_card() {
            let v = made();
            by_hand(&v, NOTE, "due", "null");
            by_hand(&v, NOTE, "title", "null");
            assert_eq!(run(&v, &[hw(dt(2026, 9, 16))], &[]), Vec::<String>::new());
            assert_eq!((get(&v, NOTE, "due"), get(&v, NOTE, "title")), (None, None));
            assert_eq!(files(&v, "approvals"), Vec::<String>::new());
            let v = made();
            by_hand(&v, NOTE, "due", "2026-09-18T23:59");
            assert_eq!(run(&v, &[hw(None)], &[]), ["skipped (no date): HW 01"]);
            assert_eq!(get(&v, NOTE, "due").as_deref(), Some("2026-09-18T23:59"));
            by_hand(&v, NOTE, "title", "\"HW 01 (mine)\"");
            assert_eq!(run(&v, &[ev("bb:1", "HW 01\u{2028}revised", dt(2026, 9, 18))], &[]), Vec::<String>::new());
            assert_eq!(files(&v, "approvals"), Vec::<String>::new());
        }

        /// Plan §10 R3: a feed card is a proposal like any other. Sixteen contradicted notes in one
        /// run file sixteen cards; `approvals::defer_over_budget` at 15 snoozes one to tomorrow and
        /// deletes none.
        #[test]
        fn the_feed_card_is_charged_to_the_daily_budget() {
            let v = sync_vault();
            let events = |day: i8| (1..=16).map(|n| ev(&format!("bb:{n}"), &format!("HW {n:02}"), dt(2026, 9, day))).collect::<Vec<_>>();
            assert_eq!(run(&v, &events(11), &[]).len(), 16);
            for n in 1..=16 {
                by_hand(&v, &format!("tasks/task-hw-{n:02}.md"), "due", "2026-09-18T23:59");
            }
            assert_eq!(run(&v, &events(16), &[]).len(), 16);
            assert_eq!(crate::approvals::count_proposals_created(&v, TODAY), 16);
            let mut journal = crate::journal::Journal::new(&v);
            let deferred = crate::approvals::defer_over_budget(&v, TODAY, 15, &crate::approvals::default_ctx(), &mut journal);
            assert_eq!(deferred, ["amend-task-hw-16-due"]);
            assert_eq!(files(&v, "approvals").len(), 16, "deferred, never deleted");
            let last = "approvals/amend-task-hw-16-due.md";
            assert_eq!((get(&v, last, "status"), get(&v, last, "snooze_until")), (Some("snoozed".into()), Some("2026-08-29".into())));
        }

        /// Plan §10 R14 with R3 (Checkpoint A's failing input): the card `defer_over_budget` snoozed
        /// is still the open decision on its field. The next ingest files no second card for it,
        /// whether the feed says the same value or a new one, so the overflow never grows a slot at
        /// a time.
        #[test]
        fn a_card_the_budget_snoozed_is_still_the_open_decision() {
            let v = sync_vault();
            let events = |day: i8| (1..=16).map(|n| ev(&format!("bb:{n}"), &format!("HW {n:02}"), dt(2026, 9, day))).collect::<Vec<_>>();
            run(&v, &events(11), &[]);
            for n in 1..=16 {
                by_hand(&v, &format!("tasks/task-hw-{n:02}.md"), "due", "2026-09-18T23:59");
            }
            assert_eq!(run(&v, &events(16), &[]).len(), 16);
            let mut journal = crate::journal::Journal::new(&v);
            let deferred = crate::approvals::defer_over_budget(&v, TODAY, 15, &crate::approvals::default_ctx(), &mut journal);
            assert_eq!(deferred, ["amend-task-hw-16-due"]);
            assert_eq!(run(&v, &events(16), &[]), Vec::<String>::new());
            assert_eq!(run(&v, &events(14), &[]), Vec::<String>::new(), "a moved feed value waits too");
            assert_eq!(files(&v, "approvals").len(), 16);
            assert!(!v.join("approvals/amend-task-hw-16-due-2.md").exists());
            assert_eq!(get(&v, "tasks/task-hw-16.md", "due").as_deref(), Some("2026-09-18T23:59"));
        }

        /// Plan §10 R14 with R16: a card the student snoozed by hand is still the open decision on
        /// its field while it sleeps. So is a mail card held to the vault's second day (R16), whoever
        /// filed it; a snoozed card for another field blocks nothing.
        #[test]
        fn a_card_the_student_snoozed_is_still_the_open_decision() {
            let v = made();
            by_hand(&v, NOTE, "due", "2026-09-18T23:59");
            run(&v, &[hw(dt(2026, 9, 16))], &[]);
            by_hand(&v, CARD, "status", "snoozed");
            by_hand(&v, CARD, "snooze_until", "2026-09-01");
            assert_eq!(run(&v, &[hw(dt(2026, 9, 16))], &[]), Vec::<String>::new());
            assert_eq!(run(&v, &[hw(dt(2026, 9, 14))], &[]), Vec::<String>::new(), "a moved feed value waits too");
            assert_eq!(files(&v, "approvals"), ["amend-task-hw-01-due.md"]);
            assert_eq!(get(&v, NOTE, "due").as_deref(), Some("2026-09-18T23:59"));
            // A mail `{due}` card snoozed to day 2 blocks the feed's due card, and not its title card.
            let v = made();
            by_hand(&v, NOTE, "title", "\"HW 01 (mine)\"");
            by_hand(&v, NOTE, "due", "2026-09-18T23:59");
            let mail = "---\ntype: approval\nkind: amend\ntitle: Lab moved\nstatus: snoozed\ntarget: tasks/task-hw-01.md\n\
                        proposed_at: 2026-08-29\nfirst_proposed_at: 2026-08-28\nexpires: null\nsnooze_until: 2026-08-29\n\
                        created_by: mail\nchanges:\n  due:\n    from: 2026-09-18T23:59\n    to: 2026-09-19T23:59\n---\n\nMail.\n";
            pystr::write_text(&v.join("approvals").join("amend-mail.md"), mail).unwrap();
            let log = run(&v, &[ev("bb:1", "HW 01 revised", dt(2026, 9, 16))], &[]);
            assert_eq!(log, ["proposed task-hw-01: title"]);
            assert_eq!(files(&v, "approvals"), ["amend-mail.md", "amend-task-hw-01-title.md"]);
        }
    }

    // -- O1 T2b: the gradebook half of "turned-in work is not a task" -------------------------
    //
    // Red state: `gradable_id` and `gradebook_done` are `todo!("O1 T5")` stubs and `run_lines_on`
    // reads no `grades/`, so a turned-in item is still created live. Every grade note below is built
    // by `grades::apply` on an inline v2-shape bundle (no top-level `score`, the number under
    // `displayGrade`), the path a real capture takes. Every course, column and name is invented.

    mod gradebook {
        use super::*;
        use serde_json::{json, Value as J};

        const DAY: Date = Date::constant(2026, 8, 29);
        const CONFIG: &str = "ics_url: \"https://example.invalid/learn.ics\"\ntimezone: America/Chicago\ncourse_map:\n  XX-101: xx-101\n";
        const GB: &str = "_blackboard.platform.gradebook2.GradableItem-";

        #[derive(Clone, Copy)]
        enum Row {
            Score(f64),
            Percent(f64),
            NeedsGrading,
            Exempt,
            InProgress,
            Absent,
            Overall(f64),
        }

        /// The invented gradebook: `(column id, name)`.
        const ESSAY1: (&str, &str) = ("_11_1", "XX-101 Essay One");
        const ESSAY2: (&str, &str) = ("_12_1", "XX-101 Essay Two");
        const QUIZ: (&str, &str) = ("_13_1", "XX-101 Quiz One");
        const LAB1: (&str, &str) = ("_14_1", "XX-101 Lab One");
        const LAB2: (&str, &str) = ("_15_1", "XX-101 Lab Two");
        const LAB3: (&str, &str) = ("_16_1", "XX-101 Lab Three");
        const LAB4: (&str, &str) = ("_17_1", "XX-101 Lab Four");
        const TOTAL: (&str, &str) = ("_19_1", "XX-101 Total");
        const EARLY: (&str, &str) = ("_21_1", "XX-101 Warmup");

        fn vault(tag: &str) -> PathBuf {
            let d = std::env::temp_dir().join(format!("qo-ingest-gb-{tag}-{}-{:?}", std::process::id(), std::thread::current().id()));
            let _ = std::fs::remove_dir_all(&d);
            for folder in ["config", "tasks", "courses"] {
                std::fs::create_dir_all(d.join(folder)).unwrap();
            }
            pystr::write_text(&d.join("config").join("ingest.yaml"), CONFIG).unwrap();
            pystr::write_text(&d.join("courses").join("xx-101.md"), "---\ntitle: Intro Studies\ncode: _900_1\n---\n").unwrap();
            d
        }

        /// A slot that has already been ranked once: ingest is no longer a first run.
        fn ranked(v: &Path) {
            std::fs::create_dir_all(v.join("state")).unwrap();
            pystr::write_text(&v.join("state").join("today.md"), "# Today\n").unwrap();
        }

        /// `grades::apply` on an inline v2 bundle with one column and one row per item.
        fn grade(v: &Path, rows: &[((&str, &str), Row)]) {
            let (mut columns, mut grades) = (Vec::<J>::new(), Vec::<J>::new());
            for ((id, name), row) in rows {
                let mut column = json!({"id": id, "name": name, "score": {"possible": 40.0}});
                let row_json = |status: &str, exempt: bool, display: J| {
                    json!({"columnId": id, "userId": "_77_1", "status": status, "exempt": exempt, "displayGrade": display})
                };
                match *row {
                    Row::Score(n) => grades.push(row_json("Graded", false, json!({"scaleType": "Score", "score": n, "possible": 40.0}))),
                    Row::Percent(n) => grades.push(row_json("Graded", false, json!({"scaleType": "Percent", "score": n, "possible": 100.0}))),
                    Row::NeedsGrading => grades.push(row_json("NeedsGrading", false, J::Null)),
                    Row::Exempt => grades.push(row_json("Graded", true, J::Null)),
                    Row::InProgress => grades.push(row_json("InProgress", false, J::Null)),
                    Row::Absent => {}
                    Row::Overall(n) => {
                        column["externalGrade"] = json!(true);
                        grades.push(row_json("Graded", false, json!({"scaleType": "Score", "score": n, "possible": 100.0})));
                    }
                }
                columns.push(column);
            }
            let bundle: crate::grades::Bundle = serde_json::from_value(json!({
                "schema": 1, "lms": "blackboard", "host": "lms.example.test",
                "fetched_at": "2026-08-29T12:00:00Z", "user_id": "_77_1",
                "courses": [{"membership": {"courseId": "_900_1"}, "columns": columns, "grades": grades, "categories": null}],
            }))
            .unwrap();
            let ctx = crate::write::WriteContext::new(crate::grades::ACTOR, "local-runner");
            let mut journal = crate::journal::Journal::new(v);
            let report = crate::grades::apply(v, &bundle, &ctx, &mut journal);
            assert!(report.not_written.is_empty(), "{:?}", report.not_written);
        }

        /// One VEVENT: `uid`, `title`, local due `YYYYMMDDTHHMMSS`.
        fn vevent(uid: &str, title: &str, due: &str) -> String {
            format!("BEGIN:VEVENT\nUID:{uid}\nSUMMARY:{title}\nDESCRIPTION:from the feed\nDTSTART:{due}\nEND:VEVENT\n")
        }

        fn gb_event(item: (&str, &str), due: &str) -> String {
            vevent(&format!("{GB}{}", item.0), item.1, due)
        }

        fn ics(events: &[String]) -> String {
            format!("BEGIN:VCALENDAR\nVERSION:2.0\n{}END:VCALENDAR\n", events.concat())
        }

        const LATER: &str = "20260905T235900";
        const PAST: &str = "20260820T235900";

        fn ingest(v: &Path, feed_text: &str) -> (i32, Vec<String>) {
            let text = feed_text.to_string();
            let fetch = move |_: &str| -> Result<String, String> { Ok(text.clone()) };
            run_lines_on(v, "local-runner", None, Some(&fetch), Some(DAY))
        }

        fn folder_of(v: &Path, uid: &str) -> String {
            existing_by_uid(v)
                .get(uid)
                .and_then(|p| p.parent()?.file_name().map(|n| n.to_string_lossy().into_owned()))
                .unwrap_or_else(|| "absent".to_string())
        }

        fn field_of(v: &Path, uid: &str, key: &str) -> Option<String> {
            let path = existing_by_uid(v).get(uid)?.clone();
            let meta = split_frontmatter(&pystr::read_text(&path).ok()?).ok()?.0;
            crate::yaml::get(&meta, key).and_then(crate::yaml::text)
        }

        fn uid(item: (&str, &str)) -> String {
            format!("{GB}{}", item.0)
        }

        fn stem_of(v: &Path, uid: &str) -> String {
            existing_by_uid(v).get(uid).unwrap().file_stem().unwrap().to_string_lossy().into_owned()
        }

        /// Journal records on the note holding `uid`, oldest first.
        fn trail(v: &Path, uid: &str) -> Vec<J> {
            let path = existing_by_uid(v).get(uid).unwrap().clone();
            let rel = crate::ids::rel(v, &path);
            records_of(v).into_iter().filter(|r| r["path"] == json!(rel)).collect()
        }

        fn agent_marks(v: &Path, uid: &str) -> usize {
            trail(v, uid).iter().filter(|r| r["actor"] == json!(crate::completion::ACTOR) && r["new"] == json!("done")).count()
        }

        fn hand_set(v: &Path, rel: &str, value: &str) {
            let ctx = crate::write::WriteContext::new(crate::journal::LEGACY_HUMAN_ACTOR, "dashboard");
            let mut journal = crate::journal::Journal::new(v);
            crate::write::write_literals(v, rel, &[("status".to_string(), value.to_string())], &ctx, &mut journal, &crate::write::WriteOpts::default()).unwrap();
        }

        /// The whole invented feed: eight gradable items, a calendar entry and an assignment-style UID
        /// that happen to carry course titles, none of which may join the gradebook.
        fn full_feed() -> String {
            ics(&[
                gb_event(ESSAY1, LATER),
                gb_event(ESSAY2, LATER),
                gb_event(QUIZ, LATER),
                gb_event(LAB1, LATER),
                gb_event(LAB2, LATER),
                gb_event(LAB3, LATER),
                gb_event(LAB4, LATER),
                gb_event(TOTAL, LATER),
                vevent("_blackboard.data.calendar.CalendarEntry-_99_1", "XX-101 Office Hours", LATER),
                vevent("BB-ASSIGN-5001@lms.example.test", "XX-101 Paper", LATER),
            ])
        }

        fn all_rows() -> Vec<((&'static str, &'static str), Row)> {
            vec![
                (ESSAY1, Row::Score(31.5)),
                (ESSAY2, Row::Score(0.0)),
                (QUIZ, Row::NeedsGrading),
                (LAB1, Row::Exempt),
                (LAB2, Row::Percent(85.0)),
                (LAB3, Row::InProgress),
                (LAB4, Row::Absent),
                (TOTAL, Row::Overall(90.0)),
            ]
        }

        // 14
        #[test]
        fn the_gradable_id_is_the_suffix_after_exactly_the_gradebook_prefix() {
            assert_eq!(gradable_id("_blackboard.platform.gradebook2.GradableItem-_4686399_1"), Some("_4686399_1"));
            assert_eq!(gradable_id("_blackboard.data.calendar.CalendarEntry-_4686399_1"), None);
            assert_eq!(gradable_id("BB-ASSIGN-1111@ua.blackboard.com"), None);
            assert_eq!(gradable_id("_blackboard.platform.gradebook2.GradableItem-"), None);
            assert_eq!(gradable_id(""), None);
        }

        // 15
        #[test]
        fn the_done_rule_is_needs_grading_exempt_or_graded_above_zero() {
            for (status, score, want) in [
                ("needs-grading", None, true),
                ("exempt", None, true),
                ("graded", Some(31.5), true),
                ("graded", Some(0.5), true),
                ("graded", Some(0.0), false),
                ("graded", None, false),
                ("in-progress", None, false),
                ("in-progress", Some(12.0), false),
                ("not-submitted", None, false),
                ("removed", Some(10.0), false),
            ] {
                assert_eq!(gradebook_done(status, score), want, "{status} {score:?}");
            }
        }

        // 15, through captured scores: the notes come from `grades::apply`, never a hand-written `score:`.
        #[test]
        fn captured_v2_rows_feed_the_done_rule_as_the_notes_hold_them() {
            let v = vault("captured");
            grade(&v, &all_rows());
            let note = |item: (&str, &str)| {
                let rel = crate::grades::note_path("xx-101", item.0);
                let meta = split_frontmatter(&pystr::read_text(&v.join(rel)).unwrap()).unwrap().0;
                let status = crate::yaml::get(&meta, "status").and_then(crate::yaml::text).unwrap();
                let score = crate::yaml::get(&meta, "score").and_then(crate::yaml::f64_of);
                (status, score)
            };
            let done = |item| { let (s, sc) = note(item); gradebook_done(&s, sc) };
            assert_eq!(note(ESSAY1), ("graded".to_string(), Some(31.5)));
            assert!(done(ESSAY1), "a Score row at 31.5 is turned in");
            assert!(!done(ESSAY2), "the same row at 0 is not");
            assert!(!done(LAB2), "a Percent row at 85 has no captured score and stays live");
            assert!(done(QUIZ) && done(LAB1));
            assert!(!done(LAB3) && !done(LAB4));
            let _ = std::fs::remove_dir_all(&v);
        }

        // 15a, 16a
        #[test]
        fn a_first_ingest_archives_joined_done_items_as_imported_done_and_creates_the_rest() {
            let v = vault("first");
            grade(&v, &all_rows());
            let (code, lines) = ingest(&v, &full_feed());
            assert_eq!(code, 0, "{lines:?}");
            for item in [ESSAY1, QUIZ, LAB1] {
                assert_eq!(folder_of(&v, &uid(item)), "archive", "{item:?}");
                assert_eq!(field_of(&v, &uid(item), "archived_reason").as_deref(), Some(IMPORTED_DONE), "{item:?}");
                assert_eq!(field_of(&v, &uid(item), "status").as_deref(), Some("archived"));
                assert!(load_seen(&v).contains(&uid(item)), "{item:?} must be in the seen ledger");
            }
            for live in [uid(ESSAY2), uid(LAB2), uid(LAB3), uid(LAB4), uid(TOTAL), "_blackboard.data.calendar.CalendarEntry-_99_1".to_string(), "BB-ASSIGN-5001@lms.example.test".to_string()] {
                assert_eq!(folder_of(&v, &live), "tasks", "{live}");
                assert_eq!(field_of(&v, &live, "status").as_deref(), Some("active"), "{live}");
            }
            assert_eq!(lines.iter().filter(|l| l.starts_with("archived (imported-done)")).count(), 3, "{lines:?}");
            assert!(lines.contains(&"ingest: first run — 0 item(s) already past were archived".to_string()), "{lines:?}");
            assert!(lines.contains(&"ingest: first run — 3 item(s) already turned in were archived".to_string()), "{lines:?}");
            let _ = std::fs::remove_dir_all(&v);
        }

        // 16
        #[test]
        fn past_and_done_on_a_first_ingest_is_imported_past_and_counts_only_as_past() {
            let v = vault("precedence");
            grade(&v, &[(EARLY, Row::NeedsGrading), (ESSAY1, Row::Score(31.5))]);
            let feed_text = ics(&[gb_event(EARLY, PAST), gb_event(ESSAY1, LATER)]);
            let (_, lines) = ingest(&v, &feed_text);
            assert_eq!(field_of(&v, &uid(EARLY), "archived_reason").as_deref(), Some(IMPORTED_PAST));
            assert_eq!(field_of(&v, &uid(ESSAY1), "archived_reason").as_deref(), Some(IMPORTED_DONE));
            assert!(lines.contains(&"ingest: first run — 1 item(s) already past were archived".to_string()), "{lines:?}");
            assert!(lines.contains(&"ingest: first run — 1 item(s) already turned in were archived".to_string()), "{lines:?}");
            let _ = std::fs::remove_dir_all(&v);
        }

        // 16a: the second line is present only when its count is above 0.
        #[test]
        fn the_turned_in_line_is_absent_when_nothing_was_turned_in() {
            let v = vault("noline");
            grade(&v, &[(ESSAY2, Row::Score(0.0)), (LAB2, Row::Percent(85.0))]);
            let (_, lines) = ingest(&v, &ics(&[gb_event(ESSAY2, LATER), gb_event(LAB2, LATER)]));
            assert!(lines.contains(&"ingest: first run — 0 item(s) already past were archived".to_string()), "{lines:?}");
            assert!(!lines.iter().any(|l| l.contains("turned in")), "{lines:?}");
            assert_eq!(folder_of(&v, &uid(ESSAY2)), "tasks");
            let _ = std::fs::remove_dir_all(&v);
        }

        // 17
        #[test]
        fn a_later_ingest_marks_a_live_task_done_when_the_gradebook_now_says_so_in_feed_order() {
            let v = vault("later");
            ranked(&v);
            grade(&v, &[(QUIZ, Row::InProgress), (LAB1, Row::InProgress)]);
            let feed_text = ics(&[gb_event(LAB1, LATER), gb_event(QUIZ, LATER)]);
            let (_, first) = ingest(&v, &feed_text);
            assert!(!first.iter().any(|l| l.starts_with("marked done")), "{first:?}");
            assert_eq!(field_of(&v, &uid(QUIZ), "status").as_deref(), Some("active"));

            grade(&v, &[(QUIZ, Row::NeedsGrading), (LAB1, Row::Exempt)]);
            let (code, lines) = ingest(&v, &feed_text);
            assert_eq!(code, 0, "{lines:?}");
            let marked: Vec<&String> = lines.iter().filter(|l| l.starts_with("marked done:")).collect();
            assert_eq!(
                marked,
                [
                    &format!("marked done: {} (gradebook)", stem_of(&v, &uid(LAB1))),
                    &format!("marked done: {} (gradebook)", stem_of(&v, &uid(QUIZ))),
                ],
                "feed order, the lab first: {lines:?}"
            );
            for item in [QUIZ, LAB1] {
                assert_eq!(field_of(&v, &uid(item), "status").as_deref(), Some("done"), "{item:?}");
                assert_eq!(folder_of(&v, &uid(item)), "tasks", "marked done, not archived");
                let last = trail(&v, &uid(item)).pop().unwrap();
                assert_eq!(last["actor"], json!(crate::completion::ACTOR));
                assert_eq!((last["field"].as_str(), last["old"].as_str(), last["new"].as_str()), (Some("status"), Some("active"), Some("done")));
                assert_eq!(last["evidence"]["source"], json!("blackboard-gradebook"));
            }
            // Idempotent: a third ingest marks nothing and writes nothing more.
            let (_, again) = ingest(&v, &feed_text);
            assert!(!again.iter().any(|l| l.starts_with("marked done")), "{again:?}");
            assert_eq!(agent_marks(&v, &uid(QUIZ)), 1);
            let _ = std::fs::remove_dir_all(&v);
        }

        // 18
        #[test]
        fn a_reopened_task_and_a_task_with_a_pending_or_rejected_card_stay_active() {
            let v = vault("protected");
            ranked(&v);
            let items = [ESSAY1, QUIZ, LAB1];
            let feed_text = ics(&items.map(|i| gb_event(i, LATER)));
            grade(&v, &[(ESSAY1, Row::InProgress), (QUIZ, Row::InProgress), (LAB1, Row::InProgress)]);
            ingest(&v, &feed_text);

            // The reopened one: marked once, then the student sets it back to active.
            grade(&v, &[(ESSAY1, Row::Score(31.5)), (QUIZ, Row::InProgress), (LAB1, Row::InProgress)]);
            ingest(&v, &feed_text);
            assert_eq!(field_of(&v, &uid(ESSAY1), "status").as_deref(), Some("done"));
            let essay_rel = crate::ids::rel(&v, existing_by_uid(&v).get(&uid(ESSAY1)).unwrap());
            hand_set(&v, &essay_rel, "active");

            // A pending card on the quiz, a rejected one on the lab, both filed as mail files them.
            let file_card = |item: (&str, &str)| {
                let rel = crate::ids::rel(&v, existing_by_uid(&v).get(&uid(item)).unwrap());
                let evidence = crate::completion::Evidence { source: "email".to_string(), summary: "an email says so".to_string(), detail: serde_json::Map::new() };
                let ctx = crate::write::WriteContext::new(crate::completion::ACTOR, "local-runner");
                let mut journal = crate::journal::Journal::new(&v);
                match crate::completion::propose_done(&v, Path::new(&rel), &evidence, DAY, &ctx, &mut journal, false, None).unwrap() {
                    crate::completion::Outcome::Proposed(card) => card,
                    other => panic!("card not filed: {other:?}"),
                }
            };
            let pending = file_card(QUIZ);
            let rejected = file_card(LAB1);
            hand_set(&v, &crate::ids::rel(&v, &rejected), "rejected");
            let mut journal = crate::journal::Journal::new(&v);
            let result = crate::approvals::process_approvals(&v, DAY, DAY.at(12, 0, 0, 0), &crate::approvals::default_ctx(), &mut journal);
            assert_eq!(result.rejected.len(), 1, "{:?}", result.warnings);
            let pending_bytes = std::fs::read(&pending).unwrap();
            let before = [agent_marks(&v, &uid(ESSAY1)), agent_marks(&v, &uid(QUIZ)), agent_marks(&v, &uid(LAB1))];

            grade(&v, &[(ESSAY1, Row::Score(31.5)), (QUIZ, Row::NeedsGrading), (LAB1, Row::Exempt)]);
            let (code, lines) = ingest(&v, &feed_text);
            assert_eq!(code, 0, "{lines:?}");
            assert!(!lines.iter().any(|l| l.starts_with("marked done")), "{lines:?}");
            for item in items {
                assert_eq!(field_of(&v, &uid(item), "status").as_deref(), Some("active"), "{item:?}");
            }
            assert_eq!([agent_marks(&v, &uid(ESSAY1)), agent_marks(&v, &uid(QUIZ)), agent_marks(&v, &uid(LAB1))], before);
            assert_eq!(std::fs::read(&pending).unwrap(), pending_bytes, "the pending card was touched");
            let _ = std::fs::remove_dir_all(&v);
        }

        // 19
        #[test]
        fn no_grades_folder_changes_nothing_and_an_empty_one_is_the_same() {
            let feed_text = ics(&[gb_event(ESSAY1, LATER), gb_event(QUIZ, PAST), vevent("BB-ASSIGN-5001@lms.example.test", "XX-101 Paper", LATER)]);
            let run = |tag: &str, empty_grades_dir: bool| {
                let v = vault(tag);
                if empty_grades_dir {
                    std::fs::create_dir_all(v.join("grades")).unwrap();
                }
                let (code, lines) = ingest(&v, &feed_text);
                let mut notes = Vec::new();
                for folder in ["tasks", "archive"] {
                    let Ok(entries) = std::fs::read_dir(v.join(folder)) else { continue };
                    for e in entries.flatten() {
                        let text = pystr::read_text(&e.path()).unwrap();
                        let body: Vec<&str> = text.lines().filter(|l| !l.starts_with("id:")).collect();
                        notes.push((folder, e.file_name().to_string_lossy().into_owned(), body.join("\n")));
                    }
                }
                notes.sort();
                let _ = std::fs::remove_dir_all(&v);
                (code, lines, notes)
            };
            let without = run("nogrades-a", false);
            let with_empty = run("nogrades-b", true);
            assert_eq!(without, with_empty);
            let (_, lines, notes) = without;
            assert!(!lines.iter().any(|l| l.contains("turned in") || l.starts_with("marked done")), "{lines:?}");
            assert_eq!(notes.iter().filter(|n| n.0 == "tasks").count(), 2, "only the future feed items are live: {notes:?}");
        }
    }

    // -- F6c (plan 2026-10-23 wave B, §14.45 and §14.46): a removed course's items, and the
    // bounded uid pin. Synthetic codes and ids only; every vault is a temp dir.

    /// The bounded uid pin (§14.46). A fragment of the exact form `_<digits>_<digits>` matches the
    /// uid only when no ASCII letter, digit or `_` precedes it and no ASCII digit follows it.
    mod bounded_pin {
        use super::*;

        const GB: &str = "_blackboard.platform.gradebook2.GradableItem-";

        fn pins(order: &[(&str, &str)]) -> Vec<(String, String)> {
            order.iter().map(|(f, s)| (f.to_string(), s.to_string())).collect()
        }

        fn attributed(uid: &str, map: &[(String, String)]) -> Option<String> {
            match_course_fields(&format!("{GB}{uid}"), "Quiz", map)
        }

        #[test]
        fn pin_71_1_does_not_claim_71_10_in_either_order() {
            for order in [[("_71_1", "a-101"), ("_71_10", "b-202")], [("_71_10", "b-202"), ("_71_1", "a-101")]] {
                let map = pins(&order);
                assert_eq!(attributed("_71_10", &map), Some("b-202".to_string()), "{order:?}");
                assert_eq!(attributed("_71_1", &map), Some("a-101".to_string()), "{order:?}");
            }
        }

        #[test]
        fn a_lone_pin_does_not_claim_a_longer_id_that_starts_with_it() {
            let map = pins(&[("_71_1", "a-101")]);
            assert_eq!(attributed("_71_10", &map), None);
            assert_eq!(attributed("_71_1", &map), Some("a-101".to_string()));
        }

        /// The plan's own example, kept as written: `_171_1` never contains `_71_1`, so this holds
        /// today and must keep holding.
        #[test]
        fn a_lone_pin_does_not_claim_171_1() {
            let map = pins(&[("_71_1", "a-101")]);
            assert_eq!(attributed("_171_1", &map), None);
        }

        /// What the "not preceded" half of the bound is for: a pin inside a longer id whose front
        /// part is a digit or an underscore (`_5_71_1` holds `_71_1`; so does `__71_1`).
        #[test]
        fn a_lone_pin_does_not_claim_an_id_it_sits_inside_the_tail_of() {
            let map = pins(&[("_71_1", "a-101")]);
            assert_eq!(attributed("_5_71_1", &map), None, "preceded by a digit");
            assert_eq!(attributed("x_71_1", &map), None, "preceded by a letter");
            assert_eq!(attributed("_9__71_1", &map), None, "preceded by an underscore");
        }

        #[test]
        fn a_pin_at_the_very_end_of_the_uid_matches() {
            let map = pins(&[("_71_1", "a-101")]);
            assert_eq!(match_course_fields("x-_71_1", "", &map), Some("a-101".to_string()));
            assert_eq!(match_course_fields("_71_1", "", &map), Some("a-101".to_string()));
        }

        #[test]
        fn a_pin_followed_by_a_non_digit_matches() {
            let map = pins(&[("_71_1", "a-101")]);
            assert_eq!(match_course_fields("x-_71_1@x", "", &map), Some("a-101".to_string()));
            assert_eq!(match_course_fields("x-_71_1.ics", "", &map), Some("a-101".to_string()));
        }

        /// Only the exact pin form is bounded; every other fragment keeps today's substring test.
        #[test]
        fn a_fragment_that_is_not_a_pin_keeps_the_substring_test() {
            let map = pins(&[("SYN-110", "syn-110"), ("_71_1_", "odd"), ("_71", "short")]);
            assert_eq!(match_course_fields("xSYN-110x", "", &map), Some("syn-110".to_string()));
            assert_eq!(match_course_fields("a_71_1_9", "", &pins(&[("_71_1_", "odd")])), Some("odd".to_string()));
            assert_eq!(match_course_fields("a_71_9", "", &pins(&[("_71", "short")])), Some("short".to_string()));
        }

        /// A pin that fails its bound still falls through to pass (b), the title test, unchanged.
        #[test]
        fn a_pin_that_fails_the_bound_falls_through_to_the_title_pass() {
            let map = pins(&[("_71_1", "a-101"), ("SYN 110", "syn-110")]);
            assert_eq!(
                match_course_fields(&format!("{GB}_71_10"), "SYN 110 Quiz", &map),
                Some("syn-110".to_string())
            );
        }
    }

    /// `course_exclude` end to end: `run_lines_on` over an inline feed and a config in a temp vault.
    /// The key is a single-line flow sequence of fragments (F6b writes it that way, §14.45).
    mod course_exclude {
        use super::*;

        const DAY: Date = Date::constant(2026, 8, 29);
        const GB: &str = "_blackboard.platform.gradebook2.GradableItem-";
        const HEAD: &str = "ics_url: \"https://example.invalid/learn.ics\"\ntimezone: America/Chicago\n";
        const KEPT_AND_REMOVED: &str = "course_map:\n  SYN 110: syn-110\n  _900071_1: syn-110\ncourse_exclude: [SYN 120, _900072_1]\n";
        const LEFT_OUT: &str = "item(s) from courses you removed were left out";

        fn vault(tag: &str, config: &str) -> PathBuf {
            let d = std::env::temp_dir().join(format!("knowlu-f6c-{}-{tag}", std::process::id()));
            let _ = std::fs::remove_dir_all(&d);
            std::fs::create_dir_all(d.join("config")).unwrap();
            std::fs::create_dir_all(d.join("tasks")).unwrap();
            pystr::write_text(&d.join("config").join("ingest.yaml"), &format!("{HEAD}{config}")).unwrap();
            d
        }

        fn vevent(uid: &str, title: &str, due: &str) -> String {
            format!("BEGIN:VEVENT\nUID:{uid}\nSUMMARY:{title}\nDESCRIPTION:from the feed\nDTSTART:{due}\nEND:VEVENT\n")
        }

        fn go(v: &Path, events: &[String]) -> (i32, Vec<String>) {
            let text = format!("BEGIN:VCALENDAR\nVERSION:2.0\n{}END:VCALENDAR\n", events.concat());
            let fetch = move |_: &str| -> Result<String, String> { Ok(text.clone()) };
            run_lines_on(v, "cli", None, Some(&fetch), Some(DAY))
        }

        fn names(v: &Path, folder: &str) -> Vec<String> {
            let mut out: Vec<String> = std::fs::read_dir(v.join(folder))
                .map(|rd| rd.flatten().map(|e| e.file_name().to_string_lossy().into_owned()).collect())
                .unwrap_or_default();
            out.sort();
            out
        }

        fn journal_len(v: &Path) -> usize {
            std::fs::read_dir(v.join("state").join("journal"))
                .map(|rd| {
                    rd.flatten()
                        .map(|e| pystr::read_text(&e.path()).unwrap().lines().filter(|l| !l.trim().is_empty()).count())
                        .sum::<usize>()
                })
                .unwrap_or(0)
        }

        const LATER: &str = "20260905T235900";

        #[test]
        fn a_removed_courses_item_never_becomes_a_task() {
            let v = vault("removed", KEPT_AND_REMOVED);
            let by_code = vevent("bb:syn120-1", "SYN 120 Quiz 1", LATER);
            let by_pin = vevent(&format!("{GB}_900072_1"), "Quiz 2", LATER);
            let (code, lines) = go(&v, &[by_code, by_pin]);
            assert_eq!(code, 0, "{lines:?}");
            assert_eq!(names(&v, "tasks"), Vec::<String>::new(), "{lines:?}");
            assert_eq!(names(&v, "archive"), Vec::<String>::new());
            assert_eq!(journal_len(&v), 0, "no journal record on a left-out item's account");
            assert!(load_seen(&v).is_empty(), "left-out uids stay out of the seen ledger");
            assert!(
                lines.iter().any(|l| l.contains(&format!("2 {LEFT_OUT}"))),
                "the run names the count: {lines:?}"
            );
            let _ = std::fs::remove_dir_all(&v);
        }

        #[test]
        fn a_kept_course_wins_an_item_both_claim() {
            let v = vault("kept-wins", KEPT_AND_REMOVED);
            let by_title = vevent("bb:both-1", "SYN 110 and SYN 120 joint lab", LATER);
            let by_pin = vevent(&format!("{GB}_900071_1"), "SYN 120 Quiz 3", LATER);
            let (code, lines) = go(&v, &[by_title, by_pin]);
            assert_eq!(code, 0, "{lines:?}");
            let tasks = names(&v, "tasks");
            assert_eq!(tasks.len(), 2, "{tasks:?} {lines:?}");
            assert!(tasks.iter().all(|t| t.starts_with("syn-110-")), "both under the kept slug: {tasks:?}");
            assert!(!lines.iter().any(|l| l.contains(LEFT_OUT)), "nothing was left out: {lines:?}");
            let _ = std::fs::remove_dir_all(&v);
        }

        #[test]
        fn an_unmatched_item_is_still_imported_course_less() {
            let v = vault("unmatched", KEPT_AND_REMOVED);
            let (code, lines) = go(&v, &[vevent("bb:free-1", "Reading response", LATER)]);
            assert_eq!(code, 0, "{lines:?}");
            assert_eq!(names(&v, "tasks"), vec!["task-reading-response.md".to_string()], "{lines:?}");
            assert!(!lines.iter().any(|l| l.contains(LEFT_OUT)), "{lines:?}");
            let _ = std::fs::remove_dir_all(&v);
        }

        /// A task already in the vault whose item is now excluded: no update, no archive.
        #[test]
        fn an_existing_task_is_left_untouched_when_its_course_is_removed() {
            let v = vault("existing", "course_map:\n  SYN 110: syn-110\n");
            let (code, lines) = go(&v, &[vevent("bb:syn120-1", "SYN 120 Quiz 1", LATER)]);
            assert_eq!(code, 0, "{lines:?}");
            let before = names(&v, "tasks");
            assert_eq!(before, vec!["task-syn-120-quiz-1.md".to_string()], "{lines:?}");
            let bytes = std::fs::read(v.join("tasks").join(&before[0])).unwrap();
            let records = journal_len(&v);
            // Delete the ledger too, so the known-by-uid backfill path is the one under test.
            std::fs::remove_file(v.join("state").join("ingest-seen.md")).unwrap();

            pystr::write_text(&v.join("config").join("ingest.yaml"), &format!("{HEAD}{KEPT_AND_REMOVED}")).unwrap();
            // The feed moves the due date, which today is an in-place update.
            let (code, lines) = go(&v, &[vevent("bb:syn120-1", "SYN 120 Quiz 1", "20260912T235900")]);
            assert_eq!(code, 0, "{lines:?}");
            assert_eq!(names(&v, "tasks"), before, "still in tasks/");
            assert_eq!(names(&v, "archive"), Vec::<String>::new(), "not archived");
            assert_eq!(std::fs::read(v.join("tasks").join(&before[0])).unwrap(), bytes, "no update: {lines:?}");
            assert_eq!(journal_len(&v), records, "no journal record");
            assert!(!lines.iter().any(|l| l.starts_with("updated ")), "{lines:?}");
            assert!(!load_seen(&v).contains("bb:syn120-1"), "no backfill for a left-out uid");
            let _ = std::fs::remove_dir_all(&v);
        }

        /// No key, or an empty one: nothing changes, and no line is added.
        #[test]
        fn with_no_course_exclude_key_nothing_changes() {
            for (tag, extra) in [("nokey", ""), ("emptykey", "course_exclude: []\n")] {
                let v = vault(tag, &format!("course_map:\n  SYN 110: syn-110\n{extra}"));
                let (code, lines) = go(&v, &[vevent("bb:syn120-1", "SYN 120 Quiz 1", LATER)]);
                assert_eq!(code, 0, "{lines:?}");
                assert_eq!(names(&v, "tasks"), vec!["task-syn-120-quiz-1.md".to_string()], "{tag}: {lines:?}");
                assert!(!lines.iter().any(|l| l.contains(LEFT_OUT)), "{tag}: {lines:?}");
                let _ = std::fs::remove_dir_all(&v);
            }
        }

        /// An empty fragment would be a substring of every uid; reading drops it.
        #[test]
        fn an_empty_fragment_leaves_nothing_out() {
            for (tag, key) in [("empty", "['']"), ("blank", "[' ', SYN 120]")] {
                let v = vault(tag, &format!("course_exclude: {key}
"));
                let (code, lines) = go(&v, &[vevent("bb:free-1", "Reading response", LATER)]);
                assert_eq!(code, 0, "{lines:?}");
                assert_eq!(names(&v, "tasks"), vec!["task-reading-response.md".to_string()], "{tag}: {lines:?}");
                assert!(!lines.iter().any(|l| l.contains(LEFT_OUT)), "{tag}: {lines:?}");
                let _ = std::fs::remove_dir_all(&v);
            }
        }

        /// A turned-in grade row marks an existing task done, unless its course was removed: a done
        /// mark is a write on that item's account. The kept run is the control.
        #[test]
        fn the_gradebook_does_not_mark_a_left_out_task_done() {
            use serde_json::json;
            let item = format!("{GB}_13_1");
            for (tag, exclude, want) in [("gb-kept", "", "done"), ("gb-removed", "course_exclude: [SYN 120]
", "active")] {
                let v = vault(tag, "course_map:
  SYN 110: syn-110
");
                // Not a first run, so the grade does not archive the item on creation.
                std::fs::create_dir_all(v.join("state")).unwrap();
                pystr::write_text(&v.join("state").join("today.md"), "# Today
").unwrap();
                std::fs::create_dir_all(v.join("courses")).unwrap();
                pystr::write_text(&v.join("courses").join("syn-120.md"), "---
title: Synthetic Studies
code: _900_1
---
").unwrap();
                let (code, lines) = go(&v, &[vevent(&item, "SYN 120 Quiz One", LATER)]);
                assert_eq!(code, 0, "{lines:?}");
                let bundle: crate::grades::Bundle = serde_json::from_value(json!({
                    "schema": 1, "lms": "blackboard", "host": "lms.example.test",
                    "fetched_at": "2026-08-29T12:00:00Z", "user_id": "_77_1",
                    "courses": [{"membership": {"courseId": "_900_1"},
                        "columns": [{"id": "_13_1", "name": "SYN 120 Quiz One", "score": {"possible": 40.0}}],
                        "grades": [{"columnId": "_13_1", "userId": "_77_1", "status": "NeedsGrading", "exempt": false, "displayGrade": null}],
                        "categories": null}],
                })).unwrap();
                let gctx = crate::write::WriteContext::new(crate::grades::ACTOR, "local-runner");
                let mut journal = crate::journal::Journal::new(&v);
                let report = crate::grades::apply(&v, &bundle, &gctx, &mut journal);
                assert!(report.not_written.is_empty(), "{:?}", report.not_written);

                pystr::write_text(&v.join("config").join("ingest.yaml"), &format!("{HEAD}course_map:
  SYN 110: syn-110
{exclude}")).unwrap();
                let (code, lines) = go(&v, &[vevent(&item, "SYN 120 Quiz One", LATER)]);
                assert_eq!(code, 0, "{lines:?}");
                let task = names(&v, "tasks").into_iter().find(|n| n.contains("quiz-one")).unwrap_or_else(|| panic!("{tag}: task moved: {lines:?}"));
                let text = pystr::read_text(&v.join("tasks").join(&task)).unwrap();
                let meta = split_frontmatter(&text).unwrap().0;
                let status = crate::yaml::get(&meta, "status").and_then(crate::yaml::text).unwrap_or_default();
                assert_eq!(status, want, "{tag}: {lines:?}");
                let _ = std::fs::remove_dir_all(&v);
            }
        }

        /// An excluded pin is bounded too: `_900072_1` leaves out its own item and not `_900072_10`.
        #[test]
        fn an_excluded_pin_is_bounded() {
            let v = vault("bounded", "course_exclude: [_900072_1]\n");
            let own = vevent(&format!("{GB}_900072_1"), "Quiz A", LATER);
            let other = vevent(&format!("{GB}_900072_10"), "Quiz B", LATER);
            let (code, lines) = go(&v, &[own, other]);
            assert_eq!(code, 0, "{lines:?}");
            assert_eq!(names(&v, "tasks"), vec!["task-quiz-b.md".to_string()], "{lines:?}");
            assert!(lines.iter().any(|l| l.contains(&format!("1 {LEFT_OUT}"))), "{lines:?}");
            let _ = std::fs::remove_dir_all(&v);
        }
    }
}
