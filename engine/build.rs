//! Embeds the checkout's short git SHA as KNOWLU_BUILD_SHA so a binary can say which commit it
//! is (spec §15 Q1: the console shows `engine newer than console` when the two disagree). Falls
//! back to "unknown" outside a checkout; never fails the build.
//!
//! Reruns are pinned to the real HEAD file via `git rev-parse --git-path HEAD`, not a literal
//! `.git/HEAD` — this checkout can be a git *worktree*, where `.git` is a file and the real HEAD
//! lives under the main repo's `.git/worktrees/<name>/HEAD`; `--git-path` resolves correctly for
//! a worktree and a plain checkout alike. When that file holds a symbolic ref (the normal case,
//! not a detached HEAD), the branch's own ref file is watched too — otherwise a commit made on the
//! branch would never trigger a rebuild.
use std::process::Command;

fn main() {
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
