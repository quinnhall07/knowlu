//! Port of `engine/models.py` — frontmatter parsing and the `Task` model.
//!
//! Every note read in the entire engine passes through `split_frontmatter` and `coerce_datetime`,
//! so the traps here are the highest-leverage in the port.
//!
//! # Documented deviation, stated once
//!
//! Python writes `meta.get(k, default)`. When a key is *present but null*, that yields `None`, not
//! the default. For the numeric fields the consequence is visible — `float(None)` raises, the
//! exception is caught by `load_tasks`, and the note is skipped as unreadable — so that behaviour
//! is preserved exactly. For the string fields (`title`, `domain`) Python would carry a `None` into
//! a field typed `str`; this port uses the default instead. No note in the fixtures or the live
//! vault sets any of them to null, so the string path is unreachable. The numeric path, which is
//! the one with an observable consequence, is faithful.

use std::path::{Path, PathBuf};

use jiff::civil::{Date, DateTime, Time};
use serde_yaml_ng::{Mapping, Value};

use crate::yaml::{self, get as lookup, present};

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum NoteError {
    /// YAML would not parse. Python: `yaml.YAMLError`.
    Yaml(String),
    /// Frontmatter parsed to something that is not a mapping.
    ///
    /// Python returns the non-dict and the caller's `.get` raises `AttributeError`, which every
    /// call site catches in the same `except` tuple as a parse failure. Collapsing the two into
    /// one error is behaviourally identical at every call site — verified against `load_tasks`
    /// and `existing_by_uid`, the only two readers that distinguish readable from unreadable.
    NotAMapping,
    /// A field that Python coerces with `float()`/`int()` held something that would raise.
    BadField(String),
    /// Python: `ValueError(f"unparseable date: {value!r}")`.
    UnparseableDate(String),
    Io(String),
}

impl std::fmt::Display for NoteError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            NoteError::Yaml(m) => write!(f, "yaml: {m}"),
            NoteError::NotAMapping => write!(f, "frontmatter is not a mapping"),
            NoteError::BadField(m) => write!(f, "bad field: {m}"),
            NoteError::UnparseableDate(v) => write!(f, "unparseable date: {v}"),
            NoteError::Io(m) => write!(f, "io: {m}"),
        }
    }
}

/// Split a note into (frontmatter, body).
///
/// Three traps, all of which change output:
///
/// 1. **Bounded split.** Python uses `text.split("---", 2)` — at most three parts — so a body
///    containing `---` (a horizontal rule, common in these notes) stays intact. `splitn(3, _)` is
///    the equivalent; an unbounded split truncates the body at the first rule.
/// 2. **Absent frontmatter is not an error.** Returns an empty mapping and the text unchanged.
///    Callers depend on the empty-map path.
/// 3. **`lstrip("\n")` strips *all* leading newlines** from the body, not one.
pub fn split_frontmatter(text: &str) -> Result<(Mapping, String), NoteError> {
    if !text.starts_with("---") {
        return Ok((Mapping::new(), text.to_string()));
    }
    let parts: Vec<&str> = text.splitn(3, "---").collect();
    if parts.len() < 3 {
        return Ok((Mapping::new(), text.to_string()));
    }

    // PyYAML's safe_load("") is None, and `or {}` turns that into an empty dict.
    let meta = if parts[1].trim().is_empty() {
        Mapping::new()
    } else {
        match serde_yaml_ng::from_str::<Value>(parts[1]) {
            Ok(Value::Null) => Mapping::new(),
            Ok(Value::Mapping(m)) => m,
            Ok(_) => return Err(NoteError::NotAMapping),
            Err(e) => return Err(NoteError::Yaml(e.to_string())),
        }
    };
    Ok((meta, parts[2].trim_start_matches('\n').to_string()))
}

fn value_to_string(v: &Value) -> Option<String> {
    yaml::text(v)
}

/// Python `float(x)` — the failure becomes a `BadField`, which `load_tasks` catches and turns into
/// a skipped note, exactly as the `except (…TypeError…)` tuple does.
fn to_f64(v: &Value, field: &str) -> Result<f64, NoteError> {
    yaml::f64_of(v).ok_or_else(|| NoteError::BadField(format!("{field} is not a float")))
}

/// Python `int(x)`.
fn to_i64(v: &Value, field: &str) -> Result<i64, NoteError> {
    yaml::i64_of(v).ok_or_else(|| NoteError::BadField(format!("{field} is not an int")))
}

