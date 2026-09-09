//! The judgment log (Knowlu spec §5.4, §5.6, decision 13): what was judged, by which tier, how
//! sure, and what was written — **by id and by field value, never by content.**
//!
//! **Where it is not.** Not in the vault, so `backup::tick` (which mirrors the vault and nothing
//! else) never copies it, `history::sync` never commits it, and a second device replaying the
//! journal never receives it. It lives under the *profile's* app-data directory —
//! `%LOCALAPPDATA%\knowlu\profiles\<id>\judgments\` — which is per profile because two vaults on one
//! machine are two different sets of judgments, unlike `updates\`, which is one bundle for the whole
//! install.
//!
//! **What it is for.** Two things. Today it is the only place that answers "why is nothing being
//! enriched?" — a week of `model not installed` lines is the answer. Tomorrow it is 3b's seed
//! corpus: a rule that reproduces the model's output across repeated cases is what gets proposed for
//! promotion into tier 2, and this file is the evidence for that.
//!
//! **The shape is a `JsonlLedger`**, the same day-file, `merge=union`, fsync-on-append primitive the
//! journal and the interaction events use — so the `ts`-decides-the-day rule, the sort order and
//! `ledger::dumps_value`'s Python separators all come for free and cannot drift from the other two.
//!
//! **The split, for `Outcome::LowConfidence`.** `judge::LowCause` says *why* structurally —
//! `"below floor"`, `"incomplete"`, `"model failed"` — and that closed vocabulary is what this log
//! records. The full text (`why`), which on the `ModelFailed` branch carries the runtime's own
//! stderr, goes to stdout instead — Task 7 prints it, Task 9 captures it into the slot log — and
//! never into this file. Without the split, all three collapse to the one label `"low confidence"`
//! and a runtime that installs but does not run reads identically, forever, to an answer that was
//! merely unsure.

use std::path::Path;

use serde_json::{json, Value};

use crate::journal::{device_name, now_ts};
use crate::ledger::{JsonlLedger, Record};

/// One judgment, ready to record.
///
/// **There is nowhere in this struct to put a title, a body or a prompt**, and that is the design:
/// §5.6 is enforced by the type, not by the discipline of whoever calls it. `fields` is what was (or
/// would have been) written — field name to the literal — which are values the note itself carries
/// in the open.
#[derive(Debug, Clone)]
pub struct Entry<'a> {
    /// The note's opaque `id:`.
    pub id: &'a str,
    /// 0 nothing, 1 heuristics, 2 a promoted rule, 3 the model.
    pub tier: u8,
    /// `Outcome::label()`.
    pub outcome: &'a str,
    pub confidence: f64,
    pub fields: &'a [(String, String)],
    pub run_id: Option<&'a str>,
    /// How long the judgment took, milliseconds — what tells a slow model from a slow disk.
    pub ms: i64,
    /// `Some(LowCause::label())` on `Outcome::LowConfidence`, `None` on every other outcome.
    ///
    /// **`&'static str` from a closed three-value enum is the whole safety argument here** —
    /// unlike `why` (which this struct still has nowhere to put), there is no value `cause` can
    /// hold that came from a note, a model, or a process, so the privacy property is structural
    /// rather than a matter of care at each call site.
    pub cause: Option<&'static str>,
}

/// Build an `Entry` from an `Outcome`, so the tier, the confidence and the label are read off the
/// outcome in one place rather than at each call site.
pub fn entry_for<'a>(
    id: &'a str,
    outcome: &crate::judge::Outcome,
    fields: &'a [(String, String)],
    run_id: Option<&'a str>,
    ms: i64,
) -> Entry<'a> {
    let verdict = outcome.verdict();
    let cause = match outcome {
        crate::judge::Outcome::LowConfidence { cause, .. } => Some(cause.label()),
        _ => None,
    };
    Entry {
        id,
        tier: verdict.tier,
        outcome: outcome.label(),
        confidence: verdict.confidence,
        fields,
        run_id,
        ms,
        cause,
    }
}

/// Append one line to `<dir>/<UTC day>.jsonl`.
///
/// `Err` when the directory cannot be made or the append fails. The caller **reports it and carries
/// on**: the judgment is the work and the log is the record of it, so a full disk must not stop a
/// slot from enriching.
pub fn record(dir: &Path, entry: &Entry<'_>, now: Option<jiff::Timestamp>) -> Result<(), String> {
    let mut fields = serde_json::Map::new();
    for (name, literal) in entry.fields {
        fields.insert(name.clone(), Value::String(literal.clone()));
    }
    let mut rec: Record = Record::new();
    rec.insert("ts".into(), json!(now_ts(now)));
    rec.insert("device".into(), json!(device_name()));
    rec.insert("id".into(), json!(entry.id));
    rec.insert("tier".into(), json!(entry.tier));
    rec.insert("outcome".into(), json!(entry.outcome));
    rec.insert("confidence".into(), json!(entry.confidence));
    rec.insert("fields".into(), Value::Object(fields));
    rec.insert("run_id".into(), entry.run_id.map(|r| json!(r)).unwrap_or(Value::Null));
    rec.insert("ms".into(), json!(entry.ms));
    rec.insert("cause".into(), entry.cause.map(|c| json!(c)).unwrap_or(Value::Null));
    JsonlLedger::new(dir).append(&rec).map_err(|e| format!("judgment log: {e}"))
}

