//! The pure builders for an accepted event: its shape, its instance payload, its commitment and
//! its register task.
//!
//! Spec `2026-09-29-events-design.md` §4.2, §5.1 and Q1, Q1a, Q1b, with the plan's P4 (one shape
//! classifier), P5 (the `instances:` payload carries `url`), P6 (the past-midnight true end rides
//! in the commitment mapping as `ends:`) and PQ2 (a zero-length event goes in the all-day lane with
//! no commitment: Knowlu invents no hours). Nothing here reads or writes a vault: the settlement
//! (`approvals.rs`) and the carry (`eventcarry.rs`) call these and do the writing.

use jiff::civil::{Date, DateTime, Time};
use serde_yaml_ng::{Mapping, Value};

use crate::commitments::Level;
use crate::events::DiscoveredEvent;
use crate::judge::one_line;
use crate::yaml;
use crate::yamlemit::{safe_dump_block, Node};

/// The caps P5 gives the payload's free text, and the commitment's own (`title` 200, `where` 80).
const TITLE_MAX: usize = 200;
const LOCATION_MAX: usize = 120;
const URL_MAX: usize = 500;
const WHERE_MAX: usize = 80;
/// Characters of title slug in a `register-` file stem.
const SLUG_MAX: usize = 40;

/// Wall-clock, seconds, no zone: how `instances:` holds `start` and `end`, the carry line's own
/// span format (`eventledger::record_carried_answer`), so a card and a carry line for one instance
/// agree. A card written in minutes still reads (`from_yaml`).
const DATETIME_FORMAT: &str = "%Y-%m-%dT%H:%M:%S";
/// A past-midnight event books to 23:59 on its start day, so it must start before it.
const LAST_MINUTE: Time = Time::constant(23, 59, 0, 0);

/// How an event sits in a day (P4). The one classifier: the builders, the settlement and
/// `surface` all call [`shape`].
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Shape {
    /// `start < end` on one day: a commitment from start to end.
    Timed,
    /// Ends on the next day at or before its start's time of day (a 10pm-1am or 10pm-12am event):
    /// a commitment on the start day that ends at 23:59. `ends` is the true end as a clock label.
    PastMidnight { ends: String },
    /// All-day or multi-day: no commitment; drawn in the all-day lane on each day from `first` to
    /// `last`, both inclusive.
    AllDay { first: Date, last: Date },
    /// `end <= start` on one day at minute resolution (a sub-minute event included), or a
    /// past-midnight event that starts at 23:59: no commitment; drawn in the all-day lane on `day`
    /// (PQ2).
    ZeroLength { day: Date },
}

impl Shape {
    /// The days the all-day lane draws the event on, `(first, last)`, for the two lane shapes.
    pub fn lane(&self) -> Option<(Date, Date)> {
        match self {
            Shape::AllDay { first, last } => Some((*first, *last)),
            Shape::ZeroLength { day } => Some((*day, *day)),
            Shape::Timed | Shape::PastMidnight { .. } => None,
        }
    }
}

/// `10am`, `10:30pm`, or `midnight` for 00:00 (a true end of a past-midnight event).
fn ends_label(end: DateTime) -> String {
    if end.time() == Time::midnight() {
        "midnight".to_string()
    } else {
        format!("{} the next day", crate::eventemit::clock(end.time(), true))
    }
}

/// `at` without its seconds: the resolution a commitment books at.
fn to_minute(at: DateTime) -> DateTime {
    at.date().at(at.hour(), at.minute(), 0, 0)
}

/// `at` without its sub-seconds: the resolution the payload and the carry line store.
fn to_second(at: DateTime) -> DateTime {
    at.date().at(at.hour(), at.minute(), at.second(), 0)
}

/// P4's classifier over wall-clock `start` and `end`, at minute resolution, and the one place that
/// decides booking or lane: every `Timed` and `PastMidnight` shape books (`commitment_for`), every
/// other shape is drawn. The rules extend `eventemit::when_label`'s: an end at 00:00 on a later day
/// belongs to the day before it.
pub fn shape(start: DateTime, end: DateTime) -> Shape {
    let (start, end) = (to_minute(start), to_minute(end));
    let midnight = Time::midnight();
    let last_day = if end.time() == midnight && end.date() > start.date() {
        end.date().yesterday().unwrap_or(start.date())
    } else {
        end.date().max(start.date())
    };
    if start.time() == midnight && end.time() == midnight {
        return Shape::AllDay { first: start.date(), last: last_day };
    }
    let next_day = start.date().tomorrow().ok();
    if Some(end.date()) == next_day && end.time() <= start.time() {
        if start.time() >= LAST_MINUTE {
            // No room before the 23:59 booking end: drawn, not booked (PQ2).
            return Shape::ZeroLength { day: start.date() };
        }
        return Shape::PastMidnight { ends: ends_label(end) };
    }
    if last_day > start.date() {
        return Shape::AllDay { first: start.date(), last: last_day };
    }
    if end <= start {
        return Shape::ZeroLength { day: start.date() };
    }
    Shape::Timed
}