/// Coerce a frontmatter value to a naive datetime.
///
/// **A bare date becomes 23:59, never midnight.** That placeholder is what the whole due-date
/// design keys off (a source-published time is evidence; `23:59` is an assumption), and a midnight
/// default would silently move every dateless-time task a day earlier in every slack computation.
///
/// PyYAML resolves an unquoted `2026-08-26T23:59` to a `datetime` and an unquoted `2026-08-26` to
/// a `date`, so Python reaches this through its isinstance branches. serde_yaml_ng hands both over
/// as strings, and parsing them with the same format list converges on the same value. Verified
/// against the live vault, which holds only these shapes: 134 `YYYY-MM-DDTHH:MM`, 18 null, 4 bare
/// dates, 2 quoted datetimes. No timezone suffixes and no seconds anywhere.
pub fn coerce_datetime(value: Option<&Value>) -> Result<Option<DateTime>, NoteError> {
    let value = match value {
        None | Some(Value::Null) => return Ok(None),
        Some(v) => v,
    };
    let raw = value_to_string(value)
        .ok_or_else(|| NoteError::UnparseableDate(format!("{value:?}")))?;
    let text = raw.trim().replace(' ', "T");

    // Same order as Python's tuple; the bare-date branch is the one that rewrites the time.
    if let Some(dt) = parse_exact(&text, "%Y-%m-%dT%H:%M:%S") {
        return Ok(Some(dt));
    }
    if let Some(dt) = parse_exact(&text, "%Y-%m-%dT%H:%M") {
        return Ok(Some(dt));
    }
    if let Some(d) = parse_date(&text) {
        return Ok(Some(DateTime::from_parts(d, Time::constant(23, 59, 0, 0))));
    }
    Err(NoteError::UnparseableDate(raw))
}

fn parse_exact(text: &str, fmt: &str) -> Option<DateTime> {
    DateTime::strptime(fmt, text).ok()
}

fn parse_date(text: &str) -> Option<Date> {
    Date::strptime("%Y-%m-%d", text).ok()
}

#[derive(Debug, Clone, PartialEq)]
pub struct Task {
    pub slug: String,
    pub title: String,
    pub due: Option<DateTime>,
    pub effort_hours: f64,
    pub importance: i64,
    pub status: String,
    pub progress: i64,
    pub course: Option<String>,
    pub domain: String,
    pub rank_override: Option<i64>,
    pub slice_hours: Option<f64>,
    /// Written by the routine onto an obligation whose event collides with a class, work block, or
    /// commitment. Unread, it recorded that Quinn is expected in two places at once and never told
    /// him.
    pub conflicts_with: Option<String>,
}

impl Task {
    /// `effort_hours * (1 - progress/100)`.
    ///
    /// Keep the expression shape. Reordering it changes the last bits, and the renderer prints one
    /// decimal place — `3.1` at `progress: 10` must yield `2.79`, which is what the golden file's
    /// `2.8h left` rounds from.
    pub fn remaining_hours(&self) -> f64 {
        self.effort_hours * (1.0 - self.progress as f64 / 100.0)
    }

    pub fn from_file(path: &Path) -> Result<Task, NoteError> {
        Task::from_file_with_meta(path).map(|(task, _meta)| task)
    }

    /// `from_file`, plus the raw frontmatter mapping — the console's read model needs fields
    /// (`id`, `judgment`, …) that `Task` does not carry. Same read path as `from_file`: NOT
    /// `pystr::read_text` — Python's `load_tasks` never ran the notes through universal-newline
    /// translation, only the ledgers and journal do, so this stays raw `read_to_string`.
    pub fn from_file_with_meta(path: &Path) -> Result<(Task, Mapping), NoteError> {
        let text = std::fs::read_to_string(path).map_err(|e| NoteError::Io(e.to_string()))?;
        let (meta, _) = split_frontmatter(&text)?;
        let slug = path
            .file_stem()
            .map(|s| s.to_string_lossy().to_string())
            .unwrap_or_default();
        let task = Task::from_meta(&meta, slug)?;
        Ok((task, meta))
    }

