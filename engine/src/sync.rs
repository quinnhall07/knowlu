//! Journal sync (cloud design §5.5, as amended 2026-09-17): the account's own copy of its own vault.
//!
//! **What leaves this machine.** Every journal record this device writes, as `ledger::dumps_value`
//! wrote it, and the whole text of every note that changed. The account is the source of truth for
//! that data (amendment ruling 2) and this folder is a mirror of it; our service can read both, and
//! says so on the privacy page in the words Quinn and the lawyer approved. What does **not** leave
//! is a portal password, a session token in a log, an app-data path or a machine name: the `device`
//! column is `sha256(account_id + "\n" + hostname)` truncated to sixteen hex characters, which is an
//! identity for "the same machine as last time" and nothing else.
//!
//! **What this module is not.** It is transport, never judgment: `rank` does not reach it and
//! neither does anything under `cli.rs` except the `sync` subcommand itself. Nothing here calls a
//! model. And it never deletes a local note because the account does not have one — the mirror is
//! filled by the account, never emptied by it.
//!
//! **Every failure is a named line and exit 0.** No account, no session, no entitlement, no network,
//! a 402, a 5xx: the day still ranks from the folder on disk, because a slot step that exits
//! non-zero is retry backoff and an amber tray twice a day for ever.
//!
//! **Delivery is a pull, and the pull carries one thing.** `/sync-pull` returns records **another
//! desktop of this account** pushed. The service's own writes — judgment fields, Gmail-derived
//! notes, event verdicts, rule proposals — are delivered by C2, per item, inside the slot step that
//! asks for them, and are journalled on the device that asked; they then go up through this module
//! like any other record. The fidelity ledger's `§5.5 Down` row argues it.

use std::path::Path;

use ring::digest;
use serde_json::Value;

use crate::journal::Journal;

/// The largest journal record this device will send, in bytes, matching
/// `sync_records.body`'s `octet_length` check. A record is a handful of scalars and, for a
/// `create`, one frontmatter mapping; 16 KiB is far past any that exists. A record over it is
/// reported by name and left in the journal, never truncated — a truncated record is a lie.
pub const MAX_RECORD_BYTES: usize = 16 * 1024;

/// The largest note text this device will send, in bytes, matching `sync_notes.body`'s check. A
/// note longer than 128 KiB is a pasted document, not a task; it stays in the vault and the push
/// says so in one line.
pub const MAX_NOTE_BYTES: usize = 128 * 1024;

/// Everything this module can refuse. **A closed set of words** (ruling R-3a-20's shape): no value
/// here came from a note or a server body.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SyncError {
    /// No `config/cloud.yaml`: this vault has no account.
    NoAccount,
    /// There is an account but no usable session: the student is signed out on this machine.
    NoSession(String),
    /// The service refused or could not be reached. Carries `CloudError`'s own sentence, which is
    /// already scrubbed of the bearer.
    Service(String),
    /// A pulled row is not what it claims to be.
    Shape(&'static str),
    /// The vault, or the cursor file.
    Io(String),
}

impl SyncError {
    /// The word the `sync` step prints and the page shows. Never a path, never a body.
    pub fn label(&self) -> &'static str {
        match self {
            SyncError::NoAccount => "no account",
            SyncError::NoSession(_) => "no session",
            SyncError::Service(_) => "the service refused",
            SyncError::Shape(_) => "an unreadable row",
            SyncError::Io(_) => "the vault could not be read",
        }
    }
}

impl std::fmt::Display for SyncError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            SyncError::NoSession(why) | SyncError::Service(why) | SyncError::Io(why) => {
                write!(f, "{} ({why})", self.label())
            }
            SyncError::Shape(what) => write!(f, "{} ({what})", self.label()),
            other => write!(f, "{}", other.label()),
        }
    }
}

/// Lowercase hex of `SHA-256(bytes)`. The one hash this module computes, and the one the server
/// re-derives: `sync-push` refuses a row whose `hash` is not this of its `body`, so the idempotence
/// key is a fact about the row rather than a claim about it.
///
/// `ring`, not `sha2`: this binary already links `ring` for TLS, so the primitive is free, and the
/// alternative would put a second `digest` major beside `sha1`'s (`dependency_boundary.rs` argues it
/// at length and refuses the alternative by name).
pub fn sha256_hex(bytes: &[u8]) -> String {
    digest::digest(&digest::SHA256, bytes).as_ref().iter().map(|b| format!("{b:02x}")).collect()
}