/// One event instance as an `event-accept` or `event-check` card carries it in `instances:`
/// (P5): the settlement's only input (D10). Free text is already one line and clipped.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Instance {
    pub uid: String,
    pub title: String,
    pub start: DateTime,
    pub end: DateTime,
    pub location: String,
    pub url: String,
    pub registration: bool,
    pub registration_deadline: Option<Date>,
}

fn parse_datetime(value: Option<&Value>) -> Option<DateTime> {
    yaml::text(value?)?.trim().parse::<DateTime>().ok()
}

impl Instance {
    pub fn from_event(event: &DiscoveredEvent) -> Instance {
        Instance {
            uid: event.uid.clone(),
            title: one_line(&event.title, TITLE_MAX),
            start: to_second(event.start()),
            end: to_second(event.end()),
            location: one_line(&event.location, LOCATION_MAX),
            url: one_line(&event.url, URL_MAX),
            registration: event.registration,
            registration_deadline: event.registration_deadline,
        }
    }

    /// The payload mapping, `start` and `end` formatted from the real datetimes (never a bare
    /// datetime scalar: see the module note in `eventemit.rs` on preserved defect 15).
    pub fn to_node(&self) -> Node {
        Node::map(vec![
            ("uid", Node::text(&self.uid)),
            ("title", Node::text(&self.title)),
            ("start", Node::text(&self.start.strftime(DATETIME_FORMAT).to_string())),
            ("end", Node::text(&self.end.strftime(DATETIME_FORMAT).to_string())),
            ("location", Node::text(&self.location)),
            ("url", Node::text(&self.url)),
            ("registration", Node::Bool(self.registration)),
            (
                "registration_deadline",
                self.registration_deadline.map(Node::Date).unwrap_or(Node::Null),
            ),
        ])
    }

    /// One `instances:` entry read back. A malformed entry (no uid, a start or end that is not a
    /// datetime, a registration that is not a boolean, a deadline that is not a date) reads as
    /// `None`.
    pub fn from_yaml(map: &Mapping) -> Option<Instance> {
        let uid = yaml::opt_text(yaml::get(map, "uid"))?.trim().to_string();
        if uid.is_empty() {
            return None;
        }
        let registration = match yaml::get(map, "registration") {
            None | Some(Value::Null) => false,
            Some(Value::Bool(b)) => *b,
            Some(_) => return None,
        };
        let registration_deadline = match yaml::get(map, "registration_deadline") {
            None | Some(Value::Null) => None,
            Some(value) => Some(yaml::text(value)?.trim().parse::<Date>().ok()?),
        };
        let text = |key: &str| yaml::opt_text(yaml::get(map, key)).unwrap_or_default();
        Some(Instance {
            uid,
            title: text("title"),
            start: parse_datetime(yaml::get(map, "start"))?,
            end: parse_datetime(yaml::get(map, "end"))?,
            location: text("location"),
            url: text("url"),
            registration,
            registration_deadline,
        })
    }
}

fn string(text: &str) -> Value {
    Value::String(text.to_string())
}

