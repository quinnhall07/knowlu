//! Replan C2c (`docs/plans/2026-11-04-onboarding-replan-draft.md` §6.6, "When a later registrar read
//! meets a class-search note"; §8 C2c): a registrar read adopts a confirmed class-search note by
//! re-keying it, `classsearch:<school>:<term>-<crn>` -> `registrar:<school>:<term>-<crn>`.
//!
//! What a re-key may change, pinned here: one `source_uid` write through `write_literals` (journal
//! first, single-line surgery, no other byte of the note), as `agent:knowlu.registrar` via
//! `dashboard`. What it may not: any field the student set (judge once), any other term's or
//! school's note, anything on a second read of the same answer. Every title, key and date is
//! invented, and every vault write in a setup goes through `write`.
use super::apply_support::*;
use crate::commitments::{self as cm, Instance, Meet, Rule, Series};
use jiff::civil::Time;
use serde_json::{json, Value};

const CS_LECTURE: &str = "classsearch:ua:202640-40001";
const REG_LECTURE: &str = "registrar:ua:202640-40001";
const CS_LAB: &str = "classsearch:ua:202640-40002";
const REG_LAB: &str = "registrar:ua:202640-40002";
const CS_SPRING: &str = "classsearch:ua:202710-50001";
const CS_OTHER_SCHOOL: &str = "classsearch:xx:202640-40001";
const CS_ABSENT: &str = "classsearch:ua:202640-49999";
const CS_CAL: &str = "classsearch:ua:202640";
const SPRING_CAL: &str = "classsearch:ua:202710";
const ACTOR: &str = "agent:knowlu.registrar";
const ABSENT: &str = "commitments/cs-100-absent.md";
const SPRING_NOTE: &str = "commitments/cs-100-spring.md";
const XX_NOTE: &str = "commitments/cs-100-xx.md";

/// A confirmed note exactly as the class-search confirm would mint it: the lecture's own times.
fn front(id: &str, key: &str, kind: &str, meets: &str) -> String {
    format!(
        "id: {id}\ntype: commitment\nkind: {kind}\nlevel: hard\ntitle: \"CS 100 Invented\"\ncourse: cs-100\n\
         meets: [{meets}]\nuntil: 2026-12-04\nwhere: \"Invented Hall 101\"\nsource_uid: \"{key}\"\nstatus: confirmed\n"
    )
}

const MWF_NOON: &str = "{days: [mon, wed, fri], start: \"12:00\", end: \"12:50\"}";
const MWF_ELEVEN: &str = "{days: [mon, wed, fri], start: \"11:00\", end: \"11:50\"}";

fn lecture(id: &str, key: &str) -> String {
    front(id, key, "class", MWF_NOON)
}

/// A lab note: the answer's lab meets Tue/Thu in the fixture, so nothing here depends on its times.
fn lab(id: &str, key: &str) -> String {
    front(id, key, "lab", "{days: [tue, thu], start: \"14:00\", end: \"15:50\"}")
}

fn lab_with(name: &str, notes: &[(&str, String)]) -> Lab {
    let lab = Lab::new(name, "2026-08-31");
    for (file, text) in notes {
        lab.plant_note(file, text);
    }
    lab
}

/// The `set` records of `source_uid`, in journal order: `(path, actor, via, old, new)`.
fn rekeys(lab: &Lab) -> Vec<(String, String, String, Value, Value)> {
    lab.journal()
        .into_iter()
        .filter(|r| r["op"] == "set" && r["field"] == "source_uid")
        .map(|r| {
            let s = |k: &str| r[k].as_str().unwrap().to_string();
            (s("path"), s("actor"), s("via"), r["old"].clone(), r["new"].clone())
        })
        .collect()
}

