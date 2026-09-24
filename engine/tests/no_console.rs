//! Every child this crate spawns outside its tests must go through `childproc::NoConsole`.
//!
//! Knowlu is a GUI application, and a console-subsystem child spawned without `CREATE_NO_WINDOW`
//! flashes a console window. `history::git_with` USED TO run on the app's housekeeping tick —
//! **every 60 seconds** — so one missing call there was a terminal blinking at the user once a
//! minute for the whole session. That is exactly what happened, and it went unnoticed until
//! 2026-09-09 because no test can see a window and the app had never been run for long enough.
//! `history.rs` is gone (git leaves the product, C3' Task 10); the two spawns this crate still
//! makes, `runs::git_sha` and `runtime.rs`, are what this guard checks now.
//!
//! The scan stops at the file's own `#[cfg(test)] mod …` boundary (fix round 1, review I2 — never
//! at an earlier bare mention of the words `#[cfg(test)]`, which a doc comment can carry in prose
//! without meaning it): a test that spawns `powershell` to fill a pipe wants no flag and gets no
//! window anyway, because `cargo test` is already a console process.
//!
//! What this proves and what it does not: it proves no file spawns a child while ignoring the
//! helper entirely. It does not prove a *second* spawn added to a file that already has one is
//! covered — the counts are compared, so an uncovered second spawn does fail, but a reviewer should
//! still read the call site rather than trust the arithmetic alone.

use std::path::Path;

/// Fix round 1, review I2. The scan must stop at the file's real `#[cfg(test)] mod …` boundary,
/// never at an earlier BARE MENTION of the words `#[cfg(test)]` inside a doc comment —
/// `journal.rs`'s `#[cfg(test)] pub(crate) static DEVICE_ENV_MUTEX` is a real, non-module use of
/// the attribute that used to cut the scan off at line 92 of 594, hiding everything after it.
fn non_test_code(text: &str) -> &str {
    text.split("\n#[cfg(test)]\nmod ").next().unwrap_or(text)
}

#[test]
fn the_scan_is_not_fooled_by_a_doc_comment_naming_cfg_test() {
    // A doc comment that merely NAMES `#[cfg(test)]` in prose, or a real `#[cfg(test)]` attribute
    // on something that is not a test module (`journal.rs`'s shape), must not be mistaken for the
    // real test-module boundary and hide a real, unguarded spawn after it.
    let text = "//! The scan below stops at the first #[cfg(test)] mention, in prose only.\n\
                 #[cfg(test)]\n\
                 pub(crate) static X: u8 = 0;\n\
                 fn oops() { std::process::Command::new(\"git\"); }\n\
                 \n\
                 #[cfg(test)]\n\
                 mod tests {\n    fn t() {}\n}\n";
    let code = non_test_code(text);
    assert!(code.contains("Command::new(\"git\")"), "a real spawn after a doc-comment mention of #[cfg(test)], or a real non-module use of it, must still be scanned");
    assert!(!code.contains("mod tests"), "the real test module stays excluded");
}

#[test]
fn every_non_test_spawn_suppresses_its_console_window() {
    let src = Path::new(env!("CARGO_MANIFEST_DIR")).join("src");
    assert!(src.is_dir(), "{} is not a directory — the guard cannot see the source", src.display());

    let mut scanned = 0;
    let mut with_spawns = 0;
    for entry in std::fs::read_dir(&src).unwrap_or_else(|e| panic!("{}: {e}", src.display())).flatten() {
        let path = entry.path();
        if path.extension().map(|x| x == "rs") != Some(true) {
            continue;
        }
        let text = std::fs::read_to_string(&path).unwrap_or_else(|e| panic!("{}: {e}", path.display()));
        // Non-test code only.
        let code = non_test_code(&text);
        let spawns = code.matches("Command::new").count();
        let guarded = code.matches(".no_console()").count();
        assert!(
            guarded >= spawns,
            "{}: {spawns} spawn(s) but only {guarded} .no_console() call(s) — a GUI child process \
             without CREATE_NO_WINDOW flashes a console window at the user",
            path.display()
        );
        if spawns > 0 {
            with_spawns += 1;
        }
        scanned += 1;
    }

    assert!(scanned >= 10, "the scan found suspiciously few source files: {scanned}");
    // A guard that cannot fail is worse than no guard: if the spawn sites ever vanish entirely,
    // this test would pass vacuously and nobody would notice it had stopped meaning anything.
    // `history.rs` was one of the three engine files that spawned a child process (git leaves the
    // product, C3' Task 10) and it is gone; `runs::git_sha` and `runtime.rs` are the two that remain.
    assert!(with_spawns >= 2, "expected at least two files to spawn children, found {with_spawns}");
}
