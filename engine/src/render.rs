//! Port of `engine/render.py` — `state/today.md`, the one page Quinn reads.
//!
//! This module is where the port stops being about behaviour and starts being about **bytes**.
//! `tests/fixtures/golden-today-s1.md` is checked in, and the oracle compares this module's output
//! to it character for character. Nothing here may be tidied.
//!
//! # Three things that look like mistakes and are not
//!
//! 1. **The date formats are inconsistent by design.** One page carries `Friday, August 28`,
//!    `Wed 8/26`, `Mon 08-31` (inside a task's own title, not produced here) and `9/25 → 9/22`.
//!    Each format string is ported where it is used. Unifying them would be an improvement and a
//!    regression.
//! 2. **Three different dashes.** `—` (em dash, U+2014) separates a title from its detail, `–`
//!    (en dash, U+2013) joins a schedule time range, and `−` (U+2212 MINUS SIGN, not a hyphen)
//!    subtracts calendar time from template capacity. They are not interchangeable.
//! 3. **Hours are rounded at three different precisions.** Capacity keeps two decimals through
//!    Python's `str(float)` (`8.25h`, and `8.0h` — never `8h`); remaining work prints one
//!    (`2.8h left`); a recurring commitment prints its configured value verbatim (`0.75h`).
//!    Match each site; do not pick a global rule.
//!
//! # Must-do renders unconditionally
//!
//! Slack ≤ 0 puts a task in Must do before any capacity fitting happens. Duration never decides
//! visibility — deadline feasibility does. A task too big for the day still appears, followed by
//! the line that says so.

use std::path::Path;

use jiff::civil::{Date, DateTime};
use serde_yaml_ng::Value;

use crate::approvals::{as_note_datetime, resolve_amend_target, AmendmentEntry, ApprovalsResult};
use crate::models::{split_frontmatter, Task};
use crate::planning::PlanningConfig;
use crate::pystr;
use crate::ranking::partition_must_do;
use crate::scheduling::{slack_days, start_by, DEFAULT_BUFFER_DAYS};
use crate::weekcal::WeekCalendar;

/// The slice of a discovered event the renderer reads.
///
/// Python passes `engine.events.DiscoveredEvent` straight through; that module lands in wave 4,
/// and inventing its other fourteen fields now would be guessing. These five are exactly what
/// `_coming_up` touches, and `location`/`organizer` are `String` rather than `Option` because
/// Python tests them for *falsiness* — `None` and `""` behave identically there.
#[derive(Debug, Clone, PartialEq)]
pub struct ComingUpEvent {
    pub uid: String,
    pub title: String,
    pub start: DateTime,
    pub location: String,
    pub organizer: String,
}

/// Python's `str(float)` as an f-string interpolates it.
///
/// Rust's `{:?}` for `f64` is the same shortest-round-tripping form as Python's `repr`, and, like
/// it, keeps the decimal point on an integral value — `8.0`, never `8`. The golden file's
/// `— 2.0h (of 2.5h left)` depends on that: `{}` would render `2h` and silently change the page.
///
/// The two forms diverge at the magnitudes where each switches to exponent notation. Unreachable
/// for hours in a day; noted so nobody uses this as a general float formatter.
pub fn py_float(value: f64) -> String {
    format!("{value:?}")
}

/// Python's `round(x, 2)` — half to EVEN, matching `ranking::round2`.
pub fn round2(x: f64) -> f64 {
    (x * 100.0).round_ties_even() / 100.0
}

pub fn status_word(runway: f64) -> &'static str {
    if runway < 0.0 {
        "behind"
    } else if runway <= 1.0 {
        "tight"
    } else if runway <= 4.0 {
        "steady"
    } else {
        "ahead"
    }
}

/// `month/day`, unpadded — `9/25`, never `09/25`.
pub fn short_date(value: Date) -> String {
    format!("{}/{}", value.month(), value.day())
}

/// Flatten a value that is spliced straight into a markdown bullet.
///
/// Titles reach `today.md` from a note's frontmatter or a proposal, neither of which is
/// single-line checked. A second line escapes the list, and one beginning with `##` injects a
/// heading that reorganises the page.
///
/// `pystr::is_python_space` rather than `char::is_whitespace`: Python's `str.split()` treats
/// `\x1c`–`\x1f` as whitespace and Unicode's `White_Space` property does not.
pub fn one_line(text: &str) -> String {
    text.split(|c| pystr::is_python_space(c))
        .filter(|part| !part.is_empty())
        .collect::<Vec<_>>()
        .join(" ")
}

/// Render a deadline move so the words match the size of the change.
///
/// Two guards on top of the day count, both there because a bare day count lies:
/// a cross-year move (`1/5 → 10/5 (92 days EARLIER)` reads as a nine-month postponement) gains
/// years; a move of a day or less is dominated by the clock (00:30 to the previous 23:00 is ninety
/// minutes) and gains times.
pub fn due_move(before: DateTime, after: DateTime) -> String {
    let gap = crate::scheduling::days_between(before.date(), after.date());
    let mut left = short_date(before.date());
    let mut right = short_date(after.date());
    if before.year() != after.year() {
        left = format!("{left}/{}", before.year());
        right = format!("{right}/{}", after.year());
    }
    if gap.abs() <= 1 {
        left = format!("{left} {}", before.strftime("%H:%M"));
        right = format!("{right} {}", after.strftime("%H:%M"));
    }
    if gap == 0 {
        if after < before {
            return format!("{left} → {right} (EARLIER same day)");
        }
        if after > before {
            return format!("{left} → {right} (later same day)");
        }
        return format!("{left} → {right} (no change)");
    }
    let unit = if gap.abs() == 1 { "day" } else { "days" };
    let direction = if gap > 0 { "EARLIER" } else { "later" };
    format!("{left} → {right} ({} {unit} {direction})", gap.abs())
}

