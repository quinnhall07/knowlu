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

// ---------------------------------------------------------------------------
// Fix round 2 (re-review round 1): O1, O2, O3, O4 (R-C3′-exec-14).
// ---------------------------------------------------------------------------

#[test]
fn a_case_only_rename_made_on_another_desktop_never_archives_the_note_here() {
    // O1. `build_push` sends the live row under the NEW spelling and a tombstone for the OLD one in
    // the same push — the two rows are one rename event, not a delete. Comparing every tombstone in
    // this page against every live row, case-insensitively, is what tells them apart; the live
    // row's own handling does not depend on the tombstone at all. Both wire orders and both
    // with/without a companion `move` record must all land the same way: one live note under the
    // new spelling, this device's OWN content untouched, and nothing archived.
    for with_move in [false, true] {
        for tombstone_first in [false, true] {
            let dir = fixture_with_id(&format!("case-rename-elsewhere-{with_move}-{tombstone_first}"));
            let mut journal = Journal::new(&dir);
            let ctx = knowlu_engine::write::WriteContext::new(sync::ACTOR, "local-runner");
            let old_rel = "tasks/cs-100-hw-01.md";
            let new_rel = "tasks/CS-100-HW-01.md";
            let local_text = knowlu_engine::pystr::read_text(&dir.join(old_rel)).expect("the fixture note");
            // Different from the local copy, so a wrongly-overwritten note is distinguishable from a
            // correctly-renamed one: the rename must keep THIS device's body, never the pulled one.
            let foreign_text = local_text.replace("Body text.", "Body text from elsewhere.");
            let mut records = Vec::new();
            if with_move {
                let mut spec = knowlu_engine::journal::NewRecord::new("move", old_rel, "quinn", "dashboard");
                spec.id = Some("task_0000000001");
                spec.ts = Some("2026-09-17T10:00:00.000Z".to_string());
                spec.device = Some("OtherDesktop".to_string());
                spec.new = serde_json::json!(new_rel);
                records.push(knowlu_engine::journal::make_record(spec).expect("a record"));
            }
            let live = sync::PulledNote { device: "fedcba9876543210".into(), path: new_rel.into(), text: Some(foreign_text) };
            let tomb = sync::PulledNote { device: "fedcba9876543210".into(), path: old_rel.into(), text: None };
            let notes = if tombstone_first { vec![tomb, live] } else { vec![live, tomb] };
            let report = sync::apply(&dir, &pulled(records, notes), &ctx, &mut journal, "2026-09-17".parse().unwrap());
            assert!(dir.join(new_rel).exists(), "with_move={with_move} tombstone_first={tombstone_first}: {report:?}");
            let archived_any = std::fs::read_dir(dir.join("archive")).map(|d| {
                d.flatten().any(|e| e.file_name().to_string_lossy().to_lowercase().contains("100-hw-01"))
            }).unwrap_or(false);
            assert!(!archived_any, "with_move={with_move} tombstone_first={tombstone_first}: never archived: {report:?}");
            let in_tasks: Vec<String> = std::fs::read_dir(dir.join("tasks")).expect("tasks").flatten()
                .map(|e| e.file_name().to_string_lossy().to_string())
                .filter(|n| n.to_lowercase().contains("100-hw-01")).collect();
            assert_eq!(
                in_tasks, vec!["CS-100-HW-01.md".to_string()],
                "with_move={with_move} tombstone_first={tombstone_first}: exactly one live copy, under the new spelling: {in_tasks:?}"
            );
            let final_text = knowlu_engine::pystr::read_text(&dir.join(new_rel)).expect("the renamed note");
            assert_eq!(
                final_text, local_text,
                "with_move={with_move} tombstone_first={tombstone_first}: the rename must keep this device's own id and body, never the pulled copy"
            );
            let _ = std::fs::remove_dir_all(&dir);
        }
    }
}

