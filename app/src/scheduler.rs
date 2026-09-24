//! The app's scheduler runtime (Knowlu plan 1, Task 12): a tick thread that fires the local
//! runner's slots as child processes when `config/runners.yaml` says `scheduler: app` for the
//! `local` entry AND this device is the one named there, plus a housekeeping thread for
//! debounced backup and the tray's health colour. Inert (never ticks, never runs a slot) on
//! `scheduler: script` — which is every vault today, since the live vault carries no such key.
//!
//! The app never builds anything and never writes to the vault itself: each slot step runs the
//! sibling `knowlu-engine.exe` as a child process with exactly the argv the retired
//! `scripts/local-run.ps1` used, and that engine binary's own `runs` module records the run. The
//! slot's own first step is `sync` (C3′): the engine's `sync` subcommand pulls another desktop's
//! writes down and pushes this device's own up to the account. `state::run_sync` runs the same
//! engine code in-process for the console's own *Sync now* button.
use std::collections::HashMap;
use std::io::Read;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Mutex, MutexGuard};
use tauri::{AppHandle, Manager};
use knowlu_engine::runs::RunnerConfig;
use knowlu_engine::schedule::{self, SchedulerMode};
use crate::state::{self, ConsoleState};
use crate::tray::{self, TrayState};

/// A hung child, unreachable vendor login, or any other stall inside a step: after this long
/// `run_child` kills the child rather than leaving the scheduler wedged forever (Task 12 review
/// item 2).
const CHILD_TIMEOUT: std::time::Duration = std::time::Duration::from_secs(20 * 60);

/// Slot log FILES kept in `log_dir()` — files, not runs. A slot writes one per step, and since
/// Knowlu plan 3a a vault with an LMS feed and a model installed has four steps (coursework,
/// ingest, judge, rank), so two slots a day is eight files a day: eighty is about twenty runs, or
/// ten days. Long enough that a bug report can quote the run that went wrong, short enough that the
/// directory stays bounded. `quit-*` records are neither counted nor pruned (see `prune_logs`).
const LOGS_KEPT: usize = 80;

/// Every mutex in this module is taken through here, never `.lock().unwrap()` (plan 1 final fix
/// wave B4, widened by plan 2 Task 5 to the whole file). A panic anywhere under a slot poisons
/// whatever it was holding, and an `unwrap()` on the next tick turns one bad slot into a scheduler
/// that stays dead until the app is restarted — with a tray that still says "scheduler on". None of
/// the guarded values carries an invariant a panic could break: a bool, a summary, a cached
/// `(mode, device_ok)` pair, a retry counter. Recovering the inner value is strictly better than
/// propagating the panic.
/// Crate-wide since the final fix wave (C1): `commands::attach_scheduler` reads two of these
/// same mutexes on every poll and had its own `.map_err(|_| "lock")?`, which every caller then
/// discarded with `let _ =` — one poisoned mutex silently dropped `last_slot` and `scheduler`
/// from every envelope thereafter. One helper, one policy.
/// `pub`, not `pub(crate)`, since plan 4a Task 8: `tests/updates.rs` reads `Scheduler.running`
/// (which is already `pub`) to prove an install holds and releases it, and an integration test is
/// a separate crate — `pub(crate)` would have forced exactly the `.lock().unwrap()` this helper
/// exists to replace into the one test whose subject is a flag taken across a fallible operation.
pub fn lock<T>(m: &Mutex<T>) -> MutexGuard<'_, T> { m.lock().unwrap_or_else(|e| e.into_inner()) }

/// Managed alongside `ConsoleState` (`app.manage(Scheduler::default())`), not folded into it —
/// `ConsoleState` is exercised directly by `tests/commands.rs` without a scheduler in play.
pub struct Scheduler {
    pub paused: AtomicBool,
    pub running: Mutex<bool>,
    pub last: Mutex<Option<RunSummary>>,
    /// The tray's "Pause scheduling"/"Resume scheduling" item, stashed here by `tray::build` so
    /// the menu handler can relabel it in place instead of rebuilding the whole menu.
    pub pause_item: Mutex<Option<tauri::menu::MenuItem<tauri::Wry>>>,
    /// `(mode, device_ok)`, cached so the ten `#[tauri::command]` wrappers that call
    /// `commands::attach_scheduler` on every poll don't each re-read and re-parse
    /// `config/runners.yaml`. Set once at `spawn` and refreshed by the housekeeping thread's 60 s
    /// pass (review item 5) — the tick thread's own gating always re-reads fresh, since that
    /// decision is the correctness-critical one.
    pub mode_device: Mutex<(SchedulerMode, bool)>,
    /// Retry state for slots whose engine steps failed: how many attempts have been made, and when
    /// the last one ended. The tick consults it through `should_retry` before firing, clears the
    /// key the moment a slot succeeds, and drops everything not from today on the way past.
    /// Deliberately not persisted — a relaunched app retries at once, which is what someone who
    /// just restarted it after a failure expects.
    ///
    /// **Keyed by the due slot's LOCAL RFC-3339 spelling** (`SlotDue.due.to_string()`, e.g.
    /// `2026-11-01T18:00:00-06:00[America/Chicago]`), not by its UTC timestamp: the first ten
    /// characters have to be the slot's own local date for the daily sweep to recognise it, and an
    /// 18:00 America/Chicago slot is 00:00 the NEXT day in UTC — it would be swept the instant it
    /// was written, which is exactly the retry storm the ladder exists to stop.
    pub attempts: Mutex<HashMap<String, (u32, std::time::Instant)>>,
}

impl Default for Scheduler {
    fn default() -> Self {
        Scheduler {
            paused: AtomicBool::new(false),
            running: Mutex::new(false),
            last: Mutex::new(None),
            pause_item: Mutex::new(None),
            mode_device: Mutex::new((SchedulerMode::Script, true)),
            attempts: Mutex::new(HashMap::new()),
        }
    }
}

