//! Close-out task B2, the B3c tests: the registrar block in `state/calendar-series.json`, its bytes,
//! its survival across `rank` and restarts, and the report's keys. Spec A2 items 13 and 14 and item
//! 20 (round 1 and round 2). The entry shapes are those `apply_support`'s header pins. Invented data.
use super::apply_support::*;
use jiff::civil::date;
use serde_json::{json, Value};

/// The answer without the two rows no note holds (an online course and a midnight class), so the
/// block's lists hold only what a test means them to.
fn trimmed() -> Value {
    let mut answer = fx();
    drop_crn(&mut answer, "40004");
    drop_crn(&mut answer, "40005");
    answer
}

/// One fetch that applies, ends, keeps, reads one row under another status and one with no times.
fn busy(name: &str) -> Lab {
    let lab = Lab::new(name, "2026-08-31");
    lab.add_courses();
    lab.fetch_registered(D1, &trimmed(), &REGISTERED);
    lab.hand_set(LECTURE, "until", "2026-11-20");
    let mut later = trimmed();
    retime(&mut later, "40001", "1300", "1350");
    meeting(&mut later, "40001")["room"] = json!("102");
    meeting(&mut later, "40001")["endDate"] = json!("12/11/2026");
    drop_crn(&mut later, "40002");
    set_status(&mut later, "40006", "WD", "Invented Withdrawn");
    for m in row(&mut later, "40003")["meetingTimes"].as_array_mut().unwrap() {
        m["beginTime"] = Value::Null;
        m["endTime"] = Value::Null;
    }
    lab.fetch_registered(D10, &later, &REGISTERED);
    lab
}

const BLOCK: &str = r#""registrar": {"registrar:ua:202640": {"applied": [{"day": "2026-09-10", "field": "meets", "from": [{"days": ["mon", "wed", "fri"], "end": "12:50", "start": "12:00"}], "note": "commitments/cs-100.md", "to": [{"days": ["mon", "wed", "fri"], "end": "13:50", "start": "13:00"}]}, {"day": "2026-09-10", "field": "where", "from": "Invented Hall 101", "note": "commitments/cs-100.md", "to": "Invented Hall 102"}], "ended": [{"day": "2026-09-10", "kept_yours": false, "meets": [{"days": ["thu"], "end": "15:50", "start": "14:00"}], "note": "commitments/cs-100-lab.md", "until": "2026-09-09", "where": "Invented Hall B12"}], "first_fetch": "2026-09-01", "kept": [{"field": "until", "note": "commitments/cs-100.md", "registrar": "2026-12-11", "student": "2026-11-20"}], "last_fetch": "2026-09-10", "status": [{"key": "registrar:ua:202640-40006", "note": "commitments/art-110.md", "status": "Invented Withdrawn"}], "unread": [{"key": "registrar:ua:202640-40003", "note": "commitments/engl-101.md", "reason": "myBama lists this course with no class time; nothing changed"}]}}"#;

/// B3c (A2 item 13, "Its bytes"). The block goes through `ledger::dumps_value` with `file_bytes`:
/// the top level reads `calendars`, `ended`, `registrar`, `series`; every key of a term is present;
/// entries are in the spec's fixed order; the whole block is these bytes.
#[test]
fn the_blocks_bytes_are_dumps_value_in_sorted_key_order() {
    let lab = busy("block-bytes");
    let text = lab.series_text();
    assert!(text.contains(BLOCK), "the block differs:\n{text}");
    let at = |key: &str| text.find(&format!("\"{key}\": ")).unwrap_or_else(|| panic!("no `{key}` in {text}"));
    assert!(at("calendars") < at("ended") && at("ended") < at("registrar") && at("registrar") < at("series"), "{text}");
    assert!(text.ends_with("}\n"), "one trailing newline");
}

