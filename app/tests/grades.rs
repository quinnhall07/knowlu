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
use knowlu::grades::{availability, Availability};
use knowlu::scaffold::{create_vault, Curated, VaultPlan, CAMPUSES};
use std::path::PathBuf;

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
