//! Spec test 29: `mail-applied` lists the mail pass's own writes, joined to each note's current value.
//!
//! The vaults are written by the real `mail::Writer`, so the body line the parser reads is the one
//! the pass writes. Every vault is a temporary directory; nothing touches a network.

use std::path::{Path, PathBuf};

use jiff::civil::date;
use jiff::Timestamp;
use serde_json::{json, Value};

use super::*;
use crate::mail::{Source, Writer, MAIL_ACTOR};
use crate::mailreconcile::{
    Action, Authenticity, CardStatus, Claim, ClaimKind, ClaimRef, ClaimSource, ItemKind, Message, NewNote, Phase, Place, Proposal, Sender,
};
use crate::write::{self, WriteContext, WriteOpts};

const VIA: &str = "local-runner";
const LINE: &str = "2026-10-01 \u{b7} Course Staff <staff@example.edu>: The instructor moved the due date \u{2014} \"Lab 5 is now due Monday\"";

fn scratch(tag: &str) -> PathBuf {
    let vault = std::env::temp_dir().join(format!("knowlu-mailapplied-{tag}-{}-{:?}", std::process::id(), std::thread::current().id()));
    let _ = std::fs::remove_dir_all(&vault);
    for folder in ["tasks", "approvals", "archive", "info", "state", "config"] {
        std::fs::create_dir_all(vault.join(folder)).unwrap();
    }
    vault
}

fn ts(text: &str) -> Timestamp {
    text.parse().unwrap()
}

fn claim() -> Claim {
    Claim {
        kind: ClaimKind::Change,
        course: Some("CS 101".to_string()),
        target: None,
        field: None,
        value: None,
        item: None,
        topic: None,
        evidence: "Lab 5 is now due Monday".to_string(),
        why: "The instructor moved the due date".to_string(),
        confidence: 0.9,
        source: ClaimSource::Model,
        resolution: None,
    }
}

fn message() -> Message {
    Message {
        uid: "mail:aa".to_string(),
        ordering_date: ts("2026-10-01T12:00:00Z"),
        message_date: Some(ts("2026-10-01T11:00:00Z")),
        phase: Phase::Backfill,
        authenticity: Authenticity::Aligned,
        sender: Sender { name: Some("Course Staff".to_string()), address: "staff@example.edu".to_string() },
        claims: vec![claim()],
    }
}

fn proposal() -> Proposal {
    let day = date(2026, 10, 1);
    Proposal { status: CardStatus::Pending, proposed_at: day, first_proposed_at: day, snooze_until: None }
}

fn new_note(title: &str, due: &str, past: bool) -> NewNote {
    NewNote {
        title: title.to_string(),
        course: Some("CS 101".to_string()),
        due: Some(due.to_string()),
        item_kind: ItemKind::Task,
        effort_hours: 2.0,
        importance: 3,
        imported_past: past,
        mail_asof: Some(ts("2026-10-01T12:00:00Z")),
    }
}

/// Run planned actions through the real writer, as the mail actor.
fn mail_writes(vault: &Path, actions: &[Action]) {
    let (m, mut journal) = (message(), Journal::new(vault));
    let mut writer = Writer::new(vault, VIA, None, &mut journal, date(2026, 10, 1), proposal());
    let src = Source { message: &m, claim: &m.claims[0], judgment_id: None };
    for action in actions {
        writer.apply(action, &src).unwrap();
    }
}

fn at(index: usize) -> ClaimRef {
    ClaimRef { message: "mail:aa".to_string(), index }
}

fn set_due(rel: &str, to: &str) -> Action {
    Action::Set {
        claim: at(0),
        target: Place::Path(rel.to_string()),
        field: "due".to_string(),
        to: json!(to),
        mail_asof: ts("2026-10-01T12:00:00Z"),
        body_line: LINE.to_string(),
    }
}

fn student() -> WriteContext {
    // Every scratch vault has no `config/actor.yaml`, which reads as the legacy human token.
    WriteContext::new(crate::journal::LEGACY_HUMAN_ACTOR, "dashboard")
}

/// A note the course feed made (an agent, not the student), so mail may set its due directly.
fn feed_note(vault: &Path, rel: &str, due: &str) {
    let text = format!("---\ntitle: Lab 5\ncourse: CS 101\ndomain: school\ndue: {due}\nimportance: 4\nstatus: active\n---\n\nFrom the feed.\n");
    let ctx = WriteContext::new("agent:knowlu.ingest", VIA);
    write::create(vault, rel, &text, &ctx, &mut Journal::new(vault), None).unwrap();
}

