//! Phase 2 (spec `docs/specs/2026-09-24-commitment-model-phase2-design.md` §3):
//! `knowlu-engine commitments --confirm`, run as the binary. Every key is invented.

use std::path::{Path, PathBuf};
use std::process::{Command, Output};

fn binary() -> PathBuf {
    PathBuf::from(env!("CARGO_BIN_EXE_knowlu-engine"))
}

fn scratch(name: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!("knowlu-confirm-{}-{name}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(dir.join("config")).unwrap();
    dir
}

/// The input goes in a file beside the vault, never inside it, so the tree below is the vault's.
fn run(vault: &Path, text: &str) -> Output {
    let file = vault.with_extension("confirm.json");
    std::fs::write(&file, text).unwrap();
    let out = Command::new(binary())
        .args(["commitments", "--vault"])
        .arg(vault)
        .args(["--today", "2026-09-24", "--confirm"])
        .arg(&file)
        .output()
        .unwrap();
    let _ = std::fs::remove_file(&file);
    out
}

/// Every file under the vault, relative, sorted.
fn tree(vault: &Path) -> Vec<String> {
    fn walk(dir: &Path, root: &Path, out: &mut Vec<String>) {
        for entry in std::fs::read_dir(dir).into_iter().flatten().flatten() {
            let path = entry.path();
            if path.is_dir() {
                walk(&path, root, out);
            } else {
                out.push(path.strip_prefix(root).unwrap().to_string_lossy().replace('\\', "/"));
            }
        }
    }
    let mut out = Vec::new();
    walk(vault, vault, &mut out);
    out.sort();
    out
}

#[test]
fn an_invalid_window_writes_nothing_and_exits_2() {
    let v = scratch("badwindow");
    let before = tree(&v);
    let out = run(&v, r#"{"window": "[{days: [mon], start: \"15:00\", end: \"14:00\"}]"}"#);
    assert_eq!(out.status.code(), Some(2), "{out:?}");
    assert!(String::from_utf8_lossy(&out.stderr).contains("planning day mon"), "{out:?}");
    assert!(out.stdout.is_empty());
    assert_eq!(tree(&v), before);
    let _ = std::fs::remove_dir_all(&v);
}

#[test]
fn unreadable_input_exits_2() {
    let v = scratch("unreadable");
    assert_eq!(run(&v, "not json").status.code(), Some(2));
    let missing = Command::new(binary())
        .args(["commitments", "--vault"])
        .arg(&v)
        .args(["--confirm"])
        .arg(v.join("no-such-file.json"))
        .output()
        .unwrap();
    assert_eq!(missing.status.code(), Some(2));
    assert!(!tree(&v).iter().any(|f| f.starts_with("commitments/") || f.starts_with("state/")), "{:?}", tree(&v));
    let _ = std::fs::remove_dir_all(&v);
}

#[test]
fn a_window_prints_the_report_and_journals_the_human() {
    let v = scratch("report");
    let out = run(&v, r#"{"window": "[{days: [mon, tue, wed, thu, fri], start: \"08:00\", end: \"22:00\"}]"}"#);
    assert!(out.status.success(), "{out:?}");
    assert_eq!(
        String::from_utf8_lossy(&out.stdout),
        "{\"created\": 0, \"declined\": 0, \"warnings\": [], \"window\": \"created\"}\n"
    );
    assert!(v.join("commitments").join("planning-day.md").is_file());
    let journal: String = std::fs::read_dir(v.join("state").join("journal"))
        .unwrap()
        .flatten()
        .map(|e| std::fs::read_to_string(e.path()).unwrap())
        .collect();
    assert!(journal.contains("\"actor\": \"quinn\"") && journal.contains("\"via\": \"dashboard\""), "{journal}");
    let _ = std::fs::remove_dir_all(&v);
}

/// Spec §3: `--confirm` makes no network call. The vault names a feed on a loopback listener
/// that accepts nothing; after the command, no connection is waiting.
#[test]
fn confirm_makes_no_network_call() {
    let v = scratch("nonet");
    let listener = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
    listener.set_nonblocking(true).unwrap();
    let port = listener.local_addr().unwrap().port();
    std::fs::write(
        v.join("config").join("ingest.yaml"),
        format!("timezone: America/Chicago\ncalendars:\n  - name: personal\n    ics_url: http://127.0.0.1:{port}/a.ics\n"),
    )
    .unwrap();
    let out = run(
        &v,
        r#"{"mine": [{"source_uid": "gcal-series:x", "level": "hard"}], "not_mine": ["gcal-series:y"], "window": "[{days: [mon], start: \"08:00\", end: \"22:00\"}]"}"#,
    );
    assert!(out.status.success(), "{out:?}");
    match listener.accept() {
        Err(e) if e.kind() == std::io::ErrorKind::WouldBlock => {}
        other => panic!("--confirm opened a connection: {other:?}"),
    }
    let _ = std::fs::remove_dir_all(&v);
}