/// The mapping `commitments::create_confirmed` takes for an accepted instance (spec §4.2 step 1),
/// or `None` for the lane shapes (all-day, multi-day, zero-length: Q1b, PQ2). [`shape`] alone
/// decides which: its booking shapes always have `start < end` at minute resolution, as `meets`
/// needs. `level` is `Hard` for an obligation and `Soft` for an opportunity. Times are the
/// instance's wall clock, to the minute. A past-midnight event books to 23:59 on its start day and
/// carries its true end in `ends` (P6), which only the note's body reads.
pub fn commitment_for(instance: &Instance, level: Level) -> Option<Mapping> {
    let (end_time, ends) = match shape(instance.start, instance.end) {
        Shape::Timed => (instance.end.strftime("%H:%M").to_string(), None),
        Shape::PastMidnight { ends } => ("23:59".to_string(), Some(ends)),
        Shape::AllDay { .. } | Shape::ZeroLength { .. } => return None,
    };
    let start_time = instance.start.strftime("%H:%M").to_string();
    debug_assert!(start_time < end_time, "shape books only start < end at minute resolution");
    let day = instance.start.date();
    let title = one_line(&instance.title, TITLE_MAX);
    let mut meet = Mapping::new();
    meet.insert(string("days"), Value::Sequence(vec![string(crate::planning::day_key(day))]));
    meet.insert(string("start"), string(&start_time));
    meet.insert(string("end"), string(&end_time));
    let mut map = Mapping::new();
    map.insert(string("kind"), string("event"));
    map.insert(string("level"), string(level.as_str()));
    map.insert(string("title"), string(if title.is_empty() { "(untitled)" } else { &title }));
    map.insert(string("meets"), Value::Sequence(vec![Value::Mapping(meet)]));
    map.insert(string("from"), string(&day.to_string()));
    map.insert(string("until"), string(&day.to_string()));
    let location = one_line(&instance.location, usize::MAX);
    if (1..=WHERE_MAX).contains(&location.chars().count()) {
        map.insert(string("where"), string(&location));
    }
    if let Some(ends) = ends {
        map.insert(string("ends"), string(&ends));
    }
    Some(map)
}

/// `Thu 1 Oct`.
fn day_label(day: Date) -> String {
    format!("{} {} {}", day.strftime("%a"), day.day(), day.strftime("%b"))
}

/// The "Register" task for an instance whose feed says registration is required (spec §4.2 step
/// 2): the note's file stem (`register-<slug>-<start date>`) and its full text, frontmatter through `safe_dump_block` and then the
/// body. `None` unless `registration` is set. With no deadline the task is undated (`due: null`):
/// Knowlu never invents one, and the event's start is named only in the body, as an upper bound.
pub fn register_task(instance: &Instance) -> Option<(String, String)> {
    if !instance.registration {
        return None;
    }
    let title = one_line(&instance.title, TITLE_MAX);
    let title = if title.is_empty() { "(untitled)".to_string() } else { title };
    let due = instance
        .registration_deadline
        .map(|d| Node::text(&format!("{}T23:59", d.strftime("%Y-%m-%d"))))
        .unwrap_or(Node::Null);
    let front = Node::map(vec![
        ("title", Node::text(&format!("Register: {title}"))),
        ("due", due),
        ("effort_hours", Node::Float(0.25)),
        ("importance", Node::Int(3)),
        ("domain", Node::text("school")),
        ("status", Node::text("active")),
        ("created_by", Node::text("events")),
        ("source_uid", Node::text(&format!("register:{}", instance.uid))),
    ]);
    let mut body = match instance.registration_deadline {
        Some(deadline) => format!("Register for this event by {}.", day_label(deadline)),
        None => {
            let start = instance.start;
            let when = if start.time() == Time::midnight() {
                day_label(start.date())
            } else {
                format!("{} {}", day_label(start.date()), crate::eventemit::clock(start.time(), true))
            };
            format!(
                "Registration closes by {when} at the latest; check the event page for the real \
                 deadline."
            )
        }
    };
    let url = one_line(&instance.url, URL_MAX);
    if !url.is_empty() {
        body.push_str(&format!("\n\n{url}"));
    }
    let text = format!("---\n{}---\n\n{body}\n", safe_dump_block(&front));
    // Unique per instance, not per title: two series titled alike, or a task the student already
    // keeps, must not make `write::create` fail on `Exists` and stall the card (spec §4.2, D10).
    let slug: String = crate::ingest::slugify(&title).chars().take(SLUG_MAX).collect();
    let stem = format!(
        "register-{}-{}",
        slug.trim_end_matches('-'),
        instance.start.strftime("%Y-%m-%d")
    );
    Some((stem, text))
}

#[cfg(test)]
mod tests {
    use super::*;
    use jiff::civil::date;

    fn at(month: i8, day: i8, hour: i8, minute: i8) -> DateTime {
        date(2026, month, day).at(hour, minute, 0, 0)
    }

    fn instance(start: DateTime, end: DateTime) -> Instance {
        Instance {
            uid: "localist:77:1".into(),
            title: "Career fair".into(),
            start,
            end,
            location: "Ferguson Center".into(),
            url: "https://example.edu/events/77".into(),
            registration: false,
            registration_deadline: None,
        }
    }

