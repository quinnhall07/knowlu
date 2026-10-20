//! Port of `engine/reconcile.py` — pure two-device reconciliation.
//!
//! Per field, the later timestamp wins and **the loser is recorded as `supersede`, never dropped**.
//! Nothing here touches the filesystem: it takes upstream frontmatter and two record lists and
//! returns what to apply. The sync loop that calls it is S2's, still suspended.
//!
//! # Deterministic for identical inputs
//!
//! Every `supersede` record's `ts` is the **later of the two contenders' timestamps**, never the
//! wall clock. `device` comes from the running machine (environment, not input) via `make_record`.
//! Both matter: a reconciliation replayed on two machines has to produce the same records, or the
//! union-merged journal grows a different history on each.

use std::collections::BTreeMap;

use serde_json::{Map, Value};

use crate::journal::{actor_rank, latest_by_field, make_record, NewRecord};
use crate::ledger::Record;

#[derive(Debug, Clone, Default, PartialEq)]
pub struct Resolution {
    /// field -> the value to write to the local note.
    pub apply: BTreeMap<String, Value>,
    pub supersede: Vec<Record>,
    pub notes: Vec<String>,
}

fn str_of(rec: &Record, key: &str) -> String {
    match rec.get(key) {
        Some(Value::String(s)) => s.clone(),
        _ => String::new(),
    }
}

fn value_of(rec: &Record, key: &str) -> Value {
    rec.get(key).cloned().unwrap_or(Value::Null)
}

/// The four fields that identify a record inside a `supersede`'s `old`/`new`.
pub fn record_key(rec: &Record) -> Value {
    let mut out = Map::new();
    out.insert("ts".into(), value_of(rec, "ts"));
    out.insert("device".into(), value_of(rec, "device"));
    out.insert("seq".into(), value_of(rec, "seq"));
    out.insert("field".into(), value_of(rec, "field"));
    Value::Object(out)
}

/// Later `ts` wins; on a tie the **lower** actor rank (quinn) wins; then device name, for
/// determinism.
///
/// Python spells the middle term `-actor_rank(...)` inside a tuple compared with `>=`, so a
/// smaller rank sorts higher. The tuple is `(ts, -rank, device)`.
fn sort_key(rec: &Record) -> (String, i64, String) {
    (str_of(rec, "ts"), -actor_rank(&str_of(rec, "actor")), str_of(rec, "device"))
}

/// Python's `wins(a, b)`: `a` on a tie, because the comparison is `>=`.
pub fn wins<'a>(a: &'a Record, b: &'a Record) -> &'a Record {
    if sort_key(a) >= sort_key(b) {
        a
    } else {
        b
    }
}

