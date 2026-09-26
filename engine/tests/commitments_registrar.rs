//! Phase 3 (spec `docs/specs/2026-09-26-commitment-model-phase3-design.md` §3):
//! `knowlu-engine commitments --registrar`, run as the binary. Every course is invented.

use std::path::{Path, PathBuf};
use std::process::{Command, Output};

const FIXTURE: &str = include_str!("fixtures/registrar/banner-ua-registration.json");

fn binary() -> PathBuf {
    PathBuf::from(env!("CARGO_BIN_EXE_knowlu-engine"))
}

fn scratch(name: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!("knowlu-registrar-{}-{name}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(dir.join("config")).unwrap();
    std::fs::create_dir_all(dir.join("courses")).unwrap();
    std::fs::write(dir.join("courses").join("cs-100.md"), "---\ntitle: \"CS 100 Invented Computing\"\ncode: \"CS 100\"\n---\n").unwrap();
    dir
}

/// The input sits beside the vault, never inside it, so the tree below is the vault's.
fn run(vault: &Path, text: &str, extra: &[&str]) -> Output {
    let file = vault.with_extension("registrar.json");
    std::fs::write(&file, text).unwrap();
    let out = Command::new(binary())
        .args(["commitments", "--vault"])
        .arg(vault)
        .args(["--today", "2026-09-01", "--registrar"])
        .arg(&file)
        .args(extra)
        .output()
        .unwrap();
    let _ = std::fs::remove_file(&file);
    out
}

/// Every file under the vault, relative, sorted (`commitments_confirm.rs`'s helper).
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
fn a_fetch_prints_the_report_and_writes_the_series_and_the_notes() {
    let v = scratch("ok");
    let out = run(&v, FIXTURE, &["--school", "ua"]);
    assert!(out.status.success(), "{out:?}");
    let report: serde_json::Value = serde_json::from_slice(&out.stdout).unwrap();
    assert_eq!(report["term"], "202640");
    assert_eq!((report["rows"].as_u64(), report["confirmed"].as_u64(), report["proposed"].as_u64()), (Some(4), Some(2), Some(2)));
    assert_eq!(report["dropped"], serde_json::json!({ "midnight": 1, "no_time": 1 }));
    let files = tree(&v);
    for f in ["state/calendar-series.json", "commitments/cs-100.md", "commitments/cs-100-lab.md"] {
        assert!(files.iter().any(|x| x == f), "{f} not written: {files:?}");
    }
    let _ = std::fs::remove_dir_all(&v);
}

#[test]
fn every_exit_2_writes_nothing() {
    for (name, text, extra) in [
        ("school", FIXTURE, vec!["--school", "zz"]),
        ("html", "<!DOCTYPE html><html>Sign in</html>", vec!["--school", "ua"]),
        ("empty", "{\"data\": []}", vec!["--school", "ua"]),
        ("noschool", FIXTURE, vec![]),
        ("both", FIXTURE, vec!["--school", "ua", "--confirm", "x.json"]),
    ] {
        let v = scratch(name);
        let before = tree(&v);
        let out = run(&v, text, &extra);
        assert_eq!(out.status.code(), Some(2), "{name}: {out:?}");
        assert!(out.stdout.is_empty(), "{name}");
        assert_eq!(tree(&v), before, "{name} wrote something");
        let _ = std::fs::remove_dir_all(&v);
    }
}

/// Spec §3: `--registrar` makes no network call. The vault names a feed on a loopback listener
/// that accepts nothing; after the command, no connection is waiting.
#[test]
fn registrar_makes_no_network_call() {
    let v = scratch("nonet");
    let listener = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
    listener.set_nonblocking(true).unwrap();
    let port = listener.local_addr().unwrap().port();
    std::fs::write(
        v.join("config").join("ingest.yaml"),
        format!("timezone: America/Chicago\ncalendars:\n  - name: personal\n    ics_url: http://127.0.0.1:{port}/a.ics\n"),
    )
    .unwrap();
    std::fs::write(v.join("config").join("cloud.yaml"), format!("api_base: http://127.0.0.1:{port}/functions/v1\n")).unwrap();
    assert!(run(&v, FIXTURE, &["--school", "ua"]).status.success());
    match listener.accept() {
        Err(e) if e.kind() == std::io::ErrorKind::WouldBlock => {}
        other => panic!("--registrar opened a connection: {other:?}"),
    }
    let _ = std::fs::remove_dir_all(&v);
}
