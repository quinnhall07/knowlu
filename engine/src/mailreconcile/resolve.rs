//! What a claim's target is: §4.8 steps 2 and 4 (email spec 2026-09-30; plan T5.2).
//!
//! Pure, like the rest of the reconciler. [`resolve`] reads one claim against the vault view;
//! [`to_ask`] lists the claims it leaves ambiguous, which `mail.rs` batches per course to
//! `mail-resolve` before [`super::plan`] runs (§4.8 step 3, D12). The answers come back stored on
//! the claims, so a re-pull replays them with no call.
//!
//! **The steps, in order.**
//! 1. The claim's uid (`mail:<h>:<n>`, [`ClaimRef::uid`]) already on a note or card in `tasks/`,
//!    `archive/` or `approvals/`: another desktop wrote it and sync brought it here (§4.8 step 4,
//!    test 22e). Nothing is written.
//! 2. A stored answer wins (D12): a `target_id` naming an item note resolves to it; a stored "none",
//!    or an id naming no item note, is unmatched. Never a guess.
//! 3. A `change`'s `target` (§4.8 step 2): equal to an item note's `source_uid` or LMS id; else equal,
//!    normalised, to the title of an item note in the claim's course; else the one active note in the
//!    course whose title holds every token of it. At each step one match resolves, two or more are
//!    ambiguous and none goes on to the next; none at the last is ambiguous.
//! 4. A `new_item` whose normalised title equals an active note in its course is a change to that
//!    note's `due`, never a duplicate (test 24); two such notes are ambiguous; none is a new note.
//!
//! **Words.** An *item note* is a note in `tasks/` or `archive/`, never a card in `approvals/`. An
//! *active* note is in `tasks/` with `status: active` (no `status:` line reads as active, as
//! `models` reads it). A title is *normalised* by folding case, punctuation and whitespace: its
//! words, split at every character that is not a letter or a digit, lowercased, joined by one space;
//! its *tokens* are those words. A note's *LMS id* is its `source_uid` after the feed's prefix
//! (`zybooks:1839992` is `1839992`); a feed that writes its uid bare (Blackboard's ICS UID) is
//! matched by the `source_uid` itself.

use super::{is_noise, order, Claim, ClaimField, ClaimKind, ClaimRef, Message, NoteView, VaultView};

/// Where a note or card made from a claim sits once written (§4.8 step 4).
const APPLIED_FOLDERS: [&str; 3] = ["tasks", "archive", "approvals"];

/// Where a claim's target may sit: items, never a card.
const ITEM_FOLDERS: [&str; 2] = ["tasks", "archive"];

/// A claim once its target is resolved (§4.8 steps 2 and 4).
#[derive(Debug, Clone, PartialEq)]
pub enum Resolved<'v> {
    /// Its uid is already on a note or card: another desktop's write (§4.8 step 4). Nothing is
    /// written.
    AlreadyApplied,
    /// A `new_item` naming no active note in its course: a new note (§4.9's `new_item` rows).
    New,
    /// A change to one field of `note`: a `change` claim's `field` and `value`, or a `new_item`
    /// read as a change to the note's `due`, to the item's due (test 24).
    Change { note: &'v NoteView, field: ClaimField, value: Option<String> },
    /// Zero deterministic matches, or two or more, and no stored answer: listed by [`to_ask`]
    /// for `mail-resolve` (§4.8 step 3).
    Ambiguous,
    /// A stored "none", a stored id naming no item note (D12), or a claim with nothing to change.
    Unmatched,
    /// `course_info` and `noise`: no target to resolve.
    NoTarget,
}

impl ClaimRef {
    /// `mail:<h>:<n>` (D20): the message's uid and the claim's 0-based index in `claims`, the
    /// `source_uid` a note or card made from this claim carries (§5.2).
    pub fn uid(&self) -> String {
        format!("{}:{}", self.message, self.index)
    }
}

