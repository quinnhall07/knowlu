//! Ruling 11 (plan D2, T3): the CLI's six human flags — `commitments --actor`, `info --actor`,
//! `issues --actor`, `write --actor`, `info open --opened-by` and `info close --closed-by` — default
//! to the vault's own token (`config/actor.yaml`; absent means the legacy token), never to a fixed
//! name. A bad file is exit 2 and nothing written. Run as the binary, on scratch vaults only.

use std::path::{Path, PathBuf};
use std::process::{Command, Output};

fn binary() -> PathBuf {
    PathBuf::from(env!("CARGO_BIN_EXE_knowlu-engine"))
}

const TASK: &str = "---\ntitle: Invented task\nstatus: active\nprogress: 0\nid: task_0123456789\n---\n\nBody.\n";

/// A scratch vault with one task. `actor` is `config/actor.yaml`'s text, or `None` for a legacy
/// vault that has no such file.
fn scratch(name: &str, actor: Option<&str>) -> PathBuf {
    let dir = std::env::temp_dir().join(format!("knowlu-cli-actor-{}-{name}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    for sub in ["config", "tasks", "info", "issues", "archive", "state/journal"] {
        std::fs::create_dir_all(dir.join(sub)).unwrap();
    }
    std::fs::write(dir.join("tasks").join("a.md"), TASK).unwrap();
    if let Some(text) = actor {
        std::fs::write(dir.join("config").join("actor.yaml"), text).unwrap();
    }
    dir
}

/// The two vault shapes and the token each must write.
const VAULTS: [(&str, Option<&str>, &str); 2] =
    [("legacy", None, "quinn"), ("student", Some("human_actor: student\n"), "student")];

fn run(vault: &Path, args: &[&str]) -> Output {
    let (command, rest) = args.split_first().unwrap();
    Command::new(binary()).arg(command).arg("--vault").arg(vault).args(rest).output().unwrap()
}

/// Every journal record, in file then line order.
fn journal(vault: &Path) -> Vec<serde_json::Value> {
    let mut files: Vec<PathBuf> = std::fs::read_dir(vault.join("state").join("journal"))
        .unwrap()
        .flatten()
        .map(|e| e.path())
        .collect();
    files.sort();
    files
        .iter()
        .flat_map(|f| std::fs::read_to_string(f).unwrap().lines().map(|l| serde_json::from_str(l).unwrap()).collect::<Vec<_>>())
        .collect()
}

fn actors(vault: &Path) -> Vec<String> {
    journal(vault).iter().map(|r| r["actor"].as_str().unwrap().to_string()).collect()
}

/// Every file under the vault with its bytes — the "nothing written" oracle.
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

fn ok(out: &Output) -> String {
    assert!(out.status.success(), "{out:?}");
    String::from_utf8_lossy(&out.stdout).to_string()
}

fn front(path: &Path, key: &str) -> String {
    let text = std::fs::read_to_string(path).unwrap();
    text.lines().find_map(|l| l.strip_prefix(&format!("{key}: "))).unwrap_or_default().to_string()
}

#[test]
fn write_set_without_actor_journals_the_vaults_token() {
    for (name, file, token) in VAULTS {
        let v = scratch(&format!("write-{name}"), file);
        ok(&run(&v, &["write", "set", "tasks/a.md", "status=done"]));
        assert_eq!(actors(&v), [token], "{name}");
        let _ = std::fs::remove_dir_all(&v);
    }
}

#[test]
fn info_open_and_close_default_to_the_vaults_token() {
    for (name, file, token) in VAULTS {
        let v = scratch(&format!("info-{name}"), file);
        let opened = ok(&run(&v, &["info", "open", "--title", "Invented notice"]));
        let note = opened.trim().strip_prefix("opened ").expect("opened <name>").to_string();
        assert_eq!(front(&v.join("info").join(&note), "opened_by"), token, "{name}");
        let id = note.trim_end_matches(".md").to_string();
        ok(&run(&v, &["info", "close", "--id", &id]));
        assert_eq!(front(&v.join("archive").join(&note), "closed_by"), format!("\"{token}\""), "{name}");
        let all = actors(&v);
        assert!(!all.is_empty() && all.iter().all(|a| a == token), "{name}: {all:?}");
        let _ = std::fs::remove_dir_all(&v);
    }
}

#[test]
fn commitments_confirm_without_actor_writes_as_the_vaults_token() {
    for (name, file, token) in VAULTS {
        let v = scratch(&format!("confirm-{name}"), file);
        let input = v.with_extension("confirm.json");
        std::fs::write(&input, r#"{"window": "[{days: [mon, tue, wed, thu, fri], start: \"08:00\", end: \"22:00\"}]"}"#).unwrap();
        let out = run(&v, &["commitments", "--today", "2026-09-24", "--confirm", input.to_str().unwrap()]);
        let _ = std::fs::remove_file(&input);
        ok(&out);
        let all = actors(&v);
        assert!(!all.is_empty() && all.iter().all(|a| a == token), "{name}: {all:?}");
        let _ = std::fs::remove_dir_all(&v);
    }
}

#[test]
fn issues_open_without_actor_writes_as_the_vaults_token() {
    for (name, file, token) in VAULTS {
        let v = scratch(&format!("issues-{name}"), file);
        ok(&run(&v, &["issues", "open", "tasks/a.md", "--category", "wrong-effort", "--text", "invented"]));
        assert_eq!(actors(&v), [token], "{name}");
        let _ = std::fs::remove_dir_all(&v);
    }
}

#[test]
fn a_bad_actor_file_exits_2_and_writes_nothing() {
    let v = scratch("bad", Some("human_actor: alice\n"));
    let input = v.with_extension("confirm.json");
    std::fs::write(&input, r#"{"window": "[{days: [mon], start: \"08:00\", end: \"22:00\"}]"}"#).unwrap();
    let before = fingerprint(&v);
    for args in [
        vec!["write", "set", "tasks/a.md", "status=done"],
        vec!["write", "append-body", "tasks/a.md", "--line", "x"],
        vec!["info", "open", "--title", "Invented notice"],
        vec!["info", "close", "--key", "invented"],
        vec!["issues", "open", "tasks/a.md", "--category", "wrong-effort"],
        vec!["commitments", "--today", "2026-09-24", "--confirm", input.to_str().unwrap()],
    ] {
        let out = run(&v, &args);
        assert_eq!(out.status.code(), Some(2), "{args:?}: {out:?}");
        let err = String::from_utf8_lossy(&out.stderr);
        assert!(err.contains("config/actor.yaml: ") && err.contains("\"alice\""), "{args:?}: {err}");
        assert!(out.stdout.is_empty(), "{args:?}: {out:?}");
        assert_eq!(fingerprint(&v), before, "{args:?}: nothing written");
    }
    // Commands that only read never look at the file.
    for args in [vec!["info", "list"], vec!["issues", "list"]] {
        let out = run(&v, &args);
        assert!(out.status.success(), "{args:?}: {out:?}");
        assert!(!String::from_utf8_lossy(&out.stderr).contains("actor.yaml"), "{args:?}: {out:?}");
    }
    let _ = std::fs::remove_file(&input);
    let _ = std::fs::remove_dir_all(&v);
}

#[test]
fn an_explicit_agent_actor_is_passed_through() {
    // Even on a vault whose file is bad: an agent is never the student, so nothing is resolved.
    for (name, file) in [("legacy", None), ("bad", Some("human_actor: alice\n"))] {
        let v = scratch(&format!("agent-{name}"), file);
        ok(&run(&v, &["write", "--actor", "agent:x", "set", "tasks/a.md", "status=done"]));
        assert_eq!(actors(&v), ["agent:x"], "{name}");
        let _ = std::fs::remove_dir_all(&v);
    }
}