/// Clears `Scheduler.running` on every exit from `run_slot_inner` — including an early return or
/// a panic unwind — so a slot that dies partway through never wedges every later trigger onto the
/// stale "already running" branch forever (Task 12 review item 2). No-op under the release
/// profile's `panic = "abort"` (nothing runs after an abort), but correct for `cargo test`'s
/// default unwinding profile and for any future profile change.
struct RunGuard<'a>(&'a Scheduler);
impl Drop for RunGuard<'_> {
    fn drop(&mut self) {
        // Poison-tolerant (final fix wave, B4). This runs on the unwind path of a panicking slot —
        // exactly the moment the mutex is likely to be poisoned — and an `unwrap()` here would
        // panic during a panic, aborting the process instead of clearing the flag. Clearing
        // `running` is the whole point of the guard: leave it set and every later trigger takes
        // the stale "already running" branch forever.
        *lock(&self.0.running) = false;
    }
}

#[derive(Debug, Clone, serde::Serialize)]
pub struct RunSummary {
    pub started: String,
    pub ended: String,
    pub steps: Vec<(String, i32)>,
    /// Every step succeeded, sync and backup included.
    pub ok: bool,
    /// The two engine steps (coursework, rank) both exited 0 — the signal the tray's warn colour
    /// and the housekeeping thread's "last slot failed" check use, so a transient sync collision
    /// or an offline git fetch never paints the tray as if the actual work failed (review item 4).
    pub engine_ok: bool,
    pub late: bool,
    /// `Some` only when the slot was refused outright (review item 7) — the engine was never
    /// invoked, `steps` is a single sentinel entry, and no run record was written.
    pub reason: Option<String>,
    /// Which attempt at this slot this run was: 1 for a first try or a manual "Run now", higher
    /// once the tick has retried a failed slot under the backoff. `0` when the engine never ran at
    /// all (a refusal, or the placeholder returned while another slot is in flight). The page's
    /// sync line appends `(attempt N)` above 1, so the same slot failing twice reads as one slot
    /// being retried rather than two unrelated failures.
    pub attempts: u32,
}

impl RunSummary {
    /// True only when the engine actually ran and failed. A refusal (`reason: Some`) never ran
    /// the engine, so it must not paint the tray Warn until the next successful slot — on a
    /// `scheduler: script` vault that slot never comes (final re-review, minor 1).
    pub fn engine_failed(&self) -> bool { !self.engine_ok && self.reason.is_none() }
}

/// `KNOWLU_ENGINE_EXE` if set (and non-empty), else a `knowlu-engine.exe` sitting beside the running
/// exe, else `Err` naming both places so a friend's bug report says exactly what to check.
pub fn engine_exe() -> Result<PathBuf, String> {
    if let Ok(p) = std::env::var("KNOWLU_ENGINE_EXE") {
        if !p.is_empty() {
            return Ok(PathBuf::from(p));
        }
    }
    let sibling = std::env::current_exe().ok().and_then(|e| e.parent().map(|d| d.join("knowlu-engine.exe")));
    match sibling {
        Some(p) if p.is_file() => Ok(p),
        Some(p) => Err(format!("engine not found: {} (or set KNOWLU_ENGINE_EXE)", p.display())),
        None => Err("engine not found: no sibling knowlu-engine.exe and KNOWLU_ENGINE_EXE unset".into()),
    }
}

/// Why a slot will or will not run `ingest` (R-P4a-17). The two "no" cases are different problems
/// and get different words on the Runs view: a config that does not parse is something to fix, a
/// config with no feed is a friend who has not connected one.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum IcsState { Feed, NoUrl, Unreadable }

/// The engine's `ingest` exits **1** on an empty `ics_url` with no account to fall back to (Python
/// did, and the port keeps it), and a slot step that exits non-zero sets `engine_ok = false` —
/// which would put every friend with no LMS feed into permanent retry backoff and an amber tray,
/// twice a day, forever. So a vault with NO account and no `ics_url` has the APP decide not to run
/// the step at all (ruling R-P4a-9).
///
/// **A cloud vault is different (C2 final review A-1/A-2, restating the stale rule this doc
/// carried before): the feed lives in the account, not the vault, so this app runs `ingest`
/// regardless of what `ics_state` says.** The engine itself names a missing `lms_ics` source as a
/// skip — `/ingest-ics` answering 404 is exit 0, never exit 1 — so there is no longer a failure
/// mode here for this app to protect a cloud vault from; `ingest_included` (below `has_ics_url`) is
/// the one decision point both `slot_argv` and `run_slot_inner`'s own skip line agree on.
///
/// A **missing** file is `NoUrl` (a vault without one has no feed, which is not a fault); a file
/// that does not parse is `Unreadable`. Both states are still meaningful for a vault with NO
/// account — `ingest_included` is what decides whether they ever reach the Runs view at all.
pub fn ics_state(vault: &Path) -> IcsState {
    let path = vault.join("config").join("ingest.yaml");
    let Ok(text) = std::fs::read_to_string(&path) else { return IcsState::NoUrl };
    let Ok(v) = serde_yaml_ng::from_str::<serde_yaml_ng::Value>(&text) else { return IcsState::Unreadable };
    match v.get("ics_url").and_then(|u| u.as_str()) {
        Some(s) if !s.trim().is_empty() => IcsState::Feed,
        _ => IcsState::NoUrl,
    }
}

pub fn has_ics_url(vault: &Path) -> bool { ics_state(vault) == IcsState::Feed }

/// Whether this slot includes `ingest` at all — the ONE decision point `slot_argv` (what actually
/// runs) and `run_slot_inner` (what the skip line, if any, says) both have to agree with (A-2 fix).
///
/// Before this, the two answered related but DIFFERENT questions, each on its own: `slot_argv`
/// checked exactly the condition below, while `run_slot_inner`'s skip line was pushed on
/// `ics_state` alone, with no `cloud.yaml` check at all. A cloud vault with no `ics_url` therefore
/// got BOTH a fake `ingest (skipped: no ics_url)` step AND the real `ingest` child process's own
/// step in the same `RunSummary.steps` — two `ingest` entries, one of them describing a step that
/// never actually ran.
pub fn ingest_included(vault: &Path) -> bool {
    has_ics_url(vault) || vault.join("config").join("cloud.yaml").is_file()
}

