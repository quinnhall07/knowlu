//! Close-out task B2, the B3d tests: a dropped course is ended once, applied and listed, and the
//! CRN's come-back closes the entry. Spec A2 item 11 and item 20 (round 2 and convergence round 1).
//! The hand-set `until` halves are B3e's and sit in `kept_tests`. Invented data only.
use super::apply_support::*;
use jiff::civil::date;
use serde_json::{json, Value};

fn without_lab() -> Value {
    let mut later = fx();
    drop_crn(&mut later, "40002");
    later
}

fn one_end_record(lab: &Lab, until: &str) {
    let sets = lab.sets(LAB, "until");
    assert_eq!(sets.len(), 1, "{sets:?}");
    assert_eq!((sets[0].0.as_str(), sets[0].1.as_str(), sets[0].2.as_str()), ("agent:knowlu.registrar", "dashboard", Some(until)));
}

/// B3d (A2 item 11). A refetch with no row for a confirmed note's CRN sets its `until` to the day
/// before the fetch through `write` as the registrar, lists it as ended, and files no end card.
#[test]
fn a_refetch_without_a_row_ends_the_note_the_day_before_and_lists_it() {
    let lab = Lab::new("end-basic", "2026-08-31");
    lab.fetch(D1, &fx());
    let report = lab.fetch(D10, &without_lab());
    let ended = of_note(&list(&report, "ended"), LAB);
    assert_eq!(ended.len(), 1, "{report}");
    assert_eq!((ended[0]["until"].as_str(), ended[0]["kept_yours"].as_bool()), (Some("2026-09-09"), Some(false)));
    assert_eq!(lab.loaded(LAB).until, Some(date(2026, 9, 9)));
    one_end_record(&lab, "2026-09-09");
    assert!(of_note(&list(&report, "applied"), LAB).is_empty(), "an end is listed as ended, not as applied");
    lab.rank(D10);
    assert!(lab.change_cards().is_empty(), "{:?}", lab.change_cards());
}

/// B3d (A2 item 20, round 2, "Ended once, across days"). The same answer on day 20 writes nothing,
/// adds no second `ended` entry and keeps `until` at the day before the first fetch that missed it.
#[test]
fn a_course_is_ended_once_across_days() {
    let lab = Lab::new("end-once", "2026-08-31");
    lab.fetch(D1, &fx());
    lab.fetch(D10, &without_lab());
    let journal = lab.journal().len();
    let report = lab.fetch(D20, &without_lab());
    assert!(list(&report, "ended").is_empty() && list(&report, "applied").is_empty(), "{report}");
    assert_eq!(lab.journal().len(), journal, "the second fetch wrote a record");
    one_end_record(&lab, "2026-09-09");
    assert_eq!(lab.loaded(LAB).until, Some(date(2026, 9, 9)));
    let entries = lab.ended_entries();
    assert_eq!(entries.len(), 1, "{entries:?}");
    assert_eq!((entries[0]["note"].as_str(), entries[0]["until"].as_str(), entries[0]["day"].as_str()), (Some(LAB), Some("2026-09-09"), Some("2026-09-10")));
    assert!(entries[0].get("returned").is_none(), "an open entry has no `returned`: {entries:?}");
}

/// B3d (A2 item 11, "Block absent"). With the block lost between the fetches, day 20 still writes
/// nothing: the journal's last `until` record by the registrar, dated before the fetch, says so.
#[test]
fn a_course_is_ended_once_even_with_the_block_lost() {
    let lab = Lab::new("end-block-lost", "2026-08-31");
    lab.fetch(D1, &fx());
    lab.fetch(D10, &without_lab());
    one_end_record(&lab, "2026-09-09");
    lab.drop_block();
    let journal = lab.journal().len();
    let report = lab.fetch(D20, &without_lab());
    assert!(list(&report, "ended").is_empty(), "{report}");
    assert_eq!(lab.journal().len(), journal);
    one_end_record(&lab, "2026-09-09");
}

