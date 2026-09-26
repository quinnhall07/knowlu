//! Port of `engine/write.py` — the single write path.
//!
//! Every change to a note's frontmatter or body goes through here so the journal is complete by
//! construction.
//!
//! > **Order is journal FIRST, note SECOND.** If the append fails, nothing changes. If the process
//! > dies in between, `passes::verify_tail` re-applies. Reversing the order turns a crash into a
//! > silent, unattributable edit — which is the failure the journal exists to make impossible.
//!
//! That ordering is not a convention here; it is pinned by a test that fails the file write and
//! asserts the record survived.

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};
use std::sync::LazyLock;

use regex::Regex;
use serde_yaml_ng::{Mapping, Value};

use crate::ids::{is_id, kind_for, new_id, read_meta, rel, resolve_target, IdError};
use crate::info::{python_path, CliError};
use crate::ingest::{apply_frontmatter_fields_to_text, update_frontmatter_fields, IngestError};
use crate::journal::{make_record, now_ts, Journal, NewRecord};
use crate::ledger::Record;
use crate::models::split_frontmatter;
use crate::provenance::{
    guard_block_style, is_agent, judged_fields_for, judgment_literal, judgment_value,
    ProvenanceError,
};
use crate::pystr;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum WriteError {
    NoFrontmatter(String),
    AppendLine(&'static str),
    Exists(String),
    /// Two-desktop design D3: the import id this note would take is already held in the vault — the
    /// item's note is here already. Refused before anything is journalled or written.
    IdHeld(String),
    Id(IdError),
    Ingest(IngestError),
    Provenance(ProvenanceError),
    Io(String),
}

impl std::fmt::Display for WriteError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            WriteError::NoFrontmatter(p) => write!(f, "{p} has no readable frontmatter"),
            WriteError::AppendLine(why) => write!(f, "append line {why}"),
            WriteError::Exists(p) => write!(f, "{p} exists"),
            WriteError::IdHeld(id) => write!(f, "already held as {id}"),
            WriteError::Id(e) => write!(f, "{e}"),
            WriteError::Ingest(e) => write!(f, "{e}"),
            WriteError::Provenance(e) => write!(f, "{e}"),
            WriteError::Io(m) => write!(f, "{m}"),
        }
    }
}

impl From<IdError> for WriteError {
    fn from(e: IdError) -> Self {
        WriteError::Id(e)
    }
}
impl From<IngestError> for WriteError {
    fn from(e: IngestError) -> Self {
        WriteError::Ingest(e)
    }
}
impl From<ProvenanceError> for WriteError {
    fn from(e: ProvenanceError) -> Self {
        WriteError::Provenance(e)
    }
}

// ---------------------------------------------------------------------------
// Body append (moved verbatim from approvals.py in S1)
// ---------------------------------------------------------------------------

/// Why `value` is unsafe to write into a note as one line, or `None`.
///
/// **Deliberately `splitlines()` rather than a `\n`/`\r` test.** `splitlines` is what
/// `append_body_line`'s idempotence check uses, and it also breaks on `\x0b`, `\x0c`, `\x1c`–`\x1e`
/// and U+0085. A line carrying one of those never matched itself on re-read, so it was appended
/// again on every pass — the note grew a blockquote twice a day, forever. Tying the rule to the
/// function that consumes it is what stops the two disagreeing again.
pub fn single_line_problem(value: &str) -> Option<&'static str> {
    if value != pystr::splitlines(value).concat() {
        return Some("must be a single line");
    }
    if value.trim_start().starts_with("---") {
        return Some("must not start a frontmatter block");
    }
    None
}

/// Append one blockquoted line to the end of a note's body.
///
/// Append is the **only** operation: existing prose is never read for modification, reordered or
/// deleted. Idempotent, so re-executing after a partial failure cannot double-write.
pub fn append_body_line(path: &Path, line: &str) -> Result<bool, WriteError> {
    if let Some(problem) = single_line_problem(line) {
        return Err(WriteError::AppendLine(problem));
    }
    let mut text = pystr::read_text(path).map_err(|e| WriteError::Io(e.to_string()))?;
    let stamped = format!("> {line}");

    // Body is extracted TEXTUALLY, without YAML parsing, so a malformed note can still be
    // appended to rather than crashing the run.
    let body = if text.starts_with("---\n") {
        match text[4..].find("\n---\n") {
            Some(offset) => text[4 + offset + 5..].to_string(),
            None => text.clone(),
        }
    } else {
        text.clone()
    };

    if pystr::splitlines(&body).iter().any(|l| *l == stamped) {
        return Ok(false);
    }
    if !text.ends_with('\n') {
        text.push('\n');
    }
    let out = format!("{text}\n{stamped}\n");
    pystr::write_text(path, &out).map_err(|e| WriteError::Io(e.to_string()))?;
    Ok(true)
}

// ---------------------------------------------------------------------------
// Context and result
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WriteContext {
    pub actor: String,
    pub via: String,
    pub run_id: Option<String>,
}

impl WriteContext {
    pub fn new(actor: &str, via: &str) -> WriteContext {
        WriteContext { actor: actor.to_string(), via: via.to_string(), run_id: None }
    }
    pub fn with_actor(&self, actor: &str) -> WriteContext {
        WriteContext { actor: actor.to_string(), via: self.via.clone(), run_id: self.run_id.clone() }
    }
}

#[derive(Debug, Default)]
pub struct WriteResult {
    pub path: PathBuf,
    /// **Ordered**, because it feeds `apply_frontmatter_fields_to_text`, whose insertion point
    /// shifts as keys are appended. A map would make the output order-dependent on hashing.
    pub written: Vec<(String, String)>,
    pub skipped: BTreeMap<String, String>,
    pub records: Vec<Record>,
    pub proposal: Option<PathBuf>,
}

static ISO_DATE_LIKE: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"^\d{4}-\d{2}-\d{2}(T\d{2}:\d{2}(:\d{2})?)?$").unwrap());

/// A value -> the frontmatter text existing notes already use.
///
/// The ISO branch matters: a bare date string (re-applied by `verify_tail` from a journal record)
/// must round-trip as the vault's own YAML date convention. **Quoting it silently retypes `due`
/// from a date to a str**, and every consumer that compares dates then stops matching.
pub fn to_literal(value: &Value) -> String {
    match value {
        Value::Null => "null".to_string(),
        Value::Bool(b) => if *b { "true" } else { "false" }.to_string(),
        Value::Number(n) => n.to_string(),
        Value::String(s) => {
            if ISO_DATE_LIKE.is_match(s) {
                s.clone()
            } else {
                // ensure_ascii=False: an em dash is written literally, matching coursework/ingest.
                // serde_json escapes control characters and quotes but leaves other unicode alone,
                // which is the same output.
                serde_json::to_string(s).unwrap_or_else(|_| format!("{s:?}"))
            }
        }
        other => pystr::strip(&crate::yamlemit::safe_dump_flow(other)).to_string(),
    }
}

pub fn parse_literal(literal: &str) -> Value {
    serde_yaml_ng::from_str::<Value>(literal).unwrap_or(Value::Null)
}

fn load(path: &Path, guard: bool) -> Result<(String, Mapping), WriteError> {
    let text = pystr::read_text(path).map_err(|e| WriteError::Io(e.to_string()))?;
    let (meta, _) = split_frontmatter(&text)
        .map_err(|_| WriteError::NoFrontmatter(path.display().to_string()))?;
    if guard {
        guard_block_style(&text)?;
    }
    Ok((text, meta))
}

fn get_str(meta: &Mapping, key: &str) -> Option<String> {
    crate::yaml::get(meta, key).and_then(crate::yaml::text)
}

fn yaml_to_json(value: &Value) -> serde_json::Value {
    crate::yaml::to_json(value)
}

// ---------------------------------------------------------------------------
// The write path
// ---------------------------------------------------------------------------

pub struct WriteOpts<'a> {
    pub judged: bool,
    pub evidence: Option<&'a serde_json::Value>,
    pub propose: bool,
    pub inputs: Option<&'a Mapping>,
}

impl Default for WriteOpts<'_> {
    fn default() -> Self {
        WriteOpts { judged: false, evidence: None, propose: false, inputs: None }
    }
}

