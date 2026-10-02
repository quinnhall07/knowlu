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
//! A change to a note this plan files waits on the open plan amendment (plan §9.5).
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
    Action, Authenticity, CardStatus, Change, Claim, ClaimField, ClaimRef, Claimed, ItemKind, Message, NewItem, NewNote,
    NoteView, Phase, Proposal, Sender, SkipReason, Step,
};
use crate::approvals::AMENDABLE_FOLDERS;
use crate::write;

/// The `created_by` of a note the mail pass made (D20): the only note mail may write directly.
const MAIL: &str = "mail";

/// The actions one claim plans, from its resolution (§4.9). T5.3 holds the `new_item` rows and
/// T5.4 the `change` rows; a `course_info` and an unresolved claim are unmatched until T5.5 fills
/// in their rows, and so is a change to a note this plan files ([`Target::Planned`]) until the
/// open plan amendment says how an action names it.
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
        (Resolved::New, Some(item)) => vec![new_item(message, at, claim, item, today, vault_day)],
        (Resolved::Change { target: Target::Note(note), field, value }, _) => {
            vec![change(message, at, claim, note, *field, value.as_deref(), today, proposal(today, vault_day))]
        }
        (Resolved::AlreadyApplied, _) => skip(at, SkipReason::AlreadyApplied),
        (Resolved::Restated { .. }, _) => skip(at, SkipReason::Equal),
        // `resolve` gives `New` only with an item; the rest wait on T5.5 and the amendment.
        _ => skip(at, SkipReason::Unmatched),
    }
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

/// §4.9's `change` rows 5, 6, 10 and 11 for one claim against one vault note (D11 (1) and (2)),
/// `card` the place a card takes in the deck.
///
/// - **Nothing changes, nothing is written** ([`SkipReason::Equal`]): a due naming no known date
///   ("no date known", never "date removed"), the value the note holds in the vault's literal form,
///   or a `cancelled` for a note already done or archived (out of the plan already).
/// - **Direct** only for `due` on a mail note (`created_by: mail`) whose `due` the view's
///   `human_set` does not hold, from a message that [`writes_direct`]: row 5's backfill column.
///   `mail.rs`'s direct-set writer re-reads the journal first, and may only turn this into a card.
///   On a note outside `tasks/` (archived `imported-past`) only a due still before `today` is set:
///   a later one would leave the plan holding an archived item rank never shows (spec risk 1,
///   T5.3's past). It takes the card row instead, and no card can amend outside `tasks/`, so it
///   skips as [`SkipReason::NotAmendable`] until the open ruling (plan §9.5).
/// - **Otherwise a `kind: amend` card**: row 5 live (and an unaligned backfill, §4.6 step 6); row
///   6, a field the student set, a hand-made note's included, in both phases; row 10, a Gmail or
///   events note, in both phases; and row 11, `cancelled`, always, proposing `status: active →
///   archived` and no new field. Every other note, a feed's included, is a card here too: row 7
///   for a model claim; T5.5's template rows 8 and 9 refine it for `source: template`.
/// - **A card `approvals::validate_amendment` would refuse is never planned** (D11 (3)): a note
///   outside `tasks/`, or a null `from`, skips as [`SkipReason::NotAmendable`].
#[allow(clippy::too_many_arguments)]
fn change(
    message: &Message,
    at: ClaimRef,
    claim: &Claim,
    note: &NoteView,
    field: ClaimField,
    value: Option<&str>,
    today: Date,
    card: Proposal,
) -> Action {
    let skip = |claim, reason| Action::Skip { claim, reason };
    let (name, from, to) = match field {
        ClaimField::Cancelled => match note.status.as_deref() {
            Some("done" | "archived") => return skip(at, SkipReason::Equal),
            status => ("status", status.map_or(Value::Null, |s| Value::String(s.to_string())), Value::String("archived".to_string())),
        },
        ClaimField::Due => match value.filter(|due| due_date(due).is_some()) {
            Some(due) => ("due", note.due.clone(), Value::String(due.to_string())),
            None => return skip(at, SkipReason::Equal),
        },
    };
    if literal(&from) == literal(&to) {
        return skip(at, SkipReason::Equal);
    }
    let mails = note.created_by.as_deref() == Some(MAIL) && !note.human_set.contains(name);
    let stays_past = || to.as_str().and_then(due_date).is_some_and(|due| due < today);
    if field == ClaimField::Due && mails && writes_direct(message) && (note.folder == "tasks" || stays_past()) {
        let (target, field, mail_asof) = (note.path.clone(), name.to_string(), message.ordering_date);
        Action::Set { claim: at, target, field, to, mail_asof, body_line: body_line(message, claim) }
    } else if amendable(note, &from) {
        let changes = vec![Change { field: name.to_string(), from, to }];
        Action::AmendCard { claim: at, target: note.path.clone(), changes, proposal: card, also: Vec::new() }
    } else {
        skip(at, SkipReason::NotAmendable)
    }
}

