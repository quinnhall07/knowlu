//! The mail pass's device half (email spec 2026-09-30, §6.1; plan T6).
//!
//! Pulls forwarded-mail claims through [`crate::cloudmodel`], plans them with
//! [`crate::mailreconcile`] (pure) and writes the planned actions through `write` as
//! `agent:knowlu.mail`, journal first. Transport and writing only: `rank` never reaches it and
//! no mail text, address or session enters a log, the journal or telemetry.
//!
//! T6.2 (this file's [`Writer`]) holds one writer per [`Action`] kind and decides nothing: card or
//! direct was decided by `mailreconcile::plan` (D11), and the one thing a writer re-checks is that a
//! student's `set` has not landed since the view was built, which can only turn a direct set into a
//! card (plan §10, R10). T6.3 adds the pass; T6.4 asks `mail-resolve` per course before it plans
//! ([`asks`]) and calls it from `enrich`.

use std::collections::BTreeMap;
use std::path::Path;

use jiff::civil::Date;
use jiff::Timestamp;
use serde_json::Value as Json;

use crate::completion;
use crate::ids;
use crate::info::{self, InfoError, NewInfo};
use crate::ingest::slugify;
use crate::journal::Journal;
use crate::mailreconcile::{self, Action, Change, Claim, ClaimRef, Message, NewNote, Place, Proposal, SkipReason};
use crate::write::{self, WriteContext, WriteError, WriteOpts};

mod asks;
mod cards;
mod heads;
mod held;
mod pass;
mod text;
mod view;

#[cfg(test)]
mod tests;

pub use pass::{pull_mail, pull_mail_on};
use text::{asof_literal, json_lit, note_text, single_lines, Origin};

/// D20: the journal actor of every write the mail pass makes. `agent:` keeps judge-once.
pub const MAIL_ACTOR: &str = "agent:knowlu.mail";

/// The claim an action came from, as the writers need it: its message (sender, dates), the claim
/// (`why`, `evidence`) and the service's `judgment_id` for it, when it wrote one.
#[derive(Debug, Clone, Copy)]
pub struct Source<'a> {
    pub message: &'a Message,
    pub claim: &'a Claim,
    pub judgment_id: Option<&'a str>,
}

/// What one writer did, for T6.3's run line.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Wrote {
    /// A new note, at this vault-relative path.
    Note(String),
    /// A direct change to the note at this path.
    Set(String),
    /// A `kind: task` or `kind: amend` card, at this path. A direct set a student's `set` pre-empted
    /// is a card too.
    Card(String),
    /// An `info/` item, at this path.
    Notice(String),
    /// A submission receipt's "mark done" card, at this path.
    Receipt(String),
    /// Nothing was written.
    Nothing(Why),
}

/// Why a writer wrote nothing.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Why {
    /// A `Skip` action: the reconciler already named the reason.
    Skipped(SkipReason),
    /// An amend card for this note and field is already open, `pending` or `snoozed`, whoever filed
    /// it (§9.5; [`ingest::open_amendment`]).
    OpenCard,
    /// The action names a note a `Create` of this run was to file and did not (plan §9.5): counted,
    /// never retargeted.
    NoPlannedNote,
    /// The note already holds the value (a student set it between plan and write).
    Equal,
    /// `approvals::validate_amendment` would refuse the card, so none is filed. T6.3 counts it as
    /// `NotAmendable` (PQ7: in `U`).
    NotAmendable,
    /// `completion::propose_done` declined, with its own reason (already asked, status set by hand).
    ReceiptDeclined(String),
}

/// The writers. Holds the run's context and the path each `Create` of this run wrote, so a later
/// action naming `Place::Planned` reaches it (plan §9.5).
pub struct Writer<'a> {
    vault: &'a Path,
    ctx: WriteContext,
    journal: &'a mut Journal,
    today: Date,
    /// The proposal a direct set carries when it turns into a card (R10): the day's, held on day 1.
    held: Proposal,
    planned: BTreeMap<ClaimRef, String>,
}

