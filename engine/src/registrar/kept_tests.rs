//! Close-out task B2, the B3e tests: a field the student set by hand is never written, its change
//! is kept and filed as one card at most once; the card is withdrawn with its entry; one filer.
//! Spec A2 items 11 (the hand-set `until`) and 12, and item 20 (round 1, round 2, convergence rounds
//! 1, 3 and 5). Invented data only. The standard scene: CS 100's lecture is confirmed at the
//! registrar's Mon/Wed/Fri 12:00-12:50, and the student then sets `meets` to Tue 09:00-09:50 by hand.
use super::apply_support::*;
use crate::commitments::Settled;
use serde_json::{json, Value};

const STUDENT: &str = "[{days: [tue], start: \"09:00\", end: \"09:50\"}]";
const KEY_ENGL: &str = "registrar:ua:202640-40003";

fn meets(days: &[&str], start: &str, end: &str) -> Value {
    json!([{ "days": days, "start": start, "end": end }])
}

fn student_meets() -> Value {
    meets(&["tue"], "09:00", "09:50")
}

/// The registrar's lecture, moved to Mon/Wed/Fri `begin`-`end` (HHMM).
fn lecture_at(begin: &str, end: &str) -> Value {
    let mut answer = fx();
    retime(&mut answer, "40001", begin, end);
    answer
}

fn at_13() -> Value {
    lecture_at("1300", "1350")
}

fn at_13_json() -> Value {
    meets(&["mon", "wed", "fri"], "13:00", "13:50")
}

/// A vault with the lecture confirmed on `D1` and then edited by the student.
fn edited(name: &str) -> Lab {
    let lab = Lab::new(name, "2026-08-31");
    lab.fetch(D1, &fx());
    lab.hand_set(LECTURE, "meets", STUDENT);
    lab
}

/// The one open card for a note's field; the message names what is missing.
fn the_card(lab: &Lab, note: &str, field: &str) -> Card {
    let mut cards = lab.open_cards(note, field);
    assert_eq!(cards.len(), 1, "expected one open {field} card for {note}: {:?}", lab.change_cards());
    cards.remove(0)
}

fn without_lab() -> Value {
    let mut later = fx();
    drop_crn(&mut later, "40002");
    later
}

fn kept_of(report: &Value, note: &str, field: &str) -> Vec<Value> {
    of_note(&list(report, "kept"), note).into_iter().filter(|e| e["field"] == field).collect()
}

/// B3e (A2 item 12; item 20 plan bullet 3). The registrar moves a field the student set by hand:
/// it is not written, the change is kept and listed as filed, and exactly one card proposes it
/// (`was` the student's value). A second identical fetch and a `rank` file no second card.
#[test]
fn a_hand_set_field_is_not_written_and_gets_one_card_and_a_repeat_files_no_second() {
    let lab = edited("kept-one");
    let report = lab.fetch(D10, &at_13());
    assert!(of_note(&list(&report, "applied"), LECTURE).iter().all(|e| e["field"] != "meets"), "{report}");
    let kept = kept_of(&report, LECTURE, "meets");
    assert_eq!(kept.len(), 1, "{report}");
    assert_eq!((&kept[0]["registrar"], &kept[0]["student"], &kept[0]["card"]), (&at_13_json(), &student_meets(), &json!("filed")));
    assert_eq!(lab.loaded(LECTURE).meets[0].days, vec!["tue"], "the registrar overwrote the student");
    assert_eq!(lab.sets(LECTURE, "meets").len(), 1, "only the student's own record");
    let cards = lab.open_cards(LECTURE, "meets");
    assert_eq!(cards.len(), 1, "{:?}", lab.change_cards());
    assert_eq!((&cards[0].meta["change"]["meets"], &cards[0].meta["was"]["meets"]), (&at_13_json(), &student_meets()));
    let again = lab.fetch(D20, &at_13());
    assert_eq!(kept_of(&again, LECTURE, "meets").len(), 1, "a standing entry is still listed");
    lab.rank(D20);
    assert_eq!(lab.change_cards().len(), 1, "a second card: {:?}", lab.change_cards());
}

/// B3e (A2 item 20, round 1, "A held card is not lost"). With the day's five `commitment-check`
/// cards already used, the fetch files nothing, keeps the entry and lists it as waiting; the next
/// day's `rank` files the card; a third run files no second.
#[test]
fn a_held_card_is_not_lost() {
    let lab = edited("kept-held");
    lab.spend_the_cap(D10);
    let report = lab.fetch(D10, &at_13());
    assert_eq!(kept_of(&report, LECTURE, "meets")[0]["card"], "waiting", "{report}");
    assert!(lab.change_cards().is_empty(), "{:?}", lab.change_cards());
    lab.rank(D11);
    assert_eq!(lab.open_cards(LECTURE, "meets").len(), 1);
    lab.rank(D11);
    assert_eq!(lab.change_cards().len(), 1, "{:?}", lab.change_cards());
}