/// A vault that has never been ranked is owed its first slot at launch, whatever the clock says
/// (cloud design §4.2 step 7 — "the first slot runs immediately"; Quinn's cut-day note,
/// 2026-09-09: an empty first page until 12:00 is not a first session). `state/today.md` — the
/// file `rank` actually writes (`cli.rs`), never a root `today.md` — is rewritten by every `rank`,
/// so its absence is the whole test; once it exists, launch owes nothing until the next scheduled
/// slot. The first draft checked the vault root, which no engine step writes, and would have run a
/// slot at every launch; the C2 plan review caught it (R-C2-10).
pub fn needs_first_run(vault: &Path) -> bool { !vault.join("state").join("today.md").exists() }

/// Everything `knowlu-engine judge` needs that only the app knows: where the runtime was installed,
/// which model file was chosen, and this profile's own judgments directory.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct JudgeArgs {
    pub runtime: PathBuf,
    pub model: PathBuf,
    pub log_dir: PathBuf,
}

/// Why a slot will or will not run `judge` — the same three-state shape `IcsState` uses, and for
/// the same reason: the two "no" cases are different problems and get different words on the Runs
/// view. A friend who has installed neither is told about the runtime, because it is the
/// prerequisite; one who has the runtime and no model is told about the model.
///
/// **Neither is a fault.** Spec §5.3: "the app runs with no model present." The step is recorded
/// with exit code 0 and an explanatory name, exactly as `ingest (skipped: no ics_url)` is.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum JudgeState {
    Ready(JudgeArgs),
    NoRuntime,
    NoModel,
}

/// Resolve the judge step under an explicit app-data root — the seam the tests use, mirroring
/// `state::app_data_root_in`.
///
/// **Not optional** (review S3): without it the test below reaches the developer's *real*
/// `%LOCALAPPDATA%\knowlu`, so it would pass today and start failing the first time Task 10's
/// settings row installed a runtime on this machine — a test that breaks when the feature starts
/// working is worse than no test.
pub fn judge_state_in(root: &Path, cs: &ConsoleState) -> JudgeState {
    let Some(runtime) = crate::inference::runtime_exe(root) else { return JudgeState::NoRuntime };
    let Some(model) = crate::inference::model_file(root) else { return JudgeState::NoModel };
    JudgeState::Ready(JudgeArgs { runtime, model, log_dir: crate::inference::judgments_dir(&cs.data_dir) })
}

/// Resolve the judge step for this profile. Reads the install-wide root for the runtime and the
/// model and the profile's own `data_dir` for the log directory (spec §5.4/§5.6: the log is per
/// profile and never in the vault). A root that cannot be resolved at all is `NoRuntime`: there is
/// nowhere for one to be.
pub fn judge_state(cs: &ConsoleState) -> JudgeState {
    let Ok(root) = crate::commands::inference_root() else { return JudgeState::NoRuntime };
    judge_state_in(&root, cs)
}

/// Is this install allowed to run a cloud step? Reads the vault's own `config/cloud.yaml` and the
/// profile's cached entitlement, and decides with `account::decide` — which owns the 72-hour grace so
/// that a later cloud step (C2's `/judge/*`, `/ingest/*`, `/events`) asks the same question in one
/// place rather than four.
///
/// **A missing file and an unreadable one are different states** (fix round 1, item 2): `decide`'s
/// own `cloud_configured` flag cannot tell them apart, so the file's existence is checked here,
/// first — a vault with no `config/cloud.yaml` at all is `NoAccount` (falls through to the local
/// runtime/model gate, unchanged); a file that IS there but does not parse, or is missing a required
/// key, is `Unreadable` and never reaches `decide` at all.
pub fn entitlement_state(cs: &ConsoleState) -> crate::account::EntitlementState {
    if !cs.vault.join("config").join("cloud.yaml").exists() {
        return crate::account::EntitlementState::NoAccount;
    }
    match crate::account::cloud_config(&cs.vault) {
        Ok(_) => crate::account::decide(true, crate::account::load_cache(&cs.data_dir).as_ref(), jiff::Timestamp::now()),
        Err(_) => crate::account::EntitlementState::Unreadable,
    }
}

/// How this slot runs `judge` — or does not. **Three states, and the order between them matters**
/// (agreed with C2, 2026-09-09):
///
/// 1. **Entitlement outranks everything.** Past the 72-hour grace the step is a named skip, whatever
///    is installed locally, because from C2 on `judge` *is* a cloud call.
/// 2. **A vault with `config/cloud.yaml` runs `judge`, full stop.** No `--runtime`, no `--model`, and
///    nothing about the account on the command line: the engine reads `config/cloud.yaml` itself and
///    decides everything else, and it still always exits 0. A runtime-based skip here would leave
///    C2's whole judgment service inert on every machine that never installed llama.cpp.
/// 3. **Only a vault with no `config/cloud.yaml`** — a pre-C1 install, until Task 18's overlay adopts
///    it — falls through to the local runtime and model, which is exactly plan 3a's behaviour and
///    leaves in C4.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum JudgePlan {
    /// `judge --vault <v> --via local-runner --log-dir <d>`. The engine reads the account out of the
    /// vault; the only thing it cannot know is where this profile's judgment logs go, and **judgment
    /// logs never enter the vault** (`CLAUDE.md`), so `--log-dir` is in this arm as well as the local
    /// one (R-X-4).
    Cloud { log_dir: PathBuf },
    /// The local runtime and model this profile installed (plan 3a; removed in C4).
    Local(JudgeArgs),
    /// Not this slot, and why. A named step with exit code **0** — never a failure.
    Skip(&'static str),
}

pub fn judge_plan(cs: &ConsoleState) -> JudgePlan {
    judge_plan_for(entitlement_state(cs), cs)
}

