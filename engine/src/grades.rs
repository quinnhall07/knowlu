//! Grades from Blackboard, the pure half (M1 spec 2026-09-29, §5, §6 and §8).
//!
//! The app captures Blackboard's gradebook responses verbatim into a bundle (§5); this module
//! reads that bundle, decides which vault course each captured course is, turns one gradebook
//! column plus the student's row for it into the fields of a `grades/` note (§6), and computes a
//! course's standing from its notes (§8, G2, G3).
//!
//! Pure: no I/O, no clock, no network, no model. The apply half (`write::create` and
//! `write::write` through the journal) is a later task and lives beside these functions. Rows
//! stay `serde_json::Value`, verbatim, so this module owns every interpretation and a fixture
//! recorded from a real account replays through the same code.

use jiff::tz::TimeZone;
use jiff::Timestamp;
use serde::Deserialize;
use serde_json::Value;

/// The letter scale's minus sign is U+2212, the character the spec (G3) writes; the tests build
/// the expected letters from it.
#[cfg(test)]
const MINUS: char = '\u{2212}';

/// The capture bundle, version 1 (spec §5).
#[derive(Debug, Clone, Deserialize, PartialEq)]
pub struct Bundle {
    pub schema: u32,
    pub lms: String,
    pub host: String,
    pub fetched_at: String,
    pub user_id: String,
    pub courses: Vec<BundleCourse>,
}

/// One captured course. Every list is verbatim from the LMS; `error` is `"<status or reason>"`
/// for a course whose calls failed (its lists are then empty), and `categories` is `None` when
/// the category list was refused, which only drops the grouping.
#[derive(Debug, Clone, Deserialize, PartialEq)]
pub struct BundleCourse {
    pub membership: Value,
    #[serde(default)]
    pub columns: Vec<Value>,
    #[serde(default)]
    pub grades: Vec<Value>,
    #[serde(default)]
    pub categories: Option<Vec<Value>>,
    #[serde(default)]
    pub error: Option<String>,
}

/// A vault course note, reduced to what matching needs: its slug and its `code:`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CourseRef {
    pub slug: String,
    pub code: String,
}

/// `kind:` of a grade note (spec §6).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Kind {
    Item,
    Overall,
}

/// `status:` of a grade note (spec §6).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Status {
    Graded,
    NeedsGrading,
    InProgress,
    NotSubmitted,
    Exempt,
    Removed,
}

/// The fields one gradebook column yields (spec §6), less the ones the caller knows: `id`,
/// `type`, `course` and `source`. `category` is filled from `category_title`.
#[derive(Debug, Clone, PartialEq)]
pub struct GradeFields {
    pub title: String,
    pub source_uid: String,
    pub kind: Kind,
    pub possible: Option<f64>,
    pub score: Option<f64>,
    pub status: Status,
    pub counts: bool,
    pub category: Option<String>,
    pub due: Option<String>,
}

/// A grade note as `course_grade` reads it.
#[derive(Debug, Clone, PartialEq)]
pub struct GradeNote {
    pub kind: Kind,
    pub score: Option<f64>,
    pub possible: Option<f64>,
    pub status: Status,
    pub counts: bool,
}

/// Which number the ring shows (spec §8).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Basis {
    Overall,
    Points,
    None,
}

/// A course's standing, computed from its notes (spec §8). `earned` and `possible` are the
/// points-so-far sums whatever the basis.
#[derive(Debug, Clone, PartialEq)]
pub struct Standing {
    pub basis: Basis,
    pub pct: Option<f64>,
    pub letter: Option<&'static str>,
    pub family: Option<char>,
    pub earned: f64,
    pub possible: f64,
    pub graded: usize,
    pub items: usize,
    pub pending: usize,
}

impl Kind {
    pub fn as_str(self) -> &'static str {
        match self {
            Kind::Item => "item",
            Kind::Overall => "overall",
        }
    }
    pub fn parse(text: &str) -> Option<Kind> {
        match text {
            "item" => Some(Kind::Item),
            "overall" => Some(Kind::Overall),
            _ => None,
        }
    }
}

