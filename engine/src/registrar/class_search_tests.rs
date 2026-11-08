//! Replan C1 (`docs/plans/2026-11-04-onboarding-replan-draft.md` §6.3, §6.4): the name rule and the
//! class-search parse. Every value is invented: term 209940, subject XYZ, made-up rooms. The row
//! shape is R0b's: `{success, totalCount, data: [...]}`, meetings at
//! `data[].meetingsFaculty[].meetingTime`.
use super::*;
use jiff::civil::date;
use serde_json::json;

const SHAPE: NameShape = NameShape::TermSubjectNumberSection;
const ALL_DAYS: [&str; 7] = ["monday", "tuesday", "wednesday", "thursday", "friday", "saturday", "sunday"];

// ---- section_in_name ----------------------------------------------------------------------

fn sref(term: &str, subject: &str, number: &str, section: &str) -> SectionRef {
    SectionRef { term: term.into(), subject: subject.into(), number: number.into(), section: section.into() }
}

#[test]
fn the_fitting_form_is_read() {
    assert_eq!(section_in_name("209940-XYZ-100-001", SHAPE), Some(sref("209940", "XYZ", "100", "001")));
}

#[test]
fn trailing_words_are_allowed_after_a_boundary() {
    assert_eq!(section_in_name("209940-XYZ-100-001 Intro to Invented Things", SHAPE), Some(sref("209940", "XYZ", "100", "001")));
    assert_eq!(section_in_name("209940-XYZ-100-001 (Fall)", SHAPE), Some(sref("209940", "XYZ", "100", "001")));
}

#[test]
fn a_merged_shell_does_not_fit_and_is_never_cut_down() {
    assert_eq!(section_in_name("209940-XYZ-100-001-002", SHAPE), None);
    assert_eq!(section_in_name("209940-XYZ-100-001-002 Merged", SHAPE), None);
}

#[test]
fn a_four_digit_number_does_not_fit() {
    assert_eq!(section_in_name("209940-XYZ-1000-001", SHAPE), None);
}

#[test]
fn a_two_digit_number_does_not_fit() {
    assert_eq!(section_in_name("209940-XYZ-10-001", SHAPE), None);
}

#[test]
fn a_number_may_carry_one_trailing_letter() {
    assert_eq!(section_in_name("209940-XYZ-100L-001", SHAPE), Some(sref("209940", "XYZ", "100L", "001")));
    assert_eq!(section_in_name("209940-XYZ-100LL-001", SHAPE), None);
}

#[test]
fn lower_case_does_not_fit() {
    assert_eq!(section_in_name("209940-xyz-100-001", SHAPE), None);
    assert_eq!(section_in_name("209940-Xyz-100-001", SHAPE), None);
}

#[test]
fn a_name_with_no_term_does_not_fit() {
    assert_eq!(section_in_name("XYZ-100-001", SHAPE), None);
    assert_eq!(section_in_name("XYZ 100 Intro", SHAPE), None);
}

#[test]
fn a_term_is_exactly_six_digits() {
    assert_eq!(section_in_name("20994-XYZ-100-001", SHAPE), None);
    assert_eq!(section_in_name("2099400-XYZ-100-001", SHAPE), None);
}

#[test]
fn a_name_with_the_code_mid_string_does_not_fit() {
    assert_eq!(section_in_name("Fall 209940-XYZ-100-001", SHAPE), None);
    assert_eq!(section_in_name("Intro (209940-XYZ-100-001)", SHAPE), None);
}

#[test]
fn a_subject_is_two_to_four_capitals() {
    assert_eq!(section_in_name("209940-XY-100-001", SHAPE), Some(sref("209940", "XY", "100", "001")));
    assert_eq!(section_in_name("209940-XYZW-100-001", SHAPE), Some(sref("209940", "XYZW", "100", "001")));
    assert_eq!(section_in_name("209940-X-100-001", SHAPE), None);
    assert_eq!(section_in_name("209940-XYZWV-100-001", SHAPE), None);
}

#[test]
fn a_section_is_one_to_four_letters_or_digits() {
    assert_eq!(section_in_name("209940-XYZ-100-A", SHAPE), Some(sref("209940", "XYZ", "100", "A")));
    assert_eq!(section_in_name("209940-XYZ-100-A1B2", SHAPE), Some(sref("209940", "XYZ", "100", "A1B2")));
    assert_eq!(section_in_name("209940-XYZ-100-00001", SHAPE), None);
    assert_eq!(section_in_name("209940-XYZ-100-", SHAPE), None);
}

#[test]
fn an_empty_or_unrelated_name_is_none() {
    assert_eq!(section_in_name("", SHAPE), None);
    assert_eq!(section_in_name("Intro to Invented Things", SHAPE), None);
}

#[test]
fn a_school_row_names_its_shape() {
    assert_eq!(school("ua").unwrap().name_shape, NameShape::TermSubjectNumberSection);
}

// ---- parse_class_search -------------------------------------------------------------------

fn want() -> SectionRef {
    sref("209940", "XYZ", "100", "001")
}

