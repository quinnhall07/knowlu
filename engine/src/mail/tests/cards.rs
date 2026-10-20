//! The card writer: spec tests 21 and 22d's writer half, D11 (3), D21, F3 (T8c), plan §9.5, R14, R16.

use std::collections::{BTreeMap, BTreeSet};

use super::*;
use crate::mailreconcile::{NoteView, VaultView};
use crate::models::split_frontmatter;

fn view_of(rel: &str) -> VaultView {
    let note = NoteView {
        id: "task_aaaa".to_string(),
        path: rel.to_string(),
        folder: "tasks".to_string(),
        kind: None,
        course: Some("CS 101".to_string()),
        title: "Lab 5".to_string(),
        due: json!("2026-10-03"),
        status: Some("active".to_string()),
        archived_reason: None,
        source_uid: Some("mail:aa:0".to_string()),
        created_by: Some("mail".to_string()),
        mail_asof: BTreeMap::new(),
        human_set: BTreeSet::new(),
    };
    VaultView { notes: vec![note] }
}

/// Plan a live change on `vault_day`, then write what the plan says, as `mail.rs` will.
fn live_change(vault: &Path, uid: &str, to: &str, vault_day: i64) -> Wrote {
    let m = message(uid, Phase::Live, vec![change("Lab 5", to)]);
    let plan = mailreconcile::plan(&view_of("tasks/lab-5.md"), std::slice::from_ref(&m), today(), vault_day);
    let [action] = &plan[..] else { panic!("{plan:?}") };
    apply(vault, action, &m, 0, held())
}

fn validated(vault: &Path, rel: &str) -> Result<std::path::PathBuf, String> {
    let (meta, _) = split_frontmatter(&read(vault, rel)).unwrap();
    crate::approvals::validate_amendment(vault, &meta)
}

#[test]
fn a_second_claim_while_a_mail_card_is_pending_files_no_second_card() {
    for (vault_day, status) in [(2, "pending"), (1, "snoozed")] {
        let vault = scratch(&format!("open{vault_day}"));
        mail_note(&vault, "tasks/lab-5.md", "2026-10-03");
        let first = live_change(&vault, "mail:bb", "2026-10-08", vault_day);
        assert_eq!(first, Wrote::Card("approvals/amend-lab-5-due.md".to_string()), "day {vault_day}");
        assert_eq!(field(&vault, "approvals/amend-lab-5-due.md", "status"), Some(json!(status)));
        // Day 1's first card is written snoozed (R16) and still blocks the second (§9.5).
        let second = live_change(&vault, "mail:cc", "2026-10-09", vault_day);
        assert_eq!(second, Wrote::Nothing(Why::OpenCard), "day {vault_day}");
        assert_eq!(files(&vault, "approvals"), ["amend-lab-5-due.md"]);
    }
}

#[test]
fn a_snoozed_card_for_the_field_files_no_second_card() {
    let vault = scratch("snoozed");
    mail_note(&vault, "tasks/lab-5.md", "2026-10-03");
    // Whoever filed it: `defer_over_budget` snoozed this one, or the student did.
    let hand = "---\ntype: approval\nkind: amend\ntitle: Moved\nstatus: snoozed\ntarget: tasks/lab-5.md\n\
                proposed_at: 2026-10-02\nfirst_proposed_at: 2026-10-01\nexpires: null\nsnooze_until: 2026-10-02\n\
                created_by: blackboard\nchanges:\n  due:\n    from: 2026-10-03\n    to: 2026-10-07\n---\n\nbody\n";
    let ctx = WriteContext::new("agent:knowlu.ingest", VIA);
    write::create(&vault, "approvals/amend-lab-5-due.md", hand, &ctx, &mut Journal::new(&vault), None).unwrap();
    assert_eq!(live_change(&vault, "mail:bb", "2026-10-08", 5), Wrote::Nothing(Why::OpenCard));
    assert_eq!(files(&vault, "approvals"), ["amend-lab-5-due.md"]);
}

#[test]
fn a_first_day_card_is_written_snoozed_to_day_two() {
    let vault = scratch("day1");
    mail_note(&vault, "tasks/lab-5.md", "2026-10-03");
    live_change(&vault, "mail:bb", "2026-10-08", 1);
    let card = "approvals/amend-lab-5-due.md";
    let got = ["status", "proposed_at", "first_proposed_at", "snooze_until"].map(|key| field(&vault, card, key));
    assert_eq!(got, [Some(json!("snoozed")), Some(json!("2026-10-02")), Some(json!("2026-10-01")), Some(json!("2026-10-02"))]);
    // A card on any later day is pending with no hold.
    let vault = scratch("day2");
    mail_note(&vault, "tasks/lab-5.md", "2026-10-03");
    live_change(&vault, "mail:bb", "2026-10-08", 2);
    let got = ["status", "proposed_at", "first_proposed_at", "snooze_until"].map(|key| field(&vault, card, key));
    assert_eq!(got, [Some(json!("pending")), Some(json!("2026-10-01")), Some(json!("2026-10-01")), Some(json!(null))]);
}

