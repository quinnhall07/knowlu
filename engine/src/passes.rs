//! Port of `engine/passes.py` — the run-start passes over the journal.
//!
//! Two passes, run in this order at the top of every rank, both no-ops on a clean vault:
//!
//! 1. [`verify_tail`] heals a write whose journal record landed but whose note did not — the
//!    process died in between. Journal-first ordering is what makes that recoverable at all.
//! 2. [`detect_external`] journals the edits Quinn made in Obsidian, which bypass `engine::write`
//!    entirely. Without it those values look, to the next agent write, like values nobody set.
//!
//! # The unmigrated-vault guard is the most important thing in this file
//!
//! A vault with real notes but no `system:migration` record anywhere in its journal has never been
//! through `scripts/migrate_s1.py`. Left unguarded, `detect_external` would journal every
//! pre-existing note as a `quinn`/`external` create and permanently lock every judged field via
//! judge-once — a false "Quinn already set this by hand" for fields nobody ever judged.
//!
//! The guard keys on `system:migration` **specifically**, not on "any create": ingest, coursework
//! and the events digest all mint notes through `write::create` in the same run, so one Blackboard
//! assignment arriving on an unmigrated vault would satisfy a plain any-create test and unlock the
//! whole destructive path for every other note.
//!
//! # Documented deviation: iteration order
//!
//! Python's `latest_by_field` returns a dict in insertion order; this crate's returns a `BTreeMap`
//! sorted by `(id, field)`. Each entry names a distinct field, so the *work* is identical either
//! way; only the order of the log lines can differ, and those reach the run record, not
//! `today.md`. Recorded rather than worked around — the sorted order is also the reproducible one.

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

use jiff::civil::Date;
use jiff::Timestamp;
use serde_json::Value;

use crate::ids::{build_index, is_id, read_meta, rel, scan_notes};
use crate::ingest::update_frontmatter_fields;
use crate::journal::{latest_by_field, make_record, now_ts, Journal, NewRecord};
use crate::ledger::Record;
use crate::provenance::judgment_literal;
use crate::write::{to_literal, WriteContext};

pub const INDEX_NAME: &str = ".journal-index.json";

/// Rewritten by `engine::write` itself, so a difference here is never an "external" edit.
pub const SKIP_FIELDS: [&str; 1] = ["judgment"];

pub const LOOKBACK_DAYS: i64 = 3;

/// Python spells this as `verify_tail(..., *, limit: int = 200)`.
pub const VERIFY_TAIL_LIMIT: usize = 200;

/// id -> field -> last journaled value.
pub type Index = BTreeMap<String, BTreeMap<String, Value>>;

fn index_path(vault: &Path) -> PathBuf {
    vault.join("state").join(INDEX_NAME)
}

fn str_of(rec: &Record, key: &str) -> Option<String> {
    match rec.get(key) {
        Some(Value::String(s)) => Some(s.clone()),
        _ => None,
    }
}

fn value_of(rec: &Record, key: &str) -> Value {
    rec.get(key).cloned().unwrap_or(Value::Null)
}

fn apply(index: &mut Index, records: &[Record]) {
    for ((note_id, field), rec) in latest_by_field(records) {
        index.entry(note_id).or_default().insert(field, value_of(&rec, "new"));
    }
}

/// The cached watermark, moved back [`LOOKBACK_DAYS`]. `None` (= read everything) if unparseable.
///
/// Python slices `through[:10]` by *code point*; `chars().take(10)` is the same thing, and a
/// byte slice would panic on a multi-byte lead.
fn lookback(through: &str) -> Option<String> {
    let head: String = through.chars().take(10).collect();
    Date::strptime("%Y-%m-%d", &head)
        .ok()
        .and_then(|d| d.checked_sub(jiff::Span::new().days(LOOKBACK_DAYS)).ok())
        .map(|d| d.strftime("%Y-%m-%d").to_string())
}

