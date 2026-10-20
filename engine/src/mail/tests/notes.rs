//! The note, notice and receipt writers: spec tests 26 and 28c's writer half, §5.2, D13, PQ4.

use super::*;

fn plan_create(m: &Message, vault_day: i64) -> Action {
    let plan = mailreconcile::plan(&mailreconcile::VaultView::default(), std::slice::from_ref(m), today(), vault_day);
    let [action] = &plan[..] else { panic!("{plan:?}") };
    action.clone()
}

#[test]
fn a_mail_note_is_written_needs_enrichment_false_with_the_claims_effort_and_importance() {
    let vault = scratch("note");
    let m = message("mail:bb", Phase::Backfill, vec![new_item("Lab 6", Some("2026-10-20"))]);
    let action = plan_create(&m, 9);
    assert!(matches!(action, Action::Create { .. }), "{action:?}");
    assert_eq!(apply(&vault, &action, &m, 0, held()), Wrote::Note("tasks/cs-101-lab-6.md".to_string()));

    let get = |key: &str| field(&vault, "tasks/cs-101-lab-6.md", key);
    assert_eq!(get("needs_enrichment"), Some(json!(false)));
    assert_eq!((get("effort_hours"), get("importance")), (Some(json!(2.5)), Some(json!(4))));
    assert_eq!((get("created_by"), get("source_uid")), (Some(json!("mail")), Some(json!("mail:bb:0"))));
    assert_eq!((get("judgment_id"), get("judgment_kind")), (Some(json!("0b1c2d3e-0000-4000-8000-000000000001")), Some(json!("mail_claim"))));
    assert_eq!(get("status"), Some(json!("active")));
    assert_eq!(get("due"), Some(json!("2026-10-20")));
    assert_eq!(get("mail_asof"), Some(json!({"due": "2026-10-01T12:00:00Z"})));
    assert_eq!(get("effort_source"), Some(json!("inferred")));
    assert!(read(&vault, "tasks/cs-101-lab-6.md").contains("From email. 2026-10-01 \u{b7} Course Staff <staff@example.edu>"));
    // A second note with the same title takes the next free name, never an overwrite.
    assert_eq!(apply(&vault, &action, &m, 0, held()), Wrote::Note("tasks/cs-101-lab-6-2.md".to_string()));
}

#[test]
fn an_undated_note_has_a_null_due_and_no_mail_asof_and_a_past_one_is_archived_imported_past() {
    let vault = scratch("undated");
    let m = message("mail:bb", Phase::Backfill, vec![new_item("Lab 6", None), new_item("Lab 1", Some("2026-09-01"))]);
    let plan = mailreconcile::plan(&mailreconcile::VaultView::default(), std::slice::from_ref(&m), today(), 9);
    let wrote: Vec<Wrote> = plan.iter().enumerate().map(|(i, action)| apply(&vault, action, &m, i, held())).collect();
    assert_eq!(wrote, [Wrote::Note("tasks/cs-101-lab-6.md".to_string()), Wrote::Note("archive/cs-101-lab-1.md".to_string())]);
    assert_eq!(field(&vault, "tasks/cs-101-lab-6.md", "due"), Some(json!(null)));
    assert_eq!(field(&vault, "tasks/cs-101-lab-6.md", "mail_asof"), None);
    let (status, reason) = (field(&vault, "archive/cs-101-lab-1.md", "status"), field(&vault, "archive/cs-101-lab-1.md", "archived_reason"));
    assert_eq!((status, reason), (Some(json!("archived")), Some(json!("imported-past"))));
}

#[test]
fn a_later_notice_closes_the_earlier_by_close_key() {
    let vault = scratch("notice");
    let room = Claim { topic: Some(crate::mailreconcile::Topic::Room), ..claim(ClaimKind::CourseInfo) };
    let m = message("mail:bb", Phase::Live, vec![room]);
    let mut first = plan_create(&m, 5);
    let Action::Notice { title, close_key, .. } = &mut first else { panic!("{first:?}") };
    assert_eq!((title.as_str(), close_key.as_deref()), ("CS 101: room", Some("mail:CS 101:room")));

    apply(&vault, &first, &m, 0, held());
    let Action::Notice { title, .. } = &mut first else { unreachable!() };
    *title = "CS 101: room (moved)".to_string();
    apply(&vault, &first, &m, 0, held());

    let open = crate::info::list_info(&vault);
    assert_eq!(open.len(), 1, "the earlier notice is closed");
    let open = &open[0];
    let text = |key: &str| crate::yaml::get(open, key).and_then(crate::yaml::text);
    assert_eq!(text("title").as_deref(), Some("CS 101: room (moved)"));
    assert_eq!((text("kind").as_deref(), text("opened_by").as_deref()), (Some("notice"), Some("agent:knowlu.mail")));
    assert_eq!(text("close_key").as_deref(), Some("mail:CS 101:room"));
    assert!(read(&vault, text("path").as_deref().unwrap()).contains("2026-10-01 \u{b7} Course Staff"));
}

#[test]
fn a_heads_up_opens_through_the_same_writer() {
    let vault = scratch("headsup");
    let mut journal = Journal::new(&vault);
    let mut writer = Writer::new(&vault, VIA, None, &mut journal, today(), held());
    let done = writer.notice("heads-up", Some("mail-source:addr_1"), "Email forwarding is off", "Turn it back on in Settings").unwrap();
    assert!(matches!(done, Wrote::Notice(path) if path.starts_with("info/info_")));
    assert_eq!(crate::info::list_info(&vault).len(), 1);
    assert!(writer.notice("sticker", None, "x", "y").is_err(), "an unknown kind is refused, never written");
}

#[test]
fn a_receipt_files_the_mark_done_card_once() {
    let vault = scratch("receipt");
    mail_note(&vault, "tasks/lab-5.md", "2026-10-03");
    let receipt = Claim { kind: ClaimKind::Receipt, source: ClaimSource::Template, target: Some("Lab 5".to_string()), ..claim(ClaimKind::Receipt) };
    let m = message("mail:bb", Phase::Live, vec![receipt]);
    let action = Action::ProposeDone { claim: at("mail:bb", 0), target: Place::Path("tasks/lab-5.md".to_string()) };
    let first = apply(&vault, &action, &m, 0, held());
    assert_eq!(first, Wrote::Receipt("approvals/amend-lab-5-done.md".to_string()));
    let card = "approvals/amend-lab-5-done.md";
    assert_eq!(field(&vault, card, "created_by"), Some(json!(MAIL_ACTOR)));
    assert_eq!(field(&vault, card, "judgment_kind"), Some(json!("mail_claim")));
    assert_eq!(field(&vault, card, "changes"), Some(json!({"status": {"from": "active", "to": "done"}})));
    let again = apply(&vault, &action, &m, 0, held());
    assert_eq!(again, Wrote::Nothing(Why::ReceiptDeclined("already proposed".to_string())));
}
