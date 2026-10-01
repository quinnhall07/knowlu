//! The vault-writing event state outside `approvals.rs`: the ever-written set, the series carry,
//! D9's rebuild and `answered_series`.
//!
//! Spec `2026-09-29-events-design.md` §4.5 and §6.1. [`answered_series`] is the one definition of
//! an answered `event-accept` series. The emitter's never-ask-twice, the carry and `judge_roster`
//! all call it and none restates it, so one `rank` cannot both carry a series and ask about it.

use std::collections::BTreeMap;
use std::path::Path;

use serde_yaml_ng::Mapping;

use crate::models::split_frontmatter;
use crate::{ids, pystr, yaml};

/// The card kind whose answers [`answered_series`] reads.
const EVENT_ACCEPT: &str = "event-accept";

/// One series an archived `event-accept` card answered.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SeriesAccept {
    /// `executed` answers accept (`true`); `rejected` answers decline (`false`).
    pub accepted: bool,
    /// The answering card's file name in `archive/`.
    pub file: String,
    /// The answering card's `id:`, or empty when it has none or [`ids::is_id`] refuses it. The
    /// carry's ledger line names the card by it (the plan's P15, `from:`).
    pub id: String,
}

/// Every series an `event-accept` card answered, keyed by `series_uid` (spec §6.1).
///
/// The edge rules follow `eventemit::settled_series`:
/// - it reads `archive/` only, through `approvals::sorted_md`, and only `type: approval`,
///   `kind: event-accept` cards: an `event-check` card is not counted (accepted `event-check`
///   series reach the carry through their own reader, never through this one);
/// - `executed` answers accept and `rejected` answers decline; `expired` or any other status
///   answers nothing (ruling G1), so it shadows no other card;
/// - a card with a missing or empty `series_uid` answers no series; the uid rule (§4.1) closes
///   that card's own instances;
/// - if two cards answer one series, the lowest file name wins.
///
/// It reads no ledger. An unreadable card answers nothing. The `id:` is read as `ids::build_index`
/// reads it (unstripped, then [`ids::is_id`]), so the carry never names a card by an id the index
/// would refuse.
pub fn answered_series(vault: &Path) -> BTreeMap<String, SeriesAccept> {
    let mut answered: BTreeMap<String, SeriesAccept> = BTreeMap::new();
    for path in crate::approvals::sorted_md(&vault.join("archive")) {
        let Ok(text) = pystr::read_text(&path) else { continue };
        let Ok((meta, _)) = split_frontmatter(&text) else { continue };
        if card_text(&meta, "type") != "approval" || card_text(&meta, "kind") != EVENT_ACCEPT {
            continue;
        }
        let accepted = match card_text(&meta, "status").as_str() {
            "executed" => true,
            "rejected" => false,
            _ => continue,
        };
        let series = card_text(&meta, "series_uid");
        if series.is_empty() || answered.contains_key(&series) {
            continue;
        }
        let file = path.file_name().map(|n| n.to_string_lossy().into_owned()).unwrap_or_default();
        let id = yaml::opt_text(yaml::get(&meta, "id")).filter(|id| ids::is_id(id)).unwrap_or_default();
        answered.insert(series, SeriesAccept { accepted, file, id });
    }
    answered
}

/// A card field as text, stripped; empty when absent, null or not a scalar.
fn card_text(meta: &Mapping, key: &str) -> String {
    let raw = yaml::opt_text(yaml::get(meta, key)).unwrap_or_default();
    pystr::strip(&raw).to_string()
}

#[cfg(test)]
mod tests {
    //! T2b.1: `answered_series`' edge rules (spec §6.1), each with an answering card beside it so
    //! a reader that answers nothing fails them.

    use super::*;
    use std::fs;
    use std::path::PathBuf;

    const ID_A: &str = "appr_0123456789";
    const ID_B: &str = "appr_abcdef0123";

    fn tmp(name: &str) -> PathBuf {
        let dir = std::env::temp_dir().join(format!("qo-carry-{}-{}", name, std::process::id()));
        let _ = fs::remove_dir_all(&dir);
        fs::create_dir_all(&dir).unwrap();
        dir
    }

