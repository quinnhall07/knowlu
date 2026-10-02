//! The mail reconciler (email spec 2026-09-30, §6.1; plan T5).
//!
//! **Pure.** Input: a view of the vault's notes ([`VaultView`]), the forwarded messages with their
//! claims ([`Message`]), `today` and the vault day. Output: an ordered list of [`Action`]s. No file,
//! clock or network access: `today` and the vault day are parameters, so the same vault, the same
//! claims and the same day always plan the same actions in the same order (spec test 18).
//!
//! **The one place card or direct write is decided (D11).** Nothing else makes that choice. The
//! view's `human_set` is a snapshot; `mail.rs`'s direct-set writer re-checks it from a fresh journal
//! read just before it writes and files the amend card instead if a human set the field in between,
//! and that re-check never turns a card into a direct write (plan §10, R10).
//!
//! **The order (§4.8, D10).** [`plan`] takes every claim by its message's ordering date, then the
//! message's uid, then the claim's index in the message, whatever order the pulls delivered them in.
//!
//! T5.1 holds the types, the order and the noise screen; T5.2's [`resolve`] holds §4.8 steps 2 and
//! 4, what a claim's target is, read against the vault the earlier claims' actions leave (D10).
//! `rules.rs` holds §4.9's table: T5.3's `new_item` rows, authenticity and the first day; T5.4's
//! `change` rows for mail, the student, other notes, `cancelled` and two values at one ordering
//! date, for a vault note and a note the same plan files alike ([`Place::Planned`], plan §9.5's
//! planned-note amendment); and T5.5's feed and template rows, supersession by `mail_asof`, F3
//! (b)'s cards, receipts (PQ4) and notices. A claim still ambiguous at plan time with a course waits
//! on `mail-resolve` and writes nothing.

use std::collections::{BTreeMap, BTreeSet};

use jiff::civil::Date;
use jiff::Timestamp;
use serde::Deserialize;
use serde_json::Value;

use crate::write;

pub mod resolve;
mod rules;

use resolve::{Resolved, Working};

// ---------------------------------------------------------------------------
// The input: the vault view and the messages
// ---------------------------------------------------------------------------

/// The notes a claim may resolve against or touch, built by `mail.rs` from the vault and its
/// journal. A snapshot: [`plan`] reads it and never writes.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct VaultView {
    pub notes: Vec<NoteView>,
}

/// One note as the reconciler sees it.
#[derive(Debug, Clone, PartialEq)]
pub struct NoteView {
    /// The note's opaque `id:`.
    pub id: String,
    /// Vault-relative and `/`-separated (`tasks/essay-2.md`): the target `write` and
    /// `write::find_pending_amendment` take.
    pub path: String,
    /// `tasks`, `archive`, `approvals`, …
    pub folder: String,
    pub course: Option<String>,
    pub title: String,
    /// `due:` as the frontmatter holds it, `Value::Null` when absent or null. Compared in the
    /// vault's literal form (`write::to_literal`), so a date and a date-time never compare equal.
    pub due: Value,
    pub status: Option<String>,
    /// `archived_reason:`, `imported-past` (`ingest::IMPORTED_PAST`) for an item archived because
    /// its due had passed when it arrived. Approving a mail card that gives such a note a due on or
    /// after the approval day un-archives it (F3 (b), plan T8c), so the rules read it.
    pub archived_reason: Option<String>,
    pub source_uid: Option<String>,
    /// Whose note it is: a feed (`blackboard`, …), `mail`, `gmail`, `events`, or `None` by hand.
    pub created_by: Option<String>,
    /// `mail_asof:`: field → the ordering date of the last mail value applied to it (§5.2).
    pub mail_asof: BTreeMap<String, Timestamp>,
    /// The fields `Journal::human_set` finds a record for: a human `set`, or a human `create`
    /// carrying the field, so a note the student made by hand is theirs (D11 (1)).
    pub human_set: BTreeSet<String>,
}

