//! Close-out task B2, the B3a tests: "missing" is read from `seen`, the merge carries the last
//! whole row, a part read creates nothing, the status rule and G3's two rules. Spec A2 items 10, 11
//! and 15 and the D3 amendment's P6 and P9. Run on the invented fixture and its edits; every test
//! says which B3 sub-task turns it green. Invented data only.
use super::apply_support::*;
use jiff::civil::{date, Date};
use serde_json::{json, Value};

const KEY_LECTURE: &str = "registrar:ua:202640-40001";
const KEY_ENGL: &str = "registrar:ua:202640-40003";
const NOTES: [&str; 4] = [LECTURE, LAB, "commitments/engl-101.md", "commitments/art-110.md"];

/// A vault with all four matched rows confirmed on `D1`.
fn four_notes(name: &str) -> Lab {
    let lab = Lab::new(name, "2026-08-31");
    lab.add_courses();
    let first = lab.fetch(D1, &fx());
    assert_eq!(first["confirmed"], 4, "{first}");
    lab
}

fn series_entry(lab: &Lab, key: &str) -> Value {
    let mut entry = lab.series_file()["series"].as_array().unwrap().iter().find(|s| s["source_uid"] == key).unwrap_or_else(|| panic!("{key} is not under series")).clone();
    entry.as_object_mut().unwrap().remove("last_seen");
    entry
}

/// B3a (A2 item 11, "A row the parser saw but could not read"; P9). A CRN in `seen` is never
/// missing: a row with null times, a row with only an unknown meeting type and a row that lost one
/// of two `CLAS` meetings write nothing and end nothing.
#[test]
fn a_row_read_in_part_or_not_at_all_is_not_a_missing_course() {
    let lab = four_notes("seen-missing");
    let before: Vec<String> = NOTES.iter().map(|n| lab.note(n)).collect();
    let mut later = fx();
    blank_times(&mut later, "40001");
    meeting(&mut later, "40002")["meetingType"] = json!("XYZ");
    retime(&mut later, "40003", "1000", "1115");
    add_timeless_meeting(&mut later, "40003");
    lab.fetch(D10, &later);
    for (note, was) in NOTES.iter().zip(&before) {
        assert_eq!(&lab.note(note), was, "{note} was written");
        assert!(lab.sets(note, "meets").is_empty() && lab.sets(note, "until").is_empty(), "{note}");
    }
    assert!(lab.ended_keys().is_empty(), "a seen CRN moved to ended: {:?}", lab.ended_keys());
    assert_eq!(lab.series_keys().len(), 4, "{:?}", lab.series_keys());
}

/// B3c (A2 items 11 and 13, `unread`). The same three rows are listed under `unread` with the
/// parser's reason, and none is applied, ended or kept.
#[test]
fn a_row_the_parser_saw_but_could_not_read_is_listed_under_unread_with_its_reason() {
    let lab = four_notes("seen-unread");
    let mut later = fx();
    blank_times(&mut later, "40001");
    meeting(&mut later, "40002")["meetingType"] = json!("XYZ");
    retime(&mut later, "40003", "1000", "1115");
    add_timeless_meeting(&mut later, "40003");
    let report = lab.fetch(D10, &later);
    let unread = list(&report, "unread");
    assert_eq!(unread.iter().filter(|e| !e["note"].is_null()).count(), 3, "{unread:?}");
    let reason = |note: &str| of_note(&unread, note).first().map(|e| e["reason"].as_str().unwrap().to_string()).unwrap_or_default();
    assert!(reason(LECTURE).contains("no class time") && reason(LECTURE).contains("nothing changed"), "{unread:?}");
    for note in [LAB, "commitments/engl-101.md"] {
        assert!(reason(note).contains("cannot read") && reason(note).contains("nothing changed"), "{note}: {unread:?}");
    }
    assert_eq!(of_note(&unread, LECTURE)[0]["key"], KEY_LECTURE);
    for key in ["applied", "ended", "kept"] {
        assert!(list(&report, key).is_empty(), "{key}: {report}");
    }
}

