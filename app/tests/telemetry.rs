//! (a) and (b), derived on the device. The rule under every assertion here: **nothing that leaves is
//! content**. An id is not content; a number is not content; a title, a course name and a note body
//! are.
use knowlu::telemetry::{read_corrections, read_events, Batch, VALUED_FIELDS, FLAGGED_FIELDS};
use std::io::{Read, Write};
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

/// Fix round 1 (C1, ruling R-C1-39): `domain`, `effort_confidence` and `status` are free-text inputs
/// in the console with no vocabulary check anywhere in the write path — so a `VALUED_FIELDS`
/// correction's `theirs` can be a sentence, and D5 forbids free text on the wire. The row must
/// survive (the fact of a correction is still the eval suite's signal); the sentence must not.
#[test]
fn a_free_text_correction_on_a_valued_field_keeps_the_row_and_drops_only_the_sentence() {
    let v = vault("freetext");
    write(&v.join("state/journal/2026-09-10.jsonl"), &[
        // The agent judged it with a closed-vocabulary value…
        r#"{"actor": "agent:knowlu.enrich", "device": "M", "evidence": null, "field": "effort_confidence", "id": "task_0000000005", "new": "low", "old": null, "op": "set", "path": "tasks/e.md", "run_id": null, "ts": "2026-09-10T08:30:00.000Z", "via": "local-runner"}"#,
        // …and the human retyped the chip as a sentence. Nothing stops that in the console today.
        r#"{"actor": "quinn", "device": "M", "evidence": null, "field": "effort_confidence", "id": "task_0000000005", "new": "I honestly have no idea how long this will take", "old": "low", "op": "set", "path": "tasks/e.md", "run_id": null, "ts": "2026-09-10T09:30:00.000Z", "via": "dashboard"}"#,
    ]);
    let cs = read_corrections(&v, None);
    assert_eq!(cs.len(), 1, "{cs:?}");
    assert_eq!(cs[0].field, "effort_confidence");
    assert_eq!(cs[0].ours.as_deref(), Some("low"), "a closed-vocabulary value still travels");
    assert_eq!(cs[0].theirs, None, "a sentence must never reach theirs");
    let batch = Batch { events: vec![], corrections: cs };
    let wire = serde_json::to_string(&batch).unwrap();
    assert!(!wire.contains("I honestly have no idea"), "{wire}");
    let _ = std::fs::remove_dir_all(&v);
}

/// Fix round 1 (M2): a capped page must never end on a `ts` it shares with a row that got cut — the
/// watermark, taken from the kept page's last `ts`, would otherwise land mid-tie and strand the
/// untransmitted twin on every later read (`since` is exclusive). 501 rows, with row #500 (index 499)
/// and row #501 (index 500) sharing a `ts` — exactly what one `IntersectionObserver` callback firing
/// twice in the same millisecond would produce.
#[test]
fn a_tied_timestamp_at_the_cap_boundary_never_splits_the_watermark_mid_tie() {
    use knowlu::telemetry::{truncate_on_ts_boundary, EventRow};
    let mut rows: Vec<EventRow> = (0..499)
        .map(|i| EventRow {
            ts: format!("2026-09-09T{:02}:{:02}:{:02}.000Z", i / 3600, (i / 60) % 60, i % 60),
            session: "sess_1".into(),
            view: "today".into(),
            action: "view_opened".into(),
            object_id: None,
            object_kind: None,
            ms: None,
        })
        .collect();
    let tied_ts = "2026-09-09T23:59:59.000Z".to_string();
    for _ in 0..2 {
        rows.push(EventRow {
            ts: tied_ts.clone(),
            session: "sess_1".into(),
            view: "today".into(),
            action: "view_opened".into(),
            object_id: None,
            object_kind: None,
            ms: None,
        });
    }
    assert_eq!(rows.len(), 501);
    assert_eq!(rows[499].ts, rows[500].ts, "the fixture's own property: #500 and #501 tie");
    truncate_on_ts_boundary(&mut rows, 500, |r| r.ts.as_str());
    // The tie is deferred whole: neither tied row is in the kept page, and the kept page's last `ts`
    // is strictly earlier than the tie — so a watermark taken from it can never land mid-tie.
    assert_eq!(rows.len(), 499, "{}", rows.len());
    assert!(rows.last().unwrap().ts < tied_ts, "{}", rows.last().unwrap().ts);
}

/// Fix round 2: the boundary-pair cut above has its own edge when the tie is not a pair but the
/// *entire* overflow page — 501 rows, all sharing one `ts` (the more extreme version of the same
/// "several UI events in one millisecond" scenario, an unrealistic burst that reads as a corrupt or
/// hand-edited ledger). Cutting to empty would defer this same page every slot forever and never make
/// progress; the ruled fix keeps the whole page instead, losing only the one row past the cap — no
/// panic, no empty batch, and the watermark still advances past the tie.
#[test]
fn a_fully_tied_overflow_page_sends_the_whole_page_rather_than_going_empty() {
    use knowlu::telemetry::{truncate_on_ts_boundary, EventRow};
    let tied_ts = "2026-09-09T23:59:59.000Z".to_string();
    let mut rows: Vec<EventRow> = (0..501)
        .map(|_| EventRow {
            ts: tied_ts.clone(),
            session: "sess_1".into(),
            view: "today".into(),
            action: "view_opened".into(),
            object_id: None,
            object_kind: None,
            ms: None,
        })
        .collect();
    truncate_on_ts_boundary(&mut rows, 500, |r| r.ts.as_str());
    assert_eq!(rows.len(), 500, "the whole page sends rather than dropping to one row or to empty");
    assert!(rows.iter().all(|r| r.ts == tied_ts));
}

