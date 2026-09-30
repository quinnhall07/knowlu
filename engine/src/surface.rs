//! The read model — everything the console page shows, computed from the vault and never written
//! back. Port target: S2's `engine/surface.py`, which was designed and never built; the console
//! spec (2026-09-02) §4 is the contract. Two rules the whole module keeps:
//!
//! * **Never writes.** Not `today.md`, not a run record, not a journal record. `build_state` is
//!   byte-compared before/after on every fixture. So nothing here calls `process_approvals`,
//!   `defer_over_budget`, `emit_digest`, `write_roster`, `info_pass`, `load_calendar_events` or
//!   `ensure_ids` — every one of those writes or fetches.
//! * **Shared logic has one implementation.** The status word, the capacity sentence and
//!   `due_move` come from `render`; the ranking from `ranking`; nothing is re-derived here.

use std::collections::BTreeMap;
use std::path::Path;

use jiff::civil::{Date, DateTime};
use serde::Serialize;
use serde_yaml_ng::Mapping;

use crate::models::{apply_slice_default, load_task_notes, Task};
use crate::planning::{load_planning, PlanningConfig};
use crate::ranking::{designate_today_explained, partition_must_do, rank, Take};
use crate::render::{capacity_breakdown, py_float, round2, runway_days, short_date, status_word};
use crate::scheduling::{add_days, days_between, slack_days, DEFAULT_BUFFER_DAYS};
use crate::weekcal::WeekCalendar;

pub const SPILL_CAP: i64 = 120;
pub const BLOCK_MINUTES: i64 = 30;
/// `delta.records` newest-first bound — a page must never be asked to render an unbounded diff.
pub const DELTA_RECORD_CAP: usize = 200;

/// `-0.0` prints as `-0.0` through `py_float`; an empty sum is `0.0`. Apply at every summed hours
/// field before it is stored (total_hours, hours per group, spare_hours, open_hours,
/// commitments_hours, undated_hours, bucket hours).
fn nz(x: f64) -> f64 {
    if x == 0.0 {
        0.0
    } else {
        x
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "kebab-case")]
pub enum View {
    Today,
    Overdue,
    Week,
    Later,
    #[serde(rename = "all")]
    AllActive,
    Decisions,
    GoodToKnow,
    Issues,
    Runs,
}

impl View {
    pub fn parse(name: &str) -> Option<View> {
        Some(match name {
            "today" => View::Today,
            "overdue" => View::Overdue,
            "week" => View::Week,
            "later" => View::Later,
            "all" => View::AllActive,
            "decisions" => View::Decisions,
            "good-to-know" => View::GoodToKnow,
            "issues" => View::Issues,
            "runs" => View::Runs,
            _ => return None,
        })
    }

    pub fn name(&self) -> &'static str {
        match self {
            View::Today => "today",
            View::Overdue => "overdue",
            View::Week => "week",
            View::Later => "later",
            View::AllActive => "all",
            View::Decisions => "decisions",
            View::GoodToKnow => "good-to-know",
            View::Issues => "issues",
            View::Runs => "runs",
        }
    }
}

/// The vault, read once for one page. Every builder takes `&Loaded`; none re-reads `tasks/`.
pub struct Loaded {
    pub tasks: Vec<Task>,
    pub metas: BTreeMap<String, Mapping>,
    pub unreadable: Vec<String>,
    pub cal: WeekCalendar,
    pub planning: PlanningConfig,
    pub ranked: Vec<Task>,
    pub takes: Vec<Take>,
    /// §6.4's "what moved": today's plan against the day's first window (or, in the `--window`
    /// preview, against the current window). `None` with no planning-day note, and whenever the
    /// windows or the plans are equal.
    pub moved: Option<crate::commitments::Moved>,
}

/// The one place the vault is read for the page: notes, calendar snapshot, and the ranking and
/// today's takes computed from them. `state/calendar.md` is the snapshot the last runner wrote,
/// not a fetch — capacity here is only as fresh as the last run, which is what keeps this
/// function free of network access and file writes.
pub fn load(vault: &Path, today: Date) -> Loaded {
    load_with(vault, today, None)
}

/// A planning window per weekday (`DAY_KEYS` order), as `commitments::parse_window` returns it.
pub type Window = [Option<(jiff::civil::Time, jiff::civil::Time)>; 7];

/// `cal` with its whole planning window replaced by `window`: a weekday `window` leaves out gets
/// the template's `(day_start, day_end)`. Seven consecutive days from `today` cover every weekday.
fn with_window(mut cal: WeekCalendar, today: Date, window: &Window) -> WeekCalendar {
    let template = (cal.day_start, cal.day_end);
    for offset in 0..7 {
        let day = add_days(today, offset);
        let idx = crate::planning::DAY_KEYS
            .iter()
            .position(|k| *k == crate::planning::day_key(day))
            .unwrap_or(0);
        let (start, end) = window[idx].unwrap_or(template);
        cal = cal.with_day_window(day, start, end);
    }
    cal
}

/// [`load`], optionally under a proposed planning window (the `--window` preview, §6.4). Live
/// (`preview: None`), `moved` diffs today's plan against `commitments::baseline` — only when that
/// baseline differs from `cal.window(today)`. Under a preview, the day is computed under the
/// proposed window and `moved` diffs it against the **current** window. Both designations come
/// from the same ranked list. Writes nothing.
fn load_with(vault: &Path, today: Date, preview: Option<&Window>) -> Loaded {
    let planning = load_planning(&vault.join("config").join("planning.yaml"));
    let mut unreadable = Vec::new();
    let notes = load_task_notes(&vault.join("tasks"), Some(&mut unreadable));
    let metas: BTreeMap<String, Mapping> =
        notes.iter().map(|(t, m)| (t.slug.clone(), m.clone())).collect();
    let mut tasks: Vec<Task> = notes.into_iter().map(|(t, _)| t).collect();
    apply_slice_default(&mut tasks, planning.slice_hours);
    let events = crate::calfeed::read_snapshot(&vault.join("state").join("calendar.md"))
        .into_values()
        .flatten()
        .collect();
    let current = WeekCalendar::for_vault(vault, events);
    let (cal, against) = match preview {
        Some(window) => (with_window(current.clone(), today, window), Some(current)),
        None => {
            let set = crate::commitments::load(vault);
            let base = crate::commitments::baseline(vault, &set, &current, today)
                .filter(|base| *base != current.window(today));
            let against = base.map(|(start, end)| current.clone().with_day_window(today, start, end));
            (current, against)
        }
    };
    let ranked = rank(&tasks, today, &cal);
    let takes = designate_today_explained(&ranked, today, &cal, Some(&planning));
    let moved = against
        .and_then(|base| crate::commitments::moved(&ranked, today, &cal, &base, Some(&planning)));
    Loaded { tasks, metas, unreadable, cal, planning, ranked, takes, moved }
}

#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct NavCount {
    pub count: i64,
    pub hours: f64,
}

#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct NavCounts {
    pub today: NavCount,
    pub overdue: NavCount,
    pub week: NavCount,
    pub later: NavCount,
    pub undated: NavCount,
    pub all: NavCount,
    pub decisions: NavCount,
    pub good_to_know: NavCount,
    pub issues: NavCount,
    pub runs_warn: NavCount,
}

// Nav hours read like `render_today`'s must-do lines — one decimal, `2.8h` — not the two-decimal
// precision `capacity_breakdown` uses for the capacity sentence. GN 103's remaining hours are
// `3.1 * 0.9 = 2.79`; rounding each task before summing and rounding the sum agree here (`round1`
// of the raw sum), but the golden `week` bucket (2.79 + 2.5 = 5.29 → 5.3) only comes out right at
// one decimal — `round2` leaves it `5.29`, which is the display precision this bucket does not use.
fn count_of(tasks: &[&Task]) -> NavCount {
    // This toolchain's `Sum<&f64> for f64` returns `-0.0` over an empty iterator (not the `+0.0`
    // the identity element would suggest) — `nz` guards it the same way the named fields do.
    NavCount { count: tasks.len() as i64, hours: nz(round1(tasks.iter().map(|t| t.remaining_hours()).sum())) }
}

/// Horizon of a dated task relative to `today`: "overdue" (< today), "week" (today ..= today+6),
/// "later" (> today+6); undated tasks are their own bucket and are never dropped.
pub fn horizon(task: &Task, today: Date) -> &'static str {
    match task.due {
        None => "undated",
        Some(due) => {
            let gap = days_between(due.date(), today);
            if gap < 0 {
                "overdue"
            } else if gap <= 6 {
                "week"
            } else {
                "later"
            }
        }
    }
}

/// The nav rail's counts and hours: dated tasks partitioned by [`horizon`], must-do (`today`),
/// everything active (`all`), and the four counters the caller passes straight through from other
/// pages' pending counts — `nav_counts` never reads approvals, info, issues, or runs itself.
pub fn nav_counts(
    l: &Loaded,
    today: Date,
    pending: i64,
    info_open: i64,
    issues_open: i64,
    runs_warn: i64,
) -> NavCounts {
    let (must, _) = partition_must_do(&l.ranked, today, &l.cal);
    let by = |h: &str| l.ranked.iter().filter(|t| horizon(t, today) == h).collect::<Vec<_>>();
    NavCounts {
        today: count_of(&must.iter().collect::<Vec<_>>()),
        overdue: count_of(&by("overdue")),
        week: count_of(&by("week")),
        later: count_of(&by("later")),
        undated: count_of(&by("undated")),
        all: count_of(&l.ranked.iter().collect::<Vec<_>>()),
        decisions: NavCount { count: pending, hours: 0.0 },
        good_to_know: NavCount { count: info_open, hours: 0.0 },
        issues: NavCount { count: issues_open, hours: 0.0 },
        runs_warn: NavCount { count: runs_warn, hours: 0.0 },
    }
}

#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct Topline {
    pub date: String,
    pub runway_days: f64,
    pub status_word: String,
    pub active: i64,
    pub generated_at: Option<String>,
    pub generated_by: Option<String>,
    pub engine_build: Option<String>,
}

const WEEKDAYS: [&str; 7] = ["Mon", "Tue", "Wed", "Thu", "Fri", "Sat", "Sun"];
const MONTHS: [&str; 12] =
    ["Jan", "Feb", "Mar", "Apr", "May", "Jun", "Jul", "Aug", "Sep", "Oct", "Nov", "Dec"];

/// The page's masthead: today's date and runway, spelled the way the console shows it (`Fri 28
/// Aug`, never `today.md`'s `Friday, August 28`), plus who and when the last run finished — read
/// from `state/runs/`'s newest `end` record, never from a fresh run.
pub fn topline(vault: &Path, l: &Loaded, today: Date) -> Topline {
    let (generated_at, generated_by) = newest_end_record(vault)
        .map(|(ts, runner)| (Some(ts), Some(runner)))
        .unwrap_or((None, None));
    let rw = runway_days(&l.ranked, today, &l.cal);
    Topline {
        date: format!(
            "{} {} {}",
            WEEKDAYS[today.weekday().to_monday_zero_offset() as usize],
            today.day(),
            MONTHS[today.month() as usize - 1]
        ),
        runway_days: rw.round(),
        status_word: status_word(rw).to_string(),
        active: l.ranked.len() as i64,
        generated_at,
        generated_by,
        engine_build: crate::BUILD_SHA.map(str::to_string),
    }
}

/// `(ts as "%Y-%m-%d %H:%M" in the vault's zone, runner)` of the newest `end` record, if any.
fn newest_end_record(vault: &Path) -> Option<(String, String)> {
    let mut runs = crate::runs::Runs::new(vault);
    let mut best: Option<(String, String)> = None;
    for rec in runs.read(None) {
        if rec.get("phase").and_then(|v| v.as_str()) != Some("end") {
            continue;
        }
        let Some(ts) = rec.get("ts").and_then(|v| v.as_str()) else { continue };
        let runner = rec.get("runner").and_then(|v| v.as_str()).unwrap_or("").to_string();
        if best.as_ref().map_or(true, |(b, _)| ts > b.as_str()) {
            best = Some((ts.to_string(), runner));
        }
    }
    best.map(|(ts, runner)| (local_stamp(vault, &ts), runner))
}

/// A journal/runs `ts` (`2026-09-01T20:55:12.345Z`) rendered as the page's `2026-09-01 20:55` in
/// the vault's time zone — the same zone `cli::local_now` uses, so the topline agrees with the
/// footer `today.md` carries.
pub fn local_stamp(vault: &Path, ts: &str) -> String {
    let tz = crate::cli::vault_zone(vault);
    match ts.parse::<jiff::Timestamp>() {
        Ok(t) => t.to_zoned(tz).strftime("%Y-%m-%d %H:%M").to_string(),
        Err(_) => ts.to_string(),
    }
}

#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct Verdict {
    pub runway_days: f64,
    pub word: String,
    pub headline: String,
    pub lede: String,
    pub must_hours: f64,
    pub capacity_hours: f64,
    pub template_hours: f64,
    pub calendar_hours: f64,
    pub active: i64,
    pub capacity_text: String,
    pub overflow_sentence: Option<String>,
}

/// The verdict card: one headline, one lede, the capacity sentence `render_today` also prints, and
/// the overflow warning when must-do work does not fit — verbatim from render.rs's `Must-do work
/// (…h) exceeds today's capacity (…h)` line (old lines 458–464), so the console and `today.md`
/// never say this two different ways. The fit test there is the RAW must-do sum against
/// `capacity`, not the rounded `must_hours` this struct reports — matching render.rs exactly,
/// including the one case they could differ: a sum that rounds down to equal capacity but was
/// fractionally over it. The lede and the overflow sentence spell that raw sum the same way
/// (`{must_raw:.1}`, render's own precision for remaining hours) — one card must never show two
/// different numbers for the same quantity. `status_word` reads the unrounded runway, exactly as
/// `topline` does, via the same [`crate::render::runway_days`]; only the `runway_days` field is
/// rounded.
pub fn verdict(l: &Loaded, today: Date) -> Verdict {
    let rw = runway_days(&l.ranked, today, &l.cal);
    let word = status_word(rw).to_string();
    let cap = capacity_breakdown(&l.cal, today);
    let (must_tasks, _) = partition_must_do(&l.ranked, today, &l.cal);
    // See `count_of`: an empty sum on this toolchain is `-0.0`, and this value both prints
    // (`must_text`) and is stored (`must_hours`) — guard it once, at the source.
    let must_raw: f64 = nz(must_tasks.iter().map(|t| t.remaining_hours()).sum());
    let must_text = format!("{must_raw:.1}");
    let fits = must_raw <= cap.capacity;
    let headline = if l.ranked.is_empty() {
        EMPTY_TEXT.active.to_string()
    } else if rw < 0.0 {
        format!("{} behind, and today {}.", days_phrase(-rw), if fits { "fits" } else { "doesn't fit" })
    } else {
        format!("{}, and today {}.", capitalise(&word), if fits { "fits" } else { "doesn't fit" })
    };
    let lede = format!("{must_text}h of must-do work against {}h of capacity.", py_float(cap.capacity));
    let overflow_sentence = (!fits).then(|| {
        format!(
            "⚠ Must-do work ({must_text}h) exceeds today's capacity ({}h). Something has to give — decide what, deliberately.",
            py_float(cap.capacity)
        )
    });
    Verdict {
        runway_days: rw.round(),
        word,
        headline,
        lede,
        must_hours: round1(must_raw),
        capacity_hours: cap.capacity,
        template_hours: cap.template,
        calendar_hours: cap.calendar,
        active: l.ranked.len() as i64,
        capacity_text: cap.text,
        overflow_sentence,
    }
}

fn days_phrase(days: f64) -> String {
    let n = days as i64;
    if n == 1 {
        "One day".into()
    } else {
        format!("{n} days")
    }
}

fn capitalise(word: &str) -> String {
    let mut c = word.chars();
    match c.next() {
        Some(f) => f.to_uppercase().collect::<String>() + c.as_str(),
        None => String::new(),
    }
}

#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct SpillItem {
    pub id: String,
    pub title: String,
    pub hours: f64,
}

#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct Meter {
    pub block_minutes: i64,
    pub fit_blocks: i64,
    pub spill_blocks: i64,
    pub capped: i64,
    pub spill_hours: f64,
    pub spill_items: Vec<SpillItem>,
    pub capacity_text: String,
}

/// The day laid out in 30-minute blocks: how many today's capacity fits, how many must-do work
/// spills past it (capped at [`SPILL_CAP`], with `capped` counting the rest), and which tasks
/// account for the spill.
pub fn meter(l: &Loaded, today: Date) -> Meter {
    let (must, _) = partition_must_do(&l.ranked, today, &l.cal);
    let cap = capacity_breakdown(&l.cal, today);
    let must_h: f64 = must.iter().map(|t| t.remaining_hours()).sum();
    let fit = (cap.capacity * 2.0).floor() as i64;
    let spill = ((must_h * 2.0).ceil() as i64 - fit).max(0);
    let mut remaining = cap.capacity;
    let mut items = Vec::new();
    for t in &must {
        if remaining >= t.remaining_hours() {
            remaining -= t.remaining_hours();
        } else {
            items.push(SpillItem { id: note_id(l, &t.slug), title: t.title.clone(), hours: round1(t.remaining_hours()) });
        }
    }
    Meter {
        block_minutes: BLOCK_MINUTES,
        fit_blocks: fit,
        spill_blocks: spill.min(SPILL_CAP),
        capped: (spill - SPILL_CAP).max(0),
        spill_hours: round1((must_h - cap.capacity).max(0.0)),
        spill_items: items,
        capacity_text: cap.text,
    }
}

/// `round(x, 1)`, half to EVEN — matching `render::round2` one line's reasoning up a decimal
/// place, not half-away-from-zero. Both ties are reachable frontmatter (`0.5h @ 50%` remaining is
/// `0.25`; `2.5h @ 50%` is `1.25`), and half-away-from-zero disagreed with `render_today`'s own
/// `{:.1}` display of the same figure at both of them (`0.25` prints `0.2h left`, `1.25` prints
/// `1.2h left`) — the nav rail and `spill_items` must say the same thing `today.md` does.
fn round1(x: f64) -> f64 {
    (x * 10.0).round_ties_even() / 10.0
}

/// The note's opaque `id:`; the slug when a pre-S1 note has none.
fn note_id(l: &Loaded, slug: &str) -> String {
    l.metas
        .get(slug)
        .and_then(|m| crate::yaml::get(m, "id"))
        .and_then(crate::yaml::text)
        .unwrap_or_else(|| slug.to_string())
}

pub const OVERSIZED_HOURS: f64 = 6.0;

/// A pending `amend` proposal, exactly as `process_approvals` builds it (`approvals.rs`'s
/// `AmendmentEntry`) — the console badges rows and cards from the same shape the renderer reads,
/// never a second one. Task 7 is where this list gets loaded from `approvals/`; until then callers
/// pass `&[]`.
pub use crate::approvals::AmendmentEntry as PendingAmendment;

#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct AmendBadge {
    pub fields: Vec<String>,
    pub proposal: String,
    pub urgency: String,
    pub age_days: i64,
    pub sentence: String,
}

#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct Row {
    pub id: String,
    pub slug: String,
    pub title: String,
    pub course: Option<String>,
    pub source_tag: String,
    pub due: Option<String>,
    pub due_short: Option<String>,
    pub start_by: Option<String>,
    pub slack_days: Option<f64>,
    pub overdue_start: bool,
    pub hours: f64,
    pub progress_pct: i64,
    pub importance: i64,
    pub importance_reason: Option<String>,
    pub effort_source: Option<String>,
    pub effort_confidence: Option<String>,
    pub status: String,
    pub judged: bool,
    pub oversized: bool,
    pub amend_badge: Option<AmendBadge>,
    pub flag_text: Option<String>,
    pub merge_text: Option<String>,
    pub conflict_text: Option<String>,
    pub why: String,
    /// `"od"` (overdue start) / `"soon"` (due within 2 days) / `""` — the mockup's pip classes.
    pub urgency: String,
}

