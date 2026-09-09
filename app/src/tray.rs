//! The tray icon: six menu items, a colour that reflects sync health, and a diagnostics blob a
//! friend can paste into a message (Knowlu plan 1, Task 7; *Settings* is plan 4a, Task 7).
//! `run-now` and `pause` are wired directly to the scheduler (Task 12) — no event round-trip.
use tauri::{menu::{Menu, MenuItem}, tray::TrayIconBuilder, AppHandle, Manager};
use crate::scheduler::{self, Scheduler};
use crate::state::{self, ConsoleState};

pub enum TrayState { Ok, Warn }

fn square(rgb: [u8; 3]) -> tauri::image::Image<'static> {
    let mut px = Vec::with_capacity(32 * 32 * 4);
    for y in 0..32 {
        for x in 0..32 {
            let edge = x < 2 || y < 2 || x > 29 || y > 29;
            if edge { px.extend_from_slice(&[0, 0, 0, 0]) } else { px.extend_from_slice(&[rgb[0], rgb[1], rgb[2], 255]) }
        }
    }
    tauri::image::Image::new_owned(px, 32, 32)
}

pub fn build(app: &AppHandle) -> tauri::Result<()> {
    let open = MenuItem::with_id(app, "open", "Open", true, None::<&str>)?;
    let run = MenuItem::with_id(app, "run-now", "Run now", true, None::<&str>)?;
    let pause = MenuItem::with_id(app, "pause", "Pause scheduling", true, None::<&str>)?;
    let settings = MenuItem::with_id(app, "settings", "Settings", true, None::<&str>)?;
    // Plan 4a Task 8: disabled until something is staged AND no slot is running — `check_for_updates`
    // and the housekeeping tick both enable it from `updates::update_offer`, never from a check here.
    let update = MenuItem::with_id(app, "update", "Restart to update", false, None::<&str>)?;
    let diag = MenuItem::with_id(app, "diag", "Copy diagnostics", true, None::<&str>)?;
    let quit = MenuItem::with_id(app, "quit", "Quit", true, None::<&str>)?;
    let menu = Menu::with_items(app, &[&open, &run, &pause, &settings, &update, &diag, &quit])?;
    if let Some(sch) = app.try_state::<Scheduler>() {
        *scheduler::lock(&sch.pause_item) = Some(pause.clone());
    }
    if let Some(up) = app.try_state::<crate::updates::Updates>() {
        *scheduler::lock(&up.item) = Some(update.clone());
    }
    TrayIconBuilder::with_id("main")
        .icon(square([0x3F, 0xB6, 0x8B]))
        .tooltip("Knowlu")
        .menu(&menu)
        .show_menu_on_left_click(false)
        .on_menu_event(|app, ev| match ev.id().as_ref() {
            "open" => { if let Some(w) = app.get_webview_window("main") { let _ = w.show(); let _ = w.set_focus(); } }
            "run-now" => {
                let h = app.clone();
                std::thread::spawn(move || { let _ = scheduler::run_slot(&h, false); });
            }
            "pause" => {
                let sch = app.state::<Scheduler>();
                let now_paused = !sch.paused.load(std::sync::atomic::Ordering::SeqCst);
                sch.paused.store(now_paused, std::sync::atomic::Ordering::SeqCst);
                let guard = scheduler::lock(&sch.pause_item);
                if let Some(item) = guard.as_ref() {
                    let _ = item.set_text(if now_paused { "Resume scheduling" } else { "Pause scheduling" });
                }
            }
            "settings" => {
                if let Some(w) = app.get_webview_window("main") {
                    let _ = w.show();
                    let _ = w.set_focus();
                    // One way into the panel, from both entry points: the page owns the panel and
                    // the tray only asks. `eval` rather than an event keeps the capability set as
                    // it is (no `core:event` grant needed for a menu click).
                    let _ = w.eval("window.KNOWLU_OPEN_SETTINGS && window.KNOWLU_OPEN_SETTINGS()");
                }
            }
            "update" => {
                // A plain std thread with no runtime under it, so the BLOCKING twin (B4 /
                // R-P4a-12): `tauri::async_runtime::block_on` is correct HERE and panics inside a
                // command. It also takes the same install hold the command does (S11), so a slot
                // cannot start under an install begun from the tray either.
                let h = app.clone();
                std::thread::spawn(move || {
                    let sch = h.state::<Scheduler>();
                    let up = h.state::<crate::updates::Updates>();
                    // An explicit action, so it clears the previous action's outcome on the way in
                    // and records its own (fix round 1, IMPORTANT 2).
                    crate::updates::clear_action_error(&up);
                    let Some(s) = scheduler::lock(&up.staged).clone() else { return };
                    // R-P4a-24: a tray click has no envelope to answer in, so a refusal or a
                    // failure that is only `let _ = …` is a menu item that visibly did nothing.
                    // Both are written to `last_action_error`, which the settings row shows and
                    // which `record_check` never overwrites — the first round wrote them to
                    // `last_error`, where the next check (every boot runs one) wiped them before
                    // any reader saw them. No dialog: the tray is not allowed to interrupt, and a
                    // failed install is not an emergency.
                    let Some(_hold) = crate::updates::hold_for_install(&sch) else {
                        crate::updates::note_error(&up, crate::updates::SLOT_RUNNING);
                        return;
                    };
                    if let Err(e) = crate::updates::install_staged_blocking(&h, &s) {
                        crate::updates::note_error(&up, &e);
                    }
                });
            }
            "diag" => {
                let cs = app.state::<ConsoleState>();
                let text = diagnostics_text(&cs);
                use tauri_plugin_clipboard_manager::ClipboardExt;
                let _ = app.clipboard().write_text(text);
            }
            "quit" => {
                let cs = app.state::<ConsoleState>();
                {
                    let mut s = cs.settings.lock().unwrap_or_else(|e| e.into_inner());
                    s.quit_at = Some(knowlu_engine::journal::now_ts(None));
                    let _ = s.save(&cs.settings_path);
                }
                // F10: push on close and back up on quit, synchronously, capped at 10 s. The
                // process ends inside `then`, which `quit_flush` calls before the scoped worker is
                // joined — so a hung remote costs the user ten seconds, never the whole join.
                let logs = cs.data_dir.join("logs");
                state::quit_flush(&cs, std::time::Duration::from_secs(10), move |q| {
                    let _ = std::fs::create_dir_all(&logs);
                    let stamp = knowlu_engine::journal::now_ts(None).replace(':', "");
                    let _ = std::fs::write(logs.join(format!("quit-{stamp}.txt")), format!("synced={} backed_up={} timed_out={}\n", q.synced, q.backed_up, q.timed_out));
                    std::process::exit(0);
                });
            }
            _ => {}
        })
        .on_tray_icon_event(|tray, ev| {
            if let tauri::tray::TrayIconEvent::DoubleClick { .. } = ev {
                if let Some(w) = tray.app_handle().get_webview_window("main") { let _ = w.show(); let _ = w.set_focus(); }
            }
        })
        .build(app)?;
    Ok(())
}

