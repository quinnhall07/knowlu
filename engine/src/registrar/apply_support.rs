//! Close-out task B2: the shared fixtures and helpers of the apply-and-list tests
//! (`seen_tests`, `apply_tests`, `kept_tests`, `block_tests`). Test code only; every course, room,
//! name and number is invented. Nothing here is implementation.
//!
//! **What B2 pins that the spec leaves open** (a B3 implementer follows these, or says so in the
//! ledger and the test changes with the controller's say):
//! - the command's JSON lists (`applied`, `ended`, `kept`, `status`, `unread`) and the registrar
//!   block hold entries with these keys: `applied` {day, field, from, note, to}; `ended` {day,
//!   kept_yours, meets?, note, returned?, until, where?} (`meets` and `where`: the dropped row's
//!   last whole values, present when the file held its row; B3eR fix 2); `kept` {field, note,
//!   registrar, student} (the report's copy adds `card`: "filed", "waiting" or "listed");
//!   `status` {key, note, status}; `unread` {key, note, reason}. `note` is the vault-relative
//!   path (`null` when no note holds the key); dates are `YYYY-MM-DD`; a `meets` value is the note's own list (`[{days, start, end}]`).
//! - `commitments --json`'s overview carries the same five lists, read from the block, under the
//!   new key `registrar_changes` (its existing `registrar` object is untouched).
#![allow(dead_code)]

use std::cell::RefCell;

use std::path::{Path, PathBuf};

use jiff::civil::Date;
use serde_json::Value;

use crate::cli::{run_with, Fetchers, SeriesStash, StashEntry};
use crate::journal::Journal;
use crate::write::{WriteContext, WriteOpts};

pub(super) use super::tests::FIXTURE;
pub(super) const D1: Date = Date::constant(2026, 9, 1);
pub(super) const D2: Date = Date::constant(2026, 9, 2);
pub(super) const D10: Date = Date::constant(2026, 9, 10);
pub(super) const D11: Date = Date::constant(2026, 9, 11);
pub(super) const D20: Date = Date::constant(2026, 9, 20);
pub(super) const D30: Date = Date::constant(2026, 9, 30);
pub(super) const D40: Date = Date::constant(2026, 10, 10);
pub(super) const D50: Date = Date::constant(2026, 10, 20);
pub(super) const CAL: &str = "registrar:ua:202640";
pub(super) const LECTURE: &str = "commitments/cs-100.md";
pub(super) const LAB: &str = "commitments/cs-100-lab.md";
pub(super) const REGISTERED: [&str; 1] = ["IR"];

const FEEDS: &str = "timezone: America/Chicago\ncalendars:\n  - name: google\n    ics_url: 'cloud:google'\n  - name: personal\n    ics_url: https://calendar.example.test/personal.ics\n";
const TEMPLATE: &str = "day_start: '08:00'\nday_end: '18:00'\nclasses:\n  mon: []\n  tue: []\n  wed: []\n  thu: []\n  fri: []\n  sat: []\n  sun: []\n";

// ---- the invented answer, and the edits the tests make to it ------------------------------------

pub(super) fn fx() -> Value {
    serde_json::from_str(FIXTURE).unwrap()
}

pub(super) fn row<'a>(v: &'a mut Value, crn: &str) -> &'a mut Value {
    v["data"]["registrations"].as_array_mut().unwrap().iter_mut().find(|r| r["courseReferenceNumber"] == crn).unwrap_or_else(|| panic!("no row {crn}"))
}

/// The row's first meeting (a `CLAS` one in every fixture row that has one).
pub(super) fn meeting<'a>(v: &'a mut Value, crn: &str) -> &'a mut Value {
    &mut row(v, crn)["meetingTimes"][0]
}

pub(super) fn retime(v: &mut Value, crn: &str, begin: &str, end: &str) {
    meeting(v, crn)["beginTime"] = Value::from(begin);
    meeting(v, crn)["endTime"] = Value::from(end);
}

pub(super) fn drop_crn(v: &mut Value, crn: &str) {
    v["data"]["registrations"].as_array_mut().unwrap().retain(|r| r["courseReferenceNumber"] != crn);
}

