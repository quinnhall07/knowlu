//! Port of `engine/approvals.py` — the approvals queue and its state machine.
//!
//! # Status: PARTIAL (wave 3, task 20 in progress)
//!
//! This file currently holds only the surface `render` needs, implemented for real. The state
//! machine (`process_approvals`, `defer_over_budget`, `count_proposals_created`,
//! `apply_amendment`, `_expand_digest`, and the validation stack) is task 20's job and lands
//! next. The signatures below are the contract between the two halves and must not change.

use std::collections::BTreeSet;
use std::path::{Path, PathBuf};
use std::sync::LazyLock;

use jiff::civil::{Date, DateTime};
use regex::Regex;
use serde_yaml_ng::{Mapping, Value};

use crate::commitments::{AskSettled, Codes, Commitments, Proposal, SeriesFile, Settled, COMMITMENT_ASK, COMMITMENT_CHECK};
use crate::eventledger::{record_answer, record_declined, VerdictError};
use crate::ids::resolve_lenient;
use crate::ingest::{apply_frontmatter_fields_to_text, slugify};
use crate::journal::Journal;
use crate::models::{coerce_datetime, split_frontmatter};
use crate::pystr;
use crate::write::{
    append_body, create, delete, single_line_problem, write_literals, WriteContext, WriteError,
    WriteOpts,
};

/// `WriteContext(actor="agent:approvals", via="cli")`.
///
/// Python defaults every entry point to this when the caller passes none. The Rust signatures take
/// `ctx` by reference instead — `WriteContext` owns two `String`s, so a module-level constant is
/// not available — and `cli` builds the real one with the run's `via` and `run_id` anyway.
pub fn default_ctx() -> WriteContext {
    WriteContext::new("agent:approvals", "cli")
}

/// Fields an `amend` proposal may touch. Anything else is refused by `validate_amendment`.
///
/// The last three arrived with S1: judged fields a routine may *re-propose* rather than re-set
/// (the judge-once rule).
pub const AMENDABLE_FIELDS: [&str; 9] = [
    "due",
    "effort_hours",
    "importance",
    "importance_reason",
    "status",
    "slice_hours",
    "course",
    "domain",
    "effort_confidence",
];

pub const AMENDABLE_FOLDERS: [&str; 2] = ["tasks", "courses"];

/// PyYAML's YAML 1.1 timestamp resolver, transcribed from `yaml/resolver.py`.
///
/// serde_yaml_ng follows YAML **1.2**, whose core schema has no timestamp tag, so it hands over
/// `2026-10-09T13:00:00-05:00` as a plain string where PyYAML hands Python a `datetime`. Every
/// place the Python special-cases a date or datetime therefore has to re-derive the distinction
/// here, from the text.
///
/// The two alternatives are PyYAML's own: a bare `YYYY-MM-DD`, or a full timestamp that **requires
/// seconds** (which is why `2026-10-09T13:00` stays a string in both engines).
///
/// The spaces are spelled `\x20` because this pattern is in `(?x)` mode, and Rust's verbose mode
/// strips whitespace **inside character classes too** — so a literal `[ \t]` compiles to `[\t]`,
/// and `2026-10-09 13:00:00` (the space-separated spelling PyYAML accepts) silently stops matching.
static PYYAML_TIMESTAMP: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(
        r"(?x)
        ^(?:
            (?P<d_year>[0-9]{4})-(?P<d_month>[0-9]{2})-(?P<d_day>[0-9]{2})
          |
            (?P<year>[0-9]{4})-(?P<month>[0-9]{1,2})-(?P<day>[0-9]{1,2})
            (?:[Tt]|[\x20\t]+)
            (?P<hour>[0-9]{1,2}):(?P<minute>[0-9]{2}):(?P<second>[0-9]{2})
            (?:\.[0-9]*)?
            (?:[\x20\t]*(?:Z|[-+][0-9]{1,2}(?::[0-9]{2})?))?
        )$",
    )
    .unwrap()
});

/// Render a YAML-loaded value the way a note spells it.
///
/// Dates and datetimes go through the ingest's canonical `format_due`, so a written value matches
/// its neighbours (`2026-10-02T13:00`, not `2026-10-02 13:00:00`) and a logged value carries no
/// Python repr.
///
/// **A timezone offset is dropped, not applied.** PyYAML attaches `tzinfo` and leaves the wall
/// clock alone, so `2026-10-09T13:00:00-05:00` formats as `13:00` — verified against the
/// interpreter, because shifting to UTC here would move every offset-bearing proposal's deadline
/// by hours. That is also the whole reason `as_note_datetime` exists: the vault cannot represent
/// an aware timestamp, and comparing one against a naive one raises `TypeError`, which escapes
/// every handler between `process_approvals` and `today.md`.
///
/// **Documented deviation.** serde_yaml_ng cannot report whether a scalar was quoted, so a
/// *quoted* `"2026-10-09T13:00:00"` — which PyYAML keeps as a string and passes through verbatim —
/// is normalised here to `2026-10-09T13:00`. Both spellings coerce to the same instant, so nothing
/// downstream compares differently; only a re-written literal would differ, and no such value
/// exists in the vault or the fixtures.
pub fn plain(value: &Value) -> String {
    let text = match crate::yaml::text(value) {
        Some(t) => t,
        None => return format!("{value:?}"),
    };
    match PYYAML_TIMESTAMP.captures(&text) {
        None => text,
        Some(caps) => {
            let num = |name: &str| -> i64 {
                caps.name(name).map(|m| m.as_str().parse().unwrap_or(0)).unwrap_or(0)
            };
            if caps.name("d_year").is_some() {
                // A bare date: `format_due` writes `%Y-%m-%d`, which is what it already reads as.
                text
            } else {
                format!(
                    "{:04}-{:02}-{:02}T{:02}:{:02}",
                    num("year"),
                    num("month"),
                    num("day"),
                    num("hour"),
                    num("minute"),
                )
            }
        }
    }
}

/// `plain`, flattened for interpolation into a single-line log message.
pub fn inline(value: &Value) -> String {
    plain(value).split_whitespace().collect::<Vec<_>>().join(" ")
}

/// Coerce a proposal's date value the way the note would actually spell it.
///
/// Python's docstring is the whole reason this exists rather than a bare `coerce_datetime`:
/// PyYAML makes `2026-10-09T13:00:00-05:00` aware and `2026-10-02T13:00:00` naive, and comparing
/// the two raises `TypeError` — **not** `ValueError` — which escapes every handler between here
/// and `cli.run`, leaving `today.md` silently stale. Rendering through `plain` first collapses
/// both to the vault's canonical minute-precision wall clock.
///
/// A value that will not parse returns `None`, never an error.
pub fn as_note_datetime(value: Option<&Value>) -> Option<DateTime> {
    let value = match value {
        None | Some(Value::Null) => return None,
        Some(v) => v,
    };
    let text = Value::String(plain(value));
    coerce_datetime(Some(&text)).ok().flatten()
}

/// Resolve a proposal's `target` inside the amendable folders, or `None`.
///
/// Shared by `validate_amendment` and the **renderer**: both read the target file, so both need
/// the same containment check. Without it a proposal naming `../outside.md` renders that file's
/// title into `today.md`.
///
/// Python tests `any(parent in target.parents for parent in allowed)`, which requires an allowed
/// folder to be a *strict* ancestor — `target == vault/tasks` is refused, and so is a bare
/// `tasks/`. `starts_with` alone would accept both, so the check below re-adds the strictness by
/// comparing against the target's parent chain.
pub fn resolve_amend_target(vault: &Path, raw_target: Option<&Value>) -> Option<PathBuf> {
    let raw = match raw_target {
        Some(Value::String(s)) if !s.is_empty() => s,
        _ => return None,
    };
    let root = resolve_lenient(vault);
    let target = resolve_lenient(&root.join(raw));
    for folder in AMENDABLE_FOLDERS {
        let allowed = resolve_lenient(&root.join(folder));
        if target.ancestors().skip(1).any(|p| p == allowed) {
            return Some(target);
        }
    }
    None
}

/// What `process_approvals` learned about one pending `amend` proposal.
///
/// Python builds a plain dict; the renderer reads `title`, `proposal`, `target`, `age_days`,
/// `changes` and `urgency` out of it. A struct is the same data with the key names checked at
/// compile time.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct AmendmentEntry {
    pub target: String,
    pub title: String,
    /// `increases` / `decreases` / `neutral`, re-derived by `derive_urgency` rather than trusted.
    pub urgency: String,
    /// The proposal note's stem — the renderer links `approvals/<proposal>.md`.
    pub proposal: String,
    pub changes: Mapping,
    pub age_days: i64,
}

/// The whole approvals pass, summarised for the run record and the page.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct ApprovalsResult {
    pub pending: i64,
    pub oldest_pending_days: i64,
    pub awaiting_calendar: i64,
    pub executed: Vec<String>,
    pub expired: Vec<String>,
    pub woken: Vec<String>,
    pub rejected: Vec<String>,
    pub warnings: Vec<String>,
    pub amendments_pending: Vec<AmendmentEntry>,
    pub events_in_digest: i64,
}

impl ApprovalsResult {
    pub fn acted(&self) -> bool {
        !self.executed.is_empty()
            || !self.expired.is_empty()
            || !self.woken.is_empty()
            || !self.rejected.is_empty()
    }
}

// ---------------------------------------------------------------------------------------------
// Python-shaped primitives
//
// `dict.get`, `str()` and truthiness come free in Python and cost a helper here. They live at the
// top because the whole module is written against them, and because two of them encode decisions
// that are observable: `python_str(None)` is `"None"` (not `""`), and an empty mapping is falsy
// (so `meta.get("changes") or {}` swallows `changes: {}`).
// ---------------------------------------------------------------------------------------------

static EMPTY_MAP: LazyLock<Mapping> = LazyLock::new(Mapping::new);
static EMPTY_SEQ: LazyLock<Vec<Value>> = LazyLock::new(Vec::new);

/// Python's `str(x)` on a YAML-loaded value, and Python's truthiness over one. Both moved to
/// `pystr` in wave 6, where `zybooks` and `coursework` also reach them; the semantics are
/// unchanged and the reasons they matter are documented at the definitions.
use crate::pystr::{yaml_str as python_str, yaml_truthy as is_truthy};

/// A mapping key as text. Keys are strings in every note that exists; the fallback keeps a
/// pathological key from panicking.
fn key_text(key: &Value) -> String {
    crate::yaml::text(key).unwrap_or_else(|| format!("{key:?}"))
}

/// `str(meta.get(key, "")).strip()`.
fn str_field(meta: &Mapping, key: &str) -> String {
    let raw = match crate::yaml::get(meta, key) {
        None => String::new(),
        Some(value) => python_str(value),
    };
    pystr::strip(&raw).to_string()
}

/// `str(map.get(key) or "").strip()` — the `or ""` collapses null, absent and empty alike.
fn truthy_str(map: &Mapping, key: &str) -> String {
    let raw = match crate::yaml::get(map, key) {
        Some(value) if is_truthy(value) => python_str(value),
        _ => String::new(),
    };
    pystr::strip(&raw).to_string()
}

fn spec_get<'a>(spec: &'a Value, key: &str) -> Option<&'a Value> {
    match spec {
        Value::Mapping(map) => crate::yaml::get(map, key),
        _ => None,
    }
}

/// `Path(path).relative_to(vault).as_posix()`.
///
/// The vault side has to be resolved as well: [`resolve_amend_target`] hands back a *canonical*
/// path (on Windows that means the `\\?\` verbatim prefix), which shares no prefix with the caller's
/// plain `vault`. Python's `relative_to` never sees the difference because `Path.resolve()` does not
/// add a prefix; here the second attempt is what stands in for that.
fn rel_path(vault: &Path, path: &Path) -> String {
    if let Ok(tail) = path.strip_prefix(vault) {
        return tail.to_string_lossy().replace('\\', "/");
    }
    let root = resolve_lenient(vault);
    if let Ok(tail) = path.strip_prefix(&root) {
        return tail.to_string_lossy().replace('\\', "/");
    }
    crate::ids::rel(vault, path)
}

fn stem_of(path: &Path) -> String {
    path.file_stem().map(|s| s.to_string_lossy().to_string()).unwrap_or_default()
}

fn name_of(path: &Path) -> String {
    path.file_name().map(|s| s.to_string_lossy().to_string()).unwrap_or_default()
}

/// `sorted(folder.glob("*.md"))`.
///
/// The sort is not decoration: it fixes the order proposals are settled in, which fixes the order
/// warnings appear in `state/runner-log.md` and the order archive collisions take their `-2`
/// suffix. `read_dir` order is unspecified, so it has to be re-imposed here.
pub(crate) fn sorted_md(folder: &Path) -> Vec<PathBuf> {
    let mut paths: Vec<PathBuf> = match std::fs::read_dir(folder) {
        Ok(entries) => entries
            .filter_map(|e| e.ok())
            .map(|e| e.path())
            .filter(|p| p.extension().map(|x| x == "md") == Some(true))
            .collect(),
        Err(_) => return Vec::new(),
    };
    paths.sort();
    paths
}

/// Python's `(to - from).days` for two dates.
fn days_between(from: Date, to: Date) -> i64 {
    to.since(from).map(|span| i64::from(span.get_days())).unwrap_or(0)
}

/// `_plain` on a value that may be absent, for interpolation into a log line.
///
/// `_inline(None)` is `"None"` in Python — the stale-amendment message says `due is None` when the
/// target has no `due` at all, and that string reaches `state/runner-log.md`.
fn inline_opt(value: Option<&Value>) -> String {
    match value {
        None | Some(Value::Null) => "None".to_string(),
        Some(value) => inline(value),
    }
}

// ---------------------------------------------------------------------------------------------
// Validation
// ---------------------------------------------------------------------------------------------

/// Check an amendment proposal without touching anything.
///
/// `Ok(target)` when the proposal is safe to apply, `Err(reason)` when it is not. Never writes —
/// except for the dry run inside [`write_problem`], which writes nothing but a temporary string.
///
/// Python returns `(Path | None, str | None)`; a `Result` is the same two states with the
/// impossible third one (both `None`) removed.
pub fn validate_amendment(vault: &Path, meta: &Mapping) -> Result<PathBuf, String> {
    let raw_target = crate::yaml::get(meta, "target");
    let raw = match raw_target {
        Some(Value::String(s)) if !s.is_empty() => s.clone(),
        _ => return Err("missing target".to_string()),
    };
    let target = match resolve_amend_target(vault, raw_target) {
        Some(target) => target,
        None => {
            return Err(format!("target outside {}: {}", AMENDABLE_FOLDERS.join("/"), raw));
        }
    };
    if !target.is_file() {
        return Err(format!("target does not exist: {raw}"));
    }

    let (changes, appends) = changes_and_appends(meta)?;
    if changes.is_empty() && appends.is_empty() {
        return Err("nothing to apply".to_string());
    }

    for (key, spec) in changes {
        let name = key_text(key);
        if !AMENDABLE_FIELDS.contains(&name.as_str()) {
            return Err(format!("field not amendable: {name}"));
        }
        if !matches!(spec, Value::Mapping(_)) {
            return Err(format!("{name}: expected from/to map"));
        }
        let from = spec_get(spec, "from").filter(|v| !matches!(v, Value::Null));
        let to = spec_get(spec, "to").filter(|v| !matches!(v, Value::Null));
        let (Some(from), Some(to)) = (from, to) else {
            return Err(format!("{name}: from/to must both be present and non-null"));
        };
        for (side, value) in [("from", from), ("to", to)] {
            if matches!(value, Value::Sequence(_) | Value::Mapping(_)) {
                // `str()` of a collection is a single line and legal YAML, so it slips past the
                // newline check and lands as `due: ['a', 'b']` — a note that no longer coerces,
                // i.e. the same silent drop from ranking by a different door.
                return Err(format!("{name}: {side} must be a scalar"));
            }
            if let Some(problem) = single_line_problem(&plain(value)) {
                // Refused before the writer ever sees it: a pre-write refusal is recoverable and
                // leaves the proposal fixable by hand, where a newline written into frontmatter is
                // not (it either splits the block or injects a key).
                return Err(format!("{name}: {side} {problem}"));
            }
        }
    }

    // `to` is the only side ever written, so it is the only side that has to survive being
    // written — and the only way to know is to write it.
    if let Some(problem) = write_problem(&target, changes) {
        return Err(problem);
    }
    Ok(target)
}

/// `changes = meta.get("changes") or {}` / `appends = meta.get("append") or []`, with the shared
/// type check both halves share a refusal string for.
fn changes_and_appends(meta: &Mapping) -> Result<(&Mapping, &Vec<Value>), String> {
    const WRONG: &str = "changes must be a map and append a list";
    let raw_changes = crate::yaml::get(meta, "changes").filter(|v| is_truthy(v));
    let raw_appends = crate::yaml::get(meta, "append").filter(|v| is_truthy(v));
    let changes: &Mapping = match raw_changes {
        None => &EMPTY_MAP,
        Some(Value::Mapping(map)) => map,
        Some(_) => return Err(WRONG.to_string()),
    };
    let appends: &Vec<Value> = match raw_appends {
        None => &EMPTY_SEQ,
        Some(Value::Sequence(seq)) => seq,
        Some(_) => return Err(WRONG.to_string()),
    };
    Ok((changes, appends))
}

/// A frontmatter value normalised so equivalent spellings compare equal.
///
/// The variants are Python's return types, which are heterogeneous on purpose: comparing a
/// `Float` against a `Text` is comparing `4.0 != "four"`, and both languages answer "different".
/// `Float` keeps `f64`'s `PartialEq`, so a NaN never equals itself — which is also what Python
/// does, and is why a proposal claiming `from: nan` always reads as stale.
#[derive(Debug, Clone, PartialEq)]
enum Comparable {
    Absent,
    Bool(bool),
    Stamp(DateTime),
    Float(f64),
    Int(i64),
    Text(String),
}

/// Normalise a frontmatter value so equivalent spellings compare equal.
///
/// Everything goes through [`plain`] first, so the comparison is against what the writer would
/// actually store. Without that the guard is finer-grained than the writer: `to: 2026-10-02T13:00:30`
/// gets refused for a corruption that cannot happen, and a genuine mismatch renders as
/// "due is X, expected X" because the message prints `plain` of both sides.
///
/// The bool arm is the one guard between a stale proposal and a clobbered hand edit: Python's
/// `bool` subclasses `int`, so `float(True) == 1.0` and an unguarded YAML `yes` satisfies the
/// staleness check against `effort_hours: 1.0`.
fn comparable(field: &str, value: Option<&Value>) -> Comparable {
    let value = match value {
        None | Some(Value::Null) => return Comparable::Absent,
        Some(value) => value,
    };
    if let Value::Bool(b) = value {
        return Comparable::Bool(*b);
    }
    let text = plain(value);
    let stripped = pystr::strip(&text);
    match field {
        "due" => match coerce_datetime(Some(&Value::String(text.clone()))) {
            Ok(Some(stamp)) => Comparable::Stamp(stamp),
            _ => Comparable::Text(stripped.to_string()),
        },
        "effort_hours" | "slice_hours" => match stripped.parse::<f64>() {
            Ok(number) => Comparable::Float(number),
            Err(_) => Comparable::Text(stripped.to_string()),
        },
        // Deliberately `int(text)`, not `int(value)`: `int(3.9)` truncates to 3 and would match a
        // note saying 3, where `int("3.9")` raises and falls through to a string compare that
        // correctly differs.
        "importance" => match stripped.parse::<i64>() {
            Ok(number) => Comparable::Int(number),
            Err(_) => Comparable::Text(stripped.to_string()),
        },
        _ => Comparable::Text(stripped.trim_matches('"').to_string()),
    }
}

const QUOTED_FIELDS: [&str; 2] = ["status", "importance_reason"];

/// `models.load_tasks` keeps only `status == "active"`; the design doc lists these three.
/// Anything else is not a status, it is a disappearance.
const TASK_STATUSES: [&str; 3] = ["active", "done", "archived"];

/// Escape backslashes and double quotes so `text` is safe inside a `"..."` YAML scalar.
fn quote_escape(text: &str) -> String {
    text.replace('\\', "\\\\").replace('"', "\\\"")
}

/// The YAML scalar to write for field `name`.
///
/// Free-text fields are quoted and escaped so a colon or a quote cannot corrupt the block; the
/// date and numeric fields are safe unquoted. [`validate_amendment`] has already refused newlines,
/// so quoting only has to survive backslashes and double quotes.
fn frontmatter_literal(name: &str, value: &Value) -> String {
    let text = plain(value);
    if QUOTED_FIELDS.contains(&name) {
        format!("\"{}\"", quote_escape(&text))
    } else {
        text
    }
}

/// Why the field's own reader could not use `value`, or `None`.
///
/// Storing a value faithfully is not the same as storing a usable one. `due: TBD @ 5pm` round-trips
/// exactly, and then `coerce_datetime` fails in `Task::from_file` and `load_tasks` drops the task —
/// visible as `skipped (unreadable)`, but the assignment is gone from Today either way.
/// `status: Active` is worse and fully silent: the note parses, the `Task` builds, and
/// `load_tasks`' `status == "active"` filter removes it with nothing skipped and the run still ok.
///
/// The value is judged **per field**: `TBD @ 5pm` is nonsense as a due date and perfectly
/// reasonable as an `importance_reason`.
fn unusable_reason(name: &str, value: Option<&Value>) -> Option<String> {
    if name == "importance_reason" {
        return None;
    }
    if let Some(Value::Bool(_)) = value {
        return Some("to must not be a yes/no value".to_string());
    }
    match name {
        "due" => {
            let readable = match value {
                None | Some(Value::Null) => false,
                Some(value) => matches!(
                    coerce_datetime(Some(&Value::String(plain(value)))),
                    Ok(Some(_))
                ),
            };
            if !readable {
                return Some("to is not a date the engine can read".to_string());
            }
        }
        "effort_hours" | "slice_hours" => {
            if value.and_then(crate::yaml::f64_of).is_none() {
                return Some("to is not a number".to_string());
            }
        }
        "importance" => {
            if value.and_then(crate::yaml::i64_of).is_none() {
                return Some("to is not a whole number".to_string());
            }
        }
        "status" => {
            let text = value.map(python_str).unwrap_or_else(|| "None".to_string());
            if !TASK_STATUSES.contains(&text.as_str()) {
                return Some(format!("to must be one of {}", TASK_STATUSES.join(", ")));
            }
        }
        _ => {}
    }
    None
}

/// Write one field into `text` for real, then read it back for real.
fn reread_problem(text: &str, name: &str, value: &Value) -> Option<String> {
    let literal = frontmatter_literal(name, value);
    let applied =
        match apply_frontmatter_fields_to_text(text, &[(name.to_string(), literal)], "note") {
            Ok(applied) => applied,
            // Python's `lines.index("---", 1)` raises ValueError, which is inside `_UNREADABLE`.
            Err(_) => return Some("to would corrupt frontmatter".to_string()),
        };
    let meta = match split_frontmatter(&applied) {
        Ok((meta, _)) => meta,
        Err(_) => return Some("to would corrupt frontmatter".to_string()),
    };
    let stored = crate::yaml::get(&meta, name);
    if comparable(name, stored) != comparable(name, Some(value)) {
        return Some("to would corrupt frontmatter".to_string());
    }
    // Judge the value as the note will actually hold it, not as proposed.
    unusable_reason(name, stored)
}

/// Dry-run the real write against the real note, or say why it is unsafe.
///
/// This replaces a simulation that parsed a synthetic two-key document. The simulation was wrong in
/// the way simulations always are: it modelled neither the real writer (line surgery that replaces
/// the FIRST matching `key:` line) nor the real reader (`split_frontmatter`, which once split on an
/// un-anchored `---` and truncated the block at an inline marker; it now closes only on a whole
/// `---` line). A duplicated key means YAML reads the last while the writer rewrites the first.
///
/// So: apply the actual changes to the actual text, re-read with the actual parser, and require
/// three things — every changed field reads back as intended, every untouched field is untouched,
/// and the body has not moved.
fn write_problem(target: &Path, changes: &Mapping) -> Option<String> {
    const MALFORMED: &str = "target note has malformed YAML frontmatter";
    let text = match pystr::read_text(target) {
        Ok(text) => text,
        Err(_) => return Some(MALFORMED.to_string()),
    };
    let (before_meta, before_body) = match split_frontmatter(&text) {
        Ok(parts) => parts,
        Err(_) => return Some(MALFORMED.to_string()),
    };

    // split('\n'), not `lines()`: the closing marker is a line *exactly* equal to `---`, and
    // `lines()` would silently accept `---\r` as well on a note read without translation.
    let lines: Vec<&str> = text.split('\n').collect();
    let Some(end) = lines.iter().skip(1).position(|line| *line == "---").map(|i| i + 1) else {
        // `update_frontmatter_fields` raises here — but it would raise *after* `append_body_line`
        // has already written, leaving the note half amended and the proposal stuck approved with a
        // retry that can never succeed. `split_frontmatter` tolerates the same note, so only an
        // explicit check catches it before anything is written.
        return Some("target note frontmatter has no closing delimiter".to_string());
    };

    for key in changes.keys() {
        let name = key_text(key);
        let prefix = format!("{name}:");
        if lines[1..end].iter().filter(|line| line.starts_with(&prefix)).count() > 1 {
            return Some(format!(
                "{name}: the target note has more than one `{name}:` line — YAML reads the last \
                 and the writer rewrites the first, so the change would report success without \
                 taking effect"
            ));
        }
    }

    // Per field first, so the refusal names the field responsible.
    for (key, spec) in changes {
        let name = key_text(key);
        let to = spec_get(spec, "to").unwrap_or(&Value::Null);
        if let Some(problem) = reread_problem(&text, &name, to) {
            return Some(format!("{name}: {problem}"));
        }
    }

    // Then all fields together: only the combined result shows a key being dropped or the body
    // moving.
    let literals: Vec<(String, String)> = changes
        .iter()
        .map(|(key, spec)| {
            let name = key_text(key);
            let to = spec_get(spec, "to").unwrap_or(&Value::Null);
            let literal = frontmatter_literal(&name, to);
            (name, literal)
        })
        .collect();
    const CORRUPT: &str = "would corrupt the target note's frontmatter";
    let applied =
        match apply_frontmatter_fields_to_text(&text, &literals, &target.display().to_string()) {
            Ok(applied) => applied,
            Err(_) => return Some(CORRUPT.to_string()),
        };
    let (after_meta, after_body) = match split_frontmatter(&applied) {
        Ok(parts) => parts,
        Err(_) => return Some(CORRUPT.to_string()),
    };
    for (key, value) in &before_meta {
        if changes.contains_key(key) {
            continue;
        }
        if crate::yaml::get(&after_meta, &key_text(key)) != Some(value) {
            return Some(format!("would drop or alter `{}` in the target note", key_text(key)));
        }
    }
    if after_body != before_body {
        return Some("would alter the target note's body".to_string());
    }
    None
}

/// Refusals Quinn cannot fix by editing the proposal (spec §4.3, "archive unexecuted").
///
/// Everything else is recoverable and stays pending — the safe default, since a proposal left
/// pending is visible, escalated, and expires, where an archived one is silently gone.
///
/// `"target note has malformed YAML frontmatter"` was on this list and should not have been: the
/// fault is in the target, not the proposal, and repairing the note's YAML by hand fixes it.
/// Archiving instead discarded a real amendment because of a condition elsewhere — and the
/// frontmatter corruptions this module now refuses were exactly what produced that condition, so
/// the two compounded into permanent loss of every amendment aimed at the damaged note.
pub const UNRECOVERABLE_REFUSALS: [&str; 5] = [
    "missing target",
    "target outside ",
    "target does not exist",
    "nothing to apply",
    "changes must be a map and append a list",
];

/// True when a refusal should leave the proposal pending (spec §4.3).
pub fn is_recoverable_refusal(reason: &str) -> bool {
    !UNRECOVERABLE_REFUSALS.iter().any(|prefix| reason.starts_with(prefix))
}

// ---------------------------------------------------------------------------------------------
// Applying an amendment
// ---------------------------------------------------------------------------------------------

/// Apply a validated amendment.
///
/// `Ok(None)` on success, `Ok(Some(reason))` on refusal, `Err` where the Python **raises** —
/// which is not the same thing and must not be collapsed into one channel. A refusal routes
/// through [`is_recoverable_refusal`] and either leaves the proposal pending or archives it; a
/// raise is caught one level up as `transition failed:` and leaves the proposal untouched, so the
/// next run retries it unchanged. Collapsing the two would archive proposals over a transient file
/// lock.
///
/// Everything is validated before anything is written, so a refusal never leaves the target
/// partially amended.
pub fn apply_amendment(
    vault: &Path,
    meta: &Mapping,
    ctx: &WriteContext,
    journal: &mut Journal,
) -> Result<Option<String>, WriteError> {
    let target = match validate_amendment(vault, meta) {
        Ok(target) => target,
        Err(reason) => return Ok(Some(reason)),
    };
    let (changes, appends) = match changes_and_appends(meta) {
        Ok(pair) => pair,
        Err(reason) => return Ok(Some(reason)),
    };

    const MALFORMED: &str = "target note has malformed YAML frontmatter";
    let text = pystr::read_text(&target).map_err(|e| WriteError::Io(e.to_string()))?;
    // Guard the parse: `split_frontmatter` raises on malformed YAML, and can return a non-mapping
    // whose `.get` would then raise `AttributeError`. Both are one error here.
    let current = match split_frontmatter(&text) {
        Ok((meta, _)) => meta,
        Err(_) => return Ok(Some(MALFORMED.to_string())),
    };

    // Already applied? Then this is a retry after the executed-stamp failed, not a stale proposal.
    // Without this the from-check sees `to` in place, calls it stale, and writes the proposal back
    // to pending — so today.md shows an already-applied deadline change as an outstanding PENDING
    // AMENDMENT and asks Quinn to re-approve work that is already done.
    let already_applied = !changes.is_empty()
        && changes.iter().all(|(key, spec)| {
            let name = key_text(key);
            comparable(&name, crate::yaml::get(&current, &name))
                == comparable(&name, spec_get(spec, "to"))
        });

    if !already_applied {
        for (key, spec) in changes {
            let name = key_text(key);
            let expected = comparable(&name, spec_get(spec, "from"));
            let actual = comparable(&name, crate::yaml::get(&current, &name));
            if expected != actual {
                // Flattened before interpolation: this reason becomes a `state/runner-log.md`
                // line, and a newline there produces an orphan with no
                // `- <date> <runner> <status>` shape. The retention pass cannot classify it, so it
                // is kept forever, and merge=union carries it into every future merge.
                return Ok(Some(format!(
                    "stale amendment: {name} is {}, expected {}",
                    inline_opt(crate::yaml::get(&current, &name)),
                    inline_opt(spec_get(spec, "from")),
                )));
            }
        }
    }

    // Pre-flight the append lines so a bad one cannot leave changes half-applied.
    let mut lines: Vec<&str> = Vec::new();
    for entry in appends {
        // PyYAML resolves an unquoted `- 2026-08-20` to a `date`, which is not a `str` — the whole
        // point of this check. serde_yaml_ng hands it over as a string, so the type has to be
        // re-derived from the text, exactly as `plain` does. **Documented deviation:** a
        // *quoted* `- "2026-08-20"` is a `str` to Python and is refused here; it is meaningless as
        // an append line either way, and the refusal routes identically (recoverable, stays
        // pending, fixable by hand).
        let bad_type = match entry {
            Value::String(line) => PYYAML_TIMESTAMP.is_match(line),
            _ => true,
        };
        if bad_type {
            return Ok(Some("append entries must be strings".to_string()));
        }
        let Value::String(line) = entry else { unreachable!("guarded above") };
        if let Some(problem) = single_line_problem(line) {
            return Ok(Some(format!("append line {problem}")));
        }
        lines.push(line);
    }

    // Appends first (idempotent), then frontmatter (not). If the frontmatter write fails after the
    // appends succeed, the retry re-runs the appends as a no-op while the from-values still match,
    // then applies the frontmatter change.
    let rel = rel_path(vault, &target);
    for line in lines {
        append_body(vault, &rel, line, ctx, journal)?;
    }

    if !changes.is_empty() && !already_applied {
        let literals: Vec<(String, String)> = changes
            .iter()
            .map(|(key, spec)| {
                let name = key_text(key);
                let to = spec_get(spec, "to").unwrap_or(&Value::Null);
                let literal = frontmatter_literal(&name, to);
                (name, literal)
            })
            .collect();
        write_literals(vault, &rel, &literals, ctx, journal, &WriteOpts::default())?;
    }
    Ok(None)
}

