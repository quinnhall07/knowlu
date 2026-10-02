//! What a claim's target is: §4.8 steps 2 and 4 (email spec 2026-09-30; plan T5.2).
//!
//! Pure, like the rest of the reconciler. [`Working::resolve`] reads one claim against the vault as
//! the plan leaves it so far; [`to_ask`] lists the claims the plan leaves ambiguous, which `mail.rs`
//! batches per course to `mail-resolve` before [`super::plan`] runs (§4.8 step 3, D12). The answers
//! come back stored on the claims, so a re-pull replays them with no call.
//!
//! **Against the vault the earlier claims leave (D10).** A batch is applied whole, so each claim is
//! resolved against a [`Working`] vault: the view, plus every note or card an earlier claim's action
//! files and every field one sets ([`Working::apply`]). A batch so plans as its claims would one pull
//! at a time: a due that mail moves later in the batch that created the item reaches that item, and a
//! second `new_item` naming it is a change, never a second note (test 24). [`super::plan`] and
//! [`to_ask`] take the same walk, so what is asked is what the plan finds ambiguous; an answer never
//! changes what a later claim resolves to, since an answered claim's actions only set a field or file
//! a card under its own uid, neither of which resolution reads, and only a `New` claim adds an item
//! (T5.3–T5.5's rules keep it so).
//!
//! **The steps, in order.**
//! 1. The claim's uid (`mail:<h>:<n>`, [`ClaimRef::uid`]) already on a note or card in `tasks/`,
//!    `archive/` or `approvals/`: another desktop wrote it and sync brought it here (§4.8 step 4,
//!    test 22e), or an earlier copy of the same message in this plan filed it. Nothing is written.
//! 2. A stored answer wins (D12): a `target_id` naming an item note in the vault resolves to it; a
//!    stored "none", or an id naming none, is unmatched. Never a guess. (The call offers the vault's
//!    notes, so an answer never names a note this plan has yet to write.)
//! 3. A `change`'s `target` (§4.8 step 2): equal to an item note's `source_uid` or LMS id; else equal,
//!    normalised, to the title of an item note in the claim's course; else the one active note in the
//!    course whose title holds every token of it. At each step one match resolves, two or more are
//!    ambiguous and none goes on to the next; none at the last is ambiguous.
//! 4. A `new_item` whose normalised title equals an active note in its course is a change to that
//!    note's `due`, never a duplicate (test 24); two such notes are ambiguous; none is a new note.
//!    One with no due is [`Resolved::Restated`] and changes nothing: no due means "no date known",
//!    never "date removed" (spec risk 1; §4.8 step 2 is silent on it, so it waits on Quinn's ruling).
//!    A due naming no real day is no due ([`super::NewItem::known_due`]), so never a change to it.
//!
//! **Words.** An *item note* is a note in `tasks/` or `archive/`, never a card in `approvals/`, in
//! the vault or *planned* (filed by an earlier claim's action in this plan, [`Target::Planned`]). An
//! *active* note is in `tasks/` with `status: active` (no `status:` line reads as active, as
//! `models` reads it). A title is *normalised* by folding case, punctuation and whitespace: its
//! words, split at every character that is not a letter or a digit, lowercased, joined by one space;
//! its *tokens* are those words. A note's *LMS id* is its `source_uid` after the feed's prefix
//! (`zybooks:1839992` is `1839992`); a feed that writes its uid bare (Blackboard's ICS UID) is
//! matched by the `source_uid` itself.

use std::collections::{BTreeMap, BTreeSet};

use jiff::civil::Date;
use serde_json::Value;

use super::{Action, CardStatus, Claim, ClaimField, ClaimKind, ClaimRef, Message, NoteView, Place, VaultView};

/// Where a note or card made from a claim sits once written (§4.8 step 4).
const APPLIED_FOLDERS: [&str; 3] = ["tasks", "archive", "approvals"];

/// Where a claim's target may sit: items, never a card.
const ITEM_FOLDERS: [&str; 2] = ["tasks", "archive"];