pub(super) fn set_status(v: &mut Value, crn: &str, code: &str, description: &str) {
    row(v, crn)["statusIndicator"] = Value::from(code);
    row(v, crn)["statusDescription"] = Value::from(description);
}

/// The row keeps its CRN and loses its times: P9 reads it `unread`.
pub(super) fn blank_times(v: &mut Value, crn: &str) {
    meeting(v, crn)["beginTime"] = Value::Null;
    meeting(v, crn)["endTime"] = Value::Null;
}

/// A second `CLAS` meeting with no times beside the usable one: P9 reads the row `partial`.
pub(super) fn add_timeless_meeting(v: &mut Value, crn: &str) {
    let mut extra = meeting(v, crn).clone();
    extra["beginTime"] = Value::Null;
    extra["endTime"] = Value::Null;
    row(v, crn)["meetingTimes"].as_array_mut().unwrap().push(extra);
}

/// The same answer for term 202710, with new CRNs (a course taken again in spring).
pub(super) fn spring() -> Value {
    let text = FIXTURE
        .replace("202640", "202710").replace("Fall 2026", "Spring 2027")
        .replace("08/19/2026", "01/12/2027").replace("12/04/2026", "05/01/2027")
        .replace("\"4000", "\"5000");
    serde_json::from_str(&text).unwrap()
}

pub(super) fn list(report: &Value, key: &str) -> Vec<Value> {
    report[key].as_array().unwrap_or_else(|| panic!("the report has no `{key}` list: {report}")).clone()
}

pub(super) fn of_note(entries: &[Value], note: &str) -> Vec<Value> {
    entries.iter().filter(|e| e["note"] == note).cloned().collect()
}

// ---- a scratch vault ---------------------------------------------------------------------------

/// A new vault (`config/actor.yaml` says `student`) whose only course is CS 100, with `rank`'s own
/// inputs, so `rank` and a fetch run on it. Removed when dropped, a failed test included.
pub(super) struct Lab {
    pub v: PathBuf,
}

impl Drop for Lab {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.v);
    }
}

/// One journal record stamped noon UTC on `day`: the vault's first day for `vault_day`.
fn seed_day(vault: &Path, day: &str) {
    let mut spec = crate::journal::NewRecord::new("create", "archive/_migrated.md", "system:migration", "cli");
    spec.ts = Some(format!("{day}T12:00:00.000Z"));
    let mut rec = crate::journal::make_record(spec).unwrap();
    Journal::new(vault).append(&mut rec).unwrap();
}

impl Lab {
    /// `first_day` is the vault's day 1 (`2026-08-31` makes `D1` its day 2).
    pub fn new(name: &str, first_day: &str) -> Lab {
        let v = std::env::temp_dir().join(format!("knowlu-b2-{}-{name}", std::process::id()));
        let _ = std::fs::remove_dir_all(&v);
        for dir in ["config", "tasks", "courses", "archive"] {
            std::fs::create_dir_all(v.join(dir)).unwrap();
        }
        let put = |rel: &str, text: &str| crate::pystr::write_text(&v.join(rel), text).unwrap();
        put("config/actor.yaml", "human_actor: student\n");
        put("config/week_template.yaml", TEMPLATE);
        put("config/ingest.yaml", FEEDS);
        put("tasks/essay.md", "---\ntitle: Invented essay\ndue: 2026-12-30\neffort_hours: 2\nimportance: 3\n---\n");
        put("courses/cs-100.md", "---\nid: course_00000000c1\ntitle: \"CS 100 Intro to Computing\"\ncode: \"CS 100\"\n---\n");
        let migration = WriteContext::new("system:migration", "cli");
        let note = "---\ntitle: migrated\nstatus: archived\n---\n";
        crate::write::create(&v, "archive/_migrated.md", note, &migration, &mut Journal::new(&v), None).unwrap();
        seed_day(&v, first_day);
        Lab { v }
    }

