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
    /// A frontmatter literal carrying a line break, named by its field. Refused before anything is
    /// journalled or written: surgery replaces one line, so a second line would be orphaned.
    LineBreak(String),
    Exists(String),
    Id(IdError),
    Ingest(IngestError),
    Provenance(ProvenanceError),
    Io(String),
    /// Ruling 11: a human write refused because `config/actor.yaml` is not valid, named by the
    /// reader's own line. Refused before anything is journalled or written.
    Actor(String),
    /// M2 D5: a body edit refused because the note's body is not the one the caller last read,
    /// named by the note's path. Nothing is journalled or written.
    Conflict(String),
    /// M2 D4: a body edit refused for a named reason (an unclosed fence, a NUL, a note over the
    /// sync limit, a `---` first line on a note without frontmatter), before any record.
    Body(&'static str),
    /// M2 D10: a list the one-line surgery cannot replace because its value spans lines, named by
    /// its key, before any record.
    MultiLine(String),
}

impl std::fmt::Display for WriteError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            WriteError::NoFrontmatter(p) => write!(f, "{p} has no readable frontmatter"),
            WriteError::AppendLine(why) => write!(f, "append line {why}"),
            WriteError::LineBreak(field) => {
                write!(f, "{field}: a frontmatter value must not contain a line break")
            }
            WriteError::Exists(p) => write!(f, "{p} exists"),
            WriteError::Id(e) => write!(f, "{e}"),
            WriteError::Ingest(e) => write!(f, "{e}"),
            WriteError::Provenance(e) => write!(f, "{e}"),
            WriteError::Io(m) => write!(f, "{m}"),
            WriteError::Actor(m) => write!(f, "{m}"),
            WriteError::Conflict(p) => {
                write!(f, "{p} changed since it was read; nothing was written")
            }
            WriteError::Body(why) => write!(f, "the body was not written: {why}"),
            WriteError::MultiLine(key) => {
                write!(f, "{key}: Knowlu can only edit a list written on one line")
            }
        }
    }
}

/// M2 §7.2: **the one hash** of a note body, SHA-256 of its UTF-8 bytes in lowercase hex. A
/// `set_body` record's `old` and `new` carry it, so the app and the tests call this and never hash
/// on their own.
pub fn body_sha256(body: &str) -> String {
    let digest = ring::digest::digest(&ring::digest::SHA256, body.as_bytes());
    digest.as_ref().iter().map(|b| format!("{b:02x}")).collect()
}

/// Ruling 11 (plan D3): **the one gate** every public entry point below takes before the journal is
/// touched, so no caller — the CLI, the console, the scaffold, a pass — can write as the student
/// while the vault's `config/actor.yaml` is invalid. An `agent:` or `system:` actor is never gated:
/// `judge`, `sync`, `coursework` and the rank's own passes keep their exit-0 contracts whatever the
/// file says. The file is read on every human write, never cached, so a hand fix takes effect at
/// once.
///
/// Plan Q2: a human actor that is not the vault's own token — `quinn` on a `student` vault,
/// `student` on a legacy one, any name — is refused too, so one stray CLI call can never put a
/// second token into an account (which would flip every later restore's choice).
fn human_gate(vault: &Path, ctx: &WriteContext) -> Result<(), WriteError> {
    if is_agent(&ctx.actor) || ctx.actor.starts_with("system:") {
        return Ok(());
    }
    let token = crate::journal::read_human_actor(vault).map_err(|e| WriteError::Actor(e.to_string()))?;
    if ctx.actor != token {
        return Err(WriteError::Actor(format!(
            "{}: this vault's human is {token}, so a write as {:?} is refused (one account writes one token)",
            crate::journal::ACTOR_FILE,
            ctx.actor
        )));
    }
    Ok(())
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
    human_gate(vault, ctx)?;
    // Every literal is one line, or nothing is written: surgery replaces exactly one line, so a
    // literal carrying a break would orphan its tail (and a tail of `---` would close the block).
    // Only `\n` and `\r` count: surgery splits on `\n` and `read_text` maps `\r` to `\n`. U+2028,
    // U+2029 and U+0085 are not breaks to anything that re-reads a note, so they are written
    // (refusing them would fail a whole coursework or enrich batch on an upstream title).
    if let Some((name, _)) = literals.iter().find(|(_, l)| l.contains(['\n', '\r'])) {
        return Err(WriteError::LineBreak(name.clone()));
    }
    let path = resolve_target(vault, target)?;
    let rel_path = rel(vault, &path);
    let (_, meta) = load(&path, true)?;
    let note_id = get_str(&meta, "id").filter(|s| is_id(s));
    let kind = kind_for(&path, Some(&meta));
    let mut result = WriteResult { path: path.clone(), ..Default::default() };
    // `(name, old, new)` — `propose_amendment` writes both sides into the card's `changes` block,
    // and `approvals::apply_amendment` compares `from` against the note before it applies.
    let mut proposed: Vec<(String, Value, Value)> = Vec::new();
    // Ruling 11: the human records behind `proposed`, so the card names their token, not a person.
    let mut proposed_over: Vec<Record> = Vec::new();

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
                    proposed_over.push(human);
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
                    // D5: the actor the record names, never a person's — on a legacy vault that is
                    // still the legacy token, so the reason's bytes are unchanged.
                    let who = human.get("actor").and_then(|v| v.as_str()).unwrap_or("").to_string();
                    result.skipped.insert(
                        name.clone(),
                        format!("judge-once: {name} set by {who} at {ts}"),
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
                // F5(b): whenever the judged write that produced this card carried the service's
                // judgment id (`enrich`'s `inputs`), the card keeps it too — both keys or
                // neither, read straight off `opts.inputs` rather than re-derived.
                let judgment_ids = opts.inputs.and_then(|m| {
                    Some((get_str(m, "judgment_id")?, get_str(m, "judgment_kind")?))
                });
                let judgment = judgment_ids.as_ref().map(|(id, kind)| (id.as_str(), kind.as_str()));
                // The legacy token if any field's hand-set carries it, as the restore rule reads an
                // account; otherwise the student's.
                let human = crate::journal::pick_human_actor(proposed_over.iter());
                result.proposal = Some(propose_amendment(
                    vault, &path, &meta, &proposed, ctx, journal, opts.evidence, today, judgment, human,
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
    human_gate(vault, ctx)?;
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
            let minted = new_id(&kind_for(&path, Some(&meta)));
            text = apply_frontmatter_fields_to_text(
                &text,
                &[("id".to_string(), minted.clone())],
                rel_path,
            )?;
            meta.insert(Value::String("id".into()), Value::String(minted.clone()));
            minted
        }
    };
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
    human_gate(vault, ctx)?;
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
    human_gate(vault, ctx)?;
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
    human_gate(vault, ctx)?;
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
    human_gate(vault, ctx)?;
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

// ---------------------------------------------------------------------------
// Body replace (M2 §6.1, §7.1)
// ---------------------------------------------------------------------------

const UNCLOSED: &str = "the frontmatter has no closing line";
const SPACED: &str = "the frontmatter's closing line is not exactly ---";

/// M2 D6, **the one normaliser**: `read_text`'s line endings (`\r\n` and a lone `\r` become `\n`),
/// leading and trailing newlines trimmed, one final `\n` when anything is left. The compare-and-swap
/// and the write both use it, so the drawer, the hash and the file agree on one string.
fn normalise_body(raw: &str) -> String {
    let text = pystr::universal_newlines(raw);
    let trimmed = text.trim_matches('\n');
    if trimmed.is_empty() { String::new() } else { format!("{trimmed}\n") }
}

/// M2 §6.1: `(head, run, body)` of a note's text as `read_text` gives it. `head` runs through the
/// closing fence's three dashes, `run` is the newlines after them (the fence's own line ending
/// included) and `body` is exactly what `split_frontmatter` returns, i.e. what `note_detail` shows. A
/// text not starting `---` is all body. The closing line is the **reader's** (the first that is
/// `---` once trailing whitespace is stripped), and it must also be the **writer's** (exactly `---`,
/// `apply_frontmatter_fields_to_text`): where the two differ the body the drawer shows is not the
/// text after the fence the surgery finds, so the note is refused, never guessed at.
fn split_body(text: &str) -> Result<(&str, &str, &str), WriteError> {
    if !text.starts_with("---") {
        return Ok(("", "", text));
    }
    let mut start = text.find('\n').map(|i| i + 1).ok_or(WriteError::Body(UNCLOSED))?;
    let close = loop {
        if start >= text.len() {
            return Err(WriteError::Body(UNCLOSED));
        }
        let end = text[start..].find('\n').map_or(text.len(), |i| start + i);
        let line = &text[start..end];
        if line.trim_end() == "---" {
            if line != "---" {
                return Err(WriteError::Body(SPACED));
            }
            break start + 3;
        }
        start = end + 1;
    };
    let rest = &text[close..];
    let body = rest.trim_start_matches('\n');
    Ok((&text[..close], &rest[..rest.len() - body.len()], body))
}

/// A body as a `set_body` record carries it (§6.2, D3): its digest and byte count, never its text.
fn body_digest(body: &str) -> serde_json::Value {
    serde_json::json!({"sha256": body_sha256(body), "bytes": body.len()})
}

/// M2 §7.1: replace a note's body, everything after its closing fence, and nothing before it. The
/// head is carried over as text, never parsed and re-dumped. `expected` is the body the caller last
/// read; `Ok(true)` when it wrote, `Ok(false)` when the new body equals the current one under D6.
/// Each refusal comes before the record, so it leaves the journal and the note untouched. **Journal
/// first, file second**, like every write here.
pub fn set_body(
    vault: &Path,
    target: &str,
    expected: &str,
    new_body: &str,
    ctx: &WriteContext,
    journal: &mut Journal,
) -> Result<bool, WriteError> {
    human_gate(vault, ctx)?;
    let path = resolve_target(vault, target)?;
    let rel_path = rel(vault, &path);
    let (text, meta) = load(&path, false)?;
    let (head, run, current) = split_body(&text)?;
    // D5, compare-and-swap on normalised bodies (equal strings, equal `body_sha256` digests), so a
    // copy that differs from the file only by CRLF or a trailing newline is not a conflict.
    let shown = normalise_body(current);
    if shown != normalise_body(expected) {
        return Err(WriteError::Conflict(rel_path));
    }
    let new = normalise_body(new_body);
    // D6, §7.1 step 4: a save equal to the current body under D6's rule writes nothing, no record
    // and no file (the F4 rule of `write_literals`), and leaves a file not in D6's form as it is.
    if new == shown {
        return Ok(false);
    }
    // D4's refusals, each by name.
    if new.contains('\u{0}') {
        return Err(WriteError::Body("the body contains a NUL character"));
    }
    if head.is_empty() && new.starts_with("---") {
        return Err(WriteError::Body("a note without frontmatter cannot start with ---"));
    }
    // D6: the file's own run of newlines after the fence, or one blank line (the `create_task`
    // shape) when the body was empty and there was none, so the fence always ends its line.
    let run = if !head.is_empty() && current.is_empty() && run.len() < 2 { "\n\n" } else { run };
    let out = format!("{head}{run}{new}");
    // `sync` measures a note after `read_text`, which is `out` exactly: it holds no `\r`.
    if out.len() > crate::sync::MAX_NOTE_BYTES {
        return Err(WriteError::Body("the note would be too large to sync"));
    }

    let note_id = get_str(&meta, "id").filter(|s| is_id(s));
    let mut spec = NewRecord::new("set_body", &rel_path, &ctx.actor, &ctx.via);
    spec.id = note_id.as_deref();
    spec.old = body_digest(current);
    spec.new = body_digest(&new);
    spec.run_id = ctx.run_id.as_deref();
    let mut record = make_record(spec).map_err(|e| WriteError::Io(e.to_string()))?;
    // ---- journal first ----
    journal.append(&mut record).map_err(|e| WriteError::Io(e.to_string()))?;
    // ---- note second ----
    pystr::write_text(&path, &out).map_err(|e| WriteError::Io(e.to_string()))?;
    Ok(true)
}

// ---------------------------------------------------------------------------
// The profile files (M2 §6.3, §7.2, D10, D11)
// ---------------------------------------------------------------------------

/// The names [`create_profile_file`] takes, each the file `profile/<name>.md` (D11).
const PROFILE_FILES: [&str; 2] = ["preferences", "interests"];

/// M2 D11, §7.2: create `profile/<name>.md`, for `name` `preferences` or `interests` only and only
/// when it is absent, holding `text` exactly as given. **Journal first**: one `create` record with
/// a null `id` and `new` the frontmatter mapping `text` holds (`{}` when it has none). File second.
/// It never stamps an id: [`create`] would mint `task_…` (`ids::kind_for`'s default) and refuses a
/// text with no frontmatter. The caller holds the canonical texts (`profile.rs`).
pub fn create_profile_file(
    vault: &Path,
    name: &str,
    text: &str,
    ctx: &WriteContext,
    journal: &mut Journal,
) -> Result<PathBuf, WriteError> {
    human_gate(vault, ctx)?;
    if !PROFILE_FILES.contains(&name) {
        return Err(WriteError::Body("only profile/preferences.md and profile/interests.md can be created"));
    }
    let path = crate::ids::inside_vault(vault, &vault.join("profile").join(format!("{name}.md")))?;
    let rel_path = rel(vault, &path);
    // `symlink_metadata`, not `exists`: a dangling link is present, and writing through it is not
    // creating the file.
    if path.symlink_metadata().is_ok() {
        return Err(WriteError::Exists(rel_path));
    }
    let (meta, _) = split_frontmatter(&pystr::universal_newlines(text))
        .map_err(|_| WriteError::NoFrontmatter(rel_path.clone()))?;
    let mut spec = NewRecord::new("create", &rel_path, &ctx.actor, &ctx.via);
    spec.new = yaml_to_json(&Value::Mapping(meta));
    spec.run_id = ctx.run_id.as_deref();
    let mut record = make_record(spec).map_err(|e| WriteError::Io(e.to_string()))?;
    // ---- journal first ----
    journal.append(&mut record).map_err(|e| WriteError::Io(e.to_string()))?;
    // ---- file second ----
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent).map_err(|e| WriteError::Io(e.to_string()))?;
    }
    pystr::write_text(&path, text).map_err(|e| WriteError::Io(e.to_string()))?;
    Ok(path)
}

