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

use jiff::civil::{Date, DateTime, Time};

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
///
/// `rank` has no moment: it is `rank_at` with `None`, so it is the pressure order and nothing else.
pub fn rank(tasks: &[Task], today: Date, cal: &WeekCalendar) -> Vec<Task> {
    rank_at(tasks, today, cal, None)
}

/// The hour from which tasks closing tonight outrank the rest: 20:00 in the vault's zone.
/// Quinn's ruling (a), 2026-10-24.
pub const EVENING_HOUR: Time = Time::constant(20, 0, 0, 0);

/// When a task closes tonight, as seen from moment `m`, or `None` if it does not.
///
/// v1: a task closes tonight when its `due` falls on `m`'s date and is strictly after `m`. A due
/// time already passed, or equal to the moment, has closed. **This is the one place O6's late-window
/// close goes**: a late task whose window closes tonight returns that close here. Daily late-penalty
/// steps are out of v1 (ruling b), so only a window's close counts.
fn closes_tonight(task: &Task, m: DateTime) -> Option<DateTime> {
    task.due.filter(|due| due.date() == m.date() && *due > m)
}

/// `rank` at a moment. With `None`, or before `EVENING_HOUR`, or when the moment is not on `today`,
/// this is exactly the pressure order. From `EVENING_HOUR` on, tasks closing tonight (see
/// `closes_tonight`) come first, earliest close first, ties keeping the pressure order (stable
/// sort on exact `DateTime`s, no float key); every other task keeps its pressure order after them.
/// Rank overrides are inserted last, as `rank` always did, so they still win. Deterministic, no model.
pub fn rank_at(tasks: &[Task], today: Date, cal: &WeekCalendar, moment: Option<DateTime>) -> Vec<Task> {
    let mut decorated: Vec<(f64, i64, Task)> = tasks
        .iter()
        .map(|t| (pressure(t, today, cal), -t.importance, t.clone()))
        .collect();
    decorated.sort_by(|a, b| b.0.total_cmp(&a.0).then(b.1.cmp(&a.1)));
    let scored: Vec<Task> = decorated.into_iter().map(|(_, _, t)| t).collect();

    let mut overridden: Vec<Task> = scored.iter().filter(|t| t.rank_override.is_some()).cloned().collect();
    let mut result: Vec<Task> = scored.into_iter().filter(|t| t.rank_override.is_none()).collect();

    if let Some(m) = moment.filter(|m| m.date() == today && m.time() >= EVENING_HOUR) {
        let mut closing: Vec<(DateTime, Task)> = Vec::new();
        let mut rest: Vec<Task> = Vec::new();
        for t in result {
            match closes_tonight(&t, m) {
                Some(at) => closing.push((at, t)),
                None => rest.push(t),
            }
        }
        closing.sort_by_key(|(at, _)| *at);
        result = closing.into_iter().map(|(_, t)| t).chain(rest).collect();
    }

    overridden.sort_by_key(|t| t.rank_override.unwrap_or(0));
    for task in overridden {
        let target = task.rank_override.unwrap_or(1) - 1;
        let index = target.max(0).min(result.len() as i64) as usize;
        result.insert(index, task);
    }
    result
}

/// The spare time step 0 and step 3 require: Quinn's Q4, 2026-10-24.
pub const FIT_MARGIN_MINUTES: i64 = 30;

/// An item that cannot fit before its deadline (filled in by T3b).
#[derive(Debug, Clone, PartialEq)]
pub struct WontFit {
    pub slug: String,
    pub deadline: DateTime,
    pub work_minutes: i64,
    pub available_minutes: i64,
    pub prefix: Vec<String>,
    pub prefix_minutes: i64,
    pub overflower: String,
    pub overflow_deadline: DateTime,
}

/// The deadline plan.
#[derive(Debug, Clone, PartialEq)]
pub struct DeadlinePlan {
    pub order: Vec<Task>,
    pub safe: bool,
    pub wont_fit: Vec<WontFit>,
}