/// B3a (P9, "The merge keeps what it could not read"; A2 item 20, convergence round 1 and round
/// 3's "A confirmed note is still carried"). A confirmed note's CRN read `partial`, `unread` or
/// `status` keeps its previous row under `series`, byte for byte apart from `last_seen`.
#[test]
fn a_confirmed_notes_crn_read_in_part_keeps_its_last_whole_row() {
    for how in ["partial", "unread", "status"] {
        let lab = Lab::new(&format!("seen-carry-{how}"), "2026-08-31");
        lab.fetch(D1, &fx());
        let whole = series_entry(&lab, KEY_LECTURE);
        let mut later = fx();
        read_in_part(&mut later, "40001", how);
        fetch_read_in_part(&lab, D10, &later, how);
        assert!(!lab.ended_keys().contains(&KEY_LECTURE.to_string()), "{how}: moved to ended");
        assert_eq!(series_entry(&lab, KEY_LECTURE), whole, "{how}: the carried row changed");
    }
}

/// B3a (A2 item 10; item 20, convergence round 1). A row first read in part creates no note and
/// stores no series; a later fetch that reads it whole creates the note `hard`, as the student.
#[test]
fn a_partial_first_read_creates_nothing_and_a_whole_read_then_creates_the_note() {
    let lab = Lab::new("seen-first", "2026-08-31");
    let mut first = fx();
    add_timeless_meeting(&mut first, "40002");
    let report = lab.fetch(D1, &first);
    assert!(!lab.path(LAB).exists(), "a part read created {LAB}");
    assert!(lab.path(LECTURE).exists(), "the whole row is still created");
    assert_eq!(report["confirmed"], 1, "{report}");
    assert!(!lab.series_keys().contains(&"registrar:ua:202640-40002".to_string()), "{:?}", lab.series_keys());
    lab.fetch(D2, &fx());
    assert_eq!(lab.line(LAB, "level").as_deref(), Some("hard"));
    let made = lab.journal().into_iter().find(|r| r["op"] == "create" && r["path"] == LAB).expect("the lab's create record");
    assert_eq!(made["actor"], "student", "{made}");
}

/// B3a (P6; A2 item 20, "The status rule"). With a non-empty registered set, a confirmed note whose
/// row comes back under another status is neither written nor ended, and a row under such a status
/// on a first fetch creates no note. With the empty set every row counts.
#[test]
fn a_row_under_a_non_registered_status_is_neither_written_nor_ended_and_creates_no_note() {
    let lab = Lab::new("status-rule", "2026-08-31");
    let mut first = fx();
    set_status(&mut first, "40002", "WD", "Invented Withdrawn");
    lab.fetch_registered(D1, &first, &REGISTERED);
    assert!(!lab.path(LAB).exists() && lab.path(LECTURE).exists(), "a row outside the set created a note");
    lab.fetch_registered(D2, &fx(), &REGISTERED);
    assert!(lab.path(LAB).exists(), "registered again, the lab is created");
    let before = lab.note(LECTURE);
    let mut third = fx();
    set_status(&mut third, "40001", "WD", "Invented Withdrawn");
    retime(&mut third, "40001", "1500", "1550");
    lab.fetch_registered(D10, &third, &REGISTERED);
    assert_eq!(lab.note(LECTURE), before, "a row outside the set was written");
    assert!(lab.ended_keys().is_empty(), "a row outside the set was ended: {:?}", lab.ended_keys());

    let empty = Lab::new("status-empty-set", "2026-08-31");
    empty.fetch_registered(D1, &first, &[]);
    assert!(empty.path(LAB).exists(), "with the empty set every row counts");
}

/// B3c (A2 item 13, `status`): the row under another status is listed, named by its description.
#[test]
fn a_row_outside_the_registered_set_is_listed_under_status() {
    let lab = Lab::new("status-list", "2026-08-31");
    lab.fetch(D1, &fx());
    let mut later = fx();
    set_status(&mut later, "40001", "WD", "Invented Withdrawn");
    let report = lab.fetch_registered(D10, &later, &REGISTERED);
    let status = list(&report, "status");
    assert_eq!(status.len(), 1, "{status:?}");
    assert_eq!((status[0]["key"].as_str(), status[0]["note"].as_str()), (Some(KEY_LECTURE), Some(LECTURE)));
    assert!(status[0]["status"].as_str().unwrap().contains("Invented Withdrawn"), "{status:?}");
    assert!(list(&report, "applied").is_empty() && list(&report, "ended").is_empty(), "{report}");
}