/// The journal without its stamps: what two identical runs must agree on.
/// `Lab::new`'s own setup note (`archive/_migrated.md`) is left out: `create` mints its `id`, so
/// two vaults never agree on it, and it is no part of the input a fetch reads.
fn trail(lab: &Lab) -> Vec<(String, String, String, String, Value)> {
    lab.journal()
        .into_iter()
        .filter(|r| r["path"] != "archive/_migrated.md")
        .map(|r| {
            let s = |k: &str| r[k].as_str().unwrap_or("").to_string();
            (s("op"), s("path"), s("field"), s("actor"), r["new"].clone())
        })
        .collect()
}

fn uids(lab: &Lab) -> Vec<String> {
    let mut out: Vec<String> = cm::load(&lab.v).confirmed.into_iter().filter_map(|n| n.source_uid).collect();
    out.sort();
    out
}

// ---- what changes ------------------------------------------------------------------------------

/// A confirmed note of the fetched term whose CRN the answer reads whole becomes the registrar's:
/// one `source_uid` record, the registrar as actor, `dashboard` as via, the old key and the new.
#[test]
fn a_read_that_finds_the_section_rekeys_the_note_with_one_record() {
    let lab = lab_with("adopt-rekey", &[("cs-100.md", lecture("cmt_00000000e1", CS_LECTURE))]);
    lab.fetch(D10, &fx());
    assert_eq!(lab.loaded(LECTURE).source_uid.as_deref(), Some(REG_LECTURE));
    assert_eq!(
        rekeys(&lab),
        [(LECTURE.to_string(), ACTOR.to_string(), "dashboard".to_string(), json!(CS_LECTURE), json!(REG_LECTURE))]
    );
}

/// The actor starts `agent:`, so `provenance::is_agent` reads the re-key as an agent's and "judge
/// once" holds for the field.
#[test]
fn the_actor_of_a_rekey_is_an_agent() {
    let lab = lab_with("adopt-actor", &[("cs-100.md", lecture("cmt_00000000e1", CS_LECTURE))]);
    lab.fetch(D10, &fx());
    let (_, actor, _, _, _) = rekeys(&lab).remove(0);
    assert!(actor.starts_with("agent:") && crate::provenance::is_agent(&actor), "{actor}");
}

/// Single-line surgery: with an unchanged row, the only byte of the note that moves is the
/// `source_uid` line (same quoting), and the id and the create record's `new` are untouched.
#[test]
fn the_rekey_changes_the_source_uid_line_and_nothing_else() {
    let lab = lab_with("adopt-surgery", &[("cs-100.md", lecture("cmt_00000000e1", CS_LECTURE))]);
    let before = lab.note(LECTURE);
    lab.fetch(D10, &fx());
    assert_eq!(lab.note(LECTURE), before.replace(CS_LECTURE, REG_LECTURE));
    assert!(!lab.note(LECTURE).contains("classsearch"));
    let others: Vec<String> =
        lab.journal().into_iter().filter(|r| r["path"] == LECTURE && r["op"] == "set").map(|r| r["field"].as_str().unwrap().to_string()).collect();
    assert_eq!(others, ["source_uid"], "a row equal to the note writes no other field");
}

/// After the read there is exactly one note for the section: the re-keyed one. R24 does not make
/// a second note for CRN 40001, and no note anywhere keeps the class-search key.
#[test]
fn there_is_exactly_one_note_for_the_adopted_section() {
    let lab = lab_with("adopt-one", &[("cs-100.md", lecture("cmt_00000000e1", CS_LECTURE))]);
    lab.fetch(D10, &fx());
    assert_eq!(lab.notes_with("40001"), [LECTURE]);
    assert!(uids(&lab).iter().all(|u| !u.starts_with("classsearch:")), "{:?}", uids(&lab));
    assert_eq!(uids(&lab).iter().filter(|u| u.as_str() == REG_LECTURE).count(), 1);
}

// ---- what stays the same -----------------------------------------------------------------------

