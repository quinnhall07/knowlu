//! Port of `engine/issues.py` — issue notes (spec §2.4): Quinn flags an AI judgment; nothing
//! acts on it autonomously.
//!
//! An issue **snapshots its target at click time** — title, the judged fields, the `judgment:`
//! record — so that a later re-judgment cannot alter what was flagged. The snapshot is nested
//! (a `fields` mapping, a `categories` list, an optional `target_record` mapping), which is the
//! whole reason the crate needed a block-style emitter: this note's frontmatter is produced by
//! [`crate::yamlemit::safe_dump_block`], and it must be the bytes PyYAML writes.
//!
//! Addressing an issue is line surgery through `write::write`, then `write::delete` settles it
//! into `archive/`; `list_issues(open_only=False)` reads both folders.

use std::path::{Path, PathBuf};

use jiff::Timestamp;
use serde_yaml_ng::{Mapping, Value};

use crate::approvals::sorted_md;
use crate::ids::{kind_for, new_id, read_meta, rel, resolve_target, IdError};
use crate::info::{python_path, CliError};
use crate::journal::{device_name, now_ts, Journal};
use crate::ledger::Record;
use crate::pystr;
use crate::write::{create, delete, write, WriteContext, WriteError, WriteOpts};
use crate::yamlemit::{safe_dump_block, Node};

pub const CATEGORIES: [&str; 8] = [
    "wrong-effort",
    "wrong-course",
    "duplicate",
    "should-not-exist",
    "wrong-tier",
    "wrong-date",
    "wrong-verdict",
    "other",
];

/// The fields a snapshot copies, **in this order** — it is the order they appear in the note.
pub const SNAPSHOT_FIELDS: [&str; 17] = [
    "title",
    "status",
    "progress",
    "due",
    "effort_hours",
    "effort_confidence",
    "effort_source",
    "importance",
    "importance_reason",
    "course",
    "domain",
    "kind",
    "expires",
    "proposed_at",
    "first_proposed_at",
    "created_by",
    "source_uid",
];

/// `KIND_NAMES`: `ids::kind_for`'s abbreviations, spelled out. Total over what `kind_for` can
/// return — `iss` is refused before the lookup.
pub fn kind_name(kind: &str) -> Option<&'static str> {
    match kind {
        "task" => Some("task"),
        "appr" => Some("approval"),
        "info" => Some("info"),
        "course" => Some("course"),
        _ => None,
    }
}

/// Python: `ValueError`, with the message verbatim, plus the two error types `write` raises.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum IssueError {
    UnknownCategories(Vec<String>),
    Empty,
    Unreadable(String),
    TargetsIssue,
    NotOpen { issue_id: String, status: String },
    Id(IdError),
    Write(WriteError),
}

impl std::fmt::Display for IssueError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            IssueError::UnknownCategories(bad) => {
                // Python's list repr: `['not-a-chip']`.
                let items: Vec<String> = bad.iter().map(|c| format!("'{c}'")).collect();
                write!(f, "unknown categories: [{}]", items.join(", "))
            }
            IssueError::Empty => {
                write!(f, "an issue needs at least one category chip or some text")
            }
            IssueError::Unreadable(target) => write!(f, "cannot read {target}"),
            IssueError::TargetsIssue => write!(f, "an issue cannot target another issue"),
            IssueError::NotOpen { issue_id, status } => {
                write!(f, "issue {issue_id} is not open (status: {status})")
            }
            IssueError::Id(e) => write!(f, "{e}"),
            IssueError::Write(e) => write!(f, "{e}"),
        }
    }
}

impl From<IdError> for IssueError {
    fn from(e: IdError) -> Self {
        IssueError::Id(e)
    }
}
impl From<WriteError> for IssueError {
    fn from(e: WriteError) -> Self {
        IssueError::Write(e)
    }
}