/// A real-shaped meeting with a class type.
fn mt(kind: &str, begin: Value, end: Value, days: &[&str]) -> Value {
    let mut m = json!({ "meetingType": kind, "meetingScheduleType": "LEC", "beginTime": begin, "endTime": end,
        "startDate": "08/17/2099", "endDate": "12/04/2099", "building": "Invented Hall", "room": "12", "hoursWeek": 2.5 });
    for d in ALL_DAYS {
        m[d] = json!(days.contains(&d));
    }
    m
}

fn clas(begin: &str, end: &str, days: &[&str]) -> Value {
    mt("CLAS", json!(begin), json!(end), days)
}

/// A row of `term subject number section` whose meetings are wrapped as R0b recorded them.
fn row(term: &str, subject: &str, number: &str, section: &str, crn: &str, meetings: Vec<Value>) -> Value {
    let faculty: Vec<Value> = meetings.into_iter().map(|m| json!({ "meetingTime": m })).collect();
    json!({ "term": term, "subject": subject, "courseNumber": number, "sequenceNumber": section,
        "courseReferenceNumber": crn, "scheduleTypeDescription": "Lecture", "isSectionLinked": false,
        "linkIdentifier": null, "meetingsFaculty": faculty })
}

fn mine(crn: &str, meetings: Vec<Value>) -> Value {
    row("209940", "XYZ", "100", "001", crn, meetings)
}

fn answer(rows: Vec<Value>) -> Value {
    json!({ "success": true, "totalCount": rows.len(), "data": rows })
}

fn read(rows: Vec<Value>) -> ClassSearchRead {
    parse_class_search(&answer(rows), "ua", &want()).unwrap()
}

#[test]
fn one_section_matched_among_many_is_found() {
    let r = read(vec![
        row("209940", "XYZ", "100", "002", "90002", vec![clas("1000", "1050", &["tuesday"])]),
        mine("90001", vec![clas("0900", "0950", &["monday", "wednesday", "friday"])]),
        row("209940", "XYZ", "100", "003", "90003", vec![clas("1100", "1150", &["thursday"])]),
    ]);
    assert_eq!(r.state, SectionState::Found);
    let s = r.series.expect("a found section carries its series");
    assert!(s.source_uid.starts_with("classsearch:ua:"), "{}", s.source_uid);
    assert!(s.source_uid.ends_with("209940-90001"), "{}", s.source_uid);
    assert_eq!(s.title, "XYZ 100");
    assert_eq!(s.meets.len(), 1);
    assert_eq!(s.meets[0].days, vec!["mon", "wed", "fri"]);
    assert_eq!((s.meets[0].start.hour(), s.meets[0].start.minute()), (9, 0));
    assert_eq!((s.meets[0].end.hour(), s.meets[0].end.minute()), (9, 50));
    assert_eq!((s.first, s.until), (Some(date(2099, 8, 17)), Some(date(2099, 12, 4))));
    assert_eq!(s.where_.as_deref(), Some("Invented Hall 12"));
    assert_eq!(s.event_type.as_deref(), Some(EVENT_TYPE));
    assert!(!r.linked);
    assert_eq!(r.dropped, 0, "another section is not a dropped row");
}

#[test]
fn the_meetings_faculty_wrapper_is_lifted_with_every_meeting() {
    let r = read(vec![mine("90001", vec![clas("0900", "0950", &["monday"]), clas("1300", "1350", &["thursday"])])]);
    assert_eq!(r.state, SectionState::Found);
    assert_eq!(r.series.unwrap().meets.len(), 2);
}

#[test]
fn two_rows_for_one_section_are_ambiguous_and_never_picked() {
    let r = read(vec![
        mine("90001", vec![clas("0900", "0950", &["monday"])]),
        mine("90009", vec![clas("1400", "1450", &["tuesday"])]),
    ]);
    assert_eq!(r.state, SectionState::Ambiguous);
    assert!(r.series.is_none());
}

#[test]
fn rows_with_another_term_subject_or_number_are_dropped_and_counted() {
    let r = read(vec![
        row("209930", "XYZ", "100", "001", "90011", vec![clas("0800", "0850", &["monday"])]),
        row("209940", "QRS", "100", "001", "90012", vec![clas("0800", "0850", &["monday"])]),
        row("209940", "XYZ", "200", "001", "90013", vec![clas("0800", "0850", &["monday"])]),
        mine("90001", vec![clas("0900", "0950", &["tuesday"])]),
    ]);
    assert_eq!(r.state, SectionState::Found, "the stale rows must not make it ambiguous");
    assert_eq!(r.dropped, 3);
    assert!(r.series.unwrap().source_uid.ends_with("90001"));
}

#[test]
fn an_answer_holding_only_foreign_rows_is_not_found() {
    let r = read(vec![row("209940", "QRS", "100", "001", "90012", vec![clas("0800", "0850", &["monday"])])]);
    assert_eq!(r.state, SectionState::NotFound);
    assert_eq!(r.dropped, 1);
    assert!(r.series.is_none());
}

