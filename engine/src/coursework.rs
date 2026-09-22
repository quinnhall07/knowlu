//! Coursework sync — the types both source parsers produce, and (from Task 12) the sync that
//! consumes them. Port of `engine/coursework.py`.
//!
//! Wave 6 of the Rust port. This file currently holds only the two types that `zybooks` and `vhl`
//! both need; `sync_coursework`, `collect` and the CLI entry point land with Task 12. That split
//! mirrors the Python module graph rather than working around it: `coursework <-> zybooks` and
//! `coursework <-> vhl` are two of the four import cycles the crate is one crate in order to keep
//! (see `Cargo.toml`).

use std::fmt;
use std::path::{Path, PathBuf};
use std::sync::LazyLock;

use jiff::civil::{Date, DateTime};
use jiff::tz::TimeZone;
use regex::Regex;
use serde_yaml_ng::{Mapping, Value as Yaml};

use crate::ingest::{existing_by_uid, format_due, load_seen, record_seen, Due};
use crate::journal::Journal;
use crate::pystr::{self, yaml_str};
use crate::write::{write_literals, WriteContext, WriteOpts};

/// One task note's worth of coursework, already fully resolved.
///
/// Both source parsers produce these; `sync_coursework` consumes them and knows nothing about
/// zyBooks or VHL. Every field is derived from payload plus config — there is no judgment left to
/// make by the time one of these exists, which is what keeps coursework ingest inside the
/// engine's no-inference rule.
#[derive(Debug, Clone, PartialEq)]
pub struct Assignment {
    pub uid: String,
    pub slug: String,
    pub title: String,
    /// Naive local time, exactly as Python's `datetime` here is naive. The conversion from the
    /// vendor's UTC stamp happens in the parser; nothing downstream re-applies a zone.
    pub due: DateTime,
    /// Python's `str | None`. **An empty string is not the same as `None` to the note template**
    /// — Python writes `course: null` for both, because `if item.course` is falsy for `""`. The
    /// parsers pass whatever the config held, so the emptiness check belongs at the template.
    pub course: Option<String>,
    pub effort_hours: f64,
    pub effort_confidence: String,
    pub effort_source: String,
    pub importance: i64,
    pub importance_reason: String,
    pub progress: i64,
    pub created_by: String,
    pub body: String,
}

/// Why one coursework source produced nothing.
///
/// Python raises two distinguishable things here and `collect` catches them separately:
/// `NotLoggedIn` (its own class, defined beside `Assignment` for exactly this reason) and
/// everything else, which reaches the bare `except Exception`. The distinction is not cosmetic —
/// it picks between two different warning lines, and *"session invalid"* is the one that tells
/// Quinn a password needs re-entering rather than that a vendor is down.
///
/// `Display` is the payload string alone, because Python interpolates `{err}` into those warning
/// lines and `str(exc)` is the message it was constructed with.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SourceError {
    /// Python's `NotLoggedIn`: the fetched page or payload is not an authenticated one.
    ///
    /// Kept distinct from "parsed zero buckets" because the two need different responses: this
    /// one means the credentials or session are the problem, and it must never be mistaken for a
    /// semester with no homework in it.
    NotLoggedIn(String),
    /// Everything Python's `except Exception` in `collect` would catch — a transport failure, a
    /// rejected signin, a payload shaped in a way the parser cannot read.
    Failed(String),
}

impl fmt::Display for SourceError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            SourceError::NotLoggedIn(message) | SourceError::Failed(message) => {
                f.write_str(message)
            }
        }
    }
}

impl std::error::Error for SourceError {}

/// `^\s*(?:(?P<h>\d+)\s*h)?\s*(?:(?P<m>\d+)\s*m)?\s*$`, case-insensitive.
///
/// Both groups are optional, so the pattern also matches the empty string — which is why the
/// caller has to check that at least one of them fired, not merely that the match succeeded.
static DURATION: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"(?i)^\s*(?:(\d+)\s*h)?\s*(?:(\d+)\s*m)?\s*$").unwrap());

/// `"1h 22m"` -> `1.37`, `"27m"` -> `0.45`, `"3h 4m"` -> `3.07`.
///
/// `Err` on anything it cannot read. Returning `0.0` for an unparseable estimate would be worse
/// than failing: a zero-effort task fits in any capacity gap and so never shows up as work that
/// needs a slot.
///
/// The error text spells the input the way Python's `{text!r}` does for the ordinary case. A value
/// containing a quote character would render differently in the two engines; it reaches a warning
/// line only, and no vendor emits one.
pub fn parse_duration_hours(text: &str) -> Result<f64, String> {
    let unparseable = || format!("unparseable duration: '{text}'");
    let captures = DURATION.captures(text).ok_or_else(unparseable)?;
    let group = |index: usize| -> Option<f64> {
        captures
            .get(index)
            .and_then(|m| m.as_str().parse::<f64>().ok())
    };
    let (hours, minutes) = (group(1), group(2));
    if hours.is_none() && minutes.is_none() {
        return Err(unparseable());
    }
    let total = hours.unwrap_or(0.0) + minutes.unwrap_or(0.0) / 60.0;
    Ok((total * 100.0).round_ties_even() / 100.0)
}

// ---------------------------------------------------------------------------------------------
// config
// ---------------------------------------------------------------------------------------------

pub const DEFAULT_TZ: &str = "America/Chicago";

/// Read `config/ingest.yaml`. Absent is silent; unreadable is a warning.
///
/// **The `Err` arm is load-bearing and matches a Python shape that looks like an oversight.**
/// `load_coursework_config` catches only `(yaml.YAMLError, OSError)`, so a config file that is not
/// valid UTF-8 raises `UnicodeDecodeError` straight *through* it and into `main`'s outer handler,
/// where it becomes `coursework pass failed (...)` and still lands a WARN line. Collapsing it into
/// the warning list here would be the tidier design and the wrong port: `main`'s try/except is
/// what makes "never crash the run" hold for a broken config file, and this is the only input that
/// exercises it.
pub fn load_coursework_config(vault: &Path) -> Result<(Mapping, Vec<String>), SourceError> {
    let path = vault.join("config").join("ingest.yaml");
    if !path.exists() {
        return Ok((Mapping::new(), Vec::new()));
    }
    let bytes = match std::fs::read(&path) {
        Ok(bytes) => bytes,
        // OSError.
        Err(err) => return Ok((Mapping::new(), vec![format!("config unreadable: {err}")])),
    };
    let text = String::from_utf8(bytes).map_err(|err| {
        // UnicodeDecodeError: neither yaml.YAMLError nor OSError, so Python does not catch it here.
        SourceError::Failed(format!("{err}"))
    })?;
    match serde_yaml_ng::from_str::<serde_yaml_ng::Value>(&pystr::universal_newlines(&text)) {
        Ok(serde_yaml_ng::Value::Mapping(map)) => Ok((map, Vec::new())),
        // `yaml.safe_load(...) or {}` — a scalar or empty document is an empty config.
        Ok(_) => Ok((Mapping::new(), Vec::new())),
        Err(err) => Ok((Mapping::new(), vec![format!("config unreadable: {err}")])),
    }
}

/// `ZoneInfo(config.get("timezone", DEFAULT_TZ))`, warning and falling back on anything unknown.
pub fn resolve_timezone(config: &Mapping, warnings: &mut Vec<String>) -> TimeZone {
    let name = match crate::yaml::get(config, "timezone") {
        // `.get(key, default)` returns the value even when it is null, and `ZoneInfo(None)` then
        // raises — which is why a `timezone: ~` warns rather than silently defaulting.
        Some(value) => crate::yaml::text(value),
        None => Some(DEFAULT_TZ.to_string()),
    };
    if let Some(name) = &name {
        if let Ok(tz) = TimeZone::get(name) {
            return tz;
        }
    }
    let shown = match &name {
        Some(text) => format!("'{text}'"),
        None => "None".to_string(),
    };
    warnings.push(format!(
        "bad timezone {shown} (no time zone found with key {}); using {DEFAULT_TZ}",
        name.clone().unwrap_or_else(|| "None".to_string())
    ));
    TimeZone::get(DEFAULT_TZ).unwrap_or(TimeZone::UTC)
}

// ---------------------------------------------------------------------------------------------
// the sync
// ---------------------------------------------------------------------------------------------

const NOTE_TEMPLATE: &str = "---\n\
title: {title}\n\
course: {course}\n\
domain: school\n\
due: {due}\n\
effort_hours: {effort_hours}\n\
effort_confidence: {effort_confidence}\n\
effort_source: {effort_source}\n\
importance: {importance}\n\
importance_reason: {importance_reason}\n\
status: active\n\
progress: {progress}\n\
created_by: {created_by}\n\
source_uid: {uid}\n\
---\n\
\n\
{body}\n";

/// Python's `f"{value}"` on a float, which keeps the decimal point on a whole number — `3.0`,
/// never `3`. The frontmatter value depends on it: `effort_hours: 3` and `effort_hours: 3.0` load
/// as different YAML types.
fn py_float(value: f64) -> String {
    format!("{value:?}")
}

/// `json.dumps(text, ensure_ascii=False)` — the quoted form written into frontmatter.
///
/// `ensure_ascii=False` is the whole point: VHL titles carry an em dash, and the `—` escape
/// is what Quinn would see on opening the note.
fn json_quoted(text: &str) -> String {
    crate::ledger::dumps_value(&serde_json::Value::String(text.to_string()))
}

/// `coerce_datetime(existing) != coerce_datetime(new_due)`, with an unreadable value counting as
/// changed.
fn due_changed(existing: Option<&Yaml>, new_due: &str) -> bool {
    let incoming = crate::models::coerce_datetime(Some(&Yaml::String(new_due.to_string())));
    match (crate::models::coerce_datetime(existing), incoming) {
        (Ok(left), Ok(right)) => left != right,
        _ => true,
    }
}

/// `float(existing) != float(new_effort)`. Missing or unreadable counts as changed, because
/// writing the vendor's number is the repair.
fn effort_changed(existing: Option<&Yaml>, new_effort: f64) -> bool {
    match existing.and_then(crate::yaml::f64_of) {
        Some(current) => current != new_effort,
        None => true,
    }
}

/// Create or surgically update task notes. Never deletes, never rewrites wholesale.
///
/// The three fields this may touch are `title`, `due` and `effort_hours` — and `effort_hours` only
/// when **both** the note's and the incoming item's `effort_source` is `vendor`.
///
/// **`progress` is never written after creation.** Quinn ticks it forward by hand, and a vendor's
/// `percentage_complete` seeds it once. Gating the effort refresh on both sides is what stops an
/// incoming `effort_source: inferred` item that happens to share a uid from writing a number no
/// vendor produced into a note whose `effort_confidence: high` then certifies it.
pub fn sync_coursework(
    assignments: &[Assignment],
    vault: &Path,
    today: Option<Date>,
    dry_run: bool,
    ctx: Option<&WriteContext>,
    journal: Option<&mut Journal>,
) -> Result<Vec<String>, SourceError> {
    let mut owned_journal;
    let journal: &mut Journal = match journal {
        Some(supplied) => supplied,
        None => {
            owned_journal = Journal::new(vault);
            &mut owned_journal
        }
    };
    let mut log: Vec<String> = Vec::new();
    let mut known = existing_by_uid(vault);
    let mut seen = load_seen(vault);
    let tasks_dir = vault.join("tasks");
    // `mkdir(exist_ok=True)`, without `parents=True`: a missing vault is an error, not something
    // to create on the way past.
    if let Err(err) = std::fs::create_dir(&tasks_dir) {
        if err.kind() != std::io::ErrorKind::AlreadyExists {
            return Err(SourceError::Failed(format!("{err}")));
        }
    }
    let stamp = today
        .unwrap_or_else(|| jiff::Zoned::now().date())
        .strftime("%Y-%m-%d")
        .to_string();

    for item in assignments {
        let item_ctx = match ctx {
            Some(supplied) => supplied.with_actor(&format!("agent:coursework.{}", item.created_by)),
            None => WriteContext::new(
                &format!("agent:coursework.{}", item.created_by),
                "cli",
            ),
        };
        let new_due = format_due(Due::DateTime(item.due));

        if let Some(path) = known.get(&item.uid).cloned() {
            let stem = path
                .file_stem()
                .map(|s| s.to_string_lossy().into_owned())
                .unwrap_or_default();
            let meta = match pystr::read_text(&path).ok().and_then(|text| {
                crate::models::split_frontmatter(&text)
                    .ok()
                    .map(|(meta, _)| meta)
            }) {
                Some(meta) => meta,
                None => {
                    // Known by uid but unreadable. Skipping outright is deliberate: falling
                    // through to creation would produce a duplicate.
                    log.push(format!("skipped (unreadable): {stem}"));
                    continue;
                }
            };
            let status = match crate::yaml::get(&meta, "status") {
                Some(value) => crate::yaml::text(value),
                None => Some("active".to_string()),
            };
            if !seen.contains(&item.uid) && !dry_run {
                record_seen(vault, &item.uid, &item.title, &stamp)
                    .map_err(|err| SourceError::Failed(format!("{err}")))?;
                seen.insert(item.uid.clone());
            }
            let archived = path
                .parent()
                .and_then(|p| p.file_name())
                .map(|n| n == "archive")
                .unwrap_or(false);
            if archived || status.as_deref() != Some("active") {
                continue;
            }

            let mut changes: Vec<(String, String)> = Vec::new();
            let existing_title = match crate::yaml::get(&meta, "title") {
                Some(value) => yaml_str(value),
                None => String::new(),
            };
            if existing_title != item.title {
                changes.push(("title".to_string(), json_quoted(&item.title)));
            }
            if due_changed(crate::yaml::get(&meta, "due"), &new_due) {
                changes.push(("due".to_string(), new_due.clone()));
            }
            let note_is_vendor = crate::yaml::get(&meta, "effort_source")
                .map(yaml_str)
                .unwrap_or_default()
                == "vendor";
            if note_is_vendor
                && item.effort_source == "vendor"
                && effort_changed(crate::yaml::get(&meta, "effort_hours"), item.effort_hours)
            {
                changes.push((
                    "effort_hours".to_string(),
                    py_float(item.effort_hours),
                ));
            }
            if changes.is_empty() {
                continue;
            }
            let mut names: Vec<String> = changes.iter().map(|(k, _)| k.clone()).collect();
            names.sort();
            let named = names.join(", ");
            if dry_run {
                log.push(format!("would update {stem}: {named}"));
                continue;
            }
            let target = crate::ids::rel(vault, &path);
            // `split_frontmatter`'s read above (`str.split("---", 2)`) tolerates a closing
            // delimiter line with trailing whitespace; the write path's `lines.index("---", 1)`
            // does not. A note can pass the read and still fail here. Unguarded, that error would
            // escape and silently drop every assignment still left in the batch.
            match write_literals(vault, &target, &changes, &item_ctx, journal, &WriteOpts::default())
            {
                Ok(_) => log.push(format!("updated {stem}: {named}")),
                Err(_) => log.push(format!("skipped (unwritable): {stem}")),
            }
            continue;
        }

        if seen.contains(&item.uid) {
            log.push(format!("skipped (deleted earlier): {}", item.title));
            continue;
        }

        let mut path = tasks_dir.join(format!("{}.md", item.slug));
        let mut suffix = 2;
        while path.exists() {
            path = tasks_dir.join(format!("{}-{suffix}.md", item.slug));
            suffix += 1;
        }
        let stem = path
            .file_stem()
            .map(|s| s.to_string_lossy().into_owned())
            .unwrap_or_default();
        if dry_run {
            log.push(format!("would create {stem} (due {new_due})"));
            continue;
        }
        let text = NOTE_TEMPLATE
            .replace("{title}", &json_quoted(&item.title))
            .replace(
                "{course}",
                // Quoted, not bare: a course containing a colon produces frontmatter that raises
                // ScannerError, and the loader then drops the note from ranking without saying so.
                &match item.course.as_deref().filter(|c| !c.is_empty()) {
                    Some(course) => json_quoted(course),
                    None => "null".to_string(),
                },
            )
            .replace("{due}", &new_due)
            .replace("{effort_hours}", &py_float(item.effort_hours))
            .replace("{effort_confidence}", &item.effort_confidence)
            .replace("{effort_source}", &item.effort_source)
            .replace("{importance}", &item.importance.to_string())
            .replace("{importance_reason}", &json_quoted(&item.importance_reason))
            .replace("{progress}", &item.progress.to_string())
            .replace("{created_by}", &item.created_by)
            .replace("{uid}", &json_quoted(&item.uid))
            .replace("{body}", &item.body);
        let target = crate::ids::rel(vault, &path);
        crate::write::create(vault, &target, &text, &item_ctx, journal, None)
            .map_err(|err| SourceError::Failed(format!("{err}")))?;
        log.push(format!("created {stem}"));
        known.insert(item.uid.clone(), path);
        record_seen(vault, &item.uid, &item.title, &stamp)
            .map_err(|err| SourceError::Failed(format!("{err}")))?;
    }
    Ok(log)
}

// ---------------------------------------------------------------------------------------------
// the sources
// ---------------------------------------------------------------------------------------------

/// One coursework source. The seam Python gets from passing `fetchers=` a dict of callables.
pub type Fetcher<'a> =
    &'a dyn Fn(&Mapping, &TimeZone, &mut Vec<String>) -> Result<Vec<Assignment>, SourceError>;

/// `str(cfg.get(key) or default)`.
fn cfg_str(cfg: &Mapping, key: &str, default: &str) -> String {
    match crate::yaml::get(cfg, key) {
        Some(value) if crate::pystr::yaml_truthy(value) => yaml_str(value),
        _ => default.to_string(),
    }
}

/// What `fetch_zybooks` does with one zybook code.
///
/// Extracted from the fetch because the decision is the part worth testing and the fetch is the
/// part that needs a live token. Python reaches the same logic by monkeypatching four module
/// globals; this is the same behaviour with no network in reach of a test.
#[derive(Debug, Clone, PartialEq)]
pub enum BookRouting {
    /// In `courses`: fetch it and parse it under this mapping.
    Mapped(Mapping),
    /// In `ignore`: skipped **silently**. `HowToUseZyBooks2` is zyBooks' own onboarding book --
    /// zero assignments, never coursework. A WARN here would fire on every single healthy run and
    /// train the reader to ignore `state/runner-log.md` altogether.
    Ignored,
    /// In neither: a genuinely new course appearing is something Quinn must be told about.
    Unmapped,
}

pub fn route_zybook(code: &str, courses: &Mapping, ignore: &[String]) -> BookRouting {
    match crate::yaml::get(courses, code) {
        // `if not mapping:` -- an empty mapping is falsy in Python and is not a course.
        Some(Yaml::Mapping(map)) if !map.is_empty() => BookRouting::Mapped(map.clone()),
        _ if ignore.iter().any(|entry| entry == code) => BookRouting::Ignored,
        _ => BookRouting::Unmapped,
    }
}

pub fn fetch_zybooks(
    cfg: &Mapping,
    tz: &TimeZone,
    warnings: &mut Vec<String>,
) -> Result<Vec<Assignment>, SourceError> {
    fetch_zybooks_into(cfg, tz, warnings, &CompletionSink::default())
}

/// [`fetch_zybooks`], also reporting each mapped book's completion figures into `sink`
/// (stream J T8) — read from the payload this fetch already holds.
#[cfg(windows)]
pub fn fetch_zybooks_into(
    cfg: &Mapping,
    tz: &TimeZone,
    warnings: &mut Vec<String>,
    sink: &CompletionSink,
) -> Result<Vec<Assignment>, SourceError> {
    let credential = crate::wincred::read_credential(&cfg_str(cfg, "credential_target", ""))
        .map_err(|err| SourceError::Failed(format!("{err}")))?;
    let (token, user_id) = crate::zybooks::signin(
        &credential.username,
        credential.password.expose(),
        None,
    )?;
    let courses = match crate::yaml::get(cfg, "courses") {
        Some(Yaml::Mapping(map)) => map.clone(),
        _ => Mapping::new(),
    };
    let ignore: Vec<String> = match crate::yaml::get(cfg, "ignore") {
        Some(Yaml::Sequence(items)) => items.iter().map(yaml_str).collect(),
        _ => Vec::new(),
    };
    let mut out: Vec<Assignment> = Vec::new();
    for code in crate::zybooks::fetch_zybook_codes(&token, user_id, None)? {
        let mapping = match route_zybook(&code, &courses, &ignore) {
            BookRouting::Mapped(map) => map,
            BookRouting::Ignored => {
                println!("zybook {code} ignored per config");
                continue;
            }
            BookRouting::Unmapped => {
                warnings.push(format!("zybook {code} not in config; skipped"));
                continue;
            }
        };
        let payload = crate::zybooks::fetch_assignments(&token, &code, None)?;
        sink.borrow_mut().extend(crate::zybooks::completions(&payload));
        out.extend(crate::zybooks::parse_assignments(
            &payload,
            &cfg_str(&mapping, "course", ""),
            &cfg_str(&mapping, "label", ""),
            cfg,
            tz,
            warnings,
        )?);
    }
    Ok(out)
}

pub fn fetch_vhl(
    cfg: &Mapping,
    tz: &TimeZone,
    warnings: &mut Vec<String>,
) -> Result<Vec<Assignment>, SourceError> {
    fetch_vhl_into(cfg, tz, warnings, &CompletionSink::default())
}