/// A value in the vault's literal form (`write::to_literal`, through `yaml::from_json` as
/// `verify_tail` re-applies a journalled value), so a date and a date-time never compare equal.
fn literal(value: &Value) -> String {
    write::to_literal(&crate::yaml::from_json(value))
}

/// Whether `approvals::validate_amendment` accepts a card against `note` amending from `from`: the
/// note in an amendable folder (`tasks/`, for an item) and `from` a non-null scalar.
fn amendable(note: &NoteView, from: &Value) -> bool {
    AMENDABLE_FOLDERS.contains(&note.folder.as_str()) && !matches!(from, Value::Null | Value::Array(_) | Value::Object(_))
}

/// §5.2's body line for a direct change: `<date> · <sender>: <why> — "<evidence>"`. The date is the
/// message's Date header, else its ordering date, as a UTC day (the module has no time zone, and
/// the journal's days are UTC); the sender is its display name and address (D21).
fn body_line(message: &Message, claim: &Claim) -> String {
    let date = message.message_date.unwrap_or(message.ordering_date).strftime("%Y-%m-%d");
    format!("{date} \u{b7} {}: {} \u{2014} \"{}\"", sender(&message.sender), claim.why, claim.evidence)
}

/// A sender as D21 names it: `Name <address>`, or the address alone when it has no display name.
fn sender(sender: &Sender) -> String {
    match sender.name.as_deref().map(str::trim).filter(|name| !name.is_empty()) {
        Some(name) => format!("{name} <{}>", sender.address),
        None => sender.address.clone(),
    }
}

/// What one claim says a vault note's field should be, the key §4.9's two-value row groups by.
/// Only a known due is ever said: a `cancelled` always says `"true"`, so two never differ.
#[derive(Debug, Clone, PartialEq)]
pub(super) struct Said {
    path: String,
    field: &'static str,
    value: String,
}

/// What `resolved` says, when it is a change to a vault note's due naming a known date.
pub(super) fn said(resolved: &Resolved<'_>) -> Option<Said> {
    match resolved {
        Resolved::Change { target: Target::Note(note), field: ClaimField::Due, value: Some(value) } if due_date(value).is_some() => {
            Some(Said { path: note.path.clone(), field: "due", value: value.clone() })
        }
        _ => None,
    }
}

