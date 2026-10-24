//! Wave B plan F7a: `knowlu-engine onboarding --vault <v> --json --preview ...`, run as the binary on
//! a scratch vault with inline captures. Read-only, ungated, always exit 0. Every name is invented;
//! no fixture file is read.

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};
use std::process::{Command, Output};

use serde_json::{json, Value};

const GB: &str = "_blackboard.platform.gradebook2.GradableItem-";
/// Pinned clock (vault zone): the command takes `--now` like `surface` does.
const NOW: &str = "2026-09-01T12:00";

fn binary() -> PathBuf {
    PathBuf::from(env!("CARGO_BIN_EXE_knowlu-engine"))
}

fn scratch(name: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!("knowlu-onboarding-cmd-{}-{name}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    for sub in ["config", "tasks", "archive", "grades", "state/journal"] {
        std::fs::create_dir_all(dir.join(sub)).unwrap();
    }
    std::fs::write(dir.join("config/ingest.yaml"), "timezone: America/Chicago\n").unwrap();
    std::fs::write(dir.join("tasks/lab-1.md"), "---\nid: task_00000000a1\ntitle: Lab 1\n---\n\nBody.\n").unwrap();
    std::fs::write(
        dir.join("state/journal/2026-08-30.jsonl"),
        "{\"actor\": \"student\", \"path\": \"tasks/lab-1.md\"}\n",
    )
    .unwrap();
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
                let rel = path.strip_prefix(root).unwrap().to_string_lossy().replace('\\', "/");
                out.insert(rel, std::fs::read(&path).unwrap());
            }
        }
    }
    let mut out = BTreeMap::new();
    walk(vault, vault, &mut out);
    out
}

fn vevent(uid: &str, title: &str, dtstart: &str) -> String {
    format!("BEGIN:VEVENT\nUID:{uid}\nSUMMARY:{title}\nDESCRIPTION:from the feed\nDTSTART:{dtstart}\nEND:VEVENT\n")
}

fn feed_text() -> String {
    let events = [
        vevent("BB-ASSIGN-1", "Past thing", "20260820T235900"),
        vevent(&format!("{GB}_71_1"), "Quiz A", "20260905T235900"),
        vevent(&format!("{GB}_73_1"), "Lab C", "20260907T235900"),
        vevent("BB-ASSIGN-2", "Quiz A", "20260908T235900"),
    ];
    format!("BEGIN:VCALENDAR\nVERSION:2.0\n{}END:VCALENDAR\n", events.concat())
}

fn courses_json() -> Value {
    json!([
        {"code": "_900001_1", "name": "Synthetic Studies I", "slug": "syn-110", "label": "SYN 110"},
        {"code": "_900002_1", "name": "Synthetic Studies II", "slug": "syn-120", "label": "SYN 120"},
    ])
}

/// Course one: `_71_1` graded 8 of 10 (done), `_73_1` no row (live), and an Overall column.
fn bundle_json() -> Value {
    json!({
        "schema": 1, "lms": "blackboard", "host": "lms.example.test",
        "fetched_at": "2026-08-30T12:00:00Z", "user_id": "_900_1",
        "courses": [{
            "membership": {"courseId": "_900001_1"},
            "columns": [
                {"id": "_71_1", "name": "Quiz A", "externalGrade": false, "score": {"possible": 10.0}},
                {"id": "_73_1", "name": "Lab C", "externalGrade": false, "score": {"possible": 10.0}},
                {"id": "_79_1", "name": "Overall Grade", "externalGrade": true},
            ],
            "grades": [
                {"columnId": "_71_1", "status": "Graded", "exempt": false,
                 "displayGrade": {"scaleType": "Score", "score": 8.0, "possible": 10.0}},
                {"columnId": "_79_1", "status": "Graded", "exempt": false,
                 "displayGrade": {"scaleType": "Score", "score": 8.0, "possible": 10.0}},
            ],
            "categories": null,
        }],
    })
}

struct Captures {
    feed: PathBuf,
    courses: PathBuf,
    grades: PathBuf,
}

