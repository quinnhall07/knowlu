//! Port of `engine/scheduling.py` — `start_by` and `slack_days`.
//!
//! 36 lines of Python that produce two of the golden file's columns: `start by 8/24` and
//! `1d slack`. Because those are printed for all five fixture tasks, the golden file doubles as a
//! five-case table for this module.
//!
//! # Civil date arithmetic
//!
//! `days_between` and `add_days` are implemented here from Howard Hinnant's `days_from_civil`
//! rather than borrowed from a crate. Date arithmetic is the spine of this engine — every slack,
//! every start-by, every capacity walk goes through it — so it is worth having one small, audited,
//! exactly-specified implementation with no dependency on a third party's span-unit semantics.

use jiff::civil::Date;

use crate::models::Task;
use crate::weekcal::WeekCalendar;

pub const DEFAULT_BUFFER_DAYS: i64 = 1;
pub const NO_DUE_DATE_SLACK: f64 = 999.0;

/// Days since 1970-01-01. Howard Hinnant's `days_from_civil`, valid for any proleptic Gregorian
/// date; exact, branch-cheap, and independent of any calendar library.
pub fn to_days(date: Date) -> i64 {
    let (y, m, d) = (date.year() as i64, date.month() as i64, date.day() as i64);
    let y = if m <= 2 { y - 1 } else { y };
    let era = if y >= 0 { y } else { y - 399 } / 400;
    let yoe = y - era * 400;
    let mp = (m + 9) % 12;
    let doy = (153 * mp + 2) / 5 + d - 1;
    let doe = yoe * 365 + yoe / 4 - yoe / 100 + doy;
    era * 146097 + doe - 719468
}

/// Inverse of [`to_days`].
pub fn from_days(days: i64) -> Date {
    let z = days + 719468;
    let era = if z >= 0 { z } else { z - 146096 } / 146097;
    let doe = z - era * 146097;
    let yoe = (doe - doe / 1460 + doe / 36524 - doe / 146096) / 365;
    let y = yoe + era * 400;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let d = doy - (153 * mp + 2) / 5 + 1;
    let m = if mp < 10 { mp + 3 } else { mp - 9 };
    let y = if m <= 2 { y + 1 } else { y };
    Date::new(y as i16, m as i8, d as i8).expect("valid civil date")
}

pub fn days_between(later: Date, earlier: Date) -> i64 {
    to_days(later) - to_days(earlier)
}

pub fn add_days(date: Date, days: i64) -> Date {
    from_days(to_days(date) + days)
}

#[derive(Debug, Clone, PartialEq)]
pub struct StartByWhy {
    pub start_by: Date,
    pub needed_hours: f64,
    pub days_walked: i64,
    pub slice_cap: Option<f64>,
    pub buffer_days: i64,
}

/// `start_by` with the pieces it computes on the way kept (spec §4.3): the page renders
/// "start by 8/25 — 3.0h needs 2 working days at your 2.0h session cap, minus a 1-day buffer".
///
/// Walks backwards from the due date accumulating each day's capacity, capped at the task's
/// session slice — because contributing more than one session to a task in a day is fiction. The
/// 365-iteration bound is Python's, and it is load-bearing: without it an undoable task loops
/// forever. When the loop exhausts without accumulating enough, Python falls out of the `for` and
/// returns whatever `day` it reached, which is 365 days before the due date. That is preserved.
pub fn start_by_explained(task: &Task, cal: &WeekCalendar, buffer_days: i64) -> Option<StartByWhy> {
    let due = task.due?;
    let needed = task.remaining_hours();
    let mut day = due.date();
    let mut accumulated = 0.0_f64;
    let mut days_walked = 0_i64;
    for _ in 0..365 {
        days_walked += 1;
        let mut daily = cal.capacity(day);
        if let Some(slice) = task.slice_hours {
            daily = daily.min(slice);
        }
        accumulated += daily;
        if accumulated >= needed {
            break;
        }
        day = add_days(day, -1);
    }
    let start_by = add_days(day, -buffer_days);
    Some(StartByWhy { start_by, needed_hours: needed, days_walked, slice_cap: task.slice_hours, buffer_days })
}

/// The last date work can begin and still finish, with a buffer.
pub fn start_by(task: &Task, cal: &WeekCalendar, buffer_days: i64) -> Option<Date> {
    start_by_explained(task, cal, buffer_days).map(|w| w.start_by)
}

