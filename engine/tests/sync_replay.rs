//! The round trip, end to end, with no network: a fixture vault's notes and journal are built into a
//! push, fed straight back as a pull, materialised into an empty folder, and compared.
//!
//! **The notes are compared byte for byte.** The journal is compared as a SET OF RECORDS rather than
//! as file bytes, because `JsonlLedger::append` writes `pystr::NEWLINE` (CRLF on Windows) and the
//! fixture's own day files are whatever they are — and because the records, not the bytes, are what
//! the journal is (`ledger.read` sorts them into one order whatever order they arrived in).
use std::collections::BTreeSet;
use std::io::{BufRead, BufReader, Read, Write};
use std::net::TcpListener;
use std::path::{Path, PathBuf};

use knowlu_engine::cloudmodel::{CloudClient, CloudConfig};
use knowlu_engine::journal::Journal;
use knowlu_engine::sync::{self, Cursor};

// ---------------------------------------------------------------------------
// The loopback harness (fix round 1, I2/I3/I6) — copied from `sync_contract.rs`, which copied it
// from `cloud_contract.rs` (review M7's accepted trade: two forty-line harnesses beat a third
// crate). Every test that uses it binds `127.0.0.1:0`, serves its own request from a second thread,
// and joins that thread before returning — no DNS, no route off the machine.
// ---------------------------------------------------------------------------

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

/// Returns the base URL and a handle whose `join()` yields the raw request text of each reply
/// served, in order — so a caller that cares can assert what went on the wire, and one that does
/// not can simply join it to know the server thread finished cleanly.
fn loopback(replies: Vec<(u16, String)>) -> (String, std::thread::JoinHandle<Vec<String>>) {
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
    (format!("http://127.0.0.1:{port}/functions/v1"), handle)
}

fn cfg(base: &str) -> CloudConfig {
    CloudConfig {
        api_base: base.to_string(),
        anon_key: "anon-not-a-secret".to_string(),
        session_credential_target: "knowlu/test-profile/session".to_string(),
        account_id: "acct-1".to_string(),
    }
}

/// One `/sync-pull` reply body, built from raw records and note rows rather than a round trip —
/// these tests are about an ACCOUNT'S OWN copy, made of writes from devices this vault has never
/// been (unlike `round_trip`, which replays THIS vault's own push).
fn page_reply(records: Vec<(&str, serde_json::Value)>, notes: Vec<serde_json::Value>, record_cursor: i64, note_cursor: i64, more: bool) -> String {
    let records_json: Vec<serde_json::Value> = records
        .into_iter()
        .enumerate()
        .map(|(i, (device, body))| {
            let body_str = knowlu_engine::ledger::dumps_value(&body);
            serde_json::json!({
                "seq": i + 1, "device": device,
                "record_hash": sync::sha256_hex(body_str.as_bytes()),
                "body": body_str
            })
        })
        .collect();
    let body = serde_json::json!({
        "records": records_json, "notes": notes,
        "record_cursor": record_cursor, "note_cursor": note_cursor, "more": more
    });
    knowlu_engine::ledger::dumps_value(&body)
}

/// Everything the source vault would push, read straight back into a `Pulled` — the wire without the
/// wire. The two endpoints are under test in `sync_contract.rs`; this test is about the replay.
fn round_trip(source: &Path) -> sync::Pulled {
    let mut journal = Journal::new(source);
    let (batch, _) = sync::build_push(source, &Cursor::default(), "acct-1", &mut journal);
    assert!(batch.warnings.is_empty(), "{:?}", batch.warnings);
    let body = serde_json::json!({
        "records": batch.records.iter().enumerate().map(|(i, r)| serde_json::json!({
            "seq": i + 1, "device": batch.device, "record_hash": r["hash"], "body": r["body"]
        })).collect::<Vec<_>>(),
        "notes": batch.notes.iter().enumerate().map(|(i, n)| serde_json::json!({
            "rev": i + 1, "device": batch.device, "path": n["path"],
            "deleted": n.get("deleted").cloned().unwrap_or(serde_json::Value::Bool(false)),
            "body": n.get("body").cloned().unwrap_or(serde_json::Value::Null)
        })).collect::<Vec<_>>(),
        "record_cursor": batch.records.len(), "note_cursor": batch.notes.len(), "more": false
    });
    sync::pulled_from_reply(&body).expect("every row reads back")
}

fn records_of(vault: &Path) -> BTreeSet<String> {
    Journal::new(vault).read(None, None).into_iter()
        .map(|r| knowlu_engine::ledger::dumps_value(&serde_json::Value::Object(r))).collect()
}

fn notes_of(vault: &Path) -> Vec<(String, Vec<u8>)> {
    sync::note_paths(vault).into_iter()
        .map(|rel| (rel.clone(), std::fs::read(vault.join(&rel)).expect("read a note"))).collect()
}

#[test]
fn a_fixture_vaults_notes_and_journal_replay_into_an_empty_folder() {
    let source = temp("source");
    copy_tree(Path::new("tests/fixtures/vault-full"), &source);
    let dest = temp("dest");
    std::fs::create_dir_all(&dest).expect("an empty folder");

    let page = round_trip(&source);
    let report = sync::restore(&dest, &page, &[]).expect("restore into an empty folder");

    let before = notes_of(&source);
    let after = notes_of(&dest);
    assert_eq!(before.len(), after.len(), "note count");
    assert_eq!(report.notes, before.len());
    assert!(!report.empty);
    for ((rel_a, bytes_a), (rel_b, bytes_b)) in before.iter().zip(after.iter()) {
        assert_eq!(rel_a, rel_b);
        assert_eq!(bytes_a, bytes_b, "{rel_a} differs after a restore");
    }
    assert_eq!(records_of(&source), records_of(&dest), "every record, as a record");
    let _ = std::fs::remove_dir_all(&source);
    let _ = std::fs::remove_dir_all(&dest);
}

