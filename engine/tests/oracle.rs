//! THE ORACLE — the acceptance test for the entire Rust port.
//!
//! > The Rust engine is correct when it produces the same bytes as the Python engine,
//! > from the same vault, at the same pinned date.
//!
//! This is a direct translation of `tests/test_today_unchanged.py`, and it shares that file's
//! fixtures **verbatim** — the two suites must never diverge on what they read, because the
//! fixture directory is the contract between them.
//!
//! ## Expected state through the port
//!
//! | Wave | This test |
//! |------|-----------|
//! | 0-2  | **RED**, failing with "rank is not implemented yet". This is the only expected red. |
//! | 3    | **GREEN**, and it stays green. |
//!
//! ## The rule that makes it mean something
//!
//! On a mismatch, **fix the Rust**. Never regenerate `golden-today-s1.md`. If the golden file
//! genuinely looks wrong, that is either a real Python bug — log it for after cutover, do not
//! fix it — or a misunderstanding. Both need a human. Re-baselining here silently converts the
//! one mechanical correctness check in this project into a rubber stamp.

use std::path::{Path, PathBuf};
use std::process::Command;

/// Every number in the golden file keys off this date. `test_today_unchanged.py` pins the same one.
const PINNED_DATE: &str = "2026-08-28";

const FIXTURE: &str = "tests/fixtures/vault-s1";
const MIGRATED: &str = "tests/fixtures/vault-s1-migrated";
const GOLDEN: &str = "tests/fixtures/golden-today-s1.md";
const FULL: &str = "tests/fixtures/vault-full";
const GOLDEN_FULL: &str = "tests/fixtures/golden-today-full.md";

/// The footer carries the wall-clock `generated_at`; everything else keys off the pinned date.
///
/// Mirrors `_strip_footer` in `tests/test_today_unchanged.py`, including its `\n` join. Compare
/// the *stripped strings*, never raw file bytes — otherwise a trailing-newline difference reads
/// as a content difference and sends you hunting for a renderer bug that isn't there.
fn strip_footer(text: &str) -> String {
    text.lines()
        .filter(|line| !line.starts_with("*Generated "))
        .collect::<Vec<_>>()
        .join("\n")
}