/// A claim once its target is resolved (§4.8 steps 2 and 4).
#[derive(Debug, Clone, PartialEq)]
pub enum Resolved<'v> {
    /// Its uid is already on a note or card: another desktop's write, or the same message delivered
    /// twice in one plan (§4.8 step 4). Nothing is written.
    AlreadyApplied,
    /// A `new_item` naming no active note in its course: a new note (§4.9's `new_item` rows).
    New,
    /// A change to one field of `target`: a `change` claim's `field` and `value`, or a dated
    /// `new_item` read as a change to the note's `due`, to the item's due (test 24).
    Change { target: Target<'v>, field: ClaimField, value: Option<String> },
    /// A `new_item` with no due naming `target`, an active note in its course: the mail restates an
    /// item the vault holds and knows no date for it, so nothing changes and nothing is duplicated.
    Restated { target: Target<'v> },
    /// Zero deterministic matches, or two or more, and no stored answer: listed by [`to_ask`]
    /// for `mail-resolve` (§4.8 step 3).
    Ambiguous,
    /// A stored "none", a stored id naming no item note (D12), or a claim with nothing to change.
    Unmatched,
    /// `course_info` and `noise`: no target to resolve.
    NoTarget,
}

/// The note a claim resolves to.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Target<'v> {
    /// A note in the vault: an action names it by its `path`.
    Note(&'v NoteView),
    /// A note the `Create` of an earlier claim in this plan (`by`) files, not written yet: its `id`
    /// and `path` are empty, so an action names it as [`Place::Planned`]`(by)` (plan §9.5's
    /// planned-note amendment). The rules read it as the mail note it will be: `created_by: mail`,
    /// no field a human set.
    Planned { by: &'v ClaimRef, note: &'v NoteView },
}

impl<'v> Target<'v> {
    /// The note as the plan leaves it so far.
    pub fn note(self) -> &'v NoteView {
        match self {
            Target::Note(note) | Target::Planned { note, .. } => note,
        }
    }

    /// How an action names this note: a vault note by its path, a planned one by the claim whose
    /// `Create` files it.
    pub fn place(self) -> Place {
        match self {
            Target::Note(note) => Place::Path(note.path.clone()),
            Target::Planned { by, .. } => Place::Planned(by.clone()),
        }
    }
}

impl ClaimRef {
    /// `mail:<h>:<n>` (D20): the message's uid and the claim's 0-based index in `claims`, the
    /// `source_uid` a note or card made from this claim carries (§5.2).
    pub fn uid(&self) -> String {
        format!("{}:{}", self.message, self.index)
    }
}

/// The vault as the plan leaves it so far (D10): the view's notes, then each note or card an earlier
/// claim's action files, with every field an earlier action sets. `super::walk` builds one per plan
/// and advances it with [`Working::apply`] after each claim, in plan order.
#[derive(Debug, Clone)]
pub struct Working {
    /// Each note with the claim whose action files it in this plan; `None` for a note in the vault.
    notes: Vec<(NoteView, Option<ClaimRef>)>,
}

impl Working {
    pub fn new(view: &VaultView) -> Working {
        Working { notes: view.notes.iter().map(|note| (note.clone(), None)).collect() }
    }