/// One forwarded message as `mail-pull` delivers it (§5.1's `mail_claims` row).
#[derive(Debug, Clone, PartialEq)]
pub struct Message {
    /// `mail:<h>` (D20).
    pub uid: String,
    /// The earlier of the Date header and the received time (§4.8). The only date that orders, so
    /// a message dated in the future cannot outrank later mail.
    pub ordering_date: Timestamp,
    /// The Date header, which cards and body lines show (D21); `None` when the message had none.
    pub message_date: Option<Timestamp>,
    pub phase: Phase,
    pub authenticity: Authenticity,
    pub sender: Sender,
    pub claims: Vec<Claim>,
}

/// Who a message came from (D21). Untrusted mail text, like every claim string.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Sender {
    pub name: Option<String>,
    pub address: String,
}

/// The phase a message arrived in (§4.9). The wire word `recover` reads as [`Phase::Live`], so no
/// rule can tell the two apart: recover brings in live mail forwarding missed, and a recover batch
/// that finishes an interrupted backfill is opened as `backfill` (plan T5's rules).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Phase {
    Backfill,
    #[serde(alias = "recover")]
    Live,
}

/// §4.6 step 6's word. Only an `aligned` backfill original may write directly (§4.9).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Authenticity {
    Aligned,
    Unaligned,
}

/// One claim in §5.1's shape and wire names. `mail.rs` deserializes each element of a row's
/// `claims` array into it, which is the device's re-check of the shape.
#[derive(Debug, Clone, PartialEq, Deserialize)]
pub struct Claim {
    pub kind: ClaimKind,
    pub course: Option<String>,
    /// A `change`'s item, as the mail names it.
    pub target: Option<String>,
    pub field: Option<ClaimField>,
    /// A `change`'s resolved due (wire shape), or `"true"` for `cancelled`.
    pub value: Option<String>,
    /// A `new_item`'s item.
    pub item: Option<NewItem>,
    /// A `course_info`'s topic.
    pub topic: Option<Topic>,
    /// Verbatim from the message (D9), at most 140 characters.
    pub evidence: String,
    /// One line, at most 140 characters.
    pub why: String,
    pub confidence: f64,
    pub source: ClaimSource,
    /// `mail-resolve`'s stored answer (D12): `Some` once asked, and `target_id: None` is "none".
    pub resolution: Option<Resolution>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ClaimKind {
    NewItem,
    Change,
    CourseInfo,
    Noise,
    /// A submission receipt (PQ4, Quinn 2026-10-02; §5.1's dated marker): `target` names the
    /// item, and `field`, `value`, `item` and `topic` are null. Only a template emits one
    /// (`source: template`); the device re-checks both ([`is_noise`]).
    Receipt,
}

/// A `change`'s field. `cancelled` is always a card proposing `status: active → archived` (§4.9).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ClaimField {
    Due,
    Cancelled,
}

/// A `new_item`'s item. Its effort and importance are written as given: no device re-judgment (§5.2).
#[derive(Debug, Clone, PartialEq, Deserialize)]
pub struct NewItem {
    pub title: String,
    pub due: Option<String>,
    pub item_kind: ItemKind,
    pub effort_hours: f64,
    pub importance: i64,
}

impl NewItem {
    /// The due the reconciler acts on: `due` when it names a real date and time in §5.1's wire shape.
    /// A shaped due naming none (`2026-09-31`, `T24:00`; the server's shape check passes it) is no
    /// date known, "needs a date" (§4.9 row 1), as the server's resolver reads a due it cannot place
    /// on one calendar day (T3b). A due off the shape is noise ([`is_noise`]) and never reaches here.
    pub fn known_due(&self) -> Option<&str> {
        self.due.as_deref().filter(|due| rules::due_date(due).is_some())
    }
}

/// `exam` follows the task-shaped rows; an `event` is the student's call (D24).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ItemKind {
    Task,
    Exam,
    Event,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Topic {
    Schedule,
    Room,
    Instructor,
    OfficeHours,
    Policy,
    Other,
}

