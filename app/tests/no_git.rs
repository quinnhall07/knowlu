//! Git is not part of this product (cloud design §4.1, §4.4).
//!
//! **There is no `git2` and there never was.** `history.rs` drove the `git` EXECUTABLE, through
//! `git_with("git", …)` → `Command::new(program)`, so a dependency-graph check alone would have
//! passed on every day the app was still spawning a process every sixty seconds. And the app never
//! wrote `Command::new("git")` either: it called into the engine. So this scans for the names the
//! app actually used — `history::`, `refresh_history`, `refresh_head`, `git_sha` — plus the spawn
//! and the crates, because an assertion that was never true of anything passes vacuously.
use std::path::Path;

fn sources(dir: &Path) -> Vec<(String, String)> {
    let mut out = Vec::new();
    for entry in std::fs::read_dir(dir).unwrap_or_else(|e| panic!("{}: {e}", dir.display())).flatten() {
        let path = entry.path();
        if path.extension().map(|x| x == "rs") == Some(true) {
            let name = path.file_name().map(|n| n.to_string_lossy().to_string()).unwrap_or_default();
            out.push((name, std::fs::read_to_string(&path).unwrap_or_default()));
        }
    }
    assert!(out.len() >= 10, "the scan found suspiciously few source files in {}", dir.display());
    out
}

#[test]
fn no_git_process_is_spawned_for_a_vault() {
    let src = Path::new(env!("CARGO_MANIFEST_DIR")).join("src");
    let mut scanned = 0;
    for (name, text) in sources(&src) {
        let code = text.split("#[cfg(test)]").next().unwrap_or("");
        for forbidden in ["Command::new(\"git\")", "history::", "refresh_history", "refresh_head", "git_sha"] {
            assert!(!code.contains(forbidden), "app/src/{name} still names {forbidden}");
        }
        scanned += 1;
    }
    assert!(scanned >= 14, "the scan found suspiciously few app source files: {scanned}");
}

#[test]
fn the_engines_git_transport_is_gone_too() {
    // `history.rs` spawned through `git_with("git", …)`, not `Command::new("git")`, so the name to
    // look for on the engine side is the helper — the one that would come back if anyone re-added
    // the module. `runs::git_sha` stays and is named here so its survival is deliberate: a run
    // record's `sha` field is a contract with existing vaults and already answers `None` on a
    // folder that is not a repository, which is every vault the app creates.
    let src = Path::new(env!("CARGO_MANIFEST_DIR")).join("..").join("engine").join("src");
    for (name, text) in sources(&src) {
        let code = text.split("#[cfg(test)]").next().unwrap_or("");
        assert!(!code.contains("git_with"), "engine/src/{name} still drives the git executable");
        assert!(!code.contains("SyncLock"), "engine/src/{name} still holds history.rs's lock");
    }
    let runs = std::fs::read_to_string(src.join("runs.rs")).expect("runs.rs");
    assert!(runs.contains("pub fn git_sha"), "runs::git_sha stays: a run record's `sha` is a contract");
}

#[test]
fn the_history_module_is_gone_and_nothing_declares_it() {
    let engine = Path::new(env!("CARGO_MANIFEST_DIR")).join("..").join("engine");
    assert!(!engine.join("src").join("history.rs").exists(), "engine/src/history.rs is still here");
    let lib = std::fs::read_to_string(engine.join("src").join("lib.rs")).expect("lib.rs");
    assert!(!lib.contains("pub mod history"), "lib.rs still declares it");
    // `childproc` outlives it: `runs::git_sha` and `runtime.rs` both spawn children, and a GUI
    // application's child must never flash a console window.
    assert!(lib.contains("pub mod childproc"), "childproc must stay");
}

#[test]
fn no_git_library_is_in_any_manifest() {
    for (name, manifest) in [
        ("app/Cargo.toml", include_str!("../Cargo.toml")),
        ("Cargo.toml", include_str!("../../Cargo.toml")),
        ("engine/Cargo.toml", include_str!("../../engine/Cargo.toml")),
    ] {
        for forbidden in ["git2", "libgit2", "gix", "gitoxide"] {
            assert!(!manifest.contains(forbidden), "`{forbidden}` must not be a dependency ({name}): git is not part of this product");
        }
    }
}

#[test]
fn the_page_says_nothing_about_a_repository() {
    let js = std::fs::read_to_string(Path::new(env!("CARGO_MANIFEST_DIR")).join("static").join("console.js")).expect("console.js");
    for word in ["is_repo", "has_remote", "auto-sync", "auto_sync", "pending push", "engine_newer", "vault_head"] {
        assert!(!js.contains(word), "the page still renders {word}, which no longer exists");
    }
}

#[test]
fn the_local_snapshot_mirror_is_still_a_module() {
    // §5.5: "the local snapshot tick stays". The account's copy answers "I lost the laptop"; the
    // mirror answers "I deleted the file five minutes ago". They are different products.
    //
    // **This asserts existence, not identity** (review M4). "Byte-identical to `main`" is a claim
    // about a diff and belongs in the gate, where Task 12 step 2 runs
    // `git diff --quiet main -- engine/src/backup.rs`; a test that greps for `pub fn` would pass on
    // a file somebody had rewritten, so it says only what it checks.
    let backup = std::fs::read_to_string(
        Path::new(env!("CARGO_MANIFEST_DIR")).join("..").join("engine").join("src").join("backup.rs"),
    ).expect("engine/src/backup.rs");
    assert!(backup.contains("pub fn"), "backup.rs is still here and still a module");
}
