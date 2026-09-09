//! Run records and the rendered `runner-log.md` line. Port of `engine/runs.py`.
//!
//! Wave 7, plan Task 14. [`log_line`] landed a day early because `coursework::main` cannot append
//! its run-log line without it — the chain is
//! `coursework::main -> cli::append_run_log -> runs::log_line`.
//!
//! # Two rules here that look like implementation detail and are not
//!
//! - **[`SEQ`] is process-global, not vault-global**, and resets on every process start. The live
//!   records prove that is the behaviour rather than an accident. Deriving it from the ledger
//!   would produce values no existing record has.
//! - **A `start` without an `end` past grace is *crashed*; no `start` past grace is *missing*;
//!   inside grace is *late* — and a late run must not be acted on.** That is the 2026-08-26
//!   lesson, and [`expected_status`] is where it lives.
//!
//! # F11: the log line is rendered FROM the run record, by exactly one code path
//!
//! That path is [`log_line`], and it is also the only place that flattens a multi-line summary.
//! A WARN carrying an embedded newline — an events-pass traceback, say — would otherwise write two
//! physical lines into `runner-log.md`, and the second of them is not a log line at all:
//! `cli::line_status` would classify it by whatever its fifth space-separated token happened to be,
//! and the retention rule would then keep or drop it for the wrong reason.

use std::io::Read;
use std::path::Path;
use std::process::{Command, Stdio};
use std::sync::atomic::{AtomicI64, Ordering};
use std::time::{Duration, Instant};

use jiff::civil::DateTime;
use jiff::tz::TimeZone;
use jiff::{SignedDuration, Timestamp};
use serde_json::Value;

use crate::pystr;

use crate::journal::{device_name, now_ts};
use crate::ledger::{JsonlLedger, LedgerError, Record};

/// Python's `_SEQ = itertools.count(1)` — **process-global, not vault-global**, and a *different*
/// counter from `journal`'s.
///
/// It resets on every process start, and the live records prove that is the real behaviour rather
/// than an accident: the local runner's `end` record carries `"seq": 2` (one process wrote both
/// `start` and `end`) while the cloud runner's carries `"seq": 1` (separate invocations, each
/// restarting the count). Deriving it from the ledger instead would produce ever-growing values
/// that no existing record has.
static SEQ: AtomicI64 = AtomicI64::new(1);

fn next_seq() -> i64 {
    SEQ.fetch_add(1, Ordering::SeqCst)
}

/// `git rev-parse --short HEAD` in `vault`, with a 5-second timeout. **Never propagates.**
///
/// Every failure — git absent, not a repository, a hung index lock, a non-zero exit, empty output
/// — is `None`. The sha is provenance on a run record; losing it must never cost the run.
///
/// Rust's `Command` has no timeout, so this spawns and polls. The output is one short line, well
/// inside the pipe buffer, so reading it after the process exits cannot deadlock.
pub fn git_sha(vault: &Path) -> Option<String> {
    use crate::childproc::NoConsole;
    let mut child = Command::new("git")
        .no_console()
        .args(["rev-parse", "--short", "HEAD"])
        .current_dir(vault)
        .stdout(Stdio::piped())
        .stderr(Stdio::null())
        .stdin(Stdio::null())
        .spawn()
        .ok()?;
    let deadline = Instant::now() + Duration::from_secs(5);
    let status = loop {
        match child.try_wait() {
            Ok(Some(status)) => break status,
            Ok(None) => {
                if Instant::now() >= deadline {
                    let _ = child.kill();
                    let _ = child.wait();
                    return None;
                }
                std::thread::sleep(Duration::from_millis(10));
            }
            Err(_) => return None,
        }
    };
    if !status.success() {
        return None;
    }
    let mut out = String::new();
    child.stdout.as_mut()?.read_to_string(&mut out).ok()?;
    let trimmed = crate::pystr::strip(&out);
    if trimmed.is_empty() {
        None
    } else {
        Some(trimmed.to_string())
    }
}

/// `state/runs/` — the same `JsonlLedger` the journal uses, pointed at a different root.
pub struct Runs {
    ledger: JsonlLedger,
}

impl Runs {
    pub fn new(vault: &Path) -> Runs {
        Runs { ledger: JsonlLedger::new(vault.join("state").join("runs")) }
    }

    /// Appends, stamping `seq` only if the caller has not.
    pub fn append(&self, record: &mut Record) -> Result<(), LedgerError> {
        record.entry("seq".to_string()).or_insert_with(|| Value::from(next_seq()));
        self.ledger.append(record)
    }

    pub fn read(&mut self, since: Option<&str>) -> Vec<Record> {
        self.ledger.read(since, None)
    }
}

fn record_of(pairs: Vec<(&str, Value)>) -> Record {
    pairs.into_iter().map(|(k, v)| (k.to_string(), v)).collect()
}

fn str_field(record: &Record, key: &str) -> String {
    record.get(key).map(crate::pystr::json_str).unwrap_or_default()
}

/// Open a run. The id is `<runner>-<the first 19 characters of the timestamp>Z`.
pub fn start_run(
    vault: &Path,
    runner: &str,
    now: Option<Timestamp>,
    device: Option<&str>,
    git: Option<&str>,
) -> String {
    let ts = now_ts(now);
    // `ts[:19]` — `2026-08-29T17:00:03.000Z` becomes `2026-08-29T17:00:03`, then `Z` is re-added.
    let stamp: String = ts.chars().take(19).collect();
    let run_id = format!("{runner}-{stamp}Z");
    let mut record = record_of(vec![
        ("run_id", Value::from(run_id.clone())),
        ("phase", Value::from("start")),
        ("runner", Value::from(runner)),
        ("device", Value::from(device.map(str::to_string).unwrap_or_else(device_name))),
        ("ts", Value::from(ts)),
        ("git_before", git.map(Value::from).unwrap_or(Value::Null)),
    ]);
    let _ = Runs::new(vault).append(&mut record);
    run_id
}

