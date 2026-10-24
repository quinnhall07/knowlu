use knowlu::commands::attach_scheduler;
use knowlu::scheduler::{device_ok, engine_exe, entitlement_state, has_ics_url, ics_state, ingest_included, judge_plan, lock, mode, prune_logs, run_child, run_slot_inner, should_retry, slot_argv, IcsState, JudgePlan, LiveSlot, Scheduler};
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
    // A skip plan: the step is left out entirely, exactly as `ingest` is on a vault with no feed.
    // `sync` is always first (C3′, cloud design §5.5 as amended): the pull half has to land before
    // `rank` orders the day, and the push half carries everything written since the last sync.
    let argv = slot_argv(&v, exe, &JudgePlan::Skip("judge (skipped: no account)"), None);
    assert_eq!(names(&argv), vec!["sync", "coursework", "rank"]);
    assert_eq!(argv[0].1, vec!["sync", "--vault", v.to_string_lossy().as_ref(), "--via", "local-runner"]);
    assert_eq!(argv[1].1, vec!["coursework", "--vault", v.to_string_lossy().as_ref(), "--via", "local-runner"]);
    assert_eq!(argv[2].1, vec!["rank", "--vault", v.to_string_lossy().as_ref(), "--runner", "local"]);

    let cfg = v.join("config").join("ingest.yaml");
    let old = std::fs::read_to_string(&cfg).unwrap();
    std::fs::write(&cfg, format!("ics_url: \"https://lms.example.invalid/learn.ics\"\n{old}")).unwrap();
    assert!(has_ics_url(&v));
    let argv = slot_argv(&v, exe, &JudgePlan::Skip("judge (skipped: no account)"), None);
    assert_eq!(names(&argv), vec!["sync", "coursework", "ingest", "rank"]);
    assert_eq!(argv[2].1, vec!["ingest", "--vault", v.to_string_lossy().as_ref(), "--via", "local-runner"]);
    assert!(argv.iter().all(|(e, _)| e == exe));

    // A cloud plan: FIVE steps, and judge sits BEFORE rank so the day's ranking sees what it
    // just wrote. `--via local-runner`, the same value coursework and ingest pass — journal::VIAS
    // does not grow for this.
    let plan = JudgePlan::Cloud { log_dir: PathBuf::from(r"C:\data\judgments") };
    let argv = slot_argv(&v, exe, &plan, None);
    assert_eq!(names(&argv), vec!["sync", "coursework", "ingest", "judge", "rank"]);
    assert_eq!(argv[3].1, vec![
        "judge".to_string(), "--vault".to_string(), v.to_string_lossy().to_string(),
        "--via".to_string(), "local-runner".to_string(),
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

    let argv = slot_argv(&v, exe, &JudgePlan::Skip("judge (skipped: no account)"), None);
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
    let steps = slot_argv(&v, Path::new("knowlu-engine.exe"), &JudgePlan::Skip("judge (skipped: no entitlement)"), None);
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
    let steps = slot_argv(&v, exe, &JudgePlan::Skip("judge (skipped: no entitlement)"), None);
    assert_eq!(steps[0].1[0], "sync", "{steps:?}");
    assert_eq!(steps[1].1[0], "coursework", "and nothing was displaced");
    assert!(steps.iter().any(|(_, a)| a[0] == "rank"), "{steps:?}");
    // One row in the Runs view, not two: `run_slot_inner` names a step by `args[0]`.
    assert_eq!(steps.iter().filter(|(_, a)| a[0] == "sync").count(), 1);
    let _ = std::fs::remove_dir_all(&v);
}

/// D7: a vault with no account is NORMAL. The step is recorded with code 0 and a name that says
/// why — the shape `ingest (skipped: no ics_url)` already uses — so a friend with no account sees an
/// explanation on the Runs view rather than a slot that quietly does less.
///
/// What it pins: no account is a named skip, `judge (skipped: no account)`, at code 0, **and
/// `engine_ok`** (the property that matters: the skip must never paint the tray amber).
///
/// **Against a temp `LOCALAPPDATA`, never the real one.** After the local judge plan's removal nothing
/// on `run_slot_inner`'s path reaches `state::app_data_root()`, so the seam below is a guard that keeps
/// any future call off the developer's real app data (and its `profiles::migrate_flat_layout`), not a
/// shield for a current one. The seam is an `EnvSeam` held for the whole function, so a panicking
/// assertion below still restores both env vars via `Drop`.
#[test]
fn a_vault_with_no_account_records_the_judge_skip_and_stays_green() {
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
    assert_eq!(judge, "judge (skipped: no account)");
    let code = summary.steps.iter().find(|(n, _)| n.starts_with("judge")).unwrap().1;
    assert_eq!(code, 0, "a skip is not a failure — it must never paint the tray amber");
    // Task 10 review: the step's own exit code being 0 is not the same claim as the slot staying
    // green — `engine_ok` is what `attach_scheduler` feeds the tray, so THIS is what must be true.
    assert!(summary.engine_ok, "a skipped judge step must not paint the tray amber: {:?}", summary.steps);
    let _ = std::fs::remove_dir_all(&fake_local_appdata);
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
/// **Against a temp `LOCALAPPDATA`, never the real one**: after the local judge plan's removal no call
/// on `run_slot_inner`'s path reaches `state::app_data_root()`, so the seam is a guard that keeps any
/// future call off the developer's real app data, exactly as in its sibling
/// `a_vault_with_no_account_records_the_judge_skip_and_stays_green`.
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

/// …and a vault with no cloud config at all is `NoAccount`, which is what makes `judge` the named
/// skip `judge (skipped: no account)` rather than a cloud call — case (b).
#[test]
fn a_vault_with_no_cloud_config_says_no_account_rather_than_no_entitlement() {
    use knowlu::account::EntitlementState;
    let v = scratch("noaccount");
    let cs = open(&v, "noaccount");
    assert_eq!(entitlement_state(&cs), EntitlementState::NoAccount);
    let _ = std::fs::remove_dir_all(&v);
}

/// Case (a), and the one C2 depends on: **an entitled vault runs `judge`, with nothing but the vault
/// on the command line.** A skip keyed on anything installed on the machine would leave the whole
/// judgment service inert on every machine that has no local install, which is every machine.
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
    // Pinned to equality (L6 T1): the folder must stay byte for byte `<profile data>\judgments`.
    assert_eq!(log_dir, cs.data_dir.join("judgments"));
    let argv = slot_argv(&v, Path::new(r"C:\bin\knowlu-engine.exe"), &plan, None);
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

/// Ruling 10 (L6): a vault with no `config/cloud.yaml` gets a named judge skip, the same words `sync`
/// uses, never a silent omission. It runs against a fresh temp `LOCALAPPDATA` so the check can never
/// read the developer's real app data.
#[test]
fn a_vault_with_no_account_skips_judge_by_name() {
    let v = scratch("noaccountjudge");
    assert!(!v.join("config").join("cloud.yaml").exists(), "the fixture has no account");
    let fake_local_appdata = std::env::temp_dir().join(format!("qo-console-sched-noacct-localappdata-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&fake_local_appdata);
    std::fs::create_dir_all(&fake_local_appdata).unwrap();
    let _guard = ENGINE_ENV_LOCK.lock().unwrap_or_else(|e| e.into_inner());
    let _env = EnvSeam::set(&[("LOCALAPPDATA", fake_local_appdata.as_os_str())]);
    let cs = open(&v, "noaccountjudge");
    assert_eq!(judge_plan(&cs), JudgePlan::Skip("judge (skipped: no account)"));
    let _ = std::fs::remove_dir_all(&fake_local_appdata);
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
    // Nothing of the judge's or ingest's was skipped, so nothing was written for them: D8 adds a line
    // for a skip, not for every slot. (The grades step's own `not a Blackboard school` skip is spec
    // §10's and is on every vault that is not a Blackboard school, this scratch vault included.)
    let log = knowlu_engine::pystr::read_text(&v.join("state").join("runner-log.md")).unwrap_or_default();
    assert!(!log.contains("judge (skipped:") && !log.contains("ingest (skipped:"), "{log}");
    let _ = std::fs::remove_dir_all(&fake);
    let _ = std::fs::remove_dir_all(&v);
}

/// Fix round 1, item 2: `config/cloud.yaml` exists but does not parse — a corrupted file, or one
/// edited by hand and left broken. This must NOT collapse into `NoAccount`, whose
/// `judge (skipped: no account)` would hide a broken file: it is its own skip, distinct from
/// "no entitlement" and "no account".
#[test]
fn an_unreadable_cloud_yaml_is_named_and_never_reads_as_no_account() {
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

// ---- Task 6 of the grades plan: the slot's grades step ------------------------------------------

use knowlu::account::EntitlementState;
use knowlu::grades::{self as grades_app, CaptureError};
use knowlu::scaffold::{Curated, CAMPUSES};
use knowlu::scheduler::{grades_step, run_slot_with, GradesSeam, GradesStep};
use std::cell::Cell;

/// The one place this file builds a curated row (as `tests/grades.rs` does): a synthetic school, a
/// struct update from a `CAMPUSES` row, never a full `Curated` literal and never an edit to `CAMPUSES`.
fn test_row(lms_kind: &'static str, policy_read: Option<&'static str>) -> Curated {
    Curated { unitid: "999999", lms_host: "lms.example.test", lms_kind, policy_read, ..CAMPUSES[0] }
}

fn with_session(tag: &str) -> PathBuf {
    let d = std::env::temp_dir().join(format!("qo-sched-grades-{tag}-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&d);
    std::fs::create_dir_all(grades_app::session_dir(&d)).unwrap();
    d
}

fn bundle() -> Value {
    json!({ "lms": "blackboard", "host": "lms.example.test", "courses": [] })
}

fn skip_of(s: GradesStep) -> String {
    match s {
        GradesStep::Skip(n) => n,
        GradesStep::Captured(p) => panic!("expected a skip, a bundle was written at {}", p.display()),
    }
}

#[test]
fn slot_argv_places_grades_after_coursework_and_before_ingest_only_with_a_bundle() {
    let v = scratch("grades-argv");
    let exe = Path::new(r"C:\bin\knowlu-engine.exe");
    let names = |a: &Vec<(PathBuf, Vec<String>)>| a.iter().map(|(_, x)| x[0].clone()).collect::<Vec<_>>();
    let bundle = PathBuf::from(r"C:\data\grades-capture.json");
    let judge = JudgePlan::Cloud { log_dir: PathBuf::from("logs") };
    let with = slot_argv(&v, exe, &judge, Some(&bundle));
    let at = names(&with).iter().position(|n| n == "grades").expect("a grades step");
    assert_eq!(names(&with)[at - 1], "coursework", "{:?}", names(&with));
    assert_eq!(names(&with)[at + 1], "judge", "no feed on this vault, so ingest is out: {:?}", names(&with));
    assert_eq!(with[at].1, vec!["grades", "--vault", v.to_string_lossy().as_ref(), "--input", bundle.to_string_lossy().as_ref(), "--via", "local-runner"]);
    // O1 (Q11): with a feed it is still right after coursework, and ingest follows it, so ingest
    // reads the grade notes this slot just wrote.
    let cfg = v.join("config").join("ingest.yaml");
    let old = std::fs::read_to_string(&cfg).unwrap();
    std::fs::write(&cfg, format!("ics_url: \"https://lms.example.invalid/learn.ics\"\n{old}")).unwrap();
    let with = slot_argv(&v, exe, &judge, Some(&bundle));
    let n = names(&with);
    assert_eq!(&n[n.iter().position(|x| x == "coursework").unwrap() + 1..][..3], ["grades", "ingest", "judge"], "{n:?}");
    // And without a bundle the step is not there at all.
    assert!(!names(&slot_argv(&v, exe, &judge, None)).contains(&"grades".to_string()));
    let _ = std::fs::remove_dir_all(&v);
}

/// The gate's fourth caller: with a saved session in every case, only a curated Blackboard row
/// captures — dated or not while ruling 12's read is suspended (cloud design Amendment 2026-09-30, A13).
#[test]
fn the_grades_decision_asks_the_predicate_and_never_rederives_it() {
    let calls = Cell::new(0);
    let capture = |_host: &'static str| { calls.set(calls.get() + 1); Ok(bundle()) };
    let dated = test_row("blackboard", Some("2026-10-01"));
    let undated = test_row("blackboard", None);
    let canvas = test_row("canvas", Some("2026-10-01"));

    let d = with_session("decision");
    let got = grades_step(Some(&dated), "blackboard", EntitlementState::Entitled, &d, false, Some(&capture));
    let GradesStep::Captured(p) = got else { panic!("a dated row captures") };
    assert_ne!(p, grades_app::bundle_path(&d), "the slot's bundle is its own, never the console's file");
    assert_eq!(p.parent(), Some(d.as_path()), "still under the profile's app data");
    assert_eq!(calls.get(), 1);
    assert!(p.is_file(), "the bundle is on disk for the engine step");
    let _ = std::fs::remove_dir_all(&d);

    let d = with_session("undated");
    let GradesStep::Captured(p) = grades_step(Some(&undated), "blackboard", EntitlementState::Entitled, &d, false, Some(&capture)) else {
        panic!("an undated row captures while the gate is suspended")
    };
    assert!(p.is_file());
    assert_eq!(calls.get(), 2);
    let _ = std::fs::remove_dir_all(&d);

    for (row, lms) in [(None, "blackboard")] {
        let d = with_session("uncurated");
        let got = grades_step(row, lms, EntitlementState::Entitled, &d, false, Some(&capture));
        assert_eq!(skip_of(got), "grades (skipped: not available at your school yet)");
        assert!(!grades_app::bundle_path(&d).exists());
        let _ = std::fs::remove_dir_all(&d);
    }
    let d = with_session("canvas");
    assert_eq!(skip_of(grades_step(Some(&canvas), "canvas", EntitlementState::Entitled, &d, false, Some(&capture))), "grades (skipped: not a Blackboard school)");
    assert_eq!(skip_of(grades_step(None, "canvas", EntitlementState::Entitled, &d, false, Some(&capture))), "grades (skipped: not a Blackboard school)");
    let _ = std::fs::remove_dir_all(&d);
    assert_eq!(calls.get(), 2, "the capture ran for the dated and undated rows only");
}

/// One test per adjacent pair: the earlier skip wins when both hold.
#[test]
fn the_skip_order_is_school_then_availability_then_entitlement_then_session_then_window() {
    let calls = Cell::new(0);
    let capture = |_h: &'static str| { calls.set(calls.get() + 1); Ok(bundle()) };
    let (dated, canvas) = (test_row("blackboard", Some("2026-10-01")), test_row("canvas", None));
    let no_session = std::env::temp_dir().join(format!("qo-sched-grades-nosession-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&no_session);
    let with = with_session("order");
    let lapsed = EntitlementState::NotEntitled;
    let ok = EntitlementState::Entitled;

    // not a Blackboard school beats not available yet
    assert_eq!(skip_of(grades_step(Some(&canvas), "canvas", lapsed, &no_session, true, Some(&capture))), "grades (skipped: not a Blackboard school)");
    // not available yet (an uncurated Blackboard school) beats no entitlement
    assert_eq!(skip_of(grades_step(None, "blackboard", lapsed, &no_session, true, Some(&capture))), "grades (skipped: not available at your school yet)");
    // no entitlement beats not connected
    assert_eq!(skip_of(grades_step(Some(&dated), "blackboard", lapsed, &no_session, true, Some(&capture))), "grades (skipped: no entitlement)");
    // not connected beats sign-in window open
    assert_eq!(skip_of(grades_step(Some(&dated), "blackboard", ok, &no_session, true, Some(&capture))), "grades (skipped: not connected)");
    // sign-in window open, with a session
    assert_eq!(skip_of(grades_step(Some(&dated), "blackboard", ok, &with, true, Some(&capture))), "grades (skipped: sign-in window open)");
    // no window on this run: after not connected
    assert_eq!(skip_of(grades_step(Some(&dated), "blackboard", ok, &no_session, true, None)), "grades (skipped: not connected)");
    assert_eq!(skip_of(grades_step(Some(&dated), "blackboard", ok, &with, false, None)), "grades (skipped: no window on this run)");
    // an account-less vault is not gated by entitlement, as the engine's own gate treats it
    assert!(matches!(grades_step(Some(&dated), "blackboard", EntitlementState::NoAccount, &with, false, Some(&capture)), GradesStep::Captured(_)));
    assert_eq!(calls.get(), 1, "only the last call reached the capture");
    let _ = std::fs::remove_dir_all(&with);
}

#[test]
fn a_captures_own_outcomes_are_named_and_signed_out_is_remembered() {
    let dated = test_row("blackboard", Some("2026-10-01"));
    let d = with_session("outcomes");
    let out = |e: CaptureError| skip_of(grades_step(Some(&dated), "blackboard", EntitlementState::Entitled, &d, false, Some(&move |_h| Err(e))));
    assert_eq!(out(CaptureError::Unreachable), "grades (skipped: Blackboard unreachable)");
    assert_eq!(grades_app::status_for(Some(&dated), "blackboard", &d, &d)["signed_out"], json!(false));
    assert_eq!(out(CaptureError::SignedOut), "grades (skipped: signed out)");
    assert_eq!(grades_app::status_for(Some(&dated), "blackboard", &d, &d)["signed_out"], json!(true), "the strip asks the student to sign in again");
    assert_eq!(out(CaptureError::WindowOpen), "grades (skipped: sign-in window open)");
    // a good capture clears the mark
    let good = |_h: &'static str| Ok(bundle());
    assert!(matches!(grades_step(Some(&dated), "blackboard", EntitlementState::Entitled, &d, false, Some(&good)), GradesStep::Captured(_)));
    assert_eq!(grades_app::status_for(Some(&dated), "blackboard", &d, &d)["signed_out"], json!(false));
    let _ = std::fs::remove_dir_all(&d);
}

/// Spec §10 and §5: a capture never paints the tray amber, and each capture's bundle is deleted by its
/// own step. The console's Refresh writes and deletes `grades-capture.json` while the slot's engine
/// step is still to run; the slot's bundle must survive that, and must not be the console's.
#[test]
fn the_slots_bundle_is_private_to_its_run_and_survives_a_console_refresh() {
    let dated = test_row("blackboard", Some("2026-10-01"));
    let d = with_session("private");
    let mine = |_h: &'static str| Ok(json!({ "lms": "blackboard", "host": "lms.example.test", "courses": [], "mine": "slot" }));
    let GradesStep::Captured(p) = grades_step(Some(&dated), "blackboard", EntitlementState::Entitled, &d, false, Some(&mine)) else { panic!("captured") };
    // The console's Refresh: writes its own bundle, then deletes it after its engine step.
    let console = grades_app::bundle_path(&d);
    std::fs::write(&console, "{\"mine\":\"console\"}").unwrap();
    std::fs::remove_file(&console).unwrap();
    assert!(p.is_file(), "the console's delete must not take the slot's bundle");
    assert!(std::fs::read_to_string(&p).unwrap().contains("\"slot\""), "and it is still the slot's own capture");
    // The console writing over its own path must not overwrite the slot's either.
    std::fs::write(&console, "{\"mine\":\"console\"}").unwrap();
    assert!(std::fs::read_to_string(&p).unwrap().contains("\"slot\""));
    // Two slot captures never share a path.
    let GradesStep::Captured(q) = grades_step(Some(&dated), "blackboard", EntitlementState::Entitled, &d, false, Some(&mine)) else { panic!("captured") };
    assert_ne!(p, q);
    let _ = std::fs::remove_dir_all(&d);
}

/// No `grades-capture*.json` of any run is left in the profile's app data.
fn no_bundles_left(data_dir: &Path) -> bool {
    std::fs::read_dir(data_dir).map(|d| d.flatten().all(|e| !e.file_name().to_string_lossy().starts_with("grades-capture"))).unwrap_or(true)
}

fn app_slot_vault(tag: &str) -> (PathBuf, ConsoleState) {
    let v = scratch(tag);
    std::fs::write(
        v.join("config").join("runners.yaml"),
        format!("runners:\n  - name: local\n    times: [\"12:00\"]\n    tz: America/Chicago\n    grace_minutes: 20\n    device: {}\n    scheduler: app\n", knowlu_engine::journal::device_name()),
    ).unwrap();
    let cs = open(&v, tag);
    (v, cs)
}

/// Each skip is a named step at exit 0 and a runner-log line with status `ok`, through the same
/// filter as the ingest and judge skips; and a slot that captured runs the `grades` step after
/// `coursework` and deletes the bundle whatever the step's exit code.
#[test]
fn the_slot_records_the_grades_skip_and_runs_and_cleans_up_the_grades_step() {
    let (v, cs) = app_slot_vault("grades-slot");
    let sch = Scheduler::default();
    let fake = std::env::temp_dir().join(format!("qo-sched-grades-localappdata-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&fake);
    std::fs::create_dir_all(&fake).unwrap();
    let _guard = ENGINE_ENV_LOCK.lock().unwrap_or_else(|e| e.into_inner());
    let _env = EnvSeam::set(&[("KNOWLU_ENGINE_EXE", std::ffi::OsStr::new("cmd")), ("LOCALAPPDATA", fake.as_os_str())]);
    std::fs::create_dir_all(grades_app::session_dir(&cs.data_dir)).unwrap();

    // An uncurated Blackboard school (no curated row; the vault's `campus.yaml` says `blackboard`): the
    // named skip, exit 0, in the runner log as `ok`, and the capture is never called.
    std::fs::write(v.join("config").join("campus.yaml"), "unitid: '999999'\r\nlms: 'blackboard'\r\n").unwrap();
    let calls = Cell::new(0);
    let capture = |_h: &'static str| { calls.set(calls.get() + 1); Ok(bundle()) };
    let s = run_slot_with(&cs, &sch, None, false, &GradesSeam { row: None, capture: Some(&capture), window_open: false }, &MailSeam::empty());
    let skip = s.steps.iter().find(|(n, _)| n.starts_with("grades (skipped")).expect("a named grades skip");
    assert_eq!(skip, &("grades (skipped: not available at your school yet)".to_string(), 0));
    assert!(s.engine_ok, "a skip never paints the tray amber: {:?}", s.steps);
    assert_eq!(calls.get(), 0);
    assert!(!s.steps.iter().any(|(n, _)| n == "grades"));
    let log = knowlu_engine::pystr::read_text(&v.join("state").join("runner-log.md")).unwrap();
    assert!(log.contains("local ok grades (skipped: not available at your school yet)"), "{log}");

    // Dated row: captured, the engine step runs between coursework and rank, and the bundle is gone.
    let dated = test_row("blackboard", Some("2026-10-01"));
    let s = run_slot_with(&cs, &sch, None, false, &GradesSeam { row: Some(&dated), capture: Some(&capture), window_open: false }, &MailSeam::empty());
    let names: Vec<&str> = s.steps.iter().map(|(n, _)| n.as_str()).collect();
    let at = names.iter().position(|n| *n == "grades").unwrap_or_else(|| panic!("{names:?}"));
    assert!(names.iter().position(|n| *n == "coursework").unwrap() < at && at < names.iter().position(|n| *n == "rank").unwrap(), "{names:?}");
    assert_eq!(calls.get(), 1);
    assert!(no_bundles_left(&cs.data_dir), "the bundle is deleted after the step");

    // A failing engine step (a path that cannot be spawned) deletes it too.
    unsafe { std::env::set_var("KNOWLU_ENGINE_EXE", r"C:\knowlu-test-no-such-dir\knowlu-engine.exe") };
    let s = run_slot_with(&cs, &sch, None, false, &GradesSeam { row: Some(&dated), capture: Some(&capture), window_open: false }, &MailSeam::empty());
    assert_eq!(calls.get(), 2);
    assert!(s.steps.iter().any(|(n, c)| n == "grades" && *c != 0), "{:?}", s.steps);
    assert!(no_bundles_left(&cs.data_dir), "the bundle is deleted whatever the exit code");

    let _ = std::fs::remove_dir_all(&fake);
    let _ = std::fs::remove_dir_all(&cs.data_dir);
    let _ = std::fs::remove_dir_all(&v);
}

// ---- T11.5: the `mail` step in the slot (spec test 36, plan P18, R7, R17) -------------------------------------
//
// The decision (`mail_step`) is tested through its seam with closures, the way `grades_step` is; the pass behind
// it (`mail::slot::run_step`) over a fake window and a loopback service in `support/slot_rig.rs`, whose address
// and bearer are unique markers. Nothing here opens a real window or leaves the machine.

use knowlu::mail::slot::{reconcile_due_with, run_step, Cadence, SlotEnv, RECONCILE_CADENCE};
use knowlu::mail_session::MailState;
use knowlu::scheduler::{mail_step, MailSeam};

#[path = "support/slot_rig.rs"]
mod rig;

fn mail_dir(tag: &str) -> PathBuf {
    let d = std::env::temp_dir().join(format!("qo-sched-mail-{tag}-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&d);
    std::fs::create_dir_all(&d).unwrap();
    d
}

fn second_row() -> Value {
    let mut b = rig::row("active", "m365");
    b["address_id"] = json!("0b0e5b2c-2222-4aaa-8bbb-000000000002");
    b
}

#[test]
fn the_mail_row_and_each_named_skip_exit_zero() {
    let d = mail_dir("skips");
    let rows = || Ok(vec![rig::row("active", "m365")]);
    let ran = Cell::new(0);
    let pass = |_: &[Value]| {
        ran.set(ran.get() + 1);
        Ok("mail: forwarding on, 2 recovered".to_string())
    };
    let seam = |window_open| MailSeam { mailboxes: &rows, pass: Some(&pass), window_open };
    let ok = EntitlementState::Entitled;
    let step = |est, window_open| mail_step(est, &d, rig::NOW, &seam(window_open));
    assert_eq!(step(ok, false).as_deref(), Some("mail: forwarding on, 2 recovered"));
    assert_eq!(ran.get(), 1);
    assert_eq!(step(EntitlementState::NotEntitled, false).as_deref(), Some("mail (skipped: no entitlement)"));
    assert_eq!(step(ok, true).as_deref(), Some("mail (skipped: mail window open)"));
    MailState::update(&d, |s| {
        s.last_reconcile.insert(rig::ID.to_string(), rig::iso(rig::NOW - 3600));
        true
    })
    .unwrap();
    assert_eq!(step(ok, false).as_deref(), Some("mail (skipped: not due)"));
    MailState::update(&d, |s| {
        s.keep_session = false;
        true
    })
    .unwrap();
    assert_eq!(step(ok, false).as_deref(), Some("mail (skipped: saved sign-in off)"));
    assert_eq!(step(EntitlementState::NotEntitled, false).as_deref(), Some("mail (skipped: no entitlement)"), "no entitlement is named before the switch");
    assert_eq!(ran.get(), 1, "no skip ran the pass");

    // In a slot every one of them is a step at exit 0 and never paints the tray amber.
    let (v, cs) = app_slot_vault("mail-skips");
    MailState::update(&cs.data_dir, |s| {
        s.keep_session = false;
        true
    })
    .unwrap();
    let sch = Scheduler::default();
    let s = run_mail_slot(&cs, &sch, &MailSeam { mailboxes: &rows, pass: Some(&pass), window_open: false });
    assert!(s.steps.contains(&("mail (skipped: saved sign-in off)".to_string(), 0)), "{:?}", s.steps);
    assert!(s.engine_ok && s.ok, "a skip is not a failure: {:?}", s.steps);
    let _ = std::fs::remove_dir_all(&cs.data_dir);
    let _ = std::fs::remove_dir_all(&v);
    let _ = std::fs::remove_dir_all(&d);
}

/// Checkpoint D (spec §4.11: one `mail` row or a named skip): a mailbox list the service could not give (an
/// expired session, a 5xx, no network) is no proof there is no mailbox, so it is a named skip at exit 0, before
/// every other one, and never the silence of a student with none. A list that answered with none is still no row.
#[test]
fn a_mailbox_list_the_service_could_not_give_is_a_named_skip_never_silence() {
    let d = mail_dir("unlisted");
    let ran = Cell::new(0);
    let pass = |_: &[Value]| {
        ran.set(ran.get() + 1);
        Ok("mail: forwarding on, 0 recovered".to_string())
    };
    let failed = || Err("server");
    let none = || Ok(Vec::new());
    let named = Some("mail (skipped: could not read your mailboxes)");
    let step = |est, mailboxes: &dyn Fn() -> Result<Vec<Value>, &'static str>, window_open| {
        mail_step(est, &d, rig::NOW, &MailSeam { mailboxes, pass: Some(&pass), window_open })
    };
    assert_eq!(step(EntitlementState::Entitled, &failed, false).as_deref(), named);
    assert_eq!(step(EntitlementState::NotEntitled, &failed, true).as_deref(), named, "named before entitlement and the window");
    assert_eq!(step(EntitlementState::Entitled, &none, false), None, "a list of none is still no row");
    assert_eq!(ran.get(), 0, "nothing ran");
    // With no app handle there is no pass and nothing to ask: no row, as before.
    assert_eq!(mail_step(EntitlementState::Entitled, &d, rig::NOW, &MailSeam { mailboxes: &failed, pass: None, window_open: false }), None);

    // In a slot it is a row at exit 0 that never paints the tray amber.
    let (v, cs) = app_slot_vault("mail-unlisted");
    let sch = Scheduler::default();
    let s = run_mail_slot(&cs, &sch, &MailSeam { mailboxes: &failed, pass: Some(&pass), window_open: false });
    assert!(s.steps.contains(&("mail (skipped: could not read your mailboxes)".to_string(), 0)), "{:?}", s.steps);
    assert!(s.engine_ok && s.ok, "a skip is not a failure: {:?}", s.steps);
    let _ = std::fs::remove_dir_all(&cs.data_dir);
    let _ = std::fs::remove_dir_all(&v);
    let _ = std::fs::remove_dir_all(&d);
}

/// Checkpoint D: the pass names why it did not run. The mail window taken meanwhile is `mail window open`;
/// the account's client that cannot be had (the session gone between the list and the pass) is `no session`,
/// never the window, so nobody looks for a window that is not there. The window is held for the whole run.
#[test]
fn a_pass_whose_client_cannot_be_had_is_no_session_never_the_window() {
    use knowlu::scheduler::mail_pass;
    struct Hold<'a>(&'a Cell<bool>);
    impl Drop for Hold<'_> {
        fn drop(&mut self) {
            self.0.set(true);
        }
    }
    let ran = Cell::new(0);
    let run = |_: &()| {
        ran.set(ran.get() + 1);
        Some("mail: forwarding on, 0 recovered".to_string())
    };
    assert_eq!(mail_pass(|| Some(()), || Err::<(), _>("no account"), run), Err("no session"));
    let asked = Cell::new(false);
    let client = || {
        asked.set(true);
        Ok::<(), ()>(())
    };
    assert_eq!(mail_pass(|| None::<()>, client, run), Err("mail window open"));
    assert!(!asked.get(), "with the window taken the client is never asked");
    assert_eq!(ran.get(), 0, "neither ran the pass");
    let released = Cell::new(false);
    let held = |_: &()| {
        assert!(!released.get(), "the run holds the mail window");
        None
    };
    assert_eq!(mail_pass(|| Some(Hold(&released)), || Ok::<(), ()>(()), held), Ok("mail: nothing to check".to_string()));
    assert!(released.get(), "and lets it go when it is done");

    // Through the step, each is its own named skip.
    let d = mail_dir("pass-reason");
    let rows = || Ok(vec![rig::row("active", "m365")]);
    let no_session = |_: &[Value]| mail_pass(|| Some(()), || Err::<(), _>("no account"), |_: &()| Some("mail: ran".to_string()));
    let taken = |_: &[Value]| mail_pass(|| None::<()>, || Ok::<(), ()>(()), |_: &()| Some("mail: ran".to_string()));
    let step = |pass: &dyn Fn(&[Value]) -> Result<String, &'static str>| mail_step(EntitlementState::Entitled, &d, rig::NOW, &MailSeam { mailboxes: &rows, pass: Some(pass), window_open: false });
    assert_eq!(step(&no_session).as_deref(), Some("mail (skipped: no session)"));
    assert_eq!(step(&taken).as_deref(), Some("mail (skipped: mail window open)"));
    let _ = std::fs::remove_dir_all(&d);
}

/// A slot on the stand-in engine (`cmd`) and a throwaway `LOCALAPPDATA`, as the grades slot test runs one.
fn run_mail_slot(cs: &ConsoleState, sch: &Scheduler, mail: &MailSeam) -> knowlu::scheduler::RunSummary {
    let fake = std::env::temp_dir().join(format!("qo-sched-mail-localappdata-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&fake);
    std::fs::create_dir_all(&fake).unwrap();
    let _guard = ENGINE_ENV_LOCK.lock().unwrap_or_else(|e| e.into_inner());
    let _env = EnvSeam::set(&[("KNOWLU_ENGINE_EXE", std::ffi::OsStr::new("cmd")), ("LOCALAPPDATA", fake.as_os_str())]);
    let nothing = GradesSeam { row: None, capture: None, window_open: false };
    let s = run_slot_with(cs, sch, None, false, &nothing, mail);
    let _ = std::fs::remove_dir_all(&fake);
    s
}

#[test]
fn reconcile_follows_reconcile_cadence_in_both_modes() {
    assert_eq!(RECONCILE_CADENCE, Cadence::Daily, "the shipped value, until S4's revisit changes only the value");
    let ago = |h: i64| Some(rig::iso(rig::NOW - h * 3600));
    let due = |mode, last: Option<String>, status: &str| reconcile_due_with(mode, last.as_deref(), status, rig::NOW);
    // Daily: an active mailbox at most once a day.
    assert!(due(Cadence::Daily, None, "active"), "never reconciled");
    assert!(!due(Cadence::Daily, ago(1), "active"));
    assert!(!due(Cadence::Daily, ago(19), "active"));
    assert!(due(Cadence::Daily, ago(21), "active"), "a day later");
    // Daily still repairs a mailbox that needs it at the next slot (spec 4.10), however recent the last pass.
    for status in ["quiet", "off", "lapsed"] {
        assert!(due(Cadence::Daily, ago(1), status), "{status}");
    }
    // RepairOnly: only a mailbox that needs repair, and an active one never.
    assert!(!due(Cadence::RepairOnly, None, "active"));
    assert!(!due(Cadence::RepairOnly, ago(500), "active"));
    for status in ["quiet", "off", "lapsed"] {
        assert!(due(Cadence::RepairOnly, ago(1), status), "{status}");
    }
    // A mailbox still in setup (or blocked, or paused by a refused send) is the setup's, never the slot's.
    for mode in [Cadence::Daily, Cadence::RepairOnly] {
        for status in ["pending", "confirmed", "blocked", "paused_send"] {
            assert!(!due(mode, ago(1), status), "{status}");
        }
    }
    // The slot hands the pass only the mailboxes that are due.
    let d = mail_dir("cadence");
    MailState::update(&d, |s| {
        s.last_reconcile.insert(rig::ID.to_string(), rig::iso(rig::NOW - 3600));
        true
    })
    .unwrap();
    let rows = || Ok(vec![rig::row("active", "m365"), second_row()]);
    let seen = std::cell::RefCell::new(Vec::new());
    let pass = |due: &[Value]| {
        seen.borrow_mut().extend(due.iter().map(|r| r["address_id"].as_str().unwrap().to_string()));
        Ok("mail: ran".to_string())
    };
    let seam = MailSeam { mailboxes: &rows, pass: Some(&pass), window_open: false };
    assert!(mail_step(EntitlementState::Entitled, &d, rig::NOW, &seam).is_some());
    assert_eq!(*seen.borrow(), vec!["0b0e5b2c-2222-4aaa-8bbb-000000000002".to_string()]);
    let _ = std::fs::remove_dir_all(&d);
}

#[test]
fn slot_argv_is_unchanged() {
    let v = scratch("argv-unchanged");
    let exe = Path::new(r"C:\bin\knowlu-engine.exe");
    let vault = v.to_string_lossy().to_string();
    let bundle = PathBuf::from(r"C:\data\grades-capture.json");
    // The six steps, in the order and with the arguments they had before the mail step existed.
    let argv = slot_argv(&v, exe, &JudgePlan::Cloud { log_dir: PathBuf::from("logs") }, Some(&bundle));
    let first: Vec<&str> = argv.iter().map(|(_, a)| a[0].as_str()).collect();
    assert_eq!(first, ["sync", "coursework", "grades", "judge", "rank"], "no feed on this vault, so no ingest");
    assert_eq!(argv[0].1, ["sync", "--vault", vault.as_str(), "--via", "local-runner"]);
    assert_eq!(argv[1].1, ["coursework", "--vault", vault.as_str(), "--via", "local-runner"]);
    assert_eq!(argv[4].1, ["rank", "--vault", vault.as_str(), "--runner", "local"]);
    assert!(argv.iter().all(|(e, a)| e == exe && !a[0].contains("mail")), "the mail step is the app's, never a child: {argv:?}");
    let _ = std::fs::remove_dir_all(&v);
}

#[test]
fn the_mail_step_runs_after_the_grades_capture_and_before_sync() {
    let (v, cs) = app_slot_vault("mail-order");
    std::fs::create_dir_all(grades_app::session_dir(&cs.data_dir)).unwrap();
    std::fs::write(v.join("config").join("campus.yaml"), "unitid: '999999'\r\nlms: 'blackboard'\r\n").unwrap();
    let events = std::cell::RefCell::new(Vec::new());
    let capture = |_h: &'static str| {
        events.borrow_mut().push("grades capture");
        Ok(bundle())
    };
    let dated = test_row("blackboard", Some("2026-10-01"));
    let rows = || Ok(vec![rig::row("active", "m365")]);
    let pass = |_: &[Value]| {
        events.borrow_mut().push("mail");
        Ok("mail: forwarding on, 0 recovered".to_string())
    };
    let sch = Scheduler::default();
    let fake = std::env::temp_dir().join(format!("qo-sched-mail-order-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&fake);
    std::fs::create_dir_all(&fake).unwrap();
    let _guard = ENGINE_ENV_LOCK.lock().unwrap_or_else(|e| e.into_inner());
    let _env = EnvSeam::set(&[("KNOWLU_ENGINE_EXE", std::ffi::OsStr::new("cmd")), ("LOCALAPPDATA", fake.as_os_str())]);
    let grades = GradesSeam { row: Some(&dated), capture: Some(&capture), window_open: false };
    let s = run_slot_with(&cs, &sch, None, false, &grades, &MailSeam { mailboxes: &rows, pass: Some(&pass), window_open: false });
    assert_eq!(*events.borrow(), ["grades capture", "mail"], "the capture first, then the mail pass");
    let at = |name: &str| s.steps.iter().position(|(n, _)| n == name).unwrap_or_else(|| panic!("{name} in {:?}", s.steps));
    let (mail, sync) = (at("mail: forwarding on, 0 recovered"), at("sync"));
    assert!(mail < sync, "before the child-process loop starts with sync: {:?}", s.steps);
    assert!(s.steps.iter().all(|(n, c)| !n.starts_with("mail") || *c == 0));
    let _ = std::fs::remove_dir_all(&fake);
    let _ = std::fs::remove_dir_all(&cs.data_dir);
    let _ = std::fs::remove_dir_all(&v);
}

#[test]
fn no_mailbox_records_no_mail_row_and_no_mail_step_name_reaches_the_runner_log() {
    let (v, cs) = app_slot_vault("mail-log");
    let sch = Scheduler::default();
    let mail_rows = |s: &knowlu::scheduler::RunSummary| s.steps.iter().filter(|(n, _)| n.starts_with("mail")).count();
    let pass = |_: &[Value]| Ok("mail (needs sign-in)".to_string());
    // No mailbox connected: nothing is asked of the window and no row is added, whatever the service listed.
    let none = || Ok(Vec::new());
    let revoked = || Ok(vec![rig::row("revoked", "m365")]);
    for mailboxes in [&none as &dyn Fn() -> Result<Vec<Value>, &'static str>, &revoked] {
        let s = run_mail_slot(&cs, &sch, &MailSeam { mailboxes, pass: Some(&pass), window_open: false });
        assert_eq!(mail_rows(&s), 0, "{:?}", s.steps);
    }
    // A list the service could not give is one named row (Checkpoint D), and no line in the runner log either.
    let unreadable = || Err("server");
    let mut summary = run_mail_slot(&cs, &sch, &MailSeam { mailboxes: &unreadable, pass: Some(&pass), window_open: false });
    assert_eq!(mail_rows(&summary), 1, "{:?}", summary.steps);
    // One connected: every named outcome is a row in the summary, and none is a line in the runner log.
    MailState::update(&cs.data_dir, |s| {
        s.keep_session = false;
        true
    })
    .unwrap();
    let one = || Ok(vec![rig::row("active", "m365")]);
    let s = run_mail_slot(&cs, &sch, &MailSeam { mailboxes: &one, pass: Some(&pass), window_open: false });
    assert_eq!(mail_rows(&s), 1, "{:?}", s.steps);
    summary.steps.extend(s.steps);
    MailState::update(&cs.data_dir, |s| {
        s.keep_session = true;
        true
    })
    .unwrap();
    for window_open in [false, true] {
        let s = run_mail_slot(&cs, &sch, &MailSeam { mailboxes: &one, pass: Some(&pass), window_open });
        assert_eq!(mail_rows(&s), 1);
        summary.steps.extend(s.steps);
    }
    for row in ["mail (needs sign-in)", "mail (skipped: saved sign-in off)", "mail (skipped: could not read your mailboxes)"] {
        assert!(summary.steps.iter().any(|(n, _)| n == row), "{row} in {:?}", summary.steps);
    }
    let log = knowlu_engine::pystr::read_text(&v.join("state").join("runner-log.md")).unwrap_or_default();
    assert!(!log.lines().any(|l| l.contains("local ok mail") || l.contains("mail (") || l.contains("mail:")), "a mail step reached the runner log:\n{log}");
    let _ = std::fs::remove_dir_all(&cs.data_dir);
    let _ = std::fs::remove_dir_all(&v);
}

#[test]
fn the_grades_capture_and_the_mail_step_never_run_at_once() {
    let (v, cs) = app_slot_vault("mail-once");
    std::fs::create_dir_all(grades_app::session_dir(&cs.data_dir)).unwrap();
    std::fs::write(v.join("config").join("campus.yaml"), "unitid: '999999'\r\nlms: 'blackboard'\r\n").unwrap();
    let busy = Cell::new(false);
    let overlaps = Cell::new(0);
    let runs = Cell::new(0);
    let hold = |who: &Cell<u32>| {
        if busy.replace(true) {
            overlaps.set(overlaps.get() + 1);
        }
        who.set(who.get() + 1);
        std::thread::sleep(Duration::from_millis(30));
        busy.set(false);
    };
    let grades_ran = Cell::new(0);
    let capture = |_h: &'static str| {
        hold(&grades_ran);
        Ok(bundle())
    };
    let rows = || Ok(vec![rig::row("active", "m365")]);
    let pass = |_: &[Value]| {
        hold(&runs);
        Ok("mail: forwarding on, 0 recovered".to_string())
    };
    let dated = test_row("blackboard", Some("2026-10-01"));
    let sch = Scheduler::default();
    let fake = std::env::temp_dir().join(format!("qo-sched-mail-once-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&fake);
    std::fs::create_dir_all(&fake).unwrap();
    let _guard = ENGINE_ENV_LOCK.lock().unwrap_or_else(|e| e.into_inner());
    let _env = EnvSeam::set(&[("KNOWLU_ENGINE_EXE", std::ffi::OsStr::new("cmd")), ("LOCALAPPDATA", fake.as_os_str())]);
    let grades = GradesSeam { row: Some(&dated), capture: Some(&capture), window_open: false };
    let _ = run_slot_with(&cs, &sch, None, false, &grades, &MailSeam { mailboxes: &rows, pass: Some(&pass), window_open: false });
    assert_eq!((grades_ran.get(), runs.get(), overlaps.get()), (1, 1, 0), "one after the other, never at once");
    // And the mail window's own hold is one at a time: while a slot's pass holds it, nothing else takes it.
    let held = knowlu::mail::MailWindow::try_take().expect("free");
    assert!(knowlu::mail::MailWindow::try_take().is_none());
    drop(held);
    assert!(knowlu::mail::MailWindow::try_take().is_some());
    let _ = std::fs::remove_dir_all(&fake);
    let _ = std::fs::remove_dir_all(&cs.data_dir);
    let _ = std::fs::remove_dir_all(&v);
}

/// One mailbox's pass over the fake window and the loopback service: the line it reports, and the rig.
struct Pass {
    d: PathBuf,
    svc: rig::Svc,
    host: rig::FakeHost,
    clk: rig::Clk,
}

impl Pass {
    /// A mailbox with `status` whose forwarding page reads `fwd`, and a saved sign-in on disk.
    fn new(tag: &str, status: &str, fwd: Value) -> Pass {
        Pass::with_provider(tag, "google", status, fwd)
    }

    fn with_provider(tag: &str, provider: &str, status: &str, fwd: Value) -> Pass {
        let d = mail_dir(tag);
        std::fs::create_dir_all(knowlu::mail_session::session_dir(&d, rig::ID).unwrap()).unwrap();
        Pass { d, svc: rig::Svc::start(vec![rig::row(status, provider)]), host: rig::FakeHost::new(fwd), clk: rig::Clk::at(rig::NOW) }
    }

    fn rows(&self) -> Vec<Value> {
        self.svc.client().list_addresses().unwrap()
    }

    fn run(&self) -> Option<String> {
        let client = self.svc.client();
        run_step(&SlotEnv { host: &self.host, client: &client, clock: &self.clk, data_dir: &self.d }, &self.rows())
    }

    fn last_reconcile(&self, secs: i64) {
        MailState::update(&self.d, |s| {
            s.last_reconcile.insert(rig::ID.to_string(), rig::iso(secs));
            true
        })
        .unwrap();
    }

    fn mail(&self, msgs: &[(&str, i64)]) {
        self.host.st.lock().unwrap().msgs = msgs.iter().map(|(subject, at)| ("ana.lopez@school.example.invalid".to_string(), subject.to_string(), *at)).collect();
    }

    fn at(&self, call: &str) -> usize {
        let calls = self.svc.calls();
        calls.iter().position(|c| c == call).unwrap_or_else(|| panic!("{call} in {calls:?}"))
    }
}

impl Drop for Pass {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.d);
    }
}

#[test]
fn a_forwarding_found_off_is_turned_back_on_and_reported_reenabled() {
    let p = Pass::new("reenable", "quiet", rig::fwd("off", "none"));
    p.last_reconcile(rig::NOW - 10 * rig::DAY);
    // One message from eight days ago (the forwarding was off since) and one from the last hour.
    p.mail(&[("Older notice", rig::NOW - 8 * rig::DAY), ("Newer notice", rig::NOW - 3600)]);
    assert_eq!(p.run().as_deref(), Some("mail: forwarding was off, turned back on, 2 recovered"));
    let scripts = p.host.scripts();
    assert_eq!(&scripts[..3], ["gmail/forwarding_read.js", "gmail/forwarding_set.js", "gmail/forwarding_read.js"], "{scripts:?}");
    assert_eq!(p.host.calls("gmail/forwarding_set.js"), [json!({ "address": rig::KNOWLU, "replace": false })], "never replaces another address");
    let id = rig::ID;
    let (off, canary, reenabled) = (p.at(&format!("POST /addresses/{id}/state off")), p.at("POST /canary"), p.at(&format!("POST /addresses/{id}/state reenabled")));
    assert!(off < canary && canary < reenabled, "{:?}", p.svc.calls());
    assert!(p.at("POST /reconcile") > reenabled, "the reconcile follows the repair");
    // It reaches back to when forwarding was last known on, not three days.
    assert_eq!(p.svc.bodies("/reconcile")[0]["since"], json!(rig::iso(rig::NOW - 10 * rig::DAY)));
    assert!(p.host.opens().iter().all(|(_, visible)| !visible) && p.host.st.lock().unwrap().back == 0, "no window is shown");
    assert!(p.host.st.lock().unwrap().closed >= 1);
    assert_eq!(MailState::load(&p.d).unwrap().last_reconcile[rig::ID], rig::iso(rig::NOW), "a whole pass is the next last-known-on");
}

#[test]
fn a_microsoft_forwarding_found_off_is_turned_back_on_and_reported_reenabled_with_no_recover_pass() {
    // D13: Microsoft's forwarding repair is not retired and still runs; its recover pass is held while C3 is built.
    let p = Pass::with_provider("reenable-m365", "m365", "quiet", rig::fwd("off", "none"));
    p.last_reconcile(rig::NOW - 10 * rig::DAY);
    p.mail(&[("Older notice", rig::NOW - 8 * rig::DAY), ("Newer notice", rig::NOW - 3600)]);
    assert_eq!(p.run().as_deref(), Some("mail: forwarding was off, turned back on"));
    assert_eq!(p.host.scripts(), ["m365/forwarding_read.js", "m365/forwarding_set.js", "m365/forwarding_read.js"], "nothing else ran");
    for retired in ["m365/search.js", "m365/list.js", "m365/forward.js", "m365/sent_cleanup.js"] {
        assert!(p.host.calls(retired).is_empty(), "{retired} ran");
    }
    assert_eq!(p.host.calls("m365/forwarding_set.js"), [json!({ "address": rig::KNOWLU, "replace": false })], "never replaces another address");
    let id = rig::ID;
    let (off, canary, reenabled) = (p.at(&format!("POST /addresses/{id}/state off")), p.at("POST /canary"), p.at(&format!("POST /addresses/{id}/state reenabled")));
    assert!(off < canary && canary < reenabled, "{:?}", p.svc.calls());
    assert!(!p.svc.calls().iter().any(|c| c.contains("reconcile") || c.contains("batch")), "{:?}", p.svc.calls());
    assert_eq!(MailState::load(&p.d).unwrap().last_reconcile[rig::ID], rig::iso(rig::NOW - 10 * rig::DAY), "no pass was made, so none is recorded");
    assert!(p.host.opens().iter().all(|(_, visible)| !visible) && p.host.st.lock().unwrap().back == 0, "no window is shown");
}

#[test]
fn a_failed_forwarding_read_never_moves_last_known_on_forward() {
    let p = Pass::new("unchecked", "quiet", json!({ "result": "error" }));
    p.last_reconcile(rig::NOW - 10 * rig::DAY);
    p.mail(&[("Newer notice", rig::NOW - 3600)]);
    assert_eq!(p.run().as_deref(), Some("mail: forwarding not checked, 1 recovered"));
    assert_eq!(p.svc.bodies("/reconcile")[0]["since"], json!(rig::iso(rig::NOW - 3 * rig::DAY)));
    assert_eq!(MailState::load(&p.d).unwrap().last_reconcile[rig::ID], rig::iso(rig::NOW - 10 * rig::DAY), "an unconfirmed forwarding is not a last-known-on");
}

#[test]
fn recovery_reaches_back_three_days_when_forwarding_is_on() {
    let p = Pass::new("on", "active", rig::fwd("on", "ours"));
    p.mail(&[("Older notice", rig::NOW - 8 * rig::DAY), ("Newer notice", rig::NOW - 3600)]);
    assert_eq!(p.run().as_deref(), Some("mail: forwarding on, 1 recovered"));
    assert_eq!(p.host.calls("gmail/forwarding_set.js").len(), 0);
    assert!(!p.svc.calls().iter().any(|c| c.contains("state") || c.contains("canary")), "nothing to report: {:?}", p.svc.calls());
    assert_eq!(p.svc.bodies("/reconcile")[0]["since"], json!(rig::iso(rig::NOW - 3 * rig::DAY)));
}

// ---- spec §4.11 step 3, amended 2026-10-19: a whole pass over a lapse's gap reports it `covered` ---------------

/// The stamps as the service answers them, with microseconds and an offset, so a reparse would show. `SINCE` is
/// ten days before the rig's `NOW`.
const SINCE: &str = "2026-09-11T09:13:20.123456-05:00";
const LAST: &str = "2026-09-13T00:00:00.654321+00:00";

impl Pass {
    /// A Gmail mailbox with `status`, forwarding on, whose lapse the service stamped `since` and `last`.
    fn lapsed(tag: &str, status: &str, since: &str, last: &str) -> Pass {
        let p = Pass::new(tag, status, rig::fwd("on", "ours"));
        {
            let mut st = p.svc.st.lock().unwrap();
            st.rows[0]["lapsed_since"] = json!(since);
            st.rows[0]["last_lapsed_at"] = json!(last);
        }
        p
    }

    /// The bodies of every `covered` report sent.
    fn covered(&self) -> Vec<Value> {
        self.svc.bodies(&format!("/addresses/{}/state", rig::ID)).into_iter().filter(|b| b["state"] == "covered").collect()
    }
}

#[test]
fn a_whole_pass_over_the_gap_reports_it_covered_with_the_stamps_read_before_the_pass() {
    let p = Pass::lapsed("covered", "quiet", SINCE, LAST);
    p.mail(&[("Dropped while lapsed", rig::NOW - 8 * rig::DAY), ("Newer notice", rig::NOW - 3600)]);
    let (client, rows) = (p.svc.client(), p.rows());
    // A lapse while the pass runs moves the service's stamps on: the report still carries the ones read before it.
    p.svc.st.lock().unwrap().rows[0]["last_lapsed_at"] = json!("2026-09-21T12:00:00Z");
    let line = run_step(&SlotEnv { host: &p.host, client: &client, clock: &p.clk, data_dir: &p.d }, &rows);
    assert_eq!(line.as_deref(), Some("mail: forwarding on, 2 recovered"));
    assert_eq!(p.svc.bodies("/reconcile")[0]["since"], json!(rig::iso(rig::NOW - 10 * rig::DAY)), "the window reached the gap");
    assert_eq!(p.covered(), [json!({ "state": "covered", "lapsed_since": SINCE, "last_lapsed_at": LAST })], "verbatim, once");
    let state = &p.svc.bodies(&format!("/addresses/{}/state", rig::ID))[0];
    assert!(state.to_string().contains(SINCE) && state.to_string().contains(LAST), "byte for byte: {state}");
    assert!(p.at(&format!("POST /addresses/{}/state covered", rig::ID)) > p.at("POST /batches/batch-1/sent"), "after the batch closed");
    assert_eq!(p.svc.calls().iter().filter(|c| *c == "GET /addresses").count(), 1, "never a later read: {:?}", p.svc.calls());
}

#[test]
fn no_covered_report_after_a_pass_that_stopped_early_fell_short_of_the_gap_or_left_a_row() {
    let at = rig::NOW - 3_600 - rig::NOW.rem_euclid(60);
    // Stopped early: the only missing row will not attach.
    let p = Pass::lapsed("covered-stopped", "quiet", SINCE, LAST);
    p.mail(&[("Stuck", at)]);
    p.host.st.lock().unwrap().no_chip = Some("Stuck".into());
    assert!(p.run().unwrap().ends_with("stopped: the attachments would not attach"));
    assert!(p.covered().is_empty(), "{:?}", p.svc.calls());
    // Ended done, but a row was left (kept as left from its third stalled day): recover did not send all it found missing.
    let q = Pass::lapsed("covered-left", "quiet", SINCE, LAST);
    q.mail(&[("Stuck", at)]);
    q.host.st.lock().unwrap().no_chip = Some("Stuck".into());
    MailState::update(&q.d, |s| s.lone_left.insert(rig::ID.to_string(), vec![at]).is_none()).unwrap();
    assert_eq!(q.run().as_deref(), Some("mail: forwarding on, 0 recovered"));
    assert!(q.covered().is_empty(), "{:?}", q.svc.calls());
    // A stamp the device cannot read as an instant: the window is the usual three days, short of any gap.
    let r = Pass::lapsed("covered-short", "quiet", "not an instant", LAST);
    r.mail(&[("Newer notice", rig::NOW - 3600)]);
    assert_eq!(r.run().as_deref(), Some("mail: forwarding on, 1 recovered"));
    assert_eq!(r.svc.bodies("/reconcile")[0]["since"], json!(rig::iso(rig::NOW - 3 * rig::DAY)));
    assert!(r.covered().is_empty(), "{:?}", r.svc.calls());
}

#[test]
fn no_covered_report_while_the_mailbox_is_still_lapsed() {
    let p = Pass::lapsed("covered-still-lapsed", "lapsed", SINCE, LAST);
    p.mail(&[("Dropped while lapsed", rig::NOW - 8 * rig::DAY)]);
    assert_eq!(p.run().as_deref(), Some("mail: forwarding on, 1 recovered"));
    assert_eq!(p.svc.bodies("/reconcile")[0]["since"], json!(rig::iso(rig::NOW - 10 * rig::DAY)), "the gap is still recovered");
    assert!(p.covered().is_empty(), "{:?}", p.svc.calls());
}

#[test]
fn a_refused_failed_or_uncleared_covered_report_leaves_the_slot_as_it_was() {
    let run = |tag: &str, reply: Option<(u16, String)>| {
        let p = Pass::lapsed(tag, "quiet", SINCE, LAST);
        p.mail(&[("Dropped while lapsed", rig::NOW - 8 * rig::DAY)]);
        p.svc.st.lock().unwrap().covered = reply;
        let line = p.run();
        assert_eq!(p.covered().len(), 1, "{tag}: sent once, never retried in the slot");
        (line, MailState::load(&p.d).unwrap())
    };
    let cleared = run("covered-ok", None);
    assert_eq!(cleared.0.as_deref(), Some("mail: forwarding on, 1 recovered"));
    let uncleared = json!({ "address_id": rig::ID, "state": "covered", "status": "quiet", "cleared": false }).to_string();
    for (tag, reply) in [
        ("covered-400", (400, r#"{"error":"lapsed_since and last_lapsed_at must be the mailbox's own, each an instant with its zone"}"#.to_string())),
        ("covered-500", (500, r#"{"error":"failed"}"#.to_string())),
        ("covered-false", (200, uncleared.clone())),
    ] {
        assert_eq!(run(tag, Some(reply)), cleared, "{tag}: the same line and the same mail.json");
    }
}

#[test]
fn a_pass_covers_the_gap_only_when_done_whole_and_reaching_back_to_it() {
    use knowlu::mail::recover::{window, Outcome, Stop, Window};
    use knowlu::mail::slot::covers_gap;
    let (now, l) = (rig::NOW, rig::NOW - 10 * rig::DAY);
    let w = window(now, Some(l), None);
    let done = Outcome { stop: Stop::Done, expected: 2, handled: 2, held: 0, sent: 2, left: 0, cleaned: 2, closed: true, frontier: None, stalled_at: None, stuck_cleared: false, left_cleared: vec![], left_at_once: vec![] };
    assert!(covers_gap(&done, w, l, now));
    let old = now - 90 * rig::DAY;
    assert!(covers_gap(&done, window(now, Some(old), None), old, now), "30 days back is enough for an older lapse");
    assert!(!covers_gap(&done, Window { since: l + 1, ..w }, l, now), "a second short of the gap");
    assert!(!covers_gap(&done, window(now, None, None), l, now), "the usual three days");
    assert!(!covers_gap(&Outcome { stop: Stop::SlotCap, ..done.clone() }, w, l, now));
    assert!(!covers_gap(&Outcome { left: 1, ..done.clone() }, w, l, now));
    assert!(!covers_gap(&Outcome { closed: false, ..done.clone() }, w, l, now), "a batch not closed with its count");
    assert!(covers_gap(&Outcome { sent: 0, cleaned: 0, closed: false, ..done.clone() }, w, l, now), "nothing missing, no batch");
}

#[test]
fn a_turned_off_mailbox_is_never_turned_back_on_even_after_a_failed_disconnect() {
    use knowlu::mail::commands::{disconnect_with, Disconnect};
    let p = Pass::with_provider("turnedoff", "m365", "off", rig::fwd("on", "ours"));
    p.svc.st.lock().unwrap().delete_status = 500;
    let client = p.svc.client();
    // Disconnect: step 0 records the mailbox, forwarding goes off, and the server call then fails.
    let out = disconnect_with(&Disconnect { host: &p.host, client: &client, clock: &p.clk, data_dir: &p.d }, rig::ID);
    assert_eq!(out["ok"], json!(false), "{out}");
    assert!(MailState::load(&p.d).unwrap().is_turned_off(rig::ID));
    assert_eq!(p.host.st.lock().unwrap().fwd["forwarding"], json!("off"));
    let (scripts, opens, calls) = (p.host.scripts(), p.host.opens(), p.svc.calls());
    // The mailbox is still listed, `off`, with a kept profile: the slot never touches it.
    let rows = || client.list_addresses().map_err(|e| e.name());
    let ran = Cell::new(0);
    let pass = |due: &[Value]| {
        ran.set(ran.get() + 1);
        run_step(&SlotEnv { host: &p.host, client: &client, clock: &p.clk, data_dir: &p.d }, due).ok_or("nothing ran")
    };
    let seam = MailSeam { mailboxes: &rows, pass: Some(&pass), window_open: false };
    assert_eq!(mail_step(EntitlementState::Entitled, &p.d, rig::NOW, &seam), None, "no row for a mailbox the student turned off");
    assert_eq!(ran.get(), 0);
    // Even handed the row directly, the pass opens nothing and runs nothing.
    assert_eq!(p.run(), None);
    assert_eq!((p.host.scripts(), p.host.opens()), (scripts, opens), "no window, no script");
    assert!(p.svc.calls()[calls.len()..].iter().all(|c| c == "GET /addresses"), "{:?}", p.svc.calls());
    assert_eq!(p.host.st.lock().unwrap().fwd["forwarding"], json!("off"));
}

#[test]
fn a_forwarding_pointing_elsewhere_is_left_unchanged_and_asked_about() {
    let p = Pass::new("elsewhere", "quiet", rig::fwd("on", "elsewhere"));
    assert_eq!(p.run().as_deref(), Some("mail: forwarding goes to another address, 0 recovered"));
    assert!(!p.host.scripts().iter().any(|s| s.contains("forwarding_set") || s.contains("forwarding_off")), "{:?}", p.host.scripts());
    let id = rig::ID;
    p.at(&format!("POST /addresses/{id}/state elsewhere"));
    assert!(!p.svc.calls().iter().any(|c| c.contains("reenabled") || c.contains("canary")), "{:?}", p.svc.calls());
    // Off with another address typed in is the same: never replaced in a hidden step.
    let q = Pass::new("elsewhere-off", "off", rig::fwd("off", "elsewhere"));
    assert_eq!(q.run().as_deref(), Some("mail: forwarding goes to another address, 0 recovered"));
    assert!(!q.host.scripts().iter().any(|s| s.contains("forwarding_set")));
}

#[test]
fn a_signed_out_profile_records_needs_sign_in_and_shows_no_window() {
    let p = Pass::new("signedout", "active", rig::fwd("on", "ours"));
    p.host.st.lock().unwrap().signed_in = false;
    assert_eq!(p.run().as_deref(), Some("mail (needs sign-in)"));
    let opens = p.host.opens();
    assert!(!opens.is_empty() && opens.iter().all(|(_, visible)| !visible), "the window is opened hidden: {opens:?}");
    assert_eq!(p.host.st.lock().unwrap().back, 0, "and never brought to the front");
    assert!(p.host.scripts().is_empty(), "no script ran on a signed-out page");
    assert!(p.svc.calls().iter().all(|c| c == "GET /addresses"), "nothing else was asked of the service: {:?}", p.svc.calls());
    assert!(!p.host.st.lock().unwrap().window, "the window is closed again");
    // A mailbox with no saved sign-in on this device (connected elsewhere) opens no window at all.
    let q = Pass::new("nodevice", "active", rig::fwd("on", "ours"));
    std::fs::remove_dir_all(knowlu::mail_session::session_dir(&q.d, rig::ID).unwrap()).unwrap();
    assert_eq!(q.run().as_deref(), Some("mail (needs sign-in)"));
    assert!(q.host.opens().is_empty());
}

#[test]
fn a_microsoft_mailbox_is_never_recovered_while_c3_is_built_and_a_gmail_one_still_is() {
    // D13 and C3 (Quinn, 2026-10-11 and 2026-10-13): no Microsoft path searches, lists, forwards or cleans
    // Sent. The forwarding check is not retired, so it still runs; the recover pass after it does not,
    // whether or not a backfill is open for the mailbox.
    let m = Pass::with_provider("d13-slot-m365", "m365", "quiet", rig::fwd("on", "ours"));
    m.mail(&[("Older notice", rig::NOW - 8 * rig::DAY), ("Newer notice", rig::NOW - 3600)]);
    MailState::update(&m.d, |s| {
        s.open_backfills.push(rig::ID.to_string());
        true
    })
    .unwrap();
    assert_eq!(m.run().as_deref(), Some("mail: forwarding on"));
    assert_eq!(m.host.scripts(), ["m365/forwarding_read.js"], "only the forwarding check ran");
    assert!(!m.svc.calls().iter().any(|c| c.contains("reconcile") || c.contains("batch")), "{:?}", m.svc.calls());
    assert!(MailState::load(&m.d).unwrap().last_reconcile.is_empty(), "no pass was made, so none is recorded");
    assert_eq!(MailState::load(&m.d).unwrap().open_backfills, [rig::ID], "an open backfill stays open for C3");
    // Gmail's recover is unchanged.
    let g = Pass::with_provider("d13-slot-gmail", "google", "quiet", rig::fwd("on", "ours"));
    g.mail(&[("Newer notice", rig::NOW - 3600)]);
    assert_eq!(g.run().as_deref(), Some("mail: forwarding on, 1 recovered"));
    assert!(g.svc.calls().iter().any(|c| c.contains("reconcile")));
}

#[test]
fn a_report_that_lands_after_one_slots_last_look_stops_the_next_slot_at_its_first_look() {
    // Slots run about 11 and 13 hours apart, each with a Routine seeded from `mail.json`: the look reads back to
    // the last one saved, not to a fixed number of hours before the batch.
    let g = Pass::with_provider("last-look-slots", "google", "quiet", rig::fwd("on", "ours"));
    g.mail(&[("Notice", rig::NOW - 3600)]);
    assert_eq!(g.run().as_deref(), Some("mail: forwarding on, 1 recovered"));
    // A mailbox with no saved look reaches back a day before its batch opened.
    assert_eq!(g.host.calls("gmail/bounce.js")[0]["since"], json!(rig::iso(rig::NOW - 24 * 3600)));
    // The final look waits out Gmail's Undo Send delay after the forward (31 s), whatever ends the pass.
    assert_eq!(MailState::load(&g.d).unwrap().last_look.get(rig::ID), Some(&(rig::NOW + 31)), "the final look was saved");
    // A report lands an hour later; the next slot is 13 hours after the first.
    g.host.st.lock().unwrap().report_at = Some(rig::NOW + 3600);
    g.clk.0.store(rig::NOW + 13 * 3600, std::sync::atomic::Ordering::SeqCst);
    let line = g.run().expect("a line");
    assert!(line.contains("paused: a send was refused"), "{line}");
    assert_eq!(g.host.calls("gmail/bounce.js")[1]["since"], json!(rig::iso(rig::NOW + 31)), "it read from the saved look");
    assert_eq!(MailState::load(&g.d).unwrap().last_look.get(rig::ID), Some(&(rig::NOW + 13 * 3600 + 31)), "a refused look is saved too");
}

fn pace(d: &Path) -> Option<knowlu::mail_session::BackfillPace> {
    MailState::load(d).unwrap().backfill_pace.get(rig::ID).cloned()
}

#[test]
fn a_slot_pass_keeps_its_sends_beside_the_backfill_pace_so_the_day_never_passes_600() {
    // The bug the 2026-10-19 design named: a slot pass seeded its pace from mail.json but never saved it, so a
    // backfill resumed later the same UTC day started from a count without the slot's sends.
    let g = Pass::with_provider("slot-keeps-pace", "google", "active", rig::fwd("on", "ours"));
    g.mail(&[("Notice", rig::NOW - 3600)]);
    assert_eq!(g.run().as_deref(), Some("mail: forwarding on, 1 recovered"));
    let kept = pace(&g.d).expect("the slot's sends are kept");
    assert_eq!((kept.day, kept.sent_today), (rig::NOW.div_euclid(rig::DAY), 1));
    // A backfill's own count for the day is kept and added to, never lowered.
    let b = Pass::with_provider("slot-adds-to-pace", "google", "active", rig::fwd("on", "ours"));
    b.mail(&[("Notice", rig::NOW - 3600)]);
    MailState::update(&b.d, |s| {
        s.open_backfills.push(rig::ID.to_string());
        let day = rig::NOW.div_euclid(rig::DAY);
        s.backfill_pace.insert(rig::ID.to_string(), knowlu::mail_session::BackfillPace { day, sent_today: 41, paused_until: None, loads_today: 7, messages_today: 0 });
        true
    })
    .unwrap();
    assert_eq!(b.run().as_deref(), Some("mail: forwarding on, 1 recovered"));
    let kept = pace(&b.d).unwrap();
    assert_eq!(kept.sent_today, 42);
    assert!(kept.loads_today > 7, "the slot's page loads are kept too: {kept:?}");
    // A pass that sent nothing, for a mailbox with no backfill, writes no pace at all.
    let quiet = Pass::with_provider("slot-quiet-pace", "google", "active", rig::fwd("on", "ours"));
    assert_eq!(quiet.run().as_deref(), Some("mail: forwarding on, 0 recovered"));
    assert_eq!(pace(&quiet.d), None);
}

#[test]
fn a_slot_pass_keeps_each_gmail_message_in_mail_json_before_its_end_even_with_no_pace_yet() {
    // Review round 2 of e7ae803, #4 (a): a slot pass's note after each send only merged into a pace the day already
    // kept, so on a day with none its messages reached mail.json only at the pass's end, and an app killed mid-pass
    // lost up to 30. The final look comes after the send and before the end: the message is already kept.
    let g = Pass::with_provider("slot-messages-each-send", "google", "active", rig::fwd("on", "ours"));
    g.mail(&[("Notice", rig::NOW - 3600)]);
    g.host.st.lock().unwrap().probe_dir = Some(g.d.clone());
    assert_eq!(g.run().as_deref(), Some("mail: forwarding on, 1 recovered"));
    let probed = g.host.st.lock().unwrap().probed.clone();
    assert_eq!(probed.len(), 1, "the one look, at the pass's end");
    // No file yet is nothing kept.
    let st: MailState = if probed[0].is_empty() { MailState::default() } else { serde_json::from_str(&probed[0]).expect("mail.json at the look") };
    assert_eq!(st.backfill_pace.get(rig::ID).map(|p| p.messages_today), Some(1), "{}", probed[0]);
}

#[test]
fn a_slot_whose_only_missing_row_never_attaches_stalls_two_days_and_leaves_it_on_the_third() {
    // Review of c8806b3: the lone row that stalls a backfill stalls every slot too. The slots count its days in
    // mail.json as the backfill does (its received minute only), and from the third the row is left.
    let g = Pass::with_provider("slot-lone-stall", "google", "active", rig::fwd("on", "ours"));
    let at = rig::NOW - 3_600 - rig::NOW.rem_euclid(60);
    g.mail(&[("Stuck", at)]);
    g.host.st.lock().unwrap().no_chip = Some("Stuck".into());
    for d in 0..2 {
        g.clk.0.store(rig::NOW + d * rig::DAY, std::sync::atomic::Ordering::SeqCst);
        let line = g.run().expect("a line");
        assert!(line.ends_with("0 recovered, stopped: the attachments would not attach"), "day {d}: {line}");
        // The slot's own record since the review of 5f71623..6a8e738, #2 (`lone_stall` is the backfill's).
        let r = MailState::load(&g.d).unwrap().recover_stall[rig::ID];
        assert_eq!((r.at, r.days, r.last_day), (at, d as u32 + 1, (rig::NOW + d * rig::DAY).div_euclid(rig::DAY)), "day {d}");
    }
    g.clk.0.store(rig::NOW + 2 * rig::DAY, std::sync::atomic::Ordering::SeqCst);
    assert_eq!(g.run().as_deref(), Some("mail: forwarding on, 0 recovered"));
    assert_eq!(g.host.calls("gmail/forward.js").len(), 3, "tried once a day");
    // Review round 2 of e7ae803, #1: the row left at once is kept as left (it was the record, kept until the row was
    // sent or held, which the next stuck row could replace), and the record is free for the next one.
    let st = MailState::load(&g.d).unwrap();
    assert_eq!(st.lone_left.get(rig::ID).cloned(), Some(vec![at]), "{st:?}");
    assert!(!st.recover_stall.contains_key(rig::ID) && !st.lone_stall.contains_key(rig::ID), "{st:?}");
    // It stays left on the days after.
    g.clk.0.store(rig::NOW + 3 * rig::DAY, std::sync::atomic::Ordering::SeqCst);
    assert_eq!(g.run().as_deref(), Some("mail: forwarding on, 0 recovered"));
    assert_eq!(MailState::load(&g.d).unwrap().lone_left.get(rig::ID).cloned(), Some(vec![at]));
}

/// Runs `g`'s slot `d` days after NOW: its line.
fn stuck_slot(g: &Pass, d: i64) -> String {
    g.clk.0.store(rig::NOW + d * rig::DAY, std::sync::atomic::Ordering::SeqCst);
    g.run().expect("a line")
}

#[test]
fn a_lone_row_sent_on_its_third_day_is_not_kept_as_left() {
    // Review of 5f71623..6a8e738, #3: the record moved to `lone_left` on its third day whether or not the row was
    // sent or held that day, so a row that attached at last was kept as left: a recover-only mailbox grew a kept
    // minute for each, and any other single row in that minute that failed once was left at once, never retried.
    let g = Pass::with_provider("lone-left-sent-third-day", "google", "active", rig::fwd("on", "ours"));
    let at = rig::NOW - 3_600 - rig::NOW.rem_euclid(60);
    g.mail(&[("Stuck", at)]);
    g.host.st.lock().unwrap().no_chip = Some("Stuck".into());
    for d in 0..2 {
        assert!(stuck_slot(&g, d).ends_with("stopped: the attachments would not attach"), "day {d}");
    }
    g.host.st.lock().unwrap().no_chip = None;
    assert_eq!(stuck_slot(&g, 2), "mail: forwarding on, 1 recovered");
    let st = MailState::load(&g.d).unwrap();
    assert!(st.lone_left.is_empty() && st.recover_stall.is_empty(), "{st:?}");
}

#[test]
fn a_row_kept_as_left_is_dropped_once_its_minute_is_sent() {
    // Review of 5f71623..6a8e738, #3: a row kept as left stayed kept until a backfill closed, which a recover-only
    // mailbox never has. Once a row in its minute is sent (or held), the minute is no longer kept.
    let g = Pass::with_provider("lone-left-dropped-when-sent", "google", "active", rig::fwd("on", "ours"));
    let at = rig::NOW - 3_600 - rig::NOW.rem_euclid(60);
    g.mail(&[("Stuck", at)]);
    g.host.st.lock().unwrap().no_chip = Some("Stuck".into());
    for d in 0..2 {
        assert!(stuck_slot(&g, d).ends_with("stopped: the attachments would not attach"), "day {d}");
    }
    assert_eq!(stuck_slot(&g, 2), "mail: forwarding on, 0 recovered");
    assert_eq!(MailState::load(&g.d).unwrap().lone_left.get(rig::ID).cloned(), Some(vec![at]));
    g.host.st.lock().unwrap().no_chip = None;
    // A later slot the same day (the row is still in the slot's window).
    assert_eq!(stuck_slot(&g, 2), "mail: forwarding on, 1 recovered");
    let st = MailState::load(&g.d).unwrap();
    assert!(st.lone_left.is_empty() && st.recover_stall.is_empty(), "{st:?}");
}

#[test]
fn a_row_left_at_once_beside_a_held_row_in_its_minute_stays_left() {
    // Review round 1 of e7ae803 ruled that the stall wins over a held row from the same burst; the same holds for a
    // row left at once: a held row in its minute does not undo it, or the stuck row would stall the slot again.
    let g = Pass::with_provider("lone-left-burst", "google", "active", rig::fwd("on", "ours"));
    let at = rig::NOW - 3_600 - rig::NOW.rem_euclid(60);
    let who = "ana.lopez@school.example.invalid";
    g.mail(&[("Stuck", at), ("Burst", at + 30)]);
    g.svc.st.lock().unwrap().held = vec![rig::held(who, "Burst", at + 30)];
    g.host.st.lock().unwrap().no_chip = Some("Stuck".into());
    for d in 0..2 {
        assert!(stuck_slot(&g, d).ends_with("stopped: the attachments would not attach"), "day {d}");
    }
    // The row stays in the slot's window through day 2; later slots that day leave it at once too.
    for d in [2, 2, 2] {
        assert_eq!(stuck_slot(&g, d), "mail: forwarding on, 0 recovered", "day {d}");
        assert_eq!(MailState::load(&g.d).unwrap().lone_left.get(rig::ID).cloned(), Some(vec![at]), "day {d}");
    }
}

#[test]
fn a_backfill_and_a_slot_stalled_on_different_lone_rows_each_count_their_days_and_both_rows_are_left_on_the_third() {
    // Review of 5f71623..6a8e738, #2: a backfill pass and a slot pass shared one lone-stall record. Row A, old, stalls
    // every backfill pass with nothing newer sent after it; row C, recent, stalls every slot. Each day the backfill set
    // the record to A at one day and the slot set it to C at one day, so neither reached three and both failed every
    // day for good. Each kind of pass keeps its own record: both count to three and are left.
    use knowlu::mail::backfill::{drive_backfill, Drive, Drove};
    use knowlu::mail::setup::Shared;
    use knowlu::mail_steps::{Provider, Routine};
    use std::sync::atomic::Ordering::SeqCst;
    let g = Pass::with_provider("lone-stall-backfill-and-slot", "google", "active", rig::fwd("on", "ours"));
    let minute = |t: i64| t - t.rem_euclid(60);
    let (a, c) = (minute(rig::NOW - 30 * rig::DAY), minute(rig::NOW - 3_600));
    g.mail(&[("Stuck A", a), ("Stuck C", c)]);
    g.host.st.lock().unwrap().no_chips = vec!["Stuck A".into(), "Stuck C".into()];
    let since: jiff::civil::Date = rig::iso(rig::NOW - 40 * rig::DAY)[..10].parse().unwrap();
    let backfill = |at: i64| {
        g.clk.0.store(at, SeqCst);
        let (client, shared) = (g.svc.client(), Shared::new());
        let d = Drive {
            mb: &g.host,
            client: &client,
            clock: &g.clk,
            provider: Provider::Gmail,
            address_id: rig::ID,
            knowlu_address: rig::KNOWLU,
            since,
            shared: &shared,
            on_closed: &|| {},
            data_dir: &g.d,
        };
        drive_backfill(&d, &mut Routine::new(Provider::Gmail, at.div_euclid(rig::DAY), 0))
    };
    let slot = |at: i64| {
        g.clk.0.store(at, SeqCst);
        g.run().expect("a line")
    };
    let stuck = "0 recovered, stopped: the attachments would not attach";
    for d in 0..2 {
        let day = rig::NOW + d * rig::DAY;
        assert_eq!(backfill(day), Drove::Failed("the attachments would not attach"), "day {d}");
        let line = slot(day + 3_600);
        assert!(line.ends_with(stuck), "day {d}: {line}");
        let st = MailState::load(&g.d).unwrap();
        assert_eq!(st.lone_stall.get(rig::ID).map(|r| (r.at, r.days)), Some((a, d as u32 + 1)), "day {d}: {st:?}");
        assert_eq!(st.recover_stall.get(rig::ID).map(|r| (r.at, r.days)), Some((c, d as u32 + 1)), "day {d}: {st:?}");
    }
    // Day 3: the backfill leaves A at once and then stalls on C, which the slot leaves at once. Both are kept as left.
    let day = rig::NOW + 2 * rig::DAY;
    assert_eq!(backfill(day), Drove::Failed("the attachments would not attach"));
    assert_eq!(slot(day + 3_600), "mail: forwarding on, 0 recovered");
    assert_eq!(MailState::load(&g.d).unwrap().lone_left.get(rig::ID).cloned(), Some(vec![a, c]));
    // Day 4: the backfill leaves both and is done.
    assert_eq!(backfill(day + rig::DAY), Drove::Done);
    let tries = g.host.calls("gmail/forward.js");
    let tried = |s: &str| tries.iter().filter(|t| t["rows"].as_array().is_some_and(|r| r.len() == 1 && r[0]["subject"] == s)).count();
    assert_eq!((tried("Stuck A"), tried("Stuck C")), (4, 7), "alone, A once a backfill pass and C once a pass of each kind");
}

#[test]
fn a_slot_pass_on_a_day_whose_pages_are_spent_says_page_limit_reached_and_opens_nothing() {
    let g = Pass::with_provider("slot-page-limit", "google", "active", rig::fwd("on", "ours"));
    g.mail(&[("Notice", rig::NOW - 3600)]);
    MailState::update(&g.d, |s| {
        let day = rig::NOW.div_euclid(rig::DAY);
        s.backfill_pace.insert(rig::ID.to_string(), knowlu::mail_session::BackfillPace { day, sent_today: 3, paused_until: None, loads_today: 600, messages_today: 0 });
        true
    })
    .unwrap();
    assert_eq!(g.run().as_deref(), Some("mail: forwarding on, 0 recovered, page limit reached"));
    assert!(g.host.calls("gmail/search.js").is_empty(), "no search was made");
}

#[test]
fn a_report_seen_once_does_not_pause_the_next_days_first_slot_again() {
    let g = Pass::with_provider("last-look-next-day", "google", "quiet", rig::fwd("on", "ours"));
    g.mail(&[("Notice", rig::NOW - 3600)]);
    g.host.st.lock().unwrap().report_at = Some(rig::NOW - 60);
    let line = g.run().expect("a line");
    assert!(line.contains("paused: a send was refused"), "{line}");
    // The next slot is on the next UTC day, 13 hours later; the same report is still in the Inbox, and a day's
    // reach-back would find it again: it was seen at the look before, so it is not counted twice.
    g.clk.0.store(rig::NOW + 13 * 3600, std::sync::atomic::Ordering::SeqCst);
    assert_ne!((rig::NOW + 13 * 3600).div_euclid(rig::DAY), rig::NOW.div_euclid(rig::DAY));
    assert_eq!(g.run().as_deref(), Some("mail: forwarding on, 1 recovered"));
    // The first slot's look ran once Gmail's Undo Send delay had passed after its forward.
    assert_eq!(g.host.calls("gmail/bounce.js")[1]["since"], json!(rig::iso(rig::NOW + 31)));
}

/// Every file under `dir` except the provider's own browser profiles, which are WebView2's and not Knowlu's.
fn knowlus_own_files(dir: &Path, out: &mut Vec<PathBuf>) {
    for e in std::fs::read_dir(dir).into_iter().flatten().flatten() {
        let (p, name) = (e.path(), e.file_name().to_string_lossy().to_string());
        if p.is_dir() {
            if !(name.starts_with("mail-session-") || name == "mail-setup") {
                knowlus_own_files(&p, out);
            }
        } else {
            out.push(p);
        }
    }
}

/// Spec 5.4 and the plan's R17: a setup and a slot over a service whose address and bearer are unique
/// markers. A byte search of every file Knowlu writes (the data folder and the vault), the slot's summary and
/// its runner log then finds neither. The progress view may show the address, from the in-memory answer.
#[test]
fn no_file_or_log_of_knowlus_own_holds_the_address_or_token() {
    use knowlu::mail::setup::{Env, Kind, Plan, Setups};
    let (v, cs) = app_slot_vault("mail-markers");
    let svc = rig::Svc::start(vec![rig::row("active", "m365")]);
    let host = rig::FakeHost::new(rig::fwd("on", "ours"));
    let curated = knowlu::scaffold::CAMPUSES.iter().find(|c| c.mail_provider == "m365").expect("a curated m365 row");
    let today = jiff::Timestamp::from_second(rig::NOW).unwrap().to_zoned(jiff::tz::TimeZone::UTC).date();
    let plan = Plan { kind: Kind::School { unitid: Some(curated.unitid.to_string()), address: None }, data_dir: cs.data_dir.clone(), keep_session: true, setup_day: today };
    let env = Env {
        host: Box::new(rig::FakeHost { st: host.st.clone() }),
        client: svc.client(),
        clock: Box::new(rig::Clk::at(rig::NOW)),
        on_closed: Box::new(|| {}),
        guard: None,
    };
    let setups = Setups::new();
    let run = setups.start(plan, env).expect("a setup started");
    // D13: a Microsoft setup ends at the guided backfill step, which the student closes with "I've done this".
    let until = std::time::Instant::now() + Duration::from_secs(10);
    while !run.progress().rows.iter().any(|r| r.step == "backfill" && r.state == "needs_you") {
        assert!(std::time::Instant::now() < until, "the guided backfill step was shown");
        std::thread::sleep(Duration::from_millis(5));
    }
    assert!(run.answer(knowlu::mail::setup::Answer::Done));
    assert!(run.wait_done(Duration::from_secs(30)), "the setup ended");
    assert!(host.scripts().contains(&"m365/forwarding_set.js".to_string()), "it really ran: {:?}", host.scripts());

    let client = svc.client();
    let clk = rig::Clk::at(rig::NOW);
    let rows = || client.list_addresses().map_err(|e| e.name());
    let pass = |due: &[Value]| Ok(run_step(&SlotEnv { host: &host, client: &client, clock: &clk, data_dir: &cs.data_dir }, due).unwrap_or_default());
    let sch = Scheduler::default();
    let s = run_mail_slot(&cs, &sch, &MailSeam { mailboxes: &rows, pass: Some(&pass), window_open: false });
    assert!(s.steps.iter().any(|(n, _)| n.starts_with("mail: forwarding on")), "the slot's pass ran: {:?}", s.steps);

    let mut files = Vec::new();
    knowlus_own_files(&cs.data_dir, &mut files);
    knowlus_own_files(&v, &mut files);
    assert!(files.iter().any(|f| f.file_name().is_some_and(|n| n == "mail.json")), "mail.json is among them: {files:?}");
    let mut texts: Vec<(String, Vec<u8>)> = files.iter().map(|f| (f.display().to_string(), std::fs::read(f).unwrap_or_default())).collect();
    texts.push(("the RunSummary".to_string(), format!("{s:?}{}", serde_json::to_string(&s).unwrap()).into_bytes()));
    for (what, bytes) in &texts {
        for marker in [rig::KNOWLU, rig::TOKEN, "marker-address-qzx7731", "marker-bearer-qzx7731"] {
            assert!(!bytes.windows(marker.len()).any(|w| w == marker.as_bytes()), "{what} holds {marker}");
        }
    }
    let _ = std::fs::remove_dir_all(&cs.data_dir);
    let _ = std::fs::remove_dir_all(&v);
}

// ---- L4 T6: until the privacy update is accepted, the slot's cloud steps are named skips ---------------------
//
// The gate is driven by writing `privacy.json`. Each rig vault names a loopback `api_base` that a test-owned
// listener serves and counts (every request, whatever its method or path), and a generated
// `session_credential_target` that nothing ever writes. So the slot only READS Credential Manager at that target
// (the session step, `refresh_privacy`, `refresh_entitlement`, telemetry), which is the never-written-target
// exemption `m4_quit_flush_reports_a_skip_as_a_skip_never_synced` records: no `CREDMAN_LOCK`, no write, no delete.
// Each of those reads fails before a request, so the listener must count none.

use knowlu::account::{privacy_cache_path, save_cache, save_privacy_cache, EntitlementCache, PrivacyCache, PRIVACY_VERSION};
use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};
use std::sync::Arc;

const HELD: &str = "(skipped: privacy update not accepted)";

struct CountingListener {
    port: u16,
    hits: Arc<AtomicUsize>,
    stop: Arc<AtomicBool>,
    join: Option<std::thread::JoinHandle<()>>,
}

impl CountingListener {
    fn start() -> Self {
        let listener = std::net::TcpListener::bind("127.0.0.1:0").expect("bind loopback");
        listener.set_nonblocking(true).unwrap();
        let port = listener.local_addr().unwrap().port();
        let (hits, stop) = (Arc::new(AtomicUsize::new(0)), Arc::new(AtomicBool::new(false)));
        let (h, s) = (hits.clone(), stop.clone());
        let join = std::thread::spawn(move || {
            while !s.load(Ordering::SeqCst) {
                match listener.accept() {
                    Ok(_) => {
                        h.fetch_add(1, Ordering::SeqCst);
                    }
                    Err(_) => std::thread::sleep(Duration::from_millis(5)),
                }
            }
        });
        CountingListener { port, hits, stop, join: Some(join) }
    }
    fn hits(&self) -> usize { self.hits.load(Ordering::SeqCst) }
}

impl Drop for CountingListener {
    fn drop(&mut self) {
        self.stop.store(true, Ordering::SeqCst);
        if let Some(j) = self.join.take() {
            let _ = j.join();
        }
    }
}

/// One scratch app-scheduled vault with an account, a never-written session target, an entitlement cache of
/// `status` and the env seam (`KNOWLU_ENGINE_EXE=cmd`, a fresh `LOCALAPPDATA`, `KNOWLU_API_BASE` = the listener).
struct PrivacyRig {
    v: PathBuf,
    cs: ConsoleState,
    fake: PathBuf,
    listener: CountingListener,
    _env: EnvSeam,
    _guard: std::sync::MutexGuard<'static, ()>,
}

fn privacy_rig(tag: &str, status: &str, grades_connected: bool) -> PrivacyRig {
    let guard = ENGINE_ENV_LOCK.lock().unwrap_or_else(|e| e.into_inner());
    let listener = CountingListener::start();
    let (v, cs) = app_slot_vault(&format!("privacy-{tag}"));
    let base = format!("http://127.0.0.1:{}/functions/v1", listener.port);
    let target = format!("knowlu/test-l4-gate-{}-{tag}/session", std::process::id());
    std::fs::write(
        v.join("config").join("cloud.yaml"),
        format!("api_base: '{base}'\nanon_key: 'anon'\nsession_credential_target: '{target}'\naccount_id: 'acc-1'\n"),
    )
    .unwrap();
    std::fs::write(v.join("config").join("campus.yaml"), "unitid: '999999'\r\nlms: 'blackboard'\r\n").unwrap();
    if grades_connected {
        std::fs::create_dir_all(grades_app::session_dir(&cs.data_dir)).unwrap();
    }
    save_cache(&cs.data_dir, &EntitlementCache {
        status: status.into(),
        current_period_end: None,
        plan: Some("monthly".into()),
        checked_at: knowlu_engine::journal::now_ts(None),
    })
    .unwrap();
    let fake = std::env::temp_dir().join(format!("qo-sched-privacy-localappdata-{tag}-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&fake);
    std::fs::create_dir_all(&fake).unwrap();
    let env = EnvSeam::set(&[
        ("KNOWLU_ENGINE_EXE", std::ffi::OsStr::new("cmd")),
        ("LOCALAPPDATA", fake.as_os_str()),
        ("KNOWLU_API_BASE", std::ffi::OsStr::new(&base)),
    ]);
    PrivacyRig { v, cs, fake, listener, _env: env, _guard: guard }
}

impl PrivacyRig {
    fn set_privacy(&self, recorded: Option<&str>) {
        save_privacy_cache(&self.cs.data_dir, &PrivacyCache {
            account_id: "acc-1".into(),
            recorded: recorded.map(String::from),
            checked_at: "2026-10-20T00:00:00Z".into(),
        })
        .unwrap();
    }
    fn cache_bytes(&self) -> Option<Vec<u8>> { std::fs::read(privacy_cache_path(&self.cs.data_dir)).ok() }
    fn slot(&self, grades: &GradesSeam, mail: &MailSeam) -> knowlu::scheduler::RunSummary {
        run_slot_with(&self.cs, &Scheduler::default(), None, false, grades, mail)
    }
    fn runner_log(&self) -> String { knowlu_engine::pystr::read_text(&self.v.join("state").join("runner-log.md")).unwrap_or_default() }
    /// Slot logs the child loop wrote: `slot-<started>-<i>-<step>.txt`, one per child it started.
    fn children_started(&self) -> Vec<String> {
        let mut names: Vec<String> = std::fs::read_dir(self.cs.data_dir.join("logs"))
            .map(|d| d.flatten().map(|e| e.file_name().to_string_lossy().to_string()).collect())
            .unwrap_or_default();
        names.sort();
        names.iter().map(|n| n.trim_end_matches(".txt").rsplit('-').next().unwrap_or("").to_string()).collect()
    }
}

impl Drop for PrivacyRig {
    fn drop(&mut self) {
        let _ = &self.listener;
        let _ = std::fs::remove_dir_all(&self.fake);
        let _ = std::fs::remove_dir_all(&self.cs.data_dir);
        let _ = std::fs::remove_dir_all(&self.v);
    }
}

fn names_of(s: &knowlu::scheduler::RunSummary) -> Vec<String> { s.steps.iter().map(|(n, _)| n.clone()).collect() }

fn panic_capture(_h: &'static str) -> Result<Value, CaptureError> { panic!("the grades capture seam was called") }
fn panic_pass(_due: &[Value]) -> Result<String, &'static str> { panic!("the mail pass seam was called") }
fn one_due_mailbox() -> Result<Vec<Value>, &'static str> { Ok(vec![rig::row("active", "m365")]) }
fn no_mailbox() -> Result<Vec<Value>, &'static str> { Ok(Vec::new()) }

#[test]
fn an_older_privacy_version_skips_every_cloud_step_by_name_and_still_ranks() {
    let r = privacy_rig("older", "active", true);
    r.set_privacy(Some("2026-01-01"));
    let before = r.cache_bytes();
    let dated = test_row("blackboard", Some("2026-10-01"));
    let grades = GradesSeam { row: Some(&dated), capture: Some(&panic_capture), window_open: false };
    let mail = MailSeam { mailboxes: &one_due_mailbox, pass: Some(&panic_pass), window_open: false };
    let s = r.slot(&grades, &mail);
    let named = names_of(&s);
    // The session step's row comes first (its reason is the Credential Manager read's), then exactly one
    // privacy row, then the skips in the slot's order.
    assert!(named[0].starts_with("session (refresh failed: "), "{named:?}");
    assert_eq!(named[1], "privacy (check failed: no session)", "{named:?}");
    let held: Vec<String> = ["grades", "mail", "judge", "sync", "coursework", "ingest"].iter().map(|n| format!("{n} {HELD}")).collect();
    assert_eq!(named[2..8], held[..], "{named:?}");
    assert_eq!(named.iter().filter(|n| n.starts_with("privacy")).count(), 1, "exactly one privacy row: {named:?}");
    assert!(named.iter().any(|n| n == "rank"), "rank still runs: {named:?}");
    assert!(s.steps.iter().all(|(_, c)| *c == 0) && s.engine_ok && s.ok, "every code is 0: {:?}", s.steps);
    // None of the held steps ran: no bare row, and the child loop started nothing but `rank`.
    for step in ["sync", "coursework", "ingest", "grades", "judge"] {
        assert!(!named.iter().any(|n| n == step), "{step} ran: {named:?}");
    }
    assert_eq!(r.children_started(), ["rank"]);
    // The stand-in capture marks nothing: not a "capture running" row, and no signed-out mark.
    assert!(!named.iter().any(|n| n.contains("another grades capture is running")), "{named:?}");
    assert!(!r.cs.data_dir.join("grades-signed-out").exists(), "no signed-out mark");
    assert!(no_bundles_left(&r.cs.data_dir));
    assert_eq!(r.cache_bytes(), before, "a failed check leaves the cache as it was");
    assert_eq!(r.listener.hits(), 0, "no request of any kind");
}

#[test]
fn an_older_version_names_only_steps_that_would_have_run() {
    let r = privacy_rig("older-quiet", "active", false);
    r.set_privacy(None);
    let dated = test_row("blackboard", Some("2026-10-01"));
    let grades = GradesSeam { row: Some(&dated), capture: Some(&panic_capture), window_open: false };
    let mail = MailSeam { mailboxes: &no_mailbox, pass: Some(&panic_pass), window_open: false };
    let s = r.slot(&grades, &mail);
    let named = names_of(&s);
    assert!(named.iter().any(|n| n == "grades (skipped: not connected)"), "today's name, not privacy's: {named:?}");
    assert!(!named.iter().any(|n| n.starts_with("mail")), "no mailbox, no mail row: {named:?}");
    assert!(named.iter().any(|n| n == &format!("judge {HELD}")), "{named:?}");
    assert!(!named.iter().any(|n| n == &format!("grades {HELD}")), "{named:?}");
    assert_eq!(r.listener.hits(), 0);
}

#[test]
fn an_accepted_privacy_version_leaves_the_slot_as_it_was() {
    let r = privacy_rig("accepted", "active", true);
    r.set_privacy(Some(PRIVACY_VERSION));
    let before = r.cache_bytes();
    let dated = test_row("blackboard", Some("2026-10-01"));
    let (captures, passes) = (Cell::new(0), Cell::new(0));
    let capture = |_h: &'static str| { captures.set(captures.get() + 1); Ok(bundle()) };
    let pass = |_: &[Value]| { passes.set(passes.get() + 1); Ok("mail: forwarding on, 0 recovered".to_string()) };
    let grades = GradesSeam { row: Some(&dated), capture: Some(&capture), window_open: false };
    let mail = MailSeam { mailboxes: &one_due_mailbox, pass: Some(&pass), window_open: false };
    let s = r.slot(&grades, &mail);
    let named = names_of(&s);
    assert!(named[0].starts_with("session (refresh failed: "), "{named:?}");
    assert!(!named.iter().any(|n| n.contains("privacy")), "no privacy row of any kind: {named:?}");
    assert_eq!((captures.get(), passes.get()), (1, 1), "both real seams ran");
    // The child loop's argv is slot_argv's unfiltered output.
    let bundle_path = PathBuf::from("bundle.json");
    let plan = JudgePlan::Cloud { log_dir: PathBuf::from("logs") };
    let want: Vec<String> = slot_argv(&r.v, Path::new("cmd"), &plan, Some(&bundle_path)).into_iter().map(|(_, a)| a[0].clone()).collect();
    assert_eq!(want, ["sync", "coursework", "grades", "ingest", "judge", "rank"]);
    assert_eq!(r.children_started(), want);
    assert_eq!(r.cache_bytes(), before, "an accepted cache makes no call");
    assert_eq!(r.listener.hits(), 0);
}

#[test]
fn an_unknown_privacy_version_names_only_the_failed_check() {
    let r = privacy_rig("unknown", "active", true);
    assert!(r.cache_bytes().is_none(), "no cache to begin with");
    let dated = test_row("blackboard", Some("2026-10-01"));
    let (captures, passes) = (Cell::new(0), Cell::new(0));
    let capture = |_h: &'static str| { captures.set(captures.get() + 1); Ok(bundle()) };
    let pass = |_: &[Value]| { passes.set(passes.get() + 1); Ok("mail: forwarding on, 0 recovered".to_string()) };
    let grades = GradesSeam { row: Some(&dated), capture: Some(&capture), window_open: false };
    let mail = MailSeam { mailboxes: &one_due_mailbox, pass: Some(&pass), window_open: false };
    let s = r.slot(&grades, &mail);
    let named = names_of(&s);
    assert!(named[0].starts_with("session (refresh failed: "), "{named:?}");
    assert_eq!(named[1], "privacy (check failed: no session)", "{named:?}");
    assert_eq!(named.iter().filter(|n| n.starts_with("privacy")).count(), 1, "exactly one privacy row: {named:?}");
    assert!(!named.iter().any(|n| n.contains("(skipped: privacy")), "{named:?}");
    assert_eq!((captures.get(), passes.get()), (1, 1), "an unknown gate holds nothing back");
    assert_eq!(r.children_started(), ["sync", "coursework", "grades", "ingest", "judge", "rank"]);
    assert!(r.cache_bytes().is_none(), "a failed check creates no cache file");
    assert!(s.engine_ok && s.ok, "{:?}", s.steps);
    assert_eq!(r.listener.hits(), 0);
}

/// M3: a step the entitlement stops keeps the entitlement's name, whatever the privacy cache says.
#[test]
fn an_unentitled_account_keeps_the_entitlements_names() {
    let dated = test_row("blackboard", Some("2026-10-01"));
    let seams = || (GradesSeam { row: Some(&dated), capture: Some(&panic_capture), window_open: false }, MailSeam { mailboxes: &one_due_mailbox, pass: Some(&panic_pass), window_open: false });
    // Today's rows: the same vault and slot with an accepted privacy cache (which adds nothing).
    let accepted = {
        let r = privacy_rig("unentitled", "canceled", true);
        r.set_privacy(Some(PRIVACY_VERSION));
        let (g, m) = seams();
        let s = r.slot(&g, &m);
        assert_eq!(r.listener.hits(), 0);
        names_of(&s)
    };
    let r = privacy_rig("unentitled", "canceled", true);
    r.set_privacy(Some("2026-01-01"));
    let before = r.cache_bytes();
    let (g, m) = seams();
    let s = r.slot(&g, &m);
    let named = names_of(&s);
    assert_eq!(named, accepted, "an Older cache changes nothing for an unentitled account");
    for want in ["grades (skipped: no entitlement)", "mail (skipped: no entitlement)", "judge (skipped: no entitlement)", "sync", "coursework", "ingest", "rank"] {
        assert!(named.iter().any(|n| n == want), "{want} in {named:?}");
    }
    assert!(!named.iter().any(|n| n.contains("privacy")), "{named:?}");
    assert_eq!(r.children_started(), ["sync", "coursework", "ingest", "rank"], "the argv is unfiltered");
    assert_eq!(r.cache_bytes(), before);
    assert_eq!(r.listener.hits(), 0);
}

#[test]
fn the_privacy_skips_reach_the_runner_log_except_mails() {
    let r = privacy_rig("log", "active", true);
    r.set_privacy(Some("2026-01-01"));
    let dated = test_row("blackboard", Some("2026-10-01"));
    let grades = GradesSeam { row: Some(&dated), capture: Some(&panic_capture), window_open: false };
    let mail = MailSeam { mailboxes: &one_due_mailbox, pass: Some(&panic_pass), window_open: false };
    let s = r.slot(&grades, &mail);
    assert!(names_of(&s).iter().any(|n| n == &format!("mail {HELD}")), "mail is in the summary: {:?}", s.steps);
    let log = r.runner_log();
    for line in ["sync", "coursework", "ingest", "grades", "judge"] {
        assert!(log.contains(&format!("local ok {line} {HELD}")), "{line} in the runner log:\n{log}");
    }
    // Beside today's bare session line, the bare privacy line; no reason text from either.
    assert!(log.contains("local ok session (refresh failed)"), "{log}");
    assert!(log.contains("local ok privacy (check failed)"), "{log}");
    assert!(!log.contains("check failed:") && !log.contains("refresh failed:") && !log.contains("no session"), "no reason text:\n{log}");
    assert!(!log.contains("mail"), "a mail step never reaches the runner log:\n{log}");
    assert_eq!(r.listener.hits(), 0);
}

// ---- L4 T8-R3rust: the two sync paths outside the slot honour the same gate --------------------------------------
//
// *Sync now* (`state::run_sync`, through `commands::sync_inner`) and the quit push (`state::quit_flush`) do not run
// the slot, so T6's gate did not reach them. The rig is the one above: a never-written session target, so a sync
// that does run answers its own `no session` skip and writes the engine's status file; one that is held does
// neither. The panel says an unaccepted update "does not sync, import or read anything else new", so both are held.

const HELD_SYNC: &str = "sync (skipped: privacy update not accepted)";

#[test]
fn sync_now_while_the_privacy_update_is_owed_is_a_named_skip_never_an_error() {
    let r = privacy_rig("syncnow-older", "active", false);
    r.set_privacy(Some("2026-01-01"));
    let before = knowlu_engine::sync::load_status(&r.v);
    let status = knowlu::state::run_sync(&r.cs);
    assert_eq!(status.skipped.as_deref(), Some("privacy update not accepted"), "{status:?}");
    assert_eq!(status.lines, vec![HELD_SYNC.to_string()], "{status:?}");
    assert!(status.ok && status.last_error.is_none(), "a hold is not an error: {status:?}");
    assert_eq!(*r.cs.sync.lock().unwrap(), status, "the page reads the same status");
    assert_eq!(knowlu_engine::sync::load_status(&r.v), before, "the engine's sync never ran, so its status file is as it was");
    // The command's envelope: ok, no error, and the page's topline carries the skip.
    let env = knowlu::commands::sync_inner(&r.cs, "today").unwrap();
    assert_eq!(env["ok"], true, "{env}");
    assert!(env["error"].is_null(), "{env}");
    assert_eq!(env["state"]["topline"]["sync"]["skipped"], "privacy update not accepted", "{env}");
    assert_eq!(r.listener.hits(), 0);
}

#[test]
fn sync_now_runs_as_it_did_when_the_privacy_version_is_accepted_or_unknown() {
    for (tag, recorded) in [("syncnow-accepted", Some(PRIVACY_VERSION)), ("syncnow-unknown", None)] {
        let r = privacy_rig(tag, "active", false);
        if tag.ends_with("accepted") { r.set_privacy(recorded); }
        let status = knowlu::state::run_sync(&r.cs);
        assert_eq!(status.skipped.as_deref(), Some("no session"), "the engine's own answer, {tag}: {status:?}");
        assert!(!status.lines.iter().any(|l| l == HELD_SYNC), "{tag}: {status:?}");
        assert!(knowlu_engine::sync::load_status(&r.v).at.is_some(), "{tag}: the engine's sync ran and saved its status");
    }
}

#[test]
fn the_quit_push_is_held_while_the_privacy_update_is_owed() {
    let r = privacy_rig("quit-older", "active", false);
    r.set_privacy(Some("2026-01-01"));
    let before = knowlu_engine::sync::load_status(&r.v);
    let q = quit_flush(&r.cs, Duration::from_secs(10), |_| {});
    assert!(!q.synced && !q.timed_out, "{q:?}");
    assert_eq!(knowlu_engine::sync::load_status(&r.v), before, "no push was attempted, so the engine's status file is as it was");
    assert_eq!(r.listener.hits(), 0);
}

#[test]
fn the_quit_push_runs_as_it_did_when_the_privacy_version_is_accepted_or_unknown() {
    for (tag, accepted) in [("quit-accepted", true), ("quit-unknown", false)] {
        let r = privacy_rig(tag, "active", false);
        if accepted { r.set_privacy(Some(PRIVACY_VERSION)); }
        let q = quit_flush(&r.cs, Duration::from_secs(10), |_| {});
        assert!(!q.synced && !q.timed_out, "{tag}: signed out, so nothing pushed: {q:?}");
        assert!(knowlu_engine::sync::load_status(&r.v).at.is_some(), "{tag}: the engine's push ran and saved its status");
    }
}

// ======================================================================================================
// O3 F1 (plan tests 15, 18 and 27): the scheduler's half of onboarding. `run_slot_scoped` takes the same
// seams `run_slot_with` takes; the rig is the L4 `privacy_rig` above (a cloud vault, a loopback listener
// nothing is sent to, a never-written session target). Invented data only.
// ======================================================================================================

use knowlu::scheduler::{prime_mode_device, run_slot_scoped, SlotScope};

fn accepted_rig(tag: &str, grades_connected: bool) -> PrivacyRig {
    let r = privacy_rig(tag, "active", grades_connected);
    r.set_privacy(Some(PRIVACY_VERSION));
    r
}

fn rows_starting<'a>(names: &'a [String], stage: &str) -> Vec<&'a String> {
    names.iter().filter(|n| n.as_str() == stage || n.starts_with(&format!("{stage} ("))).collect()
}

/// Every stage a stopped slot did not start is named, and none ran.
fn assert_nothing_ran(r: &PrivacyRig, s: &knowlu::scheduler::RunSummary, runner_log_before: &str) {
    let named = names_of(s);
    for stage in ["sync", "coursework", "ingest", "judge", "rank"] {
        let rows = rows_starting(&named, stage);
        assert!(!rows.is_empty(), "the summary names {stage} as not run: {named:?}");
        assert!(rows.iter().all(|n| n.contains("not run")), "{stage} did not run: {named:?}");
    }
    for stage in ["session", "grades", "mail", "backup", "telemetry"] {
        assert!(rows_starting(&named, stage).iter().all(|n| n.contains("not run")), "{stage} started: {named:?}");
    }
    assert!(s.engine_ok && s.reason.is_none(), "a stop is not a failure: {s:?}");
    assert!(s.steps.iter().all(|(_, c)| *c == 0), "{:?}", s.steps);
    assert!(r.children_started().is_empty(), "no engine child started: {:?}", r.children_started());
    assert_eq!(r.runner_log(), runner_log_before, "no run-log line is added");
    assert!(no_bundles_left(&r.cs.data_dir));
    assert_eq!(r.listener.hits(), 0);
}

// ---- test 15: the scheduler's stop -------------------------------------------------------------------------

#[test]
fn test_15_a_stop_set_before_the_slot_starts_no_stage_and_survives_the_slot() {
    // A session sits at the vault's generated target, long expired, so a slot that did start its
    // session stage would refresh it against the counting listener: the "no refresh hit" below proves
    // the stage was skipped, not just that there was nothing to refresh. Lock first, so the credential
    // is cleaned up before the lock is released.
    let _credman_guard = CREDMAN_LOCK.lock().unwrap_or_else(|e| e.into_inner());
    let r = accepted_rig("o3-15-before", true);
    let target = format!("knowlu/test-l4-gate-{}-o3-15-before/session", std::process::id());
    let _cred_cleanup = CredCleanup(target.clone());
    knowlu::account::save_session(&target, "acc-1", &knowlu::account::Session {
        access_token: "stale-at".into(),
        refresh_token: "stale-rt".into(),
        expires_at: 0,
        email: "a@example.invalid".into(),
    }).unwrap();
    let dated = test_row("blackboard", Some("2026-10-01"));
    let grades = GradesSeam { row: Some(&dated), capture: Some(&panic_capture), window_open: false };
    let mail = MailSeam { mailboxes: &one_due_mailbox, pass: Some(&panic_pass), window_open: false };
    let sch = Scheduler::default();
    sch.stop.store(true, Ordering::SeqCst);
    let log_before = r.runner_log();
    let s = run_slot_scoped(&r.cs, &sch, None, false, &grades, &mail, SlotScope::Full);
    assert_nothing_ran(&r, &s, &log_before);
    // Nothing cleared the stop, and a second slot on the same Scheduler also runs no stage.
    assert!(sch.stop.load(Ordering::SeqCst), "nothing clears the stop");
    let s2 = run_slot_scoped(&r.cs, &sch, None, false, &grades, &mail, SlotScope::NoRank);
    assert_nothing_ran(&r, &s2, &log_before);
    assert!(sch.stop.load(Ordering::SeqCst));
    assert_eq!(r.listener.hits(), 0, "no session refresh request reached the service");
    assert_eq!(knowlu::account::load_session(&target).map(|(_, s)| s.access_token).ok().as_deref(), Some("stale-at"), "the session was not touched");
}

#[test]
fn test_15_a_stop_set_from_the_grades_capture_lets_it_finish_and_removes_its_bundle() {
    let r = accepted_rig("o3-15-mid", true);
    let dated = test_row("blackboard", Some("2026-10-01"));
    let sch = Scheduler::default();
    let captures = Cell::new(0);
    let capture = |_h: &'static str| {
        captures.set(captures.get() + 1);
        sch.stop.store(true, Ordering::SeqCst);
        Ok(bundle())
    };
    let grades = GradesSeam { row: Some(&dated), capture: Some(&capture), window_open: false };
    let mail = MailSeam { mailboxes: &one_due_mailbox, pass: Some(&panic_pass), window_open: false };
    let s = run_slot_scoped(&r.cs, &sch, None, false, &grades, &mail, SlotScope::Full);
    assert_eq!(captures.get(), 1, "the capture in flight finishes");
    assert!(r.children_started().is_empty(), "no engine child starts after the stop: {:?}", r.children_started());
    let named = names_of(&s);
    assert!(rows_starting(&named, "mail").iter().all(|n| n.contains("not run")), "the mail pass did not start (panic_pass would have fired): {named:?}");
    assert!(no_bundles_left(&r.cs.data_dir), "the bundle is gone when the slot returns, a stop included");
    assert!(s.engine_ok && s.reason.is_none(), "{s:?}");
    assert!(sch.stop.load(Ordering::SeqCst));
}

// ---- test 18: slot.partial through run_slot_scoped -------------------------------------------------------------

fn runner_log_of(r: &PrivacyRig) -> String { r.runner_log() }

#[test]
fn test_18_no_rank_names_the_grades_skip_like_a_full_slot_and_holds_rank_back_without_a_log_line() {
    let r = accepted_rig("o3-18-norank", true);
    let _ = std::fs::remove_file(r.v.join("state").join("today.md"));
    let dated = test_row("blackboard", Some("2026-10-01"));
    let capture = |_h: &'static str| Err(CaptureError::Unreachable);
    let grades = GradesSeam { row: Some(&dated), capture: Some(&capture), window_open: false };
    let s = run_slot_scoped(&r.cs, &Scheduler::default(), None, false, &grades, &MailSeam { mailboxes: &no_mailbox, pass: None, window_open: false }, SlotScope::NoRank);
    let named = names_of(&s);
    assert!(named.iter().any(|n| n == "grades (skipped: Blackboard unreachable)"), "{named:?}");
    assert!(runner_log_of(&r).contains("local ok grades (skipped: Blackboard unreachable)"), "the runner log gains its line as for a full slot");
    assert_eq!(r.children_started(), ["sync", "coursework", "ingest", "judge"], "rank is not in the argv");
    assert_eq!(named.iter().filter(|n| n.as_str() == "rank (not yet: the first full slot ranks)").count(), 1, "{named:?}");
    assert!(!runner_log_of(&r).contains("not yet"), "a scope's rows stay out of the runner log");
    assert!(!r.v.join("state").join("today.md").exists(), "no state/today.md was written");
}

#[test]
fn test_18_before_config_runs_sync_only_never_captures_and_names_every_other_step_once() {
    let r = accepted_rig("o3-18-before", true);
    let _ = std::fs::remove_file(r.v.join("config").join("ingest.yaml"));
    let dated = test_row("blackboard", Some("2026-10-01"));
    let grades = GradesSeam { row: Some(&dated), capture: Some(&panic_capture), window_open: false };
    let s = run_slot_scoped(&r.cs, &Scheduler::default(), None, false, &grades, &MailSeam { mailboxes: &no_mailbox, pass: None, window_open: false }, SlotScope::BeforeConfig);
    let named = names_of(&s);
    assert_eq!(r.children_started(), ["sync"], "the recorded argv is sync only");
    assert!(no_bundles_left(&r.cs.data_dir), "the capture seam was never called, so no bundle was written");
    for step in ["coursework", "grades", "ingest", "judge", "rank"] {
        let rows = rows_starting(&named, step);
        assert_eq!(rows.len(), 1, "{step} is one row (grades once: its capture and step together): {named:?}");
        assert!(rows[0].starts_with(&format!("{step} (not yet:")), "{}", rows[0]);
        if step == "grades" { assert_eq!(rows[0], "grades (not yet: your courses are not in yet)", "§4.3: one row stands for the capture and the step"); }
    }
    assert!(!runner_log_of(&r).contains("not yet"), "no runner-log line for a scope's rows");
}

#[test]
fn test_18_the_seams_belong_to_the_slot_so_no_rank_runs_them_as_a_full_slot_does() {
    let r = accepted_rig("o3-18-seams", true);
    let dated = test_row("blackboard", Some("2026-10-01"));
    let (captures, passes) = (Cell::new(0), Cell::new(0));
    let capture = |_h: &'static str| { captures.set(captures.get() + 1); Ok(bundle()) };
    let pass = |_: &[Value]| { passes.set(passes.get() + 1); Ok("mail: forwarding on, 0 recovered".to_string()) };
    let grades = GradesSeam { row: Some(&dated), capture: Some(&capture), window_open: false };
    let mail = MailSeam { mailboxes: &one_due_mailbox, pass: Some(&pass), window_open: false };
    let full = run_slot_scoped(&r.cs, &Scheduler::default(), None, false, &grades, &mail, SlotScope::Full);
    let full_children = r.children_started();
    assert_eq!((captures.get(), passes.get()), (1, 1), "{:?}", names_of(&full));
    let no_rank = run_slot_scoped(&r.cs, &Scheduler::default(), None, false, &grades, &mail, SlotScope::NoRank);
    assert_eq!((captures.get(), passes.get()), (2, 2), "both seams ran under NoRank too: {:?}", names_of(&no_rank));
    let all = r.children_started();
    let second: Vec<String> = all[full_children.len()..].to_vec();
    let want: Vec<String> = full_children.iter().filter(|c| c.as_str() != "rank").cloned().collect();
    assert_eq!(second, want, "the same steps as a full slot, without rank");
}

#[test]
fn test_18_a_pending_privacy_update_holds_the_same_steps_by_the_same_names_in_every_scope() {
    let r = accepted_rig("o3-18-privacy", true);
    r.set_privacy(Some("2026-01-01"));
    let dated = test_row("blackboard", Some("2026-10-01"));
    let grades = GradesSeam { row: Some(&dated), capture: Some(&panic_capture), window_open: false };
    let mail = MailSeam { mailboxes: &one_due_mailbox, pass: Some(&panic_pass), window_open: false };
    let held = |s: &knowlu::scheduler::RunSummary| -> Vec<String> { names_of(s).into_iter().filter(|n| n.contains(HELD)).collect() };
    let full = run_slot_scoped(&r.cs, &Scheduler::default(), None, false, &grades, &mail, SlotScope::Full);
    let no_rank = run_slot_scoped(&r.cs, &Scheduler::default(), None, false, &grades, &mail, SlotScope::NoRank);
    assert_eq!(held(&full).len(), 6, "grades, mail, judge, sync, coursework, ingest: {:?}", names_of(&full));
    assert_eq!(held(&no_rank), held(&full), "named exactly as in a full slot");
    // And the scope still applies on top of the hold: rank is not held by the privacy update, so a
    // NoRank slot names it as not yet, while a full slot runs it.
    assert!(names_of(&no_rank).iter().any(|n| n == "rank (not yet: the first full slot ranks)"), "{:?}", names_of(&no_rank));
    assert!(names_of(&full).iter().any(|n| n == "rank"), "{:?}", names_of(&full));
    let privacy_rows = |s: &knowlu::scheduler::RunSummary| names_of(s).iter().filter(|n| n.starts_with("privacy")).count();
    assert_eq!((privacy_rows(&full), privacy_rows(&no_rank)), (1, 1));
}

// ---- F8s review r1, minors 1 and 3 -------------------------------------------------------------------------

#[test]
fn test_15_a_stopped_slot_whose_judge_would_be_a_named_skip_adds_no_runner_log_line() {
    // "canceled": the judge plan is `Skip(no entitlement)`, which a running slot logs.
    let r = privacy_rig("o3-15-judge-skip", "canceled", true);
    let dated = test_row("blackboard", Some("2026-10-01"));
    let grades = GradesSeam { row: Some(&dated), capture: Some(&panic_capture), window_open: false };
    let mail = MailSeam { mailboxes: &one_due_mailbox, pass: Some(&panic_pass), window_open: false };
    let sch = Scheduler::default();
    sch.stop.store(true, Ordering::SeqCst);
    let log_before = r.runner_log();
    let s = run_slot_scoped(&r.cs, &sch, None, false, &grades, &mail, SlotScope::Full);
    assert_nothing_ran(&r, &s, &log_before);
    assert!(!names_of(&s).iter().any(|n| n.contains("no entitlement")), "the skip is not named: {:?}", names_of(&s));
}

#[test]
fn test_15_a_stop_set_after_the_privacy_hold_was_read_names_no_held_step_and_logs_none() {
    let r = accepted_rig("o3-15-late", false);
    r.set_privacy(Some("2026-01-01"));
    let sch = Scheduler::default();
    let late_stop = || { sch.stop.store(true, Ordering::SeqCst); Ok(Vec::new()) };
    let grades = GradesSeam { row: None, capture: None, window_open: false };
    let mail = MailSeam { mailboxes: &late_stop, pass: Some(&panic_pass), window_open: false };
    let s = run_slot_scoped(&r.cs, &sch, None, false, &grades, &mail, SlotScope::Full);
    let named = names_of(&s);
    assert!(sch.stop.load(Ordering::SeqCst), "the mail stage's mailbox read set the stop");
    assert!(r.children_started().is_empty(), "{:?}", r.children_started());
    for step in ["sync", "coursework", "ingest", "judge", "rank"] {
        assert!(rows_starting(&named, step).iter().all(|n| n.contains("not run")), "{step}: {named:?}");
    }
    let log = r.runner_log();
    for step in ["sync", "coursework", "ingest", "judge"] {
        assert!(!log.contains(&format!("{step} {HELD}")), "{step} has no held line in the runner log:\n{log}");
    }
    assert!(s.engine_ok && s.reason.is_none(), "{s:?}");
}

#[test]
fn test_18_before_config_keeps_the_judge_skip_its_entitlement_name_and_log_line() {
    let r = privacy_rig("o3-18-before-judge-skip", "canceled", false);
    let _ = std::fs::remove_file(r.v.join("config").join("ingest.yaml"));
    let grades = GradesSeam { row: None, capture: None, window_open: false };
    let mail = MailSeam { mailboxes: &no_mailbox, pass: None, window_open: false };
    let s = run_slot_scoped(&r.cs, &Scheduler::default(), None, false, &grades, &mail, SlotScope::BeforeConfig);
    let named = names_of(&s);
    assert_eq!(rows_starting(&named, "judge"), ["judge (skipped: no entitlement)"], "one row, its own name: {named:?}");
    assert!(runner_log_of(&r).contains("local ok judge (skipped: no entitlement)"), "and its log line");
    assert!(!runner_log_of(&r).contains("not yet"), "a scope's rows stay out of the log");
    assert_eq!(r.children_started(), ["sync"]);
}

// ---- test 27: onboarding mode primes the scheduler's cached mode ---------------------------------------------------

fn topline_scheduler(sch: &Scheduler) -> Value {
    let mut env = json!({ "state": { "topline": {} } });
    attach_scheduler(&mut env, sch).unwrap();
    env["state"]["topline"]["scheduler"].clone()
}

#[test]
fn test_27_a_default_scheduler_reports_the_script_placeholder_until_it_is_primed() {
    let t = topline_scheduler(&Scheduler::default());
    assert_eq!((t["mode"].as_str(), t["device_ok"].as_bool()), (Some("script"), Some(true)), "today's placeholder, pinned: {t}");
}

#[test]
fn test_27_priming_on_a_step_2_vault_reports_app_mode_on_this_device() {
    let (v, cs) = app_slot_vault("o3-27-prime");
    let sch = Scheduler::default();
    prime_mode_device(&cs, &sch);
    let t = topline_scheduler(&sch);
    assert_eq!((t["mode"].as_str(), t["device_ok"].as_bool()), (Some("app"), Some(true)), "{t}");
    let _ = std::fs::remove_dir_all(&cs.data_dir);
    let _ = std::fs::remove_dir_all(&v);
}

fn src_text(file: &str) -> String {
    let p = Path::new(env!("CARGO_MANIFEST_DIR")).join("src").join(file);
    let text = std::fs::read_to_string(&p).unwrap_or_else(|e| panic!("{}: {e}", p.display()));
    text.lines().filter(|l| !l.trim_start().starts_with("//")).collect::<Vec<_>>().join("\n")
}

#[test]
fn test_27_main_primes_the_mode_and_checks_for_an_update_once_and_never_installs() {
    let main = src_text("main.rs");
    assert!(main.contains("scheduler::prime_mode_device"), "the onboarding arm primes the cached mode before the window is built");
    // Before the window is built, inside `run_console`'s own body (a prime in `run_shell` would not count).
    let start = main.find("fn run_console").expect("main.rs defines run_console");
    let body = &main[start..];
    let (prime, build) = (body.find("scheduler::prime_mode_device"), body.find(".build(tauri::generate_context!())"));
    assert!(matches!((prime, build), (Some(p), Some(b)) if p < b), "run_console primes the mode ({prime:?}) before it builds the window ({build:?})");
    assert_eq!(main.matches("scheduler::spawn_update_check").count(), 1, "the onboarding arm calls spawn_update_check exactly once");
    for banned in ["install_staged", "hold_for_install"] {
        assert!(!main.contains(banned), "onboarding has no install path: main.rs names {banned}");
    }
}

#[test]
fn test_27_the_housekeeping_tick_calls_spawn_update_check_and_the_check_lives_only_inside_it() {
    let sched = src_text("scheduler.rs");
    let at = sched.find("pub fn spawn_update_check").expect("scheduler.rs defines spawn_update_check");
    let rest = &sched[at..];
    let body = &rest[..rest.find("\n}").map(|i| i + 2).unwrap_or(rest.len())];
    assert!(body.contains("check_and_stage_blocking"), "the check moved into spawn_update_check");
    assert_eq!(sched.matches("check_and_stage_blocking").count(), 1, "and appears nowhere else");
    assert!(sched.matches("spawn_update_check(").count() >= 2, "the tick calls it as well as defining it");
}