/// Copy a fixture vault into a scratch directory.
///
/// The port NEVER runs against the working tree: the cloud routine is still writing the live
/// vault twice a day on frozen Python, and two engines writing one vault produces an
/// unattributable journal. Rewrite spec section 3.
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
    let dir = std::env::temp_dir().join(format!("knowlu-oracle-{name}-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    dir
}

/// Path to the binary under test. Cargo exports this for integration tests.
fn binary() -> PathBuf {
    PathBuf::from(env!("CARGO_BIN_EXE_knowlu-engine"))
}

/// Render `today.md` in a scratch copy of `fixture` and return its contents.
fn render(fixture: &str, name: &str) -> String {
    let vault = scratch(name);
    copy_vault(Path::new(fixture), &vault).expect("copy fixture vault");

    let output = Command::new(binary())
        .args(["rank", "--vault"])
        .arg(&vault)
        .args(["--today", PINNED_DATE, "--runner", "manual"])
        .output()
        .expect("run knowlu-engine");

    assert!(
        output.status.success(),
        "`knowlu-engine rank` failed ({}). stderr:\n{}",
        output.status,
        String::from_utf8_lossy(&output.stderr),
    );

    std::fs::read_to_string(vault.join("state").join("today.md"))
        .expect("state/today.md was not written")
}

/// Assert equality with a diff that names the first offending line.
///
/// A wall of unified diff at 3am is the difference between a five-minute fix and an hour, and
/// this test is expected to fail many times before it passes for good.
fn assert_matches(rendered: &str, label: &str, golden: &str) {
    let golden_raw = std::fs::read_to_string(golden).expect("read golden file");
    let expected = strip_footer(&golden_raw);
    let actual = strip_footer(rendered);

    if actual == expected {
        return;
    }

    let exp: Vec<&str> = expected.lines().collect();
    let act: Vec<&str> = actual.lines().collect();
    let mut report = format!("\n[{label}] rendered today.md does not match the golden file.\n");

    for i in 0..exp.len().max(act.len()) {
        let e = exp.get(i);
        let a = act.get(i);
        if e != a {
            report.push_str(&format!(
                "\nFirst difference at line {}:\n  expected: {:?}\n  actual:   {:?}\n",
                i + 1,
                e.unwrap_or(&"<missing>"),
                a.unwrap_or(&"<missing>"),
            ));
            break;
        }
    }
    report.push_str(&format!(
        "\n({} golden lines vs {} rendered lines)\n\
         Fix the Rust. Do NOT regenerate {}.\n",
        exp.len(),
        act.len(),
        golden
    ));
    panic!("{report}");
}

#[test]
fn today_renders_identically_to_golden() {
    assert_matches(&render(FIXTURE, "vault-s1"), "vault-s1", GOLDEN);
}

/// The same golden, on the vault Quinn actually runs — `vault-s1` after `migrate_s1.py`.
///
/// Two things must hold at once: the page is byte-identical, AND the run is silent. The migrated
/// fixture carries `system:migration` create records, so `detect_external` must invent no external
/// edits from values the journal already knows. A migration that renders the same page but WARNs
/// twice a day is not a migration anyone can live with.
///
/// The run-record half of the Python original's assertions is asserted in
/// `cli::tests::the_migrated_fixture_runs_without_inventing_a_single_external_edit`, against
/// `RunOutcome::steps` rather than a written record — `runs` (`state/runs/`, `runner-log.md`) is
/// wave 7, and the claim worth pinning is about the passes, not about the file they land in.
/// Its opposite number,
/// `cli::tests::the_unmigrated_fixture_trips_the_guard_and_still_renders`, covers `vault-s1`.
/// This test asserts the render only.
#[test]
fn migrated_vault_renders_identically() {
    assert_matches(&render(MIGRATED, "vault-s1-migrated"), "vault-s1-migrated", GOLDEN);
}

/// THE SECOND ORACLE — a live-shaped vault, covering what `vault-s1` structurally cannot.
///
/// `vault-s1` has no `calendars:` block and no `config/events.yaml`. That absence is precisely
/// why the waves 0-3 oracle could be green while four steps of `cli.run` were unported, and it
/// is why waves 4-7 needed a second fixture rather than a louder assertion on the first.
///
/// `vault-full` is `vault-s1-migrated` plus the inputs the live vault has, and it is OFFLINE by
/// construction: the `calendars:` entry carries no `ics_url` so calfeed falls back to
/// `state/calendar.md`, and the four event sources use an unknown URL scheme so every fetch
/// fails in the URL parser without opening a socket and `cli.run` falls back to
/// `state/events.md`. Both are the real production fallback paths, not test-only shims.
///
/// What this golden covers that `golden-today-s1.md` cannot:
///
/// - `Capacity today: 6.0h (template 8.25h - 2.25h calendar)` — busy time reaching capacity
/// - a `## Schedule` section, a render branch the first golden never emits
/// - capacity-fitted slices computed against a REDUCED capacity
/// - the events digest header and the `## Coming up` section
///
/// ## GREEN since 2026-09-01, end of wave 5
///
/// It was `#[ignore]`d through waves 0-4 because it could not pass, and its red was a true
/// report: capacity 8.25h where the golden said 6.0h, 27 rendered lines against 38. calfeed
/// (wave 4) fixed the capacity line and the `## Schedule` section; the events pass (wave 5)
/// fixed the digest header and `## Coming up`. Un-ignored ahead of Task 17 Step 1 because it
/// passes: an oracle that passes and is skipped is worth nothing.
///
/// **What it still does NOT cover**, so a green result is not read for more than it says:
/// `info_pass` (this vault's `info/` is empty) and `runs` (`state/runs/` and `runner-log.md`
/// are not part of `today.md`). Both are wave 7. Giving the fixture an `info/` note is a
/// deliberate act that changes the golden and must be recorded — see `scripts/build-vault-full.py`.
///
/// Never regenerate `golden-today-full.md`. Same rule as its older sibling.
#[test]
fn full_vault_renders_identically() {
    assert_matches(&render(FULL, "vault-full"), "vault-full", GOLDEN_FULL);
}
