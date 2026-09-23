//! Port of `engine/weekcal.py` — free-block computation and daily capacity.
//!
//! This is the most directly verifiable module in wave 1. The golden file says
//! **`Capacity today: 8.25h`** for Friday 2026-08-28, and that number is reproducible by hand:
//!
//! | Free block | Minutes | Kept? |
//! |---|---|---|
//! | 08:00-12:00 | 240 | yes |
//! | 12:50-13:00 | 10 | no, below `min_block_minutes: 45` |
//! | 13:45-18:00 | 255 | yes |
//!
//! 495 minutes = 8.25h. If this module produces anything else, every later number is wrong too.

use std::collections::{BTreeMap, HashMap, HashSet};
use std::path::Path;

use jiff::civil::{Date, DateTime, Time};
use serde_yaml_ng::{Mapping, Value};

use crate::planning::{day_key, DAY_KEYS};

/// One of `planning::DAY_KEYS` — the weekday a commitment span recurs on.
pub type DayKey = &'static str;

/// A confirmed hard/soft commitment's weekly meeting time — the spec's `Span`, renamed to stay
/// clear of `jiff::Span`. Plain data: this module knows nothing of `commitments.rs`.
#[derive(Debug, Clone, PartialEq)]
pub struct CommitmentSpan {
    pub day: DayKey,
    pub start: Time,
    pub end: Time,
    pub from: Option<Date>,
    pub until: Option<Date>,
    pub title: String,
    pub kind: String,
    pub source_uid: String,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Block {
    pub start: DateTime,
    pub end: DateTime,
}

impl Block {
    pub fn hours(&self) -> f64 {
        hours_between(self.start, self.end)
    }
}

/// The single place datetime arithmetic happens, so there is exactly one thing to fix if jiff's
/// API shifts under us.
fn hours_between(a: DateTime, b: DateTime) -> f64 {
    a.duration_until(b).as_secs_f64() / 3600.0
}

/// A busy calendar event. All-day events never subtract busy time — they are context.
#[derive(Debug, Clone, PartialEq)]
pub struct CalEvent {
    pub title: String,
    pub start: DateTime,
    pub end: DateTime,
    pub all_day: bool,
}

/// Remove `[start, end)` from a block list.
///
/// A zero-length or inverted event subtracts nothing — a malformed event must not silently eat
/// capacity.
fn subtract(blocks: &[Block], start: DateTime, end: DateTime) -> Vec<Block> {
    if end <= start {
        return blocks.to_vec();
    }
    let mut out = Vec::new();
    for block in blocks {
        if end <= block.start || start >= block.end {
            out.push(*block);
            continue;
        }
        if block.start < start {
            out.push(Block { start: block.start, end: start });
        }
        if end < block.end {
            out.push(Block { start: end, end: block.end });
        }
    }
    out
}

/// `DAY_KEYS`' index for `day`'s weekday, the slot `window` and `with_day_window` use.
fn weekday_index(day: Date) -> Option<usize> {
    DAY_KEYS.iter().position(|k| *k == day_key(day))
}

pub fn parse_hm(text: &str) -> Time {
    let mut parts = text.trim().split(':');
    let hour: i8 = parts.next().and_then(|h| h.trim().parse().ok()).unwrap_or(0);
    let minute: i8 = parts.next().and_then(|m| m.trim().parse().ok()).unwrap_or(0);
    Time::new(hour, minute, 0, 0).unwrap_or(Time::midnight())
}

#[derive(Clone)]
pub struct WeekCalendar {
    pub day_start: Time,
    pub day_end: Time,
    pub min_block_minutes: i64,
    classes: HashMap<&'static str, Vec<(Time, Time)>>,
    events: Vec<CalEvent>,
    commitment_spans: Vec<CommitmentSpan>,
    instances: BTreeMap<String, (Date, Date, Vec<(Date, Time, Time)>)>,
    window: [Option<(Time, Time)>; 7],
}

use crate::yaml::{get, opt_text as as_text};

impl WeekCalendar {
    pub fn new(config: &Mapping, events: Vec<CalEvent>) -> WeekCalendar {
        let day_start = parse_hm(&as_text(get(config, "day_start")).unwrap_or_else(|| "08:00".into()));
        let day_end = parse_hm(&as_text(get(config, "day_end")).unwrap_or_else(|| "18:00".into()));
        let min_block_minutes = match get(config, "min_block_minutes") {
            Some(Value::Number(n)) => n.as_i64().unwrap_or(45),
            Some(Value::String(s)) => s.trim().parse().unwrap_or(45),
            _ => 45,
        };

        let empty = Mapping::new();
        let raw_classes = match get(config, "classes") {
            Some(Value::Mapping(m)) => m,
            _ => &empty,
        };
        let mut classes = HashMap::new();
        for key in DAY_KEYS {
            let mut spans = Vec::new();
            if let Some(Value::Sequence(items)) = get(raw_classes, key) {
                for item in items {
                    if let Value::Sequence(pair) = item {
                        if pair.len() >= 2 {
                            if let (Some(s), Some(e)) =
                                (as_text(pair.first()), as_text(pair.get(1)))
                            {
                                spans.push((parse_hm(&s), parse_hm(&e)));
                            }
                        }
                    }
                }
            }
            classes.insert(key, spans);
        }

        let mut events = events;
        events.sort_by(|a, b| {
            a.start
                .cmp(&b.start)
                .then(a.end.cmp(&b.end))
                .then(a.title.cmp(&b.title))
        });

        WeekCalendar {
            day_start,
            day_end,
            min_block_minutes,
            classes,
            events,
            commitment_spans: Vec::new(),
            instances: BTreeMap::new(),
            window: [None; 7],
        }
    }

