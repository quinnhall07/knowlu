//! `knowlu-engine onboarding` (Wave B plan §4.4, F7a). Read-only: step 3's counts from raw captures.
//!
//! Arithmetic over files: it writes nothing, opens no socket, and an input it cannot read is a
//! `null` count and a named line in `errors`, never a failure.

use std::collections::{BTreeMap, BTreeSet};
use std::path::{Path, PathBuf};

use jiff::civil::DateTime;
use jiff::tz::TimeZone;
use serde_json::{json, Value};
use serde_yaml_ng::Mapping;

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
///
/// `by_course` is an object keyed by course slug (the label when the slug is empty) whose values
/// carry `assignments_upcoming`, `turned_in` and `class_times`, each `null` where its input is
/// missing, so the reveal can re-total after the student removes a course. An item that matches no
/// course is in the totals and in no bucket; `null` when there is no course list.
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

/// One course of the captured list: the `by_course` key (the slug, or the label when the slug is
/// empty; empty when it has neither), the LMS's course key, and the codes its label and slug compact to.
struct Course {
    key: String,
    lms: String,
    codes: Vec<String>,
}

/// The course list, in the order captured.
fn read_courses(text: &str, errors: &mut Vec<String>) -> Option<Vec<Course>> {
    let Some(list) = serde_json::from_str::<Value>(text).ok().and_then(|v| v.as_array().cloned()) else {
        errors.push("courses: not a JSON array of courses".to_string());
        return None;
    };
    let field = |c: &Value, k: &str| c.get(k).and_then(Value::as_str).unwrap_or("").trim().to_string();
    Some(
        list.iter()
            .map(|c| {
                let (slug, label) = (field(c, "slug"), field(c, "label"));
                let codes = [&label, &slug].iter().map(|t| compact(t)).filter(|t| !t.is_empty()).collect();
                Course { key: if slug.is_empty() { label } else { slug }, lms: field(c, "code"), codes }
            })
            .collect(),
    )
}

/// The first course (in list order) one of whose codes is `code`.
fn course_with_code(courses: &[Course], code: &str) -> Option<usize> {
    courses.iter().position(|c| c.codes.iter().any(|k| k == code))
}

/// Does `text` open with a course code? Its alphanumeric runs are joined from the start, one to
/// three of them, and compared with the course's codes: `SYN-110-003-FALL2026`, `SYN 110: Reading`
/// and `SYN110.55555` all open with `SYN110`; `SYN-1100-001` does not. Never a mid-sentence mention.
fn opens_with_code(text: &str, codes: &[String]) -> bool {
    let runs: Vec<&str> = text.split(|c: char| !c.is_ascii_alphanumeric()).filter(|t| !t.is_empty()).collect();
    (1..=runs.len().min(3)).any(|k| codes.contains(&compact(&runs[..k].concat())))
}

/// The ICS has no one reliable course field (the fixture's gradebook items carry none, a homework
/// item has the section id in a trailing `[...]` of its SUMMARY, another in CATEGORIES). So the
/// candidates are the SUMMARY's trailing bracket, each CATEGORIES value and the SUMMARY itself
/// (its prefix); never DESCRIPTION, never a title match.
fn course_texts(raw: &str) -> Vec<String> {
    let mut out = Vec::new();
    for line in raw.split('\n') {
        let Some((name, _, value)) = ingest::parse_property(line) else { continue };
        let value = ingest::unescape(&value);
        match name.as_str() {
            "SUMMARY" => {
                if let Some((_, inner)) = value.trim_end().strip_suffix(']').and_then(|r| r.rsplit_once('[')) {
                    out.push(inner.to_string());
                }
                out.push(value);
            }
            "CATEGORIES" => out.extend(value.split(',').map(|s| s.trim().to_string())),
            _ => {}
        }
    }
    out
}

