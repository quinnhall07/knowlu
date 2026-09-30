//! M1 Task 3: `knowlu-engine grades` (spec 2026-09-29-grades-design.md, "6. The engine"), run as
//! the binary against a temp copy of `vault-s1` with two synthetic course notes whose `code:` match
//! the synthetic bundles under `fixtures/grades/`. Every course, column and score is invented.

use std::collections::{BTreeMap, BTreeSet};
use std::path::{Path, PathBuf};
use std::process::{Command, Output};

use knowlu_engine::childproc::NoConsole;
use knowlu_engine::grades::note_path;
use knowlu_engine::ids::{derived_id, read_meta};
use knowlu_engine::journal::{self, Journal};
use knowlu_engine::write::{self, WriteContext, WriteOpts};

const ACTOR: &str = "agent:knowlu.grades";

fn fixtures() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("tests").join("fixtures")
}

fn bundle_text(name: &str) -> String {
    std::fs::read_to_string(fixtures().join("grades").join(name)).expect("bundle fixture")
}

fn binary() -> PathBuf {
    PathBuf::from(env!("CARGO_BIN_EXE_knowlu-engine"))
}

fn copy_tree(from: &Path, to: &Path) {
    std::fs::create_dir_all(to).expect("mkdir");
    for entry in std::fs::read_dir(from).expect("read_dir").flatten() {
        let (p, t) = (entry.path(), to.join(entry.file_name()));
        if p.is_dir() { copy_tree(&p, &t) } else { std::fs::copy(&p, &t).expect("copy"); }
    }
}

/// A temp copy of `vault-s1` with the two course notes the bundles' matched courses name: SYN 110
/// by `membership.courseId`, SYN 330 by `course.courseId`. SYN 220 carries `"error"` in both.
fn vault(name: &str) -> PathBuf {
    let root = std::env::temp_dir().join(format!("knowlu-grades-{}-{name}", std::process::id()));
    let _ = std::fs::remove_dir_all(&root);
    let v = root.join("vault");
    copy_tree(&fixtures().join("vault-s1"), &v);
    std::fs::create_dir_all(v.join("courses")).unwrap();
    std::fs::write(v.join("courses").join("syn-110.md"), "---\ntitle: Synthetic Studies I\ncode: _900110_1\n---\n").unwrap();
    std::fs::write(v.join("courses").join("syn-330.md"), "---\ntitle: Synthetic Studies III\ncode: SYN-330-002-FA2026\n---\n").unwrap();
    v
}

/// The bundle goes beside the vault, never inside it, as the app keeps it in its own data.
fn run_text(v: &Path, text: &str) -> Output {
    let input = v.with_extension("bundle.json");
    std::fs::write(&input, text).unwrap();
    let out = run_args(v, &[&input]);
    let _ = std::fs::remove_file(&input);
    out
}

fn run(v: &Path, fixture: &str) -> Output {
    run_text(v, &bundle_text(fixture))
}

fn run_args(v: &Path, input: &[&Path]) -> Output {
    let mut cmd = Command::new(binary());
    cmd.no_console().args(["grades", "--vault"]).arg(v).args(["--via", "local-runner"]);
    for i in input {
        cmd.arg("--input").arg(i);
    }
    cmd.output().expect("run knowlu-engine")
}

fn stdout(out: &Output) -> String {
    String::from_utf8_lossy(&out.stdout).to_string()
}

/// Every file under `dir`, relative and POSIX, with its bytes.
fn files(dir: &Path) -> BTreeMap<String, Vec<u8>> {
    fn walk(dir: &Path, root: &Path, out: &mut BTreeMap<String, Vec<u8>>) {
        for entry in std::fs::read_dir(dir).into_iter().flatten().flatten() {
            let p = entry.path();
            if p.is_dir() {
                walk(&p, root, out);
            } else {
                let rel = p.strip_prefix(root).unwrap().to_string_lossy().replace('\\', "/");
                out.insert(rel, std::fs::read(&p).unwrap());
            }
        }
    }
    let mut out = BTreeMap::new();
    walk(dir, dir, &mut out);
    out
}

/// The vault's bytes, less the runner log (every run appends its line there by design).
fn vault_bytes(v: &Path) -> BTreeMap<String, Vec<u8>> {
    let mut all = files(v);
    all.remove("state/runner-log.md");
    all
}

