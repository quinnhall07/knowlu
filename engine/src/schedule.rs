//! Slot arithmetic for the app's scheduler (Knowlu spec §3). Pure: the app's tick thread asks
//! "is a slot owed?" and this module answers from the runner config and the run records it is
//! handed. It never reads the clock and never writes. The console is the only caller.
use std::path::Path;
use jiff::{civil::Date, tz::TimeZone, Span, Timestamp, Zoned};
use crate::runs::RunnerConfig;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SchedulerMode { Script, App }

impl SchedulerMode {
    /// Absent or anything but the exact word `app` is `Script` (spec §3: the key is inert until
    /// phase 2 flips it, and a typo must never start a second runner).
    pub fn parse(s: Option<&str>) -> SchedulerMode { if s == Some("app") { SchedulerMode::App } else { SchedulerMode::Script } }
}

#[derive(Debug, Clone, PartialEq)]
pub struct SlotDue { pub due: Zoned, pub late: bool }

const EARLY_MINUTES: i64 = 5;

fn tz_of(cfg: &RunnerConfig) -> TimeZone { TimeZone::get(&cfg.tz).unwrap_or(TimeZone::UTC) }

fn slots_on(cfg: &RunnerConfig, day: Date) -> Vec<Zoned> {
    let tz = tz_of(cfg);
    let mut out: Vec<Zoned> = cfg.times.iter()
        .filter_map(|t| t.parse::<jiff::civil::Time>().ok())
        .filter_map(|t| day.to_datetime(t).to_zoned(tz.clone()).ok())
        .collect();
    out.sort();
    out
}

fn settled(due: &Zoned, window_end: &Zoned, starts: &[Timestamp]) -> bool {
    let lo = due.checked_sub(Span::new().minutes(EARLY_MINUTES)).map(|z| z.timestamp()).unwrap_or_else(|_| due.timestamp());
    let hi = window_end.timestamp();
    starts.iter().any(|s| *s >= lo && *s < hi)
}

/// The earliest slot of `now`'s day (in `cfg.tz`) that is due (`<= now`) and unsettled — no
/// `start` in `[due − 5 min, next slot or end of day)`. `late` is whether `now` is past the
/// runner's grace window; a late slot is still returned, never skipped (the 2026-08-26 lesson:
/// do not act on a late run, but do not pretend it is not owed either).
pub fn due_slot(cfg: &RunnerConfig, now: &Zoned, starts: &[Timestamp]) -> Option<SlotDue> {
    let now = now.with_time_zone(tz_of(cfg));
    let slots = slots_on(cfg, now.date());
    let end_of_day = now.date().tomorrow().ok()?.to_datetime(jiff::civil::Time::midnight()).to_zoned(tz_of(cfg)).ok()?;
    for (i, due) in slots.iter().enumerate() {
        if *due > now { break; }
        let window_end = slots.get(i + 1).cloned().unwrap_or_else(|| end_of_day.clone());
        if settled(due, &window_end, starts) { continue; }
        let grace_end = due.checked_add(Span::new().minutes(cfg.grace_minutes)).unwrap_or_else(|_| due.clone());
        return Some(SlotDue { due: due.clone(), late: now > grace_end });
    }
    None
}

/// The count of slots in `(from, to]` with no settling start — the quit gap the app's status
/// line reports after a long time away.
pub fn missed_slots(cfg: &RunnerConfig, from: &Zoned, to: &Zoned, starts: &[Timestamp]) -> usize {
    let tz = tz_of(cfg);
    let (from, to) = (from.with_time_zone(tz.clone()), to.with_time_zone(tz.clone()));
    let mut day = from.date();
    let mut missed = 0;
    while day <= to.date() {
        let slots = slots_on(cfg, day);
        let end_of_day = day.tomorrow().ok().and_then(|d| d.to_datetime(jiff::civil::Time::midnight()).to_zoned(tz.clone()).ok());
        for (i, due) in slots.iter().enumerate() {
            if *due <= from || *due > to { continue; }
            let window_end = slots.get(i + 1).cloned().or_else(|| end_of_day.clone()).unwrap_or_else(|| due.clone());
            if !settled(due, &window_end, starts) { missed += 1; }
        }
        day = match day.tomorrow() { Ok(d) => d, Err(_) => break };
    }
    missed
}