#[test]
fn a_mail_amend_card_passes_validate_amendment_and_carries_from_mail_date_and_evidence() {
    let vault = scratch("amend");
    mail_note(&vault, "tasks/lab-5.md", "2026-10-03");
    live_change(&vault, "mail:bb", "2026-10-08", 2);
    let card = "approvals/amend-lab-5-due.md";
    assert!(validated(&vault, card).is_ok(), "{:?}", validated(&vault, card));
    let keys: Vec<String> = meta(&vault, card).keys().filter_map(crate::yaml::text).collect();
    assert_eq!(
        keys,
        ["type", "kind", "title", "status", "target", "proposed_at", "first_proposed_at", "expires", "snooze_until", "created_by",
         "judgment_id", "judgment_kind", "source_uid", "from", "mail_date", "evidence", "changes", "id"]
    );
    let get = |key: &str| field(&vault, card, key);
    assert_eq!(get("created_by"), Some(json!("mail")));
    assert_eq!(get("from"), Some(json!("Course Staff <staff@example.edu>")));
    assert_eq!(get("mail_date"), Some(json!("2026-10-01")));
    assert_eq!(get("evidence"), Some(json!("Lab 5 is now due Monday")));
    assert_eq!(get("source_uid"), Some(json!("mail:bb:0")));
    assert_eq!(get("judgment_kind"), Some(json!("mail_claim")));
    assert_eq!(get("expires"), Some(json!(null)));
    assert_eq!(get("changes"), Some(json!({"due": {"from": "2026-10-03", "to": "2026-10-08"}})));
    let text = read(&vault, card);
    assert!(!text.contains("\nappend:"), "a mail card never carries `append` (R15)");
    assert!(text.contains("**Why proposed:** The instructor moved the due date"));
    assert!(text.contains("meta-bind-button"));
}

#[test]
fn a_fill_due_card_and_an_unarchive_card_pass_validate_amendment() {
    // `from: null` on an undated note.
    let vault = scratch("fill");
    mail_note(&vault, "tasks/lab-5.md", "null");
    let m = message("mail:bb", Phase::Live, vec![change("Lab 5", "2026-10-08")]);
    let fill = due_card(at("mail:bb", 0), "tasks/lab-5.md", json!(null), "2026-10-08", pending());
    assert_eq!(apply(&vault, &fill, &m, 0, held()), Wrote::Card("approvals/amend-lab-5-due.md".to_string()));
    let card = "approvals/amend-lab-5-due.md";
    assert!(validated(&vault, card).is_ok(), "{:?}", validated(&vault, card));
    assert_eq!(field(&vault, card, "changes"), Some(json!({"due": {"from": null, "to": "2026-10-08"}})));
    assert_eq!(field(&vault, card, "title"), Some(json!("Email gave a due date for Lab 5")));

    // `target: archive/<name>` on an `imported-past` note.
    let vault = scratch("unarchive");
    let past = "---\ntitle: Lab 5\ncourse: CS 101\ndue: 2026-09-01\nstatus: archived\narchived_reason: imported-past\n\
                created_by: mail\nsource_uid: mail:aa:0\n---\n\nFrom email.\n";
    write::create(&vault, "archive/lab-5.md", past, &WriteContext::new(MAIL_ACTOR, VIA), &mut Journal::new(&vault), None).unwrap();
    let ahead = due_card(at("mail:bb", 0), "archive/lab-5.md", json!("2026-09-01"), "2026-10-20", pending());
    assert_eq!(apply(&vault, &ahead, &m, 0, held()), Wrote::Card("approvals/amend-lab-5-due.md".to_string()));
    assert!(validated(&vault, card).is_ok(), "{:?}", validated(&vault, card));
    assert_eq!(field(&vault, card, "target"), Some(json!("archive/lab-5.md")));
}

#[test]
fn a_card_the_executor_would_refuse_is_never_filed() {
    let vault = scratch("refused");
    mail_note(&vault, "tasks/lab-5.md", "2026-10-03");
    let m = message("mail:bb", Phase::Live, vec![change("Lab 5", "2026-10-08")]);
    let title = Action::AmendCard {
        claim: at("mail:bb", 0),
        target: Place::Path("tasks/lab-5.md".to_string()),
        changes: vec![Change { field: "title".to_string(), from: json!("Lab 5"), to: json!("Lab 6") }],
        proposal: pending(),
        also: Vec::new(),
    };
    assert_eq!(apply(&vault, &title, &m, 0, held()), Wrote::Nothing(Why::NotAmendable));
    assert!(files(&vault, "approvals").is_empty());
}

#[test]
fn a_task_card_is_write_gmail_cards_shape_with_the_note_as_its_payload() {
    let vault = scratch("task");
    let m = message("mail:bb", Phase::Live, vec![new_item("Lab 6", Some("2026-10-20"))]);
    let plan = mailreconcile::plan(&VaultView::default(), std::slice::from_ref(&m), today(), 1);
    let [action @ Action::TaskCard { .. }] = &plan[..] else { panic!("{plan:?}") };
    assert_eq!(apply(&vault, action, &m, 0, held()), Wrote::Card("approvals/task-lab-6.md".to_string()));
    let card = "approvals/task-lab-6.md";
    let got = ["kind", "status", "proposed_at", "snooze_until", "created_by", "source_uid", "from", "mail_date", "expires"].map(|k| field(&vault, card, k));
    let want = ["task", "snoozed", "2026-10-02", "2026-10-02", "mail", "mail:bb:0", "Course Staff <staff@example.edu>", "2026-10-01", "2026-10-16"];
    assert_eq!(got.map(|v| v.unwrap()), want.map(|w| json!(w)));
    let text = read(&vault, card);
    let payload = text.split_once("```task\n").unwrap().1.strip_suffix("```\n").unwrap();
    let (note, _) = split_frontmatter(payload).unwrap();
    let note = |key: &str| crate::yaml::get(&note, key).map(crate::yaml::to_json);
    assert_eq!((note("created_by"), note("needs_enrichment"), note("source_uid")), (Some(json!("mail")), Some(json!(false)), Some(json!("mail:bb:0"))));
}