fn records(v: &Path) -> Vec<serde_json::Value> {
    files(&v.join("state").join("journal"))
        .values()
        .flat_map(|b| String::from_utf8_lossy(b).lines().map(str::to_string).collect::<Vec<_>>())
        .filter(|l| !l.trim().is_empty())
        .map(|l| serde_json::from_str(&l).expect("a journal line is JSON"))
        .collect()
}

fn grade_notes(v: &Path) -> BTreeSet<String> {
    files(&v.join("grades")).keys().map(|k| format!("grades/{k}")).collect()
}

fn field(v: &Path, rel: &str, key: &str) -> Option<serde_yaml_ng::Value> {
    read_meta(&v.join(rel)).and_then(|m| knowlu_engine::yaml::get(&m, key).cloned())
}

fn text_field(v: &Path, rel: &str, key: &str) -> Option<String> {
    field(v, rel, key).and_then(|x| knowlu_engine::yaml::text(&x))
}

fn cleanup(v: &Path) {
    let _ = std::fs::remove_dir_all(v.parent().unwrap());
}

/// Every column the bundle carries for a course, as `(course slug, column id)`.
const BASIC_COLUMNS: [(&str, &str); 11] = [
    ("syn-110", "_8110001_1"), ("syn-110", "_8110002_1"), ("syn-110", "_8110003_1"), ("syn-110", "_8110004_1"),
    ("syn-110", "_8110005_1"), ("syn-110", "_8110006_1"), ("syn-110", "_8110007_1"),
    ("syn-330", "_8330001_1"), ("syn-330", "_8330002_1"), ("syn-330", "_8330003_1"), ("syn-330", "_8330004_1"),
];

// ---- first run: one note per column, the deterministic id, a create record each ----

#[test]
fn the_first_run_creates_one_note_per_column_as_the_agent_with_the_derived_id() {
    let v = vault("first");
    let out = run(&v, "bundle-basic.json");
    assert!(out.status.success(), "{out:?}");
    let expected: BTreeSet<String> = BASIC_COLUMNS.iter().map(|(s, c)| note_path(s, c)).collect();
    assert_eq!(grade_notes(&v), expected);
    let recs = records(&v);
    assert_eq!(recs.len(), expected.len(), "one journal record per note: {recs:?}");
    for rel in &expected {
        // `write::create` keeps a caller-chosen id only when `ids::is_id` accepts it; a silent
        // re-mint would give a random one here.
        let id = text_field(&v, rel, "id").expect("id");
        assert_eq!(id, derived_id("grade", rel), "{rel}");
        assert_eq!(text_field(&v, rel, "type").as_deref(), Some("grade"), "{rel}");
        assert_eq!(text_field(&v, rel, "source").as_deref(), Some("blackboard"), "{rel}");
        let rec = recs.iter().find(|r| r["path"] == rel.as_str()).expect("a record for the note");
        assert_eq!((rec["op"].as_str(), rec["actor"].as_str(), rec["via"].as_str()), (Some("create"), Some(ACTOR), Some("local-runner")));
        assert_eq!(rec["id"].as_str(), Some(id.as_str()));
    }
    // Homework 1: every field of spec 6
    let hw1 = note_path("syn-110", "_8110001_1");
    assert_eq!(hw1, "grades/syn-110-8110001-1.md");
    let text = String::from_utf8(std::fs::read(v.join(&hw1)).unwrap()).unwrap().replace("\r\n", "\n");
    for line in [
        "title: Homework 1", "course: syn-110", "source_uid: _8110001_1", "kind: item", "possible: 20.0",
        "score: 18.0", "status: graded", "counts: true", "category: Homework", "due: 2026-09-04T23:59",
    ] {
        assert!(text.lines().any(|l| l == line), "{line:?} in\n{text}");
    }
    // Homework 3: no row, so not-submitted and no score; the overall column; the refused categories
    let hw3 = note_path("syn-110", "_8110004_1");
    assert_eq!(text_field(&v, &hw3, "status").as_deref(), Some("not-submitted"));
    assert_eq!(field(&v, &hw3, "score"), None, "score is absent until graded");
    assert_eq!(text_field(&v, &note_path("syn-110", "_8110007_1"), "kind").as_deref(), Some("overall"));
    assert_eq!(field(&v, &note_path("syn-330", "_8330001_1"), "category"), None, "a refused category list");
    assert_eq!(field(&v, &note_path("syn-330", "_8330003_1"), "possible"), None, "no score.possible");
    assert_eq!(field(&v, &note_path("syn-110", "_8110006_1"), "counts"), Some(serde_yaml_ng::Value::Bool(false)));
    cleanup(&v);
}