impl<'a> Writer<'a> {
    /// `via` is the slot's (`local-runner`); `held` is the proposal a card made from a direct set
    /// carries, which T6.3 takes from `mailreconcile`'s own day rule.
    pub fn new(
        vault: &'a Path,
        via: &str,
        run_id: Option<&str>,
        journal: &'a mut Journal,
        today: Date,
        held: Proposal,
    ) -> Writer<'a> {
        let ctx = WriteContext { actor: MAIL_ACTOR.to_string(), via: via.to_string(), run_id: run_id.map(str::to_string) };
        Writer { vault, ctx, journal, today, held, planned: BTreeMap::new() }
    }

    /// Write one planned action, through `write`, journal first.
    pub fn apply(&mut self, action: &Action, src: &Source<'_>) -> Result<Wrote, WriteError> {
        match action {
            Action::Create { claim, note } => self.create_note(claim, note, src),
            Action::Set { claim, target, field, to, mail_asof, body_line } => {
                self.set_field(claim, target, field, to, *mail_asof, body_line, src)
            }
            Action::TaskCard { claim, note, proposal } => self.task_card(claim, note, proposal, src),
            Action::AmendCard { claim, target, changes, proposal, also } => match self.place(target) {
                Some(rel) => self.amend_card(claim, &rel, changes, proposal, also, src),
                None => Ok(Wrote::Nothing(Why::NoPlannedNote)),
            },
            Action::Notice { kind, close_key, title, body_line, .. } => {
                self.notice(kind, close_key.as_deref(), title, body_line)
            }
            Action::ProposeDone { claim, target } => self.propose_done(claim, target, src),
            Action::Skip { reason, .. } => Ok(Wrote::Nothing(Why::Skipped(*reason))),
        }
    }

    /// The vault-relative path a [`Place`] names now.
    fn place(&self, place: &Place) -> Option<String> {
        match place {
            Place::Path(path) => Some(path.clone()),
            Place::Planned(by) => self.planned.get(by).cloned(),
        }
    }

    fn origin<'s>(&self, at: &ClaimRef, src: &Source<'s>) -> Result<Origin<'s>, WriteError> {
        let origin = Origin {
            uid: at.uid(),
            judgment_id: src.judgment_id,
            sender: mailreconcile::sender_text(&src.message.sender),
            day: mailreconcile::message_day(src.message),
            why: &src.claim.why,
            evidence: &src.claim.evidence,
            body_line: mailreconcile::body_line(src.message, src.claim),
        };
        single_lines(&[("sender", &origin.sender), ("why", origin.why), ("evidence", origin.evidence)])?;
        Ok(origin)
    }

    /// The first free `<folder>/<stem>[-N].md`, ingest's own suffix rule.
    fn free_path(&self, folder: &str, stem: &str) -> String {
        let mut name = format!("{folder}/{stem}.md");
        let mut suffix = 2;
        while self.vault.join(&name).exists() {
            name = format!("{folder}/{stem}-{suffix}.md");
            suffix += 1;
        }
        name
    }

    /// §5.2's note, written directly. Past-due items go to `archive/` as `imported-past` (D24).
    fn create_note(&mut self, at: &ClaimRef, note: &NewNote, src: &Source<'_>) -> Result<Wrote, WriteError> {
        let origin = self.origin(at, src)?;
        single_lines(&[("title", &note.title), ("course", note.course.as_deref().unwrap_or(""))])?;
        let folder = if note.imported_past { "archive" } else { "tasks" };
        let stem = format!("{}-{}", slugify(note.course.as_deref().unwrap_or("task")), slugify(&note.title));
        let rel = self.free_path(folder, &stem);
        let path = write::create(self.vault, &rel, &note_text(note, &origin), &self.ctx, self.journal, None)?;
        let rel = ids::rel(self.vault, &path);
        self.planned.insert(at.clone(), rel.clone());
        Ok(Wrote::Note(rel))
    }

    /// §5.2's direct change: the field and `mail_asof` through `write_literals` with
    /// `WriteOpts::default()` (P17), then the one body line through `append_body`.
    ///
    /// Plan §10, R10: first `journal.invalidate()` and a fresh `human_set`. If a student set the
    /// field since the view was built, the amend card is filed instead and their value stands. The
    /// re-check never turns a card into a direct write.
    #[allow(clippy::too_many_arguments)]
    fn set_field(
        &mut self,
        at: &ClaimRef,
        target: &Place,
        field: &str,
        to: &Json,
        asof: Timestamp,
        body_line: &str,
        src: &Source<'_>,
    ) -> Result<Wrote, WriteError> {
        let Some(rel) = self.place(target) else { return Ok(Wrote::Nothing(Why::NoPlannedNote)) };
        single_lines(&[("body", body_line)])?;
        let meta = ids::read_meta(&self.vault.join(&rel)).ok_or_else(|| WriteError::NoFrontmatter(rel.clone()))?;
        let id = crate::yaml::get(&meta, "id").and_then(crate::yaml::text);
        self.journal.invalidate();
        if id.is_some_and(|id| self.journal.human_set(&id, field).is_some()) {
            let from = crate::yaml::get(&meta, field).map_or(Json::Null, crate::yaml::to_json);
            if &from == to {
                return Ok(Wrote::Nothing(Why::Equal));
            }
            let held = self.held;
            return self.amend_card(at, &rel, &[Change { field: field.to_string(), from, to: to.clone() }], &held, &[], src);
        }
        let mut stamps: Vec<(String, String)> = match crate::yaml::get(&meta, "mail_asof") {
            Some(serde_yaml_ng::Value::Mapping(map)) => map
                .iter()
                .filter_map(|(k, v)| Some((crate::yaml::text(k)?, crate::yaml::text(v)?)))
                .collect(),
            _ => Vec::new(),
        };
        match stamps.iter_mut().find(|(name, _)| name == field) {
            Some((_, stamp)) => *stamp = asof.to_string(),
            None => stamps.push((field.to_string(), asof.to_string())),
        }
        let literals = [(field.to_string(), json_lit(to)), ("mail_asof".to_string(), asof_literal(&stamps))];
        write::write_literals(self.vault, &rel, &literals, &self.ctx, self.journal, &WriteOpts::default())?;
        write::append_body(self.vault, &rel, body_line, &self.ctx, self.journal)?;
        Ok(Wrote::Set(rel))
    }

    /// D13: an `info/` item. A later notice with the same `close_key` supersedes the earlier one
    /// (spec test 26), so the open ones with that key are closed first. `kind` is one of
    /// `info::KINDS`; T6.3's heads-ups come through here too.
    pub fn notice(&mut self, kind: &str, close_key: Option<&str>, title: &str, body_line: &str) -> Result<Wrote, WriteError> {
        single_lines(&[("title", title), ("body", body_line), ("close_key", close_key.unwrap_or(""))])?;
        let io = |e: InfoError| match e {
            InfoError::Write(e) => e,
            other => WriteError::Io(other.to_string()),
        };
        if let Some(key) = close_key {
            info::close_info(self.vault, Some(key), None, MAIL_ACTOR, &self.ctx, Some(self.journal), None).map_err(io)?;
        }
        let item = NewInfo { title, kind, body: body_line, opened_by: MAIL_ACTOR, close_key, expires: None };
        let path = info::open_info(self.vault, &item, &self.ctx, Some(self.journal), None).map_err(io)?;
        Ok(Wrote::Notice(ids::rel(self.vault, &path)))
    }

    /// Close every open `info/` item with this `close_key` (a heads-up whose mailbox is active again).
    pub fn close_notices(&mut self, close_key: &str) -> Result<(), WriteError> {
        let closed = info::close_info(self.vault, Some(close_key), None, MAIL_ACTOR, &self.ctx, Some(self.journal), None);
        closed.map(|_| ()).map_err(|e| match e {
            InfoError::Write(e) => e,
            other => WriteError::Io(other.to_string()),
        })
    }

    /// PQ4: a template receipt is `completion::propose_done`, which always proposes and asks once
    /// per task. The evidence is the claim's uid and a fixed sentence, never mail text.
    fn propose_done(&mut self, at: &ClaimRef, target: &Place, src: &Source<'_>) -> Result<Wrote, WriteError> {
        let Some(rel) = self.place(target) else { return Ok(Wrote::Nothing(Why::NoPlannedNote)) };
        let mut detail = serde_json::Map::new();
        detail.insert("uid".to_string(), Json::String(at.uid()));
        let evidence = completion::Evidence {
            source: "email".to_string(),
            summary: "An email confirms this was submitted".to_string(),
            detail,
        };
        let judgment = src.judgment_id.map(|id| (id, "mail_claim"));
        let done = completion::propose_done(
            self.vault, Path::new(&rel), &evidence, self.today, &self.ctx, self.journal, false, judgment,
        )?;
        Ok(match done {
            completion::Outcome::Proposed(path) => Wrote::Receipt(ids::rel(self.vault, &path)),
            completion::Outcome::WouldPropose => Wrote::Nothing(Why::ReceiptDeclined("dry run".to_string())),
            completion::Outcome::Skipped(why) => Wrote::Nothing(Why::ReceiptDeclined(why)),
        })
    }
}