/// CRN present with a changed time: the change applies to the re-keyed note, a field the student
/// set (`where`, `level`) is kept, listed, never written by the registrar, and still one note.
#[test]
fn a_changed_time_applies_and_a_hand_set_field_is_kept() {
    let lab = lab_with("adopt-apply", &[("cs-100.md", front("cmt_00000000e1", CS_LECTURE, "class", MWF_ELEVEN))]);
    lab.hand_set(LECTURE, "where", "\"Student Room\"");
    lab.hand_set(LECTURE, "level", "soft");
    let mut answer = fx();
    meeting(&mut answer, "40001")["room"] = json!("102");
    let report = lab.fetch(D10, &answer);
    let applied = of_note(&list(&report, "applied"), LECTURE);
    let fields: Vec<&str> = applied.iter().map(|e| e["field"].as_str().unwrap()).collect();
    assert_eq!(fields, ["meets"], "{report}");
    assert_eq!(lab.loaded(LECTURE).meets[0].start, Time::constant(12, 0, 0, 0));
    assert_eq!(lab.loaded(LECTURE).where_.as_deref(), Some("Student Room"));
    assert_eq!(lab.line(LECTURE, "level").as_deref(), Some("soft"));
    for field in ["where", "level"] {
        assert!(lab.sets(LECTURE, field).iter().all(|(actor, _, _)| !actor.starts_with("agent:")), "{field}");
    }
    let kept = of_note(&list(&report, "kept"), LECTURE);
    assert!(kept.iter().any(|e| e["field"] == "where"), "{report}");
    assert_eq!(lab.loaded(LECTURE).source_uid.as_deref(), Some(REG_LECTURE));
    assert_eq!(lab.notes_with("40001"), [LECTURE]);
}

/// A student's own fields survive the re-key byte for byte: title, course, kind and level of a
/// hand-edited note are the lines they were.
#[test]
fn the_students_fields_are_not_touched() {
    let lab = lab_with("adopt-fields", &[("cs-100.md", lecture("cmt_00000000e1", CS_LECTURE))]);
    lab.hand_set(LECTURE, "title", "\"My own title\"");
    let lines = |lab: &Lab| -> Vec<String> {
        ["id", "type", "kind", "level", "title", "course", "meets", "until", "where", "status"]
            .iter().map(|k| lab.line(LECTURE, k).unwrap_or_default()).collect()
    };
    let before = lines(&lab);
    lab.fetch(D10, &fx());
    assert_eq!(lines(&lab), before);
}

// ---- when it does not adopt --------------------------------------------------------------------

/// A mismatch never adopts: a class-search note of another term, and one of another school with
/// the same term and CRN, keep their bytes and get no record.
#[test]
fn a_note_of_another_term_or_school_is_never_adopted() {
    let lab = lab_with(
        "adopt-mismatch",
        &[
            ("cs-100-spring.md", lecture("cmt_00000000e2", CS_SPRING)),
            ("cs-100-xx.md", lecture("cmt_00000000e3", CS_OTHER_SCHOOL)),
        ],
    );
    let (spring, xx) = (lab.note(SPRING_NOTE), lab.note(XX_NOTE));
    lab.fetch(D10, &fx());
    assert_eq!((lab.note(SPRING_NOTE), lab.note(XX_NOTE)), (spring, xx));
    assert!(rekeys(&lab).is_empty(), "{:?}", rekeys(&lab));
}

/// An answer with no usable row is an error before anything is written: no note, no journal
/// record, no file changes.
#[test]
fn an_empty_answer_writes_nothing() {
    let lab = lab_with("adopt-empty", &[("cs-100.md", lecture("cmt_00000000e1", CS_LECTURE))]);
    let before = lab.snapshot();
    let mut empty = fx();
    empty["data"]["registrations"] = json!([]);
    assert!(lab.try_fetch(D10, &empty).is_err());
    assert_eq!(lab.snapshot(), before);
    assert_eq!(lab.loaded(LECTURE).source_uid.as_deref(), Some(CS_LECTURE));
}

