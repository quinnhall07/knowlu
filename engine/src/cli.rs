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

use std::cell::RefCell;
use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

use jiff::civil::{Date, DateTime, Time};
use jiff::tz::TimeZone;
use jiff::Timestamp;

use crate::approvals::{count_proposals_created, defer_over_budget, process_approvals};
use crate::calfeed::load_calendar_events;
use crate::eventemit::{emit_digest, emit_event_checks, inherit_series_answers};
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

/// The vault's timezone name, exactly as `config/ingest.yaml` names it — `None` when the file is
/// missing, unreadable, leaves the key out, sets it null, or sets it to an empty (or all-blank)
/// string. Unlike [`vault_zone`], this never falls back to the machine's own clock: a cloud
/// request body (`cloudmodel::email_request`, `pull_gmail_queue`'s `/gmail-read` body) must say
/// nothing rather than claim a zone the vault never named. Not validated against `jiff`'s zone
/// table on purpose — the due resolver already degrades an unrecognised name to "no timezone"
/// without throwing, so a typo here costs nothing extra by traveling.
pub fn vault_timezone_name(vault: &Path) -> Option<String> {
    let config = crate::yaml::mapping_from_file(&vault.join("config").join("ingest.yaml"));
    let name = crate::yaml::get(&config, "timezone").and_then(crate::yaml::text)?;
    let trimmed = name.trim();
    if trimmed.is_empty() {
        None
    } else {
        Some(trimmed.to_string())
    }
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

/// One calendar feed's raw series material, stashed by the closure that fetched it (spec §4.2),
/// keyed by the feed's **URL** exactly as the calendar closure received it (`cloud:google`, or a
/// direct `https://` address) — the same string `calfeed::calendar_entries` hands back, so P16's
/// `commitments::refresh_series` can match a stash entry to its `config/ingest.yaml` row.
#[derive(Debug, Clone, PartialEq)]
pub enum StashEntry {
    /// `/ingest-calendar`'s `series` field, verbatim, for `cloud:google` (§4.1) — normalised
    /// directly, with no ICS round trip.
    Google(serde_json::Value),
    /// The ICS text of any other feed the closure fetched — a direct address, or `cloud:personal`
    /// (which the function never sends series for). **Not** `cloud:google` with no `series`: that
    /// case stashes nothing at all (§3.3/§4.3 — see `calendar_fetcher`). `calfeed::weekly_series`
    /// derives series from this text.
    Ics(String),
}

/// One run's series material, gathered while `Fetchers.calendar` runs and read afterward by P16's
/// `commitments::refresh_series` — never written to or read from inside the calendar closure
/// itself except to insert. A `RefCell` because the closure `run` hands to `run_with` is a shared
/// `&dyn Fn`, not a `FnMut`.
pub type SeriesStash = RefCell<BTreeMap<String, StashEntry>>;

/// The three network reads a run performs, replaceable so a test can drive the **whole** run
/// offline against a scripted feed.
///
/// Python's `test_cli.py` monkeypatches `calfeed.fetch_ics` and `eventfeed.fetch_event_source`
/// as module globals; `calfeed` and `eventfeed` already carry this seam one level down, and
/// `run_with` only threads it through. **Production (`run`) builds both, since hand-off H4**: an
/// `events` closure that tries the service first and falls back to `eventfeed::fetch_event_source`,
/// and a `calendar` closure that routes a `cloud:<name>` url to the service and everything else to
/// `calfeed::fetch_ics`. Only tests construct a bare `Fetchers` directly, and several of those pass
/// `Fetchers::default()`, which leaves every field at `None` — the on-device fallbacks `calfeed`
/// and `eventfeed` would have used anyway, and no series stash.
///
/// `series` is explicit rather than folded into the `calendar` closure's own state (plan review
/// I8): `run_with`'s callers hand it a `&SeriesStash` so a test can inspect what the closure
/// stashed, or inject series ahead of time, with no network at all.
#[derive(Default, Clone, Copy)]
pub struct Fetchers<'a> {
    pub calendar: Option<&'a dyn Fn(&str) -> Result<String, String>>,
    pub events: Option<&'a dyn Fn(&str) -> Result<String, String>>,
    pub series: Option<&'a SeriesStash>,
}

/// Builds the real `calendar` closure `run` hands to `run_with` — pulled out of `run` itself so a
/// test can drive it directly against a loopback service or a loopback feed, with no vault and no
/// `run_with` call at all.
///
/// A `cloud:<name>` url routes to `cloudmodel::fetch_calendar`. For `name == "google"` the reply's
/// `series` is stashed as `StashEntry::Google` **only when the reply carried one**; when it did
/// not — an old server that predates §4.1, or a new one whose series budget ran out or whose
/// series gathering failed — that feed's stash is left **untouched**, exactly as if this run had
/// never asked (§3.3: "a Google reply that carried no `series` field — keeps its series and its
/// date untouched"; §4.3: "`series` absent → `None` → that feed's series keep their previous
/// state"). Every other successfully fetched feed — a direct address, or a `cloud:`-routed feed
/// the function never sends series for (`cloud:personal`) — stashes its ICS text as
/// `StashEntry::Ics` instead. A failed fetch stashes nothing, under every branch. The closure's
/// own `Fn(&str) -> Result<String, String>` shape is unchanged, so `calfeed::load_calendar_events`
/// and every oracle test are untouched (spec §4.2).
fn calendar_fetcher<'a>(
    cloud: &'a Option<crate::cloudmodel::CloudClient>,
    stash: &'a SeriesStash,
) -> impl Fn(&str) -> Result<String, String> + 'a {
    move |url: &str| -> Result<String, String> {
        match (url.strip_prefix("cloud:"), cloud) {
            (Some(name), Some(client)) => {
                let (ics, series) = crate::cloudmodel::fetch_calendar(client, name)?;
                match (series, name) {
                    (Some(series), "google") => {
                        stash.borrow_mut().insert(url.to_string(), StashEntry::Google(series));
                    }
                    // §3.3/§4.3: no `series` field on a `cloud:google` reply means this calendar
                    // was not read this run — the stash must stay exactly as it was, never gain a
                    // phantom ICS-derived entry under the same url.
                    (None, "google") => {}
                    _ => {
                        stash.borrow_mut().insert(url.to_string(), StashEntry::Ics(ics.clone()));
                    }
                }
                Ok(ics)
            }
            // A `cloud:` feed on a vault with no account is not an error worth failing a run for:
            // `load_calendar_events` turns this into "using snapshot" and the day still ranks.
            (Some(_), None) => Err("no account on this vault".to_string()),
            (None, _) => {
                let ics = crate::calfeed::fetch_ics(url)?;
                stash.borrow_mut().insert(url.to_string(), StashEntry::Ics(ics.clone()));
                Ok(ics)
            }
        }
    }
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
    // C2 (cloud design §3.1): event feeds are fetched by the service, which can present a real
    // browser's headers and follow a redirect chain the desktop could not. A transport swap and
    // nothing more — the parsers, the pre-filter, the roster and the digest are the same
    // deterministic code, and `rank` still never calls a model (decision 11). A vault with no
    // account, or a service that is down, falls straight back to `eventfeed::fetch_event_source`.
    let cloud = crate::cloudmodel::resolve(vault).ok();
    let events = |url: &str| -> Result<String, String> {
        match &cloud {
            Some(client) => crate::cloudmodel::fetch_event_source(client, url)
                .or_else(|_| crate::eventfeed::fetch_event_source(url)),
            None => crate::eventfeed::fetch_event_source(url),
        }
    };
    // C2 Task 8 (cloud design §11a): a `calendars:` entry whose `ics_url` is `cloud:<name>` is
    // served by `/ingest-calendar` — the Google grant's events, or the account's stored secret
    // iCal address, rendered as ICS. Every other url is fetched on the device exactly as before,
    // so the `calendar_ics` secret-address path keeps working with no account at all. `calfeed`
    // then parses, bounds to its 28-day horizon, dedups and snapshots it like any other feed:
    // there is no second parser and no new vault file. `run` owns the one stash the closure fills
    // (spec §4.2); P16's `commitments::refresh_series` reads it after `load_calendar_events` runs.
    let stash: SeriesStash = RefCell::new(BTreeMap::new());
    let calendar = calendar_fetcher(&cloud, &stash);
    run_with(
        vault,
        today_iso,
        runner,
        run_id,
        Fetchers { calendar: Some(&calendar), events: Some(&events), series: Some(&stash) },
    )
}

/// [`run`] with the network seam exposed — a test seam: production (`run`) always builds real
/// fetchers, and only tests construct a `Fetchers` directly (several pass `Fetchers::default()`,
/// which leaves both closures at their module-level defaults). Everything else is identical.
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
        let mut ledger = load_ledger(vault, Some(&mut ledger_warnings));
        // A settled series card answers the series' later instances (F2) — before the roster and
        // the digest, so this run's roster already shows the inherited verdict.
        let (_, inherit_warnings) = inherit_series_answers(vault, &candidates, &mut ledger, today);
        ledger_warnings.extend(inherit_warnings);
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
        // The "Does this apply to you?" cards (F2), sized to what the digest left of the budget.
        let (_, checks) = emit_event_checks(
            vault,
            &candidates,
            &ledger,
            &events_config,
            today,
            (remaining_budget - emitted as i64).max(0),
            &ctx.with_actor("agent:events"),
            &mut journal,
        );
        approvals.pending += checks as i64;
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
    let (cal_events, mut cal_warnings) = load_calendar_events(vault, today, fetchers.calendar);

    // The commitment passes (spec §4.2, §5.2, §5.4; P16): after the calendar closure filled the
    // stash and before the calendar is built. Every warning joins the `calendar` step (R13: no new
    // step). The cards they file are counted into this run's pending line, as the events pass
    // counts its checks (re-review M-a).
    let passes = commitment_passes(
        vault,
        today,
        &planning,
        fetchers.series,
        &ctx,
        &mut journal,
        &mut cal_warnings,
    );
    // A card withdrawn this run was counted pending by `process_approvals`: take it back out, and
    // re-age the queue without it (P16 fix round 1, I1).
    approvals.pending += passes.filed - passes.withdrawn_pending;
    if passes.withdrawn_pending > 0 {
        approvals.oldest_pending_days = crate::approvals::oldest_pending_days(vault, today);
    }
    let commitments = passes.set;

    // §6.1 (C3): the one constructor `surface` uses too, so `today.md` and the console agree.
    let cal = WeekCalendar::for_vault(vault, cal_events);
    cal_warnings.extend(crate::commitments::record_baseline(vault, &commitments, &cal, today));

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

/// §3.2 over one run's stash: each entry becomes `(calendar key, its series)` for every calendar
/// it read fresh and complete. A Google entry is normalised directly (its `calendars_read` are
/// the keys; a calendar with an unreadable item is left out by `series_from_google`); an ICS entry
/// is normalised once per `config/ingest.yaml` feed whose url it was fetched from, under that
/// feed's name (text that is not a calendar is no read at all). In stash (url) order, so the same
/// stash always gives the same list. Returns `(fresh, warnings)`.
fn normalise_stash(
    vault: &Path,
    entries: &BTreeMap<String, StashEntry>,
    tz: &TimeZone,
    today: Date,
) -> (Vec<(String, Vec<crate::commitments::Series>)>, Vec<String>) {
    let feeds = crate::calfeed::calendar_entries(vault);
    let mut fresh: Vec<(String, Vec<crate::commitments::Series>)> = Vec::new();
    let mut warnings: Vec<String> = Vec::new();
    for (url, entry) in entries {
        match entry {
            StashEntry::Google(value) => {
                let (series, calendars, found) = crate::commitments::series_from_google(value, tz, today);
                warnings.extend(found);
                for calendar in calendars {
                    let list = series.iter().filter(|s| s.calendar == calendar).cloned().collect();
                    fresh.push((calendar, list));
                }
            }
            StashEntry::Ics(text) => {
                // The closure stashes only urls `calendar_entries` handed it this run, so a url
                // with no feed row cannot occur; if it ever did, nothing is read (no ageing).
                for (name, _) in feeds.iter().filter(|(_, feed_url)| feed_url == url) {
                    let (series, found) = crate::commitments::series_from_ics(name, text, tz, today);
                    warnings.extend(found);
                    if let Some(series) = series {
                        fresh.push((name.clone(), series));
                    }
                }
            }
        }
    }
    (fresh, warnings)
}