/// Which machine pushed a row, to a server that has no reason to learn which machine that is.
///
/// The hostname is in the mix so two desktops of one account differ; the account id is in the mix so
/// the same hostname under two accounts does not collide; sixteen hex characters is enough to
/// separate the handful of desktops one student owns and short enough that it is obviously not a
/// name. `journal::device_name()` is the same value every journal record already carries **inside**
/// the row, where the student can read it — this is the outside, where nobody needs to.
pub fn device_token(account_id: &str) -> String {
    let mut input = String::with_capacity(account_id.len() + 1 + 32);
    input.push_str(account_id);
    input.push('\n');
    input.push_str(&crate::journal::device_name());
    sha256_hex(input.as_bytes())[..16].to_string()
}

/// One page, both ways. The same number `sync_rows.ts::MAX_ROWS` and `sync-pull`'s `MAX_PAGE` use,
/// and under PostgREST's own `max_rows` (1000), so a short page always means "that is all".
pub const PAGE: usize = 500;

/// A byte budget for one page, on top of the row cap (R-C3′-exec-11). The server
/// (`_shared/http.ts::readJson`, called from `sync-push`'s handler with `MAX_PUSH_BYTES`) reads a
/// whole push under a **4 MiB** cap measured in UTF-16 code units (`String.length`), not bytes.
/// UTF-8 bytes are never fewer than UTF-16 units for the same text, so a page whose rows sum to at
/// most this many UTF-8 bytes always fits under the server's cap. 3 MiB, not 4: `PAGE` rows at
/// either per-row cap (`MAX_RECORD_BYTES`, `MAX_NOTE_BYTES`) could sum past 4 MiB on their own, and
/// the margin is what lets `build_push` stop a page here, on its own terms, rather than lean on the
/// server's 413 (the transport cap, not the account's 403 ceiling — see [`push`]).
pub const PUSH_BUDGET_BYTES: usize = 3_145_728;

/// Who a pulled write is attributed to on THIS machine's side of a reconciliation. The foreign
/// records keep their own actor; this is the actor of the local effect — the supersede record, the
/// amend card, the note write — so the Runs view and the journal both say a sync did it.
///
/// **Declared here, in the push half, and not beside `apply` where it is used most**, because
/// `build_push` is what has to recognise it: every record under this actor is a mirror of something
/// the account already holds, and pushing one would tell every other desktop about an event that
/// only ever happened on this one (review S1, ruling R-C3′-plan-2).
pub const ACTOR: &str = "agent:knowlu.sync";

/// Generated, device-local, and **never synced**: it holds two integers, one timestamp and a map of
/// note path → content hash, every one of which is already in the vault in plainer form.
pub const CURSOR_FILE: &str = "state/sync-cursor.json";

/// What this device has already seen and already sent.
///
/// `pushed_through` plus `boundary` is the pair that makes "where did I stop" exact. The journal is
/// `ts`-ordered and several records can share a `ts` to the millisecond, so a cursor that stored
/// only a timestamp would either re-send the whole millisecond every run or skip the half of it that
/// landed after the read. `boundary` holds the hashes of the records at exactly `pushed_through`;
/// the next run reads from that timestamp inclusive and drops the ones it already sent.
#[derive(Debug, Clone, Default, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct Cursor {
    #[serde(default)] pub record_cursor: i64,
    #[serde(default)] pub note_cursor: i64,
    #[serde(default)] pub pushed_through: String,
    #[serde(default)] pub boundary: Vec<String>,
    #[serde(default)] pub notes: std::collections::BTreeMap<String, String>,
}

/// A missing or unreadable cursor is a **fresh** cursor, never an error: the worst it costs is one
/// full re-push, every row of which is idempotent on its own hash, and the alternative — a slot that
/// fails because a generated file was hand-edited — is the failure mode this product does not have.
pub fn load_cursor(vault: &Path) -> Cursor {
    crate::pystr::read_text(&vault.join(CURSOR_FILE))
        .ok()
        .and_then(|t| serde_json::from_str(&t).ok())
        .unwrap_or_default()
}

