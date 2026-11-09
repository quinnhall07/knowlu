//! The two `generate_handler!` lists in `app/src/main.rs`, read as source text (Gmail connect plan
//! P6, spec §8.1 item 6). A Tauri command in the wrong list compiles and fails only at run time,
//! when the page's `invoke` answers "command not found"; this test is the compile-time stand-in.
//! It lives in its own file so a non-credential test never falls under `account.rs`'s
//! contract-list treatment.

use std::path::Path;

/// Every `generate_handler![…]` list in `main.rs`, each as its trimmed command paths.
fn handler_lists(src: &str) -> Vec<Vec<String>> {
    const OPEN: &str = "generate_handler![";
    let mut lists = Vec::new();
    let mut rest = src;
    while let Some(at) = rest.find(OPEN) {
        let body = &rest[at + OPEN.len()..];
        let end = body.find(']').expect("a generate_handler! list is closed with ]");
        let names = body[..end].split(',').map(|n| n.trim().to_string()).filter(|n| !n.is_empty()).collect();
        lists.push(names);
        rest = &body[end..];
    }
    lists
}

/// The list holding `marker`, found by content rather than by position in the file.
fn list_with<'a>(lists: &'a [Vec<String>], marker: &str) -> &'a [String] {
    let found: Vec<&Vec<String>> = lists.iter().filter(|l| l.iter().any(|n| n == marker)).collect();
    assert_eq!(found.len(), 1, "exactly one generate_handler! list names {marker}");
    found[0]
}

const GOOGLE: [&str; 3] = ["account::google_status", "account::google_connect", "account::google_disconnect"];

#[test]
fn the_console_list_names_the_three_google_commands_and_the_wizard_list_none() {
    let path = Path::new(env!("CARGO_MANIFEST_DIR")).join("src").join("main.rs");
    let src = std::fs::read_to_string(&path).unwrap_or_else(|e| panic!("{}: {e}", path.display()));
    let lists = handler_lists(&src);
    assert_eq!(lists.len(), 2, "main.rs has the wizard's list and the console's list");

    // The console list is the one that serves the page (`commands::state`); the wizard's is the one
    // that finishes onboarding. Neither is picked by its position in the file.
    let console = list_with(&lists, "commands::state");
    let wizard = list_with(&lists, "onboarding::finish_onboarding");

    for name in GOOGLE {
        assert!(console.iter().any(|n| n == name), "the console list must name {name}");
        assert!(!wizard.iter().any(|n| n == name), "the wizard list must not name {name}");
    }
    assert!(
        !console.iter().any(|n| n == "account::open_external"),
        "the console list must still not name account::open_external"
    );
}

/// P17 (Quinn, Checkpoint B): `remove_lane_date` is the one command events adds, in the console's
/// list only.
#[test]
fn the_console_list_names_remove_lane_date_and_the_wizard_list_does_not() {
    let path = Path::new(env!("CARGO_MANIFEST_DIR")).join("src").join("main.rs");
    let src = std::fs::read_to_string(&path).unwrap_or_else(|e| panic!("{}: {e}", path.display()));
    let lists = handler_lists(&src);
    let console = list_with(&lists, "commands::state");
    let wizard = list_with(&lists, "onboarding::finish_onboarding");
    assert!(console.iter().any(|n| n == "commands::remove_lane_date"), "the console list must name commands::remove_lane_date");
    assert!(!wizard.iter().any(|n| n == "commands::remove_lane_date"), "the wizard list must not name commands::remove_lane_date");
}

/// Email spec test 37 (plan T11.4; H5 puts the names in the list, §10 R5): the ten `mail_*` commands are
/// in the console's list and none in the wizard's. The wizard has no profile folder and no `ConsoleState`.
const MAIL: [&str; 10] = [
    "mail::mail_status",
    "mail::mail_setup_start",
    "mail::mail_setup_progress",
    "mail::mail_setup_answer",
    "mail::mail_setup_stop",
    "mail::mail_sign_in",
    "mail::mail_disconnect",
    "mail::mail_rotate",
    "mail::mail_keep_session",
    "mail::mail_applied",
];

#[test]
fn the_ten_mail_commands_are_in_the_console_list_and_none_in_the_wizards() {
    let path = Path::new(env!("CARGO_MANIFEST_DIR")).join("src").join("main.rs");
    let src = std::fs::read_to_string(&path).unwrap_or_else(|e| panic!("{}: {e}", path.display()));
    let lists = handler_lists(&src);
    let console = list_with(&lists, "commands::state");
    let wizard = list_with(&lists, "onboarding::finish_onboarding");
    for name in MAIL {
        assert!(console.iter().any(|n| n == name), "the console list must name {name}");
        assert!(!wizard.iter().any(|n| n == name), "the wizard list must not name {name}");
    }
    let named = console.iter().filter(|n| n.starts_with("mail::")).count();
    assert_eq!(named, MAIL.len(), "the console list names exactly the ten mail commands, no more");
}