/// Apply named frontmatter changes, journalling first.
pub fn write_literals(
    vault: &Path,
    target: &str,
    literals: &[(String, String)],
    ctx: &WriteContext,
    journal: &mut Journal,
    opts: &WriteOpts<'_>,
) -> Result<WriteResult, WriteError> {
    let path = resolve_target(vault, target)?;
    let rel_path = rel(vault, &path);
    let (_, meta) = load(&path, true)?;
    let note_id = get_str(&meta, "id").filter(|s| is_id(s));
    let kind = kind_for(&path, Some(&meta));
    let mut result = WriteResult { path: path.clone(), ..Default::default() };
    // `(name, old, new)` — `propose_amendment` writes both sides into the card's `changes` block,
    // and `approvals::apply_amendment` compares `from` against the note before it applies.
    let mut proposed: Vec<(String, Value, Value)> = Vec::new();

    for (name, literal) in literals {
        let new = parse_literal(literal);
        let old = crate::yaml::get(&meta, name).cloned().unwrap_or(Value::Null);

        // F4: a no-op field produces no record and no write.
        if yaml_to_json(&old) == yaml_to_json(&new) {
            continue;
        }

        if opts.judged && is_agent(&ctx.actor) && judged_fields_for(&kind).contains(name.as_str()) {
            let human = note_id.as_ref().and_then(|id| journal.human_set(id, name));
            if let Some(human) = human {
                if opts.propose && old != Value::Null {
                    proposed.push((name.clone(), old.clone(), new.clone()));
                } else if opts.propose {
                    // validate_amendment refuses a null `from` (the 08-21 hardening rule); a
                    // proposal it can never apply is worse than no proposal at all.
                    result
                        .skipped
                        .insert(name.clone(), "judge-once: cannot propose from null".to_string());
                } else {
                    let ts = human
                        .get("ts")
                        .and_then(|v| v.as_str())
                        .unwrap_or("")
                        .to_string();
                    result.skipped.insert(
                        name.clone(),
                        format!("judge-once: {name} set by quinn at {ts}"),
                    );
                }
                continue;
            }
        }

        result.written.push((name.clone(), literal.clone()));
        let mut spec = NewRecord::new("set", &rel_path, &ctx.actor, &ctx.via);
        spec.id = note_id.as_deref();
        spec.field = Some(name);
        spec.old = yaml_to_json(&old);
        spec.new = yaml_to_json(&new);
        spec.run_id = ctx.run_id.as_deref();
        spec.evidence = opts.evidence.cloned().unwrap_or(serde_json::Value::Null);
        result.records.push(make_record(spec).map_err(|e| WriteError::Io(e.to_string()))?);
    }

    if opts.judged && is_agent(&ctx.actor) {
        let allowed = judged_fields_for(&kind);
        let mut judged_now: Vec<String> = result
            .written
            .iter()
            .map(|(n, _)| n.clone())
            .filter(|n| allowed.contains(n.as_str()))
            .collect();
        judged_now.sort();
        if !judged_now.is_empty() {
            let empty = Mapping::new();
            let block = judgment_value(
                ctx.run_id.as_deref(),
                &ctx.actor,
                &now_ts(None),
                Some(opts.inputs.unwrap_or(&empty)),
                &judged_now,
            );
            let block_value = Value::Mapping(block.clone());
            result
                .written
                .push(("judgment".to_string(), judgment_literal(&block_value)?));

            let mut spec = NewRecord::new("set", &rel_path, &ctx.actor, &ctx.via);
            spec.id = note_id.as_deref();
            spec.field = Some("judgment");
            spec.old = yaml_to_json(&crate::yaml::get(&meta, "judgment").cloned().unwrap_or(Value::Null));
            spec.new = yaml_to_json(&block_value);
            spec.run_id = ctx.run_id.as_deref();
            result.records.push(make_record(spec).map_err(|e| WriteError::Io(e.to_string()))?);
        }
    }

    // ---- journal first ----
    for record in result.records.iter_mut() {
        journal.append(record).map_err(|e| WriteError::Io(e.to_string()))?;
    }
    // ---- note second ----
    if !result.written.is_empty() {
        update_frontmatter_fields(&path, &result.written)?;
    }

    if !proposed.is_empty() {
        // An identical pending proposal already IS the re-proposal; point at it and report the
        // skip, rather than minting a fresh `amend-*.md` on every run and burying the queue.
        let fields: std::collections::BTreeSet<String> =
            proposed.iter().map(|(n, _, _)| n.clone()).collect();
        match find_pending_amendment(vault, &rel_path, &fields) {
            Some(existing) => {
                let name = existing
                    .file_name()
                    .map(|n| n.to_string_lossy().to_string())
                    .unwrap_or_default();
                for (field, _, _) in &proposed {
                    result.skipped.insert(
                        field.clone(),
                        format!("judge-once: proposal already pending ({name})"),
                    );
                }
                result.proposal = Some(existing);
            }
            None => {
                // `today` is the system date, as Python's `today or date.today()` resolves it for
                // every production caller. Nothing in the engine passes a pinned date to this path.
                let today = jiff::Zoned::now().date();
                result.proposal = Some(propose_amendment(
                    vault, &path, &meta, &proposed, ctx, journal, opts.evidence, today,
                )?);
            }
        }
    }
    Ok(result)
}

/// Convenience wrapper: convert values to literals, then delegate.
pub fn write(
    vault: &Path,
    target: &str,
    changes: &[(String, Value)],
    ctx: &WriteContext,
    journal: &mut Journal,
    opts: &WriteOpts<'_>,
) -> Result<WriteResult, WriteError> {
    let literals: Vec<(String, String)> =
        changes.iter().map(|(k, v)| (k.clone(), to_literal(v))).collect();
    write_literals(vault, target, &literals, ctx, journal, opts)
}

/// Mint a new note. Stamps an `id` if the text has none, and refuses to create one the write path
/// could never edit again.
pub fn create(
    vault: &Path,
    rel_path: &str,
    text: &str,
    ctx: &WriteContext,
    journal: &mut Journal,
    evidence: Option<&serde_json::Value>,
) -> Result<PathBuf, WriteError> {
    create_minting(vault, rel_path, text, ctx, journal, evidence, None)
}

/// Two-desktop design D3: [`create`] for a note an automatic step makes because an outside item
/// exists. With an `ids::import_key` the note takes `ids::import_id` exactly where `create` would
/// have put a minted id, and the journal record keeps its shape — only the id's value differs.
/// Without one it is `create`. An id already in `held` (the vault's ids, `ids::held_ids`, read once
/// per producer run) is refused as [`WriteError::IdHeld`]; a created id joins `held`. Every producer
/// dedups by `source_uid` first, so this is the belt.
pub fn create_imported(
    vault: &Path,
    rel_path: &str,
    text: &str,
    ctx: &WriteContext,
    journal: &mut Journal,
    held: &mut std::collections::BTreeSet<String>,
) -> Result<PathBuf, WriteError> {
    create_minting(vault, rel_path, text, ctx, journal, None, Some(held))
}

fn create_minting(
    vault: &Path,
    rel_path: &str,
    text: &str,
    ctx: &WriteContext,
    journal: &mut Journal,
    evidence: Option<&serde_json::Value>,
    mut held: Option<&mut std::collections::BTreeSet<String>>,
) -> Result<PathBuf, WriteError> {
    let path = crate::ids::inside_vault(vault, &vault.join(rel_path))?;
    if path.exists() {
        return Err(WriteError::Exists(rel_path.to_string()));
    }
    let (mut meta, _) = split_frontmatter(text)
        .map_err(|_| WriteError::NoFrontmatter(rel_path.to_string()))?;
    if meta.is_empty() {
        return Err(WriteError::NoFrontmatter(rel_path.to_string()));
    }

    let mut text = text.to_string();
    let existing = get_str(&meta, "id");
    let note_id = match existing {
        Some(id) if is_id(&id) => id,
        _ => {
            // D1-D3: an imported note's id is derived from the item, so two computers mint the same
            // one; every other note's is random, as ever.
            let imported = if held.is_some() { crate::ids::import_key(&path, &meta) } else { None };
            let minted = match imported {
                Some((kind, vendor, key)) => crate::ids::import_id(&kind, &vendor, &key),
                None => new_id(&kind_for(&path, Some(&meta))),
            };
            text = apply_frontmatter_fields_to_text(
                &text,
                &[("id".to_string(), minted.clone())],
                rel_path,
            )?;
            meta.insert(Value::String("id".into()), Value::String(minted.clone()));
            minted
        }
    };
    // D3: one id, one note. Checked before the journal, so a refusal leaves no trace.
    if held.as_deref().is_some_and(|held| held.contains(&note_id)) {
        return Err(WriteError::IdHeld(note_id));
    }
    // Never mint a note the write path cannot later edit.
    guard_block_style(&text)?;

    let rel_path = rel(vault, &path);
    let mut spec = NewRecord::new("create", &rel_path, &ctx.actor, &ctx.via);
    spec.id = Some(&note_id);
    spec.new = yaml_to_json(&Value::Mapping(meta));
    spec.run_id = ctx.run_id.as_deref();
    spec.evidence = evidence.cloned().unwrap_or(serde_json::Value::Null);
    let mut record = make_record(spec).map_err(|e| WriteError::Io(e.to_string()))?;
    journal.append(&mut record).map_err(|e| WriteError::Io(e.to_string()))?;

    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent).map_err(|e| WriteError::Io(e.to_string()))?;
    }
    pystr::write_text(&path, &text).map_err(|e| WriteError::Io(e.to_string()))?;
    if let Some(held) = held.as_deref_mut() {
        held.insert(note_id);
    }
    Ok(path)
}

fn free_slot(folder: &Path, name: &str) -> PathBuf {
    let mut target = folder.join(name);
    let stem = Path::new(name).file_stem().map(|s| s.to_string_lossy().to_string()).unwrap_or_default();
    let ext = Path::new(name).extension().map(|s| format!(".{}", s.to_string_lossy())).unwrap_or_default();
    let mut suffix = 2;
    while target.exists() {
        target = folder.join(format!("{stem}-{suffix}{ext}"));
        suffix += 1;
    }
    target
}

/// Re-state one field's CURRENT value in the journal, **without touching the note**
/// (R-C3′-exec-20, Task 6b). Returns whether a record was appended.
///
/// **Why it exists.** When the student rejects a sync amend card ("keep mine"), nothing on this
/// device changes — the value the student kept is already on the note — so nothing would travel,
/// and the other desktop, which won that field at its own end and has no card, would keep its value
/// for ever: an explicit choice silently lost. The rejection has to be said as a record. It cannot
/// go through [`write_literals`]: a write that changes nothing produces no record and no write
/// (F4), and that is exactly this write's shape.
///
/// **What it appends.** One `op: set` record for `field` on `target`: `old` is the value being
/// turned down (the card's `to`, the other desktop's value), `new` is the note's current value, `ts`
/// is fresh, and the actor/via/run_id are the caller's — the deck's, never `sync::ACTOR`, so it
/// travels on the next push like any local edit. On the other desktop it then applies cleanly
/// ("upstream never moved": that desktop still holds `old`), or, if that desktop has moved on
/// since, it is a conflict this fresh `ts` wins, and that desktop files a card of its own — never
/// silent either way. It goes through [`Journal::append`] like every other record, so its bytes are
/// `ledger::dumps_value`'s. **The note's frontmatter is read, never written.**
///
/// Nothing is appended when `old` equals the current value — there is no disagreement to re-state.
pub fn reassert(
    vault: &Path,
    target: &str,
    field: &str,
    old: &serde_json::Value,
    ctx: &WriteContext,
    journal: &mut Journal,
) -> Result<bool, WriteError> {
    let path = resolve_target(vault, target)?;
    let (_, meta) = load(&path, false)?;
    let current = yaml_to_json(&crate::yaml::get(&meta, field).cloned().unwrap_or(Value::Null));
    if &current == old {
        return Ok(false);
    }
    let rel_path = rel(vault, &path);
    let note_id = get_str(&meta, "id").filter(|s| is_id(s));
    let mut spec = NewRecord::new("set", &rel_path, &ctx.actor, &ctx.via);
    spec.id = note_id.as_deref();
    spec.field = Some(field);
    spec.old = old.clone();
    spec.new = current;
    spec.run_id = ctx.run_id.as_deref();
    let mut record = make_record(spec).map_err(|e| WriteError::Io(e.to_string()))?;
    journal.append(&mut record).map_err(|e| WriteError::Io(e.to_string()))?;
    Ok(true)
}