/// `start` records for `runner` since `since_date`, as timestamps. Reads `state/runs/` through
/// the runs ledger; never writes.
pub fn run_starts(vault: &Path, runner: &str, since_date: Date) -> Vec<Timestamp> {
    let mut runs = crate::runs::Runs::new(vault);
    let since = since_date.to_string();
    runs.read(Some(&since)).iter()
        .filter(|r| r.get("phase").and_then(|v| v.as_str()) == Some("start") && r.get("runner").and_then(|v| v.as_str()) == Some(runner))
        .filter_map(|r| r.get("ts").and_then(|v| v.as_str()).and_then(|s| s.parse::<Timestamp>().ok()))
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::runs::RunnerConfig;
    fn cfg() -> RunnerConfig { RunnerConfig { name: "local".into(), times: vec!["12:00".into(), "18:00".into()], tz: "America/Chicago".into(), grace_minutes: 20 } }
    fn at(s: &str) -> jiff::Zoned { s.parse::<jiff::civil::DateTime>().unwrap().to_zoned(jiff::tz::TimeZone::get("America/Chicago").unwrap()).unwrap() }
    fn ts(s: &str) -> jiff::Timestamp { at(s).timestamp() }

    #[test]
    fn nothing_is_due_before_the_first_slot() { assert!(due_slot(&cfg(), &at("2026-09-04T11:59"), &[]).is_none()); }

    #[test]
    fn the_first_slot_is_due_on_time_inside_grace_and_late_after() {
        let d = due_slot(&cfg(), &at("2026-09-04T12:05"), &[]).unwrap();
        assert_eq!(d.due, at("2026-09-04T12:00")); assert!(!d.late);
        let d = due_slot(&cfg(), &at("2026-09-04T12:21"), &[]).unwrap();
        assert!(d.late, "past grace is late, never skipped");
    }

    #[test]
    fn a_start_record_in_the_window_settles_the_slot() {
        let started = [ts("2026-09-04T12:03")];
        assert!(due_slot(&cfg(), &at("2026-09-04T13:00"), &started).is_none());
        let d = due_slot(&cfg(), &at("2026-09-04T19:34"), &started).unwrap();
        assert_eq!(d.due, at("2026-09-04T18:00"), "the 18:00 slot is still owed after wake");
    }

    #[test]
    fn a_start_five_minutes_early_counts_and_one_from_yesterday_does_not() {
        assert!(due_slot(&cfg(), &at("2026-09-04T12:30"), &[ts("2026-09-04T11:56")]).is_none());
        assert!(due_slot(&cfg(), &at("2026-09-04T12:30"), &[ts("2026-09-03T12:01")]).is_some());
    }

    #[test]
    fn the_earliest_owed_slot_wins_when_two_are_owed() {
        let d = due_slot(&cfg(), &at("2026-09-04T19:00"), &[]).unwrap();
        assert_eq!(d.due, at("2026-09-04T12:00"));
    }

    #[test]
    fn missed_slots_counts_the_quit_gap() {
        assert_eq!(missed_slots(&cfg(), &at("2026-09-03T13:00"), &at("2026-09-04T19:00"), &[ts("2026-09-04T12:10")]), 2, "09-03 18:00 and 09-04 18:00");
    }

    /// DST (Knowlu plan 2, Task 5 — the pin plan 1's final review wanted before `scheduler: app`
    /// ever flips): America/Chicago falls back on 2026-11-01 (01:00 happens twice) and springs
    /// forward on 2026-03-08 (02:00–03:00 does not happen at all). Slots at 12:00/18:00 are
    /// unaffected; **01:30 lands in the repeated hour and 02:30 lands in the gap**, and each must
    /// fire exactly once per day whichever instant `slots_on` resolves it to. The gap slot is the
    /// one that matters: `slots_on` builds each slot with `to_zoned(…).ok()`, and a filtered-out
    /// `None` there would drop the slot silently — a slot the runner simply never runs, which is
    /// the F4 violation (a slot is never skipped) wearing a timezone costume. Walking the whole
    /// day a minute at a time is the tick thread's own cadence, so a double fire in the repeated
    /// hour would show here too.
    #[test]
    fn dst_days_fire_each_slot_exactly_once() {
        let cfg = RunnerConfig { name: "local".into(), times: vec!["01:30".into(), "02:30".into(), "12:00".into(), "18:00".into()], tz: "America/Chicago".into(), grace_minutes: 20 };
        for day in ["2026-11-01", "2026-03-08"] {
            let mut starts: Vec<Timestamp> = Vec::new();
            let mut fired = 0;
            let mut now: Zoned = format!("{day}T00:00[America/Chicago]").parse().unwrap();
            let end: Zoned = format!("{day}T23:59[America/Chicago]").parse().unwrap();
            while now < end {
                if let Some(d) = due_slot(&cfg, &now, &starts) { fired += 1; starts.push(d.due.timestamp()); }
                now = now.checked_add(Span::new().minutes(1)).unwrap();
            }
            assert_eq!(fired, 4, "{day}: every configured slot exactly once, the gap slot included");
            let from: Zoned = format!("{day}T00:00[America/Chicago]").parse().unwrap();
            assert_eq!(missed_slots(&cfg, &from, &end, &[]), 4, "{day}: four missed with no starts");
        }
    }

    #[test]
    fn scheduler_mode_defaults_to_script() {
        assert_eq!(SchedulerMode::parse(None), SchedulerMode::Script);
        assert_eq!(SchedulerMode::parse(Some("script")), SchedulerMode::Script);
        assert_eq!(SchedulerMode::parse(Some("App")), SchedulerMode::Script, "exactly `app`, nothing else, turns the app scheduler on");
        assert_eq!(SchedulerMode::parse(Some("app")), SchedulerMode::App);
    }
}
