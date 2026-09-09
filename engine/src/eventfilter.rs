//! The pre-filter that cuts the firehose to candidates — port of `engine/eventfilter.py`.
//!
//! Wave 5 of the Rust port.
//!
//! **Both halves of the return value matter.** `prefilter_events` returns `(candidates, dropped)`
//! and `cli.run` writes the dropped half into the roster's audit section, because spec §5 calls
//! that section *"the mechanism by which a wrong drop is recoverable by reading rather than by
//! missing"*. A port that returned only the survivors would silently delete the audit trail.

use std::sync::LazyLock;

use jiff::civil::{Date, Time};
use jiff::Span;
use regex::Regex;

use crate::events::{DiscoveredEvent, EventsConfig, Interests};

pub const EXCLUDED_AUDIENCES: [&str; 4] = ["faculty", "staff", "alumni", "graduate"];
pub const STANDING_EXHIBIT_DAYS: i64 = 7;

/// `\b{term}\b` per excluded term. The word boundaries are the whole point: without them
/// "graduate" matches "undergraduate" and the filter drops exactly the audience it exists to keep.
static AUDIENCE_PATTERNS: LazyLock<Vec<Regex>> = LazyLock::new(|| {
    EXCLUDED_AUDIENCES
        .iter()
        .filter_map(|term| Regex::new(&format!(r"\b{}\b", regex::escape(term))).ok())
        .collect()
});

fn haystack(event: &DiscoveredEvent) -> String {
    let parts = [
        event.title.clone(),
        event.organizer.clone(),
        event.categories.join(" "),
    ];
    parts.join(" ").to_lowercase()
}

/// A multi-week entry that starts at midnight is a gallery listing, not an event.
fn is_standing_exhibit(event: &DiscoveredEvent) -> bool {
    let start = event.start();
    let end = event.end();
    // Python subtracts two naive datetimes, which is an ABSOLUTE duration. jiff's `until` on a
    // civil DateTime yields a calendar span (months and days), and `total(Unit::Day)` on that
    // needs a reference date to resolve month lengths -- it errors, and the early return silently
    // stopped filtering every standing exhibit. `duration_until` is the faithful operation.
    let span = start.duration_until(end);
    if span.as_secs() < STANDING_EXHIBIT_DAYS * 24 * 60 * 60 {
        return false;
    }
    start.time() == Time::midnight()
}

/// Is this ONE audience label an excluded one? Word-boundary matching, so "graduate" does not
/// match "undergraduate".
fn audience_excluded(audience: &str) -> bool {
    AUDIENCE_PATTERNS.iter().any(|re| re.is_match(audience))
}

/// Cut the firehose to candidates. Returns `(candidates, filtered_out)`.
pub fn prefilter_events(
    events: &[DiscoveredEvent],
    interests: &Interests,
    config: &EventsConfig,
    today: Date,
) -> (Vec<DiscoveredEvent>, Vec<DiscoveredEvent>) {
    let window_start = today.to_datetime(Time::midnight());
    let window_end = window_start
        .checked_add(Span::new().try_days(config.roster_window_days).unwrap_or_default())
        .unwrap_or(window_start);
    let never: Vec<String> = interests
        .never
        .iter()
        .filter(|n| !n.is_empty())
        .map(|n| n.to_lowercase())
        .collect();

    let mut keep = Vec::new();
    let mut dropped = Vec::new();
    for event in events {
        if event.end() <= window_start || event.start() >= window_end {
            dropped.push(event.clone());
            continue;
        }
        if is_standing_exhibit(event) {
            dropped.push(event.clone());
            continue;
        }
        let hay = haystack(event);
        if never.iter().any(|term| hay.contains(term.as_str())) {
            dropped.push(event.clone());
            continue;
        }
        // Each audience label is judged on its own. Localist events routinely carry several, and
        // "open to everyone" is spelled as a list that INCLUDES Faculty/Staff and Alumni — so an
        // event is only excluded when NOTHING inclusive survives.
        let inclusive = event
            .audiences
            .iter()
            .filter(|a| !audience_excluded(&a.to_lowercase()))
            .count();
        if !event.audiences.is_empty() && inclusive == 0 {
            dropped.push(event.clone());
            continue;
        }
        keep.push(event.clone());
    }
    (keep, dropped)
}

#[cfg(test)]
mod tests {
    //! Direct port of `tests/test_event_filter.py` — all 16 tests, same names.

    use super::*;
    use jiff::civil::date;

    const TODAY: Date = Date::constant(2026, 8, 20);

    fn interests() -> Interests {
        Interests {
            strong: vec!["hackathons".into()],
            never: vec!["greek life".into(), "alumni".into()],
            ..Default::default()
        }
    }

    /// `event(**kwargs)` — the Python helper's defaults, then the overrides each test applies.
    fn event() -> DiscoveredEvent {
        DiscoveredEvent::new(
            "u1",
            "Some Event",
            date(2026, 8, 25).at(12, 0, 0, 0),
            date(2026, 8, 25).at(13, 0, 0, 0),
            "campus",
        )
    }