/// `judge_plan`'s body, taking the entitlement state as a parameter rather than reading it itself —
/// fix round 1 (M3): `run_slot_inner` computes `entitlement_state(cs)` once and passes the same value
/// in here and to the telemetry step below, instead of two reads of `config/cloud.yaml` and the
/// entitlement cache per slot. `judge_plan(cs)` above is the public, one-read-per-call shape every
/// existing caller and test still uses.
fn judge_plan_for(state: crate::account::EntitlementState, cs: &ConsoleState) -> JudgePlan {
    match state {
        crate::account::EntitlementState::NotEntitled => JudgePlan::Skip("judge (skipped: no entitlement)"),
        // The same directory the local arm uses — `inference::judgments_dir(&cs.data_dir)`, which is
        // `%LOCALAPPDATA%\knowlu\profiles\<id>\judgments`. C4 removes the runtime, not this folder.
        crate::account::EntitlementState::Entitled => JudgePlan::Cloud { log_dir: crate::inference::judgments_dir(&cs.data_dir) },
        // Fix round 1, item 2: named distinctly from "no entitlement" — a broken config file is a
        // different problem from a lapsed subscription, and must never fall through to the local arm.
        crate::account::EntitlementState::Unreadable => JudgePlan::Skip("judge (skipped: cloud.yaml unreadable)"),
        crate::account::EntitlementState::NoAccount => match judge_state(cs) {
            JudgeState::Ready(a) => JudgePlan::Local(a),
            JudgeState::NoRuntime => JudgePlan::Skip("judge (skipped: no runtime)"),
            JudgeState::NoModel => JudgePlan::Skip("judge (skipped: no model)"),
        },
    }
}

/// coursework → **ingest** → **judge** → rank, as child processes of the sibling engine exe.
/// `ingest` is included only when the vault has a feed; `judge` only when a runtime and a model are
/// both installed, which is what `judge_state` decides.
///
/// **`judge` sits before `rank`**: it writes `effort_hours`, `importance` and `course`, and a rank
/// that ran first would order the day from the values the judge was about to replace — every
/// enrichment would be a slot late, forever.
///
/// Never build; never write to the vault directly.
pub fn slot_argv(vault: &Path, exe: &Path, judge: &JudgePlan) -> Vec<(PathBuf, Vec<String>)> {
    let v = vault.to_string_lossy().to_string();
    // **`sync` first** (C3′, cloud design §5.5 as amended). The pull is the half that has to precede
    // `rank`: a field another desktop set this morning must be in the note before the day is
    // ordered, or every second desktop ranks a slot behind forever. The push then carries
    // everything written since the last sync — the console's own edits and the previous slot's
    // machine writes. One step, not two, because `run_slot_inner` names a step by `args[0]` and two
    // rows both reading `sync` would say less than one row does. It always exits 0.
    let mut steps = vec![(exe.to_path_buf(), vec!["sync".into(), "--vault".into(), v.clone(), "--via".into(), "local-runner".into()])];
    steps.push((exe.to_path_buf(), vec!["coursework".into(), "--vault".into(), v.clone(), "--via".into(), "local-runner".into()]));
    // C2 Task 8: the LMS capability URL lives in the account from here on, so a cloud vault runs
    // `ingest` whether or not the vault still carries a copy — the feed lives in the account, and
    // the engine names a missing one as a skip (A-1) rather than this app leaving the step out.
    // Safe only with C2's H3/A-1 applied — without either, a blank `ics_url` (or a 404 from the
    // service) exits 1 forever before this app or the student can do anything about it.
    if ingest_included(vault) {
        steps.push((exe.to_path_buf(), vec!["ingest".into(), "--vault".into(), v.clone(), "--via".into(), "local-runner".into()]));
    }
    match judge {
        JudgePlan::Cloud { log_dir } => {
            // The vault and the log directory, and nothing else: `config/cloud.yaml` is in the vault,
            // and an account id on a command line is an account id in a process list.
            steps.push((exe.to_path_buf(), vec![
                "judge".into(), "--vault".into(), v.clone(), "--via".into(), "local-runner".into(),
                "--log-dir".into(), log_dir.to_string_lossy().into_owned(),
            ]));
        }
        JudgePlan::Local(j) => {
            steps.push((exe.to_path_buf(), vec![
                "judge".into(), "--vault".into(), v.clone(), "--via".into(), "local-runner".into(),
                "--runtime".into(), j.runtime.to_string_lossy().into_owned(),
                "--model".into(), j.model.to_string_lossy().into_owned(),
                "--log-dir".into(), j.log_dir.to_string_lossy().into_owned(),
            ]));
        }
        JudgePlan::Skip(_) => {}
    }
    steps.push((exe.to_path_buf(), vec!["rank".into(), "--vault".into(), v, "--runner".into(), "local".into()]));
    steps
}

/// The `local` entry from `config/runners.yaml`, if one exists — the only runner this scheduler
/// ever fires.
pub fn local_runner(vault: &Path) -> Option<RunnerConfig> {
    knowlu_engine::runs::load_runners_config(&vault.join("config").join("runners.yaml")).ok()?.into_iter().find(|r| r.name == "local")
}

/// `Script` unless `config/runners.yaml`'s `local` entry says `scheduler: app` — absent means
/// script, and the live vault has no such key throughout this plan.
pub fn mode(vault: &Path) -> SchedulerMode {
    knowlu_engine::runs::runner_settings(&vault.join("config").join("runners.yaml"), "local").scheduler
}

/// `true` when the `local` entry names no device, or names this one. Compared against
/// `journal::device_name()` (which checks `KNOWLU_DEVICE` before `COMPUTERNAME`/`HOSTNAME`) —
/// not the raw env vars directly — so a device pinned via `KNOWLU_DEVICE` is recognised the
/// same way the engine itself recognises it (review item 3); otherwise a second install of Knowlu
/// against a shared vault never double-runs the same slots.
pub fn device_ok(vault: &Path) -> bool {
    match knowlu_engine::runs::runner_settings(&vault.join("config").join("runners.yaml"), "local").device {
        None => true,
        Some(d) => knowlu_engine::journal::device_name().eq_ignore_ascii_case(&d),
    }
}