/// "Delete" = settle into `archive/` keeping frontmatter. **Nothing is ever unlinked** (F14).
pub fn delete(
    vault: &Path,
    target: &str,
    ctx: &WriteContext,
    journal: &mut Journal,
) -> Result<PathBuf, WriteError> {
    let path = resolve_target(vault, target)?;
    let (_, meta) = load(&path, false)?;
    let archive = vault.join("archive");
    std::fs::create_dir_all(&archive).map_err(|e| WriteError::Io(e.to_string()))?;
    let name = path.file_name().map(|n| n.to_string_lossy().to_string()).unwrap_or_default();
    let dest = free_slot(&archive, &name);

    let from = rel(vault, &path);
    let to = rel(vault, &dest);
    let mut spec = NewRecord::new("delete", &from, &ctx.actor, &ctx.via);
    let id = get_str(&meta, "id");
    spec.id = id.as_deref();
    spec.old = serde_json::Value::String(from.clone());
    spec.new = serde_json::Value::String(to);
    spec.run_id = ctx.run_id.as_deref();
    let mut record = make_record(spec).map_err(|e| WriteError::Io(e.to_string()))?;
    journal.append(&mut record).map_err(|e| WriteError::Io(e.to_string()))?;

    std::fs::rename(&path, &dest).map_err(|e| WriteError::Io(e.to_string()))?;
    Ok(dest)
}

pub fn move_note(
    vault: &Path,
    target: &str,
    new_rel: &str,
    ctx: &WriteContext,
    journal: &mut Journal,
) -> Result<PathBuf, WriteError> {
    let path = resolve_target(vault, target)?;
    let (_, meta) = load(&path, false)?;
    let dest = crate::ids::inside_vault(vault, &vault.join(new_rel))?;
    if dest.exists() {
        return Err(WriteError::Exists(new_rel.to_string()));
    }
    let from = rel(vault, &path);
    let mut spec = NewRecord::new("move", &from, &ctx.actor, &ctx.via);
    let id = get_str(&meta, "id");
    spec.id = id.as_deref();
    spec.old = serde_json::Value::String(from.clone());
    spec.new = serde_json::Value::String(new_rel.to_string());
    spec.run_id = ctx.run_id.as_deref();
    let mut record = make_record(spec).map_err(|e| WriteError::Io(e.to_string()))?;
    journal.append(&mut record).map_err(|e| WriteError::Io(e.to_string()))?;

    if let Some(parent) = dest.parent() {
        std::fs::create_dir_all(parent).map_err(|e| WriteError::Io(e.to_string()))?;
    }
    std::fs::rename(&path, &dest).map_err(|e| WriteError::Io(e.to_string()))?;
    Ok(dest)
}

pub fn append_body(
    vault: &Path,
    target: &str,
    line: &str,
    ctx: &WriteContext,
    journal: &mut Journal,
) -> Result<bool, WriteError> {
    let path = resolve_target(vault, target)?;
    let (_, meta) = load(&path, false)?;
    if let Some(problem) = single_line_problem(line) {
        return Err(WriteError::AppendLine(problem));
    }
    let text = pystr::read_text(&path).map_err(|e| WriteError::Io(e.to_string()))?;
    let body = if text.starts_with("---\n") {
        match text[4..].find("\n---\n") {
            Some(offset) => text[4 + offset + 5..].to_string(),
            None => text.clone(),
        }
    } else {
        text.clone()
    };
    let stamped = format!("> {line}");
    if pystr::splitlines(&body).iter().any(|l| *l == stamped) {
        return Ok(false); // idempotent, same rule as append_body_line
    }

    let rel_path = rel(vault, &path);
    let mut spec = NewRecord::new("append_body", &rel_path, &ctx.actor, &ctx.via);
    let id = get_str(&meta, "id");
    spec.id = id.as_deref();
    spec.new = serde_json::Value::String(line.to_string());
    spec.run_id = ctx.run_id.as_deref();
    let mut record = make_record(spec).map_err(|e| WriteError::Io(e.to_string()))?;
    journal.append(&mut record).map_err(|e| WriteError::Io(e.to_string()))?;

    append_body_line(&path, line)
}

/// An already-pending `kind: amend` proposal for the same target and the same **set of fields**.
///
/// Matched on the field-name set, not the values: a re-judgement that moved `due` again is still
/// the same open decision ("may the agent change `due` on this note?"), and approving the older one
/// answers it. Unreadable approvals are skipped, never fatal.
pub fn find_pending_amendment(
    vault: &Path,
    target_rel: &str,
    fields: &std::collections::BTreeSet<String>,
) -> Option<PathBuf> {
    let mut paths: Vec<PathBuf> = std::fs::read_dir(vault.join("approvals"))
        .ok()?
        .filter_map(|e| e.ok())
        .map(|e| e.path())
        .filter(|p| p.extension().map(|x| x == "md") == Some(true))
        .collect();
    paths.sort();

    for path in paths {
        let Some(meta) = read_meta(&path) else { continue };
        if get_str(&meta, "type").as_deref() != Some("approval")
            || get_str(&meta, "kind").as_deref() != Some("amend")
            || get_str(&meta, "status").as_deref() != Some("pending")
            || get_str(&meta, "target").as_deref() != Some(target_rel)
        {
            continue;
        }
        if let Some(Value::Mapping(changes)) = crate::yaml::get(&meta, "changes") {
            let names: std::collections::BTreeSet<String> = changes
                .keys()
                .filter_map(crate::yaml::text)
                .collect();
            if &names == fields {
                return Some(path);
            }
        }
    }
    None
}

/// The two meta-bind buttons every approval note carries, byte-for-byte
/// `engine/write.py:AMEND_BUTTONS` (lines 287–307).
///
/// A literal, not something built from a template: it is Obsidian markup that has to be identical
/// to what Python writes for `scripts/diff-engines-notes.ps1` to compare the two engines' proposals
/// as bytes, and identical to every other approval in `approvals/` for the deck's own parser.
/// Leading newline included — Python concatenates it straight after the `**Why proposed:**` line.
pub const AMEND_BUTTONS: &str = "\n```meta-bind-button\nlabel: Approve\nstyle: primary\naction:\n  type: updateMetadata\n  bindTarget: status\n  evaluate: false\n  value: approved\n```\n\n```meta-bind-button\nlabel: Reject\nstyle: destructive\naction:\n  type: updateMetadata\n  bindTarget: status\n  evaluate: false\n  value: rejected\n```\n";