/// Resolve one note, field by field.
///
/// **Several offline writes to one field are ONE effective change** — earliest `old` to latest
/// `new` — and the latest record is the contender. Comparing the upstream value against the
/// *earliest* `old` is what makes "upstream never moved" detectable after a chain of local edits.
///
/// When upstream holds a value nobody journaled, it is a hand edit: it becomes a synthetic
/// `quinn`/`external` contender stamped with the upstream file's mtime, and a note says so. That
/// synthetic contender can still lose — a human write is not automatically newer.
#[allow(clippy::too_many_arguments)]
pub fn resolve(
    upstream_meta: &serde_yaml_ng::Mapping,
    upstream_records: &[Record],
    local_records: &[Record],
    upstream_mtime_ts: &str,
    note_id: Option<&str>,
    path: &str,
    via: &str,
) -> Resolution {
    let mut res = Resolution::default();

    // latest_by_field keys on (id, field); this collapses to field alone, last writer winning,
    // exactly as Python's dict comprehension over the same items does.
    let mut up_latest: BTreeMap<String, Record> = BTreeMap::new();
    for ((_, field), rec) in latest_by_field(upstream_records) {
        up_latest.insert(field, rec);
    }

    let mut chains: BTreeMap<String, Vec<Record>> = BTreeMap::new();
    for rec in local_records {
        if str_of(rec, "op") == "set" {
            let field = str_of(rec, "field");
            if !field.is_empty() {
                chains.entry(field).or_default().push(rec.clone());
            }
        }
    }

    for (field, chain) in chains {
        let mut chain = chain;
        chain.sort_by_key(sort_key);
        let first = chain.first().expect("a chain is never empty").clone();
        let local = chain.last().expect("a chain is never empty").clone();

        let upstream_value = crate::yaml::get(upstream_meta, &field)
            .map(crate::yaml::to_json)
            .unwrap_or(Value::Null);

        if upstream_value == value_of(&first, "old") {
            // Upstream never moved: no conflict, and no supersede record either.
            res.apply.insert(field.clone(), value_of(&local, "new"));
            continue;
        }

        let contender = match up_latest.get(&field) {
            Some(rec) if value_of(rec, "new") == upstream_value => rec.clone(),
            _ => {
                res.notes.push(format!(
                    "{field}: upstream value {} was an external write",
                    python_repr(&upstream_value)
                ));
                let mut synthetic = Map::new();
                synthetic.insert("ts".into(), Value::String(upstream_mtime_ts.to_string()));
                synthetic.insert("device".into(), Value::String("upstream".into()));
                synthetic.insert("seq".into(), Value::Number(0.into()));
                // Ruling 11 (D4): the human, under the new token. Never written (the supersede
                // keeps only `record_key`'s four fields); its actor only feeds `actor_rank`, where
                // both tokens rank 0 — so the outcome and every byte are unchanged.
                synthetic.insert("actor".into(), Value::String(crate::journal::HUMAN_ACTOR.into()));
                synthetic.insert("via".into(), Value::String("external".into()));
                synthetic.insert("field".into(), Value::String(field.clone()));
                synthetic.insert("old".into(), value_of(&first, "old"));
                synthetic.insert("new".into(), upstream_value.clone());
                synthetic
            }
        };

        // The same comparison `wins` makes, spelled inline so the branch below can name both
        // sides without an identity test on two borrows.
        let local_wins = sort_key(&local) >= sort_key(&contender);
        let (winner, loser) =
            if local_wins { (&local, &contender) } else { (&contender, &local) };
        if local_wins {
            res.apply.insert(field.clone(), value_of(&local, "new"));
        }

        let ts = std::cmp::max(str_of(winner, "ts"), str_of(loser, "ts"));
        let mut spec = NewRecord::new("supersede", path, "system:reconcile", via);
        spec.id = note_id;
        spec.field = Some(&field);
        spec.old = record_key(loser);
        spec.new = record_key(winner);
        spec.ts = Some(ts);
        if let Ok(rec) = make_record(spec) {
            res.supersede.push(rec);
        }
    }

    res
}