/// B3e (round 1, "The student's override is not proposed back"). A hand-set value other than the
/// registrar's, with the registrar's value unchanged since the previous fetch, gets no kept entry
/// and no card, at the fetch or at `rank`.
#[test]
fn the_students_override_is_not_proposed_back() {
    let lab = edited("kept-override");
    let report = lab.fetch(D10, &fx());
    assert!(list(&report, "kept").is_empty(), "{report}");
    lab.rank(D10);
    assert!(lab.change_cards().is_empty(), "{:?}", lab.change_cards());
    assert_eq!(lab.loaded(LECTURE).meets[0].days, vec!["tue"]);
}

/// B3e (round 1, "No prior row"). With no prior registrar row in the series file, a hand-set
/// field is kept and listed only: no card, at the fetch or at `rank`.
#[test]
fn a_kept_entry_with_no_prior_row_is_listed_only() {
    let lab = edited("kept-noprior");
    std::fs::remove_file(lab.path("state/calendar-series.json")).unwrap();
    let report = lab.fetch(D10, &at_13());
    let kept = kept_of(&report, LECTURE, "meets");
    assert_eq!((kept.len(), kept[0]["card"].as_str()), (1, Some("listed")), "{report}");
    lab.rank(D10);
    assert!(lab.change_cards().is_empty(), "{:?}", lab.change_cards());
}

/// B3e (round 1, "Change cards on day 1"). A kept-field card is filed on the vault's first day.
#[test]
fn a_kept_field_card_is_filed_on_the_vaults_first_day() {
    let lab = Lab::new("kept-day1", "2026-09-01");
    lab.fetch(D1, &fx());
    lab.hand_set(LECTURE, "meets", STUDENT);
    lab.fetch(D1, &at_13());
    lab.rank(D1);
    assert_eq!(lab.open_cards(LECTURE, "meets").len(), 1, "{:?}", lab.change_cards());
}

/// B3e (round 1, "Agreement clears it", the first case). After the student accepts the card the
/// entry is neither listed nor offered; a later fetch whose value equals the note's removes it from
/// the block.
#[test]
fn accepting_the_card_clears_the_entry() {
    let lab = edited("kept-accept");
    lab.fetch(D10, &at_13());
    let card = the_card(&lab, LECTURE, "meets");
    lab.set_status_of("approvals", &card.name, "approved");
    lab.rank(D11);
    assert_eq!(lab.loaded(LECTURE).meets[0].days, vec!["mon", "wed", "fri"], "the approved card was not applied");
    lab.rank(D11);
    assert_eq!(lab.change_cards().len(), 1, "the entry was offered again: {:?}", lab.change_cards());
    let later = lab.fetch(D20, &at_13());
    assert!(list(&later, "kept").is_empty(), "{later}");
    assert!(lab.kept_entries().is_empty(), "{:?}", lab.term());
}

/// B3e (convergence round 3, "Agreement clears it", the second case). With the card still pending,
/// a fetch whose value moves to the student's own removes the entry and stamps the card
/// `superseded`; approving it afterwards writes nothing; the next `rank` files no card.
#[test]
fn the_registrar_agreeing_with_the_student_withdraws_the_pending_card() {
    let lab = edited("kept-agree");
    lab.fetch(D10, &at_13());
    let card = the_card(&lab, LECTURE, "meets");
    let mut agree = fx();
    set_days(&mut agree, "40001", &["tuesday"]);
    retime(&mut agree, "40001", "0900", "0950");
    let report = lab.fetch(D20, &agree);
    assert!(list(&report, "kept").is_empty() && lab.kept_entries().is_empty(), "{report}");
    let after: Vec<_> = lab.change_cards().into_iter().filter(|c| c.name == card.name).collect();
    assert_eq!((after[0].folder, after[0].status()), ("archive", "superseded"), "{after:?}");
    let moved = Card { folder: after[0].folder, ..card.clone() };
    assert!(matches!(lab.settle(&moved, D20), Settled::Superseded(_)));
    assert_eq!(lab.loaded(LECTURE).meets[0].days, vec!["tue"], "approving the withdrawn card wrote");
    lab.rank(D20);
    assert!(lab.open_cards(LECTURE, "meets").is_empty(), "{:?}", lab.change_cards());
}

