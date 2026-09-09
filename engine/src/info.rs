//! Port of `engine/info.py` — info items (spec §2.5): things to know, with an open/close
//! lifecycle keyed by `close_key`.
//!
//! An item is a note in `info/` whose frontmatter is **produced** by [`crate::yamlemit`] in
//! block style — this and `issues` are the two places the crate writes a whole new note from a
//! mapping rather than splicing one line into an existing one. Closing is ordinary line surgery
//! through `write::write`, then `write::delete` settles the note into `archive/`.
//!
//! Two rules that are easy to get wrong, both pinned by the ported tests: closing by key closes
//! **every** open item with that key, and the match is exact, never a prefix; and `info_pass`
//! expires an item only when `expires` is strictly **before** today — the same rule approvals
//! use, so an item expiring today is still shown today.

use std::path::{Path, PathBuf};

use jiff::civil::Date;
use jiff::Timestamp;
use serde_yaml_ng::{Mapping, Value};

use crate::approvals::{as_date, sorted_md};
use crate::ids::{new_id, read_meta, rel};
use crate::journal::{now_ts, Journal};
use crate::pystr;
use crate::write::{create, delete, write, WriteContext, WriteError, WriteOpts};
use crate::yamlemit::{safe_dump_block, Node};

pub const KINDS: [&str; 4] = ["package", "notice", "heads-up", "other"];

/// Python: `ValueError`, with the message verbatim.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum InfoError {
    UnknownKind(String),
    NeedsKeyOrId,
    KeyAndId,
    Write(WriteError),
}

impl std::fmt::Display for InfoError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            InfoError::UnknownKind(kind) => write!(f, "unknown info kind: {kind}"),
            InfoError::NeedsKeyOrId => write!(f, "close_info needs key= or info_id="),
            InfoError::KeyAndId => write!(f, "close_info takes key= or info_id=, not both"),
            InfoError::Write(e) => write!(f, "{e}"),
        }
    }
}

impl From<WriteError> for InfoError {
    fn from(e: WriteError) -> Self {
        InfoError::Write(e)
    }
}

/// The keyword arguments of `open_info`.
#[derive(Debug, Clone, Default)]
pub struct NewInfo<'a> {
    pub title: &'a str,
    pub kind: &'a str,
    pub body: &'a str,
    pub opened_by: &'a str,
    pub close_key: Option<&'a str>,
    pub expires: Option<Date>,
}

/// Mint `info/<id>.md`. The frontmatter is `yaml.safe_dump(front, allow_unicode=True,
/// sort_keys=False, width=10**6)` — key order is the Python dict's, and `expires` is a real date
/// (plain `2026-09-05`), not the quoted string the same text would be.
pub fn open_info(
    vault: &Path,
    item: &NewInfo<'_>,
    ctx: &WriteContext,
    journal: Option<&mut Journal>,
    now: Option<Timestamp>,
) -> Result<PathBuf, InfoError> {
    if !KINDS.contains(&item.kind) {
        return Err(InfoError::UnknownKind(item.kind.to_string()));
    }
    let mut owned;
    let journal: &mut Journal = match journal {
        Some(supplied) => supplied,
        None => {
            owned = Journal::new(vault);
            &mut owned
        }
    };
    let info_id = new_id("info");
    let front = Node::map(vec![
        ("type", Node::text("info")),
        ("id", Node::text(&info_id)),
        ("status", Node::text("open")),
        ("kind", Node::text(item.kind)),
        ("title", Node::text(item.title)),
        ("opened_at", Node::text(&now_ts(now))),
        ("opened_by", Node::text(item.opened_by)),
        ("close_key", Node::opt_text(item.close_key)),
        ("expires", item.expires.map(Node::Date).unwrap_or(Node::Null)),
        ("closed_at", Node::Null),
        ("closed_by", Node::Null),
    ]);
    let body = pystr::strip(item.body);
    let body = if body.is_empty() { String::new() } else { format!("{body}\n") };
    let text = format!("---\n{}---\n\n{body}", safe_dump_block(&front));
    Ok(create(vault, &format!("info/{info_id}.md"), &text, ctx, journal, None)?)
}