/// Load (and rewrite) the per-device value cache, folding in every journal record since the
/// watermark.
///
/// **The watermark is read with a three-day lookback, not from `through` itself.** Records reach
/// this device through git, so one can land carrying a `ts` EARLIER than the cached watermark — a
/// cloud run merged a day late, or clock skew between two machines. `since = through` would never
/// fold such a record into the index, and [`detect_external`] would then read its value off disk
/// and re-journal it as a `quinn`/`external` edit — inventing a human write and locking the field
/// via judge-once. Re-applying the tail is idempotent, so the only cost is a few more lines read.
pub fn load_index(vault: &Path, journal: &mut Journal) -> Index {
    let path = index_path(vault);
    let mut index: Index = BTreeMap::new();
    let mut through: Option<String> = None;

    if path.exists() {
        // A malformed cache is discarded, never raised on — it is a cache, and the journal is
        // always able to rebuild it.
        if let Ok(text) = std::fs::read_to_string(&path) {
            if let Ok(Value::Object(cached)) = serde_json::from_str::<Value>(&text) {
                let values = cached.get("values");
                let cached_through = cached.get("through");
                let values_ok = matches!(values, Some(Value::Object(map))
                    if map.values().all(|v| matches!(v, Value::Object(_))));
                let through_ok = matches!(cached_through, None | Some(Value::Null) | Some(Value::String(_)));
                if values_ok && through_ok {
                    if let Some(Value::Object(map)) = values {
                        for (note_id, fields) in map {
                            let Value::Object(fields) = fields else { continue };
                            index.insert(
                                note_id.clone(),
                                fields.iter().map(|(k, v)| (k.clone(), v.clone())).collect(),
                            );
                        }
                    }
                    through = match cached_through {
                        Some(Value::String(s)) => Some(s.clone()),
                        _ => None,
                    };
                }
            }
        }
    }

    // Python: `journal.read(since=_lookback(through)) if through else journal.read()`. A `through`
    // that will not parse yields `since=None`, which reads everything — the safe direction.
    let records = match &through {
        Some(t) if !t.is_empty() => {
            let since = lookback(t);
            journal.read(since.as_deref(), None)
        }
        _ => journal.read(None, None),
    };
    apply(&mut index, &records);
    if let Some(max) = records.iter().filter_map(|r| str_of(r, "ts")).max() {
        through = Some(max);
    }
    write_index(&path, through.as_deref(), &index);
    index
}

fn write_index(path: &Path, through: Option<&str>, index: &Index) {
    let mut values = serde_json::Map::new();
    for (note_id, fields) in index {
        let mut inner = serde_json::Map::new();
        for (field, value) in fields {
            inner.insert(field.clone(), value.clone());
        }
        values.insert(note_id.clone(), Value::Object(inner));
    }
    let mut doc = serde_json::Map::new();
    doc.insert(
        "through".into(),
        through.map(|t| Value::String(t.to_string())).unwrap_or(Value::Null),
    );
    doc.insert("values".into(), Value::Object(values));
    if let Some(parent) = path.parent() {
        let _ = std::fs::create_dir_all(parent);
    }
    // `state/.journal-index.json` is git-ignored and read only by this function, so the one
    // divergence from Python here — `sort_keys` applied where Python passed only
    // `ensure_ascii=False` — is unobservable. Going through the ledger's writer anyway keeps
    // the "all JSON leaves through one door" rule intact.
    let _ = std::fs::write(path, crate::ledger::dumps_value(&Value::Object(doc)));
}

/// Re-apply a journaled write whose note still shows `old` — the process died between the append
/// and the write.
///
/// **The note is located by id, not by the record's recorded path.** A note moved (archived, say)
/// after the crashed append still needs healing at its new home, or `detect_external` folds the
/// still-`old` disk value into the index and fabricates a `quinn`/`external` edit.
///
/// The re-apply writes **no new record**: the journal already holds the intent. `judgment` is not
/// skipped here — [`SKIP_FIELDS`] is a `detect_external` rule — because a healed judged write must
/// keep its provenance block or judge-once loses its record that Quinn never touched the field.
pub fn verify_tail(
    vault: &Path,
    journal: &mut Journal,
    _ctx: &WriteContext,
    limit: usize,
) -> Vec<String> {
    let mut log: Vec<String> = Vec::new();
    let all = journal.read(None, None);
    let sets: Vec<Record> = all
        .into_iter()
        .filter(|r| {
            str_of(r, "op").as_deref() == Some("set")
                && str_of(r, "field").is_some_and(|f| !f.is_empty())
        })
        .collect();
    // Python's `[-limit:]` keeps the tail and is a no-op when shorter.
    let tail = &sets[sets.len().saturating_sub(limit)..];
    let latest = latest_by_field(tail);
    let index = build_index(vault);

    for ((note_id, field), rec) in latest {
        let Some(path) = index.get(&note_id) else { continue };
        if !path.is_file() {
            continue;
        }
        let Some(meta) = read_meta(path) else { continue };
        let on_disk_id = crate::yaml::get(&meta, "id").and_then(crate::yaml::text);
        if !note_id.is_empty() && on_disk_id.as_deref() != Some(note_id.as_str()) {
            continue;
        }
        let current = crate::yaml::get(&meta, &field).map(crate::yaml::to_json).unwrap_or(Value::Null);
        let new = value_of(&rec, "new");
        if current == new {
            continue;
        }
        if current != value_of(&rec, "old") {
            continue;
        }
        let literal = if field == "judgment" && new.is_object() {
            match judgment_literal(&crate::yaml::from_json(&new)) {
                Ok(l) => l,
                // Python lets the ValueError escape; a judgment that cannot be written as a
                // single-line flow mapping is a real fault, not something to paper over. Here it
                // would abort the whole pass, so the note is left un-healed and the next run
                // tries again — the same end state as Python's crash, minus the lost run.
                Err(_) => continue,
            }
        } else {
            to_literal(&crate::yaml::from_json(&new))
        };
        if update_frontmatter_fields(path, &[(field.clone(), literal)]).is_err() {
            continue;
        }
        log.push(format!(
            "re-applied {field} on {} from journal {}",
            rel(vault, path),
            str_of(&rec, "ts").unwrap_or_default()
        ));
    }
    log
}

