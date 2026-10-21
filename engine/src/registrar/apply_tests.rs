//! Close-out task B2, the B3b tests (and the A1-F1 end-to-end of B4a): a fetch applies a changed
//! `meets`, `where` or `until` to a confirmed registrar note and lists it; `rank` stops carding the
//! registrar. Spec A2 items 8, 9, 14 and the plan's B2 bullets. Invented data only.
use super::apply_support::*;
use crate::cli::StashEntry;
use crate::commitments::Meet;
use crate::journal::Journal;
use jiff::civil::{date, time};
use serde_json::{json, Value};

fn mwf(begin: (i8, i8), end: (i8, i8)) -> Vec<Meet> {
    vec![Meet { days: vec!["mon", "wed", "fri"], start: time(begin.0, begin.1, 0, 0), end: time(end.0, end.1, 0, 0) }]
}

fn fields_of(entries: &[Value]) -> Vec<String> {
    let mut fields: Vec<String> = entries.iter().map(|e| e["field"].as_str().unwrap().to_string()).collect();
    fields.sort();
    fields
}

fn moved_lecture() -> Value {
    let mut later = fx();
    retime(&mut later, "40001", "1300", "1350");
    meeting(&mut later, "40001")["room"] = json!("102");
    meeting(&mut later, "40001")["endDate"] = json!("12/11/2026");
    later
}

/// B3b (A2 items 8 and 9). A refetch with a moved time, room and end applies each changed field
/// through `write` as `agent:knowlu.registrar` via `dashboard`, one record per field, lists it with
/// its old and new value, and `rank` files no card for it.
#[test]
fn a_refetch_with_a_moved_time_room_and_end_applies_each_field_and_lists_it() {
    let lab = Lab::new("apply-moved", "2026-08-31");
    lab.fetch(D1, &fx());
    let report = lab.fetch(D10, &moved_lecture());
    let applied = of_note(&list(&report, "applied"), LECTURE);
    assert_eq!(fields_of(&applied), ["meets", "until", "where"], "{report}");
    let entry = |field: &str| applied.iter().find(|e| e["field"] == field).unwrap().clone();
    assert_eq!(entry("meets")["from"], json!([{ "days": ["mon", "wed", "fri"], "start": "12:00", "end": "12:50" }]));
    assert_eq!(entry("meets")["to"], json!([{ "days": ["mon", "wed", "fri"], "start": "13:00", "end": "13:50" }]));
    assert_eq!((entry("where")["from"].as_str(), entry("where")["to"].as_str()), (Some("Invented Hall 101"), Some("Invented Hall 102")));
    assert_eq!((entry("until")["from"].as_str(), entry("until")["to"].as_str()), (Some("2026-12-04"), Some("2026-12-11")));
    let now = lab.loaded(LECTURE);
    assert_eq!((now.meets, now.where_.as_deref(), now.until), (mwf((13, 0), (13, 50)), Some("Invented Hall 102"), Some(date(2026, 12, 11))));
    for field in ["meets", "where", "until"] {
        let sets = lab.sets(LECTURE, field);
        assert_eq!(sets.len(), 1, "{field}: {sets:?}");
        assert_eq!((sets[0].0.as_str(), sets[0].1.as_str()), ("agent:knowlu.registrar", "dashboard"), "{field}");
    }
    assert_eq!(lab.loaded(LAB).meets.len(), 1, "the lab, unchanged, is not listed");
    assert!(of_note(&list(&report, "applied"), LAB).is_empty());
    lab.rank(D10);
    assert!(lab.change_cards().is_empty(), "rank carded a change the fetch applied: {:?}", lab.change_cards());
}

/// B3b (A2 item 9). Only `meets`, `where` and `until` are ever written: a row that now names
/// another course, another kind and another title changes none of `level`, `kind`, `course` or
/// the title.
#[test]
fn level_kind_course_and_title_are_never_written() {
    let lab = Lab::new("apply-never", "2026-08-31");
    lab.fetch(D1, &fx());
    let was: Vec<Option<String>> = ["level", "kind", "course", "title"].iter().map(|k| lab.line(LECTURE, k)).collect();
    let mut later = fx();
    row(&mut later, "40001")["subject"] = json!("MATH");
    row(&mut later, "40001")["courseNumber"] = json!("200");
    row(&mut later, "40001")["scheduleDescription"] = json!("Laboratory");
    retime(&mut later, "40001", "1300", "1350");
    let report = lab.fetch(D10, &later);
    assert_eq!(fields_of(&of_note(&list(&report, "applied"), LECTURE)), ["meets"], "{report}");
    for (field, was) in ["level", "kind", "course", "title"].iter().zip(was) {
        assert!(lab.sets(LECTURE, field).is_empty(), "{field} was written");
        assert_eq!(lab.line(LECTURE, field), was, "{field} changed");
    }
}