    fn text_of(map: &Mapping, key: &str) -> Option<String> {
        crate::yaml::get(map, key).and_then(crate::yaml::text)
    }

    fn meets_of(map: &Mapping) -> Vec<Mapping> {
        match crate::yaml::get(map, "meets") {
            Some(Value::Sequence(items)) => items
                .iter()
                .map(|v| match v {
                    Value::Mapping(m) => m.clone(),
                    other => panic!("meets entry is not a mapping: {other:?}"),
                })
                .collect(),
            other => panic!("no meets: {other:?}"),
        }
    }

    #[test]
    fn a_timed_one_day_event_gives_the_commitment_mapping() {
        // Thu 1 Oct 2026, 10am-3pm.
        let inst = instance(at(10, 1, 10, 0), at(10, 1, 15, 0));
        assert_eq!(shape(inst.start, inst.end), Shape::Timed);
        let hard = commitment_for(&inst, Level::Hard).expect("a timed event books");
        assert_eq!(text_of(&hard, "kind").as_deref(), Some("event"));
        assert_eq!(text_of(&hard, "level").as_deref(), Some("hard"));
        assert_eq!(text_of(&hard, "title").as_deref(), Some("Career fair"));
        assert_eq!(text_of(&hard, "from").as_deref(), Some("2026-10-01"));
        assert_eq!(text_of(&hard, "until").as_deref(), Some("2026-10-01"));
        assert_eq!(text_of(&hard, "where").as_deref(), Some("Ferguson Center"));
        assert!(crate::yaml::get(&hard, "ends").is_none(), "no ends key on a timed event");
        let meets = meets_of(&hard);
        assert_eq!(meets.len(), 1);
        assert_eq!(text_of(&meets[0], "start").as_deref(), Some("10:00"));
        assert_eq!(text_of(&meets[0], "end").as_deref(), Some("15:00"));
        let days = match crate::yaml::get(&meets[0], "days") {
            Some(Value::Sequence(d)) => d.iter().filter_map(crate::yaml::text).collect::<Vec<_>>(),
            other => panic!("no days: {other:?}"),
        };
        assert_eq!(days, vec!["thu".to_string()]);
        // An opportunity is soft.
        let soft = commitment_for(&inst, Level::Soft).unwrap();
        assert_eq!(text_of(&soft, "level").as_deref(), Some("soft"));
        // The weekday key is DAY_KEYS' own, for every day.
        for offset in 0..7 {
            let day = date(2026, 10, 5).checked_add(jiff::Span::new().days(offset)).unwrap();
            let i = instance(day.at(9, 0, 0, 0), day.at(10, 0, 0, 0));
            let m = commitment_for(&i, Level::Hard).unwrap();
            let got = match crate::yaml::get(&meets_of(&m)[0], "days") {
                Some(Value::Sequence(d)) => crate::yaml::text(&d[0]).unwrap(),
                other => panic!("no days: {other:?}"),
            };
            assert_eq!(got, crate::planning::DAY_KEYS[offset as usize]);
        }
    }

    #[test]
    fn a_timed_event_cuts_its_title_to_200_and_keeps_a_where_of_one_to_80_characters() {
        let mut inst = instance(at(10, 1, 10, 0), at(10, 1, 15, 0));
        inst.title = "T".repeat(300);
        inst.location = "L".repeat(80);
        let m = commitment_for(&inst, Level::Hard).unwrap();
        assert_eq!(text_of(&m, "title").unwrap().chars().count(), 200);
        assert_eq!(text_of(&m, "where").unwrap().chars().count(), 80);
        inst.location = "L".repeat(81);
        assert!(crate::yaml::get(&commitment_for(&inst, Level::Hard).unwrap(), "where").is_none());
        inst.location = String::new();
        assert!(crate::yaml::get(&commitment_for(&inst, Level::Hard).unwrap(), "where").is_none());
    }