/// `journal.jsonable` over a YAML value, as a node the emitter can write: mapping keys become
/// `str(k)`, everything else is carried through. The YAML 1.1 date gap `yaml::to_json`
/// documents applies here too — a `date` Python would have formatted arrives as the same text.
fn jsonable(value: &Value) -> Node {
    match value {
        Value::Mapping(m) => Node::Map(
            m.iter()
                .map(|(k, v)| (Node::Str(pystr::yaml_str(k)), jsonable(v)))
                .collect(),
        ),
        Value::Sequence(items) => Node::Seq(items.iter().map(jsonable).collect()),
        Value::Tagged(tagged) => jsonable(&tagged.value),
        scalar => Node::from(scalar),
    }
}

fn snapshot(meta: &Mapping) -> Vec<(Node, Node)> {
    let fields: Vec<(Node, Node)> = SNAPSHOT_FIELDS
        .iter()
        .filter_map(|f| crate::yaml::get(meta, f).map(|v| (Node::text(f), jsonable(v))))
        .collect();
    vec![
        (Node::text("title"), crate::yaml::get(meta, "title").map(jsonable).unwrap_or(Node::Null)),
        (Node::text("fields"), Node::Map(fields)),
        (Node::text("judgment"), crate::yaml::get(meta, "judgment").map(jsonable).unwrap_or(Node::Null)),
    ]
}

/// Python's `meta.get(k) == "literal"`: only a string value can be equal.
fn str_value<'a>(meta: &'a Mapping, key: &str) -> Option<&'a str> {
    match crate::yaml::get(meta, key) {
        Some(Value::String(s)) => Some(s.as_str()),
        _ => None,
    }
}

/// Mint `issues/<id>.md` against `target` (an id or a vault-relative path).
///
/// `target_record` — a journal record Quinn flagged rather than a note — switches
/// `target_kind` to `journal`, copies the record's `ts`/`device`/`seq`/`field` into
/// `target_record`, and appends the whole record to the snapshot. A record's key order is the
/// journal's, which is sorted on disk by both engines.
#[allow(clippy::too_many_arguments)]
pub fn open_issue(
    vault: &Path,
    target: &str,
    categories: &[String],
    text: Option<&str>,
    ctx: &WriteContext,
    journal: Option<&mut Journal>,
    now: Option<Timestamp>,
    target_record: Option<&Record>,
) -> Result<PathBuf, IssueError> {
    let bad: Vec<String> =
        categories.iter().filter(|c| !CATEGORIES.contains(&c.as_str())).cloned().collect();
    if !bad.is_empty() {
        return Err(IssueError::UnknownCategories(bad));
    }
    let text = pystr::strip(text.unwrap_or(""));
    if categories.is_empty() && text.is_empty() {
        return Err(IssueError::Empty);
    }
    let path = resolve_target(vault, target)?;
    let meta = read_meta(&path).ok_or_else(|| IssueError::Unreadable(target.to_string()))?;
    let kind = kind_for(&path, Some(&meta));
    if kind == "iss" {
        return Err(IssueError::TargetsIssue);
    }
    let mut owned;
    let journal: &mut Journal = match journal {
        Some(supplied) => supplied,
        None => {
            owned = Journal::new(vault);
            &mut owned
        }
    };
    let issue_id = new_id("iss");
    let mut snapshot = snapshot(&meta);
    let target_kind = match target_record {
        Some(_) => "journal".to_string(),
        None => kind_name(&kind).expect("kind_for returns a named kind").to_string(),
    };
    let target_record_node = match target_record {
        Some(record) => {
            snapshot.push((
                Node::text("record"),
                Node::from_json(&serde_json::Value::Object(record.clone())),
            ));
            Node::map(
                ["ts", "device", "seq", "field"]
                    .iter()
                    .map(|k| (*k, record.get(*k).map(Node::from_json).unwrap_or(Node::Null)))
                    .collect(),
            )
        }
        None => Node::Null,
    };
    let front = Node::map(vec![
        ("type", Node::text("issue")),
        ("id", Node::text(&issue_id)),
        ("status", Node::text("open")),
        ("target_id", crate::yaml::get(&meta, "id").map(jsonable).unwrap_or(Node::Null)),
        ("target_kind", Node::text(&target_kind)),
        ("target_path", Node::text(&rel(vault, &path))),
        ("target_record", target_record_node),
        ("categories", Node::Seq(categories.iter().map(|c| Node::text(c)).collect())),
        ("opened_at", Node::text(&now_ts(now))),
        ("opened_on", Node::text(&device_name())),
        ("snapshot", Node::Map(snapshot)),
        ("resolution", Node::Null),
        ("addressed_at", Node::Null),
        ("addressed_in", Node::Null),
    ]);
    let body = if text.is_empty() { String::new() } else { format!("{text}\n") };
    let note = format!("---\n{}---\n\n{body}", safe_dump_block(&front));
    Ok(create(vault, &format!("issues/{issue_id}.md"), &note, ctx, journal, None)?)
}

