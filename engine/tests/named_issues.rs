//! L7 (named issues), T1: the read model's half. Plan `docs/plans/2026-10-19-l7-named-issues-plan.md`
//! section 3.5. A source heads-up (`mail-source:`, `judge-service:`, `gmail-source:`) belongs in the
//! Issues panel, not in Good to know (Quinn's Q1, 2026-10-19), and a vault with none serialises exactly
//! as it did before.
//!
//! The vault is a scratch directory and the read model is built the way `surface_oracle.rs` builds it,
//! by running the binary; the keys are string literals, so this file compiles before any L7 code exists.
//! Nothing here touches a real vault, a socket or the clock (the date and the `now` are pinned).

use std::path::{Path, PathBuf};
use std::process::Command;

use knowlu_engine::info::{self, NewInfo};
use knowlu_engine::write::WriteContext;
use serde_json::Value;

const PINNED_DATE: &str = "2026-08-28";
const PINNED_NOW: &str = "2026-08-28T09:00";

fn scratch(tag: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!("knowlu-named-issues-{tag}-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).expect("scratch vault");
    dir
}

fn plant(vault: &Path, kind: &str, key: Option<&str>, title: &str, body: &str) {
    let ctx = WriteContext::new("agent:knowlu.test", "local-runner");
    let item = NewInfo { title, kind, body, opened_by: "agent:knowlu.test", close_key: key, expires: None };
    info::open_info(vault, &item, &ctx, None, None).expect("the heads-up files");
}

fn surface(vault: &Path) -> Value {
    let out = Command::new(env!("CARGO_BIN_EXE_knowlu-engine"))
        .args(["surface", "--vault"])
        .arg(vault)
        .args(["--view", "today", "--today", PINNED_DATE, "--now", PINNED_NOW, "--build-sha", "pinned"])
        .output()
        .expect("run knowlu-engine surface");
    assert!(out.status.success(), "surface failed: {}", String::from_utf8_lossy(&out.stderr));
    serde_json::from_slice(&out.stdout).expect("the read model is one JSON document")
}

fn keys_of(items: &Value) -> Vec<String> {
    items
        .as_array()
        .expect("an array")
        .iter()
        .map(|i| i["close_key"].as_str().unwrap_or_default().to_string())
        .collect()
}

/// Test 10. Source heads-ups list under `issues_panel.sources` in path order, each with its title, its
/// sentence and the day it opened; the other two items stay in Good to know; the one open number and
/// both nav badges follow. Guards plan 3.5 and Q1. Red today: `sources` is absent and all four items
/// sit in `good_to_know`.
#[test]
fn the_read_model_lists_source_issues_in_the_issues_panel() {
    let vault = scratch("panel");
    plant(&vault, "heads-up", Some("mail-source:a1"), "Forwarding paused", "Your mailbox stopped forwarding.");
    plant(&vault, "heads-up", Some("mail-reenabled:a1"), "Forwarding is back", "Forwarding is on again.");
    plant(&vault, "heads-up", Some("gmail-source:quiet"), "Gmail has gone quiet", "No mail found for a week.");
    plant(&vault, "notice", None, "A package arrived", "It is at the front desk.");

    let state = surface(&vault);
    let panel = &state["issues_panel"];

    let sources = panel["sources"].as_array().unwrap_or_else(|| panic!("issues_panel.sources is absent: {panel}"));
    let mut found = keys_of(&panel["sources"]);
    found.sort();
    assert_eq!(found, vec!["gmail-source:quiet".to_string(), "mail-source:a1".to_string()], "{panel}");
    let paths: Vec<&str> = sources.iter().map(|s| s["path"].as_str().expect("a path")).collect();
    let mut sorted = paths.clone();
    sorted.sort();
    assert_eq!(paths, sorted, "rows are in the path order list_info returns");
    for row in sources {
        assert!(row["title"].as_str().is_some_and(|t| !t.is_empty()), "{row}");
        assert!(row["since"].as_str().is_some_and(|s| !s.is_empty()), "{row}");
    }
    let by_key = |key: &str| sources.iter().find(|s| s["close_key"] == key).unwrap_or_else(|| panic!("{key}: {panel}"));
    assert_eq!(by_key("gmail-source:quiet")["text"], "No mail found for a week.");
    assert_eq!(by_key("mail-source:a1")["text"], "Your mailbox stopped forwarding.");

    let mut rest = keys_of(&state["good_to_know"]);
    rest.sort();
    assert_eq!(rest, vec!["".to_string(), "mail-reenabled:a1".to_string()], "the other two stay: {}", state["good_to_know"]);

    assert_eq!(panel["open_count"], 2, "{panel}");
    assert!(panel["empty_text"].is_null(), "{panel}");
    assert_eq!(state["nav_counts"]["issues"]["count"], 2, "{}", state["nav_counts"]);
    assert_eq!(state["nav_counts"]["good_to_know"]["count"], 2, "{}", state["nav_counts"]);
    let _ = std::fs::remove_dir_all(&vault);
}

/// Test 11. With no source heads-up, `issues_panel` has no `sources` key at all and is otherwise
/// today's three keys. This is the direct pin behind the frozen `surface-today-*.json` references, which
/// no vault of ours has a source heads-up to disturb. Green at T1 and must stay green.
#[test]
fn an_issues_panel_with_no_sources_serialises_without_the_key() {
    let vault = scratch("empty");
    let state = surface(&vault);
    let panel = state["issues_panel"].as_object().expect("issues_panel is an object");
    assert!(!panel.contains_key("sources"), "{panel:?}");
    let mut keys: Vec<&str> = panel.keys().map(String::as_str).collect();
    keys.sort();
    assert_eq!(keys, vec!["empty_text", "open_count", "rows"], "{panel:?}");
    assert_eq!(panel["open_count"], 0);
    let _ = std::fs::remove_dir_all(&vault);
}