    /// ENGL 101 and ART 110 become courses of the vault, so those rows match (R24) and get notes
    /// (`commitments/engl-101.md`, `commitments/art-110.md`) beside CS 100's two.
    pub fn add_courses(&self) {
        let put = |rel: &str, text: &str| crate::pystr::write_text(&self.path(rel), text).unwrap();
        put("courses/engl-101.md", "---\nid: course_00000000c2\ntitle: \"ENGL 101 Invented Writing\"\ncode: \"ENGL 101\"\n---\n");
        put("courses/art-110.md", "---\nid: course_00000000c3\ntitle: \"ART 110 Invented Drawing\"\ncode: \"ART 110\"\n---\n");
    }

    /// The vault's human token as the engine binary resolves it, through `dashboard`.
    pub fn ctx(&self) -> WriteContext {
        WriteContext::new(crate::journal::read_human_actor(&self.v).unwrap(), "dashboard")
    }

    pub fn try_fetch(&self, day: Date, answer: &Value) -> Result<Value, String> {
        super::run(&self.v, &answer.to_string(), "ua", day, &self.ctx(), &mut Journal::new(&self.v)).map(|r| r.to_json())
    }

    /// `commitments --registrar` on `day`; the report as the command prints it.
    pub fn fetch(&self, day: Date, answer: &Value) -> Value {
        self.try_fetch(day, answer).unwrap_or_else(|e| panic!("the fetch failed: {e}"))
    }

    /// The same, parsed under an invented registered set (P6).
    pub fn fetch_registered(&self, day: Date, answer: &Value, registered: &[&str]) -> Value {
        let parsed = super::parse_banner_with(answer, "ua", registered).unwrap();
        let report = super::run_parsed(&self.v, parsed, "ua", day, &self.ctx(), &mut Journal::new(&self.v));
        report.unwrap_or_else(|e| panic!("the fetch failed: {e}")).to_json()
    }

    pub fn rank(&self, day: Date) {
        self.rank_with(day, Vec::new());
    }

    /// `rank` on `day` with `entries` injected into the stash (no network).
    pub fn rank_with(&self, day: Date, entries: Vec<(&str, StashEntry)>) {
        let stash: SeriesStash = RefCell::new(entries.into_iter().map(|(url, e)| (url.to_string(), e)).collect());
        let empty = |_: &str| Ok("BEGIN:VCALENDAR\nEND:VCALENDAR\n".to_string());
        let fetchers = Fetchers { calendar: Some(&empty), events: None, series: Some(&stash) };
        run_with(&self.v, Some(&day.to_string()), "manual", None, fetchers).unwrap();
    }

    /// The student changes `field` of `rel` by hand: a `set` record by the human token.
    pub fn hand_set(&self, rel: &str, field: &str, literal: &str) {
        let changes = [(field.to_string(), literal.to_string())];
        let mut journal = Journal::new(&self.v);
        crate::write::write_literals(&self.v, rel, &changes, &self.ctx(), &mut journal, &WriteOpts::default()).unwrap();
    }

    /// A card the student settles: `status` goes to `approved`, the way the app writes it.
    pub fn set_status_of(&self, folder: &str, name: &str, status: &str) {
        let rel = format!("{folder}/{name}");
        let changes = [("status".to_string(), status.to_string())];
        let mut journal = Journal::new(&self.v);
        crate::write::write_literals(&self.v, &rel, &changes, &self.ctx(), &mut journal, &WriteOpts::default()).unwrap();
    }
}

// ---- reading the vault back --------------------------------------------------------------------

/// A `commitment-check` change card (one with a `target`), wherever it sits.
#[derive(Debug, Clone)]
pub(super) struct Card {
    pub folder: &'static str,
    pub name: String,
    pub meta: Value,
}

impl Card {
    pub fn status(&self) -> &str {
        self.meta["status"].as_str().unwrap_or("")
    }
}

fn meta_json(path: &Path) -> Option<Value> {
    let meta = crate::ids::read_meta(path)?;
    Some(crate::yaml::to_json(&serde_yaml_ng::Value::Mapping(meta)))
}

impl Lab {
    pub fn path(&self, rel: &str) -> PathBuf {
        self.v.join(rel)
    }

