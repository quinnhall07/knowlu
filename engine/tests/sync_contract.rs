//! What the device sends, and where it starts from next time (C3' Task 5, engine/src/sync.rs).
//!
//! Every test that reaches a server here binds a `TcpListener` to `127.0.0.1:0`, serves its own
//! request from a second thread, and joins that thread before returning. That is not egress: no
//! DNS, no route off the machine, no listener on a routable interface (CLAUDE.md; plan 3a's
//! loopback rule, copied here rather than shared as a library — see the file header this was copied
//! from, `engine/tests/cloud_contract.rs`, for why two forty-line harnesses beat a third crate). No
//! test here touches a real vault; every one copies `tests/fixtures/vault-s1` into its own temp
//! directory first.

use std::io::{BufRead, BufReader, Read, Write};
use std::net::TcpListener;
use std::path::{Path, PathBuf};

use knowlu_engine::cloudmodel::{CloudClient, CloudConfig};
use knowlu_engine::journal::Journal;
use knowlu_engine::sync::{self, Cursor};

// ---------------------------------------------------------------------------
// The loopback harness — copied from `cloud_contract.rs` (review M7's accepted trade).
// ---------------------------------------------------------------------------

/// A loopback server that answers `replies` in order and hands back everything it was sent.
struct Loopback {
    base: String,
    handle: Option<std::thread::JoinHandle<Vec<String>>>,
}

impl Loopback {
    fn requests(&mut self) -> Vec<String> {
        self.handle.take().expect("joined once").join().expect("the listener thread did not panic")
    }
}

fn read_request(stream: &std::net::TcpStream) -> String {
    let mut reader = BufReader::new(stream.try_clone().expect("clone the accepted stream"));
    let mut head = String::new();
    let mut length = 0usize;
    loop {
        let mut line = String::new();
        if reader.read_line(&mut line).unwrap_or(0) == 0 {
            break;
        }
        if let Some(rest) = line.to_ascii_lowercase().strip_prefix("content-length:") {
            length = rest.trim().parse().unwrap_or(0);
        }
        let blank = line == "\r\n" || line == "\n";
        head.push_str(&line);
        if blank {
            break;
        }
    }
    let mut body = vec![0u8; length];
    if length > 0 {
        let _ = reader.read_exact(&mut body);
    }
    format!("{head}{}", String::from_utf8_lossy(&body))
}

fn loopback(replies: Vec<(u16, String)>) -> Loopback {
    let listener = TcpListener::bind("127.0.0.1:0").expect("bind the loopback listener");
    let port = listener.local_addr().expect("the listener has an address").port();
    let handle = std::thread::spawn(move || {
        let mut seen = Vec::new();
        for (code, body) in replies {
            let Ok((mut stream, _)) = listener.accept() else { break };
            seen.push(read_request(&stream));
            let response = format!(
                "HTTP/1.1 {code} X\r\ncontent-type: application/json\r\ncontent-length: {}\r\nconnection: close\r\n\r\n{body}",
                body.len()
            );
            let _ = stream.write_all(response.as_bytes());
            let _ = stream.flush();
        }
        seen
    });
    Loopback { base: format!("http://127.0.0.1:{port}/functions/v1"), handle: Some(handle) }
}

fn cfg(base: &str) -> CloudConfig {
    CloudConfig {
        api_base: base.to_string(),
        anon_key: "anon-not-a-secret".to_string(),
        session_credential_target: "knowlu/test-profile/session".to_string(),
        account_id: "acct-1".to_string(),
    }
}

// ---------------------------------------------------------------------------
// A private, seeded copy of `tests/fixtures/vault-s1`.
// ---------------------------------------------------------------------------

fn copy_vault(src: &Path, dst: &Path) -> std::io::Result<()> {
    std::fs::create_dir_all(dst)?;
    for entry in std::fs::read_dir(src)? {
        let entry = entry?;
        let target = dst.join(entry.file_name());
        if entry.file_type()?.is_dir() {
            copy_vault(&entry.path(), &target)?;
        } else {
            std::fs::copy(entry.path(), &target)?;
        }
    }
    Ok(())
}