/// An object carries ⚑ when its `judgment:` block says more than `{migrated: true}`, or the
/// journal shows an `agent:*` record for it (S2 §7.8). `row` passes `&[]` until Task 8's
/// `note_detail` wires the journal.
pub fn is_judged(meta: &Mapping, records: &[crate::ledger::Record]) -> bool {
    if let Some(serde_yaml_ng::Value::Mapping(block)) = crate::yaml::get(meta, "judgment") {
        if block.keys().any(|k| crate::yaml::text(k).as_deref() != Some("migrated")) {
            return true;
        }
    }
    records.iter().any(|r| r.get("actor").and_then(|v| v.as_str()).is_some_and(|a| a.starts_with("agent:")))
}

fn meta_text(m: Option<&Mapping>, key: &str) -> Option<String> {
    m.and_then(|m| crate::yaml::get(m, key)).and_then(crate::yaml::text)
}

fn source_tag(meta: Option<&Mapping>) -> String {
    // The chip the mockup shows: the source_uid's prefix (`zybooks`, `vhl`, `gmail`, `bb`), else created_by, else "manual".
    if let Some(uid) = meta_text(meta, "source_uid") {
        if let Some((prefix, _)) = uid.split_once(':') {
            return prefix.to_string();
        }
    }
    meta_text(meta, "created_by").unwrap_or_else(|| "manual".to_string())
}

/// A change-set value into the words the badge sentence shows: `changes.due` on a real amend
/// proposal is a `{from, to}` mapping (`amend_card` reads the same shape), so a mapping value
/// renders its `to`; anything else — a bare scalar — renders itself.
fn amend_change_text(value: &serde_yaml_ng::Value) -> String {
    let target = match value {
        serde_yaml_ng::Value::Mapping(m) => crate::yaml::get(m, "to").cloned(),
        other => Some(other.clone()),
    };
    target
        .as_ref()
        .and_then(|v| crate::approvals::as_note_datetime(Some(v)).map(|d| short_date(d.date())).or_else(|| crate::yaml::text(v)))
        .unwrap_or_default()
}

fn amend_sentence(changes: &Mapping) -> String {
    // "a proposal would move due to 9/22" — from the change set, never the proposal's title.
    let mut parts = Vec::new();
    for (k, v) in changes {
        let key = crate::yaml::text(k).unwrap_or_default();
        parts.push(format!("{key} to {}", amend_change_text(v)));
    }
    format!("a proposal would move {}", parts.join(", "))
}

pub fn row(l: &Loaded, task: &Task, today: Date, amendments: &[PendingAmendment]) -> Row {
    let meta = l.metas.get(&task.slug);
    let why = crate::scheduling::start_by_explained(task, &l.cal, DEFAULT_BUFFER_DAYS);
    let slack = task.due.map(|_| slack_days(task, today, &l.cal, DEFAULT_BUFFER_DAYS));
    let rel = format!("tasks/{}.md", task.slug);
    let id = note_id(l, &task.slug);
    let badge = amendments
        .iter()
        .find(|a| a.target == rel || a.target == id || a.target == task.slug)
        .map(|a| AmendBadge {
            fields: a.changes.keys().filter_map(crate::yaml::text).collect(),
            proposal: a.proposal.clone(),
            urgency: a.urgency.clone(),
            age_days: a.age_days,
            sentence: amend_sentence(&a.changes),
        });
    let hours = round1(task.remaining_hours());
    Row {
        id,
        slug: task.slug.clone(),
        title: task.title.clone(),
        course: task.course.clone(),
        source_tag: source_tag(meta),
        due: task.due.map(|d| d.strftime("%Y-%m-%dT%H:%M").to_string()),
        due_short: task.due.map(|d| short_date(d.date())),
        start_by: why.as_ref().map(|w| short_date(w.start_by)),
        slack_days: slack,
        overdue_start: slack.is_some_and(|s| s < 0.0),
        hours,
        progress_pct: task.progress,
        importance: task.importance,
        importance_reason: meta_text(meta, "importance_reason"),
        effort_source: meta_text(meta, "effort_source"),
        effort_confidence: meta_text(meta, "effort_confidence"),
        status: task.status.clone(),
        judged: meta.is_some_and(|m| is_judged(m, &[])),
        oversized: task.remaining_hours() > OVERSIZED_HOURS && meta_text(meta, "split_of").is_none(),
        amend_badge: badge,
        flag_text: None,
        merge_text: None,
        conflict_text: task.conflicts_with.as_ref().map(|c| format!("⚠ conflicts with {c}")),
        why: why
            .as_ref()
            .map(|w| {
                format!(
                    "start by {} — {}h of work needs {} working day{} at your {}h session cap, minus a {}-day buffer",
                    short_date(w.start_by),
                    py_float(round1(w.needed_hours)),
                    w.days_walked,
                    if w.days_walked == 1 { "" } else { "s" },
                    w.slice_cap.map(py_float).unwrap_or_else(|| "uncapped".into()),
                    w.buffer_days
                )
            })
            .unwrap_or_else(|| "no due date — never scheduled by deadline".to_string()),
        urgency: match slack {
            Some(s) if s < 0.0 => "od",
            Some(s) if s <= 2.0 => "soon",
            _ => "",
        }
        .to_string(),
    }
}

// flag_text/merge_text stay `None` in this plan: they are S3's markers and S3 has not run on
// Rust; the fields exist so the markup is not dead when it does (S2a contracts §5 decision 6).

#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct Group {
    pub label: String,
    pub count: i64,
    pub hours: f64,
    pub rows: Vec<Row>,
}
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct AmendCard {
    pub target_title: String,
    pub due_move: Option<String>,
    pub stale: bool,
    pub proposal_path: String,
    pub age_days: i64,
    pub urgency: String,
}
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct MustDo {
    pub groups: Vec<Group>,
    pub total_count: i64,
    pub total_hours: f64,
    pub amendments: Vec<AmendCard>,
    pub empty_text: Option<String>,
}

/// Sort key for a row that is not the `m/d` text it displays: `start_by` text does not sort
/// chronologically across months (`"9/1" < "8/24"` as strings), so the real dates are recomputed
/// here, paired with each row while sorting, and discarded once the sort is done.
fn sort_key(l: &Loaded, task: &Task) -> (Option<Date>, Option<jiff::civil::DateTime>) {
    (crate::scheduling::start_by_explained(task, &l.cal, DEFAULT_BUFFER_DAYS).map(|w| w.start_by), task.due)
}

/// `None` sorts after any real date — an undated task (or one with no due date, hence no
/// start-by) belongs at the end of its group, not the front, matching the sort the previous
/// string-keyed design gave it via a `"~"` fallback (ASCII-greater than every digit).
fn cmp_none_last<T: Ord>(a: &Option<T>, b: &Option<T>) -> std::cmp::Ordering {
    match (a, b) {
        (Some(x), Some(y)) => x.cmp(y),
        (Some(_), None) => std::cmp::Ordering::Less,
        (None, Some(_)) => std::cmp::Ordering::Greater,
        (None, None) => std::cmp::Ordering::Equal,
    }
}

/// Sort `(task, row)` pairs into `start_by, due` order and return just the rows.
///
/// Takes ownership of one already-paired sequence rather than a task slice and a row `Vec` sorted
/// against each other: two independently-managed sequences can silently disagree in length (a
/// `zip` truncates to the shorter, and the caller never sees it) in the one module whose stated
/// invariant is "never a silently shorter list" (F7/F14). A caller builds `paired` with a single
/// iterator over its tasks, producing each row and its sort key from the same `&Task` — there is
/// no second sequence left to come apart from the first.
fn sort_rows(l: &Loaded, paired: Vec<(&Task, Row)>) -> Vec<Row> {
    let mut keyed: Vec<_> = paired.into_iter().map(|(t, r)| (sort_key(l, t), r)).collect();
    keyed.sort_by(|(a, _), (b, _)| cmp_none_last(&a.0, &b.0).then_with(|| cmp_none_last(&a.1, &b.1)));
    keyed.into_iter().map(|(_, r)| r).collect()
}

fn group_label(task: &Task, today: Date) -> String {
    match task.due {
        None => "No due date".to_string(),
        Some(due) => {
            let gap = days_between(due.date(), today);
            if gap < 0 {
                "Overdue".into()
            } else if gap == 0 {
                "Due today".into()
            } else if gap <= 6 {
                format!("Due {}", WEEKDAYS[due.date().weekday().to_monday_zero_offset() as usize])
            } else {
                "Later".into()
            }
        }
    }
}

pub fn must_do(vault: &Path, l: &Loaded, today: Date, amendments: &[PendingAmendment]) -> MustDo {
    let (must, _) = partition_must_do(&l.ranked, today, &l.cal);
    // `must` is already in rank order; group boundaries follow it, rows are re-sorted per group
    // below. Each group's member list is a single `Vec<(&Task, Row)>`, pushed one pair at a time
    // from the same `task` this loop is already holding — never two lists built separately and
    // matched up after the fact.
    let mut labels: Vec<String> = Vec::new();
    let mut members: Vec<Vec<(&Task, Row)>> = Vec::new();
    for task in &must {
        let label = group_label(task, today);
        let r = row(l, task, today, amendments);
        match labels.iter().position(|l| *l == label) {
            Some(i) => members[i].push((task, r)),
            None => {
                labels.push(label);
                members.push(vec![(task, r)]);
            }
        }
    }
    let groups: Vec<Group> = labels
        .into_iter()
        .zip(members)
        .map(|(label, pairs)| {
            let rows = sort_rows(l, pairs);
            let hours = nz(round1(rows.iter().map(|r| r.hours).sum()));
            Group { label, count: rows.len() as i64, hours, rows }
        })
        .collect();
    let cards =
        amendments.iter().filter(|a| a.urgency == "increases").map(|a| amend_card(vault, a)).collect::<Vec<_>>();
    let total_count = must.len() as i64;
    let total_hours = nz(round1(must.iter().map(|t| t.remaining_hours()).sum()));
    let empty_text = (must.is_empty() && cards.is_empty() && l.unreadable.is_empty()).then(|| EMPTY_TEXT.must_do.to_string());
    MustDo { groups, total_count, total_hours, amendments: cards, empty_text }
}

/// The PENDING AMENDMENT block, as data: reads the target note for its real title and due exactly
/// as `render::amendment_block` (render.rs:147–193) does — same `resolve_amend_target`,
/// `pystr::read_text`, `models::split_frontmatter` — and the same "before"/"stale" rule: the
/// change set's `due` is a `{from, to}` mapping, `before` is the target's real due (falling back
/// to `from` when the note is unreadable or has no due), and `stale` fires only when the note's
/// real due and the proposal's stated `from` both exist and disagree.
fn amend_card(vault: &Path, a: &PendingAmendment) -> AmendCard {
    let target = crate::approvals::resolve_amend_target(vault, Some(&serde_yaml_ng::Value::String(a.target.clone())));
    let (title, target_due) = target
        .and_then(|p| crate::pystr::read_text(&p).ok())
        .and_then(|t| crate::models::split_frontmatter(&t).ok())
        .map(|(m, _)| {
            (
                meta_text(Some(&m), "title").unwrap_or_else(|| a.title.clone()),
                crate::approvals::as_note_datetime(crate::yaml::get(&m, "due")),
            )
        })
        .unwrap_or((a.title.clone(), None));
    let (stated, after) = match crate::yaml::get(&a.changes, "due") {
        Some(serde_yaml_ng::Value::Mapping(due)) => (
            crate::approvals::as_note_datetime(crate::yaml::get(due, "from")),
            crate::approvals::as_note_datetime(crate::yaml::get(due, "to")),
        ),
        _ => (None, None),
    };
    let before = target_due.or(stated);
    let (due_move, stale) = match (before, after) {
        (Some(before), Some(after)) => {
            (Some(crate::render::due_move(before, after)), target_due.is_some() && stated.is_some() && target_due != stated)
        }
        _ => (None, false),
    };
    AmendCard {
        target_title: crate::render::one_line(&title),
        due_move,
        stale,
        proposal_path: format!("approvals/{}.md", a.proposal),
        age_days: a.age_days,
        urgency: a.urgency.clone(),
    }
}

#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct RankedList {
    pub horizon: String,
    pub count: i64,
    pub hours: f64,
    pub rows: Vec<Row>,
    pub unreadable: Vec<String>,
    pub empty_text: Option<String>,
}

pub fn ranked_list(l: &Loaded, today: Date, horizon_name: &str, amendments: &[PendingAmendment]) -> RankedList {
    let paired: Vec<(&Task, Row)> = l
        .ranked
        .iter()
        .filter(|t| horizon_name == "all" || horizon(t, today) == horizon_name)
        .map(|t| (t, row(l, t, today, amendments)))
        .collect();
    let rows = sort_rows(l, paired);
    let unreadable = if horizon_name == "all" { l.unreadable.iter().map(|n| format!("tasks/{n}")).collect() } else { Vec::new() };
    let empty_text = (rows.is_empty() && unreadable.is_empty()).then(|| EMPTY_TEXT.list.to_string());
    RankedList {
        horizon: horizon_name.to_string(),
        count: rows.len() as i64,
        hours: round1(rows.iter().map(|r| r.hours).sum()),
        rows,
        unreadable,
        empty_text,
    }
}

#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct Commitment {
    pub name: String,
    pub hours: f64,
}
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct RecommendedRow {
    pub row: Row,
    pub take_hours: f64,
    pub of_hours: f64,
    pub why: String,
}
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct Recommended {
    pub commitments: Vec<Commitment>,
    pub rows: Vec<RecommendedRow>,
    pub spare_hours: f64,
    pub empty_text: Option<String>,
}

pub fn recommended(l: &Loaded, today: Date) -> Recommended {
    let commitments: Vec<Commitment> =
        l.planning.recurring_for(today).into_iter().map(|r| Commitment { name: r.name.clone(), hours: round1(r.hours) }).collect();
    let rows: Vec<RecommendedRow> = l
        .takes
        .iter()
        .map(|t| RecommendedRow {
            row: row(l, &t.task, today, &[]),
            take_hours: t.hours,
            of_hours: round1(t.task.remaining_hours()),
            why: format!(
                "{}h in the {} free block (had {}h; {}h left after) — capped by {}",
                py_float(t.hours),
                ordinal(t.block_index + 1),
                py_float(t.block_before),
                py_float(t.block_after),
                t.capped_by
            ),
        })
        .collect();
    let (must, _) = partition_must_do(&l.ranked, today, &l.cal);
    let must_raw: f64 = must.iter().map(|t| t.remaining_hours()).sum();
    let spare = nz(round1((l.cal.capacity(today) - must_raw).max(0.0)));
    let empty_text = (rows.is_empty() && commitments.is_empty()).then(|| EMPTY_TEXT.recommended.to_string());
    Recommended { commitments, rows, spare_hours: spare, empty_text }
}

fn ordinal(n: usize) -> String {
    match n {
        1 => "first".into(),
        2 => "second".into(),
        3 => "third".into(),
        n => format!("{n}th"),
    }
}

/// `DateTime` as the console's block times: `"08:00"`, `"12:50"` — always today's, never a full
/// timestamp.
fn hm(t: DateTime) -> String {
    t.strftime("%H:%M").to_string()
}

/// Hours between two same-day timestamps, `round2`'d like `capacity_breakdown`'s own arithmetic.
/// `Span::get_seconds()` (jiff 0.2) reads one component of a *balanced* span, not a total — the
/// faithful op is `DateTime::duration_since` into a `SignedDuration`, then `as_secs_f64() / 3600`
/// (`weekcal::Block::hours` takes the mirror-image `duration_until`; both land on the same value).
fn hours_between(a: DateTime, b: DateTime) -> f64 {
    round2(b.duration_since(a).as_secs_f64() / 3600.0)
}

#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct TakeView {
    pub title: String,
    pub hours: f64,
    pub of_hours: f64,
}
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct DayBlock {
    pub start: String,
    pub end: String,
    /// `"free"` / `"busy"` / `"class"`.
    pub kind: String,
    pub label: String,
    pub hours: f64,
    pub takes: Vec<TakeView>,
}
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct TheDay {
    pub all_day: Vec<String>,
    pub commitments: Vec<Commitment>,
    pub commitments_hours: f64,
    pub blocks: Vec<DayBlock>,
    pub open_hours: f64,
    pub empty_text: Option<String>,
}

/// The day laid out as one lane: free blocks (carrying today's takes), calendar busy time, commitment
/// blocks and the class gaps in the timetable — everything on the same timeline, in start order. A
/// recurring commitment (`planning.recurring`) spends the effort budget and renders in
/// `commitments`; it never appears as a block, because it spends the student's time, not a slot on
/// today's clock — a confirmed `commitments/` note is different: it IS a slot on today's clock, so
/// it draws one (§6.2). An all-day event never subtracts capacity (`weekcal::free_blocks` already
/// excludes it), so it surfaces only in `all_day`, read for display and nothing else.
pub fn the_day(l: &Loaded, today: Date) -> TheDay {
    let free = l.cal.free_blocks(today);
    let mut blocks: Vec<DayBlock> = free
        .iter()
        .enumerate()
        .map(|(i, b)| DayBlock {
            start: hm(b.start),
            end: hm(b.end),
            kind: "free".into(),
            label: "open".into(),
            hours: b.hours(),
            takes: l
                .takes
                .iter()
                .filter(|t| t.block_index == i)
                .map(|t| TakeView { title: t.task.title.clone(), hours: t.hours, of_hours: round1(t.task.remaining_hours()) })
                .collect(),
        })
        .collect();

    // §6.1/§6.3: every date-based use of day_start/day_end goes through `window(day)`, which
    // falls back to the template's when there is no confirmed planning day.
    let (window_start, window_end) = l.cal.window(today);
    let day_start = today.to_datetime(window_start);
    let day_end = today.to_datetime(window_end);

    // Each confirmed commitment span active today draws its own block (§6.2) — `kind: "class"`
    // for a class or lab, `"busy"` otherwise, `label` the note's title — clamped to the window and
    // dropped if wholly outside it. Two ranges are kept, but only for a span that IS drawn: a span
    // wholly outside the window `continue`s before either is recorded, so a Google event mirroring
    // an out-of-window commitment still shows (NEW-1) rather than being silently dropped by a
    // commitment that itself left no trace. `commitment_raw` (the UNCLAMPED span) matches a Google
    // event, which is never clamped either (I1: a class that straddles the window — the normal
    // case, since confirmed commitments come from Google series — must still de-duplicate against
    // its own event, which sits at the unclamped time); `commitment_drawn` (the clamped, drawn
    // range) matches a template class-gap block covering the identical span (M1).
    let mut commitment_raw: Vec<(DateTime, DateTime)> = Vec::new();
    let mut commitment_drawn: Vec<(DateTime, DateTime)> = Vec::new();
    for (start, end, span) in l.cal.spans_on(today) {
        let clamped_start = start.max(day_start);
        let clamped_end = end.min(day_end);
        if clamped_start >= clamped_end {
            continue; // wholly outside window(today): not drawn (R15), and no trace kept either
        }
        commitment_raw.push((start, end));
        let kind = if span.kind == "class" || span.kind == "lab" { "class" } else { "busy" };
        blocks.push(DayBlock { start: hm(clamped_start), end: hm(clamped_end), kind: kind.into(), label: span.title.clone(), hours: hours_between(clamped_start, clamped_end), takes: Vec::new() });
        commitment_drawn.push((clamped_start, clamped_end));
    }

    let mut all_day = Vec::new();
    for e in l.cal.events_on(today) {
        if e.all_day {
            all_day.push(e.title.clone());
        } else if !commitment_raw.iter().any(|(s, en)| *s == e.start && *en == e.end) {
            // A Google event identical to a commitment's own (unclamped) span is drawn once — the
            // commitment's own block above, not this one (R15, I1).
            blocks.push(DayBlock { start: hm(e.start), end: hm(e.end), kind: "busy".into(), label: e.title.clone(), hours: hours_between(e.start, e.end), takes: Vec::new() });
        }
    }
    // Classes are the gaps between the TEMPLATE's classes alone inside the day window —
    // `template_only_blocks` (weekcal.rs) has no commitment span folded in, so a confirmed club
    // is never mistaken for a class gap here (R15); the commitment spans draw their own blocks
    // above instead. A gap that exactly matches a commitment's own (clamped) span is dropped too
    // (M1, controller ruling): a class listed in both `week_template.yaml` and `commitments/` with
    // an equal span draws once, from the commitment, since it carries the title; an unequal span
    // draws both.
    let template = l.cal.template_only_blocks(today);
    let mut cursor = day_start;
    for b in &template {
        if b.start > cursor && !commitment_drawn.iter().any(|(s, e)| *s == cursor && *e == b.start) {
            blocks.push(DayBlock { start: hm(cursor), end: hm(b.start), kind: "class".into(), label: "class".into(), hours: hours_between(cursor, b.start), takes: Vec::new() });
        }
        cursor = b.end;
    }
    if cursor < day_end && !commitment_drawn.iter().any(|(s, e)| *s == cursor && *e == day_end) {
        blocks.push(DayBlock { start: hm(cursor), end: hm(day_end), kind: "class".into(), label: "class".into(), hours: hours_between(cursor, day_end), takes: Vec::new() });
    }
    blocks.sort_by(|a, b| a.start.cmp(&b.start).then(a.end.cmp(&b.end)));
    let commitments: Vec<Commitment> =
        l.planning.recurring_for(today).into_iter().map(|r| Commitment { name: r.name.clone(), hours: round1(r.hours) }).collect();
    let committed = nz(round1(commitments.iter().map(|c| c.hours).sum()));
    let empty_text = free.is_empty().then(|| EMPTY_TEXT.the_day.to_string());
    let open_hours = round1(l.cal.capacity(today) - committed);
    TheDay { all_day, commitments, commitments_hours: committed, blocks, open_hours: nz(open_hours.max(0.0)), empty_text }
}

