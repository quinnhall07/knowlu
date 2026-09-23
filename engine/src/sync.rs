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

use std::io::Write;
use std::path::{Path, PathBuf};

use ring::digest;
use serde_json::Value;

use crate::journal::Journal;
use crate::ledger::Record;

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
///
/// **The amend cards it files never leave this device either** (R-C3′-exec-18, probe N18): a sync
/// card's `from` is this device's withheld value, so it can only ever be answered here, and a copy
/// anywhere else can only mislead — worse, its status records would travel back and settle the real
/// card unanswered. See [`SyncCards`]. And because nothing under this actor is ever sent, a `create`
/// under it in this vault's journal always means "filed on this vault's machine" — which is what
/// `apply` keys card ownership on, so a machine rename cannot orphan a card (probe N19); a PULLED
/// record under this actor is refused outright for the same reason.
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
    note_paths_and_unreadable(vault).0
}

/// `note_paths`, plus (M2) the folders whose listing failed for a reason other than "the folder does
/// not exist". A missing folder is an ordinary empty folder — plenty of vaults have no `courses/`
/// note yet — but any OTHER `read_dir` failure (permissions, a transient handle problem, or, as the
/// test proves deterministically, a folder having been replaced by a plain file) must not read as
/// "this folder is now empty", or `build_push`'s tombstone pass would mark every note the cursor
/// remembers there as deleted. `build_push` uses this directly; `note_paths` stays the public,
/// one-tuple-element shape the brief and its own tests already depend on.
fn note_paths_and_unreadable(vault: &Path) -> (Vec<String>, std::collections::BTreeSet<&'static str>) {
    let mut out = Vec::new();
    let mut unreadable = std::collections::BTreeSet::new();
    for folder in crate::ids::NOTE_FOLDERS {
        match std::fs::read_dir(vault.join(folder)) {
            Ok(entries) => {
                for entry in entries.flatten() {
                    let path = entry.path();
                    if path.extension().map(|x| x == "md") == Some(true) && path.is_file() {
                        out.push(format!("{folder}/{}", path.file_name().unwrap_or_default().to_string_lossy()));
                    }
                }
            }
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => {}
            Err(_) => {
                unreadable.insert(folder);
            }
        }
    }
    out.sort();
    (out, unreadable)
}

/// Every sync amend card this vault holds — by `id` and by every path it has ever had — so
/// `build_push` can keep each one, and everything about it, off the wire (R-C3′-exec-18, probe N18).
///
/// A card's existence and state could reach the account by five paths, and this is what closes
/// each one in `build_push`:
/// 1. **its note**, live in `approvals/` or settled into `archive/` — a note whose `created_by` is
///    [`ACTOR`] is never sent as a row (the notes loop);
/// 2. **its `create`**, under [`ACTOR`] — already never sent (the actor filter);
/// 3. **its settle** by sync (`write::delete` under [`ACTOR`]) — already never sent (the actor
///    filter);
/// 4. **its status and its settle by anyone else** — the student's `approved`/`rejected`, the
///    deck's `executed`, the cap's snooze, `process_approvals`' archive move — ordinary `set` and
///    `delete` records under the student's or the runner's actor, each carrying the CARD's `id`:
///    a record whose `id` is a sync card's, or whose path is one of its paths, is never sent (the
///    records loop);
/// 5. **its tombstone**, once it leaves `approvals/` — its paths are never recorded in the cursor
///    (the notes loop drops them), and a path that is one of its paths is never tombstoned (the
///    tombstone pass), so even a cursor from before this rule sends nothing.
///
/// What an approved card DOES to its target — `apply_amendment`'s write of the new value onto the
/// task — is a record carrying the TASK's `id`, and travels like any other edit: that is how the
/// other desktop converges.
#[derive(Debug, Default)]
struct SyncCards {
    ids: std::collections::BTreeSet<String>,
    paths: std::collections::BTreeSet<String>,
}

impl SyncCards {
    /// From the notes on disk (every `approvals/` and `archive/` note whose frontmatter says
    /// `created_by: agent:knowlu.sync` — a read, never a re-dump) and from this vault's journal
    /// (every `create` under [`ACTOR`], or of a note whose frontmatter says so), then every path any
    /// journalled record ever gave one of those ids: its `path`, and a move's or settle's `old`/`new`.
    fn find(vault: &Path, on_disk: &[String], journal: &mut Journal) -> SyncCards {
        let mut cards = SyncCards::default();
        for rel in on_disk.iter().filter(|r| r.starts_with("approvals/") || r.starts_with("archive/")) {
            let Ok(text) = crate::pystr::read_text(&vault.join(rel)) else { continue };
            if let Some(id) = sync_card_note(&text) {
                cards.paths.insert(rel.clone());
                if let Some(id) = id {
                    cards.ids.insert(id);
                }
            }
        }
        let records = journal.read(None, None);
        let str_of = |r: &Record, k: &str| r.get(k).and_then(Value::as_str).unwrap_or_default().to_string();
        for record in &records {
            let by_sync = str_of(record, "actor") == ACTOR
                || record.get("new").and_then(|n| n.get("created_by")).and_then(Value::as_str) == Some(ACTOR);
            if str_of(record, "op") == "create" && by_sync && !str_of(record, "id").is_empty() {
                cards.ids.insert(str_of(record, "id"));
            }
        }
        for record in &records {
            if !cards.ids.contains(&str_of(record, "id")) {
                continue;
            }
            for key in ["path", "old", "new"] {
                let value = str_of(record, key);
                if value.ends_with(".md") {
                    cards.paths.insert(value);
                }
            }
        }
        cards
    }

    fn covers(&self, record: &Record) -> bool {
        let str_of = |k: &str| record.get(k).and_then(Value::as_str).unwrap_or_default();
        (!str_of("id").is_empty() && self.ids.contains(str_of("id"))) || self.paths.contains(str_of("path"))
    }
}