    fn run(e: DiscoveredEvent) -> (Vec<DiscoveredEvent>, Vec<DiscoveredEvent>) {
        prefilter_events(&[e], &interests(), &EventsConfig::default(), TODAY)
    }

    #[test]
    fn ordinary_event_is_a_candidate() {
        let (keep, drop) = run(event());
        assert_eq!(keep.len(), 1);
        assert!(drop.is_empty());
    }

    #[test]
    fn never_match_on_title_is_filtered() {
        let mut e = event();
        e.title = "Greek Life Recruitment Night".into();
        let (keep, drop) = run(e);
        assert!(keep.is_empty());
        assert_eq!(drop.len(), 1);
    }

    #[test]
    fn never_match_on_organizer_or_category_is_filtered() {
        let mut e = event();
        e.organizer = "Alumni Association".into();
        assert!(run(e).0.is_empty());

        let mut e = event();
        e.categories = vec!["Alumni".into()];
        assert!(run(e).0.is_empty());
    }

    #[test]
    fn excluded_audiences_are_filtered() {
        for audience in ["Faculty/Staff", "Alumni", "Graduate Students"] {
            let mut e = event();
            e.audiences = vec![audience.into()];
            assert!(run(e).0.is_empty(), "{audience}");
        }
    }

    #[test]
    fn student_audience_survives() {
        let mut e = event();
        e.audiences = vec!["Students".into()];
        assert_eq!(run(e).0.len(), 1);
    }

    #[test]
    fn past_events_are_filtered() {
        let mut e = event();
        e.start = Some(date(2026, 8, 1).at(12, 0, 0, 0));
        e.end = Some(date(2026, 8, 1).at(13, 0, 0, 0));
        assert!(run(e).0.is_empty());
    }

    #[test]
    fn events_beyond_the_roster_window_are_filtered() {
        let mut e = event();
        e.start = Some(date(2027, 1, 1).at(12, 0, 0, 0));
        e.end = Some(date(2027, 1, 1).at(13, 0, 0, 0));
        assert!(run(e).0.is_empty());
    }

    #[test]
    fn multi_week_standing_exhibit_is_filtered() {
        let mut e = event();
        e.title = "Made in America Exhibit".into();
        e.start = Some(date(2026, 8, 21).at(0, 0, 0, 0));
        e.end = Some(date(2026, 9, 30).at(0, 0, 0, 0));
        let (keep, drop) = run(e);
        assert!(keep.is_empty());
        assert_eq!(drop.len(), 1);
    }

    #[test]
    fn a_long_but_timed_event_survives() {
        // An all-day hackathon is long, but it has a real start time.
        let mut e = event();
        e.title = "24 Hour Hackathon".into();
        e.start = Some(date(2026, 8, 25).at(9, 0, 0, 0));
        e.end = Some(date(2026, 8, 26).at(9, 0, 0, 0));
        assert_eq!(run(e).0.len(), 1);
    }

    #[test]
    fn never_matching_is_case_insensitive_and_substring() {
        let mut e = event();
        e.title = "GREEK LIFE bid day".into();
        assert!(run(e).0.is_empty());
    }

    #[test]
    fn undergraduate_students_audience_survives() {
        // Word-boundary matching: "graduate" must not match "undergraduate". Regression test for
        // substring over-matching.
        let mut e = event();
        e.audiences = vec!["Undergraduate Students".into()];
        assert_eq!(
            run(e).0.len(),
            1,
            "Undergraduate Students should survive the filter"
        );
    }

    #[test]
    fn graduate_students_audience_is_filtered() {
        let mut e = event();
        e.audiences = vec!["Graduate Students".into()];
        assert!(run(e).0.is_empty(), "Graduate Students should be filtered");
    }

    #[test]
    fn mixed_audience_including_faculty_survives() {
        let mut e = event();
        e.audiences = vec!["Students".into(), "Faculty/Staff".into()];
        assert_eq!(run(e).0.len(), 1, "Students + Faculty/Staff should survive");
    }

    #[test]
    fn undergraduate_and_graduate_audiences_survive() {
        let mut e = event();
        e.audiences = vec!["Undergraduate Students".into(), "Graduate Students".into()];
        assert_eq!(
            run(e).0.len(),
            1,
            "open to undergrads survives even when grads are also listed"
        );
    }

    #[test]
    fn general_public_with_alumni_survives() {
        let mut e = event();
        e.audiences = vec!["General Public".into(), "Alumni".into()];
        assert_eq!(run(e).0.len(), 1, "General Public is an inclusive audience");
    }

    #[test]
    fn audience_list_that_is_entirely_excluded_is_filtered() {
        let mut e = event();
        e.audiences = vec!["Faculty/Staff".into(), "Alumni".into()];
        assert!(
            run(e).0.is_empty(),
            "no surviving audience means the event is not for Quinn"
        );
    }
}
