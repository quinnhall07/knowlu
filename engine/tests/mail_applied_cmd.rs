//! Email plan H2: `knowlu-engine mail-applied --vault <v> [--since <ts>]`, run as the binary on a
//! scratch vault. It prints JSON, exits 0, writes nothing and is outside the entitlement gate.

use std::path::{Path, PathBuf};
use std::process::{Command, Output};

fn binary() -> PathBuf {
    PathBuf::from(env!("CARGO_BIN_EXE_knowlu-engine"))
}

fn scratch(name: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!("knowlu-mail-applied-{}-{name}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(dir.join("config")).unwrap();
    dir
}

fn run(vault: &Path, extra: &[&str]) -> Output {
    Command::new(binary()).args(["mail-applied", "--vault"]).arg(vault).args(extra).output().unwrap()
}

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
fn mail_applied_prints_json_and_exits_zero_on_a_scratch_vault_and_writes_nothing() {
    let v = scratch("plain");
    let before = tree(&v);
    for extra in [&[][..], &["--since", "2026-09-01T00:00:00Z"][..]] {
        let out = run(&v, extra);
        assert_eq!(out.status.code(), Some(0), "{out:?}");
        let json: serde_json::Value = serde_json::from_slice(&out.stdout).expect("stdout is JSON");
        assert!(json["courses"].is_array(), "{json}");
        assert!(out.stderr.is_empty(), "{out:?}");
    }
    assert_eq!(tree(&v), before);
    let _ = std::fs::remove_dir_all(&v);
}