/// Every open item in `info/`, path order, as `{**meta, "path": rel}`.
pub fn list_info(vault: &Path) -> Vec<Mapping> {
    let mut found = Vec::new();
    for path in sorted_md(&vault.join("info")) {
        let Some(mut meta) = read_meta(&path) else { continue };
        if text_field(&meta, "type").as_deref() == Some("info")
            && text_field(&meta, "status").as_deref() == Some("open")
        {
            meta.insert(Value::String("path".into()), Value::String(rel(vault, &path)));
            found.push(meta);
        }
    }
    found
}

/// Python's `meta.get(k)` compared against a `str`: only a string value can be equal. A bare
/// `123` in a hand-edited note does not match the key `"123"`, exactly as it would not in Python.
fn str_value<'a>(meta: &'a Mapping, key: &str) -> Option<&'a str> {
    match crate::yaml::get(meta, key) {
        Some(Value::String(s)) => Some(s.as_str()),
        _ => None,
    }
}

fn text_field(meta: &Mapping, key: &str) -> Option<String> {
    crate::yaml::get(meta, key).and_then(crate::yaml::text)
}

fn close_one(
    vault: &Path,
    rel_path: &str,
    closed_by: &str,
    ctx: &WriteContext,
    journal: &mut Journal,
    now: Option<Timestamp>,
) -> Result<PathBuf, WriteError> {
    let changes = [
        ("status".to_string(), Value::String("closed".into())),
        ("closed_at".to_string(), Value::String(now_ts(now))),
        ("closed_by".to_string(), Value::String(closed_by.into())),
    ];
    write(vault, rel_path, &changes, ctx, journal, &WriteOpts::default())?;
    delete(vault, rel_path, ctx, journal)
}

/// Close open info item(s) matching `key` (exact `close_key`) or `info_id`, never both.
///
/// Closing by key closes EVERY open item whose `close_key` matches exactly — a close signal
/// settles all items keyed to it, including duplicates; the returned list says how many.
/// An empty string counts as absent, as it does under Python's `not key`.
pub fn close_info(
    vault: &Path,
    key: Option<&str>,
    info_id: Option<&str>,
    closed_by: &str,
    ctx: &WriteContext,
    journal: Option<&mut Journal>,
    now: Option<Timestamp>,
) -> Result<Vec<PathBuf>, InfoError> {
    let key = key.filter(|k| !k.is_empty());
    let info_id = info_id.filter(|k| !k.is_empty());
    if key.is_none() && info_id.is_none() {
        return Err(InfoError::NeedsKeyOrId);
    }
    if key.is_some() && info_id.is_some() {
        return Err(InfoError::KeyAndId);
    }
    let mut owned;
    let journal: &mut Journal = match journal {
        Some(supplied) => supplied,
        None => {
            owned = Journal::new(vault);
            &mut owned
        }
    };
    let mut closed = Vec::new();
    for item in list_info(vault) {
        let by_id = info_id.is_some() && str_value(&item, "id") == info_id;
        let by_key = key.is_some() && str_value(&item, "close_key") == key;
        if by_id || by_key {
            let rel_path = text_field(&item, "path").unwrap_or_default();
            closed.push(close_one(vault, &rel_path, closed_by, ctx, journal, now)?);
        }
    }
    Ok(closed)
}

/// Close items whose `expires` is before today (exclusive of today, same rule as approvals).
pub fn info_pass(
    vault: &Path,
    today: Date,
    ctx: &WriteContext,
    journal: Option<&mut Journal>,
) -> Result<Vec<String>, InfoError> {
    let mut owned;
    let journal: &mut Journal = match journal {
        Some(supplied) => supplied,
        None => {
            owned = Journal::new(vault);
            &mut owned
        }
    };
    let expiry = ctx.with_actor("system:expiry");
    let mut log = Vec::new();
    for item in list_info(vault) {
        let expires = as_date(crate::yaml::get(&item, "expires"));
        if let Some(expires) = expires {
            if expires < today {
                let rel_path = text_field(&item, "path").unwrap_or_default();
                close_one(vault, &rel_path, "system:expiry", &expiry, journal, None)?;
                log.push(format!(
                    "expired info {} ({})",
                    text_field(&item, "id").unwrap_or_default(),
                    crate::yaml::get(&item, "title").map(pystr::yaml_str).unwrap_or("None".into())
                ));
            }
        }
    }
    Ok(log)
}

