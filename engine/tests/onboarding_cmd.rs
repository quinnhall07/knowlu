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

// ---- F7c: `--tidied`, the reveal's counts read from the vault ----

fn tidied(vault: &Path) -> Output {
    run(vault, &["--tidied".as_ref()])
}

fn task_note(id: &str, title: &str, created_by: &str) -> String {
    format!(
        "---\nid: {id}\ntitle: {title}\nstatus: active\neffort_hours: 1.5\neffort_source: inferred\n\
         importance: 3\ncreated_by: {created_by}\nsource_uid: \"{created_by}:{id}\"\n---\n\nBody.\n"
    )
}

fn archive_note(id: &str, title: &str, reason: &str) -> String {
    format!(
        "---\nid: {id}\ntitle: {title}\nstatus: archived\narchived_reason: {reason}\ncreated_by: blackboard\n---\n\nBody.\n"
    )
}

/// One journal line, as the engine writes it: sorted keys, Python `json.dumps` separators.
fn journal_line(seq: u32, actor: &str, path: &str, field: &str) -> String {
    record_line(seq, "set", actor, path, &format!("\"{field}\""))
}

/// The id a note's journal records carry: the note's own `id:`, as the engine writes it.
fn note_id_of(path: &str) -> &'static str {
    match path {
        "tasks/quiz-a.md" => "task_00000000a1",
        "tasks/quiz-b.md" => "task_00000000a2",
        "tasks/essay.md" => "task_00000000a3",
        "tasks/reading.md" => "task_00000000a4",
        "archive/old-1.md" => "task_00000000b1",
        "tasks/mail.md" => "task_00000000a7",
        "tasks/typed.md" => "task_00000000a8",
        "tasks/self.md" => "task_00000000a9",
        _ => "task_00000000zz",
    }
}

/// `field` is a JSON literal (`"effort_hours"` or `null`, a `create` having none).
fn record_line(seq: u32, op: &str, actor: &str, path: &str, field: &str) -> String {
    record_with_id(seq, op, actor, path, note_id_of(path), field)
}

/// A record with the note id given outright, for a record about a note other than the one now at `path`.
fn record_with_id(seq: u32, op: &str, actor: &str, path: &str, id: &str, field: &str) -> String {
    format!(
        "{{\"actor\": \"{actor}\", \"device\": \"testbox\", \"evidence\": null, \"field\": {field}, \
         \"id\": \"{id}\", \"new\": 2.0, \"old\": 1.5, \"op\": \"{op}\", \"path\": \"{path}\", \
         \"run_id\": \"run-{seq}\", \"seq\": {seq}, \"ts\": \"2026-08-30T12:00:0{seq}.000Z\", \"via\": \"cli\"}}\n"
    )
}

/// A vault built inline. Live tasks: two from `blackboard`, one `gmail`, one `zybooks`. Archive:
/// two `imported-past`, one `imported-done`, one archived for another reason. The journal: an agent
/// set the effort of `quiz-a` and `reading`; the student set `quiz-b`'s; an agent set only the
/// importance of `essay`; an agent set the effort of an archived note (not live, so not counted).
fn tidy_vault(tag: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!("knowlu-onboarding-cmd-{}-{tag}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    for sub in ["config", "tasks", "archive", "state/journal"] {
        std::fs::create_dir_all(dir.join(sub)).unwrap();
    }
    let put = |rel: &str, text: String| std::fs::write(dir.join(rel), text).unwrap();
    put("config/ingest.yaml", "timezone: America/Chicago\n".to_string());
    put("tasks/quiz-a.md", task_note("task_00000000a1", "Quiz A", "blackboard"));
    put("tasks/quiz-b.md", task_note("task_00000000a2", "Quiz B", "blackboard"));
    put("tasks/essay.md", task_note("task_00000000a3", "Essay", "gmail"));
    put("tasks/reading.md", task_note("task_00000000a4", "Reading", "zybooks"));
    put("archive/old-1.md", archive_note("task_00000000b1", "Old 1", "imported-past"));
    put("archive/old-2.md", archive_note("task_00000000b2", "Old 2", "imported-past"));
    put("archive/done-1.md", archive_note("task_00000000b3", "Done 1", "imported-done"));
    put("archive/other-1.md", archive_note("task_00000000b4", "Other 1", "completed"));
    let journal = [
        journal_line(1, "agent:knowlu.enrich", "tasks/quiz-a.md", "effort_hours"),
        journal_line(2, "student", "tasks/quiz-b.md", "effort_hours"),
        journal_line(3, "agent:knowlu.enrich", "tasks/essay.md", "importance"),
        journal_line(4, "agent:knowlu.enrich", "tasks/reading.md", "effort_hours"),
        journal_line(5, "agent:knowlu.enrich", "archive/old-1.md", "effort_hours"),
    ];
    put("state/journal/2026-08-30.jsonl", journal.concat());
    dir
}