/// [`fetch_vhl`], also reporting every bucket's completion figure into `sink` (stream J T8).
#[cfg(windows)]
pub fn fetch_vhl_into(
    cfg: &Mapping,
    tz: &TimeZone,
    warnings: &mut Vec<String>,
    sink: &CompletionSink,
) -> Result<Vec<Assignment>, SourceError> {
    let credential = crate::wincred::read_credential(&cfg_str(cfg, "credential_target", ""))
        .map_err(|err| SourceError::Failed(format!("{err}")))?;
    let base = cfg_str(cfg, "base_url", "https://www.vhlcentral.com");
    let html = crate::vhl::login_and_fetch_dashboard(
        &credential.username,
        credential.password.expose(),
        &base,
        None,
    )?;
    let items = crate::vhl::parse_dashboard(&html, cfg, tz, warnings)?;
    sink.borrow_mut().extend(crate::vhl::completions(&html));
    Ok(items)
}

/// The credential store is Windows-only (spec §6.5), so the live fetchers are too. A cloud build
/// still compiles; it simply has no way to authenticate, and says so rather than pretending the
/// semester is empty.
#[cfg(not(windows))]
pub fn fetch_zybooks_into(
    _cfg: &Mapping,
    _tz: &TimeZone,
    _warnings: &mut Vec<String>,
    _sink: &CompletionSink,
) -> Result<Vec<Assignment>, SourceError> {
    Err(SourceError::Failed(
        "credential store unavailable on this platform".to_string(),
    ))
}

#[cfg(not(windows))]
pub fn fetch_vhl_into(
    _cfg: &Mapping,
    _tz: &TimeZone,
    _warnings: &mut Vec<String>,
    _sink: &CompletionSink,
) -> Result<Vec<Assignment>, SourceError> {
    Err(SourceError::Failed(
        "credential store unavailable on this platform".to_string(),
    ))
}

// ---------------------------------------------------------------------------------------------
// fetch on device, think in the cloud (C2 Task 7, D11, §4.3)
// ---------------------------------------------------------------------------------------------

/// The keys the redacted config may carry, per source. **An allowlist, not a denylist** — a config
/// grows keys and a denylist is a list of things somebody remembered. `credential_target` is the
/// one that matters (a portal password's address is not the server's business, D11), but so are
/// `base_url` and `enabled`, which are the device's own operational settings.
pub const REDACT_ALLOW_ZYBOOKS: [&str; 5] = ["courses", "ignore", "categories", "effort", "importance"];
pub const REDACT_ALLOW_VHL: [&str; 3] = ["sections", "importance", "importance_reason"];

/// What of a source's config may travel to `/ingest-coursework`.
pub fn redact(cfg: &Mapping, source: &str) -> Mapping {
    let allow: &[&str] = match source {
        "zybooks" => &REDACT_ALLOW_ZYBOOKS,
        "vhl" => &REDACT_ALLOW_VHL,
        _ => &[],
    };
    let mut out = Mapping::new();
    for key in allow {
        if let Some(value) = crate::yaml::get(cfg, key) {
            out.insert(Yaml::String((*key).to_string()), value.clone());
        }
    }
    out
}

/// A YAML mapping as the JSON the request body carries. Only the four scalar kinds and the two
/// containers — a YAML tag, an alias or a date in a coursework config is not something the parser
/// reads, and dropping it is safer than inventing a JSON spelling for it.
pub fn yaml_to_json_for_request(value: &Mapping) -> serde_json::Value {
    fn one(value: &Yaml) -> serde_json::Value {
        match value {
            Yaml::Null => serde_json::Value::Null,
            Yaml::Bool(b) => serde_json::Value::Bool(*b),
            Yaml::Number(n) => n
                .as_i64().map(serde_json::Value::from)
                .or_else(|| n.as_f64().and_then(serde_json::Number::from_f64).map(serde_json::Value::Number))
                .unwrap_or(serde_json::Value::Null),
            Yaml::String(s) => serde_json::Value::String(s.clone()),
            Yaml::Sequence(items) => serde_json::Value::Array(items.iter().map(one).collect()),
            Yaml::Mapping(map) => serde_json::Value::Object(
                map.iter()
                    .filter_map(|(k, v)| Some((crate::pystr::yaml_str(k), one(v))))
                    .collect(),
            ),
            _ => serde_json::Value::Null,
        }
    }
    one(&Yaml::Mapping(value.clone()))
}

/// One reply row back into an [`Assignment`]. `None` for anything the engine cannot read — a note
/// written from a half-formed row would rank as though it had no deadline.
pub fn assignment_from_row(row: &serde_json::Value) -> Option<Assignment> {
    let text = |key: &str| row.get(key).and_then(serde_json::Value::as_str).map(str::to_string);
    let due = DateTime::strptime("%Y-%m-%dT%H:%M", text("due")?.as_str()).ok()?;
    Some(Assignment {
        uid: text("uid")?,
        slug: text("slug")?,
        title: text("title")?,
        due,
        course: row.get("course").and_then(serde_json::Value::as_str).map(str::to_string),
        effort_hours: row.get("effort_hours").and_then(serde_json::Value::as_f64)?,
        effort_confidence: text("effort_confidence").unwrap_or_default(),
        effort_source: text("effort_source").unwrap_or_default(),
        importance: row.get("importance").and_then(serde_json::Value::as_i64)?,
        importance_reason: text("importance_reason").unwrap_or_default(),
        progress: row.get("progress").and_then(serde_json::Value::as_i64).unwrap_or(0),
        created_by: text("created_by").unwrap_or_default(),
        body: text("body").unwrap_or_default(),
    })
}

/// One source's fetch result, already redacted and JSON-shaped — everything
/// [`coursework_request`] needs and nothing [`collect_cloud`]'s credentialed step still holds.
/// `pub(crate)` rather than private: it is the seam a loopback test builds directly, with no
/// `wincred` or network in reach (R-C2-E17 / I1).
pub(crate) enum FetchedSource {
    ZyBooks { config: serde_json::Value, books: Vec<(String, serde_json::Value)> },
    Vhl { config: serde_json::Value, html: String },
}

/// Where a run's fetchers report the vendors' completion figures (stream J T8). A `RefCell`
/// because a [`Fetcher`] is a shared `Fn` and cannot take a `&mut` of its own; one sink per run.
pub type CompletionSink = std::cell::RefCell<Vec<crate::completion::VendorCompletion>>;

/// The completion figures in the raw vendor bytes a cloud vault posts (stream J T8). **Pure** and
/// device-side: the server parses these same bytes into assignments, and the uid scheme both
/// parsers share (`zybooks:<assignment_id>`, `vhl:<section>:<date>`) is what joins a figure to the
/// note the reply creates. No cloud change is needed for it.
pub(crate) fn completions_from_sources(
    sources: &[FetchedSource],
) -> Vec<crate::completion::VendorCompletion> {
    let mut out = Vec::new();
    for source in sources {
        match source {
            FetchedSource::ZyBooks { books, .. } => {
                for (_, payload) in books {
                    out.extend(crate::zybooks::completions(payload));
                }
            }
            FetchedSource::Vhl { html, .. } => out.extend(crate::vhl::completions(html)),
        }
    }
    out
}

/// The exact JSON body `/ingest-coursework` receives. **Pure** — no credential, no fetch, no
/// clock: everything here is either already-redacted config or the vendor's own raw bytes. This is
/// the function a test can assert the wire-format of without ever touching Credential Manager.
pub(crate) fn coursework_request(tz_name: &str, sources: &[FetchedSource]) -> serde_json::Value {
    let sources: Vec<serde_json::Value> = sources
        .iter()
        .map(|source| match source {
            FetchedSource::ZyBooks { config, books } => serde_json::json!({
                "name": "zybooks", "config": config,
                "books": books
                    .iter()
                    .map(|(code, payload)| serde_json::json!({"code": code, "payload": payload}))
                    .collect::<Vec<_>>(),
            }),
            FetchedSource::Vhl { config, html } => {
                serde_json::json!({ "name": "vhl", "config": config, "html": html })
            }
        })
        .collect();
    serde_json::json!({ "timezone": tz_name, "sources": sources })
}

/// `post_coursework`'s full decode of an `/ingest-coursework` reply: the assignments it carries
/// and the proposals beside them (R-OB-1) — one per source key the account's mapping does not
/// recognise yet. A struct rather than a bare tuple so `result.proposals` reads at the call site.
#[derive(Debug, Default)]
pub(crate) struct PostCourseworkResult {
    pub assignments: Vec<Assignment>,
    pub proposals: Vec<MapProposal>,
}

/// Posts an already-built request and turns the reply into assignments, proposals and warnings.
/// The service-unavailable and malformed-row paths both live here, where a loopback test can reach
/// them with no credential and no live vendor (R-C2-E17 / I1).
pub(crate) fn post_coursework(
    client: &crate::cloudmodel::CloudClient,
    request: &serde_json::Value,
    warnings: &mut Vec<String>,
) -> PostCourseworkResult {
    let reply = match client.post("/ingest-coursework", request) {
        Ok(reply) => reply,
        Err(err) => {
            warnings.push(format!("coursework: the service is unavailable ({err}); nothing changed"));
            return PostCourseworkResult::default();
        }
    };
    for warning in reply.get("warnings").and_then(serde_json::Value::as_array).into_iter().flatten() {
        if let Some(text) = warning.as_str() {
            // C2 final review E-4: a server-supplied warning lands in the run record, which the
            // Runs view renders and `runs` prints — so it goes through the same clip-and-scrub
            // every other borrowed string does. `one_line` collapses newlines (a multi-line
            // warning would otherwise forge run-record lines) and bounds the length.
            warnings.push(crate::judge::one_line(text, 200));
        }
    }
    let mut assignments = Vec::new();
    for row in reply.get("assignments").and_then(serde_json::Value::as_array).into_iter().flatten() {
        match assignment_from_row(row) {
            Some(item) => assignments.push(item),
            None => warnings.push("coursework: a reply row could not be read; skipped".to_string()),
        }
    }
    let mut proposals = Vec::new();
    for row in reply.get("proposals").and_then(serde_json::Value::as_array).into_iter().flatten() {
        let text = |k: &str| row.get(k).and_then(serde_json::Value::as_str).map(str::to_string);
        // A proposal with no key names nothing `write_map_card` could file a card under; skipped
        // rather than warned, the same way a malformed assignment row is a warning but a wholly
        // absent field here is not — there is nothing here to say was even attempted.
        let key = match text("key") {
            Some(k) if !k.is_empty() => k,
            _ => continue,
        };
        proposals.push(MapProposal {
            source: text("source").unwrap_or_default(),
            key,
            label: text("label").unwrap_or_default(),
            suggested_course: text("suggested_course").filter(|c| !c.is_empty()),
        });
    }
    PostCourseworkResult { assignments, proposals }
}

/// The agent actor for coursework-mapping cards. `agent:` prefix, so `provenance::is_agent` holds.
pub const MAP_ACTOR: &str = "agent:knowlu.coursework";

/// One thing the account's mapping does not know about (R-OB-1), as the service reported it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MapProposal {
    pub source: String,
    pub key: String,
    pub label: String,
    pub suggested_course: Option<String>,
}

/// *Fetch on device, think in the cloud* (D11, §4.3). Every credentialed request is made here with
/// the student's own password out of Credential Manager; the raw payload then goes to
/// `/ingest-coursework` and the reconciled list comes back.
///
/// One source's failure never stops another, and the server applies the same
/// *an-empty-parse-is-a-failure* rule `collect` applies locally. The timezone is resolved through
/// [`resolve_timezone`] — the same validation and single fallback warning `collect` gives the
/// local path (I3 / R-C2-E17): an unknown IANA name must not silently drop the whole source.
///
/// R-OB-1 / R-C2-E17 correction F5: widened over Task 7's shape to carry what the card-writing
/// half needs — `vault`, `ctx` and `today` — so an unmapped book or section becomes a card in
/// `approvals/` rather than only a line in `warnings`. R-C2-E18 fix 1 widens it once more with
/// `dry_run`: the fetch and the reconciliation still run (dry run has never skipped the network
/// half — `collect`'s local path doesn't either), but the card-minting step below does not.
/// Stream J T8 widens it with `completions`: the vendors' completion figures, read from the raw
/// payloads before they are posted (see [`completions_from_sources`]).
#[cfg(windows)]
pub fn collect_cloud(
    vault: &Path,
    config: &Mapping,
    warnings: &mut Vec<String>,
    ctx: &crate::write::WriteContext,
    today: Date,
    dry_run: bool,
    client: &crate::cloudmodel::CloudClient,
    completions: &mut Vec<crate::completion::VendorCompletion>,
) -> Vec<Assignment> {
    let block = match crate::yaml::get(config, "coursework") {
        Some(Yaml::Mapping(map)) => map.clone(),
        _ => Mapping::new(),
    };
    let tz_name = resolve_timezone(config, warnings)
        .iana_name()
        .unwrap_or(DEFAULT_TZ)
        .to_string();
    let mut sources: Vec<FetchedSource> = Vec::new();

    for name in ["zybooks", "vhl"] {
        let cfg = match crate::yaml::get(&block, name) {
            Some(Yaml::Mapping(map)) => map.clone(),
            _ => continue,
        };
        if !crate::yaml::get(&cfg, "enabled").map(crate::pystr::yaml_truthy).unwrap_or(false) {
            continue;
        }
        let credential = match crate::wincred::read_credential(&cfg_str(&cfg, "credential_target", "")) {
            Ok(c) => c,
            Err(err) => {
                warnings.push(format!("{name}: fetch failed ({err}); nothing changed"));
                continue;
            }
        };
        let redacted = yaml_to_json_for_request(&redact(&cfg, name));
        match name {
            "zybooks" => match crate::zybooks::fetch_payloads(&credential.username, credential.password.expose()) {
                Ok(books) => sources.push(FetchedSource::ZyBooks { config: redacted, books }),
                Err(SourceError::NotLoggedIn(m)) => warnings.push(format!("zybooks: session invalid ({m}); nothing changed")),
                Err(SourceError::Failed(m)) => warnings.push(format!("zybooks: fetch failed ({m}); nothing changed")),
            },
            _ => {
                let base = cfg_str(&cfg, "base_url", "https://www.vhlcentral.com");
                match crate::vhl::login_and_fetch_dashboard(&credential.username, credential.password.expose(), &base, None) {
                    Ok(html) => sources.push(FetchedSource::Vhl { config: redacted, html }),
                    Err(SourceError::NotLoggedIn(m)) => warnings.push(format!("vhl: session invalid ({m}); nothing changed")),
                    Err(SourceError::Failed(m)) => warnings.push(format!("vhl: fetch failed ({m}); nothing changed")),
                }
            }
        }
    }

    if sources.is_empty() {
        return Vec::new();
    }
    // Stream J T8: the vendors' own completion figures, read on the device from the same bytes
    // the server is about to parse.
    completions.extend(completions_from_sources(&sources));
    let request = coursework_request(&tz_name, &sources);
    let result = post_coursework(client, &request, warnings);

    // R-OB-1: one card per unmapped book or section, before the items are written, so a student
    // whose only zyBook is unmapped opens the deck to a question rather than to an empty list.
    propose_map_cards(vault, &result.proposals, today, ctx, dry_run, warnings);
    result.assignments
}

/// One card per still-unasked proposal — or, in a dry run, one line saying it would have filed
/// one. Pulled out of `collect_cloud` (which needs a real Credential Manager entry before it ever
/// gets here, and so is not exercised directly by this module's own tests — see the comment above
/// `the_coursework_payload_carries_no_credential`) so this half, which touches neither network nor
/// credential, is unit-testable on its own: `a_dry_run_leaves_config_approvals_archive_and_the_journal_byte_identical`
/// below calls it directly.
fn propose_map_cards(
    vault: &Path,
    proposals: &[MapProposal],
    today: Date,
    ctx: &crate::write::WriteContext,
    dry_run: bool,
    warnings: &mut Vec<String>,
) {
    let card_ctx = ctx.with_actor(MAP_ACTOR);
    // Every key already asked about, live or decided — computed ONCE before the loop, exactly as
    // Task 12's `existing_rule_ids` is. `write::create`'s `Exists` guard only sees `approvals/`, and
    // a rejected card is in `archive/`; without this a book the student said no to is re-proposed
    // every slot until the end of time.
    let asked = asked_map_keys(vault, today);
    if dry_run {
        // R-C2-E18 fix 1: no `Journal`, no `write_map_card` call — a dry run must not create a
        // note or a journal record, only say what it would have proposed.
        for proposal in proposals {
            if asked.contains(&(proposal.source.clone(), proposal.key.clone())) {
                continue;
            }
            warnings.push(format!(
                "{}: dry run — would file a card for {} ({})",
                proposal.source, proposal.label, proposal.key
            ));
        }
        return;
    }
    let mut journal = Journal::new(vault);
    for proposal in proposals {
        if asked.contains(&(proposal.source.clone(), proposal.key.clone())) {
            continue;
        }
        match write_map_card(vault, proposal, today, &card_ctx, &mut journal) {
            Ok(path) => {
                let stem = path.file_stem().map(|s| s.to_string_lossy().to_string()).unwrap_or_default();
                warnings.push(format!("{}: not mapped; proposed ({stem})", proposal.source));
            }
            // Reached only for a key `asked` did NOT already know about (the check above already
            // filtered those out) — so this is not the same case, and is not unreachable. The
            // stem is `map-<source>-<slugify(key)>` and `slugify` lowercases and truncates at 60
            // chars: two different vendor codes can collide on one filename, and a silent skip here
            // would make the second book's card vanish from the deck with no explanation at all.
            Err(crate::write::WriteError::Exists(_)) => warnings.push(format!(
                "{}: proposal not written (a card already exists at this key's filename; {} may \
                 collide with another key after slugifying)",
                proposal.source, proposal.key
            )),
            Err(e) => warnings.push(format!("{}: proposal not written ({e})", proposal.source)),
        }
    }
}

/// The credential store is Windows-only (spec §6.5), so the credentialed fetch is too. A cloud
/// build still compiles; it simply has no way to authenticate, and says so rather than pretending
/// the semester is empty — the same shape `fetch_zybooks` and `fetch_vhl` already have.
#[cfg(not(windows))]
pub fn collect_cloud(
    _vault: &Path,
    _config: &Mapping,
    warnings: &mut Vec<String>,
    _ctx: &crate::write::WriteContext,
    _today: Date,
    _dry_run: bool,
    _client: &crate::cloudmodel::CloudClient,
    _completions: &mut Vec<crate::completion::VendorCompletion>,
) -> Vec<Assignment> {
    warnings.push("coursework: credential store unavailable on this platform".to_string());
    Vec::new()
}

/// The card the deck renders. `kind: coursework-map` is a kind `approvals.rs` leaves alone
/// (hand-off H12): the decision is *applied* by the next `coursework` step, which then archives it.
///
/// `Err` when a card for this key already exists — the next slot fetches the same unmapped book and
/// must not mint a second card for it. The stem carries the key, so the guard is the filename.
/// That guard sees only `approvals/`, which is why `asked_map_keys` (below) is the real one and
/// this is defence in depth: a **rejected** card is in `archive/` and this check cannot see it.
pub fn write_map_card(
    vault: &Path,
    proposal: &MapProposal,
    today: Date,
    ctx: &WriteContext,
    journal: &mut Journal,
) -> Result<PathBuf, crate::write::WriteError> {
    let lit = |s: &str| crate::write::to_literal(&Yaml::String(s.to_string()));
    let stamp = today.strftime("%Y-%m-%d").to_string();
    let expires = today
        .checked_add(jiff::Span::new().days(30))
        .unwrap_or(today)
        .strftime("%Y-%m-%d")
        .to_string();
    let course = proposal.suggested_course.clone().unwrap_or_default();
    let text = format!(
        "---\ntype: approval\nkind: coursework-map\ntitle: {}\nstatus: pending\n\
         proposed_at: {stamp}\nfirst_proposed_at: {stamp}\nexpires: {expires}\nsnooze_until: null\n\
         created_by: coursework\nsource: {}\nmap_key: {}\ncourse: {}\n---\n\n\
         {} returned work, and nothing in your setup says which course it belongs to — so none of \
         it reached your list.\n\n\
         Approve to map it. **Edit `course:` above first if the guess is wrong**, or if it is \
         blank: it is the course slug the items will be filed under, the same one your course note \
         uses.\n",
        lit(&format!("Map {} to a course", proposal.label)),
        // R-C2-E18: all three go through `lit()`, the crate's one escaping barrier — a key arriving
        // verbatim off the HTTP reply could carry `: `, a leading `*`/`&`/`!`, `---`, or a newline,
        // and unquoted would either fail `create`'s `split_frontmatter` check (no card at all, the
        // silent skip R-OB-1 exists to abolish) or inject frontmatter lines. Nothing downstream
        // cares that these are quoted: `apply_map_cards` and `asked_map_keys` read them back
        // through `yaml::opt_text`.
        lit(&proposal.source),
        lit(&proposal.key),
        if course.is_empty() { "\"\"".to_string() } else { lit(&course) },
        proposal.label,
    );
    let approvals = vault.join("approvals");
    std::fs::create_dir_all(&approvals).map_err(|e| crate::write::WriteError::Io(e.to_string()))?;
    // The key is in the stem, so a second card for the same book is `WriteError::Exists` and not a
    // duplicate — the same guard `find_pending_amendment` gives an amend card.
    let stem = format!("map-{}-{}", proposal.source, crate::ingest::slugify(&proposal.key));
    let rel = crate::ids::rel(vault, &approvals.join(format!("{stem}.md")));
    crate::write::create(vault, &rel, &text, ctx, journal, None)
}

