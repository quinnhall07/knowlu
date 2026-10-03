//! The bytes the mail pass writes: a note, a task card, an amend card (spec §5.2, D11 (3), D21).
//!
//! Pure text. Every mail-derived string goes through [`lit`], which is `write::to_literal`, so a
//! colon, a quote or markup in a title, sender, `why` or evidence lands as one literal line (plan
//! §10, R11). Each template is one `format!`, never a `replace` chain: a title that holds `{why}`
//! must not be substituted by a later pass over the text.

use jiff::civil::Date;
use serde_json::Value as Json;
use serde_yaml_ng::Value as Yaml;

use crate::ingest::IMPORTED_PAST;
use crate::mailreconcile::{CardStatus, Change, Claimed, NewNote, Proposal};
use crate::write::{self, to_literal, WriteError};

/// A string as the vault writes it on one line.
pub(super) fn lit(text: &str) -> String {
    to_literal(&Yaml::String(text.to_string()))
}

/// A claim's JSON value (a due, a status word, null) as the vault writes it.
pub(super) fn json_lit(value: &Json) -> String {
    to_literal(&crate::yaml::from_json(value))
}

/// A JSON scalar as plain text, for a card's prose.
fn plain(value: &Json) -> String {
    match value {
        Json::String(text) => text.clone(),
        Json::Null => "no date".to_string(),
        other => other.to_string(),
    }
}

/// Refuse mail text that would break a line, naming the field. T5 skips such a claim as noise, so
/// this is the writer's own belt: no byte is written for it.
pub(super) fn single_lines(fields: &[(&str, &str)]) -> Result<(), WriteError> {
    match fields.iter().find(|(_, text)| write::single_line_problem(text).is_some()) {
        Some((name, _)) => Err(WriteError::LineBreak(name.to_string())),
        None => Ok(()),
    }
}

/// What a note or card shows of the claim it came from.
pub(super) struct Origin<'a> {
    /// `mail:<h>:<n>`.
    pub uid: String,
    pub judgment_id: Option<&'a str>,
    /// D21: `Name <address>`, or the address alone.
    pub sender: String,
    /// The message's day, as `YYYY-MM-DD`.
    pub day: String,
    pub why: &'a str,
    pub evidence: &'a str,
    /// §5.2's `<date> · <sender>: <why> — "<evidence>"`.
    pub body_line: String,
}

impl Origin<'_> {
    /// `judgment_id` and `judgment_kind: mail_claim`, both keys or neither.
    fn judgment_lines(&self) -> String {
        match self.judgment_id {
            Some(id) => format!("judgment_id: {}\njudgment_kind: mail_claim\n", lit(id)),
            None => String::new(),
        }
    }
}

/// §5.2's note: `GMAIL_NOTE`'s shape with `created_by: mail`, `source_uid: mail:<h>:<n>`, the judgment
/// keys, the claim's own `effort_hours` and `importance`, `needs_enrichment: false`, and
/// `mail_asof: {due: …}` when the due came from mail. A past-due item is `status: archived` with
/// `archived_reason: imported-past`.
pub(super) fn note_text(note: &NewNote, origin: &Origin<'_>) -> String {
    let effort = to_literal(&Yaml::Number(serde_yaml_ng::Number::from(note.effort_hours)));
    let status = match note.imported_past {
        true => format!("status: archived\narchived_reason: {IMPORTED_PAST}\n"),
        false => "status: active\n".to_string(),
    };
    let asof = match note.mail_asof {
        Some(at) => format!("mail_asof: {}\n", asof_literal(&[("due".to_string(), at.to_string())])),
        None => String::new(),
    };
    format!(
        "---\ntitle: {}\ncourse: {}\ndomain: school\ndue: {}\neffort_hours: {effort}\n\
         effort_confidence: low\neffort_source: inferred\nimportance: {}\nimportance_reason: {}\n\
         {status}progress: 0\ncreated_by: mail\nsource_uid: {}\n{}{asof}needs_enrichment: false\n---\n\n\
         From email. {}\n",
        lit(&note.title),
        note.course.as_deref().map_or("null".to_string(), lit),
        note.due.as_deref().map_or("null".to_string(), lit),
        note.importance,
        lit(origin.why),
        lit(&origin.uid),
        origin.judgment_lines(),
        origin.body_line,
    )
}