    pub fn note(&self, rel: &str) -> String {
        std::fs::read_to_string(self.path(rel)).unwrap_or_else(|e| panic!("{rel}: {e}"))
    }

    /// The text after `key:` on the note's frontmatter line, or `None`.
    pub fn line(&self, rel: &str, key: &str) -> Option<String> {
        let prefix = format!("{key}: ");
        self.note(rel).lines().find_map(|l| l.strip_prefix(prefix.as_str()).map(|r| r.trim().to_string()))
    }

    /// Every file under the vault with its bytes, sorted: the "nothing written" oracle.
    pub fn snapshot(&self) -> Vec<(PathBuf, Vec<u8>)> {
        fn walk(dir: &Path, out: &mut Vec<(PathBuf, Vec<u8>)>) {
            for entry in std::fs::read_dir(dir).into_iter().flatten().flatten() {
                let path = entry.path();
                match path.is_dir() {
                    true => walk(&path, out),
                    false => out.push((path.clone(), std::fs::read(&path).unwrap())),
                }
            }
        }
        let mut out = Vec::new();
        walk(&self.v, &mut out);
        out.retain(|(p, _)| p.file_name().is_none_or(|n| n != "calendar-series.lock"));
        out.sort();
        out
    }

    pub fn series_text(&self) -> String {
        std::fs::read_to_string(self.path("state/calendar-series.json")).unwrap_or_default()
    }

    pub fn series_file(&self) -> Value {
        serde_json::from_str(&self.series_text()).unwrap_or(Value::Null)
    }

    /// The whole `registrar` block (`null` when the file has none).
    pub fn block(&self) -> Value {
        self.series_file()["registrar"].clone()
    }

    /// One term's block entry.
    pub fn term(&self) -> Value {
        self.block()[CAL].clone()
    }

    /// The `source_uid`s the file holds under `series`.
    pub fn series_keys(&self) -> Vec<String> {
        let file = self.series_file();
        file["series"].as_array().map(|a| a.iter().filter_map(|s| s["source_uid"].as_str().map(String::from)).collect()).unwrap_or_default()
    }

    pub fn ended_keys(&self) -> Vec<String> {
        self.series_file()["ended"].as_object().map(|o| o.keys().cloned().collect()).unwrap_or_default()
    }

    /// Every journal record, in file order.
    pub fn journal(&self) -> Vec<Value> {
        let Ok(dir) = std::fs::read_dir(self.path("state/journal")) else { return Vec::new() };
        let mut files: Vec<PathBuf> = dir.flatten().map(|e| e.path()).collect();
        files.sort();
        files.iter().flat_map(|f| std::fs::read_to_string(f).unwrap().lines().map(|l| serde_json::from_str(l).unwrap()).collect::<Vec<Value>>()).collect()
    }

    /// The `set` records for one note and field, as `(actor, via, new)`.
    pub fn sets(&self, rel: &str, field: &str) -> Vec<(String, String, Value)> {
        self.journal().into_iter()
            .filter(|r| r["op"] == "set" && r["path"] == rel && r["field"] == field)
            .map(|r| (r["actor"].as_str().unwrap().to_string(), r["via"].as_str().unwrap().to_string(), r["new"].clone()))
            .collect()
    }

    /// Every `commitment-check` card, `approvals/` then `archive/` (the planted fillers excepted).
    pub fn all_cards(&self) -> Vec<Card> {
        let mut out = Vec::new();
        for folder in ["approvals", "archive"] {
            let Ok(dir) = std::fs::read_dir(self.path(folder)) else { continue };
            let mut paths: Vec<PathBuf> = dir.flatten().map(|e| e.path()).collect();
            paths.sort();
            for path in paths {
                let name = path.file_name().unwrap().to_string_lossy().to_string();
                let Some(meta) = meta_json(&path) else { continue };
                if meta["kind"] == "commitment-check" && !name.contains("filler") {
                    out.push(Card { folder: if folder == "approvals" { "approvals" } else { "archive" }, name, meta });
                }
            }
        }
        out
    }

