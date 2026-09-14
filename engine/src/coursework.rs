//! Coursework sync — the types both source parsers produce, and (from Task 12) the sync that
//! consumes them. Port of `engine/coursework.py`.
//!
//! Wave 6 of the Rust port. This file currently holds only the two types that `zybooks` and `vhl`
//! both need; `sync_coursework`, `collect` and the CLI entry point land with Task 12. That split
//! mirrors the Python module graph rather than working around it: `coursework <-> zybooks` and
//! `coursework <-> vhl` are two of the four import cycles the crate is one crate in order to keep
//! (see `Cargo.toml`).

use std::fmt;
use std::path::Path;
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

#[cfg(windows)]
pub fn fetch_zybooks(
    cfg: &Mapping,
    tz: &TimeZone,
    warnings: &mut Vec<String>,
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

#[cfg(windows)]
pub fn fetch_vhl(
    cfg: &Mapping,
    tz: &TimeZone,
    warnings: &mut Vec<String>,
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
    crate::vhl::parse_dashboard(&html, cfg, tz, warnings)
}

/// The credential store is Windows-only (spec §6.5), so the live fetchers are too. A cloud build
/// still compiles; it simply has no way to authenticate, and says so rather than pretending the
/// semester is empty.
#[cfg(not(windows))]
pub fn fetch_zybooks(
    _cfg: &Mapping,
    _tz: &TimeZone,
    _warnings: &mut Vec<String>,
) -> Result<Vec<Assignment>, SourceError> {
    Err(SourceError::Failed(
        "credential store unavailable on this platform".to_string(),
    ))
}

#[cfg(not(windows))]
pub fn fetch_vhl(
    _cfg: &Mapping,
    _tz: &TimeZone,
    _warnings: &mut Vec<String>,
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

/// *Fetch on device, think in the cloud* (D11, §4.3). Every credentialed request is made here with
/// the student's own password out of Credential Manager; the raw payload then goes to
/// `/ingest-coursework` and the reconciled list comes back.
///
/// One source's failure never stops another, and the server applies the same
/// *an-empty-parse-is-a-failure* rule `collect` applies locally.
#[cfg(windows)]
pub fn collect_cloud(
    config: &Mapping,
    warnings: &mut Vec<String>,
    client: &crate::cloudmodel::CloudClient,
) -> Vec<Assignment> {
    let block = match crate::yaml::get(config, "coursework") {
        Some(Yaml::Mapping(map)) => map.clone(),
        _ => Mapping::new(),
    };
    let tz_name = match crate::yaml::get(config, "timezone") {
        Some(value) => crate::yaml::text(value).unwrap_or_else(|| DEFAULT_TZ.to_string()),
        None => DEFAULT_TZ.to_string(),
    };
    let mut sources: Vec<serde_json::Value> = Vec::new();

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
                Ok(books) => sources.push(serde_json::json!({
                    "name": "zybooks", "config": redacted,
                    "books": books.into_iter().map(|(code, payload)| serde_json::json!({"code": code, "payload": payload})).collect::<Vec<_>>(),
                })),
                Err(SourceError::NotLoggedIn(m)) => warnings.push(format!("zybooks: session invalid ({m}); nothing changed")),
                Err(SourceError::Failed(m)) => warnings.push(format!("zybooks: fetch failed ({m}); nothing changed")),
            },
            _ => {
                let base = cfg_str(&cfg, "base_url", "https://www.vhlcentral.com");
                match crate::vhl::login_and_fetch_dashboard(&credential.username, credential.password.expose(), &base, None) {
                    Ok(html) => sources.push(serde_json::json!({"name": "vhl", "config": redacted, "html": html})),
                    Err(SourceError::NotLoggedIn(m)) => warnings.push(format!("vhl: session invalid ({m}); nothing changed")),
                    Err(SourceError::Failed(m)) => warnings.push(format!("vhl: fetch failed ({m}); nothing changed")),
                }
            }
        }
    }

    if sources.is_empty() {
        return Vec::new();
    }
    let body = serde_json::json!({ "timezone": tz_name, "sources": sources });
    let reply = match client.post("/ingest-coursework", &body) {
        Ok(reply) => reply,
        Err(err) => {
            warnings.push(format!("coursework: the service is unavailable ({err}); nothing changed"));
            return Vec::new();
        }
    };
    for warning in reply.get("warnings").and_then(serde_json::Value::as_array).into_iter().flatten() {
        if let Some(text) = warning.as_str() {
            warnings.push(text.to_string());
        }
    }
    let mut out = Vec::new();
    for row in reply.get("assignments").and_then(serde_json::Value::as_array).into_iter().flatten() {
        match assignment_from_row(row) {
            Some(item) => out.push(item),
            None => warnings.push("coursework: a reply row could not be read; skipped".to_string()),
        }
    }
    out
}