    #[test]
    fn a_past_midnight_event_ends_at_2359_and_carries_its_true_end() {
        // 10pm-1am: the commitment ends at 23:59 on the start day and says when it truly ends.
        let inst = instance(at(10, 1, 22, 0), at(10, 2, 1, 0));
        assert_eq!(shape(inst.start, inst.end), Shape::PastMidnight { ends: "1am the next day".into() });
        let m = commitment_for(&inst, Level::Hard).unwrap();
        let meets = meets_of(&m);
        assert_eq!(text_of(&meets[0], "start").as_deref(), Some("22:00"));
        assert_eq!(text_of(&meets[0], "end").as_deref(), Some("23:59"));
        assert_eq!(text_of(&m, "from").as_deref(), Some("2026-10-01"));
        assert_eq!(text_of(&m, "until").as_deref(), Some("2026-10-01"));
        assert_eq!(text_of(&m, "ends").as_deref(), Some("1am the next day"));
        // 10pm-12am: the end is midnight.
        let inst = instance(at(10, 1, 22, 0), at(10, 2, 0, 0));
        assert_eq!(shape(inst.start, inst.end), Shape::PastMidnight { ends: "midnight".into() });
        let m = commitment_for(&inst, Level::Soft).unwrap();
        assert_eq!(text_of(&m, "ends").as_deref(), Some("midnight"));
        assert_eq!(text_of(&meets_of(&m)[0], "end").as_deref(), Some("23:59"));
    }

    #[test]
    fn an_all_day_or_multi_day_event_gives_the_lane_marker_and_no_commitment() {
        // One all-day day: the feed gives midnight to the next midnight.
        let one = instance(at(10, 1, 0, 0), at(10, 2, 0, 0));
        let lane = shape(one.start, one.end);
        assert_eq!(lane, Shape::AllDay { first: date(2026, 10, 1), last: date(2026, 10, 1) });
        assert_eq!(lane.lane(), Some((date(2026, 10, 1), date(2026, 10, 1))));
        // Three all-day days: an end at 00:00 belongs to the day before.
        let three = instance(at(10, 1, 0, 0), at(10, 4, 0, 0));
        assert_eq!(
            shape(three.start, three.end),
            Shape::AllDay { first: date(2026, 10, 1), last: date(2026, 10, 3) }
        );
        // Fri 2 Oct 5pm - Sun 4 Oct 2pm.
        let weekend = instance(at(10, 2, 17, 0), at(10, 4, 14, 0));
        let lane = shape(weekend.start, weekend.end);
        assert_eq!(lane, Shape::AllDay { first: date(2026, 10, 2), last: date(2026, 10, 4) });
        assert_eq!(lane.lane(), Some((date(2026, 10, 2), date(2026, 10, 4))));
        for inst in [&one, &three, &weekend] {
            for level in [Level::Hard, Level::Soft] {
                assert!(commitment_for(inst, level).is_none());
            }
        }
        // A timed shape is no lane.
        assert_eq!(shape(at(10, 1, 10, 0), at(10, 1, 11, 0)).lane(), None);
        assert_eq!(shape(at(10, 1, 22, 0), at(10, 2, 1, 0)).lane(), None);
    }

    #[test]
    fn a_zero_length_event_gives_the_lane_marker() {
        // PQ2: 3pm-3pm, and 3pm-2pm on one day. No hours are invented.
        for (start, end) in [(at(10, 1, 15, 0), at(10, 1, 15, 0)), (at(10, 1, 15, 0), at(10, 1, 14, 0))] {
            let found = shape(start, end);
            assert_eq!(found, Shape::ZeroLength { day: date(2026, 10, 1) });
            assert_eq!(found.lane(), Some((date(2026, 10, 1), date(2026, 10, 1))));
            assert!(!matches!(found, Shape::PastMidnight { .. }));
            let inst = instance(start, end);
            for level in [Level::Hard, Level::Soft] {
                assert!(commitment_for(&inst, level).is_none());
            }
        }
    }

