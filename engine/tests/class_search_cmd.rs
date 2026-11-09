//! Replan C3 (`docs/plans/2026-11-04-onboarding-replan-draft.md` §6.5, §6.7, §8 "C3"):
//! `knowlu-engine commitments --class-search` and `onboarding --preview --class-search`, run as the
//! binary on scratch vaults. Every course, CRN, room and time is invented. The bundle is the shape
//! `app/src/class_search.rs` writes: `{school, term, queries: [{subject, number, sections, status,
//! body}]}`, `body` the raw answer text.

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};
use std::process::{Command, Output};

use serde_json::{json, Value};

const SERIES: &str = "state/calendar-series.json";
const LOCK: &str = "state/calendar-series.lock";
/// A fall-term day (`term_for("ua", ..)` is `202640`), before every invented section's dates.
const FALL_DAY: &str = "2026-09-01";
const NOW: &str = "2026-09-01T12:00";

fn binary() -> PathBuf {
    PathBuf::from(env!("CARGO_BIN_EXE_knowlu-engine"))
}

fn scratch(name: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!("knowlu-class-search-cmd-{}-{name}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    for sub in ["config", "courses", "state/journal"] {
        std::fs::create_dir_all(dir.join(sub)).unwrap();
    }
    for (stem, code) in [("xyz-100", "XYZ 100"), ("abc-200", "ABC 200")] {
        let note = format!("---\ntitle: \"{code} Invented Studies\"\ncode: \"{code}\"\n---\n");
        std::fs::write(dir.join("courses").join(format!("{stem}.md")), note).unwrap();
    }
    std::fs::write(dir.join("state/journal/2026-08-30.jsonl"), "{\"actor\": \"student\", \"path\": \"courses/xyz-100.md\"}\n").unwrap();
    dir
}

/// Every file under the vault with its bytes, by relative path.
fn snapshot(vault: &Path) -> BTreeMap<String, Vec<u8>> {
    fn walk(dir: &Path, root: &Path, out: &mut BTreeMap<String, Vec<u8>>) {
        for entry in std::fs::read_dir(dir).into_iter().flatten().flatten() {
            let path = entry.path();
            if path.is_dir() {
                walk(&path, root, out);
            } else {
                out.insert(path.strip_prefix(root).unwrap().to_string_lossy().replace('\\', "/"), std::fs::read(&path).unwrap());
            }
        }
    }
    let mut out = BTreeMap::new();
    walk(vault, vault, &mut out);
    out
}

/// The paths added, changed or removed between two snapshots.
fn changed(before: &BTreeMap<String, Vec<u8>>, after: &BTreeMap<String, Vec<u8>>) -> Vec<String> {
    let mut out: Vec<String> = after.iter().filter(|(k, v)| before.get(*k) != Some(*v)).map(|(k, _)| k.clone()).collect();
    out.extend(before.keys().filter(|k| !after.contains_key(*k)).cloned());
    out.sort();
    out
}

// ---- the invented answer ----

fn meeting(kind: &str, begin: &str, end: &str, days: &[&str]) -> Value {
    let mut m = json!({ "meetingType": kind, "meetingScheduleType": "LEC", "beginTime": begin, "endTime": end,
        "startDate": "08/24/2026", "endDate": "12/11/2026", "building": "Invented Hall", "room": "12", "hoursWeek": 2.5 });
    for d in ["monday", "tuesday", "wednesday", "thursday", "friday", "saturday", "sunday"] {
        m[d] = json!(days.contains(&d));
    }
    m
}

fn row(subject: &str, number: &str, crn: &str, meetings: Vec<Value>) -> Value {
    let faculty: Vec<Value> = meetings.into_iter().map(|m| json!({ "meetingTime": m })).collect();
    json!({ "term": "202640", "subject": subject, "courseNumber": number, "sequenceNumber": "001",
        "courseReferenceNumber": crn, "scheduleTypeDescription": "Lecture", "isSectionLinked": false,
        "linkIdentifier": null, "meetingsFaculty": faculty })
}

/// One query, answered with `rows`.
fn answered(subject: &str, number: &str, rows: Vec<Value>) -> Value {
    let body = json!({ "success": true, "totalCount": rows.len(), "data": rows }).to_string();
    json!({ "subject": subject, "number": number, "sections": ["001"], "status": "answered", "body": body })
}

