//! The read model's acceptance artefacts. `knowlu-engine surface --view today` on each fixture vault
//! at the pinned date and a pinned `now`, compared byte for byte against a checked-in JSON file.
//!
//! These references are RUST-generated — S2 was never built in Python, so there is no Python
//! oracle for this module — and their rule differs from the eight Python-written references:
//! regenerating one is allowed, but only in a commit whose diff shows the change and whose message
//! says why (console spec §4.6). Generate from Git Bash — NOT PowerShell's `>`, which writes
//! UTF-16LE with a BOM in 5.1 (the `from_utf8` below then panics) and yields LF, not the CRLF
//! these files carry, from cmd — run from `engine/`, where the fixtures are:
//!   ../target/debug/knowlu-engine.exe surface --vault tests/fixtures/vault-full --view today --today 2026-08-28 --now 2026-08-28T09:00 --build-sha pinned | sed 's/$/\r/' > tests/fixtures/surface-today-full.json
//! (one JSON line + CRLF, UTF-8, no BOM — the bytes actually checked in) from a scratch COPY if
//! the command ever grows a write (it must not; surface::tests proves it).
use std::path::{Path, PathBuf};
use std::process::Command;

const PINNED_DATE: &str = "2026-08-28";
const PINNED_NOW: &str = "2026-08-28T09:00";

fn binary() -> PathBuf { PathBuf::from(env!("CARGO_BIN_EXE_knowlu-engine")) }

fn surface(fixture: &str) -> String {
    let out = Command::new(binary()).args(["surface", "--vault", &format!("tests/fixtures/{fixture}"), "--view", "today", "--today", PINNED_DATE, "--now", PINNED_NOW, "--build-sha", "pinned"]).output().expect("run knowlu-engine surface");
    assert!(out.status.success(), "surface failed: {}", String::from_utf8_lossy(&out.stderr));
    String::from_utf8(out.stdout).expect("utf-8")
}

fn reference(name: &str) -> String {
    let raw = std::fs::read(Path::new("tests/fixtures").join(name)).expect("reference file");
    String::from_utf8(raw).expect("utf-8").replace("\r\n", "\n")
}

fn check(fixture: &str, reference_name: &str) {
    let got = surface(fixture).replace("\r\n", "\n");
    let want = reference(reference_name);
    assert!(got == want, "{fixture}: the read model disagrees with {reference_name}.\nIf the change is intended, regenerate the file IN A COMMIT THAT SHOWS THE DIFF AND SAYS WHY (spec §4.6).\n--- got (first 600 chars)\n{}\n--- want\n{}", &got[..got.len().min(600)], &want[..want.len().min(600)]);
}

#[test] fn surface_today_s1_is_frozen() { check("vault-s1", "surface-today-s1.json"); }
#[test] fn surface_today_s1_migrated_is_frozen() { check("vault-s1-migrated", "surface-today-s1-migrated.json"); }
#[test] fn surface_today_full_is_frozen() { check("vault-full", "surface-today-full.json"); }

#[test]
fn surface_refuses_an_unknown_view_with_exit_2() {
    let out = Command::new(binary()).args(["surface", "--vault", "tests/fixtures/vault-s1", "--view", "tomorrow", "--today", PINNED_DATE, "--now", PINNED_NOW, "--build-sha", "pinned"]).output().unwrap();
    assert_eq!(out.status.code(), Some(2));
}
