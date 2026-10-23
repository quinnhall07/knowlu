//! §4.9's table, row by row: what each resolved claim writes, and whether it writes directly or
//! files a card (email spec 2026-09-30; plan T5.3–T5.5).
//!
//! Pure, like the rest of the reconciler. [`rule`] takes one claim as [`super::resolve`] left it
//! and returns its actions, and nothing else decides card or direct (D11).
//!
//! **Direct writes (§4.9, §4.6 step 6).** Only the backfill writes directly, and only from an
//! `aligned` original ([`writes_direct`]): each direct row of an `unaligned` backfilled original
//! files the card its live row files. Live mail, recovered mail included, never writes directly.
//!
//! **`new_item`, §4.9's first four rows (T5.3).** `task` and `exam` are task-shaped; an `event` is
//! the student's call (D24).
//! - Task-shaped, due ahead or undated: a note written directly (undated: "needs a date"); live, a
//!   `kind: task` card.
//! - Task-shaped, already past due: a note created and archived `imported-past`; live, a
//!   `kind: task` card, which proposes an active task (late work is still the student's to judge).
//! - An event still ahead, or undated: a card in both phases (D24).
//! - An event already past: recorded and archived `imported-past` in the backfill; otherwise
//!   nothing, counted past, since no one can accept a past event (D24). That holds for an
//!   `unaligned` backfilled original too: its direct row may not be written, and D24 files no card
//!   for a past event.
//!
//! A note made from a `new_item` carries the claim's `effort_hours` and `importance` as given
//! (§5.2), and `mail_asof` for `due` at the message's ordering date when the mail gave the due. A
//! due in §5.1's wire shape naming no real day or time is no due ([`super::NewItem::known_due`]):
//! the item needs a date, and is created or carded and counted like any undated item.
//!
//! **`change`, §4.9's rows 5, 6, 10, 11 and 12 (T5.4).** Card or direct is decided from the view's
//! `human_set` snapshot (D11 (1)), never from `write`'s judged set, which does not cover `due`.
//! Only a mail note's `due` that the student never set takes an aligned backfill's value directly,
//! and outside `tasks/` only a due still past; every other change is a `kind: amend` card in both
//! phases, `cancelled` always (`status: active → archived`). A change that changes nothing writes
//! nothing, and a card `approvals::validate_amendment` would refuse is never planned ([`change`]).
//! Two claims at one ordering date saying different values for one note's field make one card
//! naming both, proposing a value the note does not hold ([`settle`], P20 as plan §9.5 reads it).
//! A note this plan files takes every row as the mail note it will be, and its actions name it by
//! its creating claim ([`Place::Planned`]; plan §9.5's planned-note amendment), so a batch planned
//! whole writes what the same batch split across pulls writes (D10, spec test 18).
//!
//! **T5.5: feeds, templates, supersession, F3, receipts and notices.**
//! - Rows 7 to 9: a fed item ([`FEEDS`], P10) takes a model's change as a card unless equal; a
//!   backfill template's change to it writes nothing, counted superseded; a template's change to
//!   any other note takes that note's own row, so mail writes directly only to its own note.
//! - §4.8 step 5: a claim ordered before its note's `mail_asof` for the field is superseded; a stamp
//!   at the claim's own ordering date is its run's and never supersedes it ([`change`]).
//! - F3 (b), Quinn 2026-10-02: the cards T8c's executor accepts are planned where T5.4 skipped, a
//!   date for a note with no due (from `null`) and a due ahead for an `imported-past` note; what it
//!   still refuses stays [`SkipReason::NotAmendable`] ([`amendable`]).
//! - A receipt (PQ4) is `ProposeDone` in both phases. A `course_info` claim and a claim no item
//!   matches are notices (D13, D12), written in both phases and never held.
//! - D24: no action names a note in `commitments/`; a claim's target is only ever an item note.
//!
//! **T5.6: the same mail by two routes (PQ8 (ii), Quinn 2026-10-02).** A live `new_item` an open
//! mail `kind: task` card already proposes ([`Resolved::Carded`]: normalised title and course, the
//! card `pending` or `snoozed`, and no known due or one on the card's due day, PQ13 (b), Quinn
//! 2026-10-03) files nothing, skipped as already applied, before any `new_item` row. Nothing is
//! written, so it needs no Undo and no notice. Another due day takes its own row, a card, so a
//! date is never lost unseen. A backfill keeps T5.3's rows.
//!
//! **Past** is a due whose date is strictly before `today`, never before *now*: the module has no
//! clock, and `ingest` and `coursework` archive `imported-past` by the same rule, since an item due
//! at 23:59 today is today's work.
//!
//! **The first day (§4.8 step 5, plan §10 R16).** On vault day 1 a card is held to day 2: written
//! `status: snoozed` with `proposed_at` and `snooze_until` both day 2, so it charges day 2's budget
//! and shows from day 2, and `first_proposed_at` day 1, the day it was filed
//! ([`proposal`]). `approvals::transition_note` wakes only a `snoozed` card, so a `pending` one would
//! show on day 1. Direct writes and notices are not held.

use std::collections::{BTreeMap, BTreeSet};

use jiff::civil::Date;
use serde_json::Value;

use super::resolve::{Resolved, Target, Working};
use super::{
    Action, Authenticity, CardStatus, Change, Claim, ClaimField, ClaimKind, ClaimRef, ClaimSource, Claimed, ItemKind, Message,
    NewItem, NewNote, NoteView, NoticeAbout, Phase, Place, Proposal, Sender, SkipReason, Step,
};
use crate::approvals::AMENDABLE_FOLDERS;
use crate::write;

/// The `created_by` of a note the mail pass made (D20): the only note mail may write directly.
pub(super) const MAIL: &str = "mail";

/// P10: the creators whose notes an authoritative feed supplies, and so owns (§4.9 rows 7 and 8).
pub(super) const FEEDS: [&str; 3] = ["blackboard", "zybooks", "vhl"];

/// D12's words for a claim no item matched, the title of its notice.
const UNMATCHED_NOTICE: &str = "Knowlu could not match this to an item";

/// The actions one claim plans, from its resolution (§4.9). T5.3 holds the `new_item` rows, T5.4
/// and T5.5 the `change` rows, for a vault note and for a note this plan files alike (plan §9.5's
/// planned-note amendment), and T5.5 the receipt, `course_info` and unmatched rows.
///
/// A claim still ambiguous here has no stored answer. With a course, `mail.rs` asks `mail-resolve`
/// before it plans (§4.8 step 3), so the claim waits and writes nothing; with none it can only be
/// answered "none" ([`super::resolve::to_ask`]), so it is unmatched now.
pub(super) fn rule(
    resolved: &Resolved<'_>,
    message: &Message,
    at: ClaimRef,
    claim: &Claim,
    today: Date,
    vault_day: i64,
) -> Vec<Action> {
    let skip = |claim, reason| vec![Action::Skip { claim, reason }];
    match (resolved, &claim.item) {
        // PQ8 (ii), plan T5.6: live, an item an open mail task card already proposes for its due day
        // (PQ13 (b)) files nothing, counted with the already-applied skips, and the student decides
        // the one card. Before every `new_item` row, so whatever the item's kind; a backfill keeps
        // T5.3's rows.
        (Resolved::Carded, _) if message.phase == Phase::Live => skip(at, SkipReason::AlreadyApplied),
        (Resolved::New | Resolved::Carded, Some(item)) => vec![new_item(message, at, claim, item, today, vault_day)],
        (Resolved::Change { target, field, value }, _) => {
            vec![change(message, at, claim, *target, *field, value.as_deref(), today, proposal(today, vault_day))]
        }
        // §4.9's receipt row (PQ4): `completion::propose_done` in both phases, which always asks and
        // asks once per task; `mail_asof` and supersession do not apply.
        (Resolved::Receipt { target }, _) => vec![Action::ProposeDone { claim: at, target: target.place() }],
        (Resolved::AlreadyApplied, _) => skip(at, SkipReason::AlreadyApplied),
        (Resolved::Restated { .. }, _) => skip(at, SkipReason::Equal),
        (Resolved::NoTarget, _) if claim.kind == ClaimKind::CourseInfo => vec![course_notice(message, at, claim)],
        (Resolved::Unmatched, _) => vec![unmatched(message, at, claim)],
        (Resolved::Ambiguous, _) if claim.course.is_none() => vec![unmatched(message, at, claim)],
        // `resolve` gives `New` only with an item, and `NoTarget` only to `course_info` past the screen.
        _ => skip(at, SkipReason::Unmatched),
    }
}

/// D13 and §5.2: a `course_info` claim opens a notice titled by course and topic and keyed
/// `mail:<course>:<topic>` (the topic's wire word), so a later notice for the same course and topic
/// closes it. With no course it is titled by its topic and keyed by its own claim's uid
/// (`mail:<h>:<n>`), so it closes nothing else and a message delivered twice keeps one open (§4.8
/// step 4). Written in both phases and never held (§4.8 step 5).
fn course_notice(message: &Message, at: ClaimRef, claim: &Claim) -> Action {
    let Some(topic) = claim.topic else { return Action::Skip { claim: at, reason: SkipReason::Noise } };
    let (close_key, title) = match claim.course.as_deref().filter(|course| !course.trim().is_empty()) {
        Some(course) => (format!("mail:{course}:{}", topic.word()), format!("{course}: {}", topic.label())),
        None => {
            let mut label = topic.label().chars();
            let title = label.next().map(|first| first.to_uppercase().chain(label).collect()).unwrap_or_default();
            (at.uid(), title)
        }
    };
    notice(message, at, claim, close_key, title, NoticeAbout::CourseInfo)
}

/// §4.9's unmatched row and D12: an info notice, never a guess, keyed by the claim's own uid as a
/// course-less [`course_notice`] is.
fn unmatched(message: &Message, at: ClaimRef, claim: &Claim) -> Action {
    let close_key = at.uid();
    notice(message, at, claim, close_key, UNMATCHED_NOTICE.to_string(), NoticeAbout::Unmatched)
}

/// An `info/` notice (`info::KINDS`' `notice`) with §5.2's one-line body.
fn notice(message: &Message, at: ClaimRef, claim: &Claim, close_key: String, title: String, about: NoticeAbout) -> Action {
    let body_line = body_line(message, claim);
    Action::Notice { claim: at, kind: "notice", close_key: Some(close_key), title, body_line, about }
}

/// §4.9's first four rows, for a `new_item` naming no note (D24 for events).
fn new_item(message: &Message, at: ClaimRef, claim: &Claim, item: &NewItem, today: Date, vault_day: i64) -> Action {
    let due = item.known_due();
    let past = due.and_then(due_date).is_some_and(|due| due < today);
    let note = |imported_past| NewNote {
        title: item.title.clone(),
        course: claim.course.clone(),
        due: due.map(str::to_string),
        item_kind: item.item_kind,
        effort_hours: item.effort_hours,
        importance: item.importance,
        imported_past,
        mail_asof: due.map(|_| message.ordering_date),
    };
    let direct = writes_direct(message);
    match (item.item_kind, past) {
        (ItemKind::Event, true) if direct => Action::Create { claim: at, note: note(true) },
        (ItemKind::Event, true) => Action::Skip { claim: at, reason: SkipReason::Past },
        (ItemKind::Event, false) => Action::TaskCard { claim: at, note: note(false), proposal: proposal(today, vault_day) },
        (ItemKind::Task | ItemKind::Exam, past) if direct => Action::Create { claim: at, note: note(past) },
        (ItemKind::Task | ItemKind::Exam, _) => Action::TaskCard { claim: at, note: note(false), proposal: proposal(today, vault_day) },
    }
}

/// §4.6 step 6 and §4.9: a message may write directly only when it is a backfilled original and
/// the provider saw it `aligned` at delivery.
pub(super) fn writes_direct(message: &Message) -> bool {
    message.phase == Phase::Backfill && message.authenticity == Authenticity::Aligned
}

/// §4.9's `change` rows 5 to 11 for one claim against one vault note (D11 (1) and (2)), `card` the
/// place a card takes in the deck.
///
/// - **Superseded** (§4.8 step 5, T5.5): a claim ordered before the note's `mail_asof` for its field
///   writes nothing. A stamp at the claim's own ordering date is its own run's (a `Create` or a set
///   of the same message, or of the same run in an earlier pass), so it never supersedes: claims
///   at one ordering date are §4.9's two-value row's to settle ([`settle`]; plan §9.5).
/// - **Row 8, backfill**: a template's change to an item a feed supplies ([`FEEDS`]) writes nothing,
///   counted superseded: the feed runs every slot, so a historical notice never overwrites its
///   newer value, and a due the student set there is the feed's card to file (T8b).
/// - **Nothing changes, nothing is written** ([`SkipReason::Equal`]): a due naming no known date
///   ("no date known", never "date removed"), the value the note holds in the vault's literal form,
///   or a `cancelled` for a note already done or archived (out of the plan already).
/// - **Direct** only for `due` on a mail note (`created_by: mail`) whose `due` the view's
///   `human_set` does not hold, from a message that [`writes_direct`]: rows 5 and 9's backfill
///   column. `mail.rs`'s direct-set writer re-reads the journal first, and may only turn this into a
///   card. On a note outside `tasks/` (archived `imported-past`) only a due still before `today` is
///   set: a later one would leave the plan holding an archived item rank never shows (spec risk 1,
///   T5.3's past). It takes the card row instead, which F3 (b) files and approving un-archives.
/// - **Otherwise a `kind: amend` card**: row 5 live (and an unaligned backfill, §4.6 step 6); row
///   6, a field the student set, a hand-made note's included, in both phases; row 7, a model's change
///   to a fed item, and row 8 live, unless already equal; row 9 to any note but mail's own (plan
///   §9.5: mail writes directly only to its own note); row 10, a Gmail or events note, in both
///   phases; and row 11, `cancelled`, always, proposing `status: active → archived` and no new field.
/// - **A card `approvals::validate_amendment` would refuse is never planned** (D11 (3)): what T8c's
///   two arms still refuse ([`amendable`]) skips as [`SkipReason::NotAmendable`].
///
/// A note this plan files ([`Target::Planned`]) takes these rows as the note it will be, a mail
/// note no human set a field of, so a batch planned whole writes what it writes split across pulls
/// (D10); its actions name it by its creating claim ([`Place::Planned`]).
#[allow(clippy::too_many_arguments)]
fn change(
    message: &Message,
    at: ClaimRef,
    claim: &Claim,
    target: Target<'_>,
    field: ClaimField,
    value: Option<&str>,
    today: Date,
    card: Proposal,
) -> Action {
    let note = target.note();
    let skip = |claim, reason| Action::Skip { claim, reason };
    let name = match field {
        ClaimField::Due => "due",
        ClaimField::Cancelled => "status",
    };
    if superseded(note, name, message) {
        return skip(at, SkipReason::Superseded);
    }
    let (from, to) = match field {
        ClaimField::Cancelled => match note.status.as_deref() {
            Some("done" | "archived") => return skip(at, SkipReason::Equal),
            status => (status.map_or(Value::Null, |s| Value::String(s.to_string())), Value::String("archived".to_string())),
        },
        ClaimField::Due => match value.filter(|due| due_date(due).is_some()) {
            Some(due) => (note.due.clone(), Value::String(due.to_string())),
            None => return skip(at, SkipReason::Equal),
        },
    };
    let fed = note.created_by.as_deref().is_some_and(|by| FEEDS.contains(&by));
    if field == ClaimField::Due && claim.source == ClaimSource::Template && message.phase == Phase::Backfill && fed {
        return skip(at, SkipReason::Superseded);
    }
    if literal(&from) == literal(&to) {
        return skip(at, SkipReason::Equal);
    }
    let mails = note.created_by.as_deref() == Some(MAIL) && !note.human_set.contains(name);
    let stays_past = || to.as_str().and_then(due_date).is_some_and(|due| due < today);
    if field == ClaimField::Due && mails && writes_direct(message) && (note.folder == "tasks" || stays_past()) {
        let (target, field, mail_asof) = (target.place(), name.to_string(), message.ordering_date);
        Action::Set { claim: at, target, field, to, mail_asof, body_line: body_line(message, claim) }
    } else if amendable(note, name, &from, &to, today) {
        let changes = vec![Change { field: name.to_string(), from, to }];
        Action::AmendCard { claim: at, target: target.place(), changes, proposal: card, also: Vec::new() }
    } else {
        skip(at, SkipReason::NotAmendable)
    }
}

/// A value in the vault's literal form (`write::to_literal`, through `yaml::from_json` as
/// `verify_tail` re-applies a journalled value), so a date and a date-time never compare equal.
fn literal(value: &Value) -> String {
    write::to_literal(&crate::yaml::from_json(value))
}