    /// Resolve one claim (§4.8 steps 2 and 4), in the module's order of steps.
    pub fn resolve(&self, at: &ClaimRef, claim: &Claim) -> Resolved<'_> {
        let uid = at.uid();
        let applied = |(n, _): &(NoteView, Option<ClaimRef>)| {
            APPLIED_FOLDERS.contains(&n.folder.as_str()) && n.source_uid.as_deref() == Some(&uid)
        };
        if self.notes.iter().any(applied) {
            return Resolved::AlreadyApplied;
        }
        self.resolve_target(claim)
    }

    /// The item note `place` names, as the plan leaves it so far: a vault note by its path, or the
    /// note a claim's `Create` files in this plan (never a card that claim files).
    pub(super) fn note_in(&self, place: &Place) -> Option<&NoteView> {
        self.notes.iter().find(|entry| names(entry, place)).map(|(n, _)| n)
    }

    /// Steps 2 to 4 alone: what the claim's target would be were its uid not yet applied. §4.9's
    /// two-value row reads it for an applied claim too, so a message delivered again after its
    /// card was filed groups exactly as it did the first time ([`super::rules::settle`]).
    pub(super) fn resolve_target(&self, claim: &Claim) -> Resolved<'_> {
        // What the claim changes, and what names its target. A change with no field, or a new_item
        // with no item, has nothing to change: unmatched, so it is shown and never guessed at.
        let (field, value, named) = match (claim.kind, claim.field, &claim.item) {
            (ClaimKind::Change, Some(field), _) => (field, claim.value.clone(), Named::Target(claim.target.as_deref())),
            (ClaimKind::NewItem, _, Some(item)) => (ClaimField::Due, item.known_due().map(str::to_string), Named::Title(&item.title)),
            (ClaimKind::Change, None, _) | (ClaimKind::NewItem, _, None) => return Resolved::Unmatched,
            (ClaimKind::CourseInfo | ClaimKind::Noise, _, _) => return Resolved::NoTarget,
        };
        let found = match &claim.resolution {
            Some(stored) => {
                let answer = |t: &Target| matches!(t, Target::Note(n) if stored.target_id.as_deref() == Some(n.id.as_str()));
                match one(self.items().filter(answer)) {
                    Found::One(target) => Found::One(target),
                    Found::Zero | Found::Many => return Resolved::Unmatched,
                }
            }
            None => match named {
                Named::Target(target) => self.by_target(claim.course.as_deref(), target),
                Named::Title(title) => self.by_title(claim.course.as_deref(), title),
            },
        };
        match found {
            Found::One(target) if claim.kind == ClaimKind::NewItem && value.is_none() => Resolved::Restated { target },
            Found::One(target) => Resolved::Change { target, field, value },
            Found::Zero if claim.kind == ClaimKind::NewItem => Resolved::New,
            Found::Zero | Found::Many => Resolved::Ambiguous,
        }
    }

    /// Advance past one planned action, as `mail.rs` will write it: a created note is an item in
    /// `tasks/`, or in `archive/` and archived when imported past (R-OB-3's folder; either is an item
    /// and neither active, so resolution reads them alike); a task or amend card sits in
    /// `approvals/` under its claim's uid; a set moves the field (mail sets only `due` directly,
    /// §4.9) and stamps its `mail_asof`. A notice (`info/`), a propose-done and a skip leave
    /// nothing resolution reads. Each note and card carries `created_by: mail` and the claim's uid.
    pub fn apply(&mut self, action: &Action) {
        let card = |status: CardStatus| match status {
            CardStatus::Pending => "pending",
            CardStatus::Snoozed => "snoozed",
        };
        let (filed_note, by) = match action {
            Action::Create { claim, note } => {
                let (folder, status) = if note.imported_past { ("archive", "archived") } else { ("tasks", "active") };
                let mut created = filed(claim, folder, status, &note.title, note.course.clone(), note.due.clone());
                created.mail_asof.extend(note.mail_asof.map(|asof| ("due".to_string(), asof)));
                (created, claim)
            }
            Action::TaskCard { claim, note, proposal } => {
                (filed(claim, "approvals", card(proposal.status), &note.title, note.course.clone(), note.due.clone()), claim)
            }
            Action::AmendCard { claim, target, proposal, .. } => {
                let (title, course) = self.note_in(target).map(|n| (n.title.clone(), n.course.clone())).unwrap_or_default();
                (filed(claim, "approvals", card(proposal.status), &title, course, None), claim)
            }
            Action::Set { target, field, to, mail_asof, .. } => {
                if let Some((note, _)) = self.notes.iter_mut().find(|entry| names(entry, target)) {
                    if field == "due" {
                        note.due = to.clone();
                    }
                    note.mail_asof.insert(field.clone(), *mail_asof);
                }
                return;
            }
            Action::Notice { .. } | Action::ProposeDone { .. } | Action::Skip { .. } => return,
        };
        self.notes.push((filed_note, Some(by.clone())));
    }
}

/// Whether a working entry is the item note `place` names: a vault note at its path, or the item
/// the claim's `Create` filed (a claim files one item at most, and a card is no item).
fn names((note, by): &(NoteView, Option<ClaimRef>), place: &Place) -> bool {
    match place {
        Place::Path(path) => by.is_none() && &note.path == path,
        Place::Planned(claim) => by.as_ref() == Some(claim) && ITEM_FOLDERS.contains(&note.folder.as_str()),
    }
}