/// A private copy of `tests/fixtures/vault-s1`, in a temp directory named for the calling test, with
/// one journal record already in it.
///
/// **Deviation from the brief's literal comment.** The frozen fixture itself carries no
/// `state/journal/` — the oracle it exists for never needed one, and CLAUDE.md's "never regenerate
/// a frozen reference" rule means this suite must not add one to `tests/fixtures/vault-s1` itself.
/// So this helper copies the fixture's notes into a scratch directory and then appends one ordinary
/// `quinn`/`dashboard` record to that COPY's journal, which is what
/// `a_first_push_carries_every_record_and_every_note_once` means by "the fixture has a journal".
fn fixture(name: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!("knowlu-sync-{name}-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    copy_vault(Path::new("tests/fixtures/vault-s1"), &dir).expect("copy fixture vault");
    let mut journal = Journal::new(&dir);
    let mut spec =
        knowlu_engine::journal::NewRecord::new("set", "tasks/cs-100-hw-01.md", "quinn", "dashboard");
    spec.field = Some("importance");
    spec.old = serde_json::json!(3);
    spec.new = serde_json::json!(4);
    spec.ts = Some("2026-08-20T12:00:00.000Z".to_string());
    spec.device = Some("TestPC".to_string());
    let mut rec = knowlu_engine::journal::make_record(spec).expect("a seed record");
    journal.append(&mut rec).expect("seed the fixture's journal");
    dir
}

// ---------------------------------------------------------------------------
// The push builder.
// ---------------------------------------------------------------------------

#[test]
fn a_first_push_carries_every_record_and_every_note_once() {
    let dir = fixture("first-push");
    let mut journal = Journal::new(&dir);
    let (batch, next) = sync::build_push(&dir, &Cursor::default(), "acct-1", &mut journal);
    assert!(batch.warnings.is_empty(), "{:?}", batch.warnings);
    assert_eq!(batch.device.len(), 16);
    assert_eq!(batch.notes.len(), sync::note_paths(&dir).len(), "every note on disk");
    assert!(!batch.records.is_empty(), "the fixture has a journal");
    // The SECOND build with the cursor the first returned sends nothing.
    let (again, _) = sync::build_push(&dir, &next, "acct-1", &mut journal);
    assert_eq!((again.records.len(), again.notes.len()), (0, 0), "a push is idempotent against its own cursor");
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn a_records_hash_is_the_sha256_of_the_canonical_bytes_the_server_will_see() {
    // The server re-derives this and refuses a mismatch (Task 3). The bytes it hashes are the ones
    // in `body`, so the two computations agree only if `body` IS `dumps_value`'s output — which is
    // why nothing here ever reaches for `serde_json::to_string`.
    let dir = fixture("hash");
    let mut journal = Journal::new(&dir);
    let (batch, _) = sync::build_push(&dir, &Cursor::default(), "acct-1", &mut journal);
    assert!(!batch.records.is_empty(), "the fixture has a journal");
    for row in &batch.records {
        let body = row["body"].as_str().expect("a body");
        assert_eq!(row["hash"].as_str().expect("a hash"), sync::sha256_hex(body.as_bytes()));
        let parsed: serde_json::Value = serde_json::from_str(body).expect("a record parses");
        assert_eq!(knowlu_engine::ledger::dumps_value(&parsed), body, "the body is canonical, not re-serialised");
    }
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn a_note_that_has_not_changed_is_not_pushed_again_and_one_that_moved_is_a_tombstone() {
    let dir = fixture("tombstone");
    let mut journal = Journal::new(&dir);
    let (_, cursor) = sync::build_push(&dir, &Cursor::default(), "acct-1", &mut journal);
    let gone = sync::note_paths(&dir).into_iter().next().expect("a note");
    std::fs::remove_file(dir.join(&gone)).expect("remove one note");
    let (batch, _) = sync::build_push(&dir, &cursor, "acct-1", &mut journal);
    assert_eq!(batch.notes.len(), 1, "{:?}", batch.notes);
    assert_eq!(batch.notes[0]["path"].as_str(), Some(gone.as_str()));
    assert_eq!(batch.notes[0]["deleted"], serde_json::Value::Bool(true));
    assert!(batch.notes[0].get("body").is_none(), "a tombstone carries no bytes");
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn a_record_or_a_note_over_the_cap_is_named_and_left_behind_never_truncated() {
    let dir = fixture("oversize");
    let big = format!("---\nid: task_0000000001\n---\n{}", "x".repeat(sync::MAX_NOTE_BYTES));
    knowlu_engine::pystr::write_text(&dir.join("tasks").join("huge.md"), &big).expect("write");
    let mut journal = Journal::new(&dir);
    let (batch, _) = sync::build_push(&dir, &Cursor::default(), "acct-1", &mut journal);
    assert!(!batch.notes.iter().any(|n| n["path"] == "tasks/huge.md"), "the oversize note is not sent");
    assert!(batch.warnings.iter().any(|w| w.contains("tasks/huge.md") && w.contains("too large")), "{:?}", batch.warnings);
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn a_record_this_devices_own_sync_wrote_is_never_pushed_back() {
    // **The echo** (review S1, ruling R-C3′-plan-2). Everything `apply` carries out goes through a
    // `write::` function under `sync::ACTOR`, and every one of those journals a second, locally
    // authored record beside the foreign one it mirrors — with a fresh `ts` and this machine's own
    // `device_name()`, so it hashes new and the content filter cannot see it. Pushed, it would reach
    // every other desktop as an event that had never happened there, credited to the sync rather
    // than to whoever acted. **The human record and the verbatim foreign record still go up**: the
    // first is this device's own news, and the second costs nothing because `/sync-push` re-derives
    // its hash and upserts on `sync_records_once`, so it lands on the row it already is.
    let dir = fixture("no-echo");
    let mut journal = Journal::new(&dir);

    // 1. A human write on this machine.
    let mine = knowlu_engine::write::WriteContext::new("quinn", "dashboard");
    knowlu_engine::write::write_literals(&dir, "tasks/cs-100-hw-01.md", &[("importance".to_string(), "4".to_string())], &mine, &mut journal, &Default::default()).expect("my edit");

    // 2. A foreign record, appended verbatim exactly as `apply` appends one.
    let mut spec = knowlu_engine::journal::NewRecord::new("set", "tasks/cs-100-hw-01.md", "quinn", "dashboard");
    spec.id = Some("task_0000000001");
    spec.field = Some("effort");
    spec.old = serde_json::json!(1.0);
    spec.new = serde_json::json!(2.0);
    spec.ts = Some("2026-09-17T10:00:00.000Z".to_string());
    spec.device = Some("OtherDesktop".to_string());
    let mut foreign = knowlu_engine::journal::make_record(spec).expect("a record");
    knowlu_engine::ledger::JsonlLedger::new(dir.join("state").join("journal")).append(&foreign).expect("verbatim");
    let _ = &mut foreign;

    // 3. The echo `apply` leaves when it mirrors that foreign change.
    let echo = knowlu_engine::write::WriteContext::new(sync::ACTOR, "local-runner");
    knowlu_engine::write::write_literals(&dir, "tasks/cs-100-hw-01.md", &[("effort".to_string(), "2.0".to_string())], &echo, &mut journal, &Default::default()).expect("the mirror");

    journal.invalidate();
    let (batch, _) = sync::build_push(&dir, &Cursor::default(), "acct-1", &mut journal);
    let actors: Vec<String> = batch.records.iter()
        .map(|r| serde_json::from_str::<serde_json::Value>(r["body"].as_str().expect("a body")).expect("a record"))
        .map(|v| v["actor"].as_str().unwrap_or_default().to_string())
        .collect();
    assert!(!actors.iter().any(|a| a == sync::ACTOR), "an echo went up: {actors:?}");
    // The fixture's own seed record (`quinn`/`dashboard`) plus this test's human write and the
    // foreign record: three, not two — `fixture()` no longer hands back an empty journal.
    assert_eq!(actors.len(), 3, "the seed, the human write and the foreign record, and nothing else: {actors:?}");
    let _ = std::fs::remove_dir_all(&dir);
}

// The brief names this test after the `PAGE` constant; `#[allow]` keeps that name under the
// workspace's 0-warnings gate (CLAUDE.md) rather than lower-casing it into something less exact.
#[allow(non_snake_case)]
#[test]
fn a_page_is_at_most_PAGE_and_the_cursor_does_not_skip_the_rest() {
    // A term of catching up must not be one enormous request, and it must not silently drop the
    // tail either: the cursor stops exactly where the page stopped.
    let dir = fixture("page");
    let mut journal = Journal::new(&dir);
    for n in 0..(sync::PAGE + 7) {
        let mut spec = knowlu_engine::journal::NewRecord::new("set", "tasks/x.md", "quinn", "dashboard");
        spec.field = Some("importance");
        spec.new = serde_json::json!(n as i64 % 5);
        let mut rec = knowlu_engine::journal::make_record(spec).expect("a record");
        journal.append(&mut rec).expect("append");
    }
    journal.invalidate();
    let (batch, next) = sync::build_push(&dir, &Cursor::default(), "acct-1", &mut journal);
    assert_eq!(batch.records.len(), sync::PAGE);
    let (rest, _) = sync::build_push(&dir, &next, "acct-1", &mut journal);
    // M4: the exact remainder, not just "non-empty" — 1 seed record (from `fixture()`) plus
    // `PAGE + 7` loop records, minus the `PAGE` already sent, and this also proves the same-`ts`
    // boundary survives a page break: the tight loop above shares milliseconds across many records.
    assert_eq!(rest.records.len(), 8, "{:?}", rest.records.len());
    let _ = std::fs::remove_dir_all(&dir);
}

/// M4 (Minor 4): `save_cursor`/`load_cursor` had no test of their own.
#[test]
fn a_saved_cursor_loads_back_equal_to_itself() {
    let dir = fixture("cursor-roundtrip");
    let mut cursor = Cursor::default();
    cursor.record_cursor = 3;
    cursor.note_cursor = 7;
    cursor.pushed_through = "2026-09-20T00:00:00.000Z".to_string();
    cursor.boundary = vec![sync::sha256_hex(b"a"), sync::sha256_hex(b"b")];
    cursor.notes.insert("tasks/x.md".to_string(), sync::sha256_hex(b"x"));
    sync::save_cursor(&dir, &cursor).expect("save the cursor");
    assert_eq!(sync::load_cursor(&dir), cursor, "a saved cursor must load back byte-for-byte equal");

    // A vault that has never synced loads back a fresh default, not an error.
    let empty_dir = fixture("cursor-missing");
    assert_eq!(sync::load_cursor(&empty_dir), Cursor::default());

    let _ = std::fs::remove_dir_all(&dir);
    let _ = std::fs::remove_dir_all(&empty_dir);
}

#[test]
fn a_push_carries_no_bearer_no_credential_target_and_no_app_data_path() {
    // The data-minimisation assertion, kept from the sealed design because it is still true and
    // still worth failing on. **It does not assert the hostname** (review M6): every journal record
    // carries `device` inside its own `body`, where the student can read it, and that is the design —
    // what this checks is that nothing about the SESSION or this machine's app data travels beside
    // it. The `device` COLUMN is the opaque token, and `a_device_token_is_sixteen_hex_stable_and_not_the_hostname`
    // (in `engine/src/sync.rs`'s own unit tests) is where that is proved.
    let dir = fixture("minimal");
    let mut journal = Journal::new(&dir);
    let (batch, _) = sync::build_push(&dir, &Cursor::default(), "acct-1", &mut journal);
    let wire = knowlu_engine::ledger::dumps_value(&serde_json::json!({
        "device": batch.device, "records": batch.records, "notes": batch.notes
    }));
    for forbidden in ["Bearer", "access_token", "LOCALAPPDATA", "AppData", "knowlu/", "sync-key"] {
        assert!(!wire.contains(forbidden), "the wire carries {forbidden}");
    }
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn the_push_posts_to_sync_push_and_reads_the_two_counts() {
    let mut server = loopback(vec![(200, r#"{"records":2,"notes":1,"bytes_used":10,"bytes_ceiling":20}"#.to_string())]);
    let client = CloudClient::new(&cfg(&server.base), "jwt-not-a-secret");
    let batch = sync::PushBatch { device: "0123456789abcdef".into(), records: vec![], notes: vec![], warnings: vec![] };
    assert_eq!(sync::push(&client, &batch).expect("a 200"), (2, 1));
    let sent = server.requests();
    // Deviation from the brief's literal string: the loopback's `base` (reused from C2's own
    // harness) already carries `/functions/v1`, exactly as `cloud_contract.rs`'s own
    // `judge-task` test expects `POST /functions/v1/judge-task HTTP/1.1` rather than `POST
    // /judge-task `. `CloudClient::post` prepends that base to every path, so this request line
    // carries the same prefix.
    assert!(sent[0].starts_with("POST /functions/v1/sync-push "), "{}", sent[0]);
    // Lower case: `ureq` writes header names as given and C2's own contract test learned this the
    // hard way — do not "fix" it back to `Authorization`.
    assert!(sent[0].contains("authorization: Bearer jwt-not-a-secret"), "{}", sent[0]);
}

/// R-C3′-exec-7: the account's size ceiling answers 403, not 413. A 413 now means only "the whole
/// request body was over the server's own transport cap" — a different failure, and not one this
/// test scripts, because `push` sends whatever `build_push` gave it without measuring it again.
#[test]
fn a_403_is_the_named_ceiling_refusal() {
    let mut server = loopback(vec![(403, r#"{"error":"this account's copy is at its size limit"}"#.to_string())]);
    let client = CloudClient::new(&cfg(&server.base), "jwt-not-a-secret");
    let batch = sync::PushBatch { device: "0123456789abcdef".into(), records: vec![], notes: vec![], warnings: vec![] };
    match sync::push(&client, &batch) {
        Err(knowlu_engine::cloudmodel::CloudError::Status { code, .. }) => assert_eq!(code, 403),
        other => panic!("{other:?}"),
    }
    let _ = server.requests();
}

// ---------------------------------------------------------------------------
// R-C3′-exec-11: a byte budget per page.
// ---------------------------------------------------------------------------

#[test]
fn a_push_over_the_byte_budget_is_split_across_two_build_push_calls_and_nothing_is_skipped() {
    let dir = fixture("budget");
    // Each note is comfortably under `MAX_NOTE_BYTES` on its own (120,000 of 131,072), but thirty of
    // them together are well past `PUSH_BUDGET_BYTES` (3,145,728), so one call cannot carry them all.
    let body = "x".repeat(120_000);
    for n in 0..30 {
        knowlu_engine::pystr::write_text(&dir.join("tasks").join(format!("big-{n:02}.md")), &body).expect("write");
    }
    let mut journal = Journal::new(&dir);
    let (first, next) = sync::build_push(&dir, &Cursor::default(), "acct-1", &mut journal);
    assert!(first.warnings.is_empty(), "{:?}", first.warnings);

    let sent: usize = first.records.iter().chain(first.notes.iter())
        .map(knowlu_engine::ledger::dumps_value)
        .map(|s| s.len())
        .sum();
    assert!(sent <= sync::PUSH_BUDGET_BYTES, "the first batch is {sent} bytes");

    let all = sync::note_paths(&dir);
    assert!(first.notes.len() < all.len(), "the folder did not fit in one page: {} of {}", first.notes.len(), all.len());

    let (second, _) = sync::build_push(&dir, &next, "acct-1", &mut journal);
    assert!(!second.notes.is_empty(), "the tail is still owed");

    // Nothing was skipped: together the two pushes carry every note in the folder exactly once.
    let mut carried: Vec<String> = first.notes.iter().chain(second.notes.iter())
        .map(|row| row["path"].as_str().expect("a path").to_string())
        .collect();
    carried.sort();
    let mut expected = all;
    expected.sort();
    assert_eq!(carried, expected, "the union of both batches must be every note, once each");
    let _ = std::fs::remove_dir_all(&dir);
}

// ---------------------------------------------------------------------------
// R-C3′-exec-8/11: the device applies the server's exact path rule.
// ---------------------------------------------------------------------------

#[test]
fn is_note_path_and_the_servers_regex_agree() {
    // The rule lives in three places by necessity — Rust, TypeScript and a column check — so it is
    // pinned from one side rather than trusted three times. The TS source is read, not imported.
    let ts = std::fs::read_to_string(
        std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("..").join("cloud").join("supabase")
            .join("functions").join("_shared").join("sync_rows.ts"),
    ).expect("sync_rows.ts");
    assert!(ts.contains("(tasks|approvals|archive|courses|issues|info)"), "the server's folder list is the engine's");
    assert!(ts.contains("[A-Za-z0-9._ /-]{1,300}"), "the server's character class and length are the engine's");
    let vault = std::env::temp_dir();
    for ok in ["tasks/x.md", "courses/cs-100.md", "info/a.md"] { assert!(sync::is_note_path(&vault, ok), "{ok}"); }
    for bad in ["state/journal/2026-09-17.jsonl", "config/ingest.yaml", "tasks/../../x.md", "tasks//x.md",
                "tasks/x.txt", "tasks\\x.md", "/tasks/x.md", "tasks", ""] {
        assert!(!sync::is_note_path(&vault, bad), "{bad}");
    }
    // The middle's own length bound: 300 characters accepted, 301 refused.
    let ok300 = format!("tasks/{}.md", "a".repeat(300));
    assert!(sync::is_note_path(&vault, &ok300), "a 300-character middle must be accepted");
    let bad301 = format!("tasks/{}.md", "a".repeat(301));
    assert!(!sync::is_note_path(&vault, &bad301), "a 301-character middle must be refused");
    // Characters outside the class: a parenthesis, and a non-ASCII letter.
    for bad in ["tasks/Essay (draft).md", "courses/español.md"] {
        assert!(!sync::is_note_path(&vault, bad), "{bad}");
    }
}

#[test]
fn a_note_with_a_name_the_account_cannot_store_gets_a_warning_and_no_row() {
    let dir = fixture("badname");
    knowlu_engine::pystr::write_text(&dir.join("tasks").join("Essay (draft).md"), "body text").expect("write");
    let mut journal = Journal::new(&dir);
    let (batch, _) = sync::build_push(&dir, &Cursor::default(), "acct-1", &mut journal);
    assert!(!batch.notes.iter().any(|n| n["path"] == "tasks/Essay (draft).md"), "the badly named note is not sent");
    assert!(
        batch.warnings.iter().any(|w| w.contains("Essay (draft).md") && w.contains("cannot store")),
        "{:?}", batch.warnings
    );
    let _ = std::fs::remove_dir_all(&dir);
}

// ---------------------------------------------------------------------------
// Fix round 1 — R-C3′-exec-12, the wedge principle: the device predicts every refusal the server
// makes of a row and skips that row with one named warning, never sending it.
// ---------------------------------------------------------------------------

/// Important 1: a budget `break` in the notes loop must not read as "everything after this point in
/// the listing is gone" — the tombstone pass has to check the COMPLETE on-disk listing, never the
/// loop's own visited set, or a page that breaks mid-folder marks live notes as deleted.
#[test]
fn a_budget_break_in_the_notes_loop_never_tombstones_a_note_still_on_disk() {
    let dir = fixture("i1-tombstone-budget");
    let mut journal = Journal::new(&dir);
    let (_, cursor1) = sync::build_push(&dir, &Cursor::default(), "acct-1", &mut journal);
    let tracked_before: std::collections::BTreeSet<String> = cursor1.notes.keys().cloned().collect();
    assert!(!tracked_before.is_empty(), "the fixture's first push must have tracked its notes");

    // Grow two of the fixture's own notes, and add enough large filler notes — sorting alphabetically
    // BEFORE every one of the fixture's own `tasks/` files — that the folder's total is well past
    // `PUSH_BUDGET_BYTES` before the loop ever reaches an already-tracked note again.
    let grown = "x".repeat(120_000);
    for name in ["done-already.md", "task-erste-reflexion.md"] {
        knowlu_engine::pystr::write_text(&dir.join("tasks").join(name), &grown).expect("grow a note");
    }
    for n in 0..28 {
        knowlu_engine::pystr::write_text(&dir.join("tasks").join(format!("aaa-{n:02}.md")), &grown).expect("write filler");
    }

    let (batch2, cursor2) = sync::build_push(&dir, &cursor1, "acct-1", &mut journal);
    let on_disk: std::collections::BTreeSet<String> = sync::note_paths(&dir).into_iter().collect();
    let tombstoned: Vec<&str> = batch2.notes.iter()
        .filter(|n| n.get("deleted") == Some(&serde_json::Value::Bool(true)))
        .filter_map(|n| n["path"].as_str())
        .collect();
    for path in &tombstoned {
        assert!(!on_disk.contains(*path), "a note still on disk must never be tombstoned: {path}");
    }
    for path in &tracked_before {
        if on_disk.contains(path) {
            assert!(!tombstoned.contains(&path.as_str()), "{path} is on disk and must not be a tombstone");
        }
    }

    // A third call from the second cursor still owes whatever the budget break left behind.
    let (batch3, _) = sync::build_push(&dir, &cursor2, "acct-1", &mut journal);
    assert!(!batch3.notes.is_empty(), "the tail is still owed after the budget break");
    let _ = std::fs::remove_dir_all(&dir);
}

/// Important 2: the server refuses an empty body (`sync_rows.ts:111`, "a note has no body") and a
/// raw NUL byte (`sync_rows.ts:114`) — the device predicts both and never sends either.
#[test]
fn a_note_that_is_empty_or_has_a_null_byte_gets_a_warning_and_the_rest_of_the_batch_still_goes() {
    let dir = fixture("empty-or-null");
    knowlu_engine::pystr::write_text(&dir.join("tasks").join("empty.md"), "").expect("write empty");
    knowlu_engine::pystr::write_text(&dir.join("tasks").join("nulled.md"), "before\u{0}after").expect("write a null byte");
    let mut journal = Journal::new(&dir);
    let (batch, _) = sync::build_push(&dir, &Cursor::default(), "acct-1", &mut journal);
    for bad in ["tasks/empty.md", "tasks/nulled.md"] {
        assert!(!batch.notes.iter().any(|n| n["path"] == bad), "{bad} must not be sent");
        assert!(batch.warnings.iter().any(|w| w.contains(bad)), "{:?}", batch.warnings);
    }
    // Everything else in the fixture still went up.
    assert_eq!(batch.notes.len(), sync::note_paths(&dir).len() - 2, "{:?}", batch.notes);
    let _ = std::fs::remove_dir_all(&dir);
}

/// Minor 3 (records half): the server refuses a record whose body has no non-empty string `op` or
/// `actor` (`sync_rows.ts:89-93`); `ledger::read` itself requires only `ts`, so a hand-edited or
/// legacy journal line can carry neither. Treated exactly like an oversize record: skipped, so it
/// never advances the cursor and never wedges a page that also carries good records.
#[test]
fn a_record_missing_a_usable_op_or_actor_is_never_sent() {
    let dir = fixture("badrecord");
    let mut rec = serde_json::Map::new();
    rec.insert("ts".to_string(), serde_json::json!("2026-09-20T00:00:00.000Z"));
    rec.insert("device".to_string(), serde_json::json!("TestPC"));
    rec.insert("actor".to_string(), serde_json::json!("")); // empty, and no "op" at all
    rec.insert("path".to_string(), serde_json::json!("tasks/legacy-marker.md"));
    knowlu_engine::ledger::JsonlLedger::new(dir.join("state").join("journal"))
        .append(&rec)
        .expect("append a legacy line directly, bypassing make_record's validation");

    let mut journal = Journal::new(&dir);
    let (batch, _) = sync::build_push(&dir, &Cursor::default(), "acct-1", &mut journal);
    let sent_the_marker = batch.records.iter().any(|r| {
        r["body"].as_str().unwrap_or_default().contains("legacy-marker.md")
    });
    assert!(!sent_the_marker, "a record with no usable op/actor must never be sent: {:?}", batch.records);
    assert!(
        batch.warnings.iter().any(|w| w.contains("op") || w.contains("actor")),
        "{:?}", batch.warnings
    );
    let _ = std::fs::remove_dir_all(&dir);
}

/// Minor 3 (notes half): the tombstone loop must also run `is_note_path` on `cursor.notes` keys — a
/// hand-edited cursor could carry a path the server's own `NOTE_PATH_RE`/column check would refuse,
/// and without this the device would resend a 400-producing tombstone every single run.
#[test]
fn a_tombstone_whose_path_fails_is_note_path_is_dropped_not_sent() {
    let dir = fixture("bad-cursor-path");
    let mut cursor = Cursor::default();
    cursor.notes.insert("tasks/Bad Name!.md".to_string(), sync::sha256_hex(b"whatever"));
    let mut journal = Journal::new(&dir);
    let (batch, next) = sync::build_push(&dir, &cursor, "acct-1", &mut journal);
    assert!(
        !batch.notes.iter().any(|n| n["path"] == "tasks/Bad Name!.md"),
        "an unsendable tombstone path must never be sent: {:?}", batch.notes
    );
    assert!(
        !next.notes.contains_key("tasks/Bad Name!.md"),
        "it must be dropped from the cursor, not retried forever"
    );
    assert!(batch.warnings.iter().any(|w| w.contains("Bad Name!.md")), "{:?}", batch.warnings);
    let _ = std::fs::remove_dir_all(&dir);
}

/// M2: a folder whose listing fails for a reason OTHER than "it does not exist" must not read as
/// "this folder is now empty" — that would tombstone every note the cursor remembers there. Standing
/// a plain file in place of `approvals/` is a deterministic way to make `read_dir` fail with
/// something other than `NotFound` on Windows, entirely inside this test's own scratch vault and
/// without touching permissions of anything (this repo's CLAUDE.md rules out touching ACLs in tests).
#[test]
fn an_unreadable_folder_suppresses_tombstones_for_that_folder_only() {
    let dir = fixture("unreadable-folder");
    let mut journal = Journal::new(&dir);
    let (_, cursor) = sync::build_push(&dir, &Cursor::default(), "acct-1", &mut journal);
    let approvals_known: Vec<String> =
        cursor.notes.keys().filter(|p| p.starts_with("approvals/")).cloned().collect();
    assert!(!approvals_known.is_empty(), "the fixture ships approvals notes");

    std::fs::remove_dir_all(dir.join("approvals")).expect("clear the real folder");
    knowlu_engine::pystr::write_text(&dir.join("approvals"), "not a directory any more").expect("stand a file in its place");

    let (batch, _) = sync::build_push(&dir, &cursor, "acct-1", &mut journal);
    for known in &approvals_known {
        assert!(
            !batch.notes.iter().any(|n| n["path"] == known.as_str()),
            "{known} must not be tombstoned when its folder cannot be listed: {:?}", batch.notes
        );
    }
    let _ = std::fs::remove_dir_all(&dir);
}

// ---------------------------------------------------------------------------
// Task 6: the pull, `reconcile` with the roles reversed, and the amend card.
// ---------------------------------------------------------------------------

/// A foreign `set` record: another desktop, another `seq`, a chosen `ts`.
fn foreign_set(id: &str, path: &str, field: &str, old: serde_json::Value, new: serde_json::Value, ts: &str) -> knowlu_engine::ledger::Record {
    let mut spec = knowlu_engine::journal::NewRecord::new("set", path, "quinn", "dashboard");
    spec.id = Some(id);
    spec.field = Some(field);
    spec.old = old;
    spec.new = new;
    spec.ts = Some(ts.to_string());
    spec.device = Some("OtherDesktop".to_string());
    knowlu_engine::journal::make_record(spec).expect("a record")
}

fn pulled(records: Vec<knowlu_engine::ledger::Record>, notes: Vec<sync::PulledNote>) -> sync::Pulled {
    sync::Pulled {
        records: records.into_iter().map(|r| ("fedcba9876543210".to_string(), r)).collect(),
        notes, record_cursor: 1, note_cursor: 1, more: false, warnings: Vec::new(),
    }
}

/// `fixture(name)`, plus an `id:` line stamped on `tasks/cs-100-hw-01.md` — as every wizard-made
/// note carries, but this frozen fixture predates id-stamping. Stamped onto the TEMP COPY only,
/// never onto `vault-s1` itself (review I1, fix round 1): without an `id:` line, `write_literals`
/// journals this device's own edits with `id: null`, so `journal::records_for` (keyed on `id`) can
/// never find them and every conflict test falls back to `reconcile::resolve`'s synthetic "external
/// write" branch — the mtime fallback — rather than exercising the real, reversed-role comparison
/// against this device's own journalled records this task exists to get right.
fn fixture_with_id(name: &str) -> PathBuf {
    let dir = fixture(name);
    let path = dir.join("tasks").join("cs-100-hw-01.md");
    let text = knowlu_engine::pystr::read_text(&path).expect("the fixture note");
    let stamped = text.replacen("title:", "id: task_0000000001\ntitle:", 1);
    knowlu_engine::pystr::write_text(&path, &stamped).expect("stamp an id on the temp copy");
    dir
}

#[test]
fn a_field_only_the_other_desktop_touched_applies_cleanly_and_files_no_card() {
    let dir = fixture("clean");
    let mut journal = Journal::new(&dir);
    let ctx = knowlu_engine::write::WriteContext::new(sync::ACTOR, "local-runner");
    // Deviation from the brief's literal `old: 3` — the committed `vault-s1` fixture's
    // `cs-100-hw-01.md` carries `importance: 2` on disk (confirmed unchanged since the initial
    // import; this fixture also underlies the frozen `golden-today-s1.md` oracle, so its content is
    // not touched to make the test's number line up instead). `old` must equal the note's actual,
    // on-disk value for `reconcile::resolve`'s "upstream never moved" branch to fire, which is this
    // test's whole premise — so `old` is `2` here, matching the fixture as committed.
    let rec = foreign_set("task_0000000001", "tasks/cs-100-hw-01.md", "importance", serde_json::json!(2), serde_json::json!(5), "2026-09-17T10:00:00.000Z");
    let report = sync::apply(&dir, &pulled(vec![rec], vec![]), &ctx, &mut journal, "2026-09-17".parse().unwrap());
    assert_eq!((report.applied, report.cards, report.superseded), (1, 0, 0), "{report:?}");
    let meta = knowlu_engine::ids::read_meta(&dir.join("tasks").join("cs-100-hw-01.md")).expect("the note");
    assert_eq!(knowlu_engine::yaml::get(&meta, "importance").and_then(knowlu_engine::yaml::i64_of), Some(5));
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn a_foreign_record_is_appended_verbatim_with_its_own_device_and_ts() {
    // The whole reason a pull and a restore do not go through `write`: these records already exist,
    // and rewriting their `ts` or their `device` would destroy the attribution judge-once reads.
    let dir = fixture("verbatim");
    let mut journal = Journal::new(&dir);
    let ctx = knowlu_engine::write::WriteContext::new(sync::ACTOR, "local-runner");
    let rec = foreign_set("task_0000000001", "tasks/cs-100-hw-01.md", "importance", serde_json::json!(3), serde_json::json!(5), "2026-09-17T10:00:00.000Z");
    sync::apply(&dir, &pulled(vec![rec], vec![]), &ctx, &mut journal, "2026-09-17".parse().unwrap());
    journal.invalidate();
    let kept = journal.read(None, None).into_iter()
        .find(|r| r.get("device").and_then(|v| v.as_str()) == Some("OtherDesktop")).expect("the foreign record");
    assert_eq!(kept.get("ts").and_then(|v| v.as_str()), Some("2026-09-17T10:00:00.000Z"));
    assert_eq!(kept.get("actor").and_then(|v| v.as_str()), Some("quinn"));
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn a_record_this_device_already_has_is_not_applied_twice() {
    let dir = fixture("dedupe");
    let mut journal = Journal::new(&dir);
    let ctx = knowlu_engine::write::WriteContext::new(sync::ACTOR, "local-runner");
    let rec = foreign_set("task_0000000001", "tasks/cs-100-hw-01.md", "importance", serde_json::json!(3), serde_json::json!(5), "2026-09-17T10:00:00.000Z");
    let first = sync::apply(&dir, &pulled(vec![rec.clone()], vec![]), &ctx, &mut journal, "2026-09-17".parse().unwrap());
    assert_eq!(first.records, 1);
    let second = sync::apply(&dir, &pulled(vec![rec], vec![]), &ctx, &mut journal, "2026-09-17".parse().unwrap());
    assert_eq!((second.records, second.applied, second.cards), (0, 0, 0), "{second:?}");
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn a_field_both_desktops_moved_becomes_one_amend_card_and_not_a_silent_merge() {
    // VISION, and §5.5's own sentence. The card is the ordinary `kind: amend` card the deck already
    // answers, through the ordinary `propose_amendment` path, so the fifteen-a-day cap applies.
    // `fixture_with_id`, not the plain `fixture` (review I1, fix round 1): with a real `id:` on the
    // note, this device's own edit below is found by `journal::records_for` and the conflict below
    // is resolved against it directly, rather than falling back to the mtime-based synthetic
    // contender every other test that used the id-less fixture was quietly exercising instead.
    let dir = fixture_with_id("conflict");
    let mut journal = Journal::new(&dir);
    let mine = knowlu_engine::write::WriteContext::new("quinn", "dashboard");
    knowlu_engine::write::write_literals(&dir, "tasks/cs-100-hw-01.md", &[("importance".to_string(), "4".to_string())], &mine, &mut journal, &Default::default()).expect("my edit");
    let ctx = knowlu_engine::write::WriteContext::new(sync::ACTOR, "local-runner");
    let rec = foreign_set("task_0000000001", "tasks/cs-100-hw-01.md", "importance", serde_json::json!(3), serde_json::json!(5), "2036-09-17T10:00:00.000Z");
    let report = sync::apply(&dir, &pulled(vec![rec], vec![]), &ctx, &mut journal, "2026-09-17".parse().unwrap());
    assert_eq!(report.cards, 1, "{report:?}");
    // **The carded field is withheld from `apply`** (review C2): the card is the write, so nothing
    // counts as applied and the note below still holds this device's value. Without this assertion
    // the note could hold 5, the card's `from: 4` would no longer match it, and
    // `approvals::validate_amendment` would refuse the card for ever — a silent merge with an
    // unapplyable card on top.
    assert_eq!(report.applied, 0, "a carded field is not also written: {report:?}");
    // The note keeps what this device had until the card is answered: never a silent merge.
    let meta = knowlu_engine::ids::read_meta(&dir.join("tasks").join("cs-100-hw-01.md")).expect("the note");
    assert_eq!(knowlu_engine::yaml::get(&meta, "importance").and_then(knowlu_engine::yaml::i64_of), Some(4));
    // `amend-cs-100-hw-01-`, not just `amend-`: the fixture vault already ships an unrelated
    // `amend-ph-106-due.md` card, and matching on the target note's own stem is what keeps this
    // assertion (and the recount below, after the M3 fix files a second card) pointed at the right
    // one regardless of directory read order.
    let card = std::fs::read_dir(dir.join("approvals")).expect("approvals").flatten().map(|e| e.path())
        .find(|p| p.file_name().map(|n| n.to_string_lossy().starts_with("amend-cs-100-hw-01-")).unwrap_or(false)).expect("one amend card");
    let text = knowlu_engine::pystr::read_text(&card).expect("the card");
    assert!(text.contains("kind: amend") && text.contains("from: 4") && text.contains("to: 5"), "{text}");
    // A second pull with a NEWER conflicting value for the same field set is not an identical
    // re-proposal (review M3, fix round 1): the stale card, which still offers `to: 5`, is settled
    // (archived, never deleted outright) and a fresh one filed with `to: 6` — approving the old card
    // would otherwise write a value the other desktop has already moved past.
    let again = foreign_set("task_0000000001", "tasks/cs-100-hw-01.md", "importance", serde_json::json!(3), serde_json::json!(6), "2036-09-17T11:00:00.000Z");
    let second = sync::apply(&dir, &pulled(vec![again], vec![]), &ctx, &mut journal, "2026-09-17".parse().unwrap());
    assert_eq!(second.cards, 1, "a newer conflicting value replaces the stale card, it does not sit beside it: {second:?}");
    let cards: Vec<PathBuf> = std::fs::read_dir(dir.join("approvals")).expect("approvals").flatten().map(|e| e.path())
        .filter(|p| p.file_name().map(|n| n.to_string_lossy().starts_with("amend-cs-100-hw-01-")).unwrap_or(false)).collect();
    assert_eq!(cards.len(), 1, "the stale card is settled, not left beside the fresh one: {cards:?}");
    let fresh_text = knowlu_engine::pystr::read_text(&cards[0]).expect("the fresh card");
    assert!(fresh_text.contains("to: 6") && !fresh_text.contains("to: 5"), "{fresh_text}");
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn a_pulled_record_that_is_not_a_record_never_reaches_the_ledger() {
    // Review I7, inherited. The rows are plaintext now, which makes this MORE important rather than
    // less: a malformed record used to fail to decrypt, and now it arrives looking like data.
    let dir = fixture("malformed");
    let mut journal = Journal::new(&dir);
    let ctx = knowlu_engine::write::WriteContext::new(sync::ACTOR, "local-runner");
    let good = foreign_set("task_0000000001", "tasks/cs-100-hw-01.md", "importance", serde_json::json!(3), serde_json::json!(5), "2026-09-17T10:00:00.000Z");
    let mut bad = Vec::new();
    for (key, value) in [
        ("op", serde_json::json!("rm -rf")), ("via", serde_json::json!("telepathy")),
        ("ts", serde_json::json!("last Tuesday")), ("actor", serde_json::Value::Null),
        ("id", serde_json::json!("../../etc/hosts")), ("path", serde_json::json!("config/cloud.yaml")),
    ] {
        let mut r = good.clone();
        r.insert(key.to_string(), value);
        bad.push(r);
    }
    let mut all = bad.clone();
    all.push(good);
    let report = sync::apply(&dir, &pulled(all, vec![]), &ctx, &mut journal, "2026-09-17".parse().unwrap());
    assert_eq!(report.refused, bad.len(), "{report:?}");
    assert_eq!(report.records, 1, "the good one still lands");
    journal.invalidate();
    let text = knowlu_engine::ledger::dumps_value(&serde_json::json!(journal.read(None, None)));
    assert!(!text.contains("rm -rf") && !text.contains("telepathy"), "a refused record reached the ledger");
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn a_pulled_note_for_a_path_this_device_has_never_seen_arrives_whole() {
    // The second-desktop case: a note created on the other machine has no local file to reconcile
    // against, so its text is written as it stands — and a path outside the six folders is refused
    // before anything is written.
    let dir = fixture("newnote");
    let mut journal = Journal::new(&dir);
    let ctx = knowlu_engine::write::WriteContext::new(sync::ACTOR, "local-runner");
    let body = "---\nid: task_0000009999\nstatus: open\n---\n\nFrom the other desktop.\n";
    let report = sync::apply(&dir, &pulled(vec![], vec![
        sync::PulledNote { device: "fedcba9876543210".into(), path: "tasks/from-elsewhere.md".into(), text: Some(body.to_string()) },
        sync::PulledNote { device: "fedcba9876543210".into(), path: "config/cloud.yaml".into(), text: Some("api_base: evil\n".into()) },
    ]), &ctx, &mut journal, "2026-09-17".parse().unwrap());
    assert_eq!(report.notes_written, 1, "{report:?}");
    assert_eq!(report.refused, 1);
    assert_eq!(knowlu_engine::pystr::read_text(&dir.join("tasks").join("from-elsewhere.md")).expect("the note"), body);
    assert!(!knowlu_engine::pystr::read_text(&dir.join("config").join("cloud.yaml")).unwrap_or_default().contains("evil"));
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn a_pulled_note_never_overwrites_a_note_this_device_already_has() {
    // **The bound on the second recorded exception** (review I4). A pulled note's text is written
    // with `pystr::write_text` only for a path this device has never seen. For a path it HAS, the
    // frontmatter was settled by `reconcile` a few lines earlier and the body is never merged — so
    // the pulled text is dropped and the file on disk keeps its own. Without this the other
    // desktop's body would silently replace an edit made here between two slots.
    let dir = fixture("no-clobber");
    let mut journal = Journal::new(&dir);
    let ctx = knowlu_engine::write::WriteContext::new(sync::ACTOR, "local-runner");
    let rel = "tasks/cs-100-hw-01.md";
    let mine = knowlu_engine::pystr::read_text(&dir.join(rel)).expect("the fixture note");
    let report = sync::apply(&dir, &pulled(vec![], vec![
        sync::PulledNote { device: "fedcba9876543210".into(), path: rel.into(), text: Some("---\nid: task_0000000001\n---\ntheirs\n".into()) },
    ]), &ctx, &mut journal, "2026-09-17".parse().unwrap());
    assert_eq!(report.notes_written, 0, "{report:?}");
    assert_eq!(knowlu_engine::pystr::read_text(&dir.join(rel)).expect("the note"), mine, "the local body is untouched");
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn a_conflict_this_device_won_writes_nothing_and_files_no_card() {
    // `reconcile::resolve` fills `apply` only when the side passed as `local_records` wins, and with
    // the roles reversed that side is the foreign one. A foreign write OLDER than this device's
    // therefore leaves the note alone and produces a supersede record and nothing else — which is
    // right: nothing changed here, so there is nothing to propose.
    let dir = fixture("i-won");
    let mut journal = Journal::new(&dir);
    let mine = knowlu_engine::write::WriteContext::new("quinn", "dashboard");
    knowlu_engine::write::write_literals(&dir, "tasks/cs-100-hw-01.md", &[("importance".to_string(), "4".to_string())], &mine, &mut journal, &Default::default()).expect("my edit");
    let ctx = knowlu_engine::write::WriteContext::new(sync::ACTOR, "local-runner");
    let rec = foreign_set("task_0000000001", "tasks/cs-100-hw-01.md", "importance", serde_json::json!(3), serde_json::json!(5), "2016-09-17T10:00:00.000Z");
    let report = sync::apply(&dir, &pulled(vec![rec], vec![]), &ctx, &mut journal, "2026-09-17".parse().unwrap());
    assert_eq!((report.applied, report.cards), (0, 0), "{report:?}");
    assert_eq!(report.superseded, 1, "the losing write is still on the record");
    let meta = knowlu_engine::ids::read_meta(&dir.join("tasks").join("cs-100-hw-01.md")).expect("the note");
    assert_eq!(knowlu_engine::yaml::get(&meta, "importance").and_then(knowlu_engine::yaml::i64_of), Some(4));
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn a_conflict_on_an_unamendable_field_takes_reconciles_rule_and_says_so() {
    // `approvals::validate_amendment` only ever applies a card under `AMENDABLE_FOLDERS` with a
    // field in `AMENDABLE_FIELDS`. A conflict outside that set cannot become a card that would ever
    // apply, so withholding it from `apply` would strand the value with nothing to answer: it takes
    // reconcile's own rule (later `ts` wins), keeps its supersede record, and says so in one line.
    let dir = fixture("unamendable");
    let mut journal = Journal::new(&dir);
    let mine = knowlu_engine::write::WriteContext::new("quinn", "dashboard");
    knowlu_engine::write::write_literals(&dir, "tasks/cs-100-hw-01.md", &[("notes_link".to_string(), "'mine'".to_string())], &mine, &mut journal, &Default::default()).expect("my edit");
    assert!(!knowlu_engine::approvals::AMENDABLE_FIELDS.contains(&"notes_link"), "the test's premise");
    let ctx = knowlu_engine::write::WriteContext::new(sync::ACTOR, "local-runner");
    let rec = foreign_set("task_0000000001", "tasks/cs-100-hw-01.md", "notes_link", serde_json::json!("old"), serde_json::json!("theirs"), "2036-09-17T10:00:00.000Z");
    let report = sync::apply(&dir, &pulled(vec![rec], vec![]), &ctx, &mut journal, "2026-09-17".parse().unwrap());
    assert_eq!((report.cards, report.applied, report.superseded), (0, 1, 1), "{report:?}");
    assert!(
        report.warnings.iter().any(|w| w.contains("cs-100-hw-01") && w.contains("notes_link")),
        "the losing value is named, never silently dropped: {:?}", report.warnings,
    );
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn a_pulled_tombstone_settles_the_note_rather_than_unlinking_it() {
    // "Nothing is ever unlinked" is the vault's own rule: a settled note moves to `archive/` through
    // `write::delete`, which journals the move. `fs::remove_file` would lose it and journal nothing.
    let dir = fixture("tombstone-apply");
    let mut journal = Journal::new(&dir);
    let ctx = knowlu_engine::write::WriteContext::new(sync::ACTOR, "local-runner");
    let rel = "tasks/cs-100-hw-01.md";
    assert!(dir.join(rel).exists(), "the fixture has it");
    let report = sync::apply(&dir, &pulled(vec![], vec![
        sync::PulledNote { device: "fedcba9876543210".into(), path: rel.into(), text: None },
    ]), &ctx, &mut journal, "2026-09-17".parse().unwrap());
    assert_eq!(report.moved, 1, "{report:?}");
    assert!(!dir.join(rel).exists(), "the task is gone from tasks/");
    assert!(
        std::fs::read_dir(dir.join("archive")).expect("archive").flatten()
            .any(|e| e.file_name().to_string_lossy().starts_with("cs-100-hw-01")),
        "and it is in archive/, not deleted",
    );
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn a_valid_pulled_move_relocates_the_note_and_counts_it() {
    // **The move is performed, not only journalled** (review R4, ruled). A rename that was recorded
    // and not carried out would leave the old file in place while the renamed note's text arrived at
    // the new path as a note this device had never seen — one note in two places.
    let dir = fixture("move-applied");
    let mut journal = Journal::new(&dir);
    let ctx = knowlu_engine::write::WriteContext::new(sync::ACTOR, "local-runner");
    let from = "tasks/cs-100-hw-01.md";
    let dest = "tasks/cs-100-homework-01.md";
    let mut spec = knowlu_engine::journal::NewRecord::new("move", from, "quinn", "dashboard");
    spec.id = Some("task_0000000001");
    spec.ts = Some("2026-09-17T10:00:00.000Z".to_string());
    spec.device = Some("OtherDesktop".to_string());
    spec.new = serde_json::json!(dest);
    let rec = knowlu_engine::journal::make_record(spec).expect("a record");
    let report = sync::apply(&dir, &pulled(vec![rec], vec![]), &ctx, &mut journal, "2026-09-17".parse().unwrap());
    assert_eq!((report.records, report.moved, report.refused), (1, 1, 0), "{report:?}");
    assert!(!dir.join(from).exists(), "the old path is empty");
    assert!(dir.join(dest).exists(), "and the note is at the new one");
    // A second pull of the same record changes nothing: the record dedupes by hash, and the move
    // pass finds no file at the old path.
    let again = sync::apply(&dir, &pulled(vec![], vec![]), &ctx, &mut journal, "2026-09-17".parse().unwrap());
    assert_eq!((again.records, again.moved), (0, 0), "{again:?}");
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn a_pulled_move_may_not_escape_the_vault() {
    // A `move` record carries a destination, and the destination is a string another machine sent.
    // It is checked with the same `is_note_path` every other path goes through, BEFORE
    // `write::move_note` is called — not after, when the file would already be somewhere else.
    let dir = fixture("escape");
    let mut journal = Journal::new(&dir);
    let ctx = knowlu_engine::write::WriteContext::new(sync::ACTOR, "local-runner");
    let mut spec = knowlu_engine::journal::NewRecord::new("move", "tasks/cs-100-hw-01.md", "quinn", "dashboard");
    spec.id = Some("task_0000000001");
    spec.ts = Some("2026-09-17T10:00:00.000Z".to_string());
    spec.device = Some("OtherDesktop".to_string());
    spec.new = serde_json::json!("../../../Windows/System32/x.md");
    let rec = knowlu_engine::journal::make_record(spec).expect("a record");
    let report = sync::apply(&dir, &pulled(vec![rec], vec![]), &ctx, &mut journal, "2026-09-17".parse().unwrap());
    assert_eq!((report.refused, report.moved, report.records), (1, 0, 0), "{report:?}");
    assert!(dir.join("tasks").join("cs-100-hw-01.md").exists(), "and the note did not move");
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn a_pull_reads_the_two_cursors_and_the_more_flag_from_the_reply() {
    let body = serde_json::json!({
        "records": [], "notes": [], "record_cursor": 41, "note_cursor": 17, "more": true
    });
    let mut server = loopback(vec![(200, knowlu_engine::ledger::dumps_value(&body))]);
    let client = CloudClient::new(&cfg(&server.base), "jwt-not-a-secret");
    let page = sync::pull(&client, 40, 16).expect("a 200");
    assert_eq!((page.record_cursor, page.note_cursor, page.more), (41, 17, true));
    let sent = server.requests();
    // Deviation from the brief's literal string, for the same reason
    // `the_push_posts_to_sync_push_and_reads_the_two_counts` already carries one: the loopback's
    // `base` (C2's own harness) already carries `/functions/v1`, which `CloudClient::get` prepends
    // to every path exactly as `CloudClient::post` does.
    assert!(sent[0].starts_with("GET /functions/v1/sync-pull?records_after=40&notes_after=16&limit=500 "), "{}", sent[0]);
}

// ---------------------------------------------------------------------------
// Fix round 1 (review): I1, I2, I3, M2, M4, M5, M7.
// ---------------------------------------------------------------------------

#[test]
fn a_later_foreign_write_never_loses_to_an_unrelated_local_mtime_bump() {
    // I1. The brief's own `ts >= first_ts` filter dropped this device's real, earlier `importance`
    // record out of `upstream_records` whenever the foreign write's `ts` was later, so
    // `reconcile::resolve` fell back to a synthetic "external write" contender stamped with the
    // file's raw mtime. An UNRELATED local edit to a DIFFERENT field, made after the foreign write,
    // bumps that mtime past the foreign `ts` — so the older local value silently won, with no card
    // and no warning, and the two desktops stayed different for good.
    let dir = fixture_with_id("later-write-wins");
    let mut journal = Journal::new(&dir);
    let mine = knowlu_engine::write::WriteContext::new("quinn", "dashboard");
    // T1: this device's own, real, journalled edit.
    knowlu_engine::write::write_literals(&dir, "tasks/cs-100-hw-01.md", &[("importance".to_string(), "4".to_string())], &mine, &mut journal, &Default::default()).expect("T1");
    std::thread::sleep(std::time::Duration::from_millis(30));
    // T2: the foreign human's write, genuinely later than T1.
    let t2 = knowlu_engine::journal::now_ts(None);
    std::thread::sleep(std::time::Duration::from_millis(30));
    // T3: an UNRELATED local edit, after T2 — bumps the file's mtime without touching `importance`.
    knowlu_engine::write::write_literals(&dir, "tasks/cs-100-hw-01.md", &[("progress".to_string(), "10".to_string())], &mine, &mut journal, &Default::default()).expect("T3");
    let ctx = knowlu_engine::write::WriteContext::new(sync::ACTOR, "local-runner");
    let rec = foreign_set("task_0000000001", "tasks/cs-100-hw-01.md", "importance", serde_json::json!(2), serde_json::json!(5), &t2);
    let report = sync::apply(&dir, &pulled(vec![rec], vec![]), &ctx, &mut journal, "2026-09-22".parse().unwrap());
    // A genuinely later foreign write must never lose silently: never `cards == 0 && applied == 0`
    // with the note quietly keeping this device's older value.
    assert_ne!((report.applied, report.cards), (0, 0), "a later foreign write lost silently: {report:?}");
    assert_eq!((report.applied, report.cards, report.superseded), (0, 1, 1), "{report:?}");
    let meta = knowlu_engine::ids::read_meta(&dir.join("tasks").join("cs-100-hw-01.md")).expect("the note");
    assert_eq!(
        knowlu_engine::yaml::get(&meta, "importance").and_then(knowlu_engine::yaml::i64_of), Some(4),
        "withheld pending the card, not silently kept for ever"
    );
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn a_pulled_records_field_name_is_checked_before_it_can_touch_a_note() {
    // I2. Nothing on the path checked `field`: `apply_frontmatter_fields_to_text` writes
    // `"{key}: {value}"` onto the note unescaped, so a `field` carrying a newline and a `---` can
    // inject arbitrary frontmatter lines, and `field: "id"` would let a pulled `set` retarget the
    // note's own opaque identity.
    let dir = fixture("field-injection");
    let mut journal = Journal::new(&dir);
    let ctx = knowlu_engine::write::WriteContext::new(sync::ACTOR, "local-runner");
    let hijack = foreign_set("task_0000000001", "tasks/cs-100-hw-01.md", "zz\n---\nhijack", serde_json::Value::Null, serde_json::json!(1), "2026-09-17T10:00:00.000Z");
    let retarget = foreign_set("task_0000000001", "tasks/cs-100-hw-01.md", "id", serde_json::json!("task_0123456789"), serde_json::json!("task_9999999999"), "2026-09-17T10:01:00.000Z");
    let report = sync::apply(&dir, &pulled(vec![hijack, retarget], vec![]), &ctx, &mut journal, "2026-09-17".parse().unwrap());
    assert_eq!((report.refused, report.records, report.applied), (2, 0, 0), "{report:?}");
    let text = knowlu_engine::pystr::read_text(&dir.join("tasks").join("cs-100-hw-01.md")).expect("the note");
    assert!(!text.contains("hijack"), "{text}");
    let meta = knowlu_engine::ids::read_meta(&dir.join("tasks").join("cs-100-hw-01.md")).expect("the note still parses");
    assert_eq!(
        knowlu_engine::yaml::get(&meta, "importance").and_then(knowlu_engine::yaml::i64_of), Some(2),
        "the frontmatter is intact"
    );
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn a_case_only_local_rename_survives_a_tombstone_for_the_old_spelling() {
    // I3. `sync-pull` returns every row for the account with no device filter, so a case-only rename
    // this device itself made can pull back its OWN tombstone for the old spelling. `file.exists()`
    // answers case-insensitively on NTFS, so `write::delete` would archive the still-live,
    // differently-cased note — the note silently leaves Today.
    let dir = fixture("case-rename");
    let mut journal = Journal::new(&dir);
    let ctx = knowlu_engine::write::WriteContext::new(sync::ACTOR, "local-runner");
    std::fs::rename(dir.join("tasks").join("cs-100-hw-01.md"), dir.join("tasks").join("CS-100-HW-01.md")).expect("the case-only rename");
    let report = sync::apply(&dir, &pulled(vec![], vec![
        sync::PulledNote { device: "fedcba9876543210".into(), path: "tasks/cs-100-hw-01.md".into(), text: None },
    ]), &ctx, &mut journal, "2026-09-17".parse().unwrap());
    assert_eq!(report.moved, 0, "the live note under its new spelling must not be archived: {report:?}");
    assert!(dir.join("tasks").join("CS-100-HW-01.md").exists(), "the live, renamed note is untouched");
    assert!(
        std::fs::read_dir(dir.join("archive"))
            .map(|d| d.flatten().all(|e| !e.file_name().to_string_lossy().to_lowercase().contains("100-hw-01")))
            .unwrap_or(true),
        "nothing was archived"
    );
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn a_supersede_record_never_leaves_this_device() {
    // M2. `reconcile::resolve` stamps a supersede record's `ts` as whichever contender WON — often
    // in the past relative to the push cursor — and its actor is always `system:reconcile`, never
    // `sync::ACTOR`, so neither of `build_push`'s existing filters would catch it. `sync::ACTOR`'s
    // own doc comment lists "the supersede record" among what stays on this device.
    let dir = fixture("supersede-stays");
    let mut journal = Journal::new(&dir);
    let mut spec = knowlu_engine::journal::NewRecord::new("supersede", "tasks/cs-100-hw-01.md", "system:reconcile", "local-runner");
    spec.id = Some("task_0000000001");
    spec.field = Some("importance");
    spec.old = serde_json::json!({"ts": "2026-09-17T10:00:00.000Z", "device": "OtherDesktop", "seq": 1, "field": "importance"});
    spec.new = serde_json::json!({"ts": "2026-09-17T11:00:00.000Z", "device": "ThisPC", "seq": 2, "field": "importance"});
    spec.ts = Some("2026-09-17T11:00:00.000Z".to_string());
    let mut rec = knowlu_engine::journal::make_record(spec).expect("a record");
    journal.append(&mut rec).expect("append the supersede record");
    let (batch, _) = sync::build_push(&dir, &Cursor::default(), "acct-1", &mut journal);
    assert!(!batch.records.is_empty(), "the fixture's own seed record must still go up");
    assert!(
        !batch.records.iter().any(|r| r["body"].as_str().unwrap_or_default().contains("supersede")),
        "a supersede record reached the wire: {:?}", batch.records
    );
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn a_live_note_with_no_body_is_skipped_never_written_as_a_blank_file() {
    // M4. `pulled_from_reply` used to turn a missing or non-string `body` on a LIVE row into
    // `Some("")` — a zero-byte file at `apply`'s note-write step, which `build_push` would then warn
    // about, for ever, on every subsequent run.
    let dir = fixture("bodyless");
    let mut journal = Journal::new(&dir);
    let ctx = knowlu_engine::write::WriteContext::new(sync::ACTOR, "local-runner");
    let reply = serde_json::json!({
        "records": [], "notes": [{"device": "fedcba9876543210", "path": "tasks/blank.md", "rev": 1}],
        "record_cursor": 0, "note_cursor": 1, "more": false
    });
    let page = sync::pulled_from_reply(&reply).expect("a parse");
    assert!(page.notes.is_empty(), "a bodyless live row must never become a note: {:?}", page.notes);
    assert!(page.warnings.iter().any(|w| w.contains("body")), "{:?}", page.warnings);
    let report = sync::apply(&dir, &page, &ctx, &mut journal, "2026-09-17".parse().unwrap());
    assert_eq!(report.notes_written, 0, "{report:?}");
    assert!(!dir.join("tasks").join("blank.md").exists(), "no zero-byte file must appear");
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn a_pulled_set_with_no_id_names_the_note_it_can_never_reach() {
    // M5. A `set` with no `id` is well-formed — `record_is_well_formed`'s `is_id` check only fires
    // on a NON-empty id — but the per-note pass groups by `id`, so it is journalled and then does
    // nothing, silently, for ever.
    let dir = fixture("no-id-set");
    let mut journal = Journal::new(&dir);
    let ctx = knowlu_engine::write::WriteContext::new(sync::ACTOR, "local-runner");
    let mut spec = knowlu_engine::journal::NewRecord::new("set", "tasks/cs-100-hw-01.md", "quinn", "dashboard");
    spec.field = Some("importance");
    spec.old = serde_json::json!(2);
    spec.new = serde_json::json!(5);
    spec.ts = Some("2026-09-17T10:00:00.000Z".to_string());
    spec.device = Some("OtherDesktop".to_string());
    let rec = knowlu_engine::journal::make_record(spec).expect("a record");
    let report = sync::apply(&dir, &pulled(vec![rec], vec![]), &ctx, &mut journal, "2026-09-17".parse().unwrap());
    assert_eq!(report.records, 1, "the record still lands — it is well-formed, just unreachable: {report:?}");
    assert!(
        report.warnings.iter().any(|w| w.contains("cs-100-hw-01") && w.contains("id")),
        "{:?}", report.warnings
    );
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn a_pulled_records_non_canonical_ts_is_refused() {
    // M7. Every ordering here — `wins`, the journal's own sort, the ledger's day-file name —
    // compares the CANONICAL `ts` string, not the parsed instant, so `…T10:00:00Z` (valid to
    // `jiff`, missing milliseconds) sorts in a place its own value disagrees with.
    let dir = fixture("noncanonical-ts");
    let mut journal = Journal::new(&dir);
    let ctx = knowlu_engine::write::WriteContext::new(sync::ACTOR, "local-runner");
    let mut rec = foreign_set("task_0000000001", "tasks/cs-100-hw-01.md", "importance", serde_json::json!(2), serde_json::json!(5), "2026-09-17T10:00:00.000Z");
    rec.insert("ts".to_string(), serde_json::json!("2026-09-17T10:00:00Z"));
    let report = sync::apply(&dir, &pulled(vec![rec], vec![]), &ctx, &mut journal, "2026-09-17".parse().unwrap());
    assert_eq!((report.refused, report.records), (1, 0), "{report:?}");
    assert!(report.warnings.iter().any(|w| w.contains("ts")), "{:?}", report.warnings);
    let _ = std::fs::remove_dir_all(&dir);
}
