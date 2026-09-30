//! Phase 2 of the commitment model (spec `docs/specs/2026-09-24-commitment-model-phase2-design.md`
//! §7): the four `week.rs` commands against a scratch copy of `vault-full`. The spawning half
//! runs the real sibling engine: build it first with `cargo build -p knowlu-engine -j 2`, because
//! `app/build.rs` leaves a zero-byte placeholder at `target/debug/knowlu-engine.exe`.

use std::path::{Path, PathBuf};
use serde_json::json;
use knowlu::state::ConsoleState;
use knowlu::week::{commitment_proposals_inner, commitments_confirm_inner, confirm_argv, preview_window_inner, proposals_argv, your_week_inner};

fn scratch(name: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!("qo-week-{name}-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    copy(Path::new("../engine/tests/fixtures/vault-full"), &dir);
    dir
}
fn copy(from: &Path, to: &Path) { std::fs::create_dir_all(to).unwrap(); for e in std::fs::read_dir(from).unwrap().flatten() { let p = e.path(); let t = to.join(e.file_name()); if p.is_dir() { copy(&p, &t); } else { std::fs::copy(&p, &t).unwrap(); } } }

fn open(v: &Path, name: &str) -> ConsoleState {
    ConsoleState::open(v.to_path_buf(), std::env::temp_dir().join(format!("qo-week-data-{name}-{}", std::process::id())))
}

/// `KNOWLU_ENGINE_EXE` is process-wide; this file's own lock and seam, `onboarding.rs`'s shape.
static ENGINE_ENV_LOCK: std::sync::Mutex<()> = std::sync::Mutex::new(());
struct EngineExeSeam(Option<std::ffi::OsString>);
impl EngineExeSeam {
    fn set(exe: &std::ffi::OsStr) -> Self {
        let prev = std::env::var_os("KNOWLU_ENGINE_EXE");
        unsafe { std::env::set_var("KNOWLU_ENGINE_EXE", exe) };
        EngineExeSeam(prev)
    }
}
impl Drop for EngineExeSeam {
    fn drop(&mut self) {
        match self.0.take() {
            Some(p) => unsafe { std::env::set_var("KNOWLU_ENGINE_EXE", p) },
            None => unsafe { std::env::remove_var("KNOWLU_ENGINE_EXE") },
        }
    }
}

#[test]
fn your_week_reports_setup_on_the_vaults_first_day_only_and_writes_nothing() {
    let v = scratch("yourweek");
    let cs = open(&v, "yourweek");
    // vault-full's earliest journal file is 2026-08-27.
    cs.set_test_today(Some("2026-08-27".parse().unwrap()));
    let env = your_week_inner(&cs);
    assert_eq!(env["ok"], true, "{env}");
    let week = &env["week"];
    assert_eq!(week["setup"], true, "{week}");
    assert_eq!(week["window"].as_array().unwrap().len(), 7);
    assert_eq!(week["window"][0]["source"], "template");
    assert_eq!(week["commitments"], json!([]));
    assert_eq!(week["uncovered_courses"], json!([]), "vault-full has no courses/");
    cs.set_test_today(Some("2026-08-28".parse().unwrap()));
    assert_eq!(your_week_inner(&cs)["week"]["setup"], false);
    assert!(!v.join("commitments").exists() && !v.join("state/calendar-series.json").exists());
}

#[test]
fn preview_window_reports_moved_and_refuses_a_bad_window() {
    let v = scratch("preview");
    let cs = open(&v, "preview");
    cs.set_test_today(Some("2026-08-28".parse().unwrap()));
    // vault-full's pinned day schedules work inside 08:00–18:00 (golden-today-full.md), so a
    // window of 21:00–21:59 moves or drops every take: `moved` must be present.
    let env = preview_window_inner(&cs, "[{days: [mon, tue, wed, thu, fri, sat, sun], start: \"21:00\", end: \"21:59\"}]");
    assert_eq!(env["ok"], true, "{env}");
    assert!(env["state"]["moved"]["text"].is_string(), "{}", env["state"]["moved"]);
    assert!(env["state"]["the_day"]["blocks"].is_array());
    let bad = preview_window_inner(&cs, "[{days: [fri], start: \"15:00\", end: \"14:00\"}]");
    assert_eq!(bad["ok"], false);
    assert!(bad["error"].as_str().unwrap().contains("planning day fri"), "{bad}");
    assert!(!v.join("commitments").exists(), "a preview writes nothing");
}

#[test]
fn the_argv_carries_the_vault_the_file_today_and_the_consoles_via() {
    let v = Path::new(r"C:\v");
    assert_eq!(proposals_argv(v), ["commitments", "--vault", r"C:\v", "--json"]);
    assert_eq!(
        confirm_argv(v, Path::new(r"C:\d\tmp\c.json"), "2026-09-24".parse().unwrap()),
        ["commitments", "--vault", r"C:\v", "--today", "2026-09-24", "--confirm", r"C:\d\tmp\c.json", "--via", "dashboard"]
    );
}

#[test]
fn commitment_proposals_with_no_engine_is_an_empty_list_and_a_reason() {
    let _guard = ENGINE_ENV_LOCK.lock().unwrap_or_else(|e| e.into_inner());
    let _seam = EngineExeSeam::set(std::ffi::OsStr::new(r"C:\nowhere\knowlu-engine.exe"));
    let v = scratch("noengine");
    let env = commitment_proposals_inner(&open(&v, "noengine"));
    assert_eq!(env["ok"], true, "no error wall: {env}");
    assert_eq!(env["proposals"], json!([]));
    assert_eq!(env["uncovered_courses"], json!([]));
    assert!(env["error"].is_string(), "{env}");
}

/// Ruling 11: the confirm call passes no `--actor`, so the engine writes as the vault's own token —
/// `student` on a vault that says so (the test below is the legacy half, on a vault with no file).
#[test]
fn commitments_confirm_on_a_student_vault_journals_the_student() {
    let exe = Path::new("../target/debug/knowlu-engine.exe");
    let len = std::fs::metadata(exe).map(|m| m.len()).unwrap_or(0);
    assert!(len > 0, "{}: run `cargo build -p knowlu-engine -j 2` first (build.rs leaves a zero-byte placeholder)", exe.display());
    let _guard = ENGINE_ENV_LOCK.lock().unwrap_or_else(|e| e.into_inner());
    let _seam = EngineExeSeam::set(exe.as_os_str());
    let v = scratch("confirm-student");
    knowlu_engine::journal::create_actor_file(&v, knowlu_engine::journal::HUMAN_ACTOR).unwrap();
    let cs = open(&v, "confirm-student");
    cs.set_test_today(Some("2026-08-28".parse().unwrap()));
    let given = json!({ "window": "[{days: [mon, tue, wed, thu, fri], start: \"08:00\", end: \"22:00\"}]" });
    let env = commitments_confirm_inner(&cs, "today", &given);
    assert_eq!(env["ok"], true, "{env}");
    let journal: String = std::fs::read_dir(v.join("state/journal")).unwrap().flatten().map(|e| std::fs::read_to_string(e.path()).unwrap()).collect();
    let planning: Vec<&str> = journal.lines().filter(|l| l.contains("commitments/planning-day.md")).collect();
    assert!(!planning.is_empty() && planning.iter().all(|l| l.contains("\"actor\": \"student\"")), "{journal}");
}

#[test]
fn commitments_confirm_against_the_real_engine_sets_the_planning_day_as_the_student() {
    let exe = Path::new("../target/debug/knowlu-engine.exe");
    let len = std::fs::metadata(exe).map(|m| m.len()).unwrap_or(0);
    assert!(len > 0, "{}: run `cargo build -p knowlu-engine -j 2` first (build.rs leaves a zero-byte placeholder)", exe.display());
    let _guard = ENGINE_ENV_LOCK.lock().unwrap_or_else(|e| e.into_inner());
    let _seam = EngineExeSeam::set(exe.as_os_str());
    let v = scratch("confirm");
    let cs = open(&v, "confirm");
    cs.set_test_today(Some("2026-08-28".parse().unwrap()));
    let given = json!({
        "mine": [{ "source_uid": "gcal-series:none", "level": "hard" }],
        "window": "[{days: [mon, tue, wed, thu, fri], start: \"08:00\", end: \"22:00\"}]",
    });
    let env = commitments_confirm_inner(&cs, "today", &given);
    assert_eq!(env["ok"], true, "{env}");
    assert_eq!(env["result"]["window"], "created");
    assert_eq!(env["result"]["created"], 0);
    assert!(env["result"]["warnings"][0].as_str().unwrap().starts_with("gcal-series:none: "), "{env}");
    assert_eq!(env["state"]["schema"], 1);
    let note = std::fs::read_to_string(v.join("commitments/planning-day.md")).unwrap();
    assert!(note.contains("kind: planning-day") && note.contains("confirmed_at: 2026-08-28"), "{note}");
    let journal: String = std::fs::read_dir(v.join("state/journal")).unwrap().flatten().map(|e| std::fs::read_to_string(e.path()).unwrap()).collect();
    assert!(journal.contains("commitments/planning-day.md") && journal.contains("\"actor\": \"quinn\""), "journaled as the student");
    let tmp = cs.data_dir.join("tmp");
    assert!(std::fs::read_dir(&tmp).map(|d| d.count()).unwrap_or(0) == 0, "the input file is deleted afterwards");
    let bad = commitments_confirm_inner(&cs, "today", &json!({ "window": "[{days: [mon], start: \"15:00\", end: \"14:00\"}]" }));
    assert_eq!(bad["ok"], false);
    assert!(bad["error"].as_str().unwrap().contains("planning day mon"), "{bad}");
    assert_eq!(std::fs::read_to_string(v.join("commitments/planning-day.md")).unwrap(), note, "nothing written");
}

/// The built engine, or a failed assertion naming the build step (`app/build.rs` leaves a
/// zero-byte placeholder at the same path).
fn real_engine() -> PathBuf {
    let exe = std::path::absolute("../target/debug/knowlu-engine.exe").unwrap();
    let len = std::fs::metadata(&exe).map(|m| m.len()).unwrap_or(0);
    assert!(len > 0, "{}: run `cargo build -p knowlu-engine -j 2` first (build.rs leaves a zero-byte placeholder)", exe.display());
    exe
}

const WINDOW: &str = "[{days: [mon, tue, wed, thu, fri], start: \"08:00\", end: \"22:00\"}]";

/// W1 P1P2-important. Since C3′ the slot's `sync` step is a child process that takes no
/// `vault_io`: `state/sync.lock` (`sync::RunLock`) is the only thing that keeps it apart, and
/// `commitments/` is a synced folder (P21). A confirm that ran while a sync held that lock would
/// write `commitments/` notes while a pull rewrote them. The test holds the same file with the
/// same exclusive OS lock `RunLock` takes, standing in for the slot's sync child.
#[test]
fn commitments_confirm_refuses_while_a_sync_holds_the_run_lock() {
    let exe = real_engine();
    let _guard = ENGINE_ENV_LOCK.lock().unwrap_or_else(|e| e.into_inner());
    let _seam = EngineExeSeam::set(exe.as_os_str());
    let v = scratch("confirm-busy");
    let cs = open(&v, "confirm-busy");
    cs.set_test_today(Some("2026-08-28".parse().unwrap()));
    let lock_path = v.join(knowlu_engine::sync::RUN_LOCK_FILE);
    std::fs::create_dir_all(lock_path.parent().unwrap()).unwrap();
    let held = std::fs::OpenOptions::new().create(true).write(true).open(&lock_path).unwrap();
    held.try_lock().unwrap();
    let given = json!({ "window": WINDOW });
    let env = commitments_confirm_inner(&cs, "today", &given);
    assert_eq!(env["ok"], false, "confirm ran under a sync's lock: {}", env["result"]);
    assert_eq!(env["error"], "a sync is running; try again in a moment");
    assert!(!v.join("commitments").exists(), "nothing written while a sync holds the lock");
    let tmp = cs.data_dir.join("tmp");
    assert!(std::fs::read_dir(&tmp).map(|d| d.count()).unwrap_or(0) == 0, "the input file is deleted on a refusal too");
    drop(held);
    let env = commitments_confirm_inner(&cs, "today", &given);
    assert_eq!(env["ok"], true, "the lock was the only reason: {}", env["error"]);
    assert!(v.join("commitments/planning-day.md").is_file());
}

/// W1 P1P2-important, the other half: the lock is held for the confirm child's whole run, so the
/// slot's `sync` child starting mid-confirm takes its named skip instead of writing under it.
/// Observed without timing: the stand-in engine, called as `commitments`, runs the REAL engine's
/// `sync` on the same vault, a separate process as the slot's is, and keeps what it printed.
#[test]
fn a_sync_started_during_commitments_confirm_takes_its_named_skip() {
    let real = real_engine();
    let root = std::env::temp_dir().join(format!("qo-week-standin-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&root);
    std::fs::create_dir_all(&root).unwrap();
    let bat = root.join("engine.bat");
    let said = root.join("sync-out.txt");
    // CRLF inside the file, as `scheduler.rs`'s stand-in has it. `%~3` is `--vault`'s value.
    std::fs::write(&bat, format!(
        "@echo off\r\nif not \"%1\"==\"commitments\" exit /b 9\r\n\"{}\" sync --vault \"%~3\" > \"%~dp0sync-out.txt\" 2>&1\r\nexit /b 0\r\n",
        real.display(),
    )).unwrap();
    let _guard = ENGINE_ENV_LOCK.lock().unwrap_or_else(|e| e.into_inner());
    let _seam = EngineExeSeam::set(bat.as_os_str());
    let v = scratch("confirm-during");
    let cs = open(&v, "confirm-during");
    cs.set_test_today(Some("2026-08-28".parse().unwrap()));
    let _ = commitments_confirm_inner(&cs, "today", &json!({ "window": WINDOW }));
    let during = std::fs::read_to_string(&said).expect("the stand-in ran the real sync");
    assert_eq!(during.trim(), "sync (skipped: another sync is running)", "a sync mid-confirm must not run");
    // The hold ends with the child: the next sync takes the lock (vault-full has no account).
    let after = std::process::Command::new(&real).args(["sync", "--vault"]).arg(&v).output().unwrap();
    assert_eq!(String::from_utf8_lossy(&after.stdout).trim(), "sync (skipped: no account)");
    let _ = std::fs::remove_dir_all(&root);
}
