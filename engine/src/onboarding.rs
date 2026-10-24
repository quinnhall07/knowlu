//! `knowlu-engine onboarding` (Wave B plan §4.4, F7a). Read-only: step 3's counts from raw captures.
//!
//! Arithmetic over files: it writes nothing, opens no socket, and an input it cannot read is a
//! `null` count and a named line in `errors`, never a failure.

use std::collections::BTreeSet;
use std::path::PathBuf;

use jiff::civil::DateTime;
use jiff::tz::TimeZone;
use serde_json::{json, Value};

use crate::ingest::{self, Due};

/// The raw captures `--preview` reads, as text. `None` is an input that was not given.
#[derive(Debug, Default, Clone)]
pub struct PreviewInputs<'a> {
    /// The LMS calendar feed (ICS text).
    pub feed: Option<&'a str>,
    /// The captured course list: a JSON array of `{code, name, slug, label}`.
    pub courses: Option<&'a str>,
    /// The v2 grades bundle (`grades::Bundle` JSON).
    pub grades: Option<&'a str>,
    /// The registrar's raw answer (Banner JSON, as `commitments --registrar` reads it).
    pub registrar: Option<&'a str>,
    /// The registrar's school key (`registrar::SCHOOLS`); `registrar` is unreadable without it.
    pub school: Option<&'a str>,
}

/// Step 3's counts. Keys: `classes`, `class_times`, `assignments_upcoming`, `turned_in`,
/// `grades_courses` (each a number or `null`) and `errors` (an array of lines).
pub fn preview(inputs: &PreviewInputs, tz: &TimeZone, now: DateTime) -> Value {
    preview_inner(inputs, &[], tz, now)
}

/// A course code with its separators dropped: `SYN 110`, `syn-110`, `SYN110` all read `SYN110`.
fn compact(code: &str) -> String {
    code.chars().filter(|c| c.is_ascii_alphanumeric()).map(|c| c.to_ascii_uppercase()).collect()
}

/// An input that is required must be given and readable; `unread` names the ones whose file
/// could not be read, which are an error of their own, not "not given".
fn input<'a>(name: &str, text: Option<&'a str>, unread: &[&str], required: bool, errors: &mut Vec<String>) -> Option<&'a str> {
    if unread.contains(&name) {
        errors.push(format!("{name}: the file could not be read"));
        return None;
    }
    if text.is_none() && required {
        errors.push(format!("{name}: not given"));
    }
    text
}

/// The course list: its length and the codes its labels and slugs compact to.
fn read_courses(text: &str, errors: &mut Vec<String>) -> Option<(usize, BTreeSet<String>)> {
    let Some(list) = serde_json::from_str::<Value>(text).ok().and_then(|v| v.as_array().cloned()) else {
        errors.push("courses: not a JSON array of courses".to_string());
        return None;
    };
    let mut codes = BTreeSet::new();
    for course in &list {
        for key in ["label", "slug"] {
            if let Some(code) = course.get(key).and_then(Value::as_str).map(compact).filter(|c| !c.is_empty()) {
                codes.insert(code);
            }
        }
    }
    Some((list.len(), codes))
}

/// The distinct UIDs of the feed's items due after `now` in the vault's zone. A date-only due on
/// today's date is today's work (ingest's rule), so it counts. Institutional noise and an event
/// with no due are not items. `None` when the text is not a calendar.
fn read_feed(text: &str, tz: &TimeZone, now: DateTime, errors: &mut Vec<String>) -> Option<Vec<String>> {
    if !text.contains("BEGIN:VCALENDAR") {
        errors.push("feed: not a calendar".to_string());
        return None;
    }
    let (events, _malformed) = ingest::parse_ics(text, tz);
    let upcoming: BTreeSet<String> = events
        .into_iter()
        .filter(|e| !ingest::is_institutional_noise(&e.uid))
        .filter(|e| match e.due {
            Some(Due::DateTime(dt)) => dt > now,
            Some(Due::Date(d)) => d >= now.date(),
            None => false,
        })
        .map(|e| e.uid)
        .collect();
    Some(upcoming.into_iter().collect())
}