// ---------------------------------------------------------------------------------------------
// Urgency, weight and dates
// ---------------------------------------------------------------------------------------------

/// `""` and the literal string `"null"` count as absent, alongside a missing key and a real null.
///
/// The routine writes `expires: null` and `snooze_until: null` as text often enough that the two
/// spellings had to be made equivalent; treating `"null"` as a date string would make every such
/// proposal warn `bad expires` twice a day forever.
pub(crate) fn as_date(value: Option<&Value>) -> Option<Date> {
    let value = match value {
        None | Some(Value::Null) => return None,
        Some(Value::String(s)) if s.is_empty() || s == "null" => return None,
        Some(value) => value,
    };
    match coerce_datetime(Some(&Value::String(plain(value)))) {
        Ok(Some(stamp)) => Some(stamp.date()),
        _ => None,
    }
}

/// Re-derive urgency from the change set rather than trusting the label.
///
/// The routine is a language model writing YAML, and this is the one field where being wrong means
/// silently missing a deadline. **Erring toward `increases` costs one extra line in Must do; erring
/// the other way costs an assignment.** So an earlier `due`, or a status moving back to `active`,
/// overrides whatever the proposal claims.
pub fn derive_urgency(meta: &Mapping) -> String {
    if let Some(Value::Mapping(changes)) = crate::yaml::get(meta, "changes").filter(|v| is_truthy(v))
    {
        if let Some(Value::Mapping(due)) = crate::yaml::get(changes, "due") {
            let before = as_note_datetime(crate::yaml::get(due, "from"));
            let after = as_note_datetime(crate::yaml::get(due, "to"));
            if let (Some(before), Some(after)) = (before, after) {
                if after < before {
                    return "increases".to_string();
                }
            }
        }
        if let Some(Value::Mapping(status)) = crate::yaml::get(changes, "status") {
            let to = crate::yaml::get(status, "to").map(python_str).unwrap_or_else(|| "None".into());
            if pystr::strip(&to) == "active" {
                return "increases".to_string();
            }
        }
    }
    let stated = str_field(meta, "urgency");
    match stated.as_str() {
        "increases" | "decreases" | "neutral" => stated,
        _ => "neutral".to_string(),
    }
}

/// One proposal is one decision Quinn has to make, not one file.
///
/// **The `events-digest` arm stays forever.** A digest is a batch UI — one note holding N checkbox
/// events that [`expand_digest`] turns into N tasks on approval — so counting it as 1 would let a
/// single digest smuggle a whole day's ceiling past the budget.
pub fn proposal_weight(meta: &Mapping) -> i64 {
    if str_field(meta, "kind") == "events-digest" {
        return match crate::yaml::get(meta, "events") {
            Some(Value::Sequence(events)) => events.len() as i64,
            _ => 0,
        };
    }
    1
}

/// Read a note's frontmatter, or `None` for anything this module treats as unreadable.
fn read_note(path: &Path) -> Option<(Mapping, String)> {
    let text = pystr::read_text(path).ok()?;
    split_frontmatter(&text).ok()
}

// ---------------------------------------------------------------------------------------------
// The daily budget
// ---------------------------------------------------------------------------------------------

/// Proposals stamped `proposed_at == today`, across `approvals/` **and** `archive/`.
///
/// Settled proposals are moved to `archive/` by the delete path, keeping their frontmatter.
/// **Counting only `approvals/` would silently refund budget that was already spent**, the moment
/// Quinn approved or rejected something.
///
/// Unreadable or non-approval notes are skipped rather than raised on: this runs on every rank and
/// must never cost a run.
pub fn count_proposals_created(vault: &Path, today: Date) -> i64 {
    let mut total = 0;
    for folder in ["approvals", "archive"] {
        let folder = vault.join(folder);
        if !folder.is_dir() {
            continue;
        }
        for path in sorted_md(&folder) {
            let Some((meta, _)) = read_note(&path) else { continue };
            if str_field(&meta, "type") != "approval" {
                continue;
            }
            if as_date(crate::yaml::get(&meta, "proposed_at")) != Some(today) {
                continue;
            }
            total += proposal_weight(&meta);
        }
    }
    total
}

/// Snooze same-day proposals past the day's remaining budget to tomorrow.
///
/// **Deferred, never deleted**: a proposal is a judgment the cloud run already made, and the
/// standing rule is that a lost run is a lost judgment rather than a deferred one. Snoozing reuses
/// the existing wake path in [`process_approvals`], so a busy Tuesday's surplus fills a quiet
/// Wednesday.
///
/// Digests are excluded from the candidates themselves — the emitter is handed the remaining budget
/// up front and sizes itself, so there is never digest overflow to clean up here. But **any**
/// proposal created today already spent part of the day's ceiling whether or not it is still a
/// pending candidate, so the allowance is the general form
/// `max(0, budget - (count_proposals_created - candidate_weight))`. Without the `max(0, …)` clamp a
/// day already over budget produces a negative allowance and a negative slice index, which silently
/// keeps a couple of candidates from the tail instead of deferring all of them.
///
/// A write failure stops the loop and returns what was already deferred. Python raises there and
/// the raise kills the run; there is no return channel for it in this signature.
pub fn defer_over_budget(
    vault: &Path,
    today: Date,
    budget: i64,
    ctx: &WriteContext,
    journal: &mut Journal,
) -> Vec<String> {
    let folder = vault.join("approvals");
    if !folder.is_dir() || budget < 0 {
        return Vec::new();
    }

    let mut candidates: Vec<(Date, String, PathBuf)> = Vec::new();
    for path in sorted_md(&folder) {
        let Some((meta, _)) = read_note(&path) else { continue };
        if str_field(&meta, "type") != "approval" {
            continue;
        }
        if str_field(&meta, "kind") == "events-digest" {
            continue;
        }
        if str_field(&meta, "status") != "pending" {
            continue;
        }
        if as_date(crate::yaml::get(&meta, "proposed_at")) != Some(today) {
            continue;
        }
        let expires = as_date(crate::yaml::get(&meta, "expires")).unwrap_or(Date::MAX);
        candidates.push((expires, stem_of(&path), path));
    }

    let candidate_weight = candidates.len() as i64;
    let already_spent = count_proposals_created(vault, today) - candidate_weight;
    let allowance = (budget - already_spent).max(0) as usize;

    // Soonest-expiring wins a contested day; undated (`date.max`) rolls forward first.
    candidates.sort_by(|a, b| (a.0, &a.1).cmp(&(b.0, &b.1)));
    let Ok(tomorrow) = today.tomorrow() else { return Vec::new() };
    let stamp = tomorrow.strftime("%Y-%m-%d").to_string();

    let mut deferred: Vec<String> = Vec::new();
    for (_, stem, path) in candidates.into_iter().skip(allowance) {
        let literals = vec![
            ("status".to_string(), "snoozed".to_string()),
            ("snooze_until".to_string(), stamp.clone()),
            ("proposed_at".to_string(), stamp.clone()),
        ];
        let rel = rel_path(vault, &path);
        if write_literals(vault, &rel, &literals, ctx, journal, &WriteOpts::default()).is_err() {
            break;
        }
        deferred.push(stem);
    }
    deferred
}

// ---------------------------------------------------------------------------------------------
// Digests
// ---------------------------------------------------------------------------------------------

static CHECKED: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"^- \[[xX]\] .*`([^`]+)`\s*$").unwrap());
static UNCHECKED: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"^- \[ \] .*`([^`]+)`\s*$").unwrap());

/// `(checked, unchecked)` uids from a digest body.
fn digest_uids(body: &str) -> (BTreeSet<String>, BTreeSet<String>) {
    let mut checked = BTreeSet::new();
    let mut unchecked = BTreeSet::new();
    for line in pystr::splitlines(body) {
        if let Some(caps) = CHECKED.captures(line) {
            checked.insert(caps[1].to_string());
            continue;
        }
        if let Some(caps) = UNCHECKED.captures(line) {
            unchecked.insert(caps[1].to_string());
        }
    }
    (checked, unchecked)
}

/// YAML parses `start: 2026-09-03T18:00:00` into a datetime; write it back in ISO form.
///
/// Without this the interpolation renders `2026-09-03 18:00:00`, which the routine's calendar
/// executor does not expect. [`plain`] already re-derives the date/datetime distinction serde
/// collapses, so the whole job here is Python's trailing `.strip()`.
fn stamp(value: Option<&Value>) -> String {
    match value {
        None | Some(Value::Null) => "None".to_string(),
        Some(value) => pystr::strip(&plain(value)).to_string(),
    }
}

/// Write one approved calendar-event note from a digest payload entry.
fn calendar_note(
    vault: &Path,
    entry: &Mapping,
    ctx: &WriteContext,
    journal: &mut Journal,
) -> Result<Option<String>, WriteError> {
    let uid = truthy_str(entry, "uid");
    let summary = truthy_str(entry, "summary");
    if uid.is_empty() || summary.is_empty() {
        return Ok(None);
    }
    // `slugify` already caps at 60 characters and never returns empty, so both the slice and the
    // `or "event"` fallback are inert. Kept because they are in the Python.
    let slug: String = slugify(&summary).chars().take(60).collect();
    let slug = if slug.is_empty() { "event".to_string() } else { slug };

    let folder = vault.join("approvals");
    let mut target = folder.join(format!("calendar-event-{slug}.md"));
    let mut suffix = 2;
    while target.exists() {
        target = folder.join(format!("calendar-event-{slug}-{suffix}.md"));
        suffix += 1;
    }

    let summary_literal = quote_escape(&summary);
    let location_raw = match crate::yaml::get(entry, "location") {
        Some(value) if is_truthy(value) => python_str(value),
        _ => String::new(),
    };
    let location_literal = quote_escape(&location_raw);
    // The uid is escaped like every other written scalar. It is safe unescaped only because the
    // ledger's uid charclass rejects quotes and backslashes; relying on that from here would make
    // widening the charclass corrupt this note.
    let uid_literal = quote_escape(&uid);
    let text = format!(
        "---\n\
         type: approval\n\
         kind: calendar-event\n\
         title: \"{summary_literal}\"\n\
         status: approved\n\
         source_uid: \"{uid_literal}\"\n\
         created_by: events\n\
         event:\n  \
         summary: \"{summary_literal}\"\n  \
         start: {start}\n  \
         end: {end}\n  \
         location: \"{location_literal}\"\n\
         ---\n\n\
         Approved from the events digest. Source uid `{uid}`.\n",
        start = stamp(crate::yaml::get(entry, "start")),
        end = stamp(crate::yaml::get(entry, "end")),
    );
    let rel = rel_path(vault, &target);
    create(vault, &rel, &text, ctx, journal, None)?;
    Ok(Some(stem_of(&target)))
}

/// Every `source_uid` already written into a note in `approvals/` or `archive/`.
///
/// Spec §8's once-only guarantee, covering the two halves the ledger line does not: an existing
/// note in `approvals/` blocks re-emission, a settled note in `archive/` blocks it permanently.
/// Without the check, a digest whose `executed` stamp failed — a transient Windows file lock from
/// Obsidian Git is enough — stays `approved`, is re-expanded next run, and [`calendar_note`]'s
/// uniquifying suffix loop turns the collision into a SECOND approved note, hence a second real
/// Google Calendar event. Never fails: an unreadable neighbour is skipped.
pub fn existing_source_uids(vault: &Path) -> BTreeSet<String> {
    let mut uids = BTreeSet::new();
    for folder in ["approvals", "archive"] {
        let directory = vault.join(folder);
        if !directory.is_dir() {
            continue;
        }
        for path in sorted_md(&directory) {
            let Some((meta, _)) = read_note(&path) else { continue };
            let uid = truthy_str(&meta, "source_uid");
            if !uid.is_empty() {
                uids.insert(uid);
            }
        }
    }
    uids
}

/// Expand a settled digest. Returns `(created, declined, warnings)`.
#[allow(clippy::too_many_arguments)]
fn expand_digest(
    vault: &Path,
    path: &Path,
    meta: &Mapping,
    body: &str,
    today: Date,
    approve: bool,
    ctx: &WriteContext,
    journal: &mut Journal,
) -> Result<(i64, i64, Vec<String>), WriteError> {
    let name = name_of(path);
    let payload = match crate::yaml::get(meta, "events") {
        Some(Value::Sequence(events)) if !events.is_empty() => events.clone(),
        _ => return Ok((0, 0, vec![format!("events-digest with no payload: {name}")])),
    };
    let (checked, _) = digest_uids(body);
    let mut already = if approve { existing_source_uids(vault) } else { BTreeSet::new() };
    let mut created = 0;
    let mut declined = 0;
    let mut warnings: Vec<String> = Vec::new();

    for entry in &payload {
        let Value::Mapping(entry) = entry else { continue };
        let uid = truthy_str(entry, "uid");
        if uid.is_empty() {
            warnings.push(format!("{name}: payload entry missing uid"));
            continue;
        }
        if approve && checked.contains(&uid) {
            if already.contains(&uid) {
                created += 1; // an earlier pass already wrote it; re-expansion is a no-op
                continue;
            }
            match calendar_note(vault, entry, ctx, journal)? {
                None => {
                    warnings.push(format!("{name}: bad payload entry for {uid}"));
                    continue;
                }
                Some(_) => {
                    already.insert(uid);
                    created += 1;
                }
            }
        } else {
            record_declined(vault, &uid, today).map_err(|e| WriteError::Io(e.to_string()))?;
            declined += 1;
        }
    }
    Ok((created, declined, warnings))
}

// ---------------------------------------------------------------------------------------------
// The pass
// ---------------------------------------------------------------------------------------------

/// Settle every proposal in `approvals/`.
///
/// **This must never raise into the run.** `cli.run` calls it before `today.md` is written and
/// before any runner-log line is appended, so an escaping error leaves the page silently stale.
/// Every per-note failure becomes one of four warning strings — `unreadable:`,
/// `not an approval note:`, `transition failed:`, `unknown status:` — and those strings are part of
/// the observable output: they reach `state/runner-log.md`.
pub fn process_approvals(
    vault: &Path,
    today: Date,
    now: DateTime,
    ctx: &WriteContext,
    journal: &mut Journal,
) -> ApprovalsResult {
    let mut result = ApprovalsResult::default();
    let folder = vault.join("approvals");
    if !folder.is_dir() {
        return result;
    }
    let mut oldest: Option<Date> = None;

    for path in sorted_md(&folder) {
        let name = name_of(&path);
        let Some((meta, body)) = read_note(&path) else {
            result.warnings.push(format!("unreadable: {name}"));
            continue;
        };
        if str_field(&meta, "type") != "approval" {
            result.warnings.push(format!("not an approval note: {name}"));
            continue;
        }
        let transition = transition_note(
            vault,
            &path,
            &meta,
            &body,
            today,
            now,
            ctx,
            journal,
            &mut result,
            &mut oldest,
        );
        if transition.is_err() {
            result.warnings.push(format!("transition failed: {name}"));
        }
    }

    if let Some(oldest) = oldest {
        result.oldest_pending_days = days_between(oldest, today).max(0);
    }
    result
}

/// A rejected SYNC card re-asserts this device's value (R-C3′-exec-20, Task 6b).
///
/// A sync card (`created_by: agent:knowlu.sync`) offers another desktop's later value over this
/// device's own, which sync withheld. Rejecting it means "keep mine" — but nothing on this device
/// changes, so without a record nothing travels, and the other desktop, which won the field at its
/// own end and has no card, keeps its value: A and B diverge for good, with the student's explicit
/// choice lost. So for each field the card names, one journal-only `set` goes out through
/// `write::reassert` — `old` the card's `to`, `new` the note's current value, a fresh `ts`, this
/// context's actor — and the other desktop takes it cleanly or, if it has moved on, files a card of
/// its own.
///
/// **A field is skipped when the note no longer holds the card's `from` for it** — compared exactly
/// as `apply_amendment`'s own from-check compares: a later local edit is already travelling as its
/// own record, and re-asserting over it would restate the wrong thing. A judge-once card is never
/// passed here: rejecting one tells no other device anything. A card whose target has gone, or whose
/// `changes` or target frontmatter no longer parse, re-asserts nothing — the rejection itself still
/// settles the card.
fn reassert_rejected_sync_card(
    vault: &Path,
    meta: &Mapping,
    ctx: &WriteContext,
    journal: &mut Journal,
) -> Result<(), WriteError> {
    let Some(target) = resolve_amend_target(vault, crate::yaml::get(meta, "target")) else { return Ok(()) };
    if !target.is_file() {
        return Ok(());
    }
    let Ok((changes, _)) = changes_and_appends(meta) else { return Ok(()) };
    let Ok(text) = pystr::read_text(&target) else { return Ok(()) };
    let Ok((current, _)) = split_frontmatter(&text) else { return Ok(()) };
    let target_rel = rel_path(vault, &target);
    for (key, spec) in changes {
        let name = key_text(key);
        if !AMENDABLE_FIELDS.contains(&name.as_str()) {
            continue;
        }
        let Some(to) = spec_get(spec, "to") else { continue };
        if comparable(&name, spec_get(spec, "from")) != comparable(&name, crate::yaml::get(&current, &name)) {
            continue;
        }
        crate::write::reassert(vault, &target_rel, &name, &crate::yaml::to_json(to), ctx, journal)?;
    }
    Ok(())
}

/// One note's state transition. `Err` stands in for Python's `except (OSError, ValueError,
/// yaml.YAMLError)`, and every early `return Ok(())` is one of the original's `continue`s.
#[allow(clippy::too_many_arguments)]
fn transition_note(
    vault: &Path,
    path: &Path,
    meta: &Mapping,
    body: &str,
    today: Date,
    now: DateTime,
    ctx: &WriteContext,
    journal: &mut Journal,
    result: &mut ApprovalsResult,
    oldest: &mut Option<Date>,
) -> Result<(), WriteError> {
    let name = name_of(path);
    let stem = stem_of(path);
    let rel = rel_path(vault, path);
    let mut status = str_field(meta, "status");
    let kind = str_field(meta, "kind");

    if status == "rejected" {
        if kind == "events-digest" {
            let (_, _, warnings) =
                expand_digest(vault, path, meta, body, today, false, ctx, journal)?;
            result.warnings.extend(warnings);
        } else if kind == "event-check" {
            settle_event_check(vault, meta, "drop", today, journal)?;
        } else if kind == "event-accept" {
            // Events spec §4.3: one `declined` line per uid the card's `events:` lists, before the
            // move, so a failed ledger write leaves the card `rejected` for the next run. These are
            // the uids the carry's D9 rebuild reads from the archived card. No note, no new word.
            for uid in crate::eventemit::card_event_uids(meta) {
                record_declined(vault, &uid, today).map_err(|e| WriteError::Io(e.to_string()))?;
            }
        } else if kind == COMMITMENT_CHECK {
            // P13 (§5.2, §5.4): the decline marker(s) first, so a failed write leaves the card
            // `rejected` for the next run rather than archived with the question left open.
            crate::commitments::settle_rejected(vault, meta, ctx, journal)?;
        } else if kind == COMMITMENT_ASK {
            // Phase-2 spec §5: the card:<slug> marker first, then the generic archive.
            crate::commitments::settle_ask_rejected(vault, meta, ctx, journal)?;
        }
        if kind == "amend" && str_field(meta, "created_by") == crate::sync::ACTOR {
            reassert_rejected_sync_card(vault, meta, ctx, journal)?;
        }
        delete(vault, &rel, ctx, journal)?;
        result.rejected.push(stem);
        return Ok(());
    }

    if (kind == COMMITMENT_CHECK || kind == COMMITMENT_ASK) && matches!(status.as_str(), "refused" | "superseded") {
        // Fix round 1, m1 (controller ruling): the settlement's stamp landed and the run died
        // before the move. Finish it, quietly. `commitment-check` only: the amend path keeps its
        // behaviour.
        delete(vault, &rel, ctx, journal)?;
        return Ok(());
    }

    if status == "executed" {
        // Stamped by the cloud calendar executor; archiving is our job.
        delete(vault, &rel, ctx, journal)?;
        result.executed.push(stem);
        return Ok(());
    }

    if status == "snoozed" {
        let raw = crate::yaml::get(meta, "snooze_until");
        let absent = match raw {
            None | Some(Value::Null) => true,
            Some(Value::String(s)) => s.is_empty() || s == "null",
            Some(_) => false,
        };
        if absent {
            result.warnings.push(format!("snoozed without snooze_until: {name}"));
            return Ok(());
        }
        let Some(wake) = as_date(raw) else {
            result.warnings.push(format!("bad snooze_until: {name}"));
            return Ok(());
        };
        if wake > today {
            return Ok(());
        }
        let literals = vec![("status".to_string(), "pending".to_string())];
        write_literals(vault, &rel, &literals, ctx, journal, &WriteOpts::default())?;
        result.woken.push(stem.clone());
        status = "pending".to_string();
    }

    if status == "pending" {
        let raw = crate::yaml::get(meta, "expires");
        let expires = as_date(raw);
        let raw_is_absent = match raw {
            None | Some(Value::Null) => true,
            Some(Value::String(s)) => s.is_empty() || s == "null",
            Some(_) => false,
        };
        if expires.is_none() && !raw_is_absent {
            result.warnings.push(format!("bad expires: {name}"));
        }
        if let Some(expires) = expires {
            if expires < today {
                if kind == "events-digest" {
                    let (_, _, warnings) =
                        expand_digest(vault, path, meta, body, today, false, ctx, journal)?;
                    result.warnings.extend(warnings);
                }
                let literals = vec![("status".to_string(), "expired".to_string())];
                write_literals(vault, &rel, &literals, ctx, journal, &WriteOpts::default())?;
                delete(vault, &rel, ctx, journal)?;
                result.expired.push(stem);
                return Ok(());
            }
        }
        if kind == "events-digest" {
            result.events_in_digest += proposal_weight(meta);
        } else {
            result.pending += 1;
        }
        let proposed = as_date(crate::yaml::get(meta, "proposed_at"));
        // `defer_over_budget` rewrites `proposed_at` to the day the proposal actually charges, so a
        // repeatedly-deferred proposal never ages. `first_proposed_at` is set once and is the true
        // age; notes written before S1 lack it and fall back.
        let first = as_date(crate::yaml::get(meta, "first_proposed_at")).or(proposed);
        if proposed.is_none() {
            // `age_days` is the escalation block's only urgency signal. Defaulting it to 0 silently
            // makes Must do read "(0d unactioned)" forever however long it has really sat, while
            // the adjacent `expires` warns for the same fault.
            result.warnings.push(format!("bad proposed_at: {name}"));
        }
        if kind == "amend" {
            let raw_changes = crate::yaml::get(meta, "changes");
            let changes = match raw_changes {
                Some(Value::Mapping(map)) => map.clone(),
                _ => Mapping::new(),
            };
            let title = match crate::yaml::get(meta, "title") {
                Some(value) if is_truthy(value) => python_str(value),
                _ => stem.clone(),
            };
            result.amendments_pending.push(AmendmentEntry {
                target: truthy_str(meta, "target"),
                title,
                urgency: derive_urgency(meta),
                proposal: stem.clone(),
                changes,
                age_days: first.map(|first| days_between(first, today).max(0)).unwrap_or(0),
            });
        }
        if let Some(first) = first {
            if oldest.is_none_or(|current| first < current) {
                *oldest = Some(first);
            }
        }
        return Ok(());
    }

    if status == "approved" {
        let stamped = format!("\"{}\"", now.strftime("%Y-%m-%d %H:%M"));
        if kind == "calendar-event" {
            result.awaiting_calendar += 1;
        } else if kind == "task" {
            let Some(slug) = materialize(vault, path, body, ctx, journal)? else {
                result.warnings.push(format!("missing task payload: {name}"));
                return Ok(());
            };
            let literals = vec![
                ("status".to_string(), "executed".to_string()),
                ("executed_at".to_string(), stamped),
            ];
            write_literals(vault, &rel, &literals, ctx, journal, &WriteOpts::default())?;
            delete(vault, &rel, ctx, journal)?;
            result.executed.push(slug);
        } else if kind == "amend" {
            if let Some(reason) = apply_amendment(vault, meta, ctx, journal)? {
                result.warnings.push(format!("{name}: {reason}"));
                // Recoverable by hand (stale value, bad field, bad line) → write the status back to
                // pending, so the proposal is counted, escalated into Must do, and eventually
                // expires. Leaving it `approved` on disk means none of that happens and it re-WARNs
                // forever. Unrecoverable (target gone, outside the allowed folders) → archive so it
                // stops re-WARNing.
                if is_recoverable_refusal(&reason) {
                    let literals = vec![("status".to_string(), "pending".to_string())];
                    write_literals(vault, &rel, &literals, ctx, journal, &WriteOpts::default())?;
                    return Ok(());
                }
                // Stamp a terminal status first: `archive/` is where settled things go, and a note
                // left reading `approved` is the only archived state that misrepresents itself —
                // anything that moves it back re-attempts the change.
                let literals = vec![("status".to_string(), "refused".to_string())];
                write_literals(vault, &rel, &literals, ctx, journal, &WriteOpts::default())?;
                delete(vault, &rel, ctx, journal)?;
                return Ok(());
            }
            let literals = vec![
                ("status".to_string(), "executed".to_string()),
                ("executed_at".to_string(), stamped),
            ];
            write_literals(vault, &rel, &literals, ctx, journal, &WriteOpts::default())?;
            delete(vault, &rel, ctx, journal)?;
            result.executed.push(stem);
        } else if kind == "events-digest" {
            let (created, _, warnings) =
                expand_digest(vault, path, meta, body, today, true, ctx, journal)?;
            result.warnings.extend(warnings);
            let literals = vec![
                ("status".to_string(), "executed".to_string()),
                ("executed_at".to_string(), stamped),
            ];
            write_literals(vault, &rel, &literals, ctx, journal, &WriteOpts::default())?;
            delete(vault, &rel, ctx, journal)?;
            result.executed.push(format!("{stem} ({created} events)"));
        } else if kind == "event-check" {
            // F3: the student's "yes, this applies to me". Ledger first, so a failed answer leaves
            // the card `approved` for the next run rather than archived with nothing written.
            settle_event_check(vault, meta, "obligation", today, journal)?;
            let literals = vec![
                ("status".to_string(), "executed".to_string()),
                ("executed_at".to_string(), stamped),
            ];
            write_literals(vault, &rel, &literals, ctx, journal, &WriteOpts::default())?;
            delete(vault, &rel, ctx, journal)?;
            result.executed.push(stem);
        } else if kind == "event-accept" {
            // Events spec §4.2 (D1, D10): the commitments and the register task first, the stamp
            // second, as `event-check` does. A failed write leaves the card `approved` for the next
            // run, and the ever-written set makes that retry write nothing twice. A card with no
            // level or no payload (only a hand edit makes one) writes nothing and waits, as an
            // approved `task` card with no payload does.
            let level = match str_field(meta, "verdict").as_str() {
                "obligation" => crate::commitments::Level::Hard,
                "opportunity" => crate::commitments::Level::Soft,
                _ => {
                    result.warnings.push(format!("{name}: no verdict the settlement reads"));
                    return Ok(());
                }
            };
            let Some(warnings) = settle_event_accept(vault, meta, level, today, ctx, journal)? else {
                result.warnings.push(format!("missing event payload: {name}"));
                return Ok(());
            };
            result.warnings.extend(warnings.into_iter().map(|w| format!("{name}: {w}")));
            let literals = vec![
                ("status".to_string(), "executed".to_string()),
                ("executed_at".to_string(), stamped),
            ];
            write_literals(vault, &rel, &literals, ctx, journal, &WriteOpts::default())?;
            delete(vault, &rel, ctx, journal)?;
            result.executed.push(stem);
        } else if kind == COMMITMENT_CHECK {
            // P13 (§5.2, §5.4): the confirmed note, the planning day or the change first, the
            // stamp second — a failed write leaves the card `approved` for the next run.
            let settled = crate::commitments::settle_approved(vault, meta, today, ctx, journal)?;
            let (status, warning) = match &settled {
                Settled::Executed => ("executed", None),
                Settled::Refused(why) => ("refused", Some(why)),
                Settled::Superseded(why) => ("superseded", Some(why)),
            };
            let mut literals = vec![("status".to_string(), status.to_string())];
            if settled == Settled::Executed {
                literals.push(("executed_at".to_string(), stamped));
            }
            write_literals(vault, &rel, &literals, ctx, journal, &WriteOpts::default())?;
            delete(vault, &rel, ctx, journal)?;
            match warning {
                Some(why) => result
                    .warnings
                    .push(format!("{name}: {}", crate::commitments::single_line(why))),
                None => result.executed.push(stem),
            }
        } else if kind == COMMITMENT_ASK {
            // Phase-2 spec §5 (Plan ruling Q7-a): the class note first, the stamp second. An
            // invalid answer puts the card back to pending, unarchived, with the warning.
            match crate::commitments::settle_ask_approved(vault, meta, today, ctx, journal)? {
                AskSettled::Returned(why) => {
                    let literals = vec![("status".to_string(), "pending".to_string())];
                    write_literals(vault, &rel, &literals, ctx, journal, &WriteOpts::default())?;
                    result.warnings.push(format!("{name}: {}", crate::commitments::single_line(&why)));
                }
                AskSettled::Executed => {
                    let literals = vec![
                        ("status".to_string(), "executed".to_string()),
                        ("executed_at".to_string(), stamped),
                    ];
                    write_literals(vault, &rel, &literals, ctx, journal, &WriteOpts::default())?;
                    delete(vault, &rel, ctx, journal)?;
                    result.executed.push(stem);
                }
                AskSettled::Refused(why) => {
                    let literals = vec![("status".to_string(), "refused".to_string())];
                    write_literals(vault, &rel, &literals, ctx, journal, &WriteOpts::default())?;
                    delete(vault, &rel, ctx, journal)?;
                    result.warnings.push(format!("{name}: {}", crate::commitments::single_line(&why)));
                }
            }
        } else if kind == "coursework-map" {
            // C2 (§11a, R-OB-1): a mapping from an unmapped zyBook or VHL section to a course.
            // `rank` does not apply it — the next `coursework` step does, before it fetches, so a
            // card approved at 11am is a mapping the noon slot already uses. Left exactly as it is
            // here, and deliberately not an `unknown kind` warning: a WARN line on every run until
            // the next slot would train the reader to ignore the list.
        } else if kind == "rule" {
            // C2 (cloud design §5.4): a promoted rule. `rank` never opens a socket for a judgment,
            // so the decision is *sent* by the next `judge` step, which then stamps this card and
            // archives it. Left exactly as it is here, deliberately not an `unknown kind` warning,
            // because a WARN line on every run until the next slot would train the reader to ignore
            // the list, which is the one thing `rank_warnings` exists to prevent.
        } else {
            result.warnings.push(format!("unknown kind: {name}"));
        }
        return Ok(());
    }

    result.warnings.push(format!("unknown status: {name}"));
    Ok(())
}