/// The distinct UIDs of the feed's items that ingest's first run would keep live: an item is left
/// out only when its due date is before today in the vault's zone (`ingest.rs`'s `past_due`), so an
/// item timed earlier today still counts and the proof's number equals what lands in the vault.
/// Institutional noise and an event with no due are not items. `None` when the text is not a
/// calendar.
fn read_feed(text: &str, tz: &TimeZone, now: DateTime, errors: &mut Vec<String>) -> Option<Vec<FeedItem>> {
    if !text.contains("BEGIN:VCALENDAR") {
        errors.push("feed: not a calendar".to_string());
        return None;
    }
    let (events, _malformed) = ingest::parse_ics(text, tz);
    let mut upcoming: BTreeMap<String, Vec<String>> = BTreeMap::new();
    for e in events {
        let live = match e.due {
            Some(Due::DateTime(dt)) => dt.date() >= now.date(),
            Some(Due::Date(d)) => d >= now.date(),
            None => false,
        };
        if live && !ingest::is_institutional_noise(&e.uid) {
            upcoming.entry(e.uid).or_insert_with(|| course_texts(&e.raw));
        }
    }
    Some(upcoming.into_iter().map(|(uid, texts)| FeedItem { uid, texts }).collect())
}

/// An upcoming feed item: its UID and the texts that may name its course.
struct FeedItem {
    uid: String,
    texts: Vec<String>,
}

/// From the v2 bundle: the item columns whose row says turned in (O1's rule), and how many courses
/// have at least one item column with a graded row. "Graded" follows `ingest::gradebook_done`: a
/// `graded` row with a score above 0, or an `exempt` row; a row still `needs-grading` is turned in
/// but not graded, and a graded zero is live, so neither makes a grades course. An Overall column is
/// neither; a course whose capture failed has no columns to count.
fn read_grades(text: &str, tz: &TimeZone, errors: &mut Vec<String>) -> Option<Grades> {
    let Ok(bundle) = serde_json::from_str::<crate::grades::Bundle>(text) else {
        errors.push("grades: not a v2 grades bundle".to_string());
        return None;
    };
    let (mut done, mut courses, mut owner) = (BTreeSet::new(), 0, BTreeMap::new());
    for course in bundle.courses.iter().filter(|c| c.error.is_none()) {
        let lms = course.membership["courseId"].as_str().unwrap_or("").to_string();
        let mut graded = false;
        for column in &course.columns {
            let Some(id) = column["id"].as_str().filter(|s| !s.is_empty()) else { continue };
            let row = course.grades.iter().find(|r| r["columnId"].as_str() == Some(id));
            let fields = crate::grades::column_to_fields(column, row, tz);
            if fields.kind != crate::grades::Kind::Item {
                continue;
            }
            owner.insert(id.to_string(), lms.clone());
            if ingest::gradebook_done(fields.status.as_str(), fields.score) {
                done.insert(id.to_string());
                graded |= fields.status != crate::grades::Status::NeedsGrading;
            }
        }
        courses += usize::from(graded);
    }
    Some(Grades { done, courses, owner })
}

/// What the bundle gives: the done item columns, the count of courses with a graded item, and each
/// item column's LMS course key (how a feed item with no course text still reaches its course).
struct Grades {
    done: BTreeSet<String>,
    courses: usize,
    owner: BTreeMap<String, String>,
}