/// Judge-once's other half: the agent may **re-propose, never silently overwrite** (Quinn,
/// 2026-08-29). Port of `engine/write.py:propose_amendment` (lines 333–353).
///
/// Called only from [`write_literals`], and only when the journal shows Quinn set the field himself
/// and `propose` is on. It mints one `kind: amend` approval naming every re-judged field, through
/// [`create`] — so the proposal is journalled like any other note and gets its own `id`.
///
/// Four details that are load-bearing:
///
/// 1. **`from` is the note's current value and `to` is the agent's**, both put through
///    `yaml::to_json` first (Python's `jsonable`), because `approvals::validate_amendment` refuses a
///    collection on either side and `approvals::apply_amendment` compares `from` against the note
///    before applying.
/// 2. **`expires: null`** — an amendment does not go stale on a date; it is answered or it is not.
/// 3. **`proposed_at` and `first_proposed_at` are both today.** The second is the S1 field
///    `age_days` and `oldest_pending_days` read, set once and never rewritten;
///    `approvals::defer_over_budget` may later move `proposed_at` and must not move that.
/// 4. **The stem carries six characters of a fresh id**, so two re-judgements of two different
///    fields on the same note never collide on a filename — and `find_pending_amendment` is what
///    stops the same field set minting a second card at all.
///
/// **The evidence branch, and its one documented divergence.** Python is `if evidence:` — plain
/// truthiness, so `None` **and an empty dict** both append nothing. `filter(|e| !e.is_null())` alone
/// would let `{}` through and write a bare `Evidence: {}` the other engine never writes, so the
/// filter tests for both. What is *not* reproducible is key ORDER in a multi-key mapping:
/// `json.dumps` keeps Python's insertion order, while `serde_json`'s `Map` is a `BTreeMap` (no
/// `preserve_order` feature) and `ledger::dumps_value` sorts as well, so `{"b": 1, "a": 2}` renders
/// as `{"b": 1, "a": 2}` there and `{"a": 2, "b": 1}` here. It is **not fixable in this task** —
/// `preserve_order` would change the shape of every journal record in the crate — and it is
/// unreachable from production, where `enrich` passes `None` and only a hand-typed
/// `write set --evidence` can supply one. `scripts/diff-engines-notes.ps1` therefore uses a
/// **one-key** evidence mapping on `w14`, exactly as `w2` already does, which compares the branch
/// byte for byte without tripping an ordering difference neither engine is wrong about.
#[allow(clippy::too_many_arguments)]
pub fn propose_amendment(
    vault: &Path,
    target_path: &Path,
    meta: &Mapping,
    changes: &[(String, Value, Value)],
    ctx: &WriteContext,
    journal: &mut Journal,
    evidence: Option<&serde_json::Value>,
    today: jiff::civil::Date,
) -> Result<PathBuf, WriteError> {
    let target_rel = rel(vault, target_path);
    let stem_of_target = target_path
        .file_stem()
        .map(|s| s.to_string_lossy().to_string())
        .unwrap_or_default();
    let mut names: Vec<String> = changes.iter().map(|(n, _, _)| n.clone()).collect();
    names.sort();
    let fields = names.join(", ");
    let suffix = {
        let id = new_id("appr");
        id.chars().skip(id.chars().count().saturating_sub(6)).collect::<String>()
    };
    let stem = format!("amend-{stem_of_target}-{suffix}");

    // `meta.get('title', target_path.stem)`: the stem is the ABSENT-key default, nothing more. A
    // present `title: null` interpolates as the literal `"None"` in Python's f-string and
    // `title: true` as `"True"` — `yaml::text` alone would answer `"true"`. Only reachable on a
    // malformed note, but "fall back to the stem" is the one thing `dict.get` does not do.
    //
    // Bounded gap, deliberate: a `title:` holding a LIST or a MAPPING renders as `"None"` here and
    // as `str(list)` / `str(dict)` in Python (`['a', 'b']`). Reproducing Python's `repr` of a
    // collection is a module of its own, the note would already be malformed, and the blast radius
    // is one card's title text — no field value, no journal record.
    let title = match crate::yaml::get(meta, "title") {
        None => stem_of_target.clone(),
        Some(Value::Bool(b)) => if *b { "True" } else { "False" }.to_string(),
        Some(v) => crate::yaml::text(v).unwrap_or_else(|| "None".to_string()),
    };
    let mut change_block: Vec<(crate::yamlemit::Node, crate::yamlemit::Node)> = Vec::new();
    for (name, old, new) in changes {
        change_block.push((
            crate::yamlemit::Node::text(name),
            crate::yamlemit::Node::map(vec![
                ("from", crate::yamlemit::Node::from_json(&yaml_to_json(old))),
                ("to", crate::yamlemit::Node::from_json(&yaml_to_json(new))),
            ]),
        ));
    }
    let front = crate::yamlemit::Node::map(vec![
        ("type", crate::yamlemit::Node::text("approval")),
        ("kind", crate::yamlemit::Node::text("amend")),
        ("title", crate::yamlemit::Node::text(&format!("Re-proposed {fields} for {title}"))),
        ("status", crate::yamlemit::Node::text("pending")),
        ("target", crate::yamlemit::Node::text(&target_rel)),
        // Two plain dates, and `engine/write.py` spells `today.replace()` for the second so that
        // it is a DISTINCT object. PyYAML aliases by identity, so one object in both slots would
        // write `proposed_at: &id001 …` / `first_proposed_at: *id001` — which
        // `approvals::defer_over_budget` then breaks by rewriting the `proposed_at` line alone.
        // Anchors are incompatible with single-line surgery; see the ruling in `src/yamlemit.rs`.
        ("proposed_at", crate::yamlemit::Node::Date(today)),
        ("first_proposed_at", crate::yamlemit::Node::Date(today)),
        ("expires", crate::yamlemit::Node::Null),
        ("snooze_until", crate::yamlemit::Node::Null),
        ("created_by", crate::yamlemit::Node::text(&ctx.actor)),
        ("changes", crate::yamlemit::Node::Map(change_block)),
    ]);
    let mut why = format!(
        "{} re-judged {fields}; Quinn had set them by hand, so this is a proposal (judge-once rule).",
        ctx.actor
    );
    // Python's `if evidence:` is TRUTHINESS, so an empty mapping appends nothing — not just `None`.
    // A `!e.is_null()` test alone would write a bare `Evidence: {}` the other engine never writes.
    let truthy = |e: &&serde_json::Value| match e {
        serde_json::Value::Null => false,
        serde_json::Value::Object(m) => !m.is_empty(),
        serde_json::Value::Array(a) => !a.is_empty(),
        serde_json::Value::String(s) => !s.is_empty(),
        serde_json::Value::Bool(b) => *b,
        serde_json::Value::Number(n) => n.as_f64().map(|f| f != 0.0).unwrap_or(true),
    };
    if let Some(ev) = evidence.filter(truthy) {
        why.push_str(" Evidence: ");
        why.push_str(&crate::ledger::dumps_value(ev));
    }
    let text = format!(
        "---\n{}---\n\n**Why proposed:** {why}\n{AMEND_BUTTONS}",
        crate::yamlemit::safe_dump_block(&front)
    );
    create(vault, &format!("approvals/{stem}.md"), &text, ctx, journal, evidence)
}

// ---------------------------------------------------------------------------
// `python -m engine.write` (Task 16 step 6)
// ---------------------------------------------------------------------------

/// The subcommands `engine.write.main` exposes. `--via` is validated by clap (argparse's
/// `choices`, exit 2), everything else here.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum WriteCommand {
    Set {
        target: String,
        /// `field=literal`, see [`parse_assignments`].
        pairs: Vec<String>,
        judged: bool,
        propose: bool,
        /// JSON mapping recorded in the judgment block.
        inputs: Option<String>,
        /// JSON mapping recorded on each field's journal record.
        evidence: Option<String>,
    },
    Create {
        rel_path: String,
        from_file: PathBuf,
    },
    Delete {
        target: String,
    },
    Move {
        target: String,
        new_rel: String,
    },
    AppendBody {
        target: String,
        line: String,
    },
}

/// `_parse_assignments`: split each pair on its **first** `=`, strip both halves. A pair with no
/// `=` is a `raise SystemExit(str)` — Python prints the message and exits **1**, not argparse's
/// 2. The result was a dict, so a repeated field keeps its first position and its last value.
pub fn parse_assignments(pairs: &[String]) -> Result<Vec<(String, String)>, CliError> {
    let mut literals: Vec<(String, String)> = Vec::new();
    for pair in pairs {
        let Some((name, literal)) = pair.split_once('=') else {
            return Err(CliError::Failed(format!("expected field=literal, got {}", py_repr(pair))));
        };
        let name = pystr::strip(name).to_string();
        let literal = pystr::strip(literal).to_string();
        match literals.iter_mut().find(|(n, _)| *n == name) {
            Some(slot) => slot.1 = literal,
            None => literals.push((name, literal)),
        }
    }
    Ok(literals)
}

/// Python's `repr()` of a str, as far as the message above can ever need it: single quotes unless
/// the text holds a single quote and no double quote, and a backslash doubled.
fn py_repr(text: &str) -> String {
    let escaped = text.replace('\\', "\\\\");
    if text.contains('\'') && !text.contains('"') {
        format!("\"{escaped}\"")
    } else {
        format!("'{}'", escaped.replace('\'', "\\'"))
    }
}

fn json_arg(raw: Option<&str>) -> Result<Option<serde_json::Value>, CliError> {
    match raw {
        // `if args.inputs` — an empty string is falsy and means "none", not a parse error.
        Some(text) if !text.is_empty() => serde_json::from_str(text)
            .map(Some)
            .map_err(|e| CliError::Failed(e.to_string())),
        _ => Ok(None),
    }
}