/// B3e (A2 item 11, "When the student set `until` by hand"; item 20, round 1). A dropped course
/// whose `until` the student set writes nothing, is listed "ended by myBama, kept yours", and gets
/// one change card proposing the day before the first fetch that found it missing.
#[test]
fn a_dropped_course_with_a_hand_set_until_is_kept_yours_with_one_card() {
    let lab = Lab::new("kept-until", "2026-08-31");
    lab.fetch(D1, &fx());
    lab.hand_set(LAB, "until", "2026-11-20");
    let report = lab.fetch(D10, &without_lab());
    assert_eq!(lab.loaded(LAB).until, Some(jiff::civil::date(2026, 11, 20)), "the registrar overwrote the student");
    assert_eq!(lab.sets(LAB, "until").len(), 1, "only the student's own record");
    let ended = of_note(&list(&report, "ended"), LAB);
    assert_eq!((ended.len(), ended[0]["until"].as_str(), ended[0]["kept_yours"].as_bool()), (1, Some("2026-09-09"), Some(true)), "{report}");
    let kept = kept_of(&report, LAB, "until");
    assert_eq!((kept[0]["registrar"].as_str(), kept[0]["student"].as_str(), kept[0]["card"].as_str()), (Some("2026-09-09"), Some("2026-11-20"), Some("filed")));
    let cards = lab.open_cards(LAB, "until");
    assert_eq!((cards.len(), cards[0].meta["change"]["until"].as_str()), (1, Some("2026-09-09")), "{:?}", lab.change_cards());
}

/// B3eR fix 1 (A2 item 12, "A held card is not lost", across 11's come-back). A hand-set `meets`
/// the registrar moved has its kept entry and one card; the course is missing, then whole again
/// with the same moved value. The come-back has no prior row (the drop moved it to `ended`), which
/// detects nothing: the entry stands, is not turned into a listed-only one, and keeps its one card.
#[test]
fn a_kept_entry_and_its_card_survive_a_drop_and_a_whole_come_back() {
    let lab = edited("kept-comeback");
    lab.fetch(D10, &at_13());
    the_card(&lab, LECTURE, "meets");
    let mut gone = at_13();
    drop_crn(&mut gone, "40001");
    lab.fetch(D20, &gone);
    let back = lab.fetch(D30, &at_13());
    let kept = kept_of(&back, LECTURE, "meets");
    assert_eq!((kept.len(), kept[0]["card"].as_str()), (1, Some("filed")), "{back}");
    assert!(lab.kept_entries().iter().all(|e| e.get("listed").is_none()), "{:?}", lab.kept_entries());
    the_card(&lab, LECTURE, "meets");
    lab.rank(D30);
    assert_eq!(cards_of(&lab, LECTURE, "meets").len(), 1, "{:?}", lab.change_cards());
}

/// B3eR fix 1, the second scenario. A hand-set `where` the registrar never changed is not listed
/// "myBama changed it, kept yours" after the course is missing and whole again.
#[test]
fn a_hand_set_field_the_registrar_never_changed_is_not_listed_after_a_come_back() {
    let lab = Lab::new("kept-comeback-where", "2026-08-31");
    lab.fetch(D1, &fx());
    lab.hand_set(LECTURE, "where", "\"Invented Hall 9\"");
    let mut gone = fx();
    drop_crn(&mut gone, "40001");
    lab.fetch(D10, &gone);
    let back = lab.fetch(D20, &fx());
    assert!(list(&back, "kept").is_empty() && lab.kept_entries().is_empty(), "{back}");
    lab.rank(D20);
    assert!(lab.change_cards().is_empty(), "{:?}", lab.change_cards());
}

/// B3e (round 2, "`was` is read at filing"). A kept entry recorded with the student's value; the
/// student edits the field to a third value; the next `rank` files the card with `was` that third
/// value, and it is not superseded at that `rank`.
#[test]
fn was_is_read_at_filing() {
    let lab = edited("kept-was");
    lab.spend_the_cap(D10);
    lab.fetch(D10, &at_13());
    lab.hand_set(LECTURE, "meets", "[{days: [wed], start: \"10:00\", end: \"10:50\"}]");
    lab.rank(D11);
    let cards = lab.open_cards(LECTURE, "meets");
    assert_eq!(cards.len(), 1, "{:?}", lab.change_cards());
    assert_eq!(cards[0].meta["was"]["meets"], meets(&["wed"], "10:00", "10:50"));
    assert!(lab.change_cards().iter().all(|c| c.status() == "pending"), "{:?}", lab.change_cards());
}