/// The registrar's matched class times: Banner's series read `Reading::Whole` (a partly read row is
/// not counted, as the registrar's own confirm skips it) whose course code (a lab's " Lab" dropped)
/// is a code in the course list. Nothing is written and nothing is fetched.
///
/// A student whose classes are all online has no `CLAS` meeting with a day and times, so the parse
/// fails: `class_times` is `null` plus a named `registrar:` line in `errors`. The reveal treats that
/// as a named outcome (no class times to show), never as a failure.
fn read_registrar(text: &str, school: Option<&str>, courses: &[Course], errors: &mut Vec<String>) -> Option<Vec<usize>> {
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
        Ok(parsed) => {
            let whole = |uid: &str| {
                let crn = uid.rsplit_once('-').map_or("", |(_, crn)| crn);
                parsed.seen.get(crn) == Some(&crate::registrar::Reading::Whole)
            };
            let named = |title: &str| course_with_code(courses, &compact(title.strip_suffix(" Lab").unwrap_or(title)));
            Some(parsed.series.iter().filter(|s| whole(&s.source_uid)).filter_map(|s| named(&s.title)).collect())
        }
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
        (Some(text), Some(list)) => read_registrar(text, inputs.school, list, &mut errors),
        _ => None,
    };
    let is_done = |u: &FeedItem| grades.as_ref().is_some_and(|g| ingest::gradable_id(&u.uid).is_some_and(|id| g.done.contains(id)));
    let turned_in = match (&feed, &grades) {
        (Some(items), Some(_)) => Some(items.iter().filter(|i| is_done(i)).count()),
        _ => None,
    };
    let by_course = courses.as_ref().map(|list| {
        let mut buckets: BTreeMap<String, [Option<usize>; 3]> = BTreeMap::new();
        for c in list.iter().filter(|c| !c.key.is_empty()) {
            let zero = |on: bool| on.then_some(0);
            buckets.insert(c.key.clone(), [zero(feed.is_some()), zero(feed.is_some() && grades.is_some()), zero(class_times.is_some())]);
        }
        let mut add = |idx: Option<usize>, slot: usize| {
            let Some(c) = idx.map(|i| &list[i]).filter(|c| !c.key.is_empty()) else { return };
            if let Some(n) = buckets.get_mut(&c.key).and_then(|b| b[slot].as_mut()) {
                *n += 1;
            }
        };
        for item in feed.iter().flatten() {
            let idx = course_of_item(item, list, grades.as_ref());
            add(idx, 0);
            if is_done(item) {
                add(idx, 1);
            }
        }
        for idx in class_times.iter().flatten() {
            add(Some(*idx), 2);
        }
        let entry = |b: &[Option<usize>; 3]| json!({"assignments_upcoming": b[0], "turned_in": b[1], "class_times": b[2]});
        Value::Object(buckets.iter().map(|(k, b)| (k.clone(), entry(b))).collect())
    });
    json!({
        "assignments_upcoming": feed.as_ref().map(Vec::len),
        "by_course": by_course,
        "class_times": class_times.as_ref().map(Vec::len),
        "classes": courses.as_ref().map(Vec::len),
        "errors": errors,
        "grades_courses": grades.as_ref().map(|g| g.courses),
        "turned_in": turned_in,
    })
}