/// Record one step of a run.
///
/// `device` is on **every** phase so records sharing a timestamp keep insertion order under the
/// ledger's `(ts, device, seq)` sort — a `start`, `step` and `end` written in the same millisecond
/// stay in order.
pub fn add_step(
    vault: &Path,
    run_id: &str,
    name: &str,
    result: &str,
    counts: &[(&str, i64)],
    message: &str,
    now: Option<Timestamp>,
) {
    let mut record = record_of(vec![
        ("run_id", Value::from(run_id)),
        ("phase", Value::from("step")),
        ("ts", Value::from(now_ts(now))),
        ("device", Value::from(device_name())),
        ("name", Value::from(name)),
        ("result", Value::from(result)),
        ("counts", counts_value(counts)),
        ("message", Value::from(message)),
    ]);
    let _ = Runs::new(vault).append(&mut record);
}

pub fn counts_value(counts: &[(&str, i64)]) -> Value {
    Value::Object(
        counts.iter().map(|(k, v)| ((*k).to_string(), Value::from(*v))).collect(),
    )
}

/// `{"name": …, "result": …, "counts": {…}, "message": …}` — every key present, defaults filled.
///
/// A step handed in with only `name` and `result` comes back with an empty `counts` and an empty
/// `message`, which is what keeps the `end` record's shape constant no matter who built the step.
pub fn normalize_step(step: &Record) -> Record {
    record_of(vec![
        ("name", step.get("name").cloned().unwrap_or_else(|| Value::from(""))),
        ("result", step.get("result").cloned().unwrap_or_else(|| Value::from(""))),
        (
            "counts",
            match step.get("counts") {
                Some(Value::Object(map)) => Value::Object(map.clone()),
                // `dict(s.get("counts") or {})` — anything falsy becomes `{}`.
                _ => Value::Object(serde_json::Map::new()),
            },
        ),
        ("message", step.get("message").cloned().unwrap_or_else(|| Value::from(""))),
    ])
}

static RUN_ID_DAY: std::sync::LazyLock<regex::Regex> =
    std::sync::LazyLock::new(|| regex::Regex::new(r"\d{4}-\d{2}-\d{2}").unwrap());

/// Close a run, folding in every `step` record it wrote plus any handed in directly.
///
/// **The `since` bound is a performance guard with teeth.** A run's own step records can only be
/// in its own day-file or the one before it (a run started before midnight UTC ending after it),
/// and `run_id` carries the date. Reading the whole runs history to fold them would make every run
/// cost the entire history — twice a day, forever. An id whose date does not parse falls back to
/// reading everything rather than folding nothing.
#[allow(clippy::too_many_arguments)]
pub fn end_run(
    vault: &Path,
    run_id: &str,
    result: &str,
    summary: &str,
    journal_records: i64,
    now: Option<Timestamp>,
    git: Option<&str>,
    steps: &[Record],
    runner: Option<&str>,
) -> Record {
    let mut runs = Runs::new(vault);
    // `date.fromisoformat(day) - timedelta(days=1)`. The regex matches `2026-13-45` happily and
    // the parse then fails, which is exactly Python's path to `since = None`.
    let since = RUN_ID_DAY.find(run_id).and_then(|found| {
        jiff::civil::Date::strptime("%Y-%m-%d", found.as_str())
            .ok()
            .and_then(|day| day.yesterday().ok())
            .map(|day| day.strftime("%Y-%m-%d").to_string())
    });
    let recorded: Vec<Record> = runs
        .read(since.as_deref())
        .into_iter()
        .filter(|r| str_field(r, "run_id") == run_id && str_field(r, "phase") == "step")
        .collect();
    let mut folded: Vec<Value> = recorded
        .iter()
        .map(|r| Value::Object(normalize_step(r)))
        .collect();
    folded.extend(steps.iter().map(|s| Value::Object(normalize_step(s))));

    // The caller (`cli::run`) knows the runner it was actually invoked as; an id derived from
    // `run_id` is the fallback for callers that only have the id.
    let resolved_runner = match runner {
        Some(name) => name.to_string(),
        // `run_id.split("-20", 1)[0]` — "<runner>-<YYYY-…>Z" becomes "<runner>".
        None => run_id.split("-20").next().unwrap_or(run_id).to_string(),
    };
    let mut end = record_of(vec![
        ("run_id", Value::from(run_id)),
        ("phase", Value::from("end")),
        ("runner", Value::from(resolved_runner)),
        ("ts", Value::from(now_ts(now))),
        ("device", Value::from(device_name())),
        ("result", Value::from(result)),
        ("summary", Value::from(summary)),
        ("git_after", git.map(Value::from).unwrap_or(Value::Null)),
        ("steps", Value::Array(folded)),
        ("journal_records", Value::from(journal_records)),
    ]);
    let _ = runs.append(&mut end);
    end
}

