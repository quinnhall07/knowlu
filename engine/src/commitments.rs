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

use jiff::civil::{Date, Time};
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
}
