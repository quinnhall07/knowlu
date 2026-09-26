//! Port of `engine/ids.py` — stable ids for every note.
//!
//! `id` is the **internal** key (journal, issues, dashboard). `source_uid` stays the **external**
//! key for ingest dedup. The two never merge, and conflating them would make a vendor's identifier
//! load-bearing for the journal.

use std::collections::{BTreeMap, BTreeSet};
use std::path::{Component, Path, PathBuf};
use std::sync::LazyLock;

use regex::Regex;
use serde_yaml_ng::Mapping;
use sha1::{Digest, Sha1};

use crate::ingest::{update_frontmatter_fields, IngestError};
use crate::models::split_frontmatter;
use crate::pystr;

pub const KINDS: [&str; 5] = ["task", "appr", "info", "iss", "course"];
pub const NOTE_FOLDERS: [&str; 6] =
    ["tasks", "approvals", "archive", "courses", "issues", "info"];

pub static ID_RE: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"^(task|appr|info|iss|course)_[0-9a-f]{10}$").unwrap());

pub fn is_id(text: &str) -> bool {
    ID_RE.is_match(text)
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum IdError {
    EscapesVault(String),
    UnknownId(String),
    NoSuchNote(String),
}

impl std::fmt::Display for IdError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            IdError::EscapesVault(p) => write!(f, "target escapes the vault: {p}"),
            IdError::UnknownId(t) => write!(f, "unknown id: {t}"),
            IdError::NoSuchNote(t) => write!(f, "no such note: {t}"),
        }
    }
}

/// Vault-relative, POSIX-separated. This string is the id input, so its separator matters.
pub fn rel(vault: &Path, path: &Path) -> String {
    path.strip_prefix(vault)
        .unwrap_or(path)
        .to_string_lossy()
        .replace('\\', "/")
}

/// Path-derived so both machines running the migration agree.
///
/// SHA-1 of the POSIX-normalised relative path, first **10 hex characters**. Not a security
/// primitive — a stable, machine-independent identity. Changing the hash would re-id every note.
pub fn derived_id(kind: &str, rel_path: &str) -> String {
    let mut hasher = Sha1::new();
    hasher.update(rel_path.replace('\\', "/").as_bytes());
    let digest = hasher.finalize();
    let hex: String = digest.iter().map(|b| format!("{b:02x}")).collect();
    format!("{kind}_{}", &hex[..10])
}

/// `secrets.token_hex(5)` — 5 random bytes, 10 hex characters.
pub fn new_id(kind: &str) -> String {
    let mut bytes = [0u8; 5];
    getrandom::getrandom(&mut bytes).expect("OS entropy unavailable");
    let hex: String = bytes.iter().map(|b| format!("{b:02x}")).collect();
    format!("{kind}_{hex}")
}

/// Two-desktop design D1: the id of an imported note, derived from the item so that two computers
/// importing one item mint one id. The same shape as every id (`kind_` + 10 lowercase hex, `ID_RE`)
/// and `derived_id`'s SHA-1 and cut — an identity, not a security primitive. The first line of the
/// hashed text separates this hash from `derived_id`'s and versions it. **Frozen once shipped**: a
/// second derivation would re-open doubles between computers on different builds.
pub fn import_id(kind: &str, vendor: &str, key: &str) -> String {
    let mut hasher = Sha1::new();
    hasher.update(format!("knowlu/import-id/1\n{kind}\n{vendor}\n{key}").as_bytes());
    let hex: String = hasher.finalize().iter().map(|b| format!("{b:02x}")).collect();
    format!("{kind}_{}", &hex[..10])
}

/// The note's kind, from its `type:` field, falling back to its folder.
///
/// Note the asymmetry: `approval` and `issue` are abbreviated (`appr`, `iss`) but `info` is not.
/// Only `course` is decided by folder; everything unrecognised is a task.
pub fn kind_for(path: &Path, meta: Option<&Mapping>) -> String {
    let declared = meta
        .and_then(|m| crate::yaml::get(m, "type"))
        .and_then(crate::yaml::text)
        .unwrap_or_default();
    match declared.as_str() {
        "approval" => return "appr".to_string(),
        "issue" => return "iss".to_string(),
        "info" => return "info".to_string(),
        _ => {}
    }
    if path.parent().and_then(|p| p.file_name()).map(|n| n == "courses") == Some(true) {
        return "course".to_string();
    }
    "task".to_string()
}

/// D2: the `created_by:` words whose notes an automatic step creates because an outside item exists —
/// the notes two desktops create independently. A closed list: a person, the console, an agent's
/// judgment and every other word keep `new_id`.
pub const IMPORT_VENDORS: [&str; 7] = ["zybooks", "vhl", "blackboard", "gmail", "events", "coursework", "rules"];

