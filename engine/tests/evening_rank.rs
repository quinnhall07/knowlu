//! The evening ranking rule, through the real binary (plan `2026-10-24-evening-ranking-plan.md`).
//!
//! This file asserts against the frozen golden files, so it follows `oracle.rs` to the letter:
//! every run is on a scratch COPY of a fixture vault in the temp directory, the footer is stripped
//! the same way, and nothing here ever writes under `tests/fixtures/`. A golden is never
//! regenerated; if the engine disagrees with one, the engine is wrong.

use std::path::{Path, PathBuf};
use std::process::{Command, Output};

const PINNED_DATE: &str = "2026-08-28";

const FIXTURE: &str = "tests/fixtures/vault-s1";
const MIGRATED: &str = "tests/fixtures/vault-s1-migrated";
const GOLDEN: &str = "tests/fixtures/golden-today-s1.md";
const FULL: &str = "tests/fixtures/vault-full";
const GOLDEN_FULL: &str = "tests/fixtures/golden-today-full.md";

/// Copied from `oracle.rs`, not imported: drops only the `*Generated ` footer line.
fn strip_footer(text: &str) -> String {
    text.lines()
        .filter(|line| !line.starts_with("*Generated "))
        .collect::<Vec<_>>()
        .join("\n")
}

/// Copied from `oracle.rs`.
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

fn scratch(name: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!("knowlu-evening-{name}-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    dir
}

fn binary() -> PathBuf {
    PathBuf::from(env!("CARGO_BIN_EXE_knowlu-engine"))
}

/// `rank --today <PINNED_DATE> --now <now>` on a scratch copy of `fixture`; returns the vault and
/// the process output.
fn rank_at(fixture: &str, name: &str, now: &str) -> (PathBuf, Output) {
    let vault = scratch(name);
    copy_vault(Path::new(fixture), &vault).expect("copy fixture vault");
    let output = Command::new(binary())
        .args(["rank", "--vault"])
        .arg(&vault)
        .args(["--today", PINNED_DATE, "--now", now, "--runner", "manual"])
        .output()
        .expect("run knowlu-engine");
    (vault, output)
}

fn assert_renders_golden(fixture: &str, golden: &str, label: &str, now: &str) {
    let (vault, output) = rank_at(fixture, &format!("{label}-{}", now.replace(':', "")), now);
    assert!(
        output.status.success(),
        "`rank --now {now}` on {label} failed ({}). stderr:\n{}",
        output.status,
        String::from_utf8_lossy(&output.stderr),
    );
    let rendered = std::fs::read_to_string(vault.join("state").join("today.md"))
        .expect("state/today.md was not written");
    let expected = strip_footer(&std::fs::read_to_string(golden).expect("read golden file"));
    assert_eq!(
        strip_footer(&rendered),
        expected,
        "[{label} at {now}] today.md differs from {golden}. Fix the Rust; do NOT regenerate it."
    );
    let _ = std::fs::remove_dir_all(&vault);
}

/// No fixture task is due on 2026-08-28, so an evening moment must leave every golden alone.
#[test]
fn rank_at_an_evening_moment_still_renders_both_goldens() {
    for now in ["2026-08-28T21:00", "2026-08-28T23:58"] {
        assert_renders_golden(FIXTURE, GOLDEN, "vault-s1", now);
        assert_renders_golden(MIGRATED, GOLDEN, "vault-s1-migrated", now);
        assert_renders_golden(FULL, GOLDEN_FULL, "vault-full", now);
    }
}

#[test]
fn rank_refuses_a_bad_now_with_exit_2() {
    let (vault, output) = rank_at(FIXTURE, "badnow", "tonight");
    assert_eq!(output.status.code(), Some(2), "stdout:\n{}", String::from_utf8_lossy(&output.stdout));
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(stderr.contains("--now"), "stderr should name --now:\n{stderr}");
    let _ = std::fs::remove_dir_all(&vault);
}