#[test]
fn tidied_counts_archive_notes_by_archived_reason() {
    let v = tidy_vault("tidy-archive");
    let j = json_of(&tidied(&v));
    assert_eq!(j["imported_past"], json!(2), "{j}");
    assert_eq!(j["imported_done"], json!(1), "another archived_reason is neither: {j}");
    assert_eq!(j["errors"], json!([]), "{j}");
    let _ = std::fs::remove_dir_all(&v);
}

#[test]
fn tidied_counts_live_tasks_by_created_by() {
    let v = tidy_vault("tidy-live");
    let j = json_of(&tidied(&v));
    assert_eq!(j["live_by_source"], json!({"blackboard": 2, "gmail": 1, "zybooks": 1}), "archive notes are not live: {j}");
    let _ = std::fs::remove_dir_all(&v);
}

#[test]
fn tidied_effort_estimated_counts_live_tasks_an_agent_set_the_effort_of() {
    let v = tidy_vault("tidy-effort");
    let j = json_of(&tidied(&v));
    assert_eq!(
        j["effort_estimated"],
        json!(2),
        "quiz-a and reading; not the student's set, not another field, not an archived note: {j}"
    );
    let _ = std::fs::remove_dir_all(&v);
}

#[test]
fn tidied_writes_nothing_to_the_vault_or_its_journal() {
    let v = tidy_vault("tidy-writes");
    let before = snapshot(&v);
    json_of(&tidied(&v));
    assert_eq!(snapshot(&v), before, "the vault's bytes, journal included, must not change");
    let _ = std::fs::remove_dir_all(&v);
}

#[test]
fn tidied_runs_without_a_cloud_config_and_is_not_gated() {
    let v = tidy_vault("tidy-ungated");
    assert!(!v.join("config/cloud.yaml").exists());
    let out = tidied(&v);
    json_of(&out);
    assert!(!String::from_utf8_lossy(&out.stdout).to_lowercase().contains("entitlement"), "{out:?}");
    let _ = std::fs::remove_dir_all(&v);
}

#[test]
fn tidied_same_input_gives_the_same_bytes_with_sorted_keys() {
    let v = tidy_vault("tidy-twice");
    let a = tidied(&v);
    let b = tidied(&v);
    assert_eq!(a.status.code(), Some(0), "{a:?}");
    assert_eq!(a.stdout, b.stdout);
    let text = String::from_utf8_lossy(&a.stdout).to_string();
    let at = |k: &str| text.find(&format!("\"{k}\"")).unwrap_or_else(|| panic!("no {k} in {text}"));
    assert!(at("effort_estimated") < at("errors") && at("errors") < at("imported_done"), "{text}");
    assert!(at("imported_done") < at("imported_past") && at("imported_past") < at("live_by_source"), "{text}");
    let _ = std::fs::remove_dir_all(&v);
}

#[test]
fn tidied_with_no_archive_folder_nulls_only_the_archive_counts() {
    let v = tidy_vault("tidy-noarchive");
    std::fs::remove_dir_all(v.join("archive")).unwrap();
    let j = json_of(&tidied(&v));
    assert_eq!(j["imported_past"], Value::Null, "{j}");
    assert_eq!(j["imported_done"], Value::Null, "{j}");
    assert_eq!(j["effort_estimated"], json!(2), "the other counts still stand: {j}");
    let errors = j["errors"].as_array().unwrap_or_else(|| panic!("errors is an array: {j}"));
    assert!(errors.iter().any(|e| e.as_str().is_some_and(|s| s.contains("archive"))), "{j}");
    let _ = std::fs::remove_dir_all(&v);
}

