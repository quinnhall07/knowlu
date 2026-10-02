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
//! **Past** is a due whose date is strictly before `today`, never before *now*: the module has no
//! clock, and `ingest` and `coursework` archive `imported-past` by the same rule, since an item due
//! at 23:59 today is today's work.
//!
//! **The first day (§4.8 step 5, plan §10 R16).** On vault day 1 a card is held to day 2: written
//! `status: snoozed` with `proposed_at` and `snooze_until` both day 2, so it charges day 2's budget
//! and shows from day 2, and `first_proposed_at` day 1, the day it was filed
//! ([`proposal`]). `approvals::transition_note` wakes only a `snoozed` card, so a `pending` one would
//! show on day 1. Direct writes and notices are not held.

use jiff::civil::Date;

use super::resolve::Resolved;
use super::{Action, Authenticity, CardStatus, Claim, ClaimRef, ItemKind, Message, NewItem, NewNote, Phase, Proposal, SkipReason};

/// The actions one claim plans, from its resolution (§4.9). T5.3 holds the `new_item` rows; every
/// `change`, `course_info` and unresolved claim is unmatched until T5.4 and T5.5 fill in their rows.
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
        (Resolved::AlreadyApplied, _) => skip(at, SkipReason::AlreadyApplied),
        (Resolved::Restated { .. }, _) => skip(at, SkipReason::Equal),
        // `resolve` gives `New` only with an item; the rest wait on T5.4 and T5.5.
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
}