#[test]
fn a_second_identical_run_writes_no_record_and_changes_no_byte() {
    let v = vault("idempotent");
    assert!(run(&v, "bundle-basic.json").status.success());
    let before = vault_bytes(&v);
    let out = run(&v, "bundle-basic.json");
    assert!(out.status.success(), "{out:?}");
    assert_eq!(vault_bytes(&v), before, "nothing but the runner log may move");
    assert!(stdout(&out).lines().any(|l| l == "grades: 2 courses, 0 changed items"), "{}", stdout(&out));
    cleanup(&v);
}

// ---- the changed bundle: only what changed, and the gone column is removed ----

#[test]
fn the_changed_bundle_writes_only_the_changed_fields_and_removes_the_gone_column() {
    let v = vault("changed");
    assert!(run(&v, "bundle-basic.json").status.success());
    let first = records(&v).len();
    let out = run(&v, "bundle-changed.json");
    assert!(out.status.success(), "{out:?}");
    let recs = records(&v);
    let new: BTreeSet<(String, String)> = recs[first..]
        .iter()
        .inspect(|r| assert_eq!((r["op"].as_str(), r["actor"].as_str()), (Some("set"), Some(ACTOR)), "{r}"))
        .map(|r| (r["path"].as_str().unwrap().to_string(), r["field"].as_str().unwrap().to_string()))
        .collect();
    let (hw3, exam, lab2) = (note_path("syn-110", "_8110004_1"), note_path("syn-110", "_8110003_1"), note_path("syn-330", "_8330002_1"));
    let expected: BTreeSet<(String, String)> = [
        (hw3.clone(), "status"), (hw3.clone(), "score"), (exam.clone(), "title"), (lab2.clone(), "status"),
    ]
    .into_iter()
    .map(|(p, f)| (p, f.to_string()))
    .collect();
    assert_eq!(new, expected);
    assert_eq!(recs.len() - first, 4, "one record per changed field, no more");
    assert_eq!(text_field(&v, &hw3, "status").as_deref(), Some("graded"));
    assert_eq!(field(&v, &hw3, "score").and_then(|x| x.as_f64()), Some(17.0));
    assert_eq!(text_field(&v, &exam, "title").as_deref(), Some("Exam 1"));
    assert_eq!(text_field(&v, &lab2, "status").as_deref(), Some("removed"));
    assert_eq!(field(&v, &lab2, "score").and_then(|x| x.as_f64()), Some(38.0), "a removed column keeps its score");
    assert!(stdout(&out).lines().any(|l| l == "grades: 2 courses, 3 changed items"), "{}", stdout(&out));
    // and a third run of the same bundle is quiet again
    let before = vault_bytes(&v);
    assert!(run(&v, "bundle-changed.json").status.success());
    assert_eq!(vault_bytes(&v), before);
    cleanup(&v);
}

// ---- a refused category list is not "no category" (spec 5, 6) ----

#[test]
fn a_refused_category_list_leaves_known_categories_and_the_run_is_quiet() {
    let v = vault("categories-refused");
    assert!(run(&v, "bundle-basic.json").status.success());
    let hw1 = note_path("syn-110", "_8110001_1");
    assert_eq!(text_field(&v, &hw1, "category").as_deref(), Some("Homework"));
    let before = vault_bytes(&v);
    let first = records(&v).len();
    let mut refused: serde_json::Value = serde_json::from_str(&bundle_text("bundle-basic.json")).unwrap();
    refused["courses"][0]["categories"] = serde_json::Value::Null;
    let out = run_text(&v, &refused.to_string());
    assert!(out.status.success(), "{out:?}");
    assert!(stdout(&out).lines().any(|l| l == "grades: 2 courses, 0 changed items"), "{}", stdout(&out));
    assert_eq!(records(&v).len(), first, "no journal record: {:?}", &records(&v)[first..]);
    assert_eq!(vault_bytes(&v), before, "nothing but the runner log may move");
    assert_eq!(text_field(&v, &hw1, "category").as_deref(), Some("Homework"));
    cleanup(&v);
}

// ---- judge once: a field the student set is never re-set ----