#[test]
fn tidied_with_no_journal_nulls_effort_estimated_and_names_it() {
    let v = tidy_vault("tidy-nojournal");
    std::fs::remove_dir_all(v.join("state")).unwrap();
    let j = json_of(&tidied(&v));
    assert_eq!(j["effort_estimated"], Value::Null, "an unread journal is not zero: {j}");
    assert_eq!(j["imported_past"], json!(2), "{j}");
    assert_eq!(j["live_by_source"], json!({"blackboard": 2, "gmail": 1, "zybooks": 1}), "{j}");
    let errors = j["errors"].as_array().unwrap_or_else(|| panic!("errors is an array: {j}"));
    assert!(errors.iter().any(|e| e.as_str().is_some_and(|s| s.contains("journal"))), "{j}");
    let _ = std::fs::remove_dir_all(&v);
}

#[test]
fn tidied_on_a_vault_that_does_not_exist_is_all_null_a_named_error_and_exit_zero() {
    let v = std::env::temp_dir().join(format!("knowlu-onboarding-cmd-{}-tidy-missing", std::process::id()));
    let _ = std::fs::remove_dir_all(&v);
    let j = json_of(&tidied(&v));
    for key in ["imported_past", "imported_done", "effort_estimated", "live_by_source"] {
        assert_eq!(j[key], Value::Null, "{key}: {j}");
    }
    assert!(j["errors"].as_array().is_some_and(|e| !e.is_empty()), "{j}");
    assert!(!v.exists(), "the command must not create the vault");
}

#[test]
fn tidied_puts_a_live_task_with_no_created_by_in_the_unknown_bucket() {
    let v = tidy_vault("tidy-unknown");
    // One note with no created_by at all, one with an empty value: both are "unknown".
    std::fs::write(v.join("tasks/loose.md"), "---\nid: task_00000000a5\ntitle: Loose\nstatus: active\n---\n\nBody.\n").unwrap();
    std::fs::write(v.join("tasks/blank.md"), "---\nid: task_00000000a6\ntitle: Blank\nstatus: active\ncreated_by: \"\"\n---\n\nBody.\n").unwrap();
    let j = json_of(&tidied(&v));
    assert_eq!(
        j["live_by_source"],
        json!({"blackboard": 2, "gmail": 1, "unknown": 2, "zybooks": 1}),
        "a live task with no created_by is counted, not dropped: {j}"
    );
    let _ = std::fs::remove_dir_all(&v);
}

#[test]
fn tidied_counts_a_task_whose_agent_effort_the_student_later_overrode() {
    let v = tidy_vault("tidy-override");
    // The student also set quiz-a's effort afterwards; the agent estimate still happened.
    let extra = journal_line(6, "student", "tasks/quiz-a.md", "effort_hours");
    let path = v.join("state/journal/2026-08-30.jsonl");
    let mut text = std::fs::read_to_string(&path).unwrap();
    text.push_str(&extra);
    std::fs::write(&path, text).unwrap();
    let j = json_of(&tidied(&v));
    assert_eq!(j["effort_estimated"], json!(2), "{j}");
    let _ = std::fs::remove_dir_all(&v);
}