/// `<profile app data>\logs` — per profile since plan 4a Task 2, so two profiles on one machine
/// never interleave their slot logs. The temp dir is no longer a fallback here: `ConsoleState`
/// always has a data dir, because `open` was given one.
fn log_dir(cs: &ConsoleState) -> PathBuf {
    let d = cs.data_dir.join("logs");
    let _ = std::fs::create_dir_all(&d);
    d
}

/// Kills the child AND everything it spawned. `Child::kill` is a single `TerminateProcess` against
/// the one pid: a `cmd` wrapper — or any future engine step that forks — hands its children
/// inherited duplicates of the stdout/stderr pipe write ends, so killing the parent alone leaves
/// the pipes open and `run_child`'s drain threads block until the grandchild finishes on its own
/// (plan 1 final review, B5: the kill fired at the cap, the call returned minutes later). Windows
/// has no process group to signal, so this is `taskkill /T /F`; the plain kill after it is the
/// fallback for the case where `taskkill` is missing or refuses, and is harmless when the tree is
/// already gone.
fn kill_tree(child: &mut std::process::Child) {
    #[cfg(windows)]
    {
        use knowlu_engine::childproc::NoConsole;
        let _ = Command::new("taskkill")
            .no_console()
            .args(["/T", "/F", "/PID", &child.id().to_string()])
            .stdin(Stdio::null())
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .status();
    }
    let _ = child.kill();
    let _ = child.wait();
}

/// How long a failed slot waits before the tick tries it again: 5 min, 15 min, 30 min, then hourly
/// forever. A slot that fails before the engine writes its `start` record is never settled, so
/// without this the tick re-runs it every 60 seconds until midnight (plan 1 Task 12, minor).
///
/// **This never cancels a slot (F4)** — the slot stays owed exactly as `schedule::due_slot` says;
/// this decides only how often it is retried, and it never stops retrying.
pub fn should_retry(attempts: u32, since: std::time::Duration) -> bool {
    let wait_secs = match attempts { 0 => 0, 1 => 5 * 60, 2 => 15 * 60, 3 => 30 * 60, _ => 60 * 60 };
    since.as_secs() >= wait_secs
}

/// Keeps the newest `keep` **slot logs** in `dir` and returns how many were removed. Oldest by
/// modified time first.
///
/// Only regular files directly in `dir` whose name starts with `slot-` are counted or removed:
/// `log_dir()` also holds the tray's `quit-<ts>.txt` records, and anything else a later task drops
/// there. Counting those toward `keep` would silently shorten the slot history, and removing them
/// would delete the one trace a quit that timed out leaves behind. A missing directory is `0`, not
/// an error — housekeeping calls this on a schedule, before anything has necessarily written a log.
pub fn prune_logs(dir: &Path, keep: usize) -> usize {
    let Ok(rd) = std::fs::read_dir(dir) else { return 0 };
    let mut files: Vec<(std::time::SystemTime, PathBuf)> = rd.flatten()
        .filter(|e| e.file_type().map(|t| t.is_file()).unwrap_or(false))
        .filter(|e| e.file_name().to_string_lossy().starts_with("slot-"))
        .filter_map(|e| e.metadata().ok().and_then(|m| m.modified().ok()).map(|t| (t, e.path())))
        .collect();
    if files.len() <= keep { return 0; }
    files.sort();
    let doomed = files.len() - keep;
    files.iter().take(doomed).filter(|(_, p)| std::fs::remove_file(p).is_ok()).count()
}

/// Spawns `exe` with `args`, drains stdout/stderr on their own threads (so a full pipe can never
/// deadlock the wait loop), and polls `try_wait` every 500 ms up to `timeout` — past which the
/// child is killed and the step is recorded as `-2` ("timeout") rather than left to hang the whole
/// scheduler on a wedged vendor login (Task 12 review item 2).
///
/// `timeout` is a parameter, not the constant, purely as a test seam (final fix wave, B5):
/// production always passes `CHILD_TIMEOUT`, and `tests/scheduler.rs` can exercise both the kill
/// path and the pipe-filling drain path in seconds instead of twenty minutes.
pub fn run_child(exe: &Path, args: &[String], log: &Path, timeout: std::time::Duration) -> i32 {
    use knowlu_engine::childproc::NoConsole;
    let mut child = match Command::new(exe).no_console().args(args).stdin(Stdio::null()).stdout(Stdio::piped()).stderr(Stdio::piped()).spawn() {
        Ok(c) => c,
        Err(e) => {
            let _ = std::fs::write(log, format!("spawn failed: {e}"));
            return -1;
        }
    };
    let stdout = child.stdout.take();
    let stderr = child.stderr.take();
    let out_thread = std::thread::spawn(move || {
        let mut buf = Vec::new();
        if let Some(mut s) = stdout {
            let _ = s.read_to_end(&mut buf);
        }
        buf
    });
    let err_thread = std::thread::spawn(move || {
        let mut buf = Vec::new();
        if let Some(mut s) = stderr {
            let _ = s.read_to_end(&mut buf);
        }
        buf
    });
    let deadline = std::time::Instant::now() + timeout;
    let code = loop {
        match child.try_wait() {
            Ok(Some(status)) => break status.code().unwrap_or(-1),
            Ok(None) => {
                if std::time::Instant::now() >= deadline {
                    kill_tree(&mut child);
                    break -2;
                }
                std::thread::sleep(std::time::Duration::from_millis(500));
            }
            Err(_) => break -1,
        }
    };
    let out_buf = out_thread.join().unwrap_or_default();
    let err_buf = err_thread.join().unwrap_or_default();
    let _ = std::fs::write(log, [out_buf.as_slice(), b"\n--- stderr ---\n", err_buf.as_slice()].concat());
    code
}