/// Run basic, have the vault's human retitle the exam note (the actor `console_ctx(vault)` gives:
/// the vault's own token, what `Journal::human_set` recognises), then run the changed bundle.
fn student_title_survives(v: &Path, token: &str) {
    assert!(run(v, "bundle-basic.json").status.success());
    let exam = note_path("syn-110", "_8110003_1");
    let human = journal::read_human_actor(v).expect("actor file");
    assert_eq!(human, token);
    let ctx = WriteContext::new(human, "dashboard");
    let mut j = Journal::new(v);
    let lit = write::to_literal(&serde_yaml_ng::Value::String("My midterm".into()));
    write::write_literals(v, &exam, &[("title".into(), lit)], &ctx, &mut j, &WriteOpts::default()).expect("the student's write");
    let id = text_field(v, &exam, "id").unwrap();
    assert_eq!(j.human_set(&id, "title").and_then(|r| r["actor"].as_str().map(str::to_string)).as_deref(), Some(token));
    let before = records(v).len();
    let out = run(v, "bundle-changed.json");
    assert!(out.status.success(), "{out:?}");
    assert_eq!(text_field(v, &exam, "title").as_deref(), Some("My midterm"), "judge once");
    let after = records(v);
    assert!(after[before..].iter().all(|r| r["path"] != exam.as_str()), "no record for the kept field: {:?}", &after[before..]);
    assert_eq!(after.len() - before, 3, "the other three changes still land");
}

#[test]
fn a_field_the_student_set_is_never_reset_on_a_legacy_vault() {
    let v = vault("judge-once-legacy");
    assert!(!v.join(journal::ACTOR_FILE).exists());
    student_title_survives(&v, journal::LEGACY_HUMAN_ACTOR);
    cleanup(&v);
}

#[test]
fn a_field_the_student_set_is_never_reset_on_a_student_vault() {
    let v = vault("judge-once-student");
    journal::create_actor_file(&v, journal::HUMAN_ACTOR).expect("actor file");
    student_title_survives(&v, journal::HUMAN_ACTOR);
    cleanup(&v);
}

// ---- the entitlement gate ----

