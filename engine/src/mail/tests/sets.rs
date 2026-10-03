//! The direct-set writer: spec test 27, §5.2, plan §10 R1, R10 and R13.

use super::*;

#[test]
fn mail_asof_is_a_single_line_flow_mapping_and_no_other_byte_moves() {
    let vault = scratch("asof");
    mail_note(&vault, "tasks/lab-5.md", "2026-10-03");
    let before = read(&vault, "tasks/lab-5.md");
    let m = message("mail:bb", Phase::Backfill, vec![change("Lab 5", "2026-10-08")]);

    let done = apply(&vault, &set_due(at("mail:bb", 0), "tasks/lab-5.md", "2026-10-08", "2026-10-01T12:00:00Z"), &m, 0, held());
    assert_eq!(done, Wrote::Set("tasks/lab-5.md".to_string()));

    let line = "mail_asof: {due: '2026-10-01T12:00:00Z'}";
    let want = before.replace("due: 2026-10-03\n", "due: 2026-10-08\n").replace("\n---\n\n", &format!("\n{line}\n---\n\n"))
        + "\n> 2026-10-01 \u{b7} Course Staff <staff@example.edu>: The instructor moved the due date \u{2014} \"Lab 5 is now due Monday\"\n";
    let after = read(&vault, "tasks/lab-5.md");
    assert_eq!(after, want);
    assert_eq!(after.lines().filter(|l| l.starts_with("mail_asof:")).count(), 1);

    // A later change merges into the same line: the earlier field's stamp stays, none moves.
    let again = set_of(at("mail:cc", 0), Place::Path("tasks/lab-5.md".to_string()), "importance", json!(5), "2026-10-02T09:00:00Z");
    apply(&vault, &again, &m, 0, held());
    let line = read(&vault, "tasks/lab-5.md").lines().find(|l| l.starts_with("mail_asof:")).unwrap().to_string();
    assert_eq!(line, "mail_asof: {due: '2026-10-01T12:00:00Z', importance: '2026-10-02T09:00:00Z'}");
}

#[test]
fn a_direct_change_sets_the_field_and_mail_asof_and_appends_one_body_line() {
    let vault = scratch("direct");
    let id = mail_note(&vault, "tasks/lab-5.md", "2026-10-03");
    let m = message("mail:bb", Phase::Backfill, vec![change("Lab 5", "2026-10-08")]);
    apply(&vault, &set_due(at("mail:bb", 0), "tasks/lab-5.md", "2026-10-08", "2026-10-01T12:00:00Z"), &m, 0, held());

    let records = Journal::new(&vault).records_for(&id, None);
    let sets: Vec<(String, String, String)> = records
        .iter()
        .filter(|r| r["actor"] == MAIL_ACTOR && r["op"] != "create")
        .map(|r| (r["op"].as_str().unwrap().to_string(), r["field"].as_str().unwrap_or("").to_string(), r["via"].as_str().unwrap().to_string()))
        .collect();
    let want = [("set", "due"), ("set", "mail_asof"), ("append_body", "")].map(|(op, f)| (op.to_string(), f.to_string(), VIA.to_string()));
    assert_eq!(sets, want);
    assert_eq!(field(&vault, "tasks/lab-5.md", "due"), Some(json!("2026-10-08")));
    assert_eq!(field(&vault, "tasks/lab-5.md", "mail_asof"), Some(json!({"due": "2026-10-01T12:00:00Z"})));
    assert_eq!(read(&vault, "tasks/lab-5.md").matches("\n> ").count(), 1);
    // Applied twice, nothing more is written.
    apply(&vault, &set_due(at("mail:bb", 0), "tasks/lab-5.md", "2026-10-08", "2026-10-01T12:00:00Z"), &m, 0, held());
    assert_eq!(Journal::new(&vault).records_for(&id, None).len(), records.len());
}

#[test]
fn a_human_set_landing_between_plan_and_apply_turns_the_direct_set_into_a_card() {
    let vault = scratch("race");
    let id = mail_note(&vault, "tasks/lab-5.md", "2026-10-03");
    let m = message("mail:bb", Phase::Backfill, vec![change("Lab 5", "2026-10-08")]);

    // The plan is made from a view of the note as the mail left it, as T6.3 builds it, so it is a
    // direct Set. The writer's journal has read the ledger by then, as building the view does.
    let note = NoteView {
        id, path: "tasks/lab-5.md".to_string(), folder: "tasks".to_string(), kind: None, course: Some("CS 101".to_string()),
        title: "Lab 5".to_string(), due: json!("2026-10-03"), status: Some("active".to_string()), archived_reason: None,
        source_uid: Some("mail:aa:0".to_string()), created_by: Some("mail".to_string()),
        mail_asof: Default::default(), human_set: Default::default(),
    };
    let actions = mailreconcile::plan(&VaultView { notes: vec![note] }, std::slice::from_ref(&m), today(), 9);
    assert!(matches!(actions.as_slice(), [Action::Set { field, .. }] if field == "due"), "{actions:?}");
    let mut journal = Journal::new(&vault);
    journal.read(None, None);
    let mut writer = Writer::new(&vault, VIA, None, &mut journal, today(), held());

    // The student's set lands after that read, as `resolve_mail`'s network call leaves room for.
    let human = WriteContext::new(crate::journal::read_human_actor(&vault).unwrap(), "dashboard");
    let student = [("due".to_string(), serde_yaml_ng::Value::String("2026-10-05".to_string()))];
    write::write(&vault, "tasks/lab-5.md", &student, &human, &mut Journal::new(&vault), &WriteOpts::default()).unwrap();

    let src = Source { message: &m, claim: &m.claims[0], judgment_id: None };
    let done = writer.apply(&actions[0], &src).unwrap();
    assert_eq!(done, Wrote::Card("approvals/amend-lab-5-due.md".to_string()));
    assert_eq!(field(&vault, "tasks/lab-5.md", "due"), Some(json!("2026-10-05")), "the student's value stands");
    assert_eq!(field(&vault, "tasks/lab-5.md", "mail_asof"), None);
    let card = "approvals/amend-lab-5-due.md";
    assert_eq!(field(&vault, card, "changes"), Some(json!({"due": {"from": "2026-10-05", "to": "2026-10-08"}})));
    // The card carries the day's hold, so a first-day race is not shown on day 1 (R16).
    assert_eq!(field(&vault, card, "status"), Some(json!("snoozed")));
}