impl Status {
    pub fn as_str(self) -> &'static str {
        match self {
            Status::Graded => "graded",
            Status::NeedsGrading => "needs-grading",
            Status::InProgress => "in-progress",
            Status::NotSubmitted => "not-submitted",
            Status::Exempt => "exempt",
            Status::Removed => "removed",
        }
    }
    pub fn parse(text: &str) -> Option<Status> {
        [
            Status::Graded,
            Status::NeedsGrading,
            Status::InProgress,
            Status::NotSubmitted,
            Status::Exempt,
            Status::Removed,
        ]
        .into_iter()
        .find(|s| s.as_str() == text)
    }
}

/// A bundle course's vault course: `membership.courseId`, else `course.courseId`, else
/// `course.id`, each against the notes' `code:`; no match is `None`, never a guess (spec §6).
pub fn match_course<'a>(course: &BundleCourse, notes: &'a [CourseRef]) -> Option<&'a CourseRef> {
    let m = &course.membership;
    [&m["courseId"], &m["course"]["courseId"], &m["course"]["id"]]
        .into_iter()
        .filter_map(Value::as_str)
        .filter(|key| !key.is_empty())
        .find_map(|key| notes.iter().find(|n| n.code == key))
}

/// One column and the student's row for it (`None` when Blackboard has no row: an unattempted
/// item) as the fields of a grade note. `due` is converted into `zone`, the vault's.
pub fn column_to_fields(column: &Value, row: Option<&Value>, zone: &TimeZone) -> GradeFields {
    let text = |v: &Value| v.as_str().unwrap_or("").to_string();
    let score = row.and_then(|r| r["score"].as_f64());
    let status = match row {
        None => Status::NotSubmitted,
        Some(r) if r["exempt"].as_bool() == Some(true) => Status::Exempt,
        Some(_) if score.is_some() => Status::Graded,
        Some(r) => match r["status"].as_str() {
            Some("NeedsGrading") => Status::NeedsGrading,
            // Complete/Incomplete or letter-only column the instructor has graded: graded, no score.
            Some("Graded") => Status::Graded,
            _ => Status::InProgress,
        },
    };
    GradeFields {
        title: text(&column["name"]),
        source_uid: text(&column["id"]),
        kind: if column["externalGrade"].as_bool() == Some(true) { Kind::Overall } else { Kind::Item },
        possible: column["score"]["possible"].as_f64(),
        score,
        status,
        counts: column["includeInCalculations"].as_bool().unwrap_or(true),
        category: None,
        due: column["grading"]["due"].as_str().and_then(|raw| due_in(raw, zone)),
    }
}

/// An ISO instant (`2026-09-05T04:59:00.000Z`) as the vault writes a due: local wall time, minutes.
fn due_in(raw: &str, zone: &TimeZone) -> Option<String> {
    let instant: Timestamp = raw.parse().ok()?;
    let local = instant.to_zoned(zone.clone()).datetime();
    Some(crate::ingest::format_due(crate::ingest::Due::DateTime(local)))
}

/// The title of the category a column belongs to, when the category list came back and names it.
pub fn category_title(column: &Value, categories: Option<&[Value]>) -> Option<String> {
    let id = column["gradebookCategoryId"].as_str()?;
    categories?
        .iter()
        .find(|c| c["id"].as_str() == Some(id))
        .and_then(|c| c["title"].as_str())
        .map(str::to_string)
}

/// Longest slug part: with `grades/`, the joining dash and `.md` a path stays under 140 bytes.
const SLUG_MAX: usize = 60;

/// `[a-z0-9-]` only, runs of anything else one dash, trimmed, bounded; `fallback` when nothing is left.
fn slug_part(text: &str, fallback: &str) -> String {
    let mut out = String::new();
    for c in text.chars() {
        if c.is_ascii_alphanumeric() {
            out.push(c.to_ascii_lowercase());
        } else if !out.ends_with('-') {
            out.push('-');
        }
    }
    let capped: String = out.trim_matches('-').chars().take(SLUG_MAX).collect();
    let capped = capped.trim_end_matches('-');
    if capped.is_empty() { fallback.to_string() } else { capped.to_string() }
}