#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct Bucket {
    pub date: String,
    pub weekday: String,
    pub hours: f64,
    pub items: i64,
    pub weekend: bool,
}
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct Ahead {
    pub buckets: Vec<Bucket>,
    pub peak: Bucket,
    pub clear_days: Vec<String>,
    pub undated_hours: f64,
    pub takeaway: String,
}

/// A fortnight of dated work, `today ..= today+13`, bucketed by the day it lands on: overdue work
/// lands on `today` (it is already late, not spread across the days it missed), everything else on
/// its own due date. A due date past the fortnight is simply outside every bucket — not folded into
/// the last one and not counted as undated, which is its own separate total (`undated_hours`).
pub fn ahead(l: &Loaded, today: Date) -> Ahead {
    let mut buckets: Vec<Bucket> = (0..14)
        .map(|i| {
            let d = add_days(today, i);
            Bucket {
                date: d.to_string(),
                weekday: WEEKDAYS[d.weekday().to_monday_zero_offset() as usize].into(),
                hours: 0.0,
                items: 0,
                weekend: d.weekday().to_monday_zero_offset() >= 5,
            }
        })
        .collect();
    let mut undated = 0.0;
    for t in &l.ranked {
        match t.due {
            None => undated += t.remaining_hours(),
            Some(due) => {
                let day = if due.date() < today { today } else { due.date() };
                let i = days_between(day, today);
                if (0..14).contains(&i) {
                    buckets[i as usize].hours += t.remaining_hours();
                    buckets[i as usize].items += 1;
                }
            }
        }
    }
    for b in &mut buckets {
        b.hours = nz(round1(b.hours));
    }
    let peak = buckets
        .iter()
        .fold(None::<&Bucket>, |best, b| match best { Some(p) if p.hours >= b.hours => Some(p), _ => Some(b) })
        .cloned()
        .unwrap();
    let clear_days: Vec<String> = buckets.iter().filter(|b| b.hours == 0.0).map(|b| b.date.clone()).collect();
    let takeaway = if peak.hours == 0.0 {
        "Nothing due in the next fortnight.".to_string()
    } else {
        let pd: Date = peak.date.parse().unwrap_or(today);
        format!(
            "{} {} {} is the wall — {}h across {} item{}, the heaviest day in a fortnight. {} clear day{}.",
            WEEKDAYS[pd.weekday().to_monday_zero_offset() as usize],
            pd.day(),
            MONTHS[pd.month() as usize - 1],
            py_float(peak.hours),
            peak.items,
            if peak.items == 1 { "" } else { "s" },
            clear_days.len(),
            if clear_days.len() == 1 { "" } else { "s" }
        )
    };
    Ahead { buckets, peak, clear_days, undated_hours: nz(round1(undated)), takeaway }
}

#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct ComingUp {
    pub uid: String,
    pub title: String,
    pub start: String,
    pub when: String,
    pub location: String,
    pub organizer: String,
}

/// The next five relevant events, read the same way `cli::run`'s events step assembles them for
/// `render::coming_up` (`cli.rs:352-362`): the roster and its verdict ledger, filtered to what's
/// still relevant, converted into `render::ComingUpEvent` — `start()` read before the `String`
/// fields move out of `DiscoveredEvent` — then trimmed to the picks `render::upcoming` makes for
/// `today.md`, so the console never shows a different five than the page does.
pub fn coming_up(vault: &Path, today: Date, now: DateTime) -> Vec<ComingUp> {
    let roster = crate::eventroster::read_roster(&vault.join("state").join("events.md"));
    let ledger = crate::eventledger::load_ledger(vault, None);
    let events: Vec<crate::render::ComingUpEvent> = crate::eventroster::relevant_events(&roster, &ledger)
        .into_iter()
        .map(|e| {
            let start = e.start();
            crate::render::ComingUpEvent { uid: e.uid, title: e.title, start, location: e.location, organizer: e.organizer }
        })
        .collect();
    crate::render::upcoming(&events, today, Some(now))
        .into_iter()
        .map(|e| ComingUp {
            when: format!("{} {} {}", WEEKDAYS[e.start.date().weekday().to_monday_zero_offset() as usize], short_date(e.start.date()), hm(e.start)),
            start: e.start.strftime("%Y-%m-%dT%H:%M").to_string(),
            uid: e.uid,
            title: e.title,
            location: e.location,
            organizer: e.organizer,
        })
        .collect()
}

/// The named empty states (S2 §9.6). One place; the page renders these strings, never its own.
pub struct EmptyText {
    pub active: &'static str,
    pub must_do: &'static str,
    pub recommended: &'static str,
    pub list: &'static str,
    pub the_day: &'static str,
    pub decisions: &'static str,
    pub info: &'static str,
    pub issues: &'static str,
    pub runs: &'static str,
    pub coming_up: &'static str,
    pub closed: &'static str,
    pub gauge: &'static str,
    pub offline: &'static str,
}
pub const EMPTY_TEXT: EmptyText = EmptyText {
    active: "Nothing active.",
    must_do: "Nothing is at risk today.",
    recommended: "No spare capacity today.",
    list: "Nothing here.",
    the_day: "No free block today.",
    decisions: "Queue clear — nothing waiting on you.",
    info: "Nothing to know right now.",
    issues: "No open issues.",
    runs: "No run recorded in the last 24 hours.",
    coming_up: "No events coming up.",
    closed: "Nothing closed this week.",
    gauge: "no history yet — the gauge fills from the next run",
    offline: "The engine did not answer — the page is as of the last state it returned.",
};

/// The strings `console.js` renders verbatim for the three sections whose data is a bare `Vec`
/// with no per-region `empty_text` field (S2 §9.6: the page renders these, never its own).
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct Texts {
    pub coming_up: &'static str,
    pub info: &'static str,
    pub closed: &'static str,
    pub offline: &'static str,
}

// -------------------------------------------------------------------------------------------
// The read-only approvals scan and the rails (console spec §4.4-§4.7): decisions,
// good_to_know, issues_panel, closed_this_week, runs_panel, gauge.
// -------------------------------------------------------------------------------------------

#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct Card {
    pub id: String,
    pub slug: String,
    pub title: String,
    pub kind: String,
    pub source_uid: Option<String>,
    pub first_proposed_at: Option<String>,
    pub age_days: i64,
    pub urgency: Option<String>,
    pub why: String,
    pub expires: Option<String>,
    pub snooze_until: Option<String>,
    pub target: Option<String>,
    pub changes: serde_json::Value,
}

#[derive(Debug, Clone, PartialEq)]
pub struct ApprovalsRead {
    pub cards: Vec<Card>,
    pub pending: i64,
    pub oldest_days: i64,
    pub awaiting_calendar: i64,
    pub snoozed: i64,
    pub events_in_digest: i64,
    pub amendments: Vec<PendingAmendment>,
}

/// A note's date-valued field, for *display* only — `expires`/`snooze_until` text on a [`Card`].
/// [`read_approvals`] never uses this for a classification decision; it calls
/// `approvals::as_date` there instead, because that is what `transition_note` itself calls.
fn note_date(meta: &Mapping, key: &str) -> Option<Date> {
    crate::approvals::as_note_datetime(crate::yaml::get(meta, key)).map(|d| d.date())
}

fn first_paragraph(body: &str) -> String {
    // The card's `why`: the first non-empty, non-heading paragraph of the proposal's body, on one line.
    body.split("\n\n")
        .map(str::trim)
        .find(|p| !p.is_empty() && !p.starts_with('#') && !p.starts_with("```") && !p.starts_with("- ["))
        .map(crate::render::one_line)
        .unwrap_or_default()
}

/// Read-only twin of `process_approvals`' classification (`approvals.rs:1161-1345`,
/// `transition_note`). Same folder, same `type: approval` filter, same status words — and no
/// transitions, no archiving, no writes: a note the pass would wake, expire or leave alone is
/// only ever read as what the pass *would* leave it as; nothing here calls `write_literals`,
/// `delete` or `expand_digest`.
///
/// Three places this deliberately does not read like a naive port, each pinned to the pass by
/// line number (console plan 1, Task 7, controller rulings R15-R18):
///
/// * **R15 — snoozed** (approvals.rs:1247-1268). `transition_note` wakes a `snoozed` note in
///   place — rewrites `status` to `pending` on disk — the moment its `snooze_until` parses and is
///   `<= today`, then falls through and processes the rest of the function as `pending` in the
///   same call. So here: `snooze_until <= today` counts as pending (a card, `pending`, `oldest`,
///   an `AmendmentEntry` if `kind: amend`); `snooze_until > today` is `snoozed` (deferred); an
///   absent or unparseable date is neither — the pass only warns (`"snoozed without
///   snooze_until"` / `"bad snooze_until"`) and leaves the note exactly as it was, so the read
///   model counts it nowhere either.
/// * **R16 — expired** (approvals.rs:1270-1292). The pass archives a pending note the instant its
///   `expires` parses and is `< today`. The read model excludes that note entirely — showing a
///   card for a note that will be gone after the next `rank` would be a lie by the time Quinn
///   reads it. An unparseable-but-present `expires` only warns there (`"bad expires"`) and the
///   note stays pending; so it does here.
/// * **R17/R18 — oldest and the digest** (approvals.rs:1296-1335). `events_in_digest` accumulates
///   `proposal_weight` (the digest's `events:` payload count, never 1) and the digest never
///   becomes a `Card` — a batch UI is a header count, not a row. But `oldest` is the minimum
///   `first_proposed_at` (falling back to `proposed_at`) over **every** pending note, digests
///   included; a first sketch that `continue`d past a digest before touching `oldest` would let
///   the single oldest proposal in the vault hide behind a `continue`. `tests/fixtures/vault-full`
///   carries no digest note — `emit_digest` only creates one *during* a `rank` run, from the
///   roster — so `events_in_digest` is `0` reading the pristine fixture, never the golden page's
///   post-`rank` `3`.
pub fn read_approvals(vault: &Path, today: Date) -> ApprovalsRead {
    let mut out = ApprovalsRead {
        cards: Vec::new(),
        pending: 0,
        oldest_days: 0,
        awaiting_calendar: 0,
        snoozed: 0,
        events_in_digest: 0,
        amendments: Vec::new(),
    };
    let folder = vault.join("approvals");
    let Ok(rd) = std::fs::read_dir(&folder) else { return out };
    let mut paths: Vec<_> = rd
        .filter_map(|e| e.ok().map(|e| e.path()))
        .filter(|p| p.extension().is_some_and(|x| x == "md"))
        .collect();
    paths.sort();
    let mut oldest: Option<Date> = None;

    for path in paths {
        let Ok(text) = crate::pystr::read_text(&path) else { continue };
        let Ok((meta, body)) = crate::models::split_frontmatter(&text) else { continue };
        if meta_text(Some(&meta), "type").as_deref() != Some("approval") {
            continue;
        }
        let mut status = meta_text(Some(&meta), "status").unwrap_or_default();
        let kind = meta_text(Some(&meta), "kind").unwrap_or_default();
        let slug = path.file_stem().map(|s| s.to_string_lossy().to_string()).unwrap_or_default();

        // R15: wake a past-due snooze in place, in memory only.
        if status == "snoozed" {
            match crate::approvals::as_date(crate::yaml::get(&meta, "snooze_until")) {
                Some(wake) if wake <= today => status = "pending".to_string(),
                Some(_) => {
                    out.snoozed += 1;
                    continue;
                }
                None => continue, // absent or unparseable: the pass warns and leaves it be
            }
        }

        if status == "approved" {
            if kind == "calendar-event" {
                out.awaiting_calendar += 1;
            }
            continue;
        }
        if status != "pending" {
            continue; // executed/rejected/refused/expired notes settle out of approvals/ already
        }

        // R16: a parsed, past `expires` is what the pass would archive next run.
        if let Some(expires) = crate::approvals::as_date(crate::yaml::get(&meta, "expires")) {
            if expires < today {
                continue;
            }
        }

        let first = crate::approvals::as_date(crate::yaml::get(&meta, "first_proposed_at"))
            .or_else(|| crate::approvals::as_date(crate::yaml::get(&meta, "proposed_at")));

        if kind == "events-digest" {
            out.events_in_digest += crate::approvals::proposal_weight(&meta);
        } else {
            out.pending += 1;
        }
        // R17: every pending note's `first` feeds `oldest`, digests included.
        if let Some(f) = first {
            oldest = Some(oldest.map_or(f, |o| o.min(f)));
        }

        let age_days = first.map(|f| days_between(today, f).max(0)).unwrap_or(0);
        let urgency = (kind == "amend").then(|| crate::approvals::derive_urgency(&meta));

        if kind == "amend" {
            let changes = match crate::yaml::get(&meta, "changes") {
                Some(serde_yaml_ng::Value::Mapping(m)) => m.clone(),
                _ => Mapping::new(),
            };
            // approvals.rs:1317-1320 — title falls back to the file stem when absent or blank.
            let title = match crate::yaml::get(&meta, "title") {
                Some(v) if crate::pystr::yaml_truthy(v) => crate::pystr::yaml_str(v),
                _ => slug.clone(),
            };
            out.amendments.push(PendingAmendment {
                target: meta_text(Some(&meta), "target").unwrap_or_default(),
                title,
                urgency: urgency.clone().unwrap_or_default(),
                proposal: slug.clone(),
                changes,
                age_days,
            });
        }

        if kind != "events-digest" {
            // R18: the digest is a header count, never a row — one batch UI must never smuggle a
            // second card past the "one proposal, one decision" budget rule it already owns.
            out.cards.push(Card {
                id: meta_text(Some(&meta), "id").unwrap_or_else(|| slug.clone()),
                slug: slug.clone(),
                title: meta_text(Some(&meta), "title").unwrap_or_default(),
                kind: kind.clone(),
                source_uid: meta_text(Some(&meta), "source_uid"),
                first_proposed_at: first.map(|d| d.to_string()),
                age_days,
                urgency,
                why: first_paragraph(&body),
                expires: note_date(&meta, "expires").map(|d| d.to_string()),
                snooze_until: note_date(&meta, "snooze_until").map(|d| d.to_string()),
                target: meta_text(Some(&meta), "target"),
                changes: crate::yaml::get(&meta, "changes")
                    .map(crate::yaml::to_json)
                    .unwrap_or(serde_json::Value::Null),
            });
        }
    }

    out.oldest_days = oldest.map(|o| days_between(today, o).max(0)).unwrap_or(0);
    out.cards.sort_by(|a, b| a.first_proposed_at.cmp(&b.first_proposed_at).then(a.slug.cmp(&b.slug)));
    out
}

#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct Decisions {
    pub cards: Vec<Card>,
    pub pending: i64,
    pub oldest_days: i64,
    pub budget_total: i64,
    pub budget_used: i64,
    pub budget_remaining: i64,
    pub deferred: i64,
    pub awaiting_calendar: i64,
    pub events_in_digest: i64,
    /// The neutral/decreasing amendments the deck does not show as a card — `must_do` only ever
    /// cards an `increases` amendment (`amendments.iter().filter(|a| a.urgency == "increases")`),
    /// so this is the count of the rest, kept so the page can still say "N more waiting quietly"
    /// instead of pretending they do not exist.
    pub hidden_amendments: i64,
    pub empty_text: Option<String>,
}

/// Controller ruling R19 (console plan 1, Task 7 fix 1): `empty_text` fires only when there is
/// truly nothing waiting on Quinn — `ar.cards.is_empty()` alone is not enough, because a pending
/// `events-digest` never becomes a card (it is a header count, `events_in_digest`) yet is still N
/// decisions by CLAUDE.md's own rule ("one proposal = one decision... an events-digest counts its
/// event count, not 1"). Showing "Queue clear — nothing waiting on you." beside
/// `events_in_digest: 3` would have the page contradict itself in the same breath, so the empty
/// state requires both `cards` and `events_in_digest` to be empty.
pub fn decisions(vault: &Path, today: Date, ar: &ApprovalsRead, planning: &PlanningConfig) -> Decisions {
    let used = crate::approvals::count_proposals_created(vault, today);
    let hidden = ar.amendments.iter().filter(|a| a.urgency != "increases").count() as i64;
    Decisions {
        cards: ar.cards.clone(),
        pending: ar.pending,
        oldest_days: ar.oldest_days,
        budget_total: planning.daily_approval_budget,
        budget_used: used,
        budget_remaining: (planning.daily_approval_budget - used).max(0),
        deferred: ar.snoozed,
        awaiting_calendar: ar.awaiting_calendar,
        events_in_digest: ar.events_in_digest,
        hidden_amendments: hidden,
        empty_text: (ar.cards.is_empty() && ar.events_in_digest == 0).then(|| EMPTY_TEXT.decisions.to_string()),
    }
}

#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct InfoItem {
    pub id: String,
    pub title: String,
    pub kind: String,
    pub close_key: Option<String>,
    pub expires: Option<String>,
    pub opened_at: Option<String>,
    pub path: String,
}