/// Withdraw every pending or snoozed `commitment-check` card whose question went away (§5.2
/// "Withdrawn", by card shape — plan review C1; [`crate::commitments::withdrawal_reason`] holds
/// the three tests): stamped `superseded` and archived, no other write. Called by `rank` (P16)
/// after `commitments::load`. Only `approvals/` is read — a card the student deleted is already
/// in `archive/` as it stood, and closes its question like an answer (carry-forward 4).
///
/// **No cap-charge loop, by construction:** each condition also stops P9's `proposals` or P12's
/// `detect_changes` producing that card again, except an edited `was`, where the change is asked
/// once more with the new `was` and that card then survives. `today` dates the "series has
/// finished" test (fix round 1, m3); there is no expiry (R9).
///
/// Never raises into the run: returns the withdrawn stems, how many of them were `pending` on
/// disk, and one `transition failed:` warning per card that could not be moved.
pub fn withdraw_stale(
    vault: &Path,
    file: &SeriesFile,
    set: &Commitments,
    today: Date,
    ctx: &WriteContext,
    journal: &mut Journal,
) -> Withdrawal {
    let mut out = Withdrawal::default();
    let folder = vault.join("approvals");
    if !folder.is_dir() {
        return out;
    }
    let (codes, _) = Codes::load(vault);
    for path in sorted_md(&folder) {
        let Some((meta, _)) = read_note(&path) else { continue };
        if str_field(&meta, "type") != "approval"
            || str_field(&meta, "kind") != COMMITMENT_CHECK
            || !matches!(str_field(&meta, "status").as_str(), "pending" | "snoozed")
        {
            continue;
        }
        if crate::commitments::withdrawal_reason(vault, &meta, file, set, &codes, today).is_none() {
            continue;
        }
        let rel = rel_path(vault, &path);
        let literals = vec![("status".to_string(), "superseded".to_string())];
        let moved = write_literals(vault, &rel, &literals, ctx, journal, &WriteOpts::default())
            .and_then(|_| delete(vault, &rel, ctx, journal));
        match moved {
            Ok(_) => {
                out.withdrawn.push(stem_of(&path));
                if str_field(&meta, "status") == "pending" {
                    out.pending += 1;
                }
            }
            Err(_) => out.warnings.push(format!("transition failed: {}", name_of(&path))),
        }
    }
    out
}

/// Withdraw every pending or snoozed `commitment-ask` card whose question went away
/// ([`crate::commitments::ask_withdrawal_reason`]): stamped `superseded` and archived, no other
/// write. `rank` calls it once the proposals exist (Plan ruling Q7-c). Never raises into the run.
pub fn withdraw_asks(
    vault: &Path,
    proposals: &[Proposal],
    set: &Commitments,
    ctx: &WriteContext,
    journal: &mut Journal,
) -> Withdrawal {
    let mut out = Withdrawal::default();
    let folder = vault.join("approvals");
    if !folder.is_dir() {
        return out;
    }
    let (codes, _) = Codes::load(vault);
    for path in sorted_md(&folder) {
        let Some((meta, _)) = read_note(&path) else { continue };
        if str_field(&meta, "type") != "approval"
            || str_field(&meta, "kind") != COMMITMENT_ASK
            || !matches!(str_field(&meta, "status").as_str(), "pending" | "snoozed")
        {
            continue;
        }
        if crate::commitments::ask_withdrawal_reason(&meta, proposals, set, &codes).is_none() {
            continue;
        }
        let rel = rel_path(vault, &path);
        let literals = vec![("status".to_string(), "superseded".to_string())];
        let moved = write_literals(vault, &rel, &literals, ctx, journal, &WriteOpts::default())
            .and_then(|_| delete(vault, &rel, ctx, journal));
        match moved {
            Ok(_) => {
                out.withdrawn.push(stem_of(&path));
                if str_field(&meta, "status") == "pending" {
                    out.pending += 1;
                }
            }
            Err(_) => out.warnings.push(format!("transition failed: {}", name_of(&path))),
        }
    }
    out
}

/// What [`withdraw_stale`] did. `pending` counts the withdrawn cards that were `pending` on disk
/// — the ones `process_approvals` already counted into this run's `Approvals: N pending` line (a
/// snoozed card never was), so `rank` takes them back out (P16 fix round 1, I1).
#[derive(Debug, Default)]
pub struct Withdrawal {
    pub withdrawn: Vec<String>,
    pub pending: i64,
    pub warnings: Vec<String>,
}

/// The age in days of the oldest `pending` approval in `approvals/` — `first_proposed_at`, else
/// `proposed_at`, as [`process_approvals`] ages one — or 0 when none is pending. `rank` re-reads
/// it only after a withdrawal took cards out of the queue `process_approvals` aged (I1).
pub fn oldest_pending_days(vault: &Path, today: Date) -> i64 {
    let folder = vault.join("approvals");
    if !folder.is_dir() {
        return 0;
    }
    let mut oldest: Option<Date> = None;
    for path in sorted_md(&folder) {
        let Some((meta, _)) = read_note(&path) else { continue };
        if str_field(&meta, "type") != "approval" || str_field(&meta, "status") != "pending" {
            continue;
        }
        let first = as_date(crate::yaml::get(&meta, "first_proposed_at"))
            .or(as_date(crate::yaml::get(&meta, "proposed_at")));
        if let Some(first) = first {
            if oldest.is_none_or(|current| first < current) {
                oldest = Some(first);
            }
        }
    }
    oldest.map(|first| days_between(first, today).max(0)).unwrap_or(0)
}

/// Write the student's answer to a `kind: event-check` card into the event ledger (F3):
/// `obligation` for an approved card, `drop` for a rejected one.
///
/// One [`record_answer`] line per uid in the card's `events:` (falling back to `[source_uid]`).
/// `by` is the actor of the journal's human `status` set on the card, or `"unknown"` when there is
/// none (a card edited by hand outside the console). The primary (`source_uid`) carries the card's
/// `judgment_id`; every other uid carries its own ledger entry's, or none. Each uid's title is its
/// own ledger entry's, so a series card's `· +N more` title is never copied onto every instance;
/// the card's title is only the fallback. An I/O failure is `WriteError::Io`, which leaves the card
/// where it is for the next run; answering twice is harmless (`load_ledger` reads the same state).
fn settle_event_check(
    vault: &Path,
    meta: &Mapping,
    verdict: &str,
    today: Date,
    journal: &mut Journal,
) -> Result<(), WriteError> {
    let source = truthy_str(meta, "source_uid");
    let mut uids = crate::eventemit::card_event_uids(meta);
    if uids.is_empty() && !source.is_empty() {
        uids.push(source.clone());
    }
    let card_id = truthy_str(meta, "id");
    let by = if card_id.is_empty() { None } else { journal.human_set(&card_id, "status") }
        .and_then(|record| record.get("actor").and_then(|a| a.as_str()).map(str::to_string))
        .unwrap_or_else(|| "unknown".to_string());
    let card_jid = truthy_str(meta, "judgment_id");
    let card_title = truthy_str(meta, "title");
    let ledger = crate::eventledger::load_ledger(vault, None);
    for uid in &uids {
        let entry = ledger.get(uid);
        let jid = if *uid == source {
            card_jid.clone()
        } else {
            entry.map(|e| e.judgment_id.clone()).unwrap_or_default()
        };
        let title = entry
            .map(|e| e.title.clone())
            .filter(|t| !t.is_empty())
            .unwrap_or_else(|| card_title.clone());
        let jid = Some(jid.as_str()).filter(|j| !j.is_empty());
        let recorded = match record_answer(vault, uid, &title, today, verdict, &by, jid) {
            // A hand-edited, malformed `judgment_id` must not strand the card forever: the answer
            // matters more than the trace back to its judgment.
            Err(VerdictError::BadField("jid")) => {
                record_answer(vault, uid, &title, today, verdict, &by, None)
            }
            other => other,
        };
        recorded.map_err(|e| WriteError::Io(e.to_string()))?;
    }
    Ok(())
}

/// Write the notes an accepted event card books (events spec §4.2, D1, D10), from the card's own
/// `instances:`. `process_approvals` runs with no feed at hand, so the payload is the only input.
///
/// For each entry (`eventaccept::Instance::from_yaml`) whose uid is not in
/// `eventcarry::ever_written`, the commitment `eventaccept::commitment_for` gives at `level`
/// through `commitments::create_confirmed` (actor `agent:commitments`, journal first). A lane shape
/// (all-day, multi-day, zero-length) has none: the answer stands, and nothing warns. Then, for the
/// primary (`source_uid`) alone, the "Register" task `eventaccept::register_task` gives, through
/// `write::create` under `ctx`, unless `register:<uid>` is in the set; `-2`, `-3` on a name
/// collision. The set reads `commitments/`, `tasks/` and `archive/`, approval cards left out, so a
/// retry after a failed stamp writes nothing twice and a note the student deleted never returns.
///
/// `Ok(None)`: the card carries no `instances:` payload, and nothing was written. Otherwise the
/// warnings, one per malformed entry, which is skipped. `Err` is a failed write: the caller leaves
/// the card `approved`, and the next pass retries.
fn settle_event_accept(
    vault: &Path,
    meta: &Mapping,
    level: crate::commitments::Level,
    today: Date,
    ctx: &WriteContext,
    journal: &mut Journal,
) -> Result<Option<Vec<String>>, WriteError> {
    let entries = match crate::yaml::get(meta, "instances") {
        Some(Value::Sequence(entries)) if !entries.is_empty() => entries,
        _ => return Ok(None),
    };
    let primary = truthy_str(meta, "source_uid");
    let mut written = crate::eventcarry::ever_written(vault);
    let mut warnings = Vec::new();
    let mut register = None;
    for (index, entry) in entries.iter().enumerate() {
        let Some(instance) = entry.as_mapping().and_then(crate::eventaccept::Instance::from_yaml) else {
            warnings.push(format!("bad instance entry {}", index + 1));
            continue;
        };
        if instance.uid == primary && register.is_none() {
            register = crate::eventaccept::register_task(&instance);
        }
        if written.contains(&instance.uid) {
            continue;
        }
        let Some(commitment) = crate::eventaccept::commitment_for(&instance, level) else { continue };
        crate::commitments::create_confirmed(vault, &commitment, &instance.uid, today, ctx, journal)?;
        written.insert(instance.uid);
    }
    if let Some((stem, text)) = register.filter(|_| !written.contains(&format!("register:{primary}"))) {
        let mut rel = format!("tasks/{stem}.md");
        let mut suffix = 2;
        while vault.join(&rel).exists() {
            rel = format!("tasks/{stem}-{suffix}.md");
            suffix += 1;
        }
        create(vault, &rel, &text, ctx, journal, None)?;
    }
    Ok(Some(warnings))
}

static PAYLOAD_FENCE: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"(?sm)^```task[ \t]*\n(.*?)^```[ \t]*$").unwrap());

