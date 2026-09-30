use knowlu::commands::attach_scheduler;
use knowlu::scheduler::{device_ok, engine_exe, entitlement_state, has_ics_url, ics_state, ingest_included, judge_plan, judge_state_in, lock, mode, prune_logs, run_child, run_slot_inner, should_retry, slot_argv, IcsState, JudgeArgs, JudgePlan, JudgeState, LiveSlot, Scheduler};
use knowlu::state::{quit_flush, ConsoleState};
use knowlu_engine::schedule::SchedulerMode;
use serde_json::{json, Value};
use std::path::{Path, PathBuf};
use std::time::{Duration, Instant};

/// Same helper as `tests/commands.rs` — copied rather than shared, since each `tests/*.rs` file
/// is its own crate.
fn scratch(name: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!("qo-console-sched-{name}-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    copy(Path::new("../engine/tests/fixtures/vault-full"), &dir);
    dir
}
fn copy(from: &Path, to: &Path) {
    std::fs::create_dir_all(to).unwrap();
    for e in std::fs::read_dir(from).unwrap().flatten() {
        let p = e.path();
        let t = to.join(e.file_name());
        if p.is_dir() { copy(&p, &t); } else { std::fs::copy(&p, &t).unwrap(); }
    }
}

fn open(v: &Path, tag: &str) -> ConsoleState {
    ConsoleState::open(v.to_path_buf(), std::env::temp_dir().join(format!("qo-console-sched-appdata-{tag}-{}", std::process::id())))
}

/// `KNOWLU_ENGINE_EXE` is process-wide state and `cargo test` runs this file's tests on several
/// threads at once — every test that sets or reads it (this one and the two `run_slot_inner`
/// skip tests below, Task 10 review) takes this lock for the whole of its read-modify-restore
/// section, so they can never interleave and race each other's view of the variable.
static ENGINE_ENV_LOCK: std::sync::Mutex<()> = std::sync::Mutex::new(());

/// Residual 2 (final fix wave re-review): an RAII seam for the process-global env vars this file's
/// tests set, so the restore survives a panicking assertion. The straight-line
/// set-call-restore this replaced was not unwind-safe: a panic between the call and the restore
/// left the seam (`LOCALAPPDATA`, `KNOWLU_ENGINE_EXE`) set for the rest of the process, and
/// because `ENGINE_ENV_LOCK` recovers from poison (`unwrap_or_else(|e| e.into_inner())`) that leak
/// would ride silently into every later test in this binary — the same shape as the
/// `KNOWLU_DEVICE` race this plan already paid for once. Kept alive for the whole test function
/// (never dropped early), `Drop::drop` restores every variable in one place whether the function
/// returns normally or unwinds.
struct EnvSeam {
    vars: Vec<(&'static str, Option<std::ffi::OsString>)>,
}
impl EnvSeam {
    fn set(pairs: &[(&'static str, &std::ffi::OsStr)]) -> Self {
        let vars = pairs
            .iter()
            .map(|(k, v)| {
                let prev = std::env::var_os(k);
                unsafe { std::env::set_var(k, v) };
                (*k, prev)
            })
            .collect();
        EnvSeam { vars }
    }
}
impl Drop for EnvSeam {
    fn drop(&mut self) {
        for (k, prev) in self.vars.drain(..) {
            match prev {
                Some(p) => unsafe { std::env::set_var(k, p) },
                None => unsafe { std::env::remove_var(k) },
            }
        }
    }
}

/// Fix round 1 (I1): making the offline-telemetry test send for real means it reaches
/// `valid_access_token_at`, which reads the real Windows Credential Manager. `CLAUDE.md` and
/// `app/tests/account.rs:204` require every test that touches the store to take a file-scoped lock —
/// this file's own, since `account.rs`'s is private to that crate. Windows races parallel
/// `CredWriteW`/`CredReadW` calls (spurious `ERROR_NOT_FOUND`), so this serialises every test in this
/// file that writes, reads or deletes a real credential, the same way it already serialises
/// `KNOWLU_ENGINE_EXE` above. The store is shared by every process as well, so this is the
/// cross-process lock in `support/credman_lock.rs`.
#[cfg(windows)]
#[path = "support/credman_lock.rs"]
mod credman_lock;
#[cfg(windows)]
static CREDMAN_LOCK: credman_lock::CredmanLock = credman_lock::CredmanLock::new();

/// Deletes the named Credential Manager target when the test ends, on any exit path — a passing
/// assertion, a failing one, or a panic. The same shape as `app/tests/account.rs`'s `Cleanup`.
#[cfg(windows)]
struct CredCleanup(String);
#[cfg(windows)]
impl Drop for CredCleanup {
    fn drop(&mut self) {
        let _ = knowlu::credentials::delete(&self.0);
    }
}

#[test]
fn the_slot_runs_coursework_ingest_judge_rank_and_leaves_out_what_is_not_configured() {
    let v = scratch("argv");
    let exe = Path::new(r"C:\bin\knowlu-engine.exe");
    let names = |a: &Vec<(PathBuf, Vec<String>)>| a.iter().map(|(_, x)| x[0].clone()).collect::<Vec<_>>();
    assert!(!has_ics_url(&v));
    // No judge args: the step is left out entirely, exactly as `ingest` is on a vault with no feed.
    // `sync` is always first (C3′, cloud design §5.5 as amended): the pull half has to land before
    // `rank` orders the day, and the push half carries everything written since the last sync.
    let argv = slot_argv(&v, exe, &JudgePlan::Skip("judge (skipped: no runtime)"));
    assert_eq!(names(&argv), vec!["sync", "coursework", "rank"]);
    assert_eq!(argv[0].1, vec!["sync", "--vault", v.to_string_lossy().as_ref(), "--via", "local-runner"]);
    assert_eq!(argv[1].1, vec!["coursework", "--vault", v.to_string_lossy().as_ref(), "--via", "local-runner"]);
    assert_eq!(argv[2].1, vec!["rank", "--vault", v.to_string_lossy().as_ref(), "--runner", "local"]);

    let cfg = v.join("config").join("ingest.yaml");
    let old = std::fs::read_to_string(&cfg).unwrap();
    std::fs::write(&cfg, format!("ics_url: \"https://lms.example.invalid/learn.ics\"\n{old}")).unwrap();
    assert!(has_ics_url(&v));
    let argv = slot_argv(&v, exe, &JudgePlan::Skip("judge (skipped: no runtime)"));
    assert_eq!(names(&argv), vec!["sync", "coursework", "ingest", "rank"]);
    assert_eq!(argv[2].1, vec!["ingest", "--vault", v.to_string_lossy().as_ref(), "--via", "local-runner"]);
    assert!(argv.iter().all(|(e, _)| e == exe));

    // With judge args: FIVE steps, and judge sits BEFORE rank so the day's ranking sees what it
    // just wrote. `--via local-runner`, the same value coursework and ingest pass — journal::VIAS
    // does not grow for this.
    let ja = JudgeArgs {
        runtime: PathBuf::from(r"C:\rt\llama-cli.exe"),
        model: PathBuf::from(r"C:\rt\model.gguf"),
        log_dir: PathBuf::from(r"C:\data\judgments"),
    };
    let argv = slot_argv(&v, exe, &JudgePlan::Local(ja));
    assert_eq!(names(&argv), vec!["sync", "coursework", "ingest", "judge", "rank"]);
    assert_eq!(argv[3].1, vec![
        "judge".to_string(), "--vault".to_string(), v.to_string_lossy().to_string(),
        "--via".to_string(), "local-runner".to_string(),
        "--runtime".to_string(), r"C:\rt\llama-cli.exe".to_string(),
        "--model".to_string(), r"C:\rt\model.gguf".to_string(),
        "--log-dir".to_string(), r"C:\data\judgments".to_string(),
    ]);
    assert!(knowlu_engine::journal::VIAS.contains(&"local-runner"));

    std::fs::write(&cfg, "ics_url: \"   \"\n").unwrap();
    assert!(!has_ics_url(&v), "a blank url is no url — the engine would exit 1 on it");
    // R-P4a-17: the three states are distinguishable, and a missing file is "no url", not a fault.
    assert_eq!(ics_state(&v), IcsState::NoUrl);
    std::fs::write(&cfg, "ics_url: [unclosed\n").unwrap();
    assert_eq!(ics_state(&v), IcsState::Unreadable);
    std::fs::remove_file(&cfg).unwrap();
    assert_eq!(ics_state(&v), IcsState::NoUrl);
}

/// C2 hand-off H5: a cloud vault runs `ingest` even with a blank `ics_url`, because the feed can
/// come from `/ingest-ics` instead — safe only because C2's H3 makes the engine ask the service
/// before it refuses a blank url. A vault with no `config/cloud.yaml` keeps leaving `ingest` out,
/// exactly as `the_slot_runs_coursework_ingest_judge_rank_and_leaves_out_what_is_not_configured`
/// pins above.
#[test]
fn a_cloud_vault_runs_ingest_with_no_ics_url() {
    let v = scratch("cloud-argv");
    let exe = Path::new(r"C:\bin\knowlu-engine.exe");
    let names = |a: &Vec<(PathBuf, Vec<String>)>| a.iter().map(|(_, x)| x[0].clone()).collect::<Vec<_>>();

    let cfg = v.join("config").join("ingest.yaml");
    std::fs::write(&cfg, "ics_url: \"   \"\n").unwrap();
    assert!(!has_ics_url(&v));

    std::fs::write(
        v.join("config").join("cloud.yaml"),
        "api_base: 'https://cloud.example.invalid/functions/v1'\nanon_key: 'anon-not-a-secret'\nsession_credential_target: 'knowlu/test-profile/session'\naccount_id: 'acct-1'\n",
    )
    .unwrap();

    let argv = slot_argv(&v, exe, &JudgePlan::Skip("judge (skipped: no runtime)"));
    assert_eq!(names(&argv), vec!["sync", "coursework", "ingest", "rank"]);
}

/// C3′ Task 11, verifying hand-off H8c from the outside: the superseded plan was going to add a
/// fourth `IcsState` for exactly this, and C2's own A-2 fix (`ingest_included`) made it unnecessary.
/// A vault carrying `ics_url: ''` and no account at all leaves `ingest` out, same as always; the
/// moment `config/cloud.yaml` exists, the feed lives in the account and the step runs regardless of
/// the vault's own (now-empty) url.
#[test]
fn a_cloud_vault_runs_ingest_with_no_url_in_the_vault_at_all() {
    let v = scratch("cloud-ingest");
    std::fs::write(v.join("config").join("ingest.yaml"), "timezone: America/Chicago\nics_url: ''\n").expect("ingest.yaml");
    assert_eq!(ics_state(&v), IcsState::NoUrl);
    assert!(!ingest_included(&v), "with no account and no url the step is left out, as it always was");
    std::fs::write(
        v.join("config").join("cloud.yaml"),
        "api_base: 'https://x.example.invalid/functions/v1'\nanon_key: 'anon'\nsession_credential_target: 'knowlu/p/session'\naccount_id: 'acct-1'\n",
    ).expect("cloud.yaml");
    assert!(ingest_included(&v), "an account is a feed, wherever the URL lives");
    let steps = slot_argv(&v, Path::new("knowlu-engine.exe"), &JudgePlan::Skip("judge (skipped: no entitlement)"));
    assert_eq!(steps[0].1[0], "sync", "sync runs first");
    assert_eq!(steps.iter().filter(|(_, a)| a[0] == "ingest").count(), 1, "{steps:?}");
    let _ = std::fs::remove_dir_all(&v);
}

/// The plan's own case for hand-off H8a: `sync` is the slot's first step, always, and it displaces
/// nothing that was there before. One row in the Runs view, not two, because `run_slot_inner` names
/// a step by `args[0]` and two rows both reading `sync` would say less than one row does.
#[test]
fn sync_is_the_slots_first_step() {
    let v = scratch("sync-first");
    let exe = Path::new("knowlu-engine.exe");
    let steps = slot_argv(&v, exe, &JudgePlan::Skip("judge (skipped: no entitlement)"));
    assert_eq!(steps[0].1[0], "sync", "{steps:?}");
    assert_eq!(steps[1].1[0], "coursework", "and nothing was displaced");
    assert!(steps.iter().any(|(_, a)| a[0] == "rank"), "{steps:?}");
    // One row in the Runs view, not two: `run_slot_inner` names a step by `args[0]`.
    assert_eq!(steps.iter().filter(|(_, a)| a[0] == "sync").count(), 1);
    let _ = std::fs::remove_dir_all(&v);
}

/// D7: no runtime and no model are NORMAL. The step is recorded with code 0 and a name that says
/// which half is missing — the shape `ingest (skipped: no ics_url)` already uses — so a friend with
/// no model sees an explanation on the Runs view rather than a slot that quietly does less.
///
/// **Against a temp `LOCALAPPDATA`, never the real one** (M2, final fix wave): `run_slot_inner`
/// resolves the judge step through `judge_state(cs)` -> `commands::inference_root()` ->
/// `state::app_data_root()`, which reads the process's real `LOCALAPPDATA` when the variable is
/// left alone — unlike `judge_state_names_the_missing_half_runtime_first` below, this test had no
/// seam for it. The first time Quinn uses the settings row this branch shipped to install a
/// runtime and a model, `judge_state` would return `Ready`, no skip step would be pushed, and
/// `.expect("a judge step, named")` would panic — exactly the failure mode
/// `judge_state_in`'s own doc names ("a test that breaks when the feature starts working is worse
/// than no test"). Seaming it also removes a side effect this test had before the fix: with the
/// real root in play, `app_data_root()`'s call to `profiles::migrate_flat_layout` ran a migration
/// attempt against the user's live app data on every `cargo test`.
///
/// Residual 2 (final fix wave re-review): the seam is now an `EnvSeam` held for the whole
/// function, so a panicking assertion below still restores both env vars via `Drop` — the
/// straight-line restore this replaced would have left `LOCALAPPDATA` pointed at a (by-then
/// deleted) temp directory for every later test in this binary.
#[test]
fn a_machine_with_no_model_records_the_judge_skip_and_stays_green() {
    let v = scratch("judgeskip");
    std::fs::write(
        v.join("config").join("runners.yaml"),
        format!("runners:\n  - name: local\n    times: [\"12:00\"]\n    tz: America/Chicago\n    grace_minutes: 20\n    device: {}\n    scheduler: app\n", knowlu_engine::journal::device_name()),
    ).unwrap();
    let cs = open(&v, "judgeskip");
    let sch = Scheduler::default();
    let fake_local_appdata = std::env::temp_dir().join(format!("qo-console-sched-localappdata-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&fake_local_appdata);
    std::fs::create_dir_all(&fake_local_appdata).unwrap();
    // Task 10 review: the property that actually matters is `engine_ok`, not just the skipped
    // step's own exit code — and proving that needs the OTHER steps (coursework, rank) to run
    // something real rather than fail on a missing engine exe for an unrelated reason. `cmd`,
    // resolved via PATH like the child-process tests above, ignores unknown positional args and
    // exits 0 on non-interactive stdio — a stand-in "always succeeds" engine. Held under
    // `ENGINE_ENV_LOCK` for the whole call, since `KNOWLU_ENGINE_EXE` and `LOCALAPPDATA` are both
    // process-global state (M2) -- and `_env` stays alive to the end of the function (never
    // dropped early), so its restore runs even if an assertion below panics.
    let _guard = ENGINE_ENV_LOCK.lock().unwrap_or_else(|e| e.into_inner());
    let _env = EnvSeam::set(&[
        ("KNOWLU_ENGINE_EXE", std::ffi::OsStr::new("cmd")),
        ("LOCALAPPDATA", fake_local_appdata.as_os_str()),
    ]);
    let summary = run_slot_inner(&cs, &sch, None, false);
    let names: Vec<String> = summary.steps.iter().map(|(n, _)| n.clone()).collect();
    let judge = names.iter().find(|n| n.starts_with("judge")).expect("a judge step, named");
    assert!(judge.contains("skipped: no runtime") || judge.contains("skipped: no model"), "{judge}");
    let code = summary.steps.iter().find(|(n, _)| n.starts_with("judge")).unwrap().1;
    assert_eq!(code, 0, "a skip is not a failure — it must never paint the tray amber");
    // Task 10 review: the step's own exit code being 0 is not the same claim as the slot staying
    // green — `engine_ok` is what `attach_scheduler` feeds the tray, so THIS is what must be true.
    assert!(summary.engine_ok, "a skipped judge step must not paint the tray amber: {:?}", summary.steps);
    let _ = std::fs::remove_dir_all(&fake_local_appdata);
    let _ = std::fs::remove_dir_all(&v);
}

/// `judge_state_in` names which half is missing — the runtime first, because it is the prerequisite
/// for the other.
///
/// **Against a temp root, never the real one** (review S3): `judge_state` reads
/// `%LOCALAPPDATA%\knowlu`, so a test written against it would pass today and start failing the
/// first time the settings row actually installed something on this machine.
#[test]
fn judge_state_names_the_missing_half_runtime_first() {
    let v = scratch("judgestate");
    let cs = open(&v, "judgestate");
    let root = std::env::temp_dir().join(format!("knowlu-judgestate-root-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&root);
    std::fs::create_dir_all(&root).unwrap();

    assert_eq!(judge_state_in(&root, &cs), JudgeState::NoRuntime, "neither half installed");

    let rt = knowlu::inference::runtime_dir(&root);
    std::fs::create_dir_all(&rt).unwrap();
    std::fs::write(rt.join(knowlu_engine::runtime::RUNTIME_EXE), b"MZ").unwrap();
    assert_eq!(judge_state_in(&root, &cs), JudgeState::NoModel, "runtime but no model");

    let models = knowlu::inference::models_dir(&root);
    std::fs::create_dir_all(&models).unwrap();
    std::fs::write(models.join("m.gguf"), b"gg").unwrap();
    match judge_state_in(&root, &cs) {
        JudgeState::Ready(a) => {
            assert!(a.runtime.ends_with(knowlu_engine::runtime::RUNTIME_EXE));
            assert!(a.model.ends_with("m.gguf"));
            // The log is the PROFILE's, never the install-wide root's (spec §5.4).
            assert_eq!(a.log_dir, cs.data_dir.join("judgments"));
            assert!(!a.log_dir.starts_with(&root), "the log must not live beside the model");
        }
        other => panic!("expected Ready, got {other:?}"),
    }
    let _ = std::fs::remove_dir_all(&root);
    let _ = std::fs::remove_dir_all(&v);
}

/// R-P4a-9: the skip is visible — the explanation is recorded before the engine exe is resolved,
/// so a missing engine never hides it.
///
/// **Task 10 review**: the property that actually matters is `engine_ok`, not just the skipped
/// step's own exit code, and proving THAT needs the other steps (coursework, rank) to run
/// something real rather than fail on a missing engine exe for an unrelated reason — with no
/// stub, `engine_ok` would be false here regardless of whether the skip logic has a bug. `cmd`,
/// resolved via PATH like the child-process tests above, ignores unknown positional args and
/// exits 0 on non-interactive stdio — a stand-in "always succeeds" engine.
///
/// **Against a temp `LOCALAPPDATA`, never the real one** (residual 1, final fix wave re-review):
/// `run_slot_inner` calls `judge_state(cs)` unconditionally — this test never asserts on the
/// judge step, but the call still resolves `state::app_data_root()` against whatever
/// `LOCALAPPDATA` is set to, running `profiles::migrate_flat_layout` against it. M2 seamed
/// `a_machine_with_no_model_records_the_judge_skip_and_stays_green` but left this sibling, its
/// exact twin in every way that matters, still reaching the real root on every `cargo test` run.
#[test]
fn a_vault_without_a_feed_records_the_ingest_skip_in_the_step_list() {
    let v = scratch("skipstep");
    std::fs::write(
        v.join("config").join("runners.yaml"),
        format!("runners:\n  - name: local\n    times: [\"12:00\"]\n    tz: America/Chicago\n    grace_minutes: 20\n    device: {}\n    scheduler: app\n", knowlu_engine::journal::device_name()),
    ).unwrap();
    let cs = open(&v, "skipstep");
    let sch = Scheduler::default();
    let fake_local_appdata = std::env::temp_dir().join(format!("qo-console-sched-localappdata-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&fake_local_appdata);
    std::fs::create_dir_all(&fake_local_appdata).unwrap();
    // Held under `ENGINE_ENV_LOCK` for the whole call, since `KNOWLU_ENGINE_EXE` and
    // `LOCALAPPDATA` are both process-global state and this file's tests run on several threads at
    // once. `_env` stays alive to the end of the function so its `Drop` restore runs even if an
    // assertion below panics (residual 2).
    let _guard = ENGINE_ENV_LOCK.lock().unwrap_or_else(|e| e.into_inner());
    let _env = EnvSeam::set(&[
        ("KNOWLU_ENGINE_EXE", std::ffi::OsStr::new("cmd")),
        ("LOCALAPPDATA", fake_local_appdata.as_os_str()),
    ]);
    let s = run_slot_inner(&cs, &sch, None, false);
    assert!(s.reason.is_none(), "not refused: {:?}", s.reason);
    let named: Vec<String> = s.steps.iter().map(|(n, _)| n.clone()).collect();
    assert!(named.contains(&"ingest (skipped: no ics_url)".to_string()), "{named:?}");
    assert!(!named.iter().any(|n| n == "ingest"), "the step itself never ran");
    assert_eq!(s.steps.iter().find(|(n, _)| n.starts_with("ingest (skipped")).unwrap().1, 0, "a skip is not a failure");
    // Task 10 review: the step's own exit code being 0 is not the same claim as the slot staying
    // green — `engine_ok` is what `attach_scheduler` feeds the tray, so THIS is what must be true.
    assert!(s.engine_ok, "a skipped ingest step must not paint the tray amber: {:?}", s.steps);
    let _ = std::fs::remove_dir_all(&fake_local_appdata);
    let _ = std::fs::remove_dir_all(&v);
}

/// A-2: a cloud vault with no `ics_url` used to get BOTH a fake `ingest (skipped: no ics_url)`
/// step (pushed on `ics_state` alone) AND the real `ingest` child process's own step (`slot_argv`
/// includes it whenever `config/cloud.yaml` exists) — two `ingest` entries in one
/// `RunSummary.steps`, one of them describing a step that never actually ran. The fix is one
/// decision point (`ingest_included`) both places now read; this proves `steps` names `ingest`
/// exactly once for a cloud vault, and that the one entry is the REAL step, never the skip line.
#[test]
fn a_cloud_vault_names_ingest_exactly_once_never_the_skip_line_too() {
    let v = scratch("cloudingestonce");
    std::fs::write(
        v.join("config").join("runners.yaml"),
        format!("runners:\n  - name: local\n    times: [\"12:00\"]\n    tz: America/Chicago\n    grace_minutes: 20\n    device: {}\n    scheduler: app\n", knowlu_engine::journal::device_name()),
    ).unwrap();
    // No `ics_url` at all — exactly `a_vault_without_a_feed_records_the_ingest_skip_in_the_step_list`'s
    // setup, except this vault ALSO has an account, which is the one thing that must change the
    // answer (H5/A-1: the feed lives in the account from here on).
    std::fs::write(
        v.join("config").join("cloud.yaml"),
        "api_base: 'https://cloud.example.invalid/functions/v1'\nanon_key: 'anon-not-a-secret'\nsession_credential_target: 'knowlu/test/session'\naccount_id: 'acct-1'\n",
    ).unwrap();
    assert!(!has_ics_url(&v));
    let cs = open(&v, "cloudingestonce");
    let sch = Scheduler::default();
    let fake_local_appdata = std::env::temp_dir().join(format!("qo-console-sched-localappdata-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&fake_local_appdata);
    std::fs::create_dir_all(&fake_local_appdata).unwrap();
    let _guard = ENGINE_ENV_LOCK.lock().unwrap_or_else(|e| e.into_inner());
    let _env = EnvSeam::set(&[
        ("KNOWLU_ENGINE_EXE", std::ffi::OsStr::new("cmd")),
        ("LOCALAPPDATA", fake_local_appdata.as_os_str()),
    ]);
    let s = run_slot_inner(&cs, &sch, None, false);
    let named: Vec<String> = s.steps.iter().map(|(n, _)| n.clone()).collect();
    let ingest_entries: Vec<&String> = named.iter().filter(|n| n.starts_with("ingest")).collect();
    assert_eq!(ingest_entries.len(), 1, "exactly one ingest entry, real or skipped: {named:?}");
    assert_eq!(ingest_entries[0], "ingest", "the one entry must be the REAL step, not a skip line: {named:?}");
    let _ = std::fs::remove_dir_all(&fake_local_appdata);
    let _ = std::fs::remove_dir_all(&v);
}

#[test]
fn engine_exe_prefers_the_env_override_and_names_both_places_when_missing() {
    let _guard = ENGINE_ENV_LOCK.lock().unwrap_or_else(|e| e.into_inner());
    unsafe { std::env::set_var("KNOWLU_ENGINE_EXE", r"C:\somewhere\knowlu-engine.exe"); }
    assert_eq!(engine_exe().unwrap(), PathBuf::from(r"C:\somewhere\knowlu-engine.exe"));
    unsafe { std::env::remove_var("KNOWLU_ENGINE_EXE"); }
    match engine_exe() {
        Ok(p) => assert!(p.ends_with("knowlu-engine.exe")),
        Err(e) => assert!(e.contains("KNOWLU_ENGINE_EXE") && e.contains("knowlu-engine.exe"), "{e}"),
    }
}

#[test]
fn the_scheduler_is_inert_on_script_and_only_on_the_designated_device() {
    let v = scratch("mode");
    assert_eq!(mode(&v), SchedulerMode::Script, "vault-full's runners.yaml has no scheduler key");
    let yaml = std::fs::read_to_string(v.join("config/runners.yaml")).unwrap();
    std::fs::write(v.join("config/runners.yaml"), yaml.replace("grace_minutes: 20", "grace_minutes: 20\n    scheduler: app\n    device: NOT-THIS-MACHINE")).unwrap();
    assert_eq!(mode(&v), SchedulerMode::App);
    assert!(!device_ok(&v), "a second install never double-runs a shared vault");
}

/// Review item 6: `attach_scheduler` copies live scheduler status into an envelope's
/// `state.topline`, and is a true no-op — keys absent, not null placeholders — on an envelope
/// that carries no `state` key at all. Task 15's page must tolerate `topline.scheduler` and
/// `topline.last_slot` being missing entirely, not assume they are always present.
#[test]
fn attach_scheduler_copies_live_status_and_leaves_a_stateless_envelope_untouched() {
    let sch = Scheduler::default();
    *sch.mode_device.lock().unwrap() = (SchedulerMode::App, false);

    let mut env = json!({ "ok": true, "error": Value::Null, "state": { "topline": {} } });
    attach_scheduler(&mut env, &sch).unwrap();
    assert_eq!(env["state"]["topline"]["scheduler"]["mode"], "app");
    assert_eq!(env["state"]["topline"]["scheduler"]["paused"], false);
    assert_eq!(env["state"]["topline"]["scheduler"]["device_ok"], false);
    assert!(env["state"]["topline"]["last_slot"].is_null(), "no slot has run yet");

    let mut stateless = json!({ "ok": false, "error": "no vault", "state": Value::Null });
    let before = stateless.clone();
    attach_scheduler(&mut stateless, &sch).unwrap();
    assert_eq!(stateless, before, "a state-less envelope is returned byte-for-byte unchanged");
}

/// Final fix wave C1: `attach_scheduler` read `sch.last` and `sch.mode_device` — two of the
/// mutexes plan 2 Task 5 made poison-tolerant — with `.map_err(|_| "lock")?`, and every caller
/// throws its error away (`let _ = attach_scheduler(...)`). So one panic under a slot would have
/// silently stripped `last_slot` and `scheduler` from every poll for the life of the process,
/// with nothing said in the envelope, the sync line or the log. Both go through the scheduler's
/// own `lock` helper now, and a poisoned mutex reports the value it was holding.
#[test]
fn attach_scheduler_still_reports_after_a_panic_poisoned_the_scheduler_mutexes() {
    let sch = Scheduler::default();
    *sch.mode_device.lock().unwrap() = (SchedulerMode::App, true);
    // Poison both by panicking while holding them, the way a panic under a slot would.
    let _ = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
        let _g = sch.last.lock().unwrap();
        panic!("a slot panicked while holding sch.last");
    }));
    let _ = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
        let _g = sch.mode_device.lock().unwrap();
        panic!("a slot panicked while holding sch.mode_device");
    }));
    assert!(sch.last.lock().is_err() && sch.mode_device.lock().is_err(), "both mutexes really are poisoned");

    let mut env = json!({ "ok": true, "error": Value::Null, "state": { "topline": {} } });
    attach_scheduler(&mut env, &sch).expect("a poisoned mutex is not an error here");
    assert_eq!(env["state"]["topline"]["scheduler"]["mode"], "app");
    assert_eq!(env["state"]["topline"]["scheduler"]["device_ok"], true);
    assert!(env["state"]["topline"].get("last_slot").is_some(), "last_slot is present, not silently dropped");
}

/// Review item 7 / ruling R-T12b: `run_slot_inner` refuses outright — no child spawned, no run
/// record written — when the vault's `local` entry isn't `scheduler: app`, so a tray click or
/// `--run-slot-once` on the live vault (or any vault still on the PowerShell runner) can never
/// start a second concurrent local run.
#[test]
fn run_slot_inner_refuses_on_scheduler_script_without_spawning_anything() {
    let v = scratch("refuse");
    let cs = open(&v, "refuse");
    let sch = Scheduler::default();
    assert_eq!(mode(&v), SchedulerMode::Script, "vault-full's runners.yaml has no scheduler key");
    let runs_dir = v.join("state/runs");
    let snapshot = |dir: &Path| -> Vec<(std::path::PathBuf, u64)> {
        let mut v: Vec<_> = std::fs::read_dir(dir).unwrap().flatten()
            .map(|e| (e.path(), e.metadata().unwrap().len())).collect();
        v.sort();
        v
    };
    let before = snapshot(&runs_dir);

    let summary = run_slot_inner(&cs, &sch, None, false);

    assert!(!summary.ok);
    assert!(!summary.engine_ok);
    assert_eq!(summary.steps, vec![("refused".to_string(), -3)]);
    assert_eq!(summary.reason.as_deref(), Some("scheduler is script on this vault"));
    assert_eq!(before, snapshot(&runs_dir), "no run record was written or appended to — nothing was spawned");
    // Final fix wave, B6: a refusal is a slot outcome, so it lands in `sch.last` — that is what
    // `commands::attach_scheduler` copies into `topline.last_slot`, so the page's sync line and
    // the tray tooltip can say WHY nothing ran instead of showing the last successful slot.
    let last = sch.last.lock().unwrap().clone().expect("a refusal is recorded as the last slot");
    assert!(last.reason.is_some(), "…carrying its reason: {last:?}");
    assert_eq!(last.reason.as_deref(), summary.reason.as_deref());
    assert!(!last.ok && !last.engine_ok);
    // Final re-review, minor 1: a refusal never ran the engine, so housekeeping must not read it
    // as "the last slot failed" and latch the tray Warn — on a script-mode vault no successful
    // slot would ever clear it.
    assert!(!last.engine_failed(), "a refusal is not an engine failure: {last:?}");
}

/// B5: `run_child`'s cap is a parameter now (production still passes `CHILD_TIMEOUT`), so the kill
/// path is testable in a second instead of twenty minutes. A child that outlives its cap is killed
/// and recorded as `-2` — a code no real process exit produces, so "timed out" stays distinguishable
/// from "ran and failed". No vault, no network.
#[test]
fn a_child_that_outlives_its_cap_is_killed_and_recorded_as_minus_two() {
    let log = std::env::temp_dir().join(format!("qo-console-sched-timeout-{}.txt", std::process::id()));
    // `ping -n 5 127.0.0.1` takes about four seconds on loopback — comfortably past a one-second
    // cap, and no network leaves the machine.
    //
    // Spawned DIRECTLY, not as `cmd /c ping …` (measured, final fix wave): `cmd` creates `ping`
    // with handle inheritance on, so `ping` holds duplicates of the stdout/stderr pipe write ends
    // whatever `cmd`'s own redirections say. Killing `cmd` at the cap therefore does not close the
    // pipes, and `run_child`'s drain threads keep reading until `ping` finishes on its own — the
    // kill fires at 1 s but the call returns at 4 s. Production only ever runs `knowlu-engine.exe`,
    // which spawns no grandchildren of its own, so the direct spawn is the case that matters.
    let args: Vec<String> = ["-n", "5", "127.0.0.1"].iter().map(|s| s.to_string()).collect();
    let started = Instant::now();
    let code = run_child(Path::new("ping"), &args, &log, Duration::from_secs(1));
    let elapsed = started.elapsed();
    assert_eq!(code, -2, "a killed child is -2, never a real exit code");
    assert!(elapsed < Duration::from_secs(3), "the cap fired rather than the child finishing: {elapsed:?}");
}

/// B5: the two drain threads exist so a child that fills the pipe buffer (4 KB on Windows) cannot
/// deadlock the `try_wait` loop — the child blocks writing, the parent never reads, and both sit
/// there until the cap. 2000 lines is well past the buffer; with a generous cap, a hang would show
/// as a timeout (`-2`) rather than a clean exit.
#[test]
fn a_chatty_child_fills_the_pipe_and_still_exits_clean_with_its_whole_log() {
    let log = std::env::temp_dir().join(format!("qo-console-sched-chatty-{}.txt", std::process::id()));
    let args: Vec<String> = ["/c", "for", "/l", "%i", "in", "(1,1,2000)", "do", "@echo", "line", "%i"].iter().map(|s| s.to_string()).collect();
    let code = run_child(Path::new("cmd"), &args, &log, Duration::from_secs(60));
    assert_eq!(code, 0, "the drain threads beat the pipe; a deadlock would have timed out at -2");
    let text = std::fs::read_to_string(&log).unwrap();
    assert!(text.contains("line 1") && text.contains("line 2000"), "the whole of a pipe-filling stdout is captured ({} bytes)", text.len());
}

/// Plan 2 Task 5 (F1; plan 1 final review, B5): the cap kills the process TREE. `cmd /c ping` is
/// exactly the shape the fix wave could not meet — `cmd` creates `ping` with handle inheritance
/// on, so the grandchild holds duplicates of the stdout/stderr pipe write ends and killing `cmd`
/// alone left `run_child`'s drain threads reading until `ping` finished on its own (the kill fired
/// at 1 s, the call returned at ~5 s). `taskkill /T` takes the whole tree, so the call now returns
/// at the cap. No vault, no network — loopback only.
#[test]
fn a_child_tree_that_outlives_its_cap_is_killed_whole_and_returns_at_the_cap() {
    let log = std::env::temp_dir().join(format!("qo-console-sched-tree-{}.log", std::process::id()));
    let args: Vec<String> = vec!["/c".into(), "ping -n 6 127.0.0.1 >nul".into()];
    let started = Instant::now();
    let code = run_child(Path::new("cmd"), &args, &log, Duration::from_secs(1));
    let elapsed = started.elapsed();
    assert_eq!(code, -2, "a killed child is -2, never a real exit code");
    assert!(elapsed < Duration::from_secs(3), "returned at the cap, not when the grandchild finished: {elapsed:?}");
}

/// F4 (plan 1 Task 12, minor): a slot that fails before the engine writes its `start` record is
/// never settled, so the tick would otherwise re-run it every 60 s until midnight. The backoff
/// decides only HOW OFTEN a failed slot is retried — the slot itself stays owed either way.
#[test]
fn retry_backoff_is_5_15_30_then_hourly_and_never_forever() {
    use std::time::Duration as D;
    assert!(should_retry(0, D::ZERO), "a slot never attempted is due now");
    assert!(!should_retry(1, D::from_secs(4 * 60)) && should_retry(1, D::from_secs(5 * 60)));
    assert!(!should_retry(2, D::from_secs(14 * 60)) && should_retry(2, D::from_secs(15 * 60)));
    assert!(!should_retry(3, D::from_secs(29 * 60)) && should_retry(3, D::from_secs(30 * 60)));
    assert!(!should_retry(9, D::from_secs(59 * 60)) && should_retry(9, D::from_secs(60 * 60)), "hourly forever, never giving up");
}

/// Four slot logs a day (one per step, two slots), kept forever, is an unbounded directory in the
/// user's app data. Housekeeping prunes to the newest `keep` — oldest by modified time first, and
/// only regular `slot-` files directly in that one directory. The tray's `quit-<ts>.txt` shares the
/// directory: counting it toward the cap would silently shorten the slot history, and removing it
/// would delete the one trace a quit that timed out leaves behind.
#[test]
fn log_retention_keeps_the_newest_slot_logs_and_leaves_everything_else_alone() {
    let dir = std::env::temp_dir().join(format!("qo-console-sched-logs-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();
    for i in 0..35 {
        std::fs::write(dir.join(format!("slot-{i:03}.log")), b"x").unwrap();
    }
    let quit = dir.join("quit-2026-09-05T120000.000Z.txt");
    std::fs::write(&quit, b"synced=true backed_up=true timed_out=false\n").unwrap();
    assert_eq!(std::fs::read_dir(&dir).unwrap().count(), 36, "thirty-five distinct slot logs and one quit record");

    let removed = prune_logs(&dir, 30);
    assert_eq!(removed, 5, "only the five oldest SLOT logs — the quit record was never a candidate");
    assert_eq!(std::fs::read_dir(&dir).unwrap().count(), 31, "thirty slot logs plus the quit record");
    assert!(quit.is_file(), "the quit record survives: it is not a slot log, so it is neither counted nor removed");
    assert_eq!(prune_logs(&dir, 30), 0, "a directory already at the cap is left alone");
    let _ = std::fs::remove_dir_all(&dir);
}

/// F10 (console spec §8 push on close, Knowlu spec §4 back up on quit): the Quit arm flushes
/// synchronously under a 10 s cap. This scratch vault has no account, so there is nothing to push —
/// `synced: false`, and that is not an error — and a backup folder is set, so the mirror is
/// written. `then` runs on the caller's thread before the scope joins; in production it writes the
/// quit log and ends the process, which is why a worker still stuck in a slow push can never hold
/// the click past the cap.
#[test]
fn quit_flush_backs_up_and_reports_within_the_cap() {
    let v = scratch("quitflush");
    let cs = open(&v, "quitflush");
    let bdir = std::env::temp_dir().join(format!("qo-console-quit-backup-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&bdir);
    { let mut s = cs.settings.lock().unwrap(); s.backup_dir = Some(bdir.clone()); }
    let started = Instant::now();
    let q = quit_flush(&cs, Duration::from_secs(10), |_| {});
    assert!(started.elapsed() < Duration::from_secs(10), "inside the cap: {:?}", started.elapsed());
    assert!(!q.timed_out && q.backed_up && !q.synced, "{q:?}");
    let profile = cs.settings.lock().unwrap().profile_id.clone();
    assert!(bdir.join(&profile).join("vault").join("tasks").is_dir(), "the mirror exists");
    let _ = std::fs::remove_dir_all(&bdir);
}

/// Fix round 1, review M4: a configured-but-signed-out vault is a named SKIP inside
/// `run_lines_with` (`totals.errors` empty, `totals.skipped` set) — before this fix `q.synced` read
/// only `errors.is_empty()`, so a quit that pushed nothing at all still reported `synced: true`.
/// `QuitFlush`'s own contract is "this completed, never this was attempted".
///
/// **Reads Credential Manager, and the exemption is the same one `sync_contract.rs`'s own
/// `a_vault_with_an_account_and_no_session_says_exactly_that` already records**: this only READS a
/// target (`knowlu/c3-fix1-no-such-profile/session`) that nothing in this product ever writes, so
/// there is no `CredWriteW`/`CredReadW` race on the SAME target to serialise against — the file's
/// own `CREDMAN_LOCK` is for tests that WRITE a credential, which this one does not.
#[test]
fn m4_quit_flush_reports_a_skip_as_a_skip_never_synced() {
    let v = scratch("quitflush-skip");
    std::fs::write(
        v.join("config").join("cloud.yaml"),
        "api_base: 'http://127.0.0.1:9/functions/v1'\nanon_key: 'anon'\nsession_credential_target: 'knowlu/c3-fix1-no-such-profile/session'\naccount_id: 'acct-1'\n",
    )
    .unwrap();
    let cs = open(&v, "quitflush-skip");
    let q = quit_flush(&cs, Duration::from_secs(10), |_| {});
    assert!(!q.synced, "a signed-out quit must not report synced: true: {q:?}");
    let _ = std::fs::remove_dir_all(&v);
}

/// Fix round 1, review I4: the slot's own `sync` step is a CHILD PROCESS — `run_slot_inner` records
/// only its exit code, and the lines it printed went to a log file, never to `cs.sync` — so the
/// only way the page ever sees what a slot's sync did is by reading back the status file that
/// child would have written. The `cmd` stand-in engine below writes no such file (it does not know
/// sync exists), so this test writes one itself, exactly as a real engine's `sync` subcommand
/// would, and checks that `run_slot_inner`'s own `state::refresh_sync` call is what moves it into
/// `cs.sync`, not `ConsoleState::open` (which ran first, before the file existed).
#[test]
fn a_slot_run_leaves_cs_sync_filled_from_the_status_file() {
    let v = scratch("sync-status-fill");
    std::fs::write(
        v.join("config").join("runners.yaml"),
        format!("runners:\n  - name: local\n    times: [\"12:00\"]\n    tz: America/Chicago\n    grace_minutes: 20\n    device: {}\n    scheduler: app\n", knowlu_engine::journal::device_name()),
    ).unwrap();
    let cs = open(&v, "sync-status-fill");
    assert_eq!(
        *cs.sync.lock().unwrap(),
        knowlu_engine::sync::SyncStatus::default(),
        "no status file exists yet, so `open` left the ordinary default"
    );

    let status = knowlu_engine::sync::SyncStatus {
        ok: true,
        at: Some("2026-09-23T07:00:00.000Z".to_string()),
        lines: vec!["sync: 2 record(s) and 0 note(s) down; 2 applied, 0 card(s), 0 refused".to_string()],
        last_error: None,
        skipped: None,
    };
    let status_path = v.join(knowlu_engine::sync::STATUS_FILE);
    std::fs::create_dir_all(status_path.parent().unwrap()).unwrap();
    std::fs::write(&status_path, knowlu_engine::ledger::dumps_value(&serde_json::to_value(&status).unwrap())).unwrap();

    let sch = Scheduler::default();
    let fake_local_appdata = std::env::temp_dir().join(format!("qo-console-sched-localappdata-syncfill-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&fake_local_appdata);
    std::fs::create_dir_all(&fake_local_appdata).unwrap();
    let _guard = ENGINE_ENV_LOCK.lock().unwrap_or_else(|e| e.into_inner());
    let _env = EnvSeam::set(&[
        ("KNOWLU_ENGINE_EXE", std::ffi::OsStr::new("cmd")),
        ("LOCALAPPDATA", fake_local_appdata.as_os_str()),
    ]);
    let _summary = run_slot_inner(&cs, &sch, None, false);
    assert_eq!(
        *cs.sync.lock().unwrap(),
        status,
        "run_slot_inner's own refresh_sync should have picked up the file the child would have written"
    );
    let _ = std::fs::remove_dir_all(&fake_local_appdata);
    let _ = std::fs::remove_dir_all(&v);
}

#[test]
#[ignore = "runs the real engine exe against a scratch vault; needs a built target/release/knowlu-engine.exe and network for rank's feeds — run by hand before Task 17"]
fn run_slot_end_to_end() { unimplemented!("see the ignore reason") }

use knowlu::scheduler::needs_first_run;

/// Spec §4.2 step 7, and Quinn's cut-day note (2026-09-09): a vault that has never been ranked
/// runs its first slot at launch, whatever the clock says — otherwise the first page a new user
/// sees is empty until the next scheduled slot. `today.md` is rewritten by every `rank`, so its
/// absence is the whole test; once it exists, launch owes nothing until the next slot.
#[test]
fn a_vault_that_has_never_been_ranked_is_owed_its_first_run_at_launch() {
    let v = scratch("first-run");
    std::fs::create_dir_all(v.join("state")).unwrap();
    assert!(needs_first_run(&v), "no state/today.md yet: the first slot is owed at launch");
    // The engine writes `state/today.md` (cli.rs), never a root `today.md`: a root file must not
    // count, or the predicate is permanently true and every launch runs a slot.
    std::fs::write(v.join("today.md"), "# not the engine's file\n").unwrap();
    assert!(needs_first_run(&v), "a root today.md is not the engine's page");
    std::fs::write(v.join("state").join("today.md"), "# Today\n").unwrap();
    assert!(!needs_first_run(&v), "once ranked, launch owes nothing until the next slot");
    let _ = std::fs::remove_dir_all(&v);
}

/// Spec §5.1: past the grace the slots keep ranking and the cloud steps are skipped **as named
/// steps**, never as failures. The shape is `ingest (skipped: no ics_url)`'s, deliberately — the two
/// sit side by side in the Runs view and a reader should not have to learn two conventions.
#[test]
fn a_vault_with_an_account_and_no_entitlement_records_the_judge_skip_and_stays_green() {
    // D1: this vault has an account and no cache, so the slot now attempts one refresh, which
    // reaches `valid_access_token_at` and the real Credential Manager (CLAUDE.md).
    let _credman_guard = CREDMAN_LOCK.lock().unwrap_or_else(|e| e.into_inner());
    let v = scratch("noentitlement");
    std::fs::write(
        v.join("config").join("runners.yaml"),
        format!("runners:\n  - name: local\n    times: [\"12:00\"]\n    tz: America/Chicago\n    grace_minutes: 20\n    device: {}\n    scheduler: app\n", knowlu_engine::journal::device_name()),
    ).unwrap();
    // A vault that HAS an account: the four keys the C2 contract fixes. The host is the closed
    // loopback shape `an_offline_telemetry_send…` already uses (M7) — nothing is listening on it —
    // rather than a real host `cloud_config`'s R-C1-59 I1 check would now refuse outright; the
    // `KNOWLU_API_BASE` seam below points this build's own `api_base()` at the same value so the two
    // agree, exactly as a real vault's does by construction.
    std::fs::write(
        v.join("config").join("cloud.yaml"),
        "api_base: 'http://127.0.0.1:9/functions/v1'\nanon_key: 'anon'\nsession_credential_target: 'knowlu/profile_x/session'\naccount_id: 'acc-1'\n",
    ).unwrap();
    let cs = open(&v, "noentitlement");
    // …and no entitlement cache at all, which is a fresh install that has not reached the cloud yet.
    let sch = Scheduler::default();
    let fake = std::env::temp_dir().join(format!("qo-sched-ent-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&fake);
    std::fs::create_dir_all(&fake).unwrap();
    let _guard = ENGINE_ENV_LOCK.lock().unwrap_or_else(|e| e.into_inner());
    let _env = EnvSeam::set(&[
        ("KNOWLU_ENGINE_EXE", std::ffi::OsStr::new("cmd")),
        ("LOCALAPPDATA", fake.as_os_str()),
        ("KNOWLU_API_BASE", std::ffi::OsStr::new("http://127.0.0.1:9/functions/v1")),
    ]);
    let s = run_slot_inner(&cs, &sch, None, false);
    let named: Vec<String> = s.steps.iter().map(|(n, _)| n.clone()).collect();
    assert!(named.contains(&"judge (skipped: no entitlement)".to_string()), "{named:?}");
    assert!(!named.iter().any(|n| n == "judge"), "the step itself never ran");
    assert_eq!(s.steps.iter().find(|(n, _)| n.starts_with("judge (skipped")).unwrap().1, 0, "a skip is not a failure");
    assert!(s.engine_ok, "a skipped judge step must not paint the tray amber: {:?}", s.steps);
    let _ = std::fs::remove_dir_all(&fake);
    let _ = std::fs::remove_dir_all(&v);
}

/// …and a vault with no cloud config at all is `NoAccount`, which is what sends it down the local
/// runtime path rather than to the cloud — case (b), and the reason the two pre-C1 tests still pass.
#[test]
fn a_vault_with_no_cloud_config_says_no_account_rather_than_no_entitlement() {
    use knowlu::account::EntitlementState;
    let v = scratch("noaccount");
    let cs = open(&v, "noaccount");
    assert_eq!(entitlement_state(&cs), EntitlementState::NoAccount);
    let _ = std::fs::remove_dir_all(&v);
}

/// Case (a), and the one C2 depends on: **an entitled vault runs `judge`, with nothing but the vault
/// on the command line.** A runtime-based skip here would leave the whole judgment service inert on
/// every machine that never installed llama.cpp — which, after C4, is every machine.
#[test]
fn an_entitled_vault_runs_judge_with_no_runtime_and_no_account_on_the_command_line() {
    use knowlu::account::{save_cache, EntitlementCache};
    let v = scratch("cloudjudge");
    // The closed-loopback host (M7), with `KNOWLU_API_BASE` pointed at the same value so
    // `cloud_config`'s R-C1-59 I1 host check accepts it — nothing here reaches a socket either way.
    std::fs::write(
        v.join("config").join("cloud.yaml"),
        "api_base: 'http://127.0.0.1:9/functions/v1'\nanon_key: 'anon'\nsession_credential_target: 'knowlu/profile_x/session'\naccount_id: 'acc-1'\n",
    ).unwrap();
    let _guard = ENGINE_ENV_LOCK.lock().unwrap_or_else(|e| e.into_inner());
    let _env = EnvSeam::set(&[("KNOWLU_API_BASE", std::ffi::OsStr::new("http://127.0.0.1:9/functions/v1"))]);
    let cs = open(&v, "cloudjudge");
    save_cache(&cs.data_dir, &EntitlementCache {
        status: "active".into(),
        current_period_end: None,
        plan: Some("monthly".into()),
        checked_at: knowlu_engine::journal::now_ts(None),
    }).unwrap();
    let plan = judge_plan(&cs);
    let JudgePlan::Cloud { log_dir } = plan.clone() else { panic!("not the cloud plan: {plan:?}") };
    // The profile's own judgments folder, never the vault: judgment logs never enter a vault.
    assert!(log_dir.starts_with(&cs.data_dir), "{log_dir:?} is not under {:?}", cs.data_dir);
    let argv = slot_argv(&v, Path::new(r"C:\bin\knowlu-engine.exe"), &plan);
    let judge = argv.iter().find(|(_, a)| a[0] == "judge").expect("the judge step is in the argv");
    assert_eq!(
        judge.1,
        vec!["judge", "--vault", v.to_string_lossy().as_ref(), "--via", "local-runner", "--log-dir", log_dir.to_string_lossy().as_ref()]
    );
    // Nothing about the account is on a command line — a process list is not a place for one.
    for (_, args) in &argv {
        assert!(!args.iter().any(|a| a.contains("acc-1") || a == "--runtime" || a == "--model"), "{args:?}");
    }
    let _ = std::fs::remove_dir_all(&v);
}

/// F2 (task-11-review.md, round 1): `cloud_config`'s own host check compares scheme and host only,
/// never the path, so a `cloud.yaml` whose `api_base` names the right host but the wrong path (no
/// trailing `/functions/v1`) passes `cloud_config` and only then fails `auth_base` — reachable only
/// from a hand-edited or corrupted file, never from anything the wizard writes, but the session
/// pre-flight must name this failure exactly like a refused refresh rather than sit silent while the
/// entitlement block right below (gated the same way, and hitting the identical broken value) visibly
/// reports its own.
#[test]
fn a_malformed_api_base_names_the_session_step_instead_of_silence() {
    let v = scratch("sessionbadbase");
    std::fs::write(
        v.join("config").join("runners.yaml"),
        format!("runners:\n  - name: local\n    times: [\"12:00\"]\n    tz: America/Chicago\n    grace_minutes: 20\n    device: {}\n    scheduler: app\n", knowlu_engine::journal::device_name()),
    ).unwrap();
    // The host matches this build's own `api_base()` (set below via `KNOWLU_API_BASE`), so
    // `cloud_config`'s host check accepts it; the path does not end in `/functions/v1`, so only
    // `auth_base` refuses it.
    std::fs::write(
        v.join("config").join("cloud.yaml"),
        "api_base: 'http://127.0.0.1:9/not-functions'\nanon_key: 'anon'\nsession_credential_target: 'knowlu/profile_x/session'\naccount_id: 'acc-1'\n",
    ).unwrap();
    let cs = open(&v, "sessionbadbase");
    let sch = Scheduler::default();
    let fake = std::env::temp_dir().join(format!("qo-sched-sessionbadbase-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&fake);
    std::fs::create_dir_all(&fake).unwrap();
    let _guard = ENGINE_ENV_LOCK.lock().unwrap_or_else(|e| e.into_inner());
    let _env = EnvSeam::set(&[
        ("KNOWLU_ENGINE_EXE", std::ffi::OsStr::new("cmd")),
        ("LOCALAPPDATA", fake.as_os_str()),
        ("KNOWLU_API_BASE", std::ffi::OsStr::new("http://127.0.0.1:9/not-functions")),
    ]);
    let s = run_slot_inner(&cs, &sch, None, false);
    let named: Vec<String> = s.steps.iter().map(|(n, _)| n.clone()).collect();
    let session_at = named.iter().position(|n| n.starts_with("session (refresh failed:"))
        .unwrap_or_else(|| panic!("no session step: {named:?}"));
    let coursework_at = named.iter().position(|n| n == "coursework")
        .unwrap_or_else(|| panic!("no coursework step: {named:?}"));
    assert!(session_at < coursework_at, "the session failure is still named before any engine step: {named:?}");
    assert_eq!(s.steps[session_at].1, 0, "a malformed api_base is not a failed slot");
    assert!(named[session_at].contains("functions/v1"), "the auth_base error names what is wrong: {}", named[session_at]);
    assert!(s.engine_ok, "a named session failure must never paint the tray amber: {:?}", s.steps);
    let _ = std::fs::remove_dir_all(&fake);
    let _ = std::fs::remove_dir_all(&v);
}

/// R-C1c-13: the slot refreshes the account session before its first cloud step — the entitlement
/// refresh right after it is itself one such step, and every child process after that (sync,
/// coursework, ingest, judge) authenticates with the same session. Since the C3′ merge this also
/// pins that the refresh precedes the `sync` child, the first to spend the token. A one-hour token is routinely down to single
/// digits by a noon or 6pm slot, and the app's only other refresh point (`valid_access_token_at`'s own
/// 120-second margin) is reached only at the slot's END, by telemetry — too late for anything earlier
/// in the same slot. A session with ten minutes left, under this pre-flight's 45-minute floor, and an
/// `api_base` that answers nothing (port 9, the closed host this file's other tests already use) must
/// still leave the slot green: a network refusing the refresh is not a failed slot, and the failure is
/// named and sanitized exactly as the entitlement step's own is (R-C1c-final-3).
#[test]
fn a_near_expiry_session_is_refreshed_before_any_engine_step_and_a_failure_is_named() {
    let _credman_guard = CREDMAN_LOCK.lock().unwrap_or_else(|e| e.into_inner());
    let v = scratch("sessionrefresh");
    std::fs::write(
        v.join("config").join("runners.yaml"),
        format!("runners:\n  - name: local\n    times: [\"12:00\"]\n    tz: America/Chicago\n    grace_minutes: 20\n    device: {}\n    scheduler: app\n", knowlu_engine::journal::device_name()),
    ).unwrap();
    let target = format!("knowlu/test-sessionrefresh-{}-{}", std::process::id(), line!());
    std::fs::write(
        v.join("config").join("cloud.yaml"),
        format!("api_base: 'http://127.0.0.1:9/functions/v1'\nanon_key: 'anon'\nsession_credential_target: '{target}'\naccount_id: 'acc-1'\n"),
    ).unwrap();
    let _cred_cleanup = CredCleanup(target.clone());
    knowlu::account::save_session(&target, "acc-1", &knowlu::account::Session {
        access_token: "stale-at".into(),
        refresh_token: "stale-rt".into(),
        // Ten minutes left — under the 45-minute floor the scheduler's pre-flight asks for, so a
        // refresh is attempted (and, against the closed port below, fails).
        expires_at: jiff::Timestamp::now().as_second() + 600,
        email: "a@example.invalid".into(),
    }).unwrap();
    let cs = open(&v, "sessionrefresh");
    let sch = Scheduler::default();
    let fake = std::env::temp_dir().join(format!("qo-sched-sessionrefresh-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&fake);
    std::fs::create_dir_all(&fake).unwrap();
    let _guard = ENGINE_ENV_LOCK.lock().unwrap_or_else(|e| e.into_inner());
    let _env = EnvSeam::set(&[
        ("KNOWLU_ENGINE_EXE", std::ffi::OsStr::new("cmd")),
        ("LOCALAPPDATA", fake.as_os_str()),
        // `cloud_config`'s R-C1-59 I1 host check needs this build's own `api_base()` to name the
        // same port-9 host the vault's `cloud.yaml` does, above.
        ("KNOWLU_API_BASE", std::ffi::OsStr::new("http://127.0.0.1:9/functions/v1")),
    ]);
    let s = run_slot_inner(&cs, &sch, None, false);
    let named: Vec<String> = s.steps.iter().map(|(n, _)| n.clone()).collect();
    let session_at = named.iter().position(|n| n.starts_with("session (refresh failed:"))
        .unwrap_or_else(|| panic!("no session step: {named:?}"));
    let coursework_at = named.iter().position(|n| n == "coursework")
        .unwrap_or_else(|| panic!("no coursework step: {named:?}"));
    assert!(session_at < coursework_at, "the session is refreshed before any engine step: {named:?}");
    // The C3′ merge: `slot_argv` puts `sync` first, ahead of coursework, and the engine's `sync`
    // spends this same token on `/sync-pull` and `/sync-push` — so the one session refresh a slot
    // makes has to come before `sync` too, or a second-day slot syncs on a stale token and reads
    // "signed out". This vault's slot runs `sync` (it is unconditional in `slot_argv`).
    let sync_at = named.iter().position(|n| n == "sync")
        .unwrap_or_else(|| panic!("no sync step: {named:?}"));
    assert!(
        session_at < sync_at,
        "the session is refreshed before the sync step, because the engine's sync spends the token: {named:?}"
    );
    assert!(sync_at < coursework_at, "sync is the slot's first engine step: {named:?}");
    assert_eq!(
        named.iter().filter(|n| n.starts_with("session")).count(),
        1,
        "exactly one session refresh per slot — never one per step: {named:?}"
    );
    assert_eq!(s.steps[session_at].1, 0, "a network refusing the refresh is not a failed slot");
    assert!(s.engine_ok, "a failed session refresh must never paint the tray amber: {:?}", s.steps);
    // R-C1c-final-3: the runner log gets the sanitized line, never the service's failure reason.
    let log = knowlu_engine::pystr::read_text(&v.join("state").join("runner-log.md")).unwrap();
    assert!(log.contains("local ok session (refresh failed)"), "{log}");
    assert!(!log.contains("refresh failed:"), "a service error's text must never reach the vault: {log}");
    let _ = std::fs::remove_dir_all(&fake);
    let _ = std::fs::remove_dir_all(&v);
}

/// Case (c): entitled once, but not for four days. The grace is over, the cloud step stands down as a
/// named step, and the day is still ranked.
///
/// Fix round 1, item 4: driven through `run_slot_inner`, exactly as its `…no_entitlement…` sibling
/// above, so the name is true of the whole slot — not just of `judge_plan` in isolation.
///
/// R-C1c-11: this cache is now past-grace, so `run_slot_inner` attempts its own refresh before the
/// judge decision (`a_stale_entitlement_is_refreshed_inside_the_slot_before_it_decides` proves the
/// ordering) — which reaches `valid_access_token_at` and the real Credential Manager, so this test
/// takes this file's lock too, as CLAUDE.md requires of every test that touches the store.
#[test]
fn an_entitlement_past_the_grace_skips_judge_by_name_and_keeps_the_slot_green() {
    let _credman_guard = CREDMAN_LOCK.lock().unwrap_or_else(|e| e.into_inner());
    use knowlu::account::{save_cache, EntitlementCache};
    let v = scratch("pastgrace");
    std::fs::write(
        v.join("config").join("runners.yaml"),
        format!("runners:\n  - name: local\n    times: [\"12:00\"]\n    tz: America/Chicago\n    grace_minutes: 20\n    device: {}\n    scheduler: app\n", knowlu_engine::journal::device_name()),
    ).unwrap();
    // The closed-loopback host (M7), with `KNOWLU_API_BASE` pointed at the same value below so
    // `cloud_config`'s R-C1-59 I1 host check accepts it — nothing here reaches a socket either way.
    std::fs::write(
        v.join("config").join("cloud.yaml"),
        "api_base: 'http://127.0.0.1:9/functions/v1'\nanon_key: 'anon'\nsession_credential_target: 'knowlu/profile_x/session'\naccount_id: 'acc-1'\n",
    ).unwrap();
    let cs = open(&v, "pastgrace");
    let four_days_ago = (jiff::Timestamp::now() - jiff::SignedDuration::from_hours(96)).to_string();
    save_cache(&cs.data_dir, &EntitlementCache {
        status: "active".into(),
        current_period_end: None,
        plan: Some("monthly".into()),
        checked_at: four_days_ago,
    }).unwrap();

    let sch = Scheduler::default();
    let fake = std::env::temp_dir().join(format!("qo-sched-pastgrace-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&fake);
    std::fs::create_dir_all(&fake).unwrap();
    let _guard = ENGINE_ENV_LOCK.lock().unwrap_or_else(|e| e.into_inner());
    let _env = EnvSeam::set(&[
        ("KNOWLU_ENGINE_EXE", std::ffi::OsStr::new("cmd")),
        ("LOCALAPPDATA", fake.as_os_str()),
        ("KNOWLU_API_BASE", std::ffi::OsStr::new("http://127.0.0.1:9/functions/v1")),
    ]);
    assert_eq!(judge_plan(&cs), JudgePlan::Skip("judge (skipped: no entitlement)"));
    let s = run_slot_inner(&cs, &sch, None, false);
    let named: Vec<String> = s.steps.iter().map(|(n, _)| n.clone()).collect();
    assert!(named.contains(&"judge (skipped: no entitlement)".to_string()), "{named:?}");
    assert!(!named.iter().any(|n| n == "judge"), "the step itself never ran");
    assert!(s.engine_ok, "a skipped judge step must not paint the tray amber: {:?}", s.steps);
    let _ = std::fs::remove_dir_all(&fake);
    let _ = std::fs::remove_dir_all(&v);
}

/// §2 / D1: the first slot of a brand-new cloud vault used to lose a race. `scheduler::spawn`
/// starts the first slot and the launch entitlement refresh on two threads, and on the first live
/// onboarding the slot reached the judge decision before the refresh had written its cache — so the
/// judge was skipped for want of an answer that was already on its way. A vault that has an account
/// and has never cached an entitlement now refreshes it INSIDE the slot, before the decision.
#[test]
fn a_first_slot_refreshes_the_entitlement_before_it_decides_about_judge() {
    // The refresh reaches `valid_access_token_at`, which reads the real Credential Manager
    // (CLAUDE.md: every test that touches the store takes this file's lock).
    let _credman_guard = CREDMAN_LOCK.lock().unwrap_or_else(|e| e.into_inner());
    let v = scratch("entrefresh");
    std::fs::write(
        v.join("config").join("runners.yaml"),
        format!("runners:\n  - name: local\n    times: [\"12:00\"]\n    tz: America/Chicago\n    grace_minutes: 20\n    device: {}\n    scheduler: app\n", knowlu_engine::journal::device_name()),
    ).unwrap();
    std::fs::write(
        v.join("config").join("cloud.yaml"),
        "api_base: 'http://127.0.0.1:9/functions/v1'\nanon_key: 'anon'\nsession_credential_target: 'knowlu/profile_x/session'\naccount_id: 'acc-1'\n",
    ).unwrap();
    let cs = open(&v, "entrefresh");
    assert!(!knowlu::account::cache_path(&cs.data_dir).exists(), "nothing has been cached yet");
    let sch = Scheduler::default();
    let fake = std::env::temp_dir().join(format!("qo-sched-entrefresh-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&fake);
    std::fs::create_dir_all(&fake).unwrap();
    let _guard = ENGINE_ENV_LOCK.lock().unwrap_or_else(|e| e.into_inner());
    let _env = EnvSeam::set(&[
        ("KNOWLU_ENGINE_EXE", std::ffi::OsStr::new("cmd")),
        ("LOCALAPPDATA", fake.as_os_str()),
        ("KNOWLU_API_BASE", std::ffi::OsStr::new("http://127.0.0.1:9/functions/v1")),
    ]);
    let s = run_slot_inner(&cs, &sch, None, false);
    let named: Vec<String> = s.steps.iter().map(|(n, _)| n.clone()).collect();
    let refresh = named.iter().position(|n| n.starts_with("entitlement (refresh failed:"))
        .unwrap_or_else(|| panic!("no entitlement step: {named:?}"));
    let judge = named.iter().position(|n| n == "judge (skipped: no entitlement)")
        .unwrap_or_else(|| panic!("no judge skip: {named:?}"));
    assert!(refresh < judge, "the refresh is attempted BEFORE the decision: {named:?}");
    assert_eq!(s.steps[refresh].1, 0, "a service that could not be reached is not a failed slot");
    assert!(s.engine_ok, "an entitlement refresh must never paint the tray amber: {:?}", s.steps);
    // D8: and the skip reaches the file a student can open, in the engine's own format — with
    // status `ok`, because a skip is routine (Ruling R-C1c-plan-4).
    let log = knowlu_engine::pystr::read_text(&v.join("state").join("runner-log.md")).unwrap();
    assert!(log.contains("local ok judge (skipped: no entitlement)"), "{log}");
    assert!(!log.contains("local skip"), "a skip must not be a non-ok line: {log}");
    // Final review, I3: the entitlement step reaches `state/runner-log.md` too, as an `ok` line —
    // but never with the service's failure reason. `s.steps` (asserted above via `refresh`) keeps
    // the full `entitlement (refresh failed: <reason>)` for the in-memory `RunSummary`; the file a
    // student can open gets the bare sentence only.
    assert!(log.contains("local ok entitlement (refresh failed)"), "{log}");
    assert!(!log.contains("refresh failed:"), "a service error's text must never reach the vault: {log}");
    let _ = std::fs::remove_dir_all(&fake);
    let _ = std::fs::remove_dir_all(&v);
}

/// R-C1c-11: the same race Task 2 fixed for a cache that was MISSING also applies to one that
/// EXISTS but is past its 72-hour grace. `scheduler::spawn` starts the first slot and the launch
/// refresh on two threads; a student who reopens the laptop after a long weekend used to get a
/// first slot with every cloud step skipped (`judge (skipped: no entitlement)`) while the launch
/// refresh — and the six-hourly housekeeping refresh behind it — was still hours away. This proves
/// the slot itself asks once, synchronously, before it decides, exactly as it already does for a
/// cache that has never been written at all.
#[test]
fn a_stale_entitlement_is_refreshed_inside_the_slot_before_it_decides() {
    // The refresh reaches `valid_access_token_at`, which reads the real Credential Manager
    // (CLAUDE.md: every test that touches the store takes this file's lock).
    let _credman_guard = CREDMAN_LOCK.lock().unwrap_or_else(|e| e.into_inner());
    let v = scratch("entstale");
    std::fs::write(
        v.join("config").join("runners.yaml"),
        format!("runners:\n  - name: local\n    times: [\"12:00\"]\n    tz: America/Chicago\n    grace_minutes: 20\n    device: {}\n    scheduler: app\n", knowlu_engine::journal::device_name()),
    ).unwrap();
    std::fs::write(
        v.join("config").join("cloud.yaml"),
        "api_base: 'http://127.0.0.1:9/functions/v1'\nanon_key: 'anon'\nsession_credential_target: 'knowlu/profile_x/session'\naccount_id: 'acc-1'\n",
    ).unwrap();
    let cs = open(&v, "entstale");
    // Well past the grace, computed from the clock at run time — never a hard-coded date.
    let stale = (jiff::Timestamp::now() - jiff::SignedDuration::from_hours(100)).to_string();
    knowlu::account::save_cache(&cs.data_dir, &knowlu::account::EntitlementCache {
        status: "active".into(), current_period_end: None, plan: Some("monthly".into()), checked_at: stale,
    }).unwrap();
    let sch = Scheduler::default();
    let fake = std::env::temp_dir().join(format!("qo-sched-entstale-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&fake);
    std::fs::create_dir_all(&fake).unwrap();
    let _guard = ENGINE_ENV_LOCK.lock().unwrap_or_else(|e| e.into_inner());
    let _env = EnvSeam::set(&[
        ("KNOWLU_ENGINE_EXE", std::ffi::OsStr::new("cmd")),
        ("LOCALAPPDATA", fake.as_os_str()),
        ("KNOWLU_API_BASE", std::ffi::OsStr::new("http://127.0.0.1:9/functions/v1")),
    ]);
    let s = run_slot_inner(&cs, &sch, None, false);
    let named: Vec<String> = s.steps.iter().map(|(n, _)| n.clone()).collect();
    let refresh = named.iter().position(|n| n.starts_with("entitlement (refresh failed:"))
        .unwrap_or_else(|| panic!("no entitlement step: {named:?}"));
    let judge = named.iter().position(|n| n == "judge (skipped: no entitlement)")
        .unwrap_or_else(|| panic!("no judge skip: {named:?}"));
    assert!(refresh < judge, "the refresh is attempted BEFORE the decision: {named:?}");
    assert_eq!(s.steps[refresh].1, 0, "a service that could not be reached is not a failed slot");
    assert!(s.engine_ok, "an entitlement refresh must never paint the tray amber: {:?}", s.steps);
    // R-C1c-final-3: the runner log gets the sanitized line, never the service's failure reason.
    let log = knowlu_engine::pystr::read_text(&v.join("state").join("runner-log.md")).unwrap();
    assert!(log.contains("local ok entitlement (refresh failed)"), "{log}");
    assert!(!log.contains("refresh failed:"), "a service error's text must never reach the vault: {log}");
    let _ = std::fs::remove_dir_all(&fake);
    let _ = std::fs::remove_dir_all(&v);
}

/// …and the same is true of a cache that is fresh but no longer `active`/`trialing` — cancelled,
/// for instance. `checked_at` being recent does not matter: `account::decide` reads the status
/// first, and a lapsed status is asked again exactly as a stale one is.
#[test]
fn a_cancelled_entitlement_is_asked_again_inside_the_slot() {
    let _credman_guard = CREDMAN_LOCK.lock().unwrap_or_else(|e| e.into_inner());
    let v = scratch("entcancelled");
    std::fs::write(
        v.join("config").join("runners.yaml"),
        format!("runners:\n  - name: local\n    times: [\"12:00\"]\n    tz: America/Chicago\n    grace_minutes: 20\n    device: {}\n    scheduler: app\n", knowlu_engine::journal::device_name()),
    ).unwrap();
    std::fs::write(
        v.join("config").join("cloud.yaml"),
        "api_base: 'http://127.0.0.1:9/functions/v1'\nanon_key: 'anon'\nsession_credential_target: 'knowlu/profile_x/session'\naccount_id: 'acc-1'\n",
    ).unwrap();
    let cs = open(&v, "entcancelled");
    knowlu::account::save_cache(&cs.data_dir, &knowlu::account::EntitlementCache {
        status: "canceled".into(), current_period_end: None, plan: Some("monthly".into()),
        checked_at: knowlu_engine::journal::now_ts(None),
    }).unwrap();
    let sch = Scheduler::default();
    let fake = std::env::temp_dir().join(format!("qo-sched-entcancelled-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&fake);
    std::fs::create_dir_all(&fake).unwrap();
    let _guard = ENGINE_ENV_LOCK.lock().unwrap_or_else(|e| e.into_inner());
    let _env = EnvSeam::set(&[
        ("KNOWLU_ENGINE_EXE", std::ffi::OsStr::new("cmd")),
        ("LOCALAPPDATA", fake.as_os_str()),
        ("KNOWLU_API_BASE", std::ffi::OsStr::new("http://127.0.0.1:9/functions/v1")),
    ]);
    let s = run_slot_inner(&cs, &sch, None, false);
    let named: Vec<String> = s.steps.iter().map(|(n, _)| n.clone()).collect();
    let refresh = named.iter().position(|n| n.starts_with("entitlement (refresh failed:"))
        .unwrap_or_else(|| panic!("no entitlement step: {named:?}"));
    let judge = named.iter().position(|n| n == "judge (skipped: no entitlement)")
        .unwrap_or_else(|| panic!("no judge skip: {named:?}"));
    assert!(refresh < judge, "the refresh is attempted BEFORE the decision: {named:?}");
    assert_eq!(s.steps[refresh].1, 0, "a service that could not be reached is not a failed slot");
    assert!(s.engine_ok, "an entitlement refresh must never paint the tray amber: {:?}", s.steps);
    let _ = std::fs::remove_dir_all(&fake);
    let _ = std::fs::remove_dir_all(&v);
}

/// …and a cache that is FRESH and `active` — `Entitled` — is still never refreshed here: the
/// six-hourly housekeeping refresh alone owns re-asking a question that already has a good answer.
/// R-C1c-11 changes every OTHER cache shape (missing, stale, or no longer active/trialing); this is
/// the one case that must not.
#[test]
fn a_fresh_active_entitlement_is_never_refreshed_inside_the_slot() {
    let _credman_guard = CREDMAN_LOCK.lock().unwrap_or_else(|e| e.into_inner());
    let v = scratch("entcached");
    std::fs::write(
        v.join("config").join("runners.yaml"),
        format!("runners:\n  - name: local\n    times: [\"12:00\"]\n    tz: America/Chicago\n    grace_minutes: 20\n    device: {}\n    scheduler: app\n", knowlu_engine::journal::device_name()),
    ).unwrap();
    std::fs::write(
        v.join("config").join("cloud.yaml"),
        "api_base: 'http://127.0.0.1:9/functions/v1'\nanon_key: 'anon'\nsession_credential_target: 'knowlu/profile_x/session'\naccount_id: 'acc-1'\n",
    ).unwrap();
    let cs = open(&v, "entcached");
    knowlu::account::save_cache(&cs.data_dir, &knowlu::account::EntitlementCache {
        status: "active".into(), current_period_end: None, plan: Some("monthly".into()),
        checked_at: knowlu_engine::journal::now_ts(None),
    }).unwrap();
    let sch = Scheduler::default();
    let fake = std::env::temp_dir().join(format!("qo-sched-entcached-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&fake);
    std::fs::create_dir_all(&fake).unwrap();
    let _guard = ENGINE_ENV_LOCK.lock().unwrap_or_else(|e| e.into_inner());
    let _env = EnvSeam::set(&[
        ("KNOWLU_ENGINE_EXE", std::ffi::OsStr::new("cmd")),
        ("LOCALAPPDATA", fake.as_os_str()),
        ("KNOWLU_API_BASE", std::ffi::OsStr::new("http://127.0.0.1:9/functions/v1")),
    ]);
    let s = run_slot_inner(&cs, &sch, None, false);
    let named: Vec<String> = s.steps.iter().map(|(n, _)| n.clone()).collect();
    assert!(!named.iter().any(|n| n.starts_with("entitlement (")), "{named:?}");
    // Nothing was skipped, so nothing was written: D8 adds a line for a skip, not for every slot.
    let log = knowlu_engine::pystr::read_text(&v.join("state").join("runner-log.md")).unwrap_or_default();
    assert!(!log.contains("(skipped:"), "{log}");
    let _ = std::fs::remove_dir_all(&fake);
    let _ = std::fs::remove_dir_all(&v);
}

/// Fix round 1, item 2: `config/cloud.yaml` exists but does not parse — a corrupted file, or one
/// edited by hand and left broken. This must NOT collapse into `NoAccount` (which would fall through
/// to the local runtime/model gate and run `judge` with no account at all): it is named as its own
/// skip, distinct from "no entitlement" and from "no runtime"/"no model".
#[test]
fn an_unreadable_cloud_yaml_is_named_and_never_falls_into_the_local_arm() {
    use knowlu::account::EntitlementState;
    let v = scratch("unreadablecloud");
    std::fs::write(
        v.join("config").join("runners.yaml"),
        format!("runners:\n  - name: local\n    times: [\"12:00\"]\n    tz: America/Chicago\n    grace_minutes: 20\n    device: {}\n    scheduler: app\n", knowlu_engine::journal::device_name()),
    ).unwrap();
    // Garbage: present, but neither valid YAML nor a mapping with the required keys.
    std::fs::write(v.join("config").join("cloud.yaml"), "not: [valid\n").unwrap();
    let cs = open(&v, "unreadablecloud");
    assert_eq!(entitlement_state(&cs), EntitlementState::Unreadable);
    assert_eq!(judge_plan(&cs), JudgePlan::Skip("judge (skipped: cloud.yaml unreadable)"));

    let sch = Scheduler::default();
    let fake = std::env::temp_dir().join(format!("qo-sched-unreadablecloud-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&fake);
    std::fs::create_dir_all(&fake).unwrap();
    let _guard = ENGINE_ENV_LOCK.lock().unwrap_or_else(|e| e.into_inner());
    let _env = EnvSeam::set(&[("KNOWLU_ENGINE_EXE", std::ffi::OsStr::new("cmd")), ("LOCALAPPDATA", fake.as_os_str())]);
    let s = run_slot_inner(&cs, &sch, None, false);
    let named: Vec<String> = s.steps.iter().map(|(n, _)| n.clone()).collect();
    assert!(named.contains(&"judge (skipped: cloud.yaml unreadable)".to_string()), "{named:?}");
    assert!(!named.iter().any(|n| n == "judge"), "the step itself never ran");
    assert!(s.engine_ok, "a skipped judge step must not paint the tray amber: {:?}", s.steps);
    let _ = std::fs::remove_dir_all(&fake);
    let _ = std::fs::remove_dir_all(&v);
}

/// Task 10 review (Important): a `config/cloud.yaml` that exists but is unreadable must NOT trigger
/// the R-C1c-11 in-slot refresh either — `refresh_entitlement`'s own first call is the identical
/// `cloud_config` that already fails the gate, so a refresh attempt here could only ever record a
/// doomed `entitlement (refresh failed: …)` step, forever, for a vault that will never recover on
/// its own. `judge_plan_for` already names this vault's own skip (`cloud.yaml unreadable`); a second,
/// separate step naming the same broken file a second way adds nothing a student can act on.
#[test]
fn an_unreadable_cloud_yaml_never_triggers_an_entitlement_refresh_either() {
    let v = scratch("unreadablecloudent");
    std::fs::write(
        v.join("config").join("runners.yaml"),
        format!("runners:\n  - name: local\n    times: [\"12:00\"]\n    tz: America/Chicago\n    grace_minutes: 20\n    device: {}\n    scheduler: app\n", knowlu_engine::journal::device_name()),
    ).unwrap();
    // Garbage: present, but neither valid YAML nor a mapping with the required keys — exactly its
    // sibling test's fixture above, kept in its own vault so the two tests never share state.
    std::fs::write(v.join("config").join("cloud.yaml"), "not: [valid\n").unwrap();
    let cs = open(&v, "unreadablecloudent");
    let sch = Scheduler::default();
    let fake = std::env::temp_dir().join(format!("qo-sched-unreadablecloudent-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&fake);
    std::fs::create_dir_all(&fake).unwrap();
    let _guard = ENGINE_ENV_LOCK.lock().unwrap_or_else(|e| e.into_inner());
    let _env = EnvSeam::set(&[("KNOWLU_ENGINE_EXE", std::ffi::OsStr::new("cmd")), ("LOCALAPPDATA", fake.as_os_str())]);
    let s = run_slot_inner(&cs, &sch, None, false);
    let named: Vec<String> = s.steps.iter().map(|(n, _)| n.clone()).collect();
    assert!(!named.iter().any(|n| n.starts_with("entitlement (")), "{named:?}");
    let _ = std::fs::remove_dir_all(&fake);
    let _ = std::fs::remove_dir_all(&v);
}

/// The telemetry step never fails a slot. This vault has an account, no reachable cloud (the api_base
/// points at a port nothing is listening on) and therefore an offline send — and the slot is green.
#[test]
fn an_offline_telemetry_send_is_a_named_step_and_never_a_failure() {
    // Fix round 1 (I1): a real send reaches `valid_access_token_at`, which reads the real
    // Credential Manager for the session this test writes below — held for the whole test.
    let _credman_guard = CREDMAN_LOCK.lock().unwrap_or_else(|e| e.into_inner());
    let v = scratch("teleoffline");
    std::fs::write(
        v.join("config").join("runners.yaml"),
        format!("runners:\n  - name: local\n    times: [\"12:00\"]\n    tz: America/Chicago\n    grace_minutes: 20\n    device: {}\n    scheduler: app\n", knowlu_engine::journal::device_name()),
    ).unwrap();
    // Fix round 1 (I1): `vault-full` carries no `state/events-ui/` directory at all, so `send`
    // returned `(0, 0)` before ever reaching `valid_access_token_at` or `post_batch_at` — the step
    // read "nothing new" and the test never exercised the offline branch it is named for. One row
    // (the brief's own `app/tests/telemetry.rs` fixture line) is enough to make `send` reach the
    // network.
    std::fs::create_dir_all(v.join("state").join("events-ui")).unwrap();
    std::fs::write(
        v.join("state").join("events-ui").join("2026-09-09.jsonl"),
        format!("{}\n", r#"{"action": "view_opened", "device": "M", "ms": null, "object_id": null, "object_kind": null, "session": "sess_1", "ts": "2026-09-09T12:00:00.000Z", "view": "today"}"#),
    ).unwrap();
    // Port 9 is `discard`: nothing on this machine answers it, so the send fails fast and locally.
    std::fs::write(
        v.join("config").join("cloud.yaml"),
        "api_base: 'http://127.0.0.1:9/functions/v1'\nanon_key: 'anon'\nsession_credential_target: 'knowlu/profile_x/session'\naccount_id: 'acc-1'\n",
    ).unwrap();
    let cs = open(&v, "teleoffline");
    knowlu::account::save_cache(&cs.data_dir, &knowlu::account::EntitlementCache {
        status: "active".into(), current_period_end: None, plan: None, checked_at: knowlu_engine::journal::now_ts(None),
    }).unwrap();
    // A real session, far from expiry, so `valid_access_token_at` reads it straight off the store
    // with no refresh call — the only network reach this test exercises is `post_batch_at` itself,
    // against port 9.
    let target = "knowlu/profile_x/session";
    let _cred_cleanup = CredCleanup(target.to_string());
    knowlu::account::save_session(target, "acc-1", &knowlu::account::Session {
        access_token: "test-access-token".into(),
        refresh_token: "test-refresh-token".into(),
        expires_at: jiff::Timestamp::now().as_second() + 3600,
        email: "a@example.invalid".into(),
    }).unwrap();
    let sch = Scheduler::default();
    let fake = std::env::temp_dir().join(format!("qo-sched-tele-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&fake);
    std::fs::create_dir_all(&fake).unwrap();
    let _guard = ENGINE_ENV_LOCK.lock().unwrap_or_else(|e| e.into_inner());
    let _env = EnvSeam::set(&[
        ("KNOWLU_ENGINE_EXE", std::ffi::OsStr::new("cmd")),
        ("LOCALAPPDATA", fake.as_os_str()),
        // `cloud_config`'s R-C1-59 I1 host check needs this build's own `api_base()` to name the
        // same port-9 host the vault's `cloud.yaml` does, above.
        ("KNOWLU_API_BASE", std::ffi::OsStr::new("http://127.0.0.1:9/functions/v1")),
    ]);
    let s = run_slot_inner(&cs, &sch, None, false);
    let named: Vec<String> = s.steps.iter().map(|(n, _)| n.clone()).collect();
    // The exact name, not merely a prefix that "nothing new" and "no account" would also match
    // (I1) — this is the one assertion that would catch the offline branch being changed to fail.
    assert!(named.contains(&"telemetry (skipped: offline)".to_string()), "{named:?}");
    assert!(s.steps.iter().filter(|(n, _)| n.starts_with("telemetry")).all(|(_, c)| *c == 0));
    assert!(s.engine_ok, "telemetry must never paint the tray amber: {:?}", s.steps);
    let _ = std::fs::remove_dir_all(&fake);
    let _ = std::fs::remove_dir_all(&v);
}

/// §4.2 step 7, §11a: a vault the wizard just made has no `today.md`, so the console's own launch is
/// what fires the first slot — and a vault **adopted in place** (Task 18) already has one, so nothing
/// fires a second time on top of the work that is already there.
#[test]
fn a_new_vault_needs_a_first_run_and_an_adopted_one_does_not() {
    use knowlu::scheduler::needs_first_run;
    let v = scratch("firstrun");
    // `rank` writes `state/today.md` (cli.rs); a root `today.md` is nobody's file and must not count.
    let state = v.join("state");
    std::fs::create_dir_all(&state).unwrap();
    let today = state.join("today.md");
    let _ = std::fs::remove_file(&today);
    assert!(needs_first_run(&v), "a vault with no state/today.md is owed its first slot");
    std::fs::write(v.join("today.md"), b"# not the engine's file\n").unwrap();
    assert!(needs_first_run(&v), "a root today.md is not the ranked page");
    std::fs::write(&today, b"# Today\n").unwrap();
    assert!(!needs_first_run(&v), "an adopted vault already has state/today.md and must not run again");
    let _ = std::fs::remove_dir_all(&v);
}

/// An app-scheduled `local` runner on this device, so `run_slot_inner` runs rather than refuses.
fn app_scheduled(v: &Path) {
    std::fs::write(
        v.join("config").join("runners.yaml"),
        format!("runners:\n  - name: local\n    times: [\"12:00\"]\n    tz: America/Chicago\n    grace_minutes: 20\n    device: {}\n    scheduler: app\n", knowlu_engine::journal::device_name()),
    ).unwrap();
}

/// R-C1c-8: the first-run view reads `Scheduler.live` while a slot runs, because `Scheduler.last`
/// is written only when the slot ends — by which time `rank` has written the day and the view is
/// gone. So every step the slot records has to reach `live` as it lands, from every kind of site
/// (a named skip, a child, the backup, telemetry), and when the slot is over the two lists are the
/// same list. `current` names a step in progress; a finished slot has none.
///
/// The sync steps are not named (R-C1c-final2 M2): C3′ replaces the pull and the push with a `sync`
/// child, and `live.steps == summary.steps` already covers whatever the slot records.
///
/// The `cmd` stand-in engine and a temp `LOCALAPPDATA`, under `ENGINE_ENV_LOCK`, exactly as the
/// skip tests above.
#[test]
fn a_slot_publishes_every_step_it_records_while_it_runs() {
    let v = scratch("livesteps");
    app_scheduled(&v);
    let cs = open(&v, "livesteps");
    // A backup folder, so the backup step lands too and the mirror is proved over it.
    let bdir = std::env::temp_dir().join(format!("qo-console-sched-livesteps-backup-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&bdir);
    { cs.settings.lock().unwrap().backup_dir = Some(bdir.clone()); }
    let sch = Scheduler::default();
    let fake = std::env::temp_dir().join(format!("qo-sched-livesteps-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&fake);
    std::fs::create_dir_all(&fake).unwrap();
    let _guard = ENGINE_ENV_LOCK.lock().unwrap_or_else(|e| e.into_inner());
    let _env = EnvSeam::set(&[("KNOWLU_ENGINE_EXE", std::ffi::OsStr::new("cmd")), ("LOCALAPPDATA", fake.as_os_str())]);
    let s = run_slot_inner(&cs, &sch, None, false);
    assert!(s.reason.is_none(), "not refused: {:?}", s.reason);
    let live = lock(&sch.live).clone();
    assert_eq!(live.steps, s.steps, "the published list is the slot's own list, step for step");
    assert!(live.current.is_none(), "a slot that has ended has nothing in progress: {live:?}");
    let named: Vec<&str> = live.steps.iter().map(|(n, _)| n.as_str()).collect();
    for want in ["ingest (skipped: no ics_url)", "coursework", "rank", "backup", "telemetry (skipped: no account)"] {
        assert!(named.contains(&want), "{want} did not reach the published list: {named:?}");
    }
    assert!(named.iter().any(|n| n.starts_with("judge (skipped:")), "{named:?}");
    let _ = std::fs::remove_dir_all(&bdir);
    let _ = std::fs::remove_dir_all(&fake);
    let _ = std::fs::remove_dir_all(&v);
}

/// R-C1c-8: a slot that really starts clears the published list in the same step that sets
/// `running`, so no poll can pair `running: true` with the previous slot's steps. A slot that does
/// not start — refused, or turned away because one is already running — leaves it alone.
#[test]
fn a_new_slot_starts_with_an_empty_live_list() {
    let v = scratch("livereset");
    app_scheduled(&v);
    let cs = open(&v, "livereset");
    let sch = Scheduler::default();
    let junk = || LiveSlot { steps: vec![("an older slot's step".to_string(), 7)], current: Some("an older slot's step in progress".to_string()) };
    *lock(&sch.live) = junk();

    // Turned away: a slot is already running, and that slot's list is the one on the page.
    *lock(&sch.running) = true;
    let _ = run_slot_inner(&cs, &sch, None, false);
    *lock(&sch.running) = false;
    assert_eq!(lock(&sch.live).steps, junk().steps, "an overlapping trigger must not clear the running slot's list");
    // Refused: vault-full's own runners.yaml says `scheduler: script`.
    let refused_vault = scratch("livereset-refused");
    let refused = run_slot_inner(&open(&refused_vault, "livereset-refused"), &sch, None, false);
    assert!(refused.reason.is_some(), "this slot was refused: {refused:?}");
    assert_eq!(lock(&sch.live).current, junk().current, "a refusal never started, so it publishes nothing");

    let fake = std::env::temp_dir().join(format!("qo-sched-livereset-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&fake);
    std::fs::create_dir_all(&fake).unwrap();
    let _guard = ENGINE_ENV_LOCK.lock().unwrap_or_else(|e| e.into_inner());
    let _env = EnvSeam::set(&[("KNOWLU_ENGINE_EXE", std::ffi::OsStr::new("cmd")), ("LOCALAPPDATA", fake.as_os_str())]);
    let s = run_slot_inner(&cs, &sch, None, false);
    let live = lock(&sch.live).clone();
    assert!(!live.steps.iter().any(|(n, _)| n == "an older slot's step"), "the older slot's step survived: {live:?}");
    assert!(live.current.is_none(), "the older slot's step in progress survived: {live:?}");
    assert_eq!(live.steps, s.steps);
    let _ = std::fs::remove_dir_all(&fake);
    let _ = std::fs::remove_dir_all(&refused_vault);
    let _ = std::fs::remove_dir_all(&v);
}

/// Creates the handshake's go-ahead file when dropped — on a failing assertion too — so the
/// stand-in engine below never waits out the twenty-minute child cap.
struct GoAhead(PathBuf);
impl Drop for GoAhead {
    fn drop(&mut self) {
        let _ = std::fs::write(&self.0, b"go");
    }
}

/// R-C1c-8: while a step does its work, `live.current` names it and `live.steps` holds exactly what
/// has landed before it. Observed by handshake, not by timing: the stand-in engine is a batch file
/// that, when called as `coursework`, drops a `started` marker and then waits until this test
/// writes `go`, so the slot is provably inside that step while `live` is read. Every other step
/// exits 0 at once.
#[test]
fn a_slot_names_the_step_in_progress_while_it_runs() {
    let v = scratch("livecurrent");
    app_scheduled(&v);
    let cs = open(&v, "livecurrent");
    let sch = Scheduler::default();
    let hs = std::env::temp_dir().join(format!("qo-sched-handshake-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&hs);
    std::fs::create_dir_all(&hs).unwrap();
    // CRLF inside the file: `goto` in a batch file with bare LF line ends is unreliable.
    let bat = hs.join("engine.bat");
    std::fs::write(&bat, concat!(
        "@echo off\r\n",
        "if not \"%1\"==\"coursework\" exit /b 0\r\n",
        "echo started>\"%KNOWLU_TEST_HANDSHAKE%\\started\"\r\n",
        ":wait\r\n",
        "if exist \"%KNOWLU_TEST_HANDSHAKE%\\go\" exit /b 0\r\n",
        "ping -n 2 127.0.0.1 >nul\r\n",
        "goto wait\r\n",
    )).unwrap();
    let fake = std::env::temp_dir().join(format!("qo-sched-livecurrent-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&fake);
    std::fs::create_dir_all(&fake).unwrap();
    let _guard = ENGINE_ENV_LOCK.lock().unwrap_or_else(|e| e.into_inner());
    let _env = EnvSeam::set(&[
        ("KNOWLU_ENGINE_EXE", bat.as_os_str()),
        ("LOCALAPPDATA", fake.as_os_str()),
        ("KNOWLU_TEST_HANDSHAKE", hs.as_os_str()),
    ]);
    let (mid, s) = std::thread::scope(|scope| {
        let slot = scope.spawn(|| run_slot_inner(&cs, &sch, None, false));
        let go = GoAhead(hs.join("go"));
        let deadline = Instant::now() + Duration::from_secs(60);
        while !hs.join("started").exists() {
            assert!(!slot.is_finished(), "the slot ended without the stand-in engine reaching coursework");
            assert!(Instant::now() < deadline, "the stand-in engine never reached coursework");
            std::thread::sleep(Duration::from_millis(20));
        }
        let mid = lock(&sch.live).clone();
        drop(go);
        (mid, slot.join().unwrap())
    });
    assert_eq!(mid.current.as_deref(), Some("coursework"), "the step in progress is named: {mid:?}");
    let landed: Vec<(String, i32)> = s.steps.iter().take_while(|(n, _)| n != "coursework").cloned().collect();
    assert!(!landed.is_empty(), "the named skips land before the first child: {:?}", s.steps);
    assert_eq!(mid.steps, landed, "mid-slot, the list is exactly what has landed so far");
    assert_eq!(s.steps.iter().find(|(n, _)| n == "coursework").map(|(_, c)| *c), Some(0), "{:?}", s.steps);
    assert!(lock(&sch.live).current.is_none(), "nothing is in progress once the slot has ended");
    let _ = std::fs::remove_dir_all(&hs);
    let _ = std::fs::remove_dir_all(&fake);
    let _ = std::fs::remove_dir_all(&v);
}