/// A refusal is a slot outcome like any other, so it is stored in `sch.last` before it is returned
/// (final fix wave, B6): `attach_scheduler` copies `sch.last` into `topline.last_slot`, which is
/// what the page's sync line and the tray tooltip read. Before this, a refusal was returned and
/// dropped — `--run-slot-once` exited 2 and said so on stdout, but a running app went on showing
/// the previous *successful* run as the last slot, with nothing anywhere saying a run had been
/// refused or why.
fn refuse(sch: &Scheduler, late: bool, reason: &str) -> RunSummary {
    let now = knowlu_engine::journal::now_ts(None);
    let summary = RunSummary { started: now.clone(), ended: now, steps: vec![("refused".to_string(), -3)], ok: false, engine_ok: false, late, reason: Some(reason.to_string()), attempts: 0 };
    *lock(&sch.last) = Some(summary.clone());
    summary
}

/// Runs one slot: each step of `slot_argv` as a child process (the first of them `sync`, which
/// pulls and pushes inside the child) stdio captured to a log file → backup → refresh the shell's
/// caches, the sync status among them → tray colour (fix round 1, review M2: no separate pull or
/// push here any more, and no git). Never two at once — a slot already in progress returns the
/// last completed summary instead of starting a second.
///
/// Split into a window-free half (`run_slot_inner`) so a hidden CLI path (`--run-slot-once`) can
/// run exactly this without an `AppHandle` or a tray to update.
pub fn run_slot(app: &AppHandle, late: bool) -> RunSummary {
    let cs = app.state::<ConsoleState>();
    let sch = app.state::<Scheduler>();
    run_slot_inner(&cs, &sch, Some(app), late)
}

/// Refuses outright — no child spawned, no run record written — unless this vault's `local` entry
/// says `scheduler: app` AND this is the designated device (review item 7, ruling R-T12b): on the
/// live vault today (`scheduler` absent) a tray click or `--run-slot-once` would otherwise start a
/// second concurrent local run beside the PowerShell runner, and `rank` takes no lock of its own.
/// `paused` does NOT gate this — an explicit "Run now" always attempts to run; only the tick
/// thread's own automatic firing respects `paused`.
pub fn run_slot_inner(cs: &ConsoleState, sch: &Scheduler, tray_app: Option<&AppHandle>, late: bool) -> RunSummary {
    if mode(&cs.vault) != SchedulerMode::App {
        return refuse(sch, late, "scheduler is script on this vault");
    }
    if !device_ok(&cs.vault) {
        return refuse(sch, late, "not the designated device");
    }
    {
        let mut r = lock(&sch.running);
        if *r {
            return lock(&sch.last).clone().unwrap_or_else(|| RunSummary { started: String::new(), ended: String::new(), steps: vec![], ok: false, engine_ok: false, late, reason: None, attempts: 0 });
        }
        *r = true;
    }
    let _guard = RunGuard(sch);
    let started = knowlu_engine::journal::now_ts(None);
    let mut steps = Vec::new();
    let mut engine_ok = true;
    // A skipped step is still a step: without this line the Runs view and the sync line would show
    // a slot with no ingest in it and no reason why. `0` because a skip is not a failure. Recorded
    // before the engine is resolved, so a missing exe does not also hide the explanation. The two
    // reasons read differently (R-P4a-17).
    //
    // A-2 fix: gated on `ingest_included`, the SAME predicate `slot_argv` uses to decide whether to
    // run the step at all — never on `ics_state` alone. A cloud vault always has `ingest_included`
    // true (the feed lives in the account), so this line is never pushed for one; only a vault with
    // NO account and no local feed ever sees it, which is the one case `slot_argv` truly leaves the
    // step out.
    if !ingest_included(&cs.vault) {
        match ics_state(&cs.vault) {
            IcsState::Feed => {}
            IcsState::NoUrl => steps.push(("ingest (skipped: no ics_url)".to_string(), 0)),
            IcsState::Unreadable => steps.push(("ingest (skipped: config unreadable)".to_string(), 0)),
        }
    }
    // Every arm records a step with exit code **0** and a sentence — never a non-zero code, which
    // would set `engine_ok = false`, paint the tray amber and put the slot into retry backoff twice a
    // day for someone who has simply not paid, or not connected.
    //
    // Fix round 1 (M3): computed once and reused for the telemetry step below too, rather than
    // re-reading `config/cloud.yaml` and the entitlement cache from disk a second time in the same
    // slot — and guaranteeing the two steps agree even if the file changes mid-slot.
    let est = entitlement_state(cs);
    let judge = judge_plan_for(est, cs);
    if let JudgePlan::Skip(note) = &judge {
        steps.push(((*note).to_string(), 0));
    }
    match engine_exe() {
        Ok(exe) => {
            for (i, (e, args)) in slot_argv(&cs.vault, &exe, &judge).into_iter().enumerate() {
                let log = log_dir(cs).join(format!("slot-{}-{}-{}.txt", started.replace(':', ""), i, args[0]));
                let code = run_child(&e, &args, &log, CHILD_TIMEOUT);
                if code != 0 {
                    engine_ok = false;
                }
                steps.push((args[0].clone(), code));
            }
        }
        Err(e) => {
            engine_ok = false;
            steps.push((format!("engine: {e}"), -1));
        }
    }
    // F11: the backup walks and copies the whole working tree — the same tree the slot's own `sync`
    // step can rewrite — so it takes `vault_io` like every other vault-touching step. Taken HERE,
    // after the child wait and scoped to the engine call alone: `vault_io` is never held across a
    // child process (see `ConsoleState::vault_io`), which may run for twenty minutes.
    let backed = { let _io = lock(&cs.vault_io); state::run_backup(cs, jiff::Timestamp::now()) };
    if let Ok(st) = backed {
        steps.push(("backup".to_string(), if st.last_error.is_none() { 0 } else { 1 }));
    }
    // Spec §6: batched to `/telemetry` at each slot. **Never a failure** — a student on a train has
    // nothing to apologise for, and an analytics upload has no business turning a slot amber. Every
    // outcome is a named step with exit code 0, the same shape the ingest and judge skips use.
    //
    // Fix round 1 (M1): an unreadable `cloud.yaml` is its own named skip, distinct from "offline" —
    // the sibling `judge` step makes the same distinction and for the same reason: a broken config
    // file is a different problem from a network. (I2): a batch the server refuses outright (400,
    // 413, 422 — malformed and will never become well-formed) is *consumed*, not retried forever;
    // `telemetry::send` already advanced the watermark past it, so this only has to name the step.
    let telemetry = match est {
        crate::account::EntitlementState::NoAccount => ("telemetry (skipped: no account)".to_string(), 0),
        crate::account::EntitlementState::Unreadable => ("telemetry (skipped: cloud.yaml unreadable)".to_string(), 0),
        _ => match crate::telemetry::send(&cs.vault, &cs.data_dir, &cs.vault_io) {
            Ok(crate::telemetry::SendOutcome::Sent(0, 0)) => ("telemetry (nothing new)".to_string(), 0),
            Ok(crate::telemetry::SendOutcome::Sent(e, c)) => (format!("telemetry ({e} events, {c} corrections)"), 0),
            Ok(crate::telemetry::SendOutcome::Refused(status)) => (format!("telemetry (refused: {status})"), 0),
            Err(_) => ("telemetry (skipped: offline)".to_string(), 0),
        },
    };
    steps.push(telemetry);
    // Fix round 1, review I4: the slot's own `sync` step is a child process and cannot fill
    // `cs.sync` itself, so this reads back what it (or its own skip) left in `state/sync-status.json`.
    state::refresh_sync(cs);
    let ok = steps.iter().all(|(_, c)| *c == 0);
    let summary = RunSummary { started, ended: knowlu_engine::journal::now_ts(None), steps, ok, engine_ok, late, reason: None, attempts: 1 };
    *lock(&sch.last) = Some(summary.clone());
    if let Some(app) = tray_app {
        tray::set_state(app, if engine_ok { TrayState::Ok } else { TrayState::Warn });
    }
    summary
}

