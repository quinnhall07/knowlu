//! M1 grades, the app's half (spec `2026-09-29-grades-design.md`). Nothing here opens a window or
//! reaches a network.
//!
//! **Task 5a: the gate.** Grades are offered only for a curated campus row whose `lms_kind` is
//! `blackboard` and which carries `policy_read` (spec §4; cloud design ruling 12). One predicate,
//! `grades::availability`, decides it. The date-and-bump test keeps any such date off `main` until
//! privacy bump #1 has put spec §11's sentences on the page.
//!
//! Every row a test needs is built by `test_row` and nothing else — never by editing
//! `scaffold::CAMPUSES` and never as a full `Curated` literal, so a field another lane adds to
//! `Curated` needs no edit here.
//!
//! **Task 5b: the pure pieces** — `grades.json`, the bundle's assembly from canned responses, paging,
//! sign-in detection and the kept session's directory. Still no window and no network: every
//! response here is canned JSON and every host is `lms.example.test`.
use knowlu::grades::{
    assemble_bundle, availability, forget, is_session_dir, next_page, read_list, session_dir, signed_in, Availability,
    CaptureHead, CourseCalls, GradesPrefs, MAX_PAGES,
};
use knowlu::scaffold::{create_vault, Curated, VaultPlan, CAMPUSES};
use serde_json::{json, Value};
use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

/// The one place this file builds a curated row: a synthetic school (`unitid` and `lms_host` no real
/// school uses), everything else from `CAMPUSES[0]`.
fn test_row(lms_kind: &'static str, policy_read: Option<&'static str>) -> Curated {
    Curated { unitid: "999999", lms_host: "lms.example.test", lms_kind, policy_read, ..CAMPUSES[0] }
}