fn status_on(lab: &Lab, day: Date) -> Value {
    let (file, _) = crate::commitments::read_series_file(&lab.v);
    super::status(&file, day)
}

/// B3a (A2 item 15, G3, "Over is not in play"). A term is over for the button the day after its
/// last `CLAS` meeting; keep-alive is Amendment 1's separate rule (28 days past), so a term 10 days
/// past its last class is over for the button and still in play.
#[test]
fn a_term_ten_days_past_its_last_class_is_over_for_the_button_and_still_in_play() {
    let lab = Lab::new("over", "2026-08-31");
    lab.fetch(D1, &fx());
    assert_eq!(status_on(&lab, date(2026, 12, 4))["refresh"], false, "the last class day is not over");
    assert_eq!(status_on(&lab, date(2026, 12, 5))["refresh"], true, "the day after is");
    let ten_past = date(2026, 12, 14);
    assert_eq!(status_on(&lab, ten_past)["refresh"], true);
    let (file, _) = crate::commitments::refresh_series(&lab.v, &[("personal".to_string(), Vec::new())], ten_past);
    assert_eq!(file.series.len(), 4, "10 days past, the term is still in play");
}

/// B3a (A2 item 15, "Which term it fetches then", "With no held term"). The engine names the next
/// term: after a held, over fall `<year>40` it is spring `<year+1>10`; after a held, over spring
/// `<year>10` it is fall `<year>40`; with no held term `term_for`'s month table decides.
#[test]
fn the_next_term_follows_the_latest_held_one_once_it_is_over() {
    let fall = Lab::new("next-fall", "2026-08-31");
    fall.fetch(D1, &fx());
    assert_eq!(status_on(&fall, date(2026, 11, 1))["current"], "202640", "in session, the held term");
    assert_eq!(status_on(&fall, date(2026, 12, 14))["current"], "202710");
    let spring = Lab::new("next-spring", "2026-12-30");
    spring.fetch(date(2027, 1, 5), &super::apply_support::spring());
    assert_eq!(status_on(&spring, date(2027, 5, 20))["current"], "202740");
    assert_eq!(status_on(&spring, date(2027, 5, 20))["refresh"], true);
    let (none, _) = crate::commitments::read_series_file(&Lab::new("next-none", "2026-08-31").v);
    assert_eq!(super::status(&none, date(2027, 1, 4)), Value::Null);
    assert_eq!(super::term_for("ua", date(2027, 1, 4)).as_deref(), Some("202710"));
}

/// B3a (P9, "With no confirmed note, nothing is carried"; A2 item 20, convergence round 3, "A
/// dropped elective is not carried", the series part). An unmatched course's row read whole is a
/// proposal; read in part or under a non-registered status it leaves `series` for `ended` and no
/// note is created; read whole again it is back and its `ended` entry is gone.
#[test]
fn a_dropped_electives_row_is_not_carried_and_comes_back_when_read_whole() {
    for how in ["status", "partial", "unread"] {
        let lab = Lab::new(&format!("elective-{how}"), "2026-08-31");
        lab.fetch_registered(D1, &fx(), &REGISTERED);
        assert!(lab.series_keys().contains(&KEY_ENGL.to_string()));
        let mut later = fx();
        read_in_part(&mut later, "40003", how);
        lab.fetch_registered(D10, &later, &REGISTERED);
        assert!(!lab.series_keys().contains(&KEY_ENGL.to_string()), "{how}: carried with no note to hold it");
        assert!(lab.ended_keys().contains(&KEY_ENGL.to_string()), "{how}: not in ended");
        assert!(!lab.path("commitments/engl-101.md").exists(), "{how}: a note was created");
        lab.fetch_registered(D20, &fx(), &REGISTERED);
        assert!(lab.series_keys().contains(&KEY_ENGL.to_string()), "{how}: not back under series");
        assert!(!lab.ended_keys().contains(&KEY_ENGL.to_string()), "{how}: its ended entry stayed");
    }
}