pub fn good_to_know(vault: &Path) -> Vec<InfoItem> {
    crate::info::list_info(vault)
        .iter()
        .map(|m| InfoItem {
            id: meta_text(Some(m), "id").unwrap_or_default(),
            title: meta_text(Some(m), "title").unwrap_or_default(),
            kind: meta_text(Some(m), "kind").unwrap_or_default(),
            close_key: meta_text(Some(m), "close_key"),
            expires: meta_text(Some(m), "expires"),
            opened_at: meta_text(Some(m), "opened_at"),
            path: meta_text(Some(m), "path").unwrap_or_default(),
        })
        .collect()
}

#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct IssueRow {
    pub id: String,
    pub title: String,
    pub categories: Vec<String>,
    pub target: Option<String>,
    pub opened_at: Option<String>,
    pub path: String,
}
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct IssuesPanel {
    pub open_count: i64,
    pub rows: Vec<IssueRow>,
    pub empty_text: Option<String>,
}

pub fn issues_panel(vault: &Path) -> IssuesPanel {
    let rows: Vec<IssueRow> = crate::issues::list_issues(vault, true)
        .iter()
        .map(|m| IssueRow {
            id: meta_text(Some(m), "id").unwrap_or_default(),
            title: meta_text(Some(m), "title").unwrap_or_default(),
            categories: match crate::yaml::get(m, "categories") {
                Some(serde_yaml_ng::Value::Sequence(s)) => s.iter().filter_map(crate::yaml::text).collect(),
                _ => Vec::new(),
            },
            target: meta_text(Some(m), "target"),
            opened_at: meta_text(Some(m), "opened_at"),
            path: meta_text(Some(m), "path").unwrap_or_default(),
        })
        .collect();
    IssuesPanel { open_count: rows.len() as i64, empty_text: rows.is_empty().then(|| EMPTY_TEXT.issues.to_string()), rows }
}

#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct Closed {
    pub id: Option<String>,
    pub title: String,
    pub mark: String,
    pub when: String,
    pub path: String,
}

/// Last 7 days of the journal: progress → 100, status → done/archived, or a delete (S2 §7.3).
///
/// `lapsed` (S2 §7.3) needs S3's `status: lapsed`, which no note carries yet; the mark set here is
/// what the journal can prove today, and this doc comment is the anatomy entry that says so.
pub fn closed_this_week(vault: &Path, now: DateTime, journal: &mut crate::journal::Journal) -> Vec<Closed> {
    let since = crate::scheduling::add_days(now.date(), -7).to_string();
    // Both sides compared at second precision: a record's `ts` carries milliseconds and a `Z`
    // the formatted bound does not, so comparing the raw strings would make a record from the
    // very same second as `now` sort after it (a false "not closed yet").
    let now_bound = now.strftime("%Y-%m-%dT%H:%M:%S").to_string();
    // One row per note even when a single write closes it on two fields (progress AND status,
    // one journal moment) — keyed by path, but insertion order is kept (first sight of a note),
    // not the map's own key order, so a caller with no duplicates still sees chronological rows.
    let mut order: Vec<String> = Vec::new();
    let mut by_path: BTreeMap<String, Closed> = BTreeMap::new();
    for r in journal.read(Some(&since), None) {
        let ts = r.get("ts").and_then(|v| v.as_str()).unwrap_or("");
        let ts_bound = ts.get(..19).unwrap_or(ts);
        if ts_bound > now_bound.as_str() {
            continue; // a record after `now` is not closed yet
        }
        let field = r.get("field").and_then(|v| v.as_str()).unwrap_or("");
        let op = r.get("op").and_then(|v| v.as_str()).unwrap_or("");
        let new = r.get("new");
        let closes = op == "delete"
            || (field == "progress" && new.and_then(|v| v.as_i64()) == Some(100))
            || (field == "status" && matches!(new.and_then(|v| v.as_str()), Some("done") | Some("archived")));
        if !closes {
            continue;
        }
        let actor = r.get("actor").and_then(|v| v.as_str()).unwrap_or("");
        let mark = if crate::journal::is_human(actor) { "me" } else if actor.starts_with("agent:") { "agent" } else { "system" };
        let path = r.get("path").and_then(|v| v.as_str()).unwrap_or("").to_string();
        if !by_path.contains_key(&path) {
            order.push(path.clone());
        }
        by_path.insert(
            path.clone(),
            Closed {
                id: r.get("id").and_then(|v| v.as_str()).map(str::to_string),
                title: path.rsplit('/').next().unwrap_or("").trim_end_matches(".md").to_string(),
                mark: mark.into(),
                when: local_stamp(vault, ts),
                path,
            },
        );
    }
    order.into_iter().filter_map(|p| by_path.remove(&p)).collect()
}

#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct RunRow {
    pub run_id: String,
    pub runner: String,
    pub started: Option<String>,
    pub ended: Option<String>,
    pub result: String,
    pub summary: String,
}
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct ExpectedRow {
    pub runner: String,
    pub due: String,
    pub status: String,
}
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct RunsPanel {
    pub recent: Vec<RunRow>,
    pub expected: Vec<ExpectedRow>,
    pub warn_count: i64,
    pub empty_text: Option<String>,
    /// `expected_status`'s error, verbatim, if the schedule could not be read at all — a
    /// malformed `config/runners.yaml` entry, never a missing file (that is `Ok(vec![])`
    /// upstream).
    pub warnings: Vec<String>,
}

pub fn runs_panel(vault: &Path, now_ts: jiff::Timestamp) -> RunsPanel {
    let since = (now_ts - jiff::SignedDuration::from_hours(48)).to_zoned(jiff::tz::TimeZone::UTC).date().to_string();
    let mut runs = crate::runs::Runs::new(vault);
    let mut by_id: BTreeMap<String, RunRow> = BTreeMap::new();
    for r in runs.read(Some(&since)) {
        let id = r.get("run_id").and_then(|v| v.as_str()).unwrap_or("").to_string();
        if id.is_empty() {
            continue;
        }
        let row = by_id.entry(id.clone()).or_insert_with(|| RunRow {
            run_id: id.clone(),
            runner: r.get("runner").and_then(|v| v.as_str()).unwrap_or("").into(),
            started: None,
            ended: None,
            result: "running".into(),
            summary: String::new(),
        });
        let ts = r.get("ts").and_then(|v| v.as_str()).map(|t| local_stamp(vault, t));
        match r.get("phase").and_then(|v| v.as_str()) {
            Some("start") => row.started = ts,
            Some("end") => {
                row.ended = ts;
                row.result = r.get("result").and_then(|v| v.as_str()).unwrap_or("").into();
                row.summary = r.get("summary").and_then(|v| v.as_str()).unwrap_or("").into();
            }
            _ => {}
        }
    }
    let mut recent: Vec<RunRow> = by_id.into_values().collect();
    recent.sort_by(|a, b| b.started.cmp(&a.started));
    recent.truncate(10);
    let (expected_rows, warnings) = match crate::runs::expected_status(vault, now_ts) {
        Ok(rows) => (rows, Vec::new()),
        Err(e) => (Vec::new(), vec![e]),
    };
    let expected: Vec<ExpectedRow> = expected_rows
        .into_iter()
        .map(|s| ExpectedRow { runner: s.runner, due: local_stamp(vault, &s.due), status: s.status.to_string() })
        .collect();
    let warn_count = recent.iter().filter(|r| r.result != "ok" && r.result != "running").count() as i64
        + expected.iter().filter(|e| e.status == "missing" || e.status == "crashed").count() as i64;
    let empty_text = recent.iter().all(|r| r.ended.is_none()).then(|| EMPTY_TEXT.runs.to_string());
    RunsPanel { recent, expected, warn_count, empty_text, warnings }
}

#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct GaugePoint {
    pub date: String,
    pub runway_days: i64,
    pub deficit_hours: f64,
    pub must_count: i64,
}
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct Gauge {
    pub points: Vec<GaugePoint>,
    pub direction: String,
}

/// Reads `runway_days`, `deficit_hours_x10`, `must_count` from the `tasks` step of each run's
/// records over the last 14 days. Nothing writes them until the cutover closes (spec §4.5), so
/// today this is always `no-history`; the reader ships now so the writer is a one-line change.
pub fn gauge(vault: &Path, now_ts: jiff::Timestamp) -> Gauge {
    let since =
        (now_ts - jiff::SignedDuration::from_hours(14 * 24)).to_zoned(jiff::tz::TimeZone::UTC).date().to_string();
    let mut runs = crate::runs::Runs::new(vault);
    let mut points = Vec::new();
    for r in runs.read(Some(&since)) {
        if r.get("phase").and_then(|v| v.as_str()) != Some("step") || r.get("name").and_then(|v| v.as_str()) != Some("tasks") {
            continue;
        }
        let Some(counts) = r.get("counts").and_then(|v| v.as_object()) else { continue };
        let (Some(rw), Some(dx10), Some(mc)) = (
            counts.get("runway_days").and_then(|v| v.as_i64()),
            counts.get("deficit_hours_x10").and_then(|v| v.as_i64()),
            counts.get("must_count").and_then(|v| v.as_i64()),
        ) else {
            continue;
        };
        points.push(GaugePoint {
            date: r.get("ts").and_then(|v| v.as_str()).unwrap_or("").chars().take(10).collect(),
            runway_days: rw,
            deficit_hours: dx10 as f64 / 10.0,
            must_count: mc,
        });
    }
    let direction = match (points.first(), points.last()) {
        (Some(a), Some(b)) if points.len() >= 2 => {
            let d = (b.runway_days - a.runway_days) as f64 - (b.deficit_hours - a.deficit_hours);
            if d > 0.5 {
                "gaining"
            } else if d < -0.5 {
                "losing"
            } else {
                "holding"
            }
        }
        _ => "no-history",
    };
    Gauge { points, direction: direction.into() }
}

// -------------------------------------------------------------------------------------------
// Task 8: delta, note_detail, build_state, state_json — one payload, never a write.
// -------------------------------------------------------------------------------------------

#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct DeltaRecord {
    pub ts: String,
    pub op: String,
    pub field: Option<String>,
    pub path: String,
    pub id: Option<String>,
    pub actor: String,
    pub via: String,
    pub old: serde_json::Value,
    pub new: serde_json::Value,
    pub run_id: Option<String>,
    pub text: String,
}
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct Delta {
    pub since: Option<String>,
    pub since_kind: String,
    pub summary: String,
    pub records: Vec<DeltaRecord>,
    /// `true` when `records` was cut down to [`DELTA_RECORD_CAP`] newest — `summary` still
    /// reflects every record since `since`, only the display list is bounded.
    pub truncated: bool,
}

/// One journal record into one sentence a person reads, not a diff dump: `set`/`supersede` name
/// old and new; anything else just names the operation. `who` reads `via` because `detect_external`
/// (ingest.rs) journals an Obsidian hand-edit as `actor: quinn, via: external` — the phrase the
/// page needs is "an Obsidian edit", not "quinn via external".
fn describe(r: &crate::ledger::Record) -> DeltaRecord {
    let s = |k: &str| r.get(k).and_then(|v| v.as_str()).map(str::to_string);
    let (op, path, actor, via) = (
        s("op").unwrap_or_default(),
        s("path").unwrap_or_default(),
        s("actor").unwrap_or_default(),
        s("via").unwrap_or_default(),
    );
    let field = s("field");
    let old = r.get("old").cloned().unwrap_or(serde_json::Value::Null);
    let new = r.get("new").cloned().unwrap_or(serde_json::Value::Null);
    let who = if via == "external" { "an Obsidian edit".to_string() } else { format!("{actor} via {via}") };
    let text = match (op.as_str(), field.as_deref()) {
        ("set", Some(f)) => format!("{path}: {f} {} → {} ({who})", crate::ledger::dumps_value(&old), crate::ledger::dumps_value(&new)),
        ("supersede", Some(f)) => format!("{path}: {f} — {} won over {} ({who})", crate::ledger::dumps_value(&new), crate::ledger::dumps_value(&old)),
        (op, _) => format!("{path}: {op} ({who})"),
    };
    DeltaRecord { ts: s("ts").unwrap_or_default(), op, field, path, id: s("id"), actor, via, old, new, run_id: s("run_id"), text }
}

/// "Since I last looked" (spec §2), falling back to the newest run `end`; grouped one phrase per
/// (op, field, actor class). `seen_at` is the page's own last-read stamp (round-tripped by the
/// caller, never written here); with none given, the newest run `end` record stands in for it, and
/// with no run at all (`vault-full`'s pristine `state/runs/`) there is nothing to diff against.
pub fn delta(vault: &Path, now: DateTime, seen_at: Option<&str>, journal: &mut crate::journal::Journal) -> Delta {
    let _ = now;
    let (since, since_kind) = match seen_at {
        Some(s) => (Some(s.to_string()), "seen"),
        None => match newest_end_ts(vault) {
            Some(ts) => (Some(ts), "run"),
            None => (None, "none"),
        },
    };
    // R21: pass `since` whole, never sliced here -- `JsonlLedger::read` already derives its
    // own day bound via `first_chars(since, 10)` (ledger.rs) and applies the record-level
    // `ts < since` filter itself. A `&s[..10]` byte-slice on caller-supplied input (Task 9's
    // `--seen-at`, Task 10's persisted stamp) panics on anything under 10 bytes or not a char
    // boundary at 10 -- this function must never crash the page over a malformed timestamp.
    let mut records: Vec<DeltaRecord> = match &since {
        Some(s) => journal
            .read(Some(s.as_str()), None)
            .iter()
            .filter(|r| r.get("ts").and_then(|v| v.as_str()).is_some_and(|t| t > s.as_str()))
            .map(describe)
            .collect(),
        None => Vec::new(),
    };
    records.sort_by(|a, b| a.ts.cmp(&b.ts));
    let mut groups: BTreeMap<(String, String, String), i64> = BTreeMap::new();
    for r in &records {
        let class = if crate::journal::is_human(&r.actor) { "you" } else if r.actor.starts_with("agent:") { "agent" } else { "system" };
        *groups.entry((r.op.clone(), r.field.clone().unwrap_or_default(), class.into())).or_insert(0) += 1;
    }
    let summary = groups
        .iter()
        .map(|((op, field, class), n)| if field.is_empty() { format!("{n} {op} by {class}") } else { format!("{n} {field} {op} by {class}") })
        .collect::<Vec<_>>()
        .join(" · ");
    // The summary above covers every record since `since`; only the display list is bounded —
    // a page must never be asked to render an unbounded diff.
    let truncated = records.len() > DELTA_RECORD_CAP;
    if truncated {
        records.drain(..records.len() - DELTA_RECORD_CAP);
    }
    Delta { since, since_kind: since_kind.into(), summary, records, truncated }
}

fn newest_end_ts(vault: &Path) -> Option<String> {
    let mut runs = crate::runs::Runs::new(vault);
    runs.read(None)
        .into_iter()
        .filter(|r| r.get("phase").and_then(|v| v.as_str()) == Some("end"))
        .filter_map(|r| r.get("ts").and_then(|v| v.as_str()).map(str::to_string))
        .max()
}

#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct NoteDetail {
    pub id: String,
    pub path: String,
    pub slug: String,
    pub folder: String,
    pub frontmatter: serde_json::Value,
    pub body: String,
    pub judgment: serde_json::Value,
    pub history: Vec<DeltaRecord>,
    pub row: Option<Row>,
}

/// One note, read for the console's detail pane: its frontmatter as JSON (the one sanctioned raw
/// dump — `state_json` is the only *serialiser*, this is a *read*), its body, its `judgment:`
/// block, its full journal history newest-first, and its row when it is an active task (so the
/// pane can show the same start-by/slack/why the list does, not a second computation of them).
/// Re-loads the vault via [`load`] to build that row — one extra full read per detail click, never
/// cached, so the pane can never show a row staler than the note it is attached to.
pub fn note_detail(vault: &Path, id: &str, today: Date, journal: &mut crate::journal::Journal) -> Option<NoteDetail> {
    let path = crate::ids::build_index(vault).remove(id)?;
    let text = crate::pystr::read_text(&path).ok()?;
    let (meta, body) = crate::models::split_frontmatter(&text).ok()?;
    let rel = crate::ids::rel(vault, &path);
    let folder = rel.split('/').next().unwrap_or("").to_string();
    let slug = path.file_stem().map(|s| s.to_string_lossy().to_string()).unwrap_or_default();
    let history: Vec<DeltaRecord> = journal.records_for(id, None).iter().rev().map(describe).collect();
    // R22: the same amendments a list row would badge — a task's drawer must not disagree
    // with the row that opened it about a pending amendment. `read_approvals` is a read-only
    // scan (proven write-free by `build_state_never_writes`), never `process_approvals`.
    let row = if folder == "tasks" {
        let l = load(vault, today);
        let ar = read_approvals(vault, today);
        l.ranked.iter().find(|t| t.slug == slug).map(|t| row(&l, t, today, &ar.amendments))
    } else {
        None
    };
    Some(NoteDetail {
        id: id.to_string(),
        path: rel,
        slug,
        folder,
        frontmatter: crate::yaml::to_json(&serde_yaml_ng::Value::Mapping(meta.clone())),
        body,
        judgment: crate::yaml::get(&meta, "judgment").map(crate::yaml::to_json).unwrap_or(serde_json::Value::Null),
        history,
        row,
    })
}

#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct Empty {
    pub active: bool,
    pub must_do: Option<String>,
    pub recommended: Option<String>,
    pub the_day: Option<String>,
    /// Always present, never null: the page reaches its empty branch by three routes (no
    /// gauge, `no-history`, or no points) and only one of them is the direction, so an
    /// Option here would render the literal "null" on the other two.
    pub gauge: &'static str,
}

#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct State {
    pub schema: u32,
    pub view: String,
    pub today: String,
    pub now: String,
    pub revision: String,
    pub topline: Topline,
    pub delta: Delta,
    pub verdict: Verdict,
    pub meter: Meter,
    pub gauge: Gauge,
    pub must_do: MustDo,
    pub recommended: Recommended,
    pub list: Option<RankedList>,
    pub the_day: TheDay,
    pub decisions: Decisions,
    pub ahead: Ahead,
    pub coming_up: Vec<ComingUp>,
    pub good_to_know: Vec<InfoItem>,
    pub closed_this_week: Vec<Closed>,
    pub runs_panel: RunsPanel,
    pub issues_panel: IssuesPanel,
    pub nav_counts: NavCounts,
    pub empty: Empty,
    pub texts: Texts,
    pub unreadable: Vec<String>,
    /// §6.4's "what moved", on the today view only; omitted (not `null`) when there is none, so a
    /// vault with no planning-day note serialises exactly as before.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub moved: Option<crate::commitments::Moved>,
    /// Every `Journal::warnings()` this build touched (a malformed journal line) — collected
    /// once, after every builder that opens the shared `Journal` has run.
    pub warnings: Vec<String>,
}

/// Assembles one page's worth of read model from one vault read: every builder above, called with
/// the arguments it actually takes, never a second load of `tasks/` or a second approvals scan.
/// `horizon` is `None` for `Today` and every rail-only view (`Decisions`, `GoodToKnow`, `Issues`,
/// `Runs`) — only the four horizon views (`Overdue`/`Week`/`Later`/`AllActive`) carry a `list`.
pub fn build_state(vault: &Path, view: View, today: Date, now: &jiff::Zoned, seen_at: Option<&str>) -> State {
    build_state_with(vault, view, today, now, seen_at, None)
}

/// `surface --view today --window '<flow sequence>'` (§6.4 "Preview", the phase-2 window editor's
/// data source): the today view under a proposed planning window, with `moved` against the
/// **current** window. `window` is validated as a planning-day note's `window` is, strictly; an
/// invalid one, or any view but today, is an `Err` the CLI turns into exit 2. Writes nothing.
pub fn build_state_preview(
    vault: &Path,
    view: View,
    today: Date,
    now: &jiff::Zoned,
    seen_at: Option<&str>,
    window: &str,
) -> Result<State, String> {
    if view != View::Today {
        return Err(format!("--window previews the today view, not {:?}", view.name()));
    }
    let window = crate::commitments::parse_window(window)?;
    Ok(build_state_with(vault, view, today, now, seen_at, Some(&window)))
}