/// B3e (round 2, "Expired reopens", M-f). A kept-field card that expires unanswered is filed again
/// at the next `rank`, once; a rejected one is not.
#[test]
fn an_expired_kept_card_is_filed_again_and_a_rejected_one_is_not() {
    for (outcome, again) in [("expired", 1), ("rejected", 0)] {
        let lab = edited(&format!("kept-{outcome}"));
        lab.fetch(D10, &at_13());
        let card = the_card(&lab, LECTURE, "meets");
        lab.set_status_of("approvals", &card.name, outcome);
        lab.rank(D11);
        lab.rank(D11);
        assert_eq!(lab.open_cards(LECTURE, "meets").len(), again, "{outcome}: {:?}", lab.change_cards());
        assert_eq!(lab.change_cards().len(), 1 + again, "{outcome}: {:?}", lab.change_cards());
    }
}

/// B3e (round 2, "One filer"). With `SERIES_LOCK_FILE` held by the test, a `rank` that has a kept
/// entry to file does not write a card until the lock is released; a fetch and a `rank` on the
/// same day file one card between them. B3eR fix 1: the "held" check runs once the rank has
/// signalled from the filer's lock step (`rank_interleaved`), never while it is still in
/// `refresh_series`, so a filer that takes no lock turns this test red.
#[test]
fn one_filer_holds_the_series_lock_and_a_fetch_and_a_rank_file_one_card() {
    let lab = edited("kept-lock");
    lab.spend_the_cap(D10);
    lab.fetch(D10, &at_13());
    assert!(lab.change_cards().is_empty());
    rank_interleaved(&lab, D11, || assert!(lab.change_cards().is_empty(), "a card was written while the lock was held"));
    assert_eq!(lab.open_cards(LECTURE, "meets").len(), 1);
    let other = edited("kept-sameday");
    other.fetch(D10, &at_13());
    other.rank(D10);
    assert_eq!(other.change_cards().len(), 1, "{:?}", other.change_cards());
}

const CHESS: &str = "id: cmt_00000000a4\ntype: commitment\nkind: club\nlevel: soft\ntitle: \"Chess Club\"\n\
    meets: [{days: [wed], start: \"18:00\", end: \"19:00\"}]\nsource_uid: \"gcal-series:chess01\"\nstatus: confirmed\n";

fn cards_of(lab: &Lab, note: &str, field: &str) -> Vec<Card> {
    lab.change_cards().into_iter().filter(|c| c.meta["target"] == note && c.meta["change"].get(field).is_some()).collect()
}

fn named(lab: &Lab, card: &Card) -> Card {
    lab.change_cards().into_iter().find(|c| c.name == card.name).unwrap_or_else(|| panic!("{} is gone", card.name))
}

/// B3e (convergence round 1 and round 3, "The same with a hand-set `until`"). Day 1 end (one card),
/// day 20 whole again (the entry closed, the day-1 card withdrawn, a card for the row's value), day
/// 40 missing again (a new kept value, the day-20 card withdrawn, one new card), day 50 the same
/// answer (no more). At most one `until` card is open at any time, and the withdrawn ones write
/// nothing when approved.
#[test]
fn dropped_back_and_dropped_again_with_a_hand_set_until_keeps_one_open_card() {
    let lab = Lab::new("kept-twice", "2026-08-31");
    lab.fetch(D1, &fx());
    lab.hand_set(LAB, "until", "2026-11-20");
    lab.fetch(D10, &without_lab());
    let first = the_card(&lab, LAB, "until");
    assert_eq!(first.meta["change"]["until"], "2026-09-09");
    let back = lab.fetch(D20, &fx());
    assert_eq!(lab.sets(LAB, "until").len(), 1, "the come-back wrote");
    assert_eq!(kept_of(&back, LAB, "until")[0]["registrar"], "2026-12-04", "{back}");
    assert_eq!(lab.ended_entries()[0]["returned"], "2026-09-20");
    let (gone, second) = (named(&lab, &first), the_card(&lab, LAB, "until"));
    assert_eq!((gone.folder, gone.status()), ("archive", "superseded"), "the day-1 card");
    assert_eq!(second.meta["change"]["until"], "2026-12-04");
    lab.fetch(D40, &without_lab());
    assert_eq!(lab.ended_entries().len(), 2, "a second, open entry");
    let (gone_again, third) = (named(&lab, &second), the_card(&lab, LAB, "until"));
    assert_eq!(gone_again.status(), "superseded", "the day-20 card");
    assert_eq!(third.meta["change"]["until"], "2026-10-09");
    lab.rank(D40);
    lab.fetch(D50, &without_lab());
    lab.rank(D50);
    assert_eq!((lab.open_cards(LAB, "until").len(), cards_of(&lab, LAB, "until").len()), (1, 3), "{:?}", lab.change_cards());
    assert!(matches!(lab.settle(&gone, D50), Settled::Superseded(_)));
    assert_eq!(lab.loaded(LAB).until, Some(jiff::civil::date(2026, 11, 20)), "a withdrawn card wrote");
}