/// B3c (the order of list entries, A2 item 13). `applied` and `ended` entries are in (fetch day, note
/// path, field) order, `kept`, `status` and `unread` in (note path, field) order: a lab dropped on
/// day 10 comes before a later drop of the art course whose path sorts first, and two notes under
/// another status are in path order, not CRN order.
#[test]
fn entries_are_in_the_fixed_order() {
    let lab = Lab::new("block-order", "2026-08-31");
    lab.add_courses();
    lab.fetch_registered(D1, &trimmed(), &REGISTERED);
    let mut tenth = trimmed();
    drop_crn(&mut tenth, "40002");
    lab.fetch_registered(D10, &tenth, &REGISTERED);
    let mut twentieth = tenth.clone();
    drop_crn(&mut twentieth, "40006");
    set_status(&mut twentieth, "40003", "WD", "Invented Withdrawn");
    lab.fetch_registered(D20, &twentieth, &REGISTERED);
    let notes: Vec<Value> = lab.ended_entries().iter().map(|e| e["note"].clone()).collect();
    assert_eq!(notes, [json!(LAB), json!("commitments/art-110.md")], "day first, then path");
    let mut both = twentieth.clone();
    set_status(&mut both, "40001", "WD", "Invented Withdrawn");
    lab.fetch_registered(D30, &both, &REGISTERED);
    let status: Vec<Value> = lab.term()["status"].as_array().unwrap().iter().map(|e| e["note"].clone()).collect();
    assert_eq!(status, [json!(LECTURE), json!("commitments/engl-101.md")], "path order");
}

/// B3c (A2 item 13, "Every writer keeps it"). A `rank` that rewrites the file leaves the block
/// byte-identical while the term is in play and drops that term's entry once the calendar ages out;
/// a file without the block reads as an empty one.
#[test]
fn a_rank_keeps_the_block_and_drops_an_aged_out_terms_entry() {
    let lab = busy("block-rank");
    let block = |lab: &Lab| crate::ledger::dumps_value(&lab.block());
    let before = block(&lab);
    assert!(before.contains("registrar:ua:202640"), "no block was written: {before}");
    let fresh = || vec![("cloud:google", google_entry(Vec::new()))];
    lab.rank_with(D11, fresh());
    assert_eq!(block(&lab), before);
    lab.rank_with(date(2027, 1, 10), fresh());
    assert!(lab.series_file().get("registrar").is_none(), "the aged-out term kept its block: {}", lab.series_text());
    lab.drop_block();
    let (file, warnings) = crate::commitments::read_series_file(&lab.v);
    assert!(warnings.is_empty() && file.calendars.len() <= 2, "{warnings:?}");
}

/// B3c (A2 item 13, "Its bytes": no `registrar` key in a vault that never fetched). A vault whose
/// series file comes from a Google read alone writes exactly the bytes `main` writes today.
#[test]
fn a_vault_that_never_fetched_writes_the_series_file_main_writes_today() {
    let lab = Lab::new("block-none", "2026-08-31");
    lab.rank_with(D7, vec![("cloud:google", google_entry(vec![google_item("chess01", "Chess Club", &[3], "18:00", "19:00", 8)]))]);
    let text = lab.series_text();
    assert!(!text.is_empty() && lab.series_file().get("registrar").is_none(), "{text}");
    // Captured on this branch before any apply-and-list change, which is what `main` writes.
    const TODAY: &str = r#"{"calendars": {"google:0a1b2c3d4e5f6071": "2026-09-07"}, "ended": {}, "series": [{"calendar": "google:0a1b2c3d4e5f6071", "event_type": "default", "first": "2026-09-10", "has_master": true, "instances": [["2026-09-10", "18:00", "19:00"], ["2026-09-17", "18:00", "19:00"], ["2026-09-24", "18:00", "19:00"], ["2026-10-01", "18:00", "19:00"]], "last_seen": "2026-09-07", "meets": [{"days": ["thu"], "end": "19:00", "start": "18:00"}], "rdate": false, "rule": {"count": null, "freq": "WEEKLY", "interval": 1, "until": null}, "source_uid": "gcal-series:chess01", "title": "Chess Club", "unsupported": false, "until": null, "where": null}]}"#;
    assert_eq!(text, format!("{TODAY}\n"));
    lab.rank_with(D8, vec![("cloud:google", google_entry(vec![google_item("chess01", "Chess Club", &[3], "18:00", "19:00", 8)]))]);
    assert!(lab.series_file().get("registrar").is_none());
}

