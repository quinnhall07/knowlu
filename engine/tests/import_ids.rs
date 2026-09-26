//! Two-desktop design D1–D3 from outside the crate: the LMS feed's producer, `ingest::sync_tasks`
//! (hand-off H1), mints the item's import id, and a vault that already holds it skips the item by name.

use std::path::{Path, PathBuf};

use knowlu_engine::ingest::{load_seen, sync_tasks, Due, Event};
use knowlu_engine::journal::Journal;

fn vault(tag: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!("knowlu-importids-{tag}-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    for folder in ["tasks", "archive", "state"] {
        std::fs::create_dir_all(dir.join(folder)).expect("mkdir");
    }
    dir
}

/// One Blackboard item, carrying the uid behind §2.1's second reference value.
fn quiz() -> Event {
    Event {
        uid: "_blackboard.platform.gradebook2.GradableItem-_4732722_1".to_string(),
        title: "Quiz 3".to_string(),
        due: Some(Due::DateTime(jiff::civil::date(2026, 10, 2).at(23, 59, 0, 0))),
        description: "Chapter 3".to_string(),
        raw: String::new(),
    }
}

fn id_of(path: &Path) -> String {
    let meta = knowlu_engine::ids::read_meta(path).expect("the note");
    knowlu_engine::yaml::get(&meta, "id").and_then(knowlu_engine::yaml::text).expect("an id")
}

#[test]
fn the_lms_feed_mints_the_items_import_id() {
    let v = vault("lms");
    let mut journal = Journal::new(&v);
    let log = sync_tasks(&[quiz()], &v, &[], None, &mut journal, Some(jiff::civil::date(2026, 9, 25)), false);
    assert_eq!(log, vec!["created task-quiz-3".to_string()]);
    assert_eq!(id_of(&v.join("tasks").join("task-quiz-3.md")), "task_d8891a504b", "§2.1's reference value");
    let _ = std::fs::remove_dir_all(&v);
}

#[test]
fn an_lms_item_archived_on_a_first_run_carries_the_same_import_id() {
    let v = vault("lms-past");
    let mut journal = Journal::new(&v);
    let log = sync_tasks(&[quiz()], &v, &[], None, &mut journal, Some(jiff::civil::date(2026, 10, 5)), true);
    assert_eq!(log, vec!["archived (imported-past) task-quiz-3".to_string()]);
    assert_eq!(id_of(&v.join("archive").join("task-quiz-3.md")), "task_d8891a504b");
    let _ = std::fs::remove_dir_all(&v);
}

/// R-TD1-1: a note carrying the item's import id but not its `source_uid` (so dedup cannot find it)
/// makes the create `IdHeld` — named, not written, and the uid recorded as seen.
#[test]
fn an_lms_item_whose_id_is_already_held_is_skipped_by_name_and_recorded_seen() {
    let v = vault("lms-held");
    std::fs::write(v.join("tasks").join("renamed.md"), "---\ntitle: \"Quiz 3\"\nid: task_d8891a504b\n---\n\nb\n")
        .expect("seed");
    let mut journal = Journal::new(&v);
    let log = sync_tasks(&[quiz()], &v, &[], None, &mut journal, Some(jiff::civil::date(2026, 9, 25)), false);
    assert_eq!(log, vec!["skipped (already held as task_d8891a504b): task-quiz-3".to_string()]);
    assert!(!v.join("tasks").join("task-quiz-3.md").exists());
    assert!(load_seen(&v).contains("_blackboard.platform.gradebook2.GradableItem-_4732722_1"));
    let _ = std::fs::remove_dir_all(&v);
}