/// Plan §6.6 step 2, "CRN absent": the note is the registrar's, ended and listed, because the
/// student is not registered in that section. One note, one re-key, one `ended` entry, and it is
/// the registrar who wrote the `until`.
#[test]
fn a_section_the_answer_lacks_is_rekeyed_then_ended_and_listed() {
    let lab = lab_with("adopt-absent", &[("cs-100-absent.md", lecture("cmt_00000000e4", CS_ABSENT))]);
    let report = lab.fetch(D10, &fx());
    assert_eq!(lab.loaded(ABSENT).source_uid.as_deref(), Some("registrar:ua:202640-49999"));
    assert_eq!(of_note(&list(&report, "ended"), ABSENT).len(), 1, "{report}");
    assert!(lab.sets(ABSENT, "until").iter().all(|(actor, _, _)| actor == ACTOR));
    assert_eq!(lab.notes_with("49999"), [ABSENT]);
}

// ---- a failed re-key ---------------------------------------------------------------------------

/// A note the write path refuses (a block-style `judgment:` line) is a named warning; the note
/// keeps its class-search key, R24 skips that CRN, and there is never a second note for it.
#[test]
fn a_failed_rekey_is_a_warning_and_leaves_one_note() {
    let bad = format!("{}judgment: not a flow mapping\n", lecture("cmt_00000000e1", CS_LECTURE));
    let lab = lab_with("adopt-failed", &[("cs-100.md", bad)]);
    let report = lab.fetch(D10, &fx());
    let warnings: Vec<String> = list(&report, "warnings").iter().filter_map(|w| w.as_str().map(String::from)).collect();
    assert!(warnings.iter().any(|w| w.contains(LECTURE) && w.contains("source_uid")), "{warnings:?}");
    assert_eq!(lab.loaded(LECTURE).source_uid.as_deref(), Some(CS_LECTURE));
    assert_eq!(lab.notes_with("40001"), [LECTURE], "R24 made a second note for the CRN");
    assert!(rekeys(&lab).is_empty());
}

/// Controller ruling 2 (judge once): a note whose `source_uid` the journal shows the student set
/// is never re-keyed. Its bytes stay, no agent writes the field, the warning is in fixed words, and
/// R24 makes no second note for the CRN.
#[test]
fn a_source_uid_the_student_set_is_never_rekeyed() {
    let lab = lab_with("adopt-yours", &[("cs-100.md", lecture("cmt_00000000e1", "manual:cs-100"))]);
    lab.hand_set(LECTURE, "source_uid", &format!("\"{CS_LECTURE}\""));
    let before = lab.note(LECTURE);
    let report = lab.fetch(D10, &fx());
    assert_eq!(lab.note(LECTURE), before);
    assert!(rekeys(&lab).iter().all(|(_, actor, _, _, _)| !actor.starts_with("agent:")), "{:?}", rekeys(&lab));
    let warning = format!("registrar: {LECTURE}: source_uid kept yours; the class-search note was not adopted");
    assert!(list(&report, "warnings").contains(&json!(warning)), "{report}");
    assert_eq!(lab.notes_with("40001"), [LECTURE], "R24 made a second note for the CRN");
}

// ---- determinism, and a second run -------------------------------------------------------------

fn two_notes(name: &str) -> Lab {
    lab_with(name, &[("cs-100.md", lecture("cmt_00000000e1", CS_LECTURE)), ("cs-100-lab.md", lab("cmt_00000000e5", CS_LAB))])
}

