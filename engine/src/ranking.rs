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

use jiff::civil::{Date, DateTime};

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
/// `rank` has no moment: the pressure order and nothing else. (`plan_at` with no moment returns this.)
pub fn rank(tasks: &[Task], today: Date, cal: &WeekCalendar) -> Vec<Task> {
    let (overridden, result) = pressure_sorted(tasks, today, cal);
    insert_overrides(result, overridden)
}

/// The scored list split into (rank-overridden, the rest), the rest in pressure order.
fn pressure_sorted(tasks: &[Task], today: Date, cal: &WeekCalendar) -> (Vec<Task>, Vec<Task>) {
    let mut decorated: Vec<(f64, i64, Task)> =
        tasks.iter().map(|t| (pressure(t, today, cal), -t.importance, t.clone())).collect();
    decorated.sort_by(|a, b| b.0.total_cmp(&a.0).then(b.1.cmp(&a.1)));
    decorated.into_iter().map(|(_, _, t)| t).partition(|t| t.rank_override.is_some())
}

/// Rank overrides are inserted last, in override order, so they still win.
fn insert_overrides(mut result: Vec<Task>, mut overridden: Vec<Task>) -> Vec<Task> {
    overridden.sort_by_key(|t| t.rank_override.unwrap_or(0));
    for task in overridden {
        let index = (task.rank_override.unwrap_or(1) - 1).max(0).min(result.len() as i64) as usize;
        result.insert(index, task);
    }
    result
}

/// `rank` at a moment: the order of `plan_at` (the deadline formula, spec §2). With `None`, or a
/// moment not on `today`, or when today's pressure order is safe, this is exactly the pressure order.
/// Deterministic, no model.
pub fn rank_at(tasks: &[Task], today: Date, cal: &WeekCalendar, moment: Option<DateTime>) -> Vec<Task> {
    plan_at(tasks, today, cal, moment).order
}

/// The spare time step 0 and step 3 require: Quinn's Q4, 2026-10-24.
pub const FIT_MARGIN_MINUTES: i64 = 30;

/// An item the fit walk (spec §2.6) removed because it cannot fit before its deadline. Each record
/// is made at the moment of removal, from the items kept then.
#[derive(Debug, Clone, PartialEq)]
pub struct WontFit {
    pub slug: String,
    pub deadline: DateTime,
    pub work_minutes: i64,
    /// `A(d)`: whole minutes of the timeline before `deadline`.
    pub available_minutes: i64,
    /// Every other item kept at removal, up to and including the overflower.
    pub prefix: Vec<String>,
    /// The prefix's work, in minutes, at removal.
    pub prefix_minutes: i64,
    /// The item whose addition overflowed (may be this one).
    pub overflower: String,
    /// The overflower's deadline.
    pub overflow_deadline: DateTime,
}

/// The deadline plan: the Must do order the page and `today.md` use, whether today's pressure order
/// was safe, and what the walk removed.
#[derive(Debug, Clone, PartialEq)]
pub struct DeadlinePlan {
    pub order: Vec<Task>,
    /// Step 0: every dated item in pressure order keeps `FIT_MARGIN_MINUTES` spare.
    pub safe: bool,
    /// The items that cannot fit, ordered by (deadline, scored index), not by removal order. Empty
    /// when `safe`.
    pub wont_fit: Vec<WontFit>,
}

