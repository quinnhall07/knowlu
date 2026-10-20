//! Ruling 10 (Amendment 2026-09-29), lane L6, plan T1: with the local runtime gone, `judge` has one
//! production model, `CloudModel`. A vault with no account is a named skip at exit 0, never a
//! "runtime not installed" line, and the `--runtime` and `--model` flags no longer exist. The two
//! CLI tests run the engine binary on a scratch vault with no `config/cloud.yaml`, so nothing reaches
//! a network; they never set `KNOWLU_RUNTIME` or `KNOWLU_MODEL`.

use knowlu_engine::judge::{self, Heuristics, Item, Missing, NoRules, Outcome};
use std::path::{Path, PathBuf};
use std::process::Command;

const NOTE: &str = "---\ntitle: \"CS-100 Homework 3\"\ncourse: null\ndomain: school\ndue: 2026-10-01T23:59\neffort_hours: 1.0\neffort_confidence: unset\neffort_source: inferred\nimportance: 3\nimportance_reason: \"pending enrichment\"\nstatus: active\nprogress: 0\ncreated_by: blackboard\nsource_uid: \"blackboard:_884411_1\"\nneeds_enrichment: true\nid: task_1111111111\n---\n\nChapters 4 and 5.\n";

fn engine() -> PathBuf {
    PathBuf::from(env!("CARGO_BIN_EXE_knowlu-engine"))
}

/// A scratch vault with one flagged task whose `source_uid` is pinned to a course, and no
/// `config/cloud.yaml`. Named for the test and the process id; the caller removes it.
fn scratch(tag: &str) -> PathBuf {
    let d = std::env::temp_dir().join(format!("kn-l6-{tag}-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&d);
    for sub in ["tasks", "approvals", "config", "courses"] {
        std::fs::create_dir_all(d.join(sub)).unwrap();
    }
    std::fs::write(d.join("tasks").join("hw3.md"), NOTE).unwrap();
    std::fs::write(d.join("config").join("ingest.yaml"), "course_map:\n  CS-100: cs-100\n  _884411_1: cs-100\n").unwrap();
    std::fs::write(d.join("courses").join("cs-100.md"), "---\ntitle: CS 100\n---\n\n## Grade weights\n- Homework: 20%\n").unwrap();
    d
}

fn run(args: &[&str], vault: &Path) -> std::process::Output {
    Command::new(engine())
        .arg("judge")
        .arg("--vault")
        .arg(vault)
        .args(args)
        .env_remove("KNOWLU_RUNTIME")
        .env_remove("KNOWLU_MODEL")
        .output()
        .expect("the engine binary starts")
}

fn text(out: &std::process::Output) -> String {
    format!("{}{}", String::from_utf8_lossy(&out.stdout), String::from_utf8_lossy(&out.stderr))
}

/// Ruling 10 and VISION "Failures are visible": no account is a named skip, the first line says why,
/// tier 1 is still written, and the exit stays 0.
#[test]
fn judge_on_a_vault_with_no_account_names_the_reason_and_exits_zero() {
    let v = scratch("noaccount");
    let out = run(&[], &v);
    let all = text(&out);
    assert_eq!(out.status.code(), Some(0), "judge always exits 0:\n{all}");
    assert!(
        String::from_utf8_lossy(&out.stdout).lines().any(|l| l.starts_with("judge: skipped (no account")),
        "a lead line names the reason:\n{all}"
    );
    assert!(!all.contains("runtime not installed"), "the local runtime no longer exists:\n{all}");
    let note = std::fs::read_to_string(v.join("tasks").join("hw3.md")).unwrap();
    assert!(note.contains("course: \"cs-100\""), "tier 1 still attributes the course:\n{note}");
    let _ = std::fs::remove_dir_all(&v);
}

/// Ruling 10: `--runtime` and `--model` are gone, so clap refuses both (usage error, exit 2) and
/// nothing is written.
#[test]
fn judge_refuses_the_runtime_and_model_flags() {
    for flag in [["--runtime", "x.exe"], ["--model", "y.gguf"]] {
        let v = scratch("flags");
        let before = std::fs::read_to_string(v.join("tasks").join("hw3.md")).unwrap();
        let out = run(&flag, &v);
        assert_eq!(out.status.code(), Some(2), "{} must be a usage error:\n{}", flag[0], text(&out));
        assert!(!v.join("state").exists(), "{} wrote state", flag[0]);
        assert_eq!(std::fs::read_to_string(v.join("tasks").join("hw3.md")).unwrap(), before);
        let _ = std::fs::remove_dir_all(&v);
    }
}

fn item() -> Item {
    Item {
        id: "task_0123456789".to_string(),
        rel_path: "tasks/x.md".to_string(),
        title: "CS-100 Homework 3".to_string(),
        body: "Submit through the course page.".to_string(),
        source_uid: "blackboard:nothing".to_string(),
        created_by: "blackboard".to_string(),
        course: None,
        due: Some("2026-10-01T23:59".to_string()),
        effort_hours: 1.0,
        effort_source: "inferred".to_string(),
    }
}

fn heur() -> Heuristics {
    Heuristics {
        course_map: vec![("CS-100".to_string(), "cs-100".to_string())],
        slice_hours: 2.0,
        weights: std::collections::BTreeMap::new(),
        preferences: String::new(),
    }
}

/// Green pin. Replaces the deleted `a_missing_runtime_or_model_…` test: a missing service is an
/// outcome that still carries tier 1.
#[test]
fn a_missing_service_still_carries_tier_one() {
    let r = judge::judge_task(&item(), &heur(), &NoRules, Err(Missing::Service("no account")));
    assert!(matches!(r, Outcome::ServiceUnavailable(_)), "{r:?}");
    assert_eq!(r.verdict().course.as_deref(), Some("cs-100"));
    assert_eq!(r.label(), "service unavailable");
}

/// Green pin. Replaces the deleted prompt-bound test at the boundary where the bound now lives: the
/// request body the cloud receives.
#[test]
fn the_task_request_is_bounded_however_long_the_note_is() {
    let mut long = item();
    long.body = "x".repeat(10_000);
    let req = knowlu_engine::cloudmodel::task_request(&long, &heur(), &judge::Verdict::default());
    let body = req["item"]["body"].as_str().expect("the request carries a body string");
    assert!(body.chars().count() <= judge::MAX_BODY_CHARS, "{} chars", body.chars().count());
}
