#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]
use tauri::Manager;
use knowlu::{account, commands, lms_link, onboarding, profiles, report, scheduler, state::{app_data_root, resolve_vault, ConsoleState}, tray};

/// A missing/bad vault is fatal before any window exists, so it has to reach the user some way
/// other than a console that may not be attached (the exe carries `windows_subsystem = "windows"`
/// in release) — hence the `MessageBoxW`, not just the `eprintln!`.
fn fatal(msg: &str) -> ! {
    eprintln!("Knowlu: {msg}");
    #[cfg(windows)]
    unsafe {
        use windows::Win32::UI::WindowsAndMessaging::{MessageBoxW, MB_ICONERROR, MB_OK};
        use windows::core::PCWSTR;
        let text: Vec<u16> = msg.encode_utf16().chain(std::iter::once(0)).collect();
        let title: Vec<u16> = "Knowlu".encode_utf16().chain(std::iter::once(0)).collect();
        MessageBoxW(None, PCWSTR(text.as_ptr()), PCWSTR(title.as_ptr()), MB_OK | MB_ICONERROR);
    }
    std::process::exit(2)
}

fn main() {
    let args: Vec<String> = std::env::args().collect();
    let vault_arg = args.iter().position(|a| a == "--vault").and_then(|i| args.get(i + 1)).cloned();

    // R-P4a-1: a relaunch for a profile switch waits, bounded, for the process that spawned it to
    // exit — `tauri-plugin-single-instance` is keyed on the app identifier, so registering before
    // the parent is gone gets THIS window killed instead of the old one. Bounded at 10 s; past
    // that the launch goes ahead anyway and the plugin closes whichever loses.
    if let Some(pid) = args.iter().position(|a| a == "--after-pid").and_then(|i| args.get(i + 1)).and_then(|s| s.parse::<u32>().ok()) {
        profiles::wait_for_pid_gone(pid, std::time::Duration::from_secs(10));
    }

    // Hidden manual-test path (Knowlu plan 1, Task 12): one scheduler slot, synchronously, no
    // window and no tray, summary as JSON on stdout. It is never reached by a normal launch, it
    // always carries `--vault`, and a bad one there IS fatal — nobody is watching a window.
    if args.iter().any(|a| a == "--run-slot-once") {
        let cwd = std::env::current_dir().unwrap_or_default();
        let vault = match resolve_vault(vault_arg.as_deref(), &cwd) { Ok(v) => v, Err(msg) => fatal(&msg) };
        let data = std::env::temp_dir().join("knowlu-run-slot-once-appdata");
        let cs = ConsoleState::open(vault, data);
        let sch = scheduler::Scheduler::default();
        let summary = scheduler::run_slot_inner(&cs, &sch, None, false);
        println!("{}", serde_json::to_string(&summary).unwrap_or_default());
        let code = if summary.reason.is_some() { 2 } else if summary.ok { 0 } else { 1 };
        std::process::exit(code);
    }

    // F14, and it must run before anything creates the new root: the fold moves only the app's own
    // flat files into `knowlu\profiles\<id>\` (R-P4a-28) and uses the raw `--vault` argument, if any,
    // as the hint for the one-entry registry it writes. Whatever it
    // could not do is printed here rather than swallowed (review round 1, IMPORTANT 2) — the same
    // `Knowlu: <text>` shape `ConsoleState::open` uses for a settings file it could not read.
    if let Some(base) = std::env::var("LOCALAPPDATA").ok().filter(|s| !s.is_empty()) {
        if let Some(m) = profiles::migrate_flat_layout(std::path::Path::new(&base), vault_arg.as_deref().map(std::path::Path::new)) {
            for e in &m.errors { eprintln!("Knowlu: {e}"); }
        }
    }
    let root = match app_data_root() { Some(r) => r, None => std::env::temp_dir().join("knowlu") };

    // `--pick` forces the picker whatever the registry holds: it is how *Switch profile…* in the
    // settings overlay gets back to the chooser with one profile registered, or none (R-P4a-15).
    let launch = if args.iter().any(|a| a == "--pick") {
        match profiles::load(&root) { Ok(ps) => profiles::Launch::Pick(ps), Err(e) => profiles::Launch::Broken(e) }
    } else {
        profiles::resolve_launch(&root, vault_arg.as_deref().map(std::path::Path::new))
    };
    match launch {
        // S2: the profile is registered only AFTER the vault has been validated — a `--vault` typo
        // must not leave a broken entry in `profiles.json` that the picker then offers forever.
        profiles::Launch::Open(p) => run_console(p, root),
        profiles::Launch::Onboard => run_shell(root, "wizard", Vec::new()),
        profiles::Launch::Pick(ps) => run_shell(root, "picker", ps),
        // A registry that exists and cannot be read stops the launch dead (review round 1,
        // IMPORTANT 3). Every other outcome would eventually call `register`, which rewrites the
        // file — so treating an unreadable registry as an empty one silently deletes every profile
        // on the machine. Nothing is written; the user is told which file and why.
        profiles::Launch::Broken(e) => fatal(&format!(
            "{e}\n\nKnowlu will not rewrite a profile registry it cannot read, because that would lose every profile on this machine. Repair or move that file, then start Knowlu again."
        )),
    }
}