    pub fn from_meta(meta: &Mapping, slug: String) -> Result<Task, NoteError> {
        // A present-but-null numeric raises in Python and skips the note; `present` returning
        // Some(None) reproduces that, while an absent key takes the default.
        let num = |key: &str, default: f64| -> Result<f64, NoteError> {
            match present(meta, key) {
                None => Ok(default),
                Some(None) => Err(NoteError::BadField(format!("{key} is null"))),
                Some(Some(v)) => to_f64(&v, key),
            }
        };
        let int = |key: &str, default: i64| -> Result<i64, NoteError> {
            match present(meta, key) {
                None => Ok(default),
                Some(None) => Err(NoteError::BadField(format!("{key} is null"))),
                Some(Some(v)) => to_i64(&v, key),
            }
        };
        let text = |key: &str| -> Option<String> {
            lookup(meta, key).and_then(value_to_string)
        };

        // `slice_hours`: a non-positive value is treated as UNSET. Python's comment explains why —
        // a zero slice makes `consume()` never accumulate hours, degenerating `start_by` to about a
        // year early. Note `if parsed_slice and parsed_slice > 0` also rejects 0.0 via truthiness.
        let slice_hours = match present(meta, "slice_hours") {
            None | Some(None) => None,
            Some(Some(v)) => {
                let parsed = to_f64(&v, "slice_hours")?;
                if parsed > 0.0 {
                    Some(parsed)
                } else {
                    None
                }
            }
        };

        Ok(Task {
            title: text("title").unwrap_or_else(|| slug.clone()),
            slug,
            due: coerce_datetime(lookup(meta, "due"))?,
            effort_hours: num("effort_hours", 1.0)?,
            importance: int("importance", 3)?,
            status: text("status").unwrap_or_else(|| "active".to_string()),
            progress: int("progress", 0)?,
            course: text("course"),
            domain: text("domain").unwrap_or_else(|| "school".to_string()),
            rank_override: match present(meta, "rank_override") {
                None | Some(None) => None,
                Some(Some(v)) => Some(to_i64(&v, "rank_override")?),
            },
            slice_hours,
            conflicts_with: text("conflicts_with"),
        })
    }
}

/// Every `*.md` in `folder`, in path order, read with [`Task::from_file_with_meta`] — one `Result`
/// per file so the two public callers below can each report failures their own way: `load_tasks`
/// prints `path.display()`, `load_task_notes` pushes just the file name and prints nothing.
fn scan_task_notes(folder: &Path) -> Vec<(PathBuf, Result<(Task, Mapping), NoteError>)> {
    let mut paths: Vec<PathBuf> = match std::fs::read_dir(folder) {
        Ok(entries) => entries
            .filter_map(|e| e.ok())
            .map(|e| e.path())
            .filter(|p| p.extension().map(|x| x == "md").unwrap_or(false))
            .collect(),
        Err(_) => return Vec::new(),
    };
    paths.sort();
    paths
        .into_iter()
        .map(|path| {
            let loaded = Task::from_file_with_meta(&path);
            (path, loaded)
        })
        .collect()
}

/// Load every `*.md` in `folder`, skipping unreadable notes, then keep only `status: active`.
///
/// Both halves matter: an unreadable note is *skipped with a printed line*, never fatal, and the
/// active filter runs **after** loading. The golden file's `5 active` is 6 task files minus
/// `done-already.md`, so this filter sits directly on the oracle path.
pub fn load_tasks(folder: &Path, skipped: Option<&mut Vec<String>>) -> Vec<Task> {
    let mut tasks = Vec::new();
    let mut skips = Vec::new();
    for (path, loaded) in scan_task_notes(folder) {
        match loaded {
            Ok((task, _meta)) => tasks.push(task),
            Err(_) => {
                println!("skipped (unreadable): {}", path.display());
                if let Some(name) = path.file_name() {
                    skips.push(name.to_string_lossy().to_string());
                }
            }
        }
    }
    if let Some(out) = skipped {
        out.extend(skips);
    }
    tasks.retain(|t| t.status == "active");
    tasks
}

/// Every readable, active task in `folder` with its raw frontmatter, in path order. Unreadable
/// notes are pushed to `skipped` by file name and NOTHING is printed — `knowlu-engine surface` writes
/// JSON to stdout, and the console's read model needs the frontmatter fields `Task` does not carry
/// (`id`, `created_by`, `effort_source`, `effort_confidence`, `importance_reason`, `source_uid`,
/// `judgment`). Same file set, same order, same skip names as `load_tasks` — verified in the tests
/// below by running both loaders over the same fixture and comparing.
pub fn load_task_notes(folder: &Path, skipped: Option<&mut Vec<String>>) -> Vec<(Task, Mapping)> {
    let mut notes = Vec::new();
    let mut skips = Vec::new();
    for (path, loaded) in scan_task_notes(folder) {
        match loaded {
            Ok((task, meta)) if task.status == "active" => notes.push((task, meta)),
            Ok(_) => {}
            Err(_) => {
                if let Some(name) = path.file_name() {
                    skips.push(name.to_string_lossy().to_string());
                }
            }
        }
    }
    if let Some(out) = skipped {
        out.extend(skips);
    }
    notes
}