/// One query that never got an answer (`closed`, `paused` or `fetch_failed`).
fn unanswered(subject: &str, number: &str, status: &str) -> Value {
    json!({ "subject": subject, "number": number, "sections": ["001"], "status": status, "body": "" })
}

/// One query for every state §6.4's parse and the bundle's statuses can end in, once each.
fn all_states() -> Vec<Value> {
    vec![
        answered("XYZ", "100", vec![row("XYZ", "100", "90001", vec![meeting("CLAS", "0900", "0950", &["monday", "wednesday", "friday"])])]),
        answered("ABC", "200", vec![row("ABC", "200", "90002", Vec::new())]),
        answered("MID", "300", vec![row("MID", "300", "90003", vec![meeting("CLAS", "2300", "0100", &["friday"])])]),
        answered("NOP", "400", Vec::new()),
        answered(
            "AMB",
            "110",
            vec![
                row("AMB", "110", "90004", vec![meeting("CLAS", "1000", "1050", &["tuesday"])]),
                row("AMB", "110", "90005", vec![meeting("CLAS", "1400", "1450", &["thursday"])]),
            ],
        ),
        unanswered("CLS", "120", "closed"),
        unanswered("PSD", "130", "paused"),
        unanswered("FLD", "140", "fetch_failed"),
    ]
}

fn bundle_of(term: &str, queries: Vec<Value>) -> String {
    json!({ "school": "ua", "term": term, "queries": queries }).to_string()
}

const STATES: [&str; 8] = ["found", "no_time", "part_read", "not_found", "ambiguous", "closed", "paused", "fetch_failed"];

// ---- running ----

/// The bundle sits beside the vault, never inside it, so a snapshot of the vault sees only vault bytes.
fn put_beside(vault: &Path, text: &str) -> PathBuf {
    let file = vault.with_extension("bundle.json");
    std::fs::write(&file, text).unwrap();
    file
}

fn run_cs(vault: &Path, bundle: &str, today: &str, extra: &[&str]) -> Output {
    let file = put_beside(vault, bundle);
    Command::new(binary())
        .args(["commitments", "--vault"])
        .arg(vault)
        .args(["--today", today, "--class-search"])
        .arg(&file)
        .args(extra)
        .output()
        .unwrap()
}

/// `--class-search` on the all-states bundle, fall day, school `ua`; the report and what changed.
fn ran(name: &str) -> (PathBuf, Value, Vec<String>) {
    let v = scratch(name);
    let before = snapshot(&v);
    let out = run_cs(&v, &bundle_of("202640", all_states()), FALL_DAY, &["--school", "ua"]);
    assert_eq!(out.status.code(), Some(0), "{out:?}");
    let report: Value = serde_json::from_slice(&out.stdout).unwrap_or_else(|e| panic!("stdout is a JSON report ({e}): {out:?}"));
    let after = changed(&before, &snapshot(&v));
    (v, report, after)
}

fn series_file(v: &Path) -> Value {
    let text = std::fs::read_to_string(v.join(SERIES)).unwrap_or_else(|e| panic!("{SERIES} not written: {e}"));
    serde_json::from_str(&text).unwrap()
}

fn compact(s: &str) -> String {
    s.chars().filter(char::is_ascii_alphanumeric).collect::<String>().to_lowercase()
}

// ---- commitments --class-search ----

/// Writes `state/calendar-series.json` and nothing else: no note, no card, no journal record. The
/// lock file may stay. Red now: the stub writes nothing at all.
#[test]
fn class_search_writes_the_series_file_only() {
    let (v, _, after) = ran("series-only");
    assert!(after.contains(&SERIES.to_string()), "the series file was not written: {after:?}");
    let stray: Vec<_> = after.iter().filter(|p| *p != SERIES && *p != LOCK).collect();
    assert!(stray.is_empty(), "wrote more than the series file: {stray:?}");
    let _ = std::fs::remove_dir_all(&v);
}

/// A `found` row is one series under a `classsearch:` key and calendar (§6.4's keys), and the
/// calendar is read fresh today. One section matched, so one series: no other state makes one.
#[test]
fn found_rows_carry_class_search_keys() {
    let (v, _, _) = ran("found-keys");
    let file = series_file(&v);
    let series = file["series"].as_array().unwrap();
    assert_eq!(series.len(), 1, "{file}");
    assert_eq!(series[0]["source_uid"], json!("classsearch:ua:202640-90001"), "{file}");
    assert_eq!(series[0]["calendar"], json!("classsearch:ua:202640"), "{file}");
    assert_eq!(file["calendars"]["classsearch:ua:202640"], json!(FALL_DAY), "{file}");
    let _ = std::fs::remove_dir_all(&v);
}