// ---------------------------------------------------------------------------------------------
// `python -m engine.info`
// ---------------------------------------------------------------------------------------------

/// `--expires`: `date.fromisoformat`, or the argparse error verbatim. Only the `YYYY-MM-DD` form
/// is read — Python 3.11+ also accepts `20260905` and week dates, which nothing has ever passed.
pub fn parse_expires(text: Option<&str>) -> Result<Option<Date>, String> {
    match text {
        None | Some("") => Ok(None),
        Some(raw) => raw
            .parse::<Date>()
            .map(Some)
            .map_err(|_| format!("--expires must be YYYY-MM-DD, got '{raw}'")),
    }
}

/// The subcommands, already parsed. `main.rs` owns the `clap` surface.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum InfoCommand {
    Open {
        title: String,
        kind: String,
        body: String,
        opened_by: String,
        close_key: Option<String>,
        expires: Option<String>,
    },
    Close {
        key: Option<String>,
        id: Option<String>,
        closed_by: String,
    },
    List,
}

/// How a subcommand failed. `Usage` is argparse's `parser.error` (exit 2); `Failed` is an
/// uncaught `ValueError` (exit 1).
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CliError {
    Usage(String),
    Failed(String),
}

impl std::fmt::Display for CliError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            CliError::Usage(m) | CliError::Failed(m) => write!(f, "{m}"),
        }
    }
}

impl CliError {
    pub fn exit_code(&self) -> u8 {
        match self {
            CliError::Usage(_) => 2,
            CliError::Failed(_) => 1,
        }
    }
}

/// Python's `str(Path)`, which the `closed ->` / `addressed ->` lines print. `pathlib`
/// normalises every separator to the platform's on construction, so a vault given as
/// `C:/x/vault` prints as `C:\x\vault\archive\…`; `Path::display` would keep the caller's
/// slashes and append a backslash. Measured in the dual run, and the only difference it found.
pub fn python_path(path: &Path) -> String {
    let text = path.to_string_lossy();
    if cfg!(windows) {
        text.replace('/', "\\")
    } else {
        text.to_string()
    }
}