/// The vault-less shell: the same window and the same page, no `ConsoleState`, no tray, no
/// scheduler, no autostart registration — none of those has a vault to point at yet. It ends by
/// relaunching into `run_console` (`open_profile` / `finish_onboarding`), never by managing a
/// `ConsoleState` after the fact.
fn run_shell(root: std::path::PathBuf, mode: &'static str, ps: Vec<profiles::Profile>) -> ! {
    tauri::Builder::default()
        .plugin(tauri_plugin_single_instance::init(|app, _args, _cwd| { if let Some(w) = app.get_webview_window("main") { let _ = w.show(); let _ = w.set_focus(); } }))
        .plugin(tauri_plugin_window_state::Builder::default().build())
        .setup(move |app| {
            app.manage(onboarding::Onboarding { root, mode, profiles: std::sync::Mutex::new(ps) });
            // R-C1-40 (I1b): the app holds the sign-in window's session directory, so wiping it does
            // not depend on the page remembering to ask. Nothing else reads this state.
            app.manage(lms_link::LmsSession::default());
            Ok(())
        })
        // …and the window's own close button ends a capture just as `close_lms_window` does.
        // `Destroyed`, not `CloseRequested`: the wipe wants the webview gone before it starts.
        // Scoped to the sign-in window's label — every other window this builder makes is the
        // wizard's own and is not a campus session.
        .on_window_event(|w, e| {
            if w.label() == lms_link::WINDOW && matches!(e, tauri::WindowEvent::Destroyed) {
                lms_link::wipe_session(w.app_handle());
            }
        })
        .invoke_handler(tauri::generate_handler![onboarding::launch_state, onboarding::pick_folder, onboarding::pick_file, onboarding::adopt_vault, onboarding::open_profile, onboarding::create_vault, onboarding::restore_vault, onboarding::apply_profile_settings, onboarding::store_credentials, onboarding::retarget_credentials, onboarding::finish_onboarding, account::google_sign_in, account::send_magic_link, account::verify_email_code, account::sign_out, account::open_policy, account::entitlement_now, account::open_checkout, account::google_connect_url, account::google_connected, account::open_external, lms_link::open_lms_window, lms_link::capture_calendar_link, lms_link::capture_courses, lms_link::paste_calendar_link, lms_link::close_lms_window, onboarding::discover_coursework, onboarding::timezone_for_state, onboarding::campus_search])
        .build(tauri::generate_context!())
        .expect("Knowlu: failed to start the Tauri runtime")
        // A session still open when the shell exits goes with it (R-C1-40, I1b). Anything this
        // cannot finish in time is swept by the next launch's `open_lms_window`.
        .run(|app, event| { if matches!(event, tauri::RunEvent::Exit) { lms_link::wipe_session_on_exit(app); } });
    std::process::exit(0)
}

