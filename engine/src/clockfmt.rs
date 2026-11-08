//! The one 12-hour clock formatter: `10am`, `10:30am`, `12pm`, `7–9pm`.
//!
//! Pure: it reads no clock, zone or locale, and takes no format input. One fixed format for every
//! student (twelve-hour-clock plan, Q2).
//!
//! # Its output is written into the vault
//!
//! These strings are not only displayed. They become vault bytes:
//!
//! - approval `title:` frontmatter, through `commitments::proposal_title` and
//!   `eventemit::what_and_when`;
//! - **approval file names**, because a card's title becomes its slug
//!   (`commitment-check-cs-100-mon-wed-fri-12-12-50pm-a-class.md`);
//! - commitment `ends:` frontmatter, through `eventaccept::ends_label`;
//! - the Register task's body.
//!
//! A change to any output therefore gives new notes a different format and different file names
//! from old ones. It needs a ruling and a plan of its own. The pins are
//! `commitments.rs`'s `a_class_proposal_files_one_card_with_the_exact_title_frontmatter_and_body`
//! and `eventaccept.rs`'s past-midnight test (`ends:`).

use jiff::civil::Time;

/// `10am`, `10:30`, `7pm`: a 12-hour clock, minutes only when non-zero. With `with_meridiem`
/// false the `am`/`pm` suffix is left off (a range's start on the same side of noon as its end).
pub fn clock(time: Time, with_meridiem: bool) -> String {
    let hour = match time.hour() % 12 {
        0 => 12,
        h => h,
    };
    let mut out = if time.minute() == 0 {
        hour.to_string()
    } else {
        format!("{hour}:{:02}", time.minute())
    };
    if with_meridiem {
        out.push_str(if time.hour() < 12 { "am" } else { "pm" });
    }
    out
}

/// `10am–3pm`, `7–9pm`, `10:30–11:15am`, `11am–12pm`, `12–1pm`. The end always carries am/pm;
/// the start carries it only when it is on the other side of noon. The dash is U+2013.
pub fn range(start: Time, end: Time) -> String {
    let split_by_noon = (start.hour() < 12) != (end.hour() < 12);
    format!("{}\u{2013}{}", clock(start, split_by_noon), clock(end, true))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn t(h: i8, m: i8) -> Time {
        Time::new(h, m, 0, 0).unwrap()
    }

    #[test]
    fn clock_covers_midnight_noon_and_the_edges() {
        assert_eq!(clock(t(0, 0), true), "12am");
        assert_eq!(clock(t(0, 30), true), "12:30am");
        assert_eq!(clock(t(9, 0), true), "9am");
        assert_eq!(clock(t(12, 0), true), "12pm");
        assert_eq!(clock(t(12, 50), true), "12:50pm");
        assert_eq!(clock(t(13, 5), true), "1:05pm");
        assert_eq!(clock(t(23, 59), true), "11:59pm");
    }

    #[test]
    fn clock_without_meridiem_drops_the_suffix() {
        assert_eq!(clock(t(0, 0), false), "12");
        assert_eq!(clock(t(9, 0), false), "9");
        assert_eq!(clock(t(12, 50), false), "12:50");
        assert_eq!(clock(t(13, 5), false), "1:05");
    }

    #[test]
    fn range_puts_am_pm_on_the_start_only_across_noon() {
        assert_eq!(range(t(10, 0), t(15, 0)), "10am\u{2013}3pm");
        assert_eq!(range(t(19, 0), t(21, 0)), "7\u{2013}9pm");
        assert_eq!(range(t(10, 30), t(11, 15)), "10:30\u{2013}11:15am");
        assert_eq!(range(t(11, 0), t(12, 0)), "11am\u{2013}12pm");
        assert_eq!(range(t(12, 0), t(13, 0)), "12\u{2013}1pm");
        assert_eq!(range(t(11, 30), t(12, 15)), "11:30am\u{2013}12:15pm");
        assert_eq!(range(t(8, 0), t(22, 0)), "8am\u{2013}10pm");
    }
}
