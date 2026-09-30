//! Grades from Blackboard, the app's half (spec `2026-09-29-grades-design.md`).
//!
//! **The gate** (spec §4; cloud design ruling 12). [`availability`] is the one predicate that decides
//! whether grades are offered, and its four callers — `grades_status`, `grades_connect`,
//! `grades_refresh` and the scheduler's grades step — check it and never re-derive it. No build
//! skips it: no `cfg`, feature or environment variable reaches this function.
use crate::scaffold::Curated;

/// What the predicate answers. `Available` carries the curated row's own `lms_host`, the only host
/// grades are ever fetched from (spec §4: no address entry).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Availability {
    Available { host: &'static str },
    /// The slot's `grades (skipped: not a Blackboard school)`.
    NotBlackboard,
    /// The slot's `grades (skipped: not available at your school yet)`, and the refusal
    /// `grades_connect` and `grades_refresh` return.
    NotAvailableYet,
}

/// **Whether grades are available**, from the vault's curated row (`scaffold::curated` of
/// `config/campus.yaml`'s `unitid`, `None` for an uncurated school or no `unitid`) and that file's
/// `lms`.
///
/// Available only for a curated row whose `lms_kind` is `blackboard` and which carries
/// `policy_read`. A curated row's kind wins over the vault's `lms`; `lms` only tells an uncurated
/// Blackboard school (not available yet) from anything else (not a Blackboard school).
pub fn availability(row: Option<&Curated>, campus_lms: &str) -> Availability {
    match row {
        Some(c) if c.lms_kind != "blackboard" => Availability::NotBlackboard,
        Some(c) => match c.policy_read {
            Some(_) => Availability::Available { host: c.lms_host },
            None => Availability::NotAvailableYet,
        },
        None if campus_lms.trim() == "blackboard" => Availability::NotAvailableYet,
        None => Availability::NotBlackboard,
    }
}