/// A note or card an action files, as the working vault holds it until `mail.rs` writes it: no `id`
/// or `path` yet (see [`Target::Planned`]), `created_by: mail` and the claim's uid as `source_uid`
/// (§5.2, D20), and no field a human set.
fn filed(claim: &ClaimRef, folder: &str, status: &str, title: &str, course: Option<String>, due: Option<String>) -> NoteView {
    NoteView {
        id: String::new(),
        path: String::new(),
        folder: folder.to_string(),
        course,
        title: title.to_string(),
        due: due.map(Value::String).unwrap_or(Value::Null),
        status: Some(status.to_string()),
        source_uid: Some(claim.uid()),
        created_by: Some("mail".to_string()),
        mail_asof: BTreeMap::new(),
        human_set: BTreeSet::new(),
    }
}

/// The claims the plan leaves ambiguous, in plan order: what `mail.rs` asks `mail-resolve` about,
/// one batch per course (§4.8 step 3, D12). It takes [`super::plan`]'s own walk, so a claim an
/// earlier claim in the batch answers is never asked. A noise claim is never asked, and neither is
/// a claim with no course: the call offers a course's candidate items, so with none it could only
/// answer "none".
pub fn to_ask<'m>(view: &VaultView, messages: &'m [Message], today: Date, vault_day: i64) -> Vec<(ClaimRef, &'m Claim)> {
    let steps = super::planned_steps(view, messages, today, vault_day);
    steps.into_iter().filter(|step| step.ambiguous && step.claim.course.is_some()).map(|step| (step.at, step.claim)).collect()
}

/// How a claim names its note: a `change`'s `target`, or a `new_item`'s title.
enum Named<'c> {
    Target(Option<&'c str>),
    Title(&'c str),
}

/// How many notes a step found.
enum Found<'v> {
    Zero,
    One(Target<'v>),
    Many,
}

fn one<'v>(mut matches: impl Iterator<Item = Target<'v>>) -> Found<'v> {
    match (matches.next(), matches.next()) {
        (None, _) => Found::Zero,
        (Some(target), None) => Found::One(target),
        (Some(_), Some(_)) => Found::Many,
    }
}