#[test]
fn an_answer_with_no_such_section_is_not_found() {
    let r = read(vec![row("209940", "XYZ", "100", "002", "90002", vec![clas("1000", "1050", &["tuesday"])])]);
    assert_eq!(r.state, SectionState::NotFound);
    assert_eq!(r.dropped, 0);
}

#[test]
fn an_answer_with_no_rows_at_all_is_not_found_never_an_error() {
    let r = read(Vec::new());
    assert_eq!(r.state, SectionState::NotFound);
    assert!(r.series.is_none());
}

#[test]
fn an_answer_with_no_data_list_is_an_error() {
    for v in [json!({ "success": true }), json!({ "data": "x" }), json!([]), json!({ "data": { "rows": [] } })] {
        assert!(parse_class_search(&v, "ua", &want()).is_err(), "{v}");
    }
}

#[test]
fn exam_meetings_are_skipped_and_counted_and_add_no_weekday() {
    let r = read(vec![mine("90001", vec![clas("0900", "0950", &["monday"]), mt("EXAM", json!("1000"), json!("1200"), &["tuesday"])])]);
    assert_eq!(r.state, SectionState::Found);
    assert_eq!(r.exam, 1);
    assert_eq!(r.series.unwrap().meets[0].days, vec!["mon"]);
}

#[test]
fn a_row_with_only_an_exam_is_no_time() {
    let r = read(vec![mine("90001", vec![mt("EXAM", json!("1000"), json!("1200"), &["tuesday"])])]);
    assert_eq!(r.state, SectionState::NoTime);
    assert_eq!(r.exam, 1);
    assert!(r.series.is_none());
}

#[test]
fn a_row_with_no_time_is_no_time_and_nothing_is_filled_in() {
    for meetings in [
        Vec::new(),
        vec![clas("", "", &[])],
        vec![mt("CLAS", Value::Null, Value::Null, &[])],
    ] {
        let r = read(vec![mine("90001", meetings.clone())]);
        assert_eq!(r.state, SectionState::NoTime, "{meetings:?}");
        assert!(r.series.is_none());
    }
}

#[test]
fn a_course_whose_every_row_has_no_time_is_no_failure() {
    let r = parse_class_search(
        &answer(vec![mine("90001", Vec::new()), row("209940", "XYZ", "100", "002", "90002", Vec::new())]),
        "ua",
        &want(),
    );
    assert_eq!(r.unwrap().state, SectionState::NoTime, "the registrar's empty-parse failure does not apply here");
}

#[test]
fn a_row_with_a_meeting_that_cannot_be_read_is_part_read_with_no_series() {
    let r = read(vec![mine("90001", vec![clas("0900", "0950", &["monday"]), clas("1300", "1350", &[])])]);
    assert_eq!(r.state, SectionState::PartRead);
    assert!(r.series.is_none(), "part_read files no proposal");
}

#[test]
fn a_meeting_across_midnight_is_dropped_and_counted() {
    let r = read(vec![mine("90001", vec![clas("2300", "0100", &["friday"])])]);
    assert_eq!(r.midnight, 1);
    assert!(r.series.is_none());
    assert_ne!(r.state, SectionState::Found);
    assert!(r.warnings.iter().any(|w| w.contains("XYZ 100") && w.contains("midnight")), "{:?}", r.warnings);
}

#[test]
fn a_lab_is_titled_by_its_schedule_type() {
    let mut lab = mine("90001", vec![clas("1300", "1550", &["wednesday"])]);
    lab["scheduleTypeDescription"] = json!("Laboratory");
    let r = read(vec![lab]);
    assert_eq!(r.series.unwrap().title, "XYZ 100 Lab");
}

#[test]
fn a_linked_section_carries_linked_true() {
    let mut linked = mine("90001", vec![clas("0900", "0950", &["monday"])]);
    linked["isSectionLinked"] = json!(true);
    linked["linkIdentifier"] = json!("A1");
    let r = read(vec![linked]);
    assert_eq!(r.state, SectionState::Found);
    assert!(r.linked);
}

#[test]
fn a_school_with_a_registered_set_still_reads_rows_that_carry_no_status() {
    let a = answer(vec![mine("90001", vec![clas("0900", "0950", &["monday"])])]);
    let r = parse_class_search_with(&a, "ua", &["IR", "RW"], &want()).unwrap();
    assert_eq!(r.state, SectionState::Found);
    assert!(r.series.is_some());
}

#[test]
fn the_same_rows_in_another_order_give_the_same_read() {
    let a = row("209940", "XYZ", "100", "002", "90002", vec![clas("1000", "1050", &["tuesday"])]);
    let b = mine("90001", vec![clas("0900", "0950", &["monday"])]);
    let c = row("209930", "XYZ", "100", "001", "90011", vec![clas("0800", "0850", &["monday"])]);
    assert_eq!(read(vec![a.clone(), b.clone(), c.clone()]), read(vec![c, b, a]));
}