/// Whole minutes of work left on a task: `round_ties_even(hours * 60)`, so every figure after this
/// is an integer. Clamped at 0: a `progress` over 100 is no work, never negative work that could
/// hide another item's overflow (controller ruling, T3b review). `pressure` and
/// `Task::remaining_hours` are unchanged.
pub fn work_minutes(task: &Task) -> i64 {
    ((task.remaining_hours() * 60.0).round_ties_even() as i64).max(0)
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

/// The deadline an item is held to at moment `m`: its `due` while that is after `m` (an item due
/// exactly at `m` has closed), else `None` (overdue, no window). **This is the one place O6's
/// late-window close goes**: a late item whose window closes after `m` returns that close here.
/// Daily late-penalty steps are out of v1 (ruling b), so only a window's close will count.
fn deadline(task: &Task, m: DateTime) -> Option<DateTime> {
    task.due.filter(|due| *due > m)
}

/// The deadline plan (spec §2). With no moment, or a moment not on `today`, this is `rank`. Otherwise
/// Must do (non-override, slack <= 0) is checked in pressure order against the timeline (step 0);
/// when every dated item keeps `FIT_MARGIN_MINUTES` spare the order is `rank`'s, else dated items go
/// in deadline order (step 1), the fit walk drops what cannot fit (step 2), overdue items are merged
/// into the kept order while they cost no kept item its margin (step 3), and the dropped items
/// (`wont_fit`, by deadline and scored position) go last. The result is refilled into Must do's
/// positions; overrides are inserted as `rank` does. Deterministic, no model.
pub fn plan_at(tasks: &[Task], today: Date, cal: &WeekCalendar, moment: Option<DateTime>) -> DeadlinePlan {
    let Some(m) = moment.filter(|m| m.date() == today) else {
        return DeadlinePlan { order: rank(tasks, today, cal), safe: true, wont_fit: Vec::new() };
    };
    let m = m.with().second(0).subsec_nanosecond(0).build().unwrap_or(m);
    let (overridden, mut result) = pressure_sorted(tasks, today, cal);

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
    let mut wont_fit = Vec::new();
    if !safe {
        let mut dated: Vec<(DateTime, usize)> =
            positions.iter().filter_map(|&i| deadline(&result[i], m).map(|d| (d, i))).collect();
        dated.sort();
        // Step 2, the walk (spec §2.6). Each record is made at the removal, from what is kept then.
        let mut kept: Vec<(DateTime, usize)> = Vec::new();
        let mut removed: Vec<(DateTime, usize, WontFit)> = Vec::new();
        let mut t = 0;
        for &(d, i) in &dated {
            kept.push((d, i));
            t += work_minutes(&result[i]);
            if t <= timeline.before(d) {
                continue;
            }
            let pos = (0..kept.len())
                .max_by_key(|&p| (work_minutes(&result[kept[p].1]), kept[p].0, kept[p].1))
                .unwrap_or(0);
            let (rd, ri) = kept.remove(pos);
            let work = work_minutes(&result[ri]);
            t -= work;
            let slugs_kept: Vec<String> = kept.iter().map(|&(_, k)| result[k].slug.clone()).collect();
            removed.push((rd, ri, WontFit {
                slug: result[ri].slug.clone(),
                deadline: rd,
                work_minutes: work,
                available_minutes: timeline.before(rd),
                prefix: slugs_kept,
                prefix_minutes: t,
                overflower: result[i].slug.clone(),
                overflow_deadline: d,
            }));
        }
        removed.sort_by_key(|(rd, ri, _)| (*rd, *ri));
        let overdue: Vec<usize> =
            positions.iter().copied().filter(|&i| deadline(&result[i], m).is_none()).collect();
        let refill: Vec<Task> = merge_overdue(&result, &kept, overdue, &timeline)
            .into_iter()
            .chain(removed.iter().map(|&(_, i, _)| i))
            .map(|i| result[i].clone())
            .collect();
        for (&position, task) in positions.iter().zip(refill) {
            result[position] = task;
        }
        wont_fit = removed.into_iter().map(|(_, _, w)| w).collect();
    }
    DeadlinePlan { order: insert_overrides(result, overridden), safe, wont_fit }
}

/// Step 3 (spec §2.7). `kept` is the walk's survivors in deadline order; `overdue` is in scored
/// order. From the front, the first overdue item that leaves every remaining kept item its
/// `FIT_MARGIN_MINUTES` goes next; with none, the next kept item. Returns indexes into `items`.
fn merge_overdue(items: &[Task], kept: &[(DateTime, usize)], mut overdue: Vec<usize>, timeline: &Timeline) -> Vec<usize> {
    let mut merged = Vec::new();
    let (mut t, mut next) = (0, 0);
    while next < kept.len() || !overdue.is_empty() {
        let placeable = overdue.iter().position(|&o| {
            let mut ahead = t + work_minutes(&items[o]);
            kept[next..].iter().all(|&(d, k)| {
                ahead += work_minutes(&items[k]);
                ahead <= timeline.before(d) - FIT_MARGIN_MINUTES
            })
        });
        let index = match placeable {
            Some(p) => overdue.remove(p),
            None if next < kept.len() => {
                next += 1;
                kept[next - 1].1
            }
            None => break,
        };
        t += work_minutes(&items[index]);
        merged.push(index);
    }
    merged
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

    // ---- The evening rule's kept tests (R1-R15; R3, R4, R7, R14, R15 retired with the 8 pm rule, Q5). Invented tasks on the vault-s1 week template. ----

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

    // ---- Deadline ordering: the walk and its record (DP11-DP14, DP20). ----

    fn assert_rank(tasks: &[Task], expected: &[&str]) {
        assert_eq!(slugs(&rank(tasks, D, &week())), names(expected), "setup: the rank order");
    }

    /// An item the walk named, with the record it must carry.
    fn wont_fit(slug: &str, deadline: DateTime, work: i64, available: i64, prefix: &[&str], prefix_minutes: i64, overflower: &str, overflow_deadline: DateTime) -> WontFit {
        WontFit {
            slug: slug.into(),
            deadline,
            work_minutes: work,
            available_minutes: available,
            prefix: names(prefix),
            prefix_minutes,
            overflower: overflower.into(),
            overflow_deadline,
        }
    }

    #[test]
    fn the_walk_names_the_longest() {
        let tasks = vec![
            task("x", Some(at(28, 23, 59)), 1.0, 5),
            task("y", Some(at(28, 23, 59)), 2.5, 3),
            task("z", Some(at(28, 23, 59)), 1.5, 1),
        ];
        assert_rank(&tasks, &["x", "y", "z"]);
        let plan = plan_at(&tasks, D, &week(), Some(at(28, 21, 0)));
        assert!(!plan.safe);
        assert_eq!(slugs(&plan.order), names(&["x", "z", "y"]));
        assert_eq!(plan.wont_fit, vec![wont_fit("y", at(28, 23, 59), 150, 179, &["x"], 60, "y", at(28, 23, 59))]);
    }

    #[test]
    fn a_walk_tie_names_the_later_deadline_then_the_lower_pressure() {
        let cal = week();
        // Equal work, different deadlines: 90 + 90 > 179 at the second; the later deadline goes.
        let tasks = vec![task("p-2300", Some(at(28, 23, 0)), 1.5, 3), task("q-2359", Some(at(28, 23, 59)), 1.5, 3)];
        let plan = plan_at(&tasks, D, &cal, Some(at(28, 21, 0)));
        assert!(!plan.safe);
        assert_eq!(plan.wont_fit.iter().map(|w| w.slug.as_str()).collect::<Vec<_>>(), ["q-2359"]);

        // Equal work and deadline: the one later in rank's order goes, whichever that is.
        let mut earlier: Option<Vec<String>> = None;
        for (r_importance, s_importance) in [(5, 1), (1, 5)] {
            let tasks = vec![
                task("r", Some(at(28, 23, 59)), 1.5, r_importance),
                task("s", Some(at(28, 23, 59)), 1.5, s_importance),
            ];
            let ranked = slugs(&rank(&tasks, D, &cal));
            // Guard (T3b review): the two pairs really do rank in opposite orders.
            if let Some(first) = &earlier {
                assert_ne!(&ranked, first, "setup: the importance pairs must give different rank orders");
            }
            earlier = Some(ranked.clone());
            let plan = plan_at(&tasks, D, &cal, Some(at(28, 21, 0)));
            assert!(!plan.safe);
            let named: Vec<String> = plan.wont_fit.iter().map(|w| w.slug.clone()).collect();
            assert_eq!(named, vec![ranked[1].clone()], "rank order {ranked:?}");
        }
    }

    #[test]
    fn an_unsafe_order_can_name_nothing() {
        let tasks = lab_and_homework();
        assert_first(&tasks, "lab");
        let plan = plan_at(&tasks, D, &week(), Some(at(28, 19, 30)));
        assert!(!plan.safe);
        assert_eq!(slugs(&plan.order), names(&["hw", "lab"]));
        assert!(plan.wont_fit.is_empty());
    }

    #[test]
    fn the_prefix_names_a_later_overflower() {
        let cal = week();
        let now = Some(at(28, 21, 0));
        // Y first under rank: X's addition overflows, Y is removed, X comes after it.
        let y_first = vec![task("y", Some(at(28, 23, 59)), 2.5, 5), task("x", Some(at(28, 23, 59)), 1.0, 1)];
        assert_rank(&y_first, &["y", "x"]);
        let plan = plan_at(&y_first, D, &cal, now);
        assert_eq!(plan.wont_fit, vec![wont_fit("y", at(28, 23, 59), 150, 179, &["x"], 60, "x", at(28, 23, 59))]);

        // X first: Y overflows itself.
        let x_first = vec![task("y", Some(at(28, 23, 59)), 2.5, 1), task("x", Some(at(28, 23, 59)), 1.0, 5)];
        assert_rank(&x_first, &["x", "y"]);
        let plan = plan_at(&x_first, D, &cal, now);
        assert_eq!(plan.wont_fit, vec![wont_fit("y", at(28, 23, 59), 150, 179, &["x"], 60, "y", at(28, 23, 59))]);

        // DP21's set without D: B overflows itself at 22:30; C fits alone but K's addition removes it.
        let set = vec![
            task("k", Some(at(28, 23, 59)), 1.0, 3),
            task("b", Some(at(28, 22, 30)), 2.0, 3),
            task("c", Some(at(28, 23, 30)), 2.5, 3),
        ];
        let plan = plan_at(&set, D, &cal, now);
        assert!(!plan.safe);
        assert_eq!(
            plan.wont_fit,
            vec![
                wont_fit("b", at(28, 22, 30), 120, 90, &[], 0, "b", at(28, 22, 30)),
                wont_fit("c", at(28, 23, 30), 150, 150, &["k"], 60, "k", at(28, 23, 59)),
            ]
        );
    }

    #[test]
    fn the_plan_is_deterministic() {
        let cal = week();
        let tasks = vec![
            task("k", Some(at(28, 23, 59)), 1.0, 3),
            task("b", Some(at(28, 22, 30)), 2.0, 4),
            task("c", Some(at(28, 23, 30)), 2.5, 2),
        ];
        let pressures: Vec<f64> = tasks.iter().map(|t| pressure(t, D, &cal)).collect();
        assert!(pressures[0] != pressures[1] && pressures[1] != pressures[2] && pressures[0] != pressures[2], "setup: all keys differ");
        let now = Some(at(28, 21, 0));
        let plan = plan_at(&tasks, D, &cal, now);
        assert_eq!(plan, plan_at(&tasks, D, &cal, now));
        let mut reversed = tasks.clone();
        reversed.reverse();
        assert_eq!(plan, plan_at(&reversed, D, &cal, now));
    }

    #[test]
    fn a_prefix_holds_kept_items_before_and_after_the_removed_one() {
        // At D 21:00 the timeline gives before(22:00)=60, before(23:30)=150, before(23:59)=179.
        // The walk has no margin (spec 2.6). Deadline order: a (30 min, 22:00), c (120 min, 23:30),
        // k (60 min, 23:59).
        //   a: t = 30  <= 60   kept.
        //   c: t = 150 <= 150  kept (exactly full).
        //   k: t = 210 >  179  overflow; the longest kept is c (120), removed.
        // Kept after the removal: a, k (t = 90). So c's prefix is [a, k]: a came before c, k after.
        // (Step 0 is unsafe: 210 > 179 - 30.) The brief's 2.5h c cannot work: 30 + 150 > 150.
        let tasks = vec![
            task("a", Some(at(28, 22, 0)), 0.5, 3),
            task("c", Some(at(28, 23, 30)), 2.0, 3),
            task("k", Some(at(28, 23, 59)), 1.0, 3),
        ];
        let plan = plan_at(&tasks, D, &week(), Some(at(28, 21, 0)));
        assert!(!plan.safe);
        assert_eq!(plan.wont_fit, vec![wont_fit("c", at(28, 23, 30), 120, 150, &["a", "k"], 90, "k", at(28, 23, 59))]);
        assert_eq!(slugs(&plan.order), names(&["a", "k", "c"]));
    }

    #[test]
    fn progress_over_100_counts_as_no_work() {
        // Controller ruling (T3b review): work_minutes clamps at 0, never negative.
        let mut done = task("done", Some(at(28, 23, 59)), 2.0, 3);
        done.progress = 150;
        assert_eq!(work_minutes(&done), 0);

        // Inside a walk it must not hide another item's overflow: 3h = 180 > 179 at 23:59.
        let tasks = vec![done, task("big", Some(at(28, 23, 59)), 3.0, 3)];
        let plan = plan_at(&tasks, D, &week(), Some(at(28, 21, 0)));
        assert!(!plan.safe);
        assert_eq!(plan.wont_fit, vec![wont_fit("big", at(28, 23, 59), 180, 179, &["done"], 0, "big", at(28, 23, 59))]);
    }

    // ---- Deadline ordering: the merge, assembly and the run path (DP15-DP19, DP21). ----

    #[test]
    fn the_lab_leads_until_1929_and_yields_at_1930() {
        let tasks = lab_and_homework();
        let cal = week();
        assert_first(&tasks, "lab");
        for moment in [at(28, 15, 0), at(28, 19, 29)] {
            assert_eq!(plan_at(&tasks, D, &cal, Some(moment)).order, rank(&tasks, D, &cal), "{moment:?}");
        }
        assert_eq!(slugs(&plan_at(&tasks, D, &cal, Some(at(28, 19, 30))).order), names(&["hw", "lab"]));
    }

    #[test]
    fn a_daytime_clash_puts_the_essay_first() {
        let tasks = vec![task("essay", Some(at(28, 23, 59)), 11.0, 3), task("lab", Some(at(20, 23, 59)), 3.0, 3)];
        assert_rank(&tasks, &["lab", "essay"]);
        let plan = plan_at(&tasks, D, &week(), Some(at(28, 9, 0)));
        assert_eq!(slugs(&plan.order), names(&["essay", "lab"]));
        assert!(plan.wont_fit.is_empty());
    }

    #[test]
    fn a_shorter_overdue_item_goes_ahead_of_a_longer_one() {
        let tasks = vec![
            task("long-overdue", Some(at(26, 23, 59)), 2.5, 3),
            task("short-overdue", Some(at(26, 23, 59)), 0.5, 3),
            task("k", Some(at(28, 23, 59)), 1.0, 3),
        ];
        assert_rank(&tasks, &["long-overdue", "short-overdue", "k"]);
        let plan = plan_at(&tasks, D, &week(), Some(at(28, 21, 0)));
        assert_eq!(slugs(&plan.order), names(&["short-overdue", "k", "long-overdue"]));
    }

    #[test]
    fn everything_outside_must_do_keeps_its_index() {
        let mut tasks = lab_and_homework();
        tasks.push(task("later", Some(DateTime::constant(2026, 9, 7, 23, 59, 0, 0)), 1.0, 3));
        tasks.push(task("undated", None, 1.0, 3));
        let cal = week();
        let ranked = rank(&tasks, D, &cal);
        let must: Vec<bool> = ranked.iter().map(|t| slack_days(t, D, &cal, DEFAULT_BUFFER_DAYS) <= 0.0).collect();
        assert_eq!(must.iter().filter(|m| **m).count(), 2, "setup: only the lab and hw are Must do");
        let plan = plan_at(&tasks, D, &cal, Some(at(28, 19, 30)));
        assert!(!plan.safe);
        assert_eq!(plan.order.len(), ranked.len());
        for (i, is_must) in must.iter().enumerate() {
            if !is_must {
                assert_eq!(plan.order[i], ranked[i], "index {i}");
            }
        }
        let mut in_plan: Vec<String> = (0..must.len()).filter(|&i| must[i]).map(|i| plan.order[i].slug.clone()).collect();
        let mut in_rank: Vec<String> = (0..must.len()).filter(|&i| must[i]).map(|i| ranked[i].slug.clone()).collect();
        in_plan.sort();
        in_rank.sort();
        assert_eq!(in_plan, in_rank);
    }

    #[test]
    fn overrides_win_and_are_not_charged() {
        let mut pinned = task("pinned", Some(at(26, 23, 59)), 3.0, 3);
        pinned.rank_override = Some(1);
        let tasks = vec![pinned, task("hw", Some(at(28, 23, 59)), 1.0, 3)];
        let plan = plan_at(&tasks, D, &week(), Some(at(28, 21, 0)));
        assert_eq!(slugs(&plan.order), names(&["pinned", "hw"]));
        assert!(plan.wont_fit.is_empty());
    }

    #[test]
    fn the_merge_runs_before_wont_fit_items() {
        // The walk: b overflows itself (120 > 90); c fits (150 <= 150); k overflows (210 > 179) and c,
        // the longest of c and k, is removed. d (30) fits ahead of k: 30 + 60 <= 179 - 30.
        let tasks = vec![
            task("k", Some(at(28, 23, 59)), 1.0, 3),
            task("b", Some(at(28, 22, 30)), 2.0, 3),
            task("c", Some(at(28, 23, 30)), 2.5, 3),
            task("d", Some(at(26, 23, 59)), 0.5, 3),
        ];
        let plan = plan_at(&tasks, D, &week(), Some(at(28, 21, 0)));
        assert!(!plan.safe);
        assert_eq!(
            plan.wont_fit,
            vec![
                wont_fit("b", at(28, 22, 30), 120, 90, &[], 0, "b", at(28, 22, 30)),
                wont_fit("c", at(28, 23, 30), 150, 150, &["k"], 60, "k", at(28, 23, 59)),
            ]
        );
        assert_eq!(slugs(&plan.order), names(&["d", "k", "b", "c"]));
    }
}