/// The credential store is Windows-only (spec §6.5), so the credentialed fetch is too. A cloud
/// build still compiles; it simply has no way to authenticate, and says so rather than pretending
/// the semester is empty — the same shape `fetch_zybooks` and `fetch_vhl` already have.
#[cfg(not(windows))]
pub fn collect_cloud(
    _config: &Mapping,
    warnings: &mut Vec<String>,
    _client: &crate::cloudmodel::CloudClient,
) -> Vec<Assignment> {
    warnings.push("coursework: credential store unavailable on this platform".to_string());
    Vec::new()
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
const FAILURE_MARKERS: [&str; 5] = [
    "session invalid",
    "fetch failed",
    "0 assignments parsed",
    "config unreadable",
    "coursework pass failed",
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
    let outcome = (|| -> Result<(), SourceError> {
        let (config, config_warnings) = load_coursework_config(vault)?;
        warnings.extend(config_warnings);
        assignments = match (fetchers, crate::cloudmodel::resolve(vault).ok()) {
            // `fetchers` is the test seam and always wins; a vault with an account parses on the
            // server (§4.3); everything else is plan-3a's local path, unchanged until C4.
            (None, Some(client)) => collect_cloud(&config, &mut warnings, &client),
            _ => collect(vault, &config, &mut warnings, fetchers),
        };
        if !assignments.is_empty() {
            log = sync_coursework(&assignments, vault, None, dry_run, Some(&ctx), None)?;
        }
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
        assert_eq!(FAILURE_MARKERS.len(), 5);
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

    /// D11 and the VISION amendment, at the boundary that now exists: the payload the device posts
    /// carries the vendor's own bytes and the parser's config, and **no credential of any kind**.
    /// The device signs in; the server never could.
    #[test]
    fn the_coursework_payload_carries_no_credential() {
        let cfg = crate::yaml::mapping_of(concat!(
            "enabled: true\n",
            "credential_target: knowlu/test-profile/zybooks\n",
            "base_url: https://www.vhlcentral.com\n",
            "courses:\n  cs-100-2026:\n    course: cs-100\n    label: CS 100\n",
            "ignore:\n  - HowToUseZyBooks2\n",
            "categories:\n  HW: hw\n",
            "effort:\n  minutes_per_section: 6\n  floors:\n    hw: 0.25\n",
            "importance:\n  hw: 2\n",
        ));
        let redacted = redact(&cfg, "zybooks");
        let rendered = crate::ledger::dumps_value(&serde_json::json!({
            "timezone": "America/Chicago",
            "sources": [{ "name": "zybooks", "config": yaml_to_json_for_request(&redacted), "books": [] }],
        }));
        for forbidden in ["credential_target", "knowlu/test-profile", "base_url", "enabled", "password"] {
            assert!(!rendered.contains(forbidden), "the request body carried {forbidden:?}: {rendered}");
        }
        // And everything the parser genuinely needs did travel.
        for needed in ["courses", "cs-100-2026", "categories", "minutes_per_section", "importance", "ignore"] {
            assert!(rendered.contains(needed), "the request body lost {needed:?}: {rendered}");
        }
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
}