/// B3e (convergence round 3, "A replaced value withdraws its card"). The registrar's value goes B,
/// then C: card B is stamped `superseded` at the fetch that replaces it, card C is filed, approving
/// B writes nothing, and a fetch back to B files a new card (a superseded card does not close the
/// question).
#[test]
fn a_replaced_value_withdraws_its_card() {
    let lab = edited("kept-replaced");
    lab.fetch(D10, &at_13());
    let b = the_card(&lab, LECTURE, "meets");
    lab.fetch(D20, &lecture_at("1400", "1450"));
    let (b_now, c) = (named(&lab, &b), the_card(&lab, LECTURE, "meets"));
    assert_eq!((b_now.folder, b_now.status()), ("archive", "superseded"));
    assert_eq!(c.meta["change"]["meets"], meets(&["mon", "wed", "fri"], "14:00", "14:50"));
    assert!(matches!(lab.settle(&b_now, D20), Settled::Superseded(_)));
    assert_eq!(lab.loaded(LECTURE).meets[0].days, vec!["tue"], "approving the withdrawn card wrote");
    lab.fetch(D30, &at_13());
    let again = the_card(&lab, LECTURE, "meets");
    assert_eq!(again.meta["change"]["meets"], at_13_json());
    assert_eq!((named(&lab, &c).status(), cards_of(&lab, LECTURE, "meets").len()), ("superseded", 3));
}

/// B3e (convergence round 3, "A block that is gone withdraws nothing"). With a pending kept-field
/// card, a `rank` over a series file whose registrar block was removed leaves the card pending; a
/// `gcal-series:` change card is untouched.
#[test]
fn a_block_that_is_gone_withdraws_nothing() {
    let lab = edited("kept-gone");
    lab.plant_note("chess-club.md", CHESS);
    lab.fetch(D10, &at_13());
    let chess = || vec![("cloud:google", google_entry(vec![google_item("chess01", "Chess Club", &[3], "18:00", "19:00", 8)]))];
    lab.rank_with(D10, chess());
    assert_eq!(lab.change_cards().len(), 2, "{:?}", lab.change_cards());
    lab.drop_block();
    lab.rank_with(D11, chess());
    let all = lab.change_cards();
    assert!(all.len() == 2 && all.iter().all(|c| c.folder == "approvals" && c.status() == "pending"), "{all:?}");
}

/// B3e (convergence round 3 and round 4, "A `rank` does not withdraw a fresh card"). A `rank` that
/// read the series file before a fetch wrote a new kept entry, and whose withdrawal calls run after
/// that fetch filed the entry's card, leaves the card pending: the second call re-reads the block
/// under the lock. The fetch is staged by running it on a copy of the vault and adopting its state
/// while the `rank` waits for the lock, never by timing.
#[test]
fn a_rank_that_read_the_file_before_a_fetch_does_not_withdraw_the_fetchs_card() {
    let lab = edited("kept-race3");
    let fetched = lab.clone_as("kept-race3-fetched", "2026-08-31");
    fetched.fetch(D10, &at_13());
    assert_eq!(fetched.open_cards(LECTURE, "meets").len(), 1, "the staged fetch filed no card");
    rank_interleaved(&lab, D10, || lab.adopt(&fetched));
    let cards = lab.change_cards();
    assert!(cards.len() == 1 && cards[0].folder == "approvals" && cards[0].status() == "pending", "{cards:?}");
}

/// Round 4's replaced-value case. The rank's `refresh_series` read entry B (card B pending); a
/// fetch then moves the value to C (entry C, card B superseded, card C filed). The rank leaves
/// card C pending and files no new card B; exactly one kept-field card is pending, and the next
/// `rank` files nothing.
#[test]
fn a_rank_that_read_entry_b_does_not_refile_b_after_a_fetch_moved_it_to_c() {
    let lab = edited("kept-race4");
    lab.fetch(D10, &at_13());
    assert_eq!(the_card(&lab, LECTURE, "meets").meta["change"]["meets"], at_13_json());
    let fetched = lab.clone_as("kept-race4-fetched", "2026-08-31");
    fetched.fetch(D20, &lecture_at("1400", "1450"));
    rank_interleaved(&lab, D20, || lab.adopt(&fetched));
    let open = lab.open_cards(LECTURE, "meets");
    assert_eq!(open.len(), 1, "{:?}", lab.change_cards());
    assert_eq!(open[0].meta["change"]["meets"], meets(&["mon", "wed", "fri"], "14:00", "14:50"));
    assert_eq!(cards_of(&lab, LECTURE, "meets").len(), 2, "a new card B: {:?}", lab.change_cards());
    lab.rank(D20);
    assert_eq!(cards_of(&lab, LECTURE, "meets").len(), 2);
}