// ---------------------------------------------------------------------------------------------
// the expected-schedule check
// ---------------------------------------------------------------------------------------------

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RunnerConfig {
    pub name: String,
    pub times: Vec<String>,
    pub tz: String,
    pub grace_minutes: i64,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StatusRow {
    pub runner: String,
    pub due: String,
    pub status: &'static str,
}

/// Read `config/runners.yaml`. A missing or malformed file degrades to no runners.
///
/// The `Err` arm is Python's uncaught `KeyError`/`ValueError` on a *well-formed* YAML file with a
/// bad entry — a runner with no `name`, or a `grace_minutes` that is not an integer. Python does
/// not catch those, and neither does this: a monitoring config that silently drops a runner is
/// worse than one that fails loudly, because the whole point of the file is noticing a runner that
/// did not fire.
pub fn load_runners_config(path: &Path) -> Result<Vec<RunnerConfig>, String> {
    if !path.exists() {
        return Ok(Vec::new());
    }
    let text = match crate::pystr::read_text(path) {
        Ok(text) => text,
        Err(_) => return Ok(Vec::new()),
    };
    let data = match serde_yaml_ng::from_str::<serde_yaml_ng::Value>(&text) {
        Ok(serde_yaml_ng::Value::Mapping(map)) => map,
        // A YAMLError, an empty document, or a non-mapping: all three are "no runners".
        _ => return Ok(Vec::new()),
    };
    let entries = match crate::yaml::get(&data, "runners") {
        Some(serde_yaml_ng::Value::Sequence(items)) => items.clone(),
        _ => Vec::new(),
    };
    let mut out = Vec::new();
    for entry in &entries {
        let map = match entry.as_mapping() {
            Some(map) => map,
            None => return Err("a runners.yaml entry is not a mapping".to_string()),
        };
        let name = match crate::yaml::get(map, "name") {
            Some(value) => crate::pystr::yaml_str(value),
            None => return Err("a runners.yaml entry has no name".to_string()),
        };
        let times = match crate::yaml::get(map, "times") {
            Some(serde_yaml_ng::Value::Sequence(items)) => {
                items.iter().map(crate::pystr::yaml_str).collect()
            }
            _ => Vec::new(),
        };
        let tz = match crate::yaml::get(map, "tz") {
            Some(value) => crate::pystr::yaml_str(value),
            None => "UTC".to_string(),
        };
        let grace_minutes = match crate::yaml::get(map, "grace_minutes") {
            Some(value) => crate::yaml::i64_of(value)
                .ok_or_else(|| format!("grace_minutes for {name} is not an int"))?,
            None => 30,
        };
        out.push(RunnerConfig { name, times, tz, grace_minutes });
    }
    Ok(out)
}

#[derive(Debug, Clone, PartialEq)]
pub struct RunnerSettings {
    pub device: Option<String>,
    pub scheduler: crate::schedule::SchedulerMode,
}

/// The named runner entry's `device` and `scheduler` keys (cutover plan Task 1; Knowlu spec §3).
/// Neither engine's run reads them; `load_runners_config` ignores them (tested). A missing file,
/// entry or key yields the defaults: no device, `Script`.
pub fn runner_settings(path: &Path, name: &str) -> RunnerSettings {
    let default = RunnerSettings { device: None, scheduler: crate::schedule::SchedulerMode::Script };
    let Ok(text) = crate::pystr::read_text(path) else { return default };
    let Ok(doc) = serde_yaml_ng::from_str::<serde_yaml_ng::Value>(&text) else { return default };
    let Some(entries) = doc.get("runners").and_then(|r| r.as_sequence()) else { return default };
    for e in entries {
        if e.get("name").and_then(|n| n.as_str()) == Some(name) {
            return RunnerSettings {
                device: e.get("device").and_then(|d| d.as_str()).map(|s| s.to_string()),
                scheduler: crate::schedule::SchedulerMode::parse(e.get("scheduler").and_then(|s| s.as_str())),
            };
        }
    }
    default
}

/// Every scheduled time for one runner that fell inside the last 24 hours, in order.
///
/// Both yesterday's and today's *local* dates are walked, because a runner scheduled at 18:00
/// local has a due time within the last 24 hours that belongs to yesterday's date whenever the
/// clock is earlier than 18:00 now.
fn due_times(
    times: &[String],
    tz: &TimeZone,
    now: Timestamp,
) -> Result<Vec<Timestamp>, String> {
    let local_now = now.to_zoned(tz.clone());
    let day_ago = now
        .checked_sub(SignedDuration::from_hours(24))
        .map_err(|err| err.to_string())?;
    let mut dues: Vec<Timestamp> = Vec::new();
    for offset in [1i64, 0] {
        // `local_now - timedelta(days=offset)` then `.date()`: an absolute 24-hour shift on an
        // aware datetime, so `SignedDuration`, not a calendar span.
        let shifted = local_now
            .checked_sub(SignedDuration::from_hours(24 * offset))
            .map_err(|err| err.to_string())?;
        let day = shifted.date();
        for hhmm in times {
            let parts: Vec<&str> = hhmm.split(':').collect();
            if parts.len() != 2 {
                return Err(format!("bad runner time {hhmm:?}"));
            }
            let hour: i8 = parts[0]
                .trim()
                .parse()
                .map_err(|_| format!("bad runner time {hhmm:?}"))?;
            let minute: i8 = parts[1]
                .trim()
                .parse()
                .map_err(|_| format!("bad runner time {hhmm:?}"))?;
            let time = jiff::civil::Time::new(hour, minute, 0, 0)
                .map_err(|_| format!("bad runner time {hhmm:?}"))?;
            let due = day
                .to_datetime(time)
                .to_zoned(tz.clone())
                .map_err(|err| err.to_string())?
                .timestamp();
            if day_ago <= due && due <= now {
                dues.push(due);
            }
        }
    }
    dues.sort();
    Ok(dues)
}

fn parse_ts(text: &str) -> Option<Timestamp> {
    text.parse::<Timestamp>().ok()
}

/// For each runner and each due time in the last 24h: `seen` | `late` | `missing` | `crashed`.
///
/// **The 2026-08-26 lesson, encoded.** A `start` without an `end` past grace is *crashed*; no
/// `start` at all past grace is *missing*; inside the grace window it is *late* — and a late run
/// must not be acted on. On that date the scheduler fired 31 minutes late, `list_runs` showed
/// nothing, and the run began 89 seconds after the check that concluded it was dead. Acting on
/// that read fired a duplicate run onto a day whose approval budget was already spent.
///
/// Python opens with `if now.tzinfo is None: now = now.replace(tzinfo=utc)`. That branch is
/// structurally unreachable here — a `Timestamp` is always an absolute instant.
///
/// A `start` record whose `ts` will not parse is skipped, where Python raises. Only a corrupt
/// ledger reaches it, and a monitoring call that dies is worse than one that under-counts.
pub fn expected_status(vault: &Path, now: Timestamp) -> Result<Vec<StatusRow>, String> {
    let config = load_runners_config(&vault.join("config").join("runners.yaml"))?;
    let since = now
        .checked_sub(SignedDuration::from_hours(26))
        .map_err(|err| err.to_string())?
        .to_zoned(TimeZone::UTC)
        .strftime("%Y-%m-%d")
        .to_string();
    let records = Runs::new(vault).read(Some(&since));
    let starts: Vec<&Record> =
        records.iter().filter(|r| str_field(r, "phase") == "start").collect();
    let ends: std::collections::BTreeSet<String> = records
        .iter()
        .filter(|r| str_field(r, "phase") == "end")
        .map(|r| str_field(r, "run_id"))
        .collect();

    let mut out: Vec<StatusRow> = Vec::new();
    for runner in &config {
        let tz = TimeZone::get(&runner.tz)
            .map_err(|_| format!("unknown time zone {:?} for runner {}", runner.tz, runner.name))?;
        let grace = SignedDuration::from_mins(runner.grace_minutes);
        for due in due_times(&runner.times, &tz, now)? {
            let window_start = due
                .checked_sub(SignedDuration::from_mins(5))
                .map_err(|err| err.to_string())?;
            let window_end = due.checked_add(grace).map_err(|err| err.to_string())?;
            let hits: Vec<&&Record> = starts
                .iter()
                .filter(|s| {
                    str_field(s, "runner") == runner.name
                        && match parse_ts(&str_field(s, "ts")) {
                            Some(ts) => window_start <= ts && ts <= window_end,
                            None => false,
                        }
                })
                .collect();
            let status = if let Some(last) = hits.last() {
                if hits.iter().any(|h| ends.contains(&str_field(h, "run_id"))) {
                    "seen"
                } else {
                    let started = parse_ts(&str_field(last, "ts")).unwrap_or(due);
                    let deadline = started.checked_add(grace).unwrap_or(window_end);
                    if now > deadline {
                        "crashed"
                    } else {
                        "late"
                    }
                }
            } else if now <= window_end {
                "late"
            } else {
                "missing"
            };
            out.push(StatusRow {
                runner: runner.name.clone(),
                due: due
                    .to_zoned(TimeZone::UTC)
                    .strftime("%Y-%m-%dT%H:%M:%SZ")
                    .to_string(),
                status,
            });
        }
    }
    Ok(out)
}

/// `--counts name=int` — the shape `engine.runs step` accepts on the command line.
///
/// Python calls `parser.error()`, which exits 2. Here the caller decides; the message is the same.
pub fn parse_counts(pairs: &[String]) -> Result<Vec<(String, i64)>, String> {
    let mut counts = Vec::new();
    for pair in pairs {
        let (key, value) = pair
            .split_once('=')
            .ok_or_else(|| format!("--counts expects name=int, got '{pair}'"))?;
        let parsed: i64 = value
            .parse()
            .map_err(|_| format!("--counts expects name=int, got '{pair}'"))?;
        counts.push((key.to_string(), parsed));
    }
    Ok(counts)
}

/// `f"- {when:%Y-%m-%d %H:%M} {runner} {result} {summary}"`, with the summary flattened.
///
/// The flattening is Python's `" ".join(str(x).split())`: a bare `split()` breaks on *runs* of
/// whitespace and strips the ends, so every newline, tab and repeated space collapses to one
/// space. [`pystr::is_python_space`] is the predicate rather than Rust's `char::is_whitespace`
/// because Python's `str.split()` also breaks on `\x1c`–`\x1f`, which Unicode does not call
/// whitespace.
///
/// A missing `runner` or `result` renders as an empty field where Python raises `KeyError`. No
/// caller can produce that — both are written by `end_run` — and raising here would cost a run
/// the only durable diagnostic it has.
pub fn log_line(end_record: &serde_json::Value, when: DateTime) -> String {
    let field = |key: &str| -> String {
        end_record
            .get(key)
            .map(pystr::json_str)
            .unwrap_or_else(String::new)
    };
    let raw_summary = end_record
        .get("summary")
        .map(pystr::json_str)
        .unwrap_or_default();
    let summary = raw_summary
        .split(pystr::is_python_space)
        .filter(|part| !part.is_empty())
        .collect::<Vec<&str>>()
        .join(" ");
    format!(
        "- {} {} {} {}",
        when.strftime("%Y-%m-%d %H:%M"),
        field("runner"),
        field("result"),
        summary
    )
}

#[cfg(test)]
mod tests {
    //! `tests/test_runs.py`, ported.

    use jiff::civil::date;

    use super::*;

    // --- the rest of tests/test_runs.py (plan Task 14) ---------------------------------------
    //
    // Python's tests monkeypatch `KNOWLU_DEVICE`. Rust tests share one process and run in
    // parallel, so setting a process-wide env var here would leak into every other test that
    // reads `device_name()`. `start_run` already takes a `device` override — that is what the
    // device assertion uses; the `step` and `end` records are checked against `device_name()`
    // itself, which is the same value Python's monkeypatch was standing in for — and the test
    // that does so holds `journal::DEVICE_ENV_MUTEX` for the whole span, since a `history.rs`
    // test mid-`set_var` could otherwise be observed here too.

    use std::path::PathBuf;

    use jiff::civil::date as civil_date;

    const CFG: &str = concat!(
        "runners:\n",
        "  - name: cloud\n",
        "    times: [\"13:00\", \"20:00\"]\n",
        "    tz: UTC\n",
        "    grace_minutes: 45\n",
        "  - name: local\n",
        "    times: [\"12:00\", \"18:00\"]\n",
        "    tz: America/Chicago\n",
        "    grace_minutes: 20\n",
    );

    fn scratch(name: &str) -> PathBuf {
        let dir = std::env::temp_dir().join(format!(
            "qo-runs-{name}-{}-{:?}",
            std::process::id(),
            std::thread::current().id()
        ));
        let _ = std::fs::remove_dir_all(&dir);
        dir
    }

    fn make_vault(name: &str) -> PathBuf {
        let vault = scratch(name);
        std::fs::create_dir_all(vault.join("config")).unwrap();
        pystr::write_text(&vault.join("config").join("runners.yaml"), CFG).unwrap();
        vault
    }

    /// Every record in `state/runs/`, in file order.
    fn records(vault: &Path) -> Vec<serde_json::Value> {
        let dir = vault.join("state").join("runs");
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
                    out.push(serde_json::from_str(line).expect("every run record is valid JSON"));
                }
            }
        }
        out
    }

    fn at(hour: i8, minute: i8, second: i8) -> Timestamp {
        civil_date(2026, 8, 29)
            .at(hour, minute, second, 0)
            .to_zoned(TimeZone::UTC)
            .unwrap()
            .timestamp()
    }

    #[test]
    fn start_step_end_records_full_text_and_fold_steps() {
        // `add_step` below reads the process-global `device_name()` internally (no override is
        // passed), and the assertion further down compares against it — so this test holds
        // `journal::DEVICE_ENV_MUTEX` across both, the same lock every `KNOWLU_DEVICE` setter
        // takes, rather than racing a `history.rs` test that is mid-`set_var`/`remove_var`.
        let _guard = crate::journal::DEVICE_ENV_MUTEX.lock().unwrap();
        let vault = make_vault("startstepend");
        let t0 = at(17, 0, 3);
        let run_id = start_run(&vault, "local", Some(t0), Some("desktop"), Some("a7994ca"));
        assert_eq!(run_id, "local-2026-08-29T17:00:03Z");
        // Captured once, while the lock is held, and reused below instead of calling
        // `device_name()` a second time at the assertion.
        let expected_device = crate::journal::device_name();
        add_step(
            &vault,
            &run_id,
            "coursework",
            "ok",
            &[("created", 0), ("updated", 1)],
            "",
            Some(t0),
        );
        // **Never truncated (F11).** The whole point of the run record is that the run-log line is
        // a summary of something that still holds the full text.
        let long_warning = format!("blount: not an ICS response — {}", "x".repeat(500));
        let extra: Record = serde_json::from_value(serde_json::json!({
            "name": "events", "result": "WARN", "counts": {}, "message": long_warning
        }))
        .unwrap();
        let end = end_run(
            &vault,
            &run_id,
            "WARN",
            "today.md refreshed (133 active)",
            3,
            Some(at(17, 0, 41)),
            Some("b1c2d3e"),
            &[extra],
            None,
        );

        let recs = records(&vault);
        let phases: Vec<&str> = recs.iter().map(|r| r["phase"].as_str().unwrap()).collect();
        assert_eq!(phases, vec!["start", "step", "end"]);
        assert_eq!(recs[0]["runner"], serde_json::json!("local"));
        assert_eq!(recs[0]["device"], serde_json::json!("desktop"));
        assert_eq!(recs[0]["git_before"], serde_json::json!("a7994ca"));
        assert_eq!(recs[1]["device"], serde_json::json!(expected_device));

        assert_eq!(end["result"], serde_json::json!("WARN"));
        assert_eq!(end["git_after"], serde_json::json!("b1c2d3e"));
        assert_eq!(end["journal_records"], serde_json::json!(3));
        let names: Vec<&str> = end["steps"]
            .as_array()
            .unwrap()
            .iter()
            .map(|s| s["name"].as_str().unwrap())
            .collect();
        assert_eq!(names, vec!["coursework", "events"]);
        assert_eq!(end["steps"][1]["message"], serde_json::json!(long_warning));
        assert_eq!(
            end["summary"],
            serde_json::json!("today.md refreshed (133 active)")
        );
        // `runner` was not supplied, so it comes from splitting the id on "-20".
        assert_eq!(end["runner"], serde_json::json!("local"));
        let _ = std::fs::remove_dir_all(&vault);
    }

    #[test]
    fn end_run_tolerates_an_unparseable_day_in_run_id() {
        // The regex matches `2026-13-45`; the date parse then fails and `since` falls back to
        // reading everything, rather than the whole call failing.
        let vault = make_vault("badday");
        start_run(&vault, "cloud", Some(at(13, 0, 0)), None, None);
        let end = end_run(
            &vault,
            "cloud-2026-13-45T00:00:00Z",
            "ok",
            "x",
            0,
            None,
            None,
            &[],
            None,
        );
        assert_eq!(end["run_id"], serde_json::json!("cloud-2026-13-45T00:00:00Z"));
        assert_eq!(end["phase"], serde_json::json!("end"));
        assert_eq!(end["result"], serde_json::json!("ok"));
        let _ = std::fs::remove_dir_all(&vault);
    }

    #[test]
    fn load_runners_config_defaults() {
        let vault = make_vault("cfg");
        let cfg = load_runners_config(&vault.join("config").join("runners.yaml")).unwrap();
        assert_eq!(cfg[0].name, "cloud");
        assert_eq!(cfg[0].grace_minutes, 45);
        assert_eq!(cfg[0].times, vec!["13:00", "20:00"]);
        assert_eq!(cfg[1].tz, "America/Chicago");
        assert_eq!(cfg[1].grace_minutes, 20);
        assert_eq!(
            load_runners_config(&vault.join("missing.yaml")).unwrap(),
            Vec::new()
        );
        let _ = std::fs::remove_dir_all(&vault);
    }

    #[test]
    fn a_runner_with_no_tz_defaults_to_utc_and_no_grace_to_thirty() {
        let vault = scratch("cfgdefaults");
        std::fs::create_dir_all(vault.join("config")).unwrap();
        pystr::write_text(
            &vault.join("config").join("runners.yaml"),
            "runners:\n  - name: bare\n",
        )
        .unwrap();
        let cfg = load_runners_config(&vault.join("config").join("runners.yaml")).unwrap();
        assert_eq!(
            cfg,
            vec![RunnerConfig {
                name: "bare".to_string(),
                times: Vec::new(),
                tz: "UTC".to_string(),
                grace_minutes: 30,
            }]
        );
        let _ = std::fs::remove_dir_all(&vault);
    }

    #[test]
    fn a_runner_entry_with_an_extra_key_is_read_as_python_reads_it() {
        // The cutover plan (Task 1) puts `device:` under the `local` entry so local-run.ps1 can
        // refuse to run on any machine but the designated one. Neither engine reads it: Python's
        // `load_runners_config` picks the four fields it knows and ignores the rest (measured
        // 2026-09-02), and this pins the port to the same shape so the key can never become a
        // parse failure in one engine and not the other.
        let vault = scratch("cfgextrakey");
        std::fs::create_dir_all(vault.join("config")).unwrap();
        pystr::write_text(
            &vault.join("config").join("runners.yaml"),
            "runners:\n  - name: local\n    times: [\"12:00\", \"18:00\"]\n    tz: America/Chicago\n    grace_minutes: 20\n    device: SOME-MACHINE\n",
        )
        .unwrap();
        let cfg = load_runners_config(&vault.join("config").join("runners.yaml")).unwrap();
        assert_eq!(
            cfg,
            vec![RunnerConfig {
                name: "local".to_string(),
                times: vec!["12:00".to_string(), "18:00".to_string()],
                tz: "America/Chicago".to_string(),
                grace_minutes: 20,
            }]
        );
        let _ = std::fs::remove_dir_all(&vault);
    }

    #[test]
    fn a_runner_entry_with_no_name_is_a_loud_failure_not_a_silent_drop() {
        // Python raises KeyError here and does not catch it. Dropping the runner instead would
        // make the one file whose job is noticing a runner that did not fire quietly monitor one
        // fewer runner.
        let vault = scratch("cfgnoname");
        std::fs::create_dir_all(vault.join("config")).unwrap();
        pystr::write_text(
            &vault.join("config").join("runners.yaml"),
            "runners:\n  - times: [\"13:00\"]\n",
        )
        .unwrap();
        assert!(load_runners_config(&vault.join("config").join("runners.yaml")).is_err());
        let _ = std::fs::remove_dir_all(&vault);
    }

    #[test]
    fn expected_status_seen_late_missing_crashed() {
        let vault = make_vault("status");
        // cloud due 13:00Z: a run started 13:31 and ended -> seen. Late but fine; that is the
        // 2026-08-26 lesson, where a 31-minute-late run looked identical to a dead one.
        let rid = start_run(&vault, "cloud", Some(at(13, 31, 48)), None, None);
        end_run(&vault, &rid, "ok", "x", 0, Some(at(13, 40, 0)), None, &[], None);
        // local due 12:00 CT (17:00Z): started, never ended -> crashed once past grace.
        start_run(&vault, "local", Some(at(17, 0, 3)), None, None);

        // cloud's 20:00Z is due and inside its 45-minute grace.
        let now = at(20, 10, 0);
        let rows = expected_status(&vault, now).unwrap();
        let status = |runner: &str, due: &str| -> &'static str {
            rows.iter()
                .find(|r| r.runner == runner && r.due == due)
                .unwrap_or_else(|| panic!("no row for {runner} {due}\n{rows:#?}"))
                .status
        };
        assert_eq!(status("cloud", "2026-08-29T13:00:00Z"), "seen");
        assert_eq!(status("cloud", "2026-08-29T20:00:00Z"), "late");
        assert_eq!(status("local", "2026-08-29T17:00:00Z"), "crashed");
        // yesterday's 18:00 CT, never ran
        assert_eq!(status("local", "2026-08-28T23:00:00Z"), "missing");
        assert_eq!(rows.len(), 4, "{rows:#?}");
        let _ = std::fs::remove_dir_all(&vault);
    }

    #[test]
    fn a_run_still_inside_its_grace_window_is_late_not_crashed() {
        // The distinction that matters: acting on a "late" run is what fired a duplicate onto a
        // day whose approval budget was already spent.
        let vault = make_vault("grace");
        start_run(&vault, "cloud", Some(at(13, 5, 0)), None, None);
        let rows = expected_status(&vault, at(13, 30, 0)).unwrap();
        let row = rows
            .iter()
            .find(|r| r.runner == "cloud" && r.due == "2026-08-29T13:00:00Z")
            .unwrap();
        assert_eq!(row.status, "late");
        // …and crashed once the grace window has passed.
        let rows = expected_status(&vault, at(14, 30, 0)).unwrap();
        let row = rows
            .iter()
            .find(|r| r.runner == "cloud" && r.due == "2026-08-29T13:00:00Z")
            .unwrap();
        assert_eq!(row.status, "crashed");
        let _ = std::fs::remove_dir_all(&vault);
    }

    #[test]
    fn malformed_runners_yaml_degrades_to_empty() {
        let vault = scratch("badyaml");
        std::fs::create_dir_all(vault.join("config")).unwrap();
        pystr::write_text(
            &vault.join("config").join("runners.yaml"),
            "runners: [unclosed\n",
        )
        .unwrap();
        assert_eq!(
            load_runners_config(&vault.join("config").join("runners.yaml")).unwrap(),
            Vec::new()
        );
        assert_eq!(expected_status(&vault, at(20, 10, 0)).unwrap(), Vec::new());
        let _ = std::fs::remove_dir_all(&vault);
    }

    #[test]
    fn step_rejects_non_int_counts_and_records_valid_ones() {
        let vault = make_vault("clistep");
        let run_id = start_run(&vault, "cloud", Some(at(13, 0, 0)), None, None);
        assert!(parse_counts(&["created=three".to_string()]).is_err());
        assert!(parse_counts(&["created".to_string()]).is_err());
        let counts = parse_counts(&["created=2".to_string(), "proposed=1".to_string()]).unwrap();
        let pairs: Vec<(&str, i64)> =
            counts.iter().map(|(k, v)| (k.as_str(), *v)).collect();
        add_step(&vault, &run_id, "gmail", "ok", &pairs, "", None);

        let bare: Record = serde_json::from_value(serde_json::json!({
            "name": "bare", "result": "ok"
        }))
        .unwrap();
        let end = end_run(
            &vault,
            &run_id,
            "ok",
            "x",
            0,
            Some(at(13, 5, 0)),
            None,
            &[bare],
            None,
        );
        assert_eq!(
            end["steps"][0]["counts"],
            serde_json::json!({"created": 2, "proposed": 1})
        );
        // A step handed in bare comes back with every key filled.
        assert_eq!(
            end["steps"][1],
            serde_json::json!({"name": "bare", "result": "ok", "counts": {}, "message": ""})
        );
        assert!(records(&vault).iter().all(|r| r["seq"].is_i64()));
        let _ = std::fs::remove_dir_all(&vault);
    }

    /// The artefact oracle for `state/runs/`: four records, exactly as **Python** writes them.
    ///
    /// `tests/fixtures/run-records-reference.json` was produced by calling the Python
    /// `engine.runs` through `start_run` / `add_step` / `end_run` at pinned timestamps. It is a
    /// **frozen reference**: if this fails, the Rust engine is wrong. Do not regenerate it.
    ///
    /// Two fields are dropped from both sides before comparing, and only two: `seq`, which is a
    /// *process* counter and so depends on how many records this process has already written, and
    /// `device`, which is the machine's name. Both are pinned by unit tests above instead; leaving
    /// them in would make the reference unportable rather than stronger.
    ///
    /// What it does pin: the run-id shape, the millisecond timestamp spelling, `git_before` /
    /// `git_after`, the folded step list in order, an **untruncated** multi-line message, the
    /// filled-in defaults on a bare step, and `journal_records`.
    #[test]
    fn the_run_records_match_python_byte_for_byte() {
        let vault = make_vault("oracle");
        let t0 = at(17, 0, 3);
        let run_id = start_run(&vault, "local", Some(t0), Some("desktop"), Some("a7994ca"));
        add_step(
            &vault,
            &run_id,
            "coursework",
            "ok",
            &[("created", 0), ("updated", 1)],
            "",
            Some(t0),
        );
        add_step(
            &vault,
            &run_id,
            "events",
            "WARN",
            &[],
            "blount: not an ICS response\nsecond line",
            Some(at(17, 0, 12)),
        );
        let bare: Record =
            serde_json::from_value(serde_json::json!({"name": "bare", "result": "ok"})).unwrap();
        end_run(
            &vault,
            &run_id,
            "WARN",
            "today.md refreshed (133 active)",
            3,
            Some(at(17, 0, 41)),
            Some("b1c2d3e"),
            &[bare],
            Some("local"),
        );

        let rows: Vec<serde_json::Value> = records(&vault)
            .into_iter()
            .map(|mut record| {
                if let Some(map) = record.as_object_mut() {
                    map.remove("seq");
                    map.remove("device");
                }
                record
            })
            .collect();
        let rendered = crate::ledger::dumps_value(&serde_json::Value::Array(rows));
        let expected = std::fs::read_to_string("tests/fixtures/run-records-reference.json")
            .expect("the frozen Python reference is committed");
        assert_eq!(
            rendered, expected,
            "the Rust run records diverged from Python. The reference is frozen: fix the engine, \
             do not regenerate tests/fixtures/run-records-reference.json."
        );
        let _ = std::fs::remove_dir_all(&vault);
    }

    #[test]
    fn git_sha_reads_this_repo_and_never_propagates() {
        // The repo the tests run in is a git checkout, so this exercises the success path.
        let here = git_sha(Path::new("."));
        assert!(here.is_some(), "cargo test runs from the repo root");
        let sha = here.unwrap();
        assert!(
            sha.len() >= 7 && sha.chars().all(|c| c.is_ascii_hexdigit()),
            "{sha:?}"
        );
        // Not a repository: git exits non-zero, and every failure is None rather than a raise.
        let outside = scratch("nogit");
        std::fs::create_dir_all(&outside).unwrap();
        assert_eq!(git_sha(&outside), None);
        let _ = std::fs::remove_dir_all(&outside);
    }

    #[test]
    fn due_times_walks_yesterday_and_today_and_keeps_only_the_last_24_hours() {
        let tz = TimeZone::get("America/Chicago").unwrap();
        let times = vec!["12:00".to_string(), "18:00".to_string()];
        let dues = due_times(&times, &tz, at(20, 10, 0)).unwrap();
        let rendered: Vec<String> = dues
            .iter()
            .map(|d| {
                d.to_zoned(TimeZone::UTC)
                    .strftime("%Y-%m-%dT%H:%M:%SZ")
                    .to_string()
            })
            .collect();
        // 08-28 18:00 CT and 08-29 12:00 CT are inside the window; 08-28 12:00 CT is 27 hours ago
        // and 08-29 18:00 CT has not happened yet.
        assert_eq!(
            rendered,
            vec!["2026-08-28T23:00:00Z", "2026-08-29T17:00:00Z"]
        );
    }

    #[test]
    fn a_malformed_runner_time_is_an_error_not_a_skipped_slot() {
        let tz = TimeZone::get("UTC").unwrap();
        for bad in ["13", "13:00:00", "ab:cd", "25:00"] {
            assert!(
                due_times(&[bad.to_string()], &tz, at(20, 10, 0)).is_err(),
                "{bad:?} should not parse"
            );
        }
    }

    #[test]
    fn log_line_renders_from_end_record() {
        let end = serde_json::json!({
            "runner": "cloud",
            "result": "ok",
            "summary": "today.md refreshed (5 active)"
        });
        assert_eq!(
            log_line(&end, date(2026, 8, 29).at(8, 1, 0, 0)),
            "- 2026-08-29 08:01 cloud ok today.md refreshed (5 active)"
        );
    }

    #[test]
    fn log_line_flattens_a_multiline_summary() {
        let end = serde_json::json!({
            "runner": "cloud",
            "result": "WARN",
            "summary": "events pass failed (line1\nline2)"
        });
        let line = log_line(&end, date(2026, 8, 29).at(8, 1, 0, 0));
        assert!(!line.contains('\n'));
        assert_eq!(
            line,
            "- 2026-08-29 08:01 cloud WARN events pass failed (line1 line2)"
        );
    }

    #[test]
    fn runs_of_whitespace_collapse_to_one_space_and_the_ends_are_stripped() {
        // `" ".join(x.split())`, not `x.replace("\n", " ")`.
        let end = serde_json::json!({
            "runner": "local",
            "result": "ok",
            "summary": "  a\t\tb \n\n c  "
        });
        assert_eq!(
            log_line(&end, date(2026, 8, 29).at(8, 1, 0, 0)),
            "- 2026-08-29 08:01 local ok a b c"
        );
    }

    #[test]
    fn an_absent_summary_is_empty_but_a_null_one_is_the_word_none() {
        // Python: `str(end_record.get("summary", ""))`. Absent yields the default; present-and-null
        // yields `str(None)`.
        let when = date(2026, 8, 29).at(8, 1, 0, 0);
        assert_eq!(
            log_line(&serde_json::json!({"runner": "local", "result": "ok"}), when),
            "- 2026-08-29 08:01 local ok "
        );
        assert_eq!(
            log_line(
                &serde_json::json!({"runner": "local", "result": "ok", "summary": null}),
                when
            ),
            "- 2026-08-29 08:01 local ok None"
        );
    }

    #[test]
    fn runner_settings_reads_device_and_scheduler_and_load_runners_config_still_ignores_them() {
        let d = std::env::temp_dir().join(format!("qo-runners-{}", std::process::id()));
        std::fs::create_dir_all(&d).unwrap();
        let p = d.join("runners.yaml");
        std::fs::write(&p, "runners:\n  - name: cloud\n    times: [\"13:00\"]\n  - name: local\n    times: [\"12:00\", \"18:00\"]\n    tz: America/Chicago\n    grace_minutes: 20\n    device: BOX-1\n    scheduler: app\n").unwrap();
        let s = runner_settings(&p, "local");
        assert_eq!(s.device.as_deref(), Some("BOX-1"));
        assert_eq!(s.scheduler, crate::schedule::SchedulerMode::App);
        assert_eq!(runner_settings(&p, "cloud").scheduler, crate::schedule::SchedulerMode::Script);
        assert_eq!(load_runners_config(&p).unwrap().len(), 2, "unknown keys in an entry are ignored (cutover plan Task 1)");
    }
}