    pub fn from_file(path: &Path, events: Vec<CalEvent>) -> WeekCalendar {
        let text = std::fs::read_to_string(path).unwrap_or_default();
        let config = match serde_yaml_ng::from_str::<Value>(&text) {
            Ok(Value::Mapping(m)) => m,
            _ => Mapping::new(),
        };
        WeekCalendar::new(&config, events)
    }

    /// The one constructor `rank` and `surface` both build their calendar from (spec §6.1):
    /// `config/week_template.yaml`, plus `vault`'s confirmed hard/soft commitments and planning
    /// day (`commitments::load`), plus the fresh-read horizon and actual instances
    /// (`commitments::read_series_file`). A vault with no `commitments/` and no series file gives
    /// exactly what `from_file` alone would (§8): `load` returns empty spans and no window, and
    /// `read_series_file` returns an empty instances map.
    pub fn for_vault(vault: &Path, events: Vec<CalEvent>) -> WeekCalendar {
        WeekCalendar::for_vault_with_warnings(vault, events).0
    }

    /// [`WeekCalendar::for_vault`], plus every warning `commitments::load` and
    /// `commitments::read_series_file` produced along the way — a caller (the `calendar` run-record
    /// step, §6.5) folds them in without re-reading the vault.
    pub fn for_vault_with_warnings(vault: &Path, events: Vec<CalEvent>) -> (WeekCalendar, Vec<String>) {
        let commitments = crate::commitments::load(vault);
        let (series, series_warnings) = crate::commitments::read_series_file(vault);
        let mut warnings = commitments.warnings.clone();
        warnings.extend(series_warnings);

        let template = vault.join("config").join("week_template.yaml");
        let calendar = WeekCalendar::from_file(&template, events)
            .with_commitments(commitments.spans(), commitments.window)
            .with_instances(series.instances_map());
        (calendar, warnings)
    }

    /// Plain data in: confirmed hard/soft commitment spans, plus the planning-day window per
    /// weekday. *Reason:* §6.1 — keeps `weekcal` buildable and pure before `commitments.rs` exists.
    pub fn with_commitments(
        mut self,
        spans: Vec<CommitmentSpan>,
        window: [Option<(Time, Time)>; 7],
    ) -> Self {
        self.commitment_spans = spans;
        self.window = window;
        self
    }

    /// Per `source_uid`, the fresh-read horizon `[read date, read date + 28)` and that source's
    /// actual instances inside it (R21).
    pub fn with_instances(
        mut self,
        instances: BTreeMap<String, (Date, Date, Vec<(Date, Time, Time)>)>,
    ) -> Self {
        self.instances = instances;
        self
    }

    /// A copy whose window for `day`'s weekday is replaced. *Reason:* P18 plans today under the
    /// baseline window and the `--window` preview under a proposed one, from the same calendar
    /// otherwise. An inverted or zero-length window (`start >= end`) is ignored (M-1): this is the
    /// one place a `--window` preview string reaches `WeekCalendar`, and a malformed one must not
    /// silently corrupt the day rather than being ignored.
    pub fn with_day_window(mut self, day: Date, start: Time, end: Time) -> Self {
        if start >= end {
            return self;
        }
        if let Some(idx) = weekday_index(day) {
            self.window[idx] = Some((start, end));
        }
        self
    }

    /// The planning day's entry for `day_key(day)`, else the template's `(day_start, day_end)`.
    /// Every date-based use of `day_start`/`day_end` in this file goes through this. *Reason:*
    /// §6.1.
    pub fn window(&self, day: Date) -> (Time, Time) {
        weekday_index(day)
            .and_then(|idx| self.window[idx])
            .unwrap_or((self.day_start, self.day_end))
    }

    /// The template's classes for `day`'s weekday, as `(DateTime, DateTime)` busy spans. No
    /// commitment spans.
    fn class_busy(&self, day: Date) -> Vec<(DateTime, DateTime)> {
        self.classes
            .get(day_key(day))
            .map(|spans| {
                spans
                    .iter()
                    .map(|(s, e)| {
                        (DateTime::from_parts(day, *s), DateTime::from_parts(day, *e))
                    })
                    .collect()
            })
            .unwrap_or_default()
    }

    /// The weekly rule alone, with no idea of `instances`: busy on `span`'s weekday when
    /// `from <= day <= until`. An inverted or zero-length span (`end <= start`) is dropped rather
    /// than fed to the cursor walk, which would double-count (M-1: defence in depth — a span
    /// arrives from a note or a series, and P6/§6.3 validate upstream, but this file never trusts
    /// that alone). *Reason:* §6.1, R21.
    fn weekly_span_block(day: Date, span: &CommitmentSpan) -> Option<(Time, Time)> {
        if span.end <= span.start {
            return None;
        }
        if day_key(day) != span.day {
            return None;
        }
        let after_from = span.from.map_or(true, |f| f <= day);
        let before_until = span.until.map_or(true, |u| day <= u);
        if after_from && before_until {
            Some((span.start, span.end))
        } else {
            None
        }
    }