/// The captures live beside the vault, never inside it, so a snapshot of the vault sees only vault bytes.
fn captures(tag: &str) -> Captures {
    let dir = std::env::temp_dir().join(format!("knowlu-onboarding-cmd-{}-{tag}-captures", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();
    let c = Captures { feed: dir.join("feed.ics"), courses: dir.join("courses.json"), grades: dir.join("grades.json") };
    std::fs::write(&c.feed, feed_text()).unwrap();
    std::fs::write(&c.courses, courses_json().to_string()).unwrap();
    std::fs::write(&c.grades, bundle_json().to_string()).unwrap();
    c
}

fn run(vault: &Path, extra: &[&std::ffi::OsStr]) -> Output {
    Command::new(binary())
        .args(["onboarding", "--vault"])
        .arg(vault)
        .args(["--json", "--now", NOW])
        .args(extra)
        .output()
        .unwrap()
}

fn preview(vault: &Path, c: &Captures, with_grades: bool) -> Output {
    let mut args: Vec<&std::ffi::OsStr> = vec![
        "--preview".as_ref(),
        "--feed".as_ref(),
        c.feed.as_os_str(),
        "--courses".as_ref(),
        c.courses.as_os_str(),
    ];
    if with_grades {
        args.push("--grades".as_ref());
        args.push(c.grades.as_os_str());
    }
    run(vault, &args)
}

fn json_of(out: &Output) -> Value {
    assert_eq!(out.status.code(), Some(0), "{out:?}");
    serde_json::from_slice(&out.stdout).unwrap_or_else(|e| panic!("stdout is JSON ({e}): {out:?}"))
}

#[test]
fn preview_counts_the_inline_captures() {
    let v = scratch("counts");
    let c = captures("counts");
    let j = json_of(&preview(&v, &c, true));
    assert_eq!(j["classes"], json!(2), "{j}");
    assert_eq!(j["assignments_upcoming"], json!(3), "past item left out: {j}");
    assert_eq!(j["turned_in"], json!(1), "by gradable id only; the same title on BB-ASSIGN-2 is live: {j}");
    assert_eq!(j["grades_courses"], json!(1), "{j}");
    assert_eq!(j["errors"], json!([]), "{j}");
    let _ = std::fs::remove_dir_all(&v);
}

#[test]
fn preview_writes_nothing_to_the_vault_or_its_journal() {
    let v = scratch("writes");
    let c = captures("writes");
    let before = snapshot(&v);
    for with_grades in [true, false] {
        json_of(&preview(&v, &c, with_grades));
    }
    assert_eq!(snapshot(&v), before, "the vault's bytes, journal included, must not change");
    let _ = std::fs::remove_dir_all(&v);
}

#[test]
fn preview_takes_no_cloud_config_and_runs_without_one() {
    // No `config/cloud.yaml`, no session, no entitlement: the command still answers (never gated).
    let v = scratch("ungated");
    assert!(!v.join("config/cloud.yaml").exists());
    let c = captures("ungated");
    let out = preview(&v, &c, true);
    json_of(&out);
    assert!(!String::from_utf8_lossy(&out.stdout).to_lowercase().contains("entitlement"), "{out:?}");
    let _ = std::fs::remove_dir_all(&v);
}

#[test]
fn the_same_input_gives_the_same_bytes() {
    let v = scratch("twice");
    let c = captures("twice");
    let a = preview(&v, &c, true);
    let b = preview(&v, &c, true);
    assert_eq!(a.status.code(), Some(0), "{a:?}");
    assert!(!a.stdout.is_empty(), "{a:?}");
    assert_eq!(a.stdout, b.stdout);
    let _ = std::fs::remove_dir_all(&v);
}

#[test]
fn a_missing_feed_is_a_null_field_a_named_error_and_exit_zero() {
    let v = scratch("nofeed");
    let mut c = captures("nofeed");
    c.feed = c.feed.with_file_name("does-not-exist.ics");
    let j = json_of(&preview(&v, &c, true));
    assert_eq!(j["assignments_upcoming"], Value::Null, "{j}");
    assert_eq!(j["turned_in"], Value::Null, "{j}");
    assert_eq!(j["classes"], json!(2), "the other inputs still count: {j}");
    let errors = j["errors"].as_array().unwrap_or_else(|| panic!("errors is an array: {j}"));
    assert!(errors.iter().any(|e| e.as_str().is_some_and(|s| s.contains("feed"))), "{j}");
    let _ = std::fs::remove_dir_all(&v);
}

#[test]
fn an_unreadable_courses_file_is_a_null_field_a_named_error_and_exit_zero() {
    let v = scratch("badcourses");
    let c = captures("badcourses");
    std::fs::write(&c.courses, "{ this is not json").unwrap();
    let j = json_of(&preview(&v, &c, true));
    assert_eq!(j["classes"], Value::Null, "{j}");
    let errors = j["errors"].as_array().unwrap_or_else(|| panic!("errors is an array: {j}"));
    assert!(errors.iter().any(|e| e.as_str().is_some_and(|s| s.contains("courses"))), "{j}");
    let _ = std::fs::remove_dir_all(&v);
}

#[test]
fn a_missing_grades_flag_gives_null_turned_in_and_null_grades_courses() {
    let v = scratch("nogrades");
    let c = captures("nogrades");
    let j = json_of(&preview(&v, &c, false));
    assert_eq!(j["turned_in"], Value::Null, "{j}");
    assert_eq!(j["grades_courses"], Value::Null, "{j}");
    assert_eq!(j["assignments_upcoming"], json!(3), "{j}");
    assert_eq!(j["errors"], json!([]), "an absent optional input is not an error: {j}");
    let _ = std::fs::remove_dir_all(&v);
}

#[test]
fn no_mode_prints_a_named_usage_line_and_exits_zero() {
    let v = scratch("nomode");
    let before = snapshot(&v);
    let out = run(&v, &[]);
    assert_eq!(out.status.code(), Some(0), "{out:?}");
    let text = format!("{}{}", String::from_utf8_lossy(&out.stdout), String::from_utf8_lossy(&out.stderr));
    assert!(text.contains("onboarding: exactly one mode"), "usage line names the command: {text}");
    assert_eq!(snapshot(&v), before);
    let _ = std::fs::remove_dir_all(&v);
}

#[test]
fn two_modes_print_the_same_usage_line_and_exit_zero() {
    let v = scratch("twomodes");
    let c = captures("twomodes");
    let out = run(
        &v,
        &["--preview".as_ref(), "--tidied".as_ref(), "--feed".as_ref(), c.feed.as_os_str(), "--courses".as_ref(), c.courses.as_os_str()],
    );
    assert_eq!(out.status.code(), Some(0), "{out:?}");
    let text = format!("{}{}", String::from_utf8_lossy(&out.stdout), String::from_utf8_lossy(&out.stderr));
    assert!(text.contains("onboarding: exactly one mode"), "{text}");
    let _ = std::fs::remove_dir_all(&v);
}

fn banner_file(tag: &str) -> PathBuf {
    let meets = json!([{"meetingType": "CLAS", "monday": true, "wednesday": true, "beginTime": "0900",
                        "endTime": "0950", "startDate": "08/20/2026", "endDate": "12/05/2026"}]);
    let row = |crn: &str, subject: &str, number: &str| {
        json!({"term": "202640", "courseReferenceNumber": crn, "subject": subject, "courseNumber": number,
               "scheduleDescription": "Lecture", "statusIndicator": "IR", "meetingTimes": meets.clone()})
    };
    let dir = std::env::temp_dir().join(format!("knowlu-onboarding-cmd-{}-{tag}-registrar", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();
    let path = dir.join("registrar.json");
    std::fs::write(&path, json!({"data": {"registrations": [row("90001", "SYN", "110"), row("90002", "ZZT", "999")]}}).to_string()).unwrap();
    path
}

#[test]
fn registrar_and_school_give_class_times_and_write_nothing() {
    let v = scratch("registrar");
    let c = captures("registrar");
    let r = banner_file("registrar");
    let before = snapshot(&v);
    let out = run(
        &v,
        &["--preview".as_ref(), "--feed".as_ref(), c.feed.as_os_str(), "--courses".as_ref(), c.courses.as_os_str(),
          "--registrar".as_ref(), r.as_os_str(), "--school".as_ref(), "ua".as_ref()],
    );
    let j = json_of(&out);
    assert_eq!(j["class_times"], json!(1), "{j}");
    assert_eq!(j["errors"], json!([]), "{j}");
    assert_eq!(snapshot(&v), before);
    let _ = std::fs::remove_dir_all(&v);
}

#[test]
fn without_registrar_class_times_is_null_and_no_error() {
    let v = scratch("noregistrar");
    let c = captures("noregistrar");
    let j = json_of(&preview(&v, &c, true));
    assert_eq!(j["class_times"], Value::Null, "{j}");
    assert_eq!(j["errors"], json!([]), "{j}");
    let _ = std::fs::remove_dir_all(&v);
}

#[test]
fn tidied_alone_prints_a_named_not_built_line_and_exits_zero() {
    let v = scratch("tidied");
    let before = snapshot(&v);
    let out = run(&v, &["--tidied".as_ref()]);
    assert_eq!(out.status.code(), Some(0), "{out:?}");
    let text = format!("{}{}", String::from_utf8_lossy(&out.stdout), String::from_utf8_lossy(&out.stderr));
    assert!(text.contains("onboarding: --tidied is not built yet"), "{text}");
    assert_eq!(snapshot(&v), before);
    let _ = std::fs::remove_dir_all(&v);
}