    /// A card written by hand into `folder`. `extra` is whole frontmatter lines, each ending `\n`.
    fn card(vault: &Path, folder: &str, name: &str, kind: &str, status: &str, extra: &str) {
        fs::create_dir_all(vault.join(folder)).unwrap();
        let text = format!(
            "---\n{extra}type: approval\nkind: {kind}\ntitle: \"Required · Weekly meeting · Tue 22 Sep 7–9pm\"\n\
             status: {status}\nverdict: obligation\nsource_uid: \"lx:77:1\"\nevents:\n- \"lx:77:1\"\n\
             proposed_at: 2026-09-20\nfirst_proposed_at: 2026-09-20\nexpires: 2026-09-22\n\
             snooze_until: null\ncreated_by: events\n---\n\nbody\n"
        );
        pystr::write_text(&vault.join(folder).join(name), &text).unwrap();
    }

    /// An `event-accept` card in `archive/` answering `series`; `id` is the YAML text of `id:`.
    fn accept(vault: &Path, name: &str, status: &str, series: &str, id: Option<&str>) {
        let id_line = id.map(|i| format!("id: {i}\n")).unwrap_or_default();
        card(vault, "archive", name, EVENT_ACCEPT, status, &format!("{id_line}series_uid: \"{series}\"\n"));
    }

    fn keys(answered: &BTreeMap<String, SeriesAccept>) -> Vec<&str> {
        answered.keys().map(String::as_str).collect()
    }

    #[test]
    fn an_executed_card_answers_accept_and_a_rejected_one_decline() {
        let vault = tmp("answers");
        accept(&vault, "event-weekly-meeting-2026-09-22.md", "executed", "lx:77", Some(ID_A));
        accept(&vault, "event-film-night-2026-09-23.md", "rejected", "lx:78", Some(ID_B));
        accept(&vault, "event-no-id-2026-09-24.md", "executed", "lx:79", None);
        accept(&vault, "event-bad-id-2026-09-25.md", "rejected", "lx:80", Some("not-an-id"));
        // Read as `ids::build_index` reads an id, unstripped: this one names no note.
        accept(&vault, "event-spaced-id-2026-09-26.md", "executed", "lx:81", Some("\" appr_0123456789\""));

        let answered = answered_series(&vault);
        assert_eq!(keys(&answered), ["lx:77", "lx:78", "lx:79", "lx:80", "lx:81"]);
        let weekly = &answered["lx:77"];
        assert!(weekly.accepted, "executed answers accept");
        assert_eq!(weekly.file, "event-weekly-meeting-2026-09-22.md");
        assert_eq!(weekly.id, ID_A);
        let film = &answered["lx:78"];
        assert!(!film.accepted, "rejected answers decline");
        assert_eq!(film.file, "event-film-night-2026-09-23.md");
        assert_eq!(film.id, ID_B);
        for (series, accepted) in [("lx:79", true), ("lx:80", false), ("lx:81", true)] {
            assert_eq!(answered[series].accepted, accepted, "{series}");
            assert_eq!(answered[series].id, "", "{series}: no id, or one ids::is_id refuses");
        }
    }

    #[test]
    fn an_expired_or_other_status_answers_nothing() {
        let vault = tmp("statuses");
        for (name, status, series) in [
            ("event-a-expired.md", "expired", "lx:1"),
            // A `pending` card moved into `archive/` by hand was never answered.
            ("event-b-pending.md", "pending", "lx:2"),
            ("event-c-approved.md", "approved", "lx:3"),
            ("event-d-failed.md", "failed", "lx:4"),
            ("event-e-empty.md", "\"\"", "lx:5"),
            ("event-f-null.md", "null", "lx:6"),
        ] {
            accept(&vault, name, status, series, Some(ID_A));
        }
        // Ruling G1: an expired card answers nothing, so it does not shadow a later answer.
        accept(&vault, "event-g-expired.md", "expired", "lx:9", Some(ID_A));
        accept(&vault, "event-h-executed.md", "executed", "lx:9", Some(ID_B));
        let broken = "---\ntype: approval\nkind: [event-accept\nstatus: executed\n---\n";
        pystr::write_text(&vault.join("archive").join("event-i-unreadable.md"), broken).unwrap();

        let answered = answered_series(&vault);
        assert_eq!(keys(&answered), ["lx:9"]);
        assert!(answered["lx:9"].accepted);
        assert_eq!(answered["lx:9"].file, "event-h-executed.md");
        assert_eq!(answered["lx:9"].id, ID_B);
    }