/// Every `(source, map_key)` a card has already asked about and whose card has not yet expired —
/// `approvals/` for the live ones and **`archive/` for the decided ones**, which is the whole point.
///
/// The same mechanism as Task 12's `existing_rule_ids`, and for the same reason. `write_map_card`'s
/// only other guard is `write::create`'s `Exists` on `approvals/map-<source>-<slug>.md`, and a
/// **rejected** card is not there — `process_approvals` settles it into `archive/`. Without this
/// the next `coursework` run finds the book still unmapped, gets the same proposal and mints a
/// fresh card, twice a day, forever, each one charging the 15-a-day proposal budget.
///
/// **Where the memory lives:** in the card itself, at `archive/map-<source>-<slug>.md`. The vault
/// keeps `archive/` forever and the journal records the move, so nothing new has to be remembered
/// anywhere else. **For how long:** until the `expires:` that card was minted with —
/// `write_map_card` sets it to 30 days out — after which a still-unmapped book is proposed
/// again. That is the loop H12 describes, and this is what makes it true: a student who does not
/// recognise a book is asked again next month, not twice a day forever.
fn asked_map_keys(vault: &Path, today: Date) -> std::collections::BTreeSet<(String, String)> {
    let stamp = today.strftime("%Y-%m-%d").to_string();
    let mut out = std::collections::BTreeSet::new();
    for folder in ["approvals", "archive"] {
        for path in crate::approvals::sorted_md(&vault.join(folder)) {
            let Ok(text) = pystr::read_text(&path) else { continue };
            let Ok((meta, _)) = crate::models::split_frontmatter(&text) else { continue };
            if crate::yaml::opt_text(crate::yaml::get(&meta, "kind")).as_deref() != Some("coursework-map") {
                continue;
            }
            // ISO dates compare correctly as text, which is why the vault writes them that way.
            let expires = crate::yaml::opt_text(crate::yaml::get(&meta, "expires")).unwrap_or_default();
            if expires < stamp {
                continue;
            }
            let source = crate::yaml::opt_text(crate::yaml::get(&meta, "source")).unwrap_or_default();
            let key = crate::yaml::opt_text(crate::yaml::get(&meta, "map_key")).unwrap_or_default();
            if !key.is_empty() {
                out.insert((source, key));
            }
        }
    }
    out
}

/// Apply every approved `kind: coursework-map` card, then archive it.
///
/// **The write into `config/ingest.yaml` is a text-level insertion, never a rewrite** (ruling
/// R-C2-8). CLAUDE.md's invariant is absolute and has no config exemption — *no vault file is
/// parsed and re-dumped*, and `src/yamlemit.rs` is the crate's one YAML emitter. (`scaffold::
/// ingest_yaml` writing this file at birth is **creation**, which is a different act from
/// rewriting one that already exists.) So `write_mapping` below finds the `courses:` /
/// `sections:` line and inserts three lines beneath it, and the *card* it came from is journalled
/// through `write` like every other note, which is where the audit trail lives.
///
/// R-C2-E18 fix 1: `dry_run` guards every write in this function. A dry run reads and reports —
/// neither branch below calls `write_mapping`, `write_literals` or `delete` when it is set, so
/// `config/ingest.yaml`, `approvals/`, `archive/` and `state/journal/` are untouched, matching the
/// promise `sync_coursework`'s own dry run already makes.
pub fn apply_map_cards(vault: &Path, ctx: &WriteContext, journal: &mut Journal, dry_run: bool) -> Vec<String> {
    let mut lines: Vec<String> = Vec::new();
    for path in crate::approvals::sorted_md(&vault.join("approvals")) {
        let Ok(text) = pystr::read_text(&path) else { continue };
        let Ok((meta, _)) = crate::models::split_frontmatter(&text) else { continue };
        if crate::yaml::opt_text(crate::yaml::get(&meta, "kind")).as_deref() != Some("coursework-map") {
            continue;
        }
        if crate::yaml::opt_text(crate::yaml::get(&meta, "status")).as_deref() != Some("approved") {
            continue;
        }
        let stem = path.file_stem().map(|s| s.to_string_lossy().to_string()).unwrap_or_default();
        let source = crate::yaml::opt_text(crate::yaml::get(&meta, "source")).unwrap_or_default();
        let key = crate::yaml::opt_text(crate::yaml::get(&meta, "map_key")).unwrap_or_default();
        let course = crate::yaml::opt_text(crate::yaml::get(&meta, "course")).unwrap_or_default();
        if course.trim().is_empty() {
            if dry_run {
                lines.push(format!("coursework: dry run — {stem} would not be applied (no course on the card)"));
                continue;
            }
            // Kept, not archived: an empty `course` would file every item from that book under
            // `course: ""`, which reads as attributed-to-nothing rather than unattributed. The
            // student edits the card and approves it again.
            //
            // Written back to **pending**, which is what `approvals.rs` does with a recoverable
            // refusal (`approvals.rs:1360-1368`) and for the same reason: a card left `approved` on
            // disk is never counted, never escalated into Must do and never expires, so it re-logs
            // this line every run forever. Pending, it ages.
            let rel = crate::ids::rel(vault, &path);
            let pending = vec![("status".to_string(), "pending".to_string())];
            let _ = crate::write::write_literals(vault, &rel, &pending, ctx, journal, &WriteOpts::default());
            lines.push(format!("coursework: {stem} not applied (no course on the card)"));
            continue;
        }
        if dry_run {
            // Neither `write_mapping` (which writes the moment it would insert) nor the
            // status/archive writes below run here — a dry run does not even learn whether the key
            // is already mapped, because finding out would mean touching nothing on disk that a
            // real run wouldn't, and `write_mapping` has no such read-only mode.
            lines.push(format!("coursework: dry run — would apply mapping {key} to {course}"));
            continue;
        }
        match write_mapping(vault, &source, &key, &course) {
            Ok(inserted) => {
                let rel = crate::ids::rel(vault, &path);
                let stamped = format!("\"{}\"", jiff::Zoned::now().strftime("%Y-%m-%d %H:%M"));
                let literals = vec![
                    ("status".to_string(), "executed".to_string()),
                    ("executed_at".to_string(), stamped),
                ];
                if crate::write::write_literals(vault, &rel, &literals, ctx, journal, &WriteOpts::default()).is_ok() {
                    let _ = crate::write::delete(vault, &rel, ctx, journal);
                }
                // R-C2-8's idempotence. `Ok(false)` is "the key was already there" — a re-approved
                // card, or a hand edit that got there first — and it is a line, not an error: the
                // file already says what the card was asking for, so the card is still archived.
                lines.push(if inserted {
                    format!("coursework: mapped {key} to {course}")
                } else {
                    format!("coursework: {key} was already mapped; card archived, file untouched")
                });
            }
            // Unwritable — there is no `coursework.<source>:` block to insert under, so the
            // mapping would be dead config. Same treatment as a blank course, and the same reason.
            Err(e) => {
                let rel = crate::ids::rel(vault, &path);
                let pending = vec![("status".to_string(), "pending".to_string())];
                let _ = crate::write::write_literals(vault, &rel, &pending, ctx, journal, &WriteOpts::default());
                lines.push(format!("coursework: {stem} not applied ({e})"));
            }
        }
    }
    lines
}

/// The indentation of a line, in spaces. Vault YAML is space-indented — a tab is invalid YAML
/// there and every reader in the crate would already have refused the file.
fn indent_of(line: &str) -> usize {
    line.len() - line.trim_start().len()
}

/// A line that carries no structure: blank, or a comment. Its column means nothing, so neither
/// helper below may let one end a block or set an indent.
fn is_filler(line: &str) -> bool {
    let t = line.trim_start();
    t.is_empty() || t.starts_with('#')
}

/// The first line in `[from, end)` at exactly `depth` whose key is `name`, or `None` if the block
/// ends first. A block ends at the first structural line indented *less* than `depth`.
fn find_key(lines: &[String], from: usize, end: usize, name: &str, depth: usize) -> Option<usize> {
    for i in from..end.min(lines.len()) {
        if is_filler(&lines[i]) {
            continue;
        }
        let ind = indent_of(&lines[i]);
        if ind < depth {
            return None;
        }
        if ind == depth {
            let rest = lines[i].trim_start();
            if rest == format!("{name}:") || rest.starts_with(&format!("{name}: ")) {
                return Some(i);
            }
        }
    }
    None
}

/// Where the block headed by `header` ends: the first structural line indented no deeper than the
/// header itself, or the end of the file.
fn block_end(lines: &[String], header: usize) -> usize {
    let depth = indent_of(&lines[header]);
    (header + 1..lines.len())
        .find(|&i| !is_filler(&lines[i]) && indent_of(&lines[i]) <= depth)
        .unwrap_or(lines.len())
}

/// **The file's own indentation**: the column of the first structural child of `header`, or the
/// header's own plus 2 when it has none yet. Never a constant — a student who indents by four
/// keeps indenting by four.
fn child_indent(lines: &[String], header: usize, end: usize) -> usize {
    (header + 1..end)
        .find(|&i| !is_filler(&lines[i]))
        .map(|i| indent_of(&lines[i]))
        .unwrap_or_else(|| indent_of(&lines[header]) + 2)
}

/// Insert one mapping into `config/ingest.yaml` — `coursework.zybooks.courses.<code>` or
/// `coursework.vhl.sections.<id>`, each `{course, label}`.
///
/// **A text-level insertion, never a parse-and-re-dump** (ruling R-C2-8). CLAUDE.md's engine
/// invariant is absolute and has no config exemption: *no vault file is parsed and re-dumped*,
/// and `src/yamlemit.rs` is the crate's one YAML emitter. A `serde_yaml_ng::to_string` round trip
/// would re-render every line of a file whose whole point is that it is editable text — comments
/// gone, quoting style changed, key order at the serialiser's mercy. (`scaffold::ingest_yaml`
/// writing this file at birth is **creation**, a different act from rewriting one that exists.)
///
/// So this finds the `courses:` / `sections:` line and inserts three lines directly beneath it at
/// the file's own indentation, and **touches no other byte**. The one shape it rewrites is a
/// `courses: {}` scalar, which becomes the block form the insertion needs — one line replaced,
/// in place, with the same leading spaces it already had.
///
/// **It can only ADD a key, and that costs nothing.** A book or section that is already mapped
/// never produces a card in the first place: `route_zybook` answers `BookRouting::Mapped` and
/// `parse_vhl` finds its section, so `/ingest-coursework` reports no proposal and there is no path
/// by which an existing key needs updating. A student who wants to change one edits the file,
/// which is the point of it being text.
///
/// `Ok(true)` inserted, `Ok(false)` the key was already there — a no-op, and the file is not
/// opened for writing at all. `Err` when the source's block is missing, because a mapping under a
/// `coursework.vhl:` nobody configured is dead config; the card is kept so the student sees why.
///
/// Line endings are the crate's usual discipline: `pystr::read_text` normalises to `\n` and
/// `pystr::write_text` writes `os.linesep` back, so a CRLF vault file stays CRLF exactly as it
/// does for every note this engine writes.
fn write_mapping(vault: &Path, source: &str, key: &str, course: &str) -> Result<bool, String> {
    let (block, field) = match source {
        "zybooks" => ("zybooks", "courses"),
        "vhl" => ("vhl", "sections"),
        other => return Err(format!("unknown coursework source {other:?}")),
    };
    let path = vault.join("config").join("ingest.yaml");
    let text = pystr::read_text(&path).map_err(|e| e.to_string())?;
    let mut lines: Vec<String> = text.split('\n').map(str::to_string).collect();

    let coursework = find_key(&lines, 0, lines.len(), "coursework", 0)
        .ok_or_else(|| "config/ingest.yaml has no coursework: block".to_string())?;
    let coursework_end = block_end(&lines, coursework);
    let source_depth = child_indent(&lines, coursework, coursework_end);
    let source_line = find_key(&lines, coursework + 1, coursework_end, block, source_depth)
        .ok_or_else(|| format!("config/ingest.yaml has no coursework.{block}: block"))?;
    let source_end = block_end(&lines, source_line);
    let field_depth = child_indent(&lines, source_line, source_end);

    // Both scalars go through the crate's ONE literal writer — `write::to_literal`, the same
    // function every frontmatter value goes through, which routes anything non-scalar to
    // `yamlemit`. No second emitter is introduced. Quoting the key is what makes it work: an
    // unquoted `2102121:` is an integer key and `yaml::get(sections, "2102121")` then misses.
    let lit = |s: &str| crate::write::to_literal(&Yaml::String(s.to_string()));
    // The label prefixes every title the parser produces (`CS 100 HW 01`). Derived from the slug
    // rather than asked for: one field on the card is one decision, and this one is mechanical.
    let label = course.trim().to_uppercase().replace('-', " ");

    let (insert_at, entry_indent) = match find_key(&lines, source_line + 1, source_end, field, field_depth) {
        Some(at) => {
            let value = lines[at].trim_start()[field.len() + 1..].trim().to_string();
            if value == "{}" || value == "{ }" {
                // The birth shape `courses: {}` (C1's `scaffold::ingest_yaml`). One line replaced,
                // in place, by the block header the insertion needs — same indentation, and it is
                // the only line in the file this function ever rewrites.
                lines[at] = format!("{}{field}:", " ".repeat(field_depth));
            } else if !value.is_empty() {
                return Err(format!("coursework.{block}.{field} is not a block mapping ({value})"));
            }
            let end = block_end(&lines, at);
            let deeper = child_indent(&lines, at, end);
            // Idempotence: a key already under this field is a no-op, never a second entry — two
            // `UACS100Fall2026:` keys is a duplicate-key YAML error and the file stops loading.
            for i in (at + 1)..end {
                if is_filler(&lines[i]) || indent_of(&lines[i]) != deeper {
                    continue;
                }
                let name = lines[i].trim_start();
                let name = name.split(':').next().unwrap_or("").trim_matches(['\'', '"']);
                if name == key {
                    return Ok(false);
                }
            }
            (at + 1, deeper)
        }
        // No `courses:` line under a `zybooks:` that exists: insert the field header too. Still a
        // pure insertion — two lines added, none changed.
        None => {
            lines.insert(source_line + 1, format!("{}{field}:", " ".repeat(field_depth)));
            (source_line + 2, field_depth + 2)
        }
    };

    let pad = " ".repeat(entry_indent);
    lines.splice(
        insert_at..insert_at,
        [
            format!("{pad}{}:", lit(key)),
            format!("{pad}  course: {}", lit(course.trim())),
            format!("{pad}  label: {}", lit(&label)),
        ],
    );
    pystr::write_text(&path, &lines.join("\n")).map_err(|e| e.to_string())?;
    Ok(true)
}

// ---------------------------------------------------------------------------------------------
// discovery (C1 H10) — read-only, writes nothing
// ---------------------------------------------------------------------------------------------

/// The zyBooks half of `discover_json`'s shaping, pulled out pure: one row per fetched code,
/// routed exactly the way [`fetch_zybooks`] routes — same [`route_zybook`], not a second
/// interpretation. Takes no network and no credential, so it is directly testable against a
/// `courses`/`ignore` config loaded from a real vault.
fn zybooks_rows(codes: &[String], zycfg: &Mapping) -> Vec<serde_json::Value> {
    use serde_json::json;
    let courses = match crate::yaml::get(zycfg, "courses") {
        Some(Yaml::Mapping(map)) => map.clone(),
        _ => Mapping::new(),
    };
    let ignore: Vec<String> = match crate::yaml::get(zycfg, "ignore") {
        Some(Yaml::Sequence(items)) => items.iter().map(yaml_str).collect(),
        _ => Vec::new(),
    };
    codes
        .iter()
        .map(|code| {
            let routing = route_zybook(code, &courses, &ignore);
            json!({
                "code": code,
                "mapped": matches!(routing, BookRouting::Mapped(_)),
                "ignored": matches!(routing, BookRouting::Ignored),
            })
        })
        .collect()
}

/// The VHL half of `discover_json`'s shaping, pulled out pure: one row per `(course_id,
/// section_id)` pair [`crate::vhl::discover_sections`] found, `mapped` computed the way
/// [`crate::vhl::parse_dashboard`] routes a section — [`crate::vhl::section_mapping`], the one
/// predicate both callers share, not a second interpretation. Takes no network and no
/// credential, so it is directly testable against a `sections` config loaded from a real vault.
fn vhl_rows(pairs: &[(String, String)], vhlcfg: &Mapping) -> Vec<serde_json::Value> {
    use serde_json::json;
    let sections = match crate::yaml::get(vhlcfg, "sections") {
        Some(Yaml::Mapping(map)) => map.clone(),
        _ => Mapping::new(),
    };
    pairs
        .iter()
        .map(|(course_id, section_id)| {
            let mapped = crate::vhl::section_mapping(&sections, section_id).is_some();
            json!({ "course_id": course_id, "section": section_id, "mapped": mapped })
        })
        .collect()
}

/// Everything onboarding needs to build a mapping, and nothing else. **Never writes**, never
/// starts a run record, and always returns a JSON object — a source that could not be reached is
/// an `errors` entry, not an exit code, because the wizard's answer to "we could not reach
/// zyBooks" is to let the student type the mapping, not to stop.
///
/// With `vault`, the credential targets and `enabled`-independent `courses`/`sections` mappings
/// come from that vault's own `config/ingest.yaml` and `mapped` is computed from it. Without a
/// vault (the wizard, whose vault does not exist yet) the targets come from the two flags alone
/// and `mapped` is always `false`. A missing target — no flag and, with a vault, no
/// `credential_target` in its config — means that source is simply skipped, not an error.
///
/// **A config that cannot be read is an `errors` entry, never a silent empty config.** A
/// malformed `config/ingest.yaml` — bad YAML or non-UTF-8 bytes — reports `config: <reason>` and
/// falls back to an empty config *for shaping only*; it does not stop a source a target flag
/// still names from being queried, because losing the codes is worse than losing the mapping.
#[cfg(windows)]
pub fn discover_json(
    vault: Option<&Path>,
    zybooks_target: Option<&str>,
    vhl_target: Option<&str>,
) -> String {
    use serde_json::json;
    let mut errors: Vec<String> = Vec::new();
    let config: Mapping = match vault.map(load_coursework_config) {
        Some(Ok((cfg, warnings))) => {
            errors.extend(warnings.into_iter().map(|w| format!("config: {w}")));
            cfg
        }
        Some(Err(err)) => {
            errors.push(format!("config: {err}"));
            Mapping::new()
        }
        None => Mapping::new(),
    };
    let block = match crate::yaml::get(&config, "coursework") {
        Some(Yaml::Mapping(map)) => map.clone(),
        _ => Mapping::new(),
    };
    let source = |name: &str| -> Mapping {
        match crate::yaml::get(&block, name) {
            Some(Yaml::Mapping(map)) => map.clone(),
            _ => Mapping::new(),
        }
    };

    // ---- zyBooks
    let zycfg = source("zybooks");
    let zytarget = zybooks_target
        .map(str::to_string)
        .unwrap_or_else(|| cfg_str(&zycfg, "credential_target", ""));
    let mut zybooks: Vec<serde_json::Value> = Vec::new();
    if !zytarget.is_empty() {
        match (|| -> Result<Vec<String>, SourceError> {
            let cred = crate::wincred::read_credential(&zytarget)
                .map_err(|err| SourceError::Failed(format!("{err}")))?;
            let (token, user_id) =
                crate::zybooks::signin(&cred.username, cred.password.expose(), None)?;
            crate::zybooks::fetch_zybook_codes(&token, user_id, None)
        })() {
            Ok(codes) => zybooks = zybooks_rows(&codes, &zycfg),
            Err(err) => errors.push(format!("zybooks: {err}")),
        }
    }

    // ---- VHL
    let vhlcfg = source("vhl");
    let vhltarget = vhl_target
        .map(str::to_string)
        .unwrap_or_else(|| cfg_str(&vhlcfg, "credential_target", ""));
    let mut vhl: Vec<serde_json::Value> = Vec::new();
    if !vhltarget.is_empty() {
        match (|| -> Result<String, SourceError> {
            let cred = crate::wincred::read_credential(&vhltarget)
                .map_err(|err| SourceError::Failed(format!("{err}")))?;
            crate::vhl::login_and_fetch_dashboard(
                &cred.username,
                cred.password.expose(),
                &cfg_str(&vhlcfg, "base_url", "https://www.vhlcentral.com"),
                None,
            )
        })() {
            Ok(html) => vhl = vhl_rows(&crate::vhl::discover_sections(&html), &vhlcfg),
            Err(err) => errors.push(format!("vhl: {err}")),
        }
    }

    crate::ledger::dumps_value(&json!({ "zybooks": zybooks, "vhl": vhl, "errors": errors }))
}

/// The credential store is Windows-only, so discovery is too — and it says so rather than
/// reporting an empty semester, which is the same rule [`fetch_zybooks`] follows.
#[cfg(not(windows))]
pub fn discover_json(
    _vault: Option<&Path>,
    _zybooks_target: Option<&str>,
    _vhl_target: Option<&str>,
) -> String {
    use serde_json::json;
    crate::ledger::dumps_value(&json!({
        "zybooks": [], "vhl": [],
        "errors": ["credential store unavailable on this platform"],
    }))
}

