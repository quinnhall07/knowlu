//! C2c re-review (A1-F1, "cannot drift"): a `registrar:` note the registrar has ended closes no row
//! of its own term's calendar by signature, and every site that closes a question by signature
//! (`proposals`, `confirm`, `withdrawal_reason`, `settle_approved`) agrees. It still closes every
//! other row. Every title, key and date is invented.
use super::apply_support::*;
use crate::commitments::{self as cm, Instance, Meet, Rule, Series};
use jiff::civil::Time;
use serde_json::Value;

const ENGL_OLD: &str = "registrar:ua:202640-40003";
const ENGL_NEW: &str = "registrar:ua:202640-40013";
const ENDED_KEY: &str = "registrar:ua:202640-49999";

/// The review's input, with no class search: a row for a course the code table lacks ("to check"),
/// approved by card (a note with no `course`, matched by title), then swapped by the registrar for
/// another section at the same times. The next read ends the old note and R24 skips the new row
/// (course unknown). Two `rank`s give exactly one card for it, still pending after the second, and
/// approving it makes the new section's note.
#[test]
fn a_swapped_sections_card_is_filed_once_and_approving_it_makes_the_note() {
    let lab = Lab::new("ended-swap", "2026-08-31");
    lab.fetch(D1, &fx());
    lab.rank(D2);
    let first = lab.proposal_cards(ENGL_OLD).remove(0);
    lab.set_status_of("approvals", &first.name, "approved");
    lab.rank(D2);
    let old = lab.notes_with(ENGL_OLD);
    assert_eq!(old.len(), 1, "the approved card made the note");
    assert_eq!(lab.line(&old[0], "course"), None, "the note matches by title");
    let swapped: Value = serde_json::from_str(&FIXTURE.replace("40003", "40013")).unwrap();
    lab.fetch(D10, &swapped);
    assert_eq!(lab.line(&old[0], "until").as_deref(), Some("2026-09-09"), "the old section's note ended");
    assert!(lab.notes_with(ENGL_NEW).is_empty(), "R24 makes no note for a course the vault lacks");
    lab.rank(D10);
    lab.rank(D11);
    let cards = lab.proposal_cards(ENGL_NEW);
    assert_eq!(cards.len(), 1, "{cards:?}");
    assert_eq!((cards[0].folder, cards[0].status()), ("approvals", "pending"), "{cards:?}");
    lab.set_status_of("approvals", &cards[0].name, "approved");
    lab.rank(D11);
    assert_eq!(lab.notes_with(ENGL_NEW).len(), 1, "approving the card makes the new section's note");
}

const MWF_NOON: &str = "{days: [mon, wed, fri], start: \"12:00\", end: \"12:50\"}";

/// A vault holding one confirmed `registrar:` CS 100 lecture of term 202640 that the registrar
/// ended on `2026-09-09` (before `D10`), at Mon/Wed/Fri 12:00-12:50.
fn with_ended_note(name: &str) -> Lab {
    let lab = Lab::new(name, "2026-08-31");
    let front = format!(
        "id: cmt_00000000e6\ntype: commitment\nkind: class\nlevel: hard\ntitle: \"CS 100 Invented\"\ncourse: cs-100\n\
         meets: [{MWF_NOON}]\nuntil: 2026-09-09\nsource_uid: {ENDED_KEY}\nstatus: confirmed\nconfirmed_at: 2026-08-31\n"
    );
    lab.plant_note("cs-100-ended.md", &front);
    lab
}

/// A weekly row at the ended note's own times (so the same signature), under `calendar`.
fn noon_row(uid: &str, calendar: &str) -> Series {
    let t = |h: i8, m: i8| Time::new(h, m, 0, 0).unwrap();
    Series {
        source_uid: uid.to_string(),
        calendar: calendar.to_string(),
        title: "CS 100 Invented".to_string(),
        where_: None,
        event_type: None,
        rule: Rule { freq: "WEEKLY".into(), interval: 1, until: None, count: None },
        has_master: true,
        rdate: false,
        unsupported: false,
        instances: Vec::<Instance>::new(),
        meets: vec![Meet { days: vec!["mon", "wed", "fri"], start: t(12, 0), end: t(12, 50) }],
        first: Some(D1),
        until: None,
        last_seen: Some(D1),
    }
}

fn write_rows(lab: &Lab, rows: Vec<Series>) {
    let mut file = cm::SeriesFile::default();
    for row in &rows {
        file.calendars.insert(row.calendar.clone(), D1);
    }
    file.series = rows;
    std::fs::create_dir_all(lab.path("state")).unwrap();
    std::fs::write(lab.path("state/calendar-series.json"), cm::file_bytes(&file)).unwrap();
}

/// Whether `key` is a question with no notes (`unanswered`) and a question over the vault's notes.
fn asked(lab: &Lab, key: &str) -> (bool, bool) {
    let stored = cm::stored_proposals(&lab.v, D10);
    let has = |list: &[cm::Proposal]| list.iter().any(|p| p.source_uid == key);
    (has(&stored.unanswered), has(&stored.proposals))
}

/// "It still closes every other row": a class-search row of the same term, another term's
/// registrar row and a Google class, each at the ended note's signature, each a question on its
/// own, stay unproposed beside it, as before C2c.
#[test]
fn an_ended_registrar_note_still_closes_every_other_row_by_signature() {
    let rows = [
        ("ended-cs", "classsearch:ua:202640-40001", "classsearch:ua:202640"),
        ("ended-spring", "registrar:ua:202710-50001", "registrar:ua:202710"),
    ];
    for (name, key, calendar) in rows {
        let lab = with_ended_note(name);
        write_rows(&lab, vec![noon_row(key, calendar)]);
        assert_eq!(asked(&lab, key), (true, false), "{name}");
    }
    let lab = with_ended_note("ended-google");
    let google = google_item("cs100bb", "CS 100", &[0, 2, 4], "12:00", "12:50", 8);
    lab.rank_with(D10, vec![("cloud:google", google_entry(vec![google]))]);
    assert_eq!(asked(&lab, "gcal-series:cs100bb"), (true, false), "a Google class");
    assert!(lab.proposal_cards("gcal-series:cs100bb").is_empty(), "no card for the Google class");
}

/// The same term's row is not closed by it, and Mine on that row through the screen's path
/// (`commitments --confirm`, [`cm::confirm`]) makes its note, with no warning.
#[test]
fn mine_on_the_same_terms_row_makes_its_note_beside_the_ended_one() {
    let lab = with_ended_note("ended-mine");
    let key = "registrar:ua:202640-40011";
    write_rows(&lab, vec![noon_row(key, CAL)]);
    assert_eq!(asked(&lab, key), (true, true));
    let input = cm::ConfirmInput { mine: vec![(key.to_string(), "hard".to_string())], not_mine: Vec::new(), window: None };
    let done = cm::confirm(&lab.v, &input, D10, &lab.ctx(), &mut crate::journal::Journal::new(&lab.v)).unwrap();
    assert_eq!((done.created, done.warnings.len()), (1, 0), "{:?}", done.warnings);
    assert_eq!(lab.notes_with(key).len(), 1);
}