/// The console over one profile. `resolve_vault` runs HERE, so a registered profile whose folder
/// has since been deleted never panics — and the registry is only touched once that has passed (S2).
///
/// **An unopenable vault falls back to the picker** (final review, I1). A moved, renamed or deleted
/// vault folder used to end the launch at a message box: the friend whose vault lives on a USB stick
/// they had not plugged in could not reach their other profile, and could not repair this one,
/// because the only window Knowlu was ever going to show was the error. `profiles::fallback_shell`
/// decides it — the picker whenever a profile is registered — and the fatal stays for the case where
/// there is nothing to pick from.
fn run_console(p: profiles::Profile, root: std::path::PathBuf) -> ! {
    let cwd = std::env::current_dir().unwrap_or_default();
    // A vault path that is not valid UTF-8 must NOT fall through to `resolve_vault(None, cwd)`,
    // which would quietly open whatever vault the current directory happens to be — a different
    // person's notes, on a shared machine (review round 1, minor). `to_str()` failing is fatal.
    let Some(arg) = p.vault.to_str() else {
        fatal(&format!("vault path is not valid UTF-8, so Knowlu cannot open it: {}", p.vault.display()))
    };
    let vault = match resolve_vault(Some(arg), &cwd) {
        Ok(v) => v,
        Err(msg) => match profiles::fallback_shell(&root) {
            Some(ps) => { eprintln!("Knowlu: {msg}"); run_shell(root, "picker", ps) }
            None => fatal(&msg),
        },
    };
    // The registry write is refused, loudly, when the registry is unreadable (IMPORTANT 3).
    if let Err(e) = profiles::register(&root, &p.name, &vault) { fatal(&e); }
    let data_dir = profiles::profile_dir(&root, &p.id);
    // `Box::leak` because `tauri_plugin_autostart` wants `&'static str` for the arguments it
    // registers — unchanged from today, and the reason autostart relaunches THIS profile.
    let vault_str: &'static str = Box::leak(vault.to_string_lossy().into_owned().into_boxed_str());
    tauri::Builder::default()
        .plugin(tauri_plugin_single_instance::init(|app, _args, _cwd| { if let Some(w) = app.get_webview_window("main") { let _ = w.show(); let _ = w.set_focus(); } }))
        .plugin(tauri_plugin_window_state::Builder::default().build())
        .plugin(tauri_plugin_clipboard_manager::init())
        .plugin(tauri_plugin_autostart::init(tauri_plugin_autostart::MacosLauncher::LaunchAgent, Some(vec!["--vault", vault_str])))
        // Plan 4a Task 8's key-gated step, and **on this builder only**: `run_shell` has no vault,
        // no tray, no slot to defer an install to and no window that should be offering one. The
        // plugin's `setup` deserializes `plugins.updater` from `tauri.conf.json` at startup and
        // `pubkey` has no serde default, so this line and that config block are one commit
        // (R-P4a-13) — registering without it does not fail a test, it fails the launch.
        .plugin(tauri_plugin_updater::Builder::new().build())
        .setup(move |app| {
            let cs = ConsoleState::open(vault.clone(), data_dir.clone());
            let autostart = cs.settings.lock().unwrap().autostart;
            app.manage(cs);
            app.manage(scheduler::Scheduler::default());
            // Plan 4a Task 8, and BEFORE `tray::build`: the tray stashes its *Restart to update*
            // item in here, and `try_state` would find nothing if this came after.
            app.manage(knowlu::updates::Updates::default());
            tray::build(app.handle())?;
            use tauri_plugin_autostart::ManagerExt;
            let al = app.autolaunch();
            if autostart { let _ = al.enable(); } else { let _ = al.disable(); }
            // The scheduler's tick and housekeeping threads (Task 12) — inert unless
            // `config/runners.yaml`'s `local` entry says `scheduler: app`.
            scheduler::spawn(app.handle().clone());
            Ok(())
        })
        // Closing the window hides it to the tray instead of ending the process — Knowlu keeps
        // running so the scheduler (Task 12) can still fire. Quit is the tray's job.
        .on_window_event(|w, e| { if let tauri::WindowEvent::CloseRequested { api, .. } = e { api.prevent_close(); let _ = w.hide(); } })
        .invoke_handler(tauri::generate_handler![commands::state, commands::note, commands::mark_seen, commands::ui_event, commands::set_fields, commands::create_task, commands::delete_note, commands::decide, commands::close_info, commands::open_issue, commands::resolve_issue, commands::sync, commands::backup_now, commands::get_settings, commands::set_settings, commands::set_profile_name, commands::copy_diagnostics, commands::copy_text, commands::settings_context, commands::switch_profile, commands::check_for_updates, commands::install_update, commands::inference_status, commands::install_inference_file, commands::install_inference_download, commands::remove_inference_model, onboarding::launch_state, onboarding::pick_folder, onboarding::pick_file, account::google_sign_in, account::send_magic_link, account::verify_email_code, account::sign_out, account::open_policy, account::entitlement_now, account::open_checkout, account::account_status, account::open_portal, account::attach_account, account::delete_my_data, report::report_preview, report::report_send])
        .run(tauri::generate_context!())
        .expect("Knowlu: failed to start the Tauri runtime");
    std::process::exit(0)
}
