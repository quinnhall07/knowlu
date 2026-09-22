//! Enrichment (Knowlu spec §5.5 step 1, §5.1): the cloud routine's step 3, run locally at the app's
//! own slots, so the routine wakes up and finds nothing left to do.
//!
//! **Starve, never switch.** Nothing here turns the routine off or edits it. Both producers running
//! at once is the expected state for weeks: whichever runs second finds `needs_enrichment: false`
//! and writes nothing, because `write`'s F4 rule makes an identical value a no-op with no record.
//!
//! **This command always exits 0.** A missing runtime, a missing model, a runtime that will not
//! start and a model that answers nonsense are all normal outcomes reported on stdout. A non-zero
//! exit would set `RunSummary.engine_ok = false` in the app's scheduler, which paints the tray amber
//! and puts the slot into retry backoff twice a day forever — for a machine that has simply not
//! downloaded a 2 GB file. `coursework` exits 0 for the same reason; `ingest` does not, and that
//! difference is deliberate on both sides.
//!
//! **What it writes, and as whom.** Exactly the fields the routine's step 3 writes — `course`,
//! `effort_hours`, `effort_confidence: low`, `importance`, `importance_reason`, and
//! `needs_enrichment: false` — through `write` with `judged: true`, `propose: true` and
//! `inputs: {source_uid, title_seen}`, as **`agent:knowlu.enrich`**. Five of the six are in
//! `provenance::JUDGED_FIELDS_TASK`, so judge-once protects them: a field Quinn set in the console
//! is never overwritten and comes back as a `kind: amend` card instead.
//!
//! **On a real vault the `judgment:` block usually names four of those five, not five.**
//! `ingest::NOTE_TEMPLATE` seeds every freshly ingested task at `effort_confidence: low` already,
//! and this module's own answer for that field is always `"low"` too — so `write`'s F4 rule (an
//! identical value is a no-op with no record) swallows it before it ever reaches `written`, and
//! therefore before it ever reaches the block. `effort_confidence` only appears there when a note
//! started somewhere else, which in practice means "never, on a note this pass created" and "only
//! if something upstream changes it" otherwise. This is expected, not a bug in either module.
//!
//! **What it deliberately does not write.** The routine also appends a `course_map` pin to
//! `config/ingest.yaml` when it attributes a gradebook item. Learning a rule from a model's output
//! and promoting it is spec §5.4's loop, which is plan **3b's**; a config line written here would be
//! one that loop then had to argue with. Nothing is lost meanwhile — the routine keeps writing pins
//! until it is turned off, and an unpinned item is simply re-attributed each time.
//!
//! **No server, no load phase (spec §5.3 as amended 2026-09-07).** Task 1's spike measured a
//! `Server` (start once, call many times) against one `llama-cli` process per judgment and found the
//! server win shrink, not grow, as the batch got bigger — 2.15x at thirty items, converging to
//! 2.21x, against a pre-committed 3x bar. So there is no server here: `run_lines` resolves the
//! runtime and the model once per run and hands `judge_task` a fresh `runtime::PerCall` for every
//! item, which is what actually spawns and kills a process.

use std::path::{Path, PathBuf};
use std::time::Duration;

use serde_yaml_ng::Value;

use crate::journal::Journal;
use crate::judge::{self, Heuristics, Item, NoRules, Outcome};
use crate::write::{self, WriteContext, WriteOpts};

/// The agent actor for plan 3a's writes. **`agent:` prefix, not `knowlu/`**: `provenance::is_agent`
/// is `starts_with("agent:")` and nothing else, so the spec §5.2 spelling would silently skip
/// judge-once and write no provenance block. 3b uses `agent:knowlu.events`, 3c `agent:knowlu.gmail`.
pub const ACTOR: &str = "agent:knowlu.enrich";

/// How many items one run judges by default. A semester's Blackboard ingest can flag a hundred at
/// once and each is a model call of a second or two; the rest are taken at the next slot, and the
/// summary line says how many are left, so nothing is lost and no slot is held for an hour.
pub const DEFAULT_LIMIT: usize = 50;

/// M1 (final fix wave): a wall-clock budget for one run's items, checked **before starting each
/// item** — not a per-call bound, which `runtime::CALL_TIMEOUT` already is. `DEFAULT_LIMIT *
/// CALL_TIMEOUT` is 100 minutes against `scheduler::CHILD_TIMEOUT`'s 20-minute slot budget:
/// eleven consecutive timed-out calls are enough for `run_child` to return `-2`, which sets
/// `engine_ok = false` and paints the tray amber with retry backoff twice a day forever — exactly
/// the outcome `knowlu-engine judge`'s always-exit-0 rule exists to prevent, on exactly the machine
/// the design was written for. Fifteen minutes plus one more `CALL_TIMEOUT` in flight when the
/// budget is checked (17 minutes worst case) still lands inside `CHILD_TIMEOUT` with headroom.
/// Untaken items — whether cut by `limit` or by this budget — are folded into one
/// `"{left} left for the next slot"` line; nothing is lost, and no slot is held for an hour.
pub const BATCH_BUDGET: Duration = Duration::from_secs(15 * 60);

/// `Copy` so the cloud arm can build a variant with one field changed (`..*opts`); every field is
/// already a shared reference or a plain value, so a copy is what a borrow would have been.
#[derive(Clone, Copy)]
pub struct Options<'a> {
    pub via: &'a str,
    pub run_id: Option<&'a str>,
    /// Where the app installed the runtime. `None` falls back to `KNOWLU_RUNTIME` and then to a
    /// sibling of the exe (`runtime::resolve`).
    pub runtime: Option<&'a Path>,
    pub model: Option<&'a Path>,
    /// The profile's `judgments\` directory. `None` writes no log — which is what a hand-typed
    /// `knowlu-engine judge` does, since only the app knows where a profile's app data is (§5.4).
    pub log_dir: Option<&'a Path>,
    pub limit: usize,
    /// M1: the wall-clock budget for this run, checked before each item starts. Production always
    /// passes [`BATCH_BUDGET`] (`run`); tests shrink it to prove the cutoff without sleeping for
    /// fifteen minutes.
    pub budget: Duration,
}

/// Every task note flagged `needs_enrichment: true`, plus one line per note that had to be skipped.
///
/// Path order (`approvals::sorted_md`), so a bounded run takes the same items in the same order
/// twice. **A note with no `id:` is skipped and named**: judge-once is `journal::human_set(id,
/// field)`, so without an id the protection is void and writing anyway would be exactly the silent
/// overwrite the whole rule exists to prevent.
///
/// Read with `std::fs::read_to_string`, not `pystr::read_text` — the same choice
/// `models::Task::from_file_with_meta` makes, since Python's task loader never ran notes through
/// universal-newline translation.
pub fn pending(vault: &Path) -> (Vec<Item>, Vec<String>) {
    let mut items = Vec::new();
    let mut skipped = Vec::new();
    for path in crate::approvals::sorted_md(&vault.join("tasks")) {
        let Ok(text) = std::fs::read_to_string(&path) else { continue };
        let Ok((meta, body)) = crate::models::split_frontmatter(&text) else { continue };
        if !matches!(crate::yaml::get(&meta, "needs_enrichment"), Some(Value::Bool(true))) {
            continue;
        }
        let name = path.file_name().map(|n| n.to_string_lossy().to_string()).unwrap_or_default();
        let id = crate::yaml::opt_text(crate::yaml::get(&meta, "id")).filter(|s| crate::ids::is_id(s));
        let Some(id) = id else {
            skipped.push(format!("{name}: no id, so judge-once cannot protect it"));
            continue;
        };
        let title = crate::yaml::opt_text(crate::yaml::get(&meta, "title")).unwrap_or_default();
        if title.trim().is_empty() {
            skipped.push(format!("{name}: no title, so there is nothing to judge from"));
            continue;
        }
        items.push(Item {
            id,
            rel_path: crate::ids::rel(vault, &path),
            title,
            body: crate::pystr::strip(&body).to_string(),
            source_uid: crate::yaml::opt_text(crate::yaml::get(&meta, "source_uid")).unwrap_or_default(),
            created_by: crate::yaml::opt_text(crate::yaml::get(&meta, "created_by")).unwrap_or_default(),
            course: crate::yaml::opt_text(crate::yaml::get(&meta, "course")),
            due: crate::yaml::get(&meta, "due").filter(|v| !matches!(v, Value::Null)).map(crate::approvals::plain),
            effort_hours: crate::yaml::opt_f64(crate::yaml::get(&meta, "effort_hours"), 1.0),
            effort_source: crate::yaml::opt_text(crate::yaml::get(&meta, "effort_source")).unwrap_or_default(),
        });
    }
    (items, skipped)
}

/// The frontmatter literals one verdict becomes.
///
/// Free text goes through `write::to_literal` so a colon or a quote in `importance_reason` survives
/// as YAML; the numbers are their own spelling. **`needs_enrichment=false` only when the verdict is
/// complete** — a partial answer leaves the flag set, so the routine (while it still runs) or a
/// later slot with a model finishes the job.
///
/// `effort_confidence` is `low` **only when the effort came from the model** (`v.tier == 3`) —
/// m5 (final fix wave): before this gate the field was pushed whenever `effort_hours` was present
/// at all, which would rewrite `coursework`'s vendor-authored `high` down to `low` on any note
/// that also carried an hour estimate, while leaving the vendor's hours untouched. Unreachable
/// today (only `ingest::NOTE_TEMPLATE` seeds `needs_enrichment: true`, always at
/// `effort_source: inferred`), but wrong in principle: an hour figure a tier-1 heuristic merely
/// carried forward is not this pass's own estimate, and only the model's guess from a title and a
/// description earns the "low" label — writing that on a vendor's own number would misrepresent
/// it to `rank`.
pub fn literals_for(v: &judge::Verdict, complete: bool) -> Vec<(String, String)> {
    let mut out: Vec<(String, String)> = Vec::new();
    if let Some(c) = &v.course {
        out.push(("course".to_string(), write::to_literal(&Value::String(c.clone()))));
    }
    if let Some(e) = v.effort_hours {
        // Through `to_literal` with a YAML `Number`, **never `format!("{e}")`**: Rust's `Display`
        // for `2.0_f64` is `2`, which retypes the field from float to int on a live vault — every
        // other producer (`ingest`'s template, `coursework`, the routine) writes `2.0`, and two
        // spellings of one estimate is exactly the silent drift the single write path exists to
        // stop. `serde_yaml_ng`'s `Number` renders a float through ryu, which always emits the
        // decimal point.
        out.push((
            "effort_hours".to_string(),
            write::to_literal(&Value::Number(serde_yaml_ng::Number::from(e))),
        ));
        if v.tier == 3 {
            out.push(("effort_confidence".to_string(), write::to_literal(&Value::String("low".into()))));
        }
    }
    if let Some(i) = v.importance {
        out.push(("importance".to_string(), i.to_string()));
    }
    if let Some(r) = &v.importance_reason {
        out.push(("importance_reason".to_string(), write::to_literal(&Value::String(r.clone()))));
    }
    if complete {
        out.push(("needs_enrichment".to_string(), "false".to_string()));
    }
    out
}

/// The testable core: the model — or the reason there is none — is handed in, and **no process is
/// started here**. `run_lines` is this with `runtime::resolve` (and a fresh `runtime::PerCall`) in
/// front.
pub fn enrich_with(
    vault: &Path,
    opts: &Options<'_>,
    model: Result<&dyn judge::Model, judge::Missing>,
) -> (i32, Vec<String>) {
    let (all, skipped) = pending(vault);
    let mut lines: Vec<String> = skipped.into_iter().map(|s| format!("skipped {s}")).collect();
    if all.is_empty() {
        lines.push("judge: nothing to enrich".to_string());
        return (0, lines);
    }
    let left = all.len().saturating_sub(opts.limit);
    let batch: Vec<Item> = all.into_iter().take(opts.limit).collect();

    let heuristics = Heuristics::load(vault);
    let ctx = WriteContext {
        actor: ACTOR.to_string(),
        via: opts.via.to_string(),
        run_id: opts.run_id.map(str::to_string),
    };
    let mut journal = Journal::new(vault);
    let mut answered = 0usize;
    let mut proposed = 0usize;
    let mut processed = 0usize;
    let batch_started = std::time::Instant::now();

    for item in &batch {
        // M1: checked BEFORE starting each item, not after — the worst case this bounds is the
        // budget plus one `CALL_TIMEOUT` already in flight, not the budget plus a whole extra item.
        if batch_started.elapsed() >= opts.budget {
            break;
        }
        processed += 1;
        let started = std::time::Instant::now();
        let outcome = judge::judge_task(item, &heuristics, &NoRules, model);
        let complete = matches!(outcome, Outcome::Answered(_));
        let literals = literals_for(outcome.verdict(), complete);
        let ms = started.elapsed().as_millis().min(i64::MAX as u128) as i64;

        // Ids and field values only (§5.6) — and logged for EVERY outcome, because a week of
        // "model not installed" is exactly what answers "why is nothing being enriched?".
        if let Some(dir) = opts.log_dir {
            let entry = crate::judgelog::entry_for(&item.id, &outcome, &literals, opts.run_id, ms);
            if let Err(e) = crate::judgelog::record(dir, &entry, None) {
                lines.push(format!("judge: {e}"));
            }
        }

        // C1 (Task 7 fix round 1): computed BEFORE the `literals.is_empty()` early exit, and used
        // in all three places below. `tier1` seeds nothing at all unless a note already names a
        // course or the uid is pinned — a wizard-created friend's vault has an empty `course_map`
        // and `effort_source: inferred`, so EVERY item seeds empty. On the machine where the
        // runtime is broken, every item then takes the early exit, and if `why` were appended only
        // in the `Ok(res)` arm below, the diagnostic that exists FOR that machine would never
        // appear on it: the line would read `nothing to write (low confidence)` with the runtime's
        // stderr tail nowhere. This is the third time this exact diagnostic has been dropped at a
        // boundary (Task 3's process boundary, Task 5's log boundary, now stdout) — the standing
        // question is "does the failure text survive this boundary?", not "does this case compile".
        let why_suffix = match &outcome {
            Outcome::LowConfidence { why, .. } => format!(" — {why}"),
            _ => String::new(),
        };
        if literals.is_empty() {
            lines.push(format!("{} nothing to write ({}){why_suffix}", item.rel_path, outcome.label()));
            continue;
        }
        let mut inputs = serde_yaml_ng::Mapping::new();
        inputs.insert("source_uid".into(), Value::String(item.source_uid.clone()));
        inputs.insert("title_seen".into(), Value::String(item.title.clone()));
        let write_opts = WriteOpts { judged: true, evidence: None, propose: true, inputs: Some(&inputs) };
        match write::write_literals(vault, &item.rel_path, &literals, &ctx, &mut journal, &write_opts) {
            Ok(res) => {
                if complete {
                    answered += 1;
                }
                let wrote: Vec<&str> = res.written.iter().map(|(n, _)| n.as_str()).filter(|n| *n != "judgment").collect();
                let mut line = format!("{} {} [{}]", item.rel_path, outcome.label(), wrote.join(" "));
                // `res.proposal` is the CARD — the whole point of `propose: true` — and it is the
                // thing that has to be counted and named. `res.skipped` is the explanation beside
                // it, and it is also set for the "already pending" case, where nothing new was
                // minted; counting `skipped` alone reported "0 proposed" on the very run that filed
                // one, and named no card for the reader to go and look at (review S2).
                if let Some(card) = &res.proposal {
                    proposed += 1;
                    let name = card.file_name().map(|n| n.to_string_lossy().to_string()).unwrap_or_default();
                    line.push_str(&format!(" — proposed instead ({name})"));
                }
                if !res.skipped.is_empty() {
                    let mut why: Vec<String> = res.skipped.iter().map(|(n, w)| format!("{n}: {w}")).collect();
                    why.sort();
                    line.push_str(&format!(" [{}]", why.join("; ")));
                }
                line.push_str(&why_suffix);
                lines.push(line);
            }
            // A single unwritable note must not end the batch: the rest are still worth judging.
            // `why_suffix` here too: a write failure on a low-confidence item must still say what
            // the model did and why, not just that the write failed.
            Err(e) => lines.push(format!("{} not written: {e}{why_suffix}", item.rel_path)),
        }
    }

    // M1: `left` already counts what `limit` excluded from the batch; fold in whatever the budget
    // cut off the batch itself, so one line always names everything not taken this run.
    let left = left + (batch.len() - processed);
    let mut summary = format!("judge: {processed} item(s), {answered} enriched, {proposed} proposed");
    if left > 0 {
        summary.push_str(&format!(", {left} left for the next slot"));
    }
    lines.push(summary);
    (0, lines)
}

/// Resolve the service — or, on a vault with no account, plan 3a's local runtime — and enrich.
///
/// **The cloud comes first from C2 on** (cloud design D3, §3.2). The local branch below is plan
/// 3a's and is removed by C4; until then a vault with no `config/cloud.yaml` behaves exactly as it
/// did. Every arm exits 0: no account, no session, no network and no entitlement are all normal
/// outcomes reported on stdout.
pub fn run_lines(vault: &Path, opts: &Options<'_>) -> (i32, Vec<String>) {
    match crate::cloudmodel::resolve(vault) {
        Ok(client) => run_lines_with(vault, opts, Some(&client)),
        Err(crate::cloudmodel::Unavailable::NoConfig) => run_lines_with(vault, opts, None),
        Err(why) => {
            // A vault WITH an account whose session is missing or unreadable. Not a fallback to
            // the local runtime: this is a cloud machine and the answer is "sign in again".
            let (code, mut lines) =
                enrich_with(vault, opts, Err(judge::Missing::Service(why.label())));
            lines.insert(0, format!("judge: skipped ({why})"));
            (code, lines)
        }
    }
}