/// B3b (A2 item 14, idempotence). A second identical fetch applies nothing, lists nothing new and
/// writes no journal record.
#[test]
fn an_identical_refetch_applies_nothing() {
    let lab = Lab::new("apply-twice", "2026-08-31");
    lab.fetch(D1, &fx());
    lab.fetch(D10, &moved_lecture());
    assert_eq!(lab.loaded(LECTURE).meets, mwf((13, 0), (13, 50)), "the first refetch applied it");
    let (journal, notes) = (lab.journal().len(), [lab.note(LECTURE), lab.note(LAB)]);
    let report = lab.fetch(D20, &moved_lecture());
    assert!(list(&report, "applied").is_empty() && list(&report, "ended").is_empty(), "{report}");
    assert_eq!((lab.journal().len(), [lab.note(LECTURE), lab.note(LAB)]), (journal, notes));
    lab.rank(D20);
    assert!(lab.change_cards().is_empty());
}

/// B3b (A2 item 14). A fetch that exits 2 writes nothing: not a note, not a journal record, not the
/// series file, applied writes included. Spec §3 for every shape of failure.
#[test]
fn an_exit_2_fetch_writes_nothing_applied_writes_included() {
    let lab = Lab::new("apply-exit2", "2026-08-31");
    lab.add_courses();
    lab.fetch(D1, &fx());
    let before = lab.snapshot();
    let mut nothing_usable = moved_lecture();
    blank_all(&mut nothing_usable);
    let answers = [
        ("empty", json!({ "data": { "registrations": [] } }).to_string()),
        ("no list", json!({ "data": {} }).to_string()),
        ("sign-in page", "<!DOCTYPE html><html>Sign in</html>".to_string()),
        ("no usable row", nothing_usable.to_string()),
    ];
    for (name, text) in answers {
        let result = super::run(&lab.v, &text, "ua", D10, &lab.ctx(), &mut Journal::new(&lab.v));
        assert!(result.is_err(), "{name}: {result:?}");
        assert_eq!(lab.snapshot(), before, "{name} wrote something");
    }
}

const CHESS: &str = "id: cmt_00000000a4\ntype: commitment\nkind: club\nlevel: soft\ntitle: \"Chess Club\"\n\
    meets: [{days: [wed], start: \"18:00\", end: \"19:00\"}]\nsource_uid: \"gcal-series:chess01\"\nstatus: confirmed\n";
const ROBOTICS: &str = "id: cmt_00000000a5\ntype: commitment\nkind: club\nlevel: soft\ntitle: \"Robotics Club\"\n\
    meets: [{days: [tue], start: \"18:00\", end: \"19:00\"}]\nsource_uid: \"ics-series:robotics@example.edu\"\nstatus: confirmed\n";
const ROBOTICS_ICS: &str = "BEGIN:VCALENDAR\nVERSION:2.0\nPRODID:-//Invented//EN\nBEGIN:VEVENT\nUID:robotics@example.edu\n\
    SUMMARY:Robotics Club\nDTSTART;TZID=America/Chicago:20260908T190000\nDTEND;TZID=America/Chicago:20260908T200000\n\
    RRULE:FREQ=WEEKLY;BYDAY=TU\nEND:VEVENT\nEND:VCALENDAR\n";