/// Every issue, path order, as `{**meta, "body": body, "path": rel}` — `issues/` only when
/// `open_only`, else `issues/` then `archive/`.
pub fn list_issues(vault: &Path, open_only: bool) -> Vec<Mapping> {
    let folders: &[&str] = if open_only { &["issues"] } else { &["issues", "archive"] };
    let mut found = Vec::new();
    for folder in folders {
        for path in sorted_md(&vault.join(folder)) {
            let Some(mut meta) = read_meta(&path) else { continue };
            if str_value(&meta, "type") != Some("issue") {
                continue;
            }
            if open_only && str_value(&meta, "status") != Some("open") {
                continue;
            }
            let text = pystr::read_text(&path).unwrap_or_default();
            let body = match text.split_once("\n---\n") {
                Some((_, rest)) => rest.to_string(),
                None => String::new(),
            };
            meta.insert(Value::String("body".into()), Value::String(body));
            meta.insert(Value::String("path".into()), Value::String(rel(vault, &path)));
            found.push(meta);
        }
    }
    found
}

/// Mark an open issue addressed and settle it into `archive/`. Refuses anything not open — an
/// archived issue resolves by id just as an open one does, so the second call reads
/// `status: addressed` and raises rather than writing a `-2` copy.
pub fn address_issue(
    vault: &Path,
    issue_id: &str,
    resolution: &str,
    commit: Option<&str>,
    ctx: &WriteContext,
    journal: Option<&mut Journal>,
    now: Option<Timestamp>,
) -> Result<PathBuf, IssueError> {
    let mut owned;
    let journal: &mut Journal = match journal {
        Some(supplied) => supplied,
        None => {
            owned = Journal::new(vault);
            &mut owned
        }
    };
    let path = resolve_target(vault, issue_id)?;
    let meta = read_meta(&path).unwrap_or_default();
    if str_value(&meta, "status") != Some("open") {
        return Err(IssueError::NotOpen {
            issue_id: issue_id.to_string(),
            status: crate::yaml::get(&meta, "status")
                .map(pystr::yaml_str)
                .unwrap_or_else(|| "None".into()),
        });
    }
    let changes = [
        ("status".to_string(), Value::String("addressed".into())),
        ("resolution".to_string(), Value::String(resolution.into())),
        ("addressed_at".to_string(), Value::String(now_ts(now))),
        (
            "addressed_in".to_string(),
            commit.map(|c| Value::String(c.into())).unwrap_or(Value::Null),
        ),
    ];
    write(vault, issue_id, &changes, ctx, journal, &WriteOpts::default())?;
    Ok(delete(vault, issue_id, ctx, journal)?)
}

// ---------------------------------------------------------------------------------------------
// `python -m engine.issues`
// ---------------------------------------------------------------------------------------------

/// The subcommands, already parsed. `main.rs` owns the `clap` surface.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum IssueCommand {
    Open { target: String, categories: Vec<String>, text: String },
    List { all: bool },
    Address { issue_id: String, resolution: String, commit: Option<String> },
}