pub fn set_state(app: &AppHandle, s: TrayState) {
    if let Some(t) = app.tray_by_id("main") {
        let _ = t.set_icon(Some(match s {
            TrayState::Ok => square([0x3F, 0xB6, 0x8B]),
            TrayState::Warn => square([0xD9, 0xA4, 0x41]),
        }));
    }
}

/// Replaces any whitespace-separated token whose trimmed form ends in `.md` with `<note>`.
///
/// `RunRow.summary` is engine-authored text, not structured data (e.g.
/// `"today.md refreshed (5 active; 1 unreadable: bad.md)"` — `src/cli.rs`'s
/// `an_unreadable_note_is_counted_and_named_in_the_summary`), and a note's filename is its
/// slugified title. Trimming trailing punctuation before the `.md` check catches
/// `"bad.md)"`/`"bad.md,"` too; the replacement drops that punctuation along with the token,
/// which is fine — this is a diagnostics blob, not a rendering.
fn redact(s: &str) -> String {
    s.split(' ')
        .map(|tok| {
            let trimmed = tok.trim_end_matches(|c: char| !c.is_alphanumeric());
            if trimmed.to_ascii_lowercase().ends_with(".md") { "<note>" } else { tok }
        })
        .collect::<Vec<_>>()
        .join(" ")
}

/// Version, build, the last three run lines, the last sync/backup/settings error, the slot
/// config — and **no note content**: a friend pastes this into a message. Every line that can
/// carry engine-authored free text (a run summary, a sync/backup/settings error) goes through
/// `redact` first.
pub fn diagnostics_text(cs: &ConsoleState) -> String {
    let now = jiff::Timestamp::now();
    let runs = knowlu_engine::surface::runs_panel(&cs.vault, now);
    let mut out = format!("Knowlu {} build {}\n", env!("CARGO_PKG_VERSION"), crate::commands::CONSOLE_BUILD.unwrap_or("unknown"));
    if let Some(e) = &cs.settings_error { out.push_str(&redact(&format!("settings error: {e}"))); out.push('\n'); }
    for r in runs.recent.iter().take(3) {
        out.push_str(&redact(&format!("run {} {} {} {}", r.runner, r.started.clone().unwrap_or_default(), r.result, r.summary)));
        out.push('\n');
    }
    for e in &runs.expected {
        out.push_str(&redact(&format!("expected {} {} {}", e.runner, e.due, e.status)));
        out.push('\n');
    }
    if let Some(e) = &scheduler::lock(&cs.history).last_error { out.push_str(&redact(&format!("sync error: {e}"))); out.push('\n'); }
    if let Some(e) = &scheduler::lock(&cs.backup).last_error { out.push_str(&redact(&format!("backup error: {e}"))); out.push('\n'); }
    out
}
