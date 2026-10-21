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
    assert_eq!(report["dropped"], serde_json::json!({ "exam": 1, "meeting_type": 0, "midnight": 1, "no_time": 1, "status": 0 }));
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

/// `run` on a chosen day (the helper above pins 2026-09-01).
fn run_on(vault: &Path, day: &str, text: &str, extra: &[&str]) -> Output {
    let file = vault.with_extension("registrar.json");
    std::fs::write(&file, text).unwrap();
    let out = Command::new(binary())
        .args(["commitments", "--vault"])
        .arg(vault)
        .args(["--today", day, "--registrar"])
        .arg(&file)
        .args(extra)
        .output()
        .unwrap();
    let _ = std::fs::remove_file(&file);
    out
}

/// Every file under the vault with its bytes, sorted - the "nothing written" oracle.
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

fn journal_records(vault: &Path) -> Vec<serde_json::Value> {
    let Ok(dir) = std::fs::read_dir(vault.join("state").join("journal")) else { return Vec::new() };
    let mut files: Vec<PathBuf> = dir.flatten().map(|e| e.path()).collect();
    files.sort();
    files.iter().flat_map(|f| std::fs::read_to_string(f).unwrap().lines().map(|l| serde_json::from_str(l).unwrap()).collect::<Vec<_>>()).collect()
}

/// Close-out B2 / B3b and B3c, run as the binary. A refetch with a moved time applies it as
/// `agent:knowlu.registrar` via `dashboard`, prints it under `applied`; a new process's
/// `commitments --json` lists it again under `registrar_changes`; and a fetch that exits 2 after
/// that writes nothing, applied writes included.
#[test]
fn a_refetch_applies_as_the_registrar_and_a_new_process_lists_it() {
    let v = scratch("apply");
    std::fs::write(v.join("config").join("actor.yaml"), "human_actor: student\n").unwrap();
    assert!(run_on(&v, "2026-09-01", FIXTURE, &["--school", "ua"]).status.success());
    let mut moved: serde_json::Value = serde_json::from_str(FIXTURE).unwrap();
    moved["data"]["registrations"][0]["meetingTimes"][0]["beginTime"] = serde_json::json!("1300");
    moved["data"]["registrations"][0]["meetingTimes"][0]["endTime"] = serde_json::json!("1350");
    let out = run_on(&v, "2026-09-10", &moved.to_string(), &["--school", "ua"]);
    assert!(out.status.success(), "{out:?}");
    let report: serde_json::Value = serde_json::from_slice(&out.stdout).unwrap();
    let applied = report["applied"].as_array().unwrap_or_else(|| panic!("no `applied` list: {report}")).clone();
    assert_eq!(applied.len(), 1, "{report}");
    assert_eq!((applied[0]["note"].as_str(), applied[0]["field"].as_str()), (Some("commitments/cs-100.md"), Some("meets")));
    let sets: Vec<_> = journal_records(&v).into_iter().filter(|r| r["op"] == "set" && r["path"] == "commitments/cs-100.md" && r["field"] == "meets").collect();
    assert_eq!(sets.len(), 1, "{sets:?}");
    assert_eq!((sets[0]["actor"].as_str(), sets[0]["via"].as_str()), (Some("agent:knowlu.registrar"), Some("dashboard")));
    let overview = Command::new(binary()).args(["commitments", "--vault"]).arg(&v).args(["--today", "2026-09-10", "--json"]).output().unwrap();
    let overview: serde_json::Value = serde_json::from_slice(&overview.stdout).unwrap();
    assert_eq!(overview["registrar_changes"]["applied"], serde_json::Value::Array(applied), "{overview}");
    let before = fingerprint(&v);
    let bad = run_on(&v, "2026-09-20", "{\"data\": {\"registrations\": []}}", &["--school", "ua"]);
    assert_eq!(bad.status.code(), Some(2), "{bad:?}");
    assert!(bad.stdout.is_empty());
    assert_eq!(fingerprint(&v), before, "an exit 2 wrote something");
    let _ = std::fs::remove_dir_all(&v);
}

/// P7 (fixture review M1): instructor names, emails and ids are never read, logged or put in a
/// warning or error. The fixture carries invented ones on every row; none may surface in what
/// `parse_banner` returns (series, warnings, counts) or in what the command prints or writes.
#[test]
fn instructor_data_never_surfaces() {
    const SECRETS: [&str; 3] = ["Invented Instructor", "example.invalid", "INV00000"];
    fn check(what: &str, text: &str) {
        for s in SECRETS {
            assert!(!text.contains(s), "{what} carries {s:?}: {text}");
        }
    }
    let json: serde_json::Value = serde_json::from_str(FIXTURE).unwrap();
    assert!(FIXTURE.contains("Invented Instructor One"), "the fixture no longer carries instructor data");
    let parsed = knowlu_engine::registrar::parse_banner(&json, "ua").unwrap();
    assert!(!parsed.series.is_empty());
    check("parse_banner", &format!("{parsed:?}"));
    // A parse error names no row data either. Until B9a records UA's registered status codes,
    // `SCHOOLS`' UA entry has `registered: &[]` (every row counts), so this leg parses cleanly and
    // only checks the Ok value; it exercises the error path once B9a fills the set.
    let mut bad = json.clone();
    for r in bad["data"]["registrations"].as_array_mut().unwrap() {
        r["statusIndicator"] = "XX".into();
    }
    check("parse_banner on dropped rows", &format!("{:?}", knowlu_engine::registrar::parse_banner(&bad, "ua")));
    let v = scratch("instructors");
    let out = run(&v, FIXTURE, &["--school", "ua"]);
    assert!(out.status.success(), "{out:?}");
    check("stdout", &String::from_utf8_lossy(&out.stdout));
    check("stderr", &String::from_utf8_lossy(&out.stderr));
    for (path, bytes) in fingerprint(&v) {
        check(&path.to_string_lossy(), &String::from_utf8_lossy(&bytes));
    }
    let _ = std::fs::remove_dir_all(&v);
}
