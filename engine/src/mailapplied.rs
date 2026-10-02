//! `knowlu-engine mail-applied` (email spec 2026-09-30, §4.14 and §6.1; plan T7).
//!
//! Read-only: it lists what the mail pass wrote as `agent:knowlu.mail`, joined to each note's
//! current value, so the app can show the student what forwarded mail changed. It never writes
//! and is never entitlement-gated, like `surface`. The dispatch line in `cli.rs` is H2.
//!
//! The journal is the only source of what mail did. A direct change is a `set` record followed by
//! the `append_body` record of its `<date> · <sender>: <why> — "<evidence>"` line (§5.2), which is
//! where the row's sender and date come from. `mail_asof` is the pass's own bookkeeping and never a
//! row. A row's `changed_since` is true when its note is gone or no longer holds what mail left
//! there; the app then shows no Undo (§4.14).
//!
//! Output, per course (a notice has none, so its group has `course: null` and comes last):
//! `{since, courses: [{course, created, changed, archived: {count, items}, notices}]}`.

use std::collections::BTreeMap;
use std::path::Path;

use serde_json::{json, Map, Value};

use crate::ids;
use crate::journal::Journal;
use crate::ledger::Record;
use crate::mail::MAIL_ACTOR;
use crate::yaml;

#[cfg(test)]
mod tests;

/// The printed JSON, through `ledger::dumps_value` (Python's separators).
pub fn run(vault: &Path, since: Option<&str>) -> String {
    crate::ledger::dumps_value(&applied(vault, since))
}

/// One course's rows.
#[derive(Default)]
struct Group {
    created: Vec<Value>,
    changed: Vec<Value>,
    archived: Vec<Value>,
    notices: Vec<Value>,
}

fn text<'a>(rec: &'a Record, key: &str) -> Option<&'a str> {
    rec.get(key).and_then(Value::as_str)
}

fn field(rec: &Record, key: &str) -> Value {
    rec.get(key).cloned().unwrap_or(Value::Null)
}

fn is_mail(rec: &Record) -> bool {
    text(rec, "actor") == Some(MAIL_ACTOR)
}

/// What the journal says about every note id: where it is now (`None`: gone), and the course a mail
/// create gave it.
struct Notes {
    home: BTreeMap<String, Option<String>>,
    course: BTreeMap<String, Option<String>>,
    /// The last value mail itself set per `(id, field)`.
    mail_set: BTreeMap<(String, String), Value>,
}

impl Notes {
    fn read(records: &[Record]) -> Notes {
        let mut notes = Notes { home: BTreeMap::new(), course: BTreeMap::new(), mail_set: BTreeMap::new() };
        for rec in records {
            let Some(id) = text(rec, "id").filter(|id| !id.is_empty()) else { continue };
            match text(rec, "op") {
                Some("create") => {
                    notes.home.insert(id.to_string(), text(rec, "path").map(str::to_string));
                    let course = rec.get("new").and_then(|n| n.get("course")).and_then(Value::as_str);
                    notes.course.insert(id.to_string(), course.map(str::to_string));
                }
                Some("move") => {
                    notes.home.insert(id.to_string(), text(rec, "new").map(str::to_string));
                }
                Some("delete") => {
                    notes.home.insert(id.to_string(), None);
                }
                Some("set") if is_mail(rec) => {
                    if let Some(name) = text(rec, "field") {
                        notes.mail_set.insert((id.to_string(), name.to_string()), field(rec, "new"));
                    }
                }
                _ => {}
            }
        }
        notes
    }

    /// The note's frontmatter as it is on disk now, and where.
    fn current(&self, vault: &Path, id: &str) -> Option<(String, serde_yaml_ng::Mapping)> {
        let rel = self.home.get(id)?.clone()?;
        let meta = ids::read_meta(&vault.join(&rel))?;
        Some((rel, meta))
    }
}

fn current_of(meta: Option<&(String, serde_yaml_ng::Mapping)>, key: &str) -> Value {
    meta.and_then(|(_, m)| yaml::get(m, key)).map_or(Value::Null, yaml::to_json)
}

/// `<date> · <sender>: <why> — "<evidence>"` into its date and sender. A sender is `Name <address>`
/// or an address alone (D21); the name is mail text and may hold `": "`, so an address is told by
/// its `@` and no whitespace.
fn parse_line(line: &str) -> (Option<String>, Option<String>) {
    let Some((day, rest)) = line.split_once(" \u{b7} ") else { return (None, None) };
    let shaped = day.len() == 10
        && day.bytes().enumerate().all(|(i, b)| if i == 4 || i == 7 { b == b'-' } else { b.is_ascii_digit() });
    let day = shaped.then(|| day.to_string());
    if let Some(i) = rest.find(": ") {
        let head = &rest[..i];
        if head.contains('@') && !head.contains(char::is_whitespace) && !head.contains('<') {
            return (day, Some(head.to_string()));
        }
    }
    let mut from = 0;
    while let Some(offset) = rest[from..].find(">: ") {
        let end = from + offset + 1;
        let candidate = &rest[..end];
        if let Some(lt) = candidate.rfind('<') {
            let address = &candidate[lt + 1..end - 1];
            if address.contains('@') && !address.contains(char::is_whitespace) {
                return (day, Some(candidate.to_string()));
            }
        }
        from = end;
    }
    (day, None)
}