fn build_state_with(
    vault: &Path,
    view: View,
    today: Date,
    now: &jiff::Zoned,
    seen_at: Option<&str>,
    preview: Option<&Window>,
) -> State {
    let l = load_with(vault, today, preview);
    let civil = now.datetime();
    // `closed_this_week`'s "not closed yet" bound compares against journal `ts` values, which
    // are UTC ISO strings — `civil` above is the vault-LOCAL wall clock (it feeds the page's
    // displayed `now`), so this is a separate conversion, not a reuse of `civil`.
    let now_utc = now.timestamp().to_zoned(jiff::tz::TimeZone::UTC).datetime();
    let mut journal = crate::journal::Journal::new(vault);
    let ar = read_approvals(vault, today);
    let good = good_to_know(vault);
    let issues = issues_panel(vault);
    let runs = runs_panel(vault, now.timestamp());
    let horizon = match view {
        View::Overdue => Some("overdue"),
        View::Week => Some("week"),
        View::Later => Some("later"),
        View::AllActive => Some("all"),
        _ => None,
    };
    let must = must_do(vault, &l, today, &ar.amendments);
    let rec = recommended(&l, today);
    let day = the_day(&l, today);
    let mut state = State {
        schema: 1,
        view: view.name().into(),
        today: today.to_string(),
        now: civil.strftime("%Y-%m-%dT%H:%M").to_string(),
        revision: String::new(),
        topline: topline(vault, &l, today),
        delta: delta(vault, civil, seen_at, &mut journal),
        verdict: verdict(&l, today),
        meter: meter(&l, today),
        gauge: gauge(vault, now.timestamp()),
        empty: Empty { active: !l.ranked.is_empty(), must_do: must.empty_text.clone(), recommended: rec.empty_text.clone(), the_day: day.empty_text.clone(), gauge: EMPTY_TEXT.gauge },
        texts: Texts { coming_up: EMPTY_TEXT.coming_up, info: EMPTY_TEXT.info, closed: EMPTY_TEXT.closed, offline: EMPTY_TEXT.offline },
        must_do: must,
        recommended: rec,
        list: horizon.map(|h| ranked_list(&l, today, h, &ar.amendments)),
        the_day: day,
        decisions: decisions(vault, today, &ar, &l.planning),
        ahead: ahead(&l, today),
        coming_up: coming_up(vault, today, civil),
        nav_counts: nav_counts(&l, today, ar.pending, good.len() as i64, issues.open_count, runs.warn_count),
        good_to_know: good,
        closed_this_week: closed_this_week(vault, now_utc, &mut journal),
        runs_panel: runs,
        issues_panel: issues,
        unreadable: l.unreadable.iter().map(|n| format!("tasks/{n}")).collect(),
        moved: if view == View::Today { l.moved.clone() } else { None },
        warnings: Vec::new(),
    };
    state.warnings = journal.warnings().to_vec();
    state.revision = revision(&state);
    state
}

/// sha1 over the payload minus the fields that move on every call (`now`, `revision`) — identical
/// means the page repaints nothing (S2 §9.7). `delta.since` stays in: a changed window (a new
/// `seen_at`, or a fresh run's `end` record standing in for one) is a legitimate reason to repaint,
/// not the clock noise `now`/`revision` themselves are.
fn revision(state: &State) -> String {
    use sha1::Digest;
    let mut v = serde_json::to_value(state).unwrap_or(serde_json::Value::Null);
    if let Some(o) = v.as_object_mut() {
        o.remove("now");
        o.remove("revision");
    }
    let mut h = sha1::Sha1::new();
    h.update(crate::ledger::dumps_value(&v).as_bytes());
    let digest = h.finalize();
    digest.iter().take(6).map(|b| format!("{b:02x}")).collect()
}

/// `revision`, made `pub` under this name: the CLI's `--build-sha` override mutates
/// `state.topline.engine_build` after `build_state` returns, so it must recompute the revision
/// over the overridden payload rather than serve the stale one `build_state` already stamped.
pub fn revision_of(state: &State) -> String {
    revision(state)
}

/// The one serialiser: `State` → Python-shaped JSON (sorted keys, `", "`/`": "` separators) via
/// `ledger::dumps_value`, so the CLI's `state.json`, a future reference file, and any test all read
/// the same bytes `serde_json::to_string` would not produce.
pub fn state_json(state: &State) -> String {
    crate::ledger::dumps_value(&serde_json::to_value(state).unwrap_or(serde_json::Value::Null))
}

#[cfg(test)]
mod tests {
    use super::*;
    use jiff::civil::Date;
    use std::path::Path;

    pub(super) const TODAY: Date = Date::constant(2026, 8, 28);
    pub(super) fn full() -> Loaded { load(Path::new("tests/fixtures/vault-full"), TODAY) }

    fn copy_dir_all(src: &Path, dst: &Path) -> std::io::Result<()> {
        std::fs::create_dir_all(dst)?;
        for entry in std::fs::read_dir(src)? {
            let entry = entry?;
            let target = dst.join(entry.file_name());
            if entry.file_type()?.is_dir() {
                copy_dir_all(&entry.path(), &target)?;
            } else {
                std::fs::copy(entry.path(), &target)?;
            }
        }
        Ok(())
    }