impl Topic {
    /// The wire word (§5.1), which a notice's `close_key` carries (D13).
    pub fn word(self) -> &'static str {
        match self {
            Topic::Schedule => "schedule",
            Topic::Room => "room",
            Topic::Instructor => "instructor",
            Topic::OfficeHours => "office_hours",
            Topic::Policy => "policy",
            Topic::Other => "other",
        }
    }

    /// The words a notice's title shows (D13: "titled by course and topic").
    pub fn label(self) -> &'static str {
        match self {
            Topic::OfficeHours => "office hours",
            Topic::Other => "course news",
            topic => topic.word(),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ClaimSource {
    Template,
    Model,
}

#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
pub struct Resolution {
    pub target_id: Option<String>,
}

// ---------------------------------------------------------------------------
// The output: actions
// ---------------------------------------------------------------------------

/// Which claim an action came from: its message's uid and its 0-based index in `claims`.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct ClaimRef {
    pub message: String,
    pub index: usize,
}

/// The note an action writes to or files a card against (plan §9.5's planned-note amendment).
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum Place {
    /// A note in the vault, by its [`NoteView::path`] (`tasks/lab-5.md`).
    Path(String),
    /// The note the [`Action::Create`] of this claim files, earlier in the same plan: a batch
    /// applied whole reaches the item it creates (D10, spec test 18). `mail.rs` executes actions in
    /// plan order, so that note is written first, and an action naming it writes at the path its
    /// `Create` gave. Only a `Create` files an item, so no other claim is ever named here.
    Planned(ClaimRef),
}

/// What the mail pass writes for one claim, in plan order (§4.9). `mail.rs` executes each through
/// `write`, journal first, as `agent:knowlu.mail`, and decides nothing.
#[derive(Debug, Clone, PartialEq)]
pub enum Action {
    /// A new note in `tasks/` (§5.2), written directly.
    Create { claim: ClaimRef, note: NewNote },
    /// One field set directly on `target`, with its `mail_asof` stamp and the body line
    /// `<date> · <sender>: <why> — "<evidence>"` (§5.2).
    Set {
        claim: ClaimRef,
        target: Place,
        field: String,
        to: Value,
        mail_asof: Timestamp,
        body_line: String,
    },
    /// A `kind: task` card proposing a new note.
    TaskCard { claim: ClaimRef, note: NewNote, proposal: Proposal },
    /// A `kind: amend` card against `target` (D11 (3)). `also` holds the other claims a two-value
    /// card names (§4.9's row for two claims at one ordering date, P20), in plan order, each with
    /// the value it claims and its sender, and never the card's own claim, whose value is `to`: the
    /// last in plan order whose value the note did not already hold (plan §9.5). Empty on every
    /// other card. A field T5.1's action list did not have, recorded in plan §9.5.
    AmendCard { claim: ClaimRef, target: Place, changes: Vec<Change>, proposal: Proposal, also: Vec<Claimed> },
    /// An `info/` item: `kind` is one of `info::KINDS`, and a later notice with the same
    /// `close_key` supersedes it (D13). `about` says which count the run line puts it in.
    Notice {
        claim: ClaimRef,
        kind: &'static str,
        close_key: Option<String>,
        title: String,
        body_line: String,
        about: NoticeAbout,
    },
    /// A submission receipt: `completion::propose_done` on `target`.
    ProposeDone { claim: ClaimRef, target: Place },
    /// Nothing is written; the reason is counted on the run line (§4.8 step 7).
    Skip { claim: ClaimRef, reason: SkipReason },
}

impl Action {
    pub fn claim(&self) -> &ClaimRef {
        match self {
            Action::Create { claim, .. }
            | Action::Set { claim, .. }
            | Action::TaskCard { claim, .. }
            | Action::AmendCard { claim, .. }
            | Action::Notice { claim, .. }
            | Action::ProposeDone { claim, .. }
            | Action::Skip { claim, .. } => claim,
        }
    }
}

