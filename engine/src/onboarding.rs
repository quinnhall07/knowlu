//! `knowlu-engine onboarding` (Wave B plan §4.4, F7a). Read-only: step 3's counts from raw captures.
//! STUB: F7a's tests are written against these signatures; the implementation is F7a part 2.

use jiff::civil::DateTime;
use jiff::tz::TimeZone;
use serde_json::Value;

/// The raw captures `--preview` reads, as text. `None` is an input that was not given.
#[derive(Debug, Default, Clone)]
pub struct PreviewInputs<'a> {
    /// The LMS calendar feed (ICS text).
    pub feed: Option<&'a str>,
    /// The captured course list: a JSON array of `{code, name, slug, label}`.
    pub courses: Option<&'a str>,
    /// The v2 grades bundle (`grades::Bundle` JSON).
    pub grades: Option<&'a str>,
}

/// Step 3's counts. Keys: `classes`, `class_times`, `assignments_upcoming`, `turned_in`,
/// `grades_courses` (each a number or `null`) and `errors` (an array of lines).
pub fn preview(_inputs: &PreviewInputs, _tz: &TimeZone, _now: DateTime) -> Value {
    Value::Null
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    const GB: &str = "_blackboard.platform.gradebook2.GradableItem-";

    fn chicago() -> TimeZone {
        TimeZone::get("America/Chicago").unwrap()
    }

    /// Invented "now": 2026-09-01 12:00 in the vault's zone.
    fn now() -> DateTime {
        DateTime::constant(2026, 9, 1, 12, 0, 0, 0)
    }

    fn vevent(uid: &str, title: &str, dtstart: &str) -> String {
        format!("BEGIN:VEVENT\nUID:{uid}\nSUMMARY:{title}\nDESCRIPTION:from the feed\nDTSTART:{dtstart}\nEND:VEVENT\n")
    }

    fn ics(events: &[String]) -> String {
        format!("BEGIN:VCALENDAR\nVERSION:2.0\n{}END:VCALENDAR\n", events.concat())
    }

    fn courses() -> String {
        json!([
            {"code": "_900001_1", "name": "Synthetic Studies I", "slug": "syn-110", "label": "SYN 110"},
            {"code": "_900002_1", "name": "Synthetic Studies II", "slug": "syn-120", "label": "SYN 120"},
        ])
        .to_string()
    }

    /// A v2 grades bundle, invented. Course one: item columns `_71_1` (graded 8/10), `_72_1`
    /// (needs grading), `_73_1` (no row: not submitted), plus an Overall column `_79_1` with a
    /// graded row. Course two: only an Overall column. Course three: a failed capture.
    fn bundle() -> String {
        let item = |id: &str, name: &str| json!({"id": id, "name": name, "externalGrade": false, "score": {"possible": 10.0}});
        let row = |id: &str, status: &str, score: Option<f64>| match score {
            Some(s) => json!({"columnId": id, "status": status, "exempt": false,
                              "displayGrade": {"scaleType": "Score", "score": s, "possible": 10.0}}),
            None => json!({"columnId": id, "status": status, "exempt": false}),
        };
        let overall = json!({"id": "_79_1", "name": "Overall Grade", "externalGrade": true});
        json!({
            "schema": 1, "lms": "blackboard", "host": "lms.example.test",
            "fetched_at": "2026-08-30T12:00:00Z", "user_id": "_900_1",
            "courses": [
                {"membership": {"courseId": "_900001_1"},
                 "columns": [item("_71_1", "Quiz A"), item("_72_1", "Essay B"), item("_73_1", "Lab C"), overall],
                 "grades": [row("_71_1", "Graded", Some(8.0)), row("_72_1", "NeedsGrading", None),
                            row("_79_1", "Graded", Some(8.0))],
                 "categories": null},
                {"membership": {"courseId": "_900002_1"},
                 "columns": [{"id": "_89_1", "name": "Overall Grade", "externalGrade": true}],
                 "grades": [], "categories": null},
                {"membership": {"courseId": "_900003_1"}, "columns": [], "grades": [], "categories": null, "error": "403"},
            ],
        })
        .to_string()
    }

    fn run(feed: &str, grades: Option<&str>) -> Value {
        let c = courses();
        let inputs = PreviewInputs { feed: Some(feed), courses: Some(&c), grades };
        preview(&inputs, &chicago(), now())
    }

    fn count(v: &Value, key: &str) -> Value {
        v.get(key).cloned().unwrap_or_else(|| panic!("no key {key:?} in {v}"))
    }

    #[test]
    fn classes_are_the_course_list_length() {
        let v = run(&ics(&[]), None);
        assert_eq!(count(&v, "classes"), json!(2), "{v}");
    }

    #[test]
    fn only_items_due_after_now_in_the_vault_zone_are_upcoming() {
        let feed = ics(&[
            vevent("BB-ASSIGN-1", "Past thing", "20260820T235900"),
            vevent("BB-ASSIGN-2", "Soon thing", "20260905T235900"),
            vevent("BB-ASSIGN-3", "Later thing", "20260910T080000"),
        ]);
        let v = run(&feed, None);
        assert_eq!(count(&v, "assignments_upcoming"), json!(2), "{v}");
    }

    #[test]
    fn upcoming_is_judged_in_the_vault_zone_not_utc() {
        // 2026-09-02 02:00 UTC is 2026-09-01 21:00 in Chicago (CDT): after a 12:00 now, so upcoming.
        // 2026-09-01 15:00 UTC is 10:00 in Chicago: already past, though 15:00 > 12:00 read as UTC.
        let feed = ics(&[
            vevent("BB-ASSIGN-1", "Evening thing", "20260902T020000Z"),
            vevent("BB-ASSIGN-2", "Morning thing", "20260901T150000Z"),
        ]);
        let v = run(&feed, None);
        assert_eq!(count(&v, "assignments_upcoming"), json!(1), "{v}");
    }

    #[test]
    fn turned_in_joins_by_gradable_id_and_a_done_row() {
        let feed = ics(&[
            vevent(&format!("{GB}_71_1"), "Renamed in the feed", "20260905T235900"), // graded 8 of 10: done
            vevent(&format!("{GB}_72_1"), "Essay B", "20260906T235900"),             // needs grading: done
            vevent(&format!("{GB}_73_1"), "Lab C", "20260907T235900"),               // no row: live
            vevent(&format!("{GB}_74_1"), "Unknown column", "20260908T235900"),      // no column: live
        ]);
        let v = run(&feed, Some(&bundle()));
        assert_eq!(count(&v, "assignments_upcoming"), json!(4), "{v}");
        assert_eq!(count(&v, "turned_in"), json!(2), "{v}");
    }

    #[test]
    fn a_title_match_alone_is_never_turned_in() {
        // "Quiz A" is a done column's name, but this event's UID is not a gradebook UID.
        let feed = ics(&[
            vevent("BB-ASSIGN-9", "Quiz A", "20260905T235900"),
            vevent("cal-entry-1", "Essay B", "20260906T235900"),
        ]);
        let v = run(&feed, Some(&bundle()));
        assert_eq!(count(&v, "turned_in"), json!(0), "{v}");
        assert_eq!(count(&v, "assignments_upcoming"), json!(2), "{v}");
    }

    #[test]
    fn turned_in_counts_upcoming_items_only() {
        let feed = ics(&[vevent(&format!("{GB}_71_1"), "Quiz A", "20260820T235900")]);
        let v = run(&feed, Some(&bundle()));
        assert_eq!(count(&v, "assignments_upcoming"), json!(0), "{v}");
        assert_eq!(count(&v, "turned_in"), json!(0), "{v}");
    }

    #[test]
    fn an_overall_column_counts_no_extra_item() {
        // The feed carries an event whose id is the Overall column's; its graded row is not an item.
        let with = ics(&[
            vevent(&format!("{GB}_71_1"), "Quiz A", "20260905T235900"),
            vevent(&format!("{GB}_79_1"), "Overall Grade", "20260906T235900"),
        ]);
        let v = run(&with, Some(&bundle()));
        assert_eq!(count(&v, "turned_in"), json!(1), "the Overall row must not join: {v}");
        // And the bundle alone, with no feed event for it, adds nothing to either count.
        let without = ics(&[vevent(&format!("{GB}_71_1"), "Quiz A", "20260905T235900")]);
        let w = run(&without, Some(&bundle()));
        assert_eq!(count(&w, "turned_in"), json!(1), "{w}");
        assert_eq!(count(&w, "assignments_upcoming"), json!(1), "{w}");
    }

    #[test]
    fn grades_courses_are_courses_with_an_item_column() {
        // Course one has item columns; course two only an Overall; course three failed.
        let v = run(&ics(&[]), Some(&bundle()));
        assert_eq!(count(&v, "grades_courses"), json!(1), "{v}");
    }

    #[test]
    fn a_missing_grades_bundle_is_null_not_zero() {
        let feed = ics(&[vevent(&format!("{GB}_71_1"), "Quiz A", "20260905T235900")]);
        let v = run(&feed, None);
        assert_eq!(count(&v, "turned_in"), Value::Null, "{v}");
        assert_eq!(count(&v, "grades_courses"), Value::Null, "{v}");
        assert_eq!(count(&v, "assignments_upcoming"), json!(1), "{v}");
    }

    #[test]
    fn an_unreadable_input_is_a_null_field_and_a_named_error() {
        let inputs = PreviewInputs { feed: Some("this is not a calendar"), courses: Some("{not json"), grades: Some("[]") };
        let v = preview(&inputs, &chicago(), now());
        assert_eq!(count(&v, "classes"), Value::Null, "{v}");
        assert_eq!(count(&v, "grades_courses"), Value::Null, "{v}");
        assert_eq!(count(&v, "turned_in"), Value::Null, "{v}");
        let errors = count(&v, "errors");
        assert!(errors.as_array().is_some_and(|e| e.len() >= 2), "{v}");
    }

    #[test]
    fn a_clean_run_has_an_empty_errors_array() {
        let v = run(&ics(&[]), Some(&bundle()));
        assert_eq!(count(&v, "errors"), json!([]), "{v}");
    }

    #[test]
    fn the_same_input_gives_the_same_bytes_with_sorted_keys() {
        let feed = ics(&[vevent(&format!("{GB}_71_1"), "Quiz A", "20260905T235900")]);
        let g = bundle();
        let a = crate::ledger::dumps_value(&run(&feed, Some(&g)));
        let b = crate::ledger::dumps_value(&run(&feed, Some(&g)));
        assert_eq!(a, b);
        let at = |k: &str| a.find(&format!("\"{k}\"")).unwrap_or_else(|| panic!("no {k} in {a}"));
        assert!(at("assignments_upcoming") < at("classes") && at("classes") < at("turned_in"), "{a}");
    }

    /// The module is read-only by construction: nothing above `mod tests` names a writer or a socket.
    #[test]
    fn the_module_has_no_writer_and_no_socket() {
        let source = include_str!("onboarding.rs");
        let code = &source[..source.find("#[cfg(test)]").unwrap()];
        for banned in [
            "TcpStream", "TcpListener", "UdpSocket", "ureq", "fetch_ics", "cloudmodel", "entitle::",
            "fs::write", "File::create", "OpenOptions", "create_dir", "remove_file", "pystr::write_text",
            "crate::write", "Journal",
        ] {
            assert!(!code.contains(banned), "onboarding.rs must not use {banned}");
        }
    }
}
