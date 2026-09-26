//! The app's half of the console-window guard. See `tests/no_console.rs` in the engine crate for
//! why this exists: Knowlu is a GUI application, and a console child spawned without
//! `CREATE_NO_WINDOW` flashes a window. This crate spawns the engine exe on every slot and
//! `taskkill` when one times out, so after the scheduler goes live an unguarded spawn would blink a
//! terminal twice a day rather than once a minute — rarer, and therefore harder to attribute.

use std::path::Path;

/// Fix round 1, review I2. The scan must stop at the file's real `#[cfg(test)] mod …` boundary,
/// never at an earlier BARE MENTION of the words `#[cfg(test)]` inside a doc comment —
/// `app/src/account.rs`'s own header names the attribute in prose, which hid 1,417 of its 1,467
/// lines from this scan before this fix.
fn non_test_code(text: &str) -> &str {
    text.split("\n#[cfg(test)]\nmod ").next().unwrap_or(text)
}

#[test]
fn the_scan_is_not_fooled_by_a_doc_comment_naming_cfg_test() {
    // A doc comment that merely NAMES `#[cfg(test)]` in prose (exactly `account.rs`'s shape) must
    // not be mistaken for the real test-module boundary and hide a real, unguarded spawn after it.
    let text = "//! The scan below stops at the first #[cfg(test)] mention, in prose only.\n\
                 fn oops() { std::process::Command::new(\"notepad\"); }\n\
                 \n\
                 #[cfg(test)]\n\
                 mod tests {\n    fn t() {}\n}\n";
    let code = non_test_code(text);
    assert!(code.contains("Command::new(\"notepad\")"), "a real spawn after a doc-comment mention of #[cfg(test)] must still be scanned");
    assert!(!code.contains("mod tests"), "the real test module stays excluded");
}

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

    assert!(scanned >= 5, "the scan found suspiciously few source files: {scanned}");
    assert!(with_spawns >= 2, "expected at least two files to spawn children, found {with_spawns}");
}