/// `account.rs`'s own loopback harness, copied rather than shared — each `tests/*.rs` file is its
/// own crate. Serves exactly `responses.len()` requests on `127.0.0.1:0`, then stops. Returns the
/// base URL and a handle whose `join()` yields the raw request text of each one, bounded on both
/// ends so a test that forgets to send a request fails loudly in seconds instead of hanging the
/// suite.
fn loopback(responses: Vec<(u16, String)>) -> (String, std::thread::JoinHandle<Vec<String>>) {
    let listener = std::net::TcpListener::bind("127.0.0.1:0").expect("bind loopback");
    listener.set_nonblocking(true).expect("nonblocking listener");
    let port = listener.local_addr().expect("addr").port();
    let handle = std::thread::spawn(move || {
        let mut seen = Vec::new();
        for (status, body) in responses {
            let deadline = std::time::Instant::now() + std::time::Duration::from_secs(10);
            let mut stream = loop {
                match listener.accept() {
                    Ok((s, _)) => break s,
                    Err(e) if e.kind() == std::io::ErrorKind::WouldBlock => {
                        if std::time::Instant::now() >= deadline {
                            panic!("loopback: no client connected within 10s");
                        }
                        std::thread::sleep(std::time::Duration::from_millis(10));
                    }
                    Err(e) => panic!("loopback: accept failed: {e}"),
                }
            };
            stream.set_nonblocking(false).expect("blocking stream");
            stream
                .set_read_timeout(Some(std::time::Duration::from_secs(10)))
                .expect("read timeout");
            let mut buf: Vec<u8> = Vec::new();
            let mut chunk = [0u8; 1024];
            let head_end = loop {
                let n = stream.read(&mut chunk).unwrap_or(0);
                if n == 0 {
                    break buf.len();
                }
                buf.extend_from_slice(&chunk[..n]);
                if let Some(i) = buf.windows(4).position(|w| w == b"\r\n\r\n") {
                    break i + 4;
                }
            };
            let head = String::from_utf8_lossy(&buf[..head_end]).to_string();
            let want: usize = head
                .lines()
                .find(|l| l.to_ascii_lowercase().starts_with("content-length:"))
                .and_then(|l| l.split(':').nth(1)?.trim().parse().ok())
                .unwrap_or(0);
            while buf.len() < head_end + want {
                let n = stream.read(&mut chunk).unwrap_or(0);
                if n == 0 {
                    break;
                }
                buf.extend_from_slice(&chunk[..n]);
            }
            seen.push(String::from_utf8_lossy(&buf).to_string());
            let reason = if (200..300).contains(&status) { "OK" } else { "Bad Request" };
            let resp = format!(
                "HTTP/1.1 {status} {reason}\r\ncontent-type: application/json\r\ncontent-length: {}\r\nconnection: close\r\n\r\n{body}",
                body.len()
            );
            let _ = stream.write_all(resp.as_bytes());
            let _ = stream.flush();
        }
        seen
    });
    (format!("http://127.0.0.1:{port}"), handle)
}

fn sample_batch() -> Batch {
    use knowlu::telemetry::CorrectionRow;
    Batch {
        events: vec![],
        corrections: vec![CorrectionRow {
            ts: "2026-09-10T09:00:00.000Z".into(),
            item_id: "task_0123456789".into(),
            field: "effort_hours".into(),
            ours: Some("2.0".into()),
            theirs: Some("0.5".into()),
            kind: "task".into(),
        }],
    }
}

/// Fix round 1 (I3): `post_batch_at` had no test at all. One compact JSON body line, the bearer in
/// `authorization`, `content-type: application/json` — and the reply's counts come back as the tuple.
#[test]
fn post_batch_at_sends_one_compact_json_line_with_the_bearer_and_reads_the_reply() {
    let (base, handle) = loopback(vec![(200, r#"{"events":1,"corrections":0}"#.to_string())]);
    let out = knowlu::telemetry::post_batch_at(&format!("{base}/functions/v1"), "test-bearer-token", &sample_batch());
    let seen = handle.join().expect("server thread");
    let (events, corrections) = out.expect("post ok");
    assert_eq!((events, corrections), (1, 0));
    let req = &seen[0];
    assert!(req.starts_with("POST /functions/v1/telemetry "), "{req}");
    assert!(req.to_ascii_lowercase().contains("content-type: application/json"), "{req}");
    assert!(req.to_ascii_lowercase().contains("authorization: bearer test-bearer-token"), "{req}");
    let body = req.split("\r\n\r\n").nth(1).unwrap_or("");
    assert!(!body.contains('\n'), "the body must be one compact line: {body}");
    assert!(body.contains("\"item_id\":"), "{body}");
}

/// Fix round 1 (I2, I3): a 400 is a status refusal, never a transport error — `send` tells the two
/// apart to decide whether the batch is consumed or kept for the next slot.
#[test]
fn post_batch_at_treats_a_400_as_a_status_refusal_not_a_transport_error() {
    use knowlu::telemetry::PostError;
    let (base, handle) = loopback(vec![(400, r#"{"error":"bad request"}"#.to_string())]);
    let out = knowlu::telemetry::post_batch_at(&format!("{base}/functions/v1"), "t", &sample_batch());
    let _seen = handle.join().expect("server thread");
    assert_eq!(out, Err(PostError::Status(400)));
}
