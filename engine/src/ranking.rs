//! Port of `engine/ranking.py` — pressure, rank, the must-do partition, and today's designation.
//!
//! The golden file's Recommended block is this module's output verbatim:
//!
//! ```text
//! - [ ] **GN 103 Hausaufgaben — …** — 1.5h (of 2.8h left)
//! - [ ] **BUI 101 reading - Augustine, …** — 2.0h (of 2.5h left)
//! ```
//!
//! Both numbers fall out of the interaction between the 2.0h session slice and the 4.0h daily
//! budget, so the fixture pins the interaction rather than either mechanism alone.

use jiff::civil::Date;

use crate::models::Task;
use crate::planning::PlanningConfig;
use crate::scheduling::{slack_days, DEFAULT_BUFFER_DAYS};
use crate::weekcal::WeekCalendar;

pub const MIN_SESSION_HOURS: f64 = 0.75;

/// Python's `round(x, 2)` is round-half-to-EVEN, not half-up.
///
/// `remaining` and `budget` are re-rounded on every take in `designate_today`, so a half-up
/// implementation would drift from Python on any value landing exactly on a half-cent — and those
/// values then decide which task gets the last block.
fn round2(x: f64) -> f64 {
    (x * 100.0).round_ties_even() / 100.0
}

/// Higher means work on it sooner. Driven by slack, scaled by importance.
pub fn pressure(task: &Task, today: Date, cal: &WeekCalendar) -> f64 {
    let slack = slack_days(task, today, cal, DEFAULT_BUFFER_DAYS);
    let base = if slack < 0.0 {
        100.0 + slack.abs() * 10.0
    } else {
        50.0 / (1.0 + slack)
    };
    round2(base * (0.6 + 0.1 * task.importance as f64))
}

/// Rank by pressure, then apply manual overrides by insertion.
///
/// **Preserved quirk.** Python sorts on the tuple `(pressure, -importance)` with `reverse=True`,
/// which reverses BOTH components — so among tasks of equal pressure the one with the *lower*
/// importance sorts first. That reads like a bug and it is not fixed here: this is a
/// behaviour-preserving port, and the golden file is rendered from the current behaviour.
///
/// Python's sort is stable and so is Rust's `sort_by`, so fully-equal keys keep their input order
/// in both. Keys are computed once per task (decorate-sort-undecorate) to match Python's `key=`
/// semantics — `pressure` walks up to 365 days of capacity, so calling it from a comparator would
/// be both slower and, for NaN, differently ordered.
pub fn rank(tasks: &[Task], today: Date, cal: &WeekCalendar) -> Vec<Task> {
    let mut decorated: Vec<(f64, i64, Task)> = tasks
        .iter()
        .map(|t| (pressure(t, today, cal), -t.importance, t.clone()))
        .collect();
    decorated.sort_by(|a, b| b.0.total_cmp(&a.0).then(b.1.cmp(&a.1)));
    let scored: Vec<Task> = decorated.into_iter().map(|(_, _, t)| t).collect();

    let mut overridden: Vec<Task> = scored.iter().filter(|t| t.rank_override.is_some()).cloned().collect();
    let mut result: Vec<Task> = scored.into_iter().filter(|t| t.rank_override.is_none()).collect();

    overridden.sort_by_key(|t| t.rank_override.unwrap_or(0));
    for task in overridden {
        let target = task.rank_override.unwrap_or(1) - 1;
        let index = target.max(0).min(result.len() as i64) as usize;
        result.insert(index, task);
    }
    result
}

/// Split into (must do today no matter what, everything recommendable).
///
/// A task is must-do when its slack is zero or negative: starting any later than today means
/// missing the deadline given real free hours. **Duration never decides visibility here** — that
/// is the must-do-first rule the whole page is built on.
pub fn partition_must_do(tasks: &[Task], today: Date, cal: &WeekCalendar) -> (Vec<Task>, Vec<Task>) {
    let must = tasks
        .iter()
        .filter(|t| slack_days(t, today, cal, DEFAULT_BUFFER_DAYS) <= 0.0)
        .cloned()
        .collect();
    let rest = tasks
        .iter()
        .filter(|t| slack_days(t, today, cal, DEFAULT_BUFFER_DAYS) > 0.0)
        .cloned()
        .collect();
    (must, rest)
}

/// Take `need` hours out of the remaining block space, front to back.
fn consume(remaining: &mut [f64], mut need: f64) {
    for available in remaining.iter_mut() {
        let take = need.min(*available);
        *available = round2(*available - take);
        need -= take;
        if need <= 0.0 {
            return;
        }
    }
}

/// A single recommendation with the arithmetic that produced it kept (spec §4.3): the page
/// renders which block it came from and which cap decided the hours — "2.0h of 2.8h, capped by
/// your session slice" versus "1.0h of 2.5h, capped by the block".
#[derive(Debug, Clone, PartialEq)]
pub struct Take {
    pub task: Task,
    pub hours: f64,
    pub block_index: usize,
    pub block_before: f64,
    pub block_after: f64,
    pub budget_after: f64,
    pub capped_by: &'static str,
}