/// `(kind, vendor, key)` — the three inputs of [`import_id`].
pub type ImportKey = (String, String, String);

/// D2: whether a note is imported, and under which key — the one function that answers it (§2.2).
///
/// Only kinds `task` and `appr` ([`kind_for`], so the prefix is the one `create` would have minted),
/// only a `created_by:` in [`IMPORT_VENDORS`], and the key is the producer's word, a colon, and the
/// note's `source_uid:` or, where a producer has none, its own key. Every input is frontmatter the
/// producer writes from the item itself; none is per-desktop. The map card's and the digest's keys
/// carry their date, because each is re-proposed into a free path and must not take the old card's id.
pub fn import_key(path: &Path, meta: &Mapping) -> Option<ImportKey> {
    let kind = kind_for(path, Some(meta));
    if kind != "task" && kind != "appr" {
        return None;
    }
    let field = |name: &str| crate::yaml::get(meta, name).and_then(crate::yaml::text).filter(|v| !v.is_empty());
    let vendor = field("created_by")?;
    if !IMPORT_VENDORS.contains(&vendor.as_str()) {
        return None;
    }
    let card = field("kind");
    let (word, rest) = match (vendor.as_str(), kind.as_str(), card.as_deref()) {
        ("zybooks" | "vhl", "task", _) => ("coursework", field("source_uid")?),
        ("blackboard", "task", _) => ("lms", field("source_uid")?),
        ("gmail", "task" | "appr", _) => ("gmail", field("source_uid")?),
        ("events", "appr", Some("calendar-event")) => ("calendar-event", field("source_uid")?),
        ("events", "appr", Some("events-digest")) => ("events-digest", field("proposed_at")?),
        ("coursework", "appr", Some("coursework-map")) => {
            ("map", format!("{}:{}:{}", field("source")?, field("map_key")?, field("first_proposed_at")?))
        }
        ("rules", "appr", Some("rule")) => ("rule", field("rule_id")?),
        _ => return None,
    };
    Some((kind, vendor, format!("{word}:{rest}")))
}

/// D3's `held`: every id this vault already holds — [`build_index`]'s keys, read once per producer
/// run and extended by `write::create_imported` as the run creates.
pub fn held_ids(vault: &Path) -> BTreeSet<String> {
    build_index(vault).into_keys().collect()
}

/// Two-desktop design D6: `{loser id: winner id}` for every alias group — one imported item under
/// two or more ids. Generated and device-local; never synced (`sync::build_push` reads only the note
/// folders); rebuilt by every `sync::apply`. A lost file costs one more full reconcile, never a wrong
/// write (re-review m2).
pub const ALIASES_FILE: &str = "state/id-aliases.json";

/// Missing or unreadable is `{}`, never an error: a vault with no doubles has no groups.
pub fn load_aliases(vault: &Path) -> BTreeMap<String, String> {
    pystr::read_text(&vault.join(ALIASES_FILE))
        .ok()
        .and_then(|text| serde_json::from_str(&text).ok())
        .unwrap_or_default()
}

/// Through `ledger::dumps_value`, and a temp file renamed into place, as `sync`'s other generated
/// files are written, so a crash never leaves half a file.
pub fn save_aliases(vault: &Path, aliases: &BTreeMap<String, String>) -> Result<(), String> {
    let path = vault.join(ALIASES_FILE);
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent).map_err(|e| e.to_string())?;
    }
    let value = serde_json::to_value(aliases).map_err(|e| e.to_string())?;
    let tmp = path.with_extension("tmp");
    pystr::write_text(&tmp, &crate::ledger::dumps_value(&value)).map_err(|e| e.to_string())?;
    std::fs::rename(&tmp, &path).map_err(|e| e.to_string())
}

/// The winner of `id`'s group, or `id` itself when it is in none.
pub fn canonical<'a>(aliases: &'a BTreeMap<String, String>, id: &'a str) -> &'a str {
    aliases.get(id).map(String::as_str).unwrap_or(id)
}

/// Every id of `id`'s group, the winner included; `{id}` when it is in none.
pub fn alias_group(aliases: &BTreeMap<String, String>, id: &str) -> BTreeSet<String> {
    let winner = canonical(aliases, id).to_string();
    let mut group: BTreeSet<String> =
        aliases.iter().filter(|(_, w)| **w == winner).map(|(loser, _)| loser.clone()).collect();
    group.insert(winner);
    group
}