#[test]
fn mail_asof_reapplied_from_its_journal_record_is_byte_identical() {
    let vault = scratch("replay");
    mail_note(&vault, "tasks/lab-5.md", "2026-10-03");
    let m = message("mail:bb", Phase::Backfill, vec![change("Lab 5", "2026-10-08")]);
    apply(&vault, &set_due(at("mail:bb", 0), "tasks/lab-5.md", "2026-10-08", "2026-10-01T12:00:00Z"), &m, 0, held());
    let written = read(&vault, "tasks/lab-5.md");

    // A crash between the journal record and the note write: the line is missing, the record is not.
    let torn: String = written.split_inclusive('\n').filter(|l| !l.starts_with("mail_asof:")).collect();
    std::fs::write(vault.join("tasks/lab-5.md"), &torn).unwrap();
    let ctx = WriteContext::new(MAIL_ACTOR, VIA);
    let log = crate::passes::verify_tail(&vault, &mut Journal::new(&vault), &ctx, crate::passes::VERIFY_TAIL_LIMIT);
    assert!(log.iter().any(|l| l.contains("re-applied mail_asof")), "{log:?}");
    assert_eq!(read(&vault, "tasks/lab-5.md"), written);
}

#[test]
fn a_mail_body_line_under_an_open_editor_makes_set_body_refuse_as_a_conflict() {
    let vault = scratch("editor");
    mail_note(&vault, "tasks/lab-5.md", "2026-10-03");
    let opened = read(&vault, "tasks/lab-5.md").split_once("\n---\n").unwrap().1.to_string();
    let m = message("mail:bb", Phase::Backfill, vec![change("Lab 5", "2026-10-08")]);
    apply(&vault, &set_due(at("mail:bb", 0), "tasks/lab-5.md", "2026-10-08", "2026-10-01T12:00:00Z"), &m, 0, held());

    let human = WriteContext::new(crate::journal::read_human_actor(&vault).unwrap(), "dashboard");
    let saved = write::set_body(&vault, "tasks/lab-5.md", &opened, "My own notes\n", &human, &mut Journal::new(&vault));
    assert_eq!(saved, Err(WriteError::Conflict("tasks/lab-5.md".to_string())));
    assert!(read(&vault, "tasks/lab-5.md").contains("\n> 2026-10-01"), "the line is kept, never dropped");
}

#[test]
fn an_action_naming_a_planned_note_reaches_it_and_one_whose_create_wrote_nothing_writes_nothing() {
    let vault = scratch("planned");
    let m = message("mail:bb", Phase::Backfill, vec![new_item("Lab 6", Some("2026-10-20")), change("Lab 6", "2026-10-22")]);
    let note = NewNote {
        title: "Lab 6".to_string(), course: Some("CS 101".to_string()), due: Some("2026-10-20".to_string()),
        item_kind: ItemKind::Task, effort_hours: 2.5, importance: 4, imported_past: false, mail_asof: Some(ts("2026-10-01T12:00:00Z")),
    };
    let mut journal = Journal::new(&vault);
    let mut writer = Writer::new(&vault, VIA, None, &mut journal, today(), held());
    let src = |i: usize| Source { message: &m, claim: &m.claims[i], judgment_id: None };
    let made = writer.apply(&Action::Create { claim: at("mail:bb", 0), note }, &src(0)).unwrap();
    assert_eq!(made, Wrote::Note("tasks/cs-101-lab-6.md".to_string()));

    let set_at = |place: Place| set_of(at("mail:bb", 1), place, "due", json!("2026-10-22"), "2026-10-01T12:00:00Z");
    let moved = set_at(Place::Planned(at("mail:bb", 0)));
    assert_eq!(writer.apply(&moved, &src(1)).unwrap(), Wrote::Set("tasks/cs-101-lab-6.md".to_string()));
    let orphan = set_at(Place::Planned(at("mail:zz", 0)));
    assert_eq!(writer.apply(&orphan, &src(1)).unwrap(), Wrote::Nothing(Why::NoPlannedNote));
    assert_eq!(files(&vault, "tasks"), ["cs-101-lab-6.md"]);
}