    /// The cards that carry a `target`: change cards.
    pub fn change_cards(&self) -> Vec<Card> {
        self.all_cards().into_iter().filter(|c| c.meta.get("target").is_some_and(|t| !t.is_null())).collect()
    }

    /// The proposal cards for one `source_uid`.
    pub fn proposal_cards(&self, key: &str) -> Vec<Card> {
        self.all_cards().into_iter().filter(|c| c.meta["source_uid"] == key && c.meta.get("target").is_none()).collect()
    }

    /// The change cards a student still has to answer for `target` and `field`.
    pub fn open_cards(&self, target: &str, field: &str) -> Vec<Card> {
        self.change_cards().into_iter()
            .filter(|c| c.folder == "approvals" && matches!(c.status(), "pending" | "snoozed"))
            .filter(|c| c.meta["target"] == target && c.meta["change"].get(field).is_some())
            .collect()
    }

    /// Five `commitment-check` cards first proposed on `day`, settled, so the day's allowance is spent.
    pub fn spend_the_cap(&self, day: Date) {
        std::fs::create_dir_all(self.path("archive")).unwrap();
        for i in 0..5 {
            let text = format!("---\ntype: approval\nkind: commitment-check\ntitle: filler {i}\nstatus: rejected\nproposed_at: {day}\nfirst_proposed_at: {day}\nsource_uid: \"gcal-series:filler{i}\"\n---\n\nInvented.\n");
            crate::pystr::write_text(&self.path(&format!("archive/commitment-check-filler-{i}.md")), &text).unwrap();
        }
    }
}

// ---- Google and ICS feeds, and notes made by hand ----------------------------------------------

pub(super) const D7: Date = Date::constant(2026, 9, 7);
pub(super) const D8: Date = Date::constant(2026, 9, 8);
pub(super) const GOOGLE: &str = "google:0a1b2c3d4e5f6071";
pub(super) const ICS_URL: &str = "https://calendar.example.test/personal.ics";

/// One Google `series` item: weekly on `days` (0 = Monday) from `D7`, `start`-`end` (CDT).
pub(super) fn google_item(id: &str, title: &str, days: &[i64], start: &str, end: &str, weeks: i64) -> Value {
    let names = ["MO", "TU", "WE", "TH", "FR", "SA", "SU"];
    let mut instances = Vec::new();
    for week in 0..weeks {
        for d in days {
            let day = D7.checked_add(jiff::Span::new().days(week * 7 + d)).unwrap();
            instances.push(serde_json::json!({ "start": format!("{day}T{start}:00-05:00"), "end": format!("{day}T{end}:00-05:00") }));
        }
    }
    let byday: Vec<&str> = days.iter().map(|d| names[*d as usize]).collect();
    serde_json::json!({
        "calendar": GOOGLE, "id": id, "title": title, "location": "", "description": "", "event_type": "default",
        "first": instances[0]["start"].clone(), "recurrence": [format!("RRULE:FREQ=WEEKLY;BYDAY={}", byday.join(","))], "instances": instances,
    })
}

/// A fresh, complete read of the Google grant returning `items`.
pub(super) fn google_entry(items: Vec<Value>) -> StashEntry {
    StashEntry::Google(serde_json::json!({ "calendars_read": [GOOGLE], "items": items }))
}

impl Lab {
    /// A confirmed note as the confirm path would mint it: the file, and its `create` record so
    /// `rank` does not read it as a hand edit nobody journaled.
    pub fn plant_note(&self, file: &str, front: &str) {
        std::fs::create_dir_all(self.path("commitments")).unwrap();
        let text = format!("---\n{front}---\n\nInvented.\n");
        crate::pystr::write_text(&self.path(&format!("commitments/{file}")), &text).unwrap();
        let (meta, _) = crate::models::split_frontmatter(&text).unwrap();
        let id = crate::yaml::get(&meta, "id").and_then(crate::yaml::text).unwrap();
        let rel = format!("commitments/{file}");
        let mut spec = crate::journal::NewRecord::new("create", &rel, crate::commitments::CARD_ACTOR, "cli");
        spec.id = Some(&id);
        spec.new = crate::yaml::to_json(&serde_yaml_ng::Value::Mapping(meta));
        spec.ts = Some("2026-08-31T13:00:00.000Z".into());
        let mut rec = crate::journal::make_record(spec).unwrap();
        Journal::new(&self.v).append(&mut rec).unwrap();
    }

