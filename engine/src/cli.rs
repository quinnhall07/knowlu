//! Port of `engine/cli.py` — `knowlu-engine rank`, the one command both runners call.
//!
//! The step order is the contract: id repair, then the journal passes, then approvals, then the
//! ranking, then the page. Each earlier step can change what a later one sees, so reordering is a
//! behaviour change even when nothing errors.
//!
//! # Every step is ported, and a test says so
//!
//! `cli.run` in Python touches eleven modules; as of wave 7 Task 15 all eleven are here. Waves
//! 3–7 carried the not-yet-ported steps in [`RunOutcome::not_ported`] under one rule — **a stub
//! must never be silent** — because the failure mode this file is written against is a cutover
//! where a pass simply stops running and nothing says so: `today.md` still looks current,
//! `runner-log.md` still says `ok`, and event discovery is dead for weeks. The list is now empty
//! by construction and a test pins it that way; it stays in the outcome (and `rank` still prints
//! it when non-empty) so that a step removed from this function can only ever be removed loudly.

use std::path::{Path, PathBuf};

use jiff::civil::{Date, DateTime};
use jiff::tz::TimeZone;
use jiff::Timestamp;

use crate::approvals::{count_proposals_created, defer_over_budget, process_approvals};
use crate::calfeed::load_calendar_events;
use crate::eventemit::emit_digest;
use crate::eventfeed::load_discovered_events;
use crate::eventfilter::prefilter_events;
use crate::eventledger::load_ledger;
use crate::eventroster::{read_roster, relevant_events, write_roster};
use crate::events::{load_events_config, load_interests};
use crate::ids::{ensure_ids, rel};
use crate::info::info_pass;
use crate::ingest::IngestError;
use crate::journal::Journal;
use crate::models::{apply_slice_default, load_tasks};
use crate::passes::{detect_external, verify_tail, VERIFY_TAIL_LIMIT};
use crate::planning::load_planning;
use crate::pystr;
use crate::ranking::{designate_today, rank};
use crate::runs::{end_run, git_sha, start_run};
use crate::render::{render_today, RenderOpts};
use crate::weekcal::WeekCalendar;
use crate::write::{write_literals, WriteContext, WriteOpts};

/// One entry of the run record's `steps` list.
#[derive(Debug, Clone, PartialEq)]
pub struct Step {
    pub name: &'static str,
    pub result: &'static str,
    pub counts: Vec<(&'static str, i64)>,
    pub message: String,
}

#[derive(Debug)]
pub struct RunOutcome {
    pub output: PathBuf,
    pub status: &'static str,
    pub summary: String,
    pub steps: Vec<Step>,
    /// Named, in order, so `rank` can print them and no step can go missing quietly.
    pub not_ported: Vec<&'static str>,
    pub run_id: String,
}

#[derive(Debug)]
pub enum RunError {
    BadRunId(String),
    BadDate(String),
    Io(String),
}

impl RunError {
    /// Python's crash summary interpolates `type(err).__name__` — the *Python* exception class.
    /// The nearest honest equivalent is this crate's variant name; the message that follows it is
    /// identical in shape.
    fn variant_name(&self) -> &'static str {
        match self {
            RunError::BadRunId(_) => "BadRunId",
            RunError::BadDate(_) => "BadDate",
            RunError::Io(_) => "Io",
        }
    }

    /// What `python -m engine.cli` exits with. `main` there catches `ValueError` and hands it to
    /// `parser.error`, which is exit **2** — and both validation refusals are `ValueError`s.
    /// Anything else escapes as a traceback: exit 1. (A `ValueError` raised deeper inside `run`
    /// would be 2 in Python as well; here every such crash is `Io` and exits 1 — recorded in the
    /// Task 17 sweep. `scripts/local-run.ps1` tests only for non-zero.)
    pub fn exit_code(&self) -> u8 {
        match self {
            RunError::BadRunId(_) | RunError::BadDate(_) => 2,
            RunError::Io(_) => 1,
        }
    }
}

impl std::fmt::Display for RunError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            RunError::BadRunId(v) => write!(
                f,
                "--run-id must look like <runner>-YYYY-MM-DDTHH:MM:SSZ, got {v:?}"
            ),
            RunError::BadDate(v) => write!(f, "--today must be an ISO date, got {v:?}"),
            RunError::Io(m) => write!(f, "{m}"),
        }
    }
}

/// The vault's clock: `config/ingest.yaml`'s `timezone` when the file parses and names a zone
/// `jiff` recognises, otherwise the machine's local zone — the same fallback `jiff::Zoned::now()`
/// uses internally (`TimeZone::try_system().unwrap_or(TimeZone::UTC)`), spelled out here so
/// `local_now` and anything else that needs the vault's zone (`surface::local_stamp`) agree with
/// it by construction rather than by coincidence.
pub fn vault_zone(vault: &Path) -> TimeZone {
    let config = crate::yaml::mapping_from_file(&vault.join("config").join("ingest.yaml"));
    if let Some(name) = crate::yaml::get(&config, "timezone").and_then(crate::yaml::text) {
        if let Ok(tz) = TimeZone::get(&name) {
            return tz;
        }
    }
    TimeZone::try_system().unwrap_or(TimeZone::UTC)
}

/// Wall-clock time in the vault's own timezone, as a naive datetime.
///
/// `config/ingest.yaml`'s `timezone` is the vault's clock; everything on the page is local wall
/// time. An unreadable config or an unknown zone falls back to the machine's local time, exactly
/// as Python's bare `except Exception` does — a bad timezone string must not cost a run.
pub fn local_now(vault: &Path) -> DateTime {
    Timestamp::now().to_zoned(vault_zone(vault)).datetime()
}

fn run_id_is_well_formed(value: &str) -> bool {
    // Python: ^[a-z][a-z-]*-\d{4}-\d{2}-\d{2}T\d{2}:\d{2}:\d{2}Z$
    static RE: std::sync::LazyLock<regex::Regex> = std::sync::LazyLock::new(|| {
        regex::Regex::new(r"^[a-z][a-z-]*-\d{4}-\d{2}-\d{2}T\d{2}:\d{2}:\d{2}Z$").unwrap()
    });
    RE.is_match(value)
}

/// A run that dies partway still has to leave a record and a log line.
///
/// Python does this in an `except Exception ... raise`: write the FAIL `end` record, append the
/// log line, then re-raise. **A missing run-log line reads as "nothing to do", not "this failed"**
/// — which is why the record is written before the error is returned rather than instead of it.
fn crash(vault: &Path, run_id: &str, runner: &str, now: DateTime, err: RunError) -> RunError {
    let flattened: Vec<String> = err
        .to_string()
        .split(pystr::is_python_space)
        .filter(|part| !part.is_empty())
        .map(str::to_string)
        .collect();
    let summary = format!(
        "engine crashed: {}: {}",
        err.variant_name(),
        flattened.join(" ")
    );
    let end = end_run(
        vault,
        run_id,
        "FAIL",
        &summary,
        0,
        None,
        git_sha(vault).as_deref(),
        &[],
        Some(runner),
    );
    let _ = append_run_log(
        vault,
        &record_str(&end, "runner"),
        "FAIL",
        &record_str(&end, "summary"),
        Some(now),
    );
    err
}

fn record_str(record: &crate::ledger::Record, key: &str) -> String {
    record.get(key).map(pystr::json_str).unwrap_or_default()
}

/// One `Step` as the run record's `steps` list wants it.
///
/// `pub(crate)` since plan 2 Task 8: `coursework` builds run records of the same shape, and
/// a second constructor for one record shape is how the two drift apart.
pub(crate) fn step_record(step: &Step) -> crate::ledger::Record {
    let mut record = crate::ledger::Record::new();
    record.insert("name".into(), serde_json::Value::from(step.name));
    record.insert("result".into(), serde_json::Value::from(step.result));
    record.insert("counts".into(), crate::runs::counts_value(&step.counts));
    record.insert("message".into(), serde_json::Value::from(step.message.clone()));
    record
}