/// The body line mail appended for the `set` at `at`: the next `append_body` by mail on the same
/// note, unless another field's `set` comes first (the writer appends straight after its sets).
fn body_line(records: &[Record], at: usize) -> Option<&str> {
    let id = text(&records[at], "id")?;
    for rec in &records[at + 1..] {
        if text(rec, "id") != Some(id) {
            continue;
        }
        match text(rec, "op") {
            Some("append_body") if is_mail(rec) => return text(rec, "new"),
            Some("set") if text(rec, "field") != Some("mail_asof") => return None,
            _ => {}
        }
    }
    None
}

/// What mail wrote, grouped by course. `since` is a journal timestamp (or a bare date), compared as
/// text like the ledger's own bounds; `None` is every record by mail.
pub fn applied(vault: &Path, since: Option<&str>) -> Value {
    let records = Journal::new(vault).read(None, None);
    let notes = Notes::read(&records);
    let since = since.filter(|s| !s.is_empty());
    let mut groups: BTreeMap<(bool, String), Group> = BTreeMap::new();
    let mut first: Option<String> = None;
    for (at, rec) in records.iter().enumerate().filter(|(_, r)| is_mail(r)) {
        let stamp = text(rec, "ts").unwrap_or_default();
        if since.is_some_and(|s| stamp < s) {
            continue;
        }
        let (Some(id), Some(path)) = (text(rec, "id").filter(|id| !id.is_empty()), text(rec, "path")) else { continue };
        let now = notes.current(vault, id);
        let (key, row, kind) = match text(rec, "op") {
            Some("create") if path.starts_with("info/") => notice_row(rec, id, path, stamp, now.as_ref()),
            Some("create") if path.starts_with("tasks/") || path.starts_with("archive/") => {
                created_row(rec, id, path, stamp, now.as_ref(), &notes)
            }
            Some("set") if !path.starts_with("approvals/") && !path.starts_with("info/") => {
                let Some(name) = text(rec, "field").filter(|f| *f != "mail_asof") else { continue };
                changed_row(records.as_slice(), at, name, now.as_ref(), &notes)
            }
            _ => continue,
        };
        first.get_or_insert_with(|| stamp.to_string());
        let course = key.or_else(|| notes.course.get(id).cloned().flatten());
        let group = groups.entry((course.is_none(), course.clone().unwrap_or_default())).or_default();
        match kind {
            Kind::Created => group.created.push(row),
            Kind::Archived => group.archived.push(row),
            Kind::Changed => group.changed.push(row),
            Kind::Notice => group.notices.push(row),
        }
    }
    let courses: Vec<Value> = groups.into_iter().map(|((none, name), g)| group_json(none, name, g)).collect();
    json!({"since": since.map(str::to_string).or(first), "courses": courses})
}

fn group_json(none: bool, name: String, g: Group) -> Value {
    let course = if none { Value::Null } else { Value::String(name) };
    json!({
        "course": course,
        "created": g.created,
        "changed": g.changed,
        "archived": {"count": g.archived.len(), "items": g.archived},
        "notices": g.notices,
    })
}

enum Kind {
    Created,
    Archived,
    Changed,
    Notice,
}

type Row = (Option<String>, Value, Kind);
type Now = (String, serde_yaml_ng::Mapping);

fn created_row(rec: &Record, id: &str, path: &str, stamp: &str, now: Option<&Now>, notes: &Notes) -> Row {
    let new = rec.get("new").and_then(Value::as_object).cloned().unwrap_or_default();
    let said = |name: &str| new.get(name).cloned().unwrap_or(Value::Null);
    // Mail's own later sets on the note are expected there; anything else is a change since.
    let expected = |name: &str| notes.mail_set.get(&(id.to_string(), name.to_string())).cloned().unwrap_or_else(|| said(name));
    let (title, due) = (current_of(now, "title"), current_of(now, "due"));
    let changed_since = now.is_none() || title != expected("title") || due != expected("due");
    let current = now.map(|_| json!({"title": title, "due": due, "status": current_of(now, "status")}));
    let row = json!({
        "path": now.map_or(path, |(rel, _)| rel.as_str()), "id": id, "uid": said("source_uid"),
        "title": said("title"), "due": said("due"), "ts": stamp,
        "current": current, "changed_since": changed_since,
    });
    let kind = if path.starts_with("archive/") { Kind::Archived } else { Kind::Created };
    (new.get("course").and_then(Value::as_str).map(str::to_string), row, kind)
}

fn changed_row(records: &[Record], at: usize, name: &str, now: Option<&Now>, notes: &Notes) -> Row {
    let rec = &records[at];
    let id = text(rec, "id").unwrap_or_default();
    let (day, sender) = body_line(records, at).map_or((None, None), parse_line);
    let current = current_of(now, name);
    let row = json!({
        "path": now.map_or(text(rec, "path").unwrap_or_default(), |(rel, _)| rel.as_str()), "id": id,
        "title": current_of(now, "title"), "field": name, "old": field(rec, "old"), "new": field(rec, "new"),
        "current": current, "sender": sender, "date": day, "ts": text(rec, "ts"),
        "changed_since": now.is_none() || current != field(rec, "new"),
    });
    let course = current_of(now, "course").as_str().map(str::to_string);
    (notes.course.get(id).cloned().flatten().or(course), row, Kind::Changed)
}

fn notice_row(rec: &Record, id: &str, path: &str, stamp: &str, now: Option<&Now>) -> Row {
    let new: Map<String, Value> = rec.get("new").and_then(Value::as_object).cloned().unwrap_or_default();
    let open = current_of(now, "status") == json!("open");
    let row = json!({
        "path": now.map_or(path, |(rel, _)| rel.as_str()), "id": id, "kind": new.get("kind"),
        "title": new.get("title"), "opened_at": new.get("opened_at"), "ts": stamp, "changed_since": !open,
    });
    (None, row, Kind::Notice)
}