/// A note a `new_item` creates or proposes (§5.2).
#[derive(Debug, Clone, PartialEq)]
pub struct NewNote {
    pub title: String,
    pub course: Option<String>,
    /// `None`: "needs a date".
    pub due: Option<String>,
    pub item_kind: ItemKind,
    pub effort_hours: f64,
    pub importance: i64,
    /// Created `status: archived`, `archived_reason: imported-past` (§4.9, D24).
    pub imported_past: bool,
    /// `mail_asof: {due: …}`, when `due` came from mail.
    pub mail_asof: Option<Timestamp>,
}

/// One `changes:` entry of an amend card: `{field: {from, to}}`.
#[derive(Debug, Clone, PartialEq)]
pub struct Change {
    pub field: String,
    pub from: Value,
    pub to: Value,
}

/// What one claim says a field should be, named on a two-value card's body with its sender (P20).
#[derive(Debug, Clone, PartialEq)]
pub struct Claimed {
    pub claim: ClaimRef,
    pub value: Value,
    pub sender: Sender,
}

/// A card's place in the deck (§4.8 step 5, plan §10 R16). `approvals::transition_note` wakes only
/// `snoozed`, so a card held to day 2 is written `snoozed`, never `pending`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Proposal {
    pub status: CardStatus,
    /// The day it charges the 15-a-day budget.
    pub proposed_at: Date,
    /// Set once, the day it was filed; drives every age.
    pub first_proposed_at: Date,
    pub snooze_until: Option<Date>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CardStatus {
    Pending,
    Snoozed,
}

/// What a notice tells, so §4.8 step 7's line counts it once: `J notices` or `U unmatched`. A field
/// T5.1's action list did not have, recorded in plan §9.5.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NoticeAbout {
    /// A `course_info` claim (D13).
    CourseInfo,
    /// A claim no item matched after resolve (§4.9's unmatched row, D12).
    Unmatched,
}

/// Why a claim writes nothing.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SkipReason {
    /// Ordered before the field's `mail_asof` (§4.8 step 5; a stamp at the claim's own ordering date
    /// is its own run's, plan §9.5), a feed holds the newer value (§4.9 row 8), or another claim of
    /// a two-value run makes the card (P20).
    Superseded,
    /// A live event already past (D24).
    Past,
    /// Its uid is already on a note or card, another desktop's write (§4.8 step 4).
    AlreadyApplied,
    /// The note already holds the value.
    Equal,
    /// A `noise` claim, one whose mail text would break a line (plan §10, R11), or a `new_item` whose
    /// due is off §5.1's wire shape.
    Noise,
    /// No note matched (§4.8 step 2).
    Unmatched,
    /// A card row whose card `approvals::validate_amendment` would refuse even with T8c's two arms
    /// (F3 (b), Quinn 2026-10-02): a note the student archived (in `archive/` without `archived_reason:
    /// imported-past`), a `cancelled` for a note with no `status:` line, a collection `from`, or a due
    /// still before today for an `imported-past` note. The feed's rule for the same refusal (plan §10,
    /// R14): the note keeps its value and no card is filed. A seventh reason beyond T5.1's six; T6.3
    /// counts it as PQ7 rules.
    NotAmendable,
}

// ---------------------------------------------------------------------------
// The plan
// ---------------------------------------------------------------------------

/// Plan every claim of `messages` against `view` (§4.8 steps 2, 4 and 5; §4.9).
pub fn plan(view: &VaultView, messages: &[Message], today: Date, vault_day: i64) -> Vec<Action> {
    planned_steps(view, messages, today, vault_day).into_iter().flat_map(|step| step.actions).collect()
}

/// One claim of a walk: which it is, whether the plan left it ambiguous, what it says a vault note's
/// field should be (applied or not, for §4.9's two-value row), and what it plans.
struct Step<'m> {
    at: ClaimRef,
    message: &'m Message,
    claim: &'m Claim,
    ambiguous: bool,
    said: Option<rules::Said>,
    actions: Vec<Action>,
}