/// The commitment passes of one `rank`, in the order plan review I5 and the P13/P8 carry-forwards
/// fix: normalise the stash; `refresh_series` → the file; `commitments::load` → the set (after
/// `process_approvals`, so this run's settlements are in it); `withdraw_stale` — skipped when the
/// series file could not be read, as is the **ended** rule, because a bad read is never "gone";
/// `detect_changes`; `successor_keys`; `proposals`; `emit_checks`, with the budget recounted here
/// after the events pass took its share; `emit_asks` (skipped on a bad series read), with what the
/// check cards left. Pure of clocks, networks and models: the stash was filled
/// by the calendar closure and everything here is deterministic. Every warning is pushed onto
/// `warnings`.
fn commitment_passes(
    vault: &Path,
    today: Date,
    planning: &crate::planning::PlanningConfig,
    stash: Option<&SeriesStash>,
    ctx: &WriteContext,
    journal: &mut Journal,
    warnings: &mut Vec<String>,
) -> CommitmentPasses {
    use crate::commitments as cm;
    let entries: BTreeMap<String, StashEntry> = stash.map(|s| s.borrow().clone()).unwrap_or_default();
    let (fresh, normalise_warnings) = normalise_stash(vault, &entries, &vault_zone(vault), today);
    warnings.extend(normalise_warnings);

    let (file, series_warnings) = cm::refresh_series(vault, &fresh, today);
    let read_failed = cm::series_read_failed(&series_warnings);
    warnings.extend(series_warnings);

    let set = cm::load(vault);
    warnings.extend(set.warnings.iter().cloned());

    let mut withdrawn_pending = 0;
    if !read_failed {
        let withdrawal = crate::approvals::withdraw_stale(vault, &file, &set, today, ctx, journal);
        withdrawn_pending = withdrawal.pending;
        warnings.extend(withdrawal.warnings);
    }

    let (codes, code_warnings) = cm::Codes::load(vault);
    warnings.extend(cm::code_warnings_hit(code_warnings, &file));
    let names: Vec<String> = planning.recurring.iter().map(|r| r.name.clone()).collect();
    let mut fresh_keys: std::collections::BTreeSet<String> = fresh.iter().map(|(c, _)| c.clone()).collect();
    // Phase 3 (Plan ruling R2-g): a registrar term is read only on a button press, and the file
    // always holds that read, so its notes are watched on every run.
    fresh_keys.extend(file.calendars.keys().filter(|c| c.starts_with(crate::registrar::CALENDAR_PREFIX)).cloned());
    let watched = match read_failed {
        true => cm::SeriesFile { ended: BTreeMap::new(), ..file.clone() },
        false => file.clone(),
    };
    let (changes, change_warnings) = cm::detect_changes(&watched, &set, &codes, &names, &fresh_keys, today, journal);
    warnings.extend(change_warnings);

    let held = cm::successor_keys(vault);
    let template = WeekCalendar::from_file(&vault.join("config").join("week_template.yaml"), Vec::new());
    let proposals = cm::proposals(&file, &set, &codes, &names, &template, &held, today, true);
    // Phase-2 spec §5 (Plan ruling Q7-c): an ask whose course now has a class proposal, a class
    // note or a marker is withdrawn before either emitter runs.
    let asks_out = crate::approvals::withdraw_asks(vault, &proposals, &set, ctx, journal);
    withdrawn_pending += asks_out.pending;
    warnings.extend(asks_out.warnings);
    // Phase-2 spec D2 (Plan ruling Q1-b): on the vault's first day the confirm screen asks, so no
    // proposal or window card is filed. Change cards cannot exist yet, and pass through unchanged.
    let asked: &[cm::Proposal] = if cm::vault_day(vault, today) == 1 { &[] } else { &proposals };
    let budget = std::cmp::max(0, planning.daily_approval_budget - count_proposals_created(vault, today));
    let (_, filed, card_warnings) = cm::emit_checks(vault, asked, &changes, today, budget, ctx, journal);
    warnings.extend(card_warnings);
    // Phase-2 spec §5: the per-course fallback cards, with what the check cards left of the
    // budget. Skipped on a bad series read (Plan ruling Q6-b): an unread calendar would make
    // every course look uncovered, and while a registrar term has not begun (phase-3 D7, Plan
    // ruling R2-h).
    let mut asks = 0;
    if !read_failed && !cm::asks_wait_for_registrar(&file, today) {
        let ask_budget = std::cmp::max(0, planning.daily_approval_budget - count_proposals_created(vault, today));
        let (_, n, ask_warnings) = cm::emit_asks(vault, &proposals, today, ask_budget, ctx, journal);
        asks = n;
        warnings.extend(ask_warnings);
    }
    CommitmentPasses { filed: (filed + asks) as i64, withdrawn_pending, set }
}

/// What [`commitment_passes`] hands back to `run_with`.
struct CommitmentPasses {
    /// Cards filed this run, checks and asks (added to the pending line).
    filed: i64,
    /// Cards withdrawn this run that `process_approvals` had counted pending (taken back out).
    withdrawn_pending: i64,
    /// The notes, for `record_baseline`.
    set: crate::commitments::Commitments,
}

// -----------------------------------------------------------------------------------------------
// R14: the read-only `commitments` command (spec §5.1's phase-2 data source).
// -----------------------------------------------------------------------------------------------

/// [`commitments_report`]'s answer: the current proposals (the window proposal included, office
/// hours included — R8's card-only exclusion does not apply here) and every warning collecting
/// them raised. Never a note, a card or a journal record.
pub struct CommitmentsReport {
    pub proposals: Vec<crate::commitments::Proposal>,
    /// Phase-2 spec §2: the courses with neither a class proposal nor a confirmed class note.
    pub uncovered: Vec<crate::commitments::UncoveredCourse>,
    pub warnings: Vec<String>,
}

/// `"HH:MM"`, 24-hour — [`crate::commitments`]'s own `hm` is private to that module, so the
/// command prints its own (same format, spec §5.2).
fn hm(time: Time) -> String {
    format!("{:02}:{:02}", time.hour(), time.minute())
}

/// One proposal as the phase-2 screen reads it: everything [`crate::commitments::file_card`]
/// would put in a card's `commitment:` mapping, plus the `source_uid` the app answers back with
/// (§5.1: "the app passes keys, levels and the window ... re-derives each proposal from the
/// series file by its `source_uid`"). Not a byte contract — no frozen fixture pins this shape.
/// Delegates to `commitments::proposal_value` (phase 2 adds `when`).
pub fn proposal_json(p: &crate::commitments::Proposal) -> serde_json::Value {
    crate::commitments::proposal_value(p)
}

/// One proposal, one line, for a human running `commitments` without `--json`.
pub fn proposal_line(p: &crate::commitments::Proposal) -> String {
    let when: Vec<String> = p
        .meets
        .iter()
        .map(|m| format!("{} {}-{}", m.days.join("/"), hm(m.start), hm(m.end)))
        .collect();
    if when.is_empty() {
        format!("{} ({})", p.title, p.kind)
    } else {
        format!("{} ({}) {}", p.title, p.kind, when.join(", "))
    }
}

/// Production `commitments`: builds the same calendar fetcher and stash [`run`] does, so a
/// `cloud:` feed and a direct address are both reached exactly as a slot would reach them, and
/// hands off to [`commitments_report_with`].
pub fn commitments_report(vault: &Path, today_iso: Option<&str>) -> CommitmentsReport {
    let cloud = crate::cloudmodel::resolve(vault).ok();
    let stash: SeriesStash = RefCell::new(BTreeMap::new());
    let calendar = calendar_fetcher(&cloud, &stash);
    commitments_report_with(
        vault,
        today_iso,
        Fetchers { calendar: Some(&calendar), events: None, series: Some(&stash) },
    )
}

/// [`commitments_report`] with the network seam exposed (mirrors [`run`]/[`run_with`]) — a test
/// seam: production always builds the real fetcher; a test can inject stash entries directly, as
/// [`run_with`]'s own commitment-pass tests do, with no network at all.
///
/// R14: fetches each `calfeed::calendar_entries` feed **directly through `fetchers.calendar`,
/// never through `load_calendar_events`**, so `state/calendar.md` is not rewritten; refreshes
/// `state/calendar-series.json` (`commitments::refresh_series`, written only when a feed answered
/// and the bytes changed); reads `commitments::load`, the codes and the planning names exactly as
/// [`commitment_passes`] does; builds the proposals with `for_cards: false`, so office hours are
/// not filtered out of what the phase-2 screen sees. **Always succeeds**: an unreadable or
/// malformed `config/ingest.yaml`, and a feed that fails to fetch, each become one line in
/// `warnings` rather than an error — the command has nothing to crash a morning's ranking with,
/// because it never touches `today.md`, a note, a card or the journal.
pub fn commitments_report_with(
    vault: &Path,
    today_iso: Option<&str>,
    fetchers: Fetchers<'_>,
) -> CommitmentsReport {
    use crate::commitments as cm;
    let mut warnings: Vec<String> = Vec::new();
    let today = match today_iso {
        Some(iso) => match Date::strptime("%Y-%m-%d", iso) {
            Ok(d) => d,
            Err(_) => {
                warnings.push(format!("bad --today {iso:?}; using today"));
                Timestamp::now().to_zoned(vault_zone(vault)).date()
            }
        },
        None => Timestamp::now().to_zoned(vault_zone(vault)).date(),
    };

    // `calendar_entries` itself swallows a bad `config/ingest.yaml` silently (an empty feed list —
    // correct for `refresh_series`, which must never treat a config problem as a reason to drop
    // series it already has), so the same read is repeated here only to name the failure.
    let config_path = vault.join("config").join("ingest.yaml");
    if config_path.exists() {
        match pystr::read_text(&config_path) {
            Err(err) => warnings.push(format!("config unreadable: {err}")),
            Ok(text) => match serde_yaml_ng::from_str::<serde_yaml_ng::Value>(&text) {
                Err(err) => warnings.push(format!("config unreadable: {err}")),
                Ok(config) => {
                    // M4 (fix round 1): a `calendars:` value that is present and truthy but not a
                    // list is exactly the shape `calendars_feeds` (private to `calfeed.rs`) still
                    // iterates without error — a string one character at a time, a mapping one key
                    // at a time — and `calendar_entries` then filters every one of those out
                    // silently (`feed.as_mapping()` is `None`), so today's command would otherwise
                    // report "no proposals, no warnings" for a config it could not use at all.
                    if let Some(value) = config.get("calendars") {
                        if pystr::yaml_truthy(value) && value.as_sequence().is_none() {
                            warnings.push(format!(
                                "config: calendars is not a list ({}); no feed will be read",
                                pystr::yaml_type_name(value)
                            ));
                        }
                    }
                }
            },
        }
    }

    if let Some(fetch) = fetchers.calendar {
        for (name, url) in crate::calfeed::calendar_entries(vault) {
            if url.is_empty() {
                continue;
            }
            if let Err(err) = fetch(&url) {
                warnings.push(format!("{name}: fetch failed ({err})"));
            }
        }
    }

    let entries: BTreeMap<String, StashEntry> =
        fetchers.series.map(|s| s.borrow().clone()).unwrap_or_default();
    let (fresh, normalise_warnings) = normalise_stash(vault, &entries, &vault_zone(vault), today);
    warnings.extend(normalise_warnings);

    let (file, series_warnings) = cm::refresh_series(vault, &fresh, today);
    warnings.extend(series_warnings);

    let set = cm::load(vault);
    warnings.extend(set.warnings.iter().cloned());

    let (codes, code_warnings) = cm::Codes::load(vault);
    warnings.extend(cm::code_warnings_hit(code_warnings, &file));

    let planning = load_planning(&vault.join("config").join("planning.yaml"));
    let names: Vec<String> = planning.recurring.iter().map(|r| r.name.clone()).collect();
    let held = cm::successor_keys(vault);
    let template = WeekCalendar::from_file(&vault.join("config").join("week_template.yaml"), Vec::new());
    let proposals = cm::proposals(&file, &set, &codes, &names, &template, &held, today, false);

    let uncovered = cm::uncovered_courses(vault, &set, &proposals, &codes);
    CommitmentsReport { proposals, uncovered, warnings }
}

/// Phase-2 spec §3: `commitments --confirm`. Parses the input, pins `today` (the vault's zone
/// when none is given) and runs `commitments::confirm` under `ctx`. It fetches nothing: no
/// calendar fetcher is built and `config/cloud.yaml` is not read. `Err` is the exit-2 message.
pub fn commitments_confirm(
    vault: &Path,
    today_iso: Option<&str>,
    input: &str,
    ctx: &WriteContext,
) -> Result<crate::commitments::ConfirmReport, String> {
    let input = crate::commitments::parse_confirm(input)?;
    let today = match today_iso {
        Some(iso) => Date::strptime("%Y-%m-%d", iso).map_err(|_| format!("bad --today {iso:?}"))?,
        None => Timestamp::now().to_zoned(vault_zone(vault)).date(),
    };
    let mut journal = Journal::new(vault);
    crate::commitments::confirm(vault, &input, today, ctx, &mut journal)
}