/// Written through `ledger::dumps_value` and `pystr::write_text`, like every other file this crate
/// puts in a vault.
pub fn save_cursor(vault: &Path, cursor: &Cursor) -> Result<(), SyncError> {
    let value = serde_json::to_value(cursor).map_err(|e| SyncError::Io(e.to_string()))?;
    let path = vault.join(CURSOR_FILE);
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent).map_err(|e| SyncError::Io(e.to_string()))?;
    }
    crate::pystr::write_text(&path, &crate::ledger::dumps_value(&value)).map_err(|e| SyncError::Io(e.to_string()))
}

/// Inside the vault, under one of `ids::NOTE_FOLDERS`, markdown, and no segment that climbs out.
///
/// **One implementation, three callers** (review I7): the push builder, the applier and the
/// restorer. `sync_rows.ts::NOTE_PATH_RE` and `sync_notes.path`'s own check are the same rule on the
/// other two sides of the wire, and `is_note_path_and_the_servers_regex_agree` pins them together.
///
/// **The name itself is bounded too** (R-C3′-exec-8/11): the server's regex is
/// `^(tasks|approvals|archive|courses|issues|info)\/[A-Za-z0-9._ /-]{1,300}\.md$`, so the part after
/// `<folder>/` and before the final `.md` must be 1–300 characters, each one of
/// `A-Z a-z 0-9 . _ space / -`. Checked with plain character comparisons rather than a new `regex`
/// call — the crate already depends on `regex` for `ids.rs`, but this rule is small enough that a
/// dependency is not the cheaper way to state it.
pub fn is_note_path(vault: &Path, rel: &str) -> bool {
    if rel.is_empty() || rel.contains('\\') || rel.contains("//") || !rel.ends_with(".md") {
        return false;
    }
    if rel.split('/').any(|seg| seg == ".." || seg.is_empty()) {
        return false;
    }
    let Some((folder, rest)) = rel.split_once('/') else { return false };
    if !crate::ids::NOTE_FOLDERS.contains(&folder) {
        return false;
    }
    let Some(middle) = rest.strip_suffix(".md") else { return false };
    let len = middle.chars().count();
    if len == 0 || len > 300 {
        return false;
    }
    if !middle.chars().all(|c| c.is_ascii_alphanumeric() || matches!(c, '.' | '_' | ' ' | '/' | '-')) {
        return false;
    }
    crate::ids::inside_vault(vault, &vault.join(rel)).is_ok()
}

/// Every note in the vault, vault-relative and POSIX-separated, sorted.
///
/// **Flat, like every other pass in this engine.** `ids::scan_notes` and `backup::BACKUP_FOLDERS`
/// treat the six folders as flat and nothing in this product has ever produced a nested note;
/// `is_note_path` accepts any depth because a *pulled* path must be checked whatever it is, but
/// making this side recursive alone would push a file nothing else in the engine can see. Deferred,
/// and recorded in *Deferred minors*.
pub fn note_paths(vault: &Path) -> Vec<String> {
    let mut out = Vec::new();
    for folder in crate::ids::NOTE_FOLDERS {
        let Ok(entries) = std::fs::read_dir(vault.join(folder)) else { continue };
        for entry in entries.flatten() {
            let path = entry.path();
            if path.extension().map(|x| x == "md") == Some(true) && path.is_file() {
                out.push(format!("{folder}/{}", path.file_name().unwrap_or_default().to_string_lossy()));
            }
        }
    }
    out.sort();
    out
}

/// What one push carries. `warnings` are lines the run prints; they are never sent.
#[derive(Debug, Clone, Default)]
pub struct PushBatch {
    pub device: String,
    pub records: Vec<Value>,
    pub notes: Vec<Value>,
    pub warnings: Vec<String>,
}

