//! v0.4.0 window chrome (note §3, plan T6): the frameless `main` window, read as config and as
//! `main.rs` source text. The window cannot be started in a test (no GUI is launched here), so
//! these pins hold the shape the live proof then checks by `PrintWindow`: a window that starts
//! hidden and undecorated, a window-state plugin that never restores the native frame and never
//! touches `main` at creation (B1, N2), one show path run once after first paint or after 3 s
//! (N9), the Windows 11 border set to none (N8), and the page's exact capability list (N6).

use serde_json::Value;
use std::path::Path;

/// The CSP as it stood before v0.4.0 (note D5: unchanged).
const CSP: &str = "default-src 'self'; connect-src ipc: http://ipc.localhost; img-src 'self' data:; style-src 'self' 'unsafe-inline'; font-src 'self'";

fn conf() -> Value {
    serde_json::from_str(&std::fs::read_to_string("tauri.conf.json").unwrap()).unwrap()
}

fn main_rs() -> String {
    let path = Path::new(env!("CARGO_MANIFEST_DIR")).join("src").join("main.rs");
    std::fs::read_to_string(&path).unwrap_or_else(|e| panic!("{}: {e}", path.display()))
}

/// `s` with every whitespace character removed, so a pin does not depend on line breaks.
fn squash(s: &str) -> String {
    s.chars().filter(|c| !c.is_whitespace()).collect()
}

/// The body of `fn <name>` in `src`, from its opening brace to the matching closing one. Braces
/// inside string literals come in pairs in `main.rs` (format strings), so a plain count is enough.
fn fn_body<'a>(src: &'a str, name: &str) -> &'a str {
    let head = format!("fn {name}(");
    let found: Vec<usize> = src.match_indices(&head).map(|(i, _)| i).collect();
    assert_eq!(found.len(), 1, "main.rs defines `fn {name}` exactly once");
    let open = found[0] + src[found[0]..].find('{').expect("a function body");
    let mut depth = 0usize;
    for (i, c) in src[open..].char_indices() {
        match c {
            '{' => depth += 1,
            '}' => {
                depth -= 1;
                if depth == 0 {
                    return &src[open..=open + i];
                }
            }
            _ => {}
        }
    }
    panic!("`fn {name}` is never closed");
}

/// The two builders: `run_shell` (wizard and picker) and `run_console`.
const BUILDERS: [&str; 2] = ["run_shell", "run_console"];

#[test]
fn main_is_frameless_hidden_and_sized() {
    let conf = conf();
    let windows = conf["app"]["windows"].as_array().unwrap();
    let main: Vec<&Value> = windows.iter().filter(|w| w["label"] == "main").collect();
    assert_eq!(main.len(), 1, "one `main` window in tauri.conf.json");
    let w = main[0];
    assert_eq!(w["decorations"], Value::Bool(false), "main is frameless (ruling 4)");
    assert_eq!(w["visible"], Value::Bool(false), "main starts hidden and Rust shows it (N2, N9)");
    assert_eq!(w["width"], 1280, "default width (D6)");
    assert_eq!(w["height"], 800, "default height (D6)");
    assert_eq!(w["minWidth"], 820, "minimum width (D6)");
    assert_eq!(w["minHeight"], 600, "minimum height (D6)");
    assert_eq!(w["center"], Value::Bool(true), "main opens centered");
    assert!(w.get("backgroundColor").is_none(), "no fixed background: it flashes in one theme (M9)");
    assert_ne!(w["resizable"], Value::Bool(false), "the undecorated window still resizes at its edges");
    assert_eq!(conf["app"]["security"]["csp"], CSP, "the CSP is unchanged (D5)");
}

#[test]
fn both_window_state_builders_skip_main_and_never_restore_the_frame() {
    let src = main_rs();
    let flat = squash(&src);
    // `StateFlags::all() & !DECORATIONS & !VISIBLE` in the note; bitflags' `&` and `!` are trait
    // impls, not `const`, so the const is written with the const `difference`.
    assert!(
        flat.contains("constFLAGS:StateFlags=StateFlags::all().difference(StateFlags::DECORATIONS).difference(StateFlags::VISIBLE);"),
        "FLAGS is every state flag but DECORATIONS and VISIBLE (B1)"
    );
    // Every mention of the two flags is a removal: nothing restores the frame or shows the window.
    for flag in ["StateFlags::DECORATIONS", "StateFlags::VISIBLE"] {
        for (i, _) in flat.match_indices(flag) {
            assert!(flat[..i].ends_with(".difference("), "{flag} appears outside a removal in main.rs");
        }
    }
    assert_eq!(
        flat.matches("tauri_plugin_window_state::Builder").count(),
        2,
        "exactly one window-state plugin per builder"
    );
    for b in BUILDERS {
        let body = squash(fn_body(&src, b));
        assert!(
            body.contains(
                ".plugin(tauri_plugin_window_state::Builder::default().with_state_flags(FLAGS).skip_initial_state(\"main\").build())"
            ),
            "`{b}` registers the window-state plugin with FLAGS and skip_initial_state(\"main\") (N2)"
        );
    }
}