/// Ruling 10 (L6): no command in either list reaches a local model, and `onboarding::pick_file`
/// stays in both lists (L5's syllabus Offer B uses it).
#[test]
fn the_console_list_names_no_inference_command_and_both_lists_keep_pick_file() {
    let path = Path::new(env!("CARGO_MANIFEST_DIR")).join("src").join("main.rs");
    let src = std::fs::read_to_string(&path).unwrap_or_else(|e| panic!("{}: {e}", path.display()));
    let lists = handler_lists(&src);
    let console = list_with(&lists, "commands::state");
    let wizard = list_with(&lists, "onboarding::finish_onboarding");
    for (which, list) in [("console", console), ("wizard", wizard)] {
        let bad: Vec<&String> = list.iter().filter(|n| n.contains("inference")).collect();
        assert!(bad.is_empty(), "the {which} list still names local-model commands: {bad:?}");
        assert!(list.iter().any(|n| n == "onboarding::pick_file"), "the {which} list must keep onboarding::pick_file");
    }
}

/// L4 (T5): the two privacy commands are in the console's list only. The wizard has no vault and
/// never asks whether a privacy update is owed.
#[test]
fn the_console_list_names_the_two_privacy_commands_and_the_wizard_list_neither() {
    let path = Path::new(env!("CARGO_MANIFEST_DIR")).join("src").join("main.rs");
    let src = std::fs::read_to_string(&path).unwrap_or_else(|e| panic!("{}: {e}", path.display()));
    let lists = handler_lists(&src);
    let console = list_with(&lists, "commands::state");
    let wizard = list_with(&lists, "onboarding::finish_onboarding");
    for name in ["account::privacy_status", "account::accept_privacy"] {
        assert!(console.iter().any(|n| n == name), "the console list must name {name}");
        assert!(!wizard.iter().any(|n| n == name), "the wizard list must not name {name}");
    }
}

/// O3 F5c (test 43; plan §14.57.1, .9): the two step-8 wrappers and F2b3's `onboarding_mail_not_now` are
/// in the console's list and none in the wizard's. The wizard keeps the two path-taking commands.
#[test]
fn the_console_list_names_the_three_onboarding_commands_f5c_adds_and_the_wizard_list_none() {
    let path = Path::new(env!("CARGO_MANIFEST_DIR")).join("src").join("main.rs");
    let src = std::fs::read_to_string(&path).unwrap_or_else(|e| panic!("{}: {e}", path.display()));
    let lists = handler_lists(&src);
    let console = list_with(&lists, "commands::state");
    let wizard = list_with(&lists, "onboarding::finish_onboarding");
    const F5C: [&str; 3] = [
        "onboarding::onboarding_discover_coursework",
        "onboarding::onboarding_store_credentials",
        "onboarding_state::onboarding_mail_not_now",
    ];
    for name in F5C {
        assert_eq!(console.iter().filter(|n| *n == name).count(), 1, "the console list names {name} once");
        assert!(!wizard.iter().any(|n| n == name), "the wizard list must not name {name}");
    }
    for name in ["onboarding::discover_coursework", "onboarding::store_credentials"] {
        assert!(wizard.iter().any(|n| n == name), "the wizard list keeps {name}");
        assert!(!console.iter().any(|n| n == name), "the console list must not name the path-taking {name}");
    }
    assert_eq!(console.len(), 92, "the console list's count; recount it when a command is added");
}

/// Plan §14.63: the main page's `onboarding_handover` is on the console's list once and not on the wizard's,
/// and the wizard's count is 21 (F5b).
#[test]
fn the_console_list_names_onboarding_handover_once_and_the_wizard_list_does_not() {
    let path = Path::new(env!("CARGO_MANIFEST_DIR")).join("src").join("main.rs");
    let src = std::fs::read_to_string(&path).unwrap_or_else(|e| panic!("{}: {e}", path.display()));
    let lists = handler_lists(&src);
    let console = list_with(&lists, "commands::state");
    let wizard = list_with(&lists, "onboarding::finish_onboarding");
    assert_eq!(console.iter().filter(|n| *n == "onboarding_state::onboarding_handover").count(), 1);
    assert!(!wizard.iter().any(|n| n == "onboarding_state::onboarding_handover"));
    assert_eq!(wizard.len(), 21, "the wizard list's count; recount it when a command is added or retired");
}

#[test]
fn the_parser_reads_a_list_by_its_names() {
    let text = "x.invoke_handler(tauri::generate_handler![a::one, b::two,\n c::three])\n\
                y.invoke_handler(tauri::generate_handler![d::four])";
    let lists = handler_lists(text);
    assert_eq!(lists, vec![vec!["a::one", "b::two", "c::three"], vec!["d::four"]]);
}

