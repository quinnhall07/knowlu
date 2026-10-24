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
    /// The captured course list: a JSON array of `{code, name, slug, label}`, each optionally with
    /// `pairs` (the fragments the app saved for it: codes and gradebook uid pins; a string or a
    /// `[fragment, slug]` pair each), which are then its only fragments.
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
/// `by_course` is an object of buckets whose values carry `assignments_upcoming`, `turned_in` and
/// `class_times` (each `null` where its input is missing) and `codes`, so the reveal can re-total
/// after the student removes a course. An item is matched against each course one at a time with
/// `ingest::match_course` and counted once under the sorted set of course keys that claim it. A
/// course's key is its LMS course key (`code`; the slug, then the label, when it is empty). The bucket
/// is named by those keys joined with `,` (its `codes` array is the set): one key in the usual case,
/// two for an item a past and a current shell both claim, and `unmatched` (empty `codes`) for an item
/// no course claims. The buckets sum to the `assignments_upcoming`, `turned_in` and `class_times`
/// totals, and with a course list the `graded: true` buckets count to `grades_courses`; the app
/// totals every bucket whose `codes` is empty or names a kept course, which is what ingest will import. `null` when there is no course list.
/// Each course's own bucket (present even at zero counts) also carries `graded`: `true` when the
/// grades bundle has a graded item column for it, `false` when it has none, `null` with no bundle;
/// multi-code buckets and `unmatched` carry none. Class times sit in their buckets for display
/// only: a removal never subtracts one (§14.47).
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

/// One course of the captured list. `key` is the `by_course` key: the LMS course key (`code`),
/// else the slug, else the label; empty when it has none of the three, and then it has no bucket.
/// Two enrolments of one course in different terms share a slug but never a `code`, so a removed
/// past-term shell can never take a live course's counts with it. `codes` are what the registrar's
/// class times are matched against: the saved pairs' fragments compacted, or, for an entry with no
/// `pairs`, its label and slug compacted. `pairs` are the fragments the app saved for the course
/// (`course_map`'s keys: the codes and the gradebook uid pins), so a label the student edited is
/// display only and moves no bucket (§14.46); `None` when the entry carries none.
struct Course {
    key: String,
    code: String,
    label: String,
    codes: Vec<String>,
    pairs: Option<Vec<String>>,
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
                let (code, slug, label) = (field(c, "code"), field(c, "slug"), field(c, "label"));
                let key = [&code, &slug, &label].into_iter().find(|t| !t.is_empty()).cloned().unwrap_or_default();
                let pairs = saved_pairs(c, &key, errors);
                let codes = match &pairs {
                    Some(fragments) => fragments.iter().map(|f| compact(f)).filter(|t| !t.is_empty()).collect(),
                    None => [&label, &slug].iter().map(|t| compact(t)).filter(|t| !t.is_empty()).collect(),
                };
                Course { key, code, label, codes, pairs }
            })
            .collect(),
    )
}

/// A course entry's saved `pairs`: an array whose elements are a fragment string or a
/// `[fragment, slug]` pair (`course_map`'s own shape). `None` only when the entry has no `pairs` key
/// at all (the engine then derives its own). A `pairs` that is not an array, or an element that is
/// neither shape, is never silently re-derived: it is a named `courses: <key>: pairs unreadable`
/// line (once per course), a non-array giving no fragments and an element being left out. The value
/// half of a pair is not read: the bucket key is the course's, whatever slug the pair names.
fn saved_pairs(course: &Value, key: &str, errors: &mut Vec<String>) -> Option<Vec<String>> {
    let raw = course.get("pairs")?;
    let mut readable = true;
    let mut fragment = |p: &Value| match p {
        Value::String(f) => Some(f.clone()),
        Value::Array(a) if a.first().is_some_and(Value::is_string) => a[0].as_str().map(str::to_string),
        _ => {
            readable = false;
            None
        }
    };
    let list = match raw.as_array() {
        Some(list) => list.iter().filter_map(|p| fragment(p)).map(|f| f.trim().to_string()).filter(|f| !f.is_empty()).collect(),
        None => {
            readable = false;
            Vec::new()
        }
    };
    if !readable {
        errors.push(format!("courses: {key}: pairs unreadable"));
    }
    Some(list)
}

/// The keys of every course (not only the first) one of whose codes is `code`: a past-term shell
/// and the live shell of one course both carry it, and both claim the class.
fn courses_with_code(courses: &[Course], code: &str) -> BTreeSet<String> {
    courses.iter().filter(|c| !c.key.is_empty() && c.codes.iter().any(|k| k == code)).map(|c| c.key.clone()).collect()
}

/// One course's `(fragment, key)` pairs as `config.ingest` writes them into `course_map`: the very
/// pairs the app saved for it when the entry carries `pairs` (§14.46), else, for an entry without,
/// derived as R-C1-48 does: its LMS course key and its human code (the label). Each points at the
/// course's `by_course` key.
/// A course is matched on its own pairs (see [`claiming_keys`]), so two shells that share a label
/// both claim an item that names it; no fragment is dropped for being named first elsewhere.
fn course_pairs(c: &Course) -> Vec<(String, String)> {
    let mut out: Vec<(String, String)> = Vec::new();
    if !c.key.is_empty() {
        let derived = [c.code.clone(), c.label.clone()];
        for fragment in c.pairs.as_deref().unwrap_or(&derived).iter().filter(|f| !f.is_empty()) {
            if !out.iter().any(|(f, _)| f == fragment) {
                out.push((fragment.clone(), c.key.clone()));
            }
        }
    }
    out
}

