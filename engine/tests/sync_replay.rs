//! The round trip, end to end, with no network: a fixture vault's notes and journal are built into a
//! push, fed straight back as a pull, materialised into an empty folder, and compared.
//!
//! **The notes are compared byte for byte.** The journal is compared as a SET OF RECORDS rather than
//! as file bytes, because `JsonlLedger::append` writes `pystr::NEWLINE` (CRLF on Windows) and the
//! fixture's own day files are whatever they are — and because the records, not the bytes, are what
//! the journal is (`ledger.read` sorts them into one order whatever order they arrived in).
use std::collections::BTreeSet;
use std::path::{Path, PathBuf};

use knowlu_engine::journal::Journal;
use knowlu_engine::sync::{self, Cursor};

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
