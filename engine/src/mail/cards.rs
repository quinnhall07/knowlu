//! The card writer (plan T6.2, D11 (3), P13): `write::create` into `approvals/`, after
//! [`ingest::open_amendment`] finds no open card for the note and field.
//!
//! A card carries the `status`, `proposed_at`, `first_proposed_at` and `snooze_until` T5 planned, so
//! `approvals::defer_over_budget` charges it (plan §10, R3, R15, R16). An amend card is filed only
//! if `approvals::validate_amendment` accepts it as written: a card the executor would refuse is
//! never filed (the feed's rule, R14), and the claim is counted `NotAmendable`.

use crate::ingest::{self, slugify};
use crate::mailreconcile::{Change, ClaimRef, Claimed, NewNote, Proposal};
use crate::models::split_frontmatter;
use crate::write::{self, WriteError};
use crate::yaml;

use super::text::{amend_card_text, note_text, single_lines, task_card_text};
use super::{Source, Why, Wrote, Writer};

impl Writer<'_> {
    /// A `kind: task` card for a new item, in `write_gmail_card`'s shape. The stem starts `task-`,
    /// which `approvals::materialize` strips to name the note.
    pub(super) fn task_card(
        &mut self,
        at: &ClaimRef,
        note: &NewNote,
        proposal: &Proposal,
        src: &Source<'_>,
    ) -> Result<Wrote, WriteError> {
        let origin = self.origin(at, src)?;
        single_lines(&[("title", &note.title), ("course", note.course.as_deref().unwrap_or(""))])?;
        let text = task_card_text(proposal, &note.title, &note_text(note, &origin), &origin);
        let rel = self.free_path("approvals", &format!("task-{}", slugify(&note.title)));
        let path = write::create(self.vault, &rel, &text, &self.ctx, self.journal, None)?;
        Ok(Wrote::Card(crate::ids::rel(self.vault, &path)))
    }

    /// A `kind: amend` card for the note at `target` (a vault-relative path): one open decision per
    /// field, `pending` or `snoozed`, whoever filed it (§9.5).
    pub(super) fn amend_card(
        &mut self,
        at: &ClaimRef,
        target: &str,
        changes: &[Change],
        proposal: &Proposal,
        also: &[Claimed],
        src: &Source<'_>,
    ) -> Result<Wrote, WriteError> {
        if changes.iter().any(|change| ingest::open_amendment(self.vault, target, &change.field).is_some()) {
            return Ok(Wrote::Nothing(Why::OpenCard));
        }
        let origin = self.origin(at, src)?;
        let senders: Vec<String> = also.iter().map(|c| crate::mailreconcile::sender_text(&c.sender)).collect();
        single_lines(&senders.iter().map(|s| ("sender", s.as_str())).collect::<Vec<_>>())?;
        let stem = std::path::Path::new(target).file_stem().map(|s| s.to_string_lossy().to_string()).unwrap_or_default();
        let note_title = ids_title(self.vault, target).unwrap_or_else(|| stem.clone());
        single_lines(&[("title", &note_title)])?;
        let text = amend_card_text(proposal, &card_title(&note_title, changes), target, changes, also, &origin);
        let refused = match split_frontmatter(&text) {
            Ok((meta, _)) => crate::approvals::validate_amendment(self.vault, &meta).is_err(),
            Err(_) => true,
        };
        if refused {
            return Ok(Wrote::Nothing(Why::NotAmendable));
        }
        let fields: Vec<&str> = changes.iter().map(|change| change.field.as_str()).collect();
        let rel = self.free_path("approvals", &format!("amend-{stem}-{}", fields.join("-")));
        let path = write::create(self.vault, &rel, &text, &self.ctx, self.journal, None)?;
        Ok(Wrote::Card(crate::ids::rel(self.vault, &path)))
    }
}

/// The note's own `title:`, the words a card is named by.
fn ids_title(vault: &std::path::Path, rel: &str) -> Option<String> {
    let meta = crate::ids::read_meta(&vault.join(rel))?;
    yaml::get(&meta, "title").and_then(yaml::text)
}

/// "Email changed due for Lab 5", "Email gave a due date for Lab 5" (a null `from`), or "Email says
/// Lab 5 was cancelled" (`status` to `archived`).
fn card_title(note_title: &str, changes: &[Change]) -> String {
    match changes {
        [only] if only.field == "status" && only.to == "archived" => format!("Email says {note_title} was cancelled"),
        [only] if only.field == "due" && only.from.is_null() => format!("Email gave a due date for {note_title}"),
        _ => {
            let fields: Vec<&str> = changes.iter().map(|change| change.field.as_str()).collect();
            format!("Email changed {} for {note_title}", fields.join(", "))
        }
    }
}