/// The `⚠ PENDING AMENDMENT` block: three or four lines under Must do.
///
/// **Containment is enforced here as well as at execution.** The renderer reads the target note
/// for its title, so a proposal naming `../outside.md` would otherwise render that file's title
/// into `today.md`. Out of bounds, unreadable, or malformed frontmatter → the proposal's own title.
///
/// **The day count is measured from what the note says now, not from what the proposal claims it
/// said.** The target was already opened for its title, so the real value is in hand. When the two
/// disagree the proposal will be refused as stale on approval, which is worth saying — a confident
/// day count computed from a value the note no longer holds is a false deadline alarm at the top
/// of the page.
fn amendment_block(entry: &AmendmentEntry, vault: Option<&Path>) -> Vec<String> {
    let mut title = if entry.title.is_empty() { entry.proposal.clone() } else { entry.title.clone() };
    let mut target_due: Option<DateTime> = None;

    if let (Some(vault), false) = (vault, entry.target.is_empty()) {
        let raw = Value::String(entry.target.clone());
        if let Some(target) = resolve_amend_target(vault, Some(&raw)) {
            // Unreadable file or non-mapping frontmatter → keep the proposal's title. Python
            // swallows OSError/ValueError/YAMLError/AttributeError/TypeError here for the same
            // reason: today.md must render.
            if let Ok(text) = pystr::read_text(&target) {
                if let Ok((meta, _)) = split_frontmatter(&text) {
                    if let Some(found) = crate::yaml::get(&meta, "title").and_then(crate::yaml::text)
                    {
                        if !found.is_empty() {
                            title = found;
                        }
                    }
                    target_due = as_note_datetime(crate::yaml::get(&meta, "due"));
                }
            }
        }
    }

    let mut lines = vec![
        format!("- ⚠ **PENDING AMENDMENT** ({}d unactioned)", entry.age_days),
        format!("  - {}", one_line(&title)),
    ];

    if let Some(Value::Mapping(due)) = crate::yaml::get(&entry.changes, "due") {
        // as_note_datetime, not coerce_datetime: the same-day branch compares full datetimes, so
        // a mixed aware/naive pair would raise TypeError in Python and lose today.md.
        let stated = as_note_datetime(crate::yaml::get(due, "from"));
        let after = as_note_datetime(crate::yaml::get(due, "to"));
        let before = target_due.or(stated);
        if let (Some(before), Some(after)) = (before, after) {
            let note = match (target_due, stated) {
                (Some(t), Some(s)) if t != s => " — stale: the note now reads a different date",
                _ => "",
            };
            lines.push(format!("  - due {}{note}", due_move(before, after)));
        }
    }

    lines.push(format!("  - → `approvals/{}.md`", entry.proposal));
    lines
}

/// The next five relevant events — context, not work.
///
/// The cutoff is the run's date, and borrows the clock only when the two agree. `generated_at` is
/// wall-clock time: taken on its own it would hide every event of a fixed-date run once real time
/// drifted past them, so the same vault and the same `today` would render differently depending on
/// when the run happened. The engine is deterministic; this is where that rule is easiest to break.
///
/// Split out of `coming_up`'s selection half so `surface` can read the same five events, in the
/// same order, without duplicating the cutoff/sort/truncate rule (spec §4.2).
pub fn upcoming(
    events: &[ComingUpEvent],
    today: Date,
    generated_at: Option<DateTime>,
) -> Vec<ComingUpEvent> {
    if events.is_empty() {
        return Vec::new();
    }
    let cutoff = match generated_at {
        Some(stamp) if stamp.date() == today => stamp,
        _ => today.to_datetime(jiff::civil::Time::midnight()),
    };
    let mut picked: Vec<&ComingUpEvent> =
        events.iter().filter(|e| e.start >= cutoff).collect();
    picked.sort_by(|a, b| (a.start, &a.uid).cmp(&(b.start, &b.uid)));
    picked.truncate(5);
    picked.into_iter().cloned().collect()
}

/// Renders `upcoming`'s picks as the "## Coming up" block, or nothing at all when it picks none.
fn coming_up(
    events: &[ComingUpEvent],
    today: Date,
    generated_at: Option<DateTime>,
) -> Vec<String> {
    let picked = upcoming(events, today, generated_at);
    if picked.is_empty() {
        return Vec::new();
    }
    let mut lines = vec![String::new(), "## Coming up".to_string(), String::new()];
    for event in picked {
        let detail = [event.location.as_str(), event.organizer.as_str()]
            .into_iter()
            .filter(|b| !b.is_empty())
            .collect::<Vec<_>>()
            .join(" · ");
        let stamp = format!(
            "{} {}/{} {}",
            event.start.strftime("%a"),
            event.start.month(),
            event.start.day(),
            event.start.strftime("%H:%M"),
        );
        let mut line = format!("- {stamp} · {}", event.title);
        if !detail.is_empty() {
            line.push_str(&format!(" · {detail}"));
        }
        lines.push(line);
    }
    lines
}

/// Everything `render_today` needs beyond the ranking. Python spells these as keyword arguments
/// with defaults; a struct with `Default` is the same call surface without eight positional
/// `None`s at every call site.
#[derive(Default)]
pub struct RenderOpts<'a> {
    pub runner: Option<&'a str>,
    pub generated_at: Option<DateTime>,
    pub planning: Option<&'a PlanningConfig>,
    pub approvals: Option<&'a ApprovalsResult>,
    pub vault: Option<&'a Path>,
    pub events: &'a [ComingUpEvent],
}

/// The capacity sentence and the three figures behind it. THE formatter — `render_today` calls it,
/// and so does `surface::verdict`/`meter`, so the page and `today.md` cannot disagree about this
/// sentence (spec §4.2). Moved verbatim from the inline block in `render_today`; the separator is
/// U+2212 MINUS SIGN, all three figures are `round2`, and when the calendar takes less than 0.005h
/// the sentence collapses to the short form with no parenthetical at all.
#[derive(Debug, Clone, PartialEq)]
pub struct Capacity {
    pub text: String,
    pub capacity: f64,
    pub template: f64,
    pub calendar: f64,
}

/// The runway in days: the minimum slack over dated tasks, seeded with `NO_DUE_DATE_SLACK` —
/// exactly as the page's status line has always computed it (extracted verbatim from the fold
/// below; plan-1 ledger L92). ONE implementation — `surface` calls this instead of keeping a
/// private twin.
pub fn runway_days(tasks: &[Task], today: Date, cal: &WeekCalendar) -> f64 {
    let slacks: Vec<f64> = tasks
        .iter()
        .filter(|t| t.due.is_some())
        .map(|t| slack_days(t, today, cal, DEFAULT_BUFFER_DAYS))
        .collect();
    // Python's `min` over floats; no NaN reaches here (slack_days is arithmetic on day counts).
    slacks
        .iter()
        .copied()
        .fold(None::<f64>, |acc, s| Some(match acc {
            Some(m) if m <= s => m,
            _ => s,
        }))
        .unwrap_or(crate::scheduling::NO_DUE_DATE_SLACK)
}

pub fn capacity_breakdown(cal: &WeekCalendar, today: Date) -> Capacity {
    let capacity = round2(cal.capacity(today));
    let template = round2(cal.template_capacity(today));
    let calendar = round2(template - capacity);
    // 0.005 rather than 0.0: both sides are already rounded to two places, so anything smaller is
    // float noise and would print `(template 8.17h − 0.0h calendar)` on a day with no events.
    let text = if template - capacity > 0.005 {
        format!(
            "Capacity today: {}h (template {}h − {}h calendar)",
            py_float(capacity),
            py_float(template),
            py_float(calendar)
        )
    } else {
        format!("Capacity today: {}h", py_float(capacity))
    };
    Capacity { text, capacity, template, calendar: if template - capacity > 0.005 { calendar } else { 0.0 } }
}