/// Run every enabled source. **One source's failure never stops another.**
///
/// And the rule this function exists for: *an empty parse is failure, never an empty semester.*
/// A 200 response yielding nothing is far more likely to be a dead session than a term with no
/// homework in it, so zero items warns and changes nothing.
pub fn collect(
    _vault: &Path,
    config: &Mapping,
    warnings: &mut Vec<String>,
    fetchers: Option<&[(&str, Fetcher)]>,
) -> Vec<Assignment> {
    let defaults: [(&str, Fetcher); 2] = [
        ("zybooks", &fetch_zybooks),
        ("vhl", &fetch_vhl),
    ];
    let active: &[(&str, Fetcher)] = fetchers.unwrap_or(&defaults);
    let block = match crate::yaml::get(config, "coursework") {
        Some(Yaml::Mapping(map)) => map.clone(),
        _ => Mapping::new(),
    };
    let tz = resolve_timezone(config, warnings);
    let mut out: Vec<Assignment> = Vec::new();
    for (name, fetch) in active {
        let source_cfg = match crate::yaml::get(&block, name) {
            Some(Yaml::Mapping(map)) => map.clone(),
            _ => Mapping::new(),
        };
        let enabled = crate::yaml::get(&source_cfg, "enabled")
            .map(crate::pystr::yaml_truthy)
            .unwrap_or(false);
        if !enabled {
            continue;
        }
        let mut source_warnings: Vec<String> = Vec::new();
        let items = match fetch(&source_cfg, &tz, &mut source_warnings) {
            Ok(items) => items,
            Err(SourceError::NotLoggedIn(message)) => {
                warnings.push(format!("{name}: session invalid ({message}); nothing changed"));
                continue;
            }
            Err(SourceError::Failed(message)) => {
                warnings.push(format!("{name}: fetch failed ({message}); nothing changed"));
                continue;
            }
        };
        warnings.extend(source_warnings.into_iter().map(|w| format!("{name}: {w}")));
        if items.is_empty() {
            // Deliberately not "no assignments this semester". See coursework spec §9.
            warnings.push(format!("{name}: 0 assignments parsed; treating as failure"));
            continue;
        }
        out.extend(items);
    }
    out
}

/// Substrings that mark a warning as a source- or run-level failure rather than a note about one
/// item. `collect` and `load_coursework_config` own every phrase here, so this stays a closed set
/// rather than a guess about arbitrary text.
const FAILURE_MARKERS: [&str; 7] = [
    "session invalid",
    "fetch failed",
    "0 assignments parsed",
    "config unreadable",
    "coursework pass failed",
    // C2 Task 7 fix 1 (R-C2-E17 / I2): `collect_cloud` and the service's own handler can now
    // fail a whole source without ever saying "fetch failed" or "session invalid" — a dead
    // service and a server-side parse error are new failure shapes, and without their own
    // markers here they collapse into `(+N more)` behind a benign per-item note.
    "the service is unavailable",
    "parse failed",
];

/// Failures first, benign per-item notes last; **stable** within each group.
///
/// Only one warning reaches `state/runner-log.md` — the rest collapse into `(+N more)` — and under
/// Task Scheduler stdout is discarded, so that line is the only durable diagnostic. `collect`
/// iterates zybooks-then-vhl, so without this a harmless "uncategorised; using default importance"
/// wins the slot and hides a dead VHL session, which is the exact failure this design exists to
/// make visible.
pub fn rank_warnings(warnings: &[String]) -> Vec<String> {
    let mut ranked: Vec<String> = warnings.to_vec();
    // `sorted(..., key=...)` is stable, and so is `sort_by_key`.
    ranked.sort_by_key(|w| {
        if FAILURE_MARKERS.iter().any(|marker| w.contains(marker)) {
            0
        } else {
            1
        }
    });
    ranked
}

/// The `knowlu-engine coursework` entry point. Returns a process exit code, and it is always `0`.
///
/// **Coursework must never fail the run: the rank matters more than the fetch.** `collect` already
/// swallows per-source failures, so reaching the outer handler means either the config load hit
/// something it does not itself catch or a genuine bug elsewhere. Either way the run-log line is
/// still appended, because a *missing* line reads as "nothing to do" rather than "this failed".
///
/// Argument parsing and the `--via` choice list belong to the `clap` layer (plan Task 16 step 6);
/// this is the function behind it.
pub fn main(vault: &Path, dry_run: bool, via: &str, run_id: Option<&str>) -> i32 {
    main_with_fetchers(vault, dry_run, via, run_id, None)
}

/// [`main`] with the source seam exposed.
///
/// Python's tests reach this by monkeypatching `engine.coursework.fetch_zybooks`; Rust has no
/// such handle, so the seam `collect` already carries is threaded one level up. `main` passes
/// `None` and nothing in production supplies anything else -- the parameter exists so the
/// run-log summary, which is the only durable diagnostic under Task Scheduler, can be tested
/// against a source that fails the way a dead session does.
pub fn main_with_fetchers(
    vault: &Path,
    dry_run: bool,
    via: &str,
    run_id: Option<&str>,
    fetchers: Option<&[(&str, Fetcher)]>,
) -> i32 {
    main_with_sources(vault, dry_run, via, run_id, fetchers, None)
}

/// [`main_with_fetchers`] with the completion sink exposed too (stream J T8).
///
/// Every fetcher of a run reports the vendors' completion figures into one [`CompletionSink`];
/// after the sync, each figure at 100% whose uid names an active task note becomes one
/// `status: done` amend card ([`crate::completion::propose_vendor_completions`]). `None` means the
/// run owns its sink — the production path, where the default fetchers and the cloud path both
/// fill it. A test passes its own so a stand-in fetcher can report a figure the way a real one does.
pub fn main_with_sources(
    vault: &Path,
    dry_run: bool,
    via: &str,
    run_id: Option<&str>,
    fetchers: Option<&[(&str, Fetcher)]>,
    completions: Option<&CompletionSink>,
) -> i32 {
    let own_sink = CompletionSink::default();
    let sink = completions.unwrap_or(&own_sink);
    let zybooks_into = |cfg: &Mapping, tz: &TimeZone, warnings: &mut Vec<String>| {
        fetch_zybooks_into(cfg, tz, warnings, sink)
    };
    let vhl_into = |cfg: &Mapping, tz: &TimeZone, warnings: &mut Vec<String>| {
        fetch_vhl_into(cfg, tz, warnings, sink)
    };
    let defaults: [(&str, Fetcher); 2] = [("zybooks", &zybooks_into), ("vhl", &vhl_into)];
    let local_fetchers: &[(&str, Fetcher)] = fetchers.unwrap_or(&defaults);
    // `sync_coursework` re-actors this per item (`agent:coursework.zybooks` / `.vhl`); only `via`
    // and `run_id` need to survive from the command line. Without them the runner's writes journal
    // as `via: cli, run_id: null` — indistinguishable from someone typing the command by hand, and
    // unattributable to the run that made them.
    let ctx = WriteContext {
        actor: "agent:coursework".to_string(),
        via: via.to_string(),
        run_id: run_id.map(str::to_string),
    };

    // Plan 2 Task 8 (S2 §15, F7): coursework joins RUNS. Its own runner name — not
    // `local` — so `expected_status`, which only knows the runners in config/runners.yaml,
    // never mistakes a coursework start for the slot's rank run. A dry run writes nothing:
    // a pass that changed no note is not a run.
    //
    // Shadowed AFTER `ctx` on purpose. `ctx.run_id` keeps the CALLER's value, so a
    // hand-typed `knowlu-engine coursework` still journals `run_id: null` — which
    // `main_defaults_to_cli_via` pins. The consequence is that the run record's counts are
    // not joinable to the journal records they describe unless the caller passed an id;
    // that is the plan's choice, and it is written down rather than quietly changed here.
    let run_id = if dry_run {
        None
    } else {
        Some(run_id.map(str::to_string).unwrap_or_else(|| {
            crate::runs::start_run(vault, "coursework", None, None, crate::runs::git_sha(vault).as_deref())
        }))
    };

    let mut warnings: Vec<String> = Vec::new();
    let mut log: Vec<String> = Vec::new();
    let mut assignments: Vec<Assignment> = Vec::new();
    // H12 / R-OB-1: today's date, computed once and shared by the map-card machinery below —
    // both when a proposal is stamped and when a still-unmapped book is asked about again.
    let today = jiff::Zoned::now().date();
    let outcome = (|| -> Result<(), SourceError> {
        // H12: an approved coursework-map card is applied BEFORE this run fetches, so a card
        // approved in the console at 11am is a mapping the noon slot already uses. `dry_run`
        // threaded through: a dry run reads and reports, and writes nothing (R-C2-E18 fix 1).
        let mut map_journal = Journal::new(vault);
        log.extend(apply_map_cards(vault, &ctx, &mut map_journal, dry_run));
        let (config, config_warnings) = load_coursework_config(vault)?;
        warnings.extend(config_warnings);
        assignments = match (fetchers, crate::cloudmodel::resolve(vault).ok()) {
            // `fetchers` is the test seam and always wins; a vault with an account parses on the
            // server (§4.3); everything else is plan-3a's local path, unchanged until C4.
            (None, Some(client)) => collect_cloud(
                vault,
                &config,
                &mut warnings,
                &ctx,
                today,
                dry_run,
                &client,
                &mut sink.borrow_mut(),
            ),
            _ => collect(vault, &config, &mut warnings, Some(local_fetchers)),
        };
        if !assignments.is_empty() {
            log.extend(sync_coursework(&assignments, vault, None, dry_run, Some(&ctx), None)?);
        }
        // Stream J T8: after the sync, so a note created this run can be proposed this run. Its
        // log lines (`proposed done: …`) start with none of the sync step's counted prefixes.
        let (done_log, done_warnings) = crate::completion::propose_vendor_completions(
            vault,
            &sink.borrow(),
            today,
            &ctx,
            dry_run,
        );
        log.extend(done_log);
        warnings.extend(done_warnings);
        Ok(())
    })();
    if let Err(err) = outcome {
        warnings.push(format!("coursework pass failed ({err})"));
        assignments = Vec::new();
    }

    // One step per source with its item count, then the sync counts. A source's warnings
    // are matched by the `{name}: ` prefix `collect` gives every one of them; run-level
    // warnings (`config unreadable`, `coursework pass failed`) name no source and so leave
    // both source steps `ok` while the run's own result is still WARN — which is right:
    // the failure was not zyBooks' or VHL's.
    let mut steps: Vec<crate::ledger::Record> = Vec::new();
    if let Some(rid) = run_id.as_deref() {
        for source in ["zybooks", "vhl"] {
            let prefix = format!("{source}:");
            let items =
                assignments.iter().filter(|a| a.uid.starts_with(&prefix)).count() as i64;
            let source_warns: Vec<&str> = warnings
                .iter()
                .filter(|w| w.starts_with(&format!("{source}: ")))
                .map(String::as_str)
                .collect();
            let result = if source_warns.is_empty() { "ok" } else { "WARN" };
            let message = source_warns.join("\n");
            let counts = vec![("items", items)];
            crate::runs::add_step(vault, rid, source, result, &counts, &message, None);
            steps.push(crate::cli::step_record(&crate::cli::Step {
                name: source,
                result,
                counts,
                message,
            }));
        }
        let created = log.iter().filter(|l| l.starts_with("created")).count() as i64;
        let updated = log.iter().filter(|l| l.starts_with("updated")).count() as i64;
        let skipped = log.iter().filter(|l| l.starts_with("skipped")).count() as i64;
        let counts = vec![("created", created), ("updated", updated), ("skipped", skipped)];
        crate::runs::add_step(vault, rid, "sync", "ok", &counts, "", None);
        steps.push(crate::cli::step_record(&crate::cli::Step {
            name: "sync",
            result: "ok",
            counts,
            message: String::new(),
        }));
    }

    for line in &log {
        println!("{line}");
    }
    for line in &warnings {
        println!("WARN {line}");
    }

    if !dry_run {
        let created = log.iter().filter(|l| l.starts_with("created")).count();
        let updated = log.iter().filter(|l| l.starts_with("updated")).count();
        let mut summary = format!(
            "coursework ({} assignments; {created} created, {updated} updated",
            assignments.len()
        );
        if !warnings.is_empty() {
            // Ranked only for the pick; the printed WARN lines above stay in source order, which
            // is what a human reading a terminal wants.
            let ranked = rank_warnings(&warnings);
            let extra = if ranked.len() > 1 {
                format!(" (+{} more)", ranked.len() - 1)
            } else {
                String::new()
            };
            summary.push_str(&format!("; {}{extra}", ranked[0]));
        }
        summary.push(')');
        let status = if warnings.is_empty() { "ok" } else { "WARN" };
        let _ = crate::cli::append_run_log(vault, "local", status, &summary, None);
        if let Some(rid) = run_id.as_deref() {
            // `journal_records` is 0 deliberately: `sync_coursework` journals through
            // `write` under its own actor, and the journal already holds its own record of
            // every one of those writes. Counting them here would report the same work twice.
            crate::runs::end_run(
                vault,
                rid,
                status,
                &summary,
                0,
                None,
                crate::runs::git_sha(vault).as_deref(),
                &steps,
                Some("coursework"),
            );
        }
    }
    0
}

#[cfg(test)]
mod tests {
    //! The `parse_duration_hours` half of `tests/test_coursework.py`. It is ported ahead of the
    //! rest of that file because `vhl` cannot parse a bucket without it — VHL's `estimated_time`
    //! is the only effort number in the system that comes from a vendor rather than a judgment.

    use super::*;

    // --- the sync (plan Task 12) --------------------------------------------------------------
    //
    // `test_assignment_is_frozen` has no port: Python needs `@dataclass(frozen=True)` to get what
    // Rust gives for free — an `Assignment` handed out by value cannot be mutated through a shared
    // reference, and `sync_coursework` takes `&[Assignment]`.

    use std::path::PathBuf;

    use jiff::civil::date;

    fn scratch(name: &str) -> PathBuf {
        let dir = std::env::temp_dir().join(format!(
            "qo-cw-{name}-{}-{:?}",
            std::process::id(),
            std::thread::current().id()
        ));
        let _ = std::fs::remove_dir_all(&dir);
        dir
    }

    /// `vault_with(tmp_path)` — `tasks/`, `archive/` and `state/`.
    fn vault_with(name: &str) -> PathBuf {
        let vault = scratch(name);
        for folder in ["tasks", "archive", "state"] {
            std::fs::create_dir_all(vault.join(folder)).unwrap();
        }
        vault
    }

    /// A `std::env::temp_dir()`-rooted vault carrying `config/`, `approvals/`, `archive/`,
    /// `state/` and `tasks/` — never the worktree (R-3a-12). Unique per `tag`, the same way
    /// [`scratch`] is unique per `name`, so the coursework-map tests below can run concurrently
    /// without treading on each other's `config/ingest.yaml` or `approvals/`.
    fn scratch_vault(tag: &str) -> PathBuf {
        let vault = scratch(&format!("map-{tag}"));
        for folder in ["config", "approvals", "archive", "state", "tasks"] {
            std::fs::create_dir_all(vault.join(folder)).unwrap();
        }
        vault
    }

    /// Every file under `vault`, as `(path relative to vault, its bytes)`, sorted — a byte-level
    /// fingerprint of the whole tree. R-C2-E18 fix 1's dry-run test compares one of these taken
    /// before against one taken after, rather than guessing which files a leak might land in.
    fn snapshot(vault: &Path) -> Vec<(String, Vec<u8>)> {
        fn walk(dir: &Path, root: &Path, out: &mut Vec<(String, Vec<u8>)>) {
            let Ok(entries) = std::fs::read_dir(dir) else { return };
            for entry in entries.flatten() {
                let path = entry.path();
                if path.is_dir() {
                    walk(&path, root, out);
                } else if let Ok(bytes) = std::fs::read(&path) {
                    let rel = path.strip_prefix(root).unwrap().to_string_lossy().replace('\\', "/");
                    out.push((rel, bytes));
                }
            }
        }
        let mut out = Vec::new();
        walk(vault, vault, &mut out);
        out.sort_by(|a, b| a.0.cmp(&b.0));
        out
    }

    fn make(uid: &str, slug: &str, title: &str, due: DateTime, progress: i64) -> Assignment {
        Assignment {
            uid: uid.to_string(),
            slug: slug.to_string(),
            title: title.to_string(),
            due,
            course: Some("cs-100".to_string()),
            effort_hours: 2.5,
            effort_confidence: "low".to_string(),
            effort_source: "inferred".to_string(),
            importance: 2,
            importance_reason: "reason text".to_string(),
            progress,
            created_by: "zybooks".to_string(),
            body: "body text".to_string(),
        }
    }

    fn plain() -> Assignment {
        make(
            "zybooks:1",
            "cs-100-hw-01",
            "CS 100 HW 01",
            date(2026, 8, 26).at(23, 59, 0, 0),
            0,
        )
    }

    const VENDOR_NOTE: &str = "gn-103-hausaufgaben-2026-08-28.md";

    fn make_vendor(effort: f64, progress: i64, title: &str) -> Assignment {
        Assignment {
            uid: "vhl:2102121:2026-08-28".to_string(),
            slug: "gn-103-hausaufgaben-2026-08-28".to_string(),
            title: title.to_string(),
            due: date(2026, 8, 28).at(23, 59, 0, 0),
            course: Some("gn-103".to_string()),
            effort_hours: effort,
            effort_confidence: "high".to_string(),
            effort_source: "vendor".to_string(),
            importance: 3,
            importance_reason: "no late Hausaufgaben accepted".to_string(),
            progress,
            created_by: "vhl".to_string(),
            body: "body text".to_string(),
        }
    }

    const VENDOR_TITLE: &str = "GN 103 Hausaufgaben — due Fri 08-28 (21 activities)";

    fn sync(items: &[Assignment], vault: &Path, dry_run: bool) -> Vec<String> {
        sync_coursework(items, vault, Some(date(2026, 8, 25)), dry_run, None, None)
            .expect("the sync completed")
    }

    fn meta_of(path: &Path) -> Mapping {
        let text = pystr::read_text(path).unwrap();
        crate::models::split_frontmatter(&text).unwrap().0
    }

    fn body_of(path: &Path) -> String {
        let text = pystr::read_text(path).unwrap();
        crate::models::split_frontmatter(&text).unwrap().1
    }

    fn field(meta: &Mapping, key: &str) -> String {
        crate::yaml::get(meta, key).map(yaml_str).unwrap_or_default()
    }

    fn journal_records(vault: &Path) -> Vec<serde_json::Value> {
        let dir = vault.join("state").join("journal");
        let mut files: Vec<PathBuf> = match std::fs::read_dir(&dir) {
            Ok(entries) => entries.flatten().map(|e| e.path()).collect(),
            Err(_) => return Vec::new(),
        };
        files.sort();
        let mut out = Vec::new();
        for file in files {
            let text = pystr::read_text(&file).unwrap_or_default();
            for line in pystr::splitlines(&text) {
                if !pystr::strip(line).is_empty() {
                    if let Ok(value) = serde_json::from_str(line) {
                        out.push(value);
                    }
                }
            }
        }
        out
    }

    fn write_note(path: &Path, text: &str) {
        std::fs::create_dir_all(path.parent().unwrap()).unwrap();
        pystr::write_text(path, text).unwrap();
    }

    // --- config -------------------------------------------------------------------------------

    #[test]
    fn load_config_absent_is_silent() {
        let vault = scratch("cfgabsent");
        std::fs::create_dir_all(vault.join("config")).unwrap();
        let (config, warnings) = load_coursework_config(&vault).unwrap();
        assert!(config.is_empty());
        assert!(warnings.is_empty());
        let _ = std::fs::remove_dir_all(&vault);
    }

    #[test]
    fn load_config_unreadable_warns() {
        let vault = scratch("cfgbad");
        std::fs::create_dir_all(vault.join("config")).unwrap();
        write_note(
            &vault.join("config").join("ingest.yaml"),
            "coursework: [unclosed",
        );
        let (config, warnings) = load_coursework_config(&vault).unwrap();
        assert!(config.is_empty());
        assert_eq!(warnings.len(), 1);
        assert!(warnings[0].contains("unreadable"), "{warnings:?}");
        let _ = std::fs::remove_dir_all(&vault);
    }

    #[test]
    fn load_config_returns_coursework_block() {
        let vault = scratch("cfgok");
        std::fs::create_dir_all(vault.join("config")).unwrap();
        write_note(
            &vault.join("config").join("ingest.yaml"),
            "timezone: America/Chicago\ncoursework:\n  zybooks:\n    enabled: true\n",
        );
        let (config, warnings) = load_coursework_config(&vault).unwrap();
        assert!(warnings.is_empty());
        let coursework = crate::yaml::get(&config, "coursework")
            .and_then(|v| v.as_mapping().cloned())
            .unwrap();
        let zybooks = crate::yaml::get(&coursework, "zybooks")
            .and_then(|v| v.as_mapping().cloned())
            .unwrap();
        assert_eq!(
            crate::yaml::get(&zybooks, "enabled"),
            Some(&Yaml::Bool(true))
        );
        let _ = std::fs::remove_dir_all(&vault);
    }

    #[test]
    fn a_config_that_is_not_utf8_escapes_into_mains_outer_handler() {
        // load_coursework_config catches only (yaml.YAMLError, OSError); UnicodeDecodeError is
        // neither, and main's try/except is what makes "never crash the run" hold for it.
        let vault = scratch("cfgutf8");
        std::fs::create_dir_all(vault.join("config")).unwrap();
        std::fs::write(
            vault.join("config").join("ingest.yaml"),
            b"\xff\xfe not valid utf-8 \x80\x81",
        )
        .unwrap();
        assert!(load_coursework_config(&vault).is_err());
        let _ = std::fs::remove_dir_all(&vault);
    }

    #[test]
    fn resolve_timezone_falls_back_and_warns() {
        let mut warnings = Vec::new();
        let tz = resolve_timezone(
            &crate::yaml::mapping_of("timezone: Mars/Olympus\n"),
            &mut warnings,
        );
        assert_eq!(tz, TimeZone::get(DEFAULT_TZ).unwrap());
        assert_eq!(warnings.len(), 1);
        assert!(warnings[0].contains("Mars/Olympus"), "{warnings:?}");
    }