/// D6's input: every import key, and the ids that name it. From every `create` record first — a
/// `create` record's `new` is the whole frontmatter as minted (`write.rs:416-418`), immutable, so a
/// later hand edit of `source_uid` or `created_by` never moves a note between keys — and from a note
/// on disk only when its id has no `create` record (re-review m3).
pub fn import_ids_by_key(
    records: &[crate::ledger::Record],
    notes: &[(PathBuf, Option<Mapping>)],
) -> BTreeMap<ImportKey, BTreeSet<String>> {
    let mut keys: BTreeMap<ImportKey, BTreeSet<String>> = BTreeMap::new();
    let mut created: BTreeSet<String> = BTreeSet::new();
    for record in records {
        let field = |name: &str| record.get(name).and_then(serde_json::Value::as_str).unwrap_or_default();
        if field("op") != "create" || !is_id(field("id")) {
            continue;
        }
        created.insert(field("id").to_string());
        let minted = crate::yaml::from_json(record.get("new").unwrap_or(&serde_json::Value::Null));
        let serde_yaml_ng::Value::Mapping(meta) = minted else { continue };
        if let Some(key) = import_key(Path::new(field("path")), &meta) {
            keys.entry(key).or_default().insert(field("id").to_string());
        }
    }
    for (path, meta) in notes {
        let Some(meta) = meta else { continue };
        let Some(id) = crate::yaml::get(meta, "id").and_then(crate::yaml::text).filter(|id| is_id(id)) else {
            continue;
        };
        if created.contains(&id) {
            continue;
        }
        if let Some(key) = import_key(path, meta) {
            keys.entry(key).or_default().insert(id);
        }
    }
    keys
}

/// D6: a key with two or more ids is an alias group, and its lowest id as a string wins. Both desktops
/// compute the same group and winner with no coordination and no knowledge of which id is derived.
pub fn aliases_from(keys: &BTreeMap<ImportKey, BTreeSet<String>>) -> BTreeMap<String, String> {
    let mut out = BTreeMap::new();
    for ids in keys.values().filter(|ids| ids.len() > 1) {
        let mut ids = ids.iter();
        let Some(winner) = ids.next() else { continue };
        for loser in ids {
            out.insert(loser.clone(), winner.clone());
        }
    }
    out
}

/// Frontmatter, or `None` for anything unreadable — a missing file, bad YAML, or a non-mapping.
/// Never raises: the caller logs and moves on, which is the same rule ingest uses.
pub fn read_meta(path: &Path) -> Option<Mapping> {
    let text = pystr::read_text(path).ok()?;
    if !text.starts_with("---") {
        return None;
    }
    split_frontmatter(&text).ok().map(|(meta, _)| meta)
}

/// Every note in the six note folders, sorted by vault-relative path.
pub fn scan_notes(vault: &Path) -> Vec<(PathBuf, Option<Mapping>)> {
    let mut found: Vec<(PathBuf, Option<Mapping>)> = Vec::new();
    for folder in NOTE_FOLDERS {
        let Ok(entries) = std::fs::read_dir(vault.join(folder)) else { continue };
        for entry in entries.filter_map(|e| e.ok()) {
            let path = entry.path();
            if path.extension().map(|x| x == "md") == Some(true) {
                let meta = read_meta(&path);
                found.push((path, meta));
            }
        }
    }
    found.sort_by(|a, b| rel(vault, &a.0).cmp(&rel(vault, &b.0)));
    found
}

/// id -> path, first writer wins.
pub fn build_index(vault: &Path) -> BTreeMap<String, PathBuf> {
    let mut index = BTreeMap::new();
    for (path, meta) in scan_notes(vault) {
        let Some(meta) = meta else { continue };
        let Some(note_id) = crate::yaml::get(&meta, "id").and_then(crate::yaml::text) else {
            continue;
        };
        if is_id(&note_id) {
            index.entry(note_id).or_insert(path);
        }
    }
    index
}

/// Callback shape of `update_frontmatter_fields`, so tests can inject a failing writer.
pub type Writer<'a> = &'a mut dyn FnMut(&Path, &[(String, String)]) -> Result<(), IngestError>;

/// Give every readable note an id; re-id the later path of a duplicate pair.
///
/// Adding an id is the **only safe automatic repair** — it never changes meaning. Unreadable notes
/// are logged and left alone, the same rule ingest uses. The log strings are part of the contract:
/// the runner prints them and `state/runner-log.md` carries them.
pub fn ensure_ids(vault: &Path, writer: Option<Writer<'_>>) -> Vec<String> {
    match writer {
        Some(write) => ensure_ids_with(vault, write),
        None => {
            let mut default_writer = |path: &Path, changes: &[(String, String)]| {
                update_frontmatter_fields(path, changes)
            };
            ensure_ids_with(vault, &mut default_writer)
        }
    }
}