/// B3e (P9; A2 item 20, convergence round 1, "The merge keeps the last whole row", the fetch-3
/// halves). The student's `meets` differs from the registrar's W. Fetch 2 reads the row `partial`
/// and detects nothing; fetch 3 reads W whole again: no kept entry, no card.
#[test]
fn a_partial_read_between_two_whole_ones_leaves_the_students_override_alone() {
    let lab = edited("kept-partial");
    let mut part = fx();
    read_in_part(&mut part, "40001", "partial");
    let second = lab.fetch(D10, &part);
    assert!(list(&second, "kept").is_empty(), "{second}");
    let third = lab.fetch(D20, &fx());
    assert!(list(&third, "kept").is_empty() && lab.kept_entries().is_empty(), "{third}");
    lab.rank(D20);
    assert!(lab.change_cards().is_empty(), "{:?}", lab.change_cards());
}

/// B3e (the `unread` variant, and the `status` variant that behaves as it does). Fetch 2 reads the
/// row `unread` (its row stays in the file); fetch 3 whose registrar value moved to X records a kept
/// entry with its card, not listed only: the prior row was carried.
#[test]
fn after_an_unread_or_status_read_a_moved_value_is_kept_with_its_card() {
    for how in ["unread", "status"] {
        let lab = edited(&format!("kept-after-{how}"));
        let mut part = fx();
        read_in_part(&mut part, "40001", how);
        fetch_read_in_part(&lab, D10, &part, how);
        let third = lab.fetch(D20, &at_13());
        let kept = kept_of(&third, LECTURE, "meets");
        assert_eq!((kept.len(), kept[0]["card"].as_str()), (1, Some("filed")), "{how}: {third}");
        the_card(&lab, LECTURE, "meets");
    }
}

/// B3e (A2 item 20, convergence round 3, "A dropped elective is not carried", the card part). An
/// unmatched course's proposal card is stamped `superseded` by the fetch that reads its row in part
/// (or under another status); the next `rank` proposes nothing for the key and no note is created;
/// a fetch that reads the row whole again leads the next `rank` to file one new card.
#[test]
fn a_dropped_electives_card_is_withdrawn_at_the_fetch_and_filed_again_when_it_returns() {
    for how in ["status", "partial", "unread"] {
        let lab = Lab::new(&format!("kept-elective-{how}"), "2026-08-31");
        lab.fetch_registered(D1, &fx(), &REGISTERED);
        lab.rank(D2);
        let filed = lab.proposal_cards(KEY_ENGL);
        assert_eq!((filed.len(), filed[0].status()), (1, "pending"), "{how}");
        let mut later = fx();
        read_in_part(&mut later, "40003", how);
        lab.fetch_registered(D10, &later, &REGISTERED);
        let after = lab.proposal_cards(KEY_ENGL);
        assert_eq!((after.len(), after[0].folder, after[0].status()), (1, "archive", "superseded"), "{how}: {after:?}");
        lab.rank(D11);
        assert_eq!(lab.proposal_cards(KEY_ENGL).len(), 1, "{how}: refiled");
        assert!(!lab.path("commitments/engl-101.md").exists(), "{how}: a note was created");
        lab.fetch_registered(D20, &fx(), &REGISTERED);
        lab.rank(D30);
        let back = lab.proposal_cards(KEY_ENGL);
        assert_eq!((back.len(), back.iter().filter(|c| c.status() == "pending").count()), (2, 1), "{how}: {back:?}");
    }
}

/// B3e (convergence round 5, "A `rank` does not refile a dropped elective's card"). The unmatched
/// course's card X is pending. A rank's `refresh_series` returns a file holding X; a fetch then
/// reads X's row in part (X moves to `ended`, card X is stamped `superseded`) and the rank runs on:
/// it files no card for X, creates no note, and after it no card for X is pending.
#[test]
fn a_rank_does_not_refile_a_dropped_electives_card() {
    for how in ["status", "partial", "unread"] {
        let lab = Lab::new(&format!("kept-race5-{how}"), "2026-08-31");
        lab.fetch_registered(D1, &fx(), &REGISTERED);
        lab.rank(D2);
        let fetched = lab.clone_as(&format!("kept-race5-{how}-fetched"), "2026-08-31");
        let mut later = fx();
        read_in_part(&mut later, "40003", how);
        fetched.fetch_registered(D10, &later, &REGISTERED);
        assert_eq!(fetched.proposal_cards(KEY_ENGL)[0].status(), "superseded", "{how}: the staged fetch withdrew nothing");
        rank_interleaved(&lab, D10, || lab.adopt(&fetched));
        let cards = lab.proposal_cards(KEY_ENGL);
        assert!(cards.len() == 1 && cards[0].status() == "superseded", "{how}: {cards:?}");
        assert!(!lab.path("commitments/engl-101.md").exists());
    }
}

