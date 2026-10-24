//! `onboarding.json`, `onboarding-answers.json`, resume, skips, the three exits and the account
//! commands of step 2 again and step 4 (plan §4.5, §4.7, §4.10; F2a-F2h).
//!
//! **Still F1 stubs here:** F2h's account commands, F8b's `save_preview`, `save_pairs` and
//! `preview_courses` (each panics, naming its task). The answers, the three exits, the status and the
//! commands are F2b's.
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

pub const STATE_FILE: &str = "onboarding.json";
/// The only `version` this build reads; a file of any other is `Loaded::Unreadable`.
pub const STATE_VERSION: u32 = 1;
pub const ANSWERS_FILE: &str = "onboarding-answers.json";
/// The two pending-credential names under `knowlu/<profile_id>/` (§4.11).
pub const PENDING_LMS: &str = "pending-lms_ics";
pub const PENDING_CALENDAR: &str = "pending-calendar_ics";

/// Beside `settings.json`, in the profile's app data, never in the vault.
pub fn state_path(data_dir: &Path) -> PathBuf { data_dir.join(STATE_FILE) }
pub fn answers_path(data_dir: &Path) -> PathBuf { data_dir.join(ANSWERS_FILE) }

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Default, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum JobState {
    #[default]
    Pending,
    Running,
    Done,
    Failed,
    Skipped,
    Held,
}