fn mtime_ts(path: &Path) -> String {
    let stamp = std::fs::metadata(path)
        .and_then(|m| m.modified())
        .ok()
        .and_then(|t| Timestamp::try_from(t).ok());
    now_ts(stamp)
}

/// Journal the edits Quinn made outside the engine — Obsidian, or a text editor.
///
/// Every difference between a note's frontmatter and the last value the journal knows becomes a
/// `quinn`/`external` record stamped with the **file's mtime**, not the wall clock, so the
/// reconciliation rule (later timestamp wins) orders it against agent writes correctly.
pub fn detect_external(vault: &Path, journal: &mut Journal, ctx: &WriteContext) -> Vec<String> {
    let mut log: Vec<String> = Vec::new();
    let records = journal.read(None, None);
    let migrated = records.iter().any(|r| {
        str_of(r, "op").as_deref() == Some("create")
            && str_of(r, "actor").is_some_and(|a| a.starts_with("system:migration"))
    });
    let notes = scan_notes(vault);
    if !notes.is_empty() && !migrated {
        log.push(
            "journal has no migration records — run scripts/migrate_s1.py before relying on \
             detect_external"
                .to_string(),
        );
        return log;
    }
    // Ruling 11: a hand edit is the student's, under the vault's own token, read once per call.
    // A bad `config/actor.yaml` stops the pass by name BEFORE `load_index` moves the index, so no
    // record is written and the edit is still there to pick up once the file is fixed — the same
    // "no human write proceeds" rule `write`'s gate applies, since these records bypass `write`.
    let human = match crate::journal::read_human_actor(vault) {
        Ok(token) => token,
        Err(e) => {
            log.push(format!("detect_external skipped: {e}"));
            return log;
        }
    };

    let mut index = load_index(vault, journal);
    for (path, meta) in notes {
        let Some(meta) = meta else { continue };
        // ensure_ids runs first; an id-less note is its job, not ours.
        let Some(note_id) = crate::yaml::get(&meta, "id").and_then(crate::yaml::text) else {
            continue;
        };
        if !is_id(&note_id) {
            continue;
        }
        let rel_path = rel(vault, &path);
        let ts = mtime_ts(&path);

        // `contains_key` rather than `get_mut` in a let-else: the borrow checker holds a
        // `get_mut` borrow across the else block, which needs `index` mutably too.
        if !index.contains_key(&note_id) {
            let whole = crate::yaml::to_json(&serde_yaml_ng::Value::Mapping(meta.clone()));
            let mut spec = NewRecord::new("create", &rel_path, human, "external");
            spec.id = Some(&note_id);
            spec.new = whole.clone();
            spec.ts = Some(ts);
            spec.run_id = ctx.run_id.as_deref();
            if let Ok(mut rec) = make_record(spec) {
                let _ = journal.append(&mut rec);
            }
            let mut fields = BTreeMap::new();
            if let Value::Object(map) = whole {
                for (k, v) in map {
                    fields.insert(k, v);
                }
            }
            index.insert(note_id.clone(), fields);
            log.push(format!("external create {rel_path}"));
            continue;
        }
        let known = index.get_mut(&note_id).expect("just checked");

        // `set(meta) | set(known)` — a field DELETED by hand is as much an edit as a changed one,
        // and reads as `value -> null`.
        let mut fields: std::collections::BTreeSet<String> = known.keys().cloned().collect();
        for key in meta.keys() {
            if let Some(name) = crate::yaml::text(key) {
                fields.insert(name);
            }
        }
        for skip in SKIP_FIELDS {
            fields.remove(skip);
        }

        for field in fields {
            let current =
                crate::yaml::get(&meta, &field).map(crate::yaml::to_json).unwrap_or(Value::Null);
            let prior = known.get(&field).cloned().unwrap_or(Value::Null);
            if current == prior {
                continue;
            }
            let mut spec = NewRecord::new("set", &rel_path, human, "external");
            spec.id = Some(&note_id);
            spec.field = Some(&field);
            spec.old = prior;
            spec.new = current.clone();
            spec.ts = Some(ts.clone());
            spec.run_id = ctx.run_id.as_deref();
            if let Ok(mut rec) = make_record(spec) {
                let _ = journal.append(&mut rec);
            }
            known.insert(field.clone(), current);
            log.push(format!("external edit {field} on {rel_path}"));
        }
    }

    journal.invalidate(); // force the final read to see every record just appended
    let through = journal.read(None, None).iter().filter_map(|r| str_of(r, "ts")).max();
    write_index(&index_path(vault), through.as_deref(), &index);
    log
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::models::split_frontmatter;
    use crate::write::{create, write, WriteOpts};
    use serde_yaml_ng::Value as Yaml;

    fn ctx() -> WriteContext {
        WriteContext {
            actor: "agent:approvals".into(),
            via: "local-runner".into(),
            run_id: Some("local-2026-08-29T17:00:00Z".into()),
        }
    }

    /// `detect_external`'s guard keys on evidence of MIGRATION, not on "some create exists" — an
    /// agent create (ingest minting one Blackboard note) must never unlock it. Fixture vaults
    /// therefore seed their notes the way `scripts/migrate_s1.py` does.
    fn migration_ctx() -> WriteContext {
        WriteContext { actor: "system:migration".into(), via: "cli".into(), run_id: None }
    }

    fn scratch(name: &str) -> PathBuf {
        let dir = std::env::temp_dir().join(format!(
            "qo-passes-{name}-{}-{:?}",
            std::process::id(),
            std::thread::current().id()
        ));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(dir.join("tasks")).unwrap();
        dir
    }

    fn meta_of(path: &Path) -> serde_yaml_ng::Mapping {
        split_frontmatter(&pystr_read(path)).unwrap().0
    }

    fn pystr_read(path: &Path) -> String {
        crate::pystr::read_text(path).unwrap()
    }

    fn field(path: &Path, name: &str) -> Option<Yaml> {
        crate::yaml::get(&meta_of(path), name).cloned()
    }

    fn make_vault(name: &str) -> (PathBuf, Journal) {
        let vault = scratch(name);
        let mut journal = Journal::new(&vault);
        create(
            &vault,
            "tasks/a.md",
            "---\ntitle: A\nprogress: 0\nimportance: 3\n---\n",
            &migration_ctx(),
            &mut journal,
            None,
        )
        .unwrap();
        (vault, journal)
    }

    fn note_id(vault: &Path, rel_path: &str) -> String {
        crate::yaml::get(&meta_of(&vault.join(rel_path)), "id")
            .and_then(crate::yaml::text)
            .expect("create stamps an id")
    }

    fn set_record(note_id: &str, path: &str, actor: &str, via: &str, f: &str, old: Value, new: Value)
        -> Record
    {
        let mut spec = NewRecord::new("set", path, actor, via);
        spec.id = Some(note_id);
        spec.field = Some(f);
        spec.old = old;
        spec.new = new;
        make_record(spec).unwrap()
    }

    // -----------------------------------------------------------------------
    // load_index
    // -----------------------------------------------------------------------

    #[test]
    fn the_index_is_built_from_the_journal_and_kept_up_to_date() {
        let (vault, mut journal) = make_vault("index");
        write(&vault, "tasks/a.md", &[("progress".into(), Yaml::from(40))], &ctx(), &mut journal,
              &WriteOpts::default()).unwrap();
        let id = note_id(&vault, "tasks/a.md");

        let index = load_index(&vault, &mut Journal::new(&vault));
        assert_eq!(index[&id]["progress"], serde_json::json!(40));
        assert_eq!(index[&id]["title"], serde_json::json!("A"));
        assert!(vault.join("state").join(INDEX_NAME).exists());

        write(&vault, "tasks/a.md", &[("progress".into(), Yaml::from(70))], &ctx(), &mut journal,
              &WriteOpts::default()).unwrap();
        let index = load_index(&vault, &mut Journal::new(&vault));
        assert_eq!(index[&id]["progress"], serde_json::json!(70), "cache updated incrementally");
        let _ = std::fs::remove_dir_all(&vault);
    }

    #[test]
    fn a_schema_corrupt_cache_is_rebuilt_rather_than_trusted() {
        let (vault, _) = make_vault("corrupt");
        std::fs::create_dir_all(vault.join("state")).unwrap();
        std::fs::write(vault.join("state").join(INDEX_NAME), r#"{"values": null, "through": 5}"#)
            .unwrap();
        let index = load_index(&vault, &mut Journal::new(&vault));
        let id = note_id(&vault, "tasks/a.md");
        assert_eq!(index[&id]["title"], serde_json::json!("A"));
        let _ = std::fs::remove_dir_all(&vault);
    }

    // -----------------------------------------------------------------------
    // verify_tail
    // -----------------------------------------------------------------------

    #[test]
    fn verify_tail_reapplies_a_half_applied_write_without_adding_a_record() {
        let (vault, mut journal) = make_vault("halfapplied");
        let id = note_id(&vault, "tasks/a.md");
        // The journal record landed; the note write never happened.
        let mut rec = set_record(&id, "tasks/a.md", "quinn", "dashboard", "progress",
                                 serde_json::json!(0), serde_json::json!(100));
        journal.append(&mut rec).unwrap();

        let log = verify_tail(&vault, &mut Journal::new(&vault), &ctx(), VERIFY_TAIL_LIMIT);
        assert_eq!(field(&vault.join("tasks/a.md"), "progress"), Some(Yaml::from(100)));
        assert!(log.iter().any(|l| l.contains("re-applied")), "{log:?}");

        // Idempotent, and it did NOT add a second record — the record already existed.
        assert!(verify_tail(&vault, &mut Journal::new(&vault), &ctx(), VERIFY_TAIL_LIMIT).is_empty());
        assert_eq!(Journal::new(&vault).records_for(&id, Some("progress")).len(), 1);
        let _ = std::fs::remove_dir_all(&vault);
    }

    #[test]
    fn verify_tail_leaves_a_field_that_moved_on_since() {
        let (vault, mut journal) = make_vault("movedon");
        let id = note_id(&vault, "tasks/a.md");
        let mut spec = NewRecord::new("set", "tasks/a.md", "quinn", "dashboard");
        spec.id = Some(&id);
        spec.field = Some("progress");
        spec.old = serde_json::json!(0);
        spec.new = serde_json::json!(100);
        spec.ts = Some("2026-08-29T10:00:00.000Z".into());
        let mut rec = make_record(spec).unwrap();
        journal.append(&mut rec).unwrap();

        write(&vault, "tasks/a.md", &[("progress".into(), Yaml::from(30))], &ctx(), &mut journal,
              &WriteOpts::default()).unwrap();
        verify_tail(&vault, &mut Journal::new(&vault), &ctx(), VERIFY_TAIL_LIMIT);
        assert_eq!(field(&vault.join("tasks/a.md"), "progress"), Some(Yaml::from(30)));
        let _ = std::fs::remove_dir_all(&vault);
    }

    /// The note is located by **id**, not by the record's recorded path: a note archived after
    /// the crashed append still needs healing at its new home, or `detect_external` folds the
    /// still-`old` disk value into the index and fabricates a `quinn`/`external` edit.
    #[test]
    fn verify_tail_heals_a_moved_note_by_id_and_detect_external_then_stays_quiet() {
        let (vault, mut journal) = make_vault("moved");
        let id = note_id(&vault, "tasks/a.md");
        let mut rec = set_record(&id, "tasks/a.md", "agent:approvals", "local-runner", "importance",
                                 serde_json::json!(3), serde_json::json!(5));
        journal.append(&mut rec).unwrap();

        std::fs::create_dir_all(vault.join("archive")).unwrap();
        std::fs::rename(vault.join("tasks/a.md"), vault.join("archive/a.md")).unwrap();

        let log = verify_tail(&vault, &mut Journal::new(&vault), &ctx(), VERIFY_TAIL_LIMIT);
        assert_eq!(field(&vault.join("archive/a.md"), "importance"), Some(Yaml::from(5)));
        assert!(log.iter().any(|l| l.contains("re-applied")), "{log:?}");

        detect_external(&vault, &mut Journal::new(&vault), &ctx());
        let invented: Vec<Record> = Journal::new(&vault)
            .read(None, None)
            .into_iter()
            .filter(|r| {
                r.get("via") == Some(&serde_json::json!("external"))
                    && r.get("field") == Some(&serde_json::json!("importance"))
            })
            .collect();
        assert!(invented.is_empty(), "{invented:?}");
        let _ = std::fs::remove_dir_all(&vault);
    }

    /// The healed value must stay a YAML **date**, not become a quoted string — every consumer
    /// that compares dates would stop matching.
    #[test]
    fn verify_tail_preserves_the_date_type() {
        let vault = scratch("datetype");
        let mut journal = Journal::new(&vault);
        create(&vault, "tasks/d.md", "---\ntitle: D\ndue: 2026-09-01\n---\n", &migration_ctx(),
               &mut journal, None).unwrap();
        let id = note_id(&vault, "tasks/d.md");
        let mut rec = set_record(&id, "tasks/d.md", "quinn", "dashboard", "due",
                                 serde_json::json!("2026-09-01"), serde_json::json!("2026-09-05"));
        journal.append(&mut rec).unwrap();

        verify_tail(&vault, &mut Journal::new(&vault), &ctx(), VERIFY_TAIL_LIMIT);
        let text = pystr_read(&vault.join("tasks/d.md"));
        assert!(text.contains("due: 2026-09-05"), "{text}");
        assert!(!text.contains("due: \"2026-09-05\""), "{text}");
        let _ = std::fs::remove_dir_all(&vault);
    }

    /// `judgment` is NOT skipped here — [`SKIP_FIELDS`] is a `detect_external` rule. A healed
    /// judged write must keep its provenance block, or judge-once loses its record that Quinn
    /// never touched the field.
    #[test]
    fn verify_tail_heals_the_judgment_block_too() {
        let (vault, mut journal) = make_vault("judgment");
        let id = note_id(&vault, "tasks/a.md");
        let block = serde_json::json!({
            "run_id": "r1",
            "actor": "agent:routine.enrich",
            "at": "2026-08-29T13:00:00.000Z",
            "inputs": {},
            "fields": ["importance"],
        });
        let mut r1 = set_record(&id, "tasks/a.md", "agent:routine.enrich", "cloud-routine",
                                "importance", serde_json::json!(3), serde_json::json!(4));
        journal.append(&mut r1).unwrap();
        let mut r2 = set_record(&id, "tasks/a.md", "agent:routine.enrich", "cloud-routine",
                                "judgment", Value::Null, block.clone());
        journal.append(&mut r2).unwrap();

        verify_tail(&vault, &mut Journal::new(&vault), &ctx(), VERIFY_TAIL_LIMIT);
        let meta = meta_of(&vault.join("tasks/a.md"));
        assert_eq!(crate::yaml::get(&meta, "importance"), Some(&Yaml::from(4)));
        let healed = crate::yaml::get(&meta, "judgment").map(crate::yaml::to_json);
        assert_eq!(healed, Some(block));
        let _ = std::fs::remove_dir_all(&vault);
    }

    // -----------------------------------------------------------------------
    // detect_external
    // -----------------------------------------------------------------------

    #[test]
    fn an_obsidian_style_edit_is_journalled_as_quinn_external() {
        let (vault, _) = make_vault("obsidian");
        load_index(&vault, &mut Journal::new(&vault));
        let path = vault.join("tasks/a.md");
        let text = pystr_read(&path).replace("importance: 3", "importance: 5");
        crate::pystr::write_text(&path, &text).unwrap();

        let log = detect_external(&vault, &mut Journal::new(&vault), &ctx());
        let recs: Vec<Record> = Journal::new(&vault)
            .read(None, None)
            .into_iter()
            .filter(|r| r.get("via") == Some(&serde_json::json!("external")))
            .collect();
        assert_eq!(recs.len(), 1, "{recs:?}");
        assert_eq!(recs[0].get("field"), Some(&serde_json::json!("importance")));
        assert_eq!(recs[0].get("old"), Some(&serde_json::json!(3)));
        assert_eq!(recs[0].get("new"), Some(&serde_json::json!(5)));
        assert_eq!(recs[0].get("actor"), Some(&serde_json::json!("quinn")));
        assert!(log.iter().any(|l| l.contains("external")), "{log:?}");

        // Nothing on a clean vault.
        assert!(detect_external(&vault, &mut Journal::new(&vault), &ctx()).is_empty());
        let _ = std::fs::remove_dir_all(&vault);
    }

    fn actor_file(vault: &Path, text: &str) {
        std::fs::create_dir_all(vault.join("config")).unwrap();
        std::fs::write(vault.join("config").join("actor.yaml"), text).unwrap();
    }

    /// Ruling 11: on a vault the app made, a hand edit is the student's, under the vault's token —
    /// both the `set` for an edited field and the `create` for a note made outside the engine.
    #[test]
    fn an_external_edit_on_a_student_vault_is_journalled_as_student() {
        let (vault, _) = make_vault("student-edit");
        actor_file(&vault, "human_actor: student\n");
        load_index(&vault, &mut Journal::new(&vault));
        let path = vault.join("tasks/a.md");
        crate::pystr::write_text(&path, &pystr_read(&path).replace("importance: 3", "importance: 5")).unwrap();
        crate::pystr::write_text(&vault.join("tasks/hand.md"), "---\ntitle: Hand\nid: task_abcdef0123\n---\n").unwrap();

        detect_external(&vault, &mut Journal::new(&vault), &ctx());
        let recs: Vec<Record> = Journal::new(&vault)
            .read(None, None)
            .into_iter()
            .filter(|r| r.get("via") == Some(&serde_json::json!("external")))
            .collect();
        assert_eq!(recs.len(), 2, "{recs:?}");
        assert!(recs.iter().all(|r| r.get("actor") == Some(&serde_json::json!("student"))), "{recs:?}");
        let _ = std::fs::remove_dir_all(&vault);
    }

    /// A bad `config/actor.yaml` stops the pass by name: one log line, no record, and the index is
    /// left exactly where it was, so the edit is picked up once the file is fixed.
    #[test]
    fn a_bad_actor_file_skips_detect_external_by_name() {
        let (vault, _) = make_vault("bad-actor");
        load_index(&vault, &mut Journal::new(&vault));
        let path = vault.join("tasks/a.md");
        crate::pystr::write_text(&path, &pystr_read(&path).replace("importance: 3", "importance: 5")).unwrap();
        actor_file(&vault, "human_actor: alice\n");
        let journal_before = Journal::new(&vault).read(None, None);
        let index_before = std::fs::read(vault.join("state").join(INDEX_NAME)).unwrap();

        let log = detect_external(&vault, &mut Journal::new(&vault), &ctx());
        assert_eq!(log.len(), 1, "{log:?}");
        assert!(log[0].starts_with("detect_external skipped: config/actor.yaml: ") && log[0].contains("\"alice\""), "{log:?}");
        assert_eq!(Journal::new(&vault).read(None, None), journal_before, "no record");
        assert_eq!(std::fs::read(vault.join("state").join(INDEX_NAME)).unwrap(), index_before, "the index did not advance");

        actor_file(&vault, "human_actor: quinn\n");
        let log = detect_external(&vault, &mut Journal::new(&vault), &ctx());
        assert_eq!(log, vec!["external edit importance on tasks/a.md".to_string()], "picked up once the file is fixed");
        let _ = std::fs::remove_dir_all(&vault);
    }

    #[test]
    fn a_note_created_outside_the_engine_is_journalled_as_an_external_create() {
        let (vault, _) = make_vault("handmade");
        load_index(&vault, &mut Journal::new(&vault));
        crate::pystr::write_text(
            &vault.join("tasks/hand.md"),
            "---\ntitle: Hand\nid: task_abcdef0123\nprogress: 0\n---\n",
        )
        .unwrap();

        detect_external(&vault, &mut Journal::new(&vault), &ctx());
        let recs: Vec<Record> = Journal::new(&vault)
            .read(None, None)
            .into_iter()
            .filter(|r| r.get("id") == Some(&serde_json::json!("task_abcdef0123")))
            .collect();
        assert!(!recs.is_empty());
        assert_eq!(recs[0].get("op"), Some(&serde_json::json!("create")));
        assert_eq!(recs[0].get("via"), Some(&serde_json::json!("external")));
        let _ = std::fs::remove_dir_all(&vault);
    }

    /// A record can reach this device with a `ts` EARLIER than the cached watermark — a cloud run
    /// merged late, or a lagging clock. Read strictly from the watermark and it is never folded
    /// into the index, so `detect_external` reads the value off disk, finds nothing journaled, and
    /// fabricates a `quinn`/`external` edit for a write an agent actually made — which then locks
    /// the field via judge-once.
    #[test]
    fn a_late_arriving_record_is_folded_in_rather_than_read_as_a_human_edit() {
        let (vault, mut journal) = make_vault("straggler");
        let path = vault.join("tasks/a.md");
        let id = note_id(&vault, "tasks/a.md");
        write(&vault, "tasks/a.md", &[("progress".into(), Yaml::from(40))], &ctx(), &mut journal,
              &WriteOpts::default()).unwrap();
        load_index(&vault, &mut Journal::new(&vault)); // cache `through` = that write's ts

        let cached: Value =
            serde_json::from_str(&std::fs::read_to_string(vault.join("state").join(INDEX_NAME)).unwrap())
                .unwrap();
        let through = cached["through"].as_str().unwrap().to_string();
        let late = through.replace('Z', "+00:00").parse::<Timestamp>().unwrap()
            - jiff::SignedDuration::from_hours(2);

        let mut spec = NewRecord::new("set", "tasks/a.md", "agent:routine.enrich", "cloud-routine");
        spec.id = Some(&id);
        spec.field = Some("due");
        spec.old = Value::Null;
        spec.new = serde_json::json!("2026-09-05");
        spec.ts = Some(now_ts(Some(late)));
        spec.device = Some("cloud".into());
        let mut rec = make_record(spec).unwrap();
        Journal::new(&vault).append(&mut rec).unwrap();

        let text = pystr_read(&path).replace("title: A", "title: A\ndue: 2026-09-05");
        crate::pystr::write_text(&path, &text).unwrap();

        let log = detect_external(&vault, &mut Journal::new(&vault), &ctx());
        assert!(log.is_empty(), "{log:?}");
        let invented: Vec<Record> = Journal::new(&vault)
            .read(None, None)
            .into_iter()
            .filter(|r| {
                r.get("via") == Some(&serde_json::json!("external"))
                    && r.get("field") == Some(&serde_json::json!("due"))
            })
            .collect();
        assert!(invented.is_empty(), "{invented:?}");
        let _ = std::fs::remove_dir_all(&vault);
    }

    const UNMIGRATED: &str = "journal has no migration records — run scripts/migrate_s1.py before \
                              relying on detect_external";

    /// An idfix `set` record is not evidence of migration.
    #[test]
    fn detect_external_refuses_to_run_on_an_unmigrated_vault() {
        let vault = scratch("unmigrated");
        crate::pystr::write_text(
            &vault.join("tasks/a.md"),
            "---\ntitle: A\nid: task_abcdef0123\nprogress: 0\nimportance: 3\n---\n",
        )
        .unwrap();
        let mut rec = set_record("task_abcdef0123", "tasks/a.md", "system:idfix", "local-runner",
                                 "id", Value::Null, serde_json::json!("task_abcdef0123"));
        Journal::new(&vault).append(&mut rec).unwrap();

        let log = detect_external(&vault, &mut Journal::new(&vault), &ctx());
        assert_eq!(log, vec![UNMIGRATED.to_string()]);
        assert!(Journal::new(&vault)
            .read(None, None)
            .iter()
            .all(|r| r.get("via") != Some(&serde_json::json!("external"))));
        let _ = std::fs::remove_dir_all(&vault);
    }

    /// The guard keys on MIGRATION, not on any `create` at all.
    ///
    /// Blackboard/zyBooks/VHL ingest all run before the passes and mint notes through
    /// `write::create`. One such note landing on a vault that was never migrated used to satisfy
    /// an any-create guard — and then `detect_external` journalled every pre-existing note as a
    /// `quinn`/`external` create, locking every judged field on the whole vault via judge-once.
    /// One arriving assignment, whole vault frozen.
    #[test]
    fn an_agent_create_in_the_same_run_does_not_unlock_the_guard() {
        let vault = scratch("agentcreate");
        for (name, id) in [("a.md", "task_abcdef0123"), ("b.md", "task_abcdef0124")] {
            crate::pystr::write_text(
                &vault.join("tasks").join(name),
                &format!("---\ntitle: {name}\nid: {id}\nprogress: 0\nimportance: 3\n---\n"),
            )
            .unwrap();
        }
        let ingest = WriteContext {
            actor: "agent:ingest.blackboard".into(),
            via: "local-runner".into(),
            run_id: Some("local-2026-08-29T17:00:00Z".into()),
        };
        create(&vault, "tasks/fresh.md", "---\ntitle: Fresh from Blackboard\nprogress: 0\n---\n",
               &ingest, &mut Journal::new(&vault), None).unwrap();

        let log = detect_external(&vault, &mut Journal::new(&vault), &ctx());
        assert_eq!(log, vec![UNMIGRATED.to_string()]);
        assert!(Journal::new(&vault)
            .read(None, None)
            .iter()
            .all(|r| r.get("via") != Some(&serde_json::json!("external"))));
        let _ = std::fs::remove_dir_all(&vault);
    }
}