/// B3d (A2 item 11, "When the CRN comes back"; item 20, round 2). A later fetch that returns the CRN
/// as a whole row moves `until` to the row's value, lists it under `applied`, and closes the day-10
/// `ended` entry with `returned` day 20.
#[test]
fn the_crn_coming_back_moves_until_and_closes_the_entry() {
    let lab = Lab::new("end-comeback", "2026-08-31");
    lab.fetch(D1, &fx());
    lab.fetch(D10, &without_lab());
    let report = lab.fetch(D20, &fx());
    let applied = of_note(&list(&report, "applied"), LAB);
    assert_eq!(applied.len(), 1, "{report}");
    assert_eq!((applied[0]["field"].as_str(), applied[0]["from"].as_str(), applied[0]["to"].as_str()), (Some("until"), Some("2026-09-09"), Some("2026-12-04")));
    assert_eq!(lab.loaded(LAB).until, Some(date(2026, 12, 4)));
    let entries = lab.ended_entries();
    assert_eq!(entries.len(), 1, "{entries:?}");
    assert_eq!((entries[0]["until"].as_str(), entries[0]["returned"].as_str()), (Some("2026-09-09"), Some("2026-09-20")));
}

/// B3d (A2 item 20, convergence round 1, "Dropped, back, dropped again"). Day 1 end, day 20 whole
/// again (entry closed), day 40 missing again: `until` is the day before day 40, written and listed,
/// a second open entry; a day-50 fetch with the same answer writes nothing and adds no third.
#[test]
fn dropped_back_and_dropped_again_is_a_new_drop() {
    let lab = Lab::new("end-twice", "2026-08-31");
    lab.fetch(D1, &fx());
    lab.fetch(D10, &without_lab());
    lab.fetch(D20, &fx());
    let report = lab.fetch(D40, &without_lab());
    let ended = of_note(&list(&report, "ended"), LAB);
    assert_eq!((ended.len(), ended[0]["until"].as_str()), (1, Some("2026-10-09")), "{report}");
    assert_eq!(lab.loaded(LAB).until, Some(date(2026, 10, 9)));
    let sets = lab.sets(LAB, "until");
    assert_eq!(sets.len(), 3, "end, come-back, end: {sets:?}");
    assert_eq!((sets[2].0.as_str(), sets[2].2.as_str()), ("agent:knowlu.registrar", Some("2026-10-09")));
    let entries = lab.ended_entries();
    assert_eq!(entries.len(), 2, "{entries:?}");
    assert_eq!(entries[0]["returned"], "2026-09-20");
    assert!(entries[1].get("returned").is_none() && entries[1]["until"] == "2026-10-09", "{entries:?}");
    lab.fetch(D50, &without_lab());
    assert_eq!((lab.sets(LAB, "until").len(), lab.ended_entries().len()), (3, 2), "the day-50 fetch added something");
}

/// B3d (A2 item 11, "A come-back that is not whole is not a come-back"; item 20, convergence
/// round 1). After the day-1 end, a day-20 fetch that returns the CRN `partial` (and, in a second
/// case, `unread`) writes nothing, closes nothing and lists it under `unread`; a day-30 miss is
/// still already ended.
#[test]
fn a_come_back_that_is_not_whole_closes_nothing() {
    for how in ["partial", "unread"] {
        let lab = Lab::new(&format!("end-notwhole-{how}"), "2026-08-31");
        lab.fetch(D1, &fx());
        lab.fetch(D10, &without_lab());
        let mut part = fx();
        match how {
            "partial" => add_timeless_meeting(&mut part, "40002"),
            _ => blank_times(&mut part, "40002"),
        }
        let report = lab.fetch(D20, &part);
        assert!(of_note(&list(&report, "applied"), LAB).is_empty(), "{how}: {report}");
        assert_eq!(of_note(&list(&report, "unread"), LAB).len(), 1, "{how}: {report}");
        assert_eq!(lab.loaded(LAB).until, Some(date(2026, 9, 9)), "{how}: written");
        assert!(lab.ended_entries()[0].get("returned").is_none(), "{how}: closed");
        let later = lab.fetch(D30, &without_lab());
        assert!(list(&later, "ended").is_empty(), "{how}: {later}");
        one_end_record(&lab, "2026-09-09");
        assert_eq!(lab.ended_entries().len(), 1, "{how}");
    }
}

/// B3d (A2 item 13, "After a restart", the `ended` part; item 20, round 1). The ended entry has
/// its `agent:knowlu.registrar` journal record, and a fresh read of the overview lists it.
#[test]
fn the_list_survives_a_restart_its_ended_part() {
    let lab = Lab::new("end-restart", "2026-08-31");
    lab.fetch(D1, &fx());
    let report = lab.fetch(D10, &without_lab());
    let listed = list(&report, "ended");
    let overview = crate::commitments::overview(&lab.v, D10).to_json();
    assert_eq!(overview["registrar_changes"]["ended"], json!(listed), "{overview}");
    let record = lab.sets(LAB, "until");
    assert_eq!((record.len(), record[0].2.as_str()), (1, listed[0]["until"].as_str()), "an ended entry without its record");
}
