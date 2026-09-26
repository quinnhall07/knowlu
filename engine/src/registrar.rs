//! Phase 3 of the commitment model (spec `docs/specs/2026-09-26-commitment-model-phase3-design.md`):
//! the school registrar's schedule, parsed on the device. **Every Banner key this crate reads is in
//! [`parse_banner`]** (Plan ruling R1-a); its fixture is
//! `engine/tests/fixtures/registrar/banner-ua-registration.json`. Nothing here fetches: the app
//! carries the bytes (spec D4).
use std::collections::BTreeSet;

use jiff::civil::{Date, Time};
use serde_json::Value;

use crate::commitments::{Meet, Rule, Series};
use crate::weekcal::DayKey;

/// A school whose registrar Knowlu reads. Adding one is a code change (parent §10).
pub struct School {
    /// The `--school` value and the middle of every key: `registrar:<key>:<term>-<crn>`.
    pub key: &'static str,
    /// `(first month, term-code suffix)`, in month order (Plan ruling R1-f).
    pub terms: &'static [(i8, &'static str)],
}

/// UA only before the pilot (Quinn, 2026-09-23).
pub const SCHOOLS: [School; 1] = [School { key: "ua", terms: &[(1, "10"), (6, "40")] }];

/// Every registrar calendar key, and every registrar `source_uid`, starts with this.
pub const CALENDAR_PREFIX: &str = "registrar:";

/// A registrar series' `event_type` (spec §3's table).
pub const EVENT_TYPE: &str = "registrar";

/// §2.2's bound on `where`.
const WHERE_MAX: usize = 80;

pub fn school(key: &str) -> Option<&'static School> {
    SCHOOLS.iter().find(|s| s.key == key)
}

/// One calendar per term (Plan ruling R2-a): `registrar:ua:202640`.
pub fn calendar_key(school: &str, term: &str) -> String {
    format!("{CALENDAR_PREFIX}{school}:{term}")
}

/// The term to fetch on `today` (Plan ruling R1-f): the last `terms` entry whose month has come.
pub fn term_for(school_key: &str, today: Date) -> Option<String> {
    let s = school(school_key)?;
    let (_, suffix) = s.terms.iter().rev().find(|(month, _)| *month <= today.month())?;
    Some(format!("{}{suffix}", today.year()))
}

/// Plan ruling R1-g: a signed-out session gets the school's HTML sign-in page, never JSON.
pub fn looks_signed_out(body: &str) -> bool {
    !matches!(serde_json::from_str::<Value>(body.trim()), Ok(Value::Array(_)) | Ok(Value::Object(_)))
}

/// What [`parse_banner`] kept and dropped.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct Parsed {
    pub term: String,
    /// Sorted by `source_uid`.
    pub series: Vec<Series>,
    /// Rows with no meeting time (online, TBA): dropped and counted.
    pub no_time: usize,
    /// Rows with a meeting across midnight: dropped with a warning (§2.2).
    pub midnight: usize,
    pub warnings: Vec<String>,
}

const DAY_FIELDS: [(&str, DayKey); 7] = [
    ("monday", "mon"), ("tuesday", "tue"), ("wednesday", "wed"), ("thursday", "thu"),
    ("friday", "fri"), ("saturday", "sat"), ("sunday", "sun"),
];

/// A trimmed, non-empty string field.
fn text<'a>(v: &'a Value, key: &str) -> Option<&'a str> {
    v.get(key).and_then(Value::as_str).map(str::trim).filter(|s| !s.is_empty())
}

/// Banner's `"0930"`.
fn hhmm(v: &Value, key: &str) -> Option<Time> {
    let s = text(v, key)?;
    if s.len() != 4 || !s.bytes().all(|b| b.is_ascii_digit()) {
        return None;
    }
    Time::new(s[..2].parse().ok()?, s[2..].parse().ok()?, 0, 0).ok()
}

/// Banner's `"MM/DD/YYYY"`.
fn mdy(v: &Value, key: &str) -> Option<Date> {
    Date::strptime("%m/%d/%Y", text(v, key)?).ok()
}

/// Plan ruling R1-c.
fn place(mt: &Value) -> Option<String> {
    let building = text(mt, "buildingDescription").or_else(|| text(mt, "building"));
    let joined = [building, text(mt, "room")].into_iter().flatten().collect::<Vec<_>>().join(" ");
    let cut: String = joined.chars().take(WHERE_MAX).collect::<String>().trim_end().to_string();
    (!cut.is_empty()).then_some(cut)
}