fn ensure_ids_with(vault: &Path, write: Writer<'_>) -> Vec<String> {
    let mut log: Vec<String> = Vec::new();
    let mut seen: BTreeMap<String, PathBuf> = BTreeMap::new();

    for (path, meta) in scan_notes(vault) {
        let relative = rel(vault, &path);
        let Some(meta) = meta else {
            log.push(format!("unreadable: {relative}"));
            continue;
        };
        let kind = kind_for(&path, Some(&meta));
        let existing = crate::yaml::get(&meta, "id").and_then(crate::yaml::text);

        let note_id = match existing {
            Some(id) if is_id(&id) => {
                if let Some(first) = seen.get(&id) {
                    let fresh = new_id(&kind);
                    if write(&path, &[("id".to_string(), fresh.clone())]).is_err() {
                        log.push(format!("unwritable: {relative}"));
                        continue;
                    }
                    // The em dash is Python's; the runner log carries this string verbatim.
                    log.push(format!(
                        "duplicate {id} ({}) — {relative} re-idd {fresh}",
                        rel(vault, first)
                    ));
                    fresh
                } else {
                    id
                }
            }
            _ => {
                let derived = derived_id(&kind, &relative);
                if write(&path, &[("id".to_string(), derived.clone())]).is_err() {
                    log.push(format!("unwritable: {relative}"));
                    continue;
                }
                log.push(format!("assigned {derived} to {relative}"));
                derived
            }
        };
        seen.insert(note_id, path);
    }
    log
}

/// Resolve a path the way Python's non-strict `Path.resolve()` does — it must work on a path that
/// **does not exist yet**, because `create` checks its target before writing it.
///
/// Canonicalises the deepest existing ancestor (so symlinks are followed, as in Python) and then
/// applies the remaining components lexically, honouring `..`.
pub(crate) fn resolve_lenient(path: &Path) -> PathBuf {
    let absolute = if path.is_absolute() {
        path.to_path_buf()
    } else {
        std::env::current_dir().unwrap_or_default().join(path)
    };
    if let Ok(real) = std::fs::canonicalize(&absolute) {
        return real;
    }
    let mut tail: Vec<Component> = Vec::new();
    let mut probe = absolute.as_path();
    loop {
        if let Ok(real) = std::fs::canonicalize(probe) {
            let mut out = real;
            for comp in tail.iter().rev() {
                match comp {
                    Component::ParentDir => {
                        out.pop();
                    }
                    Component::CurDir => {}
                    other => out.push(other.as_os_str()),
                }
            }
            return out;
        }
        match probe.parent() {
            Some(parent) => {
                if let Some(name) = probe.components().next_back() {
                    tail.push(name);
                }
                probe = parent;
            }
            None => return absolute,
        }
    }
}

/// Refuse a path that resolves outside the vault (`..`, symlinks, absolute escapes).
///
/// Every write-path entry point funnels through here so a target string never reaches the
/// filesystem unclamped.
pub fn inside_vault(vault: &Path, path: &Path) -> Result<PathBuf, IdError> {
    let root = resolve_lenient(vault);
    let target = resolve_lenient(path);
    if target.starts_with(&root) {
        Ok(path.to_path_buf())
    } else {
        Err(IdError::EscapesVault(path.display().to_string()))
    }
}