/// `grades/<course-slug>-<column-id-slug>.md`, both parts reduced to `[a-z0-9-]` and bounded, so
/// a hostile column id cannot name a path outside `grades/` (`_4686399_1` → `4686399-1`).
pub fn note_path(course_slug: &str, column_id: &str) -> String {
    format!("grades/{}-{}.md", slug_part(course_slug, "course"), slug_part(column_id, "item"))
}

/// G3's default scale: 93 A, 90 A−, 87 B+, 83 B, 80 B−, 77 C+, 73 C, 70 C−, 67 D+, 63 D, 60 D−,
/// F below. Anything from 93 up, 100 and over included, is an A.
pub fn letter(pct: f64) -> &'static str {
    const SCALE: [(f64, &str); 11] = [
        (93.0, "A"),
        (90.0, "A\u{2212}"),
        (87.0, "B+"),
        (83.0, "B"),
        (80.0, "B\u{2212}"),
        (77.0, "C+"),
        (73.0, "C"),
        (70.0, "C\u{2212}"),
        (67.0, "D+"),
        (63.0, "D"),
        (60.0, "D\u{2212}"),
    ];
    SCALE.iter().find(|(floor, _)| pct >= *floor).map_or("F", |(_, l)| l)
}

/// A course's standing from its grade notes (spec §8).
pub fn course_grade(notes: &[GradeNote]) -> Standing {
    let items: Vec<&GradeNote> = notes
        .iter()
        .filter(|n| n.kind == Kind::Item && n.status != Status::Removed)
        .collect();
    let (mut earned, mut possible) = (0.0, 0.0);
    for n in &items {
        if let (Status::Graded, true, Some(score), Some(out_of)) = (n.status, n.counts, n.score, n.possible) {
            if out_of > 0.0 {
                earned += score;
                possible += out_of;
            }
        }
    }
    let overall = notes.iter().find_map(|n| match (n.kind, n.status, n.score, n.possible) {
        (Kind::Overall, Status::Graded, Some(score), Some(out_of)) if out_of > 0.0 => Some(score * 100.0 / out_of),
        _ => None,
    });
    let (basis, pct) = match overall {
        Some(pct) => (Basis::Overall, Some(pct)),
        None if possible > 0.0 => (Basis::Points, Some(earned * 100.0 / possible)),
        None => (Basis::None, None),
    };
    let letter = pct.map(letter);
    Standing {
        basis,
        pct,
        letter,
        family: letter.and_then(|l| l.chars().next()),
        earned,
        possible,
        graded: items.iter().filter(|n| n.status == Status::Graded).count(),
        items: items.len(),
        pending: items
            .iter()
            .filter(|n| matches!(n.status, Status::NeedsGrading | Status::InProgress | Status::NotSubmitted))
            .count(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    const BASIC: &str = include_str!("../tests/fixtures/grades/bundle-basic.json");

    fn bundle() -> Bundle {
        serde_json::from_str(BASIC).expect("bundle-basic.json is spec 5's shape")
    }

    fn chicago() -> TimeZone {
        TimeZone::get("America/Chicago").unwrap()
    }

    fn row_for<'a>(course: &'a BundleCourse, column_id: &str) -> Option<&'a Value> {
        course.grades.iter().find(|r| r["columnId"] == column_id)
    }

    fn fields_of(course: &BundleCourse, column_id: &str) -> GradeFields {
        let column = course.columns.iter().find(|c| c["id"] == column_id).unwrap();
        column_to_fields(column, row_for(course, column_id), &chicago())
    }

    fn crefs() -> Vec<CourseRef> {
        vec![
            CourseRef { slug: "syn-110".into(), code: "_900110_1".into() },
            CourseRef { slug: "syn-330".into(), code: "SYN-330-002-FA2026".into() },
            CourseRef { slug: "other".into(), code: "_1_1".into() },
        ]
    }

    fn note(kind: Kind, score: Option<f64>, possible: Option<f64>, status: Status) -> GradeNote {
        GradeNote { kind, score, possible, status, counts: true }
    }

    // ---- the bundle ----

    #[test]
    fn the_bundle_deserializes_spec_5_and_keeps_rows_verbatim() {
        let b = bundle();
        assert_eq!((b.schema, b.lms.as_str(), b.host.as_str()), (1, "blackboard", "lms.example.test"));
        assert_eq!(b.fetched_at, "2026-09-29T14:02:11Z");
        assert_eq!(b.courses.len(), 3);
        let first = &b.courses[0];
        assert_eq!(first.columns.len(), 7);
        assert_eq!(first.grades[0]["displayGrade"]["text"], "18.00", "a row is the LMS's own object");
        assert_eq!(first.categories.as_ref().map(Vec::len), Some(3));
        assert_eq!(first.error, None);
        assert_eq!(b.courses[1].error.as_deref(), Some("403"));
        assert!(b.courses[1].columns.is_empty());
    }

    #[test]
    fn a_refused_category_list_is_none() {
        let course: BundleCourse = serde_json::from_value(
            json!({"membership": {}, "columns": [], "grades": [], "categories": null, "error": null}),
        )
        .unwrap();
        assert_eq!(course.categories, None);
    }

    // ---- matching ----

    #[test]
    fn a_course_matches_by_membership_course_id_then_course_course_id_then_course_id() {
        let notes = crefs();
        let by = |m: Value| -> Option<String> {
            let c = BundleCourse { membership: m, columns: vec![], grades: vec![], categories: None, error: None };
            match_course(&c, &notes).map(|n| n.slug.clone())
        };
        // the fixture's first course: membership.courseId is the note's code
        assert_eq!(by(bundle().courses[0].membership.clone()).as_deref(), Some("syn-110"));
        // the external key only
        assert_eq!(
            by(json!({"courseId": "_zzz_1", "course": {"courseId": "SYN-330-002-FA2026", "id": "_q_1"}})).as_deref(),
            Some("syn-330")
        );
        // the internal id only
        assert_eq!(by(json!({"courseId": "_zzz_1", "course": {"courseId": "no", "id": "_1_1"}})).as_deref(), Some("other"));
        // membership.courseId wins over the two below it
        assert_eq!(
            by(json!({"courseId": "_1_1", "course": {"courseId": "SYN-330-002-FA2026", "id": "_900110_1"}})).as_deref(),
            Some("other")
        );
        assert_eq!(by(json!({"courseId": "_nope_1", "course": {"courseId": "x", "id": "y"}})), None);
        assert_eq!(by(json!({})), None);
    }

    #[test]
    fn match_course_never_matches_an_empty_code() {
        let notes = vec![CourseRef { slug: "blank".into(), code: String::new() }];
        let c = BundleCourse { membership: json!({"courseId": ""}), columns: vec![], grades: vec![], categories: None, error: None };
        assert_eq!(match_course(&c, &notes), None);
    }

    // ---- column_to_fields ----

    #[test]
    fn a_numeric_score_is_graded() {
        let f = fields_of(&bundle().courses[0], "_8110001_1");
        assert_eq!(f.title, "Homework 1");
        assert_eq!(f.source_uid, "_8110001_1");
        assert_eq!((f.kind, f.status, f.counts), (Kind::Item, Status::Graded, true));
        assert_eq!((f.score, f.possible), (Some(18.0), Some(20.0)));
        assert_eq!(f.category, None, "category comes from category_title");
        assert_eq!(f.due.as_deref(), Some("2026-09-04T23:59"), "04:59Z is 23:59 in Chicago");
    }

    #[test]
    fn the_other_statuses() {
        let b = bundle();
        let c = &b.courses[0];
        let needs = fields_of(c, "_8110002_1");
        assert_eq!((needs.status, needs.score), (Status::NeedsGrading, None));
        let prog = fields_of(c, "_8110003_1");
        assert_eq!((prog.status, prog.score), (Status::InProgress, None));
        let none = fields_of(c, "_8110004_1");
        assert_eq!((none.status, none.score), (Status::NotSubmitted, None), "no row is not-submitted");
        assert!(row_for(c, "_8110004_1").is_none());
        let ex = fields_of(c, "_8110005_1");
        assert_eq!((ex.status, ex.score), (Status::Exempt, None));
    }

    #[test]
    fn exempt_wins_over_a_score_and_a_score_wins_over_needs_grading() {
        let col = json!({"id": "_1_1", "name": "X", "score": {"possible": 10.0}});
        let exempt = json!({"columnId": "_1_1", "status": "Graded", "score": 9.0, "exempt": true});
        assert_eq!(column_to_fields(&col, Some(&exempt), &chicago()).status, Status::Exempt);
        let regrade = json!({"columnId": "_1_1", "status": "NeedsGrading", "score": 9.0, "exempt": false});
        let f = column_to_fields(&col, Some(&regrade), &chicago());
        assert_eq!((f.status, f.score), (Status::Graded, Some(9.0)));
    }

    #[test]
    fn a_graded_row_with_no_number_is_graded_with_no_score() {
        let col = json!({"id": "_1_1", "name": "X"});
        let row = json!({"columnId": "_1_1", "status": "Graded", "text": "Pass"});
        let f = column_to_fields(&col, Some(&row), &chicago());
        assert_eq!((f.status, f.score), (Status::Graded, None));
    }

    #[test]
    fn a_row_with_no_number_and_an_unknown_or_absent_status_is_in_progress() {
        let col = json!({"id": "_1_1", "name": "X"});
        let unknown = json!({"columnId": "_1_1", "status": "Mystery"});
        assert_eq!(column_to_fields(&col, Some(&unknown), &chicago()).status, Status::InProgress);
        let absent = json!({"columnId": "_1_1"});
        assert_eq!(column_to_fields(&col, Some(&absent), &chicago()).status, Status::InProgress);
    }

    #[test]
    fn the_external_grade_column_is_the_overall() {
        let b = bundle();
        let f = fields_of(&b.courses[0], "_8110007_1");
        assert_eq!((f.kind, f.status, f.score, f.possible), (Kind::Overall, Status::Graded, Some(88.25), Some(100.0)));
        assert_eq!(f.due, None, "a calculated column has no due");
    }

    #[test]
    fn a_column_with_no_possible_omits_it_and_include_in_calculations_false_is_not_counted() {
        let b = bundle();
        let part = fields_of(&b.courses[2], "_8330003_1");
        assert_eq!((part.possible, part.score, part.status), (None, Some(5.0), Status::Graded));
        assert_eq!(part.due, None);
        let practice = fields_of(&b.courses[0], "_8110006_1");
        assert!(!practice.counts);
        assert_eq!(practice.due, None);
    }

    #[test]
    fn due_moves_into_the_vaults_zone_and_a_bad_due_is_absent() {
        let col = |due: &str| json!({"id": "_1_1", "name": "X", "grading": {"due": due}});
        let f = |due: &str, tz: &TimeZone| column_to_fields(&col(due), None, tz).due;
        assert_eq!(f("2026-09-30T20:00:00.000Z", &chicago()).as_deref(), Some("2026-09-30T15:00"));
        assert_eq!(f("2026-09-30T20:00:00.000Z", &TimeZone::UTC).as_deref(), Some("2026-09-30T20:00"));
        assert_eq!(f("2026-01-15T05:30:00Z", &TimeZone::get("Asia/Tokyo").unwrap()).as_deref(), Some("2026-01-15T14:30"));
        assert_eq!(f("not a date", &chicago()), None);
    }

    #[test]
    fn a_column_missing_its_include_flag_counts() {
        assert!(column_to_fields(&json!({"id": "_1_1", "name": "X"}), None, &chicago()).counts);
    }

    #[test]
    fn category_title_names_the_column_category_or_nothing() {
        let b = bundle();
        let c = &b.courses[0];
        let hw = c.columns.iter().find(|x| x["id"] == "_8110001_1").unwrap();
        assert_eq!(category_title(hw, c.categories.as_deref()).as_deref(), Some("Homework"));
        let overall = c.columns.iter().find(|x| x["id"] == "_8110007_1").unwrap();
        assert_eq!(category_title(overall, c.categories.as_deref()), None, "no gradebookCategoryId");
        assert_eq!(category_title(hw, None), None, "a refused list drops the grouping");
        let unknown = json!({"id": "_1_1", "gradebookCategoryId": "_nope_1"});
        assert_eq!(category_title(&unknown, c.categories.as_deref()), None);
    }

    // ---- note_path ----

    #[test]
    fn note_path_slugs_the_column_id() {
        assert_eq!(note_path("cs-100", "_4686399_1"), "grades/cs-100-4686399-1.md");
    }

    #[test]
    fn a_hostile_column_id_stays_inside_grades() {
        for hostile in ["../x", "a b/../../etc", "..\\x", "_1_1\n", "\u{202e}evil", "", "///"] {
            let p = note_path("cs-100", hostile);
            let name = p.strip_prefix("grades/").expect("under grades/");
            assert!(name.ends_with(".md"), "{p}");
            let stem = name.strip_suffix(".md").unwrap();
            assert!(!stem.is_empty() && stem.chars().all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '-'), "{p}");
        }
        assert_eq!(note_path("cs-100", "../x"), "grades/cs-100-x.md");
        let long = "9".repeat(300);
        let p = note_path("cs-100", &long);
        assert!(p.len() <= 140, "bounded: {}", p.len());
        assert!(p.starts_with("grades/cs-100-999"));
        let hostile_course = note_path("../../tasks/x y", "_1_1");
        assert!(hostile_course.starts_with("grades/") && !hostile_course[7..].contains('/'), "{hostile_course}");
        assert!(note_path(&"c".repeat(300), "_1_1").len() <= 140);
    }

    // ---- letter ----

    #[test]
    fn every_boundary_of_g3_on_both_sides() {
        let m = |c: char| format!("{c}{MINUS}");
        let s = |t: &str| t.to_string();
        let edges: [(f64, String, f64, String); 11] = [
            (93.0, s("A"), 92.999, m('A')),
            (90.0, m('A'), 89.999, s("B+")),
            (87.0, s("B+"), 86.999, s("B")),
            (83.0, s("B"), 82.999, m('B')),
            (80.0, m('B'), 79.999, s("C+")),
            (77.0, s("C+"), 76.999, s("C")),
            (73.0, s("C"), 72.999, m('C')),
            (70.0, m('C'), 69.999, s("D+")),
            (67.0, s("D+"), 66.999, s("D")),
            (63.0, s("D"), 62.999, m('D')),
            (60.0, m('D'), 59.999, s("F")),
        ];
        for (at, at_letter, below, below_letter) in edges {
            assert_eq!(letter(at), at_letter, "at {at}");
            assert_eq!(letter(below), below_letter, "at {below}");
        }
        assert_eq!(letter(100.0), "A");
        assert_eq!(letter(112.5), "A", "extra credit is still an A");
        assert_eq!(letter(0.0), "F");
        assert_eq!(letter(-3.0), "F");
    }

    // ---- course_grade ----

    #[test]
    fn basis_overall_when_the_overall_note_is_scored_and_it_still_carries_points_so_far() {
        let notes = [
            note(Kind::Overall, Some(88.25), Some(100.0), Status::Graded),
            note(Kind::Item, Some(18.0), Some(20.0), Status::Graded),
            note(Kind::Item, Some(7.0), Some(10.0), Status::Graded),
        ];
        let s = course_grade(&notes);
        assert_eq!(s.basis, Basis::Overall);
        assert_eq!(s.pct, Some(88.25));
        assert_eq!(s.letter, Some("B+"));
        assert_eq!(s.family, Some('B'));
        assert_eq!((s.earned, s.possible), (25.0, 30.0), "points so far, whatever the basis");
        assert_eq!((s.graded, s.items, s.pending), (2, 2, 0));
    }

    #[test]
    fn basis_points_is_earned_over_possible_on_counted_graded_items() {
        let mut uncounted = note(Kind::Item, Some(0.0), Some(50.0), Status::Graded);
        uncounted.counts = false;
        let notes = [
            note(Kind::Item, Some(18.0), Some(20.0), Status::Graded),
            note(Kind::Item, Some(45.0), Some(50.0), Status::Graded),
            note(Kind::Item, None, Some(100.0), Status::InProgress),
            note(Kind::Item, None, Some(20.0), Status::NeedsGrading),
            note(Kind::Item, None, Some(10.0), Status::NotSubmitted),
            uncounted,
        ];
        let s = course_grade(&notes);
        assert_eq!(s.basis, Basis::Points);
        assert_eq!((s.earned, s.possible), (63.0, 70.0));
        assert_eq!(s.pct, Some(90.0));
        assert_eq!((s.letter, s.family), (Some("A\u{2212}"), Some('A')));
        assert_eq!((s.graded, s.items, s.pending), (3, 6, 3));
    }

    #[test]
    fn exempt_uncounted_removed_and_possible_zero_never_enter_points() {
        let mut uncounted = note(Kind::Item, Some(10.0), Some(10.0), Status::Graded);
        uncounted.counts = false;
        let notes = [
            note(Kind::Item, Some(8.0), Some(10.0), Status::Graded),
            // each of these carries a score, so only the filter keeps it out
            note(Kind::Item, Some(10.0), Some(10.0), Status::Exempt),
            note(Kind::Item, Some(10.0), Some(10.0), Status::Removed),
            note(Kind::Item, Some(5.0), Some(0.0), Status::Graded),
            note(Kind::Item, Some(5.0), None, Status::Graded),
            uncounted,
        ];
        let s = course_grade(&notes);
        assert_eq!((s.earned, s.possible, s.pct), (8.0, 10.0, Some(80.0)));
        assert_eq!(s.items, 5, "a removed column is not an item any more");
    }

    #[test]
    fn an_overall_that_cannot_stand_falls_back_to_points() {
        let item = note(Kind::Item, Some(9.0), Some(10.0), Status::Graded);
        for overall in [
            note(Kind::Overall, None, Some(100.0), Status::InProgress),
            note(Kind::Overall, Some(80.0), None, Status::Graded),
            note(Kind::Overall, Some(80.0), Some(0.0), Status::Graded),
            note(Kind::Overall, Some(80.0), Some(100.0), Status::Removed),
            note(Kind::Overall, Some(80.0), Some(100.0), Status::Exempt),
        ] {
            let s = course_grade(&[overall.clone(), item.clone()]);
            assert_eq!((s.basis, s.pct), (Basis::Points, Some(90.0)), "{overall:?}");
        }
    }

    #[test]
    fn basis_none_when_nothing_is_graded() {
        let s = course_grade(&[note(Kind::Item, None, Some(10.0), Status::NotSubmitted)]);
        assert_eq!((s.basis, s.pct, s.letter, s.family), (Basis::None, None, None, None));
        assert_eq!((s.earned, s.possible, s.graded, s.items, s.pending), (0.0, 0.0, 0, 1, 1));
        let empty = course_grade(&[]);
        assert_eq!((empty.basis, empty.items), (Basis::None, 0));
    }

    #[test]
    fn the_fixtures_first_course_reads_as_overall_88_25() {
        let b = bundle();
        let c = &b.courses[0];
        let notes: Vec<GradeNote> = c
            .columns
            .iter()
            .map(|col| {
                let f = column_to_fields(col, row_for(c, col["id"].as_str().unwrap()), &chicago());
                GradeNote { kind: f.kind, score: f.score, possible: f.possible, status: f.status, counts: f.counts }
            })
            .collect();
        let s = course_grade(&notes);
        assert_eq!((s.basis, s.pct), (Basis::Overall, Some(88.25)));
        // Homework 1 (18/20) counts; the uncounted practice quiz (7/10) and the exempt quiz do not
        assert_eq!((s.earned, s.possible), (18.0, 20.0));
    }

    #[test]
    fn status_and_kind_round_trip_through_their_words() {
        for s in [Status::Graded, Status::NeedsGrading, Status::InProgress, Status::NotSubmitted, Status::Exempt, Status::Removed] {
            assert_eq!(Status::parse(s.as_str()), Some(s));
        }
        assert_eq!(Status::parse("done"), None);
        assert_eq!(Kind::parse("overall"), Some(Kind::Overall));
        assert_eq!(Kind::parse(Kind::Item.as_str()), Some(Kind::Item));
        assert_eq!(Kind::parse("x"), None);
    }

    #[test]
    fn the_module_reads_no_files_and_opens_no_sockets() {
        let src = include_str!("grades.rs");
        let code = &src[..src.find("#[cfg(test)]\nmod tests").unwrap()];
        for banned in ["std::fs", "std::net", "ureq", "std::process"] {
            assert!(!code.contains(banned), "{banned}");
        }
    }
}