/// Render `state/today.md`.
///
/// The returned string is LF-joined with a single trailing newline, exactly as Python's
/// `"\n".join(lines) + "\n"`. **It is the caller's job to write it through `pystr::write_text`**,
/// which is where the CRLF translation Python does implicitly in `Path.write_text` happens.
pub fn render_today(
    ranked: &[Task],
    designated: &[(Task, f64)],
    today: Date,
    cal: &WeekCalendar,
    opts: RenderOpts<'_>,
) -> String {
    let runner = opts.runner.unwrap_or("manual");
    let stamp = opts.generated_at.unwrap_or_else(|| jiff::Zoned::now().datetime());
    let footer = format!("*Generated {} by {runner}*", stamp.strftime("%Y-%m-%d %H:%M"));

    let mut lines: Vec<String> = vec![
        format!("# Today — {} {}", today.strftime("%A, %B"), today.day()),
        String::new(),
    ];

    let capacity_info = capacity_breakdown(cal, today);
    let capacity = capacity_info.capacity;
    let capacity_text = capacity_info.text;

    let mut approvals_line: Option<String> = None;
    if let Some(approvals) = opts.approvals {
        if approvals.pending != 0 || approvals.awaiting_calendar != 0 {
            if approvals.pending != 0 {
                let mut line = format!(
                    "**Approvals: {} pending** (oldest {}d)",
                    approvals.pending, approvals.oldest_pending_days
                );
                if approvals.awaiting_calendar != 0 {
                    line.push_str(&format!(
                        " · {} approved awaiting calendar pass",
                        approvals.awaiting_calendar
                    ));
                }
                approvals_line = Some(line);
            } else {
                approvals_line = Some(format!(
                    "**Approvals: {} approved** awaiting calendar pass",
                    approvals.awaiting_calendar
                ));
            }
        }
        if approvals.events_in_digest != 0 {
            let events_text =
                format!("**Events: {} in today's digest**", approvals.events_in_digest);
            approvals_line = Some(match approvals_line {
                Some(existing) => format!("{existing} · {events_text}"),
                None => events_text,
            });
        }
    }

    let urgent: Vec<&AmendmentEntry> = opts
        .approvals
        .map(|a| a.amendments_pending.iter().filter(|e| e.urgency == "increases").collect())
        .unwrap_or_default();

    if ranked.is_empty() {
        lines.push("Nothing active.".to_string());
        lines.push(String::new());
        if let Some(line) = &approvals_line {
            lines.push(line.clone());
            lines.push(String::new());
        }
        // An urgent amendment must surface here too. An empty vault is exactly when Quinn is
        // least likely to be looking at the page.
        if !urgent.is_empty() {
            lines.push("## Must do".to_string());
            lines.push(String::new());
            for entry in &urgent {
                lines.extend(amendment_block(entry, opts.vault));
            }
            lines.push(String::new());
        }
        lines.extend(coming_up(opts.events, today, opts.generated_at));
        lines.push(String::new());
        lines.push(capacity_text);
        lines.push(String::new());
        lines.push(footer);
        return lines.join("\n") + "\n";
    }

    let runway = runway_days(ranked, today, cal);

    lines.push(format!(
        "**Runway: {:.0} days ({})** · {capacity_text} · {} active",
        runway,
        status_word(runway),
        ranked.len(),
    ));
    lines.push(String::new());

    if let Some(line) = &approvals_line {
        lines.push(line.clone());
        lines.push(String::new());
    }

    let calendar_events = cal.events_on(today);
    if !calendar_events.is_empty() {
        lines.push("## Schedule".to_string());
        lines.push(String::new());
        for event in calendar_events {
            if event.all_day {
                lines.push(format!("- (all day) {}", event.title));
            } else {
                lines.push(format!(
                    "- {}–{} {}",
                    event.start.strftime("%H:%M"),
                    event.end.strftime("%H:%M"),
                    event.title
                ));
            }
        }
        lines.push(String::new());
    }

    lines.push("## Must do".to_string());
    lines.push(String::new());
    let (must, _) = partition_must_do(ranked, today, cal);
    if !must.is_empty() {
        for task in &must {
            let due = match task.due {
                Some(d) => format!("{} {}", d.strftime("%a"), short_date(d.date())),
                None => "no due date".to_string(),
            };
            lines.push(format!(
                "- [ ] **{}** — due {due} · {:.1}h left",
                task.title,
                task.remaining_hours()
            ));
            // An obligation the routine flagged as colliding with a class, work block, or
            // commitment says so here — being expected in two places at once is only useful
            // information if it reaches the page Quinn reads.
            let conflict = task.conflicts_with.as_deref().unwrap_or("");
            let conflict = pystr::strip(conflict);
            if !conflict.is_empty() {
                lines.push(format!("  ⚠ conflicts with {conflict}"));
            }
        }
        let must_hours: f64 = must.iter().map(|t| t.remaining_hours()).sum();
        if must_hours > capacity {
            lines.push(format!(
                "- ⚠ Must-do work ({must_hours:.1}h) exceeds today's capacity ({}h). \
                 Something has to give — decide what, deliberately.",
                py_float(capacity)
            ));
        }
    }
    for entry in &urgent {
        lines.extend(amendment_block(entry, opts.vault));
    }
    if must.is_empty() && urgent.is_empty() {
        lines.push("- Nothing is at risk today.".to_string());
    }

    lines.push(String::new());
    lines.push("## Recommended".to_string());
    lines.push(String::new());
    let recurring = opts.planning.map(|p| p.recurring_for(today)).unwrap_or_default();
    for item in &recurring {
        lines.push(format!(
            "- [ ] **{}** — {}h · recurring",
            item.name,
            py_float(item.hours)
        ));
    }
    if !designated.is_empty() {
        for (task, hours) in designated {
            let left = round2(task.remaining_hours());
            if left - hours > 0.005 {
                lines.push(format!(
                    "- [ ] **{}** — {}h (of {left:.1}h left)",
                    task.title,
                    py_float(*hours)
                ));
            } else {
                lines.push(format!("- [ ] **{}** — {}h", task.title, py_float(*hours)));
            }
        }
    } else if recurring.is_empty() {
        lines.push("- No spare capacity today.".to_string());
    }

    lines.push(String::new());
    lines.push("## Everything active".to_string());
    lines.push(String::new());
    for (index, task) in ranked.iter().enumerate() {
        let due = match task.due {
            Some(d) => format!("{} {}", d.strftime("%a"), short_date(d.date())),
            None => "no due date".to_string(),
        };
        let begin_text = match start_by(task, cal, DEFAULT_BUFFER_DAYS) {
            Some(begin) => format!("start by {}", short_date(begin)),
            None => "unscheduled".to_string(),
        };
        let slack = slack_days(task, today, cal, DEFAULT_BUFFER_DAYS);
        let slack_text = if slack < 0.0 {
            "OVERDUE START".to_string()
        } else {
            format!("{slack:.0}d slack")
        };
        lines.push(format!(
            "{}. {} — due {due} · {begin_text} · {slack_text} · {:.1}h left",
            index + 1,
            task.title,
            task.remaining_hours(),
        ));
    }

    lines.extend(coming_up(opts.events, today, opts.generated_at));
    lines.push(String::new());
    lines.push(footer);
    lines.join("\n") + "\n"
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::planning::Recurring;
    use crate::ranking::{designate_today, rank};
    use crate::weekcal::CalEvent;
    use serde_yaml_ng::Mapping;
    use std::path::PathBuf;

    /// `tests/test_render.py` renders against the LIVE `config/week_template.yaml`, so these do
    /// too — the Monday capacity it asserts (`8.17h`) is a property of that file.
    const TODAY: Date = Date::constant(2026, 9, 7);

    fn cal() -> WeekCalendar {
        WeekCalendar::from_file(Path::new("tests/fixtures/vault-s1/config/week_template.yaml"), Vec::new())
    }

    /// An empty week template: `test_render_events.py` builds one so capacity is the full day.
    fn open_cal() -> WeekCalendar {
        WeekCalendar::new(
            &crate::yaml::mapping_of(
                "day_start: '08:00'\nday_end: '18:00'\nclasses:\n  mon: []\n  tue: []\n  wed: []\n  thu: []\n  fri: []\n  sat: []\n  sun: []\n",
            ),
            Vec::new(),
        )
    }

    /// Python's `Task(...)` dataclass defaults, spelled once.
    fn task(slug: &str, title: &str, due: Option<DateTime>, effort_hours: f64) -> Task {
        Task {
            slug: slug.to_string(),
            title: title.to_string(),
            due,
            effort_hours,
            importance: 3,
            status: "active".to_string(),
            progress: 0,
            course: None,
            domain: "school".to_string(),
            rank_override: None,
            slice_hours: None,
            conflicts_with: None,
        }
    }

    fn dt(y: i16, m: i8, d: i8, h: i8, mi: i8) -> DateTime {
        DateTime::constant(y, m, d, h, mi, 0, 0)
    }

    fn between<'a>(text: &'a str, start: &str, end: &str) -> &'a str {
        let from = text.find(start).expect("section start") + start.len();
        let rest = &text[from..];
        match rest.find(end) {
            Some(to) => &rest[..to],
            None => rest,
        }
    }

    fn must_do(text: &str) -> &str {
        between(text, "## Must do", "## Recommended")
    }

    fn render(ranked: &[Task], designated: &[(Task, f64)], cal: &WeekCalendar) -> String {
        render_today(ranked, designated, TODAY, cal, RenderOpts::default())
    }

    // -----------------------------------------------------------------------------------------
    // Knowlu plan 1, Task 1: the runway fold, extracted to a single implementation.
    // -----------------------------------------------------------------------------------------

    #[test]
    fn runway_days_is_the_single_implementation_the_page_uses() {
        let tasks = [
            task("a", "PH 106 set 4", Some(dt(2026, 9, 11, 23, 59)), 3.0),
            task("b", "German vocab", Some(dt(2026, 9, 8, 23, 59)), 0.5),
        ];
        let ranked = rank(&tasks, TODAY, &cal());
        let r = runway_days(&ranked, TODAY, &cal());
        assert!(r.is_finite());
        assert_eq!(
            runway_days(&[], TODAY, &cal()),
            crate::scheduling::NO_DUE_DATE_SLACK,
            "an empty vault seeds the fold the way render always did"
        );
    }

    // -----------------------------------------------------------------------
    // Shape of the page
    // -----------------------------------------------------------------------

    #[test]
    fn must_do_comes_before_recommended_which_comes_before_everything_active() {
        let tasks = [
            task("a", "PH 106 set 4", Some(dt(2026, 9, 11, 23, 59)), 3.0),
            task("b", "German vocab", Some(dt(2026, 9, 8, 23, 59)), 0.5),
        ];
        let ranked = rank(&tasks, TODAY, &cal());
        let designated = designate_today(&ranked, TODAY, &cal(), None);
        let text = render(&ranked, &designated, &cal());

        assert!(text.contains("# Today"));
        assert!(text.contains("Runway:"));
        assert!(text.contains("8.17h"), "Monday capacity: {text}");
        let must_at = text.find("## Must do").unwrap();
        let rec_at = text.find("## Recommended").unwrap();
        let all_at = text.find("## Everything active").unwrap();
        assert!(must_at < rec_at && rec_at < all_at);
        // German vocab is due tomorrow (zero slack) so it is must-do; PH 106 has slack, so it
        // is a recommendation.
        let vocab = text.find("German vocab").unwrap();
        let ph = text.find("PH 106 set 4").unwrap();
        assert!(must_at < vocab && vocab < rec_at);
        assert!(rec_at < ph && ph < all_at);
    }

    /// Must-do renders unconditionally; the page then says the day does not fit.
    #[test]
    fn a_must_do_bigger_than_the_day_still_renders_and_says_so() {
        let huge = [task("h", "Huge lab", Some(dt(2026, 9, 8, 23, 59)), 20.0)];
        let ranked = rank(&huge, TODAY, &cal());
        let designated = designate_today(&ranked, TODAY, &cal(), None);
        let text = render(&ranked, &designated, &cal());
        assert!(text.contains("Huge lab"));
        assert!(text.contains("exceeds today's capacity"), "{text}");
    }

    #[test]
    fn an_empty_vault_renders_nothing_active() {
        assert!(render(&[], &[], &cal()).contains("Nothing active"));
    }

    #[test]
    fn the_footer_names_the_runner_and_the_wall_clock() {
        let text = render_today(
            &[],
            &[],
            TODAY,
            &cal(),
            RenderOpts {
                runner: Some("cloud"),
                generated_at: Some(dt(2026, 9, 7, 8, 3)),
                ..Default::default()
            },
        );
        assert!(text.contains("*Generated 2026-09-07 08:03 by cloud*"), "{text}");
    }

    // -----------------------------------------------------------------------
    // Capacity — two decimals, `str(float)`, and a MINUS SIGN
    // -----------------------------------------------------------------------

    #[test]
    fn a_calendar_booking_shows_the_schedule_and_the_capacity_breakdown() {
        let booking = CalEvent {
            title: "Client call".to_string(),
            start: dt(2026, 9, 7, 9, 0),
            end: dt(2026, 9, 7, 9, 30),
            all_day: false,
        };
        let cal = WeekCalendar::from_file(Path::new("tests/fixtures/vault-s1/config/week_template.yaml"), vec![booking]);
        let tasks = [task("a", "Essay", Some(dt(2026, 9, 25, 23, 59)), 2.0)];
        let ranked = rank(&tasks, TODAY, &cal);
        let designated = designate_today(&ranked, TODAY, &cal, None);
        let text = render(&ranked, &designated, &cal);

        // U+2212 MINUS SIGN between the two numbers, not a hyphen.
        assert!(
            text.contains("Capacity today: 7.67h (template 8.17h \u{2212} 0.5h calendar)"),
            "{text}"
        );
        // U+2013 EN DASH inside the time range.
        assert!(text.contains("- 09:00\u{2013}09:30 Client call"), "{text}");
        assert!(text.find("## Schedule").unwrap() < text.find("## Must do").unwrap());
    }

    #[test]
    fn no_calendar_events_means_no_schedule_and_no_breakdown() {
        let tasks = [task("a", "Essay", Some(dt(2026, 9, 25, 23, 59)), 2.0)];
        let ranked = rank(&tasks, TODAY, &cal());
        let designated = designate_today(&ranked, TODAY, &cal(), None);
        let text = render(&ranked, &designated, &cal());
        assert!(!text.contains("## Schedule"));
        assert!(!text.contains("template"));
    }

    /// Python interpolates a float with `str()`, which keeps the decimal point on a whole number.
    /// `{}` in Rust renders `8` where Python renders `8.0`, which would change the page.
    #[test]
    fn py_float_keeps_the_decimal_point_python_keeps() {
        assert_eq!(py_float(8.0), "8.0");
        assert_eq!(py_float(8.25), "8.25");
        assert_eq!(py_float(0.75), "0.75");
        assert_eq!(py_float(7.67), "7.67");
        assert_eq!(py_float(2.0), "2.0");
    }

    // -----------------------------------------------------------------------
    // Recommended
    // -----------------------------------------------------------------------

    #[test]
    fn recurring_commitments_are_listed_before_recommendations() {
        let planning = PlanningConfig {
            recurring: vec![Recurring {
                name: "German practice".to_string(),
                hours: 0.75,
                days: vec!["mon".to_string()],
            }],
            ..PlanningConfig::default()
        };
        let tasks = [task("a", "Essay", Some(dt(2026, 9, 25, 23, 59)), 2.0)];
        let ranked = rank(&tasks, TODAY, &cal());
        let designated = designate_today(&ranked, TODAY, &cal(), Some(&planning));
        let text = render_today(
            &ranked,
            &designated,
            TODAY,
            &cal(),
            RenderOpts { planning: Some(&planning), ..Default::default() },
        );
        assert!(
            text.contains("- [ ] **German practice** \u{2014} 0.75h \u{b7} recurring"),
            "{text}"
        );
        assert!(text.find("German practice").unwrap() < text.find("Essay").unwrap());
    }

    /// A slice cap makes today's take smaller than what is left, and the page says both.
    #[test]
    fn a_partial_take_shows_how_much_is_left() {
        let mut prep = task("p", "PH 106 prep", Some(dt(2026, 10, 9, 23, 59)), 10.0);
        prep.slice_hours = Some(2.0);
        let ranked = rank(&[prep], TODAY, &cal());
        let designated = designate_today(&ranked, TODAY, &cal(), None);
        let text = render(&ranked, &designated, &cal());
        assert!(text.contains("**PH 106 prep** \u{2014} 2.0h (of 10.0h left)"), "{text}");
    }

    // -----------------------------------------------------------------------
    // The approvals line
    // -----------------------------------------------------------------------

    fn with_approvals(approvals: &ApprovalsResult, ranked: &[Task]) -> String {
        render_today(
            ranked,
            &[],
            TODAY,
            &cal(),
            RenderOpts { approvals: Some(approvals), ..Default::default() },
        )
    }

    #[test]
    fn the_approvals_line_carries_pending_age_and_the_calendar_backlog() {
        let one = [task("t", "T", Some(dt(2026, 9, 11, 23, 59)), 2.0)];
        let out = with_approvals(
            &ApprovalsResult {
                pending: 2,
                oldest_pending_days: 2,
                awaiting_calendar: 1,
                ..Default::default()
            },
            &one,
        );
        assert!(
            out.contains(
                "**Approvals: 2 pending** (oldest 2d) \u{b7} 1 approved awaiting calendar pass"
            ),
            "{out}"
        );

        let out = with_approvals(
            &ApprovalsResult { pending: 1, oldest_pending_days: 0, ..Default::default() },
            &one,
        );
        assert!(out.contains("**Approvals: 1 pending** (oldest 0d)"), "{out}");
        assert!(!out.contains("awaiting calendar"));

        let out =
            with_approvals(&ApprovalsResult { awaiting_calendar: 1, ..Default::default() }, &one);
        assert!(out.contains("**Approvals: 1 approved** awaiting calendar pass"), "{out}");

        // Quiet queue, no line at all.
        let out = with_approvals(&ApprovalsResult::default(), &one);
        assert!(!out.contains("Approvals"), "{out}");
    }

    #[test]
    fn the_approvals_line_survives_the_nothing_active_path() {
        let out = with_approvals(
            &ApprovalsResult { pending: 1, oldest_pending_days: 1, ..Default::default() },
            &[],
        );
        assert!(out.contains("Nothing active."));
        assert!(out.contains("**Approvals: 1 pending** (oldest 1d)"));
    }

    #[test]
    fn a_digest_count_joins_the_approvals_line_and_stands_alone_without_one() {
        let out = render_today(
            &[],
            &[],
            TODAY,
            &open_cal(),
            RenderOpts {
                approvals: Some(&ApprovalsResult {
                    pending: 3,
                    oldest_pending_days: 1,
                    events_in_digest: 5,
                    ..Default::default()
                }),
                ..Default::default()
            },
        );
        assert!(out.contains("Approvals: 3 pending"), "{out}");
        assert!(out.contains("Events: 5 in today's digest"), "{out}");

        let out = render_today(
            &[],
            &[],
            TODAY,
            &open_cal(),
            RenderOpts {
                approvals: Some(&ApprovalsResult { events_in_digest: 2, ..Default::default() }),
                ..Default::default()
            },
        );
        assert!(out.contains("Events: 2 in today's digest"), "{out}");
    }

    // -----------------------------------------------------------------------
    // The pending-amendment escalation
    // -----------------------------------------------------------------------

    fn changes(from: &str, to: &str) -> Mapping {
        crate::yaml::mapping_of(&format!("due:\n  from: {from}\n  to: {to}\n"))
    }

    fn amendment(urgency: &str, age: i64) -> AmendmentEntry {
        AmendmentEntry {
            target: "tasks/ph-106-exam-1-prep.md".to_string(),
            title: "PH 106 exam 1 moved".to_string(),
            urgency: urgency.to_string(),
            proposal: "amend-ph-106-exam-1".to_string(),
            changes: changes("2026-10-09T13:00", "2026-10-02T13:00"),
            age_days: age,
        }
    }

    fn scratch(name: &str) -> PathBuf {
        let dir = std::env::temp_dir().join(format!(
            "qo-render-{name}-{}-{:?}",
            std::process::id(),
            std::thread::current().id()
        ));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(dir.join("tasks")).unwrap();
        dir
    }

    /// A vault whose `tasks/ph-106-exam-1-prep.md` says what the caller asked it to say.
    fn target_note(vault: &Path, frontmatter: &str) {
        pystr::write_text(
            &vault.join("tasks").join("ph-106-exam-1-prep.md"),
            &format!("---\n{frontmatter}---\n\n"),
        )
        .unwrap();
    }

    fn render_amendment(entry: AmendmentEntry, vault: Option<&Path>) -> String {
        // One far-off task, so the page takes the main path rather than the empty one.
        let slack = [task("s", "Far off", Some(dt(2026, 12, 1, 23, 59)), 1.0)];
        let ranked = rank(&slack, TODAY, &cal());
        let designated = designate_today(&ranked, TODAY, &cal(), None);
        let approvals =
            ApprovalsResult { pending: 1, amendments_pending: vec![entry], ..Default::default() };
        render_today(
            &ranked,
            &designated,
            TODAY,
            &cal(),
            RenderOpts { approvals: Some(&approvals), vault, ..Default::default() },
        )
    }

    #[test]
    fn an_urgency_increasing_amendment_escalates_into_must_do() {
        let out = render_amendment(amendment("increases", 2), None);
        let must = must_do(&out);
        assert!(must.contains("PENDING AMENDMENT"), "{must}");
        assert!(must.contains("2d unactioned"), "{must}");
        // `_short_date` renders 10/9, not 10/09.
        assert!(must.contains("10/9 \u{2192} 10/2"), "{must}");
        assert!(must.contains("EARLIER"), "{must}");
        assert!(must.contains("approvals/amend-ph-106-exam-1.md"), "{must}");
        // The escalation replaces the all-clear line rather than sitting beside it.
        assert!(!must.contains("Nothing is at risk today."), "{must}");
    }

    #[test]
    fn a_decreasing_or_neutral_amendment_stays_out_of_must_do() {
        for urgency in ["decreases", "neutral"] {
            let out = render_amendment(amendment(urgency, 2), None);
            let must = must_do(&out);
            assert!(!must.contains("PENDING AMENDMENT"), "{urgency}: {must}");
        }
        let out = render_amendment(amendment("decreases", 2), None);
        assert!(must_do(&out).contains("Nothing is at risk today."));
    }

    #[test]
    fn an_urgent_amendment_surfaces_even_when_nothing_is_active() {
        let approvals = ApprovalsResult {
            pending: 1,
            amendments_pending: vec![amendment("increases", 2)],
            ..Default::default()
        };
        let out = render_today(
            &[],
            &[],
            TODAY,
            &cal(),
            RenderOpts { approvals: Some(&approvals), ..Default::default() },
        );
        assert!(out.contains("Nothing active."), "{out}");
        assert!(out.contains("PENDING AMENDMENT"), "{out}");
    }

    /// The day count is measured against the note, not the proposal's claim about the note.
    ///
    /// The proposal says the deadline was 10/20; the note actually reads 10/16. Rendering the
    /// proposal's own arithmetic prints "15 days EARLIER" for a move that is really 11 — a false
    /// deadline alarm at the top of Must do — and hides that the proposal is already stale.
    #[test]
    fn the_day_count_comes_from_the_note_and_a_disagreement_is_called_stale() {
        let vault = scratch("stale");
        target_note(&vault, "title: PH 106 exam 1\ndue: 2026-10-16T13:00\n");
        let mut entry = amendment("increases", 2);
        entry.changes = changes("2026-10-20T13:00", "2026-10-05T13:00");
        let out = render_amendment(entry, Some(&vault));
        assert!(!out.contains("15 days"), "{out}");
        assert!(out.contains("11 days"), "{out}");
        assert!(out.to_lowercase().contains("stale"), "{out}");
        let _ = std::fs::remove_dir_all(&vault);
    }

    #[test]
    fn a_cross_year_move_carries_its_years() {
        let vault = scratch("crossyear");
        target_note(&vault, "title: PH 106 exam 1\ndue: 2027-01-05T13:00\n");
        let mut entry = amendment("increases", 2);
        entry.changes = changes("2027-01-05T13:00", "2026-10-05T13:00");
        let out = render_amendment(entry, Some(&vault));
        assert!(out.contains("2027") && out.contains("2026"), "{out}");
        let _ = std::fs::remove_dir_all(&vault);
    }

    #[test]
    fn a_one_day_move_is_grammatical_and_shows_the_clock() {
        let vault = scratch("oneday");
        target_note(&vault, "title: PH 106 exam 1\ndue: 2026-10-09T13:00\n");
        let mut entry = amendment("increases", 2);
        entry.changes = changes("2026-10-09T13:00", "2026-10-08T13:00");
        let out = render_amendment(entry, Some(&vault));
        assert!(!out.contains("1 days"), "{out}");
        assert!(out.contains("1 day "), "{out}");
        let _ = std::fs::remove_dir_all(&vault);

        // 00:30 to the previous 23:00 is ninety minutes; a bare day count overstates it.
        let vault = scratch("midnight");
        target_note(&vault, "title: PH 106 exam 1\ndue: 2026-10-09T00:30\n");
        let mut entry = amendment("increases", 2);
        entry.changes = changes("2026-10-09T00:30", "2026-10-08T23:00");
        let out = render_amendment(entry, Some(&vault));
        assert!(out.contains("00:30") && out.contains("23:00"), "{out}");
        let _ = std::fs::remove_dir_all(&vault);
    }

    /// `derive_urgency` escalated this on the full datetimes, so the block must not then claim
    /// "0 days later" and contradict its own reason for being in Must do.
    #[test]
    fn a_same_day_move_renders_the_times_rather_than_zero_days() {
        let mut earlier = amendment("increases", 2);
        earlier.changes = changes("2026-10-09T23:59", "2026-10-09T13:00");
        let must = must_do(&render_amendment(earlier, None)).to_string();
        assert!(must.contains("due 10/9 23:59 \u{2192} 10/9 13:00 (EARLIER same day)"), "{must}");
        assert!(!must.contains("0 days"), "{must}");

        let mut later = amendment("increases", 2);
        later.changes = changes("2026-10-09T13:00", "2026-10-09T23:59");
        let must = must_do(&render_amendment(later, None)).to_string();
        assert!(must.contains("due 10/9 13:00 \u{2192} 10/9 23:59 (later same day)"), "{must}");

        let mut same = amendment("increases", 2);
        same.changes = changes("2026-10-09T13:00", "2026-10-09T13:00");
        assert!(must_do(&render_amendment(same, None)).contains("(no change)"));
    }

    /// A mixed aware/naive pair is a bare `<` in Python and raises TypeError, which loses
    /// today.md — `render_today` runs after `process_approvals` in `cli.run`. Both sides must
    /// arrive naive, at the wall clock each states.
    #[test]
    fn a_mixed_timezone_amendment_does_not_lose_the_page() {
        let mut entry = amendment("increases", 2);
        entry.changes = changes("2026-10-09T13:00:00-05:00", "2026-10-09T09:00:00");
        let must = must_do(&render_amendment(entry, None)).to_string();
        assert!(must.contains("PENDING AMENDMENT"), "{must}");
        assert!(must.contains("due 10/9 13:00 \u{2192} 10/9 09:00 (EARLIER same day)"), "{must}");
    }

    #[test]
    fn the_heading_prefers_the_targets_title_and_falls_back_when_it_cannot_be_read() {
        let vault = scratch("title");
        target_note(&vault, "title: PH 106 exam 1 prep (Ch 13-16)\ndue: 2026-10-09T13:00\n");
        let out = render_amendment(amendment("increases", 2), Some(&vault));
        assert!(out.contains("PH 106 exam 1 prep (Ch 13-16)"), "{out}");

        // Non-mapping frontmatter is as unreadable as a missing file.
        pystr::write_text(
            &vault.join("tasks").join("ph-106-exam-1-prep.md"),
            "---\n- a\n- b\n---\n\nbody\n",
        )
        .unwrap();
        let out = render_amendment(amendment("increases", 2), Some(&vault));
        assert!(out.contains("PH 106 exam 1 moved"), "{out}");
        let _ = std::fs::remove_dir_all(&vault);

        // Missing file entirely.
        let vault = scratch("missing");
        let out = render_amendment(amendment("increases", 2), Some(&vault));
        assert!(out.contains("PH 106 exam 1 moved"), "{out}");
        let _ = std::fs::remove_dir_all(&vault);
    }

    /// Containment is enforced at execution; the renderer reads the target too, so a proposal
    /// naming `../outside.md` must not pull that file's title onto the page.
    #[test]
    fn a_target_outside_the_vault_is_not_read_at_render_time() {
        let outer = scratch("escape");
        let vault = outer.join("vault");
        std::fs::create_dir_all(vault.join("tasks")).unwrap();
        pystr::write_text(
            &outer.join("outside.md"),
            "---\ntitle: SECRET NOT IN THE VAULT\n---\n\n",
        )
        .unwrap();
        let mut entry = amendment("increases", 2);
        entry.target = "../outside.md".to_string();
        let out = render_amendment(entry, Some(&vault));
        assert!(!out.contains("SECRET NOT IN THE VAULT"), "{out}");
        assert!(out.contains("PH 106 exam 1 moved"), "{out}");
        let _ = std::fs::remove_dir_all(&outer);
    }

    /// The title is spliced straight into a markdown bullet. A second line escapes the list, and
    /// one starting with `##` injects a heading that reorganises the page.
    #[test]
    fn a_multiline_title_cannot_break_the_must_do_list() {
        let vault = scratch("inject");
        target_note(
            &vault,
            "title: \"PH 106 exam\\n## Injected heading\"\ndue: 2026-10-09T13:00\n",
        );
        let out = render_amendment(amendment("increases", 2), Some(&vault));
        assert!(!out.contains("\n## Injected heading"), "{out}");
        assert!(out.contains("PH 106 exam ## Injected heading"), "{out}");
        let _ = std::fs::remove_dir_all(&vault);
    }

    /// A `changes` map with no usable `due` renders the block without the move line rather than
    /// failing. Python reaches the same state from a `changes` that is not a mapping at all;
    /// `AmendmentEntry.changes` is typed as a `Mapping`, so an empty one is the port's spelling
    /// of that case.
    #[test]
    fn an_amendment_without_a_due_change_renders_no_move_line() {
        let mut entry = amendment("increases", 2);
        entry.changes = Mapping::new();
        let must = must_do(&render_amendment(entry, None)).to_string();
        assert!(must.contains("PENDING AMENDMENT"), "{must}");
        assert!(!must.contains("  - due"), "{must}");
    }

    // -----------------------------------------------------------------------
    // conflicts_with
    // -----------------------------------------------------------------------

    #[test]
    fn a_conflicting_obligation_warns_inside_its_must_do_bullet() {
        let today = Date::constant(2026, 8, 20);
        let mut t = task("convocation", "BUI 100 Convocation", Some(dt(2026, 8, 21, 18, 0)), 1.0);
        t.conflicts_with = Some("MATH 125 lecture 17:00\u{2013}18:15".to_string());
        let ranked = rank(&[t], today, &open_cal());
        let text = render_today(&ranked, &[], today, &open_cal(), RenderOpts::default());
        let must = must_do(&text);
        assert!(must.contains("BUI 100 Convocation"), "{must}");
        assert!(
            must.contains("  \u{26a0} conflicts with MATH 125 lecture 17:00\u{2013}18:15"),
            "{must}"
        );
    }

    /// A note that spells the field but leaves it blank parses as null; a whitespace-only value
    /// must not produce a bare warning either.
    #[test]
    fn an_absent_or_blank_conflict_renders_nothing_extra() {
        let today = Date::constant(2026, 8, 20);
        for value in [None, Some(""), Some("   ")] {
            let mut t = task("a", "Essay", Some(dt(2026, 8, 21, 18, 0)), 1.0);
            t.conflicts_with = value.map(str::to_string);
            let ranked = rank(&[t], today, &open_cal());
            let text = render_today(&ranked, &[], today, &open_cal(), RenderOpts::default());
            assert!(!text.contains("conflicts with"), "{value:?}: {text}");
        }
    }

    // -----------------------------------------------------------------------
    // Coming up
    // -----------------------------------------------------------------------

    fn event(title: &str, day: i8, hour: i8) -> ComingUpEvent {
        ComingUpEvent {
            uid: format!("u-{title}"),
            title: title.to_string(),
            start: dt(2026, 8, day, hour, 0),
            location: "Lloyd 38".to_string(),
            organizer: String::new(),
        }
    }

    fn render_events(events: &[ComingUpEvent], generated_at: Option<DateTime>) -> String {
        render_today(
            &[],
            &[],
            Date::constant(2026, 8, 20),
            &open_cal(),
            RenderOpts { events, generated_at, ..Default::default() },
        )
    }

    #[test]
    fn coming_up_lists_upcoming_events_with_their_location() {
        let text = render_events(&[event("AI Club Kickoff", 25, 18)], None);
        assert!(text.contains("## Coming up"), "{text}");
        assert!(
            text.contains("- Tue 8/25 18:00 \u{b7} AI Club Kickoff \u{b7} Lloyd 38"),
            "{text}"
        );
    }

    #[test]
    fn coming_up_is_capped_at_five_and_absent_when_empty() {
        let events: Vec<ComingUpEvent> =
            (0..8).map(|i| event(&format!("Event {i}"), 21 + i as i8, 18)).collect();
        let text = render_events(&events, None);
        assert_eq!(text.matches("\u{b7} Lloyd 38").count(), 5, "{text}");

        assert!(!render_events(&[], None).contains("## Coming up"));
        // Everything already past — the section disappears rather than rendering empty.
        assert!(!render_events(&[event("Old Thing", 1, 18)], None).contains("Old Thing"));
    }

    #[test]
    fn the_no_active_tasks_path_keeps_a_blank_line_between_coming_up_and_capacity() {
        let text = render_events(&[event("Evening Event", 20, 18)], None);
        let lines: Vec<&str> = text.split('\n').collect();
        let at = lines
            .iter()
            .position(|l| l.starts_with("- ") && l.contains("Evening Event"))
            .expect("event line");
        assert_eq!(lines[at + 1], "", "{text}");
        assert!(lines[at + 2].contains("Capacity today:"), "{text}");
    }

    #[test]
    fn coming_up_renders_after_everything_active_on_the_main_path() {
        let today = Date::constant(2026, 8, 20);
        let tasks = [task("a", "Essay", Some(dt(2026, 8, 25, 23, 59)), 2.0)];
        let ranked = rank(&tasks, today, &open_cal());
        let text = render_today(
            &ranked,
            &[],
            today,
            &open_cal(),
            RenderOpts { events: &[event("Seminar", 22, 15)], ..Default::default() },
        );
        assert!(text.find("## Everything active").unwrap() < text.find("## Coming up").unwrap());
        assert!(text.contains("Seminar"));
    }

    #[test]
    fn the_clock_narrows_the_cutoff_only_on_the_runs_own_day() {
        // Rendering at 14:00 hides the 09:00 event and keeps the 18:00 one.
        let morning = ComingUpEvent {
            uid: "u-morning".to_string(),
            title: "Morning Class".to_string(),
            start: dt(2026, 8, 20, 9, 0),
            location: "Lloyd 38".to_string(),
            organizer: String::new(),
        };
        let text = render_events(
            &[morning, event("Evening Talk", 20, 18)],
            Some(dt(2026, 8, 20, 14, 0)),
        );
        assert!(!text.contains("Morning Class"), "{text}");
        assert!(text.contains("Evening Talk"), "{text}");

        // A wall clock from another day must NOT narrow it: the same vault and the same `today`
        // have to render the same page however long after the fact the run happens.
        let text = render_events(&[event("Blount Dinner", 25, 18)], Some(dt(2026, 8, 26, 0, 14)));
        assert!(text.contains("## Coming up"), "{text}");
        assert!(text.contains("Blount Dinner"), "{text}");
    }

    // -----------------------------------------------------------------------
    // The golden file's own lines
    // -----------------------------------------------------------------------

    /// Reference output. Every line below is copied from `tests/fixtures/golden-today-s1.md`,
    /// which the **Python** engine produced. They exercise all three hour precisions, both
    /// date formats and the em dash on one page, and they are the lines the oracle diffs first.
    #[test]
    fn the_golden_files_own_lines_render_exactly() {
        use crate::models::{apply_slice_default, load_tasks};
        use crate::planning::load_planning;

        let root = Path::new("tests/fixtures/vault-s1");
        let planning = load_planning(&root.join("config/planning.yaml"));
        let mut tasks = load_tasks(&root.join("tasks"), None);
        apply_slice_default(&mut tasks, planning.slice_hours);
        let cal = WeekCalendar::from_file(&root.join("config/week_template.yaml"), Vec::new());
        let today = Date::constant(2026, 8, 28);
        let ranked = rank(&tasks, today, &cal);
        let designated = designate_today(&ranked, today, &cal, Some(&planning));

        let text = render_today(
            &ranked,
            &designated,
            today,
            &cal,
            RenderOpts { planning: Some(&planning), ..Default::default() },
        );

        for line in [
            "# Today \u{2014} Friday, August 28",
            "**Runway: -4 days (behind)** \u{b7} Capacity today: 8.25h \u{b7} 5 active",
            "- [ ] **CS 100 HW 01 Introduction to C** \u{2014} due Wed 8/26 \u{b7} 2.5h left",
            "- [ ] **Erste Reflexion** \u{2014} due Thu 8/27 \u{b7} 0.5h left",
            "- [ ] **GN 103 Hausaufgaben \u{2014} due Mon 08-31 (25 activities)** \u{2014} 1.5h (of 2.8h left)",
            "- [ ] **BUI 101 reading - Augustine, Confessions Books IV-VI** \u{2014} 2.0h (of 2.5h left)",
            "1. CS 100 HW 01 Introduction to C \u{2014} due Wed 8/26 \u{b7} start by 8/24 \u{b7} OVERDUE START \u{b7} 2.5h left",
            "3. GN 103 Hausaufgaben \u{2014} due Mon 08-31 (25 activities) \u{2014} due Mon 8/31 \u{b7} start by 8/29 \u{b7} 1d slack \u{b7} 2.8h left",
            "5. PH 106 Exam 1 prep \u{2014} due Fri 9/25 \u{b7} start by 9/19 \u{b7} 22d slack \u{b7} 12.0h left",
        ] {
            assert!(text.contains(line), "missing:\n  {line}\n\nrendered:\n{text}");
        }
    }

    #[test]
    fn capacity_breakdown_is_the_sentence_render_builds_byte_for_byte() {
        // U+2212 MINUS SIGN, not a hyphen; all three figures round2; py_float keeps "8.0".
        let vault = Path::new("tests/fixtures/vault-full");
        let events = crate::calfeed::read_snapshot(&vault.join("state").join("calendar.md"))
            .into_values()
            .flatten()
            .collect();
        let cal = WeekCalendar::from_file(&vault.join("config").join("week_template.yaml"), events);
        let today = Date::constant(2026, 8, 28);
        let c = capacity_breakdown(&cal, today);
        assert_eq!(c.text, "Capacity today: 6.0h (template 8.25h − 2.25h calendar)");
        assert_eq!((c.capacity, c.template, c.calendar), (6.0, 8.25, 2.25));
    }

    #[test]
    fn capacity_breakdown_collapses_when_the_calendar_takes_nothing() {
        let vault = Path::new("tests/fixtures/vault-s1");
        let cal = WeekCalendar::from_file(&vault.join("config").join("week_template.yaml"), Vec::new());
        let c = capacity_breakdown(&cal, Date::constant(2026, 8, 28));
        assert!(!c.text.contains('('), "{}", c.text);
        assert_eq!(c.calendar, 0.0);
    }

    #[test]
    fn upcoming_is_what_coming_up_renders() {
        let today = Date::constant(2026, 8, 28);
        let ev = |uid: &str, m: i8, d: i8| ComingUpEvent {
            uid: uid.into(), title: uid.to_uppercase(), start: Date::constant(2026, m, d).at(12, 0, 0, 0),
            location: String::new(), organizer: String::new(),
        };
        let events = vec![ev("f", 9, 6), ev("a", 9, 1), ev("b", 9, 2), ev("c", 9, 3), ev("d", 9, 4), ev("e", 9, 5), ev("z", 8, 1)];
        let picked = upcoming(&events, today, None);
        assert_eq!(picked.iter().map(|e| e.uid.as_str()).collect::<Vec<_>>(), ["a", "b", "c", "d", "e"]);
        // and the rendered lines are the same five, in the same order
        let lines = coming_up(&events, today, None);
        assert_eq!(lines.iter().filter(|l| l.starts_with("- ")).count(), 5);
        assert!(lines[3].contains("· A"));
    }
}