    #[test]
    fn a_minute_degenerate_event_is_zero_length_and_goes_in_the_lane() {
        // R1 minor 1: the shape alone decides booking or lane. A past-midnight event that starts at
        // 23:59 has no room before the 23:59 booking end, and a timed event shorter than a minute
        // has none at minute resolution: both are zero-length (PQ2), drawn, never booked.
        let late = (at(10, 1, 23, 59), at(10, 2, 0, 30));
        let blink = (date(2026, 10, 1).at(10, 0, 0, 0), date(2026, 10, 1).at(10, 0, 40, 0));
        let late_seconds = (date(2026, 10, 1).at(23, 59, 30, 0), at(10, 2, 1, 0));
        for (start, end) in [late, blink, late_seconds] {
            let found = shape(start, end);
            assert_eq!(found, Shape::ZeroLength { day: date(2026, 10, 1) }, "{start} -> {end}");
            assert_eq!(found.lane(), Some((date(2026, 10, 1), date(2026, 10, 1))));
            let inst = instance(start, end);
            for level in [Level::Hard, Level::Soft] {
                assert!(commitment_for(&inst, level).is_none());
            }
        }
        // A shape that is no lane always books.
        for (start, end) in [
            (at(10, 1, 23, 58), at(10, 2, 0, 30)),
            (date(2026, 10, 1).at(10, 0, 0, 0), date(2026, 10, 1).at(10, 1, 10, 0)),
        ] {
            let found = shape(start, end);
            assert_eq!(found.lane(), None, "{start} -> {end}");
            assert!(commitment_for(&instance(start, end), Level::Hard).is_some(), "{start} -> {end}");
        }
    }

    fn registering(deadline: Option<Date>) -> Instance {
        let mut inst = instance(at(10, 1, 10, 0), at(10, 1, 15, 0));
        inst.registration = true;
        inst.registration_deadline = deadline;
        inst
    }

    fn task_front(text: &str) -> Mapping {
        crate::models::split_frontmatter(text).expect("a task note with frontmatter").0
    }

    #[test]
    fn two_instances_with_one_title_get_different_register_stems() {
        let mut first = registering(None);
        let mut second = registering(None);
        first.start = at(10, 1, 10, 0);
        second.start = at(10, 8, 10, 0);
        second.uid = "localist:77:2".into();
        let (a, _) = register_task(&first).unwrap();
        let (b, _) = register_task(&second).unwrap();
        assert_eq!(a, "register-career-fair-2026-10-01");
        assert_eq!(b, "register-career-fair-2026-10-08");
        assert_ne!(a, b);
    }

    #[test]
    fn a_long_title_clips_its_register_stem_to_40_characters_of_slug() {
        let mut inst = registering(None);
        inst.title = "An exceedingly long career fair title that runs on and on".into();
        let (stem, _) = register_task(&inst).unwrap();
        let slug = stem.strip_prefix("register-").unwrap().strip_suffix("-2026-10-01").unwrap();
        assert!(slug.chars().count() <= 40 && !slug.ends_with('-'), "{slug}");
    }

    #[test]
    fn the_register_task_is_due_at_the_deadline_at_2359() {
        let inst = registering(Some(date(2026, 9, 28)));
        let (stem, text) = register_task(&inst).expect("registration: true gives a task");
        assert_eq!(stem, "register-career-fair-2026-10-01");
        let meta = task_front(&text);
        assert_eq!(text_of(&meta, "title").as_deref(), Some("Register: Career fair"));
        assert_eq!(text_of(&meta, "due").as_deref(), Some("2026-09-28T23:59"));
        assert_eq!(crate::yaml::opt_f64(crate::yaml::get(&meta, "effort_hours"), 0.0), 0.25);
        assert_eq!(crate::yaml::opt_i64(crate::yaml::get(&meta, "importance"), 0), 3);
        assert_eq!(text_of(&meta, "domain").as_deref(), Some("school"));
        assert_eq!(text_of(&meta, "status").as_deref(), Some("active"));
        assert_eq!(text_of(&meta, "created_by").as_deref(), Some("events"));
        assert_eq!(text_of(&meta, "source_uid").as_deref(), Some("register:localist:77:1"));
        assert!(text.starts_with("---\n"));
    }

    #[test]
    fn the_register_task_without_a_deadline_is_undated_and_names_the_start_as_an_upper_bound() {
        let inst = registering(None);
        let (_, text) = register_task(&inst).unwrap();
        let meta = task_front(&text);
        assert!(matches!(crate::yaml::get(&meta, "due"), Some(Value::Null)), "due is null: {meta:?}");
        // No frontmatter field holds the event's start.
        for (_, value) in meta.iter() {
            let shown = crate::yaml::text(value).unwrap_or_default();
            assert!(!shown.contains("2026-10-01"), "a field holds the event's date: {shown}");
        }
        let body = text.split("\n---\n").nth(1).expect("a body after the frontmatter");
        assert!(
            body.contains(
                "Registration closes by Thu 1 Oct 10am at the latest; check the event page for the \
                 real deadline."
            ),
            "{body}"
        );
        assert!(body.contains("https://example.edu/events/77"), "{body}");
    }