fn count(counts: &[(&'static str, i64)], key: &str) -> i64 {
    counts.iter().find(|(k, _)| *k == key).map(|(_, v)| *v).unwrap_or(0)
}

/// The two network reads a run performs, replaceable so a test can drive the **whole** run offline
/// against a scripted feed.
///
/// Python's `test_cli.py` monkeypatches `calfeed.fetch_ics` and `eventfeed.fetch_event_source`
/// as module globals; `calfeed` and `eventfeed` already carry this seam one level down, and
/// `run_with` only threads it through. Production (`run`) passes neither, so the default fetchers
/// are the ones both modules would have used anyway.
#[derive(Default, Clone, Copy)]
pub struct Fetchers<'a> {
    pub calendar: Option<&'a dyn Fn(&str) -> Result<String, String>>,
    pub events: Option<&'a dyn Fn(&str) -> Result<String, String>>,
}

/// Rank, run the passes, and write `state/today.md`.
///
/// Mirrors `cli.run`'s step order exactly. Returns what the run record carried, so a caller can
/// see it without re-reading `state/runs/`.
pub fn run(
    vault: &Path,
    today_iso: Option<&str>,
    runner: &str,
    run_id: Option<&str>,
) -> Result<RunOutcome, RunError> {
    run_with(vault, today_iso, runner, run_id, Fetchers::default())
}

/// [`run`] with the network seam exposed. Everything else is identical.
pub fn run_with(
    vault: &Path,
    today_iso: Option<&str>,
    runner: &str,
    run_id: Option<&str>,
    fetchers: Fetchers<'_>,
) -> Result<RunOutcome, RunError> {
    if let Some(id) = run_id {
        if !run_id_is_well_formed(id) {
            return Err(RunError::BadRunId(id.to_string()));
        }
    }
    let now = local_now(vault);
    let today = match today_iso {
        Some(iso) => Date::strptime("%Y-%m-%d", iso)
            .map_err(|_| RunError::BadDate(iso.to_string()))?,
        None => Timestamp::now().to_zoned(TimeZone::system()).date(),
    };
    let via = match runner {
        "local" => "local-runner",
        "cloud" => "cloud-routine",
        _ => "cli",
    };
    // A caller-supplied id means the run record was already opened by whoever minted it (the
    // cloud routine does exactly that, so every write it makes carries the same run_id).
    let run_id = match run_id {
        Some(id) => id.to_string(),
        None => start_run(vault, runner, None, None, git_sha(vault).as_deref()),
    };

    let ctx = WriteContext { actor: "agent:approvals".into(), via: via.into(), run_id: Some(run_id.clone()) };
    let mut journal = Journal::new(vault);
    let mut pass_log: Vec<String> = Vec::new();
    // Empty since wave 7 Task 15; `every_unported_step_is_named_rather_than_silently_skipped`
    // asserts it. Push here only if a step is deliberately taken out of this run.
    let not_ported: Vec<&'static str> = Vec::new();

    // Spec §5: idfix -> verify_tail -> detect_external -> info_pass, all no-ops on a clean vault.
    {
        let idfix = ctx.with_actor("system:idfix");
        let mut writer = |path: &Path, literals: &[(String, String)]| -> Result<(), IngestError> {
            let rel_path = rel(vault, path);
            write_literals(vault, &rel_path, literals, &idfix, &mut journal, &WriteOpts::default())
                .map(|_| ())
                .map_err(|e| IngestError::Io(e.to_string()))
        };
        pass_log.extend(ensure_ids(vault, Some(&mut writer)));
    }
    pass_log.extend(verify_tail(vault, &mut journal, &ctx, VERIFY_TAIL_LIMIT));
    pass_log.extend(detect_external(vault, &mut journal, &ctx));
    // Python lets a failed close raise out of the run; here it lands as a crashed run record.
    pass_log.extend(
        info_pass(vault, today, &ctx, Some(&mut journal))
            .map_err(|e| crash(vault, &run_id, runner, now, RunError::Io(e.to_string())))?,
    );

    // Spec §2.3/§6: a record more than 5 minutes in the future is still honoured (the conflict
    // rule needs an order) but must WARN, naming the device whose clock is wrong — otherwise
    // later-timestamp-wins quietly hands that device every conflict it takes part in.
    let skew = journal.skew_warnings(Timestamp::now(), 5);

    let planning = load_planning(&vault.join("config").join("planning.yaml"));

    // Trim same-day overflow BEFORE the approvals pass, so the count below and the digest below
    // both see a queue that already respects the ceiling.
    let deferred = defer_over_budget(
        vault,
        today,
        planning.daily_approval_budget,
        &ctx,
        &mut journal,
    );
    let mut approvals = process_approvals(vault, today, now, &ctx, &mut journal);
    // The events pass — five modules, wave 5, PORTED.
    //
    // Python wraps this whole stretch in `except Exception` with the comment "a crash here would
    // cost the whole morning's ranking". Rust has no equivalent of that catch: a panic aborts the
    // process and no `today.md` is written at all. So this block, and every function it reaches,
    // is written to be UNABLE to panic — no unwrap, no expect, no slice indexing, no unchecked
    // arithmetic. `events_pass` returns its warnings instead of raising, and every module it calls
    // degrades to a warning internally.
    let remaining_budget =
        std::cmp::max(0, planning.daily_approval_budget - count_proposals_created(vault, today));
    let (events_config, mut event_warnings) =
        load_events_config(&vault.join("config").join("events.yaml"));
    let mut coming_up: Vec<crate::render::ComingUpEvent> = Vec::new();
    if events_config.sources.iter().any(|s| s.enabled) {
        let (discovered, feed_warnings) = load_discovered_events(vault, fetchers.events);
        let feeds_failed = !feed_warnings.is_empty();
        event_warnings.extend(feed_warnings);

        let (interests, interests_warnings) =
            load_interests(&vault.join("profile").join("interests.md"));
        event_warnings.extend(interests_warnings);

        let (mut candidates, filtered) =
            prefilter_events(&discovered, &interests, &events_config, today);
        let roster_path = vault.join("state").join("events.md");
        if candidates.is_empty() && feeds_failed {
            // Every feed failed this run — keep the last known roster rather than blanking the
            // page Quinn reads.
            candidates = read_roster(&roster_path);
        }

        let mut ledger_warnings: Vec<String> = Vec::new();
        let ledger = load_ledger(vault, Some(&mut ledger_warnings));
        for w in ledger_warnings {
            event_warnings.push(format!("ledger: {w}"));
        }

        // The pre-filter's drops go to the roster too: an unreachable drop is an unauditable one
        // (spec §2/§5).
        if let Err(err) = write_roster(
            &roster_path,
            &candidates,
            &ledger,
            &events_config,
            today,
            &filtered,
        ) {
            event_warnings.push(format!("roster unwritable ({err})"));
        }

        let (_, emitted) = emit_digest(
            vault,
            &candidates,
            &ledger,
            &events_config,
            today,
            Some(remaining_budget),
            Some(&ctx.with_actor("agent:events")),
            &mut journal,
        );
        approvals.events_in_digest += emitted as i64;
        coming_up = relevant_events(&candidates, &ledger)
            .into_iter()
            .map(|e| {
                // start() must be read before the String fields move out of `e`.
                let start = e.start();
                crate::render::ComingUpEvent {
                    uid: e.uid,
                    title: e.title,
                    start,
                    location: e.location,
                    organizer: e.organizer,
                }
            })
            .collect();
    }

    // Calendar busy-time ingest — wave 4, PORTED.
    let (cal_events, cal_warnings) = load_calendar_events(vault, today, fetchers.calendar);

    let cal = WeekCalendar::from_file(
        &vault.join("config").join("week_template.yaml"),
        cal_events,
    );

    let mut skipped: Vec<String> = Vec::new();
    let mut tasks = load_tasks(&vault.join("tasks"), Some(&mut skipped));
    apply_slice_default(&mut tasks, planning.slice_hours);
    let ranked = rank(&tasks, today, &cal);
    let designated = designate_today(&ranked, today, &cal, Some(&planning));

    // Gauge counts (S2 §7.7; plan 2 Task 7, F6). Three integers on the record `rank` already
    // writes every run — the trend history the gauge could never backfill, because nothing else
    // stores what runway was on a day that has passed. `runway_days` uses the topline's own
    // rounding so the header and the gauge can never disagree by one.
    let gauge_runway = crate::render::runway_days(&ranked, today, &cal).round() as i64;
    let gauge_cap = crate::render::capacity_breakdown(&cal, today).capacity;
    let (gauge_must, _) = crate::ranking::partition_must_do(&ranked, today, &cal);
    let gauge_must_hours: f64 = gauge_must.iter().map(|t| t.remaining_hours()).sum();
    // Tenths as an integer: a run record's counts are i64, and a deficit of 2.5h has to survive
    // the round trip. Clamped at zero — a surplus is not a negative deficit, it is no deficit.
    let gauge_deficit_x10 = ((gauge_must_hours - gauge_cap).max(0.0) * 10.0).round() as i64;

    let output = vault.join("state").join("today.md");
    if let Some(parent) = output.parent() {
        std::fs::create_dir_all(parent)
            .map_err(|e| crash(vault, &run_id, runner, now, RunError::Io(e.to_string())))?;
    }
    let page = render_today(
        &ranked,
        &designated,
        today,
        &cal,
        RenderOpts {
            runner: Some(runner),
            generated_at: Some(now),
            planning: Some(&planning),
            approvals: Some(&approvals),
            vault: Some(vault),
            events: &coming_up,
        },
    );
    pystr::write_text(&output, &page)
        .map_err(|e| crash(vault, &run_id, runner, now, RunError::Io(e.to_string())))?;

    // -----------------------------------------------------------------------
    // The run record's problems, summary and steps
    // -----------------------------------------------------------------------
    let mut problems: Vec<String> = Vec::new();
    if !skipped.is_empty() {
        problems.push(format!("{} unreadable: {}", skipped.len(), skipped.join(", ")));
    }
    problems.extend(first_with_count("calendar", &cal_warnings));
    problems.extend(first_with_count("events", &event_warnings));
    problems.extend(first_with_count("approvals", &approvals.warnings));
    problems.extend(first_with_count("journal", journal.warnings()));
    if !skew.is_empty() {
        // skew_warnings renders "<ts> from <device> is <n> min in the future"; the device is the
        // actionable half, so the run-log line leads with it.
        let device = skew[0]
            .split_once(" from ")
            .map(|(_, rest)| rest.split_once(" is ").map(|(d, _)| d).unwrap_or(rest))
            .unwrap_or("")
            .to_string();
        problems.push(format!("journal: {} record(s) from the future ({device})", skew.len()));
    }

    let passes_counts: Vec<(&'static str, i64)> = vec![
        ("unreadable", count_prefix(&pass_log, "unreadable")),
        ("unwritable", count_prefix(&pass_log, "unwritable")),
        ("re_applied", count_prefix(&pass_log, "re-applied")),
        ("external", count_prefix(&pass_log, "external")),
        ("assigned", count_prefix(&pass_log, "assigned")),
        ("duplicate", count_prefix(&pass_log, "duplicate")),
        ("unmigrated", count_prefix(&pass_log, "journal has no migration records")),
        ("future", skew.len() as i64),
    ];
    let passes_bad = count(&passes_counts, "unreadable")
        + count(&passes_counts, "unwritable")
        + count(&passes_counts, "re_applied")
        + count(&passes_counts, "duplicate")
        + count(&passes_counts, "unmigrated");
    let passes_result = if passes_bad > 0 { "WARN" } else { "ok" };
    if passes_bad > 0 {
        let mut line = format!(
            "passes: {} unreadable, {} unwritable, {} re-applied",
            count(&passes_counts, "unreadable"),
            count(&passes_counts, "unwritable"),
            count(&passes_counts, "re_applied"),
        );
        if count(&passes_counts, "duplicate") > 0 {
            line.push_str(&format!(", {} duplicate", count(&passes_counts, "duplicate")));
        }
        if count(&passes_counts, "unmigrated") > 0 {
            line.push_str(&format!(", {} unmigrated", count(&passes_counts, "unmigrated")));
        }
        problems.push(line);
    }

    let status = if problems.is_empty() { "ok" } else { "WARN" };
    let mut summary = format!("today.md refreshed ({} active", tasks.len());
    let acted: Vec<(usize, &str)> = vec![
        (approvals.executed.len(), "executed"),
        (approvals.expired.len(), "expired"),
        (approvals.woken.len(), "woken"),
        (approvals.rejected.len(), "rejected"),
        (deferred.len(), "deferred"),
    ];
    let segs: Vec<String> = acted
        .iter()
        .filter(|(n, _)| *n > 0)
        .map(|(n, label)| format!("{n} {label}"))
        .collect();
    if !segs.is_empty() {
        summary.push_str("; approvals: ");
        summary.push_str(&segs.join(", "));
    }
    if !problems.is_empty() {
        summary.push_str("; ");
        summary.push_str(&problems.join("; "));
    }
    summary.push(')');

    let steps = vec![
        Step {
            name: "passes",
            result: passes_result,
            counts: passes_counts,
            message: pass_log.join("\n"),
        },
        Step {
            name: "tasks",
            result: if skipped.is_empty() { "ok" } else { "WARN" },
            counts: vec![
                ("active", tasks.len() as i64),
                ("unreadable", skipped.len() as i64),
                ("runway_days", gauge_runway),
                ("deficit_hours_x10", gauge_deficit_x10),
                ("must_count", gauge_must.len() as i64),
            ],
            message: if skipped.is_empty() {
                String::new()
            } else {
                format!("{} unreadable: {}", skipped.len(), skipped.join(", "))
            },
        },
        Step {
            name: "calendar",
            result: if cal_warnings.is_empty() { "ok" } else { "WARN" },
            counts: Vec::new(),
            message: cal_warnings.join("\n"),
        },
        Step {
            name: "events",
            result: if event_warnings.is_empty() { "ok" } else { "WARN" },
            counts: vec![("coming_up", coming_up.len() as i64)],
            message: event_warnings.join("\n"),
        },
        Step {
            name: "approvals",
            result: if approvals.warnings.is_empty() { "ok" } else { "WARN" },
            counts: vec![
                ("executed", approvals.executed.len() as i64),
                ("expired", approvals.expired.len() as i64),
                ("woken", approvals.woken.len() as i64),
                ("rejected", approvals.rejected.len() as i64),
                ("deferred", deferred.len() as i64),
                ("pending", approvals.pending),
            ],
            message: approvals.warnings.join("\n"),
        },
    ];

    // Pick up every record appended this run — including this run's own idfix, verify_tail and
    // detect_external writes, which landed after the cache was filled.
    journal.invalidate();
    let journal_records = journal
        .read(None, None)
        .iter()
        .filter(|r| record_str(r, "run_id") == run_id)
        .count() as i64;
    let step_records: Vec<crate::ledger::Record> = steps.iter().map(step_record).collect();
    let end = end_run(
        vault,
        &run_id,
        status,
        &summary,
        journal_records,
        None,
        git_sha(vault).as_deref(),
        &step_records,
        Some(runner),
    );
    // The log line is rendered FROM the record (F11), and `now` is the vault-local clock the run
    // started on, not the wall clock at the moment of writing.
    let _ = append_run_log(
        vault,
        &record_str(&end, "runner"),
        &record_str(&end, "result"),
        &record_str(&end, "summary"),
        Some(now),
    );

    Ok(RunOutcome { output, status, summary, steps, not_ported, run_id })
}

/// Python: `f"{label}: {warnings[0]}" + (" (+N more)" if len > 1 else "")`.
fn first_with_count(label: &str, warnings: &[String]) -> Option<String> {
    let first = warnings.first()?;
    let extra = if warnings.len() > 1 {
        format!(" (+{} more)", warnings.len() - 1)
    } else {
        String::new()
    };
    Some(format!("{label}: {first}{extra}"))
}

fn count_prefix(log: &[String], prefix: &str) -> i64 {
    log.iter().filter(|l| l.starts_with(prefix)).count() as i64
}

/// The fifth space-separated token of a rendered log line: `- DATE TIME RUNNER STATUS ...`.
///
/// `split(" ")`, not `split_whitespace()` — Python splits on the single character, so a doubled
/// space shifts the token rather than being absorbed. That only matters for a line this module did
/// not write, and the point of classifying such a line at all is that it must not be silently
/// treated as `ok` and aged out early.
fn line_status(line: &str) -> &str {
    let parts: Vec<&str> = line.split(' ').collect();
    if parts.len() > 4 {
        parts[4]
    } else {
        ""
    }
}

/// Keep the newest 50 `ok` lines, but preserve up to the newest 100 non-`ok` lines.
///
/// A failure must not age out of the log while routine `ok` runs keep flowing. Selection is
/// **index-based**, not a set of strings, so identical lines all survive independently, and the
/// output keeps the original chronological order.
fn trim_log_lines(lines: &[String]) -> Vec<String> {
    let ok: Vec<usize> = (0..lines.len())
        .filter(|i| line_status(&lines[*i]) == "ok")
        .collect();
    let other: Vec<usize> = (0..lines.len())
        .filter(|i| line_status(&lines[*i]) != "ok")
        .collect();
    let mut keep: std::collections::BTreeSet<usize> = std::collections::BTreeSet::new();
    keep.extend(ok.iter().rev().take(50));
    keep.extend(other.iter().rev().take(100));
    (0..lines.len())
        .filter(|i| keep.contains(i))
        .map(|i| lines[i].clone())
        .collect()
}

/// Append one line to `state/runner-log.md`, trimming the file to the retention rule.
///
/// `when` defaults to the machine's local clock, not the vault's configured timezone — that is
/// what Python's bare `datetime.now()` does here, and the two differ whenever the vault is
/// configured for a zone the machine is not in.
///
/// The line itself is rendered by [`crate::runs::log_line`] and by nothing else (F11), which is
/// also what flattens a multi-line summary before it can break the file into lines
/// [`trim_log_lines`] would misclassify.
pub fn append_run_log(
    vault: &Path,
    runner: &str,
    status: &str,
    summary: &str,
    when: Option<DateTime>,
) -> std::io::Result<()> {
    let log_path = vault.join("state").join("runner-log.md");
    let stamp = when.unwrap_or_else(|| jiff::Zoned::now().datetime());
    let record = serde_json::json!({
        "runner": runner,
        "result": status,
        "summary": summary,
    });
    let line = crate::runs::log_line(&record, stamp);
    let mut lines: Vec<String> = Vec::new();
    if log_path.exists() {
        let text = pystr::read_text(&log_path)?;
        lines = pystr::splitlines(&text)
            .into_iter()
            .filter(|l| !pystr::strip(l).is_empty())
            .map(str::to_string)
            .collect();
    }
    lines.push(line);
    let trimmed = trim_log_lines(&lines);
    if let Some(parent) = log_path.parent() {
        std::fs::create_dir_all(parent)?;
    }
    pystr::write_text(&log_path, &(trimmed.join("\n") + "\n"))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn scratch(name: &str) -> PathBuf {
        let dir = std::env::temp_dir().join(format!(
            "qo-cli-{name}-{}-{:?}",
            std::process::id(),
            std::thread::current().id()
        ));
        let _ = std::fs::remove_dir_all(&dir);
        dir
    }

    fn copy_tree(src: &Path, dst: &Path) {
        std::fs::create_dir_all(dst).unwrap();
        for entry in std::fs::read_dir(src).unwrap().flatten() {
            let target = dst.join(entry.file_name());
            if entry.file_type().unwrap().is_dir() {
                copy_tree(&entry.path(), &target);
            } else {
                std::fs::copy(entry.path(), &target).unwrap();
            }
        }
    }

    fn fixture(name: &str, from: &str) -> PathBuf {
        let vault = scratch(name);
        copy_tree(Path::new(from), &vault);
        vault
    }

    // --- runner-log.md: the line, and the retention rule (plan Task 16 step 3, pulled forward
    // because coursework::main needs it) --------------------------------------------------------

    fn log_vault(name: &str) -> PathBuf {
        let vault = scratch(name);
        std::fs::create_dir_all(vault.join("state")).unwrap();
        vault
    }

    fn log_lines(vault: &Path) -> Vec<String> {
        let text = pystr::read_text(&vault.join("state").join("runner-log.md")).unwrap();
        pystr::splitlines(pystr::strip(&text))
            .into_iter()
            .map(str::to_string)
            .collect()
    }

    #[test]
    fn run_log_trims_ok_lines_to_newest_50_but_keeps_the_fail() {
        let vault = log_vault("logtrim");
        let seed: Vec<String> = (0..60)
            .map(|i| format!("- 2026-09-01 00:{i:02} manual ok old {i}"))
            .collect();
        pystr::write_text(
            &vault.join("state").join("runner-log.md"),
            &(seed.join("\n") + "\n"),
        )
        .unwrap();
        append_run_log(&vault, "cloud", "FAIL", "ingest: boom", None).unwrap();
        let lines = log_lines(&vault);
        // 50 newest ok lines survive the trim, plus the FAIL line itself.
        assert_eq!(lines.len(), 51);
        assert!(lines[lines.len() - 1].ends_with("cloud FAIL ingest: boom"));
        let _ = std::fs::remove_dir_all(&vault);
    }

    #[test]
    fn run_log_preserves_old_fail_line_across_days_of_ok_lines() {
        let vault = log_vault("logkeepfail");
        let mut seed = vec!["- 2026-09-01 00:00 cloud FAIL ingest: boom".to_string()];
        seed.extend((0..60).map(|i| format!("- 2026-09-01 01:{i:02} manual ok old {i}")));
        pystr::write_text(
            &vault.join("state").join("runner-log.md"),
            &(seed.join("\n") + "\n"),
        )
        .unwrap();
        append_run_log(&vault, "local", "ok", "today.md refreshed (3 active)", None).unwrap();
        let lines = log_lines(&vault);
        assert!(lines.iter().any(|l| l.ends_with("cloud FAIL ingest: boom")));
        // 50 newest ok lines (including the one just appended) + the FAIL.
        assert_eq!(lines.len(), 51);
        assert!(lines[lines.len() - 1].ends_with("today.md refreshed (3 active)"));
        let _ = std::fs::remove_dir_all(&vault);
    }

    #[test]
    fn append_run_log_flattens_a_multi_line_summary_to_one_line() {
        // F11: the line is rendered FROM the record by `runs::log_line`, and only log_line
        // collapses whitespace. A WARN carrying an embedded newline would otherwise write two
        // physical lines, the second of which `line_status` cannot classify as ok/WARN/FAIL and
        // which would then count forever as a non-ok line.
        let vault = log_vault("logflatten");
        append_run_log(
            &vault,
            "cloud",
            "WARN",
            "events pass failed (line1\nline2)",
            None,
        )
        .unwrap();
        let lines = log_lines(&vault);
        assert_eq!(lines.len(), 1);
        assert!(lines[0].contains("line1 line2"));
        assert_eq!(lines[0].split(' ').nth(4), Some("WARN"));
        let _ = std::fs::remove_dir_all(&vault);
    }

    #[test]
    fn the_trim_keeps_duplicate_lines_independently_and_in_order() {
        // Index-based selection, not a set of strings: three identical ok lines are three lines.
        let lines: Vec<String> = std::iter::repeat("- 2026-09-01 00:00 manual ok same".to_string())
            .take(3)
            .collect();
        assert_eq!(trim_log_lines(&lines), lines);
    }

    #[test]
    fn a_line_too_short_to_classify_counts_as_not_ok_and_is_retained() {
        // `line_status` returns "" for a line with five or fewer space-separated tokens, and ""
        // is not "ok" — so a stray line is kept under the 100-line rule rather than aged out with
        // the routine successes.
        assert_eq!(line_status("- 2026-09-01 00:00 manual"), "");
        assert_eq!(line_status("- 2026-09-01 00:00 manual ok x"), "ok");
        let mut lines: Vec<String> = vec!["not a log line at all".to_string()];
        lines.extend((0..60).map(|i| format!("- 2026-09-01 00:{i:02} manual ok old {i}")));
        let trimmed = trim_log_lines(&lines);
        assert_eq!(trimmed.len(), 51);
        assert_eq!(trimmed[0], "not a log line at all");
    }

    #[test]
    fn rank_writes_today_md_and_reports_the_step_sequence() {
        let vault = fixture("steps", "tests/fixtures/vault-s1");
        let out = run(&vault, Some("2026-08-28"), "manual", None).unwrap();
        assert!(out.output.is_file());

        let names: Vec<&str> = out.steps.iter().map(|s| s.name).collect();
        assert_eq!(names, vec!["passes", "tasks", "calendar", "events", "approvals"]);
        let tasks = out.steps.iter().find(|s| s.name == "tasks").unwrap();
        // `load_tasks` drops `status: done`, so the fixture's six notes count as five active —
        // the same five the golden file's header reports.
        assert_eq!(count(&tasks.counts, "active"), 5);
        assert_eq!(count(&tasks.counts, "unreadable"), 0);

        // Plan 2 Task 7 (S2 §7.7, F6): the three gauge counts, recorded from the first
        // run. The gauge cannot backfill a trend, so a run that does not write them loses that
        // day forever. Recomputed here through `surface::load` — the loader the page itself
        // uses, and one that never fetches — rather than hard-coding whatever this fixture
        // happens to produce today.
        let day = jiff::civil::Date::constant(2026, 8, 28);
        let l = crate::surface::load(&vault, day);
        let (must, _) = crate::ranking::partition_must_do(&l.ranked, day, &l.cal);
        assert_eq!(
            count(&tasks.counts, "must_count"),
            must.len() as i64,
            "must_count is the must-do partition's size"
        );
        assert!(tasks.counts.iter().any(|(k, _)| *k == "runway_days"));
        assert!(count(&tasks.counts, "deficit_hours_x10") >= 0);
        // The count agrees with the topline's integer, or the gauge and the header disagree
        // forever — one page showing two runway figures is worse than showing none.
        let tl = crate::surface::topline(&vault, &l, day);
        assert_eq!(count(&tasks.counts, "runway_days"), tl.runway_days.round() as i64);

        let _ = std::fs::remove_dir_all(&vault);
    }

    /// Task 23 step 2 (waves 0–3), then Task 17 step 2 (wave 7): a step this crate cannot run
    /// must be **named**, not silently absent — and now none is. This test failed on purpose four
    /// times, once per ported step (calfeed in wave 4, the events pass in wave 5, `runs` in wave
    /// 7 Task 14, `info_pass` in Task 15). It is the structural statement that the port is
    /// complete; if a step is ever taken out of `run`, it must be pushed back onto the list and
    /// this expectation updated, never deleted.
    #[test]
    fn every_unported_step_is_named_rather_than_silently_skipped() {
        let vault = fixture("notported", "tests/fixtures/vault-s1");
        let out = run(&vault, Some("2026-08-28"), "manual", None).unwrap();
        assert_eq!(out.not_ported, Vec::<&str>::new());
        let _ = std::fs::remove_dir_all(&vault);
    }

    /// `info_pass` runs inside `rank` (Task 15), and the oracle's blind spot is named honestly:
    /// no fixture vault holds an `info/` note, so the golden never exercised the pass. Here one
    /// does — an expired item is settled into `archive/` by `system:expiry`, one expiring today
    /// survives (exclusive of today, like approvals) — and **the page is still the golden**,
    /// because nothing in `render` reads `info/`. The pinned date makes the comparison exact.
    #[test]
    fn info_pass_runs_inside_rank_and_the_page_is_still_the_golden() {
        use crate::info::{list_info, open_info, NewInfo};
        let vault = fixture("infopass", "tests/fixtures/vault-s1-migrated");
        let ctx = WriteContext::new("quinn", "cli");
        let stamp: Timestamp = "2026-08-27T12:00:00Z".parse().unwrap();
        for (title, expires) in [("Old", "2026-08-27"), ("Today", "2026-08-28")] {
            let item = NewInfo {
                title,
                kind: "notice",
                body: "",
                opened_by: "quinn",
                close_key: None,
                expires: Some(expires.parse().unwrap()),
            };
            open_info(&vault, &item, &ctx, None, Some(stamp)).unwrap();
        }
        let out = run(&vault, Some("2026-08-28"), "manual", None).unwrap();
        assert_eq!(out.status, "ok", "{}", out.summary);

        let open: Vec<String> = list_info(&vault)
            .iter()
            .map(|i| crate::yaml::get(i, "title").map(pystr::yaml_str).unwrap_or_default())
            .collect();
        assert_eq!(open, vec!["Today"]);
        let archived = crate::approvals::sorted_md(&vault.join("archive"));
        let expired: Vec<PathBuf> = archived
            .into_iter()
            .filter(|p| p.file_name().is_some_and(|n| n.to_string_lossy().starts_with("info_")))
            .collect();
        assert_eq!(expired.len(), 1);
        let meta = crate::ids::read_meta(&expired[0]).unwrap();
        assert_eq!(crate::yaml::get(&meta, "closed_by").map(pystr::yaml_str).as_deref(), Some("system:expiry"));

        let strip_footer = |text: &str| -> String {
            text.lines().filter(|l| !l.starts_with("*Generated ")).collect::<Vec<_>>().join("\n")
        };
        let page = pystr::read_text(&out.output).unwrap();
        let golden = pystr::read_text(Path::new("tests/fixtures/golden-today-s1.md")).unwrap();
        assert_eq!(strip_footer(&page), strip_footer(&golden));
        let _ = std::fs::remove_dir_all(&vault);
    }

    /// An enabled events config whose feed is unreachable must warn and still render.
    ///
    /// This test used to assert `events pass not ported`. The pass IS ported now (wave 5), so the
    /// same shape exercises the real thing: every feed fails, `load_discovered_events` warns,
    /// `cli::run` falls back to `read_roster` (empty here), and a page is still written.
    ///
    /// The URL uses an unknown scheme rather than a real host: it fails in the URL parser without
    /// opening a socket, so the test stays offline and instant. A `.invalid` hostname would still
    /// cost a DNS lookup.
    #[test]
    fn an_unreachable_events_feed_warns_and_the_run_still_renders() {
        let vault = fixture("eventswarn", "tests/fixtures/vault-s1");
        pystr::write_text(
            &vault.join("config").join("events.yaml"),
            "sources:
  - name: blount
    type: ics
    url: unreachable://x.ics
    enabled: true
",
        )
        .unwrap();
        let out = run(&vault, Some("2026-08-28"), "manual", None).unwrap();
        assert_eq!(out.status, "WARN");
        assert!(out.summary.contains("blount: fetch failed"), "{}", out.summary);
        assert!(!out.not_ported.iter().any(|s| s.contains("events pass")));
        assert!(vault.join("state").join("today.md").exists());
        assert!(vault.join("state").join("events.md").exists(), "the roster must still be written");
        let _ = std::fs::remove_dir_all(&vault);
    }

    /// A `calendars:` entry that is a bare string, not a mapping, must warn and carry on.
    ///
    /// This test used to assert `calfeed not ported`. calfeed IS ported now (wave 4), so the
    /// same config exercises the real thing: `load_calendar_events` cannot call `.get` on a
    /// string, warns `calendar: bad feed entry (...)`, falls back to the snapshot for that feed,
    /// and the run still produces a page. Never-crash, preserved from the Python.
    #[test]
    fn a_malformed_calendars_entry_warns_and_the_run_still_renders() {
        let vault = fixture("calwarn", "tests/fixtures/vault-s1");
        pystr::write_text(
            &vault.join("config").join("ingest.yaml"),
            "timezone: America/Chicago
calendars:
  - https://example.invalid/private.ics
",
        )
        .unwrap();
        let out = run(&vault, Some("2026-08-28"), "manual", None).unwrap();
        assert_eq!(out.status, "WARN");
        assert!(out.summary.contains("bad feed entry"), "{}", out.summary);
        assert!(!out.not_ported.iter().any(|s| s.contains("calfeed")));
        assert!(vault.join("state").join("today.md").exists());
        let _ = std::fs::remove_dir_all(&vault);
    }

    #[test]
    fn a_malformed_run_id_is_refused_before_anything_is_written() {
        let vault = fixture("badrunid", "tests/fixtures/vault-s1");
        let err = run(&vault, Some("2026-08-28"), "manual", Some("nope")).unwrap_err();
        assert!(matches!(err, RunError::BadRunId(_)));
        // argparse's `parser.error`: exit 2, and nothing on disk — not even a start record.
        // Measured against `python -m engine.cli` for both refusals (Task 17 sweep).
        assert_eq!(err.exit_code(), 2);
        let bad_date = run(&vault, Some("2026-13-45"), "manual", None).unwrap_err();
        assert!(matches!(bad_date, RunError::BadDate(_)));
        assert_eq!(bad_date.exit_code(), 2);
        assert!(!vault.join("state").exists(), "no start record, no page");
        assert_eq!(RunError::Io("x".into()).exit_code(), 1);
        assert!(!vault.join("state").join("today.md").exists());
        assert!(run(&vault, Some("2026-08-28"), "manual", Some("local-2026-08-29T17:00:00Z")).is_ok());
        let _ = std::fs::remove_dir_all(&vault);
    }

    /// The migrated fixture is the vault Quinn actually runs. Two things must hold at once: the
    /// page renders, and the passes are SILENT — no external edits invented from values the
    /// journal already knows, and no unmigrated-vault refusal.
    #[test]
    fn the_migrated_fixture_runs_without_inventing_a_single_external_edit() {
        let vault = fixture("migrated", "tests/fixtures/vault-s1-migrated");
        let out = run(&vault, Some("2026-08-28"), "manual", None).unwrap();
        let passes = out.steps.iter().find(|s| s.name == "passes").unwrap();
        assert_eq!(count(&passes.counts, "external"), 0, "{}", passes.message);
        assert_eq!(count(&passes.counts, "unmigrated"), 0, "{}", passes.message);
        assert!(!passes.message.contains("migration records"), "{}", passes.message);
        let _ = std::fs::remove_dir_all(&vault);
    }

    /// The un-migrated fixture must trip the guard rather than journal its whole vault as human
    /// writes. This is the other half of why the oracle runs on both fixtures.
    #[test]
    fn the_unmigrated_fixture_trips_the_guard_and_still_renders() {
        let vault = fixture("unmigrated", "tests/fixtures/vault-s1");
        let out = run(&vault, Some("2026-08-28"), "manual", None).unwrap();
        let passes = out.steps.iter().find(|s| s.name == "passes").unwrap();
        assert_eq!(count(&passes.counts, "unmigrated"), 1, "{}", passes.message);
        assert_eq!(count(&passes.counts, "external"), 0);
        assert!(out.output.is_file(), "the page still renders");
        let _ = std::fs::remove_dir_all(&vault);
    }

    // -----------------------------------------------------------------------
    // local_now
    // -----------------------------------------------------------------------

    /// A vault built the way `test_cli.py::make_vault` builds one: the week template (the fixture
    /// copy — this crate's root is not a vault), an empty `tasks/`, nothing else.
    fn bare_vault(name: &str) -> PathBuf {
        let vault = scratch(name);
        std::fs::create_dir_all(vault.join("config")).unwrap();
        std::fs::create_dir_all(vault.join("tasks")).unwrap();
        std::fs::copy(
            "tests/fixtures/vault-s1/config/week_template.yaml",
            vault.join("config").join("week_template.yaml"),
        )
        .unwrap();
        vault
    }

    /// Give the vault one real `create` record so `detect_external`'s unmigrated guard does not
    /// fire — hand-written fixture notes otherwise look exactly like a vault that has never been
    /// through the migration. Archived, and out of every other pass's way.
    fn seed_migrated(vault: &Path) {
        std::fs::create_dir_all(vault.join("archive")).unwrap();
        crate::write::create(
            vault,
            "archive/_migrated.md",
            "---\ntitle: migrated\nstatus: archived\n---\n",
            &WriteContext::new("system:migration", "cli"),
            &mut Journal::new(vault),
            None,
        )
        .unwrap();
    }

    fn seconds_apart(a: DateTime, b: DateTime) -> f64 {
        a.duration_until(b).as_secs_f64().abs()
    }

    #[test]
    fn local_now_uses_the_configured_timezone() {
        let vault = bare_vault("tz");
        pystr::write_text(&vault.join("config").join("ingest.yaml"), "timezone: America/Chicago\n")
            .unwrap();
        let expected = Timestamp::now().to_zoned(TimeZone::get("America/Chicago").unwrap()).datetime();
        assert!(seconds_apart(local_now(&vault), expected) < 5.0);
        let _ = std::fs::remove_dir_all(&vault);
    }

    /// A missing or malformed config must never cost a run — Python swallows every exception here
    /// on purpose, and a bad timezone string is exactly the shape that would otherwise crash it.
    #[test]
    fn local_now_falls_back_cleanly_when_the_config_is_absent_or_malformed() {
        let vault = bare_vault("tzfallback");
        let expected = jiff::Zoned::now().datetime();
        assert!(seconds_apart(local_now(&vault), expected) < 5.0, "absent config");

        pystr::write_text(&vault.join("config").join("ingest.yaml"), "ics_url: [unclosed\n").unwrap();
        assert!(seconds_apart(local_now(&vault), expected) < 5.0, "malformed config");

        pystr::write_text(&vault.join("config").join("ingest.yaml"), "timezone: Not/AZone\n").unwrap();
        assert!(seconds_apart(local_now(&vault), expected) < 5.0, "unknown zone");
        let _ = std::fs::remove_dir_all(&vault);
    }

    // -----------------------------------------------------------------------
    // The rest of the run
    // -----------------------------------------------------------------------

    fn page(vault: &Path) -> String {
        pystr::read_text(&vault.join("state").join("today.md")).unwrap()
    }

    #[test]
    fn run_reads_the_vaults_planning_config() {
        let vault = bare_vault("planning");
        pystr::write_text(
            &vault.join("config").join("planning.yaml"),
            "recurring:\n  - name: German practice\n    hours: 0.75\n    days: [mon]\n",
        )
        .unwrap();
        pystr::write_text(
            &vault.join("tasks").join("a.md"),
            "---\ntitle: Test task\ndue: 2026-09-25\neffort_hours: 6\nimportance: 3\n---\n",
        )
        .unwrap();
        run(&vault, Some("2026-09-07"), "manual", None).unwrap();
        let out = page(&vault);
        assert!(out.contains("**German practice** \u{2014} 0.75h \u{b7} recurring"), "{out}");
        // A 6h task at the default 2h slice: today's take is 2h of 6h.
        assert!(out.contains("(of 6.0h left)"), "{out}");
        let _ = std::fs::remove_dir_all(&vault);
    }

    #[test]
    fn an_unreadable_note_is_counted_and_named_in_the_summary() {
        let vault = bare_vault("unreadable");
        seed_migrated(&vault);
        pystr::write_text(
            &vault.join("tasks").join("good.md"),
            "---\ntitle: Good\ndue: 2026-09-11\neffort_hours: 1\nimportance: 3\n---\n",
        )
        .unwrap();
        pystr::write_text(
            &vault.join("tasks").join("bad.md"),
            "---\ntitle: Bad\ndue: [unclosed\nstatus: active\n---\n",
        )
        .unwrap();
        let out = run(&vault, Some("2026-09-07"), "manual", None).unwrap();
        assert_eq!(out.status, "WARN");
        assert!(out.summary.contains("1 unreadable: bad.md"), "{}", out.summary);
        let tasks = out.steps.iter().find(|s| s.name == "tasks").unwrap();
        assert_eq!(tasks.result, "WARN");
        assert!(tasks.message.contains("unreadable"), "the full text lives in the run record");
        let _ = std::fs::remove_dir_all(&vault);
    }

    /// Spec §6: "note without id / duplicate id — fixed at run start, journaled, WARN once".
    /// A copied note (Obsidian's "Make a copy") carries its source's id; idfix silently re-ids it,
    /// and a silent repair of an *identity* is exactly what the run record must not swallow.
    #[test]
    fn a_duplicate_id_is_repaired_and_counted_in_the_passes_step() {
        let vault = bare_vault("dupes");
        seed_migrated(&vault);
        for name in ["a.md", "b.md"] {
            pystr::write_text(
                &vault.join("tasks").join(name),
                "---\ntitle: T\nid: task_abcdef0123\ndue: 2026-09-11\neffort_hours: 2\nimportance: 3\n---\n",
            )
            .unwrap();
        }
        let out = run(&vault, Some("2026-09-07"), "local", None).unwrap();
        let passes = out.steps.iter().find(|s| s.name == "passes").unwrap();
        assert_eq!(passes.result, "WARN", "{}", passes.message);
        assert_eq!(count(&passes.counts, "duplicate"), 1, "{}", passes.message);
        assert_eq!(out.status, "WARN");
        assert!(out.summary.contains("1 duplicate"), "{}", out.summary);
        let _ = std::fs::remove_dir_all(&vault);
    }

    /// Spec §2.3/§6: the record is still honoured — later-timestamp-wins needs a total order — but
    /// an unreported skew hands a fast-clocked device every conflict it takes part in, silently.
    #[test]
    fn a_record_from_the_future_warns_and_names_the_device() {
        let vault = bare_vault("skew");
        seed_migrated(&vault);
        pystr::write_text(
            &vault.join("tasks").join("a.md"),
            "---\ntitle: T\ndue: 2026-09-11\neffort_hours: 2\nimportance: 3\n---\n",
        )
        .unwrap();
        let ahead = Timestamp::now() + jiff::SignedDuration::from_hours(3);
        let mut spec = crate::journal::NewRecord::new("set", "tasks/a.md", "quinn", "dashboard");
        spec.id = Some("task_abcdef0123");
        spec.field = Some("progress");
        spec.old = serde_json::json!(0);
        spec.new = serde_json::json!(50);
        spec.ts = Some(crate::journal::now_ts(Some(ahead)));
        spec.device = Some("laptop".into());
        let mut rec = crate::journal::make_record(spec).unwrap();
        Journal::new(&vault).append(&mut rec).unwrap();

        let out = run(&vault, Some("2026-09-07"), "local", None).unwrap();
        let passes = out.steps.iter().find(|s| s.name == "passes").unwrap();
        assert_eq!(count(&passes.counts, "future"), 1);
        assert_eq!(out.status, "WARN");
        assert!(
            out.summary.contains("1 record(s) from the future (laptop)"),
            "{}",
            out.summary
        );
        let _ = std::fs::remove_dir_all(&vault);
    }

    #[test]
    fn run_assigns_ids_and_journals_them_as_system_idfix() {
        let vault = bare_vault("idfix");
        pystr::write_text(
            &vault.join("tasks").join("a.md"),
            "---\ntitle: T\ndue: 2026-09-11\neffort_hours: 2\nimportance: 3\n---\n",
        )
        .unwrap();
        run(&vault, Some("2026-09-07"), "manual", None).unwrap();
        let text = pystr::read_text(&vault.join("tasks").join("a.md")).unwrap();
        assert!(text.contains("\nid: task_"), "{text}");

        let records = Journal::new(&vault).read(None, None);
        assert_eq!(records[0].get("actor"), Some(&serde_json::json!("system:idfix")));
        assert_eq!(records[0].get("field"), Some(&serde_json::json!("id")));
        let _ = std::fs::remove_dir_all(&vault);
    }

    /// A caller-supplied run id is carried onto every record the run writes, so the whole run is
    /// attributable to the process that started it.
    #[test]
    fn an_external_run_id_is_accepted_and_stamped_on_the_runs_records() {
        let vault = bare_vault("runid");
        pystr::write_text(
            &vault.join("tasks").join("a.md"),
            "---\ntitle: T\ndue: 2026-09-11\neffort_hours: 2\nimportance: 3\n---\n",
        )
        .unwrap();
        let out =
            run(&vault, Some("2026-09-07"), "cloud", Some("cloud-2026-09-07T13:00:00Z")).unwrap();
        assert_eq!(out.run_id, "cloud-2026-09-07T13:00:00Z");
        let records = Journal::new(&vault).read(None, None);
        assert!(!records.is_empty());
        assert!(records
            .iter()
            .all(|r| r.get("run_id") == Some(&serde_json::json!("cloud-2026-09-07T13:00:00Z"))));
        let _ = std::fs::remove_dir_all(&vault);
    }

    // -----------------------------------------------------------------------
    // The rest of test_cli.py (Task 16 step 1). Names kept where the Python has one; the four
    // that monkeypatch a fetcher go through `run_with`, the two that stub or crash a function go
    // through a filesystem stand-in explained on the test.
    // -----------------------------------------------------------------------

    fn runner_log(vault: &Path) -> String {
        pystr::read_text(&vault.join("state").join("runner-log.md")).unwrap()
    }

    fn run_records(vault: &Path) -> Vec<crate::ledger::Record> {
        crate::runs::Runs::new(vault).read(None)
    }

    fn step_of<'a>(end: &'a crate::ledger::Record, name: &str) -> &'a serde_json::Value {
        end.get("steps")
            .and_then(|s| s.as_array())
            .and_then(|steps| steps.iter().find(|s| s.get("name") == Some(&serde_json::json!(name))))
            .unwrap_or_else(|| panic!("no {name} step in {end:?}"))
    }

    fn md_names(dir: &Path, prefix: &str) -> Vec<String> {
        let mut names: Vec<String> = std::fs::read_dir(dir)
            .map(|rd| {
                rd.flatten()
                    .map(|e| e.file_name().to_string_lossy().to_string())
                    .filter(|n| n.starts_with(prefix) && n.ends_with(".md"))
                    .collect()
            })
            .unwrap_or_default();
        names.sort();
        names
    }

    fn meta_str(path: &Path, key: &str) -> String {
        let meta = crate::ids::read_meta(path).unwrap();
        crate::yaml::get(&meta, key).map(pystr::yaml_str).unwrap_or_default()
    }

    #[test]
    fn run_appends_ok_line_to_runner_log() {
        let vault = bare_vault("oklog");
        run(&vault, Some("2026-09-07"), "local", None).unwrap();
        let log = runner_log(&vault);
        assert!(log.contains(" local ok "), "{log}");
        let _ = std::fs::remove_dir_all(&vault);
    }

    const ONE_FEED: &str = "timezone: America/Chicago\ncalendars:\n  - name: personal\n    ics_url: unreachable://example.test/cal.ics\n";

    /// Python monkeypatches `calfeed.fetch_ics`; here the feed arrives through `run_with`. The
    /// url's scheme is unreachable on purpose: if the seam were ever bypassed, the real fetcher
    /// fails in its URL parser and this test goes red offline rather than touching the network.
    #[test]
    fn run_subtracts_calendar_events() {
        let vault = bare_vault("calsub");
        pystr::write_text(&vault.join("config").join("ingest.yaml"), ONE_FEED).unwrap();
        let ics = "BEGIN:VCALENDAR\nBEGIN:VEVENT\nUID:x\nSUMMARY:Client call\n\
                   DTSTART;TZID=America/Chicago:20260907T090000\n\
                   DTEND;TZID=America/Chicago:20260907T093000\nEND:VEVENT\nEND:VCALENDAR\n";
        let fetch = |_: &str| Ok(ics.to_string());
        pystr::write_text(
            &vault.join("tasks").join("a.md"),
            "---\ntitle: Test task\ndue: 2026-09-25\neffort_hours: 2\nimportance: 3\n---\n",
        )
        .unwrap();
        let fetchers = Fetchers { calendar: Some(&fetch), events: None };
        run_with(&vault, Some("2026-09-07"), "manual", None, fetchers).unwrap();
        let out = page(&vault);
        assert!(out.contains("- 09:00\u{2013}09:30 Client call"), "{out}");
        assert!(
            out.contains("Capacity today: 7.67h (template 8.17h \u{2212} 0.5h calendar)"),
            "{out}"
        );
        let snapshot = pystr::read_text(&vault.join("state").join("calendar.md")).unwrap();
        assert!(
            snapshot.contains("- personal \u{b7} 2026-09-07 09:00\u{2013}09:30 \u{b7} Client call"),
            "{snapshot}"
        );
        let _ = std::fs::remove_dir_all(&vault);
    }

    #[test]
    fn run_falls_back_to_snapshot_and_warns() {
        let vault = bare_vault("calsnap");
        pystr::write_text(&vault.join("config").join("ingest.yaml"), ONE_FEED).unwrap();
        std::fs::create_dir_all(vault.join("state")).unwrap();
        pystr::write_text(
            &vault.join("state").join("calendar.md"),
            "# snap\n- personal \u{b7} 2026-09-07 09:00\u{2013}09:30 \u{b7} Client call\n",
        )
        .unwrap();
        // At least one active task, or the renderer takes its "Nothing active" early-return and
        // no Schedule section is emitted.
        pystr::write_text(
            &vault.join("tasks").join("a.md"),
            "---\ntitle: Test task\ndue: 2026-09-25\neffort_hours: 2\nimportance: 3\n---\n",
        )
        .unwrap();
        let boom = |_: &str| Err("tunnel closed".to_string());
        let fetchers = Fetchers { calendar: Some(&boom), events: None };
        run_with(&vault, Some("2026-09-07"), "local", None, fetchers).unwrap();
        let out = page(&vault);
        assert!(out.contains("- 09:00\u{2013}09:30 Client call"), "{out}"); // snapshot kept the event
        let log = runner_log(&vault);
        assert!(log.contains(" local WARN "), "{log}");
        assert!(log.contains("calendar: personal: fetch failed"), "{log}");
        let _ = std::fs::remove_dir_all(&vault);
    }

    const APPROVED_PROPOSAL: &str = "---