    #[test]
    fn a_missing_timezone_key_defaults_silently_but_a_null_one_warns() {
        // `config.get("timezone", DEFAULT_TZ)` returns the value even when it is null, and
        // `ZoneInfo(None)` then raises.
        let mut warnings = Vec::new();
        assert_eq!(
            resolve_timezone(&Mapping::new(), &mut warnings),
            TimeZone::get(DEFAULT_TZ).unwrap()
        );
        assert!(warnings.is_empty());
        let mut warnings = Vec::new();
        resolve_timezone(&crate::yaml::mapping_of("timezone: ~\n"), &mut warnings);
        assert_eq!(warnings.len(), 1);
        assert!(warnings[0].contains("None"), "{warnings:?}");
    }

    // --- creating -----------------------------------------------------------------------------

    #[test]
    fn creates_a_note_with_every_field() {
        let vault = vault_with("create");
        let log = sync(&[plain()], &vault, false);
        let path = vault.join("tasks").join("cs-100-hw-01.md");
        assert!(path.is_file());
        let meta = meta_of(&path);
        assert_eq!(field(&meta, "title"), "CS 100 HW 01");
        assert_eq!(field(&meta, "course"), "cs-100");
        assert_eq!(field(&meta, "due"), "2026-08-26T23:59");
        assert_eq!(field(&meta, "effort_hours"), "2.5");
        assert_eq!(field(&meta, "effort_source"), "inferred");
        assert_eq!(field(&meta, "importance"), "2");
        assert_eq!(field(&meta, "status"), "active");
        assert_eq!(field(&meta, "created_by"), "zybooks");
        assert_eq!(field(&meta, "source_uid"), "zybooks:1");
        assert!(body_of(&path).contains("body text"));
        assert!(log.iter().any(|l| l.contains("created")));
        let _ = std::fs::remove_dir_all(&vault);
    }

    #[test]
    fn creating_records_the_ledger_line() {
        let vault = vault_with("ledger");
        sync(&[plain()], &vault, false);
        let ledger = pystr::read_text(&vault.join("state").join("ingest-seen.md")).unwrap();
        assert!(ledger.contains("zybooks:1"), "{ledger}");
        let _ = std::fs::remove_dir_all(&vault);
    }

    #[test]
    fn second_run_does_not_duplicate() {
        let vault = vault_with("nodup");
        sync(&[plain()], &vault, false);
        sync(&[plain()], &vault, false);
        let count = std::fs::read_dir(vault.join("tasks")).unwrap().count();
        assert_eq!(count, 1);
        let _ = std::fs::remove_dir_all(&vault);
    }

    #[test]
    fn filename_collision_gets_a_suffix() {
        let vault = vault_with("collide");
        let squatter = vault.join("tasks").join("cs-100-hw-01.md");
        write_note(
            &squatter,
            "---\ntitle: \"Unrelated\"\ndue: 2026-08-26\nstatus: active\n---\n\nbody\n",
        );
        sync(&[plain()], &vault, false);
        assert!(vault.join("tasks").join("cs-100-hw-01-2.md").is_file());
        assert!(pystr::read_text(&squatter).unwrap().contains("Unrelated"));
        let _ = std::fs::remove_dir_all(&vault);
    }

    #[test]
    fn course_containing_a_colon_still_yields_loadable_frontmatter() {
        // An unquoted colon in a YAML scalar raises ScannerError, and the loader then drops the
        // note from ranking silently.
        let vault = vault_with("colon");
        let mut item = plain();
        item.slug = "odd-course".to_string();
        item.title = "HW 9".to_string();
        item.course = Some("cs: 100".to_string());
        sync(&[item], &vault, false);
        let meta = meta_of(&vault.join("tasks").join("odd-course.md"));
        assert_eq!(field(&meta, "course"), "cs: 100");
        let _ = std::fs::remove_dir_all(&vault);
    }

    #[test]
    fn a_course_that_is_none_or_empty_is_written_as_null() {
        // `json.dumps(item.course) if item.course else "null"` — an empty string is falsy.
        let vault = vault_with("nullcourse");
        let mut item = plain();
        item.course = Some(String::new());
        sync(&[item], &vault, false);
        let text = pystr::read_text(&vault.join("tasks").join("cs-100-hw-01.md")).unwrap();
        assert!(text.contains("course: null"), "{text}");
        let _ = std::fs::remove_dir_all(&vault);
    }

    #[test]
    fn non_ascii_title_is_written_literally_and_stays_idempotent() {
        // json.dumps defaults to ensure_ascii=True, which would write VHL's em dash as a
        // backslash-u escape — what Quinn then sees on opening the note.
        let vault = vault_with("emdash");
        let mut item = plain();
        item.title = VENDOR_TITLE.to_string();
        sync(std::slice::from_ref(&item), &vault, false);
        let text = pystr::read_text(&vault.join("tasks").join("cs-100-hw-01.md")).unwrap();
        assert!(text.contains('—'), "{text}");
        assert!(!text.contains("\\u2014"), "{text}");
        let log = sync(&[item], &vault, false);
        assert!(!log.iter().any(|l| l.contains("updated")), "{log:?}");
        let _ = std::fs::remove_dir_all(&vault);
    }

    // --- updating in place --------------------------------------------------------------------

    #[test]
    fn changed_due_date_updates_in_place() {
        let vault = vault_with("duechange");
        sync(&[plain()], &vault, false);
        let mut moved = plain();
        moved.due = date(2026, 9, 2).at(23, 59, 0, 0);
        let log = sync(&[moved], &vault, false);
        let meta = meta_of(&vault.join("tasks").join("cs-100-hw-01.md"));
        assert_eq!(field(&meta, "due"), "2026-09-02T23:59");
        assert!(log.iter().any(|l| l.contains("updated")), "{log:?}");
        let _ = std::fs::remove_dir_all(&vault);
    }

    #[test]
    fn adopts_an_existing_note_by_uid_regardless_of_filename() {
        // The hand-backfilled notes live at names the parser would not generate. Adoption is by
        // source_uid in frontmatter, never by filename.
        let vault = vault_with("adopt");
        let hand = vault.join("tasks").join("hand-written-name.md");
        write_note(
            &hand,
            "---\ntitle: \"Old title\"\ncourse: cs-100\ndomain: school\n\
             due: 2026-08-26T23:59\neffort_hours: 9.0\nimportance: 2\n\
             status: active\nprogress: 0\ncreated_by: claude\n\
             source_uid: \"zybooks:1\"\n---\n\nkeep this body\n",
        );
        let mut item = plain();
        item.title = "CS 100 HW 01 Introduction to C".to_string();
        sync(&[item], &vault, false);
        assert!(!vault.join("tasks").join("cs-100-hw-01.md").exists());
        let meta = meta_of(&hand);
        assert_eq!(field(&meta, "title"), "CS 100 HW 01 Introduction to C");
        // The sync only ever touches title, due and (vendor) effort_hours.
        assert_eq!(field(&meta, "effort_hours"), "9.0");
        assert!(body_of(&hand).contains("keep this body"));
        let _ = std::fs::remove_dir_all(&vault);
    }

    #[test]
    fn deleted_note_is_not_resurrected() {
        let vault = vault_with("deleted");
        sync(&[plain()], &vault, false);
        std::fs::remove_file(vault.join("tasks").join("cs-100-hw-01.md")).unwrap();
        let log = sync(&[plain()], &vault, false);
        assert!(!vault.join("tasks").join("cs-100-hw-01.md").exists());
        assert!(
            log.iter().any(|l| l.contains("deleted earlier")),
            "{log:?}"
        );
        let _ = std::fs::remove_dir_all(&vault);
    }

    #[test]
    fn archived_note_is_left_alone() {
        let vault = vault_with("archived");
        let archived = vault.join("archive").join("cs-100-hw-01.md");
        write_note(
            &archived,
            "---\ntitle: \"Old\"\ndue: 2026-08-26T23:59\nstatus: done\n\
             source_uid: \"zybooks:1\"\n---\n\nbody\n",
        );
        let mut item = plain();
        item.title = "New title".to_string();
        sync(&[item], &vault, false);
        assert!(pystr::read_text(&archived).unwrap().contains("Old"));
        assert!(!vault.join("tasks").join("cs-100-hw-01.md").exists());
        let _ = std::fs::remove_dir_all(&vault);
    }

    #[test]
    fn done_note_is_left_alone() {
        let vault = vault_with("done");
        let done = vault.join("tasks").join("cs-100-hw-01.md");
        write_note(
            &done,
            "---\ntitle: \"Old\"\ndue: 2026-08-26T23:59\nstatus: done\nprogress: 100\n\
             source_uid: \"zybooks:1\"\n---\n\nbody\n",
        );
        let mut item = plain();
        item.title = "New title".to_string();
        sync(&[item], &vault, false);
        assert!(pystr::read_text(&done).unwrap().contains("Old"));
        let _ = std::fs::remove_dir_all(&vault);
    }

    #[test]
    fn progress_is_never_overwritten_on_an_existing_note() {
        // VHL's percentage_complete seeds progress at creation only. Quinn ticking a note forward
        // must not be reverted by the next fetch.
        let vault = vault_with("progress");
        let item = make(
            "vhl:1:2026-08-26",
            "gn-103-x",
            "CS 100 HW 01",
            date(2026, 8, 26).at(23, 59, 0, 0),
            10,
        );
        sync(std::slice::from_ref(&item), &vault, false);
        let path = vault.join("tasks").join("gn-103-x.md");
        crate::ingest::update_frontmatter_fields(
            &path,
            &[("progress".to_string(), "80".to_string())],
        )
        .unwrap();
        sync(&[item], &vault, false);
        assert_eq!(field(&meta_of(&path), "progress"), "80");
        let _ = std::fs::remove_dir_all(&vault);
    }

    #[test]
    fn unwritable_note_does_not_stop_the_rest_of_the_batch() {
        // split_frontmatter parses via str.split("---", 2), so it tolerates a closing delimiter
        // line with trailing whitespace. The write path requires an exact "---" line, so it does
        // not. A note in that state passes the read and then fails on the write. One bad note must
        // cost one note, not the tail of the batch.
        let vault = vault_with("unwritable");
        write_note(
            &vault.join("tasks").join("cs-100-hw-01.md"),
            "---\ntitle: \"Old title\"\ndue: 2026-08-20T23:59\nstatus: active\n\
             source_uid: \"zybooks:1\"\n--- \n\nbody\n",
        );
        let mut changed = plain();
        changed.title = "CS 100 HW 01 changed".to_string();
        let healthy = make(
            "zybooks:2",
            "cs-100-hw-02",
            "CS 100 HW 02",
            date(2026, 8, 26).at(23, 59, 0, 0),
            0,
        );
        let log = sync(&[changed, healthy], &vault, false);
        assert!(
            log.iter().any(|l| l.contains("skipped (unwritable)")),
            "{log:?}"
        );
        assert!(vault.join("tasks").join("cs-100-hw-02.md").is_file());
        let _ = std::fs::remove_dir_all(&vault);
    }

    // --- the vendor-effort rule ---------------------------------------------------------------

    #[test]
    fn vendor_effort_is_refreshed_when_the_vendor_number_changes() {
        // VHL buckets grow mid-week. The title already advertises the new activity count, and
        // effort_source: vendor certifies the number as authoritative — so a stale effort_hours
        // would be permanently wrong and self-certified.
        let vault = vault_with("vendorrefresh");
        sync(&[make_vendor(2.53, 0, VENDOR_TITLE)], &vault, false);
        let path = vault.join("tasks").join(VENDOR_NOTE);
        let log = sync(
            &[make_vendor(
                3.1,
                0,
                "GN 103 Hausaufgaben — due Fri 08-28 (25 activities)",
            )],
            &vault,
            false,
        );
        let meta = meta_of(&path);
        assert_eq!(field(&meta, "effort_hours"), "3.1");
        assert_eq!(field(&meta, "effort_source"), "vendor");
        assert!(field(&meta, "title").ends_with("(25 activities)"));
        assert!(body_of(&path).contains("body text"));
        assert!(log.iter().any(|l| l.contains("effort_hours")), "{log:?}");
        let _ = std::fs::remove_dir_all(&vault);
    }

    #[test]
    fn unchanged_vendor_effort_writes_nothing() {
        let vault = vault_with("vendorsame");
        sync(&[make_vendor(2.53, 0, VENDOR_TITLE)], &vault, false);
        let path = vault.join("tasks").join(VENDOR_NOTE);
        let before = std::fs::read(&path).unwrap();
        let log = sync(&[make_vendor(2.53, 0, VENDOR_TITLE)], &vault, false);
        assert_eq!(std::fs::read(&path).unwrap(), before);
        assert!(!log.iter().any(|l| l.contains("updated")), "{log:?}");
        let _ = std::fs::remove_dir_all(&vault);
    }

    #[test]
    fn hand_tuned_effort_is_never_refreshed() {
        // Changing effort_source away from "vendor" is the opt-out: ingest stops touching
        // effort_hours but keeps tracking title and due.
        let vault = vault_with("handtuned");
        sync(&[make_vendor(2.53, 0, VENDOR_TITLE)], &vault, false);
        let path = vault.join("tasks").join(VENDOR_NOTE);
        crate::ingest::update_frontmatter_fields(
            &path,
            &[
                ("effort_source".to_string(), "inferred".to_string()),
                ("effort_hours".to_string(), "9.0".to_string()),
            ],
        )
        .unwrap();
        sync(&[make_vendor(3.1, 0, "a changed title")], &vault, false);
        let meta = meta_of(&path);
        assert_eq!(field(&meta, "effort_hours"), "9.0");
        assert_eq!(field(&meta, "effort_source"), "inferred");
        assert_eq!(field(&meta, "title"), "a changed title");
        let _ = std::fs::remove_dir_all(&vault);
    }

    #[test]
    fn note_without_an_effort_source_is_never_refreshed() {
        let vault = vault_with("nosource");
        let hand = vault.join("tasks").join("hand-written-vhl.md");
        write_note(
            &hand,
            "---\ntitle: \"Old title\"\ncourse: gn-103\ndomain: school\n\
             due: 2026-08-28T23:59\neffort_hours: 9.0\nimportance: 3\n\
             status: active\nprogress: 0\ncreated_by: claude\n\
             source_uid: \"vhl:2102121:2026-08-28\"\n---\n\nkeep this body\n",
        );
        sync(&[make_vendor(3.1, 0, VENDOR_TITLE)], &vault, false);
        let meta = meta_of(&hand);
        assert_eq!(field(&meta, "effort_hours"), "9.0");
        assert!(body_of(&hand).contains("keep this body"));
        let _ = std::fs::remove_dir_all(&vault);
    }

    #[test]
    fn incoming_inferred_effort_never_overwrites_a_vendor_stamped_note() {
        // Only the note's own effort_source used to gate the refresh; requiring the incoming
        // item's effort_source to also be vendor stops a number no vendor produced from being
        // written under a vendor's authority (effort_confidence stays high, unearned).
        let vault = vault_with("bothvendor");
        sync(&[make_vendor(2.53, 0, VENDOR_TITLE)], &vault, false);
        let path = vault.join("tasks").join(VENDOR_NOTE);
        let mut incoming = make_vendor(3.1, 0, VENDOR_TITLE);
        incoming.effort_confidence = "low".to_string();
        incoming.effort_source = "inferred".to_string();
        let log = sync(&[incoming], &vault, false);
        let meta = meta_of(&path);
        assert_eq!(field(&meta, "effort_hours"), "2.53");
        assert_eq!(field(&meta, "effort_source"), "vendor");
        assert!(!log.iter().any(|l| l.contains("effort_hours")), "{log:?}");
        let _ = std::fs::remove_dir_all(&vault);
    }

    #[test]
    fn progress_survives_a_vendor_effort_refresh() {
        let vault = vault_with("progvendor");
        sync(&[make_vendor(2.53, 10, VENDOR_TITLE)], &vault, false);
        let path = vault.join("tasks").join(VENDOR_NOTE);
        crate::ingest::update_frontmatter_fields(
            &path,
            &[("progress".to_string(), "80".to_string())],
        )
        .unwrap();
        sync(&[make_vendor(3.1, 10, VENDOR_TITLE)], &vault, false);
        let meta = meta_of(&path);
        assert_eq!(field(&meta, "progress"), "80");
        assert_eq!(field(&meta, "effort_hours"), "3.1");
        let _ = std::fs::remove_dir_all(&vault);
    }

    #[test]
    fn progress_survives_when_effort_is_not_refreshed() {
        // The same guarantee on the branch that writes only title, not effort_hours.
        let vault = vault_with("proginferred");
        let mut item = plain();
        item.progress = 10;
        sync(std::slice::from_ref(&item), &vault, false);
        let path = vault.join("tasks").join("cs-100-hw-01.md");
        crate::ingest::update_frontmatter_fields(
            &path,
            &[("progress".to_string(), "80".to_string())],
        )
        .unwrap();
        item.title = "changed".to_string();
        sync(&[item], &vault, false);
        let meta = meta_of(&path);
        assert_eq!(field(&meta, "progress"), "80");
        assert_eq!(field(&meta, "effort_hours"), "2.5");
        assert_eq!(field(&meta, "title"), "changed");
        let _ = std::fs::remove_dir_all(&vault);
    }

    // --- dry runs -----------------------------------------------------------------------------

    #[test]
    fn dry_run_writes_nothing() {
        let vault = vault_with("dryrun");
        let log = sync(&[plain()], &vault, true);
        assert!(!vault.join("tasks").join("cs-100-hw-01.md").exists());
        assert!(!vault.join("state").join("ingest-seen.md").exists());
        assert!(log.iter().any(|l| l.contains("would create")), "{log:?}");
        let _ = std::fs::remove_dir_all(&vault);
    }

    #[test]
    fn dry_run_update_leaves_the_note_and_ledger_byte_identical() {
        // This branch produces the "would update ..." output the rollout's review step relies on
        // to confirm the hand-written notes are adopted rather than duplicated. It is the only
        // safeguard before 75 real notes.
        let vault = vault_with("dryupdate");
        sync(&[plain()], &vault, false);
        let path = vault.join("tasks").join("cs-100-hw-01.md");
        let ledger = vault.join("state").join("ingest-seen.md");
        let note_before = std::fs::read(&path).unwrap();
        let ledger_before = std::fs::read(&ledger).unwrap();

        let mut changed = plain();
        changed.title = "CS 100 HW 01 Introduction to C".to_string();
        changed.due = date(2026, 9, 2).at(23, 59, 0, 0);
        let log = sync(&[changed], &vault, true);

        let line = log
            .iter()
            .find(|l| l.contains("would update"))
            .expect("a would-update line");
        assert!(line.contains("cs-100-hw-01"), "{line}");
        assert!(line.contains("title") && line.contains("due"), "{line}");
        assert_eq!(std::fs::read(&path).unwrap(), note_before);
        assert_eq!(std::fs::read(&ledger).unwrap(), ledger_before);
        assert_eq!(std::fs::read_dir(vault.join("tasks")).unwrap().count(), 1);
        let _ = std::fs::remove_dir_all(&vault);
    }

    #[test]
    fn dry_run_update_records_no_new_ledger_line_for_an_unseen_uid() {
        // An adopted hand-written note is known by uid but absent from the ledger. A dry run must
        // not write that line either.
        let vault = vault_with("dryunseen");
        let hand = vault.join("tasks").join("hand-written-name.md");
        write_note(
            &hand,
            "---\ntitle: \"Old title\"\ncourse: cs-100\ndomain: school\n\
             due: 2026-08-26T23:59\neffort_hours: 9.0\nimportance: 2\n\
             status: active\nprogress: 0\ncreated_by: claude\n\
             source_uid: \"zybooks:1\"\n---\n\nkeep this body\n",
        );
        let before = std::fs::read(&hand).unwrap();
        let mut item = plain();
        item.title = "CS 100 HW 01 Introduction to C".to_string();
        let log = sync(&[item], &vault, true);
        assert!(
            log.iter().any(|l| l.contains("would update hand-written-name")),
            "{log:?}"
        );
        assert_eq!(std::fs::read(&hand).unwrap(), before);
        assert!(!vault.join("state").join("ingest-seen.md").exists());
        let _ = std::fs::remove_dir_all(&vault);
    }

    #[test]
    fn dry_run_writes_no_journal() {
        let vault = vault_with("dryjournal");
        let item = Assignment {
            uid: "vhl:2026-08-31".to_string(),
            slug: "gn-103-hausaufgaben-2026-08-31".to_string(),
            title: "HA".to_string(),
            due: date(2026, 8, 31).at(23, 59, 0, 0),
            course: Some("gn-103".to_string()),
            effort_hours: 3.1,
            effort_confidence: "high".to_string(),
            effort_source: "vendor".to_string(),
            importance: 3,
            importance_reason: "x".to_string(),
            progress: 10,
            created_by: "vhl".to_string(),
            body: String::new(),
        };
        let log = sync(&[item], &vault, true);
        assert_eq!(
            log,
            vec!["would create gn-103-hausaufgaben-2026-08-31 (due 2026-08-31T23:59)"]
        );
        assert!(!vault.join("state").join("journal").exists());
        let _ = std::fs::remove_dir_all(&vault);
    }

    // --- journalling --------------------------------------------------------------------------

