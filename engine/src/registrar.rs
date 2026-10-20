//! Phase 3 of the commitment model (spec `docs/specs/2026-09-26-commitment-model-phase3-design.md`):
//! the school registrar's schedule, parsed on the device. **Every Banner key this crate reads is in
//! [`parse_banner`]** (Plan ruling R1-a); its fixture is
//! `engine/tests/fixtures/registrar/banner-ua-registration.json`. Nothing here fetches: the app
//! carries the bytes (spec D4).
use std::collections::BTreeSet;
use std::path::Path;

use jiff::civil::{Date, Time};
use serde_json::{json, Value};

use crate::commitments::{Meet, Rule, Series, SeriesFile};
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
    /// P3: `EXAM` meetings seen (a count of meetings, never a warning).
    pub exam: usize,
    /// P3: meetings whose `meetingType` is missing or unknown (meetings, warned).
    pub meeting_type: usize,
    /// P6: rows outside the school's registered set (rows, warned).
    pub status: usize,
    /// P9: every `courseReferenceNumber` of the answer, once, with how its row was read.
    pub seen: std::collections::BTreeMap<String, Reading>,
}

/// P9: how [`parse_banner`] read one row.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Reading {
    /// A series, no `CLAS` meeting skipped.
    Whole,
    /// A series, but a `CLAS` meeting (P4) or an unknown-type meeting (P3) was skipped.
    Partial,
    /// No series: P4 left no usable `CLAS` meeting, or only unknown-type meetings, or midnight.
    Unread,
    /// P6: the row's status is outside the registered set.
    Status,
}

