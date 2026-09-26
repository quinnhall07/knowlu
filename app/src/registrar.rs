//! Phase 3 of the commitment model (spec `docs/specs/2026-09-26-commitment-model-phase3-design.md`
//! D1, D2, D4, §2): the school's registrar, read through the school's own sign-in window.
//!
//! **Knowlu never asks for a campus credential here either.** The window is `lms_link`'s own:
//! incognito, a throwaway data directory, no capability grant (Plan ruling R4-a). Its cookies are
//! read for the registrar host only and live on `capture`'s stack; the fetched bytes go to one
//! temp file that the engine reads and this module deletes. Nothing here parses them.
use std::path::Path;
use std::time::{Duration, SystemTime};
use jiff::civil::Date;
use serde_json::{json, Value};
use tauri::{Manager, State};
use knowlu_engine::childproc::NoConsole;
use crate::lms_link;
use crate::scaffold::Registrar;
use crate::scheduler::Scheduler;
use crate::state::ConsoleState;

const NO_REGISTRAR: &str = "Knowlu can't read your school's class schedule yet";

/// The vault's school, from `config/campus.yaml`'s `unitid` (Plan ruling R4-c), and its registrar.
pub fn school_of(vault: &Path) -> Option<&'static Registrar> {
    let text = std::fs::read_to_string(vault.join("config").join("campus.yaml")).ok()?;
    let doc: serde_yaml_ng::Value = serde_yaml_ng::from_str(&text).ok()?;
    let unitid = match doc.get("unitid")? {
        serde_yaml_ng::Value::String(s) => s.trim().to_string(),
        serde_yaml_ng::Value::Number(n) => n.to_string(),
        _ => return None,
    };
    crate::scaffold::curated(&unitid)?.registrar.as_ref()
}

pub fn start_url(reg: &Registrar) -> String {
    format!("https://{}{}{}", reg.host, reg.prefix, reg.start)
}

/// `(method, url, form)` per call, `{term}` filled in.
pub fn call_urls(reg: &Registrar, term: &str) -> Vec<(String, String, Option<String>)> {
    reg.calls
        .iter()
        .map(|c| {
            let url = format!("https://{}{}{}", reg.host, reg.prefix, c.path.replace("{term}", term));
            (c.method.to_string(), url, c.form.map(|f| f.replace("{term}", term)))
        })
        .collect()
}

/// `--via dashboard`, and `--actor` keeps the engine's default, `quinn`: the R24 notes are the
/// student's (spec §3, Plan ruling R3-c), as `week::confirm_argv`'s are.
pub fn registrar_argv(vault: &Path, file: &Path, school: &str, today: Date) -> Vec<String> {
    vec![
        "commitments".into(), "--vault".into(), vault.to_string_lossy().into_owned(),
        "--today".into(), today.to_string(),
        "--registrar".into(), file.to_string_lossy().into_owned(),
        "--school".into(), school.into(),
        "--via".into(), "dashboard".into(),
    ]
}

/// Writes the fetched bytes to `<profile>\tmp\`, runs `commitments --registrar` under `vault_io`,
/// deletes the file whatever happened, and rebuilds the state for `view`. `closed` is always true
/// here: the capture closes the window after this returns (Plan ruling R4-d).
pub fn run_file(cs: &ConsoleState, view: &str, school: &str, bytes: &[u8], today: Date) -> Value {
    let dir = cs.data_dir.join("tmp");
    let stamp = knowlu_engine::journal::now_ts(None).replace([':', '.', '-'], "");
    let file = dir.join(format!("registrar-{}-{stamp}.json", std::process::id()));
    let ran = spawn(cs, &dir, &file, school, bytes, today);
    let _ = std::fs::remove_file(&file);
    let (ok, error, result) = match ran {
        Err(e) => (false, json!(e), Value::Null),
        Ok(out) if out.status.success() => {
            cs.note_write();
            match serde_json::from_slice::<Value>(&out.stdout) {
                Ok(v) => (true, Value::Null, v),
                Err(_) => (false, json!("the engine printed no report"), Value::Null),
            }
        }
        Ok(out) => {
            let why = String::from_utf8_lossy(&out.stderr).trim().trim_start_matches("knowlu-engine: ").to_string();
            (false, json!(why), Value::Null)
        }
    };
    let state = crate::commands::state_inner(cs, view).map(|env| env["state"].clone()).unwrap_or(Value::Null);
    json!({ "ok": ok, "error": error, "closed": true, "result": result, "state": state })
}

fn spawn(cs: &ConsoleState, dir: &Path, file: &Path, school: &str, bytes: &[u8], today: Date) -> Result<std::process::Output, String> {
    std::fs::create_dir_all(dir).map_err(|e| e.to_string())?;
    std::fs::write(file, bytes).map_err(|e| e.to_string())?;
    let exe = crate::scheduler::engine_exe()?;
    let _io = cs.vault_io.lock().unwrap_or_else(|e| e.into_inner());
    std::process::Command::new(exe).no_console().args(registrar_argv(&cs.vault, file, school, today)).output().map_err(|e| e.to_string())
}