/// [`run_lines`] with the service seam exposed, exactly as `ingest::run_with` exposes its fetch and
/// `coursework::main_with_fetchers` exposes its sources. Production resolves; tests pass a client
/// aimed at a listener bound to `127.0.0.1:0`, so no test needs a credential or an environment
/// variable and there is no process-global state to race.
///
/// **No server, no load phase (spec §5.3 as amended 2026-09-07).** The spike found the server's
/// advantage shrinking as the batch grows, not growing — 2.15x at thirty items, converging to
/// 2.21x, against a pre-committed 3x bar — so there is no long-lived process here: `judge_task`
/// spawns one `llama-cli` process per judgment, through `PerCall`, and it is gone before the next
/// item starts. A missing runtime or model is one line and exit 0 — never a failed slot.
pub fn run_lines_with(
    vault: &Path,
    opts: &Options<'_>,
    cloud: Option<&crate::cloudmodel::CloudClient>,
) -> (i32, Vec<String>) {
    let Some(client) = cloud else {
        return match crate::runtime::resolve(opts.runtime, opts.model) {
            Err(missing) => enrich_with(vault, opts, Err(missing)),
            Ok((rt, gguf)) => enrich_with(
                vault,
                opts,
                Ok(&crate::runtime::PerCall::new(&rt, &gguf, crate::runtime::CALL_TIMEOUT)),
            ),
        };
    };
    let model = crate::cloudmodel::CloudModel::new(client);
    // C2 Task 9 fix 1 (R-C2-E22 #2): the cloud arm's wall clock starts here, before the probe —
    // the events pass below shares `opts.budget` with the enrichment batch rather than getting a
    // fresh fifteen minutes of its own, and the two together must still land inside
    // `scheduler::CHILD_TIMEOUT`.
    let arm_started = std::time::Instant::now();
    let (events_config, _) =
        crate::events::load_events_config(&vault.join("config").join("events.yaml"));
    let any_feed = events_config.sources.iter().any(|s| s.enabled);
    // R-C2-E38: the Gmail pull's own precondition, computed once and reused below — a vault that
    // has never linked a Google calendar must not make the probe fire just to learn there is
    // nothing to pull, and a vault that HAS linked one must not be starved by an empty enrichment
    // queue and no event source, or its mail would never be asked for.
    let google = google_calendar_linked(vault);
    // R-C2-E15, widened by fix 1 (R-C2-E22 #1), again by Task 11 (R-C2-E38) and again by Task 12
    // (R-C2-E46): the probe is a network round trip, and it must not fire when there is nothing to
    // judge or send in ANY of the four passes — reusing `pending`'s own predicate, `judge_roster`'s
    // own enabled-source predicate, `google_calendar_linked`, and `rule_decisions_waiting` (an
    // answered `kind: rule` card not yet sent) here, rather than a second scanning routine for any
    // of them, is what lets a job that runs twice a day forever skip both the call and, on a
    // network that black-holes rather than refuses, the up-to-`CALL_TIMEOUT` stall a probe with no
    // queue behind it would otherwise risk. `enrich_with` below re-derives the pending list; that
    // second read is the accepted cost of leaving `enrich_with`'s own signature — and every test
    // that calls it directly — untouched.
    if pending(vault).0.is_empty() && !any_feed && !google && !rule_decisions_waiting(vault) {
        return enrich_with(vault, opts, Ok(&model));
    }
    // A session or entitlement problem answers every item identically, so the FIRST call decides
    // the batch. Asked once before the loop, the whole run then reports one honest outcome
    // (`Missing::Service`, which becomes `Outcome::ServiceUnavailable` and logs as
    // `service unavailable`) instead of fifty `model failed` lines — ruling R-3a-20's point, at
    // the boundary a cloud judge adds. Every other failure (a 429, a 5xx, a dropped connection)
    // stays per-item, because the next item genuinely may succeed.
    let probe = model.probe();
    // C2 final review E-1: `enrich_with` starts its OWN clock (`batch_started`), so before this it
    // was handed a full `opts.budget` no matter how much of the slot the probe had already spent —
    // and on a network that black-holes rather than refuses, the probe alone can cost a whole
    // `CALL_TIMEOUT`. The arm's remaining time is what every pass below gets: the enrichment batch
    // here, then `judge_roster`, then `pull_gmail`, then `pull_rules`, each subtracting what the
    // ones before it spent. The sum is what has to fit inside `scheduler::CHILD_TIMEOUT`.
    let arm_opts = Options { budget: opts.budget.saturating_sub(arm_started.elapsed()), ..*opts };
    match probe {
        Some(reason) => {
            // Exactly one line: `probe()` already set `model.fatal()` to this same reason, so the
            // `if let Some(reason) = model.fatal()` line below — which exists for a batch that
            // turned fatal partway through — must not also fire here, or the run would print two
            // contradictory summaries ("nothing was sent" and "the rest ... was not sent") for the
            // one call that was actually made.
            let (code, mut lines) = enrich_with(vault, &arm_opts, Err(judge::Missing::Service(reason)));
            lines.insert(0, format!("judge: the service answered {reason}; nothing was sent"));
            (code, lines)
        }
        None => {
            let (code, mut lines) = enrich_with(vault, &arm_opts, Ok(&model));

            // C2 Task 9 — the events pass. Runs whenever at least one event source is enabled,
            // even when the enrichment batch above was empty: a vault can have nothing to enrich
            // and forty events to judge (the early return above already covers "neither"). The
            // feeds are fetched through the same server-side proxy `rank` uses, so the HTML
            // sources return a page here too. Fix 1 (R-C2-E22 #2): it shares `opts.budget` with
            // the enrichment batch rather than getting a fresh fifteen minutes of its own — the
            // remaining time is whatever `arm_started` has not already spent — and
            // `judge_roster`'s own loop breaks on it exactly as `enrich_with`'s does above, folding
            // what it does not reach into a "left for the next slot" line. C2 final review E-4:
            // `events_config` is read HERE only to decide whether this pass runs at all
            // (`any_feed`) and to pass `judge_per_run_cap` — `judge_roster` re-reads
            // `config/events.yaml` itself, so this is not the config it works from, and an earlier
            // comment claiming it was handed down was wrong.
            let remaining_budget = opts.budget.saturating_sub(arm_started.elapsed());
            let proxy = |url: &str| -> Result<String, String> {
                crate::cloudmodel::fetch_event_source(client, url)
                    .or_else(|_| crate::eventfeed::fetch_event_source(url))
            };
            lines.extend(crate::events::judge_roster(
                vault,
                &model,
                Some(&proxy),
                jiff::Zoned::now().date(),
                events_config.judge_per_run_cap.max(0) as usize,
                remaining_budget,
            ));

            // Task 11 — the Gmail pull. Runs whenever this vault has ever linked a Google
            // calendar, even when both passes above found nothing (the early return above already
            // covers "neither, and no Google either"). `pull_gmail` talks to `/gmail-read` through
            // `client` directly, not through `model`, so it carries its own failure lines rather
            // than feeding `model.fatal()`. R-C2-E42: it takes whatever of `opts.budget` the
            // events pass has not already spent, the same way `judge_roster` above does.
            if google {
                let gmail_budget = opts.budget.saturating_sub(arm_started.elapsed());
                lines.extend(pull_gmail(vault, client, opts, gmail_budget));
            }

            // C2 Task 12 — rule promotion. Runs whenever the cloud arm got past the probe,
            // whatever the three passes above found: it sends any decision the student already
            // made and files whatever the service is offering, and both directions are cheap
            // PostgREST reads/writes with no model call behind them. C2 final review E-1: it takes
            // whatever of `opts.budget` the three passes above have not already spent, the same
            // way `judge_roster` and `pull_gmail` do — before this it took no budget at all and
            // could start a request after the slot's own twenty minutes were already gone.
            let rules_budget = opts.budget.saturating_sub(arm_started.elapsed());
            lines.extend(pull_rules(vault, client, opts, rules_budget));

            if let Some(reason) = model.fatal() {
                lines.push(format!(
                    "judge: the service answered {reason}, so the rest of the batch was not sent"
                ));
            }
            (code, lines)
        }
    }
}

/// [`run_lines`] with the output printed, in the order it was produced.
pub fn run_with(vault: &Path, opts: &Options<'_>) -> i32 {
    let (code, lines) = run_lines(vault, opts);
    for line in &lines {
        println!("{line}");
    }
    code
}

/// What `main` calls.
pub fn run(
    vault: &Path,
    via: &str,
    run_id: Option<&str>,
    runtime: Option<&PathBuf>,
    model: Option<&PathBuf>,
    log_dir: Option<&PathBuf>,
    limit: usize,
) -> i32 {
    run_with(
        vault,
        &Options {
            via,
            run_id,
            runtime: runtime.map(PathBuf::as_path),
            model: model.map(PathBuf::as_path),
            log_dir: log_dir.map(PathBuf::as_path),
            limit,
            budget: BATCH_BUDGET,
        },
    )
}

/// R-C2-E38, narrowed to a pure read by R-C2-E43. The vault's `config/ingest.yaml` `calendars:`
/// list is the device's only trace of a Google grant — hand-off H9 writes `- name: google` /
/// `ics_url: 'cloud:google'` — so a vault that has never linked a Google calendar asks the service
/// nothing extra.
///
/// **Named for what the marker actually is, not for "the account has a grant"**: the app's Tauri
/// command `google_connected` means the latter — a live grant, which this vault might have and
/// still never have taken the wizard's calendar step to write the marker for (or vice versa, on a
/// vault whose grant was later revoked server-side). This predicate answers only "does this
/// vault's OWN config carry the marker", through [`crate::calfeed::calendar_entries`] — the one
/// place `calendars:` is parsed, so this is a config read and nothing else: no fetch, no snapshot
/// write, no network round trip.
fn google_calendar_linked(vault: &Path) -> bool {
    crate::calfeed::calendar_entries(vault).iter().any(|(_, url)| url.starts_with("cloud:google"))
}

/// The agent actor for Gmail-derived writes. `agent:` prefix, so judge-once holds and a field the
/// student set comes back as a `kind: amend` card rather than being overwritten.
pub const GMAIL_ACTOR: &str = "agent:knowlu.gmail";

/// How many times one slot asks. The service stops at its own wall-clock budget and says `more`;
/// three rounds covers a 180-message backlog and still cannot hold the slot open.
pub const PULL_ROUNDS: usize = 3;

/// The template a `tier: task` message becomes. `needs_enrichment: false` because the service has
/// already judged effort and importance — flagging it would send it straight back for a second
/// judgment of the same thing.
const GMAIL_NOTE: &str = "---\ntitle: {title}\ncourse: {course}\ndomain: school\ndue: {due}\n\
effort_hours: {effort_hours}\neffort_confidence: low\neffort_source: inferred\n\
importance: {importance}\nimportance_reason: {why}\nstatus: active\nprogress: 0\n\
created_by: gmail\nsource_uid: {uid}\nneeds_enrichment: false\n---\n\n{body}\n";

/// Pull the service's queued Gmail judgments and write them, then acknowledge them.
///
/// Every write goes through `write` (journal first, single-line surgery second) under
/// [`GMAIL_ACTOR`]; a `tier: task` becomes a note, the three middle tiers become ordinary
/// proposals — so the 15-a-day cap applies to them exactly as it applies to every other card,
/// through `approvals::defer_over_budget` at the next `rank` — and `information` is dropped with
/// its uid recorded so it is never asked about again.
pub fn pull_gmail(
    vault: &Path,
    client: &crate::cloudmodel::CloudClient,
    opts: &Options<'_>,
    budget: std::time::Duration,
) -> Vec<String> {
    let ctx = WriteContext {
        actor: GMAIL_ACTOR.to_string(),
        via: opts.via.to_string(),
        run_id: opts.run_id.map(str::to_string),
    };
    let mut journal = Journal::new(vault);
    let today = jiff::Zoned::now().date();
    let stamp = today.strftime("%Y-%m-%d").to_string();
    let mut lines: Vec<String> = Vec::new();
    let (mut notes, mut cards, mut dropped, mut invalid, mut failed) =
        (0usize, 0usize, 0usize, 0usize, 0usize);
    let mut deferred = 0u64;
    let mut ack: Vec<String> = Vec::new();
    let started = std::time::Instant::now();

    for round in 0..PULL_ROUNDS {
        // R-C2-E42: the same discipline `judge_roster` uses — checked BEFORE the round, not
        // after, so the worst case this bounds is the budget plus one call already in flight, not
        // the budget plus a whole extra round. `CALL_TIMEOUT` is the bound because a round can
        // include model calls at up to that cost each; asking with less of the slot left than one
        // more call could take risks stalling past `scheduler::CHILD_TIMEOUT` for nothing.
        if started.elapsed() + crate::cloudmodel::CALL_TIMEOUT > budget {
            // C2 final review E-4: only when this run actually queued something. A slot whose
            // budget ran out before the FIRST round has nothing to leave for the next one, and
            // `gmail: 0 queued, left for the next slot` read as a loss rather than a no-op.
            let queued = notes + cards + dropped;
            if queued > 0 {
                lines.push(format!("gmail: {queued} queued, left for the next slot"));
            }
            break;
        }
        let pulled = match crate::cloudmodel::pull_gmail_queue(client, &ack) {
            Ok(got) => got,
            // R-C2-E41: a calendar-only grant is not a failure at all — a step left out, exactly
            // like `judge (skipped: no model)` — so it prints nothing rather than a line that
            // would read as trouble on an account that has none.
            Err(crate::cloudmodel::CloudError::Quiet(crate::cloudmodel::QuietReason::NoGmailScope)) => {
                return lines;
            }
            Err(e) => {
                lines.push(format!("gmail: skipped ({e})"));
                return lines;
            }
        };
        let (items, more) = (pulled.items, pulled.more);
        deferred += pulled.deferred;
        ack.clear();
        if items.is_empty() {
            if round == 0 {
                return lines;
            }
            break;
        }
        // Re-read per round: the previous round's `record_seen` calls are in it.
        let seen = crate::ingest::load_seen(vault);
        // R-C2-E44/E45 (2): the per-batch guard, tracked apart from `seen` (the disk ledger, read
        // fresh above and never mutated here) because a duplicate row must be handled differently
        // depending on how its FIRST occurrence in this batch turned out: acked again if it was
        // written or seen-recorded, but never acked, re-attempted or re-counted if it failed — an
        // unwritten duplicate must not be told to the server as written.
        let mut acked_this_batch: std::collections::HashSet<String> = std::collections::HashSet::new();
        let mut failed_this_batch: std::collections::HashSet<String> = std::collections::HashSet::new();
        for item in &items {
            // The uid is the same `gmail:<message-id>` the server deduplicates on, so a message is
            // written once whichever side asked. A uid already here means an earlier slot (or an
            // earlier, SUCCESSFUL duplicate row in this same batch) wrote it and the
            // acknowledgement did not reach the server — acknowledge and move on.
            if seen.contains(&item.uid) || acked_this_batch.contains(&item.uid) {
                ack.push(item.uid.clone());
                continue;
            }
            // A duplicate row whose first occurrence in THIS batch already failed: skip silently,
            // rather than re-attempt the same doomed write or count the same failure twice.
            if failed_this_batch.contains(&item.uid) {
                continue;
            }
            // R-C2-E43: a shape check on `due` (and `uid`) mirroring the server's own — belt and
            // braces, since this doc comment's "every write goes through `write`" claim must hold
            // even if a payload somehow arrived malformed. Routed through the same `Result` the
            // write functions return, so a malformed item is handled exactly like an unwritable
            // one: not written, not acknowledged, and counted.
            let outcome: Result<String, String> = match gmail_item_shape_ok(item) {
                Err(reason) => {
                    invalid += 1;
                    Err(reason)
                }
                Ok(()) => match item.tier.as_str() {
                    // `information` is the noise tier: nothing is written, and the uid is recorded
                    // so the service is never asked about that message again. Recorded with a
                    // fixed title, never the model's: the note it would have named is dropped
                    // exactly because it wasn't worth writing, so its subject is not worth keeping
                    // either (R-C2-E44).
                    "information" => {
                        dropped += 1;
                        Ok(String::new())
                    }
                    "task" => write_gmail_note(vault, item, &ctx, &mut journal)
                        .map(|stem| {
                            notes += 1;
                            format!("created {stem}")
                        })
                        .inspect_err(|_| failed += 1),
                    "borderline" | "event" | "opportunity" => {
                        write_gmail_card(vault, item, today, &ctx, &mut journal)
                            .map(|stem| {
                                cards += 1;
                                format!("proposed {stem}")
                            })
                            .inspect_err(|_| failed += 1)
                    }
                    // T9: an email confirming the student already submitted a piece of work. A
                    // `status: done` proposal when exactly one active task carries that title;
                    // otherwise nothing is written and the uid is recorded as `information` is.
                    "completion" => propose_gmail_completion(vault, item, today, &ctx, &mut journal)
                        .map(|stem| match stem {
                            Some(stem) => {
                                cards += 1;
                                format!("proposed done {stem}")
                            }
                            None => {
                                dropped += 1;
                                String::new()
                            }
                        })
                        .inspect_err(|_| failed += 1),
                    // T9: a tier this engine does not know is dropped and recorded like
                    // `information`. The old catch-all filed it as a `kind: task` card, which for a
                    // newer tier can mean the opposite of what it says: a receipt for finished work
                    // read as a proposal to add that work.
                    _ => {
                        dropped += 1;
                        Ok(String::new())
                    }
                },
            };
            match outcome {
                Ok(note) => {
                    if !note.is_empty() {
                        lines.push(format!("gmail {}: {} ({})", item.uid, item.tier, note));
                    }
                    // The underlying write (or the deliberate no-write for `information`) already
                    // succeeded here — a duplicate of this uid later in the batch is safe to ack
                    // again regardless of what the seen-ledger write below does.
                    acked_this_batch.insert(item.uid.clone());
                    // Only a write names its title here. Everything dropped (information, an
                    // unmatched completion, an unknown tier) is recorded as "(email)" (R-C2-E44).
                    let title = if note.is_empty() { "(email)" } else { &item.title };
                    if let Err(e) = crate::ingest::record_seen(vault, &item.uid, title, &stamp) {
                        lines.push(format!("gmail {}: seen ledger not written ({e})", item.uid));
                        continue;
                    }
                    ack.push(item.uid.clone());
                }
                // A single unwritable item must not end the batch, and it must NOT be
                // acknowledged: an unacknowledged row comes back next slot, which is the recovery.
                // R-C2-E45 (2): a duplicate of this SAME uid later in the batch must not be acked
                // either — nothing was written or seen-recorded for it, so telling the server
                // otherwise would lose it for good the moment the ack reached the server.
                Err(e) => {
                    failed_this_batch.insert(item.uid.clone());
                    lines.push(format!("gmail {}: not written ({e})", item.uid));
                }
            }
        }
        if !more {
            break;
        }
    }

    if !ack.is_empty() {
        if let Err(e) = crate::cloudmodel::pull_gmail_queue(client, &ack) {
            lines.push(format!(
                "gmail: {} written but not acknowledged ({e}); they will come back next slot",
                ack.len()
            ));
        }
    }
    let mut summary = format!("gmail: {notes} task(s), {cards} proposed, {dropped} dropped as information");
    // C2 final review E-4: the service's own count of messages it read and could not queue — a
    // spent cap, a verdict under the floor, a model failure. They are neither queued nor marked
    // seen server-side, so they come back next slot; without this line a student whose account is
    // capped sees `0 task(s), 0 proposed, 0 dropped` and nothing to explain it.
    if deferred > 0 {
        summary.push_str(&format!(", {deferred} deferred by the service"));
    }
    if invalid > 0 {
        summary.push_str(&format!(", {invalid} skipped as malformed"));
    }
    if failed > 0 {
        summary.push_str(&format!(", {failed} not written"));
    }
    lines.push(summary);
    lines
}

/// T9: the key two titles are compared on: whitespace trimmed and collapsed, case folded, every
/// straight or curly single or double quote made one character, and the few HTML entities a
/// templated email carries decoded. The service one-lines a title (`oneLine` turns `"` into `'`)
/// and a task note stores it YAML-escaped (unescaped by the YAML read), so the two sides of a
/// match can differ in exactly these ways.
fn completion_title_key(title: &str) -> String {
    let decoded = title
        .replace("&quot;", "\"")
        .replace("&#39;", "'")
        .replace("&apos;", "'")
        .replace("&lt;", "<")
        .replace("&gt;", ">")
        .replace("&amp;", "&");
    let folded: String = decoded
        .chars()
        .map(|c| match c {
            '"' | '\u{201c}' | '\u{201d}' | '\u{201e}' | '\u{2018}' | '\u{2019}' | '\u{201a}' | '`'
            | '\u{b4}' => '\'',
            other => other,
        })
        .collect();
    judge::one_line(&folded, usize::MAX).to_lowercase()
}

/// The active task notes directly under `tasks/` whose title matches `title` by
/// [`completion_title_key`], in path order.
fn active_tasks_titled(vault: &Path, title: &str) -> Vec<PathBuf> {
    let want = completion_title_key(title);
    if want.is_empty() {
        return Vec::new();
    }
    let tasks = vault.join("tasks");
    if !tasks.is_dir() {
        return Vec::new();
    }
    crate::approvals::sorted_md(&tasks)
        .into_iter()
        .filter(|path| {
            let Some(meta) = crate::ids::read_meta(path) else { return false };
            let text = |key: &str| crate::yaml::opt_text(crate::yaml::get(&meta, key));
            text("status").as_deref() == Some("active")
                && text("title").is_some_and(|t| completion_title_key(&t) == want)
        })
        .collect()
}

/// T9: a `tier: completion` message. Exactly one active task titled as the payload says gets
/// [`crate::completion::propose_done`], which alone decides the 15-a-day cap, "never re-file after
/// a rejection" and "never touch a status the student set by hand". `Ok(Some(stem))` when a card
/// was filed; `Ok(None)` for every "nothing to do" (zero or several matches, or a skip), which the
/// caller records exactly as it records `information`. The evidence is templated: the message uid,
/// and no address, confirmation number or body text.
fn propose_gmail_completion(
    vault: &Path,
    item: &crate::cloudmodel::GmailItem,
    today: jiff::civil::Date,
    ctx: &WriteContext,
    journal: &mut Journal,
) -> Result<Option<String>, String> {
    let matches = active_tasks_titled(vault, &item.title);
    let [task] = matches.as_slice() else { return Ok(None) };
    let mut detail = serde_json::Map::new();
    detail.insert("uid".to_string(), serde_json::Value::String(item.uid.clone()));
    let evidence = crate::completion::Evidence {
        source: "email".to_string(),
        summary: "An email confirms this was submitted".to_string(),
        detail,
    };
    let ctx = ctx.with_actor(crate::completion::ACTOR);
    match crate::completion::propose_done(vault, task, &evidence, today, &ctx, journal, false) {
        Ok(crate::completion::Outcome::Proposed(_)) => {
            Ok(Some(task.file_stem().map(|s| s.to_string_lossy().to_string()).unwrap_or_default()))
        }
        Ok(_) => Ok(None),
        Err(e) => Err(e.to_string()),
    }
}

