//! (a) and (b), derived on the device. The rule under every assertion here: **nothing that leaves is
//! content**. An id is not content; a number is not content; a title, a course name and a note body
//! are.
use knowlu::telemetry::{read_corrections, read_events, Batch, VALUED_FIELDS, FLAGGED_FIELDS};
use std::path::{Path, PathBuf};

fn vault(tag: &str) -> PathBuf {
    let d = std::env::temp_dir().join(format!("knowlu-tele-{tag}-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&d);
    std::fs::create_dir_all(d.join("state").join("events-ui")).unwrap();
    std::fs::create_dir_all(d.join("state").join("journal")).unwrap();
    d
}

fn write(path: &Path, lines: &[&str]) {
    std::fs::write(path, format!("{}\n", lines.join("\n"))).unwrap();
}

#[test]
fn interaction_events_come_off_the_ledger_and_carry_no_free_text() {
    let v = vault("events");
    write(&v.join("state/events-ui/2026-09-09.jsonl"), &[
        r#"{"action": "view_opened", "device": "M", "ms": null, "object_id": null, "object_kind": null, "session": "sess_1", "ts": "2026-09-09T12:00:00.000Z", "view": "today"}"#,
        r#"{"action": "object_seen", "device": "M", "ms": 2400, "object_id": "task_0123456789", "object_kind": "task", "session": "sess_1", "ts": "2026-09-09T12:00:02.000Z", "view": "today"}"#,
    ]);
    write(&v.join("state/events-ui/2026-09-10.jsonl"), &[
        r#"{"action": "decision_made", "device": "M", "ms": null, "object_id": "appr_0000000001", "object_kind": "approval", "session": "sess_2", "ts": "2026-09-10T09:00:00.000Z", "view": "decisions"}"#,
    ]);
    let all = read_events(&v, None);
    assert_eq!(all.len(), 3);
    assert_eq!(all[0].action, "view_opened");
    assert_eq!(all[1].object_id.as_deref(), Some("task_0123456789"));
    assert_eq!(all[2].view, "decisions");
    // The watermark is exclusive: the same batch is never sent twice.
    let after = read_events(&v, Some("2026-09-09T12:00:02.000Z"));
    assert_eq!(after.len(), 1);
    assert_eq!(after[0].action, "decision_made");
    let _ = std::fs::remove_dir_all(&v);
}

#[test]
fn a_correction_is_a_human_overriding_a_field_the_agent_set() {
    let v = vault("corrections");
    write(&v.join("state/journal/2026-09-10.jsonl"), &[
        // The agent judged it…
        r#"{"actor": "agent:knowlu.enrich", "device": "M", "evidence": null, "field": "effort_hours", "id": "task_0123456789", "new": 2.0, "old": null, "op": "set", "path": "tasks/a.md", "run_id": null, "ts": "2026-09-10T08:00:00.000Z", "via": "local-runner"}"#,
        // …and the human disagreed, from the console. That is a correction.
        r#"{"actor": "quinn", "device": "M", "evidence": null, "field": "effort_hours", "id": "task_0123456789", "new": 0.5, "old": 2.0, "op": "set", "path": "tasks/a.md", "run_id": null, "ts": "2026-09-10T09:00:00.000Z", "via": "dashboard"}"#,
        // A human editing a field NO agent ever set is not a correction — it is just work.
        r#"{"actor": "quinn", "device": "M", "evidence": null, "field": "importance", "id": "task_0000000002", "new": 3, "old": null, "op": "set", "path": "tasks/b.md", "run_id": null, "ts": "2026-09-10T09:05:00.000Z", "via": "dashboard"}"#,
        // A course correction: the row survives, the two course NAMES do not.
        r#"{"actor": "agent:knowlu.enrich", "device": "M", "evidence": null, "field": "course", "id": "task_0000000003", "new": "MATH 125", "old": null, "op": "set", "path": "tasks/c.md", "run_id": null, "ts": "2026-09-10T08:10:00.000Z", "via": "local-runner"}"#,
        r#"{"actor": "quinn", "device": "M", "evidence": null, "field": "course", "id": "task_0000000003", "new": "SPAN 101", "old": "MATH 125", "op": "set", "path": "tasks/c.md", "run_id": null, "ts": "2026-09-10T09:10:00.000Z", "via": "dashboard"}"#,
        // A title is neither valued nor flagged: no row at all.
        r#"{"actor": "agent:knowlu.enrich", "device": "M", "evidence": null, "field": "title", "id": "task_0000000004", "new": "Read chapter 3", "old": null, "op": "set", "path": "tasks/d.md", "run_id": null, "ts": "2026-09-10T08:20:00.000Z", "via": "local-runner"}"#,
        r#"{"actor": "quinn", "device": "M", "evidence": null, "field": "title", "id": "task_0000000004", "new": "Read ch. 3 of Calculus", "old": "Read chapter 3", "op": "set", "path": "tasks/d.md", "run_id": null, "ts": "2026-09-10T09:20:00.000Z", "via": "dashboard"}"#,
    ]);
    let cs = read_corrections(&v, None);
    assert_eq!(cs.len(), 2, "{cs:?}");
    assert_eq!(cs[0].item_id, "task_0123456789");
    assert_eq!(cs[0].field, "effort_hours");
    assert_eq!(cs[0].ours.as_deref(), Some("2.0"));
    assert_eq!(cs[0].theirs.as_deref(), Some("0.5"));
    assert_eq!(cs[0].kind, "task");
    assert_eq!(cs[1].field, "course");
    assert_eq!(cs[1].ours, None, "a course name is content and never leaves");
    assert_eq!(cs[1].theirs, None);
    // The whole file, read as text, must not contain either course name or the title.
    let batch = Batch { events: vec![], corrections: cs };
    let wire = serde_json::to_string(&batch).unwrap();
    for content in ["MATH 125", "SPAN 101", "Read chapter 3", "Read ch. 3 of Calculus"] {
        assert!(!wire.contains(content), "{content} reached the wire: {wire}");
    }
    assert!(VALUED_FIELDS.contains(&"effort_hours") && !VALUED_FIELDS.contains(&"course"));
    assert!(FLAGGED_FIELDS.contains(&"course"));
    let _ = std::fs::remove_dir_all(&v);
}

/// The watermark may never step past a stream that was truncated — the bug that would silently drop
/// events 501-600 forever and leave the comment claiming the opposite.
#[test]
fn a_capped_batch_advances_the_watermark_only_as_far_as_the_slower_stream() {
    use knowlu::telemetry::{read_corrections, read_events, watermark};
    let v = vault("watermark");
    // 600 events across two days, and one correction dated after all of them.
    let day = |n: usize, base: &str| -> Vec<String> {
        (0..n).map(|i| format!(
            r#"{{"action": "view_opened", "device": "M", "ms": null, "object_id": null, "object_kind": null, "session": "sess_1", "ts": "{base}T{:02}:{:02}:{:02}.000Z", "view": "today"}}"#,
            i / 3600, (i / 60) % 60, i % 60)).collect()
    };
    let d1 = day(600, "2026-09-09");
    write(&v.join("state/events-ui/2026-09-09.jsonl"), &d1.iter().map(String::as_str).collect::<Vec<_>>());
    write(&v.join("state/journal/2026-09-10.jsonl"), &[
        r#"{"actor": "agent:knowlu.enrich", "device": "M", "evidence": null, "field": "importance", "id": "task_0000000009", "new": 2, "old": null, "op": "set", "path": "tasks/z.md", "run_id": null, "ts": "2026-09-10T08:00:00.000Z", "via": "local-runner"}"#,
        r#"{"actor": "quinn", "device": "M", "evidence": null, "field": "importance", "id": "task_0000000009", "new": 3, "old": 2, "op": "set", "path": "tasks/z.md", "run_id": null, "ts": "2026-09-10T09:00:00.000Z", "via": "dashboard"}"#,
    ]);
    let events = read_events(&v, None);
    let corrections = read_corrections(&v, None);
    assert_eq!(events.len(), 600);
    assert_eq!(corrections.len(), 1);
    // The fixture's own property first: the 500th event is EARLIER than the correction, so a
    // watermark taken as the maximum would step past events 501-600.
    assert!(events[499].ts < corrections[0].ts, "the fixture must have a later correction than the cap");
    // …and then the arithmetic itself, both branches. `send` needs a cloud config it will not get
    // here, so the rule lives in `watermark`, where a test can reach it.
    let e = events[499].ts.clone();
    let c = corrections[0].ts.clone();
    assert_eq!(watermark(Some(e.as_str()), Some(c.as_str()), true), e, "a capped batch stops at the earlier tail");
    assert_eq!(watermark(Some(e.as_str()), Some(c.as_str()), false), c, "an uncapped batch goes as far as either stream got");
    // One empty stream is still answerable, and an empty batch has no watermark at all.
    assert_eq!(watermark(None, Some(c.as_str()), true), c);
    assert_eq!(watermark(Some(e.as_str()), None, false), e);
    assert_eq!(watermark(None, None, false), String::new());
    let _ = std::fs::remove_dir_all(&v);
}

/// The engine owns the action vocabulary; this file must not drift from it. A cheap cross-language
/// pin: the TypeScript that receives these rows lists the same eleven, so a new action added to the
/// engine fails here until it lands in both.
#[test]
fn the_action_vocabulary_is_the_engines_on_both_sides_of_the_wire() {
    let ts = std::fs::read_to_string("../cloud/supabase/functions/telemetry/handler.ts").expect("the telemetry handler");
    for action in knowlu_engine::uievents::ACTIONS {
        assert!(ts.contains(&format!("\"{action}\"")), "the cloud does not accept {action}");
    }
    // …and the cloud accepts nothing the engine does not emit. Split on `= [` first: the declaration
    // is `export const ACTIONS: readonly string[] = [`, so splitting on `]` alone stops at `string[]`
    // and counts nothing at all.
    let listed = ts
        .split("export const ACTIONS")
        .nth(1)
        .and_then(|s| s.split("= [").nth(1))
        .and_then(|s| s.split(']').next())
        .expect("the ACTIONS array");
    assert_eq!(listed.matches('"').count() / 2, knowlu_engine::uievents::ACTIONS.len(), "{listed}");
}