/// A `no_time` section makes no series; its course lands in the block under the term, as a course
/// key only (the slug and its code compact alike), and nothing of the section is in the file.
#[test]
fn no_time_courses_land_in_the_block() {
    let (v, _, _) = ran("no-time-block");
    let file = series_file(&v);
    let block = file["class_search"].as_object().unwrap_or_else(|| panic!("no class_search block: {file}"));
    assert_eq!(block.keys().collect::<Vec<_>>(), ["classsearch:ua:202640"], "{file}");
    let listed: Vec<String> = block["classsearch:ua:202640"]["no_time"].as_array().unwrap().iter().map(|c| compact(c.as_str().unwrap())).collect();
    assert_eq!(listed, ["abc200"], "{file}");
    assert!(!file["series"].to_string().contains("90002"), "a no-time section makes no series: {file}");
    let _ = std::fs::remove_dir_all(&v);
}

/// The report holds the term and one count per state (flat keys, one section each here), and no
/// class name, code, CRN, room or time: not even through the midnight row's parse warning, which
/// names its class (`class search: MID 300 meets across midnight`). Red now: the stub prints a line.
#[test]
fn the_report_holds_counts_and_fixed_words_only() {
    let (v, report, _) = ran("report");
    assert_eq!(report["term"], json!("202640"), "{report}");
    for state in STATES {
        assert_eq!(report[state], json!(1), "{state}: {report}");
    }
    assert!(report["warnings"].is_array(), "{report}");
    let text = report.to_string();
    for leaked in ["XYZ", "ABC", "MID", "NOP", "AMB", "90001", "90003", "Invented", "0900", "09:00", "2300"] {
        assert!(!text.contains(leaked), "the report leaks {leaked:?}: {text}");
    }
    let _ = std::fs::remove_dir_all(&v);
}

/// A bundle that cannot be read is a named line at exit 0 with nothing written. It is never exit 2
/// (unlike `--registrar`): a student with no usable bundle must not fail a slot.
#[test]
fn a_bad_bundle_exits_0_with_a_named_line_and_writes_nothing() {
    let cases: [(&str, String); 5] = [
        ("html", "<!DOCTYPE html><html>Sign in</html>".to_string()),
        ("empty", String::new()),
        ("noqueries", json!({ "school": "ua", "term": "202640" }).to_string()),
        ("queries-not-a-list", json!({ "school": "ua", "term": "202640", "queries": "x" }).to_string()),
        ("array", "[]".to_string()),
    ];
    for (name, text) in cases {
        let v = scratch(&format!("bad-{name}"));
        let before = snapshot(&v);
        let out = run_cs(&v, &text, FALL_DAY, &["--school", "ua"]);
        assert_eq!(out.status.code(), Some(0), "{name}: {out:?}");
        let line = String::from_utf8_lossy(&out.stdout).to_string();
        assert!(line.trim().lines().count() == 1 && line.to_lowercase().contains("class"), "{name}: a named line, got {line:?}");
        assert_eq!(snapshot(&v), before, "{name} wrote something");
        let _ = std::fs::remove_dir_all(&v);
    }
    // A bundle file that is not there is the same.
    let v = scratch("bad-missing");
    let before = snapshot(&v);
    let out = Command::new(binary())
        .args(["commitments", "--vault"])
        .arg(&v)
        .args(["--today", FALL_DAY, "--school", "ua", "--class-search"])
        .arg(v.with_extension("nothing-here.json"))
        .output()
        .unwrap();
    assert_eq!(out.status.code(), Some(0), "{out:?}");
    assert!(!out.stdout.is_empty(), "a named line, got nothing");
    assert_eq!(snapshot(&v), before);
    let _ = std::fs::remove_dir_all(&v);
}

/// An unknown `--school` is a named line (exit 0, nothing written), as the bad bundle is.
#[test]
fn an_unknown_school_is_a_named_line_and_writes_nothing() {
    let v = scratch("school");
    let before = snapshot(&v);
    let out = run_cs(&v, &bundle_of("202640", all_states()), FALL_DAY, &["--school", "zz"]);
    assert_eq!(out.status.code(), Some(0), "{out:?}");
    let line = String::from_utf8_lossy(&out.stdout).to_string();
    assert!(line.trim().lines().count() == 1 && line.to_lowercase().contains("school"), "a line naming the school: {line:?}");
    assert_eq!(snapshot(&v), before, "wrote something");
    let _ = std::fs::remove_dir_all(&v);
}