/// §4.8 step 5: whether the note's `mail_asof` for `field` is later than the message's ordering
/// date. Strictly later: a stamp at the message's own ordering date is its own run's ([`change`]).
fn superseded(note: &NoteView, field: &str, message: &Message) -> bool {
    note.mail_asof.get(field).is_some_and(|asof| *asof > message.ordering_date)
}

/// Whether `approvals::validate_amendment` accepts a mail card (`created_by: mail`) amending `field`
/// of `note` from `from` to `to`, as T8c reads it (F3 (b), Quinn 2026-10-02):
/// - `from` a non-null scalar, or, for `due` alone, null: arm (1) fills an empty due;
/// - the note in an amendable folder (`tasks/`, for an item), or, for `due` alone, in `archive/` as
///   `status: archived` with `archived_reason: imported-past`: arm (2), whose approval on or after
///   the new due's day un-archives it. Only a `to` on or after `today` is planned there; an earlier
///   one would only re-date an item that stays archived.
///
/// So a note the student archived, finished or deleted, a `status` amended from null and a
/// collection `from` are still refused, as T8c refuses them.
fn amendable(note: &NoteView, field: &str, from: &Value, to: &Value, today: Date) -> bool {
    let due = field == "due";
    let from_ok = match from {
        Value::Null => due,
        Value::Array(_) | Value::Object(_) => false,
        _ => true,
    };
    let imported_past = note.folder == "archive"
        && note.status.as_deref() == Some("archived")
        && note.archived_reason.as_deref() == Some(crate::ingest::IMPORTED_PAST);
    let ahead = to.as_str().and_then(due_date).is_some_and(|day| day >= today);
    from_ok && (AMENDABLE_FOLDERS.contains(&note.folder.as_str()) || (due && imported_past && ahead))
}

/// §5.2's body line for a direct change: `<date> · <sender>: <why> — "<evidence>"`. The date is the
/// message's Date header, else its ordering date, as a UTC day (the module has no time zone, and
/// the journal's days are UTC); the sender is its display name and address (D21).
pub(crate) fn body_line(message: &Message, claim: &Claim) -> String {
    format!("{} \u{b7} {}: {} \u{2014} \"{}\"", message_day(message), sender(&message.sender), claim.why, claim.evidence)
}

/// The day a card or a body line shows for a message (D21): its Date header, else its ordering date,
/// as a UTC day. `mail.rs`'s writers read it here, so a card and a body line always agree.
pub(crate) fn message_day(message: &Message) -> String {
    message.message_date.unwrap_or(message.ordering_date).strftime("%Y-%m-%d").to_string()
}

/// A sender as D21 names it: `Name <address>`, or the address alone when it has no display name.
pub(crate) fn sender(sender: &Sender) -> String {
    match sender.name.as_deref().map(str::trim).filter(|name| !name.is_empty()) {
        Some(name) => format!("{name} <{}>", sender.address),
        None => sender.address.clone(),
    }
}

/// What one claim says a note's field should be, the key §4.9's two-value row groups by. Only a
/// known due is ever said: a `cancelled` always says `"true"`, so two never differ.
#[derive(Debug, Clone, PartialEq)]
pub(super) struct Said {
    place: Place,
    field: &'static str,
    value: String,
}

/// What `resolved` says, when it is a change to a note's due naming a known date: a vault note, or
/// one this plan files (plan §9.5).
pub(super) fn said(resolved: &Resolved<'_>) -> Option<Said> {
    match resolved {
        Resolved::Change { target, field: ClaimField::Due, value: Some(value) } if due_date(value).is_some() => {
            Some(Said { place: target.place(), field: "due", value: value.clone() })
        }
        _ => None,
    }
}

