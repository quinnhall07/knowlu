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
        if uid.contains(fragment.as_str()) {
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
/// This is how course-less gradebook items get pinned by their Blackboard item id.
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
/// slot order is `coursework → ingest → judge → rank`. So on a genuinely fresh vault the
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
    let default_ctx = crate::write::WriteContext::new("agent:ingest.blackboard", "cli");
    let ctx = ctx.unwrap_or(&default_ctx);
    let stamp_date = today.unwrap_or_else(|| jiff::Zoned::now().date());
    let stamp = stamp_date.strftime("%Y-%m-%d").to_string();

    let mut log: Vec<String> = Vec::new();
    let mut known = existing_by_uid(vault);
    let mut seen = load_seen(vault);
    let tasks_dir = vault.join("tasks");
    let _ = std::fs::create_dir_all(&tasks_dir);

    for event in events {
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

            let mut changes: Vec<(String, String)> = Vec::new();
            let current_title = crate::yaml::get(&meta, "title")
                .and_then(crate::yaml::text)
                .unwrap_or_default();
            if current_title != event.title {
                // ensure_ascii=True here, unlike the create branch below. Preserved defect 13.
                changes.push(("title".to_string(), json_dumps_ascii(&event.title)));
            }
            if due_changed(crate::yaml::get(&meta, "due"), &new_due) {
                changes.push(("due".to_string(), new_due.clone()));
            }
            if !changes.is_empty() {
                let rel_path = crate::ids::rel(vault, &path);
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
        if past_due {
            let slug = format!("{}-{}", course.clone().unwrap_or_else(|| "task".into()), slugify(&event.title));
            let archive_dir = vault.join("archive");
            let _ = std::fs::create_dir_all(&archive_dir);
            let mut path = archive_dir.join(format!("{slug}.md"));
            let mut suffix = 2;
            while path.exists() {
                path = archive_dir.join(format!("{slug}-{suffix}.md"));
                suffix += 1;
            }
            let body = IMPORTED_PAST_TEMPLATE
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
                    log.push(format!("archived (imported-past) {stem}"));
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
    log
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
        crate::cloudmodel::CloudError::Status { code: 404, .. } => {
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
    let mut journal = crate::journal::Journal::new(vault);
    let mut log = sync_tasks(&events, vault, &course_map, Some(&ctx), &mut journal, None, first_run);
    if first_run {
        log.push(format!("ingest: first run — {} item(s) already past were archived",
            log.iter().filter(|l| l.starts_with("archived (imported-past)")).count()));
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
        let (code, lines) = run_lines(&v, "local-runner", Some("local-2026-08-29T17:00:00Z"), Some(&f));
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
        let (code, lines) = run_lines(&v, "cli", None, Some(&f));
        assert_eq!(code, 0);
        let records = records_of(&v);
        for r in &records { assert_eq!(r["via"], "cli"); assert!(r["run_id"].is_null()); }
        let created = std::fs::read_dir(v.join("tasks")).unwrap().flatten().count();
        assert!(created > 0, "the fixture feed created notes");
        assert!(lines.iter().any(|l| l.starts_with("created ")), "{lines:?}");
        assert!(v.join("state").join("ingest-seen.md").is_file(), "the seen ledger is written");
        // A second run over the same feed is idempotent: the ledger stops every uid.
        let (code2, lines2) = run_lines(&v, "cli", None, Some(&f));
        assert_eq!(code2, 0);
        assert!(!lines2.iter().any(|l| l.starts_with("created ")), "{lines2:?}");
        let _ = std::fs::remove_dir_all(&v);
    }
}