/// B3c (A2 item 14, idempotence, the `applied` part; item 20, round 1). A second identical fetch,
/// on the same day and again on a later day, adds no `applied` or `ended` entry, closes none, and
/// changes only `last_fetch`.
#[test]
fn an_identical_refetch_changes_only_last_fetch() {
    let lab = busy("block-idem");
    let same_day = lab.term();
    let mut again = trimmed();
    retime(&mut again, "40001", "1300", "1350");
    meeting(&mut again, "40001")["room"] = json!("102");
    meeting(&mut again, "40001")["endDate"] = json!("12/11/2026");
    drop_crn(&mut again, "40002");
    set_status(&mut again, "40006", "WD", "Invented Withdrawn");
    for m in row(&mut again, "40003")["meetingTimes"].as_array_mut().unwrap() {
        m["beginTime"] = Value::Null;
        m["endTime"] = Value::Null;
    }
    lab.fetch_registered(D10, &again, &REGISTERED);
    assert_eq!(lab.term(), same_day, "the same day changed the block");
    lab.fetch_registered(D20, &again, &REGISTERED);
    let mut later = lab.term();
    assert_eq!(later["last_fetch"], "2026-09-20");
    later["last_fetch"] = same_day["last_fetch"].clone();
    assert_eq!(later, same_day, "a later fetch changed more than last_fetch");
}

/// B3c (A2 item 13, "After a restart"; item 20, round 1, the `applied`, `status` and `unread`
/// part). The overview a new process reads lists what the fetch reported, from the block, and each
/// `applied` entry has its `agent:knowlu.registrar` journal record. The existing `registrar` object
/// of the overview is untouched.
#[test]
fn the_list_survives_a_restart_its_applied_status_and_unread_part() {
    let lab = Lab::new("block-restart", "2026-08-31");
    lab.add_courses();
    lab.fetch_registered(D1, &trimmed(), &REGISTERED);
    let mut later = trimmed();
    retime(&mut later, "40001", "1300", "1350");
    set_status(&mut later, "40006", "WD", "Invented Withdrawn");
    for m in row(&mut later, "40003")["meetingTimes"].as_array_mut().unwrap() {
        m["beginTime"] = Value::Null;
        m["endTime"] = Value::Null;
    }
    let report = lab.fetch_registered(D10, &later, &REGISTERED);
    let overview = crate::commitments::overview(&lab.v, D10).to_json();
    for key in ["applied", "status", "unread"] {
        assert!(!list(&report, key).is_empty(), "{key}: {report}");
        assert_eq!(overview["registrar_changes"][key], json!(list(&report, key)), "{key}: {overview}");
    }
    let records = lab.sets(LECTURE, "meets");
    assert_eq!((records.len(), records[0].0.as_str()), (1, "agent:knowlu.registrar"), "an applied entry without its record");
    assert_eq!(overview["registrar"]["held"], json!(["202640"]), "the existing object is unchanged");
}

/// B3c (A2 item 13). The command's JSON gains `applied`, `ended`, `kept`, `status`, `unread` (lists)
/// and `dropped` gains `exam`, `meeting_type` and `status` beside `no_time` and `midnight`.
#[test]
fn the_report_carries_the_new_keys() {
    let lab = Lab::new("block-report", "2026-08-31");
    let report = lab.fetch(D1, &fx());
    for key in ["applied", "ended", "kept", "status", "unread"] {
        assert!(report[key].is_array(), "{key}: {report}");
    }
    assert_eq!(report["dropped"], json!({ "exam": 1, "meeting_type": 0, "midnight": 1, "no_time": 1, "status": 0 }), "{report}");
}