#[test]
fn both_builders_register_ui_ready_and_the_fallback() {
    let src = main_rs();
    for b in BUILDERS {
        let body = squash(fn_body(&src, b));
        assert!(
            body.contains("show_main_triggers(app.handle());"),
            "`{b}`'s setup registers the ui-ready listener and the 3 s fallback"
        );
    }
    let triggers = squash(fn_body(&src, "show_main_triggers"));
    assert!(triggers.contains(".listen(\"ui-ready\","), "the page's ui-ready event is listened for in Rust (no command)");
    assert!(triggers.contains("Duration::from_secs(3)"), "a 3 s fallback, so a script error never leaves an invisible app");
    assert!(triggers.contains("std::thread::spawn("), "the fallback waits on its own thread, never on the event loop");
    // Both triggers marshal the show path to the event loop's thread: a window-state change made
    // off the UI thread hung a prototype at startup.
    assert_eq!(triggers.matches("run_on_main_thread(").count(), 2, "each trigger runs show_main on the main thread");
    assert_eq!(triggers.matches("show_main(&").count(), 2, "each trigger calls show_main");
}

#[test]
fn show_main_runs_once_restores_maximizes_if_small_and_sets_the_border() {
    let src = main_rs();
    let flat = squash(&src);
    assert!(flat.contains("staticMAIN_SHOWN:AtomicBool=AtomicBool::new(false);"), "one AtomicBool per process");
    let body = squash(fn_body(&src, "show_main"));
    let at = |needle: &str| body.find(needle).unwrap_or_else(|| panic!("show_main lacks `{needle}`"));
    let once = at("MAIN_SHOWN.swap(true,");
    let restore = at(".restore_state(FLAGS)");
    let maximized = at(".is_maximized()");
    let work_area = at(".work_area()");
    let maximize = at(".maximize()");
    let border = at("DwmSetWindowAttribute(");
    let show = at(".show()");
    let focus = at(".set_focus()");
    assert!(once < restore, "the once-only check comes first");
    assert!(restore < maximized && maximized < maximize && work_area < maximize,
            "maximize only after the restore, and only when not maximized and the work area is smaller (D6)");
    assert!(maximize < border && border < show && show < focus, "the border is set before the window shows, then focus");
    assert!(body.contains("DWMWA_BORDER_COLOR"), "the border attribute is the named constant (N8)");
    assert!(body.contains("DWMWA_COLOR_NONE"), "the colour is the named constant (N8)");
    assert!(body.contains("HWND(") && body.contains(".hwnd()") && body.contains(".0)"),
            "tauri's windows-0.61 HWND is converted through its raw pointer (N8)");
}

/// **Ruling R-P4a-30**: the page is granted NO updater permission, and this test is how it stays
/// that way. Moved here from `static_assets.rs` by v0.4.0 (N6), so the swap's delete of that file
/// cannot take it, with the five window grants the drawn chrome needs (note §3).
///
/// The app drives `tauri-plugin-updater` from Rust - `updater_builder()` / `updater()` are plain
/// `Manager::state` (the plugin's `lib.rs`), which no capability gates - so the ONLY thing
/// `updater:default` would add is the plugin's four IPC commands. Two of those are
/// `allow-install` and `allow-download-and-install`, and `withGlobalTauri: true` means any script
/// running in the page could then call `invoke("plugin:updater|download_and_install")` and walk
/// straight past `update_offer`'s mid-run gate (R9) and `hold_for_install` (S11). The gate is only
/// a gate if it is the only door: `check_for_updates` and `install_update`.
///
/// The whole set is asserted, not just the absence, so a quiet widening of any grant shows up here.
/// No `allow-show`: Rust shows the window.
#[test]
fn the_page_is_granted_no_updater_permission() {
    let cap: serde_json::Value =
        serde_json::from_str(&std::fs::read_to_string("capabilities/default.json").unwrap()).unwrap();
    let perms: Vec<&str> = cap["permissions"].as_array().unwrap()
        .iter().map(|p| p.as_str().unwrap()).collect();
    assert!(!perms.iter().any(|p| p.starts_with("updater:")),
            "R-P4a-30: no updater permission reaches the page: {perms:?}");
    // No `window-state:default`: Rust drives the plugin, and a page call to `restoreStateCurrent()`
    // would bring back a saved `decorated: true` (B1).
    assert_eq!(perms, ["core:default", "autostart:default",
                       "clipboard-manager:allow-write-text",
                       "core:window:allow-minimize", "core:window:allow-toggle-maximize",
                       "core:window:allow-close", "core:window:allow-start-dragging",
                       "core:window:allow-set-theme"]);
    assert_eq!(cap["windows"].as_array().unwrap().len(), 1, "the console window, and only it");
}