/// Schedule files a crash left in `<profile>\tmp\` (R4 review M2): `registrar-*.json` older than
/// `older_than`, and nothing else. A fresh one is a run in progress. Returns how many went.
pub fn sweep_stale_files_in(dir: &Path, now: SystemTime, older_than: Duration) -> usize {
    let Ok(entries) = std::fs::read_dir(dir) else { return 0 };
    let mut swept = 0;
    for entry in entries.flatten() {
        let name = entry.file_name();
        let Some(name) = name.to_str() else { continue };
        if !(name.starts_with("registrar-") && name.ends_with(".json")) { continue; }
        let stale = entry.metadata().ok().and_then(|m| m.modified().ok())
            .and_then(|t| now.duration_since(t).ok()).is_some_and(|age| age >= older_than);
        if stale && std::fs::remove_file(entry.path()).is_ok() { swept += 1; }
    }
    swept
}

/// A failed request in fixed words, by kind (R4 review M1). ureq's own text can carry the URL, so
/// it never reaches the envelope.
pub fn fetch_error(label: &str, e: &ureq::Error) -> String {
    match e {
        ureq::Error::StatusCode(_) | ureq::Error::BodyExceedsLimit(_) => format!("{label} answered with an error"),
        ureq::Error::Timeout(_) | ureq::Error::BodyStalled => format!("{label} took too long"),
        _ => format!("couldn't reach {label}"),
    }
}

fn failed(error: String, closed: bool) -> Value {
    json!({ "ok": false, "error": error, "closed": closed, "result": Value::Null, "state": Value::Null })
}

/// Spec §2 step 2: the recorded calls, with the window's cookies for each call's host only
/// (`cookie_url`). The cookies live on this stack: never written, logged or put in an error.
fn capture(app: &tauri::AppHandle, cs: &ConsoleState, view: &str) -> Value {
    // The envelope says `closed`, so the window really is closed (plan review M1).
    let Some(reg) = school_of(&cs.vault) else { lms_link::close_and_wipe(app); return failed(NO_REGISTRAR.into(), true) };
    let Some(w) = app.get_webview_window(lms_link::WINDOW) else {
        return failed(format!("the sign-in window is not open; press Get my class times from {} again", reg.label), true);
    };
    let today = crate::commands::now_in(cs).date();
    let Some(term) = knowlu_engine::registrar::term_for(reg.school, today) else { return failed(NO_REGISTRAR.into(), false) };
    let here = w.url().ok();
    let agent = lms_link::session_agent();
    let mut body = String::new();
    for (method, url, form) in call_urls(reg, &term) {
        let Ok(jar_url) = lms_link::cookie_url(here.as_ref().map(|u| u.as_str()), &url).parse::<tauri::Url>() else {
            return failed(format!("{} could not be read (a bad address)", reg.label), false);
        };
        let jar: String = match w.cookies_for_url(jar_url) {
            Ok(cookies) => cookies.iter().map(|c| format!("{}={}", c.name(), c.value())).collect::<Vec<_>>().join("; "),
            Err(_) => return failed("the sign-in could not be read".into(), false),
        };
        let sent = match method.as_str() {
            "POST" => agent.post(&url).header("cookie", &jar).header("accept", "application/json")
                .header("x-requested-with", "XMLHttpRequest").header("content-type", "application/x-www-form-urlencoded")
                .send(form.unwrap_or_default()),
            _ => agent.get(&url).header("cookie", &jar).header("accept", "application/json")
                .header("x-requested-with", "XMLHttpRequest").call(),
        };
        match sent.and_then(|mut r| r.body_mut().with_config().limit(1 << 22).read_to_string()) {
            Ok(text) => body = text,
            Err(e) => return failed(fetch_error(reg.label, &e), false),
        }
    }
    if knowlu_engine::registrar::looks_signed_out(&body) {
        return failed(format!("You're not signed in yet. Finish signing in to {} in the window, then press I'm signed in.", reg.label), false);
    }
    let env = run_file(cs, view, reg.school, body.as_bytes(), today);
    lms_link::close_and_wipe(app);
    env
}

#[tauri::command(async)]
pub fn open_registrar_window(app: tauri::AppHandle, cs: State<'_, ConsoleState>) -> Value {
    lms_link::sweep_stale_sessions();
    sweep_stale_files_in(&cs.data_dir.join("tmp"), SystemTime::now(), lms_link::STALE_AFTER);
    let Some(reg) = school_of(&cs.vault) else { return json!({ "ok": false, "error": NO_REGISTRAR, "opened": false }) };
    match lms_link::open_window_at(&app, &start_url(reg), &lms_link::session_dir()) {
        Ok(()) => json!({ "ok": true, "error": Value::Null, "opened": true }),
        Err(e) => json!({ "ok": false, "error": e, "opened": false }),
    }
}

#[tauri::command(async)]
pub fn capture_registrar(app: tauri::AppHandle, cs: State<'_, ConsoleState>, sch: State<'_, Scheduler>, view: String) -> Value {
    let mut env = capture(&app, &cs, &view);
    let _ = crate::commands::attach_scheduler(&mut env, &sch);
    env
}

#[tauri::command(async)]
pub fn close_registrar_window(app: tauri::AppHandle) -> Value {
    lms_link::close_and_wipe(&app);
    json!({ "ok": true, "error": Value::Null })
}
