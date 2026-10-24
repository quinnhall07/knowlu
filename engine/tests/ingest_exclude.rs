//! F6c (plan `docs/plans/2026-10-23-wave-b-long-onboarding-plan.md`, §14.45 and §14.46): `ingest`
//! alone reads `course_exclude`. Every other reader of `config/ingest.yaml` (`commitments`,
//! `enrich`, `cloudmodel`, `judge`'s tier 1, `coursework`) must give the same answer, with no
//! warning, when the file carries the key, and when `course_map` carries pin-form keys
//! (`_<digits>_<digits>`).
//!
//! These hold from the first commit: they pin readers that must STAY ignorant of the key. The red
//! tests for the lane live beside the code they drive (`engine/src/ingest.rs`). Synthetic data only;
//! every vault is a temp dir.

use std::path::PathBuf;
use std::time::Duration;

use knowlu_engine::judge::{Heuristics, Item, Missing};
use knowlu_engine::{cloudmodel, commitments, coursework, enrich, judge};

const BASE: &str = "ics_url: \"https://example.invalid/learn.ics\"\ntimezone: America/Chicago\n\
                    course_map:\n  SYN-110: syn-110\n  SYN-210: syn-210\n";
/// The same file with the removed course's pairs under the new key.
const WITH_EXCLUDE: &str = "ics_url: \"https://example.invalid/learn.ics\"\ntimezone: America/Chicago\n\
                    course_map:\n  SYN-110: syn-110\n  SYN-210: syn-210\ncourse_exclude: [SYN-120, _900072_1]\n";
/// The base file plus kept-course pins in `course_map`, in the pin form.
const BASE_PINS: &str = "ics_url: \"https://example.invalid/learn.ics\"\ntimezone: America/Chicago\n\
                    course_map:\n  SYN-110: syn-110\n  _900071_1: syn-110\n  SYN-210: syn-210\n  _900073_1: syn-210\n";

const NOTE: &str = "---\ntitle: \"SYN-110 Homework 3\"\ncourse: null\ndomain: school\ndue: 2026-10-01T23:59\n\
                    effort_hours: 1.0\neffort_confidence: unset\neffort_source: inferred\nimportance: 3\n\
                    importance_reason: \"pending enrichment\"\nstatus: active\nprogress: 0\ncreated_by: blackboard\n\
                    source_uid: \"blackboard:_900071_1\"\nneeds_enrichment: true\nid: task_1111111111\n---\n\nChapters 4 and 5.\n";

fn vault(tag: &str, config: &str) -> PathBuf {
    let d = std::env::temp_dir().join(format!("knowlu-ingest-exclude-{}-{tag}", std::process::id()));
    let _ = std::fs::remove_dir_all(&d);
    for sub in ["config", "tasks", "courses", "approvals", "state/journal"] {
        std::fs::create_dir_all(d.join(sub)).unwrap();
    }
    std::fs::write(d.join("config").join("ingest.yaml"), config).unwrap();
    std::fs::write(d.join("tasks").join("hw3.md"), NOTE).unwrap();
    std::fs::write(d.join("courses").join("syn-110.md"), "---\ntitle: SYN 110\nslug: syn-110\n---\n\n## Grade weights\n- Homework: 20%\n").unwrap();
    d
}

fn item() -> Item {
    Item {
        id: "task_1111111111".into(),
        rel_path: "tasks/hw3.md".into(),
        title: "Homework 3".into(),
        body: String::new(),
        source_uid: "blackboard:_900071_1".into(),
        created_by: "blackboard".into(),
        course: None,
        due: Some("2026-10-01T23:59".into()),
        effort_hours: 1.0,
        effort_source: "inferred".into(),
    }
}

fn clean(v: &PathBuf) {
    let _ = std::fs::remove_dir_all(v);
}

/// `commitments::code_table`: same table, same warnings, with or without the key.
#[test]
fn commitments_ignores_course_exclude() {
    let plain = vault("cm-plain", BASE);
    let keyed = vault("cm-keyed", WITH_EXCLUDE);
    let (codes_a, warn_a) = commitments::code_table(&plain);
    let (codes_b, warn_b) = commitments::code_table(&keyed);
    assert_eq!(codes_a, codes_b);
    assert_eq!(warn_a, warn_b);
    assert!(warn_b.is_empty(), "{warn_b:?}");
    assert!(!codes_b.contains_key("SYN120"), "a removed course's code is not a source: {codes_b:?}");
    clean(&plain);
    clean(&keyed);
}

