//! T6.2's tests: the writers, against a scratch vault. Helpers here; the cases beside them.
//!
//! Every vault is a temporary directory and nothing touches a network (P16).

use std::path::{Path, PathBuf};

use jiff::civil::{date, Date};
use jiff::Timestamp;
use serde_json::json;

use super::*;
use crate::mailreconcile::{Authenticity, CardStatus, ClaimField, ClaimKind, ClaimSource, ItemKind, NewItem, Phase, Sender};

mod cards;
mod hostile;
mod notes;
mod sets;

pub(super) const VIA: &str = "local-runner";

pub(super) fn scratch(tag: &str) -> PathBuf {
    let vault = std::env::temp_dir().join(format!("knowlu-mail-{tag}-{}-{:?}", std::process::id(), std::thread::current().id()));
    let _ = std::fs::remove_dir_all(&vault);
    for folder in ["tasks", "approvals", "archive", "info", "state", "config"] {
        std::fs::create_dir_all(vault.join(folder)).unwrap();
    }
    vault
}

pub(super) fn ts(text: &str) -> Timestamp {
    text.parse().unwrap()
}

pub(super) fn today() -> Date {
    date(2026, 10, 1)
}

/// A card charged today.
pub(super) fn pending() -> Proposal {
    Proposal { status: CardStatus::Pending, proposed_at: today(), first_proposed_at: today(), snooze_until: None }
}

/// A card filed on the vault's first day: snoozed to day 2 (§4.8 step 5, R16).
pub(super) fn held() -> Proposal {
    let tomorrow = date(2026, 10, 2);
    Proposal { status: CardStatus::Snoozed, proposed_at: tomorrow, first_proposed_at: today(), snooze_until: Some(tomorrow) }
}

pub(super) fn claim(kind: ClaimKind) -> Claim {
    Claim {
        kind,
        course: Some("CS 101".to_string()),
        target: None,
        field: None,
        value: None,
        item: None,
        topic: None,
        evidence: "Lab 5 is now due Monday".to_string(),
        why: "The instructor moved the due date".to_string(),
        confidence: 0.9,
        source: ClaimSource::Model,
        resolution: None,
    }
}

pub(super) fn new_item(title: &str, due: Option<&str>) -> Claim {
    let item = NewItem { title: title.to_string(), due: due.map(str::to_string), item_kind: ItemKind::Task, effort_hours: 2.5, importance: 4 };
    Claim { item: Some(item), ..claim(ClaimKind::NewItem) }
}

pub(super) fn change(target: &str, value: &str) -> Claim {
    let (target, value) = (Some(target.to_string()), Some(value.to_string()));
    Claim { target, field: Some(ClaimField::Due), value, ..claim(ClaimKind::Change) }
}

pub(super) fn message(uid: &str, phase: Phase, claims: Vec<Claim>) -> Message {
    Message {
        uid: uid.to_string(),
        ordering_date: ts("2026-10-01T12:00:00Z"),
        message_date: Some(ts("2026-10-01T11:00:00Z")),
        phase,
        authenticity: Authenticity::Aligned,
        sender: Sender { name: Some("Course Staff".to_string()), address: "staff@example.edu".to_string() },
        claims,
    }
}

pub(super) fn at(uid: &str, index: usize) -> ClaimRef {
    ClaimRef { message: uid.to_string(), index }
}

/// A note minted as a mail note: through `write::create`, as the mail actor.
pub(super) fn mail_note(vault: &Path, rel: &str, due: &str) -> String {
    let text = format!(
        "---\ntitle: Lab 5\ncourse: CS 101\ndomain: school\ndue: {due}\nimportance: 4\nstatus: active\n\
         created_by: mail\nsource_uid: mail:aa:0\n---\n\nFrom email.\n"
    );
    let ctx = WriteContext::new(MAIL_ACTOR, VIA);
    write::create(vault, rel, &text, &ctx, &mut Journal::new(vault), None).unwrap();
    crate::yaml::get(&ids::read_meta(&vault.join(rel)).unwrap(), "id").and_then(crate::yaml::text).unwrap()
}

pub(super) fn read(vault: &Path, rel: &str) -> String {
    crate::pystr::read_text(&vault.join(rel)).unwrap()
}

pub(super) fn meta(vault: &Path, rel: &str) -> serde_yaml_ng::Mapping {
    ids::read_meta(&vault.join(rel)).unwrap_or_else(|| panic!("{rel} has no frontmatter"))
}

pub(super) fn field(vault: &Path, rel: &str, key: &str) -> Option<serde_json::Value> {
    crate::yaml::get(&meta(vault, rel), key).map(crate::yaml::to_json)
}

pub(super) fn files(vault: &Path, folder: &str) -> Vec<String> {
    let mut names: Vec<String> = std::fs::read_dir(vault.join(folder))
        .map(|dir| dir.flatten().map(|e| e.file_name().to_string_lossy().to_string()).collect())
        .unwrap_or_default();
    names.sort();
    names
}

/// Run one action through a fresh writer.
pub(super) fn apply(vault: &Path, action: &Action, message: &Message, index: usize, held: Proposal) -> Wrote {
    let mut journal = Journal::new(vault);
    let mut writer = Writer::new(vault, VIA, None, &mut journal, today(), held);
    let src = Source { message, claim: &message.claims[index], judgment_id: Some("0b1c2d3e-0000-4000-8000-000000000001") };
    writer.apply(action, &src).unwrap()
}

pub(super) fn set_of(claim: ClaimRef, target: Place, field: &str, to: serde_json::Value, asof: &str) -> Action {
    Action::Set {
        claim,
        target,
        field: field.to_string(),
        to,
        mail_asof: ts(asof),
        body_line: "2026-10-01 \u{b7} Course Staff <staff@example.edu>: The instructor moved the due date \u{2014} \"Lab 5 is now due Monday\"".to_string(),
    }
}

pub(super) fn set_due(claim: ClaimRef, rel: &str, to: &str, asof: &str) -> Action {
    set_of(claim, Place::Path(rel.to_string()), "due", json!(to), asof)
}

pub(super) fn due_card(claim: ClaimRef, rel: &str, from: serde_json::Value, to: &str, proposal: Proposal) -> Action {
    let changes = vec![Change { field: "due".to_string(), from, to: json!(to) }];
    Action::AmendCard { claim, target: Place::Path(rel.to_string()), changes, proposal, also: Vec::new() }
}