/// Plan F5 (§14.60): the fourteen commands are on the console's list once each; the ten `onboarding_state` ones are not on
/// the wizard's.
#[test]
fn the_console_list_names_the_fourteen_f5_commands_once_and_the_wizard_list_none() {
    let path = Path::new(env!("CARGO_MANIFEST_DIR")).join("src").join("main.rs");
    let src = std::fs::read_to_string(&path).unwrap_or_else(|e| panic!("{}: {e}", path.display()));
    let lists = handler_lists(&src);
    let console = list_with(&lists, "commands::state");
    let wizard = list_with(&lists, "onboarding::finish_onboarding");
    const F5: [&str; 14] = [
        "onboarding::campus_search",
        "lms_link::open_lms_window",
        "lms_link::capture_courses",
        "lms_link::close_lms_window",
        "onboarding_state::onboarding_status",
        "onboarding_state::onboarding_answer",
        "onboarding_state::onboarding_skip",
        "onboarding_state::onboarding_retry",
        "onboarding_state::onboarding_finish",
        "onboarding_state::onboarding_quit",
        "onboarding_state::onboarding_sign_out",
        "onboarding_state::onboarding_attach_session",
        "onboarding_state::onboarding_entitlement",
        "onboarding_state::onboarding_checkout",
    ];
    for name in F5 {
        assert_eq!(console.iter().filter(|n| *n == name).count(), 1, "the console list names {name} once");
        // F5b took the campus and school-window names off the wizard's list; `capture_courses` stays on it.
        if name != "lms_link::capture_courses" {
            assert!(!wizard.iter().any(|n| n == name), "the wizard list must not name {name}");
        }
    }
    // Plan F5: never the two that store under the pending session (review r6, I-2).
    for name in ["lms_link::capture_calendar_link", "lms_link::paste_calendar_link"] {
        assert!(!console.iter().any(|n| n == name), "the console list must not name {name}");
    }
}

/// Plan F5b (plan 14.48 and 14.60): the three moved names are console-present and wizard-absent; the seven retired or
/// pending-target names are on neither list or wizard-absent; the two path-taking names stay on the wizard's.
#[test]
fn f5b_moves_three_retires_seven_and_keeps_the_path_taking_two_on_the_wizard() {
    let path = Path::new(env!("CARGO_MANIFEST_DIR")).join("src").join("main.rs");
    let src = std::fs::read_to_string(&path).unwrap_or_else(|e| panic!("{}: {e}", path.display()));
    let lists = handler_lists(&src);
    let console = list_with(&lists, "commands::state");
    let wizard = list_with(&lists, "onboarding::finish_onboarding");
    for name in ["onboarding::campus_search", "lms_link::open_lms_window", "lms_link::close_lms_window"] {
        assert_eq!(console.iter().filter(|n| *n == name).count(), 1, "the console list names {name} once");
        assert!(!wizard.iter().any(|n| n == name), "the wizard list must not name {name}");
    }
    for name in ["account::open_external", "onboarding::retarget_credentials", "onboarding::timezone_for_state"] {
        assert!(!console.iter().any(|n| n == name), "the console list must not name {name}");
        assert!(!wizard.iter().any(|n| n == name), "the wizard list must not name {name}");
    }
    for name in [
        "account::google_connect_url",
        "account::google_connected",
        "lms_link::paste_calendar_link",
        "lms_link::capture_calendar_link",
    ] {
        assert!(!wizard.iter().any(|n| n == name), "the wizard list must not name {name}");
    }
    for name in ["onboarding::store_credentials", "onboarding::discover_coursework"] {
        assert!(wizard.iter().any(|n| n == name), "the wizard list keeps {name}");
        assert!(!console.iter().any(|n| n == name), "the console list must not name {name}");
    }
}

/// S6r (onboarding plan S6t): the school sign-in row's two commands and P1's save command are in the
/// console's list and not the wizard's (the wizard has no profile folder and no `ConsoleState`).
#[test]
fn the_console_list_names_the_school_row_commands_and_p1s_save_command() {
    let path = Path::new(env!("CARGO_MANIFEST_DIR")).join("src").join("main.rs");
    let src = std::fs::read_to_string(&path).unwrap_or_else(|e| panic!("{}: {e}", path.display()));
    let lists = handler_lists(&src);
    let console = list_with(&lists, "commands::state");
    let wizard = list_with(&lists, "onboarding::finish_onboarding");
    for name in ["school_session::school_status", "school_session::school_forget", "school_password::school_password_save"] {
        assert_eq!(console.iter().filter(|n| *n == name).count(), 1, "the console list names {name} once");
        assert!(!wizard.iter().any(|n| n == name), "the wizard list must not name {name}");
    }
}