/// §4.9's row 12 and P20, over one run of claims sharing an ordering date (`super::walk`): claims
/// saying different values for one vault note's field make one `kind: amend` card, in both phases,
/// whatever each alone planned. The card proposes, from the value the note held before the run
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
/// For T5.5: a member's own direct set stamps `mail_asof` at the run's date, so per-claim
/// supersession would skip the next member; this rewrite replaces both. A group no later than the
/// note's `mail_asof` from before the run is a different case, superseded whole, which T5.5's
/// supersession must keep here.
pub(super) fn settle(before: &Working, run: &mut [Step<'_>], today: Date, vault_day: i64) -> bool {
    let mut groups: BTreeMap<(String, &'static str), Vec<usize>> = BTreeMap::new();
    for (i, step) in run.iter().enumerate() {
        if let Some(said) = &step.said {
            groups.entry((said.path.clone(), said.field)).or_default().push(i);
        }
    }
    let mut rewrote = false;
    for ((path, field), steps) in groups {
        let (mut members, mut copies): (Vec<usize>, Vec<usize>) = (Vec::new(), Vec::new());
        for i in steps {
            let copy = members.iter().any(|&m| run[m].at == run[i].at);
            if copy { copies.push(i) } else { members.push(i) }
        }
        let value = |i: usize| run[i].said.as_ref().map_or(Value::Null, |said| Value::String(said.value.clone()));
        let values: BTreeSet<String> = members.iter().map(|&i| literal(&value(i))).collect();
        let Some(note) = before.note_at(&path) else { continue };
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
        } else if amendable(note, &note.due) {
            let named = members.iter().filter(|&&i| i != proposer);
            let also = named.map(|&i| Claimed { claim: run[i].at.clone(), value: value(i), sender: run[i].message.sender.clone() });
            let changes = vec![Change { field: field.to_string(), from: note.due.clone(), to: value(proposer) }];
            let (target, proposal) = (path.clone(), proposal(today, vault_day));
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

/// A mail card's place in the deck on `today` (§4.8 step 5, plan §10 R16): held to day 2 on the
/// vault's first day, else `pending` today. `commitments::vault_day` is never below 1; a lower
/// number is read as the first day, so a card is held rather than shown early.
pub(super) fn proposal(today: Date, vault_day: i64) -> Proposal {
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
    use crate::mailreconcile::{plan, ClaimKind, ClaimSource, Sender, VaultView};
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
            course: Some("CS 101".to_string()),
            title: "Lab 5".to_string(),
            due: json!("2026-10-12"),
            status: Some("active".to_string()),
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
        let (target, field, to) = (PATH.to_string(), "due".to_string(), json!(to));
        Action::Set { claim, target, field, to, mail_asof, body_line: body_line.to_string() }
    }

    fn amend(field: &str, from: Value, to: Value, proposal: Proposal) -> Action {
        let changes = vec![Change { field: field.to_string(), from, to }];
        Action::AmendCard { claim: at(), target: PATH.to_string(), changes, proposal, also: vec![] }
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
            Action::AmendCard { claim, target: PATH.to_string(), changes, proposal: pending(), also }
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
    /// which refuses a target outside `tasks/` and a null `from`. A card row whose card it would
    /// refuse files nothing and the note keeps its value (plan §10 R14's rule for the feed's card);
    /// where the row writes directly, the direct write needs neither.
    #[test]
    fn a_card_the_approvals_executor_would_refuse_is_never_planned() {
        let not_amendable = skip(SkipReason::NotAmendable);
        let (folder, path, status) = ("archive".to_string(), "archive/lab-5.md".to_string(), Some("done".to_string()));
        let archived = NoteView { folder, path, status, ..lab(Some("mail"), &[]) };
        let undated = NoteView { due: Value::Null, ..lab(Some("mail"), &[]) };
        // The archived note in every phase: a due today or later is never set directly outside
        // `tasks/` either (spec risk 1; `a_backfill_never_sets_an_archived_mail_note_…`).
        for (note, phases) in [(&archived, &PHASES[..]), (&undated, &PHASES[1..])] {
            for &(phase, authenticity) in phases {
                assert_eq!(on(note, phase, authenticity, &moved(Some(MOVED)), 9), not_amendable, "{} {phase:?}", note.path);
            }
        }
        assert_eq!(on(&undated, Backfill, Aligned, &moved(Some(MOVED)), 9), set(MOVED, LINE));
        // A hand-made note created with `due: null` is the student's: a card row in both phases, and
        // a card that cannot be filed.
        let hand = NoteView { due: Value::Null, ..lab(Some("student"), &["title", "due", "status"]) };
        for (phase, authenticity) in PHASES {
            assert_eq!(on(&hand, phase, authenticity, &moved(Some(MOVED)), 9), not_amendable, "{phase:?}");
        }
        // `cancelled` on an active note with no `status:` line: `from` would be null.
        let no_status = NoteView { status: None, ..lab(Some("mail"), &[]) };
        assert_eq!(on(&no_status, Live, Aligned, &cancelled(), 9), not_amendable);
        // Row 12 on a note no card can amend: nothing is filed, and neither value is set.
        let a = Message { uid: "mail:aaaa".to_string(), ..message(Backfill, Aligned, vec![moved(Some(MOVED))]) };
        let b = Message { uid: "mail:bbbb".to_string(), ..message(Backfill, Aligned, vec![moved(Some("2026-10-16"))]) };
        let (first, last) = (ClaimRef { message: a.uid.clone(), index: 0 }, ClaimRef { message: b.uid.clone(), index: 0 });
        let want = vec![Action::Skip { claim: first, reason: SkipReason::Superseded }, Action::Skip { claim: last, reason: SkipReason::NotAmendable }];
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
            let card = Action::AmendCard { claim: at("mail:bbbb"), target: PATH.to_string(), changes, proposal: pending(), also };
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
            let card = Action::AmendCard { claim: at(0), target: PATH.to_string(), changes, proposal: pending(), also };
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
    /// more than the same batch planned whole (D10). A due still before today is set.
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
            due: json!("2026-09-28"),
            source_uid: Some(at().uid()),
            mail_asof: BTreeMap::from([("due".to_string(), ordering())]),
            ..lab(Some("mail"), &[])
        };
        let not_amendable = Action::Skip { claim: later_at.clone(), reason: SkipReason::NotAmendable };
        for due in ["2026-10-05", "2026-10-01", "2026-10-01T08:00"] {
            assert_eq!(against(vec![archived.clone()], &[later(vec![moved(Some(due))])], 9), vec![not_amendable.clone()], "{due}");
        }
        let writes = |actions: Vec<Action>| actions.into_iter().filter(|a| !matches!(a, Action::Skip { .. })).collect::<Vec<_>>();
        let slot_two = against(vec![archived.clone()], &[later(vec![extended.clone()])], 9);
        let whole = against(vec![], &[message(Backfill, Aligned, vec![first.clone()]), later(vec![extended])], 9);
        assert_eq!(writes(whole), writes([slot_one, slot_two].concat()), "split or whole, the same writes");
        let still_past = against(vec![archived.clone()], &[later(vec![moved(Some("2026-09-30"))])], 9);
        let set_there = matches!(&still_past[..], [Action::Set { target, to, .. }] if target == "archive/lab-5.md" && to == &json!("2026-09-30"));
        assert!(set_there, "{still_past:?}");
    }

    /// D10 and §4.8 step 4 across two desktops: one aligned backfill message creates "Lab 5" due
    /// 2026-10-12 and moves it to 2026-10-14. The desktop that plans it first creates the note (the
    /// move to a note this plan files waits on the planned-note amendment, plan §9.5); the other,
    /// once that note has synced, writes nothing more, never a card from 10-12 to 10-14.
    #[test]
    fn a_run_another_desktop_planned_writes_nothing_more_after_sync() {
        let lab_5 = item("Lab 5", Some("2026-10-12"), Task);
        let both = message(Backfill, Aligned, vec![lab_5.clone(), moved(Some(MOVED))]);
        let first = against(vec![], &[both.clone()], 9);
        assert_eq!(first.first(), Some(&created(&lab_5, false)), "{first:?}");
        let mail_asof = BTreeMap::from([("due".to_string(), ordering())]);
        let synced = NoteView { source_uid: Some(at().uid()), mail_asof, ..lab(Some("mail"), &[]) };
        let second_at = ClaimRef { message: UID.to_string(), index: 1 };
        let want = vec![skip(SkipReason::AlreadyApplied), Action::Skip { claim: second_at, reason: SkipReason::Superseded }];
        assert_eq!(against(vec![synced], &[both], 9), want);
    }
}
