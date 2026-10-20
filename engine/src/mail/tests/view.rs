//! The vault view T6.3 builds for the reconciler (plan T6.3, T5.6's addendum).

use super::*;
use crate::mail::view;

fn lab_text(extra: &str) -> String {
    format!("---\ntitle: Lab 5\ncourse: CS 101\ndue: 2026-10-05\nstatus: active\n{extra}---\n\nBody.\n")
}

/// The one-pass `human_set` the view takes is `Journal::human_set`'s answer for every note and field: a
/// note a student made by hand (its `create` carries the fields), an agent note a student then set, and
/// an agent note only an agent set.
#[test]
fn the_views_human_set_is_journal_human_sets_answer_for_every_field() {
    let vault = scratch("humanview");
    let mut journal = Journal::new(&vault);
    let (human, agent) = (WriteContext::new(crate::journal::read_human_actor(&vault).unwrap(), "dashboard"), WriteContext::new(MAIL_ACTOR, VIA));
    write::create(&vault, "tasks/a.md", &lab_text(""), &human, &mut journal, None).unwrap();
    write::create(&vault, "tasks/b.md", &lab_text(""), &agent, &mut journal, None).unwrap();
    write::create(&vault, "tasks/c.md", &lab_text(""), &agent, &mut journal, None).unwrap();
    let set = |rel: &str, ctx: &WriteContext, journal: &mut Journal| {
        let literals = [("due".to_string(), "2026-10-09".to_string())];
        write::write_literals(&vault, rel, &literals, ctx, journal, &WriteOpts::default()).unwrap();
    };
    set("tasks/b.md", &human, &mut journal);
    set("tasks/c.md", &agent, &mut journal);

    let built = view::build(&vault, &mut journal);
    assert_eq!(built.notes.len(), 3);
    let mut humans = 0;
    for note in &built.notes {
        for field in ["title", "course", "due", "status", "importance", "mail_asof"] {
            let asked = journal.human_set(&note.id, field).is_some();
            assert_eq!(note.human_set.contains(field), asked, "{} {field}", note.path);
            humans += usize::from(asked);
        }
    }
    assert!(humans > 4, "the hand-made note and the student's set both count: {humans}");
    let by_path = |path: &str| built.notes.iter().find(|n| n.path == path).unwrap();
    assert!(by_path("tasks/a.md").human_set.contains("due"), "a note the student made by hand is theirs");
    assert!(by_path("tasks/b.md").human_set.contains("due"));
    assert!(!by_path("tasks/c.md").human_set.contains("due"), "an agent's own set is not a human's");
}

#[test]
fn a_task_card_is_viewed_by_its_payload_and_a_note_by_its_frontmatter() {
    let vault = scratch("cardview");
    let ctx = WriteContext::new(MAIL_ACTOR, VIA);
    let mut journal = Journal::new(&vault);
    let card = |payload: &str| {
        format!(
            "---\ntype: approval\nkind: task\ntitle: Card title\nstatus: snoozed\ncreated_by: mail\nsource_uid: mail:aa:0\n---\n\nWhy.\n\n```task\n{payload}```\n"
        )
    };
    let payload = "---\ntitle: Lab 5\ncourse: CS 101\ndue: 2026-10-12T23:59\nstatus: active\n---\n\nFrom email.\n";
    write::create(&vault, "approvals/task-lab-5.md", &card(payload), &ctx, &mut journal, None).unwrap();
    write::create(&vault, "approvals/task-bare.md", &card("no frontmatter here\n"), &ctx, &mut journal, None).unwrap();
    let archived = lab_text("archived_reason: imported-past\nmail_asof: {due: '2026-09-20T10:00:00Z'}\n").replace("status: active", "status: archived");
    write::create(&vault, "archive/lab-4.md", &archived, &ctx, &mut journal, None).unwrap();

    let built = view::build(&vault, &mut journal);
    let find = |path: &str| built.notes.iter().find(|n| n.path == path).unwrap_or_else(|| panic!("{path} is not in the view"));
    let card = find("approvals/task-lab-5.md");
    assert_eq!(card.kind.as_deref(), Some("task"));
    assert_eq!((card.title.as_str(), card.course.as_deref()), ("Lab 5", Some("CS 101")));
    assert_eq!(card.due, json!("2026-10-12T23:59"), "the due is the payload's, which the card has none of");
    assert_eq!((card.status.as_deref(), card.created_by.as_deref(), card.source_uid.as_deref()), (Some("snoozed"), Some("mail"), Some("mail:aa:0")));
    let bare = find("approvals/task-bare.md");
    assert_eq!((bare.course.clone(), bare.due.clone()), (None, serde_json::Value::Null), "no payload frontmatter: it matches no claim");
    assert_eq!(bare.title, "Card title");
    let note = find("archive/lab-4.md");
    assert_eq!((note.kind.clone(), note.archived_reason.as_deref()), (None, Some("imported-past")));
    assert_eq!(note.mail_asof.get("due"), Some(&ts("2026-09-20T10:00:00Z")));
    assert_eq!(note.folder, "archive");
}