    #[test]
    fn no_register_task_unless_the_feed_says_registration() {
        let mut inst = registering(Some(date(2026, 9, 28)));
        inst.registration = false;
        assert!(register_task(&inst).is_none());
        inst.registration_deadline = None;
        assert!(register_task(&inst).is_none());
    }

    /// `instances:` entries as the dump of a card's list reads back, one mapping per entry.
    fn read_back(instances: &[Instance]) -> Vec<Mapping> {
        let front = Node::map(vec![("instances", Node::Seq(instances.iter().map(Instance::to_node).collect()))]);
        let parsed = crate::yaml::mapping_of(&crate::yamlemit::safe_dump_block(&front));
        match crate::yaml::get(&parsed, "instances") {
            Some(Value::Sequence(items)) => items
                .iter()
                .map(|v| match v {
                    Value::Mapping(m) => m.clone(),
                    other => panic!("not a mapping: {other:?}"),
                })
                .collect(),
            other => panic!("no instances: {other:?}"),
        }
    }

    #[test]
    fn an_instance_payload_round_trips() {
        // P5: built from a DiscoveredEvent, dumped, read back, the same instance.
        let mut event = DiscoveredEvent::new(
            "localist:77:1",
            "Career fair",
            at(10, 1, 10, 30),
            at(10, 1, 15, 0),
            "campus",
        );
        event.location = "Ferguson Center".into();
        event.url = "https://example.edu/events/77".into();
        event.registration = true;
        event.registration_deadline = Some(date(2026, 9, 28));
        let with_deadline = Instance::from_event(&event);
        assert_eq!(with_deadline.start, at(10, 1, 10, 30));
        assert_eq!(with_deadline.end, at(10, 1, 15, 0));
        // An instance without a deadline, an all-day one and a messy title round-trip too.
        let mut messy = DiscoveredEvent::new("a:2", "Two\nlines: and\ta colon", at(10, 1, 0, 0), at(10, 2, 0, 0), "campus");
        messy.title = format!("{}   {}", messy.title, "x".repeat(300));
        let messy = Instance::from_event(&messy);
        assert!(!messy.title.contains('\n') && messy.title.chars().count() <= 200, "{}", messy.title);
        let plain = Instance::from_event(&DiscoveredEvent::new("a:1", "T", at(10, 1, 9, 5), at(10, 1, 10, 0), "c"));
        let all = [with_deadline, plain, messy];
        let back: Vec<Option<Instance>> = read_back(&all).iter().map(Instance::from_yaml).collect();
        assert_eq!(back, all.iter().cloned().map(Some).collect::<Vec<_>>());
    }

