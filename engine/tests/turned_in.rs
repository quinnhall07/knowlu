//! O1 T2a: cross-module pins for "turned-in work is not a task" (plan
//! `docs/plans/2026-10-23-o1-turned-in-work-plan.md`, tests 22 and 23). Every name is invented, every
//! vault lives under the temp directory.
//!
//! Both tests pin behaviour that must STAY true once `imported-done` exists, so they hold from the
//! first commit; the red tests for the lane live beside the code they drive.

use std::path::PathBuf;

use jiff::civil::{Date, DateTime};
use knowlu_engine::approvals::process_approvals;
use knowlu_engine::journal::Journal;
use knowlu_engine::write::WriteContext;

const TODAY: Date = Date::constant(2026, 8, 20);

fn vault(tag: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!("knowlu-turned-in-{}-{tag}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(dir.join("archive")).unwrap();
    std::fs::create_dir_all(dir.join("approvals")).unwrap();
    std::fs::create_dir_all(dir.join("tasks")).unwrap();
    dir
}

/// Test 22 (plan T2a): a mail card that only moves `due`, aimed at an `imported-done` note in
/// `archive/`, is refused like a card aimed at any archived note. The un-archive arm
/// (`approvals::imported_past_target`) matches `imported-past` only, so an item the source said was
/// turned in never comes back because a mail moved its date.
#[test]
fn a_mail_due_card_cannot_unarchive_an_imported_done_note() {
    let v = vault("mail-card");
    let note = "---\nid: task_00000000e1\ntitle: Lab 5\ncourse: cs-101\ndue: 2026-08-01\n\
                status: archived\narchived_reason: imported-done\ncreated_by: zybooks\n---\n\nBody.\n";
    std::fs::write(v.join("archive/lab-5.md"), note).unwrap();
    let card = "---\nid: appr_00000000e2\ntype: approval\nkind: amend\ntarget: archive/lab-5.md\n\
                title: Lab 5 dated\nstatus: approved\nsource_uid: \"mail:5a5a:2\"\nproposed_at: 2026-08-18\n\
                first_proposed_at: 2026-08-18\nexpires: null\nsnooze_until: null\ncreated_by: mail\n\
                changes:\n  due:\n    from: 2026-08-01\n    to: 2026-09-01\n---\n\n**Why proposed:** dated.\n";
    std::fs::write(v.join("approvals/amend-lab-5.md"), card).unwrap();

    let card_text = std::fs::read_to_string(v.join("approvals/amend-lab-5.md")).unwrap();
    let (meta, _) = knowlu_engine::models::split_frontmatter(&card_text).unwrap();
    let verdict = knowlu_engine::approvals::validate_amendment(&v, &meta);
    assert!(
        verdict.as_ref().is_err_and(|why| why.starts_with("target outside")),
        "an imported-done note must be out of reach like any archived note: {verdict:?}"
    );

    let ctx = WriteContext::new("agent:approvals", "local-runner");
    let now: DateTime = DateTime::constant(2026, 8, 20, 12, 0, 0, 0);
    let result = process_approvals(&v, TODAY, now, &ctx, &mut Journal::new(&v));
    assert!(result.executed.is_empty(), "{:?}", result.executed);
    assert!(v.join("archive/lab-5.md").is_file(), "the note left archive/");
    assert!(!v.join("tasks/lab-5.md").exists(), "the note reached tasks/");
    assert_eq!(std::fs::read_to_string(v.join("archive/lab-5.md")).unwrap(), note);
    let _ = std::fs::remove_dir_all(&v);
}

/// Test 23 (plan T2a): the one spelling of the reason, shared. Coursework's template half is in
/// `coursework.rs`'s tests (its template is private to the crate's tests); ingest's is T2b's.
#[test]
fn imported_done_is_one_spelling_beside_imported_past() {
    assert_eq!(knowlu_engine::ingest::IMPORTED_DONE, "imported-done");
    assert_ne!(knowlu_engine::ingest::IMPORTED_DONE, knowlu_engine::ingest::IMPORTED_PAST);
}