/// Resolve the config-wide session cap onto tasks that don't override it.
pub fn apply_slice_default(tasks: &mut [Task], default: f64) {
    for task in tasks.iter_mut() {
        if task.slice_hours.is_none() {
            task.slice_hours = Some(default);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const FIXTURE_TASKS: &str = "tests/fixtures/vault-s1/tasks";

    #[test]
    fn no_frontmatter_returns_empty_map_and_original_text() {
        let (meta, body) = split_frontmatter("just a body").unwrap();
        assert!(meta.is_empty());
        assert_eq!(body, "just a body");
    }

    #[test]
    fn unterminated_frontmatter_is_not_an_error() {
        let (meta, body) = split_frontmatter("---\ntitle: x\n").unwrap();
        assert!(meta.is_empty());
        assert_eq!(body, "---\ntitle: x\n");
    }

    #[test]
    fn body_containing_a_horizontal_rule_survives_intact() {
        // The bounded split is the whole point: an unbounded split truncates here.
        let text = "---\ntitle: \"x\"\n---\n\nintro\n\n---\n\nafter the rule\n";
        let (meta, body) = split_frontmatter(text).unwrap();
        assert_eq!(meta.len(), 1);
        assert_eq!(body, "intro\n\n---\n\nafter the rule\n");
    }

    #[test]
    fn empty_frontmatter_block_is_an_empty_map() {
        let (meta, body) = split_frontmatter("---\n---\nbody\n").unwrap();
        assert!(meta.is_empty());
        assert_eq!(body, "body\n");
    }

    #[test]
    fn frontmatter_that_is_not_a_mapping_is_unreadable() {
        assert_eq!(
            split_frontmatter("---\njust a scalar\n---\nbody\n").unwrap_err(),
            NoteError::NotAMapping
        );
    }

    #[test]
    fn a_bare_date_becomes_2359_not_midnight() {
        let got = coerce_datetime(Some(&Value::String("2026-08-26".into()))).unwrap().unwrap();
        assert_eq!(got, DateTime::constant(2026, 8, 26, 23, 59, 0, 0));
    }

    #[test]
    fn datetime_shapes_round_trip() {
        let cases = [
            ("2026-08-26T23:59", DateTime::constant(2026, 8, 26, 23, 59, 0, 0)),
            ("2026-08-26T09:00:30", DateTime::constant(2026, 8, 26, 9, 0, 30, 0)),
            // Python replaces a space with T before parsing.
            ("2026-08-26 09:00", DateTime::constant(2026, 8, 26, 9, 0, 0, 0)),
        ];
        for (text, want) in cases {
            let got = coerce_datetime(Some(&Value::String(text.into()))).unwrap().unwrap();
            assert_eq!(got, want, "parsing {text}");
        }
    }

    #[test]
    fn absent_and_null_dates_are_none() {
        assert_eq!(coerce_datetime(None).unwrap(), None);
        assert_eq!(coerce_datetime(Some(&Value::Null)).unwrap(), None);
    }

    #[test]
    fn garbage_dates_raise() {
        assert!(coerce_datetime(Some(&Value::String("not a date".into()))).is_err());
    }

    #[test]
    fn remaining_hours_matches_the_golden_files_arithmetic() {
        // GN 103: effort_hours 3.1 at progress 10 -> 2.79, which renders as "2.8h left".
        let meta: Mapping = serde_yaml_ng::from_str("effort_hours: 3.1\nprogress: 10\n").unwrap();
        let task = Task::from_meta(&meta, "gn".into()).unwrap();
        assert!((task.remaining_hours() - 2.79).abs() < 1e-9);
    }

    #[test]
    fn non_positive_slice_hours_is_treated_as_unset() {
        for raw in ["slice_hours: 0", "slice_hours: 0.0", "slice_hours: -1"] {
            let meta: Mapping = serde_yaml_ng::from_str(raw).unwrap();
            let task = Task::from_meta(&meta, "t".into()).unwrap();
            assert_eq!(task.slice_hours, None, "for {raw}");
        }
        let meta: Mapping = serde_yaml_ng::from_str("slice_hours: 1.5").unwrap();
        assert_eq!(Task::from_meta(&meta, "t".into()).unwrap().slice_hours, Some(1.5));
    }

    #[test]
    fn defaults_apply_when_keys_are_absent() {
        let meta: Mapping = serde_yaml_ng::from_str("title: \"x\"").unwrap();
        let task = Task::from_meta(&meta, "slug".into()).unwrap();
        assert_eq!(task.effort_hours, 1.0);
        assert_eq!(task.importance, 3);
        assert_eq!(task.progress, 0);
        assert_eq!(task.status, "active");
        assert_eq!(task.domain, "school");
        assert_eq!(task.due, None);
    }

    #[test]
    fn title_falls_back_to_the_slug() {
        let meta = Mapping::new();
        assert_eq!(Task::from_meta(&meta, "my-slug".into()).unwrap().title, "my-slug");
    }

    #[test]
    fn a_null_numeric_makes_the_note_unreadable() {
        // Python: float(None) raises TypeError, load_tasks catches it, the note is skipped.
        let meta: Mapping = serde_yaml_ng::from_str("effort_hours: null").unwrap();
        assert!(Task::from_meta(&meta, "t".into()).is_err());
    }

    #[test]
    fn load_task_notes_returns_frontmatter_beside_each_active_task_and_prints_nothing() {
        let folder = Path::new("tests/fixtures/vault-full/tasks");
        let mut skipped = Vec::new();
        let notes = load_task_notes(folder, Some(&mut skipped));
        assert_eq!(notes.len(), 5, "done-already.md is filtered like load_tasks does");
        assert!(notes.iter().all(|(t, m)| m.get("title").is_some() && t.status == "active"));
        assert!(skipped.is_empty());
        // Same tasks, same order, as the loader the runner uses.
        let plain = load_tasks(folder, None);
        assert_eq!(notes.into_iter().map(|(t, _)| t).collect::<Vec<_>>(), plain);
    }

    #[test]
    fn load_task_notes_reports_an_unreadable_note_by_file_name() {
        let dir = std::env::temp_dir().join(format!("qo-ltn-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        crate::pystr::write_text(&dir.join("bad.md"), "---\ntitle: [unclosed\n---\n").unwrap();
        crate::pystr::write_text(&dir.join("ok.md"), "---\ntitle: Ok\nstatus: active\ndue: 2026-09-01\neffort_hours: 1.0\n---\n").unwrap();
        let mut skipped = Vec::new();
        let notes = load_task_notes(&dir, Some(&mut skipped));
        assert_eq!(notes.len(), 1);
        assert_eq!(skipped, vec!["bad.md".to_string()]);
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn fixture_vault_loads_five_active_tasks_from_six_files() {
        // This is the golden file's "5 active" and it is on the oracle path.
        let mut skipped = Vec::new();
        let tasks = load_tasks(Path::new(FIXTURE_TASKS), Some(&mut skipped));
        assert!(skipped.is_empty(), "no fixture note should be unreadable: {skipped:?}");
        assert_eq!(tasks.len(), 5, "got {:?}", tasks.iter().map(|t| &t.slug).collect::<Vec<_>>());
        assert!(!tasks.iter().any(|t| t.slug == "done-already"));
    }

    #[test]
    fn fixture_cs_100_parses_exactly() {
        let tasks = load_tasks(Path::new(FIXTURE_TASKS), None);
        let cs = tasks.iter().find(|t| t.slug == "cs-100-hw-01").expect("cs-100-hw-01");
        assert_eq!(cs.title, "CS 100 HW 01 Introduction to C");
        assert_eq!(cs.course.as_deref(), Some("cs-100"));
        assert_eq!(cs.due, Some(DateTime::constant(2026, 8, 26, 23, 59, 0, 0)));
        assert_eq!(cs.effort_hours, 2.5);
        assert_eq!(cs.importance, 2);
        assert_eq!(cs.progress, 0);
        assert_eq!(cs.remaining_hours(), 2.5);
    }

    #[test]
    fn apply_slice_default_only_fills_the_unset() {
        let meta: Mapping = serde_yaml_ng::from_str("slice_hours: 1.5").unwrap();
        let mut tasks = vec![
            Task::from_meta(&meta, "capped".into()).unwrap(),
            Task::from_meta(&Mapping::new(), "uncapped".into()).unwrap(),
        ];
        apply_slice_default(&mut tasks, 2.0);
        assert_eq!(tasks[0].slice_hours, Some(1.5));
        assert_eq!(tasks[1].slice_hours, Some(2.0));
    }
}