/// Banner 9's registration rows → [`Series`] (spec §3; shape (b) as R0 recorded it). A row is kept
/// when it has a term, a CRN, a subject, a course number and at least one pattern with days and
/// both times (R1-d). `Err` when no row is kept: an empty parse is a failure, never an empty
/// semester. `school` goes into the key only.
pub fn parse_banner(json: &Value, school: &str) -> Result<Parsed, String> {
    let rows = match json {
        Value::Array(rows) => rows,
        _ => json.get("data").and_then(Value::as_array).ok_or("not a list of registration rows")?,
    };
    let mut out = Parsed::default();
    let mut kept: BTreeSet<String> = BTreeSet::new();
    for row in rows {
        let (Some(term), Some(crn), Some(subject), Some(number)) =
            (text(row, "term"), text(row, "courseReferenceNumber"), text(row, "subject"), text(row, "courseNumber"))
        else {
            out.warnings.push("registrar: a row without a term, CRN, subject or course number; skipped".into());
            continue;
        };
        let name = format!("{subject} {number}");
        // A whole word, so "Collaborative Seminar" is not a lab (plan review M2).
        let lab = text(row, "scheduleTypeDescription").is_some_and(|t| {
            t.split(|c: char| !c.is_ascii_alphabetic()).any(|w| w.eq_ignore_ascii_case("lab") || w.eq_ignore_ascii_case("laboratory"))
        }) || text(row, "scheduleType").is_some_and(|t| t.eq_ignore_ascii_case("LAB"));
        let mut meets: Vec<Meet> = Vec::new();
        let (mut first, mut until, mut place_of, mut crosses) = (None::<Date>, None::<Date>, None, false);
        for mf in row.get("meetingsFaculty").and_then(Value::as_array).into_iter().flatten() {
            let Some(mt) = mf.get("meetingTime") else { continue };
            let days: Vec<DayKey> = DAY_FIELDS.iter().filter(|(f, _)| mt.get(*f).and_then(Value::as_bool) == Some(true)).map(|(_, d)| *d).collect();
            let (Some(start), Some(end)) = (hhmm(mt, "beginTime"), hhmm(mt, "endTime")) else { continue };
            if days.is_empty() {
                continue;
            }
            if end <= start {
                crosses = true;
                continue;
            }
            let meet = Meet { days, start, end };
            if !meets.contains(&meet) {
                meets.push(meet);
            }
            first = [first, mdy(mt, "startDate")].into_iter().flatten().min();
            until = [until, mdy(mt, "endDate")].into_iter().flatten().max();
            place_of = place_of.or_else(|| place(mt));
        }
        if crosses {
            out.midnight += 1;
            out.warnings.push(format!("registrar: {name} meets across midnight; dropped"));
            continue;
        }
        if meets.is_empty() {
            out.no_time += 1;
            continue;
        }
        if out.term.is_empty() {
            out.term = term.to_string();
        } else if out.term != term {
            out.warnings.push(format!("registrar: {name} is in term {term}, not {}; skipped", out.term));
            continue;
        }
        let key = format!("{CALENDAR_PREFIX}{school}:{term}-{crn}");
        if !kept.insert(key.clone()) {
            out.warnings.push(format!("registrar: CRN {crn} listed twice; the first kept"));
            continue;
        }
        out.series.push(Series {
            source_uid: key,
            calendar: calendar_key(school, term),
            title: if lab { format!("{name} Lab") } else { name },
            where_: place_of,
            event_type: Some(EVENT_TYPE.to_string()),
            rule: Rule { freq: "WEEKLY".into(), interval: 1, until, count: None },
            has_master: true,
            rdate: false,
            unsupported: false,
            instances: Vec::new(),
            meets,
            first,
            until,
            last_seen: None,
        });
    }
    if out.series.is_empty() {
        return Err(format!(
            "no class with meeting times in the registrar's answer ({} without times, {} across midnight)",
            out.no_time, out.midnight
        ));
    }
    out.series.sort_by(|a, b| a.source_uid.cmp(&b.source_uid));
    Ok(out)
}

#[cfg(test)]
mod tests {
    //! Phase 3 (spec `docs/specs/2026-09-26-commitment-model-phase3-design.md` §3). The fixture is
    //! hand-written and invented (R0); every other value here is invented too.
    use super::*;
    use jiff::civil::{date, time};
    use serde_json::json;

    pub(crate) const FIXTURE: &str = include_str!("../tests/fixtures/registrar/banner-ua-registration.json");

    fn fixture() -> Value {
        serde_json::from_str(FIXTURE).unwrap()
    }

