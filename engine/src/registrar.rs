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

#[cfg(test)]
mod d5_d7_tests {
    //! Phase 3, D5 and D7 in `commitments.rs`, driven with R1's parse of the invented fixture.
    use super::*;
    use crate::commitments::{self as cm, Class, Codes, Commitment, Commitments, Instance, Level, SeriesFile};
    use jiff::civil::date;
    use std::collections::{BTreeMap, BTreeSet};
    use std::path::PathBuf;

    const CAL: &str = "registrar:ua:202640";

    fn fall() -> Vec<Series> {
        parse_banner(&serde_json::from_str(super::tests::FIXTURE).unwrap(), "ua").unwrap().series
    }

    fn without(crn: &str) -> Vec<Series> {
        fall().into_iter().filter(|s| !s.source_uid.ends_with(crn)).collect()
    }

    fn codes() -> Codes {
        Codes { table: [("CS100".to_string(), "cs-100".to_string())].into_iter().collect(), names: BTreeMap::new() }
    }

    /// A scratch vault with only a timezone in `config/ingest.yaml`: no feed is configured.
    fn vault(name: &str) -> PathBuf {
        let dir = std::env::temp_dir().join(format!("knowlu-p3-{}-{name}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(dir.join("config")).unwrap();
        std::fs::write(dir.join("config").join("ingest.yaml"), "timezone: America/Chicago\n").unwrap();
        dir
    }

    fn keys(file: &SeriesFile) -> Vec<&str> {
        file.series.iter().map(|s| s.source_uid.as_str()).collect()
    }

    #[test]
    fn the_registrar_arm_takes_kind_from_the_title_and_course_from_the_code_table() {
        let f = fall();
        let class = |uid: &str| cm::classify(f.iter().find(|s| s.source_uid.ends_with(uid)).unwrap(), &codes(), &[]);
        assert_eq!(class("40001"), Some(Class::Kind { kind: "class".into(), course: Some("cs-100".into()) }));
        assert_eq!(class("40002"), Some(Class::Kind { kind: "lab".into(), course: Some("cs-100".into()) }));
        assert_eq!(class("40006"), Some(Class::Kind { kind: "class".into(), course: None }));
    }

    #[test]
    fn a_registrar_calendar_stays_configured_while_its_term_is_in_play() {
        let v = vault("inplay");
        cm::refresh_series(&v, &[(CAL.to_string(), fall())], date(2026, 8, 20));
        // Two months on, another calendar's fresh read rewrites the file: the term stays whole.
        let (file, _) = cm::refresh_series(&v, &[("personal".to_string(), Vec::new())], date(2026, 10, 20));
        assert_eq!(keys(&file).len(), 4, "{:?}", keys(&file));
        assert_eq!(file.calendars.get(CAL), Some(&date(2026, 8, 20)));
        // 28 days past the term's end (2026-12-04), it ages out like a removed feed: never `ended`.
        let (file, _) = cm::refresh_series(&v, &[("personal".to_string(), Vec::new())], date(2027, 1, 1));
        assert!(keys(&file).is_empty(), "{:?}", keys(&file));
        assert!(file.ended.is_empty() && !file.calendars.contains_key(CAL));
        let _ = std::fs::remove_dir_all(&v);
    }

    #[test]
    fn a_row_missing_from_a_fresh_fetch_of_its_term_ends_that_day() {
        let v = vault("dropped");
        cm::refresh_series(&v, &[(CAL.to_string(), fall())], date(2026, 8, 20));
        let (file, _) = cm::refresh_series(&v, &[(CAL.to_string(), without("40002"))], date(2026, 9, 10));
        assert_eq!(keys(&file).len(), 3);
        let gone = file.ended.get("registrar:ua:202640-40002").expect("the dropped lab is in ended");
        assert_eq!((gone.calendar.as_str(), gone.dropped), (CAL, date(2026, 9, 10)));
        assert_eq!((gone.last_instance, gone.until), (Some(date(2026, 9, 9)), Some(date(2026, 12, 4))));
        let _ = std::fs::remove_dir_all(&v);
    }

    #[test]
    fn a_next_terms_fetch_leaves_this_term_alone() {
        let v = vault("twoterms");
        cm::refresh_series(&v, &[(CAL.to_string(), fall())], date(2026, 10, 20));
        // 26 days later: past `UNSEEN_DAYS`, so only the in-play rule keeps the fall term (R2 m1).
        let spring: Vec<Series> = fall().into_iter().map(|mut s| {
            s.source_uid = s.source_uid.replace("202640", "202710");
            s.calendar = "registrar:ua:202710".into();
            s
        }).collect();
        let (file, _) = cm::refresh_series(&v, &[("registrar:ua:202710".to_string(), spring)], date(2026, 11, 15));
        assert_eq!(keys(&file).len(), 8, "{:?}", keys(&file));
        assert!(file.ended.is_empty(), "{:?}", file.ended);
        let _ = std::fs::remove_dir_all(&v);
    }

    #[test]
    fn a_registrar_series_blocks_by_its_meets_not_by_instances() {
        let mut file = SeriesFile::default();
        file.calendars.insert(CAL.into(), date(2026, 9, 1));
        file.series = fall();
        assert!(file.instances_map().keys().all(|k| !k.starts_with(CALENDAR_PREFIX)), "{:?}", file.instances_map().keys());
    }
    /// D7: the registrar's CS 100 and a Google series with the same signature are one proposal,
    /// and it is the registrar's.
    #[test]
    fn a_registrar_series_outranks_its_google_twin() {
        let today = date(2026, 9, 1);
        let reg = fall().into_iter().find(|s| s.source_uid.ends_with("40001")).unwrap();
        let mut google = reg.clone();
        google.source_uid = "gcal-series:invented".into();
        google.calendar = "google:invented".into();
        google.event_type = Some("default".into());
        google.last_seen = Some(today);
        google.instances = [31, 2, 4, 7, 9, 11].iter().map(|d| {
            let day = if *d == 31 { date(2026, 8, 31) } else { date(2026, 9, *d) };
            Instance { date: day, start: Some(reg.meets[0].start), end: Some(reg.meets[0].end) }
        }).collect();
        let mut file = SeriesFile::default();
        for (cal, s) in [(CAL, reg), ("google:invented", google)] {
            file.calendars.insert(cal.into(), today);
            file.series.push(s);
        }
        file.series.sort_by(|a, b| (&a.source_uid, &a.calendar).cmp(&(&b.source_uid, &b.calendar)));
        let template = crate::weekcal::WeekCalendar::new(&serde_yaml_ng::Mapping::new(), Vec::new());
        let got = cm::proposals(&file, &Commitments::default(), &codes(), &[], &template, &BTreeSet::new(), today, false);
        let keys: Vec<&str> = got.iter().filter(|p| !p.is_window()).map(|p| p.source_uid.as_str()).collect();
        assert_eq!(keys, ["registrar:ua:202640-40001"]);
    }

    /// D5 + §5.4: a confirmed registrar note whose row left the term files an end card at the day
    /// before the fetch (R2-c's `last_instance`), and `registrar:` keys are watched (R2-g).
    #[test]
    fn a_dropped_registrar_course_ends_its_confirmed_note() {
        let v = vault("endcard");
        cm::refresh_series(&v, &[(CAL.to_string(), fall())], date(2026, 8, 20));
        let (file, _) = cm::refresh_series(&v, &[(CAL.to_string(), without("40002"))], date(2026, 9, 10));
        let lab = fall().into_iter().find(|s| s.source_uid.ends_with("40002")).unwrap();
        let note = Commitment {
            id: "cmt_invented01".into(), path: PathBuf::from("commitments/cs-100-lab.md"), kind: "lab".into(),
            level: Level::Hard, title: lab.title.clone(), course: Some("cs-100".into()), meets: lab.meets.clone(),
            where_: lab.where_.clone(), from: lab.first, until: lab.until, source_uid: Some(lab.source_uid.clone()),
        };
        let set = Commitments { confirmed: vec![note], ..Commitments::default() };
        let mut journal = crate::journal::Journal::new(&v);
        let fresh: BTreeSet<String> = [CAL.to_string()].into_iter().collect();
        let (changes, warnings) = cm::detect_changes(&file, &set, &codes(), &[], &fresh, date(2026, 9, 10), &mut journal);
        assert!(warnings.is_empty(), "{warnings:?}");
        assert_eq!(changes.len(), 1, "{changes:?}");
        assert_eq!(changes[0].change.get("until").and_then(|u| u.as_str()), Some("2026-09-09"));
        let _ = std::fs::remove_dir_all(&v);
    }

    #[test]
    fn asks_wait_until_the_day_after_the_registrars_term_starts() {
        let mut file = SeriesFile::default();
        assert!(!cm::asks_wait_for_registrar(&file, date(2026, 8, 1)), "no registrar, no wait");
        file.series = fall();
        assert!(cm::asks_wait_for_registrar(&file, date(2026, 8, 1)));
        assert!(cm::asks_wait_for_registrar(&file, date(2026, 8, 19)), "the term's first day still waits");
        assert!(!cm::asks_wait_for_registrar(&file, date(2026, 8, 20)), "the day after it does not");
        assert!(!cm::asks_wait_for_registrar(&file, date(2026, 12, 20)), "a term already over holds nothing");
    }
}
