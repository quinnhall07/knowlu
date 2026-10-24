//! `onboarding.json`, `onboarding-answers.json`, resume, skips, the three exits and the account
//! commands of step 2 again and step 4 (plan §4.5, §4.7, §4.10; F2a-F2h).
//!
//! **F1 stub.** The types and signatures are the contract `app/tests/onboarding_state.rs` and
//! `app/tests/onboarding_jobs.rs` write against. Every function panics (`todo!`) until the F2 tasks
//! replace it, except the two path helpers, which are the plan's own words.
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
    pub fn slot_ended(&mut self) { todo!("F8f: clear the slot items") }
    /// A mail signal read from `mail.json`: clears only its own item (§4.10).
    pub fn mail_signal(&mut self, _signal: MailSignal) { todo!("F8f: clear the mail items") }
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
pub fn onboarding_answer_inner(
    _data_dir: &Path,
    _step: u8,
    _answer: &Value,
    _enqueue: &mut dyn FnMut(&str),
) -> Value {
    todo!("F2b: onboarding_answer")
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
    // Held across load, change and save, so a Remove and a `save_preview` never overwrite each other.
    let _held = ANSWERS_LOCK.lock().unwrap_or_else(|p| p.into_inner());
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
/// `onboarding.json` to step 2 (jobs pending, `account_settled` and `school` cleared, skips kept).
/// A finished file is left alone. `account::sign_out_clean` calls the real-store wrapper.
pub fn on_sign_out_with(data_dir: &Path, pending: &dyn PendingStore) -> Result<(), String> {
    // Every step runs; the failures are collected, so one stuck file never keeps the rest on the device.
    let mut failed: Vec<String> = Vec::new();
    let answers = answers_path(data_dir);
    match std::fs::remove_file(&answers) {
        Ok(()) => {}
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => {}
        Err(e) => failed.push(format!("{}: could not be deleted ({e})", answers.display())),
    }
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
}

/// One exit at a time: a second exit while one drains answers `{ok: true, exiting: true}`.
#[derive(Default)]
pub struct ExitGuard(pub std::sync::atomic::AtomicBool);

/// Finish later and the window's close: stop, drain, save, stamp `quit_at`, flush, exit.
pub fn onboarding_quit_inner(_s: &dyn ExitSeams, _g: &ExitGuard) -> Value { todo!("F2b: onboarding_quit") }
/// Stop, drain, save, freeze, sign out, relaunch `--pick`, exit; no flush.
pub fn onboarding_sign_out_inner(_s: &dyn ExitSeams, _g: &ExitGuard) -> Value { todo!("F2b: onboarding_sign_out") }
/// Refuse while the gate is closed (`now_secs` against the cap) or a lane job is not `done`; then
/// stop, drain, save, `write_finish`, `relaunch_with`, and today's quit path.
pub fn onboarding_finish_inner(_s: &dyn ExitSeams, _g: &ExitGuard, _now_secs: i64) -> Value {
    todo!("F2b: onboarding_finish")
}
/// F4's close handler: spawns [`onboarding_quit_inner`] on its own thread and returns at once.
pub fn close_requested(_s: std::sync::Arc<dyn ExitSeams + Send + Sync>, _g: std::sync::Arc<ExitGuard>) {
    todo!("F2b: the close never waits on the event loop")
}
/// Every required item of §4.6 not `done`, each with its clearing signal.
pub fn handover_for(_state: &OnboardingState) -> Vec<HandoverItem> { todo!("F8f: the hand-over list") }

/// `onboarding_status`'s body: the file plus the live job states, the gate, and whether step 10's
/// proposals may be asked (only once `config.ingest` is `done`). Keys: `jobs`, `proposals_ready`,
/// `gate`.
pub fn status_inner(_state: &OnboardingState, _now_secs: i64) -> Value { todo!("F2b: onboarding_status") }

// ---- step 2 again, and step 4 (F2g, F2h) ---------------------------------------------------------

/// A local delete of the pending session and nothing else: never a revoke (review r6, I-1).
/// `credentials::delete`'s not-found result is ignored.
pub fn forget_pending_local(_target: &str) { todo!("F2g: the local forget") }

/// Moves the pending session onto the vault's own target only when the pending session, the
/// sign-in's answer and `config/cloud.yaml` name one account. Otherwise `forget_pending` runs and the
/// answer is the named refusal (`wrong_account: true` where it applies). `after_attach` is the
/// best-effort entitlement refresh, a seam so a test makes no request.
pub fn attach_session_in(
    _vault: &Path,
    _pending_target: &str,
    _expected_account_id: &str,
    _forget_pending: &dyn Fn(),
    _after_attach: &dyn Fn(),
) -> Value {
    todo!("F2g: attach the session")
}

/// The command: passes `account::PENDING_TARGET` and the real seams.
#[tauri::command(async)]
pub fn onboarding_attach_session(_cs: tauri::State<'_, crate::state::ConsoleState>, _account_id: String) -> Value {
    todo!("F2g: onboarding_attach_session")
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