/// R-C2-E43: the device's own shape check, mirroring the server's `judge_validate.ts` regex
/// (`^\d{4}-\d{2}-\d{2}(T\d{2}:\d{2})?$`). The server already validates this before a verdict is
/// queued; this is the belt to that server-side brace — nothing here is written to a note without
/// also being checked on this side.
static GMAIL_DUE_RE: std::sync::LazyLock<regex::Regex> =
    std::sync::LazyLock::new(|| regex::Regex::new(r"^\d{4}-\d{2}-\d{2}(T\d{2}:\d{2})?$").unwrap());

fn gmail_item_shape_ok(item: &crate::cloudmodel::GmailItem) -> Result<(), String> {
    if item.uid.contains('\n') || item.uid.contains('\r') {
        return Err("the uid carries a line break".to_string());
    }
    if let Some(due) = item.due.as_deref() {
        if due.contains('\n') || due.contains('\r') || !GMAIL_DUE_RE.is_match(due) {
            return Err(format!("malformed due ({due:?})"));
        }
    }
    Ok(())
}

/// A `tier: task` message as a note. Through `write::create`, so the journal record comes first.
fn write_gmail_note(
    vault: &Path,
    item: &crate::cloudmodel::GmailItem,
    ctx: &WriteContext,
    journal: &mut Journal,
) -> Result<String, String> {
    let text = gmail_note_text(item);
    let stem = crate::ingest::slugify(&item.title);
    let tasks = vault.join("tasks");
    std::fs::create_dir_all(&tasks).map_err(|e| e.to_string())?;
    let mut path = tasks.join(format!("{stem}.md"));
    let mut suffix = 2;
    while path.exists() {
        path = tasks.join(format!("{stem}-{suffix}.md"));
        suffix += 1;
    }
    let rel = crate::ids::rel(vault, &path);
    crate::write::create(vault, &rel, &text, ctx, journal, None)
        .map(|p| p.file_stem().map(|s| s.to_string_lossy().to_string()).unwrap_or_default())
        .map_err(|e| e.to_string())
}

/// The frontmatter one item becomes. Every free-text field goes through `write::to_literal`, so a
/// colon or a quote in a subject line cannot produce frontmatter the loader silently drops.
fn gmail_note_text(item: &crate::cloudmodel::GmailItem) -> String {
    let lit = |s: &str| write::to_literal(&Value::String(s.to_string()));
    let course = match item.course.as_deref().filter(|c| !c.is_empty()) {
        Some(c) => lit(c),
        None => "null".to_string(),
    };
    let due = match item.due.as_deref().filter(|d| !d.is_empty()) {
        Some(d) => d.to_string(),
        None => "null".to_string(),
    };
    let effort = write::to_literal(&Value::Number(serde_yaml_ng::Number::from(
        item.effort_hours.unwrap_or(1.0),
    )));
    GMAIL_NOTE
        .replace("{title}", &lit(&item.title))
        .replace("{course}", &course)
        .replace("{due}", &due)
        .replace("{effort_hours}", &effort)
        .replace("{importance}", &item.importance.unwrap_or(3).to_string())
        .replace("{why}", &lit(&item.why))
        .replace("{uid}", &lit(&item.uid))
        .replace("{body}", &format!("From email. {}", item.why))
}

/// The three middle tiers as a `kind: task` approval card. The payload is the same note text, in a
/// fenced `task` block, because that is exactly what `approvals::materialize` turns into a note
/// when the card is approved — so an approved card and a `tier: task` message produce the same
/// note, and there is one note writer rather than two.
///
/// The stem must start with `task-`: `materialize` strips that prefix to name the note.
fn write_gmail_card(
    vault: &Path,
    item: &crate::cloudmodel::GmailItem,
    today: jiff::civil::Date,
    ctx: &WriteContext,
    journal: &mut Journal,
) -> Result<String, String> {
    let lit = |s: &str| write::to_literal(&Value::String(s.to_string()));
    let stamp = today.strftime("%Y-%m-%d").to_string();
    let expires = today
        .checked_add(jiff::Span::new().days(14))
        .unwrap_or(today)
        .strftime("%Y-%m-%d")
        .to_string();
    let stem = format!("task-{}", crate::ingest::slugify(&item.title));
    let text = format!(
        "---\ntype: approval\nkind: task\ntitle: {}\nstatus: pending\nproposed_at: {stamp}\n\
         first_proposed_at: {stamp}\nexpires: {expires}\nsnooze_until: null\ncreated_by: gmail\n\
         source_uid: {}\n---\n\n{}\n\n```task\n{}```\n",
        lit(&item.title),
        lit(&item.uid),
        item.why,
        gmail_note_text(item),
    );
    let approvals = vault.join("approvals");
    std::fs::create_dir_all(&approvals).map_err(|e| e.to_string())?;
    let mut path = approvals.join(format!("{stem}.md"));
    let mut suffix = 2;
    while path.exists() {
        path = approvals.join(format!("{stem}-{suffix}.md"));
        suffix += 1;
    }
    let rel = crate::ids::rel(vault, &path);
    crate::write::create(vault, &rel, &text, ctx, journal, None)
        .map(|p| p.file_stem().map(|s| s.to_string_lossy().to_string()).unwrap_or_default())
        .map_err(|e| e.to_string())
}

// -----------------------------------------------------------------------------------------
// C2 Task 12 — rule promotion: the loop that retires model calls.
// -----------------------------------------------------------------------------------------

/// The agent actor for rule cards. Its own name, not `agent:knowlu.enrich`, so the journal says
/// which loop filed a card.
pub const RULES_ACTOR: &str = "agent:knowlu.rules";

/// Send the decisions the student has already made, then file whatever the service is offering.
///
/// **Runs on a cloud arm that got past the probe** — not on "every cloud arm". A vault with
/// nothing to enrich, no enabled event source, no Google grant and no answered rule card takes
/// `run_lines_with`'s early return and never reaches this, which is exactly what keeps a twice-daily
/// slot from spending two PostgREST round trips a day forever on a vault that has nothing to say.
///
/// **Sending first** means a slot never proposes a rule the student answered an hour ago. The card
/// is an ordinary proposal, so it is counted, escalated, snoozed past the 15-a-day cap and expired
/// exactly as every other card is. `rank` leaves an approved one alone (hand-off H6), because
/// `rank` never opens a socket for a judgment.
///
/// `budget` is whatever of the slot the three passes before this one have left (C2 final review
/// E-1), checked before each request exactly as [`pull_gmail`] checks its own. Both directions
/// here are cheap PostgREST calls with no model behind them — but "cheap" is a property of the
/// server, not of the network, and each one can still cost a whole `CALL_TIMEOUT` on a connection
/// that black-holes. Whatever this run does not reach is named and left for the next slot: a
/// decision is re-read from its still-`approved` card, and a proposal is re-offered.
pub fn pull_rules(
    vault: &Path,
    client: &crate::cloudmodel::CloudClient,
    opts: &Options<'_>,
    budget: std::time::Duration,
) -> Vec<String> {
    let ctx = WriteContext {
        actor: RULES_ACTOR.to_string(),
        via: opts.via.to_string(),
        run_id: opts.run_id.map(str::to_string),
    };
    let mut journal = Journal::new(vault);
    let mut lines: Vec<String> = Vec::new();
    let today = jiff::Zoned::now().date();
    let started = std::time::Instant::now();
    // The same discipline `pull_gmail` uses: checked BEFORE the request, not after, so the worst
    // case this bounds is the budget plus one call already in flight.
    let spent = |elapsed: std::time::Duration| elapsed + crate::cloudmodel::CALL_TIMEOUT > budget;

    let cards = decided_rule_cards(vault);
    for (i, (path, id, decision)) in cards.iter().enumerate() {
        if spent(started.elapsed()) {
            lines.push(format!("rules: {} decision(s) left for the next slot", cards.len() - i));
            return lines;
        }
        let (path, id, decision) = (path.as_path(), *id, decision.as_str());
        match crate::cloudmodel::decide_rule(client, id, decision) {
            Ok(()) => {
                archive_decided_card(vault, path, &ctx, &mut journal);
                lines.push(format!("rules {id}: {decision}"));
            }
            // I4 (R-C2-E47 fix 1), narrowed by R-C2-E49 fix 3: a 404 means the service already
            // settled this exact decision — most likely this same card, sent by a previous run
            // that died between the POST above and the stamp below. Treated as success, not
            // failure: stamped and archived exactly as `Ok` is. Without this, a card whose POST
            // succeeded once but whose stamp never landed would re-POST every slot forever, 404
            // every time, and — because `rule_decisions_waiting` scans exactly this card — keep the
            // widened early return (R-C2-E46) from ever firing again for this vault.
            //
            // **Matched on the handler's own body, not the status alone** (`judge-rules/handler.ts`'s
            // `Response.json({ error: "no such undecided proposal" }, { status: 404 })`, extracted
            // into `detail` by `CloudClient::finish`) — a gateway 404 (the function not deployed, a
            // stale `api_base`, a slug typo) is ALSO an HTTP 404 and would otherwise be read as
            // "already settled" and silently discard a real, unsent decision.
            Err(crate::cloudmodel::CloudError::Status { code: 404, ref detail })
                if detail == "no such undecided proposal" =>
            {
                archive_decided_card(vault, path, &ctx, &mut journal);
                lines.push(format!("rules {id}: already settled"));
            }
            // NOT archived on any other failure — including a 404 whose body does not match the
            // handler's own text — an unsent decision must come back next slot.
            Err(e) => lines.push(format!("rules {id}: not sent ({e})")),
        }
    }

    if spent(started.elapsed()) {
        lines.push("rules: the offer was left for the next slot".to_string());
        return lines;
    }
    let proposals = match crate::cloudmodel::pull_rule_proposals(client) {
        Ok(p) => p,
        Err(e) => {
            lines.push(format!("rules: skipped ({e})"));
            return lines;
        }
    };
    if proposals.is_empty() {
        return lines;
    }
    let existing = existing_rule_ids(vault);
    let mut filed = 0usize;
    for proposal in &proposals {
        // A card is never minted twice for one proposal id — the id is in the card's frontmatter
        // and this is the guard, exactly as `find_pending_amendment` guards an amend card.
        if existing.contains(&proposal.id) {
            continue;
        }
        match write_rule_card(vault, proposal, today, &ctx, &mut journal) {
            Ok(stem) => {
                filed += 1;
                lines.push(format!("rules {}: proposed ({stem})", proposal.id));
            }
            Err(e) => lines.push(format!("rules {}: not written ({e})", proposal.id)),
        }
    }
    lines.push(format!("rules: {filed} proposed of {} offered", proposals.len()));
    lines
}

/// Stamp a decided rule card `executed` and archive it — the shared tail of both `pull_rules`
/// outcomes that count as settled (the service accepted the decision, or already had it). Best
/// effort: if the stamp write fails the card is simply left `approved`/`rejected` and comes back
/// next slot, exactly as it would if this were never called.
fn archive_decided_card(vault: &Path, path: &std::path::Path, ctx: &WriteContext, journal: &mut Journal) {
    let rel = crate::ids::rel(vault, path);
    let stamped = format!("\"{}\"", jiff::Zoned::now().strftime("%Y-%m-%d %H:%M"));
    let literals = vec![
        ("status".to_string(), "executed".to_string()),
        ("executed_at".to_string(), stamped),
    ];
    if crate::write::write_literals(vault, &rel, &literals, ctx, journal, &WriteOpts::default()).is_ok() {
        let _ = crate::write::delete(vault, &rel, ctx, journal);
    }
}

/// `kind: rule` cards the student has answered, as `(path, rule_id, decision)`. Also what
/// [`rule_decisions_waiting`] scans — one scanner for "is there an answered rule card", not two.
fn decided_rule_cards(vault: &Path) -> Vec<(std::path::PathBuf, i64, String)> {
    let mut out = Vec::new();
    for path in crate::approvals::sorted_md(&vault.join("approvals")) {
        let Ok(text) = crate::pystr::read_text(&path) else { continue };
        let Ok((meta, _)) = crate::models::split_frontmatter(&text) else { continue };
        if crate::yaml::opt_text(crate::yaml::get(&meta, "kind")).as_deref() != Some("rule") {
            continue;
        }
        let status = crate::yaml::opt_text(crate::yaml::get(&meta, "status")).unwrap_or_default();
        if status != "approved" && status != "rejected" {
            continue;
        }
        let Some(id) = crate::yaml::get(&meta, "rule_id").and_then(crate::yaml::i64_of) else { continue };
        out.push((path, id, status));
    }
    out
}

/// R-C2-E46: whether this vault has an answered `kind: rule` card not yet sent to the service —
/// reusing [`decided_rule_cards`], the same scan `pull_rules` performs to find them, so there is
/// one scanner for this question, not two.
fn rule_decisions_waiting(vault: &Path) -> bool {
    !decided_rule_cards(vault).is_empty()
}

/// Every `rule_id` already filed — `approvals/` for the live ones and `archive/` for the settled,
/// because a card that was decided last week must not be re-offered this week.
fn existing_rule_ids(vault: &Path) -> std::collections::BTreeSet<i64> {
    let mut out = std::collections::BTreeSet::new();
    for folder in ["approvals", "archive"] {
        for path in crate::approvals::sorted_md(&vault.join(folder)) {
            let Ok(text) = crate::pystr::read_text(&path) else { continue };
            let Ok((meta, _)) = crate::models::split_frontmatter(&text) else { continue };
            if let Some(id) = crate::yaml::get(&meta, "rule_id").and_then(crate::yaml::i64_of) {
                out.insert(id);
            }
        }
    }
    out
}

