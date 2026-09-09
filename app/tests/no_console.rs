//! The app's half of the console-window guard. See `tests/no_console.rs` in the engine crate for
//! why this exists: Knowlu is a GUI application, and a console child spawned without
//! `CREATE_NO_WINDOW` flashes a window. This crate spawns the engine exe on every slot and
//! `taskkill` when one times out, so after the scheduler goes live an unguarded spawn would blink a
//! terminal twice a day rather than once a minute — rarer, and therefore harder to attribute.

use std::path::Path;

#[test]
fn every_non_test_spawn_suppresses_its_console_window() {
    let src = Path::new(env!("CARGO_MANIFEST_DIR")).join("src");
    assert!(src.is_dir(), "{} is not a directory — the guard cannot see this crate's source", src.display());

    let mut scanned = 0;
    let mut with_spawns = 0;
    for entry in std::fs::read_dir(&src).unwrap_or_else(|e| panic!("{}: {e}", src.display())).flatten() {
        let path = entry.path();
        if path.extension().map(|x| x == "rs") != Some(true) {
            continue;
        }
        let text = std::fs::read_to_string(&path).unwrap_or_else(|e| panic!("{}: {e}", path.display()));
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

    assert!(scanned >= 5, "the scan found suspiciously few source files: {scanned}");
    assert!(with_spawns >= 2, "expected at least two files to spawn children, found {with_spawns}");
}