/// From the v2 bundle: the item columns whose row says turned in (O1's rule), and how many courses
/// have at least one item column. An Overall column is neither; a course whose capture failed has
/// no columns to count.
fn read_grades(text: &str, tz: &TimeZone, errors: &mut Vec<String>) -> Option<(BTreeSet<String>, usize)> {
    let Ok(bundle) = serde_json::from_str::<crate::grades::Bundle>(text) else {
        errors.push("grades: not a v2 grades bundle".to_string());
        return None;
    };
    let (mut done, mut courses) = (BTreeSet::new(), 0);
    for course in bundle.courses.iter().filter(|c| c.error.is_none()) {
        let mut items = 0;
        for column in &course.columns {
            let Some(id) = column["id"].as_str().filter(|s| !s.is_empty()) else { continue };
            let row = course.grades.iter().find(|r| r["columnId"].as_str() == Some(id));
            let fields = crate::grades::column_to_fields(column, row, tz);
            if fields.kind != crate::grades::Kind::Item {
                continue;
            }
            items += 1;
            if ingest::gradebook_done(fields.status.as_str(), fields.score) {
                done.insert(id.to_string());
            }
        }
        courses += usize::from(items > 0);
    }
    Some((done, courses))
}

/// The registrar's matched class times: Banner's series whose course code (a lab's " Lab" dropped)
/// is a code in the course list. Nothing is written and nothing is fetched.
fn read_registrar(text: &str, school: Option<&str>, codes: &BTreeSet<String>, errors: &mut Vec<String>) -> Option<usize> {
    let Some(key) = school else {
        errors.push("registrar: no school given".to_string());
        return None;
    };
    if crate::registrar::school(key).is_none() {
        errors.push("registrar: unknown school".to_string());
        return None;
    }
    let Ok(json) = serde_json::from_str::<Value>(text) else {
        errors.push("registrar: not JSON".to_string());
        return None;
    };
    match crate::registrar::parse_banner(&json, key) {
        Ok(parsed) => Some(
            parsed.series.iter().filter(|s| codes.contains(&compact(s.title.strip_suffix(" Lab").unwrap_or(&s.title)))).count(),
        ),
        Err(e) => {
            errors.push(format!("registrar: {e}"));
            None
        }
    }
}

fn preview_inner(inputs: &PreviewInputs, unread: &[&str], tz: &TimeZone, now: DateTime) -> Value {
    let mut errors = Vec::new();
    let feed = input("feed", inputs.feed, unread, true, &mut errors).and_then(|t| read_feed(t, tz, now, &mut errors));
    let courses = input("courses", inputs.courses, unread, true, &mut errors).and_then(|t| read_courses(t, &mut errors));
    let grades = input("grades", inputs.grades, unread, false, &mut errors).and_then(|t| read_grades(t, tz, &mut errors));
    let registrar = input("registrar", inputs.registrar, unread, false, &mut errors);
    let class_times = match (registrar, &courses) {
        (Some(text), Some((_, codes))) => read_registrar(text, inputs.school, codes, &mut errors),
        _ => None,
    };
    let turned_in = match (&feed, &grades) {
        (Some(uids), Some((done, _))) => {
            Some(uids.iter().filter(|u| ingest::gradable_id(u).is_some_and(|id| done.contains(id))).count())
        }
        _ => None,
    };
    json!({
        "assignments_upcoming": feed.as_ref().map(Vec::len),
        "class_times": class_times,
        "classes": courses.as_ref().map(|c| c.0),
        "errors": errors,
        "grades_courses": grades.as_ref().map(|g| g.1),
        "turned_in": turned_in,
    })
}