fn student_sets(vault: &Path, rel: &str, field: &str, to: &str) {
    let changes = [(field.to_string(), serde_yaml_ng::Value::String(to.to_string()))];
    write::write(vault, rel, &changes, &student(), &mut Journal::new(vault), &WriteOpts::default()).unwrap();
}

fn notice(vault: &Path, key: &str) {
    let mut journal = Journal::new(vault);
    let mut writer = Writer::new(vault, VIA, None, &mut journal, date(2026, 10, 1), proposal());
    writer.notice("notice", Some(key), "Room changed", "CS 101 moved to Hall B").unwrap();
}

fn course<'a>(out: &'a Value, name: Option<&str>) -> &'a Value {
    let found = out["courses"].as_array().unwrap().iter().find(|c| c["course"].as_str() == name);
    found.unwrap_or_else(|| panic!("no group for {name:?} in {out}"))
}

/// Every file under the vault, with its bytes.
fn snapshot(vault: &Path) -> Vec<(PathBuf, Vec<u8>)> {
    fn walk(dir: &Path, out: &mut Vec<(PathBuf, Vec<u8>)>) {
        let mut entries: Vec<_> = std::fs::read_dir(dir).unwrap().flatten().map(|e| e.path()).collect();
        entries.sort();
        for path in entries {
            match path.is_dir() {
                true => walk(&path, out),
                false => out.push((path.clone(), std::fs::read(&path).unwrap())),
            }
        }
    }
    let mut out = Vec::new();
    walk(vault, &mut out);
    out
}

#[test]
fn mail_applied_lists_each_mail_write_joined_to_the_notes_current_value() {
    let vault = scratch("lists");
    feed_note(&vault, "tasks/lab-5.md", "2026-10-05");
    mail_writes(
        &vault,
        &[
            Action::Create { claim: at(0), note: new_note("Lab 6", "2026-10-20", false) },
            Action::Create { claim: at(1), note: new_note("Lab 1", "2026-09-01", true) },
            set_due("tasks/lab-5.md", "2026-10-07"),
        ],
    );
    notice(&vault, "room:cs101");

    let out = applied(&vault, None);
    let group = course(&out, Some("CS 101"));

    let created = group["created"].as_array().unwrap();
    assert_eq!(created.len(), 1, "{group}");
    assert_eq!(created[0]["title"], json!("Lab 6"));
    assert_eq!(created[0]["due"], json!("2026-10-20"));
    assert_eq!(created[0]["path"], json!("tasks/cs-101-lab-6.md"));
    assert_eq!(created[0]["changed_since"], json!(false));

    let archived = &group["archived"];
    assert_eq!(archived["count"], json!(1));
    assert_eq!(archived["items"][0]["title"], json!("Lab 1"));

    let changed = group["changed"].as_array().unwrap();
    assert_eq!(changed.len(), 1, "mail_asof is bookkeeping, never a row: {group}");
    let row = &changed[0];
    assert_eq!((&row["field"], &row["old"], &row["new"]), (&json!("due"), &json!("2026-10-05"), &json!("2026-10-07")));
    assert_eq!(row["current"], json!("2026-10-07"));
    assert_eq!(row["sender"], json!("Course Staff <staff@example.edu>"));
    assert_eq!(row["date"], json!("2026-10-01"));
    assert_eq!(row["changed_since"], json!(false));
    assert_eq!(row["path"], json!("tasks/lab-5.md"));

    // A notice has no course: its own group, last. The feed's own create is nobody's row here.
    let notices = course(&out, None)["notices"].as_array().unwrap();
    assert_eq!((notices.len(), &notices[0]["title"], &notices[0]["changed_since"]), (1, &json!("Room changed"), &json!(false)));
    assert_eq!(out["courses"].as_array().unwrap().last().unwrap()["course"], json!(null));
}