    /// The note at `rel`, as `commitments::load` reads it.
    pub fn loaded(&self, rel: &str) -> crate::commitments::Commitment {
        crate::commitments::load(&self.v).confirmed.into_iter()
            .find(|n| n.path.to_string_lossy().replace('\\', "/") == rel)
            .unwrap_or_else(|| panic!("{rel} is not a confirmed note"))
    }

    /// The notes under `commitments/` whose text holds `needle`.
    pub fn notes_with(&self, needle: &str) -> Vec<String> {
        let Ok(dir) = std::fs::read_dir(self.path("commitments")) else { return Vec::new() };
        let mut out: Vec<String> = dir.flatten()
            .filter(|e| std::fs::read_to_string(e.path()).is_ok_and(|t| t.contains(needle)))
            .map(|e| format!("commitments/{}", e.file_name().to_string_lossy()))
            .collect();
        out.sort();
        out
    }
}

/// Every meeting of every row loses its times: the answer parses to no usable row (exit 2).
pub(super) fn blank_all(v: &mut Value) {
    for r in v["data"]["registrations"].as_array_mut().unwrap() {
        for m in r["meetingTimes"].as_array_mut().unwrap() {
            m["beginTime"] = Value::Null;
            m["endTime"] = Value::Null;
        }
    }
}

/// The weekdays the row's first meeting is on (full names, as Banner has them).
pub(super) fn set_days(v: &mut Value, crn: &str, days: &[&str]) {
    for day in ["monday", "tuesday", "wednesday", "thursday", "friday", "saturday", "sunday"] {
        meeting(v, crn)[day] = Value::Bool(days.contains(&day));
    }
}

impl Lab {
    /// The series file as an older engine wrote it: no `registrar` block.
    pub fn drop_block(&self) {
        let mut file = self.series_file();
        file.as_object_mut().unwrap().remove("registrar");
        crate::pystr::write_text(&self.path("state/calendar-series.json"), &format!("{}\n", crate::ledger::dumps_value(&file))).unwrap();
    }

    /// The series file with one key gone from `series` (its calendars and `ended` left as they are).
    pub fn forget_series(&self, key: &str) {
        let mut file = self.series_file();
        file["series"].as_array_mut().unwrap().retain(|s| s["source_uid"] != key);
        crate::pystr::write_text(&self.path("state/calendar-series.json"), &format!("{}\n", crate::ledger::dumps_value(&file))).unwrap();
    }

    /// The term block's `ended` entries (empty without a block).
    pub fn ended_entries(&self) -> Vec<Value> {
        self.term()["ended"].as_array().cloned().unwrap_or_default()
    }

    pub fn kept_entries(&self) -> Vec<Value> {
        self.term()["kept"].as_array().cloned().unwrap_or_default()
    }

    /// A card's frontmatter as `settle_approved` takes it.
    pub fn card_meta(&self, card: &Card) -> serde_yaml_ng::Mapping {
        crate::ids::read_meta(&self.path(&format!("{}/{}", card.folder, card.name))).unwrap()
    }

    /// `settle_approved` on `card`, as `process_approvals` would call it for an approved card.
    pub fn settle(&self, card: &Card, day: Date) -> crate::commitments::Settled {
        let meta = self.card_meta(card);
        crate::commitments::settle_approved(&self.v, &meta, day, &self.ctx(), &mut Journal::new(&self.v)).unwrap()
    }

    /// A copy of this vault under another name (the lock file left behind).
    pub fn clone_as(&self, name: &str, first_day: &str) -> Lab {
        fn copy(from: &Path, to: &Path) {
            std::fs::create_dir_all(to).unwrap();
            for entry in std::fs::read_dir(from).unwrap().flatten() {
                let (src, dst) = (entry.path(), to.join(entry.file_name()));
                match src.is_dir() {
                    true => copy(&src, &dst),
                    false if src.extension().is_some_and(|e| e == "lock") => {}
                    false => {
                        std::fs::copy(&src, &dst).unwrap();
                    }
                }
            }
        }
        let other = Lab::new(name, first_day);
        let _ = std::fs::remove_dir_all(&other.v);
        copy(&self.v, &other.v);
        other
    }