/// Whole minutes of work left on a task: `round_ties_even(hours * 60)`, so every figure after this
/// is an integer.
pub fn work_minutes(task: &Task) -> i64 {
    (task.remaining_hours() * 60.0).round_ties_even() as i64
}

/// Free time from a moment forward, to local midnight each date: `weekcal`'s midnight blocks per
/// date, the date of the moment cut to start at it. Civil minutes, no task involved.
pub struct Timeline {
    blocks: Vec<(DateTime, DateTime)>,
}

impl Timeline {
    /// `from` is truncated to the minute. Each date from `from`'s to `until`'s (at most 365) has its
    /// blocks computed whole, minimum filter included, and only then clipped to start at `from`.
    pub fn build(cal: &WeekCalendar, from: DateTime, until: DateTime) -> Timeline {
        let from = from.with().second(0).subsec_nanosecond(0).build().unwrap_or(from);
        let mut blocks = Vec::new();
        let mut date = from.date();
        for _ in 0..365 {
            if date > until.date() {
                break;
            }
            for block in cal.free_blocks_to_midnight(date) {
                let start = block.start.max(from);
                if block.end > start {
                    blocks.push((start, block.end));
                }
            }
            match date.tomorrow() {
                Ok(next) => date = next,
                Err(_) => break,
            }
        }
        Timeline { blocks }
    }

    /// `A(d)`: whole minutes of the timeline before `d`. Never shrinks as `d` moves later.
    pub fn before(&self, d: DateTime) -> i64 {
        self.blocks
            .iter()
            .map(|(start, end)| {
                let end = (*end).min(d);
                if end > *start { start.duration_until(end).as_secs() / 60 } else { 0 }
            })
            .sum()
    }
}

/// The deadline an item is held to at moment `m`: its `due` while that is after `m`, else `None`
/// (overdue, no window). **This is the one place O6's late-window close goes**: a late item whose
/// window closes after `m` returns that close here. Daily late-penalty steps are out of v1.
fn deadline(task: &Task, m: DateTime) -> Option<DateTime> {
    task.due.filter(|due| *due > m)
}