/// B3b (A2 item 8; Plan ruling R2-g goes). `rank` over a vault whose registrar series moved (a
/// change), lost a row (an end) and gained a spring twin of the dropped lab (a swap) files no
/// card for a `registrar:` key, and still files a change card for a `gcal-series:` and an
/// `ics-series:` note in the same run. The series move without any fetch applying them, so the
/// test pins `rank` alone.
#[test]
fn rank_files_no_change_end_or_swap_card_for_a_registrar_key_but_still_for_google_and_ics() {
    let lab = Lab::new("apply-rank", "2026-08-31");
    lab.add_courses();
    assert_eq!(lab.fetch(D1, &fx())["confirmed"], 4);
    lab.plant_note("chess-club.md", CHESS);
    lab.plant_note("robotics-club.md", ROBOTICS);
    let mut later = fx();
    retime(&mut later, "40001", "1300", "1350");
    drop_crn(&mut later, "40002");
    drop_crn(&mut later, "40003");
    let fall = super::parse_banner(&later, "ua").unwrap();
    crate::commitments::refresh_series(&lab.v, &[(CAL.to_string(), fall.series)], D10);
    let mut spring_answer = spring();
    drop_crn(&mut spring_answer, "50003"); // no spring twin of ENGL 101, so its question is a plain end
    retime(&mut spring_answer, "50002", "1500", "1650");
    let spring_parsed = super::parse_banner(&spring_answer, "ua").unwrap();
    crate::commitments::refresh_series(&lab.v, &[("registrar:ua:202710".to_string(), spring_parsed.series)], D10);
    let entries = vec![
        ("cloud:google", google_entry(vec![google_item("chess01", "Chess Club", &[3], "18:00", "19:00", 8)])),
        (ICS_URL, StashEntry::Ics(ROBOTICS_ICS.to_string())),
    ];
    lab.rank_with(D10, entries);
    let mut keys: Vec<String> = lab.change_cards().iter().map(|c| c.meta["source_uid"].as_str().unwrap().to_string()).collect();
    keys.sort();
    assert_eq!(keys, ["gcal-series:chess01", "ics-series:robotics@example.edu"], "{:?}", lab.change_cards());
}

/// B3b (A2 item 8; plan ruling R2 m3). A course dropped in fall and taken again in spring: the
/// spring fetch creates the spring lab's note as the student, and `rank` files neither a swap
/// card moving the fall note to the spring key nor an end card for the fall lab (the fetch that
/// dropped it is what ends it, B3d). The new note already comes today; the cards are what is red.
#[test]
fn a_fall_course_retaken_in_spring_gets_a_new_note_and_no_swap_card() {
    let lab = Lab::new("apply-retaken", "2026-08-31");
    lab.fetch(D1, &fx());
    let mut dropped = fx();
    drop_crn(&mut dropped, "40002");
    lab.fetch(D10, &dropped);
    let mut spring_answer = spring();
    retime(&mut spring_answer, "50002", "1500", "1650");
    lab.fetch(D20, &spring_answer);
    let made = lab.notes_with("registrar:ua:202710-50002");
    assert_eq!(made.len(), 1, "the spring lab's note: {made:?}");
    assert_eq!(lab.line(&made[0], "level").as_deref(), Some("hard"));
    lab.rank(D20);
    assert!(lab.change_cards().is_empty(), "a swap or end card: {:?}", lab.change_cards());
}

fn cs100_one_minute_off() -> Value {
    google_item("cs100aa", "CS 100", &[0, 2, 4], "12:00", "12:51", 8)
}

fn class_notes(lab: &Lab) -> usize {
    lab.notes_with("course: cs-100").iter().filter(|n| lab.line(n, "kind").as_deref() == Some("class")).count()
}

/// B4a's end to end (close-out plan B2, A1-F1; a guard that should already be green). `rank` files
/// a card for a Google class one minute off the registrar's times; `commitments --registrar` runs
/// and creates the registrar's class; the card is approved before the next `rank`: no second class
/// note for the course, and the card is stamped `superseded`.
#[test]
fn an_approved_google_class_card_is_superseded_once_the_registrar_holds_the_course() {
    let lab = Lab::new("f1-approved", "2026-08-31");
    let google = || vec![("cloud:google", google_entry(vec![cs100_one_minute_off()]))];
    lab.rank_with(D7, google());
    let cards = lab.proposal_cards("gcal-series:cs100aa");
    assert_eq!((cards.len(), cards[0].status()), (1, "pending"), "{cards:?}");
    // Approved before the fetch (close-out B3e'): the fetch's withdrawal pass archives a card
    // still pending, as `rank` would, so only an approved card reaches the next `rank`'s settlement.
    lab.set_status_of("approvals", &cards[0].name, "approved");
    lab.fetch(D7, &fx());
    assert_eq!(class_notes(&lab), 1, "the registrar's class");
    lab.rank_with(D8, google());
    let after = lab.proposal_cards("gcal-series:cs100aa");
    assert_eq!((after.len(), after[0].folder, after[0].status()), (1, "archive", "superseded"), "{after:?}");
    assert_eq!(class_notes(&lab), 1, "a second class note for the course");
}