    /// Every commitment span active on `day`, with the time it is actually busy that day.
    ///
    /// A source with an `instances` entry whose horizon covers `day` is handled **once**, on the
    /// first span in `commitment_spans` that names it: every instance dated `day` for that source
    /// is emitted (not just the first — two instances of one source on the same date, such as a
    /// moved class landing on another class's day, are both busy), tagged with that first span (so
    /// the drawn title/kind come from one place). Every other span sharing that `source_uid` emits
    /// nothing here — P6 emits one span per `(meet, day)` sharing a `source_uid`, so without this,
    /// each of a multi-day note's spans would independently find the same instance and draw it
    /// once per weekday. A span whose source has no horizon entry, or whose horizon does not cover
    /// `day`, falls back to `weekly_span_block`, per span as before.
    fn active_spans(&self, day: Date) -> Vec<(Time, Time, &CommitmentSpan)> {
        let mut out = Vec::new();
        let mut handled_sources: HashSet<&str> = HashSet::new();
        for span in &self.commitment_spans {
            let horizon = self
                .instances
                .get(span.source_uid.as_str())
                .filter(|(start, end, _)| *start <= day && day < *end);
            if let Some((_, _, instances)) = horizon {
                if handled_sources.insert(span.source_uid.as_str()) {
                    out.extend(
                        instances
                            .iter()
                            .filter(|(date, start, end)| *date == day && start < end)
                            .map(|(_, start, end)| (*start, *end, span)),
                    );
                }
                continue;
            }
            if let Some((start, end)) = Self::weekly_span_block(day, span) {
                out.push((start, end, span));
            }
        }
        out
    }

    /// The active spans as drawn: the same rule `template_blocks` uses to fold spans into the busy
    /// list. *Reason:* one rule, two readers (P17 draws from this).
    pub fn spans_on(&self, day: Date) -> Vec<(DateTime, DateTime, &CommitmentSpan)> {
        self.active_spans(day)
            .into_iter()
            .map(|(s, e, span)| {
                (DateTime::from_parts(day, s), DateTime::from_parts(day, e), span)
            })
            .collect()
    }

    /// Sort, cursor walk and clamp to `window(day)`'s end — unchanged from before commitments, now
    /// shared by `template_blocks` and `template_only_blocks`.
    fn blocks_from_busy(&self, day: Date, mut busy: Vec<(DateTime, DateTime)>) -> Vec<Block> {
        busy.sort();

        let (window_start, window_end) = self.window(day);
        let mut cursor = DateTime::from_parts(day, window_start);
        let end_of_day = DateTime::from_parts(day, window_end);
        let mut blocks = Vec::new();
        for (start, end) in busy {
            let boundary = start.min(end_of_day);
            if cursor < boundary {
                blocks.push(Block { start: cursor, end: boundary });
            }
            cursor = cursor.max(end);
        }
        if cursor < end_of_day {
            blocks.push(Block { start: cursor, end: end_of_day });
        }
        blocks
    }

    /// Free blocks from the timetable alone, WITHOUT the minimum-length filter. The busy list is
    /// the template's classes for `day`'s weekday plus each commitment span active that day.
    pub fn template_blocks(&self, day: Date) -> Vec<Block> {
        let mut busy = self.class_busy(day);
        busy.extend(
            self.active_spans(day)
                .into_iter()
                .map(|(s, e, _)| (DateTime::from_parts(day, s), DateTime::from_parts(day, e))),
        );
        self.blocks_from_busy(day, busy)
    }

    /// Today's computation (template classes only) within `window(day)` — no commitment spans.
    /// *Reason:* §6.2's gap walk must not paint a club as a class (R15).
    pub fn template_only_blocks(&self, day: Date) -> Vec<Block> {
        let busy = self.class_busy(day);
        self.blocks_from_busy(day, busy)
    }

    /// Applied AFTER subtraction, never to the class list. Filtering earlier gives 8.417h for the
    /// fixture Friday — plausible-looking and wrong.
    fn min_filter(&self, blocks: Vec<Block>) -> Vec<Block> {
        let minimum = self.min_block_minutes as f64 / 60.0;
        blocks.into_iter().filter(|b| b.hours() >= minimum).collect()
    }

    pub fn free_blocks(&self, day: Date) -> Vec<Block> {
        let mut blocks = self.template_blocks(day);
        for event in &self.events {
            if event.all_day {
                continue;
            }
            blocks = subtract(&blocks, event.start, event.end);
        }
        self.min_filter(blocks)
    }

    pub fn template_capacity(&self, day: Date) -> f64 {
        self.min_filter(self.template_blocks(day)).iter().map(|b| b.hours()).sum()
    }

    pub fn events_on(&self, day: Date) -> Vec<&CalEvent> {
        self.events
            .iter()
            .filter(|e| {
                if e.all_day {
                    e.start.date() <= day && day < e.end.date()
                } else {
                    e.start.date() == day
                }
            })
            .collect()
    }