/// [`parse_banner`] with the school's registered `statusIndicator` set given (P6). An empty set
/// means every row counts. TEST SKELETON (B4 step 2): the implementer replaces this body and has
/// `parse_banner` call it with `SCHOOLS`' set.
pub fn parse_banner_with(json: &Value, school: &str, _registered: &[&str]) -> Result<Parsed, String> {
    parse_banner(json, school)
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

/// What `commitments --registrar` did (spec §3's output).
#[derive(Debug, Clone, PartialEq, Default)]
pub struct Report {
    pub term: String,
    pub rows: usize,
    pub confirmed: usize,
    pub proposed: usize,
    pub no_time: usize,
    pub midnight: usize,
    pub warnings: Vec<String>,
}

impl Report {
    pub fn to_json(&self) -> Value {
        json!({
            "term": self.term, "rows": self.rows, "confirmed": self.confirmed, "proposed": self.proposed,
            "dropped": { "no_time": self.no_time, "midnight": self.midnight },
            "warnings": self.warnings,
        })
    }
}

/// `commitments --registrar <file> --school <key>` (spec §3). Parses first: an unknown school, a
/// file that is not JSON and a parse with no usable row are `Err` (exit 2) before anything is
/// written. Then the term's calendar is merged under D5 ([`crate::commitments::refresh_series`]),
/// and every kept row that names a vault course and is a current proposal is confirmed at `hard`
/// through phase 2's `confirm`, under `ctx` (the student: the vault's human token via
/// `dashboard`; Plan rulings R3-b, R3-c). No network.
pub fn run(
    vault: &Path,
    text: &str,
    school_key: &str,
    today: Date,
    ctx: &crate::write::WriteContext,
    journal: &mut crate::journal::Journal,
) -> Result<Report, String> {
    use crate::commitments as cm;
    if school(school_key).is_none() {
        return Err(format!("--school {school_key:?} is not a school Knowlu reads"));
    }
    let json: Value = serde_json::from_str(text).map_err(|e| format!("--registrar is not JSON ({e})"))?;
    let parsed = parse_banner(&json, school_key)?;
    let calendar = calendar_key(school_key, &parsed.term);
    let (_, mut warnings) = cm::refresh_series(vault, &[(calendar, parsed.series.clone())], today);
    warnings.extend(parsed.warnings.iter().cloned());
    let stored = cm::stored_proposals(vault, today);
    let current: BTreeSet<&str> = stored.proposals.iter().map(|p| p.source_uid.as_str()).collect();
    let (mut mine, mut proposed) = (Vec::new(), 0);
    for s in &parsed.series {
        match cm::registrar_course(s, &stored.codes) {
            Some(_) if current.contains(s.source_uid.as_str()) => mine.push((s.source_uid.clone(), "hard".to_string())),
            Some(_) => {}
            // Final review m5: "to check" is a row the student can still answer, not one declined.
            None if current.contains(s.source_uid.as_str()) => proposed += 1,
            None => {}
        }
    }
    let input = cm::ConfirmInput { mine, not_mine: Vec::new(), window: None };
    let done = cm::confirm(vault, &input, today, ctx, journal)?;
    warnings.extend(done.warnings);
    Ok(Report {
        term: parsed.term,
        rows: parsed.series.len(),
        confirmed: done.created,
        proposed,
        no_time: parsed.no_time,
        midnight: parsed.midnight,
        warnings,
    })
}

/// The *Schedule* view's registrar line (Plan ruling R3-d): the school and the terms the series
/// file holds, the term [`term_for`] names today, and whether that one is missing. `null` when the
/// file holds no registrar calendar.
pub fn status(file: &SeriesFile, today: Date) -> Value {
    let mut school_key: Option<String> = None;
    let mut held: BTreeSet<String> = BTreeSet::new();
    for calendar in file.calendars.keys() {
        let Some((s, term)) = calendar.strip_prefix(CALENDAR_PREFIX).and_then(|r| r.split_once(':')) else { continue };
        school_key.get_or_insert_with(|| s.to_string());
        held.insert(term.to_string());
    }
    let Some(school_key) = school_key else { return Value::Null };
    let current = term_for(&school_key, today);
    let refresh = current.as_ref().is_some_and(|t| !held.contains(t));
    json!({ "school": school_key, "held": held, "current": current, "refresh": refresh })
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
        v["data"]["registrations"][0]["scheduleDescription"] = json!("Collaborative Seminar");
        assert_eq!(by_crn(&parse_banner(&v, "ua").unwrap(), "40001").title, "CS 100", "a word inside a word is not a lab");
    }

    #[test]
    fn where_is_cut_to_80_and_a_row_without_a_building_has_none() {
        let long = "B".repeat(90);
        let row = |building: Value, room: Value| json!({ "data": { "registrations": [{ "term": "202640", "courseReferenceNumber": "1", "subject": "ZZT",
            "courseNumber": "101", "scheduleDescription": "Lecture", "meetingTimes": [{ "meetingType": "CLAS", "meetingScheduleType": "LEC",
            "beginTime": "0800", "endTime": "0850", "monday": true, "startDate": "08/19/2026", "endDate": "12/04/2026",
            "buildingDescription": building, "room": room }] }] } });
        let p = parse_banner(&row(json!(long), json!("1")), "ua").unwrap();
        assert_eq!(p.series[0].where_.as_deref().map(|w| w.chars().count()), Some(80));
        let p = parse_banner(&row(Value::Null, Value::Null), "ua").unwrap();
        assert_eq!(p.series[0].where_, None);
    }

    #[test]
    fn an_empty_parse_is_a_failure_never_an_empty_semester() {
        assert!(parse_banner(&json!([]), "ua").is_err());
        assert!(parse_banner(&json!({ "data": [] }), "ua").is_err());
        assert!(parse_banner(&json!({ "data": { "registrations": [] } }), "ua").is_err());
        assert!(parse_banner(&json!({ "nothing": 1 }), "ua").is_err());
        let online_only = json!({ "data": { "registrations": [{ "term": "202640", "courseReferenceNumber": "9", "subject": "ZZT", "courseNumber": "150",
            "meetingTimes": [{ "meetingType": "CLAS", "beginTime": null, "endTime": null }] }] } });
        let err = parse_banner(&online_only, "ua").unwrap_err();
        assert!(err.contains("1 without times"), "{err}");
    }

    #[test]
    fn another_term_or_a_repeated_crn_is_skipped_with_a_warning() {
        let mut v = fixture();
        let rows = v["data"]["registrations"].as_array_mut().unwrap();
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

    /// Final review I1 (a): the Google "CS 100" class ends one minute later than Banner's, so the
    /// signatures differ; a Google series with its instances, `event_type` and `last_seen`.
    fn google_minute_off(today: Date, crn: &str) -> Series {
        let mut google = fall().into_iter().find(|s| s.source_uid.ends_with(crn)).unwrap();
        google.meets[0].end = google.meets[0].end.checked_add(jiff::SignedDuration::from_mins(1)).unwrap();
        google.source_uid = format!("gcal-series:invented{crn}");
        google.calendar = "google:invented".into();
        google.event_type = Some("default".into());
        google.last_seen = Some(today);
        let (start, end) = (google.meets[0].start, google.meets[0].end);
        google.instances = [date(2026, 8, 31), date(2026, 9, 2), date(2026, 9, 4), date(2026, 9, 7), date(2026, 9, 9)]
            .into_iter().map(|day| Instance { date: day, start: Some(start), end: Some(end) }).collect();
        google
    }

    fn proposed(file: &SeriesFile, set: &Commitments, today: Date) -> Vec<String> {
        let template = crate::weekcal::WeekCalendar::new(&serde_yaml_ng::Mapping::new(), Vec::new());
        cm::proposals(file, set, &codes(), &[], &template, &BTreeSet::new(), today, false)
            .into_iter().filter(|p| !p.is_window()).map(|p| p.source_uid).collect()
    }

    /// I1 (a): a Google class for a course the registrar holds, one minute off, is never proposed,
    /// whether the registrar holds it as a series in play or as a confirmed note.
    #[test]
    fn a_google_class_for_a_registrar_course_is_not_proposed() {
        let today = date(2026, 9, 10);
        let google = google_minute_off(today, "40001");
        let mut alone = SeriesFile::default();
        alone.calendars.insert("google:invented".into(), today);
        alone.series.push(google.clone());
        assert_eq!(proposed(&alone, &Commitments::default(), today), ["gcal-series:invented40001"], "no registrar data");

        let mut both = alone.clone();
        both.calendars.insert(CAL.into(), today);
        both.series.extend(fall().into_iter().filter(|s| s.source_uid.ends_with("40001")));
        both.series.sort_by(|a, b| (&a.source_uid, &a.calendar).cmp(&(&b.source_uid, &b.calendar)));
        assert_eq!(proposed(&both, &Commitments::default(), today), ["registrar:ua:202640-40001"]);

        let reg = fall().into_iter().find(|s| s.source_uid.ends_with("40001")).unwrap();
        let note = Commitment {
            id: "cmt_invented02".into(), path: PathBuf::from("commitments/cs-100.md"), kind: "class".into(),
            level: Level::Hard, title: reg.title.clone(), course: Some("cs-100".into()), meets: reg.meets.clone(),
            where_: reg.where_.clone(), from: reg.first, until: reg.until, source_uid: Some(reg.source_uid.clone()),
        };
        let set = Commitments { confirmed: vec![note], ..Commitments::default() };
        assert!(proposed(&alone, &set, today).is_empty(), "a confirmed registrar note holds the course");
        assert!(proposed(&both, &set, today).is_empty());
    }

    /// Re-review m8: a registrar note from a past term (its `until` before today) holds nothing, so
    /// Google's CS 100 class is proposed again.
    #[test]
    fn a_past_terms_registrar_note_leaves_a_google_class_proposed() {
        let today = date(2026, 9, 10);
        let mut file = SeriesFile::default();
        file.calendars.insert("google:invented".into(), today);
        file.series.push(google_minute_off(today, "40001"));
        let reg = fall().into_iter().find(|s| s.source_uid.ends_with("40001")).unwrap();
        let note = Commitment {
            id: "cmt_invented03".into(), path: PathBuf::from("commitments/cs-100.md"), kind: "class".into(),
            level: Level::Hard, title: reg.title.clone(), course: Some("cs-100".into()), meets: reg.meets.clone(),
            where_: reg.where_.clone(), from: Some(date(2026, 1, 12)), until: Some(date(2026, 5, 1)),
            source_uid: Some("registrar:ua:202610-40001".into()),
        };
        let set = Commitments { confirmed: vec![note], ..Commitments::default() };
        assert_eq!(proposed(&file, &set, today), ["gcal-series:invented40001"]);
    }

    /// I1, refined per kind: a registrar lecture holds the course's class proposals only. With
    /// Banner listing just the CS 100 lecture, Google's CS 100 lab is still proposed.
    #[test]
    fn a_registrar_lecture_leaves_a_google_lab_proposed() {
        let today = date(2026, 9, 10);
        let mut file = SeriesFile::default();
        file.calendars.insert("google:invented".into(), today);
        file.calendars.insert(CAL.into(), today);
        file.series.push(google_minute_off(today, "40002"));
        file.series.extend(fall().into_iter().filter(|s| s.source_uid.ends_with("40001")));
        file.series.sort_by(|a, b| (&a.source_uid, &a.calendar).cmp(&(&b.source_uid, &b.calendar)));
        let lab = file.series.iter().find(|s| s.calendar == "google:invented").unwrap();
        assert!(matches!(cm::classify(lab, &codes(), &[]), Some(Class::Kind { ref kind, .. }) if kind == "lab"), "the Google row is a lab");
        assert_eq!(proposed(&file, &Commitments::default(), today), ["gcal-series:invented40002", "registrar:ua:202640-40001"]);
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

#[cfg(test)]
mod run_tests {
    //! Phase 3, `run` and `status`, on a scratch vault whose only course is CS 100. Invented data.
    use super::*;
    use crate::journal::Journal;
    use crate::write::WriteContext;
    use jiff::civil::date;
    use std::path::PathBuf;

    fn vault(name: &str) -> PathBuf {
        let dir = std::env::temp_dir().join(format!("knowlu-p3run-{}-{name}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(dir.join("config")).unwrap();
        std::fs::create_dir_all(dir.join("courses")).unwrap();
        std::fs::write(dir.join("config").join("ingest.yaml"), "timezone: America/Chicago\n").unwrap();
        std::fs::write(dir.join("courses").join("cs-100.md"), "---\ntitle: \"CS 100 Invented Computing\"\ncode: \"CS 100\"\n---\n").unwrap();
        dir
    }

    /// The vault's human token, as the engine binary resolves it (ruling 11); never a literal.
    fn ctx(v: &Path) -> WriteContext {
        WriteContext::new(crate::journal::read_human_actor(v).unwrap(), "dashboard")
    }

    /// A new vault: `config/actor.yaml` says `student`.
    fn student_vault(name: &str) -> PathBuf {
        let dir = vault(name);
        std::fs::write(dir.join("config").join("actor.yaml"), "human_actor: student
").unwrap();
        dir
    }

    fn fetch(v: &Path, text: &str, day: Date) -> Result<Report, String> {
        run(v, text, "ua", day, &ctx(v), &mut Journal::new(v))
    }

    #[test]
    fn matched_rows_are_confirmed_by_the_student_and_the_rest_proposed() {
        let v = student_vault("r24");
        let r = fetch(&v, super::tests::FIXTURE, date(2026, 9, 1)).unwrap();
        assert_eq!((r.term.as_str(), r.rows, r.confirmed, r.proposed, r.no_time, r.midnight), ("202640", 4, 2, 2, 1, 1));
        let lab = std::fs::read_to_string(v.join("commitments").join("cs-100-lab.md")).unwrap();
        for line in ["kind: lab", "level: hard", "course: cs-100", "source_uid: registrar:ua:202640-40002", "until: 2026-12-04"] {
            assert!(lab.contains(line), "{line} missing: {lab}");
        }
        assert!(lab.contains("Found in your school's class schedule."), "{lab}");
        let journal: String = std::fs::read_dir(v.join("state").join("journal")).unwrap().flatten()
            .map(|e| std::fs::read_to_string(e.path()).unwrap()).collect();
        assert!(journal.contains("\"actor\": \"student\"") && !journal.contains("quinn") && journal.contains("\"via\": \"dashboard\""), "{journal}");
        assert!(!journal.contains("\"actor\": \"agent:commitments\""), "R3-c: the R24 writes are the student's (spec §3)");
        let again = fetch(&v, super::tests::FIXTURE, date(2026, 9, 2)).unwrap();
        assert_eq!((again.confirmed, again.warnings.iter().filter(|w| w.contains("not a current proposal")).count()), (0, 0));
        let _ = std::fs::remove_dir_all(&v);
    }

    #[test]
    fn a_bad_school_or_file_writes_nothing() {
        let v = vault("bad");
        assert!(run(&v, super::tests::FIXTURE, "zz", date(2026, 9, 1), &ctx(&v), &mut Journal::new(&v)).is_err());
        assert!(fetch(&v, "<html>sign in</html>", date(2026, 9, 1)).is_err());
        assert!(fetch(&v, "[]", date(2026, 9, 1)).is_err());
        assert!(!v.join("state").exists() && !v.join("commitments").exists());
        let _ = std::fs::remove_dir_all(&v);
    }

    #[test]
    fn the_overview_names_the_term_and_the_registrar_proposals() {
        let v = vault("overview");
        fetch(&v, super::tests::FIXTURE, date(2026, 9, 1)).unwrap();
        let o = crate::commitments::overview(&v, date(2026, 9, 1));
        assert_eq!(o.registrar, serde_json::json!({ "school": "ua", "held": ["202640"], "current": "202640", "refresh": false }));
        let keys: Vec<&str> = o.registrar_proposals.iter().map(|p| p["source_uid"].as_str().unwrap()).collect();
        assert_eq!(keys, ["registrar:ua:202640-40003", "registrar:ua:202640-40006"]);
        // What `your_week` hands the page (R5 reads `week.registrar` and `week.registrar_proposals`).
        let json = o.to_json();
        assert_eq!((&json["registrar"], json["registrar_proposals"].as_array().map(Vec::len)), (&o.registrar, Some(2)));
        assert_eq!(crate::commitments::overview(&v, date(2027, 1, 5)).registrar["refresh"], true);
        let fresh = vault("none");
        assert_eq!(crate::commitments::overview(&fresh, date(2026, 9, 1)).registrar, serde_json::Value::Null);
        let _ = std::fs::remove_dir_all(&v);
        let _ = std::fs::remove_dir_all(&fresh);
    }

    /// Final review m5: "n to check" counts only the course-less rows that are still current
    /// proposals; one the student declined is not asked about again.
    #[test]
    fn to_check_counts_only_current_proposals() {
        let v = vault("tocheck");
        fetch(&v, super::tests::FIXTURE, date(2026, 9, 1)).unwrap();
        let input = crate::commitments::ConfirmInput {
            mine: Vec::new(), not_mine: vec!["registrar:ua:202640-40006".into()], window: None,
        };
        crate::commitments::confirm(&v, &input, date(2026, 9, 1), &ctx(&v), &mut Journal::new(&v)).unwrap();
        let again = fetch(&v, super::tests::FIXTURE, date(2026, 9, 2)).unwrap();
        assert_eq!((again.confirmed, again.proposed), (0, 1));
        let _ = std::fs::remove_dir_all(&v);
    }

    /// Final review I3: the overview's confirmed rows carry their `source_uid`, so the confirm
    /// screen can list what the registrar confirmed without computing anything.
    #[test]
    fn the_overviews_confirmed_rows_carry_their_source_uid() {
        let v = vault("confirmedkeys");
        fetch(&v, super::tests::FIXTURE, date(2026, 9, 1)).unwrap();
        let o = crate::commitments::overview(&v, date(2026, 9, 1));
        let mut keys: Vec<&str> = o.commitments.iter().filter_map(|c| c["source_uid"].as_str()).collect();
        keys.sort();
        assert_eq!(keys, ["registrar:ua:202640-40001", "registrar:ua:202640-40002"]);
        let _ = std::fs::remove_dir_all(&v);
    }
}

#[cfg(test)]
mod parse_rule_tests {
    //! B4 step 2: the spec's D3 amendment, rules P1-P6 and P9, on the recorded shape (flat
    //! `meetingTimes[]`, `data.registrations`). The main fixture is the invented stand-in for the
    //! recorded shape; every edge case here is invented JSON built on it, never added to it.
    use super::*;
    use jiff::civil::{date, time};
    use serde_json::json;

    const ALL_DAYS: [&str; 7] = ["monday", "tuesday", "wednesday", "thursday", "friday", "saturday", "sunday"];

    fn fixture() -> Value {
        serde_json::from_str(super::tests::FIXTURE).unwrap()
    }

    /// One meeting of `kind` (`CLAS`, `EXAM`, ... ; null leaves the key out) on `days`.
    fn meeting(kind: Value, sched: &str, begin: Value, end: Value, days: &[&str]) -> Value {
        let mut m = json!({ "meetingScheduleType": sched, "beginTime": begin, "endTime": end,
            "startDate": "08/19/2026", "endDate": "12/04/2026", "buildingDescription": "Invented Hall", "room": "7" });
        if !kind.is_null() {
            m["meetingType"] = kind;
        }
        for d in ALL_DAYS {
            m[d] = json!(days.contains(&d));
        }
        m
    }

    fn clas(begin: &str, end: &str, days: &[&str]) -> Value {
        meeting(json!("CLAS"), "LEC", json!(begin), json!(end), days)
    }

    /// A registered row of `ZZT <number>` with these meetings.
    fn row(crn: &str, number: &str, meetings: Vec<Value>) -> Value {
        json!({ "term": "202640", "courseReferenceNumber": crn, "subject": "ZZT", "courseNumber": number,
            "scheduleDescription": "Lecture", "statusIndicator": "IR", "statusDescription": "Invented Registered",
            "meetingTimes": meetings })
    }

    /// The fixture's rows plus these.
    fn with(extra: Vec<Value>) -> Value {
        let mut v = fixture();
        v["data"]["registrations"].as_array_mut().unwrap().extend(extra);
        v
    }

    fn only(rows: Vec<Value>) -> Value {
        json!({ "data": { "registrations": rows } })
    }

    fn parse(v: &Value) -> Parsed {
        parse_banner(v, "ua").unwrap()
    }

    fn series<'a>(p: &'a Parsed, crn: &str) -> Option<&'a Series> {
        let key = format!("registrar:ua:202640-{crn}");
        p.series.iter().find(|s| s.source_uid == key)
    }

    // ---- P1: rows ---------------------------------------------------------------------------

    #[test]
    fn p1_an_answer_without_data_registrations_is_not_a_list_of_rows() {
        for v in [json!({ "data": {} }), json!({ "data": [] }), json!({ "success": true }), json!([]), json!({ "data": { "registrations": "x" } })] {
            let err = parse_banner(&v, "ua").unwrap_err();
            assert!(err.contains("not a list of registration rows"), "{v}: {err}");
        }
    }

    #[test]
    fn p1_a_present_but_empty_registrations_is_the_empty_parse_not_the_missing_list() {
        let err = parse_banner(&only(Vec::new()), "ua").unwrap_err();
        assert!(!err.contains("not a list of registration rows"), "an empty list is a list: {err}");
        assert!(err.contains("no class with meeting times"), "{err}");
    }

    // ---- P2: meetings -----------------------------------------------------------------------

    #[test]
    fn p2_the_meetings_faculty_wrapper_is_not_read() {
        let mut old = row("60001", "201", Vec::new());
        old.as_object_mut().unwrap().remove("meetingTimes");
        old["meetingsFaculty"] = json!([{ "meetingTime": clas("0800", "0850", &["monday"]) }]);
        let p = parse(&with(vec![old]));
        assert!(series(&p, "60001").is_none(), "{:?}", p.series);
        assert_eq!(p.no_time, 2, "the fixture's online row and this one: no CLAS meeting in meetingTimes[]");
        assert_eq!(p.seen["60001"], Reading::Unread);
    }

    // ---- P3: only CLAS becomes a series ------------------------------------------------------

    #[test]
    fn p3_a_final_adds_no_weekday_moves_no_date_and_is_counted() {
        let p = parse(&fixture());
        let s = series(&p, "40001").unwrap();
        assert_eq!(s.meets, vec![Meet { days: vec!["mon", "wed", "fri"], start: time(12, 0, 0, 0), end: time(12, 50, 0, 0) }], "the Tuesday final is not a weekly class");
        assert_eq!((s.first, s.until), (Some(date(2026, 8, 19)), Some(date(2026, 12, 4))), "the 12/08 final stretches nothing");
        assert_eq!(s.rule.until, Some(date(2026, 12, 4)));
        assert_eq!(s.where_.as_deref(), Some("Invented Hall 101"), "the final's room 140 is not the place");
        assert_eq!(p.exam, 1);
        assert!(!p.warnings.iter().any(|w| w.to_lowercase().contains("exam") || w.contains("CS 100")), "a final is expected, never warned: {:?}", p.warnings);
    }

    #[test]
    fn p3_a_missing_or_unknown_meeting_type_is_skipped_counted_warned_and_never_read_as_clas() {
        let extra = row("60002", "202", vec![
            clas("0800", "0850", &["monday"]),
            meeting(json!("LABX"), "LEC", json!("1300"), json!("1350"), &["tuesday"]),
            meeting(Value::Null, "LEC", json!("1300"), json!("1350"), &["friday"]),
        ]);
        let p = parse(&with(vec![extra]));
        let s = series(&p, "60002").unwrap();
        assert_eq!(s.meets.len(), 1, "{:?}", s.meets);
        assert_eq!(s.meets[0].days, vec!["mon"]);
        assert_eq!(p.meeting_type, 2);
        assert!(p.warnings.iter().any(|w| w.contains("ZZT 202")), "{:?}", p.warnings);
        assert_eq!(p.exam, 1, "the unknown ones are not finals");
    }

    #[test]
    fn p3_a_row_with_only_unknown_type_meetings_forms_no_series() {
        let extra = row("60003", "203", vec![meeting(json!("OTHR"), "LEC", json!("0800"), json!("0850"), &["monday"])]);
        let p = parse(&with(vec![extra]));
        assert!(series(&p, "60003").is_none());
        assert_eq!(p.meeting_type, 1);
        assert_eq!(p.seen["60003"], Reading::Unread);
    }

    #[test]
    fn p3_a_row_whose_meetings_are_all_finals_forms_no_series_and_counts_in_no_time() {
        let extra = row("60004", "204", vec![meeting(json!("EXAM"), "LEC", json!("0800"), json!("1030"), &["tuesday"])]);
        let p = parse(&with(vec![extra]));
        assert!(series(&p, "60004").is_none());
        assert_eq!((p.exam, p.no_time), (2, 2), "the fixture's final and this one; the fixture's online row and this one");
        assert!(p.warnings.iter().any(|w| w.contains("ZZT 204")), "P4: a row left with no usable CLAS meeting is named: {:?}", p.warnings);
        assert_eq!(p.seen["60004"], Reading::Unread);
    }

    // ---- P4: no time, no day ----------------------------------------------------------------

    #[test]
    fn p4_a_clas_meeting_with_no_time_or_no_day_is_skipped_never_filled_in() {
        let mut absent = clas("0800", "0850", &["monday"]);
        absent.as_object_mut().unwrap().remove("beginTime");
        let cases = [
            ("absent begin", absent),
            ("null end", meeting(json!("CLAS"), "LEC", json!("0800"), Value::Null, &["monday"])),
            ("empty begin", clas("", "0850", &["monday"])),
            ("blank end", clas("0800", "  ", &["monday"])),
            ("no day true", clas("0800", "0850", &[])),
        ];
        for (name, m) in cases {
            let p = parse(&with(vec![row("60005", "205", vec![m])]));
            assert!(series(&p, "60005").is_none(), "{name}");
            assert_eq!(p.no_time, 2, "{name}: the fixture's online row and this one");
            assert!(p.warnings.iter().any(|w| w.contains("ZZT 205")), "{name}: {:?}", p.warnings);
            assert!(!p.warnings.iter().any(|w| w.contains("across midnight")), "{name}");
        }
    }

    #[test]
    fn p4_the_fixtures_online_row_is_named_by_subject_and_number() {
        let p = parse(&fixture());
        assert_eq!(p.no_time, 1);
        assert!(p.warnings.iter().any(|w| w.contains("HIST 105")), "{:?}", p.warnings);
        assert!(series(&p, "40004").is_none());
    }

    #[test]
    fn p4_a_timeless_meeting_beside_a_timed_one_is_not_filled_in_from_it() {
        let extra = row("60006", "206", vec![clas("0800", "0850", &["monday"]), clas("", "", &["friday"])]);
        let p = parse(&with(vec![extra]));
        let s = series(&p, "60006").unwrap();
        assert_eq!(s.meets, vec![Meet { days: vec!["mon"], start: time(8, 0, 0, 0), end: time(8, 50, 0, 0) }]);
        assert_eq!(p.no_time, 1, "the row formed a series, so only the fixture's online row is a dropped row");
    }

    // ---- P5: lab or class -------------------------------------------------------------------

    fn title_of(sched_description: &str, meeting_type: &str, legacy: Option<(&str, &str)>) -> String {
        let mut r = row("60007", "207", vec![meeting(json!("CLAS"), meeting_type, json!("0800"), json!("0850"), &["monday"])]);
        r["scheduleDescription"] = json!(sched_description);
        if let Some((key, value)) = legacy {
            r[key] = json!(value);
        }
        series(&parse(&with(vec![r])), "60007").unwrap().title.clone()
    }

    #[test]
    fn p5_a_lab_meeting_schedule_type_or_a_whole_word_schedule_description_is_a_lab() {
        assert_eq!(title_of("Lecture", "LAB", None), "ZZT 207 Lab");
        assert_eq!(title_of("Laboratory", "LEC", None), "ZZT 207 Lab");
        assert_eq!(title_of("Science lab", "LEC", None), "ZZT 207 Lab");
        assert_eq!(title_of("Lab", "LEC", None), "ZZT 207 Lab");
        let p = parse(&fixture());
        assert_eq!(series(&p, "40002").unwrap().title, "CS 100 Lab", "the fixture's lab row, by scheduleDescription");
    }

    #[test]
    fn p5_lec_llb_rct_and_sem_are_classes_and_a_word_inside_a_word_is_not_a_lab() {
        for t in ["LEC", "LLB", "RCT", "SEM"] {
            assert_eq!(title_of("Lecture", t, None), "ZZT 207", "{t}");
        }
        assert_eq!(title_of("Collaborative Seminar", "LEC", None), "ZZT 207");
        assert_eq!(title_of("Labor Studies", "LEC", None), "ZZT 207");
    }

    #[test]
    fn p5_the_keys_this_call_does_not_carry_are_not_read() {
        assert_eq!(title_of("Lecture", "LEC", Some(("scheduleTypeDescription", "Laboratory"))), "ZZT 207");
        assert_eq!(title_of("Lecture", "LEC", Some(("scheduleType", "LAB"))), "ZZT 207");
    }

    // ---- P6: registration status ------------------------------------------------------------

    fn with_status(v: &mut Value, crn: &str, code: Value, description: &str) {
        let r = v["data"]["registrations"].as_array_mut().unwrap().iter_mut().find(|r| r["courseReferenceNumber"] == crn).unwrap();
        r["statusIndicator"] = code;
        r["statusDescription"] = json!(description);
    }

    #[test]
    fn p6_with_an_empty_registered_set_every_row_counts() {
        let mut v = fixture();
        with_status(&mut v, "40003", json!("XX"), "Invented Withdrawn");
        for p in [parse_banner(&v, "ua").unwrap(), parse_banner_with(&v, "ua", &[]).unwrap()] {
            assert!(series(&p, "40003").is_some());
            assert_eq!(p.status, 0);
            assert_eq!(p.seen["40003"], Reading::Whole);
        }
    }

    #[test]
    fn p6_a_row_outside_a_non_empty_set_forms_no_series_is_counted_and_named_with_its_description() {
        let mut v = fixture();
        with_status(&mut v, "40003", json!("XX"), "Invented Withdrawn");
        let p = parse_banner_with(&v, "ua", &["IR"]).unwrap();
        assert!(series(&p, "40003").is_none());
        assert_eq!(p.status, 1);
        assert!(p.warnings.iter().any(|w| w.contains("ENGL 101") && w.contains("Invented Withdrawn")), "{:?}", p.warnings);
        assert_eq!(p.seen["40003"], Reading::Status);
        for crn in ["40001", "40002", "40006"] {
            assert!(series(&p, crn).is_some(), "{crn}: rows inside the set are untouched");
            assert_eq!(p.seen[crn], Reading::Whole);
        }
    }

    #[test]
    fn p6_a_row_with_no_status_indicator_is_outside_a_non_empty_set() {
        let mut v = fixture();
        v["data"]["registrations"][2].as_object_mut().unwrap().remove("statusIndicator");
        let p = parse_banner_with(&v, "ua", &["IR"]).unwrap();
        assert!(series(&p, "40003").is_none());
        assert_eq!((p.status, p.seen["40003"]), (1, Reading::Status));
    }

    #[test]
    fn p6_a_set_that_excludes_every_row_is_the_empty_parse() {
        assert!(parse_banner_with(&fixture(), "ua", &["ZZ"]).is_err());
    }

    // ---- P9: seen ---------------------------------------------------------------------------

    #[test]
    fn p9_a_row_whose_only_clas_meeting_has_null_times_is_unread_and_forms_no_series() {
        let p = parse(&fixture());
        assert_eq!(p.seen["40004"], Reading::Unread);
        assert!(series(&p, "40004").is_none());
    }

    #[test]
    fn p9_a_row_with_only_unknown_type_meetings_is_unread() {
        let extra = row("60008", "208", vec![meeting(json!("WEIRD"), "LEC", json!("0800"), json!("0850"), &["monday"])]);
        assert_eq!(parse(&with(vec![extra])).seen["60008"], Reading::Unread);
    }

    #[test]
    fn p9_a_usable_and_a_timeless_clas_meeting_is_partial_with_a_series_from_the_usable_one() {
        let extra = row("60009", "209", vec![clas("0800", "0850", &["monday"]), clas("", "", &["friday"])]);
        let p = parse(&with(vec![extra]));
        assert_eq!(p.seen["60009"], Reading::Partial);
        assert_eq!(series(&p, "60009").unwrap().meets[0].days, vec!["mon"]);
    }

    #[test]
    fn p9_a_usable_and_an_unknown_type_meeting_is_partial() {
        let extra = row("60010", "210", vec![clas("0800", "0850", &["monday"]), meeting(json!("WEIRD"), "LEC", json!("1300"), json!("1350"), &["friday"])]);
        assert_eq!(parse(&with(vec![extra])).seen["60010"], Reading::Partial);
    }

    #[test]
    fn p9_an_exam_meeting_beside_a_clas_one_leaves_the_row_whole() {
        let p = parse(&fixture());
        assert_eq!(p.exam, 1, "the fixture's CS 100 carries its final");
        assert_eq!(p.seen["40001"], Reading::Whole);
    }

    #[test]
    fn p9_a_row_under_a_non_registered_status_is_status_not_unread() {
        let mut v = fixture();
        with_status(&mut v, "40001", json!("XX"), "Invented Dropped");
        let p = parse_banner_with(&v, "ua", &["IR"]).unwrap();
        assert_eq!(p.seen["40001"], Reading::Status);
    }

    #[test]
    fn p9_a_meeting_across_midnight_leaves_no_series_and_the_row_unread() {
        assert_eq!(parse(&fixture()).seen["40005"], Reading::Unread);
    }

    #[test]
    fn p9_every_crn_of_the_answer_is_in_seen_exactly_once() {
        let p = parse(&fixture());
        let mut keys: Vec<&str> = p.seen.keys().map(String::as_str).collect();
        keys.sort_unstable();
        assert_eq!(keys, ["40001", "40002", "40003", "40004", "40005", "40006"]);
        let readings: Vec<Reading> = ["40001", "40002", "40003", "40004", "40005", "40006"].iter().map(|c| p.seen[*c]).collect();
        assert_eq!(readings, [Reading::Whole, Reading::Whole, Reading::Whole, Reading::Unread, Reading::Unread, Reading::Whole]);
        let mut v = fixture();
        let again = v["data"]["registrations"][1].clone();
        v["data"]["registrations"].as_array_mut().unwrap().push(again);
        let twice = parse(&v);
        assert_eq!(twice.seen.len(), 6, "a CRN listed twice is one entry");
        assert_eq!(twice.seen["40002"], Reading::Whole, "the first reading stands");
    }

    #[test]
    fn p9_a_row_of_the_wrong_term_is_still_seen() {
        let mut other = row("60011", "211", vec![clas("0800", "0850", &["monday"])]);
        other["term"] = json!("202710");
        let p = parse(&with(vec![other]));
        assert!(p.seen.contains_key("60011"), "{:?}", p.seen);
        assert!(series(&p, "60011").is_none());
    }
}
