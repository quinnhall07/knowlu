//! `tauri_build::build()` plus the engine's build-SHA logic, copied verbatim from the root
//! `build.rs`: the shell's `KNOWLU_BUILD_SHA` must be the same commit's short SHA the engine
//! embeds, so `commands::build_state_value` can compare `console_build` against
//! `state.topline.engine_build` (spec §15 Q1: "engine newer than console").
//!
//! Falls back to "unknown" outside a checkout; never fails the build.
//!
//! Reruns are pinned to the real HEAD file via `git rev-parse --git-path HEAD`, not a literal
//! `.git/HEAD` — this checkout can be a git *worktree*, where `.git` is a file and the real HEAD
//! lives under the main repo's `.git/worktrees/<name>/HEAD`; `--git-path` resolves correctly for
//! a worktree and a plain checkout alike. When that file holds a symbolic ref (the normal case,
//! not a detached HEAD), the branch's own ref file is watched too — otherwise a commit made on the
//! branch would never trigger a rebuild.
//!
//! **The sidecar placeholder** (ruling R-P4a-26). `bundle.externalBin` makes `tauri_build::build()`
//! copy `binaries/knowlu-engine-<target triple>.exe` next to the built exe (`tauri-build` 2.6.3,
//! `copy_binaries`), and a missing file is a hard error - `resource path ... doesn't exist` -
//! raised before a single test compiles. `app/binaries/` is git-ignored build output, so a fresh
//! clone, a fresh worktree, `cargo test` and the local runner's `cargo build --release` would all
//! fail on any checkout that has never run `scripts\release.ps1`. So when the sidecar is absent
//! this drops a ZERO-BYTE placeholder in its place.
//!
//! It is deliberately **silent**: `cargo:warning=` would add a line to every `cargo test`, and
//! zero new warnings is part of green in this repo.
//!
//! The placeholder can never ship. Plain `cargo` does not bundle at all, and `scripts\release.ps1`
//! refuses to bundle a sidecar smaller than 1 MiB, so only a real engine ever reaches an installer.
use std::process::Command;

fn main() {
    stage_sidecar_placeholder();
    tauri_build::build();

    let manifest_dir =
        std::env::var("CARGO_MANIFEST_DIR").unwrap_or_else(|_| ".".to_string());

    let git = |args: &[&str]| -> Option<String> {
        Command::new("git")
            .current_dir(&manifest_dir)
            .args(args)
            .output()
            .ok()
            .filter(|o| o.status.success())
            .map(|o| String::from_utf8_lossy(&o.stdout).trim().to_string())
    };

    let sha = git(&["rev-parse", "--short", "HEAD"]).unwrap_or_else(|| "unknown".to_string());
    println!("cargo:rustc-env=KNOWLU_BUILD_SHA={sha}");

    if let Some(head_path) = git(&["rev-parse", "--git-path", "HEAD"]) {
        println!("cargo:rerun-if-changed={head_path}");
        if let Ok(contents) = std::fs::read_to_string(&head_path) {
            if let Some(reference) = contents.trim().strip_prefix("ref: ") {
                if let Some(ref_path) = git(&["rev-parse", "--git-path", reference]) {
                    println!("cargo:rerun-if-changed={ref_path}");
                }
            }
        }
    }
}


/// See the module docs: keeps `cargo build` and `cargo test` working on a checkout that has never
/// staged the real engine. `TARGET` is set by cargo for every build script.
fn stage_sidecar_placeholder() {
    // Watched so that staging the real engine later re-runs this script - and `tauri-build` copies
    // the new bytes - instead of reusing a cached run that only ever saw the placeholder.
    println!("cargo:rerun-if-changed=binaries/");

    let Ok(target) = std::env::var("TARGET") else { return };
    // The third and last spelling of the sidecar stem. `scripts/release.ps1` cross-checks its own
    // against `bundle.externalBin`, and `app/tests/static_assets.rs` checks this one against the
    // same config, so the three cannot drift apart unnoticed.
    let path = format!("binaries/knowlu-engine-{target}.exe");
    if std::path::Path::new(&path).exists() {
        return;
    }
    if std::fs::create_dir_all("binaries").is_ok() {
        let _ = std::fs::File::create(&path);
    }
}
