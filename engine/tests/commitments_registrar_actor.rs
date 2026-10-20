//! Ruling 11 (close-out plan B1a, A1-F2): `commitments --registrar` writes R24's notes as the
//! vault's own human token (`config/actor.yaml`; absent means the legacy token), never a fixed
//! name. Run as the binary, on scratch vaults only; every course is invented.

use std::path::{Path, PathBuf};
use std::process::{Command, Output};

const FIXTURE: &str = include_str!("fixtures/registrar/banner-ua-registration.json");

fn binary() -> PathBuf {
    PathBuf::from(env!("CARGO_BIN_EXE_knowlu-engine"))
}

/// A scratch vault whose only course is CS 100. `actor` is `config/actor.yaml`'s text, or `None`
/// for a legacy vault that has no such file.
fn scratch(name: &str, actor: Option<&str>) -> PathBuf {
    let dir = std::env::temp_dir().join(format!("knowlu-registrar-actor-{}-{name}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(dir.join("config")).unwrap();
    std::fs::create_dir_all(dir.join("courses")).unwrap();
    std::fs::write(dir.join("courses").join("cs-100.md"), "---\ntitle: \"CS 100 Invented Computing\"\ncode: \"CS 100\"\n---\n").unwrap();
    if let Some(text) = actor {
        std::fs::write(dir.join("config").join("actor.yaml"), text).unwrap();
    }
    dir
}

/// The input sits beside the vault, never inside it, so the tree below is the vault's.
fn run(vault: &Path, extra: &[&str]) -> Output {
    let file = vault.with_extension("registrar.json");
    std::fs::write(&file, FIXTURE).unwrap();
    let out = Command::new(binary())
        .args(["commitments", "--vault"])
        .arg(vault)
        .args(["--today", "2026-09-01", "--registrar"])
        .arg(&file)
        .args(["--school", "ua"])
        .args(extra)
        .output()
        .unwrap();
    let _ = std::fs::remove_file(&file);
    out
}

/// Every journal line of the vault, as raw text, in file order.
fn journal_text(vault: &Path) -> String {
    let Ok(dir) = std::fs::read_dir(vault.join("state").join("journal")) else { return String::new() };
    let mut files: Vec<PathBuf> = dir.flatten().map(|e| e.path()).collect();
    files.sort();
    files.iter().map(|f| std::fs::read_to_string(f).unwrap()).collect()
}

fn actors(vault: &Path) -> Vec<String> {
    journal_text(vault)
        .lines()
        .map(|l| serde_json::from_str::<serde_json::Value>(l).unwrap()["actor"].as_str().unwrap().to_string())
        .collect()
}

/// Every file under the vault with its bytes - the "nothing written" oracle.
fn fingerprint(vault: &Path) -> Vec<(PathBuf, Vec<u8>)> {
    fn walk(dir: &Path, out: &mut Vec<(PathBuf, Vec<u8>)>) {
        for entry in std::fs::read_dir(dir).into_iter().flatten().flatten() {
            let path = entry.path();
            if path.is_dir() { walk(&path, out) } else { out.push((path.clone(), std::fs::read(&path).unwrap())) }
        }
    }
    let mut out = Vec::new();
    walk(vault, &mut out);
    out.sort();
    out
}

#[test]
fn a_new_vault_writes_every_r24_record_as_student() {
    let v = scratch("student", Some("human_actor: student\n"));
    let out = run(&v, &[]);
    assert!(out.status.success(), "{out:?}");
    assert!(v.join("commitments").join("cs-100-lab.md").exists(), "R24 wrote the matched rows");
    let all = actors(&v);
    assert!(!all.is_empty() && all.iter().all(|a| a == "student"), "{all:?}");
    assert!(!journal_text(&v).contains("quinn"), "no journal record of the run carries quinn");
    let _ = std::fs::remove_dir_all(&v);
}

#[test]
fn a_legacy_vault_writes_every_r24_record_as_quinn() {
    let v = scratch("legacy", None);
    let out = run(&v, &[]);
    assert!(out.status.success(), "{out:?}");
    let all = actors(&v);
    assert!(!all.is_empty() && all.iter().all(|a| a == "quinn"), "{all:?}");
    let _ = std::fs::remove_dir_all(&v);
}

#[test]
fn a_bad_actor_file_exits_2_and_writes_nothing() {
    let v = scratch("bad", Some("human_actor: alice\n"));
    let before = fingerprint(&v);
    let out = run(&v, &[]);
    assert_eq!(out.status.code(), Some(2), "{out:?}");
    let err = String::from_utf8_lossy(&out.stderr);
    assert!(err.contains("config/actor.yaml: ") && err.contains("\"alice\""), "{err}");
    assert!(out.stdout.is_empty(), "{out:?}");
    assert_eq!(fingerprint(&v), before, "nothing written, journal included");
    let _ = std::fs::remove_dir_all(&v);
}

#[test]
fn an_explicit_actor_passes_through_and_the_write_gate_still_judges_it() {
    // The vault's own token, spelled out: accepted, and what is written is that token.
    let v = scratch("explicit-ok", Some("human_actor: student\n"));
    let out = run(&v, &["--actor", "student"]);
    assert!(out.status.success(), "{out:?}");
    let all = actors(&v);
    assert!(!all.is_empty() && all.iter().all(|a| a == "student"), "{all:?}");
    let _ = std::fs::remove_dir_all(&v);

    // Another human token: not rewritten to the vault's, and `write`'s gate refuses it.
    let v = scratch("explicit-wrong", Some("human_actor: student\n"));
    let out = run(&v, &["--actor", "quinn"]);
    assert!(out.status.success(), "a refused note is a named warning, not a failed run: {out:?}");
    let report: serde_json::Value = serde_json::from_slice(&out.stdout).unwrap();
    assert_eq!(report["confirmed"], 0, "{report}");
    let named = report["warnings"].as_array().unwrap().iter().filter(|w| {
        let w = w.as_str().unwrap();
        w.contains("not written") && w.contains("human is student") && w.contains("\"quinn\"")
    });
    assert_eq!(named.count(), 2, "one named refusal per matched row: {report}");
    assert!(!actors(&v).iter().any(|a| a == "quinn"), "the refused actor reached the journal: {:?}", actors(&v));
    assert!(!v.join("commitments").exists(), "the refused actor wrote a note");
    let _ = std::fs::remove_dir_all(&v);
}