    #[test]
    fn sync_coursework_journals_with_vendor_actor_and_never_touches_progress() {
        let vault = vault_with("journalactor");
        let item = Assignment {
            uid: "zybooks:1".to_string(),
            slug: "cs-100-hw-01".to_string(),
            title: "HW 01".to_string(),
            due: date(2026, 8, 26).at(23, 59, 0, 0),
            course: Some("cs-100".to_string()),
            effort_hours: 2.5,
            effort_confidence: "high".to_string(),
            effort_source: "vendor".to_string(),
            importance: 2,
            importance_reason: "x".to_string(),
            progress: 0,
            created_by: "zybooks".to_string(),
            body: String::new(),
        };
        sync(std::slice::from_ref(&item), &vault, false);
        let records = journal_records(&vault);
        assert_eq!(
            records[records.len() - 1]["actor"],
            serde_json::json!("agent:coursework.zybooks")
        );

        // Quinn ticks progress by hand; a vendor refresh must not touch it (F19).
        let note = vault.join("tasks").join("cs-100-hw-01.md");
        let text = pystr::read_text(&note).unwrap();
        pystr::write_text(&note, &text.replace("progress: 0", "progress: 40")).unwrap();
        let mut second = item.clone();
        second.effort_hours = 3.0;
        second.progress = 100;
        sync_coursework(
            &[second],
            &vault,
            Some(date(2026, 8, 26)),
            false,
            None,
            None,
        )
        .unwrap();
        assert!(pystr::read_text(&note).unwrap().contains("progress: 40"));
        let set_fields: Vec<String> = journal_records(&vault)
            .iter()
            .filter(|r| r["op"] == serde_json::json!("set"))
            .map(|r| r["field"].as_str().unwrap_or_default().to_string())
            .collect();
        assert_eq!(set_fields, vec!["effort_hours"]);
        let _ = std::fs::remove_dir_all(&vault);
    }

    // --- collect ------------------------------------------------------------------------------

    fn base_config() -> Mapping {
        crate::yaml::mapping_of(concat!(
            "timezone: America/Chicago\n",
            "coursework:\n",
            "  zybooks:\n",
            "    enabled: true\n",
            "    credential_target: knowlu/zybooks\n",
            "  vhl:\n",
            "    enabled: true\n",
            "    credential_target: knowlu/vhl\n",
        ))
    }

    #[test]
    fn disabled_sources_fetch_nothing() {
        let config =
            crate::yaml::mapping_of("coursework:\n  zybooks:\n    enabled: false\n  vhl:\n    enabled: false\n");
        let boom = |_: &Mapping, _: &TimeZone, _: &mut Vec<String>| -> Result<Vec<Assignment>, SourceError> {
            panic!("must not fetch when disabled")
        };
        let fetchers: [(&str, Fetcher); 2] = [("zybooks", &boom), ("vhl", &boom)];
        let mut warnings = Vec::new();
        let items = collect(Path::new("."), &config, &mut warnings, Some(&fetchers));
        assert!(items.is_empty());
        assert!(warnings.is_empty(), "{warnings:?}");
    }

    #[test]
    fn a_failing_source_warns_and_the_other_still_runs() {
        let zy_boom =
            |_: &Mapping, _: &TimeZone, _: &mut Vec<String>| -> Result<Vec<Assignment>, SourceError> {
                Err(SourceError::Failed("network down".to_string()))
            };
        let vhl_ok =
            |_: &Mapping, _: &TimeZone, _: &mut Vec<String>| -> Result<Vec<Assignment>, SourceError> {
                Ok(vec![make(
                    "vhl:2102121:2026-08-26",
                    "gn-103-x",
                    "CS 100 HW 01",
                    date(2026, 8, 26).at(23, 59, 0, 0),
                    0,
                )])
            };
        let fetchers: [(&str, Fetcher); 2] = [("zybooks", &zy_boom), ("vhl", &vhl_ok)];
        let mut warnings = Vec::new();
        let items = collect(Path::new("."), &base_config(), &mut warnings, Some(&fetchers));
        assert_eq!(items.len(), 1);
        assert!(
            warnings
                .iter()
                .any(|w| w.contains("zybooks") && w.contains("network down")),
            "{warnings:?}"
        );
    }

    #[test]
    fn empty_result_from_an_enabled_source_is_a_warning() {
        // The guard that matters most: a 200 response yielding nothing is far more likely to be a
        // dead session than a semester with no homework in it.
        let empty =
            |_: &Mapping, _: &TimeZone, _: &mut Vec<String>| -> Result<Vec<Assignment>, SourceError> {
                Ok(Vec::new())
            };
        let fetchers: [(&str, Fetcher); 2] = [("zybooks", &empty), ("vhl", &empty)];
        let mut warnings = Vec::new();
        let items = collect(Path::new("."), &base_config(), &mut warnings, Some(&fetchers));
        assert!(items.is_empty());
        assert_eq!(
            warnings
                .iter()
                .filter(|w| w.contains("0 assignments"))
                .count(),
            2
        );
    }

    #[test]
    fn not_logged_in_is_reported_as_a_session_problem() {
        // NotLoggedIn is a cross-source classification: a revoked zyBooks token or an expired VHL
        // session must read as a session problem, not as "0 assignments parsed".
        let expired =
            |_: &Mapping, _: &TimeZone, _: &mut Vec<String>| -> Result<Vec<Assignment>, SourceError> {
                Err(SourceError::NotLoggedIn(
                    "dashboard mount element absent".to_string(),
                ))
            };
        let ok = |_: &Mapping, _: &TimeZone, _: &mut Vec<String>| -> Result<Vec<Assignment>, SourceError> {
            Ok(vec![plain()])
        };
        for (name, fetchers) in [
            ("vhl", [("zybooks", &ok as Fetcher), ("vhl", &expired)]),
            ("zybooks", [("zybooks", &expired as Fetcher), ("vhl", &ok)]),
        ] {
            let mut warnings = Vec::new();
            collect(Path::new("."), &base_config(), &mut warnings, Some(&fetchers));
            assert!(
                warnings
                    .iter()
                    .any(|w| w.contains(name) && w.to_lowercase().contains("session")),
                "{warnings:?}"
            );
            assert!(
                !warnings.iter().any(|w| w.contains("0 assignments")),
                "{warnings:?}"
            );
        }
    }

    #[test]
    fn a_sources_own_warnings_are_prefixed_with_its_name() {
        let noisy =
            |_: &Mapping, _: &TimeZone, warnings: &mut Vec<String>| -> Result<Vec<Assignment>, SourceError> {
                warnings.push("Midterm Reflection: uncategorised".to_string());
                Ok(vec![plain()])
            };
        let quiet =
            |_: &Mapping, _: &TimeZone, _: &mut Vec<String>| -> Result<Vec<Assignment>, SourceError> {
                Ok(vec![plain()])
            };
        let fetchers: [(&str, Fetcher); 2] = [("zybooks", &noisy), ("vhl", &quiet)];
        let mut warnings = Vec::new();
        collect(Path::new("."), &base_config(), &mut warnings, Some(&fetchers));
        assert_eq!(warnings, vec!["zybooks: Midterm Reflection: uncategorised"]);
    }

    // --- preserved defects (see docs/superpowers/reports/2026-09-01-preserved-python-defects.md)

    /// **Preserved defect 18.** The create branch never adds the uid to the in-memory `seen` set —
    /// only the update branch does. So the same uid twice in one batch takes the update path the
    /// second time, finds itself absent from `seen`, and appends a **second** ledger line.
    ///
    /// Verified against the interpreter, not inferred: Python writes
    /// `- zybooks:1 · CS 100 HW 01 · first seen 2026-08-25` and
    /// `- zybooks:1 · CS 100 HW 01 renamed · first seen 2026-08-25`.
    ///
    /// `state/ingest-seen.md` is `merge=union`, so two *different* lines for one uid both survive
    /// a two-device sync — permanently. Reading still dedups (`load_seen` keys on the uid), so the
    /// harm is a ledger that grows a wrong "first seen" title rather than a resurrected task. Not
    /// fixed: the oracle is behaviour parity, and one `seen.insert` is exactly the kind of quiet
    /// improvement that makes a diff impossible to trust.
    #[test]
    fn preserved_defect_18_a_uid_twice_in_one_batch_writes_two_ledger_lines() {
        let vault = vault_with("defect18");
        let mut renamed = plain();
        renamed.title = "CS 100 HW 01 renamed".to_string();
        let log = sync(&[plain(), renamed], &vault, false);
        assert_eq!(log, vec!["created cs-100-hw-01", "updated cs-100-hw-01: title"]);
        let ledger = pystr::read_text(&vault.join("state").join("ingest-seen.md")).unwrap();
        assert_eq!(
            ledger.matches("- zybooks:1 ").count(),
            2,
            "defect 18 has been fixed; update the report before changing this test\n{ledger}"
        );
        assert_eq!(std::fs::read_dir(vault.join("tasks")).unwrap().count(), 1);
        let _ = std::fs::remove_dir_all(&vault);
    }

    /// **Preserved defect 19.** `if not source_cfg.get("enabled")` is a *truthiness* test, so a
    /// quoted `enabled: "false"` — a non-empty string — turns the source **on**.
    ///
    /// Verified against the interpreter: PyYAML gives `{'enabled': 'false'}` for the quoted form
    /// and `False` for the bare one, and `collect` then calls the fetcher twice for the first and
    /// never for the second. The symptom of getting it wrong is the opposite of alarming — a
    /// source you believe is off quietly authenticating twice a day.
    #[test]
    fn preserved_defect_19_a_quoted_false_enables_the_source() {
        let called = std::cell::Cell::new(0);
        let counting =
            |_: &Mapping, _: &TimeZone, _: &mut Vec<String>| -> Result<Vec<Assignment>, SourceError> {
                called.set(called.get() + 1);
                Ok(Vec::new())
            };
        let fetchers: [(&str, Fetcher); 2] = [("zybooks", &counting), ("vhl", &counting)];

        let quoted = crate::yaml::mapping_of(
            "coursework:\n  zybooks:\n    enabled: \"false\"\n  vhl:\n    enabled: \"no\"\n",
        );
        collect(Path::new("."), &quoted, &mut Vec::new(), Some(&fetchers));
        assert_eq!(
            called.get(),
            2,
            "defect 19 has been fixed; update the report before changing this test"
        );

        called.set(0);
        let bare = crate::yaml::mapping_of(
            "coursework:\n  zybooks:\n    enabled: false\n  vhl:\n    enabled: false\n",
        );
        collect(Path::new("."), &bare, &mut Vec::new(), Some(&fetchers));
        assert_eq!(called.get(), 0);
    }

    // --- the zyBooks book routing -------------------------------------------------------------

    #[test]
    fn a_configured_ignore_is_silent_but_a_new_code_is_not() {
        // HowToUseZyBooks2 is zyBooks' own onboarding book: zero assignments, never coursework. A
        // WARN for it would fire on every single healthy run.
        let courses = crate::yaml::mapping_of("UACS100Fall2026:\n  course: cs-100\n  label: CS 100\n");
        let ignore = vec!["HowToUseZyBooks2".to_string()];
        assert!(matches!(
            route_zybook("UACS100Fall2026", &courses, &ignore),
            BookRouting::Mapped(_)
        ));
        assert_eq!(
            route_zybook("HowToUseZyBooks2", &courses, &ignore),
            BookRouting::Ignored
        );
        assert_eq!(
            route_zybook("SomeNewCourse", &courses, &ignore),
            BookRouting::Unmapped
        );
    }

    // --- rank_warnings ------------------------------------------------------------------------

    #[test]
    fn rank_warnings_puts_failures_first_and_is_stable_within_each_group() {
        let warnings: Vec<String> = [
            "zybooks: Midterm Reflection: uncategorised; using default importance",
            "vhl: session invalid (dashboard mount element absent); nothing changed",
            "zybooks: HW 42: 2 due dates; using the earliest",
            "zybooks: fetch failed (network down); nothing changed",
        ]
        .iter()
        .map(|s| s.to_string())
        .collect();
        let ranked = rank_warnings(&warnings);
        assert_eq!(ranked[0], warnings[1]);
        assert_eq!(ranked[1], warnings[3]);
        assert_eq!(ranked[2], warnings[0]);
        assert_eq!(ranked[3], warnings[2]);
    }

    #[test]
    fn every_failure_marker_is_one_collect_or_the_config_loader_actually_emits() {
        // The set is closed on purpose — it classifies text this module owns, not arbitrary text.
        for marker in FAILURE_MARKERS {
            assert!(rank_warnings(&[format!("x {marker} y")])[0].contains(marker));
        }
        assert_eq!(FAILURE_MARKERS.len(), 7);
    }

    /// C2 Task 7 fix 1 (R-C2-E17 / I2): a server-side parse failure must rank ahead of a benign
    /// per-item note, exactly as a dead session already does — the run-log summary keeps only one
    /// warning, and it must never be the harmless one.
    #[test]
    fn a_parse_failed_warning_sorts_as_a_failure() {
        let warnings: Vec<String> = vec![
            "zybooks: Midterm Reflection: uncategorised; using default importance".to_string(),
            "vhl: parse failed (server timeout); nothing changed".to_string(),
        ];
        let ranked = rank_warnings(&warnings);
        assert_eq!(ranked[0], warnings[1]);
    }

    // --- main ---------------------------------------------------------------------------------

    fn runnable_vault(name: &str, config: &str) -> PathBuf {
        let vault = scratch(name);
        for folder in ["config", "state", "tasks"] {
            std::fs::create_dir_all(vault.join(folder)).unwrap();
        }
        write_note(&vault.join("config").join("ingest.yaml"), config);
        vault
    }

    fn run_log(vault: &Path) -> String {
        pystr::read_text(&vault.join("state").join("runner-log.md")).unwrap_or_default()
    }

    #[test]
    fn main_returns_zero_even_when_everything_fails() {
        // Coursework must never fail the run: the rank matters more than the fetch.
        let vault = runnable_vault(
            "mainfail",
            "timezone: America/Chicago\ncoursework:\n  zybooks:\n    enabled: true\n",
        );
        let boom =
            |_: &Mapping, _: &TimeZone, _: &mut Vec<String>| -> Result<Vec<Assignment>, SourceError> {
                Err(SourceError::Failed("boom".to_string()))
            };
        let fetchers: [(&str, Fetcher); 1] = [("zybooks", &boom)];
        assert_eq!(
            main_with_fetchers(&vault, false, "cli", None, Some(&fetchers)),
            0
        );
        assert!(run_log(&vault).contains("WARN"), "{}", run_log(&vault));
        let _ = std::fs::remove_dir_all(&vault);
    }

    #[test]
    fn config_with_invalid_utf8_still_returns_zero_and_still_warns() {
        // A missing runner-log line would itself be the failure signature (CLAUDE.md), so this
        // must still land a WARN line, not merely avoid crashing.
        let vault = scratch("mainutf8");
        for folder in ["config", "state", "tasks"] {
            std::fs::create_dir_all(vault.join(folder)).unwrap();
        }
        std::fs::write(
            vault.join("config").join("ingest.yaml"),
            b"\xff\xfe not valid utf-8 \x80\x81",
        )
        .unwrap();
        assert_eq!(main(&vault, false, "cli", None), 0);
        let log = run_log(&vault);
        assert!(log.contains("WARN"), "{log}");
        assert!(log.contains("coursework pass failed"), "{log}");
        assert_eq!(std::fs::read_dir(vault.join("tasks")).unwrap().count(), 0);
        let _ = std::fs::remove_dir_all(&vault);
    }

    #[test]
    fn runner_log_surfaces_the_session_failure_over_a_benign_warning() {
        // collect iterates zybooks-then-vhl, so a harmless zyBooks note used to win warnings[0]
        // and hide a dead VHL session behind "(+1 more)". Under Task Scheduler that summary line
        // is the only durable diagnostic.
        let vault = runnable_vault(
            "mainrank",
            "timezone: America/Chicago\ncoursework:\n  zybooks:\n    enabled: true\n  vhl:\n    enabled: true\n",
        );
        let zy =
            |_: &Mapping, _: &TimeZone, warnings: &mut Vec<String>| -> Result<Vec<Assignment>, SourceError> {
                warnings.push("Midterm Reflection: uncategorised; using default importance".to_string());
                Ok(vec![plain()])
            };
        let vhl =
            |_: &Mapping, _: &TimeZone, _: &mut Vec<String>| -> Result<Vec<Assignment>, SourceError> {
                Err(SourceError::NotLoggedIn(
                    "dashboard mount element absent".to_string(),
                ))
            };
        let fetchers: [(&str, Fetcher); 2] = [("zybooks", &zy), ("vhl", &vhl)];
        assert_eq!(
            main_with_fetchers(&vault, false, "cli", None, Some(&fetchers)),
            0
        );
        let log = run_log(&vault);
        let line = pystr::splitlines(&log)
            .into_iter()
            .find(|l| l.contains("coursework ("))
            .expect("a coursework summary line");
        assert!(line.contains("session invalid"), "{line}");
        assert!(line.contains("(+1 more)"), "{line}");
        assert!(!line.contains("uncategorised"), "{line}");
        let _ = std::fs::remove_dir_all(&vault);
    }

    #[test]
    fn main_journals_under_the_runners_via_and_run_id() {
        // The local runner's coursework pass is a RUN's writes, not someone's typing. Defaulting
        // to `via: cli, run_id: null` makes every synced assignment unattributable to the run that
        // fetched it.
        let vault = runnable_vault(
            "mainvia",
            "timezone: America/Chicago\ncoursework:\n  zybooks:\n    enabled: true\n",
        );
        let ok = |_: &Mapping, _: &TimeZone, _: &mut Vec<String>| -> Result<Vec<Assignment>, SourceError> {
            Ok(vec![plain()])
        };
        let fetchers: [(&str, Fetcher); 1] = [("zybooks", &ok)];
        assert_eq!(
            main_with_fetchers(
                &vault,
                false,
                "local-runner",
                Some("local-2026-08-29T17:00:00Z"),
                Some(&fetchers)
            ),
            0
        );
        let records = journal_records(&vault);
        assert!(!records.is_empty(), "coursework created a note but journalled nothing");
        assert!(records
            .iter()
            .all(|r| r["via"] == serde_json::json!("local-runner")));
        assert!(records
            .iter()
            .all(|r| r["run_id"] == serde_json::json!("local-2026-08-29T17:00:00Z")));
        assert!(records
            .iter()
            .all(|r| r["actor"] == serde_json::json!("agent:coursework.zybooks")));
        let _ = std::fs::remove_dir_all(&vault);
    }

    /// Plan 2 Task 8 (S2 §15, F7): coursework is a runner like the others now — start, one
    /// step per source with its item count, a sync step with created/updated/skipped, and an
    /// end whose summary is the line `append_run_log` already wrote. A dry run writes no
    /// record at all, because a run that changed nothing is not a run.
    #[test]
    fn main_writes_a_coursework_run_record_with_per_source_counts() {
        let vault = runnable_vault(
            "cw-runs",
            "timezone: America/Chicago\ncoursework:\n  zybooks:\n    enabled: true\n  vhl:\n    enabled: true\n",
        );
        let two = |_: &Mapping, _: &TimeZone, _: &mut Vec<String>| -> Result<Vec<Assignment>, SourceError> {
            Ok(vec![
                make("zybooks:1", "cs-100-hw-01", "CS 100 HW 01", date(2026, 8, 26).at(23, 59, 0, 0), 0),
                make("zybooks:2", "cs-100-hw-02", "CS 100 HW 02", date(2026, 8, 27).at(23, 59, 0, 0), 0),
            ])
        };
        let dead = |_: &Mapping, _: &TimeZone, _: &mut Vec<String>| -> Result<Vec<Assignment>, SourceError> {
            Err(SourceError::Failed("no session".into()))
        };
        let fetchers: [(&str, Fetcher); 2] = [("zybooks", &two), ("vhl", &dead)];
        assert_eq!(main_with_fetchers(&vault, false, "local-runner", None, Some(&fetchers)), 0);

        let day = crate::runs::Runs::new(&vault).read(None);
        // Only `start` and `end` carry `runner`; the steps between them are joined by `run_id`.
        // (Indexing a Record with a missing key panics, so the plan's sketch could not be used
        // verbatim here -- and grouping by run_id is what actually identifies one run's records.)
        let rid = day
            .iter()
            .find(|r| r.get("runner").and_then(|v| v.as_str()) == Some("coursework"))
            .and_then(|r| r.get("run_id").and_then(|v| v.as_str()))
            .expect("a coursework run was started")
            .to_string();
        let cw: Vec<_> = day
            .iter()
            .filter(|r| r.get("run_id").and_then(|v| v.as_str()) == Some(rid.as_str()))
            .collect();
        let phases: Vec<&str> = cw.iter().map(|r| r["phase"].as_str().unwrap()).collect();
        assert_eq!(phases, ["start", "step", "step", "step", "end"]);
        let names: Vec<&str> =
            cw.iter().filter(|r| r["phase"] == "step").map(|r| r["name"].as_str().unwrap()).collect();
        assert_eq!(names, ["zybooks", "vhl", "sync"]);
        assert_eq!(cw[1]["counts"]["items"], 2);
        assert_eq!(cw[1]["result"], "ok");
        // vhl fetched nothing and said why: its own step carries the WARN, zybooks stays ok.
        assert_eq!(cw[2]["counts"]["items"], 0);
        assert_eq!(cw[2]["result"], "WARN");
        assert!(cw[2]["message"].as_str().unwrap().starts_with("vhl: fetch failed"));
        assert_eq!(cw[3]["counts"]["created"], 2);
        assert_eq!(cw[4]["result"], "WARN");
        assert!(cw[4]["summary"]
            .as_str()
            .unwrap()
            .starts_with("coursework (2 assignments; 2 created, 0 updated;"));

        // A dry run leaves state/runs/ exactly as it was.
        let before = crate::runs::Runs::new(&vault).read(None).len();
        assert_eq!(main_with_fetchers(&vault, true, "local-runner", None, Some(&fetchers)), 0);
        assert_eq!(crate::runs::Runs::new(&vault).read(None).len(), before);
        let _ = std::fs::remove_dir_all(&vault);
    }

