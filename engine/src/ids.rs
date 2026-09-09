//! Port of `engine/ids.py` — stable ids for every note.
//!
//! `id` is the **internal** key (journal, issues, dashboard). `source_uid` stays the **external**
//! key for ingest dedup. The two never merge, and conflating them would make a vendor's identifier
//! load-bearing for the journal.

use std::collections::BTreeMap;
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
        let path = index
            .get(target)
            .ok_or_else(|| IdError::UnknownId(target.to_string()))?;
        return inside_vault(vault, path);
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