/// Days of slack. A task with no due date gets [`NO_DUE_DATE_SLACK`], which sorts it last
/// everywhere without needing a special case.
pub fn slack_days(task: &Task, today: Date, cal: &WeekCalendar, buffer_days: i64) -> f64 {
    match start_by(task, cal, buffer_days) {
        None => NO_DUE_DATE_SLACK,
        Some(target) => days_between(target, today) as f64,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::models::{apply_slice_default, load_tasks};
    use std::path::Path;

    fn fixture() -> (Vec<Task>, WeekCalendar) {
        let mut tasks = load_tasks(Path::new("tests/fixtures/vault-s1/tasks"), None);
        // The engine resolves the config-wide session cap before scheduling; the fixture's
        // planning.yaml sets slice_hours: 2.0, and start_by depends on it.
        apply_slice_default(&mut tasks, 2.0);
        let cal = WeekCalendar::from_file(
            Path::new("tests/fixtures/vault-s1/config/week_template.yaml"),
            Vec::new(),
        );
        (tasks, cal)
    }

    fn task<'a>(tasks: &'a [Task], slug: &str) -> &'a Task {
        tasks.iter().find(|t| t.slug == slug).expect(slug)
    }

    #[test]
    fn civil_day_arithmetic_round_trips() {
        for (y, m, d) in [(1970, 1, 1), (2026, 8, 28), (2000, 2, 29), (1999, 12, 31)] {
            let date = Date::constant(y, m, d);
            assert_eq!(from_days(to_days(date)), date, "{y}-{m}-{d}");
        }
        assert_eq!(to_days(Date::constant(1970, 1, 1)), 0);
        assert_eq!(days_between(Date::constant(2026, 8, 28), Date::constant(2026, 8, 24)), 4);
        assert_eq!(add_days(Date::constant(2026, 3, 1), -1), Date::constant(2026, 2, 28));
    }

    /// The golden file prints `start by` for all five fixture tasks, so this is its table.
    #[test]
    fn start_by_matches_every_golden_row() {
        let (tasks, cal) = fixture();
        let cases = [
            ("cs-100-hw-01", Date::constant(2026, 8, 24)),
            ("task-erste-reflexion", Date::constant(2026, 8, 26)),
            ("gn-103-hausaufgaben-2026-08-31", Date::constant(2026, 8, 29)),
            ("bui-101-reading-augustine-iv-vi", Date::constant(2026, 8, 30)),
            ("ph-106-exam-1-prep", Date::constant(2026, 9, 19)),
        ];
        for (slug, want) in cases {
            assert_eq!(
                start_by(task(&tasks, slug), &cal, DEFAULT_BUFFER_DAYS),
                Some(want),
                "start_by for {slug}"
            );
        }
    }

    /// And its slack column: OVERDUE START twice, then 1d, 2d, 22d.
    #[test]
    fn slack_days_matches_every_golden_row() {
        let (tasks, cal) = fixture();
        let today = Date::constant(2026, 8, 28);
        let cases = [
            ("cs-100-hw-01", -4.0),
            ("task-erste-reflexion", -2.0),
            ("gn-103-hausaufgaben-2026-08-31", 1.0),
            ("bui-101-reading-augustine-iv-vi", 2.0),
            ("ph-106-exam-1-prep", 22.0),
        ];
        for (slug, want) in cases {
            assert_eq!(
                slack_days(task(&tasks, slug), today, &cal, DEFAULT_BUFFER_DAYS),
                want,
                "slack for {slug}"
            );
        }
    }

    #[test]
    fn a_task_with_no_due_date_has_sentinel_slack() {
        let (tasks, cal) = fixture();
        let mut undated = task(&tasks, "cs-100-hw-01").clone();
        undated.due = None;
        assert_eq!(start_by(&undated, &cal, DEFAULT_BUFFER_DAYS), None);
        assert_eq!(
            slack_days(&undated, Date::constant(2026, 8, 28), &cal, DEFAULT_BUFFER_DAYS),
            NO_DUE_DATE_SLACK
        );
    }

    #[test]
    fn the_session_slice_moves_start_by_earlier() {
        // A 2.0h cap on a 2.5h task forces a second day; without the cap one day suffices.
        let (tasks, cal) = fixture();
        let mut uncapped = task(&tasks, "cs-100-hw-01").clone();
        uncapped.slice_hours = None;
        let capped = start_by(task(&tasks, "cs-100-hw-01"), &cal, DEFAULT_BUFFER_DAYS).unwrap();
        let free = start_by(&uncapped, &cal, DEFAULT_BUFFER_DAYS).unwrap();
        assert!(capped < free, "capped {capped} should precede uncapped {free}");
    }

    #[test]
    fn start_by_explained_agrees_with_start_by_on_every_fixture_task_and_says_why() {
        for fixture in ["vault-s1", "vault-s1-migrated", "vault-full"] {
            let vault = Path::new("tests/fixtures").join(fixture);
            let cal = WeekCalendar::from_file(&vault.join("config").join("week_template.yaml"), Vec::new());
            for task in crate::models::load_tasks(&vault.join("tasks"), None) {
                let plain = start_by(&task, &cal, DEFAULT_BUFFER_DAYS);
                let why = start_by_explained(&task, &cal, DEFAULT_BUFFER_DAYS);
                assert_eq!(plain, why.as_ref().map(|w| w.start_by), "{fixture}/{}", task.slug);
                if let Some(w) = why {
                    assert!((w.needed_hours - task.remaining_hours()).abs() < 1e-9);
                    assert!(w.days_walked >= 1, "{}", task.slug);
                    assert_eq!(w.buffer_days, DEFAULT_BUFFER_DAYS);
                }
            }
        }
    }
}