/// The same, without the approval: the next `rank` archives the card (A1-F1).
#[test]
fn an_unanswered_google_class_card_is_archived_by_the_next_rank_once_the_registrar_holds_the_course() {
    let lab = Lab::new("f1-unanswered", "2026-08-31");
    let google = || vec![("cloud:google", google_entry(vec![cs100_one_minute_off()]))];
    lab.rank_with(D7, google());
    assert_eq!(lab.proposal_cards("gcal-series:cs100aa").len(), 1);
    lab.fetch(D7, &fx());
    lab.rank_with(D8, google());
    let after = lab.proposal_cards("gcal-series:cs100aa");
    assert_eq!((after.len(), after[0].folder, after[0].status()), (1, "archive", "superseded"), "{after:?}");
    assert_eq!(class_notes(&lab), 1);
}

/// A pending Google class card for CS 100 and the fetch that creates the registrar's class: the
/// card's archive records after the fetch.
fn fetch_over_a_pending_google_card(name: &str) -> (Lab, Vec<Value>) {
    let lab = Lab::new(name, "2026-08-31");
    lab.rank_with(D7, vec![("cloud:google", google_entry(vec![cs100_one_minute_off()]))]);
    let card = lab.proposal_cards("gcal-series:cs100aa").remove(0);
    let before = lab.journal().len();
    lab.fetch(D7, &fx());
    let records = lab.journal().split_off(before).into_iter()
        .filter(|r| r["path"].as_str().is_some_and(|p| p.ends_with(&format!("/{}", card.name))))
        .collect();
    (lab, records)
}

/// B3 minors (close-out B3e', the 2f785ae reorder). With the registrar holding the course, the
/// fetch's own withdrawal pass archives a still-pending `gcal-series:` class card, no `rank` needed.
#[test]
fn the_fetch_archives_a_pending_google_class_card_once_the_registrar_holds_the_course() {
    let (lab, records) = fetch_over_a_pending_google_card("f1-fetch-archives");
    let after = lab.proposal_cards("gcal-series:cs100aa");
    assert_eq!((after.len(), after[0].folder, after[0].status()), (1, "archive", "superseded"), "{after:?}");
    assert!(records.iter().any(|r| r["op"] == "delete"), "{records:?}");
    assert_eq!(class_notes(&lab), 1);
}

/// B3 minors (review B3ePR, 9f53ad5). That withdrawal journals as `rank`'s approvals actor,
/// `agent:approvals`, never as the student whose context the fetch runs under.
#[test]
fn the_fetchs_withdrawal_journals_as_agent_approvals() {
    let (_lab, records) = fetch_over_a_pending_google_card("f1-fetch-actor");
    assert!(!records.is_empty(), "the fetch withdrew nothing");
    assert!(records.iter().all(|r| r["actor"] == "agent:approvals"), "{records:?}");
}

/// B3f (A2 item 16, "not watched stays unwatched"; a guard that should already be green). A
/// confirmed note whose `source_uid` is outside `registrar:`, `gcal-series:` and `ics-series:`,
/// with a series under that key whose times moved, gets no `commitment-check` card.
#[test]
fn a_note_keyed_outside_the_three_prefixes_files_no_change_card() {
    let lab = Lab::new("unwatched", "2026-08-31");
    let chess = || vec![("cloud:google", google_entry(vec![google_item("chess01", "Chess Club", &[3], "18:00", "19:00", 8)]))];
    lab.rank_with(D7, chess());
    let mut file = lab.series_file();
    file["series"][0]["source_uid"] = json!("other:chess01");
    crate::pystr::write_text(&lab.path("state/calendar-series.json"), &format!("{}\n", crate::ledger::dumps_value(&file))).unwrap();
    lab.plant_note("chess-club.md", OTHER);
    lab.rank(D8);
    assert!(lab.change_cards().is_empty(), "{:?}", lab.change_cards());
}

const OTHER: &str = "id: cmt_00000000a6\ntype: commitment\nkind: club\nlevel: soft\ntitle: \"Chess Club\"\n\
    meets: [{days: [wed], start: \"18:00\", end: \"19:00\"}]\nsource_uid: \"other:chess01\"\nstatus: confirmed\n";