/// Recommend tasks for today's free time, filling toward the effort budget.
///
/// Must-dos are not recommendations, but they reserve block time first — capped at their session
/// slice, because contributing more than a session to one task today is fiction and would starve
/// everything else. Recurring commitments consume block time and budget next. Then recommendations
/// fill in rank order, each take bounded by the task's remaining work, its session cap, the block's
/// space, and the budget still unspent.
pub fn designate_today_explained(
    ranked: &[Task],
    today: Date,
    cal: &WeekCalendar,
    planning: Option<&PlanningConfig>,
) -> Vec<Take> {
    let default = PlanningConfig::default();
    let planning = planning.unwrap_or(&default);

    let (must, rest) = partition_must_do(ranked, today, cal);
    let mut remaining: Vec<f64> = cal.free_blocks(today).iter().map(|b| b.hours()).collect();
    let mut budget = planning.daily_effort_budget;

    for task in &must {
        let mut need = task.remaining_hours();
        if let Some(slice) = task.slice_hours {
            need = need.min(slice);
        }
        consume(&mut remaining, need);
    }

    for item in planning.recurring_for(today) {
        consume(&mut remaining, item.hours);
        budget = round2(budget - item.hours);
    }

    let mut chosen = Vec::new();
    for task in rest {
        if budget < MIN_SESSION_HOURS {
            break;
        }
        for index in 0..remaining.len() {
            let available = remaining[index];
            if available < MIN_SESSION_HOURS {
                continue;
            }
            let budget_before = budget;
            let mut take = task.remaining_hours().min(available).min(budget);
            if let Some(slice) = task.slice_hours {
                take = take.min(slice);
            }
            take = round2(take);
            // A whole task smaller than MIN_SESSION_HOURS is not a "scrap": the floor scales down
            // to the task's own remaining work so a short task can still be finished in one sitting.
            let floor = MIN_SESSION_HOURS.min(task.remaining_hours());
            if take <= 0.0 || take < floor {
                continue;
            }
            // Named in the order the caps were applied above, so the first match is the binding one.
            let capped_by = if take == round2(task.remaining_hours()) {
                "remaining"
            } else if take == round2(available) {
                "block"
            } else if take == round2(budget_before) {
                "budget"
            } else {
                "slice"
            };
            remaining[index] = round2(available - take);
            budget = round2(budget - take);
            chosen.push(Take {
                task: task.clone(),
                hours: take,
                block_index: index,
                block_before: available,
                block_after: remaining[index],
                budget_after: budget,
                capped_by,
            });
            break;
        }
    }
    chosen
}