#[test]
fn preview_by_course_buckets_the_inline_captures_by_course() {
    let v = scratch("bycourse");
    let c = captures("bycourse");
    let j = json_of(&preview(&v, &c, true));
    // The feed carries no course text: _71_1 and _73_1 reach the course with LMS key _900001_1
    // through the grades bundle (a uid pin); BB-ASSIGN-2 reaches no course, so it is under
    // `unmatched`, which keeps the buckets summing to the totals.
    assert_eq!(j["by_course"]["_900001_1"]["assignments_upcoming"], json!(2), "{j}");
    assert_eq!(j["by_course"]["_900001_1"]["turned_in"], json!(1), "{j}");
    assert_eq!(j["by_course"]["_900001_1"]["class_times"], Value::Null, "{j}");
    assert_eq!(j["by_course"]["_900002_1"]["assignments_upcoming"], json!(0), "{j}");
    assert_eq!(j["by_course"]["unmatched"]["assignments_upcoming"], json!(1), "{j}");
    assert_eq!(j["assignments_upcoming"], json!(3), "the totals stay as they were: {j}");
    let _ = std::fs::remove_dir_all(&v);
}

fn append_journal(v: &Path, lines: &[String]) {
    let path = v.join("state/journal/2026-08-30.jsonl");
    let mut text = std::fs::read_to_string(&path).unwrap();
    text.push_str(&lines.concat());
    std::fs::write(&path, text).unwrap();
}

fn put_task(v: &Path, name: &str, id: Option<&str>, source: &str) {
    let id_line = id.map(|i| format!("id: {i}\n")).unwrap_or_default();
    let text = format!("---\n{id_line}title: {name}\nstatus: active\neffort_hours: 1.0\neffort_source: {source}\ncreated_by: gmail\n---\n\nBody.\n");
    std::fs::write(v.join(format!("tasks/{name}.md")), text).unwrap();
}

#[test]
fn tidied_counts_an_agent_create_of_an_inferred_note_and_nothing_else_that_creates() {
    let v = tidy_vault("tidy-gmail");
    put_task(&v, "mail", Some("task_00000000a7"), "inferred"); // agent create, inferred: counted
    put_task(&v, "typed", Some("task_00000000a8"), "user"); // agent create, effort typed: not counted
    put_task(&v, "self", Some("task_00000000a9"), "inferred"); // the student's own create: not counted
    append_journal(
        &v,
        &[
            record_line(6, "create", "agent:knowlu.gmail", "tasks/mail.md", "null"),
            record_line(7, "create", "agent:knowlu.gmail", "tasks/typed.md", "null"),
            record_line(8, "create", "student", "tasks/self.md", "null"),
        ],
    );
    let j = json_of(&tidied(&v));
    assert_eq!(j["effort_estimated"], json!(3), "quiz-a, reading and the Gmail task: {j}");
    let _ = std::fs::remove_dir_all(&v);
}

#[test]
fn tidied_joins_a_record_to_a_note_by_path_only_when_the_note_has_no_id() {
    let v = tidy_vault("tidy-path");
    // A note re-created at a path an agent once set (a record carrying the old note's id): not counted.
    put_task(&v, "reused", Some("task_00000000a5"), "user");
    // A note with no id at all: its path is the only join, so the agent's set counts.
    put_task(&v, "noid", None, "user");
    append_journal(
        &v,
        &[record_with_id(6, "set", "agent:knowlu.enrich", "tasks/reused.md", "task_00000000d0", "\"effort_hours\""), journal_line(7, "agent:knowlu.enrich", "tasks/noid.md", "effort_hours")],
    );
    let j = json_of(&tidied(&v));
    assert_eq!(j["effort_estimated"], json!(3), "quiz-a, reading and noid, not reused: {j}");
    let _ = std::fs::remove_dir_all(&v);
}

#[test]
fn tidied_reads_notes_written_with_crlf() {
    let v = tidy_vault("tidy-crlf");
    let crlf = |t: String| t.replace('\n', "\r\n");
    std::fs::write(v.join("tasks/crlf.md"), crlf(task_note("task_00000000c1", "Crlf", "blackboard"))).unwrap();
    std::fs::write(v.join("archive/crlf-old.md"), crlf(archive_note("task_00000000c2", "Crlf old", "imported-past"))).unwrap();
    let j = json_of(&tidied(&v));
    assert_eq!(j["live_by_source"]["blackboard"], json!(3), "{j}");
    assert_eq!(j["imported_past"], json!(3), "{j}");
    assert_eq!(j["errors"], json!([]), "{j}");
    let _ = std::fs::remove_dir_all(&v);
}