/// Turn an approved `kind: task` proposal's fenced payload into a real task note.
///
/// `Ok(None)` means "missing task payload", which is what the caller warns. That covers a missing
/// fence **and** a fence whose content has no frontmatter (or a block-style `judgment:`): the old
/// writer minted a note `load_tasks` would silently skip, and settling it the same way as a missing
/// fence is what stops that.
fn materialize(
    vault: &Path,
    path: &Path,
    body: &str,
    ctx: &WriteContext,
    journal: &mut Journal,
) -> Result<Option<String>, WriteError> {
    let Some(caps) = PAYLOAD_FENCE.captures(body) else { return Ok(None) };
    let tasks_dir = vault.join("tasks");
    std::fs::create_dir_all(&tasks_dir).map_err(|e| WriteError::Io(e.to_string()))?;

    let stem = stem_of(path);
    let slug = stem.strip_prefix("task-").unwrap_or(&stem).to_string();
    let mut target = tasks_dir.join(format!("{slug}.md"));
    let mut suffix = 2;
    while target.exists() {
        target = tasks_dir.join(format!("{slug}-{suffix}.md"));
        suffix += 1;
    }

    let mut content = caps[1].to_string();
    if !content.ends_with('\n') {
        content.push('\n');
    }
    let rel = rel_path(vault, &target);
    match create(vault, &rel, &content, ctx, journal, None) {
        Ok(_) => Ok(Some(stem_of(&target))),
        // Python catches only `ValueError` here. `FileExistsError` and an I/O failure are
        // `OSError`, and they propagate to the caller's `transition failed:`.
        Err(err @ (WriteError::Io(_) | WriteError::Exists(_))) => Err(err),
        Err(_) => Ok(None),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn v(text: &str) -> Value {
        Value::String(text.to_string())
    }

    #[test]
    fn as_note_datetime_gives_a_bare_date_the_2359_placeholder() {
        assert_eq!(
            as_note_datetime(Some(&v("2026-09-22"))),
            Some(DateTime::constant(2026, 9, 22, 23, 59, 0, 0))
        );
    }

    /// Reference outputs. Every right-hand side below came from running the **Python**
    /// `engine.approvals._plain` against `yaml.safe_load`, not from the Rust.
    #[test]
    fn plain_reproduces_pyyamls_timestamp_resolution() {
        for (raw, want) in [
            ("2026-10-09T13:00:00-05:00", "2026-10-09T13:00"),
            ("2026-10-09 13:00:00 -5", "2026-10-09T13:00"),
            ("2026-1-5T3:04:05.123Z", "2026-01-05T03:04"),
            ("2026-10-9t3:00:00", "2026-10-09T03:00"),
            ("2026-10-09", "2026-10-09"),
            // No seconds → PyYAML leaves it a string, so `_plain` passes it through.
            ("2026-10-09T13:00", "2026-10-09T13:00"),
            ("not a date", "not a date"),
        ] {
            assert_eq!(plain(&v(raw)), want, "plain({raw:?})");
        }
    }

    #[test]
    fn as_note_datetime_flattens_an_aware_timestamp_to_the_vaults_wall_clock() {
        // The comparison in `_amendment_block`'s same-day branch is a bare `<` between these
        // two. In Python one is aware and one is naive, and `<` on that pair raises TypeError —
        // which loses today.md. Both must land naive, at the stated hour.
        assert_eq!(
            as_note_datetime(Some(&v("2026-10-09T13:00:00-05:00"))),
            Some(DateTime::constant(2026, 10, 9, 13, 0, 0, 0))
        );
        assert_eq!(
            as_note_datetime(Some(&v("2026-10-09T09:00:00"))),
            Some(DateTime::constant(2026, 10, 9, 9, 0, 0, 0))
        );
    }

    #[test]
    fn as_note_datetime_returns_none_rather_than_raising_on_junk() {
        assert_eq!(as_note_datetime(Some(&v("not a date"))), None);
        assert_eq!(as_note_datetime(Some(&Value::Null)), None);
        assert_eq!(as_note_datetime(None), None);
    }

    #[test]
    fn amend_target_must_sit_strictly_inside_tasks_or_courses() {
        let root = std::env::temp_dir().join(format!("qo-appr-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&root);
        std::fs::create_dir_all(root.join("tasks")).unwrap();
        std::fs::create_dir_all(root.join("courses")).unwrap();

        assert!(resolve_amend_target(&root, Some(&v("tasks/ph-106.md"))).is_some());
        assert!(resolve_amend_target(&root, Some(&v("courses/gn-103.md"))).is_some());
        // The folder itself is not a target: Python's `parent in target.parents` is strict.
        assert!(resolve_amend_target(&root, Some(&v("tasks"))).is_none());
        assert!(resolve_amend_target(&root, Some(&v("../outside.md"))).is_none());
        assert!(resolve_amend_target(&root, Some(&v("approvals/x.md"))).is_none());
        assert!(resolve_amend_target(&root, None).is_none());
        let _ = std::fs::remove_dir_all(&root);
    }

    // -----------------------------------------------------------------------------------------
    // Scaffolding. Mirrors `tests/test_approvals.py`'s module-level fixtures so a Python test and
    // its Rust counterpart start from byte-identical notes.
    // -----------------------------------------------------------------------------------------

    use std::sync::atomic::{AtomicU64, Ordering};

    static SEQ: AtomicU64 = AtomicU64::new(0);

    const TODAY: Date = Date::constant(2026, 8, 20);

    fn now() -> DateTime {
        DateTime::constant(2026, 8, 20, 12, 0, 0, 0)
    }

    fn vault() -> PathBuf {
        let n = SEQ.fetch_add(1, Ordering::SeqCst);
        let dir = std::env::temp_dir()
            .join(format!("qo-approvals-{}-{n}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        dir
    }

    fn proposal(vault: &Path, name: &str, front: &str, body: &str) -> PathBuf {
        let folder = vault.join("approvals");
        std::fs::create_dir_all(&folder).unwrap();
        let path = folder.join(name);
        pystr::write_text(&path, &format!("---\n{front}\n---\n\n{body}")).unwrap();
        path
    }

    fn with_task(vault: &Path, name: &str, front: &str) -> PathBuf {
        let folder = vault.join("tasks");
        std::fs::create_dir_all(&folder).unwrap();
        let path = folder.join(name);
        pystr::write_text(&path, &format!("---\n{front}\n---\n\n")).unwrap();
        path
    }

    /// `vault_with_task(tmp_path)` — the default target every amendment test aims at.
    fn task_t(vault: &Path) -> PathBuf {
        with_task(vault, "t.md", "title: T\ndue: 2026-10-09T13:00")
    }

    const PENDING: &str = "type: approval\nkind: task\ntitle: Study group\nstatus: pending\n\
                           source_uid: \"gmail:abc\"\nproposed_at: 2026-08-18\n\
                           expires: 2026-08-25\nsnooze_until: null\ncreated_by: gmail";

    const AMEND: &str = "type: approval\nkind: amend\ntarget: tasks/t.md\ntitle: Exam moved\n\
                         status: pending\nurgency: increases\n\
                         source_uid: \"gmail:abc\"\nproposed_at: 2026-08-18\n\
                         expires: 2026-08-25\nsnooze_until: null\ncreated_by: gmail\n\
                         changes:\n  due:\n    from: \"2026-10-09T13:00\"\n    to: \"2026-10-02T13:00\"";

    const TASK_PAYLOAD: &str = "---\ntitle: Study group kickoff\ncourse: cs-100\ndomain: school\n\
                                due: 2026-08-25T18:00\neffort_hours: 1.0\neffort_confidence: low\n\
                                effort_source: inferred\nimportance: 3\n\
                                importance_reason: \"attendance helps\"\nstatus: active\n\
                                progress: 0\ncreated_by: gmail\nsource_uid: \"gmail:abc\"\n---\n\n\
                                Bring chapter 2 questions.\n";

    fn approved_task_body() -> String {
        format!(
            "**Why proposed:** borderline.\n\n```task\n{TASK_PAYLOAD}```\n\n\
             ```meta-bind-button\nlabel: Approve\n```\n"
        )
    }

    fn run(vault: &Path) -> ApprovalsResult {
        run_at(vault, TODAY, now())
    }

    fn run_at(vault: &Path, today: Date, at: DateTime) -> ApprovalsResult {
        let ctx = default_ctx();
        let mut journal = Journal::new(vault);
        process_approvals(vault, today, at, &ctx, &mut journal)
    }

    fn apply(vault: &Path, meta: &Mapping) -> Option<String> {
        let ctx = default_ctx();
        let mut journal = Journal::new(vault);
        apply_amendment(vault, meta, &ctx, &mut journal).expect("apply_amendment must not fail")
    }

    fn defer(vault: &Path, today: Date, budget: i64) -> Vec<String> {
        let ctx = default_ctx();
        let mut journal = Journal::new(vault);
        defer_over_budget(vault, today, budget, &ctx, &mut journal)
    }

    fn yaml(source: &str) -> Mapping {
        serde_yaml_ng::from_str(source).expect("test fixture must be valid YAML")
    }

    fn read(path: &Path) -> String {
        pystr::read_text(path).unwrap()
    }

    /// Parse a note's frontmatter exactly as production does, failing loudly.
    ///
    /// Deliberately `models::split_frontmatter` and not a line-anchored search of our own: a test
    /// parser kinder than the production parser cannot see the corruptions that matter.
    fn front(path: &Path) -> Mapping {
        let text = read(path);
        assert!(text.starts_with("---\n"), "no opening delimiter in {}", path.display());
        split_frontmatter(&text).expect("frontmatter must parse").0
    }

    fn field(path: &Path, key: &str) -> String {
        crate::yaml::get(&front(path), key).map(python_str).unwrap_or_default()
    }

    fn journal_records(vault: &Path) -> Vec<crate::ledger::Record> {
        Journal::new(vault).read(None, None)
    }

    fn exists(vault: &Path, rel: &str) -> bool {
        vault.join(rel).exists()
    }

    // -----------------------------------------------------------------------------------------
    // process_approvals — counting, ageing and the state machine
    // -----------------------------------------------------------------------------------------

    #[test]
    fn a_missing_approvals_folder_is_a_no_op_and_creates_nothing() {
        let v = vault();
        let result = run(&v);
        assert_eq!(result.pending, 0);
        assert!(!result.acted());
        assert!(!exists(&v, "approvals"));
    }

    #[test]
    fn pending_proposals_are_counted_and_aged_from_the_oldest() {
        let v = vault();
        proposal(&v, "task-a.md", PENDING, "");
        proposal(
            &v,
            "task-b.md",
            &PENDING.replace("proposed_at: 2026-08-18", "proposed_at: 2026-08-20"),
            "",
        );
        let result = run(&v);
        assert_eq!(result.pending, 2);
        assert_eq!(result.oldest_pending_days, 2);
    }

    #[test]
    fn a_future_snooze_is_neither_counted_nor_touched() {
        let v = vault();
        let front_text = PENDING
            .replace("status: pending", "status: snoozed")
            .replace("snooze_until: null", "snooze_until: 2026-08-22");
        let path = proposal(&v, "task-a.md", &front_text, "");
        let result = run(&v);
        assert_eq!(result.pending, 0);
        assert!(result.woken.is_empty());
        assert!(read(&path).contains("status: snoozed"));
    }

    #[test]
    fn a_snooze_due_today_wakes_to_pending_and_is_counted_in_the_same_pass() {
        let v = vault();
        let front_text = PENDING
            .replace("status: pending", "status: snoozed")
            .replace("snooze_until: null", "snooze_until: 2026-08-20");
        let path = proposal(&v, "task-a.md", &front_text, "");
        let result = run(&v);
        assert_eq!(result.woken, vec!["task-a".to_string()]);
        assert_eq!(result.pending, 1);
        assert!(read(&path).contains("status: pending"));
    }

    #[test]
    fn a_snooze_without_a_usable_date_warns_and_stays_snoozed() {
        // Two faults, two distinct warnings, and in both cases the note is left alone: an absent
        // `snooze_until` and an unparseable one.
        for (replacement, want) in [
            ("snooze_until: null", "snoozed without snooze_until: task-a.md"),
            ("snooze_until: whenever", "bad snooze_until: task-a.md"),
        ] {
            let v = vault();
            let front_text = PENDING
                .replace("status: pending", "status: snoozed")
                .replace("snooze_until: null", replacement);
            let path = proposal(&v, "task-a.md", &front_text, "");
            let result = run(&v);
            assert_eq!(result.warnings, vec![want.to_string()]);
            assert!(result.woken.is_empty());
            assert!(read(&path).contains("status: snoozed"));
        }
    }

    #[test]
    fn rejected_and_cloud_executed_proposals_settle_into_archive() {
        let v = vault();
        proposal(&v, "task-a.md", &PENDING.replace("status: pending", "status: rejected"), "");
        let executed = PENDING
            .replace("kind: task", "kind: calendar-event")
            .replace("status: pending", "status: executed")
            + "\nevent_id: \"evt123\"\nexecuted_at: \"2026-08-20 08:05\"";
        proposal(&v, "calendar-event-a.md", &executed, "");

        let result = run(&v);
        assert_eq!(result.rejected, vec!["task-a".to_string()]);
        assert_eq!(result.executed, vec!["calendar-event-a".to_string()]);
        assert!(!exists(&v, "approvals/task-a.md"));
        assert!(exists(&v, "archive/task-a.md"));
        assert!(exists(&v, "archive/calendar-event-a.md"));
    }

    #[test]
    fn expiry_stamps_the_status_before_archiving_and_is_exclusive_of_today() {
        let v = vault();
        proposal(&v, "task-a.md", &PENDING.replace("expires: 2026-08-25", "expires: 2026-08-19"), "");
        let result = run(&v);
        assert_eq!(result.expired, vec!["task-a".to_string()]);
        assert!(read(&v.join("archive").join("task-a.md")).contains("status: expired"));

        // `expires == today` is NOT expired: the comparison is `expires < today`.
        let v = vault();
        proposal(&v, "task-a.md", &PENDING.replace("expires: 2026-08-25", "expires: 2026-08-20"), "");
        let result = run(&v);
        assert!(result.expired.is_empty());
        assert_eq!(result.pending, 1);
    }

    #[test]
    fn an_archive_collision_takes_a_numeric_suffix_and_never_clobbers() {
        let v = vault();
        std::fs::create_dir_all(v.join("archive")).unwrap();
        pystr::write_text(&v.join("archive").join("task-a.md"), "old\n").unwrap();
        proposal(&v, "task-a.md", &PENDING.replace("status: pending", "status: rejected"), "");
        run(&v);
        assert!(exists(&v, "archive/task-a-2.md"));
        assert_eq!(read(&v.join("archive").join("task-a.md")), "old\n");
    }

    #[test]
    fn an_unreadable_or_foreign_note_warns_and_is_skipped() {
        let v = vault();
        proposal(&v, "task-bad.md", "title: [unclosed\nstatus: pending", "");
        assert_eq!(run(&v).warnings, vec!["unreadable: task-bad.md".to_string()]);

        let v = vault();
        proposal(&v, "stray.md", "title: Not an approval\nstatus: active", "");
        assert_eq!(run(&v).warnings, vec!["not an approval note: stray.md".to_string()]);
    }

    #[test]
    fn an_approved_calendar_event_counts_as_awaiting_and_is_left_for_the_cloud_executor() {
        let v = vault();
        let front_text = PENDING
            .replace("kind: task", "kind: calendar-event")
            .replace("status: pending", "status: approved");
        let path = proposal(&v, "calendar-event-a.md", &front_text, "");
        let result = run(&v);
        assert_eq!(result.awaiting_calendar, 1);
        assert!(path.exists());
    }

    #[test]
    fn an_unrecognised_status_or_kind_warns() {
        let v = vault();
        proposal(&v, "task-a.md", &PENDING.replace("status: pending", "status: maybe"), "");
        assert_eq!(run(&v).warnings, vec!["unknown status: task-a.md".to_string()]);

        let v = vault();
        let front_text = PENDING
            .replace("status: pending", "status: approved")
            .replace("kind: task", "kind: sideways");
        proposal(&v, "task-a.md", &front_text, "");
        assert_eq!(run(&v).warnings, vec!["unknown kind: task-a.md".to_string()]);
    }

    #[test]
    fn a_note_the_writer_cannot_edit_warns_instead_of_killing_the_pass() {
        // Closing delimiter `--- ` (trailing space): `split_frontmatter` tolerates it, but the
        // writer's `lines.index("---", 1)` cannot find a closing marker. The pass must warn and
        // carry on — the neighbouring proposal is still counted. (This test once used `---x`,
        // which only the old un-anchored split read; a `---x` line closes nothing now.)
        let v = vault();
        let front_text = PENDING
            .replace("status: pending", "status: snoozed")
            .replace("snooze_until: null", "snooze_until: 2026-08-20");
        let folder = v.join("approvals");
        std::fs::create_dir_all(&folder).unwrap();
        let bad = folder.join("task-bad-delim.md");
        pystr::write_text(&bad, &format!("---\n{front_text}\n--- \n\nbody")).unwrap();
        proposal(&v, "task-b.md", PENDING, "");

        let result = run(&v);
        assert_eq!(result.warnings, vec!["transition failed: task-bad-delim.md".to_string()]);
        assert!(bad.exists());
        assert_eq!(result.pending, 1);
    }

    #[test]
    fn a_proposal_with_an_unreadable_proposed_at_warns_rather_than_ageing_at_zero() {
        let v = vault();
        task_t(&v);
        proposal(&v, "amend-a.md", &AMEND.replace("proposed_at: 2026-08-18", "proposed_at: next Tuesday"), "");
        let result = run(&v);
        assert!(result.warnings.iter().any(|w| w.contains("proposed_at")), "{:?}", result.warnings);
    }

    // -----------------------------------------------------------------------------------------
    // Materialisation
    // -----------------------------------------------------------------------------------------

    #[test]
    fn an_approved_task_is_materialised_verbatim_apart_from_the_minted_id() {
        let v = vault();
        proposal(
            &v,
            "task-study-group.md",
            &PENDING.replace("status: pending", "status: approved"),
            &approved_task_body(),
        );
        let result = run(&v);
        assert_eq!(result.executed, vec!["study-group".to_string()]);

        let created = read(&v.join("tasks").join("study-group.md"));
        let id = field(&v.join("tasks").join("study-group.md"), "id");
        assert!(crate::ids::is_id(&id), "materialised note must carry a real id, got {id:?}");
        assert_eq!(created, TASK_PAYLOAD.replacen("---\n\nBring", &format!("id: {id}\n---\n\nBring"), 1));

        let archived = read(&v.join("archive").join("task-study-group.md"));
        assert!(archived.contains("status: executed"));
        assert!(archived.contains("executed_at: \"2026-08-20 12:00\""));
        assert!(!exists(&v, "approvals/task-study-group.md"));
    }

    #[test]
    fn the_materialised_slug_drops_a_task_prefix_and_never_overwrites() {
        // No `task-` prefix: the whole stem is the slug.
        let v = vault();
        proposal(
            &v,
            "followup.md",
            &PENDING.replace("status: pending", "status: approved"),
            &approved_task_body(),
        );
        assert_eq!(run(&v).executed, vec!["followup".to_string()]);
        assert!(exists(&v, "tasks/followup.md"));

        // An occupied slot takes the `-2` suffix rather than clobbering.
        let v = vault();
        std::fs::create_dir_all(v.join("tasks")).unwrap();
        pystr::write_text(&v.join("tasks").join("study-group.md"), "existing\n").unwrap();
        proposal(
            &v,
            "task-study-group.md",
            &PENDING.replace("status: pending", "status: approved"),
            &approved_task_body(),
        );
        assert_eq!(run(&v).executed, vec!["study-group-2".to_string()]);
        assert_eq!(read(&v.join("tasks").join("study-group.md")), "existing\n");
        assert!(exists(&v, "tasks/study-group-2.md"));
    }

    #[test]
    fn a_missing_or_frontmatterless_payload_warns_and_leaves_the_proposal_approved() {
        let v = vault();
        let path = proposal(
            &v,
            "task-a.md",
            &PENDING.replace("status: pending", "status: approved"),
            "no fence here\n",
        );
        let result = run(&v);
        assert_eq!(result.warnings, vec!["missing task payload: task-a.md".to_string()]);
        assert!(result.executed.is_empty());
        assert!(read(&path).contains("status: approved"));

        // A fence whose content has no frontmatter settles the same way — the old writer minted a
        // note `load_tasks` would silently skip.
        let v = vault();
        proposal(
            &v,
            "task-broken.md",
            "type: approval\nkind: task\ntitle: Broken\nstatus: approved\n\
             proposed_at: 2026-08-28\nexpires: null",
            "```task\nno frontmatter here\n```\n",
        );
        let result = run_at(&v, Date::constant(2026, 8, 29), DateTime::constant(2026, 8, 29, 8, 0, 0, 0));
        assert!(result.warnings.iter().any(|w| w.contains("missing task payload")));
        assert!(!result.warnings.iter().any(|w| w.contains("transition failed")));
        assert!(sorted_md(&v.join("tasks")).is_empty());
    }

    // -----------------------------------------------------------------------------------------
    // validate_amendment
    // -----------------------------------------------------------------------------------------

    #[test]
    fn amendable_fields_is_exactly_the_nine() {
        let mut got: Vec<&str> = AMENDABLE_FIELDS.to_vec();
        got.sort_unstable();
        assert_eq!(
            got,
            vec![
                "course",
                "domain",
                "due",
                "effort_confidence",
                "effort_hours",
                "importance",
                "importance_reason",
                "slice_hours",
                "status",
            ]
        );
    }

    #[test]
    fn a_valid_amendment_resolves_its_target() {
        let v = vault();
        let path = task_t(&v);
        let meta = yaml("target: tasks/t.md\nchanges:\n  due:\n    from: \"2026-10-09T13:00\"\n    to: \"2026-10-02T13:00\"\n");
        assert_eq!(validate_amendment(&v, &meta), Ok(resolve_lenient(&path)));

        // An append with no changes is a complete amendment on its own.
        let meta = yaml("target: tasks/t.md\nappend:\n  - a note\n");
        assert!(validate_amendment(&v, &meta).is_ok());
    }

    #[test]
    fn the_target_must_exist_and_sit_inside_tasks_or_courses() {
        let v = vault();
        task_t(&v);
        std::fs::create_dir_all(v.join("config")).unwrap();
        pystr::write_text(&v.join("config").join("planning.yaml"), "x: 1").unwrap();
        let good = "changes:\n  due:\n    from: a\n    to: b\n";

        let cases: [(&str, &str); 4] = [
            (good, "missing target"),
            ("target: tasks/gone.md\nappend:\n  - x\n", "does not exist"),
            ("target: config/planning.yaml\nappend:\n  - x\n", "outside"),
            ("target: tasks/../../secrets.md\nappend:\n  - x\n", "outside"),
        ];
        for (source, want) in cases {
            let reason = validate_amendment(&v, &yaml(source)).unwrap_err();
            assert!(reason.contains(want), "{source:?} gave {reason:?}");
        }
    }

    #[test]
    fn the_change_set_must_be_a_map_of_amendable_fields_with_both_sides_present() {
        let v = vault();
        task_t(&v);
        let cases: [(&str, &str); 5] = [
            // `title` is deliberately outside the set; S1 moved course/domain/effort_confidence in.
            ("target: tasks/t.md\nchanges:\n  title:\n    from: a\n    to: b\n", "title"),
            ("target: tasks/t.md\n", "nothing to apply"),
            (
                "target: tasks/t.md\nchanges:\n  due:\n    from: \"2026-10-09T13:00\"\n    to: null\n",
                "from/to",
            ),
            ("target: tasks/t.md\nchanges:\n  due:\n    to: \"2026-10-02T13:00\"\n", "from/to"),
            ("target: tasks/t.md\nchanges:\n  - a\n", "changes must be a map and append a list"),
        ];
        for (source, want) in cases {
            let reason = validate_amendment(&v, &yaml(source)).unwrap_err();
            assert!(reason.contains(want), "{source:?} gave {reason:?}");
        }
    }

    // -----------------------------------------------------------------------------------------
    // apply_amendment — the happy paths
    // -----------------------------------------------------------------------------------------

    #[test]
    fn a_frontmatter_change_and_an_append_both_land() {
        let v = vault();
        let path = task_t(&v);
        let meta = yaml(
            "target: tasks/t.md\nchanges:\n  due:\n    from: \"2026-10-09T13:00\"\n    to: \"2026-10-02T13:00\"\n\
             append:\n  - location is Lloyd 38\n",
        );
        assert_eq!(apply(&v, &meta), None);
        let text = read(&path);
        assert!(text.contains("due: 2026-10-02T13:00"));
        assert!(text.contains("> location is Lloyd 38"));
        assert_eq!(field(&path, "due"), "2026-10-02T13:00");
    }

    #[test]
    fn equivalent_spellings_of_from_still_match() {
        // A bare date coerces to 23:59, so `from: 2026-10-09T23:59` matches `due: 2026-10-09`...
        let v = vault();
        with_task(&v, "t.md", "title: T\ndue: 2026-10-09");
        let meta = yaml("target: tasks/t.md\nchanges:\n  due:\n    from: \"2026-10-09T23:59\"\n    to: \"2026-10-02\"\n");
        assert_eq!(apply(&v, &meta), None);

        // ...and "4.0" matches an integer 4.
        let v = vault();
        with_task(&v, "t.md", "title: T\neffort_hours: 4");
        let meta = yaml("target: tasks/t.md\nchanges:\n  effort_hours:\n    from: \"4.0\"\n    to: \"6.0\"\n");
        assert_eq!(apply(&v, &meta), None);

        // ...and '3' matches 3.0, writing 4.0 back.
        let v = vault();
        let path = with_task(&v, "t.md", "title: T\neffort_hours: 3.0");
        let meta = yaml("target: tasks/t.md\nchanges:\n  effort_hours:\n    from: '3'\n    to: 4.0\n");
        assert_eq!(apply(&v, &meta), None);
        assert!(read(&path).contains("effort_hours: 4.0"));
    }

    #[test]
    fn every_legitimate_value_still_applies_through_the_real_write_path() {
        // The guard must refuse only values that change meaning in the round trip. Every spelling
        // here is one a real proposal produces.
        let cases: [(&str, &str, &str); 7] = [
            ("due", "2026-10-09T13:00", "2026-10-02T13:00"),
            ("due", "2026-10-09T13:00", "2026-10-02"),
            ("effort_hours", "2.0", "3.0"),
            ("effort_hours", "2.0", "3"),
            ("slice_hours", "2.0", "1.5"),
            ("importance", "3", "4"),
            ("status", "active", "done"),
        ];
        for (index, (name, old, new)) in cases.iter().enumerate() {
            let v = vault();
            let file = format!("t{index}.md");
            let path = with_task(&v, &file, &format!("title: T\n{name}: {old}"));
            let meta = yaml(&format!(
                "target: tasks/{file}\nchanges:\n  {name}:\n    from: \"{old}\"\n    to: \"{new}\"\n"
            ));
            assert_eq!(apply(&v, &meta), None, "{name} -> {new}");
            // The block still parses and the untouched key survived: proof the write landed as one
            // key, not two.
            assert_eq!(field(&path, "title"), "T", "{name} -> {new}");
        }
    }

    #[test]
    fn a_hostile_but_storable_importance_reason_round_trips() {
        // Everything a single line can throw at YAML *and still be storable*: colon-space, double
        // quotes, a backslash, a comment marker, flow-map braces. An inline `---` used to be on
        // this list and now belongs with the refusals.
        let hostile = "Exam moved: \"read ch 2\" \\ see #3 {not: a map}";
        let v = vault();
        let path = with_task(&v, "t.md", "title: T\ndue: 2026-10-09T13:00\nimportance_reason: initial");
        let mut spec = Mapping::new();
        spec.insert(Value::String("from".into()), Value::String("initial".into()));
        spec.insert(Value::String("to".into()), Value::String(hostile.into()));
        let mut changes = Mapping::new();
        changes.insert(Value::String("importance_reason".into()), Value::Mapping(spec));
        let mut meta = Mapping::new();
        meta.insert(Value::String("target".into()), Value::String("tasks/t.md".into()));
        meta.insert(Value::String("changes".into()), Value::Mapping(changes));

        assert_eq!(apply(&v, &meta), None);
        assert_eq!(field(&path, "importance_reason"), hostile);
        assert_eq!(field(&path, "title"), "T");
        assert_eq!(field(&path, "due"), "2026-10-09T13:00");
    }

    #[test]
    fn a_from_value_the_engine_cannot_read_is_still_a_legal_amendment() {
        // Only `to` is ever written, so only `to` is round-trip checked. Checking `from` would
        // refuse a legitimate amendment to a note that already holds an odd value.
        let v = vault();
        let path = with_task(&v, "t.md", "title: T\ndue: TBD");
        let meta = yaml("target: tasks/t.md\nchanges:\n  due:\n    from: TBD\n    to: \"2026-10-02T13:00\"\n");
        assert_eq!(apply(&v, &meta), None);
        assert_eq!(field(&path, "due"), "2026-10-02T13:00");
    }

    #[test]
    fn a_re_applied_amendment_is_a_no_op_rather_than_stale() {
        // If the target write succeeds but the executed stamp does not, the next pass sees `from`
        // no longer matching, calls it stale, and writes the proposal back to pending — so
        // today.md asks Quinn to re-approve work already done.
        let v = vault();
        let path = task_t(&v);
        let meta = yaml("target: tasks/t.md\nchanges:\n  due:\n    from: \"2026-10-09T13:00\"\n    to: \"2026-10-02T13:00\"\n");
        assert_eq!(apply(&v, &meta), None);
        assert_eq!(apply(&v, &meta), None, "re-applying must be a no-op, not stale");
        assert_eq!(field(&path, "due"), "2026-10-02T13:00");
    }

    #[test]
    fn an_amended_task_still_loads_into_ranking() {
        // The end-to-end property all the guards serve.
        let v = vault();
        with_task(&v, "t.md", "title: T\nstatus: active\ndue: 2026-10-09T13:00");
        let meta = yaml("target: tasks/t.md\nchanges:\n  due:\n    from: \"2026-10-09T13:00\"\n    to: \"2026-10-02T13:00\"\n");
        assert_eq!(apply(&v, &meta), None);
        let mut skipped = Vec::new();
        let tasks = crate::models::load_tasks(&v.join("tasks"), Some(&mut skipped));
        assert!(skipped.is_empty(), "{skipped:?}");
        assert_eq!(tasks.len(), 1);
        assert_eq!(tasks[0].due, Some(DateTime::constant(2026, 10, 2, 13, 0, 0, 0)));
    }

    // -----------------------------------------------------------------------------------------
    // apply_amendment — refusals that must not touch the vault
    // -----------------------------------------------------------------------------------------

    #[test]
    fn a_stale_from_refuses_and_writes_nothing_at_all() {
        let v = vault();
        let path = task_t(&v);
        let before = read(&path);
        let meta = yaml(
            "target: tasks/t.md\nchanges:\n  due:\n    from: \"2026-09-01T13:00\"\n    to: \"2026-10-02T13:00\"\n\
             append:\n  - should not be written\n",
        );
        let reason = apply(&v, &meta).expect("must refuse");
        assert!(reason.contains("stale"), "{reason:?}");
        assert_eq!(read(&path), before, "not even the append may land");
    }

    #[test]
    fn a_stacked_amendment_is_stale_against_the_value_the_first_one_wrote() {
        let v = vault();
        let path = task_t(&v);
        let first = yaml("target: tasks/t.md\nchanges:\n  due:\n    from: \"2026-10-09T13:00\"\n    to: \"2026-10-02T13:00\"\n");
        let second = yaml("target: tasks/t.md\nchanges:\n  due:\n    from: \"2026-10-09T13:00\"\n    to: \"2026-10-05T13:00\"\n");
        assert_eq!(apply(&v, &first), None);
        assert!(apply(&v, &second).unwrap().contains("stale"));
        assert!(read(&path).contains("due: 2026-10-02T13:00"));
    }

    #[test]
    fn the_stale_message_reads_plainly_and_never_prints_two_identical_values() {
        let v = vault();
        with_task(&v, "t.md", "title: T\ndue: 2026-10-20");
        let meta = yaml("target: tasks/t.md\nchanges:\n  due:\n    from: \"2026-10-09T13:00\"\n    to: \"2026-10-02T13:00\"\n");
        // Reference output: the exact line that lands in state/runner-log.md.
        assert_eq!(
            apply(&v, &meta),
            Some("stale amendment: due is 2026-10-20, expected 2026-10-09T13:00".to_string())
        );

        // If the comparison were finer-grained than the display, a genuine mismatch would render
        // as "due is X, expected X" — a refusal that loops forever asserting the two are the same.
        let v = vault();
        task_t(&v);
        let meta = yaml("target: tasks/t.md\nchanges:\n  due:\n    from: 2026-10-09T13:00:30\n    to: \"2026-10-02T13:00\"\n");
        if let Some(reason) = apply(&v, &meta) {
            let (before, after) = reason.split_once(", expected ").expect("shape");
            assert_ne!(before.rsplit(" is ").next().unwrap(), after, "{reason}");
        }
    }

    #[test]
    fn a_boolean_or_fractional_from_never_satisfies_a_numeric_field() {
        // `float(True) == 1.0`, so an unguarded YAML `true` satisfied the staleness check against
        // `effort_hours: 1.0` — the one guard between a stale proposal and a clobbered hand edit.
        let v = vault();
        let path = with_task(&v, "t.md", "title: T\neffort_hours: 1.0");
        let meta = yaml("target: tasks/t.md\nchanges:\n  effort_hours:\n    from: true\n    to: 2.0\n");
        assert!(apply(&v, &meta).unwrap().contains("stale"));
        assert!(read(&path).contains("effort_hours: 1.0"));

        // `int(3.9) == 3`, so a proposal claiming importance was 3.9 passed against a note saying 3.
        let v = vault();
        let path = with_task(&v, "t.md", "title: T\nimportance: 3");
        let meta = yaml("target: tasks/t.md\nchanges:\n  importance:\n    from: 3.9\n    to: 5\n");
        assert!(apply(&v, &meta).unwrap().contains("stale"));
        assert!(read(&path).contains("importance: 3"));
    }

    #[test]
    fn a_newline_anywhere_in_from_or_to_is_refused_before_the_writer_sees_it() {
        // Reference outputs: these three strings are the whole refusal, field and side included.
        let cases: [(&str, &str, &str); 4] = [
            (
                "title: T\ndue: 2026-10-09T13:00\nimportance_reason: initial",
                "changes:\n  importance_reason:\n    from: initial\n    to: \"a\\n---\\nb\"\n",
                "importance_reason: to must be a single line",
            ),
            (
                "title: T\ndue: 2026-10-09T13:00",
                "changes:\n  due:\n    from: \"2026-10-09T13:00\"\n    to: \"2026-10-02\\ntitle: HACKED\"\n",
                "due: to must be a single line",
            ),
            (
                "title: T\ndue: 2026-10-09T13:00",
                "changes:\n  due:\n    from: \"2026-10-09T13:00\\nx: y\"\n    to: \"2026-10-02T13:00\"\n",
                "due: from must be a single line",
            ),
            // A YAML block scalar is the natural way a model writes two sentences, so the refusal
            // has to survive the YAML round trip rather than only a hand-built map.
            (
                "title: T\ndue: 2026-10-09T13:00\nimportance_reason: initial",
                "changes:\n  importance_reason:\n    from: initial\n    to: |\n      Exam moved.\n      Room changed too.\n",
                "importance_reason: to must be a single line",
            ),
        ];
        for (note, changes, want) in cases {
            let v = vault();
            let path = with_task(&v, "t.md", note);
            let before = std::fs::read(&path).unwrap();
            let meta = yaml(&format!("target: tasks/t.md\n{changes}"));
            assert_eq!(apply(&v, &meta), Some(want.to_string()));
            assert_eq!(std::fs::read(&path).unwrap(), before, "{want}");
        }
    }

    #[test]
    fn a_collection_or_a_leading_marker_in_to_is_refused_with_its_own_reason() {
        let cases: [(&str, &str, &str); 2] = [
            (
                "title: T\ndue: 2026-10-09T13:00",
                "changes:\n  due:\n    from: \"2026-10-09T13:00\"\n    to:\n      - a\n      - b\n",
                "due: to must be a scalar",
            ),
            (
                "title: T\ndue: 2026-10-09T13:00\nimportance_reason: initial",
                "changes:\n  importance_reason:\n    from: initial\n    to: \"--- not a delimiter\"\n",
                "importance_reason: to must not start a frontmatter block",
            ),
        ];
        for (note, changes, want) in cases {
            let v = vault();
            let path = with_task(&v, "t.md", note);
            let before = std::fs::read(&path).unwrap();
            let meta = yaml(&format!("target: tasks/t.md\n{changes}"));
            assert_eq!(apply(&v, &meta), Some(want.to_string()));
            assert_eq!(std::fs::read(&path).unwrap(), before);
        }
    }

    #[test]
    fn a_value_that_survives_the_single_line_check_but_corrupts_the_write_is_refused() {
        // Each of these is a single line and legal YAML, and each destroys the note a different
        // way: `": "` turns one key into two; `[a, b]` reads back as a list; `# gone` reads back as
        // null. Only comparing the re-read value catches them.
        for to in ["moved: see email", "[a, b]", "# gone"] {
            let v = vault();
            let path = task_t(&v);
            let before = std::fs::read(&path).unwrap();
            let meta = yaml(&format!(
                "target: tasks/t.md\nchanges:\n  due:\n    from: \"2026-10-09T13:00\"\n    to: \"{to}\"\n"
            ));
            assert_eq!(apply(&v, &meta), Some("due: to would corrupt frontmatter".to_string()), "{to:?}");
            assert_eq!(std::fs::read(&path).unwrap(), before);
        }
    }

    #[test]
    fn an_inline_frontmatter_marker_is_written_whole_and_never_drops_a_neighbouring_key() {
        // `---` mid-value is not a leading `---`, so the single-line check passes and the value is
        // a legal plain scalar. The reader once split on an un-anchored `---`, truncating the block
        // at the inline marker and discarding every later key, so this was refused. The reader now
        // closes the block only on a whole `---` line: the text is legitimate and lands intact.
        let v = vault();
        let path = with_task(&v, "t.md", "title: T\ndue: 2026-10-09T13:00\nimportance_reason: initial\nimportance: 5");
        let meta = yaml("target: tasks/t.md\nchanges:\n  importance_reason:\n    from: initial\n    to: \"prof moved it --- see announcement\"\n");
        assert_eq!(apply(&v, &meta), None);
        let parsed = split_frontmatter(&read(&path)).unwrap().0;
        assert_eq!(
            crate::yaml::get(&parsed, "importance_reason").and_then(crate::yaml::text).as_deref(),
            Some("prof moved it --- see announcement")
        );
        assert_eq!(crate::yaml::opt_i64(crate::yaml::get(&parsed, "importance"), -1), 5);

        // A `due` with an inline marker is still refused — because it is not a date, nothing else.
        let v = vault();
        let path = with_task(&v, "t.md", "title: T\ndue: 2026-10-09T13:00\neffort_hours: 6.0\nimportance: 5\nprogress: 90");
        let before = std::fs::read(&path).unwrap();
        let meta = yaml("target: tasks/t.md\nchanges:\n  due:\n    from: \"2026-10-09T13:00\"\n    to: \"2026-10-05 --- tentative\"\n");
        assert_eq!(apply(&v, &meta), Some("due: to is not a date the engine can read".to_string()));
        assert_eq!(std::fs::read(&path).unwrap(), before);
    }

    #[test]
    fn a_block_scalar_target_value_is_refused_rather_than_orphaned() {
        // The writer replaces only the `key:` header line, orphaning the indented continuation
        // lines, which then parse as garbage or not at all.
        let v = vault();
        let path = with_task(
            &v,
            "t.md",
            "title: T\ndue: 2026-10-09T13:00\nimportance_reason: >\n  a long reason\n  spanning lines",
        );
        let before = std::fs::read(&path).unwrap();
        let meta = yaml(
            "target: tasks/t.md\nchanges:\n  importance_reason:\n    from: a long reason spanning lines\n    to: new reason\n",
        );
        assert!(apply(&v, &meta).is_some());
        assert_eq!(std::fs::read(&path).unwrap(), before);
    }

    #[test]
    fn a_note_the_writer_could_only_half_amend_is_refused_before_the_append_lands() {
        // `update_frontmatter_fields` fails on `--- ` (trailing space) AFTER `append_body_line` has
        // already written, leaving the target half amended and the proposal stuck at approved with
        // a retry that can never succeed. The dry run has to catch it before anything is written.
        let v = vault();
        let folder = v.join("tasks");
        std::fs::create_dir_all(&folder).unwrap();
        let path = folder.join("t.md");
        pystr::write_text(&path, "---\ntitle: T\ndue: 2026-10-09T13:00\n--- \n\nbody\n").unwrap();
        let before = std::fs::read(&path).unwrap();
        let meta = yaml(
            "target: tasks/t.md\nchanges:\n  due:\n    from: \"2026-10-09T13:00\"\n    to: \"2026-10-02T13:00\"\n\
             append:\n  - \"2026-08-20 (announcement): moved\"\n",
        );
        assert!(apply(&v, &meta).is_some());
        assert_eq!(std::fs::read(&path).unwrap(), before, "the append must not land");
    }

    #[test]
    fn a_target_with_malformed_or_non_mapping_frontmatter_refuses_without_writing() {
        for note in ["title: [unclosed\ndue: 2026-10-09T13:00", "- a\n- b"] {
            let v = vault();
            let folder = v.join("tasks");
            std::fs::create_dir_all(&folder).unwrap();
            let path = folder.join("t.md");
            pystr::write_text(&path, &format!("---\n{note}\n---\n\n")).unwrap();
            let before = std::fs::read(&path).unwrap();
            let meta = yaml("target: tasks/t.md\nchanges:\n  due:\n    from: \"2026-10-09T13:00\"\n    to: \"2026-10-02T13:00\"\n");
            let reason = apply(&v, &meta).expect("must refuse");
            assert!(reason.contains("malformed"), "{reason:?}");
            assert_eq!(std::fs::read(&path).unwrap(), before);
        }
    }

    #[test]
    fn a_duplicated_frontmatter_key_is_refused_rather_than_silently_no_op() {
        // PyYAML keeps the LAST duplicate; the writer rewrites the FIRST. So the from-check passes
        // against the last, the write lands on the first, and the deadline never moves — reported
        // as executed, with no way to detect it later because `from` still matches.
        let v = vault();
        let path = with_task(
            &v,
            "t.md",
            "title: T\ndue: 2026-10-09T13:00\neffort_hours: 1.0\ndue: 2026-11-01T13:00",
        );
        let before = std::fs::read(&path).unwrap();
        let meta = yaml("target: tasks/t.md\nchanges:\n  due:\n    from: \"2026-11-01T13:00\"\n    to: \"2026-10-02T13:00\"\n");
        assert!(apply(&v, &meta).is_some(), "a no-op amendment must not report success");
        assert_eq!(std::fs::read(&path).unwrap(), before);
    }

    #[test]
    fn a_value_stored_faithfully_but_unusable_by_its_reader_is_refused() {
        // Each of these round-trips perfectly and then disappears from Today: a capitalised status
        // fails `load_tasks`' `== "active"` filter with nothing skipped and nothing warned; a
        // trailing space does the same; an undateable `due` makes `Task::from_file` fail; and
        // "3 hours" is not a number.
        let cases: [(&str, &str, &str); 4] = [
            ("title: T\nstatus: active", "status", "Active"),
            ("title: T\nstatus: active", "status", "active "),
            ("title: T\ndue: 2026-10-09T13:00", "due", "TBD @ 5pm"),
            ("title: T\neffort_hours: 2.0", "effort_hours", "3 hours"),
        ];
        for (note, name, to) in cases {
            let v = vault();
            let path = with_task(&v, "t.md", note);
            let before = std::fs::read(&path).unwrap();
            let from = note.rsplit_once(": ").unwrap().1;
            let meta = yaml(&format!(
                "target: tasks/t.md\nchanges:\n  {name}:\n    from: \"{from}\"\n    to: \"{to}\"\n"
            ));
            assert!(apply(&v, &meta).is_some(), "{name} -> {to:?} must refuse");
            assert_eq!(std::fs::read(&path).unwrap(), before, "{name} -> {to:?}");
        }

        // A status the engine recognises is accepted...
        let v = vault();
        let path = with_task(&v, "t.md", "title: T\nstatus: active");
        let meta = yaml("target: tasks/t.md\nchanges:\n  status:\n    from: active\n    to: done\n");
        assert_eq!(apply(&v, &meta), None);
        assert_eq!(field(&path, "status"), "done");

        // ...and the same text that is nonsense as a `due` is perfectly legal as free text, because
        // `importance_reason` has no reader that must parse it.
        let v = vault();
        let path = with_task(&v, "t.md", "title: T\nimportance_reason: initial");
        let meta = yaml("target: tasks/t.md\nchanges:\n  importance_reason:\n    from: initial\n    to: \"TBD @ 5pm\"\n");
        assert_eq!(apply(&v, &meta), None);
        assert_eq!(field(&path, "importance_reason"), "TBD @ 5pm");
    }

    #[test]
    fn an_out_of_range_date_is_refused_recoverably_rather_than_escaping() {
        // PyYAML's timestamp constructor raises a *bare* `ValueError` for a bad month, which is not
        // a `YAMLError`. Escaping `validate_amendment` would leave the proposal `approved` forever,
        // never counted, escalated or expired — it would just re-WARN on every pass.
        let v = vault();
        task_t(&v);
        let meta = yaml("target: tasks/t.md\nchanges:\n  due:\n    from: \"2026-10-09T13:00\"\n    to: \"2026-13-45\"\n");
        let reason = apply(&v, &meta).expect("must refuse");
        assert!(is_recoverable_refusal(&reason), "{reason:?}");
    }

    #[test]
    fn a_seconds_precision_datetime_is_accepted_and_stored_at_minute_precision() {
        // The writer renders every datetime through `format_due`, which is minute-precision.
        // Refusing a value because the writer will normalise it describes a corruption that cannot
        // happen.
        let v = vault();
        let path = task_t(&v);
        let meta = yaml("target: tasks/t.md\nchanges:\n  due:\n    from: \"2026-10-09T13:00\"\n    to: 2026-10-02T13:00:30\n");
        assert_eq!(apply(&v, &meta), None);
        assert_eq!(field(&path, "due"), "2026-10-02T13:00");
    }

    #[test]
    fn a_yaml_datetime_is_written_canonically_and_a_yaml_date_stays_date_only() {
        // `str()` of a datetime spells it "2026-10-02 13:00:00", unlike every neighbouring note...
        let v = vault();
        let path = task_t(&v);
        let meta = yaml("target: tasks/t.md\nchanges:\n  due:\n    from: 2026-10-09T13:00:00\n    to: 2026-10-02T13:00:00\n");
        assert_eq!(apply(&v, &meta), None);
        assert!(read(&path).contains("due: 2026-10-02T13:00"));

        // ...and a date-only due means end of day, so it must stay date-only rather than T00:00.
        let v = vault();
        let path = task_t(&v);
        let meta = yaml("target: tasks/t.md\nchanges:\n  due:\n    from: 2026-10-09 13:00\n    to: 2026-10-02\n");
        assert_eq!(apply(&v, &meta), None);
        assert!(read(&path).contains("due: 2026-10-02\n"));
    }

    #[test]
    fn an_append_only_amendment_refuses_a_non_string_or_multi_line_entry() {
        for (append, want) in [
            ("append:\n  - 2026-08-20\n", "append entries must be strings"),
            ("append:\n  - \"a\\nb\"\n", "append line must be a single line"),
            ("append:\n  - \"--- x\"\n", "append line must not start a frontmatter block"),
        ] {
            let v = vault();
            let path = task_t(&v);
            let before = std::fs::read(&path).unwrap();
            let meta = yaml(&format!("target: tasks/t.md\n{append}"));
            assert_eq!(apply(&v, &meta), Some(want.to_string()));
            assert_eq!(std::fs::read(&path).unwrap(), before);
        }
    }

    #[test]
    fn a_bad_append_line_refuses_before_the_frontmatter_change_lands() {
        let v = vault();
        let path = task_t(&v);
        let before = read(&path);
        let meta = yaml(
            "target: tasks/t.md\nchanges:\n  due:\n    from: \"2026-10-09T13:00\"\n    to: \"2026-10-02T13:00\"\n\
             append:\n  - \"one\\ntwo\"\n",
        );
        assert!(apply(&v, &meta).is_some());
        assert_eq!(read(&path), before);
    }

    #[test]
    fn an_append_that_landed_before_a_failed_frontmatter_write_is_not_duplicated_on_retry() {
        // Stands in for the Python's monkeypatched `update_frontmatter_fields`. A block-style
        // `judgment:` makes `write_literals` fail (it guards on load) while `append_body` succeeds
        // (it does not), which is exactly the half-applied state the ordering exists to survive.
        let v = vault();
        let folder = v.join("tasks");
        std::fs::create_dir_all(&folder).unwrap();
        let path = folder.join("t.md");
        pystr::write_text(
            &path,
            "---\ntitle: T\ndue: 2026-10-09T13:00\njudgment:\n  migrated: true\n---\n\n",
        )
        .unwrap();
        let meta = yaml(
            "target: tasks/t.md\nchanges:\n  due:\n    from: \"2026-10-09T13:00\"\n    to: \"2026-10-02T13:00\"\n\
             append:\n  - \"2026-10-02 (announcement): moved up a week\"\n",
        );

        let ctx = default_ctx();
        let mut journal = Journal::new(&v);
        assert!(
            apply_amendment(&v, &meta, &ctx, &mut journal).is_err(),
            "the frontmatter write must fail for this test to mean anything"
        );
        let mid = read(&path);
        assert_eq!(mid.matches("> 2026-10-02 (announcement): moved up a week").count(), 1);
        assert!(mid.contains("due: 2026-10-09T13:00"), "frontmatter must be untouched");

        // Repair the note, then retry: the append is a no-op and the change completes.
        pystr::write_text(&path, &mid.replace("judgment:\n  migrated: true\n", "")).unwrap();
        assert_eq!(apply(&v, &meta), None);
        let after = read(&path);
        assert_eq!(after.matches("> 2026-10-02 (announcement): moved up a week").count(), 1);
        assert_eq!(field(&path, "due"), "2026-10-02T13:00");
    }

    // -----------------------------------------------------------------------------------------
    // Refusal routing (spec §4.3)
    // -----------------------------------------------------------------------------------------

    #[test]
    fn every_refusal_reason_routes_per_spec_and_leaves_the_vault_untouched() {
        // Driven for real, one refusal per row: "leave pending" == recoverable, "archive
        // unexecuted" == not. The five unrecoverable rows are the whole of
        // `UNRECOVERABLE_REFUSALS`; everything else stays pending, which is the safe default since
        // a pending proposal is visible, escalated and expires, where an archived one is gone.
        let v = vault();
        let path = task_t(&v);
        pystr::write_text(&v.join("tasks").join("bad.md"), "---\ntitle: [unclosed\n---\n\n").unwrap();
        let good = "changes:\n  due:\n    from: \"2026-10-09T13:00\"\n    to: \"2026-10-02T13:00\"\n";

        let cases: Vec<(String, bool)> = vec![
            ("".to_string(), false),                                       // missing target
            (format!("target: ../escape.md\n{good}"), false),              // target outside
            (format!("target: tasks/gone.md\n{good}"), false),             // target missing
            ("target: tasks/t.md\nchanges:\n  - a\n".to_string(), false),  // changes not a map
            ("target: tasks/t.md\n".to_string(), false),                   // nothing to apply
            // Recoverable: the fault is in the target, not the proposal, and repairing the note's
            // YAML by hand fixes it.
            (format!("target: tasks/bad.md\n{good}"), true),
            ("target: tasks/t.md\nchanges:\n  progress:\n    from: 1\n    to: 2\n".to_string(), true),
            ("target: tasks/t.md\nchanges:\n  due: not a map\n".to_string(), true),
            ("target: tasks/t.md\nchanges:\n  due:\n    from: \"2026-10-09T13:00\"\n".to_string(), true),
            ("target: tasks/t.md\nchanges:\n  due:\n    from: \"x\\ny\"\n    to: z\n".to_string(), true),
            ("target: tasks/t.md\nchanges:\n  due:\n    from: a\n    to: \"b\\nc\"\n".to_string(), true),
            ("target: tasks/t.md\nchanges:\n  due:\n    from: a\n    to:\n      - b\n      - c\n".to_string(), true),
            ("target: tasks/t.md\nchanges:\n  status:\n    from: active\n    to: \"--- x\"\n".to_string(), true),
            ("target: tasks/t.md\nchanges:\n  due:\n    from: \"2026-10-09T13:00\"\n    to: \"m: e\"\n".to_string(), true),
            ("target: tasks/t.md\nchanges:\n  due:\n    from: \"2026-01-01\"\n    to: \"2026-02-02\"\n".to_string(), true),
            ("target: tasks/t.md\nappend:\n  - 123\n".to_string(), true),
            ("target: tasks/t.md\nappend:\n  - \"a\\nb\"\n".to_string(), true),
            ("target: tasks/t.md\nappend:\n  - \"--- x\"\n".to_string(), true),
        ];
        assert_eq!(cases.len(), 18);
        for (source, recoverable) in cases {
            let meta = if source.is_empty() { Mapping::new() } else { yaml(&source) };
            let reason = apply(&v, &meta).unwrap_or_else(|| panic!("{source:?} must refuse"));
            assert_eq!(is_recoverable_refusal(&reason), recoverable, "{reason:?}");
        }
        assert!(read(&path).contains("due: 2026-10-09T13:00"), "no refusal may touch the vault");
    }

    // -----------------------------------------------------------------------------------------
    // derive_urgency — the escalation that puts ⚠ PENDING AMENDMENT into Must do
    // -----------------------------------------------------------------------------------------

    #[test]
    fn urgency_is_re_derived_from_the_change_set_and_errs_toward_increases() {
        // An earlier due overrides a `neutral` label...
        let neutral_earlier = yaml(
            "urgency: neutral\nchanges:\n  due:\n    from: \"2026-10-09T13:00\"\n    to: \"2026-10-02T13:00\"\n",
        );
        assert_eq!(derive_urgency(&neutral_earlier), "increases");

        // ...a status moving back to active does too...
        let reinstated = yaml("urgency: neutral\nchanges:\n  status:\n    from: done\n    to: active\n");
        assert_eq!(derive_urgency(&reinstated), "increases");

        // ...a later due keeps the stated label...
        let later = yaml(
            "urgency: decreases\nchanges:\n  due:\n    from: \"2026-10-09T13:00\"\n    to: \"2026-11-20T13:00\"\n",
        );
        assert_eq!(derive_urgency(&later), "decreases");

        // ...and an unrecognised or absent label falls back to neutral.
        assert_eq!(derive_urgency(&yaml("urgency: sideways\n")), "neutral");
        assert_eq!(derive_urgency(&Mapping::new()), "neutral");
    }

    #[test]
    fn a_mixed_timezone_due_change_reads_as_increases_instead_of_killing_the_pass() {
        // Both spellings are legal unquoted YAML timestamps, and PyYAML makes the first aware and
        // the second naive. Comparing them raises `TypeError`, which is NOT in
        // `process_approvals`' handler — so it escapes `cli.run`, which calls it before today.md is
        // written or any runner-log line is appended.
        let cross_day = yaml("changes:\n  due:\n    from: 2026-10-09T13:00:00-05:00\n    to: 2026-10-02T13:00:00\n");
        assert_eq!(derive_urgency(&cross_day), "increases");

        // The same-day branch compares full datetimes rather than dates, so it is the one place a
        // mixed pair reaches a bare `<`.
        let same_day = yaml("changes:\n  due:\n    from: 2026-10-09T13:00:00-05:00\n    to: 2026-10-09T09:00:00\n");
        assert_eq!(derive_urgency(&same_day), "increases");

        // ...and the whole pass survives it end to end.
        let v = vault();
        task_t(&v);
        let front_text = AMEND.replace(
            "changes:\n  due:\n    from: \"2026-10-09T13:00\"\n    to: \"2026-10-02T13:00\"",
            "changes:\n  due:\n    from: 2026-10-09T13:00:00-05:00\n    to: 2026-10-02T13:00:00",
        );
        proposal(&v, "amend-tz.md", &front_text, "");
        let result = run(&v);
        assert_eq!(result.pending, 1);
        assert_eq!(result.amendments_pending[0].urgency, "increases");
    }

    // -----------------------------------------------------------------------------------------
    // Amendments inside the pass
    // -----------------------------------------------------------------------------------------

    #[test]
    fn a_pending_amendment_is_collected_with_the_metadata_the_renderer_needs() {
        let v = vault();
        task_t(&v);
        proposal(&v, "amend-a.md", AMEND, "");
        let result = run(&v);
        assert_eq!(result.pending, 1);
        assert_eq!(result.amendments_pending.len(), 1);
        let entry = &result.amendments_pending[0];
        assert_eq!(entry.target, "tasks/t.md");
        assert_eq!(entry.title, "Exam moved");
        assert_eq!(entry.urgency, "increases");
        assert_eq!(entry.proposal, "amend-a");
        assert_eq!(entry.age_days, 2);
        assert!(entry.changes.contains_key(Value::String("due".into())));
    }

    #[test]
    fn a_snoozed_amendment_is_not_collected_and_a_non_map_change_set_becomes_empty() {
        let v = vault();
        task_t(&v);
        let front_text = AMEND
            .replace("status: pending", "status: snoozed")
            .replace("snooze_until: null", "snooze_until: 2026-08-25");
        proposal(&v, "amend-a.md", &front_text, "");
        assert!(run(&v).amendments_pending.is_empty());

        // A syntactically valid proposal can still carry a non-mapping `changes:`; the collected
        // entry must store an empty map rather than the raw value.
        let v = vault();
        task_t(&v);
        let front_text = AMEND.replace(
            "changes:\n  due:\n    from: \"2026-10-09T13:00\"\n    to: \"2026-10-02T13:00\"",
            "changes:\n  - a\n  - b",
        );
        proposal(&v, "amend-a.md", &front_text, "");
        let result = run(&v);
        assert_eq!(result.amendments_pending.len(), 1);
        assert!(result.amendments_pending[0].changes.is_empty());
    }

    #[test]
    fn an_approved_amendment_applies_stamps_and_archives() {
        let v = vault();
        let target = task_t(&v);
        proposal(&v, "amend-a.md", &AMEND.replace("status: pending", "status: approved"), "");
        let result = run(&v);
        assert_eq!(result.executed, vec!["amend-a".to_string()]);
        assert!(read(&target).contains("due: 2026-10-02T13:00"));
        let archived = v.join("archive").join("amend-a.md");
        assert!(archived.exists());
        assert!(!exists(&v, "approvals/amend-a.md"));
        let text = read(&archived);
        assert!(text.contains("status: executed"));
        assert!(text.contains("executed_at:"));
    }

    #[test]
    fn a_recoverable_refusal_is_written_back_to_pending_rather_than_left_approved() {
        // Left as `approved` it is never counted, never escalated into Must do, and never expires —
        // it re-WARNs on every pass forever.
        let v = vault();
        let target = with_task(&v, "t.md", "title: T\ndue: 2026-11-01T13:00");
        let path = proposal(&v, "amend-a.md", &AMEND.replace("status: pending", "status: approved"), "");
        let first = run(&v);
        assert!(first.warnings.iter().any(|w| w.contains("stale")), "{:?}", first.warnings);
        assert!(path.exists());
        assert_eq!(field(&path, "status"), "pending");
        assert!(read(&target).contains("due: 2026-11-01T13:00"));

        // ...and on the next pass it rejoins the pending population.
        let second = run(&v);
        assert_eq!(second.pending, 1);
        assert_eq!(
            second.amendments_pending.iter().map(|a| a.proposal.clone()).collect::<Vec<_>>(),
            vec!["amend-a".to_string()]
        );
        assert_eq!(second.amendments_pending[0].urgency, "increases");

        // ...and it eventually expires, exactly as an ordinary pending proposal does.
        let later = run_at(&v, Date::constant(2026, 8, 26), DateTime::constant(2026, 8, 26, 12, 0, 0, 0));
        assert_eq!(later.expired, vec!["amend-a".to_string()]);
        assert!(!path.exists());
    }

    #[test]
    fn other_recoverable_refusals_also_stay_pending_and_leave_the_target_alone() {
        // A malformed target: the condition is external to the proposal and fixable by hand.
        let v = vault();
        std::fs::create_dir_all(v.join("tasks")).unwrap();
        pystr::write_text(&v.join("tasks").join("t.md"), "---\ntitle: [unclosed\n---\n\n").unwrap();
        let path = proposal(&v, "amend-a.md", &AMEND.replace("status: pending", "status: approved"), "");
        let result = run(&v);
        assert!(result.warnings.iter().any(|w| w.contains("malformed")));
        assert!(path.exists(), "a fixable condition must not archive the proposal");
        assert!(!exists(&v, "archive/amend-a.md"));
        assert_eq!(field(&path, "status"), "pending");

        // A field outside AMENDABLE_FIELDS: the model can fix its own proposal.
        let v = vault();
        task_t(&v);
        let front_text = AMEND
            .replace("status: pending", "status: approved")
            .replace(
                "changes:\n  due:\n    from: \"2026-10-09T13:00\"\n    to: \"2026-10-02T13:00\"",
                "changes:\n  progress:\n    from: 0\n    to: 50",
            );
        let path = proposal(&v, "amend-a.md", &front_text, "");
        let result = run(&v);
        assert!(result.warnings.iter().any(|w| w.contains("field not amendable: progress")));
        assert!(path.exists());
        assert!(!exists(&v, "archive/amend-a.md"));
        assert_eq!(field(&path, "status"), "pending");

        // A non-string `append` entry — an unquoted YAML date parses to a non-str type.
        let v = vault();
        let target = task_t(&v);
        let front_text = "type: approval\nkind: amend\ntarget: tasks/t.md\ntitle: Bad append\n\
                          status: approved\nurgency: neutral\nproposed_at: 2026-08-18\n\
                          expires: 2026-08-25\nsnooze_until: null\nappend:\n  - 2026-08-20";
        let path = proposal(&v, "amend-a.md", front_text, "");
        let result = run(&v);
        assert!(result.warnings.iter().any(|w| w.contains("append entries must be strings")));
        assert!(path.exists());
        assert!(!exists(&v, "archive/amend-a.md"));
        assert!(read(&target).contains("due: 2026-10-09T13:00"));
    }

    #[test]
    fn an_unrecoverable_refusal_is_stamped_refused_and_archived() {
        // archive/ is where settled things go. A note left saying `approved` is the only archived
        // state that misrepresents itself, and anything that moves it back re-attempts the change.
        let v = vault();
        std::fs::create_dir_all(v.join("tasks")).unwrap();
        proposal(&v, "amend-a.md", &AMEND.replace("status: pending", "status: approved"), "");
        let result = run(&v);
        assert!(result.warnings.iter().any(|w| w.contains("does not exist")));
        assert!(exists(&v, "archive/amend-a.md"));

        let v = vault();
        task_t(&v);
        let front_text = AMEND
            .replace("status: pending", "status: approved")
            .replace("target: tasks/t.md", "target: config/planning.yaml");
        proposal(&v, "amend-a.md", &front_text, "");
        run(&v);
        let archived = v.join("archive").join("amend-a.md");
        assert!(archived.exists());
        assert_eq!(field(&archived, "status"), "refused");
    }

    #[test]
    fn no_warning_the_pass_emits_ever_contains_a_raw_newline() {
        // Warnings are interpolated into state/runner-log.md. A newline there produces an orphan
        // line with no `- <date> <runner> <status>` shape, which the retention logic cannot
        // classify — so it is kept forever, and merge=union preserves it across every future merge.
        let v = vault();
        with_task(&v, "t.md", "title: T\nimportance_reason: \"one\"");
        let front_text = AMEND.replace("status: pending", "status: approved").replace(
            "changes:\n  due:\n    from: \"2026-10-09T13:00\"\n    to: \"2026-10-02T13:00\"",
            "changes:\n  importance_reason:\n    from: \"two\"\n    to: \"three\"",
        );
        proposal(&v, "amend-a.md", &front_text, "");
        let result = run(&v);
        assert!(!result.warnings.is_empty());
        for warning in &result.warnings {
            assert!(!warning.contains('\n') && !warning.contains('\r'), "{warning:?}");
        }
    }

    // -----------------------------------------------------------------------------------------
    // The journal
    // -----------------------------------------------------------------------------------------

    #[test]
    fn expiry_journals_the_stamp_before_the_archive_move() {
        let v = vault();
        proposal(
            &v,
            "task-old.md",
            "type: approval\nkind: task\ntitle: Old\nstatus: pending\n\
             proposed_at: 2026-08-01\nexpires: 2026-08-10",
            "",
        );
        run_at(&v, Date::constant(2026, 8, 29), DateTime::constant(2026, 8, 29, 8, 0, 0, 0));
        let records = journal_records(&v);
        let ops: Vec<(String, Option<String>)> = records
            .iter()
            .map(|r| {
                (
                    r.get("op").and_then(|v| v.as_str()).unwrap_or_default().to_string(),
                    r.get("field").and_then(|v| v.as_str()).map(str::to_string),
                )
            })
            .collect();
        assert_eq!(
            ops,
            vec![("set".to_string(), Some("status".to_string())), ("delete".to_string(), None)]
        );
        assert_eq!(records[0].get("new").and_then(|v| v.as_str()), Some("expired"));
        assert_eq!(records[0].get("actor").and_then(|v| v.as_str()), Some("agent:approvals"));
    }

    #[test]
    fn a_materialised_task_is_a_journalled_create_carrying_its_id() {
        let v = vault();
        proposal(
            &v,
            "task-new.md",
            "type: approval\nkind: task\ntitle: New\nstatus: approved\n\
             proposed_at: 2026-08-28\nexpires: null",
            "```task\n---\ntitle: New task\nstatus: active\nprogress: 0\n---\n\nbody\n```\n",
        );
        run_at(&v, Date::constant(2026, 8, 29), DateTime::constant(2026, 8, 29, 8, 0, 0, 0));
        let creates: Vec<_> = journal_records(&v)
            .into_iter()
            .filter(|r| r.get("op").and_then(|v| v.as_str()) == Some("create"))
            .collect();
        assert_eq!(creates.len(), 1);
        assert_eq!(creates[0].get("path").and_then(|v| v.as_str()), Some("tasks/new.md"));
        let id = creates[0].get("new").and_then(|v| v.get("id")).and_then(|v| v.as_str()).unwrap();
        assert!(id.starts_with("task_"));
        assert_eq!(field(&v.join("tasks").join("new.md"), "id"), id);
    }

    #[test]
    fn deferring_a_proposal_journals_all_three_fields_it_rewrites() {
        let v = vault();
        proposal(
            &v,
            "task-a.md",
            "type: approval\nkind: task\ntitle: A\nstatus: pending\n\
             proposed_at: 2026-08-29\nexpires: null",
            "",
        );
        defer(&v, Date::constant(2026, 8, 29), 0);
        let mut fields: Vec<String> = journal_records(&v)
            .iter()
            .filter_map(|r| r.get("field").and_then(|v| v.as_str()).map(str::to_string))
            .collect();
        fields.sort();
        assert_eq!(fields, vec!["proposed_at", "snooze_until", "status"]);
    }

    // -----------------------------------------------------------------------------------------
    // The daily budget
    // -----------------------------------------------------------------------------------------

    const BUDGET_TODAY: Date = Date::constant(2026, 8, 26);

    fn note(vault: &Path, folder: &str, name: &str, front: &str) -> PathBuf {
        let folder = vault.join(folder);
        std::fs::create_dir_all(&folder).unwrap();
        let path = folder.join(name);
        pystr::write_text(&path, &format!("---\n{front}\n---\n\nbody\n")).unwrap();
        path
    }

    fn task_front(proposed_at: &str, status: &str) -> String {
        format!(
            "type: approval\nkind: task\ntitle: t\nstatus: {status}\n\
             proposed_at: {proposed_at}\nexpires: null\n"
        )
    }

    fn dated_front(proposed_at: &str, expires: &str) -> String {
        format!(
            "type: approval\nkind: task\ntitle: t\nstatus: pending\n\
             proposed_at: {proposed_at}\nexpires: {expires}\nsnooze_until: null\n"
        )
    }

    fn digest_front(proposed_at: &str, count: usize, status: &str) -> String {
        let events: Vec<String> = (0..count).map(|i| format!("  - uid: e{i}")).collect();
        format!(
            "type: approval\nkind: events-digest\ntitle: d\nstatus: {status}\n\
             proposed_at: {proposed_at}\nexpires: null\nevents:\n{}\n",
            events.join("\n")
        )
    }

    #[test]
    fn count_proposals_created_counts_today_across_approvals_and_archive() {
        let v = vault();
        note(&v, "approvals", "task-a.md", &task_front("2026-08-26", "pending"));
        note(&v, "approvals", "task-b.md", &task_front("2026-08-26", "pending"));
        note(&v, "approvals", "task-old.md", &task_front("2026-08-25", "pending"));
        // Approving a proposal moves it to archive/. Counting only approvals/ would hand back
        // budget the moment Quinn acts on something.
        note(&v, "archive", "task-done.md", &task_front("2026-08-26", "approved"));
        // ...and neither a non-approval nor an unreadable note may contribute.
        note(&v, "approvals", "stray.md", "type: something\nproposed_at: 2026-08-26\n");
        pystr::write_text(&v.join("approvals").join("broken.md"), "---\n: : :\n---\n").unwrap();

        assert_eq!(count_proposals_created(&v, BUDGET_TODAY), 3);
        assert_eq!(count_proposals_created(&vault(), BUDGET_TODAY), 0, "missing folders are zero");
    }

    #[test]
    fn a_digest_counts_as_its_event_count_not_one() {
        // The `events-digest` arm of `proposal_weight` stays forever: a digest is a batch UI, so
        // counting it as 1 would let a single digest smuggle a whole day's ceiling past the budget.
        let v = vault();
        note(
            &v,
            "approvals",
            "events-digest-2026-08-26.md",
            "type: approval\nkind: events-digest\ntitle: d\nstatus: pending\n\
             proposed_at: 2026-08-26\nexpires: null\nevents:\n  - uid: e1\n  - uid: e2\n  - uid: e3\n",
        );
        assert_eq!(count_proposals_created(&v, BUDGET_TODAY), 3);

        // A digest with no payload weighs nothing rather than 1.
        let v = vault();
        note(
            &v,
            "approvals",
            "events-digest-2026-08-26.md",
            "type: approval\nkind: events-digest\ntitle: d\nstatus: pending\n\
             proposed_at: 2026-08-26\nexpires: null\n",
        );
        assert_eq!(count_proposals_created(&v, BUDGET_TODAY), 0);
    }

    #[test]
    fn under_budget_defers_nothing_and_the_soonest_expiring_survives_a_contested_day() {
        let v = vault();
        note(&v, "approvals", "task-a.md", &dated_front("2026-08-26", "2026-09-01"));
        assert!(defer(&v, BUDGET_TODAY, 5).is_empty());

        let v = vault();
        note(&v, "approvals", "task-late.md", &dated_front("2026-08-26", "2026-09-30"));
        note(&v, "approvals", "task-soon.md", &dated_front("2026-08-26", "2026-08-27"));
        assert_eq!(defer(&v, BUDGET_TODAY, 1), vec!["task-late".to_string()]);
        assert_eq!(field(&v.join("approvals").join("task-soon.md"), "status"), "pending");
        let pushed = v.join("approvals").join("task-late.md");
        assert_eq!(field(&pushed, "status"), "snoozed");
        assert_eq!(field(&pushed, "snooze_until"), "2026-08-27");
        assert_eq!(field(&pushed, "proposed_at"), "2026-08-27");
    }

    #[test]
    fn an_undated_proposal_rolls_forward_before_a_dated_one() {
        let v = vault();
        note(&v, "approvals", "task-undated.md", &dated_front("2026-08-26", "null"));
        note(&v, "approvals", "task-dated.md", &dated_front("2026-08-26", "2026-09-30"));
        assert_eq!(defer(&v, BUDGET_TODAY, 1), vec!["task-undated".to_string()]);
    }

    #[test]
    fn a_deferred_proposal_stops_counting_against_today() {
        let v = vault();
        for i in 0..3 {
            note(&v, "approvals", &format!("task-{i}.md"), &dated_front("2026-08-26", &format!("2026-09-0{}", i + 1)));
        }
        defer(&v, BUDGET_TODAY, 2);
        assert_eq!(count_proposals_created(&v, BUDGET_TODAY), 2);
    }

    #[test]
    fn only_todays_pending_non_digest_proposals_are_eligible_to_defer() {
        let v = vault();
        note(&v, "approvals", "task-old.md", &dated_front("2026-08-25", "2026-08-27"));
        note(
            &v,
            "approvals",
            "events-digest-2026-08-26.md",
            "type: approval\nkind: events-digest\ntitle: d\nstatus: pending\n\
             proposed_at: 2026-08-26\nexpires: null\nevents:\n  - uid: e1\n",
        );
        // Quinn already approved this one this morning; deferring it would flip a made decision
        // back to snoozed, destroying it.
        note(&v, "approvals", "task-approved.md", &task_front("2026-08-26", "approved"));
        assert!(defer(&v, BUDGET_TODAY, 0).is_empty());
        assert_eq!(field(&v.join("approvals").join("task-approved.md"), "status"), "approved");
    }

    #[test]
    fn everything_created_today_shrinks_the_allowance_even_once_it_is_settled() {
        // A morning digest already settled (approved -> archived) still spent budget today.
        let v = vault();
        note(&v, "archive", "events-digest-2026-08-26.md", &digest_front("2026-08-26", 3, "executed"));
        for i in 0..4 {
            note(&v, "approvals", &format!("task-{i}.md"), &dated_front("2026-08-26", &format!("2026-09-0{}", i + 1)));
        }
        assert_eq!(defer(&v, BUDGET_TODAY, 5).len(), 2);

        // The same with no digest anywhere: five non-digest proposals approved and archived
        // earlier today, so 5 (archived) + 12 (kept) would otherwise be 17 against a budget of 15.
        let v = vault();
        for i in 0..5 {
            note(&v, "archive", &format!("done-{i}.md"), &task_front("2026-08-26", "approved"));
        }
        for i in 0..12 {
            note(&v, "approvals", &format!("task-{i}.md"), &dated_front("2026-08-26", &format!("2026-09-{:02}", i + 1)));
        }
        assert_eq!(defer(&v, BUDGET_TODAY, 15).len(), 2);
    }

    #[test]
    fn a_digest_at_or_above_the_budget_defers_every_non_digest_proposal() {
        let v = vault();
        note(&v, "approvals", "events-digest-2026-08-26.md", &digest_front("2026-08-26", 5, "pending"));
        note(&v, "approvals", "task-a.md", &dated_front("2026-08-26", "2026-09-01"));
        note(&v, "approvals", "task-b.md", &dated_front("2026-08-26", "2026-09-02"));
        let mut deferred = defer(&v, BUDGET_TODAY, 5);
        deferred.sort();
        assert_eq!(deferred, vec!["task-a".to_string(), "task-b".to_string()]);

        // Strictly ABOVE budget (7 > 5), not merely at it. Without the `max(0, …)` clamp the
        // allowance is -2 and `candidates[-2:]` defers only the last 2 of 3 — a silent under-defer.
        let v = vault();
        note(&v, "approvals", "events-digest-2026-08-26.md", &digest_front("2026-08-26", 7, "pending"));
        for (i, expires) in ["2026-09-01", "2026-09-02", "2026-09-03"].iter().enumerate() {
            note(&v, "approvals", &format!("task-{i}.md"), &dated_front("2026-08-26", expires));
        }
        let mut deferred = defer(&v, BUDGET_TODAY, 5);
        deferred.sort();
        assert_eq!(deferred, vec!["task-0".to_string(), "task-1".to_string(), "task-2".to_string()]);
    }

    #[test]
    fn deferring_rewrites_proposed_at_but_never_first_proposed_at() {
        // `proposed_at` is not a creation timestamp: it is the day the proposal actually charges.
        // `first_proposed_at` is set once and is the true age.
        let v = vault();
        let path = note(
            &v,
            "approvals",
            "task-a.md",
            "type: approval\nkind: task\ntitle: A\nstatus: pending\n\
             proposed_at: 2026-08-29\nfirst_proposed_at: 2026-08-20\nexpires: null\n",
        );
        assert_eq!(defer(&v, Date::constant(2026, 8, 29), 0), vec!["task-a".to_string()]);
        assert_eq!(field(&path, "proposed_at"), "2026-08-30");
        assert_eq!(field(&path, "first_proposed_at"), "2026-08-20");
    }

    #[test]
    fn age_reads_first_proposed_at_and_falls_back_to_proposed_at() {
        let v = vault();
        note(
            &v,
            "approvals",
            "task-a.md",
            "type: approval\nkind: task\ntitle: A\nstatus: pending\n\
             proposed_at: 2026-08-29\nfirst_proposed_at: 2026-08-20\nexpires: null\n",
        );
        // A pre-S1 note with no first_proposed_at falls back to proposed_at.
        note(
            &v,
            "approvals",
            "task-b.md",
            "type: approval\nkind: task\ntitle: B\nstatus: pending\n\
             proposed_at: 2026-08-27\nexpires: null\n",
        );
        let result = run_at(&v, Date::constant(2026, 8, 29), DateTime::constant(2026, 8, 29, 8, 0, 0, 0));
        assert_eq!(result.oldest_pending_days, 9);
    }

    // -----------------------------------------------------------------------------------------
    // Digests
    // -----------------------------------------------------------------------------------------

    const DIGEST_TODAY: Date = Date::constant(2026, 8, 21);

    fn digest_now() -> DateTime {
        DateTime::constant(2026, 8, 21, 9, 0, 0, 0)
    }

    fn digest(vault: &Path, status: &str, checked: &[&str]) -> PathBuf {
        let mut lines = Vec::new();
        for (uid, title) in [("engage:1", "AI Club Kickoff"), ("localist:2", "Co-op Info Session")] {
            let box_ = if checked.contains(&uid) { "x" } else { " " };
            lines.push(format!("- [{box_}] Thu 9/3 18:00 · {title} · `{uid}`"));
        }
        let front_text = format!(
            "type: approval\nkind: events-digest\ntitle: \"Events — 2026-08-21\"\n\
             status: {status}\nproposed_at: 2026-08-21\nexpires: 2026-09-03\n\
             snooze_until: null\ncreated_by: events\nevents:\n\
             \x20 - uid: \"engage:1\"\n    summary: \"AI Club Kickoff\"\n\
             \x20   start: 2026-09-03T18:00\n    end: 2026-09-03T19:30\n    location: \"Lloyd 38\"\n\
             \x20 - uid: \"localist:2\"\n    summary: \"Co-op Info Session\"\n\
             \x20   start: 2026-09-04T12:00\n    end: 2026-09-04T13:00\n    location: \"H.M. Comer 2004\""
        );
        proposal(vault, "events-digest-2026-08-21.md", &front_text, &format!("{}\n", lines.join("\n")))
    }

    fn calendar_notes(vault: &Path) -> Vec<String> {
        let mut names: Vec<String> = sorted_md(&vault.join("approvals"))
            .iter()
            .map(|p| name_of(p))
            .filter(|n| n.starts_with("calendar-event-"))
            .collect();
        names.sort();
        names
    }

    #[test]
    fn a_pending_digest_counts_its_events_rather_than_itself() {
        let v = vault();
        digest(&v, "pending", &["engage:1"]);
        let result = run_at(&v, DIGEST_TODAY, digest_now());
        assert_eq!(result.events_in_digest, 2);
        assert_eq!(result.pending, 0);
        assert!(result.warnings.is_empty());
    }

    #[test]
    fn an_approved_digest_writes_notes_only_for_checked_lines_and_declines_the_rest() {
        let v = vault();
        let path = digest(&v, "approved", &["engage:1"]);
        let result = run_at(&v, DIGEST_TODAY, digest_now());
        assert_eq!(calendar_notes(&v), vec!["calendar-event-ai-club-kickoff.md".to_string()]);

        let text = read(&v.join("approvals").join("calendar-event-ai-club-kickoff.md"));
        assert!(text.contains("kind: calendar-event"));
        assert!(text.contains("status: approved"));
        assert!(text.contains("AI Club Kickoff"));
        assert!(text.contains("Lloyd 38"));
        assert!(text.contains("start: 2026-09-03T18:00"));
        assert!(!result.executed.is_empty());
        assert!(crate::ids::is_id(&field(&v.join("approvals").join("calendar-event-ai-club-kickoff.md"), "id")));

        let ledger = crate::eventledger::load_ledger(&v, None);
        assert!(ledger["localist:2"].declined, "an unchecked line is declined");
        assert!(!ledger.contains_key("engage:1") || !ledger["engage:1"].declined);

        assert!(!path.exists(), "the digest is archived");
        assert!(exists(&v, "archive/events-digest-2026-08-21.md"));
    }

    #[test]
    fn a_rejected_or_expired_digest_declines_every_event() {
        let v = vault();
        digest(&v, "rejected", &["engage:1"]);
        run_at(&v, DIGEST_TODAY, digest_now());
        let ledger = crate::eventledger::load_ledger(&v, None);
        assert!(ledger["engage:1"].declined);
        assert!(ledger["localist:2"].declined);

        // Expiry declines them too — `expires: 2026-09-03`, run on 2026-09-04.
        let v = vault();
        let path = digest(&v, "pending", &["engage:1"]);
        run_at(&v, Date::constant(2026, 9, 4), DateTime::constant(2026, 9, 4, 9, 0, 0, 0));
        let ledger = crate::eventledger::load_ledger(&v, None);
        assert!(ledger["engage:1"].declined);
        assert!(ledger["localist:2"].declined);
        assert!(!path.exists());
    }

    #[test]
    fn a_digest_with_no_payload_or_a_uidless_entry_warns_without_crashing() {
        let v = vault();
        proposal(
            &v,
            "events-digest-2026-08-21.md",
            "type: approval\nkind: events-digest\nstatus: approved\n\
             proposed_at: 2026-08-21\nexpires: null\nsnooze_until: null",
            "empty\n",
        );
        let result = run_at(&v, DIGEST_TODAY, digest_now());
        assert_eq!(
            result.warnings,
            vec!["events-digest with no payload: events-digest-2026-08-21.md".to_string()]
        );

        let v = vault();
        proposal(
            &v,
            "events-digest-2026-08-21.md",
            "type: approval\nkind: events-digest\ntitle: \"Events\"\nstatus: approved\n\
             proposed_at: 2026-08-21\nexpires: 2026-09-03\nsnooze_until: null\ncreated_by: events\n\
             events:\n  - summary: \"No UID Here\"\n    start: 2026-09-03T18:00\n\
             \x20   end: 2026-09-03T19:30\n    location: \"TBD\"\n\
             \x20 - uid: \"engage:1\"\n    summary: \"AI Club Kickoff\"\n\
             \x20   start: 2026-09-03T18:00\n    end: 2026-09-03T19:30\n    location: \"Lloyd 38\"",
            "- [x] Thu 9/3 18:00 · AI Club Kickoff · `engage:1`\n",
        );
        let result = run_at(&v, DIGEST_TODAY, digest_now());
        assert_eq!(calendar_notes(&v).len(), 1);
        assert_eq!(result.warnings.len(), 1);
        assert!(result.warnings[0].contains("missing uid"));
        assert!(result.warnings[0].contains("events-digest-2026-08-21.md"));
    }

    #[test]
    fn a_quoted_summary_location_or_uid_cannot_corrupt_the_calendar_note() {
        let v = vault();
        let summary = "Info Session: \"Ask Me Anything\"";
        let location = "Room \"204\"";
        proposal(
            &v,
            "events-digest-2026-08-21.md",
            &format!(
                "type: approval\nkind: events-digest\ntitle: \"Events\"\nstatus: approved\n\
                 proposed_at: 2026-08-21\nexpires: 2026-09-03\nsnooze_until: null\ncreated_by: events\n\
                 events:\n  - uid: \"engage:9\"\n    summary: '{summary}'\n\
                 \x20   start: 2026-09-03T18:00\n    end: 2026-09-03T19:30\n    location: '{location}'"
            ),
            "- [x] Thu 9/3 18:00 · Info Session · `engage:9`\n",
        );
        let result = run_at(&v, DIGEST_TODAY, digest_now());
        assert_eq!(calendar_notes(&v).len(), 1);
        let created = v.join("approvals").join(&calendar_notes(&v)[0]);
        let meta = front(&created);
        let event = match crate::yaml::get(&meta, "event") {
            Some(Value::Mapping(m)) => m.clone(),
            other => panic!("event must be a map, got {other:?}"),
        };
        assert_eq!(crate::yaml::get(&event, "summary").map(python_str), Some(summary.to_string()));
        assert_eq!(crate::yaml::get(&event, "location").map(python_str), Some(location.to_string()));
        assert!(result.warnings.is_empty());

        // The uid goes through the same escaper — it is safe unescaped only because the ledger's
        // charclass rejects quotes, and relying on that here would make widening it corrupt this.
        let v = vault();
        let hostile = "engage:\"1\"";
        proposal(
            &v,
            "events-digest-2026-08-21.md",
            &format!(
                "type: approval\nkind: events-digest\nstatus: approved\n\
                 proposed_at: 2026-08-21\nexpires: 2026-09-03\nsnooze_until: null\n\
                 events:\n  - uid: '{hostile}'\n    summary: \"Odd\"\n    start: 2026-09-03T18:00\n\
                 \x20   end: 2026-09-03T19:00\n    location: \"\""
            ),
            &format!("- [x] Thu 9/3 18:00 · Odd · `{hostile}`\n"),
        );
        let result = run_at(&v, DIGEST_TODAY, digest_now());
        assert_eq!(calendar_notes(&v).len(), 1, "{:?}", result.warnings);
        let created = v.join("approvals").join(&calendar_notes(&v)[0]);
        assert_eq!(field(&created, "source_uid"), hostile);
    }

    #[test]
    fn a_re_expanded_digest_never_writes_a_second_calendar_note() {
        // The digest is expanded and THEN stamped `executed`. If the stamp fails — a transient file
        // lock from Obsidian Git is enough — the digest stays `approved` and is re-expanded next
        // run, and `calendar_note`'s suffix loop would turn that into a second approved note, hence
        // a second real calendar event. A block-style `judgment:` stands in for the monkeypatch:
        // `write_literals` guards on load, `create` does not.
        let v = vault();
        let path = digest(&v, "approved", &["engage:1"]);
        let text = read(&path);
        pystr::write_text(&path, &text.replacen("events:\n", "judgment:\n  migrated: true\nevents:\n", 1))
            .unwrap();

        let result = run_at(&v, DIGEST_TODAY, digest_now());
        assert!(result.warnings.iter().any(|w| w.contains("transition failed")), "{:?}", result.warnings);
        assert!(path.exists(), "the digest must still be approved and un-archived");
        assert_eq!(calendar_notes(&v).len(), 1);

        // Repair the digest and re-run: the existing note blocks a second one.
        let text = read(&path);
        pystr::write_text(&path, &text.replace("judgment:\n  migrated: true\n", "")).unwrap();
        run_at(&v, DIGEST_TODAY, digest_now());
        assert_eq!(calendar_notes(&v), vec!["calendar-event-ai-club-kickoff.md".to_string()]);
    }

    #[test]
    fn a_settled_note_in_archive_blocks_re_creation_permanently() {
        // Spec §8: an existing note in approvals/ blocks re-emission, a settled one in archive/
        // blocks it forever.
        let v = vault();
        digest(&v, "approved", &["engage:1"]);
        run_at(&v, DIGEST_TODAY, digest_now());
        std::fs::create_dir_all(v.join("archive")).unwrap();
        for name in calendar_notes(&v) {
            std::fs::rename(v.join("approvals").join(&name), v.join("archive").join(&name)).unwrap();
        }
        digest(&v, "approved", &["engage:1"]);
        run_at(&v, DIGEST_TODAY, digest_now());
        assert!(calendar_notes(&v).is_empty());
    }

    #[test]
    fn existing_source_uids_reads_both_folders_and_skips_the_unreadable() {
        let v = vault();
        note(&v, "approvals", "a.md", "type: approval\nsource_uid: \"ics:1\"\n");
        note(&v, "archive", "b.md", "type: approval\nsource_uid: \"ics:2\"\n");
        note(&v, "approvals", "c.md", "type: approval\nsource_uid: null\n");
        pystr::write_text(&v.join("approvals").join("broken.md"), "---\ntitle: [unclosed\n---\n").unwrap();
        let uids = existing_source_uids(&v);
        assert_eq!(uids.len(), 2);
        assert!(uids.contains("ics:1") && uids.contains("ics:2"));
    }

    /// Reference output. Every byte below came from running the **Python** `_calendar_note`
    /// against the same payload entry, with the minted id masked. This note becomes a real Google
    /// Calendar event, so its shape is a contract with the cloud executor, not a detail.
    #[test]
    fn a_calendar_note_reproduces_pythons_bytes_exactly() {
        let v = vault();
        std::fs::create_dir_all(v.join("approvals")).unwrap();
        let entry = yaml(
            "uid: \"engage:1\"\nsummary: \"AI Club Kickoff\"\nstart: 2026-09-03T18:00\n\
             end: 2026-09-03T19:30\nlocation: \"Lloyd 38\"\n",
        );
        let ctx = default_ctx();
        let mut journal = Journal::new(&v);
        let stem = calendar_note(&v, &entry, &ctx, &mut journal).unwrap().unwrap();
        assert_eq!(stem, "calendar-event-ai-club-kickoff");

        let path = v.join("approvals").join(format!("{stem}.md"));
        let id = field(&path, "id");
        assert!(crate::ids::is_id(&id));
        assert_eq!(
            read(&path).replace(&id, "<ID>"),
            "---\n\
             type: approval\n\
             kind: calendar-event\n\
             title: \"AI Club Kickoff\"\n\
             status: approved\n\
             source_uid: \"engage:1\"\n\
             created_by: events\n\
             event:\n  \
             summary: \"AI Club Kickoff\"\n  \
             start: 2026-09-03T18:00\n  \
             end: 2026-09-03T19:30\n  \
             location: \"Lloyd 38\"\n\
             id: <ID>\n\
             ---\n\n\
             Approved from the events digest. Source uid `engage:1`.\n"
        );

        // ...and a payload entry missing either half writes nothing at all.
        assert_eq!(
            calendar_note(&v, &yaml("uid: \"x:1\"\n"), &ctx, &mut journal).unwrap(),
            None
        );
        assert_eq!(
            calendar_note(&v, &yaml("summary: \"No uid\"\n"), &ctx, &mut journal).unwrap(),
            None
        );
    }

    /// A seconds-bearing `start:` is written back at minute precision, because PyYAML resolves it
    /// to a `datetime` and `_stamp` formats it — the executor does not expect `18:00:00`.
    #[test]
    fn a_digest_payload_stamp_is_written_at_minute_precision() {
        assert_eq!(stamp(Some(&Value::String("2026-09-03T18:00:00".into()))), "2026-09-03T18:00");
        assert_eq!(stamp(Some(&Value::String("2026-09-03T18:00".into()))), "2026-09-03T18:00");
        assert_eq!(stamp(Some(&Value::String("  spaced  ".into()))), "spaced");
        assert_eq!(stamp(None), "None");
        assert_eq!(stamp(Some(&Value::Null)), "None");
    }

    #[test]
    fn digest_uids_reads_only_the_last_backticked_token_on_a_checkbox_line() {
        let (checked, unchecked) = digest_uids(
            "- [x] Thu 9/3 · `Career Fair` · `ics:evt-1`\n\
             - [X] Fri · `ics:evt-2`  \n\
             - [ ] Sat · `ics:evt-3`\n\
             not a checkbox line `ics:evt-4`\n",
        );
        assert_eq!(checked.iter().cloned().collect::<Vec<_>>(), vec!["ics:evt-1", "ics:evt-2"]);
        assert_eq!(unchecked.iter().cloned().collect::<Vec<_>>(), vec!["ics:evt-3"]);
    }

    // -----------------------------------------------------------------------------------------
    // The event-check card (F3): approve → obligation, reject → drop, by the student
    // -----------------------------------------------------------------------------------------

    mod event_check {
        use super::*;
        use crate::eventledger::{load_ledger, record_judged_verdict};
        use crate::events::{DiscoveredEvent, EventsConfig};

        const J: &str = "0f0e0d0c-0b0a-4908-8706-050403020100";
        const J2: &str = "1a2b3c4d-5e6f-4a1b-8c2d-3e4f5a6b7c8d";
        const WHY: &str = "the listing does not say who it is for";

        /// A 7–9pm event on `(month, day)` of 2026.
        fn event(uid: &str, title: &str, day: i8) -> DiscoveredEvent {
            DiscoveredEvent {
                uid: uid.to_string(),
                title: title.to_string(),
                start: Some(Date::constant(2026, 8, day).at(19, 0, 0, 0)),
                end: Some(Date::constant(2026, 8, day).at(21, 0, 0, 0)),
                source: "campus".into(),
                ..Default::default()
            }
            .normalized()
        }

        fn unsure(vault: &Path, uid: &str, title: &str, jid: Option<&str>) {
            record_judged_verdict(vault, uid, title, TODAY, "unsure", WHY, jid).unwrap();
        }

        /// The card F2's emitter files for `events`, from the ledger as it stands.
        fn file_card(vault: &Path, events: &[DiscoveredEvent]) -> PathBuf {
            let ledger = load_ledger(vault, None);
            let ctx = WriteContext::new("agent:events", "cli");
            let mut journal = Journal::new(vault);
            let (paths, count) = crate::eventemit::emit_event_checks(
                vault,
                events,
                &ledger,
                &EventsConfig::default(),
                TODAY,
                15,
                &ctx,
                &mut journal,
            );
            assert_eq!(count, 1, "{paths:?}");
            paths[0].clone()
        }

        /// What `decide_inner` does (`app/src/commands.rs`): the status under the console's
        /// context, then `process_approvals` in-process under the default one.
        fn decide(vault: &Path, card: &Path, status: &str) -> ApprovalsResult {
            let rel = rel_path(vault, card);
            let console = WriteContext::new("quinn", "dashboard");
            let mut journal = Journal::new(vault);
            let literals = vec![("status".to_string(), status.to_string())];
            write_literals(vault, &rel, &literals, &console, &mut journal, &WriteOpts::default())
                .unwrap();
            let ctx = default_ctx();
            process_approvals(vault, TODAY, now(), &ctx, &mut journal)
        }

        fn seen(vault: &Path) -> String {
            pystr::read_text(&vault.join("state").join("events-seen.md")).unwrap_or_default()
        }

        /// The ledger's human-answer lines, in file order.
        fn answer_lines(vault: &Path) -> Vec<String> {
            pystr::splitlines(&seen(vault))
                .into_iter()
                .filter(|l| l.contains(" · by:"))
                .map(str::to_string)
                .collect()
        }

        fn archived(vault: &Path, card: &Path) -> PathBuf {
            vault.join("archive").join(card.file_name().unwrap())
        }

        #[test]
        fn approving_an_event_check_records_obligation_by_the_human_and_archives() {
            let v = vault();
            let fair = event("ics:fair-1", "Career fair", 25);
            unsure(&v, "ics:fair-1", "Career fair", Some(J));
            let card = file_card(&v, std::slice::from_ref(&fair));
            let before = load_ledger(&v, None);
            assert!(crate::eventroster::relevant_events(std::slice::from_ref(&fair), &before).is_empty());

            let result = decide(&v, &card, "approved");
            assert!(result.warnings.is_empty(), "{:?}", result.warnings);
            let entry = &load_ledger(&v, None)["ics:fair-1"];
            assert_eq!(entry.verdict.as_deref(), Some("obligation"));
            assert_eq!(entry.answered_by, "quinn");
            assert_eq!(entry.judgment_id, J);

            assert!(!card.exists());
            let archived = archived(&v, &card);
            assert_eq!(field(&archived, "status"), "executed");
            assert!(!field(&archived, "executed_at").is_empty());
            assert_eq!(result.executed, vec![card.file_stem().unwrap().to_string_lossy().to_string()]);
            let after = load_ledger(&v, None);
            let relevant = crate::eventroster::relevant_events(std::slice::from_ref(&fair), &after);
            assert_eq!(relevant.len(), 1);
            assert_eq!(relevant[0].uid, "ics:fair-1");
        }

        #[test]
        fn rejecting_an_event_check_records_drop() {
            let v = vault();
            let fair = event("ics:fair-1", "Career fair", 25);
            unsure(&v, "ics:fair-1", "Career fair", Some(J));
            let card = file_card(&v, std::slice::from_ref(&fair));

            let result = decide(&v, &card, "rejected");
            assert!(result.warnings.is_empty(), "{:?}", result.warnings);
            let entry = &load_ledger(&v, None)["ics:fair-1"];
            assert_eq!(entry.verdict.as_deref(), Some("drop"));
            assert_eq!(entry.answered_by, "quinn");
            assert_eq!(entry.judgment_id, J);
            assert!(!card.exists());
            assert_eq!(field(&archived(&v, &card), "status"), "rejected");
            let after = load_ledger(&v, None);
            assert!(crate::eventroster::relevant_events(std::slice::from_ref(&fair), &after).is_empty());
        }

        #[test]
        fn every_instance_on_a_series_card_is_answered() {
            let v = vault();
            unsure(&v, "lx:5:1", "Weekly club", Some(J));
            unsure(&v, "lx:5:2", "Weekly club (week 2)", Some(J2));
            // `lx:5:3` has no ledger line at all: its answer falls back to the card's title.
            let text = format!(
                "---\ntype: approval\nkind: event-check\n\
                 title: \"Weekly club · Mon 24 Aug 7–9pm · +2 more\"\nstatus: pending\n\
                 source_uid: \"lx:5:1\"\nseries_uid: \"lx:5\"\n\
                 events:\n- \"lx:5:1\"\n- \"lx:5:2\"\n- \"lx:5:3\"\n\
                 judgment_id: {J}\njudgment_kind: event\n\
                 proposed_at: 2026-08-20\nfirst_proposed_at: 2026-08-20\nexpires: 2026-08-24\n\
                 snooze_until: null\ncreated_by: events\n---\n\nbody\n"
            );
            let ctx = WriteContext::new("agent:events", "cli");
            let mut journal = Journal::new(&v);
            let rel = "approvals/event-check-weekly-club-2026-08-24.md";
            create(&v, rel, &text, &ctx, &mut journal, None).unwrap();

            let result = decide(&v, &v.join(rel), "approved");
            assert!(result.warnings.is_empty(), "{:?}", result.warnings);
            assert_eq!(
                answer_lines(&v),
                vec![
                    format!("- lx:5:1 · Weekly club · verdict:obligation · by:quinn · jid:{J} · answered 2026-08-20"),
                    format!("- lx:5:2 · Weekly club (week 2) · verdict:obligation · by:quinn · jid:{J2} · answered 2026-08-20"),
                    "- lx:5:3 · Weekly club - Mon 24 Aug 7–9pm - +2 more · verdict:obligation · by:quinn · answered 2026-08-20".to_string(),
                ]
            );
            let ledger = load_ledger(&v, None);
            for uid in ["lx:5:1", "lx:5:2", "lx:5:3"] {
                assert_eq!(ledger[uid].verdict.as_deref(), Some("obligation"), "{uid}");
                assert_eq!(ledger[uid].answered_by, "quinn", "{uid}");
            }
            assert_eq!(ledger["lx:5:3"].judgment_id, "");
        }

        #[test]
        fn an_expired_or_snoozed_event_check_writes_nothing_to_the_ledger() {
            // Expired: the card's `expires` (the event's day) is behind us.
            let v = vault();
            let fair = event("ics:fair-1", "Career fair", 25);
            unsure(&v, "ics:fair-1", "Career fair", Some(J));
            let card = file_card(&v, std::slice::from_ref(&fair));
            let before = seen(&v);
            let result = run_at(&v, Date::constant(2026, 8, 26), now());
            assert_eq!(result.expired.len(), 1, "{result:?}");
            assert_eq!(field(&archived(&v, &card), "status"), "expired");
            assert_eq!(seen(&v), before);
            assert_eq!(load_ledger(&v, None)["ics:fair-1"].verdict.as_deref(), Some("unsure"));

            // Snoozed: nothing happens until it wakes.
            let v = vault();
            unsure(&v, "ics:fair-1", "Career fair", Some(J));
            let card = file_card(&v, std::slice::from_ref(&fair));
            let rel = rel_path(&v, &card);
            let console = WriteContext::new("quinn", "dashboard");
            let mut journal = Journal::new(&v);
            let literals = vec![
                ("status".to_string(), "snoozed".to_string()),
                ("snooze_until".to_string(), "2026-08-23".to_string()),
            ];
            write_literals(&v, &rel, &literals, &console, &mut journal, &WriteOpts::default()).unwrap();
            let before = seen(&v);
            let result = run(&v);
            assert!(result.warnings.is_empty(), "{:?}", result.warnings);
            assert!(card.exists());
            assert_eq!(field(&card, "status"), "snoozed");
            assert_eq!(seen(&v), before);
        }

        #[test]
        fn a_ledger_write_failure_leaves_the_card_for_the_next_run() {
            let v = vault();
            let fair = event("ics:fair-1", "Career fair", 25);
            unsure(&v, "ics:fair-1", "Career fair", Some(J));
            let card = file_card(&v, std::slice::from_ref(&fair));
            let seen_path = v.join("state").join("events-seen.md");
            std::fs::remove_file(&seen_path).unwrap();
            std::fs::create_dir_all(&seen_path).unwrap();

            let result = decide(&v, &card, "approved");
            let name = card.file_name().unwrap().to_string_lossy().to_string();
            assert_eq!(result.warnings, vec![format!("transition failed: {name}")]);
            assert!(card.exists());
            assert_eq!(field(&card, "status"), "approved");
            assert!(result.executed.is_empty());

            // The next run, with the ledger writable again, settles it.
            std::fs::remove_dir_all(&seen_path).unwrap();
            unsure(&v, "ics:fair-1", "Career fair", Some(J));
            let result = run(&v);
            assert!(result.warnings.is_empty(), "{:?}", result.warnings);
            assert!(!card.exists());
            assert_eq!(load_ledger(&v, None)["ics:fair-1"].answered_by, "quinn");
        }

        #[test]
        fn settling_twice_is_idempotent() {
            let v = vault();
            let fair = event("ics:fair-1", "Career fair", 25);
            unsure(&v, "ics:fair-1", "Career fair", Some(J));
            let card = file_card(&v, std::slice::from_ref(&fair));
            let meta = front(&card);
            let mut journal = Journal::new(&v);
            settle_event_check(&v, &meta, "obligation", TODAY, &mut journal).unwrap();
            let once = load_ledger(&v, None);
            settle_event_check(&v, &meta, "obligation", TODAY, &mut journal).unwrap();
            let twice = load_ledger(&v, None);
            assert_eq!(once, twice);
            assert_eq!(twice["ics:fair-1"].verdict.as_deref(), Some("obligation"));
            let events = std::slice::from_ref(&fair);
            assert_eq!(
                crate::eventroster::relevant_events(events, &once),
                crate::eventroster::relevant_events(events, &twice)
            );
        }

        #[test]
        fn a_card_with_no_journaled_human_decision_records_by_unknown() {
            let v = vault();
            let fair = event("ics:fair-1", "Career fair", 25);
            unsure(&v, "ics:fair-1", "Career fair", Some(J));
            let card = file_card(&v, std::slice::from_ref(&fair));
            // Approved by a hand edit outside the console: no journal record says who.
            let text = read(&card).replace("status: pending", "status: approved");
            pystr::write_text(&card, &text).unwrap();

            let result = run(&v);
            assert!(result.warnings.is_empty(), "{:?}", result.warnings);
            let entry = &load_ledger(&v, None)["ics:fair-1"];
            assert_eq!(entry.verdict.as_deref(), Some("obligation"));
            assert_eq!(entry.answered_by, "unknown");
            assert_eq!(field(&archived(&v, &card), "status"), "executed");
        }
    }

    // -----------------------------------------------------------------------------------------
    // The event-accept card (events spec §4.2, D1, D10; spec tests 9–12): Accept writes the
    // commitment and the register task from the card's own `instances:`, then the stamp. Every
    // title, place and uid is invented.
    // -----------------------------------------------------------------------------------------

    mod event_accept {
        use super::*;
        use crate::commitments::{Level, Meet, CARD_ACTOR};
        use crate::eventledger::{load_ledger, record_judged_verdict};
        use crate::events::{DiscoveredEvent, EventsConfig};
        use jiff::civil::Time;

        const J: &str = "0f0e0d0c-0b0a-4908-8706-050403020100";
        const WHY: &str = "the listing says every first-year attends";
        const URL: &str = "https://events.example.edu/e/1";

        /// A 7–9pm campus event on 2026-08-`day`, in `series` (its own uid when empty).
        fn event(uid: &str, series: &str, title: &str, day: i8) -> DiscoveredEvent {
            DiscoveredEvent {
                uid: uid.to_string(),
                title: title.to_string(),
                start: Some(Date::constant(2026, 8, day).at(19, 0, 0, 0)),
                end: Some(Date::constant(2026, 8, day).at(21, 0, 0, 0)),
                source: "campus".into(),
                location: "Ferguson Center".into(),
                url: URL.into(),
                series_uid: series.to_string(),
                ..Default::default()
            }
            .normalized()
        }

        /// `e`, with the feed saying registration is required, by `deadline` when it gives one.
        fn registered(mut e: DiscoveredEvent, deadline: Option<Date>) -> DiscoveredEvent {
            e.registration = true;
            e.registration_deadline = deadline;
            e
        }

        /// The one `event-accept` card the emitter files for `events`, each judged `verdict` first.
        fn file_card(vault: &Path, events: &[DiscoveredEvent], verdict: &str) -> PathBuf {
            for e in events {
                record_judged_verdict(vault, &e.uid, &e.title, TODAY, verdict, WHY, Some(J)).unwrap();
            }
            let ledger = load_ledger(vault, None);
            let config = EventsConfig { event_cards: true, ..EventsConfig::default() };
            let ctx = WriteContext::new("agent:events", "cli");
            let mut journal = Journal::new(vault);
            let (paths, count) = crate::eventemit::emit_event_accepts(
                vault, events, &ledger, &config, TODAY, 15, verdict, &ctx, &mut journal,
            );
            assert_eq!(count, 1, "{paths:?}");
            paths[0].clone()
        }

        /// The console's half of `decide_inner` (`app/src/commands.rs`): the status, set by the
        /// vault's own human actor.
        fn mark(vault: &Path, card: &Path, status: &str) {
            let human = crate::journal::read_human_actor(vault).unwrap();
            let console = WriteContext::new(human, "dashboard");
            let mut journal = Journal::new(vault);
            let literals = vec![("status".to_string(), status.to_string())];
            let rel = rel_path(vault, card);
            write_literals(vault, &rel, &literals, &console, &mut journal, &WriteOpts::default()).unwrap();
        }

        /// All of `decide_inner`: the status, then `process_approvals` under the default context.
        fn decide(vault: &Path, card: &Path, status: &str) -> ApprovalsResult {
            mark(vault, card, status);
            run(vault)
        }

        fn archived(vault: &Path, card: &Path) -> PathBuf {
            vault.join("archive").join(card.file_name().unwrap())
        }

        fn files(vault: &Path, folder: &str) -> Vec<String> {
            let mut out: Vec<String> = std::fs::read_dir(vault.join(folder))
                .map(|d| d.filter_map(|e| e.ok()).map(|e| e.file_name().to_string_lossy().to_string()).collect())
                .unwrap_or_default();
            out.sort();
            out
        }

        fn text(record: &crate::ledger::Record, key: &str) -> String {
            record.get(key).and_then(|v| v.as_str()).unwrap_or_default().to_string()
        }

        /// The journal's `create` records under `folder/`, in order.
        fn creates(vault: &Path, folder: &str) -> Vec<crate::ledger::Record> {
            let prefix = format!("{folder}/");
            journal_records(vault)
                .into_iter()
                .filter(|r| text(r, "op") == "create" && text(r, "path").starts_with(&prefix))
                .collect()
        }

        /// A hand-written note at `rel`, written outside the engine (no journal record).
        fn seed(vault: &Path, rel: &str, front: &str) {
            let path = vault.join(rel);
            std::fs::create_dir_all(path.parent().unwrap()).unwrap();
            pystr::write_text(&path, &format!("---\n{front}\n---\n\nMine.\n")).unwrap();
        }

        fn at(h: i8, m: i8) -> Time {
            Time::new(h, m, 0, 0).unwrap()
        }

        #[test]
        fn approving_an_obligation_card_writes_a_hard_commitment() {
            let v = vault();
            let card = file_card(&v, &[event("ics:fair-1", "", "Career fair", 25)], "obligation");
            assert_eq!(proposal_weight(&front(&card)), 1);
            let card_rel = rel_path(&v, &card);
            let result = decide(&v, &card, "approved");
            assert!(result.warnings.is_empty(), "{:?}", result.warnings);

            assert_eq!(files(&v, "commitments"), vec!["career-fair.md"]);
            let note = v.join("commitments").join("career-fair.md");
            for (key, want) in [
                ("type", "commitment"),
                ("kind", "event"),
                ("level", "hard"),
                ("title", "Career fair"),
                ("where", "Ferguson Center"),
                ("from", "2026-08-25"),
                ("until", "2026-08-25"),
                ("source_uid", "ics:fair-1"),
                ("status", "confirmed"),
            ] {
                assert_eq!(field(&note, key), want, "{key}");
            }
            assert!(read(&note).ends_with("\n\nYou accepted this from your campus events.\n"), "{}", read(&note));
            // The one-day `meets` (Tuesday 25 Aug, 7–9pm), as `commitments::load` reads it back.
            let set = crate::commitments::load(&v);
            assert!(set.warnings.is_empty(), "{:?}", set.warnings);
            assert_eq!(set.confirmed.len(), 1);
            assert_eq!(set.confirmed[0].level, Level::Hard);
            assert_eq!(set.confirmed[0].meets, vec![Meet { days: vec!["tue"], start: at(19, 0), end: at(21, 0) }]);
            assert!(files(&v, "tasks").is_empty(), "no registration, no task");

            // The journal: the note's `create` (as `agent:commitments`, carrying its id), then the
            // card's stamp, then its move to `archive/`.
            let records = journal_records(&v);
            let index = |wanted: &dyn Fn(&crate::ledger::Record) -> bool| {
                records.iter().position(wanted).expect("the record is journaled")
            };
            let created = index(&|r| text(r, "op") == "create" && text(r, "path") == "commitments/career-fair.md");
            let stamped = index(&|r| text(r, "op") == "set" && text(r, "path") == card_rel && text(r, "new") == "executed");
            let moved = index(&|r| text(r, "op") == "delete" && text(r, "path") == card_rel);
            assert!(created < stamped && stamped < moved, "{created} {stamped} {moved}");
            assert_eq!(text(&records[created], "actor"), CARD_ACTOR);
            assert_eq!(text(&records[created], "id"), field(&note, "id"));

            assert!(!card.exists());
            let archived = archived(&v, &card);
            assert_eq!(field(&archived, "status"), "executed");
            assert!(!field(&archived, "executed_at").is_empty());
            assert_eq!(result.executed, vec![stem_of(&card)]);
        }

        #[test]
        fn approving_an_opportunity_card_writes_a_soft_commitment() {
            let v = vault();
            let card = file_card(&v, &[event("ics:talk-1", "", "Engineering talk", 26)], "opportunity");
            assert_eq!(field(&card, "verdict"), "opportunity");
            let result = decide(&v, &card, "approved");
            assert!(result.warnings.is_empty(), "{:?}", result.warnings);

            assert_eq!(files(&v, "commitments"), vec!["engineering-talk.md"]);
            let note = v.join("commitments").join("engineering-talk.md");
            for (key, want) in [
                ("kind", "event"),
                ("level", "soft"),
                ("from", "2026-08-26"),
                ("until", "2026-08-26"),
                ("source_uid", "ics:talk-1"),
                ("status", "confirmed"),
            ] {
                assert_eq!(field(&note, key), want, "{key}");
            }
            let set = crate::commitments::load(&v);
            assert_eq!(set.confirmed.len(), 1);
            assert_eq!(set.confirmed[0].level, Level::Soft);
            assert_eq!(set.confirmed[0].meets, vec![Meet { days: vec!["wed"], start: at(19, 0), end: at(21, 0) }]);
            assert_eq!(field(&archived(&v, &card), "status"), "executed");
            assert_eq!(result.executed, vec![stem_of(&card)]);
        }

        #[test]
        fn registration_writes_the_register_task() {
            // With a deadline: a two-date series books both dates and writes one task, the
            // primary's, due at the deadline at 23:59, under the pass's own context.
            let v = vault();
            let deadline = Some(Date::constant(2026, 8, 23));
            let series = [
                registered(event("lx:7:1", "lx:7", "Resume clinic", 25), deadline),
                registered(event("lx:7:2", "lx:7", "Resume clinic", 27), deadline),
            ];
            let card = file_card(&v, &series, "obligation");
            let result = decide(&v, &card, "approved");
            assert!(result.warnings.is_empty(), "{:?}", result.warnings);
            assert_eq!(files(&v, "commitments").len(), 2);
            assert_eq!(files(&v, "tasks"), vec!["register-resume-clinic-2026-08-25.md"]);
            let task = v.join("tasks").join("register-resume-clinic-2026-08-25.md");
            for (key, want) in [
                ("title", "Register: Resume clinic"),
                ("due", "2026-08-23T23:59"),
                ("effort_hours", "0.25"),
                ("importance", "3"),
                ("domain", "school"),
                ("status", "active"),
                ("created_by", "events"),
                ("source_uid", "register:lx:7:1"),
            ] {
                assert_eq!(field(&task, key), want, "{key}");
            }
            let made = creates(&v, "tasks");
            assert_eq!(made.len(), 1);
            assert_eq!(text(&made[0], "actor"), "agent:approvals");
            assert_eq!(text(&made[0], "id"), field(&task, "id"));
            assert_eq!(field(&archived(&v, &card), "status"), "executed");

            // With `registration_deadline: null`: `due` is null, the task is in the read model's
            // undated list, and no field holds the event's start. The body names the start as an
            // upper bound, with the URL.
            let v = vault();
            let expo = registered(event("ics:expo-1", "", "Startup expo", 26), None);
            let card = file_card(&v, std::slice::from_ref(&expo), "obligation");
            let result = decide(&v, &card, "approved");
            assert!(result.warnings.is_empty(), "{:?}", result.warnings);
            assert_eq!(files(&v, "tasks"), vec!["register-startup-expo-2026-08-26.md"]);
            let task = v.join("tasks").join("register-startup-expo-2026-08-26.md");
            let meta = front(&task);
            assert_eq!(crate::yaml::get(&meta, "due"), Some(&Value::Null));
            for (key, value) in &meta {
                let shown = python_str(value);
                assert!(!shown.contains("2026-08-26") && !shown.contains("19:00"), "{}: {shown}", key_text(key));
            }
            let body = read(&task);
            assert!(
                body.contains(
                    "Registration closes by Wed 26 Aug 7pm at the latest; check the event page for the real deadline."
                ),
                "{body}"
            );
            assert!(body.contains(URL), "{body}");
            let loaded = crate::surface::load(&v, TODAY);
            let undated = crate::surface::ranked_list(&loaded, TODAY, "undated", &[]);
            let slugs: Vec<&str> = undated.rows.iter().map(|r| r.slug.as_str()).collect();
            assert_eq!(slugs, vec!["register-startup-expo-2026-08-26"]);
        }

        #[test]
        fn a_retried_settlement_writes_no_second_note() {
            // A card left `approved` with its notes written (the stamp failed), settled again:
            // nothing new is written, and the card is stamped and archived.
            let v = vault();
            let fair = registered(event("ics:fair-1", "", "Career fair", 25), Some(Date::constant(2026, 8, 23)));
            let card = file_card(&v, std::slice::from_ref(&fair), "obligation");
            mark(&v, &card, "approved");
            let mut journal = Journal::new(&v);
            let settled = settle_event_accept(&v, &front(&card), Level::Hard, TODAY, &default_ctx(), &mut journal);
            assert_eq!(settled.unwrap(), Some(Vec::new()));
            assert_eq!(field(&card, "status"), "approved");
            let (notes, tasks) = (files(&v, "commitments"), files(&v, "tasks"));
            assert_eq!((notes.len(), tasks.len()), (1, 1));
            let journaled = journal_records(&v).len();

            let result = run(&v);
            assert!(result.warnings.is_empty(), "{:?}", result.warnings);
            assert_eq!((files(&v, "commitments"), files(&v, "tasks")), (notes, tasks));
            let since: Vec<String> = journal_records(&v)[journaled..].iter().map(|r| text(r, "op")).collect();
            assert!(!since.is_empty() && since.iter().all(|op| op != "create"), "{since:?}");
            assert_eq!(field(&archived(&v, &card), "status"), "executed");
            assert_eq!(result.executed, vec![stem_of(&card)]);

            // The ever-written set: a uid on a live commitment, a live task or an archived (deleted)
            // commitment is skipped, and so is the primary's register task deleted to `archive/`.
            // An archived approval card carrying the primary as its `source_uid` does not count, so
            // the primary is still written.
            let v = vault();
            let series = [
                registered(event("lx:9:1", "lx:9", "Writing lab", 24), None),
                event("lx:9:2", "lx:9", "Writing lab", 25),
                event("lx:9:3", "lx:9", "Writing lab", 26),
                event("lx:9:4", "lx:9", "Writing lab", 27),
            ];
            let card = file_card(&v, &series, "obligation");
            seed(&v, "commitments/writing-lab-two.md", "type: commitment\nkind: event\nsource_uid: \"lx:9:2\"");
            seed(&v, "archive/writing-lab-three.md", "type: commitment\nkind: event\nsource_uid: \"lx:9:3\"");
            seed(&v, "tasks/writing-lab-four.md", "title: Writing lab four\nsource_uid: \"lx:9:4\"");
            seed(&v, "archive/register-writing-lab.md", "title: \"Register: Writing lab\"\nsource_uid: \"register:lx:9:1\"");
            seed(
                &v,
                "archive/event-check-writing-lab.md",
                "type: approval\nkind: event-check\nstatus: expired\nsource_uid: \"lx:9:1\"\nseries_uid: \"lx:9\"",
            );
            let result = decide(&v, &card, "approved");
            assert!(result.warnings.is_empty(), "{:?}", result.warnings);
            let made = creates(&v, "commitments");
            assert_eq!(made.len(), 1, "{made:?}");
            let source = made[0].get("new").and_then(|n| n.get("source_uid")).and_then(|s| s.as_str());
            assert_eq!(source, Some("lx:9:1"));
            assert!(creates(&v, "tasks").is_empty());
            assert_eq!(files(&v, "commitments"), vec!["writing-lab-two.md", "writing-lab.md"]);
            assert_eq!(files(&v, "tasks"), vec!["writing-lab-four.md"]);
            assert_eq!(field(&archived(&v, &card), "status"), "executed");
        }

        #[test]
        fn a_failed_write_leaves_the_card_approved_for_the_next_pass() {
            let v = vault();
            let card = file_card(&v, &[event("ics:fair-1", "", "Career fair", 25)], "obligation");
            mark(&v, &card, "approved");
            // The journal cannot be appended to, so the commitment's `create` fails before its file.
            let journal_dir = v.join("state").join("journal");
            let aside = v.join("state").join("journal-aside");
            std::fs::rename(&journal_dir, &aside).unwrap();
            pystr::write_text(&journal_dir, "not a folder\n").unwrap();
            let result = run(&v);
            assert_eq!(result.warnings, vec![format!("transition failed: {}", name_of(&card))]);
            assert!(result.executed.is_empty());
            assert_eq!(field(&card, "status"), "approved");
            assert!(files(&v, "commitments").is_empty());

            // The next pass, with the journal back, settles it, once.
            std::fs::remove_file(&journal_dir).unwrap();
            std::fs::rename(&aside, &journal_dir).unwrap();
            let result = run(&v);
            assert!(result.warnings.is_empty(), "{:?}", result.warnings);
            assert_eq!(files(&v, "commitments"), vec!["career-fair.md"]);
            assert_eq!(field(&archived(&v, &card), "status"), "executed");
        }

        #[test]
        fn a_card_with_no_payload_or_no_verdict_it_reads_stays_approved_and_warns() {
            // Only a hand edit makes either. Nothing is written, and the card waits, as an approved
            // `task` card with no payload does.
            for (from, to, warning) in [
                ("\ninstances:", "\nlisted:", "missing event payload: {name}"),
                ("verdict: obligation", "verdict: required", "{name}: no verdict the settlement reads"),
            ] {
                let v = vault();
                let card = file_card(&v, &[event("ics:fair-1", "", "Career fair", 25)], "obligation");
                let edited = read(&card).replacen(from, to, 1);
                assert_ne!(edited, read(&card));
                pystr::write_text(&card, &edited).unwrap();
                let result = decide(&v, &card, "approved");
                let name = name_of(&card);
                assert_eq!(result.warnings, vec![warning.replace("{name}", &name)]);
                assert!(result.executed.is_empty());
                assert_eq!(field(&card, "status"), "approved");
                assert!(files(&v, "commitments").is_empty() && files(&v, "tasks").is_empty());
            }
        }

        #[test]
        fn a_malformed_instance_entry_is_skipped_with_a_warning_and_the_rest_settles() {
            let v = vault();
            let series = [event("lx:3:1", "lx:3", "Study hall", 25), event("lx:3:2", "lx:3", "Study hall", 27)];
            let card = file_card(&v, &series, "obligation");
            let edited = read(&card).replacen("2026-08-27T19:00", "soon", 1);
            assert_ne!(edited, read(&card));
            pystr::write_text(&card, &edited).unwrap();
            let result = decide(&v, &card, "approved");
            assert_eq!(result.warnings, vec![format!("{}: bad instance entry 2", name_of(&card))]);
            let made = creates(&v, "commitments");
            assert_eq!(made.len(), 1, "{made:?}");
            let source = made[0].get("new").and_then(|n| n.get("source_uid")).and_then(|s| s.as_str());
            assert_eq!(source, Some("lx:3:1"));
            assert_eq!(field(&archived(&v, &card), "status"), "executed");
        }

        // --- T3.1b: the rejected arm and expiry (spec tests 13–14; §4.3, §4.4, D8) ------------

        fn ledger_text(vault: &Path) -> String {
            pystr::read_text(&vault.join("state").join("events-seen.md")).unwrap_or_default()
        }

        /// The ledger's `declined` lines, in file order.
        fn declined_lines(vault: &Path) -> Vec<String> {
            let text = ledger_text(vault);
            pystr::splitlines(&text).into_iter().filter(|l| l.contains(" · declined ")).map(str::to_string).collect()
        }

        #[test]
        fn rejecting_writes_one_declined_line_per_listed_uid() {
            // One `declined` line per uid the card's `events:` lists, in its order, then the card
            // archived `rejected`. Nothing else: no note, no task, no `create` record.
            let v = vault();
            let deadline = Some(Date::constant(2026, 8, 23));
            let series = [
                registered(event("lx:7:1", "lx:7", "Resume clinic", 25), deadline),
                registered(event("lx:7:2", "lx:7", "Resume clinic", 27), deadline),
            ];
            let card = file_card(&v, &series, "obligation");
            assert_eq!(crate::eventemit::card_event_uids(&front(&card)), vec!["lx:7:1", "lx:7:2"]);
            assert_eq!(crate::eventroster::relevant_events(&series, &load_ledger(&v, None)).len(), 2);
            let journaled = journal_records(&v).len();

            let result = decide(&v, &card, "rejected");
            assert!(result.warnings.is_empty(), "{:?}", result.warnings);
            assert_eq!(declined_lines(&v), vec!["- lx:7:1 · declined 2026-08-20", "- lx:7:2 · declined 2026-08-20"]);
            let after = load_ledger(&v, None);
            assert!(crate::eventroster::relevant_events(&series, &after).is_empty(), "both leave Coming up");
            assert!(!card.exists());
            assert_eq!(field(&archived(&v, &card), "status"), "rejected");
            assert_eq!(result.rejected, vec![stem_of(&card)]);
            assert!(result.executed.is_empty());
            assert!(files(&v, "commitments").is_empty() && files(&v, "tasks").is_empty());
            let since: Vec<String> = journal_records(&v)[journaled..].iter().map(|r| text(r, "op")).collect();
            assert_eq!(since, vec!["set", "delete"], "the console's status, then the move");

            // D9's rebuild reads the same archived card and finds every line written.
            let before = ledger_text(&v);
            let (mut ledger, mut journal) = (after, Journal::new(&v));
            let carried = crate::eventcarry::run(&v, &series, &mut ledger, TODAY, &default_ctx(), &mut journal);
            assert_eq!(carried, (0, Vec::new()));
            assert_eq!(ledger_text(&v), before);
        }

        #[test]
        fn a_failed_decline_leaves_the_card_rejected_for_the_next_pass() {
            // The ledger first, the move second: a ledger that cannot be appended to leaves the
            // card `rejected` in `approvals/`, and the next pass settles it.
            let v = vault();
            let card = file_card(&v, &[event("ics:fair-1", "", "Career fair", 25)], "obligation");
            let seen = v.join("state").join("events-seen.md");
            let aside = v.join("state").join("events-seen-aside.md");
            std::fs::rename(&seen, &aside).unwrap();
            std::fs::create_dir_all(&seen).unwrap();
            let result = decide(&v, &card, "rejected");
            assert_eq!(result.warnings, vec![format!("transition failed: {}", name_of(&card))]);
            assert!(result.rejected.is_empty());
            assert_eq!(field(&card, "status"), "rejected");

            std::fs::remove_dir(&seen).unwrap();
            std::fs::rename(&aside, &seen).unwrap();
            let result = run(&v);
            assert!(result.warnings.is_empty(), "{:?}", result.warnings);
            assert_eq!(declined_lines(&v), vec!["- ics:fair-1 · declined 2026-08-20"]);
            assert_eq!(field(&archived(&v, &card), "status"), "rejected");
            assert_eq!(result.rejected, vec![stem_of(&card)]);
        }

        #[test]
        fn an_expired_event_accept_card_writes_nothing() {
            // D8: expiry is not an answer. Each card is stamped `expired` and archived, and nothing
            // else is written: no ledger line, no note, no task. The events stay in Coming up, and
            // the carry sees no answered series. The digest's expiry still declines, by contrast
            // (`a_rejected_or_expired_digest_declines_every_event`, unchanged).
            let v = vault();
            let fair = registered(event("ics:fair-1", "", "Career fair", 25), Some(Date::constant(2026, 8, 23)));
            let talk = event("ics:talk-1", "", "Engineering talk", 26);
            let fair_card = file_card(&v, std::slice::from_ref(&fair), "obligation");
            let talk_card = file_card(&v, std::slice::from_ref(&talk), "opportunity");
            assert_eq!(field(&fair_card, "expires"), "2026-08-25");
            assert_eq!(field(&talk_card, "expires"), "2026-08-26");
            let before = ledger_text(&v);
            let journaled = journal_records(&v).len();

            let day = Date::constant(2026, 8, 27);
            let result = run_at(&v, day, day.at(9, 0, 0, 0));
            assert!(result.warnings.is_empty(), "{:?}", result.warnings);
            assert_eq!(result.expired, vec![stem_of(&fair_card), stem_of(&talk_card)]);
            assert!(result.rejected.is_empty() && result.executed.is_empty());
            for card in [&fair_card, &talk_card] {
                assert!(!card.exists());
                assert_eq!(field(&archived(&v, card), "status"), "expired");
            }
            assert_eq!(ledger_text(&v), before, "no ledger line");
            assert!(declined_lines(&v).is_empty());
            assert!(files(&v, "commitments").is_empty() && files(&v, "tasks").is_empty());
            let since: Vec<String> = journal_records(&v)[journaled..].iter().map(|r| text(r, "op")).collect();
            assert_eq!(since, vec!["set", "delete", "set", "delete"], "each card's stamp, then its move");
            let events = [fair, talk];
            let relevant = crate::eventroster::relevant_events(&events, &load_ledger(&v, None));
            assert_eq!(relevant.len(), 2, "both stay in Coming up");
            assert!(crate::eventcarry::answered_series(&v).is_empty(), "an expired card answers nothing");
        }
    }

    // -----------------------------------------------------------------------------------------
    // The commitment-check card (P13, spec §5.2, §5.4, §5.5): approve → the confirmed note or the
    // change, reject → the decline marker, withdrawal by card shape. Every title, room, key and
    // calendar is invented.
    // -----------------------------------------------------------------------------------------

    mod commitment_check {
        use super::*;
        use crate::commitments::{
            self, Change, Instance, Level, Meet, Proposal, Rule, Series, SeriesFile,
            PLANNING_DAY, WINDOW_MARKER, WINDOW_TITLE,
        };
        use crate::weekcal::DayKey;
        use jiff::civil::Time;

        const CS100: &str = "gcal-series:cs100";
        const GOOGLE: &str = "google:abc";

        fn t(h: i8, m: i8) -> Time {
            Time::new(h, m, 0, 0).unwrap()
        }

        fn meet(days: &[DayKey], start: Time, end: Time) -> Meet {
            Meet { days: days.to_vec(), start, end }
        }

        fn proposal(kind: &str, title: &str, uid: &str, meets: Vec<Meet>) -> Proposal {
            Proposal {
                kind: kind.to_string(),
                level: commitments::default_level(kind).unwrap_or(Level::Soft),
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
                from: Some(Date::constant(2026, 8, 19)),
                until: Some(Date::constant(2026, 12, 4)),
                ..proposal("class", "CS 100", CS100, vec![meet(&["mon", "wed", "fri"], t(12, 0), t(12, 50))])
            }
        }

        fn club(uid: &str) -> Proposal {
            proposal("club", "Chess Club", uid, vec![meet(&["tue"], t(18, 0), t(19, 0))])
        }

        fn window() -> Proposal {
            Proposal {
                level: Level::Optional,
                ..proposal(
                    PLANNING_DAY,
                    WINDOW_TITLE,
                    "window:gcal-series:wake",
                    vec![meet(&["mon", "tue", "wed", "thu", "fri"], t(8, 0), t(22, 0))],
                )
            }
        }

        /// The card P11's emitter files for one proposal or one change.
        fn emit(vault: &Path, proposals: &[Proposal], changes: &[Change]) -> PathBuf {
            let ctx = WriteContext::new("agent:rank", "cli");
            let mut journal = Journal::new(vault);
            let (paths, count, warnings) =
                commitments::emit_checks(vault, proposals, changes, TODAY, 15, &ctx, &mut journal);
            assert_eq!(count, 1, "{paths:?} {warnings:?}");
            paths[0].clone()
        }

        fn card(vault: &Path, p: Proposal) -> PathBuf {
            emit(vault, &[p], &[])
        }

        /// What `decide_inner` does (`app/src/commands.rs`): the status under the console's
        /// context — the vault contract's human actor — then `process_approvals` in-process under
        /// the default one.
        fn decide(vault: &Path, card: &Path, status: &str) -> ApprovalsResult {
            let rel = rel_path(vault, card);
            let console = WriteContext::new("quinn", "dashboard");
            let mut journal = Journal::new(vault);
            let literals = vec![("status".to_string(), status.to_string())];
            write_literals(vault, &rel, &literals, &console, &mut journal, &WriteOpts::default())
                .unwrap();
            process_approvals(vault, TODAY, now(), &default_ctx(), &mut journal)
        }

        fn archived(vault: &Path, card: &Path) -> PathBuf {
            vault.join("archive").join(card.file_name().unwrap())
        }

        /// A hand-written note in `commitments/`, its id set.
        fn note(vault: &Path, name: &str, front: &str) -> PathBuf {
            let folder = vault.join("commitments");
            std::fs::create_dir_all(&folder).unwrap();
            let path = folder.join(name);
            pystr::write_text(&path, &format!("---\nid: cmt_0123456789\n{front}\n---\n\nMine.\n")).unwrap();
            path
        }

        /// CS 100 as a confirmed note: Mon/Wed/Fri 12–12:50pm in Room 101, no `until`.
        fn cs100_note(vault: &Path) -> PathBuf {
            note(
                vault,
                "cs-100.md",
                "type: commitment\nkind: class\nlevel: hard\ntitle: \"CS 100\"\ncourse: cs-100\n\
                 meets: [{days: [mon, wed, fri], start: \"12:00\", end: \"12:50\"}]\n\
                 where: \"Room 101\"\nsource_uid: \"gcal-series:cs100\"\nstatus: confirmed",
            )
        }

        fn mapping(pairs: &[(&str, Value)]) -> Mapping {
            pairs.iter().map(|(k, v)| (Value::String(k.to_string()), v.clone())).collect()
        }

        fn s(text: &str) -> Value {
            Value::String(text.to_string())
        }

        fn meets_value(json: serde_json::Value) -> Value {
            crate::yaml::from_json(&json)
        }

        fn mwf_value() -> Value {
            meets_value(serde_json::json!([{"days": ["mon", "wed", "fri"], "start": "12:00", "end": "12:50"}]))
        }

        fn tue_thu_value() -> Value {
            meets_value(serde_json::json!([{"days": ["tue", "thu"], "start": "09:30", "end": "10:45"}]))
        }

        fn change(change: Mapping, was: Mapping) -> Change {
            Change {
                target: "commitments/cs-100.md".into(),
                source_uid: CS100.into(),
                title: "CS 100".into(),
                change,
                was,
            }
        }

        fn files(vault: &Path, folder: &str) -> Vec<String> {
            let mut out: Vec<String> = std::fs::read_dir(vault.join(folder))
                .map(|d| d.filter_map(|e| e.ok()).map(|e| e.file_name().to_string_lossy().to_string()).collect())
                .unwrap_or_default();
            out.sort();
            out
        }

        fn lines_starting(path: &Path, prefix: &str) -> usize {
            read(path).lines().filter(|l| l.starts_with(prefix)).count()
        }

        fn sha_hex10(key: &str) -> String {
            let digest = ring::digest::digest(&ring::digest::SHA256, key.as_bytes());
            digest.as_ref().iter().map(|b| format!("{b:02x}")).collect::<String>()[..10].to_string()
        }

        /// A weekly series held by `cal`, three weeks from Monday 2026-08-17, on `days`, `s`–`e`.
        fn series(uid: &str, cal: &str, title: &str, days: &[DayKey], s: Time, e: Time) -> Series {
            let monday = Date::constant(2026, 8, 17);
            let mut instances = Vec::new();
            for week in 0..3i64 {
                for day in days {
                    let offset = crate::planning::DAY_KEYS.iter().position(|k| k == day).unwrap() as i64;
                    let date = monday.checked_add(jiff::Span::new().days(week * 7 + offset)).unwrap();
                    instances.push(Instance { date, start: Some(s), end: Some(e) });
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
                first: instances.first().map(|i| i.date),
                instances,
                meets: vec![meet(days, s, e)],
                until: None,
                last_seen: Some(TODAY),
            }
        }

        fn file_of(series: Vec<Series>, calendars: &[&str]) -> SeriesFile {
            let mut file = SeriesFile::default();
            for cal in calendars {
                file.calendars.insert(cal.to_string(), TODAY);
            }
            file.series = series;
            file.series.sort_by(|a, b| (&a.source_uid, &a.calendar).cmp(&(&b.source_uid, &b.calendar)));
            file
        }

        fn write_file(vault: &Path, file: &SeriesFile) {
            std::fs::create_dir_all(vault.join("state")).unwrap();
            std::fs::write(vault.join(commitments::SERIES_FILE), commitments::file_bytes(file)).unwrap();
        }

        fn withdraw(vault: &Path, file: &SeriesFile) -> (Vec<String>, Vec<String>) {
            let set = commitments::load(vault);
            let mut journal = Journal::new(vault);
            let done = withdraw_stale(vault, file, &set, TODAY, &default_ctx(), &mut journal);
            (done.withdrawn, done.warnings)
        }

        fn creates_in_commitments(vault: &Path) -> Vec<crate::ledger::Record> {
            journal_records(vault)
                .into_iter()
                .filter(|r| {
                    r.get("op").and_then(|v| v.as_str()) == Some("create")
                        && r.get("path").and_then(|v| v.as_str()).is_some_and(|p| p.starts_with("commitments/"))
                })
                .collect()
        }

        // ---- approved ----

        #[test]
        fn approving_a_class_card_creates_the_confirmed_note_and_archives_executed() {
            let v = vault();
            let card = card(&v, cs100());
            let result = decide(&v, &card, "approved");
            assert!(result.warnings.is_empty(), "{:?}", result.warnings);

            let note = v.join("commitments").join("cs-100.md");
            assert_eq!(files(&v, "commitments"), vec!["cs-100.md"]);
            for (key, want) in [
                ("type", "commitment"),
                ("kind", "class"),
                ("level", "hard"),
                ("title", "CS 100"),
                ("course", "cs-100"),
                ("where", "Room 101"),
                ("from", "2026-08-19"),
                ("until", "2026-12-04"),
                ("source_uid", CS100),
                ("status", "confirmed"),
                ("confirmed_at", "2026-08-20"),
            ] {
                assert_eq!(field(&note, key), want, "{key}");
            }
            let id = field(&note, "id");
            assert!(id.starts_with("cmt_") && id.len() == 14, "{id}");
            // I3: `meets:` is one frontmatter line, and it parses back to the sequence.
            assert_eq!(lines_starting(&note, "meets:"), 1, "{}", read(&note));
            assert!(matches!(crate::yaml::get(&front(&note), "meets"), Some(Value::Sequence(_))));
            let set = commitments::load(&v);
            assert!(set.warnings.is_empty(), "{:?}", set.warnings);
            assert_eq!(set.confirmed.len(), 1);
            assert_eq!(set.confirmed[0].meets, vec![meet(&["mon", "wed", "fri"], t(12, 0), t(12, 50))]);

            let creates = creates_in_commitments(&v);
            assert_eq!(creates.len(), 1);
            assert_eq!(creates[0].get("actor").and_then(|a| a.as_str()), Some(commitments::CARD_ACTOR));

            assert!(!card.exists());
            let archived = archived(&v, &card);
            assert_eq!(field(&archived, "status"), "executed");
            assert!(!field(&archived, "executed_at").is_empty());
            assert_eq!(result.executed, vec![stem_of(&card)]);
        }

        #[test]
        fn a_second_confirmed_note_with_the_same_title_takes_dash_2() {
            let v = vault();
            note(
                &v,
                "cs-100.md",
                "type: commitment\nkind: class\ntitle: \"CS 100\"\ncourse: cs-100\n\
                 meets: [{days: [tue], start: \"09:00\", end: \"10:00\"}]\n\
                 source_uid: \"gcal-series:other\"\nstatus: confirmed",
            );
            let card = card(&v, cs100());
            let result = decide(&v, &card, "approved");
            assert!(result.warnings.is_empty(), "{:?}", result.warnings);
            assert_eq!(files(&v, "commitments"), vec!["cs-100-2.md", "cs-100.md"]);
            assert_eq!(field(&v.join("commitments").join("cs-100-2.md"), "source_uid"), CS100);
        }

        #[test]
        fn approving_when_a_confirmed_note_has_the_key_or_signature_is_refused_and_writes_nothing() {
            // The key: a note with the card's source_uid (another desktop confirmed it first).
            let v = vault();
            note(
                &v,
                "cs-100.md",
                "type: commitment\nkind: class\ntitle: \"CS 100\"\ncourse: cs-100\n\
                 meets: [{days: [tue], start: \"09:00\", end: \"10:00\"}]\n\
                 source_uid: \"gcal-series:cs100\"\nstatus: confirmed",
            );
            let card_a = card(&v, cs100());
            let before = read(&v.join("commitments").join("cs-100.md"));
            let result = decide(&v, &card_a, "approved");
            assert_eq!(result.warnings.len(), 1, "{:?}", result.warnings);
            assert!(result.executed.is_empty());
            assert_eq!(files(&v, "commitments"), vec!["cs-100.md"]);
            assert_eq!(read(&v.join("commitments").join("cs-100.md")), before);
            assert!(creates_in_commitments(&v).is_empty());
            assert_eq!(field(&archived(&v, &card_a), "status"), "refused");

            // The signature: a hand-written note with no key, the same meeting.
            let w = vault();
            note(
                &w,
                "chess.md",
                "type: commitment\nkind: club\ntitle: \"chess  club\"\n\
                 meets: [{days: [tue], start: \"18:00\", end: \"19:00\"}]\nstatus: confirmed",
            );
            let card_b = card(&w, club("gcal-series:chess"));
            let result = decide(&w, &card_b, "approved");
            assert_eq!(result.warnings.len(), 1, "{:?}", result.warnings);
            assert_eq!(files(&w, "commitments"), vec!["chess.md"]);
            assert!(creates_in_commitments(&w).is_empty());
            assert_eq!(field(&archived(&w, &card_b), "status"), "refused");
        }

        #[test]
        fn approving_the_window_card_writes_the_planning_day_note() {
            let v = vault();
            let card = card(&v, window());
            let result = decide(&v, &card, "approved");
            assert!(result.warnings.is_empty(), "{:?}", result.warnings);
            let note = v.join("commitments").join("planning-day.md");
            assert_eq!(files(&v, "commitments"), vec!["planning-day.md"]);
            assert_eq!(field(&note, "kind"), PLANNING_DAY);
            assert_eq!(field(&note, "status"), "confirmed");
            assert_eq!(field(&note, "confirmed_at"), "2026-08-20");
            assert!(field(&note, "id").starts_with("cmt_"));
            assert_eq!(lines_starting(&note, "window:"), 1, "{}", read(&note));
            let set = commitments::load(&v);
            assert!(set.warnings.is_empty(), "{:?}", set.warnings);
            assert!(set.planning_day.is_some());
            assert_eq!(set.window[0], Some((t(8, 0), t(22, 0))));
            assert_eq!(set.window[5], None);
            assert_eq!(field(&archived(&v, &card), "status"), "executed");
        }

        #[test]
        fn approving_a_window_card_when_a_planning_day_exists_is_refused() {
            let v = vault();
            note(
                &v,
                "planning-day.md",
                "type: commitment\nkind: planning-day\nstatus: confirmed\n\
                 window: [{days: [mon], start: \"07:00\", end: \"21:00\"}]",
            );
            let card = card(&v, window());
            let result = decide(&v, &card, "approved");
            assert_eq!(result.warnings.len(), 1, "{:?}", result.warnings);
            assert_eq!(files(&v, "commitments"), vec!["planning-day.md"]);
            assert_eq!(field(&archived(&v, &card), "status"), "refused");
        }

        // ---- rejected ----

        #[test]
        fn rejecting_a_class_card_writes_one_anonymous_marker() {
            let v = vault();
            let card = card(&v, cs100());
            let result = decide(&v, &card, "rejected");
            assert!(result.warnings.is_empty(), "{:?}", result.warnings);
            let name = format!("declined-{}.md", sha_hex10(CS100));
            assert_eq!(files(&v, "commitments"), vec![name.clone()]);
            let marker = v.join("commitments").join(&name);
            let keys: Vec<String> =
                front(&marker).keys().filter_map(crate::yaml::text).collect();
            assert_eq!(keys, vec!["id", "type", "status", "source_uid"]);
            assert_eq!(field(&marker, "type"), "commitment");
            assert_eq!(field(&marker, "status"), "declined");
            assert_eq!(field(&marker, "source_uid"), CS100);
            assert!(field(&marker, "id").starts_with("cmt_"));
            // No title, room or time anywhere in its bytes (§2.3).
            let bytes = read(&marker);
            for leak in ["CS 100", "Room", "12:00", "title", "meets"] {
                assert!(!bytes.contains(leak), "{leak} in {bytes}");
            }
            assert!(commitments::load(&v).declined.contains(CS100));
            assert_eq!(field(&archived(&v, &card), "status"), "rejected");
            assert_eq!(result.rejected, vec![stem_of(&card)]);
        }

        #[test]
        fn rejecting_the_window_card_writes_the_window_marker() {
            let v = vault();
            let card = card(&v, window());
            let result = decide(&v, &card, "rejected");
            assert!(result.warnings.is_empty(), "{:?}", result.warnings);
            let name = format!("declined-{}.md", sha_hex10(WINDOW_MARKER));
            assert_eq!(files(&v, "commitments"), vec![name.clone()]);
            assert_eq!(field(&v.join("commitments").join(&name), "source_uid"), WINDOW_MARKER);
            let set = commitments::load(&v);
            assert!(set.declined.contains(WINDOW_MARKER));
            assert!(set.planning_day.is_none());
        }

        #[test]
        fn a_marker_that_exists_is_not_written_twice() {
            let v = vault();
            let first = card(&v, cs100());
            decide(&v, &first, "rejected");
            // A second card for the key (another desktop's, synced in) is rejected too.
            let text = read(&archived(&v, &first)).replace("status: rejected", "status: pending");
            let again = v.join("approvals").join("commitment-check-again.md");
            pystr::write_text(&again, &text).unwrap();
            let result = decide(&v, &again, "rejected");
            assert!(result.warnings.is_empty(), "{:?}", result.warnings);
            assert_eq!(files(&v, "commitments").len(), 1);
        }

        #[test]
        fn rejecting_a_card_whose_series_has_twins_marks_every_twin() {
            // One real series under two keys (an Outlook invite keeps its own UID).
            let v = vault();
            let gcal = series("gcal-series:chess", GOOGLE, "Chess Club", &["tue"], t(18, 0), t(19, 0));
            let ics = series("ics-series:chess-x", "personal", "Chess Club", &["tue"], t(18, 0), t(19, 0));
            let file = file_of(vec![gcal, ics], &[GOOGLE, "personal"]);
            write_file(&v, &file);
            let card = card(&v, club("gcal-series:chess"));
            let result = decide(&v, &card, "rejected");
            assert!(result.warnings.is_empty(), "{:?}", result.warnings);

            let set = commitments::load(&v);
            let declined: Vec<&str> = set.declined.iter().map(String::as_str).collect();
            assert_eq!(declined, vec!["gcal-series:chess", "ics-series:chess-x"]);
            let (codes, _) = commitments::Codes::load(&v);
            let template = crate::weekcal::WeekCalendar::new(&Mapping::new(), Vec::new());
            let again = commitments::proposals(
                &file, &set, &codes, &[], &template, &BTreeSet::new(), TODAY, true,
            );
            assert!(again.is_empty(), "{again:?}");
        }

        // ---- change cards ----

        #[test]
        fn an_until_change_onto_a_note_with_no_until_applies() {
            let v = vault();
            let target = cs100_note(&v);
            let card = emit(
                &v,
                &[],
                &[change(mapping(&[("until", s("2026-12-04"))]), mapping(&[("until", Value::Null)]))],
            );
            let result = decide(&v, &card, "approved");
            assert!(result.warnings.is_empty(), "{:?}", result.warnings);
            assert_eq!(field(&target, "until"), "2026-12-04");
            assert_eq!(commitments::load(&v).confirmed[0].until, Some(Date::constant(2026, 12, 4)));
            let set = journal_records(&v)
                .into_iter()
                .find(|r| r.get("field").and_then(|f| f.as_str()) == Some("until"))
                .expect("a journaled set of until");
            assert_eq!(set.get("actor").and_then(|a| a.as_str()), Some(commitments::CARD_ACTOR));
            assert_eq!(field(&archived(&v, &card), "status"), "executed");
            assert_eq!(result.executed, vec![stem_of(&card)]);
        }

        #[test]
        fn a_meets_change_applies_and_reads_back_as_a_sequence_load_accepts() {
            let v = vault();
            let target = cs100_note(&v);
            let card = emit(
                &v,
                &[],
                &[change(mapping(&[("meets", tue_thu_value())]), mapping(&[("meets", mwf_value())]))],
            );
            let result = decide(&v, &card, "approved");
            assert!(result.warnings.is_empty(), "{:?}", result.warnings);
            assert_eq!(lines_starting(&target, "meets:"), 1, "{}", read(&target));
            let set = commitments::load(&v);
            assert!(set.warnings.is_empty(), "{:?}", set.warnings);
            assert_eq!(set.confirmed[0].meets, vec![meet(&["tue", "thu"], t(9, 30), t(10, 45))]);
            assert_eq!(field(&archived(&v, &card), "status"), "executed");
        }

        #[test]
        fn a_stale_was_is_superseded_and_archived() {
            // The brief's `a_stale_was_is_refused_and_archived`, stamped `superseded` per the
            // controller's carry-forward 3: the change is then asked once more with the new `was`.
            let v = vault();
            let target = note(
                &v,
                "cs-100.md",
                "type: commitment\nkind: class\ntitle: \"CS 100\"\ncourse: cs-100\n\
                 meets: [{days: [mon], start: \"12:00\", end: \"12:50\"}]\n\
                 where: \"Room 3\"\nsource_uid: \"gcal-series:cs100\"\nstatus: confirmed",
            );
            let card = emit(
                &v,
                &[],
                &[change(mapping(&[("where", s("Room 2"))]), mapping(&[("where", s("Room 101"))]))],
            );
            let before = read(&target);
            let result = decide(&v, &card, "approved");
            assert_eq!(result.warnings.len(), 1, "{:?}", result.warnings);
            assert_eq!(read(&target), before);
            assert_eq!(field(&archived(&v, &card), "status"), "superseded");
            assert!(result.executed.is_empty());
        }

        #[test]
        fn a_hand_written_note_that_differs_only_in_spelling_is_not_stale() {
            // Carry-forward 2: `9:00` is `09:00`, day order and whitespace are not an edit.
            let v = vault();
            let target = note(
                &v,
                "cs-100.md",
                "type: commitment\nkind: class\ntitle: \"CS 100\"\n\
                 meets: [{days: [fri, mon, wed], start: \"9:00\", end: \"9:50\"}]\n\
                 where: \"  Room   101 \"\nsource_uid: \"gcal-series:cs100\"\nstatus: confirmed",
            );
            let was_meets =
                meets_value(serde_json::json!([{"days": ["mon", "wed", "fri"], "start": "09:00", "end": "09:50"}]));
            let card = emit(
                &v,
                &[],
                &[change(
                    mapping(&[("meets", tue_thu_value()), ("where", s("Room 2"))]),
                    mapping(&[("meets", was_meets), ("where", s("Room 101"))]),
                )],
            );
            let result = decide(&v, &card, "approved");
            assert!(result.warnings.is_empty(), "{:?}", result.warnings);
            assert_eq!(field(&target, "where"), "Room 2");
            let set = commitments::load(&v);
            assert_eq!(set.confirmed[0].meets, vec![meet(&["tue", "thu"], t(9, 30), t(10, 45))]);
            assert_eq!(field(&archived(&v, &card), "status"), "executed");
        }

        #[test]
        fn rejecting_a_successor_change_writes_the_successors_marker() {
            let v = vault();
            let target = cs100_note(&v);
            let before = read(&target);
            let card = emit(
                &v,
                &[],
                &[change(
                    mapping(&[("meets", tue_thu_value()), ("source_uid", s("gcal-series:cs100-new"))]),
                    mapping(&[("meets", mwf_value()), ("source_uid", s(CS100))]),
                )],
            );
            let result = decide(&v, &card, "rejected");
            assert!(result.warnings.is_empty(), "{:?}", result.warnings);
            assert_eq!(read(&target), before);
            let set = commitments::load(&v);
            assert!(set.declined.contains("gcal-series:cs100-new"));
            assert!(!set.declined.contains(CS100));
            assert_eq!(field(&archived(&v, &card), "status"), "rejected");
        }

        #[test]
        fn rejecting_a_plain_change_writes_nothing() {
            let v = vault();
            let target = cs100_note(&v);
            let before = read(&target);
            let card = emit(
                &v,
                &[],
                &[change(mapping(&[("until", s("2026-12-04"))]), mapping(&[("until", Value::Null)]))],
            );
            decide(&v, &card, "rejected");
            assert_eq!(read(&target), before);
            assert_eq!(files(&v, "commitments"), vec!["cs-100.md"]);
        }

        #[test]
        fn a_change_card_may_write_only_the_change_fields() {
            let v = vault();
            let target = cs100_note(&v);
            let before = read(&target);
            let card = emit(
                &v,
                &[],
                &[change(mapping(&[("level", s("optional"))]), mapping(&[("level", s("hard"))]))],
            );
            let result = decide(&v, &card, "approved");
            assert_eq!(result.warnings.len(), 1, "{:?}", result.warnings);
            assert_eq!(read(&target), before);
            assert_eq!(field(&archived(&v, &card), "status"), "refused");
        }

        // ---- withdrawal, by card shape (plan review C1) ----

        fn cs100_series(cal: &str) -> Series {
            series(CS100, cal, "CS 100", &["mon", "wed", "fri"], t(12, 0), t(12, 50))
        }

        fn is_pending(card: &Path) -> bool {
            card.exists() && field(card, "status") == "pending"
        }

        #[test]
        fn a_pending_card_whose_series_left_the_file_is_withdrawn_superseded() {
            let v = vault();
            let pending = card(&v, cs100());
            let snoozed = card(&v, club("gcal-series:chess"));
            let literals = vec![
                ("status".to_string(), "snoozed".to_string()),
                ("snooze_until".to_string(), "2026-09-30".to_string()),
            ];
            let mut journal = Journal::new(&v);
            write_literals(&v, &rel_path(&v, &snoozed), &literals, &default_ctx(), &mut journal, &WriteOpts::default())
                .unwrap();
            // The calendar was read, and neither key is under it any more.
            let file = file_of(vec![], &[GOOGLE]);
            let (withdrawn, warnings) = withdraw(&v, &file);
            assert!(warnings.is_empty(), "{warnings:?}");
            assert_eq!(withdrawn.len(), 2, "{withdrawn:?}");
            for card in [&pending, &snoozed] {
                assert!(!card.exists());
                assert_eq!(field(&archived(&v, card), "status"), "superseded");
            }
            assert!(files(&v, "commitments").is_empty());
        }

        #[test]
        fn a_proposal_card_survives_an_empty_series_file() {
            // No calendar at all (a file never written, or unreadable): nothing is known to have
            // left, so nothing is withdrawn.
            let v = vault();
            let pending = card(&v, cs100());
            let (withdrawn, _) = withdraw(&v, &SeriesFile::default());
            assert!(withdrawn.is_empty());
            assert!(is_pending(&pending));
        }

        #[test]
        fn a_proposal_card_is_withdrawn_once_a_confirmed_note_has_its_key_or_signature() {
            let file = file_of(
                vec![
                    cs100_series(GOOGLE),
                    series("gcal-series:chess", GOOGLE, "Chess Club", &["tue"], t(18, 0), t(19, 0)),
                ],
                &[GOOGLE],
            );
            // Its key: the note another desktop confirmed.
            let v = vault();
            let a = card(&v, cs100());
            note(
                &v,
                "cs-100.md",
                "type: commitment\nkind: class\ntitle: \"CS 100\"\n\
                 meets: [{days: [tue], start: \"09:00\", end: \"10:00\"}]\n\
                 source_uid: \"gcal-series:cs100\"\nstatus: confirmed",
            );
            assert_eq!(withdraw(&v, &file).0.len(), 1);
            assert_eq!(field(&archived(&v, &a), "status"), "superseded");

            // Its key on a decline marker.
            let m = vault();
            let b = card(&m, cs100());
            let mut journal = Journal::new(&m);
            commitments::create_marker(&m, CS100, &default_ctx(), &mut journal).unwrap();
            assert_eq!(withdraw(&m, &file).0.len(), 1);
            assert_eq!(field(&archived(&m, &b), "status"), "superseded");

            // Its signature (M15): a hand-written note with no key, the same meeting.
            let w = vault();
            let c = card(&w, club("gcal-series:chess"));
            note(
                &w,
                "chess.md",
                "type: commitment\nkind: club\ntitle: \"Chess Club\"\n\
                 meets: [{days: [tue], start: \"18:00\", end: \"19:00\"}]\nstatus: confirmed",
            );
            assert_eq!(withdraw(&w, &file).0.len(), 1);
            assert_eq!(field(&archived(&w, &c), "status"), "superseded");
        }

        #[test]
        fn a_proposal_card_whose_key_another_calendar_still_holds_survives() {
            // I2: filed from the Google record; only the ICS feed holds the key now.
            let v = vault();
            let pending = card(&v, cs100());
            let file = file_of(vec![cs100_series("personal")], &[GOOGLE, "personal"]);
            let (withdrawn, warnings) = withdraw(&v, &file);
            assert!(withdrawn.is_empty() && warnings.is_empty(), "{withdrawn:?} {warnings:?}");
            assert!(is_pending(&pending));
        }

        #[test]
        fn a_pending_change_card_survives_the_next_rank() {
            let v = vault();
            cs100_note(&v);
            let pending = emit(
                &v,
                &[],
                &[change(mapping(&[("meets", tue_thu_value())]), mapping(&[("meets", mwf_value())]))],
            );
            // Its key is the note's, which a confirmed note always has, and the series is gone.
            let file = file_of(vec![], &[GOOGLE]);
            assert!(withdraw(&v, &file).0.is_empty());
            assert!(is_pending(&pending));
        }

        #[test]
        fn a_pending_ended_card_survives_the_next_rank() {
            let v = vault();
            cs100_note(&v);
            let pending = emit(
                &v,
                &[],
                &[change(mapping(&[("until", s("2026-12-04"))]), mapping(&[("until", Value::Null)]))],
            );
            let mut file = file_of(vec![], &[GOOGLE]);
            file.ended.insert(
                CS100.to_string(),
                commitments::Ended {
                    calendar: GOOGLE.into(),
                    dropped: TODAY,
                    last_instance: Some(Date::constant(2026, 12, 4)),
                    until: None,
                },
            );
            assert!(withdraw(&v, &file).0.is_empty());
            assert!(is_pending(&pending));
        }

        #[test]
        fn a_pending_window_card_survives_the_next_rank() {
            let v = vault();
            let pending = card(&v, window());
            let file = file_of(vec![], &[GOOGLE]);
            assert!(withdraw(&v, &file).0.is_empty());
            assert!(is_pending(&pending));
        }

        #[test]
        fn a_window_card_is_withdrawn_once_a_planning_day_note_exists() {
            let file = file_of(vec![], &[GOOGLE]);
            let v = vault();
            let a = card(&v, window());
            note(
                &v,
                "planning-day.md",
                "type: commitment\nkind: planning-day\nstatus: confirmed\n\
                 window: [{days: [mon], start: \"07:00\", end: \"21:00\"}]",
            );
            assert_eq!(withdraw(&v, &file).0.len(), 1);
            assert_eq!(field(&archived(&v, &a), "status"), "superseded");

            // … and once the `window` marker does.
            let m = vault();
            let b = card(&m, window());
            let mut journal = Journal::new(&m);
            commitments::create_marker(&m, WINDOW_MARKER, &default_ctx(), &mut journal).unwrap();
            assert_eq!(withdraw(&m, &file).0.len(), 1);
            assert_eq!(field(&archived(&m, &b), "status"), "superseded");
        }

        #[test]
        fn a_change_card_is_withdrawn_when_the_note_was_edited_since() {
            let v = vault();
            let target = cs100_note(&v);
            let card = emit(
                &v,
                &[],
                &[change(mapping(&[("meets", tue_thu_value())]), mapping(&[("meets", mwf_value())]))],
            );
            let edited = read(&target).replace("start: \"12:00\", end: \"12:50\"", "start: \"13:00\", end: \"13:50\"");
            pystr::write_text(&target, &edited).unwrap();
            let file = file_of(vec![], &[GOOGLE]);
            assert_eq!(withdraw(&v, &file).0.len(), 1);
            assert_eq!(field(&archived(&v, &card), "status"), "superseded");
            assert_eq!(read(&target), edited);
        }

        #[test]
        fn a_change_card_is_withdrawn_when_its_target_is_gone() {
            let v = vault();
            let target = cs100_note(&v);
            let card = emit(
                &v,
                &[],
                &[change(mapping(&[("until", s("2026-12-04"))]), mapping(&[("until", Value::Null)]))],
            );
            std::fs::remove_file(&target).unwrap();
            let file = file_of(vec![], &[GOOGLE]);
            assert_eq!(withdraw(&v, &file).0.len(), 1);
            assert_eq!(field(&archived(&v, &card), "status"), "superseded");
        }

        #[test]
        fn a_card_the_student_deleted_is_left_where_it_is() {
            // Carry-forward 4: the app's delete archives the card as it stood, which closes its
            // question like an answer (P11); withdrawal reads only `approvals/`, so it never
            // rewrites that card as `superseded` (which would re-open the question).
            let v = vault();
            let pending = card(&v, cs100());
            let console = WriteContext::new("quinn", "dashboard");
            let mut journal = Journal::new(&v);
            delete(&v, &rel_path(&v, &pending), &console, &mut journal).unwrap();
            let file = file_of(vec![], &[GOOGLE]);
            assert!(withdraw(&v, &file).0.is_empty());
            assert_eq!(field(&archived(&v, &pending), "status"), "pending");
        }

        #[test]
        fn withdrawal_leaves_other_kinds_alone() {
            let v = vault();
            super::proposal(&v, "p.md", PENDING, "");
            let file = file_of(vec![], &[GOOGLE]);
            assert!(withdraw(&v, &file).0.is_empty());
            assert!(v.join("approvals").join("p.md").exists());
        }

        // ---- fix round 1 ----

        /// One rank's commitment passes, as P16 will run them: withdraw, detect, file.
        fn rank_once(vault: &Path, file: &SeriesFile) {
            withdraw(vault, file);
            let set = commitments::load(vault);
            let (codes, _) = commitments::Codes::load(vault);
            let fresh: BTreeSet<String> = file.calendars.keys().cloned().collect();
            let mut journal = Journal::new(vault);
            let (changes, _) = commitments::detect_changes(file, &set, &codes, &[], &fresh, TODAY, &mut journal);
            let ctx = WriteContext::new("agent:rank", "cli");
            commitments::emit_checks(vault, &[], &changes, TODAY, 15, &ctx, &mut journal);
        }

        #[test]
        fn a_hand_written_list_where_is_read_as_absent_and_never_refiled() {
            // I1: `load` reads a collection `where:` as absent, so P12 writes `was: {where: null}`;
            // the staleness test must read it the same way, or the card loops file → withdraw → file.
            let v = vault();
            let target = note(
                &v,
                "cs-100.md",
                "type: commitment\nkind: class\ntitle: \"CS 100\"\ncourse: cs-100\n\
                 meets: [{days: [mon, wed, fri], start: \"12:00\", end: \"12:50\"}]\n\
                 where: [Room 1, Room 2]\nsource_uid: \"gcal-series:cs100\"\nstatus: confirmed",
            );
            let mut live = series(CS100, GOOGLE, "CS 100", &["mon", "wed", "fri"], t(12, 0), t(12, 50));
            live.where_ = Some("Room 2".into());
            let file = file_of(vec![live], &[GOOGLE]);

            rank_once(&v, &file);
            rank_once(&v, &file);
            let pending = files(&v, "approvals");
            assert_eq!(pending.len(), 1, "{pending:?}");
            assert!(files(&v, "archive").is_empty(), "{:?}", files(&v, "archive"));
            let card = v.join("approvals").join(&pending[0]);
            assert_eq!(read(&card).lines().filter(|l| l.starts_with("was: {where: null}")).count(), 1, "{}", read(&card));

            let result = decide(&v, &card, "approved");
            assert!(result.warnings.is_empty(), "{:?}", result.warnings);
            assert_eq!(field(&target, "where"), "Room 2");
            assert_eq!(field(&archived(&v, &card), "status"), "executed");
        }

        /// The console's approval, then the settlement, then a crash before the stamp: the card is
        /// left `approved` with its write done.
        fn settle_then_crash(vault: &Path, card: &Path) {
            let console = WriteContext::new("quinn", "dashboard");
            let mut journal = Journal::new(vault);
            let literals = vec![("status".to_string(), "approved".to_string())];
            write_literals(vault, &rel_path(vault, card), &literals, &console, &mut journal, &WriteOpts::default())
                .unwrap();
            let settled = commitments::settle_approved(vault, &front(card), TODAY, &default_ctx(), &mut journal)
                .unwrap();
            assert_eq!(settled, commitments::Settled::Executed);
        }

        #[test]
        fn a_change_settled_before_a_crash_is_stamped_executed_on_the_rerun() {
            let v = vault();
            let target = cs100_note(&v);
            let card = emit(
                &v,
                &[],
                &[change(mapping(&[("meets", tue_thu_value())]), mapping(&[("meets", mwf_value())]))],
            );
            settle_then_crash(&v, &card);
            let after = read(&target);
            let result = run(&v);
            assert!(result.warnings.is_empty(), "{:?}", result.warnings);
            assert_eq!(read(&target), after);
            assert_eq!(field(&archived(&v, &card), "status"), "executed");
            assert_eq!(result.executed, vec![stem_of(&card)]);
        }

        #[test]
        fn a_note_created_before_a_crash_is_stamped_executed_on_the_rerun() {
            let v = vault();
            let card = card(&v, cs100());
            settle_then_crash(&v, &card);
            let result = run(&v);
            assert!(result.warnings.is_empty(), "{:?}", result.warnings);
            assert_eq!(files(&v, "commitments"), vec!["cs-100.md"]);
            assert_eq!(field(&archived(&v, &card), "status"), "executed");
        }

        #[test]
        fn a_planning_day_created_before_a_crash_is_stamped_executed_on_the_rerun() {
            let v = vault();
            let card = card(&v, window());
            settle_then_crash(&v, &card);
            let result = run(&v);
            assert!(result.warnings.is_empty(), "{:?}", result.warnings);
            assert_eq!(files(&v, "commitments"), vec!["planning-day.md"]);
            assert_eq!(field(&archived(&v, &card), "status"), "executed");
        }

        #[test]
        fn a_note_confirmed_before_the_approval_is_still_refused() {
            // The journal's create record predates the student's approval: another answer.
            let v = vault();
            let card = card(&v, cs100());
            let mut journal = Journal::new(&v);
            let mut map = Mapping::new();
            for (k, val) in [
                ("kind", s("class")),
                ("title", s("CS 100")),
                ("meets", tue_thu_value()),
            ] {
                map.insert(s(k), val);
            }
            commitments::create_confirmed(&v, &map, CS100, TODAY, &default_ctx(), &mut journal).unwrap();
            let result = decide(&v, &card, "approved");
            assert_eq!(result.warnings.len(), 1, "{:?}", result.warnings);
            assert_eq!(field(&archived(&v, &card), "status"), "refused");
        }

        #[test]
        fn a_stamped_card_left_in_approvals_by_a_crash_is_archived_quietly() {
            // m1 (controller ruling): the stamp landed, the move did not.
            for status in ["refused", "superseded"] {
                let v = vault();
                let card = card(&v, cs100());
                let literals = vec![("status".to_string(), status.to_string())];
                let mut journal = Journal::new(&v);
                write_literals(&v, &rel_path(&v, &card), &literals, &default_ctx(), &mut journal, &WriteOpts::default())
                    .unwrap();
                let result = run(&v);
                assert!(result.warnings.is_empty(), "{status}: {:?}", result.warnings);
                assert!(!card.exists());
                assert_eq!(field(&archived(&v, &card), "status"), status);
            }
        }

        #[test]
        fn a_refused_amend_left_in_approvals_still_warns() {
            // m1 is for commitment-check cards only; the amend path is unchanged.
            let v = vault();
            super::proposal(&v, "a.md", &AMEND.replace("status: pending", "status: refused"), "");
            let result = run(&v);
            assert_eq!(result.warnings, vec!["unknown status: a.md".to_string()]);
        }

        #[test]
        fn a_proposal_card_whose_series_ended_is_withdrawn() {
            // m3 (controller ruling): the key is in the file's `ended` map.
            let v = vault();
            let pending = card(&v, cs100());
            let mut file = SeriesFile::default();
            file.ended.insert(
                CS100.to_string(),
                commitments::Ended { calendar: GOOGLE.into(), dropped: TODAY, last_instance: None, until: None },
            );
            assert_eq!(withdraw(&v, &file).0.len(), 1);
            assert_eq!(field(&archived(&v, &pending), "status"), "superseded");
        }

        #[test]
        fn a_proposal_card_whose_series_has_finished_is_withdrawn() {
            // m3 (the review's case): the series now ends before today, so P9 no longer proposes it.
            let v = vault();
            let pending = card(&v, cs100());
            let mut done = cs100_series(GOOGLE);
            done.until = Some(Date::constant(2026, 8, 19));
            let file = file_of(vec![done], &[GOOGLE]);
            assert_eq!(withdraw(&v, &file).0.len(), 1);
            assert_eq!(field(&archived(&v, &pending), "status"), "superseded");
        }

        #[test]
        fn rejecting_a_card_whose_key_is_confirmed_writes_no_marker() {
            // m5: another desktop confirmed it; a marker beside the note would say the opposite.
            let v = vault();
            let card = card(&v, cs100());
            cs100_note(&v);
            let result = decide(&v, &card, "rejected");
            assert!(result.warnings.is_empty(), "{:?}", result.warnings);
            assert_eq!(files(&v, "commitments"), vec!["cs-100.md"]);
        }

        #[test]
        fn amendable_folders_is_unchanged() {
            assert_eq!(AMENDABLE_FOLDERS, ["tasks", "courses"]);
        }
    }

    // The commitment-ask card (phase-2 spec §5, parent §5.3): approve with an answer → the class
    // note; an invalid answer → back to pending with the warning; reject → the card:<slug>
    // marker; withdrawal once the course is covered. Every course and time is invented.
    mod commitment_ask {
        use super::*;
        use crate::commitments::{self, Level, Meet, Proposal};
        use jiff::civil::Time;

        const NAME: &str = "commitment-ask-when-does-bui-100-meet.md";

        fn ask_vault() -> (PathBuf, PathBuf) {
            let v = vault();
            let courses = v.join("courses");
            std::fs::create_dir_all(&courses).unwrap();
            pystr::write_text(&courses.join("bui-100.md"), "---\ntitle: \"BUI 100\"\nslug: bui-100\ncode: \"BUI 100\"\nstatus: active\n---\n").unwrap();
            let card = proposal(
                &v,
                NAME,
                "id: appr_00000000a1\ntype: approval\nkind: commitment-ask\ntitle: When does BUI 100 meet?\nstatus: pending\n\
                 source_uid: card:bui-100\ncourse: bui-100\nproposed_at: 2026-08-20\nfirst_proposed_at: 2026-08-20\n\
                 expires: null\nsnooze_until: null\ncreated_by: agent:commitments",
                "Invented.\n",
            );
            (v, card)
        }

        /// What `answer_card` does before it approves: the answer, as the student.
        fn answer(vault: &Path, card: &Path, literal: &str) {
            let console = WriteContext::new("quinn", "dashboard");
            let literals = vec![("answer_meets".to_string(), literal.to_string())];
            write_literals(vault, &rel_path(vault, card), &literals, &console, &mut Journal::new(vault), &WriteOpts::default()).unwrap();
        }

        /// What `decide_inner` does: the status as the student, then the pass as `agent:approvals`.
        fn decide(vault: &Path, card: &Path, status: &str) -> ApprovalsResult {
            let console = WriteContext::new("quinn", "dashboard");
            let mut journal = Journal::new(vault);
            let literals = vec![("status".to_string(), status.to_string())];
            write_literals(vault, &rel_path(vault, card), &literals, &console, &mut journal, &WriteOpts::default()).unwrap();
            process_approvals(vault, TODAY, now(), &default_ctx(), &mut journal)
        }

        #[test]
        fn a_valid_answer_creates_the_class_note_and_archives_the_card_executed() {
            let (v, card) = ask_vault();
            answer(&v, &card, "[{days: [mon, wed], start: '14:00', end: '15:15'}]");
            let result = decide(&v, &card, "approved");
            assert!(result.warnings.is_empty(), "{:?}", result.warnings);
            assert_eq!(result.executed, ["commitment-ask-when-does-bui-100-meet"]);
            let set = commitments::load(&v);
            assert_eq!(set.confirmed.len(), 1);
            let note = &set.confirmed[0];
            assert_eq!(note.path, PathBuf::from("commitments/bui-100.md"));
            assert_eq!((note.kind.as_str(), note.level, note.course.as_deref()), ("class", Level::Hard, Some("bui-100")));
            assert_eq!(note.source_uid.as_deref(), Some("card:bui-100"));
            assert_eq!(note.meets, vec![Meet { days: vec!["mon", "wed"], start: Time::constant(14, 0, 0, 0), end: Time::constant(15, 15, 0, 0) }]);
            assert!(!card.exists());
            assert!(read(&v.join("archive").join(NAME)).contains("status: executed"));
        }

        #[test]
        fn an_invalid_answer_goes_back_to_pending_with_the_warning() {
            let (v, card) = ask_vault();
            answer(&v, &card, "[{days: [mon], start: '15:00', end: '14:00'}]");
            let result = decide(&v, &card, "approved");
            assert_eq!(result.warnings.len(), 1, "{:?}", result.warnings);
            assert!(result.warnings[0].starts_with(&format!("{NAME}: answer_meets entry mon")), "{:?}", result.warnings);
            assert!(card.exists(), "the card stays in approvals/");
            assert!(read(&card).contains("status: pending"));
            assert!(!v.join("commitments").exists());
        }

        #[test]
        fn no_answer_at_all_also_goes_back_to_pending() {
            let (v, card) = ask_vault();
            let result = decide(&v, &card, "approved");
            assert!(result.warnings[0].starts_with(&format!("{NAME}: no answer_meets")), "{:?}", result.warnings);
            assert!(read(&card).contains("status: pending"));
        }

        #[test]
        fn rejecting_writes_the_card_marker_and_archives() {
            let (v, card) = ask_vault();
            decide(&v, &card, "rejected");
            assert!(commitments::load(&v).declined.contains("card:bui-100"));
            assert!(!card.exists() && v.join("archive").join(NAME).exists());
        }

        fn class_for_bui() -> Proposal {
            Proposal {
                kind: "class".into(),
                level: Level::Hard,
                title: "BUI 100".into(),
                course: Some("bui-100".into()),
                meets: vec![Meet { days: vec!["tue"], start: Time::constant(9, 0, 0, 0), end: Time::constant(10, 0, 0, 0) }],
                where_: None,
                from: None,
                until: None,
                source_uid: "gcal-series:bui".into(),
            }
        }

        #[test]
        fn a_pending_ask_is_withdrawn_once_a_class_proposal_for_its_course_appears() {
            let (v, card) = ask_vault();
            let set = commitments::load(&v);
            let none = withdraw_asks(&v, &[], &set, &default_ctx(), &mut Journal::new(&v));
            assert!(none.withdrawn.is_empty());
            let done = withdraw_asks(&v, &[class_for_bui()], &set, &default_ctx(), &mut Journal::new(&v));
            assert_eq!(done.withdrawn, ["commitment-ask-when-does-bui-100-meet"]);
            assert_eq!(done.pending, 1);
            assert!(!card.exists());
            assert!(read(&v.join("archive").join(NAME)).contains("status: superseded"));
        }

        /// Review finding 3: a hand-written class note with no `course:` whose title leads with
        /// the course's code covers the course (Q2-c), so the pending ask is withdrawn.
        #[test]
        fn a_title_only_class_note_withdraws_the_ask() {
            let (v, card) = ask_vault();
            let folder = v.join("commitments");
            std::fs::create_dir_all(&folder).unwrap();
            pystr::write_text(
                &folder.join("bui.md"),
                "---\nid: cmt_00000000d1\ntype: commitment\nkind: class\ntitle: \"BUI 100 Lecture\"\n\
                 meets: [{days: [tue], start: \"09:00\", end: \"10:00\"}]\nstatus: confirmed\n---\n\nMine.\n",
            )
            .unwrap();
            let set = commitments::load(&v);
            let done = withdraw_asks(&v, &[], &set, &default_ctx(), &mut Journal::new(&v));
            assert_eq!(done.withdrawn, ["commitment-ask-when-does-bui-100-meet"]);
            assert!(!card.exists());
        }
    }
}