/// The distinct UIDs of the feed's items that ingest's first run would keep live: an item is left
/// out only when its due date is before today in the vault's zone (`ingest.rs`'s `past_due`), so an
/// item timed earlier today still counts and the proof's number equals what lands in the vault.
/// Institutional noise and an event with no due are not items. `None` when the text is not a
/// calendar.
fn read_feed(text: &str, tz: &TimeZone, now: DateTime, errors: &mut Vec<String>) -> Option<Vec<ingest::Event>> {
    if !text.contains("BEGIN:VCALENDAR") {
        errors.push("feed: not a calendar".to_string());
        return None;
    }
    let (events, _malformed) = ingest::parse_ics(text, tz);
    let mut upcoming: BTreeMap<String, ingest::Event> = BTreeMap::new();
    for e in events {
        let live = match e.due {
            Some(Due::DateTime(dt)) => dt.date() >= now.date(),
            Some(Due::Date(d)) => d >= now.date(),
            None => false,
        };
        if live && !ingest::is_institutional_noise(&e.uid) {
            upcoming.entry(e.uid.clone()).or_insert(e);
        }
    }
    Some(upcoming.into_values().collect())
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
    let (mut done, mut courses, mut owner, mut graded_lms) = (BTreeSet::new(), 0, BTreeMap::new(), BTreeSet::new());
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
        if graded && !lms.is_empty() {
            graded_lms.insert(lms);
        }
    }
    Some(Grades { done, courses, owner, graded_lms })
}

/// What the bundle gives: the done item columns, the count of courses with a graded item, and each
/// item column's LMS course key (how a feed item with no course text still reaches its course).
struct Grades {
    done: BTreeSet<String>,
    courses: usize,
    owner: BTreeMap<String, String>,
    /// The LMS course keys with a graded item column (what `grades_courses` counts), for `graded`.
    graded_lms: BTreeSet<String>,
}

/// The registrar's matched class times: Banner's series read `Reading::Whole` (a partly read row is
/// not counted, as the registrar's own confirm skips it) whose course code (a lab's " Lab" dropped)
/// is a code in the course list; each is returned as the keys of every course that carries the code
/// (the match set, as for a feed item). Nothing is written and nothing is fetched.
///
/// A student whose classes are all online has no `CLAS` meeting with a day and times, so the parse
/// fails: `class_times` is `null` plus a named `registrar:` line in `errors`. The reveal treats that
/// as a named outcome (no class times to show), never as a failure.
fn read_registrar(text: &str, school: Option<&str>, courses: &[Course], errors: &mut Vec<String>) -> Option<Vec<BTreeSet<String>>> {
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
            let named = |title: &str| courses_with_code(courses, &compact(title.strip_suffix(" Lab").unwrap_or(title)));
            Some(parsed.series.iter().filter(|s| whole(&s.source_uid)).map(|s| named(&s.title)).filter(|k| !k.is_empty()).collect())
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
    let is_done = |e: &ingest::Event| grades.as_ref().is_some_and(|g| ingest::gradable_id(&e.uid).is_some_and(|id| g.done.contains(id)));
    let turned_in = match (&feed, &grades) {
        (Some(items), Some(_)) => Some(items.iter().filter(|i| is_done(i)).count()),
        _ => None,
    };
    let by_course = courses.as_ref().map(|list| {
        let zero = |on: bool| on.then_some(0);
        let empty = [zero(feed.is_some()), zero(feed.is_some() && grades.is_some()), zero(class_times.is_some())];
        // Every keyed course has its own bucket from the start; a set of two or more codes (an item
        // two courses both claim) is added when an item lands there.
        let mut buckets: BTreeMap<String, ([Option<usize>; 3], Vec<String>)> = BTreeMap::new();
        for c in list.iter().filter(|c| !c.key.is_empty()) {
            buckets.insert(c.key.clone(), (empty, vec![c.key.clone()]));
        }
        buckets.insert(UNMATCHED.to_string(), (empty, Vec::new()));
        let mut add = |codes: &BTreeSet<String>, slot: usize| {
            let key = if codes.is_empty() { UNMATCHED.to_string() } else { codes.iter().cloned().collect::<Vec<_>>().join(",") };
            let entry = buckets.entry(key).or_insert_with(|| (empty, codes.iter().cloned().collect()));
            if let Some(n) = entry.0[slot].as_mut() {
                *n += 1;
            }
        };
        for item in feed.iter().flatten() {
            let codes = claiming_keys(item, list, grades.as_ref());
            add(&codes, 0);
            if is_done(item) {
                add(&codes, 1);
            }
        }
        for keys in class_times.iter().flatten() {
            add(keys, 2);
        }
        // `graded` is on each course's own bucket only: true when the bundle has a graded item column
        // for it, false when it has none, null with no bundle (a multi-code bucket and `unmatched`
        // carry none). Two entries sharing a key are graded if either is.
        let graded = |key: &str| -> Option<Value> {
            let mine = list.iter().filter(|c| c.key == key);
            list.iter().any(|c| c.key == key).then(|| match &grades {
                None => Value::Null,
                Some(g) => json!(mine.into_iter().any(|c| !c.code.is_empty() && g.graded_lms.contains(&c.code))),
            })
        };
        let entry = |k: &str, (n, codes): &([Option<usize>; 3], Vec<String>)| {
            let mut v = json!({"assignments_upcoming": n[0], "class_times": n[2], "codes": codes, "turned_in": n[1]});
            if let (true, Some(g)) = (codes.len() == 1, graded(k)) {
                v["graded"] = g;
            }
            v
        };
        Value::Object(buckets.iter().map(|(k, b)| (k.clone(), entry(k, b))).collect())
    });
    // With a course list, the count of graded single-course buckets (so it equals what the app totals
    // over the buckets, a bundle course left out of the list included or not); else the bundle-wide count.
    let grades_courses = match (&by_course, &grades) {
        (Some(Value::Object(buckets)), Some(_)) => Some(buckets.values().filter(|b| b["graded"] == json!(true)).count()),
        _ => grades.as_ref().map(|g| g.courses),
    };
    json!({
        "assignments_upcoming": feed.as_ref().map(Vec::len),
        "by_course": by_course,
        "class_times": class_times.as_ref().map(Vec::len),
        "classes": courses.as_ref().map(Vec::len),
        "errors": errors,
        "grades_courses": grades_courses,
        "turned_in": turned_in,
    })
}