    pub fn capacity(&self, day: Date) -> f64 {
        self.free_blocks(day).iter().map(|b| b.hours()).sum()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const TEMPLATE: &str = "tests/fixtures/vault-s1/config/week_template.yaml";

    fn cal(events: Vec<CalEvent>) -> WeekCalendar {
        WeekCalendar::from_file(Path::new(TEMPLATE), events)
    }

    /// The golden file's headline number, computed by hand in the module docs.
    #[test]
    fn fixture_friday_capacity_is_exactly_8_25_hours() {
        let friday = Date::constant(2026, 8, 28);
        let blocks = cal(Vec::new()).free_blocks(friday);

        assert_eq!(
            blocks,
            vec![
                Block {
                    start: DateTime::constant(2026, 8, 28, 8, 0, 0, 0),
                    end: DateTime::constant(2026, 8, 28, 12, 0, 0, 0),
                },
                Block {
                    start: DateTime::constant(2026, 8, 28, 13, 45, 0, 0),
                    end: DateTime::constant(2026, 8, 28, 18, 0, 0, 0),
                },
            ],
            "the 12:50-13:00 gap must be dropped by min_block_minutes"
        );
        assert_eq!(cal(Vec::new()).capacity(friday), 8.25);
    }

    #[test]
    fn a_day_with_no_classes_is_the_whole_day() {
        // 2026-08-29 is a Saturday; the fixture has sat: [].
        assert_eq!(cal(Vec::new()).capacity(Date::constant(2026, 8, 29)), 10.0);
    }

    #[test]
    fn template_capacity_ignores_events_and_capacity_does_not() {
        let wednesday = Date::constant(2026, 8, 26);
        let event = CalEvent {
            title: "Dentist".into(),
            start: DateTime::constant(2026, 8, 26, 9, 0, 0, 0),
            end: DateTime::constant(2026, 8, 26, 11, 0, 0, 0),
            all_day: false,
        };
        let with_event = cal(vec![event]);
        assert!(with_event.capacity(wednesday) < with_event.template_capacity(wednesday));
    }

    #[test]
    fn all_day_events_never_subtract_capacity() {
        let friday = Date::constant(2026, 8, 28);
        let all_day = CalEvent {
            title: "Reading day".into(),
            start: DateTime::constant(2026, 8, 28, 0, 0, 0, 0),
            end: DateTime::constant(2026, 8, 29, 0, 0, 0, 0),
            all_day: true,
        };
        assert_eq!(cal(vec![all_day]).capacity(friday), 8.25);
    }

    #[test]
    fn an_inverted_event_subtracts_nothing() {
        let blocks = vec![Block {
            start: DateTime::constant(2026, 8, 28, 8, 0, 0, 0),
            end: DateTime::constant(2026, 8, 28, 12, 0, 0, 0),
        }];
        let inverted = subtract(
            &blocks,
            DateTime::constant(2026, 8, 28, 11, 0, 0, 0),
            DateTime::constant(2026, 8, 28, 10, 0, 0, 0),
        );
        assert_eq!(inverted, blocks);
    }

    #[test]
    fn subtracting_the_middle_splits_a_block_in_two() {
        let blocks = vec![Block {
            start: DateTime::constant(2026, 8, 28, 8, 0, 0, 0),
            end: DateTime::constant(2026, 8, 28, 12, 0, 0, 0),
        }];
        let out = subtract(
            &blocks,
            DateTime::constant(2026, 8, 28, 9, 0, 0, 0),
            DateTime::constant(2026, 8, 28, 10, 0, 0, 0),
        );
        assert_eq!(out.len(), 2);
        assert_eq!(out[0].hours(), 1.0);
        assert_eq!(out[1].hours(), 2.0);
    }

    #[test]
    fn a_block_of_exactly_the_minimum_is_kept() {
        // min_filter uses >=, so 45 minutes qualifies.
        let c = cal(Vec::new());
        let exactly = Block {
            start: DateTime::constant(2026, 8, 28, 8, 0, 0, 0),
            end: DateTime::constant(2026, 8, 28, 8, 45, 0, 0),
        };
        assert_eq!(c.min_filter(vec![exactly]).len(), 1);
    }

    #[test]
    fn parse_hm_reads_the_template_format() {
        assert_eq!(parse_hm("08:00"), Time::constant(8, 0, 0, 0));
        assert_eq!(parse_hm("13:45"), Time::constant(13, 45, 0, 0));
    }

    // --- P3: commitment spans, instances and the window ---

    #[test]
    fn window_falls_back_to_template_day_start_and_end() {
        let friday = Date::constant(2026, 8, 28);
        let calendar = cal(Vec::new());
        assert_eq!(calendar.window(friday), (calendar.day_start, calendar.day_end));
    }

    #[test]
    fn a_window_entry_replaces_that_weekday_only() {
        let wednesday = Date::constant(2026, 8, 26);
        let saturday = Date::constant(2026, 8, 29);
        let mut window = [None; 7];
        for slot in window.iter_mut().take(5) {
            // mon..fri
            *slot = Some((Time::constant(8, 0, 0, 0), Time::constant(22, 0, 0, 0)));
        }
        let calendar = cal(Vec::new()).with_commitments(Vec::new(), window);

        let wed_blocks = calendar.template_blocks(wednesday);
        let last = wed_blocks.last().expect("Wednesday has at least one free block");
        assert_eq!(
            last.end,
            DateTime::constant(2026, 8, 26, 22, 0, 0, 0),
            "the window entry moves the last block's end to 22:00"
        );

        assert_eq!(
            calendar.template_blocks(saturday),
            vec![Block {
                start: DateTime::constant(2026, 8, 29, 8, 0, 0, 0),
                end: DateTime::constant(2026, 8, 29, 18, 0, 0, 0),
            }],
            "Saturday has no window entry, so it keeps the template's day_start/day_end"
        );
    }

    #[test]
    fn a_commitment_span_is_busy_on_its_weekday_within_from_until() {
        let from = Date::constant(2026, 8, 24); // Monday
        let until = Date::constant(2026, 9, 4); // Friday, two weeks later
        let spans = vec![
            CommitmentSpan {
                day: "mon",
                start: Time::constant(12, 0, 0, 0),
                end: Time::constant(12, 50, 0, 0),
                from: Some(from),
                until: Some(until),
                title: "Study group".into(),
                kind: "club".into(),
                source_uid: "test:study-group-mon".into(),
            },
            CommitmentSpan {
                day: "wed",
                start: Time::constant(12, 0, 0, 0),
                end: Time::constant(12, 50, 0, 0),
                from: Some(from),
                until: Some(until),
                title: "Study group".into(),
                kind: "club".into(),
                source_uid: "test:study-group-wed".into(),
            },
        ];
        let calendar =
            WeekCalendar::new(&Mapping::new(), Vec::new()).with_commitments(spans, [None; 7]);

        let monday_in_range = Date::constant(2026, 8, 24);
        assert_eq!(
            calendar.template_blocks(monday_in_range),
            vec![
                Block {
                    start: DateTime::constant(2026, 8, 24, 8, 0, 0, 0),
                    end: DateTime::constant(2026, 8, 24, 12, 0, 0, 0),
                },
                Block {
                    start: DateTime::constant(2026, 8, 24, 12, 50, 0, 0),
                    end: DateTime::constant(2026, 8, 24, 18, 0, 0, 0),
                },
            ],
            "the span splits Monday's block around 12:00-12:50"
        );

        let wednesday_in_range = Date::constant(2026, 8, 26);
        assert_eq!(
            calendar.template_blocks(wednesday_in_range).len(),
            2,
            "Wednesday's span splits its day too"
        );

        let monday_before_from = Date::constant(2026, 8, 17);
        assert_eq!(
            calendar.template_blocks(monday_before_from),
            vec![Block {
                start: DateTime::constant(2026, 8, 17, 8, 0, 0, 0),
                end: DateTime::constant(2026, 8, 17, 18, 0, 0, 0),
            }],
            "before from, the span is absent"
        );

        let monday_after_until = Date::constant(2026, 9, 7);
        assert_eq!(
            calendar.template_blocks(monday_after_until),
            vec![Block {
                start: DateTime::constant(2026, 9, 7, 8, 0, 0, 0),
                end: DateTime::constant(2026, 9, 7, 18, 0, 0, 0),
            }],
            "after until, the span is absent"
        );
    }

    #[test]
    fn inside_the_horizon_actual_instances_replace_the_weekly_span() {
        let span = CommitmentSpan {
            day: "tue",
            start: Time::constant(15, 0, 0, 0),
            end: Time::constant(15, 50, 0, 0),
            from: None,
            until: None,
            title: "Tuesday club".into(),
            kind: "club".into(),
            source_uid: "test:tue-club".into(),
        };
        let horizon_start = Date::constant(2026, 8, 24);
        let horizon_end = Date::constant(2026, 9, 21); // horizon_start + 28 days
        let moved_date = Date::constant(2026, 8, 25); // Tuesday, inside the horizon, moved
        let mut instances = BTreeMap::new();
        instances.insert(
            "test:tue-club".to_string(),
            (
                horizon_start,
                horizon_end,
                vec![(moved_date, Time::constant(16, 0, 0, 0), Time::constant(16, 50, 0, 0))],
            ),
        );
        let calendar = WeekCalendar::new(&Mapping::new(), Vec::new())
            .with_commitments(vec![span], [None; 7])
            .with_instances(instances);

        assert_eq!(
            calendar.template_blocks(moved_date),
            vec![
                Block {
                    start: DateTime::constant(2026, 8, 25, 8, 0, 0, 0),
                    end: DateTime::constant(2026, 8, 25, 16, 0, 0, 0),
                },
                Block {
                    start: DateTime::constant(2026, 8, 25, 16, 50, 0, 0),
                    end: DateTime::constant(2026, 8, 25, 18, 0, 0, 0),
                },
            ],
            "a moved instance is busy at its new time, not the weekly 15:00-15:50"
        );

        let cancelled_date = Date::constant(2026, 9, 1); // Tuesday, inside the horizon, no instance
        assert_eq!(
            calendar.template_blocks(cancelled_date),
            vec![Block {
                start: DateTime::constant(2026, 9, 1, 8, 0, 0, 0),
                end: DateTime::constant(2026, 9, 1, 18, 0, 0, 0),
            }],
            "a cancelled date has no instance, so the day is free"
        );

        let past_horizon_date = Date::constant(2026, 9, 22); // Tuesday, past horizon_end
        assert_eq!(
            calendar.template_blocks(past_horizon_date),
            vec![
                Block {
                    start: DateTime::constant(2026, 9, 22, 8, 0, 0, 0),
                    end: DateTime::constant(2026, 9, 22, 15, 0, 0, 0),
                },
                Block {
                    start: DateTime::constant(2026, 9, 22, 15, 50, 0, 0),
                    end: DateTime::constant(2026, 9, 22, 18, 0, 0, 0),
                },
            ],
            "a day past the horizon uses the weekly span"
        );
    }

    #[test]
    fn template_only_blocks_ignores_commitment_spans() {
        let wednesday = Date::constant(2026, 8, 26);
        let span = CommitmentSpan {
            day: "wed",
            start: Time::constant(8, 15, 0, 0),
            end: Time::constant(8, 30, 0, 0),
            from: None,
            until: None,
            title: "Morning club".into(),
            kind: "club".into(),
            source_uid: "test:morning-club".into(),
        };
        let without_spans = cal(Vec::new());
        let with_spans = cal(Vec::new()).with_commitments(vec![span], [None; 7]);

        // The span would split the 08:00-09:00 free block if it were not ignored.
        assert_ne!(with_spans.template_blocks(wednesday), without_spans.template_blocks(wednesday));
        assert_eq!(
            with_spans.template_only_blocks(wednesday),
            without_spans.template_blocks(wednesday),
            "template_only_blocks must not paint a club as a class"
        );
    }

    #[test]
    fn with_day_window_changes_only_that_weekday() {
        let wednesday = Date::constant(2026, 8, 26);
        let thursday = Date::constant(2026, 8, 27);
        let calendar = cal(Vec::new()).with_day_window(
            wednesday,
            Time::constant(8, 0, 0, 0),
            Time::constant(22, 0, 0, 0),
        );

        assert_eq!(
            calendar.window(wednesday),
            (Time::constant(8, 0, 0, 0), Time::constant(22, 0, 0, 0))
        );
        assert_eq!(calendar.window(thursday), (calendar.day_start, calendar.day_end));
    }

    #[test]
    fn overlapping_spans_behave_like_overlapping_classes() {
        let monday = Date::constant(2026, 8, 24); // TEMPLATE's Monday has CS 100 at 12:00-12:50
        let duplicate = CommitmentSpan {
            day: "mon",
            start: Time::constant(12, 0, 0, 0),
            end: Time::constant(12, 50, 0, 0),
            from: None,
            until: None,
            title: "CS 100".into(),
            kind: "class".into(),
            source_uid: "test:cs-100-dup".into(),
        };
        let without_span = cal(Vec::new());
        let with_span = cal(Vec::new()).with_commitments(vec![duplicate], [None; 7]);

        assert_eq!(with_span.template_blocks(monday), without_span.template_blocks(monday));
    }

    // --- P3 fix round 1: I-1 (instances handled once per source), M-1 (inverted spans/windows),
    // M-2 (partial-overlap and early-window coverage) ---

    fn cal_with_yaml(yaml: &str, events: Vec<CalEvent>) -> WeekCalendar {
        let config = match serde_yaml_ng::from_str::<Value>(yaml).expect("valid test yaml") {
            Value::Mapping(m) => m,
            _ => Mapping::new(),
        };
        WeekCalendar::new(&config, events)
    }

    #[test]
    fn a_multi_day_note_inside_the_horizon_is_drawn_once() {
        let source_uid = "test:mw-class";
        let spans = vec![
            CommitmentSpan {
                day: "mon",
                start: Time::constant(12, 0, 0, 0),
                end: Time::constant(12, 50, 0, 0),
                from: None,
                until: None,
                title: "CS 200".into(),
                kind: "class".into(),
                source_uid: source_uid.into(),
            },
            CommitmentSpan {
                day: "wed",
                start: Time::constant(12, 0, 0, 0),
                end: Time::constant(12, 50, 0, 0),
                from: None,
                until: None,
                title: "CS 200".into(),
                kind: "class".into(),
                source_uid: source_uid.into(),
            },
        ];
        let horizon_start = Date::constant(2026, 8, 24);
        let horizon_end = Date::constant(2026, 9, 21);
        let monday = Date::constant(2026, 8, 24);
        let mut instances = BTreeMap::new();
        instances.insert(
            source_uid.to_string(),
            (
                horizon_start,
                horizon_end,
                vec![(monday, Time::constant(12, 0, 0, 0), Time::constant(12, 50, 0, 0))],
            ),
        );
        let calendar = WeekCalendar::new(&Mapping::new(), Vec::new())
            .with_commitments(spans, [None; 7])
            .with_instances(instances);

        assert_eq!(
            calendar.spans_on(monday).len(),
            1,
            "the Mon and Wed spans share one source_uid, so Monday's one instance is drawn once, \
             not twice"
        );
    }

    #[test]
    fn two_instances_on_one_date_are_both_busy() {
        let span = CommitmentSpan {
            day: "thu",
            start: Time::constant(9, 0, 0, 0),
            end: Time::constant(9, 50, 0, 0),
            from: None,
            until: None,
            title: "Doubled session".into(),
            kind: "class".into(),
            source_uid: "test:doubled".into(),
        };
        let horizon_start = Date::constant(2026, 8, 24);
        let horizon_end = Date::constant(2026, 9, 21);
        let thursday = Date::constant(2026, 8, 27);
        let mut instances = BTreeMap::new();
        instances.insert(
            "test:doubled".to_string(),
            (
                horizon_start,
                horizon_end,
                vec![
                    (thursday, Time::constant(10, 0, 0, 0), Time::constant(10, 50, 0, 0)),
                    (thursday, Time::constant(14, 0, 0, 0), Time::constant(14, 50, 0, 0)),
                ],
            ),
        );
        let calendar = WeekCalendar::new(&Mapping::new(), Vec::new())
            .with_commitments(vec![span], [None; 7])
            .with_instances(instances);

        assert_eq!(
            calendar.spans_on(thursday).len(),
            2,
            "both instances of the one source on the same date are drawn"
        );
        assert_eq!(
            calendar.template_blocks(thursday),
            vec![
                Block {
                    start: DateTime::constant(2026, 8, 27, 8, 0, 0, 0),
                    end: DateTime::constant(2026, 8, 27, 10, 0, 0, 0),
                },
                Block {
                    start: DateTime::constant(2026, 8, 27, 10, 50, 0, 0),
                    end: DateTime::constant(2026, 8, 27, 14, 0, 0, 0),
                },
                Block {
                    start: DateTime::constant(2026, 8, 27, 14, 50, 0, 0),
                    end: DateTime::constant(2026, 8, 27, 18, 0, 0, 0),
                },
            ],
            "both instances subtract capacity, not just the first found"
        );
    }

    #[test]
    fn an_inverted_span_is_dropped_rather_than_double_counted() {
        let monday = Date::constant(2026, 8, 24);
        let inverted = CommitmentSpan {
            day: "mon",
            start: Time::constant(12, 0, 0, 0),
            end: Time::constant(11, 0, 0, 0), // end before start
            from: None,
            until: None,
            title: "Bad span".into(),
            kind: "club".into(),
            source_uid: "test:inverted".into(),
        };
        let calendar =
            WeekCalendar::new(&Mapping::new(), Vec::new()).with_commitments(vec![inverted], [None; 7]);

        assert_eq!(
            calendar.template_blocks(monday),
            vec![Block {
                start: DateTime::constant(2026, 8, 24, 8, 0, 0, 0),
                end: DateTime::constant(2026, 8, 24, 18, 0, 0, 0),
            }],
            "an inverted span subtracts nothing, rather than producing overlapping free blocks"
        );
    }

    #[test]
    fn with_day_window_ignores_an_inverted_or_zero_length_window() {
        let day = Date::constant(2026, 8, 26);

        let inverted = cal(Vec::new()).with_day_window(
            day,
            Time::constant(10, 0, 0, 0),
            Time::constant(9, 0, 0, 0),
        );
        assert_eq!(inverted.window(day), (inverted.day_start, inverted.day_end));

        let zero_length = cal(Vec::new()).with_day_window(
            day,
            Time::constant(10, 0, 0, 0),
            Time::constant(10, 0, 0, 0),
        );
        assert_eq!(zero_length.window(day), (zero_length.day_start, zero_length.day_end));
    }

    #[test]
    fn a_span_partially_overlapping_a_class_unions_the_busy_time() {
        let yaml = "day_start: \"08:00\"\nday_end: \"18:00\"\nmin_block_minutes: 45\nclasses:\n  mon:\n    - [\"12:00\", \"12:50\"]\n";
        let monday = Date::constant(2026, 8, 24);
        let span = CommitmentSpan {
            day: "mon",
            start: Time::constant(12, 30, 0, 0),
            end: Time::constant(13, 30, 0, 0),
            from: None,
            until: None,
            title: "Overlapping club".into(),
            kind: "club".into(),
            source_uid: "test:overlap".into(),
        };
        let calendar = cal_with_yaml(yaml, Vec::new()).with_commitments(vec![span], [None; 7]);

        assert_eq!(
            calendar.template_blocks(monday),
            vec![
                Block {
                    start: DateTime::constant(2026, 8, 24, 8, 0, 0, 0),
                    end: DateTime::constant(2026, 8, 24, 12, 0, 0, 0),
                },
                Block {
                    start: DateTime::constant(2026, 8, 24, 13, 30, 0, 0),
                    end: DateTime::constant(2026, 8, 24, 18, 0, 0, 0),
                },
            ],
            "the class and the overlapping span union into one busy stretch, with no gap between \
             them"
        );
    }

    #[test]
    fn a_window_starting_before_the_template_actually_starts_earlier() {
        let friday = Date::constant(2026, 8, 28); // TEMPLATE fixture, day_start 08:00
        let calendar = cal(Vec::new()).with_day_window(
            friday,
            Time::constant(7, 0, 0, 0),
            Time::constant(18, 0, 0, 0),
        );

        let blocks = calendar.template_blocks(friday);
        let first = blocks.first().expect("Friday has at least one free block");
        assert_eq!(
            first.start,
            DateTime::constant(2026, 8, 28, 7, 0, 0, 0),
            "an earlier window start is honoured, not clamped to the template's day_start"
        );
    }

    // --- P15: `for_vault` and the no-commitments equivalence (spec §6.1, §8) ---

    const FIXTURE_VAULTS: [&str; 3] =
        ["tests/fixtures/vault-s1", "tests/fixtures/vault-s1-migrated", "tests/fixtures/vault-full"];

    /// The 35 days spec §8 test 1 names, starting 2026-08-24.
    fn horizon_35(start: Date) -> Vec<Date> {
        (0..35).map(|n| crate::scheduling::add_days(start, n)).collect()
    }

    /// `for_vault(vault, ..)` and `from_file(vault/config/week_template.yaml, ..)` must draw the
    /// identical calendar over `days` — spec §8 test 1's five-way comparison, shared by the fixture
    /// test and the decline-markers test.
    fn assert_for_vault_equals_from_file(vault: &Path, days: &[Date]) {
        let template = vault.join("config").join("week_template.yaml");
        let from_file = WeekCalendar::from_file(&template, Vec::new());
        let for_vault = WeekCalendar::for_vault(vault, Vec::new());
        for &day in days {
            assert_eq!(
                for_vault.template_blocks(day),
                from_file.template_blocks(day),
                "template_blocks disagree on {day}"
            );
            assert_eq!(
                for_vault.free_blocks(day),
                from_file.free_blocks(day),
                "free_blocks disagree on {day}"
            );
            assert_eq!(
                for_vault.capacity(day),
                from_file.capacity(day),
                "capacity disagrees on {day}"
            );
            assert_eq!(
                for_vault.template_capacity(day),
                from_file.template_capacity(day),
                "template_capacity disagrees on {day}"
            );
            assert_eq!(for_vault.window(day), from_file.window(day), "window disagrees on {day}");
        }
    }

    #[test]
    fn for_vault_equals_from_file_without_commitments() {
        let days = horizon_35(Date::constant(2026, 8, 24));
        for name in FIXTURE_VAULTS {
            assert_for_vault_equals_from_file(Path::new(name), &days);
        }
    }

    #[test]
    fn template_only_blocks_equals_template_blocks_without_commitments() {
        let days = horizon_35(Date::constant(2026, 8, 24));
        for name in FIXTURE_VAULTS {
            let for_vault = WeekCalendar::for_vault(Path::new(name), Vec::new());
            for &day in &days {
                assert_eq!(
                    for_vault.template_only_blocks(day),
                    for_vault.template_blocks(day),
                    "{name} {day}: no commitments, so template_only_blocks must match template_blocks"
                );
            }
        }
    }

    fn copy_dir_all(src: &Path, dst: &Path) {
        std::fs::create_dir_all(dst).unwrap();
        for entry in std::fs::read_dir(src).unwrap() {
            let entry = entry.unwrap();
            let target = dst.join(entry.file_name());
            if entry.file_type().unwrap().is_dir() {
                copy_dir_all(&entry.path(), &target);
            } else {
                std::fs::copy(entry.path(), &target).unwrap();
            }
        }
    }

    /// A private scratch copy of `tests/fixtures/vault-full`, safe for a test to add
    /// `commitments/` notes into. Every call gets its own directory so parallel tests never
    /// collide.
    fn scratch_vault_full(tag: &str) -> std::path::PathBuf {
        static COUNTER: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(0);
        let n = COUNTER.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
        let dir = std::env::temp_dir().join(format!("knowlu-p15-{}-{tag}-{n}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        copy_dir_all(Path::new("tests/fixtures/vault-full"), &dir);
        std::fs::create_dir_all(dir.join("commitments")).unwrap();
        dir
    }

    #[test]
    fn decline_markers_change_nothing() {
        let days = horizon_35(Date::constant(2026, 8, 24));
        let vault = scratch_vault_full("decline");
        std::fs::write(
            vault.join("commitments").join("declined.md"),
            "---\ntype: commitment\nstatus: declined\nsource_uid: \"gcal-series:declined-club\"\n---\n\nInvented.\n",
        )
        .unwrap();

        assert_for_vault_equals_from_file(&vault, &days);

        let _ = std::fs::remove_dir_all(&vault);
    }

    #[test]
    fn a_confirmed_class_note_reduces_capacity_on_its_days() {
        static COUNTER: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(0);
        let n = COUNTER.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
        let vault =
            std::env::temp_dir().join(format!("knowlu-p15-class-{}-{n}", std::process::id()));
        let _ = std::fs::remove_dir_all(&vault);
        std::fs::create_dir_all(vault.join("commitments")).unwrap();
        std::fs::write(
            vault.join("commitments").join("class.md"),
            "---\ntype: commitment\nkind: class\ntitle: \"CS 100\"\nmeets: [{days: [mon, wed], start: \"12:00\", end: \"12:50\"}]\nstatus: confirmed\n---\n\nInvented.\n",
        )
        .unwrap();

        let baseline = WeekCalendar::new(&Mapping::new(), Vec::new());
        let with_class = WeekCalendar::for_vault(&vault, Vec::new());
        let _ = std::fs::remove_dir_all(&vault);

        // Both Mon and Wed carry the 12:00-12:50 class (50 minutes).
        for day in [Date::constant(2026, 8, 24), Date::constant(2026, 8, 26)] {
            let dropped = baseline.capacity(day) - with_class.capacity(day);
            assert!(
                (dropped - 50.0 / 60.0).abs() < 1e-9,
                "{day}: capacity must drop by exactly the 50-minute overlap, got {dropped}"
            );
        }
        // Tuesday has no class in this note, so it is untouched.
        let tuesday = Date::constant(2026, 8, 25);
        assert_eq!(with_class.capacity(tuesday), baseline.capacity(tuesday));
    }

    #[test]
    fn for_vault_with_warnings_surfaces_commitments_load_warnings() {
        let vault = scratch_vault_full("warnings");
        std::fs::write(
            vault.join("commitments").join("bad.md"),
            "---\ntype: commitment\nkind: class\ntitle: \"Bad\"\nstatus: confirmed\n---\n\nInvented.\n",
        )
        .unwrap();

        let template = vault.join("config").join("week_template.yaml");
        let from_file = WeekCalendar::from_file(&template, Vec::new());
        let (calendar, warnings) = WeekCalendar::for_vault_with_warnings(&vault, Vec::new());
        let _ = std::fs::remove_dir_all(&vault);

        assert!(
            warnings.iter().any(|w| w.contains("no valid meets entry")),
            "expected a no-valid-meets warning, got {warnings:?}"
        );
        // The invalid note is skipped entirely, so it contributes no span.
        let monday = Date::constant(2026, 8, 24);
        assert_eq!(calendar.template_blocks(monday), from_file.template_blocks(monday));
    }

    #[test]
    fn for_vault_with_warnings_is_empty_with_no_commitments_folder() {
        let (_, warnings) =
            WeekCalendar::for_vault_with_warnings(Path::new("tests/fixtures/vault-full"), Vec::new());
        assert!(warnings.is_empty(), "{warnings:?}");
    }
}
