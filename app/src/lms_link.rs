//! The LMS calendar link, captured through a sign-in window (spec §11a).
//!
//! **Knowlu never asks for a campus credential.** The student signs in on the university's own page,
//! in a window that has *no capability grant* — so that page cannot reach a single Tauri command —
//! and with its own data directory under the system temp folder, which `close_and_wipe` deletes. We
//! keep the calendar link. We keep nothing else: not the password, not the cookies, not the session.
use std::path::{Path, PathBuf};
use serde_json::{json, Value};
use tauri::{Manager, WebviewUrl, WebviewWindowBuilder};

/// The window's label. One at a time, by construction: `open` closes any previous one first.
pub const WINDOW: &str = "lms-signin";

/// **Not under `%LOCALAPPDATA%\knowlu`.** A campus session's cookies are not app data — they are
/// somebody's live login — so they go to a throwaway directory named for this process and this
/// capture, and they are deleted the moment the capture ends. `app/tests/lms_link.rs` pins that this
/// path is neither the app's own data root nor a profile folder.
pub fn session_dir() -> PathBuf {
    std::env::temp_dir().join(format!("knowlu-lms-session-{}-{}", std::process::id(), knowlu_engine::ids::new_id("cap")))
}

pub fn open_window_at(app: &tauri::AppHandle, url: &str, data_dir: &Path) -> Result<(), String> {
    close_window(app);
    let parsed: tauri::Url = url.parse().map_err(|e| format!("{url}: {e}"))?;
    let _ = std::fs::create_dir_all(data_dir);
    WebviewWindowBuilder::new(app, WINDOW, WebviewUrl::External(parsed))
        .title("Sign in to your school")
        .inner_size(1000.0, 760.0)
        .center()
        .data_directory(data_dir.to_path_buf())
        .build()
        .map(|_| ())
        .map_err(|e| e.to_string())
}

pub fn window_url(app: &tauri::AppHandle) -> Option<String> {
    app.get_webview_window(WINDOW).and_then(|w| w.url().ok()).map(|u| u.to_string())
}

pub fn navigate(app: &tauri::AppHandle, url: &str) -> Result<(), String> {
    let w = app.get_webview_window(WINDOW).ok_or_else(|| "the sign-in window is not open".to_string())?;
    let parsed: tauri::Url = url.parse().map_err(|e| format!("{url}: {e}"))?;
    w.navigate(parsed).map_err(|e| e.to_string())
}

/// Close the window and delete its data directory. Best effort on both: a window the user already
/// closed is not an error, and a directory WebView2 still has open is retried once the process ends
/// — which is why it lives in the temp folder rather than anywhere we would have to promise about.
pub fn close_and_wipe(app: &tauri::AppHandle, data_dir: &Path) {
    close_window(app);
    let _ = std::fs::remove_dir_all(data_dir);
}

fn close_window(app: &tauri::AppHandle) {
    if let Some(w) = app.get_webview_window(WINDOW) {
        let _ = w.close();
    }
}

#[tauri::command(async)]
pub fn open_lms_window(app: tauri::AppHandle, campus: String) -> Value {
    let Some(url) = crate::scaffold::CAMPUSES.iter().find(|(k, _, _)| *k == campus).map(|(_, _, u)| *u).filter(|u| !u.is_empty()) else {
        return json!({ "ok": false, "error": "no sign-in page is known for that school yet", "opened": false });
    };
    let dir = session_dir();
    match open_window_at(&app, url, &dir) {
        Ok(()) => json!({ "ok": true, "error": Value::Null, "opened": true, "session_dir": dir.to_string_lossy() }),
        Err(e) => json!({ "ok": false, "error": e, "opened": false }),
    }
}

#[tauri::command(async)]
pub fn close_lms_window(app: tauri::AppHandle, session_dir: String) -> Value {
    close_and_wipe(&app, Path::new(&session_dir));
    json!({ "ok": true, "error": Value::Null })
}

/// Filled by Task 14 with the spike's outcome. During the spike it reports what it can see, so the
/// run itself is the evidence.
#[tauri::command(async)]
pub fn capture_calendar_link(app: tauri::AppHandle, campus: String) -> Value {
    let _ = campus;
    json!({ "ok": true, "error": Value::Null, "url": window_url(&app), "spike": true })
}