impl Working {
    /// A `change`'s target: by uid or LMS id, then by normalised title, then by tokens (§4.8 step 2).
    fn by_target(&self, course: Option<&str>, target: Option<&str>) -> Found<'_> {
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
            match one(self.items().filter(|t| step(t.note()))) {
                Found::Zero => continue,
                decided => return decided,
            }
        }
        Found::Zero
    }

    /// A `new_item`'s title against the active notes of its course (§4.8 step 2, test 24).
    fn by_title(&self, course: Option<&str>, title: &str) -> Found<'_> {
        let title = normalise(title);
        let equal = |t: &Target| {
            let n = t.note();
            in_course(n, course) && is_active(n) && !title.is_empty() && normalise(&n.title) == title
        };
        one(self.items().filter(equal))
    }

    /// The item notes, in the vault and then planned, in the order they were added.
    fn items(&self) -> impl Iterator<Item = Target<'_>> {
        let items = self.notes.iter().filter(|(n, _)| ITEM_FOLDERS.contains(&n.folder.as_str()));
        items.map(|(note, by)| match by {
            None => Target::Note(note),
            Some(by) => Target::Planned { by, note },
        })
    }
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
    use crate::mailreconcile::{plan, Authenticity, ClaimSource, ItemKind, NewItem, NewNote, Phase, Proposal, Resolution, Sender, SkipReason};
    use serde_json::json;

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
        Resolved::Change { target: Target::Note(note), field: ClaimField::Due, value: Some("2026-10-09".to_string()) }
    }

    fn today() -> Date {
        jiff::civil::date(2026, 10, 1)
    }

    fn asked(view: &VaultView, claims: Vec<Claim>) -> Vec<usize> {
        to_ask(view, &[message(claims)], today(), 9).into_iter().map(|(at, _)| at.index).collect()
    }

    /// §4.8 step 2, first clause: the uid is the external key, so it matches in any course.
    #[test]
    fn a_target_equal_to_a_source_uid_or_lms_id_resolves() {
        let view = view(vec![
            note("task-zy", "tasks", "CS 101", "Participation 3.1", Some("zybooks:1839992")),
            note("task-bb", "tasks", "CS 101", "HW 01", Some("_901_1")),
            note("card-zy", "approvals", "CS 101", "Mark done: Participation 3.1", Some("zybooks:1839992")),
        ]);
        let working = Working::new(&view);
        for (target, want) in [("zybooks:1839992", 0), ("1839992", 0), ("_901_1", 1)] {
            for course in [Some("CS 101"), Some("MATH 220"), None] {
                let got = working.resolve(&at(0), &change(course, Some(target)));
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
        let working = Working::new(&view);
        let ask = |course: Option<&str>, target: &str| working.resolve(&at(0), &change(course, Some(target)));
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
        let working = Working::new(&view);
        let ask = |target: &str| working.resolve(&at(0), &change(Some("CS 101"), Some(target)));
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
        let working = Working::new(&view);
        let ask = |target: Option<&str>| working.resolve(&at(0), &change(Some("CS 101"), target));
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
        let working = Working::new(&view);
        let essay_1 = change(Some("CS 101"), Some("Essay 1"));
        assert_eq!(working.resolve(&at(0), &essay_1), moved(&view.notes[0]), "unanswered, it resolves itself");
        assert_eq!(working.resolve(&at(0), &stored(essay_1.clone(), Some("task-e2"))), moved(&view.notes[1]));
        assert_eq!(working.resolve(&at(0), &stored(essay_1.clone(), None)), Resolved::Unmatched, "a stored none");
        assert_eq!(working.resolve(&at(0), &stored(essay_1.clone(), Some("task-gone"))), Resolved::Unmatched);
        assert_eq!(working.resolve(&at(0), &stored(essay_1, Some("card-e2"))), Resolved::Unmatched, "a card is no item");
        let due = Resolved::Change { target: Target::Note(&view.notes[1]), field: ClaimField::Due, value: Some("2026-10-12".to_string()) };
        assert_eq!(working.resolve(&at(0), &stored(new_item("CS 101", "Essay"), Some("task-e2"))), due, "a new_item's answer");
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
        let working = Working::new(&view);
        let ask = |course: &str, title: &str| working.resolve(&at(0), &new_item(course, title));
        let target = Target::Note(&view.notes[0]);
        let due = Resolved::Change { target, field: ClaimField::Due, value: Some("2026-10-12".to_string()) };
        assert_eq!(ask("CS 101", "ESSAY #2"), due);
        // With no due the mail knows no date: it restates the note and never clears its due (risk 1),
        // answered or not, and plans one skip that writes nothing.
        let mut undated = new_item("CS 101", "essay 2");
        undated.item.as_mut().unwrap().due = None;
        assert_eq!(working.resolve(&at(0), &undated), Resolved::Restated { target }, "never a change to no due");
        let answered = stored(undated.clone(), Some("task-essay"));
        assert_eq!(working.resolve(&at(0), &answered), Resolved::Restated { target }, "answered");
        let skip = Action::Skip { claim: at(0), reason: SkipReason::Equal };
        assert_eq!(plan(&view, &[message(vec![undated])], today(), 9), vec![skip.clone()]);
        // A due in the wire shape naming no real day is no date known either: never a change to it.
        let mut misprinted = new_item("CS 101", "essay 2");
        misprinted.item.as_mut().unwrap().due = Some("2026-09-31".to_string());
        assert_eq!(working.resolve(&at(0), &misprinted), Resolved::Restated { target }, "never a change to 2026-09-31");
        assert_eq!(plan(&view, &[message(vec![misprinted])], today(), 9), vec![skip]);
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
                let working = Working::new(&view);
                assert_eq!(working.resolve(&at(index), &claims[index]), Resolved::AlreadyApplied, "{folder}, {index}");
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
        let working = Working::new(&view);
        assert_eq!(working.resolve(&at(1), &claims[1]), moved(&view.notes[0]));
        assert_eq!(working.resolve(&at(0), &claims[0]), Resolved::New);
    }

    /// One backfill message, `uid`, ordered at `ordering`.
    fn backfill(uid: &str, ordering: &str, claims: Vec<Claim>) -> Message {
        let (uid, ordering_date) = (uid.to_string(), ordering.parse().unwrap());
        Message { uid, ordering_date, phase: Phase::Backfill, claims, ..message(vec![]) }
    }

    fn dated(title: &str, due: Option<&str>) -> Claim {
        let mut claim = new_item("CS 101", title);
        claim.item.as_mut().unwrap().due = due.map(str::to_string);
        claim
    }

    /// A resolution in one line: the note it names (in the vault by its `source_uid`, or planned by
    /// the claim that files it) and the due it carries.
    fn summary(resolved: &Resolved<'_>) -> String {
        let named = |target: &Target| match target {
            Target::Note(note) => format!("note {}", note.source_uid.as_deref().unwrap_or("-")),
            Target::Planned { by, .. } => format!("planned {}", by.uid()),
        };
        match resolved {
            Resolved::Change { target, value, .. } => format!("change {} to {}", named(target), value.as_deref().unwrap_or("-")),
            Resolved::Restated { target } => format!("restated {}", named(target)),
            other => format!("{other:?}"),
        }
    }

    /// A stand-in for T5.3's backfill row: a `New` claim creates its note; nothing else writes.
    fn stand_in(seen: &mut Vec<String>, resolved: &Resolved<'_>, message: &Message, at: ClaimRef, claim: &Claim) -> Vec<Action> {
        seen.push(summary(resolved));
        match (resolved, &claim.item) {
            (Resolved::New, Some(item)) => {
                let (title, course, due) = (item.title.clone(), claim.course.clone(), item.due.clone());
                let mail_asof = due.as_ref().map(|_| message.ordering_date);
                let (item_kind, effort_hours, importance) = (item.item_kind, item.effort_hours, item.importance);
                let note = NewNote { title, course, due, item_kind, effort_hours, importance, imported_past: false, mail_asof };
                vec![Action::Create { claim: at, note }]
            }
            _ => vec![Action::Skip { claim: at, reason: SkipReason::Unmatched }],
        }
    }

    /// D10 and test 24, the review's two inputs: one backfill batch for CS 101 on an empty vault,
    /// delivered out of order. A creates "HW 5", B moves its due, C names it again dated and undated.
    /// Each later claim resolves to the note A files, as it would were A's note already written, so
    /// nothing is asked (no `mail-resolve` call) and nothing is made twice.
    #[test]
    fn a_batch_resolves_each_claim_against_the_notes_its_earlier_claims_file() {
        let moving = Claim { value: Some("2026-10-10".to_string()), ..change(Some("CS 101"), Some("HW 5")) };
        let batch = [
            backfill("mail:cccc", "2026-09-12T09:00:00Z", vec![dated("hw-5", Some("2026-10-10")), dated("HW #5", None)]),
            backfill("mail:bbbb", "2026-09-09T09:00:00Z", vec![moving]),
            backfill("mail:aaaa", "2026-09-02T09:00:00Z", vec![dated("HW 5", Some("2026-10-08"))]),
        ];
        let mut seen = Vec::new();
        let steps = crate::mailreconcile::walk(&VaultView::default(), &batch, |r, m, at, c| stand_in(&mut seen, r, m, at, c), |_, _| false);
        let a = "planned mail:aaaa:0";
        let want = ["New".to_string(), format!("change {a} to 2026-10-10"), format!("change {a} to 2026-10-10"), format!("restated {a}")];
        assert_eq!(seen, want);
        assert!(steps.iter().all(|step| !step.ambiguous), "nothing is asked");
        let created = steps.iter().flat_map(|step| &step.actions).filter(|a| matches!(a, Action::Create { .. }));
        assert_eq!(created.count(), 1, "one note, never a duplicate");
        // Split across pulls: A's note is in the vault, and B and C resolve to it the same way.
        let mut written = note("task-hw-5", "tasks", "CS 101", "HW 5", Some("mail:aaaa:0"));
        written.created_by = Some("mail".to_string());
        let mut later = Vec::new();
        crate::mailreconcile::walk(&view(vec![written]), &batch[..2], |r, m, at, c| stand_in(&mut later, r, m, at, c), |_, _| false);
        let n = "note mail:aaaa:0";
        assert_eq!(later, [format!("change {n} to 2026-10-10"), format!("change {n} to 2026-10-10"), format!("restated {n}")]);
    }

    /// What each action leaves for the claims after it, as `mail.rs` will write it: a created note is
    /// an item, archived when imported past; a card is never an item but carries its claim's uid; a
    /// set moves the note's due and stamps its `mail_asof`.
    #[test]
    fn the_working_vault_holds_what_each_earlier_action_files() {
        let view = view(vec![note("task-e2", "tasks", "CS 101", "Essay 2", Some("_902_1"))]);
        let mut working = Working::new(&view);
        let filed = |title: &str, imported_past: bool| NewNote {
            title: title.to_string(),
            course: Some("CS 101".to_string()),
            due: Some("2026-09-20".to_string()),
            item_kind: ItemKind::Task,
            effort_hours: 2.0,
            importance: 3,
            imported_past,
            mail_asof: None,
        };
        let proposal = Proposal { status: CardStatus::Pending, proposed_at: today(), first_proposed_at: today(), snooze_until: None };
        let (quiz, lab) = (ClaimRef { message: "mail:aaaa".to_string(), index: 0 }, ClaimRef { message: "mail:bbbb".to_string(), index: 0 });
        working.apply(&Action::Create { claim: quiz.clone(), note: filed("Quiz 3", true) });
        working.apply(&Action::TaskCard { claim: lab.clone(), note: filed("Lab 4", false), proposal });
        let essay = Place::Path("tasks/task-e2.md".to_string());
        working.apply(&Action::AmendCard { claim: at(6), target: essay.clone(), changes: vec![], proposal, also: vec![] });
        let stamp: jiff::Timestamp = "2026-09-09T09:00:00Z".parse().unwrap();
        let (target, field, body_line) = (essay, "due".to_string(), String::new());
        working.apply(&Action::Set { claim: at(5), target, field, to: json!("2026-10-10"), mail_asof: stamp, body_line });
        // The imported-past quiz is an item and never active: a change names it by its title, a
        // new_item with that title is new, and its tokens reach nothing. No answer names it.
        let quiz_3 = change(Some("CS 101"), Some("quiz 3"));
        let got = working.resolve(&at(0), &quiz_3);
        assert!(matches!(got, Resolved::Change { target: Target::Planned { by, .. }, .. } if *by == quiz), "{got:?}");
        assert_eq!(working.resolve(&at(0), &new_item("CS 101", "Quiz 3")), Resolved::New);
        assert_eq!(working.resolve(&at(0), &change(Some("CS 101"), Some("quiz"))), Resolved::Ambiguous);
        assert_eq!(working.resolve(&at(0), &stored(quiz_3, Some(""))), Resolved::Unmatched, "an answer names a vault note");
        // The lab's card is never a target; it and the amend card mark their claims applied.
        assert_eq!(working.resolve(&at(0), &change(Some("CS 101"), Some("Lab 4"))), Resolved::Ambiguous);
        assert_eq!(working.resolve(&lab, &new_item("CS 101", "Lab 4")), Resolved::AlreadyApplied);
        assert_eq!(working.resolve(&at(6), &change(Some("CS 101"), Some("Essay 2"))), Resolved::AlreadyApplied);
        // The set moved Essay 2's due and stamped it; the note is still the vault's, named by its path.
        let got = working.resolve(&at(0), &change(Some("CS 101"), Some("Essay 2")));
        let Resolved::Change { target: Target::Note(essay), .. } = got else { panic!("{got:?}") };
        assert_eq!((&essay.due, essay.mail_asof.get("due"), essay.path.as_str()), (&json!("2026-10-10"), Some(&stamp), "tasks/task-e2.md"));
        // Plan §9.5's planned-note amendment: a set naming a planned note by its claim reaches the
        // note that claim's `Create` files; a claim that filed only a card names no note.
        let later: jiff::Timestamp = "2026-09-10T09:00:00Z".parse().unwrap();
        for (claim, to) in [(quiz.clone(), "2026-09-25"), (lab.clone(), "2026-10-30")] {
            let (target, field, body_line) = (Place::Planned(claim), "due".to_string(), String::new());
            working.apply(&Action::Set { claim: at(7), target, field, to: json!(to), mail_asof: later, body_line });
        }
        let got = working.resolve(&at(0), &change(Some("CS 101"), Some("quiz 3")));
        let Resolved::Change { target: Target::Planned { note: quiz_note, .. }, .. } = got else { panic!("{got:?}") };
        assert_eq!((&quiz_note.due, quiz_note.mail_asof.get("due")), (&json!("2026-09-25"), Some(&later)));
        assert_eq!(working.note_in(&Place::Planned(lab)), None, "a card is no item");
        assert_eq!(Target::Planned { by: &quiz, note: quiz_note }.place(), Place::Planned(quiz.clone()));
    }
}