/// The same race's two controls, each with no card for X yet: with no fetch alongside the rank
/// files X's card as it does today; with only the re-read malformed (staged by the test) it files
/// no `registrar:` proposal card and no kept-field card.
#[test]
fn the_race_controls_file_as_today_and_file_nothing_on_a_malformed_reread() {
    let plain = Lab::new("kept-race5-plain", "2026-08-31");
    plain.fetch_registered(D1, &fx(), &REGISTERED);
    plain.rank(D2);
    assert_eq!(plain.proposal_cards(KEY_ENGL).len(), 1, "no fetch alongside: the card is filed");

    let lab = edited("kept-race5-bad");
    lab.spend_the_cap(D10);
    assert_eq!(kept_of(&lab.fetch(D10, &at_13()), LECTURE, "meets")[0]["card"], "waiting");
    rank_interleaved(&lab, D11, || std::fs::write(lab.path("state/calendar-series.json"), "{ not json").unwrap());
    assert!(lab.proposal_cards(KEY_ENGL).is_empty() && lab.proposal_cards("registrar:ua:202640-40006").is_empty(), "{:?}", lab.all_cards());
    assert!(lab.change_cards().is_empty(), "{:?}", lab.change_cards());
}

/// A fresh vault with the unmatched course's card X pending (it was proposed whole, then filed).
fn with_card_x(name: &str) -> (Lab, Card) {
    let lab = Lab::new(name, "2026-08-31");
    lab.fetch_registered(D1, &fx(), &REGISTERED);
    lab.rank(D2);
    let card = lab.proposal_cards(KEY_ENGL).remove(0);
    (lab, card)
}

fn no_create_for_x(lab: &Lab) {
    assert!(!lab.path("commitments/engl-101.md").exists(), "a note was created");
    assert!(lab.journal().iter().all(|r| !(r["op"] == "create" && r["path"] == "commitments/engl-101.md")), "a create record");
}

/// What a fetch that reads X's row under another status would merge: the fall series, X gone.
fn series_without_x() -> Vec<crate::commitments::Series> {
    let mut answer = fx();
    read_in_part(&mut answer, "40003", "status");
    super::parse_banner_with(&answer, "ua", &REGISTERED).unwrap().series
}

/// B3e (convergence round 5, "Approving a dropped section's card writes nothing"). Card X is
/// pending; a fetch's series-file write moves X to `ended`; the card is settled before that fetch's
/// withdrawal pass: `Superseded`, no note, no `create` record. The same when the file on disk holds
/// X's key under no calendar but knows another, and when a later `rank`'s `process_approvals`
/// first settles the approved card. A fetch that reads the row whole again leads the next `rank` to
/// file one new card for X.
#[test]
fn approving_a_dropped_sections_card_writes_nothing() {
    let (lab, card) = with_card_x("settle-ended");
    crate::commitments::refresh_series(&lab.v, &[(CAL.to_string(), series_without_x())], D10);
    assert!(matches!(lab.settle(&card, D10), Settled::Superseded(_)), "X is in `ended`");
    no_create_for_x(&lab);

    let (lab, card) = with_card_x("settle-gone");
    lab.forget_series(KEY_ENGL);
    assert!(matches!(lab.settle(&card, D10), Settled::Superseded(_)), "the file knows a calendar and holds no X");
    no_create_for_x(&lab);

    let (lab, card) = with_card_x("settle-rank");
    lab.set_status_of("approvals", &card.name, "approved");
    crate::commitments::refresh_series(&lab.v, &[(CAL.to_string(), series_without_x())], D10);
    lab.rank(D11);
    let after = lab.proposal_cards(KEY_ENGL);
    assert_eq!((after.len(), after[0].folder, after[0].status()), (1, "archive", "superseded"), "{after:?}");
    no_create_for_x(&lab);
    lab.fetch_registered(D20, &fx(), &REGISTERED);
    lab.rank(D30);
    let back = lab.proposal_cards(KEY_ENGL);
    assert_eq!((back.len(), back.iter().filter(|c| c.status() == "pending").count()), (2, 1), "{back:?}");
}