/// `mail_asof`'s one-line flow mapping (§5.2, P17): field to ordering date, in the order given,
/// through `write::to_literal` and nothing else.
pub(super) fn asof_literal(entries: &[(String, String)]) -> String {
    let mut map = serde_yaml_ng::Mapping::new();
    for (field, at) in entries {
        map.insert(Yaml::String(field.clone()), Yaml::String(at.clone()));
    }
    to_literal(&Yaml::Mapping(map))
}

fn status_line(proposal: &Proposal) -> &'static str {
    match proposal.status {
        CardStatus::Pending => "status: pending\n",
        CardStatus::Snoozed => "status: snoozed\n",
    }
}

/// The two days a card is charged and aged by, and its hold (§4.8 step 5, plan §10 R3 and R16):
/// `proposed_at` charges the 15-a-day budget, `first_proposed_at` is set once.
fn day_lines(proposal: &Proposal, expires: &str) -> String {
    format!(
        "proposed_at: {}\nfirst_proposed_at: {}\nexpires: {expires}\nsnooze_until: {}\n",
        proposal.proposed_at,
        proposal.first_proposed_at,
        proposal.snooze_until.map_or("null".to_string(), |day: Date| day.to_string()),
    )
}

/// The three additive fields every mail card carries (§5.2, D21): who, when, and the quote.
fn source_lines(origin: &Origin<'_>) -> String {
    format!("from: {}\nmail_date: {}\nevidence: {}\n", lit(&origin.sender), lit(&origin.day), lit(origin.evidence))
}

/// A `kind: task` card in `write_gmail_card`'s shape: the note as a fenced `task` payload, which is
/// exactly what `approvals::materialize` turns into the note on approval. `created_by: mail`.
pub(super) fn task_card_text(proposal: &Proposal, title: &str, note: &str, origin: &Origin<'_>) -> String {
    let expires = proposal.proposed_at.checked_add(jiff::Span::new().days(14)).unwrap_or(proposal.proposed_at);
    format!(
        "---\ntype: approval\nkind: task\ntitle: {}\n{}{}created_by: mail\n{}source_uid: {}\n{}---\n\n{}\n\n```task\n{note}```\n",
        lit(title),
        status_line(proposal),
        day_lines(proposal, &expires.to_string()),
        origin.judgment_lines(),
        lit(&origin.uid),
        source_lines(origin),
        origin.body_line,
    )
}

/// A `kind: amend` card in the shape `approvals::validate_amendment` accepts (D11 (3)): `target`,
/// `changes: {field: {from, to}}`, `expires: null`, plus `created_by: mail` and D21's fields, and
/// **never** an `append` (plan §10, R15). `also` names the other claims of a two-value card, each
/// with its sender (P20), in the body.
pub(super) fn amend_card_text(
    proposal: &Proposal,
    title: &str,
    target: &str,
    changes: &[Change],
    also: &[Claimed],
    origin: &Origin<'_>,
) -> String {
    let changes: String = changes
        .iter()
        .map(|c| format!("  {}:\n    from: {}\n    to: {}\n", c.field, json_lit(&c.from), json_lit(&c.to)))
        .collect();
    let also: String = also
        .iter()
        .map(|c| format!("\n**Also claimed:** {} (from {})\n", plain(&c.value), crate::mailreconcile::sender_text(&c.sender)))
        .collect();
    format!(
        "---\ntype: approval\nkind: amend\ntitle: {}\n{}target: {}\n{}created_by: mail\n{}source_uid: {}\n{}\
         changes:\n{changes}---\n\n**Why proposed:** {}\n\n**From:** {} on {}\n\n**Evidence:** \"{}\"\n{also}{}",
        lit(title),
        status_line(proposal),
        lit(target),
        day_lines(proposal, "null"),
        origin.judgment_lines(),
        lit(&origin.uid),
        source_lines(origin),
        origin.why,
        origin.sender,
        origin.day,
        origin.evidence,
        write::AMEND_BUTTONS,
    )
}