/// Resolve one claim against `view` (§4.8 steps 2 and 4), in the module's order of steps.
pub fn resolve<'v>(view: &'v VaultView, at: &ClaimRef, claim: &Claim) -> Resolved<'v> {
    let uid = at.uid();
    let applied = |n: &NoteView| APPLIED_FOLDERS.contains(&n.folder.as_str()) && n.source_uid.as_deref() == Some(&uid);
    if view.notes.iter().any(applied) {
        return Resolved::AlreadyApplied;
    }
    // What the claim changes, and what names its target. A change with no field, or a new_item
    // with no item, has nothing to change: unmatched, so it is shown and never guessed at.
    let (field, value, named) = match (claim.kind, claim.field, &claim.item) {
        (ClaimKind::Change, Some(field), _) => (field, claim.value.clone(), Named::Target(claim.target.as_deref())),
        (ClaimKind::NewItem, _, Some(item)) => (ClaimField::Due, item.due.clone(), Named::Title(&item.title)),
        (ClaimKind::Change, None, _) | (ClaimKind::NewItem, _, None) => return Resolved::Unmatched,
        (ClaimKind::CourseInfo | ClaimKind::Noise, _, _) => return Resolved::NoTarget,
    };
    let found = match &claim.resolution {
        Some(stored) => match one(items(view).filter(|n| stored.target_id.as_deref() == Some(n.id.as_str()))) {
            Found::One(note) => Found::One(note),
            Found::Zero | Found::Many => return Resolved::Unmatched,
        },
        None => match named {
            Named::Target(target) => by_target(view, claim.course.as_deref(), target),
            Named::Title(title) => by_title(view, claim.course.as_deref(), title),
        },
    };
    match found {
        Found::One(note) => Resolved::Change { note, field, value },
        Found::Zero if claim.kind == ClaimKind::NewItem => Resolved::New,
        Found::Zero | Found::Many => Resolved::Ambiguous,
    }
}

/// The claims [`resolve`] leaves ambiguous, in plan order: what `mail.rs` asks `mail-resolve`
/// about, one batch per course (§4.8 step 3, D12). A noise claim is never asked, and neither is a
/// claim with no course: the call offers a course's candidate items, so with none it could only
/// answer "none".
pub fn to_ask<'m>(view: &VaultView, messages: &'m [Message]) -> Vec<(ClaimRef, &'m Claim)> {
    let mut asks = Vec::new();
    for (message, index) in order(messages) {
        let (claim, at) = (&message.claims[index], ClaimRef { message: message.uid.clone(), index });
        if claim.course.is_some() && !is_noise(message, claim) && resolve(view, &at, claim) == Resolved::Ambiguous {
            asks.push((at, claim));
        }
    }
    asks
}

/// How a claim names its note: a `change`'s `target`, or a `new_item`'s title.
enum Named<'c> {
    Target(Option<&'c str>),
    Title(&'c str),
}

/// How many notes a step found.
enum Found<'v> {
    Zero,
    One(&'v NoteView),
    Many,
}

fn one<'v>(mut matches: impl Iterator<Item = &'v NoteView>) -> Found<'v> {
    match (matches.next(), matches.next()) {
        (None, _) => Found::Zero,
        (Some(note), None) => Found::One(note),
        (Some(_), Some(_)) => Found::Many,
    }
}

/// A `change`'s target: by uid or LMS id, then by normalised title, then by tokens (§4.8 step 2).
fn by_target<'v>(view: &'v VaultView, course: Option<&str>, target: Option<&str>) -> Found<'v> {
    let Some(target) = target.filter(|t| !t.is_empty()) else { return Found::Zero };
    let (title, wanted) = (normalise(target), tokens(target));
    let lms_id = |uid: &str| uid.split_once(':').map(|(_, id)| id == target).unwrap_or(false);
    let by_uid = |n: &NoteView| n.source_uid.as_deref().is_some_and(|uid| uid == target || lms_id(uid));
    let by_title = |n: &NoteView| in_course(n, course) && !title.is_empty() && normalise(&n.title) == title;
    let by_tokens = |n: &NoteView| {
        let held = tokens(&n.title);
        in_course(n, course) && is_active(n) && !wanted.is_empty() && wanted.iter().all(|t| held.contains(t))
    };
    let steps: [&dyn Fn(&NoteView) -> bool; 3] = [&by_uid, &by_title, &by_tokens];
    for step in steps {
        match one(items(view).filter(|n| step(n))) {
            Found::Zero => continue,
            decided => return decided,
        }
    }
    Found::Zero
}