/// The same test's three controls (a guard that should already be green): it never stamps a
/// `registrar:` card whose key is under `series` on disk, a `registrar:` card over a series file
/// that reads as empty, or a `gcal-series:` card whose key left its calendar.
#[test]
fn the_settlement_test_never_stamps_a_card_it_should_not() {
    let (lab, card) = with_card_x("settle-control-held");
    assert_eq!(lab.settle(&card, D10), Settled::Executed, "X is under series on disk");
    assert!(lab.path("commitments/engl-101.md").exists());

    let (lab, card) = with_card_x("settle-control-empty");
    std::fs::remove_file(lab.path("state/calendar-series.json")).unwrap();
    assert_eq!(lab.settle(&card, D10), Settled::Executed, "a file that reads as empty says nothing");

    let lab = Lab::new("settle-control-gcal", "2026-08-31");
    lab.rank_with(D7, vec![("cloud:google", google_entry(vec![google_item("cs100aa", "CS 100", &[0, 2, 4], "12:00", "12:50", 8)]))]);
    let card = lab.proposal_cards("gcal-series:cs100aa").remove(0);
    lab.forget_series("gcal-series:cs100aa");
    assert_eq!(lab.settle(&card, D8), Settled::Executed, "a gcal-series: card is not tested");
}

/// B3e (A2 item 13, "After a restart", the `kept` part; item 20, round 1). The overview a new
/// process reads lists the kept entry with the card shown as filed, read from the block and
/// `approvals/`, and the existing `registrar` object is untouched.
#[test]
fn the_list_survives_a_restart_its_kept_part() {
    let lab = edited("kept-restart");
    let report = lab.fetch(D10, &at_13());
    let overview = crate::commitments::overview(&lab.v, D10).to_json();
    let listed = kept_of(&overview["registrar_changes"], LECTURE, "meets");
    assert_eq!(listed, kept_of(&report, LECTURE, "meets"), "{overview}");
    assert_eq!((listed[0]["registrar"].clone(), listed[0]["student"].clone(), listed[0]["card"].clone()), (at_13_json(), student_meets(), json!("filed")));
    let before = lab.series_text();
    lab.rank(D11);
    assert_eq!(lab.series_text(), before, "a rank must not edit the block");
}

/// B3e (A2 item 14, idempotence, read with 12; item 20, round 1). A second identical fetch, on the
/// same day and on a later day, adds no `applied` or `ended` entry, files no second card, withdraws
/// none, still lists the standing `kept` entry, and changes only `last_fetch`.
#[test]
fn an_identical_refetch_keeps_the_standing_entries_and_files_nothing_new() {
    let lab = edited("kept-idem");
    lab.fetch(D10, &at_13());
    let first = lab.term();
    let same_day = lab.fetch(D10, &at_13());
    assert_eq!(kept_of(&same_day, LECTURE, "meets").len(), 1, "{same_day}");
    assert_eq!(lab.term(), first, "the same day changed the block");
    let later = lab.fetch(D20, &at_13());
    assert_eq!(kept_of(&later, LECTURE, "meets")[0]["card"], "filed", "{later}");
    let mut after = lab.term();
    assert_eq!(after["last_fetch"], "2026-09-20");
    after["last_fetch"] = first["last_fetch"].clone();
    assert_eq!(after, first, "a later fetch changed more than last_fetch");
    assert_eq!(lab.change_cards().len(), 1, "{:?}", lab.change_cards());
    assert_eq!(lab.change_cards()[0].status(), "pending", "the card was withdrawn");
}

/// B3e (A2 item 20, round 2, "Ended once, across days", the hand-set variant). A hand-set `until`
/// with the course missing on day 1 and again on day 20: one kept entry whose registrar value is
/// the day before day 1, and exactly one card over both fetches and the `rank`s between them.
#[test]
fn a_hand_set_end_is_asked_once_across_days() {
    let lab = Lab::new("kept-until-once", "2026-08-31");
    lab.fetch(D1, &fx());
    lab.hand_set(LAB, "until", "2026-11-20");
    lab.fetch(D10, &without_lab());
    lab.rank(D10);
    lab.fetch(D20, &without_lab());
    lab.rank(D20);
    let kept = lab.kept_entries();
    assert_eq!((kept.len(), kept[0]["registrar"].as_str()), (1, Some("2026-09-09")), "{kept:?}");
    assert_eq!(lab.ended_entries().len(), 1);
    assert_eq!(cards_of(&lab, LAB, "until").len(), 1, "{:?}", lab.change_cards());
}