/// The `by_course` key of the items (and class times) that no course claims: they count in the
/// totals, so the buckets sum to those totals, and under this key so the app can leave them out of a
/// kept-courses total. (An LMS course key never reads like this.)
const UNMATCHED: &str = "unmatched";

/// The keys of the courses that claim a feed item, by ingest's own rule: each course is matched on
/// its own pairs with `ingest::match_course` (a uid pin, then the SUMMARY and CATEGORIES against the
/// `course_map` fragments), so a proof and a vault cannot disagree. Usually one key; two for an item
/// a past-term and a current shell both claim; none for an item no course claims. A gradebook item
/// the bundle holds in a course is pinned to it first, the way a `course_map` entry naming the
/// item's own UID would pin it; that is the only use of the bundle.
fn claiming_keys(item: &ingest::Event, courses: &[Course], grades: Option<&Grades>) -> BTreeSet<String> {
    let owner = grades.and_then(|g| g.owner.get(ingest::gradable_id(&item.uid)?));
    courses
        .iter()
        .filter(|c| !c.key.is_empty())
        .filter(|c| {
            // Saved pairs already carry the course's pins; only a derived course takes the bundle's.
            let pin = owner.filter(|lms| c.pairs.is_none() && !c.code.is_empty() && &c.code == *lms).map(|_| (item.uid.clone(), c.key.clone()));
            let map: Vec<(String, String)> = pin.into_iter().chain(course_pairs(c)).collect();
            ingest::match_course(item, &map).is_some()
        })
        .map(|c| c.key.clone())
        .collect()
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

/// What the journal says an `agent:` actor did to the effort of a note, keyed by note id and by path.
#[derive(Default)]
struct AgentEffort {
    /// An `op: set` of `effort_hours`.
    set_ids: BTreeSet<String>,
    set_paths: BTreeSet<String>,
    /// An `op: create` (the note was minted with its effort already judged, as Gmail's are).
    created_ids: BTreeSet<String>,
    created_paths: BTreeSet<String>,
}

impl AgentEffort {
    /// Did an agent estimate this note's effort? A journal `set` of `effort_hours`, or an agent
    /// `create` of a note whose `effort_source` is `inferred`. The record is matched by the note's
    /// `id`; by `path` only when the note has no `id`, so a note archived and re-created at the
    /// same path does not inherit the old one's estimate.
    fn estimated(&self, path: &str, meta: &Mapping) -> bool {
        let inferred = field_text(meta, "effort_source").as_deref() == Some("inferred");
        match field_text(meta, "id") {
            Some(id) => self.set_ids.contains(&id) || (inferred && self.created_ids.contains(&id)),
            None => self.set_paths.contains(path) || (inferred && self.created_paths.contains(path)),
        }
    }
}

/// The journal's agent effort records, from its day files. `None` and a named line when the folder
/// is missing. Reads through the ledger reader, which writes nothing.
fn agent_effort(vault: &Path, errors: &mut Vec<String>) -> Option<AgentEffort> {
    let mut ledger = crate::ledger::JsonlLedger::new(vault.join("state").join("journal"));
    if !ledger.root.is_dir() {
        errors.push("journal: folder not found".to_string());
        return None;
    }
    let mut out = AgentEffort::default();
    let text = |r: &crate::ledger::Record, k: &str| r.get(k).and_then(Value::as_str).map(str::to_string);
    for r in ledger.read(None, None) {
        if !text(&r, "actor").is_some_and(|a| crate::provenance::is_agent(&a)) {
            continue;
        }
        let (ids, paths) = match (text(&r, "op").as_deref(), text(&r, "field").as_deref()) {
            (Some("set"), Some("effort_hours")) => (&mut out.set_ids, &mut out.set_paths),
            (Some("create"), _) => (&mut out.created_ids, &mut out.created_paths),
            _ => continue,
        };
        ids.extend(text(&r, "id").filter(|i| !i.is_empty()));
        paths.extend(text(&r, "path").map(|p| p.replace('\\', "/")));
    }
    errors.extend(ledger.warnings.iter().map(|w| format!("journal: {w}")));
    Some(out)
}

/// `--tidied`: the reveal's counts from the vault. `imported_past` and `imported_done` count
/// `archive/` notes by `archived_reason`; `live_by_source` counts the active tasks in `tasks/` by
/// `created_by` (none named is `unknown`); `effort_estimated` counts those tasks with at least one
/// `op: set` of `effort_hours` by an `agent:` actor in the journal (or an agent `create` of a note
/// whose `effort_source` is `inferred`), even if the student overrode it since. A folder that is missing is `null` for what it feeds and a named line in `errors`; a vault
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
        let sets = agent_effort(vault, &mut errors);
        if let Some(tasks) = &tasks {
            let mut by_source: BTreeMap<String, usize> = BTreeMap::new();
            for (_, meta) in tasks {
                *by_source.entry(field_text(meta, "created_by").unwrap_or_else(|| "unknown".to_string())).or_default() += 1;
            }
            live = Some(by_source);
        }
        if let (Some(tasks), Some(sets)) = (&tasks, &sets) {
            effort = Some(tasks.iter().filter(|(path, meta)| sets.estimated(path, meta)).count());
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
        // All three bundle courses are listed, so the bucket count and the bundle-wide count agree.
        let list = json!([
            {"code": "_900001_1", "name": "One", "slug": "syn-110", "label": "SYN 110"},
            {"code": "_900002_1", "name": "Two", "slug": "syn-120", "label": "SYN 120"},
            {"code": "_900003_1", "name": "Three", "slug": "syn-130", "label": "SYN 130"},
        ])
        .to_string();
        let f = ics(&[]);
        let inputs = PreviewInputs { feed: Some(&f), courses: Some(&list), grades: Some(&bundle), ..Default::default() };
        let v = preview(&inputs, &chicago(), now());
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

    /// An event with an optional CATEGORIES line, as Blackboard writes the section id.
    fn vevent_in(uid: &str, summary: &str, categories: Option<&str>, description: &str) -> String {
        let cat = categories.map(|c| format!("CATEGORIES:{c}\n")).unwrap_or_default();
        format!("BEGIN:VEVENT\nUID:{uid}\nSUMMARY:{summary}\n{cat}DESCRIPTION:{description}\nDTSTART:20260905T235900\nEND:VEVENT\n")
    }

    fn by_course(v: &Value, key: &str, field: &str) -> Value {
        v["by_course"][key][field].clone()
    }

    /// Every number in `by_course` under `field`, summed over all keys (`unmatched` included).
    fn sum_over_keys(v: &Value, field: &str) -> u64 {
        v["by_course"].as_object().unwrap().values().map(|b| b[field].as_u64().unwrap()).sum()
    }

    fn two_class_banner() -> String {
        let meets = json!([{"meetingType": "CLAS", "monday": true, "beginTime": "0900", "endTime": "0950",
                            "startDate": "08/20/2026", "endDate": "12/05/2026"}]);
        let row = |crn: &str, subject: &str, number: &str| {
            json!({"term": "202640", "courseReferenceNumber": crn, "subject": subject, "courseNumber": number,
                   "scheduleDescription": "Lecture", "statusIndicator": "IR", "meetingTimes": meets.clone()})
        };
        banner_rows(vec![row("90001", "SYN", "110"), row("90002", "SYN", "120"), row("90003", "ZZT", "999")])
    }

    fn full_preview(feed: &str) -> Value {
        let (c, g, r) = (courses(), bundle(), two_class_banner());
        let inputs = PreviewInputs { feed: Some(feed), courses: Some(&c), grades: Some(&g), registrar: Some(&r), school: Some("ua") };
        preview(&inputs, &chicago(), now())
    }

    /// Course one is LMS key `_900001_1` ("SYN 110"), course two `_900002_1` ("SYN 120").
    fn all_mapped_feed() -> String {
        ics(&[
            // The LMS course key in the UID: ingest's uid pass.
            vevent_in("UA-_900001_1-HW2", "Homework 2", None, "x"),
            // The human code in CATEGORIES, and at the head of a SUMMARY.
            vevent_in("BB-ASSIGN-2", "Kapitel 2 Quiz", Some("SYN 120"), "x"),
            vevent_in("BB-ASSIGN-3", "SYN 120: Reading", None, "x"),
            // No course text at all; the grades bundle holds column _71_1 in course _900001_1.
            vevent_in(&format!("{GB}_71_1"), "Renamed", None, "x"),
        ])
    }

    #[test]
    fn per_course_counts_sum_to_the_totals_when_every_item_maps() {
        let v = full_preview(&all_mapped_feed());
        assert_eq!(count(&v, "assignments_upcoming"), json!(4), "{v}");
        assert_eq!(by_course(&v, "_900001_1", "assignments_upcoming"), json!(2), "{v}");
        assert_eq!(by_course(&v, "_900002_1", "assignments_upcoming"), json!(2), "{v}");
        assert_eq!(by_course(&v, "unmatched", "assignments_upcoming"), json!(0), "{v}");
        assert_eq!(by_course(&v, "_900001_1", "turned_in"), json!(1), "{v}");
        assert_eq!(count(&v, "class_times"), json!(2), "ZZT 999 is not in the course list: {v}");
        for (field, total) in [("assignments_upcoming", 4), ("turned_in", 1), ("class_times", 2)] {
            assert_eq!(sum_over_keys(&v, field), total, "{field}: {v}");
        }
    }

    #[test]
    fn an_item_reaching_no_course_is_under_unmatched_and_the_buckets_still_sum_to_the_totals() {
        let feed = ics(&[
            vevent_in("UA-_900001_1-HW", "Homework", None, "x"),
            vevent_in("BB-ASSIGN-2", "Orphan [ZZT-999-001-FALL2026]", None, "x"),
            // Ingest's rule: a SUMMARY that names a course code at a boundary is that course, wherever
            // in the sentence. Only DESCRIPTION is never read, and SYN 1100 is not SYN 110.
            vevent_in("BB-ASSIGN-3", "Quiz covering SYN 120", None, "x"),
            vevent_in("BB-ASSIGN-4", "Quiz A", None, "SYN 120 reading"),
            vevent_in("BB-ASSIGN-5", "Quiz B", Some("SYN 1100"), "x"),
        ]);
        let v = full_preview(&feed);
        assert_eq!(count(&v, "assignments_upcoming"), json!(5), "{v}");
        assert_eq!(by_course(&v, "_900001_1", "assignments_upcoming"), json!(1), "{v}");
        assert_eq!(by_course(&v, "_900002_1", "assignments_upcoming"), json!(1), "{v}");
        assert_eq!(by_course(&v, "unmatched", "assignments_upcoming"), json!(3), "{v}");
        assert_eq!(sum_over_keys(&v, "assignments_upcoming"), 5, "{v}");
    }

    #[test]
    fn attribution_is_ingests_own_rule_over_each_courses_pairs() {
        // The claim is `ingest::match_course` over the pairs `config.ingest` writes for the course,
        // so the proof's course is the course the vault's task gets, whatever that rule is.
        let c = courses();
        let list = read_courses(&c, &mut Vec::new()).unwrap();
        assert_eq!(course_pairs(&list[0]), [("_900001_1".to_string(), "_900001_1".to_string()), ("SYN 110".into(), "_900001_1".into())]);
        let (events, _) = ingest::parse_ics(&all_mapped_feed(), &chicago());
        for e in &events {
            let theirs: BTreeSet<String> =
                list.iter().filter(|c| ingest::match_course(e, &course_pairs(c)).is_some()).map(|c| c.key.clone()).collect();
            let ours = claiming_keys(e, &list, None);
            assert_eq!(ours, theirs, "{}", e.uid);
        }
    }

    /// What the app totals for the kept courses: every bucket whose code set is empty or names a kept code.
    fn kept_total(v: &Value, kept: &[&str], field: &str) -> u64 {
        v["by_course"]
            .as_object()
            .unwrap()
            .values()
            .filter(|b| {
                let codes = b["codes"].as_array().unwrap();
                codes.is_empty() || codes.iter().any(|c| kept.contains(&c.as_str().unwrap()))
            })
            .map(|b| b[field].as_u64().unwrap())
            .sum()
    }

    #[test]
    fn two_shells_of_one_course_are_two_keys_and_a_shared_item_sits_in_a_two_code_bucket_once() {
        // A past-term shell and the live one share a slug and a label but not an LMS key.
        let c = json!([
            {"code": "_100_1", "name": "Old", "slug": "syn-110", "label": "SYN 110"},
            {"code": "_200_1", "name": "Live", "slug": "syn-110", "label": "SYN 110"},
        ])
        .to_string();
        let f = ics(&[
            vevent_in("UA-_100_1-A", "Old only", None, "x"),
            vevent_in("UA-_200_1-B", "Live only", None, "x"),
            vevent_in("BB-ASSIGN-3", "SYN 110: Reading", None, "x"), // both shells claim it
            vevent_in("BB-ASSIGN-4", "Nobody's", None, "x"),
        ]);
        let inputs = PreviewInputs { feed: Some(&f), courses: Some(&c), ..Default::default() };
        let v = preview(&inputs, &chicago(), now());
        assert_eq!(count(&v, "classes"), json!(2), "{v}");
        assert_eq!(count(&v, "assignments_upcoming"), json!(4), "{v}");
        assert_eq!(by_course(&v, "_100_1", "assignments_upcoming"), json!(1), "{v}");
        assert_eq!(by_course(&v, "_200_1", "assignments_upcoming"), json!(1), "{v}");
        assert_eq!(by_course(&v, "_100_1,_200_1", "assignments_upcoming"), json!(1), "counted once: {v}");
        assert_eq!(v["by_course"]["_100_1,_200_1"]["codes"], json!(["_100_1", "_200_1"]), "{v}");
        assert_eq!(by_course(&v, "unmatched", "assignments_upcoming"), json!(1), "{v}");
        assert_eq!(sum_over_keys(&v, "assignments_upcoming"), 4, "{v}");
        // The app's total over kept codes: removing one shell keeps the shared item, removing both drops it.
        let both = ["_100_1", "_200_1"];
        assert_eq!(kept_total(&v, &both, "assignments_upcoming"), 4, "{v}");
        assert_eq!(kept_total(&v, &["_200_1"], "assignments_upcoming"), 3, "live only, shared, unmatched: {v}");
        assert_eq!(kept_total(&v, &[], "assignments_upcoming"), 1, "only the unmatched item is always in: {v}");
    }

    #[test]
    fn a_class_time_two_shells_both_name_sits_in_their_two_code_bucket_once() {
        // The old shell is listed first and carries the same "SYN 110": the meeting is claimed by
        // both, so removing the old shell must not take the class with it.
        let c = json!([
            {"code": "_100_1", "name": "Old", "slug": "syn-110", "label": "SYN 110"},
            {"code": "_200_1", "name": "Live", "slug": "syn-110", "label": "SYN 110"},
            {"code": "_300_1", "name": "Other", "slug": "syn-120", "label": "SYN 120"},
        ])
        .to_string();
        let b = banner_rows(vec![banner_row("90001", "110", "Lecture", json!([clas(Some("0900"))]))]);
        let f = ics(&[]);
        let inputs = PreviewInputs { feed: Some(&f), courses: Some(&c), registrar: Some(&b), school: Some("ua"), ..Default::default() };
        let v = preview(&inputs, &chicago(), now());
        assert_eq!(count(&v, "class_times"), json!(1), "one meeting, one class time: {v}");
        assert_eq!(by_course(&v, "_100_1,_200_1", "class_times"), json!(1), "{v}");
        assert_eq!(by_course(&v, "_100_1", "class_times"), json!(0), "{v}");
        assert_eq!(sum_over_keys(&v, "class_times"), 1, "{v}");
        // §14.47: the registrar holds only the current term, so a removal never subtracts a class
        // time. The buckets are for display; the app keeps the total (1) whichever shell, or both,
        // the student removes, so nothing here reads `kept_total` for `class_times`.
        assert_eq!(count(&v, "class_times"), json!(1), "{v}");
    }

    #[test]
    fn graded_marks_each_single_course_bucket_and_a_course_with_no_items_still_has_one() {
        // No feed items at all: both courses still have a bucket, with zero counts.
        let (c, f) = (courses(), ics(&[]));
        let g = bundle(); // course one has a graded item column; course two only an Overall column
        let with = PreviewInputs { feed: Some(&f), courses: Some(&c), grades: Some(&g), ..Default::default() };
        let v = preview(&with, &chicago(), now());
        assert_eq!(by_course(&v, "_900001_1", "graded"), json!(true), "{v}");
        assert_eq!(by_course(&v, "_900002_1", "graded"), json!(false), "{v}");
        assert_eq!(by_course(&v, "_900002_1", "assignments_upcoming"), json!(0), "{v}");
        assert!(v["by_course"]["unmatched"].get("graded").is_none(), "{v}");
        assert_eq!(count(&v, "grades_courses"), json!(1), "the total counts the true buckets: {v}");
        // No bundle: null on both.
        let without = PreviewInputs { feed: Some(&f), courses: Some(&c), ..Default::default() };
        let w = preview(&without, &chicago(), now());
        assert_eq!(by_course(&w, "_900001_1", "graded"), Value::Null, "{w}");
        assert_eq!(by_course(&w, "_900002_1", "graded"), Value::Null, "{w}");
        assert!(w["by_course"]["_900001_1"].get("graded").is_some(), "null, but present: {w}");
    }

    #[test]
    fn grades_courses_is_the_count_of_graded_buckets_when_courses_are_given() {
        // The bundle grades course one (_900001_1), which this list leaves out: the bundle-wide count
        // would say 1, the buckets say 0, and the top level must equal the buckets.
        let c = json!([{"code": "_900002_1", "name": "Synthetic Studies II", "slug": "syn-120", "label": "SYN 120"}]).to_string();
        let (f, g) = (ics(&[]), bundle());
        let inputs = PreviewInputs { feed: Some(&f), courses: Some(&c), grades: Some(&g), ..Default::default() };
        let v = preview(&inputs, &chicago(), now());
        assert_eq!(by_course(&v, "_900002_1", "graded"), json!(false), "{v}");
        assert_eq!(count(&v, "grades_courses"), json!(0), "{v}");
        // With the graded course listed it is 1, and with no bundle it stays null.
        let both = courses();
        let inputs = PreviewInputs { feed: Some(&f), courses: Some(&both), grades: Some(&g), ..Default::default() };
        assert_eq!(count(&preview(&inputs, &chicago(), now()), "grades_courses"), json!(1));
        let none = PreviewInputs { feed: Some(&f), courses: Some(&c), ..Default::default() };
        assert_eq!(count(&preview(&none, &chicago(), now()), "grades_courses"), Value::Null);
        // Without `--courses` nothing changes: the bundle-wide count.
        let no_courses = PreviewInputs { feed: Some(&f), grades: Some(&g), ..Default::default() };
        assert_eq!(count(&preview(&no_courses, &chicago(), now()), "grades_courses"), json!(1));
    }

    #[test]
    fn a_multi_code_bucket_carries_no_graded() {
        let c = json!([
            {"code": "_100_1", "name": "Old", "slug": "syn-110", "label": "SYN 110"},
            {"code": "_200_1", "name": "Live", "slug": "syn-110", "label": "SYN 110"},
        ])
        .to_string();
        let f = ics(&[vevent_in("BB-ASSIGN-3", "SYN 110: Reading", None, "x")]);
        let g = bundle();
        let inputs = PreviewInputs { feed: Some(&f), courses: Some(&c), grades: Some(&g), ..Default::default() };
        let v = preview(&inputs, &chicago(), now());
        assert_eq!(by_course(&v, "_100_1,_200_1", "assignments_upcoming"), json!(1), "{v}");
        assert!(v["by_course"]["_100_1,_200_1"].get("graded").is_none(), "{v}");
        assert_eq!(by_course(&v, "_100_1", "graded"), json!(false), "neither shell is in the bundle: {v}");
    }

    /// A course list whose first entry carries saved `pairs` (strings, and `[fragment, slug]` pairs
    /// as `course_map` holds them) and whose label the student has since edited.
    fn courses_with_saved_pairs() -> String {
        json!([
            {"code": "_900001_1", "name": "Synthetic Studies I", "slug": "renamed-slug", "label": "My edited label",
             "pairs": ["_900001_1", ["SYN 110", "syn-110"]]},
            {"code": "_900002_1", "name": "Synthetic Studies II", "slug": "syn-120", "label": "SYN 120"},
        ])
        .to_string()
    }

    #[test]
    fn a_provided_pair_overrides_an_edited_label() {
        let c = courses_with_saved_pairs();
        let f = ics(&[
            vevent_in("BB-ASSIGN-1", "SYN 110: Reading", None, "x"), // the saved human code
            vevent_in("BB-ASSIGN-2", "SYN 110: Quiz", None, "x"),
            vevent_in("BB-ASSIGN-3", "My edited label: notes", None, "x"), // the edited label: no longer a fragment
            vevent_in("BB-ASSIGN-4", "SYN 120: Lab", None, "x"), // a course with no `pairs` derives its own
        ]);
        let b = banner_rows(vec![banner_row("90001", "110", "Lecture", json!([clas(Some("0900"))]))]);
        let inputs = PreviewInputs { feed: Some(&f), courses: Some(&c), registrar: Some(&b), school: Some("ua"), ..Default::default() };
        let v = preview(&inputs, &chicago(), now());
        assert_eq!(by_course(&v, "_900001_1", "assignments_upcoming"), json!(2), "{v}");
        assert_eq!(by_course(&v, "_900002_1", "assignments_upcoming"), json!(1), "{v}");
        assert_eq!(by_course(&v, "unmatched", "assignments_upcoming"), json!(1), "{v}");
        // The class time names SYN 110, which the saved pairs carry; the label plays no part.
        assert_eq!(by_course(&v, "_900001_1", "class_times"), json!(1), "{v}");
    }

    #[test]
    fn unreadable_pairs_are_a_named_error_and_never_a_silent_re_derivation() {
        let c = json!([
            {"code": "_1_1", "slug": "a", "label": "SYN 110", "pairs": "SYN 110"},       // not an array
            {"code": "_2_1", "slug": "b", "label": "SYN 120", "pairs": ["SYN 120", 5]},  // one element unreadable
            {"code": "_3_1", "slug": "c", "label": "SYN 130"},                           // no pairs key: derived
            {"code": "_4_1", "slug": "d", "label": "SYN 140", "pairs": null},            // present, not an array
        ])
        .to_string();
        let f = ics(&[
            vevent_in("BB-ASSIGN-1", "SYN 110: x", None, "x"),
            vevent_in("BB-ASSIGN-2", "SYN 120: x", None, "x"),
            vevent_in("BB-ASSIGN-3", "SYN 130: x", None, "x"),
            vevent_in("BB-ASSIGN-4", "SYN 140: x", None, "x"),
        ]);
        let inputs = PreviewInputs { feed: Some(&f), courses: Some(&c), ..Default::default() };
        let v = preview(&inputs, &chicago(), now());
        assert_eq!(v["errors"], json!(["courses: _1_1: pairs unreadable", "courses: _2_1: pairs unreadable", "courses: _4_1: pairs unreadable"]), "{v}");
        assert_eq!(by_course(&v, "_1_1", "assignments_upcoming"), json!(0), "no fragments, and no label fallback: {v}");
        assert_eq!(by_course(&v, "_2_1", "assignments_upcoming"), json!(1), "the readable element stands: {v}");
        assert_eq!(by_course(&v, "_3_1", "assignments_upcoming"), json!(1), "{v}");
        assert_eq!(by_course(&v, "_4_1", "assignments_upcoming"), json!(0), "{v}");
        assert_eq!(by_course(&v, "unmatched", "assignments_upcoming"), json!(2), "{v}");
    }

    #[test]
    fn saved_pairs_carry_the_gradebook_pins_and_the_bundle_pins_nothing_of_its_own() {
        let g = bundle(); // column _71_1 is held in course _900001_1
        let feed = ics(&[vevent(&format!("{GB}_71_1"), "Renamed", "20260905T235900")]);
        let run = |c: &str| {
            let inputs = PreviewInputs { feed: Some(&feed), courses: Some(c), grades: Some(&g), ..Default::default() };
            preview(&inputs, &chicago(), now())
        };
        // Saved pairs with the pin: claimed by its fragment.
        let with_pin = json!([{"code": "_900001_1", "slug": "syn-110", "label": "SYN 110", "pairs": ["SYN 110", "_71_1"]}]).to_string();
        let v = run(&with_pin);
        assert_eq!(by_course(&v, "_900001_1", "assignments_upcoming"), json!(1), "{v}");
        assert_eq!(by_course(&v, "_900001_1", "turned_in"), json!(1), "{v}");
        // Saved pairs without it: the bundle is not consulted for this course.
        let without = json!([{"code": "_900001_1", "slug": "syn-110", "label": "SYN 110", "pairs": ["SYN 110"]}]).to_string();
        let w = run(&without);
        assert_eq!(by_course(&w, "_900001_1", "assignments_upcoming"), json!(0), "{w}");
        assert_eq!(by_course(&w, "unmatched", "assignments_upcoming"), json!(1), "{w}");
        // No `pairs` at all: the engine's own derivation, with the bundle's pin, as before.
        let derived = json!([{"code": "_900001_1", "slug": "syn-110", "label": "SYN 110"}]).to_string();
        assert_eq!(by_course(&run(&derived), "_900001_1", "assignments_upcoming"), json!(1));
    }

    #[test]
    fn by_course_counts_are_null_where_their_input_is_missing() {
        let (f, c) = (all_mapped_feed(), courses());
        let inputs = PreviewInputs { feed: Some(&f), courses: Some(&c), ..Default::default() };
        let v = preview(&inputs, &chicago(), now());
        // Without the bundle, the id-only item (_71_1) has no course: it is unmatched.
        assert_eq!(by_course(&v, "_900001_1", "assignments_upcoming"), json!(1), "{v}");
        assert_eq!(by_course(&v, "unmatched", "assignments_upcoming"), json!(1), "{v}");
        assert_eq!(by_course(&v, "_900001_1", "turned_in"), Value::Null, "{v}");
        assert_eq!(by_course(&v, "unmatched", "class_times"), Value::Null, "{v}");
        let (c, g) = (courses(), bundle());
        let no_feed = PreviewInputs { courses: Some(&c), grades: Some(&g), ..Default::default() };
        let w = preview(&no_feed, &chicago(), now());
        assert_eq!(by_course(&w, "_900001_1", "assignments_upcoming"), Value::Null, "{w}");
        assert_eq!(by_course(&w, "_900001_1", "turned_in"), Value::Null, "{w}");
    }

    #[test]
    fn by_course_is_keyed_by_code_then_slug_then_label_and_null_without_a_course_list() {
        let c = json!([
            {"code": "_1_1", "name": "A", "slug": "syn-110", "label": "SYN 110"},
            {"code": "", "name": "B", "slug": "syn-120", "label": "SYN 120"},
            {"code": "", "name": "C", "slug": "", "label": "SYN 130"},
            {"code": "", "name": "D", "slug": "", "label": ""},
        ])
        .to_string();
        let f = all_mapped_feed();
        let inputs = PreviewInputs { feed: Some(&f), courses: Some(&c), ..Default::default() };
        let v = preview(&inputs, &chicago(), now());
        assert_eq!(count(&v, "classes"), json!(4), "{v}");
        let keys: Vec<&String> = v["by_course"].as_object().unwrap().keys().collect();
        assert_eq!(keys, ["SYN 130", "_1_1", "syn-120", "unmatched"], "a course with none of the three has no key: {v}");
        assert_eq!(by_course(&v, "syn-120", "assignments_upcoming"), json!(2), "{v}");
        let none = PreviewInputs { feed: Some(&f), ..Default::default() };
        assert_eq!(count(&preview(&none, &chicago(), now()), "by_course"), Value::Null);
    }

    #[test]
    fn a_crlf_feed_with_a_folded_summary_buckets_like_an_lf_one() {
        // CRLF line ends, and a SUMMARY folded in the middle of the course code's line.
        let crlf = "BEGIN:VCALENDAR\r\nVERSION:2.0\r\nBEGIN:VEVENT\r\nUID:BB-ASSIGN-1\r\nSUMMARY:Reading for SYN\r\n  110 today\r\nDTSTART:20260905T235900\r\nEND:VEVENT\r\nBEGIN:VEVENT\r\nUID:UA-_900002_1-X\r\nSUMMARY:Other\r\nDTSTART:20260906T235900\r\nEND:VEVENT\r\nEND:VCALENDAR\r\n";
        let v = full_preview(crlf);
        assert_eq!(count(&v, "assignments_upcoming"), json!(2), "{v}");
        assert_eq!(by_course(&v, "_900001_1", "assignments_upcoming"), json!(1), "the folded SUMMARY names SYN 110: {v}");
        assert_eq!(by_course(&v, "_900002_1", "assignments_upcoming"), json!(1), "{v}");
        assert_eq!(by_course(&v, "unmatched", "assignments_upcoming"), json!(0), "{v}");
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