/// A pin-form key in `course_map` gives `commitments` no new claimant and no warning (§14.46): the
/// table is exactly the one the code fragments alone give.
#[test]
fn commitments_gets_no_claimant_or_warning_from_a_pin_key() {
    let plain = vault("cp-plain", BASE);
    let pinned = vault("cp-pinned", BASE_PINS);
    let (codes_a, warn_a) = commitments::code_table(&plain);
    let (codes_b, warn_b) = commitments::code_table(&pinned);
    assert_eq!(codes_a, codes_b, "a pin adds no code");
    assert_eq!(warn_a, warn_b);
    assert!(warn_b.is_empty(), "{warn_b:?}");
    clean(&plain);
    clean(&pinned);
}

/// `judge`'s tier 1 reads `course_map` through `Heuristics::load`; the key changes nothing there,
/// and an excluded code never attributes an item to a kept slug.
#[test]
fn judge_tier_1_ignores_course_exclude() {
    let plain = vault("j-plain", BASE);
    let keyed = vault("j-keyed", WITH_EXCLUDE);
    let (a, b) = (Heuristics::load(&plain), Heuristics::load(&keyed));
    assert_eq!(a, b);
    assert_eq!(judge::tier1(&item(), &a), judge::tier1(&item(), &b));
    let mut other = item();
    other.title = "SYN-120 Quiz".into();
    other.source_uid = "bb:other".into();
    assert_eq!(judge::tier1(&other, &b).course, None, "tier 1 never sees the removed course");
    clean(&plain);
    clean(&keyed);
}

/// A pin key beside the code keys changes nothing about what tier 1 reads or answers.
#[test]
fn judge_tier_1_reads_a_pin_key_as_a_pin() {
    let pinned = vault("jp-pinned", BASE_PINS);
    let h = Heuristics::load(&pinned);
    assert_eq!(judge::tier1(&item(), &h).course.as_deref(), Some("syn-110"));
    clean(&pinned);
}

/// `cloudmodel::task_request` (the prompt body): the key adds nothing to what travels.
#[test]
fn cloudmodel_request_ignores_course_exclude() {
    let plain = vault("c-plain", BASE);
    let keyed = vault("c-keyed", WITH_EXCLUDE);
    let build = |v: &PathBuf| {
        let h = Heuristics::load(v);
        let seed = judge::tier1(&item(), &h);
        cloudmodel::task_request(&item(), &h, &seed)
    };
    let (a, b) = (build(&plain), build(&keyed));
    assert_eq!(a, b);
    let text = b.to_string();
    assert!(!text.contains("SYN-120") && !text.contains("_900072_1"), "no excluded fragment travels: {text}");
    clean(&plain);
    clean(&keyed);
}

/// `enrich` (the judge pass): same lines and the same note bytes with or without the key.
#[test]
fn enrich_ignores_course_exclude() {
    let run = |tag: &str, config: &str| {
        let v = vault(tag, config);
        let opts = enrich::Options {
            via: "cli",
            run_id: None,
            log_dir: None,
            limit: 50,
            budget: Duration::from_secs(60),
        };
        let (code, lines) = enrich::enrich_with(&v, &opts, Err(Missing::Service("no entitlement")));
        let mut note = String::from_utf8(std::fs::read(v.join("tasks").join("hw3.md")).unwrap()).unwrap();
        // The `judgment:` mapping carries the wall-clock `at:`; blank it so the bytes compare.
        if let Some(start) = note.find("at: '") {
            let from = start + "at: '".len();
            let len = note[from..].find('\'').unwrap();
            note.replace_range(from..from + len, "T");
        }
        clean(&v);
        (code, lines, note)
    };
    let plain = run("e-plain", BASE);
    let keyed = run("e-keyed", WITH_EXCLUDE);
    assert_eq!(plain, keyed);
    assert_eq!(keyed.0, 0);
}

/// `coursework` reads the whole file as a mapping: the key adds no warning and no entry that
/// was not in the file.
#[test]
fn coursework_ignores_course_exclude() {
    let plain = vault("w-plain", BASE);
    let keyed = vault("w-keyed", WITH_EXCLUDE);
    let (map_a, warn_a) = coursework::load_coursework_config(&plain).unwrap();
    let (map_b, warn_b) = coursework::load_coursework_config(&keyed).unwrap();
    assert!(warn_a.is_empty() && warn_b.is_empty(), "{warn_a:?} {warn_b:?}");
    let keys = |m: &serde_yaml_ng::Mapping| -> Vec<String> {
        m.keys().filter_map(|k| k.as_str().map(str::to_string)).filter(|k| k != "course_exclude").collect()
    };
    assert_eq!(keys(&map_a), keys(&map_b));
    assert_eq!(map_a.get("course_map"), map_b.get("course_map"));
    assert!(map_a.get("coursework").is_none() && map_b.get("coursework").is_none());
    clean(&plain);
    clean(&keyed);
}
