//! The vault as the reconciler sees it (plan T6.3, T5.6's addendum): a [`VaultView`] built from `tasks/`,
//! `archive/` and `approvals/` and the journal. Read-only.
//!
//! - Each note's `kind`, `archived_reason` and `mail_asof` are as its frontmatter holds them.
//! - A `kind: task` card is viewed as the task it proposes: its own frontmatter gives `status`,
//!   `created_by` and `source_uid`, its fenced `task` payload's gives `title`, `course` and `due`
//!   (`write_gmail_card`'s shape puts neither `course:` nor `due:` on the card). The fence is found by
//!   `approvals::materialize`'s pattern, mirrored here; the payload is only read, never re-dumped. A card
//!   whose payload has no frontmatter or no `course:` keeps `course: None`, so it matches no claim: a
//!   second card, never a dropped task.
//! - `human_set` is [`Journal::human_set`]'s answer for every field, taken in one pass over the journal:
//!   asking per note and field would clone the whole journal each time. The first unit test below pins
//!   the two together.

use std::collections::{BTreeMap, BTreeSet};
use std::path::Path;
use std::sync::LazyLock;

use jiff::Timestamp;
use regex::Regex;
use serde_json::Value as Json;
use serde_yaml_ng::{Mapping, Value as Yaml};

use crate::approvals::sorted_md;
use crate::ids;
use crate::journal::{is_human, Journal};
use crate::mailreconcile::{NoteView, VaultView};
use crate::models::split_frontmatter;
use crate::pystr;
use crate::yaml;

const FOLDERS: [&str; 3] = ["tasks", "archive", "approvals"];

/// `approvals::PAYLOAD_FENCE` (private there; `approvals.rs` is not edited).
static PAYLOAD_FENCE: LazyLock<Regex> = LazyLock::new(|| Regex::new(r"(?sm)^```task[ \t]*\n(.*?)^```[ \t]*$").unwrap());

/// The view of every note a claim may resolve against or touch.
pub(super) fn build(vault: &Path, journal: &mut Journal) -> VaultView {
    journal.invalidate();
    let human = human_fields(journal);
    let notes = FOLDERS
        .iter()
        .flat_map(|folder| sorted_md(&vault.join(folder)).into_iter().map(move |path| (*folder, path)))
        .filter_map(|(folder, path)| note_view(vault, folder, &path, &human))
        .collect();
    VaultView { notes }
}

/// Note id to the fields [`Journal::human_set`] finds a record for: a human `set` of the field, or a
/// human `create` whose `new` mapping carries it.
pub(super) fn human_fields(journal: &mut Journal) -> BTreeMap<String, BTreeSet<String>> {
    let mut found: BTreeMap<String, BTreeSet<String>> = BTreeMap::new();
    for record in journal.read(None, None) {
        let text = |key: &str| record.get(key).and_then(Json::as_str);
        let (Some(id), Some(actor)) = (text("id").filter(|id| !id.is_empty()), text("actor")) else { continue };
        if !is_human(actor) {
            continue;
        }
        match text("op") {
            Some("set") => {
                if let Some(field) = text("field") {
                    found.entry(id.to_string()).or_default().insert(field.to_string());
                }
            }
            Some("create") => {
                if let Some(Json::Object(new)) = record.get("new") {
                    found.entry(id.to_string()).or_default().extend(new.keys().cloned());
                }
            }
            _ => {}
        }
    }
    found
}

fn note_view(vault: &Path, folder: &str, path: &Path, human: &BTreeMap<String, BTreeSet<String>>) -> Option<NoteView> {
    let text = pystr::read_text(path).ok()?;
    if !text.starts_with("---") {
        return None;
    }
    let (meta, body) = split_frontmatter(&text).ok()?;
    let rel = ids::rel(vault, path);
    let read = |meta: &Mapping, key: &str| yaml::opt_text(yaml::get(meta, key));
    let kind = read(&meta, "kind");
    // A card proposing a task: the task's own title, course and due are in its payload.
    let payload = (folder == "approvals" && kind.as_deref() == Some("task")).then(|| payload_meta(&body));
    let item = payload.as_ref().unwrap_or(&meta);
    let id = read(&meta, "id").unwrap_or_else(|| ids::derived_id(&ids::kind_for(path, Some(&meta)), &rel));
    let title = read(item, "title").or_else(|| read(&meta, "title")).unwrap_or_default();
    Some(NoteView {
        human_set: human.get(&id).cloned().unwrap_or_default(),
        id,
        path: rel,
        folder: folder.to_string(),
        kind,
        course: read(item, "course"),
        title,
        due: yaml::get(item, "due").map_or(Json::Null, yaml::to_json),
        status: read(&meta, "status"),
        archived_reason: read(&meta, "archived_reason"),
        source_uid: read(&meta, "source_uid"),
        created_by: read(&meta, "created_by"),
        mail_asof: mail_asof(&meta),
    })
}

/// The frontmatter of a card's fenced `task` payload; empty when it has no fence or no frontmatter.
fn payload_meta(body: &str) -> Mapping {
    let Some(caps) = PAYLOAD_FENCE.captures(body) else { return Mapping::new() };
    split_frontmatter(&caps[1]).map(|(meta, _)| meta).unwrap_or_default()
}

/// `mail_asof: {due: '<ordering date>'}`: a stamp that does not parse is no stamp.
fn mail_asof(meta: &Mapping) -> BTreeMap<String, Timestamp> {
    let Some(Yaml::Mapping(stamps)) = yaml::get(meta, "mail_asof") else { return BTreeMap::new() };
    stamps.iter().filter_map(|(field, at)| Some((yaml::text(field)?, yaml::text(at)?.parse().ok()?))).collect()
}