#[test]
fn a_restored_vault_ranks_the_same_day_as_the_one_it_came_from() {
    // Review B3, inherited and unchanged: `state/calendar.md`, `state/events.md` and
    // `state/events-seen.md` are `rank` INPUTS a restore does not carry (a restore carries notes and
    // records), and `golden-today-full.md` proves both reach the rendered day — its line 3 says
    // "2.25h calendar" and its line 5 says "Events: 3 in today's digest". So the comparison copies
    // those three beside `config/` and `profile/`, and the assertion is the whole rendered file.
    let source = temp("rank-source");
    copy_tree(Path::new("tests/fixtures/vault-full"), &source);
    let dest = temp("rank-dest");
    std::fs::create_dir_all(&dest).expect("an empty folder");
    sync::restore(&dest, &round_trip(&source), &[]).expect("restore");
    for rel in ["config", "profile"] { copy_tree(&source.join(rel), &dest.join(rel)); }
    for rel in ["state/calendar.md", "state/events.md", "state/events-seen.md"] {
        let from = source.join(rel);
        if from.exists() {
            std::fs::create_dir_all(dest.join("state")).expect("state");
            std::fs::copy(&from, dest.join(rel)).expect("copy an input a restore does not carry");
        }
    }
    for v in [&source, &dest] {
        knowlu_engine::cli::run(v, Some(knowlu_engine::PINNED_FIXTURE_DATE), "manual", None).expect("rank");
    }
    assert_eq!(
        knowlu_engine::pystr::read_text(&source.join("state").join("today.md")).expect("source today"),
        knowlu_engine::pystr::read_text(&dest.join("state").join("today.md")).expect("dest today"),
        "a restored vault renders the same day",
    );
    let _ = std::fs::remove_dir_all(&source);
    let _ = std::fs::remove_dir_all(&dest);
}

#[test]
fn a_restore_refuses_a_note_it_did_not_put_there_and_tolerates_exactly_the_seeds() {
    // Review B1, inherited. The wizard's vault is seconds old and `scaffold::seed_writes` has
    // already written into it, so a restore that demanded an empty folder could never run from its
    // only caller. The allowlist is what the caller has just written, computed at the moment it runs.
    let dest = temp("allowlist");
    std::fs::create_dir_all(dest.join("tasks")).expect("mkdir");
    knowlu_engine::pystr::write_text(&dest.join("tasks").join("get-to-know-knowlu.md"), "---\nid: task_0000000001\n---\n").expect("seed");
    knowlu_engine::pystr::write_text(&dest.join("tasks").join("someone-elses.md"), "---\nid: task_0000000002\n---\n").expect("stray");
    let page = sync::Pulled::default();
    let err = sync::restore(&dest, &page, &["tasks/get-to-know-knowlu.md".to_string()]).expect_err("a stray note is a refusal");
    assert!(format!("{err}").contains("someone-elses"), "{err}");
    assert!(sync::restore(&dest, &page, &sync::note_paths(&dest)).is_ok(), "everything on disk tolerated is a restore that runs");
    let _ = std::fs::remove_dir_all(&dest);
}

#[test]
fn a_restored_note_overwrites_the_seed_at_the_same_path_and_an_empty_copy_is_not_a_failure() {
    let dest = temp("overwrite");
    std::fs::create_dir_all(dest.join("tasks")).expect("mkdir");
    let seed = dest.join("tasks").join("get-to-know-knowlu.md");
    knowlu_engine::pystr::write_text(&seed, "---\nid: task_0000000001\n---\nthe seed\n").expect("seed");
    let mine = "---\nid: task_0000000009\n---\nfrom the other desktop\n";
    let page = sync::Pulled {
        notes: vec![sync::PulledNote { device: "fedcba9876543210".into(), path: "tasks/get-to-know-knowlu.md".into(), text: Some(mine.into()) }],
        ..Default::default()
    };
    let report = sync::restore(&dest, &page, &sync::note_paths(&dest)).expect("restore over the seed");
    assert_eq!(knowlu_engine::pystr::read_text(&seed).expect("the note"), mine);
    assert!(!report.empty);
    let nothing = sync::restore(&dest, &sync::Pulled::default(), &sync::note_paths(&dest)).expect("an empty copy");
    assert!(nothing.empty, "an empty account copy is a fact, not an error");
    let _ = std::fs::remove_dir_all(&dest);
}

/// I4 (fix round 1, review probe c). The account already deleted the wizard's own welcome task and
/// pushed its own archived copy; this device is restoring for the first time and still has the seed
/// `scaffold::create_vault` just wrote. The tombstone must settle the seed — `apply`'s own tombstone
/// semantics, `write::delete` under `sync::ACTOR` — not leave it live while an unrelated archived
/// copy also lands.
///
/// Fix round 4 (E3): this vault has no `state/seed-hashes.json`, so nothing proves the file is an
/// untouched wizard seed and it is settled through `write::delete`, the fallback kept for exactly that
/// case. With the seed record present, the seed is removed outright instead:
/// `a_seed_the_account_already_settled_is_removed_outright_and_nothing_about_it_is_pushed`.
#[test]
fn a_tombstone_for_a_seed_settles_it_with_apply_s_own_tombstone_semantics() {
    let dest = temp("tombstone-seed");
    std::fs::create_dir_all(dest.join("tasks")).expect("mkdir");
    let seed = dest.join("tasks").join("get-to-know-knowlu.md");
    knowlu_engine::pystr::write_text(&seed, "---\nid: task_0000000001\n---\nthe seed\n").expect("seed");
    let archived_text = "---\nid: task_0000000001\n---\narchived on the other desktop\n";
    let page = sync::Pulled {
        notes: vec![
            sync::PulledNote { device: "fedcba9876543210".into(), path: "tasks/get-to-know-knowlu.md".into(), text: None },
            sync::PulledNote { device: "fedcba9876543210".into(), path: "archive/get-to-know-knowlu.md".into(), text: Some(archived_text.into()) },
        ],
        ..Default::default()
    };
    let tolerate = sync::note_paths(&dest);
    let report = sync::restore(&dest, &page, &tolerate).expect("restore settles the seed");
    assert!(!seed.exists(), "the tombstoned seed is no longer at its old path");
    assert_eq!(
        knowlu_engine::pystr::read_text(&dest.join("archive").join("get-to-know-knowlu.md")).expect("the archived note"),
        archived_text,
        "the account's own archived copy is what ends up there, not the stale local seed"
    );
    assert!(!report.empty);
    let _ = std::fs::remove_dir_all(&dest);
}