fn now_in(vault: &Path) -> jiff::Zoned {
    jiff::Zoned::now().with_time_zone(knowlu_engine::cli::vault_zone(vault))
}

/// Starts the tick and housekeeping threads, and computes `startup_missed` from the quit stamp
/// `settings.json` carried since last launch (cleared once read, so a crash between read and save
/// never double-counts).
pub fn spawn(app: AppHandle) {
    {
        let cs = app.state::<ConsoleState>();
        let quit_at = {
            let mut s = lock(&cs.settings);
            let q = s.quit_at.take();
            let _ = s.save(&cs.settings_path);
            q
        };
        if let (Some(q), Some(cfg)) = (quit_at, local_runner(&cs.vault)) {
            if let Ok(t) = q.parse::<jiff::Timestamp>() {
                let tz = knowlu_engine::cli::vault_zone(&cs.vault);
                let now = now_in(&cs.vault);
                let starts = schedule::run_starts(&cs.vault, "local", t.to_zoned(tz.clone()).date());
                cs.startup_missed.store(schedule::missed_slots(&cfg, &t.to_zoned(tz), &now, &starts), Ordering::SeqCst);
            }
        }
        // Prime the cached `(mode, device_ok)` immediately, rather than leaving it at the
        // `Default` placeholder until the housekeeping thread's first 60 s pass (review item 5).
        let sch = app.state::<Scheduler>();
        *lock(&sch.mode_device) = (mode(&cs.vault), device_ok(&cs.vault));
        // The first slot of a never-ranked vault runs now (`needs_first_run`), under the same
        // guards the tick applies — an app-scheduled vault on its designated device — and on its
        // own thread so the window is never held. `run_slot` serialises against a slot already in
        // flight and the tick's `due_slot` bookkeeping sees this run's `start` record like any
        // other, so nothing is run twice.
        if mode(&cs.vault) == SchedulerMode::App && device_ok(&cs.vault) && needs_first_run(&cs.vault) {
            let first = app.clone();
            std::thread::spawn(move || { let _ = run_slot(&first, false); });
        }
    }

    let tick = app.clone();
    std::thread::spawn(move || loop {
        {
            let cs = tick.state::<ConsoleState>();
            let sch = tick.state::<Scheduler>();
            if mode(&cs.vault) == SchedulerMode::App && device_ok(&cs.vault) && !sch.paused.load(Ordering::SeqCst) {
                if let Some(cfg) = local_runner(&cs.vault) {
                    let now = now_in(&cs.vault);
                    let starts = schedule::run_starts(&cs.vault, "local", now.date().yesterday().unwrap_or(now.date()));
                    if let Some(due) = schedule::due_slot(&cfg, &now, &starts) {
                        // A failed slot is still owed (F4) — `due_slot` keeps returning it until a
                        // `start` record settles it — so without a backoff the tick re-runs it every
                        // 60 s until midnight. `should_retry` decides only the cadence; the slot is
                        // never cancelled or skipped past.
                        //
                        // A slot sitting in backoff does hold up the day's LATER slots, since
                        // `due_slot` returns the earliest owed one and nothing else is looked at
                        // until it settles — but nothing is skipped, and the hold-up is one backoff
                        // window, at most an hour.
                        let key = due.due.to_string();
                        // A slot whose engine wrote its `start` record and then exited non-zero is
                        // settled: `due_slot` never returns it again, so its ladder entry would
                        // otherwise live for the life of the process. Sweep everything that is not
                        // from today's local date — the key's own first ten characters.
                        let today = now.date().to_string();
                        lock(&sch.attempts).retain(|k, _| k.get(..10) == Some(today.as_str()));
                        // Every guard is bound and dropped before the next line: `run_slot` takes
                        // `sch.running` and `sch.last` itself, so holding either across the call
                        // would deadlock the tick thread against its own slot.
                        let prior = lock(&sch.attempts).get(&key).copied();
                        let (n, since) = match prior {
                            Some((n, at)) => (n, at.elapsed()),
                            None => (0, std::time::Duration::ZERO),
                        };
                        // A slot already in flight (a tray "Run now", or a previous slot still
                        // going) would only get the placeholder summary back — leave its retry
                        // bookkeeping alone rather than stamping an attempt count onto someone
                        // else's run.
                        let in_flight = *lock(&sch.running);
                        if !in_flight && should_retry(n, since) {
                            let mut summary = run_slot(&tick, due.late);
                            // A refusal never invoked the engine, so it is not an attempt: it must
                            // neither count toward the ladder nor CLEAR one that earlier real
                            // attempts built (a `scheduler: app` key edited away mid-day, or a
                            // device rename, would otherwise reset the backoff to "retry now" every
                            // 60 s). `RunSummary.attempts` stays 0 on it, as its doc says.
                            if summary.reason.is_none() {
                                summary.attempts = n + 1;
                                if summary.engine_ok {
                                    lock(&sch.attempts).remove(&key);
                                } else {
                                    lock(&sch.attempts).insert(key, (n + 1, std::time::Instant::now()));
                                }
                                *lock(&sch.last) = Some(summary);
                            }
                        }
                    }
                }
            }
        }
        std::thread::sleep(std::time::Duration::from_secs(60));
    });

    let house = app.clone();
    std::thread::spawn(move || {
        let mut n: u64 = 0;
        loop {
            std::thread::sleep(std::time::Duration::from_secs(10));
            n += 1;
            let cs = house.state::<ConsoleState>();
            let sch = house.state::<Scheduler>();
            // A slot in flight already runs its own sync and its own backup around the child steps
            // — the housekeeping thread's debounced backup stands down rather than racing it for
            // `vault_io` (review item 4).
            let slot_running = *lock(&sch.running);
            let due_write = lock(&cs.last_write).map(|t| t.elapsed().as_secs() >= 30).unwrap_or(false) && cs.pending_edits.load(Ordering::SeqCst) > 0;
            if !slot_running && due_write {
                // F11, as in `run_slot_inner`: the backup reads the whole working tree, so it takes
                // `vault_io` for the engine call and gives it straight back.
                { let _io = lock(&cs.vault_io); let _ = state::run_backup(&cs, jiff::Timestamp::now()); }
                *lock(&cs.last_write) = None;
            }
            // Once an hour: up to six slot logs a day (two slots, three steps each on a vault with
            // a feed) would otherwise accumulate forever in the user's app data with nothing ever
            // removing them.
            if n % 360 == 0 {
                let _ = prune_logs(&log_dir(&cs), LOGS_KEPT);
            }
            if n % 6 == 0 {
                *lock(&sch.mode_device) = (mode(&cs.vault), device_ok(&cs.vault));
                let warn = knowlu_engine::runs::expected_status(&cs.vault, jiff::Timestamp::now())
                    .map(|rows| rows.iter().any(|r| r.status == "missing" || r.status == "crashed"))
                    .unwrap_or(true);
                let last_bad = lock(&sch.last).as_ref().map(|l| l.engine_failed()).unwrap_or(false);
                tray::set_state(&house, if warn || last_bad { TrayState::Warn } else { TrayState::Ok });
            }
            // Spec §5.1: at launch and every six hours. On its own thread for the reason the update
            // check is (R-P4a-24): one HTTPS round trip on a captive-portal wifi must not hold the
            // debounced sync, the backup, the log prune and the tray colour behind it. A failure is
            // silence — the previous cache stands, and the grace is what it is for.
            if n == 1 || n % 2160 == 0 {
                let vault = cs.vault.clone();
                let data = cs.data_dir.clone();
                std::thread::spawn(move || { let _ = crate::account::refresh_entitlement(&vault, &data); });
            }
            // Plan 4a Task 8: once a day while resident, and once at launch (n == 1) — Knowlu
            // spec §6. A failure is one recorded line and nothing else: until the site exists the
            // endpoint is unreachable, and that is the expected state, not an incident. It is
            // recorded (`record_check` writes `last_error`) but it never paints the tray Warn —
            // `set_state` above reads run records and the last slot, and an updater that cannot
            // reach a website has nothing to do with either.
            //
            // **On its own thread** (R-P4a-24). A manifest round-trip plus a multi-megabyte
            // download run inline would hold this loop — and with it the debounced sync, the
            // backup, the log prune and the tray colour — for as long as the endpoint takes to
            // answer, which is unbounded on a captive-portal wifi. The check is also timeboxed
            // inside `check_and_stage` (`CHECK_TIMEOUT`/`DOWNLOAD_TIMEOUT`), and single-flighted,
            // so this thread cannot pile up behind a slow one either: a second one refuses at once.
            if n == 1 || n % 8640 == 0 {
                let h = house.clone();
                std::thread::spawn(move || {
                    let Some(up) = h.try_state::<crate::updates::Updates>() else { return };
                    // The command's own resolver, not a copy of it (final review, I3): the staging
                    // folder a tick downloads into and the one *Check now* downloads into are the
                    // same folder, and stay the same folder if either ever moves.
                    let r = match crate::commands::updates_dir() {
                        // m4: no private folder to stage in is a failed check, recorded quietly
                        // like any other — never a stage into the shared system temp directory.
                        Err(e) => Err(e),
                        // The blocking twin: this is a std thread with no runtime under it, not a
                        // command (B4 / R-P4a-12) — `block_on` inside a command would panic.
                        Ok(dir) => crate::updates::check_and_stage_blocking(&h, &up, &dir),
                    };
                    // m2: a single-flight refusal is not a check — a manual one is already in
                    // flight and will record its own outcome. The tick says NOTHING rather than
                    // stamping `last_check` or planting an action error the user never caused;
                    // this thread is background work, and background work never touches that field.
                    if r.as_ref().err().map(String::as_str) != Some(crate::updates::ALREADY_CHECKING) {
                        crate::updates::record_check(&up, r, &knowlu_engine::journal::now_ts(None));
                    }
                    let sch = h.state::<Scheduler>();
                    let offer = crate::updates::update_offer(*lock(&sch.running), lock(&up.staged).as_ref());
                    // Braced deliberately: as the closure's tail expression, `lock(&up.item)`'s
                    // guard would outlive `up` itself and fail to borrow-check. The inner block
                    // drops the guard first, which is what the borrow checker is asking for.
                    {
                        let g = lock(&up.item);
                        if let Some(item) = g.as_ref() { let _ = item.set_enabled(offer.is_some()); }
                    }
                });
            }
        }
    });
}