/// `rank_at`'s successor. With no moment, or a moment not on `today`, this is `rank`. Otherwise Must
/// do (non-override, slack <= 0) is checked in pressure order against the timeline (step 0); when
/// every dated item keeps `FIT_MARGIN_MINUTES` spare the order is `rank`'s, else dated items go in
/// deadline order (step 1), then overdue ones in pressure order, refilled into Must do's positions.
/// The walk (step 2) and the overdue merge (step 3) come later. Deterministic, no model.
pub fn plan_at(tasks: &[Task], today: Date, cal: &WeekCalendar, moment: Option<DateTime>) -> DeadlinePlan {
    let Some(m) = moment.filter(|m| m.date() == today) else {
        return DeadlinePlan { order: rank(tasks, today, cal), safe: true, wont_fit: Vec::new() };
    };
    let m = m.with().second(0).subsec_nanosecond(0).build().unwrap_or(m);
    let mut decorated: Vec<(f64, i64, Task)> =
        tasks.iter().map(|t| (pressure(t, today, cal), -t.importance, t.clone())).collect();
    decorated.sort_by(|a, b| b.0.total_cmp(&a.0).then(b.1.cmp(&a.1)));
    let (overridden, mut result): (Vec<Task>, Vec<Task>) =
        decorated.into_iter().map(|(_, _, t)| t).partition(|t| t.rank_override.is_some());

    let positions: Vec<usize> = (0..result.len())
        .filter(|&i| slack_days(&result[i], today, cal, DEFAULT_BUFFER_DAYS) <= 0.0)
        .collect();
    let last = positions.iter().filter_map(|&i| deadline(&result[i], m)).max();
    let timeline = Timeline::build(cal, m, last.unwrap_or(m));
    let mut total = 0;
    let safe = positions.iter().all(|&i| {
        total += work_minutes(&result[i]);
        deadline(&result[i], m).is_none_or(|d| total <= timeline.before(d) - FIT_MARGIN_MINUTES)
    });
    if !safe {
        let mut dated: Vec<(DateTime, usize)> =
            positions.iter().filter_map(|&i| deadline(&result[i], m).map(|d| (d, i))).collect();
        dated.sort();
        let overdue = positions.iter().copied().filter(|&i| deadline(&result[i], m).is_none());
        let refill: Vec<Task> =
            dated.iter().map(|(_, i)| *i).chain(overdue).map(|i| result[i].clone()).collect();
        for (&position, task) in positions.iter().zip(refill) {
            result[position] = task;
        }
    }
    let mut overridden = overridden;
    overridden.sort_by_key(|t| t.rank_override.unwrap_or(0));
    for task in overridden {
        let index = (task.rank_override.unwrap_or(1) - 1).max(0).min(result.len() as i64) as usize;
        result.insert(index, task);
    }
    DeadlinePlan { order: result, safe, wont_fit: Vec::new() }
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

    // ---- The evening rule (R1-R15). Invented tasks on the vault-s1 week template. ----

    /// Friday 2026-08-28; `at(27, ..)` is Thursday, `at(29, ..)` Saturday.
    const D: Date = Date::constant(2026, 8, 28);

    fn at(day: i8, hour: i8, minute: i8) -> DateTime {
        DateTime::constant(2026, 8, day, hour, minute, 0, 0)
    }

    fn week() -> WeekCalendar {
        WeekCalendar::from_file(Path::new("tests/fixtures/vault-s1/config/week_template.yaml"), Vec::new())
    }

    fn task(slug: &str, due: Option<DateTime>, effort: f64, importance: i64) -> Task {
        Task {
            slug: slug.into(),
            title: slug.into(),
            due,
            effort_hours: effort,
            importance,
            status: "active".into(),
            progress: 0,
            course: None,
            domain: "school".into(),
            rank_override: None,
            slice_hours: None,
            conflicts_with: None,
        }
    }

    fn slugs(tasks: &[Task]) -> Vec<String> {
        tasks.iter().map(|t| t.slug.clone()).collect()
    }

    fn names(list: &[&str]) -> Vec<String> {
        list.iter().map(|s| s.to_string()).collect()
    }

    /// An overdue task (Wednesday) and one due tonight, the overdue one with the higher pressure.
    fn overdue_and_tonight() -> Vec<Task> {
        vec![task("tonight", Some(at(28, 23, 59)), 1.0, 3), task("overdue", Some(at(26, 23, 59)), 2.0, 3)]
    }

    fn assert_overdue_leads(tasks: &[Task]) {
        let cal = week();
        assert_eq!(slugs(&rank(tasks, D, &cal))[0], "overdue", "setup: the overdue task must lead under rank");
    }

    #[test]
    fn rank_at_with_no_moment_is_rank() {
        let (tasks, cal, _) = fixture();
        assert_eq!(rank_at(&tasks, TODAY, &cal, None), rank(&tasks, TODAY, &cal));
        let mem = overdue_and_tonight();
        assert_eq!(rank_at(&mem, D, &week(), None), rank(&mem, D, &week()));
    }

    #[test]
    fn before_the_hour_nothing_changes() {
        let tasks = overdue_and_tonight();
        assert_overdue_leads(&tasks);
        let cal = week();
        assert_eq!(rank_at(&tasks, D, &cal, Some(at(28, 19, 59))), rank(&tasks, D, &cal));
    }

    #[test]
    fn at_the_hour_a_task_due_tonight_leads_an_overdue_one() {
        let tasks = overdue_and_tonight();
        assert_overdue_leads(&tasks);
        let got = rank_at(&tasks, D, &week(), Some(at(28, 20, 0)));
        assert_eq!(slugs(&got), names(&["tonight", "overdue"]));
    }

    #[test]
    fn closing_tonight_orders_by_due_time_then_pressure() {
        let tasks = vec![
            task("due-2100", Some(at(28, 21, 0)), 1.0, 1),
            task("big-2359", Some(at(28, 23, 59)), 12.0, 3),
            task("small-2359", Some(at(28, 23, 59)), 1.0, 3),
        ];
        let cal = week();
        let ranked = slugs(&rank(&tasks, D, &cal));
        assert_ne!(ranked[0], "due-2100", "setup: the 21:00 task must not already lead");
        assert_eq!(ranked[..2], names(&["big-2359", "small-2359"])[..], "setup: big has the higher pressure");
        let got = rank_at(&tasks, D, &cal, Some(at(28, 20, 30)));
        assert_eq!(slugs(&got), names(&["due-2100", "big-2359", "small-2359"]));
    }

    #[test]
    fn a_due_time_already_passed_is_not_closing_tonight() {
        let tasks = vec![
            task("passed-1900", Some(at(28, 19, 0)), 1.0, 3),
            task("equal-2100", Some(at(28, 21, 0)), 1.0, 3),
            task("overdue", Some(at(26, 23, 59)), 2.0, 3),
        ];
        let cal = week();
        assert_eq!(slugs(&rank(&tasks, D, &cal))[0], "overdue", "setup");
        assert_eq!(rank_at(&tasks, D, &cal, Some(at(28, 21, 0))), rank(&tasks, D, &cal));
    }

    #[test]
    fn a_past_due_task_with_no_window_is_not_closing_tonight() {
        let tasks = vec![
            task("tonight", Some(at(28, 22, 0)), 1.0, 3),
            task("overdue-old", Some(at(25, 23, 59)), 2.0, 3),
            task("overdue-today", Some(at(28, 8, 0)), 8.0, 3),
        ];
        let cal = week();
        let ranked = rank(&tasks, D, &cal);
        assert_ne!(ranked[0].slug, "tonight", "setup");
        let got = slugs(&rank_at(&tasks, D, &cal, Some(at(28, 21, 0))));
        let mut want = names(&["tonight"]);
        want.extend(slugs(&ranked).into_iter().filter(|s| s != "tonight"));
        assert_eq!(got, want, "the overdue tasks keep rank's order below tonight's item");
    }

    #[test]
    fn tonight_ends_at_local_midnight() {
        let cal = week();
        let tasks = vec![task("after-midnight", Some(at(29, 0, 30)), 1.0, 3), task("overdue", Some(at(26, 23, 59)), 2.0, 3)];
        assert_eq!(slugs(&rank(&tasks, D, &cal))[0], "overdue", "setup");
        assert_eq!(rank_at(&tasks, D, &cal, Some(at(28, 23, 0))), rank(&tasks, D, &cal));

        let sat = Date::constant(2026, 8, 29);
        let tasks = vec![task("later", Some(at(29, 23, 59)), 1.0, 3), task("overdue", Some(at(26, 23, 59)), 2.0, 3)];
        assert_eq!(slugs(&rank(&tasks, sat, &cal))[0], "overdue", "setup");
        assert_eq!(rank_at(&tasks, sat, &cal, Some(at(29, 0, 30))), rank(&tasks, sat, &cal));
    }

    #[test]
    fn a_moment_on_another_day_changes_nothing() {
        let tasks = overdue_and_tonight();
        assert_overdue_leads(&tasks);
        let cal = week();
        for moment in [at(29, 21, 0), at(27, 21, 0)] {
            assert_eq!(rank_at(&tasks, D, &cal, Some(moment)), rank(&tasks, D, &cal), "{moment}");
        }
    }

    #[test]
    fn a_rank_override_still_wins_after_the_hour() {
        let mut pinned = task("pinned", Some(at(26, 23, 59)), 2.0, 3);
        pinned.rank_override = Some(1);
        let tasks = vec![
            task("tonight", Some(at(28, 23, 59)), 1.0, 3),
            pinned,
            task("overdue-old", Some(at(25, 23, 59)), 2.0, 3),
        ];
        let cal = week();
        assert_eq!(slugs(&rank(&tasks, D, &cal)), names(&["pinned", "overdue-old", "tonight"]), "setup");
        let got = rank_at(&tasks, D, &cal, Some(at(28, 21, 0)));
        assert_eq!(slugs(&got), names(&["pinned", "tonight", "overdue-old"]));
    }

    #[test]
    fn the_evening_order_keeps_must_do_and_the_takes() {
        let mut tasks = overdue_and_tonight();
        tasks.push(task("far", Some(DateTime::constant(2026, 9, 7, 12, 0, 0, 0)), 1.0, 3));
        let cal = week();
        let moment = Some(at(28, 21, 0));
        let evening = rank_at(&tasks, D, &cal, moment);
        let daytime = rank(&tasks, D, &cal);
        assert_ne!(slugs(&evening), slugs(&daytime), "the evening order must differ for this test to mean anything");
        let sorted_must = |ranked: &[Task]| {
            let mut s = slugs(&partition_must_do(ranked, D, &cal).0);
            s.sort();
            s
        };
        assert_eq!(sorted_must(&evening), sorted_must(&daytime));
        assert_eq!(
            designate_today_explained(&evening, D, &cal, None),
            designate_today_explained(&daytime, D, &cal, None)
        );
    }

    #[test]
    fn the_evening_order_is_deterministic() {
        let tasks = vec![
            task("tonight-2100", Some(at(28, 21, 0)), 1.0, 3),
            task("tonight-2300", Some(at(28, 23, 0)), 4.0, 4),
            task("overdue-a", Some(at(26, 23, 59)), 2.0, 3),
            task("overdue-b", Some(at(24, 23, 59)), 3.0, 2),
            task("far", Some(DateTime::constant(2026, 9, 7, 12, 0, 0, 0)), 1.0, 3),
        ];
        let cal = week();
        let mut pressures: Vec<u64> = tasks.iter().map(|t| pressure(t, D, &cal).to_bits()).collect();
        pressures.sort();
        pressures.dedup();
        assert_eq!(pressures.len(), tasks.len(), "setup: every pressure differs");
        let moment = Some(at(28, 20, 15));
        let first = rank_at(&tasks, D, &cal, moment);
        assert_eq!(first, rank_at(&tasks, D, &cal, moment));
        let mut reversed = tasks.clone();
        reversed.reverse();
        assert_eq!(first, rank_at(&reversed, D, &cal, moment));
    }

    #[test]
    fn undated_tasks_never_close_tonight() {
        let mut tasks = overdue_and_tonight();
        tasks.push(task("undated", None, 1.0, 3));
        let cal = week();
        let daytime = slugs(&rank(&tasks, D, &cal));
        assert_eq!(daytime.last().unwrap(), "undated", "setup");
        let got = slugs(&rank_at(&tasks, D, &cal, Some(at(28, 21, 0))));
        assert_eq!(got, names(&["tonight", "overdue", "undated"]));
    }

    #[test]
    fn a_bare_date_due_today_closes_at_2359() {
        let meta: serde_yaml_ng::Mapping = serde_yaml_ng::from_str("due: 2026-08-28\neffort_hours: 1.0\n").unwrap();
        let bare = Task::from_meta(&meta, "bare".into()).unwrap();
        let tasks = vec![bare, task("overdue", Some(at(26, 23, 59)), 2.0, 3)];
        let cal = week();
        assert_eq!(slugs(&rank(&tasks, D, &cal))[0], "overdue", "setup");
        let got = rank_at(&tasks, D, &cal, Some(at(28, 21, 0)));
        assert_eq!(slugs(&got), names(&["bare", "overdue"]));
    }

    #[test]
    fn evening_hour_is_twenty_hundred() {
        assert_eq!(
            EVENING_HOUR,
            Time::constant(20, 0, 0, 0),
            "20:00 is Quinn's ruling (a), 2026-10-24; changing it needs a new ruling"
        );
    }

    #[test]
    fn everything_else_keeps_the_pressure_order() {
        let tasks = vec![
            task("overdue-a", Some(at(26, 23, 59)), 2.0, 3),
            task("tonight-2200", Some(at(28, 22, 0)), 1.0, 3),
            task("overdue-b", Some(at(24, 23, 59)), 3.0, 2),
            task("tonight-2330", Some(at(28, 23, 30)), 2.0, 3),
            task("far", Some(DateTime::constant(2026, 9, 7, 12, 0, 0, 0)), 1.0, 3),
            task("undated", None, 1.0, 3),
        ];
        let cal = week();
        let closing = ["tonight-2200", "tonight-2330"];
        let rest = |ranked: Vec<Task>| -> Vec<String> {
            slugs(&ranked).into_iter().filter(|s| !closing.contains(&s.as_str())).collect()
        };
        let evening = rank_at(&tasks, D, &cal, Some(at(28, 21, 0)));
        assert_eq!(slugs(&evening)[..2], names(&closing)[..], "the closing group leads, by due time");
        assert_eq!(rest(evening), rest(rank(&tasks, D, &cal)));
    }

    // ---- Deadline ordering: the timeline and step 0 (DP1-DP10). ----

    /// `at` with seconds, for the minute-truncation tests.
    fn at_s(day: i8, hour: i8, minute: i8, second: i8) -> DateTime {
        DateTime::constant(2026, 8, day, hour, minute, second, 0)
    }

    fn assert_first(tasks: &[Task], slug: &str) {
        assert_eq!(slugs(&rank(tasks, D, &week()))[0], slug, "setup: {slug} must lead under rank");
    }

    /// A 3h overdue lab and a 1h task due tonight; the lab leads under `rank`.
    fn lab_and_homework() -> Vec<Task> {
        vec![task("lab", Some(at(26, 23, 59)), 3.0, 3), task("hw", Some(at(28, 23, 59)), 1.0, 3)]
    }

    /// DP10's pair: the 23:59 task leads under `rank`, the 21:00 one must go first.
    fn late_big_and_early_small() -> Vec<Task> {
        vec![task("big-2359", Some(at(28, 23, 59)), 2.0, 4), task("small-2100", Some(at(28, 21, 0)), 0.5, 2)]
    }

    #[test]
    fn fit_margin_is_thirty_minutes() {
        assert_eq!(
            FIT_MARGIN_MINUTES, 30,
            "30 minutes is Quinn's Q4 ruling, 2026-10-24; changing it needs a new ruling"
        );
    }

    #[test]
    fn work_is_whole_minutes() {
        assert_eq!(work_minutes(&task("a", None, 2.79, 3)), 167);
        assert_eq!(work_minutes(&task("b", None, 0.5, 3)), 30);
        let mut progressed = task("c", None, 3.1, 3);
        progressed.progress = 10;
        assert_eq!(work_minutes(&progressed), 167);
    }

    #[test]
    fn the_timeline_runs_to_midnight_and_cuts_at_the_moment() {
        let cal = week();
        let until = at(28, 23, 59);
        assert_eq!(Timeline::build(&cal, at(28, 21, 0), until).before(until), 179);
        assert_eq!(Timeline::build(&cal, at(28, 15, 0), until).before(until), 539);
        assert_eq!(Timeline::build(&cal, at(28, 9, 0), until).before(until), 794);
    }

    #[test]
    fn the_cut_comes_after_the_block_filter() {
        // 08:00-12:00 is a real block; cut at 11:30 it is 30 minutes, under the 45-minute minimum.
        let timeline = Timeline::build(&week(), at(28, 11, 30), at(28, 23, 59));
        assert_eq!(timeline.before(at(28, 12, 0)), 30);
    }

    #[test]
    fn available_time_never_shrinks_as_the_deadline_moves_later() {
        let timeline = Timeline::build(&week(), at(27, 19, 0), at(28, 23, 59));
        assert_eq!(timeline.before(at(28, 7, 59)), 300);
        assert_eq!(timeline.before(at(28, 12, 0)), 540);
    }

    #[test]
    fn the_moment_is_truncated_to_the_minute() {
        let tasks = vec![task("due-2100", Some(at(28, 21, 0)), 1.0, 3), task("big", Some(at(28, 23, 59)), 2.5, 3)];
        let cal = week();
        assert_first(&tasks, "due-2100");
        assert_eq!(
            plan_at(&tasks, D, &cal, Some(at_s(28, 21, 0, 45))),
            plan_at(&tasks, D, &cal, Some(at(28, 21, 0)))
        );
        // Due at 21:00 and seen at 21:00:30 is overdue, not on time: it goes after the dated big task.
        let plan = plan_at(&tasks, D, &cal, Some(at_s(28, 21, 0, 30)));
        assert!(!plan.safe);
        assert_eq!(slugs(&plan.order), names(&["big", "due-2100"]));
    }

    #[test]
    fn no_moment_or_another_day_is_rank() {
        let tasks = late_big_and_early_small();
        let cal = week();
        assert_first(&tasks, "big-2359");
        for moment in [None, Some(at(29, 19, 30)), Some(at(27, 19, 30))] {
            let plan = plan_at(&tasks, D, &cal, moment);
            assert_eq!(plan.order, rank(&tasks, D, &cal), "moment {moment:?}");
            assert!(plan.safe, "moment {moment:?}");
            assert!(plan.wont_fit.is_empty(), "moment {moment:?}");
        }
    }

    #[test]
    fn a_safe_pressure_order_is_kept_with_dated_items() {
        let tasks = vec![task("essay", Some(at(28, 23, 59)), 11.0, 5), task("reflection", Some(at(27, 23, 59)), 0.5, 1)];
        let cal = week();
        assert_first(&tasks, "essay");
        let plan = plan_at(&tasks, D, &cal, Some(at(28, 9, 0)));
        assert_eq!(plan.order, rank(&tasks, D, &cal));
        assert!(plan.safe);
    }

    #[test]
    fn under_thirty_minutes_spare_is_unsafe() {
        let tasks = lab_and_homework();
        let cal = week();
        assert_first(&tasks, "lab");
        assert!(plan_at(&tasks, D, &cal, Some(at(28, 19, 29))).safe, "240 minutes in 270, 30 spare");
        assert!(!plan_at(&tasks, D, &cal, Some(at(28, 19, 30))).safe, "240 minutes in 269, 29 spare");
    }

    #[test]
    fn unsafe_dated_items_go_in_deadline_order() {
        let tasks = late_big_and_early_small();
        let cal = week();
        assert_first(&tasks, "big-2359");
        let plan = plan_at(&tasks, D, &cal, Some(at(28, 19, 30)));
        assert!(!plan.safe);
        assert_eq!(slugs(&plan.order), names(&["small-2100", "big-2359"]));
        assert!(plan.wont_fit.is_empty());

        // Same due time: ties keep the pressure order.
        let tied = vec![
            task("c-2359", Some(at(28, 23, 59)), 0.5, 5),
            task("qa-2100", Some(at(28, 21, 0)), 0.6, 3),
            task("qb-2100", Some(at(28, 21, 0)), 0.6, 2),
        ];
        assert_first(&tied, "c-2359");
        let ranked = slugs(&rank(&tied, D, &cal));
        let mut expected: Vec<String> = ranked.iter().filter(|s| s.ends_with("2100")).cloned().collect();
        expected.push("c-2359".into());
        let plan = plan_at(&tied, D, &cal, Some(at(28, 19, 30)));
        assert!(!plan.safe);
        assert_eq!(slugs(&plan.order), expected);
    }
}