    #[test]
    fn main_defaults_to_cli_via() {
        let vault = runnable_vault(
            "maincli",
            "timezone: America/Chicago\ncoursework:\n  zybooks:\n    enabled: true\n",
        );
        let ok = |_: &Mapping, _: &TimeZone, _: &mut Vec<String>| -> Result<Vec<Assignment>, SourceError> {
            Ok(vec![plain()])
        };
        let fetchers: [(&str, Fetcher); 1] = [("zybooks", &ok)];
        assert_eq!(
            main_with_fetchers(&vault, false, "cli", None, Some(&fetchers)),
            0
        );
        let records = journal_records(&vault);
        assert!(!records.is_empty());
        assert!(records.iter().all(|r| r["via"] == serde_json::json!("cli")));
        assert!(records.iter().all(|r| r["run_id"].is_null()));
        let _ = std::fs::remove_dir_all(&vault);
    }

    #[test]
    fn a_dry_run_appends_no_run_log_line() {
        // `if not args.dry_run:` guards the whole summary block.
        let vault = runnable_vault(
            "maindry",
            "timezone: America/Chicago\ncoursework:\n  zybooks:\n    enabled: true\n",
        );
        let ok = |_: &Mapping, _: &TimeZone, _: &mut Vec<String>| -> Result<Vec<Assignment>, SourceError> {
            Ok(vec![plain()])
        };
        let fetchers: [(&str, Fetcher); 1] = [("zybooks", &ok)];
        assert_eq!(
            main_with_fetchers(&vault, true, "cli", None, Some(&fetchers)),
            0
        );
        assert!(!vault.join("state").join("runner-log.md").exists());
        assert_eq!(std::fs::read_dir(vault.join("tasks")).unwrap().count(), 0);
        let _ = std::fs::remove_dir_all(&vault);
    }

    #[test]
    fn a_clean_run_logs_ok_with_the_created_and_updated_counts() {
        let vault = runnable_vault(
            "mainok",
            "timezone: America/Chicago\ncoursework:\n  zybooks:\n    enabled: true\n",
        );
        let ok = |_: &Mapping, _: &TimeZone, _: &mut Vec<String>| -> Result<Vec<Assignment>, SourceError> {
            Ok(vec![plain()])
        };
        let fetchers: [(&str, Fetcher); 1] = [("zybooks", &ok)];
        main_with_fetchers(&vault, false, "cli", None, Some(&fetchers));
        let log = run_log(&vault);
        assert!(
            log.contains("local ok coursework (1 assignments; 1 created, 0 updated)"),
            "{log}"
        );
        let _ = std::fs::remove_dir_all(&vault);
    }

    #[test]
    fn parse_duration_hours_and_minutes() {
        assert_eq!(parse_duration_hours("1h 22m"), Ok(1.37));
    }

    #[test]
    fn parse_duration_minutes_only() {
        assert_eq!(parse_duration_hours("27m"), Ok(0.45));
    }

    #[test]
    fn parse_duration_single_digit_minutes() {
        // VHL emits "3h 4m", not "3h 04m" — the regex must not assume two digits.
        assert_eq!(parse_duration_hours("3h 4m"), Ok(3.07));
    }

    #[test]
    fn parse_duration_hours_only() {
        assert_eq!(parse_duration_hours("2h"), Ok(2.0));
    }

    #[test]
    fn parse_duration_rejects_garbage() {
        // An unparseable estimate must fail, not silently become 0.0 — a task with 0.0 effort is
        // invisible to capacity fitting.
        assert!(parse_duration_hours("about an hour").is_err());
    }

    #[test]
    fn parse_duration_rejects_empty() {
        // The pattern matches the empty string; only the both-groups-absent check rejects it.
        assert!(parse_duration_hours("").is_err());
        assert!(parse_duration_hours("   ").is_err());
    }

    #[test]
    fn the_case_of_the_unit_letters_does_not_matter() {
        // re.IGNORECASE in the original.
        assert_eq!(parse_duration_hours("1H 30M"), Ok(1.5));
    }

    #[test]
    fn a_source_error_displays_as_its_message_alone() {
        // `collect` interpolates `{err}` into its warning lines, and Python's `str(exc)` is the
        // message the exception was constructed with — not a type name and not a Debug form.
        assert_eq!(
            SourceError::NotLoggedIn("session gone".to_string()).to_string(),
            "session gone"
        );
        assert_eq!(
            SourceError::Failed("transport blew up".to_string()).to_string(),
            "transport blew up"
        );
    }

    // --- discover_json (C1 H10) ----------------------------------------------------------------
    //
    // None of these touch the network or the real Credential Manager. With no vault and no target
    // flags, both sources are skipped before any fetch is attempted. The row-shaping tests drive
    // `zybooks_rows`/`vhl_rows` directly against a config loaded from a real temp vault by the
    // real `load_coursework_config`, exercising the same `coursework.<source>.*` key path
    // `discover_json` reads, with no fetch in the loop at all. The malformed-config test drives
    // `discover_json` itself, since what changes is the `errors` list a real config-load failure
    // produces, not anything a row shaper sees.

    fn coursework_block(config: &Mapping, source: &str) -> Mapping {
        crate::yaml::get(config, "coursework")
            .and_then(|v| v.as_mapping())
            .and_then(|block| crate::yaml::get(block, source))
            .and_then(|v| v.as_mapping().cloned())
            .unwrap_or_default()
    }

    #[test]
    fn discover_json_with_no_vault_and_no_targets_is_the_empty_shape() {
        let json = discover_json(None, None, None);
        assert_eq!(json, "{\"errors\": [], \"vhl\": [], \"zybooks\": []}");
    }

    #[test]
    fn zybooks_rows_routes_each_code_the_way_fetch_zybooks_does() {
        let vault = scratch("discover-zybooks");
        std::fs::create_dir_all(vault.join("config")).unwrap();
        write_note(
            &vault.join("config").join("ingest.yaml"),
            "coursework:\n  zybooks:\n    courses:\n      UACS100Fall2026:\n        course: cs-100\n        label: CS 100\n    ignore:\n      - HowToUseZyBooks2\n",
        );
        let (config, warnings) = load_coursework_config(&vault).unwrap();
        assert!(warnings.is_empty());
        let zycfg = coursework_block(&config, "zybooks");

        let codes = vec![
            "UACS100Fall2026".to_string(),
            "HowToUseZyBooks2".to_string(),
            "SomeNewCourse".to_string(),
        ];
        assert_eq!(
            zybooks_rows(&codes, &zycfg),
            vec![
                serde_json::json!({"code": "UACS100Fall2026", "mapped": true, "ignored": false}),
                serde_json::json!({"code": "HowToUseZyBooks2", "mapped": false, "ignored": true}),
                serde_json::json!({"code": "SomeNewCourse", "mapped": false, "ignored": false}),
            ]
        );
        let _ = std::fs::remove_dir_all(&vault);
    }

    #[test]
    fn vhl_rows_are_mapped_the_way_parse_dashboard_routes_a_section() {
        let vault = scratch("discover-vhl");
        std::fs::create_dir_all(vault.join("config")).unwrap();
        write_note(
            &vault.join("config").join("ingest.yaml"),
            "coursework:\n  vhl:\n    sections:\n      \"2102121\":\n        course: gn-103\n        label: GN 103 Hausaufgaben\n",
        );
        let (config, warnings) = load_coursework_config(&vault).unwrap();
        assert!(warnings.is_empty());
        let vhlcfg = coursework_block(&config, "vhl");

        let pairs = vec![
            ("1623220".to_string(), "2102121".to_string()),
            ("1623220".to_string(), "9999999".to_string()),
        ];
        assert_eq!(
            vhl_rows(&pairs, &vhlcfg),
            vec![
                serde_json::json!({"course_id": "1623220", "section": "2102121", "mapped": true}),
                serde_json::json!({"course_id": "1623220", "section": "9999999", "mapped": false}),
            ]
        );
        let _ = std::fs::remove_dir_all(&vault);
    }

    #[test]
    fn a_malformed_config_is_an_errors_entry_not_a_silent_empty_config() {
        // Before this fix, `.ok()` on `load_coursework_config` swallowed both the `Err` arm (a
        // non-UTF-8 config) and its warnings list (a malformed-YAML config), so a broken
        // `config/ingest.yaml` read as "no coursework block" instead of "this vault's config is
        // broken" — worst with a `--*-target` flag present, where the source was queried against
        // a silently emptied config rather than being skipped or reported (verified against the
        // built binary rather than here, since exercising it needs a real credential-target
        // lookup and this test suite does not touch the real Credential Manager).
        let vault = scratch("discover-badcfg");
        std::fs::create_dir_all(vault.join("config")).unwrap();
        write_note(&vault.join("config").join("ingest.yaml"), "]]]\n");
        let json = discover_json(Some(&vault), None, None);
        assert!(
            json.contains("\"errors\": [\"config: config unreadable"),
            "{json}"
        );
        assert!(json.contains("\"vhl\": []"), "{json}");
        assert!(json.contains("\"zybooks\": []"), "{json}");
        let _ = std::fs::remove_dir_all(&vault);
    }

    // --- fetch on device, think in the cloud (C2 Task 7) --------------------------------------
    //
    // C2 Task 7 fix 1 (R-C2-E17 / I1): `collect_cloud` itself is not exercised here — it reads a
    // real Credential Manager entry inline (`wincred::read_credential`) before it ever builds a
    // request, and this suite does not touch the real credential store (CLAUDE.md). What IS
    // exercised, end to end over a real loopback socket, is the seam `collect_cloud` hands off
    // to once the credentialed fetch is done: `coursework_request` (pure) and `post_coursework`
    // (the wire). That is where the credential-free guarantee and the reply-decoding paths
    // actually live, and it needs no credential of any kind to prove.

    use std::io::{BufRead, BufReader, Read as StdRead, Write as StdWrite};
    use std::net::TcpListener;

    /// A loopback server that answers `replies` in order and hands back everything it was sent.
    /// Lifted from `engine/tests/cloud_contract.rs`'s pattern; duplicated here (not shared) because
    /// `coursework_request`/`post_coursework` are `pub(crate)` and this module's own test
    /// submodule is the only place outside `coursework.rs` that can reach them.
    struct Loopback {
        base: String,
        handle: Option<std::thread::JoinHandle<Vec<String>>>,
    }

    impl Loopback {
        fn requests(&mut self) -> Vec<String> {
            self.handle.take().expect("joined once").join().expect("the listener thread did not panic")
        }
    }

    fn read_request(stream: &std::net::TcpStream) -> String {
        let mut reader = BufReader::new(stream.try_clone().expect("clone the accepted stream"));
        let mut head = String::new();
        let mut length = 0usize;
        loop {
            let mut line = String::new();
            if reader.read_line(&mut line).unwrap_or(0) == 0 {
                break;
            }
            if let Some(rest) = line.to_ascii_lowercase().strip_prefix("content-length:") {
                length = rest.trim().parse().unwrap_or(0);
            }
            let blank = line == "\r\n" || line == "\n";
            head.push_str(&line);
            if blank {
                break;
            }
        }
        let mut body = vec![0u8; length];
        if length > 0 {
            let _ = reader.read_exact(&mut body);
        }
        format!("{head}{}", String::from_utf8_lossy(&body))
    }

    fn loopback(replies: Vec<(u16, String)>) -> Loopback {
        let listener = TcpListener::bind("127.0.0.1:0").expect("bind the loopback listener");
        let port = listener.local_addr().expect("the listener has an address").port();
        let handle = std::thread::spawn(move || {
            let mut seen = Vec::new();
            for (code, body) in replies {
                let Ok((mut stream, _)) = listener.accept() else { break };
                seen.push(read_request(&stream));
                let response = format!(
                    "HTTP/1.1 {code} X\r\ncontent-type: application/json\r\ncontent-length: {}\r\nconnection: close\r\n\r\n{body}",
                    body.len()
                );
                let _ = stream.write_all(response.as_bytes());
                let _ = stream.flush();
            }
            seen
        });
        Loopback { base: format!("http://127.0.0.1:{port}/functions/v1"), handle: Some(handle) }
    }

    fn cloud_test_config(base: &str) -> crate::cloudmodel::CloudConfig {
        crate::cloudmodel::CloudConfig {
            api_base: base.to_string(),
            anon_key: "anon-not-a-secret".to_string(),
            session_credential_target: "knowlu/test-profile/session".to_string(),
            account_id: "acct-1".to_string(),
        }
    }