/// The walk [`plan`] and [`resolve::to_ask`] share, with the rules: one function, so what is asked
/// is exactly what the plan finds ambiguous.
fn planned_steps<'m>(view: &VaultView, messages: &'m [Message], today: Date, vault_day: i64) -> Vec<Step<'m>> {
    walk(
        view,
        messages,
        |resolved, message, at, claim| rules::rule(resolved, message, at, claim, today, vault_day),
        |before, run| rules::settle(before, run, today, vault_day),
    )
}

/// Every claim in §4.8's order, each resolved against the vault the earlier claims' actions leave
/// ([`resolve::Working`], D10), so a batch plans as its claims would one pull at a time. A noise
/// claim is skipped before it is resolved. `rule` turns a resolution into actions.
///
/// Claims sharing an ordering date are one *run*: once a run of two or more is planned, `settle`
/// may rewrite its steps' actions against the vault as it stood before the run (§4.9's two-value
/// row), and the working vault is then rebuilt from that point with the run's final actions. A
/// rewrite only exchanges members' sets and cards for skips and one card under one member's uid; it
/// adds and removes no item, resolution reads no `due`, and it reads a card only for its own
/// claim's uid, so a claim delivered once resolves the same for it. A claim delivered twice does
/// not: its later copy resolved against the first copy's actions, which a rewrite may change, so
/// `settle` plans each claim once, by its first copy, and skips each later copy as already applied
/// (§4.8 step 4), as a later copy delivered in another pull reads the first's card.
fn walk<'m>(
    view: &VaultView,
    messages: &'m [Message],
    mut rule: impl FnMut(&Resolved<'_>, &Message, ClaimRef, &Claim) -> Vec<Action>,
    mut settle: impl FnMut(&Working, &mut [Step<'m>]) -> bool,
) -> Vec<Step<'m>> {
    let mut working = Working::new(view);
    let mut steps: Vec<Step<'m>> = Vec::new();
    for run in order(messages).chunk_by(|(a, _), (b, _)| a.ordering_date == b.ordering_date) {
        let (before, start) = ((run.len() > 1).then(|| working.clone()), steps.len());
        for &(message, index) in run {
            let (claim, at) = (&message.claims[index], ClaimRef { message: message.uid.clone(), index });
            let (ambiguous, said, actions) = if is_noise(message, claim) {
                (false, None, vec![Action::Skip { claim: at.clone(), reason: SkipReason::Noise }])
            } else {
                let resolved = working.resolve(&at, claim);
                // An applied claim says what it said when first planned. One whose target is the
                // note it created itself (its uid on it) was `New` then, and said nothing.
                let said = match resolved {
                    Resolved::AlreadyApplied => match working.resolve_target(claim) {
                        Resolved::Change { target, .. } if target.note().source_uid == Some(at.uid()) => None,
                        first => rules::said(&first),
                    },
                    ref resolved => rules::said(resolved),
                };
                (resolved == Resolved::Ambiguous, said, rule(&resolved, message, at.clone(), claim))
            };
            actions.iter().for_each(|action| working.apply(action));
            steps.push(Step { at, message, claim, ambiguous, said, actions });
        }
        if let Some(before) = before {
            if settle(&before, &mut steps[start..]) {
                working = before;
                steps[start..].iter().flat_map(|step| &step.actions).for_each(|action| working.apply(action));
            }
        }
    }
    steps
}

/// §4.8's order: `(message, claim index)` by ordering date, then uid, then claim index, whatever
/// order the pulls delivered them in. The key is total: a uid names one message (D20), so two
/// entries tie only when one message was delivered twice, and then they are the same claim.
fn order(messages: &[Message]) -> Vec<(&Message, usize)> {
    let mut claims: Vec<(&Message, usize)> =
        messages.iter().flat_map(|m| (0..m.claims.len()).map(move |i| (m, i))).collect();
    claims.sort_by(|(a, i), (b, j)| (a.ordering_date, &a.uid, i).cmp(&(b.ordering_date, &b.uid, j)));
    claims
}

/// §4.9's `noise` row, and plan §10 R11: a claim whose title, sender, why or evidence
/// `write::single_line_problem` refuses is noise whatever its kind, so no mail text that breaks a
/// line (any `pystr::splitlines` boundary) ever reaches a writer. So is a `new_item` whose due is
/// off §5.1's wire shape ([`rules::wire_shaped`]; a line break or another digit set included): the
/// server's shape check refuses it, so the claim never passed validation, and such a due can
/// neither be ordered against today nor written. A shaped due naming no real day passes the server
/// and is no due here ([`NewItem::known_due`]), never noise: the item needs a date and is counted.
/// A `change`'s value is held to its field's shape the same way (§5.1): a `due` value off the wire
/// shape, or a `cancelled` value other than `"true"`, is noise; a null due names no date, and the
/// rules write nothing for it. T5.5 holds two more kinds to §5.1's shape: a `course_info` needs a
/// topic, which keys its notice (D13), and a receipt comes only from a template and carries no
/// `field`, `value`, `item` or `topic` (PQ4). A course is written into a notice's title and key, so
/// one that would break a line is noise too.
fn is_noise(message: &Message, claim: &Claim) -> bool {
    let title = claim.item.as_ref().map(|item| item.title.as_str());
    let sender = [message.sender.name.as_deref(), Some(message.sender.address.as_str())];
    let texts = [title, claim.course.as_deref(), Some(claim.why.as_str()), Some(claim.evidence.as_str())];
    let due = claim.item.as_ref().and_then(|item| item.due.as_deref());
    let off_shape = match (claim.kind, claim.field) {
        (ClaimKind::Change, Some(ClaimField::Due)) => claim.value.as_deref().is_some_and(|due| !rules::wire_shaped(due)),
        (ClaimKind::Change, Some(ClaimField::Cancelled)) => claim.value.as_deref() != Some("true"),
        (ClaimKind::CourseInfo, _) => claim.topic.is_none(),
        (ClaimKind::Receipt, field) => {
            claim.source != ClaimSource::Template || field.is_some() || claim.value.is_some() || claim.item.is_some() || claim.topic.is_some()
        }
        _ => false,
    };
    claim.kind == ClaimKind::Noise
        || sender.into_iter().chain(texts).flatten().any(|text| write::single_line_problem(text).is_some())
        || due.is_some_and(|due| !rules::wire_shaped(due))
        || off_shape
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::pystr;
    use serde_json::json;

    fn ts(text: &str) -> Timestamp {
        text.parse().unwrap()
    }

    fn today() -> Date {
        jiff::civil::date(2026, 10, 1)
    }

    fn claim(kind: ClaimKind) -> Claim {
        Claim {
            kind,
            course: Some("CS 101".to_string()),
            target: None,
            field: None,
            value: None,
            item: None,
            topic: None,
            evidence: "Essay 2 is now due Monday".to_string(),
            why: "The instructor moved the due date".to_string(),
            confidence: 0.9,
            source: ClaimSource::Model,
            resolution: None,
        }
    }

    fn new_item(title: &str, due: Option<&str>) -> Claim {
        let item = NewItem {
            title: title.to_string(),
            due: due.map(str::to_string),
            item_kind: ItemKind::Task,
            effort_hours: 2.0,
            importance: 3,
        };
        Claim { item: Some(item), ..claim(ClaimKind::NewItem) }
    }

    fn change(target: &str, value: &str) -> Claim {
        let (target, value) = (Some(target.to_string()), Some(value.to_string()));
        Claim { target, field: Some(ClaimField::Due), value, ..claim(ClaimKind::Change) }
    }

    fn message(uid: &str, ordering: &str, phase: Phase, claims: Vec<Claim>) -> Message {
        Message {
            uid: uid.to_string(),
            ordering_date: ts(ordering),
            message_date: Some(ts(ordering)),
            phase,
            authenticity: Authenticity::Aligned,
            sender: Sender { name: Some("Course Staff".to_string()), address: "staff@example.edu".to_string() },
            claims,
        }
    }

    fn note(id: &str, title: &str, due: &str, created_by: Option<&str>, human_set: &[&str]) -> NoteView {
        NoteView {
            id: id.to_string(),
            path: format!("tasks/{id}.md"),
            folder: "tasks".to_string(),
            course: Some("CS 101".to_string()),
            title: title.to_string(),
            due: json!(due),
            status: Some("active".to_string()),
            archived_reason: None,
            source_uid: created_by.map(|by| format!("{by}:{id}")),
            created_by: created_by.map(str::to_string),
            mail_asof: BTreeMap::new(),
            human_set: human_set.iter().map(|f| f.to_string()).collect(),
        }
    }

    fn view() -> VaultView {
        VaultView {
            notes: vec![
                note("task-1a2b", "Essay 2", "2026-10-03", Some("gmail"), &[]),
                note("task-3c4d", "Lab 4", "2026-10-08T23:59", Some("blackboard"), &["due"]),
                note("task-5e6f", "Reading 1", "2026-10-02", None, &["title", "due", "status"]),
            ],
        }
    }

    /// Delivered out of order: two messages share an ordering date, one carries two claims, and one
    /// is dated far in the future but ordered at its received time.
    fn messages() -> Vec<Message> {
        let room = Claim { topic: Some(Topic::Room), ..claim(ClaimKind::CourseInfo) };
        let cancelled = Claim { field: Some(ClaimField::Cancelled), ..change("Reading 1", "true") };
        let mut future = message("mail:0f0f", "2026-09-25T08:00:00Z", Phase::Live, vec![claim(ClaimKind::Noise)]);
        future.message_date = Some(ts("2099-01-01T00:00:00Z"));
        vec![
            message("mail:b2b2", "2026-09-22T12:00:00Z", Phase::Live, vec![change("Lab 4", "2026-10-09T23:59")]),
            future,
            message("mail:c3c3", "2026-09-20T09:00:00Z", Phase::Backfill, vec![new_item("Essay 3", Some("2026-10-12")), room]),
            message("mail:a1a1", "2026-09-22T12:00:00Z", Phase::Backfill, vec![change("Essay 2", "2026-10-05"), cancelled]),
        ]
    }

    #[test]
    fn claims_are_planned_by_ordering_date_then_uid_then_claim_index() {
        let delivered = messages();
        let got: Vec<(&str, usize)> = order(&delivered).into_iter().map(|(m, i)| (m.uid.as_str(), i)).collect();
        let want = [("mail:c3c3", 0), ("mail:c3c3", 1), ("mail:a1a1", 0), ("mail:a1a1", 1), ("mail:b2b2", 0), ("mail:0f0f", 0)];
        assert_eq!(got, want);
    }

    /// Spec test 18, first half.
    #[test]
    fn the_same_vault_and_claims_plan_the_same_actions_twice() {
        let (view, messages) = (view(), messages());
        let first = plan(&view, &messages, today(), 9);
        let second = plan(&view, &messages, today(), 9);
        assert!(!first.is_empty(), "every claim plans an action, a skip included");
        assert_eq!(first, second);
    }

    /// Spec test 18, second half.
    #[test]
    fn a_shuffled_delivery_plans_the_same_actions() {
        let view = view();
        let delivered = messages();
        let expected = plan(&view, &delivered, today(), 9);
        let mut reversed = delivered.clone();
        reversed.reverse();
        for base in [delivered, reversed] {
            for turn in 0..base.len() {
                let mut shuffled = base.clone();
                shuffled.rotate_left(turn);
                assert_eq!(plan(&view, &shuffled, today(), 9), expected, "rotated by {turn}");
            }
        }
    }

    /// Plan §10, R11: a break in mail text could forge a line wherever it is written (the
    /// seen ledger, a body line). Every break `pystr::splitlines` makes, in every mail-derived
    /// field, is noise and nothing else.
    #[test]
    fn a_claim_with_a_line_break_in_any_field_is_skipped_as_noise() {
        let mut breaks: Vec<String> = (0..=0x10FFFF_u32)
            .filter_map(char::from_u32)
            .filter(|c| pystr::is_line_boundary(*c))
            .map(String::from)
            .collect();
        breaks.push("\r\n".to_string());
        for named in ["\r", "\n", "\u{85}", "\u{2028}", "\u{2029}"] {
            assert!(breaks.iter().any(|b| b == named), "{named:?}");
        }
        let clean = message("mail:d4d4", "2026-09-23T10:00:00Z", Phase::Backfill, vec![new_item("Essay 3", Some("2026-10-12"))]);
        let at = ClaimRef { message: "mail:d4d4".to_string(), index: 0 };
        let noise = Action::Skip { claim: at, reason: SkipReason::Noise };
        assert!(!plan(&view(), &[clean.clone()], today(), 9).contains(&noise), "the clean claim is not noise");
        let fields: [(&str, fn(&mut Message, &str)); 5] = [
            ("title", |m, b| m.claims[0].item.as_mut().unwrap().title = format!("Essay{b}- bb-lab-4 · x")),
            ("sender name", |m, b| m.sender.name = Some(format!("Course{b}Staff"))),
            ("sender address", |m, b| m.sender.address = format!("staff@example.edu{b}")),
            ("why", |m, b| m.claims[0].why = format!("Moved{b}again")),
            ("evidence", |m, b| m.claims[0].evidence = format!("due{b}Monday")),
        ];
        for (field, inject) in fields {
            for b in &breaks {
                let mut hostile = clean.clone();
                inject(&mut hostile, b);
                assert_eq!(plan(&view(), &[hostile], today(), 9), vec![noise.clone()], "{field} carrying {b:?}");
            }
        }
    }

    /// §5.1's names are the wire's; `recover` reads as live; an unknown kind is refused. PQ4 made
    /// `receipt` a kind (Quinn 2026-10-02), so it deserializes, and the unknown-kind probe is
    /// `completed`, the field value PQ4 declined (plan §9.5's sanctioned change).
    #[test]
    fn a_claim_in_the_wire_shape_deserializes_and_recover_reads_as_live() {
        let wire = json!({
            "kind": "change", "course": "CS 101", "target": "Essay 2", "field": "due",
            "value": "2026-10-05", "item": null, "topic": null,
            "evidence": "Essay 2 is now due Monday", "why": "The instructor moved the due date",
            "confidence": 0.9, "source": "template", "resolution": {"target_id": null}
        });
        let parsed: Claim = serde_json::from_value(wire.clone()).unwrap();
        let resolution = Some(Resolution { target_id: None });
        let want = Claim { source: ClaimSource::Template, resolution, ..change("Essay 2", "2026-10-05") };
        assert_eq!(parsed, want);
        let item = json!({"title": "Midterm", "due": null, "item_kind": "exam", "effort_hours": 3.5, "importance": 5});
        let parsed: Claim = serde_json::from_value(json!({
            "kind": "new_item", "course": null, "item": item, "topic": "office_hours",
            "evidence": "Midterm", "why": "An exam", "confidence": 0.7, "source": "model"
        }))
        .unwrap();
        assert_eq!((parsed.item.unwrap().item_kind, parsed.topic), (ItemKind::Exam, Some(Topic::OfficeHours)));
        let receipt = json!({
            "kind": "receipt", "course": "CS 101", "target": "Lab 5", "field": null, "value": null,
            "item": null, "topic": null, "evidence": "Submission received", "why": "The platform confirmed it",
            "confidence": 1.0, "source": "template", "resolution": null
        });
        let parsed: Claim = serde_json::from_value(receipt).unwrap();
        assert_eq!((parsed.kind, parsed.target.as_deref(), parsed.field), (ClaimKind::Receipt, Some("Lab 5"), None));
        let mut unknown = wire;
        unknown["kind"] = json!("completed");
        assert!(serde_json::from_value::<Claim>(unknown).is_err());
        let phase = |w: &str| serde_json::from_value::<Phase>(json!(w)).unwrap();
        assert_eq!([phase("backfill"), phase("live"), phase("recover")], [Phase::Backfill, Phase::Live, Phase::Live]);
        assert_eq!(serde_json::from_value::<Authenticity>(json!("unaligned")).unwrap(), Authenticity::Unaligned);
    }
}