/// M2 D10, §7.2: does `key`'s value in the frontmatter of `text` (a note's text; `\r\n` reads as
/// `\n`) continue past its own line, so that the one-line surgery (`apply_frontmatter_fields_to_text`)
/// would replace its first line and orphan the rest? The frontmatter is found by the writer's fence
/// rule: it opens with `---` and closes on the first later line exactly `---`. A text with none has
/// no value to continue, and [`write_one_line_literals`] refuses it by name before any record.
///
/// True when the key's line (the first starting `key:`, as the surgery finds it) is followed by a
/// non-blank line before the next line that starts at column 0 with a character other than `-`, `#`
/// or whitespace, or before the closing fence; and when the key starts two lines. **It errs toward
/// true**: a false true refuses an edit by name, a false false corrupts the file. So it is also true
/// wherever the reader (`split_frontmatter`, which `events::load_interests` uses) and the surgery
/// would disagree about the key: the reader closes on another line or cannot parse the frontmatter,
/// or the key's line read alone does not hold the value the reader reads (a flow list continued at
/// column 0, a key the surgery cannot find, such as a quoted one or one on the opening line).
pub fn value_spans_lines(text: &str, key: &str) -> bool {
    let text = pystr::universal_newlines(text);
    if !text.starts_with("---") {
        return false;
    }
    let lines: Vec<&str> = text.split('\n').collect();
    let close = |rule: fn(&str) -> bool| lines.iter().skip(1).position(|&l| rule(l)).map(|i| i + 1);
    // The writer's closing line and the reader's (`split_frontmatter` strips trailing whitespace).
    let end = match (close(|l| l == "---"), close(|l| l.trim_end() == "---")) {
        (None, None) => return false,
        (Some(writer), Some(reader)) if writer == reader => writer,
        _ => return true,
    };
    let prefix = format!("{key}:");
    let mut found = (1..end).filter(|&i| lines[i].starts_with(&prefix));
    let at = found.next();
    if found.next().is_some() {
        return true;
    }
    if let Some(at) = at {
        for line in &lines[at + 1..end] {
            match line.chars().next() {
                Some(c) if c != '-' && c != '#' && !c.is_whitespace() => break,
                // Blank is YAML's blank: spaces and tabs only, so anything else counts as text.
                _ if line.trim_matches([' ', '\t']).is_empty() => {}
                _ => return true,
            }
        }
    }
    // Read-only parses, never re-dumped: the reader's value for the key against its line alone.
    let Ok((meta, _)) = split_frontmatter(&text) else { return true };
    let alone = match at {
        None => None,
        Some(at) => match serde_yaml_ng::from_str::<Value>(lines[at]) {
            Ok(Value::Mapping(line)) => crate::yaml::get(&line, key).cloned(),
            _ => return true,
        },
    };
    alone.as_ref() != crate::yaml::get(&meta, key)
}

/// The writer's fence rule (`ingest::apply_frontmatter_fields_to_text`): `text`, its newlines
/// already universal, opens with `---` and has a later line exactly `---`.
fn has_writer_frontmatter(text: &str) -> bool {
    text.starts_with("---") && text.split('\n').skip(1).any(|line| line == "---")
}