/// `--class-search` conflicts with `--confirm` and with `--registrar` (clap's usage error, exit 2,
/// as `--registrar` and `--confirm` conflict), and writes nothing.
#[test]
fn it_conflicts_with_confirm_and_with_registrar() {
    for other in ["--confirm", "--registrar"] {
        let v = scratch(&format!("conflict{other}"));
        let before = snapshot(&v);
        let out = run_cs(&v, &bundle_of("202640", all_states()), FALL_DAY, &["--school", "ua", other, "x.json"]);
        assert_eq!(out.status.code(), Some(2), "{other}: {out:?}");
        assert!(out.stdout.is_empty(), "{other}: {out:?}");
        assert_eq!(snapshot(&v), before, "{other} wrote something");
        let _ = std::fs::remove_dir_all(&v);
    }
}

/// §6.7 step 1: "Only `found` rows that name a kept course become series." A section whose course the
/// vault does not hold makes no series. (Inferred from §6.7; the count it earns is not pinned.)
#[test]
fn a_found_row_naming_no_vault_course_makes_no_series() {
    let v = scratch("no-course");
    let queries = vec![
        answered("QQQ", "500", vec![row("QQQ", "500", "90009", vec![meeting("CLAS", "1300", "1350", &["monday"])])]),
        // The control, so the file exists and the test cannot pass by writing nothing.
        answered("XYZ", "100", vec![row("XYZ", "100", "90001", vec![meeting("CLAS", "0900", "0950", &["monday"])])]),
    ];
    let out = run_cs(&v, &bundle_of("202640", queries), FALL_DAY, &["--school", "ua"]);
    assert_eq!(out.status.code(), Some(0), "{out:?}");
    let text = std::fs::read_to_string(v.join(SERIES)).unwrap_or_else(|e| panic!("{SERIES} not written: {e}"));
    assert!(text.contains("classsearch:ua:202640-90001"), "the control row is missing: {text}");
    assert!(!text.contains("classsearch:ua:202640-90009"), "a course the vault lacks got a series: {text}");
    let _ = std::fs::remove_dir_all(&v);
}

// ---- the term gap (C2b's binding note) ----

fn uncovered(v: &Path, today: &str) -> Vec<String> {
    let out = Command::new(binary()).args(["commitments", "--vault"]).arg(v).args(["--json", "--today", today]).output().unwrap();
    assert_eq!(out.status.code(), Some(0), "{out:?}");
    let j: Value = serde_json::from_slice(&out.stdout).unwrap();
    j["uncovered_courses"].as_array().unwrap().iter().map(|c| c["slug"].as_str().unwrap().to_string()).collect()
}

/// The vault holds a registrar calendar for spring 2026 (`202610`) that has ended, so
/// `registrar::fetch_term` answers the coming fall (`202640`), the term the app reads class search
/// at, while `term_for` still says `202610` until June, and `no_time_courses` reads at `term_for`.
/// A no-time course the bundle names must stop its ask in both halves of that gap: on 2026-05-20,
/// when the reader is at `202610`, and on 2026-06-02, once `202640` has begun. A block written only
/// under the bundle's term fails the first; one written only under `term_for`'s fails the second
/// (the first `rank` of the new term drops it). The test names the outcome, not how it is reached.
#[test]
fn the_no_time_block_suppresses_asks_across_the_gap_between_terms() {
    let v = scratch("gap");
    std::fs::write(v.join(SERIES), "{\"calendars\": {\"registrar:ua:202610\": \"2026-05-01\"}, \"ended\": {}, \"series\": []}\n").unwrap();
    // The control: with no block, the course is uncovered on both days.
    assert!(uncovered(&v, "2026-05-20").contains(&"abc-200".to_string()), "the control: no block, uncovered");
    assert!(uncovered(&v, "2026-06-02").contains(&"abc-200".to_string()), "the control: no block, uncovered");
    let queries = vec![answered("ABC", "200", vec![row("ABC", "200", "90002", Vec::new())])];
    let out = run_cs(&v, &bundle_of("202640", queries), "2026-05-20", &["--school", "ua"]);
    assert_eq!(out.status.code(), Some(0), "{out:?}");
    assert!(!uncovered(&v, "2026-05-20").contains(&"abc-200".to_string()), "gap: the reader at 202610 must see the block");
    assert!(!uncovered(&v, "2026-06-02").contains(&"abc-200".to_string()), "202640 has begun: the block must still stand");
    let _ = std::fs::remove_dir_all(&v);
}

