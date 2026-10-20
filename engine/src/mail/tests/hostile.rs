//! Untrusted text (spec §5.2, plan §10 R11): a value T5 lets through lands as one literal line, and
//! one T5 skips writes no byte.

use super::*;
use crate::models::split_frontmatter;

const HOSTILE: &str = "Quiz: \"two\" <img src=x onerror=alert(1)> - {why} #tag";

#[test]
fn a_colon_a_quote_and_an_img_onerror_in_evidence_survive_as_literals() {
    let mut claim = new_item(HOSTILE, Some("2026-10-20"));
    claim.why = HOSTILE.to_string();
    claim.evidence = HOSTILE.to_string();
    claim.course = Some(format!("CS: {HOSTILE}"));
    let mut m = message("mail:bb", Phase::Backfill, vec![claim]);
    m.sender.name = Some(HOSTILE.to_string());
    let sender = format!("{HOSTILE} <staff@example.edu>");

    // Every writer in turn: a direct note, a task card, a notice and an amend card.
    let vault = scratch("literals");
    let backfill = mailreconcile::plan(&mailreconcile::VaultView::default(), std::slice::from_ref(&m), today(), 9);
    let [create @ Action::Create { .. }] = &backfill[..] else { panic!("{backfill:?}") };
    let Wrote::Note(rel) = apply(&vault, create, &m, 0, held()) else { panic!() };
    let note = |key: &str| field(&vault, &rel, key);
    assert_eq!((note("title"), note("importance_reason")), (Some(json!(HOSTILE)), Some(json!(HOSTILE))));
    assert_eq!(note("course"), Some(json!(format!("CS: {HOSTILE}"))));
    assert_eq!(note("source_uid"), Some(json!("mail:bb:0")));

    m.phase = Phase::Live;
    let live = mailreconcile::plan(&mailreconcile::VaultView::default(), std::slice::from_ref(&m), today(), 9);
    let [card @ Action::TaskCard { .. }] = &live[..] else { panic!("{live:?}") };
    let Wrote::Card(rel) = apply(&vault, card, &m, 0, held()) else { panic!() };
    let card = |key: &str| field(&vault, &rel, key);
    assert_eq!((card("title"), card("from"), card("evidence")), (Some(json!(HOSTILE)), Some(json!(sender)), Some(json!(HOSTILE))));
    let keys: Vec<String> = meta(&vault, &rel).keys().filter_map(crate::yaml::text).collect();
    assert!(keys.iter().all(|key| !key.contains("onerror")), "{keys:?}");

    mail_note(&vault, "tasks/lab-5.md", "2026-10-03");
    let amend = due_card(at("mail:bb", 0), "tasks/lab-5.md", json!("2026-10-03"), "2026-10-08", pending());
    let Wrote::Card(rel) = apply(&vault, &amend, &m, 0, held()) else { panic!() };
    assert_eq!(field(&vault, &rel, "evidence"), Some(json!(HOSTILE)));
    assert_eq!(field(&vault, &rel, "from"), Some(json!(sender)));
    assert!(split_frontmatter(&read(&vault, &rel)).is_ok());

    let notice = Action::Notice {
        claim: at("mail:bb", 0),
        kind: "notice",
        close_key: Some(format!("mail:CS: {HOSTILE}:room")),
        title: format!("CS: {HOSTILE}"),
        body_line: mailreconcile::body_line(&m, &m.claims[0]),
        about: crate::mailreconcile::NoticeAbout::CourseInfo,
    };
    let Wrote::Notice(rel) = apply(&vault, &notice, &m, 0, held()) else { panic!() };
    assert_eq!(field(&vault, &rel, "title"), Some(json!(format!("CS: {HOSTILE}"))));
}

#[test]
fn a_line_break_in_any_mail_field_writes_no_byte() {
    for breaker in ["\r", "\u{85}", "\u{2028}", "\n"] {
        for which in ["title", "name", "address", "why", "evidence", "course"] {
            let mut claim = new_item("Lab 6", Some("2026-10-20"));
            let mut sender = Sender { name: Some("Course Staff".to_string()), address: "staff@example.edu".to_string() };
            let hostile = format!("a{breaker}- bb-uid-123 \u{b7} x");
            match which {
                "title" => claim.item.as_mut().unwrap().title = hostile,
                "name" => sender.name = Some(hostile),
                "address" => sender.address = hostile,
                "why" => claim.why = hostile,
                "evidence" => claim.evidence = hostile,
                _ => claim.course = Some(hostile),
            }
            let mut m = message("mail:bb", Phase::Backfill, vec![claim]);
            m.sender = sender;
            let vault = scratch("breaks");
            for phase in [Phase::Backfill, Phase::Live] {
                m.phase = phase;
                let plan = mailreconcile::plan(&mailreconcile::VaultView::default(), std::slice::from_ref(&m), today(), 9);
                for action in &plan {
                    assert!(matches!(action, Action::Skip { .. }), "{which} {breaker:?}: {action:?}");
                    assert!(matches!(apply(&vault, action, &m, 0, held()), Wrote::Nothing(_)));
                }
            }
            let written: Vec<String> = ["tasks", "approvals", "archive", "info"].iter().flat_map(|folder| files(&vault, folder)).collect();
            assert!(written.is_empty(), "{which} {breaker:?}: {written:?}");
            assert!(!vault.join("state/journal").exists() || files(&vault, "state/journal").is_empty());
        }
    }
}

#[test]
fn a_writer_refuses_a_line_break_a_plan_let_through_before_it_writes() {
    // The writers' own belt, should a caller hand one a claim T5 would have skipped.
    let vault = scratch("belt");
    let mut claim = new_item("Lab 6", Some("2026-10-20"));
    claim.evidence = "one\u{2028}two".to_string();
    let m = message("mail:bb", Phase::Live, vec![claim]);
    let amend = due_card(at("mail:bb", 0), "tasks/lab-5.md", json!("2026-10-03"), "2026-10-08", pending());
    mail_note(&vault, "tasks/lab-5.md", "2026-10-03");
    let mut journal = Journal::new(&vault);
    let mut writer = Writer::new(&vault, VIA, None, &mut journal, today(), held());
    let src = Source { message: &m, claim: &m.claims[0], judgment_id: None };
    assert_eq!(writer.apply(&amend, &src), Err(WriteError::LineBreak("evidence".to_string())));
    assert!(files(&vault, "approvals").is_empty());
}