/// `{value!r}` for the value shapes that reach the note line.
///
/// Python interpolates the *repr* of a JSON-able value: a string gets single quotes, `None`
/// becomes `None`, `True`/`False` capitalise. This is a log line, not a data format, but it is a
/// log line the sync loop shows Quinn, so it is ported rather than approximated.
fn python_repr(value: &Value) -> String {
    match value {
        Value::Null => "None".to_string(),
        Value::Bool(true) => "True".to_string(),
        Value::Bool(false) => "False".to_string(),
        Value::String(s) => format!("'{}'", s.replace('\\', "\\\\").replace('\'', "\\'")),
        other => crate::ledger::dumps_value(other),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn rec(ts: &str, device: &str, field: &str, old: Value, new: Value, actor: &str, seq: i64) -> Record {
        let mut r = Map::new();
        r.insert("ts".into(), json!(ts));
        r.insert("device".into(), json!(device));
        r.insert("seq".into(), json!(seq));
        r.insert("actor".into(), json!(actor));
        r.insert("via".into(), json!("dashboard"));
        r.insert("op".into(), json!("set"));
        r.insert("id".into(), json!("task_a"));
        r.insert("path".into(), json!("tasks/a.md"));
        r.insert("field".into(), json!(field));
        r.insert("old".into(), old);
        r.insert("new".into(), new);
        r.insert("run_id".into(), Value::Null);
        r.insert("evidence".into(), Value::Null);
        r
    }

    fn quinn(ts: &str, device: &str, field: &str, old: Value, new: Value, seq: i64) -> Record {
        rec(ts, device, field, old, new, "quinn", seq)
    }

    fn up(yaml: &str) -> serde_yaml_ng::Mapping {
        crate::yaml::mapping_of(yaml)
    }

    fn go(
        upstream: &serde_yaml_ng::Mapping,
        upstream_records: &[Record],
        local_records: &[Record],
        mtime: &str,
    ) -> Resolution {
        resolve(upstream, upstream_records, local_records, mtime, Some("task_a"), "tasks/a.md", "local-runner")
    }

    #[test]
    fn different_fields_do_not_conflict() {
        let res = go(
            &up("progress: 0\ndue: '2026-09-05'\n"),
            &[quinn("2026-08-29T10:00:00.000Z", "desktop", "due", json!("2026-09-01"), json!("2026-09-05"), 1)],
            &[quinn("2026-08-29T10:01:00.000Z", "laptop", "progress", json!(0), json!(40), 1)],
            "2026-08-29T10:00:00.000Z",
        );
        assert_eq!(res.apply.get("progress"), Some(&json!(40)));
        assert_eq!(res.apply.len(), 1);
        assert!(res.supersede.is_empty());
    }

    /// VHL marks complete at 12:00 upstream; Quinn slides to 60% at 12:05 on the laptop.
    #[test]
    fn the_later_timestamp_wins_and_the_loser_is_superseded_not_dropped() {
        let vhl = rec("2026-08-29T12:00:00.000Z", "desktop", "progress", json!(10), json!(100),
                      "agent:coursework.vhl", 1);
        let q = quinn("2026-08-29T12:05:00.000Z", "laptop", "progress", json!(10), json!(60), 1);
        let res = go(&up("progress: 100\n"), &[vhl], &[q], "2026-08-29T12:00:00.000Z");

        assert_eq!(res.apply.get("progress"), Some(&json!(60)));
        assert_eq!(res.supersede.len(), 1);
        let sup = &res.supersede[0];
        assert_eq!(sup.get("op"), Some(&json!("supersede")));
        assert_eq!(sup.get("actor"), Some(&json!("system:reconcile")));
        assert_eq!(
            sup.get("old"),
            Some(&json!({"ts": "2026-08-29T12:00:00.000Z", "device": "desktop", "seq": 1, "field": "progress"}))
        );
        assert_eq!(
            sup.get("new"),
            Some(&json!({"ts": "2026-08-29T12:05:00.000Z", "device": "laptop", "seq": 1, "field": "progress"}))
        );
    }

    #[test]
    fn an_earlier_local_write_loses_and_is_not_applied() {
        let vhl = rec("2026-08-29T12:10:00.000Z", "desktop", "progress", json!(10), json!(100),
                      "agent:coursework.vhl", 1);
        let q = quinn("2026-08-29T12:05:00.000Z", "laptop", "progress", json!(10), json!(60), 1);
        let res = go(&up("progress: 100\n"), &[vhl], &[q], "2026-08-29T12:10:00.000Z");
        assert!(res.apply.is_empty());
        assert_eq!(res.supersede[0].get("old").unwrap().get("device"), Some(&json!("laptop")));
    }

    #[test]
    fn ties_break_human_over_agent_then_by_device_name() {
        let a = rec("2026-08-29T12:00:00.000Z", "desktop", "progress", json!(0), json!(1), "agent:x", 1);
        let q = quinn("2026-08-29T12:00:00.000Z", "laptop", "progress", json!(0), json!(2), 1);
        assert_eq!(wins(&a, &q), &q, "a human write outranks an agent's at the same instant");

        let d1 = quinn("2026-08-29T12:00:00.000Z", "desktop", "progress", json!(0), json!(1), 1);
        let d2 = quinn("2026-08-29T12:00:00.000Z", "laptop", "progress", json!(0), json!(2), 1);
        // Deterministic regardless of argument order.
        assert_eq!(wins(&d1, &d2), wins(&d2, &d1));
    }

    /// Upstream holds a value nobody journaled — a hand edit. It becomes a synthetic external
    /// contender stamped with the upstream file's mtime, and it can still win.
    #[test]
    fn an_unjournaled_upstream_value_is_treated_as_an_external_write() {
        let local = quinn("2026-08-29T09:00:00.000Z", "laptop", "progress", json!(0), json!(40), 1);
        let res = go(&up("progress: 55\n"), &[], &[local], "2026-08-29T10:00:00.000Z");
        assert!(res.apply.is_empty(), "the upstream mtime is later, so upstream wins");
        assert_eq!(res.supersede[0].get("old").unwrap().get("device"), Some(&json!("laptop")));
        assert!(res.notes.iter().any(|n| n.contains("external")), "{:?}", res.notes);
        assert_eq!(res.notes[0], "progress: upstream value 55 was an external write");
    }

    /// Ruling 11 (D4): the synthetic contender is the human, under the new token, and still wins a
    /// same-instant tie against an agent exactly as before. It is never written — the supersede
    /// record keeps only `record_key`'s four fields — so no byte carries its actor.
    #[test]
    fn an_external_write_still_wins_a_tie_as_the_human() {
        let local = rec("2026-08-29T10:00:00.000Z", "laptop", "progress", json!(0), json!(40), "agent:coursework.vhl", 1);
        let res = go(&up("progress: 55\n"), &[], &[local], "2026-08-29T10:00:00.000Z");
        assert!(res.apply.is_empty(), "the hand edit outranks the agent at the same instant");
        let sup = &res.supersede[0];
        assert_eq!(sup.get("new"), Some(&json!({"ts": "2026-08-29T10:00:00.000Z", "device": "upstream", "seq": 0, "field": "progress"})));
        assert_eq!(sup.get("old").unwrap().get("device"), Some(&json!("laptop")));
        assert!(!crate::ledger::dumps_value(&Value::Object(sup.clone())).contains("student"), "the contender's actor is never written");
        assert_eq!(res.notes, vec!["progress: upstream value 55 was an external write".to_string()]);
    }

    #[test]
    fn an_upstream_value_that_never_moved_is_not_a_conflict() {
        let local = quinn("2026-08-29T09:00:00.000Z", "laptop", "progress", json!(0), json!(40), 1);
        let res = go(
            &up("progress: 0\n"),
            &[quinn("2026-08-29T08:00:00.000Z", "desktop", "progress", json!(0), json!(0), 1)],
            &[local],
            "2026-08-29T08:00:00.000Z",
        );
        assert_eq!(res.apply.get("progress"), Some(&json!(40)));
        assert!(res.supersede.is_empty());
    }

    /// Several offline writes to one field are ONE effective change: earliest `old`, latest `new`.
    #[test]
    fn two_offline_local_writes_fold_into_one_change_in_either_order() {
        let a = quinn("2026-08-29T09:00:00.000Z", "laptop", "progress", json!(0), json!(20), 1);
        let b = quinn("2026-08-29T09:05:00.000Z", "laptop", "progress", json!(20), json!(40), 2);
        for order in [vec![a.clone(), b.clone()], vec![b.clone(), a.clone()]] {
            let res = go(&up("progress: 0\n"), &[], &order, "2026-08-29T08:00:00.000Z");
            assert_eq!(res.apply.get("progress"), Some(&json!(40)));
            assert!(res.supersede.is_empty());
            assert!(res.notes.is_empty());
        }
    }

    #[test]
    fn a_folded_chain_conflicts_through_its_latest_record() {
        let vhl = rec("2026-08-29T12:00:00.000Z", "desktop", "progress", json!(10), json!(100),
                      "agent:coursework.vhl", 1);
        let q1 = quinn("2026-08-29T11:50:00.000Z", "laptop", "progress", json!(10), json!(30), 1);
        let q2 = quinn("2026-08-29T12:05:00.000Z", "laptop", "progress", json!(30), json!(60), 2);
        let res = go(&up("progress: 100\n"), &[vhl], &[q1, q2], "2026-08-29T12:00:00.000Z");
        assert_eq!(res.apply.get("progress"), Some(&json!(60)));
        assert_eq!(res.supersede.len(), 1);
        assert_eq!(res.supersede[0].get("new").unwrap().get("seq"), Some(&json!(2)));
    }

    /// The supersede record's `ts` is the later of the two contenders, never the wall clock —
    /// otherwise a reconciliation replayed on two machines grows a different history on each.
    #[test]
    fn resolve_is_deterministic_and_stamps_the_later_contenders_timestamp() {
        let vhl = rec("2026-08-29T12:00:00.000Z", "desktop", "progress", json!(10), json!(100),
                      "agent:coursework.vhl", 1);
        let q = quinn("2026-08-29T12:05:00.000Z", "laptop", "progress", json!(10), json!(60), 1);
        let strip = |r: &Record| {
            let mut c = r.clone();
            c.remove("device"); // the running machine's name, not an input
            c.remove("seq");
            c
        };
        let first = go(&up("progress: 100\n"), &[vhl.clone()], &[q.clone()], "2026-08-29T12:00:00.000Z");
        let second = go(&up("progress: 100\n"), &[vhl.clone()], &[q.clone()], "2026-08-29T12:00:00.000Z");
        assert_eq!(
            first.supersede.iter().map(strip).collect::<Vec<_>>(),
            second.supersede.iter().map(strip).collect::<Vec<_>>()
        );
        assert_eq!(first.supersede[0].get("ts"), Some(&json!("2026-08-29T12:05:00.000Z")));
        assert_eq!(first.supersede[0].get("via"), Some(&json!("local-runner")));

        let other = resolve(
            &up("progress: 100\n"), &[vhl], &[q], "2026-08-29T12:00:00.000Z",
            Some("task_a"), "tasks/a.md", "dashboard",
        );
        assert_eq!(other.supersede[0].get("via"), Some(&json!("dashboard")));
    }
}