/// I5 (fix round 1): the shared filter — `materialise`'s side of it. Neither row is a real push:
/// [`sync::ACTOR`]'s own records never leave the device that made them, and a note that IS one of
/// the account's own amend cards can only ever be answered where it was filed.
#[test]
fn a_restore_refuses_the_accounts_own_sync_actor_record_and_its_live_amend_cards() {
    let dest = temp("restore-sync-affairs");
    std::fs::create_dir_all(&dest).expect("mkdir");
    let hostile_record = serde_json::json!({
        "op": "set", "path": "tasks/x.md", "field": "status", "old": "todo", "new": "done",
        "actor": sync::ACTOR, "via": "dashboard",
        "device": "fedcba9876543210", "ts": "2026-08-01T10:00:00.000Z"
    });
    let card_text = "---\nid: approval_0000000001\ncreated_by: agent:knowlu.sync\n---\n# amend\n";
    let page = sync::Pulled {
        records: vec![("fedcba9876543210".to_string(), hostile_record.as_object().unwrap().clone())],
        notes: vec![sync::PulledNote { device: "fedcba9876543210".into(), path: "approvals/amend-x.md".into(), text: Some(card_text.into()) }],
        ..Default::default()
    };
    let report = sync::restore(&dest, &page, &[]).expect("restore runs; it only refuses the two rows");
    assert!(report.empty, "both rows were refused; nothing landed: {report:?}");
    assert!(!dest.join("tasks").join("x.md").exists());
    assert!(!dest.join("approvals").join("amend-x.md").exists());
    assert!(report.warnings.iter().any(|w| w.contains("sync actor")), "{:?}", report.warnings);
    assert!(report.warnings.iter().any(|w| w.contains("amend card")), "{:?}", report.warnings);
    let _ = std::fs::remove_dir_all(&dest);
}

/// I6 (fix round 1): `restore_all` direct — paging across two pages, and the cursor it leaves.
#[test]
fn restore_all_pages_across_more_than_one_page_and_leaves_a_saved_cursor() {
    let dest = temp("restore-all-pages");
    std::fs::create_dir_all(&dest).expect("mkdir");
    let device = "aaaaaaaaaaaaaaaa";
    let rec1 = serde_json::json!({"op":"create","path":"tasks/a.md","actor":"quinn","via":"dashboard","device":device,"ts":"2026-08-01T10:00:00.000Z","id":"task_0000000001","new":{"id":"task_0000000001"}});
    let page1 = page_reply(
        vec![(device, rec1)],
        vec![serde_json::json!({"path":"tasks/a.md","device":device,"deleted":false,"body":"---\nid: task_0000000001\n---\nfirst page\n"})],
        1, 1, true,
    );
    let rec2 = serde_json::json!({"op":"create","path":"tasks/b.md","actor":"quinn","via":"dashboard","device":device,"ts":"2026-08-01T10:00:01.000Z","id":"task_0000000002","new":{"id":"task_0000000002"}});
    let page2 = page_reply(
        vec![(device, rec2)],
        vec![serde_json::json!({"path":"tasks/b.md","device":device,"deleted":false,"body":"---\nid: task_0000000002\n---\nsecond page\n"})],
        2, 2, false,
    );
    let (base, handle) = loopback(vec![(200, page1), (200, page2)]);
    let client = CloudClient::new(&cfg(&base), "jwt-not-a-secret");
    let report = sync::restore_all(&dest, &client, &[]).expect("two pages restore cleanly");
    handle.join().expect("the loopback thread did not panic");
    assert_eq!((report.notes, report.records), (2, 2), "{report:?}");
    assert!(!report.empty);
    assert!(dest.join("tasks").join("a.md").is_file(), "page 1's note landed");
    assert!(dest.join("tasks").join("b.md").is_file(), "page 2's note landed");
    assert!(dest.join("state").join("sync-cursor.json").is_file(), "the cursor is saved after the last page");
    let _ = std::fs::remove_dir_all(&dest);
}

