//! The deadline-ordering guards (plan `2026-10-24-deadline-ordering-plan.md` T1o, spec §8).
//!
//! These assert against frozen references, so the file follows `oracle.rs`: every `rank` run is on
//! a scratch COPY in the temp directory, the footer is stripped the same way, and nothing here ever
//! writes under `tests/fixtures/`. `surface` never writes, so it reads the fixtures in place. Both
//! tests are guards: green before the formula exists, green unedited at every commit after. On a
//! mismatch, fix the Rust; never regenerate a golden or a `surface-today-*.json`.

use std::path::{Path, PathBuf};
use std::process::Command;

const PINNED_DATE: &str = "2026-08-28";
const GOLDEN: &str = "tests/fixtures/golden-today-s1.md";
const GOLDEN_FULL: &str = "tests/fixtures/golden-today-full.md";
const MOMENTS: [&str; 4] = ["09:00", "19:30", "21:00", "23:58"];

/// Copied from `oracle.rs`, not imported: drops only the `*Generated ` footer line.
fn strip_footer(text: &str) -> String {
    text.lines().filter(|line| !line.starts_with("*Generated ")).collect::<Vec<_>>().join("\n")
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

fn binary() -> PathBuf { PathBuf::from(env!("CARGO_BIN_EXE_knowlu-engine")) }

/// `rank --today <PINNED_DATE> --now <now>` on a scratch copy of `fixture`, against `golden`.
fn assert_rank_renders(fixture: &str, golden: &str, now: &str) {
    let tag = format!("{fixture}-{}-{}", now.replace(':', ""), std::process::id());
    let vault = std::env::temp_dir().join(format!("knowlu-deadline-order-{tag}"));
    let _ = std::fs::remove_dir_all(&vault);
    copy_vault(&Path::new("tests/fixtures").join(fixture), &vault).expect("copy fixture vault");
    let out = Command::new(binary()).args(["rank", "--vault"]).arg(&vault)
        .args(["--today", PINNED_DATE, "--now", now, "--runner", "manual"]).output().expect("run knowlu-engine");
    assert!(out.status.success(), "`rank --now {now}` on {fixture} failed ({}). stderr:\n{}", out.status, String::from_utf8_lossy(&out.stderr));
    let rendered = std::fs::read_to_string(vault.join("state").join("today.md")).expect("state/today.md was not written");
    let expected = strip_footer(&std::fs::read_to_string(golden).expect("read golden file"));
    assert_eq!(strip_footer(&rendered), expected, "[{fixture} at {now}] today.md differs from {golden}. Fix the Rust; do NOT regenerate it.");
    let _ = std::fs::remove_dir_all(&vault);
}

/// Must do in every fixture is two past-due tasks with no window and nothing dated, so step 0 is
/// safe at any moment and the page is the golden's.
#[test]
fn rank_at_any_moment_renders_both_goldens() {
    for t in MOMENTS {
        let now = format!("{PINNED_DATE}T{t}");
        assert_rank_renders("vault-s1", GOLDEN, &now);
        assert_rank_renders("vault-s1-migrated", GOLDEN, &now);
        assert_rank_renders("vault-full", GOLDEN_FULL, &now);
    }
}

/// `must_do`, `recommended`, `meter` and `nav_counts` without `runs_warn`. The clock-driven parts
/// (`now`, `revision`, the runs panel, `runs_warn`) differ at 21:00 for clock reasons; not compared.
fn blocks(state: &serde_json::Value, label: &str) -> serde_json::Value {
    for key in ["must_do", "recommended", "meter"] {
        assert!(state.get(key).is_some(), "{label} has no `{key}` block");
    }
    let mut nav = state["nav_counts"].clone();
    nav.as_object_mut().unwrap_or_else(|| panic!("{label}: nav_counts is not an object")).remove("runs_warn");
    serde_json::json!({"must_do": state["must_do"], "recommended": state["recommended"], "meter": state["meter"], "nav_counts": nav})
}

#[test]
fn surface_at_nine_and_nine_pm_keeps_the_references_blocks() {
    for (fixture, reference) in [("vault-s1", "surface-today-s1.json"), ("vault-s1-migrated", "surface-today-s1-migrated.json"), ("vault-full", "surface-today-full.json")] {
        let raw = std::fs::read(Path::new("tests/fixtures").join(reference)).expect("reference file");
        let want = blocks(&serde_json::from_slice(&raw).expect("reference is JSON"), reference);
        for t in ["09:00", "21:00"] {
            let out = Command::new(binary())
                .args(["surface", "--vault", &format!("tests/fixtures/{fixture}"), "--view", "today"])
                .args(["--today", PINNED_DATE, "--now", &format!("{PINNED_DATE}T{t}"), "--build-sha", "pinned"])
                .output().expect("run knowlu-engine surface");
            assert!(out.status.success(), "surface on {fixture} at {t} failed: {}", String::from_utf8_lossy(&out.stderr));
            let got: serde_json::Value = serde_json::from_slice(&out.stdout).expect("surface prints one JSON document");
            assert_eq!(blocks(&got, fixture), want, "{fixture} at {t} disagrees with {reference}. Fix the Rust; do NOT regenerate it.");
        }
    }
}