// ---- onboarding --preview --class-search ----

fn preview_files(tag: &str) -> (PathBuf, PathBuf) {
    let dir = std::env::temp_dir().join(format!("knowlu-class-search-cmd-{}-{tag}-captures", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();
    let feed = "BEGIN:VCALENDAR\nVERSION:2.0\nBEGIN:VEVENT\nUID:BB-ASSIGN-2\nSUMMARY:Quiz A\nDTSTART:20260908T235900\nEND:VEVENT\nEND:VCALENDAR\n";
    let courses = json!([
        {"code": "_900001_1", "name": "Invented Studies I", "slug": "xyz-100", "label": "XYZ 100"},
        {"code": "_900002_1", "name": "Invented Studies II", "slug": "abc-200", "label": "ABC 200"},
    ]);
    std::fs::write(dir.join("feed.ics"), feed).unwrap();
    std::fs::write(dir.join("courses.json"), courses.to_string()).unwrap();
    (dir.join("feed.ics"), dir.join("courses.json"))
}

fn preview(vault: &Path, tag: &str, bundle: &Path) -> Output {
    let (feed, courses) = preview_files(tag);
    Command::new(binary())
        .args(["onboarding", "--vault"])
        .arg(vault)
        .args(["--json", "--now", NOW, "--preview", "--feed"])
        .arg(feed)
        .arg("--courses")
        .arg(courses)
        .arg("--class-search")
        .arg(bundle)
        .args(["--school", "ua"])
        .output()
        .unwrap()
}

/// The preview fills `class_times` (the found sections that name a course in the list) and, in
/// `class_search`, one count per state; the course's bucket carries its class time. It writes
/// nothing to the vault. Red now: the stub ignores the file.
#[test]
fn preview_fills_class_times_and_the_state_counts_and_writes_nothing() {
    let v = scratch("preview");
    let before = snapshot(&v);
    let bundle = put_beside(&v, &bundle_of("202640", all_states()));
    let out = preview(&v, "ok", &bundle);
    assert_eq!(out.status.code(), Some(0), "{out:?}");
    let j: Value = serde_json::from_slice(&out.stdout).unwrap();
    assert_eq!(j["class_times"], json!(1), "{j}");
    for state in STATES {
        assert_eq!(j["class_search"][state], json!(1), "{state}: {j}");
    }
    assert_eq!(j["by_course"]["_900001_1"]["class_times"], json!(1), "{j}");
    assert_eq!(j["by_course"]["_900002_1"]["class_times"], json!(0), "a no-time section is no class time: {j}");
    assert!(!j.to_string().contains("90001"), "no CRN in the preview: {j}");
    assert_eq!(snapshot(&v), before, "the preview wrote something");
    let _ = std::fs::remove_dir_all(&v);
}

/// A missing file and a file that is not a bundle are each `null` for `class_times` and for
/// `class_search`, a named line in `errors`, exit 0, and nothing written.
#[test]
fn preview_with_an_unreadable_bundle_is_null_and_writes_nothing() {
    let v = scratch("preview-bad");
    let before = snapshot(&v);
    let not_a_bundle = put_beside(&v, "this is not a bundle");
    for (name, file) in [("not-a-bundle", not_a_bundle), ("missing", v.with_extension("nothing-here.json"))] {
        let out = preview(&v, name, &file);
        assert_eq!(out.status.code(), Some(0), "{name}: {out:?}");
        let j: Value = serde_json::from_slice(&out.stdout).unwrap();
        assert_eq!(j["class_times"], Value::Null, "{name}: {j}");
        assert_eq!(j["class_search"], Value::Null, "{name}: {j}");
        let errors = j["errors"].as_array().unwrap();
        assert!(errors.iter().any(|e| e.as_str().is_some_and(|s| s.to_lowercase().contains("class"))), "{name}: a named line: {j}");
    }
    assert_eq!(snapshot(&v), before, "the preview wrote something");
    let _ = std::fs::remove_dir_all(&v);
}