/// `main(argv)` after parsing: the lines it would print, in order.
pub fn cli(
    vault: &Path,
    actor: &str,
    via: &str,
    command: &InfoCommand,
) -> Result<Vec<String>, CliError> {
    let ctx = WriteContext::new(actor, via);
    match command {
        InfoCommand::Open { title, kind, body, opened_by, close_key, expires } => {
            let expires = parse_expires(expires.as_deref()).map_err(CliError::Usage)?;
            let item = NewInfo {
                title,
                kind,
                body,
                opened_by,
                close_key: close_key.as_deref(),
                expires,
            };
            let path = open_info(vault, &item, &ctx, None, None)
                .map_err(|e| CliError::Failed(e.to_string()))?;
            let name = path.file_name().map(|n| n.to_string_lossy().to_string()).unwrap_or_default();
            Ok(vec![format!("opened {name}")])
        }
        InfoCommand::Close { key, id, closed_by } => {
            let key = key.as_deref().filter(|k| !k.is_empty());
            let id = id.as_deref().filter(|k| !k.is_empty());
            if key.is_some() && id.is_some() {
                return Err(CliError::Usage("close takes --key or --id, not both".into()));
            }
            let closed = close_info(vault, key, id, closed_by, &ctx, None, None)
                .map_err(|e| CliError::Failed(e.to_string()))?;
            Ok(closed.iter().map(|p| format!("closed -> {}", python_path(p))).collect())
        }
        InfoCommand::List => Ok(list_info(vault)
            .iter()
            .map(|item| {
                let field = |k: &str| {
                    crate::yaml::get(item, k).map(pystr::yaml_str).unwrap_or_else(|| "None".into())
                };
                format!(
                    "{}  {:<8}  expires={}  {}",
                    field("id"),
                    field("kind"),
                    field("expires"),
                    field("title")
                )
            })
            .collect()),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::models::split_frontmatter;

    fn ctx() -> WriteContext {
        WriteContext {
            actor: "agent:routine.gmail".into(),
            via: "cloud-routine".into(),
            run_id: Some("cloud-1".into()),
        }
    }

    /// `datetime(2026, 8, 29, 13, 2, tzinfo=timezone.utc)`.
    fn now() -> Option<Timestamp> {
        Some("2026-08-29T13:02:00Z".parse().unwrap())
    }

    fn scratch(name: &str) -> PathBuf {
        let dir = std::env::temp_dir().join(format!(
            "qo-info-{name}-{}-{:?}",
            std::process::id(),
            std::thread::current().id()
        ));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        dir
    }

    fn meta_of(path: &Path) -> Mapping {
        split_frontmatter(&pystr::read_text(path).unwrap()).unwrap().0
    }

    fn field(meta: &Mapping, key: &str) -> String {
        crate::yaml::get(meta, key).map(pystr::yaml_str).unwrap_or_else(|| "<absent>".into())
    }

    fn parent_name(path: &Path) -> String {
        path.parent().and_then(|p| p.file_name()).unwrap().to_string_lossy().to_string()
    }

    fn open(vault: &Path, item: NewInfo<'_>) -> PathBuf {
        open_info(vault, &item, &ctx(), None, now()).unwrap()
    }

    fn date(text: &str) -> Date {
        text.parse().unwrap()
    }

    #[test]
    fn test_open_then_close_by_key_archives_with_closed_by() {
        let vault = scratch("openclose");
        let path = open(
            &vault,
            NewInfo {
                title: "Package arrived at the Blount desk",
                kind: "package",
                body: "Pick up before 6pm.",
                opened_by: "gmail:1a04a6",
                close_key: Some("gmail-thread:1a04a6"),
                expires: Some(date("2026-09-05")),
            },
        );
        let meta = meta_of(&path);
        assert_eq!(parent_name(&path), "info");
        assert_eq!(field(&meta, "type"), "info");
        assert_eq!(field(&meta, "status"), "open");
        assert!(field(&meta, "id").starts_with("info_"));
        assert_eq!(field(&meta, "kind"), "package");
        assert_eq!(field(&meta, "close_key"), "gmail-thread:1a04a6");
        assert_eq!(field(&meta, "opened_by"), "gmail:1a04a6");
        // Python reads `date(2026, 9, 5)` here; serde_yaml_ng reads the same text as a string.
        assert_eq!(field(&meta, "expires"), "2026-09-05");

        let closed =
            close_info(&vault, Some("gmail-thread:1a04a6"), None, "gmail:1a04b0", &ctx(), None, now())
                .unwrap();
        assert_eq!(closed.len(), 1);
        assert_eq!(parent_name(&closed[0]), "archive");
        assert!(!path.exists());
        let archived = meta_of(&closed[0]);
        assert_eq!(field(&archived, "status"), "closed");
        assert_eq!(field(&archived, "closed_by"), "gmail:1a04b0");
        assert!(list_info(&vault).is_empty());
        let _ = std::fs::remove_dir_all(&vault);
    }

    /// The whole note, byte for byte. The frontmatter is corpus case 184 with this run's id —
    /// the same title, key, date and `now` — so this is the artefact check for `open_info`, not
    /// only for the emitter. On disk the file is CRLF, as `Path.write_text` leaves it on Windows.
    #[test]
    fn the_note_is_the_pyyaml_reference_shape_and_crlf_on_disk() {
        let vault = scratch("bytes");
        let path = open(
            &vault,
            NewInfo {
                title: "Package arrived at the Blount desk",
                kind: "package",
                body: "  Pick up before 6pm.\n\n",
                opened_by: "gmail:1a04a6",
                close_key: Some("gmail-thread:1a04a6"),
                expires: Some(date("2026-09-05")),
            },
        );
        let id = field(&meta_of(&path), "id");
        let expected = format!(
            "---\ntype: info\nid: {id}\nstatus: open\nkind: package\n\
             title: Package arrived at the Blount desk\nopened_at: '2026-08-29T13:02:00.000Z'\n\
             opened_by: gmail:1a04a6\nclose_key: gmail-thread:1a04a6\nexpires: 2026-09-05\n\
             closed_at: null\nclosed_by: null\n---\n\nPick up before 6pm.\n"
        );
        assert_eq!(pystr::read_text(&path).unwrap(), expected);
        let raw = std::fs::read(&path).unwrap();
        let expected_raw = expected.replace('\n', pystr::NEWLINE);
        assert_eq!(raw, expected_raw.as_bytes());

        // A null key, no expiry, and an empty body: corpus case 185's shape, and no body line.
        let path = open(
            &vault,
            NewInfo {
                title: "GN 103 Hausaufgaben — due Fri 08-28",
                kind: "notice",
                body: "   ",
                opened_by: "quinn",
                close_key: None,
                expires: None,
            },
        );
        let id = field(&meta_of(&path), "id");
        assert_eq!(
            pystr::read_text(&path).unwrap(),
            format!(
                "---\ntype: info\nid: {id}\nstatus: open\nkind: notice\n\
                 title: GN 103 Hausaufgaben — due Fri 08-28\nopened_at: '2026-08-29T13:02:00.000Z'\n\
                 opened_by: quinn\nclose_key: null\nexpires: null\nclosed_at: null\nclosed_by: null\n\
                 ---\n\n"
            )
        );
        let _ = std::fs::remove_dir_all(&vault);
    }

    #[test]
    fn test_close_by_key_is_exact_never_fuzzy() {
        let vault = scratch("exact");
        open(
            &vault,
            NewInfo {
                title: "A",
                kind: "notice",
                body: "",
                opened_by: "quinn",
                close_key: Some("ups:1Z999"),
                expires: None,
            },
        );
        assert!(close_info(&vault, Some("ups:1Z99"), None, "quinn", &ctx(), None, now())
            .unwrap()
            .is_empty());
        assert_eq!(list_info(&vault).len(), 1);
        let _ = std::fs::remove_dir_all(&vault);
    }

    #[test]
    fn test_close_by_id_and_unknown_kind_rejected() {
        let vault = scratch("byid");
        let path = open(
            &vault,
            NewInfo { title: "A", kind: "heads-up", body: "", opened_by: "quinn", ..Default::default() },
        );
        let id = field(&meta_of(&path), "id");
        assert_eq!(
            close_info(&vault, None, Some(&id), "quinn", &ctx(), None, now()).unwrap().len(),
            1
        );
        let bad = NewInfo { title: "B", kind: "rumour", body: "", opened_by: "quinn", ..Default::default() };
        assert_eq!(
            open_info(&vault, &bad, &ctx(), None, now()),
            Err(InfoError::UnknownKind("rumour".into()))
        );
        let _ = std::fs::remove_dir_all(&vault);
    }

    #[test]
    fn test_info_pass_closes_expired_items_only() {
        let vault = scratch("pass");
        for (title, expires) in [("Old", Some("2026-08-28")), ("Fresh", Some("2026-08-29")), ("Forever", None)] {
            open(
                &vault,
                NewInfo {
                    title,
                    kind: "notice",
                    body: "",
                    opened_by: "quinn",
                    close_key: None,
                    expires: expires.map(date),
                },
            );
        }
        let log = info_pass(&vault, date("2026-08-29"), &ctx(), None).unwrap();
        let mut titles: Vec<String> = list_info(&vault).iter().map(|i| field(i, "title")).collect();
        titles.sort();
        // Expiry is exclusive of today, like approvals.
        assert_eq!(titles, vec!["Forever", "Fresh"]);
        assert!(log.iter().any(|l| l.contains("expired")), "{log:?}");
        assert!(log[0].starts_with("expired info info_") && log[0].ends_with(" (Old)"), "{log:?}");
        let archived = sorted_md(&vault.join("archive"));
        assert_eq!(archived.len(), 1);
        assert_eq!(field(&meta_of(&archived[0]), "closed_by"), "system:expiry");
        assert!(info_pass(&vault, date("2026-08-29"), &ctx(), None).unwrap().is_empty());
        let _ = std::fs::remove_dir_all(&vault);
    }

    #[test]
    fn test_close_by_key_settles_every_open_item_with_that_key() {
        let vault = scratch("settle");
        for (title, opened_by, key) in [
            ("Pkg A", "gmail:1", "ups:1Z"),
            ("Pkg A again", "gmail:2", "ups:1Z"),
            ("Other", "gmail:3", "ups:2Z"),
        ] {
            open(
                &vault,
                NewInfo {
                    title,
                    kind: "package",
                    body: "",
                    opened_by,
                    close_key: Some(key),
                    expires: None,
                },
            );
        }
        let closed = close_info(&vault, Some("ups:1Z"), None, "gmail:4", &ctx(), None, now()).unwrap();
        assert_eq!(closed.len(), 2);
        let titles: Vec<String> = list_info(&vault).iter().map(|i| field(i, "title")).collect();
        assert_eq!(titles, vec!["Other"]);
        let _ = std::fs::remove_dir_all(&vault);
    }

    #[test]
    fn test_close_refuses_key_and_id_together() {
        let vault = scratch("both");
        let path = open(
            &vault,
            NewInfo {
                title: "A",
                kind: "notice",
                body: "",
                opened_by: "quinn",
                close_key: Some("k"),
                expires: None,
            },
        );
        let id = field(&meta_of(&path), "id");
        assert_eq!(
            close_info(&vault, Some("k"), Some(&id), "quinn", &ctx(), None, now()),
            Err(InfoError::KeyAndId)
        );
        assert_eq!(close_info(&vault, None, None, "quinn", &ctx(), None, now()), Err(InfoError::NeedsKeyOrId));
        // Python's `not key`: an empty string is as good as absent.
        assert_eq!(close_info(&vault, Some(""), Some(""), "quinn", &ctx(), None, now()), Err(InfoError::NeedsKeyOrId));
        assert_eq!(list_info(&vault).len(), 1);
        let _ = std::fs::remove_dir_all(&vault);
    }

    #[test]
    fn test_cli_open_close_list_and_bad_expires() {
        let vault = scratch("cli");
        let open = InfoCommand::Open {
            title: "Box".into(),
            kind: "package".into(),
            body: String::new(),
            opened_by: "quinn".into(),
            close_key: Some("ups:9".into()),
            expires: Some("2026-09-05".into()),
        };
        let out = cli(&vault, "quinn", "cli", &open).unwrap();
        assert_eq!(out.len(), 1);
        assert!(out[0].starts_with("opened info_") && out[0].ends_with(".md"), "{out:?}");

        let out = cli(&vault, "quinn", "cli", &InfoCommand::List).unwrap();
        assert_eq!(out.len(), 1);
        assert!(out[0].contains("Box"), "{out:?}");
        // `f"{item['id']}  {item['kind']:8}  expires={item.get('expires')}  {item['title']}"`
        assert!(out[0].contains("  package   expires=2026-09-05  Box"), "{out:?}");

        let close = InfoCommand::Close { key: Some("ups:9".into()), id: None, closed_by: "quinn".into() };
        let out = cli(&vault, "quinn", "cli", &close).unwrap();
        assert!(out.len() == 1 && out[0].contains("->"), "{out:?}");
        assert!(list_info(&vault).is_empty());

        let bad = InfoCommand::Open {
            title: "Bad".into(),
            kind: "notice".into(),
            body: String::new(),
            opened_by: "quinn".into(),
            close_key: None,
            expires: Some("next tuesday".into()),
        };
        assert_eq!(
            cli(&vault, "quinn", "cli", &bad),
            Err(CliError::Usage("--expires must be YYYY-MM-DD, got 'next tuesday'".into()))
        );
        let both = InfoCommand::Close { key: Some("k".into()), id: Some("info_0000000000".into()), closed_by: "quinn".into() };
        assert_eq!(
            cli(&vault, "quinn", "cli", &both),
            Err(CliError::Usage("close takes --key or --id, not both".into()))
        );
        let _ = std::fs::remove_dir_all(&vault);
    }

    /// `expires=None` prints as Python's `None`, and an item with no expiry is listed.
    #[test]
    fn list_prints_none_for_a_missing_expiry() {
        let vault = scratch("clinone");
        open(
            &vault,
            NewInfo { title: "Forever", kind: "other", body: "", opened_by: "quinn", ..Default::default() },
        );
        let out = cli(&vault, "quinn", "cli", &InfoCommand::List).unwrap();
        assert!(out[0].ends_with("  other     expires=None  Forever"), "{out:?}");
        let _ = std::fs::remove_dir_all(&vault);
    }
}