/// The course a feed item belongs to: the first course in list order whose code the item's own
/// texts open with; failing that, the course the grades bundle holds the item's column in. Never
/// the title alone.
fn course_of_item(item: &FeedItem, courses: &[Course], grades: Option<&Grades>) -> Option<usize> {
    let by_text = courses.iter().position(|c| item.texts.iter().any(|t| opens_with_code(t, &c.codes)));
    by_text.or_else(|| {
        let lms = grades?.owner.get(ingest::gradable_id(&item.uid)?)?;
        courses.iter().position(|c| !c.lms.is_empty() && &c.lms == lms)
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

/// The top-level `*.md` notes of one vault folder, as (vault-relative path, frontmatter), in path
/// order. `None` (and a named line) when the folder is missing; a note that cannot be read is one
/// summary line, never a failure.
fn folder_notes(vault: &Path, folder: &str, errors: &mut Vec<String>) -> Option<Vec<(String, Mapping)>> {
    let Ok(entries) = std::fs::read_dir(vault.join(folder)) else {
        errors.push(format!("{folder}: folder not found"));
        return None;
    };
    let mut paths: Vec<PathBuf> =
        entries.filter_map(|e| e.ok()).map(|e| e.path()).filter(|p| p.extension().is_some_and(|x| x == "md")).collect();
    paths.sort();
    let (mut notes, mut unreadable) = (Vec::new(), 0);
    for path in paths {
        let name = path.file_name().map(|n| n.to_string_lossy().to_string()).unwrap_or_default();
        match std::fs::read_to_string(&path).map_err(|_| ()).and_then(|t| crate::models::split_frontmatter(&t).map_err(|_| ())) {
            Ok((meta, _)) => notes.push((format!("{folder}/{name}"), meta)),
            Err(()) => unreadable += 1,
        }
    }
    if unreadable > 0 {
        errors.push(format!("{folder}: {unreadable} note(s) could not be read"));
    }
    Some(notes)
}

fn field_text(meta: &Mapping, key: &str) -> Option<String> {
    crate::yaml::get(meta, key).and_then(crate::yaml::text).map(|t| t.trim().to_string()).filter(|t| !t.is_empty())
}

/// The notes (by path) and ids an `agent:` actor set `effort_hours` on, from the journal's day files.
/// `None` and a named line when the folder is missing. Reads through the ledger reader, which writes nothing.
fn agent_effort_sets(vault: &Path, errors: &mut Vec<String>) -> Option<(BTreeSet<String>, BTreeSet<String>)> {
    let mut ledger = crate::ledger::JsonlLedger::new(vault.join("state").join("journal"));
    if !ledger.root.is_dir() {
        errors.push("journal: folder not found".to_string());
        return None;
    }
    let (mut paths, mut ids) = (BTreeSet::new(), BTreeSet::new());
    let text = |r: &crate::ledger::Record, k: &str| r.get(k).and_then(Value::as_str).map(str::to_string);
    for r in ledger.read(None, None) {
        let by_agent = text(&r, "actor").is_some_and(|a| crate::provenance::is_agent(&a));
        if by_agent && text(&r, "op").as_deref() == Some("set") && text(&r, "field").as_deref() == Some("effort_hours") {
            paths.extend(text(&r, "path").map(|p| p.replace('\\', "/")));
            ids.extend(text(&r, "id"));
        }
    }
    errors.extend(ledger.warnings.iter().map(|w| format!("journal: {w}")));
    Some((paths, ids))
}

/// `--tidied`: the reveal's counts from the vault. `imported_past` and `imported_done` count
/// `archive/` notes by `archived_reason`; `live_by_source` counts the active tasks in `tasks/` by
/// `created_by` (none named is `unknown`); `effort_estimated` counts those tasks with at least one
/// `op: set` of `effort_hours` by an `agent:` actor in the journal, even if the student overrode it
/// since. A folder that is missing is `null` for what it feeds and a named line in `errors`; a vault
/// that does not exist is all `null`. Nothing is created, written or fetched.
pub fn tidied(vault: &Path) -> Value {
    let mut errors = Vec::new();
    let (mut past, mut done, mut live, mut effort) = (None, None, None, None);
    if !vault.is_dir() {
        errors.push("vault: folder not found".to_string());
    } else {
        if let Some(notes) = folder_notes(vault, "archive", &mut errors) {
            let reason = |want: &str| notes.iter().filter(|(_, m)| field_text(m, "archived_reason").as_deref() == Some(want)).count();
            (past, done) = (Some(reason(ingest::IMPORTED_PAST)), Some(reason(ingest::IMPORTED_DONE)));
        }
        let tasks = folder_notes(vault, "tasks", &mut errors).map(|notes| {
            notes.into_iter().filter(|(_, m)| field_text(m, "status").as_deref().unwrap_or("active") == "active").collect::<Vec<_>>()
        });
        let sets = agent_effort_sets(vault, &mut errors);
        if let Some(tasks) = &tasks {
            let mut by_source: BTreeMap<String, usize> = BTreeMap::new();
            for (_, meta) in tasks {
                *by_source.entry(field_text(meta, "created_by").unwrap_or_else(|| "unknown".to_string())).or_default() += 1;
            }
            live = Some(by_source);
        }
        if let (Some(tasks), Some((paths, ids))) = (&tasks, &sets) {
            let set_by_agent = |(path, meta): &&(String, Mapping)| paths.contains(path) || field_text(meta, "id").is_some_and(|id| ids.contains(&id));
            effort = Some(tasks.iter().filter(set_by_agent).count());
        }
    }
    json!({
        "effort_estimated": effort,
        "errors": errors,
        "imported_done": done,
        "imported_past": past,
        "live_by_source": live,
    })
}

/// `--tidied` as the one JSON line, through `ledger::dumps_value`.
pub fn run_tidied(vault: &Path) -> String {
    crate::ledger::dumps_value(&tidied(vault))
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
        // 2026-09-02 02:00 UTC is 2026-09-01 21:00 in Chicago (CDT): today, so upcoming.
        // 2026-09-01 04:00 UTC is 2026-08-31 23:00 in Chicago: before today, though it reads as
        // today if the Z is ignored.
        let feed = ics(&[
            vevent("BB-ASSIGN-1", "Evening thing", "20260902T020000Z"),
            vevent("BB-ASSIGN-2", "Yesterday thing", "20260901T040000Z"),
        ]);
        let v = run(&feed, None);
        assert_eq!(count(&v, "assignments_upcoming"), json!(1), "{v}");
    }

    #[test]
    fn an_item_timed_earlier_today_still_counts_as_upcoming() {
        // Ingest's first-run rule: excluded only when the due date is before today (ingest.rs:899),
        // so the proof's number equals what lands in the vault. Now is 12:00.
        let feed = ics(&[
            vevent("BB-ASSIGN-1", "Morning thing", "20260901T090000"),
            vevent("BB-ASSIGN-2", "Yesterday thing", "20260831T235900"),
        ]);
        let v = run(&feed, None);
        assert_eq!(count(&v, "assignments_upcoming"), json!(1), "{v}");
    }

    #[test]
    fn a_date_only_due_today_counts_and_yesterday_does_not() {
        let feed = ics(&[
            "BEGIN:VEVENT
UID:BB-ASSIGN-1
SUMMARY:All day
DTSTART;VALUE=DATE:20260901
END:VEVENT
".to_string(),
            "BEGIN:VEVENT
UID:BB-ASSIGN-2
SUMMARY:Gone
DTSTART;VALUE=DATE:20260831
END:VEVENT
".to_string(),
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

    fn banner_rows(rows: Vec<Value>) -> String {
        json!({"data": {"registrations": rows}}).to_string()
    }

    fn banner_row(crn: &str, number: &str, schedule: &str, meetings: Value) -> Value {
        json!({"term": "202640", "courseReferenceNumber": crn, "subject": "SYN", "courseNumber": number,
               "scheduleDescription": schedule, "statusIndicator": "IR", "meetingTimes": meetings})
    }

    fn clas(begin: Option<&str>) -> Value {
        let mut m = json!({"meetingType": "CLAS", "monday": true, "endTime": "0950",
                           "startDate": "08/20/2026", "endDate": "12/05/2026"});
        if let Some(b) = begin {
            m["beginTime"] = json!(b);
        }
        m
    }

    #[test]
    fn only_whole_series_count_as_class_times() {
        // SYN 110: one good meeting, whole. SYN 120: a good meeting and one with no begin time,
        // so the row is read partial and does not count.
        let b = banner_rows(vec![
            banner_row("90001", "110", "Lecture", json!([clas(Some("0900"))])),
            banner_row("90002", "120", "Lecture", json!([clas(Some("0900")), clas(None)])),
        ]);
        let v = with_registrar(Some(&b), Some("ua"));
        assert_eq!(count(&v, "class_times"), json!(1), "{v}");
    }

    #[test]
    fn a_lab_row_matches_its_course() {
        let b = banner_rows(vec![
            banner_row("90001", "110", "Lecture", json!([clas(Some("0900"))])),
            banner_row("90002", "110", "Laboratory", json!([clas(Some("0800"))])),
        ]);
        let v = with_registrar(Some(&b), Some("ua"));
        assert_eq!(count(&v, "class_times"), json!(2), "the lab (titled 'SYN 110 Lab') matches SYN 110: {v}");
    }

    #[test]
    fn a_course_with_an_empty_label_matches_by_its_slug() {
        let c = json!([{"code": "_900001_1", "name": "Synthetic Studies I", "slug": "syn-110", "label": ""}]).to_string();
        let b = banner_rows(vec![banner_row("90001", "110", "Lecture", json!([clas(Some("0900"))]))]);
        let f = ics(&[]);
        let inputs = PreviewInputs { feed: Some(&f), courses: Some(&c), registrar: Some(&b), school: Some("ua"), ..Default::default() };
        let v = preview(&inputs, &chicago(), now());
        assert_eq!(count(&v, "class_times"), json!(1), "{v}");
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
    fn a_course_with_columns_but_nothing_graded_is_not_a_grades_course() {
        // Course one: an item column and no row (not submitted). Course two: a graded zero (live by
        // gradebook_done), an item still needing grading. Course three: one exempt column (turned in).
        let item = |id: &str| json!({"id": id, "name": id, "externalGrade": false, "score": {"possible": 10.0}});
        let zero = json!({"columnId": "_2_1", "status": "Graded", "exempt": false,
                          "displayGrade": {"scaleType": "Score", "score": 0.0, "possible": 10.0}});
        let needs = json!({"columnId": "_3_1", "status": "NeedsGrading", "exempt": false});
        let exempt = json!({"columnId": "_4_1", "status": "Graded", "exempt": true});
        let bundle = json!({
            "schema": 1, "lms": "blackboard", "host": "lms.example.test", "fetched_at": "2026-08-30T12:00:00Z", "user_id": "_900_1",
            "courses": [
                {"membership": {"courseId": "_900001_1"}, "columns": [item("_1_1")], "grades": [], "categories": null},
                {"membership": {"courseId": "_900002_1"}, "columns": [item("_2_1"), item("_3_1")], "grades": [zero, needs], "categories": null},
                {"membership": {"courseId": "_900003_1"}, "columns": [item("_4_1")], "grades": [exempt], "categories": null},
            ],
        })
        .to_string();
        let v = run(&ics(&[]), Some(&bundle));
        assert_eq!(count(&v, "grades_courses"), json!(1), "only the exempt course has a graded row: {v}");
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
        // `by_course` nests the same count names, so the top-level `turned_in` is the last one.
        let last = a.rfind("\"turned_in\"").unwrap();
        assert!(at("assignments_upcoming") < at("by_course") && at("by_course") < at("classes"), "{a}");
        assert!(at("classes") < at("errors") && at("errors") < at("grades_courses") && at("grades_courses") < last, "{a}");
    }

    // ---- `by_course`: the reveal's per-course counts ----

    /// An event with a CATEGORIES line, as Blackboard writes the section id.
    fn vevent_in(uid: &str, summary: &str, categories: Option<&str>, description: &str) -> String {
        let cat = categories.map(|c| format!("CATEGORIES:{c}\n")).unwrap_or_default();
        format!("BEGIN:VEVENT\nUID:{uid}\nSUMMARY:{summary}\n{cat}DESCRIPTION:{description}\nDTSTART:20260905T235900\nEND:VEVENT\n")
    }

    fn by_course(v: &Value, slug: &str, key: &str) -> Value {
        v["by_course"][slug][key].clone()
    }

    fn two_class_banner() -> String {
        let meets = json!([{"meetingType": "CLAS", "monday": true, "beginTime": "0900", "endTime": "0950",
                            "startDate": "08/20/2026", "endDate": "12/05/2026"}]);
        banner_rows(vec![
            json!({"term": "202640", "courseReferenceNumber": "90001", "subject": "SYN", "courseNumber": "110",
                   "scheduleDescription": "Lecture", "statusIndicator": "IR", "meetingTimes": meets.clone()}),
            json!({"term": "202640", "courseReferenceNumber": "90002", "subject": "SYN", "courseNumber": "120",
                   "scheduleDescription": "Lecture", "statusIndicator": "IR", "meetingTimes": meets.clone()}),
            json!({"term": "202640", "courseReferenceNumber": "90003", "subject": "ZZT", "courseNumber": "999",
                   "scheduleDescription": "Lecture", "statusIndicator": "IR", "meetingTimes": meets}),
        ])
    }

    fn full_preview(feed: &str) -> Value {
        let (c, g, r) = (courses(), bundle(), two_class_banner());
        let inputs = PreviewInputs { feed: Some(feed), courses: Some(&c), grades: Some(&g), registrar: Some(&r), school: Some("ua") };
        preview(&inputs, &chicago(), now())
    }

    fn all_mapped_feed() -> String {
        ics(&[
            vevent_in("BB-ASSIGN-1", "Homework 2 [SYN-110-003-FALL2026]", None, "x"),
            vevent_in("BB-ASSIGN-2", "Kapitel 2 Quiz", Some("SYN-120-001-FALL2026"), "x"),
            vevent_in("BB-ASSIGN-3", "SYN 120: Reading", None, "x"),
            // No course text at all; the grades bundle puts column _71_1 in course _900001_1 (SYN 110).
            vevent_in(&format!("{GB}_71_1"), "Renamed", None, "x"),
        ])
    }

    #[test]
    fn per_course_counts_sum_to_the_totals_when_every_item_maps() {
        let v = full_preview(&all_mapped_feed());
        let sum = |key: &str| -> u64 { ["syn-110", "syn-120"].iter().map(|s| by_course(&v, s, key).as_u64().unwrap()).sum() };
        assert_eq!(count(&v, "assignments_upcoming"), json!(4), "{v}");
        assert_eq!(sum("assignments_upcoming"), 4, "{v}");
        assert_eq!(count(&v, "turned_in"), json!(1), "{v}");
        assert_eq!(sum("turned_in"), 1, "{v}");
        assert_eq!(by_course(&v, "syn-110", "turned_in"), json!(1), "{v}");
        assert_eq!(count(&v, "class_times"), json!(2), "ZZT 999 is not in the course list: {v}");
        assert_eq!(sum("class_times"), 2, "{v}");
    }

    #[test]
    fn an_item_matching_no_course_is_in_the_totals_and_in_no_bucket() {
        let feed = ics(&[
            vevent_in("BB-ASSIGN-1", "Homework [SYN-110-003-FALL2026]", None, "x"),
            vevent_in("BB-ASSIGN-2", "Orphan [ZZT-999-001-FALL2026]", None, "x"),
            // The title names a course mid-sentence and the description names one: neither is a match.
            vevent_in("BB-ASSIGN-3", "Quiz covering SYN 120", None, "SYN 120 reading"),
            vevent_in("BB-ASSIGN-4", "Quiz A", None, "x"),
        ]);
        let v = full_preview(&feed);
        assert_eq!(count(&v, "assignments_upcoming"), json!(4), "{v}");
        assert_eq!(by_course(&v, "syn-110", "assignments_upcoming"), json!(1), "{v}");
        assert_eq!(by_course(&v, "syn-120", "assignments_upcoming"), json!(0), "{v}");
    }

    #[test]
    fn a_section_id_matches_at_a_token_boundary_only() {
        // SYN-1100 is not SYN 110; PH-106 would not match either course.
        let feed = ics(&[
            vevent_in("BB-ASSIGN-1", "Thing [SYN-1100-001]", None, "x"),
            vevent_in("BB-ASSIGN-2", "Thing", Some("SYN110.55555.202640"), "x"),
        ]);
        let v = full_preview(&feed);
        assert_eq!(by_course(&v, "syn-110", "assignments_upcoming"), json!(1), "{v}");
    }

    #[test]
    fn by_course_counts_are_null_where_their_input_is_missing() {
        let (f, c) = (all_mapped_feed(), courses());
        let inputs = PreviewInputs { feed: Some(&f), courses: Some(&c), ..Default::default() };
        let v = preview(&inputs, &chicago(), now());
        // Without the bundle, the id-only item (_71_1) has no course; the text match does.
        assert_eq!(by_course(&v, "syn-110", "assignments_upcoming"), json!(1), "{v}");
        assert_eq!(by_course(&v, "syn-110", "turned_in"), Value::Null, "{v}");
        assert_eq!(by_course(&v, "syn-110", "class_times"), Value::Null, "{v}");
        let (c, g) = (courses(), bundle());
        let no_feed = PreviewInputs { courses: Some(&c), grades: Some(&g), ..Default::default() };
        let w = preview(&no_feed, &chicago(), now());
        assert_eq!(by_course(&w, "syn-110", "assignments_upcoming"), Value::Null, "{w}");
        assert_eq!(by_course(&w, "syn-110", "turned_in"), Value::Null, "{w}");
    }

    #[test]
    fn by_course_is_keyed_by_slug_then_label_and_null_without_a_course_list() {
        let c = json!([
            {"code": "_1_1", "name": "A", "slug": "", "label": "SYN 110"},
            {"code": "_2_1", "name": "B", "slug": "syn-120", "label": ""},
            {"code": "_3_1", "name": "C", "slug": "", "label": ""},
        ])
        .to_string();
        let f = all_mapped_feed();
        let inputs = PreviewInputs { feed: Some(&f), courses: Some(&c), ..Default::default() };
        let v = preview(&inputs, &chicago(), now());
        assert_eq!(count(&v, "classes"), json!(3), "{v}");
        let keys: Vec<&String> = v["by_course"].as_object().unwrap().keys().collect();
        assert_eq!(keys, ["SYN 110", "syn-120"], "a course with neither slug nor label has no key: {v}");
        assert_eq!(by_course(&v, "SYN 110", "assignments_upcoming"), json!(1), "{v}");
        let none = PreviewInputs { feed: Some(&f), ..Default::default() };
        assert_eq!(count(&preview(&none, &chicago(), now()), "by_course"), Value::Null);
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