    #[test]
    fn a_seconds_bearing_instance_round_trips_and_matches_the_carry_span() {
        // R1 minor 2: the payload keeps the feed's seconds, as the carry line does
        // (`eventledger::record_carried_answer`), so the card and the line agree. Sub-seconds,
        // which neither stores, are dropped on the way in, so the instance equals what is stored.
        let start = date(2026, 10, 1).at(10, 30, 45, 0);
        let end = date(2026, 10, 1).at(15, 0, 15, 500_000_000);
        let inst = Instance::from_event(&DiscoveredEvent::new("a:1", "T", start, end, "c"));
        assert_eq!(inst.start, start);
        assert_eq!(inst.end, date(2026, 10, 1).at(15, 0, 15, 0));
        let back = read_back(std::slice::from_ref(&inst));
        assert_eq!(text_of(&back[0], "start").as_deref(), Some("2026-10-01T10:30:45"));
        assert_eq!(text_of(&back[0], "end").as_deref(), Some("2026-10-01T15:00:15"));
        assert_eq!(Instance::from_yaml(&back[0]), Some(inst.clone()));
        // The carry line's span reads back as the same instants.
        let vault = std::env::temp_dir().join(format!("qo-accept-span-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&vault);
        std::fs::create_dir_all(vault.join("state")).unwrap();
        crate::eventledger::record_carried_answer(
            &vault,
            "a:1",
            "T",
            date(2026, 10, 1),
            "opportunity",
            "appr_0123456789",
            start,
            end,
        )
        .unwrap();
        let carry = crate::eventledger::load_ledger(&vault, None)["a:1"].carry.clone().expect("a carry line");
        assert_eq!((carry.start, carry.end), (inst.start, inst.end));
        let _ = std::fs::remove_dir_all(&vault);
        // A card written in minutes (the spec's own example) still reads.
        let minutes = crate::yaml::mapping_of("uid: a:1\nstart: 2026-10-01T10:30\nend: 2026-10-01T15:00\n");
        assert_eq!(Instance::from_yaml(&minutes).unwrap().start, at(10, 1, 10, 30));
    }

    #[test]
    fn from_event_clips_free_text_to_the_payload_caps() {
        let mut event = DiscoveredEvent::new("a:1", "T", at(10, 1, 9, 0), at(10, 1, 10, 0), "c");
        event.title = "t".repeat(500);
        event.location = "l".repeat(500);
        event.url = "u".repeat(900);
        let inst = Instance::from_event(&event);
        assert_eq!(inst.title.chars().count(), 200);
        assert_eq!(inst.location.chars().count(), 120);
        assert_eq!(inst.url.chars().count(), 500);
    }

    #[test]
    fn an_instance_stores_start_and_end_to_the_second_quoted_and_a_minute_card_still_reads() {
        // Spec §5.1's bytes: a seconds string matches YAML 1.1's timestamp pattern, so the
        // emitter quotes it; the minute spelling an older card holds reads as the same instance.
        let inst = instance(at(10, 1, 10, 0), at(10, 1, 15, 0));
        let bytes = safe_dump_block(&inst.to_node());
        assert!(bytes.contains("\nstart: '2026-10-01T10:00:00'\n"), "{bytes}");
        assert!(bytes.contains("\nend: '2026-10-01T15:00:00'\n"), "{bytes}");
        let stored = Instance::from_yaml(&crate::yaml::mapping_of(&bytes)).expect("reads back");
        assert_eq!((stored.start, stored.end), (inst.start, inst.end));
        let minutes = bytes
            .replace("'2026-10-01T10:00:00'", "2026-10-01T10:00")
            .replace("'2026-10-01T15:00:00'", "2026-10-01T15:00");
        let old = Instance::from_yaml(&crate::yaml::mapping_of(&minutes)).expect("a minute card reads");
        assert_eq!((old.start, old.end), (inst.start, inst.end));
    }

    #[test]
    fn a_malformed_instance_entry_reads_as_none() {
        let good = crate::yaml::mapping_of(
            "uid: a:1\ntitle: T\nstart: 2026-10-01T10:00\nend: 2026-10-01T11:00\nlocation: ''\nurl: ''\n\
             registration: false\nregistration_deadline: null\n",
        );
        assert!(Instance::from_yaml(&good).is_some());
        for broken in [
            "title: T\nstart: 2026-10-01T10:00\nend: 2026-10-01T11:00\n",
            "uid: a:1\ntitle: T\nstart: nonsense\nend: 2026-10-01T11:00\n",
            "uid: a:1\ntitle: T\nstart: 2026-10-01T10:00\n",
            "uid: a:1\ntitle: T\nstart: 2026-10-01T10:00\nend: 2026-10-01T11:00\nregistration_deadline: soon\n",
            "[]",
        ] {
            let map = crate::yaml::mapping_of(broken);
            assert!(Instance::from_yaml(&map).is_none(), "{broken}");
        }
    }

    #[test]
    fn a_commitment_mapping_is_one_create_confirmed_accepts() {
        // The mapping is the one `create_confirmed` takes, `ends` included; the note has no `ends`.
        let vault = std::env::temp_dir().join(format!("qo-accept-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&vault);
        std::fs::create_dir_all(vault.join("commitments")).unwrap();
        let mut journal = crate::journal::Journal::new(&vault);
        let ctx = crate::write::WriteContext::new("agent:test", "test");
        for (i, (start, end)) in
            [(at(10, 1, 10, 0), at(10, 1, 15, 0)), (at(10, 2, 22, 0), at(10, 3, 1, 0))].into_iter().enumerate()
        {
            let mut inst = instance(start, end);
            inst.uid = format!("localist:77:{i}");
            let map = commitment_for(&inst, Level::Hard).unwrap();
            let path = crate::commitments::create_confirmed(
                &vault,
                &map,
                &inst.uid,
                date(2026, 9, 30),
                &ctx,
                &mut journal,
            )
            .expect("create_confirmed takes the mapping");
            let note = std::fs::read_to_string(path).unwrap();
            assert!(!note.contains("\nends:"), "{note}");
            assert!(note.contains("kind: event"), "{note}");
        }
        let _ = std::fs::remove_dir_all(&vault);
    }
}