/// Accept either an opaque `id` or a vault-relative path, and return the note's path.
pub fn resolve_target(vault: &Path, target: &str) -> Result<PathBuf, IdError> {
    if is_id(target) {
        let index = build_index(vault);
        let path = match index.get(target) {
            Some(path) => path.clone(),
            None => {
                let aliases = load_aliases(vault);
                index
                    .get(canonical(&aliases, target))
                    .or_else(|| alias_group(&aliases, target).iter().find_map(|id| index.get(id)))
                    .cloned()
                    .ok_or_else(|| IdError::UnknownId(target.to_string()))?
            }
        };
        return inside_vault(vault, &path);
    }
    let path = inside_vault(vault, &vault.join(target))?;
    if !path.is_file() {
        return Err(IdError::NoSuchNote(target.to_string()));
    }
    Ok(path)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn vault() -> PathBuf {
        let dir = std::env::temp_dir().join(format!(
            "qo-ids-{}-{:?}",
            std::process::id(),
            std::thread::current().id()
        ));
        let _ = std::fs::remove_dir_all(&dir);
        for folder in NOTE_FOLDERS {
            std::fs::create_dir_all(dir.join(folder)).unwrap();
        }
        dir
    }

    fn note(vault: &Path, rel_path: &str, body: &str) -> PathBuf {
        let path = vault.join(rel_path);
        pystr::write_text(&path, body).unwrap();
        path
    }

    /// Reference-output test. Every value below came from running the **Python** `derived_id`,
    /// not from the Rust. These ids are already stamped on notes in the live vault, so a different
    /// digest here would silently re-id the entire vault on the next `ensure_ids` pass.
    #[test]
    fn derived_id_reproduces_pythons_sha1_exactly() {
        assert_eq!(derived_id("task", "tasks/cs-100-hw-01.md"), "task_3d9521e0b7");
        assert_eq!(derived_id("course", "courses/gn-103.md"), "course_994d1bcaba");
        assert_eq!(derived_id("appr", "approvals/amend-ph-106-due.md"), "appr_531b68d643");
        // Backslashes are normalised before hashing, so Windows and POSIX agree on the id.
        assert_eq!(derived_id("task", r"tasks\cs-100-hw-01.md"), "task_3d9521e0b7");
    }

    /// Two-desktop design §2.1: the four reference values, computed outside Rust (Python's `hashlib`
    /// and `sha1sum`). Frozen once shipped (D1): a different digest here would re-open doubles between
    /// two computers on different builds.
    #[test]
    fn import_id_reproduces_the_spec_reference_values() {
        assert_eq!(import_id("task", "zybooks", "coursework:zybooks:1839992"), "task_18734fe8b7");
        assert_eq!(
            import_id("task", "blackboard", "lms:_blackboard.platform.gradebook2.GradableItem-_4732722_1"),
            "task_d8891a504b"
        );
        assert_eq!(import_id("task", "gmail", "gmail:gmail:m1"), "task_3bc4bec4a9");
        assert_eq!(import_id("appr", "events", "events-digest:2026-09-25"), "appr_40ab7c3a10");
        assert!(is_id(&import_id("task", "vhl", "coursework:vhl:1")), "the shape of every id (ID_RE)");
    }

    fn key_of(rel: &str, frontmatter: &str) -> Option<ImportKey> {
        import_key(Path::new(rel), &crate::yaml::mapping_of(frontmatter))
    }

    fn want(kind: &str, vendor: &str, key: &str) -> Option<ImportKey> {
        Some((kind.to_string(), vendor.to_string(), key.to_string()))
    }

    /// §2.2: one row per producer, each key prefixed by its producer's word, so no two producers can
    /// share a key (review M14).
    #[test]
    fn import_key_names_each_producer_by_its_own_word() {
        assert_eq!(
            key_of("tasks/cs-100-hw-01.md", "created_by: zybooks\nsource_uid: \"zybooks:1839992\""),
            want("task", "zybooks", "coursework:zybooks:1839992")
        );
        assert_eq!(
            key_of("archive/gn-103-hw.md", "created_by: vhl\nsource_uid: \"vhl:1:2026-08-28\"\nstatus: archived"),
            want("task", "vhl", "coursework:vhl:1:2026-08-28")
        );
        assert_eq!(
            key_of("tasks/cs-100-quiz.md", "created_by: blackboard\nsource_uid: \"_blackboard.platform.gradebook2.GradableItem-_4732722_1\""),
            want("task", "blackboard", "lms:_blackboard.platform.gradebook2.GradableItem-_4732722_1")
        );
        assert_eq!(
            key_of("tasks/ps-4.md", "created_by: gmail\nsource_uid: \"gmail:m1\""),
            want("task", "gmail", "gmail:gmail:m1")
        );
        assert_eq!(
            key_of("approvals/task-ps-4.md", "type: approval\nkind: task\ncreated_by: gmail\nsource_uid: \"gmail:m1\""),
            want("appr", "gmail", "gmail:gmail:m1"),
            "a Gmail card and its task are two notes: the kind keeps them apart"
        );
        assert_eq!(
            key_of("approvals/calendar-event-fair.md", "type: approval\nkind: calendar-event\ncreated_by: events\nsource_uid: \"engage:1\""),
            want("appr", "events", "calendar-event:engage:1")
        );
        assert_eq!(
            key_of("approvals/events-digest-2026-09-25.md", "type: approval\nkind: events-digest\nproposed_at: 2026-09-25\ncreated_by: events"),
            want("appr", "events", "events-digest:2026-09-25")
        );
        assert_eq!(
            key_of(
                "approvals/map-zybooks-uahcs100fall2026.md",
                "type: approval\nkind: coursework-map\nfirst_proposed_at: 2026-09-10\ncreated_by: coursework\nsource: \"zybooks\"\nmap_key: \"UAHCS100Fall2026\""
            ),
            want("appr", "coursework", "map:zybooks:UAHCS100Fall2026:2026-09-10")
        );
        assert_eq!(
            key_of("approvals/rule-42.md", "type: approval\nkind: rule\ncreated_by: rules\nrule_id: 42"),
            want("appr", "rules", "rule:42")
        );
    }

    /// §2.2: never for a person, the console, an agent or any other word; never for a kind other
    /// than `task` and `appr`; never for a row whose own key field is missing.
    #[test]
    fn import_key_is_none_for_everything_a_person_or_a_judgment_makes() {
        for who in ["quinn", "dashboard", "agent:knowlu.enrich", "agent:coursework.zybooks", "claude", "commitments"] {
            assert_eq!(key_of("tasks/x.md", &format!("created_by: {who}\nsource_uid: \"zybooks:1\"")), None, "{who}");
        }
        assert_eq!(key_of("tasks/x.md", "source_uid: \"zybooks:1\""), None, "no created_by");
        assert_eq!(key_of("courses/cs-100.md", "created_by: zybooks\nsource_uid: \"zybooks:1\""), None, "kind course");
        assert_eq!(key_of("issues/x.md", "type: issue\ncreated_by: gmail\nsource_uid: \"gmail:m1\""), None, "kind iss");
        assert_eq!(key_of("info/x.md", "type: info\ncreated_by: events\nsource_uid: \"engage:1\""), None, "kind info");
        assert_eq!(key_of("tasks/x.md", "created_by: zybooks"), None, "a coursework task with no source_uid");
        assert_eq!(
            key_of("approvals/amend-x.md", "type: approval\nkind: amend\ncreated_by: events\nsource_uid: \"engage:1\""),
            None,
            "an events card that is no producer's"
        );
        assert_eq!(key_of("approvals/events-digest-x.md", "type: approval\nkind: events-digest\ncreated_by: events"), None, "a digest with no proposed_at");
        assert_eq!(key_of("tasks/x.md", "created_by: events\nsource_uid: \"engage:1\""), None, "a task is never an events note");
    }

    /// D3's `held`: every valid id in the six note folders, `build_index`'s keys.
    #[test]
    fn held_ids_is_every_valid_id_in_the_note_folders() {
        let v = vault();
        note(&v, "tasks/a.md", "---\nid: task_0123456789\n---\n\nb\n");
        note(&v, "archive/b.md", "---\nid: task_abcdef0123\n---\n\nb\n");
        note(&v, "tasks/c.md", "---\nid: not-an-id\n---\n\nb\n");
        assert_eq!(
            held_ids(&v).into_iter().collect::<Vec<_>>(),
            vec!["task_0123456789".to_string(), "task_abcdef0123".to_string()]
        );
    }

    fn create_record(id: &str, path: &str, new: serde_json::Value) -> crate::ledger::Record {
        let mut spec = crate::journal::NewRecord::new("create", path, "agent:coursework.zybooks", "local-runner");
        spec.id = Some(id);
        spec.new = new;
        spec.ts = Some("2026-09-20T10:00:00.000Z".to_string());
        spec.device = Some("DeskA".to_string());
        crate::journal::make_record(spec).unwrap()
    }

    /// §2.6 (re-review m3): `create` records key first, and a note on disk only when its id has no
    /// `create` record — so a later hand edit of `source_uid` never moves a note between keys, and both
    /// desktops read the same immutable input.
    #[test]
    fn import_ids_by_key_reads_create_records_first_and_notes_only_without_one() {
        let v = vault();
        let minted = serde_json::json!({"created_by": "zybooks", "source_uid": "zybooks:1", "id": "task_00000000b2"});
        let records = vec![create_record("task_00000000b2", "tasks/x.md", minted)];
        // The same note on disk, its source_uid since edited by hand: its create record still keys it.
        note(&v, "tasks/x.md", "---\ncreated_by: zybooks\nsource_uid: \"zybooks:edited\"\nid: task_00000000b2\n---\n\nb\n");
        // A note with no create record here (made before the upgrade, or a text written by a pull) keys from disk.
        note(&v, "tasks/y.md", "---\ncreated_by: zybooks\nsource_uid: \"zybooks:1\"\nid: task_00000000a1\n---\n\nb\n");
        let keys = import_ids_by_key(&records, &scan_notes(&v));
        let item: ImportKey = ("task".into(), "zybooks".into(), "coursework:zybooks:1".into());
        assert_eq!(
            keys.get(&item).map(|ids| ids.iter().cloned().collect::<Vec<_>>()),
            Some(vec!["task_00000000a1".to_string(), "task_00000000b2".to_string()])
        );
        let edited: ImportKey = ("task".into(), "zybooks".into(), "coursework:zybooks:edited".into());
        assert!(!keys.contains_key(&edited), "a hand edit does not move a note between keys");
    }

    /// §2.6: two or more ids under one key are one alias group, and the lowest id (as a string; the
    /// shared `kind_` prefix makes this compare the hex) wins — the same on every computer.
    #[test]
    fn aliases_from_maps_every_loser_to_the_lowest_id() {
        let key = |k: &str| -> ImportKey { ("task".into(), "zybooks".into(), k.into()) };
        let mut keys: BTreeMap<ImportKey, BTreeSet<String>> = BTreeMap::new();
        keys.insert(
            key("coursework:zybooks:1"),
            BTreeSet::from(["task_00000000b2".to_string(), "task_00000000a1".to_string(), "task_00000000c3".to_string()]),
        );
        keys.insert(key("coursework:zybooks:2"), BTreeSet::from(["task_00000000d4".to_string()]));
        let aliases = aliases_from(&keys);
        assert_eq!(
            aliases,
            BTreeMap::from([
                ("task_00000000b2".to_string(), "task_00000000a1".to_string()),
                ("task_00000000c3".to_string(), "task_00000000a1".to_string()),
            ])
        );
        assert_eq!(canonical(&aliases, "task_00000000c3"), "task_00000000a1");
        assert_eq!(canonical(&aliases, "task_00000000d4"), "task_00000000d4");
        let group = |id: &str| alias_group(&aliases, id).into_iter().collect::<Vec<_>>();
        assert_eq!(group("task_00000000b2"), vec!["task_00000000a1", "task_00000000b2", "task_00000000c3"]);
        assert_eq!(group("task_00000000d4"), vec!["task_00000000d4"]);
    }

    /// `state/id-aliases.json`: written through `ledger::dumps_value`; missing or unreadable is `{}`.
    #[test]
    fn the_alias_file_round_trips_and_a_missing_or_broken_one_is_empty() {
        let v = vault();
        assert!(load_aliases(&v).is_empty());
        let map = BTreeMap::from([("task_00000000b2".to_string(), "task_00000000a1".to_string())]);
        save_aliases(&v, &map).unwrap();
        assert_eq!(load_aliases(&v), map);
        assert_eq!(pystr::read_text(&v.join(ALIASES_FILE)).unwrap(), "{\"task_00000000b2\": \"task_00000000a1\"}");
        pystr::write_text(&v.join(ALIASES_FILE), "not json").unwrap();
        assert!(load_aliases(&v).is_empty());
    }

    /// Review M3: an issue's `target_id`, and anything else naming the old id, keeps working.
    #[test]
    fn resolve_target_answers_an_old_id_with_its_groups_note() {
        let v = vault();
        note(&v, "tasks/x.md", "---\nid: task_00000000a1\n---\n\nb\n");
        save_aliases(&v, &BTreeMap::from([("task_00000000b2".to_string(), "task_00000000a1".to_string())])).unwrap();
        assert_eq!(resolve_target(&v, "task_00000000b2").unwrap(), v.join("tasks").join("x.md"));
        assert_eq!(resolve_target(&v, "task_00000000c3").unwrap_err(), IdError::UnknownId("task_00000000c3".into()));
    }

    #[test]
    fn new_id_is_ten_hex_characters_and_not_repeated() {
        let a = new_id("task");
        let b = new_id("task");
        assert!(is_id(&a) && is_id(&b));
        assert_eq!(a.len(), "task_".len() + 10);
        assert_ne!(a, b, "two mints must differ");
    }

    #[test]
    fn id_re_rejects_near_misses() {
        assert!(is_id("task_0123456789"));
        assert!(is_id("course_abcdef0123"));
        assert!(!is_id("task_0123456789A"), "uppercase hex is not accepted");
        assert!(!is_id("task_012345678"), "nine characters is too short");
        assert!(!is_id("note_0123456789"), "unknown kind");
    }

    #[test]
    fn kind_for_reads_the_type_field_then_the_folder() {
        let m = |src: &str| crate::yaml::mapping_of(src);
        assert_eq!(kind_for(Path::new("a/approvals/x.md"), Some(&m("type: approval"))), "appr");
        assert_eq!(kind_for(Path::new("a/issues/x.md"), Some(&m("type: issue"))), "iss");
        assert_eq!(kind_for(Path::new("a/info/x.md"), Some(&m("type: info"))), "info");
        assert_eq!(kind_for(Path::new("a/courses/x.md"), Some(&m("title: x"))), "course");
        assert_eq!(kind_for(Path::new("a/tasks/x.md"), Some(&m("title: x"))), "task");
        assert_eq!(kind_for(Path::new("a/tasks/x.md"), None), "task");
    }

    #[test]
    fn read_meta_returns_none_for_anything_unreadable() {
        let v = vault();
        assert_eq!(read_meta(&v.join("tasks").join("missing.md")), None);
        let no_fm = note(&v, "tasks/plain.md", "just a body\n");
        assert_eq!(read_meta(&no_fm), None);
        let bad = note(&v, "tasks/bad.md", "---\ntitle: \"x\nstatus: [unclosed\n---\n\nb\n");
        assert_eq!(read_meta(&bad), None);
    }

    #[test]
    fn ensure_ids_assigns_a_derived_id_and_logs_it() {
        let v = vault();
        note(&v, "tasks/a.md", "---\ntitle: \"A\"\n---\n\nb\n");
        let log = ensure_ids(&v, None);
        assert_eq!(log.len(), 1);
        assert!(log[0].starts_with("assigned task_"), "{}", log[0]);
        assert!(log[0].ends_with(" to tasks/a.md"), "{}", log[0]);

        let meta = read_meta(&v.join("tasks").join("a.md")).unwrap();
        let id = crate::yaml::get(&meta, "id").and_then(crate::yaml::text).unwrap();
        assert_eq!(id, derived_id("task", "tasks/a.md"));
    }

    #[test]
    fn ensure_ids_leaves_a_valid_id_alone() {
        let v = vault();
        note(&v, "tasks/a.md", "---\ntitle: \"A\"\nid: task_0123456789\n---\n\nb\n");
        assert!(ensure_ids(&v, None).is_empty(), "a note with a valid id needs no repair");
    }

    #[test]
    fn ensure_ids_re_ids_the_later_path_of_a_duplicate_pair() {
        let v = vault();
        note(&v, "tasks/a.md", "---\ntitle: \"A\"\nid: task_0123456789\n---\n\nb\n");
        note(&v, "tasks/b.md", "---\ntitle: \"B\"\nid: task_0123456789\n---\n\nb\n");
        let log = ensure_ids(&v, None);
        assert_eq!(log.len(), 1);
        assert!(log[0].contains("duplicate task_0123456789 (tasks/a.md) — tasks/b.md re-idd task_"), "{}", log[0]);

        // The FIRST path keeps the id; the later one is re-minted.
        let a = read_meta(&v.join("tasks").join("a.md")).unwrap();
        let b = read_meta(&v.join("tasks").join("b.md")).unwrap();
        let a_id = crate::yaml::get(&a, "id").and_then(crate::yaml::text).unwrap();
        let b_id = crate::yaml::get(&b, "id").and_then(crate::yaml::text).unwrap();
        assert_eq!(a_id, "task_0123456789");
        assert_ne!(b_id, "task_0123456789");
    }

    #[test]
    fn ensure_ids_logs_an_unreadable_note_and_carries_on() {
        let v = vault();
        note(&v, "tasks/bad.md", "---\ntitle: \"x\nstatus: [unclosed\n---\n\nb\n");
        note(&v, "tasks/good.md", "---\ntitle: \"A\"\n---\n\nb\n");
        let log = ensure_ids(&v, None);
        assert!(log.iter().any(|l| l == "unreadable: tasks/bad.md"), "{log:?}");
        assert!(log.iter().any(|l| l.starts_with("assigned")), "{log:?}");
    }

    #[test]
    fn ensure_ids_logs_unwritable_when_the_writer_fails() {
        let v = vault();
        note(&v, "tasks/a.md", "---\ntitle: \"A\"\n---\n\nb\n");
        let mut failing = |_: &Path, _: &[(String, String)]| {
            Err(IngestError::Io("read-only".into()))
        };
        let log = ensure_ids(&v, Some(&mut failing));
        assert_eq!(log, vec!["unwritable: tasks/a.md".to_string()]);
    }

    #[test]
    fn inside_vault_refuses_a_parent_traversal() {
        let v = vault();
        assert!(inside_vault(&v, &v.join("tasks").join("a.md")).is_ok());
        assert!(
            inside_vault(&v, &v.join("..").join("escaped.md")).is_err(),
            "a .. escape must be refused"
        );
    }

    #[test]
    fn inside_vault_accepts_a_path_that_does_not_exist_yet() {
        // `create` checks its target BEFORE writing it, so strict canonicalisation is not usable.
        let v = vault();
        assert!(inside_vault(&v, &v.join("tasks").join("brand-new.md")).is_ok());
    }

    #[test]
    fn resolve_target_accepts_an_id_or_a_relative_path() {
        let v = vault();
        note(&v, "tasks/a.md", "---\ntitle: \"A\"\nid: task_0123456789\n---\n\nb\n");
        assert!(resolve_target(&v, "task_0123456789").is_ok());
        assert!(resolve_target(&v, "tasks/a.md").is_ok());
        assert_eq!(
            resolve_target(&v, "task_9999999999").unwrap_err(),
            IdError::UnknownId("task_9999999999".into())
        );
        assert_eq!(
            resolve_target(&v, "tasks/nope.md").unwrap_err(),
            IdError::NoSuchNote("tasks/nope.md".into())
        );
    }
}