/// `main(argv)` after parsing: the lines it would print, in order.
pub fn cli(
    vault: &Path,
    actor: &str,
    via: &str,
    command: &IssueCommand,
) -> Result<Vec<String>, CliError> {
    let ctx = WriteContext::new(actor, via);
    match command {
        IssueCommand::Open { target, categories, text } => {
            let path = open_issue(vault, target, categories, Some(text), &ctx, None, None, None)
                .map_err(|e| CliError::Failed(e.to_string()))?;
            let name = path.file_name().map(|n| n.to_string_lossy().to_string()).unwrap_or_default();
            Ok(vec![format!("opened {name}")])
        }
        IssueCommand::List { all } => Ok(list_issues(vault, !all)
            .iter()
            .map(|issue| {
                let field = |k: &str| {
                    crate::yaml::get(issue, k).map(pystr::yaml_str).unwrap_or_else(|| "None".into())
                };
                // `",".join(issue.get("categories") or []) or "-"`
                let chips = match crate::yaml::get(issue, "categories") {
                    Some(Value::Sequence(items)) if !items.is_empty() => {
                        items.iter().map(pystr::yaml_str).collect::<Vec<_>>().join(",")
                    }
                    _ => "-".to_string(),
                };
                // `(issue.get("snapshot") or {}).get("title")`
                let title = match crate::yaml::get(issue, "snapshot") {
                    Some(Value::Mapping(snap)) => crate::yaml::get(snap, "title")
                        .map(pystr::yaml_str)
                        .unwrap_or_else(|| "None".into()),
                    _ => "None".to_string(),
                };
                let body: String = pystr::strip(&field("body")).chars().take(80).collect();
                format!(
                    "{}  {:<9}  {:<8}  {:<20}  {title}  {body}",
                    field("id"),
                    field("status"),
                    field("target_kind"),
                    chips
                )
            })
            .collect()),
        IssueCommand::Address { issue_id, resolution, commit } => {
            let path = address_issue(vault, issue_id, resolution, commit.as_deref(), &ctx, None, None)
                .map_err(|e| CliError::Failed(e.to_string()))?;
            Ok(vec![format!("addressed -> {}", python_path(&path))])
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::models::split_frontmatter;

    fn ctx() -> WriteContext {
        WriteContext::new("quinn", "dashboard")
    }

    /// `datetime(2026, 8, 29, 14, 10, 2, tzinfo=timezone.utc)`.
    fn now() -> Option<Timestamp> {
        Some("2026-08-29T14:10:02Z".parse().unwrap())
    }

    const TASK: &str = "---\ntitle: \"Prepare AEMX slide\"\nstatus: active\nprogress: 0\ndue: 2026-09-01T23:59\n\
                        effort_hours: 0.5\nimportance: 3\ncourse: null\ndomain: school\n\
                        judgment: {actor: 'agent:routine.gmail', at: '2026-08-28T13:04:11.000Z', fields: [effort_hours, importance], inputs: {source_uid: 'gmail:1'}, run_id: cloud-1}\n\
                        ---\n\nbody\n";

    fn make_vault(name: &str) -> PathBuf {
        let dir = std::env::temp_dir().join(format!(
            "qo-issues-{name}-{}-{:?}",
            std::process::id(),
            std::thread::current().id()
        ));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(dir.join("tasks")).unwrap();
        create(&dir, "tasks/a.md", TASK, &ctx(), &mut Journal::new(&dir), None).unwrap();
        dir
    }

    fn read(path: &Path) -> (Mapping, String) {
        split_frontmatter(&pystr::read_text(path).unwrap()).unwrap()
    }

    fn meta_of(path: &Path) -> Mapping {
        read(path).0
    }

    fn field(meta: &Mapping, key: &str) -> String {
        crate::yaml::get(meta, key).map(pystr::yaml_str).unwrap_or_else(|| "<absent>".into())
    }

    fn nested<'a>(meta: &'a Mapping, keys: &[&str]) -> &'a Value {
        let mut current: &Value = crate::yaml::get(meta, keys[0]).expect(keys[0]);
        for key in &keys[1..] {
            current = match current {
                Value::Mapping(m) => crate::yaml::get(m, key).expect(key),
                other => panic!("{key}: not a mapping: {other:?}"),
            };
        }
        current
    }

    fn parent_name(path: &Path) -> String {
        path.parent().and_then(|p| p.file_name()).unwrap().to_string_lossy().to_string()
    }

    fn cats(items: &[&str]) -> Vec<String> {
        items.iter().map(|s| s.to_string()).collect()
    }

    fn open(vault: &Path, target: &str, categories: &[&str], text: Option<&str>) -> Result<PathBuf, IssueError> {
        open_issue(vault, target, &cats(categories), text, &ctx(), None, now(), None)
    }

    #[test]
    fn test_open_issue_snapshots_target_at_click_time_with_chips_and_text() {
        // `open` below calls `open_issue`, which stamps `opened_on` from the process-global
        // `device_name()`. Rust tests share one process and run in parallel, so this holds the
        // same crate-wide lock every `KNOWLU_DEVICE` setter takes (`journal::DEVICE_ENV_MUTEX`)
        // across that internal read and the comparison below — reading `device_name()` again,
        // unguarded, could observe a `history.rs` test mid-`set_var`/`remove_var`.
        let _guard = crate::journal::DEVICE_ENV_MUTEX.lock().unwrap();
        let vault = make_vault("snapshot");
        let task_id = field(&meta_of(&vault.join("tasks/a.md")), "id");
        let path = open(&vault, &task_id, &["wrong-effort"], Some("This is a 2h job, not 30 min.")).unwrap();
        // Captured once, while the lock is held, and reused below instead of a second call.
        let expected_device = device_name();
        let (meta, body) = read(&path);
        assert_eq!(parent_name(&path), "issues");
        assert_eq!(field(&meta, "type"), "issue");
        assert_eq!(field(&meta, "status"), "open");
        assert!(field(&meta, "id").starts_with("iss_"));
        assert_eq!(field(&meta, "target_id"), task_id);
        assert_eq!(field(&meta, "target_kind"), "task");
        assert_eq!(field(&meta, "target_path"), "tasks/a.md");
        assert_eq!(
            nested(&meta, &["categories"]),
            &Value::Sequence(vec![Value::String("wrong-effort".into())])
        );
        // Python monkeypatches KNOWLU_DEVICE=laptop; a process-wide env var would leak into
        // every parallel test, so this asserts against the same source the monkeypatch fed.
        assert_eq!(field(&meta, "opened_on"), expected_device);
        assert!(field(&meta, "opened_at").starts_with("2026-08-29T14:10:02"));
        assert_eq!(nested(&meta, &["snapshot", "title"]), &Value::String("Prepare AEMX slide".into()));
        assert_eq!(nested(&meta, &["snapshot", "fields", "effort_hours"]), &Value::Number(0.5.into()));
        assert_eq!(
            nested(&meta, &["snapshot", "judgment", "actor"]),
            &Value::String("agent:routine.gmail".into())
        );
        assert_eq!(pystr::strip(&body), "This is a 2h job, not 30 min.");
        // Later re-judgment does not alter what Quinn flagged:
        write(
            &vault,
            &task_id,
            &[("effort_hours".to_string(), Value::Number(4.0.into()))],
            &ctx(),
            &mut Journal::new(&vault),
            &WriteOpts::default(),
        )
        .unwrap();
        assert_eq!(nested(&meta_of(&path), &["snapshot", "fields", "effort_hours"]), &Value::Number(0.5.into()));
        let _ = std::fs::remove_dir_all(&vault);
    }

    /// The whole note, byte for byte, against what `safe_dump` produces for this exact shape —
    /// the snapshot's field order is `SNAPSHOT_FIELDS`', the judgment's is the note's own, and
    /// `due: 2026-09-01T23:59` stays a plain scalar because PyYAML's timestamp needs seconds.
    #[test]
    fn the_note_is_the_pyyaml_shape() {
        // Same race as the snapshot test above: `open` stamps `opened_on` from the process-global
        // `device_name()`, so this holds `journal::DEVICE_ENV_MUTEX` from before that call through
        // building `expected` below, rather than reading `device_name()` again unguarded.
        let _guard = crate::journal::DEVICE_ENV_MUTEX.lock().unwrap();
        let vault = make_vault("bytes");
        let task_id = field(&meta_of(&vault.join("tasks/a.md")), "id");
        let path = open(&vault, "tasks/a.md", &["wrong-effort", "wrong-course"], Some("  too low  ")).unwrap();
        let id = field(&meta_of(&path), "id");
        let expected = format!(
            "---\ntype: issue\nid: {id}\nstatus: open\ntarget_id: {task_id}\ntarget_kind: task\n\
             target_path: tasks/a.md\ntarget_record: null\ncategories:\n- wrong-effort\n- wrong-course\n\
             opened_at: '2026-08-29T14:10:02.000Z'\nopened_on: {}\nsnapshot:\n  title: Prepare AEMX slide\n\
             \x20 fields:\n    title: Prepare AEMX slide\n    status: active\n    progress: 0\n\
             \x20   due: 2026-09-01T23:59\n    effort_hours: 0.5\n    importance: 3\n    course: null\n\
             \x20   domain: school\n  judgment:\n    actor: agent:routine.gmail\n\
             \x20   at: '2026-08-28T13:04:11.000Z'\n    fields:\n    - effort_hours\n    - importance\n\
             \x20   inputs:\n      source_uid: gmail:1\n    run_id: cloud-1\nresolution: null\n\
             addressed_at: null\naddressed_in: null\n---\n\ntoo low\n",
            device_name()
        );
        assert_eq!(pystr::read_text(&path).unwrap(), expected);
        let _ = std::fs::remove_dir_all(&vault);
    }

    #[test]
    fn test_chip_only_and_text_only_issues_are_both_allowed() {
        let vault = make_vault("chips");
        open(&vault, "tasks/a.md", &["duplicate"], Some("")).unwrap();
        open(&vault, "tasks/a.md", &[], Some("just words")).unwrap();
        assert_eq!(list_issues(&vault, true).len(), 2);
        assert_eq!(open(&vault, "tasks/a.md", &[], Some("")), Err(IssueError::Empty));
        assert_eq!(
            open(&vault, "tasks/a.md", &["not-a-chip"], Some("x")),
            Err(IssueError::UnknownCategories(vec!["not-a-chip".into()]))
        );
        assert_eq!(
            IssueError::UnknownCategories(vec!["not-a-chip".into()]).to_string(),
            "unknown categories: ['not-a-chip']"
        );
        let _ = std::fs::remove_dir_all(&vault);
    }

    #[test]
    fn test_journal_record_target_kind() {
        let vault = make_vault("record");
        let task_id = field(&meta_of(&vault.join("tasks/a.md")), "id");
        let rec: Record = serde_json::from_value(serde_json::json!({
            "ts": "2026-08-29T12:00:00.000Z", "device": "desktop", "seq": 3,
            "actor": "agent:completion", "op": "set", "id": task_id, "path": "tasks/a.md",
            "field": "progress", "old": 40, "new": 100, "evidence": {"vhl": "complete"}
        }))
        .unwrap();
        let path = open_issue(
            &vault,
            &task_id,
            &cats(&["wrong-verdict"]),
            Some("I had not finished this."),
            &ctx(),
            None,
            now(),
            Some(&rec),
        )
        .unwrap();
        let meta = meta_of(&path);
        assert_eq!(field(&meta, "target_kind"), "journal");
        let expected: Value = serde_yaml_ng::from_str(
            "ts: '2026-08-29T12:00:00.000Z'\ndevice: desktop\nseq: 3\nfield: progress\n",
        )
        .unwrap();
        assert_eq!(nested(&meta, &["target_record"]), &expected);
        let evidence: Value = serde_yaml_ng::from_str("vhl: complete\n").unwrap();
        assert_eq!(nested(&meta, &["snapshot", "record", "evidence"]), &evidence);
        // The record is written in the journal's (sorted) key order, `evidence` first.
        let text = pystr::read_text(&path).unwrap();
        assert!(
            text.contains("  record:\n    actor: agent:completion\n    device: desktop\n    evidence:\n      vhl: complete\n    field: progress\n"),
            "{text}"
        );
        let _ = std::fs::remove_dir_all(&vault);
    }

    #[test]
    fn test_list_and_address_moves_to_archive_with_resolution() {
        let vault = make_vault("address");
        let path = open(&vault, "tasks/a.md", &["wrong-course"], Some("It is GN 103.")).unwrap();
        let issue_id = field(&meta_of(&path), "id");
        let listed = list_issues(&vault, true);
        assert_eq!(listed.iter().map(|i| field(i, "id")).collect::<Vec<_>>(), vec![issue_id.clone()]);
        assert_eq!(pystr::strip(&field(&listed[0], "body")), "It is GN 103.");
        let archived = address_issue(
            &vault,
            &issue_id,
            "Pinned course_map; set course.",
            Some("abc1234"),
            &WriteContext::new("quinn", "cli"),
            None,
            now(),
        )
        .unwrap();
        assert_eq!(parent_name(&archived), "archive");
        assert!(!path.exists());
        let meta = meta_of(&archived);
        assert_eq!(field(&meta, "status"), "addressed");
        assert_eq!(field(&meta, "resolution"), "Pinned course_map; set course.");
        assert_eq!(field(&meta, "addressed_in"), "abc1234");
        assert!(field(&meta, "addressed_at").starts_with("2026-08-29T14:10:02"));
        assert!(list_issues(&vault, true).is_empty());
        assert_eq!(list_issues(&vault, false).len(), 1);
        let _ = std::fs::remove_dir_all(&vault);
    }

    #[test]
    fn test_categories_are_the_spec_list() {
        assert_eq!(
            CATEGORIES,
            [
                "wrong-effort",
                "wrong-course",
                "duplicate",
                "should-not-exist",
                "wrong-tier",
                "wrong-date",
                "wrong-verdict",
                "other"
            ]
        );
    }

    #[test]
    fn test_address_twice_is_refused_and_archive_is_untouched() {
        let vault = make_vault("twice");
        let path = open(&vault, "tasks/a.md", &["other"], Some("x")).unwrap();
        let issue_id = field(&meta_of(&path), "id");
        let archived = address_issue(&vault, &issue_id, "done", Some("abc"), &ctx(), None, now()).unwrap();
        assert_eq!(
            address_issue(&vault, &issue_id, "again", Some("def"), &ctx(), None, now()),
            Err(IssueError::NotOpen { issue_id: issue_id.clone(), status: "addressed".into() })
        );
        assert!(archived.exists());
        let stem = archived.file_stem().unwrap().to_string_lossy().to_string();
        assert!(!archived.parent().unwrap().join(format!("{stem}-2.md")).exists());
        assert_eq!(field(&meta_of(&archived), "resolution"), "done");
        let _ = std::fs::remove_dir_all(&vault);
    }

    #[test]
    fn test_issue_cannot_target_an_issue_and_none_text_is_empty() {
        let vault = make_vault("meta");
        let first = open(&vault, "tasks/a.md", &["other"], None).unwrap();
        assert_eq!(pystr::strip(&read(&first).1), "");
        let first_id = field(&meta_of(&first), "id");
        assert_eq!(open(&vault, &first_id, &["other"], Some("meta")), Err(IssueError::TargetsIssue));
        assert_eq!(open(&vault, "tasks/a.md", &[], None), Err(IssueError::Empty));
        // An unknown target is Python's `ValueError("unknown id: …")`, here the id error.
        assert!(matches!(
            open(&vault, "iss_0000000000", &["other"], Some("x")),
            Err(IssueError::Id(IdError::UnknownId(_)))
        ));
        let _ = std::fs::remove_dir_all(&vault);
    }

    #[test]
    fn test_cli_open_list_address_round_trip() {
        let vault = make_vault("cli");
        let open = IssueCommand::Open {
            target: "tasks/a.md".into(),
            categories: cats(&["wrong-effort"]),
            text: "too low".into(),
        };
        let out = cli(&vault, "quinn", "cli", &open).unwrap();
        assert!(out.len() == 1 && out[0].starts_with("opened iss_"), "{out:?}");
        let issue_id = field(&list_issues(&vault, true)[0], "id");
        let out = cli(&vault, "quinn", "cli", &IssueCommand::List { all: false }).unwrap();
        assert_eq!(out.len(), 1);
        assert!(out[0].contains(&issue_id) && out[0].contains("wrong-effort") && out[0].contains("too low"), "{out:?}");
        // `f"{id}  {status:9}  {target_kind:8}  {chips:20}  {title}  {body[:80]}"`
        assert_eq!(
            out[0],
            format!("{issue_id}  open       task      wrong-effort          Prepare AEMX slide  too low")
        );
        let address = IssueCommand::Address {
            issue_id: issue_id.clone(),
            resolution: "raised to 2h".into(),
            commit: Some("abc1234".into()),
        };
        let out = cli(&vault, "quinn", "cli", &address).unwrap();
        assert!(out.len() == 1 && out[0].starts_with("addressed -> "), "{out:?}");
        assert!(list_issues(&vault, true).is_empty());
        assert_eq!(list_issues(&vault, false).len(), 1);
        // `--all` lists the archived one with its new status; a second address is exit 1.
        let out = cli(&vault, "quinn", "cli", &IssueCommand::List { all: true }).unwrap();
        assert!(out[0].contains("  addressed  task      "), "{out:?}");
        assert_eq!(
            cli(&vault, "quinn", "cli", &address).unwrap_err().exit_code(),
            1
        );
        let _ = std::fs::remove_dir_all(&vault);
    }

    /// Task 17 defect sweep (2026-09-02), preserved defect 22: `address_issue` checks
    /// `status == "open"` and never `type == "issue"`, and an open **info** item is `status:
    /// open` too. So its id is accepted, it is stamped `addressed` with a `resolution` it never
    /// asked for, and it is archived. Measured through both CLIs: exit 0, `addressed -> …`,
    /// the note in `archive/`. Nothing calls `issues address` with an info id today.
    #[test]
    fn preserved_defect_22_address_accepts_an_open_info_item() {
        use crate::info::{open_info, NewInfo};
        let vault = make_vault("infoaddr");
        std::fs::create_dir_all(vault.join("info")).unwrap();
        let item = NewInfo { title: "Package", kind: "package", body: "", opened_by: "quinn", close_key: None, expires: None };
        let info = open_info(&vault, &item, &ctx(), None, None).unwrap();
        let info_id = field(&meta_of(&info), "id");
        assert!(info_id.starts_with("info_"));

        let archived = address_issue(&vault, &info_id, "oops", None, &ctx(), None, now()).unwrap();
        assert!(!info.exists());
        assert_eq!(parent_name(&archived), "archive");
        let meta = meta_of(&archived);
        assert_eq!(field(&meta, "type"), "info");
        assert_eq!(field(&meta, "status"), "addressed");
        assert_eq!(field(&meta, "resolution"), "oops");
        let _ = std::fs::remove_dir_all(&vault);
    }
}
