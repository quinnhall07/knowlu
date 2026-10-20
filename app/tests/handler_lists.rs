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

#[test]
fn the_parser_reads_a_list_by_its_names() {
    let text = "x.invoke_handler(tauri::generate_handler![a::one, b::two,\n c::three])\n\
                y.invoke_handler(tauri::generate_handler![d::four])";
    let lists = handler_lists(text);
    assert_eq!(lists, vec![vec!["a::one", "b::two", "c::three"], vec!["d::four"]]);
}