/// `Some(the card's id, if it has one)` when `text` is a note whose frontmatter says
/// `created_by: agent:knowlu.sync` — a sync amend card. A plain substring test first, so the
/// frontmatter of a note that never mentions the actor is never parsed; then a real read of the
/// frontmatter, so a body line that merely mentions it is not mistaken for one.
fn sync_card_note(text: &str) -> Option<Option<String>> {
    if !text.contains(ACTOR) {
        return None;
    }
    let (meta, _) = crate::models::split_frontmatter(text).ok()?;
    if crate::yaml::get(&meta, "created_by").and_then(crate::yaml::text).as_deref() != Some(ACTOR) {
        return None;
    }
    Some(crate::yaml::get(&meta, "id").and_then(crate::yaml::text))
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
///
/// **R-C3′-exec-12, the wedge principle.** A batch is all-or-nothing server-side
/// (`sync-push/handler.ts`), so any row the server would refuse turns every step of the slot into a
/// 400 until a human intervenes. This function therefore predicts every refusal
/// `_shared/sync_rows.ts` makes and never sends the row that would trigger it, warning once instead:
/// an oversize record or note, a record whose body has no non-empty `op`/`actor`, a note that is
/// empty or carries a raw NUL byte, and a path — on either side, a live note or a tombstoned one —
/// that fails [`is_note_path`]. **The tombstone pass in particular is computed from the complete
/// on-disk listing, never from which paths the notes loop happened to visit before a `PAGE` or
/// budget `break`**, so a page that stops partway through the folder never marks a note still on
/// disk as deleted; a folder whose own listing failed for a reason other than "does not exist yet"
/// (`note_paths_and_unreadable`) is excluded from that pass entirely, for the same reason.
pub fn build_push(vault: &Path, cursor: &Cursor, account_id: &str, journal: &mut Journal) -> (PushBatch, Cursor) {
    let mut batch = PushBatch { device: device_token(account_id), ..Default::default() };
    let mut next = cursor.clone();
    let mut budget_used: usize = 0;

    // M2: a folder `read_dir` could not list at all (not merely "does not exist yet") must not be
    // read as "empty" by the tombstone pass below, or a transient listing failure would mark every
    // note the cursor remembers in it as deleted. Listed first, because `SyncCards` needs it too.
    let (on_disk, unreadable_folders) = note_paths_and_unreadable(vault);
    // R-C3′-exec-18: every sync amend card, by id and by every path it has had. Nothing about one is
    // ever sent — see `SyncCards` for the five paths and where each is closed below.
    let sync_cards = SyncCards::find(vault, &on_disk, journal);

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
        // M2 (fix round 1): a `supersede` record stays on this device. `reconcile::resolve` stamps
        // its own actor (`system:reconcile`, not `ACTOR`) and its `ts` is whichever contender WON —
        // often the past, and never this run's own clock — so neither filter above would catch it.
        // It also does nothing useful on the other desktop: that machine has no local `set` chain to
        // reconcile it against, since the chain reconstruction lives entirely in the two records
        // that produced it, which already travel (the foreign one verbatim, this device's own echo
        // under `ACTOR`, excluded above like any other echo). `ACTOR`'s own doc comment lists "the
        // supersede record" among what stays local, and this is the line that keeps it there.
        if record.get("op").and_then(Value::as_str) == Some("supersede") { continue; }
        // R-C3′-exec-18, path 4: a record about a sync card — its status, its snooze, its settle by
        // the deck — stays here like the card itself. A `continue`, like the two filters above, so
        // it never advances `next` and can never wedge a page.
        if sync_cards.covers(&record) { continue; }
        // R-C3′-exec-12 (the wedge principle): predict every refusal `sync_rows.ts::checkRecord`
        // makes and never send the row that would trigger it. `sync_rows.ts:89-93` refuses a record
        // whose body has no non-empty string `op` or `actor` — `ledger::read` itself validates only
        // `ts`, so a hand-edited or legacy journal line can carry neither. Treated exactly like an
        // oversize record below: skipped with a `continue`, so it never advances `next` and can
        // never wedge a page that also carries good records.
        let has_str = |key: &str| record.get(key).and_then(Value::as_str).map(|s| !s.is_empty()).unwrap_or(false);
        if !has_str("op") || !has_str("actor") {
            batch.warnings.push("sync: one journal record has no usable op/actor; it stays in the journal".to_string());
            continue;
        }
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

    // I1: tombstone against the COMPLETE listing, never against which paths this loop happened to
    // *visit* — a `PAGE` or budget `break` below must not read as "everything after this point in
    // the sorted listing is gone". `on_disk` already holds every note this build can see; comparing
    // `cursor.notes` against a full `BTreeSet` of it, after the loop, is what makes a break safe.
    for rel in &on_disk {
        if batch.notes.len() >= PAGE { break; }
        if !is_note_path(vault, rel) {
            batch.warnings.push(format!(
                "sync: {rel} has a name the account cannot store (letters, digits, spaces and . _ - only); it stays on this machine until it is renamed"
            ));
            continue;
        }
        // R-C3′-exec-18, paths 1 and 5: a sync card is never a row, and its path is dropped from the
        // cursor so no tombstone can ever follow it — even from a cursor written before this rule.
        if sync_cards.paths.contains(rel) {
            next.notes.remove(rel);
            continue;
        }
        let Ok(text) = crate::pystr::read_text(&vault.join(rel)) else {
            batch.warnings.push(format!("sync: {rel} could not be read; it stays on this machine"));
            continue;
        };
        // The same rule for a sync card outside `approvals/` and `archive/`, where `SyncCards` does
        // not look — none should exist, and none leaves if one does.
        if sync_card_note(&text).is_some() {
            next.notes.remove(rel);
            continue;
        }
        // R-C3′-exec-12: `sync_rows.ts:111` refuses an empty body ("a note has no body") and
        // `sync_rows.ts:114` refuses a raw NUL byte — predicted here so neither ever reaches the
        // wire, rather than wedging every future push until the file is fixed by hand.
        if text.is_empty() || text.contains('\u{0}') {
            batch.warnings.push(format!("sync: {rel} is empty or contains a character the account cannot store; it stays on this machine until it has real content"));
            continue;
        }
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
    let on_disk_set: std::collections::BTreeSet<&String> = on_disk.iter().collect();
    for rel in cursor.notes.keys() {
        if batch.notes.len() >= PAGE { break; }
        // I1: still on disk (whether or not this build's first loop got as far as visiting it) — not
        // deleted, so never a tombstone.
        if on_disk_set.contains(rel) { continue; }
        // R-C3′-exec-18, path 5: a path a sync card has had is never tombstoned.
        if sync_cards.paths.contains(rel) {
            next.notes.remove(rel);
            continue;
        }
        // M2: this folder's listing failed transiently; conclude nothing about what is or is not in
        // it this build.
        if let Some(folder) = rel.split('/').next() {
            if unreadable_folders.contains(folder) { continue; }
        }
        // Minor 3 / item 4: a hand-edited cursor can carry a path the server's own path rule would
        // refuse. Drop it from this device's own bookkeeping rather than resend a 400 every run.
        if !is_note_path(vault, rel) {
            batch.warnings.push(format!("sync: {rel} in the cursor is not a name the account can store; dropped without sending"));
            next.notes.remove(rel);
            continue;
        }
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

// ---------------------------------------------------------------------------
// The pull, `reconcile`, and the amend card (C3' Task 6).
// ---------------------------------------------------------------------------

/// One note as the account holds it. `text: None` is a tombstone: the account knows the path is
/// settled and carries no bytes for it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PulledNote {
    /// Which desktop pushed it — the opaque sixteen-hex token, never a hostname.
    pub device: String,
    pub path: String,
    pub text: Option<String>,
}

/// One page of `/sync-pull`, opened. `Default` is what `pulled_from_reply` builds from and what a
/// restore test starts from, so it is derived rather than hand-written (review R6).
#[derive(Debug, Clone, Default)]
pub struct Pulled {
    /// `(device token, the record)`. The record is the journal record itself, parsed out of the
    /// row's `body` and checked against the row's own hash before it ever gets here.
    pub records: Vec<(String, Record)>,
    pub notes: Vec<PulledNote>,
    pub record_cursor: i64,
    pub note_cursor: i64,
    pub more: bool,
    /// Rows this device refused on the way in, as lines the run prints. Never a path, never a value.
    pub warnings: Vec<String>,
}

/// A bare identifier: `^[A-Za-z_][A-Za-z0-9_]*$`, checked character by character rather than
/// pulling in a second `regex` dependency for one small rule.
fn is_field_ident(field: &str) -> bool {
    let mut chars = field.chars();
    match chars.next() {
        Some(c) if c.is_ascii_alphabetic() || c == '_' => {}
        _ => return false,
    }
    chars.all(|c| c.is_ascii_alphanumeric() || c == '_')
}

/// Is this a journal record at all? **Eight checks, and every one of them is a shape a malformed or
/// hostile row could otherwise slip through into `state/journal/`** (review I7, and fix round 1's
/// I2/M7).
pub fn record_is_well_formed(record: &Record) -> Result<(), &'static str> {
    let s = |k: &str| record.get(k).and_then(Value::as_str).unwrap_or_default();
    if !crate::journal::OPS.contains(&s("op")) { return Err("unknown op"); }
    if !crate::journal::VIAS.contains(&s("via")) { return Err("unknown via"); }
    let ts = s("ts");
    // I7's original check only asked whether `ts` parses at all. Every ordering this crate does —
    // `wins`, the journal's own `ts`-sort, the ledger's day-file name — compares the CANONICAL
    // string, not the parsed instant, so a row carrying a jiff-parseable but non-canonical `ts`
    // (missing milliseconds, a numeric offset instead of `Z`, six-digit microseconds) would sort in
    // a place its own value disagrees with (review M7). Requiring round-trip equality through
    // `journal::now_ts` catches exactly that, with no new dependency.
    match ts.parse::<jiff::Timestamp>() {
        Ok(parsed) if crate::journal::now_ts(Some(parsed)) == ts => {}
        _ => return Err("ts not canonical"),
    }
    if s("actor").is_empty() { return Err("no actor"); }
    if s("device").is_empty() { return Err("no device"); }
    let id = s("id");
    if !id.is_empty() && !crate::ids::is_id(id) { return Err("not an id"); }
    // review I2: nothing upstream of this guard ever looks at `field`. `reconcile` will build a
    // chain for any non-empty string, `write_literals` writes any key through to
    // `apply_frontmatter_fields_to_text`, and that function writes `"{key}: {value}"` onto the note
    // unescaped — so a `field` carrying a newline and a `---` can inject arbitrary frontmatter lines,
    // and `field: "id"` would let a pulled `set` retarget the note's own opaque identity. Refused
    // before the record ever reaches the journal, same as every other guard here.
    if let Some(field) = record.get("field").and_then(Value::as_str) {
        if field == "id" || !is_field_ident(field) { return Err("bad field"); }
    }
    Ok(())
}

/// The reply, parsed. **Every field the device trusts is re-derived from the row's own body**
/// (Task 2's carried obligation (a) from the sealed design's review): the record is parsed out of
/// `body` and its hash recomputed, so a row whose `record_hash` disagrees with its bytes is a
/// warning and a skip rather than a record with a borrowed identity.
pub fn pulled_from_reply(reply: &Value) -> Result<Pulled, SyncError> {
    let mut out = Pulled { record_cursor: reply.get("record_cursor").and_then(Value::as_i64).unwrap_or(0),
                           note_cursor: reply.get("note_cursor").and_then(Value::as_i64).unwrap_or(0),
                           more: reply.get("more").and_then(Value::as_bool).unwrap_or(false),
                           ..Default::default() };
    for row in reply.get("records").and_then(Value::as_array).cloned().unwrap_or_default() {
        let device = row.get("device").and_then(Value::as_str).unwrap_or_default().to_string();
        let body = row.get("body").and_then(Value::as_str).unwrap_or_default();
        if row.get("record_hash").and_then(Value::as_str) != Some(sha256_hex(body.as_bytes()).as_str()) {
            out.warnings.push("sync: one pulled record did not match its own hash and was skipped".to_string());
            continue;
        }
        match serde_json::from_str::<Value>(body) {
            Ok(Value::Object(map)) => out.records.push((device, map)),
            _ => out.warnings.push("sync: one pulled record was not a record and was skipped".to_string()),
        }
    }
    for row in reply.get("notes").and_then(Value::as_array).cloned().unwrap_or_default() {
        let path = row.get("path").and_then(Value::as_str).unwrap_or_default().to_string();
        let device = row.get("device").and_then(Value::as_str).unwrap_or_default().to_string();
        if row.get("deleted").and_then(Value::as_bool).unwrap_or(false) {
            out.notes.push(PulledNote { device, path, text: None });
            continue;
        }
        // M4 (fix round 1): a missing or non-string `body` on a LIVE row (one that is not a
        // tombstone) used to fall through `unwrap_or_default` into `Some("")` — a zero-byte file at
        // `apply`'s note-write step, which `build_push` would then warn about on every subsequent
        // run. An empty string is treated the same way: neither is a note worth writing.
        match row.get("body").and_then(Value::as_str) {
            Some(body) if !body.is_empty() => out.notes.push(PulledNote { device, path, text: Some(body.to_string()) }),
            _ => out.warnings.push("sync: one pulled note arrived with no body and was skipped".to_string()),
        }
    }
    Ok(out)
}

/// One `GET`. The device asks for both cursors in one call so the two windows share a clock on the
/// server (`READ_LAG_SECONDS`).
pub fn pull(client: &crate::cloudmodel::CloudClient, records_after: i64, notes_after: i64) -> Result<Pulled, SyncError> {
    let path = format!("/sync-pull?records_after={records_after}&notes_after={notes_after}&limit={PAGE}");
    let reply = client.get(&path).map_err(|e| SyncError::Service(e.to_string()))?;
    pulled_from_reply(&reply)
}

/// What one `apply` did. Every field is a count the run's own line reads; none is a path or a value.
#[derive(Debug, Clone, Default)]
pub struct ApplyReport {
    pub records: usize,
    pub notes_written: usize,
    pub applied: usize,
    pub cards: usize,
    pub superseded: usize,
    /// Notes this pull **relocated or settled** — a performed `move` and a tombstone both land here,
    /// because to a reader of the Runs view they are the same fact: a note is no longer where it was.
    pub moved: usize,
    pub refused: usize,
    pub warnings: Vec<String>,
}

/// Whether `path`'s exact file name — case included — is a real entry in its own parent directory.
///
/// `Path::exists` answers case-INsensitively on Windows/NTFS, which is wrong for two of `apply`'s
/// own questions: "is a note already at this exact pulled path" and "does the note this tombstone
/// names still exist under that exact spelling" (review I3). A student who renames `foo.md` to
/// `Foo.md` pushes a tombstone for `foo.md`; `sync-pull` returns every row for the account,
/// including the caller's own, with no device filter; and `file.exists()` on the still-live
/// `Foo.md` would answer true for a query about `foo.md`, so `write::delete` would archive the live
/// note. Comparing raw `OsStr` bytes from a directory listing is exact where `exists()` is not.
fn exact_case_exists(path: &Path) -> bool {
    let Some(name) = path.file_name() else { return false };
    let Some(parent) = path.parent() else { return false };
    std::fs::read_dir(parent)
        .map(|entries| entries.flatten().any(|e| e.file_name() == name))
        .unwrap_or(false)
}

/// The real, on-disk path in `path`'s own parent directory whose name matches `path`'s, ignoring
/// case — or `None` if nothing there matches even case-insensitively.
///
/// **O1 (re-review round 2).** A case-only rename made on the OTHER desktop pushes a live row under
/// the NEW spelling and, separately, a tombstone for the OLD one; by the time the live row is
/// processed, this device's own file is still sitting under the old spelling. `exact_case_exists`
/// alone would answer "not here" and try to create a brand-new file, which then collides with the
/// still-live old-cased file at the filesystem's own case-insensitive layer — exactly what left an
/// archived duplicate behind before this fix. This is what tells "genuinely new" (`None`) apart from
/// "the same note, cased differently" (`Some`).
fn case_insensitive_match(path: &Path) -> Option<PathBuf> {
    let name = path.file_name()?.to_string_lossy().to_lowercase();
    let parent = path.parent()?;
    std::fs::read_dir(parent)
        .ok()?
        .flatten()
        .find(|e| e.file_name().to_string_lossy().to_lowercase() == name)
        .map(|e| e.path())
}

/// The `id:` a pulled note's OWN text carries, read without writing anything — a plain parse into a
/// `Mapping` that is inspected once and dropped, the same shape `ids::read_meta` already does for a
/// file on disk. Never a re-dump: nothing here reconstructs frontmatter text from the parsed value.
///
/// **B3 (re-review round 2).** A case-insensitive name collision is not proof that two rows name the
/// same note — the other desktop can delete one note and separately create an unrelated one whose
/// name only happens to collide, case only. Comparing this against the local file's own `id:` is
/// what tells the two apart before either O1 branch treats them as one rename.
fn note_frontmatter_id(text: &str) -> Option<String> {
    let (meta, _) = crate::models::split_frontmatter(text).ok()?;
    crate::yaml::get(&meta, "id").and_then(crate::yaml::text)
}

/// Rename `from` to `to` — both already resolved, real paths — in two `std::fs::rename` steps
/// through a temp name in `to`'s own folder (review O1).
///
/// A single `rename(from, to)` is unreliable for a CASE-ONLY change on Windows: some filesystem/API
/// combinations treat the two spellings as the same existing path and refuse or silently no-op the
/// call, rather than actually updating the on-disk name. Moving off the colliding name entirely
/// first, then onto the final spelling, is not that ambiguous case either time. If the second step
/// fails, this restores the original spelling rather than leaving the note stranded under the temp
/// name — "keep the old spelling" (the ruling's own words) means exactly the name it started with.
fn rename_case_only(from: &Path, to: &Path) -> std::io::Result<()> {
    let parent = to.parent().unwrap_or_else(|| Path::new("."));
    let temp = parent.join(format!(".sync-case-{}-{}", std::process::id(), crate::journal::now_ts(None).replace(['-', ':', '.'], "")));
    std::fs::rename(from, &temp)?;
    match std::fs::rename(&temp, to) {
        Ok(()) => Ok(()),
        Err(e) => {
            let _ = std::fs::rename(&temp, from);
            Err(e)
        }
    }
}

/// Every still-open amend card **sync itself filed** on `target_rel`, sorted by path, each with its
/// `changes:` mapping as read (never re-dumped: the mapping is only inspected, and a card is only
/// ever settled whole through `write::delete` or superseded by a fresh `propose_amendment`).
///
/// Four filters, each load-bearing:
/// - **`pending` or `snoozed`** (review O3): the fifteen-a-day cap defers a proposal past budget
///   rather than deleting it, so a card the cap snoozed is still an open decision — leaving it out
///   let a newer pull file a second, live card beside it, and the snoozed one would later wake up
///   still offering its stale value. An `approved`/`rejected` card is a decision already made and
///   is never touched.
/// - **`created_by: sync::ACTOR`** (review R2, fix round 4): a judge-once proposal from
///   `agent:knowlu.enrich` — or any card a person or another agent filed — is a decision the student
///   has not made, and "a proposal is deferred, never deleted". Sync's convergence or a newer sync
///   conflict on the same field says nothing about it, so sync never settles or archives it.
/// - **Filed on THIS vault's machine** (probe N16, fix round 5; rename-safe since R-C3′-exec-18,
///   probe N19): the card's `id` is in `own`, the ids with a `create` record under [`ACTOR`] in
///   this vault's journal. `propose_amendment` goes through `write::create`, which journals exactly
///   that record; no `ACTOR` record ever leaves a device, and `apply` refuses a pulled one. A copy of
///   another desktop's card — one a build from before R-C3′-exec-18 delivered as a note — has no
///   such record here, and settling it would archive a card this device cannot answer (and, before
///   that ruling, tombstone the other desktop's still-open original). No frontmatter key is added.
/// - **Any field set**: the caller decides which cards a pull resolves (review D1, fix round 4);
///   matching one exact field set is what let a two-field card survive a pull that resolved one of
///   its fields.
///
/// Kept as its own scan rather than widening the shared `write::find_pending_amendment`, which the
/// judge-once re-propose path in `write_literals` also uses.
fn live_sync_cards(
    vault: &Path,
    target_rel: &str,
    own: &std::collections::BTreeSet<String>,
) -> Vec<(PathBuf, serde_yaml_ng::Mapping)> {
    let Ok(entries) = std::fs::read_dir(vault.join("approvals")) else { return Vec::new() };
    let mut paths: Vec<PathBuf> = entries
        .filter_map(|e| e.ok())
        .map(|e| e.path())
        .filter(|p| p.extension().map(|x| x == "md") == Some(true))
        .collect();
    paths.sort();

    let mut out = Vec::new();
    for path in paths {
        let Some(meta) = crate::ids::read_meta(&path) else { continue };
        let text = |key: &str| crate::yaml::get(&meta, key).and_then(crate::yaml::text);
        if text("type").as_deref() != Some("approval")
            || text("kind").as_deref() != Some("amend")
            || !matches!(text("status").as_deref(), Some("pending") | Some("snoozed"))
            || text("created_by").as_deref() != Some(ACTOR)
            || text("target").as_deref() != Some(target_rel)
            || !text("id").is_some_and(|id| own.contains(&id))
        {
            continue;
        }
        if let Some(serde_yaml_ng::Value::Mapping(changes)) = crate::yaml::get(&meta, "changes") {
            out.push((path, changes.clone()));
        }
    }
    out
}

/// A card's `changes`, or a proposed one, as one JSON object `{field: {from, to}}` — the shape
/// `yaml::to_json` gives a card's own `changes:` mapping back, so the two compare by value rather
/// than by how the emitter spelled them.
fn changes_json(changes: &[(String, serde_yaml_ng::Value, serde_yaml_ng::Value)]) -> Value {
    let mut out = serde_json::Map::new();
    for (field, from, to) in changes {
        let mut spec = serde_json::Map::new();
        spec.insert("from".to_string(), crate::yaml::to_json(from));
        spec.insert("to".to_string(), crate::yaml::to_json(to));
        out.insert(field.clone(), Value::Object(spec));
    }
    Value::Object(out)
}

/// Apply one pulled page to this vault.
///
/// **Order matters and is the argument.** Records are appended verbatim FIRST, because they are the
/// history and because the note effects below are derived from them; then the note-level effect is
/// settled per note; then the pulled note texts land, and only for paths this device has never seen.
/// A record that fails a guard is counted and named and never reaches the ledger.
pub fn apply(
    vault: &Path,
    page: &Pulled,
    ctx: &crate::write::WriteContext,
    journal: &mut Journal,
    today: jiff::civil::Date,
) -> ApplyReport {
    // M2 (fix round 1): every local effect below runs under `ctx`, and `build_push`'s echo filter
    // only ever excludes records under `sync::ACTOR` — so a caller that passed a human's context here
    // would push every mirrored effect back up as if the OTHER desktop had made it. Debug-only: the
    // cost of checking it on every production run is not worth paying for a mistake `cli.rs` and
    // `enrich.rs`'s own tests would already catch.
    debug_assert_eq!(ctx.actor, ACTOR, "apply's local effects must be attributed to sync::ACTOR");
    let mut report = ApplyReport { warnings: page.warnings.clone(), ..Default::default() };

    // 1. What this device already has. A record is identified by the hash of its canonical bytes,
    //    which is the same identity `sync_records_once` uses, so a record that came down twice — or
    //    came back down after this device pushed it (precondition P4) — is applied once.
    let known: std::collections::BTreeSet<String> = journal
        .read(None, None)
        .into_iter()
        .map(|r| sha256_hex(crate::ledger::dumps_value(&Value::Object(r)).as_bytes()))
        .collect();

    // 2 + 3. Guard, then append verbatim. `JsonlLedger::append` files by the record's OWN `ts`, so a
    //    record made on 15 September lands in `state/journal/2026-09-15.jsonl` and not in today's.
    let ledger = crate::ledger::JsonlLedger::new(vault.join("state").join("journal"));
    let mut touched: std::collections::BTreeMap<String, Vec<Record>> = std::collections::BTreeMap::new();
    let mut moves: Vec<(String, String)> = Vec::new();
    for (_, record) in &page.records {
        let body = crate::ledger::dumps_value(&Value::Object(record.clone()));
        if known.contains(&sha256_hex(body.as_bytes())) {
            continue;
        }
        if let Err(why) = record_is_well_formed(record) {
            report.refused += 1;
            report.warnings.push(format!("sync: a pulled record was refused ({why})"));
            continue;
        }
        // R-C3′-exec-18 (iii), hardening: `build_push` never sends a record under `ACTOR`, so no
        // desktop's sync sent this one. Refused before the journal append, so a `create` under
        // `ACTOR` in this journal always means "filed on this vault's machine" — the key card
        // ownership rests on below.
        if record.get("actor").and_then(Value::as_str) == Some(ACTOR) {
            report.refused += 1;
            report.warnings.push(
                "sync: a pulled record under this app's own sync actor was refused (sync's own records never leave their device)"
                    .to_string(),
            );
            continue;
        }
        let path = record.get("path").and_then(Value::as_str).unwrap_or_default();
        if !is_note_path(vault, path) {
            report.refused += 1;
            report.warnings.push("sync: a pulled record named a path outside the vault's notes".to_string());
            continue;
        }
        // A `move`'s destination is a string another machine sent, and it is checked BEFORE
        // `write::move_note` is called — not after, when the file would already be somewhere else.
        // The move itself is **performed**, in its own pass below (review R4): a foreign rename that
        // was only journalled would leave the old file sitting where it was, and the renamed note's
        // text would arrive at the new path as a note this device had never seen — one note in two
        // places, which is the silent divergence this whole module exists to prevent.
        let mut pending_move: Option<(String, String)> = None;
        if record.get("op").and_then(Value::as_str) == Some("move") {
            let dest = record.get("new").and_then(Value::as_str).unwrap_or_default();
            if !is_note_path(vault, dest) {
                report.refused += 1;
                report.warnings.push("sync: a pulled move named a destination outside the vault's notes".to_string());
                continue;
            }
            pending_move = Some((path.to_string(), dest.to_string()));
        }
        if let Err(e) = ledger.append(record) {
            report.warnings.push(format!("sync: a pulled record could not be journalled ({e})"));
            continue;
        }
        report.records += 1;
        if let Some(pair) = pending_move {
            moves.push(pair);
        }
        let id = record.get("id").and_then(Value::as_str).unwrap_or_default().to_string();
        if !id.is_empty() {
            touched.entry(id).or_default().push(record.clone());
        } else if record.get("op").and_then(Value::as_str) == Some("set") {
            // M5 (fix round 1): a `set` with no `id` is journalled — it is well-formed, `is_id`
            // only checks a non-empty one — but it can never be reconciled, because the per-note
            // pass below groups by `id`. It sits in the ledger for ever, doing nothing, and nothing
            // said so. This is also the one real way a note lacking its own `id:` frontmatter line
            // reaches the mtime-fallback path in `reconcile::resolve` at all: `write::write_literals`
            // journals `id: null` on both desktops for such a note, so this warning is the visible
            // half of that gap.
            report.warnings.push(format!("sync: {path} — a pulled `set` with no id can never be applied"));
        }
    }
    journal.invalidate();

    // 3a. **Perform the moves, after the whole record pass and before any reconcile.** After,
    //     because a move mid-loop would move a file out from under a later record's `path`; before,
    //     because the per-note pass below reads the note through `ids::read_meta` and has to find it
    //     where it now is. `write::move_note` journals this device's own `move` record under `ACTOR`
    //     beside the foreign one — which is right and is the same shape a tombstone takes below: the
    //     foreign record is the other desktop's history, and this one is what happened here.
    for (from, dest) in moves {
        if !vault.join(&from).exists() {
            // Already where it should be (a re-pull, or this device made the same move itself).
            continue;
        }
        match crate::write::move_note(vault, &from, &dest, ctx, journal) {
            Ok(_) => report.moved += 1,
            // The destination is taken. Not a failure of the sync, and **not retried**: `Cursor`
            // carries no retry queue, so if the destination frees up later this device does not
            // notice. Pilot-acceptable and recorded as such (round-2 re-review, R4(b)): the record
            // is journalled either way so nothing is lost, the note stays at its old path rather
            // than overwriting whatever is there, and the warning names both paths. A retry queue is
            // a feature, not a one-line fix, and it needs two desktops independently choosing one
            // destination filename — a case this plan says has never been exercised even once.
            Err(crate::write::WriteError::Exists(_)) => {
                report.warnings.push(format!("sync: {from} could not be renamed to {dest} — a note is already there"));
            }
            Err(e) => report.warnings.push(format!("sync: {from} could not be renamed ({e})")),
        }
    }
    journal.invalidate();

    // This machine's own device name (R1 below), and the ids of the cards filed on THIS vault's
    // machine — read once, after the record pass. `live_sync_cards` uses the second to tell this
    // device's own amend cards from a copy that arrived some other way (probe N16). Keyed on a
    // `create` under `ACTOR` (R-C3′-exec-18, probe N19), not on `device == device_name()`: no
    // `ACTOR` record ever leaves a device and a pulled one is refused above, so such a record was
    // written right here — under whatever this machine was called at the time, which is why a
    // rename no longer orphans a card.
    let this_device = crate::journal::device_name();
    let own_created: std::collections::BTreeSet<String> = journal
        .read(None, None)
        .into_iter()
        .filter(|r| {
            r.get("op").and_then(Value::as_str) == Some("create")
                && r.get("actor").and_then(Value::as_str) == Some(ACTOR)
        })
        .filter_map(|r| r.get("id").and_then(Value::as_str).map(str::to_string))
        .collect();

    // 4. Per note, with the roles reversed exactly as the table above says.
    for (id, foreign) in &touched {
        if foreign.is_empty() { continue; }
        let path = foreign
            .iter()
            .rev()
            .find_map(|r| r.get("path").and_then(Value::as_str))
            .unwrap_or_default()
            .to_string();
        let file = vault.join(&path);
        let Some(meta) = crate::ids::read_meta(&file) else {
            // No local file: nothing to reconcile. The note's own text arrives below, if it came.
            continue;
        };
        // THIS device's records for the note — the `upstream_records` argument, because on this
        // machine the file on disk is upstream.
        //
        // **No `ts` floor here** (review I1, fix round 1). The brief kept only records at or after
        // the foreign chain's earliest `ts`, which drops this device's own, genuinely earlier record
        // for the field out of `up_latest` whenever the foreign write is later — exactly the case
        // `reconcile::resolve`'s "upstream never moved" branch exists to recognise correctly. Without
        // the real record, `resolve` falls back to a synthetic contender stamped with the file's raw
        // mtime, and an UNRELATED local edit to a different field of the same note bumps that mtime
        // past the foreign `ts`, so the older local value wins with no card and no warning — the two
        // desktops then stay different for good. `resolve` itself already limits a chain to `op ==
        // "set"` records and groups by field, so passing every record for the note, of any age, costs
        // nothing: a field the foreign side never touched simply has no chain to compare against.
        // B2 (re-review round 2, fix round 3): only the records THIS PULL ITSELF just appended —
        // `foreign`, identified by the hash of their own canonical bytes, the same identity `known`
        // uses above — not every record ever received from those devices. Round 2's fix excluded a
        // whole DEVICE, which also dropped that device's genuinely earlier records from a PRIOR
        // pull; when this device's own note value was explained by exactly one of those earlier
        // records (e.g. B's `2→5`, applied cleanly on an earlier pull), losing it from `mine` sent
        // `resolve` to the mtime stand-in again — reopening I1's silent loss through a second route.
        let foreign_hashes: std::collections::BTreeSet<String> = foreign
            .iter()
            .map(|r| sha256_hex(crate::ledger::dumps_value(&Value::Object(r.clone())).as_bytes()))
            .collect();
        // R1 (re-review round 3, fix round 4): **a foreign write that never took effect here never
        // decides a conflict here.** A `set` another desktop made whose `new` is not what the note
        // holds for that field did not land on this device — a card withheld it, or a later write
        // superseded it. B2's hash filter keeps such a record whenever it came down in an EARLIER
        // pull (the ordinary flow: B's value is carded and withheld, then B edits the field again).
        // Being later than this device's own record, it became `up_latest`; `resolve` only ever
        // takes `up_latest` as the contender when its `new` matches the note, so it fell to the
        // file-mtime stand-in instead, and one unrelated local edit made that stand-in win — B's
        // next write lost with no card. Dropping it leaves `up_latest` as the latest record that
        // explains the note's value, which is the contender `resolve` is asking for. Only another
        // device's records: when THIS device's own latest record disagrees with the note, the note
        // was edited by hand since, and the mtime stand-in is exactly the right contender for that.
        let never_took_effect = |r: &Record| {
            let field = r.get("field").and_then(Value::as_str).unwrap_or_default();
            r.get("op").and_then(Value::as_str) == Some("set")
                && !field.is_empty()
                && r.get("device").and_then(Value::as_str) != Some(this_device.as_str())
                && r.get("new").cloned().unwrap_or(Value::Null)
                    != crate::yaml::get(&meta, field).map(crate::yaml::to_json).unwrap_or(Value::Null)
        };
        let mine: Vec<Record> = journal
            .records_for(id, None)
            .into_iter()
            .filter(|r| {
                !foreign_hashes.contains(&sha256_hex(crate::ledger::dumps_value(&Value::Object(r.clone())).as_bytes()))
                    // Also drop this device's own SYNC echoes (review I1/M2): an echo carries a
                    // fresh `ts` for a value that is really the foreign write restated, so once a
                    // third desktop is in the mix an echo could out-rank a genuinely later write it
                    // is itself only a mirror of.
                    && r.get("actor").and_then(Value::as_str) != Some(ACTOR)
                    && !never_took_effect(r)
            })
            .collect();
        let mtime_ts = std::fs::metadata(&file)
            .and_then(|m| m.modified())
            .map(|t| crate::journal::now_ts(jiff::Timestamp::try_from(t).ok()))
            .unwrap_or_else(|_| crate::journal::now_ts(None));
        let resolution = crate::reconcile::resolve(&meta, &mine, foreign, &mtime_ts, Some(id), &path, ctx.via.as_str());

        // 5. **A carded field is withheld from `apply`: the card IS the write** (review C2). A field
        //    with a supersede record is a field both desktops moved; if a card could ever apply to
        //    it, writing the foreign value here would overwrite this device's and leave the card's
        //    `from` no longer matching the note, which `approvals::apply_amendment`'s from-check
        //    refuses as a "stale amendment" (`validate_amendment` checks only shape, amendability and
        //    writability). So the note keeps what this device had until the student answers.
        let folder = path.split('/').next().unwrap_or_default();
        let card_folder = crate::approvals::AMENDABLE_FOLDERS.contains(&folder);
        let superseded_fields: std::collections::BTreeSet<String> = resolution
            .supersede
            .iter()
            .filter_map(|r| r.get("field").and_then(Value::as_str).map(str::to_string))
            .collect();
        let cardable = |field: &str| card_folder && crate::approvals::AMENDABLE_FIELDS.contains(&field);

        let mut literals: Vec<(String, String)> = Vec::new();
        for (field, value) in &resolution.apply {
            if superseded_fields.contains(field) && cardable(field) {
                continue;
            }
            if superseded_fields.contains(field) {
                // A conflict no card could ever apply keeps reconcile's rule (later `ts` wins) and
                // is named, so the losing value is visible and nothing is dropped in silence.
                report.warnings.push(format!("sync: {path} — both machines set `{field}`; the later write won and the other is in the journal"));
            }
            // `write::to_literal` takes a **serde_yaml_ng** `Value` (`write.rs:18` imports `Value`
            // from `serde_yaml_ng`), and `Resolution::apply` holds **serde_json** values
            // (`reconcile.rs:24`). `yaml::from_json` is the one conversion, and it is the same one
            // `write_literals` does internally with `parse_literal` (review R3).
            literals.push((field.clone(), crate::write::to_literal(&crate::yaml::from_json(value))));
            report.applied += 1;
        }
        for record in &resolution.supersede {
            // The losing write, on the record. `JsonlLedger::append` files it by its own `ts`, like
            // every other record this function appends.
            if ledger.append(record).is_ok() {
                report.superseded += 1;
            }
        }
        if !literals.is_empty() {
            // `write_literals` journals, like every `write::` path function — so this produces one
            // `op: set` record per field under `ACTOR`, with this machine's `device_name()` and a
            // fresh `ts`, beside the foreign record already appended verbatim above. That echo is
            // correct history for THIS device and is exactly what `build_push`'s actor filter keeps
            // off the wire (review S1). It is also why the ruling covers every op rather than
            // `move`/`delete`: the field path is the common case and fires on nearly every pull.
            if let Err(e) = crate::write::write_literals(vault, &path, &literals, ctx, journal, &Default::default()) {
                report.warnings.push(format!("sync: {path} could not be written ({e})"));
            }
        }
        // **One card for the whole conflicting set, not one per field — and at most one live sync
        // card for whatever a pull resolves** (review D1, fix round 4).
        //
        // A carded field the foreign side WON (it is in `resolution.apply`) is either a real change
        // — `from` what the note holds now, because the field was withheld above, `to` the foreign
        // value — or it has CONVERGED: the note already holds `to` (review O4), the natural end of
        // every approved sync card, which proposes nothing and must spend no cap slot. A carded
        // field this device WON is neither: nothing changes here and there is nothing to ask, so it
        // resolves no card either (an earlier card on it may still carry a later offer from a
        // third desktop).
        //
        // **A cardable field this pull applied CLEANLY resolves a card too — for settling, never as
        // a change** (probe N13, fix round 5). Only a third desktop can reach it: this device's
        // card offers B's value over this device's own, then desktop C, which had this device's
        // value, writes the field again. `reconcile::resolve` sees "upstream never moved" and
        // applies C's value, so the card's `from` no longer matches the note and
        // `approvals::apply_amendment` would refuse it for ever ("stale amendment"). C's write is
        // already on the note, so there is nothing to propose in the card's place.
        //
        // **YAML values on both sides** (review R3): `propose_amendment`'s `changes` is
        // `&[(String, serde_yaml_ng::Value, serde_yaml_ng::Value)]`. `from` comes straight out of
        // the note's own frontmatter mapping — re-read after the write above, never round-tripped
        // through JSON — and `to` is the foreign value converted once.
        let now_meta = crate::ids::read_meta(&file).unwrap_or_else(|| meta.clone());
        let holds = |field: &str| crate::yaml::get(&now_meta, field).cloned().unwrap_or(serde_yaml_ng::Value::Null);
        let mut changes: Vec<(String, serde_yaml_ng::Value, serde_yaml_ng::Value)> = Vec::new();
        let mut resolved: std::collections::BTreeSet<String> = std::collections::BTreeSet::new();
        for (field, value) in resolution.apply.iter().filter(|(field, _)| cardable(field)) {
            resolved.insert(field.clone());
            if !superseded_fields.contains(field) {
                // Applied cleanly above: resolved for settling, never a change.
                continue;
            }
            let to = crate::yaml::from_json(value);
            let from = holds(field);
            if from != to {
                changes.push((field.clone(), from, to));
            }
        }
        if !resolved.is_empty() {
            // Every still-open card this device filed on this note that names ANY field this pull
            // resolved — converged, carded or applied cleanly — is settled, whatever else it names.
            // Keyed on one exact field set (rounds 1-3), a two-field card survived a pull that
            // resolved only one of its fields and went on offering a value neither desktop holds
            // (probes N10a, N10b), and a pull that grew the conflicting set filed a second card
            // beside the stale one (N10c). A card sync did not file (review R2), or that another
            // desktop filed and that travelled here as a note (N16), is never among these: see
            // `live_sync_cards`.
            let touching: Vec<(PathBuf, serde_yaml_ng::Mapping)> = live_sync_cards(vault, &path, &own_created)
                .into_iter()
                .filter(|(_, card)| card.keys().filter_map(crate::yaml::text).any(|f| resolved.contains(&f)))
                .collect();
            // What a settled card named that this pull did NOT resolve is still an open question, so
            // it is carried into the one fresh card rather than dropped with the card that held it —
            // **but only while the note still holds that card's own `from` for the field** (probes
            // N14, N14b; fix round 5). Once the note has moved on — the student answered the field
            // by hand, or a later write from a third desktop applied cleanly — the card's `to` is an
            // OLDER value than what the note now holds, and carrying it (re-based on the new value)
            // would re-offer it over that later write. The field is dropped instead: a conflict still
            // open on the other side comes back through that side's own records. A card whose
            // `from` equals its `to` proposes nothing and is dropped too.
            let mut carried: std::collections::BTreeMap<String, (serde_yaml_ng::Value, serde_yaml_ng::Value)> =
                std::collections::BTreeMap::new();
            for (_, card) in &touching {
                for (key, spec) in card {
                    let Some(field) = crate::yaml::text(key) else { continue };
                    if resolved.contains(&field) || carried.contains_key(&field) || !cardable(&field) {
                        continue;
                    }
                    let Some(spec) = spec.as_mapping() else { continue };
                    let (Some(card_from), Some(to)) = (crate::yaml::get(spec, "from"), crate::yaml::get(spec, "to")) else {
                        continue;
                    };
                    let from = holds(&field);
                    if crate::yaml::to_json(&from) == crate::yaml::to_json(card_from)
                        && crate::yaml::to_json(&from) != crate::yaml::to_json(to)
                    {
                        carried.insert(field, (from, to.clone()));
                    }
                }
            }
            changes.extend(carried.into_iter().map(|(field, (from, to))| (field, from, to)));
            changes.sort_by(|a, b| a.0.cmp(&b.0));
            // M3 / O3: a card that already carries exactly this proposal — the same fields, the same
            // `from`, the same `to` — IS the re-proposal. It is kept as it stands (a snoozed one stays
            // snoozed), so an identical re-pull costs no second unit of the day's cap (probe N4).
            let wanted = changes_json(&changes);
            let keep = if changes.is_empty() {
                None
            } else {
                touching
                    .iter()
                    .position(|(_, card)| crate::yaml::to_json(&serde_yaml_ng::Value::Mapping(card.clone())) == wanted)
            };
            for (i, (card_path, card)) in touching.iter().enumerate() {
                if Some(i) == keep {
                    continue;
                }
                // Settled whole through `write::delete` — journalled, archived, never unlinked. The
                // card's `changes:` is a block mapping, and editing a value inside it would be a
                // parse and a re-dump, which no note ever gets.
                let names = card.keys().filter_map(crate::yaml::text).collect::<Vec<String>>().join("`, `");
                match crate::write::delete(vault, &crate::ids::rel(vault, card_path), ctx, journal) {
                    Err(e) => report.warnings.push(format!(
                        "sync: {path} — a stale amend card for `{names}` could not be settled ({e})"
                    )),
                    Ok(_) if changes.is_empty() => report.warnings.push(format!(
                        "sync: {path} — the amend card for `{names}` is settled: a later write already answers it"
                    )),
                    Ok(_) => {}
                }
            }
            if !changes.is_empty() && keep.is_none() {
                match crate::write::propose_amendment(vault, &file, &now_meta, &changes, ctx, journal, None, today) {
                    // One card, however many fields it carries — the fifteen-a-day cap the deck
                    // already applies counts cards, and so does this.
                    Ok(_) => report.cards += 1,
                    Err(e) => report.warnings.push(format!("sync: {path} — the amend card could not be filed ({e})")),
                }
            }
        }
    }

    // 6. The pulled note texts. **The second recorded exception**, bounded here and nowhere else.
    //
    // O1 (re-review round 2): a case-only rename made on the OTHER desktop pushes BOTH a live row
    // under the new spelling and a tombstone for the old one — `build_push`'s own note pass always
    // emits that pair for a rename, in either order. Comparing every tombstone in THIS page against
    // every LIVE row in this page, case-insensitively, is what tells that pair apart from a genuine
    // delete (which carries no live row at all); a real delete is untouched by this. Precomputed
    // once, over the whole page, so both wire orders land the same way.
    //
    // B3 (re-review round 2, fix round 3): a case-insensitive name match is not, on its own, proof
    // of identity — the other desktop can delete one note and separately create an unrelated one
    // whose name happens to collide with the deleted note's, case only. Both halves below are
    // gated on the pulled note's own `id:` equalling the LOCAL file's `id:`, read directly off each
    // side's frontmatter (never re-dumped — this is a read, exactly like `ids::read_meta` already
    // does for an on-disk note). Only when the ids agree is this treated as one rename event.
    //
    // **Tombstones before live rows, always** (B3): when the ids do NOT agree, both halves fall
    // through to their ordinary, non-O1 behaviour — the tombstone settles its own note into
    // `archive/`, and the live row is a genuinely new note. On NTFS those two ordinary operations
    // still collide if the live row's `create_new` runs before the tombstone's `write::delete` has
    // cleared the old, case-colliding name out of the folder. Processing every tombstone first,
    // regardless of the order `page.notes` carries them in, is what makes the new note's
    // `create_new` land after the archive has already happened.
    let live_by_lower: std::collections::BTreeMap<String, &PulledNote> = page
        .notes
        .iter()
        .filter(|n| n.text.is_some())
        .map(|n| (n.path.to_lowercase(), n))
        .collect();
    let ordered_notes: Vec<&PulledNote> = page
        .notes
        .iter()
        .filter(|n| n.text.is_none())
        .chain(page.notes.iter().filter(|n| n.text.is_some()))
        .collect();
    for note in ordered_notes {
        if !is_note_path(vault, &note.path) {
            report.refused += 1;
            report.warnings.push("sync: a pulled note named a path outside the vault's notes".to_string());
            continue;
        }
        let file = vault.join(&note.path);
        match &note.text {
            // A tombstone settles the note through `write::delete`, which moves it to `archive/` and
            // journals the move. Nothing in this vault is ever unlinked. **Exact case** (review I3):
            // `sync-pull` returns every row for the account with no device filter, so a case-only
            // local rename can pull back its OWN tombstone for the old spelling; `file.exists()`
            // answers case-insensitively on NTFS and would archive the still-live, differently-cased
            // note. `exact_case_exists` answers at the byte level a directory listing would.
            None => {
                let mut renamed_elsewhere = false;
                if let Some(live_note) = live_by_lower.get(&note.path.to_lowercase()) {
                    if live_note.path != note.path {
                        let local_id = crate::ids::read_meta(&file)
                            .and_then(|m| crate::yaml::get(&m, "id").and_then(crate::yaml::text));
                        let live_id = live_note.text.as_deref().and_then(note_frontmatter_id);
                        if local_id.is_some() && local_id == live_id {
                            // O1: this tombstone and a live row in the SAME page name the same
                            // note (same id), cased differently — a rename, not a deletion. The
                            // live row's own handling below performs the actual rename; this
                            // tombstone names nothing left to settle and is dropped, never
                            // archived.
                            report.warnings.push(format!(
                                "sync: {} — a case-only rename to {}, not an archive", note.path, live_note.path
                            ));
                            renamed_elsewhere = true;
                        }
                        // B3: a case-insensitive collision with a DIFFERENT id is not a rename —
                        // two unrelated notes just happen to share a spelling. Fall through to the
                        // ordinary settle below.
                    }
                }
                if renamed_elsewhere {
                    continue;
                }
                if exact_case_exists(&file) {
                    match crate::write::delete(vault, &note.path, ctx, journal) {
                        Ok(_) => report.moved += 1,
                        Err(e) => report.warnings.push(format!("sync: {} could not be settled ({e})", note.path)),
                    }
                }
            }
            Some(text) => {
                // **Never overwrites a note this device already has — the one recorded exception to
                // "every write goes through `write::`".** Exactly two bounds, not the plan's original
                // three (review M1: its third, "the note's own `create` record must already have
                // been appended", does not hold in general — a hand-made note on disk has no record
                // at all, and an amend card `apply` files is journalled under `sync::ACTOR`, which
                // `build_push`'s echo filter never sends, so the OTHER desktop would never see that
                // record either). The path must already be `is_note_path` (checked above, for every
                // note in this loop), and no file of that EXACT name — case included (review I3) —
                // may already sit at it. `create_new` (review M6) closes the remaining
                // check-then-write race: this can never overwrite a file that appears between the
                // check above and this write, even one from a concurrent console `write` command.
                if exact_case_exists(&file) {
                    continue;
                }
                // O1: a case-insensitive match that is NOT the exact spelling MIGHT be this
                // device's own copy of the SAME note, cased differently — the other half of the
                // rename pair above. Renaming this device's file keeps its own id and body
                // untouched; any foreign `move` record for the note is journalled in the ordinary
                // record pass above (and, on NTFS, fails there at its own case-insensitive
                // `exists()` check — by design, see the moves pass — leaving the rename to land
                // here instead). B3: only when the ids agree — otherwise this is a genuinely new,
                // unrelated note that merely collides in spelling, and it falls through to the
                // ordinary `create_new` below (which, because tombstones ran first, no longer
                // collides with anything).
                if let Some(existing) = case_insensitive_match(&file) {
                    let local_id = crate::ids::read_meta(&existing)
                        .and_then(|m| crate::yaml::get(&m, "id").and_then(crate::yaml::text));
                    let live_id = note_frontmatter_id(text);
                    if local_id.is_some() && local_id == live_id {
                        match rename_case_only(&existing, &file) {
                            Ok(()) => report.moved += 1,
                            Err(e) => report.warnings.push(format!(
                                "sync: {} could not be renamed to match the account's spelling ({e}); keeping the old spelling",
                                note.path
                            )),
                        }
                        continue;
                    }
                }
                if let Some(parent) = file.parent() {
                    let _ = std::fs::create_dir_all(parent);
                }
                let translated = if crate::pystr::NEWLINE == "\n" {
                    text.clone()
                } else {
                    crate::pystr::universal_newlines(text).replace('\n', crate::pystr::NEWLINE)
                };
                let result = std::fs::OpenOptions::new()
                    .write(true)
                    .create_new(true)
                    .open(&file)
                    .and_then(|mut f| f.write_all(translated.as_bytes()));
                match result {
                    Ok(()) => report.notes_written += 1,
                    Err(e) => report.warnings.push(format!("sync: {} could not be written ({e})", note.path)),
                }
            }
        }
    }
    report
}

// ---------------------------------------------------------------------------
// The command (C3' Task 7): `sync::run_lines`, always exit 0, one line each.
// ---------------------------------------------------------------------------

/// Which halves of a sync this run does. The slot runs `Both`; the console's *Sync now* runs `Both`;
/// the two are separable for a smoke test and for a quit flush that must not pull.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Direction { Pull, Push, Both }

impl Direction {
    pub fn parse(word: &str) -> Option<Direction> {
        match word {
            "pull" => Some(Direction::Pull),
            "push" => Some(Direction::Push),
            "both" => Some(Direction::Both),
            _ => None,
        }
    }
    fn pulls(self) -> bool { matches!(self, Direction::Pull | Direction::Both) }
    fn pushes(self) -> bool { matches!(self, Direction::Push | Direction::Both) }
}

/// What one run did, for the page and the report. Counts and words; never a path or a value.
#[derive(Debug, Clone, Default)]
pub struct Totals {
    pub pulled_records: usize,
    pub pulled_notes: usize,
    pub applied: usize,
    pub cards: usize,
    pub superseded: usize,
    pub pushed_records: usize,
    pub pushed_notes: usize,
    pub more: bool,
    /// A normal state the student can act on — "no account", "no session". **Not an error.**
    pub skipped: Option<String>,
    /// Something went wrong that a later slot may fix. `SyncStatus::ok` is `errors.is_empty()`.
    pub errors: Vec<String>,
}

/// What the console's sync line renders. The ENGINE's type (hand-off H9a): the engine produces it,
/// and a second struct in the app would be a second thing to keep in step with the run that fills it.
#[derive(Debug, Clone, Default, serde::Serialize)]
pub struct SyncStatus {
    pub ok: bool,
    pub at: Option<String>,
    pub lines: Vec<String>,
    pub last_error: Option<String>,
}

impl SyncStatus {
    /// **A skip is not an error.** A student who has not signed in must not paint the tray amber;
    /// a 5xx should. `ok` is therefore `errors.is_empty()`, and `at` is stamped whatever happened,
    /// because "we tried at 07:02 and there was no account" is still a fact the page should show.
    pub fn of(totals: &Totals, lines: Vec<String>) -> SyncStatus {
        SyncStatus {
            ok: totals.errors.is_empty(),
            at: Some(crate::journal::now_ts(None)),
            lines,
            last_error: totals.errors.first().cloned(),
        }
    }
}

/// Does this vault have an account at all? `state::quit_flush` asks before pushing.
pub fn is_configured(vault: &Path) -> bool {
    crate::cloudmodel::load(vault).is_some()
}

/// The whole command. **The return code is always 0** and the function says so by construction: it
/// is the literal `0` in the one `return` and the one tail.
///
/// **The order of the first two checks is the message** (review I4). `load` before `resolve`: the
/// common answer on a machine that has never signed in is "no account", and a student who is merely
/// signed out must not read a sentence about their subscription. It also saves a Credential Manager
/// read twice a day on every vault that has no account.
pub fn run_lines_with(
    vault: &Path,
    direction: Direction,
    via: &str,
    run_id: Option<&str>,
) -> (i32, Vec<String>, Totals) {
    let mut lines = Vec::new();
    let mut totals = Totals::default();

    // 1. No account. Every fixture vault in this repository takes this path, which is why
    //    `oracle.rs` and `surface_oracle.rs` cannot move.
    //
    //    **The config is kept, not discarded** (review R2): `CloudClient` is `{ base, anon_key,
    //    token, agent }` and has no `account_id()` accessor — the account id is `CloudConfig`'s
    //    field. `cloudmodel.rs` is outside this stream's ownership, so the value travels from here
    //    rather than through an accessor added for one caller.
    let Some(cfg) = crate::cloudmodel::load(vault) else {
        totals.skipped = Some("no account".to_string());
        lines.push("sync (skipped: no account)".to_string());
        return (0, lines, totals);
    };
    // 2. No session. C1 owns the refresh; this only reads, and waits for the app's next slot.
    let client = match crate::cloudmodel::resolve(vault) {
        Ok(c) => c,
        Err(e) => {
            totals.skipped = Some(e.label().to_string());
            lines.push(format!("sync (skipped: {e})"));
            return (0, lines, totals);
        }
    };

    let mut cursor = load_cursor(vault);
    let mut journal = Journal::new(vault);
    let ctx = crate::write::WriteContext { actor: ACTOR.to_string(), via: via.to_string(), run_id: run_id.map(str::to_string) };
    let today = crate::journal::now_ts(None)[..10].parse::<jiff::civil::Date>().unwrap_or(jiff::civil::date(1970, 1, 1));

    // 3. **Pull first.** A field another desktop set this morning must be in the note before the day
    //    is ordered, or every second desktop ranks a slot behind for ever.
    if direction.pulls() {
        match pull(&client, cursor.record_cursor, cursor.note_cursor) {
            Ok(page) => {
                let report = apply(vault, &page, &ctx, &mut journal, today);
                totals.pulled_records = page.records.len();
                totals.pulled_notes = page.notes.len();
                totals.applied = report.applied;
                totals.cards = report.cards;
                totals.superseded = report.superseded;
                totals.more = page.more;
                lines.extend(report.warnings.iter().cloned());
                lines.push(format!(
                    "sync: {} record(s) and {} note(s) down; {} applied, {} card(s), {} refused",
                    totals.pulled_records, totals.pulled_notes, report.applied, report.cards, report.refused
                ));
                cursor.record_cursor = page.record_cursor;
                cursor.note_cursor = page.note_cursor;
                if let Err(e) = save_cursor(vault, &cursor) {
                    totals.errors.push(e.label().to_string());
                    lines.push(format!("sync: the cursor could not be saved ({e})"));
                }
            }
            Err(e) => {
                totals.errors.push(e.label().to_string());
                lines.push(format!("sync ({e})"));
            }
        }
    }

    // 4. **Push second, and the cursor moves only on a 200.** A cursor advanced over a batch the
    //    service never received is the one bug in this module that loses a record for good.
    if direction.pushes() {
        let (batch, next) = build_push(vault, &cursor, &cfg.account_id, &mut journal);
        lines.extend(batch.warnings.iter().cloned());
        match push(&client, &batch) {
            Ok((records, notes)) => {
                totals.pushed_records = records;
                totals.pushed_notes = notes;
                lines.push(format!("sync: {records} record(s) and {notes} note(s) up"));
                if let Err(e) = save_cursor(vault, &next) {
                    totals.errors.push(e.label().to_string());
                    lines.push(format!("sync: the cursor could not be saved ({e})"));
                }
            }
            // 5. Every refusal is one line and **is not a skip**: `sync (no network)` and
            //    `sync (no entitlement)` are things that happened, not states the student chose.
            Err(e) => {
                totals.errors.push(e.label().to_string());
                lines.push(format!("sync ({e})"));
            }
        }
    }

    // 6. A term of catching up is visible rather than silent.
    if totals.more {
        lines.push("sync: more to come — the next slot continues".to_string());
    }
    (0, lines, totals)
}

/// The printing twin, and one line of it.
pub fn run_lines(vault: &Path, direction: Direction, via: &str, run_id: Option<&str>) -> (i32, Vec<String>) {
    let (code, lines, _) = run_lines_with(vault, direction, via, run_id);
    (code, lines)
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