#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct JobRecord {
    pub state: JobState,
    /// Counts only, each `null` where its input is missing (as the engine's `--preview` writes it),
    /// so a null survives the round trip and the rail never shows 0 for "not known".
    pub proof: BTreeMap<String, Option<i64>>,
    /// A closed-set reason (and for `failed`, its fix is the page's).
    pub reason: Option<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SchoolPath { Shared, Separate, Paste }

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct School {
    /// `None` for a school typed as "isn't listed": a name and no unitid (R-OB-4).
    pub unitid: Option<String>,
    pub label: String,
    pub path: SchoolPath,
    /// True once the student has confirmed this school in this run. Never trusted across a quit.
    pub confirmed: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Skip {
    pub step: u8,
    pub source: String,
    pub cost_key: String,
}

/// The two clearing signals a [`HandoverItem`] carries (§4.10): a slot's end, or `mail.json`.
pub const SIGNAL_SLOT_END: &str = "slot_end";
pub const SIGNAL_MAIL_JSON: &str = "mail_json";
/// Required items a slot finishes (§4.10's first two table rows): the slot, the first reads of the
/// slot's own steps, the Gmail first read and a calendar read.
const SLOT_ITEMS: [&str; 9] = [
    "first.sync", "first.coursework", "first.ingest", "first.grades", "slot.partial", "slot.first_full",
    "google.gmail_first_read", "calendar.read", "google.calendar_read",
];
/// Required items only `mail.json` clears: an open setup, the canary, the first pass.
const MAIL_ITEMS: [&str; 3] = ["mail.setup", "mail.canary", "mail.first_pass"];

/// One required item not yet `done` at Open Knowlu, with the signal that clears it (§4.10):
/// `slot_end` or `mail_json`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct HandoverItem {
    pub item: String,
    pub signal: String,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct OnboardingState {
    pub version: u32,
    pub step: u8,
    pub finished: bool,
    pub school: Option<School>,
    pub skips: Vec<Skip>,
    pub jobs: BTreeMap<String, JobRecord>,
    pub account_settled: bool,
    pub step10_confirmed_at: Option<String>,
    pub started_at: String,
    pub handover: Vec<HandoverItem>,
}

impl OnboardingState {
    pub fn new(started_at: &str, step: u8) -> OnboardingState {
        OnboardingState {
            version: STATE_VERSION,
            step,
            finished: false,
            school: None,
            skips: Vec::new(),
            jobs: BTreeMap::new(),
            account_settled: false,
            step10_confirmed_at: None,
            started_at: started_at.to_string(),
            handover: Vec::new(),
        }
    }
    /// The first slot after the reveal ended (or the first onboarding slot to end, before it):
    /// clears the `slot_end` handover items and no other (§4.10).
    pub fn slot_ended(&mut self) { self.handover.retain(|h| h.signal != SIGNAL_SLOT_END); }
    /// A mail signal read from `mail.json`: clears only its own item (§4.10). Forwarding verified
    /// means the setup is no longer open, so it clears `mail.setup` with the canary; a delivered first
    /// pass implies both, so it clears all three too; a disconnect clears all three. No slot item is
    /// ever touched.
    pub fn mail_signal(&mut self, signal: MailSignal) {
        let gone: &[&str] = match signal {
            MailSignal::CanaryVerified => &["mail.setup", "mail.canary"],
            MailSignal::FirstPassDelivered => &["mail.setup", "mail.canary", "mail.first_pass"],
            MailSignal::Disconnected => &MAIL_ITEMS,
        };
        self.handover.retain(|h| h.signal != SIGNAL_MAIL_JSON || !gone.contains(&h.item.as_str()));
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MailSignal { CanaryVerified, FirstPassDelivered, Disconnected }

/// What reading `onboarding.json` found. An unreadable file is never read as finished or absent.
#[derive(Debug, Clone, PartialEq)]
pub enum Loaded {
    Absent,
    Ok(OnboardingState),
    Unreadable(String),
}

/// Through `ledger::dumps_value`, to a sibling temp file and renamed over `path`, so a crash never
/// leaves half a file (as `grades.json` and `profiles.json` are written). App data, never a vault
/// file. The temp name carries a process-wide counter, so two threads saving one file never share it.
/// A failed rename is tried again after each of [`RENAME_PAUSES`] (a scanner or indexer can hold the
/// target for a moment, as `save_privacy_cache` allows for); the temp file is removed on every failure.
pub(crate) fn write_atomic(path: &Path, v: &Value) -> Result<(), String> {
    static N: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(0);
    let dir = path.parent().ok_or_else(|| format!("{}: has no folder", path.display()))?;
    std::fs::create_dir_all(dir).map_err(|e| e.to_string())?;
    let n = N.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
    let tmp = dir.join(format!("{}.{}.{n}.tmp", path.file_name().and_then(|f| f.to_str()).unwrap_or("onboarding"), std::process::id()));
    if let Err(e) = std::fs::write(&tmp, knowlu_engine::ledger::dumps_value(v)) {
        let _ = std::fs::remove_file(&tmp);
        return Err(e.to_string());
    }
    let first = match std::fs::rename(&tmp, path) {
        Ok(()) => return Ok(()),
        Err(e) => e,
    };
    for pause in RENAME_PAUSES {
        std::thread::sleep(pause);
        if std::fs::rename(&tmp, path).is_ok() { return Ok(()); }
    }
    let _ = std::fs::remove_file(&tmp);
    Err(format!("{}: could not be written ({first})", path.display()))
}
const RENAME_PAUSES: [std::time::Duration; 2] = [std::time::Duration::from_millis(25), std::time::Duration::from_millis(200)];

/// Temp file and rename, in the profile's app data. This save has no freeze: the sign-out rewind
/// writes through it after the queue's saver is frozen and must land (review r5, M1).
pub fn save_state(data_dir: &Path, s: &OnboardingState) -> Result<(), String> {
    write_atomic(&state_path(data_dir), &serde_json::to_value(s).map_err(|e| e.to_string())?)
}

/// A missing file is `Absent`; any other failure to read or parse is `Unreadable`, never "finished".
pub fn load_state(data_dir: &Path) -> Loaded {
    let path = state_path(data_dir);
    match std::fs::read_to_string(&path) {
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => Loaded::Absent,
        Err(e) => Loaded::Unreadable(format!("{}: cannot be read ({e})", path.display())),
        Ok(text) => match serde_json::from_str::<OnboardingState>(&text) {
            Ok(s) if s.version != STATE_VERSION => Loaded::Unreadable(format!("{}: is version {}, which this build does not know", path.display(), s.version)),
            Ok(s) => Loaded::Ok(s),
            Err(e) => Loaded::Unreadable(format!("{}: is not an onboarding file ({e})", path.display())),
        },
    }
}

// ---- the answers file (§4.5) -------------------------------------------------------------------

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CourseAnswer {
    /// The human course code (`KNT 220`). Two shells of one course (a past term's and this term's)
    /// share it, so it is never a row's identity: [`CourseAnswer::key`] is.
    pub code: String,
    pub name: String,
    /// What the proof shows; the student may edit it (`{"label"}`), so it is display only and never
    /// part of [`CourseAnswer::key`] (§14.46.3).
    pub label: String,
    /// The label **as captured**, set once by [`Answers::recapture`] from the captured row's `label`
    /// (whatever a page sends here is overwritten) and never changed by a label edit: the key's last
    /// fallback, for a row with no LMS key and no slug (the paste fallback's typed list).
    #[serde(default)]
    pub captured_label: String,
    /// The LMS's own course key (one per shell, never shared by two terms): the row's identity and
    /// the engine's `code` (F7a).
    pub lms_key: String,
    /// Step 3's Remove (R-C1-51): set through `onboarding_answer`, cleared by a re-capture. A
    /// removed row is read by nobody; [`Answers::kept_courses`] is the only way to the list.
    #[serde(default)]
    pub removed: bool,
    /// The course's slug where the LMS gives one; only for [`CourseAnswer::key`] (not the note's slug,
    /// which `school.courses` records in [`Answers::slugs`]).
    #[serde(default)]
    pub slug: String,
    /// The course's pairs for `course_map` and `course_exclude` (§14.45, §14.46): its code fragments
    /// and its gradebook uid pins (column ids, `_<digits>_<digits>`), built once by F8b at capture and
    /// written only through [`save_pairs`] (or a re-capture's own non-empty list, [`Answers::recapture`]);
    /// never from a page answer. Empty until then; the hand-over then falls back to the course's code.
    #[serde(default)]
    pub fragments: Vec<String>,
}

impl CourseAnswer {
    /// The key `by_course` uses, and so Remove's, a label edit's, [`save_pairs`]'s and a re-capture's
    /// identity for a row (F7a, §14.45.2): the **LMS course key** (`lms_key`), then the slug, then
    /// **`captured_label`**, each trimmed, the first that is not empty (as the engine reads its own
    /// `code`, `slug`, `label`). Never the human `code`, which two shells share, and never the
    /// editable `label`, so a label edit changes no key, no bucket and no Remove.
    pub fn key(&self) -> String {
        [&self.lms_key, &self.slug, &self.captured_label].iter().map(|f| f.trim()).find(|t| !t.is_empty()).unwrap_or_default().to_string()
    }
}

/// The name of a match-set bucket as the engine's `by_course` writes it (F7a, 904a92c): the sorted
/// course keys joined by `,`; [`UNMATCHED`] for an item no course claims. The app never parses the
/// name: it reads [`Bucket::codes`].
pub fn bucket_key(codes: &[&str]) -> String {
    let mut c: Vec<&str> = codes.to_vec();
    c.sort_unstable();
    if c.is_empty() { UNMATCHED.to_string() } else { c.join(",") }
}
pub const UNMATCHED: &str = "unmatched";

/// One `by_course` bucket exactly as the engine writes it: the course keys that claimed its items
/// (empty for [`UNMATCHED`]) and the counts by name (`assignments_upcoming`, `turned_in`,
/// `class_times`), each **`null` where its input is missing** (`turned_in` with no grades bundle,
/// `class_times` with no registrar), never 0. A single-course bucket always exists, with zero (or
/// null) counts when nothing landed in it.
#[derive(Debug, Clone, PartialEq, Eq, Default, Serialize, Deserialize)]
pub struct Bucket {
    pub codes: Vec<String>,
    /// §14.47: on a **single-course** bucket, `true` when the grades bundle has a graded item column
    /// for the course, `false` when it has none, `null` when no bundle was given. Multi-code buckets
    /// and `unmatched` carry none; absent and `null` read the same, since only single-course buckets
    /// are counted for `grades_courses`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub graded: Option<bool>,
    #[serde(flatten)]
    pub counts: BTreeMap<String, Option<i64>>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RegistrarRow {
    pub code: String,
    pub days: String,
    pub start: String,
    pub end: String,
}

/// A feed's state: never the link (§4.11).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum FeedState { Validated, Stored, Rejected, Failed }

/// A confirmed homework-site row: the vendor's key and the course slug it belongs to.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct BookRow {
    pub source: String,
    pub key: String,
    pub course_slug: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Default, Serialize, Deserialize)]
pub struct Answers {
    pub courses: Vec<CourseAnswer>,
    pub registrar_rows: Vec<RegistrarRow>,
    /// [`CourseAnswer::key`] (the LMS course key, then the slug, then the label; never the human code,
    /// which two shells share) to the slug `school.courses` settled on (a restored note's own where
    /// one matched). Two kept shells of one human code have two entries.
    pub slugs: BTreeMap<String, String>,
    pub google_calendar: Option<bool>,
    /// `lms_ics` or `calendar_ics` to its state.
    pub feeds: BTreeMap<String, FeedState>,
    pub rows: Vec<BookRow>,
    pub zybooks_ignore: Vec<String>,
    /// `school.preview`'s `by_course` match-set buckets (§14.45), by [`bucket_key`], each a
    /// [`Bucket`]: its `codes` and its counts by name. Written only by [`save_preview`].
    #[serde(default)]
    pub buckets: BTreeMap<String, Bucket>,
    /// The service's own string for the mail setup's provider (`m365`, `google` or `other`), written by
    /// F8d's runner through [`update_answers`] and read by [`onboarding_finish_inner`] before it deletes
    /// this file. Absent keeps today's bytes.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub mail_provider: Option<String>,
}

impl Answers {
    /// The courses that were not removed, in captured order. `school.courses`, `config.ingest`'s
    /// `course_map` and the step-3 totals read the list only through this.
    pub fn kept_courses(&self) -> Vec<&CourseAnswer> { self.courses.iter().filter(|c| !c.removed).collect() }
    /// A re-capture: replaces the list with `captured`, every row un-removed and its `captured_label`
    /// set to its captured `label` **first**, before any key is compared, so the match below uses the
    /// key the new row will keep. A captured row's own non-empty `fragments` win (a new gradebook pin
    /// is never lost); a captured row with none, whose [`CourseAnswer::key`] matches an old row's,
    /// keeps that row's `fragments`.
    pub fn recapture(&mut self, captured: Vec<CourseAnswer>) {
        let old: Vec<(String, Vec<String>)> = self.courses.iter().map(|c| (c.key(), c.fragments.clone())).collect();
        self.courses = captured.into_iter().map(|mut c| {
            c.removed = false;
            c.captured_label = c.label.clone();
            if c.fragments.is_empty() {
                let key = c.key();
                if let Some((_, f)) = old.iter().find(|(k, _)| !key.is_empty() && *k == key) { c.fragments = f.clone(); }
            }
            c
        }).collect();
    }
}

/// Step 3's proof totals (§14.45, §14.47), what F6c's ingest and `registrar.confirm` will import
/// (`onboarding_status` returns it):
/// - `classes`: the number of kept rows;
/// - `assignments_upcoming`, `turned_in` (every feed count): the sum, over **every bucket whose
///   `codes` is empty ([`UNMATCHED`]) or contains at least one kept course's key**, of that count's
///   numbers. A bucket naming only removed courses is out; one naming a kept and a removed course stays;
/// - `class_times`: the sum over **every** bucket, whatever was removed (§14.47, G5: the registrar
///   holds only the current term, and every class time is imported); its buckets are display only;
/// - `grades_courses`: the number of **kept single-course buckets with `graded: true`**; `null` when
///   every single-course bucket's `graded` is `null` (no bundle).
/// A count name is **`null`** when every bucket carrying it carries `null` (its input was missing), so
/// the proof shows nothing rather than 0; with **no buckets at all** (no preview yet, or a failed one)
/// every count name is `null` and only `classes` is a number.
pub fn step3_totals(a: &Answers) -> BTreeMap<String, Option<i64>> {
    let kept: Vec<String> = a.kept_courses().iter().map(|c| c.key()).collect();
    let is_kept = |b: &Bucket| b.codes.is_empty() || b.codes.iter().any(|c| kept.contains(c));
    // Every count name starts null; a name sums over the buckets that count for it and stays null
    // while every bucket carrying it carries null.
    let mut t: BTreeMap<String, Option<i64>> = ["assignments_upcoming", "turned_in", "class_times", "grades_courses"].iter().map(|n| (n.to_string(), None)).collect();
    for b in a.buckets.values() {
        for (name, n) in &b.counts {
            let counted = name == "class_times" || is_kept(b);
            let slot = t.entry(name.clone()).or_insert(None);
            if let (true, Some(n)) = (counted, n) { *slot = Some(slot.unwrap_or(0) + n); }
        }
    }
    let single = || a.buckets.values().filter(|b| b.codes.len() == 1);
    if single().any(|b| b.graded.is_some()) {
        t.insert("grades_courses".into(), Some(single().filter(|b| b.graded == Some(true) && is_kept(b)).count() as i64));
    }
    t.insert("classes".into(), Some(kept.len() as i64));
    t
}

/// F8b's seam after `school.preview`: parses the engine's `by_course` object (its nulls included)
/// into [`Answers::buckets`], **replacing** the earlier preview's (never merged with them), and saves.
/// `null` (the engine had no course list) or anything not an object is an `Err` that changes no byte
/// of the file. Never a page answer.
pub fn save_preview(_data_dir: &Path, _by_course: &Value) -> Result<(), String> { todo!("F8b: save the preview's buckets") }

/// F8b's seam at capture (§14.46.3): each course's pairs, by [`CourseAnswer::key`], built once with
/// `scaffold::course_fragments` and the bundle's column ids, saved as that row's `fragments`. The one
/// writer of pairs besides a re-capture's own; never a page answer.
pub fn save_pairs(_data_dir: &Path, _pairs: &BTreeMap<String, Vec<String>>) -> Result<(), String> {
    todo!("F8b: save the pairs")
}

/// The answer-recording inner of `onboarding_answer(step, answer)` (F2b): saves the answer to the
/// answers file **first**, then calls `enqueue` once per job the answer asks for. Shapes and jobs:
/// - 3 `{"capture": {"courses": [CourseAnswer], "registrar_rows": [..] (default empty), "feeds":
///   {kind: state} (default empty)}}` enqueues `school.preview`. It goes through
///   [`Answers::recapture`] with each row's `fragments` **dropped**: a page never supplies pairs;
///   a matching row keeps the ones [`save_pairs`] wrote;
/// - 3 `{"remove": <CourseAnswer::key>}` and `{"label": {"key": <key>, "label": <text>}}` (display
///   only: it changes no pairs and no bucket) enqueue nothing;
/// - 6 `{"google_calendar": bool}` enqueues `google.calendar_read` for `true`, nothing for `false`;
/// - 8 `{"row": BookRow}` or `{"zybooks_ignore": <key>}` enqueue nothing (`config.ingest` is asked
///   for on leaving the step).
/// Answers `{ok, error?}`; an unknown shape is `ok: false`, enqueues nothing and changes no byte of
/// the file. **`{"buckets"}` and `{"pairs"}` are refused** the same way: both come only from F8b,
/// through [`save_preview`] and [`save_pairs`] (§4.5, §14.46.3). Nothing is held in memory, so a
/// quit between two calls loses nothing.
///
/// **Not answers:** step 3's `school.capture` is asked for by the sign-in trigger (R-C1-42: the
/// "I'm signed in" press through F8lb's `onboarding_capture_link`, or S4d's signed-in test), and
/// step 6's `google.gmail_first_read` by the Gmail grant (the consent poll, `google_status`), or
/// `mail.canary` by the forwarding setup's end (F8d, F11b). No shape here enqueues any of the three.
/// `stopping` is checked again inside the answers lock, so an answer that passed `exiting_now` just
/// before sign-out cannot put the file back (F2b review r2).
pub fn onboarding_answer_checked(
    data_dir: &Path,
    step: u8,
    answer: &Value,
    enqueue: &mut dyn FnMut(&str),
    stopping: &dyn Fn() -> bool,
) -> Value {
    let refuse = |why: &str| serde_json::json!({ "ok": false, "error": why });
    // One key, one shape: anything else (`buckets`, `pairs`, two keys at once) is refused unread.
    let Some((name, body)) = answer.as_object().filter(|o| o.len() == 1).and_then(|o| o.iter().next()) else {
        return refuse("that answer has no shape Knowlu knows");
    };
    let text = |v: &Value| v.as_str().map(str::to_string);
    let (saved, jobs): (Result<(), String>, &[&str]) = match (step, name.as_str()) {
        (3, "capture") => {
            #[derive(Deserialize)]
            struct Capture {
                courses: Vec<CourseAnswer>,
                #[serde(default)]
                registrar_rows: Vec<RegistrarRow>,
                #[serde(default)]
                feeds: BTreeMap<String, FeedState>,
            }
            let Ok(c) = serde_json::from_value::<Capture>(body.clone()) else { return refuse("step 3's capture is not a course list") };
            // The page may say only what a paste proved: a link is `validated` or `rejected`. `stored`
            // and `failed` are the store job's words.
            if c.feeds.values().any(|s| !matches!(s, FeedState::Validated | FeedState::Rejected)) { return refuse("a feed is validated or rejected here"); }
            // A page never supplies pairs: a matching row keeps the ones `save_pairs` wrote.
            let courses: Vec<CourseAnswer> = c.courses.into_iter().map(|mut r| { r.fragments.clear(); r }).collect();
            let saved = replace_answers(data_dir, stopping, &|a| {
                a.recapture(courses.clone());
                a.registrar_rows = c.registrar_rows.clone();
                a.feeds.extend(c.feeds.clone());
            });
            (saved, &["school.preview"])
        }
        (3, "remove") => {
            let Some(key) = text(body) else { return refuse("a removal names a course key") };
            if let Err(e) = require_course(data_dir, &key) { return refuse(&file_only(e, data_dir)); }
            (update_answers_unless(data_dir, stopping, &|a| { for c in a.courses.iter_mut().filter(|c| c.key() == key) { c.removed = true; } }), &[])
        }
        (3, "label") => {
            let (Some(key), Some(label)) = (text(&body["key"]), text(&body["label"])) else { return refuse("a label edit names a course key and a label") };
            if let Err(e) = require_course(data_dir, &key) { return refuse(&file_only(e, data_dir)); }
            (update_answers_unless(data_dir, stopping, &|a| { for c in a.courses.iter_mut().filter(|c| c.key() == key) { c.label = label.clone(); } }), &[])
        }
        (6, "google_calendar") => {
            let Some(on) = body.as_bool() else { return refuse("the calendar answer is yes or no") };
            (update_answers_unless(data_dir, stopping, &|a| a.google_calendar = Some(on)), if on { &["google.calendar_read"] } else { &[] })
        }
        (8, "row") => {
            let Ok(row) = serde_json::from_value::<BookRow>(body.clone()) else { return refuse("a homework row names a source, a key and a course") };
            let saved = update_answers_unless(data_dir, stopping, &|a| match a.rows.iter_mut().find(|r| r.source == row.source && r.key == row.key) {
                Some(old) => *old = row.clone(),
                None => a.rows.push(row.clone()),
            });
            (saved, &[])
        }
        (8, "zybooks_ignore") => {
            let Some(key) = text(body) else { return refuse("an ignored section is a key") };
            let saved = update_answers_unless(data_dir, stopping, &|a| { if !a.zybooks_ignore.contains(&key) { a.zybooks_ignore.push(key.clone()); } });
            (saved, &[])
        }
        _ => return refuse("that answer has no shape Knowlu knows at this step"),
    };
    if let Err(e) = saved { return if e == SETUP_CLOSING { refused_exiting() } else { refuse(&file_only(e, data_dir)) }; }
    // Saved first: a quit between the save and an enqueue loses nothing, since the answer is on disk.
    for job in jobs { enqueue(job); }
    serde_json::json!({ "ok": true, "error": Value::Null })
}

/// Step 3's capture: like [`update_answers`], but an unreadable (or absent) file is **replaced** by
/// what the capture holds, never merged into and never refused (F2d review r1, I-2). The re-capture is
/// the student's own do-over, so the damaged file is the one thing it is allowed to discard.
fn replace_answers(data_dir: &Path, stopping: &dyn Fn() -> bool, change: &dyn Fn(&mut Answers)) -> Result<(), String> {
    let _held = ANSWERS_LOCK.lock().unwrap_or_else(|p| p.into_inner());
    if stopping() { return Err(SETUP_CLOSING.into()); }
    let mut a = load_answers(data_dir).ok().flatten().unwrap_or_default();
    change(&mut a);
    save_answers(data_dir, &a)
}

/// An error that names the answers file by its name only: the app-data path carries the Windows user name.
fn file_only(e: String, data_dir: &Path) -> String {
    e.replace(&answers_path(data_dir).display().to_string(), ANSWERS_FILE).replace(&data_dir.display().to_string(), "")
}

/// True once an exit has begun: the queue's stop is set or the exit guard is held. An answer or a skip
/// written then could put back what Sign out just deleted, or land after the last save.
pub fn exiting_now(queue: &crate::onboarding_jobs::Queue, g: &ExitGuard) -> bool {
    queue.stop_requested() || g.0.load(std::sync::atomic::Ordering::SeqCst)
}

fn refused_exiting() -> Value { serde_json::json!({ "ok": false, "exiting": true, "error": "Knowlu is closing setup. Open it again to carry on." }) }

/// [`onboarding_answer_inner`], refused outright (nothing written, nothing enqueued) while `stopping`.
pub fn answer_unless_exiting(stopping: bool, data_dir: &Path, step: u8, answer: &Value, enqueue: &mut dyn FnMut(&str)) -> Value {
    answer_unless_exiting_with(&|| stopping, data_dir, step, answer, enqueue)
}

/// [`answer_unless_exiting`] with a live stop check: asked first, and again inside the answers lock.
pub fn answer_unless_exiting_with(stopping: &dyn Fn() -> bool, data_dir: &Path, step: u8, answer: &Value, enqueue: &mut dyn FnMut(&str)) -> Value {
    if stopping() { return refused_exiting(); }
    onboarding_answer_checked(data_dir, step, answer, enqueue, stopping)
}

/// [`onboarding_answer_checked`] with no stop: the tests' and the lane's direct callers.
pub fn onboarding_answer_inner(data_dir: &Path, step: u8, answer: &Value, enqueue: &mut dyn FnMut(&str)) -> Value {
    onboarding_answer_checked(data_dir, step, answer, enqueue, &|| false)
}

/// A skip recorded through `apply`, refused outright while `stopping`.
pub fn skip_unless_exiting(stopping: bool, apply: &mut dyn FnMut() -> Result<(), String>) -> Value {
    if stopping { return refused_exiting(); }
    match apply() {
        Ok(()) => serde_json::json!({ "ok": true, "error": Value::Null }),
        Err(e) => serde_json::json!({ "ok": false, "error": e }),
    }
}

/// A remove or a label edit names a saved course by its key; no such course changes no byte.
fn require_course(data_dir: &Path, key: &str) -> Result<(), String> {
    match load_answers(data_dir)? {
        Some(a) if a.courses.iter().any(|c| c.key() == key) => Ok(()),
        _ => Err("no course has that key".into()),
    }
}

/// `Ok(None)` is an absent file; an unreadable one is `Err`, never "no answers".
pub fn load_answers(data_dir: &Path) -> Result<Option<Answers>, String> {
    let path = answers_path(data_dir);
    match std::fs::read_to_string(&path) {
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(None),
        Err(e) => Err(format!("{}: cannot be read ({e})", path.display())),
        Ok(text) => serde_json::from_str::<Answers>(&text).map(Some).map_err(|e| {
            // Category, line and column only: serde's own text can quote the file's content (a course label).
            format!("{}: is not an answers file ({:?} at line {}, column {})", path.display(), e.classify(), e.line(), e.column())
        }),
    }
}
/// Temp file and rename in app data (never a vault file), as [`save_state`].
pub fn save_answers(data_dir: &Path, a: &Answers) -> Result<(), String> {
    write_atomic(&answers_path(data_dir), &serde_json::to_value(a).map_err(|e| e.to_string())?)
}
static ANSWERS_LOCK: std::sync::Mutex<()> = std::sync::Mutex::new(());

/// Load (or start empty), change, save: how each answer is written the moment it is given.
pub fn update_answers(data_dir: &Path, change: &dyn Fn(&mut Answers)) -> Result<(), String> {
    update_answers_unless(data_dir, &|| false, change)
}

/// The error a write answers when the stop was set by the time it held the lock: nothing was written.
const SETUP_CLOSING: &str = "setup is closing";

/// [`update_answers`], refused inside the lock while `stopping()` (sign-out deletes the file under it).
pub fn update_answers_unless(data_dir: &Path, stopping: &dyn Fn() -> bool, change: &dyn Fn(&mut Answers)) -> Result<(), String> {
    // Held across load, change and save, so a Remove and a `save_preview` never overwrite each other.
    let _held = ANSWERS_LOCK.lock().unwrap_or_else(|p| p.into_inner());
    if stopping() { return Err(SETUP_CLOSING.into()); }
    // An unreadable file is an error here: it is never started over from empty and written back.
    let mut a = load_answers(data_dir)?.unwrap_or_default();
    change(&mut a);
    save_answers(data_dir, &a)
}

// ---- the pending credentials (§4.11) -------------------------------------------------------------

/// The pending feed links. The real store is Credential Manager under `knowlu/<profile_id>/<name>`
/// (`credentials::write`, `exists`, `delete`); tests pass a fake.
pub trait PendingStore {
    fn put(&self, name: &str, link: &str) -> Result<(), String>;
    fn get(&self, name: &str) -> Option<String>;
    /// Deleting what is not there is not an error; a link that is there and stays is (F2d review M2).
    fn delete(&self, name: &str) -> Result<(), String>;
}

pub struct CredPending { pub profile_id: String }

impl PendingStore for CredPending {
    fn put(&self, name: &str, link: &str) -> Result<(), String> {
        crate::credentials::write(&crate::credentials::target_for(&self.profile_id, name), &self.profile_id, link)
    }
    fn get(&self, name: &str) -> Option<String> {
        let target = crate::credentials::target_for(&self.profile_id, name);
        if !crate::credentials::exists(&target) { return None; }
        knowlu_engine::wincred::read_credential(&target).ok().map(|c| c.password.expose().to_string())
    }
    fn delete(&self, name: &str) -> Result<(), String> {
        let target = crate::credentials::target_for(&self.profile_id, name);
        if crate::credentials::exists(&target) { crate::credentials::delete(&target) } else { Ok(()) }
    }
}

/// `lms_ics` to [`PENDING_LMS`], `calendar_ics` to [`PENDING_CALENDAR`].
pub fn pending_name(kind: &str) -> Option<&'static str> {
    match kind {
        "lms_ics" => Some(PENDING_LMS),
        "calendar_ics" => Some(PENDING_CALENDAR),
        _ => None,
    }
}

// ---- resume and LaunchMode (§4.5, §4.7) ----------------------------------------------------------

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LaunchMode { Normal, Onboarding }

/// What §4.7's checks prove, each a fake in the tests. Nothing here reads a school: the account
/// cannot name one (T0, 2026-10-24), and [`ResumeChecks::account_school`] exists so a test can
/// count that it is never asked.
pub trait ResumeChecks {
    fn session_present(&self) -> bool;
    fn restored_course_count(&self) -> usize;
    fn entitled(&self) -> bool;
    fn school_mailbox_active(&self) -> bool;
    fn google_granted(&self) -> bool;
    fn homework_login_stored(&self) -> bool;
    fn class_commitments_cover_courses(&self) -> bool;
    fn account_school(&self) -> Option<School>;
    /// The answers file is present and readable. Past step 3 an absent or unreadable one resumes
    /// at step 3: it means the answers were deleted or never saved, never that there were none.
    fn answers_ok(&self) -> bool;
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SchoolOffer {
    /// Step 3 asks for the school, as on a fresh install.
    Ask,
    /// The device's own earlier choice, offered as a confirm. Nothing is assumed.
    Confirm(School),
}

#[derive(Debug, Clone, PartialEq)]
pub struct Resume {
    pub mode: LaunchMode,
    /// What onboarding runs with: `running` jobs back to `pending`; for an unreadable file, a fresh
    /// state at the derived step. `None` for a normal launch.
    pub state: Option<OnboardingState>,
    /// Where the page opens: step 2 whenever the profile holds no session.
    pub step: u8,
    pub school: SchoolOffer,
}

/// Absent is a normal launch (every install before this lane); a finished file is too, with or
/// without a hand-over; unfinished and unreadable files are onboarding.
pub fn launch_mode(loaded: &Loaded) -> LaunchMode {
    match loaded {
        Loaded::Absent | Loaded::Ok(OnboardingState { finished: true, .. }) => LaunchMode::Normal,
        Loaded::Ok(_) | Loaded::Unreadable(_) => LaunchMode::Onboarding,
    }
}

/// The earliest step §4.7's checks do not prove (an unreadable file's resume). **The school is never
/// a rung** (§14.49): nothing proves it, so with no `school` in the state the ladder stops at 3.
/// Step 7 is asked only when step 6 granted no calendar and step 9 is absent in wave B, so neither
/// is a rung.
fn earliest_unproven(c: &dyn ResumeChecks, has_school: bool) -> u8 {
    if !c.session_present() { 2 }
    else if !has_school || c.restored_course_count() == 0 { 3 }
    else if !c.entitled() { 4 }
    else if !c.school_mailbox_active() { 5 }
    else if !c.google_granted() { 6 }
    else if !c.homework_login_stored() { 8 }
    else { 10 }
}

/// After the student confirms the school at step 3 (the state then holds a confirmed `school`): moves
/// the state to the earliest step past 3 the checks do not prove and returns the steps it passed
/// through, each `already-done` with its proof (§4.7). Nothing changes while the school is
/// unconfirmed, the state is not at step 3, the restore brought no courses (step 3 still needs its
/// capture), or the answers file is absent or unreadable (step 3 is asked again, never skipped).
pub fn advance_after_school(state: &mut OnboardingState, checks: &dyn ResumeChecks) -> Vec<u8> {
    if state.step != 3 || !school_apply_may_run(state) || !checks.answers_ok() { return Vec::new(); }
    let next = earliest_unproven(checks, true);
    if next <= 3 { return Vec::new(); }
    state.step = next;
    (4..next).filter(|s| *s != 9).collect()
}

pub fn resume(loaded: Loaded, checks: &dyn ResumeChecks) -> Resume {
    let mode = launch_mode(&loaded);
    let mut state = match loaded {
        Loaded::Absent | Loaded::Ok(OnboardingState { finished: true, .. }) => None,
        Loaded::Ok(s) => Some(s),
        // Rewritten from what the account and this device prove, never read as finished.
        Loaded::Unreadable(_) => Some(OnboardingState::new(&knowlu_engine::journal::now_ts(None), earliest_unproven(checks, false))),
    };
    if let Some(s) = state.as_mut() {
        for j in s.jobs.values_mut().filter(|j| j.state == JobState::Running) { j.state = JobState::Pending; }
        // A school is never trusted across a quit: it is offered again as a confirm.
        if let Some(sc) = s.school.as_mut() { sc.confirmed = false; }
    }
    // Past step 3 an absent or unreadable answers file means they were deleted or never saved, never
    // that there were none: step 3 is asked again (§4.5, §14.49).
    if !checks.answers_ok() {
        if let Some(s) = state.as_mut().filter(|s| s.step > 3) { s.step = 3; }
    }
    // A profile with no session resumes at step 2's sign-in whatever the file's step, which is kept.
    // With a session, a file at step 2 or below (a rewound file, or a vault just made) lands on step 3.
    if checks.session_present() {
        if let Some(s) = state.as_mut().filter(|s| s.step < 3) { s.step = 3; }
    }
    let step = match &state {
        Some(s) if checks.session_present() => s.step,
        Some(_) => 2,
        None => 0,
    };
    let school = school_offer(state.as_ref(), checks);
    Resume { mode, state, step, school }
}

/// Step 3's school: only ever from this device's file, never from the account.
pub fn school_offer(state: Option<&OnboardingState>, _checks: &dyn ResumeChecks) -> SchoolOffer {
    match state.and_then(|s| s.school.as_ref()) {
        Some(sc) => SchoolOffer::Confirm(School { confirmed: false, ..sc.clone() }),
        None => SchoolOffer::Ask,
    }
}

/// `school.apply` does not run until the school is confirmed.
pub fn school_apply_may_run(state: &OnboardingState) -> bool { state.school.as_ref().is_some_and(|s| s.confirmed) }
pub fn confirm_school(state: &mut OnboardingState) {
    if let Some(s) = state.school.as_mut() { s.confirmed = true; }
}
/// Step 10 asks for proposals only once `config.ingest` has written the file they read (§4.2, §4.3).
pub fn proposals_ready(state: &OnboardingState) -> bool {
    state.jobs.get("config.ingest").is_some_and(|j| j.state == JobState::Done)
}

/// The skip cost keys the rail, the reveal gate (F9) and the page name; a contract, never retyped.
pub const COST_MAIL: &str = "mail_dates_missed";
pub const COST_GOOGLE: &str = "gmail_and_calendar_missed";
pub const COST_CALENDAR: &str = "calendar_busy_time_missed";
/// Step 8's key is per site: this prefix and the site (`zybooks`, `vhl`).
pub const COST_HOMEWORK_PREFIX: &str = "homework_due_dates_missing_";

/// Account, payment, the school step and "Your week" refuse a skip; every other step takes one.
/// Steps 1 (welcome), 9 (absent in wave B) and 11 (the reveal) ask nothing, so there is nothing to skip.
/// The source must belong to the step (5 `mail`, 6 `google`, 7 `calendar`, 8 `zybooks` or `vhl`); a
/// skip is recorded once per `(step, source)`, with its cost key.
pub fn skip_step(state: &mut OnboardingState, step: u8, source: &str) -> Result<(), String> {
    let cost_key = match (step, source) {
        (5, "mail") => COST_MAIL.to_string(),
        (6, "google") => COST_GOOGLE.to_string(),
        (7, "calendar") => COST_CALENDAR.to_string(),
        (8, "zybooks" | "vhl") => format!("{COST_HOMEWORK_PREFIX}{source}"),
        (5..=8, _) => return Err(format!("step {step} has no source {source:?} to skip")),
        (2 | 3 | 4 | 10, _) => return Err(format!("step {step} cannot be skipped")),
        _ => return Err(format!("step {step} has nothing to skip")),
    };
    if !state.skips.iter().any(|k| k.step == step && k.source == source) {
        state.skips.push(Skip { step, source: source.to_string(), cost_key });
    }
    Ok(())
}

/// F2e: deletes the answers file and both pending credentials, then rewinds an unfinished
/// `onboarding.json` to step 2 (jobs pending, `account_settled`, `school`, `step10_confirmed_at` and
/// `handover` cleared, skips kept).
/// A finished file is left alone. `account::sign_out_clean` calls the real-store wrapper.
pub fn on_sign_out_with(data_dir: &Path, pending: &dyn PendingStore) -> Result<(), String> {
    // Every step runs; the failures are collected, so one stuck file never keeps the rest on the device.
    let mut failed: Vec<String> = Vec::new();
    let answers = answers_path(data_dir);
    // Under the answers lock, so a writer that is mid-save finishes first and its file is the one deleted.
    let held = ANSWERS_LOCK.lock().unwrap_or_else(|p| p.into_inner());
    match std::fs::remove_file(&answers) {
        Ok(()) => {}
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => {}
        Err(e) => failed.push(format!("{}: could not be deleted ({e})", answers.display())),
    }
    drop(held);
    for name in [PENDING_LMS, PENDING_CALENDAR] {
        if let Err(e) = pending.delete(name) { failed.push(format!("pending link {name}: {e}")); }
    }
    match load_state(data_dir) {
        Loaded::Absent => {}
        Loaded::Unreadable(e) => failed.push(e),
        Loaded::Ok(s) if s.finished => {}
        Loaded::Ok(mut s) => {
            s.step = 2;
            s.account_settled = false;
            s.school = None;
            // A step-2 file is a fresh install's: no confirmation or hand-over from before the sign-out.
            s.step10_confirmed_at = None;
            s.handover.clear();
            for job in s.jobs.values_mut() { *job = JobRecord::default(); }
            if let Err(e) = save_state(data_dir, &s) { failed.push(e); }
        }
    }
    if failed.is_empty() { Ok(()) } else { Err(failed.join("; ")) }
}
/// [`on_sign_out_with`] over this profile's own Credential Manager entries.
pub fn on_sign_out(data_dir: &Path, profile_id: &str) -> Result<(), String> {
    on_sign_out_with(data_dir, &CredPending { profile_id: profile_id.to_string() })
}

// ---- the three exits (§4.10, F2b) ----------------------------------------------------------------

/// Everything an exit touches outside the queue's own state, so F1 drives each exit with fakes.
/// `request_stop` sets the stop; `drain` waits for the safe points of the queue and the lane.
pub trait ExitSeams {
    fn request_stop(&self);
    fn drain(&self);
    fn save(&self) -> Result<(), String>;
    /// After this, the queue's saver refuses every save (F8f). F2a's file save has no such flag.
    fn freeze_saves(&self);
    /// `account::sign_out`, whose clean runs `on_sign_out`.
    fn sign_out(&self) -> Result<(), String>;
    /// `onboarding::relaunch(args)`.
    fn relaunch(&self, args: &[&str]) -> Result<(), String>;
    /// `onboarding::relaunch_with(vault)`.
    fn relaunch_with(&self) -> Result<(), String>;
    /// `settings.quit_at`, as the tray's Quit and `switch_profile` stamp it.
    fn stamp_quit_at(&self);
    /// `state::quit_flush`, capped at 10 s.
    fn flush(&self);
    fn exit(&self);
    /// The queue's state as of now, for finish's refusal and its hand-over.
    fn snapshot(&self) -> OnboardingState;
    /// Writes `finished: true` and `handover`, deletes the answers file, applies autostart.
    fn write_finish(&self, handover: Vec<HandoverItem>) -> Result<(), String>;
    /// The answers file's recorded `mail_provider`, read before `write_finish` deletes the file.
    /// Default: none recorded, which keeps every mail item in the hand-over.
    fn mail_provider(&self) -> Option<String> { None }
}

/// One exit at a time: a second exit while one drains answers `{ok: true, exiting: true}`.
#[derive(Default)]
pub struct ExitGuard(pub std::sync::atomic::AtomicBool);

/// The process's one guard, shared by the three commands and F4's close handler.
pub fn exit_guard() -> std::sync::Arc<ExitGuard> {
    static GUARD: std::sync::OnceLock<std::sync::Arc<ExitGuard>> = std::sync::OnceLock::new();
    GUARD.get_or_init(Default::default).clone()
}

/// One call sets both stops (§14.51): the scheduler's, which a slot in flight checks between its
/// stages, then the queue's. The live `ExitSeams::request_stop` is this call and nothing else.
pub fn stop_both(sch: &crate::scheduler::Scheduler, queue: &crate::onboarding_jobs::Queue) {
    sch.stop.store(true, std::sync::atomic::Ordering::SeqCst);
    queue.request_stop();
}

/// The recorded mail provider, or `None` for an absent or unreadable answers file or no record.
pub fn recorded_mail_provider(data_dir: &Path) -> Option<String> {
    load_answers(data_dir).ok().flatten().and_then(|a| a.mail_provider)
}

/// A first pass the app never drives, so no batch will ever close and a handed-over item would never
/// clear (§14.51 r3): a Guided provider, or one whose backfill is held until C3.
pub fn backfill_never_driven(p: crate::mail_steps::Provider) -> bool {
    p == crate::mail_steps::Provider::Guided || crate::mail_steps::backfill_held_for_c3(p)
}

fn already_exiting() -> Value { serde_json::json!({ "ok": true, "exiting": true }) }
/// A relaunch (or a sign-out, or a finish write) that failed: the process stays, stopped, and the page
/// shows its one restart screen, whose button is `onboarding_quit` (§4.10, review r4 M5).
fn restart_needed(why: String) -> Value { serde_json::json!({ "ok": false, "restart": true, "error": why }) }
fn begin(g: &ExitGuard) -> bool { g.0.compare_exchange(false, true, std::sync::atomic::Ordering::SeqCst, std::sync::atomic::Ordering::SeqCst).is_ok() }
fn release(g: &ExitGuard) { g.0.store(false, std::sync::atomic::Ordering::SeqCst); }

/// Finish later and the window's close: stop, drain, save, stamp `quit_at`, flush, exit.
pub fn onboarding_quit_inner(s: &dyn ExitSeams, g: &ExitGuard) -> Value {
    if !begin(g) { return already_exiting(); }
    s.request_stop();
    s.drain();
    // A save that fails (a frozen queue after a failed sign-out) must not keep the process alive.
    if let Err(e) = s.save() { eprintln!("onboarding: the last save did not land: {e}"); }
    s.stamp_quit_at();
    s.flush();
    s.exit();
    serde_json::json!({ "ok": true, "error": Value::Null })
}

/// Stop, drain, save, freeze, sign out, relaunch `--pick`, exit; no flush. The sign-out (whose clean
/// runs `on_sign_out`) and the freeze come only after the drain has returned, so no writer is left to
/// put back what the rewind deleted (§14.51 r1, C1).
pub fn onboarding_sign_out_inner(s: &dyn ExitSeams, g: &ExitGuard) -> Value {
    if !begin(g) { return already_exiting(); }
    s.request_stop();
    s.drain();
    if let Err(e) = s.save() { eprintln!("onboarding: the last save did not land: {e}"); }
    s.freeze_saves();
    if let Err(e) = s.sign_out() { release(g); return restart_needed(e); }
    if let Err(e) = s.relaunch(&["--pick"]) { release(g); return restart_needed(e); }
    s.exit();
    serde_json::json!({ "ok": true, "error": Value::Null })
}

/// Why Open Knowlu is not offered yet, if it is not: a lane job that is not `done` (`config.ingest`
/// must be; the two feed stores may also be `failed`, which settles them for `config.ingest`, §4.11),
/// or a gate that is still closed.
fn finish_refusal(state: &OnboardingState, now_secs: i64) -> Option<String> {
    let st = |j: &str| state.jobs.get(j).map(|r| r.state);
    // The four local jobs must exist and be done: a crash between two enqueues can leave one unrecorded,
    // and finish deletes the only copy of the answers it would have read.
    let lane_open = ["school.apply", "school.courses", "registrar.confirm", "config.ingest"].iter().any(|j| st(j) != Some(JobState::Done))
        || ["school.store", "calendar.store"].iter().any(|j| st(j).is_some_and(|s| !matches!(s, JobState::Done | JobState::Failed | JobState::Skipped)));
    if lane_open { return Some("Knowlu is still saving your setup. Try again in a moment.".into()); }
    // Class times are the step 10 confirm (§4.6): nothing opens the reveal before it.
    if state.step10_confirmed_at.is_none() { return Some("Confirm your first week before Knowlu opens.".into()); }
    let confirmed = state.step10_confirmed_at.as_deref().and_then(|t| t.parse::<jiff::Timestamp>().ok()).map(|t| t.as_second());
    if !crate::onboarding_gate::gate(&gate_items(state), confirmed, now_secs).open {
        return Some("Knowlu is still reading your first week.".into());
    }
    None
}

/// Refuse while the gate is closed (`now_secs` against the cap) or a lane job is not `done`; then
/// stop, drain, save, `write_finish`, `relaunch_with`, and today's quit path.
pub fn onboarding_finish_inner(s: &dyn ExitSeams, g: &ExitGuard, now_secs: i64) -> Value {
    if !begin(g) { return already_exiting(); }
    let before = s.snapshot();
    // A second Open Knowlu after the first one landed has nothing left to write.
    if before.finished {
        release(g);
        return serde_json::json!({ "ok": true, "finished": true, "error": Value::Null });
    }
    if let Some(why) = finish_refusal(&before, now_secs) {
        release(g);
        return serde_json::json!({ "ok": false, "error": why });
    }
    s.request_stop();
    s.drain();
    let mut handover = handover_for(&s.snapshot());
    // Read before `write_finish` deletes the answers file. No provider recorded keeps the item.
    if s.mail_provider().is_some_and(|p| backfill_never_driven(crate::mail_steps::Provider::from_service(&p))) {
        handover.retain(|h| h.item != "mail.first_pass");
    }
    if let Err(e) = s.write_finish(handover) { release(g); return restart_needed(e); }
    s.stamp_quit_at();
    // The spawn before the flush, so the child waits on our pid while the flush runs (`switch_profile`).
    if let Err(e) = s.relaunch_with() { release(g); return restart_needed(e); }
    s.flush();
    s.exit();
    serde_json::json!({ "ok": true, "error": Value::Null })
}

/// F4's close handler: spawns [`onboarding_quit_inner`] on its own thread and returns at once, so the
/// event loop never waits on a drain.
pub fn close_requested(s: std::sync::Arc<dyn ExitSeams + Send + Sync>, g: std::sync::Arc<ExitGuard>) {
    let spawned = std::thread::Builder::new().name("onboarding-close".into()).spawn(move || { onboarding_quit_inner(&*s, &g); });
    if let Err(e) = spawned { eprintln!("onboarding: the close could not start its thread: {e}"); }
}

/// The reveal gate's items from the state: every required job under its own name (§4.6), and each
/// skip as a source the student did not connect. A held, running or pending job is still coming; a
/// failed one is named with its reason and, until F8g's closed set supplies one, the plain fix.
fn gate_items(state: &OnboardingState) -> Vec<crate::onboarding_gate::GateItem> {
    use crate::onboarding_gate::{GateItem, ItemState};
    let mut items: Vec<GateItem> = state.jobs.iter()
        .filter(|(name, r)| (SLOT_ITEMS.contains(&name.as_str()) || MAIL_ITEMS.contains(&name.as_str())) && r.state != JobState::Skipped)
        .map(|(name, r)| GateItem {
            key: name.clone(),
            state: match r.state {
                JobState::Done => ItemState::Done,
                JobState::Failed => ItemState::Failed { reason: r.reason.clone().unwrap_or_default(), fix: "Try again".into() },
                _ => ItemState::Pending,
            },
            connected: true,
            backfill: false,
        }).collect();
    // Class times: the step 10 confirm. Pending until it is stamped, so an empty list never opens the gate.
    items.push(GateItem {
        key: "class_times".into(),
        state: if state.step10_confirmed_at.is_some() { ItemState::Done } else { ItemState::Pending },
        connected: true,
        backfill: false,
    });
    items.extend(state.skips.iter().map(|k| GateItem {
        key: if k.source == "google" { "gmail".to_string() } else { k.source.clone() },
        state: ItemState::Skipped { cost_key: k.cost_key.clone() },
        connected: false,
        backfill: false,
    }));
    items
}

/// The drain's progress, as the page words it (§4.10): the running job, whether it is a slot (`long`)
/// and when the drain began.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ExitingNow { pub job: String, pub long: bool, pub since: String }

/// Every required item of §4.6 not `done`, each with its clearing signal.
pub fn handover_for(state: &OnboardingState) -> Vec<HandoverItem> {
    // A skipped job is the student's own choice (the gate lists it as not connected), never "coming".
    let open = |name: &str| state.jobs.get(name).is_some_and(|j| !matches!(j.state, JobState::Done | JobState::Skipped));
    let signal = |name: &str| {
        if SLOT_ITEMS.contains(&name) { Some(SIGNAL_SLOT_END) } else if MAIL_ITEMS.contains(&name) { Some(SIGNAL_MAIL_JSON) } else { None }
    };
    // `jobs` is a `BTreeMap`, so the walk is already sorted by item.
    state.jobs.keys().filter(|k| open(k)).filter_map(|k| signal(k).map(|s| HandoverItem { item: k.clone(), signal: s.to_string() })).collect()
}

/// `onboarding_status`'s body over a state that was read: the file plus the live job states, the
/// gate (re-evaluated at `now_secs`), and whether step 10's proposals may be asked. Keys: `ok`,
/// `jobs`, `proposals_ready`, `gate`, `step`, `skips`, `handover`, `account_settled`.
pub fn status_inner(state: &OnboardingState, now_secs: i64) -> Value {
    use serde_json::json;
    let confirmed = state.step10_confirmed_at.as_deref().and_then(|t| t.parse::<jiff::Timestamp>().ok()).map(|t| t.as_second());
    let g = crate::onboarding_gate::gate(&gate_items(state), confirmed, now_secs);
    json!({
        "ok": true, "error": Value::Null,
        "step": state.step, "account_settled": state.account_settled, "skips": state.skips, "handover": state.handover,
        "jobs": state.jobs, "proposals_ready": proposals_ready(state),
        "gate": {
            "open": g.open,
            "still_coming": g.still_coming,
            "failed": g.failed.iter().map(|(key, reason, fix)| json!({ "key": key, "reason": reason, "fix": fix })).collect::<Vec<_>>(),
            "not_connected": g.not_connected.iter().map(|n| json!({ "key": n.key, "cost_key": n.cost_key })).collect::<Vec<_>>(),
        },
    })
}

/// `onboarding_status` fails closed (§14.50): when the jobs could not be read the answer is
/// `{ok: false, error}` with no gate and no item list, because the gate opens on an empty list.
/// `answers` adds step 3's proof totals, `exiting` the drain's progress (`null` when none).
pub fn status_for(read: Result<OnboardingState, String>, answers: Option<&Answers>, exiting: Option<ExitingNow>, now_secs: i64) -> Value {
    let state = match read {
        Ok(s) => s,
        Err(e) => return serde_json::json!({ "ok": false, "error": e }),
    };
    let mut v = status_inner(&state, now_secs);
    v["step3"] = answers.map_or(Value::Null, |a| serde_json::json!(step3_totals(a)));
    v["exiting"] = exiting.map_or(Value::Null, |e| serde_json::json!({ "job": e.job, "long": e.long, "since": e.since }));
    v
}

// ---- the live seams and the commands (F2b; registered in F5) -------------------------------------

/// `ExitSeams` over the running app: the managed `Scheduler`, `Queue` and `ConsoleState`. Cheap to
/// build, so F4's close handler makes one per close. A missing piece (no queue in a normal console)
/// makes its seam a no-op, never a panic.
pub struct LiveExit { app: tauri::AppHandle }
impl LiveExit {
    pub fn new(app: tauri::AppHandle) -> LiveExit { LiveExit { app } }
}

impl ExitSeams for LiveExit {
    fn request_stop(&self) {
        use tauri::Manager;
        if let (Some(sch), Some(queue)) = (self.app.try_state::<crate::scheduler::Scheduler>(), self.app.try_state::<crate::onboarding_jobs::Queue>()) {
            stop_both(&sch, &queue);
        }
    }
    fn drain(&self) {
        use tauri::Manager;
        if let Some(queue) = self.app.try_state::<crate::onboarding_jobs::Queue>() { queue.drain(); }
    }
    fn save(&self) -> Result<(), String> {
        use tauri::Manager;
        self.app.try_state::<crate::onboarding_jobs::Queue>().map_or(Ok(()), |queue| queue.save())
    }
    fn freeze_saves(&self) {
        use tauri::Manager;
        if let Some(queue) = self.app.try_state::<crate::onboarding_jobs::Queue>() { queue.freeze(); }
    }
    fn sign_out(&self) -> Result<(), String> {
        use tauri::Manager;
        let cs = self.app.try_state::<crate::state::ConsoleState>().ok_or("Knowlu's setup is not running")?;
        let id = cs.settings.lock().unwrap_or_else(|e| e.into_inner()).profile_id.clone();
        crate::account::sign_out(self.app.clone(), Some(id)).map(|_| ())
    }
    fn relaunch(&self, args: &[&str]) -> Result<(), String> { crate::onboarding::relaunch(args.iter().copied()) }
    fn relaunch_with(&self) -> Result<(), String> {
        use tauri::Manager;
        let cs = self.app.try_state::<crate::state::ConsoleState>().ok_or("Knowlu's setup is not running")?;
        crate::onboarding::relaunch_with(&cs.vault)
    }
    fn stamp_quit_at(&self) {
        use tauri::Manager;
        let Some(cs) = self.app.try_state::<crate::state::ConsoleState>() else { return };
        let mut s = cs.settings.lock().unwrap_or_else(|e| e.into_inner());
        s.quit_at = Some(knowlu_engine::journal::now_ts(None));
        let _ = s.save(&cs.settings_path);
    }
    /// Today's quit path (push and back up under a 10 s cap); the process ends in its `then`, as
    /// `switch_profile`'s does, because the flush's worker is joined before the call returns.
    fn flush(&self) {
        use tauri::Manager;
        let Some(cs) = self.app.try_state::<crate::state::ConsoleState>() else { return };
        let logs = cs.data_dir.join("logs");
        crate::state::quit_flush(&cs, std::time::Duration::from_secs(10), move |q| {
            let _ = std::fs::create_dir_all(&logs);
            let stamp = knowlu_engine::journal::now_ts(None).replace(':', "");
            let _ = std::fs::write(logs.join(format!("quit-{stamp}.txt")), format!("synced={} backed_up={} timed_out={} onboarding=1\n", q.synced, q.backed_up, q.timed_out));
            std::process::exit(0);
        });
    }
    fn exit(&self) { self.app.exit(0); }
    fn snapshot(&self) -> OnboardingState {
        use tauri::Manager;
        // No queue reads as a state with nothing done, so Open Knowlu is refused.
        self.app.try_state::<crate::onboarding_jobs::Queue>().map_or_else(|| OnboardingState::new(&knowlu_engine::journal::now_ts(None), 0), |queue| queue.snapshot())
    }
    fn write_finish(&self, handover: Vec<HandoverItem>) -> Result<(), String> {
        use tauri::Manager;
        use tauri_plugin_autostart::ManagerExt;
        let queue = self.app.try_state::<crate::onboarding_jobs::Queue>().ok_or("Knowlu's setup is not running")?;
        let cs = self.app.try_state::<crate::state::ConsoleState>().ok_or("Knowlu's setup is not running")?;
        queue.finish(handover)?;
        {
            let _held = ANSWERS_LOCK.lock().unwrap_or_else(|p| p.into_inner());
            match std::fs::remove_file(answers_path(&cs.data_dir)) {
                Err(e) if e.kind() != std::io::ErrorKind::NotFound => eprintln!("onboarding: the answers file could not be deleted: {e}"),
                _ => {}
            }
        }
        let on = cs.settings.lock().unwrap_or_else(|e| e.into_inner()).autostart;
        let al = self.app.autolaunch();
        if let Err(e) = if on { al.enable() } else { al.disable() } { eprintln!("onboarding: autostart could not be set: {e}"); }
        Ok(())
    }
    fn mail_provider(&self) -> Option<String> {
        use tauri::Manager;
        self.app.try_state::<crate::state::ConsoleState>().and_then(|cs| recorded_mail_provider(&cs.data_dir))
    }
}

/// Finish later, and the window's close through F4. Async: the drain never runs on the event loop.
#[tauri::command(async)]
pub fn onboarding_quit(app: tauri::AppHandle) -> Value { onboarding_quit_inner(&LiveExit::new(app), &exit_guard()) }

/// Sign out of setup: drains, rewinds to step 2 and reopens the picker.
#[tauri::command(async)]
pub fn onboarding_sign_out(app: tauri::AppHandle) -> Value { onboarding_sign_out_inner(&LiveExit::new(app), &exit_guard()) }

/// Open Knowlu: refused until the gate opens and the lane is done; then hands over and relaunches.
#[tauri::command(async)]
pub fn onboarding_finish(app: tauri::AppHandle) -> Value {
    onboarding_finish_inner(&LiveExit::new(app), &exit_guard(), jiff::Timestamp::now().as_second())
}

fn no_setup() -> Value { serde_json::json!({ "ok": false, "error": "Knowlu's setup is not running" }) }

/// The file plus the live job states, the gate and step 3's totals; fails closed (§14.50). The drain's
/// progress (`exiting`) waits for F8f2's `Queue::exiting()`.
#[tauri::command]
pub fn onboarding_status(app: tauri::AppHandle) -> Value {
    use tauri::Manager;
    let (Some(queue), Some(cs)) = (app.try_state::<crate::onboarding_jobs::Queue>(), app.try_state::<crate::state::ConsoleState>()) else { return no_setup() };
    let answers = load_answers(&cs.data_dir).ok().flatten();
    status_for(Ok(queue.snapshot()), answers.as_ref(), None, jiff::Timestamp::now().as_second())
}

/// Records the answer in the answers file, enqueues its jobs, then wakes the queue.
#[tauri::command(async)]
pub fn onboarding_answer(app: tauri::AppHandle, step: u8, answer: Value) -> Value {
    use tauri::Manager;
    let (Some(queue), Some(cs)) = (app.try_state::<crate::onboarding_jobs::Queue>(), app.try_state::<crate::state::ConsoleState>()) else { return no_setup() };
    let out = answer_unless_exiting_with(&|| exiting_now(&queue, &exit_guard()), &cs.data_dir, step, &answer, &mut |job| queue.enqueue(job));
    // The jobs that wait for the answers file (`school.courses`, `registrar.confirm`, `config.ingest`)
    // are woken by nothing else: a saved answer wakes the queue even when it enqueued no job.
    if out["ok"] == true { queue.wake(); }
    out
}

/// A skip the step takes, recorded with its cost key in the live state and saved.
#[tauri::command]
pub fn onboarding_skip(app: tauri::AppHandle, step: u8, source: String) -> Value {
    use tauri::Manager;
    let Some(queue) = app.try_state::<crate::onboarding_jobs::Queue>() else { return no_setup() };
    skip_unless_exiting(exiting_now(&queue, &exit_guard()), &mut || queue.update_state(|s| skip_step(s, step, &source)).and_then(|r| r))
}

/// "Try again" on a job the page lists.
#[tauri::command]
pub fn onboarding_retry(app: tauri::AppHandle, job: String) -> Value {
    use tauri::Manager;
    let Some(queue) = app.try_state::<crate::onboarding_jobs::Queue>() else { return no_setup() };
    if !queue.snapshot().jobs.contains_key(&job) { return serde_json::json!({ "ok": false, "error": "that job is not part of this setup" }); }
    queue.retry(&job);
    serde_json::json!({ "ok": true, "error": Value::Null })
}

// ---- step 2 again, and step 4 (F2g, F2h) ---------------------------------------------------------

/// A local delete of the pending session and nothing else: never a revoke (review r6, I-1).
/// `credentials::delete`'s not-found result is ignored.
pub fn forget_pending_local(target: &str) {
    // Always asked to delete; not-found is the quiet case, and a failure is logged only if the credential is still there.
    if let Err(e) = crate::credentials::delete(target) {
        if crate::credentials::exists(target) { eprintln!("Knowlu: the pending sign-in could not be deleted ({e})"); }
    }
}

const REFUSED_OTHER_ACCOUNT: &str = "This setup belongs to another Knowlu account: sign in with that one, or choose another profile.";
const REFUSED_NO_SIGN_IN: &str = "Knowlu has no sign-in to attach: sign in again.";
const REFUSED_CLOUD_CONFIG: &str = "This profile's account settings cannot be read: sign in again, or choose another profile.";
const REFUSED_MOVE: &str = "Knowlu could not keep that sign-in on this profile: sign in again.";

/// Moves the pending session onto the vault's own target only when the pending session, the
/// sign-in's answer and `config/cloud.yaml` name one account. Otherwise `forget_pending` runs and the
/// answer is the named refusal (`wrong_account: true` where it applies). `after_attach` is the
/// best-effort entitlement refresh, a seam so a test makes no request. No sentence names an account.
pub fn attach_session_in(
    vault: &Path,
    pending_target: &str,
    expected_account_id: &str,
    forget_pending: &dyn Fn(),
    after_attach: &dyn Fn(),
) -> Value {
    attach_session_with(vault, pending_target, expected_account_id, forget_pending, after_attach, &|| {}, &crate::account::save_session)
}

/// [`attach_session_in`] with two more seams: `after_load` runs between the check and the write (a test
/// swaps the pending credential there) and `save` writes the profile's session (a test fails it).
/// **The session that was checked is the session that is saved**: it is held from the load, never
/// re-read from the pending target, so a sign-in finishing as another account in between cannot reach
/// the profile. The pending credential is deleted only after the write succeeds.
pub fn attach_session_with(
    vault: &Path,
    pending_target: &str,
    expected_account_id: &str,
    forget_pending: &dyn Fn(),
    after_attach: &dyn Fn(),
    after_load: &dyn Fn(),
    save: &dyn Fn(&str, &str, &crate::account::Session) -> Result<(), String>,
) -> Value {
    let refuse = |wrong_account: bool, why: &str| {
        forget_pending();
        serde_json::json!({ "ok": false, "wrong_account": wrong_account, "error": why })
    };
    let Ok((pending_id, session)) = crate::account::load_session(pending_target) else { return refuse(false, REFUSED_NO_SIGN_IN) };
    after_load();
    let cfg = match crate::account::cloud_config(vault) {
        Ok(c) if !c.session_credential_target.is_empty() && c.session_credential_target != pending_target => c,
        _ => return refuse(false, REFUSED_CLOUD_CONFIG),
    };
    if pending_id != expected_account_id || pending_id != cfg.account_id { return refuse(true, REFUSED_OTHER_ACCOUNT); }
    if save(&cfg.session_credential_target, &pending_id, &session).is_err() { return refuse(false, REFUSED_MOVE); }
    forget_pending_local(pending_target);
    after_attach();
    serde_json::json!({ "ok": true, "wrong_account": false, "error": Value::Null })
}

/// The command: passes `account::PENDING_TARGET` and the real seams.
#[tauri::command(async)]
pub fn onboarding_attach_session(cs: tauri::State<'_, crate::state::ConsoleState>, account_id: String) -> Value {
    attach_session_in(
        &cs.vault,
        crate::account::PENDING_TARGET,
        &account_id,
        &|| forget_pending_local(crate::account::PENDING_TARGET),
        &|| { let _ = crate::account::refresh_entitlement(&cs.vault, &cs.data_dir); },
    )
}

/// `account::refresh_entitlement` on the vault's own session, answered as `entitlement_now`'s
/// envelope (`ok`, `error`, `status`, `plan`, `current_period_end`).
pub fn entitlement_in(_vault: &Path, _data_dir: &Path) -> Value { todo!("F2h: onboarding_entitlement") }

/// The consent-then-checkout core on the vault's own session; returns the URL and opens nothing.
pub fn checkout_url_in(_vault: &Path, _plan: &str) -> Result<String, String> { todo!("F2h: onboarding_checkout") }

/// The `--courses` list `school.preview` passes to `knowlu-engine onboarding --preview` (§14.46): one
/// object per captured course (removed rows included, so their items are attributed and bucketed),
/// carrying **`code`: the row's [`CourseAnswer::key`]** (the engine reads `code` as the LMS key and
/// buckets by it), `slug`, `name`, `label`, `lms_key` and **`pairs`**, the row's saved `fragments`,
/// passed through byte for byte. So the engine's key (its `code`, then `slug`, then `label`) is the
/// app's key for every row, a row with no LMS key included. The label is display only.
///
/// **`pairs` is always present** (engine 4c9ca48: a present array, even an empty one, is the course's
/// whole fragment list, uid pins and class-time matching included). A row with saved `fragments`
/// sends exactly those, never `[]`; a row with none yet sends the hand-over's own fallback, its human
/// `code` alone (and `[]` only for a row with neither fragments nor a human code, for which the
/// hand-over writes no pair either) (as [`crate::onboarding_jobs::IngestHandoff::course_map`] does), so the preview and
/// `ingest.yaml` still read one list.
pub fn preview_courses(_a: &Answers) -> Value { todo!("F8b: the --courses list") }