/// §4.9's row 12 and P20, over one run of claims sharing an ordering date (`super::walk`): claims
/// saying different values for one note's field make one `kind: amend` card, in both phases,
/// whatever each alone planned. A note this plan files is grouped as a vault note is, and holds
/// before the run the due it is filed with, in an earlier run or by its `Create` in this one
/// ([`filed_before`]). The card proposes, from the value the note held before the run
/// (`before`), the value of the last claim in the run's order (uid, then claim index) whose value
/// the note did not already hold, so approving it always changes something (P20 as plan §9.5 reads
/// it); it names every other claim with its value and sender (`also`), and each skips as superseded.
/// A card the executor would refuse is never planned ([`SkipReason::NotAmendable`]). Returns whether
/// it rewrote a step.
///
/// **Once per claim, and once per group (§4.8 step 4, D10).** A claim delivered twice in the run is
/// one member, its first copy; each later copy resolved against the first copy's own actions, which
/// this rewrite may change, so it skips as already applied, never as a member, and no card names its
/// own claim. A member already applied before the run (`before`, never the run's own cards) keeps its
/// skip, and then the group's card was filed when the run was first planned, so nothing more is
/// filed and every other member skips as superseded: a run delivered again files nothing, whether
/// its card is pending or the student already approved it.
///
/// **Supersession (T5.5).** A member's own direct set stamps `mail_asof` at the run's date, which
/// never supersedes another member ([`change`]: only a later stamp does), so this rewrite replaces
/// both. A claim [`rule`] planned as superseded is no member: a run ordered before the note's
/// `mail_asof` from before the run is superseded whole, and a backfill template's claim on a fed
/// item (row 8) says nothing the note could take, so neither makes a card.
pub(super) fn settle(before: &Working, run: &mut [Step<'_>], today: Date, vault_day: i64) -> bool {
    let mut groups: BTreeMap<(Place, &'static str), Vec<usize>> = BTreeMap::new();
    let planned_superseded = |step: &Step<'_>| matches!(step.actions[..], [Action::Skip { reason: SkipReason::Superseded, .. }]);
    for (i, step) in run.iter().enumerate() {
        if let Some(said) = step.said.as_ref().filter(|_| !planned_superseded(step)) {
            groups.entry((said.place.clone(), said.field)).or_default().push(i);
        }
    }
    let mut rewrote = false;
    for ((place, field), steps) in groups {
        let (mut members, mut copies): (Vec<usize>, Vec<usize>) = (Vec::new(), Vec::new());
        for i in steps {
            let copy = members.iter().any(|&m| run[m].at == run[i].at);
            if copy { copies.push(i) } else { members.push(i) }
        }
        let value = |i: usize| run[i].said.as_ref().map_or(Value::Null, |said| Value::String(said.value.clone()));
        let values: BTreeSet<String> = members.iter().map(|&i| literal(&value(i))).collect();
        let Some(note) = filed_before(before, run, &place) else { continue };
        if values.len() < 2 {
            continue;
        }
        // Two values: at least one is not the note's, so this always finds the proposer.
        let held = literal(&note.due);
        let Some(&proposer) = members.iter().rev().find(|&&i| literal(&value(i)) != held) else { continue };
        let applied: BTreeSet<usize> =
            members.iter().copied().filter(|&i| before.resolve(&run[i].at, run[i].claim) == Resolved::AlreadyApplied).collect();
        let card = if !applied.is_empty() {
            Action::Skip { claim: run[proposer].at.clone(), reason: SkipReason::Superseded }
        } else if amendable(&note, field, &note.due, &value(proposer), today) {
            let named = members.iter().filter(|&&i| i != proposer);
            let also = named.map(|&i| Claimed { claim: run[i].at.clone(), value: value(i), sender: run[i].message.sender.clone() });
            let changes = vec![Change { field: field.to_string(), from: note.due.clone(), to: value(proposer) }];
            let (target, proposal) = (place.clone(), proposal(today, vault_day));
            Action::AmendCard { claim: run[proposer].at.clone(), target, changes, proposal, also: also.collect() }
        } else {
            Action::Skip { claim: run[proposer].at.clone(), reason: SkipReason::NotAmendable }
        };
        for &i in members.iter().filter(|&&i| i != proposer && !applied.contains(&i)) {
            run[i].actions = vec![Action::Skip { claim: run[i].at.clone(), reason: SkipReason::Superseded }];
        }
        if !applied.contains(&proposer) {
            run[proposer].actions = vec![card];
        }
        for &i in &copies {
            run[i].actions = vec![Action::Skip { claim: run[i].at.clone(), reason: SkipReason::AlreadyApplied }];
        }
        rewrote = true;
    }
    rewrote
}

/// The note `place` names as it stood before a run: in `before` (a vault note, or one an earlier run
/// of this plan files), else the note a `Create` of this run files, as it files it. A `Create` is
/// never a group member (a new item says no due), so no rewrite has changed it.
fn filed_before(before: &Working, run: &[Step<'_>], place: &Place) -> Option<NoteView> {
    if let Some(note) = before.note_in(place) {
        return Some(note.clone());
    }
    let Place::Planned(by) = place else { return None };
    let mut actions = run.iter().filter(|step| &step.at == by).flat_map(|step| &step.actions);
    let create = actions.find(|action| matches!(action, Action::Create { .. }))?;
    let mut filed = before.clone();
    filed.apply(create);
    filed.note_in(place).cloned()
}

/// A mail card's place in the deck on `today` (§4.8 step 5, plan §10 R16): held to day 2 on the
/// vault's first day, else `pending` today. `commitments::vault_day` is never below 1; a lower
/// number is read as the first day, so a card is held rather than shown early.
pub(crate) fn proposal(today: Date, vault_day: i64) -> Proposal {
    if vault_day <= 1 {
        // Only 9999-12-31 has no next day.
        let day_two = today.tomorrow().unwrap_or(today);
        Proposal { status: CardStatus::Snoozed, proposed_at: day_two, first_proposed_at: today, snooze_until: Some(day_two) }
    } else {
        Proposal { status: CardStatus::Pending, proposed_at: today, first_proposed_at: today, snooze_until: None }
    }
}

/// Whether `due` is in §5.1's wire shape, `YYYY-MM-DD` or `YYYY-MM-DDTHH:MM` with ASCII digits
/// (`judge_validate.ts`'s `ABSOLUTE_DUE_RE`), whatever day and time it names.
pub(super) fn wire_shaped(due: &str) -> bool {
    let shaped = |pattern: &str| {
        due.len() == pattern.len()
            && due.bytes().zip(pattern.bytes()).all(|(c, p)| if p == b'9' { c.is_ascii_digit() } else { c == p })
    };
    shaped("9999-99-99") || shaped("9999-99-99T99:99")
}

/// The date of a due in §5.1's wire shape ([`wire_shaped`]) naming a real date and time; `None` for
/// anything else.
pub(super) fn due_date(due: &str) -> Option<Date> {
    if !wire_shaped(due) {
        return None;
    }
    // Every byte is ASCII now, so each range is on a character boundary.
    let number = |from: usize, to: usize| due[from..to].parse::<i16>().ok();
    let day = Date::new(number(0, 4)?, number(5, 7)? as i8, number(8, 10)? as i8).ok()?;
    if due.len() > 10 {
        jiff::civil::Time::new(number(11, 13)? as i8, number(14, 16)? as i8, 0, 0).ok()?;
    }
    Some(day)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::mailreconcile::{plan, Sender, Topic, VaultView};
    use jiff::civil::date;
    use jiff::Timestamp;
    use Authenticity::{Aligned, Unaligned};
    use ItemKind::{Event, Exam, Task};
    use Phase::{Backfill, Live};

    const UID: &str = "mail:5a5a0e0e1f1f2c2c3d3d4b4b6a6a7979";
    const ORDERING: &str = "2026-09-14T09:00:00Z";

    fn today() -> Date {
        date(2026, 10, 1)
    }

    fn ordering() -> Timestamp {
        ORDERING.parse().unwrap()
    }

    fn item(title: &str, due: Option<&str>, item_kind: ItemKind) -> Claim {
        let item = NewItem { title: title.to_string(), due: due.map(str::to_string), item_kind, effort_hours: 3.5, importance: 4 };
        Claim {
            kind: ClaimKind::NewItem,
            course: Some("CS 101".to_string()),
            target: None,
            field: None,
            value: None,
            item: Some(item),
            topic: None,
            evidence: "posted on the course page".to_string(),
            why: "A new assignment".to_string(),
            confidence: 0.8,
            source: ClaimSource::Model,
            resolution: None,
        }
    }

    fn message(phase: Phase, authenticity: Authenticity, claims: Vec<Claim>) -> Message {
        Message {
            uid: UID.to_string(),
            ordering_date: ordering(),
            message_date: Some(ordering()),
            phase,
            authenticity,
            sender: Sender { name: Some("Course Staff".to_string()), address: "staff@example.edu".to_string() },
            claims,
        }
    }

    fn at() -> ClaimRef {
        ClaimRef { message: UID.to_string(), index: 0 }
    }

    /// The one action a one-claim message plans on an empty vault on vault day `day`.
    fn planned(phase: Phase, authenticity: Authenticity, claim: &Claim, day: i64) -> Action {
        let actions = plan(&VaultView::default(), &[message(phase, authenticity, vec![claim.clone()])], today(), day);
        assert_eq!(actions.len(), 1, "{actions:?}");
        actions.into_iter().next().unwrap()
    }

    /// The note `claim` makes (§5.2), from the claim's own fields.
    fn note(claim: &Claim, imported_past: bool) -> NewNote {
        let item = claim.item.clone().unwrap();
        let mail_asof = item.due.as_ref().map(|_| ordering());
        let (title, due, item_kind, effort_hours, importance) = (item.title, item.due, item.item_kind, item.effort_hours, item.importance);
        NewNote { title, course: claim.course.clone(), due, item_kind, effort_hours, importance, imported_past, mail_asof }
    }

    fn created(claim: &Claim, imported_past: bool) -> Action {
        Action::Create { claim: at(), note: note(claim, imported_past) }
    }

    fn carded(claim: &Claim, proposal: Proposal) -> Action {
        Action::TaskCard { claim: at(), note: note(claim, false), proposal }
    }

    fn pending() -> Proposal {
        Proposal { status: CardStatus::Pending, proposed_at: today(), first_proposed_at: today(), snooze_until: None }
    }

    fn past() -> Action {
        Action::Skip { claim: at(), reason: SkipReason::Past }
    }

    /// Spec test 20, §4.9 row 1, backfill: a task-shaped item due ahead, or with no due ("needs a
    /// date"), is a note written directly. `exam` is task-shaped, and a due today, even at an hour
    /// already gone, is ahead (strictly before today is past, never before now).
    #[test]
    fn a_backfill_task_ahead_or_undated_is_created_direct() {
        for claim in [
            item("Lab 5", Some("2026-10-12"), Task),
            item("Lab 5", None, Task),
            item("Midterm", Some("2026-10-20T09:00"), Exam),
            item("Midterm", None, Exam),
            item("Quiz 4", Some("2026-10-01T08:00"), Task),
            item("Quiz 4", Some("2026-10-01"), Exam),
        ] {
            assert_eq!(planned(Backfill, Aligned, &claim, 9), created(&claim, false), "{:?}", claim.item);
        }
        // §5.2, spelled out: the claim's effort and importance as given, and `mail_asof` for `due` at
        // the ordering date only when the mail gave the due.
        let Action::Create { note, .. } = planned(Backfill, Aligned, &item("Lab 5", Some("2026-10-12"), Task), 9) else { panic!() };
        let fields = (note.due.as_deref(), note.effort_hours, note.importance, note.mail_asof, note.imported_past);
        assert_eq!(fields, (Some("2026-10-12"), 3.5, 4, Some(ordering()), false));
        assert_eq!((note.title.as_str(), note.course.as_deref(), note.item_kind), ("Lab 5", Some("CS 101"), Task));
        let Action::Create { note, .. } = planned(Backfill, Aligned, &item("Lab 5", None, Task), 9) else { panic!() };
        assert_eq!((note.due, note.mail_asof), (None, None), "needs a date, and no stamp for a due mail never gave");
    }

    /// Spec test 20, §4.9 rows 1 and 2, live (a recovered message reads as live): every
    /// task-shaped item, ahead, undated or past, is a `kind: task` card, pending today.
    #[test]
    fn a_live_task_is_a_task_card() {
        for claim in [
            item("Lab 5", Some("2026-10-12"), Task),
            item("Lab 5", None, Task),
            item("Midterm", Some("2026-10-20T09:00"), Exam),
            item("Midterm", None, Exam),
            item("Lab 2", Some("2026-09-20"), Task),
        ] {
            for authenticity in [Aligned, Unaligned] {
                assert_eq!(planned(Live, authenticity, &claim, 9), carded(&claim, pending()), "{:?}", claim.item);
            }
        }
    }

    /// Spec test 23, tasks (§4.9 row 2): already past due, the backfill creates the note archived
    /// `imported-past`; live files a card proposing it as an active task.
    #[test]
    fn a_past_task_backfill_is_created_archived_imported_past_and_live_is_a_card() {
        for claim in [
            item("Lab 3", Some("2026-09-30T23:59"), Task),
            item("Lab 2", Some("2026-09-20"), Task),
            item("Quiz 2", Some("2026-09-18T10:00"), Exam),
        ] {
            let Action::Create { note, .. } = planned(Backfill, Aligned, &claim, 9) else { panic!("{:?}", claim.item) };
            assert!(note.imported_past, "{:?}", claim.item);
            assert_eq!(planned(Backfill, Aligned, &claim, 9), created(&claim, true));
            assert_eq!(planned(Live, Aligned, &claim, 9), carded(&claim, pending()), "{:?}", claim.item);
        }
    }

    /// Spec test 23 and D24 (§4.9 row 3): an event still ahead, or undated, is the student's call
    /// in both phases, whatever the original's authenticity.
    #[test]
    fn a_future_event_is_a_card_in_both_phases() {
        for claim in [
            item("Career fair", Some("2026-10-05T18:00"), Event),
            item("Career fair", Some("2026-10-01T07:00"), Event),
            item("Career fair", Some("2026-10-01"), Event),
            item("Career fair", None, Event),
        ] {
            for (phase, authenticity) in [(Backfill, Aligned), (Backfill, Unaligned), (Live, Aligned), (Live, Unaligned)] {
                let got = planned(phase, authenticity, &claim, 9);
                assert_eq!(got, carded(&claim, pending()), "{phase:?} {authenticity:?} {:?}", claim.item);
            }
        }
    }

    /// Spec test 23 and D24 (§4.9 row 4): an event already past is recorded and archived
    /// `imported-past` by the backfill; live, nothing is written and it counts past, since no one
    /// can accept a past event.
    #[test]
    fn a_past_event_backfill_is_archived_imported_past_and_live_writes_nothing_and_counts() {
        for claim in [item("Career fair", Some("2026-09-25T18:00"), Event), item("Info session", Some("2026-09-30"), Event)] {
            assert_eq!(planned(Backfill, Aligned, &claim, 9), created(&claim, true), "{:?}", claim.item);
            assert_eq!(planned(Live, Aligned, &claim, 9), past(), "{:?}", claim.item);
        }
    }

    /// Spec test 22b (§4.6 step 6, §4.9): an `unaligned` backfilled original may file only cards
    /// and notices. Each row an `aligned` original writes directly files the card its live row
    /// files; a past event, which no card may ask about (D24), writes nothing and counts past.
    #[test]
    fn an_unaligned_backfill_original_makes_cards_where_an_aligned_one_writes_direct() {
        for claim in [
            item("Lab 5", Some("2026-10-12"), Task),
            item("Lab 5", None, Task),
            item("Midterm", Some("2026-10-20T09:00"), Exam),
            item("Lab 2", Some("2026-09-20"), Task),
        ] {
            assert!(matches!(planned(Backfill, Aligned, &claim, 9), Action::Create { .. }), "{:?}", claim.item);
            assert_eq!(planned(Backfill, Unaligned, &claim, 9), carded(&claim, pending()), "{:?}", claim.item);
            assert_eq!(planned(Backfill, Unaligned, &claim, 9), planned(Live, Aligned, &claim, 9), "the live row's card");
        }
        let past_event = item("Career fair", Some("2026-09-25T18:00"), Event);
        assert!(matches!(planned(Backfill, Aligned, &past_event, 9), Action::Create { .. }));
        assert_eq!(planned(Backfill, Unaligned, &past_event, 9), past());
        // One batch: an aligned and an unaligned original side by side. Only the aligned one writes.
        let (aligned, unaligned) = (item("Lab 5", Some("2026-10-12"), Task), item("Lab 6", Some("2026-10-19"), Task));
        let mut forged = message(Backfill, Unaligned, vec![unaligned.clone()]);
        forged.uid = "mail:ffff".to_string();
        let actions = plan(&VaultView::default(), &[message(Backfill, Aligned, vec![aligned.clone()]), forged], today(), 9);
        let forged_at = ClaimRef { message: "mail:ffff".to_string(), index: 0 };
        let forged_card = Action::TaskCard { claim: forged_at, note: note(&unaligned, false), proposal: pending() };
        assert_eq!(actions, vec![created(&aligned, false), forged_card]);
    }

    /// Spec test 22d, the planning half (§4.8 step 5, plan §10 R16): on vault day 1 every card is
    /// `snoozed` to day 2 (`proposed_at` and `snooze_until` day 2, `first_proposed_at` day 1); from
    /// day 2 a card is `pending` today. Direct writes, and a past event's nothing, are not held.
    #[test]
    fn a_first_day_card_is_proposed_and_snoozed_to_day_two() {
        let day_two = date(2026, 10, 2);
        let held = Proposal { status: CardStatus::Snoozed, proposed_at: day_two, first_proposed_at: today(), snooze_until: Some(day_two) };
        assert_eq!(proposal(today(), 1), held);
        assert_eq!(proposal(today(), 2), pending());
        assert_eq!(proposal(today(), 9), pending());
        let next = |day: Date| Some(day.tomorrow().unwrap());
        assert_eq!(proposal(date(2026, 12, 31), 1).snooze_until, next(date(2026, 12, 31)), "a year's end");
        assert_eq!(proposal(date(2028, 2, 28), 1).proposed_at, date(2028, 2, 29), "a leap day");
        let cards = [
            (Live, Aligned, item("Lab 5", Some("2026-10-12"), Task)),
            (Live, Aligned, item("Lab 2", Some("2026-09-20"), Task)),
            (Backfill, Unaligned, item("Lab 5", None, Task)),
            (Backfill, Aligned, item("Career fair", Some("2026-10-05T18:00"), Event)),
        ];
        for (phase, authenticity, claim) in cards {
            assert_eq!(planned(phase, authenticity, &claim, 1), carded(&claim, held), "{phase:?} {:?}", claim.item);
            assert_eq!(planned(phase, authenticity, &claim, 2), carded(&claim, pending()), "{phase:?} {:?}", claim.item);
        }
        for claim in [
            item("Lab 5", Some("2026-10-12"), Task),
            item("Lab 2", Some("2026-09-20"), Task),
            item("Career fair", Some("2026-09-25T18:00"), Event),
        ] {
            assert_eq!(planned(Backfill, Aligned, &claim, 1), planned(Backfill, Aligned, &claim, 9), "{:?}", claim.item);
        }
        assert_eq!(planned(Live, Aligned, &item("Career fair", Some("2026-09-25T18:00"), Event), 1), past());
    }

    /// The device's own check of §5.1's due, as `enrich.rs` re-checks a Gmail due (R-C2-E43): a
    /// `new_item` whose due is not `YYYY-MM-DD` or `YYYY-MM-DDTHH:MM` in ASCII digits never passed
    /// the server's shape check (`ABSOLUTE_DUE_RE`), so it can neither be ordered against today nor
    /// written: it is noise and nothing else, in both phases. A line break or another digit set is
    /// off the shape too.
    #[test]
    fn a_new_item_whose_due_is_not_the_wire_shape_is_noise() {
        let noise = Action::Skip { claim: at(), reason: SkipReason::Noise };
        for due in [
            "2026-10-12T23:59:00", "2026-10-12 23:59", "2026-10-12T23:59Z", "2026-1-12", "12/10/2026", "next Friday",
            "", "\u{FF12}026-10-12", "\u{0662}026-10-12", "2026-10-12T2\u{0663}:00", "+2026-10-12",
            "2026-10-1\u{0661}", "2026-1a-12", "2026/10/12", "2026-10-12t23:59", "2026-10-12\n", "2026-10-1\u{2028}",
            "2026-10-12\r\n", "\n2026-10-12",
        ] {
            assert_eq!(due_date(due), None, "{due:?}");
            for phase in [Backfill, Live] {
                assert_eq!(planned(phase, Aligned, &item("Lab 5", Some(due), Task), 9), noise, "{due:?}");
            }
        }
        assert_eq!(due_date("2026-10-12"), Some(date(2026, 10, 12)));
        assert_eq!(due_date("2026-10-12T23:59"), Some(date(2026, 10, 12)));
        assert_eq!(due_date("2028-02-29T00:00"), Some(date(2028, 2, 29)));
    }

    /// A due in the wire shape naming no real day or time (`2026-09-31`: a misprint in the mail the
    /// model copied, which the server's shape check passes) is no date known: the item "needs a
    /// date" (§4.9 row 1), as the server's resolver reads a due it cannot place on one calendar day
    /// (T3b). So it is created or carded and counted like any undated item, never dropped as noise,
    /// and with no `mail_asof`, since the mail gave no usable due.
    #[test]
    fn a_new_item_whose_shaped_due_names_no_real_day_or_time_needs_a_date() {
        for due in [
            "2026-09-31", "2026-02-29", "2026-02-30", "2026-13-01", "2026-00-10", "2026-10-00", "2026-09-31T23:59",
            "2026-10-12T24:00", "2026-10-12T23:60",
        ] {
            assert_eq!(due_date(due), None, "{due:?}");
            for (title, kind) in [("Lab 5", Task), ("Midterm", Exam), ("Career fair", Event)] {
                let (shaped, undated) = (item(title, Some(due), kind), item(title, None, kind));
                for (phase, authenticity) in [(Backfill, Aligned), (Backfill, Unaligned), (Live, Aligned)] {
                    let got = planned(phase, authenticity, &shaped, 9);
                    assert_eq!(got, planned(phase, authenticity, &undated, 9), "{due:?} {kind:?} {phase:?} {authenticity:?}");
                }
            }
        }
        // The finding's case, spelled out: an aligned backfill creates "Lab 5" active, needing a date.
        let Action::Create { note, .. } = planned(Backfill, Aligned, &item("Lab 5", Some("2026-09-31"), Task), 9) else { panic!() };
        assert_eq!((note.due, note.mail_asof, note.imported_past), (None, None, false));
    }

    // -----------------------------------------------------------------------------------------
    // T5.4: §4.9's change rows (5, 6, 10, 11 and 12), every one using `due`
    // -----------------------------------------------------------------------------------------

    use crate::mailreconcile::{Change, ClaimField, Claimed, NoteView};
    use serde_json::{json, Value};
    use std::collections::BTreeMap;

    const PATH: &str = "tasks/lab-5.md";
    const MOVED: &str = "2026-10-14T23:59";
    /// §5.2's body line for [`moved`] from [`message`]: the Date header's UTC day, the sender's
    /// display name and address (D21), the why, and the evidence quoted.
    const LINE: &str = "2026-09-14 \u{b7} Course Staff <staff@example.edu>: The due date moved \u{2014} \"Lab 5 is now due Wednesday\"";
    const PHASES: [(Phase, Authenticity); 4] = [(Backfill, Aligned), (Backfill, Unaligned), (Live, Aligned), (Live, Unaligned)];

    /// "Lab 5" in CS 101, active in `tasks/`, due 2026-10-12, made by `created_by`, with the fields
    /// `Journal::human_set` finds.
    fn lab(created_by: Option<&str>, human_set: &[&str]) -> NoteView {
        NoteView {
            id: "task-lab5".to_string(),
            path: PATH.to_string(),
            folder: "tasks".to_string(),
            kind: None,
            course: Some("CS 101".to_string()),
            title: "Lab 5".to_string(),
            due: json!("2026-10-12"),
            status: Some("active".to_string()),
            archived_reason: None,
            source_uid: created_by.map(|by| format!("{by}:lab-5")),
            created_by: created_by.map(str::to_string),
            mail_asof: BTreeMap::new(),
            human_set: human_set.iter().map(|f| f.to_string()).collect(),
        }
    }

    /// A `change` claim: "Lab 5" is now due `value`.
    fn moved(value: Option<&str>) -> Claim {
        Claim {
            kind: ClaimKind::Change,
            target: Some("Lab 5".to_string()),
            field: Some(ClaimField::Due),
            value: value.map(str::to_string),
            item: None,
            evidence: "Lab 5 is now due Wednesday".to_string(),
            why: "The due date moved".to_string(),
            ..item("Lab 5", None, Task)
        }
    }

    /// A `change` claim: "Lab 5" is cancelled (§5.1: `value: "true"`).
    fn cancelled() -> Claim {
        let (evidence, why) = ("Lab 5 is cancelled".to_string(), "The lab was cancelled".to_string());
        Claim { field: Some(ClaimField::Cancelled), value: Some("true".to_string()), evidence, why, ..moved(None) }
    }

    /// The actions `messages` plan against `notes` on vault day `day`.
    fn against(notes: Vec<NoteView>, messages: &[Message], day: i64) -> Vec<Action> {
        plan(&VaultView { notes }, messages, today(), day)
    }

    /// The one action a one-claim message plans against `note`.
    fn on(note: &NoteView, phase: Phase, authenticity: Authenticity, claim: &Claim, day: i64) -> Action {
        let actions = against(vec![note.clone()], &[message(phase, authenticity, vec![claim.clone()])], day);
        assert_eq!(actions.len(), 1, "{actions:?}");
        actions.into_iter().next().unwrap()
    }

    fn set(to: &str, body_line: &str) -> Action {
        set_as(at(), to, ordering(), body_line)
    }

    fn set_as(claim: ClaimRef, to: &str, mail_asof: Timestamp, body_line: &str) -> Action {
        let (target, field, to) = (Place::Path(PATH.to_string()), "due".to_string(), json!(to));
        Action::Set { claim, target, field, to, mail_asof, body_line: body_line.to_string() }
    }

    fn amend(field: &str, from: Value, to: Value, proposal: Proposal) -> Action {
        let changes = vec![Change { field: field.to_string(), from, to }];
        Action::AmendCard { claim: at(), target: Place::Path(PATH.to_string()), changes, proposal, also: vec![] }
    }

    fn held() -> Proposal {
        let day_two = date(2026, 10, 2);
        Proposal { status: CardStatus::Snoozed, proposed_at: day_two, first_proposed_at: today(), snooze_until: Some(day_two) }
    }

    fn skip(reason: SkipReason) -> Action {
        Action::Skip { claim: at(), reason }
    }

    /// Spec test 20, §4.9 row 5, backfill: a mail note's field the student never set takes an
    /// aligned original's value directly, with its `mail_asof` stamp and §5.2's body line; another
    /// field the student set does not change that. Not held on the first day.
    #[test]
    fn a_backfill_change_to_a_mail_note_field_not_set_by_the_student_is_direct() {
        for human_set in [&[][..], &["title"], &["title", "status", "course"]] {
            let note = lab(Some("mail"), human_set);
            assert_eq!(on(&note, Backfill, Aligned, &moved(Some(MOVED)), 9), set(MOVED, LINE), "{human_set:?}");
            assert_eq!(on(&note, Backfill, Aligned, &moved(Some(MOVED)), 1), set(MOVED, LINE), "day 1: {human_set:?}");
        }
        // A dated new_item naming the note is a change to its due (test 24), on the same row.
        let restated = item("lab 5", Some("2026-10-13"), Task);
        let line = "2026-09-14 \u{b7} Course Staff <staff@example.edu>: A new assignment \u{2014} \"posted on the course page\"";
        assert_eq!(on(&lab(Some("mail"), &[]), Backfill, Aligned, &restated, 9), set("2026-10-13", line));
        // A mail note still needing a date takes its first one directly.
        let undated = NoteView { due: Value::Null, ..lab(Some("mail"), &[]) };
        assert_eq!(on(&undated, Backfill, Aligned, &moved(Some("2026-10-14")), 9), set("2026-10-14", LINE));
        // No display name: the address alone. No Date header: the ordering date, as its UTC day.
        let mut bare = message(Backfill, Aligned, vec![moved(Some(MOVED))]);
        (bare.sender.name, bare.message_date) = (None, None);
        bare.ordering_date = "2026-09-15T03:30:00Z".parse().unwrap();
        let line = "2026-09-15 \u{b7} staff@example.edu: The due date moved \u{2014} \"Lab 5 is now due Wednesday\"";
        let want = set_as(at(), MOVED, bare.ordering_date, line);
        assert_eq!(against(vec![lab(Some("mail"), &[])], &[bare], 9), vec![want]);
    }

    /// Spec test 20, §4.9 row 5, live (recovered mail reads as live), and 22b for the change rows:
    /// an `unaligned` backfilled original files the card its live row files. Held on day 1.
    #[test]
    fn a_live_change_to_a_mail_note_is_an_amend_card() {
        let note = lab(Some("mail"), &[]);
        for (phase, authenticity) in [(Live, Aligned), (Live, Unaligned), (Backfill, Unaligned)] {
            let card = |proposal| amend("due", json!("2026-10-12"), json!(MOVED), proposal);
            assert_eq!(on(&note, phase, authenticity, &moved(Some(MOVED)), 9), card(pending()), "{phase:?} {authenticity:?}");
            assert_eq!(on(&note, phase, authenticity, &moved(Some(MOVED)), 1), card(held()), "day 1: {phase:?} {authenticity:?}");
        }
    }

    /// Spec test 21, the planning half, a human `create` carrying `due` (D11 (1)): the note is the
    /// student's, so mail about its due is a card in both phases, never a set.
    #[test]
    fn a_hand_created_task_gets_an_amend_card_for_due_in_both_phases() {
        // The console's create carries every field it writes, `created_by` the human token.
        let made = ["title", "course", "domain", "due", "effort_hours", "effort_source", "importance", "status", "progress", "created_by"];
        for created_by in [Some("student"), Some("quinn"), None] {
            let note = lab(created_by, &made);
            for (phase, authenticity) in PHASES {
                let got = on(&note, phase, authenticity, &moved(Some(MOVED)), 9);
                assert_eq!(got, amend("due", json!("2026-10-12"), json!(MOVED), pending()), "{created_by:?} {phase:?} {authenticity:?}");
            }
        }
    }

    /// Spec test 21, the planning half, a human `set` of `due` (D11 (1) and (2)): a mail note whose
    /// due the student set gets a card in both phases, where the same note unset takes the backfill
    /// directly. The view's `human_set` decides, never `created_by` alone.
    #[test]
    fn a_hand_set_due_gets_an_amend_card_in_both_phases() {
        for created_by in [Some("mail"), Some("blackboard"), Some("gmail")] {
            let note = lab(created_by, &["due"]);
            for (phase, authenticity) in PHASES {
                let got = on(&note, phase, authenticity, &moved(Some(MOVED)), 9);
                assert_eq!(got, amend("due", json!("2026-10-12"), json!(MOVED), pending()), "{created_by:?} {phase:?} {authenticity:?}");
            }
        }
        assert_eq!(on(&lab(Some("mail"), &[]), Backfill, Aligned, &moved(Some(MOVED)), 9), set(MOVED, LINE), "unset: direct");
    }

    /// Spec test 20, §4.9 row 10: a Gmail or events note is another source's, never mail's to
    /// write, so a change to it is a card in both phases, whatever the original's authenticity.
    #[test]
    fn a_change_to_a_gmail_or_events_note_is_an_amend_card() {
        for created_by in ["gmail", "events"] {
            let note = lab(Some(created_by), &[]);
            for (phase, authenticity) in PHASES {
                let got = on(&note, phase, authenticity, &moved(Some(MOVED)), 9);
                assert_eq!(got, amend("due", json!("2026-10-12"), json!(MOVED), pending()), "{created_by} {phase:?} {authenticity:?}");
            }
            assert_eq!(on(&note, Live, Aligned, &moved(Some(MOVED)), 1), amend("due", json!("2026-10-12"), json!(MOVED), held()));
        }
    }

    /// Spec test 22c (§4.9 row 11, §5.2, R37): `cancelled` is always a card proposing
    /// `status: active → archived`, in both phases and on every note, even a mail note an aligned
    /// backfill would set directly; the card changes `status` and nothing else, so no new field
    /// appears. A note already out of the plan (done or archived) has nothing to cancel.
    #[test]
    fn cancelled_is_an_archive_card_in_both_phases_and_never_direct() {
        let archive = amend("status", json!("active"), json!("archived"), pending());
        for (created_by, human_set) in [(Some("mail"), &[][..]), (Some("student"), &["due", "status"]), (Some("gmail"), &[]), (Some("events"), &[]), (Some("blackboard"), &[])] {
            let note = lab(created_by, human_set);
            for (phase, authenticity) in PHASES {
                let got = on(&note, phase, authenticity, &cancelled(), 9);
                assert_eq!(got, archive, "{created_by:?} {phase:?} {authenticity:?}");
            }
            let first_day = on(&note, Backfill, Aligned, &cancelled(), 1);
            assert_eq!(first_day, amend("status", json!("active"), json!("archived"), held()), "{created_by:?}");
        }
        let Action::AmendCard { changes, .. } = on(&lab(Some("mail"), &[]), Backfill, Aligned, &cancelled(), 9) else { panic!() };
        assert_eq!(changes.iter().map(|c| c.field.as_str()).collect::<Vec<_>>(), ["status"], "no new field");
        for (folder, status) in [("tasks", "done"), ("tasks", "archived"), ("archive", "done"), ("archive", "archived")] {
            let gone = NoteView { folder: folder.to_string(), status: Some(status.to_string()), ..lab(Some("mail"), &[]) };
            for (phase, authenticity) in PHASES {
                assert_eq!(on(&gone, phase, authenticity, &cancelled(), 9), skip(SkipReason::Equal), "{folder} {status} {phase:?}");
            }
        }
    }

    /// Spec test 20, §4.9 row 12 and P20: two claims about one note's field at one ordering date,
    /// with different values, make one card in both phases, even where one alone would write
    /// directly. It proposes the value of the claim that sorts last by uid and claim index, and
    /// names the other with its sender; the other skips as superseded. Delivery order is irrelevant.
    #[test]
    fn two_claims_with_one_ordering_date_and_different_values_make_one_card_naming_both() {
        let sender = |name: &str, address: &str| Sender { name: Some(name.to_string()), address: address.to_string() };
        let (ada, bo) = (sender("Prof Ada", "ada@example.edu"), sender("TA Bo", "bo@example.edu"));
        let from = |uid: &str, sender: &Sender, phase: Phase, claims: Vec<Claim>| Message {
            uid: uid.to_string(),
            sender: sender.clone(),
            ..message(phase, Aligned, claims)
        };
        let at = |uid: &str, index: usize| ClaimRef { message: uid.to_string(), index };
        let superseded = |claim: ClaimRef| Action::Skip { claim, reason: SkipReason::Superseded };
        let card = |claim: ClaimRef, from: &str, to: &str, also: Vec<Claimed>| {
            let changes = vec![Change { field: "due".to_string(), from: json!(from), to: json!(to) }];
            Action::AmendCard { claim, target: Place::Path(PATH.to_string()), changes, proposal: pending(), also }
        };
        let named = |claim: ClaimRef, value: &str, sender: &Sender| Claimed { claim, value: json!(value), sender: sender.clone() };
        let note = lab(Some("mail"), &[]);
        for phase in [Backfill, Live] {
            let a = from("mail:aaaa", &ada, phase, vec![moved(Some(MOVED))]);
            let b = from("mail:bbbb", &bo, phase, vec![moved(Some("2026-10-16"))]);
            let want = vec![
                superseded(at("mail:aaaa", 0)),
                card(at("mail:bbbb", 0), "2026-10-12", "2026-10-16", vec![named(at("mail:aaaa", 0), MOVED, &ada)]),
            ];
            assert_eq!(against(vec![note.clone()], &[b.clone(), a.clone()], 9), want, "{phase:?}");
            assert_eq!(against(vec![note.clone()], &[a.clone(), b.clone()], 9), want, "{phase:?}, the other way round");
            // The last claim saying the value the note already holds: the card proposes the other
            // (P20 as plan §9.5 reads it), so approving it changes something.
            let holds = from("mail:bbbb", &bo, phase, vec![moved(Some("2026-10-12"))]);
            let want = vec![
                card(at("mail:aaaa", 0), "2026-10-12", MOVED, vec![named(at("mail:bbbb", 0), "2026-10-12", &bo)]),
                superseded(at("mail:bbbb", 0)),
            ];
            assert_eq!(against(vec![note.clone()], &[a.clone(), holds], 9), want, "{phase:?}");
        }
        // Two claims in one message: the later index proposes.
        let both = from("mail:cccc", &ada, Backfill, vec![moved(Some("2026-10-16")), moved(Some(MOVED))]);
        let want = vec![superseded(at("mail:cccc", 0)), card(at("mail:cccc", 1), "2026-10-12", MOVED, vec![named(at("mail:cccc", 0), "2026-10-16", &ada)])];
        assert_eq!(against(vec![note.clone()], &[both], 9), want);
        // One value twice is no conflict: the backfill sets it once, and the second already holds.
        let (a, again) = (from("mail:aaaa", &ada, Backfill, vec![moved(Some(MOVED))]), from("mail:bbbb", &bo, Backfill, vec![moved(Some(MOVED))]));
        let line = "2026-09-14 \u{b7} Prof Ada <ada@example.edu>: The due date moved \u{2014} \"Lab 5 is now due Wednesday\"";
        let first = set_as(at("mail:aaaa", 0), MOVED, ordering(), line);
        let equal = Action::Skip { claim: at("mail:bbbb", 0), reason: SkipReason::Equal };
        assert_eq!(against(vec![note.clone()], &[again.clone(), a.clone()], 9), vec![first.clone(), equal]);
        // At two ordering dates there is no conflict: each applies in turn.
        let later = Message { ordering_date: "2026-09-14T10:00:00Z".parse().unwrap(), ..from("mail:0000", &bo, Backfill, vec![moved(Some("2026-10-16"))]) };
        let line = "2026-09-14 \u{b7} TA Bo <bo@example.edu>: The due date moved \u{2014} \"Lab 5 is now due Wednesday\"";
        let second = set_as(at("mail:0000", 0), "2026-10-16", later.ordering_date, line);
        assert_eq!(against(vec![note.clone()], &[later, a.clone()], 9), vec![first, second]);
        // Delivered again after the card was filed (a lost ack, or a second desktop): nothing more.
        let filed = NoteView { folder: "approvals".to_string(), path: "approvals/amend-lab-5.md".to_string(), source_uid: Some("mail:bbbb:0".to_string()), ..lab(Some("mail"), &[]) };
        let b = from("mail:bbbb", &bo, Backfill, vec![moved(Some("2026-10-16"))]);
        let applied = Action::Skip { claim: at("mail:bbbb", 0), reason: SkipReason::AlreadyApplied };
        assert_eq!(against(vec![note.clone(), filed], &[a, b], 9), vec![superseded(at("mail:aaaa", 0)), applied]);
    }

    /// A change that changes nothing writes nothing, on every row: a due mail names no known date
    /// for (null, or shaped and naming no real day: "no date known", never "date removed", as
    /// T5.2's undated new_item and T5.3's impossible due), or the value the note already holds, in
    /// the vault's literal form, so a date and a date-time on one day are not equal.
    #[test]
    fn a_change_naming_no_known_due_or_the_held_value_writes_nothing() {
        for note in [lab(Some("mail"), &[]), lab(Some("student"), &["due"]), lab(Some("gmail"), &[])] {
            for value in [None, Some("2026-09-31"), Some("2026-10-12T24:00"), Some("2026-10-12")] {
                for (phase, authenticity) in PHASES {
                    let got = on(&note, phase, authenticity, &moved(value), 9);
                    assert_eq!(got, skip(SkipReason::Equal), "{:?} {value:?} {phase:?} {authenticity:?}", note.created_by);
                }
            }
        }
        let note = lab(Some("mail"), &[]);
        assert_eq!(on(&note, Backfill, Aligned, &moved(Some("2026-10-12T23:59")), 9), set("2026-10-12T23:59", LINE));
        let date_time = NoteView { due: json!("2026-10-12T23:59"), ..lab(Some("gmail"), &[]) };
        let got = on(&date_time, Live, Aligned, &moved(Some("2026-10-12")), 9);
        assert_eq!(got, amend("due", json!("2026-10-12T23:59"), json!("2026-10-12"), pending()));
    }

    /// The device's own check of §5.1's shape for a `change`, as T5.3's for a `new_item`'s due: a
    /// `due` value off the wire shape, or a `cancelled` value other than `"true"`, never passed the
    /// server's check, so it is noise and nothing else, in both phases.
    #[test]
    fn a_change_whose_value_is_off_its_fields_shape_is_noise() {
        let note = lab(Some("mail"), &[]);
        for value in ["next Friday", "2026-10-14 23:59", "2026-10-14T23:59:00", "2026-10-14\n", "\u{FF12}026-10-14"] {
            for (phase, authenticity) in PHASES {
                assert_eq!(on(&note, phase, authenticity, &moved(Some(value)), 9), skip(SkipReason::Noise), "{value:?} {phase:?}");
            }
        }
        for value in [None, Some("false"), Some("True"), Some("yes"), Some("2026-10-14")] {
            let claim = Claim { value: value.map(str::to_string), ..cancelled() };
            for (phase, authenticity) in PHASES {
                assert_eq!(on(&note, phase, authenticity, &claim, 9), skip(SkipReason::Noise), "cancelled {value:?} {phase:?}");
            }
        }
    }

    /// D11 (3): a mail card is written only in the shape `approvals::validate_amendment` accepts,
    /// with T8c's two arms (F3 (b), Quinn 2026-10-02): a `from: null` for `due`, and an
    /// `imported-past` note in `archive/`. A card row whose card it would still refuse files nothing
    /// and the note keeps its value (plan §10 R14's rule for the feed's card); where the row writes
    /// directly, the direct write needs neither.
    #[test]
    fn a_card_the_approvals_executor_would_refuse_is_never_planned() {
        let not_amendable = skip(SkipReason::NotAmendable);
        let (folder, path, status) = ("archive".to_string(), "archive/lab-5.md".to_string(), Some("done".to_string()));
        let archived = NoteView { folder, path, status, ..lab(Some("mail"), &[]) };
        let undated = NoteView { due: Value::Null, ..lab(Some("mail"), &[]) };
        let from_null = |proposal| amend("due", Value::Null, json!(MOVED), proposal);
        // The archived note in every phase: a due today or later is never set directly outside
        // `tasks/` either (spec risk 1; `a_backfill_never_sets_an_archived_mail_note_…`). F3 (b): the
        // undated note's card fills its due from null.
        for &(phase, authenticity) in &PHASES[..] {
            assert_eq!(on(&archived, phase, authenticity, &moved(Some(MOVED)), 9), not_amendable, "{} {phase:?}", archived.path);
        }
        for &(phase, authenticity) in &PHASES[1..] {
            assert_eq!(on(&undated, phase, authenticity, &moved(Some(MOVED)), 9), from_null(pending()), "{} {phase:?}", undated.path);
        }
        assert_eq!(on(&undated, Backfill, Aligned, &moved(Some(MOVED)), 9), set(MOVED, LINE));
        // A hand-made note created with `due: null` is the student's: a card row in both phases, and
        // under F3 (b) a card from null.
        let hand = NoteView { due: Value::Null, ..lab(Some("student"), &["title", "due", "status"]) };
        for (phase, authenticity) in PHASES {
            assert_eq!(on(&hand, phase, authenticity, &moved(Some(MOVED)), 9), from_null(pending()), "{phase:?}");
        }
        // `cancelled` on an active note with no `status:` line: `from` would be null.
        let no_status = NoteView { status: None, ..lab(Some("mail"), &[]) };
        assert_eq!(on(&no_status, Live, Aligned, &cancelled(), 9), not_amendable);
        // Row 12 on the undated note: one card from null, naming the other value; neither is set.
        let a = Message { uid: "mail:aaaa".to_string(), ..message(Backfill, Aligned, vec![moved(Some(MOVED))]) };
        let b = Message { uid: "mail:bbbb".to_string(), ..message(Backfill, Aligned, vec![moved(Some("2026-10-16"))]) };
        let (first, last) = (ClaimRef { message: a.uid.clone(), index: 0 }, ClaimRef { message: b.uid.clone(), index: 0 });
        let also = vec![Claimed { claim: first.clone(), value: json!(MOVED), sender: a.sender.clone() }];
        let changes = vec![Change { field: "due".to_string(), from: Value::Null, to: json!("2026-10-16") }];
        let card = Action::AmendCard { claim: last, target: Place::Path(PATH.to_string()), changes, proposal: pending(), also };
        let want = vec![Action::Skip { claim: first, reason: SkipReason::Superseded }, card];
        assert_eq!(against(vec![undated], &[b, a], 9), want);
    }

    /// §4.8 step 4 and D10 on §4.9's two-value row: one message delivered twice in a run is one
    /// claim. Its first copy is the group's member and each later copy skips as already applied, so
    /// the card is the one a single copy files, and no card names its own claim.
    #[test]
    fn a_claim_delivered_twice_in_a_two_value_run_is_planned_once() {
        let at = |uid: &str| ClaimRef { message: uid.to_string(), index: 0 };
        let note = lab(Some("mail"), &[]);
        for phase in [Backfill, Live] {
            let a = Message { uid: "mail:aaaa".to_string(), ..message(phase, Aligned, vec![moved(Some(MOVED))]) };
            let b = Message { uid: "mail:bbbb".to_string(), ..message(phase, Aligned, vec![moved(Some("2026-10-16"))]) };
            let changes = vec![Change { field: "due".to_string(), from: json!("2026-10-12"), to: json!("2026-10-16") }];
            let also = vec![Claimed { claim: at("mail:aaaa"), value: json!(MOVED), sender: a.sender.clone() }];
            let card = Action::AmendCard { claim: at("mail:bbbb"), target: Place::Path(PATH.to_string()), changes, proposal: pending(), also };
            let superseded = Action::Skip { claim: at("mail:aaaa"), reason: SkipReason::Superseded };
            assert_eq!(against(vec![note.clone()], &[a.clone(), b.clone()], 9), vec![superseded.clone(), card.clone()], "{phase:?}");
            let want = vec![superseded, card, Action::Skip { claim: at("mail:bbbb"), reason: SkipReason::AlreadyApplied }];
            for delivered in [[a.clone(), b.clone(), b.clone()], [b.clone(), a.clone(), b.clone()], [b.clone(), b.clone(), a.clone()]] {
                assert_eq!(against(vec![note.clone()], &delivered, 9), want, "{phase:?}");
            }
        }
    }

    /// P20 as plan §9.5 reads it: a two-value card proposes the value of the last claim, by uid and
    /// claim index, whose value the note does not already hold, and names every other claim. Read
    /// literally, P20 would propose the note's own value when one message names the old date and
    /// the new, a card whose approval changes nothing (`approvals::apply_amendment` reads `to` in
    /// place as applied). Delivered again once the card is filed, pending or approved, the run
    /// files nothing more (§4.8 step 4).
    #[test]
    fn a_two_value_card_proposes_the_last_value_the_note_does_not_hold() {
        let note = lab(Some("mail"), &[]);
        let at = |index| ClaimRef { message: UID.to_string(), index };
        for phase in [Backfill, Live] {
            let both = message(phase, Aligned, vec![moved(Some(MOVED)), moved(Some("2026-10-12"))]);
            let changes = vec![Change { field: "due".to_string(), from: json!("2026-10-12"), to: json!(MOVED) }];
            let also = vec![Claimed { claim: at(1), value: json!("2026-10-12"), sender: both.sender.clone() }];
            let card = Action::AmendCard { claim: at(0), target: Place::Path(PATH.to_string()), changes, proposal: pending(), also };
            let superseded = Action::Skip { claim: at(1), reason: SkipReason::Superseded };
            assert_eq!(against(vec![note.clone()], &[both.clone()], 9), vec![card, superseded.clone()], "{phase:?}");
            let (folder, path) = ("approvals".to_string(), "approvals/amend-lab-5.md".to_string());
            let filed = NoteView { folder, path, source_uid: Some(at(0).uid()), ..lab(Some("mail"), &[]) };
            let applied = Action::Skip { claim: at(0), reason: SkipReason::AlreadyApplied };
            for due in ["2026-10-12", MOVED] {
                let now = NoteView { due: json!(due), ..note.clone() };
                let got = against(vec![now, filed.clone()], &[both.clone()], 9);
                assert_eq!(got, vec![applied.clone(), superseded.clone()], "{phase:?}, the note due {due}");
            }
        }
    }

    /// Spec risk 1 and T5.3's `imported-past` (a due before today): a direct set never gives a note
    /// outside `tasks/` a due today or later, where rank would never show it. A backfill that
    /// archives "Lab 5" in one slot and brings its extension in the next (§4.8 step 1) writes no
    /// more than the same batch planned whole (D10). A due still before today is set. F3 (b): a due
    /// today or later is a card, which approving un-archives (T8c), named by the note's path split
    /// and by its creating claim whole.
    #[test]
    fn a_backfill_never_sets_an_archived_mail_note_to_a_due_today_or_later() {
        let (first, extended) = (item("Lab 5", Some("2026-09-28"), Task), moved(Some("2026-10-05")));
        let later_at = ClaimRef { message: "mail:bbbb".to_string(), index: 0 };
        let later = |claims: Vec<Claim>| Message {
            uid: later_at.message.clone(),
            ordering_date: "2026-09-20T09:00:00Z".parse().unwrap(),
            message_date: None,
            ..message(Backfill, Aligned, claims)
        };
        let slot_one = against(vec![], &[message(Backfill, Aligned, vec![first.clone()])], 9);
        assert_eq!(slot_one, vec![created(&first, true)]);
        let archived = NoteView {
            folder: "archive".to_string(),
            path: "archive/lab-5.md".to_string(),
            status: Some("archived".to_string()),
            archived_reason: Some(crate::ingest::IMPORTED_PAST.to_string()),
            due: json!("2026-09-28"),
            source_uid: Some(at().uid()),
            mail_asof: BTreeMap::from([("due".to_string(), ordering())]),
            ..lab(Some("mail"), &[])
        };
        let card = |target: Place, due: &str| {
            let changes = vec![Change { field: "due".to_string(), from: json!("2026-09-28"), to: json!(due) }];
            Action::AmendCard { claim: later_at.clone(), target, changes, proposal: pending(), also: vec![] }
        };
        for due in ["2026-10-05", "2026-10-01", "2026-10-01T08:00"] {
            let want = card(Place::Path("archive/lab-5.md".to_string()), due);
            assert_eq!(against(vec![archived.clone()], &[later(vec![moved(Some(due))])], 9), vec![want], "{due}");
        }
        let slot_two = against(vec![archived.clone()], &[later(vec![extended.clone()])], 9);
        let whole = against(vec![], &[message(Backfill, Aligned, vec![first.clone()]), later(vec![extended])], 9);
        let [Action::AmendCard { claim, target: Place::Path(path), changes, proposal, also }] = &slot_two[..] else { panic!("{slot_two:?}") };
        assert_eq!(path, "archive/lab-5.md");
        let (claim, changes, proposal, also) = (claim.clone(), changes.clone(), *proposal, also.clone());
        let named = Action::AmendCard { claim, target: Place::Planned(at()), changes, proposal, also };
        assert_eq!(whole, [slot_one.clone(), vec![named]].concat(), "split or whole, the same plan (plan §9.5)");
        let still_past = against(vec![archived.clone()], &[later(vec![moved(Some("2026-09-30"))])], 9);
        let set_there = matches!(&still_past[..], [Action::Set { target: Place::Path(path), to, .. }] if path == "archive/lab-5.md" && to == &json!("2026-09-30"));
        assert!(set_there, "{still_past:?}");
        // Whole, the set names the note the batch's own `Create` files.
        let whole = against(vec![], &[message(Backfill, Aligned, vec![first.clone()]), later(vec![moved(Some("2026-09-30"))])], 9);
        let [Action::Set { claim, field, to, mail_asof, body_line, .. }] = &still_past[..] else { panic!() };
        let (claim, field, to, mail_asof, body_line) = (claim.clone(), field.clone(), to.clone(), *mail_asof, body_line.clone());
        assert_eq!(whole, [slot_one, vec![Action::Set { claim, target: Place::Planned(at()), field, to, mail_asof, body_line }]].concat());
    }

    // -----------------------------------------------------------------------------------------
    // Plan §9.5's planned-note amendment: a change to a note this plan files
    // -----------------------------------------------------------------------------------------

    /// One message at `ordering`, its Date header unknown (so a body line shows the ordering day).
    fn sent(uid: &str, ordering: &str, phase: Phase, authenticity: Authenticity, claims: Vec<Claim>) -> Message {
        let (uid, ordering_date) = (uid.to_string(), ordering.parse().unwrap());
        Message { uid, ordering_date, message_date: None, ..message(phase, authenticity, claims) }
    }

    fn first_of(uid: &str) -> ClaimRef {
        ClaimRef { message: uid.to_string(), index: 0 }
    }

    /// Spec test 18's third case, the pure half (D10), and the Checkpoint A failing input: one closed
    /// aligned backfill batch on an empty vault, M1 (09-02) posting "Lab 5" due 10-12 and M2 (09-09)
    /// moving it to 10-08. Planned whole, in either delivery order, M2 reaches the note M1 creates,
    /// named by M1's claim, exactly as it reaches that note by its path once M1 is written (the batch
    /// split over two slots). Live or unaligned, the move is the card a mail note gets; `cancelled`
    /// is the archive card. The note is never left at 10-12 and never reported unmatched.
    #[test]
    fn a_batch_applied_whole_reaches_the_item_it_creates() {
        let posted = item("Lab 5", Some("2026-10-12"), Task);
        let m1 = sent("mail:aaaa", "2026-09-02T09:00:00Z", Backfill, Aligned, vec![posted.clone()]);
        let create = Action::Create { claim: first_of("mail:aaaa"), note: NewNote { mail_asof: Some(m1.ordering_date), ..note(&posted, false) } };
        let written = NoteView {
            source_uid: Some(first_of("mail:aaaa").uid()),
            mail_asof: BTreeMap::from([("due".to_string(), m1.ordering_date)]),
            ..lab(Some("mail"), &[])
        };
        assert_eq!(against(vec![], &[m1.clone()], 9), vec![create.clone()], "slot one");
        let line = "2026-09-09 \u{b7} Course Staff <staff@example.edu>: The due date moved \u{2014} \"Lab 5 is now due Wednesday\"";
        for (phase, authenticity) in PHASES {
            for claim in [moved(Some("2026-10-08")), cancelled()] {
                let m2 = sent("mail:bbbb", "2026-09-09T09:00:00Z", phase, authenticity, vec![claim.clone()]);
                let [slot_two] = &against(vec![written.clone()], &[m2.clone()], 9)[..] else { panic!("{phase:?}") };
                let planned = Place::Planned(first_of("mail:aaaa"));
                let named = match slot_two.clone() {
                    Action::Set { claim, target: Place::Path(path), field, to, mail_asof, body_line } if path == PATH => {
                        assert_eq!((phase, authenticity, to.clone(), body_line.as_str()), (Backfill, Aligned, json!("2026-10-08"), line));
                        Action::Set { claim, target: planned, field, to, mail_asof, body_line }
                    }
                    Action::AmendCard { claim, target: Place::Path(path), changes, proposal, also } if path == PATH => {
                        Action::AmendCard { claim, target: planned, changes, proposal, also }
                    }
                    other => panic!("{phase:?} {authenticity:?} {:?}: {other:?}", claim.field),
                };
                let want = vec![create.clone(), named];
                assert_eq!(against(vec![], &[m1.clone(), m2.clone()], 9), want, "{phase:?} {authenticity:?} {:?}", claim.field);
                assert_eq!(against(vec![], &[m2, m1.clone()], 9), want, "delivered the other way round");
            }
        }
    }

    /// D10 and §4.8 step 4 across two desktops, and a pass cut short: one aligned backfill message
    /// creates "Lab 5" due 2026-10-12 and moves it to 2026-10-14. The desktop that plans it first
    /// creates the note and moves it (plan §9.5). The other, once both writes have synced, writes
    /// nothing more, never a card from 10-12 to 10-14. A re-plan after a pass that wrote the note and
    /// stopped before the move still moves it: the claim that created the note said nothing about its
    /// due when first planned, so it says nothing on a re-plan either.
    #[test]
    fn a_message_that_creates_and_moves_an_item_moves_it_once_on_every_desktop() {
        let lab_5 = item("Lab 5", Some("2026-10-12"), Task);
        let both = message(Backfill, Aligned, vec![lab_5.clone(), moved(Some(MOVED))]);
        let second_at = ClaimRef { message: UID.to_string(), index: 1 };
        let (field, to, body_line) = ("due".to_string(), json!(MOVED), LINE.to_string());
        let moved_there = Action::Set { claim: second_at.clone(), target: Place::Planned(at()), field, to, mail_asof: ordering(), body_line };
        assert_eq!(against(vec![], &[both.clone()], 9), vec![created(&lab_5, false), moved_there]);
        let mail_asof = BTreeMap::from([("due".to_string(), ordering())]);
        let synced = NoteView { source_uid: Some(at().uid()), mail_asof, due: json!(MOVED), ..lab(Some("mail"), &[]) };
        let equal = Action::Skip { claim: second_at.clone(), reason: SkipReason::Equal };
        assert_eq!(against(vec![synced.clone()], &[both.clone()], 9), vec![skip(SkipReason::AlreadyApplied), equal]);
        let cut_short = NoteView { due: json!("2026-10-12"), ..synced };
        let want = vec![skip(SkipReason::AlreadyApplied), set_as(second_at, MOVED, ordering(), LINE)];
        assert_eq!(against(vec![cut_short], &[both], 9), want);
    }

    /// §4.9's row 12 and P20 on a note this plan files, as on a vault note: two claims at one
    /// ordering date saying different dues for it make one card from the due it was filed with,
    /// whether it was filed in an earlier run or by a claim of the same run. Delivered again once the
    /// note and the card have synced, the run files nothing more (§4.8 step 4).
    #[test]
    fn a_two_value_run_on_a_note_this_plan_files_makes_one_card() {
        let lab_5 = item("Lab 5", Some("2026-10-12"), Task);
        let at = |index| ClaimRef { message: UID.to_string(), index };
        let superseded = |claim| Action::Skip { claim, reason: SkipReason::Superseded };
        let card = |claim, target, also| {
            let changes = vec![Change { field: "due".to_string(), from: json!("2026-10-12"), to: json!("2026-10-16") }];
            Action::AmendCard { claim, target, changes, proposal: pending(), also }
        };
        let sender = message(Backfill, Aligned, vec![]).sender;
        // Filed by a claim of the same run.
        let one = message(Backfill, Aligned, vec![lab_5.clone(), moved(Some(MOVED)), moved(Some("2026-10-16"))]);
        let also = vec![Claimed { claim: at(1), value: json!(MOVED), sender: sender.clone() }];
        let want = vec![created(&lab_5, false), superseded(at(1)), card(at(2), Place::Planned(at(0)), also)];
        assert_eq!(against(vec![], &[one.clone()], 9), want);
        let mail_asof = BTreeMap::from([("due".to_string(), ordering())]);
        let synced = NoteView { source_uid: Some(at(0).uid()), mail_asof, ..lab(Some("mail"), &[]) };
        let (folder, path) = ("approvals".to_string(), "approvals/amend-lab-5.md".to_string());
        let filed = NoteView { folder, path, source_uid: Some(at(2).uid()), ..lab(Some("mail"), &[]) };
        let applied = |claim| Action::Skip { claim, reason: SkipReason::AlreadyApplied };
        assert_eq!(against(vec![synced, filed], &[one], 9), vec![applied(at(0)), superseded(at(1)), applied(at(2))]);
        // Filed in an earlier run.
        let m1 = sent("mail:aaaa", "2026-09-02T09:00:00Z", Backfill, Aligned, vec![lab_5.clone()]);
        let (a, b) = (first_of("mail:bbbb"), first_of("mail:cccc"));
        let m2 = sent("mail:bbbb", "2026-09-09T09:00:00Z", Backfill, Aligned, vec![moved(Some(MOVED))]);
        let m3 = sent("mail:cccc", "2026-09-09T09:00:00Z", Live, Aligned, vec![moved(Some("2026-10-16"))]);
        let create = Action::Create { claim: first_of("mail:aaaa"), note: NewNote { mail_asof: Some(m1.ordering_date), ..note(&lab_5, false) } };
        let also = vec![Claimed { claim: a.clone(), value: json!(MOVED), sender }];
        let want = vec![create, superseded(a), card(b, Place::Planned(first_of("mail:aaaa")), also)];
        assert_eq!(against(vec![], &[m3, m1, m2], 9), want);
    }

    // -----------------------------------------------------------------------------------------
    // T5.5: §4.9's feed and template rows (7, 8, 9), P10's feeds and D24
    // -----------------------------------------------------------------------------------------

    /// The same claim, from an LMS or platform template (§4.7 step 3).
    fn template(claim: Claim) -> Claim {
        Claim { source: ClaimSource::Template, ..claim }
    }

    fn card_to(to: &str, proposal: Proposal) -> Action {
        amend("due", json!("2026-10-12"), json!(to), proposal)
    }

    /// Spec test 22, §4.9 row 7: a model's change to an item a feed supplies (P10's [`FEEDS`]) is a
    /// `kind: amend` card in both phases, whatever the original's authenticity, where the same change
    /// to a mail note would be set directly. Held on day 1.
    #[test]
    fn a_model_change_to_an_lms_item_is_a_card() {
        for created_by in FEEDS {
            let note = lab(Some(created_by), &[]);
            for (phase, authenticity) in PHASES {
                let got = on(&note, phase, authenticity, &moved(Some(MOVED)), 9);
                assert_eq!(got, card_to(MOVED, pending()), "{created_by} {phase:?} {authenticity:?}");
            }
            assert_eq!(on(&note, Live, Aligned, &moved(Some(MOVED)), 1), card_to(MOVED, held()), "{created_by}");
        }
    }

    /// Spec test 22, §4.9 rows 7 and 8: an item a feed supplies that already holds the claimed value
    /// writes nothing, from the model in both phases and from a live template, compared in the
    /// vault's literal form, so a date-time on the held day is still a card.
    #[test]
    fn an_lms_item_already_holding_the_value_writes_nothing() {
        for created_by in FEEDS {
            let note = lab(Some(created_by), &[]);
            for (phase, authenticity) in PHASES {
                let got = on(&note, phase, authenticity, &moved(Some("2026-10-12")), 9);
                assert_eq!(got, skip(SkipReason::Equal), "{created_by} {phase:?} {authenticity:?}");
            }
            assert_eq!(on(&note, Live, Aligned, &template(moved(Some("2026-10-12"))), 9), skip(SkipReason::Equal), "{created_by}");
            let date_time = on(&note, Live, Aligned, &moved(Some("2026-10-12T23:59")), 9);
            assert_eq!(date_time, card_to("2026-10-12T23:59", pending()), "{created_by}");
        }
    }

    /// Spec test 22, §4.9 row 8, backfill: a template's change to an item a feed supplies writes
    /// nothing and counts superseded, held value or not, since the feed runs every slot and a
    /// historical notice never overwrites its newer value. A due the student set on that item is the
    /// feed's card to file (T8b), so it writes nothing either. Two template claims at one ordering
    /// date make no card between them; the model's change to the same item is still row 7's card.
    #[test]
    fn a_backfill_template_change_to_a_fed_item_writes_nothing_and_counts_superseded() {
        for created_by in FEEDS {
            for human_set in [&[][..], &["due"]] {
                let note = lab(Some(created_by), human_set);
                for authenticity in [Aligned, Unaligned] {
                    for value in [MOVED, "2026-10-12", "2026-10-12T23:59"] {
                        let got = on(&note, Backfill, authenticity, &template(moved(Some(value))), 9);
                        assert_eq!(got, skip(SkipReason::Superseded), "{created_by} {human_set:?} {authenticity:?} {value}");
                    }
                }
            }
            let note = lab(Some(created_by), &[]);
            assert_eq!(on(&note, Backfill, Aligned, &moved(Some(MOVED)), 9), card_to(MOVED, pending()), "{created_by}: the model's");
            let a = Message { uid: "mail:aaaa".to_string(), ..message(Backfill, Aligned, vec![template(moved(Some(MOVED)))]) };
            let b = Message { uid: "mail:bbbb".to_string(), ..message(Backfill, Aligned, vec![template(moved(Some("2026-10-16")))]) };
            let superseded = |uid: &str| Action::Skip { claim: ClaimRef { message: uid.to_string(), index: 0 }, reason: SkipReason::Superseded };
            assert_eq!(against(vec![note], &[b, a], 9), vec![superseded("mail:aaaa"), superseded("mail:bbbb")], "{created_by}");
        }
    }

    /// Spec test 22, §4.9 row 9: a template's change to a note no feed supplies is direct in an
    /// aligned backfill, as row 5 writes a mail note, with its stamp and body line, and a card live
    /// or from an unaligned original. Mail writes directly only to its own note (plan §9.5): a Gmail
    /// or events note (row 10) and a field the student set (row 6) stay cards in both phases.
    #[test]
    fn a_backfill_template_change_to_an_unfed_note_is_direct() {
        let claim = template(moved(Some(MOVED)));
        let note = lab(Some("mail"), &[]);
        assert_eq!(on(&note, Backfill, Aligned, &claim, 9), set(MOVED, LINE));
        assert_eq!(on(&note, Backfill, Aligned, &claim, 1), set(MOVED, LINE), "not held on day 1");
        for (phase, authenticity) in [(Backfill, Unaligned), (Live, Aligned), (Live, Unaligned)] {
            assert_eq!(on(&note, phase, authenticity, &claim, 9), card_to(MOVED, pending()), "{phase:?} {authenticity:?}");
        }
        let made = ["title", "course", "due", "status", "created_by"];
        for (created_by, human_set) in [(Some("gmail"), &[][..]), (Some("events"), &[]), (Some("mail"), &["due"]), (Some("student"), &made), (None, &made)] {
            for (phase, authenticity) in PHASES {
                let got = on(&lab(created_by, human_set), phase, authenticity, &claim, 9);
                assert_eq!(got, card_to(MOVED, pending()), "{created_by:?} {human_set:?} {phase:?} {authenticity:?}");
            }
        }
    }

    /// Spec test 22, §4.9 rows 8 and 9, live (recovered mail reads as live): a template's change is a
    /// card unless the note already holds the value, on a fed item and an unfed note alike. Held on
    /// day 1.
    #[test]
    fn a_live_template_change_is_a_card_unless_equal() {
        for created_by in FEEDS.into_iter().chain(["mail", "gmail", "events"]) {
            let note = lab(Some(created_by), &[]);
            for authenticity in [Aligned, Unaligned] {
                let got = on(&note, Live, authenticity, &template(moved(Some(MOVED))), 9);
                assert_eq!(got, card_to(MOVED, pending()), "{created_by} {authenticity:?}");
                let equal = on(&note, Live, authenticity, &template(moved(Some("2026-10-12"))), 9);
                assert_eq!(equal, skip(SkipReason::Equal), "{created_by} {authenticity:?}");
            }
            assert_eq!(on(&note, Live, Aligned, &template(moved(Some(MOVED))), 1), card_to(MOVED, held()), "{created_by}");
        }
    }

    /// P10: [`FEEDS`] names exactly the creators whose notes a feed owns: `ingest`'s two note
    /// templates (`blackboard`, the feed card's creator too), and the coursework parsers' values
    /// (`zybooks`, `vhl`; each parser's own test pins its value on a parsed note). Gmail, events,
    /// mail, the rules, coursework's map card, every `agent:` actor and a hand-made note are not
    /// feeds, so only a fed item takes row 8.
    #[test]
    fn feed_creators_hold_the_feeds_and_nothing_else() {
        let mut feeds = FEEDS.to_vec();
        feeds.sort_unstable();
        assert_eq!(feeds, ["blackboard", "vhl", "zybooks"]);
        for template in [crate::ingest::NOTE_TEMPLATE, crate::ingest::IMPORTED_PAST_TEMPLATE] {
            assert!(template.contains("\ncreated_by: blackboard\n") && FEEDS.contains(&"blackboard"), "{template}");
        }
        assert!(FEEDS.contains(&crate::approvals::TITLE_CARD_CREATOR));
        for (source, value) in [(include_str!("../zybooks.rs"), "zybooks"), (include_str!("../vhl.rs"), "vhl")] {
            assert!(source.contains(&format!("created_by: \"{value}\".to_string(),")) && FEEDS.contains(&value), "{value}");
        }
        let not_feeds = [
            Some("gmail"), Some("events"), Some("mail"), Some("rules"), Some("coursework"), Some("agent:commitments"),
            Some("agent:knowlu.enrich"), Some("agent:knowlu.completion"), Some("agent:knowlu.sync"), Some("student"), Some("quinn"), None,
        ];
        for created_by in not_feeds {
            assert!(!created_by.is_some_and(|by| FEEDS.contains(&by)), "{created_by:?}");
            let got = on(&lab(created_by, &[]), Backfill, Aligned, &template(moved(Some(MOVED))), 9);
            assert_ne!(got, skip(SkipReason::Superseded), "{created_by:?}: row 8 is a fed item's");
        }
    }

    // -----------------------------------------------------------------------------------------
    // T5.5: supersession through `mail_asof` (§4.8 step 5, §5.2, spec test 19)
    // -----------------------------------------------------------------------------------------

    /// One message `uid` at `ordering`, its Date header unknown.
    fn at_time(uid: &str, ordering: &str, phase: Phase, claims: Vec<Claim>) -> Message {
        sent(uid, ordering, phase, Aligned, claims)
    }

    /// "Lab 5" as the note holds it after mail applied a due at [`ORDERING`].
    fn stamped(created_by: Option<&str>, human_set: &[&str]) -> NoteView {
        NoteView { mail_asof: BTreeMap::from([("due".to_string(), ordering())]), ..lab(created_by, human_set) }
    }

    /// Spec test 19, across pulls: a claim ordered before the note's `mail_asof` for its field is
    /// superseded and writes nothing, on every row and in both phases, whatever it says; a claim
    /// ordered after it takes its row. A claim at the stamp's own ordering date is of the run that
    /// stamped it (§4.9's two-value row), never superseded by it (plan §9.5): a re-delivered claim
    /// reads equal, and a pass cut short still finishes. A run ordered before the stamp is superseded
    /// whole, with no two-value card.
    #[test]
    fn a_claim_earlier_than_mail_asof_is_superseded() {
        let earlier = "2026-09-14T08:59:00Z";
        for (created_by, human_set) in [(Some("mail"), &[][..]), (Some("student"), &["due"]), (Some("gmail"), &[]), (Some("blackboard"), &[])] {
            let note = stamped(created_by, human_set);
            for (phase, authenticity) in PHASES {
                for claim in [moved(Some(MOVED)), moved(Some("2026-10-12")), item("lab 5", Some(MOVED), Task)] {
                    let got = against(vec![note.clone()], &[sent(UID, earlier, phase, authenticity, vec![claim])], 9);
                    assert_eq!(got, vec![skip(SkipReason::Superseded)], "{created_by:?} {phase:?} {authenticity:?}");
                }
            }
        }
        let later = at_time(UID, "2026-09-14T09:01:00Z", Backfill, vec![moved(Some(MOVED))]);
        let line = "2026-09-14 \u{b7} Course Staff <staff@example.edu>: The due date moved \u{2014} \"Lab 5 is now due Wednesday\"";
        assert_eq!(against(vec![stamped(Some("mail"), &[])], &[later.clone()], 9), vec![set_as(at(), MOVED, later.ordering_date, line)]);
        assert_eq!(on(&stamped(Some("mail"), &[]), Backfill, Aligned, &moved(Some(MOVED)), 9), set(MOVED, LINE), "at the stamp");
        assert_eq!(on(&stamped(Some("mail"), &[]), Live, Aligned, &moved(Some("2026-10-12")), 9), skip(SkipReason::Equal));
        // The stamp is per field: a `cancelled` ordered before a `due` stamp is still its card.
        let cancel = against(vec![stamped(Some("mail"), &[])], &[sent(UID, earlier, Live, Aligned, vec![cancelled()])], 9);
        assert_eq!(cancel, vec![amend("status", json!("active"), json!("archived"), pending())]);
        // A run of two values ordered before the stamp: superseded whole.
        let a = at_time("mail:aaaa", earlier, Backfill, vec![moved(Some(MOVED))]);
        let b = at_time("mail:bbbb", earlier, Backfill, vec![moved(Some("2026-10-16"))]);
        let superseded = |uid: &str| Action::Skip { claim: first_of(uid), reason: SkipReason::Superseded };
        assert_eq!(against(vec![stamped(Some("mail"), &[])], &[b, a], 9), vec![superseded("mail:aaaa"), superseded("mail:bbbb")]);
    }

    /// Spec test 19: a backfill value already applied (its `mail_asof` the backfill message's
    /// ordering date) supersedes a live claim ordered before it, mail older than the backfill's
    /// forwarded or recovered after setup, which would otherwise be a card. Live mail ordered after
    /// it is the card a mail note's live change files.
    #[test]
    fn a_live_claim_older_than_an_applied_backfill_value_is_superseded() {
        let backfill = at_time("mail:bbbb", "2026-09-20T09:00:00Z", Backfill, vec![moved(Some(MOVED))]);
        let [Action::Set { mail_asof, .. }] = &against(vec![lab(Some("mail"), &[])], &[backfill.clone()], 9)[..] else { panic!() };
        let written = NoteView { due: json!(MOVED), mail_asof: BTreeMap::from([("due".to_string(), *mail_asof)]), ..lab(Some("mail"), &[]) };
        for authenticity in [Aligned, Unaligned] {
            for value in ["2026-10-16", "2026-10-12"] {
                let older = sent("mail:aaaa", "2026-09-18T09:00:00Z", Live, authenticity, vec![moved(Some(value))]);
                let got = against(vec![written.clone()], &[older], 9);
                assert_eq!(got, vec![Action::Skip { claim: first_of("mail:aaaa"), reason: SkipReason::Superseded }], "{authenticity:?} {value}");
            }
        }
        let newer = sent("mail:cccc", "2026-09-22T09:00:00Z", Live, Aligned, vec![moved(Some("2026-10-16"))]);
        let changes = vec![Change { field: "due".to_string(), from: json!(MOVED), to: json!("2026-10-16") }];
        let card = Action::AmendCard { claim: first_of("mail:cccc"), target: Place::Path(PATH.to_string()), changes, proposal: pending(), also: vec![] };
        assert_eq!(against(vec![written], &[newer], 9), vec![card]);
    }

    /// Spec test 19 (§4.8, D10): a message whose Date header is in the future is ordered at its
    /// received time (its ordering date), stamps `mail_asof` there, and never blocks later mail,
    /// in the same pull or a later one. Its body line still shows the date it carries (D21).
    #[test]
    fn a_future_dated_message_orders_at_its_received_time() {
        let future = Message { message_date: Some("2099-01-01T00:00:00Z".parse().unwrap()), ..at_time("mail:ffff", "2026-09-25T08:00:00Z", Backfill, vec![moved(Some(MOVED))]) };
        let next = at_time("mail:0000", "2026-09-26T08:00:00Z", Backfill, vec![moved(Some("2026-10-16"))]);
        let line = |date: &str| format!("{date} \u{b7} Course Staff <staff@example.edu>: The due date moved \u{2014} \"Lab 5 is now due Wednesday\"");
        let first = set_as(first_of("mail:ffff"), MOVED, future.ordering_date, &line("2099-01-01"));
        let second = set_as(first_of("mail:0000"), "2026-10-16", next.ordering_date, &line("2026-09-26"));
        for delivered in [[future.clone(), next.clone()], [next.clone(), future.clone()]] {
            assert_eq!(against(vec![lab(Some("mail"), &[])], &delivered, 9), vec![first.clone(), second.clone()]);
        }
        let written = NoteView { due: json!(MOVED), mail_asof: BTreeMap::from([("due".to_string(), future.ordering_date)]), ..lab(Some("mail"), &[]) };
        assert_eq!(against(vec![written], &[next], 9), vec![second], "a later pull");
    }

    // -----------------------------------------------------------------------------------------
    // T5.5: notices (D13, D12, §4.9's unmatched row, spec test 26's planning half)
    // -----------------------------------------------------------------------------------------

    /// A `course_info` claim about `topic`.
    fn info(topic: Option<Topic>) -> Claim {
        let (evidence, why) = ("Office hours move to Room 210".to_string(), "Office hours moved".to_string());
        Claim { kind: ClaimKind::CourseInfo, item: None, topic, evidence, why, ..item("Lab 5", None, Task) }
    }

    fn notice(claim: ClaimRef, close_key: &str, title: &str, body_line: &str, about: NoticeAbout) -> Action {
        let (close_key, title, body_line) = (Some(close_key.to_string()), title.to_string(), body_line.to_string());
        Action::Notice { claim, kind: "notice", close_key, title, body_line, about }
    }

    /// Spec test 26's planning half and D13: a `course_info` claim opens an `info/` notice titled by
    /// course and topic and keyed `mail:<course>:<topic>` (the topic's wire word), so a later one for
    /// the same course and topic closes it. Written in both phases with §5.2's body line, never held
    /// on day 1. With no course it is keyed by its own claim's uid, superseding nothing, and a message
    /// delivered twice keeps one notice open. A `course_info` with no topic is off §5.1's shape: noise.
    #[test]
    fn course_info_opens_a_notice_keyed_mail_course_topic() {
        let line = "2026-09-14 \u{b7} Course Staff <staff@example.edu>: Office hours moved \u{2014} \"Office hours move to Room 210\"";
        let want = notice(at(), "mail:CS 101:office_hours", "CS 101: office hours", line, NoticeAbout::CourseInfo);
        for (phase, authenticity) in PHASES {
            for day in [1, 9] {
                assert_eq!(planned(phase, authenticity, &info(Some(Topic::OfficeHours)), day), want, "{phase:?} {authenticity:?} {day}");
            }
        }
        let topics = [Topic::Schedule, Topic::Room, Topic::Instructor, Topic::OfficeHours, Topic::Policy, Topic::Other];
        for (topic, title) in topics.into_iter().zip(["schedule", "room", "instructor", "office hours", "policy", "course news"]) {
            assert_eq!(serde_json::from_value::<Topic>(json!(topic.word())).unwrap(), topic, "the wire word");
            let want = notice(at(), &format!("mail:CS 101:{}", topic.word()), &format!("CS 101: {title}"), line, NoticeAbout::CourseInfo);
            assert_eq!(planned(Live, Aligned, &info(Some(topic)), 9), want, "{topic:?}");
        }
        let courseless = Claim { course: None, ..info(Some(Topic::Room)) };
        assert_eq!(planned(Live, Aligned, &courseless, 9), notice(at(), &at().uid(), "Room", line, NoticeAbout::CourseInfo));
        assert_eq!(planned(Live, Aligned, &info(None), 9), skip(SkipReason::Noise));
        // Notices change nothing in the plan: a later claim still resolves against the same vault.
        let both = message(Live, Aligned, vec![info(Some(Topic::Room)), moved(Some(MOVED))]);
        let got = against(vec![lab(Some("gmail"), &[])], &[both], 9);
        assert!(matches!(&got[..], [Action::Notice { .. }, Action::AmendCard { .. }]), "{got:?}");
    }

    /// The text of D12's notice for a claim no item matched.
    const UNMATCHED: &str = "Knowlu could not match this to an item";

    /// §4.9's unmatched row and D12: a claim no item matches after resolve (a stored "none", or a
    /// stored id naming no item) is an info notice with §5.2's body line, in both phases, never a
    /// guess and never held, keyed by its own claim's uid so it supersedes nothing and a message
    /// delivered twice keeps one open. A claim with no course has no candidates to ask about, so it
    /// is unmatched too. One with a course that `mail-resolve` has not answered yet writes nothing
    /// (§4.8 step 3: the pass asks before it plans).
    #[test]
    fn an_unmatched_claim_is_a_notice() {
        let unmatched = notice(at(), &at().uid(), UNMATCHED, LINE, NoticeAbout::Unmatched);
        let answered = |target_id: Option<&str>| Claim { resolution: Some(crate::mailreconcile::Resolution { target_id: target_id.map(str::to_string) }), ..moved(Some(MOVED)) };
        let courseless = Claim { course: None, target: Some("Project proposal".to_string()), ..moved(Some(MOVED)) };
        for claim in [answered(None), answered(Some("task-gone")), courseless] {
            for (phase, authenticity) in PHASES {
                for day in [1, 9] {
                    let got = on(&lab(Some("mail"), &[]), phase, authenticity, &claim, day);
                    assert_eq!(got, unmatched, "{:?} {phase:?} {authenticity:?} {day}", claim.resolution);
                }
            }
        }
        let unanswered = Claim { target: Some("Project proposal".to_string()), ..moved(Some(MOVED)) };
        assert_eq!(on(&lab(Some("mail"), &[]), Live, Aligned, &unanswered, 9), skip(SkipReason::Unmatched));
    }

    // -----------------------------------------------------------------------------------------
    // T5.5: receipts (PQ4, Quinn 2026-10-02; §4.9's receipt row)
    // -----------------------------------------------------------------------------------------

    /// A submission receipt for "Lab 5", from a template.
    fn receipt() -> Claim {
        let (evidence, why) = ("Submission received: Lab 5".to_string(), "The platform received the submission".to_string());
        Claim { kind: ClaimKind::Receipt, target: Some("Lab 5".to_string()), item: None, evidence, why, ..template(item("Lab 5", None, Task)) }
    }

    fn done_at(claim: ClaimRef, target: Place) -> Action {
        Action::ProposeDone { claim, target }
    }

    /// §4.9's receipt row and PQ4: a template's receipt is `completion::propose_done` on its item in
    /// both phases, whatever the original's authenticity or the note's maker, and on day 1 too; never
    /// a direct write. Its target resolves as a change's does, a stored answer included, and
    /// `mail_asof` does not apply to it.
    #[test]
    fn a_template_receipt_is_propose_done_in_both_phases() {
        let lab_5 = done_at(at(), Place::Path(PATH.to_string()));
        for (created_by, human_set) in [(Some("mail"), &[][..]), (Some("zybooks"), &[]), (Some("student"), &["due", "status"]), (Some("gmail"), &[])] {
            for (phase, authenticity) in PHASES {
                for day in [1, 9] {
                    assert_eq!(on(&lab(created_by, human_set), phase, authenticity, &receipt(), day), lab_5, "{created_by:?} {phase:?} {day}");
                }
            }
        }
        let early = sent(UID, "2026-09-14T08:00:00Z", Live, Aligned, vec![receipt()]);
        assert_eq!(against(vec![stamped(Some("mail"), &[])], &[early], 9), vec![lab_5.clone()], "never superseded");
        let other = NoteView { id: "task-lab6".to_string(), path: "tasks/lab-6.md".to_string(), title: "Lab 6".to_string(), ..lab(Some("mail"), &[]) };
        let answered = Claim { resolution: Some(crate::mailreconcile::Resolution { target_id: Some("task-lab6".to_string()) }), ..receipt() };
        let got = against(vec![lab(Some("mail"), &[]), other], &[message(Live, Aligned, vec![answered])], 9);
        assert_eq!(got, vec![done_at(at(), Place::Path("tasks/lab-6.md".to_string()))], "the stored answer");
    }

    /// PQ4: only a template emits a receipt, so the device re-checks the shape: a `receipt` from the
    /// model, or one carrying a `field`, `value`, `item` or `topic`, is noise in both phases.
    #[test]
    fn a_receipt_from_the_model_is_noise() {
        let item = item("Lab 5", None, Task).item;
        let off_shape = [
            Claim { source: ClaimSource::Model, ..receipt() },
            Claim { field: Some(ClaimField::Due), ..receipt() },
            Claim { value: Some("true".to_string()), ..receipt() },
            Claim { item, ..receipt() },
            Claim { topic: Some(Topic::Room), ..receipt() },
        ];
        for claim in off_shape {
            for (phase, authenticity) in PHASES {
                assert_eq!(on(&lab(Some("mail"), &[]), phase, authenticity, &claim, 9), skip(SkipReason::Noise), "{phase:?} {claim:?}");
            }
        }
    }

    /// Plan §9.5's planned-note amendment for a receipt: a message that posts "Lab 5" and confirms
    /// its submission names the note its own `Create` files, by that claim, exactly as it names the
    /// written note by its path once the batch is split over two slots (D10).
    #[test]
    fn a_receipt_on_a_note_this_plan_files_names_it_by_its_creating_claim() {
        let posted = item("Lab 5", Some("2026-10-12"), Task);
        let both = message(Backfill, Aligned, vec![posted.clone(), receipt()]);
        let second = ClaimRef { message: UID.to_string(), index: 1 };
        assert_eq!(against(vec![], &[both.clone()], 9), vec![created(&posted, false), done_at(second.clone(), Place::Planned(at()))]);
        let written = NoteView { source_uid: Some(at().uid()), mail_asof: BTreeMap::from([("due".to_string(), ordering())]), ..lab(Some("mail"), &[]) };
        let split = against(vec![written], &[both], 9);
        assert_eq!(split, vec![skip(SkipReason::AlreadyApplied), done_at(second, Place::Path(PATH.to_string()))]);
    }

    /// PQ4 and §4.9's unmatched row: a receipt no item matches after resolve is the unmatched
    /// notice, never a guess; one whose course `mail-resolve` has not answered yet writes nothing.
    #[test]
    fn an_unmatched_receipt_is_an_unmatched_notice() {
        let line = "2026-09-14 \u{b7} Course Staff <staff@example.edu>: The platform received the submission \u{2014} \"Submission received: Lab 5\"";
        let unmatched = notice(at(), &at().uid(), UNMATCHED, line, NoticeAbout::Unmatched);
        let none = Claim { resolution: Some(crate::mailreconcile::Resolution { target_id: None }), ..receipt() };
        let courseless = Claim { course: None, target: Some("Project proposal".to_string()), ..receipt() };
        for claim in [none, courseless] {
            for (phase, authenticity) in PHASES {
                assert_eq!(on(&lab(Some("mail"), &[]), phase, authenticity, &claim, 9), unmatched, "{phase:?} {claim:?}");
            }
        }
        let unanswered = Claim { target: Some("Project proposal".to_string()), ..receipt() };
        assert_eq!(on(&lab(Some("mail"), &[]), Live, Aligned, &unanswered, 9), skip(SkipReason::Unmatched));
    }

    /// D24: events and schedules stay the student's call, so no action ever names a note in
    /// `commitments/`: whatever a claim says, in either phase, a commitment is never its target, by
    /// title, by uid or by a stored answer naming it.
    #[test]
    fn no_action_ever_targets_commitments() {
        let class = NoteView {
            id: "cmt-cs101".to_string(),
            path: "commitments/cs-101-lecture.md".to_string(),
            folder: "commitments".to_string(),
            title: "Lab 5".to_string(),
            source_uid: Some("Lab 5".to_string()),
            ..lab(Some("student"), &["title", "due", "status"])
        };
        let named = |claim: Claim| Claim { resolution: Some(crate::mailreconcile::Resolution { target_id: Some("cmt-cs101".to_string()) }), ..claim };
        let claims = [
            moved(Some(MOVED)), cancelled(), receipt(), item("Lab 5", Some(MOVED), Task), item("Lab 5", Some(MOVED), Event),
            item("Lab 5", None, Event), info(Some(Topic::Schedule)), named(moved(Some(MOVED))), named(cancelled()), named(receipt()),
        ];
        for claim in claims {
            for (phase, authenticity) in PHASES {
                for action in against(vec![class.clone()], &[message(phase, authenticity, vec![claim.clone()])], 9) {
                    let place = match &action {
                        Action::Set { target, .. } | Action::AmendCard { target, .. } | Action::ProposeDone { target, .. } => Some(target),
                        _ => None,
                    };
                    assert!(!matches!(place, Some(Place::Path(path)) if path.starts_with("commitments/")), "{phase:?} {claim:?}: {action:?}");
                }
            }
        }
    }

    // -----------------------------------------------------------------------------------------
    // T5.5: F3 (b)'s cards (Quinn 2026-10-02; T8c), and the residual `NotAmendable`
    // -----------------------------------------------------------------------------------------

    /// "Lab 5" archived on import because its due 2026-09-28 had passed (`imported-past`), made by
    /// `created_by`.
    fn imported(created_by: &str) -> NoteView {
        NoteView {
            folder: "archive".to_string(),
            path: "archive/lab-5.md".to_string(),
            status: Some("archived".to_string()),
            archived_reason: Some(crate::ingest::IMPORTED_PAST.to_string()),
            due: json!("2026-09-28"),
            ..lab(Some(created_by), &[])
        }
    }

    fn card_on(path: &str, from: Value, to: &str, proposal: Proposal) -> Action {
        let changes = vec![Change { field: "due".to_string(), from, to: json!(to) }];
        Action::AmendCard { claim: at(), target: Place::Path(path.to_string()), changes, proposal, also: vec![] }
    }

    /// F3 (b) arm (1): a mail note still needing a date takes a live date as a card from `null`,
    /// which T8c's executor accepts and fills (`from: null`). Held on day 1. An aligned backfill still
    /// sets it directly (row 5).
    #[test]
    fn a_live_date_for_a_needs_a_date_mail_note_is_an_amend_card_from_null() {
        let undated = NoteView { due: Value::Null, ..lab(Some("mail"), &[]) };
        for &(phase, authenticity) in &PHASES[1..] {
            let got = on(&undated, phase, authenticity, &moved(Some(MOVED)), 9);
            assert_eq!(got, card_on(PATH, Value::Null, MOVED, pending()), "{phase:?} {authenticity:?}");
            assert_eq!(on(&undated, phase, authenticity, &moved(Some(MOVED)), 1), card_on(PATH, Value::Null, MOVED, held()));
        }
        assert_eq!(on(&undated, Backfill, Aligned, &moved(Some(MOVED)), 9), set(MOVED, LINE));
    }

    /// F3 (b) arm (1) as the plan reads it (keyed on the card, plan T8c and §9.5): a hand-made note
    /// with no due is the student's (row 6), so a date for it is a card from `null` in both phases.
    #[test]
    fn a_date_for_a_hand_made_note_with_no_due_is_an_amend_card_from_null() {
        for created_by in [Some("student"), Some("quinn"), None] {
            let hand = NoteView { due: Value::Null, ..lab(created_by, &["title", "due", "status"]) };
            for (phase, authenticity) in PHASES {
                let got = on(&hand, phase, authenticity, &moved(Some(MOVED)), 9);
                assert_eq!(got, card_on(PATH, Value::Null, MOVED, pending()), "{created_by:?} {phase:?} {authenticity:?}");
            }
        }
    }

    /// F3 (b) arm (2): a due on or after today for an archived `imported-past` note is a card in
    /// both phases, which T8c's executor accepts and approving un-archives (a mail note's, or a feed
    /// item's from the model; a backfill template for a fed item is still row 8's nothing). A due
    /// still before today is set directly where the row writes directly, and is no card elsewhere.
    #[test]
    fn a_due_ahead_for_an_archived_imported_past_mail_note_is_an_amend_card() {
        let from = json!("2026-09-28");
        for created_by in ["mail", "blackboard"] {
            let note = imported(created_by);
            for (phase, authenticity) in PHASES {
                for due in ["2026-10-05", "2026-10-01", "2026-10-01T08:00"] {
                    let got = on(&note, phase, authenticity, &moved(Some(due)), 9);
                    assert_eq!(got, card_on("archive/lab-5.md", from.clone(), due, pending()), "{created_by} {phase:?} {authenticity:?} {due}");
                }
                if (phase, authenticity, created_by) != (Backfill, Aligned, "mail") {
                    assert_eq!(on(&note, phase, authenticity, &moved(Some("2026-09-30")), 9), skip(SkipReason::NotAmendable), "{created_by} {phase:?}");
                }
            }
            assert_eq!(on(&note, Live, Aligned, &moved(Some("2026-10-05")), 1), card_on("archive/lab-5.md", from.clone(), "2026-10-05", held()));
        }
        assert_eq!(on(&imported("blackboard"), Backfill, Aligned, &template(moved(Some("2026-10-05"))), 9), skip(SkipReason::Superseded));
        let still_past = on(&imported("mail"), Backfill, Aligned, &moved(Some("2026-09-30")), 9);
        assert!(matches!(&still_past, Action::Set { target: Place::Path(path), .. } if path == "archive/lab-5.md"), "{still_past:?}");
    }

    /// F3 (b) reaches only what the import archived: a note in `archive/` the student finished,
    /// archived or deleted by hand (no `archived_reason: imported-past`, or not `status: archived`,
    /// as T8c reads it) still skips as `NotAmendable` in every phase, a due ahead included.
    #[test]
    fn a_note_the_student_archived_is_still_not_amendable() {
        let tails = [
            (Some("done"), None),
            (Some("archived"), None),
            (Some("archived"), Some("done")),
            (Some("active"), Some(crate::ingest::IMPORTED_PAST)),
            (None, Some(crate::ingest::IMPORTED_PAST)),
        ];
        for (status, archived_reason) in tails {
            let note = NoteView {
                status: status.map(str::to_string),
                archived_reason: archived_reason.map(str::to_string),
                ..imported("mail")
            };
            for (phase, authenticity) in PHASES {
                for due in ["2026-10-05", "2026-10-01"] {
                    let got = on(&note, phase, authenticity, &moved(Some(due)), 9);
                    assert_eq!(got, skip(SkipReason::NotAmendable), "{status:?} {archived_reason:?} {phase:?} {due}");
                }
            }
        }
    }

    /// O1 T2a (a pin; no code changes): a note the source said was turned in (`imported-done`) is no
    /// `imported-past` note. A `due` move on it is refused however far ahead the new due is, while the
    /// same move on an `imported-past` note is still allowed (`amendable`'s one archive arm).
    #[test]
    fn an_imported_done_note_is_not_amendable_but_an_imported_past_one_still_is() {
        let past = imported("mail");
        let done = NoteView { archived_reason: Some(crate::ingest::IMPORTED_DONE.to_string()), ..imported("mail") };
        for to in ["2026-10-05", "2027-01-01", "2099-12-31"] {
            let to = json!(to);
            assert!(!amendable(&done, "due", &done.due, &to, today()), "imported-done must refuse {to}");
            assert!(amendable(&past, "due", &past.due, &to, today()), "imported-past must still allow {to}");
        }
    }

    /// A `cancelled` for an active note with no `status:` line would amend from a null `status`,
    /// which T8c's first arm (`due` only) never accepts: still `NotAmendable` in every phase.
    #[test]
    fn a_cancelled_claim_for_a_note_with_no_status_line_is_still_not_amendable() {
        for created_by in [Some("mail"), Some("gmail"), Some("student"), Some("blackboard")] {
            let no_status = NoteView { status: None, ..lab(created_by, &[]) };
            for (phase, authenticity) in PHASES {
                assert_eq!(on(&no_status, phase, authenticity, &cancelled(), 9), skip(SkipReason::NotAmendable), "{created_by:?} {phase:?}");
            }
        }
    }

    // -----------------------------------------------------------------------------------------
    // T5.6: PQ8 (ii), a live task already pending as a mail card files nothing
    // -----------------------------------------------------------------------------------------

    /// The first route's copy (S0 capture 5's shape, synthetic): the message as it reached Knowlu
    /// through one connected mailbox, whose claim filed the card.
    const ROUTED: &str = "mail:1e1e2d2d3c3c4b4b5a5a69697878f0f0";

    /// A mail `kind: task` card in `approvals/` with `status`, proposing "Lab 5" in CS 101, filed by
    /// [`ROUTED`]'s claim.
    fn proposing(status: &str) -> NoteView {
        NoteView {
            id: "card-lab5".to_string(),
            path: "approvals/task-lab-5.md".to_string(),
            folder: "approvals".to_string(),
            kind: Some("task".to_string()),
            status: Some(status.to_string()),
            source_uid: Some(first_of(ROUTED).uid()),
            ..lab(Some("mail"), &[])
        }
    }

    /// PQ8 (ii) (Quinn 2026-10-02): a rule forward ("FW:") of a course mail reaches Knowlu through a
    /// second connected mailbox as a new message with a new Message-ID, so intake's dedup cannot see
    /// it. Its live `new_item` naming the task an open mail card already proposes (normalised title,
    /// same course) files nothing: one skip, already applied, whatever its item kind or authenticity,
    /// when it has no known due or its due falls on the card's due day (PQ13 (b), Quinn 2026-10-03).
    /// The day is the calendar day of the floating wall-clock due, the date and the date-time alike,
    /// on the card's side too. Alone, the same claim plans its own row; the card is what silences it.
    #[test]
    fn a_live_new_item_matching_a_pending_mail_task_card_files_nothing() {
        let card = proposing("pending");
        let timed = NoteView { due: json!("2026-10-12T08:00"), ..card.clone() };
        for claim in [
            item("Lab 5", Some("2026-10-12"), Task),
            item("lab-5", Some("2026-10-12T23:59"), Task),
            item("Lab 5", Some("2026-10-12T00:00"), Exam),
            item("Lab 5", Some("2026-10-12T18:00"), Event),
            item("  LAB #5. ", None, Task),
            item("Lab 5", Some("2026-09-31"), Task),
        ] {
            for authenticity in [Aligned, Unaligned] {
                let forwarded = message(Live, authenticity, vec![claim.clone()]);
                for card in [&card, &timed] {
                    let got = against(vec![card.clone()], &[forwarded.clone()], 9);
                    assert_eq!(got, vec![skip(SkipReason::AlreadyApplied)], "{:?} {authenticity:?} {}", claim.item, card.due);
                    assert_ne!(against(vec![], &[forwarded.clone()], 9), got, "alone: {:?}", claim.item);
                }
            }
        }
        // PQ13 (b), Quinn 2026-10-03: these five carried another due day and moved from the skip to
        // their own rows, exactly as with no card (before T5.6): a card each, and a live event
        // already past is counted past (D24).
        for (claim, filed) in [
            (item("lab-5", Some("2026-10-14T23:59"), Task), None),
            (item("Lab 5", Some("2026-09-20"), Task), None),
            (item("Lab 5", Some("2026-10-20T09:00"), Exam), None),
            (item("Lab 5", Some("2026-10-05T18:00"), Event), None),
            (item("Lab 5", Some("2026-09-25T18:00"), Event), Some(past())),
        ] {
            for authenticity in [Aligned, Unaligned] {
                let forwarded = message(Live, authenticity, vec![claim.clone()]);
                let want = vec![filed.clone().unwrap_or_else(|| carded(&claim, pending()))];
                assert_eq!(against(vec![card.clone()], &[forwarded.clone()], 9), want, "{:?} {authenticity:?}", claim.item);
                assert_eq!(against(vec![], &[forwarded], 9), want, "as with no card: {:?}", claim.item);
            }
        }
        // The card comes before D24: a past copy on the card's own due day is that card's, already
        // applied, never counted past (the event) or carded again (the task), as each is alone.
        for gone in [json!("2026-09-25"), json!("2026-09-25T09:00")] {
            let gone = NoteView { due: gone, ..card.clone() };
            for (claim, alone) in [
                (item("Lab 5", Some("2026-09-25T18:00"), Event), Some(past())),
                (item("Lab 5", Some("2026-09-25"), Task), None),
            ] {
                for authenticity in [Aligned, Unaligned] {
                    let forwarded = message(Live, authenticity, vec![claim.clone()]);
                    let got = against(vec![gone.clone()], &[forwarded.clone()], 9);
                    assert_eq!(got, vec![skip(SkipReason::AlreadyApplied)], "past: {:?} {authenticity:?} {}", claim.item, gone.due);
                    let alone = alone.clone().unwrap_or_else(|| carded(&claim, pending()));
                    assert_eq!(against(vec![], &[forwarded], 9), vec![alone], "alone: {:?}", claim.item);
                }
            }
        }
    }

    /// PQ13 (b) (Quinn 2026-10-03): a live `new_item` whose due falls on another day than the open
    /// mail card's files its own card, as before T5.6, so a moved date is never lost unseen: the
    /// addendum read's "Lab 5" due 2026-10-14T23:59 against a card due 2026-10-12, the day before at
    /// 23:59, and a date for a card that has none. A weekly "Quiz" reusing its title with next week's
    /// due files its own card too, and a second route's copy of next week's then meets that card,
    /// whole or split. Accepted costs, not pinned here: two copies the model reads two dates from
    /// file two cards, and a time moved within the card's due day is not caught while it is open. A
    /// third follows from (b)'s "no due" clause, and the first test's no-due row pins its mechanism:
    /// a weekly item reusing an open card's title that arrives with no due (this week's "Quiz",
    /// announced with no date while last week's card is open) is read as that card's copy, skipped
    /// and acked with no card and no notice, so it surfaces only if later mail names it again.
    #[test]
    fn a_live_new_item_with_another_due_files_its_own_card() {
        let card = proposing("pending");
        let undated = NoteView { due: Value::Null, ..card.clone() };
        let cases = [
            (&card, item("lab-5", Some("2026-10-14T23:59"), Task)),
            (&card, item("Lab 5", Some("2026-10-11T23:59"), Task)),
            (&card, item("Lab 5", Some("2026-10-13T00:00"), Exam)),
            (&undated, item("Lab 5", Some("2026-10-12"), Task)),
        ];
        for (card, claim) in cases {
            let got = against(vec![card.clone()], &[message(Live, Aligned, vec![claim.clone()])], 9);
            assert_eq!(got, vec![carded(&claim, pending())], "{:?} against {}", claim.item, card.due);
        }
        let none = against(vec![undated], &[message(Live, Aligned, vec![item("Lab 5", None, Task)])], 9);
        assert_eq!(none, vec![skip(SkipReason::AlreadyApplied)], "no due on either side: the same task");

        // Last week's card came from last week's mail, so neither copy below is applied by its uid.
        let last_mail = Some(first_of("mail:0b0b1c1c2d2d3e3e4f4f5a5a6b6b7c7c").uid());
        let quiz = |due: &str| NoteView {
            title: "Quiz".to_string(),
            path: "approvals/task-quiz.md".to_string(),
            due: json!(due),
            source_uid: last_mail.clone(),
            ..card.clone()
        };
        let (this_week, forward) = (item("Quiz", Some("2026-10-12T09:00"), Task), item("quiz", Some("2026-10-12T09:00"), Task));
        let posted = sent(UID, "2026-10-01T08:00:00Z", Live, Aligned, vec![this_week.clone()]);
        let routed = sent(ROUTED, "2026-10-01T08:05:00Z", Live, Aligned, vec![forward]);
        let filed = Action::TaskCard { claim: at(), note: NewNote { mail_asof: Some(posted.ordering_date), ..note(&this_week, false) }, proposal: pending() };
        let applied = Action::Skip { claim: first_of(ROUTED), reason: SkipReason::AlreadyApplied };
        let last_week = quiz("2026-10-05T09:00");
        for delivered in [[posted.clone(), routed.clone()], [routed.clone(), posted.clone()]] {
            assert_eq!(against(vec![last_week.clone()], &delivered, 9), vec![filed.clone(), applied.clone()], "whole");
        }
        assert_eq!(against(vec![last_week.clone()], &[posted], 9), vec![filed], "pull one: next week's card");
        let next_week = NoteView { path: "approvals/task-quiz-2.md".to_string(), source_uid: Some(at().uid()), ..quiz("2026-10-12T09:00") };
        let both = vec![last_week, next_week];
        assert_eq!(against(both, &[routed], 9), vec![applied], "pull two meets next week's card");
    }

    /// The plan's reading of "pending" (T5.6): a card is an open decision while `pending` or
    /// `snoozed`, as §9.5's open-decision rule reads an amend card. A first-day mail card is written
    /// snoozed to day 2 (R16), so a second route's copy meets a snoozed card on day 1, and on a later
    /// day may meet one the budget or the student snoozed. Either blocks, in the vault or filed
    /// earlier in the same plan.
    #[test]
    fn a_snoozed_mail_task_card_blocks_it_too() {
        let (posted, forwarded) = (item("Lab 5", Some("2026-10-12"), Task), item("LAB 5", Some("2026-10-12"), Task));
        for day in [1, 2] {
            let got = against(vec![proposing("snoozed")], &[message(Live, Aligned, vec![forwarded.clone()])], day);
            assert_eq!(got, vec![skip(SkipReason::AlreadyApplied)], "vault day {day}");
        }
        let first = sent(ROUTED, "2026-09-28T09:00:00Z", Live, Aligned, vec![posted.clone()]);
        let second = sent(UID, "2026-09-28T09:05:00Z", Live, Aligned, vec![forwarded]);
        let filed = NewNote { mail_asof: Some(first.ordering_date), ..note(&posted, false) };
        let card = Action::TaskCard { claim: first_of(ROUTED), note: filed, proposal: held() };
        assert_eq!(against(vec![], &[second, first], 1), vec![card, skip(SkipReason::AlreadyApplied)], "vault day 1, one plan");
    }

    /// The match is exact on both keys (T5.2's normalisation; the course equal), since a match too
    /// loose drops a real task with no card (spec risk 1): another course, no course, or a title that
    /// only shares words with the card's each file their card, exactly as with no card at all.
    #[test]
    fn another_course_or_another_normalised_title_still_files_its_card() {
        let card = proposing("pending");
        let named = |course: Option<&str>, title: &str| Claim { course: course.map(str::to_string), ..item(title, Some("2026-10-12"), Task) };
        for claim in [
            named(Some("MATH 220"), "Lab 5"),
            named(Some("CS 1010"), "Lab 5"),
            named(None, "Lab 5"),
            named(Some("CS 101"), "Lab 5 report"),
            named(Some("CS 101"), "Pre-lab 5"),
            named(Some("CS 101"), "Lab 6"),
            named(Some("CS 101"), "Lab"),
            named(Some("CS 101"), "Lab 05"),
        ] {
            let forwarded = message(Live, Aligned, vec![claim.clone()]);
            let got = against(vec![card.clone()], &[forwarded.clone()], 9);
            assert_eq!(got, vec![carded(&claim, pending())], "{:?} {:?}", claim.course, claim.item);
            assert_eq!(got, against(vec![], &[forwarded], 9), "as with no card");
        }
        let (courseless, claim) = (NoteView { course: None, ..card.clone() }, named(None, "Lab 5"));
        let got = against(vec![courseless], &[message(Live, Aligned, vec![claim.clone()])], 9);
        assert_eq!(got, vec![carded(&claim, pending())], "no course matches no card, as T5.2's title match");
        let same = named(Some("CS 101"), "lab: 5");
        assert_eq!(against(vec![card], &[message(Live, Aligned, vec![same])], 9), vec![skip(SkipReason::AlreadyApplied)], "the same task");
    }

    /// Only an open mail decision on the task blocks it (PQ8 (ii)): a rejected or expired mail task
    /// card (moved to `archive/`), an approved one, a Gmail task card and a mail amend card, each with
    /// the same title and course, block nothing, alone or together.
    #[test]
    fn only_an_open_mail_task_card_blocks() {
        let open = proposing("pending");
        let gone = |status: &str| NoteView { folder: "archive".to_string(), path: "archive/task-lab-5.md".to_string(), ..proposing(status) };
        let closed = [
            gone("rejected"),
            gone("expired"),
            proposing("approved"),
            NoteView { created_by: Some("gmail".to_string()), source_uid: Some("gmail:18c2f0a1".to_string()), ..open.clone() },
            NoteView { kind: Some("amend".to_string()), path: "approvals/amend-lab-5.md".to_string(), ..open.clone() },
        ];
        let claim = item("Lab 5", Some("2026-10-12"), Task);
        let forwarded = message(Live, Aligned, vec![claim.clone()]);
        let filed = vec![carded(&claim, pending())];
        for card in &closed {
            let got = against(vec![card.clone()], &[forwarded.clone()], 9);
            assert_eq!(got, filed, "{} {:?} {:?} {:?}", card.folder, card.kind, card.status, card.created_by);
        }
        assert_eq!(against(closed.to_vec(), &[forwarded.clone()], 9), filed, "all of them together");
        assert_eq!(against([closed.to_vec(), vec![open]].concat(), &[forwarded], 9), vec![skip(SkipReason::AlreadyApplied)]);
    }

    /// Two live copies of one item by two routes (S0 capture 5): planned whole, the later claim meets
    /// the card the earlier one plans; delivered over two pulls, it meets that card in the vault. One
    /// card either way, for the copy §4.8's order takes first, and the same plan whatever order the
    /// pulls deliver (spec test 18), at two ordering dates or at one.
    #[test]
    fn two_matching_live_items_file_one_card_whole_or_split() {
        let (posted, forwarded) = (item("Lab 5", Some("2026-10-12"), Task), item("lab 5", Some("2026-10-12"), Task));
        let applied = |uid: &str| Action::Skip { claim: first_of(uid), reason: SkipReason::AlreadyApplied };
        for forwarded_at in ["2026-09-28T09:05:00Z", "2026-09-28T09:00:00Z"] {
            let first = sent(ROUTED, "2026-09-28T09:00:00Z", Live, Aligned, vec![posted.clone()]);
            let second = sent(UID, forwarded_at, Live, Aligned, vec![forwarded.clone()]);
            let filed = NewNote { mail_asof: Some(first.ordering_date), ..note(&posted, false) };
            let card = Action::TaskCard { claim: first_of(ROUTED), note: filed, proposal: pending() };
            for delivered in [[first.clone(), second.clone()], [second.clone(), first.clone()]] {
                assert_eq!(against(vec![], &delivered, 9), vec![card.clone(), applied(UID)], "whole, {forwarded_at}");
            }
            // Split: pull one files the card, which syncs into the vault as a card; pull two meets it
            // there, whether or not it delivers the first copy again.
            assert_eq!(against(vec![], &[first.clone()], 9), vec![card], "pull one, {forwarded_at}");
            assert_eq!(against(vec![proposing("pending")], &[second.clone()], 9), vec![applied(UID)], "pull two");
            assert_eq!(against(vec![proposing("pending")], &[second, first], 9), vec![applied(ROUTED), applied(UID)]);
        }
    }

    /// The rule is live only (plan T5.6): with an open mail task card for "Lab 5" in the vault, a
    /// backfill `new_item` for it, aligned or not, takes T5.3's rows exactly as with no card. A
    /// recovered message reads as live (plan T5's rules), so its copy files nothing.
    #[test]
    fn a_backfill_new_item_keeps_its_rows() {
        let card = proposing("pending");
        for claim in [
            item("Lab 5", Some("2026-10-12"), Task),
            item("Lab 5", None, Task),
            item("Lab 5", Some("2026-09-20"), Task),
            item("Lab 5", Some("2026-10-05T18:00"), Event),
            item("Lab 5", Some("2026-09-25T18:00"), Event),
        ] {
            for authenticity in [Aligned, Unaligned] {
                let backfilled = message(Backfill, authenticity, vec![claim.clone()]);
                let alone = against(vec![], &[backfilled.clone()], 9);
                assert_eq!(against(vec![card.clone()], &[backfilled], 9), alone, "{:?} {authenticity:?}", claim.item);
            }
        }
        let recover: Phase = serde_json::from_value(json!("recover")).unwrap();
        let recovered = message(recover, Aligned, vec![item("Lab 5", Some("2026-10-12"), Task)]);
        assert_eq!(against(vec![card], &[recovered], 9), vec![skip(SkipReason::AlreadyApplied)], "recover reads as live");
    }
}
