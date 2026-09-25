//! Phase 2 of the commitment model (spec `docs/specs/2026-09-24-commitment-model-phase2-design.md`
//! §2, §4, §6): the confirm screen's and the *Schedule* view's four commands. Two read in-process
//! (`your_week`, `preview_window`). Two spawn the sibling engine the way
//! `onboarding::discover_coursework` does (`commitment_proposals`, `commitments_confirm`). Nothing
//! here computes: the engine derives every row, validates every window and writes every note.
use std::path::Path;
use serde_json::{json, Value};
use tauri::State;
use knowlu_engine::childproc::NoConsole;
use crate::scheduler::Scheduler;
use crate::state::ConsoleState;

pub fn proposals_argv(vault: &Path) -> Vec<String> {
    vec!["commitments".into(), "--vault".into(), vault.to_string_lossy().into_owned(), "--json".into()]
}

/// `--via dashboard`: the console's human context (`commands::console_ctx`); `--actor` keeps the
/// engine's default, `quinn`.
pub fn confirm_argv(vault: &Path, file: &Path, today: jiff::civil::Date) -> Vec<String> {
    vec![
        "commitments".into(), "--vault".into(), vault.to_string_lossy().into_owned(),
        "--today".into(), today.to_string(),
        "--confirm".into(), file.to_string_lossy().into_owned(),
        "--via".into(), "dashboard".into(),
    ]
}

/// `commitments::overview` in-process: no fetch, no write.
pub fn your_week_inner(cs: &ConsoleState) -> Value {
    let Ok(_g) = cs.lock.lock() else { return json!({ "ok": false, "error": "console lock poisoned", "week": Value::Null }) };
    let today = crate::commands::now_in(cs).date();
    json!({ "ok": true, "error": Value::Null, "week": knowlu_engine::commitments::overview(&cs.vault, today).to_json() })
}

/// The today view under a proposed window (`surface::build_state_preview`), whole (Plan ruling
/// Q9-c). An invalid window is the engine's own message. Writes nothing.
pub fn preview_window_inner(cs: &ConsoleState, window: &str) -> Value {
    let Ok(_g) = cs.lock.lock() else { return json!({ "ok": false, "error": "console lock poisoned", "state": Value::Null }) };
    let now = crate::commands::now_in(cs);
    let built = knowlu_engine::surface::build_state_preview(&cs.vault, knowlu_engine::surface::View::Today, now.date(), &now, cs.seen_at().as_deref(), window);
    match built.and_then(|s| serde_json::to_value(&s).map_err(|e| e.to_string())) {
        Ok(state) => json!({ "ok": true, "error": Value::Null, "state": state }),
        Err(e) => json!({ "ok": false, "error": e, "state": Value::Null }),
    }
}

/// `knowlu-engine commitments --json`: it fetches the calendars and refreshes the series file,
/// then lists the proposals and the uncovered courses. A failure is an empty list with the reason
/// in `error` (Plan ruling Q9-d).
pub fn commitment_proposals_inner(cs: &ConsoleState) -> Value {
    let empty = |why: String| json!({ "ok": true, "error": why, "proposals": [], "uncovered_courses": [], "warnings": [] });
    let exe = match crate::scheduler::engine_exe() { Ok(e) => e, Err(e) => return empty(e) };
    let out = match std::process::Command::new(exe).no_console().args(proposals_argv(&cs.vault)).output() {
        Ok(o) => o,
        Err(e) => return empty(format!("could not run the engine ({e})")),
    };
    match serde_json::from_slice::<Value>(&out.stdout) {
        Ok(v) if v.is_object() => json!({
            "ok": true, "error": Value::Null,
            "proposals": v["proposals"], "uncovered_courses": v["uncovered_courses"], "warnings": v["warnings"],
        }),
        _ => {
            let code = out.status.code().map(|c| c.to_string()).unwrap_or_else(|| "unknown".into());
            empty(format!("commitments exited with status {code}: {}", String::from_utf8_lossy(&out.stderr).trim()))
        }
    }
}

/// `knowlu-engine commitments --confirm <file>` (spec §3). The input file sits in the profile's
/// `tmp\` folder and is deleted afterwards. The state is rebuilt for `view`, like every write's.
pub fn commitments_confirm_inner(cs: &ConsoleState, view: &str, confirm: &Value) -> Value {
    let today = crate::commands::now_in(cs).date();
    let dir = cs.data_dir.join("tmp");
    let stamp = knowlu_engine::journal::now_ts(None).replace([':', '.', '-'], "");
    let file = dir.join(format!("confirm-{}-{stamp}.json", std::process::id()));
    let ran = run_confirm(cs, &dir, &file, confirm, today);
    let _ = std::fs::remove_file(&file);
    let (ok, error, result) = match ran {
        Err(e) => (false, json!(e), Value::Null),
        Ok(out) if out.status.success() => {
            cs.note_write();
            match serde_json::from_slice::<Value>(&out.stdout) {
                Ok(v) => (true, Value::Null, v),
                Err(e) => (false, json!(format!("commitments --confirm printed no report ({e})")), Value::Null),
            }
        }
        Ok(out) => {
            let why = String::from_utf8_lossy(&out.stderr).trim().trim_start_matches("knowlu-engine: ").to_string();
            (false, json!(why), Value::Null)
        }
    };
    let state = crate::commands::state_inner(cs, view).map(|env| env["state"].clone()).unwrap_or(Value::Null);
    json!({ "ok": ok, "error": error, "result": result, "state": state })
}

/// Writes the input and runs the child under `vault_io` (Plan ruling Q9-a).
fn run_confirm(cs: &ConsoleState, dir: &Path, file: &Path, confirm: &Value, today: jiff::civil::Date) -> Result<std::process::Output, String> {
    std::fs::create_dir_all(dir).map_err(|e| e.to_string())?;
    std::fs::write(file, serde_json::to_vec(confirm).map_err(|e| e.to_string())?).map_err(|e| e.to_string())?;
    let exe = crate::scheduler::engine_exe()?;
    let _io = cs.vault_io.lock().unwrap_or_else(|e| e.into_inner());
    std::process::Command::new(exe).no_console().args(confirm_argv(&cs.vault, file, today)).output().map_err(|e| e.to_string())
}

#[tauri::command(async)] pub fn commitment_proposals(cs: State<'_, ConsoleState>) -> Value { commitment_proposals_inner(&cs) }
#[tauri::command(async)] pub fn commitments_confirm(cs: State<'_, ConsoleState>, sch: State<'_, Scheduler>, view: String, confirm: Value) -> Value { let mut env = commitments_confirm_inner(&cs, &view, &confirm); let _ = crate::commands::attach_scheduler(&mut env, &sch); env }
#[tauri::command] pub fn your_week(cs: State<'_, ConsoleState>) -> Value { your_week_inner(&cs) }
#[tauri::command(async)] pub fn preview_window(cs: State<'_, ConsoleState>, window: String) -> Value { preview_window_inner(&cs, &window) }