/// What `engine.write.main` prints for one command, or the exit code it would have died with.
///
/// Every failure below is an uncaught exception in Python — `FileNotFoundError`, `ValueError`,
/// `json.JSONDecodeError`, the `SystemExit(str)` of a bad pair — and all of them exit 1; the
/// message text is this crate's, not the interpreter's. One narrowing: `--inputs` must be a JSON
/// **object** (Python would store any JSON value in the judgment block; no caller passes one).
pub fn cli(
    vault: &Path,
    actor: &str,
    via: &str,
    run_id: Option<&str>,
    command: &WriteCommand,
) -> Result<Vec<String>, CliError> {
    let ctx = WriteContext { actor: actor.into(), via: via.into(), run_id: run_id.map(str::to_string) };
    let mut journal = Journal::new(vault);
    let failed = |e: WriteError| CliError::Failed(e.to_string());
    match command {
        WriteCommand::Set { target, pairs, judged, propose, inputs, evidence } => {
            // `json.loads` runs before the note is resolved, so a bad argument never reaches it.
            let inputs = json_arg(inputs.as_deref())?
                .map(|v| match crate::yaml::from_json(&v) {
                    Value::Mapping(m) => Ok(m),
                    _ => Err(CliError::Failed("--inputs must be a JSON object".into())),
                })
                .transpose()?;
            let evidence = json_arg(evidence.as_deref())?;
            let literals = parse_assignments(pairs)?;
            let opts = WriteOpts {
                judged: *judged,
                evidence: evidence.as_ref(),
                propose: *propose,
                inputs: inputs.as_ref(),
            };
            let res = write_literals(vault, target, &literals, &ctx, &mut journal, &opts).map_err(failed)?;
            let mut lines: Vec<String> = res.written.iter().map(|(name, _)| format!("set {name}")).collect();
            // `res.skipped` is a dict in insertion order: the judge-once skips in pair order, then
            // the "proposal already pending" ones, also in pair order.
            let pending = "judge-once: proposal already pending";
            for wants_pending in [false, true] {
                for (name, _) in &literals {
                    if let Some(why) = res.skipped.get(name) {
                        if why.starts_with(pending) == wants_pending {
                            lines.push(format!("skipped {name}: {why}"));
                        }
                    }
                }
            }
            if let Some(proposal) = &res.proposal {
                let name = proposal.file_name().map(|n| n.to_string_lossy().to_string()).unwrap_or_default();
                lines.push(format!("proposed {name}"));
            }
            Ok(lines)
        }
        WriteCommand::Create { rel_path, from_file } => {
            let text = pystr::read_text(from_file).map_err(|e| CliError::Failed(e.to_string()))?;
            let path = create(vault, rel_path, &text, &ctx, &mut journal, None).map_err(failed)?;
            Ok(vec![format!("created {}", python_path(&path))])
        }
        WriteCommand::Delete { target } => {
            let dest = delete(vault, target, &ctx, &mut journal).map_err(failed)?;
            Ok(vec![format!("archived to {}", python_path(&dest))])
        }
        WriteCommand::Move { target, new_rel } => {
            let dest = move_note(vault, target, new_rel, &ctx, &mut journal).map_err(failed)?;
            Ok(vec![format!("moved to {}", python_path(&dest))])
        }
        WriteCommand::AppendBody { target, line } => {
            let appended = append_body(vault, target, line, &ctx, &mut journal).map_err(failed)?;
            Ok(vec![if appended { "appended" } else { "already present" }.to_string()])
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn vault() -> PathBuf {
        let dir = std::env::temp_dir().join(format!(
            "qo-write-{}-{:?}",
            std::process::id(),
            std::thread::current().id()
        ));
        let _ = std::fs::remove_dir_all(&dir);
        for folder in crate::ids::NOTE_FOLDERS {
            std::fs::create_dir_all(dir.join(folder)).unwrap();
        }
        std::fs::create_dir_all(dir.join("state").join("journal")).unwrap();
        dir
    }

    const NOTE: &str = "---\ntitle: \"CS 100 HW 01\"\ncourse: cs-100\ndue: 2026-08-26T23:59\neffort_hours: 2.5\nimportance: 2\nstatus: active\nid: task_0123456789\n---\n\nBody text.\n";

    fn seed(v: &Path) -> PathBuf {
        let path = v.join("tasks").join("a.md");
        pystr::write_text(&path, NOTE).unwrap();
        path
    }

    fn lit(pairs: &[(&str, &str)]) -> Vec<(String, String)> {
        pairs.iter().map(|(k, x)| (k.to_string(), x.to_string())).collect()
    }

    // -- to_literal ---------------------------------------------------------

    #[test]
    fn to_literal_leaves_an_iso_date_unquoted() {
        // Quoting it would silently retype `due` from a date to a str.
        assert_eq!(to_literal(&Value::String("2026-08-26".into())), "2026-08-26");
        assert_eq!(to_literal(&Value::String("2026-08-26T23:59".into())), "2026-08-26T23:59");
        assert_eq!(to_literal(&Value::String("2026-08-26T23:59:01".into())), "2026-08-26T23:59:01");
    }

    #[test]
    fn to_literal_quotes_ordinary_strings_and_keeps_unicode_literal() {
        assert_eq!(to_literal(&Value::String("hello".into())), "\"hello\"");
        // ensure_ascii=False: an em dash stays an em dash, matching coursework/ingest.
        assert_eq!(to_literal(&Value::String("a — b".into())), "\"a — b\"");
    }

    #[test]
    fn to_literal_renders_scalars_the_way_the_vault_already_does() {
        assert_eq!(to_literal(&Value::Null), "null");
        assert_eq!(to_literal(&Value::Bool(true)), "true");
        assert_eq!(to_literal(&Value::Bool(false)), "false");
        assert_eq!(to_literal(&serde_yaml_ng::from_str::<Value>("2.5").unwrap()), "2.5");
    }

    // -- single-line rule ---------------------------------------------------

    #[test]
    fn single_line_problem_uses_pythons_wide_boundary_set() {
        assert_eq!(single_line_problem("plain"), None);
        assert_eq!(single_line_problem("two\nlines"), Some("must be a single line"));
        // The bug this rule exists for: U+2028 is a splitlines boundary, so a line carrying one
        // never matched itself on re-read and was appended again on every pass.
        assert_eq!(single_line_problem("a\u{2028}b"), Some("must be a single line"));
        assert_eq!(single_line_problem("  --- x"), Some("must not start a frontmatter block"));
    }

    // -- the ordering invariant --------------------------------------------

    #[test]
    fn the_journal_record_is_written_before_the_note() {
        // Make the note unwritable by replacing it with a DIRECTORY of the same name after the
        // journal append would have happened. Simpler and portable: delete the note between
        // resolve and write is not reachable, so instead assert the record exists after a write
        // whose file edit fails because the frontmatter marker is gone.
        let v = vault();
        let path = seed(&v);
        let mut j = Journal::new(&v);
        let ctx = WriteContext::new("quinn", "cli");

        // Corrupt the closing marker so apply_frontmatter_fields_to_text fails.
        pystr::write_text(&path, "---\nid: task_0123456789\nstatus: active\n\nno closing marker\n").unwrap();
        let err = write_literals(&v, "tasks/a.md", &lit(&[("status", "done")]), &ctx, &mut j, &WriteOpts::default());
        assert!(err.is_err(), "the file edit must fail for this test to mean anything");

        // ...and the journal still carries the record, because it was appended first.
        let records = j.read(None, None);
        assert!(
            records.iter().any(|r| r.get("field").and_then(|v| v.as_str()) == Some("status")),
            "journal-first was violated: {records:?}"
        );
    }

    #[test]
    fn a_write_records_then_edits_exactly_one_line() {
        let v = vault();
        let path = seed(&v);
        let mut j = Journal::new(&v);
        let ctx = WriteContext::new("quinn", "cli");
        let res = write_literals(&v, "tasks/a.md", &lit(&[("status", "archived")]), &ctx, &mut j, &WriteOpts::default()).unwrap();

        assert_eq!(res.written, vec![("status".to_string(), "archived".to_string())]);
        assert_eq!(res.records.len(), 1);
        let after = pystr::read_text(&path).unwrap();
        assert!(after.contains("status: archived"));
        assert!(after.contains("title: \"CS 100 HW 01\""), "other lines must be untouched");
        assert!(after.ends_with("Body text.\n"));
    }

    #[test]
    fn a_no_op_field_produces_no_record_and_no_write() {
        // F4.
        let v = vault();
        seed(&v);
        let mut j = Journal::new(&v);
        let ctx = WriteContext::new("quinn", "cli");
        let res = write_literals(&v, "tasks/a.md", &lit(&[("status", "active")]), &ctx, &mut j, &WriteOpts::default()).unwrap();
        assert!(res.written.is_empty());
        assert!(res.records.is_empty());
        assert!(j.read(None, None).is_empty());
    }

    #[test]
    fn resolving_by_id_and_by_path_reach_the_same_note() {
        let v = vault();
        seed(&v);
        let mut j = Journal::new(&v);
        let ctx = WriteContext::new("quinn", "cli");
        let a = write_literals(&v, "task_0123456789", &lit(&[("importance", "1")]), &ctx, &mut j, &WriteOpts::default()).unwrap();
        let b = write_literals(&v, "tasks/a.md", &lit(&[("importance", "4")]), &ctx, &mut j, &WriteOpts::default()).unwrap();
        assert_eq!(a.path, b.path);
    }

    // -- judge-once ---------------------------------------------------------

    #[test]
    fn an_agent_cannot_overwrite_a_field_quinn_set() {
        let v = vault();
        seed(&v);
        let mut j = Journal::new(&v);
        let quinn = WriteContext::new("quinn", "dashboard");
        write_literals(&v, "tasks/a.md", &lit(&[("importance", "1")]), &quinn, &mut j, &WriteOpts::default()).unwrap();

        let agent = WriteContext::new("agent:routine.enrich", "cloud-routine");
        let opts = WriteOpts { judged: true, ..Default::default() };
        let res = write_literals(&v, "tasks/a.md", &lit(&[("importance", "5")]), &agent, &mut j, &opts).unwrap();

        assert!(res.written.is_empty(), "the agent must not write over a human decision");
        assert!(res.skipped.get("importance").unwrap().starts_with("judge-once: importance set by quinn at"));
    }

    #[test]
    fn an_agent_may_still_set_a_field_quinn_never_touched() {
        let v = vault();
        seed(&v);
        let mut j = Journal::new(&v);
        let agent = WriteContext::new("agent:routine.enrich", "cloud-routine");
        let opts = WriteOpts { judged: true, ..Default::default() };
        let res = write_literals(&v, "tasks/a.md", &lit(&[("importance", "5")]), &agent, &mut j, &opts).unwrap();
        assert_eq!(res.written.first().unwrap().0, "importance");
        // A judged write also stamps the provenance block, on ONE line.
        let judgment = res.written.iter().find(|(n, _)| n == "judgment").expect("judgment stamped");
        assert!(!judgment.1.contains('\n'), "judgment must be a single-line flow mapping");
        assert!(judgment.1.starts_with('{') && judgment.1.ends_with('}'));
    }

    #[test]
    fn a_human_write_is_never_subject_to_judge_once() {
        let v = vault();
        seed(&v);
        let mut j = Journal::new(&v);
        let quinn = WriteContext::new("quinn", "dashboard");
        write_literals(&v, "tasks/a.md", &lit(&[("importance", "1")]), &quinn, &mut j, &WriteOpts::default()).unwrap();
        let opts = WriteOpts { judged: true, ..Default::default() };
        let res = write_literals(&v, "tasks/a.md", &lit(&[("importance", "2")]), &quinn, &mut j, &opts).unwrap();
        assert_eq!(res.written.first().unwrap().0, "importance");
    }

    // -- create / delete / move / append -----------------------------------

    #[test]
    fn create_stamps_an_id_and_journals_the_whole_note() {
        let v = vault();
        let mut j = Journal::new(&v);
        let ctx = WriteContext::new("quinn", "cli");
        let text = "---\ntitle: \"New\"\nstatus: active\n---\n\nbody\n";
        let path = create(&v, "tasks/new.md", text, &ctx, &mut j, None).unwrap();

        let meta = read_meta(&path).unwrap();
        let id = get_str(&meta, "id").unwrap();
        assert!(is_id(&id));
        let records = j.read(None, None);
        assert_eq!(records.len(), 1);
        assert_eq!(records[0].get("op").and_then(|v| v.as_str()), Some("create"));
        assert!(records[0].get("new").unwrap().get("title").is_some(), "create records the whole mapping");
    }

    #[test]
    fn create_refuses_an_existing_path_and_a_note_without_frontmatter() {
        let v = vault();
        seed(&v);
        let mut j = Journal::new(&v);
        let ctx = WriteContext::new("quinn", "cli");
        assert!(matches!(
            create(&v, "tasks/a.md", NOTE, &ctx, &mut j, None),
            Err(WriteError::Exists(_))
        ));
        assert!(matches!(
            create(&v, "tasks/b.md", "no frontmatter\n", &ctx, &mut j, None),
            Err(WriteError::NoFrontmatter(_))
        ));
    }

    // -- create_imported (two-desktop design D3) ------------------------------

    const IMPORTED: &str = "---\ntitle: \"CS 100 HW 01\"\nstatus: active\ncreated_by: zybooks\nsource_uid: \"zybooks:1839992\"\n---\n\nbody\n";

    /// D3: the import id goes exactly where `create` puts a minted one, and the journal record has the
    /// same shape — only the id's value differs.
    #[test]
    fn create_imported_puts_the_import_id_where_create_puts_a_minted_one() {
        let v = vault();
        let mut j = Journal::new(&v);
        let ctx = WriteContext::new("agent:coursework.zybooks", "local-runner");
        let mut held = std::collections::BTreeSet::new();
        let imported = create_imported(&v, "tasks/imported.md", IMPORTED, &ctx, &mut j, &mut held).unwrap();
        let minted = create(&v, "tasks/minted.md", IMPORTED, &ctx, &mut j, None).unwrap();

        assert_eq!(get_str(&read_meta(&imported).unwrap(), "id").as_deref(), Some("task_18734fe8b7"));
        let lines = |p: &Path| pystr::read_text(p).unwrap().lines().map(str::to_string).collect::<Vec<_>>();
        let without_id = |p: &Path| lines(p).into_iter().filter(|l| !l.starts_with("id: ")).collect::<Vec<_>>();
        assert_eq!(without_id(&imported), without_id(&minted), "the same text but for the id's value");
        let at = |p: &Path| lines(p).iter().position(|l| l.starts_with("id: "));
        assert_eq!(at(&imported), at(&minted), "the id line sits where create puts it");

        let records = j.read(None, None);
        assert_eq!(records.len(), 2);
        let keys = |r: &crate::ledger::Record| r.keys().cloned().collect::<Vec<_>>();
        assert_eq!(keys(&records[0]), keys(&records[1]), "the same record shape");
        assert_eq!(records[0].get("op").and_then(|x| x.as_str()), Some("create"));
        assert_eq!(records[0].get("id").and_then(|x| x.as_str()), Some("task_18734fe8b7"));
        assert_eq!(records[0]["new"]["id"], serde_json::json!("task_18734fe8b7"), "the record carries the note as minted");
        assert!(held.contains("task_18734fe8b7"), "held grows as the run creates");
    }

    /// D3: an id the vault already holds is refused before anything is journalled or written.
    #[test]
    fn create_imported_refuses_an_id_the_vault_already_holds() {
        let v = vault();
        let mut j = Journal::new(&v);
        let ctx = WriteContext::new("agent:coursework.zybooks", "local-runner");
        let mut held = std::collections::BTreeSet::from(["task_18734fe8b7".to_string()]);
        let err = create_imported(&v, "tasks/again.md", IMPORTED, &ctx, &mut j, &mut held).unwrap_err();
        assert_eq!(err, WriteError::IdHeld("task_18734fe8b7".to_string()));
        assert_eq!(err.to_string(), "already held as task_18734fe8b7");
        assert!(!v.join("tasks").join("again.md").exists(), "no file");
        assert!(j.read(None, None).is_empty(), "no record");
    }

    /// D3: with no import key it is `create` — a random id — and a taken path is still `Exists`,
    /// checked first, exactly as `create` checks it.
    #[test]
    fn create_imported_without_a_key_mints_like_create_and_checks_the_path_first() {
        let v = vault();
        seed(&v);
        let mut j = Journal::new(&v);
        let ctx = WriteContext::new("quinn", "dashboard");
        let mut held = std::collections::BTreeSet::new();
        let mine = "---\ntitle: \"Mine\"\ncreated_by: quinn\n---\n\nb\n";
        let a = create_imported(&v, "tasks/a1.md", mine, &ctx, &mut j, &mut held).unwrap();
        let b = create_imported(&v, "tasks/a2.md", mine, &ctx, &mut j, &mut held).unwrap();
        let id = |p: &Path| get_str(&read_meta(p).unwrap(), "id").unwrap();
        assert!(is_id(&id(&a)) && is_id(&id(&b)));
        assert_ne!(id(&a), id(&b), "random, as new_id mints");
        assert!(matches!(
            create_imported(&v, "tasks/a.md", IMPORTED, &ctx, &mut j, &mut held),
            Err(WriteError::Exists(_))
        ));
    }

    /// An id the text already carries is kept, as `create` keeps it — and still refused when held.
    #[test]
    fn create_imported_keeps_an_id_the_text_carries_and_still_checks_it() {
        let v = vault();
        let mut j = Journal::new(&v);
        let ctx = WriteContext::new("agent:coursework.zybooks", "local-runner");
        let mut held = std::collections::BTreeSet::new();
        let text = IMPORTED.replace("status: active\n", "status: active\nid: task_0000000abc\n");
        let p = create_imported(&v, "tasks/carried.md", &text, &ctx, &mut j, &mut held).unwrap();
        assert_eq!(get_str(&read_meta(&p).unwrap(), "id").as_deref(), Some("task_0000000abc"));
        assert!(matches!(
            create_imported(&v, "tasks/carried-2.md", &text, &ctx, &mut j, &mut held),
            Err(WriteError::IdHeld(_))
        ));
    }

    #[test]
    fn delete_settles_into_archive_and_never_unlinks() {
        let v = vault();
        let path = seed(&v);
        let mut j = Journal::new(&v);
        let ctx = WriteContext::new("quinn", "cli");
        let dest = delete(&v, "tasks/a.md", &ctx, &mut j).unwrap();
        assert!(!path.exists());
        assert!(dest.exists(), "the note must still be on disk, in archive/");
        assert_eq!(dest.parent().unwrap().file_name().unwrap(), "archive");
        assert_eq!(pystr::read_text(&dest).unwrap(), NOTE, "frontmatter is kept verbatim");
    }

    #[test]
    fn delete_finds_a_free_slot_rather_than_clobbering() {
        let v = vault();
        seed(&v);
        pystr::write_text(&v.join("archive").join("a.md"), "---\nid: task_9999999999\n---\n\nold\n").unwrap();
        let mut j = Journal::new(&v);
        let ctx = WriteContext::new("quinn", "cli");
        let dest = delete(&v, "tasks/a.md", &ctx, &mut j).unwrap();
        assert_eq!(dest.file_name().unwrap(), "a-2.md");
    }

    #[test]
    fn move_relocates_and_refuses_to_overwrite() {
        let v = vault();
        seed(&v);
        let mut j = Journal::new(&v);
        let ctx = WriteContext::new("quinn", "cli");
        let dest = move_note(&v, "tasks/a.md", "archive/moved.md", &ctx, &mut j).unwrap();
        assert!(dest.exists());
        pystr::write_text(&v.join("tasks").join("a.md"), NOTE).unwrap();
        assert!(matches!(
            move_note(&v, "tasks/a.md", "archive/moved.md", &ctx, &mut j),
            Err(WriteError::Exists(_))
        ));
    }

    #[test]
    fn append_body_is_idempotent() {
        let v = vault();
        let path = seed(&v);
        let mut j = Journal::new(&v);
        let ctx = WriteContext::new("quinn", "cli");
        assert!(append_body(&v, "tasks/a.md", "a note line", &ctx, &mut j).unwrap());
        assert!(!append_body(&v, "tasks/a.md", "a note line", &ctx, &mut j).unwrap(), "second append must be a no-op");
        let text = pystr::read_text(&path).unwrap();
        assert_eq!(text.matches("> a note line").count(), 1);
        // ...and the no-op wrote no second record.
        let appends = j.read(None, None).iter().filter(|r| r.get("op").and_then(|v| v.as_str()) == Some("append_body")).count();
        assert_eq!(appends, 1);
    }

    #[test]
    fn append_body_refuses_a_multiline_value() {
        let v = vault();
        seed(&v);
        let mut j = Journal::new(&v);
        let ctx = WriteContext::new("quinn", "cli");
        assert!(matches!(
            append_body(&v, "tasks/a.md", "two\nlines", &ctx, &mut j),
            Err(WriteError::AppendLine("must be a single line"))
        ));
    }

    // -- pending amendments -------------------------------------------------

    #[test]
    fn find_pending_amendment_matches_on_the_field_set_not_the_values() {
        let v = vault();
        let card = "---\ntype: approval\nkind: amend\nstatus: pending\ntarget: tasks/a.md\nchanges:\n  due:\n    from: 2026-09-25T09:00\n    to: 2026-09-22T09:00\n---\n\nwhy\n";
        pystr::write_text(&v.join("approvals").join("amend-a.md"), card).unwrap();

        let mut due = std::collections::BTreeSet::new();
        due.insert("due".to_string());
        assert!(find_pending_amendment(&v, "tasks/a.md", &due).is_some());

        let mut other = std::collections::BTreeSet::new();
        other.insert("importance".to_string());
        assert!(find_pending_amendment(&v, "tasks/a.md", &other).is_none(), "a different field set is a different decision");
        assert!(find_pending_amendment(&v, "tasks/b.md", &due).is_none());
    }

    #[test]
    fn find_pending_amendment_skips_unreadable_approvals() {
        let v = vault();
        pystr::write_text(&v.join("approvals").join("broken.md"), "---\ntype: \"x\nkind: [unclosed\n---\n\nb\n").unwrap();
        let mut due = std::collections::BTreeSet::new();
        due.insert("due".to_string());
        assert!(find_pending_amendment(&v, "tasks/a.md", &due).is_none(), "must not panic on a broken card");
    }

    // -- `python -m engine.write` (Task 16 step 6) ---------------------------

    fn strs(items: &[&str]) -> Vec<String> {
        items.iter().map(|s| s.to_string()).collect()
    }

    /// `test_write.py::test_cli_set_and_create_round_trip`, minus its last line: the bad `--via`
    /// is argparse's refusal (exit 2), which is clap's job here and is asserted in `main.rs`.
    #[test]
    fn cli_set_and_create_round_trip() {
        let v = vault();
        let src = v.join("new.txt");
        pystr::write_text(&src, "---\ntitle: CLI\nprogress: 0\n---\n").unwrap();
        let created = cli(
            &v,
            "quinn",
            "cli",
            None,
            &WriteCommand::Create { rel_path: "tasks/cli.md".into(), from_file: src },
        )
        .unwrap();
        assert_eq!(created, vec![format!("created {}", python_path(&v.join("tasks").join("cli.md")))]);

        let set = cli(
            &v,
            "agent:routine.enrich",
            "cloud-routine",
            None,
            &WriteCommand::Set {
                target: "tasks/cli.md".into(),
                pairs: strs(&["importance=4", "importance_reason=\"has a colon: yes\""]),
                judged: true,
                propose: false,
                inputs: None,
                evidence: Some("{\"why\": \"weight\"}".into()),
            },
        )
        .unwrap();
        assert_eq!(set, strs(&["set importance", "set importance_reason", "set judgment"]));

        let meta = read_meta(&v.join("tasks").join("cli.md")).unwrap();
        assert_eq!(crate::yaml::get(&meta, "importance"), Some(&Value::from(4)));
        assert_eq!(get_str(&meta, "importance_reason").as_deref(), Some("has a colon: yes"));
        let records = Journal::new(&v).read(None, None);
        assert!(records.iter().any(|r| r.get("field") == Some(&serde_json::json!("importance"))
            && r.get("evidence") == Some(&serde_json::json!({"why": "weight"}))));
    }

    /// The rest of `engine.write.main`'s surface, which its Python test leaves to the reader:
    /// `_parse_assignments` (split on the first `=`, strip both halves, a pair without `=` is a
    /// `SystemExit` — exit 1, not argparse's 2), the judge-once skip line, and the three
    /// path-printing commands plus `append-body`'s two words.
    #[test]
    fn cli_covers_every_subcommand_and_prints_what_python_prints() {
        assert_eq!(
            parse_assignments(&strs(&[" due = 2026-09-25 ", "title=a=b"])).unwrap(),
            lit(&[("due", "2026-09-25"), ("title", "a=b")])
        );
        assert_eq!(
            parse_assignments(&strs(&["progress"])).unwrap_err(),
            CliError::Failed("expected field=literal, got 'progress'".into())
        );

        let v = vault();
        seed(&v);
        let ctx_q = WriteContext::new("quinn", "dashboard");
        write_literals(&v, "tasks/a.md", &lit(&[("importance", "3")]), &ctx_q, &mut Journal::new(&v), &WriteOpts::default()).unwrap();
        let set = cli(
            &v,
            "agent:routine.enrich",
            "cloud-routine",
            Some("cloud-1"),
            &WriteCommand::Set {
                target: "tasks/a.md".into(),
                pairs: strs(&["importance=5", "progress=10"]),
                judged: true,
                propose: false,
                inputs: Some("{\"source\": \"syllabus\"}".into()),
                evidence: None,
            },
        )
        .unwrap();
        assert_eq!(set.len(), 2, "{set:?}");
        assert_eq!(set[0], "set progress");
        assert!(set[1].starts_with("skipped importance: judge-once: importance set by quinn at "), "{}", set[1]);
        let records = Journal::new(&v).read(None, None);
        assert!(records.iter().any(|r| r.get("field") == Some(&serde_json::json!("progress"))
            && r.get("run_id") == Some(&serde_json::json!("cloud-1"))));

        let appended = cli(&v, "quinn", "cli", None, &WriteCommand::AppendBody { target: "tasks/a.md".into(), line: "note".into() }).unwrap();
        assert_eq!(appended, strs(&["appended"]));
        let again = cli(&v, "quinn", "cli", None, &WriteCommand::AppendBody { target: "tasks/a.md".into(), line: "note".into() }).unwrap();
        assert_eq!(again, strs(&["already present"]));

        let moved = cli(&v, "quinn", "cli", None, &WriteCommand::Move { target: "tasks/a.md".into(), new_rel: "tasks/b.md".into() }).unwrap();
        assert_eq!(moved, vec![format!("moved to {}", python_path(&v.join("tasks").join("b.md")))]);
        let archived = cli(&v, "quinn", "cli", None, &WriteCommand::Delete { target: "tasks/b.md".into() }).unwrap();
        assert_eq!(archived, vec![format!("archived to {}", python_path(&v.join("archive").join("b.md")))]);

        // Failures are the uncaught exceptions Python dies with: exit 1.
        let missing = cli(&v, "quinn", "cli", None, &WriteCommand::Delete { target: "tasks/zzz.md".into() }).unwrap_err();
        assert_eq!(missing.exit_code(), 1);
        // `json.loads` runs before the note is even resolved, so the target here is real.
        let bad_json = cli(
            &v,
            "quinn",
            "cli",
            None,
            &WriteCommand::Set { target: "archive/b.md".into(), pairs: strs(&["progress=1"]), judged: false, propose: false, inputs: None, evidence: Some("{nope".into()) },
        )
        .unwrap_err();
        assert!(matches!(bad_json, CliError::Failed(_)), "{bad_json}");
        assert_eq!(bad_json.exit_code(), 1);
        assert_eq!(
            read_meta(&v.join("archive").join("b.md")).and_then(|m| get_str(&m, "progress")).as_deref(),
            Some("10"),
            "still the earlier value: nothing written"
        );
    }

    // -- judge-once re-proposal (plan 3a Task 6) ----------------------------

    fn propose_vault(tag: &str) -> PathBuf {
        let d = std::env::temp_dir().join(format!("qo-write-propose-{tag}-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&d);
        std::fs::create_dir_all(d.join("tasks")).unwrap();
        std::fs::create_dir_all(d.join("approvals")).unwrap();
        pystr::write_text(
            &d.join("tasks").join("t.md"),
            "---\ntitle: \"CS 100 HW 01\"\ncourse: cs-100\neffort_hours: 2.5\nimportance: 2\nstatus: active\nprogress: 0\nid: task_0123456789\n---\n\nBody.\n",
        ).unwrap();
        d
    }

    /// The judge-once loop, end to end: Quinn sets a field, an agent re-judges it, and instead of
    /// overwriting it the write path mints a `kind: amend` card the deck can approve. This is the
    /// path that returned `ProposeNotPorted` before this task.
    #[test]
    fn a_judged_write_over_a_field_quinn_set_files_an_amendment() {
        let v = propose_vault("files");
        let mut journal = Journal::new(&v);
        // Quinn sets it by hand.
        write_literals(&v, "tasks/t.md", &[("effort_hours".to_string(), "4.0".to_string())],
            &WriteContext::new("quinn", "dashboard"), &mut journal, &WriteOpts::default()).unwrap();
        // The agent re-judges it.
        let res = write_literals(
            &v, "tasks/t.md",
            &[("effort_hours".to_string(), "2.0".to_string())],
            &WriteContext::new("agent:knowlu.enrich", "local-runner"),
            &mut journal,
            &WriteOpts { judged: true, propose: true, ..Default::default() },
        ).unwrap();

        let path = res.proposal.expect("an amendment was filed");
        assert!(path.starts_with(v.join("approvals")), "{path:?}");
        let text = pystr::read_text(&path).unwrap();
        let (meta, body) = crate::models::split_frontmatter(&text).unwrap();
        assert_eq!(get_str(&meta, "type").as_deref(), Some("approval"));
        assert_eq!(get_str(&meta, "kind").as_deref(), Some("amend"));
        assert_eq!(get_str(&meta, "status").as_deref(), Some("pending"));
        assert_eq!(get_str(&meta, "target").as_deref(), Some("tasks/t.md"));
        assert_eq!(get_str(&meta, "created_by").as_deref(), Some("agent:knowlu.enrich"));
        // S1: `first_proposed_at` is set once and is what `age_days` reads.
        assert_eq!(get_str(&meta, "first_proposed_at"), get_str(&meta, "proposed_at"));
        assert!(body.contains("re-judged effort_hours"), "{body}");
        assert!(body.contains("meta-bind-button"), "the deck's two buttons, as every proposal has");

        // The note itself is untouched — that is the whole point.
        let note = pystr::read_text(&v.join("tasks").join("t.md")).unwrap();
        assert!(note.contains("effort_hours: 4.0"), "judge-once held: {note}");

        // And the proposal is one the engine can actually apply.
        assert!(crate::approvals::validate_amendment(&v, &meta).is_ok(), "an unappliable proposal is worse than none");
        let _ = std::fs::remove_dir_all(&v);
    }

    /// An amend card must survive the single-line surgery `defer_over_budget` performs on it.
    ///
    /// `proposed_at` and `first_proposed_at` were the SAME `date` object until 2026-09-07, and
    /// PyYAML aliases by identity, so every card read `proposed_at: &id001 2026-09-07` /
    /// `first_proposed_at: *id001`. `approvals::defer_over_budget` then rewrites the `proposed_at`
    /// **line alone** — single-line frontmatter surgery, this system's core edit invariant — which
    /// leaves the alias pointing at nothing. The card stops parsing, every tolerant reader drops it
    /// silently, and it leaves the deck *and* `find_pending_amendment`, so the next run mints a
    /// second card for the same decision and spends another slot of the day's budget.
    ///
    /// Minted, deferred, then **loaded**: `read_meta` is the tolerant reader, and `Some` is the
    /// assertion. This test fails against the anchored spelling both engines briefly carried.
    #[test]
    fn a_deferred_amendment_still_parses_after_its_proposed_at_line_is_rewritten() {
        let v = propose_vault("defer");
        let mut journal = Journal::new(&v);
        write_literals(&v, "tasks/t.md", &[("effort_hours".to_string(), "4.0".to_string())],
            &WriteContext::new("quinn", "dashboard"), &mut journal, &WriteOpts::default()).unwrap();
        let res = write_literals(
            &v, "tasks/t.md",
            &[("effort_hours".to_string(), "2.0".to_string())],
            &WriteContext::new("agent:knowlu.enrich", "local-runner"),
            &mut journal,
            &WriteOpts { judged: true, propose: true, ..Default::default() },
        ).unwrap();
        let card = res.proposal.expect("a card");
        let text = pystr::read_text(&card).unwrap();
        assert!(!text.contains("&id"), "no note this system writes may carry an anchor: {text}");

        // `today` is read back from the card's own `first_proposed_at` rather than from a fresh
        // `Zoned::now()` call here: the mint above already stamped one wall-clock read into the
        // card, and calling `now()` again would risk a midnight rollover between the mint and
        // this line desyncing the two — making `defer_over_budget` miss the card for a reason
        // that is not a defect. Pin the date once (the mint's own stamp) and reuse it for both
        // the `defer_over_budget` call and the assertions below, the way the Python half of this
        // guard pins one `today` and passes it to both `write()` and `defer_over_budget`.
        let minted = crate::ids::read_meta(&card).expect("the freshly minted card parses");
        let today: jiff::civil::Date = get_str(&minted, "first_proposed_at")
            .expect("first_proposed_at is set on mint")
            .parse()
            .expect("first_proposed_at is a plain YYYY-MM-DD stamp");
        let ctx = WriteContext::new("quinn", "local-runner");
        let deferred = crate::approvals::defer_over_budget(&v, today, 0, &ctx, &mut journal);
        assert_eq!(deferred.len(), 1, "budget 0 defers the card: {deferred:?}");

        let meta = crate::ids::read_meta(&card).expect("the deferred card still parses");
        let tomorrow = today.tomorrow().unwrap().strftime("%Y-%m-%d").to_string();
        let stamp = today.strftime("%Y-%m-%d").to_string();
        assert_eq!(get_str(&meta, "proposed_at"), Some(tomorrow));
        assert_eq!(
            get_str(&meta, "first_proposed_at"),
            Some(stamp),
            "S1: set once, never moved by a deferral"
        );

        // The control, so the `expect` above is not a tautology: this is the shape the anchor left
        // behind, and `read_meta` — the tolerant reader every scan uses — answers None. Not an
        // error anyone sees; the card simply stops existing.
        let broken = v.join("approvals").join("broken.md");
        pystr::write_text(
            &broken,
            "---\ntype: approval\nkind: amend\nstatus: pending\nproposed_at: 2026-09-08\nfirst_proposed_at: *id001\n---\n\nx\n",
        ).unwrap();
        assert!(crate::ids::read_meta(&broken).is_none(), "a dangling alias reads as no note at all");
        let _ = std::fs::remove_dir_all(&v);
    }

    /// Re-judging twice a day must not bury the queue. The second run finds the first proposal and
    /// points at it — matched on the FIELD SET, because approving the older one answers the same
    /// question.
    #[test]
    fn a_second_re_judgement_reuses_the_pending_proposal() {
        let v = propose_vault("dedup");
        let mut journal = Journal::new(&v);
        write_literals(&v, "tasks/t.md", &[("effort_hours".to_string(), "4.0".to_string())],
            &WriteContext::new("quinn", "dashboard"), &mut journal, &WriteOpts::default()).unwrap();
        let ctx = WriteContext::new("agent:knowlu.enrich", "local-runner");
        let opts = WriteOpts { judged: true, propose: true, ..Default::default() };
        let first = write_literals(&v, "tasks/t.md", &[("effort_hours".to_string(), "2.0".to_string())], &ctx, &mut journal, &opts).unwrap();
        let second = write_literals(&v, "tasks/t.md", &[("effort_hours".to_string(), "1.0".to_string())], &ctx, &mut journal, &opts).unwrap();
        assert_eq!(first.proposal, second.proposal, "one open decision, one card");
        assert!(second.skipped.get("effort_hours").unwrap().contains("already pending"), "{:?}", second.skipped);
        let n = std::fs::read_dir(v.join("approvals")).unwrap().flatten().count();
        assert_eq!(n, 1, "a second card would spend the day's approval budget on one decision");
        let _ = std::fs::remove_dir_all(&v);
    }

    /// The 2026-08-21 hardening rule: `validate_amendment` refuses a null `from`, so a proposal it
    /// could never apply is not minted at all — it is reported as a skip instead.
    ///
    /// The seeded note carries `importance_reason` with a value, so Quinn's `= null` is a real
    /// change and lands a journal record. Setting an ABSENT field to null is a no-op on both
    /// engines (`jsonable(old) == jsonable(new)`), which would journal nothing and leave the agent
    /// write with no human record to find — a test that passed for the wrong reason.
    #[test]
    fn a_field_quinn_set_to_nothing_is_skipped_rather_than_proposed_from_null() {
        let v = propose_vault("null");
        pystr::write_text(
            &v.join("tasks").join("t.md"),
            "---\ntitle: \"T\"\nimportance_reason: \"Quinn's own words.\"\nstatus: active\nid: task_0123456789\n---\n\nBody.\n",
        ).unwrap();
        let mut journal = Journal::new(&v);
        write_literals(&v, "tasks/t.md", &[("importance_reason".to_string(), "null".to_string())],
            &WriteContext::new("quinn", "dashboard"), &mut journal, &WriteOpts::default()).unwrap();
        let res = write_literals(
            &v, "tasks/t.md",
            &[("importance_reason".to_string(), "\"Worth 15%.\"".to_string())],
            &WriteContext::new("agent:knowlu.enrich", "local-runner"),
            &mut journal,
            &WriteOpts { judged: true, propose: true, ..Default::default() },
        ).unwrap();
        assert!(res.proposal.is_none());
        assert!(res.skipped.get("importance_reason").unwrap().contains("cannot propose from null"));
        let _ = std::fs::remove_dir_all(&v);
    }

    /// Several fields re-judged at once is ONE card naming all of them — one decision, one
    /// proposal, one charge against the day's budget.
    #[test]
    fn several_fields_re_judged_together_make_one_card() {
        let v = propose_vault("multi");
        let mut journal = Journal::new(&v);
        let quinn = WriteContext::new("quinn", "dashboard");
        write_literals(&v, "tasks/t.md", &[
            ("effort_hours".to_string(), "4.0".to_string()),
            ("importance".to_string(), "5".to_string()),
        ], &quinn, &mut journal, &WriteOpts::default()).unwrap();
        let res = write_literals(
            &v, "tasks/t.md",
            &[("effort_hours".to_string(), "2.0".to_string()), ("importance".to_string(), "3".to_string())],
            &WriteContext::new("agent:knowlu.enrich", "local-runner"),
            &mut journal,
            &WriteOpts { judged: true, propose: true, ..Default::default() },
        ).unwrap();
        let path = res.proposal.expect("one card");
        let meta = crate::ids::read_meta(&path).unwrap();
        let Some(serde_yaml_ng::Value::Mapping(changes)) = crate::yaml::get(&meta, "changes") else {
            panic!("no changes block");
        };
        assert_eq!(changes.len(), 2);
        assert_eq!(crate::yaml::get(changes, "effort_hours").and_then(|c| crate::yaml::get(c.as_mapping().unwrap(), "from")).and_then(crate::yaml::f64_of), Some(4.0));
        assert_eq!(crate::yaml::get(changes, "importance").and_then(|c| crate::yaml::get(c.as_mapping().unwrap(), "to")).and_then(crate::yaml::i64_of), Some(3));
        let _ = std::fs::remove_dir_all(&v);
    }

    /// Python's `if evidence:` is truthiness, so an EMPTY mapping appends nothing. `!is_null()`
    /// alone would write `Evidence: {}` here and nothing there — a one-line byte difference on a
    /// note, which is the class of divergence the dual-run scripts exist to catch.
    #[test]
    fn empty_evidence_appends_nothing_and_a_real_one_appends_itself() {
        let v = propose_vault("evidence");
        let mut journal = Journal::new(&v);
        let quinn = WriteContext::new("quinn", "dashboard");
        write_literals(&v, "tasks/t.md", &[("importance".to_string(), "5".to_string())], &quinn, &mut journal, &WriteOpts::default()).unwrap();
        let ctx = WriteContext::new("agent:knowlu.enrich", "local-runner");

        let empty = serde_json::json!({});
        let res = write_literals(&v, "tasks/t.md", &[("importance".to_string(), "2".to_string())], &ctx, &mut journal,
            &WriteOpts { judged: true, propose: true, evidence: Some(&empty), inputs: None }).unwrap();
        let body = pystr::read_text(&res.proposal.clone().unwrap()).unwrap();
        assert!(!body.contains("Evidence:"), "an empty mapping is falsy in Python: {body}");

        // A different field set, so `find_pending_amendment` mints a second card rather than
        // pointing at the first. The seeded note already says `cs-100`, so Quinn's own edit has to
        // MOVE the field — setting it to what it already is is a no-op and journals nothing.
        write_literals(&v, "tasks/t.md", &[("course".to_string(), "\"cs-101\"".to_string())], &quinn, &mut journal, &WriteOpts::default()).unwrap();
        let real = serde_json::json!({"why": "syllabus"});
        let res = write_literals(&v, "tasks/t.md", &[("course".to_string(), "\"gn-103\"".to_string())], &ctx, &mut journal,
            &WriteOpts { judged: true, propose: true, evidence: Some(&real), inputs: None }).unwrap();
        let body = pystr::read_text(&res.proposal.unwrap()).unwrap();
        assert!(body.contains("Evidence: {\"why\": \"syllabus\"}"), "{body}");
        let _ = std::fs::remove_dir_all(&v);
    }
}
