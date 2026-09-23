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

use jiff::civil::{Date, Time};
use regex::Regex;
use serde_yaml_ng::{Mapping, Value};

use crate::models::split_frontmatter;
use crate::planning::DAY_KEYS;
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
#[cfg_attr(
    not(test),
    expect(
        dead_code,
        reason = "P11's cards and P13's notes are its first callers"
    )
)]
pub(crate) enum Field {
    /// A scalar (never a `Node::Seq`/`Node::Map`): written by `yamlemit::safe_dump_block`.
    Scalar(Node),
    /// A collection: written on one line by `write::to_literal`, the flow path `judgment:` uses.
    Flow(serde_json::Value),
}

/// The one builder for the frontmatter of every note and card this model writes: one line per
/// field, in the order given. `safe_dump_block` of a whole map would put a nested map or sequence
/// on several lines, and nothing downstream would stop it reaching disk.
#[cfg_attr(
    not(test),
    expect(
        dead_code,
        reason = "P11's cards and P13's notes are its first callers"
    )
)]
pub(crate) fn front_matter(fields: &[(&str, Field)]) -> String {
    let mut out = String::new();
    for (key, field) in fields {
        match field {
            Field::Scalar(node) => {
                debug_assert!(
                    !matches!(node, Node::Seq(_) | Node::Map(_)),
                    "{key}: a collection"
                );
                out.push_str(&yamlemit::safe_dump_block(&Node::map(vec![(
                    key,
                    node.clone(),
                )])));
            }
            Field::Flow(value) => {
                let literal = crate::write::to_literal(&crate::yaml::from_json(value));
                out.push_str(&format!("{key}: {literal}\n"));
            }
        }
    }
    out
}

// =============================================================================================
// The vault's code table (§3.4, C3, R7) and the classifier (§3.4; C5), P7.
// =============================================================================================

/// A string's leading course code — `CS 100`, `cs-100`, `CS100` — at the very start of the
/// string, then a word boundary. Used both to reduce a `courses/*.md` field to a code (§3.4's
/// code table) and to find the code a series title starts with (rule 3).
static LEADING_CODE_RE: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"^[A-Za-z]{2,8}[ ._-]?[0-9]{1,4}[A-Za-z]?\b").unwrap());

/// Rule 0's own-work-time words and phrases, whole-word (`\b`), case-insensitive.
static NOT_PROPOSED_RE: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"(?i)\b(study|homework|hw|review|prep|tutoring|work on|focus)\b").unwrap()
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

/// Rule 1's whole-title words, lower-cased.
const WAKE_WORDS: [&str; 4] = ["wake", "wake up", "get up", "alarm"];
const BED_WORDS: [&str; 5] = ["bed", "bedtime", "go to bed", "sleep", "lights out"];

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
/// reading anywhere in the string as a fallback for an LMS name.
fn to_code(text: &str) -> Option<String> {
    let text = text.trim();
    if text.is_empty() {
        return None;
    }
    if let Some(m) = LEADING_CODE_RE.find(text) {
        return Some(compact(m.as_str()));
    }
    d4_fallback(text).map(|s| compact(&s))
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
            for field in ["code", "title", "name"] {
                if let Some(raw) = field_text(&meta, field) {
                    if let Some(code) = to_code(&raw) {
                        table.claim(code, &slug);
                    }
                }
            }
            if let Some(code) = to_code(&slug) {
                table.claim(code, &slug);
            }
        }
    }

    if let Ok(text) = pystr::read_text(&vault.join("config").join("ingest.yaml")) {
        if let Ok(cfg) = serde_yaml_ng::from_str::<Value>(&text) {
            if let Some(mapping) = cfg.get("course_map").and_then(|v| v.as_mapping()) {
                for (k, v) in mapping.iter() {
                    if let (Some(key), Some(slug)) = (k.as_str(), v.as_str()) {
                        if let Some(code) = to_code(key) {
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

/// The title with surrounding whitespace and punctuation trimmed (§3.4: "matching is on the
/// title trimmed of surrounding punctuation").
fn trimmed_title(title: &str) -> String {
    title
        .trim_matches(|c: char| {
            c.is_whitespace() || c.is_ascii_punctuation() || matches!(c, '–' | '—' | '\u{2018}' | '\u{2019}' | '\u{201c}' | '\u{201d}')
        })
        .to_string()
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

/// The trimmed title is exactly `sleep` (case-insensitive) — rule 1's one midnight-crossing
/// exception.
fn is_sleep(title: &str) -> bool {
    trimmed_title(title).eq_ignore_ascii_case("sleep")
}

/// Any instance's end time is not after its start time — a nightly wraparound (§3.4's midnight
/// bullet; `Instance` carries no explicit "next day" flag, so a start/end pair on the *same* date
/// with `end <= start` is how the record says "crosses midnight").
fn crosses_midnight(series: &Series) -> bool {
    series
        .instances
        .iter()
        .any(|i| matches!((i.start, i.end), (Some(s), Some(e)) if e <= s))
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

/// §3.4's eligibility bullets, in order. `series.until`'s "not before today" reads `last_seen` as
/// today: normalisation sets `last_seen` to today for every series a fresh read returns (§3.2.6),
/// and `classify` takes no `today` of its own (decision 2's signature is exactly `series, codes,
/// planning`) — so the series' own record is where "today" comes from. A series with no
/// `last_seen` (never freshly read) skips the check rather than guess.
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
    if let (Some(until), Some(today)) = (series.until, series.last_seen) {
        if until < today {
            return false;
        }
    }
    true
}

/// The slug a title starts with, per the vault's code table, and everything after it (rule 3's
/// leading-code test; rule 2 reuses it just for the slug). `None` when the title does not start
/// with a known code, or the character right after it is not one of rule 3's separators
/// (space, `-`, `–`, `:`, `(`).
fn class_course(title: &str, codes: &BTreeMap<String, String>) -> Option<(String, String)> {
    let m = LEADING_CODE_RE.find(title)?;
    let slug = codes.get(&compact(m.as_str()))?.clone();
    let rest = &title[m.end()..];
    if rest.is_empty() {
        return Some((slug, String::new()));
    }
    let sep = rest.chars().next().unwrap();
    if !matches!(sep, ' ' | '-' | '–' | ':' | '(') {
        return None;
    }
    Some((slug, rest[sep.len_utf8()..].to_string()))
}

/// Rule 3's remainder test: empty, a section word with at most one trailing token (a section
/// number), or every word also in the course's own name/title. `None` means rule 3 does not
/// match at all ("CS 100 TA hours" falls through to rules 4–6).
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
    if SECTION_WORDS.contains(&tokens[0].as_str()) && tokens.len() <= 2 {
        return Some(if tokens[0] == "lab" || tokens[0] == "laboratory" {
            "lab"
        } else {
            "class"
        });
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
    let title = trimmed_title(&series.title);

    // Rule 0: not proposed at all.
    if NOT_PROPOSED_RE.is_match(&title)
        || planning
            .iter()
            .any(|name| name.trim().eq_ignore_ascii_case(&title))
    {
        return None;
    }

    // Rule 1: routine.
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
            front_matter(&fields),
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
        let front = front_matter(&fields);
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
}