/// M2 D10, §7.2: [`write_literals`] for lists written on one line. It reads the note once and
/// refuses before any record: with `NoFrontmatter(path)` when the file has no frontmatter by the
/// writer's fence rule, and with `MultiLine(key)` when [`value_spans_lines`] is true for any key in
/// `literals` (one that would not change included), since the surgery replaces one line and would
/// orphan the rest. Otherwise it is exactly `write_literals`, which does not change.
///
/// The first refusal is its own because `write_literals` reads a file with no frontmatter (prose,
/// an unclosed fence, a BOM before the fence) as an empty mapping, as the reader does, journals a
/// record per key, and only then does the surgery refuse: records that name no id, so `verify_tail`
/// cannot heal them, and every retry would add more.
pub fn write_one_line_literals(
    vault: &Path,
    target: &str,
    literals: &[(String, String)],
    ctx: &WriteContext,
    journal: &mut Journal,
    opts: &WriteOpts<'_>,
) -> Result<WriteResult, WriteError> {
    human_gate(vault, ctx)?;
    let path = resolve_target(vault, target)?;
    let text = pystr::read_text(&path).map_err(|e| WriteError::Io(e.to_string()))?;
    if !has_writer_frontmatter(&text) {
        return Err(WriteError::NoFrontmatter(rel(vault, &path)));
    }
    if let Some((key, _)) = literals.iter().find(|(key, _)| value_spans_lines(&text, key)) {
        return Err(WriteError::MultiLine(key.clone()));
    }
    write_literals(vault, target, literals, ctx, journal, opts)
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
/// Called from [`write_literals`] when the journal shows the student set the field and `propose` is
/// on, and from `sync` for a field both desktops moved. It mints one `kind: amend` approval naming
/// every re-judged field, through [`create`] — so the proposal is journalled like any other note
/// and gets its own `id`.
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
    // F5(b): `(judgment_id, judgment_kind)`, whenever the judged write behind this card carried
    // the service's own judgment id — both or neither. `None` on every write with no id (tier 1,
    // a local run, or an older server), which renders the card exactly as before this parameter
    // existed.
    judgment: Option<(&str, &str)>,
    // Ruling 11 (CLAUDE.md rule 1): the human token whose hand-set this card re-proposes over.
    // `journal::LEGACY_HUMAN_ACTOR` keeps the sentence Python wrote, byte for byte (legacy vaults,
    // `scripts/diff-engines-notes.ps1`); any other value reads "the student", never a person.
    human: &str,
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
    let mut front_pairs: Vec<(&str, crate::yamlemit::Node)> = vec![
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
    ];
    // F5(b): after `created_by`, before `changes` — both keys or neither.
    if let Some((judgment_id, judgment_kind)) = judgment {
        front_pairs.push(("judgment_id", crate::yamlemit::Node::text(judgment_id)));
        front_pairs.push(("judgment_kind", crate::yamlemit::Node::text(judgment_kind)));
    }
    front_pairs.push(("changes", crate::yamlemit::Node::Map(change_block)));
    let front = crate::yamlemit::Node::map(front_pairs);
    let hand = if human == crate::journal::LEGACY_HUMAN_ACTOR {
        "Quinn had set them by hand"
    } else {
        "the student had set them by hand"
    };
    let mut why = format!("{} re-judged {fields}; {hand}, so this is a proposal (judge-once rule).", ctx.actor);
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

    // -- dashes and line breaks in a literal --------------------------------

    #[test]
    fn a_title_carrying_dashes_is_quoted_and_the_note_survives_a_second_edit() {
        // `a---b` and `--- x` are legitimate titles. Quoted, each is one line that is not a
        // delimiter line, so the note must still read in full and take a later edit.
        for title in ["a---b", "--- x", "x ---", "---"] {
            let v = vault();
            let path = seed(&v);
            let mut j = Journal::new(&v);
            let ctx = WriteContext::new("quinn", "cli");
            let literal = to_literal(&Value::String(title.to_string()));
            assert_eq!(literal.lines().count(), 1, "{title:?}");
            write_literals(&v, "tasks/a.md", &lit(&[("title", &literal)]), &ctx, &mut j, &WriteOpts::default())
                .unwrap_or_else(|e| panic!("{title:?}: {e}"));

            let (meta, body) = split_frontmatter(&pystr::read_text(&path).unwrap()).unwrap();
            assert_eq!(get_str(&meta, "title").as_deref(), Some(title));
            assert_eq!(get_str(&meta, "status").as_deref(), Some("active"), "{title:?}");
            assert_eq!(get_str(&meta, "id").as_deref(), Some("task_0123456789"), "{title:?}");
            assert_eq!(body, "Body text.\n", "{title:?}");

            // The second edit is what the reviewer's probe broke.
            write_literals(&v, "tasks/a.md", &lit(&[("status", "done")]), &ctx, &mut j, &WriteOpts::default())
                .unwrap_or_else(|e| panic!("second edit after {title:?}: {e}"));
            let (meta, _) = split_frontmatter(&pystr::read_text(&path).unwrap()).unwrap();
            assert_eq!(get_str(&meta, "status").as_deref(), Some("done"), "{title:?}");
            assert_eq!(get_str(&meta, "title").as_deref(), Some(title));
        }
    }

    #[test]
    fn a_literal_carrying_a_line_break_is_refused_and_nothing_is_written() {
        for raw in ["a\nb", "a\rb", "a\r\nb", "\"a\n---\nb\"", "\"a\rb\""] {
            let v = vault();
            let path = seed(&v);
            let before = std::fs::read(&path).unwrap();
            let mut j = Journal::new(&v);
            let ctx = WriteContext::new("quinn", "cli");
            let err = write_literals(&v, "tasks/a.md", &lit(&[("status", "done"), ("title", raw)]), &ctx, &mut j, &WriteOpts::default())
                .expect_err(&format!("{raw:?} must be refused"));
            assert!(err.to_string().contains("title"), "{raw:?}: {err}");
            assert!(err.to_string().contains("line break"), "{raw:?}: {err}");
            assert_eq!(std::fs::read(&path).unwrap(), before, "{raw:?}: the note must be untouched");
            assert!(j.read(None, None).is_empty(), "{raw:?}: no journal record for a refused write");
        }
    }

    #[test]
    fn an_invisible_separator_is_not_a_line_break_to_the_note_and_is_written() {
        // Surgery splits on `\n` and `read_text` maps `\r` to `\n`; nothing that re-reads a note
        // breaks on U+2028/U+2029/U+0085. Refusing them would fail a whole coursework or enrich
        // batch on every run for as long as an upstream title (pasted from a PDF) carries one.
        for (title, reads_back) in [
            ("a\u{2028}b", "a\u{2028}b"),
            ("a\u{2029}b", "a\u{2029}b"),
            // NEL is folded to a space by the YAML reader — lossy, pre-existing, not a break.
            ("a\u{85}b", "a b"),
        ] {
            let v = vault();
            let path = seed(&v);
            let mut j = Journal::new(&v);
            let ctx = WriteContext::new("quinn", "cli");
            let literal = to_literal(&Value::String(title.to_string()));
            write_literals(&v, "tasks/a.md", &lit(&[("title", &literal), ("status", "done")]), &ctx, &mut j, &WriteOpts::default())
                .unwrap_or_else(|e| panic!("{title:?}: {e}"));
            let (meta, body) = split_frontmatter(&pystr::read_text(&path).unwrap()).unwrap();
            assert_eq!(get_str(&meta, "title").as_deref(), Some(reads_back), "{title:?}");
            assert_eq!(get_str(&meta, "status").as_deref(), Some("done"), "{title:?}");
            assert_eq!(body, "Body text.\n", "{title:?}");
            assert_eq!(j.read(None, None).len(), 2, "{title:?}");
            // ...and the note still takes a second edit.
            write_literals(&v, "tasks/a.md", &lit(&[("status", "active")]), &ctx, &mut j, &WriteOpts::default())
                .unwrap_or_else(|e| panic!("second edit after {title:?}: {e}"));
        }
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

    // -- ruling 11: the human token and the gate (D3, D5) -------------------

    fn actor_file(v: &Path, text: &str) {
        std::fs::create_dir_all(v.join("config")).unwrap();
        std::fs::write(v.join("config").join("actor.yaml"), text).unwrap();
    }

    /// Every file under the vault, with its bytes, in path order — the "nothing moved" oracle.
    fn fingerprint(v: &Path) -> Vec<(PathBuf, Vec<u8>)> {
        fn walk(dir: &Path, out: &mut Vec<(PathBuf, Vec<u8>)>) {
            let Ok(entries) = std::fs::read_dir(dir) else { return };
            for entry in entries.flatten() {
                let path = entry.path();
                if path.is_dir() { walk(&path, out) } else { out.push((path.clone(), std::fs::read(&path).unwrap())) }
            }
        }
        let mut out = Vec::new();
        walk(v, &mut out);
        out.sort();
        out
    }

    #[test]
    fn a_human_write_on_a_vault_with_a_bad_actor_file_writes_nothing() {
        for human in ["quinn", "student"] {
            let v = vault();
            seed(&v);
            actor_file(&v, "human_actor: alice\n");
            let before = fingerprint(&v);
            let ctx = WriteContext::new(human, "dashboard");
            let mut j = Journal::new(&v);
            let named = |r: Result<(), WriteError>, what: &str| {
                let err = r.expect_err(&format!("{human}: {what} must be refused")).to_string();
                assert!(err.starts_with("config/actor.yaml: ") && err.contains("\"alice\""), "{human}: {what}: {err}");
            };
            named(write_literals(&v, "tasks/a.md", &lit(&[("status", "done")]), &ctx, &mut j, &WriteOpts::default()).map(|_| ()), "set");
            named(create(&v, "tasks/new.md", "---\ntitle: New\n---\n\nb\n", &ctx, &mut j, None).map(|_| ()), "create");
            named(delete(&v, "tasks/a.md", &ctx, &mut j).map(|_| ()), "delete");
            named(move_note(&v, "tasks/a.md", "tasks/b.md", &ctx, &mut j).map(|_| ()), "move");
            named(append_body(&v, "tasks/a.md", "a line", &ctx, &mut j).map(|_| ()), "append_body");
            named(reassert(&v, "tasks/a.md", "status", &serde_json::json!("done"), &ctx, &mut j).map(|_| ()), "reassert");
            assert_eq!(fingerprint(&v), before, "{human}: the journal and every note are byte-identical");
        }
    }

    #[test]
    fn an_agent_or_system_write_ignores_the_actor_file() {
        let v = vault();
        seed(&v);
        actor_file(&v, "human_actor: alice\n");
        let mut j = Journal::new(&v);
        let agent = WriteContext::new("agent:knowlu.enrich", "local-runner");
        let res = write_literals(&v, "tasks/a.md", &lit(&[("importance", "5")]), &agent, &mut j, &WriteOpts::default()).unwrap();
        assert_eq!(res.records.len(), 1);
        create(&v, "tasks/agent.md", "---\ntitle: Agent\n---\n\nb\n", &agent, &mut j, None).unwrap();
        let system = WriteContext::new("system:idfix", "cli");
        write_literals(&v, "tasks/a.md", &lit(&[("progress", "10")]), &system, &mut j, &WriteOpts::default()).unwrap();
        let actors: Vec<String> = j.read(None, None).iter().map(|r| r["actor"].as_str().unwrap().to_string()).collect();
        assert_eq!(actors, ["agent:knowlu.enrich", "agent:knowlu.enrich", "system:idfix"]);
    }

    #[test]
    fn a_student_vault_takes_student_writes() {
        let v = vault();
        seed(&v);
        actor_file(&v, "human_actor: student\n");
        let mut j = Journal::new(&v);
        let ctx = WriteContext::new("student", "dashboard");
        let res = write_literals(&v, "tasks/a.md", &lit(&[("status", "done")]), &ctx, &mut j, &WriteOpts::default()).unwrap();
        assert_eq!(res.records[0]["actor"], serde_json::json!("student"));
        assert_eq!(j.read(None, None)[0]["actor"], serde_json::json!("student"));
        assert!(pystr::read_text(&v.join("tasks").join("a.md")).unwrap().contains("status: done"));
    }

    /// Plan Q2 (recommended yes): one account writes one token, mechanically. A human actor that is
    /// not the vault's own token is refused by name, before anything is journalled or written.
    #[test]
    fn a_human_actor_other_than_the_vaults_is_refused() {
        for (file, actor, token) in [
            (Some("human_actor: student\n"), "quinn", "student"),
            (None, "student", "quinn"),
            (None, "alice", "quinn"),
            (Some("human_actor: student\n"), "alice", "student"),
            (None, "Quinn", "quinn"),
        ] {
            let v = vault();
            seed(&v);
            if let Some(text) = file { actor_file(&v, text) }
            let before = fingerprint(&v);
            let ctx = WriteContext::new(actor, "cli");
            let mut j = Journal::new(&v);
            let err = write_literals(&v, "tasks/a.md", &lit(&[("status", "done")]), &ctx, &mut j, &WriteOpts::default())
                .expect_err(&format!("{actor} on a {token} vault must be refused"))
                .to_string();
            assert!(err.starts_with("config/actor.yaml: ") && err.contains(token) && err.contains(&format!("{actor:?}")), "{err}");
            assert!(create(&v, "tasks/new.md", "---\ntitle: New\n---\n\nb\n", &ctx, &mut j, None).is_err(), "{actor}: create");
            assert_eq!(fingerprint(&v), before, "{actor} on a {token} vault: nothing written");
        }
    }

    #[test]
    fn judge_once_skip_names_the_recorded_human() {
        for (human, file) in [("quinn", None), ("student", Some("human_actor: student\n"))] {
            let v = vault();
            seed(&v);
            if let Some(text) = file { actor_file(&v, text) }
            let mut j = Journal::new(&v);
            let res = write_literals(&v, "tasks/a.md", &lit(&[("importance", "1")]), &WriteContext::new(human, "dashboard"), &mut j, &WriteOpts::default()).unwrap();
            let ts = res.records[0]["ts"].as_str().unwrap().to_string();
            let agent = WriteContext::new("agent:knowlu.enrich", "local-runner");
            let opts = WriteOpts { judged: true, ..Default::default() };
            let res = write_literals(&v, "tasks/a.md", &lit(&[("importance", "5")]), &agent, &mut j, &opts).unwrap();
            // On a legacy vault these are the exact bytes the reason has always had.
            assert_eq!(res.skipped["importance"], format!("judge-once: importance set by {human} at {ts}"));
        }
    }

    #[test]
    fn an_agent_cannot_overwrite_a_field_student_set() {
        let v = vault();
        seed(&v);
        actor_file(&v, "human_actor: student\n");
        let mut j = Journal::new(&v);
        let student = WriteContext::new("student", "dashboard");
        write_literals(&v, "tasks/a.md", &lit(&[("importance", "1")]), &student, &mut j, &WriteOpts::default()).unwrap();

        let agent = WriteContext::new("agent:routine.enrich", "cloud-routine");
        let opts = WriteOpts { judged: true, ..Default::default() };
        let res = write_literals(&v, "tasks/a.md", &lit(&[("importance", "5")]), &agent, &mut j, &opts).unwrap();

        assert!(res.written.is_empty(), "the agent must not write over the student's decision");
        assert!(res.skipped.get("importance").unwrap().starts_with("judge-once: importance set by student at"));
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

    /// F5(b): when the judged write behind the card carried the service's judgment id (as
    /// `enrich` now passes through `opts.inputs`), the card keeps both `judgment_id` and
    /// `judgment_kind`, placed right after `created_by` and before `changes`.
    #[test]
    fn an_amend_card_from_a_judged_write_carries_judgment_id_and_kind() {
        let v = propose_vault("with-jid");
        let mut journal = Journal::new(&v);
        write_literals(&v, "tasks/t.md", &[("effort_hours".to_string(), "4.0".to_string())],
            &WriteContext::new("quinn", "dashboard"), &mut journal, &WriteOpts::default()).unwrap();

        let mut inputs = Mapping::new();
        inputs.insert(Value::String("judgment_id".into()), Value::String("3fa85f64-5717-4562-b3fc-2c963f66afa6".into()));
        inputs.insert(Value::String("judgment_kind".into()), Value::String("task".into()));
        let res = write_literals(
            &v, "tasks/t.md",
            &[("effort_hours".to_string(), "2.0".to_string())],
            &WriteContext::new("agent:knowlu.enrich", "local-runner"),
            &mut journal,
            &WriteOpts { judged: true, propose: true, inputs: Some(&inputs), ..Default::default() },
        ).unwrap();

        let path = res.proposal.expect("an amendment was filed");
        let text = pystr::read_text(&path).unwrap();
        let (meta, _) = crate::models::split_frontmatter(&text).unwrap();
        assert_eq!(get_str(&meta, "judgment_id").as_deref(), Some("3fa85f64-5717-4562-b3fc-2c963f66afa6"));
        assert_eq!(get_str(&meta, "judgment_kind").as_deref(), Some("task"));

        // Position: right after `created_by`, right before `changes`.
        let lines: Vec<&str> = text.lines().collect();
        let created_by_at = lines.iter().position(|l| l.starts_with("created_by:")).unwrap();
        assert_eq!(lines[created_by_at + 1], "judgment_id: 3fa85f64-5717-4562-b3fc-2c963f66afa6");
        assert_eq!(lines[created_by_at + 2], "judgment_kind: task");
        assert_eq!(lines[created_by_at + 3], "changes:");

        // The two extra keys do not break the card's own contract.
        assert!(crate::approvals::validate_amendment(&v, &meta).is_ok());
        let _ = std::fs::remove_dir_all(&v);
    }

    /// F5(b): a judged write with no id (the common case today — a local run, tier 1, or an
    /// older server) mints a card **byte-identical** to the one before this parameter existed:
    /// same lines, same order, no `judgment_id`/`judgment_kind` anywhere. Compared against the
    /// exact bytes this vault produces, with only the run's own `proposed_at`/`first_proposed_at`
    /// (today's date) and the minted approval id spliced in, since neither is pinnable from here.
    #[test]
    fn an_amend_card_without_one_is_byte_identical() {
        let v = propose_vault("no-jid");
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

        let path = res.proposal.expect("an amendment was filed");
        let text = pystr::read_text(&path).unwrap();
        assert!(!text.contains("judgment_id"), "{text}");
        assert!(!text.contains("judgment_kind"), "{text}");

        let (meta, _) = crate::models::split_frontmatter(&text).unwrap();
        let today = get_str(&meta, "proposed_at").unwrap();
        let id = get_str(&meta, "id").unwrap();
        let expected = format!(
            "---\ntype: approval\nkind: amend\ntitle: Re-proposed effort_hours for CS 100 HW 01\n\
             status: pending\ntarget: tasks/t.md\nproposed_at: {today}\nfirst_proposed_at: {today}\n\
             expires: null\nsnooze_until: null\ncreated_by: agent:knowlu.enrich\nchanges:\n  \
             effort_hours:\n    from: 4.0\n    to: 2.0\nid: {id}\n---\n\n\
             **Why proposed:** agent:knowlu.enrich re-judged effort_hours; Quinn had set them by \
             hand, so this is a proposal (judge-once rule).\n{AMEND_BUTTONS}"
        );
        assert_eq!(text, expected);
        let _ = std::fs::remove_dir_all(&v);
    }

    /// Ruling 11 and CLAUDE.md rule 1: the card's `**Why proposed:**` line follows the recorded
    /// human's token and never names a person on a vault the app makes. A legacy vault (no
    /// `config/actor.yaml`, `quinn` in its journal) keeps the sentence Python wrote, byte for byte;
    /// the whole legacy card is pinned by `an_amend_card_without_one_is_byte_identical` above.
    #[test]
    fn an_amend_cards_why_line_names_no_person_on_a_student_vault() {
        for (file, human, hand) in [
            (None, "quinn", "Quinn had set them by hand"),
            (Some("human_actor: student\n"), "student", "the student had set them by hand"),
        ] {
            let v = propose_vault(&format!("why-{human}"));
            if let Some(text) = file { actor_file(&v, text) }
            let mut journal = Journal::new(&v);
            write_literals(&v, "tasks/t.md", &[("effort_hours".to_string(), "4.0".to_string())],
                &WriteContext::new(human, "dashboard"), &mut journal, &WriteOpts::default()).unwrap();
            let res = write_literals(
                &v, "tasks/t.md",
                &[("effort_hours".to_string(), "2.0".to_string())],
                &WriteContext::new("agent:knowlu.enrich", "local-runner"),
                &mut journal,
                &WriteOpts { judged: true, propose: true, ..Default::default() },
            ).unwrap();
            let text = pystr::read_text(&res.proposal.expect("an amendment was filed")).unwrap();
            let why = format!(
                "\n**Why proposed:** agent:knowlu.enrich re-judged effort_hours; {hand}, so this is a \
                 proposal (judge-once rule).\n"
            );
            assert!(text.contains(&why), "{human}: {text}");
            if human == "student" {
                assert!(!text.to_lowercase().contains("quinn"), "no person's name on a student's card: {text}");
            }
            let _ = std::fs::remove_dir_all(&v);
        }
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

    // -- M2: the body hash and the new refusals -----------------------------

    #[test]
    fn body_sha256_is_lowercase_hex_of_the_utf8_bytes() {
        // The empty body: §6.2's example `old`, and every SHA-256 implementation's first vector.
        assert_eq!(body_sha256(""), "e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855");
        // Non-ASCII: the digest covers the UTF-8 bytes (63 61 66 c3 a9 20 e2 80 94 20 e6 97 a5
        // e6 9c ac 0a), checked against `sha256sum` and Python's `hashlib` over those bytes.
        let body = "caf\u{e9} \u{2014} \u{65e5}\u{672c}\n";
        assert_eq!(body_sha256(body), "954d89d459ee5f2d1a8902b9d7bfe745cb84445dfc5228c168ced2bb35cab546");
        for digest in [body_sha256(""), body_sha256(body)] {
            assert_eq!(digest.len(), 64, "{digest}");
            assert!(digest.bytes().all(|b| b.is_ascii_digit() || (b'a'..=b'f').contains(&b)), "{digest}");
        }
    }

    #[test]
    fn the_new_write_errors_name_the_file_or_the_reason() {
        let conflict = WriteError::Conflict("tasks/stats-hw-4.md".into()).to_string();
        assert!(conflict.contains("tasks/stats-hw-4.md"), "{conflict}");
        let body = WriteError::Body("the frontmatter has no closing line").to_string();
        assert!(body.contains("the frontmatter has no closing line"), "{body}");
        let multi = WriteError::MultiLine("strong".into()).to_string();
        assert!(multi.contains("Knowlu can only edit a list written on one line"), "{multi}");
        assert!(multi.contains("strong"), "{multi}");
    }

    // -- M2: set_body (T1a.2) ------------------------------------------------

    /// A vault made after ruling 11 and its human's context. The token comes from the constant,
    /// never a literal (plan §5), so these tests hold on both sides of the token.
    fn human_ctx(v: &Path) -> WriteContext {
        crate::journal::create_actor_file(v, crate::journal::HUMAN_ACTOR).unwrap();
        WriteContext::new(crate::journal::HUMAN_ACTOR, "dashboard")
    }

    /// The body as the drawer shows it: `surface::note_detail`'s, which is the reader's split.
    fn shown_body(path: &Path) -> String {
        split_frontmatter(&pystr::read_text(path).unwrap()).unwrap().1
    }

    fn body_records(j: &mut Journal) -> Vec<Record> {
        j.read(None, None).into_iter().filter(|r| r["op"] == "set_body").collect()
    }

    #[test]
    fn set_body_replaces_only_the_body_and_keeps_the_head_bytes() {
        let v = vault();
        let ctx = human_ctx(&v);
        let head = "---\ntitle: \"Stats HW 4\"\nstatus: active\njudgment: {by: \"agent:knowlu.enrich\", at: \"2026-09-29T10:00:00Z\", fields: [effort_hours]}\nid: task_0123456789\n---\n";
        let path = v.join("tasks").join("a.md");
        // `write_text` writes the platform newline, so on Windows this is a CRLF note, as vaults are.
        pystr::write_text(&path, &format!("{head}\nOld notes.\n")).unwrap();
        let head_bytes = head.replace('\n', pystr::NEWLINE).into_bytes();
        assert!(std::fs::read(&path).unwrap().starts_with(&head_bytes));
        let (meta, _) = split_frontmatter(&pystr::read_text(&path).unwrap()).unwrap();
        let mut j = Journal::new(&v);

        let wrote = set_body(&v, "task_0123456789", "Old notes.\n", "Read ch. 4\r\nthen pp. 12-14", &ctx, &mut j).unwrap();
        assert!(wrote);
        let after = std::fs::read(&path).unwrap();
        assert_eq!(&after[..head_bytes.len()], &head_bytes[..], "the head, through the fence's line ending, is byte-identical");
        let (meta_after, body) = split_frontmatter(&pystr::read_text(&path).unwrap()).unwrap();
        assert_eq!(meta_after, meta);
        assert_eq!(body, "Read ch. 4\nthen pp. 12-14\n", "the body reads back as the normalised new body");
        assert_eq!(pystr::read_text(&path).unwrap(), format!("{head}\nRead ch. 4\nthen pp. 12-14\n"), "the blank line after the fence is kept");
        assert_eq!(body_records(&mut j).len(), 1);
    }

    #[test]
    fn set_body_journals_before_it_writes() {
        // Mirrors `the_journal_record_is_written_before_the_note`: the file write fails (the note is
        // read-only, which every earlier step tolerates), and the record is already in the journal.
        let v = vault();
        let ctx = human_ctx(&v);
        let path = seed(&v);
        let before = std::fs::read(&path).unwrap();
        let mut perms = std::fs::metadata(&path).unwrap().permissions();
        perms.set_readonly(true);
        std::fs::set_permissions(&path, perms.clone()).unwrap();
        let mut j = Journal::new(&v);
        let res = set_body(&v, "tasks/a.md", "Body text.\n", "New text.\n", &ctx, &mut j);
        perms.set_readonly(false);
        std::fs::set_permissions(&path, perms).unwrap();

        assert!(matches!(res, Err(WriteError::Io(_))), "the file write must fail for this test to mean anything: {res:?}");
        assert_eq!(body_records(&mut j).len(), 1, "journal-first was violated");
        assert_eq!(std::fs::read(&path).unwrap(), before, "the note is unchanged");
    }

    #[test]
    fn set_body_refuses_a_stale_expected_body() {
        let v = vault();
        let ctx = human_ctx(&v);
        let path = seed(&v);
        let read = shown_body(&path);
        // An agent appends a line after the drawer read the body, so the drawer's copy is stale.
        let mut j = Journal::new(&v);
        append_body(&v, "tasks/a.md", "an agent's line", &WriteContext::new("agent:knowlu.enrich", "local-runner"), &mut j).unwrap();
        let before = fingerprint(&v);

        let err = set_body(&v, "tasks/a.md", &read, "Mine.\n", &ctx, &mut j).unwrap_err();
        assert_eq!(err, WriteError::Conflict("tasks/a.md".into()));
        assert!(body_records(&mut j).is_empty(), "no record");
        assert_eq!(fingerprint(&v), before, "the journal and the note are byte-identical");
    }

    #[test]
    fn a_body_holding_a_dash_rule_keeps_the_frontmatter() {
        let v = vault();
        let ctx = human_ctx(&v);
        let path = seed(&v);
        let (meta, _) = split_frontmatter(&pystr::read_text(&path).unwrap()).unwrap();
        let mut j = Journal::new(&v);
        let new = "---\nPart one\n---\n--- \ntitle: not a field\n---\n";

        assert!(set_body(&v, "tasks/a.md", "Body text.\n", new, &ctx, &mut j).unwrap());
        let (meta_after, body) = split_frontmatter(&pystr::read_text(&path).unwrap()).unwrap();
        assert_eq!(meta_after, meta, "the frontmatter mapping is unchanged");
        assert_eq!(body, new, "the body reads back whole");
        // ...and the note still takes a second body edit and a field edit.
        assert!(set_body(&v, "tasks/a.md", new, "After.\n", &ctx, &mut j).unwrap());
        write_literals(&v, "tasks/a.md", &lit(&[("status", "\"done\"")]), &ctx, &mut j, &WriteOpts::default()).unwrap();
        assert_eq!(shown_body(&path), "After.\n");
        assert_eq!(get_str(&split_frontmatter(&pystr::read_text(&path).unwrap()).unwrap().0, "status").as_deref(), Some("done"));
    }

    #[test]
    fn set_body_on_a_file_without_frontmatter_replaces_the_whole_text() {
        let v = vault();
        let ctx = human_ctx(&v);
        std::fs::create_dir_all(v.join("profile")).unwrap();
        let path = v.join("profile").join("preferences.md");
        pystr::write_text(&path, "\nMornings are best.\n\nNo work after 9pm.\n").unwrap();
        let mut j = Journal::new(&v);

        assert!(set_body(&v, "profile/preferences.md", &shown_body(&path), "Evenings now.\r\n\r\n", &ctx, &mut j).unwrap());
        assert_eq!(pystr::read_text(&path).unwrap(), "Evenings now.\n", "the whole text is the body");
        let records = body_records(&mut j);
        assert_eq!(records.len(), 1);
        assert_eq!(records[0]["id"], serde_json::Value::Null, "a file with no id records a null id");

        // A first line starting `---` would turn the body into frontmatter on the next read.
        let before = fingerprint(&v);
        for new in ["---\nstrong: [x]\n---\n", "\n\n---", "--- not a rule"] {
            let err = set_body(&v, "profile/preferences.md", "Evenings now.\n", new, &ctx, &mut j).unwrap_err();
            assert!(matches!(err, WriteError::Body(_)), "{new:?}: {err}");
        }
        assert_eq!(fingerprint(&v), before, "refused before any record");
    }

    #[test]
    fn set_body_refuses_an_unclosed_fence_and_a_space_trailed_one() {
        let unclosed = "the frontmatter has no closing line";
        let spaced = "the frontmatter's closing line is not exactly ---";
        for (text, why) in [
            ("---\nid: task_0123456789\n\nno closing line\n", unclosed),
            ("---", unclosed),
            ("---\nid: task_0123456789\n--- \n\nBody\n", spaced),
            // The reader closes on `---\t`; the writer would close on the body's rule below it.
            ("---\nid: task_0123456789\n---\t\nBody\n---\nMore\n", spaced),
        ] {
            let v = vault();
            let ctx = human_ctx(&v);
            let path = v.join("tasks").join("a.md");
            pystr::write_text(&path, text).unwrap();
            let before = fingerprint(&v);
            let mut j = Journal::new(&v);
            let err = set_body(&v, "tasks/a.md", &shown_body(&path), "New.\n", &ctx, &mut j).unwrap_err();
            assert_eq!(err, WriteError::Body(why), "{text:?}");
            assert_eq!(fingerprint(&v), before, "{text:?}: nothing journalled or written");
        }
    }

    #[test]
    fn set_body_refuses_nul_and_a_note_over_the_sync_limit() {
        let v = vault();
        let ctx = human_ctx(&v);
        let path = seed(&v);
        let before = fingerprint(&v);
        let mut j = Journal::new(&v);
        let limit = crate::sync::MAX_NOTE_BYTES;
        // Everything but the body, as `sync` measures a note (after `read_text`).
        let head = NOTE.len() - "Body text.\n".len();
        let over = format!("{}\n", "x".repeat(limit - head));
        for (new, why) in [
            ("a\u{0}b", "the body contains a NUL character"),
            (over.as_str(), "the note would be too large to sync"),
        ] {
            let err = set_body(&v, "tasks/a.md", "Body text.\n", new, &ctx, &mut j).unwrap_err();
            assert_eq!(err, WriteError::Body(why));
        }
        assert_eq!(fingerprint(&v), before, "nothing journalled or written");
        // Exactly the limit is sendable, so it is written.
        let at = format!("{}\n", "x".repeat(limit - head - 1));
        assert!(set_body(&v, "tasks/a.md", "Body text.\n", &at, &ctx, &mut j).unwrap());
        assert_eq!(pystr::read_text(&path).unwrap().len(), limit);
    }

    #[test]
    fn set_body_normalises_by_d6() {
        for (raw, want) in [
            ("", ""),
            ("\n\r\n\r", ""),
            ("a", "a\n"),
            ("a\r\nb\rc\n\n\n", "a\nb\nc\n"),
            ("\n\n  indented\n", "  indented\n"),
            ("x\n  \n", "x\n  \n"),
            // Not a newline to `read_text`, so not one here: the drawer and the hash must agree.
            ("one\u{2028}line", "one\u{2028}line\n"),
        ] {
            assert_eq!(normalise_body(raw), want, "{raw:?}");
        }
    }

    #[test]
    fn set_body_takes_the_human_gate() {
        use crate::journal::{HUMAN_ACTOR, LEGACY_HUMAN_ACTOR};
        for (file, actor) in [
            (Some("human_actor: alice\n".to_string()), HUMAN_ACTOR),
            (Some(format!("human_actor: {HUMAN_ACTOR}\n")), LEGACY_HUMAN_ACTOR),
            (None, HUMAN_ACTOR),
        ] {
            let v = vault();
            seed(&v);
            if let Some(text) = &file { actor_file(&v, text) }
            let before = fingerprint(&v);
            let mut j = Journal::new(&v);
            let err = set_body(&v, "tasks/a.md", "Body text.\n", "New.\n", &WriteContext::new(actor, "dashboard"), &mut j).unwrap_err();
            assert!(matches!(err, WriteError::Actor(_)), "{actor} with {file:?}: {err}");
            assert_eq!(fingerprint(&v), before, "{actor} with {file:?}: nothing journalled or written");
        }
        // An agent is never gated, whatever the file says.
        let v = vault();
        let path = seed(&v);
        actor_file(&v, "human_actor: alice\n");
        let mut j = Journal::new(&v);
        let agent = WriteContext::new("agent:knowlu.enrich", "local-runner");
        assert!(set_body(&v, "tasks/a.md", "Body text.\n", "Agent notes.\n", &agent, &mut j).unwrap());
        assert_eq!(body_records(&mut j)[0]["actor"], "agent:knowlu.enrich");
        assert_eq!(shown_body(&path), "Agent notes.\n");
    }

    /// D6's parenthesis: an empty body with no blank line after the fence (or no line ending at all)
    /// gets the `create_task` shape, so the fence still ends its line; a note with a body keeps its
    /// own run of newlines.
    #[test]
    fn set_body_keeps_the_fence_a_whole_line_and_the_notes_own_separator() {
        let fm = "---\nid: task_0123456789\n---";
        for (text, want) in [
            (fm.to_string(), format!("{fm}\n\nNew\n")),
            (format!("{fm}\n"), format!("{fm}\n\nNew\n")),
            (format!("{fm}\n\n\n"), format!("{fm}\n\n\nNew\n")),
            (format!("{fm}\nOld\n"), format!("{fm}\nNew\n")),
            (format!("{fm}\n\n\nOld\n"), format!("{fm}\n\n\nNew\n")),
        ] {
            let v = vault();
            let ctx = human_ctx(&v);
            let path = v.join("tasks").join("a.md");
            pystr::write_text(&path, &text).unwrap();
            let mut j = Journal::new(&v);
            assert!(set_body(&v, "tasks/a.md", &shown_body(&path), "New", &ctx, &mut j).unwrap(), "{text:?}");
            assert_eq!(pystr::read_text(&path).unwrap(), want, "{text:?}");
            assert_eq!(shown_body(&path), "New\n", "{text:?}");
        }
    }

    // -- M2: set_body's no-op, and the pins (T1a.3) --------------------------

    /// The seeded task's body as `surface::note_detail` gives it to the drawer (D6's one string).
    fn detail_body(v: &Path, j: &mut Journal) -> String {
        let day = jiff::civil::Date::constant(2026, 9, 30);
        crate::surface::note_detail(v, "task_0123456789", day, j).expect("the seeded task").body
    }

    #[test]
    fn set_body_is_a_no_op_when_the_body_is_unchanged() {
        let v = vault();
        let ctx = human_ctx(&v);
        let path = seed(&v);
        let mut j = Journal::new(&v);
        let before = fingerprint(&v);
        for same in ["Body text.\r\n", "Body text.", "Body text.\n\n\n", "\n\nBody text.\n", "\r\n\r\nBody text.\r\n\r\n", "\rBody text.\r"] {
            assert_eq!(set_body(&v, "tasks/a.md", "Body text.\n", same, &ctx, &mut j), Ok(false), "{same:?}");
            // The caller's copy may differ from the file the same ways (§7.1 step 3).
            assert_eq!(set_body(&v, "tasks/a.md", same, "Body text.\n", &ctx, &mut j), Ok(false), "{same:?}");
        }
        // The no-op comes after the compare-and-swap: a stale copy is still a conflict.
        assert_eq!(set_body(&v, "tasks/a.md", "Stale.\n", "Body text.\n", &ctx, &mut j), Err(WriteError::Conflict("tasks/a.md".into())));
        assert!(body_records(&mut j).is_empty(), "nothing is journalled");
        assert_eq!(fingerprint(&v), before, "the journal and the note are byte-identical");

        // A file whose own text is not in D6's form is left in it, never rewritten into it.
        for text in [
            NOTE.replace("---\n\nBody text.\n", "---\n\n\n\nBody text.\n\n\n"),
            "---\nid: task_0123456789\n---".to_string(),
        ] {
            pystr::write_text(&path, &text).unwrap();
            let before = fingerprint(&v);
            let shown = shown_body(&path);
            assert_eq!(set_body(&v, "tasks/a.md", shown.trim_end(), &format!("\r\n{shown}\n\n"), &ctx, &mut j), Ok(false), "{text:?}");
            assert_eq!(fingerprint(&v), before, "{text:?}");
        }
    }

    #[test]
    fn set_body_record_holds_hashes_and_never_text() {
        let v = vault();
        let ctx = human_ctx(&v);
        let path = v.join("tasks").join("a.md");
        let old = "Old notes: marker-OLD-4e1c, caf\u{e9} \u{2014} \u{65e5}\u{672c}.\n";
        pystr::write_text(&path, &NOTE.replace("Body text.\n", old)).unwrap();
        let new = "New notes: marker-NEW-9b7d\r\nsecond line";
        let mut j = Journal::new(&v);
        assert!(set_body(&v, "tasks/a.md", old, new, &ctx, &mut j).unwrap());

        let records = body_records(&mut j);
        assert_eq!(records.len(), 1);
        let rec = &records[0];
        // §6.2: `make_record`'s twelve keys plus `seq`, stamped by `Journal::append`.
        let mut keys: Vec<&str> = rec.keys().map(String::as_str).collect();
        keys.sort_unstable();
        assert_eq!(keys, ["actor", "device", "evidence", "field", "id", "new", "old", "op", "path", "run_id", "seq", "ts", "via"]);
        assert_eq!(rec["actor"], ctx.actor.as_str(), "the test's own context, never §6.2's example");
        assert_eq!(rec["via"], ctx.via.as_str());
        assert_eq!(rec["op"], "set_body");
        assert_eq!(rec["id"], "task_0123456789");
        assert_eq!(rec["path"], "tasks/a.md");
        for key in ["run_id", "field", "evidence"] {
            assert_eq!(rec[key], serde_json::Value::Null, "{key}");
        }
        assert!(rec["ts"].as_str().is_some_and(|ts| ts.ends_with('Z')) && rec["device"].is_string() && rec["seq"].is_u64(), "{rec:?}");
        let norm = "New notes: marker-NEW-9b7d\nsecond line\n";
        assert_eq!(rec["old"], serde_json::json!({"sha256": body_sha256(old), "bytes": old.len()}), "UTF-8 bytes, not chars");
        assert_eq!(rec["new"], serde_json::json!({"sha256": body_sha256(norm), "bytes": norm.len()}));

        // On disk: one line, `dumps_value`'s bytes, and no text of either body in any form.
        let files: Vec<PathBuf> = std::fs::read_dir(v.join("state").join("journal")).unwrap().map(|e| e.unwrap().path()).collect();
        assert_eq!(files.len(), 1);
        let raw = std::fs::read_to_string(&files[0]).unwrap();
        assert_eq!(raw, format!("{}{}", crate::ledger::dumps_value(&serde_json::Value::Object(rec.clone())), pystr::NEWLINE));
        for marker in ["marker-OLD-4e1c", "marker-NEW-9b7d", "Old notes", "New notes", "second line", "caf\u{e9}", "\u{65e5}\u{672c}"] {
            assert!(!raw.contains(marker), "{marker:?} reached the journal: {raw}");
        }
    }

    #[test]
    fn set_body_hashes_chain() {
        let v = vault();
        let ctx = human_ctx(&v);
        seed(&v);
        let mut j = Journal::new(&v);
        let digest = |body: &str| serde_json::json!({"sha256": body_sha256(body), "bytes": body.len()});
        let b0 = detail_body(&v, &mut j);
        assert!(set_body(&v, "task_0123456789", &b0, "\r\n\r\nFirst edit\r\nline two", &ctx, &mut j).unwrap());
        let b1 = detail_body(&v, &mut j);
        assert!(set_body(&v, "task_0123456789", &b1, "Second edit.\n\n\n", &ctx, &mut j).unwrap());
        let b2 = detail_body(&v, &mut j);
        assert_eq!((b1.as_str(), b2.as_str()), ("First edit\nline two\n", "Second edit.\n"));

        let records = body_records(&mut j);
        assert_eq!(records.len(), 2);
        assert_eq!(records[0]["old"], digest(&b0), "the first `old` is the body the drawer showed");
        assert_eq!(records[0]["new"], digest(&b1), "`new` is the body `note_detail` then shows");
        assert_eq!(records[1]["old"], records[0]["new"], "a second edit's `old` is the first's `new`");
        assert_eq!(records[1]["new"], digest(&b2));
    }

    #[test]
    fn an_emptied_body_keeps_the_blank_line_after_the_fence() {
        let v = vault();
        let ctx = human_ctx(&v);
        let mut j = Journal::new(&v);
        // `create_task_inner`'s shape: the frontmatter, its fence, one blank line and no body.
        let path = create(&v, "tasks/new.md", "---\ntitle: New task\nstatus: active\n---\n\n", &ctx, &mut j, None).unwrap();
        let minted = std::fs::read(&path).unwrap();
        let minted_text = pystr::read_text(&path).unwrap();
        assert!(minted_text.ends_with("\n---\n\n"), "{minted_text:?}");
        for empty in ["", "\n", "\r\n\r\n"] {
            assert!(set_body(&v, "tasks/new.md", &shown_body(&path), "Notes.", &ctx, &mut j).unwrap());
            assert_eq!(pystr::read_text(&path).unwrap(), format!("{minted_text}Notes.\n"));
            assert!(set_body(&v, "tasks/new.md", "Notes.\n", empty, &ctx, &mut j).unwrap(), "{empty:?}");
            assert_eq!(std::fs::read(&path).unwrap(), minted, "{empty:?}: the create_task shape, byte for byte");
            assert_eq!(shown_body(&path), "", "{empty:?}");
        }
        // A note that had a body from the start keeps its blank line too.
        let seeded = seed(&v);
        assert!(set_body(&v, "tasks/a.md", "Body text.\n", "", &ctx, &mut j).unwrap());
        assert_eq!(pystr::read_text(&seeded).unwrap(), NOTE.replace("Body text.\n", ""));
    }

    #[test]
    fn set_body_records_are_invisible_to_judge_once_and_verify_tail() {
        let v = vault();
        let ctx = human_ctx(&v);
        let path = seed(&v);
        let mut j = Journal::new(&v);
        assert!(set_body(&v, "tasks/a.md", "Body text.\n", "My own notes.\n", &ctx, &mut j).unwrap());
        assert_eq!(body_records(&mut j)[0]["actor"], ctx.actor.as_str(), "a human record, so a reader that took it would freeze a field");
        // `sha256` and `bytes` are the keys of the record's `new`: a reader treating it as a `create`
        // would find them.
        for field in ["body", "sha256", "bytes", "status", "effort_hours"] {
            assert!(j.human_set("task_0123456789", field).is_none(), "human_set {field}");
            assert!(j.human_edited("task_0123456789", field).is_none(), "human_edited {field}");
        }
        assert!(crate::journal::latest_by_field(&j.read(None, None)).is_empty());
        // The file back at the old body, as a crash between record and file would leave it (§7.5):
        // `verify_tail` re-applies nothing and writes nothing.
        pystr::write_text(&path, NOTE).unwrap();
        let before = fingerprint(&v);
        assert_eq!(crate::passes::verify_tail(&v, &mut j, &ctx, 1000), Vec::<String>::new());
        assert_eq!(fingerprint(&v), before);
    }

    #[test]
    fn set_body_undo_succeeds_after_a_save_without_a_trailing_newline() {
        let v = vault();
        let ctx = human_ctx(&v);
        let path = seed(&v);
        let seeded = std::fs::read(&path).unwrap();
        let mut j = Journal::new(&v);
        let original = detail_body(&v, &mut j);
        let typed = "Edited, with no final newline";
        // D8: save, then undo with the body the page re-read as `expected` and the old text as new.
        assert!(set_body(&v, "task_0123456789", &original, typed, &ctx, &mut j).unwrap());
        let reread = detail_body(&v, &mut j);
        assert_eq!(reread, format!("{typed}\n"));
        assert_eq!(set_body(&v, "task_0123456789", &reread, &original, &ctx, &mut j), Ok(true));
        assert_eq!(detail_body(&v, &mut j), original, "the original body is back");
        assert_eq!(std::fs::read(&path).unwrap(), seeded, "byte for byte");
        let r = body_records(&mut j);
        assert_eq!((&r[1]["old"], &r[1]["new"]), (&r[0]["new"], &r[0]["old"]), "the undo's record chains back");
        // §7.1 step 3: the textarea's own text, with no final newline or with CRLF, is no conflict.
        for expected in [typed.to_string(), format!("{typed}\r\n")] {
            assert!(set_body(&v, "task_0123456789", &original, typed, &ctx, &mut j).unwrap());
            assert_eq!(set_body(&v, "task_0123456789", &expected, &original, &ctx, &mut j), Ok(true), "{expected:?}");
            assert_eq!(detail_body(&v, &mut j), original, "{expected:?}");
        }
        assert_eq!(body_records(&mut j).len(), 6);
    }

    // -- M2: the profile-file primitives (T1b) -------------------------------

    /// `profile/interests.md` as Knowlu creates it (§6.3); T3's `profile.rs` holds the canonical text.
    const INTERESTS: &str = "---\nstrong: []\nmild: []\nnever: []\nclubs: []\n---\n";

    #[test]
    fn create_profile_file_makes_only_the_two_files_once_and_stamps_no_id() {
        // The `create` record comes first: with a file where the `profile/` folder goes, the write
        // fails after the record is in the journal.
        let v = vault();
        let ctx = human_ctx(&v);
        let mut j = Journal::new(&v);
        std::fs::write(v.join("profile"), "").unwrap();
        let res = create_profile_file(&v, "interests", INTERESTS, &ctx, &mut j);
        assert!(matches!(res, Err(WriteError::Io(_))), "the file write must fail for this test to mean anything: {res:?}");
        assert_eq!(j.read(None, None).len(), 1, "journal-first was violated");

        let v = vault();
        let ctx = human_ctx(&v);
        let mut j = Journal::new(&v);
        let before = fingerprint(&v);
        for name in ["tasks", "task", "preferences.md", "Interests", "../interests", "profile/interests", ""] {
            let err = create_profile_file(&v, name, INTERESTS, &ctx, &mut j).unwrap_err();
            assert!(matches!(err, WriteError::Body(_)), "{name:?}: {err}");
        }
        assert_eq!(fingerprint(&v), before, "another name: nothing journalled or written");

        let prefs = "Mornings are best.\nNo work after 9pm.\n";
        let interests = create_profile_file(&v, "interests", INTERESTS, &ctx, &mut j).unwrap();
        let preferences = create_profile_file(&v, "preferences", prefs, &ctx, &mut j).unwrap();
        assert_eq!(interests, v.join("profile").join("interests.md"));
        assert_eq!(preferences, v.join("profile").join("preferences.md"));
        assert_eq!(pystr::read_text(&interests).unwrap(), INTERESTS, "written as given: no id stamped");
        assert_eq!(pystr::read_text(&preferences).unwrap(), prefs, "written as given: no frontmatter added");
        let records = j.read(None, None);
        assert_eq!(records.len(), 2);
        for (rec, (path, new)) in records.iter().zip([
            ("profile/interests.md", serde_json::json!({"strong": [], "mild": [], "never": [], "clubs": []})),
            ("profile/preferences.md", serde_json::json!({})),
        ]) {
            assert_eq!(rec["op"], "create", "{rec:?}");
            assert_eq!(rec["id"], serde_json::Value::Null, "{rec:?}");
            assert_eq!(rec["path"], path, "{rec:?}");
            assert_eq!(rec["new"], new, "the frontmatter mapping, or {{}} when there is none");
            assert_eq!(rec["actor"], ctx.actor.as_str(), "{rec:?}");
        }
        // The event filter's own reader reads the new file as four empty lists.
        assert_eq!(crate::events::load_interests(&interests), (crate::events::Interests::default(), Vec::new()));

        // Once: an existing file is refused by name, and nothing more is journalled or written.
        let before = fingerprint(&v);
        for (name, text) in [("interests", "---\nstrong: [research]\n---\n"), ("preferences", "Evenings.\n")] {
            assert_eq!(create_profile_file(&v, name, text, &ctx, &mut j), Err(WriteError::Exists(format!("profile/{name}.md"))));
        }
        assert_eq!(fingerprint(&v), before);
    }

    /// The four keys in the order `profile::set_interests` sends them (§7.6).
    fn four(values: [&str; 4]) -> Vec<(String, String)> {
        lit(&[("strong", values[0]), ("mild", values[1]), ("never", values[2]), ("clubs", values[3])])
    }

    /// `profile/interests.md` holding `text` in the vault at `v`, and the vault's human context.
    fn interests_vault(v: &Path, text: &str) -> (PathBuf, WriteContext) {
        let ctx = human_ctx(v);
        std::fs::create_dir_all(v.join("profile")).unwrap();
        let path = v.join("profile").join("interests.md");
        pystr::write_text(&path, text).unwrap();
        (path, ctx)
    }

    #[test]
    fn write_one_line_literals_refuses_a_value_that_spans_lines() {
        for (why, text, key) in [
            ("a block list, as the fixture writes it", "---\nstrong:\n  - research\n  - AI talks\nmild: []\nnever: []\nclubs: []\n---\n", "strong"),
            ("a list at column 0", "---\nstrong: []\nmild:\n- free food\nnever: []\nclubs: []\n---\n", "mild"),
            ("a comment line after the key", "---\nstrong: []\nmild: []\nnever:\n# none of these\n  - recruitment\nclubs: []\n---\n", "never"),
            ("trailing spaces after the key, then items", "---\nstrong:   \n  - research\nmild: []\nnever: []\nclubs: []\n---\n", "strong"),
            ("a blank line inside the block", "---\nstrong: []\nmild: []\nnever: []\nclubs:\n\n  - AI Club\n\n  - Chess Club\n---\n", "clubs"),
            ("a duplicated key", "---\nstrong: [research]\nmild: []\nnever: []\nstrong: [talks]\nclubs: []\n---\n", "strong"),
        ] {
            let v = vault();
            let (_, ctx) = interests_vault(&v, text);
            let before = fingerprint(&v);
            let mut j = Journal::new(&v);
            let err = write_one_line_literals(&v, "profile/interests.md", &four(["[x]", "[y]", "[z]", "[w]"]), &ctx, &mut j, &WriteOpts::default()).unwrap_err();
            assert_eq!(err, WriteError::MultiLine(key.to_string()), "{why}");
            assert!(err.to_string().contains("Knowlu can only edit a list written on one line"), "{why}: {err}");
            assert_eq!(fingerprint(&v), before, "{why}: no record, and the bytes are left alone");
        }

        // No frontmatter by the writer's fence rule: the reader reads four empty lists, so no key
        // spans lines, yet `write_literals` would journal a record per key before the surgery refused
        // (records with no id, which `verify_tail` cannot heal). Refused by name before any record.
        for (why, text) in [
            ("prose, which the event filter reads", "I like AI talks.\n"),
            ("an unclosed fence", "---\nstrong: [a]\n"),
            ("a BOM before the fence", "\u{feff}---\nstrong: [a]\nmild: []\nnever: []\nclubs: []\n---\n"),
            ("only a spaced closing line", "---\nstrong: [a]\nmild: []\n--- \n"),
        ] {
            let v = vault();
            let (_, ctx) = interests_vault(&v, text);
            let before = fingerprint(&v);
            let mut j = Journal::new(&v);
            for _ in 0..2 {
                let err = write_one_line_literals(&v, "profile/interests.md", &four(["[x]", "[y]", "[z]", "[w]"]), &ctx, &mut j, &WriteOpts::default()).unwrap_err();
                assert_eq!(err, WriteError::NoFrontmatter("profile/interests.md".to_string()), "{why}");
            }
            assert_eq!(fingerprint(&v), before, "{why}: no record on any retry, and the bytes are left alone");
        }

        // A one-line or absent key is exactly `write_literals`: the same bytes and the same records
        // on a copy of the same file. `clubs` is absent, `mild` does not change, the body is kept.
        let text = "---\nstrong: [research]\nmild: []\n\nnever: []  # none yet\n---\n\nMy own notes.\n";
        let literals = four(["[research, AI talks]", "[]", "[recruitment]", "[AI Club]"]);
        let copy = std::env::temp_dir().join(format!("qo-write-copy-{}-{:?}", std::process::id(), std::thread::current().id()));
        let _ = std::fs::remove_dir_all(&copy);
        std::fs::create_dir_all(copy.join("state").join("journal")).unwrap();
        let mut out = Vec::new();
        for (v, one_line) in [(vault(), true), (copy.clone(), false)] {
            let (path, ctx) = interests_vault(&v, text);
            let mut j = Journal::new(&v);
            let opts = WriteOpts::default();
            let res = if one_line {
                write_one_line_literals(&v, "profile/interests.md", &literals, &ctx, &mut j, &opts)
            } else {
                write_literals(&v, "profile/interests.md", &literals, &ctx, &mut j, &opts)
            }
            .unwrap();
            let records: Vec<Record> = j.read(None, None).into_iter().map(|mut r| { r.remove("ts"); r.remove("seq"); r }).collect();
            out.push((std::fs::read(&path).unwrap(), res.written, records, path));
        }
        let (one_line, plain) = (&out[0], &out[1]);
        assert_eq!((&one_line.0, &one_line.1, &one_line.2), (&plain.0, &plain.1, &plain.2), "the bytes, the literals written and the records");
        assert_eq!(one_line.2.len(), 3, "a record each for strong, never and clubs; none for mild");
        let (read, warnings) = crate::events::load_interests(&one_line.3);
        assert!(warnings.is_empty(), "{warnings:?}");
        assert_eq!((read.strong, read.mild, read.never, read.clubs), (vec!["research".to_string(), "AI talks".into()], vec![], vec!["recruitment".into()], vec!["AI Club".into()]));
        assert!(pystr::read_text(&one_line.3).unwrap().ends_with("---\n\nMy own notes.\n"), "the body is kept");
        let _ = std::fs::remove_dir_all(&copy);
    }

    #[test]
    fn profile_primitives_take_the_human_gate() {
        use crate::journal::{HUMAN_ACTOR, LEGACY_HUMAN_ACTOR};
        for (file, actor) in [
            (Some("human_actor: alice\n".to_string()), HUMAN_ACTOR),
            (Some(format!("human_actor: {HUMAN_ACTOR}\n")), LEGACY_HUMAN_ACTOR),
            (None, HUMAN_ACTOR),
        ] {
            let v = vault();
            std::fs::create_dir_all(v.join("profile")).unwrap();
            pystr::write_text(&v.join("profile").join("interests.md"), INTERESTS).unwrap();
            if let Some(text) = &file { actor_file(&v, text) }
            let before = fingerprint(&v);
            let mut j = Journal::new(&v);
            let ctx = WriteContext::new(actor, "dashboard");
            let created = create_profile_file(&v, "preferences", "Mornings.\n", &ctx, &mut j).unwrap_err();
            let written = write_one_line_literals(&v, "profile/interests.md", &lit(&[("strong", "[x]")]), &ctx, &mut j, &WriteOpts::default()).unwrap_err();
            for err in [created, written] {
                assert!(matches!(err, WriteError::Actor(_)), "{actor} with {file:?}: {err}");
            }
            assert_eq!(fingerprint(&v), before, "{actor} with {file:?}: nothing journalled or written");
        }
        // An agent is never gated, whatever the file says.
        let v = vault();
        actor_file(&v, "human_actor: alice\n");
        let mut j = Journal::new(&v);
        let agent = WriteContext::new("agent:knowlu.enrich", "local-runner");
        let path = create_profile_file(&v, "interests", INTERESTS, &agent, &mut j).unwrap();
        write_one_line_literals(&v, "profile/interests.md", &lit(&[("strong", "[research]")]), &agent, &mut j, &WriteOpts::default()).unwrap();
        assert_eq!(crate::events::load_interests(&path).0.strong, ["research"]);
        let records = j.read(None, None);
        assert_eq!(records.len(), 2);
        assert!(records.iter().all(|r| r["actor"] == "agent:knowlu.enrich"), "{records:?}");
    }

    /// T1b ruling: past §7.2's column rule, `value_spans_lines` is also true wherever the reader
    /// (`split_frontmatter`, so `load_interests`) and the one-line surgery would disagree about the
    /// key; and it stays false on every file Knowlu writes, so Knowlu's own lists stay editable.
    #[test]
    fn value_spans_lines_errs_toward_true_where_the_reader_and_the_surgery_disagree() {
        let flow = "---\nstrong: [research,\nAI talks]\nmild: []\n---\n";
        let read = split_frontmatter(flow).expect("the reader takes a flow list continued at column 0").0;
        assert_eq!(crate::yaml::get(&read, "strong"), Some(&parse_literal("[research, AI talks]")), "so the surgery would orphan `AI talks]`");
        let wrong: Vec<&str> = [
            ("a flow list continued at column 0", flow),
            ("a quoted key the surgery cannot find", "---\n\"strong\": [research]\n---\n"),
            ("the key on the opening line", "---strong: [research]\nmild: []\n---\n"),
            ("the reader closes on a spaced line before the writer's", "---\nmild: []\n--- \nBody\n---\n"),
            ("only a spaced closing line", "---\nstrong: []\n--- \nBody\n"),
            ("a frontmatter the reader cannot parse", "---\nstrong: [research\n---\n"),
            ("section 7.2's own rule: a comment after a one-line value", "---\nstrong: []\n# later\nmild: []\n---\n"),
        ]
        .into_iter()
        .filter(|(_, text)| !value_spans_lines(text, "strong"))
        .map(|(why, _)| why)
        .collect();
        assert!(wrong.is_empty(), "false where it must be true: {wrong:?}");

        let crlf = INTERESTS.replace('\n', "\r\n");
        let mut editable: Vec<(String, &str, &str)> = [
            ("one line, with a comment", "---\nstrong: [research]  # mine\nmild: []\n---\n"),
            ("blank lines between the keys", "---\nstrong: [research, AI talks]\n\n\nmild: []\n\n---\n"),
            ("an absent key", "---\nmild: []\n---\n"),
            ("a longer key sharing the prefix", "---\nstrongest:\n  - x\nstrong: [a]\n---\n"),
            ("list lines in the body", "---\nmild: []\n---\nstrong:\n  - x\n"),
            ("no frontmatter: write_one_line_literals refuses the file by name", "Mornings.\nstrong:\n  - x\n"),
        ]
        .map(|(why, text)| (why.to_string(), text, "strong"))
        .into();
        for key in ["strong", "mild", "never", "clubs"] {
            editable.push((format!("Knowlu's own file ({key})"), INTERESTS, key));
            editable.push((format!("Knowlu's own file, CRLF ({key})"), crlf.as_str(), key));
        }
        let wrong: Vec<String> =
            editable.into_iter().filter(|(_, text, key)| value_spans_lines(text, key)).map(|(why, ..)| why).collect();
        assert!(wrong.is_empty(), "true where it must be false: {wrong:?}");
    }
}