/// The card the deck renders. `kind: rule` is a kind `approvals.rs` leaves alone (hand-off H6):
/// the decision is *sent* by the next `judge` step, which then stamps and archives it.
fn write_rule_card(
    vault: &Path,
    proposal: &crate::cloudmodel::RuleProposal,
    today: jiff::civil::Date,
    ctx: &WriteContext,
    journal: &mut Journal,
) -> Result<String, String> {
    let lit = |s: &str| write::to_literal(&Value::String(s.to_string()));
    let stamp = today.strftime("%Y-%m-%d").to_string();
    let expires = today
        .checked_add(jiff::Span::new().days(14))
        .unwrap_or(today)
        .strftime("%Y-%m-%d")
        .to_string();
    let title = format!("Always judge {} the same way", proposal.value.replace('|', " "));
    let mut body = String::from(
        "Three judgments agreed and none disagreed in 60 days. Approving this stops the model \
         being asked about this pattern again; the fields it will write are below.\n\n",
    );
    if let Some(map) = proposal.verdict.as_object() {
        let mut names: Vec<&String> = map.keys().collect();
        names.sort();
        for name in names {
            body.push_str(&format!("- {name}: {}\n", crate::pystr::json_str(&map[name])));
        }
    }
    let text = format!(
        "---\ntype: approval\nkind: rule\ntitle: {}\nstatus: pending\nproposed_at: {stamp}\n\
         first_proposed_at: {stamp}\nexpires: {expires}\nsnooze_until: null\ncreated_by: rules\n\
         rule_id: {}\nfeature: {}\nvalue: {}\n---\n\n{body}",
        lit(&title),
        proposal.id,
        lit(&proposal.feature),
        lit(&proposal.value),
    );
    let approvals = vault.join("approvals");
    std::fs::create_dir_all(&approvals).map_err(|e| e.to_string())?;
    let stem = format!("rule-{}", proposal.id);
    let rel = crate::ids::rel(vault, &approvals.join(format!("{stem}.md")));
    crate::write::create(vault, &rel, &text, ctx, journal, None)
        .map(|p| p.file_stem().map(|s| s.to_string_lossy().to_string()).unwrap_or_default())
        .map_err(|e| e.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    // NOTE: `effort_confidence` deliberately starts as `unset`, not the `low` the real ingest
    // template writes. Every producer's answer for this field is `low` (literals_for hard-codes
    // it), so a fixture that started there too would make `write`'s F4 rule ("a no-op field
    // produces no record and no write") swallow it silently on every run in this file — never
    // wrong in production (the field is genuinely unchanged there), but it would make
    // `enrichment_writes_the_five_fields_...` unable to demonstrate that `effort_confidence` is
    // one of the five judged fields this pass writes and records.
    const NOTE: &str = "---\ntitle: \"CS-100 Homework 3\"\ncourse: null\ndomain: school\ndue: 2026-10-01T23:59\neffort_hours: 1.0\neffort_confidence: unset\neffort_source: inferred\nimportance: 3\nimportance_reason: \"pending enrichment\"\nstatus: active\nprogress: 0\ncreated_by: blackboard\nsource_uid: \"blackboard:_884411_1\"\nneeds_enrichment: true\nid: task_1111111111\n---\n\nChapters 4 and 5.\n";

    fn vault(tag: &str) -> PathBuf {
        let d = std::env::temp_dir().join(format!("qo-enrich-{tag}-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&d);
        std::fs::create_dir_all(d.join("tasks")).unwrap();
        std::fs::create_dir_all(d.join("approvals")).unwrap();
        std::fs::create_dir_all(d.join("config")).unwrap();
        std::fs::create_dir_all(d.join("courses")).unwrap();
        crate::pystr::write_text(&d.join("tasks").join("hw3.md"), NOTE).unwrap();
        crate::pystr::write_text(
            &d.join("config").join("ingest.yaml"),
            "course_map:\n  CS-100: cs-100\n  _884411_1: cs-100\n",
        ).unwrap();
        crate::pystr::write_text(
            &d.join("courses").join("cs-100.md"),
            "---\ntitle: CS 100\n---\n\n## Grade weights\n- Homework: 20%\n",
        ).unwrap();
        d
    }

    struct Fixed(crate::judge::Verdict);
    impl crate::judge::Model for Fixed {
        fn judge(&self, _i: &crate::judge::Item, _h: &crate::judge::Heuristics, _s: &crate::judge::Verdict)
            -> Result<crate::judge::Verdict, crate::judge::ModelError> { Ok(self.0.clone()) }
    }
    fn answered() -> Fixed {
        Fixed(crate::judge::Verdict {
            course: Some("cs-100".to_string()),
            effort_hours: Some(2.5),
            importance: Some(4),
            importance_reason: Some("Homework is 20% of CS 100.".to_string()),
            confidence: 0.9,
            tier: 3,
        })
    }
    fn opts<'a>(log: &'a Path) -> Options<'a> {
        Options { via: "local-runner", run_id: Some("local-2026-09-07T18:00:00Z"), runtime: None, model: None, log_dir: Some(log), limit: 50, budget: BATCH_BUDGET }
    }
    fn meta_of(v: &Path, name: &str) -> serde_yaml_ng::Mapping {
        crate::ids::read_meta(&v.join("tasks").join(name)).unwrap()
    }
    fn records(v: &Path) -> Vec<serde_json::Value> {
        let mut out = Vec::new();
        for e in std::fs::read_dir(v.join("state").join("journal")).unwrap().flatten() {
            for line in crate::pystr::read_text(&e.path()).unwrap().split('\n').filter(|l| !l.trim().is_empty()) {
                out.push(serde_json::from_str(line).unwrap());
            }
        }
        out
    }

    /// Every file the log directory holds, as raw text — not `judgelog::read_day`'s parsed
    /// records. R-3a-21 must be checked against the bytes actually on disk, because the guarantee
    /// it defends is about what a line contains, not about what a JSON parse of it would report.
    fn raw_log_text(log: &Path) -> String {
        let mut out = String::new();
        if let Ok(entries) = std::fs::read_dir(log) {
            for e in entries.flatten() {
                if let Ok(text) = crate::pystr::read_text(&e.path()) {
                    out.push_str(&text);
                }
            }
        }
        out
    }

    /// Only flagged notes are selected, and nothing else in `tasks/` is even read for judgment —
    /// "never modify notes without the flag" is the routine's own rule and it is this pass's too.
    #[test]
    fn only_notes_carrying_the_flag_are_selected() {
        let v = vault("select");
        crate::pystr::write_text(&v.join("tasks").join("done.md"),
            &NOTE.replace("needs_enrichment: true", "needs_enrichment: false").replace("task_1111111111", "task_2222222222")).unwrap();
        crate::pystr::write_text(&v.join("tasks").join("never.md"),
            &NOTE.replace("needs_enrichment: true\n", "").replace("task_1111111111", "task_3333333333")).unwrap();
        let (items, skipped) = pending(&v);
        assert_eq!(items.len(), 1);
        assert_eq!(items[0].id, "task_1111111111");
        assert_eq!(items[0].title, "CS-100 Homework 3");
        assert_eq!(items[0].body.trim(), "Chapters 4 and 5.");
        assert_eq!(items[0].source_uid, "blackboard:_884411_1");
        assert_eq!(items[0].due.as_deref(), Some("2026-10-01T23:59"));
        assert!(skipped.is_empty());
        let _ = std::fs::remove_dir_all(&v);
    }

    /// Without an `id:` judge-once cannot answer "did Quinn set this?", so the protection is void
    /// and the note is left alone and named.
    #[test]
    fn a_note_with_no_id_is_skipped_and_said_so() {
        let v = vault("noid");
        crate::pystr::write_text(&v.join("tasks").join("hw3.md"), &NOTE.replace("id: task_1111111111\n", "")).unwrap();
        let (items, skipped) = pending(&v);
        assert!(items.is_empty());
        assert_eq!(skipped, vec!["hw3.md: no id, so judge-once cannot protect it".to_string()]);
        let _ = std::fs::remove_dir_all(&v);
    }

    /// The whole pass: five judged fields plus the flag, an agent actor, the run's via and run_id,
    /// and a `judgment:` block naming exactly the judged fields it wrote.
    #[test]
    fn enrichment_writes_the_five_fields_clears_the_flag_and_records_a_judgment() {
        // `write_literals`/`judgelog::record` both read process-global `device_name()`.
        let _guard = crate::journal::DEVICE_ENV_MUTEX.lock().unwrap();
        let v = vault("happy");
        let log = v.join("_log");
        let m = answered();
        let (code, lines) = enrich_with(&v, &opts(&log), Ok(&m));
        assert_eq!(code, 0);
        assert!(lines.last().unwrap().starts_with("judge: 1 item"), "{lines:?}");

        let meta = meta_of(&v, "hw3.md");
        assert_eq!(crate::yaml::opt_text(crate::yaml::get(&meta, "course")).as_deref(), Some("cs-100"));
        assert_eq!(crate::yaml::opt_f64(crate::yaml::get(&meta, "effort_hours"), 0.0), 2.5);
        assert_eq!(crate::yaml::opt_text(crate::yaml::get(&meta, "effort_confidence")).as_deref(), Some("low"));
        assert_eq!(crate::yaml::opt_i64(crate::yaml::get(&meta, "importance"), 0), 4);
        assert_eq!(crate::yaml::opt_text(crate::yaml::get(&meta, "importance_reason")).as_deref(), Some("Homework is 20% of CS 100."));
        assert_eq!(crate::yaml::get(&meta, "needs_enrichment"), Some(&serde_yaml_ng::Value::Bool(false)));

        // The judgment block: single-line flow mapping, agent actor, the run, and the inputs.
        let text = crate::pystr::read_text(&v.join("tasks").join("hw3.md")).unwrap();
        assert!(crate::provenance::guard_block_style(&text).is_ok(), "must stay a single-line flow mapping");
        let Some(serde_yaml_ng::Value::Mapping(j)) = crate::yaml::get(&meta, "judgment") else { panic!("no judgment") };
        assert_eq!(crate::yaml::opt_text(crate::yaml::get(j, "actor")).as_deref(), Some("agent:knowlu.enrich"));
        assert_eq!(crate::yaml::opt_text(crate::yaml::get(j, "run_id")).as_deref(), Some("local-2026-09-07T18:00:00Z"));
        let Some(serde_yaml_ng::Value::Sequence(fields)) = crate::yaml::get(j, "fields") else { panic!("no fields") };
        let names: Vec<String> = fields.iter().filter_map(crate::yaml::text).collect();
        // D12: only fields provenance protects appear here, and `needs_enrichment` is not one.
        assert_eq!(names, vec!["course", "effort_confidence", "effort_hours", "importance", "importance_reason"]);
        for n in &names {
            assert!(crate::provenance::JUDGED_FIELDS_TASK.contains(&n.as_str()), "{n} is not a judged field");
        }
        let Some(serde_yaml_ng::Value::Mapping(inputs)) = crate::yaml::get(j, "inputs") else { panic!("no inputs") };
        assert_eq!(crate::yaml::opt_text(crate::yaml::get(inputs, "source_uid")).as_deref(), Some("blackboard:_884411_1"));
        assert_eq!(crate::yaml::opt_text(crate::yaml::get(inputs, "title_seen")).as_deref(), Some("CS-100 Homework 3"));

        for r in records(&v) {
            assert_eq!(r["actor"], "agent:knowlu.enrich");
            assert_eq!(r["via"], "local-runner");
            assert_eq!(r["run_id"], "local-2026-09-07T18:00:00Z");
        }
        let logged = crate::judgelog::read_day(&log, &crate::journal::now_ts(None)[..10]);
        assert_eq!(logged.len(), 1);
        assert_eq!(logged[0]["outcome"], "answered");
        assert_eq!(logged[0]["tier"], 3);
        let _ = std::fs::remove_dir_all(&v);
    }

    /// m1 (Task 7 fix round 1): production's actual starting shape. `ingest::NOTE_TEMPLATE` seeds
    /// every freshly ingested task at `effort_confidence: low` (`src/ingest.rs:495`), which is
    /// this module's own answer for that field too — so on a REAL vault, F4 swallows it and the
    /// block names four fields, not five. This is not a bug; it is what the happy-path test above
    /// (deliberately starting at `unset` so the field's write is real, not vacuous) does not cover.
    #[test]
    fn on_a_real_vault_effort_confidence_starts_at_low_and_the_block_names_only_four() {
        let _guard = crate::journal::DEVICE_ENV_MUTEX.lock().unwrap();
        let v = vault("prod-shape");
        crate::pystr::write_text(&v.join("tasks").join("hw3.md"), &NOTE.replace("effort_confidence: unset", "effort_confidence: low")).unwrap();
        let log = v.join("_log");
        let m = answered();
        let (code, _) = enrich_with(&v, &opts(&log), Ok(&m));
        assert_eq!(code, 0);
        let meta = meta_of(&v, "hw3.md");
        assert_eq!(crate::yaml::opt_text(crate::yaml::get(&meta, "effort_confidence")).as_deref(), Some("low"), "unchanged, so F4 makes it a no-op");
        let Some(serde_yaml_ng::Value::Mapping(j)) = crate::yaml::get(&meta, "judgment") else { panic!("no judgment") };
        let Some(serde_yaml_ng::Value::Sequence(fields)) = crate::yaml::get(j, "fields") else { panic!("no fields") };
        let names: Vec<String> = fields.iter().filter_map(crate::yaml::text).collect();
        assert_eq!(names, vec!["course", "effort_hours", "importance", "importance_reason"], "effort_confidence never moved, so F4 excluded it");
        let _ = std::fs::remove_dir_all(&v);
    }

    /// D7: no runtime and no model are normal. Whatever tier 1 answered is still written — a course
    /// attributed from the uid pin is worth having — and the flag STAYS SET so the routine, or a
    /// later run with a model, finishes the job.
    #[test]
    fn with_no_model_tier_one_still_writes_and_the_flag_stays_set() {
        let _guard = crate::journal::DEVICE_ENV_MUTEX.lock().unwrap();
        let v = vault("nomodel");
        let log = v.join("_log");
        let (code, lines) = enrich_with(&v, &opts(&log), Err(crate::judge::Missing::Model));
        assert_eq!(code, 0, "a missing model is never a failed slot");
        assert!(lines.iter().any(|l| l.contains("model not installed")), "{lines:?}");
        let meta = meta_of(&v, "hw3.md");
        assert_eq!(crate::yaml::opt_text(crate::yaml::get(&meta, "course")).as_deref(), Some("cs-100"), "the uid pin answered");
        assert_eq!(crate::yaml::get(&meta, "needs_enrichment"), Some(&serde_yaml_ng::Value::Bool(true)), "still owed");
        assert_eq!(crate::yaml::opt_f64(crate::yaml::get(&meta, "effort_hours"), 0.0), 1.0, "the template's value is untouched");
        assert_eq!(crate::judgelog::read_day(&log, &crate::journal::now_ts(None)[..10])[0]["outcome"], "model not installed");
        let _ = std::fs::remove_dir_all(&v);
    }

    /// Running twice must be a no-op the second time: `write`'s F4 rule means an identical value
    /// produces no record and no write, so nothing is re-journalled and no second judgment block
    /// appears.
    #[test]
    fn a_second_run_over_the_same_vault_writes_nothing() {
        let _guard = crate::journal::DEVICE_ENV_MUTEX.lock().unwrap();
        let v = vault("idem");
        let log = v.join("_log");
        enrich_with(&v, &opts(&log), Err(crate::judge::Missing::Model));
        let before = records(&v).len();
        let before_bytes = std::fs::read(v.join("tasks").join("hw3.md")).unwrap();
        let (code, _) = enrich_with(&v, &opts(&log), Err(crate::judge::Missing::Model));
        assert_eq!(code, 0);
        assert_eq!(records(&v).len(), before, "a re-run journalled something");
        assert_eq!(std::fs::read(v.join("tasks").join("hw3.md")).unwrap(), before_bytes, "the note changed on a re-run");
        let _ = std::fs::remove_dir_all(&v);
    }

    /// D3: a field Quinn set comes back as a card, and the note is untouched — the loop the routine
    /// has followed for weeks, now run locally.
    #[test]
    fn a_field_quinn_set_becomes_an_amend_card_and_the_note_is_untouched() {
        let _guard = crate::journal::DEVICE_ENV_MUTEX.lock().unwrap();
        let v = vault("amend");
        let log = v.join("_log");
        let mut journal = crate::journal::Journal::new(&v);
        crate::write::write_literals(&v, "tasks/hw3.md", &[("effort_hours".to_string(), "6.0".to_string())],
            &crate::write::WriteContext::new("quinn", "dashboard"), &mut journal, &crate::write::WriteOpts::default()).unwrap();
        let m = answered();
        let (code, lines) = enrich_with(&v, &opts(&log), Ok(&m));
        assert_eq!(code, 0);
        // S2: the summary must COUNT the card and the item line must NAME it. `contains("proposed")`
        // alone is satisfied by the summary's own `0 proposed`, so it asserted nothing.
        assert!(lines.iter().any(|l| l.contains("amend-")), "the card is named: {lines:?}");
        assert!(lines.last().unwrap().contains("1 proposed"), "{lines:?}");
        let meta = meta_of(&v, "hw3.md");
        assert_eq!(crate::yaml::opt_f64(crate::yaml::get(&meta, "effort_hours"), 0.0), 6.0, "judge-once held");
        assert_eq!(crate::yaml::opt_i64(crate::yaml::get(&meta, "importance"), 0), 4, "the fields Quinn did not set are still written");
        let cards: Vec<_> = std::fs::read_dir(v.join("approvals")).unwrap().flatten().collect();
        assert_eq!(cards.len(), 1, "one card");
        let card = crate::ids::read_meta(&cards[0].path()).unwrap();
        assert_eq!(crate::yaml::opt_text(crate::yaml::get(&card, "kind")).as_deref(), Some("amend"));
        assert_eq!(crate::yaml::opt_text(crate::yaml::get(&card, "created_by")).as_deref(), Some("agent:knowlu.enrich"));
        let _ = std::fs::remove_dir_all(&v);
    }

    /// A model that fails on one item must not stop the batch: the other items are still judged and
    /// the failure is one line and one log record.
    #[test]
    fn one_bad_item_does_not_stop_the_batch() {
        let _guard = crate::journal::DEVICE_ENV_MUTEX.lock().unwrap();
        let v = vault("batch");
        let log = v.join("_log");
        crate::pystr::write_text(&v.join("tasks").join("hw4.md"),
            &NOTE.replace("task_1111111111", "task_4444444444").replace("Homework 3", "Homework 4")).unwrap();
        struct Flaky(std::sync::Mutex<usize>);
        impl crate::judge::Model for Flaky {
            fn judge(&self, _i: &crate::judge::Item, _h: &crate::judge::Heuristics, _s: &crate::judge::Verdict)
                -> Result<crate::judge::Verdict, crate::judge::ModelError> {
                let mut n = self.0.lock().unwrap();
                *n += 1;
                if *n == 1 { return Err(crate::judge::ModelError::Failed("llama-cli: broken pipe".into())); }
                Ok(crate::judge::Verdict { course: None, effort_hours: Some(1.5), importance: Some(3),
                    importance_reason: Some("No weights given, so 3.".into()), confidence: 0.8, tier: 3 })
            }
        }
        let m = Flaky(std::sync::Mutex::new(0));
        let (code, lines) = enrich_with(&v, &opts(&log), Ok(&m));
        assert_eq!(code, 0, "a model failure is never a failed slot");
        assert!(lines.iter().any(|l| l.contains("broken pipe")), "the failure is visible: {lines:?}");
        assert_eq!(crate::judgelog::read_day(&log, &crate::journal::now_ts(None)[..10]).len(), 2, "both judgments logged");
        let _ = std::fs::remove_dir_all(&v);
    }

    /// C1 (Task 7 fix round 1): the runtime's failure text must survive to stdout even when the
    /// seed tier1 hands the model is completely empty — an empty `course_map` plus
    /// `effort_source: inferred`, exactly the shape of a wizard-created friend's vault. Before the
    /// fix, `why` was appended only inside the write's `Ok` arm, which sits AFTER the
    /// `literals.is_empty()` early exit; on this fixture the seed answers nothing at all, so
    /// `literals` is empty and the line read `nothing to write (low confidence)` with the
    /// runtime's stderr tail gone — on exactly the machine the diagnostic exists for.
    #[test]
    fn a_broken_runtimes_stderr_survives_even_when_the_seed_answers_nothing() {
        let _guard = crate::journal::DEVICE_ENV_MUTEX.lock().unwrap();
        let v = vault("nocourse");
        // No course_map entry matches this note's source_uid or title, so tier1's course is also
        // None — the seed is completely empty, `literals_for` produces nothing, and the item takes
        // the early-exit branch this fix covers.
        crate::pystr::write_text(&v.join("config").join("ingest.yaml"), "course_map: {}\n").unwrap();
        let log = v.join("_log");
        struct Broken;
        impl crate::judge::Model for Broken {
            fn judge(&self, _i: &crate::judge::Item, _h: &crate::judge::Heuristics, _s: &crate::judge::Verdict)
                -> Result<crate::judge::Verdict, crate::judge::ModelError> {
                Err(crate::judge::ModelError::Failed(
                    "llama-cli: exited with exit code: 2 — stderr: ERROR: model load failed".into(),
                ))
            }
        }
        let (code, lines) = enrich_with(&v, &opts(&log), Ok(&Broken));
        assert_eq!(code, 0, "a broken runtime is never a failed slot");
        assert!(
            lines.iter().any(|l| l.contains("ERROR: model load failed")),
            "the runtime's stderr must reach stdout even on the empty-seed path: {lines:?}"
        );
        let _ = std::fs::remove_dir_all(&v);
    }

    /// `limit` bounds one run. A vault with two hundred flagged notes on the first launch after a
    /// semester's ingest must not hold a slot for an hour; the rest are taken at the next slot.
    #[test]
    fn the_batch_is_bounded_and_says_how_many_are_left() {
        let _guard = crate::journal::DEVICE_ENV_MUTEX.lock().unwrap();
        let v = vault("limit");
        let log = v.join("_log");
        for i in 0..4 {
            crate::pystr::write_text(&v.join("tasks").join(format!("more{i}.md")),
                &NOTE.replace("task_1111111111", &format!("task_555555555{i}"))).unwrap();
        }
        let mut o = opts(&log);
        o.limit = 2;
        let (code, lines) = enrich_with(&v, &o, Err(crate::judge::Missing::Runtime));
        assert_eq!(code, 0);
        let summary = lines.last().unwrap();
        assert!(summary.contains("2 item"), "{summary}");
        assert!(summary.contains("3 left"), "{summary}");
        let _ = std::fs::remove_dir_all(&v);
    }

    /// M1 (final fix wave): the batch has a wall-clock budget, checked BEFORE each item starts —
    /// distinct from `runtime::CALL_TIMEOUT`, which bounds one call. A model that sleeps past the
    /// budget on its first call must never be handed a second item: the run stops there, folds
    /// what it did not reach into the existing "left for the next slot" line, and still exits 0 —
    /// this is what stands between `DEFAULT_LIMIT * CALL_TIMEOUT` (100 minutes) and
    /// `scheduler::CHILD_TIMEOUT` (20 minutes) killing the slot and painting the tray amber.
    #[test]
    fn a_run_stops_at_its_wall_clock_budget_and_reports_the_remainder() {
        let _guard = crate::journal::DEVICE_ENV_MUTEX.lock().unwrap();
        let v = vault("budget");
        let log = v.join("_log");
        for i in 0..2 {
            crate::pystr::write_text(&v.join("tasks").join(format!("more{i}.md")),
                &NOTE.replace("task_1111111111", &format!("task_666666666{i}"))).unwrap();
        }
        struct Sleepy(Duration);
        impl crate::judge::Model for Sleepy {
            fn judge(&self, _i: &crate::judge::Item, _h: &crate::judge::Heuristics, _s: &crate::judge::Verdict)
                -> Result<crate::judge::Verdict, crate::judge::ModelError> {
                std::thread::sleep(self.0);
                Ok(crate::judge::Verdict {
                    effort_hours: Some(1.5),
                    importance: Some(3),
                    importance_reason: Some("No weights given, so 3.".into()),
                    confidence: 0.8,
                    tier: 3,
                    ..Default::default()
                })
            }
        }
        let mut o = opts(&log);
        o.budget = Duration::from_millis(20);
        let m = Sleepy(Duration::from_millis(300));
        let (code, lines) = enrich_with(&v, &o, Ok(&m));
        assert_eq!(code, 0, "a budget cutoff is never a failed slot");
        let summary = lines.last().unwrap();
        assert!(summary.starts_with("judge: 1 item"), "only the first item should have started: {summary}");
        assert!(summary.contains("2 left"), "the other two must be folded into the existing line: {summary}");
        let _ = std::fs::remove_dir_all(&v);
    }

    /// S8: a whole-number estimate is written `2.0`, never `2`. Rust's `Display` for `2.0_f64` is
    /// `2`, and writing that would retype the field from float to int on a live vault — every other
    /// producer writes a float, and two spellings of one estimate is exactly the silent drift the
    /// single write path exists to stop.
    #[test]
    fn a_whole_number_effort_is_written_as_a_float() {
        let v = crate::judge::Verdict {
            effort_hours: Some(2.0),
            importance: Some(3),
            importance_reason: Some("No weights given, so 3.".to_string()),
            tier: 3,
            ..Default::default()
        };
        let lits = literals_for(&v, true);
        let find = |n: &str| lits.iter().find(|(k, _)| k == n).map(|(_, l)| l.clone());
        assert_eq!(find("effort_hours").as_deref(), Some("2.0"), "Display would have written 2");
        assert_eq!(find("importance").as_deref(), Some("3"), "importance IS an int");
        assert_eq!(find("effort_confidence").as_deref(), Some("\"low\""));
        assert_eq!(find("needs_enrichment").as_deref(), Some("false"));
        assert_eq!(find("course"), None, "an unresolved course writes no literal at all");
        assert_eq!(literals_for(&v, false).iter().find(|(k, _)| k == "needs_enrichment"), None);

        // M1 (Task 7 fix round 1): bound the key set AT THE SOURCE. Reading names back out of a
        // note's `judgment:` block only proves what `write::write_literals` already guarantees
        // (it filters that block to `judged_fields_for(kind)`) — a non-judged field added here
        // (`effort_source`, `status`, ...) would be written and journalled as
        // `agent:knowlu.enrich` while staying invisible to every assertion downstream of `write`.
        for (k, _) in literals_for(&v, true) {
            assert!(
                k == "needs_enrichment" || crate::provenance::JUDGED_FIELDS_TASK.contains(&k.as_str()),
                "{k} is outside JUDGED_FIELDS_TASK ∪ {{needs_enrichment}}"
            );
        }
    }

    /// m5 (final fix wave): `effort_confidence: low` is written only when the effort came from the
    /// model (`tier == 3`). A tier-1 verdict that merely carried an hour figure forward — the shape
    /// `coursework`'s vendor-authored `effort_confidence: high` would be if it ever reached this
    /// path — must not have that field rewritten down to `low`.
    #[test]
    fn effort_confidence_low_is_written_only_when_the_model_answered() {
        let seed = crate::judge::Verdict {
            effort_hours: Some(3.0),
            tier: 1,
            ..Default::default()
        };
        let lits = literals_for(&seed, false);
        assert!(lits.iter().any(|(k, _)| k == "effort_hours"), "the hours are still carried forward");
        assert!(
            !lits.iter().any(|(k, _)| k == "effort_confidence"),
            "a non-model tier must not write effort_confidence: {lits:?}"
        );
    }

    /// A vault with nothing to do says so in one line and touches nothing.
    #[test]
    fn an_empty_queue_is_one_line() {
        let v = vault("empty");
        crate::pystr::write_text(&v.join("tasks").join("hw3.md"), &NOTE.replace("needs_enrichment: true", "needs_enrichment: false")).unwrap();
        let (code, lines) = enrich_with(&v, &opts(&v.join("_log")), Err(crate::judge::Missing::Runtime));
        assert_eq!(code, 0);
        assert_eq!(lines, vec!["judge: nothing to enrich".to_string()]);
        assert!(!v.join("state").join("journal").exists(), "nothing was journalled");
        let _ = std::fs::remove_dir_all(&v);
    }

    /// The CLI wrapper resolves nothing and starts nothing when neither path is given and no env
    /// var is set: it reports and exits 0.
    #[test]
    fn run_lines_with_no_runtime_anywhere_reports_and_exits_zero() {
        let _guard = crate::journal::DEVICE_ENV_MUTEX.lock().unwrap();
        let v = vault("cli");
        // Explicit paths that cannot exist, rather than clearing the environment: `remove_var`
        // mutates process-global state every other test in this binary shares, and they run in
        // parallel. `resolve` prefers the ARGUMENT, so this takes the same branch on a machine that
        // does have KNOWLU_RUNTIME set — which is the machine the smoke test runs on.
        let gone = v.join("does-not-exist");
        let o = Options { via: "cli", run_id: None, runtime: Some(&gone), model: Some(&gone), log_dir: None, limit: 50, budget: BATCH_BUDGET };
        let (code, lines) = run_lines(&v, &o);
        assert_eq!(code, 0);
        assert!(lines.iter().any(|l| l.contains("runtime not installed")), "{lines:?}");
        let _ = std::fs::remove_dir_all(&v);
    }

    /// R-3a-21: `judgelog::Entry` has nowhere to put a title or a body, so no call site can pass
    /// one *directly* — but `fields` carries free text verbatim by design, and this task is the one
    /// place that builds `fields` from a verdict. Build a note whose title and body each carry a
    /// distinctive token, run a full enrichment against a scripted fake, and assert the token
    /// appears in no RAW line of the judgment log (spec §5.6).
    #[test]
    fn the_judgment_log_never_carries_a_notes_title_or_body_text() {
        let _guard = crate::journal::DEVICE_ENV_MUTEX.lock().unwrap();
        let v = vault("privacy");
        let log = v.join("_log");
        let note = NOTE
            .replace("CS-100 Homework 3", "CS-100 Homework 3 ZQXTITLEf7c3")
            .replace("Chapters 4 and 5.", "Chapters 4 and 5. ZQXBODY9d18");
        crate::pystr::write_text(&v.join("tasks").join("hw3.md"), &note).unwrap();
        let m = answered();
        let (code, _) = enrich_with(&v, &opts(&log), Ok(&m));
        assert_eq!(code, 0);
        let raw = raw_log_text(&log);
        assert!(!raw.is_empty(), "the log must have written something to check");
        assert!(!raw.contains("ZQXTITLE"), "the note's title leaked into the judgment log: {raw}");
        assert!(!raw.contains("ZQXBODY"), "the note's body leaked into the judgment log: {raw}");
        let _ = std::fs::remove_dir_all(&v);
    }

    /// The whole `judge` step against a service that is not there: every item still gets tier 1's
    /// answer written, the run says why, and the exit code is 0. This is the case the app actually
    /// meets on a train, and the one a non-zero exit would turn into an amber tray forever.
    ///
    /// R-C2-E14: this vault's one note IS flagged `needs_enrichment: true`, so the write really
    /// happens — `write_literals`/`journal::make_record` both read process-global `device_name()`
    /// (see the comment above `enrichment_writes_the_five_fields_...`), so this test takes the same
    /// lock every other note-writing test in this module takes.
    #[test]
    fn a_service_that_cannot_be_reached_still_writes_tier_one_and_exits_zero() {
        let _guard = crate::journal::DEVICE_ENV_MUTEX.lock().unwrap();
        let v = vault("cloud-down");
        let listener = std::net::TcpListener::bind("127.0.0.1:0").expect("bind");
        let port = listener.local_addr().expect("addr").port();
        drop(listener);
        let cfg = crate::cloudmodel::CloudConfig {
            api_base: format!("http://127.0.0.1:{port}/functions/v1"),
            anon_key: "anon".into(),
            session_credential_target: "knowlu/test/session".into(),
            account_id: "acct-1".into(),
        };
        let client = crate::cloudmodel::CloudClient::new(&cfg, "jwt-not-a-secret");
        let opts = Options {
            via: "local-runner", run_id: None, runtime: None, model: None,
            log_dir: None, limit: 10, budget: BATCH_BUDGET,
        };
        let (code, lines) = run_lines_with(&v, &opts, Some(&client));
        assert_eq!(code, 0, "the judge step always exits 0");
        assert!(lines.iter().any(|l| l.contains("no network")), "{lines:?}");
        let _ = std::fs::remove_dir_all(&v);
    }

    /// R-C2-E15: an empty queue must never reach the service at all — not even the one-call probe.
    /// A slot that runs twice a day forever and has nothing to enrich must not spend a round trip
    /// (and, on a network that black-holes instead of refusing, risk up to `CALL_TIMEOUT` stalling
    /// the slot) proving what `pending`'s own empty result already answers for free.
    #[test]
    fn an_empty_queue_makes_no_request_to_the_service() {
        let _guard = crate::journal::DEVICE_ENV_MUTEX.lock().unwrap();
        let v = vault("cloud-empty");
        // The fixture's one note starts flagged; clear it so `pending` finds nothing.
        crate::pystr::write_text(
            &v.join("tasks").join("hw3.md"),
            &NOTE.replace("needs_enrichment: true", "needs_enrichment: false"),
        ).unwrap();
        let listener = std::net::TcpListener::bind("127.0.0.1:0").expect("bind");
        listener.set_nonblocking(true).expect("nonblocking");
        let port = listener.local_addr().expect("addr").port();
        let cfg = crate::cloudmodel::CloudConfig {
            api_base: format!("http://127.0.0.1:{port}/functions/v1"),
            anon_key: "anon".into(),
            session_credential_target: "knowlu/test/session".into(),
            account_id: "acct-1".into(),
        };
        let client = crate::cloudmodel::CloudClient::new(&cfg, "jwt-not-a-secret");
        let opts = Options {
            via: "local-runner", run_id: None, runtime: None, model: None,
            log_dir: None, limit: 10, budget: BATCH_BUDGET,
        };
        let (code, lines) = run_lines_with(&v, &opts, Some(&client));
        assert_eq!(code, 0);
        assert_eq!(lines, vec!["judge: nothing to enrich".to_string()], "{lines:?}");
        // Non-blocking, not a timed wait: `run_lines_with` above already ran to completion on this
        // thread, so a request — had one been sent to a live loopback listener — would already be
        // sitting in the accept queue. `Err` here means the probe never dialled out at all.
        assert!(
            listener.accept().is_err(),
            "an empty queue must never reach the judgment service"
        );
        let _ = std::fs::remove_dir_all(&v);
    }

    /// A loopback server that answers `replies` in order (by arrival, not by path) and hands back
    /// everything it was sent, joined once at the end — the same discipline
    /// `engine/tests/cloud_contract.rs`'s `Loopback` uses, duplicated here rather than shared
    /// because a `src` unit test cannot depend on a separate test binary.
    fn multi_reply_loopback(replies: Vec<(u16, String)>) -> (String, std::thread::JoinHandle<Vec<String>>) {
        multi_reply_loopback_slow(replies, std::time::Duration::ZERO)
    }

    /// [`multi_reply_loopback`] with the FIRST reply held back by `first_delay` — the one way a
    /// test can spend a measurable amount of the cloud arm's wall clock without sleeping in the
    /// test itself. Used by the budget test (C2 final review E-1), which needs the probe to cost
    /// more than the whole budget.
    fn multi_reply_loopback_slow(
        replies: Vec<(u16, String)>,
        first_delay: std::time::Duration,
    ) -> (String, std::thread::JoinHandle<Vec<String>>) {
        let listener = std::net::TcpListener::bind("127.0.0.1:0").expect("bind the loopback listener");
        // C2 Task 9 fix 2 (R-C2-E22 second pass, minor): non-blocking and polled against a
        // deadline, not a blocking `accept()` — a regression that makes the events pass send fewer
        // requests than `replies` expects must fail this test, not hang the thread (and the test
        // that joins it) forever.
        listener.set_nonblocking(true).expect("nonblocking");
        let port = listener.local_addr().expect("the listener has an address").port();
        let handle = std::thread::spawn(move || {
            use std::io::{BufRead, BufReader, Read, Write};
            let mut seen = Vec::new();
            for (index, (code, body)) in replies.into_iter().enumerate() {
                let deadline = std::time::Instant::now() + std::time::Duration::from_secs(5);
                let mut accepted = None;
                while std::time::Instant::now() < deadline {
                    match listener.accept() {
                        Ok(pair) => {
                            accepted = Some(pair);
                            break;
                        }
                        Err(e) if e.kind() == std::io::ErrorKind::WouldBlock => {
                            std::thread::sleep(std::time::Duration::from_millis(5));
                        }
                        Err(_) => break,
                    }
                }
                let Some((mut stream, _)) = accepted else { break };
                stream.set_nonblocking(false).expect("blocking for the request/response exchange");
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
                let mut body_buf = vec![0u8; length];
                if length > 0 {
                    let _ = reader.read_exact(&mut body_buf);
                }
                seen.push(format!("{head}{}", String::from_utf8_lossy(&body_buf)));
                if index == 0 && !first_delay.is_zero() {
                    std::thread::sleep(first_delay);
                }
                let response = format!(
                    "HTTP/1.1 {code} X\r\ncontent-type: application/json\r\ncontent-length: {}\r\nconnection: close\r\n\r\n{body}",
                    body.len()
                );
                let _ = stream.write_all(response.as_bytes());
                let _ = stream.flush();
            }
            seen
        });
        (format!("http://127.0.0.1:{port}/functions/v1"), handle)
    }

    /// C2 Task 9 fix 1 (R-C2-E22 #7a): the pull phase this task added had no test — every other
    /// `enrich::` test either takes the widened early return or runs against a `vault(tag)` with
    /// no `config/events.yaml` at all. This one gives the vault one enabled event source and an
    /// empty enrichment queue, and proves the whole chain end to end: the probe, the feed fetched
    /// through the server-side proxy, the verdict recorded in the ledger, and the summary line
    /// naming the pass — with no real socket, only a loopback listener answering in order.
    #[test]
    fn the_events_pass_runs_after_an_empty_enrichment_batch_and_the_verdict_reaches_the_ledger() {
        let _guard = crate::journal::DEVICE_ENV_MUTEX.lock().unwrap();
        let v = vault("events-pull");
        // Clear the fixture's one flagged task so the enrichment queue is empty — the events pass
        // must still run because `config/events.yaml` below names an enabled source.
        crate::pystr::write_text(
            &v.join("tasks").join("hw3.md"),
            &NOTE.replace("needs_enrichment: true", "needs_enrichment: false"),
        ).unwrap();
        crate::pystr::write_text(
            &v.join("config").join("events.yaml"),
            "sources:\n  - name: engage\n    type: ics\n    url: https://example.invalid/e.ics\n    enabled: true\n",
        ).unwrap();

        let ics = "BEGIN:VCALENDAR\r\nBEGIN:VEVENT\r\nUID:engage:1\r\nSUMMARY:AI Club\r\n\
                   DTSTART:20260829T230000Z\r\nDTEND:20260830T000000Z\r\n\
                   DESCRIPTION:Come learn ML.\r\nEND:VEVENT\r\nEND:VCALENDAR\r\n";
        let events_reply = crate::ledger::dumps_value(&serde_json::json!({ "body": ics }));
        let judge_reply = crate::ledger::dumps_value(&serde_json::json!({
            "verdict": { "verdict": "opportunity", "why": "matches interests", "confidence": 0.9 },
            "tier": 3
        }));
        // In arrival order: the probe (`GET /judge-rules`), the feed fetch (`POST /events`), the
        // verdict (`POST /judge-event`) — `enrich_with` itself makes no call at all, because its
        // own batch is empty — and, since Task 12, the rule pull's own `GET /judge-rules` at the
        // end of the cloud arm, with nothing decided and nothing offered.
        let (base, handle) = multi_reply_loopback(vec![
            (200, "{}".to_string()),
            (200, events_reply),
            (200, judge_reply),
            (200, crate::ledger::dumps_value(&serde_json::json!({ "proposals": [] }))),
        ]);
        let cfg = crate::cloudmodel::CloudConfig {
            api_base: base,
            anon_key: "anon".into(),
            session_credential_target: "knowlu/test/session".into(),
            account_id: "acct-1".into(),
        };
        let client = crate::cloudmodel::CloudClient::new(&cfg, "jwt-not-a-secret");
        let log = v.join("_log");
        let o = opts(&log);
        let (code, lines) = run_lines_with(&v, &o, Some(&client));
        assert_eq!(code, 0);
        assert!(lines.iter().any(|l| l == "judge: nothing to enrich"), "{lines:?}");
        assert!(lines.iter().any(|l| l.starts_with("events:")), "the pass must name itself: {lines:?}");
        let ledger = crate::eventledger::load_ledger(&v, None);
        assert_eq!(ledger["ics:engage:1"].verdict.as_deref(), Some("opportunity"), "{ledger:?}");

        let requests = handle.join().expect("the listener thread did not panic");
        assert_eq!(requests.len(), 4, "{requests:?}");
        assert!(requests[0].starts_with("GET /functions/v1/judge-rules"), "{}", requests[0]);
        assert!(requests[1].starts_with("POST /functions/v1/events"), "{}", requests[1]);
        assert!(requests[2].starts_with("POST /functions/v1/judge-event"), "{}", requests[2]);
        assert!(requests[3].starts_with("GET /functions/v1/judge-rules"), "{}", requests[3]);
        let _ = std::fs::remove_dir_all(&v);
    }

    // -----------------------------------------------------------------------------------------
    // C2 Task 11 — the five email tiers, judged server-side, pulled by the slot.
    // -----------------------------------------------------------------------------------------

    /// An approved Gmail card must produce the SAME note a `tier: task` message produces, or there
    /// are two note writers and one of them will drift.
    ///
    /// **Compared without `id:`** — `write::create` mints a fresh opaque id into the frontmatter of
    /// every note it creates, so a byte-for-byte comparison would fail by construction. The id is
    /// the one line that is *supposed* to differ; everything else is the contract.
    #[test]
    fn an_approved_gmail_card_materialises_the_same_note_a_task_tier_would() {
        let _guard = crate::journal::DEVICE_ENV_MUTEX.lock().unwrap();
        let vault = vault("gmail-card");
        // `vault(tag)` seeds one fixture task (`hw3.md`) for the enrichment-focused tests above;
        // this test's own `find` below picks out "whichever task is not `direct`", so an empty
        // `tasks/` is what it needs.
        std::fs::remove_file(vault.join("tasks").join("hw3.md")).unwrap();
        let item = crate::cloudmodel::GmailItem {
            uid: "gmail:m1".into(), tier: "borderline".into(),
            title: "PH 106 problem set 4".into(), course: Some("ph-106".into()),
            due: Some("2026-09-11".into()), effort_hours: Some(2.5), importance: Some(4),
            why: "the email states a Friday deadline".into(), confidence: 0.86,
        };
        let ctx = WriteContext { actor: GMAIL_ACTOR.into(), via: "local-runner".into(), run_id: None };
        let mut journal = Journal::new(&vault);
        let today = jiff::civil::date(2026, 9, 9);

        let direct = write_gmail_note(&vault, &item, &ctx, &mut journal).expect("the note writes");
        let card = write_gmail_card(&vault, &item, today, &ctx, &mut journal).expect("the card writes");
        // Approve it exactly as the deck would, then let `process_approvals` materialise it.
        let rel = format!("approvals/{card}.md");
        crate::write::write_literals(
            &vault, &rel, &[("status".to_string(), "approved".to_string())],
            &ctx, &mut journal, &WriteOpts::default(),
        )
        .expect("approve");
        let _ = crate::approvals::process_approvals(
            &vault, today, jiff::civil::date(2026, 9, 9).at(9, 0, 0, 0), &ctx, &mut journal,
        );

        let strip_id = |text: &str| {
            text.lines().filter(|l| !l.starts_with("id:")).collect::<Vec<_>>().join("\n")
        };
        let from_tier = std::fs::read_to_string(vault.join("tasks").join(format!("{direct}.md"))).unwrap();
        let materialised = crate::approvals::sorted_md(&vault.join("tasks"))
            .into_iter()
            .find(|p| p.file_stem().map(|s| s != direct.as_str()).unwrap_or(false))
            .expect("the card produced a note");
        let from_card = std::fs::read_to_string(&materialised).unwrap();
        assert_eq!(strip_id(&from_tier), strip_id(&from_card));
        let _ = std::fs::remove_dir_all(&vault);
    }

    /// The 15-a-day cap is the ENGINE's, and Gmail proposals go through the ordinary card path so
    /// it applies to them unchanged — which is the narrowing recorded in the fidelity ledger.
    #[test]
    fn an_over_budget_gmail_batch_is_snoozed_not_dropped() {
        let _guard = crate::journal::DEVICE_ENV_MUTEX.lock().unwrap();
        let vault = vault("gmail-budget");
        let ctx = WriteContext { actor: GMAIL_ACTOR.into(), via: "local-runner".into(), run_id: None };
        let mut journal = Journal::new(&vault);
        let today = jiff::civil::date(2026, 9, 9);
        for n in 0..20 {
            let item = crate::cloudmodel::GmailItem {
                uid: format!("gmail:m{n}"), tier: "opportunity".into(),
                title: format!("Opportunity {n}"), course: None, due: None,
                effort_hours: None, importance: None, why: "worth a look".into(), confidence: 0.8,
            };
            write_gmail_card(&vault, &item, today, &ctx, &mut journal).expect("card");
        }
        let deferred = crate::approvals::defer_over_budget(&vault, today, 15, &ctx, &mut journal);
        assert_eq!(deferred.len(), 5, "the surplus is snoozed to tomorrow, never deleted");
        let _ = std::fs::remove_dir_all(&vault);
    }

    /// A loopback server that answers one `/gmail-read` reply per accepted connection, in arrival
    /// order — [`multi_reply_loopback`] under a name that reads at the call site, since every test
    /// below is scripting the pull's own endpoint and nothing else.
    fn gmail_loopback(replies: Vec<(u16, String)>) -> (String, std::thread::JoinHandle<Vec<String>>) {
        multi_reply_loopback(replies)
    }

    fn gmail_reply(items: &str, more: bool) -> (u16, String) {
        (200, crate::ledger::dumps_value(&serde_json::json!({
            "items": serde_json::from_str::<serde_json::Value>(items).expect("valid items json"),
            "read": 0, "quiet": false, "more": more,
        })))
    }

    fn client_for(base: String) -> crate::cloudmodel::CloudClient {
        let cfg = crate::cloudmodel::CloudConfig {
            api_base: base,
            anon_key: "anon".into(),
            session_credential_target: "knowlu/test/session".into(),
            account_id: "acct-1".into(),
        };
        crate::cloudmodel::CloudClient::new(&cfg, "jwt-not-a-secret")
    }

    /// A scripted `127.0.0.1:0` listener answering `replies` in order, as a `CloudClient` already
    /// aimed at it — [`multi_reply_loopback`] plus [`client_for`], under the name C2 Task 12's own
    /// tests call it. `LoopbackServer::requests` joins the listener thread once, the same discipline
    /// [`multi_reply_loopback`]'s other callers use directly.
    struct LoopbackServer(Option<std::thread::JoinHandle<Vec<String>>>);
    impl LoopbackServer {
        fn requests(&mut self) -> Vec<String> {
            self.0.take().map(|h| h.join().expect("the listener thread did not panic")).unwrap_or_default()
        }
    }
    fn loopback_client(replies: Vec<(u16, String)>) -> (crate::cloudmodel::CloudClient, LoopbackServer) {
        let (base, handle) = multi_reply_loopback(replies);
        (client_for(base), LoopbackServer(Some(handle)))
    }

    /// The whole pull, end to end: one `tier: task` item becomes a note through `write::create`,
    /// `created_by: gmail`, `source_uid: gmail:<message-id>`, `needs_enrichment: false` — and the
    /// device acknowledges it on the very next request, exactly as [`pull_gmail`]'s doc comment
    /// promises.
    #[test]
    fn a_clear_task_email_becomes_a_note_with_created_by_gmail() {
        let _guard = crate::journal::DEVICE_ENV_MUTEX.lock().unwrap();
        let v = vault("gmail-task");
        let (base, handle) = gmail_loopback(vec![
            gmail_reply(
                r#"[{"uid":"gmail:m1","tier":"task","payload":{"title":"PH 106 problem set 4","course":"ph-106","due":"2026-09-11","effort_hours":2.5,"importance":4,"why":"the email states a Friday deadline","confidence":0.86}}]"#,
                false,
            ),
            gmail_reply("[]", false),
        ]);
        let client = client_for(base);
        let log = v.join("_log");
        let lines = pull_gmail(&v, &client, &opts(&log), BATCH_BUDGET);

        let meta = meta_of(&v, "ph-106-problem-set-4.md");
        assert_eq!(crate::yaml::opt_text(crate::yaml::get(&meta, "created_by")).as_deref(), Some("gmail"));
        assert_eq!(crate::yaml::opt_text(crate::yaml::get(&meta, "source_uid")).as_deref(), Some("gmail:m1"));
        assert_eq!(crate::yaml::opt_text(crate::yaml::get(&meta, "course")).as_deref(), Some("ph-106"));
        assert_eq!(crate::yaml::opt_f64(crate::yaml::get(&meta, "effort_hours"), 0.0), 2.5);
        assert_eq!(crate::yaml::opt_i64(crate::yaml::get(&meta, "importance"), 0), 4);
        assert_eq!(crate::yaml::get(&meta, "needs_enrichment"), Some(&serde_yaml_ng::Value::Bool(false)));
        assert_eq!(lines.last().unwrap(), "gmail: 1 task(s), 0 proposed, 0 dropped as information", "{lines:?}");

        let requests = handle.join().expect("the listener thread did not panic");
        assert_eq!(requests.len(), 2, "one pull, then one ack flush: {requests:?}");
        assert!(requests[1].contains("gmail:m1"), "the ack flush must name the uid it wrote: {}", requests[1]);
        let _ = std::fs::remove_dir_all(&v);
    }

    /// `borderline` (and, by the same code path, `event` and `opportunity`) becomes a `kind: task`
    /// approval card, never a note directly — the student decides, not the pull.
    #[test]
    fn a_borderline_email_becomes_a_proposal_and_not_a_note() {
        let _guard = crate::journal::DEVICE_ENV_MUTEX.lock().unwrap();
        let v = vault("gmail-borderline");
        let (base, handle) = gmail_loopback(vec![
            gmail_reply(
                r#"[{"uid":"gmail:m2","tier":"borderline","payload":{"title":"CS midterm review session","course":"cs-100","due":null,"effort_hours":1.0,"importance":3,"why":"might be worth attending","confidence":0.6}}]"#,
                false,
            ),
            gmail_reply("[]", false),
        ]);
        let client = client_for(base);
        let log = v.join("_log");
        let lines = pull_gmail(&v, &client, &opts(&log), BATCH_BUDGET);

        assert!(
            !v.join("tasks").join("cs-midterm-review-session.md").exists(),
            "a borderline tier must not write a note directly"
        );
        let meta = meta_of_approval(&v, "task-cs-midterm-review-session.md");
        assert_eq!(crate::yaml::opt_text(crate::yaml::get(&meta, "kind")).as_deref(), Some("task"));
        assert_eq!(crate::yaml::opt_text(crate::yaml::get(&meta, "status")).as_deref(), Some("pending"));
        assert_eq!(crate::yaml::opt_text(crate::yaml::get(&meta, "source_uid")).as_deref(), Some("gmail:m2"));
        assert_eq!(lines.last().unwrap(), "gmail: 0 task(s), 1 proposed, 0 dropped as information", "{lines:?}");

        let requests = handle.join().expect("the listener thread did not panic");
        assert_eq!(requests.len(), 2, "{requests:?}");
        let _ = std::fs::remove_dir_all(&v);
    }

    /// `information` is the noise tier: nothing is written, and the uid goes into
    /// `state/ingest-seen.md` — the same ledger `ingest` uses — so a re-pull that somehow re-offers
    /// the same uid is recognised locally and never turned into a second write or a second ask.
    #[test]
    fn an_information_email_writes_nothing_but_is_never_asked_about_twice() {
        let _guard = crate::journal::DEVICE_ENV_MUTEX.lock().unwrap();
        let v = vault("gmail-information");
        let (base, handle) = gmail_loopback(vec![
            gmail_reply(
                r#"[{"uid":"gmail:m3","tier":"information","payload":{"title":"Weekly newsletter","course":null,"due":null,"effort_hours":null,"importance":null,"why":"a newsletter","confidence":0.95}}]"#,
                false,
            ),
            gmail_reply("[]", false),
        ]);
        let client = client_for(base);
        let log = v.join("_log");
        let lines = pull_gmail(&v, &client, &opts(&log), BATCH_BUDGET);

        assert!(!v.join("tasks").join("weekly-newsletter.md").exists());
        assert!(!v.join("approvals").join("task-weekly-newsletter.md").exists());
        assert!(
            crate::ingest::load_seen(&v).contains("gmail:m3"),
            "the uid must be recorded so this message is never asked about again"
        );
        assert_eq!(lines.last().unwrap(), "gmail: 0 task(s), 0 proposed, 1 dropped as information", "{lines:?}");

        // R-C2-E44: an information-tier uid is recorded with a fixed title, never the model's —
        // the note it would have named was dropped exactly because it was not worth writing.
        let ledger = crate::pystr::read_text(&v.join("state").join("ingest-seen.md")).unwrap();
        assert!(ledger.contains("gmail:m3 \u{b7} (email) \u{b7}"), "{ledger}");
        assert!(!ledger.contains("Weekly newsletter"), "{ledger}");

        let requests = handle.join().expect("the listener thread did not panic");
        assert_eq!(requests.len(), 2, "one pull, then one ack flush: {requests:?}");
        let _ = std::fs::remove_dir_all(&v);
    }

    /// R-C2-E45 (renamed from `nothing_is_acknowledged_that_was_not_written`, which now names a
    /// different, per-item test below): a round that fails after an earlier round already wrote
    /// something. The write stands (it is on disk and in `state/ingest-seen.md` either way), but
    /// its uid never completes a SUCCESSFUL acknowledgement round trip, and the run says so rather
    /// than pretending nothing happened.
    ///
    /// Scripted as two requests: the first hands over one `task` item and says `more: true`, so
    /// `pull_gmail` loops for a second round carrying that item's uid in `ack`; the listener answers
    /// that second request with a 500. `pull_gmail` must stop there — no third "flush" request is
    /// ever attempted.
    #[test]
    fn a_failed_round_stops_the_pull_and_says_so() {
        let _guard = crate::journal::DEVICE_ENV_MUTEX.lock().unwrap();
        let v = vault("gmail-partial-500");
        let (base, handle) = gmail_loopback(vec![
            gmail_reply(
                r#"[{"uid":"gmail:m1","tier":"task","payload":{"title":"PH 106 problem set 4","course":"ph-106","due":"2026-09-11","effort_hours":2.5,"importance":4,"why":"the email states a Friday deadline","confidence":0.86}}]"#,
                true,
            ),
            (500, "{\"error\":\"boom\"}".to_string()),
        ]);
        let client = client_for(base);
        let log = v.join("_log");
        let lines = pull_gmail(&v, &client, &opts(&log), BATCH_BUDGET);

        assert!(
            v.join("tasks").join("ph-106-problem-set-4.md").exists(),
            "the first round's item was really written before the second round failed"
        );
        assert!(
            crate::ingest::load_seen(&v).contains("gmail:m1"),
            "written locally even though the server never confirmed the ack"
        );
        assert!(
            lines.iter().any(|l| l.starts_with("gmail: skipped (")),
            "the failure must reach the run's own summary: {lines:?}"
        );

        let requests = handle.join().expect("the listener thread did not panic");
        assert_eq!(
            requests.len(), 2,
            "no third, final ack-flush request may follow a round that already failed: {requests:?}"
        );
        let _ = std::fs::remove_dir_all(&v);
    }

    fn meta_of_approval(v: &Path, name: &str) -> serde_yaml_ng::Mapping {
        crate::ids::read_meta(&v.join("approvals").join(name)).unwrap()
    }

    /// IMPORTANT 7: a WRITE failure, not a network one — the first item's write succeeds and the
    /// second's fails, within the SAME batch. The failed uid must be absent from `ack` (never
    /// acknowledged) while the first item's uid still is, and the summary counts the failure.
    ///
    /// The second item is `borderline` (writes into `approvals/`), pre-created as a FILE rather
    /// than a directory so `create_dir_all` fails for it deterministically — while the first item
    /// (`task`, `tasks/`) is a different directory entirely and is unaffected.
    #[test]
    fn nothing_is_acknowledged_that_was_not_written() {
        let _guard = crate::journal::DEVICE_ENV_MUTEX.lock().unwrap();
        let v = vault("gmail-write-fails");
        std::fs::remove_dir_all(v.join("approvals")).unwrap();
        crate::pystr::write_text(&v.join("approvals"), "not a directory").unwrap();
        let (base, handle) = gmail_loopback(vec![
            gmail_reply(
                r#"[
                    {"uid":"gmail:m1","tier":"task","payload":{"title":"PH 106 problem set 4","course":"ph-106","due":"2026-09-11","effort_hours":2.5,"importance":4,"why":"the email states a Friday deadline","confidence":0.86}},
                    {"uid":"gmail:m2","tier":"borderline","payload":{"title":"CS midterm review session","course":"cs-100","due":null,"effort_hours":1.0,"importance":3,"why":"might be worth attending","confidence":0.6}}
                ]"#,
                false,
            ),
            gmail_reply("[]", false),
        ]);
        let client = client_for(base);
        let log = v.join("_log");
        let lines = pull_gmail(&v, &client, &opts(&log), BATCH_BUDGET);

        assert!(v.join("tasks").join("ph-106-problem-set-4.md").exists(), "the first item must still be written");
        assert!(
            lines.iter().any(|l| l.starts_with("gmail gmail:m2: not written (")),
            "the second item's failure must be named: {lines:?}"
        );
        assert!(lines.last().unwrap().ends_with(", 1 not written"), "{lines:?}");

        let requests = handle.join().expect("the listener thread did not panic");
        assert_eq!(requests.len(), 2, "{requests:?}");
        assert!(requests[1].contains("gmail:m1"), "the written uid must be acknowledged: {}", requests[1]);
        assert!(!requests[1].contains("gmail:m2"), "the failed uid must be absent from ack: {}", requests[1]);
        let _ = std::fs::remove_dir_all(&v);
    }

    /// R-C2-E42: a zero-length remaining budget must stop `pull_gmail` before its first round even
    /// asks — the round check happens before the network call, not after. C2 final review E-4
    /// narrowed what it SAYS when it stops: `gmail: 0 queued, left for the next slot` read as a
    /// loss, when in fact nothing had been pulled and so nothing was left behind. The property
    /// R-C2-E42 exists for is the empty request list below; the line is only reported when this run
    /// really did queue something it is now leaving for the next slot.
    #[test]
    fn the_gmail_pull_stops_before_its_first_round_when_the_slot_has_no_budget_left() {
        let _guard = crate::journal::DEVICE_ENV_MUTEX.lock().unwrap();
        let v = vault("gmail-nobudget");
        let (base, handle) = gmail_loopback(vec![]);
        let client = client_for(base);
        let log = v.join("_log");
        let lines = pull_gmail(&v, &client, &opts(&log), std::time::Duration::ZERO);
        assert!(
            !lines.iter().any(|l| l.contains("left for the next slot")),
            "nothing was queued, so nothing was left for the next slot: {lines:?}"
        );
        let requests = handle.join().expect("the listener thread did not panic");
        assert!(requests.is_empty(), "a zero budget must make no request at all: {requests:?}");
        let _ = std::fs::remove_dir_all(&v);
    }

    /// R-C2-E43: a `due` that could not have come from the server's own validated verdict (here,
    /// an injection attempt against the note's own frontmatter) is skipped, not written and not
    /// acknowledged, and the summary counts it — the same "not written" recovery every other
    /// per-item failure gets.
    #[test]
    fn a_malformed_due_is_skipped_not_written_and_not_acknowledged() {
        let _guard = crate::journal::DEVICE_ENV_MUTEX.lock().unwrap();
        let v = vault("gmail-bad-due");
        let (base, handle) = gmail_loopback(vec![
            gmail_reply(
                r#"[{"uid":"gmail:m1","tier":"task","payload":{"title":"PH 106 problem set 4","course":"ph-106","due":"2026-09-11\nstatus: done","effort_hours":2.5,"importance":4,"why":"the email states a Friday deadline","confidence":0.86}}]"#,
                false,
            ),
            gmail_reply("[]", false),
        ]);
        let client = client_for(base);
        let log = v.join("_log");
        let lines = pull_gmail(&v, &client, &opts(&log), BATCH_BUDGET);

        assert!(!v.join("tasks").join("ph-106-problem-set-4.md").exists(), "a malformed due must not be written");
        assert!(!crate::ingest::load_seen(&v).contains("gmail:m1"), "and must not be marked seen");
        assert!(lines.last().unwrap().ends_with(", 1 skipped as malformed"), "{lines:?}");

        let requests = handle.join().expect("the listener thread did not panic");
        assert_eq!(requests.len(), 1, "the empty ack means no items were ever ready to flush: {requests:?}");
        let _ = std::fs::remove_dir_all(&v);
    }

    /// R-C2-E44 (minor): a duplicate queue row for the SAME uid, within the SAME batch, must
    /// produce exactly one note — the per-batch guard is checked, not only the disk-persisted one.
    #[test]
    fn a_duplicate_queue_row_in_the_same_batch_produces_only_one_note() {
        let _guard = crate::journal::DEVICE_ENV_MUTEX.lock().unwrap();
        let v = vault("gmail-duplicate-row");
        let (base, handle) = gmail_loopback(vec![
            gmail_reply(
                r#"[
                    {"uid":"gmail:m1","tier":"task","payload":{"title":"PH 106 problem set 4","course":"ph-106","due":"2026-09-11","effort_hours":2.5,"importance":4,"why":"the email states a Friday deadline","confidence":0.86}},
                    {"uid":"gmail:m1","tier":"task","payload":{"title":"PH 106 problem set 4","course":"ph-106","due":"2026-09-11","effort_hours":2.5,"importance":4,"why":"the email states a Friday deadline","confidence":0.86}}
                ]"#,
                false,
            ),
            gmail_reply("[]", false),
        ]);
        let client = client_for(base);
        let log = v.join("_log");
        let lines = pull_gmail(&v, &client, &opts(&log), BATCH_BUDGET);

        assert!(v.join("tasks").join("ph-106-problem-set-4.md").exists());
        assert!(!v.join("tasks").join("ph-106-problem-set-4-2.md").exists(), "a duplicate row must not mint a second note");
        assert_eq!(lines.last().unwrap(), "gmail: 1 task(s), 0 proposed, 0 dropped as information", "{lines:?}");
        let requests = handle.join().expect("the listener thread did not panic");
        assert_eq!(requests.len(), 2, "one pull, then one ack flush carrying the uid once: {requests:?}");
        let _ = std::fs::remove_dir_all(&v);
    }

    /// A duplicate row whose FIRST occurrence in the batch already failed must not be acked by a
    /// later occurrence of the same uid — R-C2-E45 (2). The first `m1` is malformed (skipped,
    /// counted, not written); the second `m1` must be skipped silently: no re-attempt, no second
    /// count, and — critically — no ack, because nothing was ever written or seen-recorded for it.
    #[test]
    fn a_duplicate_row_whose_first_occurrence_failed_is_never_acknowledged() {
        let _guard = crate::journal::DEVICE_ENV_MUTEX.lock().unwrap();
        let v = vault("gmail-duplicate-row-failed");
        let (base, handle) = gmail_loopback(vec![
            gmail_reply(
                r#"[
                    {"uid":"gmail:m1","tier":"task","payload":{"title":"PH 106 problem set 4","course":"ph-106","due":"2026-09-11\nstatus: done","effort_hours":2.5,"importance":4,"why":"the email states a Friday deadline","confidence":0.86}},
                    {"uid":"gmail:m1","tier":"task","payload":{"title":"PH 106 problem set 4","course":"ph-106","due":"2026-09-11\nstatus: done","effort_hours":2.5,"importance":4,"why":"the email states a Friday deadline","confidence":0.86}}
                ]"#,
                false,
            ),
            gmail_reply("[]", false),
        ]);
        let client = client_for(base);
        let log = v.join("_log");
        let lines = pull_gmail(&v, &client, &opts(&log), BATCH_BUDGET);

        assert!(!v.join("tasks").join("ph-106-problem-set-4.md").exists());
        assert!(!crate::ingest::load_seen(&v).contains("gmail:m1"));
        assert_eq!(
            lines.iter().filter(|l| l.starts_with("gmail gmail:m1: not written (")).count(), 1,
            "the second occurrence must not be re-counted: {lines:?}"
        );
        assert!(lines.last().unwrap().ends_with(", 1 skipped as malformed"), "{lines:?}");

        let requests = handle.join().expect("the listener thread did not panic");
        assert_eq!(requests.len(), 1, "an empty ack means no second, ack-flush request: {requests:?}");
        let _ = std::fs::remove_dir_all(&v);
    }

    // ---------------------------------------------------------------------------------------------
    // R-C2-E41 open clause — the pull's own quiet behaviours, driven through the loopback rather
    // than asserted only at the server (Deno) layer.
    // ---------------------------------------------------------------------------------------------

    /// `reason: "no_gmail_scope"` is not a failure at all — a calendar-only grant, the routine
    /// account shape while the wizard has no Gmail button yet — so `pull_gmail` must return no
    /// line, write nothing, and never attempt a second (ack-flush) request.
    #[test]
    fn a_quiet_no_gmail_scope_reply_prints_no_line_and_makes_no_second_request() {
        let _guard = crate::journal::DEVICE_ENV_MUTEX.lock().unwrap();
        let v = vault("gmail-quiet-no-scope");
        let body = crate::ledger::dumps_value(&serde_json::json!({
            "items": [], "read": 0, "quiet": true, "reason": "no_gmail_scope", "more": false,
        }));
        let (base, handle) = gmail_loopback(vec![(200, body)]);
        let client = client_for(base);
        let log = v.join("_log");
        let lines = pull_gmail(&v, &client, &opts(&log), BATCH_BUDGET);
        assert!(lines.is_empty(), "{lines:?}");
        assert!(!lines.iter().any(|l| l.contains("HTTP 200")), "{lines:?}");
        // `vault(tag)` seeds one fixture task (`hw3.md`); nothing Gmail-derived was written.
        let tasks: Vec<_> = std::fs::read_dir(v.join("tasks")).unwrap().flatten().collect();
        assert_eq!(tasks.len(), 1, "nothing beyond the fixture task was written: {tasks:?}");
        let requests = handle.join().expect("the listener thread did not panic");
        assert_eq!(requests.len(), 1, "no ack-flush request when there was nothing to ack: {requests:?}");
        let _ = std::fs::remove_dir_all(&v);
    }

    /// `reason: "revoked"` IS the failure the student can act on, and reads as exactly one line —
    /// through `label()`, never a raw HTTP status.
    #[test]
    fn a_quiet_revoked_reply_prints_exactly_one_skipped_line() {
        let _guard = crate::journal::DEVICE_ENV_MUTEX.lock().unwrap();
        let v = vault("gmail-quiet-revoked");
        let body = crate::ledger::dumps_value(&serde_json::json!({
            "items": [], "read": 0, "quiet": true, "reason": "revoked", "more": false,
        }));
        let (base, handle) = gmail_loopback(vec![(200, body)]);
        let client = client_for(base);
        let log = v.join("_log");
        let lines = pull_gmail(&v, &client, &opts(&log), BATCH_BUDGET);
        assert_eq!(lines, vec!["gmail: skipped (gmail is not connected; re-connect from settings)".to_string()]);
        assert!(!lines.iter().any(|l| l.contains("HTTP 200")), "{lines:?}");
        let requests = handle.join().expect("the listener thread did not panic");
        assert_eq!(requests.len(), 1, "{requests:?}");
        let _ = std::fs::remove_dir_all(&v);
    }

    /// The service's 503 for an unconfigured deployment (no P2) reads as exactly one line, through
    /// `QuietReason::NotConfigured`'s `label()` — never a raw HTTP status either.
    #[test]
    fn a_503_not_configured_reply_prints_exactly_one_skipped_line() {
        let _guard = crate::journal::DEVICE_ENV_MUTEX.lock().unwrap();
        let v = vault("gmail-503-not-configured");
        let body = crate::ledger::dumps_value(&serde_json::json!({
            "error": "Google sign-in is not configured on this deployment",
        }));
        let (base, handle) = gmail_loopback(vec![(503, body)]);
        let client = client_for(base);
        let log = v.join("_log");
        let lines = pull_gmail(&v, &client, &opts(&log), BATCH_BUDGET);
        assert_eq!(lines, vec!["gmail: skipped (google sign-in is not configured on this deployment)".to_string()]);
        assert!(!lines.iter().any(|l| l.contains("HTTP 200")), "{lines:?}");
        let requests = handle.join().expect("the listener thread did not panic");
        assert_eq!(requests.len(), 1, "{requests:?}");
        let _ = std::fs::remove_dir_all(&v);
    }

    /// R-C2-E45 (3): a 503 whose body does NOT carry the exact "not configured" text is a platform
    /// failure (a redeploy, a gateway hiccup) and must stay a genuine `CloudError::Status` — never
    /// misread as "Google sign-in is not configured".
    #[test]
    fn a_bare_503_is_not_read_as_not_configured() {
        let _guard = crate::journal::DEVICE_ENV_MUTEX.lock().unwrap();
        let v = vault("gmail-bare-503");
        let (base, handle) = gmail_loopback(vec![(503, "{}".to_string())]);
        let client = client_for(base);
        let log = v.join("_log");
        let lines = pull_gmail(&v, &client, &opts(&log), BATCH_BUDGET);
        assert_eq!(lines.len(), 1, "{lines:?}");
        assert!(lines[0].contains("HTTP 503"), "{lines:?}");
        assert!(!lines[0].contains("not configured"), "{lines:?}");
        let requests = handle.join().expect("the listener thread did not panic");
        assert_eq!(requests.len(), 1, "{requests:?}");
        let _ = std::fs::remove_dir_all(&v);
    }

    /// R-C2-E38, R-C2-E43: the Gmail pull's local precondition. A vault that has linked a Google
    /// calendar — its `config/ingest.yaml` carries the `cloud:google` entry hand-off H9 writes —
    /// asks the service even with an empty enrichment queue and no event source; a vault that
    /// never linked one asks nothing at all, exactly as before Task 11. And the predicate itself
    /// never writes: no `state/calendar.md` on a vault that had none.
    #[test]
    fn the_gmail_pull_runs_only_when_the_vault_has_linked_a_google_calendar() {
        let _guard = crate::journal::DEVICE_ENV_MUTEX.lock().unwrap();

        // Linked: the early return must be skipped, so the probe AND the pull both fire.
        let connected = vault("gmail-predicate-connected");
        crate::pystr::write_text(
            &connected.join("tasks").join("hw3.md"),
            &NOTE.replace("needs_enrichment: true", "needs_enrichment: false"),
        ).unwrap();
        crate::pystr::write_text(
            &connected.join("config").join("ingest.yaml"),
            "calendars:\n  - name: google\n    ics_url: 'cloud:google'\n",
        ).unwrap();
        // The probe, then the Gmail pull, then Task 12's own rule pull at the end of the cloud
        // arm (nothing decided, nothing offered).
        let (base, handle) = gmail_loopback(vec![
            (200, "{}".to_string()),
            gmail_reply("[]", false),
            (200, crate::ledger::dumps_value(&serde_json::json!({ "proposals": [] }))),
        ]);
        let client = client_for(base);
        let log = connected.join("_log");
        let (code, _lines) = run_lines_with(&connected, &opts(&log), Some(&client));
        assert_eq!(code, 0);
        let requests = handle.join().expect("the listener thread did not panic");
        assert_eq!(requests.len(), 3, "the probe, the pull, then the rule pull: {requests:?}");
        assert!(requests[0].starts_with("GET /functions/v1/judge-rules"), "{}", requests[0]);
        assert!(requests[1].starts_with("POST /functions/v1/gmail-read"), "{}", requests[1]);
        assert!(requests[2].starts_with("GET /functions/v1/judge-rules"), "{}", requests[2]);
        assert!(
            !connected.join("state").join("calendar.md").exists(),
            "R-C2-E43: the predicate is a pure config read and must never write a snapshot"
        );
        let _ = std::fs::remove_dir_all(&connected);

        // Not linked: the widened early return still applies, exactly as it did before this
        // task — no request of any kind, `judge` included.
        let unconnected = vault("gmail-predicate-unconnected");
        crate::pystr::write_text(
            &unconnected.join("tasks").join("hw3.md"),
            &NOTE.replace("needs_enrichment: true", "needs_enrichment: false"),
        ).unwrap();
        let (base2, handle2) = gmail_loopback(vec![]);
        let client2 = client_for(base2);
        let log2 = unconnected.join("_log");
        let (code2, lines2) = run_lines_with(&unconnected, &opts(&log2), Some(&client2));
        assert_eq!(code2, 0);
        assert_eq!(lines2, vec!["judge: nothing to enrich".to_string()], "{lines2:?}");
        let requests2 = handle2.join().expect("the listener thread did not panic");
        assert!(requests2.is_empty(), "no judge request either way: {requests2:?}");
        let _ = std::fs::remove_dir_all(&unconnected);
    }

    /// C2 final review E-2 (closing deferred m5): `outcome: "capped"` is the account's daily
    /// judgment cap, not fifty model failures. Before this fix the three verdict-null arms read
    /// only `cause` — which a capped reply leaves null — so every item in the batch made its own
    /// request, got `the judgment service answered no verdict`, and logged `model failed`. Now the
    /// first capped reply sets the same fatal flag a 402 sets: no further request is made, one
    /// summary line names the cap, and the judgment log says `capped`.
    #[test]
    fn a_capped_reply_stops_the_batch_after_one_request_and_the_log_says_capped() {
        let _guard = crate::journal::DEVICE_ENV_MUTEX.lock().unwrap();
        let v = vault("cloud-capped");
        // A second flagged note, so "stopped after one request" is a claim with something to stop.
        crate::pystr::write_text(
            &v.join("tasks").join("hw4.md"),
            &NOTE.replace("Homework 3", "Homework 4").replace("task_1111111111", "task_2222222222"),
        ).unwrap();
        let capped = crate::ledger::dumps_value(&serde_json::json!({
            "verdict": null, "outcome": "capped", "cause": null, "tier": 3,
        }));
        let (client, mut server) = loopback_client(vec![
            (200, crate::ledger::dumps_value(&serde_json::json!({ "proposals": [] }))), // the probe
            (200, capped),                                                              // /judge-task
            (200, crate::ledger::dumps_value(&serde_json::json!({ "proposals": [] }))), // pull_rules
        ]);
        let log = v.join("_log");
        let (code, lines) = run_lines_with(&v, &opts(&log), Some(&client));

        assert_eq!(code, 0, "a spent cap is a normal outcome: {lines:?}");
        assert!(
            lines.iter().any(|l| l ==
                "judge: the service answered the daily judgment cap, so the rest of the batch was not sent"),
            "the student's line must name the cap: {lines:?}"
        );
        let requests = server.requests();
        let asked: Vec<&String> = requests.iter().filter(|r| r.contains("/judge-task")).collect();
        assert_eq!(asked.len(), 1, "one /judge-task and no more: {requests:?}");

        let raw = raw_log_text(&log);
        assert!(raw.contains("\"cause\": \"capped\""), "the log must record the cap: {raw}");
        assert!(!raw.contains("model failed"), "a spent cap is not a model failure: {raw}");
        let _ = std::fs::remove_dir_all(&v);
    }

    /// C2 final review E-1: the cloud arm shares ONE budget across four passes, and before this
    /// fix two of them did not know that. `enrich_with` started its own clock, so it was handed a
    /// full `opts.budget` however much the probe had already spent; `pull_rules` took no budget at
    /// all and would open a connection after the slot's own `scheduler::CHILD_TIMEOUT` had run
    /// out. Here the probe alone costs more than the whole budget — the listener holds its first
    /// reply back — so the enrichment batch must take NO item and `pull_rules` must make NO
    /// request, and the run must still exit 0 naming what was left.
    #[test]
    fn a_probe_that_eats_the_budget_leaves_the_batch_and_the_rules_pull_for_the_next_slot() {
        let _guard = crate::journal::DEVICE_ENV_MUTEX.lock().unwrap();
        let v = vault("budget-arm");
        // One scripted reply: the probe's. A second request would be refused, and every line
        // asserted below would read differently — which is what makes "no request" provable
        // without waiting on an accept that never comes.
        let (base, handle) = multi_reply_loopback_slow(
            vec![(200, crate::ledger::dumps_value(&serde_json::json!({ "proposals": [] })))],
            std::time::Duration::from_millis(500),
        );
        let client = client_for(base);
        let log = v.join("_log");
        let o = Options { budget: std::time::Duration::from_millis(150), ..opts(&log) };
        let (code, lines) = run_lines_with(&v, &o, Some(&client));

        assert_eq!(code, 0, "{lines:?}");
        assert!(
            lines.iter().any(|l| l == "judge: 0 item(s), 0 enriched, 0 proposed, 1 left for the next slot"),
            "the batch must take no item once the probe has spent the budget: {lines:?}"
        );
        assert!(
            lines.iter().any(|l| l == "rules: the offer was left for the next slot"),
            "pull_rules must not open a connection with no budget left: {lines:?}"
        );
        let requests = handle.join().expect("the listener thread did not panic");
        assert_eq!(requests.len(), 1, "the probe and nothing else: {requests:?}");
        assert!(requests[0].starts_with("GET /functions/v1/judge-rules"), "{}", requests[0]);
        let _ = std::fs::remove_dir_all(&v);
    }

    // -----------------------------------------------------------------------------------------
    // C2 Task 12 — rule promotion: the loop that retires model calls.
    // -----------------------------------------------------------------------------------------

    #[test]
    fn a_rule_proposal_becomes_a_card_the_deck_can_answer() {
        let _guard = crate::journal::DEVICE_ENV_MUTEX.lock().unwrap();
        let vault = vault("rules-file");
        let offered = crate::ledger::dumps_value(&serde_json::json!({
            "proposals": [{
                "id": 41, "kind": "task", "feature": "created_by+title_prefix",
                "value": "zybooks|CS 100 Lab",
                "verdict": {"effort_hours": "0.5", "importance": "2"},
                "proposed_at": "2026-09-11"
            }]
        }));
        let (client, mut server) = loopback_client(vec![(200, offered)]);
        let log = vault.join("_log");
        let o = opts(&log);
        let lines = pull_rules(&vault, &client, &o, BATCH_BUDGET);
        assert!(lines.iter().any(|l| l.contains("rules 41: proposed")), "{lines:?}");
        let card = std::fs::read_to_string(vault.join("approvals").join("rule-41.md")).expect("the card exists");
        assert!(card.contains("kind: rule"));
        assert!(card.contains("rule_id: 41"));
        assert!(card.contains("status: pending"));
        assert!(card.contains("- effort_hours: 0.5"));
        let _ = server.requests();
        let _ = std::fs::remove_dir_all(&vault);
    }

    #[test]
    fn a_card_is_never_minted_twice_for_one_proposal_id() {
        let _guard = crate::journal::DEVICE_ENV_MUTEX.lock().unwrap();
        let vault = vault("rules-once");
        let offered = crate::ledger::dumps_value(&serde_json::json!({
            "proposals": [{ "id": 41, "kind": "task", "feature": "title_prefix", "value": "CS 100 Lab",
                            "verdict": {"importance": "2"}, "proposed_at": "2026-09-11" }]
        }));
        let (client, mut server) =
            loopback_client(vec![(200, offered.clone()), (200, offered)]);
        let log = vault.join("_log");
        let o = opts(&log);
        let first = pull_rules(&vault, &client, &o, BATCH_BUDGET);
        assert!(first.iter().any(|l| l.contains("1 proposed of 1")), "{first:?}");
        let second = pull_rules(&vault, &client, &o, BATCH_BUDGET);
        assert!(second.iter().any(|l| l.contains("0 proposed of 1")), "{second:?}");
        let _ = server.requests();
        let _ = std::fs::remove_dir_all(&vault);
    }

    #[test]
    fn an_approved_card_is_sent_once_and_then_archived() {
        let _guard = crate::journal::DEVICE_ENV_MUTEX.lock().unwrap();
        let vault = vault("rules-decide");
        let ctx = WriteContext { actor: RULES_ACTOR.into(), via: "local-runner".into(), run_id: None };
        let mut journal = Journal::new(&vault);
        let proposal = crate::cloudmodel::RuleProposal {
            id: 41, kind: "task".into(), feature: "title_prefix".into(), value: "CS 100 Lab".into(),
            verdict: serde_json::json!({"importance": "2"}), proposed_at: "2026-09-11".into(),
        };
        let stem = write_rule_card(&vault, &proposal, jiff::civil::date(2026, 9, 11), &ctx, &mut journal)
            .expect("the card writes");
        crate::write::write_literals(
            &vault, &format!("approvals/{stem}.md"),
            &[("status".to_string(), "approved".to_string())],
            &ctx, &mut journal, &WriteOpts::default(),
        )
        .expect("approve");

        let empty = crate::ledger::dumps_value(&serde_json::json!({ "proposals": [] }));
        // M7 (R-C2-E47 fix 1): the second pull's proposals reply offers id 41 AGAIN — as the
        // service legitimately might, since nothing on its side remembers a device has already
        // filed a card for a proposal — so "still seen in archive, not re-filed" below is a real
        // assertion about `existing_rule_ids` scanning `archive/`, not a vacuous one about an
        // empty reply.
        let offered_again = crate::ledger::dumps_value(&serde_json::json!({
            "proposals": [{ "id": 41, "kind": "task", "feature": "title_prefix", "value": "CS 100 Lab",
                            "verdict": {"importance": "2"}, "proposed_at": "2026-09-11" }]
        }));
        let (client, mut server) = loopback_client(vec![
            (200, crate::ledger::dumps_value(&serde_json::json!({ "decided": "approved" }))),
            (200, empty),
            (200, offered_again),
        ]);
        let log = vault.join("_log");
        let o = opts(&log);
        let lines = pull_rules(&vault, &client, &o, BATCH_BUDGET);
        assert!(lines.iter().any(|l| l.contains("rules 41: approved")), "{lines:?}");
        assert!(!vault.join("approvals").join(format!("{stem}.md")).exists());
        // A third pull re-sends no decision (the card is archived, not `approvals/`), and — even
        // though the service offers id 41 again — files no new card: `existing_rule_ids` sees
        // `rule_id: 41` in `archive/` and skips it.
        let again = pull_rules(&vault, &client, &o, BATCH_BUDGET);
        assert!(!again.iter().any(|l| l.contains("rules 41: approved") || l.contains("rules 41: proposed")), "{again:?}");
        assert!(again.iter().any(|l| l.contains("0 proposed of 1")), "{again:?}");
        assert!(!vault.join("approvals").join("rule-41.md").exists(), "must not be re-filed");
        let sent = server.requests();
        assert_eq!(sent.iter().filter(|r| r.contains("POST /functions/v1/judge-rules")).count(), 1);
        let _ = std::fs::remove_dir_all(&vault);
    }

    /// M8 (R-C2-E47 fix 1): the `rejected` branch of `decided_rule_cards`, undriven by any
    /// existing test — a card set `status: rejected` in the file must send `decision: rejected`
    /// and archive exactly as an approved one archives, no round trip needed for the vault side.
    #[test]
    fn a_rejected_card_sends_rejected_and_is_archived() {
        let _guard = crate::journal::DEVICE_ENV_MUTEX.lock().unwrap();
        let vault = vault("rules-reject");
        let ctx = WriteContext { actor: RULES_ACTOR.into(), via: "local-runner".into(), run_id: None };
        let mut journal = Journal::new(&vault);
        let proposal = crate::cloudmodel::RuleProposal {
            id: 41, kind: "task".into(), feature: "title_prefix".into(), value: "CS 100 Lab".into(),
            verdict: serde_json::json!({"importance": "2"}), proposed_at: "2026-09-11".into(),
        };
        let stem = write_rule_card(&vault, &proposal, jiff::civil::date(2026, 9, 11), &ctx, &mut journal)
            .expect("the card writes");
        crate::write::write_literals(
            &vault, &format!("approvals/{stem}.md"),
            &[("status".to_string(), "rejected".to_string())],
            &ctx, &mut journal, &WriteOpts::default(),
        )
        .expect("reject");

        let (client, mut server) = loopback_client(vec![
            (200, crate::ledger::dumps_value(&serde_json::json!({ "decided": "rejected" }))),
            (200, crate::ledger::dumps_value(&serde_json::json!({ "proposals": [] }))),
        ]);
        let log = vault.join("_log");
        let o = opts(&log);
        let lines = pull_rules(&vault, &client, &o, BATCH_BUDGET);
        assert!(lines.iter().any(|l| l.contains("rules 41: rejected")), "{lines:?}");
        assert!(!vault.join("approvals").join(format!("{stem}.md")).exists(), "the card must archive");

        let sent = server.requests();
        let post = sent.iter().find(|r| r.starts_with("POST /functions/v1/judge-rules")).expect("a POST was sent");
        assert!(post.contains(r#""decision": "rejected""#), "the POST body must carry the rejection: {post}");
        let _ = std::fs::remove_dir_all(&vault);
    }

    /// I4 (R-C2-E47 fix 1): a 404 from `decide_rule` means the service already settled this
    /// decision — most likely this same card, sent by a previous run that died between the POST
    /// and the stamp. Without this, the card would stay `approved` and re-POST (and re-404) every
    /// slot forever, and `rule_decisions_waiting` would keep the widened early return (R-C2-E46)
    /// from ever firing again for this vault.
    #[test]
    fn a_decision_the_service_already_settled_is_archived_not_retried() {
        let _guard = crate::journal::DEVICE_ENV_MUTEX.lock().unwrap();
        let vault = vault("rules-already-settled");
        let ctx = WriteContext { actor: RULES_ACTOR.into(), via: "local-runner".into(), run_id: None };
        let mut journal = Journal::new(&vault);
        let proposal = crate::cloudmodel::RuleProposal {
            id: 41, kind: "task".into(), feature: "title_prefix".into(), value: "CS 100 Lab".into(),
            verdict: serde_json::json!({"importance": "2"}), proposed_at: "2026-09-11".into(),
        };
        let stem = write_rule_card(&vault, &proposal, jiff::civil::date(2026, 9, 11), &ctx, &mut journal)
            .expect("the card writes");
        crate::write::write_literals(
            &vault, &format!("approvals/{stem}.md"),
            &[("status".to_string(), "approved".to_string())],
            &ctx, &mut journal, &WriteOpts::default(),
        )
        .expect("approve");

        let (client, mut server) = loopback_client(vec![
            (404, crate::ledger::dumps_value(&serde_json::json!({ "error": "no such undecided proposal" }))),
            (200, crate::ledger::dumps_value(&serde_json::json!({ "proposals": [] }))),
        ]);
        let log = vault.join("_log");
        let o = opts(&log);
        let lines = pull_rules(&vault, &client, &o, BATCH_BUDGET);
        assert!(lines.iter().any(|l| l.contains("rules 41: already settled")), "{lines:?}");
        assert!(!vault.join("approvals").join(format!("{stem}.md")).exists(), "a 404 must still archive the card");
        let _ = server.requests();
        let _ = std::fs::remove_dir_all(&vault);
    }

    /// R-C2-E49 fix 3: a 404 whose body is NOT the handler's own `no such undecided proposal` text
    /// (a gateway 404 — the function not deployed, a stale `api_base`, a slug typo) must NOT read
    /// as "already settled": that would silently discard a real, unsent decision. `{}` is the
    /// shape any of those would actually produce — none of them go through `judge-rules/handler.ts`
    /// at all, so there is no `error` field to extract.
    #[test]
    fn a_404_with_no_matching_body_is_not_read_as_already_settled() {
        let _guard = crate::journal::DEVICE_ENV_MUTEX.lock().unwrap();
        let vault = vault("rules-404-generic");
        let ctx = WriteContext { actor: RULES_ACTOR.into(), via: "local-runner".into(), run_id: None };
        let mut journal = Journal::new(&vault);
        let proposal = crate::cloudmodel::RuleProposal {
            id: 41, kind: "task".into(), feature: "title_prefix".into(), value: "CS 100 Lab".into(),
            verdict: serde_json::json!({"importance": "2"}), proposed_at: "2026-09-11".into(),
        };
        let stem = write_rule_card(&vault, &proposal, jiff::civil::date(2026, 9, 11), &ctx, &mut journal)
            .expect("the card writes");
        crate::write::write_literals(
            &vault, &format!("approvals/{stem}.md"),
            &[("status".to_string(), "approved".to_string())],
            &ctx, &mut journal, &WriteOpts::default(),
        )
        .expect("approve");

        let (client, mut server) = loopback_client(vec![
            (404, "{}".to_string()),
            (200, crate::ledger::dumps_value(&serde_json::json!({ "proposals": [] }))),
        ]);
        let log = vault.join("_log");
        let o = opts(&log);
        let lines = pull_rules(&vault, &client, &o, BATCH_BUDGET);
        assert!(!lines.iter().any(|l| l.contains("already settled")), "{lines:?}");
        assert!(lines.iter().any(|l| l.contains("rules 41: not sent") && l.contains("HTTP 404")), "{lines:?}");
        assert!(vault.join("approvals").join(format!("{stem}.md")).exists(), "an unmatched 404 must not archive the card");
        let _ = server.requests();
        let _ = std::fs::remove_dir_all(&vault);
    }

    #[test]
    fn a_decision_that_cannot_be_sent_keeps_its_card() {
        let _guard = crate::journal::DEVICE_ENV_MUTEX.lock().unwrap();
        let vault = vault("rules-retry");
        let ctx = WriteContext { actor: RULES_ACTOR.into(), via: "local-runner".into(), run_id: None };
        let mut journal = Journal::new(&vault);
        let proposal = crate::cloudmodel::RuleProposal {
            id: 41, kind: "task".into(), feature: "title_prefix".into(), value: "CS 100 Lab".into(),
            verdict: serde_json::json!({"importance": "2"}), proposed_at: "2026-09-11".into(),
        };
        let stem = write_rule_card(&vault, &proposal, jiff::civil::date(2026, 9, 11), &ctx, &mut journal)
            .expect("the card writes");
        crate::write::write_literals(
            &vault, &format!("approvals/{stem}.md"),
            &[("status".to_string(), "approved".to_string())],
            &ctx, &mut journal, &WriteOpts::default(),
        )
        .expect("approve");
        let (client, mut server) = loopback_client(vec![
            (503, r#"{"error":"upstream"}"#.to_string()),
            (200, crate::ledger::dumps_value(&serde_json::json!({ "proposals": [] }))),
        ]);
        let log = vault.join("_log");
        let o = opts(&log);
        let lines = pull_rules(&vault, &client, &o, BATCH_BUDGET);
        assert!(lines.iter().any(|l| l.contains("not sent")), "{lines:?}");
        assert!(vault.join("approvals").join(format!("{stem}.md")).exists(), "the card must survive to retry");
        let _ = server.requests();
        let _ = std::fs::remove_dir_all(&vault);
    }

    /// R-C2-E46: an answered `kind: rule` card is exactly what widens the four-way early return —
    /// an empty enrichment queue, no event source and no linked Google calendar are not "nothing
    /// to do" while a rule decision is still waiting to be sent. The companion half (no answered
    /// card, everything else empty) proves the early return still fires exactly as before this
    /// task: no request of any kind, not even the probe.
    #[test]
    fn an_answered_rule_card_alone_makes_the_probe_fire_and_a_bare_vault_makes_none() {
        let _guard = crate::journal::DEVICE_ENV_MUTEX.lock().unwrap();

        let waiting = vault("rules-waiting");
        crate::pystr::write_text(
            &waiting.join("tasks").join("hw3.md"),
            &NOTE.replace("needs_enrichment: true", "needs_enrichment: false"),
        ).unwrap();
        let ctx = WriteContext { actor: RULES_ACTOR.into(), via: "local-runner".into(), run_id: None };
        let mut journal = Journal::new(&waiting);
        let proposal = crate::cloudmodel::RuleProposal {
            id: 41, kind: "task".into(), feature: "title_prefix".into(), value: "CS 100 Lab".into(),
            verdict: serde_json::json!({"importance": "2"}), proposed_at: "2026-09-11".into(),
        };
        let stem = write_rule_card(&waiting, &proposal, jiff::civil::date(2026, 9, 11), &ctx, &mut journal)
            .expect("the card writes");
        crate::write::write_literals(
            &waiting, &format!("approvals/{stem}.md"),
            &[("status".to_string(), "approved".to_string())],
            &ctx, &mut journal, &WriteOpts::default(),
        )
        .expect("approve");

        let (client, mut server) = loopback_client(vec![
            (200, "{}".to_string()),
            (200, crate::ledger::dumps_value(&serde_json::json!({ "decided": "approved" }))),
            (200, crate::ledger::dumps_value(&serde_json::json!({ "proposals": [] }))),
        ]);
        let log = waiting.join("_log");
        let (code, _lines) = run_lines_with(&waiting, &opts(&log), Some(&client));
        assert_eq!(code, 0);
        let requests = server.requests();
        assert!(
            requests.iter().any(|r| r.starts_with("POST /functions/v1/judge-rules")),
            "the answered card's decision must reach the service: {requests:?}"
        );
        let _ = std::fs::remove_dir_all(&waiting);

        let bare = vault("rules-bare");
        crate::pystr::write_text(
            &bare.join("tasks").join("hw3.md"),
            &NOTE.replace("needs_enrichment: true", "needs_enrichment: false"),
        ).unwrap();
        let listener = std::net::TcpListener::bind("127.0.0.1:0").expect("bind");
        listener.set_nonblocking(true).expect("nonblocking");
        let port = listener.local_addr().expect("addr").port();
        let cfg = crate::cloudmodel::CloudConfig {
            api_base: format!("http://127.0.0.1:{port}/functions/v1"),
            anon_key: "anon".into(),
            session_credential_target: "knowlu/test/session".into(),
            account_id: "acct-1".into(),
        };
        let client2 = crate::cloudmodel::CloudClient::new(&cfg, "jwt-not-a-secret");
        let log2 = bare.join("_log");
        let (code2, lines2) = run_lines_with(&bare, &opts(&log2), Some(&client2));
        assert_eq!(code2, 0);
        assert_eq!(lines2, vec!["judge: nothing to enrich".to_string()], "{lines2:?}");
        assert!(
            listener.accept().is_err(),
            "no rule decision waiting, and nothing else pending, must never reach the service"
        );
        let _ = std::fs::remove_dir_all(&bare);
    }

    // -------------------------------------------------------------------------------------
    // Stream J Task T9 — `completion`: an email confirming the student already submitted a piece
    // of work becomes a `status: done` proposal through `completion::propose_done`, when exactly
    // one active task carries that work's title. Every fixture is fabricated.
    // -------------------------------------------------------------------------------------

    fn completion_item(uid: &str, title: &str) -> String {
        serde_json::json!({
            "uid": uid, "tier": "completion",
            "payload": {"title": title, "course": null, "due": null, "effort_hours": null,
                        "importance": null, "why": "Blackboard submission receipt", "confidence": 1},
        })
        .to_string()
    }

    fn blackboard_task(v: &Path, stem: &str, yaml_title: &str, status: &str, id: &str) {
        crate::pystr::write_text(
            &v.join("tasks").join(format!("{stem}.md")),
            &format!(
                "---\ntitle: {yaml_title}\ncourse: null\ndomain: school\ndue: 2026-09-03T23:59\n\
                 status: {status}\nprogress: 0\ncreated_by: blackboard\nsource_uid: \"blackboard:{stem}\"\n\
                 id: {id}\n---\n\nFrom the LMS.\n"
            ),
        )
        .unwrap();
    }

    fn approvals_in(v: &Path) -> Vec<String> {
        let mut out: Vec<String> = std::fs::read_dir(v.join("approvals"))
            .map(|d| d.flatten().map(|e| e.file_name().to_string_lossy().to_string()).collect())
            .unwrap_or_default();
        out.sort();
        out
    }

    #[test]
    fn completion_titles_match_after_normalisation() {
        assert_eq!(completion_title_key("  CS-100   Homework 3 "), completion_title_key("cs-100 homework 3"));
        assert_eq!(completion_title_key("Office Space \"Quiz\""), completion_title_key("Office Space 'Quiz'"));
        assert_eq!(
            completion_title_key("Office Space \u{201c}Quiz\u{201d}"),
            completion_title_key("office space \u{2018}quiz\u{2019}")
        );
        assert_eq!(completion_title_key("Q&amp;A: Unit 2"), completion_title_key("Q&A: Unit 2"));
        assert_ne!(completion_title_key("Lab 3: Pendulum"), completion_title_key("Lab 4: Pendulum"));
        assert_eq!(completion_title_key("   "), "");
    }

    /// The "done when": a submission receipt reaches a proposal. The note itself is untouched,
    /// the card is `completion::propose_done`'s own, and the evidence names no address, no
    /// confirmation number and no body.
    #[test]
    fn a_submission_receipt_for_one_active_task_files_one_done_proposal() {
        let _guard = crate::journal::DEVICE_ENV_MUTEX.lock().unwrap();
        let v = vault("gmail-completion");
        let (base, handle) = gmail_loopback(vec![
            gmail_reply(&format!("[{}]", completion_item("gmail:r1", "cs-100  homework 3")), false),
            gmail_reply("[]", false),
        ]);
        let lines = pull_gmail(&v, &client_for(base), &opts(&v.join("_log")), BATCH_BUDGET);

        assert_eq!(approvals_in(&v), vec!["amend-hw3-done.md".to_string()], "{lines:?}");
        let card = meta_of_approval(&v, "amend-hw3-done.md");
        assert_eq!(crate::yaml::opt_text(crate::yaml::get(&card, "kind")).as_deref(), Some("amend"));
        assert_eq!(crate::yaml::opt_text(crate::yaml::get(&card, "target")).as_deref(), Some("tasks/hw3.md"));
        assert_eq!(
            crate::yaml::opt_text(crate::yaml::get(&card, "created_by")).as_deref(),
            Some(crate::completion::ACTOR)
        );
        assert_eq!(
            crate::yaml::opt_text(crate::yaml::get(&meta_of(&v, "hw3.md"), "status")).as_deref(),
            Some("active"),
            "a proposal never touches the note"
        );
        let text = crate::pystr::read_text(&v.join("approvals").join("amend-hw3-done.md")).unwrap();
        assert!(text.contains("An email confirms this was submitted"), "{text}");
        let evidence: Vec<serde_json::Value> =
            records(&v).into_iter().filter_map(|r| r.get("evidence").cloned()).collect();
        assert_eq!(evidence, vec![serde_json::json!({"source": "email", "uid": "gmail:r1"})]);
        assert!(!text.contains('@'), "no email address on the card: {text}");

        assert!(crate::ingest::load_seen(&v).contains("gmail:r1"));
        assert!(lines.iter().any(|l| l == "gmail gmail:r1: completion (proposed done hw3)"), "{lines:?}");
        assert_eq!(lines.last().unwrap(), "gmail: 0 task(s), 1 proposed, 0 dropped as information", "{lines:?}");
        let requests = handle.join().expect("the listener thread did not panic");
        assert!(requests[1].contains("gmail:r1"), "the receipt must be acknowledged: {}", requests[1]);
        let _ = std::fs::remove_dir_all(&v);
    }

    /// The addendum's quoted title: stored YAML-escaped in the note, and one-lined by the service
    /// (`oneLine` turns `"` into `'`) — the two still match.
    #[test]
    fn a_quoted_title_matches_across_yaml_escaping_and_quote_folding() {
        let _guard = crate::journal::DEVICE_ENV_MUTEX.lock().unwrap();
        let v = vault("gmail-completion-quotes");
        blackboard_task(&v, "office-space-quiz", "\"Office Space \\\"Quiz\\\"\"", "active", "task_2222222222");
        let (base, handle) = gmail_loopback(vec![
            gmail_reply(&format!("[{}]", completion_item("gmail:r2", "Office Space 'Quiz'")), false),
            gmail_reply("[]", false),
        ]);
        let lines = pull_gmail(&v, &client_for(base), &opts(&v.join("_log")), BATCH_BUDGET);
        assert_eq!(approvals_in(&v), vec!["amend-office-space-quiz-done.md".to_string()], "{lines:?}");
        handle.join().unwrap();
        let _ = std::fs::remove_dir_all(&v);
    }

    /// Zero matches, several matches, or a match that is no longer active: nothing is written,
    /// and the uid is recorded exactly as `information` records it — never asked about again.
    #[test]
    fn a_receipt_with_no_single_active_match_writes_nothing_and_is_recorded_like_information() {
        let _guard = crate::journal::DEVICE_ENV_MUTEX.lock().unwrap();
        let v = vault("gmail-completion-nomatch");
        blackboard_task(&v, "quiz-1", "Quiz 1", "active", "task_3333333331");
        blackboard_task(&v, "quiz-1-b", "QUIZ 1", "active", "task_3333333332");
        blackboard_task(&v, "essay-2", "Essay 2", "done", "task_3333333333");
        let items = format!(
            "[{},{},{}]",
            completion_item("gmail:n1", "Lab 9: Nothing like it"),
            completion_item("gmail:n2", "Quiz 1"),
            completion_item("gmail:n3", "Essay 2"),
        );
        let (base, handle) = gmail_loopback(vec![gmail_reply(&items, false), gmail_reply("[]", false)]);
        let lines = pull_gmail(&v, &client_for(base), &opts(&v.join("_log")), BATCH_BUDGET);

        assert!(approvals_in(&v).is_empty(), "no card for zero, several or inactive matches: {lines:?}");
        let seen = crate::ingest::load_seen(&v);
        for uid in ["gmail:n1", "gmail:n2", "gmail:n3"] {
            assert!(seen.contains(uid), "{uid} must be recorded");
        }
        let ledger = crate::pystr::read_text(&v.join("state").join("ingest-seen.md")).unwrap();
        assert!(ledger.contains("gmail:n1 \u{b7} (email) \u{b7}"), "{ledger}");
        assert!(!ledger.contains("Nothing like it"), "{ledger}");
        assert_eq!(lines.last().unwrap(), "gmail: 0 task(s), 0 proposed, 3 dropped as information", "{lines:?}");
        let requests = handle.join().unwrap();
        for uid in ["gmail:n1", "gmail:n2", "gmail:n3"] {
            assert!(requests[1].contains(uid), "{uid} must be acknowledged: {}", requests[1]);
        }
        let _ = std::fs::remove_dir_all(&v);
    }

    /// A resubmission sends a second receipt for the same work. `propose_done`'s one-proposal-ever
    /// rule answers it: one card, both uids recorded and acknowledged.
    #[test]
    fn a_resubmission_receipt_files_no_second_card() {
        let _guard = crate::journal::DEVICE_ENV_MUTEX.lock().unwrap();
        let v = vault("gmail-completion-twice");
        let items = format!(
            "[{},{}]",
            completion_item("gmail:s1", "CS-100 Homework 3"),
            completion_item("gmail:s2", "CS-100 Homework 3"),
        );
        let (base, handle) = gmail_loopback(vec![gmail_reply(&items, false), gmail_reply("[]", false)]);
        let lines = pull_gmail(&v, &client_for(base), &opts(&v.join("_log")), BATCH_BUDGET);
        assert_eq!(approvals_in(&v), vec!["amend-hw3-done.md".to_string()], "{lines:?}");
        assert_eq!(lines.last().unwrap(), "gmail: 0 task(s), 1 proposed, 1 dropped as information", "{lines:?}");
        let requests = handle.join().unwrap();
        assert!(requests[1].contains("gmail:s1") && requests[1].contains("gmail:s2"), "{}", requests[1]);
        let _ = std::fs::remove_dir_all(&v);
    }

    /// T9: the service hands `completion` only to a device that declares it (`gmail-read`'s
    /// `forDevice`); every other device gets `information`. So every `/gmail-read` request this
    /// engine sends, the first pull and the ack flush alike, must declare it.
    #[test]
    fn every_gmail_read_request_declares_it_accepts_completion() {
        let _guard = crate::journal::DEVICE_ENV_MUTEX.lock().unwrap();
        let v = vault("gmail-accepts");
        let (base, handle) = gmail_loopback(vec![
            gmail_reply(&format!("[{}]", completion_item("gmail:a1", "CS-100 Homework 3")), false),
            gmail_reply("[]", false),
        ]);
        let _ = pull_gmail(&v, &client_for(base), &opts(&v.join("_log")), BATCH_BUDGET);
        let requests = handle.join().unwrap();
        assert_eq!(requests.len(), 2, "{requests:?}");
        for request in &requests {
            let body = request.split("\r\n\r\n").nth(1).unwrap_or("");
            let body: serde_json::Value = serde_json::from_str(body.trim()).expect("a JSON body");
            assert_eq!(body["accepts"], serde_json::json!(["completion"]), "{request}");
        }
        let _ = std::fs::remove_dir_all(&v);
    }

    /// Forward compatibility: a tier this engine does not know is dropped and recorded like
    /// `information`, never filed as a `kind: task` card — the catch-all that would have turned a
    /// receipt for finished work into a proposal to ADD that work.
    #[test]
    fn an_unknown_tier_is_dropped_like_information_not_filed_as_a_card() {
        let _guard = crate::journal::DEVICE_ENV_MUTEX.lock().unwrap();
        let v = vault("gmail-unknown-tier");
        let (base, handle) = gmail_loopback(vec![
            gmail_reply(
                r#"[{"uid":"gmail:u1","tier":"some-future-tier","payload":{"title":"Something new","course":null,"due":null,"effort_hours":null,"importance":null,"why":"a new kind","confidence":0.9}}]"#,
                false,
            ),
            gmail_reply("[]", false),
        ]);
        let lines = pull_gmail(&v, &client_for(base), &opts(&v.join("_log")), BATCH_BUDGET);
        assert!(approvals_in(&v).is_empty(), "{lines:?}");
        assert!(crate::ingest::load_seen(&v).contains("gmail:u1"));
        let ledger = crate::pystr::read_text(&v.join("state").join("ingest-seen.md")).unwrap();
        assert!(!ledger.contains("Something new"), "{ledger}");
        assert_eq!(lines.last().unwrap(), "gmail: 0 task(s), 0 proposed, 1 dropped as information", "{lines:?}");
        handle.join().unwrap();
        let _ = std::fs::remove_dir_all(&v);
    }
}
