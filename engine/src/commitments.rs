//! The commitment model (spec `docs/specs/2026-09-23-commitment-model-design.md`): the notes in
//! `commitments/` — confirmed commitments, decline markers and the one planning day — read into
//! plain data, never re-dumped.
//!
//! `load` never fails (§6.5): a missing folder is empty, a note it cannot use is skipped with a
//! warning, notes are read in file-name order. Only `status: confirmed` counts for anything (§2.2);
//! a decline marker contributes its `source_uid` and nothing else (§2.3). Nothing here reads the
//! clock, the network or a model.

use std::collections::{BTreeMap, BTreeSet};
use std::path::{Path, PathBuf};
use std::sync::LazyLock;

use jiff::civil::{Date, DateTime, Time};
use regex::Regex;
use serde_yaml_ng::{Mapping, Value};

use crate::models::split_frontmatter;
use crate::planning::{day_key, DAY_KEYS};
use crate::pystr;
use crate::weekcal::{CommitmentSpan, DayKey};
use crate::yaml::{get, text};
use crate::yamlemit::{self, Node};

/// The approval kinds that never leave the device (R20): C3′'s sync reads this one constant and
/// keeps every card of these kinds, and every journal record about one, off the wire (phase 1s).
pub const LOCAL_CARD_KINDS: [&str; 2] = ["commitment-ask", "commitment-check"];

/// The vault folder the notes live in (one of `ids::NOTE_FOLDERS`).
pub const FOLDER: &str = "commitments";

/// How much a commitment binds (C1, R17). Ordered hard first, so a sort on it puts hard first.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum Level {
    Hard,
    Soft,
    Optional,
}

impl Level {
    pub fn parse(text: &str) -> Option<Level> {
        match text {
            "hard" => Some(Level::Hard),
            "soft" => Some(Level::Soft),
            "optional" => Some(Level::Optional),
            _ => None,
        }
    }

    pub fn as_str(self) -> &'static str {
        match self {
            Level::Hard => "hard",
            Level::Soft => "soft",
            Level::Optional => "optional",
        }
    }
}

/// The `planning-day` kind (§2.4): read into `Commitments::window`, never into `confirmed`.
pub const PLANNING_DAY: &str = "planning-day";

/// A known kind's default level (§2.2, C1, R17), `None` for a kind the spec does not name.
/// `planning-day` has no level that is used; `Optional` because it adds no busy time.
pub fn default_level(kind: &str) -> Option<Level> {
    match kind {
        "class" | "lab" | "work" | "exam" => Some(Level::Hard),
        "club" | "meeting" | "event" | "task-block" => Some(Level::Soft),
        "office-hours" | PLANNING_DAY => Some(Level::Optional),
        _ => None,
    }
}

/// One `meets` (or `window`) entry: wall-clock in the vault's timezone, `start < end`.
#[derive(Debug, Clone, PartialEq)]
pub struct Meet {
    pub days: Vec<DayKey>,
    pub start: Time,
    pub end: Time,
}

/// A confirmed note in `commitments/` (§2.1). `path` is vault-relative, forward slashes.
#[derive(Debug, Clone, PartialEq)]
pub struct Commitment {
    pub id: String,
    pub path: PathBuf,
    pub kind: String,
    pub level: Level,
    pub title: String,
    pub course: Option<String>,
    pub meets: Vec<Meet>,
    pub where_: Option<String>,
    pub from: Option<Date>,
    pub until: Option<Date>,
    pub source_uid: Option<String>,
}

/// Everything `load` found. `window` is the planning day per weekday (`DAY_KEYS` order); `None`
/// keeps `week_template.yaml`'s `day_start`/`day_end` (§6.3).
#[derive(Debug, Clone, Default)]
pub struct Commitments {
    pub confirmed: Vec<Commitment>,
    pub declined: BTreeSet<String>,
    pub window: [Option<(Time, Time)>; 7],
    pub planning_day: Option<Commitment>,
    pub warnings: Vec<String>,
}

impl Commitments {
    /// One span per `(meet, day)` of every confirmed **hard or soft** note. Optional notes,
    /// decline markers and the planning day add no busy time (§6.1).
    pub fn spans(&self) -> Vec<CommitmentSpan> {
        let mut out = Vec::new();
        for note in &self.confirmed {
            if note.level == Level::Optional {
                continue;
            }
            for meet in &note.meets {
                for day in &meet.days {
                    out.push(CommitmentSpan {
                        day,
                        start: meet.start,
                        end: meet.end,
                        from: note.from,
                        until: note.until,
                        title: note.title.clone(),
                        kind: note.kind.clone(),
                        source_uid: note.source_uid.clone().unwrap_or_default(),
                    });
                }
            }
        }
        out
    }
}

/// `"HH:MM"` 24-hour (an hour of one digit is tolerated), at most `23:59`. `24:00` and `25:00`
/// are not times (§2.4, plan review M15).
fn parse_time(raw: &str) -> Option<Time> {
    let (h, m) = raw.trim().split_once(':')?;
    let digits = |s: &str| !s.is_empty() && s.bytes().all(|b| b.is_ascii_digit());
    if !digits(h) || h.len() > 2 || !digits(m) || m.len() != 2 {
        return None;
    }
    let (hour, minute): (i8, i8) = (h.parse().ok()?, m.parse().ok()?);
    if hour > 23 || minute > 59 {
        return None;
    }
    Time::new(hour, minute, 0, 0).ok()
}

/// A day key as `DAY_KEYS`' own `&'static str`.
fn day_of(raw: &str) -> Option<DayKey> {
    DAY_KEYS.iter().copied().find(|k| *k == raw)
}

/// The raw text of an entry's field, for a warning.
fn raw_text(value: Option<&Value>) -> String {
    match value {
        Some(Value::Sequence(items)) => items
            .iter()
            .map(|v| text(v).unwrap_or_else(|| "?".into()))
            .collect::<Vec<_>>()
            .join(", "),
        Some(v) => text(v).unwrap_or_else(|| "?".into()),
        None => "?".into(),
    }
}

/// One `{days, start, end}` entry, or its raw `(days, start, end)` for the warning.
fn parse_entry(value: &Value) -> Result<Meet, (String, String, String)> {
    let map = match value {
        Value::Mapping(m) => Some(m),
        _ => None,
    };
    let field = |key: &str| map.and_then(|m| get(m, key));
    let raw = || {
        (
            raw_text(field("days")),
            raw_text(field("start")),
            raw_text(field("end")),
        )
    };
    let days: Option<Vec<DayKey>> = match field("days") {
        Some(Value::Sequence(items)) if !items.is_empty() => {
            let mut days: Vec<DayKey> = Vec::new();
            let mut ok = true;
            for item in items {
                match text(item).as_deref().and_then(day_of) {
                    Some(day) if !days.contains(&day) => days.push(day),
                    Some(_) => {}
                    None => ok = false,
                }
            }
            ok.then_some(days)
        }
        _ => None,
    };
    let start = field("start")
        .and_then(text)
        .as_deref()
        .and_then(parse_time);
    let end = field("end").and_then(text).as_deref().and_then(parse_time);
    match (days, start, end) {
        (Some(days), Some(start), Some(end)) if start < end => Ok(Meet { days, start, end }),
        _ => Err(raw()),
    }
}

/// A readable note's frontmatter, with its vault-relative path.
struct Read {
    rel: String,
    stem: String,
    meta: Mapping,
}

fn field_text(meta: &Mapping, key: &str) -> Option<String> {
    get(meta, key).and_then(text)
}

fn field_date(meta: &Mapping, key: &str, rel: &str, warnings: &mut Vec<String>) -> Option<Date> {
    let raw = field_text(meta, key)?;
    match raw.trim().parse::<Date>() {
        Ok(date) => Some(date),
        Err(_) => {
            warnings.push(format!("{rel}: {key} {raw} is not a date; ignored"));
            None
        }
    }
}

/// Every note in `commitments/` (§2), read and never re-dumped. Never fails (§6.5).
pub fn load(vault: &Path) -> Commitments {
    let mut out = Commitments::default();
    let dir = vault.join(FOLDER);
    let Ok(entries) = std::fs::read_dir(&dir) else {
        return out;
    };
    let mut names: Vec<String> = entries
        .filter_map(|e| e.ok())
        .filter(|e| e.path().is_file())
        .filter_map(|e| e.file_name().into_string().ok())
        .filter(|name| name.ends_with(".md"))
        .collect();
    names.sort();

    let mut confirmed: Vec<Commitment> = Vec::new();
    let mut planning: Vec<Read> = Vec::new();
    for name in names {
        let rel = format!("{FOLDER}/{name}");
        let stem = name.trim_end_matches(".md").to_string();
        let meta = match pystr::read_text(&dir.join(&name)).map(|t| split_frontmatter(&t)) {
            Ok(Ok((meta, _))) => meta,
            Ok(Err(e)) => {
                out.warnings
                    .push(format!("{rel}: unreadable frontmatter ({e}); ignored"));
                continue;
            }
            Err(e) => {
                out.warnings
                    .push(format!("{rel}: unreadable ({e}); ignored"));
                continue;
            }
        };
        match field_text(&meta, "status").as_deref() {
            Some("confirmed") => {}
            // §2.3: a marker contributes its key and nothing else is read.
            Some("declined") => {
                match field_text(&meta, "source_uid") {
                    Some(uid) => {
                        out.declined.insert(uid);
                    }
                    None => out
                        .warnings
                        .push(format!("{rel}: decline marker has no source_uid; ignored")),
                }
                continue;
            }
            other => {
                let status = other.unwrap_or("(none)");
                out.warnings.push(format!(
                    "{rel}: status {status} ignored; only confirmed counts"
                ));
                continue;
            }
        }
        let read = Read { rel, stem, meta };
        if field_text(&read.meta, "kind").as_deref() == Some(PLANNING_DAY) {
            planning.push(read);
        } else if let Some(note) = commitment(&read, &mut out.warnings) {
            confirmed.push(note);
        }
    }

    out.confirmed = without_duplicates(confirmed, &mut out.warnings);
    if let Some(winner) = pick_planning_day(planning, &mut out.warnings) {
        let (note, window) = planning_day(&winner, &mut out.warnings);
        out.planning_day = Some(note);
        out.window = window;
    }
    out
}

/// A confirmed note's fields under §2.2's rules, or `None` (with a warning) when no `meets` entry
/// is valid. An explicit valid `level` wins over the kind's default, an unknown kind included.
fn commitment(read: &Read, warnings: &mut Vec<String>) -> Option<Commitment> {
    let Read { rel, stem, meta } = read;
    let kind = field_text(meta, "kind").unwrap_or_default();
    let default = default_level(&kind).unwrap_or_else(|| {
        let shown = if kind.is_empty() {
            "(none)"
        } else {
            kind.as_str()
        };
        warnings.push(format!("{rel}: unknown kind {shown}; loaded as soft"));
        Level::Soft
    });
    let level = match field_text(meta, "level") {
        None => default,
        Some(raw) => Level::parse(&raw).unwrap_or_else(|| {
            warnings.push(format!("{rel}: unknown level {raw}; loaded as soft"));
            Level::Soft
        }),
    };
    let mut meets = Vec::new();
    if let Some(Value::Sequence(entries)) = get(meta, "meets") {
        for entry in entries {
            match parse_entry(entry) {
                Ok(meet) => meets.push(meet),
                Err((days, start, end)) => warnings.push(format!(
                    "{rel}: meets entry {days}: {start}–{end} invalid; skipped"
                )),
            }
        }
    }
    if meets.is_empty() {
        warnings.push(format!("{rel}: no valid meets entry; ignored"));
        return None;
    }
    Some(Commitment {
        id: field_text(meta, "id").unwrap_or_default(),
        path: PathBuf::from(rel),
        kind,
        level,
        title: field_text(meta, "title").unwrap_or_else(|| stem.clone()),
        course: field_text(meta, "course"),
        meets,
        where_: field_text(meta, "where"),
        from: field_date(meta, "from", rel, warnings),
        until: field_date(meta, "until", rel, warnings),
        source_uid: field_text(meta, "source_uid"),
    })
}

/// §2.5: of the confirmed notes sharing one `source_uid`, the lowest `id` is kept (then the lowest
/// path, so the pick is total) and the rest are named in one warning per key. File-name order is
/// kept for the survivors.
fn without_duplicates(notes: Vec<Commitment>, warnings: &mut Vec<String>) -> Vec<Commitment> {
    let mut by_uid: BTreeMap<&str, Vec<&Commitment>> = BTreeMap::new();
    for note in &notes {
        if let Some(uid) = note.source_uid.as_deref() {
            by_uid.entry(uid).or_default().push(note);
        }
    }
    let mut dropped: BTreeSet<PathBuf> = BTreeSet::new();
    for (uid, mut group) in by_uid {
        if group.len() < 2 {
            continue;
        }
        group.sort_by(|a, b| a.id.cmp(&b.id).then(a.path.cmp(&b.path)));
        let ignored: Vec<String> = group[1..]
            .iter()
            .map(|n| n.path.display().to_string())
            .collect();
        warnings.push(format!(
            "duplicate source_uid {uid}: kept {}, ignored {}",
            group[0].path.display(),
            ignored.join(", ")
        ));
        dropped.extend(group[1..].iter().map(|n| n.path.clone()));
    }
    notes
        .into_iter()
        .filter(|n| !dropped.contains(&n.path))
        .collect()
}

/// §2.4: several confirmed planning-day notes → the lowest `id` wins, the rest in one warning.
fn pick_planning_day(mut notes: Vec<Read>, warnings: &mut Vec<String>) -> Option<Read> {
    let id = |r: &Read| field_text(&r.meta, "id").unwrap_or_default();
    notes.sort_by(|a, b| id(a).cmp(&id(b)).then(a.rel.cmp(&b.rel)));
    let mut notes = notes.into_iter();
    let winner = notes.next()?;
    let rest: Vec<String> = notes.map(|r| r.rel).collect();
    if !rest.is_empty() {
        warnings.push(format!(
            "several planning-day notes: kept {}, ignored {}",
            winner.rel,
            rest.join(", ")
        ));
    }
    Some(winner)
}

/// The planning day's note and its per-weekday window (§2.4, §6.3). An invalid entry warns with
/// §6.3's exact text and leaves its weekdays on the template; a weekday already listed by an
/// earlier entry is skipped with a warning.
fn planning_day(
    read: &Read,
    warnings: &mut Vec<String>,
) -> (Commitment, [Option<(Time, Time)>; 7]) {
    let Read { rel, stem, meta } = read;
    let mut window: [Option<(Time, Time)>; 7] = [None; 7];
    let mut meets = Vec::new();
    if let Some(Value::Sequence(entries)) = get(meta, "window") {
        for entry in entries {
            match parse_entry(entry) {
                Ok(mut meet) => {
                    let span = (meet.start, meet.end);
                    meet.days.retain(|day| {
                        let Some(idx) = DAY_KEYS.iter().position(|k| k == day) else {
                            return false;
                        };
                        if window[idx].is_some() {
                            warnings.push(format!(
                                "planning day {day}: listed twice; later entry skipped"
                            ));
                            return false;
                        }
                        window[idx] = Some(span);
                        true
                    });
                    if !meet.days.is_empty() {
                        meets.push(meet);
                    }
                }
                Err((days, start, end)) => warnings.push(format!(
                    "planning day {days}: {start}–{end} ignored; using week_template"
                )),
            }
        }
    }
    let note = Commitment {
        id: field_text(meta, "id").unwrap_or_default(),
        path: PathBuf::from(rel),
        kind: PLANNING_DAY.to_string(),
        level: Level::Optional,
        title: field_text(meta, "title").unwrap_or_else(|| stem.clone()),
        course: None,
        meets,
        where_: None,
        from: None,
        until: None,
        source_uid: field_text(meta, "source_uid"),
    };
    (note, window)
}

/// One field of a new note's frontmatter (plan review I3, Global Constraint 23).
pub enum Field {
    /// A scalar (never a `Node::Seq`/`Node::Map`): written by `yamlemit::safe_dump_block`.
    Scalar(Node),
    /// A collection: written on one line by `write::to_literal`, the flow path `judgment:` uses.
    Flow(serde_json::Value),
}

/// The one builder for the frontmatter of every note and card this model writes: one line per
/// field, in the order given. `safe_dump_block` of a whole map would put a nested map or sequence
/// on several lines, and nothing downstream would stop it reaching disk.
///
/// **Refuses** (P11, P6 review m4) a `Scalar` holding a collection, and any field whose emitted
/// text is not exactly one line: a string with a line break in it (a calendar title is verbatim)
/// is written by PyYAML's emitter as a quoted scalar spread over several lines, in block and in
/// flow style alike. Callers make their values one line first ([`single_line`]); this is the
/// guard that nothing multi-line reaches a note.
pub(crate) fn front_matter(fields: &[(&str, Field)]) -> Result<String, String> {
    let mut out = String::new();
    for (key, field) in fields {
        let emitted = match field {
            Field::Scalar(Node::Seq(_) | Node::Map(_)) => {
                return Err(format!("{key}: a collection written as a scalar"));
            }
            Field::Scalar(node) => {
                yamlemit::safe_dump_block(&Node::map(vec![(key, node.clone())]))
            }
            Field::Flow(value) => {
                let literal = crate::write::to_literal(&crate::yaml::from_json(value));
                format!("{key}: {literal}\n")
            }
        };
        if pystr::splitlines(&emitted).len() != 1 {
            return Err(format!("{key}: would span more than one frontmatter line"));
        }
        out.push_str(&emitted);
    }
    Ok(out)
}

/// `text` with every line break (`str.splitlines`' set: LF, CR, U+2028, U+0085, ...) replaced
/// by one space and nothing else changed — the one change a verbatim title needs to be written
/// on one frontmatter line.
pub(crate) fn single_line(text: &str) -> String {
    pystr::splitlines(text).join(" ")
}

// =============================================================================================
// The vault's code table (§3.4, C3, R7) and the classifier (§3.4; C5), P7.
// =============================================================================================

/// A string's leading course code — `CS 100`, `cs-100`, `CS100` — at the very start of the
/// string, then a word boundary. Used both to reduce a `courses/*.md` field to a code (§3.4's
/// code table) and to find the code a series title starts with (rule 3).
static LEADING_CODE_RE: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"^[A-Za-z]{2,8}[ ._-]?[0-9]{1,4}[A-Za-z]?\b").unwrap());

/// A season+year term prefix or a bare numeric term id (fix round 1, I2): `FA26`, `SP27`, `SU26`,
/// `WI27`, `FALL2026`, `Fall 2026`, `Spring 2027`, `202640`. `to_code` skips a leading-code match
/// that is really one of these, so an LMS name like `FA26-CS-100-001` reads its course code from
/// the D4 fallback instead of reading the term as the code.
static TERM_RE: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(
        r"(?i)^(fa|sp|su|wi)[ ._-]?[0-9]{2}$|^(fall|spring|summer|winter)[ ._-]?[0-9]{4}$|^[0-9]{6}$",
    )
    .unwrap()
});

/// Rule 0's own-work-time words and phrases, whole-word (`\b`), case-insensitive, matched against
/// the normalised title (`normalize_words`) so "any spacing" phrases like `work   on` collapse to
/// one space first. §3.4 ruling I3 (fix round 1): "Work session", "Work block", "Work time",
/// "Work on …" and "Working on …" are the student's own study time, never kind `work` and never
/// proposed — rule 4's bare "starts with work" is reserved for a job shift. `review(ing)?` (fix
/// round 2, M4) so "Reviewing" is caught the same way "Studying" already is. `work ?out`/`working
/// out` (fix round 2): "Work out", "Workout" and "Working out" are exercise, not a job shift —
/// applying I3's "when unsure, not `work`" the conservative way, as a never-proposed phrase rather
/// than guessing it is a routine the lexicon does not name.
static NOT_PROPOSED_RE: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(
        r"(?i)\b(studying|study|homework|hw[0-9]*|review(ing)?|prep|tutoring|focus|work(ing)? on|work session|work block|work time|work ?out|working out)\b",
    )
    .unwrap()
});

/// Rule 2's `office hours?`, case-insensitive.
static OFFICE_HOURS_RE: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"(?i)\boffice hours?\b").unwrap());

/// Rule 2's `OH` token, capitals only — case-sensitive on purpose, so `oh` never matches.
static CAPITAL_OH_RE: LazyLock<Regex> = LazyLock::new(|| Regex::new(r"\bOH\b").unwrap());

/// Rule 4's leading `work`.
static WORK_START_RE: LazyLock<Regex> = LazyLock::new(|| Regex::new(r"(?i)^work\b").unwrap());

/// Rule 4's `shift`, anywhere.
static SHIFT_RE: LazyLock<Regex> = LazyLock::new(|| Regex::new(r"(?i)\bshift\b").unwrap());

/// Rule 5's club words, anywhere.
static CLUB_RE: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"(?i)\b(club|society|team|practice|rehearsal|chapter)\b").unwrap()
});

/// One run of letters or digits — how a remainder or a course name is split into words.
static TOKEN_RE: LazyLock<Regex> = LazyLock::new(|| Regex::new(r"[A-Za-z0-9]+").unwrap());

/// Rule 3's section words. `lab`/`laboratory` yield kind `lab`; every other one yields `class`.
const SECTION_WORDS: [&str; 13] = [
    "lab",
    "laboratory",
    "lecture",
    "lec",
    "recitation",
    "rec",
    "discussion",
    "disc",
    "seminar",
    "section",
    "sec",
    "studio",
    "class",
];

/// Rule 1's whole-title words, lower-cased, matched against the normalised title
/// (`normalize_words`) so "Wake-up", "Wake  Up" and "Wakeup" all reach `wake up`/`wakeup`, and
/// "Bed time" reaches `bedtime`'s own spaced form (fix round 1, M1).
const WAKE_WORDS: [&str; 5] = ["wake", "wake up", "get up", "alarm", "wakeup"];
const BED_WORDS: [&str; 6] = [
    "bed",
    "bedtime",
    "go to bed",
    "sleep",
    "lights out",
    "bed time",
];

/// A compact code: capitals, separators removed (`CS 100`, `cs-100`, `CS100` → `CS100`).
fn compact(code: &str) -> String {
    code.chars()
        .filter(|c| c.is_ascii_alphanumeric())
        .map(|c| c.to_ascii_uppercase())
        .collect()
}

/// c1c D4's `course_code_in_name` (`app/src/scaffold.rs`, commit 7ef2fc2), reimplemented here
/// since the engine never depends on the app crate (workspace rule; `dependency_boundary.rs`):
/// 2–4 uppercase letters not themselves preceded by an uppercase letter, a required `-` or
/// whitespace separator, exactly three digits, an optional one-letter suffix, and a
/// non-alphanumeric boundary. The fallback reading for an LMS name like `202640-BUI-100-101`
/// (§3.4), read **anywhere** in the string rather than only at its start.
fn d4_fallback(text: &str) -> Option<String> {
    let chars: Vec<char> = text.chars().collect();
    for start in 0..chars.len() {
        if start > 0 && chars[start - 1].is_ascii_uppercase() {
            continue;
        }
        let letters: String = chars[start..]
            .iter()
            .take_while(|c| c.is_ascii_uppercase())
            .collect();
        if letters.len() < 2 || letters.len() > 4 {
            continue;
        }
        let mut i = start + letters.len();
        if chars.get(i).map(|c| *c == '-' || c.is_whitespace()) != Some(true) {
            continue;
        }
        i += 1;
        let digits: String = chars[i..].iter().take_while(|c| c.is_ascii_digit()).collect();
        if digits.len() != 3 {
            continue;
        }
        i += digits.len();
        let suffix = match chars.get(i) {
            Some(c) if c.is_ascii_alphabetic() => {
                i += 1;
                c.to_ascii_uppercase().to_string()
            }
            _ => String::new(),
        };
        if chars.get(i).map(|c| c.is_ascii_alphanumeric()) == Some(true) {
            continue;
        }
        return Some(format!("{letters} {digits}{suffix}"));
    }
    None
}

/// A string reduced to one compact code (§3.4): the leading-code reading first, then c1c D4's
/// reading anywhere in the string as a fallback for an LMS name. A leading match that is itself a
/// term prefix (`TERM_RE`, fix round 1 I2) is not a code, so that case falls through to D4 too —
/// `FA26-CS-100-001` reads `CS 100`, not `FA26` — **unless** `exempt` already names that exact
/// compact code (fix round 2): a real two-digit department code like `FA 10` is never skipped when
/// the vault's own `code:` field says that is the code.
fn to_code_exempt(text: &str, exempt: &BTreeSet<String>) -> Option<String> {
    let text = text.trim();
    if text.is_empty() {
        return None;
    }
    if let Some(m) = LEADING_CODE_RE.find(text) {
        let code = compact(m.as_str());
        if !TERM_RE.is_match(m.as_str()) || exempt.contains(&code) {
            return Some(code);
        }
    }
    d4_fallback(text).map(|s| compact(&s))
}

/// `to_code_exempt` with no exemptions — every caller but `code_table` itself; kept for the tests
/// that exercise the plain reading directly.
#[cfg_attr(
    not(test),
    expect(dead_code, reason = "code_table calls to_code_exempt directly; tests call this")
)]
fn to_code(text: &str) -> Option<String> {
    to_code_exempt(text, &BTreeSet::new())
}

/// The code table under construction: a code's first claimant is remembered even after the code
/// is dropped, so a third source repeating either side never re-warns (one warning per code).
#[derive(Default)]
struct Table {
    codes: BTreeMap<String, String>,
    owner: BTreeMap<String, String>,
    dropped: BTreeSet<String>,
    warnings: Vec<String>,
}

impl Table {
    /// First mapping wins; a code a second, different slug claims is dropped (removed, not kept
    /// for either side) with one warning — when in doubt, do not propose (§3.4).
    fn claim(&mut self, code: String, slug: &str) {
        match self.owner.get(&code).cloned() {
            None => {
                self.owner.insert(code.clone(), slug.to_string());
                self.codes.insert(code, slug.to_string());
            }
            Some(existing) if existing == slug => {}
            Some(existing) => {
                if self.dropped.insert(code.clone()) {
                    self.warnings.push(format!(
                        "code {code}: claimed by both {existing} and {slug}; dropped"
                    ));
                }
                self.codes.remove(&code);
            }
        }
    }
}

/// The vault's compact-code → course-slug table (§3.4, C3, R7), read, never fetched, plus one
/// warning per code two different courses claim. In order, first mapping wins: every
/// `courses/*.md` note in file-name order (its `code`, `title` and `name` fields, then its own
/// slug), then `config/ingest.yaml`'s `course_map` keys in file order.
pub fn code_table(vault: &Path) -> (BTreeMap<String, String>, Vec<String>) {
    let mut table = Table::default();

    let mut courses: Vec<(String, Mapping)> = Vec::new();
    let dir = vault.join("courses");
    if let Ok(entries) = std::fs::read_dir(&dir) {
        let mut names: Vec<String> = entries
            .filter_map(|e| e.ok())
            .filter(|e| e.path().is_file())
            .filter_map(|e| e.file_name().into_string().ok())
            .filter(|name| name.ends_with(".md"))
            .collect();
        names.sort();
        for name in names {
            let Ok(text) = pystr::read_text(&dir.join(&name)) else {
                continue;
            };
            let Ok((meta, _)) = split_frontmatter(&text) else {
                continue;
            };
            let stem = name.trim_end_matches(".md").to_string();
            let slug = field_text(&meta, "slug")
                .filter(|s| !s.is_empty())
                .unwrap_or(stem);
            courses.push((slug, meta));
        }
    }

    // Fix round 2: a course's own explicit `code:` field is the vault's own ground truth. The
    // term-prefix skip (I2) exists for LMS junk like `FA26-CS-100-001`, never for a code the
    // student (or the wizard) wrote down deliberately, so it never applies to a compact code this
    // set already names — gathered up front, before any claim, so a course's own `code:` field
    // exempts every field's reading of it, including its own.
    let mut exempt: BTreeSet<String> = BTreeSet::new();
    for (_, meta) in &courses {
        if let Some(raw) = field_text(meta, "code") {
            let c = compact(&raw);
            if !c.is_empty() {
                exempt.insert(c);
            }
        }
    }

    for (slug, meta) in &courses {
        for field in ["code", "title", "name"] {
            if let Some(raw) = field_text(meta, field) {
                if let Some(code) = to_code_exempt(&raw, &exempt) {
                    table.claim(code, slug);
                }
            }
        }
        if let Some(code) = to_code_exempt(slug, &exempt) {
            table.claim(code, slug);
        }
    }

    if let Ok(text) = pystr::read_text(&vault.join("config").join("ingest.yaml")) {
        if let Ok(cfg) = serde_yaml_ng::from_str::<Value>(&text) {
            if let Some(mapping) = cfg.get("course_map").and_then(|v| v.as_mapping()) {
                for (k, v) in mapping.iter() {
                    if let (Some(key), Some(slug)) = (k.as_str(), v.as_str()) {
                        if let Some(code) = to_code_exempt(key, &exempt) {
                            table.claim(code, slug);
                        }
                    }
                }
            }
        }
    }

    (table.codes, table.warnings)
}

/// Each known slug's own name words, lower-cased, from its `courses/*.md` note's `title` and
/// `name` fields — rule 3's "words that all appear in that course note's own name/title"
/// ("CS 100 – Intro to Computer Science"). Read the same way `code_table` reads `courses/*.md`.
pub fn course_words(vault: &Path) -> BTreeMap<String, BTreeSet<String>> {
    let mut out: BTreeMap<String, BTreeSet<String>> = BTreeMap::new();
    let dir = vault.join("courses");
    let Ok(entries) = std::fs::read_dir(&dir) else {
        return out;
    };
    let mut names: Vec<String> = entries
        .filter_map(|e| e.ok())
        .filter(|e| e.path().is_file())
        .filter_map(|e| e.file_name().into_string().ok())
        .filter(|name| name.ends_with(".md"))
        .collect();
    names.sort();
    for name in names {
        let Ok(text) = pystr::read_text(&dir.join(&name)) else {
            continue;
        };
        let Ok((meta, _)) = split_frontmatter(&text) else {
            continue;
        };
        let stem = name.trim_end_matches(".md").to_string();
        let slug = field_text(&meta, "slug")
            .filter(|s| !s.is_empty())
            .unwrap_or(stem);
        let words = out.entry(slug).or_default();
        for field in ["title", "name"] {
            if let Some(raw) = field_text(&meta, field) {
                for m in TOKEN_RE.find_iter(&raw) {
                    words.insert(m.as_str().to_ascii_lowercase());
                }
            }
        }
    }
    out
}

/// What `classify`'s rule 3 needs from the vault's courses (§3.4): the compact code table
/// (`code_table`, unchanged, its own contract) plus each slug's own name words. *Decision beyond
/// the brief:* rule 3's "words that all appear in that course note's own name/title" clause needs
/// the note's own text, which `code_table`'s bare `(BTreeMap<String, String>, Vec<String>)`
/// return does not carry — so `classify` takes this small bundle (still named `codes`) rather
/// than `code_table`'s return directly.
#[derive(Debug, Clone, Default)]
pub struct Codes {
    pub table: BTreeMap<String, String>,
    pub names: BTreeMap<String, BTreeSet<String>>,
}

impl Codes {
    /// `code_table` and `course_words` together, plus `code_table`'s warnings.
    pub fn load(vault: &Path) -> (Codes, Vec<String>) {
        let (table, warnings) = code_table(vault);
        let names = course_words(vault);
        (Codes { table, names }, warnings)
    }
}

/// One instance of a series (§3.2.1): `start`/`end` are `None` together for an all-day event.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Instance {
    pub date: Date,
    pub start: Option<Time>,
    pub end: Option<Time>,
}

/// A series' recurrence rule (§3.2.2), from the master's `RRULE`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Rule {
    pub freq: String,
    pub interval: u32,
    pub until: Option<Date>,
    pub count: Option<u32>,
}

/// One source reduced to §3.2's one record. Plain data; P8 fills it from Google/ICS, this module
/// only reads it. `rdate` and `unsupported` are precomputed by whoever built the record (P8, from
/// `calfeed`'s recurrence helpers) — `classify` only reads them, so it stays a pure function that
/// touches no calendar-parsing code.
#[derive(Debug, Clone, PartialEq)]
pub struct Series {
    pub source_uid: String,
    pub calendar: String,
    pub title: String,
    pub where_: Option<String>,
    /// `None` (ICS, which carries no such field) or `Some("default")` are eligible; any other
    /// value (`fromGmail`, `outOfOffice`, `focusTime`, `workingLocation`, …) is not.
    pub event_type: Option<String>,
    pub rule: Rule,
    pub has_master: bool,
    /// The master carries an `RDATE`: an irregular series, never eligible.
    pub rdate: bool,
    /// `calfeed`'s recurrence helpers refused the rule (`UnsupportedRule`): never eligible.
    pub unsupported: bool,
    pub instances: Vec<Instance>,
    pub meets: Vec<Meet>,
    pub first: Option<Date>,
    pub until: Option<Date>,
    pub last_seen: Option<Date>,
}

/// What a series classifies as (§3.4). A `Kind`'s `course` is only ever set for `class`, `lab`
/// and `office-hours` (rule 2's course is optional even then). A midnight-crossing `sleep` series
/// is `Routine { wake: true, bed: true }` — both sides at once (rule 1).
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Class {
    Kind { kind: String, course: Option<String> },
    Routine { wake: bool, bed: bool },
}

/// Drops emoji and other symbol characters (fix round 2, M1): "Bedtime 🛏" reads as "Bedtime" for
/// lexicon matching. Keeps letters, digits and marks from any script (`is_alphanumeric`),
/// whitespace, and the punctuation §3.4's rules already give meaning to (word separators, code
/// separators, quotes) — everything else (pictographs, dingbats, other symbol characters) is
/// dropped outright, not just trimmed from the ends, since an emoji can sit mid-title too.
fn strip_symbols(title: &str) -> String {
    title
        .chars()
        .filter(|c| {
            c.is_alphanumeric()
                || c.is_whitespace()
                || matches!(
                    c,
                    '-' | '_'
                        | '.'
                        | ':'
                        | '('
                        | ')'
                        | '\''
                        | '–'
                        | '—'
                        | '/'
                        | '['
                        | ']'
                        | '\u{2018}'
                        | '\u{2019}'
                        | '\u{201c}'
                        | '\u{201d}'
                )
        })
        .collect()
}

/// The title with surrounding whitespace and punctuation trimmed (§3.4: "matching is on the
/// title trimmed of surrounding punctuation").
fn trimmed_title(title: &str) -> String {
    title
        .trim_matches(|c: char| {
            c.is_whitespace() || c.is_ascii_punctuation() || matches!(c, '–' | '—' | '\u{2018}' | '\u{2019}' | '\u{201c}' | '\u{201d}')
        })
        .to_string()
}

/// Collapses runs of whitespace, `-` and `_` to one space (fix round 1, M1): "Wake-up" and
/// "Wake  Up" both read as "wake up" for rules 0 and 1. A title with no separator at all
/// ("Wakeup") is untouched, which is why `WAKE_WORDS`/`BED_WORDS` also carry the glued spelling.
fn normalize_words(title: &str) -> String {
    let mut out = String::with_capacity(title.len());
    let mut at_space = true;
    for c in title.chars() {
        if c.is_whitespace() || c == '-' || c == '_' {
            if !at_space {
                out.push(' ');
                at_space = true;
            }
        } else {
            out.push(c);
            at_space = false;
        }
    }
    out.trim_end().to_string()
}

/// Rule 1's side(s) for a trimmed, lower-cased whole title — `None` when it names no routine.
fn routine_side(lower_trimmed: &str) -> Option<(bool, bool)> {
    if WAKE_WORDS.contains(&lower_trimmed) {
        return Some((true, false));
    }
    if BED_WORDS.contains(&lower_trimmed) {
        return Some((false, true));
    }
    None
}

/// The trimmed, normalised title is exactly `sleep` (case-insensitive) — rule 1's one
/// midnight-crossing exception.
fn is_sleep(title: &str) -> bool {
    normalize_words(&trimmed_title(&strip_symbols(title))).eq_ignore_ascii_case("sleep")
}

/// Any instance's end time is strictly before its start time — a nightly wraparound (§3.4's
/// midnight bullet; `Instance` carries no explicit "next day" flag, so a start/end pair on the
/// *same* date with `end < start` is how the record says "crosses midnight"). A **zero-length**
/// event (`end == start`, an alarm or a marker) never counts (fix round 1, I1): it is a point in
/// time, not a span that wraps around one.
fn crosses_midnight(series: &Series) -> bool {
    series
        .instances
        .iter()
        .any(|i| matches!((i.start, i.end), (Some(s), Some(e)) if e < s))
}

/// One "kept triple"'s instance dates (§3.2.3, §3.4): fewer than two is not "kept" at all and
/// passes vacuously; otherwise every consecutive gap must be a positive multiple of 7 days, with
/// at least one gap of exactly 7 (a holiday gap is allowed; an every-other-week series is not).
fn spacing_ok(mut dates: Vec<Date>) -> bool {
    if dates.len() < 2 {
        return true;
    }
    dates.sort();
    let mut saw_seven = false;
    for pair in dates.windows(2) {
        let gap = pair[1]
            .since(pair[0])
            .map(|s| i64::from(s.get_days()))
            .unwrap_or(0);
        if gap <= 0 || gap % 7 != 0 {
            return false;
        }
        if gap == 7 {
            saw_seven = true;
        }
    }
    saw_seven
}

/// §3.4's eligibility bullets, in order. `series.until`'s "not before today" is approximated with
/// `last_seen` — the calendar's own read date (§3.2.6), **never "today" itself** (fix round 1,
/// M5): `classify` takes no `today` of its own (decision 2's signature is exactly `series, codes,
/// planning`), and a calendar not read this run keeps an older `last_seen`, so this is the
/// freshest date the record carries, not a guarantee. P9 should either filter on the real today
/// before calling `classify`, or `classify` should grow a `today` argument. A series with no
/// `last_seen` (never read at all) skips the check rather than guess.
fn eligible(series: &Series) -> bool {
    match series.event_type.as_deref() {
        None | Some("default") => {}
        Some(_) => return false,
    }
    if !series.has_master
        || series.rule.freq != "WEEKLY"
        || series.rule.interval != 1
        || series.rdate
        || series.unsupported
    {
        return false;
    }
    if series
        .instances
        .iter()
        .any(|i| i.start.is_none() || i.end.is_none())
    {
        return false;
    }
    if crosses_midnight(series) && !is_sleep(&series.title) {
        return false;
    }
    if series.instances.len() < 2 {
        return false;
    }
    let mut groups: BTreeMap<(DayKey, Time, Time), Vec<Date>> = BTreeMap::new();
    for i in &series.instances {
        let (s, e) = (i.start.unwrap(), i.end.unwrap());
        groups
            .entry((crate::planning::day_key(i.date), s, e))
            .or_default()
            .push(i.date);
    }
    for dates in groups.into_values() {
        if !spacing_ok(dates) {
            return false;
        }
    }
    if series.meets.is_empty() {
        return false;
    }
    if let (Some(until), Some(read_date)) = (series.until, series.last_seen) {
        if until < read_date {
            return false;
        }
    }
    true
}

/// The slug a title starts with, per the vault's code table, and everything after it (rule 3's
/// leading-code test; rule 2 reuses it just for the slug). `None` when the title does not start
/// with a known code, or the character right after it is not one of rule 3's separators (space,
/// `-`, `–`, `:`, `(`, `.` — the last for I4's "CS 100.001"; `—`, `/`, `[`, `]` added fix round 2,
/// M7, so "CS 100 — Lab", "CS 100 / Lab" and "[CS 100] Lecture" (`trimmed_title` already strips
/// the leading `[`, leaving a stray `]` right after the code) all reach rule 3 too).
fn class_course(title: &str, codes: &BTreeMap<String, String>) -> Option<(String, String)> {
    let m = LEADING_CODE_RE.find(title)?;
    let slug = codes.get(&compact(m.as_str()))?.clone();
    let rest = &title[m.end()..];
    if rest.is_empty() {
        return Some((slug, String::new()));
    }
    let sep = rest.chars().next().unwrap();
    if !matches!(sep, ' ' | '-' | '–' | '—' | ':' | '(' | '.' | '/' | '[' | ']') {
        return None;
    }
    Some((slug, rest[sep.len_utf8()..].to_string()))
}

/// M3's own leading-code match, restricted to the case a `codes` lookup already failed: the
/// vault's own compact code with one trailing letter — the scaffold seeds one course note for a
/// class and its lab together, so the table carries only the bare code and `"CS 100L"` needs its
/// own reading. `L`/`l` reads as a lab; only an exact, whole-title match counts, matching M3's own
/// example ("CS 100L" alone, not "CS 100L Discussion").
fn bare_letter_suffix_lab(title: &str, codes: &BTreeMap<String, String>) -> Option<String> {
    let m = LEADING_CODE_RE.find(title)?;
    if !title[m.end()..].is_empty() {
        return None;
    }
    let matched = m.as_str();
    let last = matched.chars().next_back()?;
    if !last.eq_ignore_ascii_case(&'l') {
        return None;
    }
    let full = compact(matched);
    let bare = &full[..full.len() - 1];
    codes.get(bare).cloned()
}

/// A section word's own kind: `lab`/`laboratory` yield `lab`, every other one yields `class`.
fn section_word_kind(word: &str) -> &'static str {
    if word == "lab" || word == "laboratory" {
        "lab"
    } else {
        "class"
    }
}

/// A second token after a section word is a designator, not a second word (the review's spec-gap
/// fix): it holds a digit, or it is a single character (`01`, `2`, `001L`, `A`) — "CS 100 Lab
/// Hours" and "CS 100 Class Party" are not a section number and so are not `class`/`lab`.
fn is_number_ish(tok: &str) -> bool {
    tok.chars().any(|c| c.is_ascii_digit()) || tok.chars().count() == 1
}

/// I4's bare section designator, with no leading section word — purely numeric (`001`, `1`), a
/// single letter (`A`), or a known designator letter followed by digits (`l`/lab, `r`/recitation,
/// `d`/discussion — `L01`, `R01`, `D01`). Fix round 2: this is deliberately narrower than "any
/// short alphanumeric token with a digit" — `PS1`, `HW1` and `1on1` are none of these three shapes
/// and so are never a designator, unlike the old `is_section_number`.
fn designator_kind(tok: &str) -> Option<&'static str> {
    if !tok.is_empty() && tok.len() <= 4 && tok.chars().all(|c| c.is_ascii_digit()) {
        return Some("class");
    }
    if tok.chars().count() == 1 && tok.chars().next().is_some_and(|c| c.is_ascii_alphabetic()) {
        return Some("class");
    }
    let mut chars = tok.chars();
    let first = chars.next()?;
    let rest = &tok[first.len_utf8()..];
    if matches!(first, 'l' | 'r' | 'd') && !rest.is_empty() && rest.chars().all(|c| c.is_ascii_digit())
    {
        return Some(if first == 'l' { "lab" } else { "class" });
    }
    None
}

/// Rule 3's remainder test (§3.4, the review's spec-gap fix, and ruling I4): empty; a section word
/// alone or with one number-ish token ("CS 100 Lab 01", "CS 100 Sec 1" — not "CS 100 Lab Hours");
/// a bare section designator with no leading word ("-001", "001 LEC", "L01" — I4); or every
/// remainder word also in the course's own name/title. `None` means rule 3 does not match at all
/// ("CS 100 TA hours" falls through to rules 4–6).
fn section_kind(
    remainder: &str,
    slug: &str,
    course_words: &BTreeMap<String, BTreeSet<String>>,
) -> Option<&'static str> {
    let tokens: Vec<String> = TOKEN_RE
        .find_iter(remainder)
        .map(|m| m.as_str().to_ascii_lowercase())
        .collect();
    if tokens.is_empty() {
        return Some("class");
    }

    if SECTION_WORDS.contains(&tokens[0].as_str())
        && (tokens.len() == 1 || (tokens.len() == 2 && is_number_ish(&tokens[1])))
    {
        return Some(section_word_kind(&tokens[0]));
    }

    if tokens.len() <= 2 {
        if let Some(kind) = designator_kind(&tokens[0]) {
            if tokens.len() == 1 {
                return Some(kind);
            }
            if SECTION_WORDS.contains(&tokens[1].as_str()) {
                return Some(section_word_kind(&tokens[1]));
            }
            return None;
        }
    }

    if let Some(words) = course_words.get(slug) {
        if tokens.iter().all(|t| words.contains(t)) {
            return Some("class");
        }
    }
    None
}

/// Pure: no clock, no network, no model (`rank` never calls a model — Knowlu spec decision 11;
/// this is the same discipline one layer up). Eligibility first (§3.4's bullets), then rules 0–6
/// in order, first match wins, on the title trimmed of surrounding punctuation (C5).
pub fn classify(series: &Series, codes: &Codes, planning: &[String]) -> Option<Class> {
    if !eligible(series) {
        return None;
    }
    // M1: symbols (emoji, dingbats) dropped, then trimmed, then normalised — internal
    // whitespace/`-`/`_` collapsed to one space — so every rule below sees "Bedtime 🛏" as
    // "Bedtime" and "Wake-up"/"Wake  Up" the same way it sees "Wake up".
    let title = normalize_words(&trimmed_title(&strip_symbols(&series.title)));

    // Rule 0: not proposed at all. M2: the planning name goes through the same pipeline as the
    // title, so a planning entry "Gym." matches a series titled "Gym." too.
    if NOT_PROPOSED_RE.is_match(&title)
        || planning.iter().any(|name| {
            normalize_words(&trimmed_title(&strip_symbols(name))).eq_ignore_ascii_case(&title)
        })
    {
        return None;
    }

    // Rule 1: routine. I1: a zero-length event (`crosses_midnight` now `end < start`, not
    // `<=`) is a point marker at its start, so only a genuine wraparound forces both sides.
    let lower = title.to_ascii_lowercase();
    if let Some((mut wake, mut bed)) = routine_side(&lower) {
        if crosses_midnight(series) {
            wake = true;
            bed = true;
        }
        return Some(Class::Routine { wake, bed });
    }

    // Rule 2: office hours.
    if OFFICE_HOURS_RE.is_match(&title) || CAPITAL_OH_RE.is_match(&title) {
        let course = class_course(&title, &codes.table).map(|(slug, _)| slug);
        return Some(Class::Kind {
            kind: "office-hours".to_string(),
            course,
        });
    }

    // Rule 3: course.
    if let Some((slug, remainder)) = class_course(&title, &codes.table) {
        if let Some(kind) = section_kind(&remainder, &slug, &codes.names) {
            return Some(Class::Kind {
                kind: kind.to_string(),
                course: Some(slug),
            });
        }
    } else if let Some(slug) = bare_letter_suffix_lab(&title, &codes.table) {
        // M3: "CS 100L" is a lab of CS 100 when the table only knows the bare code.
        return Some(Class::Kind {
            kind: "lab".to_string(),
            course: Some(slug),
        });
    }

    // Rule 4: work.
    if WORK_START_RE.is_match(&title) || SHIFT_RE.is_match(&title) {
        return Some(Class::Kind {
            kind: "work".to_string(),
            course: None,
        });
    }

    // Rule 5: club.
    if CLUB_RE.is_match(&title) {
        return Some(Class::Kind {
            kind: "club".to_string(),
            course: None,
        });
    }

    // Rule 6: otherwise.
    Some(Class::Kind {
        kind: "meeting".to_string(),
        course: None,
    })
}

// ---------------------------------------------------------------------------------------------
// P8 — normalising a series (§3.2) and the series file `state/calendar-series.json` (§3.3).
// ---------------------------------------------------------------------------------------------

/// The series file, vault-relative (§3.3). Generated state: not a note, never journaled, never
/// synced; it holds titles, a place-like `where` and times, and **never a description** (§9).
pub const SERIES_FILE: &str = "state/calendar-series.json";

/// The horizon a read covers: `[read date, read date + 28)` (§3.2.1).
const HORIZON_DAYS: i64 = 28;
/// A series its fresh calendar stopped returning — or a removed feed's — lasts this long (§3.3).
const UNSEEN_DAYS: i64 = 14;
/// An `ended` entry lasts this long after `dropped` (§3.3, plan review I1).
const ENDED_DAYS: i64 = 28;
/// §2.2's bounds, applied on the device on both routes (the function already cuts Google's
/// title to 200; `where` is cut to 80 on both).
const TITLE_MAX: usize = 200;
const WHERE_MAX: usize = 80;
/// Google's `eventType` is a short word (`default`, `focusTime`, …); anything longer is cut.
const EVENT_TYPE_MAX: usize = 40;

/// §3.2.5: a line carrying any of these (case-insensitive, anywhere) is a link or a way into a
/// meeting, never a place.
const NOT_A_PLACE: [&str; 4] = ["://", "www.", "meet.google", "meeting id"];

/// §3.2.5's single words, matched on **letter** boundaries (fix round 1, I1): "Pine Hall",
/// "Chapin Hall", "Spinning Studio" and "Steamship Rm" keep their `where`, while `PIN1234`,
/// `pin: 1234`, `pwd=` and `us02web.zoom.us` are still caught (a digit or punctuation is not a
/// letter, so `` — which would miss `PIN1234` — is not used).
static MEETING_WORD_RE: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"(?i)(?:^|[^a-z])(?:zoom|teams|webex|pwd|passcode|password|pin)(?:[^a-z]|$)").unwrap()
});

/// §3.2.5's "run of six or more digits", read so a meeting id or phone number written with
/// spaces or dashes between its groups ("555 123 456", "205-555-0100") is a run too.
static DIGIT_RUN_RE: LazyLock<Regex> = LazyLock::new(|| Regex::new(r"\d(?:[ \t-]?\d){5,}").unwrap());

/// One series the file dropped because its calendar was read fresh and it went unseen for
/// 14 days (§3.3, plan review I1) — never one a removed feed took with it, never a key another
/// calendar still holds. Kept 28 days, so §5.4's **ended** rule can still file its end card.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Ended {
    pub calendar: String,
    pub dropped: Date,
    /// The last instance date the file held for it; `None` when it held none.
    pub last_instance: Option<Date>,
    /// Its last-known `until`.
    pub until: Option<Date>,
}

/// `state/calendar-series.json`, read into plain data (§3.3).
#[derive(Debug, Clone, PartialEq, Default)]
pub struct SeriesFile {
    /// Each calendar key → the date it was last read fresh and complete.
    pub calendars: BTreeMap<String, Date>,
    /// `source_uid` → the series that ended (decision 6).
    pub ended: BTreeMap<String, Ended>,
    /// Sorted by `(source_uid, calendar)`; one key may be held under two calendars (I2), so a
    /// reader takes [`SeriesFile::by_key`], never this list.
    pub series: Vec<Series>,
}

/// The one precedence between two records of one key (I2): a `google:` calendar first, then the
/// lower calendar key. Smaller wins.
fn precedence(series: &Series) -> (bool, &str) {
    (!series.calendar.starts_with("google:"), series.calendar.as_str())
}

impl SeriesFile {
    /// One record per `source_uid`, by [`precedence`] (plan review I2): one key is one set of
    /// instances, one proposal and one card. A key is absent only when no calendar holds it.
    pub fn by_key(&self) -> BTreeMap<&str, &Series> {
        let mut out: BTreeMap<&str, &Series> = BTreeMap::new();
        for series in &self.series {
            match out.get(series.source_uid.as_str()) {
                Some(held) if precedence(held) <= precedence(series) => {}
                _ => {
                    out.insert(series.source_uid.as_str(), series);
                }
            }
        }
        out
    }

    /// P3's `instances` shape (`WeekCalendar::with_instances`), from [`SeriesFile::by_key`]: per
    /// key, its calendar's read date, that + 28 (exclusive), and its timed instances. An all-day
    /// instance has no span, so it is left out.
    ///
    /// A series its calendar's last fresh read did not return (`last_seen` before that read
    /// date) gets the horizon with **no** instances (fix round 1, M1; R21: inside the fresh
    /// horizon only the actual instances subtract, and that read found none). It stops blocking
    /// time at once; the record itself, with its old instances, stays in the file until it ages
    /// into `ended` (§3.3), so P12 can still name its last instance.
    pub fn instances_map(&self) -> BTreeMap<String, (Date, Date, Vec<(Date, Time, Time)>)> {
        self.by_key()
            .into_iter()
            .filter_map(|(key, series)| {
                let read = self.calendars.get(&series.calendar).copied().or(series.last_seen)?;
                let stale = series.last_seen.is_none_or(|seen| seen < read);
                let timed = match stale {
                    true => Vec::new(),
                    false => series
                        .instances
                        .iter()
                        .filter_map(|i| Some((i.date, i.start?, i.end?)))
                        .collect(),
                };
                Some((key.to_string(), (read, add_days(read, HORIZON_DAYS), timed)))
            })
            .collect()
    }
}

fn add_days(date: Date, days: i64) -> Date {
    jiff::Span::new()
        .try_days(days)
        .ok()
        .and_then(|span| date.checked_add(span).ok())
        .unwrap_or(date)
}

/// Whole days from `from` to `to` (negative when `to` is earlier).
fn days_since(from: Date, to: Date) -> i64 {
    from.until(to).map(|span| i64::from(span.get_days())).unwrap_or(0)
}

/// At most `max` characters (code points, never a split one), trailing space trimmed.
fn cut(text: &str, max: usize) -> String {
    text.chars().take(max).collect::<String>().trim_end().to_string()
}

/// A title as both routes store it: trimmed, `(untitled)` when empty (as the busy path titles
/// an event), cut to 200.
fn title_text(raw: &str) -> String {
    let title = raw.trim();
    cut(if title.is_empty() { "(untitled)" } else { title }, TITLE_MAX)
}

/// No link, no way into a meeting, no long digit run (§3.2.5).
fn safe_line(line: &str) -> bool {
    let lower = line.to_lowercase();
    !NOT_A_PLACE.iter().any(|word| lower.contains(word))
        && !MEETING_WORD_RE.is_match(line)
        && !DIGIT_RUN_RE.is_match(line)
}

/// §3.2.5: the location's first non-empty line (fix round 1, M3: `where` is one line), trimmed
/// and cut to 80 — unless it is itself a link or passcode (a meeting URL is a common Google
/// location), which is never stored (§9); else the first
/// description line that looks like a place (≤ 80 characters, [`safe_line`]); else none. The
/// description goes no further than this function (R4).
fn where_of(location: &str, description: &str) -> Option<String> {
    let location = location.lines().map(str::trim).find(|line| !line.is_empty()).unwrap_or("");
    if !location.is_empty() && safe_line(location) {
        return Some(cut(location, WHERE_MAX));
    }
    description
        .lines()
        .map(str::trim)
        .find(|line| !line.is_empty() && line.chars().count() <= WHERE_MAX && safe_line(line))
        .map(str::to_string)
}

/// §3.2.3: `(weekday, start, end)` triples seen at least twice, grouped by `(start, end)`, days
/// in `DAY_KEYS` order, entries by `(first day, start)`. All-day instances carry no triple.
fn meets_of(instances: &[Instance]) -> Vec<Meet> {
    let mut seen: BTreeMap<(usize, Time, Time), usize> = BTreeMap::new();
    for i in instances {
        if let (Some(start), Some(end)) = (i.start, i.end) {
            let day = i.date.weekday().to_monday_zero_offset() as usize;
            *seen.entry((day, start, end)).or_default() += 1;
        }
    }
    let mut groups: BTreeMap<(Time, Time), BTreeSet<usize>> = BTreeMap::new();
    for ((day, start, end), count) in seen {
        if count >= 2 {
            groups.entry((start, end)).or_default().insert(day);
        }
    }
    let mut out: Vec<(usize, Meet)> = groups
        .into_iter()
        .map(|((start, end), days)| {
            let first = *days.iter().next().expect("a group holds at least one day");
            (first, Meet { days: days.into_iter().map(|d| DAY_KEYS[d]).collect(), start, end })
        })
        .collect();
    out.sort_by(|a, b| (a.0, a.1.start, a.1.end).cmp(&(b.0, b.1.start, b.1.end)));
    out.into_iter().map(|(_, meet)| meet).collect()
}

/// A master's recurrence, read on the device (§3.2.2, §3.2.4).
struct RuleRead {
    rule: Rule,
    rdate: bool,
    unsupported: bool,
    until: Option<Date>,
}

/// Reads `RRULE`/`EXDATE`/`RDATE` lines (Google's `recurrence`, or `IcsSeries::rule_lines` —
/// one shape). `UNTIL` becomes a date in `tz` (`20261205T055959Z` is 4 December in Chicago);
/// `COUNT` is expanded from `first` with `calfeed`'s own helper to its last date — from
/// `first` at 00:00, not the master's start time, which is exact for a date-level `until` (a
/// `COUNT` over the helper's 1000-occurrence cap gives no `until`). A rule that
/// helper refuses, a second `RRULE`, an `EXRULE`, or no `RRULE` at all is `unsupported`.
fn read_rule(lines: &[String], first: Option<Date>, tz: &jiff::tz::TimeZone) -> RuleRead {
    let mut rrules: Vec<String> = Vec::new();
    let (mut rdate, mut exrule) = (false, false);
    for line in lines {
        match crate::ingest::parse_property(line.trim()) {
            Some((name, _, value)) if name == "RRULE" => rrules.push(value),
            Some((name, _, _)) if name == "RDATE" => rdate = true,
            Some((name, _, _)) if name == "EXRULE" => exrule = true,
            _ => {}
        }
    }
    let mut rule = Rule { freq: String::new(), interval: 1, until: None, count: None };
    let Some(raw) = rrules.first() else {
        return RuleRead { rule, rdate, unsupported: true, until: None };
    };
    // `calfeed::rrule_dict`'s reading: keys upper-cased, values as written.
    let mut dict: BTreeMap<String, String> = BTreeMap::new();
    for part in raw.split(';') {
        if let Some((key, value)) = part.split_once('=') {
            dict.insert(key.to_uppercase(), value.to_string());
        }
    }
    let mut unsupported = rrules.len() > 1 || exrule;
    let field = |key: &str| dict.get(key).map(|v| v.trim()).filter(|v| !v.is_empty());
    rule.freq = field("FREQ").map(str::to_uppercase).unwrap_or_default();
    if let Some(raw) = field("INTERVAL") {
        match raw.parse::<u32>() {
            Ok(n) if n > 0 => rule.interval = n,
            _ => unsupported = true,
        }
    }
    if let Some(raw) = field("UNTIL") {
        match crate::ingest::parse_dt(raw, &BTreeMap::new(), tz) {
            Some(crate::ingest::Due::Date(d)) => rule.until = Some(d),
            Some(crate::ingest::Due::DateTime(dt)) => rule.until = Some(dt.date()),
            None => unsupported = true,
        }
    }
    if let Some(raw) = field("COUNT") {
        match raw.parse::<u32>() {
            Ok(n) => rule.count = Some(n),
            Err(_) => unsupported = true,
        }
    }
    let mut until = rule.until;
    if let Some(first) = first {
        match crate::calfeed::occurrence_starts(first.to_datetime(Time::midnight()), &dict, tz) {
            Ok(starts) => {
                if let (None, Some(count)) = (until, rule.count) {
                    // Only a full expansion names the last date (the helper stops at 1000).
                    if starts.len() == count as usize {
                        until = starts.last().map(|s| s.date());
                    }
                }
            }
            Err(_) => unsupported = true,
        }
    }
    RuleRead { rule, rdate, unsupported, until }
}

/// A Google instance time (`…Z`) or `first` (`…-05:00`, or a bare date) as a local date-time in
/// `tz`, to the minute.
fn google_time(raw: &str, tz: &jiff::tz::TimeZone) -> Option<jiff::civil::DateTime> {
    let local = match raw.parse::<jiff::Timestamp>() {
        Ok(ts) => ts.to_zoned(tz.clone()).datetime(),
        Err(_) => raw.parse::<Date>().ok()?.to_datetime(Time::midnight()),
    };
    let minute = Time::new(local.hour(), local.minute(), 0, 0).ok()?;
    Some(local.date().to_datetime(minute))
}

/// One Google item → a series, or why it cannot be read. `Ok(None)`: read, but no instance falls
/// in `[today, today + 28)` — not returned, as on the ICS route.
fn google_item(
    item: &serde_json::Value,
    tz: &jiff::tz::TimeZone,
    today: Date,
) -> Result<Option<Series>, &'static str> {
    let text = |key: &str| -> Result<Option<&str>, &'static str> {
        match item.get(key) {
            None | Some(serde_json::Value::Null) => Ok(None),
            Some(serde_json::Value::String(s)) => Ok(Some(s.as_str())),
            Some(_) => Err("a field that is not text"),
        }
    };
    let calendar = text("calendar")?.ok_or("no calendar")?;
    let id = text("id")?.filter(|id| !id.is_empty()).ok_or("no id")?;
    let first = match text("first")? {
        None => None,
        Some(raw) => Some(google_time(raw, tz).ok_or("an unreadable first")?.date()),
    };
    let recurrence: Option<Vec<String>> = match item.get("recurrence") {
        None | Some(serde_json::Value::Null) => None,
        Some(serde_json::Value::Array(lines)) => Some(
            lines
                .iter()
                .map(|l| l.as_str().map(str::to_string).ok_or("a recurrence line that is not text"))
                .collect::<Result<_, _>>()?,
        ),
        Some(_) => return Err("a recurrence that is not a list"),
    };
    let end_of_horizon = add_days(today, HORIZON_DAYS);
    let mut instances: BTreeSet<(Date, Time, Time)> = BTreeSet::new();
    let listed = item.get("instances").and_then(|v| v.as_array()).ok_or("no instances")?;
    for instance in listed {
        let at = |key: &str| {
            instance.get(key).and_then(|v| v.as_str()).and_then(|raw| google_time(raw, tz))
        };
        let (start, end) = at("start").zip(at("end")).ok_or("an unreadable instance")?;
        if start.date() >= today && start.date() < end_of_horizon {
            instances.insert((start.date(), start.time(), end.time()));
        }
    }
    if instances.is_empty() {
        return Ok(None);
    }
    let instances: Vec<Instance> = instances
        .into_iter()
        .map(|(date, start, end)| Instance { date, start: Some(start), end: Some(end) })
        .collect();
    let has_master = first.is_some() && recurrence.is_some();
    let read = match (&recurrence, has_master) {
        (Some(lines), true) => read_rule(lines, first, tz),
        _ => RuleRead {
            rule: Rule { freq: String::new(), interval: 1, until: None, count: None },
            rdate: false,
            unsupported: false,
            until: None,
        },
    };
    Ok(Some(Series {
        source_uid: format!("gcal-series:{id}"),
        calendar: calendar.to_string(),
        title: title_text(text("title")?.unwrap_or("")),
        where_: where_of(text("location")?.unwrap_or(""), text("description")?.unwrap_or("")),
        // Bounded (fix round 1, M4), never emptied: `None` would read as eligible.
        event_type: text("event_type")?.map(|kind| cut(kind, EVENT_TYPE_MAX)),
        rule: read.rule,
        has_master,
        rdate: read.rdate,
        unsupported: read.unsupported,
        meets: meets_of(&instances),
        instances,
        first,
        until: read.until,
        last_seen: Some(today),
    }))
}

/// §3.2 for the Google route: `value` is the reply's `series` field (§4.1). Returns the series,
/// the calendars read fresh and complete, and warnings. A value with no `calendars_read` or no
/// `items` list is not a read at all (no calendar). An item that cannot be read takes its
/// calendar out of the fresh list — a partial read never ages anything (§3.3) — with one
/// warning; its other items are dropped with it. A calendar in `calendars_read` with no items is
/// a fresh, empty read. Sorted by `(source_uid, calendar)`.
pub fn series_from_google(
    value: &serde_json::Value,
    tz: &jiff::tz::TimeZone,
    today: Date,
) -> (Vec<Series>, Vec<String>, Vec<String>) {
    let mut warnings = Vec::new();
    let read = value.get("calendars_read").and_then(|v| v.as_array());
    let items = value.get("items").and_then(|v| v.as_array());
    let (Some(read), Some(items)) = (read, items) else {
        warnings.push("series: a Google reply without calendars_read and items; not read".to_string());
        return (Vec::new(), Vec::new(), warnings);
    };
    // Fix round 1, M2: a Google read names only `google:` calendars; any other key could
    // replace an ICS feed's series, so it is skipped with a warning.
    let mut calendars: BTreeSet<String> = BTreeSet::new();
    for key in read {
        match key.as_str() {
            Some(key) if key.starts_with("google:") => {
                calendars.insert(key.to_string());
            }
            _ => warnings.push(format!("series: a Google calendar key without google: ({key}); skipped")),
        }
    }
    let mut broken: BTreeSet<String> = BTreeSet::new();
    let mut series = Vec::new();
    for item in items {
        let calendar = item.get("calendar").and_then(|c| c.as_str()).unwrap_or("");
        if !calendars.contains(calendar) {
            continue; // §4.1 never sends one; a calendar not read is not ours to age.
        }
        match google_item(item, tz, today) {
            Ok(Some(found)) => series.push(found),
            Ok(None) => {}
            Err(why) => {
                if broken.insert(calendar.to_string()) {
                    warnings.push(format!("series: {calendar} not read ({why})"));
                }
            }
        }
    }
    calendars.retain(|c| !broken.contains(c));
    series.retain(|s| calendars.contains(&s.calendar));
    series.sort_by(|a, b| (&a.source_uid, &a.calendar).cmp(&(&b.source_uid, &b.calendar)));
    (series, calendars.into_iter().collect(), warnings)
}

/// §3.2 for an ICS feed, through `calfeed::weekly_series` over `[today, today + 28)`. `None`
/// when the text is not a calendar (no `BEGIN:VCALENDAR`): not read, so nothing ages (§3.3). A
/// calendar with no recurring master is `Some(empty)` — a fresh read (re-review M-e).
/// `weekly_series`' own warnings are not repeated: the busy path warns about the same events.
pub fn series_from_ics(
    feed_name: &str,
    ics: &str,
    tz: &jiff::tz::TimeZone,
    today: Date,
) -> (Option<Vec<Series>>, Vec<String>) {
    if !ics.to_ascii_uppercase().contains("BEGIN:VCALENDAR") {
        return (None, vec![format!("series: {feed_name} is not a calendar; not read")]);
    }
    let (found, _) = crate::calfeed::weekly_series(ics, tz, today, add_days(today, HORIZON_DAYS));
    let series = found
        .into_iter()
        .map(|ics| {
            let read = read_rule(&ics.rule_lines, Some(ics.first), tz);
            // `weekly_series` keeps the first reason it cannot be proposed; an RDATE is `rdate`,
            // any other reason is as good as a rule it refused.
            let refused = ics.ineligible.as_deref().is_some_and(|why| why != "RDATE");
            let instances: Vec<Instance> = ics
                .instances
                .iter()
                .map(|&(date, start, end)| match ics.all_day {
                    true => Instance { date, start: None, end: None },
                    false => Instance { date, start: Some(start), end: Some(end) },
                })
                .collect();
            Series {
                source_uid: ics.key,
                calendar: feed_name.to_string(),
                title: title_text(&ics.title),
                where_: where_of(&ics.location, &ics.description),
                event_type: None,
                rule: read.rule,
                has_master: true,
                rdate: read.rdate,
                unsupported: read.unsupported || refused,
                meets: meets_of(&instances),
                instances,
                first: Some(ics.first),
                until: read.until,
                last_seen: Some(today),
            }
        })
        .collect();
    (Some(series), Vec::new())
}

fn hm(time: Time) -> String {
    format!("{:02}:{:02}", time.hour(), time.minute())
}

fn date_json(date: Option<Date>) -> serde_json::Value {
    date.map_or(serde_json::Value::Null, |d| serde_json::Value::String(d.to_string()))
}

fn series_json(s: &Series) -> serde_json::Value {
    let instances: Vec<serde_json::Value> = s
        .instances
        .iter()
        .map(|i| serde_json::json!([i.date.to_string(), i.start.map(hm), i.end.map(hm)]))
        .collect();
    let meets: Vec<serde_json::Value> = s
        .meets
        .iter()
        .map(|m| serde_json::json!({"days": m.days, "end": hm(m.end), "start": hm(m.start)}))
        .collect();
    serde_json::json!({
        "calendar": s.calendar, "event_type": s.event_type, "first": date_json(s.first),
        "has_master": s.has_master, "instances": instances, "last_seen": date_json(s.last_seen),
        "meets": meets, "rdate": s.rdate,
        "rule": {"count": s.rule.count, "freq": s.rule.freq, "interval": s.rule.interval,
                 "until": date_json(s.rule.until)},
        "source_uid": s.source_uid, "title": s.title, "unsupported": s.unsupported,
        "until": date_json(s.until), "where": s.where_,
    })
}

/// The file's bytes: `ledger::dumps_value` (keys sorted), series by `(source_uid, calendar)`,
/// and a trailing newline (§3.3). The same data is the same bytes.
pub(crate) fn file_bytes(file: &SeriesFile) -> String {
    let mut series: Vec<&Series> = file.series.iter().collect();
    series.sort_by(|a, b| (&a.source_uid, &a.calendar).cmp(&(&b.source_uid, &b.calendar)));
    let calendars: serde_json::Map<String, serde_json::Value> = file
        .calendars
        .iter()
        .map(|(k, d)| (k.clone(), date_json(Some(*d))))
        .collect();
    let ended: serde_json::Map<String, serde_json::Value> = file
        .ended
        .iter()
        .map(|(k, e)| {
            let entry = serde_json::json!({
                "calendar": e.calendar, "dropped": e.dropped.to_string(),
                "last_instance": date_json(e.last_instance), "until": date_json(e.until),
            });
            (k.clone(), entry)
        })
        .collect();
    let value = serde_json::json!({
        "calendars": calendars, "ended": ended,
        "series": series.into_iter().map(series_json).collect::<Vec<_>>(),
    });
    format!("{}\n", crate::ledger::dumps_value(&value))
}

type Parsed<T> = Result<T, String>;

fn json_date(value: Option<&serde_json::Value>, what: &str) -> Parsed<Option<Date>> {
    match value {
        None | Some(serde_json::Value::Null) => Ok(None),
        Some(serde_json::Value::String(s)) => {
            s.parse::<Date>().map(Some).map_err(|_| format!("bad {what}"))
        }
        Some(_) => Err(format!("bad {what}")),
    }
}

fn json_text(value: Option<&serde_json::Value>, what: &str) -> Parsed<Option<String>> {
    match value {
        None | Some(serde_json::Value::Null) => Ok(None),
        Some(serde_json::Value::String(s)) => Ok(Some(s.clone())),
        Some(_) => Err(format!("bad {what}")),
    }
}

fn json_bool(value: Option<&serde_json::Value>, what: &str) -> Parsed<bool> {
    value.and_then(|v| v.as_bool()).ok_or_else(|| format!("bad {what}"))
}

fn json_u32(value: Option<&serde_json::Value>, what: &str) -> Parsed<Option<u32>> {
    match value {
        None | Some(serde_json::Value::Null) => Ok(None),
        Some(v) => v
            .as_u64()
            .and_then(|n| u32::try_from(n).ok())
            .map(Some)
            .ok_or_else(|| format!("bad {what}")),
    }
}

fn json_time(value: Option<&serde_json::Value>, what: &str) -> Parsed<Option<Time>> {
    match json_text(value, what)? {
        None => Ok(None),
        Some(raw) => parse_time(&raw).map(Some).ok_or_else(|| format!("bad {what}")),
    }
}

fn parse_series(value: &serde_json::Value) -> Parsed<Series> {
    let need = |text: Option<String>, what: &str| text.ok_or_else(|| format!("no {what}"));
    let rule = value.get("rule").filter(|r| r.is_object()).ok_or("no rule")?;
    let mut instances = Vec::new();
    for i in value.get("instances").and_then(|v| v.as_array()).ok_or("no instances")? {
        let parts = i.as_array().filter(|p| p.len() == 3).ok_or("bad instance")?;
        let date = json_date(parts.first(), "instance")?.ok_or("bad instance")?;
        let (start, end) = (json_time(parts.get(1), "instance")?, json_time(parts.get(2), "instance")?);
        if start.is_some() != end.is_some() {
            return Err("bad instance".into());
        }
        instances.push(Instance { date, start, end });
    }
    let mut meets = Vec::new();
    for m in value.get("meets").and_then(|v| v.as_array()).ok_or("no meets")? {
        let days = m.get("days").and_then(|d| d.as_array()).ok_or("bad meets")?;
        let days = days
            .iter()
            .map(|d| d.as_str().and_then(day_of).ok_or_else(|| "bad meets".to_string()))
            .collect::<Parsed<Vec<DayKey>>>()?;
        let start = json_time(m.get("start"), "meets")?.ok_or("bad meets")?;
        let end = json_time(m.get("end"), "meets")?.ok_or("bad meets")?;
        meets.push(Meet { days, start, end });
    }
    Ok(Series {
        source_uid: need(json_text(value.get("source_uid"), "source_uid")?, "source_uid")?,
        calendar: need(json_text(value.get("calendar"), "calendar")?, "calendar")?,
        title: need(json_text(value.get("title"), "title")?, "title")?,
        where_: json_text(value.get("where"), "where")?,
        event_type: json_text(value.get("event_type"), "event_type")?,
        rule: Rule {
            freq: need(json_text(rule.get("freq"), "freq")?, "freq")?,
            interval: json_u32(rule.get("interval"), "interval")?.ok_or("no interval")?,
            until: json_date(rule.get("until"), "until")?,
            count: json_u32(rule.get("count"), "count")?,
        },
        has_master: json_bool(value.get("has_master"), "has_master")?,
        rdate: json_bool(value.get("rdate"), "rdate")?,
        unsupported: json_bool(value.get("unsupported"), "unsupported")?,
        instances,
        meets,
        first: json_date(value.get("first"), "first")?,
        until: json_date(value.get("until"), "until")?,
        last_seen: json_date(value.get("last_seen"), "last_seen")?,
    })
}

fn parse_file(text: &str) -> Parsed<SeriesFile> {
    let value: serde_json::Value = serde_json::from_str(text).map_err(|_| "not JSON".to_string())?;
    let object = value.as_object().ok_or("not an object")?;
    let mut file = SeriesFile::default();
    for (key, read) in object.get("calendars").and_then(|v| v.as_object()).ok_or("no calendars")? {
        let read = json_date(Some(read), "calendar date")?.ok_or("bad calendar date")?;
        file.calendars.insert(key.clone(), read);
    }
    for (key, entry) in object.get("ended").and_then(|v| v.as_object()).ok_or("no ended")? {
        let ended = Ended {
            calendar: json_text(entry.get("calendar"), "ended")?.ok_or("bad ended")?,
            dropped: json_date(entry.get("dropped"), "ended")?.ok_or("bad ended")?,
            last_instance: json_date(entry.get("last_instance"), "ended")?,
            until: json_date(entry.get("until"), "ended")?,
        };
        file.ended.insert(key.clone(), ended);
    }
    for series in object.get("series").and_then(|v| v.as_array()).ok_or("no series")? {
        file.series.push(parse_series(series)?);
    }
    file.series.sort_by(|a, b| (&a.source_uid, &a.calendar).cmp(&(&b.source_uid, &b.calendar)));
    Ok(file)
}

fn series_path(vault: &Path) -> PathBuf {
    vault.join("state").join("calendar-series.json")
}

/// What reading the file found (fix round 1, M6).
enum Loaded {
    /// Read and parsed, or missing (empty).
    Read(SeriesFile),
    /// Not UTF-8 or not the file's shape: empty, and the next fresh read regenerates it.
    Malformed(String),
    /// An I/O error other than not-found (a sharing violation, a folder in its place): the file
    /// may be fine, so it is left alone and not rewritten this run.
    Unreadable(std::io::ErrorKind),
}

fn load_series_file(vault: &Path) -> Loaded {
    match std::fs::read_to_string(series_path(vault)) {
        Ok(text) => match parse_file(&text) {
            Ok(file) => Loaded::Read(file),
            Err(why) => Loaded::Malformed(why),
        },
        Err(err) if err.kind() == std::io::ErrorKind::NotFound => Loaded::Read(SeriesFile::default()),
        Err(err) if err.kind() == std::io::ErrorKind::InvalidData => Loaded::Malformed("not UTF-8".into()),
        Err(err) => Loaded::Unreadable(err.kind()),
    }
}

/// `state/calendar-series.json` (§3.3). Missing: silent and empty. Malformed in any part, or
/// unreadable: empty, with one warning. A malformed file is rewritten by the next fresh read; an
/// unreadable one is left alone ([`refresh_series`]).
pub fn read_series_file(vault: &Path) -> (SeriesFile, Vec<String>) {
    match load_series_file(vault) {
        Loaded::Read(file) => (file, Vec::new()),
        Loaded::Malformed(why) => (
            SeriesFile::default(),
            vec![format!("series file: {SERIES_FILE} malformed ({why}); starting empty")],
        ),
        Loaded::Unreadable(kind) => (
            SeriesFile::default(),
            vec![format!("series file: {SERIES_FILE} unreadable ({kind}); read as empty")],
        ),
    }
}

/// Writes `bytes` beside the file, then renames it over (fix round 1, M5): a crash mid-write
/// leaves the old file whole, never a truncated one. The crate's other temp-then-rename writer
/// (`backup::place`) copies a file rather than writing bytes, so it is not reused.
fn write_series_file(vault: &Path, bytes: &str) -> std::io::Result<()> {
    write_state_file(vault, "calendar-series.json", bytes)
}

/// `state/<name>`, whole: `bytes` go to `<name>.tmp<pid>` beside it, then are renamed over it, so
/// a crash mid-write leaves the old file intact (the series file, fix round 1 M5; `plan.json`,
/// P16 fix round 1 M2).
fn write_state_file(vault: &Path, name: &str, bytes: &str) -> std::io::Result<()> {
    std::fs::create_dir_all(vault.join("state"))?;
    let path = vault.join("state").join(name);
    let tmp = path.with_file_name(format!("{name}.tmp{}", std::process::id()));
    std::fs::write(&tmp, bytes.as_bytes())?;
    std::fs::rename(&tmp, &path).inspect_err(|_| {
        let _ = std::fs::remove_file(&tmp);
    })
}

/// §3.3's per-calendar refresh. `fresh` holds each calendar read fresh and complete this run
/// with every series it returned (an ICS feed with no master is fresh with none).
///
/// - A fresh calendar's date becomes `today`; its series are replaced (`last_seen` today). One it
///   no longer returns keeps its old `last_seen` until that is 14 days old; then it is dropped
///   and — unless another calendar still holds its key — moves to `ended`.
/// - A calendar not in `fresh` is untouched, unless its feed left `config/ingest.yaml` (an ICS
///   calendar key is its feed's name; every `google:` key belongs to the `cloud:google` feed,
///   plan review M13): then its series and its date go 14 days after its last read, never to
///   `ended` — the student removed the source, not the class.
/// - An `ended` entry goes 28 days after `dropped`, or at once when its key is held again.
/// - A record with no `last_seen` (the engine never writes one; only a hand-edited file) counts
///   as unseen for 14 days already, so a fresh read that does not return it ages it out at once.
///
/// The file is written only when `fresh` is non-empty and the bytes differ, whole (temp file,
/// then rename). A file that could not be *read* (an I/O error, not a parse failure) is left
/// alone and not rewritten this run, with one warning; the returned data then holds only this
/// run's fresh calendars. Warnings: the old file's, and a failed write's.
pub fn refresh_series(
    vault: &Path,
    fresh: &[(String, Vec<Series>)],
    today: Date,
) -> (SeriesFile, Vec<String>) {
    let (old, mut warnings, writable) = match load_series_file(vault) {
        Loaded::Read(file) => (file, Vec::new(), true),
        Loaded::Malformed(why) => (
            SeriesFile::default(),
            vec![format!("series file: {SERIES_FILE} malformed ({why}); starting empty")],
            true,
        ),
        Loaded::Unreadable(kind) => (
            SeriesFile::default(),
            vec![format!("series file: {SERIES_FILE} unreadable ({kind}); not written this run")],
            false,
        ),
    };
    let feeds = crate::calfeed::calendar_entries(vault);
    let names: BTreeSet<&str> = feeds.iter().map(|(name, _)| name.as_str()).collect();
    let google = feeds.iter().any(|(_, url)| url == "cloud:google");
    let configured = |calendar: &str| match calendar.starts_with("google:") {
        true => google,
        false => names.contains(calendar),
    };
    let recent = |read: Option<Date>| read.is_some_and(|d| days_since(d, today) < UNSEEN_DAYS);

    let mut calendars = old.calendars;
    let mut series: Vec<Series> = Vec::new();
    let mut returned: BTreeMap<&str, BTreeSet<&str>> = BTreeMap::new();
    for (calendar, list) in fresh {
        calendars.insert(calendar.clone(), today);
        let keys = returned.entry(calendar.as_str()).or_default();
        for found in list {
            if keys.insert(found.source_uid.as_str()) {
                let mut found = found.clone();
                found.calendar = calendar.clone();
                found.last_seen = Some(today);
                series.push(found);
            }
        }
    }
    let mut aged_out: Vec<Series> = Vec::new();
    for held in old.series {
        match returned.get(held.calendar.as_str()) {
            Some(keys) if keys.contains(held.source_uid.as_str()) => {}
            Some(_) if recent(held.last_seen) => series.push(held),
            Some(_) => aged_out.push(held),
            None if configured(&held.calendar) => series.push(held),
            None => {
                let read = calendars.get(&held.calendar).copied().or(held.last_seen);
                if recent(read) {
                    series.push(held);
                }
            }
        }
    }
    calendars.retain(|calendar, read| {
        returned.contains_key(calendar.as_str()) || configured(calendar) || recent(Some(*read))
    });
    series.sort_by(|a, b| (&a.source_uid, &a.calendar).cmp(&(&b.source_uid, &b.calendar)));

    let held: BTreeSet<&str> = series.iter().map(|s| s.source_uid.as_str()).collect();
    let mut ended = old.ended;
    ended.retain(|key, e| !held.contains(key.as_str()) && days_since(e.dropped, today) < ENDED_DAYS);
    aged_out.sort_by(|a, b| precedence(a).cmp(&precedence(b)));
    for gone in aged_out {
        if held.contains(gone.source_uid.as_str()) || ended.contains_key(&gone.source_uid) {
            continue;
        }
        let entry = Ended {
            calendar: gone.calendar.clone(),
            dropped: today,
            last_instance: gone.instances.iter().map(|i| i.date).max(),
            until: gone.until,
        };
        ended.insert(gone.source_uid, entry);
    }

    let file = SeriesFile { calendars, ended, series };
    if !fresh.is_empty() && writable {
        let bytes = file_bytes(&file);
        if std::fs::read(series_path(vault)).ok().as_deref() != Some(bytes.as_bytes()) {
            if let Err(err) = write_series_file(vault, &bytes) {
                warnings.push(format!("series file: could not write {SERIES_FILE} ({})", err.kind()));
            }
        }
    }
    (file, warnings)
}

// ---------------------------------------------------------------------------------------------
// P9 — proposals and the window proposal (§3.5).
// ---------------------------------------------------------------------------------------------

/// The one decline-marker key that is not a source's (§2.3): the student rejected a window
/// proposal, and no window is proposed again (§3.5, re-review M-i).
pub const WINDOW_MARKER: &str = "window";

/// A window proposal's `source_uid` is this prefix + its routine keys joined by `,` (§5.2).
pub const WINDOW_PREFIX: &str = "window:";

/// The window proposal's title (§5.2: `Your day · Mon–Fri 8am–10pm · plan in this window?`).
pub const WINDOW_TITLE: &str = "Your day";

/// §3.5's signature `(kind, course or lower-cased title, meets)`: the same meeting from a second
/// route (a Google key and an ICS key, a registrar row, a hand-written note, a successor) has the
/// same one. `course` stands in for the title when it is set, so `title` is then empty; `meets` is
/// the set of `(DAY_KEYS index, start, end)` triples, so neither the order of the entries nor the
/// order of the days inside one matters. `course` is compared by slug **or** code (fix round 1,
/// M4): a hand-written `course: CS 100` and the classifier's slug `cs-100` are one course.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub struct Signature {
    pub kind: String,
    pub course: Option<String>,
    pub title: String,
    pub meets: BTreeSet<(usize, Time, Time)>,
}

/// The signature of whatever carries these fields (a series' classification, a note, a card's
/// `commitment:`).
pub fn signature(
    kind: &str,
    course: Option<&str>,
    title: &str,
    meets: &[Meet],
    codes: &Codes,
) -> Signature {
    let course = course
        .map(|c| course_key(c, codes))
        .filter(|c| !c.is_empty());
    let title = match course {
        Some(_) => String::new(),
        // Runs of whitespace, line breaks included, are one space (P11 review m2): a series
        // titled `Chess\nClub` and the note its card confirmed as `Chess Club` are one meeting.
        None => title
            .split(pystr::is_python_space)
            .filter(|part| !part.is_empty())
            .collect::<Vec<_>>()
            .join(" ")
            .to_lowercase(),
    };
    let meets = meets
        .iter()
        .flat_map(|m| {
            m.days
                .iter()
                .filter_map(|d| DAY_KEYS.iter().position(|k| k == d))
                .map(move |i| (i, m.start, m.end))
        })
        .collect();
    Signature {
        kind: kind.to_string(),
        course,
        title,
        meets,
    }
}

/// A course written as a slug or as a code, in one comparable form: the compact code (§3.4's
/// normalisation, the classifier's own), mapped through the vault's code table to its slug when it
/// names one, then compacted again (`CS 100`, `cs-100`, `CS100` → `CS100` for the slug `cs-100`).
fn course_key(course: &str, codes: &Codes) -> String {
    let code = compact(course);
    match codes.table.get(&code) {
        Some(slug) => compact(slug),
        None => code,
    }
}

impl Commitment {
    pub fn signature(&self, codes: &Codes) -> Signature {
        signature(&self.kind, self.course.as_deref(), &self.title, &self.meets, codes)
    }
}

/// What a confirmed note needs (§3.5, §2.1): a series the student has not answered yet, or the
/// one window proposal (`kind: planning-day`, its window per weekday in `meets`, the §2.4 shape).
/// Plain data, recomputed each run and held only in memory: nothing here writes a note, a card,
/// a journal record or a file, so a proposal stays on the device until a card or the screen
/// confirms it (R18, R20).
#[derive(Debug, Clone, PartialEq)]
pub struct Proposal {
    pub kind: String,
    pub level: Level,
    pub title: String,
    pub course: Option<String>,
    pub meets: Vec<Meet>,
    pub where_: Option<String>,
    pub from: Option<Date>,
    pub until: Option<Date>,
    pub source_uid: String,
}

impl Proposal {
    pub fn signature(&self, codes: &Codes) -> Signature {
        signature(&self.kind, self.course.as_deref(), &self.title, &self.meets, codes)
    }

    /// The window proposal (its `source_uid` starts [`WINDOW_PREFIX`]).
    pub fn is_window(&self) -> bool {
        self.source_uid.starts_with(WINDOW_PREFIX)
    }
}

/// Minutes since midnight.
fn minutes(time: Time) -> i64 {
    i64::from(time.hour()) * 60 + i64::from(time.minute())
}

/// §3.5's proposals, pure (no clock, no I/O, no model), in `source_uid` order, over
/// [`SeriesFile::by_key`] so one key is one proposal whichever calendars hold it.
///
/// A series is proposed only if it is eligible and classified (P7's `classify`), its `until` is
/// not before `today` (P7 left that check to the caller: `classify` has no `today`), its key is
/// not in `held` (keys a change card's `change.source_uid` carries, P12's `successor_keys`), no
/// note has its key (a confirmed note, a decline marker, the planning day), no confirmed note has
/// its [`Signature`], and — when `for_cards` — it is not `office-hours` (R8; the screen lists
/// them).
///
/// **Twins** (P8 review): one real series can reach the file under two keys — an Outlook/Exchange
/// invite keeps its own UID, so a `gcal-series:` key and an `ics-series:` key carry the same
/// signature. Candidates are grouped by signature and one proposal is made per group, from the
/// record [`SeriesFile::by_key`]'s precedence prefers (a `google:` calendar, then the lower
/// calendar key; then the lower key). A key answered or held closes its signature, so a declined,
/// confirmed or held twin suppresses the other.
///
/// Routines feed the one window proposal (decision 3, [`window_proposal`]), never a commitment.
#[allow(clippy::too_many_arguments)]
pub fn proposals(
    file: &SeriesFile,
    set: &Commitments,
    codes: &Codes,
    planning: &[String],
    template: &crate::weekcal::WeekCalendar,
    held: &BTreeSet<String>,
    today: Date,
    for_cards: bool,
) -> Vec<Proposal> {
    let answered: BTreeSet<&str> = set
        .confirmed
        .iter()
        .chain(set.planning_day.iter())
        .filter_map(|n| n.source_uid.as_deref())
        .chain(set.declined.iter().map(String::as_str))
        .collect();
    let mut closed: BTreeSet<Signature> = set.confirmed.iter().map(|n| n.signature(codes)).collect();

    // Each candidate carries its signature and its title-only signature: a confirmed note with no
    // `course` (a hand-written class note, say) matches on the title instead (fix round 1, M4).
    let mut candidates: Vec<(Signature, Signature, &Series, String, Option<String>)> = Vec::new();
    let mut routines: Vec<(&str, &Series, bool, bool)> = Vec::new();
    for (key, series) in file.by_key() {
        if series.until.is_some_and(|until| until < today) {
            continue;
        }
        match classify(series, codes, planning) {
            None => {}
            Some(Class::Routine { wake, bed }) => routines.push((key, series, wake, bed)),
            Some(Class::Kind { kind, course }) => {
                let sig = signature(&kind, course.as_deref(), &series.title, &series.meets, codes);
                let by_title = signature(&kind, None, &series.title, &series.meets, codes);
                if answered.contains(key) || held.contains(key) {
                    closed.insert(sig);
                    closed.insert(by_title);
                } else {
                    candidates.push((sig, by_title, series, kind, course));
                }
            }
        }
    }

    fn rank(s: &Series) -> ((bool, &str), &str) {
        (precedence(s), s.source_uid.as_str())
    }
    let mut chosen: BTreeMap<Signature, (&Series, String, Option<String>)> = BTreeMap::new();
    for (sig, by_title, series, kind, course) in candidates {
        if closed.contains(&sig) || closed.contains(&by_title) {
            continue;
        }
        match chosen.get(&sig) {
            Some((kept, _, _)) if rank(kept) <= rank(series) => {}
            _ => {
                chosen.insert(sig, (series, kind, course));
            }
        }
    }

    let mut out: Vec<Proposal> = chosen
        .into_values()
        .filter(|(_, kind, _)| !(for_cards && kind == "office-hours"))
        .map(|(series, kind, course)| Proposal {
            level: default_level(&kind).unwrap_or(Level::Soft),
            kind,
            title: series.title.clone(),
            course,
            meets: series.meets.clone(),
            where_: series.where_.clone(),
            from: series.first,
            until: series.until,
            source_uid: series.source_uid.clone(),
        })
        .collect();
    if set.planning_day.is_none() && !set.declined.contains(WINDOW_MARKER) {
        out.extend(window_proposal(&routines, template));
    }
    out.sort_by(|a, b| a.source_uid.cmp(&b.source_uid));
    out
}

/// A bed-side routine starting before this hour (`00:00`–`04:59`) is the previous night's
/// bedtime (fix round 1, M3).
const AFTER_MIDNIGHT_UNTIL_HOUR: i8 = 5;

/// The latest end a window can have (§2.4: a window past midnight is not representable).
const LAST_MINUTE: Time = Time::constant(23, 59, 0, 0);

/// Decision 3, §3.5: per weekday, `start` = the latest wake-side time (a wake event's end) and
/// `end` = the earliest bed-side time (a bed event's start); one side missing takes the
/// template's `day_start`/`day_end`; a weekday with neither side, or whose result is inverted or
/// shorter than `min_block_minutes`, is left out. Weekdays with equal `(start, end)` share one
/// entry, days in `DAY_KEYS` order, entries by their first day. `None` when no weekday is left.
///
/// Around midnight (fix round 1):
/// - a midnight-crossing `sleep` entry is both sides: its start is the bed side on its own day,
///   its end the wake side on the next — unless it ends exactly at `00:00`, which is bed side
///   only (M2: a 00:00 wake is no morning);
/// - a bed-side entry that **starts after midnight** (`00:00`–`04:59`) is the previous night's
///   bedtime, which §2.4 cannot write past `23:59`: the previous weekday's bed side is `23:59`
///   (M3, controller ruling, §2.4's own rule). For a `sleep` series its end is also that day's
///   wake side (a sleep span ends on waking); a point-like bedtime's end is not a wake time.
///
/// The `source_uid` is `window:` + **every** eligible routine key joined by `,` (M6), including
/// a routine whose days were all left out: it names the routines the proposal was built from, not
/// the days it kept. It changes whenever a routine series appears or leaves, so no consumer may
/// dedupe a window on the exact uid — P11 asks once while any `window:` card is not `superseded`,
/// and the `window` marker closes the question for good.
fn window_proposal(
    routines: &[(&str, &Series, bool, bool)],
    template: &crate::weekcal::WeekCalendar,
) -> Option<Proposal> {
    let mut wake: [Option<Time>; 7] = [None; 7];
    let mut bed: [Option<Time>; 7] = [None; 7];
    let later = |slot: &mut Option<Time>, t: Time| *slot = Some(slot.map_or(t, |s| s.max(t)));
    let earlier = |slot: &mut Option<Time>, t: Time| *slot = Some(slot.map_or(t, |s| s.min(t)));
    for (_, series, wake_side, bed_side) in routines {
        for meet in &series.meets {
            for day in &meet.days {
                let Some(i) = DAY_KEYS.iter().position(|k| k == day) else {
                    continue;
                };
                if meet.end < meet.start {
                    if *bed_side {
                        earlier(&mut bed[i], meet.start);
                    }
                    if *wake_side && meet.end != Time::midnight() {
                        later(&mut wake[(i + 1) % 7], meet.end);
                    }
                } else if *bed_side && meet.start.hour() < AFTER_MIDNIGHT_UNTIL_HOUR {
                    earlier(&mut bed[(i + 6) % 7], LAST_MINUTE);
                    if is_sleep(&series.title) {
                        later(&mut wake[i], meet.end);
                    }
                } else if *wake_side {
                    later(&mut wake[i], meet.end);
                } else if *bed_side {
                    earlier(&mut bed[i], meet.start);
                }
            }
        }
    }

    let mut groups: BTreeMap<(Time, Time), Vec<usize>> = BTreeMap::new();
    for i in 0..7 {
        if wake[i].is_none() && bed[i].is_none() {
            continue;
        }
        let start = wake[i].unwrap_or(template.day_start);
        let end = bed[i].unwrap_or(template.day_end);
        if start >= end || minutes(end) - minutes(start) < template.min_block_minutes {
            continue;
        }
        groups.entry((start, end)).or_default().push(i);
    }
    if groups.is_empty() {
        return None;
    }
    let mut meets: Vec<(usize, Meet)> = groups
        .into_iter()
        .map(|((start, end), days)| {
            let meet = Meet {
                days: days.iter().map(|&i| DAY_KEYS[i]).collect(),
                start,
                end,
            };
            (days[0], meet)
        })
        .collect();
    meets.sort_by_key(|(first, meet)| (*first, meet.start));
    let keys: Vec<&str> = routines.iter().map(|(key, ..)| *key).collect();
    Some(Proposal {
        kind: PLANNING_DAY.to_string(),
        level: default_level(PLANNING_DAY).unwrap_or(Level::Optional),
        title: WINDOW_TITLE.to_string(),
        course: None,
        meets: meets.into_iter().map(|(_, meet)| meet).collect(),
        where_: None,
        from: None,
        until: None,
        source_uid: format!("{WINDOW_PREFIX}{}", keys.join(",")),
    })
}

// ---------------------------------------------------------------------------------------------
// P11 — the `commitment-check` card (§5.2, §5.4, §5.5).
// ---------------------------------------------------------------------------------------------

/// The card kind this piece files (one of [`LOCAL_CARD_KINDS`]).
pub const COMMITMENT_CHECK: &str = "commitment-check";

/// The actor of every card and every journal record about one (§5.2); `provenance::is_agent` is
/// a `starts_with` test, so no new `journal::VIAS` entry is needed.
pub const CARD_ACTOR: &str = "agent:commitments";

/// The most `commitment-check` cards first proposed on any one day (R9).
pub const CHECKS_PER_DAY: i64 = 5;

/// The longest title a card's own title quotes (§5.2 `{title≤40}`).
const CARD_TITLE_MAX: usize = 40;

/// The card's first paragraph — what `surface::first_paragraph` shows as its `why`.
const CHECK_WHY: &str =
    "**Is this part of your week?** Knowlu found it repeating on your calendar.";
const CHECK_CLOSING: &str = "Reject and it's ignored. Either way you won't be asked again.";
const HARD_SENTENCE: &str = "Approve and Knowlu never plans anything over it.";
const SOFT_SENTENCE: &str =
    "Approve and Knowlu counts it as busy; an event suggestion may overlap it, and will say so.";
const OPTIONAL_SENTENCE: &str = "Optional: Knowlu won't plan around it.";
/// The window card's own answer paragraph (controller ruling, fix round 1: rejecting keeps the
/// usual hours, it does not "ignore" anything), then [`WINDOW_CLOSING`].
const WINDOW_SENTENCE: &str =
    "Approve to plan inside these hours. Reject to keep your usual hours.";
const WINDOW_CLOSING: &str = "Either way you won't be asked again.";
const CHANGE_WHY: &str = "**Has this changed?** Your calendar entry for this now looks different.";
const CHANGE_CLOSING: &str = "Approve and Knowlu updates this. Reject and it stays as it is. \
You won't be asked about this change again.";
/// What a change card's detail line shows for an absent old (or new) value.
const NOT_SET: &str = "(not set)";

/// A change to a confirmed commitment (§5.4), detected by P12 and filed here. `target` is
/// `commitments/<file>.md`; `source_uid` and `title` are the note's; `change` holds the new values
/// of the changed fields only; `was` the note's current values of the same fields, an absent one
/// as `null`.
#[derive(Debug, Clone, PartialEq)]
pub struct Change {
    pub target: String,
    pub source_uid: String,
    pub title: String,
    pub change: Mapping,
    pub was: Mapping,
}

/// File one local-only card: the **only** function in this piece that writes into `approvals/`
/// (§5.2 "One constructor", R20). Refuses a `kind` outside [`LOCAL_CARD_KINDS`] before touching
/// anything, so every card it writes is one C3′'s sync keeps off the wire.
///
/// The frontmatter is `type: approval`, `kind`, `title`, `status: pending`, then `fields` in the
/// order given, then `proposed_at` and `first_proposed_at` (both today, two distinct dates — never
/// an anchor), `expires: null`, `snooze_until: null`, `created_by: agent:commitments`, every line
/// through [`front_matter`] (Global Constraint 23), which refuses anything that would span two
/// lines. The file is `approvals/<kind>-<slugify(title)>.md`, `-2`, `-3` on a collision **in
/// `approvals/` only** (an archived card of the same name is no collision). The journal's actor is
/// `agent:commitments` whatever `ctx` carries; its `via` and `run_id` are kept.
#[allow(clippy::too_many_arguments)]
pub fn file_card(
    vault: &Path,
    kind: &str,
    fields: Vec<(&str, Field)>,
    title: &str,
    body: &str,
    today: Date,
    ctx: &crate::write::WriteContext,
    journal: &mut crate::journal::Journal,
) -> Result<PathBuf, String> {
    if !LOCAL_CARD_KINDS.contains(&kind) {
        return Err(format!("{kind}: not a local card kind, not filed"));
    }
    let mut all: Vec<(&str, Field)> = vec![
        ("type", Field::Scalar(Node::text("approval"))),
        ("kind", Field::Scalar(Node::text(kind))),
        ("title", Field::Scalar(Node::text(title))),
        ("status", Field::Scalar(Node::text("pending"))),
    ];
    all.extend(fields);
    all.extend([
        ("proposed_at", Field::Scalar(Node::Date(today))),
        ("first_proposed_at", Field::Scalar(Node::Date(today))),
        ("expires", Field::Scalar(Node::Null)),
        ("snooze_until", Field::Scalar(Node::Null)),
        ("created_by", Field::Scalar(Node::text(CARD_ACTOR))),
    ]);
    let front = front_matter(&all).map_err(|e| format!("card not filed ({title}): {e}"))?;
    let text = format!("---\n{front}---\n\n{body}");

    let folder = vault.join("approvals");
    std::fs::create_dir_all(&folder).map_err(|e| format!("card not filed ({title}): {e}"))?;
    let stem = format!("{kind}-{}", crate::ingest::slugify(title));
    let mut name = format!("{stem}.md");
    let mut suffix = 2;
    while folder.join(&name).exists() {
        name = format!("{stem}-{suffix}.md");
        suffix += 1;
    }
    crate::write::create(
        vault,
        &format!("approvals/{name}"),
        &text,
        &ctx.with_actor(CARD_ACTOR),
        journal,
        None,
    )
    .map_err(|e| format!("card not filed ({title}): {e}"))
}

/// `Mon`, `Tue`, … for a `DAY_KEYS` index.
fn day_name(i: usize) -> String {
    let key = DAY_KEYS[i];
    key[..1].to_uppercase() + &key[1..]
}

/// `Mon/Wed/Fri`; a run of three or more consecutive days collapses to `Mon–Fri` (`Mon–Wed/Fri`).
fn days_label(days: &[DayKey]) -> String {
    let mut idx: Vec<usize> = days
        .iter()
        .filter_map(|d| DAY_KEYS.iter().position(|k| k == d))
        .collect();
    idx.sort_unstable();
    idx.dedup();
    let mut parts: Vec<String> = Vec::new();
    let mut i = 0;
    while i < idx.len() {
        let mut j = i;
        while j + 1 < idx.len() && idx[j + 1] == idx[j] + 1 {
            j += 1;
        }
        if j - i >= 2 {
            parts.push(format!("{}\u{2013}{}", day_name(idx[i]), day_name(idx[j])));
        } else {
            parts.extend((i..=j).map(|k| day_name(idx[k])));
        }
        i = j + 1;
    }
    parts.join("/")
}

/// F2's range: `12–12:50pm`, `8am–10pm`, `9:30–10:45am` (the start carries am/pm only when it
/// is on the other side of noon).
fn range_label(start: Time, end: Time) -> String {
    let split = (start.hour() < 12) != (end.hour() < 12);
    format!(
        "{}\u{2013}{}",
        crate::eventemit::clock(start, split),
        crate::eventemit::clock(end, true)
    )
}

/// `Mon/Wed/Fri 12–12:50pm` from the first entry; ` +1 more time` (` +N more times`) for the rest.
fn meets_label(meets: &[Meet]) -> Option<String> {
    let first = meets.first()?;
    let mut out = format!("{} {}", days_label(&first.days), range_label(first.start, first.end));
    match meets.len() - 1 {
        0 => {}
        1 => out.push_str(" +1 more time"),
        n => out.push_str(&format!(" +{n} more times")),
    }
    Some(out)
}

/// A title as a card's title quotes it: one line, at most 40 characters.
fn short_title(title: &str) -> String {
    let short = crate::judge::one_line(title, CARD_TITLE_MAX);
    let short = short.trim_end();
    if short.is_empty() {
        "(untitled)".to_string()
    } else {
        short.to_string()
    }
}

fn question(kind: &str) -> &'static str {
    match kind {
        "class" => "a class?",
        "lab" => "a lab?",
        "work" => "work?",
        "club" => "a club?",
        "meeting" => "a meeting?",
        _ => "part of your week?",
    }
}

/// §5.2's title: `CS 100 · Mon/Wed/Fri 12–12:50pm · a class?`, or the window's
/// `Your day · Mon–Fri 8am–10pm · plan in this window?`.
fn proposal_title(p: &Proposal) -> String {
    let (name, ask) = if p.is_window() {
        (WINDOW_TITLE.to_string(), "plan in this window?")
    } else {
        (short_title(&p.title), question(&p.kind))
    };
    match meets_label(&p.meets) {
        Some(when) => format!("{name} · {when} · {ask}"),
        None => format!("{name} · {ask}"),
    }
}

fn meets_json(meets: &[Meet]) -> serde_json::Value {
    serde_json::Value::Array(
        meets
            .iter()
            .map(|m| {
                serde_json::json!({
                    "days": m.days.iter().map(|d| d.to_string()).collect::<Vec<_>>(),
                    "start": hm(m.start),
                    "end": hm(m.end),
                })
            })
            .collect(),
    )
}

fn opt_json(text: Option<&str>) -> serde_json::Value {
    text.map(single_line)
        .filter(|t| !t.trim().is_empty())
        .map_or(serde_json::Value::Null, serde_json::Value::String)
}

/// The card's `commitment:` mapping (§5.2): `kind`, `level`, `title`, `course`, `meets`, `where`,
/// `from`, `until` — absent ones `null` — or, for the window, `kind: planning-day`, `title` and
/// `window` (§2.4's shape). The title is verbatim but for line breaks, which become spaces.
fn proposal_commitment(p: &Proposal) -> serde_json::Value {
    if p.is_window() {
        return serde_json::json!({
            "kind": PLANNING_DAY,
            "title": single_line(&p.title),
            "window": meets_json(&p.meets),
        });
    }
    serde_json::json!({
        "kind": p.kind,
        "level": p.level.as_str(),
        "title": single_line(&p.title),
        "course": opt_json(p.course.as_deref()),
        "meets": meets_json(&p.meets),
        "where": opt_json(p.where_.as_deref()),
        "from": date_json(p.from),
        "until": date_json(p.until),
    })
}

fn proposal_body(p: &Proposal) -> String {
    let sentence = if p.is_window() {
        WINDOW_SENTENCE
    } else {
        match p.level {
            Level::Hard => HARD_SENTENCE,
            Level::Soft => SOFT_SENTENCE,
            Level::Optional => OPTIONAL_SENTENCE,
        }
    };
    let mut parts = vec![CHECK_WHY.to_string(), sentence.to_string()];
    if p.is_window() {
        parts.push(WINDOW_CLOSING.to_string());
        return parts.join("\n\n") + "\n";
    }
    if let Some(place) = p.where_.as_deref().map(single_line).filter(|w| !w.trim().is_empty()) {
        parts.push(format!("Where: {}", place.trim()));
    }
    parts.push(CHECK_CLOSING.to_string());
    parts.join("\n\n") + "\n"
}

/// A mapping as the card writes it and a reader parses it back, then as canonical flow text
/// (`safe_dump_flow(parse(value))`, §5.4) — so a `Change` and a card on disk compare equal.
fn canonical(value: &Value) -> String {
    let literal = crate::write::to_literal(&crate::yaml::from_json(&crate::yaml::to_json(value)));
    yamlemit::safe_dump_flow(&crate::write::parse_literal(&literal))
}

/// `Dec 4`.
fn short_date(day: Date) -> String {
    format!("{} {}", day.strftime("%b"), day.day())
}

fn value_meets(value: Option<&Value>) -> Option<Vec<Meet>> {
    match value {
        Some(Value::Sequence(items)) => {
            let meets: Vec<Meet> = items.iter().filter_map(|v| parse_entry(v).ok()).collect();
            (!meets.is_empty()).then_some(meets)
        }
        _ => None,
    }
}

fn value_date(value: Option<&Value>) -> Option<Date> {
    value.and_then(text).and_then(|t| t.trim().parse::<Date>().ok())
}

/// §5.4's title: `CS 100 now meets Tue/Thu 9:30–10:45am · update?` when `meets` changed, else
/// `CS 100 ends Dec 4 · update?` for `until`, else `CS 100 is now in Room 2 · update?` for
/// `where`, else `CS 100 changed · update?`.
fn change_title(c: &Change) -> String {
    let name = short_title(&c.title);
    if let Some(when) = value_meets(get(&c.change, "meets")).as_deref().and_then(meets_label) {
        return format!("{name} now meets {when} · update?");
    }
    if let Some(until) = value_date(get(&c.change, "until")) {
        return format!("{name} ends {} · update?", short_date(until));
    }
    if let Some(place) = get(&c.change, "where").and_then(text) {
        return format!("{name} is now in {} · update?", crate::judge::one_line(&place, 40));
    }
    format!("{name} changed · update?")
}

/// One `Field: now …, was …` line per changed field the student can read (`source_uid` is not
/// one).
fn change_body(c: &Change) -> String {
    let show = |field: &str, value: Option<&Value>| -> String {
        match field {
            "meets" => value_meets(value).as_deref().and_then(meets_label),
            "until" => value_date(value).map(short_date),
            _ => value.and_then(text).map(|t| crate::judge::one_line(&t, 80)),
        }
        .filter(|t| !t.is_empty())
        .unwrap_or_else(|| NOT_SET.to_string())
    };
    let mut parts = vec![CHANGE_WHY.to_string()];
    for (key, value) in &c.change {
        let Some(field) = text(key) else { continue };
        if field == "source_uid" {
            continue;
        }
        let label = field_label(&field);
        parts.push(format!(
            "{label}: now {}, was {}.",
            show(&field, Some(value)),
            show(&field, get(&c.was, &field))
        ));
    }
    parts.push(CHANGE_CLOSING.to_string());
    parts.join("\n\n") + "\n"
}

/// The student-facing name of a changed field: `Meets`, `Where`, `Ends`, `Level`; any other field
/// capitalised.
fn field_label(field: &str) -> String {
    match field {
        "meets" => "Meets".to_string(),
        "where" => "Where".to_string(),
        "until" => "Ends".to_string(),
        "level" => "Level".to_string(),
        other => {
            let mut chars = other.chars();
            chars
                .next()
                .map_or_else(String::new, |c| c.to_uppercase().collect::<String>() + chars.as_str())
        }
    }
}

/// What `approvals/` and `archive/` already ask, read once at the start of [`emit_checks`].
#[derive(Default)]
struct Asked {
    /// Every approval's `source_uid`, but a `superseded` or `expired` card's (§5.2, M-f, §5.5).
    keys: BTreeSet<String>,
    /// A card whose `source_uid` starts `window:` and is neither `superseded` nor `expired`
    /// exists (plan review C1).
    window: bool,
    /// `(target, canonical change)` of every change card neither `superseded` nor `expired`
    /// (§5.4, §5.5).
    changes: BTreeSet<(String, String)>,
    /// `commitment-check` cards whose `first_proposed_at` is today, whatever their status.
    today: i64,
}

fn asked(vault: &Path, today: Date) -> Asked {
    let mut out = Asked::default();
    for folder in ["approvals", "archive"] {
        let dir = vault.join(folder);
        if !dir.is_dir() {
            continue;
        }
        for path in crate::approvals::sorted_md(&dir) {
            let Ok(raw) = pystr::read_text(&path) else { continue };
            let Ok((meta, _)) = split_frontmatter(&raw) else { continue };
            let field = |key: &str| field_text(&meta, key).unwrap_or_default();
            if field("type") != "approval" {
                continue;
            }
            let is_check = field("kind") == COMMITMENT_CHECK;
            if is_check
                && crate::approvals::as_date(get(&meta, "first_proposed_at")) == Some(today)
            {
                out.today += 1;
            }
            // Withdrawn (`superseded`) or expired unanswered: the question is still open (M-f, and
            // the §5.5 ruling of fix round 1, as G1 rules for event cards). A card the student
            // deleted is archived with the status it had, and closes the question like an answer.
            if matches!(field("status").as_str(), "superseded" | "expired") {
                continue;
            }
            let key = field("source_uid");
            if key.starts_with(WINDOW_PREFIX) {
                out.window = true;
            }
            if !key.is_empty() {
                out.keys.insert(key);
            }
            let target = field("target");
            if is_check && !target.is_empty() {
                let change = get(&meta, "change").cloned().unwrap_or(Value::Null);
                out.changes.insert((target, canonical(&change)));
            }
        }
    }
    out
}

/// §5.2's order within the proposals: `class`, `lab`, `work`, the window, `club`, `meeting`, any
/// other kind; then the first meeting's `(day, start)`; then `source_uid`.
fn proposal_order(p: &Proposal) -> (usize, usize, Time, &str) {
    let rank = if p.is_window() {
        3
    } else {
        match p.kind.as_str() {
            "class" => 0,
            "lab" => 1,
            "work" => 2,
            "club" => 4,
            "meeting" => 5,
            _ => 6,
        }
    };
    let (day, start) = p.meets.first().map_or((DAY_KEYS.len(), Time::midnight()), |m| {
        let day = m.days.iter().filter_map(|d| DAY_KEYS.iter().position(|k| k == d)).min();
        (day.unwrap_or(DAY_KEYS.len()), m.start)
    });
    (rank, day, start, p.source_uid.as_str())
}

/// File today's `commitment-check` cards (§5.2, §5.4): change cards first, then proposals in
/// [`proposal_order`]. Returns `(paths, count, warnings)`.
///
/// At most `min(budget, 5 − commitment-check cards in approvals/ and archive/ first proposed
/// today)` are filed, so `defer_over_budget` never has overflow to snooze (R9, constraint 7).
/// **Asked once** (§5.5): one snapshot of `approvals/` and `archive/` is read on entry and every
/// key and change filed in this call joins it, so one key never files twice in a run —
/// - a proposal is skipped when any card has its `source_uid` — answered, pending, snoozed, or
///   deleted from the app (archived as it stood) — unless that card is `superseded` or
///   `expired` (M-f; §5.5 as ruled in fix round 1);
/// - the window proposal is skipped while any card neither `superseded` nor `expired` has a
///   `window:` key, **whatever its routine keys** — the window's `source_uid` changes as
///   routines come and go, so it is never compared by its exact value (P9 M6, plan review C1);
/// - a change is skipped when a card neither `superseded` nor `expired` has the same `target`
///   and the same canonical `change` text.
///
/// An `office-hours` proposal is never filed (R8; P9's `for_cards` already leaves them out). A
/// card that cannot be filed is one warning and ends this run's cards, as F2's emitter does (P11
/// review m3): the next run tries again.
#[allow(clippy::too_many_arguments)]
pub fn emit_checks(
    vault: &Path,
    proposals: &[Proposal],
    changes: &[Change],
    today: Date,
    budget: i64,
    ctx: &crate::write::WriteContext,
    journal: &mut crate::journal::Journal,
) -> (Vec<PathBuf>, usize, Vec<String>) {
    let mut filed: Vec<PathBuf> = Vec::new();
    let mut warnings: Vec<String> = Vec::new();
    let mut seen = asked(vault, today);
    let allowance = budget.min(CHECKS_PER_DAY - seen.today).max(0) as usize;
    if allowance == 0 {
        return (filed, 0, warnings);
    }

    let mut ordered_changes: Vec<(String, &Change)> = changes
        .iter()
        .map(|c| (canonical(&Value::Mapping(c.change.clone())), c))
        .collect();
    ordered_changes.sort_by(|a, b| (&a.1.target, &a.0).cmp(&(&b.1.target, &b.0)));
    for (text_of_change, change) in ordered_changes {
        if filed.len() >= allowance {
            break;
        }
        let asked_key = (change.target.clone(), text_of_change);
        if seen.changes.contains(&asked_key) {
            continue;
        }
        let fields = vec![
            ("source_uid", Field::Scalar(Node::text(&change.source_uid))),
            ("target", Field::Scalar(Node::text(&change.target))),
            ("change", Field::Flow(crate::yaml::to_json(&Value::Mapping(change.change.clone())))),
            ("was", Field::Flow(crate::yaml::to_json(&Value::Mapping(change.was.clone())))),
        ];
        let title = change_title(change);
        match file_card(vault, COMMITMENT_CHECK, fields, &title, &change_body(change), today, ctx, journal) {
            Ok(path) => {
                seen.changes.insert(asked_key);
                filed.push(path);
            }
            Err(err) => {
                warnings.push(err);
                let count = filed.len();
                return (filed, count, warnings);
            }
        }
    }

    let mut ordered: Vec<&Proposal> =
        proposals.iter().filter(|p| p.kind != "office-hours").collect();
    ordered.sort_by(|a, b| proposal_order(a).cmp(&proposal_order(b)));
    for proposal in ordered {
        if filed.len() >= allowance {
            break;
        }
        let window = proposal.is_window();
        if (window && seen.window) || seen.keys.contains(&proposal.source_uid) {
            continue;
        }
        let fields = vec![
            ("source_uid", Field::Scalar(Node::text(&proposal.source_uid))),
            ("commitment", Field::Flow(proposal_commitment(proposal))),
        ];
        let title = proposal_title(proposal);
        match file_card(vault, COMMITMENT_CHECK, fields, &title, &proposal_body(proposal), today, ctx, journal) {
            Ok(path) => {
                seen.keys.insert(proposal.source_uid.clone());
                seen.window |= window;
                filed.push(path);
            }
            Err(err) => {
                warnings.push(err);
                break;
            }
        }
    }
    let count = filed.len();
    (filed, count, warnings)
}

// ---------------------------------------------------------------------------------------------
// P12 — change detection: changed, ended, succeeded (§5.4, R22).
// ---------------------------------------------------------------------------------------------

/// The key prefixes a calendar series carries (R6); only notes keyed so are watched for changes.
const SERIES_KEY_PREFIXES: [&str; 2] = ["gcal-series:", "ics-series:"];

/// A `meets` list as the set of `(DAY_KEYS index, start, end)` triples — the comparison §3.5's
/// signature makes, so neither entry order nor day order is a change.
fn meet_set(meets: &[Meet]) -> BTreeSet<(usize, Time, Time)> {
    signature("", None, "", meets, &Codes::default()).meets
}

/// A series is the note's series now and the calendar still returns it: `false` when its
/// calendar's last fresh read did not return it (`last_seen` before that read date) — the same
/// test [`SeriesFile::instances_map`] makes.
fn is_live(file: &SeriesFile, series: &Series) -> bool {
    let read = file.calendars.get(&series.calendar).copied().or(series.last_seen);
    series.last_seen.is_some_and(|seen| read.is_none_or(|read| seen >= read))
}

/// A live, classified series of the file, one per key (R22's "fresh series").
struct Candidate<'a> {
    key: &'a str,
    series: &'a Series,
    kind: String,
    course: Option<String>,
    sig: Signature,
    by_title: Signature,
}

impl Candidate<'_> {
    /// Carries `sig` — by course, or by title for a note with no `course` (P9 fix round 1, M4).
    fn carries(&self, sig: &Signature) -> bool {
        self.sig == *sig || self.by_title == *sig
    }

    /// The date it starts: its `first`, else its earliest instance.
    fn first(&self) -> Option<Date> {
        self.series.first.or_else(|| self.series.instances.iter().map(|i| i.date).min())
    }
}

fn to_value(json: serde_json::Value) -> Value {
    crate::yaml::from_json(&json)
}

fn text_value(text: Option<&str>) -> Value {
    text.map_or(Value::Null, |t| Value::String(t.to_string()))
}

/// §5.4's detection, pure (no clock, no I/O, no model): the changes to confirmed notes keyed
/// `gcal-series:`/`ics-series:`, over [`SeriesFile::by_key`] and `file.ended`. `fresh` is the set
/// of calendar keys read fresh and complete this run. Returns `(changes, warnings)`, in note order.
///
/// - **changed** — the note's record is live on a calendar in `fresh` and differs in `meets`, in
///   a non-empty `where`, or in `until` (a `None` `until` is never proposed). An `until` that ends
///   earlier is not proposed while another live series with the note's signature runs past it —
///   a "this and following" split with the same times (R22: the note is seen).
/// - **ended** — the note's key is under no calendar, is in `file.ended`, and no live series has
///   the note's signature → `until` = `last_instance`, else `until` — but the entry's `until` when
///   it falls on or up to a week after the last instance and before the note's, so a truncated
///   series is asked with the date the live path named (fix round 1, I1); neither → one warning,
///   on the run the entry was dropped, and no change; nothing when the note's `until` is on or
///   before that date. An open-ended series (no `until`) never ends a soft or optional note
///   (fix round 1, m1: a club dormant over the summer is not lost).
/// - **succeeded** (R22) — while the note's series is ending (its record gone stale, its key in
///   `ended`, or its `until` newly earlier than the note's), a live, eligible series of the note's
///   kind (`class` or `lab`) and course, with different `meets`, first meeting on or after the old
///   series' last day, not answered (a note or marker has its key) and not already a confirmed
///   note's signature → one change: `meets`, `where` (when set and different) and `source_uid` =
///   the new key. Twins (one meeting under two keys) share a signature, so one is chosen — the
///   earliest `first`, then [`precedence`], then the key — and one change is made.
///
/// A note whose `until` is before `today` is finished and not watched. `was` holds the note's
/// current value of every changed field, an absent one as `null`; no field is proposed as `null`.
pub fn detect_changes(
    file: &SeriesFile,
    set: &Commitments,
    codes: &Codes,
    planning: &[String],
    fresh: &BTreeSet<String>,
    today: Date,
) -> (Vec<Change>, Vec<String>) {
    let by_key = file.by_key();
    let live: Vec<Candidate> = by_key
        .iter()
        .filter(|(_, series)| is_live(file, series))
        .filter_map(|(key, series)| match classify(series, codes, planning)? {
            Class::Kind { kind, course } => Some(Candidate {
                key,
                series,
                sig: signature(&kind, course.as_deref(), &series.title, &series.meets, codes),
                by_title: signature(&kind, None, &series.title, &series.meets, codes),
                kind,
                course,
            }),
            Class::Routine { .. } => None,
        })
        .collect();
    let answered: BTreeSet<&str> = set
        .confirmed
        .iter()
        .chain(set.planning_day.iter())
        .filter_map(|n| n.source_uid.as_deref())
        .chain(set.declined.iter().map(String::as_str))
        .collect();
    let confirmed_sigs: BTreeSet<Signature> = set.confirmed.iter().map(|n| n.signature(codes)).collect();

    let mut out = Vec::new();
    let mut warnings = Vec::new();
    for note in &set.confirmed {
        let Some(key) = note.source_uid.as_deref() else { continue };
        if !SERIES_KEY_PREFIXES.iter().any(|p| key.starts_with(p)) || note.until.is_some_and(|u| u < today) {
            continue;
        }
        let sig = note.signature(codes);
        let successor = |old_last: Option<Date>| -> Option<Change> {
            let old_last = old_last?;
            let course = course_key(note.course.as_deref()?, codes);
            if !matches!(note.kind.as_str(), "class" | "lab") {
                return None;
            }
            let mine = meet_set(&note.meets);
            let next = live
                .iter()
                .filter(|c| c.key != key && c.kind == note.kind && !c.series.meets.is_empty())
                .filter(|c| c.course.as_deref().is_some_and(|k| course_key(k, codes) == course))
                .filter(|c| meet_set(&c.series.meets) != mine)
                .filter(|c| c.first().is_some_and(|first| first >= old_last))
                .filter(|c| c.series.until.is_none_or(|until| until >= today))
                .filter(|c| !answered.contains(c.key) && !confirmed_sigs.contains(&c.sig))
                .min_by(|a, b| {
                    (a.first(), precedence(a.series), a.key).cmp(&(b.first(), precedence(b.series), b.key))
                })?;
            let mut change = Mapping::new();
            let mut was = Mapping::new();
            change.insert("meets".into(), to_value(meets_json(&next.series.meets)));
            was.insert("meets".into(), to_value(meets_json(&note.meets)));
            if let Some(place) = next.series.where_.as_deref().filter(|w| !w.trim().is_empty()) {
                if note.where_.as_deref().map(str::trim) != Some(place.trim()) {
                    change.insert("where".into(), Value::String(place.trim().to_string()));
                    was.insert("where".into(), text_value(note.where_.as_deref()));
                }
            }
            change.insert("source_uid".into(), Value::String(next.key.to_string()));
            was.insert("source_uid".into(), Value::String(key.to_string()));
            Some(make_change(note, key, change, was))
        };

        match by_key.get(key) {
            Some(series) => {
                if !fresh.contains(&series.calendar) {
                    continue;
                }
                let last_instance = series.instances.iter().map(|i| i.date).max();
                let live_now = is_live(file, series);
                let ends_earlier = series.until.filter(|u| note.until.is_none_or(|n| *u < n));
                if !live_now || ends_earlier.is_some() {
                    let old_last = match live_now {
                        true => series.until.or(last_instance),
                        false => last_instance.or(series.until),
                    };
                    if let Some(change) = successor(old_last) {
                        out.push(change);
                        continue;
                    }
                }
                if !live_now {
                    continue;
                }
                let mut change = Mapping::new();
                let mut was = Mapping::new();
                if !series.meets.is_empty() && meet_set(&series.meets) != meet_set(&note.meets) {
                    change.insert("meets".into(), to_value(meets_json(&series.meets)));
                    was.insert("meets".into(), to_value(meets_json(&note.meets)));
                }
                if let Some(place) = series.where_.as_deref().filter(|w| !w.trim().is_empty()) {
                    if note.where_.as_deref().map(str::trim) != Some(place.trim()) {
                        change.insert("where".into(), Value::String(place.trim().to_string()));
                        was.insert("where".into(), text_value(note.where_.as_deref()));
                    }
                }
                if let Some(until) = series.until.filter(|u| note.until != Some(*u)) {
                    let continues = live.iter().any(|c| {
                        c.key != key && c.carries(&sig) && c.series.until.is_none_or(|other| other > until)
                    });
                    if !continues {
                        change.insert("until".into(), Value::String(until.to_string()));
                        was.insert("until".into(), to_value(date_json(note.until)));
                    }
                }
                if !change.is_empty() {
                    out.push(make_change(note, key, change, was));
                }
            }
            None => {
                let Some(ended) = file.ended.get(key) else { continue };
                if live.iter().any(|c| c.carries(&sig)) {
                    continue;
                }
                // The end the live path named, when the series had been truncated before it went
                // (fix round 1, I1): its `until` on or up to a week after its last instance, and
                // before the note's. A series that vanished whole keeps its last instance.
                let end = match (ended.last_instance, ended.until) {
                    (Some(last), Some(until))
                        if until >= last
                            && days_since(last, until) < TRUNCATION_DAYS
                            && note.until.is_none_or(|n| until < n) =>
                    {
                        Some(until)
                    }
                    (last, until) => last.or(until),
                };
                if let Some(change) = successor(end) {
                    out.push(change);
                    continue;
                }
                // An open-ended series (no UNTIL, no COUNT) that stops appearing never ends a soft
                // or optional note by itself (fix round 1, m1): a club dormant over the summer is
                // not lost. The student can delete the note.
                if ended.until.is_none() && note.level != Level::Hard {
                    continue;
                }
                let Some(end) = end else {
                    // Once, on the run the entry was dropped (fix round 1, m3).
                    if ended.dropped == today {
                        warnings.push(format!(
                            "commitments: {key} ended with no last instance and no until; no end card"
                        ));
                    }
                    continue;
                };
                if note.until.is_some_and(|u| u <= end) {
                    continue;
                }
                let mut change = Mapping::new();
                let mut was = Mapping::new();
                change.insert("until".into(), Value::String(end.to_string()));
                was.insert("until".into(), to_value(date_json(note.until)));
                out.push(make_change(note, key, change, was));
            }
        }
    }
    (out, warnings)
}

/// How far after its last instance an ended series' `until` may fall and still be the end the
/// live path proposed (fix round 1, I1): a truncating UNTIL lands within the week of the last
/// meeting; a series deleted whole keeps its old, far term end.
const TRUNCATION_DAYS: i64 = 7;

fn make_change(note: &Commitment, key: &str, change: Mapping, was: Mapping) -> Change {
    Change {
        target: note.path.to_string_lossy().replace('\\', "/"),
        source_uid: key.to_string(),
        title: note.title.clone(),
        change,
        was,
    }
}

/// The successor keys a change card holds (§5.4, §5.5, plan review I5): the `change.source_uid`
/// of every `commitment-check` card in `approvals/` and `archive/` whose status is neither
/// `superseded` nor `expired` — pending, snoozed, approved or rejected. `rank` passes them to
/// [`proposals`] as `held`, so a successor is never proposed as a class of its own beside its
/// change card. `expired` is open as in P11's asked-once index (fix round 1, m2). P13 stamps a
/// change card whose `was` no longer matches the note `superseded` (§5.5), never `refused`, so a
/// stale card releases its successor key here and the change can be asked again.
pub fn successor_keys(vault: &Path) -> BTreeSet<String> {
    let mut out = BTreeSet::new();
    for folder in ["approvals", "archive"] {
        let dir = vault.join(folder);
        if !dir.is_dir() {
            continue;
        }
        for path in crate::approvals::sorted_md(&dir) {
            let Ok(raw) = pystr::read_text(&path) else { continue };
            let Ok((meta, _)) = split_frontmatter(&raw) else { continue };
            let field = |key: &str| field_text(&meta, key).unwrap_or_default();
            if field("type") != "approval" || field("kind") != COMMITMENT_CHECK || matches!(field("status").as_str(), "superseded" | "expired") {
                continue;
            }
            let Some(Value::Mapping(change)) = get(&meta, "change") else { continue };
            if let Some(key) = get(change, "source_uid").and_then(text).filter(|k| !k.trim().is_empty()) {
                out.insert(key);
            }
        }
    }
    out
}

// ---------------------------------------------------------------------------------------------
// P13 — settlement: the confirmed note, the decline marker, the change, withdrawal (§2.3, §2.5,
// §5.2, §5.4, §5.5). `approvals::transition_note` and `approvals::withdraw_stale` call in here.
// ---------------------------------------------------------------------------------------------

/// The only fields a change card may write (§5.4, §2.5): `kind`, `level`, `title`, `course` and
/// `status` are never written by an agent after creation, so a card naming anything else — only
/// a hand edit can — is refused.
pub const CHANGE_FIELDS: [&str; 4] = ["meets", "where", "until", "source_uid"];

/// What settling an approved `commitment-check` card decided; the arm stamps the card with it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Settled {
    /// The note was created, or the change written: stamped `executed`.
    Executed,
    /// Nothing was written, and never will be for this card: stamped `refused`, one warning.
    Refused(String),
    /// The question went away before the answer landed — the target is gone, or the student
    /// edited the note since the card was filed: stamped `superseded`, one warning. Not
    /// `refused`, so the asked-once index and P12's `successor_keys` treat the question as still
    /// open and a change is asked once more with the new `was` (§5.5).
    Superseded(String),
}

/// The three shapes of `commitment-check` card (plan review C1, spec §5.2 as amended).
enum Shape {
    /// It has a `target`: a change to a confirmed note (§5.4).
    Change(String),
    /// Its `source_uid` starts [`WINDOW_PREFIX`].
    Window,
    /// Neither: a proposal, keyed by its `source_uid`.
    Proposal(String),
}

fn shape(meta: &Mapping) -> Shape {
    let target = field_text(meta, "target").map(|t| t.trim().to_string()).unwrap_or_default();
    let key = field_text(meta, "source_uid").map(|t| t.trim().to_string()).unwrap_or_default();
    if !target.is_empty() {
        Shape::Change(target)
    } else if key.starts_with(WINDOW_PREFIX) {
        Shape::Window
    } else {
        Shape::Proposal(key)
    }
}

/// `commitments/declined-<first 10 hex of sha256(key)>` (§2.3), the stem only. The digest is
/// `ring`'s — the same direct `ring` edge C3′ adds for the sync content hash; `ring` is already
/// linked through `ureq` → `rustls`, so it costs no crate.
fn marker_stem(key: &str) -> String {
    let digest = ring::digest::digest(&ring::digest::SHA256, key.as_bytes());
    let hex: String = digest.as_ref().iter().map(|b| format!("{b:02x}")).collect();
    format!("declined-{}", &hex[..10])
}

/// `commitments/<stem>.md`, `-2`, `-3` on a collision (as `approvals::materialize` does).
fn free_rel(vault: &Path, stem: &str) -> String {
    let folder = vault.join(FOLDER);
    let mut name = format!("{stem}.md");
    let mut suffix = 2;
    while folder.join(&name).exists() {
        name = format!("{stem}-{suffix}.md");
        suffix += 1;
    }
    format!("{FOLDER}/{name}")
}

fn io(err: String) -> crate::write::WriteError {
    crate::write::WriteError::Io(err)
}

/// Write the decline marker for `key` (§2.3): `id`, `type: commitment`, `status: declined` and
/// `source_uid`, nothing else — no title, time, place or body. A no-op (`Ok(None)`) when a
/// marker for the key exists already. Actor `agent:commitments`: the student's decision is the
/// journal record on the card (§2.5).
pub fn create_marker(
    vault: &Path,
    key: &str,
    ctx: &crate::write::WriteContext,
    journal: &mut crate::journal::Journal,
) -> Result<Option<PathBuf>, crate::write::WriteError> {
    if load(vault).declined.contains(key) {
        return Ok(None);
    }
    let front = front_matter(&[
        ("id", Field::Scalar(Node::text(&crate::ids::new_id("cmt")))),
        ("type", Field::Scalar(Node::text("commitment"))),
        ("status", Field::Scalar(Node::text("declined"))),
        ("source_uid", Field::Scalar(Node::text(&single_line(key)))),
    ])
    .map_err(|e| io(format!("marker not written: {e}")))?;
    let rel = free_rel(vault, &marker_stem(key));
    let path = crate::write::create(
        vault,
        &rel,
        &format!("---\n{front}---\n"),
        &ctx.with_actor(CARD_ACTOR),
        journal,
        None,
    )?;
    Ok(Some(path))
}

/// A card's `commitment:` mapping read as §2.2 reads a note, or why it cannot be a note.
struct Proposed {
    kind: String,
    level: Level,
    title: String,
    course: Option<String>,
    meets: Vec<Meet>,
    where_: Option<String>,
    from: Option<Date>,
    until: Option<Date>,
}

fn opt_field(map: &Mapping, key: &str) -> Option<String> {
    field_text(map, key).map(|t| single_line(&t).trim().to_string()).filter(|t| !t.is_empty())
}

fn opt_date(map: &Mapping, key: &str) -> Result<Option<Date>, String> {
    match opt_field(map, key) {
        None => Ok(None),
        Some(raw) => raw.parse::<Date>().map(Some).map_err(|_| format!("{key} {raw} is not a date")),
    }
}

/// Every entry of `key` (a `meets` or `window` sequence), all valid, at least one.
fn strict_meets(map: &Mapping, key: &str) -> Result<Vec<Meet>, String> {
    let Some(Value::Sequence(items)) = get(map, key) else {
        return Err(format!("no {key}"));
    };
    let meets: Vec<Meet> = items
        .iter()
        .map(|item| parse_entry(item).map_err(|(d, s, e)| format!("{key} entry {d}: {s}–{e} invalid")))
        .collect::<Result<_, _>>()?;
    if meets.is_empty() {
        return Err(format!("no {key}"));
    }
    Ok(meets)
}

fn proposed(map: &Mapping) -> Result<Proposed, String> {
    let kind = opt_field(map, "kind").unwrap_or_default();
    if kind == PLANNING_DAY {
        return Ok(Proposed {
            kind,
            level: Level::Optional,
            title: WINDOW_TITLE.to_string(),
            course: None,
            meets: strict_meets(map, "window")?,
            where_: None,
            from: None,
            until: None,
        });
    }
    if kind.is_empty() {
        return Err("commitment has no kind".to_string());
    }
    let title = opt_field(map, "title").ok_or("commitment has no title")?;
    let level = opt_field(map, "level")
        .and_then(|l| Level::parse(&l))
        .or_else(|| default_level(&kind))
        .unwrap_or(Level::Soft);
    Ok(Proposed {
        level,
        title: cut(&title, TITLE_MAX),
        course: opt_field(map, "course"),
        meets: strict_meets(map, "meets")?,
        where_: opt_field(map, "where").map(|w| cut(&w, WHERE_MAX)),
        from: opt_date(map, "from")?,
        until: opt_date(map, "until")?,
        kind,
    })
}

/// The confirmed note a card's `commitment:` describes (§2.1), or — for `kind: planning-day` —
/// the planning day (§2.4): `write::create`, journal first, actor `agent:commitments`. Every line
/// through [`front_matter`], so `meets:` and `window:` are each one line (Global Constraint 23);
/// an absent optional field is left out, never written `null`. `commitments/<slugify(title)>.md`
/// (`planning-day.md` for the window), `-2` on a collision. `source_uid` is the card's; the
/// planning day carries none (§2.4). Nothing here checks for duplicates — the settlement does.
pub fn create_confirmed(
    vault: &Path,
    commitment: &Mapping,
    source_uid: &str,
    today: Date,
    ctx: &crate::write::WriteContext,
    journal: &mut crate::journal::Journal,
) -> Result<PathBuf, crate::write::WriteError> {
    let p = proposed(commitment).map_err(|e| io(format!("commitment not created: {e}")))?;
    let id = crate::ids::new_id("cmt");
    let mut fields: Vec<(&str, Field)> = vec![
        ("id", Field::Scalar(Node::text(&id))),
        ("type", Field::Scalar(Node::text("commitment"))),
        ("kind", Field::Scalar(Node::text(&p.kind))),
    ];
    let (stem, body) = if p.kind == PLANNING_DAY {
        fields.push(("status", Field::Scalar(Node::text("confirmed"))));
        fields.push(("window", Field::Flow(meets_json(&p.meets))));
        (
            PLANNING_DAY.to_string(),
            "The part of each day Knowlu plans in: from when you are up to when you stop.\n",
        )
    } else {
        fields.push(("level", Field::Scalar(Node::text(p.level.as_str()))));
        fields.push(("title", Field::Scalar(Node::text(&p.title))));
        if let Some(course) = &p.course {
            fields.push(("course", Field::Scalar(Node::text(course))));
        }
        fields.push(("meets", Field::Flow(meets_json(&p.meets))));
        if let Some(place) = &p.where_ {
            fields.push(("where", Field::Scalar(Node::text(place))));
        }
        if let Some(from) = p.from {
            fields.push(("from", Field::Scalar(Node::Date(from))));
        }
        if let Some(until) = p.until {
            fields.push(("until", Field::Scalar(Node::Date(until))));
        }
        let key = single_line(source_uid).trim().to_string();
        let body = if key.starts_with("gcal-series:") {
            "Found as a weekly series on your Google Calendar.\n"
        } else {
            "Found as a weekly series on your calendar.\n"
        };
        if !key.is_empty() {
            fields.push(("source_uid", Field::Scalar(Node::text(&key))));
        }
        fields.push(("status", Field::Scalar(Node::text("confirmed"))));
        (crate::ingest::slugify(&p.title), body)
    };
    fields.push(("confirmed_at", Field::Scalar(Node::Date(today))));
    let front = front_matter(&fields).map_err(|e| io(format!("commitment not created: {e}")))?;
    let rel = free_rel(vault, &stem);
    crate::write::create(
        vault,
        &rel,
        &format!("---\n{front}---\n\n{body}"),
        &ctx.with_actor(CARD_ACTOR),
        journal,
        None,
    )
}

/// A field's value as `load` would read it, as comparable text (carry-forward 2): `meets` and
/// `window` as the set of their **valid** entries' `(day, start, end)` triples (so `9:00` is
/// `09:00`, and neither entry nor day order matters); `until`/`from` as a date, an unparseable one
/// as absent (load warns and ignores it); any other field exactly as `load`'s `field_text` reads
/// it — a scalar, trimmed with its whitespace runs made one space, and a **collection as absent**
/// (fix round 1, I1: P12 writes `was: {where: null}` for a note whose `where:` is a list, so any
/// other reading would supersede and re-file that card on every rank). Absent, `null` and empty
/// alike are `""`.
fn as_read(field: &str, value: Option<&Value>) -> String {
    let Some(value) = value.filter(|v| !matches!(v, Value::Null)) else {
        return String::new();
    };
    match (field, value) {
        ("meets" | "window", Value::Sequence(items)) => {
            let meets: Vec<Meet> = items.iter().filter_map(|v| parse_entry(v).ok()).collect();
            format!("{:?}", meet_set(&meets))
        }
        ("until" | "from", _) => value_date(Some(value)).map(|d| d.to_string()).unwrap_or_default(),
        _ => text(value)
            .map(|t| t.split(pystr::is_python_space).filter(|p| !p.is_empty()).collect::<Vec<_>>().join(" "))
            .unwrap_or_default(),
    }
}

/// A change card's `target`, only when it is exactly `commitments/<name>.md` (no deeper path, no
/// `..`): a hand-edited card can point nowhere else.
fn target_rel(target: &str) -> Option<&str> {
    let name = target.strip_prefix("commitments/")?;
    let ok = name.ends_with(".md")
        && name.len() > 3
        && !name.contains(['/', '\\'])
        && !name.starts_with('.');
    ok.then_some(target)
}

/// Whether the student changed the note since the card was filed (§5.4, §5.5): the target is
/// gone, unreadable, or for some field of `change` its current value, as read, differs from
/// `was`'s. `None` when the note still holds `was` everywhere.
fn stale_change(vault: &Path, target: &str, meta: &Mapping) -> Option<String> {
    let rel = target_rel(target)?;
    let path = vault.join(rel);
    if !path.is_file() {
        return Some(format!("{rel} is gone"));
    }
    let note = match pystr::read_text(&path).map(|t| split_frontmatter(&t)) {
        Ok(Ok((note, _))) => note,
        _ => return Some(format!("{rel} is unreadable")),
    };
    let change = match get(meta, "change") {
        Some(Value::Mapping(m)) => m.clone(),
        _ => Mapping::new(),
    };
    let was = match get(meta, "was") {
        Some(Value::Mapping(m)) => m.clone(),
        _ => Mapping::new(),
    };
    for key in change.keys() {
        let Some(field) = text(key) else { continue };
        if as_read(&field, get(&note, &field)) != as_read(&field, get(&was, &field)) {
            return Some(format!("{rel}: {field} was edited since the card was filed"));
        }
    }
    None
}

/// The literals an approved change writes, or why it cannot be written: a target that is not a
/// note in `commitments/`, an empty `change`, a field outside [`CHANGE_FIELDS`], a `null` value,
/// or a `meets` with an invalid entry.
fn change_literals(target: &str, meta: &Mapping) -> Result<Vec<(String, String)>, String> {
    if target_rel(target).is_none() {
        return Err(format!("target {} is not a note in commitments/", single_line(target)));
    }
    let Some(Value::Mapping(change)) = get(meta, "change") else {
        return Err("change is not a mapping".to_string());
    };
    if change.is_empty() {
        return Err("change is empty".to_string());
    }
    let mut out = Vec::new();
    for (key, value) in change {
        let field = text(key).unwrap_or_default();
        if !CHANGE_FIELDS.contains(&field.as_str()) {
            return Err(format!("{} is not a field a change may write", single_line(&field)));
        }
        let usable = match (field.as_str(), value) {
            (_, Value::Null) => false,
            ("meets", Value::Sequence(items)) => {
                !items.is_empty() && items.iter().all(|i| parse_entry(i).is_ok())
            }
            ("meets", _) => false,
            ("until", _) => value_date(Some(value)).is_some(),
            (_, v) => text(v).is_some_and(|t| !t.trim().is_empty()),
        };
        if !usable {
            return Err(format!("{field}: not a value the note can hold"));
        }
        // A sequence through `to_literal` of the parsed value, so it reads back as the sequence
        // `load` accepts (§5.4); a scalar the same way (a date stays a bare date).
        let literal = match (field.as_str(), value) {
            ("until", _) => value_date(Some(value)).map(|d| d.to_string()).unwrap_or_default(),
            ("meets", _) => crate::write::to_literal(value),
            (_, v) => crate::write::to_literal(&Value::String(single_line(&text(v).unwrap_or_default()).trim().to_string())),
        };
        if pystr::splitlines(&literal).len() != 1 {
            return Err(format!("{field}: would span more than one line"));
        }
        out.push((field, literal));
    }
    Ok(out)
}

/// The signatures that close a card's question: its `commitment:`'s, by course and by title,
/// as [`proposals`] closes a signature.
fn card_signatures(meta: &Mapping, codes: &Codes) -> Vec<Signature> {
    let Some(Value::Mapping(map)) = get(meta, "commitment") else {
        return Vec::new();
    };
    let Ok(p) = proposed(map) else { return Vec::new() };
    if p.kind == PLANNING_DAY {
        return Vec::new();
    }
    vec![
        signature(&p.kind, p.course.as_deref(), &p.title, &p.meets, codes),
        signature(&p.kind, None, &p.title, &p.meets, codes),
    ]
}

/// `key` and its twins (carry-forward 1): every key of the series file whose series classifies
/// to a signature in `sigs`, or to the signature `key`'s own series has — one real series under a
/// `gcal-series:` and an `ics-series:` key (P9's twins). Classified with no planning names: a
/// twin the template's names would keep from being proposed gets a marker it does not need,
/// which closes nothing that was open.
fn twin_keys(vault: &Path, key: &str, mut sigs: Vec<Signature>, codes: &Codes) -> BTreeSet<String> {
    let (file, _) = read_series_file(vault);
    let by_key = file.by_key();
    let classified = |series: &Series| match classify(series, codes, &[]) {
        Some(Class::Kind { kind, course }) => Some((
            signature(&kind, course.as_deref(), &series.title, &series.meets, codes),
            signature(&kind, None, &series.title, &series.meets, codes),
        )),
        _ => None,
    };
    if let Some((sig, by_title)) = by_key.get(key).and_then(|s| classified(s)) {
        sigs.push(sig);
        sigs.push(by_title);
    }
    let mut out: BTreeSet<String> = [key.to_string()].into_iter().collect();
    for (other, series) in by_key {
        if let Some((sig, by_title)) = classified(series) {
            if sigs.contains(&sig) || sigs.contains(&by_title) {
                out.insert(other.to_string());
            }
        }
    }
    out
}

/// Whether the note already holds every value the card's `change` names, as read (fix round 1,
/// I2): the settlement wrote it and the run died before the card's stamp.
fn change_applied(vault: &Path, target: &str, meta: &Mapping) -> bool {
    let Some(rel) = target_rel(target) else { return false };
    let Ok(Ok((note, _))) = pystr::read_text(&vault.join(rel)).map(|t| split_frontmatter(&t)) else {
        return false;
    };
    let Some(Value::Mapping(change)) = get(meta, "change") else { return false };
    !change.is_empty()
        && change.iter().all(|(key, value)| {
            let field = text(key).unwrap_or_default();
            as_read(&field, get(&note, &field)) == as_read(&field, Some(value))
        })
}

fn record_str<'a>(record: &'a crate::ledger::Record, key: &str) -> Option<&'a str> {
    record.get(key).and_then(|v| v.as_str())
}

/// Whether this card's own settlement created the note it now finds (fix round 1, I2): in the
/// journal's order, after the last record setting the card's `status` to `approved`, a `create`
/// of a note in `commitments/` by `agent:commitments` on this device whose `new` matches — its
/// `source_uid` is the card's key, or for the window its `kind` is `planning-day` — and whose file
/// still exists. A note confirmed before the student approved (another desktop's answer, synced
/// in) has its record earlier, so it is still refused.
fn created_by_this_card(
    vault: &Path,
    meta: &Mapping,
    key: Option<&str>,
    journal: &mut crate::journal::Journal,
) -> bool {
    let Some(card_id) = field_text(meta, "id").filter(|id| !id.is_empty()) else { return false };
    let records = journal.read(None, None);
    let Some(approved) = records.iter().rposition(|r| {
        record_str(r, "id") == Some(card_id.as_str())
            && record_str(r, "op") == Some("set")
            && record_str(r, "field") == Some("status")
            && record_str(r, "new") == Some("approved")
    }) else {
        return false;
    };
    let device = crate::journal::device_name();
    records[approved + 1..].iter().any(|r| {
        let path = record_str(r, "path").unwrap_or_default();
        let new = r.get("new").and_then(|n| n.as_object());
        let matches = new.is_some_and(|n| match key {
            Some(key) => n.get("source_uid").and_then(|v| v.as_str()) == Some(key),
            None => n.get("kind").and_then(|v| v.as_str()) == Some(PLANNING_DAY),
        });
        record_str(r, "op") == Some("create")
            && record_str(r, "actor") == Some(CARD_ACTOR)
            && record_str(r, "device") == Some(device.as_str())
            && path.starts_with("commitments/")
            && matches
            && vault.join(path).is_file()
    })
}

/// Settle an approved `commitment-check` card (§5.2, §5.4). Writes nothing to the card itself —
/// the arm stamps it with the result and archives it.
///
/// - **change** → [`Settled::Refused`] for a card the note cannot take (see `change_literals`);
///   [`Settled::Superseded`] when the target is gone or the student edited a changed field since
///   (carry-forward 3); else each field through `write::write_literals`, actor
///   `agent:commitments`.
/// - **window** → refused when a `planning-day` note exists; else the planning day.
/// - **proposal** → refused when a confirmed note has the card's key or signature (§2.5: a
///   second desktop's answer is a no-op), or the `commitment:` cannot be a note; else the note.
pub fn settle_approved(
    vault: &Path,
    meta: &Mapping,
    today: Date,
    ctx: &crate::write::WriteContext,
    journal: &mut crate::journal::Journal,
) -> Result<Settled, crate::write::WriteError> {
    let set = load(vault);
    let commitment = match get(meta, "commitment") {
        Some(Value::Mapping(m)) => Some(m),
        _ => None,
    };
    match shape(meta) {
        Shape::Change(target) => {
            let literals = match change_literals(&target, meta) {
                Ok(literals) => literals,
                Err(why) => return Ok(Settled::Refused(why)),
            };
            if change_applied(vault, &target, meta) {
                return Ok(Settled::Executed);
            }
            if let Some(why) = stale_change(vault, &target, meta) {
                return Ok(Settled::Superseded(format!("{why}; not applied")));
            }
            crate::write::write_literals(
                vault,
                &target,
                &literals,
                &ctx.with_actor(CARD_ACTOR),
                journal,
                &crate::write::WriteOpts::default(),
            )?;
            Ok(Settled::Executed)
        }
        Shape::Window => {
            if let Some(day) = &set.planning_day {
                if created_by_this_card(vault, meta, None, journal) {
                    return Ok(Settled::Executed);
                }
                return Ok(Settled::Refused(format!("{} already sets the planning day", day.path.display())));
            }
            let Some(map) = commitment else {
                return Ok(Settled::Refused("the card has no commitment".to_string()));
            };
            if let Err(why) = proposed(map).and_then(|p| match p.kind == PLANNING_DAY {
                true => Ok(()),
                false => Err("a window card's commitment is not a planning day".to_string()),
            }) {
                return Ok(Settled::Refused(why));
            }
            create_confirmed(vault, map, "", today, ctx, journal)?;
            Ok(Settled::Executed)
        }
        Shape::Proposal(key) => {
            let Some(map) = commitment else {
                return Ok(Settled::Refused("the card has no commitment".to_string()));
            };
            match proposed(map) {
                Err(why) => return Ok(Settled::Refused(why)),
                Ok(p) if p.kind == PLANNING_DAY => {
                    return Ok(Settled::Refused("a planning day needs a window: key".to_string()))
                }
                Ok(_) => {}
            }
            let (codes, _) = Codes::load(vault);
            let sigs = card_signatures(meta, &codes);
            let held = set.confirmed.iter().find(|n| {
                (!key.is_empty() && n.source_uid.as_deref() == Some(key.as_str()))
                    || sigs.contains(&n.signature(&codes))
            });
            if let Some(note) = held {
                if !key.is_empty() && created_by_this_card(vault, meta, Some(&key), journal) {
                    return Ok(Settled::Executed);
                }
                return Ok(Settled::Refused(format!(
                    "{} is already confirmed; nothing written",
                    note.path.display().to_string().replace('\\', "/")
                )));
            }
            create_confirmed(vault, map, &key, today, ctx, journal)?;
            Ok(Settled::Executed)
        }
    }
}

/// Settle a rejected `commitment-check` card (§5.2, §5.4, §5.5): a proposal's decline marker —
/// one for its key **and each twin's** (carry-forward 1) — the window's one `window` marker, or,
/// for a change card whose `change` carries a new `source_uid`, that successor's marker (and its
/// twins'); a plain change writes nothing. Returns the markers written.
pub fn settle_rejected(
    vault: &Path,
    meta: &Mapping,
    ctx: &crate::write::WriteContext,
    journal: &mut crate::journal::Journal,
) -> Result<Vec<PathBuf>, crate::write::WriteError> {
    let keys: BTreeSet<String> = match shape(meta) {
        Shape::Window => [WINDOW_MARKER.to_string()].into_iter().collect(),
        Shape::Proposal(key) if key.is_empty() => BTreeSet::new(),
        Shape::Proposal(key) => {
            let (codes, _) = Codes::load(vault);
            twin_keys(vault, &key, card_signatures(meta, &codes), &codes)
        }
        Shape::Change(_) => {
            let successor = match get(meta, "change") {
                Some(Value::Mapping(change)) => opt_field(change, "source_uid"),
                _ => None,
            };
            match successor {
                Some(key) => {
                    let (codes, _) = Codes::load(vault);
                    twin_keys(vault, &key, Vec::new(), &codes)
                }
                None => BTreeSet::new(),
            }
        }
    };
    // Fix round 1, m5: a key a confirmed note already holds (another desktop confirmed it) gets no
    // marker — the note still counts, and a marker beside it would say the opposite.
    let confirmed: BTreeSet<String> =
        load(vault).confirmed.into_iter().filter_map(|n| n.source_uid).collect();
    let mut written = Vec::new();
    for key in keys.into_iter().filter(|k| !confirmed.contains(k)) {
        written.extend(create_marker(vault, &key, ctx, journal)?);
    }
    Ok(written)
}

/// Why a pending or snoozed `commitment-check` card's question went away (§5.2 "Withdrawn", by
/// card shape — plan review C1), or `None` while it stands:
/// - **change**: its target is gone, or the note no longer holds `was` (the settlement's test);
/// - **window**: a `planning-day` note or the `window` marker exists;
/// - **proposal**: its key is under no calendar of the series file, its series is in the file's
///   `ended` map or ends before `today` (fix round 1, m3), a note (confirmed, a marker,
///   the planning day) has its key, or a confirmed note has its signature. "Under no calendar"
///   is read only from a file that knows at least one calendar: an empty file (never written,
///   or unreadable this run) says nothing about what left it.
pub fn withdrawal_reason(
    vault: &Path,
    meta: &Mapping,
    file: &SeriesFile,
    set: &Commitments,
    codes: &Codes,
    today: Date,
) -> Option<String> {
    match shape(meta) {
        Shape::Change(target) => stale_change(vault, &target, meta),
        Shape::Window => {
            if set.planning_day.is_some() {
                Some("a planning day exists".to_string())
            } else if set.declined.contains(WINDOW_MARKER) {
                Some("the window was declined".to_string())
            } else {
                None
            }
        }
        Shape::Proposal(key) => {
            if key.is_empty() {
                return None;
            }
            let by_key = file.by_key();
            if !file.calendars.is_empty() && !by_key.contains_key(key.as_str()) {
                return Some(format!("{key} left the calendar"));
            }
            // Fix round 1, m3: its series ended (the file's `ended` map), or now ends before
            // `today` — P9 proposes neither, so the question is gone.
            if file.ended.contains_key(&key) {
                return Some(format!("{key} ended"));
            }
            if by_key.get(key.as_str()).is_some_and(|s| s.until.is_some_and(|until| until < today)) {
                return Some(format!("{key} has finished"));
            }
            let noted = set
                .confirmed
                .iter()
                .chain(set.planning_day.iter())
                .any(|n| n.source_uid.as_deref() == Some(key.as_str()))
                || set.declined.contains(&key);
            if noted {
                return Some(format!("{key} was answered"));
            }
            let sigs = card_signatures(meta, codes);
            if set.confirmed.iter().any(|n| sigs.contains(&n.signature(codes))) {
                return Some(format!("{key} is already confirmed"));
            }
            None
        }
    }
}

// ---------------------------------------------------------------------------------------------
// P16 — what `rank` needs beside the passes above: the series file's read outcome and the day's
// first window (§6.4).
// ---------------------------------------------------------------------------------------------

/// Whether [`refresh_series`]' warnings say the series file could not be **read** — malformed or
/// unreadable, never a failed write. The returned file then holds only this run's fresh calendars,
/// so a key held under an unread calendar would look as if it left the file: `rank` skips card
/// withdrawal and the **ended** rule that run (P8 review; never infer "gone" from a bad read).
pub fn series_read_failed(warnings: &[String]) -> bool {
    let malformed = format!("series file: {SERIES_FILE} malformed (");
    let unreadable = format!("series file: {SERIES_FILE} unreadable (");
    warnings.iter().any(|w| w.starts_with(&malformed) || w.starts_with(&unreadable))
}

/// `code_table`'s warnings that matter to this run (P16 fix round 1, M1, controller ruling): a
/// code two courses claim is kept only when a series in `file` carries it — its title, compacted
/// as codes are, contains the code — so a vault that merely has clashing `courses/` notes does not
/// turn the `calendar` step WARN on every run. Every other code-table warning goes nowhere.
pub fn code_warnings_hit(warnings: Vec<String>, file: &SeriesFile) -> Vec<String> {
    let titles: Vec<String> = file.series.iter().map(|s| compact(&s.title)).collect();
    warnings
        .into_iter()
        .filter(|w| {
            let code = w.strip_prefix("code ").and_then(|rest| rest.split_once(": claimed by both "));
            code.is_some_and(|(code, _)| !code.is_empty() && titles.iter().any(|t| t.contains(code)))
        })
        .collect()
}

/// The day's first window (§6.4), generated and device-local.
pub const PLAN_FILE: &str = "state/plan.json";

/// §6.4, as `rank` runs it: only when a confirmed `planning-day` note exists (so a vault without
/// one — every fixture — gains no file). The file's `date` is today: left alone, so the baseline
/// stays the day's first window however often the note is edited. Another day: today's current
/// `cal.window(today)`. Missing or malformed: the **template's** `(day_start, day_end)` — the
/// window the day was planned in before any note existed (re-review N2). An I/O error other than
/// not-found leaves the file alone with one warning (it may be fine). Written as
/// `{"date", "start", "end"}` through `ledger::dumps_value` with a trailing newline, whole (temp
/// file, then rename). No diff here:
/// `moved` is `surface`'s (P18). Returns warnings.
pub fn record_baseline(
    vault: &Path,
    set: &Commitments,
    cal: &crate::weekcal::WeekCalendar,
    today: Date,
) -> Vec<String> {
    if set.planning_day.is_none() {
        return Vec::new();
    }
    let path = vault.join("state").join("plan.json");
    let recorded: Option<Date> = match std::fs::read_to_string(&path) {
        Ok(text) => serde_json::from_str::<serde_json::Value>(&text)
            .ok()
            .and_then(|v| v.get("date")?.as_str()?.parse::<Date>().ok()),
        Err(err) if matches!(err.kind(), std::io::ErrorKind::NotFound | std::io::ErrorKind::InvalidData) => None,
        Err(err) => return vec![format!("plan file: {PLAN_FILE} unreadable ({}); not written this run", err.kind())],
    };
    let (start, end) = match recorded {
        Some(day) if day == today => return Vec::new(),
        Some(_) => cal.window(today),
        None => (cal.day_start, cal.day_end),
    };
    let value = serde_json::json!({ "date": today.to_string(), "start": hm(start), "end": hm(end) });
    let bytes = crate::ledger::dumps_value(&value) + "\n";
    match write_state_file(vault, "plan.json", &bytes) {
        Ok(()) => Vec::new(),
        Err(err) => vec![format!("plan file: could not write {PLAN_FILE} ({})", err.kind())],
    }
}

/// §6.4, as `surface` reads it, purely: the window today's plan is diffed against. No
/// `planning-day` note: `None` (no baseline, no `moved`). `state/plan.json` dated today: its
/// window. Missing or unreadable (a malformed file, a bad time, `start >= end`): the template's
/// `(day_start, day_end)` — the first window of a day is diffed against the template (re-review
/// N2). Dated another day (today's first `rank` has not run yet): the current `cal.window(today)`,
/// so nothing is reported. Never writes.
pub fn baseline(
    vault: &Path,
    set: &Commitments,
    cal: &crate::weekcal::WeekCalendar,
    today: Date,
) -> Option<(Time, Time)> {
    set.planning_day.as_ref()?;
    let template = (cal.day_start, cal.day_end);
    let Ok(text) = std::fs::read_to_string(vault.join("state").join("plan.json")) else {
        return Some(template);
    };
    let Ok(value) = serde_json::from_str::<serde_json::Value>(&text) else {
        return Some(template);
    };
    let field = |key: &str| value.get(key).and_then(|v| v.as_str()).map(str::to_string);
    match field("date").and_then(|d| d.parse::<Date>().ok()) {
        Some(day) if day == today => {
            let start = field("start").as_deref().and_then(parse_time);
            let end = field("end").as_deref().and_then(parse_time);
            match (start, end) {
                (Some(start), Some(end)) if start < end => Some((start, end)),
                _ => Some(template),
            }
        }
        Some(_) => Some(cal.window(today)),
        None => Some(template),
    }
}

/// A take's part of the day, from its free block's start (§6.4): before 12:00 morning, before
/// 17:00 afternoon, otherwise evening.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum Part {
    Morning,
    Afternoon,
    Evening,
}

pub fn part_of_day(start: Time) -> Part {
    if start < Time::constant(12, 0, 0, 0) {
        Part::Morning
    } else if start < Time::constant(17, 0, 0, 0) {
        Part::Afternoon
    } else {
        Part::Evening
    }
}

/// How many of today's takes moved to each part of the day (§6.4).
#[derive(Debug, Clone, Default, PartialEq, Eq, serde::Serialize)]
pub struct MovedTo {
    pub morning: usize,
    pub afternoon: usize,
    pub evening: usize,
}

/// §6.4's `moved`: `{"to": {...}, "dropped": n, "text": "..."}`, the today view's line.
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize)]
pub struct Moved {
    pub to: MovedTo,
    pub dropped: usize,
    pub text: String,
}

impl Moved {
    /// `None` when nothing moved. `text` names the largest group first, ties in morning →
    /// evening order, and appends `; n no longer fit(s) today` for `dropped`.
    pub fn from_counts(to: MovedTo, dropped: usize) -> Option<Moved> {
        let mut groups: Vec<(usize, &str)> = [
            (to.morning, "this morning"),
            (to.afternoon, "this afternoon"),
            (to.evening, "this evening"),
        ]
        .into_iter()
        .filter(|(n, _)| *n > 0)
        .collect();
        if groups.is_empty() && dropped == 0 {
            return None;
        }
        // Stable: equal counts keep morning → evening order.
        groups.sort_by(|a, b| b.0.cmp(&a.0));
        let fits = |n: usize| if n == 1 { "fits" } else { "fit" };
        let items = |n: usize| if n == 1 { "item" } else { "items" };
        let mut text = String::new();
        for (i, (n, part)) in groups.iter().enumerate() {
            if i == 0 {
                text.push_str(&format!("{n} {} moved to {part}", items(*n)));
            } else {
                text.push_str(&format!(", {n} to {part}"));
            }
        }
        if dropped > 0 {
            if text.is_empty() {
                text = format!("{dropped} {} no longer {} today", items(dropped), fits(dropped));
            } else {
                text.push_str(&format!("; {dropped} no longer {} today", fits(dropped)));
            }
        }
        Some(Moved { to, dropped, text })
    }
}

/// Today's takes by task slug, with the part of the day each one's free block starts in.
fn plan_parts(
    ranked: &[crate::models::Task],
    today: Date,
    cal: &crate::weekcal::WeekCalendar,
    planning: Option<&crate::planning::PlanningConfig>,
) -> BTreeMap<String, Part> {
    let blocks = cal.free_blocks(today);
    crate::ranking::designate_today_explained(ranked, today, cal, planning)
        .into_iter()
        .filter_map(|take| {
            let block = blocks.get(take.block_index)?;
            Some((take.task.slug, part_of_day(block.start.time())))
        })
        .collect()
}

/// §6.4, purely: designates today twice from the **same** `ranked` list — under `now_cal` and
/// under `base_cal` — and diffs the plans by task. A take counts as moved to its part of the day
/// when its part changed or it is new in the current plan; one only in the baseline plan is
/// `dropped`. `None` when nothing moved. No clock, no model, no write.
pub fn moved(
    ranked: &[crate::models::Task],
    today: Date,
    now_cal: &crate::weekcal::WeekCalendar,
    base_cal: &crate::weekcal::WeekCalendar,
    planning: Option<&crate::planning::PlanningConfig>,
) -> Option<Moved> {
    let now = plan_parts(ranked, today, now_cal, planning);
    let base = plan_parts(ranked, today, base_cal, planning);
    let mut to = MovedTo::default();
    for (slug, part) in &now {
        if base.get(slug) == Some(part) {
            continue;
        }
        match part {
            Part::Morning => to.morning += 1,
            Part::Afternoon => to.afternoon += 1,
            Part::Evening => to.evening += 1,
        }
    }
    let dropped = base.keys().filter(|slug| !now.contains_key(*slug)).count();
    Moved::from_counts(to, dropped)
}

/// The `--window` preview's argument (§6.4 "Preview"), validated as §2.4 validates a planning-day
/// note's `window` — but strictly: where `load` skips a bad entry or a repeated weekday with a
/// warning, a preview refuses it, since the student is asking about exactly that value. A weekday
/// left out keeps the template (`None`).
pub fn parse_window(raw: &str) -> Result<[Option<(Time, Time)>; 7], String> {
    let value: Value = serde_yaml_ng::from_str(raw).map_err(|_| format!("{raw:?} is not a flow sequence"))?;
    let Value::Sequence(entries) = value else {
        return Err(format!("{raw:?} is not a sequence of {{days, start, end}} entries"));
    };
    let mut window: [Option<(Time, Time)>; 7] = [None; 7];
    for entry in &entries {
        let meet = parse_entry(entry)
            .map_err(|(days, start, end)| format!("planning day {days}: {start}–{end} is not a valid window"))?;
        for day in &meet.days {
            let idx = DAY_KEYS.iter().position(|k| k == day).ok_or_else(|| format!("{day}: not a day"))?;
            if window[idx].is_some() {
                return Err(format!("planning day {day}: listed twice"));
            }
            window[idx] = Some((meet.start, meet.end));
        }
    }
    Ok(window)
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A fresh scratch vault per test (invented notes only).
    fn vault(name: &str) -> PathBuf {
        let dir = std::env::temp_dir().join(format!("knowlu-cmt-{}-{name}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(dir.join(FOLDER)).unwrap();
        dir
    }

    fn note(vault: &Path, file: &str, front: &str) {
        let text = format!("---\n{front}---\n\nInvented for a test.\n");
        std::fs::write(vault.join(FOLDER).join(file), text).unwrap();
    }

    fn t(h: i8, m: i8) -> Time {
        Time::new(h, m, 0, 0).unwrap()
    }

    fn class(id: &str, title: &str, uid: &str) -> String {
        format!(
            "id: {id}\ntype: commitment\nkind: class\ntitle: \"{title}\"\n\
             meets: [{{days: [tue, thu], start: \"09:30\", end: \"10:45\"}}]\n\
             source_uid: \"{uid}\"\nstatus: confirmed\n"
        )
    }

    #[test]
    fn load_reads_a_confirmed_commitment() {
        let v = vault("confirmed");
        note(
            &v,
            "cs-100.md",
            "id: cmt_3f9a1c2b7d\ntype: commitment\nkind: class\nlevel: hard\ntitle: \"CS 100\"\n\
             course: cs-100\nmeets: [{days: [mon, wed, fri], start: \"12:00\", end: \"12:50\"}]\n\
             where: \"Room 101\"\nfrom: 2026-08-19\nuntil: 2026-12-04\n\
             source_uid: \"gcal-series:4k2q9x7m1abc\"\nstatus: confirmed\nconfirmed_at: 2026-09-24\n",
        );
        let got = load(&v);
        assert!(got.warnings.is_empty(), "{:?}", got.warnings);
        assert_eq!(
            got.confirmed,
            vec![Commitment {
                id: "cmt_3f9a1c2b7d".into(),
                path: PathBuf::from("commitments/cs-100.md"),
                kind: "class".into(),
                level: Level::Hard,
                title: "CS 100".into(),
                course: Some("cs-100".into()),
                meets: vec![Meet {
                    days: vec!["mon", "wed", "fri"],
                    start: t(12, 0),
                    end: t(12, 50)
                }],
                where_: Some("Room 101".into()),
                from: Some(jiff::civil::date(2026, 8, 19)),
                until: Some(jiff::civil::date(2026, 12, 4)),
                source_uid: Some("gcal-series:4k2q9x7m1abc".into()),
            }]
        );
        assert!(got.declined.is_empty() && got.planning_day.is_none());
        assert_eq!(got.window, [None; 7]);
    }

    #[test]
    fn a_missing_folder_loads_empty_with_no_warning() {
        let v = vault("missing");
        std::fs::remove_dir_all(v.join(FOLDER)).unwrap();
        let got = load(&v);
        assert!(got.confirmed.is_empty() && got.declined.is_empty() && got.planning_day.is_none());
        assert!(got.warnings.is_empty());
        assert_eq!(got.window, [None; 7]);
    }

    #[test]
    fn a_hand_written_proposed_note_is_ignored_with_a_warning() {
        let v = vault("proposed");
        note(
            &v,
            "club.md",
            &class("cmt_0000000001", "Chess", "k").replace("confirmed", "proposed"),
        );
        let got = load(&v);
        assert!(got.confirmed.is_empty());
        assert_eq!(
            got.warnings,
            vec!["commitments/club.md: status proposed ignored; only confirmed counts"]
        );
    }

    #[test]
    fn reserved_kinds_load_without_a_warning_at_their_default_levels() {
        let v = vault("reserved");
        for (file, kind) in [("a.md", "event"), ("b.md", "exam"), ("c.md", "task-block")] {
            note(
                &v,
                file,
                &class("cmt_000000000a", "X", file)
                    .replace("kind: class", &format!("kind: {kind}")),
            );
        }
        let got = load(&v);
        assert!(got.warnings.is_empty(), "{:?}", got.warnings);
        let levels: Vec<(&str, Level)> = got
            .confirmed
            .iter()
            .map(|c| (c.kind.as_str(), c.level))
            .collect();
        assert_eq!(
            levels,
            vec![
                ("event", Level::Soft),
                ("exam", Level::Hard),
                ("task-block", Level::Soft)
            ]
        );
    }

    #[test]
    fn an_unknown_kind_loads_soft_with_a_warning() {
        let v = vault("unknown-kind");
        note(
            &v,
            "x.md",
            &class("cmt_0000000001", "X", "k").replace("kind: class", "kind: party"),
        );
        let got = load(&v);
        assert_eq!(got.confirmed.len(), 1);
        assert_eq!(got.confirmed[0].level, Level::Soft);
        assert_eq!(
            got.warnings,
            vec!["commitments/x.md: unknown kind party; loaded as soft"]
        );
    }

    #[test]
    fn an_unknown_level_loads_soft_with_a_warning() {
        let v = vault("unknown-level");
        note(
            &v,
            "x.md",
            &format!("{}level: urgent\n", class("cmt_0000000001", "X", "k")),
        );
        let got = load(&v);
        assert_eq!(got.confirmed.len(), 1);
        assert_eq!(got.confirmed[0].level, Level::Soft);
        assert_eq!(
            got.warnings,
            vec!["commitments/x.md: unknown level urgent; loaded as soft"]
        );
    }

    #[test]
    fn an_invalid_meets_entry_is_skipped_and_a_note_with_none_is_ignored() {
        let v = vault("invalid-meets");
        let good = "{days: [mon], start: \"09:00\", end: \"10:00\"}";
        let inverted = "{days: [tue], start: \"11:00\", end: \"10:00\"}";
        let past = "{days: [wed], start: \"09:00\", end: \"25:00\"}";
        let bad_day = "{days: [funday], start: \"09:00\", end: \"10:00\"}";
        let front = |meets: &str| {
            format!("id: cmt_0000000001\nkind: work\ntitle: Shift\nmeets: [{meets}]\nstatus: confirmed\n")
        };
        note(&v, "a.md", &front(&format!("{inverted}, {good}, {past}")));
        note(
            &v,
            "b.md",
            &front(&format!("{inverted}, {past}, {bad_day}")),
        );
        let got = load(&v);
        assert_eq!(got.confirmed.len(), 1);
        assert_eq!(
            got.confirmed[0].meets,
            vec![Meet {
                days: vec!["mon"],
                start: t(9, 0),
                end: t(10, 0)
            }]
        );
        assert_eq!(
            got.warnings,
            vec![
                "commitments/a.md: meets entry tue: 11:00–10:00 invalid; skipped",
                "commitments/a.md: meets entry wed: 09:00–25:00 invalid; skipped",
                "commitments/b.md: meets entry tue: 11:00–10:00 invalid; skipped",
                "commitments/b.md: meets entry wed: 09:00–25:00 invalid; skipped",
                "commitments/b.md: meets entry funday: 09:00–10:00 invalid; skipped",
                "commitments/b.md: no valid meets entry; ignored",
            ]
        );
    }

    #[test]
    fn duplicate_source_uid_keeps_the_lowest_id_and_warns_once() {
        let v = vault("duplicates");
        note(
            &v,
            "a.md",
            &class("cmt_000000000c", "Bio 1", "gcal-series:dup"),
        );
        note(
            &v,
            "b.md",
            &class("cmt_000000000a", "Bio 1", "gcal-series:dup"),
        );
        note(
            &v,
            "c.md",
            &class("cmt_000000000b", "Bio 1", "gcal-series:dup"),
        );
        note(
            &v,
            "d.md",
            &class("cmt_0000000001", "Art 2", "gcal-series:other"),
        );
        let got = load(&v);
        let ids: Vec<&str> = got.confirmed.iter().map(|c| c.id.as_str()).collect();
        assert_eq!(ids, vec!["cmt_000000000a", "cmt_0000000001"]);
        assert_eq!(
            got.warnings,
            vec![
                "duplicate source_uid gcal-series:dup: kept commitments/b.md, \
                 ignored commitments/c.md, commitments/a.md"
            ]
        );
    }

    #[test]
    fn a_decline_marker_contributes_its_key_only() {
        let v = vault("declined");
        note(
            &v,
            "declined-0123456789.md",
            "id: cmt_0000000009\ntype: commitment\nstatus: declined\ntitle: \"Stray\"\n\
             source_uid: \"gcal-series:no\"\n",
        );
        note(
            &v,
            "window.md",
            "id: cmt_0000000008\nstatus: declined\nsource_uid: \"window\"\n",
        );
        let got = load(&v);
        assert!(got.confirmed.is_empty() && got.planning_day.is_none());
        assert!(got.warnings.is_empty(), "{:?}", got.warnings);
        assert_eq!(
            got.declined,
            BTreeSet::from(["gcal-series:no".to_string(), "window".to_string()])
        );
    }

    fn planning(id: &str, window: &str) -> String {
        format!("id: {id}\ntype: commitment\nkind: planning-day\nstatus: confirmed\nwindow: [{window}]\n")
    }

    #[test]
    fn the_planning_day_lowest_id_wins_and_a_duplicate_weekday_is_skipped() {
        let v = vault("planning");
        note(
            &v,
            "planning-day-2.md",
            &planning(
                "cmt_00000000ff",
                "{days: [sun], start: \"06:00\", end: \"07:00\"}",
            ),
        );
        note(
            &v,
            "planning-day.md",
            &planning(
                "cmt_0000000010",
                "{days: [mon, tue], start: \"08:00\", end: \"22:00\"}, \
                 {days: [tue, sat], start: \"10:00\", end: \"20:00\"}",
            ),
        );
        let got = load(&v);
        assert!(
            got.confirmed.is_empty(),
            "the planning day is not a commitment"
        );
        let day = got.planning_day.as_ref().expect("a planning day");
        assert_eq!(day.id, "cmt_0000000010");
        assert_eq!(day.kind, PLANNING_DAY);
        let mut want: [Option<(Time, Time)>; 7] = [None; 7];
        want[0] = Some((t(8, 0), t(22, 0)));
        want[1] = Some((t(8, 0), t(22, 0)));
        want[5] = Some((t(10, 0), t(20, 0)));
        assert_eq!(got.window, want);
        assert_eq!(
            got.warnings,
            vec![
                "several planning-day notes: kept commitments/planning-day.md, \
                 ignored commitments/planning-day-2.md",
                "planning day tue: listed twice; later entry skipped",
            ]
        );
        assert!(got.spans().is_empty(), "the planning day adds no busy time");
    }

    #[test]
    fn an_invalid_window_entry_warns_with_the_spec_text() {
        let v = vault("window-inverted");
        note(
            &v,
            "planning-day.md",
            &planning(
                "cmt_0000000010",
                "{days: [mon, tue], start: \"22:00\", end: \"08:00\"}, \
                 {days: [wed], start: \"07:30\", end: \"21:00\"}",
            ),
        );
        let got = load(&v);
        assert_eq!(
            got.warnings,
            vec!["planning day mon, tue: 22:00–08:00 ignored; using week_template"]
        );
        let mut want: [Option<(Time, Time)>; 7] = [None; 7];
        want[2] = Some((t(7, 30), t(21, 0)));
        assert_eq!(got.window, want, "mon and tue keep the template");
    }

    #[test]
    fn a_window_end_past_23_59_is_invalid() {
        let v = vault("window-24");
        note(
            &v,
            "planning-day.md",
            &planning(
                "cmt_0000000010",
                "{days: [fri], start: \"08:00\", end: \"24:00\"}, \
                 {days: [sat], start: \"09:00\", end: \"23:59\"}",
            ),
        );
        let got = load(&v);
        assert_eq!(
            got.warnings,
            vec!["planning day fri: 08:00–24:00 ignored; using week_template"]
        );
        let mut want: [Option<(Time, Time)>; 7] = [None; 7];
        want[5] = Some((t(9, 0), t(23, 59)));
        assert_eq!(got.window, want, "fri keeps the template");
    }

    #[test]
    fn spans_come_from_hard_and_soft_notes_only() {
        let v = vault("spans");
        note(
            &v,
            "a-class.md",
            &class("cmt_0000000001", "Chem 1", "gcal-series:chem"),
        );
        note(
            &v,
            "b-club.md",
            "id: cmt_0000000002\nkind: club\ntitle: Chess Club\nfrom: 2026-09-01\n\
             meets: [{days: [wed], start: \"18:00\", end: \"19:00\"}]\nstatus: confirmed\n",
        );
        note(
            &v,
            "c-office.md",
            "id: cmt_0000000003\nkind: office-hours\ntitle: Office hours\n\
             meets: [{days: [mon], start: \"14:00\", end: \"15:00\"}]\nstatus: confirmed\n",
        );
        note(
            &v,
            "d-marker.md",
            "id: cmt_0000000004\nstatus: declined\nsource_uid: \"gcal-series:x\"\n",
        );
        let got = load(&v);
        assert!(got.warnings.is_empty(), "{:?}", got.warnings);
        let span = |day, s, e, from, title: &str, kind: &str, uid: &str| CommitmentSpan {
            day,
            start: s,
            end: e,
            from,
            until: None,
            title: title.into(),
            kind: kind.into(),
            source_uid: uid.into(),
        };
        assert_eq!(
            got.spans(),
            vec![
                span(
                    "tue",
                    t(9, 30),
                    t(10, 45),
                    None,
                    "Chem 1",
                    "class",
                    "gcal-series:chem"
                ),
                span(
                    "thu",
                    t(9, 30),
                    t(10, 45),
                    None,
                    "Chem 1",
                    "class",
                    "gcal-series:chem"
                ),
                span(
                    "wed",
                    t(18, 0),
                    t(19, 0),
                    Some(jiff::civil::date(2026, 9, 1)),
                    "Chess Club",
                    "club",
                    ""
                ),
            ]
        );
    }

    #[test]
    fn front_matter_puts_every_collection_field_on_one_line() {
        let scalars = vec![
            ("id", Node::text("cmt_3f9a1c2b7d")),
            ("type", Node::text("commitment")),
            ("title", Node::text("CS 100: Lab")),
            ("where", Node::Null),
            ("confirmed_at", Node::Date(jiff::civil::date(2026, 9, 24))),
        ];
        let fields: Vec<(&str, Field)> = scalars
            .iter()
            .map(|(k, n)| (*k, Field::Scalar(n.clone())))
            .collect();
        assert_eq!(
            front_matter(&fields).unwrap(),
            yamlemit::safe_dump_block(&Node::map(scalars.clone()))
        );

        let meets =
            serde_json::json!([{"days": ["mon", "wed", "fri"], "start": "12:00", "end": "12:50"}]);
        let commitment = serde_json::json!({
            "kind": "class", "level": "hard", "title": "CS 100: Lab", "course": "cs-100",
            "meets": [{"days": ["tue"], "start": "09:30", "end": "10:45"}],
            "where": null, "from": "2026-08-19", "until": null,
        });
        let fields = vec![
            ("type", Field::Scalar(Node::text("approval"))),
            ("meets", Field::Flow(meets.clone())),
            ("commitment", Field::Flow(commitment.clone())),
        ];
        let front = front_matter(&fields).unwrap();
        let lines: Vec<&str> = front.lines().collect();
        assert_eq!(lines.len(), 3, "{front:?}");
        assert!(
            lines[1].starts_with("meets: [") && lines[2].starts_with("commitment: {"),
            "{front:?}"
        );

        let (mapping, _) = split_frontmatter(&format!("---\n{front}---\n")).expect("parses");
        assert_eq!(
            get(&mapping, "meets"),
            Some(&crate::yaml::from_json(&meets))
        );
        assert_eq!(
            get(&mapping, "commitment"),
            Some(&crate::yaml::from_json(&commitment))
        );
    }

    // ---------------------------------------------------------------------------------------
    // The sync tripwires (spec §10 phase 1s; plan review C2, M8). Before C3′'s sync is on this
    // branch the files they read are absent and they pass; once it is, they stay red until P21
    // of the commitment-model phase-1 plan lands. `the_tripwires_refuse_a_comment` proves the
    // checks themselves can fail.
    // ---------------------------------------------------------------------------------------

    const P21: &str = "P21 of docs/plans/2026-09-23-commitment-model-phase1-plan.md (phase 1s)";

    /// The part of a line before any `//` comment, trimmed.
    fn code_of(line: &str) -> &str {
        line.split("//").next().unwrap_or("").trim()
    }

    /// Decision 9: `commitments::LOCAL_CARD_KINDS` on a line of code, and
    /// `fn build_push_sends_no_local_card_nor_any_record_about_one` whose nearest preceding
    /// non-blank, non-comment line is `#[test]`.
    fn sync_rs_keeps_cards_local(text: &str) -> Result<(), String> {
        let lines: Vec<&str> = text.lines().collect();
        if !lines
            .iter()
            .any(|l| code_of(l).contains("commitments::LOCAL_CARD_KINDS"))
        {
            return Err(format!(
                "sync.rs never reads commitments::LOCAL_CARD_KINDS in code; see {P21}"
            ));
        }
        let name = "fn build_push_sends_no_local_card_nor_any_record_about_one";
        let tested = lines.iter().enumerate().any(|(i, l)| {
            let code = code_of(l);
            (code.starts_with(name) || code.starts_with(&format!("pub {name}")))
                && lines[..i]
                    .iter()
                    .rev()
                    .map(|l| code_of(l))
                    .find(|c| !c.is_empty())
                    == Some("#[test]")
        });
        if !tested {
            return Err(format!("sync.rs has no #[test] {name}; see {P21}"));
        }
        Ok(())
    }

    /// The migration the server's note-path check comes from: the latest by file name.
    fn latest_migration(migrations: &[(String, String)]) -> Option<&(String, String)> {
        migrations.iter().max_by(|a, b| a.0.cmp(&b.0))
    }

    /// Decision 8: the server's rule and its latest migration name every `ids::NOTE_FOLDERS`.
    fn server_rules_name(
        folders: &[&str],
        sync_rows: Option<&str>,
        migrations: &[(String, String)],
    ) -> Result<(), String> {
        let group = format!("({})/", folders.join("|"));
        if let Some(text) = sync_rows {
            // A JS regex literal escapes its slash: `NOTE_PATH_RE` spells the group `(…)\/`.
            let escaped = format!("({})\\/", folders.join("|"));
            if !text.contains(&group) && !text.contains(&escaped) {
                return Err(format!(
                    "sync_rows.ts's NOTE_PATH_RE lacks {group}; see {P21}"
                ));
            }
        }
        if let Some((name, text)) = latest_migration(migrations) {
            if !text.contains(&group) {
                return Err(format!("migration {name} lacks {group}; see {P21}"));
            }
        }
        Ok(())
    }

    #[test]
    fn sync_keeps_every_local_card_kind_local() {
        let path = concat!(env!("CARGO_MANIFEST_DIR"), "/src/sync.rs");
        let Ok(text) = std::fs::read_to_string(path) else {
            return;
        };
        if let Err(why) = sync_rs_keeps_cards_local(&text) {
            panic!("{why}");
        }
    }

    #[test]
    fn the_servers_note_path_rules_name_every_note_folder() {
        let root = Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("..")
            .join("cloud")
            .join("supabase");
        let sync_rows = std::fs::read_to_string(root.join("functions/_shared/sync_rows.ts")).ok();
        let mut migrations = Vec::new();
        if let Ok(entries) = std::fs::read_dir(root.join("migrations")) {
            for entry in entries.filter_map(|e| e.ok()) {
                let name = entry.file_name().to_string_lossy().into_owned();
                if name.contains("sync_note_path_check") && name.ends_with(".sql") {
                    let text = std::fs::read_to_string(entry.path()).unwrap_or_default();
                    migrations.push((name, text));
                }
            }
        }
        if let Err(why) =
            server_rules_name(&crate::ids::NOTE_FOLDERS, sync_rows.as_deref(), &migrations)
        {
            panic!("{why}");
        }
    }

    /// Non-vacuity: every check above fails on the shape it exists to catch.
    #[test]
    fn the_tripwires_refuse_a_comment() {
        let body = "    fn build_push_sends_no_local_card_nor_any_record_about_one() {}\n";
        let good = format!("use crate::commitments::LOCAL_CARD_KINDS;\nlet k = commitments::LOCAL_CARD_KINDS;\n#[test]\n// why\n\n{body}");
        assert_eq!(sync_rs_keeps_cards_local(&good), Ok(()));
        let commented = format!(
            "// commitments::LOCAL_CARD_KINDS\n/// #[test]\n// {}\n",
            body.trim()
        );
        assert!(
            sync_rs_keeps_cards_local(&commented).is_err(),
            "both only in comments"
        );
        let trailing = format!("let k = 1; // commitments::LOCAL_CARD_KINDS\n#[test]\n{body}");
        assert!(
            sync_rs_keeps_cards_local(&trailing).is_err(),
            "a trailing comment is a comment"
        );
        let untested = "let k = commitments::LOCAL_CARD_KINDS;\n// #[test]\n".to_string() + body;
        assert!(
            sync_rs_keeps_cards_local(&untested).is_err(),
            "#[test] only in a comment"
        );
        let not_a_test =
            format!("let k = commitments::LOCAL_CARD_KINDS;\n#[test]\n#[ignore]\n{body}");
        assert!(
            sync_rs_keeps_cards_local(&not_a_test).is_err(),
            "an attribute between"
        );

        let seven = crate::ids::NOTE_FOLDERS;
        let six = &seven[..6];
        assert!(!six.contains(&"commitments"));
        let rule = |folders: &[&str]| {
            format!(
                "/^({})\\/[A-Za-z0-9._ /-]{{1,300}}\\.md$/",
                folders.join("|")
            )
        };
        let sql =
            |folders: &[&str]| format!("path ~ '^({})/[A-Za-z0-9._ /-]+\\.md$'", folders.join("|"));
        assert!(
            server_rules_name(&seven, Some(&rule(six)), &[]).is_err(),
            "six folders in sync_rows.ts"
        );
        assert_eq!(server_rules_name(&seven, Some(&rule(&seven)), &[]), Ok(()));
        assert!(server_rules_name(&seven, Some("// (tasks|approvals)"), &[]).is_err());
        assert_eq!(
            server_rules_name(&seven, None, &[]),
            Ok(()),
            "phase 1 before C3′"
        );

        let migrations = vec![
            (
                "20260912000400_sync_note_path_check.sql".to_string(),
                sql(six),
            ),
            (
                "20260930000100_sync_note_path_check_commitments.sql".to_string(),
                sql(&seven),
            ),
        ];
        assert_eq!(
            latest_migration(&migrations).map(|m| m.0.as_str()),
            Some(migrations[1].0.as_str())
        );
        assert_eq!(server_rules_name(&seven, None, &migrations), Ok(()));
        let reversed: Vec<_> = migrations.iter().rev().cloned().collect();
        assert_eq!(
            server_rules_name(&seven, None, &reversed),
            Ok(()),
            "order on disk is irrelevant"
        );
        assert!(
            server_rules_name(&seven, None, &migrations[..1]).is_err(),
            "only the six-folder one"
        );
    }

    // ---------------------------------------------------------------------------------------
    // The vault's code table and the classifier (§3.4, C3, R7; C5), P7.
    // ---------------------------------------------------------------------------------------

    fn course_dir(v: &Path) -> PathBuf {
        let dir = v.join("courses");
        std::fs::create_dir_all(&dir).unwrap();
        dir
    }

    fn course_note(v: &Path, file: &str, front: &str) {
        let text = format!("---\n{front}---\n\nInvented for a test.\n");
        std::fs::write(course_dir(v).join(file), text).unwrap();
    }

    fn codes_with(pairs: &[(&str, &str)]) -> Codes {
        let table = pairs
            .iter()
            .map(|(k, v)| (k.to_string(), v.to_string()))
            .collect();
        Codes {
            table,
            names: BTreeMap::new(),
        }
    }

    fn instances_weekly(start: Date, weeks: i64, s: Time, e: Time) -> Vec<Instance> {
        let mut out = Vec::new();
        let mut d = start;
        for _ in 0..weeks {
            out.push(Instance {
                date: d,
                start: Some(s),
                end: Some(e),
            });
            d = d.checked_add(jiff::Span::new().days(7)).unwrap();
        }
        out
    }

    /// A weekly series over invented instances (default two weeks). `until`/`last_seen` are left
    /// so eligibility's "not before today" check is a no-op unless a test sets both.
    fn base_series(title: &str, instances: Vec<Instance>) -> Series {
        let (day, s, e) = instances
            .first()
            .map(|i| (crate::planning::day_key(i.date), i.start, i.end))
            .unwrap_or(("mon", None, None));
        let meets = match (s, e) {
            (Some(s), Some(e)) => vec![Meet {
                days: vec![day],
                start: s,
                end: e,
            }],
            _ => Vec::new(),
        };
        Series {
            source_uid: format!("gcal-series:{}", title.to_ascii_lowercase().replace(' ', "-")),
            calendar: "google:test".to_string(),
            title: title.to_string(),
            where_: None,
            event_type: None,
            rule: Rule {
                freq: "WEEKLY".to_string(),
                interval: 1,
                until: None,
                count: None,
            },
            has_master: true,
            rdate: false,
            unsupported: false,
            instances,
            meets,
            first: None,
            until: None,
            last_seen: Some(jiff::civil::date(2026, 9, 1)),
        }
    }

    fn weekly(title: &str, weeks: i64, s: Time, e: Time) -> Series {
        base_series(
            title,
            instances_weekly(jiff::civil::date(2026, 9, 1), weeks, s, e),
        )
    }

    #[test]
    fn compact_codes_match_across_spellings() {
        assert_eq!(to_code("CS 100"), Some("CS100".to_string()));
        assert_eq!(to_code("cs-100"), Some("CS100".to_string()));
        assert_eq!(to_code("CS100"), Some("CS100".to_string()));
    }

    #[test]
    fn cs_1110_compsci_61a_and_math_20a_match_their_courses() {
        let v = vault("codes-schools");
        course_note(&v, "cs-1110.md", "title: \"CS 1110\"\nslug: cs-1110\n");
        course_note(
            &v,
            "compsci-61a.md",
            "title: \"COMPSCI 61A\"\nslug: compsci-61a\n",
        );
        course_note(&v, "math-20a.md", "title: \"MATH 20A\"\nslug: math-20a\n");
        let (codes, warnings) = code_table(&v);
        assert!(warnings.is_empty(), "{warnings:?}");
        assert_eq!(codes.get("CS1110"), Some(&"cs-1110".to_string()));
        assert_eq!(codes.get("COMPSCI61A"), Some(&"compsci-61a".to_string()));
        assert_eq!(codes.get("MATH20A"), Some(&"math-20a".to_string()));
    }

    #[test]
    fn lms_names_reduce_by_the_d4_fallback() {
        let v = vault("codes-lms");
        course_note(
            &v,
            "bui-100.md",
            "name: \"202640-BUI-100-101\"\nslug: bui-100\n",
        );
        let (codes, warnings) = code_table(&v);
        assert!(warnings.is_empty(), "{warnings:?}");
        assert_eq!(codes.get("BUI100"), Some(&"bui-100".to_string()));
    }

    #[test]
    fn a_code_claimed_by_two_courses_is_dropped_with_a_warning() {
        let v = vault("codes-conflict");
        course_note(&v, "a.md", "title: \"CS 100\"\nslug: cs-100-a\n");
        course_note(&v, "b.md", "title: \"CS 100\"\nslug: cs-100-b\n");
        let (codes, warnings) = code_table(&v);
        assert!(!codes.contains_key("CS100"), "{codes:?}");
        assert_eq!(
            warnings,
            vec!["code CS100: claimed by both cs-100-a and cs-100-b; dropped"]
        );
    }

    #[test]
    fn work_on_cs_100_cs_100_study_group_and_study_for_ph_106_are_never_proposed() {
        let codes = codes_with(&[("CS100", "cs-100"), ("PH106", "ph-106")]);
        for title in ["Work on CS 100", "CS 100 study group", "Study for PH 106"] {
            let series = weekly(title, 2, t(19, 0), t(20, 0));
            assert_eq!(classify(&series, &codes, &[]), None, "{title}");
        }
    }

    #[test]
    fn a_planning_yaml_recurring_name_is_never_proposed() {
        let codes = codes_with(&[]);
        let series = weekly("Piano lesson", 2, t(19, 0), t(20, 0));
        let planning = vec!["Piano lesson".to_string()];
        assert_eq!(classify(&series, &codes, &planning), None);
        assert_eq!(
            classify(&series, &codes, &[]),
            Some(Class::Kind {
                kind: "meeting".to_string(),
                course: None
            }),
            "without the planning entry the same series is an ordinary meeting"
        );
    }

    #[test]
    fn cs_100_lab_is_a_lab_and_cs_100_lecture_is_a_class() {
        let codes = codes_with(&[("CS100", "cs-100")]);
        let lab = weekly("CS 100 Lab", 2, t(9, 0), t(9, 50));
        assert_eq!(
            classify(&lab, &codes, &[]),
            Some(Class::Kind {
                kind: "lab".to_string(),
                course: Some("cs-100".to_string())
            })
        );
        let lecture = weekly("CS 100 Lecture", 2, t(9, 0), t(9, 50));
        assert_eq!(
            classify(&lecture, &codes, &[]),
            Some(Class::Kind {
                kind: "class".to_string(),
                course: Some("cs-100".to_string())
            })
        );
    }

    #[test]
    fn words_from_the_course_name_after_the_code_are_a_class() {
        let codes = Codes {
            table: BTreeMap::from([("CS100".to_string(), "cs-100".to_string())]),
            names: BTreeMap::from([(
                "cs-100".to_string(),
                BTreeSet::from([
                    "cs".to_string(),
                    "100".to_string(),
                    "intro".to_string(),
                    "to".to_string(),
                    "computer".to_string(),
                    "science".to_string(),
                ]),
            )]),
        };
        let series = weekly("CS 100 – Intro to Computer Science", 2, t(12, 0), t(12, 50));
        assert_eq!(
            classify(&series, &codes, &[]),
            Some(Class::Kind {
                kind: "class".to_string(),
                course: Some("cs-100".to_string())
            })
        );
    }

    #[test]
    fn cs_100_ta_hours_is_not_a_class_and_falls_through_to_meeting() {
        let codes = codes_with(&[("CS100", "cs-100")]);
        let series = weekly("CS 100 TA hours", 2, t(9, 0), t(9, 50));
        assert_eq!(
            classify(&series, &codes, &[]),
            Some(Class::Kind {
                kind: "meeting".to_string(),
                course: None
            })
        );
    }

    #[test]
    fn oh_matches_only_in_capitals() {
        let codes = codes_with(&[("CS100", "cs-100")]);
        let oh = weekly("CS 100 OH", 2, t(15, 0), t(16, 0));
        assert_eq!(
            classify(&oh, &codes, &[]),
            Some(Class::Kind {
                kind: "office-hours".to_string(),
                course: Some("cs-100".to_string())
            })
        );
        let no = weekly("oh no", 2, t(15, 0), t(16, 0));
        assert_eq!(
            classify(&no, &codes, &[]),
            Some(Class::Kind {
                kind: "meeting".to_string(),
                course: None
            }),
            "lowercase oh never matches"
        );
    }

    #[test]
    fn routines_match_the_whole_title_only() {
        let codes = codes_with(&[]);
        let wake = weekly("Wake Up", 2, t(7, 0), t(7, 5));
        assert_eq!(
            classify(&wake, &codes, &[]),
            Some(Class::Routine {
                wake: true,
                bed: false
            })
        );
        let sleep_study = weekly("Sleep study", 2, t(22, 0), t(23, 0));
        assert_eq!(
            classify(&sleep_study, &codes, &[]),
            None,
            "study makes it rule 0, not a routine"
        );
    }

    #[test]
    fn a_midnight_sleep_series_is_both_sides() {
        let codes = codes_with(&[]);
        let series = weekly("Sleep", 2, t(23, 0), t(7, 0));
        assert_eq!(
            classify(&series, &codes, &[]),
            Some(Class::Routine {
                wake: true,
                bed: true
            })
        );
    }

    #[test]
    fn a_midnight_crossing_series_other_than_sleep_is_ineligible() {
        let codes = codes_with(&[]);
        let series = weekly("Night shift", 2, t(23, 0), t(1, 0));
        assert_eq!(classify(&series, &codes, &[]), None);
    }

    #[test]
    fn work_shift_club_team_practice() {
        let codes = codes_with(&[]);
        let want_work = Some(Class::Kind {
            kind: "work".to_string(),
            course: None,
        });
        let want_club = Some(Class::Kind {
            kind: "club".to_string(),
            course: None,
        });
        let work = weekly("Work Shift", 2, t(17, 0), t(21, 0));
        assert_eq!(classify(&work, &codes, &[]), want_work);
        let shift = weekly("Evening Shift", 2, t(17, 0), t(21, 0));
        assert_eq!(classify(&shift, &codes, &[]), want_work);
        let club = weekly("Chess Club", 2, t(18, 0), t(19, 0));
        assert_eq!(classify(&club, &codes, &[]), want_club);
        let team = weekly("Soccer Practice", 2, t(16, 0), t(17, 30));
        assert_eq!(classify(&team, &codes, &[]), want_club);
    }

    #[test]
    fn ineligible_series_are_never_classified() {
        let codes = codes_with(&[("CS100", "cs-100")]);
        let base = || weekly("CS 100", 2, t(9, 0), t(9, 50));

        // M6: a positive control. Without this, a regression in `base()` itself would make every
        // sub-case below pass vacuously (`None` for the wrong reason).
        assert!(
            classify(&base(), &codes, &[]).is_some(),
            "the base series is eligible and classifies on its own"
        );

        let mut s = base();
        s.event_type = Some("fromGmail".to_string());
        assert_eq!(classify(&s, &codes, &[]), None, "fromGmail");

        let mut s = base();
        s.event_type = Some("focusTime".to_string());
        assert_eq!(classify(&s, &codes, &[]), None, "focusTime");

        let mut s = base();
        s.has_master = false;
        assert_eq!(classify(&s, &codes, &[]), None, "no master");

        let mut s = base();
        s.rule.interval = 2;
        assert_eq!(classify(&s, &codes, &[]), None, "INTERVAL=2");

        // Every-other-week: two instances 14 days apart, so no gap is exactly 7.
        let mut s = base();
        let day0 = jiff::civil::date(2026, 9, 1);
        let day14 = day0.checked_add(jiff::Span::new().days(14)).unwrap();
        s.instances = vec![
            Instance {
                date: day0,
                start: Some(t(9, 0)),
                end: Some(t(9, 50)),
            },
            Instance {
                date: day14,
                start: Some(t(9, 0)),
                end: Some(t(9, 50)),
            },
        ];
        s.meets = vec![Meet {
            days: vec![crate::planning::day_key(day0)],
            start: t(9, 0),
            end: t(9, 50),
        }];
        assert_eq!(classify(&s, &codes, &[]), None, "every-other-week");

        let mut s = base();
        s.instances.truncate(1);
        assert_eq!(classify(&s, &codes, &[]), None, "one instance");

        let mut s = base();
        for i in s.instances.iter_mut() {
            i.start = None;
            i.end = None;
        }
        assert_eq!(classify(&s, &codes, &[]), None, "all-day");

        let mut s = base();
        s.until = Some(jiff::civil::date(2026, 8, 1));
        s.last_seen = Some(jiff::civil::date(2026, 9, 1));
        assert_eq!(classify(&s, &codes, &[]), None, "until before today");

        let mut s = base();
        s.rdate = true;
        assert_eq!(classify(&s, &codes, &[]), None, "RDATE");

        let mut s = base();
        s.unsupported = true;
        assert_eq!(classify(&s, &codes, &[]), None, "BYMONTHDAY");
    }

    #[test]
    fn a_holiday_gap_is_still_weekly() {
        let codes = codes_with(&[]);
        let mut s = weekly("Chess Club", 2, t(18, 0), t(19, 0));
        let day0 = jiff::civil::date(2026, 9, 1);
        s.instances = [0, 7, 21, 28]
            .iter()
            .map(|&d| Instance {
                date: day0.checked_add(jiff::Span::new().days(d)).unwrap(),
                start: Some(t(18, 0)),
                end: Some(t(19, 0)),
            })
            .collect();
        s.meets = vec![Meet {
            days: vec![crate::planning::day_key(day0)],
            start: t(18, 0),
            end: t(19, 0),
        }];
        assert!(
            classify(&s, &codes, &[]).is_some(),
            "a holiday gap does not break the weekly pattern"
        );
    }

    // ---------------------------------------------------------------------------------------
    // Fix round 1 (review of commit 8c9dd9f): the spec-gap fix, I1-I4, and the minors.
    // ---------------------------------------------------------------------------------------

    #[test]
    fn a_section_word_followed_by_an_ordinary_word_is_not_a_class_or_lab() {
        let codes = codes_with(&[("CS100", "cs-100")]);
        let want_meeting = Some(Class::Kind {
            kind: "meeting".to_string(),
            course: None,
        });
        for title in ["CS 100 Lab Hours", "CS 100 Class Party", "CS 100 Section Leaders"] {
            let series = weekly(title, 2, t(9, 0), t(9, 50));
            assert_eq!(classify(&series, &codes, &[]), want_meeting, "{title}");
        }
        let positive = weekly("CS 100 Lab 01", 2, t(9, 0), t(9, 50));
        assert_eq!(
            classify(&positive, &codes, &[]),
            Some(Class::Kind {
                kind: "lab".to_string(),
                course: Some("cs-100".to_string())
            }),
            "a real section number after the word is still a lab"
        );
    }

    #[test]
    fn a_zero_length_event_never_crosses_midnight() {
        let codes = codes_with(&[]);
        let wake = weekly("Wake up", 2, t(7, 0), t(7, 0));
        assert_eq!(
            classify(&wake, &codes, &[]),
            Some(Class::Routine {
                wake: true,
                bed: false
            }),
            "a 07:00-07:00 alarm is a point marker at its start, not a wraparound"
        );
        let sleep = weekly("Sleep", 2, t(23, 0), t(23, 0));
        assert_eq!(
            classify(&sleep, &codes, &[]),
            Some(Class::Routine {
                wake: false,
                bed: true
            }),
            "a zero-length sleep marker is one side only, never both"
        );
    }

    #[test]
    fn a_class_ending_exactly_at_midnight_still_reads_as_crossing() {
        let codes = codes_with(&[]);
        let series = weekly("Night class", 2, t(23, 0), t(0, 0));
        assert_eq!(
            classify(&series, &codes, &[]),
            None,
            "pinned: an end of exactly 00:00 still reads as crossing midnight"
        );
    }

    #[test]
    fn term_prefixed_lms_names_read_the_course_code_not_the_term() {
        assert_eq!(to_code("FA26-CS-100-001"), Some("CS100".to_string()));
        assert_eq!(to_code("Fall 2026 - CS 100"), Some("CS100".to_string()));
        assert_eq!(to_code("202640-BUI-100-101"), Some("BUI100".to_string()));
    }

    #[test]
    fn several_courses_from_one_term_share_no_claimed_by_both_warning() {
        let v = vault("codes-term");
        course_note(&v, "cs-100.md", "name: \"FA26-CS-100-001\"\nslug: cs-100\n");
        course_note(
            &v,
            "math-200.md",
            "name: \"FA26-MATH-200-002\"\nslug: math-200\n",
        );
        let (codes, warnings) = code_table(&v);
        assert!(warnings.is_empty(), "{warnings:?}");
        assert_eq!(codes.get("CS100"), Some(&"cs-100".to_string()));
        assert_eq!(codes.get("MATH200"), Some(&"math-200".to_string()));
        assert!(!codes.contains_key("FA26"), "{codes:?}");
    }

    #[test]
    fn work_session_block_and_time_are_never_proposed_and_working_on_matches_too() {
        let codes = codes_with(&[("CS100", "cs-100")]);
        for title in [
            "Work session",
            "Work block",
            "Work time",
            "Work on CS 100",
            "Working on CS 100",
            "Work   on CS 100",
        ] {
            let series = weekly(title, 2, t(19, 0), t(20, 0));
            assert_eq!(classify(&series, &codes, &[]), None, "{title}");
        }
    }

    #[test]
    fn a_job_shift_is_kind_work() {
        let codes = codes_with(&[]);
        let want = Some(Class::Kind {
            kind: "work".to_string(),
            course: None,
        });
        for title in ["Work", "Shift", "Work @ Cafe", "Trader Joe's Shift"] {
            let series = weekly(title, 2, t(17, 0), t(21, 0));
            assert_eq!(classify(&series, &codes, &[]), want, "{title}");
        }
    }

    #[test]
    fn bare_section_designators_are_a_class_and_lab_designators_are_a_lab() {
        let codes = codes_with(&[("CS100", "cs-100"), ("CS1110", "cs-1110")]);
        let want_class = Some(Class::Kind {
            kind: "class".to_string(),
            course: Some("cs-100".to_string()),
        });
        for title in [
            "CS 100-001",
            "CS 100 001",
            "CS 100.001",
            "CS 100 LEC",
            "CS 100 SEC 1",
        ] {
            let series = weekly(title, 2, t(9, 0), t(9, 50));
            assert_eq!(classify(&series, &codes, &[]), want_class, "{title}");
        }
        let lec_series = weekly("CS 1110 001 LEC", 2, t(9, 0), t(9, 50));
        assert_eq!(
            classify(&lec_series, &codes, &[]),
            Some(Class::Kind {
                kind: "class".to_string(),
                course: Some("cs-1110".to_string())
            })
        );
        let want_lab = Some(Class::Kind {
            kind: "lab".to_string(),
            course: Some("cs-100".to_string()),
        });
        for title in ["CS 100-L01", "CS 100 L01"] {
            let series = weekly(title, 2, t(9, 0), t(9, 50));
            assert_eq!(classify(&series, &codes, &[]), want_lab, "{title}");
        }
    }

    #[test]
    fn routine_words_ignore_hyphens_double_spaces_and_glued_spelling() {
        let codes = codes_with(&[]);
        let want_wake = Some(Class::Routine {
            wake: true,
            bed: false,
        });
        for title in ["Wake-up", "Wake  Up", "Wakeup"] {
            let series = weekly(title, 2, t(7, 0), t(7, 5));
            assert_eq!(classify(&series, &codes, &[]), want_wake, "{title}");
        }
        let bed = weekly("Bed time", 2, t(22, 0), t(22, 5));
        assert_eq!(
            classify(&bed, &codes, &[]),
            Some(Class::Routine {
                wake: false,
                bed: true
            })
        );
    }

    #[test]
    fn a_planning_name_with_trailing_punctuation_matches_the_same_title() {
        let codes = codes_with(&[]);
        let series = weekly("Gym.", 2, t(6, 0), t(7, 0));
        let planning = vec!["Gym.".to_string()];
        assert_eq!(classify(&series, &codes, &planning), None);
    }

    #[test]
    fn cs_100l_is_a_lab_of_cs_100() {
        let codes = codes_with(&[("CS100", "cs-100")]);
        let series = weekly("CS 100L", 2, t(9, 0), t(9, 50));
        assert_eq!(
            classify(&series, &codes, &[]),
            Some(Class::Kind {
                kind: "lab".to_string(),
                course: Some("cs-100".to_string())
            })
        );
    }

    #[test]
    fn rule_0_catches_studying_and_hw_with_digits() {
        let codes = codes_with(&[]);
        for title in ["Studying for CS 100", "HW1", "HW 2"] {
            let series = weekly(title, 2, t(19, 0), t(20, 0));
            assert_eq!(classify(&series, &codes, &[]), None, "{title}");
        }
    }

    #[test]
    fn course_map_keys_are_a_source_and_a_third_claimant_never_re_warns() {
        let v = vault("codes-course-map");
        std::fs::create_dir_all(v.join("config")).unwrap();
        std::fs::write(
            v.join("config").join("ingest.yaml"),
            "course_map:\n  CS-100: cs-100\n  PH-106: ph-106\n",
        )
        .unwrap();
        let (codes, warnings) = code_table(&v);
        assert!(warnings.is_empty(), "{warnings:?}");
        assert_eq!(codes.get("CS100"), Some(&"cs-100".to_string()));
        assert_eq!(codes.get("PH106"), Some(&"ph-106".to_string()));
    }

    #[test]
    fn a_third_claimant_of_a_dropped_code_never_re_warns() {
        let v = vault("codes-third-claimant");
        course_note(&v, "a.md", "title: \"CS 100\"\nslug: cs-100-a\n");
        course_note(&v, "b.md", "title: \"CS 100\"\nslug: cs-100-b\n");
        std::fs::create_dir_all(v.join("config")).unwrap();
        std::fs::write(
            v.join("config").join("ingest.yaml"),
            "course_map:\n  CS-100: cs-100-c\n",
        )
        .unwrap();
        let (codes, warnings) = code_table(&v);
        assert!(!codes.contains_key("CS100"), "{codes:?}");
        assert_eq!(
            warnings,
            vec!["code CS100: claimed by both cs-100-a and cs-100-b; dropped"],
            "the third claimant (course_map) must not add a second warning"
        );
    }

    #[test]
    fn codes_load_builds_the_table_and_the_name_words_together() {
        let v = vault("codes-load");
        course_note(
            &v,
            "cs-100.md",
            "title: \"CS 100 – Intro to Computer Science\"\nslug: cs-100\n",
        );
        let (codes, warnings) = Codes::load(&v);
        assert!(warnings.is_empty(), "{warnings:?}");
        assert_eq!(codes.table.get("CS100"), Some(&"cs-100".to_string()));
        let words = codes.names.get("cs-100").expect("cs-100 words");
        assert!(
            words.contains("intro") && words.contains("computer") && words.contains("science"),
            "{words:?}"
        );
        let series = weekly("CS 100 – Intro to Computer Science", 2, t(12, 0), t(12, 50));
        assert_eq!(
            classify(&series, &codes, &[]),
            Some(Class::Kind {
                kind: "class".to_string(),
                course: Some("cs-100".to_string())
            })
        );
    }

    // ---------------------------------------------------------------------------------------
    // Fix round 2 (re-review of eeed0fe/756692f).
    // ---------------------------------------------------------------------------------------

    #[test]
    fn work_out_workout_and_working_out_are_never_kind_work() {
        let codes = codes_with(&[]);
        for title in ["Work out", "Workout", "Working out"] {
            let series = weekly(title, 2, t(17, 0), t(18, 0));
            assert_eq!(
                classify(&series, &codes, &[]),
                None,
                "{title}: I3's \"when unsure, not work\", the conservative way"
            );
        }
        // A real job shift is unaffected.
        let shift = weekly("Work", 2, t(17, 0), t(21, 0));
        assert_eq!(
            classify(&shift, &codes, &[]),
            Some(Class::Kind {
                kind: "work".to_string(),
                course: None
            })
        );
    }

    #[test]
    fn alphanumeric_tokens_like_ps1_and_1on1_are_not_section_designators() {
        let codes = codes_with(&[("CS100", "cs-100")]);
        for title in ["CS 100 PS1", "CS 100 1on1"] {
            let series = weekly(title, 2, t(9, 0), t(9, 50));
            let got = classify(&series, &codes, &[]);
            assert_ne!(
                got,
                Some(Class::Kind {
                    kind: "class".to_string(),
                    course: Some("cs-100".to_string())
                }),
                "{title}: {got:?}"
            );
            assert_ne!(
                got,
                Some(Class::Kind {
                    kind: "lab".to_string(),
                    course: Some("cs-100".to_string())
                }),
                "{title}: {got:?}"
            );
        }
    }

    #[test]
    fn bedtime_with_a_trailing_emoji_is_still_a_routine() {
        let codes = codes_with(&[]);
        let series = weekly("Bedtime \u{1F6CF}", 2, t(22, 0), t(22, 5));
        assert_eq!(
            classify(&series, &codes, &[]),
            Some(Class::Routine {
                wake: false,
                bed: true
            })
        );
    }

    #[test]
    fn reviewing_is_rule_0_the_same_way_studying_is() {
        let codes = codes_with(&[]);
        let series = weekly("Reviewing for CS 100", 2, t(19, 0), t(20, 0));
        assert_eq!(classify(&series, &codes, &[]), None);
    }

    #[test]
    fn extra_separators_reach_rule_3_and_a_leading_bracket_is_trimmed() {
        let codes = codes_with(&[("CS100", "cs-100")]);
        let want_lab = Some(Class::Kind {
            kind: "lab".to_string(),
            course: Some("cs-100".to_string()),
        });
        let em_dash = weekly("CS 100—Lab", 2, t(9, 0), t(9, 50));
        assert_eq!(classify(&em_dash, &codes, &[]), want_lab, "em dash");
        let slash = weekly("CS 100/Lab", 2, t(9, 0), t(9, 50));
        assert_eq!(classify(&slash, &codes, &[]), want_lab, "slash");
        let bracket = weekly("[CS 100] Lecture", 2, t(9, 0), t(9, 50));
        assert_eq!(
            classify(&bracket, &codes, &[]),
            Some(Class::Kind {
                kind: "class".to_string(),
                course: Some("cs-100".to_string())
            }),
            "leading bracket"
        );
    }

    #[test]
    fn a_courses_own_explicit_code_field_is_never_treated_as_a_term() {
        let v = vault("codes-fa10-exempt");
        course_note(
            &v,
            "fa-10.md",
            "code: \"FA 10\"\ntitle: \"Intro to Fine Arts\"\nslug: fa-10\n",
        );
        let (codes, warnings) = code_table(&v);
        assert!(warnings.is_empty(), "{warnings:?}");
        assert_eq!(codes.get("FA10"), Some(&"fa-10".to_string()));
    }

    #[test]
    fn without_an_explicit_code_field_a_two_digit_term_like_prefix_still_skips() {
        // Regression guard: the exemption is scoped to an explicit `code:` field, so a plain LMS
        // name with a term-shaped prefix and no matching `code:` field is still read past it.
        let v = vault("codes-fa10-not-exempt");
        course_note(&v, "cs-100.md", "name: \"FA26-CS-100-001\"\nslug: cs-100\n");
        let (codes, warnings) = code_table(&v);
        assert!(warnings.is_empty(), "{warnings:?}");
        assert!(!codes.contains_key("FA26"), "{codes:?}");
        assert_eq!(codes.get("CS100"), Some(&"cs-100".to_string()));
    }
}

/// P8's tests: normalisation (§3.2) and the series file (§3.3). Invented data only.
#[cfg(test)]
mod series_tests {
    use super::*;
    use jiff::civil::date;
    use jiff::tz::TimeZone;
    use serde_json::json;

    fn chicago() -> TimeZone {
        TimeZone::get("America/Chicago").unwrap()
    }

    fn t(h: i8, m: i8) -> Time {
        Time::new(h, m, 0, 0).unwrap()
    }

    fn plus(d: Date, days: i64) -> Date {
        d.checked_add(jiff::Span::new().days(days)).unwrap()
    }

    /// A scratch vault whose `config/ingest.yaml` names `personal` (a direct ICS feed) and the
    /// Google grant (`cloud:google`), plus any extra feed names.
    fn vault(name: &str, extra: &[&str]) -> PathBuf {
        let dir = std::env::temp_dir().join(format!("knowlu-ser-{}-{name}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(dir.join("config")).unwrap();
        config(&dir, true, extra);
        dir
    }

    fn config(dir: &Path, google: bool, extra: &[&str]) {
        let mut text = String::from(
            "timezone: America/Chicago\ncalendars:\n  - name: personal\n    ics_url: https://a.test/a.ics\n",
        );
        if google {
            text.push_str("  - name: google\n    ics_url: 'cloud:google'\n");
        }
        for name in extra {
            text.push_str(&format!("  - name: {name}\n    ics_url: https://b.test/{name}.ics\n"));
        }
        std::fs::write(dir.join("config").join("ingest.yaml"), text).unwrap();
    }

    fn file_text(v: &Path) -> Option<String> {
        std::fs::read_to_string(v.join(SERIES_FILE)).ok()
    }

    /// A plain weekly series on `dates`, 09:00–09:50, as a normaliser would hand it over.
    fn ser(uid: &str, cal: &str, dates: &[Date]) -> Series {
        let instances: Vec<Instance> = dates
            .iter()
            .map(|d| Instance { date: *d, start: Some(t(9, 0)), end: Some(t(9, 50)) })
            .collect();
        Series {
            source_uid: uid.to_string(),
            calendar: cal.to_string(),
            title: format!("Invented {uid}"),
            where_: None,
            event_type: None,
            rule: Rule { freq: "WEEKLY".into(), interval: 1, until: None, count: None },
            has_master: true,
            rdate: false,
            unsupported: false,
            meets: meets_of(&instances),
            instances,
            first: dates.first().copied(),
            until: None,
            last_seen: None,
        }
    }

    fn keys(file: &SeriesFile) -> Vec<(String, String)> {
        file.series.iter().map(|s| (s.source_uid.clone(), s.calendar.clone())).collect()
    }

    fn pair(uid: &str, cal: &str) -> (String, String) {
        (uid.to_string(), cal.to_string())
    }

    /// §4.1's example reply, with instances over two weeks around `today` = 2026-09-21 (a Monday).
    fn google_reply() -> serde_json::Value {
        let days = ["2026-09-18", "2026-09-21", "2026-09-23", "2026-09-25", "2026-09-28", "2026-09-30", "2026-10-02"];
        let instances: Vec<serde_json::Value> = days
            .iter()
            .map(|d| json!({"start": format!("{d}T17:00:00Z"), "end": format!("{d}T17:50:00Z")}))
            .collect();
        json!({"calendars_read": ["google:3b9e0c1d2a4f5e60"],
               "items": [{"calendar": "google:3b9e0c1d2a4f5e60", "id": "4k2q9x7m1abc",
                          "title": "CS 100", "location": "", "description": "Room 101",
                          "event_type": "default", "first": "2026-08-19T12:00:00-05:00",
                          "recurrence": ["RRULE:FREQ=WEEKLY;BYDAY=MO,WE,FR;UNTIL=20261205T055959Z"],
                          "instances": instances}]})
    }

    #[test]
    fn google_value_normalises_to_one_series_with_meets_and_until() {
        let today = date(2026, 9, 21);
        let (series, read, warnings) = series_from_google(&google_reply(), &chicago(), today);
        assert!(warnings.is_empty(), "{warnings:?}");
        assert_eq!(read, vec!["google:3b9e0c1d2a4f5e60".to_string()]);
        assert_eq!(series.len(), 1);
        let s = &series[0];
        assert_eq!(s.source_uid, "gcal-series:4k2q9x7m1abc");
        assert_eq!(s.calendar, "google:3b9e0c1d2a4f5e60");
        assert_eq!(s.title, "CS 100");
        assert_eq!(s.where_.as_deref(), Some("Room 101"));
        assert_eq!(s.event_type.as_deref(), Some("default"));
        assert_eq!(
            s.rule,
            Rule { freq: "WEEKLY".into(), interval: 1, until: Some(date(2026, 12, 4)), count: None }
        );
        assert!(s.has_master && !s.rdate && !s.unsupported);
        // 2026-09-18 is before today: outside [today, today + 28).
        assert_eq!(s.instances.len(), 6);
        assert_eq!(s.instances[0], Instance { date: today, start: Some(t(12, 0)), end: Some(t(12, 50)) });
        assert_eq!(s.meets, vec![Meet { days: vec!["mon", "wed", "fri"], start: t(12, 0), end: t(12, 50) }]);
        assert_eq!(s.first, Some(date(2026, 8, 19)));
        assert_eq!(s.until, Some(date(2026, 12, 4)));
        assert_eq!(s.last_seen, Some(today));

        // §3.3's example shape, through refresh_series.
        let v = vault("google-example", &[]);
        let fresh = vec![("google:3b9e0c1d2a4f5e60".to_string(), series)];
        let (_, warnings) = refresh_series(&v, &fresh, today);
        assert!(warnings.is_empty(), "{warnings:?}");
        let inst = |d: &str| format!("[\"{d}\", \"12:00\", \"12:50\"]");
        let expected = format!(
            "{{\"calendars\": {{\"google:3b9e0c1d2a4f5e60\": \"2026-09-21\"}}, \"ended\": {{}}, \
             \"series\": [{{\"calendar\": \"google:3b9e0c1d2a4f5e60\", \"event_type\": \"default\", \
             \"first\": \"2026-08-19\", \"has_master\": true, \"instances\": [{}], \
             \"last_seen\": \"2026-09-21\", \
             \"meets\": [{{\"days\": [\"mon\", \"wed\", \"fri\"], \"end\": \"12:50\", \"start\": \"12:00\"}}], \
             \"rdate\": false, \
             \"rule\": {{\"count\": null, \"freq\": \"WEEKLY\", \"interval\": 1, \"until\": \"2026-12-04\"}}, \
             \"source_uid\": \"gcal-series:4k2q9x7m1abc\", \"title\": \"CS 100\", \"unsupported\": false, \
             \"until\": \"2026-12-04\", \"where\": \"Room 101\"}}]}}\n",
            ["2026-09-21", "2026-09-23", "2026-09-25", "2026-09-28", "2026-09-30", "2026-10-02"]
                .iter()
                .map(|d| inst(d))
                .collect::<Vec<_>>()
                .join(", ")
        );
        assert_eq!(file_text(&v).unwrap(), expected);
    }

    /// One Google item with the given recurrence, `first` and Tuesday/Thursday instances.
    fn google_one(recurrence: &[&str], first: &str, location: &str, description: &str) -> serde_json::Value {
        let instances: Vec<serde_json::Value> = ["2026-09-22", "2026-09-24", "2026-09-29", "2026-10-01"]
            .iter()
            .map(|d| json!({"start": format!("{d}T14:30:00Z"), "end": format!("{d}T15:45:00Z")}))
            .collect();
        json!({"calendars_read": ["google:aa"],
               "items": [{"calendar": "google:aa", "id": "inv1", "title": "Invented Seminar",
                          "location": location, "description": description, "event_type": "default",
                          "first": first, "recurrence": recurrence, "instances": instances}]})
    }

    #[test]
    fn until_z_is_a_local_date_in_the_vault_timezone() {
        let reply = google_one(
            &["RRULE:FREQ=WEEKLY;BYDAY=TU,TH;UNTIL=20261205T055959Z"],
            "2026-09-01T09:30:00-05:00",
            "Hall 1",
            "",
        );
        let today = date(2026, 9, 21);
        let (chi, _, _) = series_from_google(&reply, &chicago(), today);
        assert_eq!(chi[0].rule.until, Some(date(2026, 12, 4)));
        assert_eq!(chi[0].until, Some(date(2026, 12, 4)));
        let (utc, _, _) = series_from_google(&reply, &TimeZone::UTC, today);
        assert_eq!(utc[0].until, Some(date(2026, 12, 5)));
        // A bare-date UNTIL is that date.
        let reply = google_one(&["RRULE:FREQ=WEEKLY;BYDAY=TU,TH;UNTIL=20261204"], "2026-09-01T09:30:00-05:00", "Hall 1", "");
        let (s, _, _) = series_from_google(&reply, &chicago(), today);
        assert_eq!(s[0].until, Some(date(2026, 12, 4)));
    }

    #[test]
    fn count_gives_until() {
        // Tue 2026-09-01, Thu 09-03, Tue 09-08, …: the 10th occurrence is Thu 2026-10-01.
        let reply = google_one(&["RRULE:FREQ=WEEKLY;BYDAY=TU,TH;COUNT=10"], "2026-09-01T09:30:00-05:00", "Hall 1", "");
        let (s, _, _) = series_from_google(&reply, &chicago(), date(2026, 9, 21));
        assert_eq!(s[0].rule.count, Some(10));
        assert_eq!(s[0].rule.until, None);
        assert_eq!(s[0].until, Some(date(2026, 10, 1)));
        assert_eq!(s[0].first, Some(date(2026, 9, 1)));
        // Neither UNTIL nor COUNT: no until.
        let reply = google_one(&["RRULE:FREQ=WEEKLY;BYDAY=TU,TH"], "2026-09-01T09:30:00-05:00", "Hall 1", "");
        let (s, _, _) = series_from_google(&reply, &chicago(), date(2026, 9, 21));
        assert_eq!(s[0].until, None);
        assert!(!s[0].unsupported);
        // An RDATE is irregular; a rule calfeed cannot expand is unsupported; no master at all.
        let reply = google_one(&["RRULE:FREQ=WEEKLY;BYDAY=TU", "RDATE:20261002T093000"], "2026-09-01T09:30:00-05:00", "", "");
        assert!(series_from_google(&reply, &chicago(), date(2026, 9, 21)).0[0].rdate);
        let reply = google_one(&["RRULE:FREQ=MONTHLY;BYDAY=1TU"], "2026-09-01T09:30:00-05:00", "", "");
        let (s, _, _) = series_from_google(&reply, &chicago(), date(2026, 9, 21));
        assert!(s[0].unsupported);
        assert_eq!(s[0].rule.freq, "MONTHLY");
        let mut reply = google_one(&[], "", "", "");
        let item = reply["items"][0].as_object_mut().unwrap();
        item.remove("first");
        item.remove("recurrence");
        let (s, _, w) = series_from_google(&reply, &chicago(), date(2026, 9, 21));
        assert!(w.is_empty(), "{w:?}");
        assert!(!s[0].has_master);
        assert_eq!((s[0].first, s[0].until), (None, None));
    }

    #[test]
    fn an_unreadable_google_item_takes_its_calendar_out_of_the_fresh_read() {
        let d0 = date(2026, 9, 21);
        let good = google_one(&["RRULE:FREQ=WEEKLY;BYDAY=TU,TH"], "2026-09-01T09:30:00-05:00", "Hall 1", "");
        let mut reply = good.clone();
        reply["calendars_read"] = json!(["google:aa", "google:bb"]);
        let mut hidden = good["items"][0].clone();
        hidden["calendar"] = json!("google:bb");
        hidden["id"] = json!("inv2");
        let mut broken = hidden.clone();
        broken["id"] = json!("inv3");
        broken["title"] = json!(5);
        reply["items"].as_array_mut().unwrap().extend([hidden, broken]);
        let (series, read, warnings) = series_from_google(&reply, &chicago(), d0);
        assert_eq!(read, vec!["google:aa".to_string()]);
        assert!(series.iter().all(|s| s.calendar == "google:aa"), "{series:?}");
        assert_eq!(warnings.len(), 1, "{warnings:?}");
        assert!(warnings[0].contains("google:bb"), "{warnings:?}");

        // A file already holding google:bb's series: the partial read neither replaces nor ages it.
        let v = vault("google-partial", &[]);
        let old = ser("gcal-series:old", "google:bb", &[d0]);
        refresh_series(&v, &[("google:bb".into(), vec![old])], d0);
        let before = read_series_file(&v).0;
        let later = plus(d0, 30);
        let fresh: Vec<(String, Vec<Series>)> = read
            .iter()
            .map(|cal| (cal.clone(), series.iter().filter(|s| &s.calendar == cal).cloned().collect()))
            .collect();
        let (file, _) = refresh_series(&v, &fresh, later);
        assert_eq!(file.calendars.get("google:bb"), Some(&d0));
        let bb: Vec<&Series> = file.series.iter().filter(|s| s.calendar == "google:bb").collect();
        assert_eq!(bb, before.series.iter().collect::<Vec<_>>());
        assert!(file.ended.is_empty());

        // No items list at all: no calendar is read.
        let (s, read, w) = series_from_google(&json!({"calendars_read": ["google:aa"]}), &chicago(), d0);
        assert!(s.is_empty() && read.is_empty());
        assert_eq!(w.len(), 1);
    }

    #[test]
    fn a_calendar_key_without_the_google_prefix_is_skipped_with_a_warning() {
        // Fix round 1, M2: a Google read may never replace an ICS feed's series.
        let mut reply = google_one(&["RRULE:FREQ=WEEKLY;BYDAY=TU,TH"], "2026-09-01T09:30:00-05:00", "Hall 1", "");
        reply["calendars_read"] = json!(["google:aa", "personal"]);
        let mut twin = reply["items"][0].clone();
        twin["calendar"] = json!("personal");
        reply["items"].as_array_mut().unwrap().push(twin);
        let (series, read, warnings) = series_from_google(&reply, &chicago(), date(2026, 9, 21));
        assert_eq!(read, vec!["google:aa".to_string()]);
        assert_eq!(series.len(), 1);
        assert_eq!(series[0].calendar, "google:aa");
        assert_eq!(warnings.len(), 1, "{warnings:?}");
    }

    #[test]
    fn an_event_type_is_cut_to_40() {
        // Fix round 1, M4: bounded, and never emptied into `None` (which would read as eligible).
        let mut reply = google_one(&["RRULE:FREQ=WEEKLY;BYDAY=TU,TH"], "2026-09-01T09:30:00-05:00", "Hall 1", "");
        reply["items"][0]["event_type"] = json!("x".repeat(90));
        let (series, _, _) = series_from_google(&reply, &chicago(), date(2026, 9, 21));
        assert_eq!(series[0].event_type.as_ref().unwrap().chars().count(), 40);
    }

    #[test]
    fn meets_keeps_triples_seen_twice_and_groups_by_time() {
        let i = |d: Date, s: Time, e: Time| Instance { date: d, start: Some(s), end: Some(e) };
        let mon = date(2026, 9, 21);
        let instances = vec![
            i(mon, t(9, 0), t(9, 50)),
            i(plus(mon, 7), t(9, 0), t(9, 50)),
            i(plus(mon, 2), t(9, 0), t(9, 50)),
            i(plus(mon, 9), t(9, 0), t(9, 50)),
            i(plus(mon, 1), t(14, 0), t(15, 15)),
            i(plus(mon, 8), t(14, 0), t(15, 15)),
            i(plus(mon, 3), t(14, 0), t(15, 15)),
            i(plus(mon, 10), t(14, 0), t(15, 15)),
            i(mon, t(16, 0), t(17, 0)),
            i(plus(mon, 7), t(16, 0), t(17, 0)),
            // Seen once: a moved instance, not a meeting time.
            i(plus(mon, 4), t(9, 0), t(9, 50)),
            // All-day instances carry no triple.
            Instance { date: plus(mon, 5), start: None, end: None },
            Instance { date: plus(mon, 12), start: None, end: None },
        ];
        assert_eq!(
            meets_of(&instances),
            vec![
                Meet { days: vec!["mon", "wed"], start: t(9, 0), end: t(9, 50) },
                Meet { days: vec!["mon"], start: t(16, 0), end: t(17, 0) },
                Meet { days: vec!["tue", "thu"], start: t(14, 0), end: t(15, 15) },
            ]
        );
    }

    #[test]
    fn where_takes_the_location_else_a_place_like_description_line() {
        assert_eq!(where_of("  Hall 2  ", "Room 9"), Some("Hall 2".to_string()));
        assert_eq!(where_of("", "\n  Science Hall 3  \nRoom 9"), Some("Science Hall 3".to_string()));
        assert_eq!(
            where_of("   ", "https://zoom.us/j/1\nPasscode: 4242\nRoom 101\n"),
            Some("Room 101".to_string())
        );
        let long = "x".repeat(81);
        assert_eq!(where_of("", &format!("{long}\nRoom 7")), Some("Room 7".to_string()));
        assert_eq!(where_of("", ""), None);
        assert_eq!(where_of("", "\n \n"), None);
    }

    #[test]
    fn a_zoom_link_or_passcode_line_is_never_where() {
        for line in [
            "https://example.test/room",
            "see www.example.test",
            "Zoom room B",
            "MS Teams call",
            "meet.google.com/abc-defg-hij",
            "Webex 5",
            "pwd=abcd",
            "Passcode 42",
            "Password: invented",
            "PIN 4242",
            "Meeting ID 5",
            "Dial 555123456",
            "Dial 555 123 456",
            "Call 205-555-0100",
        ] {
            assert_eq!(where_of("", line), None, "{line}");
        }
        // A meeting link as the location is never stored either (§9); the description may still
        // name the room.
        assert_eq!(where_of("https://zoom.us/j/1?pwd=abc", "Room 3"), Some("Room 3".to_string()));
        assert_eq!(where_of("Zoom", ""), None);
        // Fix round 1, I1: the single words match on letter boundaries only.
        for line in ["PIN1234", "pin: 1234", "PIN: 42", "pwd=abcd", "us02web.zoom.us/j/1", "Teams-call"] {
            assert_eq!(where_of("", line), None, "{line}");
            assert_eq!(where_of(line, ""), None, "{line}");
        }
    }

    #[test]
    fn a_building_name_holding_a_meeting_word_keeps_its_where() {
        for place in ["Pine Hall 2", "Chapin Hall", "Spinning Studio", "Steamship Rm", "Shipping Office", "Opinion Lab"] {
            assert_eq!(where_of(place, ""), Some(place.to_string()), "{place}");
            assert_eq!(where_of("", place), Some(place.to_string()), "{place}");
        }
    }

    #[test]
    fn a_multi_line_location_keeps_its_first_line() {
        // Fix round 1, M3: `where` is one line.
        assert_eq!(where_of("\n Hall 4 \nsecond line", ""), Some("Hall 4".to_string()));
        // A first line that is a link is never stored; the description may still name the room.
        assert_eq!(where_of("https://example.test/x\nHall 4", "Room 3"), Some("Room 3".to_string()));
    }

    /// A one-master ICS feed: Tuesdays and Thursdays 09:30–10:45 from 2026-09-01.
    fn ics(summary: &str, location: &str, description: &str) -> String {
        let mut lines = vec![
            "BEGIN:VCALENDAR".to_string(),
            "VERSION:2.0".to_string(),
            "BEGIN:VEVENT".to_string(),
            "UID:inv-seminar@example.test".to_string(),
            format!("SUMMARY:{summary}"),
        ];
        if !location.is_empty() {
            lines.push(format!("LOCATION:{location}"));
        }
        if !description.is_empty() {
            lines.push(format!("DESCRIPTION:{description}"));
        }
        lines.extend(
            [
                "DTSTART;TZID=America/Chicago:20260901T093000",
                "DTEND;TZID=America/Chicago:20260901T104500",
                "RRULE:FREQ=WEEKLY;BYDAY=TU,TH;UNTIL=20261205T055959Z",
                "END:VEVENT",
                "END:VCALENDAR",
            ]
            .iter()
            .map(|l| l.to_string()),
        );
        lines.join("\r\n") + "\r\n"
    }

    #[test]
    fn the_description_is_never_in_the_file() {
        let today = date(2026, 9, 21);
        let v = vault("no-description", &[]);
        // Google: no location, and no description line is place-like.
        let secret = "https://zoom.us/j/99 pwd=SECRETCODE\\nMeeting ID 555 1234 9999";
        let reply = google_one(&["RRULE:FREQ=WEEKLY;BYDAY=TU,TH"], "2026-09-01T09:30:00-05:00", "", &secret.replace("\\n", "\n"));
        let (g, read, _) = series_from_google(&reply, &chicago(), today);
        assert_eq!(g[0].where_, None);
        // ICS: a location, and a description that would be place-like on its own.
        let (i, _) = series_from_ics("personal", &ics("Invented Seminar", "Hall 4", "Private agenda line"), &chicago(), today);
        let i = i.unwrap();
        assert_eq!(i[0].where_.as_deref(), Some("Hall 4"));
        let fresh = vec![(read[0].clone(), g), ("personal".to_string(), i)];
        refresh_series(&v, &fresh, today);
        let text = file_text(&v).unwrap();
        for leaked in ["SECRETCODE", "zoom", "Meeting ID", "Private agenda", "description"] {
            assert!(!text.contains(leaked), "{leaked} in {text}");
        }
    }

    #[test]
    fn an_ics_title_is_cut_to_200_and_where_to_80() {
        let today = date(2026, 9, 21);
        let title = "T".repeat(250);
        let place = "P".repeat(120);
        let (got, warnings) = series_from_ics("personal", &ics(&title, &place, ""), &chicago(), today);
        assert!(warnings.is_empty(), "{warnings:?}");
        let got = got.unwrap();
        assert_eq!(got.len(), 1);
        let s = &got[0];
        assert_eq!(s.title.chars().count(), 200);
        assert_eq!(s.where_.as_ref().unwrap().chars().count(), 80);
        // The rest of the ICS route, while here.
        assert_eq!(s.source_uid, "ics-series:inv-seminar@example.test");
        assert_eq!(s.calendar, "personal");
        assert_eq!(s.event_type, None);
        assert_eq!(s.rule.until, Some(date(2026, 12, 4)));
        assert_eq!(s.first, Some(date(2026, 9, 1)));
        assert_eq!(s.meets, vec![Meet { days: vec!["tue", "thu"], start: t(9, 30), end: t(10, 45) }]);
        assert_eq!(s.instances.first().unwrap().date, date(2026, 9, 22));
        assert_eq!(s.instances.last().unwrap().date, date(2026, 10, 15));
        assert_eq!(s.last_seen, Some(today));
        // A Google title is cut on the device too.
        let mut reply = google_one(&["RRULE:FREQ=WEEKLY;BYDAY=TU,TH"], "2026-09-01T09:30:00-05:00", &place, "");
        reply["items"][0]["title"] = json!(title);
        let (g, _, _) = series_from_google(&reply, &chicago(), today);
        assert_eq!(g[0].title.chars().count(), 200);
        assert_eq!(g[0].where_.as_ref().unwrap().chars().count(), 80);
    }

    #[test]
    fn a_fresh_calendar_replaces_its_series() {
        let d0 = date(2026, 9, 21);
        let v = vault("replaces", &[]);
        let (a, b) = (ser("ics-series:a", "personal", &[d0]), ser("ics-series:b", "personal", &[d0]));
        refresh_series(&v, &[("personal".into(), vec![a.clone(), b])], d0);
        let d1 = plus(d0, 1);
        let mut a2 = a.clone();
        a2.title = "Invented renamed".into();
        let c = ser("ics-series:c", "personal", &[d1]);
        let (file, warnings) = refresh_series(&v, &[("personal".into(), vec![a2, c])], d1);
        assert!(warnings.is_empty(), "{warnings:?}");
        assert_eq!(file.calendars.get("personal"), Some(&d1));
        assert_eq!(
            keys(&file),
            vec![pair("ics-series:a", "personal"), pair("ics-series:b", "personal"), pair("ics-series:c", "personal")]
        );
        assert_eq!(file.series[0].title, "Invented renamed");
        assert_eq!(file.series[0].last_seen, Some(d1));
        // b went unseen: kept with its old last_seen.
        assert_eq!(file.series[1].last_seen, Some(d0));
        assert_eq!(read_series_file(&v).0, file);
    }

    #[test]
    fn a_series_unseen_for_14_days_is_dropped_not_before() {
        let d0 = date(2026, 9, 21);
        let v = vault("unseen-14", &[]);
        let (a, b) = (ser("ics-series:a", "personal", &[d0]), ser("ics-series:b", "personal", &[d0]));
        refresh_series(&v, &[("personal".into(), vec![a.clone(), b])], d0);
        let (file, _) = refresh_series(&v, &[("personal".into(), vec![a.clone()])], plus(d0, 13));
        assert_eq!(keys(&file), vec![pair("ics-series:a", "personal"), pair("ics-series:b", "personal")]);
        let (file, _) = refresh_series(&v, &[("personal".into(), vec![a])], plus(d0, 14));
        assert_eq!(keys(&file), vec![pair("ics-series:a", "personal")]);
    }

    #[test]
    fn an_unread_calendar_keeps_its_series_and_date() {
        let d0 = date(2026, 9, 21);
        let v = vault("unread", &[]);
        let a = ser("ics-series:a", "personal", &[d0]);
        let g = ser("gcal-series:g", "google:aa", &[d0]);
        refresh_series(&v, &[("personal".into(), vec![a.clone()]), ("google:aa".into(), vec![g])], d0);
        let (file, _) = refresh_series(&v, &[("personal".into(), vec![a])], plus(d0, 40));
        assert_eq!(file.calendars.get("google:aa"), Some(&d0));
        assert_eq!(file.calendars.get("personal"), Some(&plus(d0, 40)));
        let g = file.series.iter().find(|s| s.calendar == "google:aa").unwrap();
        assert_eq!(g.last_seen, Some(d0));
    }

    #[test]
    fn an_ics_fetched_with_zero_series_is_fresh_and_ages_its_old_series() {
        let d0 = date(2026, 9, 21);
        let empty = "BEGIN:VCALENDAR\r\nVERSION:2.0\r\nEND:VCALENDAR\r\n";
        assert_eq!(series_from_ics("personal", empty, &chicago(), d0).0, Some(Vec::new()));
        assert_eq!(series_from_ics("personal", "<html>sign in</html>", &chicago(), d0).0, None);
        let v = vault("zero-fresh", &[]);
        refresh_series(&v, &[("personal".into(), vec![ser("ics-series:a", "personal", &[d0])])], d0);
        let (file, _) = refresh_series(&v, &[("personal".into(), Vec::new())], plus(d0, 5));
        assert_eq!(file.calendars.get("personal"), Some(&plus(d0, 5)));
        assert_eq!(keys(&file), vec![pair("ics-series:a", "personal")]);
        let (file, _) = refresh_series(&v, &[("personal".into(), Vec::new())], plus(d0, 14));
        assert!(file.series.is_empty());
        assert!(file.ended.contains_key("ics-series:a"));
    }

    #[test]
    fn a_removed_feed_keeps_its_series_14_days_then_drops_them() {
        let d0 = date(2026, 9, 21);
        let v = vault("removed-feed", &["clients"]);
        let a = ser("ics-series:a", "personal", &[d0]);
        let k = ser("ics-series:k", "clients", &[d0]);
        refresh_series(&v, &[("personal".into(), vec![a.clone()]), ("clients".into(), vec![k])], d0);
        config(&v, true, &[]);
        let (file, _) = refresh_series(&v, &[("personal".into(), vec![a.clone()])], plus(d0, 13));
        assert_eq!(keys(&file), vec![pair("ics-series:a", "personal"), pair("ics-series:k", "clients")]);
        assert_eq!(file.calendars.get("clients"), Some(&d0));
        let (file, _) = refresh_series(&v, &[("personal".into(), vec![a])], plus(d0, 14));
        assert_eq!(keys(&file), vec![pair("ics-series:a", "personal")]);
        assert!(!file.calendars.contains_key("clients"));
        assert!(file.ended.is_empty());
    }

    #[test]
    fn a_removed_feeds_series_never_enter_ended() {
        let d0 = date(2026, 9, 21);
        let v = vault("removed-not-ended", &["clients"]);
        let k = ser("ics-series:k", "clients", &[d0]);
        refresh_series(&v, &[("clients".into(), vec![k])], d0);
        config(&v, true, &[]);
        for day in [14, 20, 40] {
            let (file, _) = refresh_series(&v, &[("personal".into(), Vec::new())], plus(d0, day));
            assert!(file.series.is_empty(), "day {day}");
            assert!(file.ended.is_empty(), "day {day}: {:?}", file.ended);
        }
    }

    #[test]
    fn a_google_calendar_key_ages_as_removed_when_cloud_google_leaves_the_config() {
        let d0 = date(2026, 9, 21);
        let v = vault("google-removed", &[]);
        let g = ser("gcal-series:g", "google:aa", &[d0]);
        refresh_series(&v, &[("google:aa".into(), vec![g])], d0);
        // Still configured, never read again: untouched however old.
        let (file, _) = refresh_series(&v, &[("personal".into(), Vec::new())], plus(d0, 60));
        assert_eq!(keys(&file), vec![pair("gcal-series:g", "google:aa")]);
        config(&v, false, &[]);
        let (file, _) = refresh_series(&v, &[("personal".into(), Vec::new())], plus(d0, 13));
        assert_eq!(keys(&file), vec![pair("gcal-series:g", "google:aa")]);
        let (file, _) = refresh_series(&v, &[("personal".into(), Vec::new())], plus(d0, 14));
        assert!(file.series.is_empty());
        assert!(!file.calendars.contains_key("google:aa"));
        assert!(file.ended.is_empty());
    }

    #[test]
    fn an_aged_out_series_moves_to_ended_with_its_last_instance_and_until() {
        let d0 = date(2026, 9, 21);
        let v = vault("ended", &[]);
        let mut a = ser("ics-series:a", "personal", &[plus(d0, 1), plus(d0, 8)]);
        a.until = Some(date(2026, 12, 4));
        refresh_series(&v, &[("personal".into(), vec![a])], d0);
        let drop_day = plus(d0, 14);
        let (file, _) = refresh_series(&v, &[("personal".into(), Vec::new())], drop_day);
        assert!(file.series.is_empty());
        assert_eq!(
            file.ended.get("ics-series:a"),
            Some(&Ended {
                calendar: "personal".into(),
                dropped: drop_day,
                last_instance: Some(date(2026, 9, 29)),
                until: Some(date(2026, 12, 4)),
            })
        );
        assert!(file_text(&v).unwrap().contains(
            "\"ended\": {\"ics-series:a\": {\"calendar\": \"personal\", \"dropped\": \"2026-10-05\", \
             \"last_instance\": \"2026-09-29\", \"until\": \"2026-12-04\"}}"
        ));
        // A series that held no instance ends with `last_instance: null`.
        let v = vault("ended-empty", &[]);
        let b = ser("ics-series:b", "personal", &[]);
        refresh_series(&v, &[("personal".into(), vec![b])], d0);
        let (file, _) = refresh_series(&v, &[("personal".into(), Vec::new())], drop_day);
        assert_eq!(file.ended["ics-series:b"].last_instance, None);
        assert_eq!(file.ended["ics-series:b"].until, None);
    }

    #[test]
    fn an_ended_entry_goes_after_28_days_or_when_the_key_returns() {
        let d0 = date(2026, 9, 21);
        let v = vault("ended-expiry", &[]);
        let a = ser("ics-series:a", "personal", &[d0]);
        refresh_series(&v, &[("personal".into(), vec![a.clone()])], d0);
        let dropped = plus(d0, 14);
        refresh_series(&v, &[("personal".into(), Vec::new())], dropped);
        let (file, _) = refresh_series(&v, &[("personal".into(), Vec::new())], plus(dropped, 27));
        assert!(file.ended.contains_key("ics-series:a"));
        let (file, _) = refresh_series(&v, &[("personal".into(), Vec::new())], plus(dropped, 28));
        assert!(file.ended.is_empty());
        // The key comes back in a fresh read: its entry goes at once.
        let v = vault("ended-returns", &[]);
        refresh_series(&v, &[("personal".into(), vec![a.clone()])], d0);
        refresh_series(&v, &[("personal".into(), Vec::new())], dropped);
        let (file, _) = refresh_series(&v, &[("personal".into(), vec![a])], plus(dropped, 1));
        assert!(file.ended.is_empty());
        assert_eq!(keys(&file), vec![pair("ics-series:a", "personal")]);
    }

    #[test]
    fn a_key_still_held_under_another_calendar_never_enters_ended() {
        let d0 = date(2026, 9, 21);
        let v = vault("ended-held", &[]);
        let k = "gcal-series:twin";
        refresh_series(
            &v,
            &[("personal".into(), vec![ser(k, "personal", &[d0])]), ("google:aa".into(), vec![ser(k, "google:aa", &[d0])])],
            d0,
        );
        let (file, _) = refresh_series(
            &v,
            &[("personal".into(), Vec::new()), ("google:aa".into(), vec![ser(k, "google:aa", &[d0])])],
            plus(d0, 14),
        );
        assert_eq!(keys(&file), vec![pair(k, "google:aa")]);
        assert!(file.ended.is_empty());
    }

    #[test]
    fn the_file_is_written_only_when_fresh_and_changed() {
        let d0 = date(2026, 9, 21);
        let v = vault("write-once", &[]);
        let a = ser("ics-series:a", "personal", &[d0]);
        refresh_series(&v, &[("personal".into(), vec![a.clone()])], d0);
        let path = v.join(SERIES_FILE);
        let before = std::fs::read(&path).unwrap();
        let old = std::time::UNIX_EPOCH + std::time::Duration::from_secs(1_000_000_000);
        std::fs::File::options().write(true).open(&path).unwrap().set_modified(old).unwrap();
        // The same fresh read on the same day: the same bytes, so no write.
        refresh_series(&v, &[("personal".into(), vec![a])], d0);
        assert_eq!(std::fs::read(&path).unwrap(), before);
        assert_eq!(std::fs::metadata(&path).unwrap().modified().unwrap(), old);
        // Nothing fresh: never written, even when aging would change the file.
        refresh_series(&v, &[], plus(d0, 60));
        assert_eq!(std::fs::metadata(&path).unwrap().modified().unwrap(), old);
        assert_eq!(std::fs::read(&path).unwrap(), before);
    }

    #[test]
    fn the_file_bytes_are_sorted_dumps_value_with_a_trailing_newline() {
        let d0 = date(2026, 9, 21);
        let v = vault("bytes", &[]);
        let fresh = vec![
            ("personal".to_string(), vec![ser("ics-series:b", "personal", &[d0]), ser("gcal-series:z", "personal", &[])]),
            ("google:aa".to_string(), vec![ser("gcal-series:z", "google:aa", &[])]),
        ];
        let (file, _) = refresh_series(&v, &fresh, d0);
        assert_eq!(
            keys(&file),
            vec![pair("gcal-series:z", "google:aa"), pair("gcal-series:z", "personal"), pair("ics-series:b", "personal")]
        );
        let text = file_text(&v).unwrap();
        assert!(text.ends_with("}\n") && !text.ends_with("\n\n"));
        let parsed: serde_json::Value = serde_json::from_str(&text).unwrap();
        assert_eq!(text, format!("{}\n", crate::ledger::dumps_value(&parsed)));
        assert_eq!(text, file_bytes(&file));
        assert!(text.starts_with("{\"calendars\": {\"google:aa\": \"2026-09-21\", \"personal\": \"2026-09-21\"}, \"ended\": {}, \"series\": [{\"calendar\": \"google:aa\", \"event_type\": null, \"first\": null, \"has_master\": true, \"instances\": [], "));
        // Round trip: what is read is what was written.
        assert_eq!(read_series_file(&v), (file, Vec::new()));
    }

    #[test]
    fn a_malformed_file_reads_empty_with_one_warning() {
        let v = vault("malformed", &[]);
        assert_eq!(read_series_file(&v), (SeriesFile::default(), Vec::new()));
        std::fs::create_dir_all(v.join("state")).unwrap();
        for bad in ["{not json", "[]", "{\"calendars\": {}, \"ended\": {}, \"series\": [{\"title\": 5}]}"] {
            std::fs::write(v.join(SERIES_FILE), bad).unwrap();
            let (file, warnings) = read_series_file(&v);
            assert_eq!(file, SeriesFile::default(), "{bad}");
            assert_eq!(warnings.len(), 1, "{bad}: {warnings:?}");
        }
        // The next fresh read rewrites it.
        let d0 = date(2026, 9, 21);
        let (file, warnings) = refresh_series(&v, &[("personal".into(), vec![ser("ics-series:a", "personal", &[d0])])], d0);
        assert_eq!(warnings.len(), 1);
        assert_eq!(read_series_file(&v), (file, Vec::new()));
    }

    #[test]
    fn no_fresh_calendar_writes_no_file() {
        let v = vault("no-fresh", &[]);
        let (file, warnings) = refresh_series(&v, &[], date(2026, 9, 21));
        assert_eq!(file, SeriesFile::default());
        assert!(warnings.is_empty());
        assert!(!v.join(SERIES_FILE).exists());
        assert!(!v.join("state").exists());
    }

    #[test]
    fn by_key_takes_one_record_per_key_google_first_then_calendar_key() {
        let d0 = date(2026, 9, 21);
        let mut series = vec![];
        for cal in ["zeta", "alpha", "google:bb", "google:aa"] {
            series.push(ser("gcal-series:k", cal, &[d0]));
        }
        for cal in ["zeta", "alpha"] {
            series.push(ser("ics-series:l", cal, &[d0]));
        }
        let file = SeriesFile { series, ..SeriesFile::default() };
        let by = file.by_key();
        assert_eq!(by.len(), 2);
        assert_eq!(by["gcal-series:k"].calendar, "google:aa");
        assert_eq!(by["ics-series:l"].calendar, "alpha");
    }

    #[test]
    fn instances_map_prefers_the_google_calendar() {
        let d0 = date(2026, 9, 21);
        let mut g = ser("gcal-series:k", "google:aa", &[plus(d0, 1)]);
        g.instances[0].start = Some(t(12, 0));
        g.instances[0].end = Some(t(12, 50));
        let p = ser("gcal-series:k", "personal", &[plus(d0, 1)]);
        let mut day = ser("ics-series:allday", "personal", &[]);
        day.instances.push(Instance { date: plus(d0, 2), start: None, end: None });
        let mut calendars = BTreeMap::new();
        calendars.insert("google:aa".to_string(), d0);
        calendars.insert("personal".to_string(), plus(d0, 3));
        // As refresh_series leaves them: each seen on its calendar's last read.
        let (mut g, mut p, mut day) = (g, p, day);
        g.last_seen = Some(d0);
        p.last_seen = Some(plus(d0, 3));
        day.last_seen = Some(plus(d0, 3));
        let file = SeriesFile { calendars, ended: BTreeMap::new(), series: vec![g, p, day] };
        let map = file.instances_map();
        assert_eq!(
            map["gcal-series:k"],
            (d0, plus(d0, 28), vec![(plus(d0, 1), t(12, 0), t(12, 50))])
        );
        // An all-day instance has no span to draw.
        assert_eq!(map["ics-series:allday"], (plus(d0, 3), plus(d0, 31), Vec::new()));
    }

    #[test]
    fn a_series_its_fresh_calendar_stopped_returning_blocks_no_time_in_the_horizon() {
        // Fix round 1, M1 (R21): the fresh read says it has no instance in [read, read + 28), so
        // it subtracts nothing there at once; the record (and its instances, for P12's
        // `last_instance`) stays until it ages into `ended`.
        let d0 = date(2026, 9, 21);
        let v = vault("stale-instances", &[]);
        let a = ser("ics-series:a", "personal", &[plus(d0, 1), plus(d0, 8)]);
        let b = ser("ics-series:b", "personal", &[plus(d0, 2)]);
        refresh_series(&v, &[("personal".into(), vec![a, b.clone()])], d0);
        let d1 = plus(d0, 3);
        let (file, _) = refresh_series(&v, &[("personal".into(), vec![b])], d1);
        let map = file.instances_map();
        assert_eq!(map["ics-series:a"], (d1, plus(d1, 28), Vec::new()));
        assert_eq!(map["ics-series:b"].2.len(), 1);
        let a = file.series.iter().find(|s| s.source_uid == "ics-series:a").unwrap();
        assert_eq!(a.instances.len(), 2);
    }

    #[test]
    fn an_unreadable_file_is_left_alone_and_not_rewritten() {
        // Fix round 1, M6: an I/O error (here: the path is a folder) is not a corrupt file.
        let v = vault("io-error", &[]);
        std::fs::create_dir_all(v.join(SERIES_FILE)).unwrap();
        let d0 = date(2026, 9, 21);
        let (_, warnings) = refresh_series(&v, &[("personal".into(), vec![ser("ics-series:a", "personal", &[d0])])], d0);
        assert_eq!(warnings.len(), 1, "{warnings:?}");
        assert!(warnings[0].contains("not written"), "{warnings:?}");
        assert!(v.join(SERIES_FILE).is_dir());
        let (file, warnings) = read_series_file(&v);
        assert_eq!(file, SeriesFile::default());
        assert_eq!(warnings.len(), 1);
    }

    #[test]
    fn the_file_is_replaced_whole_leaving_no_temporary_file() {
        // Fix round 1, M5: written beside itself, then renamed over.
        let d0 = date(2026, 9, 21);
        let v = vault("atomic", &[]);
        refresh_series(&v, &[("personal".into(), vec![ser("ics-series:a", "personal", &[d0])])], d0);
        let (file, _) = refresh_series(&v, &[("personal".into(), vec![ser("ics-series:b", "personal", &[d0])])], plus(d0, 1));
        let names: Vec<String> = std::fs::read_dir(v.join("state"))
            .unwrap()
            .map(|e| e.unwrap().file_name().to_string_lossy().into_owned())
            .collect();
        assert_eq!(names, vec!["calendar-series.json".to_string()]);
        assert_eq!(read_series_file(&v), (file, Vec::new()));
    }
}

#[cfg(test)]
mod proposal_tests {
    use super::*;
    use crate::weekcal::WeekCalendar;
    use jiff::civil::date;

    const TODAY: Date = date(2026, 9, 1);

    fn t(h: i8, m: i8) -> Time {
        Time::new(h, m, 0, 0).unwrap()
    }

    /// Three weeks of a weekly series from Monday 2026-08-31, on `days`, `s`–`e`; `meets` as P8
    /// derives it. Every title, key and calendar is invented.
    fn mk(uid: &str, cal: &str, title: &str, days: &[DayKey], s: Time, e: Time) -> Series {
        let monday = date(2026, 8, 31);
        let mut instances = Vec::new();
        for week in 0..3 {
            for day in days {
                let offset = DAY_KEYS.iter().position(|k| k == day).unwrap() as i64;
                instances.push(Instance {
                    date: add_days(monday, week * 7 + offset),
                    start: Some(s),
                    end: Some(e),
                });
            }
        }
        instances.sort_by_key(|i| i.date);
        let meets = meets_of(&instances);
        Series {
            source_uid: uid.to_string(),
            calendar: cal.to_string(),
            title: title.to_string(),
            where_: None,
            event_type: None,
            rule: Rule {
                freq: "WEEKLY".into(),
                interval: 1,
                until: None,
                count: None,
            },
            has_master: true,
            rdate: false,
            unsupported: false,
            instances,
            meets,
            first: Some(monday),
            until: None,
            last_seen: Some(TODAY),
        }
    }

    fn g(uid: &str, title: &str, days: &[DayKey], s: Time, e: Time) -> Series {
        mk(uid, "google:abc", title, days, s, e)
    }

    fn file_of(series: Vec<Series>) -> SeriesFile {
        let mut file = SeriesFile::default();
        for s in &series {
            file.calendars.insert(s.calendar.clone(), TODAY);
        }
        file.series = series;
        file.series
            .sort_by(|a, b| (&a.source_uid, &a.calendar).cmp(&(&b.source_uid, &b.calendar)));
        file
    }

    fn codes() -> Codes {
        Codes {
            table: [("CS100".to_string(), "cs-100".to_string())]
                .into_iter()
                .collect(),
            names: BTreeMap::new(),
        }
    }

    /// `week_template.yaml` absent: 08:00–18:00, `min_block_minutes` 45.
    fn template() -> WeekCalendar {
        WeekCalendar::new(&Mapping::new(), Vec::new())
    }

    fn run_with(
        file: &SeriesFile,
        set: &Commitments,
        held: &BTreeSet<String>,
        for_cards: bool,
    ) -> Vec<Proposal> {
        proposals(file, set, &codes(), &[], &template(), held, TODAY, for_cards)
    }

    fn run(file: &SeriesFile, set: &Commitments) -> Vec<Proposal> {
        run_with(file, set, &BTreeSet::new(), true)
    }

    fn keys(got: &[Proposal]) -> Vec<&str> {
        got.iter().map(|p| p.source_uid.as_str()).collect()
    }

    fn meet(days: &[DayKey], s: Time, e: Time) -> Meet {
        Meet {
            days: days.to_vec(),
            start: s,
            end: e,
        }
    }

    fn confirmed(
        kind: &str,
        title: &str,
        course: Option<&str>,
        meets: Vec<Meet>,
        uid: Option<&str>,
    ) -> Commitment {
        Commitment {
            id: format!("cmt_{}", title.len()),
            path: PathBuf::from("commitments/x.md"),
            kind: kind.into(),
            level: default_level(kind).unwrap_or(Level::Soft),
            title: title.into(),
            course: course.map(String::from),
            meets,
            where_: None,
            from: None,
            until: None,
            source_uid: uid.map(String::from),
        }
    }

    fn scratch(name: &str) -> PathBuf {
        let v = std::env::temp_dir().join(format!("knowlu-p9-{}-{name}", std::process::id()));
        let _ = std::fs::remove_dir_all(&v);
        std::fs::create_dir_all(v.join(FOLDER)).unwrap();
        v
    }

    fn cs100() -> Series {
        g(
            "gcal-series:cs100",
            "CS 100",
            &["mon", "wed", "fri"],
            t(12, 0),
            t(12, 50),
        )
    }

    fn club() -> Series {
        g("gcal-series:club", "Robotics Club", &["tue"], t(18, 0), t(19, 0))
    }

    fn shift() -> Series {
        g("gcal-series:shift", "Work", &["sat"], t(9, 0), t(13, 0))
    }

    #[test]
    fn a_proposal_carries_what_a_note_needs() {
        let mut s = cs100();
        s.where_ = Some("Room 101".into());
        s.until = Some(date(2026, 12, 4));
        let got = run(&file_of(vec![s]), &Commitments::default());
        assert_eq!(
            got,
            vec![Proposal {
                kind: "class".into(),
                level: Level::Hard,
                title: "CS 100".into(),
                course: Some("cs-100".into()),
                meets: vec![meet(&["mon", "wed", "fri"], t(12, 0), t(12, 50))],
                where_: Some("Room 101".into()),
                from: Some(date(2026, 8, 31)),
                until: Some(date(2026, 12, 4)),
                source_uid: "gcal-series:cs100".into(),
            }]
        );
    }

    #[test]
    fn a_confirmed_or_declined_key_is_never_proposed() {
        let file = file_of(vec![cs100(), club(), shift()]);
        let mut set = Commitments::default();
        // Keyed to the class, but a different signature: the key alone must close it.
        set.confirmed.push(confirmed(
            "class",
            "Old title",
            Some("cs-999"),
            vec![meet(&["thu"], t(8, 0), t(9, 0))],
            Some("gcal-series:cs100"),
        ));
        set.declined.insert("gcal-series:club".into());
        assert_eq!(keys(&run(&file, &set)), vec!["gcal-series:shift"]);
    }

    #[test]
    fn a_signature_match_is_never_proposed() {
        // The same class from a second route: an ICS key, with a note confirmed under a Google key.
        let ics = mk(
            "ics-series:cs100@school.example",
            "personal",
            "CS 100",
            &["mon", "wed", "fri"],
            t(12, 0),
            t(12, 50),
        );
        let mut set = Commitments::default();
        set.confirmed.push(confirmed(
            "class",
            "CS 100 Lecture",
            Some("cs-100"),
            // Days written in another order are the same meets.
            vec![meet(&["fri", "mon", "wed"], t(12, 0), t(12, 50))],
            Some("gcal-series:elsewhere"),
        ));
        let chess = g("gcal-series:chess", "Chess Club", &["wed"], t(18, 0), t(19, 0));
        let file = file_of(vec![ics, club(), chess]);

        // A hand-written club note (no source_uid, another case), read through `load`.
        let v = scratch("sig");
        std::fs::write(
            v.join(FOLDER).join("robotics.md"),
            "---\ntype: commitment\nkind: club\ntitle: \"robotics club\"\n\
             meets: [{days: [tue], start: \"18:00\", end: \"19:00\"}]\nstatus: confirmed\n---\n\nInvented.\n",
        )
        .unwrap();
        let loaded = load(&v);
        assert!(loaded.warnings.is_empty(), "{:?}", loaded.warnings);
        set.confirmed.extend(loaded.confirmed);
        let _ = std::fs::remove_dir_all(&v);

        assert_eq!(keys(&run(&file, &set)), vec!["gcal-series:chess"]);
    }

    #[test]
    fn office_hours_is_proposed_for_the_screen_not_for_cards() {
        let file = file_of(vec![g(
            "gcal-series:oh",
            "CS 100 Office Hours",
            &["thu"],
            t(15, 0),
            t(16, 0),
        )]);
        let none = BTreeSet::new();
        let screen = run_with(&file, &Commitments::default(), &none, false);
        assert_eq!(screen.len(), 1);
        assert_eq!(screen[0].kind, "office-hours");
        assert_eq!(screen[0].course.as_deref(), Some("cs-100"));
        assert_eq!(screen[0].level, Level::Optional);
        assert!(run_with(&file, &Commitments::default(), &none, true).is_empty());
    }

    const WEEKDAYS: [DayKey; 5] = ["mon", "tue", "wed", "thu", "fri"];
    const ALL: [DayKey; 7] = ["mon", "tue", "wed", "thu", "fri", "sat", "sun"];

    fn window_of(got: &[Proposal]) -> Option<&Proposal> {
        got.iter().find(|p| p.kind == PLANNING_DAY)
    }

    #[test]
    fn a_window_is_proposed_per_weekday_from_wake_and_bed() {
        let file = file_of(vec![
            g("gcal-series:wake-wk", "Wake up", &WEEKDAYS, t(7, 0), t(7, 30)),
            g("gcal-series:wake-we", "Wake up", &["sat", "sun"], t(9, 30), t(10, 0)),
            g("gcal-series:bed", "Bedtime", &ALL, t(22, 0), t(22, 15)),
        ]);
        let got = run(&file, &Commitments::default());
        assert_eq!(
            got,
            vec![Proposal {
                kind: PLANNING_DAY.into(),
                level: Level::Optional,
                title: "Your day".into(),
                course: None,
                meets: vec![
                    meet(&WEEKDAYS, t(7, 30), t(22, 0)),
                    meet(&["sat", "sun"], t(10, 0), t(22, 0)),
                ],
                where_: None,
                from: None,
                until: None,
                source_uid: "window:gcal-series:bed,gcal-series:wake-we,gcal-series:wake-wk"
                    .into(),
            }]
        );
    }

    #[test]
    fn the_latest_wake_and_the_earliest_bed_win() {
        let file = file_of(vec![
            g("gcal-series:alarm", "Alarm", &["mon"], t(6, 0), t(6, 0)),
            g("gcal-series:get-up", "Get up", &["mon"], t(7, 0), t(7, 15)),
            g("gcal-series:bed", "Bedtime", &["mon"], t(23, 0), t(23, 5)),
            g("gcal-series:lights", "Lights out", &["mon"], t(22, 30), t(22, 35)),
        ]);
        let got = run(&file, &Commitments::default());
        assert_eq!(
            window_of(&got).unwrap().meets,
            vec![meet(&["mon"], t(7, 15), t(22, 30))]
        );
    }

    #[test]
    fn a_midnight_sleep_series_gives_bed_on_its_day_and_wake_on_the_next() {
        let file = file_of(vec![g(
            "gcal-series:sleep",
            "Sleep",
            &["mon", "tue"],
            t(23, 0),
            t(7, 0),
        )]);
        let got = run(&file, &Commitments::default());
        // Mon: bed 23:00, wake from the template (08:00). Tue: wake 07:00 (Monday night), bed
        // 23:00. Wed: wake 07:00 (Tuesday night), bed from the template (18:00).
        assert_eq!(
            window_of(&got).unwrap().meets,
            vec![
                meet(&["mon"], t(8, 0), t(23, 0)),
                meet(&["tue"], t(7, 0), t(23, 0)),
                meet(&["wed"], t(7, 0), t(18, 0)),
            ]
        );
    }

    #[test]
    fn a_weekday_with_one_side_takes_the_other_from_the_template() {
        let file = file_of(vec![
            g("gcal-series:wake", "Wake up", &["mon"], t(7, 0), t(7, 30)),
            g("gcal-series:bed", "Bedtime", &["sat"], t(23, 0), t(23, 15)),
        ]);
        let got = run(&file, &Commitments::default());
        assert_eq!(
            window_of(&got).unwrap().meets,
            vec![
                meet(&["mon"], t(7, 30), t(18, 0)),
                meet(&["sat"], t(8, 0), t(23, 0))
            ]
        );
    }

    #[test]
    fn an_inverted_or_too_short_day_is_left_out() {
        let file = file_of(vec![
            // Mon: 08:00 (template) to 07:00 — inverted.
            g("gcal-series:bed-mon", "Bedtime", &["mon"], t(7, 0), t(7, 15)),
            // Tue: 17:30 to 18:00 — 30 minutes, under 45.
            g("gcal-series:wake-tue", "Alarm", &["tue"], t(17, 30), t(17, 30)),
            // Wed: 09:00 to 18:00 — kept.
            g("gcal-series:wake-wed", "Alarm", &["wed"], t(9, 0), t(9, 0)),
            // Thu: 17:15 to 18:00 — exactly 45, not shorter, kept.
            g("gcal-series:wake-thu", "Alarm", &["thu"], t(17, 15), t(17, 15)),
        ]);
        let got = run(&file, &Commitments::default());
        assert_eq!(
            window_of(&got).unwrap().meets,
            vec![
                meet(&["wed"], t(9, 0), t(18, 0)),
                meet(&["thu"], t(17, 15), t(18, 0))
            ]
        );
        // Every day left out: no window at all.
        let only_bad = file_of(vec![g(
            "gcal-series:bed-mon",
            "Bedtime",
            &["mon"],
            t(7, 0),
            t(7, 15),
        )]);
        assert!(run(&only_bad, &Commitments::default()).is_empty());
    }

    fn routines() -> Vec<Series> {
        vec![
            g("gcal-series:wake", "Wake up", &WEEKDAYS, t(7, 0), t(7, 30)),
            g("gcal-series:bed", "Bedtime", &ALL, t(22, 0), t(22, 15)),
        ]
    }

    #[test]
    fn a_planning_day_note_suppresses_the_window() {
        let v = scratch("pd");
        std::fs::write(
            v.join(FOLDER).join("planning-day.md"),
            "---\nid: cmt_0000000001\ntype: commitment\nkind: planning-day\nstatus: confirmed\n\
             window: [{days: [mon], start: \"09:00\", end: \"21:00\"}]\n---\n\nInvented.\n",
        )
        .unwrap();
        let set = load(&v);
        let _ = std::fs::remove_dir_all(&v);
        assert!(set.planning_day.is_some());
        let got = run(&file_of(routines()), &set);
        assert!(got.is_empty(), "no window, and routines are never commitments: {got:?}");
    }

    #[test]
    fn a_window_marker_suppresses_every_later_window_even_with_a_new_routine() {
        let mut set = Commitments::default();
        set.declined.insert("window".into());
        let mut series = routines();
        series.push(g("gcal-series:new-alarm", "Alarm", &["sat"], t(9, 0), t(9, 0)));
        series.push(cs100());
        let got = run(&file_of(series), &set);
        assert_eq!(keys(&got), vec!["gcal-series:cs100"]);
    }

    #[test]
    fn proposals_are_in_source_uid_order_and_deterministic() {
        let mut series = routines();
        series.push(mk(
            "ics-series:zeta",
            "personal",
            "Robotics Club",
            &["tue"],
            t(18, 0),
            t(19, 0),
        ));
        series.push(g("gcal-series:m", "Work", &["sat"], t(9, 0), t(13, 0)));
        series.push(g(
            "gcal-series:a",
            "CS 100",
            &["mon", "wed", "fri"],
            t(12, 0),
            t(12, 50),
        ));
        series.reverse();
        let file = SeriesFile {
            calendars: [
                ("google:abc".to_string(), TODAY),
                ("personal".to_string(), TODAY),
            ]
            .into_iter()
            .collect(),
            ended: BTreeMap::new(),
            series,
        };
        let first = run(&file, &Commitments::default());
        assert_eq!(
            keys(&first),
            vec![
                "gcal-series:a",
                "gcal-series:m",
                "ics-series:zeta",
                "window:gcal-series:bed,gcal-series:wake"
            ]
        );
        assert_eq!(first, run(&file, &Commitments::default()));
    }

    #[test]
    fn one_key_from_google_and_ics_is_one_proposal() {
        let mut google = cs100();
        google.where_ = Some("Room 101".into());
        let mut ics = cs100();
        ics.calendar = "personal".into();
        ics.where_ = Some("Room 9".into());
        let got = run(&file_of(vec![ics, google]), &Commitments::default());
        assert_eq!(keys(&got), vec!["gcal-series:cs100"]);
        assert_eq!(got[0].where_.as_deref(), Some("Room 101"), "the google: record wins");
    }

    /// One real series under two keys: an Exchange invite keeps its own UID (P8 review).
    fn twins() -> Vec<Series> {
        vec![
            mk(
                "ics-series:040000008200E0@exchange.example",
                "work",
                "Team Standup",
                &WEEKDAYS,
                t(9, 0),
                t(9, 15),
            ),
            g("gcal-series:standup", "Team Standup", &WEEKDAYS, t(9, 0), t(9, 15)),
        ]
    }

    #[test]
    fn twins_under_two_keys_are_one_proposal_by_google_first_precedence() {
        let got = run(&file_of(twins()), &Commitments::default());
        assert_eq!(keys(&got), vec!["gcal-series:standup"]);
        // Precedence is the calendar's, not the key's spelling: of two ICS feeds, the lower
        // calendar key wins.
        let a = mk("ics-series:b-uid", "alpha", "Team Standup", &WEEKDAYS, t(9, 0), t(9, 15));
        let b = mk("ics-series:a-uid", "beta", "Team Standup", &WEEKDAYS, t(9, 0), t(9, 15));
        assert_eq!(
            keys(&run(&file_of(vec![a, b]), &Commitments::default())),
            vec!["ics-series:b-uid"]
        );
    }

    #[test]
    fn a_declined_or_confirmed_twin_suppresses_the_other() {
        for key in [
            "gcal-series:standup",
            "ics-series:040000008200E0@exchange.example",
        ] {
            let mut declined = Commitments::default();
            declined.declined.insert(key.into());
            assert!(run(&file_of(twins()), &declined).is_empty(), "declined {key}");

            // Keyed only: this note's own meets are elsewhere, so only the key ties it.
            let mut kept = Commitments::default();
            kept.confirmed.push(confirmed(
                "meeting",
                "Standup (old)",
                None,
                vec![meet(&["sat"], t(8, 0), t(9, 0))],
                Some(key),
            ));
            assert!(run(&file_of(twins()), &kept).is_empty(), "confirmed {key}");

            let held: BTreeSet<String> = [key.to_string()].into_iter().collect();
            assert!(
                run_with(&file_of(twins()), &Commitments::default(), &held, true).is_empty(),
                "held {key}"
            );
        }
    }

    #[test]
    fn a_key_held_by_a_change_card_is_not_proposed() {
        let file = file_of(vec![cs100(), club()]);
        let held: BTreeSet<String> = ["gcal-series:cs100".to_string()].into_iter().collect();
        assert_eq!(
            keys(&run_with(&file, &Commitments::default(), &held, true)),
            vec!["gcal-series:club"]
        );
    }

    #[test]
    fn a_series_that_ended_before_today_or_is_unclassified_is_not_proposed() {
        let mut ended = cs100();
        ended.until = Some(date(2026, 8, 31));
        ended.last_seen = Some(date(2026, 8, 1));
        let study = g("gcal-series:study", "Study for CS 100", &["sun"], t(14, 0), t(16, 0));
        let mut biweekly = club();
        biweekly.rule.interval = 2;
        assert!(run(&file_of(vec![ended, study, biweekly]), &Commitments::default()).is_empty());
    }

    #[test]
    fn a_signature_is_order_free_and_case_free_on_the_title() {
        let a = signature("club", None, "Robotics Club", &[meet(&["tue", "mon"], t(18, 0), t(19, 0))], &codes());
        let b = signature(
            "club",
            None,
            " robotics club ",
            &[meet(&["mon"], t(18, 0), t(19, 0)), meet(&["tue"], t(18, 0), t(19, 0))],
            &codes(),
        );
        assert_eq!(a, b);
        let mon = [meet(&["mon"], t(9, 0), t(10, 0))];
        let c = signature("class", Some("cs-100"), "Anything", &mon, &codes());
        let d = signature("class", Some("cs-100"), "Else", &mon, &codes());
        assert_eq!(c, d, "a course, when set, stands in for the title");
        let e = signature("class", Some("CS 100"), "Else", &mon, &codes());
        assert_eq!(c, e, "a course written as its code is the same course (M4)");
    }

    // Fix round 1 (minors).

    #[test]
    fn two_sections_with_one_title_and_different_times_are_two_proposals() {
        let morning = g("gcal-series:sec-a", "CS 100", &["mon", "wed", "fri"], t(9, 0), t(9, 50));
        let noon = g("gcal-series:sec-b", "CS 100", &["mon", "wed", "fri"], t(12, 0), t(12, 50));
        let got = run(&file_of(vec![morning, noon]), &Commitments::default());
        assert_eq!(keys(&got), vec!["gcal-series:sec-a", "gcal-series:sec-b"]);
    }

    #[test]
    fn a_sleep_entry_ending_at_midnight_is_bed_side_only() {
        let file = file_of(vec![g("gcal-series:sleep", "Sleep", &["mon", "tue"], t(23, 0), t(0, 0))]);
        let got = run(&file, &Commitments::default());
        // No 00:00 wake on Tue or Wed: Mon and Tue take the template's 08:00, Wed has no side.
        assert_eq!(window_of(&got).unwrap().meets, vec![meet(&["mon", "tue"], t(8, 0), t(23, 0))]);
    }

    /// One `Sleep` series from two shapes: `a` crossing midnight, `b` starting after it.
    fn sleep_of(a: (&[DayKey], Time, Time), b: (&[DayKey], Time, Time)) -> Series {
        let mut s = g("gcal-series:sleep", "Sleep", a.0, a.1, a.2);
        let more = g("gcal-series:sleep", "Sleep", b.0, b.1, b.2);
        s.instances.extend(more.instances);
        s.instances.sort_by_key(|i| i.date);
        s.meets = meets_of(&s.instances);
        s
    }

    #[test]
    fn a_saturday_sleep_after_midnight_sets_fridays_bed_to_23_59() {
        let series = sleep_of(
            (&["mon", "tue", "wed", "thu"], t(23, 0), t(7, 0)),
            (&["sat"], t(0, 30), t(9, 0)),
        );
        let got = run(&file_of(vec![series]), &Commitments::default());
        assert_eq!(
            window_of(&got).unwrap().meets,
            vec![
                meet(&["mon"], t(8, 0), t(23, 0)),
                meet(&["tue", "wed", "thu"], t(7, 0), t(23, 0)),
                meet(&["fri"], t(7, 0), t(23, 59)),
                meet(&["sat"], t(9, 0), t(18, 0)),
            ]
        );
    }

    #[test]
    fn a_plain_sleep_after_midnight_gives_bed_23_59_the_day_before_and_its_end_as_wake() {
        let file = file_of(vec![g("gcal-series:sleep", "Sleep", &["mon", "tue"], t(0, 30), t(9, 0))]);
        let got = run(&file, &Commitments::default());
        assert_eq!(
            window_of(&got).unwrap().meets,
            vec![
                meet(&["mon"], t(9, 0), t(23, 59)),
                meet(&["tue"], t(9, 0), t(18, 0)),
                meet(&["sun"], t(8, 0), t(23, 59)),
            ]
        );
    }

    #[test]
    fn a_bedtime_after_midnight_sets_the_day_befores_bed_only() {
        let file = file_of(vec![g("gcal-series:bed", "Bedtime", &["tue"], t(0, 30), t(0, 45))]);
        let got = run(&file, &Commitments::default());
        assert_eq!(window_of(&got).unwrap().meets, vec![meet(&["mon"], t(8, 0), t(23, 59))]);
    }

    fn scratch_note(name: &str, front: &str) -> Commitments {
        let v = scratch(name);
        std::fs::write(v.join(FOLDER).join("note.md"), format!("---\n{front}---\n\nInvented.\n")).unwrap();
        let set = load(&v);
        let _ = std::fs::remove_dir_all(&v);
        assert!(set.warnings.is_empty(), "{:?}", set.warnings);
        set
    }

    const MWF_NOON: &str = "meets: [{days: [mon, wed, fri], start: \"12:00\", end: \"12:50\"}]\n";

    #[test]
    fn a_hand_written_course_as_a_code_matches_the_series_slug() {
        let set = scratch_note(
            "code",
            &format!("type: commitment\nkind: class\ntitle: \"Intro to CS\"\ncourse: CS 100\n{MWF_NOON}status: confirmed\n"),
        );
        assert!(run(&file_of(vec![cs100()]), &set).is_empty());
        let slug = scratch_note(
            "slug",
            &format!("type: commitment\nkind: class\ntitle: \"Intro to CS\"\ncourse: cs-100\n{MWF_NOON}status: confirmed\n"),
        );
        assert!(run(&file_of(vec![cs100()]), &slug).is_empty());
    }

    #[test]
    fn a_class_note_with_no_course_falls_back_to_the_title() {
        let same = scratch_note(
            "title",
            &format!("type: commitment\nkind: class\ntitle: \"cs 100\"\n{MWF_NOON}status: confirmed\n"),
        );
        assert!(run(&file_of(vec![cs100()]), &same).is_empty());
        let other = scratch_note(
            "other",
            &format!("type: commitment\nkind: class\ntitle: \"Intro to CS\"\n{MWF_NOON}status: confirmed\n"),
        );
        assert_eq!(keys(&run(&file_of(vec![cs100()]), &other)), vec!["gcal-series:cs100"]);
    }
}

// ---------------------------------------------------------------------------------------------
// P10 — `conflicts` and `fit`, the overlap API for piece 2 (§7).
// ---------------------------------------------------------------------------------------------

/// `weekcal::WeekCalendar::with_instances`' shape (P3), also [`SeriesFile::instances_map`]'s: per
/// `source_uid`, the fresh-read horizon `[start, end)` a calendar was last read over, and that
/// source's actual instances inside it (R21).
pub type InstancesMap = BTreeMap<String, (Date, Date, Vec<(Date, Time, Time)>)>;

/// A confirmed commitment's busy `(start, end)` pairs on `date` — the same rule
/// `weekcal::WeekCalendar::active_spans` applies per span, read here per whole [`Commitment`]
/// directly, so `conflicts` and capacity can never disagree (§7): inside a fresh horizon that
/// names this commitment's `source_uid` and covers `date`, only its actual instances dated `date`
/// are busy (a date the horizon covers but the read returned no instance for is free, even though
/// the weekly rule would have named it busy); outside the horizon — or with no `source_uid` at
/// all — the weekly `meets` rule applies, on `date`'s weekday, within `from`/`until`.
fn commitment_busy_on(commitment: &Commitment, date: Date, instances: &InstancesMap) -> Vec<(Time, Time)> {
    let horizon = commitment
        .source_uid
        .as_deref()
        .and_then(|uid| instances.get(uid))
        .filter(|(start, end, _)| *start <= date && date < *end);
    if let Some((_, _, list)) = horizon {
        return list
            .iter()
            .filter(|(d, s, e)| *d == date && s < e)
            .map(|(_, s, e)| (*s, *e))
            .collect();
    }
    let today = day_key(date);
    let after_from = commitment.from.map_or(true, |f| f <= date);
    let before_until = commitment.until.map_or(true, |u| date <= u);
    if !after_from || !before_until {
        return Vec::new();
    }
    commitment
        .meets
        .iter()
        .filter(|meet| meet.days.iter().any(|d| *d == today))
        .map(|meet| (meet.start, meet.end))
        .collect()
}

/// §7: every confirmed commitment whose meeting overlaps `[start, end)` — half-open, local
/// wall-clock in the vault's timezone, honouring actual instances inside the fresh horizon the
/// same way `weekcal::WeekCalendar` does (P3 decision 3, [`commitment_busy_on`]). A `[start, end)`
/// crossing midnight is split at midnight, each side checked against its own date, so a query that
/// runs past midnight still catches the next day's commitment. Sorted by `(level: hard first,
/// meeting start, title, id)` — the meeting start is the earliest overlapping busy time found for
/// that commitment. Optional commitments are included (a caller asking "during office hours"
/// wants to see them); [`fit`] is what ignores them. Pure: no clock, no I/O — `today`/the span are
/// the caller's, and `instances` is [`SeriesFile::instances_map`]'s result, the caller's to fetch.
pub fn conflicts<'a>(
    set: &'a Commitments,
    instances: &InstancesMap,
    start: DateTime,
    end: DateTime,
) -> Vec<(&'a Commitment, Level)> {
    if end <= start {
        return Vec::new();
    }
    let mut earliest: BTreeMap<&'a str, (DateTime, &'a Commitment)> = BTreeMap::new();
    let mut date = start.date();
    let last = end.date();
    loop {
        let day_start = DateTime::from_parts(date, Time::midnight());
        let tomorrow = add_days(date, 1);
        let day_end = if tomorrow > date {
            DateTime::from_parts(tomorrow, Time::midnight())
        } else {
            end
        };
        let seg_start = start.max(day_start);
        let seg_end = end.min(day_end);
        if seg_start < seg_end {
            for commitment in &set.confirmed {
                for (b_start, b_end) in commitment_busy_on(commitment, date, instances) {
                    if b_end <= b_start {
                        continue;
                    }
                    let busy_start = DateTime::from_parts(date, b_start);
                    let busy_end = DateTime::from_parts(date, b_end);
                    if busy_start < seg_end && seg_start < busy_end {
                        earliest
                            .entry(commitment.id.as_str())
                            .and_modify(|slot| slot.0 = slot.0.min(busy_start))
                            .or_insert((busy_start, commitment));
                    }
                }
            }
        }
        if date >= last || tomorrow <= date {
            break;
        }
        date = tomorrow;
    }

    let mut out: Vec<(DateTime, &'a Commitment)> = earliest.into_values().collect();
    out.sort_by(|(a_start, a), (b_start, b)| {
        (a.level, *a_start, &a.title, &a.id).cmp(&(b.level, *b_start, &b.title, &b.id))
    });
    out.into_iter().map(|(_, c)| (c, c.level)).collect()
}

/// What a candidate meeting's [`conflicts`] amounts to, before it is filed (C8, §7).
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Fit {
    Clear,
    OverlapsSoft(Vec<String>),
    OverlapsHard(Vec<String>),
}

/// The rule piece 2 applies over [`conflicts`]' result, in the order [`conflicts`] already sorts
/// by: any hard conflict wins (`OverlapsHard`, named by title — never propose); else any soft
/// (`OverlapsSoft`, named by title — propose, the card says so); optional conflicts are ignored
/// (`Clear`).
pub fn fit(conflicts: &[(&Commitment, Level)]) -> Fit {
    let hard: Vec<String> =
        conflicts.iter().filter(|(_, level)| *level == Level::Hard).map(|(c, _)| c.title.clone()).collect();
    if !hard.is_empty() {
        return Fit::OverlapsHard(hard);
    }
    let soft: Vec<String> =
        conflicts.iter().filter(|(_, level)| *level == Level::Soft).map(|(c, _)| c.title.clone()).collect();
    if !soft.is_empty() {
        return Fit::OverlapsSoft(soft);
    }
    Fit::Clear
}

/// P10's tests: `conflicts` and `fit`, the overlap API (§7). Invented data only; every date below
/// falls on the weekday its `meets` entry names (2026-08-31 is a Monday, so 2026-09-01 is a
/// Tuesday and 2026-09-02 a Wednesday).
#[cfg(test)]
mod conflicts_tests {
    use super::*;

    fn t(h: i8, m: i8) -> Time {
        Time::new(h, m, 0, 0).unwrap()
    }

    fn dt(y: i16, mo: i8, d: i8, h: i8, mi: i8) -> DateTime {
        DateTime::constant(y, mo, d, h, mi, 0, 0)
    }

    /// A confirmed commitment with one weekly `meets` entry and no `source_uid` (the weekly rule
    /// applies everywhere no instances-map horizon overrides it) — every field the tests below do
    /// not vary is a fixed, invented default.
    fn commitment(id: &str, level: Level, title: &str, day: DayKey, start: Time, end: Time) -> Commitment {
        Commitment {
            id: id.into(),
            path: PathBuf::from(format!("commitments/{id}.md")),
            kind: match level {
                Level::Hard => "class".into(),
                Level::Soft => "club".into(),
                Level::Optional => "office-hours".into(),
            },
            level,
            title: title.into(),
            course: None,
            meets: vec![Meet { days: vec![day], start, end }],
            where_: None,
            from: None,
            until: None,
            source_uid: None,
        }
    }

    fn set(confirmed: Vec<Commitment>) -> Commitments {
        Commitments { confirmed, ..Commitments::default() }
    }

    #[test]
    fn a_hard_class_overlapping_by_one_minute_is_overlaps_hard() {
        let cs100 = commitment("cmt_a", Level::Hard, "CS 100", "tue", t(9, 0), t(10, 0));
        let set = set(vec![cs100]);
        let instances = InstancesMap::new();
        // 09:59-10:59 overlaps 09:00-10:00 by exactly one minute.
        let got = conflicts(&set, &instances, dt(2026, 9, 1, 9, 59), dt(2026, 9, 1, 10, 59));
        assert_eq!(got.len(), 1);
        assert_eq!(got[0].0.title, "CS 100");
        assert_eq!(got[0].1, Level::Hard);
        assert_eq!(fit(&got), Fit::OverlapsHard(vec!["CS 100".to_string()]));
    }

    #[test]
    fn touching_end_to_start_is_clear() {
        let cs100 = commitment("cmt_a", Level::Hard, "CS 100", "tue", t(9, 0), t(10, 0));
        let set = set(vec![cs100]);
        let instances = InstancesMap::new();
        // The query starts exactly when the class ends: half-open, so no overlap.
        let got = conflicts(&set, &instances, dt(2026, 9, 1, 10, 0), dt(2026, 9, 1, 11, 0));
        assert!(got.is_empty());
        assert_eq!(fit(&got), Fit::Clear);
    }

    #[test]
    fn a_soft_club_gives_overlaps_soft_with_its_title() {
        let club = commitment("cmt_b", Level::Soft, "Chess Club", "tue", t(17, 0), t(18, 0));
        let set = set(vec![club]);
        let instances = InstancesMap::new();
        let got = conflicts(&set, &instances, dt(2026, 9, 1, 17, 30), dt(2026, 9, 1, 18, 30));
        assert_eq!(got.len(), 1);
        assert_eq!(fit(&got), Fit::OverlapsSoft(vec!["Chess Club".to_string()]));
    }

    #[test]
    fn decline_markers_never_conflict() {
        let mut declined = set(Vec::new());
        declined.declined.insert("gcal-series:declined-club".into());
        let instances = InstancesMap::new();
        let got = conflicts(&declined, &instances, dt(2026, 9, 1, 0, 0), dt(2026, 9, 2, 0, 0));
        assert!(got.is_empty());
        assert_eq!(fit(&got), Fit::Clear);
    }

    #[test]
    fn a_span_outside_from_until_is_clear() {
        let mut club = commitment("cmt_c", Level::Soft, "Study Group", "tue", t(9, 0), t(10, 0));
        club.from = Some(jiff::civil::date(2026, 8, 25));
        club.until = Some(jiff::civil::date(2026, 9, 1));
        let confirmed = set(vec![club]);
        let instances = InstancesMap::new();
        // The following Tuesday, 2026-09-08, is past `until`.
        let got = conflicts(&confirmed, &instances, dt(2026, 9, 8, 9, 0), dt(2026, 9, 8, 10, 0));
        assert!(got.is_empty());
        assert_eq!(fit(&got), Fit::Clear);
    }

    #[test]
    fn a_cancelled_instance_inside_the_horizon_is_clear() {
        let mut class = commitment("cmt_d", Level::Hard, "CS 200", "tue", t(9, 0), t(10, 0));
        class.source_uid = Some("gcal-series:cs200".into());
        let confirmed = set(vec![class]);
        let mut instances = InstancesMap::new();
        // The horizon covers 2026-09-01 but the fresh read returned no instance for it: cancelled.
        instances.insert(
            "gcal-series:cs200".to_string(),
            (jiff::civil::date(2026, 8, 25), jiff::civil::date(2026, 9, 22), Vec::new()),
        );
        let got = conflicts(&confirmed, &instances, dt(2026, 9, 1, 9, 0), dt(2026, 9, 1, 10, 0));
        assert!(got.is_empty());
        assert_eq!(fit(&got), Fit::Clear);
    }

    #[test]
    fn a_midnight_crossing_span_meets_the_next_days_commitment() {
        // An early Wednesday lab, 00:00-01:00 — no wraparound of its own; the QUERY is what
        // crosses midnight (Tuesday 23:30 into Wednesday 00:30), and it must be split so
        // Wednesday's side is checked against Wednesday's commitment.
        let lab = commitment("cmt_e", Level::Hard, "CS 100 Lab", "wed", t(0, 0), t(1, 0));
        let confirmed = set(vec![lab]);
        let instances = InstancesMap::new();
        let got = conflicts(&confirmed, &instances, dt(2026, 9, 1, 23, 30), dt(2026, 9, 2, 0, 30));
        assert_eq!(got.len(), 1);
        assert_eq!(got[0].0.title, "CS 100 Lab");
        assert_eq!(fit(&got), Fit::OverlapsHard(vec!["CS 100 Lab".to_string()]));
    }

    #[test]
    fn optional_conflicts_are_returned_but_fit_is_clear() {
        let oh = commitment("cmt_f", Level::Optional, "Office Hours", "tue", t(9, 0), t(10, 0));
        let set = set(vec![oh]);
        let instances = InstancesMap::new();
        let got = conflicts(&set, &instances, dt(2026, 9, 1, 9, 0), dt(2026, 9, 1, 10, 0));
        assert_eq!(got.len(), 1);
        assert_eq!(got[0].1, Level::Optional);
        assert_eq!(fit(&got), Fit::Clear);
    }

    // --- P15 cross-check: `commitment_busy_on` (P10) must never drift from what
    // `weekcal::WeekCalendar::for_vault` (P15) subtracts, since both read the same vault. ---

    /// A scratch vault directory, removed on every exit path — a passing assertion, a failing
    /// one, or a panic (fix round 1, M4) — the same idiom `app/tests/account.rs`'s `Cleanup` uses
    /// for Credential Manager entries.
    struct ScratchVault(PathBuf);

    impl std::ops::Deref for ScratchVault {
        type Target = Path;
        fn deref(&self) -> &Path {
            &self.0
        }
    }

    impl Drop for ScratchVault {
        fn drop(&mut self) {
            let _ = std::fs::remove_dir_all(&self.0);
        }
    }

    fn scratch_p15(tag: &str) -> ScratchVault {
        let v = std::env::temp_dir().join(format!("knowlu-p15-crosscheck-{}-{tag}", std::process::id()));
        let _ = std::fs::remove_dir_all(&v);
        std::fs::create_dir_all(v.join(FOLDER)).unwrap();
        ScratchVault(v)
    }

    /// A vault with a confirmed hard class (a `source_uid` whose fresh horizon covers
    /// `[2026-08-24, 2026-09-21)`, one moved instance inside it, meeting both Monday and
    /// Wednesday — fix round 1, M2: exercises `active_spans`' `handled_sources` de-duplication,
    /// the most fragile path where the two readers could diverge, since it draws one shared
    /// `source_uid` from two different spans), a confirmed soft club (no `source_uid`, so it
    /// always falls back to its weekly `meets` — a span the horizon never touches), and a
    /// confirmed **optional** office-hours note (M2: without one, the test's own
    /// `level != Optional` filter never ran). All three are read by `load`/`read_series_file`
    /// exactly as `for_vault` reads them.
    #[test]
    fn busy_spans_agree_with_week_calendar_for_vault() {
        let vault = scratch_p15("agree");
        std::fs::write(
            vault.join(FOLDER).join("hard.md"),
            "---\ntype: commitment\nkind: class\ntitle: \"CS 100\"\nmeets: [{days: [mon, wed], start: \"12:00\", end: \"12:50\"}]\nsource_uid: \"gcal-series:cs100\"\nstatus: confirmed\n---\n\nInvented.\n",
        )
        .unwrap();
        std::fs::write(
            vault.join(FOLDER).join("soft.md"),
            "---\ntype: commitment\nkind: club\ntitle: \"Chess Club\"\nmeets: [{days: [mon], start: \"17:00\", end: \"18:00\"}]\nstatus: confirmed\n---\n\nInvented.\n",
        )
        .unwrap();
        std::fs::write(
            vault.join(FOLDER).join("optional.md"),
            "---\ntype: commitment\nkind: office-hours\ntitle: \"Office Hours\"\nmeets: [{days: [mon], start: \"09:00\", end: \"09:50\"}]\nstatus: confirmed\n---\n\nInvented.\n",
        )
        .unwrap();

        let read_date = jiff::civil::date(2026, 8, 24); // Monday, the horizon's first day
        let moved_date = jiff::civil::date(2026, 8, 31); // second Monday, inside the horizon
        let last_in_horizon = jiff::civil::date(2026, 9, 14); // last Monday before the horizon ends
        let horizon_end = jiff::civil::date(2026, 9, 21); // read + 28, EXCLUSIVE (P3 review)
        let past_horizon = jiff::civil::date(2026, 9, 28); // a Monday past the horizon end
        let past_horizon_wed = jiff::civil::date(2026, 9, 30); // a Wednesday past the horizon end
        let series = Series {
            source_uid: "gcal-series:cs100".into(),
            calendar: "google:abc".into(),
            title: "CS 100".into(),
            where_: None,
            event_type: None,
            rule: Rule { freq: "WEEKLY".into(), interval: 1, until: None, count: None },
            has_master: true,
            rdate: false,
            unsupported: false,
            instances: vec![Instance { date: moved_date, start: Some(t(13, 0)), end: Some(t(13, 50)) }],
            meets: vec![Meet { days: vec!["mon", "wed"], start: t(12, 0), end: t(12, 50) }],
            first: Some(read_date),
            until: None,
            last_seen: Some(read_date),
        };
        let mut file = SeriesFile::default();
        file.calendars.insert("google:abc".into(), read_date);
        file.series.push(series);
        write_series_file(&vault, &file_bytes(&file)).unwrap();

        let set = load(&vault);
        assert!(set.warnings.is_empty(), "{:?}", set.warnings);
        let (series_file, series_warnings) = read_series_file(&vault);
        assert!(series_warnings.is_empty(), "{series_warnings:?}");
        let instances = series_file.instances_map();
        let calendar = crate::weekcal::WeekCalendar::for_vault(&vault, Vec::new());

        // 2026-08-24 (Mon): inside the horizon, no instance that day (cancelled) — the class is
        //   absent, the weekly club and office hours (untouched by any horizon) still meet.
        // 2026-08-31 (Mon): inside the horizon, the moved instance applies instead of the weekly
        //   time.
        // 2026-09-14 (Mon): inside the horizon (its last Monday), no instance that day —
        //   cancelled, same as 08-24 (fix round 1, I1).
        // 2026-09-21 (Mon): the horizon's exclusive end — one day past it is already outside, so
        //   the class falls back to its weekly time (fix round 1, I1: pins the `<` boundary).
        // 2026-09-28 (Mon): well past the horizon — the class falls back to its weekly time.
        // 2026-09-30 (Wed): well past the horizon, and the class's *other* weekday span — the
        //   multi-day de-duplication's other branch (fix round 1, M2).
        let dates = [
            read_date,
            moved_date,
            last_in_horizon,
            horizon_end,
            past_horizon,
            past_horizon_wed,
        ];
        for date in dates {
            let mut from_commitments: Vec<(Time, Time)> = set
                .confirmed
                .iter()
                .filter(|c| c.level != Level::Optional)
                .flat_map(|c| commitment_busy_on(c, date, &instances))
                .collect();
            from_commitments.sort();

            let mut from_weekcal: Vec<(Time, Time)> = calendar
                .spans_on(date)
                .into_iter()
                .map(|(start, end, _)| (start.time(), end.time()))
                .collect();
            from_weekcal.sort();

            assert_eq!(
                from_commitments, from_weekcal,
                "{date}: commitment_busy_on and WeekCalendar::for_vault must never disagree"
            );
        }

        // Fix round 1, I1: pin the exclusive-horizon-end boundary with absolute values, not just
        // agreement between the two readers.
        assert_eq!(
            calendar.spans_on(last_in_horizon).into_iter().map(|(s, e, _)| (s.time(), e.time())).collect::<Vec<_>>(),
            vec![(t(17, 0), t(18, 0))],
            "09-14 is inside the horizon with no instance: the class is cancelled, only the club meets"
        );
        let mut horizon_end_spans: Vec<(Time, Time)> = calendar
            .spans_on(horizon_end)
            .into_iter()
            .map(|(s, e, _)| (s.time(), e.time()))
            .collect();
        horizon_end_spans.sort();
        assert_eq!(
            horizon_end_spans,
            vec![(t(12, 0), t(12, 50)), (t(17, 0), t(18, 0))],
            "09-21 is the horizon's exclusive end (day < end fails): the class is back to its weekly time"
        );

        // Fix round 1, M2: the optional office-hours note contributes no span, but does surface in
        // `conflicts` (which `fit`, not `conflicts` itself, is what ignores optional commitments).
        assert!(
            calendar.spans_on(read_date).iter().all(|(_, _, span)| span.title != "Office Hours"),
            "an optional note must contribute no span"
        );
        let day_conflicts = conflicts(
            &set,
            &instances,
            DateTime::from_parts(read_date, Time::midnight()),
            DateTime::from_parts(add_days(read_date, 1), Time::midnight()),
        );
        assert!(
            day_conflicts
                .iter()
                .any(|(c, level)| c.title == "Office Hours" && *level == Level::Optional),
            "an optional note must still be visible to conflicts: {day_conflicts:?}"
        );
    }
}

#[cfg(test)]
mod card_tests {
    //! P11: `file_card` and `emit_checks` (§5.2, §5.4, §5.5). Every title, room and key is
    //! invented.

    use super::*;
    use crate::journal::Journal;
    use crate::write::WriteContext;
    use jiff::civil::date;

    const TODAY: Date = Date::constant(2026, 9, 24);

    /// A scratch vault removed when the test ends, pass or fail (review m4).
    struct Scratch(PathBuf);

    impl std::ops::Deref for Scratch {
        type Target = PathBuf;
        fn deref(&self) -> &PathBuf {
            &self.0
        }
    }

    impl Drop for Scratch {
        fn drop(&mut self) {
            let _ = std::fs::remove_dir_all(&self.0);
        }
    }

    fn vault(name: &str) -> Scratch {
        let dir = std::env::temp_dir().join(format!("knowlu-card-{}-{name}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        Scratch(dir)
    }

    fn ctx() -> WriteContext {
        WriteContext::new("agent:rank", "cli")
    }

    fn t(h: i8, m: i8) -> Time {
        Time::new(h, m, 0, 0).unwrap()
    }

    fn meet(days: &[DayKey], start: Time, end: Time) -> Meet {
        Meet { days: days.to_vec(), start, end }
    }

    fn proposal(kind: &str, title: &str, uid: &str, meets: Vec<Meet>) -> Proposal {
        Proposal {
            kind: kind.to_string(),
            level: default_level(kind).unwrap_or(Level::Soft),
            title: title.to_string(),
            course: None,
            meets,
            where_: None,
            from: None,
            until: None,
            source_uid: uid.to_string(),
        }
    }

    fn cs100() -> Proposal {
        Proposal {
            course: Some("cs-100".into()),
            where_: Some("Room 101".into()),
            from: Some(date(2026, 8, 19)),
            until: Some(date(2026, 12, 4)),
            ..proposal(
                "class",
                "CS 100",
                "gcal-series:cs100",
                vec![meet(&["mon", "wed", "fri"], t(12, 0), t(12, 50))],
            )
        }
    }

    fn window(uid: &str) -> Proposal {
        Proposal {
            level: Level::Optional,
            ..proposal(
                PLANNING_DAY,
                WINDOW_TITLE,
                uid,
                vec![meet(&["mon", "tue", "wed", "thu", "fri"], t(8, 0), t(22, 0))],
            )
        }
    }

    fn emit(v: &Path, proposals: &[Proposal], changes: &[Change], budget: i64) -> (Vec<PathBuf>, usize, Vec<String>) {
        let mut journal = Journal::new(v);
        emit_checks(v, proposals, changes, TODAY, budget, &ctx(), &mut journal)
    }

    fn read(path: &Path) -> (Mapping, String, String) {
        let raw = std::fs::read_to_string(path).unwrap().replace("\r\n", "\n");
        let (meta, body) = split_frontmatter(&raw).unwrap();
        (meta, body, raw)
    }

    fn field(meta: &Mapping, key: &str) -> String {
        field_text(meta, key).unwrap_or_default()
    }

    fn titles(paths: &[PathBuf]) -> Vec<String> {
        paths.iter().map(|p| field(&read(p).0, "title")).collect()
    }

    /// The frontmatter's lines, `id:` left out (it is minted).
    fn front_lines(raw: &str) -> Vec<String> {
        raw.split("---\n").nth(1).unwrap().lines().filter(|l| !l.starts_with("id: ")).map(str::to_string).collect()
    }

    fn journal_text(v: &Path) -> String {
        let dir = v.join("state").join("journal");
        let Ok(entries) = std::fs::read_dir(&dir) else { return String::new() };
        entries.map(|e| std::fs::read_to_string(e.unwrap().path()).unwrap()).collect()
    }

    fn mapping(yaml: &str) -> Mapping {
        match serde_yaml_ng::from_str::<Value>(yaml).unwrap() {
            Value::Mapping(m) => m,
            other => panic!("not a mapping: {other:?}"),
        }
    }

    fn meets_change() -> Change {
        Change {
            target: "commitments/cs-100.md".into(),
            source_uid: "gcal-series:cs100".into(),
            title: "CS 100".into(),
            change: mapping("meets: [{days: [tue, thu], start: \"09:30\", end: \"10:45\"}]"),
            was: mapping("meets: [{days: [mon, wed, fri], start: \"12:00\", end: \"12:50\"}]"),
        }
    }

    fn until_change() -> Change {
        Change {
            change: mapping("until: \"2026-12-04\""),
            was: mapping("until: null"),
            ..meets_change()
        }
    }

    /// A card written by hand into `folder`, as an earlier run would have filed it.
    fn card(v: &Path, folder: &str, name: &str, front: &str) {
        let dir = v.join(folder);
        std::fs::create_dir_all(&dir).unwrap();
        std::fs::write(dir.join(name), format!("---\ntype: approval\nkind: commitment-check\n{front}---\n\nInvented.\n")).unwrap();
    }

    // --- file_card ------------------------------------------------------------------------

    #[test]
    fn file_card_refuses_a_kind_outside_local_card_kinds() {
        let v = vault("refuse");
        for kind in ["event-check", "amend"] {
            let mut journal = Journal::new(v.as_path());
            let filed = file_card(
                &v,
                kind,
                vec![("source_uid", Field::Scalar(Node::text("gcal-series:x")))],
                "Invented · a class?",
                "Body.\n",
                TODAY,
                &ctx(),
                &mut journal,
            );
            assert!(filed.is_err(), "{kind}: {filed:?}");
        }
        let approvals = v.join("approvals");
        assert!(!approvals.exists() || std::fs::read_dir(&approvals).unwrap().next().is_none());
        assert_eq!(journal_text(&v), "");
    }

    #[test]
    fn front_matter_refuses_a_value_that_would_span_two_lines() {
        // A bare LF is written as a quoted scalar over two lines, in block and in flow: refused.
        let title = "CS 100\nLecture";
        let scalar = front_matter(&[("title", Field::Scalar(Node::text(title)))]);
        assert!(scalar.is_err(), "{scalar:?}");
        let flow = front_matter(&[("commitment", Field::Flow(serde_json::json!({"title": title})))]);
        assert!(flow.is_err(), "{flow:?}");
        // Whatever the emitter does with the rest, nothing multi-line gets through.
        for title in ["CS 100\r\nLecture", "CS 100\rLecture", "CS 100\u{2028}Lecture", "CS 100\u{85}Lecture"] {
            for fields in [
                vec![("title", Field::Scalar(Node::text(title)))],
                vec![("commitment", Field::Flow(serde_json::json!({"title": title})))],
            ] {
                if let Ok(text) = front_matter(&fields) {
                    assert_eq!(pystr::splitlines(&text).len(), 1, "{title:?}: {text:?}");
                    let (meta, _) = split_frontmatter(&format!("---\n{text}---\n")).unwrap();
                    assert_eq!(meta.len(), 1, "{text:?}");
                }
            }
        }
        let collection = front_matter(&[("meets", Field::Scalar(Node::Seq(vec![Node::text("mon")])))]);
        assert!(collection.is_err());
        assert_eq!(single_line("CS 100\r\nLecture\u{2028}A"), "CS 100 Lecture A");
        assert_eq!(single_line("CS  100"), "CS  100", "nothing but line breaks changes");
    }

    #[test]
    fn a_calendar_title_with_a_line_break_is_filed_on_one_line() {
        let v = vault("newline");
        let p = proposal("club", "Chess\nClub", "gcal-series:chess", vec![meet(&["wed"], t(18, 0), t(19, 0))]);
        let (paths, n, warnings) = emit(&v, &[p], &[], 15);
        assert_eq!((n, warnings.len()), (1, 0), "{warnings:?}");
        let (meta, _, raw) = read(&paths[0]);
        assert_eq!(field(&meta, "title"), "Chess Club · Wed 6–7pm · a club?");
        let lines = front_lines(&raw);
        assert_eq!(lines.iter().filter(|l| l.starts_with("commitment: ")).count(), 1);
        let Some(Value::Mapping(c)) = get(&meta, "commitment") else { panic!("{raw}") };
        assert_eq!(field(c, "title"), "Chess Club");
    }

    // --- the proposal card ----------------------------------------------------------------

    #[test]
    fn a_class_proposal_files_one_card_with_the_exact_title_frontmatter_and_body() {
        let v = vault("class");
        let (paths, n, warnings) = emit(&v, &[cs100()], &[], 15);
        assert_eq!((n, paths.len()), (1, 1));
        assert!(warnings.is_empty(), "{warnings:?}");
        assert_eq!(
            paths[0],
            v.join("approvals").join("commitment-check-cs-100-mon-wed-fri-12-12-50pm-a-class.md")
        );
        let (meta, body, raw) = read(&paths[0]);
        assert!(!field(&meta, "id").is_empty());
        let commitment_line = "commitment: {course: cs-100, from: '2026-08-19', kind: class, level: hard, \
             meets: [{days: [mon, wed, fri], end: '12:50', start: '12:00'}], title: CS 100, \
             until: '2026-12-04', where: Room 101}";
        assert_eq!(
            front_lines(&raw),
            vec![
                "type: approval",
                "kind: commitment-check",
                "title: CS 100 · Mon/Wed/Fri 12–12:50pm · a class?",
                "status: pending",
                "source_uid: gcal-series:cs100",
                commitment_line,
                "proposed_at: 2026-09-24",
                "first_proposed_at: 2026-09-24",
                "expires: null",
                "snooze_until: null",
                "created_by: agent:commitments",
            ]
        );
        let expected: Value = serde_yaml_ng::from_str(
            "{kind: class, level: hard, title: \"CS 100\", course: cs-100, \
             meets: [{days: [mon, wed, fri], start: \"12:00\", end: \"12:50\"}], \
             where: \"Room 101\", from: \"2026-08-19\", until: \"2026-12-04\"}",
        )
        .unwrap();
        assert_eq!(get(&meta, "commitment"), Some(&expected));
        assert_eq!(
            body.trim_start_matches('\n'),
            "**Is this part of your week?** Knowlu found it repeating on your calendar.\n\n\
             Approve and Knowlu never plans anything over it.\n\n\
             Where: Room 101\n\n\
             Reject and it's ignored. Either way you won't be asked again.\n"
        );
        let journal = journal_text(&v);
        assert!(journal.contains("\"agent:commitments\""), "{journal}");
        assert!(journal.contains("\"create\""), "{journal}");
    }

    #[test]
    fn a_soft_proposal_says_it_may_overlap() {
        let v = vault("soft");
        let p = proposal("club", "Chess Club", "gcal-series:chess", vec![meet(&["wed"], t(18, 0), t(19, 0))]);
        let (paths, ..) = emit(&v, &[p], &[], 15);
        let (_, body, _) = read(&paths[0]);
        assert!(body.contains("Approve and Knowlu counts it as busy; an event suggestion may overlap it, and will say so."));
        assert!(!body.contains("Where:"));
    }

    #[test]
    fn day_lists_collapse_and_a_second_meeting_says_plus_one_more_time() {
        assert_eq!(days_label(&["mon", "wed", "fri"]), "Mon/Wed/Fri");
        assert_eq!(days_label(&["mon", "tue", "wed", "thu", "fri"]), "Mon–Fri");
        assert_eq!(days_label(&["tue", "thu"]), "Tue/Thu");
        assert_eq!(days_label(&["tue", "wed"]), "Tue/Wed");
        assert_eq!(days_label(&["fri", "mon", "tue", "wed"]), "Mon–Wed/Fri");
        assert_eq!(range_label(t(12, 0), t(12, 50)), "12–12:50pm");
        assert_eq!(range_label(t(13, 0), t(14, 45)), "1–2:45pm");
        assert_eq!(range_label(t(8, 0), t(22, 0)), "8am–10pm");
        let v = vault("plus-one");
        let lab = proposal(
            "lab",
            "CHEM 110 Lab",
            "gcal-series:chem",
            vec![meet(&["tue"], t(14, 0), t(16, 50)), meet(&["thu"], t(9, 30), t(10, 45))],
        );
        let (paths, ..) = emit(&v, &[lab], &[], 15);
        assert_eq!(titles(&paths), vec!["CHEM 110 Lab · Tue 2–4:50pm +1 more time · a lab?"]);
    }

    #[test]
    fn a_long_title_is_cut_to_forty_characters() {
        let v = vault("long");
        let long = "An Invented Very Long Meeting Title That Runs On And On";
        let p = proposal("meeting", long, "gcal-series:long", vec![meet(&["mon"], t(15, 0), t(16, 0))]);
        let (paths, ..) = emit(&v, &[p], &[], 15);
        let title = titles(&paths).remove(0);
        let quoted = title.split(" · ").next().unwrap();
        assert!(quoted.chars().count() <= 40, "{title}");
        assert!(title.ends_with(" · Mon 3–4pm · a meeting?"), "{title}");
    }

    #[test]
    fn the_window_card_title_reads_your_day() {
        let v = vault("window");
        let (paths, ..) = emit(&v, &[window("window:gcal-series:wake,gcal-series:bed")], &[], 15);
        let (meta, body, raw) = read(&paths[0]);
        assert_eq!(field(&meta, "title"), "Your day · Mon–Fri 8am–10pm · plan in this window?");
        assert_eq!(field(&meta, "source_uid"), "window:gcal-series:wake,gcal-series:bed");
        let lines = front_lines(&raw);
        let commitment: Vec<&String> = lines.iter().filter(|l| l.starts_with("commitment")).collect();
        assert_eq!(
            commitment,
            vec!["commitment: {kind: planning-day, title: Your day, window: [{days: [mon, tue, wed, thu, fri], end: '22:00', start: '08:00'}]}"]
        );
        let expected: Value = serde_yaml_ng::from_str(
            "{kind: planning-day, title: Your day, window: [{days: [mon, tue, wed, thu, fri], start: \"08:00\", end: \"22:00\"}]}",
        )
        .unwrap();
        assert_eq!(get(&meta, "commitment"), Some(&expected));
        assert_eq!(
            body.trim_start_matches('\n'),
            "**Is this part of your week?** Knowlu found it repeating on your calendar.\n\n\
             Approve to plan inside these hours. Reject to keep your usual hours.\n\n\
             Either way you won't be asked again.\n"
        );
    }

    // --- the change card ------------------------------------------------------------------

    #[test]
    fn a_change_card_carries_target_change_and_was_each_on_one_line() {
        let v = vault("change");
        let (paths, n, _) = emit(&v, &[], &[meets_change()], 15);
        assert_eq!(n, 1);
        let (meta, body, raw) = read(&paths[0]);
        assert_eq!(field(&meta, "title"), "CS 100 now meets Tue/Thu 9:30–10:45am · update?");
        assert_eq!(field(&meta, "source_uid"), "gcal-series:cs100");
        assert_eq!(field(&meta, "target"), "commitments/cs-100.md");
        let lines = front_lines(&raw);
        assert!(lines.contains(&"change: {meets: [{days: [tue, thu], end: '10:45', start: '09:30'}]}".to_string()), "{raw}");
        assert!(lines.contains(&"was: {meets: [{days: [mon, wed, fri], end: '12:50', start: '12:00'}]}".to_string()), "{raw}");
        assert_eq!(get(&meta, "change"), Some(&Value::Mapping(meets_change().change)));
        assert_eq!(get(&meta, "was"), Some(&Value::Mapping(meets_change().was)));
        assert!(!lines.iter().any(|l| l.starts_with("commitment")));
        assert_eq!(
            body.trim_start_matches('\n'),
            "**Has this changed?** Your calendar entry for this now looks different.\n\n\
             Meets: now Tue/Thu 9:30–10:45am, was Mon/Wed/Fri 12–12:50pm.\n\n\
             Approve and Knowlu updates this. Reject and it stays as it is. \
             You won't be asked about this change again.\n"
        );
        assert!(!body.contains("note"), "{body}");

        let v = vault("change-until");
        let (paths, ..) = emit(&v, &[], &[until_change()], 15);
        let (meta, body, raw) = read(&paths[0]);
        assert_eq!(field(&meta, "title"), "CS 100 ends Dec 4 · update?");
        let lines = front_lines(&raw);
        assert!(lines.contains(&"change: {until: '2026-12-04'}".to_string()), "{raw}");
        assert!(lines.contains(&"was: {until: null}".to_string()), "{raw}");
        assert_eq!(get(&meta, "was"), Some(&Value::Mapping(until_change().was)));
        assert!(body.contains("\n\nEnds: now Dec 4, was (not set).\n\n"), "{body}");
        assert!(!body.contains("until") && !body.contains("Until"), "{body}");

        let v = vault("change-labels");
        let labelled = Change {
            change: mapping("where: \"Room 2\"\nlevel: soft"),
            was: mapping("where: null\nlevel: hard"),
            ..meets_change()
        };
        let (paths, ..) = emit(&v, &[], &[labelled], 15);
        let (meta, body, _) = read(&paths[0]);
        assert_eq!(field(&meta, "title"), "CS 100 is now in Room 2 · update?");
        assert!(body.contains("\n\nWhere: now Room 2, was (not set).\n\n"), "{body}");
        assert!(body.contains("\n\nLevel: now soft, was hard.\n\n"), "{body}");
    }

    #[test]
    fn an_optional_proposal_says_knowlu_wont_plan_around_it() {
        let v = vault("optional");
        let p = Proposal {
            level: Level::Optional,
            ..proposal("club", "Chess Club", "gcal-series:chess", vec![meet(&["wed"], t(18, 0), t(19, 0))])
        };
        let (paths, ..) = emit(&v, &[p], &[], 15);
        let (_, body, _) = read(&paths[0]);
        assert!(body.contains("\n\nOptional: Knowlu won't plan around it.\n\n"), "{body}");
    }

    #[test]
    fn a_whitespace_or_line_break_in_a_title_is_the_same_signature() {
        let codes = Codes::default();
        let meets = vec![meet(&["wed"], t(18, 0), t(19, 0))];
        let note = signature("club", None, "Chess Club", &meets, &codes);
        for title in ["Chess\nClub", "Chess\r\nClub", "  chess   club ", "Chess\u{2028}Club"] {
            assert_eq!(signature("club", None, title, &meets, &codes), note, "{title:?}");
        }
        assert_ne!(signature("club", None, "Chess Clubs", &meets, &codes), note);
    }

    #[test]
    fn a_card_deleted_in_the_app_closes_the_question() {
        let v = vault("deleted");
        let (paths, ..) = emit(&v, &[cs100(), window("window:a")], &[until_change()], 15);
        assert_eq!(paths.len(), 3);
        for path in &paths {
            let rel = format!("approvals/{}", path.file_name().unwrap().to_string_lossy());
            let mut journal = Journal::new(v.as_path());
            crate::write::delete(&v, &rel, &WriteContext::new("quinn", "dashboard"), &mut journal).unwrap();
        }
        assert_eq!(std::fs::read_dir(v.join("archive")).unwrap().count(), 3);
        let mut journal = Journal::new(v.as_path());
        let tomorrow = TODAY.tomorrow().unwrap();
        let (again, ..) =
            emit_checks(&v, &[cs100(), window("window:a,b")], &[until_change()], tomorrow, 15, &ctx(), &mut journal);
        assert!(again.is_empty(), "{again:?}");
    }

    #[test]
    fn a_card_that_expired_unanswered_does_not_close_the_question() {
        let v = vault("expired");
        card(&v, "archive", "commitment-check-cs-100.md", "status: expired\nsource_uid: gcal-series:cs100\nfirst_proposed_at: 2026-09-01\n");
        card(&v, "archive", "commitment-check-your-day.md", "status: expired\nsource_uid: \"window:a\"\nfirst_proposed_at: 2026-09-01\n");
        card(
            &v,
            "archive",
            "commitment-check-cs-100-ends.md",
            "status: expired\nsource_uid: gcal-series:cs100\ntarget: commitments/cs-100.md\n\
             change: {until: '2026-12-04'}\nwas: {until: null}\nfirst_proposed_at: 2026-09-01\n",
        );
        let (paths, ..) = emit(&v, &[cs100(), window("window:a")], &[until_change()], 15);
        assert_eq!(paths.len(), 3);
    }

    #[test]
    fn a_filing_error_stops_the_run_with_one_warning() {
        let v = vault("io-error");
        std::fs::write(v.join("approvals"), "not a folder").unwrap();
        let (paths, n, warnings) = emit(&v, &clubs(4), &[until_change()], 15);
        assert_eq!((paths.len(), n), (0, 0));
        assert_eq!(warnings.len(), 1, "{warnings:?}");
    }

    // --- the cap and the order ------------------------------------------------------------

    fn clubs(n: usize) -> Vec<Proposal> {
        (0..n)
            .map(|i| proposal("club", &format!("Club {i}"), &format!("gcal-series:club{i}"), vec![meet(&["wed"], t(18, 0), t(19, 0))]))
            .collect()
    }

    #[test]
    fn at_most_five_a_day_counted_by_first_proposed_at() {
        let v = vault("five");
        let (paths, n, _) = emit(&v, &clubs(7), &[], 15);
        assert_eq!((paths.len(), n), (5, 5));
        let (again, ..) = emit(&v, &clubs(7), &[], 15);
        assert!(again.is_empty(), "the day's five are spent");

        // A card `defer_over_budget` moved to tomorrow still counts today.
        let v = vault("five-deferred");
        card(
            &v,
            "approvals",
            "commitment-check-old.md",
            "title: Old\nstatus: snoozed\nsource_uid: gcal-series:old\nproposed_at: 2026-09-25\n\
             first_proposed_at: 2026-09-24\nsnooze_until: 2026-09-25\n",
        );
        let (paths, ..) = emit(&v, &clubs(7), &[], 15);
        assert_eq!(paths.len(), 4);

        // One first proposed yesterday does not.
        let v = vault("five-yesterday");
        card(
            &v,
            "archive",
            "commitment-check-old.md",
            "title: Old\nstatus: executed\nsource_uid: gcal-series:old\nproposed_at: 2026-09-23\n\
             first_proposed_at: 2026-09-23\n",
        );
        let (paths, ..) = emit(&v, &clubs(7), &[], 15);
        assert_eq!(paths.len(), 5);
    }

    #[test]
    fn the_budget_caps_below_five() {
        let v = vault("budget");
        let (paths, n, _) = emit(&v, &clubs(7), &[], 2);
        assert_eq!((paths.len(), n), (2, 2));
        let v = vault("budget-zero");
        let (paths, ..) = emit(&v, &clubs(7), &[], 0);
        assert!(paths.is_empty());
        assert!(!v.join("approvals").exists());
        let v = vault("budget-negative");
        assert!(emit(&v, &clubs(7), &[], -3).0.is_empty());
    }

    #[test]
    fn order_is_changes_then_class_lab_work_window_club_meeting() {
        let v = vault("order");
        let mut ps = vec![
            proposal("meeting", "Team Sync", "gcal-series:m", vec![meet(&["mon"], t(9, 0), t(10, 0))]),
            proposal("club", "Chess Club", "gcal-series:c", vec![meet(&["mon"], t(9, 0), t(10, 0))]),
            window("window:gcal-series:wake"),
            proposal("work", "Library Shift", "gcal-series:w", vec![meet(&["mon"], t(9, 0), t(10, 0))]),
            proposal("lab", "CHEM 110 Lab", "gcal-series:l", vec![meet(&["mon"], t(9, 0), t(10, 0))]),
            proposal("class", "BIO 120", "gcal-series:b", vec![meet(&["tue"], t(9, 0), t(10, 0))]),
            proposal("class", "ART 130", "gcal-series:a", vec![meet(&["mon"], t(11, 0), t(12, 0))]),
            proposal("class", "MAT 140", "gcal-series:z", vec![meet(&["mon"], t(9, 0), t(10, 0))]),
            proposal("class", "HIS 150", "gcal-series:y", vec![meet(&["mon"], t(9, 0), t(10, 0))]),
        ];
        ps.reverse();
        let mut journal = Journal::new(v.as_path());
        let (paths, ..) = emit_checks(&v, &ps, &[meets_change()], TODAY, 100, &ctx(), &mut journal);
        // Five a day: the change, then four classes by (day, start), then source_uid.
        assert_eq!(
            titles(&paths),
            vec![
                "CS 100 now meets Tue/Thu 9:30–10:45am · update?",
                "HIS 150 · Mon 9–10am · a class?",
                "MAT 140 · Mon 9–10am · a class?",
                "ART 130 · Mon 11am–12pm · a class?",
                "BIO 120 · Tue 9–10am · a class?",
            ]
        );
        // The next day, the rest in kind order.
        let mut journal = Journal::new(v.as_path());
        let tomorrow = TODAY.tomorrow().unwrap();
        let (paths, ..) = emit_checks(&v, &ps, &[meets_change()], tomorrow, 100, &ctx(), &mut journal);
        assert_eq!(
            titles(&paths),
            vec![
                "CHEM 110 Lab · Mon 9–10am · a lab?",
                "Library Shift · Mon 9–10am · work?",
                "Your day · Mon–Fri 8am–10pm · plan in this window?",
                "Chess Club · Mon 9–10am · a club?",
                "Team Sync · Mon 9–10am · a meeting?",
            ]
        );
    }

    #[test]
    fn office_hours_are_never_filed() {
        let v = vault("oh");
        let p = proposal("office-hours", "CS 100 Office Hours", "gcal-series:oh", vec![meet(&["mon"], t(9, 0), t(10, 0))]);
        assert!(emit(&v, &[p], &[], 15).0.is_empty());
    }

    // --- asked once -----------------------------------------------------------------------

    #[test]
    fn a_second_run_files_nothing() {
        let v = vault("second");
        let ps = vec![cs100(), window("window:gcal-series:wake")];
        let (first, ..) = emit(&v, &ps, &[meets_change()], 15);
        assert_eq!(first.len(), 3);
        let mut journal = Journal::new(v.as_path());
        let tomorrow = TODAY.tomorrow().unwrap();
        let (second, n, warnings) = emit_checks(&v, &ps, &[meets_change()], tomorrow, 15, &ctx(), &mut journal);
        assert_eq!((second.len(), n, warnings.len()), (0, 0, 0));
    }

    #[test]
    fn an_answered_or_archived_card_closes_the_question() {
        let v = vault("answered");
        card(&v, "archive", "commitment-check-cs-100.md", "status: rejected\nsource_uid: gcal-series:cs100\nfirst_proposed_at: 2026-09-01\n");
        assert!(emit(&v, &[cs100()], &[], 15).0.is_empty());
    }

    #[test]
    fn a_superseded_card_does_not_close_the_question() {
        let v = vault("superseded");
        let name = "commitment-check-cs-100-mon-wed-fri-12-12-50pm-a-class.md";
        card(&v, "archive", name, "status: superseded\nsource_uid: gcal-series:cs100\nfirst_proposed_at: 2026-09-01\n");
        let (paths, ..) = emit(&v, &[cs100()], &[], 15);
        assert_eq!(paths, vec![v.join("approvals").join(name)], "the base name: collisions are checked in approvals/ only");
    }

    #[test]
    fn a_name_taken_in_approvals_takes_dash_two() {
        let v = vault("dash-two");
        card(&v, "approvals", "commitment-check-cs-100-mon-wed-fri-12-12-50pm-a-class.md", "status: pending\nsource_uid: gcal-series:other\nfirst_proposed_at: 2026-09-01\n");
        let (paths, ..) = emit(&v, &[cs100()], &[], 15);
        assert_eq!(paths, vec![v.join("approvals").join("commitment-check-cs-100-mon-wed-fri-12-12-50pm-a-class-2.md")]);
    }

    #[test]
    fn one_key_from_google_and_ics_is_one_proposal_and_one_card() {
        let v = vault("twins");
        let (paths, n, warnings) = emit(&v, &[cs100(), cs100()], &[], 15);
        assert_eq!((paths.len(), n), (1, 1));
        assert!(warnings.is_empty(), "{warnings:?}");
        let (paths, ..) = emit(&v, &[window("window:a"), window("window:a,b")], &[], 15);
        assert_eq!(paths.len(), 1, "one window card per run, whatever its keys");
    }

    #[test]
    fn a_pending_window_card_blocks_a_window_proposal_with_new_routine_keys() {
        let v = vault("window-pending");
        card(&v, "approvals", "commitment-check-your-day.md", "status: pending\nsource_uid: \"window:gcal-series:wake\"\nfirst_proposed_at: 2026-09-01\n");
        let fresh = window("window:gcal-series:wake,gcal-series:bed");
        assert!(emit(&v, &[fresh.clone()], &[], 15).0.is_empty());
        // Answered and archived, it still blocks; withdrawn (superseded), it does not.
        let v = vault("window-archived");
        card(&v, "archive", "commitment-check-your-day.md", "status: rejected\nsource_uid: \"window:gcal-series:wake\"\nfirst_proposed_at: 2026-09-01\n");
        assert!(emit(&v, &[fresh.clone()], &[], 15).0.is_empty());
        let v = vault("window-superseded");
        card(&v, "archive", "commitment-check-your-day.md", "status: superseded\nsource_uid: \"window:gcal-series:wake\"\nfirst_proposed_at: 2026-09-01\n");
        assert_eq!(emit(&v, &[fresh], &[], 15).0.len(), 1);
    }

    #[test]
    fn the_same_change_is_never_asked_twice_and_a_different_one_is() {
        let v = vault("change-once");
        assert_eq!(emit(&v, &[], &[until_change()], 15).0.len(), 1);
        let tomorrow = TODAY.tomorrow().unwrap();
        let mut journal = Journal::new(v.as_path());
        let (again, ..) = emit_checks(&v, &[], &[until_change(), until_change()], tomorrow, 15, &ctx(), &mut journal);
        assert!(again.is_empty());
        let mut journal = Journal::new(v.as_path());
        let later = Change { change: mapping("until: \"2026-12-11\""), ..until_change() };
        let (different, ..) = emit_checks(&v, &[], &[later], tomorrow, 15, &ctx(), &mut journal);
        assert_eq!(titles(&different), vec!["CS 100 ends Dec 11 · update?"]);
        // The same change in one call is filed once.
        let v = vault("change-twice-in-a-call");
        assert_eq!(emit(&v, &[], &[meets_change(), meets_change()], 15).0.len(), 1);
    }

    #[test]
    fn a_superseded_change_card_does_not_close_its_change() {
        let v = vault("change-superseded");
        card(
            &v,
            "archive",
            "commitment-check-cs-100-ends-dec-4-update.md",
            "status: superseded\nsource_uid: gcal-series:cs100\ntarget: commitments/cs-100.md\n\
             change: {until: '2026-12-04'}\nwas: {until: null}\nfirst_proposed_at: 2026-09-01\n",
        );
        let reasked = Change { was: mapping("until: \"2026-12-18\""), ..until_change() };
        let (paths, ..) = emit(&v, &[], &[reasked], 15);
        assert_eq!(paths.len(), 1);
        let (meta, ..) = read(&paths[0]);
        assert_eq!(get(&meta, "was"), Some(&Value::Mapping(mapping("until: \"2026-12-18\""))));
        // Pending with the same change, it closes it.
        let v = vault("change-pending");
        card(
            &v,
            "approvals",
            "commitment-check-cs-100-ends-dec-4-update.md",
            "status: pending\nsource_uid: gcal-series:cs100\ntarget: commitments/cs-100.md\n\
             change: {until: '2026-12-04'}\nwas: {until: null}\nfirst_proposed_at: 2026-09-01\n",
        );
        assert!(emit(&v, &[], &[until_change()], 15).0.is_empty());
    }
}

#[cfg(test)]
mod change_tests {
    //! P12: `detect_changes` and `successor_keys` (§5.4, R22, plan review I1, I2, I5). Every
    //! title, room, key and calendar is invented.

    use super::*;
    use crate::journal::Journal;
    use crate::weekcal::WeekCalendar;
    use crate::write::WriteContext;
    use jiff::civil::date;
    use serde_json::json;

    /// A Monday.
    const TODAY: Date = Date::constant(2026, 10, 5);
    const GOOGLE: &str = "google:abc";
    const OLD: &str = "gcal-series:cs100";
    const NEW: &str = "gcal-series:cs100-new";

    /// A scratch vault removed when the test ends, its `config/ingest.yaml` naming `personal`
    /// (a direct ICS feed) and the Google grant.
    struct Scratch(PathBuf);

    impl std::ops::Deref for Scratch {
        type Target = PathBuf;
        fn deref(&self) -> &PathBuf {
            &self.0
        }
    }

    impl Drop for Scratch {
        fn drop(&mut self) {
            let _ = std::fs::remove_dir_all(&self.0);
        }
    }

    fn vault(name: &str) -> Scratch {
        let dir = std::env::temp_dir().join(format!("knowlu-p12-{}-{name}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(dir.join("config")).unwrap();
        std::fs::write(
            dir.join("config").join("ingest.yaml"),
            "timezone: America/Chicago\ncalendars:\n  - name: personal\n    ics_url: https://a.test/a.ics\n  \
             - name: google\n    ics_url: 'cloud:google'\n",
        )
        .unwrap();
        Scratch(dir)
    }

    fn t(h: i8, m: i8) -> Time {
        Time::new(h, m, 0, 0).unwrap()
    }

    fn meet(days: &[DayKey], s: Time, e: Time) -> Meet {
        Meet { days: days.to_vec(), start: s, end: e }
    }

    /// A weekly series on `days`, `s`–`e`, for `weeks` weeks from the Monday `monday`, last seen
    /// today; `meets` as P8 derives it, `first` its first instance.
    #[allow(clippy::too_many_arguments)]
    fn series(uid: &str, cal: &str, title: &str, days: &[DayKey], s: Time, e: Time, monday: Date, weeks: i64) -> Series {
        let mut instances = Vec::new();
        for week in 0..weeks {
            for day in days {
                let offset = DAY_KEYS.iter().position(|k| k == day).unwrap() as i64;
                instances.push(Instance { date: add_days(monday, week * 7 + offset), start: Some(s), end: Some(e) });
            }
        }
        instances.sort_by_key(|i| i.date);
        Series {
            source_uid: uid.to_string(),
            calendar: cal.to_string(),
            title: title.to_string(),
            where_: None,
            event_type: None,
            rule: Rule { freq: "WEEKLY".into(), interval: 1, until: None, count: None },
            has_master: true,
            rdate: false,
            unsupported: false,
            meets: meets_of(&instances),
            first: instances.first().map(|i| i.date),
            instances,
            until: None,
            last_seen: Some(TODAY),
        }
    }

    fn mwf() -> Vec<Meet> {
        vec![meet(&["mon", "wed", "fri"], t(12, 0), t(12, 50))]
    }

    /// The note's series as the calendar holds it now: CS 100, Mon/Wed/Fri 12–12:50pm in Room 101,
    /// until Dec 4.
    fn cs100(uid: &str, cal: &str) -> Series {
        let mut s = series(uid, cal, "CS 100", &["mon", "wed", "fri"], t(12, 0), t(12, 50), TODAY, 3);
        s.where_ = Some("Room 101".into());
        s.until = Some(date(2026, 12, 4));
        s
    }

    /// Tue/Thu 9:30–10:45am in Room 2 from the week of Oct 19 — a successor's new times.
    fn tue_thu(uid: &str, cal: &str) -> Series {
        let mut s = series(uid, cal, "CS 100", &["tue", "thu"], t(9, 30), t(10, 45), date(2026, 10, 19), 3);
        s.where_ = Some("Room 2".into());
        s.until = Some(date(2026, 12, 4));
        s
    }

    /// `cs100` split by "this and following": it now ends Oct 14.
    fn ending(uid: &str, cal: &str) -> Series {
        let mut s = series(uid, cal, "CS 100", &["mon", "wed", "fri"], t(12, 0), t(12, 50), TODAY, 2);
        s.instances.retain(|i| i.date <= date(2026, 10, 14));
        s.where_ = Some("Room 101".into());
        s.until = Some(date(2026, 10, 14));
        s
    }

    fn file_of(series: Vec<Series>) -> SeriesFile {
        let mut file = SeriesFile::default();
        for s in &series {
            file.calendars.insert(s.calendar.clone(), TODAY);
        }
        file.series = series;
        file.series.sort_by(|a, b| (&a.source_uid, &a.calendar).cmp(&(&b.source_uid, &b.calendar)));
        file
    }

    fn codes() -> Codes {
        Codes { table: [("CS100".to_string(), "cs-100".to_string())].into_iter().collect(), names: BTreeMap::new() }
    }

    /// The confirmed note `commitments/cs-100.md`.
    fn note(uid: &str) -> Commitment {
        Commitment {
            id: "cmt_0001".into(),
            path: PathBuf::from("commitments/cs-100.md"),
            kind: "class".into(),
            level: Level::Hard,
            title: "CS 100".into(),
            course: Some("cs-100".into()),
            meets: mwf(),
            where_: Some("Room 101".into()),
            from: Some(date(2026, 8, 19)),
            until: Some(date(2026, 12, 4)),
            source_uid: Some(uid.into()),
        }
    }

    fn set_of(notes: Vec<Commitment>) -> Commitments {
        Commitments { confirmed: notes, ..Commitments::default() }
    }

    fn fresh(cals: &[&str]) -> BTreeSet<String> {
        cals.iter().map(|c| c.to_string()).collect()
    }

    fn detect_on(file: &SeriesFile, set: &Commitments, cals: &[&str], today: Date) -> (Vec<Change>, Vec<String>) {
        detect_changes(file, set, &codes(), &[], &fresh(cals), today)
    }

    fn detect(file: &SeriesFile, set: &Commitments, cals: &[&str]) -> (Vec<Change>, Vec<String>) {
        detect_on(file, set, cals, TODAY)
    }

    /// A mapping as JSON, so key order does not matter to a comparison.
    fn js(map: &Mapping) -> serde_json::Value {
        crate::yaml::to_json(&Value::Mapping(map.clone()))
    }

    fn ctx() -> WriteContext {
        WriteContext::new("agent:rank", "cli")
    }

    fn emit(v: &Path, changes: &[Change], proposals: &[Proposal]) -> Vec<PathBuf> {
        let mut journal = Journal::new(v);
        emit_checks(v, proposals, changes, TODAY, 15, &ctx(), &mut journal).0
    }

    fn front(path: &Path) -> Mapping {
        let raw = std::fs::read_to_string(path).unwrap().replace("\r\n", "\n");
        split_frontmatter(&raw).unwrap().0
    }

    fn field(meta: &Mapping, key: &str) -> String {
        field_text(meta, key).unwrap_or_default()
    }

    fn assert_no_null(changes: &[Change]) {
        for c in changes {
            for (key, value) in &c.change {
                assert!(!matches!(value, Value::Null), "{key:?} proposed as null in {c:?}");
            }
        }
    }

    // --- changed ---------------------------------------------------------------------------

    #[test]
    fn a_meets_change_on_a_fresh_calendar_files_one_change_card() {
        // The note's series now meets Tue/Thu; its Outlook twin (another key, same series) too.
        let mut moved = cs100(OLD, GOOGLE);
        moved.meets = vec![meet(&["tue", "thu"], t(9, 30), t(10, 45))];
        let mut twin = moved.clone();
        twin.source_uid = "ics-series:cs100-twin".into();
        twin.calendar = "personal".into();
        let file = file_of(vec![moved, twin]);
        let set = set_of(vec![note(OLD)]);
        let (changes, warnings) = detect(&file, &set, &[GOOGLE, "personal"]);
        assert!(warnings.is_empty(), "{warnings:?}");
        assert_eq!(changes.len(), 1, "{changes:?}");
        let c = &changes[0];
        assert_eq!(c.target, "commitments/cs-100.md");
        assert_eq!(c.source_uid, OLD);
        assert_eq!(c.title, "CS 100");
        assert_eq!(js(&c.change), json!({"meets": [{"days": ["tue", "thu"], "start": "09:30", "end": "10:45"}]}));
        assert_eq!(js(&c.was), json!({"meets": [{"days": ["mon", "wed", "fri"], "start": "12:00", "end": "12:50"}]}));

        let v = vault("meets");
        let paths = emit(&v, &changes, &[]);
        assert_eq!(paths.len(), 1);
        let meta = front(&paths[0]);
        assert_eq!(field(&meta, "title"), "CS 100 now meets Tue/Thu 9:30\u{2013}10:45am · update?");
        assert_eq!(field(&meta, "target"), "commitments/cs-100.md");
        assert_eq!(field(&meta, "source_uid"), OLD);
        // Asked once: the next run detects the same change and files nothing.
        let (again, _) = detect(&file, &set, &[GOOGLE, "personal"]);
        assert!(emit(&v, &again, &[]).is_empty());
    }

    #[test]
    fn a_calendar_not_read_fresh_files_nothing() {
        let mut moved = cs100(OLD, GOOGLE);
        moved.meets = vec![meet(&["tue", "thu"], t(9, 30), t(10, 45))];
        moved.where_ = Some("Room 2".into());
        moved.until = Some(date(2026, 11, 20));
        let file = file_of(vec![moved]);
        let set = set_of(vec![note(OLD)]);
        assert!(detect(&file, &set, &[]).0.is_empty());
        assert!(detect(&file, &set, &["personal"]).0.is_empty());
        assert_eq!(detect(&file, &set, &[GOOGLE]).0.len(), 1);
    }

    #[test]
    fn a_non_empty_where_change_is_a_change_and_an_empty_one_is_not() {
        let set = set_of(vec![note(OLD)]);
        let mut moved = cs100(OLD, GOOGLE);
        moved.where_ = Some("Room 2".into());
        let (changes, _) = detect(&file_of(vec![moved]), &set, &[GOOGLE]);
        assert_eq!(changes.len(), 1);
        assert_eq!(js(&changes[0].change), json!({"where": "Room 2"}));
        assert_eq!(js(&changes[0].was), json!({"where": "Room 101"}));
        for empty in [None, Some(String::new()), Some("   ".to_string())] {
            let mut gone = cs100(OLD, GOOGLE);
            gone.where_ = empty;
            assert!(detect(&file_of(vec![gone]), &set, &[GOOGLE]).0.is_empty());
        }
        // The same room is no change.
        assert!(detect(&file_of(vec![cs100(OLD, GOOGLE)]), &set, &[GOOGLE]).0.is_empty());
        // Fix round 1, m4: a proposed `where` is stored trimmed, on the changed and succeeded paths.
        let mut padded = cs100(OLD, GOOGLE);
        padded.where_ = Some("  Room 2 \t".into());
        let (changes, _) = detect(&file_of(vec![padded]), &set, &[GOOGLE]);
        assert_eq!(js(&changes[0].change), json!({"where": "Room 2"}));
        let mut next = tue_thu(NEW, GOOGLE);
        next.where_ = Some(" Room 3 ".into());
        let (changes, _) = detect(&file_of(vec![ending(OLD, GOOGLE), next]), &set, &[GOOGLE]);
        assert_eq!(get(&changes[0].change, "where"), Some(&Value::String("Room 3".into())));
    }

    #[test]
    fn an_until_change_proposes_the_new_end_and_an_extension_too() {
        let set = set_of(vec![note(OLD)]);
        for until in [date(2026, 11, 20), date(2026, 12, 11)] {
            let mut s = cs100(OLD, GOOGLE);
            s.until = Some(until);
            let (changes, _) = detect(&file_of(vec![s]), &set, &[GOOGLE]);
            assert_eq!(changes.len(), 1);
            assert_eq!(js(&changes[0].change), json!({"until": until.to_string()}));
            assert_eq!(js(&changes[0].was), json!({"until": "2026-12-04"}));
        }
    }

    // --- ended -----------------------------------------------------------------------------

    #[test]
    fn an_ended_series_proposes_until_the_last_instance() {
        // Built by P8's `refresh_series` aging the series out (plan review I1), not by hand.
        let d0 = date(2026, 9, 21);
        let v = vault("ended");
        let mut a = series("ics-series:a", "personal", "CS 100", &["tue"], t(9, 0), t(9, 50), d0, 2);
        a.until = Some(date(2026, 12, 4));
        refresh_series(&v, &[("personal".into(), vec![a.clone()])], d0);
        let mut n = note("ics-series:a");
        n.meets = a.meets.clone();
        n.where_ = None;
        let set = set_of(vec![n]);

        // Unseen for a week: stale, not ended yet — nothing.
        let (file, _) = refresh_series(&v, &[("personal".into(), Vec::new())], add_days(d0, 7));
        assert!(detect_on(&file, &set, &["personal"], add_days(d0, 7)).0.is_empty());

        // Unseen for 14 days: dropped into `ended`, and the end card is proposed.
        let drop_day = add_days(d0, 14);
        let (file, _) = refresh_series(&v, &[("personal".into(), Vec::new())], drop_day);
        assert!(file.ended.contains_key("ics-series:a"));
        let (changes, warnings) = detect_on(&file, &set, &["personal"], drop_day);
        assert!(warnings.is_empty(), "{warnings:?}");
        assert_eq!(changes.len(), 1);
        assert_eq!(changes[0].source_uid, "ics-series:a");
        assert_eq!(js(&changes[0].change), json!({"until": "2026-09-29"}));
        assert_eq!(js(&changes[0].was), json!({"until": "2026-12-04"}));
        // A later run, the series still gone: the same question, which is never asked twice.
        let (later, _) = refresh_series(&v, &[("personal".into(), Vec::new())], add_days(drop_day, 3));
        assert_eq!(detect_on(&later, &set, &["personal"], add_days(drop_day, 3)).0, changes);
    }

    fn ended_file(last_instance: Option<Date>, until: Option<Date>) -> SeriesFile {
        let mut file = SeriesFile::default();
        file.calendars.insert("personal".into(), TODAY);
        file.ended.insert(
            "ics-series:a".into(),
            Ended { calendar: "personal".into(), dropped: TODAY, last_instance, until },
        );
        file
    }

    #[test]
    fn an_ended_entry_with_no_instance_uses_its_last_known_until() {
        let set = set_of(vec![note("ics-series:a")]);
        let (changes, warnings) = detect(&ended_file(None, Some(date(2026, 11, 20))), &set, &["personal"]);
        assert!(warnings.is_empty());
        assert_eq!(changes.len(), 1);
        assert_eq!(js(&changes[0].change), json!({"until": "2026-11-20"}));
        // A note with no `until`: `was` carries it as null.
        let mut open = note("ics-series:a");
        open.until = None;
        let (changes, _) = detect(&ended_file(Some(date(2026, 10, 20)), None), &set_of(vec![open]), &["personal"]);
        assert_eq!(js(&changes[0].change), json!({"until": "2026-10-20"}));
        assert_eq!(js(&changes[0].was), json!({"until": null}));
        // Neither: no change, and one warning.
        let (changes, warnings) = detect(&ended_file(None, None), &set, &["personal"]);
        assert!(changes.is_empty());
        assert_eq!(warnings.len(), 1, "{warnings:?}");
        assert!(warnings[0].contains("ics-series:a"), "{warnings:?}");
        // Warned once (fix round 1, m3): only on the run the entry was dropped into `ended`.
        let mut older = ended_file(None, None);
        older.ended.get_mut("ics-series:a").unwrap().dropped = add_days(TODAY, -1);
        let (changes, warnings) = detect(&older, &set, &["personal"]);
        assert!(changes.is_empty());
        assert!(warnings.is_empty(), "{warnings:?}");
    }

    #[test]
    fn a_truncated_until_that_is_not_a_meeting_day_is_asked_once_with_one_date() {
        // Fix round 1, I1: "this and following" truncates a Mon/Wed/Fri class to a Tuesday UNTIL.
        // The live card names the UNTIL; the student rejects it; when the series later ages out
        // through `refresh_series`, the ended rule names the same date, so nothing is re-asked.
        let v = vault("truncated");
        let mut s = series(OLD, GOOGLE, "CS 100", &["mon", "wed", "fri"], t(12, 0), t(12, 50), TODAY, 2);
        s.instances.retain(|i| i.date <= date(2026, 10, 13));
        s.where_ = Some("Room 101".into());
        s.until = Some(date(2026, 10, 13));
        let set = set_of(vec![note(OLD)]);
        let (file, _) = refresh_series(&v, &[(GOOGLE.into(), vec![s])], TODAY);
        let (changes, _) = detect(&file, &set, &[GOOGLE]);
        assert_eq!(changes.len(), 1);
        assert_eq!(js(&changes[0].change), json!({"until": "2026-10-13"}));
        let paths = emit(&v, &changes, &[]);
        assert_eq!(paths.len(), 1);
        // Rejected: archived as it stood, `status: rejected`.
        let raw = std::fs::read_to_string(&paths[0]).unwrap();
        std::fs::create_dir_all(v.join("archive")).unwrap();
        std::fs::write(v.join("archive").join(paths[0].file_name().unwrap()), raw.replace("status: pending", "status: rejected")).unwrap();
        std::fs::remove_file(&paths[0]).unwrap();

        let drop_day = add_days(TODAY, 14);
        let (file, _) = refresh_series(&v, &[(GOOGLE.into(), Vec::new())], drop_day);
        assert_eq!(file.ended[OLD].last_instance, Some(date(2026, 10, 12)));
        let (later, _) = detect_on(&file, &set, &[GOOGLE], drop_day);
        assert_eq!(later, changes);
        assert!(emit(&v, &later, &[]).is_empty());
    }

    #[test]
    fn an_open_ended_soft_commitment_that_stops_appearing_is_never_ended() {
        // Fix round 1, m1 (controller ruling): a club dormant over the summer is not lost. With no
        // UNTIL and no COUNT, a soft or optional note gets no end card; a hard one still does.
        let file = ended_file(Some(date(2026, 10, 20)), None);
        for (kind, level) in [("club", Level::Soft), ("meeting", Level::Soft), ("office-hours", Level::Optional)] {
            let mut n = note("ics-series:a");
            n.kind = kind.into();
            n.level = level;
            n.course = None;
            n.title = "Robotics Club".into();
            let (changes, warnings) = detect(&file, &set_of(vec![n.clone()]), &["personal"]);
            assert!(changes.is_empty(), "{kind}: {changes:?}");
            assert!(warnings.is_empty());
            // Nor with no instance either: no warning for a question that is never asked.
            let (changes, warnings) = detect(&ended_file(None, None), &set_of(vec![n]), &["personal"]);
            assert!(changes.is_empty() && warnings.is_empty(), "{kind}: {warnings:?}");
        }
        // With a known end, a soft note still gets one.
        let mut club = note("ics-series:a");
        club.kind = "club".into();
        club.level = Level::Soft;
        let (changes, _) = detect(&ended_file(Some(date(2026, 10, 20)), Some(date(2026, 11, 20))), &set_of(vec![club]), &["personal"]);
        assert_eq!(changes.len(), 1);
        // Hard kinds keep the rule.
        for kind in ["class", "lab", "work"] {
            let mut n = note("ics-series:a");
            n.kind = kind.into();
            n.level = Level::Hard;
            let (changes, _) = detect(&file, &set_of(vec![n]), &["personal"]);
            assert_eq!(changes.len(), 1, "{kind}");
            assert_eq!(js(&changes[0].change), json!({"until": "2026-10-20"}));
        }
    }

    #[test]
    fn an_ended_note_whose_until_already_ends_it_files_nothing() {
        let file = ended_file(Some(date(2026, 10, 20)), Some(date(2026, 12, 4)));
        for until in [date(2026, 10, 20), date(2026, 10, 16)] {
            let mut n = note("ics-series:a");
            n.until = Some(until);
            let (changes, warnings) = detect(&file, &set_of(vec![n]), &["personal"]);
            assert!(changes.is_empty(), "{changes:?}");
            assert!(warnings.is_empty());
        }
    }

    #[test]
    fn a_key_still_held_by_another_calendar_is_not_ended() {
        // Built by `refresh_series` (plan review I2): `personal` stops returning the key while
        // `google:aa` still holds it.
        let d0 = date(2026, 9, 21);
        let v = vault("held");
        let k = "gcal-series:twin";
        let on = |cal: &str| series(k, cal, "Robotics Club", &["tue"], t(18, 0), t(19, 0), d0, 2);
        refresh_series(&v, &[("personal".into(), vec![on("personal")]), ("google:aa".into(), vec![on("google:aa")])], d0);
        let day = add_days(d0, 14);
        let (file, _) =
            refresh_series(&v, &[("personal".into(), Vec::new()), ("google:aa".into(), vec![on("google:aa")])], day);
        let mut n = note(k);
        n.kind = "club".into();
        n.level = Level::Soft;
        n.title = "Robotics Club".into();
        n.course = None;
        n.meets = on("google:aa").meets;
        n.where_ = None;
        n.until = None;
        let set = set_of(vec![n]);
        assert!(detect_on(&file, &set, &["personal", "google:aa"], day).0.is_empty());
        // Even with a stray `ended` entry for it, a key some calendar holds is not ended.
        let mut stray = file.clone();
        stray.ended.insert(k.into(), Ended { calendar: "personal".into(), dropped: day, last_instance: Some(d0), until: None });
        assert!(detect_on(&stray, &set, &["personal", "google:aa"], day).0.is_empty());
    }

    // --- succeeded (R22) -------------------------------------------------------------------

    #[test]
    fn a_split_series_with_the_same_meets_files_nothing() {
        // "This and following" with the same times: the old key ends Oct 14, a new key carries
        // the note's signature from Oct 19. The note counts as seen.
        let mut same = series(NEW, GOOGLE, "CS 100", &["mon", "wed", "fri"], t(12, 0), t(12, 50), date(2026, 10, 19), 3);
        same.where_ = Some("Room 101".into());
        same.until = Some(date(2026, 12, 4));
        let set = set_of(vec![note(OLD)]);
        let file = file_of(vec![ending(OLD, GOOGLE), same.clone()]);
        let (changes, warnings) = detect(&file, &set, &[GOOGLE]);
        assert!(changes.is_empty(), "{changes:?}");
        assert!(warnings.is_empty());
        // Nor once the old key has aged into `ended`.
        let mut gone = file_of(vec![same]);
        gone.ended.insert(
            OLD.into(),
            Ended { calendar: GOOGLE.into(), dropped: TODAY, last_instance: Some(date(2026, 10, 14)), until: Some(date(2026, 10, 14)) },
        );
        assert!(detect(&gone, &set, &[GOOGLE]).0.is_empty());
        // And the new key is not proposed as a class of its own (P9's signature rule).
        let got = proposals(&gone, &set, &codes(), &[], &WeekCalendar::new(&Mapping::new(), Vec::new()), &BTreeSet::new(), TODAY, true);
        assert!(got.iter().all(|p| p.source_uid != NEW), "{got:?}");
    }

    /// A split to new times: the old key (and its Outlook twin) end Oct 14; the new key (and its
    /// twin) meet Tue/Thu from Oct 20.
    fn split_file() -> SeriesFile {
        file_of(vec![
            ending(OLD, GOOGLE),
            ending("ics-series:cs100-twin", "personal"),
            tue_thu(NEW, GOOGLE),
            tue_thu("ics-series:cs100-new-twin", "personal"),
        ])
    }

    #[test]
    fn a_split_with_new_meets_files_exactly_one_card_moving_source_uid() {
        let set = set_of(vec![note(OLD)]);
        let (changes, warnings) = detect(&split_file(), &set, &[GOOGLE, "personal"]);
        assert!(warnings.is_empty(), "{warnings:?}");
        assert_eq!(changes.len(), 1, "{changes:?}");
        let c = &changes[0];
        assert_eq!(c.source_uid, OLD);
        assert_eq!(
            js(&c.change),
            json!({"meets": [{"days": ["tue", "thu"], "start": "09:30", "end": "10:45"}], "where": "Room 2", "source_uid": NEW})
        );
        assert_eq!(
            js(&c.was),
            json!({"meets": [{"days": ["mon", "wed", "fri"], "start": "12:00", "end": "12:50"}], "where": "Room 101", "source_uid": OLD})
        );
        let v = vault("split");
        let paths = emit(&v, &changes, &[]);
        assert_eq!(paths.len(), 1);
        let meta = front(&paths[0]);
        assert_eq!(field(&meta, "title"), "CS 100 now meets Tue/Thu 9:30\u{2013}10:45am · update?");
        assert_eq!(field(&meta, "source_uid"), OLD);

        // Once the old key has aged into `ended`, the same change is detected — and never re-asked.
        let mut later = file_of(vec![tue_thu(NEW, GOOGLE), tue_thu("ics-series:cs100-new-twin", "personal")]);
        later.ended.insert(
            OLD.into(),
            Ended { calendar: GOOGLE.into(), dropped: TODAY, last_instance: Some(date(2026, 10, 14)), until: Some(date(2026, 10, 14)) },
        );
        let (again, _) = detect(&later, &set, &[GOOGLE, "personal"]);
        assert_eq!(again, changes);
        assert!(emit(&v, &again, &[]).is_empty());

        // A successor that starts before the old series' last instance is no successor: the old
        // one's end is asked on its own.
        let mut early = tue_thu(NEW, GOOGLE);
        early.first = Some(date(2026, 10, 13));
        let (changes, _) = detect(&file_of(vec![ending(OLD, GOOGLE), early]), &set, &[GOOGLE]);
        assert_eq!(changes.len(), 1);
        assert_eq!(js(&changes[0].change), json!({"until": "2026-10-14"}));
    }

    #[test]
    fn a_declined_successor_leaves_the_end_to_the_ended_rule() {
        // §5.4 M-d: a rejected successor's key carries a decline marker; the old note then ends.
        let mut set = set_of(vec![note(OLD)]);
        set.declined.insert(NEW.into());
        set.declined.insert("ics-series:cs100-new-twin".into());
        let (changes, _) = detect(&split_file(), &set, &[GOOGLE, "personal"]);
        assert_eq!(changes.len(), 1);
        assert_eq!(js(&changes[0].change), json!({"until": "2026-10-14"}));
    }

    // --- successor keys ----------------------------------------------------------------------

    fn card(v: &Path, folder: &str, name: &str, front: &str) {
        let dir = v.join(folder);
        std::fs::create_dir_all(&dir).unwrap();
        std::fs::write(dir.join(name), format!("---\ntype: approval\n{front}---\n\nInvented.\n")).unwrap();
    }

    #[test]
    fn successor_keys_skips_superseded_cards() {
        let v = vault("keys");
        let change = |key: &str| format!("change: {{meets: [{{days: [tue], start: '09:00', end: '10:00'}}], source_uid: '{key}'}}\n");
        card(&v, "approvals", "a.md", &format!("kind: commitment-check\nstatus: pending\ntarget: commitments/a.md\n{}", change("gcal-series:pending")));
        card(&v, "approvals", "b.md", &format!("kind: commitment-check\nstatus: snoozed\ntarget: commitments/b.md\n{}", change("gcal-series:snoozed")));
        card(&v, "archive", "c.md", &format!("kind: commitment-check\nstatus: rejected\ntarget: commitments/c.md\n{}", change("gcal-series:rejected")));
        card(&v, "archive", "d.md", &format!("kind: commitment-check\nstatus: executed\ntarget: commitments/d.md\n{}", change("gcal-series:approved")));
        card(&v, "archive", "e.md", &format!("kind: commitment-check\nstatus: superseded\ntarget: commitments/e.md\n{}", change("gcal-series:superseded")));
        // Fix round 1, m2: `expired` does not hold its key either (parity with P11's asked-once).
        card(&v, "archive", "e2.md", &format!("kind: commitment-check\nstatus: expired\ntarget: commitments/e2.md\n{}", change("gcal-series:expired")));
        // Not a successor: another kind, a change without a new key, a proposal card.
        card(&v, "approvals", "f.md", &format!("kind: amend\nstatus: pending\n{}", change("gcal-series:amend")));
        card(&v, "approvals", "g.md", "kind: commitment-check\nstatus: pending\ntarget: commitments/g.md\nchange: {until: '2026-12-04'}\n");
        card(&v, "approvals", "h.md", "kind: commitment-check\nstatus: pending\nsource_uid: gcal-series:proposal\n");
        let got: Vec<String> = successor_keys(&v).into_iter().collect();
        assert_eq!(got, ["gcal-series:approved", "gcal-series:pending", "gcal-series:rejected", "gcal-series:snoozed"]);
        assert!(successor_keys(&vault("keys-empty")).is_empty());
    }

    #[test]
    fn the_successor_is_not_proposed_while_its_change_card_exists() {
        let v = vault("held-successor");
        let file = split_file();
        let set = set_of(vec![note(OLD)]);
        let template = WeekCalendar::new(&Mapping::new(), Vec::new());
        let run = |held: &BTreeSet<String>| proposals(&file, &set, &codes(), &[], &template, held, TODAY, true);
        // With no card, the new times would be a class of their own.
        assert!(run(&BTreeSet::new()).iter().any(|p| p.source_uid == NEW));

        let (changes, _) = detect(&file, &set, &[GOOGLE, "personal"]);
        let paths = emit(&v, &changes, &[]);
        assert_eq!(paths.len(), 1);
        let held = successor_keys(&v);
        assert_eq!(held.iter().collect::<Vec<_>>(), [NEW]);
        let got = run(&held);
        assert!(got.iter().all(|p| p.source_uid != NEW && p.source_uid != "ics-series:cs100-new-twin"), "{got:?}");

        // Rejected and archived, it still holds the key; withdrawn as superseded, it does not.
        let raw = std::fs::read_to_string(&paths[0]).unwrap();
        let archived = v.join("archive").join(paths[0].file_name().unwrap());
        std::fs::create_dir_all(v.join("archive")).unwrap();
        std::fs::write(&archived, raw.replace("status: pending", "status: rejected")).unwrap();
        std::fs::remove_file(&paths[0]).unwrap();
        assert!(run(&successor_keys(&v)).iter().all(|p| p.source_uid != NEW));
        let raw = std::fs::read_to_string(&archived).unwrap();
        std::fs::write(&archived, raw.replace("status: rejected", "status: superseded")).unwrap();
        assert!(successor_keys(&v).is_empty());
        assert!(run(&successor_keys(&v)).iter().any(|p| p.source_uid == NEW));
    }

    // --- no null -------------------------------------------------------------------------------

    #[test]
    fn no_field_is_proposed_as_null() {
        let mut all: Vec<Change> = Vec::new();
        // The series lost its end date and its room: neither is proposed as null.
        let mut open = cs100(OLD, GOOGLE);
        open.until = None;
        open.where_ = None;
        open.meets = vec![meet(&["tue", "thu"], t(9, 30), t(10, 45))];
        let (changes, _) = detect(&file_of(vec![open]), &set_of(vec![note(OLD)]), &[GOOGLE]);
        assert_eq!(changes.len(), 1);
        assert_eq!(js(&changes[0].change), json!({"meets": [{"days": ["tue", "thu"], "start": "09:30", "end": "10:45"}]}));
        all.extend(changes);
        // A successor with no room: `where` is left out, not proposed as null.
        let mut roomless = tue_thu(NEW, GOOGLE);
        roomless.where_ = None;
        let (changes, _) = detect(&file_of(vec![ending(OLD, GOOGLE), roomless]), &set_of(vec![note(OLD)]), &[GOOGLE]);
        assert_eq!(changes.len(), 1);
        assert!(get(&changes[0].change, "where").is_none());
        assert!(get(&changes[0].was, "where").is_none());
        all.extend(changes);
        // A note with no room: `was` carries null, the change never does.
        let mut bare = note(OLD);
        bare.where_ = None;
        let mut moved = cs100(OLD, GOOGLE);
        moved.where_ = Some("Room 2".into());
        let (changes, _) = detect(&file_of(vec![moved]), &set_of(vec![bare]), &[GOOGLE]);
        assert_eq!(js(&changes[0].was), json!({"where": null}));
        all.extend(changes);
        assert_no_null(&all);
    }
}

#[cfg(test)]
mod moved_tests {
    use super::*;
    use crate::models::Task;
    use crate::weekcal::WeekCalendar;

    /// A Thursday.
    const DAY: Date = Date::constant(2026, 9, 24);

    fn t(h: i8, m: i8) -> Time {
        Time::constant(h, m, 0, 0)
    }

    fn task(slug: &str) -> Task {
        Task {
            slug: slug.into(),
            title: format!("Invented {slug}"),
            due: Some(DateTime::constant(2026, 11, 30, 23, 59, 0, 0)),
            effort_hours: 1.5,
            importance: 3,
            status: "todo".into(),
            progress: 0,
            course: None,
            domain: "school".into(),
            rank_override: None,
            slice_hours: Some(2.0),
            conflicts_with: None,
        }
    }

    /// Weekdays busy 08:00–17:30, a flat 08:00–18:00 day: only a later window leaves room.
    fn cal() -> WeekCalendar {
        let yaml = "day_start: '08:00'\nday_end: '18:00'\nclasses:\n  mon: [['08:00', '17:30']]\n  tue: [['08:00', '17:30']]\n  wed: [['08:00', '17:30']]\n  thu: [['08:00', '17:30']]\n  fri: [['08:00', '17:30']]\n";
        let Value::Mapping(m) = serde_yaml_ng::from_str::<Value>(yaml).unwrap() else { panic!() };
        WeekCalendar::new(&m, Vec::new())
    }

    #[test]
    fn moved_text_orders_largest_first_and_names_dropped() {
        let got = Moved::from_counts(MovedTo { morning: 1, afternoon: 0, evening: 2 }, 1).unwrap();
        assert_eq!(got.text, "2 items moved to this evening, 1 to this morning; 1 no longer fits today");
        // A tie keeps morning → evening order.
        let tie = Moved::from_counts(MovedTo { morning: 0, afternoon: 1, evening: 1 }, 0).unwrap();
        assert_eq!(tie.text, "1 item moved to this afternoon, 1 to this evening");
        let only_dropped = Moved::from_counts(MovedTo::default(), 2).unwrap();
        assert_eq!(only_dropped.text, "2 items no longer fit today");
        assert_eq!(Moved::from_counts(MovedTo::default(), 0), None);
        let json = crate::ledger::dumps_value(&serde_json::to_value(&got).unwrap());
        assert_eq!(
            json,
            "{\"dropped\": 1, \"text\": \"2 items moved to this evening, 1 to this morning; 1 no longer fits today\", \"to\": {\"afternoon\": 0, \"evening\": 2, \"morning\": 1}}"
        );
    }

    #[test]
    fn part_of_day_boundaries_are_12_and_17() {
        assert_eq!(part_of_day(t(0, 0)), Part::Morning);
        assert_eq!(part_of_day(t(11, 59)), Part::Morning);
        assert_eq!(part_of_day(t(12, 0)), Part::Afternoon);
        assert_eq!(part_of_day(t(16, 59)), Part::Afternoon);
        assert_eq!(part_of_day(t(17, 0)), Part::Evening);
        assert_eq!(part_of_day(t(23, 59)), Part::Evening);
    }

    #[test]
    fn identical_plans_move_nothing() {
        let ranked = vec![task("a"), task("b"), task("c")];
        let now = cal().with_day_window(DAY, t(8, 0), t(22, 0));
        assert_eq!(moved(&ranked, DAY, &now, &now.clone(), None), None);
        let base = cal();
        assert_eq!(moved(&ranked, DAY, &base, &base.clone(), None), None);
    }

    #[test]
    fn a_later_window_moves_takes_to_the_evening_and_an_earlier_one_drops_them() {
        let ranked = vec![task("a"), task("b"), task("c")];
        let base = cal();
        let wide = cal().with_day_window(DAY, t(8, 0), t(22, 0));
        let got = moved(&ranked, DAY, &wide, &base, None).unwrap();
        assert!(got.to.evening > 0, "{got:?}");
        assert_eq!((got.to.morning, got.to.afternoon, got.dropped), (0, 0, 0), "{got:?}");
        let back = moved(&ranked, DAY, &base, &wide, None).unwrap();
        assert_eq!(back.dropped, got.to.evening, "{back:?}");
        assert_eq!(back.to, MovedTo::default());
    }

    #[test]
    fn parse_window_accepts_the_note_shape_and_refuses_what_load_would_skip() {
        let w = parse_window("[{days: [mon, tue, wed, thu, fri], start: \"08:00\", end: \"22:00\"}, {days: [sat], start: \"10:00\", end: \"20:00\"}]").unwrap();
        assert_eq!(w[0], Some((t(8, 0), t(22, 0))));
        assert_eq!(w[5], Some((t(10, 0), t(20, 0))));
        assert_eq!(w[6], None);
        assert_eq!(parse_window("[]").unwrap(), [None; 7]);
        for bad in [
            "[{days: [mon], start: \"22:00\", end: \"08:00\"}]",
            "[{days: [mon], start: \"08:00\", end: \"24:00\"}]",
            "[{days: [funday], start: \"08:00\", end: \"18:00\"}]",
            "[{days: [mon], start: \"08:00\", end: \"18:00\"}, {days: [mon], start: \"09:00\", end: \"18:00\"}]",
            "{days: [mon], start: \"08:00\", end: \"18:00\"}",
            "[{days: [mon",
        ] {
            assert!(parse_window(bad).is_err(), "{bad} must be refused");
        }
    }
}
