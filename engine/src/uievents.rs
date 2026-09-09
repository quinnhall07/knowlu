//! Interaction events (S2 §7.6, console spec §7.5): what the page saw and what was clicked,
//! **ids only, never a title or a body**, through the same `JsonlLedger` the journal uses, to
//! `state/events-ui/YYYY-MM-DD.jsonl` (UTC day from the record's own `ts`, `merge=union`).
//! Git-ignored by default; `config/planning.yaml`'s `commit_ui_events: true` lets `history`
//! stage the folder. **Never read by the engine** — `tests/uievents_isolation.rs` proves it.
//! The console is the only writer.
use std::path::Path;
use serde_json::{json, Value};
use crate::ledger::{JsonlLedger, Record};

pub const ACTIONS: [&str; 11] = [
    "view_opened", "object_seen", "edit_started", "edit_committed", "edit_cancelled",
    "decision_made", "decision_deferred", "issue_opened", "sync_run", "delta_expanded", "why_expanded",
];

#[derive(Debug, Clone)]
pub struct UiEvent<'a> {
    pub session: &'a str,
    pub view: &'a str,
    pub action: &'a str,
    pub object_id: Option<&'a str>,
    pub object_kind: Option<&'a str>,
    pub ms: Option<i64>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum UiEventError { UnknownAction(String), FreeText(&'static str), Ledger(String) }

impl std::fmt::Display for UiEventError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            UiEventError::UnknownAction(a) => write!(f, "unknown ui action {a:?}"),
            UiEventError::FreeText(which) => write!(f, "{which} must be an id, not text"),
            UiEventError::Ledger(e) => write!(f, "events-ui ledger: {e}"),
        }
    }
}

fn is_token(s: &str) -> bool {
    !s.is_empty() && s.len() <= 64 && s.chars().all(|c| c.is_ascii_alphanumeric() || c == '_' || c == '-' || c == ':')
}

pub fn record(vault: &Path, ev: &UiEvent<'_>, now: Option<jiff::Timestamp>) -> Result<(), UiEventError> {
    if !ACTIONS.contains(&ev.action) { return Err(UiEventError::UnknownAction(ev.action.to_string())); }
    if let Some(id) = ev.object_id { if !crate::ids::is_id(id) { return Err(UiEventError::FreeText("object_id")); } }
    if let Some(k) = ev.object_kind { if !is_token(k) { return Err(UiEventError::FreeText("object_kind")); } }
    if !is_token(ev.session) { return Err(UiEventError::FreeText("session")); }
    if !is_token(ev.view) { return Err(UiEventError::FreeText("view")); }
    let mut rec: Record = Record::new();
    rec.insert("ts".into(), json!(crate::journal::now_ts(now)));
    rec.insert("device".into(), json!(crate::journal::device_name()));
    rec.insert("session".into(), json!(ev.session));
    rec.insert("view".into(), json!(ev.view));
    rec.insert("action".into(), json!(ev.action));
    rec.insert("object_id".into(), ev.object_id.map(|s| json!(s)).unwrap_or(Value::Null));
    rec.insert("object_kind".into(), ev.object_kind.map(|s| json!(s)).unwrap_or(Value::Null));
    rec.insert("ms".into(), ev.ms.map(|m| json!(m)).unwrap_or(Value::Null));
    JsonlLedger::new(vault.join("state").join("events-ui")).append(&rec).map_err(|e| UiEventError::Ledger(format!("{e:?}")))
}

/// `config/planning.yaml` → `commit_ui_events: true`. Absent, unreadable or anything else → false.
pub fn commit_opt_in(vault: &Path) -> bool {
    crate::pystr::read_text(&vault.join("config").join("planning.yaml")).ok()
        .and_then(|t| serde_yaml_ng::from_str::<serde_yaml_ng::Value>(&t).ok())
        .and_then(|v| v.get("commit_ui_events").and_then(|b| b.as_bool()))
        .unwrap_or(false)
}

#[cfg(test)]
mod tests {
    use super::*;
    fn scratch() -> std::path::PathBuf {
        // The brief's sketch used a pid-only name, shared by all three tests in this module —
        // cargo runs unit tests in parallel by default, so that races `remove_dir_all` in one
        // test against `create_dir_all` in another. A counter (the `TempDir` pattern in
        // `ledger.rs`'s tests) makes every call's directory unique.
        use std::sync::atomic::{AtomicU32, Ordering};
        static COUNTER: AtomicU32 = AtomicU32::new(0);
        let d = std::env::temp_dir().join(format!(
            "qo-uievents-{}-{}",
            std::process::id(),
            COUNTER.fetch_add(1, Ordering::Relaxed)
        ));
        let _ = std::fs::remove_dir_all(&d);
        std::fs::create_dir_all(d.join("config")).unwrap();
        std::fs::write(d.join("config/planning.yaml"), "daily_effort_budget: 4.0\n").unwrap();
        d
    }
    #[test]
    fn an_event_is_one_sorted_python_json_line_in_the_utc_day_file() {
        // Only `ledger.rs` names a ledger file directly (`only_this_module_opens_ledger_files`);
        // find the day file the write produced rather than spelling its extension here.
        let v = scratch();
        let ev = UiEvent { session: "s1", view: "today", action: "object_seen", object_id: Some("task_0123456789"), object_kind: Some("task"), ms: Some(2400) };
        record(&v, &ev, Some("2026-09-04T03:15:00Z".parse().unwrap())).unwrap();
        let events_dir = v.join("state").join("events-ui");
        let day_file = std::fs::read_dir(&events_dir)
            .unwrap()
            .flatten()
            .find(|e| !e.file_name().to_string_lossy().starts_with('.'))
            .expect("the write created a day file")
            .path();
        assert_eq!(day_file.file_stem().unwrap(), "2026-09-04");
        let text = std::fs::read_to_string(&day_file).unwrap();
        assert_eq!(text.lines().count(), 1);
        let line = text.lines().next().unwrap();
        assert!(line.starts_with("{\"action\": \"object_seen\", \"device\": "), "{line}");
        assert!(line.contains("\"ms\": 2400, \"object_id\": \"task_0123456789\", \"object_kind\": \"task\", \"session\": \"s1\", \"ts\": \"2026-09-04T03:15:00.000Z\", \"view\": \"today\"}"), "{line}");
    }
    #[test]
    fn unknown_actions_and_free_text_are_refused() {
        let v = scratch();
        let bad = UiEvent { session: "s", view: "today", action: "typed_a_title", object_id: None, object_kind: None, ms: None };
        assert!(matches!(record(&v, &bad, None), Err(UiEventError::UnknownAction(_))));
        let text = UiEvent { session: "s", view: "today", action: "edit_started", object_id: Some("Read chapter 3"), object_kind: None, ms: None };
        assert!(matches!(record(&v, &text, None), Err(UiEventError::FreeText(_))), "an object_id that is not an id is free text");
        assert!(!v.join("state/events-ui").exists());
    }
    #[test]
    fn commit_opt_in_defaults_to_false() {
        let v = scratch();
        assert!(!commit_opt_in(&v));
        std::fs::write(v.join("config/planning.yaml"), "daily_effort_budget: 4.0\ncommit_ui_events: true\n").unwrap();
        assert!(commit_opt_in(&v));
    }
}