/// One day's records, in file order. Used by the app's settings row and by 3b's promotion loop; a
/// missing directory or an unreadable line is empty or skipped, never an error — this is a report,
/// not a source of truth.
///
/// Routed through `JsonlLedger::read` rather than opening the day file itself — bounding the
/// range to `[day, day]` (the day-level filename compare and the record-level `ts` compare both
/// key off the same 10-character prefix) picks out exactly that day's records without this
/// module naming the ledger's file extension, which is `ledger.rs`'s alone to do
/// (`only_this_module_opens_ledger_files`).
pub fn read_day(dir: &Path, day: &str) -> Vec<Value> {
    JsonlLedger::new(dir).read(Some(day), Some(day)).into_iter().map(Value::Object).collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn dir(tag: &str) -> std::path::PathBuf {
        let d = std::env::temp_dir().join(format!("qo-judgelog-{tag}-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&d);
        d
    }

    fn fields() -> Vec<(String, String)> {
        vec![
            ("course".to_string(), "cs-100".to_string()),
            ("effort_hours".to_string(), "2.5".to_string()),
            ("importance".to_string(), "4".to_string()),
        ]
    }

    /// Only `ledger.rs` names a ledger file directly (`only_this_module_opens_ledger_files`);
    /// find the day file a write produced rather than spelling its extension here.
    fn day_file(dir: &std::path::Path) -> std::path::PathBuf {
        std::fs::read_dir(dir)
            .unwrap()
            .flatten()
            .find(|e| !e.file_name().to_string_lossy().starts_with('.'))
            .expect("the write created a day file")
            .path()
    }

    #[test]
    fn a_record_lands_in_the_utc_day_file_and_reads_back() {
        let d = dir("day");
        let now: jiff::Timestamp = "2026-09-07T23:30:00Z".parse().unwrap();
        let f = fields();
        let e = Entry {
            id: "task_0123456789",
            tier: 3,
            outcome: "answered",
            confidence: 0.82,
            fields: &f,
            run_id: Some("local-2026-09-07T18:00:00Z"),
            ms: 940,
            cause: None,
        };
        record(&d, &e, Some(now)).unwrap();
        assert_eq!(
            day_file(&d).file_stem().unwrap(),
            "2026-09-07",
            "the UTC day of the record's own ts"
        );
        let back = read_day(&d, "2026-09-07");
        assert_eq!(back.len(), 1);
        let r = &back[0];
        assert_eq!(r["id"], "task_0123456789");
        assert_eq!(r["tier"], 3);
        assert_eq!(r["outcome"], "answered");
        assert_eq!(r["confidence"], 0.82);
        assert_eq!(r["run_id"], "local-2026-09-07T18:00:00Z");
        assert_eq!(r["ms"], 940);
        assert_eq!(r["fields"]["course"], "cs-100");
        assert_eq!(r["fields"]["importance"], "4");
        assert!(r["device"].is_string(), "which machine judged it");
        assert!(r["cause"].is_null(), "no cause outside LowConfidence");
        let _ = std::fs::remove_dir_all(&d);
    }

    /// D9 / spec §5.6. **What this proves, precisely.** `Entry` has no field for a title, a body
    /// or a prompt — that is a structural guarantee no test is needed to prove, since there is
    /// nowhere for a caller to put one. What THIS test pins is narrower and still worth pinning:
    /// the **serialiser** (`record`) adds nothing beyond the fields `Entry` declares — in
    /// particular, that the `LowConfidence` split introduced in fix round 1 writes `cause`'s
    /// closed vocabulary and never `why`'s free text, even when `why` is populated with something
    /// that reads like it came from a process (a "stderr" line).
    ///
    /// **What this test structurally cannot cover.** `fields` carries free-text values verbatim,
    /// by design — that is what lets `importance_reason` reach the log at all. A call site that
    /// passed a note's *title* as a `fields` value would defeat every guarantee here, and no test
    /// in this module can catch it: the type permits it. That check belongs at the call site
    /// (Task 7), not here.
    #[test]
    fn no_note_text_ever_reaches_a_log_line() {
        let d = dir("private");
        let f = vec![("importance_reason".to_string(), "Worth 15% of the grade.".to_string())];
        let e = Entry {
            id: "task_0123456789",
            tier: 3,
            outcome: "answered",
            confidence: 0.9,
            fields: &f,
            run_id: None,
            ms: 12,
            cause: None,
        };
        record(&d, &e, Some("2026-09-07T12:00:00Z".parse().unwrap())).unwrap();
        let text = std::fs::read_to_string(day_file(&d)).unwrap();
        // `importance_reason` IS a field value and is meant to be here; a title and a body are not,
        // and `Entry` has nowhere to put them — this asserts the shape as well as the content.
        assert!(text.contains("Worth 15% of the grade."), "a written field value is the record");
        for forbidden in ["Title:", "Body:", "prompt", "tasks/", "\\path"] {
            assert!(!text.contains(forbidden), "{forbidden} must never reach a log line: {text}");
        }
        let _ = std::fs::remove_dir_all(&d);

        // The `cause` vocabulary: a `ModelFailed` outcome carries `why` text that looks exactly
        // like the thing this test exists to keep out — the runtime's own stderr. `cause` must
        // reach the line; `why` must not, under any key.
        let d = dir("private-cause");
        let v = crate::judge::Verdict::default();
        let stderr_secret = "llama-cli: panicked at prompt.cpp:88: token buffer overrun";
        let low = crate::judge::Outcome::LowConfidence {
            seed: v,
            why: stderr_secret.to_string(),
            cause: crate::judge::LowCause::ModelFailed,
        };
        let empty: Vec<(String, String)> = Vec::new();
        let e = entry_for("task_0123456789", &low, &empty, None, 4);
        record(&d, &e, Some("2026-09-07T12:00:00Z".parse().unwrap())).unwrap();
        let text = std::fs::read_to_string(day_file(&d)).unwrap();
        assert!(text.contains("\"cause\": \"model failed\""), "{text}");
        assert!(!text.contains(stderr_secret), "why's free text must never reach a log line: {text}");
        assert!(!text.contains("\"why\""), "the `why` key itself must never appear: {text}");
        let _ = std::fs::remove_dir_all(&d);
    }

    /// Every outcome is logged, not only the ones that wrote something: a week of "model not
    /// installed" is exactly what tells Quinn why nothing is being enriched.
    #[test]
    fn every_outcome_is_recorded_with_the_tier_that_reached_it() {
        let empty: Vec<(String, String)> = Vec::new();
        let v = crate::judge::Verdict { confidence: 1.0, tier: 1, ..Default::default() };
        let e = entry_for("task_0123456789", &crate::judge::Outcome::RuntimeNotInstalled(v.clone()), &empty, None, 3);
        assert_eq!(e.outcome, "runtime not installed");
        assert_eq!(e.tier, 1);
        assert_eq!(e.confidence, 1.0);
        assert_eq!(e.cause, None, "cause is only structural on LowConfidence");

        let low = crate::judge::Outcome::LowConfidence {
            seed: v,
            why: "confidence 0.41 below 0.60".to_string(),
            cause: crate::judge::LowCause::BelowFloor,
        };
        let e = entry_for("task_0123456789", &low, &empty, None, 5);
        assert_eq!(e.outcome, "low confidence");
        assert_eq!(e.cause, Some("below floor"));
    }

    /// The collapse fix round 1 exists to prevent: three structurally different failures must not
    /// read as the same line. All three share `outcome`; none share `cause`.
    #[test]
    fn the_three_low_confidence_causes_are_distinguishable_in_the_log() {
        let empty: Vec<(String, String)> = Vec::new();
        let seed = crate::judge::Verdict::default();
        let causes = [
            (crate::judge::LowCause::BelowFloor, "below floor"),
            (crate::judge::LowCause::Incomplete, "incomplete"),
            (crate::judge::LowCause::ModelFailed, "model failed"),
        ];
        let mut seen = std::collections::HashSet::new();
        for (cause, label) in causes {
            let outcome = crate::judge::Outcome::LowConfidence {
                seed: seed.clone(),
                why: "irrelevant here".to_string(),
                cause,
            };
            let e = entry_for("task_0123456789", &outcome, &empty, None, 1);
            assert_eq!(e.outcome, "low confidence", "all three agree in outcome");
            assert_eq!(e.cause, Some(label));
            assert!(seen.insert(e.cause), "each cause must be distinct: {label}");
        }
        assert_eq!(seen.len(), 3);
    }

    /// A directory that cannot be created is an error the caller reports, never a panic and never a
    /// reason to stop enriching: the judgment is the work, the log is the record of it.
    #[test]
    fn a_log_that_cannot_be_written_is_an_error_not_a_panic() {
        let d = dir("blocked");
        std::fs::create_dir_all(d.parent().unwrap()).unwrap();
        // A FILE where the directory should be: `create_dir_all` fails on it every time.
        std::fs::write(&d, b"not a directory").unwrap();
        let f = fields();
        let e = Entry {
            id: "task_0123456789",
            tier: 1,
            outcome: "answered",
            confidence: 1.0,
            fields: &f,
            run_id: None,
            ms: 1,
            cause: None,
        };
        assert!(record(&d, &e, None).is_err());
        let _ = std::fs::remove_file(&d);
    }

    #[test]
    fn read_day_of_a_missing_directory_is_empty() {
        assert!(read_day(&dir("absent"), "2026-09-07").is_empty());
    }
}
