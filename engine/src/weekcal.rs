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

use std::collections::HashMap;
use std::path::Path;

use jiff::civil::{Date, DateTime, Time};
use serde_yaml_ng::{Mapping, Value};

use crate::planning::{day_key, DAY_KEYS};

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

pub fn parse_hm(text: &str) -> Time {
    let mut parts = text.trim().split(':');
    let hour: i8 = parts.next().and_then(|h| h.trim().parse().ok()).unwrap_or(0);
    let minute: i8 = parts.next().and_then(|m| m.trim().parse().ok()).unwrap_or(0);
    Time::new(hour, minute, 0, 0).unwrap_or(Time::midnight())
}

pub struct WeekCalendar {
    pub day_start: Time,
    pub day_end: Time,
    pub min_block_minutes: i64,
    classes: HashMap<&'static str, Vec<(Time, Time)>>,
    events: Vec<CalEvent>,
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

        WeekCalendar { day_start, day_end, min_block_minutes, classes, events }
    }

    pub fn from_file(path: &Path, events: Vec<CalEvent>) -> WeekCalendar {
        let text = std::fs::read_to_string(path).unwrap_or_default();
        let config = match serde_yaml_ng::from_str::<Value>(&text) {
            Ok(Value::Mapping(m)) => m,
            _ => Mapping::new(),
        };
        WeekCalendar::new(&config, events)
    }

    /// Free blocks from the timetable alone, WITHOUT the minimum-length filter.
    pub fn template_blocks(&self, day: Date) -> Vec<Block> {
        let mut busy: Vec<(DateTime, DateTime)> = self
            .classes
            .get(day_key(day))
            .map(|spans| {
                spans
                    .iter()
                    .map(|(s, e)| {
                        (DateTime::from_parts(day, *s), DateTime::from_parts(day, *e))
                    })
                    .collect()
            })
            .unwrap_or_default();
        busy.sort();

        let mut cursor = DateTime::from_parts(day, self.day_start);
        let end_of_day = DateTime::from_parts(day, self.day_end);
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
}