    /// Makes this vault's cards, notes, journal and series file those of `other` (the state a fetch
    /// leaves), the way a fetch that ran alongside a `rank` would have left them here.
    pub fn adopt(&self, other: &Lab) {
        for dir in ["approvals", "archive", "commitments", "state/journal"] {
            let _ = std::fs::remove_dir_all(self.path(dir));
            if other.path(dir).is_dir() {
                copy_dir(&other.path(dir), &self.path(dir));
            }
        }
        std::fs::copy(other.path("state/calendar-series.json"), self.path("state/calendar-series.json")).unwrap();
    }
}

fn copy_dir(from: &Path, to: &Path) {
    std::fs::create_dir_all(to).unwrap();
    for entry in std::fs::read_dir(from).unwrap().flatten() {
        let (src, dst) = (entry.path(), to.join(entry.file_name()));
        match src.is_dir() {
            true => copy_dir(&src, &dst),
            false => {
                std::fs::copy(&src, &dst).unwrap();
            }
        }
    }
}

/// The series-file lock, held until the handle drops (what `refresh_series` and a filer take).
pub(super) fn hold_lock(lab: &Lab) -> std::fs::File {
    std::fs::create_dir_all(lab.path("state")).unwrap();
    let file = std::fs::OpenOptions::new().create(true).truncate(false).write(true).open(lab.path("state/calendar-series.lock")).unwrap();
    file.lock().unwrap();
    file
}

/// Waits for a thread to reach a lock it cannot take. The one wait in these tests: it proves a
/// negative ("nothing was written while the lock was held"), so a slow machine can only make a
/// test pass for the wrong reason, never fail one.
pub(super) fn let_it_block() {
    std::thread::sleep(std::time::Duration::from_millis(500));
}

/// `rank` on `day`, with the series-file lock held by the test. The rank's stash is empty, so its
/// `refresh_series` has no fresh calendar and takes no lock: it reads the series file before
/// `between` runs. The rank signals from inside `emit_checks_locked`, just before the filer takes
/// the lock (`commitments::AT_FILER_LOCK`, a channel, not a clock); `let_it_block` then lets it
/// enter the wait. `between` changes the vault the way a fetch alongside the rank would, and the
/// lock is released. A rank that never reaches the filer's lock fails here by name (B3eR fix 1).
pub(super) fn rank_interleaved(lab: &Lab, day: Date, between: impl FnOnce()) {
    let lock = hold_lock(lab);
    let (tx, rx) = std::sync::mpsc::channel::<()>();
    std::thread::scope(|scope| {
        let rank = scope.spawn(move || {
            let signal = move || {
                let _ = tx.send(());
            };
            crate::commitments::AT_FILER_LOCK.with(|hook| *hook.borrow_mut() = Some(Box::new(signal)));
            lab.rank(day);
        });
        if rx.recv_timeout(std::time::Duration::from_secs(300)).is_err() {
            drop(lock);
            let _ = rank.join();
            panic!("the rank never reached the filer's lock");
        }
        let_it_block();
        between();
        drop(lock);
        rank.join().unwrap();
    });
}
/// The three ways a row is read in part, applied to one CRN of an answer.
pub(super) fn read_in_part(answer: &mut Value, crn: &str, how: &str) {
    match how {
        "partial" => {
            retime(answer, crn, "1500", "1550");
            add_timeless_meeting(answer, crn);
        }
        "unread" => blank_times(answer, crn),
        _ => set_status(answer, crn, "WD", "Invented Withdrawn"),
    }
}


pub(super) fn fetch_read_in_part(lab: &Lab, day: Date, answer: &Value, how: &str) -> Value {
    match how {
        "status" => lab.fetch_registered(day, answer, &REGISTERED),
        _ => lab.fetch(day, answer),
    }
}