/// Notes are adopted in path order, and two vaults that hold the same input end with the same
/// records in the same order.
#[test]
fn adoption_is_in_path_order_and_the_same_input_gives_the_same_trail() {
    let (a, b) = (two_notes("adopt-det-a"), two_notes("adopt-det-b"));
    a.fetch(D10, &fx());
    b.fetch(D10, &fx());
    let order: Vec<String> = rekeys(&a).into_iter().map(|r| r.0).collect();
    assert_eq!(order, [LAB, LECTURE]);
    assert_eq!(rekeys(&a).iter().map(|r| r.4.clone()).collect::<Vec<_>>(), [json!(REG_LAB), json!(REG_LECTURE)]);
    assert_eq!(trail(&a), trail(&b));
    assert_eq!(a.note(LECTURE), b.note(LECTURE));
    assert_eq!(a.note(LAB), b.note(LAB));
}

/// A second run of the same answer writes nothing to any note and adds no journal record.
#[test]
fn a_second_run_writes_nothing() {
    let lab = two_notes("adopt-twice");
    lab.fetch(D10, &fx());
    let (journal, notes) = (lab.journal(), [lab.note(LECTURE), lab.note(LAB)]);
    lab.fetch(D10, &fx());
    assert_eq!(lab.journal(), journal);
    assert_eq!([lab.note(LECTURE), lab.note(LAB)], notes);
    assert_eq!(rekeys(&lab).len(), 2);
}

// ---- the term's unconfirmed class-search leftovers ---------------------------------------------

fn t(h: i8, m: i8) -> Time {
    Time::new(h, m, 0, 0).unwrap()
}

fn cs_row(uid: &str, cal: &str) -> Series {
    Series {
        source_uid: uid.to_string(),
        calendar: cal.to_string(),
        title: "CS 100 Invented".to_string(),
        where_: None,
        event_type: None,
        rule: Rule { freq: "WEEKLY".into(), interval: 1, until: None, count: None },
        has_master: true,
        rdate: false,
        unsupported: false,
        instances: Vec::<Instance>::new(),
        meets: vec![Meet { days: vec!["tue"], start: t(9, 0), end: t(9, 50) }],
        first: Some(D1),
        until: None,
        last_seen: Some(D1),
    }
}

/// After a read, the fetched term's unconfirmed `classsearch:` rows and its `calendars` entry are
/// gone, with no `ended` entry; another term's row and entry stay; the no-time block stays.
#[test]
fn the_terms_unconfirmed_rows_go_and_another_terms_stay() {
    let lab = Lab::new("adopt-purge", "2026-08-31");
    let mut file = cm::SeriesFile::default();
    file.series = vec![cs_row("classsearch:ua:202640-40004", CS_CAL), cs_row(CS_SPRING, SPRING_CAL)];
    file.calendars.insert(CS_CAL.to_string(), D1);
    file.calendars.insert(SPRING_CAL.to_string(), D1);
    file.class_search.insert(CS_CAL.to_string(), ["cs-200".to_string()].into_iter().collect());
    std::fs::create_dir_all(lab.path("state")).unwrap();
    std::fs::write(lab.path("state/calendar-series.json"), cm::file_bytes(&file)).unwrap();
    lab.fetch(D10, &fx());
    let (after, _) = cm::read_series_file(&lab.v);
    let keys: Vec<&str> = after.series.iter().map(|s| s.source_uid.as_str()).filter(|k| k.starts_with("classsearch:")).collect();
    assert_eq!(keys, [CS_SPRING]);
    assert!(!after.calendars.contains_key(CS_CAL) && after.calendars.contains_key(SPRING_CAL), "{:?}", after.calendars);
    assert!(after.ended.keys().all(|k| !k.starts_with("classsearch:")), "{:?}", after.ended);
    assert!(after.class_search.contains_key(CS_CAL), "the no-time block stays");
}

/// A vault with no class-search note re-keys nothing: no `source_uid` record at all.
#[test]
fn a_vault_with_no_class_search_note_has_no_rekey() {
    let lab = Lab::new("adopt-none", "2026-08-31");
    lab.fetch(D1, &fx());
    assert!(rekeys(&lab).is_empty());
    assert!(uids(&lab).iter().all(|u| u.starts_with("registrar:")), "{:?}", uids(&lab));
}