/// Everything written since the cursor, and the cursor that follows it.
///
/// **Two independent halves.** Records come from the journal, in `ts` order, from `pushed_through`
/// inclusive with `boundary` subtracted. Notes come from the folder: every path whose text hashes
/// differently from what the cursor remembers, plus a tombstone for every path the cursor remembers
/// and the folder no longer has. The note half is deliberately state-based rather than journal-based
/// — a note edited by hand in an editor has no record, and the mirror has to carry it anyway.
///
/// **One byte budget, shared across both halves** (R-C3′-exec-11): `PUSH_BUDGET_BYTES` is a running
/// total of each sent row's own `ledger::dumps_value` serialisation — the `{"hash","body"}` /
/// `{"path","body"}` / tombstone value exactly as it goes on the wire — charged to records first and
/// then to notes. A row that would carry the total past the budget ends that half's page with a
/// `break`, never a `continue`: the cursor must not advance past a row that was not sent, and a
/// `continue` would let a later, smaller row slip in out of order while a bigger one waited. The
/// `PAGE` row cap is checked independently and stays in force alongside it.
///
/// **The returned cursor is only saved by the caller after the push succeeds.** A cursor advanced
/// over a batch the service never received is the one bug that loses a record for good.
pub fn build_push(vault: &Path, cursor: &Cursor, account_id: &str, journal: &mut Journal) -> (PushBatch, Cursor) {
    let mut batch = PushBatch { device: device_token(account_id), ..Default::default() };
    let mut next = cursor.clone();
    let mut budget_used: usize = 0;

    let since = if cursor.pushed_through.is_empty() { None } else { Some(cursor.pushed_through.as_str()) };
    let already: std::collections::BTreeSet<&String> = cursor.boundary.iter().collect();
    for record in journal.read(since, None) {
        if batch.records.len() >= PAGE { break; }
        // **Never push what `apply` wrote** (review S1; ruling R-C3′-plan-2). When a pull applies a
        // foreign change, the `write::` function that carries it out journals a SECOND, locally
        // authored record under `ACTOR` — `write_literals` for a field, `write::move_note` for a
        // rename, `write::delete` for a settle — with a fresh `ts` and this machine's own
        // `device_name()`. That record hashes differently from the foreign one it mirrors, so the
        // content filter below cannot see it: it would go up as a new event and come down on every
        // other desktop as something that had never happened there. **Every** record under this
        // actor is excluded, whatever its `op`: everything `apply` writes under it is a mirror of
        // something the account already holds, so no device needs to learn it twice, and a filter
        // narrowed to `move`/`delete` would miss the `set` echo the field path produces on every
        // single pull. The foreign record itself is NOT excluded — its actor is whoever made it —
        // and re-pushing it is free: `/sync-push` re-derives `sha256(body)` and `sync_db::saveRecords`
        // upserts on `account_id,record_hash` against Task 1's `sync_records_once`, so it lands on
        // the row it already is, keeps its `seq` (an identity column does not move on an update) and
        // its `received_at` (a default, not an on-update), and therefore never resurfaces on another
        // desktop's cursor.
        if record.get("actor").and_then(Value::as_str) == Some(ACTOR) { continue; }
        let body = crate::ledger::dumps_value(&Value::Object(record.clone()));
        let hash = sha256_hex(body.as_bytes());
        if already.contains(&hash) { continue; }
        if body.len() > MAX_RECORD_BYTES {
            batch.warnings.push(format!("sync: one journal record is too large to send ({} bytes); it stays in the journal", body.len()));
            continue;
        }
        let row = serde_json::json!({ "hash": hash, "body": body });
        let row_len = crate::ledger::dumps_value(&row).len();
        if budget_used + row_len > PUSH_BUDGET_BYTES { break; }
        budget_used += row_len;
        let ts = record.get("ts").and_then(Value::as_str).unwrap_or_default().to_string();
        if ts != next.pushed_through {
            next.pushed_through = ts;
            next.boundary.clear();
        }
        next.boundary.push(hash.clone());
        batch.records.push(row);
    }

    let on_disk = note_paths(vault);
    let mut seen = std::collections::BTreeSet::new();
    for rel in &on_disk {
        if batch.notes.len() >= PAGE { break; }
        seen.insert(rel.clone());
        if !is_note_path(vault, rel) {
            batch.warnings.push(format!(
                "sync: {rel} has a name the account cannot store (letters, digits, spaces and . _ - only); it stays on this machine until it is renamed"
            ));
            continue;
        }
        let Ok(text) = crate::pystr::read_text(&vault.join(rel)) else {
            batch.warnings.push(format!("sync: {rel} could not be read; it stays on this machine"));
            continue;
        };
        if text.len() > MAX_NOTE_BYTES {
            batch.warnings.push(format!("sync: {rel} is too large to send ({} bytes); it stays on this machine", text.len()));
            continue;
        }
        let hash = sha256_hex(text.as_bytes());
        if next.notes.get(rel) == Some(&hash) { continue; }
        let row = serde_json::json!({ "path": rel, "body": text });
        let row_len = crate::ledger::dumps_value(&row).len();
        if budget_used + row_len > PUSH_BUDGET_BYTES { break; }
        budget_used += row_len;
        next.notes.insert(rel.clone(), hash);
        batch.notes.push(row);
    }
    for rel in cursor.notes.keys() {
        if batch.notes.len() >= PAGE { break; }
        if seen.contains(rel) { continue; }
        let row = serde_json::json!({ "path": rel, "deleted": true });
        let row_len = crate::ledger::dumps_value(&row).len();
        if budget_used + row_len > PUSH_BUDGET_BYTES { break; }
        budget_used += row_len;
        next.notes.remove(rel);
        batch.notes.push(row);
    }
    (batch, next)
}

