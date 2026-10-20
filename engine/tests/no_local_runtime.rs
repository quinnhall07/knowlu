//! Ruling 10 (Amendment 2026-09-29), lane L6, plan T1: **the tripwire.** Nothing model-shaped is left
//! on the device: no local runtime driver, no installer, no GBNF grammar, no `llama` anywhere in the
//! engine's or the app's source, the page, or either manifest. It fails while any listed file exists
//! or any listed token is named, reporting `file:line (token)` for every hit, and goes green only when
//! the removal is complete. Comments are scanned on purpose: a doc that still describes a removed path
//! is a bug report waiting to be filed. Like `human_actor_literal.rs`, it reads the workspace from
//! `CARGO_MANIFEST_DIR/..`. Test directories are not scanned (this file names every token).

use std::path::{Path, PathBuf};

/// Bare `runtime::` is not here on purpose: it would match `tauri::async_runtime::`.
const TOKENS: [&str; 23] = [
    "llama",
    "gguf",
    "KNOWLU_RUNTIME",
    "KNOWLU_MODEL",
    "RUNTIME_EXE",
    "SUPPORTED_RUNTIMES",
    "PerCall",
    "inference_status",
    "install_inference",
    "remove_inference_model",
    "inference_root",
    "offer_inference",
    "offer-inference",
    "set-judge",
    "RuntimeNotInstalled",
    "ModelNotInstalled",
    "mod inference",
    "mod runtime",
    "runtime.rs",
    "inference.rs",
    "crate::runtime",
    "crate::inference",
    "knowlu_engine::runtime",
];

fn root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("..")
}

/// Every file under `dir` that `keep` accepts, recursively, in path order.
fn files(dir: &Path, keep: &dyn Fn(&Path) -> bool) -> Vec<PathBuf> {
    let mut out = Vec::new();
    for entry in std::fs::read_dir(dir).into_iter().flatten().flatten() {
        let path = entry.path();
        if path.is_dir() {
            out.extend(files(&path, keep));
        } else if keep(&path) {
            out.push(path);
        }
    }
    out.sort();
    out
}

fn rel(path: &Path) -> String {
    path.strip_prefix(root()).unwrap_or(path).to_string_lossy().replace('\\', "/")
}

/// `file:line (token)` for every token named in `text`, one entry per line and token.
fn hits_in(name: &str, text: &str) -> Vec<String> {
    let mut out = Vec::new();
    for (n, line) in text.lines().enumerate() {
        for token in TOKENS {
            if line.contains(token) {
                out.push(format!("{name}:{} ({token})", n + 1));
            }
        }
    }
    out
}

/// The files the scan covers: engine and app Rust source, the page, and the two manifests.
fn scanned() -> Vec<PathBuf> {
    let rs = |p: &Path| p.extension().is_some_and(|e| e == "rs");
    let any = |_: &Path| true;
    let mut out = files(&root().join("engine/src"), &rs);
    out.extend(files(&root().join("app/src"), &rs));
    out.extend(files(&root().join("app/static"), &any));
    out.push(root().join("app/Cargo.toml"));
    out.push(root().join("engine/Cargo.toml"));
    out
}

#[test]
fn the_runtime_files_are_gone() {
    let still_there: Vec<String> = ["engine/src/runtime.rs", "app/src/inference.rs", "app/tests/inference.rs"]
        .iter()
        .filter(|p| root().join(p).exists())
        .map(|p| p.to_string())
        .collect();
    assert!(
        still_there.is_empty(),
        "ruling 10: the local runtime and its installer are removed, but these files still exist:\n{}",
        still_there.join("\n")
    );
}

#[test]
fn nothing_names_the_local_runtime() {
    let mut hits = Vec::new();
    for f in scanned() {
        // Binary assets (images, fonts) are not text; a lossy read keeps them from aborting the scan.
        let text = String::from_utf8_lossy(&std::fs::read(&f).unwrap()).to_string();
        hits.extend(hits_in(&rel(&f), &text));
    }
    assert!(
        hits.is_empty(),
        "ruling 10 (L6): these lines still name the removed local runtime, model or installer:\n{}",
        hits.join("\n")
    );
}

#[test]
fn the_app_takes_no_zip_dependency() {
    let manifest = std::fs::read_to_string(root().join("app/Cargo.toml")).unwrap();
    let zip: Vec<&str> = manifest.lines().filter(|l| l.trim_start().starts_with("zip")).collect();
    assert!(zip.is_empty(), "app/Cargo.toml still lists zip, which only the runtime installer used:\n{}", zip.join("\n"));
}

#[test]
fn the_scanner_is_not_vacuous() {
    let hits = hits_in("x.rs", "fn a() {}\n// the llama driver\nlet _ = \"KNOWLU_MODEL\";\n");
    assert_eq!(hits, vec!["x.rs:2 (llama)".to_string(), "x.rs:3 (KNOWLU_MODEL)".to_string()]);
    assert!(hits_in("x.rs", "tauri::async_runtime::spawn(f);\n").is_empty(), "bare runtime:: is legal");
    assert!(!scanned().is_empty(), "the scan covers files");
}