type: approval
kind: task
title: Study group kickoff
status: approved
source_uid: \"gmail:abc\"
proposed_at: 2026-09-05
expires: 2026-09-12
snooze_until: null
created_by: gmail
---

**Why proposed:** borderline.

```task
---
title: Study group kickoff
course: null
domain: school
due: 2026-09-11
effort_hours: 1.0
effort_confidence: low
effort_source: inferred
importance: 3
importance_reason: \"attendance helps\"
status: active
progress: 0
created_by: gmail
source_uid: \"gmail:abc\"
---

Bring questions.
```
";

    #[test]
    fn approved_task_ranks_in_same_run() {
        let vault = bare_vault("approved");
        seed_migrated(&vault);
        std::fs::create_dir_all(vault.join("approvals")).unwrap();
        pystr::write_text(&vault.join("approvals").join("task-study-group.md"), APPROVED_PROPOSAL)
            .unwrap();
        run(&vault, Some("2026-09-07"), "local", None).unwrap();
        let today = page(&vault);
        assert!(today.contains("Study group kickoff"), "{today}");
        assert!(!vault.join("approvals").join("task-study-group.md").exists());
        assert!(vault.join("archive").join("task-study-group.md").exists());
        let log = runner_log(&vault);
        assert!(log.contains("approvals: 1 executed"), "{log}");
        assert!(log.contains(" local ok "), "{log}");
        let _ = std::fs::remove_dir_all(&vault);
    }

    #[test]
    fn approvals_warning_produces_warn_run() {
        let vault = bare_vault("apprwarn");
        std::fs::create_dir_all(vault.join("approvals")).unwrap();
        pystr::write_text(
            &vault.join("approvals").join("task-bad.md"),
            "---\ntitle: [unclosed\nstatus: pending\n---\n",
        )
        .unwrap();
        run(&vault, Some("2026-09-07"), "local", None).unwrap();
        let log = runner_log(&vault);
        assert!(log.contains(" local WARN "), "{log}");
        assert!(log.contains("approvals: unreadable: task-bad.md"), "{log}");
        let _ = std::fs::remove_dir_all(&vault);
    }

    // --- the joint daily budget, through a real run ---------------------------------------------

    /// `test_cli.py::scaffold`: a flat week template and one enabled event source. The source url
    /// is unreachable for the same reason as the calendar one above.
    fn scaffold(name: &str) -> PathBuf {
        let vault = scratch(name);
        std::fs::create_dir_all(vault.join("config")).unwrap();
        std::fs::create_dir_all(vault.join("tasks")).unwrap();
        pystr::write_text(
            &vault.join("config").join("week_template.yaml"),
            "day_start: '09:00'\nday_end: '17:00'\nclasses:\n  mon: []\n  tue: []\n  wed: []\n  thu: []\n  fri: []\n  sat: []\n  sun: []\n",
        )
        .unwrap();
        pystr::write_text(
            &vault.join("config").join("events.yaml"),
            "sources:\n  - name: blount\n    type: ics\n    url: unreachable://x\n    enabled: true\n",
        )
        .unwrap();
        vault
    }

    /// `n` half-hour events on 2026-08-27 from 18:00, `ics:ev-0` first — inside the horizon and
    /// the urgency window from 2026-08-26, sharing strength and deadline so start order decides.
    fn judged_feed(vault: &Path, n: usize) -> String {
        let mut blocks: Vec<String> = Vec::new();
        for i in 0..n {
            let hour = 18 + i;
            blocks.push(format!(
                "BEGIN:VEVENT\nDTSTART;TZID=America/Chicago:20260827T{hour:02}0000\n\
                 DTEND;TZID=America/Chicago:20260827T{hour:02}3000\n\
                 UID:ev-{i}\nSUMMARY:Event {i}\nLOCATION:Union\nEND:VEVENT"
            ));
            crate::eventledger::record_verdict(
                vault,
                &format!("ics:ev-{i}"),
                &format!("Event {i}"),
                Date::constant(2026, 8, 26),
                "opportunity",
                "",
                "",
                "",
            )
            .unwrap();
        }
        format!("BEGIN:VCALENDAR\n{}\nEND:VCALENDAR\n", blocks.join("\n"))
    }

    fn pending_proposals(vault: &Path, n: usize) {
        std::fs::create_dir_all(vault.join("approvals")).unwrap();
        for i in 0..n {
            pystr::write_text(
                &vault.join("approvals").join(format!("task-{i}.md")),
                "---\ntype: approval\nkind: task\ntitle: t\nstatus: pending\nproposed_at: 2026-08-26\nexpires: 2026-09-30\nsnooze_until: null\n---\n\nb\n",
            )
            .unwrap();
        }
    }

    fn digest_uids(vault: &Path) -> Vec<String> {
        let digests = md_names(&vault.join("approvals"), "events-digest-");
        assert_eq!(digests.len(), 1, "{digests:?}");
        let meta = crate::ids::read_meta(&vault.join("approvals").join(&digests[0])).unwrap();
        crate::yaml::get(&meta, "events")
            .and_then(|v| v.as_sequence())
            .map(|entries| {
                entries
                    .iter()
                    .map(|e| e.get("uid").map(pystr::yaml_str).unwrap_or_default())
                    .collect()
            })
            .unwrap_or_default()
    }

    /// A run that already spent 14 of 15 emits at most one event.
    ///
    /// Python stubs `emit_digest` and reads the `budget` it was handed. There is nothing to stub
    /// here, so the real `emit_digest` runs over a two-event judged feed and the digest it writes
    /// is the measurement: budget 1 means exactly one of the two lands (measured against the
    /// Python engine with the same fixture, 2026-09-02).
    #[test]
    fn gmail_proposals_shrink_the_same_days_digest() {
        let vault = scaffold("shrink");
        pystr::write_text(&vault.join("config").join("planning.yaml"), "daily_approval_budget: 15\n")
            .unwrap();
        pending_proposals(&vault, 14);
        let ics = judged_feed(&vault, 2);
        let fetch = |_: &str| Ok(ics.clone());
        let fetchers = Fetchers { calendar: None, events: Some(&fetch) };
        run_with(&vault, Some("2026-08-26"), "manual", None, fetchers).unwrap();
        assert_eq!(digest_uids(&vault), vec!["ics:ev-0".to_string()]);
        let _ = std::fs::remove_dir_all(&vault);
    }

    /// Diagnostic for the `defer_over_budget` wiring itself, not just the arithmetic: a NON-default
    /// budget (3) with MORE same-day pending proposals (5) than the budget, so deferral is actually
    /// forced to act. If `run` stopped calling `defer_over_budget`, none of these files would ever
    /// flip to snoozed; if it discarded the return value, `2 deferred` would leave the log line.
    /// The digest half is the same stand-in as above: one eligible event, a budget of 0 once the
    /// ceiling is enforced, so no digest may be written.
    #[test]
    fn defer_over_budget_runs_inside_cli_and_shrinks_the_digest() {
        let vault = scaffold("defer");
        pystr::write_text(&vault.join("config").join("planning.yaml"), "daily_approval_budget: 3\n")
            .unwrap();
        pending_proposals(&vault, 5);
        let ics = judged_feed(&vault, 1);
        let fetch = |_: &str| Ok(ics.clone());
        let fetchers = Fetchers { calendar: None, events: Some(&fetch) };
        run_with(&vault, Some("2026-08-26"), "manual", None, fetchers).unwrap();

        // Sorted by (expires, stem) with all five sharing one expiry date, the allowance of 3
        // keeps task-0..task-2 and defers the tail: task-3, task-4.
        for stem in ["task-0", "task-1", "task-2"] {
            let path = vault.join("approvals").join(format!("{stem}.md"));
            assert_eq!(meta_str(&path, "status"), "pending", "{stem}");
        }
        for stem in ["task-3", "task-4"] {
            let path = vault.join("approvals").join(format!("{stem}.md"));
            assert_eq!(meta_str(&path, "status"), "snoozed", "{stem}");
            assert_eq!(meta_str(&path, "snooze_until"), "2026-08-27", "{stem}");
        }
        // The digest sees only what's left after the ceiling is enforced: 3 non-digest proposals
        // were created today against a budget of 3, so nothing is emitted.
        assert!(md_names(&vault.join("approvals"), "events-digest-").is_empty());
        // The return value must reach the run log — the cloud routine's prompt says a miscount is
        // "safe, not silent" because the engine reports what it snoozes.
        let log = runner_log(&vault);
        assert!(log.contains("2 deferred"), "{log}");
        let _ = std::fs::remove_dir_all(&vault);
    }

    /// Spec §5's headline case through a real run with a real `emit_digest`: 5 Gmail-style
    /// proposals plus a 10-event digest exactly exhausts a 15/day budget, and the 11th eligible
    /// event is left out of today's digest.
    #[test]
    fn gmail_and_event_digest_share_one_joint_daily_budget() {
        let vault = bare_vault("joint");
        pystr::write_text(
            &vault.join("config").join("events.yaml"),
            "sources:\n  - name: campus\n    type: ics\n    url: unreachable://x\n    enabled: true\n",
        )
        .unwrap();
        pystr::write_text(&vault.join("config").join("planning.yaml"), "daily_approval_budget: 15\n")
            .unwrap();
        // 5 Gmail-style proposal notes already created today: they alone spend a third of the
        // day's ceiling before the events pass ever runs.
        std::fs::create_dir_all(vault.join("approvals")).unwrap();
        for i in 0..5 {
            pystr::write_text(
                &vault.join("approvals").join(format!("gmail-{i}.md")),
                &format!(
                    "---\ntype: approval\nkind: task\ntitle: Reply to advisor\nstatus: pending\n\
                     proposed_at: 2026-08-26\nexpires: 2026-09-30\nsnooze_until: null\n\
                     created_by: gmail\nsource_uid: \"gmail:msg-{i}\"\n---\n\nbody\n"
                ),
            )
            .unwrap();
        }
        // 11 judged-opportunity events, ev-0 (earliest start) .. ev-10 (latest), so selection
        // order is deterministic: the sort key is (strength, deadline, start, uid) and every one
        // shares the same strength ("") and deadline (none).
        let mut blocks: Vec<String> = Vec::new();
        for i in 0..11 {
            let hour = 8 + i;
            blocks.push(format!(
                "BEGIN:VEVENT\nDTSTART;TZID=America/Chicago:20260827T{hour:02}0000\n\
                 DTEND;TZID=America/Chicago:20260827T{hour:02}3000\n\
                 UID:ev-{i}\nSUMMARY:Event {i}\nLOCATION:Union\nEND:VEVENT"
            ));
            crate::eventledger::record_verdict(
                &vault,
                &format!("ics:ev-{i}"),
                &format!("Event {i}"),
                Date::constant(2026, 8, 26),
                "opportunity",
                "",
                "",
                "",
            )
            .unwrap();
        }
        let ics = format!("BEGIN:VCALENDAR\n{}\nEND:VCALENDAR\n", blocks.join("\n"));
        let fetch = |_: &str| Ok(ics.clone());
        let fetchers = Fetchers { calendar: None, events: Some(&fetch) };
        run_with(&vault, Some("2026-08-26"), "manual", None, fetchers).unwrap();

        // remaining budget = 15 - 5 Gmail proposals = 10; exactly 10 of the 11 eligible events
        // land in the digest, and the latest-starting one (ev-10) is the one left out.
        let chosen = digest_uids(&vault);
        assert_eq!(chosen.len(), 10, "{chosen:?}");
        assert!(!chosen.iter().any(|u| u == "ics:ev-10"), "{chosen:?}");
        assert!(chosen.iter().any(|u| u == "ics:ev-9"), "{chosen:?}");
        let _ = std::fs::remove_dir_all(&vault);
    }

    // --- run records ------------------------------------------------------------------------

    #[test]
    fn run_writes_start_and_end_records_and_log_line_from_record() {
        let vault = bare_vault("runrecs");
        seed_migrated(&vault);
        pystr::write_text(
            &vault.join("tasks").join("a.md"),
            "---\ntitle: T\ndue: 2026-09-11\neffort_hours: 2\nimportance: 3\n---\n",
        )
        .unwrap();
        run(&vault, Some("2026-09-07"), "local", None).unwrap();
        let recs = run_records(&vault);
        let phases: Vec<String> = recs.iter().map(|r| record_str(r, "phase")).collect();
        assert_eq!(phases, vec!["start", "end"]);
        assert_eq!(record_str(&recs[0], "runner"), "local");
        assert_eq!(record_str(&recs[1], "result"), "ok");
        let summary = record_str(&recs[1], "summary");
        assert!(summary.starts_with("today.md refreshed (1 active"), "{summary}");
        let log = runner_log(&vault);
        assert!(pystr::strip(&log).ends_with(&format!("local ok {summary}")), "{log}");
        let _ = std::fs::remove_dir_all(&vault);
    }

    #[test]
    fn run_records_full_warning_text_in_end_record() {
        let vault = bare_vault("fullwarn");
        pystr::write_text(
            &vault.join("tasks").join("bad.md"),
            "---\ntitle: Bad\ndue: [unclosed\nstatus: active\n---\n",
        )
        .unwrap();
        run(&vault, Some("2026-09-07"), "manual", None).unwrap();
        let recs = run_records(&vault);
        let end = recs.last().unwrap();
        assert_eq!(record_str(end, "result"), "WARN");
        let steps = end.get("steps").and_then(|s| s.as_array()).unwrap();
        assert!(
            steps.iter().any(|s| s.get("message").and_then(|m| m.as_str()).is_some_and(|m| m.contains("unreadable"))),
            "{steps:?}"
        );
        let _ = std::fs::remove_dir_all(&vault);
    }

    /// The Python test monkeypatches `render_today` to raise. Nothing here can be patched, so the
    /// crash is a real one on the same path: `state/today.md` is a *directory*, the page write
    /// fails, and `crash` has to leave the FAIL record and the log line behind before the error
    /// comes back. The class name in the summary is this crate's variant (`Io`) where Python
    /// prints `RuntimeError` — the documented divergence on `RunError::variant_name`.
    #[test]
    fn run_crash_writes_fail_end_record_and_log_line() {
        let vault = bare_vault("crash");
        pystr::write_text(
            &vault.join("tasks").join("a.md"),
            "---\ntitle: T\ndue: 2026-09-11\neffort_hours: 2\nimportance: 3\n---\n",
        )
        .unwrap();
        std::fs::create_dir_all(vault.join("state").join("today.md")).unwrap();
        let err = run(&vault, Some("2026-09-07"), "local", None).unwrap_err();
        assert!(matches!(err, RunError::Io(_)), "{err}");

        let recs = run_records(&vault);
        let phases: Vec<String> = recs.iter().map(|r| record_str(r, "phase")).collect();
        assert_eq!(phases, vec!["start", "end"]);
        let end = recs.last().unwrap();
        assert_eq!(record_str(end, "result"), "FAIL");
        assert!(record_str(end, "summary").starts_with("engine crashed: Io: "), "{end:?}");
        let log = runner_log(&vault);
        let last = pystr::strip(&log).lines().last().unwrap_or("").to_string();
        assert!(last.contains("FAIL engine crashed: Io: "), "{last}");
        let _ = std::fs::remove_dir_all(&vault);
    }

    const DIGEST_FRONT: &str = "type: approval\nkind: events-digest\ntitle: Events\nstatus: approved\n\
proposed_at: 2026-08-28\nfirst_proposed_at: 2026-08-28\nexpires: 2026-08-30\n\
events:\n  - uid: \"ics:evt-1\"\n    summary: \"Career Fair Game Plan\"\n    start: 2026-09-03T13:00\n    end: 2026-09-03T14:00\n    location: \"Career Center\"\n";
    const DIGEST_BODY: &str = "**1 event.**\n\n- [x] Thu 9/3 13:00 \u{b7} Career Fair Game Plan \u{b7} Career Center \u{b7} `ics:evt-1`\n";

    fn set_readonly(path: &Path, readonly: bool) {
        let mut perms = std::fs::metadata(path).unwrap().permissions();
        perms.set_readonly(readonly);
        std::fs::set_permissions(path, perms).unwrap();
    }

    /// Task 12b review ruling: a crash between the executed stamp's journal record and its note
    /// write must not survive as a duplicated calendar event. Run 1 crashes mid-stamp (journal has
    /// `status: executed`, the note on disk still says `approved`); run 2's `verify_tail` heals
    /// the note from the journal, and `process_approvals` then archives the already-healed digest
    /// through the plain `status == "executed"` branch rather than re-expanding its payload — so
    /// only one calendar-event note ever exists.
    ///
    /// Python opens that window by monkeypatching `update_frontmatter_fields` to raise once. Here
    /// the digest note is made **read-only** before run 1: `write_literals` appends both records,
    /// then `std::fs::write` is refused, which is the same window at the same instruction — and
    /// `expand_digest` never writes to the digest itself, so nothing fails earlier. Measured
    /// against the Python engine with the read-only file in place of the monkeypatch (2026-09-02):
    /// identical run summaries, the same single note, the same heal lines.
    ///
    /// The sibling `approvals::a_re_expanded_digest_never_writes_a_second_calendar_note` covers
    /// the OTHER window — a stamp refused before journaling — where uid dedup is what blocks the
    /// second note.
    #[test]
    fn crash_after_executed_stamp_heals_then_archives_without_re_expansion() {
        let vault = bare_vault("stampcrash");
        std::fs::create_dir_all(vault.join("approvals")).unwrap();
        // Seeded through `write::create` — the real `emit_digest` path — so the digest starts with
        // a full `create` journal record covering every field, matching production.
        crate::write::create(
            &vault,
            "approvals/events-digest-2026-08-28.md",
            &format!("---\n{DIGEST_FRONT}---\n\n{DIGEST_BODY}"),
            &WriteContext::new("agent:events", "cli"),
            &mut Journal::new(&vault),
            None,
        )
        .unwrap();
        let digest = vault.join("approvals").join("events-digest-2026-08-28.md");

        set_readonly(&digest, true);
        let out1 = run(&vault, Some("2026-08-29"), "manual", None);
        set_readonly(&digest, false);
        out1.unwrap();

        let end1 = run_records(&vault).last().unwrap().clone();
        assert_eq!(record_str(&end1, "result"), "WARN");
        let approvals1 = step_of(&end1, "approvals");
        assert!(
            approvals1["message"].as_str().unwrap_or("").contains("transition failed"),
            "{approvals1}"
        );
        assert_eq!(md_names(&vault.join("approvals"), "calendar-event-").len(), 1);

        run(&vault, Some("2026-08-29"), "manual", None).unwrap();

        assert_eq!(md_names(&vault.join("approvals"), "calendar-event-").len(), 1); // no second note

        let archived = vault.join("archive").join("events-digest-2026-08-28.md");
        assert!(archived.exists());
        assert_eq!(meta_str(&archived, "status"), "executed");

        let end2 = run_records(&vault).last().unwrap().clone();
        let approvals2 = step_of(&end2, "approvals");
        assert!(!approvals2["message"].as_str().unwrap_or("").contains("transition failed"));

        // Prove the no-second-note property actually comes from verify_tail healing the stamp,
        // not merely from the uid-dedup safety net: the passes step must show the re-apply, and
        // detect_external must have stayed quiet entirely on the digest's id — seeing disk already
        // equal to the journal.
        let passes2 = step_of(&end2, "passes");
        assert!(
            passes2["message"]
                .as_str()
                .unwrap_or("")
                .contains("re-applied status on approvals/events-digest-2026-08-28.md"),
            "{passes2}"
        );
        let digest_id = meta_str(&archived, "id");
        assert!(!digest_id.is_empty());
        let external: Vec<crate::ledger::Record> = Journal::new(&vault)
            .read(None, None)
            .into_iter()
            .filter(|r| record_str(r, "via") == "external" && record_str(r, "id") == digest_id)
            .collect();
        assert!(external.is_empty(), "{external:?}");
        let _ = std::fs::remove_dir_all(&vault);
    }
}