/// The files `--preview` reads, by path (the command's arguments).
#[derive(Debug, Default, Clone)]
pub struct PreviewFiles {
    pub feed: Option<PathBuf>,
    pub courses: Option<PathBuf>,
    pub grades: Option<PathBuf>,
    pub registrar: Option<PathBuf>,
    pub school: Option<String>,
}

/// `--preview`: read the files (an unreadable one is a named error and a `null`), count, and return
/// the one JSON line, through `ledger::dumps_value`.
pub fn run_preview(files: &PreviewFiles, tz: &TimeZone, now: DateTime) -> String {
    let mut unread = Vec::new();
    let mut read = |name: &'static str, path: &Option<PathBuf>| {
        let path = path.as_ref()?;
        match std::fs::read_to_string(path) {
            Ok(text) => Some(text),
            Err(_) => {
                unread.push(name);
                None
            }
        }
    };
    let (feed, courses) = (read("feed", &files.feed), read("courses", &files.courses));
    let (grades, registrar) = (read("grades", &files.grades), read("registrar", &files.registrar));
    let inputs = PreviewInputs {
        feed: feed.as_deref(),
        courses: courses.as_deref(),
        grades: grades.as_deref(),
        registrar: registrar.as_deref(),
        school: files.school.as_deref(),
    };
    crate::ledger::dumps_value(&preview_inner(&inputs, &unread, tz, now))
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
        let inputs = PreviewInputs { feed: Some(feed), courses: Some(&c), grades, ..Default::default() };
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

    /// A Banner answer, invented: SYN 110 meets (matches the course list), ZZT 999 meets (no such
    /// course), SYN 120 is online with no meeting time (no class time to count).
    fn banner() -> String {
        let row = |crn: &str, subject: &str, number: &str, meetings: Value| {
            json!({"term": "202640", "courseReferenceNumber": crn, "subject": subject, "courseNumber": number,
                   "scheduleDescription": "Lecture", "statusIndicator": "IR", "meetingTimes": meetings})
        };
        let meets = json!([{"meetingType": "CLAS", "monday": true, "wednesday": true, "beginTime": "0900",
                            "endTime": "0950", "startDate": "08/20/2026", "endDate": "12/05/2026"}]);
        json!({"data": {"registrations": [
            row("90001", "SYN", "110", meets.clone()),
            row("90002", "ZZT", "999", meets),
            row("90003", "SYN", "120", json!([])),
        ]}})
        .to_string()
    }

    fn with_registrar(registrar: Option<&str>, school: Option<&str>) -> Value {
        let c = courses();
        let f = ics(&[]);
        let inputs = PreviewInputs { feed: Some(&f), courses: Some(&c), registrar, school, ..Default::default() };
        preview(&inputs, &chicago(), now())
    }

    #[test]
    fn class_times_count_registrar_series_that_name_a_course_in_the_list() {
        let v = with_registrar(Some(&banner()), Some("ua"));
        assert_eq!(count(&v, "class_times"), json!(1), "{v}");
        assert_eq!(count(&v, "errors"), json!([]), "{v}");
    }

    #[test]
    fn no_registrar_is_a_null_class_times_and_no_error() {
        let v = with_registrar(None, None);
        assert_eq!(count(&v, "class_times"), Value::Null, "{v}");
        assert_eq!(count(&v, "errors"), json!([]), "{v}");
    }

    #[test]
    fn an_unreadable_registrar_or_a_missing_school_is_null_and_a_named_error() {
        let b = banner();
        for (registrar, school) in [(Some("not json"), Some("ua")), (Some(b.as_str()), None), (Some(b.as_str()), Some("zz"))] {
            let v = with_registrar(registrar, school);
            assert_eq!(count(&v, "class_times"), Value::Null, "{v}");
            let errors = count(&v, "errors");
            assert!(errors.as_array().is_some_and(|e| e.len() == 1 && e[0].as_str().is_some_and(|s| s.contains("registrar"))), "{v}");
        }
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
        let inputs = PreviewInputs { feed: Some("this is not a calendar"), courses: Some("{not json"), grades: Some("[]"), ..Default::default() };
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