    /// D11 and the VISION amendment, at the boundary that now exists: the request `post_coursework`
    /// actually puts on the wire carries the vendor's own bytes and the parser's config, and **no
    /// credential of any kind** — not the credential target, not the portal password, not the
    /// account username, not a filesystem path pointing at either. The device signs in; the server
    /// never could. Rewritten end to end for C2 Task 7 fix 1 (R-C2-E17 / I1): the earlier version
    /// asserted over a hand-built JSON value and never touched `collect_cloud`'s own request
    /// builder or its transport.
    #[test]
    fn the_coursework_payload_carries_no_credential() {
        let cfg = crate::yaml::mapping_of(concat!(
            "enabled: true\n",
            "credential_target: knowlu/test-profile/zybooks\n",
            "base_url: https://www.vhlcentral.com\n",
            "username: user-not-a-secret\n",
            "password: pw-not-a-secret\n",
            "state_dir: C:/Users/test-profile/AppData/Local/knowlu/cache\n",
            "courses:\n  cs-100-2026:\n    course: cs-100\n    label: CS 100\n",
            "ignore:\n  - HowToUseZyBooks2\n",
            "categories:\n  HW: hw\n",
            "effort:\n  minutes_per_section: 6\n  floors:\n    hw: 0.25\n",
            "importance:\n  hw: 2\n",
        ));
        let redacted = yaml_to_json_for_request(&redact(&cfg, "zybooks"));
        let books = vec![(
            "cs-100-2026".to_string(),
            serde_json::json!({ "success": true, "assignments": [{ "assignment_id": 1839982 }] }),
        )];
        let request = coursework_request(
            "America/Chicago",
            &[FetchedSource::ZyBooks { config: redacted, books }],
        );

        let mut server = loopback(vec![(200, r#"{"assignments":[],"warnings":[]}"#.to_string())]);
        let client = crate::cloudmodel::CloudClient::new(&cloud_test_config(&server.base), "jwt-not-a-secret");
        let mut warnings: Vec<String> = Vec::new();
        let out = post_coursework(&client, &request, &mut warnings);
        assert!(out.assignments.is_empty());
        assert!(out.proposals.is_empty());
        assert!(warnings.is_empty(), "{warnings:?}");
        let sent = server.requests().remove(0);

        for forbidden in [
            "credential_target",
            "knowlu/test-profile",
            "base_url",
            "enabled",
            "pw-not-a-secret",
            "user-not-a-secret",
            "C:/Users/test-profile",
            "knowlu/",
        ] {
            assert!(!sent.contains(forbidden), "the request body carried {forbidden:?}: {sent}");
        }
        // And everything the parser genuinely needs — plus the vendor's own raw payload — did
        // travel.
        for needed in [
            "America/Chicago", "zybooks", "courses", "cs-100-2026", "categories",
            "minutes_per_section", "importance", "ignore", "assignment_id",
        ] {
            assert!(sent.contains(needed), "the request body lost {needed:?}: {sent}");
        }
    }

    /// R-C2-E17 / I1: a dead service must be exactly one named warning and nothing else — never a
    /// silently empty semester, and never a panic on a connection the OS refuses outright.
    #[test]
    fn a_dead_service_is_one_named_warning_and_nothing_changes() {
        let listener = TcpListener::bind("127.0.0.1:0").expect("bind");
        let port = listener.local_addr().expect("addr").port();
        drop(listener); // nothing is listening on that port now: the connection is refused
        let client = crate::cloudmodel::CloudClient::new(
            &cloud_test_config(&format!("http://127.0.0.1:{port}/functions/v1")),
            "jwt-not-a-secret",
        );
        let request = coursework_request("America/Chicago", &[]);
        let mut warnings: Vec<String> = Vec::new();
        let out = post_coursework(&client, &request, &mut warnings);
        assert!(out.assignments.is_empty());
        assert_eq!(warnings.len(), 1, "{warnings:?}");
        assert!(warnings[0].contains("the service is unavailable"), "{warnings:?}");
        assert!(warnings[0].ends_with("nothing changed"), "{warnings:?}");
    }

    /// R-C2-E17 / I1: `post_coursework` is where a reply row actually gets decoded off the wire —
    /// a good row and a row this engine cannot read must be told apart there, not only at
    /// `assignment_from_row`'s own unit-test level, and the malformed one is a warning, never a
    /// panic or a note written half-formed.
    #[test]
    fn post_coursework_keeps_the_good_row_and_warns_on_the_malformed_one() {
        let reply = serde_json::json!({
            "assignments": [
                {
                    "uid": "zybooks:1839992", "slug": "cs-100-hw-01", "title": "CS 100 HW 01",
                    "due": "2026-08-26T23:59", "course": "cs-100", "effort_hours": 2.5,
                    "effort_confidence": "low", "effort_source": "inferred", "importance": 2,
                    "importance_reason": "reason text", "progress": 0, "created_by": "zybooks",
                    "body": "body text",
                },
                { "uid": "zybooks:1", "title": "x", "due": "not a date" },
            ],
            "warnings": ["zybooks: zybook surprise-101 not in config; skipped"],
        })
        .to_string();
        let mut server = loopback(vec![(200, reply)]);
        let client = crate::cloudmodel::CloudClient::new(&cloud_test_config(&server.base), "jwt-not-a-secret");
        let request = coursework_request("America/Chicago", &[]);
        let mut warnings: Vec<String> = Vec::new();
        let out = post_coursework(&client, &request, &mut warnings);
        let _ = server.requests();
        assert_eq!(out.assignments.len(), 1, "{out:?}");
        assert_eq!(out.assignments[0].uid, "zybooks:1839992");
        assert!(out.proposals.is_empty(), "{out:?}");
        assert!(
            warnings.contains(&"zybooks: zybook surprise-101 not in config; skipped".to_string()),
            "{warnings:?}"
        );
        assert!(
            warnings.contains(&"coursework: a reply row could not be read; skipped".to_string()),
            "{warnings:?}"
        );
    }

    /// I3 / R-C2-E17: `collect_cloud` resolves the timezone through the same [`resolve_timezone`]
    /// `collect` uses for the local path — one warning and a fallback, never a silently dropped
    /// source. An unknown IANA name reaching `dueLocal` server-side unstamped would otherwise turn
    /// every item into "unreadable due date … skipped" and the whole source into "0 assignments
    /// parsed; treating as failure" — a full source lost to one bad word in a YAML file.
    #[test]
    fn a_bad_timezone_still_resolves_locally_and_the_request_carries_the_fallback() {
        let config = crate::yaml::mapping_of("timezone: Nowhere/Fake\n");
        let mut warnings: Vec<String> = Vec::new();
        let tz_name = resolve_timezone(&config, &mut warnings)
            .iana_name()
            .unwrap_or(DEFAULT_TZ)
            .to_string();
        assert_eq!(warnings.len(), 1, "{warnings:?}");
        assert!(warnings[0].contains("bad timezone"), "{warnings:?}");
        assert!(warnings[0].contains(&format!("using {DEFAULT_TZ}")), "{warnings:?}");
        let request = coursework_request(&tz_name, &[]);
        assert_eq!(request["timezone"], serde_json::json!(DEFAULT_TZ));
    }

    /// The reconciled list the service returns is written by the same `sync_coursework` the local
    /// parse fed, so a cloud run and a local run produce the same notes.
    #[test]
    fn a_reply_row_becomes_the_same_assignment_the_local_parser_produced() {
        let row = serde_json::json!({
            "uid": "zybooks:1839992", "slug": "cs-100-hw-01", "title": "CS 100 HW 01",
            "due": "2026-08-26T23:59", "course": "cs-100", "effort_hours": 2.5,
            "effort_confidence": "low", "effort_source": "inferred", "importance": 2,
            "importance_reason": "zyBooks hw worth 193 points across 25 sections; per-category importance from config",
            "progress": 0, "created_by": "zybooks", "body": "25 zyBooks section(s), 193 points.\n\n- 1.1 x"
        });
        let got = assignment_from_row(&row).expect("a well-formed row parses");
        assert_eq!(got.uid, "zybooks:1839992");
        assert_eq!(got.due, jiff::civil::date(2026, 8, 26).at(23, 59, 0, 0));
        assert_eq!(got.effort_hours, 2.5);
        assert_eq!(got.course.as_deref(), Some("cs-100"));
    }

    /// A row the service sent that the engine cannot read is dropped with a warning, never
    /// written half-formed: a note with no due date ranks as though it had none.
    #[test]
    fn a_malformed_reply_row_is_a_warning_and_not_a_note() {
        let row = serde_json::json!({ "uid": "zybooks:1", "title": "x", "due": "not a date" });
        assert!(assignment_from_row(&row).is_none());
    }

    // --- R-OB-1: an unmapped book or section is a card in the deck (C2 Task 7a) ----------------

    /// R-OB-1: the reply's proposals become cards in the deck, one per unmapped source.
    #[test]
    fn an_unmapped_book_becomes_a_card_the_deck_can_answer() {
        let _guard = crate::journal::DEVICE_ENV_MUTEX.lock().unwrap();
        let vault = scratch_vault("map-card");
        let ctx = WriteContext::new(MAP_ACTOR, "local-runner");
        let mut journal = Journal::new(&vault);
        let path = write_map_card(
            &vault,
            &MapProposal {
                source: "zybooks".into(),
                key: "UACS100Fall2026".into(),
                label: "UACS100Fall2026".into(),
                suggested_course: Some("cs-100".into()),
            },
            jiff::civil::date(2026, 9, 9),
            &ctx,
            &mut journal,
        )
        .expect("the card writes");
        let card = std::fs::read_to_string(&path).unwrap();
        assert!(card.contains("kind: coursework-map"));
        // R-C2-E18: quoted through `lit()`, the crate's one escaping barrier — a key off the wire
        // could carry `: `, a leading `*`/`&`/`!`, or `---`, and unquoted would either fail
        // `create`'s frontmatter check or inject lines.
        assert!(card.contains("source: \"zybooks\""));
        assert!(card.contains("map_key: \"UACS100Fall2026\""));
        assert!(card.contains("course: \"cs-100\""));
        assert!(card.contains("status: pending"));
        let _ = std::fs::remove_dir_all(&vault);
    }

    /// A card is never minted twice for one key — the next slot fetches the same unmapped book.
    #[test]
    fn a_map_card_is_never_minted_twice_for_one_key() {
        let _guard = crate::journal::DEVICE_ENV_MUTEX.lock().unwrap();
        let vault = scratch_vault("map-once");
        let ctx = WriteContext::new(MAP_ACTOR, "local-runner");
        let mut journal = Journal::new(&vault);
        let p = MapProposal {
            source: "vhl".into(),
            key: "2102121".into(),
            label: "VHL section 2102121".into(),
            suggested_course: None,
        };
        let today = jiff::civil::date(2026, 9, 9);
        assert!(write_map_card(&vault, &p, today, &ctx, &mut journal).is_ok());
        assert!(write_map_card(&vault, &p, today, &ctx, &mut journal).is_err());
        assert_eq!(crate::approvals::sorted_md(&vault.join("approvals")).len(), 1);
        let _ = std::fs::remove_dir_all(&vault);
    }

    /// A card the student REJECTED is not re-asked on the next slot — `write::create`'s `Exists`
    /// guard only sees `approvals/`, and a rejected card lives in `archive/`.
    #[test]
    fn a_rejected_map_card_is_not_re_asked_until_it_expires() {
        let _guard = crate::journal::DEVICE_ENV_MUTEX.lock().unwrap();
        let vault = scratch_vault("map-rejected");
        let ctx = WriteContext::new(MAP_ACTOR, "local-runner");
        let mut journal = Journal::new(&vault);
        let today = jiff::civil::date(2026, 9, 9);
        let p = MapProposal {
            source: "zybooks".into(),
            key: "UACS100Fall2026".into(),
            label: "UACS100Fall2026".into(),
            suggested_course: Some("cs-100".into()),
        };
        let path = write_map_card(&vault, &p, today, &ctx, &mut journal).unwrap();
        let rel = crate::ids::rel(&vault, &path);

        // The student says no, and `process_approvals` settles the card into `archive/`.
        crate::write::write_literals(
            &vault,
            &rel,
            &[("status".to_string(), "rejected".to_string())],
            &ctx,
            &mut journal,
            &crate::write::WriteOpts::default(),
        )
        .unwrap();
        crate::write::delete(&vault, &rel, &ctx, &mut journal).unwrap();
        assert!(crate::approvals::sorted_md(&vault.join("approvals")).is_empty());

        // The next slot fetches the same unmapped book and asks nothing.
        let key = ("zybooks".to_string(), "UACS100Fall2026".to_string());
        assert!(asked_map_keys(&vault, today).contains(&key), "the archived card is the memory");
        // …until it expires, 30 days out, when the question is worth asking again.
        assert!(
            asked_map_keys(&vault, jiff::civil::date(2026, 10, 10)).is_empty(),
            "asked again next month, not twice a day forever"
        );
        let _ = std::fs::remove_dir_all(&vault);
    }

    /// Approving one writes the mapping into `config/ingest.yaml` and archives the card.
    #[test]
    fn an_approved_map_card_writes_the_mapping_and_is_archived() {
        let _guard = crate::journal::DEVICE_ENV_MUTEX.lock().unwrap();
        let vault = scratch_vault("map-apply");
        std::fs::write(
            vault.join("config").join("ingest.yaml"),
            "timezone: America/Chicago\ncoursework:\n  zybooks:\n    enabled: true\n    courses: {}\n",
        )
        .unwrap();
        let ctx = WriteContext::new(MAP_ACTOR, "local-runner");
        let mut journal = Journal::new(&vault);
        let path = write_map_card(
            &vault,
            &MapProposal {
                source: "zybooks".into(),
                key: "UACS100Fall2026".into(),
                label: "UACS100Fall2026".into(),
                suggested_course: Some("cs-100".into()),
            },
            jiff::civil::date(2026, 9, 9),
            &ctx,
            &mut journal,
        )
        .unwrap();
        let rel = crate::ids::rel(&vault, &path);
        crate::write::write_literals(
            &vault,
            &rel,
            &[("status".to_string(), "approved".to_string())],
            &ctx,
            &mut journal,
            &crate::write::WriteOpts::default(),
        )
        .unwrap();

        let lines = apply_map_cards(&vault, &ctx, &mut journal, false);
        assert!(lines.iter().any(|l| l.contains("mapped UACS100Fall2026")), "{lines:?}");
        let cfg = crate::pystr::read_text(&vault.join("config").join("ingest.yaml")).unwrap();
        assert!(cfg.contains("\"UACS100Fall2026\":"), "{cfg}");
        assert!(cfg.contains("course: \"cs-100\""), "{cfg}");
        assert!(!path.exists());
        let _ = std::fs::remove_dir_all(&vault);
    }

    /// R-C2-8: the insertion goes directly beneath an existing `courses:` line, at the
    /// indentation the file already uses, and the entry that was there is left exactly as it was.
    #[test]
    fn a_mapping_is_inserted_under_an_existing_block() {
        let vault = scratch_vault("map-insert");
        let path = vault.join("config").join("ingest.yaml");
        crate::pystr::write_text(
            &path,
            "timezone: America/Chicago\ncoursework:\n  zybooks:\n    enabled: true\n    courses:\n\
             \u{20}     'UAMATH120Fall2026':\n        course: 'math-120'\n        label: 'MATH 120'\n",
        )
        .unwrap();

        assert_eq!(write_mapping(&vault, "zybooks", "UACS100Fall2026", "cs-100"), Ok(true));

        assert_eq!(
            crate::pystr::read_text(&path).unwrap(),
            "timezone: America/Chicago\ncoursework:\n  zybooks:\n    enabled: true\n    courses:\n\
             \u{20}     \"UACS100Fall2026\":\n        course: \"cs-100\"\n        label: \"CS 100\"\n\
             \u{20}     'UAMATH120Fall2026':\n        course: 'math-120'\n        label: 'MATH 120'\n"
        );
        let _ = std::fs::remove_dir_all(&vault);
    }

    /// R-C2-8: `courses: {}` — what C1's `scaffold::ingest_yaml` writes at birth — becomes the
    /// block form, one line replaced in place at its own indentation. The VHL half is `sections`.
    #[test]
    fn an_empty_flow_mapping_becomes_the_block_form() {
        let vault = scratch_vault("map-flow");
        let path = vault.join("config").join("ingest.yaml");
        crate::pystr::write_text(
            &path,
            "timezone: America/Chicago\ncoursework:\n  zybooks:\n    enabled: true\n    courses: {}\n\
             \u{20} vhl:\n    enabled: true\n    sections: {}\n",
        )
        .unwrap();

        assert_eq!(write_mapping(&vault, "zybooks", "UACS100Fall2026", "cs-100"), Ok(true));
        assert_eq!(write_mapping(&vault, "vhl", "2102121", "gn-103"), Ok(true));

        assert_eq!(
            crate::pystr::read_text(&path).unwrap(),
            "timezone: America/Chicago\ncoursework:\n  zybooks:\n    enabled: true\n    courses:\n\
             \u{20}     \"UACS100Fall2026\":\n        course: \"cs-100\"\n        label: \"CS 100\"\n\
             \u{20} vhl:\n    enabled: true\n    sections:\n\
             \u{20}     \"2102121\":\n        course: \"gn-103\"\n        label: \"GN 103\"\n"
        );
        // The section id survives as a STRING key, which is the whole reason it is quoted:
        // `route_section` looks it up by the text the dashboard gave it.
        let cfg: Yaml = serde_yaml_ng::from_str(&crate::pystr::read_text(&path).unwrap()).unwrap();
        assert!(cfg["coursework"]["vhl"]["sections"]["2102121"]["course"].as_str() == Some("gn-103"));
        let _ = std::fs::remove_dir_all(&vault);
    }

    /// R-C2-8: a second apply of the same key is a no-op with a log line — not a duplicate key,
    /// which would be a YAML load error and would take the whole config down with it.
    #[test]
    fn a_second_apply_of_the_same_key_is_a_no_op() {
        let _guard = crate::journal::DEVICE_ENV_MUTEX.lock().unwrap();
        let vault = scratch_vault("map-again");
        let path = vault.join("config").join("ingest.yaml");
        crate::pystr::write_text(
            &path,
            "timezone: America/Chicago\ncoursework:\n  zybooks:\n    enabled: true\n    courses: {}\n",
        )
        .unwrap();

        assert_eq!(write_mapping(&vault, "zybooks", "UACS100Fall2026", "cs-100"), Ok(true));
        let once = std::fs::read(&path).unwrap();
        assert_eq!(write_mapping(&vault, "zybooks", "UACS100Fall2026", "cs-100"), Ok(false));
        assert_eq!(std::fs::read(&path).unwrap(), once, "the file is not even opened for writing");

        // And the card that asks for it a second time is still executed and archived, with the
        // line that says why nothing changed.
        let ctx = WriteContext::new(MAP_ACTOR, "local-runner");
        let mut journal = Journal::new(&vault);
        let card_path = write_map_card(
            &vault,
            &MapProposal {
                source: "zybooks".into(),
                key: "UACS100Fall2026".into(),
                label: "UACS100Fall2026".into(),
                suggested_course: Some("cs-100".into()),
            },
            jiff::civil::date(2026, 9, 9),
            &ctx,
            &mut journal,
        )
        .unwrap();
        let rel = crate::ids::rel(&vault, &card_path);
        crate::write::write_literals(
            &vault,
            &rel,
            &[("status".to_string(), "approved".to_string())],
            &ctx,
            &mut journal,
            &crate::write::WriteOpts::default(),
        )
        .unwrap();
        let lines = apply_map_cards(&vault, &ctx, &mut journal, false);
        assert!(lines.iter().any(|l| l.contains("was already mapped")), "{lines:?}");
        assert_eq!(std::fs::read(&path).unwrap(), once, "still byte-identical");
        assert!(!card_path.exists());
        let _ = std::fs::remove_dir_all(&vault);
    }

    /// R-C2-8, the invariant itself: **every byte outside the inserted lines is unchanged** —
    /// comments, blank lines, key order, the single-quoted style `scaffold::ingest_yaml` uses, and
    /// the file's own line terminator. This is the test a `serde_yaml_ng::to_string` round trip
    /// cannot pass, and it is why the ruling exists.
    #[test]
    fn every_byte_outside_the_inserted_lines_is_unchanged() {
        let vault = scratch_vault("map-bytes");
        let path = vault.join("config").join("ingest.yaml");
        let original = [
            "ics_url: 'https://lms.example.invalid/feed/a.ics'",
            "timezone: America/Chicago",
            "course_map: {}",
            "calendars: []",
            "",
            "# Passwords are NOT here. They live in Windows Credential Manager under the",
            "# credential_target names below.",
            "coursework:",
            "  zybooks:",
            "    enabled: true",
            "    credential_target: 'knowlu/p1/zybooks'",
            "    ignore:",
            "      - 'HowToUseZyBooks2'",
            "    courses:",
            "      'UAMATH120Fall2026':",
            "        course: 'math-120'",
            "        label: 'MATH 120'   # hand-edited, and it stays hand-edited",
            "  vhl:",
            "    enabled: true",
            "    sections: {}",
            "",
        ]
        .join("\n");
        // Written through `write_text`, so the fixture is in THIS platform's own terminator and the
        // comparison below is a real byte comparison rather than a line-ending artefact.
        crate::pystr::write_text(&path, &original).unwrap();
        let before = String::from_utf8(std::fs::read(&path).unwrap()).unwrap();

        assert_eq!(write_mapping(&vault, "zybooks", "UACS100Fall2026", "cs-100"), Ok(true));
        let after = String::from_utf8(std::fs::read(&path).unwrap()).unwrap();

        let nl = crate::pystr::NEWLINE;
        let inserted = format!(
            "      \"UACS100Fall2026\":{nl}        course: \"cs-100\"{nl}        label: \"CS 100\"{nl}"
        );
        assert!(after.contains(&format!("    courses:{nl}{inserted}      'UAMATH120Fall2026':")));
        assert_eq!(after.replacen(&inserted, "", 1), before, "three lines added and nothing else");
        let _ = std::fs::remove_dir_all(&vault);
    }

    /// A card with no course on it is refused rather than writing an empty mapping — an empty
    /// `course` would make every item from that book `course: ""`, which reads as attributed.
    #[test]
    fn a_map_card_with_no_course_is_refused_and_kept() {
        let _guard = crate::journal::DEVICE_ENV_MUTEX.lock().unwrap();
        let vault = scratch_vault("map-blank");
        std::fs::write(
            vault.join("config").join("ingest.yaml"),
            "timezone: America/Chicago\ncoursework:\n  vhl:\n    enabled: true\n    sections: {}\n",
        )
        .unwrap();
        let ctx = WriteContext::new(MAP_ACTOR, "local-runner");
        let mut journal = Journal::new(&vault);
        let path = write_map_card(
            &vault,
            &MapProposal {
                source: "vhl".into(),
                key: "2102121".into(),
                label: "VHL section 2102121".into(),
                suggested_course: None,
            },
            jiff::civil::date(2026, 9, 9),
            &ctx,
            &mut journal,
        )
        .unwrap();
        let rel = crate::ids::rel(&vault, &path);
        crate::write::write_literals(
            &vault,
            &rel,
            &[("status".to_string(), "approved".to_string())],
            &ctx,
            &mut journal,
            &crate::write::WriteOpts::default(),
        )
        .unwrap();
        let lines = apply_map_cards(&vault, &ctx, &mut journal, false);
        assert!(lines.iter().any(|l| l.contains("no course on the card")), "{lines:?}");
        let card = std::fs::read_to_string(&path).expect("the card survives to be edited");
        assert!(card.contains("status: pending"), "back to pending, so it ages and expires: {card}");
        let _ = std::fs::remove_dir_all(&vault);
    }

    /// R-C2-E18 fix 1: a dry run over a vault holding one approved card (`apply_map_cards`'s half)
    /// and one incoming proposal for a still-unmapped book (`propose_map_cards`'s half, which is
    /// what `collect_cloud` calls after the fetch) writes nothing at all — `config/ingest.yaml`,
    /// `approvals/`, `archive/` and `state/journal/` are byte-identical before and after, the same
    /// promise `dry_run_writes_nothing` and `dry_run_writes_no_journal` already pin for the sync
    /// half of this file.
    #[test]
    fn a_dry_run_leaves_config_approvals_archive_and_the_journal_byte_identical() {
        let _guard = crate::journal::DEVICE_ENV_MUTEX.lock().unwrap();
        let vault = scratch_vault("map-dryrun-both");
        std::fs::write(
            vault.join("config").join("ingest.yaml"),
            "timezone: America/Chicago\ncoursework:\n  zybooks:\n    enabled: true\n    courses: {}\n",
        )
        .unwrap();
        let ctx = WriteContext::new(MAP_ACTOR, "local-runner");
        let today = jiff::civil::date(2026, 9, 9);

        // One card already approved and sitting in `approvals/` — set up for real, before the
        // snapshot, so only the DRY RUN below is under test.
        let mut setup_journal = Journal::new(&vault);
        let approved = MapProposal {
            source: "zybooks".into(),
            key: "UACS100Fall2026".into(),
            label: "UACS100Fall2026".into(),
            suggested_course: Some("cs-100".into()),
        };
        let path = write_map_card(&vault, &approved, today, &ctx, &mut setup_journal).unwrap();
        let rel = crate::ids::rel(&vault, &path);
        crate::write::write_literals(
            &vault,
            &rel,
            &[("status".to_string(), "approved".to_string())],
            &ctx,
            &mut setup_journal,
            &crate::write::WriteOpts::default(),
        )
        .unwrap();

        let before = snapshot(&vault);

        // The approved-card half of the guard.
        let mut dry_journal = Journal::new(&vault);
        let apply_lines = apply_map_cards(&vault, &ctx, &mut dry_journal, true);
        assert!(apply_lines.iter().any(|l| l.contains("dry run")), "{apply_lines:?}");

        // The incoming-proposal half — a DIFFERENT, still-unmapped book, exactly what
        // `collect_cloud` hands `propose_map_cards` after a reply carries a proposal.
        let incoming = MapProposal {
            source: "vhl".into(),
            key: "2102121".into(),
            label: "VHL section 2102121".into(),
            suggested_course: None,
        };
        let mut warnings = Vec::new();
        propose_map_cards(&vault, std::slice::from_ref(&incoming), today, &ctx, true, &mut warnings);
        assert!(warnings.iter().any(|w| w.contains("dry run")), "{warnings:?}");

        let after = snapshot(&vault);
        assert_eq!(
            before, after,
            "a dry run must not write config/ingest.yaml, approvals/, archive/, or the journal"
        );
        let _ = std::fs::remove_dir_all(&vault);
    }

    // --- completion detection, tier 1 (stream J T8) -------------------------------------------

    fn approvals_in(vault: &Path) -> Vec<String> {
        crate::approvals::sorted_md(&vault.join("approvals"))
            .iter()
            .map(|p| p.file_name().unwrap().to_string_lossy().to_string())
            .collect()
    }

    /// A fetcher standing in for `fetch_zybooks`: returns `plain()` and reports its figure into
    /// the sink, exactly as the real one reports what the payload it just fetched says.
    fn run_with_figure(vault: &Path, dry_run: bool, earned: f64) -> i32 {
        let sink: CompletionSink = std::cell::RefCell::new(Vec::new());
        let zy = |_: &Mapping, _: &TimeZone, _: &mut Vec<String>| -> Result<Vec<Assignment>, SourceError> {
            sink.borrow_mut().push(crate::completion::VendorCompletion {
                source: "zybooks".to_string(),
                uid: "zybooks:1".to_string(),
                earned,
                possible: 193.0,
            });
            Ok(vec![plain()])
        };
        let fetchers: [(&str, Fetcher); 1] = [("zybooks", &zy)];
        main_with_sources(vault, dry_run, "local-runner", None, Some(&fetchers), Some(&sink))
    }

    #[test]
    fn a_finished_assignment_is_proposed_done_once_across_runs() {
        let vault = runnable_vault(
            "done-once",
            "timezone: America/Chicago\ncoursework:\n  zybooks:\n    enabled: true\n",
        );
        assert_eq!(run_with_figure(&vault, false, 193.0), 0);
        assert_eq!(approvals_in(&vault), vec!["amend-cs-100-hw-01-done.md".to_string()]);
        assert_eq!(run_with_figure(&vault, false, 193.0), 0);
        assert_eq!(approvals_in(&vault).len(), 1, "a second run filed a second card");
        // `progress` is still never written after creation; the proposal is the only output.
        let note = pystr::read_text(&vault.join("tasks").join("cs-100-hw-01.md")).unwrap();
        assert!(note.contains("\nprogress: 0\n") && note.contains("\nstatus: active\n"), "{note}");
        let _ = std::fs::remove_dir_all(&vault);
    }

    #[test]
    fn a_partial_assignment_is_never_proposed_and_a_dry_run_files_nothing() {
        let vault = runnable_vault(
            "done-partial",
            "timezone: America/Chicago\ncoursework:\n  zybooks:\n    enabled: true\n",
        );
        assert_eq!(run_with_figure(&vault, false, 168.0), 0); // 87%
        assert!(approvals_in(&vault).is_empty());
        let before = snapshot(&vault);
        assert_eq!(run_with_figure(&vault, true, 193.0), 0);
        assert_eq!(snapshot(&vault), before, "a dry run wrote something");
        let _ = std::fs::remove_dir_all(&vault);
    }

    #[test]
    fn the_cloud_path_reads_completion_from_the_raw_payloads_it_posts() {
        // Cloud vaults parse on the server; the device still holds the vendor bytes it posted, and
        // reads completion from those — no cloud change, and the same uids the server's parsers use.
        let zy: serde_json::Value = serde_json::from_str(
            &std::fs::read_to_string("tests/fixtures/zybooks-assignments-scored.json").unwrap(),
        )
        .unwrap();
        let html = "<div class=\"js-student-dashboard-app\" data-assignment-summaries=\"[{&quot;due_date&quot;: &quot;2026-09-21&quot;, &quot;detail_url&quot;: &quot;/courses/1/sections/2000001/x&quot;, &quot;percentage_complete&quot;: 100}]\"></div>";
        let sources = vec![
            FetchedSource::ZyBooks {
                config: serde_json::json!({}),
                books: vec![("FAB100".to_string(), zy)],
            },
            FetchedSource::Vhl { config: serde_json::json!({}), html: html.to_string() },
        ];
        let complete: Vec<String> = completions_from_sources(&sources)
            .into_iter()
            .filter(|c| c.is_complete())
            .map(|c| c.uid)
            .collect();
        assert_eq!(
            complete,
            vec!["zybooks:9100001", "zybooks:9100005", "vhl:2000001:2026-09-21"]
        );
    }
}