#[test]
fn past_the_grace_grades_prints_the_named_skip_at_exit_0_and_writes_nothing() {
    // `entitlement_gate.rs`'s pattern: a vault with an account, a canceled cache under a temp
    // LOCALAPPDATA, the built binary.
    let v = vault("gated");
    let root = v.parent().unwrap().to_path_buf();
    std::fs::create_dir_all(v.join("config")).unwrap();
    std::fs::write(
        v.join("config").join("cloud.yaml"),
        "api_base: 'https://example.invalid/functions/v1'\nanon_key: 'anon'\nsession_credential_target: 'knowlu/profile_0a1b2c3d4e/session'\naccount_id: 'acct-1'\n",
    ).unwrap();
    let cache = root.join("knowlu").join("profiles").join("profile_0a1b2c3d4e");
    std::fs::create_dir_all(&cache).unwrap();
    std::fs::write(cache.join("entitlement.json"), r#"{"status":"canceled","current_period_end":null,"plan":"monthly","checked_at":"2026-09-17T06:00:00Z"}"#).unwrap();
    let input = root.join("bundle.json");
    std::fs::write(&input, bundle_text("bundle-basic.json")).unwrap();
    let out = Command::new(binary())
        .no_console()
        .env("LOCALAPPDATA", &root)
        .args(["grades", "--vault"]).arg(&v).arg("--input").arg(&input)
        .output()
        .unwrap();
    assert!(out.status.success(), "{out:?}");
    assert_eq!(stdout(&out).trim_end(), "grades (skipped: no entitlement)");
    assert!(!v.join("grades").exists());
    assert!(!v.join("state").join("journal").exists());
    assert!(!v.join("state").join("grades.json").exists());
    cleanup(&v);
}

// ---- one writer of grades/ at a time: the sync run lock (T9 finding 1) ----

/// `grades/` is a synced folder. The slot's `sync` child, *Sync now*, the quit push and every other
/// `grades` run take `state/sync.lock`, so a run that finds it held writes nothing and says so at
/// exit 0. The test holds the same file with the same exclusive OS lock `sync::RunLock` takes,
/// standing in for a sync mid-pull (`app/tests/week.rs`'s pattern).
#[test]
fn a_run_that_finds_the_sync_run_lock_held_writes_nothing_and_says_so_at_exit_0() {
    let v = vault("busy");
    let lock_path = v.join(knowlu_engine::sync::RUN_LOCK_FILE);
    std::fs::create_dir_all(lock_path.parent().unwrap()).unwrap();
    let held = std::fs::OpenOptions::new().create(true).write(true).open(&lock_path).unwrap();
    held.try_lock().unwrap();
    let out = run(&v, "bundle-basic.json");
    assert!(out.status.success(), "{out:?}");
    assert_eq!(stdout(&out).trim_end(), "grades (skipped: the vault is busy with a sync or another grades run)");
    assert!(!v.join("grades").exists(), "no note while a sync holds the lock");
    assert!(!v.join("state").join("journal").exists(), "no journal record");
    assert!(!v.join("state").join("grades.json").exists(), "state/grades.json untouched");
    let log = std::fs::read_to_string(v.join("state").join("runner-log.md")).unwrap();
    assert!(log.contains("local ok grades (skipped: the vault is busy with a sync or another grades run)"), "{log}");
    drop(held);
    let out = run(&v, "bundle-basic.json");
    assert!(stdout(&out).lines().any(|l| l == "grades: 2 courses, 11 changed items"), "the lock was the only reason: {}", stdout(&out));
    // and the run let go of it: a sync that starts now gets the lock
    let again = std::fs::OpenOptions::new().write(true).open(&lock_path).unwrap();
    again.try_lock().expect("grades released the run lock when it finished");
    cleanup(&v);
}

// ---- unmatched and failed courses ----

#[test]
fn an_unmatched_course_and_a_failed_course_are_named_and_change_nothing() {
    let v = vault("unmatched");
    std::fs::remove_file(v.join("courses").join("syn-330.md")).unwrap();
    let out = run(&v, "bundle-basic.json");
    assert!(out.status.success(), "{out:?}");
    let s = stdout(&out);
    assert!(s.lines().any(|l| l == "grades: SYN-330-002-FA2026 not matched"), "{s}");
    assert!(s.lines().any(|l| l == "grades: SYN-220-001-FA2026 failed (403)"), "{s}");
    assert!(s.lines().any(|l| l == "grades: 1 course, 7 changed items"), "{s}");
    assert!(grade_notes(&v).iter().all(|p| p.starts_with("grades/syn-110-")), "{:?}", grade_notes(&v));

    // Now SYN 330 is known and captured, then fails: its notes stay as they were, none removed.
    std::fs::write(v.join("courses").join("syn-330.md"), "---\ntitle: Synthetic Studies III\ncode: SYN-330-002-FA2026\n---\n").unwrap();
    assert!(run(&v, "bundle-basic.json").status.success());
    let before = vault_bytes(&v);
    let mut failing: serde_json::Value = serde_json::from_str(&bundle_text("bundle-changed.json")).unwrap();
    let c330 = &mut failing["courses"][2];
    c330["columns"] = serde_json::json!([]);
    c330["grades"] = serde_json::json!([]);
    c330["error"] = serde_json::json!("500");
    failing["courses"][0] = serde_json::from_str::<serde_json::Value>(&bundle_text("bundle-basic.json")).unwrap()["courses"][0].clone();
    let out = run_text(&v, &failing.to_string());
    assert!(stdout(&out).lines().any(|l| l == "grades: SYN-330-002-FA2026 failed (500)"), "{}", stdout(&out));
    let mut after = vault_bytes(&v);
    after.remove("state/grades.json");
    let mut before = before;
    before.remove("state/grades.json");
    assert_eq!(after, before, "a failed course changes no note");
    cleanup(&v);
}

// ---- state/grades.json ----

#[test]
fn state_grades_json_holds_fetched_at_the_host_and_the_course_counts_through_dumps_value() {
    let v = vault("state");
    std::fs::remove_file(v.join("courses").join("syn-330.md")).unwrap();
    assert!(run(&v, "bundle-basic.json").status.success());
    let text = std::fs::read_to_string(v.join("state").join("grades.json")).expect("state/grades.json");
    let expected = serde_json::json!({
        "fetched_at": "2026-09-29T14:02:11Z",
        "host": "lms.example.test",
        "courses": {"matched": 1, "skipped": 1, "failed": 1},
    });
    assert_eq!(text, format!("{}\n", knowlu_engine::ledger::dumps_value(&expected)));
    cleanup(&v);
}

// ---- exit codes ----

#[test]
fn a_missing_or_unparseable_bundle_exits_1_and_writes_nothing() {
    let v = vault("exit1");
    let before = files(&v);
    let missing = run_args(&v, &[&v.with_extension("no-such-bundle.json")]);
    assert_eq!(missing.status.code(), Some(1), "{missing:?}");
    let garbled = run_text(&v, "{not json");
    assert_eq!(garbled.status.code(), Some(1), "{garbled:?}");
    // serde's Display would quote the offending value; the message names its category, line and
    // column only.
    let quoting = run_text(&v, r#"{"schema": 1, "lms": "blackboard", "host": "h", "fetched_at": "t", "user_id": "u", "courses": "Midterm Exam 91.5"}"#);
    assert_eq!(quoting.status.code(), Some(1), "{quoting:?}");
    let err = String::from_utf8_lossy(&quoting.stderr);
    assert!(!err.contains("Midterm") && !err.contains("91.5"), "{err}");
    assert!(err.contains("line 1") && err.contains("column"), "{err}");
    let not_v1 = run_text(&v, &bundle_text("bundle-basic.json").replacen("\"schema\": 1", "\"schema\": 2", 1));
    assert_eq!(not_v1.status.code(), Some(1), "{not_v1:?}");
    let mut after = files(&v);
    after.remove("state/runner-log.md");
    assert_eq!(after, before, "nothing but the runner log");
    assert!(run_args(&v, &[]).status.code() == Some(2), "--input is required");
    cleanup(&v);
}

// ---- no grade reaches a log (spec 11) ----

/// The output shapes spec 6 documents: counts, course codes, HTTP and error codes. A per-item line
/// (`changed: Midterm 1 score 87`) matches none of them.
fn documented_shape(line: &str) -> bool {
    let shapes = [
        r"^grades: \d+ courses?, \d+ changed items?$",
        r"^grades: [A-Za-z0-9_.-]+ not matched$",
        r"^grades: [A-Za-z0-9_.-]+ failed \([A-Za-z0-9_. -]+\)$",
        r"^grades: (grades/[a-z0-9-]+\.md|state/grades\.json) not written \([a-z-]+\)$",
        r"^knowlu-engine: --input (could not be read|is not a capture bundle) \([a-z0-9 ,-]+\)$",
    ];
    shapes.iter().any(|s| regex::Regex::new(s).unwrap().is_match(line))
}

/// Every column `name` and category `title` the fixture carries.
fn names_in(fixture: &str) -> BTreeSet<String> {
    let b: serde_json::Value = serde_json::from_str(&bundle_text(fixture)).unwrap();
    let mut out = BTreeSet::new();
    for c in b["courses"].as_array().unwrap() {
        for col in c["columns"].as_array().unwrap() {
            out.insert(col["name"].as_str().unwrap().to_string());
        }
        for cat in c["categories"].as_array().into_iter().flatten() {
            out.insert(cat["title"].as_str().unwrap().to_string());
        }
    }
    out
}

#[test]
fn no_column_name_category_title_or_score_reaches_stdout_stderr_or_the_runner_log() {
    let v = vault("logs");
    std::fs::remove_file(v.join("courses").join("syn-330.md")).unwrap();
    for fixture in ["bundle-basic.json", "bundle-changed.json"] {
        if fixture == "bundle-changed.json" {
            std::fs::write(v.join("courses").join("syn-330.md"), "---\ntitle: T\ncode: SYN-330-002-FA2026\n---\n").unwrap();
        }
        let out = run(&v, fixture);
        assert!(out.status.success(), "{out:?}");
        let log = std::fs::read_to_string(v.join("state").join("runner-log.md")).unwrap();
        let last = log.lines().last().unwrap().to_string();
        let summary = regex::Regex::new(r"^- \d{4}-\d{2}-\d{2} \d{2}:\d{2} local (ok|WARN|FAIL) (.*)$").unwrap();
        let caps = summary.captures(&last).unwrap_or_else(|| panic!("{last}"));
        let mut lines: Vec<String> = caps[2].split("; ").map(str::to_string).collect();
        lines.extend(stdout(&out).lines().map(str::to_string));
        lines.extend(String::from_utf8_lossy(&out.stderr).lines().map(str::to_string));
        assert!(lines.len() >= 3, "{lines:?}");
        for line in &lines {
            assert!(documented_shape(line), "{fixture}: {line:?}");
            for name in names_in(fixture) {
                assert!(!line.contains(&name), "{fixture}: {name:?} in {line:?}");
            }
        }
    }
    // and exit 1's line is a documented shape too
    let garbled = run_text(&v, "{\"schema\": \"Homework 1\"}");
    let err = String::from_utf8_lossy(&garbled.stderr).to_string();
    assert!(err.lines().all(documented_shape) && !err.contains("Homework"), "{err}");
    cleanup(&v);
}