#[test]
fn mine_excludes_every_foreign_device_in_the_page_not_only_the_first() {
    // O2 (probe N6). Two OTHER desktops' records for the same note landed in ONE page: B set
    // importance 2->5, then C set importance 5->7, later than B and later than this device's own
    // edit. The old filter excluded only `foreign[0]`'s device (B), so C's own just-appended record
    // — itself one of `foreign`, not a real local edit — stayed in `mine` and masked this device's
    // real record behind the mtime fallback whenever an unrelated local edit landed after C's `ts`.
    let dir = fixture_with_id("two-foreign-devices");
    let mut journal = Journal::new(&dir);
    let mine_ctx = knowlu_engine::write::WriteContext::new("quinn", "dashboard");
    knowlu_engine::write::write_literals(&dir, "tasks/cs-100-hw-01.md", &[("importance".to_string(), "4".to_string())], &mine_ctx, &mut journal, &Default::default()).expect("this device's edit");
    std::thread::sleep(std::time::Duration::from_millis(30));
    let t1 = knowlu_engine::journal::now_ts(None);
    std::thread::sleep(std::time::Duration::from_millis(30));
    let t2 = knowlu_engine::journal::now_ts(None);
    std::thread::sleep(std::time::Duration::from_millis(30));
    // An unrelated local edit, after both foreign timestamps — the mtime-fallback trap.
    knowlu_engine::write::write_literals(&dir, "tasks/cs-100-hw-01.md", &[("progress".to_string(), "10".to_string())], &mine_ctx, &mut journal, &Default::default()).expect("an unrelated edit");
    let b = foreign_set("task_0000000001", "tasks/cs-100-hw-01.md", "importance", serde_json::json!(2), serde_json::json!(5), &t1);
    // C's device differs from B's — the case the old, singular `foreign[0]` filter missed.
    let mut spec = knowlu_engine::journal::NewRecord::new("set", "tasks/cs-100-hw-01.md", "quinn", "dashboard");
    spec.id = Some("task_0000000001");
    spec.field = Some("importance");
    spec.old = serde_json::json!(5);
    spec.new = serde_json::json!(7);
    spec.ts = Some(t2.clone());
    spec.device = Some("DeskC".to_string());
    let c = knowlu_engine::journal::make_record(spec).expect("a record");
    let ctx = knowlu_engine::write::WriteContext::new(sync::ACTOR, "local-runner");
    let report = sync::apply(&dir, &pulled(vec![b, c], vec![]), &ctx, &mut journal, "2026-09-22".parse().unwrap());
    assert_ne!((report.applied, report.cards), (0, 0), "a genuinely later foreign write must never lose silently: {report:?}");
    assert_eq!((report.applied, report.cards), (0, 1), "{report:?}");
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn a_snoozed_stale_card_is_settled_and_refiled_like_a_pending_one() {
    // O3. `find_pending_amendment` only ever looks at `status: pending`, so a card the fifteen-a-day
    // cap has SNOOZED (never deleted, per this vault's own rule) was invisible to the stale-card
    // check: a newer conflicting value filed a second, live card beside the snoozed one instead of
    // replacing it, and the snoozed card would later wake up still offering its now-stale value.
    let dir = fixture_with_id("snoozed-then-newer");
    let mut journal = Journal::new(&dir);
    let mine = knowlu_engine::write::WriteContext::new("quinn", "dashboard");
    knowlu_engine::write::write_literals(&dir, "tasks/cs-100-hw-01.md", &[("importance".to_string(), "4".to_string())], &mine, &mut journal, &Default::default()).expect("my edit");
    let ctx = knowlu_engine::write::WriteContext::new(sync::ACTOR, "local-runner");
    // First conflict: files a pending card offering `to: 5`.
    let first = foreign_set("task_0000000001", "tasks/cs-100-hw-01.md", "importance", serde_json::json!(3), serde_json::json!(5), "2036-09-17T10:00:00.000Z");
    let r1 = sync::apply(&dir, &pulled(vec![first], vec![]), &ctx, &mut journal, "2026-09-17".parse().unwrap());
    assert_eq!(r1.cards, 1, "{r1:?}");
    let cards_before: Vec<PathBuf> = std::fs::read_dir(dir.join("approvals")).expect("approvals").flatten().map(|e| e.path())
        .filter(|p| p.file_name().map(|n| n.to_string_lossy().starts_with("amend-cs-100-hw-01-")).unwrap_or(false)).collect();
    assert_eq!(cards_before.len(), 1, "{cards_before:?}");
    let card_rel = knowlu_engine::ids::rel(&dir, &cards_before[0]);
    // The fifteen-a-day cap snoozes it, exactly as `defer_over_budget` would — never deleted.
    knowlu_engine::write::write_literals(&dir, &card_rel, &[("status".to_string(), "snoozed".to_string()), ("snooze_until".to_string(), "2026-09-18".to_string())], &mine, &mut journal, &Default::default()).expect("snooze it");
    // A newer conflicting value arrives.
    let second = foreign_set("task_0000000001", "tasks/cs-100-hw-01.md", "importance", serde_json::json!(3), serde_json::json!(8), "2036-09-17T11:00:00.000Z");
    let r2 = sync::apply(&dir, &pulled(vec![second], vec![]), &ctx, &mut journal, "2026-09-17".parse().unwrap());
    assert_eq!(r2.cards, 1, "a newer value must replace the snoozed stale card, not sit beside it: {r2:?}");
    let live_cards: Vec<PathBuf> = std::fs::read_dir(dir.join("approvals")).expect("approvals").flatten().map(|e| e.path())
        .filter(|p| p.file_name().map(|n| n.to_string_lossy().starts_with("amend-cs-100-hw-01-")).unwrap_or(false)).collect();
    assert_eq!(live_cards.len(), 1, "exactly one LIVE card must remain in approvals/: {live_cards:?}");
    let text = knowlu_engine::pystr::read_text(&live_cards[0]).expect("the live card");
    assert!(text.contains("to: 8") && !text.contains("to: 5"), "{text}");
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn a_carded_change_whose_from_equals_to_files_no_card() {
    // O4 (probe N2). The other desktop approved ITS OWN sync card, adopting the value this device
    // already held; its approval record (old 4, new 5) then comes down here, where the note already
    // holds 5. `reconcile::resolve` still treats this as a conflict — the foreign `old` disagrees
    // with the note — but the natural end of every approved sync card is a proposal with nothing
    // left to change, which should cost no cap slot.
    let dir = fixture_with_id("converged-value");
    let mut journal = Journal::new(&dir);
    let mine = knowlu_engine::write::WriteContext::new("quinn", "dashboard");
    knowlu_engine::write::write_literals(&dir, "tasks/cs-100-hw-01.md", &[("importance".to_string(), "5".to_string())], &mine, &mut journal, &Default::default()).expect("this device already holds 5");
    std::thread::sleep(std::time::Duration::from_millis(30));
    let t4 = knowlu_engine::journal::now_ts(None);
    let ctx = knowlu_engine::write::WriteContext::new(sync::ACTOR, "local-runner");
    let approval = foreign_set("task_0000000001", "tasks/cs-100-hw-01.md", "importance", serde_json::json!(4), serde_json::json!(5), &t4);
    let report = sync::apply(&dir, &pulled(vec![approval], vec![]), &ctx, &mut journal, "2026-09-17".parse().unwrap());
    assert_eq!(report.cards, 0, "a change that proposes nothing must file no card: {report:?}");
    let meta = knowlu_engine::ids::read_meta(&dir.join("tasks").join("cs-100-hw-01.md")).expect("the note");
    assert_eq!(knowlu_engine::yaml::get(&meta, "importance").and_then(knowlu_engine::yaml::i64_of), Some(5));
    let _ = std::fs::remove_dir_all(&dir);
}

// ---------------------------------------------------------------------------
// Fix round 3 (re-review round 2): B1, B2, B3 (R-C3′-exec-15).
// ---------------------------------------------------------------------------

#[test]
fn a_stale_card_is_settled_when_the_desktops_converge_not_just_when_a_newer_card_replaces_it() {
    // B1 (probe N7). A pending card offers `4 -> 5`. The other desktop then moves BACK to 4, the
    // value this device already holds: every carded field now converges (O4 drops it as
    // `from == to`), so `changes` ends up empty and the M3 stale-card check — which lives inside
    // `if !changes.is_empty()` — never runs at all. Approving the untouched `4 -> 5` card would then
    // write 5 here and, once pushed, on the other desktop too — a value neither student holds any
    // more, which is exactly what M3 exists to prevent.
    let dir = fixture_with_id("converged-with-stale-card");
    let mut journal = Journal::new(&dir);
    let mine = knowlu_engine::write::WriteContext::new("quinn", "dashboard");
    knowlu_engine::write::write_literals(&dir, "tasks/cs-100-hw-01.md", &[("importance".to_string(), "4".to_string())], &mine, &mut journal, &Default::default()).expect("my edit");
    let ctx = knowlu_engine::write::WriteContext::new(sync::ACTOR, "local-runner");
    let first = foreign_set("task_0000000001", "tasks/cs-100-hw-01.md", "importance", serde_json::json!(2), serde_json::json!(5), "2036-09-17T10:00:00.000Z");
    let r1 = sync::apply(&dir, &pulled(vec![first], vec![]), &ctx, &mut journal, "2026-09-17".parse().unwrap());
    assert_eq!(r1.cards, 1, "a pending `4 -> 5` card must exist to begin with: {r1:?}");
    // The other desktop moves back to 4 — converging on this device's own value.
    let back = foreign_set("task_0000000001", "tasks/cs-100-hw-01.md", "importance", serde_json::json!(5), serde_json::json!(4), "2036-09-17T11:00:00.000Z");
    let r2 = sync::apply(&dir, &pulled(vec![back], vec![]), &ctx, &mut journal, "2026-09-17".parse().unwrap());
    assert_eq!(r2.cards, 0, "a converged value files no new card: {r2:?}");
    let live_cards: Vec<PathBuf> = std::fs::read_dir(dir.join("approvals")).expect("approvals").flatten().map(|e| e.path())
        .filter(|p| p.file_name().map(|n| n.to_string_lossy().starts_with("amend-cs-100-hw-01-")).unwrap_or(false)).collect();
    assert!(live_cards.is_empty(), "the stale `4 -> 5` card must be settled, not left live: {live_cards:?}");
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn mine_excludes_only_this_pulls_own_records_not_every_record_from_those_devices() {
    // B2 (probe N8). B's `importance 2->5` was applied here in an EARLIER pull (a clean apply, no
    // conflict). THIS pull carries C's later `importance 2->7` for the same note, plus an unrelated
    // `progress` record from B in the SAME page. Excluding every record from B's device — not only
    // the records THIS pull appended — throws away the very record that explains this device's
    // current value (5), so `resolve` falls back to the mtime stand-in and C's genuinely later write
    // loses silently. Tested in both page orders.
    for c_first in [false, true] {
        let dir = fixture_with_id(&format!("mine-hash-not-device-{c_first}"));
        let mut journal = Journal::new(&dir);
        let ctx = knowlu_engine::write::WriteContext::new(sync::ACTOR, "local-runner");
        let today: jiff::civil::Date = "2026-09-22".parse().unwrap();
        let b1 = foreign_set("task_0000000001", "tasks/cs-100-hw-01.md", "importance", serde_json::json!(2), serde_json::json!(5), "2026-01-01T00:00:00.000Z");
        let r1 = sync::apply(&dir, &pulled(vec![b1], vec![]), &ctx, &mut journal, today);
        assert_eq!(r1.applied, 1, "c_first={c_first}: the clean first pull must apply B's value: {r1:?}");
        let c = foreign_set("task_0000000001", "tasks/cs-100-hw-01.md", "importance", serde_json::json!(2), serde_json::json!(7), "2026-01-01T00:00:01.000Z");
        let mut spec = knowlu_engine::journal::NewRecord::new("set", "tasks/cs-100-hw-01.md", "quinn", "dashboard");
        spec.id = Some("task_0000000001");
        spec.field = Some("progress");
        spec.old = serde_json::json!(0);
        spec.new = serde_json::json!(10);
        spec.ts = Some("2026-01-01T00:00:02.000Z".to_string());
        spec.device = Some("OtherDesktop".to_string());
        let b2 = knowlu_engine::journal::make_record(spec).expect("a record");
        let recs = if c_first { vec![c, b2] } else { vec![b2, c] };
        let r2 = sync::apply(&dir, &pulled(recs, vec![]), &ctx, &mut journal, today);
        assert_eq!(r2.cards, 1, "c_first={c_first}: C's genuinely later write must never lose silently: {r2:?}");
        let _ = std::fs::remove_dir_all(&dir);
    }
}

#[test]
fn o1_never_conflates_two_notes_that_merely_share_a_case_insensitive_name() {
    // B3 (probe N9). The other desktop DELETED `cs-100-hw-01.md` and, separately, CREATED an
    // unrelated note spelled `CS-100-HW-01.md` (a different id, a different body) in the same push.
    // The two rows only look like a rename because their names collide case-insensitively; without
    // checking `id:` first, O1 would drop the tombstone, rename this device's own note onto the new
    // spelling, and the other desktop's real new note would never land here — and this device's next
    // push would then overwrite the other desktop's note on the account with its own, unrelated body.
    for tombstone_first in [false, true] {
        let dir = fixture_with_id(&format!("unrelated-case-variant-{tombstone_first}"));
        let mut journal = Journal::new(&dir);
        let ctx = knowlu_engine::write::WriteContext::new(sync::ACTOR, "local-runner");
        let other_body = "---\nid: task_0000000999\ntitle: \"A different note\"\nstatus: active\n---\n\nUnrelated body.\n";
        let live = sync::PulledNote { device: "fedcba9876543210".into(), path: "tasks/CS-100-HW-01.md".into(), text: Some(other_body.to_string()) };
        let tomb = sync::PulledNote { device: "fedcba9876543210".into(), path: "tasks/cs-100-hw-01.md".into(), text: None };
        let notes = if tombstone_first { vec![tomb, live] } else { vec![live, tomb] };
        let report = sync::apply(&dir, &pulled(vec![], notes), &ctx, &mut journal, "2026-09-17".parse().unwrap());
        // Exactly one entry survives in `tasks/`, under the NEW note's own exact spelling — never
        // both, and never the old exact spelling. `Path::exists` is case-insensitive on NTFS and
        // would answer true for the OLD spelling too, once the new file exists under a case
        // variant, so the directory listing is checked exactly instead.
        let in_tasks: Vec<String> = std::fs::read_dir(dir.join("tasks")).expect("tasks").flatten()
            .map(|e| e.file_name().to_string_lossy().to_string())
            .filter(|n| n.to_lowercase().contains("100-hw-01")).collect();
        assert_eq!(
            in_tasks, vec!["CS-100-HW-01.md".to_string()],
            "tombstone_first={tombstone_first}: exactly one entry, the new note under its own spelling: {in_tasks:?}, {report:?}"
        );
        // This device's own note is a genuine delete here, not a rename it merely resembles.
        let archived: Vec<String> = std::fs::read_dir(dir.join("archive")).expect("archive").flatten()
            .map(|e| e.file_name().to_string_lossy().to_string()).collect();
        assert!(
            archived.iter().any(|n| n.starts_with("cs-100-hw-01")),
            "tombstone_first={tombstone_first}: {archived:?}, {report:?}"
        );
        assert_eq!(report.moved, 1, "tombstone_first={tombstone_first}: {report:?}");
        // The other desktop's genuinely new, unrelated note lands, with its own id and body — on
        // NTFS the new write happens after the archive, so `create_new` succeeds either wire order.
        let new_path = dir.join("tasks").join("CS-100-HW-01.md");
        assert_eq!(report.notes_written, 1, "tombstone_first={tombstone_first}: {report:?}");
        let meta = knowlu_engine::ids::read_meta(&new_path).expect("the new note");
        assert_eq!(
            knowlu_engine::yaml::get(&meta, "id").and_then(knowlu_engine::yaml::text),
            Some("task_0000000999".to_string())
        );
        let text = knowlu_engine::pystr::read_text(&new_path).expect("the new note text");
        assert_eq!(text, other_body, "tombstone_first={tombstone_first}: the new note's own body must land unchanged");
        let _ = std::fs::remove_dir_all(&dir);
    }
}

// ---------------------------------------------------------------------------
// Fix round 4 (re-review round 3): R1, R2, D1 (R-C3′-exec-16).
// ---------------------------------------------------------------------------

/// Every LIVE amend card on `tasks/cs-100-hw-01.md` (in `approvals/`, not `archive/`), as
/// `(created_by, status, changes)`, sorted. `changes` goes through `yaml::to_json`, so a test
/// compares the values a card carries rather than the emitter's spelling of them.
fn live_cards_on_the_note(dir: &Path) -> Vec<(String, String, serde_json::Value)> {
    let mut out: Vec<(String, String, serde_json::Value)> = std::fs::read_dir(dir.join("approvals"))
        .expect("approvals")
        .flatten()
        .map(|e| e.path())
        .filter(|p| p.file_name().map(|n| n.to_string_lossy().starts_with("amend-cs-100-hw-01-")).unwrap_or(false))
        .map(|p| {
            let meta = knowlu_engine::ids::read_meta(&p).expect("a card");
            let text = |k: &str| knowlu_engine::yaml::get(&meta, k).and_then(knowlu_engine::yaml::text).unwrap_or_default();
            let changes = knowlu_engine::yaml::get(&meta, "changes").map(knowlu_engine::yaml::to_json).unwrap_or(serde_json::Value::Null);
            (text("created_by"), text("status"), changes)
        })
        .collect();
    out.sort_by(|a, b| (a.0.as_str(), a.1.as_str()).cmp(&(b.0.as_str(), b.1.as_str())));
    out
}

/// How many amend cards on `tasks/cs-100-hw-01.md` have been settled into `archive/`.
fn archived_cards_on_the_note(dir: &Path) -> usize {
    std::fs::read_dir(dir.join("archive"))
        .map(|d| d.flatten().filter(|e| e.file_name().to_string_lossy().starts_with("amend-cs-100-hw-01-")).count())
        .unwrap_or(0)
}

fn importance_and_effort(dir: &Path) -> (Option<i64>, Option<serde_json::Value>) {
    let meta = knowlu_engine::ids::read_meta(&dir.join("tasks").join("cs-100-hw-01.md")).expect("the note");
    (
        knowlu_engine::yaml::get(&meta, "importance").and_then(knowlu_engine::yaml::i64_of),
        knowlu_engine::yaml::get(&meta, "effort_hours").map(knowlu_engine::yaml::to_json),
    )
}

#[test]
fn a_write_a_card_withheld_never_decides_the_next_conflict_on_that_field() {
    // R1 (probe N12). This device sets `importance` 2 -> 4 at T1. Pull 1 brings desktop B's later
    // 2 -> 5 (T2): a conflict B wins, so it is carded and WITHHELD — this device keeps 4. Pull 2
    // brings B's 5 -> 6 (T3). B's T2 record is now in this device's journal from an earlier pull,
    // so it is not one of this pull's own records and stayed in `mine`; being later than T1 it
    // became `up_latest`, but its `new` (5) is not what the note holds (4) — it never took effect
    // here — so `reconcile::resolve` fell back to the file-mtime stand-in, and one unrelated local
    // edit after T3 made that stand-in win: B's 6 lost with no card and no warning. Both with and
    // without the unrelated edit, the answer is the same card, refiled as 4 -> 6.
    for unrelated_after in [false, true] {
        let dir = fixture_with_id(&format!("withheld-then-newer-{unrelated_after}"));
        let mut journal = Journal::new(&dir);
        let mine = knowlu_engine::write::WriteContext::new("quinn", "dashboard");
        let ctx = knowlu_engine::write::WriteContext::new(sync::ACTOR, "local-runner");
        let today: jiff::civil::Date = "2026-09-22".parse().unwrap();
        knowlu_engine::write::write_literals(&dir, "tasks/cs-100-hw-01.md", &[("importance".to_string(), "4".to_string())], &mine, &mut journal, &Default::default()).expect("T1");
        std::thread::sleep(std::time::Duration::from_millis(30));
        let t2 = knowlu_engine::journal::now_ts(None);
        std::thread::sleep(std::time::Duration::from_millis(30));
        let t3 = knowlu_engine::journal::now_ts(None);
        std::thread::sleep(std::time::Duration::from_millis(30));
        let first = foreign_set("task_0000000001", "tasks/cs-100-hw-01.md", "importance", serde_json::json!(2), serde_json::json!(5), &t2);
        let r1 = sync::apply(&dir, &pulled(vec![first], vec![]), &ctx, &mut journal, today);
        assert_eq!((r1.applied, r1.cards), (0, 1), "unrelated_after={unrelated_after}: B's 5 is carded and withheld: {r1:?}");
        if unrelated_after {
            // After T3: the file's mtime now outranks B's next write, which is the whole trap.
            knowlu_engine::write::write_literals(&dir, "tasks/cs-100-hw-01.md", &[("progress".to_string(), "10".to_string())], &mine, &mut journal, &Default::default()).expect("an unrelated edit");
        }
        let second = foreign_set("task_0000000001", "tasks/cs-100-hw-01.md", "importance", serde_json::json!(5), serde_json::json!(6), &t3);
        let r2 = sync::apply(&dir, &pulled(vec![second], vec![]), &ctx, &mut journal, today);
        assert_ne!((r2.applied, r2.cards), (0, 0), "unrelated_after={unrelated_after}: B's later 6 lost silently: {r2:?}");
        assert_eq!((r2.applied, r2.cards), (0, 1), "unrelated_after={unrelated_after}: {r2:?}");
        assert_eq!(importance_and_effort(&dir).0, Some(4), "unrelated_after={unrelated_after}: still withheld pending the card");
        assert_eq!(
            live_cards_on_the_note(&dir),
            vec![(sync::ACTOR.to_string(), "pending".to_string(), serde_json::json!({"importance": {"from": 4, "to": 6}}))],
            "unrelated_after={unrelated_after}: one live card, refiled as 4 -> 6, never the stale 4 -> 5 beside it",
        );
        let _ = std::fs::remove_dir_all(&dir);
    }
}

#[test]
fn sync_never_settles_or_archives_a_proposal_it_did_not_file() {
    // R2 (probes N11a, N11b). A judge-once proposal (`created_by: agent:knowlu.enrich`) is a
    // decision the student has not made yet; "a proposal is deferred, never deleted". Sync's own
    // stale-card handling may retire only the cards sync itself filed — neither when the desktops
    // converge on the field (a) nor when a new sync conflict on the same field is carded (b).
    for converge in [true, false] {
        let dir = fixture_with_id(&format!("judge-card-{converge}"));
        let mut journal = Journal::new(&dir);
        let mine = knowlu_engine::write::WriteContext::new("quinn", "dashboard");
        let ctx = knowlu_engine::write::WriteContext::new(sync::ACTOR, "local-runner");
        let today: jiff::civil::Date = "2026-09-22".parse().unwrap();
        knowlu_engine::write::write_literals(&dir, "tasks/cs-100-hw-01.md", &[("importance".to_string(), "4".to_string())], &mine, &mut journal, &Default::default()).expect("my edit");
        let file = dir.join("tasks").join("cs-100-hw-01.md");
        let meta = knowlu_engine::ids::read_meta(&file).expect("the note");
        let judge = knowlu_engine::write::WriteContext::new("agent:knowlu.enrich", "local-runner");
        knowlu_engine::write::propose_amendment(
            &dir, &file, &meta,
            &[("importance".to_string(), knowlu_engine::yaml::from_json(&serde_json::json!(4)), knowlu_engine::yaml::from_json(&serde_json::json!(3)))],
            &judge, &mut journal, None, today,
        ).expect("a judge-once proposal");
        let rec = if converge {
            foreign_set("task_0000000001", "tasks/cs-100-hw-01.md", "importance", serde_json::json!(5), serde_json::json!(4), "2036-09-17T11:00:00.000Z")
        } else {
            foreign_set("task_0000000001", "tasks/cs-100-hw-01.md", "importance", serde_json::json!(2), serde_json::json!(5), "2036-09-17T11:00:00.000Z")
        };
        let report = sync::apply(&dir, &pulled(vec![rec], vec![]), &ctx, &mut journal, today);
        let judge_card = ("agent:knowlu.enrich".to_string(), "pending".to_string(), serde_json::json!({"importance": {"from": 4, "to": 3}}));
        let live = live_cards_on_the_note(&dir);
        assert_eq!(archived_cards_on_the_note(&dir), 0, "converge={converge}: sync archived a card it did not file: {live:?} {report:?}");
        if converge {
            assert_eq!(report.cards, 0, "{report:?}");
            assert_eq!(live, vec![judge_card], "converge={converge}: the judge-once proposal is untouched: {report:?}");
        } else {
            assert_eq!(report.cards, 1, "{report:?}");
            assert_eq!(
                live,
                vec![judge_card, (sync::ACTOR.to_string(), "pending".to_string(), serde_json::json!({"importance": {"from": 4, "to": 5}}))],
                "converge={converge}: the sync card is filed beside the judge-once proposal, never in place of it: {report:?}",
            );
        }
        let _ = std::fs::remove_dir_all(&dir);
    }
}

#[test]
fn a_sync_card_naming_any_field_this_pull_resolves_is_settled_and_one_current_card_remains() {
    // D1 (probes N10a, N10b, N10c). One pull that carries two conflicting fields files ONE card
    // naming both. A later pull that resolves only SOME of a card's fields — or resolves a field a
    // card names alongside a newly conflicting one — must leave exactly one live card holding the
    // current values, or none: never a stale card beside a fresh one, never a card still offering a
    // value neither desktop holds, and never a still-open field dropped with the card that held it.
    let t10 = "2036-09-17T10:00:00.000Z";
    let t11 = "2036-09-17T11:00:00.000Z";
    let set = |field: &str, old: serde_json::Value, new: serde_json::Value, ts: &str| {
        foreign_set("task_0000000001", "tasks/cs-100-hw-01.md", field, old, new, ts)
    };
    for variant in ['a', 'b', 'c'] {
        let dir = fixture_with_id(&format!("multi-field-card-{variant}"));
        let mut journal = Journal::new(&dir);
        let mine = knowlu_engine::write::WriteContext::new("quinn", "dashboard");
        let ctx = knowlu_engine::write::WriteContext::new(sync::ACTOR, "local-runner");
        let today: jiff::civil::Date = "2026-09-22".parse().unwrap();
        knowlu_engine::write::write_literals(
            &dir, "tasks/cs-100-hw-01.md",
            &[("importance".to_string(), "4".to_string()), ("effort_hours".to_string(), "3".to_string())],
            &mine, &mut journal, &Default::default(),
        ).expect("this device's two edits");
        let first = if variant == 'c' {
            vec![set("importance", serde_json::json!(2), serde_json::json!(5), t10)]
        } else {
            vec![set("importance", serde_json::json!(2), serde_json::json!(5), t10), set("effort_hours", serde_json::json!(2.5), serde_json::json!(4), t10)]
        };
        let r1 = sync::apply(&dir, &pulled(first, vec![]), &ctx, &mut journal, today);
        assert_eq!(r1.cards, 1, "variant {variant}: pull 1 files one card: {r1:?}");
        let (second, expected) = match variant {
            // (a) `importance` converges; `effort_hours` is still open and is carried to the new card.
            'a' => (
                vec![set("importance", serde_json::json!(5), serde_json::json!(4), t11)],
                vec![(sync::ACTOR.to_string(), "pending".to_string(), serde_json::json!({"effort_hours": {"from": 3, "to": 4}}))],
            ),
            // (b) both converge: nothing is left to ask.
            'b' => (
                vec![set("importance", serde_json::json!(5), serde_json::json!(4), t11), set("effort_hours", serde_json::json!(4), serde_json::json!(3), t11)],
                vec![],
            ),
            // (c) a single-field card, then a newer `importance` and a newly conflicting `effort_hours`.
            _ => (
                vec![set("importance", serde_json::json!(5), serde_json::json!(6), t11), set("effort_hours", serde_json::json!(2.5), serde_json::json!(4), t11)],
                vec![(sync::ACTOR.to_string(), "pending".to_string(), serde_json::json!({"effort_hours": {"from": 3, "to": 4}, "importance": {"from": 4, "to": 6}}))],
            ),
        };
        let r2 = sync::apply(&dir, &pulled(second, vec![]), &ctx, &mut journal, today);
        assert_eq!(live_cards_on_the_note(&dir), expected, "variant {variant}: {r2:?}");
        assert_eq!(r2.applied, 0, "variant {variant}: every field here is carded or converged, none written: {r2:?}");
        assert_eq!(
            importance_and_effort(&dir),
            (Some(4), Some(serde_json::json!(3))),
            "variant {variant}: the note keeps this device's values until a card is answered",
        );
        let _ = std::fs::remove_dir_all(&dir);
    }
}

#[test]
fn an_identical_re_pull_keeps_the_card_already_filed_and_charges_nothing() {
    // D1's guard. Settling every sync card that names a field this pull resolved must not turn the
    // ordinary re-proposal into churn: when the one card on the note already carries exactly what
    // this pull would file (same fields, same `from`, same `to`), it IS the re-proposal — kept, not
    // archived and refiled, so it costs no second unit of the day's cap (probe N4, pull 2).
    let dir = fixture_with_id("identical-re-pull");
    let mut journal = Journal::new(&dir);
    let mine = knowlu_engine::write::WriteContext::new("quinn", "dashboard");
    let ctx = knowlu_engine::write::WriteContext::new(sync::ACTOR, "local-runner");
    let today: jiff::civil::Date = "2026-09-22".parse().unwrap();
    knowlu_engine::write::write_literals(&dir, "tasks/cs-100-hw-01.md", &[("importance".to_string(), "4".to_string())], &mine, &mut journal, &Default::default()).expect("my edit");
    let first = foreign_set("task_0000000001", "tasks/cs-100-hw-01.md", "importance", serde_json::json!(2), serde_json::json!(6), "2036-09-17T10:00:00.000Z");
    assert_eq!(sync::apply(&dir, &pulled(vec![first], vec![]), &ctx, &mut journal, today).cards, 1);
    let before: Vec<PathBuf> = std::fs::read_dir(dir.join("approvals")).expect("approvals").flatten().map(|e| e.path())
        .filter(|p| p.file_name().map(|n| n.to_string_lossy().starts_with("amend-cs-100-hw-01-")).unwrap_or(false)).collect();
    let charged = knowlu_engine::approvals::count_proposals_created(&dir, today);
    let again = foreign_set("task_0000000001", "tasks/cs-100-hw-01.md", "importance", serde_json::json!(2), serde_json::json!(6), "2036-09-17T11:00:00.000Z");
    let r2 = sync::apply(&dir, &pulled(vec![again], vec![]), &ctx, &mut journal, today);
    assert_eq!(r2.cards, 0, "{r2:?}");
    let after: Vec<PathBuf> = std::fs::read_dir(dir.join("approvals")).expect("approvals").flatten().map(|e| e.path())
        .filter(|p| p.file_name().map(|n| n.to_string_lossy().starts_with("amend-cs-100-hw-01-")).unwrap_or(false)).collect();
    assert_eq!(after, before, "the same card, not a refiled copy");
    assert_eq!(archived_cards_on_the_note(&dir), 0);
    assert_eq!(knowlu_engine::approvals::count_proposals_created(&dir, today), charged, "no second charge against the cap");
    let _ = std::fs::remove_dir_all(&dir);
}

// ---------------------------------------------------------------------------
// Fix round 5 (re-review round 4): N16, (a), (b) (R-C3′-exec-17).
// ---------------------------------------------------------------------------

/// A foreign `set` from a named desktop — `foreign_set` with the device chosen, for the tests that
/// need two or three other desktops apart.
fn foreign_set_on(device: &str, field: &str, old: serde_json::Value, new: serde_json::Value, ts: &str) -> knowlu_engine::ledger::Record {
    let mut spec = knowlu_engine::journal::NewRecord::new("set", "tasks/cs-100-hw-01.md", "quinn", "dashboard");
    spec.id = Some("task_0000000001");
    spec.field = Some(field);
    spec.old = old;
    spec.new = new;
    spec.ts = Some(ts.to_string());
    spec.device = Some(device.to_string());
    knowlu_engine::journal::make_record(spec).expect("a record")
}

/// One desktop's `build_push`, turned into the page the other desktop pulls — the account in the
/// middle, minus the network.
///
/// **Two machines in one process.** Every record either vault journals itself carries this
/// process's own `journal::device_name()`, because both vaults live on this machine. The account
/// would hand the receiver those records under the SENDER's name, so this relabels them the way
/// two real machines see each other: the sender's own records (this process's name) become
/// `sender`, and the receiver's own records coming back round (labelled `receiver` in the sender's
/// journal) become this process's name again — the same bytes the receiver journalled, so they
/// dedupe by hash exactly as a real echo does. `KNOWLU_DEVICE` is never touched: it is
/// process-global, and every other test in this file reads it in parallel.
fn transfer(vault: &Path, cursor: &mut Cursor, journal: &mut Journal, sender: &str, receiver: &str) -> sync::Pulled {
    let this_machine = knowlu_engine::journal::device_name();
    let (batch, next) = sync::build_push(vault, cursor, "acct-test", journal);
    *cursor = next;
    let records = batch.records.iter().filter_map(|row| {
        let mut record = match serde_json::from_str::<serde_json::Value>(row["body"].as_str()?).ok()? {
            serde_json::Value::Object(map) => map,
            _ => return None,
        };
        let device = record.get("device").and_then(|v| v.as_str()).unwrap_or_default().to_string();
        if device == this_machine {
            record.insert("device".to_string(), serde_json::json!(sender));
        } else if device == receiver {
            record.insert("device".to_string(), serde_json::json!(this_machine));
        }
        Some((batch.device.clone(), record))
    }).collect();
    let notes = batch.notes.iter().map(|row| sync::PulledNote {
        device: batch.device.clone(),
        path: row["path"].as_str().unwrap_or_default().to_string(),
        text: if row.get("deleted").and_then(|v| v.as_bool()).unwrap_or(false) { None } else { row["body"].as_str().map(str::to_string) },
    }).collect();
    sync::Pulled { records, notes, record_cursor: 1, note_cursor: 1, more: false, warnings: Vec::new() }
}

fn deliver(vault: &Path, page: &sync::Pulled, journal: &mut Journal) -> sync::ApplyReport {
    let ctx = knowlu_engine::write::WriteContext::new(sync::ACTOR, "local-runner");
    sync::apply(vault, page, &ctx, journal, "2026-09-22".parse().unwrap())
}

fn by_hand(vault: &Path, journal: &mut Journal, fields: &[(&str, &str)]) {
    let me = knowlu_engine::write::WriteContext::new("quinn", "dashboard");
    let literals: Vec<(String, String)> = fields.iter().map(|(f, v)| (f.to_string(), v.to_string())).collect();
    knowlu_engine::write::write_literals(vault, "tasks/cs-100-hw-01.md", &literals, &me, journal, &Default::default()).expect("a hand edit");
}

fn card_rels_on_the_note(vault: &Path) -> Vec<String> {
    let mut out: Vec<String> = std::fs::read_dir(vault.join("approvals")).expect("approvals").flatten()
        .map(|e| e.file_name().to_string_lossy().to_string())
        .filter(|n| n.starts_with("amend-cs-100-hw-01-"))
        .map(|n| format!("approvals/{n}"))
        .collect();
    out.sort();
    out
}

#[test]
fn a_card_another_desktop_filed_is_never_settled_here_and_no_tombstone_goes_out_for_it() {
    // N16 (load-bearing), two desktops end to end through `build_push`. A card desktop B files
    // never leaves B (R-C3′-exec-18, N18) — but a copy can still sit in A's vault, delivered by a
    // build from before that rule, and it carries the same `created_by: agent:knowlu.sync` A's own
    // cards carry. A's stale-card settle must touch only the cards A itself filed (a `create` record
    // for the card's id, under `sync::ACTOR`, in A's own journal): otherwise A archives its copy of
    // B's still-open card, and — before R-C3′-exec-18 — A's next push carried a tombstone for it and
    // B's own card was archived on B, the conflict it held lost on both desktops with nothing left
    // to raise it again.
    let a = fixture_with_id("n16-desk-a");
    let b = fixture_with_id("n16-desk-b");
    let (mut ja, mut jb) = (Journal::new(&a), Journal::new(&b));
    let (mut ca, mut cb) = (Cursor::default(), Cursor::default());
    let _ = transfer(&a, &mut ca, &mut ja, "DeskA", "DeskB");
    let _ = transfer(&b, &mut cb, &mut jb, "DeskB", "DeskA");
    by_hand(&b, &mut jb, &[("effort_hours", "4"), ("importance", "5")]);
    std::thread::sleep(std::time::Duration::from_millis(30));
    by_hand(&a, &mut ja, &[("effort_hours", "3"), ("importance", "4")]);
    // 1. B pulls A's later edits: both fields conflict and A's are later, so B files ONE card.
    let page = transfer(&a, &mut ca, &mut ja, "DeskA", "DeskB");
    let r1 = deliver(&b, &page, &mut jb);
    assert_eq!(r1.cards, 1, "{r1:?}");
    let b_card = card_rels_on_the_note(&b);
    assert_eq!(b_card.len(), 1, "{b_card:?}");
    let b_card = b_card[0].clone();
    // 2. A pulls B's edits (A's are later, so A files nothing). B's card is not in B's push at all
    //    (R-C3′-exec-18) — so a copy is delivered to A by hand, exactly as a build from before that
    //    rule delivered one, to prove A still leaves a card it did not file alone.
    let page = transfer(&b, &mut cb, &mut jb, "DeskB", "DeskA");
    let b_card_id = card_id(&b, &b_card);
    assert_eq!(traces_of(&page, &b_card_id, &b_card), Vec::<String>::new(), "B's push carried its card");
    let r2 = deliver(&a, &page, &mut ja);
    assert_eq!((r2.cards, r2.notes_written), (0, 0), "{r2:?}");
    assert!(!a.join(&b_card).exists(), "B's card never reached A through the account");
    std::fs::copy(b.join(&b_card), a.join(&b_card)).expect("an old build's copy of B's card");
    let page = transfer(&a, &mut ca, &mut ja, "DeskA", "DeskB");
    let _ = deliver(&b, &page, &mut jb);
    // 3. B's student sets `importance` by hand; A pulls it. A files a card of ITS OWN for the new
    //    conflict — and leaves its copy of B's card alone.
    std::thread::sleep(std::time::Duration::from_millis(30));
    by_hand(&b, &mut jb, &[("importance", "6")]);
    let page = transfer(&b, &mut cb, &mut jb, "DeskB", "DeskA");
    let r3 = deliver(&a, &page, &mut ja);
    assert_eq!(r3.cards, 1, "{r3:?}");
    assert!(a.join(&b_card).exists(), "A settled its copy of a card B filed: {r3:?} {:?}", card_rels_on_the_note(&a));
    // 4. A's next push carries nothing about B's card — no tombstone, no row — and B's own card
    //    survives B's pull.
    let page = transfer(&a, &mut ca, &mut ja, "DeskA", "DeskB");
    let tombstones: Vec<&str> = page.notes.iter().filter(|n| n.text.is_none()).map(|n| n.path.as_str()).collect();
    assert!(!tombstones.contains(&b_card.as_str()), "a tombstone went out for B's card: {tombstones:?}");
    assert_eq!(traces_of(&page, &b_card_id, &b_card), Vec::<String>::new(), "A's push carried B's card");
    let r4 = deliver(&b, &page, &mut jb);
    assert_eq!(r4.moved, 0, "{r4:?}");
    let meta = knowlu_engine::ids::read_meta(&b.join(&b_card)).expect("B's own card is still live on B");
    assert_eq!(knowlu_engine::yaml::get(&meta, "status").and_then(knowlu_engine::yaml::text).as_deref(), Some("pending"));
    assert_eq!(archived_cards_on_the_note(&b), 0, "nothing of B's was archived");
    let _ = std::fs::remove_dir_all(&a);
    let _ = std::fs::remove_dir_all(&b);
}

#[test]
fn a_field_the_student_answered_by_hand_is_not_carried_from_a_settled_card() {
    // (a), probe N14. A settled card's other fields are carried into the fresh card only while the
    // note still holds the card's own `from` for them. Here the student answered `effort_hours` by
    // hand (3 -> 3.5), later than B's 4, instead of through the card: carrying the field would
    // re-offer B's OLDER value over the student's own later edit. A conflict still open on the other
    // side comes back through that side's own records.
    let set = |field: &str, old: serde_json::Value, new: serde_json::Value, ts: &str| {
        foreign_set("task_0000000001", "tasks/cs-100-hw-01.md", field, old, new, ts)
    };
    {
        let dir = fixture_with_id("carry-after-hand-answer");
        let mut journal = Journal::new(&dir);
        let ctx = knowlu_engine::write::WriteContext::new(sync::ACTOR, "local-runner");
        let today: jiff::civil::Date = "2026-09-22".parse().unwrap();
        by_hand(&dir, &mut journal, &[("importance", "4"), ("effort_hours", "3")]);
        std::thread::sleep(std::time::Duration::from_millis(30));
        let t2 = knowlu_engine::journal::now_ts(None);
        std::thread::sleep(std::time::Duration::from_millis(30));
        let r1 = sync::apply(&dir, &pulled(vec![set("importance", serde_json::json!(2), serde_json::json!(5), &t2), set("effort_hours", serde_json::json!(2.5), serde_json::json!(4), &t2)], vec![]), &ctx, &mut journal, today);
        assert_eq!(r1.cards, 1, "{r1:?}");
        by_hand(&dir, &mut journal, &[("effort_hours", "3.5")]);
        std::thread::sleep(std::time::Duration::from_millis(30));
        let t4 = knowlu_engine::journal::now_ts(None);
        let r2 = sync::apply(&dir, &pulled(vec![set("importance", serde_json::json!(5), serde_json::json!(6), &t4)], vec![]), &ctx, &mut journal, today);
        assert_eq!(r2.cards, 1, "{r2:?}");
        assert_eq!(
            live_cards_on_the_note(&dir),
            vec![(sync::ACTOR.to_string(), "pending".to_string(), serde_json::json!({"importance": {"from": 4, "to": 6}}))],
            "N14: the hand-answered field is not re-offered: {r2:?}",
        );
        let _ = std::fs::remove_dir_all(&dir);
    }
}

#[test]
fn a_field_a_later_clean_apply_moved_is_not_carried_from_a_settled_card() {
    // (a), probe N14b — three desktops. Desktop C's later `effort_hours` 3 -> 6 applies cleanly on a
    // field this device's card offers B's 4 for; B then moves `importance` again. The note no longer
    // holds the card's `from` (3) for `effort_hours`, so the field is not carried: re-offering B's
    // older 4 over C's later 6 would be the stale proposal the carry exists to avoid.
    {
        let dir = fixture_with_id("carry-after-clean-apply");
        let mut journal = Journal::new(&dir);
        let ctx = knowlu_engine::write::WriteContext::new(sync::ACTOR, "local-runner");
        let today: jiff::civil::Date = "2026-09-22".parse().unwrap();
        by_hand(&dir, &mut journal, &[("importance", "4"), ("effort_hours", "3")]);
        let r1 = sync::apply(&dir, &pulled(vec![
            foreign_set_on("DeskB", "importance", serde_json::json!(2), serde_json::json!(5), "2036-09-17T10:00:00.000Z"),
            foreign_set_on("DeskB", "effort_hours", serde_json::json!(2.5), serde_json::json!(4), "2036-09-17T10:00:00.000Z"),
        ], vec![]), &ctx, &mut journal, today);
        assert_eq!(r1.cards, 1, "{r1:?}");
        let rc = sync::apply(&dir, &pulled(vec![foreign_set_on("DeskC", "effort_hours", serde_json::json!(3), serde_json::json!(6), "2036-09-17T11:00:00.000Z")], vec![]), &ctx, &mut journal, today);
        assert_eq!(rc.applied, 1, "C's write applies cleanly: {rc:?}");
        let rb = sync::apply(&dir, &pulled(vec![foreign_set_on("DeskB", "importance", serde_json::json!(5), serde_json::json!(8), "2036-09-17T12:00:00.000Z")], vec![]), &ctx, &mut journal, today);
        assert_eq!(
            live_cards_on_the_note(&dir),
            vec![(sync::ACTOR.to_string(), "pending".to_string(), serde_json::json!({"importance": {"from": 4, "to": 8}}))],
            "N14b: B's older `effort_hours` is not re-offered over C's later 6: {rb:?}",
        );
        assert_eq!(importance_and_effort(&dir), (Some(4), Some(serde_json::json!(6))));
        let _ = std::fs::remove_dir_all(&dir);
    }
}

#[test]
fn a_clean_apply_on_a_carded_field_settles_the_card_that_offered_an_older_value() {
    // (b), probe N13 — three desktops. This device holds 4; its card offers B's 5. Desktop C, which
    // had this device's 4, then writes 4 -> 7 LATER than B: a clean apply here. The card's `from`
    // (4) no longer matches the note, so `approvals::apply_amendment` would refuse it for ever
    // ("stale amendment"): a field this pull applied cleanly counts as resolved for settling —
    // and never as a change, so no card offers anything in its place.
    let dir = fixture_with_id("clean-apply-on-carded-field");
    let mut journal = Journal::new(&dir);
    let ctx = knowlu_engine::write::WriteContext::new(sync::ACTOR, "local-runner");
    let today: jiff::civil::Date = "2026-09-22".parse().unwrap();
    by_hand(&dir, &mut journal, &[("importance", "4")]);
    let r1 = sync::apply(&dir, &pulled(vec![foreign_set_on("DeskB", "importance", serde_json::json!(2), serde_json::json!(5), "2036-09-17T10:00:00.000Z")], vec![]), &ctx, &mut journal, today);
    assert_eq!(r1.cards, 1, "{r1:?}");
    let r2 = sync::apply(&dir, &pulled(vec![foreign_set_on("DeskC", "importance", serde_json::json!(4), serde_json::json!(7), "2036-09-17T11:00:00.000Z")], vec![]), &ctx, &mut journal, today);
    assert_eq!((r2.applied, r2.cards), (1, 0), "{r2:?}");
    assert_eq!(importance_and_effort(&dir).0, Some(7));
    assert_eq!(live_cards_on_the_note(&dir), vec![], "the card offering B's older 5 over C's later 7 is settled: {r2:?}");
    assert_eq!(archived_cards_on_the_note(&dir), 1, "settled into archive/, never unlinked");
    let _ = std::fs::remove_dir_all(&dir);
}

// ---------------------------------------------------------------------------
// The breaker fix (re-review round 5): (i) N18, (iii) N19 (R-C3′-exec-18).
// ---------------------------------------------------------------------------

/// The `id:` a card note carries.
fn card_id(vault: &Path, rel: &str) -> String {
    let meta = knowlu_engine::ids::read_meta(&vault.join(rel)).expect("the card");
    knowlu_engine::yaml::get(&meta, "id").and_then(knowlu_engine::yaml::text).expect("a card has an id")
}

/// Everything in one pulled page that is ABOUT the card `card_rel`: a note row (live or tombstone)
/// under its name, in `approvals/` or `archive/`, and any record carrying its `id` or naming its
/// file. A sync card never leaves its device (R-C3′-exec-18), so for one this is always empty.
fn traces_of(page: &sync::Pulled, card_id: &str, card_rel: &str) -> Vec<String> {
    let stem = Path::new(card_rel).file_stem().map(|s| s.to_string_lossy().to_string()).unwrap_or_default();
    let mut out: Vec<String> = page.notes.iter()
        .filter(|n| n.path.contains(&stem))
        .map(|n| format!("note {} ({})", n.path, if n.text.is_some() { "live" } else { "tombstone" }))
        .collect();
    for (_, record) in &page.records {
        let body = knowlu_engine::ledger::dumps_value(&serde_json::Value::Object(record.clone()));
        if record.get("id").and_then(|v| v.as_str()) == Some(card_id) || body.contains(&stem) {
            out.push(format!("record {body}"));
        }
    }
    out
}

#[test]
fn a_sync_card_never_leaves_its_device_and_a_decision_on_a_copy_never_reaches_it() {
    // (i), probe N18 — two desktops end to end through `build_push` and the deck's own
    // `process_approvals`. A sync card's `from` is its own device's withheld value, so it can only
    // ever be answered there. Before R-C3′-exec-18 the card travelled as a note: on the other
    // desktop its `to` is that desktop's own value, so approving it there took `apply_amendment`'s
    // already-applied branch, and the `approved`/`executed` (or `rejected`) status records travelled
    // back and killed the REAL card on the filing desktop without applying it — silent divergence
    // after an explicit decision. Now nothing about a sync card goes to the account: not its note,
    // not its settle, not a status change, not a tombstone.
    for approve in [true, false] {
        let tag = if approve { "approve" } else { "reject" };
        let a = fixture_with_id(&format!("n18-{tag}-desk-a"));
        let b = fixture_with_id(&format!("n18-{tag}-desk-b"));
        let (mut ja, mut jb) = (Journal::new(&a), Journal::new(&b));
        let (mut ca, mut cb) = (Cursor::default(), Cursor::default());
        let today: jiff::civil::Date = "2026-09-22".parse().unwrap();
        let now: jiff::civil::DateTime = "2026-09-22T12:00".parse().unwrap();
        let me = knowlu_engine::write::WriteContext::new("quinn", "dashboard");
        let _ = transfer(&a, &mut ca, &mut ja, "DeskA", "DeskB");
        let _ = transfer(&b, &mut cb, &mut jb, "DeskB", "DeskA");
        by_hand(&a, &mut ja, &[("importance", "4")]);
        std::thread::sleep(std::time::Duration::from_millis(30));
        by_hand(&b, &mut jb, &[("importance", "5")]);
        // B's later 5 comes down to A: A files a card and withholds 5.
        let page = transfer(&b, &mut cb, &mut jb, "DeskB", "DeskA");
        let r1 = deliver(&a, &page, &mut ja);
        assert_eq!(r1.cards, 1, "{tag}: {r1:?}");
        let a_card = card_rels_on_the_note(&a);
        assert_eq!(a_card.len(), 1, "{tag}: {a_card:?}");
        let a_card = a_card[0].clone();
        let a_card_id = card_id(&a, &a_card);
        let a_card_text = knowlu_engine::pystr::read_text(&a.join(&a_card)).expect("A's card");
        // 1. A's push carries nothing about the card, and B never gets a copy through the account.
        let page = transfer(&a, &mut ca, &mut ja, "DeskA", "DeskB");
        assert_eq!(traces_of(&page, &a_card_id, &a_card), Vec::<String>::new(), "{tag}: A's push carried its card");
        let r2 = deliver(&b, &page, &mut jb);
        assert!(card_rels_on_the_note(&b).is_empty(), "{tag}: the card reached B: {r2:?}");
        // 2. A copy reaches B anyway — as a build from before this rule delivered one — and the
        //    student decides THAT copy on B, through the deck's own path.
        std::fs::copy(a.join(&a_card), b.join(&a_card)).expect("an old build's copy");
        knowlu_engine::write::write_literals(
            &b, &a_card, &[("status".to_string(), (if approve { "approved" } else { "rejected" }).to_string())],
            &me, &mut jb, &Default::default(),
        ).expect("the student's decision on B");
        let _ = knowlu_engine::approvals::process_approvals(&b, today, now, &me, &mut jb);
        assert!(card_rels_on_the_note(&b).is_empty(), "{tag}: the deck settled the copy on B");
        // 3. B's push carries nothing about the copy: no note in approvals/ or archive/, no status
        //    record, no settle, no tombstone.
        let page = transfer(&b, &mut cb, &mut jb, "DeskB", "DeskA");
        assert_eq!(traces_of(&page, &a_card_id, &a_card), Vec::<String>::new(), "{tag}: B's push carried the copy's decision");
        let r3 = deliver(&a, &page, &mut ja);
        // 4. A's card is untouched and still live, and A's own approvals pass leaves it pending.
        assert_eq!(knowlu_engine::pystr::read_text(&a.join(&a_card)).ok(), Some(a_card_text.clone()), "{tag}: {r3:?}");
        let res = knowlu_engine::approvals::process_approvals(&a, today, now, &me, &mut ja);
        assert!(res.executed.is_empty() && res.rejected.is_empty(), "{tag}: {:?} {:?}", res.executed, res.rejected);
        let meta = knowlu_engine::ids::read_meta(&a.join(&a_card)).expect("A's card is still live");
        assert_eq!(knowlu_engine::yaml::get(&meta, "status").and_then(knowlu_engine::yaml::text).as_deref(), Some("pending"), "{tag}");
        assert_eq!(importance_and_effort(&a).0, Some(4), "{tag}: still withheld pending A's own card");
        let _ = std::fs::remove_dir_all(&a);
        let _ = std::fs::remove_dir_all(&b);
    }
}

/// Every record this vault has journalled so far, re-labelled from `from` to `to` — what the
/// journal looks like when the machine was called `to` when it wrote them. Rewritten on the temp
/// copy only, one record per line through `ledger::dumps_value`, because `KNOWLU_DEVICE` is
/// process-global and every other test in this file reads it in parallel.
fn relabel_journal_device(vault: &Path, from: &str, to: &str) {
    for entry in std::fs::read_dir(vault.join("state").join("journal")).expect("a journal").flatten() {
        let path = entry.path();
        let text = std::fs::read_to_string(&path).expect("a journal file");
        let mut out = String::new();
        for line in text.lines().filter(|l| !l.trim().is_empty()) {
            let mut record: serde_json::Value = serde_json::from_str(line).expect("a record");
            if record.get("device").and_then(|v| v.as_str()) == Some(from) {
                record["device"] = serde_json::json!(to);
            }
            out.push_str(&knowlu_engine::ledger::dumps_value(&record));
            out.push('\n');
        }
        std::fs::write(&path, out).expect("the relabelled journal");
    }
}

#[test]
fn a_card_filed_before_this_machine_was_renamed_is_still_its_own_to_settle() {
    // (iii), probe N19. "A card this device filed" was keyed on the create record's `device` equal
    // to `device_name()`, so renaming the machine — or moving the vault to a new PC — orphaned
    // every card filed before: never settled, left offering a value neither desktop holds. The key
    // is now a `create` record under `sync::ACTOR` in this vault's own journal: `build_push` never
    // sends one and `apply` refuses a pulled one, so any such record was written right here.
    let dir = fixture_with_id("renamed-machine");
    let mut journal = Journal::new(&dir);
    let ctx = knowlu_engine::write::WriteContext::new(sync::ACTOR, "local-runner");
    let today: jiff::civil::Date = "2026-09-22".parse().unwrap();
    let set = |old: serde_json::Value, new: serde_json::Value, ts: &str| {
        foreign_set("task_0000000001", "tasks/cs-100-hw-01.md", "importance", old, new, ts)
    };
    by_hand(&dir, &mut journal, &[("importance", "4")]);
    let r1 = sync::apply(&dir, &pulled(vec![set(serde_json::json!(2), serde_json::json!(5), "2036-09-17T10:00:00.000Z")], vec![]), &ctx, &mut journal, today);
    assert_eq!(r1.cards, 1, "{r1:?}");
    // The machine is renamed: everything so far was journalled under the old name.
    relabel_journal_device(&dir, &knowlu_engine::journal::device_name(), "OldName");
    journal.invalidate();
    let r2 = sync::apply(&dir, &pulled(vec![set(serde_json::json!(5), serde_json::json!(6), "2036-09-17T11:00:00.000Z")], vec![]), &ctx, &mut journal, today);
    assert_eq!(
        live_cards_on_the_note(&dir),
        vec![(sync::ACTOR.to_string(), "pending".to_string(), serde_json::json!({"importance": {"from": 4, "to": 6}}))],
        "the card filed under the old name is settled and refiled, never left beside the new one: {r2:?}",
    );
    let r3 = sync::apply(&dir, &pulled(vec![set(serde_json::json!(6), serde_json::json!(4), "2036-09-17T12:00:00.000Z")], vec![]), &ctx, &mut journal, today);
    assert_eq!(live_cards_on_the_note(&dir), vec![], "converged: no card left offering 5 or 6: {r3:?}");
    assert_eq!(archived_cards_on_the_note(&dir), 2);
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn a_pulled_record_under_the_sync_actor_is_refused_and_never_journalled() {
    // (iii)'s hardening, in the wedge principle's style. `sync::ACTOR` records are what `apply`
    // writes on THIS device, and `build_push` never sends one — so a pulled record under that actor
    // is not one any desktop's sync sent. Refused with a named line before the journal append, so a
    // `create` under `ACTOR` in this journal always means "filed here" (the ownership key above).
    let dir = fixture_with_id("pulled-sync-actor");
    let mut journal = Journal::new(&dir);
    let ctx = knowlu_engine::write::WriteContext::new(sync::ACTOR, "local-runner");
    let mut spec = knowlu_engine::journal::NewRecord::new("set", "tasks/cs-100-hw-01.md", sync::ACTOR, "local-runner");
    spec.id = Some("task_0000000001");
    spec.field = Some("importance");
    spec.old = serde_json::json!(2);
    spec.new = serde_json::json!(9);
    spec.ts = Some("2026-09-17T10:00:00.000Z".to_string());
    spec.device = Some("OtherDesktop".to_string());
    let bad = knowlu_engine::journal::make_record(spec).expect("a record");
    let good = foreign_set("task_0000000001", "tasks/cs-100-hw-01.md", "progress", serde_json::json!(0), serde_json::json!(10), "2026-09-17T10:00:00.000Z");
    let report = sync::apply(&dir, &pulled(vec![bad, good], vec![]), &ctx, &mut journal, "2026-09-22".parse().unwrap());
    assert_eq!((report.refused, report.records), (1, 1), "{report:?}");
    assert!(
        report.warnings.iter().any(|w| w.contains("sync actor")),
        "the refusal is a named line: {:?}", report.warnings,
    );
    journal.invalidate();
    assert!(
        !journal.read(None, None).iter().any(|r| r.get("actor").and_then(|v| v.as_str()) == Some(sync::ACTOR)
            && r.get("device").and_then(|v| v.as_str()) == Some("OtherDesktop")),
        "a pulled sync-actor record reached the journal",
    );
    assert_eq!(importance_and_effort(&dir).0, Some(2), "and it touched nothing");
    let _ = std::fs::remove_dir_all(&dir);
}

/// I5 (fix round 1, Task 9 review), closing Task 6's own parked finding F1: `apply`'s notes loop had
/// no guard against the account's own sync amend cards at all — only `build_push`'s SENDING side
/// (`SyncCards`) ever kept them off the wire. A build from before `63612c9` (R-C3′-exec-18) pushed a
/// card as a note; pulling it back here must refuse it, the same way a pulled `ACTOR` record already
/// is, rather than write it live.
#[test]
fn a_pulled_live_sync_amend_card_is_refused_and_never_written() {
    let dir = fixture_with_id("pulled-live-sync-card");
    let mut journal = Journal::new(&dir);
    let ctx = knowlu_engine::write::WriteContext::new(sync::ACTOR, "local-runner");
    let card_text = "---\nid: approval_0000000001\ncreated_by: agent:knowlu.sync\n---\n# amend\n";
    let note = sync::PulledNote {
        device: "fedcba9876543210".to_string(),
        path: "approvals/amend-x.md".to_string(),
        text: Some(card_text.to_string()),
    };
    let report = sync::apply(&dir, &pulled(vec![], vec![note]), &ctx, &mut journal, "2026-09-22".parse().unwrap());
    assert_eq!(report.refused, 1, "{report:?}");
    assert!(!dir.join("approvals").join("amend-x.md").exists(), "the account's own card was not written here");
    assert!(
        report.warnings.iter().any(|w| w.contains("amend card")),
        "the refusal is a named line: {:?}", report.warnings,
    );
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn a_cursor_from_before_the_rule_never_carries_or_tombstones_a_sync_card() {
    // (i), path 5. A cursor written before R-C3′-exec-18 can already remember a sync card's path,
    // from a push that sent the card as a note. From then on the card must still never go out: not
    // as a row while it is live (its path is dropped from the cursor instead), and not as a
    // tombstone once the deck settles it into `archive/` (a path a sync card has had is never
    // tombstoned), even when the old cursor is the one the push starts from.
    let dir = fixture_with_id("legacy-cursor-card");
    let mut journal = Journal::new(&dir);
    let ctx = knowlu_engine::write::WriteContext::new(sync::ACTOR, "local-runner");
    let today: jiff::civil::Date = "2026-09-22".parse().unwrap();
    by_hand(&dir, &mut journal, &[("importance", "4")]);
    std::thread::sleep(std::time::Duration::from_millis(30));
    // Later than this device's edit, earlier than the approval below — so the approval's own record
    // falls after everything the old cursor had pushed.
    let t2 = knowlu_engine::journal::now_ts(None);
    std::thread::sleep(std::time::Duration::from_millis(30));
    let r = sync::apply(&dir, &pulled(vec![foreign_set("task_0000000001", "tasks/cs-100-hw-01.md", "importance", serde_json::json!(2), serde_json::json!(5), &t2)], vec![]), &ctx, &mut journal, today);
    assert_eq!(r.cards, 1, "{r:?}");
    let card = card_rels_on_the_note(&dir)[0].clone();
    let id = card_id(&dir, &card);
    // The old build's cursor: everything on disk as pushed, the card's path under a stale hash, as
    // it was when that build sent it before the card last changed.
    let (_, mut legacy) = sync::build_push(&dir, &Cursor::default(), "acct-test", &mut journal);
    legacy.notes.insert(card.clone(), "0".repeat(64));
    let (batch, next) = sync::build_push(&dir, &legacy, "acct-test", &mut journal);
    let rows: Vec<String> = batch.notes.iter().filter_map(|n| n["path"].as_str().map(str::to_string)).collect();
    assert!(!rows.iter().any(|p| p.contains(&id) || p == &card), "the live card went out as a row: {rows:?}");
    assert!(!next.notes.contains_key(&card), "the card's path stays in the cursor: {:?}", next.notes.keys().collect::<Vec<_>>());
    // The student approves the card through the deck; it is executed and settled into archive/.
    let me = knowlu_engine::write::WriteContext::new("quinn", "dashboard");
    knowlu_engine::write::write_literals(&dir, &card, &[("status".to_string(), "approved".to_string())], &me, &mut journal, &Default::default()).expect("approve");
    let now: jiff::civil::DateTime = "2026-09-22T12:00".parse().unwrap();
    let res = knowlu_engine::approvals::process_approvals(&dir, today, now, &me, &mut journal);
    assert_eq!(res.executed.len(), 1, "{:?}", res.executed);
    assert!(card_rels_on_the_note(&dir).is_empty() && archived_cards_on_the_note(&dir) == 1);
    // From the OLD cursor again: no tombstone for the card's old path, no row for its archive copy,
    // no record about it — but the approved value on the task itself does travel.
    let (batch, next) = sync::build_push(&dir, &legacy, "acct-test", &mut journal);
    let stem = Path::new(&card).file_stem().map(|s| s.to_string_lossy().to_string()).unwrap_or_default();
    let about_the_card: Vec<String> = batch.notes.iter().chain(batch.records.iter())
        .map(knowlu_engine::ledger::dumps_value)
        .filter(|row| row.contains(&stem) || row.contains(&id))
        .collect();
    assert_eq!(about_the_card, Vec::<String>::new(), "the settled card left this device");
    assert!(!next.notes.keys().any(|p| p.contains(&stem)), "{:?}", next.notes.keys().collect::<Vec<_>>());
    assert!(
        batch.records.iter().any(|r| r["body"].as_str().is_some_and(|b| b.contains("\"field\": \"importance\"") && b.contains("\"new\": 5") && b.contains("task_0000000001"))),
        "the approved value on the task travels like any edit",
    );
    let _ = std::fs::remove_dir_all(&dir);
}

// ---------------------------------------------------------------------------
// Task 6b: a rejected sync card re-asserts this device's value (R-C3′-exec-20).
// ---------------------------------------------------------------------------

/// The student decides a card through the deck's own path: `status` set from the dashboard, then
/// `process_approvals` under the executor's own context (`approvals::default_ctx`).
fn decide(vault: &Path, journal: &mut Journal, card_rel: &str, status: &str) -> knowlu_engine::approvals::ApprovalsResult {
    let student = knowlu_engine::write::WriteContext::new("quinn", "dashboard");
    knowlu_engine::write::write_literals(vault, card_rel, &[("status".to_string(), status.to_string())], &student, journal, &Default::default())
        .expect("the student's decision");
    let today: jiff::civil::Date = "2026-09-22".parse().unwrap();
    let now: jiff::civil::DateTime = "2026-09-22T12:00".parse().unwrap();
    knowlu_engine::approvals::process_approvals(vault, today, now, &knowlu_engine::approvals::default_ctx(), journal)
}

/// The `set` records in a pulled page for the task note, as `(field, old, new, actor)`.
fn task_sets(page: &sync::Pulled) -> Vec<(String, serde_json::Value, serde_json::Value, String)> {
    page.records.iter()
        .filter(|(_, r)| r.get("op").and_then(|v| v.as_str()) == Some("set")
            && r.get("id").and_then(|v| v.as_str()) == Some("task_0000000001"))
        .map(|(_, r)| (
            r.get("field").and_then(|v| v.as_str()).unwrap_or_default().to_string(),
            r.get("old").cloned().unwrap_or(serde_json::Value::Null),
            r.get("new").cloned().unwrap_or(serde_json::Value::Null),
            r.get("actor").and_then(|v| v.as_str()).unwrap_or_default().to_string(),
        ))
        .collect()
}

/// Two desktops where B's later `importance` 5 has come down to A and A has filed a sync card for
/// it (`4 -> 5`, withheld), with both pushes settled so the next push starts clean.
fn two_desktops_with_a_card(tag: &str, a_fields: &[(&str, &str)], b_fields: &[(&str, &str)]) -> (PathBuf, PathBuf, Journal, Journal, Cursor, Cursor, String) {
    let a = fixture_with_id(&format!("t6b-{tag}-desk-a"));
    let b = fixture_with_id(&format!("t6b-{tag}-desk-b"));
    let (mut ja, mut jb) = (Journal::new(&a), Journal::new(&b));
    let (mut ca, mut cb) = (Cursor::default(), Cursor::default());
    let _ = transfer(&a, &mut ca, &mut ja, "DeskA", "DeskB");
    let _ = transfer(&b, &mut cb, &mut jb, "DeskB", "DeskA");
    by_hand(&a, &mut ja, a_fields);
    std::thread::sleep(std::time::Duration::from_millis(30));
    by_hand(&b, &mut jb, b_fields);
    let page = transfer(&b, &mut cb, &mut jb, "DeskB", "DeskA");
    let r = deliver(&a, &page, &mut ja);
    assert_eq!(r.cards, 1, "{tag}: A files one card: {r:?}");
    let page = transfer(&a, &mut ca, &mut ja, "DeskA", "DeskB");
    let _ = deliver(&b, &page, &mut jb);
    let card = card_rels_on_the_note(&a)[0].clone();
    (a, b, ja, jb, ca, cb, card)
}

#[test]
fn rejecting_a_sync_card_re_asserts_this_devices_value_and_the_other_desktop_converges() {
    // N2x (reject). A's card offers B's later 5 over A's own 4. The student rejects it on A — "keep
    // mine". Before R-C3′-exec-20 the reject wrote nothing to the task, so A's push carried nothing
    // about `importance`: A kept 4, B kept 5, no card anywhere, and the student's explicit choice
    // never reached B. Now the reject appends one journal-only `set` per field the card names —
    // `old` the card's `to` (B's value), `new` the note's current value, a fresh `ts`, the deck's
    // actor — which travels like any local edit and applies cleanly on B.
    let (a, b, mut ja, mut jb, mut ca, _cb, card) = two_desktops_with_a_card("reject", &[("importance", "4")], &[("importance", "5")]);
    let task = a.join("tasks").join("cs-100-hw-01.md");
    let before = knowlu_engine::pystr::read_text(&task).expect("A's task");
    let res = decide(&a, &mut ja, &card, "rejected");
    assert_eq!(res.rejected.len(), 1, "{:?}", res.rejected);
    assert_eq!(knowlu_engine::pystr::read_text(&task).expect("A's task"), before, "the reject never touches the note");
    let page = transfer(&a, &mut ca, &mut ja, "DeskA", "DeskB");
    assert_eq!(
        task_sets(&page),
        vec![("importance".to_string(), serde_json::json!(5), serde_json::json!(4), knowlu_engine::approvals::default_ctx().actor.clone())],
        "one re-assert: old = the card's `to`, new = A's current value, the deck's actor",
    );
    let r = deliver(&b, &page, &mut jb);
    assert_eq!((r.applied, r.cards), (1, 0), "B takes A's value cleanly: {r:?}");
    assert_eq!(importance_and_effort(&a).0, Some(4));
    assert_eq!(importance_and_effort(&b).0, Some(4), "the student's choice reached B");
    assert!(card_rels_on_the_note(&a).is_empty() && card_rels_on_the_note(&b).is_empty(), "no card anywhere");
    let _ = std::fs::remove_dir_all(&a);
    let _ = std::fs::remove_dir_all(&b);
}

#[test]
fn a_field_edited_here_after_the_card_was_filed_is_not_re_asserted_on_reject() {
    // The skip case. A's card names `importance` and `effort_hours`; the student then edits
    // `effort_hours` by hand on A (3 -> 3.5), so the note no longer holds the card's `from` (3) for
    // it — that later local edit is already travelling as its own record — and only `importance` is
    // re-asserted when the card is rejected.
    let (a, b, mut ja, mut jb, mut ca, _cb, card) = two_desktops_with_a_card(
        "skip", &[("importance", "4"), ("effort_hours", "3")], &[("importance", "5"), ("effort_hours", "4")],
    );
    by_hand(&a, &mut ja, &[("effort_hours", "3.5")]);
    let res = decide(&a, &mut ja, &card, "rejected");
    assert_eq!(res.rejected.len(), 1, "{:?}", res.rejected);
    let page = transfer(&a, &mut ca, &mut ja, "DeskA", "DeskB");
    let actor = knowlu_engine::approvals::default_ctx().actor.clone();
    let mut sets = task_sets(&page);
    sets.sort_by(|x, y| x.0.cmp(&y.0));
    assert_eq!(
        sets,
        vec![
            ("effort_hours".to_string(), serde_json::json!(3), serde_json::json!(3.5), "quinn".to_string()),
            ("importance".to_string(), serde_json::json!(5), serde_json::json!(4), actor),
        ],
        "`effort_hours` travels as the student's own edit, never re-asserted over it",
    );
    let r = deliver(&b, &page, &mut jb);
    assert_eq!(importance_and_effort(&b).0, Some(4), "{r:?}");
    let _ = std::fs::remove_dir_all(&a);
    let _ = std::fs::remove_dir_all(&b);
}

#[test]
fn rejecting_a_judge_once_card_writes_no_re_assert_record() {
    // A judge-once proposal (`created_by: agent:knowlu.enrich`) asks whether the AGENT may change a
    // field the student set; rejecting it means "leave my value", which no other device has to be
    // told. Only a sync card — whose other side is another desktop — re-asserts.
    let dir = fixture_with_id("t6b-judge-reject");
    let mut journal = Journal::new(&dir);
    let today: jiff::civil::Date = "2026-09-22".parse().unwrap();
    by_hand(&dir, &mut journal, &[("importance", "4")]);
    let file = dir.join("tasks").join("cs-100-hw-01.md");
    let meta = knowlu_engine::ids::read_meta(&file).expect("the note");
    let judge = knowlu_engine::write::WriteContext::new("agent:knowlu.enrich", "local-runner");
    let card = knowlu_engine::write::propose_amendment(
        &dir, &file, &meta,
        &[("importance".to_string(), knowlu_engine::yaml::from_json(&serde_json::json!(4)), knowlu_engine::yaml::from_json(&serde_json::json!(3)))],
        &judge, &mut journal, None, today,
    ).expect("a judge-once proposal");
    let card_rel = knowlu_engine::ids::rel(&dir, &card);
    let count = |journal: &mut Journal| journal.read(None, None).iter()
        .filter(|r| r.get("id").and_then(|v| v.as_str()) == Some("task_0000000001") && r.get("op").and_then(|v| v.as_str()) == Some("set"))
        .count();
    let before = count(&mut journal);
    let res = decide(&dir, &mut journal, &card_rel, "rejected");
    assert_eq!(res.rejected.len(), 1, "{:?}", res.rejected);
    journal.invalidate();
    assert_eq!(count(&mut journal), before, "no record about the task");
    assert_eq!(importance_and_effort(&dir).0, Some(4));
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn approving_a_sync_card_is_unchanged_and_the_other_desktop_converges() {
    // Approving is untouched by R-C3′-exec-20: `apply_amendment` writes the card's `to` onto the task
    // (one `set`, the executor's actor), that record travels, and B — which already holds it —
    // converges through O4 with no card. No re-assert record rides along.
    let (a, b, mut ja, mut jb, mut ca, _cb, card) = two_desktops_with_a_card("approve", &[("importance", "4")], &[("importance", "5")]);
    let res = decide(&a, &mut ja, &card, "approved");
    assert_eq!(res.executed.len(), 1, "{:?}", res.executed);
    let page = transfer(&a, &mut ca, &mut ja, "DeskA", "DeskB");
    assert_eq!(
        task_sets(&page),
        vec![("importance".to_string(), serde_json::json!(4), serde_json::json!(5), knowlu_engine::approvals::default_ctx().actor.clone())],
    );
    let r = deliver(&b, &page, &mut jb);
    assert_eq!(r.cards, 0, "{r:?}");
    assert_eq!((importance_and_effort(&a).0, importance_and_effort(&b).0), (Some(5), Some(5)));
    assert!(card_rels_on_the_note(&a).is_empty() && card_rels_on_the_note(&b).is_empty());
    let _ = std::fs::remove_dir_all(&a);
    let _ = std::fs::remove_dir_all(&b);
}

// ---------------------------------------------------------------------------
// The command (C3' Task 7): `sync::run_lines`, always exit 0, one line each.
// ---------------------------------------------------------------------------

#[test]
fn a_vault_with_no_account_says_so_and_exits_zero() {
    let dir = fixture("noaccount");
    let (code, lines) = sync::run_lines(&dir, sync::Direction::Both, "cli", None);
    assert_eq!(code, 0);
    assert_eq!(lines, vec!["sync (skipped: no account)".to_string()]);
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn a_vault_with_an_account_and_no_session_says_exactly_that() {
    // **The check order is the message** (review I4, inherited): `load` → `resolve`. The common
    // answer on a machine with a `cloud.yaml` and no credential is "no session", and a student who
    // has signed out must not read a sentence about their subscription. Reversing the order makes
    // this assertion impossible to write, which is how the review found it the first time.
    //
    // **This case reaches Windows Credential Manager, and the exemption is recorded** (review M10).
    // `CLAUDE.md` asks a new test file that touches the store to carry a file-scoped lock; that rule
    // exists because parallel `CredWriteW`/`CredReadW` on the SAME target race. This test only
    // READS, and it reads a target (`knowlu/c3-no-such-profile/session`) that no test and no build of
    // this product ever writes — so there is nothing to serialise with. `sync_contract.rs` therefore
    // carries no lock, deliberately, and this comment is the record of that decision.
    let dir = fixture("nosession");
    std::fs::write(
        dir.join("config").join("cloud.yaml"),
        "api_base: 'https://example.invalid/functions/v1'\nanon_key: 'anon'\nsession_credential_target: 'knowlu/c3-no-such-profile/session'\naccount_id: 'acct-1'\n",
    ).expect("cloud.yaml");
    let (code, lines) = sync::run_lines(&dir, sync::Direction::Both, "cli", None);
    assert_eq!(code, 0);
    assert_eq!(lines.len(), 1, "{lines:?}");
    assert!(lines[0].starts_with("sync (skipped: no session"), "{lines:?}");
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn every_refusal_is_a_named_line_and_never_a_non_zero_exit() {
    // The property `judge` has and the one this step must have: a non-zero exit sets
    // `RunSummary.engine_ok = false`, which paints the tray amber and puts the slot into retry
    // backoff twice a day forever.
    // **Why none of these reaches a socket, stated because it is load-bearing** (review M9):
    // `cloudmodel::load` returns `Some` only when all four keys are present and non-empty, so every
    // shape below answers `None` and `run_lines_with` stops at check one. If `CloudConfig` ever
    // gained a defaulted field, the second case would resolve `example.invalid` and this test would
    // start doing DNS while still passing — so a reader who adds a default to that struct owes this
    // test a `.invalid` host it can never leave the machine through, which is why the host is one.
    let dir = fixture("exitzero");
    for yaml in ["", "api_base: 'https://example.invalid'\n", "not: yaml: at: all\n"] {
        std::fs::write(dir.join("config").join("cloud.yaml"), yaml).expect("cloud.yaml");
        let (code, lines) = sync::run_lines(&dir, sync::Direction::Both, "cli", None);
        assert_eq!(code, 0, "{yaml:?} -> {lines:?}");
        assert!(!lines.is_empty(), "{yaml:?} said nothing");
    }
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn a_line_never_carries_a_vault_path_a_bearer_or_a_hostname() {
    let dir = fixture("quiet-lines");
    let (_, lines) = sync::run_lines(&dir, sync::Direction::Both, "cli", None);
    let joined = lines.join("\n");
    // Bound first: an array literal mixing a `&str` with two `&String` over temporaries does not
    // compile, and the first draft of this test did exactly that (review M1).
    let vault_path = dir.to_string_lossy().to_string();
    let host = knowlu_engine::journal::device_name();
    let forbidden: [&str; 3] = ["Bearer", vault_path.as_str(), host.as_str()];
    for word in forbidden {
        assert!(!joined.contains(word), "a line carried {word}: {joined}");
    }
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn the_status_the_page_reads_is_built_from_the_totals_and_never_from_a_line() {
    let totals = sync::Totals { pushed_records: 3, applied: 1, cards: 1, ..Default::default() };
    let s = sync::SyncStatus::of(&totals, vec!["sync: 3 up, 1 applied, 1 card".to_string()]);
    assert!(s.ok && s.last_error.is_none());
    assert!(s.at.is_some(), "a run that happened is stamped");
    let failed = sync::SyncStatus::of(&sync::Totals { errors: vec!["no network".into()], ..Default::default() }, vec![]);
    assert!(!failed.ok);
    assert_eq!(failed.last_error.as_deref(), Some("no network"));
    // A skip is not an error: the tray must not go amber because a student is signed out.
    let skipped = sync::SyncStatus::of(&sync::Totals { skipped: Some("no account".into()), ..Default::default() }, vec![]);
    assert!(skipped.ok && skipped.last_error.is_none());
}

// ---------------------------------------------------------------------------
// Fix round 1 (ruling R-C3'-exec-24).
// ---------------------------------------------------------------------------

#[test]
fn i2_sync_status_carries_the_skip_reason_separately_from_ok_and_last_error() {
    // Review I2: before this fix, a skip and a clean sync were the same `SyncStatus` — both
    // `ok: true, last_error: None` — so a page reading only those two fields could not tell
    // "signed out" from "in step with your account".
    let skipped = sync::SyncStatus::of(&sync::Totals { skipped: Some("no session".into()), ..Default::default() }, vec![]);
    assert_eq!(skipped.skipped.as_deref(), Some("no session"));
    assert!(skipped.ok && skipped.last_error.is_none(), "still not an error");
    let clean = sync::SyncStatus::of(&sync::Totals::default(), vec![]);
    assert_eq!(clean.skipped, None, "a clean run carries no skip reason");
}

#[test]
fn i1_a_run_that_finds_the_lock_held_skips_and_writes_nothing() {
    let dir = fixture("locked");
    let lock_path = dir.join(sync::RUN_LOCK_FILE);
    if let Some(parent) = lock_path.parent() {
        std::fs::create_dir_all(parent).expect("state/");
    }
    // This test process holds the lock first, exactly as a slot's `sync` child would while the
    // console's own *Sync now* is clicked seconds later — two different `File` handles to the same
    // path, which is what `try_lock` actually serialises against (review I1).
    let held = std::fs::OpenOptions::new().create(true).write(true).open(&lock_path).expect("open the lock file");
    held.try_lock().expect("this test process holds it first");
    let cursor_before = std::fs::read(dir.join(knowlu_engine::sync::CURSOR_FILE)).ok();
    // review N3: not even the status file — the holder saves its own result, and a save from the
    // skip could land between the holder's save and its exit, showing the skip over a real result.
    assert!(!dir.join(knowlu_engine::sync::STATUS_FILE).exists(), "no status file exists yet");
    let (code, lines) = sync::run_lines(&dir, sync::Direction::Both, "cli", None);
    assert_eq!(code, 0);
    assert_eq!(lines, vec!["sync (skipped: another sync is running)".to_string()]);
    assert_eq!(
        std::fs::read(dir.join(knowlu_engine::sync::CURSOR_FILE)).ok(),
        cursor_before,
        "no cursor was written while the lock was held"
    );
    assert!(
        !dir.join(knowlu_engine::sync::STATUS_FILE).exists(),
        "the lock-held skip must write NOTHING, not even the status file (review N3)"
    );
    drop(held);
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn i3_pull_names_a_transport_failure_as_offline() {
    // A closed port on loopback: no DNS, no route off the machine, and it fails fast — the same
    // "closed 127.0.0.1 port" pattern `app/tests/scheduler.rs` already uses for `api_base` (CLAUDE.md).
    let client = CloudClient::new(&cfg("http://127.0.0.1:9/functions/v1"), "jwt-not-a-secret");
    let err = sync::pull(&client, 0, 0).expect_err("nothing listens on port 9");
    assert!(err.is_transport(), "{err:?}");
    assert_eq!(err.label(), "offline: the account could not be reached");
}

#[test]
fn i3_pull_names_a_401_as_signed_out() {
    let mut server = loopback(vec![(401, "{\"error\":\"jwt expired\"}".to_string())]);
    let client = CloudClient::new(&cfg(&server.base), "jwt-not-a-secret");
    let err = sync::pull(&client, 0, 0).expect_err("a 401");
    assert!(!err.is_transport(), "{err:?}");
    assert_eq!(err.label(), "signed out");
    let _ = server.requests();
}

#[test]
fn i3_pull_names_a_402_as_no_entitlement() {
    let mut server = loopback(vec![(402, "{\"error\":\"no active subscription\"}".to_string())]);
    let client = CloudClient::new(&cfg(&server.base), "jwt-not-a-secret");
    let err = sync::pull(&client, 0, 0).expect_err("a 402");
    assert!(!err.is_transport(), "{err:?}");
    assert_eq!(err.label(), "no entitlement");
    let _ = server.requests();
}

#[test]
fn i3_pull_names_any_other_status_by_its_code_and_reason() {
    // Fix round 2, review N2: the server's own reason survives for every status this module has no
    // named bucket for — not just the code.
    let mut server = loopback(vec![(500, "{\"error\":\"internal\"}".to_string())]);
    let client = CloudClient::new(&cfg(&server.base), "jwt-not-a-secret");
    let err = sync::pull(&client, 0, 0).expect_err("a 500");
    assert!(!err.is_transport(), "{err:?}");
    assert_eq!(err.label(), "the service refused (500: internal)");
    let _ = server.requests();
}

#[test]
fn i3_pull_names_any_other_status_by_its_code_alone_when_the_body_carries_no_reason() {
    // Fix round 2, review N2: an empty reason keeps the plain "(<code>)" form rather than a
    // trailing ": ".
    let mut server = loopback(vec![(503, "{}".to_string())]);
    let client = CloudClient::new(&cfg(&server.base), "jwt-not-a-secret");
    let err = sync::pull(&client, 0, 0).expect_err("a 503");
    assert!(!err.is_transport(), "{err:?}");
    assert_eq!(err.label(), "the service refused (503)");
    let _ = server.requests();
}

#[test]
fn n2_a_403_size_limit_refusal_keeps_the_servers_reason() {
    // review N2: `/sync-push`'s own account-size-ceiling 403 is exactly the shape the classifier
    // used to drop — the exact scenario the review named. Pinned through `run_lines_with_client`,
    // as the review's own fix suggestion asks, so both the printed line and `last_error` are
    // proven, not just `push`'s raw `CloudError` (which `a_403_is_the_named_ceiling_refusal`
    // already covers).
    let dir = fixture("size-limit");
    let pull_body = serde_json::json!({ "records": [], "notes": [], "record_cursor": 0, "note_cursor": 0, "more": false });
    let mut server = loopback(vec![
        (200, knowlu_engine::ledger::dumps_value(&pull_body)),
        (403, r#"{"error":"this account's copy is at its size limit"}"#.to_string()),
    ]);
    let cloud = cfg(&server.base);
    let client = CloudClient::new(&cloud, "jwt-not-a-secret");
    let (lines, totals) = sync::run_lines_with_client(
        &dir, sync::Direction::Both, "cli", None, &client, &cloud, Vec::new(), sync::Totals::default(),
    );
    assert_eq!(
        totals.errors,
        vec!["the service refused (403: this account's copy is at its size limit)".to_string()],
        "{lines:?}"
    );
    assert!(
        lines.iter().any(|l| l.contains("this account's copy is at its size limit")),
        "{lines:?}"
    );
    let status = sync::SyncStatus::of(&totals, lines.clone());
    assert_eq!(
        status.last_error.as_deref(),
        Some("the service refused (403: this account's copy is at its size limit)")
    );
    let _ = server.requests();
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn n5_a_status_save_failure_is_named_on_the_run_rather_than_swallowed() {
    // A directory sitting where the status file would go makes the final `rename` fail (the temp
    // file itself writes fine, beside it) — a deterministic way to exercise the "save refused"
    // branch without touching anything platform-specific.
    let dir = fixture("status-save-fails");
    std::fs::create_dir_all(dir.join(knowlu_engine::sync::STATUS_FILE)).expect("a directory in its place");
    let (code, lines) = sync::run_lines(&dir, sync::Direction::Both, "cli", None);
    assert_eq!(code, 0, "still always 0 even when the status itself could not be saved");
    assert!(
        lines.iter().any(|l| l.starts_with("sync: the status could not be saved")),
        "a failed save must be named on the run's own output, not swallowed: {lines:?}"
    );
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn i4_run_lines_persists_the_status_to_a_file_on_every_run_including_a_skip() {
    let dir = fixture("status-persists");
    let (code, lines) = sync::run_lines(&dir, sync::Direction::Both, "cli", None);
    assert_eq!(code, 0);
    assert_eq!(lines, vec!["sync (skipped: no account)".to_string()]);
    let loaded = sync::load_status(&dir);
    assert!(loaded.ok, "a skip is not a failure: {loaded:?}");
    assert_eq!(loaded.skipped.as_deref(), Some("no account"));
    assert_eq!(loaded.lines, lines);
    assert!(loaded.at.is_some(), "a run that happened is stamped");
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn i4_load_status_of_a_vault_that_never_synced_is_the_default() {
    let dir = fixture("no-status-yet");
    assert_eq!(sync::load_status(&dir), sync::SyncStatus::default());
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn m5_run_lines_with_client_does_a_full_pull_apply_save_push_save_round() {
    let dir = fixture("roundtrip");
    let pull_body = serde_json::json!({ "records": [], "notes": [], "record_cursor": 5, "note_cursor": 2, "more": false });
    let push_body = serde_json::json!({ "records": 1, "notes": 0 });
    let mut server = loopback(vec![
        (200, knowlu_engine::ledger::dumps_value(&pull_body)),
        (200, knowlu_engine::ledger::dumps_value(&push_body)),
    ]);
    let cloud = cfg(&server.base);
    let client = CloudClient::new(&cloud, "jwt-not-a-secret");
    let (lines, totals) = sync::run_lines_with_client(
        &dir, sync::Direction::Both, "cli", None, &client, &cloud, Vec::new(), sync::Totals::default(),
    );
    assert!(totals.errors.is_empty(), "{lines:?}");
    assert_eq!((totals.pulled_records, totals.pulled_notes), (0, 0));
    assert_eq!(totals.pushed_records, 1, "{totals:?}");
    // Both halves' own cursor saves landed: the pull's record/note cursors, and the push's
    // `pushed_through` advance over the fixture's own seed record (review M5).
    let saved = sync::load_cursor(&dir);
    assert_eq!((saved.record_cursor, saved.note_cursor), (5, 2));
    assert!(!saved.pushed_through.is_empty(), "the push half advanced the record cursor too");
    let _ = server.requests();
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn m5_m7_an_offline_pull_names_itself_and_the_push_waits() {
    let dir = fixture("offline-pull");
    let cloud = cfg("http://127.0.0.1:9/functions/v1");
    let client = CloudClient::new(&cloud, "jwt-not-a-secret");
    let (lines, totals) = sync::run_lines_with_client(
        &dir, sync::Direction::Both, "cli", None, &client, &cloud, Vec::new(), sync::Totals::default(),
    );
    assert_eq!(totals.errors, vec!["offline: the account could not be reached".to_string()], "{totals:?}");
    assert!(lines.iter().any(|l| l.contains("offline: the account could not be reached")), "{lines:?}");
    assert!(lines.iter().any(|l| l == "sync: the push waits for the network"), "{lines:?}");
    assert!(!lines.iter().any(|l| l.contains(" up")), "no push was attempted: {lines:?}");
    let _ = std::fs::remove_dir_all(&dir);
}