/// A `new_item`'s title against the active notes of its course (§4.8 step 2, test 24).
fn by_title<'v>(view: &'v VaultView, course: Option<&str>, title: &str) -> Found<'v> {
    let title = normalise(title);
    let equal = |n: &&NoteView| in_course(n, course) && is_active(n) && !title.is_empty() && normalise(&n.title) == title;
    one(items(view).filter(equal))
}

fn items(view: &VaultView) -> impl Iterator<Item = &NoteView> {
    view.notes.iter().filter(|n| ITEM_FOLDERS.contains(&n.folder.as_str()))
}

fn in_course(note: &NoteView, course: Option<&str>) -> bool {
    course.is_some() && note.course.as_deref() == course
}

fn is_active(note: &NoteView) -> bool {
    note.folder == "tasks" && note.status.as_deref().unwrap_or("active") == "active"
}

/// A title's words: split at every character that is not a letter or a digit, lowercased.
fn tokens(text: &str) -> Vec<String> {
    text.split(|c: char| !c.is_alphanumeric()).filter(|w| !w.is_empty()).map(str::to_lowercase).collect()
}

/// A title with case, punctuation and whitespace folded (§4.8 step 2).
fn normalise(text: &str) -> String {
    tokens(text).join(" ")
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::mailreconcile::{plan, Action, Authenticity, ClaimSource, ItemKind, NewItem, Phase, Resolution, Sender, SkipReason};
    use serde_json::json;
    use std::collections::{BTreeMap, BTreeSet};

    const UID: &str = "mail:9a8b7c6d5e4f30211203f4e5d6c7b8a9";

    /// A note in `folder`: active in `tasks/`, done in `archive/`, a pending card in `approvals/`.
    fn note(id: &str, folder: &str, course: &str, title: &str, source_uid: Option<&str>) -> NoteView {
        let status = match folder {
            "tasks" => "active",
            "archive" => "done",
            _ => "pending",
        };
        NoteView {
            id: id.to_string(),
            path: format!("{folder}/{id}.md"),
            folder: folder.to_string(),
            course: Some(course.to_string()),
            title: title.to_string(),
            due: json!("2026-10-08"),
            status: Some(status.to_string()),
            source_uid: source_uid.map(str::to_string),
            created_by: None,
            mail_asof: BTreeMap::new(),
            human_set: BTreeSet::new(),
        }
    }

    fn view(notes: Vec<NoteView>) -> VaultView {
        VaultView { notes }
    }

    fn change(course: Option<&str>, target: Option<&str>) -> Claim {
        Claim {
            kind: ClaimKind::Change,
            course: course.map(str::to_string),
            target: target.map(str::to_string),
            field: Some(ClaimField::Due),
            value: Some("2026-10-09".to_string()),
            item: None,
            topic: None,
            evidence: "now due Friday".to_string(),
            why: "The due date moved".to_string(),
            confidence: 0.9,
            source: ClaimSource::Model,
            resolution: None,
        }
    }

    fn new_item(course: &str, title: &str) -> Claim {
        let item = NewItem {
            title: title.to_string(),
            due: Some("2026-10-12".to_string()),
            item_kind: ItemKind::Task,
            effort_hours: 2.0,
            importance: 3,
        };
        let kind = ClaimKind::NewItem;
        Claim { kind, target: None, field: None, value: None, item: Some(item), ..change(Some(course), None) }
    }

    fn stored(claim: Claim, target_id: Option<&str>) -> Claim {
        Claim { resolution: Some(Resolution { target_id: target_id.map(str::to_string) }), ..claim }
    }

    fn at(index: usize) -> ClaimRef {
        ClaimRef { message: UID.to_string(), index }
    }

    fn message(claims: Vec<Claim>) -> Message {
        Message {
            uid: UID.to_string(),
            ordering_date: "2026-09-28T09:00:00Z".parse().unwrap(),
            message_date: None,
            phase: Phase::Live,
            authenticity: Authenticity::Aligned,
            sender: Sender { name: Some("Course Staff".to_string()), address: "staff@example.edu".to_string() },
            claims,
        }
    }

    /// The change [`change`]'s claim asks for, on `note`.
    fn moved(note: &NoteView) -> Resolved<'_> {
        Resolved::Change { note, field: ClaimField::Due, value: Some("2026-10-09".to_string()) }
    }

    fn asked(view: &VaultView, claims: Vec<Claim>) -> Vec<usize> {
        to_ask(view, &[message(claims)]).into_iter().map(|(at, _)| at.index).collect()
    }

    /// §4.8 step 2, first clause: the uid is the external key, so it matches in any course.
    #[test]
    fn a_target_equal_to_a_source_uid_or_lms_id_resolves() {
        let view = view(vec![
            note("task-zy", "tasks", "CS 101", "Participation 3.1", Some("zybooks:1839992")),
            note("task-bb", "tasks", "CS 101", "HW 01", Some("_901_1")),
            note("card-zy", "approvals", "CS 101", "Mark done: Participation 3.1", Some("zybooks:1839992")),
        ]);
        for (target, want) in [("zybooks:1839992", 0), ("1839992", 0), ("_901_1", 1)] {
            for course in [Some("CS 101"), Some("MATH 220"), None] {
                let got = resolve(&view, &at(0), &change(course, Some(target)));
                assert_eq!(got, moved(&view.notes[want]), "{target} in {course:?}; a card is never the target");
            }
        }
    }

    /// §4.8 step 2, second clause: case, punctuation and whitespace folded, in the claim's course.
    #[test]
    fn a_normalised_title_in_the_claims_course_resolves() {
        let view = view(vec![
            note("task-hw", "tasks", "CS 101", "HW 01: Linked Lists", Some("_901_1")),
            note("task-math", "tasks", "MATH 220", "HW 01: Linked Lists", None),
            note("task-quiz", "archive", "CS 101", "Quiz 3", None),
            note("task-review", "tasks", "CS 101", "Quiz 3 review", None),
        ]);
        let ask = |course: Option<&str>, target: &str| resolve(&view, &at(0), &change(course, Some(target)));
        for target in ["HW 01: Linked Lists", "hw-01 linked   LISTS!", " HW 01 \u{2014} linked lists. "] {
            assert_eq!(ask(Some("CS 101"), target), moved(&view.notes[0]), "{target:?}");
            assert_eq!(ask(Some("MATH 220"), target), moved(&view.notes[1]), "{target:?} in MATH 220");
        }
        // Equality comes before tokens and counts any item note: "Quiz 3" is the archived quiz,
        // never the active review that holds its tokens.
        assert_eq!(ask(Some("CS 101"), "quiz 3"), moved(&view.notes[2]));
        assert_eq!(ask(None, "HW 01: Linked Lists"), Resolved::Ambiguous, "no course, no title in it");
    }

    /// §4.8 step 2, third clause: tokens, not substrings, and only an active note.
    #[test]
    fn the_one_active_note_holding_every_token_resolves() {
        let mut finished = note("task-peer", "tasks", "CS 101", "Essay 2 peer review", None);
        finished.status = Some("done".to_string());
        let view = view(vec![
            note("task-essay", "tasks", "CS 101", "Essay 2: Final draft", None),
            note("task-outline", "archive", "CS 101", "Essay 2 outline", None),
            note("task-other", "tasks", "MATH 220", "Essay 2 reflection", None),
            note("task-essay-1", "tasks", "CS 101", "Essay 1: Final draft", None),
            finished,
        ]);
        let ask = |target: &str| resolve(&view, &at(0), &change(Some("CS 101"), Some(target)));
        assert_eq!(ask("essay 2"), moved(&view.notes[0]));
        assert_eq!(ask("FINAL DRAFT, essay 2"), moved(&view.notes[0]));
        assert_eq!(ask("Essay 2 drafts"), Resolved::Ambiguous, "`drafts` is a token no title holds");
    }

    #[test]
    fn zero_or_two_matches_are_ambiguous() {
        let view = view(vec![
            note("task-e1", "tasks", "CS 101", "Essay 1", Some("zybooks:4410")),
            note("task-e2", "tasks", "CS 101", "Essay 2", Some("vhl:4410")),
            note("task-lab", "tasks", "CS 101", "Lab 4", None),
            note("task-lab-old", "archive", "CS 101", "lab-4", None),
        ]);
        let ask = |target: Option<&str>| resolve(&view, &at(0), &change(Some("CS 101"), target));
        assert_eq!(ask(Some("4410")), Resolved::Ambiguous, "two LMS ids equal it");
        assert_eq!(ask(Some("Lab 4")), Resolved::Ambiguous, "two titles equal it, normalised");
        assert_eq!(ask(Some("Essay")), Resolved::Ambiguous, "two active notes hold every token");
        assert_eq!(ask(Some("Project proposal")), Resolved::Ambiguous, "nothing matches");
        assert_eq!(ask(Some(" \u{2014} ")), Resolved::Ambiguous, "no token to match");
        assert_eq!(ask(Some("")), Resolved::Ambiguous, "an empty target");
        assert_eq!(ask(None), Resolved::Ambiguous, "no target");
        // Listed for mail-resolve in plan order: noise never, a resolved claim never, and a claim
        // with no course never (the call offers a course's candidates, D12).
        let noise = Claim { kind: ClaimKind::Noise, ..change(Some("CS 101"), Some("Essay")) };
        let claims = vec![
            change(Some("CS 101"), Some("Essay")),
            noise,
            change(None, Some("Essay")),
            change(Some("CS 101"), Some("Essay 2")),
            change(Some("CS 101"), Some("Lab 4")),
        ];
        assert_eq!(asked(&view, claims), vec![0, 4]);
    }

    /// D12: the model's answer is data. It decides the target whatever the vault now matches, a
    /// stored "none" is unmatched, and a claim carrying an answer is never asked again.
    #[test]
    fn a_stored_resolution_wins_and_a_stored_none_is_unmatched() {
        let view = view(vec![
            note("task-e1", "tasks", "CS 101", "Essay 1", None),
            note("task-e2", "tasks", "CS 101", "Essay 2", None),
            note("card-e2", "approvals", "CS 101", "Essay 2", Some("mail:1f1f:0")),
        ]);
        let essay_1 = change(Some("CS 101"), Some("Essay 1"));
        assert_eq!(resolve(&view, &at(0), &essay_1), moved(&view.notes[0]), "unanswered, it resolves itself");
        assert_eq!(resolve(&view, &at(0), &stored(essay_1.clone(), Some("task-e2"))), moved(&view.notes[1]));
        assert_eq!(resolve(&view, &at(0), &stored(essay_1.clone(), None)), Resolved::Unmatched, "a stored none");
        assert_eq!(resolve(&view, &at(0), &stored(essay_1.clone(), Some("task-gone"))), Resolved::Unmatched);
        assert_eq!(resolve(&view, &at(0), &stored(essay_1, Some("card-e2"))), Resolved::Unmatched, "a card is no item");
        let due = Resolved::Change { note: &view.notes[1], field: ClaimField::Due, value: Some("2026-10-12".to_string()) };
        assert_eq!(resolve(&view, &at(0), &stored(new_item("CS 101", "Essay"), Some("task-e2"))), due, "a new_item's answer");
        let essay = change(Some("CS 101"), Some("Essay"));
        assert_eq!(asked(&view, vec![essay.clone()]), vec![0], "unanswered and ambiguous: asked");
        assert_eq!(asked(&view, vec![stored(essay.clone(), Some("task-e1")), stored(essay, None)]), Vec::<usize>::new());
    }

    /// Spec test 24: one active note in the course with an equal normalised title makes a `new_item`
    /// a change to that note's `due`, never a duplicate.
    #[test]
    fn a_new_item_whose_title_equals_an_active_note_is_a_change_to_its_due() {
        let view = view(vec![
            note("task-essay", "tasks", "CS 101", "Essay 2", Some("_902_1")),
            note("task-quiz", "archive", "CS 101", "Quiz 3", None),
            note("task-lab", "tasks", "CS 101", "Lab 4: Recursion", None),
            note("task-hw", "tasks", "CS 101", "HW 5", None),
            note("task-hw-copy", "tasks", "CS 101", "hw-5", None),
        ]);
        let ask = |course: &str, title: &str| resolve(&view, &at(0), &new_item(course, title));
        let due = Resolved::Change { note: &view.notes[0], field: ClaimField::Due, value: Some("2026-10-12".to_string()) };
        assert_eq!(ask("CS 101", "ESSAY #2"), due);
        let mut undated = new_item("CS 101", "essay 2");
        undated.item.as_mut().unwrap().due = None;
        let to_none = Resolved::Change { note: &view.notes[0], field: ClaimField::Due, value: None };
        assert_eq!(resolve(&view, &at(0), &undated), to_none, "the item's due, even none");
        assert_eq!(ask("MATH 220", "Essay 2"), Resolved::New, "another course");
        assert_eq!(ask("CS 101", "Quiz 3"), Resolved::New, "an archived note is not active (step 2)");
        assert_eq!(ask("CS 101", "Lab 4"), Resolved::New, "equal titles only, never tokens");
        assert_eq!(ask("CS 101", "HW 5"), Resolved::Ambiguous, "two active notes equal it");
    }

    /// Spec test 22e, the pure half: a note or card carrying the claim's own uid was written by
    /// another desktop, so the claim writes nothing, whatever it would otherwise resolve to.
    #[test]
    fn a_claim_whose_source_uid_is_already_on_a_note_or_card_writes_nothing() {
        assert_eq!(at(3).uid(), format!("{UID}:3"), "mail:<h>:<n>, n the claim's 0-based index");
        let claims = vec![new_item("CS 101", "Essay 3"), change(Some("CS 101"), Some("Essay 2"))];
        let today = jiff::civil::date(2026, 10, 1);
        for folder in ["tasks", "archive", "approvals"] {
            for index in 0..claims.len() {
                let uid = at(index).uid();
                let synced = note("synced", folder, "CS 101", "Essay 3", Some(uid.as_str()));
                let view = view(vec![note("task-e2", "tasks", "CS 101", "Essay 2", None), synced]);
                assert_eq!(resolve(&view, &at(index), &claims[index]), Resolved::AlreadyApplied, "{folder}, {index}");
                let planned = plan(&view, &[message(claims.clone())], today, 9);
                let mine: Vec<&Action> = planned.iter().filter(|a| a.claim().index == index).collect();
                let skip = Action::Skip { claim: at(index), reason: SkipReason::AlreadyApplied };
                assert_eq!(mine, vec![&skip], "{folder}, claim {index}: nothing else is planned");
            }
        }
        // Checked before resolution: an applied claim that would be ambiguous is never asked about.
        let essays = vec![note("task-e1", "tasks", "CS 101", "Essay 1", None), note("task-e2", "tasks", "CS 101", "Essay 2", None)];
        let ambiguous = vec![change(Some("CS 101"), Some("Essay"))];
        assert_eq!(asked(&view(essays.clone()), ambiguous.clone()), vec![0]);
        let card = note("card", "approvals", "CS 101", "Essay 2", Some(at(0).uid().as_str()));
        assert_eq!(asked(&view([essays, vec![card]].concat()), ambiguous), Vec::<usize>::new());
        // Another claim's uid, or the uid on a note outside those folders, is not this claim's.
        let view = view(vec![
            note("task-e2", "tasks", "CS 101", "Essay 2", Some(format!("{UID}:7").as_str())),
            note("info-1", "info", "CS 101", "Essay 2 moved", Some(format!("{UID}:1").as_str())),
        ]);
        assert_eq!(resolve(&view, &at(1), &claims[1]), moved(&view.notes[0]));
        assert_eq!(resolve(&view, &at(0), &claims[0]), Resolved::New);
    }
}