/// I2 (fix round 1): a failure on the SECOND page must undo the FIRST page's own writes too — not
/// leave a half-filled vault this function then reports as `empty: true`.
#[test]
fn restore_all_rolls_back_every_page_this_call_wrote_when_a_later_page_fails() {
    let dest = temp("restore-all-rollback");
    std::fs::create_dir_all(dest.join("tasks")).expect("mkdir");
    let seed = dest.join("tasks").join("get-to-know-knowlu.md");
    knowlu_engine::pystr::write_text(&seed, "---\nid: task_0000000001\n---\nthe seed\n").expect("seed");
    let before = std::fs::read(&seed).expect("seed bytes");
    let device = "aaaaaaaaaaaaaaaa";
    let rec1 = serde_json::json!({"op":"create","path":"tasks/c.md","actor":"quinn","via":"dashboard","device":device,"ts":"2026-08-01T10:00:00.000Z","id":"task_0000000003","new":{"id":"task_0000000003"}});
    let page1 = page_reply(
        vec![(device, rec1)],
        vec![serde_json::json!({"path":"tasks/c.md","device":device,"deleted":false,"body":"---\nid: task_0000000003\n---\nwill be rolled back\n"})],
        1, 1, true,
    );
    let (base, handle) = loopback(vec![(200, page1), (503, r#"{"error":"down"}"#.to_string())]);
    let client = CloudClient::new(&cfg(&base), "jwt-not-a-secret");
    let tolerate = sync::note_paths(&dest);
    let err = sync::restore_all(&dest, &client, &tolerate).expect_err("page 2's failure fails the whole call");
    handle.join().expect("the loopback thread did not panic");
    assert!(format!("{err}").contains("503") || format!("{err}").contains("refused"), "the cause is named: {err}");
    assert!(!dest.join("tasks").join("c.md").exists(), "page 1's note was rolled back");
    assert_eq!(std::fs::read(&seed).expect("seed bytes"), before, "the untouched seed is unchanged, byte for byte");
    assert!(!dest.join("state").join("sync-cursor.json").exists(), "no cursor is saved on a rolled-back restore");
    let _ = std::fs::remove_dir_all(&dest);
}

/// N5 (fix round 2): the rollback undoes exactly what this call itself wrote — tracked as it
/// happened, never guessed from a whole-tree snapshot. Page 1 writes a note at a NESTED path, which
/// makes `materialise` create a brand-new folder for it; page 2 fails. The rollback must remove the
/// note, prune the folder it made (and only that folder), and never so much as touch an unrelated
/// note sitting elsewhere in the vault.
#[test]
fn restore_all_rollback_removes_only_what_it_wrote_and_prunes_only_folders_it_made() {
    let dest = temp("restore-all-rollback-precise");
    std::fs::create_dir_all(dest.join("tasks")).expect("mkdir");
    let untouched = dest.join("tasks").join("untouched.md");
    knowlu_engine::pystr::write_text(&untouched, "---\nid: task_0000000010\n---\nleave me alone\n").expect("untouched note");
    let before = std::fs::read(&untouched).expect("bytes");

    let device = "aaaaaaaaaaaaaaaa";
    let rec1 = serde_json::json!({"op":"create","path":"tasks/sub/nested.md","actor":"quinn","via":"dashboard","device":device,"ts":"2026-08-01T10:00:00.000Z","id":"task_0000000011","new":{"id":"task_0000000011"}});
    let page1 = page_reply(
        vec![(device, rec1)],
        vec![serde_json::json!({"path":"tasks/sub/nested.md","device":device,"deleted":false,"body":"---\nid: task_0000000011\n---\nnested\n"})],
        1, 1, true,
    );
    let (base, handle) = loopback(vec![(200, page1), (503, r#"{"error":"down"}"#.to_string())]);
    let client = CloudClient::new(&cfg(&base), "jwt-not-a-secret");
    let tolerate = sync::note_paths(&dest);
    let err = sync::restore_all(&dest, &client, &tolerate).expect_err("page 2 fails the whole call");
    handle.join().expect("the loopback thread did not panic");
    assert!(format!("{err}").contains("503") || format!("{err}").contains("refused"), "{err}");
    assert!(!dest.join("tasks").join("sub").join("nested.md").exists(), "the nested note was rolled back");
    assert!(!dest.join("tasks").join("sub").exists(), "the folder this call created for it is pruned too");
    assert!(dest.join("tasks").is_dir(), "the SCAFFOLD's own folder is never pruned");
    assert_eq!(std::fs::read(&untouched).expect("bytes"), before, "an unrelated note is untouched, byte for byte");
    let _ = std::fs::remove_dir_all(&dest);
}

/// R2 (fix round 3). Page 1 carries only a tombstone for the seed; page 2 fails. Before this round,
/// the tombstone was settled the moment page 1 arrived — `write::delete` moved the seed to
/// `archive/` and appended its own `delete` record, and neither the move nor the journal append was
/// ever captured by `Touched`, so a rollback on page 2's failure could undo neither. Now a
/// tombstone is only ever COLLECTED while paging, and settled once, after the whole loop succeeds —
/// so a page 2 failure means it was simply never executed at all: the seed never moved, and no
/// `delete` record was ever journalled.
#[test]
fn a_failed_restore_never_settles_a_tombstone_it_only_collected() {
    let dest = temp("restore-tombstone-never-settled");
    std::fs::create_dir_all(dest.join("tasks")).expect("mkdir");
    let seed = dest.join("tasks").join("get-to-know-knowlu.md");
    knowlu_engine::pystr::write_text(&seed, "---\nid: task_0000000001\n---\nthe seed\n").expect("seed");
    let before = std::fs::read(&seed).expect("seed bytes");

    let page1 = page_reply(
        vec![],
        vec![serde_json::json!({"path":"tasks/get-to-know-knowlu.md","device":"aaaaaaaaaaaaaaaa","deleted":true})],
        0, 1, true,
    );
    let (base, handle) = loopback(vec![(200, page1), (503, r#"{"error":"down"}"#.to_string())]);
    let client = CloudClient::new(&cfg(&base), "jwt-not-a-secret");
    let tolerate = sync::note_paths(&dest);
    let err = sync::restore_all(&dest, &client, &tolerate).expect_err("page 2's failure fails the whole call");
    handle.join().expect("the loopback thread did not panic");
    assert!(format!("{err}").contains("503") || format!("{err}").contains("refused"), "{err}");
    assert!(seed.exists(), "the seed is still live at its own path — the tombstone was never settled");
    assert_eq!(std::fs::read(&seed).expect("seed bytes"), before, "…and it is byte-identical to before");
    assert!(!dest.join("archive").join("get-to-know-knowlu.md").exists(), "nothing was archived");
    let records = Journal::new(&dest).read(None, None);
    assert!(
        !records.iter().any(|r| r.get("op").and_then(|v| v.as_str()) == Some("delete")),
        "no delete record was ever journalled: {records:?}"
    );
    let _ = std::fs::remove_dir_all(&dest);
}

/// I3 (fix round 1): after a completed restore, the cursor left behind must read exactly like a
/// completed pull-then-push already happened — so the very first push after Finish, with nothing
/// else touched, sends nothing at all. Otherwise every restored note re-uploads on the first slot,
/// which (per the fidelity ledger's own S1 argument) risks reverting a concurrent edit made
/// elsewhere in the short window before that upload lands.
#[test]
fn the_first_push_after_a_completed_restore_sends_nothing_from_an_untouched_vault() {
    let dest = temp("restore-then-push");
    std::fs::create_dir_all(&dest).expect("mkdir");
    let device = "aaaaaaaaaaaaaaaa";
    let rec1 = serde_json::json!({"op":"create","path":"tasks/d.md","actor":"quinn","via":"dashboard","device":device,"ts":"2026-08-01T10:00:00.000Z","id":"task_0000000004","new":{"id":"task_0000000004"}});
    let page1 = page_reply(
        vec![(device, rec1)],
        vec![serde_json::json!({"path":"tasks/d.md","device":device,"deleted":false,"body":"---\nid: task_0000000004\n---\nrestored\n"})],
        1, 1, false,
    );
    let (base, handle) = loopback(vec![(200, page1)]);
    let client = CloudClient::new(&cfg(&base), "jwt-not-a-secret");
    let report = sync::restore_all(&dest, &client, &[]).expect("restore completes");
    handle.join().expect("the loopback thread did not panic");
    assert!(!report.empty);

    let cursor = sync::load_cursor(&dest);
    let mut journal = Journal::new(&dest);
    let (batch, _next) = sync::build_push(&dest, &cursor, "acct-1", &mut journal);
    assert!(batch.notes.is_empty(), "no note is unsent after a caught-up restore: {:?}", batch.notes);
    assert!(batch.records.is_empty(), "no record is unsent after a caught-up restore: {:?}", batch.records);
    let _ = std::fs::remove_dir_all(&dest);
}

/// N1 (fix round 2): the round 1 cursor hashed EVERY note on disk and set `pushed_through` to the
/// WHOLE journal's own end — so a course the wizard created before the restore ever ran (the account
/// has never seen it) was marked as already pushed, and its own `create` record never left the
/// device. This is the reviewer's probe: a wizard-made seed the account does not have IS in the
/// first push after a restore, and a restored note is NOT.
#[test]
fn the_first_push_after_a_restore_sends_a_wizard_made_seed_but_not_a_restored_note() {
    let dest = temp("restore-then-push-with-seed");
    std::fs::create_dir_all(dest.join("courses")).expect("mkdir");
    // The wizard's own seed — created locally, journalled the way `scaffold::seed_writes` really
    // does (`write::create`), and never touched again. The account has never seen it.
    let mut seed_journal = Journal::new(&dest);
    let seed_ctx = knowlu_engine::write::WriteContext::new("quinn", "dashboard");
    knowlu_engine::write::create(
        &dest, "courses/new-101.md", "---\nid: course_0000000001\n---\nNew 101\n",
        &seed_ctx, &mut seed_journal, None,
    ).expect("seed the course, the way scaffold does");

    let device = "aaaaaaaaaaaaaaaa";
    let rec1 = serde_json::json!({"op":"create","path":"tasks/e.md","actor":"quinn","via":"dashboard","device":device,"ts":"2026-08-01T10:00:00.000Z","id":"task_0000000005","new":{"id":"task_0000000005"}});
    let page1 = page_reply(
        vec![(device, rec1)],
        vec![serde_json::json!({"path":"tasks/e.md","device":device,"deleted":false,"body":"---\nid: task_0000000005\n---\nrestored\n"})],
        1, 1, false,
    );
    let (base, handle) = loopback(vec![(200, page1)]);
    let client = CloudClient::new(&cfg(&base), "jwt-not-a-secret");
    let tolerate = sync::note_paths(&dest);
    let report = sync::restore_all(&dest, &client, &tolerate).expect("restore completes");
    handle.join().expect("the loopback thread did not panic");
    assert!(!report.empty);

    let cursor = sync::load_cursor(&dest);
    let mut journal = Journal::new(&dest);
    let (batch, _next) = sync::build_push(&dest, &cursor, "acct-1", &mut journal);
    let note_paths: Vec<&str> = batch.notes.iter().map(|n| n["path"].as_str().unwrap_or("")).collect();
    assert!(note_paths.contains(&"courses/new-101.md"), "the wizard's own seed goes out: {note_paths:?}");
    assert!(!note_paths.contains(&"tasks/e.md"), "the restored note must not be re-sent: {note_paths:?}");
    assert!(!batch.records.is_empty(), "the seed's own create record travels too");
    let _ = std::fs::remove_dir_all(&dest);
}

// ---------------------------------------------------------------------------
// Fix round 4 (R-C3′-exec-34): E1, E2, E3 and E4b.
// ---------------------------------------------------------------------------

/// The words the wizard leaves in a seed for the student to replace. A push that carries them has
/// sent a placeholder.
const SEED_PLACEHOLDER: &str = "Fill this in from your syllabus";

/// A wizard seed, written the way `scaffold::seed_writes` writes one: through `write::create`, so its
/// own `create` record is in the journal, with the placeholder body the wizard leaves behind.
fn wizard_seed(dest: &Path, rel: &str, id: &str) {
    let mut journal = Journal::new(dest);
    let ctx = knowlu_engine::write::WriteContext::new("quinn", "dashboard");
    let text = format!("---\nid: {id}\nstatus: active\n---\n\n## Grade weights\n\n{SEED_PLACEHOLDER}.\n");
    knowlu_engine::write::create(dest, rel, &text, &ctx, &mut journal, None).expect("the wizard's own seed");
}

/// `state/seed-hashes.json` exactly as `restore_into` writes it (every note on disk, its path mapped
/// to the sha256 of its raw bytes, through `dumps_value`), for a test that drives `restore_all`
/// directly: `restore_into` cannot reach the account without a stored session.
fn record_seeds(dest: &Path) {
    let mut map = serde_json::Map::new();
    for rel in sync::note_paths(dest) {
        let bytes = std::fs::read(dest.join(&rel)).expect("read a seed");
        map.insert(rel, serde_json::Value::String(sync::sha256_hex(&bytes)));
    }
    std::fs::create_dir_all(dest.join("state")).expect("state/");
    knowlu_engine::pystr::write_text(
        &dest.join(sync::SEED_HASHES_FILE),
        &knowlu_engine::ledger::dumps_value(&serde_json::Value::Object(map)),
    )
    .expect("write state/seed-hashes.json");
}

/// The JSON body of one request the loopback served.
fn body_of(request: &str) -> serde_json::Value {
    let body = request.split_once("\r\n\r\n").map(|(_, b)| b).unwrap_or_default();
    serde_json::from_str(body).unwrap_or(serde_json::Value::Null)
}

/// Every note path one `/sync-push` request carried, live or tombstone.
fn pushed_paths(request: &str) -> Vec<String> {
    body_of(request)["notes"]
        .as_array()
        .map(|notes| notes.iter().filter_map(|n| n["path"].as_str().map(str::to_string)).collect())
        .unwrap_or_default()
}

/// One slot's sync, both halves, against the loopback — the same call `run_lines_with` makes once it
/// has resolved a session.
fn one_sync(dest: &Path, client: &CloudClient, cloud: &CloudConfig) {
    let (lines, totals) = sync::run_lines_with_client(
        dest, sync::Direction::Both, "cli", None, client, cloud, Vec::new(), sync::Totals::default(),
    );
    assert!(totals.errors.is_empty(), "{lines:?}");
}

/// The wizard's vault after a restore that failed at Finish: the seeds, `state/seed-hashes.json`
/// written by `restore_into` itself, and no cursor. No `config/cloud.yaml` here, so `resolve` fails,
/// which is the same fold a dead network takes (`ok: false`, and ordinary syncs fill the vault).
fn a_failed_restore(tag: &str) -> PathBuf {
    let dest = temp(tag);
    std::fs::create_dir_all(&dest).expect("mkdir");
    wizard_seed(&dest, "courses/cs-100.md", "course_0000000001");
    let failed = sync::restore_into(&dest).expect("a failed restore is a fold, never an Err");
    assert!(!failed.ok, "{failed:?}");
    assert!(dest.join(sync::SEED_HASHES_FILE).is_file(), "restore_into recorded the seeds first");
    assert!(!dest.join(sync::CURSOR_FILE).exists(), "a failed restore saves no cursor");
    dest
}

const PUSH_OK: &str = r#"{"records":0,"notes":0}"#;

/// E1 (R-C3′-exec-34), the reviewer's probe P6. Finish's restore failed, so the vault holds only the
/// wizard's seeds. The account's own `courses/cs-100.md` (real grade weights, typed on another
/// desktop) sits on its SECOND page. The first slot pulls page 1 only (`more: true`), and before this
/// round its push sent the untouched seed over the account's copy. Now the seed waits until a pull has
/// reached the end, the second slot's pull replaces it with the account's copy, and no push ever
/// carries the placeholder.
#[test]
fn after_a_failed_restore_an_untouched_seed_waits_until_a_pull_has_reached_the_end() {
    let dest = a_failed_restore("e1-seed-waits");
    let device = "aaaaaaaaaaaaaaaa";
    let page1 = page_reply(
        vec![],
        vec![serde_json::json!({"path":"tasks/other.md","device":device,"deleted":false,"body":"---\nid: task_0000000020\n---\nsomething else\n"})],
        0, 1, true,
    );
    let account_copy = "---\nid: course_00000000ab\nstatus: active\n---\n\n## Grade weights\n\nExams 60%, labs 40%\n";
    let page2 = page_reply(
        vec![],
        vec![serde_json::json!({"path":"courses/cs-100.md","device":device,"deleted":false,"body":account_copy})],
        0, 2, false,
    );
    let (base, handle) = loopback(vec![(200, page1), (200, PUSH_OK.into()), (200, page2), (200, PUSH_OK.into())]);
    let cloud = cfg(&base);
    let client = CloudClient::new(&cloud, "jwt-not-a-secret");
    one_sync(&dest, &client, &cloud);
    one_sync(&dest, &client, &cloud);
    let requests = handle.join().expect("the loopback thread did not panic");
    assert_eq!(requests.len(), 4, "two pulls and two pushes");
    let first_push = pushed_paths(&requests[1]);
    assert!(!first_push.contains(&"courses/cs-100.md".to_string()), "the first slot held the seed back: {first_push:?}");
    assert!(requests[1].starts_with("POST /functions/v1/sync-push "), "the first slot did push: {}", requests[1]);
    assert!(!body_of(&requests[1]).is_null(), "and its body reads: {}", requests[1]);
    assert_eq!(
        knowlu_engine::pystr::read_text(&dest.join("courses").join("cs-100.md")).expect("the course"),
        account_copy,
        "the second slot's pull replaced the seed with the account's copy"
    );
    for (i, request) in requests.iter().enumerate() {
        assert!(!request.contains(SEED_PLACEHOLDER), "request {i} carried the seed's placeholder:\n{request}");
    }
    let _ = std::fs::remove_dir_all(&dest);
}

/// E1's other half: a seed the account never had is this desktop's own truth, and once a pull has
/// reached `more: false` it is pushed like any other local note, by the slot that got there.
#[test]
fn once_a_pull_has_reached_the_end_a_seed_the_account_never_had_is_pushed() {
    let dest = a_failed_restore("e1-seed-pushed");
    let device = "aaaaaaaaaaaaaaaa";
    let page1 = page_reply(
        vec![],
        vec![serde_json::json!({"path":"tasks/other.md","device":device,"deleted":false,"body":"---\nid: task_0000000021\n---\nsomething else\n"})],
        0, 1, true,
    );
    let page2 = page_reply(vec![], vec![], 0, 1, false);
    let (base, handle) = loopback(vec![(200, page1), (200, PUSH_OK.into()), (200, page2), (200, PUSH_OK.into())]);
    let cloud = cfg(&base);
    let client = CloudClient::new(&cloud, "jwt-not-a-secret");
    one_sync(&dest, &client, &cloud);
    one_sync(&dest, &client, &cloud);
    let requests = handle.join().expect("the loopback thread did not panic");
    assert_eq!(requests.len(), 4, "two pulls and two pushes");
    assert!(!pushed_paths(&requests[1]).contains(&"courses/cs-100.md".to_string()), "held back before the end: {:?}", pushed_paths(&requests[1]));
    let second_push = pushed_paths(&requests[3]);
    assert!(second_push.contains(&"courses/cs-100.md".to_string()), "pushed once a pull reached the end: {second_push:?}");
    assert!(requests[3].contains(SEED_PLACEHOLDER), "the seed's own text is what goes up");
    let _ = std::fs::remove_dir_all(&dest);
}

/// E1's guard on the success path (not a RED item: it pins that the hold-back never outlives a
/// restore that worked). A restore that paged to `more: false` IS a pull that reached the end, so
/// the first push after it sends a seed the account never had: N1's promise, now with
/// `state/seed-hashes.json` present, as it always is after `restore_into`.
#[test]
fn a_completed_restore_counts_as_a_pull_that_reached_the_end() {
    let dest = temp("e1-completed-restore");
    std::fs::create_dir_all(&dest).expect("mkdir");
    wizard_seed(&dest, "courses/cs-100.md", "course_0000000001");
    record_seeds(&dest);
    let device = "aaaaaaaaaaaaaaaa";
    let page = page_reply(
        vec![],
        vec![serde_json::json!({"path":"tasks/other.md","device":device,"deleted":false,"body":"---\nid: task_0000000022\n---\nsomething else\n"})],
        0, 1, false,
    );
    let (base, handle) = loopback(vec![(200, page)]);
    let client = CloudClient::new(&cfg(&base), "jwt-not-a-secret");
    let tolerate = sync::note_paths(&dest);
    sync::restore_all(&dest, &client, &tolerate).expect("the restore completes");
    handle.join().expect("the loopback thread did not panic");
    let mut journal = Journal::new(&dest);
    let (batch, _) = sync::build_push(&dest, &sync::load_cursor(&dest), "acct-1", &mut journal);
    let paths: Vec<&str> = batch.notes.iter().map(|n| n["path"].as_str().unwrap_or("")).collect();
    assert!(paths.contains(&"courses/cs-100.md"), "the seed the account never had goes out: {paths:?}");
    assert!(!paths.contains(&"tasks/other.md"), "the restored note does not: {paths:?}");
    let _ = std::fs::remove_dir_all(&dest);
}

/// E2 (R-C3′-exec-34), the reviewer's probe P7. Page 1 carries a tombstone for the welcome seed's
/// path; page 2 a live row at the same path, because the account re-created the note while this
/// restore was paging. The later row wins: the live copy stays at its path, nothing is archived, and
/// the first push does not send it back.
#[test]
fn a_live_row_on_a_later_page_outranks_a_tombstone_an_earlier_page_collected() {
    let dest = temp("e2-later-live-row");
    std::fs::create_dir_all(&dest).expect("mkdir");
    wizard_seed(&dest, "tasks/get-to-know-knowlu.md", "task_0000000001");
    record_seeds(&dest);
    let device = "aaaaaaaaaaaaaaaa";
    let path = "tasks/get-to-know-knowlu.md";
    let page1 = page_reply(vec![], vec![serde_json::json!({"path":path,"device":device,"deleted":true})], 0, 1, true);
    let live = "---\nid: task_00000000bb\nstatus: active\n---\n\nre-created on the other desktop\n";
    let page2 = page_reply(vec![], vec![serde_json::json!({"path":path,"device":device,"deleted":false,"body":live})], 0, 2, false);
    let (base, handle) = loopback(vec![(200, page1), (200, page2)]);
    let client = CloudClient::new(&cfg(&base), "jwt-not-a-secret");
    let tolerate = sync::note_paths(&dest);
    sync::restore_all(&dest, &client, &tolerate).expect("the restore completes");
    handle.join().expect("the loopback thread did not panic");
    assert_eq!(
        knowlu_engine::pystr::read_text(&dest.join(path)).expect("the live note is at its path"),
        live,
        "the later live row wins over the earlier tombstone"
    );
    assert!(!dest.join("archive").join("get-to-know-knowlu.md").exists(), "nothing was archived");
    let mut journal = Journal::new(&dest);
    let (batch, _) = sync::build_push(&dest, &sync::load_cursor(&dest), "acct-1", &mut journal);
    let paths: Vec<&str> = batch.notes.iter().map(|n| n["path"].as_str().unwrap_or("")).collect();
    assert!(!paths.iter().any(|p| p.contains("get-to-know-knowlu")), "the account's own copy is not sent back: {paths:?}");
    let _ = std::fs::remove_dir_all(&dest);
}

/// E3 (R-C3′-exec-34), the reviewer's probe P8: the usual second desktop. The student finished the
/// welcome task on another desktop, so the account holds a tombstone for
/// `tasks/get-to-know-knowlu.md` and that desktop's own archived copy at
/// `archive/get-to-know-knowlu.md`. This desktop's wizard has just made its own welcome seed at the
/// same path. The account's tombstone settles that path, so the seed (a placeholder with no history
/// worth keeping) is removed outright. It is never archived as a second copy
/// (`archive/get-to-know-knowlu-2.md`, which the first push used to send to every desktop), its
/// seed-hashes entry goes with it, and nothing about it is in the first push. A seed the account never
/// had still goes up as usual.
#[test]
fn a_seed_the_account_already_settled_is_removed_outright_and_nothing_about_it_is_pushed() {
    let dest = temp("e3-seed-removed");
    std::fs::create_dir_all(&dest).expect("mkdir");
    let seed_id = "task_0000000001";
    wizard_seed(&dest, "tasks/get-to-know-knowlu.md", seed_id);
    wizard_seed(&dest, "courses/cs-100.md", "course_0000000001");
    record_seeds(&dest);

    let device = "aaaaaaaaaaaaaaaa";
    let theirs = "task_00000000aa";
    let created = serde_json::json!({"op":"create","path":"tasks/get-to-know-knowlu.md","actor":"quinn","via":"dashboard","device":device,"ts":"2026-09-01T10:00:00.000Z","id":theirs,"new":{"id":theirs}});
    let finished = serde_json::json!({"op":"delete","path":"tasks/get-to-know-knowlu.md","actor":"quinn","via":"dashboard","device":device,"ts":"2026-09-02T10:00:00.000Z","id":theirs,"old":"tasks/get-to-know-knowlu.md","new":"archive/get-to-know-knowlu.md"});
    let archived = format!("---\nid: {theirs}\nstatus: done\n---\n\nfinished on the other desktop\n");
    let page = page_reply(
        vec![(device, created), (device, finished)],
        vec![
            serde_json::json!({"path":"tasks/get-to-know-knowlu.md","device":device,"deleted":true}),
            serde_json::json!({"path":"archive/get-to-know-knowlu.md","device":device,"deleted":false,"body":archived}),
        ],
        2, 2, false,
    );
    let (base, handle) = loopback(vec![(200, page)]);
    let client = CloudClient::new(&cfg(&base), "jwt-not-a-secret");
    let tolerate = sync::note_paths(&dest);
    sync::restore_all(&dest, &client, &tolerate).expect("the restore completes");
    handle.join().expect("the loopback thread did not panic");

    assert!(!dest.join("tasks").join("get-to-know-knowlu.md").exists(), "the seed is gone from its path");
    assert_eq!(
        knowlu_engine::pystr::read_text(&dest.join("archive").join("get-to-know-knowlu.md")).expect("the archived note"),
        archived,
        "the account's archived copy is the one archived copy"
    );
    assert!(!dest.join("archive").join("get-to-know-knowlu-2.md").exists(), "the seed was not archived as a second copy");
    let hashes = knowlu_engine::pystr::read_text(&dest.join(sync::SEED_HASHES_FILE)).expect("seed-hashes.json");
    assert!(!hashes.contains("tasks/get-to-know-knowlu.md"), "its seed-hashes entry went with it: {hashes}");
    assert!(hashes.contains("courses/cs-100.md"), "the other seed's entry stays: {hashes}");
    let records = Journal::new(&dest).read(None, None);
    assert!(
        !records.iter().any(|r| r.get("op").and_then(|v| v.as_str()) == Some("delete") && r.get("id").and_then(|v| v.as_str()) == Some(seed_id)),
        "removed outright, not settled through write::delete: {records:?}"
    );

    let mut journal = Journal::new(&dest);
    let (batch, _) = sync::build_push(&dest, &sync::load_cursor(&dest), "acct-1", &mut journal);
    let paths: Vec<&str> = batch.notes.iter().map(|n| n["path"].as_str().unwrap_or("")).collect();
    assert!(!paths.iter().any(|p| p.contains("get-to-know-knowlu")), "no note row about the seed: {paths:?}");
    let bodies: Vec<String> = batch.records.iter().map(|r| r["body"].as_str().unwrap_or("").to_string()).collect();
    assert!(!bodies.iter().any(|b| b.contains(seed_id)), "no record about the seed: {bodies:?}");
    assert!(paths.contains(&"courses/cs-100.md"), "a seed the account never had still goes: {paths:?}");
    let _ = std::fs::remove_dir_all(&dest);
}

/// E4b (R-C3′-exec-34). `restore_into` lists every note on disk as a seed, so it may only ever run
/// on a vault the wizard has just made. Anything else is a named refusal, before anything is written:
/// a note with no record of being made with the vault, a journal that already holds an edit, or a
/// vault that has restored or synced before.
#[test]
fn restore_into_refuses_a_vault_that_is_not_fresh() {
    // A note that was not made with this vault.
    let stray = temp("e4b-stray-note");
    std::fs::create_dir_all(stray.join("tasks")).expect("mkdir");
    wizard_seed(&stray, "courses/cs-100.md", "course_0000000001");
    knowlu_engine::pystr::write_text(&stray.join("tasks").join("mine.md"), "---\nid: task_0000000030\n---\nmine\n").expect("a note of my own");
    let err = sync::restore_into(&stray).expect_err("a vault holding a note the wizard did not make is refused");
    assert!(err.contains("tasks/mine.md"), "the refusal names the note: {err}");
    assert!(!stray.join(sync::SEED_HASHES_FILE).exists(), "nothing was written: {err}");

    // A journal that already records an edit.
    let edited = temp("e4b-edited");
    std::fs::create_dir_all(&edited).expect("mkdir");
    wizard_seed(&edited, "courses/cs-100.md", "course_0000000001");
    let mut journal = Journal::new(&edited);
    let ctx = knowlu_engine::write::WriteContext::new("quinn", "dashboard");
    knowlu_engine::write::write_literals(&edited, "courses/cs-100.md", &[("status".to_string(), "done".to_string())], &ctx, &mut journal, &Default::default())
        .expect("an edit, journalled");
    let err = sync::restore_into(&edited).expect_err("a vault whose journal holds an edit is refused");
    assert!(err.contains("set") && err.contains("courses/cs-100.md"), "the refusal names the edit: {err}");
    assert!(!edited.join(sync::SEED_HASHES_FILE).exists(), "nothing was written: {err}");

    // A vault that has restored before.
    let again = temp("e4b-restored-before");
    std::fs::create_dir_all(&again).expect("mkdir");
    wizard_seed(&again, "courses/cs-100.md", "course_0000000001");
    record_seeds(&again);
    let before = std::fs::read(again.join(sync::SEED_HASHES_FILE)).expect("bytes");
    let err = sync::restore_into(&again).expect_err("a vault that has restored before is refused");
    assert!(err.contains(sync::SEED_HASHES_FILE), "the refusal names what gave it away: {err}");
    assert_eq!(std::fs::read(again.join(sync::SEED_HASHES_FILE)).expect("bytes"), before, "and it is not rewritten");

    // The wizard's own vault is fresh: the same seeds, nothing else, and the fold as before.
    let fresh = temp("e4b-fresh");
    std::fs::create_dir_all(&fresh).expect("mkdir");
    wizard_seed(&fresh, "courses/cs-100.md", "course_0000000001");
    wizard_seed(&fresh, "tasks/get-to-know-knowlu.md", "task_0000000001");
    assert!(sync::restore_into(&fresh).is_ok(), "a vault the wizard has just made is fresh");
    for d in [&stray, &edited, &again, &fresh] {
        let _ = std::fs::remove_dir_all(d);
    }
}

fn temp(tag: &str) -> PathBuf {
    let d = std::env::temp_dir().join(format!("knowlu-sync-replay-{tag}-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&d);
    d
}

fn copy_tree(from: &Path, to: &Path) {
    std::fs::create_dir_all(to).expect("mkdir");
    for entry in std::fs::read_dir(from).expect("read_dir").flatten() {
        let (p, t) = (entry.path(), to.join(entry.file_name()));
        if p.is_dir() {
            copy_tree(&p, &t);
        } else {
            std::fs::copy(&p, &t).expect("copy");
        }
    }
}

/// C3′'s final fix wave, the final review's I1 on the restore path: `fold_confirmed` moved
/// `pushed_through` to every restored record's own `ts`, so a record another desktop stamped in this
/// desktop's future left the cursor ahead of this desktop's clock — and the first push after the
/// restore dropped every local write older than it (`ledger::read(since)`), for good. The cursor a
/// restore leaves is now never past the restore's own clock; the future record is simply re-sent
/// (idempotently) by a later push until the clock passes it.
#[test]
fn a_restored_record_dated_in_this_desktops_future_never_hides_the_first_local_write() {
    let dest = temp("restore-future");
    std::fs::create_dir_all(&dest).expect("mkdir");
    let device = "aaaaaaaaaaaaaaaa";
    let ahead = knowlu_engine::journal::now_ts(Some(jiff::Timestamp::now() + jiff::SignedDuration::from_hours(1)));
    let past = serde_json::json!({"op":"create","path":"tasks/a.md","actor":"quinn","via":"dashboard","device":"SlowClockDesktop","ts":"2026-08-01T10:00:00.000Z","id":"task_0000000001","new":{"id":"task_0000000001"}});
    let future = serde_json::json!({"op":"create","path":"tasks/b.md","actor":"quinn","via":"dashboard","device":"FastClockDesktop","ts":ahead,"id":"task_0000000002","new":{"id":"task_0000000002"}});
    let page = page_reply(
        vec![(device, past), (device, future)],
        vec![
            serde_json::json!({"path":"tasks/a.md","device":device,"deleted":false,"body":"---\nid: task_0000000001\n---\nfrom the past\n"}),
            serde_json::json!({"path":"tasks/b.md","device":device,"deleted":false,"body":"---\nid: task_0000000002\n---\nfrom a clock an hour ahead\n"}),
        ],
        2, 2, false,
    );
    let (base, handle) = loopback(vec![(200, page)]);
    let client = CloudClient::new(&cfg(&base), "jwt-not-a-secret");
    sync::restore_all(&dest, &client, &[]).expect("the restore completes");
    handle.join().expect("the loopback thread did not panic");

    let cursor = sync::load_cursor(&dest);
    let now = knowlu_engine::journal::now_ts(None);

    let mut journal = Journal::new(&dest);
    let mine = knowlu_engine::write::WriteContext::new("quinn", "dashboard");
    knowlu_engine::write::write_literals(&dest, "tasks/a.md", &[("importance".to_string(), "4".to_string())], &mine, &mut journal, &Default::default()).expect("the first local write");
    journal.invalidate();
    let (batch, _) = sync::build_push(&dest, &cursor, "acct-1", &mut journal);
    let sent: Vec<serde_json::Value> = batch.records.iter()
        .map(|r| serde_json::from_str(r["body"].as_str().expect("a body")).expect("a record"))
        .collect();
    assert!(
        sent.iter().any(|r| r["op"] == "set" && r["field"] == "importance" && r["new"] == serde_json::json!(4)),
        "the first write after a restore must reach the account: {sent:?}"
    );
    // The mechanism, pinned beside the outcome.
    assert!(cursor.pushed_through.as_str() <= now.as_str(), "the restore's cursor ran ahead of this desktop's clock: {} > {now}", cursor.pushed_through);
    let _ = std::fs::remove_dir_all(&dest);
}
