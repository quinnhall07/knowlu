//! Every child this crate spawns outside its tests must go through `childproc::NoConsole`.
//!
//! Knowlu is a GUI application, and a console-subsystem child spawned without `CREATE_NO_WINDOW`
//! flashes a console window. `history::git_with` runs on the app's housekeeping tick — **every 60
//! seconds** — so one missing call here is a terminal blinking at the user once a minute for the
//! whole session. That is exactly what happened, and it went unnoticed until 2026-09-09 because no
//! test can see a window and the app had never been run for long enough.
//!
//! The scan stops at the first `#[cfg(test)]`: a test that spawns `powershell` to fill a pipe wants
//! no flag and gets no window anyway, because `cargo test` is already a console process.
//!
//! What this proves and what it does not: it proves no file spawns a child while ignoring the
//! helper entirely. It does not prove a *second* spawn added to a file that already has one is
//! covered — the counts are compared, so an uncovered second spawn does fail, but a reviewer should
//! still read the call site rather than trust the arithmetic alone.

use std::path::Path;

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
        let code = text.split("#[cfg(test)]").next().unwrap_or("");
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