fn temp(tag: &str) -> PathBuf {
    let d = std::env::temp_dir().join(format!("knowlu-grades-{tag}-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&d);
    std::fs::create_dir_all(&d).unwrap();
    d
}

// ---- `Curated.policy_read` -------------------------------------------------------------------

/// No curated row carries a date on this branch: the field is absent (`None`), never an empty
/// string standing in for one.
#[test]
fn no_curated_row_carries_a_policy_read_date() {
    assert_eq!(CAMPUSES.len(), 2);
    for c in CAMPUSES {
        assert_eq!(c.policy_read, None, "{} carries a policy_read on m1-grades", c.key);
    }
}

/// Task 1's carried finding: `grades/` is a note folder, so a vault the wizard makes has it from
/// birth.
#[test]
fn a_new_vault_is_born_with_a_grades_folder() {
    let v = temp("scaffold").join("Vault");
    let plan = VaultPlan {
        profile_id: knowlu::profiles::id_for(&v),
        ics_url: None,
        personal_calendar: None,
        google_calendar: false,
        timezone: "America/Chicago".into(),
        slots: vec!["12:00".into()],
        device: "M".into(),
        campus: "none".into(),
        campus_choice: Default::default(),
        zybooks: false,
        vhl: false,
        zybooks_courses: Vec::new(),
        vhl_sections: Vec::new(),
        zybooks_ignore: Vec::new(),
        course_map: Vec::new(),
        courses: Vec::new(),
        api_base: knowlu::account::api_base(),
        anon_key: "anon".into(),
        account_id: "acc-1".into(),
        human_actor: knowlu_engine::journal::HUMAN_ACTOR,
    };
    create_vault(&v, &plan).unwrap();
    assert!(v.join("grades").is_dir(), "grades/ is part of the scaffold");
}

// ---- the predicate ---------------------------------------------------------------------------

#[test]
fn a_dated_blackboard_row_is_available_with_its_own_host() {
    let row = test_row("blackboard", Some("2026-10-01"));
    assert_eq!(availability(Some(&row), "blackboard"), Availability::Available { host: "lms.example.test" });
    // The row decides the host and the kind; the vault's own `lms` does not override it.
    assert_eq!(availability(Some(&row), ""), Availability::Available { host: "lms.example.test" });
}

#[test]
fn an_undated_blackboard_row_is_not_available_yet() {
    let row = test_row("blackboard", None);
    assert_eq!(availability(Some(&row), "blackboard"), Availability::NotAvailableYet);
}

/// No address entry (spec §4): an uncurated Blackboard school waits for a curated row with a read.
#[test]
fn an_uncurated_blackboard_school_is_not_available_yet() {
    assert_eq!(availability(None, "blackboard"), Availability::NotAvailableYet);
}

#[test]
fn a_curated_canvas_row_is_not_a_blackboard_school() {
    // Dated or not, and whatever the vault's `lms` says: the curated row's kind wins.
    assert_eq!(availability(Some(&test_row("canvas", None)), "canvas"), Availability::NotBlackboard);
    assert_eq!(availability(Some(&test_row("canvas", Some("2026-10-01"))), "blackboard"), Availability::NotBlackboard);
}

/// A vault with no `unitid` has no curated row and no `lms` (`config/campus.yaml` absent or blank).
#[test]
fn no_school_is_not_a_blackboard_school() {
    assert_eq!(availability(None, ""), Availability::NotBlackboard);
    assert_eq!(availability(None, "canvas"), Availability::NotBlackboard);
}

/// The real rows, as they stand on this branch: nothing is available anywhere.
#[test]
fn no_real_campus_is_available_on_this_branch() {
    for c in CAMPUSES {
        assert_ne!(availability(Some(&c), c.lms_kind), Availability::Available { host: c.lms_host }, "{}", c.key);
    }
}

// ---- the date-and-bump rule (spec §4; ruling 12) ---------------------------------------------

/// **Privacy bump #1's own `PRIVACY_VERSION`**, the version whose page discloses the kept session and
/// the grades the account holds (spec §11). `None` until bump #1's PR sets it, in the commit that puts
/// spec §11's sentences on the page. Keyed on that version and never on "any version after
/// `2026-09-24`", so a version moved for another page edit never lets a `policy_read` date through.
const PRIVACY_BUMP_1: Option<&str> = None;

/// The version in force when the grades spec was signed; its page does not disclose the kept session,
/// so bump #1 is later than it.
const SIGNED_AT_VERSION: &str = "2026-09-24";

/// `YYYY-MM-DD` exactly: ten characters and a real calendar date. Equal-length ISO dates then compare
/// correctly as strings.
fn is_iso_date(s: &str) -> bool {
    s.len() == 10 && jiff::civil::Date::strptime("%Y-%m-%d", s).is_ok()
}

/// The pure check. `Err` when any row carries a `policy_read` date while `version` is older than
/// bump #1 — every version is, while `anchor` is `None` — or when a date, the version or a set
/// anchor is not `YYYY-MM-DD`, or a set anchor is not later than `SIGNED_AT_VERSION`.
fn date_and_bump(rows: &[Curated], version: &str, anchor: Option<&str>) -> Result<(), String> {
    if !is_iso_date(version) {
        return Err(format!("privacy version {version:?} is not YYYY-MM-DD"));
    }
    if let Some(a) = anchor {
        if !is_iso_date(a) {
            return Err(format!("PRIVACY_BUMP_1 {a:?} is not YYYY-MM-DD"));
        }
        if a <= SIGNED_AT_VERSION {
            return Err(format!("PRIVACY_BUMP_1 {a} is not later than {SIGNED_AT_VERSION}"));
        }
    }
    for row in rows {
        let Some(date) = row.policy_read else { continue };
        if !is_iso_date(date) {
            return Err(format!("{}: policy_read {date:?} is not YYYY-MM-DD", row.unitid));
        }
        match anchor {
            None => return Err(format!("{}: policy_read {date} before privacy bump #1 (none recorded)", row.unitid)),
            Some(a) if version < a => {
                return Err(format!("{}: policy_read {date} while PRIVACY_VERSION {version} is older than bump #1 ({a})", row.unitid))
            }
            Some(_) => {}
        }
    }
    Ok(())
}

#[test]
fn a_dated_row_fails_while_bump_1_has_not_happened() {
    let dated = [test_row("blackboard", Some("2026-10-01"))];
    assert!(date_and_bump(&dated, "2026-09-24", None).is_err(), "at the version in force at signing");
    assert!(date_and_bump(&dated, "2026-10-02", None).is_err(), "a later version is not bump #1 by itself");
    assert!(date_and_bump(&dated, "2026-10-02", Some("2026-10-10")).is_err(), "a version older than the anchor");
}

#[test]
fn an_undated_row_passes_and_a_dated_row_passes_at_or_after_its_anchor() {
    assert!(date_and_bump(&[test_row("blackboard", None)], "2026-09-24", None).is_ok());
    let dated = [test_row("blackboard", Some("2026-10-01"))];
    assert!(date_and_bump(&dated, "2026-10-10", Some("2026-10-10")).is_ok(), "at the anchor");
    assert!(date_and_bump(&dated, "2026-11-01", Some("2026-10-10")).is_ok(), "after the anchor");
}

#[test]
fn dates_and_the_anchor_must_be_iso_and_the_anchor_later_than_signing() {
    let undated = [test_row("blackboard", None)];
    for bad in ["2026-9-24", "20261010", "2026-13-01", "2026-02-30", "", "2026-10-10T00:00"] {
        assert!(date_and_bump(&[test_row("blackboard", Some(bad))], "2026-11-01", Some("2026-10-10")).is_err(), "date {bad:?}");
        assert!(date_and_bump(&undated, "2026-11-01", Some(bad)).is_err(), "anchor {bad:?}");
    }
    assert!(date_and_bump(&undated, "2026-09-24", Some("2026-09-24")).is_err(), "bump #1 is later than signing");
    assert!(date_and_bump(&undated, "2026-09-24", Some("2026-09-01")).is_err());
}

/// The pin: the real rows, the real version and this file's anchor.
#[test]
fn the_real_campuses_pass_the_date_and_bump_rule() {
    date_and_bump(&CAMPUSES, knowlu::account::PRIVACY_VERSION, PRIVACY_BUMP_1).unwrap();
}

// ---- `grades.json`, the preferences (spec §3) ------------------------------------------------

/// `hidden` and nothing else — no LMS host, ever — written through `ledger::dumps_value`.
#[test]
fn grades_prefs_hold_hidden_only_and_round_trip_through_dumps_value() {
    let d = temp("prefs");
    assert_eq!(GradesPrefs::default(), GradesPrefs { hidden: false });
    GradesPrefs { hidden: true }.save(&d).unwrap();
    let text = std::fs::read_to_string(d.join("grades.json")).unwrap();
    assert_eq!(text, knowlu_engine::ledger::dumps_value(&json!({ "hidden": true })));
    assert_eq!(text, r#"{"hidden": true}"#);
    assert_eq!(GradesPrefs::load(&d), GradesPrefs { hidden: true });
    GradesPrefs { hidden: false }.save(&d).unwrap();
    assert_eq!(GradesPrefs::load(&d), GradesPrefs { hidden: false });
    assert!(!d.join("settings.json").exists(), "grades.json sits beside settings.json, never in it");
}

#[test]
fn a_missing_or_corrupt_grades_json_is_not_hidden_and_never_touches_settings() {
    let d = temp("prefs-bad");
    std::fs::write(d.join("settings.json"), "SETTINGS, UNTOUCHED").unwrap();
    assert_eq!(GradesPrefs::load(&d), GradesPrefs { hidden: false }, "missing");
    for bad in ["", "{", "[]", "null", r#"{"hidden": "yes"}"#, r#"{"shown": true}"#] {
        std::fs::write(d.join("grades.json"), bad).unwrap();
        assert_eq!(GradesPrefs::load(&d), GradesPrefs { hidden: false }, "{bad:?}");
    }
    GradesPrefs { hidden: true }.save(&d).unwrap();
    assert_eq!(std::fs::read_to_string(d.join("settings.json")).unwrap(), "SETTINGS, UNTOUCHED");
}

// ---- the capture bundle (spec §5), from canned responses -------------------------------------

const HOST: &str = "lms.example.test";

/// One page of a Blackboard list, verbatim in shape: `results`, and `paging.nextPage` while more
/// pages remain.
fn page(results: Value, next: Option<&str>) -> Value {
    match next {
        Some(n) => json!({ "results": results, "paging": { "nextPage": n } }),
        None => json!({ "results": results }),
    }
}

fn membership(course_id: &str, available: &str) -> Value {
    json!({ "userId": "_900001_1", "courseId": course_id, "availability": { "available": available },
            "course": { "id": course_id, "courseId": format!("SYN-{course_id}"), "name": "Synthetic" } })
}

fn head() -> CaptureHead<'static> {
    CaptureHead { lms: "blackboard", host: HOST, fetched_at: "2026-09-29T14:02:11Z" }
}

#[test]
fn assemble_bundle_produces_the_spec_shape_merging_pages_and_skipping_unavailable_memberships() {
    let user = json!({ "id": "_900001_1", "userName": "synthetic" });
    let (m1, m2, m3) = (membership("_1_1", "Yes"), membership("_2_1", "No"), membership("_3_1", "Yes"));
    let memberships = vec![page(json!([m1, m2]), Some("/learn/api/public/v1/users/me/courses?offset=2")), page(json!([m3]), None)];
    let (c1, c2, c3) = (json!({ "id": "_c1_1", "name": "HW 1" }), json!({ "id": "_c2_1" }), json!({ "id": "_c3_1" }));
    let (g1, g2) = (json!({ "columnId": "_c1_1", "score": 18.0 }), json!({ "columnId": "_c3_1", "score": 7.0 }));
    let cat = json!({ "id": "_k1_1", "title": "Homework" });
    let mut per_course = BTreeMap::new();
    per_course.insert(
        "_1_1".to_string(),
        CourseCalls {
            columns: Ok(vec![page(json!([c1, c2]), Some("/next")), page(json!([c3]), None)]),
            grades: Ok(vec![page(json!([g1]), Some("/next")), page(json!([g2]), None)]),
            categories: Ok(vec![page(json!([cat]), None)]),
        },
    );
    // A refused category list only drops the grouping: `categories` is null, never an error.
    per_course.insert(
        "_3_1".to_string(),
        CourseCalls { columns: Ok(vec![page(json!([]), None)]), grades: Ok(vec![page(json!([]), None)]), categories: Err("403".into()) },
    );
    let b = assemble_bundle(&head(), &user, &memberships, &per_course).unwrap();
    let course = |m: &Value, cols: Value, grades: Value, cats: Value| {
        json!({ "membership": m, "columns": cols, "grades": grades, "categories": cats, "error": null })
    };
    assert_eq!(
        b,
        json!({ "schema": 1, "lms": "blackboard", "host": HOST, "fetched_at": "2026-09-29T14:02:11Z", "user_id": "_900001_1",
                "courses": [ course(&m1, json!([c1, c2, c3]), json!([g1, g2]), json!([cat])),
                             course(&m3, json!([]), json!([]), Value::Null) ] })
    );
    // The engine reads exactly what the app writes.
    let parsed: knowlu_engine::grades::Bundle = serde_json::from_value(b).unwrap();
    assert_eq!(parsed.courses.len(), 2);
}

/// A course whose calls failed carries `"error": "<status or reason>"` and empty arrays; the others
/// are untouched. An available membership nobody captured is named the same way, never dropped.
#[test]
fn a_failed_course_records_its_error_and_empty_arrays() {
    let user = json!({ "id": "_900001_1" });
    let ms = vec![page(json!([membership("_1_1", "Yes"), membership("_2_1", "Yes"), membership("_3_1", "Yes"), membership("_4_1", "Yes")]), None)];
    let ok = || Ok(vec![page(json!([{ "id": "_c_1" }]), None)]);
    let mut per_course = BTreeMap::new();
    per_course.insert("_1_1".to_string(), CourseCalls { columns: Err("500".into()), grades: ok(), categories: ok() });
    per_course.insert("_2_1".to_string(), CourseCalls { columns: ok(), grades: Err("unreachable".into()), categories: ok() });
    // A page that is not a list is unreadable, whichever call it came from.
    per_course.insert("_3_1".to_string(), CourseCalls { columns: Ok(vec![json!({ "oops": true })]), grades: ok(), categories: ok() });
    let b = assemble_bundle(&head(), &user, &ms, &per_course).unwrap();
    let errors: Vec<&Value> = b["courses"].as_array().unwrap().iter().map(|c| &c["error"]).collect();
    assert_eq!(errors, [&json!("500"), &json!("unreachable"), &json!("unreadable"), &json!("not captured")]);
    for c in b["courses"].as_array().unwrap() {
        assert_eq!((&c["columns"], &c["grades"], &c["categories"]), (&json!([]), &json!([]), &Value::Null), "{c}");
    }
}

#[test]
fn a_bundle_needs_a_user_id_and_readable_memberships() {
    let ms = vec![page(json!([]), None)];
    assert!(assemble_bundle(&head(), &json!({}), &ms, &BTreeMap::new()).is_err(), "users/me without an id");
    assert!(assemble_bundle(&head(), &json!({ "id": "_900001_1" }), &[json!({ "oops": 1 })], &BTreeMap::new()).is_err());
    let empty = assemble_bundle(&head(), &json!({ "id": "_900001_1" }), &ms, &BTreeMap::new()).unwrap();
    assert_eq!(empty["courses"], json!([]), "no enrolment is an empty bundle, not a failure");
}

// ---- paging (spec §3: every list to the end, 20 pages at most) -------------------------------

#[test]
fn next_page_follows_paging_next_page_only_on_the_same_host() {
    let next = |n: Value| next_page(HOST, &json!({ "results": [], "paging": { "nextPage": n } }));
    let path = "/learn/api/public/v1/users/me/courses?offset=100&expand=course";
    assert_eq!(next(json!(path)), Some(format!("https://{HOST}{path}")), "Blackboard's relative nextPage");
    assert_eq!(next(json!("https://lms.example.test/x?offset=2")), Some("https://lms.example.test/x?offset=2".into()));
    assert_eq!(next(json!("https://LMS.Example.Test/x")), Some("https://LMS.Example.Test/x".into()), "hosts are case-blind");
    assert_eq!(next_page(HOST, &json!({ "results": [] })), None, "the last page");
    for foreign in [
        json!(null), json!(""), json!(7),
        json!("https://evil.example/x"), json!("http://lms.example.test/x"), json!("//evil.example/x"),
        json!("https://lms.example.test.evil.example/x"), json!("https://lms.example.test@evil.example/x"),
        json!("https://lms.example.test:8443/x"), json!("x?offset=2"), json!("/\\evil.example/x"),
        json!("https://evil.example/?h=lms.example.test"), json!("/x y"), json!("/x\ny"),
    ] {
        assert_eq!(next(foreign.clone()), None, "{foreign}");
    }
}

#[test]
fn read_list_follows_pages_to_the_end_and_stops_at_twenty() {
    assert_eq!(MAX_PAGES, 20);
    // A runaway list: every page names another.
    let mut calls = Vec::new();
    let pages = read_list(HOST, "https://lms.example.test/a", |url: &str| {
        calls.push(url.to_string());
        Ok(page(json!([calls.len()]), Some(format!("/a?offset={}", calls.len()).as_str())).to_string())
    })
    .unwrap();
    assert_eq!((calls.len(), pages.len()), (20, 20));
    assert_eq!(calls[1], "https://lms.example.test/a?offset=1");
    // A list that ends, and one whose nextPage points off the host, which is not followed.
    let mut n = 0;
    let three = read_list(HOST, "https://lms.example.test/a", |_: &str| {
        n += 1;
        Ok(page(json!([n]), (n < 3).then_some("/a?more")).to_string())
    });
    assert_eq!(three.unwrap().len(), 3);
    let off_host = read_list(HOST, "https://lms.example.test/a", |_: &str| Ok(page(json!([]), Some("https://evil.example/a")).to_string()));
    assert_eq!(off_host.unwrap().len(), 1);
    // A failed call is its status; a body that is not JSON is unreadable.
    assert_eq!(read_list(HOST, "https://lms.example.test/a", |_: &str| Err("403".into())), Err("403".to_string()));
    assert_eq!(read_list(HOST, "https://lms.example.test/a", |_: &str| Ok("<html>".into())), Err("unreadable".to_string()));
}

// ---- sign-in detection (spec §15, default 1) -------------------------------------------------

#[test]
fn signed_in_needs_the_window_on_the_lms_host_and_users_me_answering_200() {
    let on = Some("https://lms.example.test/ultra/course");
    assert!(signed_in(on, HOST, Some(200)));
    assert!(!signed_in(Some("https://sso.example.test/login?next=lms.example.test"), HOST, Some(200)), "200 alone is not");
    assert!(!signed_in(on, HOST, Some(401)), "the host alone is not");
    assert!(!signed_in(on, HOST, Some(403)));
    assert!(!signed_in(on, HOST, None), "users/me not asked or unreachable");
    assert!(!signed_in(None, HOST, Some(200)), "no window");
    assert!(!signed_in(Some("https://lms.example.test.evil.example/"), HOST, Some(200)));
    assert!(!signed_in(Some("http://lms.example.test/"), HOST, Some(200)));
}

// ---- the kept session's directory (spec §4) --------------------------------------------------

#[test]
fn session_dir_is_lms_session_in_the_profile_and_is_session_dir_refuses_anything_else() {
    let d = temp("session");
    assert_eq!(session_dir(&d), d.join("lms-session"));
    assert!(is_session_dir(&d, &session_dir(&d)));
    let other = temp("session-other");
    for wrong in [
        d.clone(), d.parent().unwrap().to_path_buf(), d.join("lms-session").join("Default"), d.join("lms-session-2"),
        d.join("LMS-SESSION"), d.join("lms-session").join("..").join("lms-session"), other.join("lms-session"),
    ] {
        assert!(!is_session_dir(&d, &wrong), "{}", wrong.display());
    }
    assert!(!is_session_dir(Path::new("relative"), &session_dir(Path::new("relative"))), "a relative profile is refused");
}

#[test]
fn forget_deletes_only_the_session_directory() {
    let d = temp("forget");
    std::fs::create_dir_all(d.join("lms-session").join("Default").join("Network")).unwrap();
    std::fs::write(d.join("lms-session").join("Default").join("Network").join("Cookies"), "synthetic").unwrap();
    std::fs::write(d.join("settings.json"), "{}").unwrap();
    std::fs::write(d.join("grades.json"), r#"{"hidden": true}"#).unwrap();
    std::fs::create_dir_all(d.join("lms-session-keep")).unwrap();
    assert!(forget(&d));
    assert!(!d.join("lms-session").exists());
    for kept in ["settings.json", "grades.json", "lms-session-keep"] {
        assert!(d.join(kept).exists(), "{kept} survives forget");
    }
    assert!(forget(&d), "nothing to forget is done, not a failure");
    assert!(!forget(Path::new("relative")), "a relative profile is refused");
}

// ---- the capability file (spec §4) -----------------------------------------------------------

/// `lms-grades` has no capability grant, like `lms-signin`: the console window is the only window
/// named, so neither a label nor a glob can reach the Blackboard window.
#[test]
fn the_capability_file_never_names_the_grades_window() {
    assert_eq!(knowlu::grades::WINDOW, "lms-grades");
    let text = std::fs::read_to_string("capabilities/default.json").unwrap();
    assert!(!text.contains(knowlu::grades::WINDOW), "capabilities/default.json names lms-grades");
    let v: Value = serde_json::from_str(&text).unwrap();
    assert_eq!(v["windows"], json!(["main"]));
    assert!(v.get("webviews").is_none());
}