    /// A private scratch copy of `tests/fixtures/vault-full`, safe for a test to mutate — remove
    /// notes, corrupt a journal file, rewrite `config/runners.yaml`. Every call gets its own
    /// directory (an atomic counter, not just the process id) so tests running in parallel
    /// threads never see each other's edits.
    ///
    /// R-P3: the checked-in fixture may lack `config/runners.yaml` or a vault timezone; if the
    /// copy lacks either, plant the values this plan uses throughout (cloud 13:00/20:00 UTC
    /// grace 45; local 12:00/18:00 America/Chicago grace 20) — in the scratch copy only, never
    /// under `tests/fixtures/`.
    pub(super) fn fixture_full() -> std::path::PathBuf {
        static COUNTER: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(0);
        let n = COUNTER.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
        let dir = std::env::temp_dir().join(format!("qo-fixture-full-{}-{n}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        copy_dir_all(Path::new("tests/fixtures/vault-full"), &dir).unwrap();
        let runners = dir.join("config").join("runners.yaml");
        if !runners.exists() {
            std::fs::write(
                &runners,
                "runners:\n  - name: cloud\n    times: [\"13:00\", \"20:00\"]\n    tz: UTC\n    grace_minutes: 45\n  - name: local\n    times: [\"12:00\", \"18:00\"]\n    tz: America/Chicago\n    grace_minutes: 20\n",
            )
            .unwrap();
        }
        let ingest = dir.join("config").join("ingest.yaml");
        let text = std::fs::read_to_string(&ingest).unwrap_or_default();
        if !text.contains("timezone:") {
            std::fs::write(&ingest, format!("timezone: America/Chicago\n{text}")).unwrap();
        }
        dir
    }

    /// `2026-08-28`, the date every other fixture-full test already pins.
    pub(super) fn pinned_today() -> Date {
        TODAY
    }

    /// `2026-08-28T09:00` America/Chicago — the same moment `zoned(TODAY)` below builds, and the
    /// one the reference regeneration recipe passes as `--now`.
    pub(super) fn pinned_now() -> jiff::Zoned {
        zoned(TODAY)
    }

    #[test]
    fn view_parses_the_nine_names_and_nothing_else() {
        for (name, view) in [("today", View::Today), ("overdue", View::Overdue), ("week", View::Week), ("later", View::Later), ("all", View::AllActive), ("decisions", View::Decisions), ("good-to-know", View::GoodToKnow), ("issues", View::Issues), ("runs", View::Runs)] {
            assert_eq!(View::parse(name), Some(view));
            assert_eq!(View::parse(name).unwrap().name(), name);
        }
        assert_eq!(View::parse("Today"), None);
    }

    #[test]
    fn load_reads_the_full_fixture_the_way_the_runner_does() {
        let l = full();
        assert_eq!(l.ranked.len(), 5);
        assert!(l.unreadable.is_empty());
        assert_eq!(l.ranked[0].slug, "cs-100-hw-01");
        assert_eq!(l.takes.len(), 2);
        assert_eq!(l.cal.capacity(TODAY), 6.0);
    }

    #[test]
    fn verdict_matches_the_golden_page() {
        let v = verdict(&full(), TODAY);
        assert_eq!((v.runway_days, v.word.as_str(), v.active), (-4.0, "behind", 5));
        assert_eq!((v.capacity_hours, v.template_hours, v.calendar_hours, v.must_hours), (6.0, 8.25, 2.25, 3.0));
        assert_eq!(v.capacity_text, "Capacity today: 6.0h (template 8.25h − 2.25h calendar)");
        assert!(v.overflow_sentence.is_none(), "3.0h fits in 6.0h");
        assert!(v.headline.contains("behind"), "{}", v.headline);
        assert!(v.lede.contains("3.0") && v.lede.contains("6.0"), "{}", v.lede);
    }

    #[test]
    fn meter_arithmetic_on_the_golden_day() {
        let m = meter(&full(), TODAY);
        assert_eq!((m.block_minutes, m.fit_blocks, m.spill_blocks, m.capped), (30, 12, 0, 0));
        assert_eq!(m.spill_hours, 0.0);
        assert!(m.spill_items.is_empty());
    }

    #[test]
    fn meter_caps_spill_at_120_blocks_and_says_how_many_more() {
        let mut l = full();
        // 30 overdue tasks of 5h each: 150h of must-do against 6h.
        let proto = l.ranked[0].clone();
        l.ranked = (0..30).map(|i| { let mut t = proto.clone(); t.slug = format!("t{i}"); t.title = format!("T{i}"); t.effort_hours = 5.0; t.progress = 0; t }).collect();
        let m = meter(&l, TODAY);
        assert_eq!(m.spill_blocks, 120);
        assert_eq!(m.capped, (150.0f64 * 2.0).ceil() as i64 - 12 - 120);
        assert_eq!(m.spill_items.len(), 29, "the first 5h task fits partly; the rest spill");
    }

    #[test]
    fn nav_counts_carry_hours_and_partition_the_dated_tasks() {
        let l = full();
        let n = nav_counts(&l, TODAY, 2, 0, 0, 0);
        assert_eq!((n.today.count, n.today.hours), (2, 3.0));
        assert_eq!((n.overdue.count, n.overdue.hours), (2, 3.0));
        assert_eq!((n.week.count, n.week.hours), (2, 5.3));   // GN 103 8/31, BUI 101 9/1
        assert_eq!((n.later.count, n.later.hours), (1, 12.0));
        assert_eq!(n.undated.count, 0);
        assert_eq!((n.all.count, n.all.hours), (5, 20.3));
        assert_eq!(n.decisions.count, 2);
    }

    #[test]
    fn topline_names_the_runner_and_the_runway() {
        let t = topline(Path::new("tests/fixtures/vault-full"), &full(), TODAY);
        assert_eq!(t.date, "Fri 28 Aug");
        assert_eq!((t.runway_days, t.status_word.as_str(), t.active), (-4.0, "behind", 5));
        assert_eq!(t.engine_build.is_some(), crate::BUILD_SHA.is_some());
        // `vault-full`'s state/runs/ holds only .gitkeep — no `end` record to read.
        assert!(t.generated_at.is_none() && t.generated_by.is_none());
    }

    #[test]
    fn round1_ties_to_even_like_round2_and_todays_own_one_decimal_display() {
        assert_eq!(round1(0.25), 0.2);
        assert_eq!(round1(1.25), 1.2);
    }

    /// A vault whose `state/runs/` holds a real `end` record: `topline` must read it back through
    /// the vault's own timezone, not UTC — `20:55:12.345Z` in `America/Chicago` (CDT, UTC-5 on
    /// this date) is `15:55`, the same arithmetic `local_now`'s tests pin in `cli.rs`. The record
    /// is written through `runs::Runs::append` — the seam `ledger::only_this_module_opens_ledger_files`
    /// polices — never by naming a `.jsonl` path here.
    #[test]
    fn topline_reads_the_newest_end_record_in_the_vaults_own_zone() {
        let dir = std::env::temp_dir().join(format!(
            "qo-surface-topline-{}-{:?}",
            std::process::id(),
            std::thread::current().id()
        ));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(dir.join("config")).unwrap();
        std::fs::create_dir_all(dir.join("tasks")).unwrap();
        std::fs::copy(
            "tests/fixtures/vault-s1/config/planning.yaml",
            dir.join("config").join("planning.yaml"),
        )
        .unwrap();
        std::fs::copy(
            "tests/fixtures/vault-s1/config/week_template.yaml",
            dir.join("config").join("week_template.yaml"),
        )
        .unwrap();
        crate::pystr::write_text(&dir.join("config").join("ingest.yaml"), "timezone: America/Chicago\n").unwrap();

        let runs = crate::runs::Runs::new(&dir);
        let mut start: crate::ledger::Record = serde_json::from_value(serde_json::json!({
            "phase": "start", "ts": "2026-09-01T17:00:03.000Z", "runner": "manual",
            "run_id": "manual-2026-09-01T17:00:03Z",
        }))
        .unwrap();
        runs.append(&mut start).unwrap();
        let mut end: crate::ledger::Record = serde_json::from_value(serde_json::json!({
            "phase": "end", "ts": "2026-09-01T20:55:12.345Z", "runner": "manual",
            "run_id": "manual-2026-09-01T17:00:03Z", "result": "ok",
            "summary": "today.md refreshed (5 active)",
        }))
        .unwrap();
        runs.append(&mut end).unwrap();

        let l = load(&dir, TODAY);
        let t = topline(&dir, &l, TODAY);
        assert_eq!(t.generated_at.as_deref(), Some("2026-09-01 15:55"));
        assert_eq!(t.generated_by.as_deref(), Some("manual"));

        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn rows_carry_start_by_slack_and_hours_from_the_golden_page() {
        let l = full();
        let rows: Vec<Row> = l.ranked.iter().map(|t| row(&l, t, TODAY, &[])).collect();
        assert_eq!(rows.iter().map(|r| r.start_by.as_deref().unwrap()).collect::<Vec<_>>(), ["8/24", "8/26", "8/29", "8/30", "9/19"]);
        assert_eq!(rows.iter().map(|r| r.overdue_start).collect::<Vec<_>>(), [true, true, false, false, false]);
        assert_eq!(rows.iter().map(|r| r.slack_days.unwrap()).collect::<Vec<_>>(), [-4.0, -2.0, 1.0, 2.0, 22.0]);
        assert_eq!(rows.iter().map(|r| r.hours).collect::<Vec<_>>(), [2.5, 0.5, 2.8, 2.5, 12.0]);
        assert_eq!(rows[4].oversized, true, "PH 106 has 12.0h left");
        assert_eq!(rows[0].urgency, "od");
        assert!(rows[0].why.contains("start by 8/24"), "{}", rows[0].why);
    }

    #[test]
    fn must_do_groups_carry_hours_and_the_pending_amendment() {
        let l = full();
        let amendments = vec![PendingAmendment { target: "tasks/ph-106-exam-1-prep.md".into(), title: "PH 106 Exam 1 prep".into(), urgency: "increases".into(), proposal: "amend-ph-106-due".into(), changes: crate::yaml::mapping_of("due:\n  from: 2026-09-25T09:00\n  to: 2026-09-22T09:00"), age_days: 0 }];
        let md = must_do(Path::new("tests/fixtures/vault-full"), &l, TODAY, &amendments);
        assert_eq!((md.total_count, md.total_hours), (2, 3.0));
        assert_eq!(md.groups.iter().map(|g| (g.label.as_str(), g.count, g.hours)).collect::<Vec<_>>(), [("Overdue", 2, 3.0)]);
        assert_eq!(md.amendments.len(), 1);
        assert_eq!(md.amendments[0].due_move.as_deref(), Some("9/25 → 9/22 (3 days EARLIER)"));
        assert_eq!(md.amendments[0].proposal_path, "approvals/amend-ph-106-due.md");
        assert!(md.empty_text.is_none());
    }

    #[test]
    fn a_pending_amendment_badges_the_row_it_would_change_and_no_other() {
        let l = full();
        let amendments = vec![PendingAmendment { target: "tasks/ph-106-exam-1-prep.md".into(), title: "PH 106 Exam 1 prep".into(), urgency: "increases".into(), proposal: "amend-ph-106-due".into(), changes: crate::yaml::mapping_of("due:\n  from: 2026-09-25T09:00\n  to: 2026-09-22T09:00"), age_days: 0 }];
        let ph = l.ranked.iter().find(|t| t.slug == "ph-106-exam-1-prep").unwrap();
        let badge = row(&l, ph, TODAY, &amendments).amend_badge.expect("badge");
        assert_eq!((badge.fields.as_slice(), badge.proposal.as_str(), badge.urgency.as_str()), (&["due".to_string()][..], "amend-ph-106-due", "increases"));
        assert!(badge.sentence.contains("9/22"), "{}", badge.sentence);
        assert!(row(&l, &l.ranked[0], TODAY, &amendments).amend_badge.is_none());
    }

    #[test]
    fn every_task_is_reachable_across_the_horizon_lists_and_undated_is_its_own_group() {
        let mut l = full();
        let mut undated = l.ranked[0].clone();
        undated.slug = "undated".into(); undated.due = None;
        l.ranked.push(undated);
        let mut seen: Vec<String> = ["overdue", "week", "later", "undated"].iter().flat_map(|h| ranked_list(&l, TODAY, h, &[]).rows.into_iter().map(|r| r.slug)).collect();
        seen.sort();
        let mut all: Vec<String> = l.ranked.iter().map(|t| t.slug.clone()).collect();
        all.sort();
        assert_eq!(seen, all);
        assert_eq!(ranked_list(&l, TODAY, "undated", &[]).rows.len(), 1);
        assert_eq!(ranked_list(&l, TODAY, "all", &[]).count, 6);
    }

    #[test]
    fn sort_is_start_by_then_due_not_importance() {
        let l = full();
        let list = ranked_list(&l, TODAY, "all", &[]);
        let keys: Vec<(Option<String>, Option<String>)> = list.rows.iter().map(|r| (r.start_by.clone(), r.due.clone())).collect();
        let mut sorted = keys.clone();
        sorted.sort();
        assert_eq!(keys, sorted, "rows must already be in start_by, due order");
    }

    #[test]
    fn recommended_shows_the_take_and_what_is_left_like_the_golden_page() {
        let r = recommended(&full(), TODAY);
        assert_eq!(r.rows.iter().map(|x| (x.row.slug.as_str(), x.take_hours, x.of_hours)).collect::<Vec<_>>(), [("gn-103-hausaufgaben-2026-08-31", 2.0, 2.8), ("bui-101-reading-augustine-iv-vi", 1.0, 2.5)]);
        assert_eq!(r.spare_hours, 3.0);
        assert!(r.commitments.is_empty() && r.empty_text.is_none());
    }

    #[test]
    fn the_day_on_the_golden_friday() {
        let d = the_day(&full(), TODAY);
        let shape: Vec<(&str, &str, &str, f64)> = d.blocks.iter().map(|b| (b.start.as_str(), b.end.as_str(), b.kind.as_str(), b.hours)).collect();
        assert_eq!(shape, [
            ("08:00", "09:00", "free", 1.0), ("09:00", "10:00", "busy", 1.0), ("10:00", "12:00", "free", 2.0),
            ("12:00", "12:50", "class", 0.83), ("13:00", "13:45", "class", 0.75), ("14:00", "15:00", "busy", 1.0), ("15:00", "18:00", "free", 3.0),
        ]);
        assert_eq!(d.blocks[1].label, "PH 106 Lecture");
        let last = d.blocks.last().unwrap();
        assert_eq!(last.takes.iter().map(|t| (t.title.as_str(), t.hours, t.of_hours)).collect::<Vec<_>>().len(), 2);
        assert_eq!((last.takes[0].hours, last.takes[0].of_hours), (2.0, 2.8));
        assert_eq!((d.open_hours, d.commitments_hours), (6.0, 0.0));
        assert!(d.all_day.is_empty() && d.empty_text.is_none());
    }

    #[test]
    fn commitments_are_commitments_and_reduce_open_hours_but_never_appear_as_blocks() {
        let mut l = full();
        l.planning.recurring.push(crate::planning::Recurring { name: "Gym".into(), hours: 1.5, days: vec!["fri".into()] });
        let d = the_day(&l, TODAY);
        assert_eq!(d.commitments, vec![Commitment { name: "Gym".into(), hours: 1.5 }]);
        assert_eq!(d.open_hours, 4.5);
        assert!(d.blocks.iter().all(|b| b.label != "Gym"));
    }

    // -----------------------------------------------------------------------------------------
    // P17: the_day's commitment blocks and window (spec §6.2).
    // -----------------------------------------------------------------------------------------

    /// A confirmed commitment note, written straight into `vault/commitments/`, never re-dumped —
    /// same shape `commitments.rs`'s own tests use.
    fn commitment_note(vault: &Path, file: &str, front: &str) {
        std::fs::create_dir_all(vault.join("commitments")).unwrap();
        std::fs::write(
            vault.join("commitments").join(file),
            format!("---\n{front}---\n\nInvented for a test.\n"),
        )
        .unwrap();
    }

    #[test]
    fn the_day_draws_a_club_as_busy_with_its_title_not_as_class() {
        let v = fixture_full();
        commitment_note(
            &v,
            "chess-club.md",
            "id: cmt_0000000001\ntype: commitment\nkind: club\ntitle: \"Chess Club\"\n\
             meets: [{days: [fri], start: \"16:00\", end: \"17:00\"}]\n\
             source_uid: \"gcal-series:chess\"\nstatus: confirmed\n",
        );
        let l = load(&v, TODAY);
        let d = the_day(&l, TODAY);
        let block = d.blocks.iter().find(|b| b.start == "16:00").expect("the club's block");
        assert_eq!((block.kind.as_str(), block.label.as_str(), block.end.as_str()), ("busy", "Chess Club", "17:00"));
        // R15's exact regression: if the gap walk ever went back to `template_blocks` (which folds
        // commitment spans into its busy list), the club would ALSO paint a generic "class" gap at
        // this span, alongside the titled "busy" block above.
        assert!(!d.blocks.iter().any(|b| b.kind == "class" && b.start == "16:00"), "{:?}", d.blocks);
    }

    #[test]
    fn a_class_commitment_is_drawn_as_class_with_its_title() {
        let v = fixture_full();
        commitment_note(
            &v,
            "extra-lab.md",
            "id: cmt_0000000002\ntype: commitment\nkind: lab\ntitle: \"Extra Lab\"\n\
             meets: [{days: [fri], start: \"10:15\", end: \"10:45\"}]\n\
             source_uid: \"gcal-series:lab\"\nstatus: confirmed\n",
        );
        let l = load(&v, TODAY);
        let d = the_day(&l, TODAY);
        let block = d.blocks.iter().find(|b| b.start == "10:15").expect("the lab's block");
        assert_eq!((block.kind.as_str(), block.label.as_str(), block.end.as_str()), ("class", "Extra Lab", "10:45"));
    }

    #[test]
    fn a_commitment_straddling_the_window_is_clamped_and_one_outside_is_not_drawn() {
        let v = fixture_full();
        // week_template.yaml's window is 08:00-18:00 on Friday.
        commitment_note(
            &v,
            "early-bird.md",
            "id: cmt_0000000003\ntype: commitment\nkind: club\ntitle: \"Early Bird\"\n\
             meets: [{days: [fri], start: \"07:00\", end: \"08:30\"}]\n\
             source_uid: \"gcal-series:early\"\nstatus: confirmed\n",
        );
        commitment_note(
            &v,
            "night-owl.md",
            "id: cmt_0000000004\ntype: commitment\nkind: club\ntitle: \"Night Owl\"\n\
             meets: [{days: [fri], start: \"19:00\", end: \"20:00\"}]\n\
             source_uid: \"gcal-series:night\"\nstatus: confirmed\n",
        );
        let l = load(&v, TODAY);
        let d = the_day(&l, TODAY);
        let block = d.blocks.iter().find(|b| b.label == "Early Bird").expect("the clamped block");
        assert_eq!((block.start.as_str(), block.end.as_str()), ("08:00", "08:30"));
        assert!(d.blocks.iter().all(|b| b.label != "Night Owl"), "wholly outside the window: never drawn");
    }

    /// NEW-1 (re-review of fix round 1): a commitment wholly outside the window leaves no trace
    /// (`commitment_raw`/`commitment_drawn` never record it), so a Google event mirroring it must
    /// not be swallowed by the event de-dup — it is the only visible sign that time is spoken for.
    #[test]
    fn a_google_event_matching_an_out_of_window_commitment_is_still_drawn() {
        let v = fixture_full();
        // week_template.yaml's window is 08:00-18:00 on Friday; this commitment is wholly outside.
        commitment_note(
            &v,
            "night-owl.md",
            "id: cmt_0000000012\ntype: commitment\nkind: club\ntitle: \"Night Owl\"\n\
             meets: [{days: [fri], start: \"19:00\", end: \"20:00\"}]\n\
             source_uid: \"gcal-series:night\"\nstatus: confirmed\n",
        );
        let mut l = load(&v, TODAY);
        let mirror = crate::weekcal::CalEvent {
            title: "Night Owl (calendar copy)".into(),
            start: TODAY.at(19, 0, 0, 0),
            end: TODAY.at(20, 0, 0, 0),
            all_day: false,
        };
        l.cal = WeekCalendar::for_vault(&v, vec![mirror]);
        let d = the_day(&l, TODAY);
        assert!(d.blocks.iter().all(|b| b.label != "Night Owl"), "the commitment itself is still never drawn");
        let event = d.blocks.iter().find(|b| b.label == "Night Owl (calendar copy)").expect("the event still draws");
        assert_eq!((event.kind.as_str(), event.start.as_str(), event.end.as_str()), ("busy", "19:00", "20:00"));
    }

    #[test]
    fn a_google_event_identical_to_a_commitment_is_drawn_once() {
        let v = fixture_full();
        commitment_note(
            &v,
            "cs-extra.md",
            "id: cmt_0000000005\ntype: commitment\nkind: class\ntitle: \"CS Extra Session\"\n\
             meets: [{days: [fri], start: \"10:00\", end: \"10:45\"}]\n\
             source_uid: \"gcal-series:extra\"\nstatus: confirmed\n",
        );
        let mut l = load(&v, TODAY);
        let duplicate = crate::weekcal::CalEvent {
            title: "CS Extra Session (calendar copy)".into(),
            start: TODAY.at(10, 0, 0, 0),
            end: TODAY.at(10, 45, 0, 0),
            all_day: false,
        };
        l.cal = WeekCalendar::for_vault(&v, vec![duplicate]);
        let d = the_day(&l, TODAY);
        let matching: Vec<_> = d.blocks.iter().filter(|b| b.start == "10:00" && b.end == "10:45").collect();
        assert_eq!(matching.len(), 1, "one block, not two: {:?}", d.blocks);
        assert_eq!(matching[0].label, "CS Extra Session", "the commitment's own block wins, the event's copy is dropped");
    }

    /// I1: the de-duplication must match the event against the commitment's UNCLAMPED span, not
    /// its clamped, drawn one — otherwise a straddling class that is also on Google (the normal
    /// case, since confirmed commitments come from Google series) draws twice: the commitment
    /// clamped to the window, and the event at its own, now-unmatched, times.
    #[test]
    fn a_google_event_matching_a_straddling_commitments_raw_span_is_drawn_once_clamped() {
        let v = fixture_full();
        // week_template.yaml's window is 08:00-18:00 on Friday; this class starts before it.
        commitment_note(
            &v,
            "early-class.md",
            "id: cmt_0000000009\ntype: commitment\nkind: class\ntitle: \"Early Class\"\n\
             meets: [{days: [fri], start: \"07:30\", end: \"08:45\"}]\n\
             source_uid: \"gcal-series:earlyclass\"\nstatus: confirmed\n",
        );
        let mut l = load(&v, TODAY);
        let duplicate = crate::weekcal::CalEvent {
            title: "Early Class (calendar copy)".into(),
            start: TODAY.at(7, 30, 0, 0),
            end: TODAY.at(8, 45, 0, 0),
            all_day: false,
        };
        l.cal = WeekCalendar::for_vault(&v, vec![duplicate]);
        let d = the_day(&l, TODAY);
        assert!(d.blocks.iter().all(|b| b.label != "Early Class (calendar copy)"), "{:?}", d.blocks);
        let matching: Vec<_> = d.blocks.iter().filter(|b| b.label == "Early Class").collect();
        assert_eq!(matching.len(), 1, "one block, not two: {:?}", d.blocks);
        assert_eq!((matching[0].start.as_str(), matching[0].end.as_str()), ("08:00", "08:45"), "clamped to the window");
    }

    /// M1 (controller ruling): a class in both `week_template.yaml` and `commitments/` with an
    /// EQUAL span is drawn once — the commitment wins, since it carries the title.
    #[test]
    fn a_class_matching_the_template_exactly_is_drawn_once_and_the_commitment_wins() {
        let v = fixture_full();
        // week_template.yaml's Friday has PH 106 at 13:00-13:45.
        commitment_note(
            &v,
            "ph-106-confirmed.md",
            "id: cmt_0000000010\ntype: commitment\nkind: class\ntitle: \"PH 106 (confirmed)\"\n\
             meets: [{days: [fri], start: \"13:00\", end: \"13:45\"}]\n\
             source_uid: \"gcal-series:ph106\"\nstatus: confirmed\n",
        );
        let l = load(&v, TODAY);
        let d = the_day(&l, TODAY);
        let matching: Vec<_> = d.blocks.iter().filter(|b| b.start == "13:00" && b.end == "13:45").collect();
        assert_eq!(matching.len(), 1, "one block, not two: {:?}", d.blocks);
        assert_eq!(matching[0].label, "PH 106 (confirmed)", "the commitment wins, the generic gap is dropped");
    }

    /// M1: an UNEQUAL span (a different end than the template's class) is not deduplicated — both
    /// the template's generic gap and the commitment's own, differently-bounded block are drawn.
    #[test]
    fn a_class_with_an_unequal_span_from_the_template_is_drawn_alongside_it() {
        let v = fixture_full();
        // week_template.yaml's Friday has CS 100 at 12:00-12:50; this commitment ends earlier.
        commitment_note(
            &v,
            "cs-100-early-half.md",
            "id: cmt_0000000011\ntype: commitment\nkind: class\ntitle: \"CS 100 (early half)\"\n\
             meets: [{days: [fri], start: \"12:00\", end: \"12:30\"}]\n\
             source_uid: \"gcal-series:cs100early\"\nstatus: confirmed\n",
        );
        let l = load(&v, TODAY);
        let d = the_day(&l, TODAY);
        assert!(d.blocks.iter().any(|b| b.start == "12:00" && b.end == "12:50" && b.label == "class"), "the template's own gap is unaffected: {:?}", d.blocks);
        assert!(d.blocks.iter().any(|b| b.start == "12:00" && b.end == "12:30" && b.label == "CS 100 (early half)"), "the commitment's own span is drawn too: {:?}", d.blocks);
    }

    #[test]
    fn the_day_uses_the_planning_window() {
        let v = fixture_full();
        commitment_note(
            &v,
            "planning-day.md",
            "id: cmt_0000000006\ntype: commitment\nkind: planning-day\nstatus: confirmed\n\
             window: [{days: [fri], start: \"07:00\", end: \"20:00\"}]\n",
        );
        let l = load(&v, TODAY);
        let d = the_day(&l, TODAY);
        // vault-full's Friday: PH 106 Lecture 09:00-10:00 and Advising appointment 14:00-15:00
        // (state/calendar.md), classes at 12:00-12:50 and 13:00-13:45 (week_template.yaml) — the
        // same shape `the_day_on_the_golden_friday` pins for the template's 08:00-18:00 window,
        // now under the widened 07:00-20:00 one: every block's bounds are checked, not just the
        // first start and last end, so a stray gap at the old 08:00/18:00 boundary would fail this.
        let shape: Vec<(&str, &str, &str, f64)> = d.blocks.iter().map(|b| (b.start.as_str(), b.end.as_str(), b.kind.as_str(), b.hours)).collect();
        assert_eq!(shape, [
            ("07:00", "09:00", "free", 2.0), ("09:00", "10:00", "busy", 1.0), ("10:00", "12:00", "free", 2.0),
            ("12:00", "12:50", "class", 0.83), ("13:00", "13:45", "class", 0.75), ("14:00", "15:00", "busy", 1.0), ("15:00", "20:00", "free", 5.0),
        ]);
    }

    #[test]
    fn ahead_is_a_fortnight_overdue_lands_on_today_and_the_peak_breaks_ties_earliest() {
        let a = ahead(&full(), TODAY);
        assert_eq!(a.buckets.len(), 14);
        assert_eq!((a.buckets[0].date.as_str(), a.buckets[0].hours, a.buckets[0].items), ("2026-08-28", 3.0, 2));
        assert_eq!(a.buckets[3].hours, 2.8);  // GN 103 due 8/31
        assert_eq!(a.buckets[4].hours, 2.5);  // BUI 101 due 9/1
        assert_eq!((a.peak.date.as_str(), a.peak.hours), ("2026-08-28", 3.0));
        assert_eq!(a.undated_hours, 0.0);
        assert_eq!(a.clear_days.len(), 11);
        assert!(a.buckets[1].weekend && a.buckets[2].weekend);
        assert!(a.takeaway.contains("Fri 28 Aug"), "{}", a.takeaway);
    }

    #[test]
    fn ahead_reports_undated_hours_separately() {
        let mut l = full();
        let mut u = l.ranked[0].clone(); u.slug = "u".into(); u.due = None; u.effort_hours = 4.0; u.progress = 0;
        l.ranked.push(u);
        assert_eq!(ahead(&l, TODAY).undated_hours, 4.0);
    }

    #[test]
    fn coming_up_is_the_golden_pages_three_events() {
        let now = TODAY.at(9, 0, 0, 0);
        let events = coming_up(Path::new("tests/fixtures/vault-full"), TODAY, now);
        assert_eq!(events.iter().map(|e| e.when.as_str()).collect::<Vec<_>>(), ["Wed 9/2 16:00", "Fri 9/4 12:00", "Thu 9/10 18:00"]);
        assert_eq!(events[0].title, "Undergraduate Research Symposium");
        assert_eq!(events[0].organizer, "Office of Undergraduate Research");
    }

    // -----------------------------------------------------------------------------------------
    // Task 7: read_approvals, decisions, good_to_know, issues_panel, closed_this_week,
    // runs_panel, gauge.
    // -----------------------------------------------------------------------------------------

    // Controller ruling R18 (2026-09-02): the fixture's two approval notes carry no
    // `events-digest`; `emit_digest` only creates one *during* a `rank` run, from the roster.
    // A read-only scan of the pristine fixture sees `events_in_digest == 0` — the golden page's
    // "3 in today's digest" is Task 6/`rank`'s doing, not this scan's.
    #[test]
    fn read_approvals_matches_the_golden_header_without_writing() {
        let vault = Path::new("tests/fixtures/vault-full");
        let before = crate::approvals::count_proposals_created(vault, TODAY);
        let ar = read_approvals(vault, TODAY);
        assert_eq!((ar.pending, ar.oldest_days, ar.events_in_digest, ar.awaiting_calendar, ar.snoozed), (2, 1, 0, 0, 0));
        assert_eq!(ar.amendments.len(), 1);
        assert_eq!((ar.amendments[0].proposal.as_str(), ar.amendments[0].urgency.as_str(), ar.amendments[0].age_days), ("amend-ph-106-due", "increases", 0));
        assert_eq!(crate::approvals::count_proposals_created(vault, TODAY), before, "read-only");
        assert_eq!(ar.cards.len(), 2, "one task proposal and one amendment; the digest is a header count, not a card");
        assert!(ar.cards.windows(2).all(|w| w[0].first_proposed_at <= w[1].first_proposed_at), "ordered by first_proposed_at");
    }

    #[test]
    fn decisions_carries_the_budget_and_the_hidden_amendment_count() {
        let vault = Path::new("tests/fixtures/vault-full");
        let l = full();
        let ar = read_approvals(vault, TODAY);
        let d = decisions(vault, TODAY, &ar, &l.planning);
        assert_eq!((d.budget_total, d.budget_used, d.budget_remaining), (15, crate::approvals::count_proposals_created(vault, TODAY), 15 - crate::approvals::count_proposals_created(vault, TODAY)));
        assert_eq!(d.hidden_amendments, 0);
        assert!(d.empty_text.is_none());
    }

    #[test]
    fn empty_rails_say_so_by_name() {
        let vault = Path::new("tests/fixtures/vault-full");
        assert!(good_to_know(vault).is_empty());
        let ip = issues_panel(vault);
        assert_eq!((ip.open_count, ip.empty_text.as_deref()), (0, Some(EMPTY_TEXT.issues)));
    }

    // The brief's sketch of this test built the day file by hand (`format!("{day}.jsonl")`), which
    // trips `ledger::tests::only_this_module_opens_ledger_files` — `.jsonl` names a ledger file
    // directly, and that guard exists precisely so only `ledger.rs` ever spells the extension.
    // `cli.rs`'s `a_record_from_the_future_warns_and_names_the_device` is the established pattern
    // for planting a journal record at an exact `ts`: `NewRecord` + `make_record` + `append`.
    #[test]
    fn closed_this_week_reads_the_journal_and_marks_who() {
        let dir = std::env::temp_dir().join(format!("qo-closed-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        let mut journal = crate::journal::Journal::new(&dir);

        let mut plant = |mut spec: crate::journal::NewRecord, ts: &str| {
            spec.ts = Some(ts.to_string());
            spec.device = Some("d".to_string());
            let mut rec = crate::journal::make_record(spec).unwrap();
            journal.append(&mut rec).unwrap();
        };

        let mut a = crate::journal::NewRecord::new("set", "tasks/a.md", "quinn", "dashboard");
        a.id = Some("task_aaaaaaaaaa");
        a.field = Some("progress");
        a.old = serde_json::json!(60);
        a.new = serde_json::json!(100);
        plant(a, "2026-08-27T15:00:00.000Z");

        let mut b = crate::journal::NewRecord::new("set", "tasks/b.md", "agent:coursework.vhl", "local-runner");
        b.id = Some("task_bbbbbbbbbb");
        b.field = Some("status");
        b.old = serde_json::json!("active");
        b.new = serde_json::json!("done");
        plant(b, "2026-08-27T16:00:00.000Z");

        let mut c = crate::journal::NewRecord::new("delete", "approvals/c.md", "system:approvals", "cli");
        c.id = Some("appr_cccccccccc");
        plant(c, "2026-08-27T17:00:00.000Z");

        let mut d = crate::journal::NewRecord::new("set", "tasks/d.md", "quinn", "dashboard");
        d.id = Some("task_dddddddddd");
        d.field = Some("progress");
        d.old = serde_json::json!(10);
        d.new = serde_json::json!(40);
        plant(d, "2026-08-27T18:00:00.000Z");

        drop(plant);
        let closed = closed_this_week(&dir, TODAY.at(9, 0, 0, 0), &mut journal);
        assert_eq!(closed.iter().map(|c| (c.path.as_str(), c.mark.as_str())).collect::<Vec<_>>(), [("tasks/a.md", "me"), ("tasks/b.md", "agent"), ("approvals/c.md", "system")]);
        let _ = std::fs::remove_dir_all(&dir);
    }

    /// Ruling 11: the action log's "me" and the delta's "you" are the vault's human under either
    /// token; an `agent:` actor is still neither.
    #[test]
    fn a_student_action_reads_as_me_and_you() {
        let dir = std::env::temp_dir().join(format!("qo-student-me-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        let mut journal = crate::journal::Journal::new(&dir);
        for (path, actor, field, new, ts) in [
            ("tasks/s.md", "student", "progress", serde_json::json!(100), "2026-08-27T15:00:00.000Z"),
            ("tasks/q.md", "quinn", "status", serde_json::json!("done"), "2026-08-27T16:00:00.000Z"),
            ("tasks/g.md", "agent:coursework.vhl", "progress", serde_json::json!(100), "2026-08-27T17:00:00.000Z"),
        ] {
            let mut spec = crate::journal::NewRecord::new("set", path, actor, "dashboard");
            spec.id = Some("task_aaaaaaaaaa");
            spec.field = Some(field);
            spec.new = new;
            spec.ts = Some(ts.to_string());
            spec.device = Some("d".to_string());
            journal.append(&mut crate::journal::make_record(spec).unwrap()).unwrap();
        }
        let closed = closed_this_week(&dir, TODAY.at(9, 0, 0, 0), &mut journal);
        assert_eq!(
            closed.iter().map(|c| (c.path.as_str(), c.mark.as_str())).collect::<Vec<_>>(),
            [("tasks/s.md", "me"), ("tasks/q.md", "me"), ("tasks/g.md", "agent")]
        );
        let d = delta(&dir, TODAY.at(9, 0, 0, 0), Some("2026-08-27T00:00:00.000Z"), &mut journal);
        assert_eq!(d.summary, "1 progress set by agent · 1 progress set by you · 1 status set by you");
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn runs_panel_says_in_words_when_nothing_ran_and_gauge_has_no_history() {
        let vault = Path::new("tests/fixtures/vault-full");
        let now: jiff::Timestamp = "2026-08-28T14:00:00Z".parse().unwrap();
        let rp = runs_panel(vault, now);
        assert!(rp.recent.is_empty() || rp.recent.iter().all(|r| !r.run_id.is_empty()));
        assert!(!rp.expected.is_empty(), "runners.yaml declares two runners; each due slot gets a row");
        assert!(rp.expected.iter().all(|e| ["seen", "late", "missing", "crashed"].contains(&e.status.as_str())));
        let g = gauge(vault, now);
        assert_eq!((g.points.len(), g.direction.as_str()), (0, "no-history"));
    }

    /// Plan 2 Task 8: `coursework` is a runner with its own name, and RUNS must not confuse it
    /// with the slot's rank run. Both appear in `recent` under the names they were written
    /// with; `expected` names only the runners `config/runners.yaml` declares, so a coursework
    /// run is never scored `late` or `missing` against a schedule it does not have; and a
    /// coursework WARN still counts toward `warn_count`, because a dead vendor session is a
    /// thing to see.
    #[test]
    fn runs_panel_keeps_coursework_and_the_slot_run_apart() {
        let dir = std::env::temp_dir().join(format!("qo-cwruns-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(dir.join("state").join("runs")).unwrap();
        std::fs::create_dir_all(dir.join("config")).unwrap();
        std::fs::copy(
            Path::new("tests/fixtures/vault-full/config/runners.yaml"),
            dir.join("config").join("runners.yaml"),
        )
        .unwrap();

        let ts: jiff::Timestamp = "2026-08-28T12:00:00Z".parse().unwrap();
        let local_id = crate::runs::start_run(&dir, "local", Some(ts), Some("d"), None);
        crate::runs::end_run(&dir, &local_id, "ok", "today.md refreshed", 0, Some(ts), None, &[], Some("local"));
        let cw_id = crate::runs::start_run(&dir, "coursework", Some(ts), Some("d"), None);
        crate::runs::end_run(
            &dir,
            &cw_id,
            "WARN",
            "coursework (2 assignments; 0 created, 0 updated; vhl: fetch failed)",
            0,
            Some(ts),
            None,
            &[],
            Some("coursework"),
        );

        let now: jiff::Timestamp = "2026-08-28T14:00:00Z".parse().unwrap();
        let rp = runs_panel(&dir, now);
        let mut runners: Vec<&str> = rp.recent.iter().map(|r| r.runner.as_str()).collect();
        runners.sort();
        assert_eq!(runners, vec!["coursework", "local"], "both rows, under the names written");
        // `expected` is the SCHEDULE, and coursework has none: runners.yaml declares two.
        let mut expected: Vec<&str> = rp.expected.iter().map(|e| e.runner.as_str()).collect();
        expected.sort();
        expected.dedup();
        assert_eq!(expected, vec!["cloud", "local"], "coursework must never be scored against a schedule");
        assert!(rp.warn_count >= 1, "a coursework WARN is still a thing to see");
        let _ = std::fs::remove_dir_all(&dir);
    }

    /// Plan 2 Task 7: the trend itself. Three runs, runway falling 2 and deficit rising 1, is
    /// `losing` — `d = (runway_b - runway_a) - (deficit_b - deficit_a)` = -2 - 1 = -3, past the
    /// -0.5 band. The counts arrive as `deficit_hours_x10`, tenths in an i64, and come back out
    /// as `deficit_hours` in real hours; a test that never crosses that boundary would not
    /// notice the division going missing.
    #[test]
    fn gauge_reads_three_planted_runs_and_calls_a_falling_runway_losing() {
        let dir = std::env::temp_dir().join(format!("qo-gauge-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(dir.join("state").join("runs")).unwrap();

        // runway 9 -> 8 -> 7, deficit 0.0h -> 0.5h -> 1.0h.
        for (day, runway, dx10, must) in
            [("2026-08-26", 9i64, 0i64, 1i64), ("2026-08-27", 8, 5, 2), ("2026-08-28", 7, 10, 3)]
        {
            let ts: jiff::Timestamp = format!("{day}T12:00:00Z").parse().unwrap();
            let run_id = crate::runs::start_run(&dir, "local", Some(ts), Some("d"), None);
            crate::runs::add_step(
                &dir,
                &run_id,
                "tasks",
                "ok",
                &[("active", 5), ("unreadable", 0), ("runway_days", runway), ("deficit_hours_x10", dx10), ("must_count", must)],
                "",
                Some(ts),
            );
        }

        let now: jiff::Timestamp = "2026-08-28T14:00:00Z".parse().unwrap();
        let g = gauge(&dir, now);
        assert_eq!(g.points.len(), 3, "one point per run, oldest first");
        assert_eq!(g.points.iter().map(|p| p.runway_days).collect::<Vec<_>>(), vec![9, 8, 7]);
        assert_eq!(g.points.iter().map(|p| p.deficit_hours).collect::<Vec<_>>(), vec![0.0, 0.5, 1.0]);
        assert_eq!(g.points.iter().map(|p| p.must_count).collect::<Vec<_>>(), vec![1, 2, 3]);
        assert_eq!(g.points[0].date, "2026-08-26");
        assert_eq!(g.direction, "losing");
        let _ = std::fs::remove_dir_all(&dir);
    }

    /// R15 (approvals.rs:1247-1268): a `snoozed` note whose `snooze_until` has already arrived is
    /// woken in place by the pass, so the read model counts it pending too — never `snoozed`.
    #[test]
    fn read_approvals_wakes_a_past_due_snooze_in_place() {
        let dir = std::env::temp_dir().join(format!("qo-approvals-snooze-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(dir.join("approvals")).unwrap();
        let note = "---\ntype: approval\nkind: task\ntitle: \"Snoozed thing\"\nstatus: snoozed\nproposed_at: 2026-08-20\nfirst_proposed_at: 2026-08-20\nsnooze_until: 2026-08-27\n---\n\nBody.\n";
        crate::pystr::write_text(&dir.join("approvals").join("snoozed-thing.md"), note).unwrap();
        let ar = read_approvals(&dir, TODAY); // TODAY is 2026-08-28; snooze_until is yesterday.
        assert_eq!((ar.pending, ar.snoozed, ar.cards.len()), (1, 0, 1));
        let _ = std::fs::remove_dir_all(&dir);
    }

    /// R16 (approvals.rs:1270-1292): a pending note whose `expires` has already passed is what the
    /// pass expires and archives next run — the read model excludes it rather than showing a card
    /// for a note about to disappear.
    #[test]
    fn read_approvals_excludes_a_pending_note_whose_expires_has_passed() {
        let dir = std::env::temp_dir().join(format!("qo-approvals-expired-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(dir.join("approvals")).unwrap();
        let note = "---\ntype: approval\nkind: task\ntitle: \"Expired thing\"\nstatus: pending\nproposed_at: 2026-08-20\nfirst_proposed_at: 2026-08-20\nexpires: 2026-08-27\n---\n\nBody.\n";
        crate::pystr::write_text(&dir.join("approvals").join("expired-thing.md"), note).unwrap();
        let ar = read_approvals(&dir, TODAY); // TODAY is 2026-08-28; expires was yesterday.
        assert_eq!((ar.pending, ar.cards.len()), (0, 0));
        let _ = std::fs::remove_dir_all(&dir);
    }

    /// R17 (approvals.rs:1296-1335): `events_in_digest` counts a digest's `events:` payload
    /// (`proposal_weight`), never becomes a card, but still feeds `oldest` like any other pending
    /// note — a controller-ruled correction (R18) to the brief's first sketch, which `continue`d
    /// past digests before touching `oldest`. R19: a digest-only queue (no cards at all) must not
    /// read as "Queue clear" — three events in today's digest are three decisions, not zero.
    #[test]
    fn read_approvals_counts_the_digest_in_the_header_never_as_a_card() {
        let digest = "---\ntype: approval\nkind: events-digest\ntitle: \"Events \u{2014} 2026-08-27\"\nstatus: pending\nproposed_at: 2026-08-27\nfirst_proposed_at: 2026-08-27\nexpires: 2026-09-10\nsnooze_until: null\ncreated_by: events\nevents:\n  - uid: \"engage:1\"\n    summary: \"AI Club Kickoff\"\n    start: 2026-08-29T18:00\n    end: 2026-08-29T19:00\n    location: \"Ferg 348\"\n  - uid: \"engage:2\"\n    summary: \"Career Fair\"\n    start: 2026-08-30T10:00\n    end: 2026-08-30T14:00\n    location: \"Coleman Coliseum\"\n  - uid: \"engage:3\"\n    summary: \"Study Abroad Info Session\"\n    start: 2026-08-31T15:00\n    end: 2026-08-31T16:00\n    location: \"Ten Hoor 106\"\n---\n\n**3 events worth a look.** Tick the ones you want on your calendar, then Approve. Anything left unticked is declined.\n\n- [ ] Sat 8/29 18:00 \u{b7} AI Club Kickoff \u{b7} Ferg 348 \u{b7} `engage:1`\n- [ ] Sun 8/30 10:00 \u{b7} Career Fair \u{b7} Coleman Coliseum \u{b7} `engage:2`\n- [ ] Mon 8/31 15:00 \u{b7} Study Abroad Info Session \u{b7} Ten Hoor 106 \u{b7} `engage:3`\n";

        let dir = std::env::temp_dir().join(format!("qo-approvals-digest-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(dir.join("approvals")).unwrap();
        crate::pystr::write_text(&dir.join("approvals").join("events-digest-2026-08-27.md"), digest).unwrap();
        let task = "---\ntype: approval\nkind: task\ntitle: \"Something\"\nstatus: pending\nsource_uid: \"gmail:xyz\"\nproposed_at: 2026-08-28\nfirst_proposed_at: 2026-08-28\nexpires: 2026-09-30\nsnooze_until: null\ncreated_by: gmail\n---\n\nBody text.\n";
        crate::pystr::write_text(&dir.join("approvals").join("task-something.md"), task).unwrap();

        let ar = read_approvals(&dir, TODAY);
        assert_eq!(ar.events_in_digest, 3);
        assert_eq!(ar.pending, 1);
        assert_eq!(ar.cards.len(), 1, "the digest is a header count, not a card");
        assert_eq!(ar.oldest_days, 1, "the digest's first_proposed_at (8/27) is older than the task's (8/28)");
        let _ = std::fs::remove_dir_all(&dir);

        let l = full();

        // R19: only the digest pending (no cards at all) — `empty_text` must not fire.
        let digest_only = std::env::temp_dir().join(format!("qo-approvals-digest-only-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&digest_only);
        std::fs::create_dir_all(digest_only.join("approvals")).unwrap();
        crate::pystr::write_text(&digest_only.join("approvals").join("events-digest-2026-08-27.md"), digest).unwrap();
        let ar_digest_only = read_approvals(&digest_only, TODAY);
        assert_eq!((ar_digest_only.cards.len(), ar_digest_only.events_in_digest), (0, 3));
        let d_digest_only = decisions(&digest_only, TODAY, &ar_digest_only, &l.planning);
        assert!(d_digest_only.empty_text.is_none(), "3 events in today's digest are 3 decisions, not a clear queue");
        let _ = std::fs::remove_dir_all(&digest_only);

        // Neither cards nor a digest: genuinely clear.
        let clear = std::env::temp_dir().join(format!("qo-approvals-clear-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&clear);
        std::fs::create_dir_all(clear.join("approvals")).unwrap();
        let ar_clear = read_approvals(&clear, TODAY);
        assert_eq!((ar_clear.cards.len(), ar_clear.events_in_digest), (0, 0));
        let d_clear = decisions(&clear, TODAY, &ar_clear, &l.planning);
        assert!(d_clear.empty_text.is_some());
        let _ = std::fs::remove_dir_all(&clear);
    }

    // -----------------------------------------------------------------------------------------
    // Task 8: delta, note_detail, build_state, state_json — one payload, never a write.
    // -----------------------------------------------------------------------------------------

    fn zoned(today: Date) -> jiff::Zoned { today.at(9, 0, 0, 0).to_zoned(jiff::tz::TimeZone::get("America/Chicago").unwrap()).unwrap() }

    #[test]
    fn build_state_has_every_region_the_schema_and_a_stable_revision() {
        let vault = Path::new("tests/fixtures/vault-full");
        let s = build_state(vault, View::Today, TODAY, &zoned(TODAY), None);
        assert_eq!((s.schema, s.view.as_str(), s.today.as_str()), (1, "today", "2026-08-28"));
        assert!(s.list.is_none(), "Today carries must_do and recommended, not a list");
        let again = build_state(vault, View::Today, TODAY, &zoned(TODAY), None);
        assert_eq!(s.revision, again.revision);
        let json = state_json(&s);
        for key in ["\"topline\"", "\"delta\"", "\"verdict\"", "\"meter\"", "\"gauge\"", "\"must_do\"", "\"recommended\"", "\"the_day\"", "\"decisions\"", "\"ahead\"", "\"coming_up\"", "\"good_to_know\"", "\"closed_this_week\"", "\"runs_panel\"", "\"issues_panel\"", "\"nav_counts\"", "\"empty\"", "\"unreadable\""] {
            assert!(json.contains(key), "{key} missing from the payload");
        }
        assert!(json.starts_with("{\"ahead\": "), "dumps_value: sorted keys, Python separators: {}", &json[..40]);
    }

    #[test]
    fn a_horizon_view_carries_its_list_and_the_same_rails() {
        let vault = Path::new("tests/fixtures/vault-full");
        let s = build_state(vault, View::Later, TODAY, &zoned(TODAY), None);
        let list = s.list.expect("list");
        assert_eq!((list.horizon.as_str(), list.count), ("later", 1));
        assert_eq!(s.nav_counts.later.count, 1);
    }

    #[test]
    fn build_state_never_writes() {
        for fixture in ["vault-s1", "vault-s1-migrated", "vault-full"] {
            let vault = Path::new("tests/fixtures").join(fixture);
            let snapshot = |root: &Path| -> BTreeMap<std::path::PathBuf, Vec<u8>> {
                let mut out = BTreeMap::new();
                fn walk(dir: &Path, out: &mut BTreeMap<std::path::PathBuf, Vec<u8>>) { for e in std::fs::read_dir(dir).unwrap().flatten() { let p = e.path(); if p.is_dir() { walk(&p, out); } else { out.insert(p.clone(), std::fs::read(&p).unwrap()); } } }
                walk(root, &mut out); out
            };
            let before = snapshot(&vault);
            for view in [View::Today, View::Overdue, View::Week, View::Later, View::AllActive, View::Decisions, View::GoodToKnow, View::Issues, View::Runs] {
                let _ = build_state(&vault, view, TODAY, &zoned(TODAY), Some("2026-08-27T00:00:00.000Z"));
            }
            // R22's guard: note_detail re-loads the vault and re-scans approvals too, so it needs
            // its own turn under this test — one real id, resolved the way a caller would.
            let l = load(&vault, TODAY);
            if let Some(t) = l.ranked.first() {
                let id = note_id(&l, &t.slug);
                let mut journal = crate::journal::Journal::new(&vault);
                let _ = note_detail(&vault, &id, TODAY, &mut journal);
            }
            assert_eq!(before, snapshot(&vault), "{fixture}: build_state wrote to the vault");
        }
    }

    #[test]
    fn delta_falls_back_to_the_last_run_and_describes_records() {
        let vault = Path::new("tests/fixtures/vault-full");
        let mut journal = crate::journal::Journal::new(vault);
        let d = delta(vault, TODAY.at(9, 0, 0, 0), None, &mut journal);
        assert!(matches!(d.since_kind.as_str(), "run" | "none"));
        let d2 = delta(vault, TODAY.at(9, 0, 0, 0), Some("2026-08-01T00:00:00.000Z"), &mut journal);
        assert_eq!(d2.since_kind, "seen");
        assert!(!d2.records.is_empty(), "the fixture journal has records after 2026-08-01");
        assert!(d2.records.iter().all(|r| !r.text.is_empty()));
        assert!(!d2.summary.is_empty());
    }

    /// R21: `seen_at` is caller-supplied (Task 9's `--seen-at`, Task 10's persisted per-device
    /// stamp) and must never crash the page. `JsonlLedger::read` derives its own day bound from
    /// the string it is given (ledger.rs's `first_chars`) and filters records itself, so `delta`
    /// has no business byte-slicing `seen_at` before handing it over.
    #[test]
    fn delta_does_not_panic_on_a_short_seen_at() {
        let vault = Path::new("tests/fixtures/vault-full");
        let mut journal = crate::journal::Journal::new(vault);
        let d = delta(vault, TODAY.at(9, 0, 0, 0), Some("x"), &mut journal);
        assert_eq!(d.since_kind, "seen");
        assert!(d.records.is_empty());
    }

    #[test]
    fn note_detail_shows_frontmatter_body_judgment_and_history() {
        let vault = Path::new("tests/fixtures/vault-full");
        let mut journal = crate::journal::Journal::new(vault);
        let l = full();
        let id = note_id(&l, "ph-106-exam-1-prep");
        let d = note_detail(vault, &id, TODAY, &mut journal).expect("found by id");
        assert_eq!((d.slug.as_str(), d.folder.as_str()), ("ph-106-exam-1-prep", "tasks"));
        assert!(d.frontmatter.get("title").is_some());
        let row = d.row.expect("an active task gets its row");
        assert!(row.amend_badge.is_some(), "R22: the fixture's pending amendment targets this task");
        assert!(note_detail(vault, "task_0000000000", TODAY, &mut journal).is_none());
    }

    // -----------------------------------------------------------------------------------------
    // Task 13: every named empty state (F20) is a real state of the live system, asserted.
    // -----------------------------------------------------------------------------------------

    #[test]
    fn an_empty_vault_names_every_empty_state() {
        let dir = std::env::temp_dir().join(format!("qo-empty-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        for d in ["tasks", "approvals", "config", "state", "info", "issues"] { std::fs::create_dir_all(dir.join(d)).unwrap(); }
        std::fs::copy("tests/fixtures/vault-s1/config/week_template.yaml", dir.join("config").join("week_template.yaml")).unwrap();
        std::fs::copy("tests/fixtures/vault-s1/config/planning.yaml", dir.join("config").join("planning.yaml")).unwrap();
        let s = build_state(&dir, View::Today, TODAY, &zoned(TODAY), None);
        assert!(!s.empty.active);
        assert_eq!(s.verdict.headline, EMPTY_TEXT.active);
        assert_eq!(s.must_do.empty_text.as_deref(), Some(EMPTY_TEXT.must_do));
        assert_eq!(s.recommended.empty_text.as_deref(), Some(EMPTY_TEXT.recommended));
        assert_eq!(s.decisions.empty_text.as_deref(), Some(EMPTY_TEXT.decisions));
        assert_eq!(s.issues_panel.empty_text.as_deref(), Some(EMPTY_TEXT.issues));
        assert_eq!(s.runs_panel.empty_text.as_deref(), Some(EMPTY_TEXT.runs));
        assert!(s.coming_up.is_empty() && s.good_to_know.is_empty());
        assert_eq!(s.ahead.takeaway, "Nothing due in the next fortnight.");
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn a_day_with_no_free_block_says_so_rather_than_hiding_the_lane() {
        // Sunday in vault-s1's template has no classes; put an all-day-window event on it.
        let mut l = load(Path::new("tests/fixtures/vault-s1"), Date::constant(2026, 8, 30));
        let day = Date::constant(2026, 8, 30);
        let busy = crate::weekcal::CalEvent { title: "Away".into(), start: day.at(8, 0, 0, 0), end: day.at(18, 0, 0, 0), all_day: false };
        l.cal = WeekCalendar::from_file(Path::new("tests/fixtures/vault-s1/config/week_template.yaml"), vec![busy]);
        let d = the_day(&l, day);
        assert_eq!(d.empty_text.as_deref(), Some(EMPTY_TEXT.the_day));
        assert_eq!(d.blocks.iter().filter(|b| b.kind == "busy").count(), 1, "the busy block still renders");
    }

    // -----------------------------------------------------------------------------------------
    // Knowlu plan 1, Task 1: the surface bundle.
    // -----------------------------------------------------------------------------------------

    #[test]
    fn expected_due_is_a_vault_local_stamp_not_utc() {
        // vault-full's config/ingest.yaml says America/Chicago; config/runners.yaml says local
        // 12:00/18:00 — this 24h lookback window (see `due_times`) actually catches BOTH the
        // 08-27 18:00 and 08-28 12:00 occurrences, so find the specific stamp rather than
        // assuming the first "local" row is it.
        let v = fixture_full();
        let now_ts: jiff::Timestamp = "2026-08-28T20:00:00Z".parse().unwrap();
        let p = runs_panel(&v, now_ts);
        let local = p
            .expected
            .iter()
            .find(|e| e.runner == "local" && e.due.starts_with("2026-08-28 12:00"))
            .unwrap_or_else(|| panic!("no local row due 2026-08-28 12:00: {:?}", p.expected));
        assert!(!local.due.ends_with('Z'));
    }

    #[test]
    fn a_missing_runners_config_is_a_warning_not_all_clear() {
        let v = fixture_full();
        std::fs::remove_file(v.join("config/runners.yaml")).unwrap();
        let p = runs_panel(&v, pinned_now().timestamp());
        assert!(p.expected.is_empty());
        assert!(p.warnings.is_empty(), "a missing file is Ok(vec![]) upstream — no warning");
        // `name: 3` alone is well-formed (Python's `str(3)`/Rust's `yaml_str` both coerce it, and
        // an entry with no `times` just contributes zero due dates) — the actually malformed case
        // documented on `load_runners_config` is a runner entry with NO `name` at all.
        std::fs::write(v.join("config/runners.yaml"), "runners:\n  - times: [\"12:00\"]\n").unwrap();
        let p = runs_panel(&v, pinned_now().timestamp());
        assert_eq!(p.warnings.len(), 1, "a malformed entry surfaces as one warning: {:?}", p.warnings);
    }

    #[test]
    fn sums_never_print_negative_zero() {
        let v = fixture_full();
        for f in std::fs::read_dir(v.join("tasks")).unwrap().flatten() { std::fs::remove_file(f.path()).unwrap(); }
        let s = build_state(&v, View::Today, pinned_today(), &pinned_now(), None);
        let json = state_json(&s);
        assert!(!json.contains("-0.0"), "{json}");
    }

    #[test]
    fn must_do_has_no_empty_text_beside_an_unreadable_row() {
        let v = fixture_full();
        for f in std::fs::read_dir(v.join("tasks")).unwrap().flatten() { std::fs::remove_file(f.path()).unwrap(); }
        std::fs::write(v.join("tasks/broken.md"), "---\ntitle: [\n---\n").unwrap();
        let s = build_state(&v, View::Today, pinned_today(), &pinned_now(), None);
        assert_eq!(s.unreadable, vec!["tasks/broken.md".to_string()]);
        assert!(s.must_do.empty_text.is_none(), "unreadable rows are the content; no empty text beside them");
    }

    #[test]
    fn verdict_uses_the_shared_empty_string() {
        let v = fixture_full();
        for f in std::fs::read_dir(v.join("tasks")).unwrap().flatten() { std::fs::remove_file(f.path()).unwrap(); }
        let s = build_state(&v, View::Today, pinned_today(), &pinned_now(), None);
        assert_eq!(s.verdict.headline, EMPTY_TEXT.active);
    }

    #[test]
    fn closed_this_week_is_one_row_per_note_and_bounded_by_now() {
        let v = fixture_full();
        let mut j = crate::journal::Journal::new(&v);
        let ctx = crate::write::WriteContext::new("quinn", "dashboard");
        let id = first_task_id(&v);
        // progress 100 + status done in ONE write → one journal moment → one row.
        crate::write::write_literals(&v, &id, &[("progress".into(), "100".into()), ("status".into(), "done".into())], &ctx, &mut j, &Default::default()).unwrap();
        let now = crate::journal::now_ts(None).parse::<jiff::Timestamp>().unwrap().to_zoned(jiff::tz::TimeZone::UTC).datetime();
        let rows = closed_this_week(&v, now, &mut j);
        assert_eq!(rows.iter().filter(|r| r.id.as_deref() == Some(id.as_str())).count(), 1);
        let past = now.checked_sub(jiff::Span::new().days(1)).unwrap();
        assert!(closed_this_week(&v, past, &mut j).iter().all(|r| r.id.as_deref() != Some(id.as_str())), "a record after `now` is not closed yet");
    }

    #[test]
    fn delta_records_are_capped_and_say_so() {
        let v = fixture_full();
        let mut j = crate::journal::Journal::new(&v);
        let ctx = crate::write::WriteContext::new("quinn", "dashboard");
        let id = first_task_id(&v);
        for i in 0..(DELTA_RECORD_CAP + 5) {
            crate::write::write_literals(&v, &id, &[("importance".into(), ((i % 5) + 1).to_string())], &ctx, &mut j, &Default::default()).unwrap();
        }
        let d = delta(&v, pinned_now().datetime(), Some("2000-01-01T00:00:00.000Z"), &mut j);
        assert_eq!(d.records.len(), DELTA_RECORD_CAP);
        assert!(d.truncated);
    }

    #[test]
    fn journal_warnings_reach_the_state() {
        // Only `ledger.rs` names a ledger file directly (`only_this_module_opens_ledger_files`);
        // find the fixture's existing day file rather than spelling its extension here.
        let v = fixture_full();
        let journal_dir = v.join("state").join("journal");
        let day_file = std::fs::read_dir(&journal_dir)
            .unwrap()
            .flatten()
            .find(|e| !e.file_name().to_string_lossy().starts_with('.'))
            .expect("fixture-full has a journal day file")
            .path();
        std::fs::write(&day_file, "not json\r\n").unwrap();
        let s = build_state(&v, View::Today, pinned_today(), &pinned_now(), None);
        assert!(!s.warnings.is_empty(), "the ledger's parse warning must surface");
    }

    #[test]
    fn open_hours_is_clamped_like_spare_hours() {
        let v = fixture_full();
        let s = build_state(&v, View::Today, pinned_today(), &pinned_now(), None);
        assert!(s.the_day.open_hours >= 0.0);
        assert!(s.recommended.spare_hours >= 0.0);
    }

    fn first_task_id(v: &std::path::Path) -> String {
        let s = build_state(v, View::Today, pinned_today(), &pinned_now(), None);
        s.must_do.groups[0].rows[0].id.clone()
    }
}


/// P18 — §6.4's `moved`, computed live by `load` and never written; the `--window` preview.
#[cfg(test)]
mod moved_tests {
    use super::*;
    use crate::journal::Journal;
    use crate::write::{WriteContext, WriteOpts};
    use std::path::PathBuf;

    /// A Thursday.
    const DAY: Date = Date::constant(2026, 9, 24);
    const NOTE: &str = "commitments/planning-day.md";
    const WEEKDAYS_18: &str = "[{days: [mon, tue, wed, thu, fri], start: \"08:00\", end: \"18:00\"}]";
    const WEEKDAYS_20: &str = "[{days: [mon, tue, wed, thu, fri], start: \"08:00\", end: \"20:00\"}]";
    const WEEKDAYS_22: &str = "[{days: [mon, tue, wed, thu, fri], start: \"08:00\", end: \"22:00\"}]";

    fn now() -> jiff::Zoned {
        DAY.at(9, 0, 0, 0).to_zoned(jiff::tz::TimeZone::UTC).unwrap()
    }

    /// A scratch vault: weekdays busy 08:00–17:30 in a flat 08:00–18:00 template (so the day's
    /// only room is what a later window opens), weekends free, three invented tasks due far out.
    fn vault(name: &str) -> PathBuf {
        static COUNTER: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(0);
        let n = COUNTER.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
        let dir = std::env::temp_dir().join(format!("qo-p18-{name}-{}-{n}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        for sub in ["config", "tasks", "state"] {
            std::fs::create_dir_all(dir.join(sub)).unwrap();
        }
        let busy = "[['08:00', '17:30']]";
        std::fs::write(
            dir.join("config").join("week_template.yaml"),
            format!("day_start: '08:00'\nday_end: '18:00'\nclasses:\n  mon: {busy}\n  tue: {busy}\n  wed: {busy}\n  thu: {busy}\n  fri: {busy}\n  sat: []\n  sun: []\n"),
        )
        .unwrap();
        for slug in ["essay", "lab", "reading"] {
            std::fs::write(
                dir.join("tasks").join(format!("{slug}.md")),
                format!("---\ntitle: Invented {slug}\ndue: 2026-11-30\neffort_hours: 1.5\nimportance: 3\n---\n"),
            )
            .unwrap();
        }
        dir
    }

    /// The planning-day note, created through `write::create` with `window`.
    fn note(v: &Path, window: &str) {
        std::fs::create_dir_all(v.join("commitments")).unwrap();
        crate::write::create(
            v,
            NOTE,
            &format!("---\ntype: commitment\nkind: planning-day\nstatus: confirmed\nwindow: {window}\n---\n\nInvented.\n"),
            &WriteContext::new("student", "dashboard"),
            &mut Journal::new(v),
            None,
        )
        .unwrap();
    }

    /// The student edits the note's window through the engine's `write`, as the console does.
    fn edit(v: &Path, window: &str) {
        crate::write::write_literals(
            v,
            NOTE,
            &[("window".to_string(), window.to_string())],
            &WriteContext::new("student", "dashboard"),
            &mut Journal::new(v),
            &WriteOpts::default(),
        )
        .unwrap();
    }

    fn plan(v: &Path, date: Date, start: &str, end: &str) {
        std::fs::write(
            v.join("state").join("plan.json"),
            format!("{{\"date\": \"{date}\", \"end\": \"{end}\", \"start\": \"{start}\"}}\n"),
        )
        .unwrap();
    }

    /// Every file under `v`, with its bytes.
    fn snapshot(v: &Path) -> BTreeMap<PathBuf, Vec<u8>> {
        let mut out = BTreeMap::new();
        let mut stack = vec![v.to_path_buf()];
        while let Some(dir) = stack.pop() {
            for entry in std::fs::read_dir(&dir).unwrap().flatten() {
                let path = entry.path();
                if path.is_dir() {
                    stack.push(path);
                } else {
                    out.insert(path.clone(), std::fs::read(&path).unwrap());
                }
            }
        }
        out
    }

    #[test]
    fn surface_reports_moved_at_once_after_a_window_edit_without_a_rank() {
        let v = vault("edit");
        note(&v, WEEKDAYS_18);
        plan(&v, DAY, "08:00", "18:00");
        assert_eq!(load(&v, DAY).moved, None, "an unedited window moves nothing");
        edit(&v, WEEKDAYS_22);
        let moved = load(&v, DAY).moved.expect("the edit shows at once, before any rank");
        assert!(moved.to.evening > 0, "{moved:?}");
        assert_eq!((moved.to.morning, moved.to.afternoon, moved.dropped), (0, 0, 0), "{moved:?}");
        assert!(moved.text.ends_with("moved to this evening"), "{}", moved.text);
        let s = build_state(&v, View::Today, DAY, &now(), None);
        assert_eq!(s.moved, Some(moved.clone()));
        assert!(state_json(&s).contains("\"moved\": {"));
        // The line is the today view's; another view carries none.
        assert_eq!(build_state(&v, View::Week, DAY, &now(), None).moved, None);
        let _ = std::fs::remove_dir_all(&v);
    }

    #[test]
    fn the_first_window_is_diffed_against_the_template() {
        let v = vault("first");
        note(&v, WEEKDAYS_22);
        assert!(!v.join("state").join("plan.json").exists());
        let moved = load(&v, DAY).moved.expect("the template's 08:00–18:00 is the baseline");
        assert!(moved.to.evening > 0, "{moved:?}");
        // An unreadable plan file reads the same way.
        std::fs::write(v.join("state").join("plan.json"), "not json\n").unwrap();
        assert_eq!(load(&v, DAY).moved, Some(moved));
        let _ = std::fs::remove_dir_all(&v);
    }

    #[test]
    fn a_second_edit_is_reported_against_the_days_first_window() {
        let v = vault("second");
        note(&v, WEEKDAYS_18);
        plan(&v, DAY, "08:00", "18:00");
        edit(&v, WEEKDAYS_22);
        let first = load(&v, DAY).moved.unwrap();
        edit(&v, WEEKDAYS_20);
        let second = load(&v, DAY).moved.expect("still against the morning's 08:00–18:00");
        // Against the first edit (22:00) a 20:00 end could only drop takes; against 18:00 it opens
        // the evening.
        assert!(second.to.evening > 0, "{second:?}");
        assert_eq!(second.dropped, 0, "{second:?}");
        assert!(second.to.evening <= first.to.evening, "{first:?} {second:?}");
        let _ = std::fs::remove_dir_all(&v);
    }

    #[test]
    fn an_undone_edit_reports_nothing() {
        let v = vault("undo");
        note(&v, WEEKDAYS_18);
        plan(&v, DAY, "08:00", "18:00");
        edit(&v, WEEKDAYS_22);
        assert!(load(&v, DAY).moved.is_some());
        edit(&v, WEEKDAYS_18);
        assert_eq!(load(&v, DAY).moved, None);
        let _ = std::fs::remove_dir_all(&v);
    }

    #[test]
    fn a_plan_json_from_yesterday_reports_nothing_before_the_first_rank() {
        let v = vault("yesterday");
        note(&v, WEEKDAYS_22);
        plan(&v, add_days(DAY, -1), "08:00", "18:00");
        assert_eq!(load(&v, DAY).moved, None);
        let _ = std::fs::remove_dir_all(&v);
    }

    #[test]
    fn no_note_means_no_moved_key_in_the_json() {
        let v = vault("nonote");
        // A plan file alone is no baseline: without the note there is none.
        plan(&v, DAY, "08:00", "12:00");
        assert_eq!(load(&v, DAY).moved, None);
        let s = build_state(&v, View::Today, DAY, &now(), None);
        assert!(!state_json(&s).contains("\"moved\""), "{}", state_json(&s));
        let _ = std::fs::remove_dir_all(&v);
    }

    #[test]
    fn surface_load_writes_nothing() {
        let v = vault("nowrite");
        note(&v, WEEKDAYS_18);
        edit(&v, WEEKDAYS_22);
        let before = snapshot(&v);
        assert!(load(&v, DAY).moved.is_some());
        let _ = build_state(&v, View::Today, DAY, &now(), None);
        let _ = build_state_preview(&v, View::Today, DAY, &now(), None, WEEKDAYS_20).unwrap();
        assert_eq!(snapshot(&v), before, "surface must not write, preview included");
        assert!(!v.join("state").join("plan.json").exists());
        let _ = std::fs::remove_dir_all(&v);
    }

    #[test]
    fn window_preview_diffs_against_the_current_window_and_writes_nothing() {
        let v = vault("preview");
        note(&v, WEEKDAYS_18);
        plan(&v, DAY, "08:00", "18:00");
        edit(&v, WEEKDAYS_22);
        let before = snapshot(&v);
        // Against the current 22:00 window (not the day's 18:00 baseline), 20:00 only drops.
        let s = build_state_preview(&v, View::Today, DAY, &now(), None, WEEKDAYS_20).unwrap();
        let moved = s.moved.clone().expect("20:00 fits less than 22:00");
        assert!(moved.dropped > 0, "{moved:?}");
        assert_eq!(moved.to, crate::commitments::MovedTo::default(), "{moved:?}");
        // The day itself is computed under the proposed window.
        assert_eq!(s.the_day, the_day(&load_with(&v, DAY, Some(&crate::commitments::parse_window(WEEKDAYS_20).unwrap())), DAY));
        // Proposing the current window moves nothing, whatever the baseline says.
        let same = build_state_preview(&v, View::Today, DAY, &now(), None, WEEKDAYS_22).unwrap();
        assert_eq!(same.moved, None);
        // With no note at all the current window is the template, and a preview still works.
        let bare = vault("preview-bare");
        let opened = build_state_preview(&bare, View::Today, DAY, &now(), None, WEEKDAYS_22).unwrap();
        assert!(opened.moved.unwrap().to.evening > 0);
        assert_eq!(snapshot(&v), before);
        let _ = std::fs::remove_dir_all(&v);
        let _ = std::fs::remove_dir_all(&bare);
    }

    #[test]
    fn a_bad_window_argument_is_refused() {
        let v = vault("bad");
        for bad in [
            "[{days: [mon], start: \"22:00\", end: \"08:00\"}]",
            "[{days: [mon], start: \"08:00\", end: \"25:00\"}]",
            "[{days: [someday], start: \"08:00\", end: \"18:00\"}]",
            "not a window",
            "[{days: [mon",
        ] {
            let err = build_state_preview(&v, View::Today, DAY, &now(), None, bad).err();
            assert!(err.is_some(), "{bad} must be refused");
        }
        assert!(build_state_preview(&v, View::Week, DAY, &now(), None, WEEKDAYS_22).is_err());
        let _ = std::fs::remove_dir_all(&v);
    }
}