pub fn designate_today(
    ranked: &[Task],
    today: Date,
    cal: &WeekCalendar,
    planning: Option<&PlanningConfig>,
) -> Vec<(Task, f64)> {
    designate_today_explained(ranked, today, cal, planning).into_iter().map(|t| (t.task, t.hours)).collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::models::{apply_slice_default, load_tasks};
    use crate::planning::load_planning;
    use std::path::Path;

    const TODAY: Date = Date::constant(2026, 8, 28);

    fn fixture() -> (Vec<Task>, WeekCalendar, PlanningConfig) {
        let planning = load_planning(Path::new("tests/fixtures/vault-s1/config/planning.yaml"));
        let mut tasks = load_tasks(Path::new("tests/fixtures/vault-s1/tasks"), None);
        apply_slice_default(&mut tasks, planning.slice_hours);
        let cal = WeekCalendar::from_file(
            Path::new("tests/fixtures/vault-s1/config/week_template.yaml"),
            Vec::new(),
        );
        (tasks, cal, planning)
    }

    #[test]
    fn must_do_is_exactly_the_two_overdue_tasks() {
        let (tasks, cal, _) = fixture();
        let (must, rest) = partition_must_do(&tasks, TODAY, &cal);
        let mut slugs: Vec<&str> = must.iter().map(|t| t.slug.as_str()).collect();
        slugs.sort();
        assert_eq!(slugs, vec!["cs-100-hw-01", "task-erste-reflexion"]);
        assert_eq!(rest.len(), 3);
    }

    /// The golden file's Recommended block, exactly.
    #[test]
    fn designate_today_reproduces_the_golden_recommendations() {
        let (tasks, cal, planning) = fixture();
        let ranked = rank(&tasks, TODAY, &cal);
        let chosen = designate_today(&ranked, TODAY, &cal, Some(&planning));

        let got: Vec<(&str, f64)> = chosen.iter().map(|(t, h)| (t.slug.as_str(), *h)).collect();
        assert_eq!(
            got,
            vec![
                ("gn-103-hausaufgaben-2026-08-31", 1.5),
                ("bui-101-reading-augustine-iv-vi", 2.0),
            ],
            "must match the golden file's Recommended block"
        );
    }

    #[test]
    fn ph_106_is_refused_because_the_budget_is_below_the_session_floor() {
        // After 1.5 + 2.0 of a 4.0 budget, 0.5 remains - under MIN_SESSION_HOURS, so the loop
        // breaks and PH 106 never appears despite having 12 hours of work outstanding.
        let (tasks, cal, planning) = fixture();
        let ranked = rank(&tasks, TODAY, &cal);
        let chosen = designate_today(&ranked, TODAY, &cal, Some(&planning));
        assert!(!chosen.iter().any(|(t, _)| t.slug == "ph-106-exam-1-prep"));
    }

    #[test]
    fn overdue_tasks_rank_above_slack_ones() {
        let (tasks, cal, _) = fixture();
        let ranked = rank(&tasks, TODAY, &cal);
        assert_eq!(ranked[0].slug, "cs-100-hw-01", "most overdue first");
        assert_eq!(ranked.last().unwrap().slug, "ph-106-exam-1-prep", "most slack last");
    }

    #[test]
    fn pressure_is_higher_for_negative_slack() {
        let (tasks, cal, _) = fixture();
        let overdue = tasks.iter().find(|t| t.slug == "cs-100-hw-01").unwrap();
        let relaxed = tasks.iter().find(|t| t.slug == "ph-106-exam-1-prep").unwrap();
        assert!(pressure(overdue, TODAY, &cal) > pressure(relaxed, TODAY, &cal));
    }

    #[test]
    fn rank_override_inserts_at_a_one_based_position() {
        let (tasks, cal, _) = fixture();
        let mut tasks = tasks;
        // Force the least urgent task to the top.
        for t in tasks.iter_mut() {
            if t.slug == "ph-106-exam-1-prep" {
                t.rank_override = Some(1);
            }
        }
        let ranked = rank(&tasks, TODAY, &cal);
        assert_eq!(ranked[0].slug, "ph-106-exam-1-prep");
        assert_eq!(ranked.len(), 5);
    }

    #[test]
    fn an_out_of_range_override_is_clamped_not_an_error() {
        let (tasks, cal, _) = fixture();
        let mut tasks = tasks;
        for t in tasks.iter_mut() {
            if t.slug == "ph-106-exam-1-prep" {
                t.rank_override = Some(999);
            }
        }
        let ranked = rank(&tasks, TODAY, &cal);
        assert_eq!(ranked.last().unwrap().slug, "ph-106-exam-1-prep");
        assert_eq!(ranked.len(), 5);
    }

    #[test]
    fn round2_is_half_to_even_like_python() {
        assert_eq!(round2(2.675), 2.68);
        assert_eq!(round2(0.125), 0.12, "half-to-even rounds down here, half-up would give 0.13");
    }

    #[test]
    fn a_recurring_commitment_eats_budget_and_block_time() {
        let (tasks, cal, _) = fixture();
        let ranked = rank(&tasks, TODAY, &cal);
        let with_recurring = PlanningConfig {
            daily_effort_budget: 4.0,
            slice_hours: 2.0,
            daily_approval_budget: 15,
            recurring: vec![crate::planning::Recurring {
                name: "German practice".into(),
                hours: 3.0,
                days: vec!["fri".into()],
            }],
        };
        let chosen = designate_today(&ranked, TODAY, &cal, Some(&with_recurring));
        let total: f64 = chosen.iter().map(|(_, h)| h).sum();
        assert!(total <= 1.0, "3h of commitments must leave at most 1h of the 4h budget");
    }

    #[test]
    fn designate_today_explained_agrees_with_designate_today_and_names_the_block() {
        let vault = Path::new("tests/fixtures/vault-full");
        let events = crate::calfeed::read_snapshot(&vault.join("state").join("calendar.md")).into_values().flatten().collect();
        let cal = WeekCalendar::from_file(&vault.join("config").join("week_template.yaml"), events);
        let planning = crate::planning::load_planning(&vault.join("config").join("planning.yaml"));
        let today = Date::constant(2026, 8, 28);
        let mut tasks = crate::models::load_tasks(&vault.join("tasks"), None);
        crate::models::apply_slice_default(&mut tasks, planning.slice_hours);
        let ranked = rank(&tasks, today, &cal);
        let plain = designate_today(&ranked, today, &cal, Some(&planning));
        let takes = designate_today_explained(&ranked, today, &cal, Some(&planning));
        assert_eq!(plain, takes.iter().map(|t| (t.task.clone(), t.hours)).collect::<Vec<_>>());
        // golden-today-full: GN 103 2.0h (of 2.8h) then BUI 101 1.0h (of 2.5h), both from the
        // 15:00–18:00 block — index 2 of the three free blocks on that Friday.
        assert_eq!(takes.len(), 2);
        assert_eq!((takes[0].hours, takes[0].block_index, takes[0].capped_by), (2.0, 2, "slice"));
        assert_eq!((takes[1].hours, takes[1].block_index, takes[1].capped_by), (1.0, 2, "block"));
        assert_eq!(takes[1].block_after, 0.0);
    }
}