/// Phase-3 spec §3: `commitments --registrar`. Pins `today` (the vault's zone when none is given)
/// and runs `registrar::run`. It fetches nothing. `Err` is the exit-2 message.
pub fn commitments_registrar(
    vault: &Path,
    today_iso: Option<&str>,
    text: &str,
    school: &str,
    ctx: &WriteContext,
) -> Result<crate::registrar::Report, String> {
    let today = match today_iso {
        Some(iso) => Date::strptime("%Y-%m-%d", iso).map_err(|_| format!("bad --today {iso:?}"))?,
        None => Timestamp::now().to_zoned(vault_zone(vault)).date(),
    };
    let mut journal = Journal::new(vault);
    crate::registrar::run(vault, text, school, today, ctx, &mut journal)
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

    /// `vault_timezone_name` feeds cloud request bodies, so — unlike `vault_zone` — it must say
    /// `None` rather than the machine's own clock whenever the vault itself named nothing usable.
    #[test]
    fn vault_timezone_name_is_none_unless_the_vault_actually_names_one() {
        let vault = bare_vault("tzname");
        assert_eq!(vault_timezone_name(&vault), None, "no config file at all");

        pystr::write_text(&vault.join("config").join("ingest.yaml"), "timezone: America/Chicago\n")
            .unwrap();
        assert_eq!(vault_timezone_name(&vault), Some("America/Chicago".to_string()));

        pystr::write_text(&vault.join("config").join("ingest.yaml"), "ics_url: https://x\n").unwrap();
        assert_eq!(vault_timezone_name(&vault), None, "key absent");

        pystr::write_text(&vault.join("config").join("ingest.yaml"), "timezone: ~\n").unwrap();
        assert_eq!(vault_timezone_name(&vault), None, "key present but null");

        pystr::write_text(&vault.join("config").join("ingest.yaml"), "timezone: \"\"\n").unwrap();
        assert_eq!(vault_timezone_name(&vault), None, "empty string");

        pystr::write_text(&vault.join("config").join("ingest.yaml"), "timezone: \"   \"\n").unwrap();
        assert_eq!(vault_timezone_name(&vault), None, "blank string");

        pystr::write_text(&vault.join("config").join("ingest.yaml"), "ics_url: [unclosed\n").unwrap();
        assert_eq!(vault_timezone_name(&vault), None, "malformed config");
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
        let fetchers = Fetchers { calendar: Some(&fetch), events: None, series: None };
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
        let fetchers = Fetchers { calendar: Some(&boom), events: None, series: None };
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
        let fetchers = Fetchers { calendar: None, events: Some(&fetch), series: None };
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
        let fetchers = Fetchers { calendar: None, events: Some(&fetch), series: None };
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
        let fetchers = Fetchers { calendar: None, events: Some(&fetch), series: None };
        run_with(&vault, Some("2026-08-26"), "manual", None, fetchers).unwrap();

        // remaining budget = 15 - 5 Gmail proposals = 10; exactly 10 of the 11 eligible events
        // land in the digest, and the latest-starting one (ev-10) is the one left out.
        let chosen = digest_uids(&vault);
        assert_eq!(chosen.len(), 10, "{chosen:?}");
        assert!(!chosen.iter().any(|u| u == "ics:ev-10"), "{chosen:?}");
        assert!(chosen.iter().any(|u| u == "ics:ev-9"), "{chosen:?}");
        let _ = std::fs::remove_dir_all(&vault);
    }

    // --- the event-check card (F2) ------------------------------------------------------------

    const CHECK_JID: &str = "0f0e0d0c-0b0a-4908-8706-050403020100";

    #[test]
    fn rank_files_an_event_check_for_an_unsure_event_and_counts_it_pending() {
        let vault = scaffold("evcheck");
        let ics = "BEGIN:VCALENDAR\nBEGIN:VEVENT\nDTSTART;TZID=America/Chicago:20260827T100000\n\
                   DTEND;TZID=America/Chicago:20260827T150000\nUID:fair-1\nSUMMARY:Career Fair\n\
                   LOCATION:Union\nEND:VEVENT\nEND:VCALENDAR\n"
            .to_string();
        crate::eventledger::record_judged_verdict(
            &vault,
            "ics:fair-1",
            "Career Fair",
            Date::constant(2026, 8, 25),
            "unsure",
            "the listing does not say who it is for",
            Some(CHECK_JID),
        )
        .unwrap();
        let fetch = |_: &str| Ok(ics.clone());
        let fetchers = Fetchers { calendar: None, events: Some(&fetch), series: None };
        run_with(&vault, Some("2026-08-26"), "manual", None, fetchers).unwrap();

        let cards = md_names(&vault.join("approvals"), "event-check-");
        assert_eq!(cards, vec!["event-check-career-fair-2026-08-27.md".to_string()]);
        let card = vault.join("approvals").join(&cards[0]);
        assert_eq!(meta_str(&card, "title"), "Career Fair · Thu 27 Aug 10am–3pm");
        assert_eq!(meta_str(&card, "judgment_id"), CHECK_JID);
        let recs = run_records(&vault);
        let end = recs.last().unwrap();
        assert_eq!(step_of(end, "approvals")["counts"]["pending"], serde_json::json!(1), "{end:?}");
        // No digest: an `unsure` event is a question, never an opportunity.
        assert!(md_names(&vault.join("approvals"), "events-digest-").is_empty());

        // The next run asks nothing new and still counts the card pending.
        let fetchers = Fetchers { calendar: None, events: Some(&fetch), series: None };
        run_with(&vault, Some("2026-08-26"), "manual", None, fetchers).unwrap();
        assert_eq!(md_names(&vault.join("approvals"), "event-check-").len(), 1);
        let recs = run_records(&vault);
        assert_eq!(step_of(recs.last().unwrap(), "approvals")["counts"]["pending"], serde_json::json!(1));
        let _ = std::fs::remove_dir_all(&vault);
    }

    #[test]
    fn rank_shows_a_later_instance_of_an_approved_series_in_the_same_run() {
        let vault = scaffold("evseries");
        pystr::write_text(
            &vault.join("config").join("events.yaml"),
            "sources:\n  - name: campus\n    type: localist\n    url: unreachable://x\n    enabled: true\n",
        )
        .unwrap();
        let feed = serde_json::json!({"events": [{"event": {
            "id": 77,
            "title": "Weekly Meeting",
            "event_instances": [
                {"event_instance": {"id": 1, "start": "2026-08-20T18:00:00-05:00", "end": "2026-08-20T19:00:00-05:00"}},
                {"event_instance": {"id": 2, "start": "2026-08-27T18:00:00-05:00", "end": "2026-08-27T19:00:00-05:00"}}
            ]
        }}]})
        .to_string();
        let seed = Date::constant(2026, 8, 19);
        crate::eventledger::record_judged_verdict(
            &vault, "localist:77:1", "Weekly Meeting", seed, "unsure", "", Some(CHECK_JID),
        )
        .unwrap();
        crate::eventledger::record_answer(
            &vault, "localist:77:1", "Weekly Meeting", seed, "obligation", "quinn", Some(CHECK_JID),
        )
        .unwrap();
        crate::eventledger::record_judged_verdict(
            &vault, "localist:77:2", "Weekly Meeting", seed, "unsure", "", None,
        )
        .unwrap();
        std::fs::create_dir_all(vault.join("archive")).unwrap();
        pystr::write_text(
            &vault.join("archive").join("event-check-weekly-meeting-2026-08-20.md"),
            "---\ntype: approval\nkind: event-check\ntitle: \"Weekly Meeting\"\nstatus: executed\n\
             source_uid: \"localist:77:1\"\nseries_uid: \"localist:77\"\nevents:\n- \"localist:77:1\"\n\
             proposed_at: 2026-08-19\nfirst_proposed_at: 2026-08-19\nexpires: 2026-08-20\n\
             snooze_until: null\ncreated_by: events\n---\n\nbody\n",
        )
        .unwrap();
        let fetch = |_: &str| Ok(feed.clone());
        let fetchers = Fetchers { calendar: None, events: Some(&fetch), series: None };
        run_with(&vault, Some("2026-08-26"), "manual", None, fetchers).unwrap();

        let seen = pystr::read_text(&vault.join("state").join("events-seen.md")).unwrap();
        assert!(
            seen.contains("- localist:77:2 · Weekly Meeting · verdict:obligation · by:quinn · answered 2026-08-26"),
            "{seen}"
        );
        let roster = pystr::read_text(&vault.join("state").join("events.md")).unwrap();
        let relevant = roster.split("## Everything else").next().unwrap_or("");
        assert!(relevant.contains("`localist:77:2`"), "{roster}");
        // Answered, so never asked.
        assert!(md_names(&vault.join("approvals"), "event-check-").is_empty());
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

    // --- the series stash (P14, spec §4.2) ----------------------------------------------------

    /// A one-shot loopback HTTP server (CLAUDE.md's loopback-only rule; mirrors
    /// `cloud_contract.rs`'s `loopback`, kept local since these tests read no header or request
    /// line — only `calendar_fetcher`'s own return value and what it left in the stash). Answers
    /// exactly one request, at 200, with `body`; hands back the bare `http://127.0.0.1:<port>`
    /// base and the server thread's handle, so a test can join it and see a panic if the read or
    /// write ever failed.
    fn loopback_once(body: String) -> (String, std::thread::JoinHandle<()>) {
        loopback_once_status(200, body)
    }

    /// [`loopback_once`] with the status line spelled out (M1, review round 1: a non-2xx reply
    /// from the cloud route).
    fn loopback_once_status(status: u16, body: String) -> (String, std::thread::JoinHandle<()>) {
        let listener = std::net::TcpListener::bind("127.0.0.1:0").expect("bind the loopback listener");
        let port = listener.local_addr().expect("the listener has an address").port();
        let handle = std::thread::spawn(move || {
            use std::io::{BufRead, Write};
            let (stream, _) = listener.accept().expect("accept the one connection");
            let mut reader = std::io::BufReader::new(stream.try_clone().expect("clone the stream"));
            loop {
                let mut line = String::new();
                let n = reader.read_line(&mut line).unwrap_or(0);
                if n == 0 || line == "\r\n" || line == "\n" {
                    break;
                }
            }
            let mut stream = reader.into_inner();
            let response = format!(
                "HTTP/1.1 {status} X\r\ncontent-type: application/octet-stream\r\ncontent-length: {}\r\n\
                 connection: close\r\n\r\n{body}",
                body.len()
            );
            stream.write_all(response.as_bytes()).expect("write the response");
            stream.flush().expect("flush the response");
        });
        (format!("http://127.0.0.1:{port}"), handle)
    }

    fn cloud_config(api_base: String) -> crate::cloudmodel::CloudConfig {
        crate::cloudmodel::CloudConfig {
            api_base,
            anon_key: "anon-not-a-secret".to_string(),
            session_credential_target: "knowlu/test-profile/session".to_string(),
            account_id: "acct-1".to_string(),
        }
    }

    #[test]
    fn fetchers_default_has_no_calendar_events_or_series() {
        let fetchers = Fetchers::default();
        assert!(fetchers.calendar.is_none());
        assert!(fetchers.events.is_none());
        assert!(fetchers.series.is_none());
    }

    #[test]
    fn the_calendar_closure_stashes_google_series_by_url() {
        let series = serde_json::json!({
            "calendars_read": ["google:3b9e0c1d2a4f5e60"],
            "items": [{
                "calendar": "google:3b9e0c1d2a4f5e60",
                "id": "4k2q9x7m1abc",
                "title": "CS 100",
                "location": "",
                "description": "Room 101",
                "event_type": "default",
                "first": "2026-08-19T12:00:00-05:00",
                "recurrence": ["RRULE:FREQ=WEEKLY;BYDAY=MO,WE,FR;UNTIL=20261205T055959Z"],
                "instances": [{"start": "2026-09-23T17:00:00Z", "end": "2026-09-23T17:50:00Z"}]
            }]
        });
        let ics = "BEGIN:VCALENDAR\r\nEND:VCALENDAR\r\n";
        let body = crate::ledger::dumps_value(
            &serde_json::json!({ "ics": ics, "source": "google_calendar", "series": series }),
        );
        let (base, handle) = loopback_once(body);
        let cloud = Some(crate::cloudmodel::CloudClient::new(
            &cloud_config(format!("{base}/functions/v1")),
            "jwt-not-a-secret",
        ));
        let stash: SeriesStash = RefCell::new(BTreeMap::new());
        let fetch = calendar_fetcher(&cloud, &stash);
        let got = fetch("cloud:google").expect("the service answered");
        assert!(got.contains("BEGIN:VCALENDAR"), "{got}");
        match stash.borrow().get("cloud:google") {
            Some(StashEntry::Google(value)) => assert_eq!(value, &series),
            other => panic!("expected a stashed Google series at \"cloud:google\", got {other:?}"),
        }
        handle.join().expect("the loopback thread did not panic");
    }

    /// I1 (review round 1): a `cloud:google` reply with no `series` field — an old server that
    /// predates §4.1, or a new one whose series budget ran out or whose series gathering failed
    /// (§4.1: "`series` is omitted") — must stash **nothing** for that feed. §3.3: "a Google reply
    /// that carried no `series` field — keeps its series and its date untouched"; §4.3: "`series`
    /// absent → `None` → that feed's series keep their previous state". The `ics` this reply
    /// carries is `toIcs` output (no `RRULE`); stashing it as `StashEntry::Ics` would let P16's
    /// `refresh_series` derive a fresh, empty series read and mint a phantom ICS-keyed calendar
    /// beside the real `google:<hash>` keys — exactly the state drift the two specs rule out.
    #[test]
    fn the_calendar_closure_stashes_nothing_for_google_without_series() {
        let ics = "BEGIN:VCALENDAR\r\nBEGIN:VEVENT\r\nUID:g1\r\nEND:VEVENT\r\nEND:VCALENDAR\r\n";
        let body = crate::ledger::dumps_value(
            &serde_json::json!({ "ics": ics, "source": "google_calendar" }),
        );
        let (base, handle) = loopback_once(body);
        let cloud = Some(crate::cloudmodel::CloudClient::new(
            &cloud_config(format!("{base}/functions/v1")),
            "jwt-not-a-secret",
        ));
        let stash: SeriesStash = RefCell::new(BTreeMap::new());
        let fetch = calendar_fetcher(&cloud, &stash);
        let got = fetch("cloud:google").expect("the service answered");
        assert_eq!(got, ics);
        assert!(stash.borrow().get("cloud:google").is_none(), "{:?}", stash.borrow());
        handle.join().expect("the loopback thread did not panic");
    }

    /// `cloud:personal` gets no `series` field (spec §4.1): the closure stashes the ICS text under
    /// the feed's own url rather than leaving the stash empty, so a later refresh still has ICS
    /// text to derive series from. Unlike `cloud:google` (see the test above), `personal` never
    /// carries series at all — this is its only branch, not a fallback from a missing one.
    #[test]
    fn the_calendar_closure_stashes_ics_text_for_a_cloud_feed_with_no_series() {
        let ics = "BEGIN:VCALENDAR\r\nBEGIN:VEVENT\r\nUID:p1\r\nEND:VEVENT\r\nEND:VCALENDAR\r\n";
        let body = crate::ledger::dumps_value(
            &serde_json::json!({ "ics": ics, "source": "personal_ics" }),
        );
        let (base, handle) = loopback_once(body);
        let cloud = Some(crate::cloudmodel::CloudClient::new(
            &cloud_config(format!("{base}/functions/v1")),
            "jwt-not-a-secret",
        ));
        let stash: SeriesStash = RefCell::new(BTreeMap::new());
        let fetch = calendar_fetcher(&cloud, &stash);
        let got = fetch("cloud:personal").expect("the service answered");
        assert_eq!(got, ics);
        match stash.borrow().get("cloud:personal") {
            Some(StashEntry::Ics(text)) => assert_eq!(text, ics),
            other => panic!("expected stashed ICS text at \"cloud:personal\", got {other:?}"),
        }
        handle.join().expect("the loopback thread did not panic");
    }

    #[test]
    fn the_calendar_closure_stashes_ics_text_for_a_direct_feed() {
        let ics = "BEGIN:VCALENDAR\r\nBEGIN:VEVENT\r\nUID:d1\r\nEND:VEVENT\r\nEND:VCALENDAR\r\n".to_string();
        let (base, handle) = loopback_once(ics.clone());
        let cloud: Option<crate::cloudmodel::CloudClient> = None;
        let stash: SeriesStash = RefCell::new(BTreeMap::new());
        let fetch = calendar_fetcher(&cloud, &stash);
        let url = format!("{base}/cal.ics");
        let got = fetch(&url).expect("the feed answered");
        assert_eq!(got, ics);
        match stash.borrow().get(&url) {
            Some(StashEntry::Ics(text)) => assert_eq!(text, &ics),
            other => panic!("expected stashed ICS text at {url:?}, got {other:?}"),
        }
        handle.join().expect("the loopback thread did not panic");
    }

    /// Two failing branches, neither of which touches the stash: a direct feed whose scheme
    /// `calfeed::fetch_ics` cannot even parse a connector for, and a `cloud:` feed on a vault with
    /// no account (`cloud: None`) — the "no account on this vault" arm `run`'s own closure takes
    /// on a vault that has never signed in.
    #[test]
    fn a_failed_fetch_stashes_nothing() {
        let stash: SeriesStash = RefCell::new(BTreeMap::new());
        let cloud: Option<crate::cloudmodel::CloudClient> = None;
        let fetch = calendar_fetcher(&cloud, &stash);

        let err = fetch("unreachable://example.test/cal.ics").unwrap_err();
        assert!(!err.is_empty());
        assert!(stash.borrow().is_empty(), "{:?}", stash.borrow());

        let err = fetch("cloud:google").unwrap_err();
        assert_eq!(err, "no account on this vault");
        assert!(stash.borrow().is_empty(), "{:?}", stash.borrow());
    }

    /// M1 (review round 1): the cloud route itself can fail two ways `fetch_calendar` surfaces as
    /// `Err` — a non-2xx status, and a 200 whose body carries no `ics` field — and the `?` in
    /// `calendar_fetcher` must return before either ever reaches an `insert`.
    #[test]
    fn a_failed_cloud_fetch_stashes_nothing() {
        let (base, handle) =
            loopback_once_status(409, crate::ledger::dumps_value(&serde_json::json!({ "error": "conflict" })));
        let cloud = Some(crate::cloudmodel::CloudClient::new(
            &cloud_config(format!("{base}/functions/v1")),
            "jwt-not-a-secret",
        ));
        let stash: SeriesStash = RefCell::new(BTreeMap::new());
        let fetch = calendar_fetcher(&cloud, &stash);
        assert!(fetch("cloud:google").is_err());
        assert!(stash.borrow().is_empty(), "{:?}", stash.borrow());
        handle.join().expect("the loopback thread did not panic");

        let (base, handle) =
            loopback_once(crate::ledger::dumps_value(&serde_json::json!({ "source": "google_calendar" })));
        let cloud = Some(crate::cloudmodel::CloudClient::new(
            &cloud_config(format!("{base}/functions/v1")),
            "jwt-not-a-secret",
        ));
        let stash: SeriesStash = RefCell::new(BTreeMap::new());
        let fetch = calendar_fetcher(&cloud, &stash);
        assert!(fetch("cloud:google").is_err(), "a body with no ics field must be Err");
        assert!(stash.borrow().is_empty(), "{:?}", stash.borrow());
        handle.join().expect("the loopback thread did not panic");
    }

    // --- P16: the commitment passes inside `rank` ----------------------------------------------

    /// Spec §8 test 5 (M7): `vault-full` ranked through the production `run` (its feed has no
    /// `ics_url`, so nothing is fetched) gains no series file, no plan file, no `commitments/` and
    /// no card, and its `calendar` step says exactly what it said before P16 (captured on a7efe3d).
    #[test]
    fn rank_on_vault_full_writes_no_new_state() {
        let vault = fixture("p16full", "tests/fixtures/vault-full");
        run(&vault, Some("2026-08-28"), "manual", None).unwrap();
        assert!(!vault.join("state").join("calendar-series.json").exists());
        assert!(!vault.join("state").join("plan.json").exists());
        assert!(!vault.join("commitments").exists());
        assert!(md_names(&vault.join("approvals"), "commitment-check-").is_empty());
        let recs = run_records(&vault);
        let calendar = step_of(recs.last().unwrap(), "calendar");
        assert_eq!(calendar["message"], serde_json::json!("personal: no ics_url"), "{calendar:?}");
        assert_eq!(calendar["result"], serde_json::json!("WARN"), "{calendar:?}");
        let _ = std::fs::remove_dir_all(&vault);
    }

    /// Two feeds (the Google grant and one direct address), all invented.
    const P16_FEEDS: &str = "timezone: America/Chicago\ncalendars:\n  - name: google\n    ics_url: 'cloud:google'\n  - name: personal\n    ics_url: https://calendar.example.test/personal.ics\n";
    const P16_ICS_URL: &str = "https://calendar.example.test/personal.ics";
    const P16_GOOGLE: &str = "google:0a1b2c3d4e5f6071";
    /// A Monday; every series below starts on it.
    const P16_MONDAY: Date = Date::constant(2026, 9, 7);

    /// A migrated scratch vault with a flat 08:00–18:00 week template, the two feeds, one active
    /// task (so `today.md` carries its capacity line) and `courses/cs-100.md` (so `CS 100`
    /// classifies as a class).
    fn p16_vault(name: &str) -> PathBuf {
        let vault = scratch(name);
        for dir in ["config", "tasks", "courses"] {
            std::fs::create_dir_all(vault.join(dir)).unwrap();
        }
        let write = |rel: &str, text: &str| pystr::write_text(&vault.join(rel), text).unwrap();
        write(
            "config/week_template.yaml",
            "day_start: '08:00'\nday_end: '18:00'\nclasses:\n  mon: []\n  tue: []\n  wed: []\n  thu: []\n  fri: []\n  sat: []\n  sun: []\n",
        );
        write("config/ingest.yaml", P16_FEEDS);
        write("tasks/essay.md", "---\ntitle: Invented essay\ndue: 2026-10-30\neffort_hours: 2\nimportance: 3\n---\n");
        write("courses/cs-100.md", "---\nid: course_00000000c1\ntitle: \"CS 100 Intro to Computing\"\ncode: \"CS 100\"\n---\n");
        seed_migrated(&vault);
        // Q1-c: P16_MONDAY is the vault's day 2, so checks are filed and asks (day 3+) are not.
        seed_journal_day(&vault, "2026-09-06");
        vault
    }

    /// One journal record stamped noon UTC on `day` (Q1-c): the vault's first day for
    /// `commitments::vault_day`. A pinned-`ts` `create` record, the shape `commitment_note` already
    /// appends; `verify_tail` only replays `set` records, so it is inert.
    fn seed_journal_day(vault: &Path, day: &str) {
        let mut spec = crate::journal::NewRecord::new("create", "archive/_migrated.md", "system:migration", "cli");
        spec.ts = Some(format!("{day}T12:00:00.000Z"));
        let mut rec = crate::journal::make_record(spec).unwrap();
        Journal::new(vault).append(&mut rec).unwrap();
    }

    /// The student already said CS 100 has no set meeting times (a `card:cs-100` marker), so a
    /// P16 test about check cards sees no ask (phase-2 spec §5).
    fn decline_cs100_ask(vault: &Path) {
        let ctx = WriteContext::new("quinn", "dashboard");
        crate::commitments::create_marker(vault, &crate::commitments::ask_key("cs-100"), &ctx, &mut Journal::new(vault)).unwrap();
    }

    fn p16_day(offset: i64) -> Date {
        P16_MONDAY.checked_add(jiff::Span::new().days(offset)).unwrap()
    }

    /// One Google `series` item: weekly on `days` (0 = Monday) from `P16_MONDAY`, `start`–`end`
    /// (CDT), for `weeks` weeks.
    fn google_item(id: &str, title: &str, days: &[i64], start: &str, end: &str, weeks: i64) -> serde_json::Value {
        let names = ["MO", "TU", "WE", "TH", "FR", "SA", "SU"];
        let mut instances = Vec::new();
        for week in 0..weeks {
            for d in days {
                let day = p16_day(week * 7 + d);
                instances.push(serde_json::json!({
                    "start": format!("{day}T{start}:00-05:00"),
                    "end": format!("{day}T{end}:00-05:00"),
                }));
            }
        }
        let byday: Vec<&str> = days.iter().map(|d| names[*d as usize]).collect();
        serde_json::json!({
            "calendar": P16_GOOGLE,
            "id": id,
            "title": title,
            "location": "",
            "description": "",
            "event_type": "default",
            "first": instances[0]["start"].clone(),
            "recurrence": [format!("RRULE:FREQ=WEEKLY;BYDAY={}", byday.join(","))],
            "instances": instances,
        })
    }

    /// A fresh, complete read of `P16_GOOGLE` returning `items`.
    fn google_entry(items: Vec<serde_json::Value>) -> StashEntry {
        StashEntry::Google(serde_json::json!({ "calendars_read": [P16_GOOGLE], "items": items }))
    }

    fn cs100_item() -> serde_json::Value {
        google_item("cs100aa", "CS 100", &[0, 2, 4], "12:00", "12:50", 8)
    }

    /// `run_with` on `day` with `entries` injected into the stash (no network: every feed answers
    /// an empty calendar and stashes nothing itself).
    fn rank_p16(vault: &Path, day: Date, entries: Vec<(&str, StashEntry)>) -> RunOutcome {
        let stash: SeriesStash =
            RefCell::new(entries.into_iter().map(|(url, entry)| (url.to_string(), entry)).collect());
        let empty = |_: &str| Ok("BEGIN:VCALENDAR\nEND:VCALENDAR\n".to_string());
        let fetchers = Fetchers { calendar: Some(&empty), events: None, series: Some(&stash) };
        run_with(vault, Some(&day.to_string()), "manual", None, fetchers).unwrap()
    }

    fn commitment_note(vault: &Path, file: &str, front: &str) {
        std::fs::create_dir_all(vault.join("commitments")).unwrap();
        let text = format!("---\n{front}---\n\nInvented.\n");
        pystr::write_text(&vault.join("commitments").join(file), &text).unwrap();
        // A real confirmed commitments note is minted by `create_confirmed` (actor
        // `commitments::CARD_ACTOR`) and is already in the journal the moment any rank first sees
        // it. Journal this fixture the same way, or `detect_external` reads it as a hand edit
        // nobody journaled and judge-once (final-fix-report m3) locks every field a
        // change-detection test needs to see move.
        let (meta, _) = crate::models::split_frontmatter(&text).unwrap();
        let id = crate::yaml::get(&meta, "id").and_then(crate::yaml::text).unwrap();
        let whole = crate::yaml::to_json(&serde_yaml_ng::Value::Mapping(meta));
        let rel_path = format!("commitments/{file}");
        let mut spec = crate::journal::NewRecord::new("create", &rel_path, crate::commitments::CARD_ACTOR, "cli");
        spec.id = Some(&id);
        spec.new = whole;
        spec.ts = Some("2026-09-06T12:00:00.000Z".into());
        let mut rec = crate::journal::make_record(spec).unwrap();
        Journal::new(vault).append(&mut rec).unwrap();
    }

    /// `CS 100`, confirmed, exactly as `cs100_item` meets.
    const CS100_NOTE: &str = "id: cmt_00000000a1\ntype: commitment\nkind: class\nlevel: hard\ntitle: \"CS 100\"\n\
        course: cs-100\nmeets: [{days: [mon, wed, fri], start: \"12:00\", end: \"12:50\"}]\n\
        source_uid: \"gcal-series:cs100aa\"\nstatus: confirmed\n";

    /// `PH 106`, confirmed at 13:00–14:45; its series (`ph106_moved`) now meets an hour later.
    const PH106_NOTE: &str = "id: cmt_00000000a2\ntype: commitment\nkind: class\nlevel: hard\ntitle: \"PH 106\"\n\
        meets: [{days: [tue, thu], start: \"13:00\", end: \"14:45\"}]\n\
        source_uid: \"gcal-series:ph106aa\"\nstatus: confirmed\n";

    fn ph106_moved() -> serde_json::Value {
        google_item("ph106aa", "PH 106", &[1, 3], "14:00", "15:45", 8)
    }

    const PLANNING_NOTE: &str = "id: cmt_00000000b1\ntype: commitment\nkind: planning-day\nstatus: confirmed\n\
        window: [{days: [mon, tue, wed, thu, fri], start: \"08:00\", end: \"22:00\"}]\n";

    fn checks(vault: &Path, folder: &str) -> Vec<String> {
        md_names(&vault.join(folder), "commitment-check-")
    }

    fn capacity_today(vault: &Path) -> f64 {
        let text = page(vault);
        let at = text.find("Capacity today: ").unwrap_or_else(|| panic!("no capacity line: {text}"));
        let rest = &text[at + "Capacity today: ".len()..];
        rest[..rest.find('h').unwrap()].parse().unwrap()
    }

    fn plan_json(vault: &Path) -> String {
        std::fs::read_to_string(vault.join("state").join("plan.json")).unwrap()
    }

    #[test]
    fn rank_files_a_commitment_check_for_an_injected_class_series_and_counts_it_pending() {
        let vault = p16_vault("p16card");
        rank_p16(&vault, P16_MONDAY, vec![("cloud:google", google_entry(vec![cs100_item()]))]);
        let cards = checks(&vault, "approvals");
        assert_eq!(cards.len(), 1, "{cards:?}");
        let card = vault.join("approvals").join(&cards[0]);
        assert_eq!(meta_str(&card, "title"), "CS 100 · Mon/Wed/Fri 12–12:50pm · a class?");
        assert_eq!(meta_str(&card, "source_uid"), "gcal-series:cs100aa");
        assert!(page(&vault).contains("**Approvals: 1 pending**"), "{}", page(&vault));
        let recs = run_records(&vault);
        assert_eq!(step_of(recs.last().unwrap(), "approvals")["counts"]["pending"], serde_json::json!(1));
        assert!(vault.join("state").join("calendar-series.json").is_file());
        // R18: a proposal is never a note.
        assert!(!vault.join("commitments").exists());
        let _ = std::fs::remove_dir_all(&vault);
    }

    /// Phase-2 D2: on the vault's first day `rank` files no proposal card, because the confirm
    /// screen is asking at the same moment. From day 2 it files them as before.
    #[test]
    fn no_commitment_check_is_filed_on_the_vaults_first_day() {
        let vault = p16_vault("p2day1");
        // `p16_vault`'s journal starts 2026-09-06 (Q1-c): that date is day 1.
        rank_p16(&vault, Date::constant(2026, 9, 6), vec![("cloud:google", google_entry(vec![cs100_item()]))]);
        assert!(checks(&vault, "approvals").is_empty(), "{:?}", checks(&vault, "approvals"));
        assert!(vault.join("state").join("calendar-series.json").is_file(), "the series file still refreshes");
        rank_p16(&vault, P16_MONDAY, vec![("cloud:google", google_entry(vec![cs100_item()]))]);
        assert_eq!(checks(&vault, "approvals").len(), 1, "day 2 files the class card");
        let _ = std::fs::remove_dir_all(&vault);
    }

    /// Phase-2 spec §5: from the vault's day 3, a course with no class gets one ask card, counted
    /// into the pending line like a check card.
    #[test]
    fn rank_files_an_ask_for_a_course_with_no_class_from_day_three() {
        let vault = p16_vault("p2ask");
        rank_p16(&vault, P16_MONDAY, Vec::new());
        assert!(md_names(&vault.join("approvals"), "commitment-ask-").is_empty(), "P16_MONDAY is day 2");
        rank_p16(&vault, p16_day(1), Vec::new());
        assert_eq!(md_names(&vault.join("approvals"), "commitment-ask-"), ["commitment-ask-when-does-cs-100-intro-to-computing-meet.md"]);
        assert!(page(&vault).contains("**Approvals: 1 pending**"), "{}", page(&vault));
        let _ = std::fs::remove_dir_all(&vault);
    }

    /// Phase-3 D7 (Plan ruling R2-h): a vault holding a registrar term that has not started yet
    /// files no ask card; the day after the term starts, the ask comes as before. ART 110 is a
    /// registrar row with no vault course, so CS 100 stays uncovered and only the gate holds it.
    #[test]
    fn asks_wait_for_the_registrars_term_to_start() {
        let vault = p16_vault("p3gate");
        let text = include_str!("../tests/fixtures/registrar/banner-ua-registration.json");
        let parsed = crate::registrar::parse_banner(&serde_json::from_str(text).unwrap(), "ua").unwrap();
        let mut art = parsed.series.into_iter().find(|s| s.source_uid.ends_with("40006")).unwrap();
        art.first = Some(Date::constant(2026, 9, 14));
        crate::commitments::refresh_series(&vault, &[(art.calendar.clone(), vec![art])], P16_MONDAY);
        rank_p16(&vault, p16_day(1), Vec::new());
        assert!(md_names(&vault.join("approvals"), "commitment-ask-").is_empty(), "the term starts on 09-14");
        rank_p16(&vault, Date::constant(2026, 9, 15), Vec::new());
        assert_eq!(md_names(&vault.join("approvals"), "commitment-ask-"), ["commitment-ask-when-does-cs-100-intro-to-computing-meet.md"]);
        let _ = std::fs::remove_dir_all(&vault);
    }

    /// Phase-3 D5 end to end: a fetch confirms CS 100 and its lab (R24); the next fetch drops the
    /// lab and moves the lecture's room; the next `rank` files an end card for the lab (the day
    /// before the fetch, R2-c) and a change card for the room (R2-g). The notes were created by
    /// `quinn` (R3-c); the cards come because the p2 judge-once fix counts only a later `set`.
    #[test]
    fn a_registrar_refetch_files_an_end_card_and_a_change_card_at_the_next_rank() {
        let vault = p16_vault("p3refetch");
        let ctx = WriteContext::new("quinn", "dashboard");
        let text = include_str!("../tests/fixtures/registrar/banner-ua-registration.json");
        let first = crate::registrar::run(&vault, text, "ua", Date::constant(2026, 9, 1), &ctx, &mut Journal::new(&vault)).unwrap();
        assert_eq!((first.confirmed, first.proposed), (2, 2), "CS 100 and its lab; ENGL 101 and ART 110 have no course here");
        let mut later: serde_json::Value = serde_json::from_str(text).unwrap();
        later["data"].as_array_mut().unwrap().retain(|r| r["courseReferenceNumber"] != "40002");
        later["data"][0]["meetingsFaculty"][0]["meetingTime"]["room"] = serde_json::json!("102");
        crate::registrar::run(&vault, &later.to_string(), "ua", P16_MONDAY, &ctx, &mut Journal::new(&vault)).unwrap();
        rank_p16(&vault, P16_MONDAY, Vec::new());
        let cards: Vec<String> = checks(&vault, "approvals")
            .iter()
            .map(|n| std::fs::read_to_string(vault.join("approvals").join(n)).unwrap())
            .collect();
        assert!(cards.iter().any(|t| t.contains("commitments/cs-100-lab.md") && t.contains("2026-09-06")), "{cards:#?}");
        assert!(cards.iter().any(|t| t.contains("commitments/cs-100.md") && t.contains("Invented Hall 102")), "{cards:#?}");
        let _ = std::fs::remove_dir_all(&vault);
    }

    #[test]
    fn commitment_checks_take_only_what_the_events_pass_left() {
        let vault = p16_vault("p16budget");
        // This test ranks on 2026-08-26, before `p16_vault`'s Q1-c seed (2026-09-06): without an
        // earlier record the vault's day-1 gate (Q1-b) would eat the class card this test is
        // about. Antedate the journal so 2026-08-26 is day 2.
        seed_journal_day(&vault, "2026-08-25");
        pystr::write_text(
            &vault.join("config").join("events.yaml"),
            "sources:\n  - name: campus\n    type: ics\n    url: unreachable://x\n    enabled: true\n",
        )
        .unwrap();
        pystr::write_text(&vault.join("config").join("planning.yaml"), "daily_approval_budget: 15\n").unwrap();
        pending_proposals(&vault, 12);
        let ics = judged_feed(&vault, 2);
        let events = |_: &str| Ok(ics.clone());
        let items = vec![
            cs100_item(),
            google_item("cs100lab", "CS 100 Lab", &[1], "15:00", "16:50", 8),
            google_item("chess01", "Chess Club", &[3], "18:00", "19:00", 8),
        ];
        let stash: SeriesStash = RefCell::new(BTreeMap::from([("cloud:google".to_string(), google_entry(items))]));
        let empty = |_: &str| Ok("BEGIN:VCALENDAR\nEND:VCALENDAR\n".to_string());
        let today = Date::constant(2026, 8, 26);
        for _ in 0..2 {
            let fetchers = Fetchers { calendar: Some(&empty), events: Some(&events), series: Some(&stash) };
            run_with(&vault, Some("2026-08-26"), "manual", None, fetchers).unwrap();
            // 12 proposals + a 2-event digest leave 1: the class takes it, and nothing is deferred.
            assert_eq!(digest_uids(&vault).len(), 2);
            assert_eq!(checks(&vault, "approvals").len(), 1, "{:?}", checks(&vault, "approvals"));
            assert_eq!(count_proposals_created(&vault, today), 15);
            let recs = run_records(&vault);
            assert_eq!(step_of(recs.last().unwrap(), "approvals")["counts"]["deferred"], serde_json::json!(0));
            for i in 0..12 {
                let path = vault.join("approvals").join(format!("task-{i}.md"));
                assert_eq!(meta_str(&path, "status"), "pending", "task-{i}");
            }
        }
        let card = vault.join("approvals").join(&checks(&vault, "approvals")[0]);
        assert_eq!(meta_str(&card, "source_uid"), "gcal-series:cs100aa");
        let _ = std::fs::remove_dir_all(&vault);
    }

    #[test]
    fn rank_subtracts_a_confirmed_class_from_capacity() {
        let vault = p16_vault("p16capacity");
        rank_p16(&vault, P16_MONDAY, Vec::new());
        let before = capacity_today(&vault);
        commitment_note(
            &vault,
            "seminar.md",
            "id: cmt_00000000a3\ntype: commitment\nkind: class\nlevel: hard\ntitle: \"Invented Seminar\"\n\
             meets: [{days: [mon], start: \"15:00\", end: \"16:00\"}]\nstatus: confirmed\n",
        );
        rank_p16(&vault, P16_MONDAY, Vec::new());
        let after = capacity_today(&vault);
        assert!((before - after - 1.0).abs() < 1e-9, "before {before}, after {after}");
        let _ = std::fs::remove_dir_all(&vault);
    }

    #[test]
    fn rank_withdraws_a_card_whose_series_left_the_file() {
        let vault = p16_vault("p16withdraw");
        decline_cs100_ask(&vault);
        rank_p16(&vault, P16_MONDAY, vec![("cloud:google", google_entry(vec![cs100_item()]))]);
        let filed = checks(&vault, "approvals");
        assert_eq!(filed.len(), 1);
        // One unrelated card, two days old on the drop run: after the withdrawal it is the only
        // pending card and the oldest (P16 fix round 1, I1).
        pystr::write_text(
            &vault.join("approvals").join("task-reply.md"),
            "---\ntype: approval\nkind: task\ntitle: Reply to advisor\nstatus: pending\n\
             proposed_at: 2026-09-19\nfirst_proposed_at: 2026-09-19\nexpires: 2026-09-30\n\
             snooze_until: null\n---\n\nInvented.\n",
        )
        .unwrap();
        // Fourteen days of fresh reads without it: the series leaves the file, the card goes.
        rank_p16(&vault, p16_day(14), vec![("cloud:google", google_entry(Vec::new()))]);
        assert!(checks(&vault, "approvals").is_empty());
        assert_eq!(checks(&vault, "archive"), filed);
        assert_eq!(meta_str(&vault.join("archive").join(&filed[0]), "status"), "superseded");
        // The withdrawn card is not counted on the run that withdrew it, nor is its age.
        let text = page(&vault);
        assert!(text.contains("**Approvals: 1 pending** (oldest 2d)"), "{text}");
        let recs = run_records(&vault);
        assert_eq!(step_of(recs.last().unwrap(), "approvals")["counts"]["pending"], serde_json::json!(1));
        let _ = std::fs::remove_dir_all(&vault);
    }

    /// C1: a change card is not withdrawn by the next ranks and not filed again.
    #[test]
    fn a_change_card_survives_two_more_ranks() {
        let vault = p16_vault("p16change");
        commitment_note(&vault, "ph-106.md", PH106_NOTE);
        for _ in 0..3 {
            rank_p16(&vault, P16_MONDAY, vec![("cloud:google", google_entry(vec![ph106_moved()]))]);
            let cards = checks(&vault, "approvals");
            assert_eq!(cards.len(), 1, "{cards:?}");
            let card = vault.join("approvals").join(&cards[0]);
            assert_eq!(meta_str(&card, "status"), "pending");
            assert_eq!(meta_str(&card, "target"), "commitments/ph-106.md");
            assert!(checks(&vault, "archive").is_empty());
        }
        let _ = std::fs::remove_dir_all(&vault);
    }

    /// C1: a proposal card, a window card and a change card, ranked on three consecutive days with
    /// the same stash, are each filed once, never archived, and charged only on day one.
    #[test]
    fn no_card_is_withdrawn_and_refiled_across_three_ranks() {
        let vault = p16_vault("p16three");
        commitment_note(&vault, "ph-106.md", PH106_NOTE);
        let items = || {
            vec![
                cs100_item(),
                ph106_moved(),
                google_item("wake01", "Wake up", &[0, 1, 2, 3, 4], "07:00", "07:15", 8),
                google_item("bed01", "Bedtime", &[0, 1, 2, 3, 4], "22:30", "22:45", 8),
            ]
        };
        for offset in 0..3 {
            rank_p16(&vault, p16_day(offset), vec![("cloud:google", google_entry(items()))]);
            let cards = checks(&vault, "approvals");
            assert_eq!(cards.len(), 3, "day {offset}: {cards:?}");
            let mut kinds: Vec<&str> = Vec::new();
            for name in &cards {
                let card = vault.join("approvals").join(name);
                assert_eq!(meta_str(&card, "status"), "pending", "{name}");
                assert_eq!(meta_str(&card, "first_proposed_at"), "2026-09-07", "{name}");
                assert_eq!(meta_str(&card, "proposed_at"), "2026-09-07", "{name}");
                kinds.push(match (meta_str(&card, "target").is_empty(), meta_str(&card, "source_uid")) {
                    (false, _) => "change",
                    (true, key) if key.starts_with("window:") => "window",
                    (true, _) => "proposal",
                });
            }
            kinds.sort();
            assert_eq!(kinds, vec!["change", "proposal", "window"], "day {offset}");
            assert!(checks(&vault, "archive").is_empty(), "day {offset}");
            assert_eq!(count_proposals_created(&vault, p16_day(offset)), if offset == 0 { 3 } else { 0 });
        }
        let _ = std::fs::remove_dir_all(&vault);
    }

    /// Ranks `CS100_NOTE`'s vault on `P16_MONDAY` with its series, then on days 7 and 14 with fresh
    /// reads that no longer return it (the drop run is day 14).
    fn age_out_cs100(vault: &Path, drop_budget: Option<i64>) {
        commitment_note(vault, "cs-100.md", CS100_NOTE);
        rank_p16(vault, P16_MONDAY, vec![("cloud:google", google_entry(vec![cs100_item()]))]);
        assert!(checks(vault, "approvals").is_empty(), "the note matches its series");
        rank_p16(vault, p16_day(7), vec![("cloud:google", google_entry(Vec::new()))]);
        assert!(checks(vault, "approvals").is_empty());
        if let Some(budget) = drop_budget {
            pystr::write_text(
                &vault.join("config").join("planning.yaml"),
                &format!("daily_approval_budget: {budget}\n"),
            )
            .unwrap();
        }
        rank_p16(vault, p16_day(14), vec![("cloud:google", google_entry(Vec::new()))]);
    }

    /// The last instance the day-0 read held: Friday Oct 2, inside `[Sep 7, Oct 5)`.
    fn assert_one_end_card(vault: &Path) {
        let cards = checks(vault, "approvals");
        assert_eq!(cards.len(), 1, "{cards:?}");
        let card = vault.join("approvals").join(&cards[0]);
        assert_eq!(meta_str(&card, "title"), "CS 100 ends Oct 2 · update?");
        assert_eq!(meta_str(&card, "target"), "commitments/cs-100.md");
        let meta = crate::ids::read_meta(&card).unwrap();
        let change = crate::yaml::to_json(crate::yaml::get(&meta, "change").unwrap());
        assert_eq!(change, serde_json::json!({ "until": "2026-10-02" }));
    }

    /// I1: an aged-out series files exactly one end card, `until` its last instance.
    #[test]
    fn an_aged_out_series_files_exactly_one_end_card_with_its_last_instance() {
        let vault = p16_vault("p16ended");
        age_out_cs100(&vault, None);
        assert_one_end_card(&vault);
        rank_p16(&vault, p16_day(15), vec![("cloud:google", google_entry(Vec::new()))]);
        assert_one_end_card(&vault);
        let _ = std::fs::remove_dir_all(&vault);
    }

    /// I1: the drop run had no budget; the next day's rank files the card from `ended`.
    #[test]
    fn an_end_card_starved_by_the_budget_is_filed_on_the_next_run() {
        let vault = p16_vault("p16starved");
        age_out_cs100(&vault, Some(0));
        assert!(checks(&vault, "approvals").is_empty());
        let series = std::fs::read_to_string(vault.join("state").join("calendar-series.json")).unwrap();
        assert!(series.contains("gcal-series:cs100aa"), "{series}");
        pystr::write_text(&vault.join("config").join("planning.yaml"), "daily_approval_budget: 15\n").unwrap();
        rank_p16(&vault, p16_day(15), vec![("cloud:google", google_entry(Vec::new()))]);
        assert_one_end_card(&vault);
        let _ = std::fs::remove_dir_all(&vault);
    }

    /// I2: one series reaching the file through the Google grant and a direct ICS feed (a Google
    /// UID keeps its `gcal-series:` key on both routes) is one card.
    #[test]
    fn one_series_from_google_and_ics_files_one_card() {
        let vault = p16_vault("p16twin");
        let ics = "BEGIN:VCALENDAR\nVERSION:2.0\nPRODID:-//Invented//EN\nBEGIN:VEVENT\nUID:cs100aa@google.com\n\
                   SUMMARY:CS 100\nDTSTART;TZID=America/Chicago:20260907T120000\n\
                   DTEND;TZID=America/Chicago:20260907T125000\nRRULE:FREQ=WEEKLY;BYDAY=MO,WE,FR\n\
                   END:VEVENT\nEND:VCALENDAR\n";
        rank_p16(
            &vault,
            P16_MONDAY,
            vec![
                ("cloud:google", google_entry(vec![cs100_item()])),
                (P16_ICS_URL, StashEntry::Ics(ics.to_string())),
            ],
        );
        let series = std::fs::read_to_string(vault.join("state").join("calendar-series.json")).unwrap();
        assert_eq!(series.matches("\"gcal-series:cs100aa\"").count(), 2, "held under both calendars: {series}");
        let cards = checks(&vault, "approvals");
        assert_eq!(cards.len(), 1, "{cards:?}");
        assert_eq!(meta_str(&vault.join("approvals").join(&cards[0]), "source_uid"), "gcal-series:cs100aa");
        let _ = std::fs::remove_dir_all(&vault);
    }

    #[test]
    fn rank_records_the_template_window_when_plan_json_is_missing() {
        let vault = p16_vault("p16plan1");
        commitment_note(&vault, "planning-day.md", PLANNING_NOTE);
        rank_p16(&vault, P16_MONDAY, Vec::new());
        assert_eq!(plan_json(&vault), "{\"date\": \"2026-09-07\", \"end\": \"18:00\", \"start\": \"08:00\"}\n");
        let _ = std::fs::remove_dir_all(&vault);
    }

    #[test]
    fn rank_keeps_the_days_first_window_across_runs() {
        let vault = p16_vault("p16plan2");
        commitment_note(&vault, "planning-day.md", PLANNING_NOTE);
        rank_p16(&vault, P16_MONDAY, Vec::new());
        let first = plan_json(&vault);
        commitment_note(&vault, "planning-day.md", &PLANNING_NOTE.replace("22:00", "23:00"));
        rank_p16(&vault, P16_MONDAY, Vec::new());
        assert_eq!(plan_json(&vault), first);
        let _ = std::fs::remove_dir_all(&vault);
    }

    #[test]
    fn rank_records_todays_window_on_a_new_day() {
        let vault = p16_vault("p16plan3");
        commitment_note(&vault, "planning-day.md", PLANNING_NOTE);
        pystr::write_text(
            &vault.join("state").join("plan.json"),
            "{\"date\": \"2026-09-06\", \"end\": \"18:00\", \"start\": \"08:00\"}\n",
        )
        .unwrap();
        rank_p16(&vault, P16_MONDAY, Vec::new());
        assert_eq!(plan_json(&vault), "{\"date\": \"2026-09-07\", \"end\": \"22:00\", \"start\": \"08:00\"}\n");
        let _ = std::fs::remove_dir_all(&vault);
    }

    /// M2 (fix round 1): `plan.json` goes through temp-then-rename, and no temp file is left.
    #[test]
    fn rank_writes_plan_json_whole_and_leaves_no_temp_file() {
        let vault = p16_vault("p16plan5");
        commitment_note(&vault, "planning-day.md", PLANNING_NOTE);
        rank_p16(&vault, P16_MONDAY, Vec::new());
        assert!(plan_json(&vault).ends_with("}\n"));
        let temps = md_like(&vault.join("state"), ".tmp");
        assert!(temps.is_empty(), "{temps:?}");
        let _ = std::fs::remove_dir_all(&vault);
    }

    fn md_like(dir: &Path, needle: &str) -> Vec<String> {
        std::fs::read_dir(dir)
            .map(|rd| {
                rd.flatten()
                    .map(|e| e.file_name().to_string_lossy().to_string())
                    .filter(|n| n.contains(needle))
                    .collect()
            })
            .unwrap_or_default()
    }

    /// M1 (fix round 1, controller ruling): a code two courses claim is only worth a warning when
    /// a series this run actually carries that code. A vault with the clash and nothing else keeps
    /// a clean `calendar` step.
    #[test]
    fn a_course_code_clash_with_no_series_leaves_the_calendar_step_clean() {
        let vault = p16_vault("p16codeclash");
        pystr::write_text(&vault.join("config").join("ingest.yaml"), "timezone: America/Chicago\n").unwrap();
        pystr::write_text(
            &vault.join("courses").join("cs-100-honors.md"),
            "---\nid: course_00000000c2\ntitle: \"CS 100 Honors\"\ncode: \"CS 100\"\n---\n",
        )
        .unwrap();
        let out = rank_p16(&vault, P16_MONDAY, Vec::new());
        let calendar = out.steps.iter().find(|s| s.name == "calendar").unwrap();
        assert_eq!((calendar.result, calendar.message.as_str()), ("ok", ""));
        let _ = std::fs::remove_dir_all(&vault);
    }

    /// M1's other half: a series that carries the ambiguous code surfaces the warning.
    #[test]
    fn a_course_code_clash_a_series_hits_warns_in_the_calendar_step() {
        let vault = p16_vault("p16codehit");
        pystr::write_text(
            &vault.join("courses").join("cs-100-honors.md"),
            "---\nid: course_00000000c2\ntitle: \"CS 100 Honors\"\ncode: \"CS 100\"\n---\n",
        )
        .unwrap();
        let out = rank_p16(&vault, P16_MONDAY, vec![("cloud:google", google_entry(vec![cs100_item()]))]);
        let calendar = out.steps.iter().find(|s| s.name == "calendar").unwrap();
        assert_eq!(calendar.result, "WARN");
        assert!(calendar.message.contains("code CS100: claimed by both"), "{}", calendar.message);
        let _ = std::fs::remove_dir_all(&vault);
    }

    /// Every file under `vault`, by vault-relative path, with its bytes.
    fn tree_bytes(vault: &Path) -> BTreeMap<String, Vec<u8>> {
        fn walk(root: &Path, dir: &Path, out: &mut BTreeMap<String, Vec<u8>>) {
            for entry in std::fs::read_dir(dir).unwrap().flatten() {
                let path = entry.path();
                if path.is_dir() {
                    walk(root, &path, out);
                } else {
                    let rel = path.strip_prefix(root).unwrap().to_string_lossy().replace('\\', "/");
                    out.insert(rel, std::fs::read(&path).unwrap());
                }
            }
        }
        let mut out = BTreeMap::new();
        walk(vault, vault, &mut out);
        out
    }

    /// M3 (fix round 1): a second rank on the same day with the same stash changes nothing but
    /// the run's own record of itself — the run ledger, the runner log and `today.md`'s
    /// generated-at line. Cards, the series file, `plan.json`, the journal and every note are
    /// byte-identical. `state/.journal-index.json` (`detect_external`'s git-ignored cache) is
    /// the one exception on the second run: it catches up to the records the first run wrote
    /// after `detect_external` had run (its cards). A third rank leaves even that unchanged.
    #[test]
    fn a_second_rank_with_the_same_inputs_changes_no_file() {
        let vault = p16_vault("p16twice");
        commitment_note(&vault, "ph-106.md", PH106_NOTE);
        commitment_note(&vault, "planning-day.md", PLANNING_NOTE);
        let entries = || vec![("cloud:google", google_entry(vec![cs100_item(), ph106_moved()]))];
        rank_p16(&vault, P16_MONDAY, entries());
        assert_eq!(checks(&vault, "approvals").len(), 2, "a proposal and a change card");
        let normalise = |mut tree: BTreeMap<String, Vec<u8>>| {
            tree.retain(|rel, _| !rel.starts_with("state/runs/") && rel != "state/runner-log.md");
            if let Some(page) = tree.get_mut("state/today.md") {
                let text = String::from_utf8(page.clone()).unwrap();
                let kept: Vec<&str> = text.lines().filter(|l| !l.contains("Generated")).collect();
                *page = kept.join("\n").into_bytes();
            }
            tree
        };
        const INDEX: &str = "state/.journal-index.json";
        let first = normalise(tree_bytes(&vault));
        rank_p16(&vault, P16_MONDAY, entries());
        let second = normalise(tree_bytes(&vault));
        assert_eq!(first.keys().collect::<Vec<_>>(), second.keys().collect::<Vec<_>>());
        for (rel, bytes) in first.iter().filter(|(rel, _)| rel.as_str() != INDEX) {
            assert!(second[rel] == *bytes, "{rel} changed on the second rank");
        }
        rank_p16(&vault, P16_MONDAY, entries());
        let third = normalise(tree_bytes(&vault));
        assert_eq!(second.keys().collect::<Vec<_>>(), third.keys().collect::<Vec<_>>());
        for (rel, bytes) in &second {
            assert!(third[rel] == *bytes, "{rel} changed on the third rank");
        }
        let _ = std::fs::remove_dir_all(&vault);
    }

    #[test]
    fn rank_writes_no_plan_json_without_a_planning_day_note() {
        let vault = p16_vault("p16plan4");
        rank_p16(&vault, P16_MONDAY, vec![("cloud:google", google_entry(vec![cs100_item()]))]);
        assert!(!vault.join("state").join("plan.json").exists());
        let _ = std::fs::remove_dir_all(&vault);
    }

    /// P8's carry-forward: a malformed series file warns in the `calendar` step (no new step), and
    /// that run withdraws nothing — a card whose key is not in this run's (fresh-only) file stays.
    #[test]
    fn a_malformed_series_file_warns_in_the_calendar_step() {
        let vault = p16_vault("p16malformed");
        std::fs::create_dir_all(vault.join("state")).unwrap();
        std::fs::write(vault.join("state").join("calendar-series.json"), "{not json").unwrap();
        std::fs::create_dir_all(vault.join("approvals")).unwrap();
        let card = vault.join("approvals").join("commitment-check-old-seminar.md");
        pystr::write_text(
            &card,
            "---\ntype: approval\nkind: commitment-check\ntitle: \"Old Seminar · Tue 3–4pm · a class?\"\n\
             status: pending\nsource_uid: \"gcal-series:elsewhere\"\nproposed_at: 2026-09-06\n\
             first_proposed_at: 2026-09-06\nexpires: null\nsnooze_until: null\ncreated_by: agent:commitments\n\
             ---\n\nInvented.\n",
        )
        .unwrap();
        let out = rank_p16(&vault, P16_MONDAY, vec![("cloud:google", google_entry(Vec::new()))]);
        let names: Vec<&str> = out.steps.iter().map(|s| s.name).collect();
        assert_eq!(names, vec!["passes", "tasks", "calendar", "events", "approvals"]);
        let calendar = out.steps.iter().find(|s| s.name == "calendar").unwrap();
        assert_eq!(calendar.result, "WARN");
        assert!(
            calendar.message.contains("series file: state/calendar-series.json malformed"),
            "{}",
            calendar.message
        );
        assert!(card.is_file(), "a bad read is never \"gone\"");
        assert_eq!(meta_str(&card, "status"), "pending");
        let _ = std::fs::remove_dir_all(&vault);
    }

    // --- P19: the `commitments` command (R14, spec §5.1, §6.5) --------------------------------

    /// Every file under `vault`, path relative with forward slashes, to its bytes — so a test can
    /// diff before and after a call and name every path that changed, not just assert a few paths
    /// stayed absent.
    fn snapshot(vault: &Path) -> BTreeMap<String, Vec<u8>> {
        fn walk(dir: &Path, root: &Path, out: &mut BTreeMap<String, Vec<u8>>) {
            for entry in std::fs::read_dir(dir).into_iter().flatten().flatten() {
                let path = entry.path();
                if path.is_dir() {
                    walk(&path, root, out);
                } else {
                    let rel = path
                        .strip_prefix(root)
                        .unwrap()
                        .to_string_lossy()
                        .replace('\\', "/");
                    out.insert(rel, std::fs::read(&path).unwrap_or_default());
                }
            }
        }
        let mut out = BTreeMap::new();
        if vault.is_dir() {
            walk(vault, vault, &mut out);
        }
        out
    }

    /// Every path present in `before` or `after` whose bytes differ (an addition, a removal or a
    /// changed file), sorted.
    fn changed_paths(before: &BTreeMap<String, Vec<u8>>, after: &BTreeMap<String, Vec<u8>>) -> Vec<String> {
        let mut changed: std::collections::BTreeSet<String> = std::collections::BTreeSet::new();
        for (path, bytes) in after {
            if before.get(path) != Some(bytes) {
                changed.insert(path.clone());
            }
        }
        for path in before.keys() {
            if !after.contains_key(path) {
                changed.insert(path.clone());
            }
        }
        changed.into_iter().collect()
    }

    /// The command builds the same fetcher-and-stash shape `rank` does, refreshes the series file
    /// from an injected class series, and prints the one proposal it found — writing nothing else
    /// under the vault: no note in `commitments/`, no card in `approvals/`, no journal record
    /// (`state/journal/` never appears in the diff at all).
    #[test]
    fn commitments_command_prints_proposals_and_writes_no_note_card_or_journal() {
        let vault = p16_vault("p19cards");
        let before = snapshot(&vault);
        let stash: SeriesStash =
            RefCell::new(BTreeMap::from([("cloud:google".to_string(), google_entry(vec![cs100_item()]))]));
        let empty = |_: &str| Ok("BEGIN:VCALENDAR\nEND:VCALENDAR\n".to_string());
        let fetchers = Fetchers { calendar: Some(&empty), events: None, series: Some(&stash) };
        let report = commitments_report_with(&vault, Some(&P16_MONDAY.to_string()), fetchers);

        assert_eq!(report.proposals.len(), 1, "{:?}", report.proposals);
        let p = &report.proposals[0];
        assert_eq!(p.title, "CS 100");
        assert_eq!(p.kind, "class");
        assert_eq!(p.source_uid, "gcal-series:cs100aa");
        assert!(report.warnings.is_empty(), "{:?}", report.warnings);

        let after = snapshot(&vault);
        assert_eq!(
            changed_paths(&before, &after),
            vec!["state/calendar-series.json".to_string(), "state/calendar-series.lock".to_string()],
            "the commitments command must write nothing else under the vault (the lock: final review I2)"
        );
        assert!(!vault.join("commitments").exists(), "R18: a proposal is never a note");
        assert!(md_names(&vault.join("approvals"), "commitment-check-").is_empty(), "no card either");
        let _ = std::fs::remove_dir_all(&vault);
    }

    /// A vault with no configured feed and no injected stash gets no series, so `refresh_series`
    /// writes nothing (`fresh` is empty) and `proposals` returns the empty list — no routines, no
    /// window either.
    #[test]
    fn commitments_command_with_no_feed_prints_an_empty_list() {
        let vault = scratch("p19nofeed");
        std::fs::create_dir_all(vault.join("config")).unwrap();
        pystr::write_text(
            &vault.join("config").join("week_template.yaml"),
            "day_start: '08:00'\nday_end: '18:00'\nclasses:\n  mon: []\n  tue: []\n  wed: []\n  thu: []\n  fri: []\n  sat: []\n  sun: []\n",
        )
        .unwrap();
        let before = snapshot(&vault);
        let report = commitments_report_with(&vault, Some(&P16_MONDAY.to_string()), Fetchers::default());
        assert!(report.proposals.is_empty(), "{:?}", report.proposals);
        assert!(report.warnings.is_empty(), "{:?}", report.warnings);
        assert_eq!(changed_paths(&before, &snapshot(&vault)), Vec::<String>::new());
        let _ = std::fs::remove_dir_all(&vault);
    }

    /// Plan review M14: two separate failures, each named in `warnings` rather than raised —
    /// a malformed `config/ingest.yaml` (this crate's own read, since `calfeed::calendar_entries`
    /// swallows a bad config silently and correctly), and a feed that fails to fetch. Both leave
    /// the vault exactly as they found it and the command still hands back its (empty) report.
    #[test]
    fn commitments_command_exits_0_on_an_unreadable_config_and_a_failed_fetch() {
        // A malformed config/ingest.yaml: an unterminated flow sequence.
        let vault = scratch("p19badconfig");
        std::fs::create_dir_all(vault.join("config")).unwrap();
        pystr::write_text(&vault.join("config").join("ingest.yaml"), "calendars: [\n  - name: x\n").unwrap();
        let before = snapshot(&vault);
        let report = commitments_report_with(&vault, Some(&P16_MONDAY.to_string()), Fetchers::default());
        assert!(report.proposals.is_empty(), "{:?}", report.proposals);
        assert!(
            report.warnings.iter().any(|w| w.starts_with("config unreadable:")),
            "{:?}",
            report.warnings
        );
        assert_eq!(changed_paths(&before, &snapshot(&vault)), Vec::<String>::new());
        let _ = std::fs::remove_dir_all(&vault);

        // A feed that is configured but fails to fetch.
        let vault = scratch("p19badfetch");
        std::fs::create_dir_all(vault.join("config")).unwrap();
        pystr::write_text(
            &vault.join("config").join("ingest.yaml"),
            "calendars:\n  - name: personal\n    ics_url: https://calendar.example.test/personal.ics\n",
        )
        .unwrap();
        let before = snapshot(&vault);
        let stash: SeriesStash = RefCell::new(BTreeMap::new());
        let boom = |_: &str| Err("connection refused".to_string());
        let fetchers = Fetchers { calendar: Some(&boom), events: None, series: Some(&stash) };
        let report = commitments_report_with(&vault, Some(&P16_MONDAY.to_string()), fetchers);
        assert!(report.proposals.is_empty(), "{:?}", report.proposals);
        assert!(
            report.warnings.iter().any(|w| w.contains("personal") && w.contains("connection refused")),
            "{:?}",
            report.warnings
        );
        assert_eq!(changed_paths(&before, &snapshot(&vault)), Vec::<String>::new());
        let _ = std::fs::remove_dir_all(&vault);
    }

    /// M2 (fix round 1): `for_cards: false` is the one behaviour that separates this command's
    /// proposal list from `rank`'s own card-filing pass — `commitments::proposals`'s
    /// `office_hours_are_never_filed`/`office_hours_is_proposed_for_the_screen_not_for_cards`
    /// tests pin the library function; this pins that the command actually passes `false`, since
    /// flipping it to `true` would still pass every other test in this file.
    #[test]
    fn commitments_command_includes_office_hours_because_for_cards_is_false() {
        let vault = p16_vault("p19officehours");
        let stash: SeriesStash = RefCell::new(BTreeMap::from([(
            "cloud:google".to_string(),
            google_entry(vec![
                cs100_item(),
                google_item("cs100oh", "CS 100 Office Hours", &[3], "15:00", "16:00", 8),
            ]),
        )]));
        let empty = |_: &str| Ok("BEGIN:VCALENDAR\nEND:VCALENDAR\n".to_string());
        let fetchers = Fetchers { calendar: Some(&empty), events: None, series: Some(&stash) };
        let report = commitments_report_with(&vault, Some(&P16_MONDAY.to_string()), fetchers);
        assert_eq!(report.proposals.len(), 2, "{:?}", report.proposals);
        assert!(
            report
                .proposals
                .iter()
                .any(|p| p.kind == "office-hours" && p.title == "CS 100 Office Hours"),
            "office hours must be in the command's list even though `rank` would never file a card \
             for one: {:?}",
            report.proposals
        );
        let _ = std::fs::remove_dir_all(&vault);
    }

    /// Phase-2 spec §2: the command names each course with no class row. `p16_vault` has
    /// `courses/cs-100.md`: with no feed it is uncovered; once CS 100's series is proposed, it is not.
    #[test]
    fn commitments_command_reports_courses_with_no_class_row() {
        let vault = p16_vault("p2uncovered");
        let report = commitments_report_with(&vault, Some(&P16_MONDAY.to_string()), Fetchers::default());
        assert_eq!(report.uncovered.len(), 1, "{:?}", report.uncovered);
        assert_eq!(report.uncovered[0].slug, "cs-100");
        assert_eq!(report.uncovered[0].title, "CS 100 Intro to Computing");
        let stash: SeriesStash = RefCell::new([("cloud:google".to_string(), google_entry(vec![cs100_item()]))].into_iter().collect());
        let empty = |_: &str| Ok("BEGIN:VCALENDAR\nEND:VCALENDAR\n".to_string());
        let fetchers = Fetchers { calendar: Some(&empty), events: None, series: Some(&stash) };
        let report = commitments_report_with(&vault, Some(&P16_MONDAY.to_string()), fetchers);
        assert!(report.uncovered.is_empty(), "{:?}", report.uncovered);
        let _ = std::fs::remove_dir_all(&vault);
    }

    /// M3 (fix round 1): `proposal_json` carries an explicit `"window"` boolean (`Proposal::is_window`)
    /// rather than making the phase-2 screen infer it from `commitments::WINDOW_PREFIX`.
    #[test]
    fn proposal_json_carries_an_explicit_window_flag() {
        let ordinary = crate::commitments::Proposal {
            kind: "class".to_string(),
            level: crate::commitments::Level::Hard,
            title: "CS 100".to_string(),
            course: Some("cs-100".to_string()),
            meets: Vec::new(),
            where_: None,
            from: None,
            until: None,
            source_uid: "gcal-series:cs100aa".to_string(),
        };
        assert_eq!(proposal_json(&ordinary)["window"], serde_json::json!(false));

        let window = crate::commitments::Proposal {
            kind: crate::commitments::PLANNING_DAY.to_string(),
            level: crate::commitments::Level::Optional,
            title: "Your day".to_string(),
            course: None,
            meets: Vec::new(),
            where_: None,
            from: None,
            until: None,
            source_uid: format!("{}mon-wake,tue-wake", crate::commitments::WINDOW_PREFIX),
        };
        assert_eq!(proposal_json(&window)["window"], serde_json::json!(true));
    }

    /// M4 (fix round 1): a `calendars:` value that parses as YAML but is not a list is exactly the
    /// shape `calendars_feeds` (private to `calfeed.rs`) still "iterates" without error — a string
    /// one character at a time — and `calendar_entries` then filters every resulting non-mapping
    /// "feed" out silently, so the command would otherwise answer "no proposals, no warnings" for
    /// a config it could not use at all.
    #[test]
    fn commitments_command_warns_on_a_wrongly_shaped_calendars_value() {
        let vault = scratch("p19badshape");
        std::fs::create_dir_all(vault.join("config")).unwrap();
        pystr::write_text(
            &vault.join("config").join("ingest.yaml"),
            "calendars: \"https://calendar.example.test/personal.ics\"\n",
        )
        .unwrap();
        let report = commitments_report_with(&vault, Some(&P16_MONDAY.to_string()), Fetchers::default());
        assert!(report.proposals.is_empty(), "{:?}", report.proposals);
        assert!(
            report.warnings.iter().any(|w| w.starts_with("config: calendars is not a list")),
            "{:?}",
            report.warnings
        );
        let _ = std::fs::remove_dir_all(&vault);

        // A falsy `calendars:` (no feeds intended at all) must stay silent — the same shape
        // `calendars_feeds` reads as `Ok(None)`, not a config problem.
        let vault2 = scratch("p19nocalendars");
        std::fs::create_dir_all(vault2.join("config")).unwrap();
        pystr::write_text(&vault2.join("config").join("ingest.yaml"), "calendars: null\n").unwrap();
        let report2 = commitments_report_with(&vault2, Some(&P16_MONDAY.to_string()), Fetchers::default());
        assert!(report2.warnings.is_empty(), "{:?}", report2.warnings);
        let _ = std::fs::remove_dir_all(&vault2);
    }

    // --- I1 (fix round 1): the runtime trap ------------------------------------------------------
    //
    // The lexical scan in `cloud_contract.rs` proves no forbidden substring appears in source
    // text; this proves something a scan cannot — over one concrete `rank`, against a server that
    // would happily answer a judgment request too, none is ever sent (spelled out, not written as
    // the literal endpoint text, so this file itself stays clean of it for that same scan).

    /// A loopback server that answers every request with a valid `/ingest-calendar` reply and
    /// records the request line it saw (via `tx`), rather than accepting exactly one connection
    /// and refusing the rest — a stray extra request during the run under test is caught, not left
    /// to hang a listener that only ever answered once. Stops on a `/__stop__` request, which the
    /// caller sends itself once the run under test has returned.
    fn judge_trap_server() -> (String, std::sync::mpsc::Receiver<String>, u16, std::thread::JoinHandle<()>) {
        let listener = std::net::TcpListener::bind("127.0.0.1:0").expect("bind the loopback listener");
        let port = listener.local_addr().expect("the listener has an address").port();
        let (tx, rx) = std::sync::mpsc::channel();
        let handle = std::thread::spawn(move || {
            use std::io::{BufRead, Write};
            loop {
                let Ok((stream, _)) = listener.accept() else { break };
                let mut reader = std::io::BufReader::new(stream.try_clone().expect("clone the stream"));
                let mut request_line = String::new();
                let _ = reader.read_line(&mut request_line);
                loop {
                    let mut line = String::new();
                    let n = reader.read_line(&mut line).unwrap_or(0);
                    if n == 0 || line == "\r\n" || line == "\n" {
                        break;
                    }
                }
                let mut stream = reader.into_inner();
                if request_line.contains("__stop__") {
                    let body = "{}";
                    let _ = write!(
                        stream,
                        "HTTP/1.1 200 X\r\ncontent-type: application/json\r\ncontent-length: {}\r\n\
                         connection: close\r\n\r\n{}",
                        body.len(),
                        body
                    );
                    break;
                }
                let _ = tx.send(request_line.trim_end().to_string());
                let body = crate::ledger::dumps_value(&serde_json::json!({
                    "ics": "BEGIN:VCALENDAR\r\nEND:VCALENDAR\r\n",
                    "source": "google_calendar",
                }));
                let _ = write!(
                    stream,
                    "HTTP/1.1 200 X\r\ncontent-type: application/json\r\ncontent-length: {}\r\n\
                     connection: close\r\n\r\n{}",
                    body.len(),
                    body
                );
            }
        });
        (format!("http://127.0.0.1:{port}/functions/v1"), rx, port, handle)
    }

    /// I1: a `rank` (`run_with`, the same production function `run` calls) against a vault that
    /// already carries a confirmed commitment, a series file and a pending `commitment-check` card
    /// — so `commitment_passes` exercises `approvals::withdraw_stale`, `calfeed`, `weekcal` and
    /// `commitments::proposals` fully, not just an empty vault's early returns — never sends a
    /// request to anything but `/ingest-calendar`, even with a server on the other end that would
    /// happily answer a judgment request instead of refusing it. The assertion below is a
    /// **whitelist** (every request must be the calendar transport), which is what actually rules
    /// a judgment request out — a blacklist checked against the literal endpoint text would have
    /// to write that text into this very file, which the extended `rank_cannot_reach_a_judgment_endpoint`
    /// scan (I1) then correctly refuses.
    #[test]
    fn rank_never_sends_a_request_to_a_judge_endpoint_on_a_vault_with_commitments_series_and_pending_cards() {
        let vault = scratch("p19trap");
        for dir in ["config", "tasks", "courses", "approvals"] {
            std::fs::create_dir_all(vault.join(dir)).unwrap();
        }
        let write = |rel: &str, text: &str| pystr::write_text(&vault.join(rel), text).unwrap();
        write(
            "config/week_template.yaml",
            "day_start: '08:00'\nday_end: '18:00'\nclasses:\n  mon: []\n  tue: []\n  wed: []\n  thu: []\n  fri: []\n  sat: []\n  sun: []\n",
        );
        write(
            "config/ingest.yaml",
            "timezone: America/Chicago\ncalendars:\n  - name: google\n    ics_url: 'cloud:google'\n",
        );
        write(
            "courses/cs-100.md",
            "---\nid: course_00000000c1\ntitle: \"CS 100 Intro to Computing\"\ncode: \"CS 100\"\n---\n",
        );
        // "commitments": CS 100 is already confirmed.
        commitment_note(&vault, "cs100.md", CS100_NOTE);
        // "pending cards": a card for a series this run will not fetch.
        write(
            "approvals/commitment-check-old-seminar.md",
            "---\ntype: approval\nkind: commitment-check\ntitle: \"Old Seminar · Tue 3–4pm · a class?\"\n\
             status: pending\nsource_uid: \"gcal-series:elsewhere\"\nproposed_at: 2026-09-06\n\
             first_proposed_at: 2026-09-06\nexpires: null\nsnooze_until: null\ncreated_by: agent:commitments\n\
             ---\n\nInvented.\n",
        );
        // "series": the series file, written offline (no network at all) before the trap run.
        let entries: BTreeMap<String, StashEntry> =
            BTreeMap::from([("cloud:google".to_string(), google_entry(vec![cs100_item()]))]);
        let (fresh, _) = normalise_stash(&vault, &entries, &vault_zone(&vault), P16_MONDAY);
        let (_file, _warn) = crate::commitments::refresh_series(&vault, &fresh, P16_MONDAY);
        assert!(vault.join("state").join("calendar-series.json").is_file());

        // The runtime trap: a real round trip through the same `calendar_fetcher` `rank` itself
        // builds, against a server that would answer any path at all rather than refuse one.
        let (base, rx, port, handle) = judge_trap_server();
        let cloud = Some(crate::cloudmodel::CloudClient::new(&cloud_config(base), "jwt-not-a-secret"));
        let stash: SeriesStash = RefCell::new(BTreeMap::new());
        let fetch = calendar_fetcher(&cloud, &stash);
        let fetchers = Fetchers { calendar: Some(&fetch), events: None, series: Some(&stash) };
        run_with(&vault, Some(&P16_MONDAY.to_string()), "manual", None, fetchers)
            .expect("a vault this shaped must still rank");

        // Tell the server to stop, then check what it actually saw.
        {
            use std::io::Write;
            if let Ok(mut s) = std::net::TcpStream::connect(("127.0.0.1", port)) {
                let _ = write!(s, "GET /__stop__ HTTP/1.1\r\nHost: x\r\nConnection: close\r\n\r\n");
            }
        }
        handle.join().expect("the loopback thread did not panic");
        let requests: Vec<String> = rx.try_iter().collect();
        assert!(!requests.is_empty(), "the calendar fetch must have reached the loopback server");
        for r in &requests {
            // A whitelist, not a blacklist against the literal endpoint text (see the doc comment
            // above): every request this run made was the calendar transport and nothing else.
            assert!(r.contains("/ingest-calendar"), "unexpected request, not the calendar transport: {r}");
        }
        let _ = std::fs::remove_dir_all(&vault);
    }
}
