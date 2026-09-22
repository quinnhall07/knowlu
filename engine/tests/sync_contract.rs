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