    #[test]
    fn a_card_with_a_missing_or_empty_series_uid_answers_no_series() {
        let vault = tmp("no-series");
        card(&vault, "archive", "event-a-missing.md", EVENT_ACCEPT, "executed", &format!("id: {ID_A}\n"));
        let empties = [("event-b-empty.md", "\"\""), ("event-c-blank.md", "\"   \""), ("event-d-null.md", "null")];
        for (name, series) in empties {
            let extra = format!("id: {ID_A}\nseries_uid: {series}\n");
            card(&vault, "archive", name, EVENT_ACCEPT, "rejected", &extra);
        }
        accept(&vault, "event-e.md", "executed", "lx:9", Some(ID_B));

        // No series, and no fallback to the card's `source_uid` (`lx:77:1`): the uid rule (§4.1)
        // closes that card's own instances, not this reader.
        let answered = answered_series(&vault);
        assert_eq!(keys(&answered), ["lx:9"]);
        assert_eq!(answered["lx:9"].file, "event-e.md");
    }

    #[test]
    fn an_event_check_card_is_not_counted() {
        let vault = tmp("event-check");
        // PQ1 (a) leaves this rule unchanged: an executed `event-check` card, even one carrying
        // `instances:`, reaches the carry only through `accepted_check_series` (T2b.2b).
        let instances = "instances:\n- uid: \"lx:1:1\"\n  title: \"Weekly meeting\"\n  \
                         start: \"2026-09-22T19:00\"\n  end: \"2026-09-22T21:00\"\n";
        let carried = format!("series_uid: \"lx:1\"\n{instances}");
        card(&vault, "archive", "event-check-a.md", "event-check", "executed", &carried);
        card(&vault, "archive", "event-check-b.md", "event-check", "rejected", "series_uid: \"lx:2\"\n");
        // A note that is not an approval is no card, whatever its `kind:` says.
        let task = "---\ntype: task\nkind: event-accept\nstatus: executed\nseries_uid: \"lx:3\"\n---\n";
        pystr::write_text(&vault.join("archive").join("event-c-task.md"), task).unwrap();
        accept(&vault, "event-d.md", "executed", "lx:9", Some(ID_A));

        assert_eq!(keys(&answered_series(&vault)), ["lx:9"]);
    }

    #[test]
    fn two_cards_for_one_series_the_lowest_file_name_wins() {
        let vault = tmp("two-cards");
        // Written highest first, so the directory's own order cannot decide.
        accept(&vault, "event-weekly-b.md", "executed", "lx:77", Some(ID_B));
        accept(&vault, "event-weekly-a.md", "rejected", "lx:77", Some(ID_A));
        accept(&vault, "event-film-b.md", "rejected", "lx:78", Some(ID_B));
        accept(&vault, "event-film-a.md", "executed", "lx:78", Some(ID_A));
        // Lowest by bytes: a collision's `-2.md` sorts before `.md`.
        accept(&vault, "event-talk-2026-10-06.md", "executed", "lx:79", Some(ID_A));
        accept(&vault, "event-talk-2026-10-06-2.md", "rejected", "lx:79", Some(ID_B));

        let answered = answered_series(&vault);
        let pick = |series: &str| {
            let a = &answered[series];
            (a.accepted, a.file.clone(), a.id.clone())
        };
        assert_eq!(pick("lx:77"), (false, "event-weekly-a.md".to_string(), ID_A.to_string()));
        assert_eq!(pick("lx:78"), (true, "event-film-a.md".to_string(), ID_A.to_string()));
        assert_eq!(pick("lx:79"), (false, "event-talk-2026-10-06-2.md".to_string(), ID_B.to_string()));
    }

    #[test]
    fn only_archive_is_read() {
        let vault = tmp("archive-only");
        assert!(answered_series(&vault).is_empty(), "a vault with no archive/ answers nothing");
        for (name, status, series) in [("event-a.md", "executed", "lx:1"), ("event-b.md", "rejected", "lx:2")] {
            let extra = format!("id: {ID_A}\nseries_uid: \"{series}\"\n");
            card(&vault, "approvals", name, EVENT_ACCEPT, status, &extra);
        }
        assert!(answered_series(&vault).is_empty(), "a card in approvals/ answers nothing");

        accept(&vault, "event-c.md", "executed", "lx:9", Some(ID_B));
        assert_eq!(keys(&answered_series(&vault)), ["lx:9"]);
    }
}
