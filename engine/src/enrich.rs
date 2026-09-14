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
    // R-C2-E15: the probe is a network round trip, and it must not fire when there is nothing to
    // judge — reusing `pending`'s own predicate here (not a second scanning routine) is what lets
    // a job that runs twice a day forever skip both the call and, on a network that black-holes
    // rather than refuses, the up-to-`CALL_TIMEOUT` stall a probe with no queue behind it would
    // otherwise risk. `enrich_with` below re-derives the same list; that second read is the
    // accepted cost of leaving `enrich_with`'s own signature — and every test that calls it
    // directly — untouched.
    if pending(vault).0.is_empty() {
        return enrich_with(vault, opts, Ok(&model));
    }
    // A session or entitlement problem answers every item identically, so the FIRST call decides
    // the batch. Asked once before the loop, the whole run then reports one honest outcome
    // (`Missing::Service`, which becomes `Outcome::ServiceUnavailable` and logs as
    // `service unavailable`) instead of fifty `model failed` lines — ruling R-3a-20's point, at
    // the boundary a cloud judge adds. Every other failure (a 429, a 5xx, a dropped connection)
    // stays per-item, because the next item genuinely may succeed.
    match model.probe() {
        Some(reason) => {
            // Exactly one line: `probe()` already set `model.fatal()` to this same reason, so the
            // `if let Some(reason) = model.fatal()` line below — which exists for a batch that
            // turned fatal partway through — must not also fire here, or the run would print two
            // contradictory summaries ("nothing was sent" and "the rest ... was not sent") for the
            // one call that was actually made.
            let (code, mut lines) = enrich_with(vault, opts, Err(judge::Missing::Service(reason)));
            lines.insert(0, format!("judge: the service answered {reason}; nothing was sent"));
            (code, lines)
        }
        None => {
            let (code, mut lines) = enrich_with(vault, opts, Ok(&model));
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
}