    fn by_crn<'a>(p: &'a Parsed, crn: &str) -> &'a Series {
        let key = format!("registrar:ua:202640-{crn}");
        p.series.iter().find(|s| s.source_uid == key).unwrap_or_else(|| panic!("{key} not kept: {:?}", p.series))
    }

    #[test]
    fn the_fixture_parses_to_four_series_of_one_term() {
        let p = parse_banner(&fixture(), "ua").unwrap();
        assert_eq!(p.term, "202640");
        let keys: Vec<&str> = p.series.iter().map(|s| s.source_uid.as_str()).collect();
        assert_eq!(keys, ["registrar:ua:202640-40001", "registrar:ua:202640-40002", "registrar:ua:202640-40003", "registrar:ua:202640-40006"]);
        assert_eq!((p.no_time, p.midnight), (1, 1));
        assert!(p.warnings.iter().any(|w| w.contains("MUS 250") && w.contains("midnight")), "{:?}", p.warnings);
    }

    #[test]
    fn a_row_becomes_the_series_the_spec_table_names() {
        let p = parse_banner(&fixture(), "ua").unwrap();
        let s = by_crn(&p, "40001");
        assert_eq!(s.calendar, "registrar:ua:202640");
        assert_eq!(s.title, "CS 100");
        assert_eq!(s.where_.as_deref(), Some("Invented Hall 101"));
        assert_eq!(s.event_type.as_deref(), Some("registrar"));
        assert_eq!(s.rule, Rule { freq: "WEEKLY".into(), interval: 1, until: Some(date(2026, 12, 4)), count: None });
        assert!(s.has_master && !s.rdate && !s.unsupported);
        assert!(s.instances.is_empty(), "a registrar series carries no instances");
        assert_eq!(s.meets, vec![Meet { days: vec!["mon", "wed", "fri"], start: time(12, 0, 0, 0), end: time(12, 50, 0, 0) }]);
        assert_eq!((s.first, s.until, s.last_seen), (Some(date(2026, 8, 19)), Some(date(2026, 12, 4)), None));
    }

    #[test]
    fn a_laboratory_is_titled_lab_and_a_repeated_pattern_is_one_meet() {
        let p = parse_banner(&fixture(), "ua").unwrap();
        assert_eq!(by_crn(&p, "40002").title, "CS 100 Lab");
        let engl = by_crn(&p, "40003");
        assert_eq!(engl.meets.len(), 1, "{:?}", engl.meets);
        assert_eq!(engl.meets[0].days, vec!["tue", "thu"]);
        assert_eq!(engl.where_.as_deref(), Some("Pretend Library 210"));
        let mut v = fixture();
        v["data"][0]["scheduleTypeDescription"] = json!("Collaborative Seminar");
        assert_eq!(by_crn(&parse_banner(&v, "ua").unwrap(), "40001").title, "CS 100", "a word inside a word is not a lab");
    }

    #[test]
    fn where_is_cut_to_80_and_a_row_without_a_building_has_none() {
        let long = "B".repeat(90);
        let row = |building: Value, room: Value| json!([{ "term": "202640", "courseReferenceNumber": "1", "subject": "ZZT",
            "courseNumber": "101", "scheduleTypeDescription": "Lecture", "meetingsFaculty": [{ "meetingTime": {
            "beginTime": "0800", "endTime": "0850", "monday": true, "startDate": "08/19/2026", "endDate": "12/04/2026",
            "buildingDescription": building, "room": room } }] }]);
        let p = parse_banner(&row(json!(long), json!("1")), "ua").unwrap();
        assert_eq!(p.series[0].where_.as_deref().map(|w| w.chars().count()), Some(80));
        let p = parse_banner(&row(Value::Null, Value::Null), "ua").unwrap();
        assert_eq!(p.series[0].where_, None);
    }

    #[test]
    fn an_empty_parse_is_a_failure_never_an_empty_semester() {
        assert!(parse_banner(&json!([]), "ua").is_err());
        assert!(parse_banner(&json!({ "data": [] }), "ua").is_err());
        assert!(parse_banner(&json!({ "nothing": 1 }), "ua").is_err());
        let online_only = json!([{ "term": "202640", "courseReferenceNumber": "9", "subject": "ZZT", "courseNumber": "150",
            "meetingsFaculty": [{ "meetingTime": { "beginTime": null, "endTime": null } }] }]);
        let err = parse_banner(&online_only, "ua").unwrap_err();
        assert!(err.contains("1 without times"), "{err}");
    }

    #[test]
    fn another_term_or_a_repeated_crn_is_skipped_with_a_warning() {
        let mut v = fixture();
        let rows = v["data"].as_array_mut().unwrap();
        let mut other = rows[0].clone();
        other["term"] = json!("202710");
        other["courseReferenceNumber"] = json!("50001");
        rows.push(other);
        rows.push(rows[1].clone());
        let p = parse_banner(&v, "ua").unwrap();
        assert_eq!(p.series.len(), 4);
        assert!(p.warnings.iter().any(|w| w.contains("202710")), "{:?}", p.warnings);
        assert!(p.warnings.iter().any(|w| w.contains("40002") && w.contains("twice")), "{:?}", p.warnings);
    }

    #[test]
    fn the_ua_term_is_spring_to_may_and_fall_from_june() {
        assert_eq!(term_for("ua", date(2026, 9, 26)).as_deref(), Some("202640"));
        assert_eq!(term_for("ua", date(2026, 6, 1)).as_deref(), Some("202640"));
        assert_eq!(term_for("ua", date(2027, 1, 4)).as_deref(), Some("202710"));
        assert_eq!(term_for("ua", date(2027, 5, 31)).as_deref(), Some("202710"));
        assert_eq!(term_for("zz", date(2026, 9, 26)), None);
        assert_eq!(calendar_key("ua", "202640"), "registrar:ua:202640");
    }

    #[test]
    fn a_sign_in_page_is_not_a_schedule() {
        assert!(looks_signed_out("<!DOCTYPE html><html><body>Sign in</body></html>"));
        assert!(looks_signed_out(""));
        assert!(looks_signed_out("\"just a string\""));
        assert!(!looks_signed_out(FIXTURE));
        assert!(!looks_signed_out(" [] "));
    }
}