/// The one POST. `CloudClient::post` already serialises through `dumps_value`, carries the bearer
/// and the anon key, and scrubs the bearer from every error string it produces.
///
/// **403 is the named ceiling, not a transport failure** (R-C3′-exec-7): `/sync-push` answers an
/// account that is at `sync_limits.ceiling` with 403 `{"error":"this account's copy is at its size
/// limit"}`, which arrives here as `CloudError::Status { code: 403, .. }` like any other status —
/// this function does no special-casing of it. A 413 means only that the whole request body was
/// over the server's own transport cap (`MAX_PUSH_BYTES`), which `PUSH_BUDGET_BYTES` is sized to
/// avoid in the first place; 403 is the one the caller has to act on, by leaving the records in the
/// journal and telling the student their copy is full.
pub fn push(client: &crate::cloudmodel::CloudClient, batch: &PushBatch) -> Result<(usize, usize), crate::cloudmodel::CloudError> {
    let body = serde_json::json!({ "device": batch.device, "records": batch.records, "notes": batch.notes });
    let reply = client.post("/sync-push", &body)?;
    let count = |key: &str| reply.get(key).and_then(Value::as_u64).unwrap_or(0) as usize;
    Ok((count("records"), count("notes")))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_content_hash_is_sha256_of_the_canonical_bytes() {
        // The empty-string vector, so a reader can check it against any SHA-256 implementation.
        assert_eq!(
            sha256_hex(b""),
            "e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855"
        );
        assert_eq!(sha256_hex(b"abc").len(), 64);
    }

    #[test]
    fn a_device_token_is_sixteen_hex_stable_and_not_the_hostname() {
        let a = device_token("acct-1");
        assert_eq!(a.len(), 16);
        assert!(a.chars().all(|c| c.is_ascii_hexdigit() && !c.is_ascii_uppercase()));
        assert_eq!(a, device_token("acct-1"), "stable across calls");
        assert_ne!(a, device_token("acct-2"), "and scoped to the account");
        let host = crate::journal::device_name().to_lowercase();
        assert!(!host.is_empty() && !a.contains(&host), "the hostname is not in the token");
    }

    #[test]
    fn a_refusal_never_carries_a_path_or_a_body() {
        let e = SyncError::Io("C:\\Users\\someone\\Knowlu\\Vault".to_string());
        assert_eq!(e.label(), "the vault could not be read");
        // The LABEL is what a line prints; `Display` is what a report may carry, and it is allowed
        // the cause. What matters is that the two are different functions and the step prints the
        // first — `run_lines_with` is where that is asserted (Task 7).
        assert!(!e.label().contains("Users"));
    }
}