#[test]
fn a_note_changed_since_or_gone_reads_changed_since() {
    let vault = scratch("changed");
    feed_note(&vault, "tasks/lab-5.md", "2026-10-05");
    mail_writes(
        &vault,
        &[Action::Create { claim: at(0), note: new_note("Lab 6", "2026-10-20", false) }, set_due("tasks/lab-5.md", "2026-10-07")],
    );
    notice(&vault, "room:cs101");

    // The student moves the due date again and deletes the mail's new note; the notice closes.
    student_sets(&vault, "tasks/lab-5.md", "due", "2026-10-09");
    write::delete(&vault, "tasks/cs-101-lab-6.md", &student(), &mut Journal::new(&vault)).unwrap();
    let ctx = WriteContext::new(MAIL_ACTOR, VIA);
    crate::info::close_info(&vault, Some("room:cs101"), None, MAIL_ACTOR, &ctx, None, None).unwrap();

    let out = applied(&vault, None);
    let group = course(&out, Some("CS 101"));
    assert_eq!(group["changed"][0]["changed_since"], json!(true), "{group}");
    assert_eq!(group["changed"][0]["current"], json!("2026-10-09"));
    assert_eq!(group["created"][0]["changed_since"], json!(true), "a deleted note is gone: {group}");
    assert_eq!(group["created"][0]["current"], json!(null));
    assert_eq!(course(&out, None)["notices"][0]["changed_since"], json!(true));
}

#[test]
fn mail_applied_leaves_the_vaults_bytes_and_journal_unchanged() {
    let vault = scratch("readonly");
    feed_note(&vault, "tasks/lab-5.md", "2026-10-05");
    mail_writes(
        &vault,
        &[Action::Create { claim: at(0), note: new_note("Lab 6", "2026-10-20", false) }, set_due("tasks/lab-5.md", "2026-10-07")],
    );
    notice(&vault, "room:cs101");

    let before = snapshot(&vault);
    let (all, since) = (applied(&vault, None), applied(&vault, Some("2999-01-01")));
    let text = run(&vault, None);
    assert_eq!(snapshot(&vault), before, "no file, journal line or directory changed");
    assert_eq!(since["courses"], json!([]), "--since a day after every record lists nothing");
    assert!(all["courses"].as_array().is_some_and(|c| !c.is_empty()));
    assert_eq!(text, crate::ledger::dumps_value(&all), "the printed JSON is dumps_value's");
}

#[test]
fn a_body_line_reads_back_its_date_and_sender() {
    let pair = |line: &str| parse_line(line);
    let (day, sender) = pair(LINE);
    assert_eq!((day.as_deref(), sender.as_deref()), (Some("2026-10-01"), Some("Course Staff <staff@example.edu>")));
    let (_, sender) = pair("2026-10-01 \u{b7} Prof: Smith <ps@example.edu>: moved \u{2014} \"x\"");
    assert_eq!(sender.as_deref(), Some("Prof: Smith <ps@example.edu>"));
    let (_, sender) = pair("2026-10-01 \u{b7} ps@example.edu: moved to <b@c>: later \u{2014} \"x\"");
    assert_eq!(sender.as_deref(), Some("ps@example.edu"));
    assert_eq!(pair("no separator"), (None, None));
}

#[test]
fn a_student_edit_other_than_title_or_due_reads_changed_since() {
    let vault = scratch("other-edit");
    mail_writes(&vault, &[Action::Create { claim: at(0), note: new_note("Lab 6", "2026-10-20", false) }]);
    let out = applied(&vault, None);
    assert_eq!(course(&out, Some("CS 101"))["created"][0]["changed_since"], json!(false), "untouched: {out}");

    // The student marks it done: title and due still match what mail wrote.
    student_sets(&vault, "tasks/cs-101-lab-6.md", "status", "done");
    let out = applied(&vault, None);
    let row = &course(&out, Some("CS 101"))["created"][0];
    assert_eq!((&row["current"]["title"], &row["current"]["due"]), (&json!("Lab 6"), &json!("2026-10-20")));
    assert_eq!(row["changed_since"], json!(true), "a changed status must hide Undo: {row}");
}

#[test]
fn a_note_with_no_create_record_is_found_by_the_path_mail_set_carries() {
    let vault = scratch("no-create");
    // A note the student wrote by hand (an id, no journal create).
    let text = "---\nid: task_aabbccddee\ntitle: Essay\ncourse: CS 101\ndomain: school\ndue: 2026-10-05\nimportance: 3\nstatus: active\n---\n\nMine.\n";
    std::fs::write(vault.join("tasks/essay.md"), text).unwrap();
    mail_writes(&vault, &[set_due("tasks/essay.md", "2026-10-07")]);

    let out = applied(&vault, None);
    let row = &course(&out, Some("CS 101"))["changed"][0];
    assert_eq!(row["current"], json!("2026-10-07"), "{row}");
    assert_eq!(row["path"], json!("tasks/essay.md"));
    assert_eq!(row["changed_since"], json!(false), "a valid, unchanged mail edit keeps Undo: {row}");
}
