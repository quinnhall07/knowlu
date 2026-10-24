//! O3 F1: the job queue, the gate, the exits, the feed and course runners, the link commands that only
//! validate (plan tests 3-6, 8, 9, 11-14, 16, 17's exits, 19, 23-26). No window, no network, no engine
//! child: the queue takes its runner as a trait and every test passes a fake. Test numbers are the
//! plan's (`docs/plans/2026-10-23-wave-b-long-onboarding-plan.md`, F1). Invented data only.
use knowlu::onboarding_gate::*;
use knowlu::onboarding_jobs::*;
use knowlu::onboarding_state::*;
use knowlu::scheduler::SlotScope;
use serde_json::Value;
use std::cell::{Cell, RefCell};
use std::collections::{BTreeMap, VecDeque};
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};
use std::sync::{Arc, Condvar, Mutex};
use std::time::{Duration, Instant};

fn scratch(tag: &str) -> PathBuf {
    let d = std::env::temp_dir().join(format!("knowlu-o3-jobs-{tag}-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&d);
    std::fs::create_dir_all(&d).unwrap();
    d
}

// ---- the fakes -------------------------------------------------------------------------------------------

#[derive(Clone, Default)]
struct Log(Arc<Mutex<Vec<String>>>);
impl Log {
    fn push(&self, s: impl Into<String>) { self.0.lock().unwrap().push(s.into()); }
    fn all(&self) -> Vec<String> { self.0.lock().unwrap().clone() }
    fn pos(&self, s: &str) -> Option<usize> { self.all().iter().position(|e| e == s) }
    /// `start:<job>` events, in order, as job names.
    fn started(&self) -> Vec<String> { self.all().iter().filter_map(|e| e.strip_prefix("start:").map(str::to_string)).collect() }
    fn ran(&self, job: &str) -> bool { self.pos(&format!("start:{job}")).is_some() }
    /// The scope each run of `job` was handed.
    fn scopes(&self, job: &str) -> Vec<String> {
        self.all().iter().filter_map(|e| e.strip_prefix(&format!("scope:{job}:")).map(str::to_string)).collect()
    }
    /// Engine steps the fake slots ran, in order, as `(job, step)`.
    fn steps(&self) -> Vec<(String, String)> {
        self.all().iter().filter_map(|e| {
            let rest = e.strip_prefix("step:")?;
            let (job, step) = rest.split_once(':')?;
            Some((job.to_string(), step.to_string()))
        }).collect()
    }
    /// The fakes' own tripwires: overlap, or a job on the wrong thread.
    fn assert_clean(&self) {
        let bad: Vec<String> = self.all().into_iter().filter(|e| e.starts_with("OVERLAP:") || e.starts_with("MISROUTED:")).collect();
        assert!(bad.is_empty(), "{bad:?}");
    }
}

struct Gate { open: Mutex<bool>, cv: Condvar }
impl Gate {
    fn new() -> Arc<Gate> { Arc::new(Gate { open: Mutex::new(false), cv: Condvar::new() }) }
    fn release(&self) { *self.open.lock().unwrap() = true; self.cv.notify_all(); }
    /// True when released; false when the stop was seen first and `honor_stop` is set.
    fn wait(&self, stop: &AtomicBool, honor_stop: bool) -> bool {
        let deadline = Instant::now() + Duration::from_secs(30);
        let mut g = self.open.lock().unwrap();
        // The stop is looked at before the gate, so a release that lands in the same instant as the
        // stop still reads as "the job noticed the stop" (the helper is the test's, not an assertion).
        loop {
            if honor_stop && stop.load(Ordering::SeqCst) { return false; }
            if *g { return true; }
            assert!(Instant::now() < deadline, "a fake gate was never released");
            g = self.cv.wait_timeout(g, Duration::from_millis(5)).unwrap().0;
        }
    }
}

const LANE_JOBS: [&str; 6] = ["school.store", "calendar.store", "school.apply", "school.courses", "registrar.confirm", "config.ingest"];
const NOTE_JOBS: [&str; 4] = ["school.courses", "registrar.confirm", "slot.partial", "slot.first_full"];

/// Sets a flag while held and records an overlap if it was already set.
struct Busy<'a>(&'a AtomicBool);
impl<'a> Busy<'a> {
    fn enter(flag: &'a AtomicBool, log: &Log, what: &str) -> Busy<'a> {
        if flag.swap(true, Ordering::SeqCst) { log.push(format!("OVERLAP:{what}")); }
        Busy(flag)
    }
}
impl Drop for Busy<'_> { fn drop(&mut self) { self.0.store(false, Ordering::SeqCst); } }

fn steps_of(scope: SlotScope) -> &'static [&'static str] {
    match scope {
        SlotScope::Full => &["sync", "coursework", "grades", "ingest", "judge", "rank"],
        SlotScope::NoRank => &["sync", "coursework", "grades", "ingest", "judge"],
        SlotScope::BeforeConfig => &["sync"],
    }
}

type Behavior = Arc<dyn Fn() -> Outcome + Send + Sync>;

#[derive(Default)]
struct Fake {
    log: Log,
    /// job -> (gate, does the job notice the stop)
    blocks: Mutex<BTreeMap<String, (Arc<Gate>, bool)>>,
    /// `<job>/<step>` -> gate a fake slot waits on inside that step.
    step_blocks: Mutex<BTreeMap<String, Arc<Gate>>>,
    behaviors: Mutex<BTreeMap<String, Behavior>>,
    serial_busy: AtomicBool,
    notes_busy: AtomicBool,
    clock: AtomicUsize,
}
impl Fake {
    fn block(&self, job: &str, honor_stop: bool) -> Arc<Gate> {
        let g = Gate::new();
        self.blocks.lock().unwrap().insert(job.to_string(), (g.clone(), honor_stop));
        g
    }
    fn block_step(&self, job: &str, step: &str) -> Arc<Gate> {
        let g = Gate::new();
        self.step_blocks.lock().unwrap().insert(format!("{job}/{step}"), g.clone());
        g
    }
    fn behave(&self, job: &str, f: impl Fn() -> Outcome + Send + Sync + 'static) {
        self.behaviors.lock().unwrap().insert(job.to_string(), Arc::new(f));
    }
    fn next_started(&self) -> String {
        let n = self.clock.fetch_add(1, Ordering::SeqCst);
        format!("2026-10-24T12:{:02}:{:02}.000Z", n / 60, n % 60)
    }
}

impl Runner for Fake {
    fn run(&self, job: &str, scope: Option<SlotScope>, stop: &AtomicBool) -> Outcome {
        let thread = std::thread::current().name().unwrap_or("").to_string();
        let lane = LANE_JOBS.contains(&job);
        self.log.push(format!("start:{job}"));
        if let Some(s) = scope { self.log.push(format!("scope:{job}:{s:?}")); }
        // A lane job must run on the lane's thread and nothing else may: the lane never starts an
        // engine child or a window (a serial job here would).
        if lane != (thread == LANE_THREAD) { self.log.push(format!("MISROUTED:{job}:{thread}")); }
        let _serial = (!lane).then(|| Busy::enter(&self.serial_busy, &self.log, job));
        let _notes = NOTE_JOBS.contains(&job).then(|| Busy::enter(&self.notes_busy, &self.log, job));
        let block = self.blocks.lock().unwrap().get(job).cloned();
        if let Some((gate, honor)) = block {
            if !gate.wait(stop, honor) {
                self.log.push(format!("abandoned:{job}"));
                return Outcome::Stopped;
            }
        }
        let behavior = self.behaviors.lock().unwrap().get(job).cloned();
        let out = if let Some(b) = behavior {
            b()
        } else if let Some(scope) = scope {
            let started = self.next_started();
            let mut ran = Vec::new();
            for step in steps_of(scope) {
                if stop.load(Ordering::SeqCst) {
                    self.log.push(format!("stopped_before:{job}:{step}"));
                    self.log.push(format!("end:{job}"));
                    return Outcome::Stopped;
                }
                self.log.push(format!("step:{job}:{step}"));
                let gate = self.step_blocks.lock().unwrap().get(&format!("{job}/{step}")).cloned();
                if let Some(g) = gate { g.wait(stop, false); }
                ran.push((step.to_string(), 0));
            }
            Outcome::Slot(SlotRun { started, steps: ran, reason: None })
        } else {
            std::thread::sleep(Duration::from_millis(3));
            Outcome::Done(BTreeMap::new())
        };
        self.log.push(format!("end:{job}"));
        out
    }
}

/// `.0` entitled, `.1` the number of `refresh` calls.
struct Ent(AtomicBool, AtomicUsize);
impl Entitlement for Ent {
    fn entitled(&self) -> bool { self.0.load(Ordering::SeqCst) }
    fn refresh(&self) { self.1.fetch_add(1, Ordering::SeqCst); }
}

/// A status `at` the fake stamps 1 ms after the slot's own `started`, in the journal's own format.
const FRESH: &str = "@fresh";
const STALE: &str = "2000-01-01T00:00:00.000Z";

/// A real `journal::now_ts` timestamp 1 ms later than `started`, so a reading counts as this slot's
/// only if the queue compares timestamps, not strings it could match by prefix.
fn one_ms_after(started: &str) -> String {
    let t: jiff::Timestamp = started.parse().unwrap_or_else(|e| panic!("{started}: {e}"));
    knowlu_engine::journal::now_ts(Some(t.checked_add(jiff::SignedDuration::from_millis(1)).unwrap()))
}

fn status(at: &str, ok: bool, skipped: Option<&str>) -> SyncStatus {
    SyncStatus { at: at.into(), ok, skipped: skipped.map(str::to_string), last_error: (!ok && skipped.is_none()).then(|| "transport".to_string()) }
}
fn cursor(pulled_to_end: bool, record: i64, note: i64) -> SyncCursor { SyncCursor { pulled_to_end, record_cursor: record, note_cursor: note } }
type Reading = (Option<SyncStatus>, Option<SyncCursor>);
/// A fresh, unskipped, `ok` reading.
fn ok_reading(pulled_to_end: bool, record: i64, note: i64) -> Reading { (Some(status(FRESH, true, None)), Some(cursor(pulled_to_end, record, note))) }

/// One reading per slot the queue reads; once they run out, a stale one (no status at all).
struct Syncs(Mutex<VecDeque<Reading>>);
impl SyncView for Syncs {
    fn read(&self, slot_started: &str) -> Reading {
        let next = self.0.lock().unwrap().pop_front();
        match next {
            Some((Some(mut s), c)) => {
                if s.at == FRESH { s.at = one_ms_after(slot_started); }
                (Some(s), c)
            }
            Some((None, c)) => (None, c),
            None => (None, None),
        }
    }
}

struct Rig {
    q: Queue,
    log: Log,
    fake: Arc<Fake>,
    ent: Arc<Ent>,
    dir: PathBuf,
}

fn state_at(step: u8, settled: bool) -> OnboardingState {
    let mut s = OnboardingState::new("2026-10-24T09:00:00Z", step);
    s.account_settled = settled;
    s
}

/// `state_at` with `school.apply` already done: `registrar.confirm` waits for it (F8h), because the
/// school it writes under comes from the `campus.yaml` that job creates.
fn state_applied(step: u8, settled: bool) -> OnboardingState {
    let mut s = state_at(step, settled);
    s.jobs.insert("school.apply".into(), JobRecord { state: JobState::Done, ..Default::default() });
    s
}

/// A rig as a real run looks at step 4 or later: the answers file holds the three captured courses
/// (no feeds) and the school is confirmed. A queue that gates on either would otherwise be held for
/// good, and one that ignored them would pass every test below unproven.
fn rig_with(tag: &str, state: OnboardingState, entitled: bool, readings: Vec<Reading>, setup: impl FnOnce(&Fake)) -> Rig {
    rig_full(tag, state, entitled, readings, true, setup)
}
fn rig_full(tag: &str, mut state: OnboardingState, entitled: bool, readings: Vec<Reading>, answers: bool, setup: impl FnOnce(&Fake)) -> Rig {
    let dir = scratch(tag);
    if answers { save_answers(&dir, &Answers { courses: three_courses(), ..Default::default() }).unwrap(); }
    if state.school.is_none() {
        state.school = Some(School { unitid: Some("900001".into()), label: "Example Test College".into(), path: SchoolPath::Shared, confirmed: true });
    }
    let fake = Arc::new(Fake::default());
    setup(&fake);
    let ent = Arc::new(Ent(AtomicBool::new(entitled), AtomicUsize::new(0)));
    let deps = Deps {
        data_dir: dir.clone(),
        runner: fake.clone(),
        entitlement: ent.clone(),
        sync_view: Arc::new(Syncs(Mutex::new(readings.into()))),
    };
    let q = Queue::start(deps, state);
    Rig { log: fake.log.clone(), q, fake, ent, dir }
}
fn rig(tag: &str, state: OnboardingState, entitled: bool) -> Rig { rig_with(tag, state, entitled, vec![ok_reading(true, 10, 10)], |_| {}) }

impl Rig {
    fn job(&self, name: &str) -> JobRecord { self.q.snapshot().jobs.get(name).cloned().unwrap_or_else(|| panic!("no job {name}")) }
    fn state_of(&self, name: &str) -> JobState { self.job(name).state }
    fn idle(&self) { assert!(self.q.wait_idle(Duration::from_secs(20)), "the queue never went idle: {:?}", self.log.all()); }
    fn wait_for(&self, what: &str, cond: impl Fn(&Rig) -> bool) {
        let deadline = Instant::now() + Duration::from_secs(20);
        while !cond(self) {
            assert!(Instant::now() < deadline, "timed out waiting for {what}: {:?}", self.log.all());
            std::thread::sleep(Duration::from_millis(5));
        }
    }
    fn wait_started(&self, job: &str) { self.wait_for(&format!("{job} to start"), |r| r.log.ran(job)); }
    fn wait_log(&self, event: &str) { self.wait_for(event, |r| r.log.pos(event).is_some()); }
}

// ---- test 3: holds -----------------------------------------------------------------------------------------

#[test]
fn test_03_without_an_entitlement_every_gated_job_is_held_and_never_started_then_runs_in_enqueue_order() {
    let r = rig("t03", state_at(4, true), false);
    // `slot.first_full` is not enqueued by hand: step 10's confirm asks for it, after the entitlement.
    let gated = ["school.store", "calendar.store", "school.apply", "school.courses", "registrar.confirm", "config.ingest", "slot.partial"];
    for j in ["school.capture", "school.preview"].into_iter().chain(gated) { r.q.enqueue(j); }
    r.wait_for("the device-side jobs", |r| r.state_of("school.preview") == JobState::Done);
    r.idle();
    assert_eq!(r.state_of("school.capture"), JobState::Done, "school.capture needs no entitlement");
    for j in gated {
        assert_eq!(r.state_of(j), JobState::Held, "{j} is held, not failed and not pending");
        assert!(!r.log.ran(j), "{j} was started while not entitled");
    }
    assert_eq!(r.ent.1.load(Ordering::SeqCst), 1, "one entitlement refresh at queue start, none per job");
    r.log.push("entitled");
    r.ent.0.store(true, Ordering::SeqCst);
    r.q.wake();
    r.idle();
    r.q.confirm_step10("2026-10-24T12:00:00Z");
    r.idle();
    let at = r.log.pos("entitled").unwrap();
    let started = r.log.started();
    let lane: Vec<&String> = started.iter().filter(|j| LANE_JOBS.contains(&j.as_str())).collect();
    assert_eq!(lane, LANE_JOBS.iter().collect::<Vec<_>>(), "the held lane jobs run in the order they were enqueued");
    for j in gated {
        assert!(r.log.pos(&format!("start:{j}")).unwrap() > at, "{j} started after the entitlement");
    }
    let serial: Vec<&String> = started.iter().filter(|j| j.starts_with("slot.")).collect();
    assert_eq!(serial, ["slot.partial", "slot.first_full"].iter().collect::<Vec<_>>(), "the full slot comes from step 10's confirm");
    assert_eq!(r.q.snapshot().step10_confirmed_at.as_deref(), Some("2026-10-24T12:00:00Z"));
    r.log.assert_clean();
}

// ---- test 5: serial -------------------------------------------------------------------------------------------

#[test]
fn test_05_two_serial_jobs_never_run_at_once() {
    let r = rig("t05", state_at(5, true), true);
    let jobs = ["school.capture", "school.preview", "coursework.discover", "mail.setup", "mail.canary", "mail.first_pass", "google.gmail_first_read", "slot.partial"];
    for j in jobs { r.q.enqueue(j); }
    r.idle();
    r.log.assert_clean();
    for j in jobs { assert!(r.log.ran(j), "{j} never ran: {:?}", r.log.all()); }
}

// ---- test 6: the gate (pure) ------------------------------------------------------------------------------------

fn item(key: &str, state: ItemState) -> GateItem { GateItem { key: key.into(), state, connected: true, backfill: false } }
fn pending(key: &str) -> GateItem { item(key, ItemState::Pending) }

#[test]
fn test_06_the_gate_opens_when_every_required_item_is_done_failed_or_skipped() {
    let t0 = Some(1_000);
    let mut items = vec![item("first.sync", ItemState::Done), item("first.grades", ItemState::Done), pending("effort_ranking")];
    let g = gate(&items, t0, 1_010);
    assert!(!g.open, "an item is still pending");
    assert_eq!(g.still_coming, ["effort_ranking"]);
    items[2] = item("effort_ranking", ItemState::Failed { reason: "ranking_failed".into(), fix: "Try again".into() });
    items.push(item("gmail", ItemState::Skipped { cost_key: "gmail_missed".into() }));
    let g = gate(&items, t0, 1_010);
    assert!(g.open, "done, failed and skipped all count");
    assert_eq!(g.failed, [("effort_ranking".to_string(), "ranking_failed".to_string(), "Try again".to_string())], "a failed item is listed with its reason, never hidden");
    assert!(g.still_coming.is_empty());
}

#[test]
fn test_06_the_gate_opens_at_exactly_three_minutes_after_the_confirm_with_the_rest_still_coming() {
    let items = [item("first.sync", ItemState::Done), pending("first.grades"), pending("effort_ranking")];
    let confirmed = 5_000;
    assert_eq!(CAP_SECS, 180);
    let before = gate(&items, Some(confirmed), confirmed + 179);
    assert!(!before.open, "one second early");
    let at = gate(&items, Some(confirmed), confirmed + 180);
    assert!(at.open, "exactly 3:00");
    assert_eq!(at.still_coming, ["first.grades", "effort_ranking"], "the rest are listed as still coming");
    assert!(!gate(&items, None, confirmed + 10_000).open, "no confirm yet, no cap");
}

#[test]
fn test_06_a_backfill_is_listed_but_never_waited_for_and_an_unconnected_source_is_not_required() {
    let mut backfill = pending("backfill.gmail_120_days");
    backfill.backfill = true;
    let mut unconnected = pending("mail");
    unconnected.connected = false;
    let items = [item("first.sync", ItemState::Done), backfill, unconnected];
    let g = gate(&items, Some(0), 1);
    assert!(g.open, "a backfill in progress and an unconnected source do not hold the gate");
    assert_eq!(g.still_coming, ["backfill.gmail_120_days"], "the backfill is listed; the unconnected source is not");
}

#[test]
fn test_06_skips_are_listed_with_their_cost_keys() {
    let items = [
        item("first.sync", ItemState::Done),
        item("mail", ItemState::Skipped { cost_key: "mail_dates_missed".into() }),
        item("gmail", ItemState::Skipped { cost_key: "gmail_and_calendar_missed".into() }),
    ];
    let g = gate(&items, Some(0), 1);
    assert!(g.open);
    assert_eq!(
        g.not_connected,
        [
            NotConnected { key: "mail".into(), cost_key: "mail_dates_missed".into() },
            NotConnected { key: "gmail".into(), cost_key: "gmail_and_calendar_missed".into() },
        ]
    );
}

#[test]
fn test_06_a_grades_not_yet_row_never_marks_first_grades_and_a_skipped_row_leaves_it_pending() {
    let row = |n: &str| vec![("sync".to_string(), 0), (n.to_string(), 0)];
    assert!(!first_grades_marked(&row("grades (not yet: your courses are not in yet)")));
    assert!(!first_grades_marked(&row("grades (skipped: not connected)")));
    assert!(!first_grades_marked(&row("grades (skipped: Blackboard unreachable)")));
    assert!(first_grades_marked(&row("grades")), "a slot whose grades ran marks it");
    assert!(!first_grades_marked(&[("sync".to_string(), 0)]), "a slot with no grades row does not");
}

// ---- test 4: slot order and scope -------------------------------------------------------------------------------

// The step order below is the FAKE's own (`steps_of`): this test pins the scope each slot is handed and the
// order the queue hands them in. The real argv order of a slot's steps is pinned by test 18 in scheduler.rs.
const CANON: [&str; 6] = ["sync", "coursework", "grades", "ingest", "judge", "rank"];

/// Enqueues the whole onboarding in `order` (account unsettled, so the first slot's `sync` settles it),
/// waits for `config.ingest`, asks for a second slot, then confirms step 10.
fn run_whole_onboarding(tag: &str, order: &[&str]) -> Rig {
    let r = rig_with(tag, state_at(4, false), true, vec![ok_reading(true, 10, 10), ok_reading(true, 10, 10), ok_reading(true, 10, 10)], |_| {});
    for j in order { r.q.enqueue(j); }
    r.wait_for("config.ingest", |r| r.state_of("config.ingest") == JobState::Done);
    r.q.enqueue("slot.partial");
    r.idle();
    r.log.push("confirm");
    r.q.confirm_step10("2026-10-24T12:40:00Z");
    r.idle();
    r
}

#[test]
fn test_04_whatever_order_the_asks_were_answered_in_a_slots_steps_keep_the_canonical_order() {
    let orders: [&[&str]; 3] = [
        &["school.apply", "school.courses", "slot.partial", "config.ingest", "registrar.confirm"],
        &["config.ingest", "registrar.confirm", "school.courses", "school.apply", "slot.partial"],
        &["slot.partial", "registrar.confirm", "config.ingest", "school.apply", "school.courses"],
    ];
    for (i, order) in orders.iter().enumerate() {
        let r = run_whole_onboarding(&format!("t04-{i}"), order);
        r.log.assert_clean();
        let all = r.log.all();
        // Per slot run, the steps appear in the canonical relative order.
        let mut per_run: Vec<Vec<String>> = Vec::new();
        for e in &all {
            if e.starts_with("start:slot.") { per_run.push(Vec::new()); }
            if let Some(step) = e.strip_prefix("step:").and_then(|x| x.split_once(':')).map(|(_, s)| s.to_string()) {
                per_run.last_mut().expect("a step inside a slot").push(step);
            }
        }
        assert!(per_run.len() >= 3, "a before-config slot, a no-rank slot and the first full slot ran: {all:?}");
        for steps in &per_run {
            let idx: Vec<usize> = steps.iter().map(|s| CANON.iter().position(|c| c == s).unwrap()).collect();
            assert!(idx.windows(2).all(|w| w[0] < w[1]), "steps out of order: {steps:?}");
        }
        // `rank` only in slot.first_full, only after step 10's confirm, and never before the first ingest.
        let ranks: Vec<usize> = all.iter().enumerate().filter(|(_, e)| e.ends_with(":rank")).map(|(i, _)| i).collect();
        assert_eq!(ranks.len(), 1, "exactly one rank: {all:?}");
        assert!(all[ranks[0]].starts_with("step:slot.first_full:"), "{}", all[ranks[0]]);
        assert!(ranks[0] > r.log.pos("confirm").unwrap());
        let first_ingest = all.iter().position(|e| e.ends_with(":ingest")).expect("an ingest ran");
        assert!(ranks[0] > first_ingest);
    }
}

#[test]
fn test_04_a_partial_slot_is_handed_before_config_until_config_ingest_is_done_and_no_rank_after() {
    let r = run_whole_onboarding("t04-scopes", &["school.apply", "school.courses", "config.ingest", "slot.partial"]);
    let scopes = r.log.scopes("slot.partial");
    assert_eq!(scopes, ["BeforeConfig", "NoRank"], "the scope each slot.partial was handed");
    assert_eq!(r.log.scopes("slot.first_full"), ["Full"]);
}

// ---- test 8: config before its readers --------------------------------------------------------------------------

fn pos_of(r: &Rig, e: &str) -> usize { r.log.pos(e).unwrap_or_else(|| panic!("no {e}: {:?}", r.log.all())) }

#[test]
fn test_08_no_reader_of_the_config_starts_before_config_ingest_and_the_jobs_wait_for_their_inputs() {
    let r = rig_with("t08", state_at(4, false), true, vec![ok_reading(true, 10, 10), ok_reading(true, 10, 10), ok_reading(true, 10, 10)], |_| {});
    // A deliberately scrambled order.
    for j in ["calendar.read", "google.calendar_read", "config.ingest", "slot.partial", "registrar.confirm", "school.courses", "calendar.store", "school.store", "school.apply"] {
        r.q.enqueue(j);
    }
    r.wait_for("config.ingest", |r| r.state_of("config.ingest") == JobState::Done);
    r.q.enqueue("slot.partial");
    r.idle();
    r.log.assert_clean();
    let ci_end = pos_of(&r, "end:config.ingest");
    for reader in ["calendar.read", "google.calendar_read"] {
        assert!(pos_of(&r, &format!("start:{reader}")) > ci_end, "{reader} started before config.ingest was done");
    }
    assert!(pos_of(&r, "start:config.ingest") > pos_of(&r, "end:school.courses"), "config.ingest waits for school.courses");
    for store in ["school.store", "calendar.store"] {
        assert!(pos_of(&r, "start:config.ingest") > pos_of(&r, &format!("end:{store}")), "config.ingest waits for {store} to settle");
    }
    assert!(pos_of(&r, "start:registrar.confirm") > pos_of(&r, "end:school.courses"), "registrar.confirm waits for school.courses");
    // `ingest` and `coursework` (engine steps) appear only in a slot that started after config.ingest;
    // `grades` only after school.apply and school.courses are both done, so never in a BeforeConfig slot.
    for (job, step) in r.log.steps() {
        let at = r.log.all().iter().position(|e| e == &format!("step:{job}:{step}")).unwrap();
        if matches!(step.as_str(), "coursework" | "ingest") { assert!(at > ci_end, "{step} ran before config.ingest was done"); }
        if step == "grades" {
            assert!(at > pos_of(&r, "end:school.apply") && at > pos_of(&r, "end:school.courses"), "grades ran before the course notes exist");
        }
    }
    assert_eq!(r.log.scopes("slot.partial"), ["BeforeConfig", "NoRank"]);
}

#[test]
fn test_08_school_courses_does_not_start_before_the_account_is_settled_and_config_ingest_waits_for_it() {
    // The account is never reached: every reading is a skip, so the chain stops and the notes never start.
    let r = rig_with("t08-unsettled", state_at(4, false), true, vec![], |_| {});
    for j in ["config.ingest", "school.courses", "school.apply", "slot.partial"] { r.q.enqueue(j); }
    r.idle();
    assert!(!r.log.ran("school.courses"), "no school.courses before account_settled: {:?}", r.log.all());
    assert!(!r.log.ran("config.ingest"), "no config.ingest before school.courses");
    assert!(r.log.ran("school.apply"), "school.apply does not need the account");
    assert_eq!(r.job("school.courses").state, JobState::Failed, "the chain stopped; Try again is the student's");
}

// ---- test 11: the local lane --------------------------------------------------------------------------------------

fn proposals_ready(r: &Rig) -> bool {
    let v: Value = status_inner(&r.q.snapshot(), 0);
    v["proposals_ready"].as_bool().unwrap_or_else(|| panic!("status has no proposals_ready: {v}"))
}

#[test]
fn test_11_config_ingest_runs_and_finishes_while_a_canary_holds_the_serial_queue() {
    let r = rig("t11", state_at(8, true), true);
    let canary = r.fake.block("mail.canary", false);
    let config = r.fake.block("config.ingest", false);
    for j in ["school.apply", "school.courses"] { r.q.enqueue(j); }
    r.wait_for("the notes", |r| r.state_of("school.courses") == JobState::Done);
    r.q.enqueue("mail.canary");
    r.wait_started("mail.canary");
    assert!(!proposals_ready(&r), "config.ingest has not been asked for yet");
    // Leaving step 8: config.ingest is enqueued while the canary holds the queue.
    r.q.enqueue("config.ingest");
    r.wait_started("config.ingest");
    assert_eq!(r.state_of("config.ingest"), JobState::Running);
    assert!(!proposals_ready(&r), "not ready while config.ingest runs");
    config.release();
    r.wait_for("config.ingest to finish", |r| r.state_of("config.ingest") == JobState::Done);
    assert!(r.log.pos("end:mail.canary").is_none(), "config.ingest did not wait for the canary's three minutes");
    assert!(proposals_ready(&r), "ready once config.ingest is done");
    canary.release();
    r.idle();
    r.log.assert_clean();
}

#[test]
fn test_11_step_10s_proposals_are_not_ready_while_config_ingest_is_pending() {
    let r = rig("t11-pending", state_at(8, true), false);
    r.q.enqueue("config.ingest");
    r.idle();
    assert!(matches!(r.state_of("config.ingest"), JobState::Held | JobState::Pending));
    assert!(!proposals_ready(&r));
}

// ---- the exits' seams (tests 9, 17, 19, 25) -----------------------------------------------------------------------

#[derive(Default)]
struct FakePending(Mutex<BTreeMap<String, String>>);
impl PendingStore for FakePending {
    fn put(&self, name: &str, link: &str) -> Result<(), String> { self.0.lock().unwrap().insert(name.into(), link.into()); Ok(()) }
    fn get(&self, name: &str) -> Option<String> { self.0.lock().unwrap().get(name).cloned() }
    fn delete(&self, name: &str) -> Result<(), String> { self.0.lock().unwrap().remove(name); Ok(()) }
}

/// The exit seams over a real queue: the process-level ones are fakes that only log.
struct Seams<'a> {
    q: &'a Queue,
    log: Log,
    dir: PathBuf,
    pending: FakePending,
    relaunch_fails: bool,
    exits: AtomicUsize,
}
impl<'a> Seams<'a> {
    fn new(r: &'a Rig) -> Seams<'a> {
        Seams { q: &r.q, log: r.log.clone(), dir: r.dir.clone(), pending: FakePending::default(), relaunch_fails: false, exits: AtomicUsize::new(0) }
    }
    fn index(&self, e: &str) -> usize { self.log.pos(e).unwrap_or_else(|| panic!("no {e:?} in {:?}", self.log.all())) }
}
impl ExitSeams for Seams<'_> {
    fn request_stop(&self) { self.log.push("set_stop"); self.q.request_stop(); }
    fn drain(&self) { self.q.drain(); self.log.push("drained"); }
    fn save(&self) -> Result<(), String> { self.log.push("save"); self.q.save() }
    fn freeze_saves(&self) { self.q.freeze(); self.log.push("freeze"); }
    fn sign_out(&self) -> Result<(), String> {
        let step = match load_state(&self.dir) { Loaded::Ok(s) => s.step as i64, _ => -1 };
        self.log.push(format!("sign_out frozen={} file_step={step}", self.q.is_frozen()));
        on_sign_out_with(&self.dir, &self.pending)
    }
    fn relaunch(&self, args: &[&str]) -> Result<(), String> {
        if self.relaunch_fails { return Err("the spawn failed".into()); }
        self.log.push(format!("relaunch:{}", args.join(" ")));
        Ok(())
    }
    fn relaunch_with(&self) -> Result<(), String> {
        if self.relaunch_fails { return Err("the spawn failed".into()); }
        self.log.push("relaunch_with");
        Ok(())
    }
    fn stamp_quit_at(&self) { self.log.push("quit_at"); }
    fn flush(&self) { self.log.push("flush"); }
    fn exit(&self) { self.log.push("exit"); self.exits.fetch_add(1, Ordering::SeqCst); }
    fn snapshot(&self) -> OnboardingState { self.q.snapshot() }
    fn write_finish(&self, handover: Vec<HandoverItem>) -> Result<(), String> {
        // Through the queue, as the real seam does: its own state is set and saved, so a later save of
        // that state (the next quit's) cannot write an unfinished copy over it.
        self.q.finish(handover)?;
        let _ = std::fs::remove_file(answers_path(&self.dir));
        self.log.push("write_finish");
        Ok(())
    }
}

fn epoch(ts: &str) -> i64 { ts.parse::<jiff::Timestamp>().unwrap().as_second() }
const CONFIRMED: &str = "2026-10-24T12:00:00Z";

/// A state ready to finish: every lane job done, step 10 confirmed at [`CONFIRMED`].
fn finishable() -> OnboardingState {
    let mut s = state_at(11, true);
    for j in LANE_JOBS { s.jobs.insert(j.into(), JobRecord { state: JobState::Done, ..Default::default() }); }
    s.step10_confirmed_at = Some(CONFIRMED.into());
    s
}

/// F11t review I5: the status carries `finish_ready`, the same rule finish refuses on, so the page's
/// Open Knowlu never re-derives the job states.
#[test]
fn test_09_the_status_says_finish_ready_exactly_when_finish_would_not_refuse() {
    let at = epoch(CONFIRMED) + 180;
    assert_eq!(status_inner(&finishable(), at)["finish_ready"], true);
    let mut absent = finishable();
    absent.jobs.remove("registrar.confirm");
    assert_eq!(status_inner(&absent, at)["finish_ready"], false, "an absent lane job is not ready");
    let mut store = finishable();
    store.jobs.insert("school.store".into(), JobRecord { state: JobState::Running, ..Default::default() });
    assert_eq!(status_inner(&store, at)["finish_ready"], false, "a running store is not settled");
    let mut unconfirmed = finishable();
    unconfirmed.step10_confirmed_at = None;
    assert_eq!(status_inner(&unconfirmed, at)["finish_ready"], false, "step 10 is not confirmed");
    let mut coming = finishable();
    coming.jobs.insert("slot.first_full".into(), JobRecord { state: JobState::Pending, ..Default::default() });
    assert_eq!(status_inner(&coming, epoch(CONFIRMED) + 10)["finish_ready"], false, "the gate is still closed before the cap");
    assert_eq!(status_inner(&coming, at)["finish_ready"], true, "and opens at the cap with the slot still coming");
}

// ---- test 9: quitting and finishing ------------------------------------------------------------------------------

/// Finish later during a running serial job and a running lane job, with one more of each queued
/// behind them: the exit stops new starts in both, waits only for the two running jobs, then saves.
fn quit_during_running_jobs(tag: &str, serial_notices_stop: bool) -> (Rig, Value) {
    let r = rig(tag, state_at(8, true), true);
    let discover = r.fake.block("coursework.discover", serial_notices_stop);
    let apply = r.fake.block("school.apply", false);
    r.q.enqueue("coursework.discover");
    r.q.enqueue("school.apply");
    r.wait_started("coursework.discover");
    r.wait_started("school.apply");
    r.q.enqueue("mail.setup");
    r.q.enqueue("school.courses");
    let seams = Seams::new(&r);
    let guard = ExitGuard::default();
    let out = std::thread::scope(|sc| {
        let exit = sc.spawn(|| onboarding_quit_inner(&seams, &guard));
        r.wait_for("the stop", |r| r.q.stop_requested());
        discover.release();
        apply.release();
        exit.join().unwrap()
    });
    (r, out)
}

#[test]
fn test_09_finish_later_waits_only_for_the_running_jobs_and_the_next_load_shows_them_done() {
    let (r, out) = quit_during_running_jobs("t09-quit-done", false);
    assert_eq!(out["ok"], true, "{out}");
    assert!(!r.log.ran("mail.setup"), "the queue starts nothing new after the stop");
    assert!(!r.log.ran("school.courses"), "nor does the local lane");
    let Loaded::Ok(saved) = load_state(&r.dir) else { panic!("the exit saves") };
    assert_eq!(saved.jobs["coursework.discover"].state, JobState::Done);
    assert_eq!(saved.jobs["school.apply"].state, JobState::Done);
    assert!(matches!(saved.jobs.get("mail.setup").map(|j| j.state), None | Some(JobState::Pending)));
    r.log.assert_clean();
}

#[test]
fn test_09_a_job_that_noticed_the_stop_is_pending_on_the_next_load() {
    let (r, out) = quit_during_running_jobs("t09-quit-stopped", true);
    assert_eq!(out["ok"], true, "{out}");
    assert!(!r.log.ran("mail.setup") && !r.log.ran("school.courses"));
    let Loaded::Ok(saved) = load_state(&r.dir) else { panic!("the exit saves") };
    assert_eq!(saved.jobs["coursework.discover"].state, JobState::Pending, "stopped, so it runs again next time");
    assert_eq!(saved.jobs["school.apply"].state, JobState::Done, "a lane job's safe point is its end");
}

#[test]
fn test_09_finish_at_the_cap_waits_for_the_running_ingest_starts_neither_judge_nor_rank_and_hands_the_slot_over() {
    let s = finishable();
    let r = rig("t09-finish-slot", s, true);
    let ingest = r.fake.block_step("slot.first_full", "ingest");
    r.q.enqueue("slot.first_full");
    r.wait_log("step:slot.first_full:ingest");
    let seams = Seams::new(&r);
    let guard = ExitGuard::default();
    let out = std::thread::scope(|sc| {
        let finish = sc.spawn(|| onboarding_finish_inner(&seams, &guard, epoch(CONFIRMED) + 180));
        r.wait_for("the stop", |r| r.q.stop_requested());
        assert!(r.log.pos("drained").is_none(), "finish waits for ingest to end");
        ingest.release();
        finish.join().unwrap()
    });
    assert_eq!(out["ok"], true, "{out}");
    let all = r.log.all();
    assert!(!all.iter().any(|e| e == "step:slot.first_full:judge" || e == "step:slot.first_full:rank"), "{all:?}");
    assert!(r.log.pos("stopped_before:slot.first_full:judge").is_some());
    let Loaded::Ok(saved) = load_state(&r.dir) else { panic!("saved") };
    assert!(saved.finished);
    assert_eq!(saved.jobs["slot.first_full"].state, JobState::Pending, "the slot is pending");
    assert!(saved.handover.iter().any(|h| h.item == "slot.first_full" && h.signal == "slot_end"), "handover names it: {:?}", saved.handover);
    assert!(!answers_path(&r.dir).exists(), "the answers file is deleted on finish");
    assert!(seams.index("write_finish") < seams.index("relaunch_with"));
}

/// §4.7's checks, all proven but the ones nothing here needs: only `resume` reading a saved file.
struct Proven;
impl ResumeChecks for Proven {
    fn session_present(&self) -> bool { true }
    fn restored_course_count(&self) -> usize { 3 }
    fn entitled(&self) -> bool { true }
    fn school_mailbox_active(&self) -> bool { false }
    fn google_granted(&self) -> bool { false }
    fn homework_login_stored(&self) -> bool { false }
    fn class_commitments_cover_courses(&self) -> bool { false }
    fn account_school(&self) -> Option<School> { None }
    fn answers_ok(&self) -> bool { true }
}

/// `Proven` after a sign-out: the session credential is gone, every other check unchanged.
struct SignedOut;
impl ResumeChecks for SignedOut {
    fn session_present(&self) -> bool { false }
    fn restored_course_count(&self) -> usize { Proven.restored_course_count() }
    fn entitled(&self) -> bool { Proven.entitled() }
    fn school_mailbox_active(&self) -> bool { Proven.school_mailbox_active() }
    fn google_granted(&self) -> bool { Proven.google_granted() }
    fn homework_login_stored(&self) -> bool { Proven.homework_login_stored() }
    fn class_commitments_cover_courses(&self) -> bool { Proven.class_commitments_cover_courses() }
    fn account_school(&self) -> Option<School> { None }
    fn answers_ok(&self) -> bool { Proven.answers_ok() }
}

/// A canary that never notices the stop (its gate stays closed): the exit must not wait for it. The
/// canary's own thread is released at the end so no fake waits out its 30 s guard.
#[test]
fn test_09_a_canary_that_never_notices_the_stop_is_abandoned_at_once_by_finish_and_by_sign_out() {
    for arm in ["finish", "sign_out"] {
        let r = rig(&format!("t09-canary-{arm}"), finishable(), true);
        let gate = r.fake.block("mail.canary", false);
        r.q.enqueue("mail.canary");
        r.wait_started("mail.canary");
        let seams = Seams::new(&r);
        let began = Instant::now();
        let out = match arm {
            "finish" => onboarding_finish_inner(&seams, &ExitGuard::default(), epoch(CONFIRMED) + 180),
            _ => onboarding_sign_out_inner(&seams, &ExitGuard::default()),
        };
        let took = began.elapsed();
        assert!(!*gate.open.lock().unwrap(), "{arm}: the gate was still closed when the exit returned");
        assert!(took < Duration::from_secs(2), "{arm}: the exit waited {took:?} for a canary that never stops");
        assert_eq!(out["ok"], true, "{arm}: {out}");
        match arm {
            "finish" => {
                let Loaded::Ok(saved) = load_state(&r.dir) else { panic!("saved") };
                assert_eq!(saved.jobs["mail.canary"].state, JobState::Pending, "abandoned, so it runs again from the start");
                assert!(saved.handover.iter().any(|h| h.item == "mail.canary" && h.signal == "mail_json"), "{:?}", saved.handover);
            }
            _ => {
                // The rewind resets every job to pending, so the canary's own state says nothing on this arm:
                // the closed gate and the time bound above are its test. Only the rewind is checked here.
                // Sign-out deleted the session, so the checks report none (controller ruling, F2a review r1, I-1).
                let resumed = resume(load_state(&r.dir), &SignedOut);
                assert_eq!((resumed.mode, resumed.step), (LaunchMode::Onboarding, 2));
            }
        }
        gate.release();
    }
}

#[test]
fn test_09_finish_is_refused_while_a_lane_job_is_not_done_or_the_gate_is_closed() {
    let mut s = finishable();
    s.jobs.insert("config.ingest".into(), JobRecord { state: JobState::Failed, reason: Some("could_not_write_config".into()), ..Default::default() });
    let r = rig("t09-refuse-lane", s, false);
    let seams = Seams::new(&r);
    let out = onboarding_finish_inner(&seams, &ExitGuard::default(), epoch(CONFIRMED) + 600);
    assert_eq!(out["ok"], false, "{out}");
    assert!(r.log.all().is_empty(), "a refusal stops nothing, saves nothing, exits nothing: {:?}", r.log.all());
    // A closed gate: the lane is done but step 10 was confirmed ten seconds ago and an item is pending.
    let mut s = finishable();
    s.jobs.insert("slot.first_full".into(), JobRecord { state: JobState::Pending, ..Default::default() });
    let r = rig("t09-refuse-gate", s, false);
    let seams = Seams::new(&r);
    let out = onboarding_finish_inner(&seams, &ExitGuard::default(), epoch(CONFIRMED) + 10);
    assert_eq!(out["ok"], false, "{out}");
    assert!(r.log.all().is_empty(), "{:?}", r.log.all());
}

#[test]
fn test_09_the_handover_lists_every_required_item_not_done_with_its_clearing_signal() {
    let mut s = state_at(11, true);
    let job = |st| JobRecord { state: st, ..Default::default() };
    for (k, st) in [
        ("first.sync", JobState::Done), ("first.grades", JobState::Pending), ("first.ingest", JobState::Failed),
        ("slot.first_full", JobState::Pending), ("google.gmail_first_read", JobState::Pending),
        ("mail.canary", JobState::Pending), ("mail.first_pass", JobState::Done), ("coursework.discover", JobState::Pending),
    ] {
        s.jobs.insert(k.into(), job(st));
    }
    let want: Vec<(&str, &str)> = vec![
        ("first.grades", "slot_end"), ("first.ingest", "slot_end"), ("google.gmail_first_read", "slot_end"),
        ("mail.canary", "mail_json"), ("slot.first_full", "slot_end"),
    ];
    let got: Vec<(String, String)> = handover_for(&s).into_iter().map(|h| (h.item, h.signal)).collect();
    assert_eq!(got, want.iter().map(|(a, b)| (a.to_string(), b.to_string())).collect::<Vec<_>>(), "sorted by item; done items and non-required jobs are absent");
}

#[test]
fn test_09_a_slot_end_clears_the_slot_items_and_a_mail_item_only_its_own_signal_or_a_disconnect() {
    let mk = || {
        let mut s = state_at(11, true);
        s.finished = true;
        s.handover = vec![
            HandoverItem { item: "slot.first_full".into(), signal: "slot_end".into() },
            HandoverItem { item: "google.gmail_first_read".into(), signal: "slot_end".into() },
            HandoverItem { item: "mail.canary".into(), signal: "mail_json".into() },
            HandoverItem { item: "mail.first_pass".into(), signal: "mail_json".into() },
        ];
        s
    };
    fn names(s: &OnboardingState) -> Vec<&str> { s.handover.iter().map(|h| h.item.as_str()).collect() }
    let mut s = mk();
    s.slot_ended();
    assert_eq!(names(&s), ["mail.canary", "mail.first_pass"], "a slot's end does not clear a mail item");
    s.mail_signal(MailSignal::CanaryVerified);
    assert_eq!(names(&s), ["mail.first_pass"]);
    s.mail_signal(MailSignal::FirstPassDelivered);
    assert!(s.handover.is_empty());
    let mut s = mk();
    s.mail_signal(MailSignal::Disconnected);
    assert_eq!(names(&s), ["slot.first_full", "google.gmail_first_read"], "a disconnect clears both mail items and no slot item");
}

// ---- test 17: sign out mid-onboarding ------------------------------------------------------------------------------

#[test]
fn test_17_sign_out_stops_abandons_saves_freezes_signs_out_relaunches_with_pick_and_exits_in_that_order() {
    let mut s = state_at(8, true);
    s.school = Some(School { unitid: Some("900001".into()), label: "Example Test College".into(), path: SchoolPath::Shared, confirmed: true });
    s.jobs.insert("school.capture".into(), JobRecord { state: JobState::Done, ..Default::default() });
    let r = rig("t17-order", s, true);
    save_answers(&r.dir, &Answers { feeds: BTreeMap::from([("lms_ics".to_string(), FeedState::Validated)]), ..Default::default() }).unwrap();
    let canary = r.fake.block("mail.canary", false);
    r.q.enqueue("mail.canary");
    r.wait_started("mail.canary");
    let seams = Seams::new(&r);
    seams.pending.put(PENDING_LMS, "https://lms.example.invalid/feed/tok-t17.ics").unwrap();
    let out = onboarding_sign_out_inner(&seams, &ExitGuard::default());
    assert_eq!(out["ok"], true, "{out}");
    let order = ["set_stop", "save", "freeze", "sign_out frozen=true file_step=8", "relaunch:--pick", "exit"];
    let at: Vec<usize> = order.iter().map(|e| seams.index(e)).collect();
    assert!(at.windows(2).all(|w| w[0] < w[1]), "stop, last save, freeze, sign-out, relaunch --pick, exit, in that order: {:?}", r.log.all());
    assert!(r.log.pos("flush").is_none(), "no push: the session is about to go");
    // on_sign_out ran inside the fake sign-out: answers and pending link gone, file rewound.
    assert!(!answers_path(&r.dir).exists());
    assert!(seams.pending.get(PENDING_LMS).is_none());
    let Loaded::Ok(rewound) = load_state(&r.dir) else { panic!("rewound, not removed") };
    assert_eq!(rewound.step, 2);
    assert!(rewound.school.is_none() && !rewound.account_settled);
    assert!(rewound.jobs.values().all(|j| j.state == JobState::Pending), "{:?}", rewound.jobs);
    canary.release(); // the abandoned canary's fake thread ends; it never noticed the stop
}

#[test]
fn test_17_no_save_lands_after_the_rewind() {
    let r = rig("t17-frozen", state_at(8, true), true);
    let seams = Seams::new(&r);
    let out = onboarding_sign_out_inner(&seams, &ExitGuard::default());
    assert_eq!(out["ok"], true, "{out}");
    let rewound = std::fs::read(state_path(&r.dir)).expect("the rewound file");
    assert!(r.q.is_frozen());
    assert!(r.q.save().is_err(), "a save the queue attempts after the freeze is refused");
    r.q.enqueue("school.capture");
    r.idle();
    assert_eq!(std::fs::read(state_path(&r.dir)).unwrap(), rewound, "the file read back is the rewound one");
}

// ---- test 19: a failed relaunch -----------------------------------------------------------------------------------------

#[test]
fn test_19_a_failed_relaunch_keeps_what_was_written_starts_no_job_and_the_next_quit_exits() {
    // Finish: the state it wrote stands.
    let r = rig("t19-finish", finishable(), true);
    let mut seams = Seams::new(&r);
    seams.relaunch_fails = true;
    let guard = ExitGuard::default();
    let out = onboarding_finish_inner(&seams, &guard, epoch(CONFIRMED) + 180);
    assert_eq!((out["ok"].clone(), out["restart"].clone()), (false.into(), true.into()), "{out}");
    let Loaded::Ok(saved) = load_state(&r.dir) else { panic!("saved") };
    assert!(saved.finished, "finished: true is kept");
    assert_eq!(seams.exits.load(Ordering::SeqCst), 0);
    let before = r.log.started().len();
    r.q.enqueue("school.capture");
    std::thread::sleep(Duration::from_millis(100));
    assert_eq!(r.log.started().len(), before, "the stop holds: no job starts");
    let out = onboarding_quit_inner(&seams, &guard);
    assert_ne!(out["exiting"], true, "the failed relaunch released the exit guard: {out}");
    assert_eq!(seams.exits.load(Ordering::SeqCst), 1, "the following onboarding_quit calls the fake exit");

    // Sign out: the rewind it made stands.
    let r = rig("t19-signout", state_at(8, true), true);
    let mut seams = Seams::new(&r);
    seams.relaunch_fails = true;
    let guard = ExitGuard::default();
    let out = onboarding_sign_out_inner(&seams, &guard);
    assert_eq!((out["ok"].clone(), out["restart"].clone()), (false.into(), true.into()), "{out}");
    let Loaded::Ok(rewound) = load_state(&r.dir) else { panic!("rewound") };
    assert_eq!(rewound.step, 2);
    let _ = onboarding_quit_inner(&seams, &guard);
    assert_eq!(seams.exits.load(Ordering::SeqCst), 1);
}

// ---- test 24: the window's close ---------------------------------------------------------------------------------------------

struct Plain {
    gate: Arc<Gate>,
    drains: AtomicUsize,
    exits: AtomicUsize,
    log: Log,
}
impl ExitSeams for Plain {
    fn request_stop(&self) { self.log.push("set_stop"); }
    fn drain(&self) { self.drains.fetch_add(1, Ordering::SeqCst); self.gate.wait(&AtomicBool::new(false), false); }
    fn save(&self) -> Result<(), String> { Ok(()) }
    fn freeze_saves(&self) {}
    fn sign_out(&self) -> Result<(), String> { Ok(()) }
    fn relaunch(&self, _args: &[&str]) -> Result<(), String> { Ok(()) }
    fn relaunch_with(&self) -> Result<(), String> { Ok(()) }
    fn stamp_quit_at(&self) {}
    fn flush(&self) {}
    fn exit(&self) { self.exits.fetch_add(1, Ordering::SeqCst); }
    fn snapshot(&self) -> OnboardingState { state_at(8, true) }
    fn write_finish(&self, _handover: Vec<HandoverItem>) -> Result<(), String> { Ok(()) }
}

#[test]
fn test_24_the_close_returns_at_once_a_second_close_starts_no_second_drain_and_the_exit_is_called_once() {
    let gate = Gate::new();
    let plain = Arc::new(Plain { gate: gate.clone(), drains: AtomicUsize::new(0), exits: AtomicUsize::new(0), log: Log::default() });
    let guard = Arc::new(ExitGuard::default());
    let t0 = Instant::now();
    close_requested(plain.clone(), guard.clone());
    assert!(t0.elapsed() < Duration::from_secs(1), "the close handler's own call does not block on the drain");
    let deadline = Instant::now() + Duration::from_secs(10);
    while plain.drains.load(Ordering::SeqCst) == 0 {
        assert!(Instant::now() < deadline, "the spawned thread never started draining");
        std::thread::sleep(Duration::from_millis(5));
    }
    assert_eq!(plain.exits.load(Ordering::SeqCst), 0, "still draining");
    let second = onboarding_quit_inner(&*plain, &guard);
    assert_eq!((second["ok"].clone(), second["exiting"].clone()), (true.into(), true.into()), "{second}");
    assert_eq!(plain.drains.load(Ordering::SeqCst), 1, "no second drain");
    gate.release();
    let deadline = Instant::now() + Duration::from_secs(10);
    while plain.exits.load(Ordering::SeqCst) == 0 {
        assert!(Instant::now() < deadline, "the exit was never called");
        std::thread::sleep(Duration::from_millis(5));
    }
    std::thread::sleep(Duration::from_millis(50));
    assert_eq!(plain.exits.load(Ordering::SeqCst), 1, "the fake exit is called once");
}

// ---- test 25: today's quit path -----------------------------------------------------------------------------------------------

#[test]
fn test_25_quit_and_finish_flush_after_the_drain_and_before_the_exit_with_quit_at_stamped_and_sign_out_flushes_nothing() {
    let r = rig("t25-quit", state_at(8, true), true);
    let seams = Seams::new(&r);
    onboarding_quit_inner(&seams, &ExitGuard::default());
    let (drained, stamped, flushed, exited) = (seams.index("drained"), seams.index("quit_at"), seams.index("flush"), seams.index("exit"));
    assert!(drained < stamped && stamped < flushed && flushed < exited, "{:?}", r.log.all());

    let r = rig("t25-finish", finishable(), true);
    let seams = Seams::new(&r);
    let out = onboarding_finish_inner(&seams, &ExitGuard::default(), epoch(CONFIRMED) + 180);
    assert_eq!(out["ok"], true, "{out}");
    let (drained, spawned, stamped, flushed, exited) =
        (seams.index("drained"), seams.index("relaunch_with"), seams.index("quit_at"), seams.index("flush"), seams.index("exit"));
    assert!(drained < spawned, "the spawn comes after the drain");
    assert!(spawned < flushed, "the spawn comes before the flush, so the child waits on our pid while it runs");
    assert!(stamped < flushed && flushed < exited, "{:?}", r.log.all());

    let r = rig("t25-signout", state_at(8, true), true);
    let seams = Seams::new(&r);
    onboarding_sign_out_inner(&seams, &ExitGuard::default());
    assert!(r.log.pos("flush").is_none() && r.log.pos("quit_at").is_none(), "sign out pushes nothing: {:?}", r.log.all());
}

// ---- shared answers for tests 12-14 ------------------------------------------------------------------------------------

const LMS_LINK: &str = "https://lms.example.invalid/feed/tok-5c19e0.ics";

fn course(code: &str, name: &str, key: &str) -> CourseAnswer {
    let label: String = name.split(' ').next().unwrap().into();
    CourseAnswer { code: code.into(), name: name.into(), captured_label: label.clone(), label, lms_key: key.into(), removed: false, slug: String::new(), fragments: Vec::new() }
}
fn three_courses() -> Vec<CourseAnswer> {
    vec![course("QLT 101", "Quilting Basics", "_1001_1"), course("KNT 220", "Knotwork Theory", "_1002_1"), course("WVG 105", "Weaving Lab", "_1003_1")]
}

struct Acct {
    fail_first: usize,
    calls: AtomicUsize,
    sent: Mutex<Vec<(String, String)>>,
}
impl Acct {
    fn ok() -> Acct { Acct { fail_first: 0, calls: AtomicUsize::new(0), sent: Mutex::new(Vec::new()) } }
    fn failing(n: usize) -> Acct { Acct { fail_first: n, ..Acct::ok() } }
    fn calls(&self) -> usize { self.calls.load(Ordering::SeqCst) }
}
impl FeedAccount for Acct {
    fn put_source(&self, kind: &str, link: &str) -> Result<(), String> {
        let n = self.calls.fetch_add(1, Ordering::SeqCst);
        if n < self.fail_first { return Err("the account service could not be reached".into()); }
        self.sent.lock().unwrap().push((kind.into(), link.into()));
        Ok(())
    }
}

/// A fake that fails if a school window is opened: a resumed store needs none.
struct NoWindow;
impl SchoolWindows for NoWindow {
    fn open_school_window(&self) { panic!("a school window was opened"); }
}

fn feed_rig(tag: &str, state: FeedState, link: &str) -> (PathBuf, FakePending) {
    let dir = scratch(tag);
    let mut a = Answers { courses: three_courses(), ..Default::default() };
    a.feeds.insert("lms_ics".into(), state);
    save_answers(&dir, &a).unwrap();
    let pending = FakePending::default();
    pending.put(PENDING_LMS, link).unwrap();
    (dir, pending)
}

fn feed_state(dir: &Path) -> Option<FeedState> { load_answers(dir).unwrap().unwrap().feeds.get("lms_ics").copied() }
fn handoff(dir: &Path, pending: &FakePending) -> IngestHandoff { ingest_handoff(&load_answers(dir).unwrap().unwrap(), pending) }

// ---- test 12: the answers survive a quit ---------------------------------------------------------------------------------

/// Steps 3-8 through the fakes, every answer given through `onboarding_answer_inner` (the inner of
/// the `onboarding_answer` command), which saves it before it enqueues. `quits` names the steps after
/// which the process "quits": the only thing kept across a quit is the folder, and the next step reads
/// the file afresh and writes it back, so a field the file format drops changes the hand-over.
fn steps_3_to_8(tag: &str, quits: &[u8]) -> String {
    let dir = scratch(tag);
    let pending = FakePending::default();
    let answer = |step: u8, a: Value| {
        let out = onboarding_answer_inner(&dir, step, &a, &mut |_| {});
        assert_eq!(out["ok"], true, "step {step}: {out}");
    };
    let quit = |at: u8| {
        if quits.contains(&at) {
            let a = load_answers(&dir).unwrap().unwrap_or_else(|| panic!("a quit at step {at} lost the answers file"));
            save_answers(&dir, &a).unwrap();
        }
    };
    // Step 3: the capture.
    answer(3, serde_json::json!({"capture": {
        "courses": three_courses(),
        "registrar_rows": [RegistrarRow { code: "QLT 101".into(), days: "MWF".into(), start: "09:00".into(), end: "09:50".into() }],
        "feeds": {"lms_ics": "validated"},
    }}));
    pending.put(PENDING_LMS, LMS_LINK).unwrap();
    // Step 4: payment, then the local lane: the feed is stored and the course notes are made.
    run_feed_store(&dir, "lms_ics", &pending, &Acct::ok(), &NoWindow);
    run_school_courses(&dir, &[], &mut |c| Ok(c.name.to_lowercase().replace(' ', "-")));
    quit(4);
    // Step 6: Google Calendar granted.
    answer(6, serde_json::json!({"google_calendar": true}));
    quit(6);
    // Step 8: each confirmed row is saved as it is confirmed.
    answer(8, serde_json::json!({"row": BookRow { source: "zybooks".into(), key: "ZYB-QLT-101".into(), course_slug: "quilting-basics".into() }}));
    answer(8, serde_json::json!({"zybooks_ignore": "ZYB-ONBOARDING"}));
    // After step 8's confirms, before leaving it.
    quit(8);
    let h = handoff(&dir, &pending);
    assert_eq!(h.course_map.len(), 3, "course_map is never empty when step 3 found courses: {h:?}");
    assert!(h.course_exclude.is_empty(), "nothing was removed, so nothing is excluded: {h:?}");
    serde_json::to_string(&h).unwrap()
}

#[test]
fn test_12_the_text_config_ingest_hands_the_builder_is_byte_equal_with_or_without_quits() {
    let straight = steps_3_to_8("t12-straight", &[]);
    for (i, quits) in [vec![4u8], vec![6], vec![8], vec![4, 6, 8]].into_iter().enumerate() {
        assert_eq!(steps_3_to_8(&format!("t12-quit-{i}"), &quits), straight, "quits at {quits:?} changed the hand-over");
    }
    assert!(straight.contains("quilting-basics") && straight.contains("zybooks_ignore"), "{straight}");
}

// ---- test 13: the feed is never silently lost -----------------------------------------------------------------------

#[test]
fn test_13a_a_store_that_fails_twice_leaves_the_feed_failed_and_the_link_in_the_text() {
    let (dir, pending) = feed_rig("t13a", FeedState::Validated, LMS_LINK);
    let acct = Acct::failing(2);
    let out = run_feed_store(&dir, "lms_ics", &pending, &acct, &NoWindow);
    assert!(matches!(out, Outcome::Failed(_)), "kept on this computer: {out:?}");
    assert_eq!(acct.calls(), 2, "one retry, as create_vault_in does");
    assert_eq!(feed_state(&dir), Some(FeedState::Failed));
    assert_eq!(pending.get(PENDING_LMS).as_deref(), Some(LMS_LINK), "the link stays where it waits");
    let h = handoff(&dir, &pending);
    assert_eq!(h.ics_url, LMS_LINK, "config.ingest's text carries the link");
    assert!(h.feed_kept_on_device);
}

#[test]
fn test_13b_a_quit_after_payment_and_before_school_store_stores_the_link_from_the_pending_credential() {
    // Reload: only the files and the pending store exist. No school window is opened (NoWindow panics).
    let (dir, pending) = feed_rig("t13b", FeedState::Validated, LMS_LINK);
    let acct = Acct::ok();
    let out = run_feed_store(&dir, "lms_ics", &pending, &acct, &NoWindow);
    assert!(matches!(out, Outcome::Done(_)), "{out:?}");
    assert_eq!(*acct.sent.lock().unwrap(), [("lms_ics".to_string(), LMS_LINK.to_string())]);
    assert_eq!(feed_state(&dir), Some(FeedState::Stored));
    assert!(pending.get(PENDING_LMS).is_none(), "stored, so the pending credential is deleted");
}

#[test]
fn test_13c_a_rejected_paste_is_never_sent_to_the_store_and_its_link_is_in_the_text() {
    let (dir, pending) = feed_rig("t13c", FeedState::Rejected, LMS_LINK);
    let acct = Acct::ok();
    run_feed_store(&dir, "lms_ics", &pending, &acct, &NoWindow);
    assert_eq!(acct.calls(), 0, "a rejected link is never sent");
    assert_eq!(feed_state(&dir), Some(FeedState::Rejected));
    assert_eq!(handoff(&dir, &pending).ics_url, LMS_LINK);
}

#[test]
fn test_13d_a_stored_feed_gives_an_empty_ics_url_and_the_pending_credential_is_deleted() {
    let (dir, pending) = feed_rig("t13d", FeedState::Stored, LMS_LINK);
    let h = handoff(&dir, &pending);
    assert_eq!(h.ics_url, "");
    assert!(!h.feed_kept_on_device);
    settle_pending_after_ingest(&pending);
    assert!(pending.get(PENDING_LMS).is_none() && pending.get(PENDING_CALENDAR).is_none(), "deleted after config.ingest");
}

#[test]
fn test_13e_back_to_another_school_before_payment_deletes_the_waiting_link() {
    let (dir, pending) = feed_rig("t13e", FeedState::Validated, LMS_LINK);
    pending.put(PENDING_CALENDAR, "https://cal.example.invalid/private/tok-77.ics").unwrap();
    let mut state = state_at(3, false);
    state.school = Some(School { unitid: Some("900001".into()), label: "Example Test College".into(), path: SchoolPath::Shared, confirmed: true });
    state.jobs.insert("school.capture".into(), JobRecord { state: JobState::Done, ..Default::default() });
    change_school(&dir, &pending, &mut state).expect("back to the picker");
    assert!(pending.get(PENDING_LMS).is_none() && pending.get(PENDING_CALENDAR).is_none());
    assert!(state.school.is_none());
    if let Some(a) = load_answers(&dir).unwrap() { assert!(a.feeds.is_empty(), "the old school's feed states are gone: {a:?}"); }
}

// ---- test 14: no duplicate course note --------------------------------------------------------------------------------

fn restored_notes() -> Vec<ExistingNote> {
    vec![
        ExistingNote { slug: "quilting-basics-restored".into(), source_uid: "_1001_1".into(), code: "QLT 101".into() },
        ExistingNote { slug: "knotwork-restored".into(), source_uid: "_1002_1".into(), code: "KNT 220".into() },
    ]
}

#[test]
fn test_14_school_courses_waits_for_the_account_then_creates_notes_only_for_courses_with_none() {
    // Step 2's restore answered ok: false, so account_settled is false through payment.
    let r = rig_with("t14", state_at(4, false), true, vec![ok_reading(true, 2, 2)], |_| {});
    save_answers(&r.dir, &Answers { courses: three_courses(), ..Default::default() }).unwrap();
    let created = Arc::new(Mutex::new(Vec::<String>::new()));
    {
        let (dir, created) = (r.dir.clone(), created.clone());
        r.fake.behave("school.courses", move || {
            run_school_courses(&dir, &restored_notes(), &mut |c| {
                created.lock().unwrap().push(c.code.clone());
                Ok("weaving-lab".to_string())
            })
        });
    }
    r.q.enqueue("school.courses");
    r.idle();
    let waiting = r.job("school.courses");
    assert_eq!(waiting.state, JobState::Pending);
    assert_eq!(waiting.reason.as_deref(), Some("waiting to reach your account"));
    assert!(!r.log.ran("school.courses") && created.lock().unwrap().is_empty());
    // A sync status reporting the account reached, carrying two course notes, settles it.
    r.q.enqueue("slot.partial");
    r.idle();
    assert!(r.q.snapshot().account_settled);
    assert_eq!(r.state_of("school.courses"), JobState::Done);
    assert_eq!(*created.lock().unwrap(), ["WVG 105"], "a note only for the course with none");
    let slugs = load_answers(&r.dir).unwrap().unwrap().slugs;
    assert_eq!(slugs.get("_1001_1").map(String::as_str), Some("quilting-basics-restored"), "the restored note's own slug");
    assert_eq!(slugs.get("_1002_1").map(String::as_str), Some("knotwork-restored"));
    assert_eq!(slugs.get("_1003_1").map(String::as_str), Some("weaving-lab"));
}

#[test]
fn test_14_a_restored_note_is_matched_by_its_source_uid_as_well_as_its_code() {
    let dir = scratch("t14-uid");
    save_answers(&dir, &Answers { courses: three_courses(), ..Default::default() }).unwrap();
    let mut notes = restored_notes();
    notes.push(ExistingNote { slug: "weaving-old".into(), source_uid: "_1003_1".into(), code: String::new() });
    let out = run_school_courses(&dir, &notes, &mut |c| panic!("a note was made for {}", c.code));
    assert!(matches!(out, Outcome::Done(_)), "{out:?}");
    assert_eq!(load_answers(&dir).unwrap().unwrap().slugs.get("_1003_1").map(String::as_str), Some("weaving-old"));
}

// ---- test 16: the notes gate and sync ----------------------------------------------------------------------------------------

fn slow_notes(r: &Rig) {
    for j in ["school.courses", "registrar.confirm"] {
        r.fake.behave(j, || { std::thread::sleep(Duration::from_millis(40)); Outcome::Done(BTreeMap::new()) });
    }
}

/// The slot's `sync` step is held open for 50 ms while the two note jobs are asked for, so a missing
/// notes gate is an overlap every time, not only on a lucky schedule. Returns when the queue is idle.
fn slot_holds_sync_while_notes_are_asked_for(r: &Rig) {
    slow_notes(r);
    let sync = r.fake.block_step("slot.partial", "sync");
    r.q.enqueue("slot.partial");
    r.wait_log("step:slot.partial:sync");
    for j in ["school.courses", "registrar.confirm"] { r.q.enqueue(j); }
    std::thread::sleep(Duration::from_millis(50));
    assert!(!r.log.ran("school.courses") && !r.log.ran("registrar.confirm"), "a note job started while sync was running: {:?}", r.log.all());
    sync.release();
    r.idle();
}

#[test]
fn test_16_at_payment_with_the_account_settled_the_note_jobs_and_sync_never_overlap_and_none_fails() {
    let r = rig("t16-settled", state_applied(4, true), true);
    slot_holds_sync_while_notes_are_asked_for(&r);
    r.log.assert_clean();
    for j in ["school.courses", "registrar.confirm", "slot.partial"] {
        assert_eq!(r.state_of(j), JobState::Done, "{j} neither failed nor was skipped: {:?}", r.log.all());
    }
    assert!(r.log.steps().contains(&("slot.partial".to_string(), "sync".to_string())), "sync is not skipped");
    assert!(r.log.pos("end:slot.partial").unwrap() < r.log.pos("start:school.courses").unwrap(), "the notes waited for the slot's sync to end");
}

#[test]
fn test_16_a_note_job_mid_write_holds_the_gate_and_a_slot_asked_for_meanwhile_waits_for_it() {
    // The other half of the gate (§14.19 I-B): school.courses is mid-write when slot.partial is asked
    // for. A lane that did not take the gate would let the slot's sync collide with the write.
    let r = rig("t16-notes-first", state_at(4, true), true);
    let write = r.fake.block("school.courses", false);
    r.q.enqueue("school.courses");
    r.wait_started("school.courses");
    r.q.enqueue("slot.partial");
    std::thread::sleep(Duration::from_millis(50));
    assert!(!r.log.ran("slot.partial"), "the slot started while a note job held the gate: {:?}", r.log.all());
    write.release();
    r.idle();
    r.log.assert_clean();
    assert!(r.log.pos("end:school.courses").unwrap() < r.log.pos("start:slot.partial").unwrap(), "{:?}", r.log.all());
    assert_eq!(r.state_of("slot.partial"), JobState::Done);
}

#[test]
fn test_16_with_the_account_unsettled_sync_runs_first_and_the_note_jobs_after_it() {
    let r = rig("t16-unsettled", state_applied(4, false), true);
    // Asked for in the other order: the notes first. Sync still runs first, because the account is not reached.
    slow_notes(&r);
    for j in ["school.courses", "registrar.confirm", "slot.partial"] { r.q.enqueue(j); }
    r.idle();
    r.log.assert_clean();
    let sync = r.log.pos("step:slot.partial:sync").expect("sync ran");
    assert!(sync < r.log.pos("start:school.courses").unwrap() && sync < r.log.pos("start:registrar.confirm").unwrap(), "{:?}", r.log.all());
}

fn first_sync(r: &Rig) -> JobRecord { r.q.snapshot().jobs.get("first.sync").cloned().unwrap_or_default() }
fn courses_failed_with(r: &Rig, words: &str) {
    let c = r.job("school.courses");
    assert_eq!(c.state, JobState::Failed, "{:?}", r.log.all());
    assert!(c.reason.as_deref().unwrap_or("").contains(words), "{:?}", c.reason);
}

/// An unsettled account, `school.courses` and one `slot.partial` asked for, then the readings play out.
fn chain(tag: &str, readings: Vec<Reading>) -> Rig {
    let r = rig_with(tag, state_at(4, false), true, readings, |_| {});
    r.q.enqueue("school.courses");
    r.q.enqueue("slot.partial");
    r.idle();
    r
}
fn skipped(why: &str) -> Reading { (Some(status(FRESH, true, Some(why))), Some(cursor(false, 0, 0))) }
fn runs(r: &Rig) -> usize { r.log.scopes("slot.partial").len() }

#[test]
fn test_16_a_skipped_or_stale_reading_is_not_a_sync_and_two_in_a_row_stop_the_chain() {
    let r = chain("t16-skip", vec![skipped("another sync is running"), skipped("no session")]);
    assert_eq!(first_sync(&r).state, JobState::Pending, "first sync is not done");
    assert!(!r.q.snapshot().account_settled);
    assert_eq!(runs(&r), 2);
    courses_failed_with(&r, "couldn't finish reading your account");
    // A stale stamp (before the slot's own start; also what a lock-held sync leaves) does the same.
    let stale: Reading = (Some(status(STALE, true, None)), Some(cursor(true, 9, 9)));
    let r = chain("t16-stale", vec![stale.clone(), stale]);
    assert_eq!(first_sync(&r).state, JobState::Pending);
    assert!(!r.q.snapshot().account_settled, "a stale run does not settle the account even with pulled_to_end");
}

#[test]
fn test_16_a_fresh_ok_run_to_the_end_settles_both_and_a_fresh_error_is_failed_with_its_fix() {
    let r = chain("t16-ok", vec![ok_reading(true, 10, 10)]);
    assert_eq!(first_sync(&r).state, JobState::Done);
    assert!(r.q.snapshot().account_settled);
    assert_eq!(r.state_of("school.courses"), JobState::Done);
    let r = chain("t16-fail", vec![(Some(status(FRESH, false, None)), Some(cursor(false, 0, 0)))]);
    let first = first_sync(&r);
    assert_eq!(first.state, JobState::Failed);
    assert!(!first.reason.unwrap_or_default().is_empty(), "failed with its fix");
    assert!(!r.q.snapshot().account_settled);
    assert_eq!(runs(&r), 1, "a failed reading stops the chain at once");
    assert_eq!(r.state_of("school.courses"), JobState::Failed);
}

#[test]
fn test_16_t0_a_no_entitlement_skip_is_neither_done_nor_failed() {
    let gated: Reading = (Some(status(FRESH, false, Some("no entitlement"))), Some(cursor(false, 0, 0)));
    let r = chain("t16-gated", vec![gated.clone(), gated]);
    assert_eq!(first_sync(&r).state, JobState::Pending, "neither done nor failed");
    assert!(!r.q.snapshot().account_settled);
}

#[test]
fn test_16_t0_a_run_that_moved_the_cursor_marks_first_sync_done_and_enqueues_one_more_until_it_ends() {
    let r = chain("t16-moving", vec![ok_reading(false, 5, 5), ok_reading(true, 9, 9)]);
    assert_eq!(r.log.scopes("slot.partial"), ["BeforeConfig", "BeforeConfig"]);
    assert_eq!(first_sync(&r).state, JobState::Done);
    assert!(r.q.snapshot().account_settled);
    assert_eq!(r.state_of("school.courses"), JobState::Done);
}

#[test]
fn test_16_t0_an_unmoved_cursor_stops_the_chain_and_fails_school_courses_with_try_again() {
    let r = chain("t16-unmoved", vec![ok_reading(false, 5, 5), ok_reading(false, 5, 5)]);
    assert_eq!(runs(&r), 2);
    assert_eq!(first_sync(&r).state, JobState::Done, "the run was fresh and ok");
    assert!(!r.q.snapshot().account_settled);
    courses_failed_with(&r, "couldn't finish reading your account");
}

#[test]
fn test_16_t0_one_skipped_reading_between_moving_runs_keeps_the_previous_cursor_as_the_comparison() {
    let r = chain("t16-between", vec![ok_reading(false, 5, 5), skipped("another sync is running"), ok_reading(false, 9, 9), ok_reading(true, 12, 12)]);
    assert_eq!(runs(&r), 4, "{:?}", r.log.all());
    assert!(r.q.snapshot().account_settled);
    assert_eq!(r.state_of("school.courses"), JobState::Done);
    // Two skipped in a row after a moving run stop it.
    let r = chain("t16-between-two", vec![ok_reading(false, 5, 5), skipped("no session"), skipped("no session")]);
    assert_eq!(runs(&r), 3);
    courses_failed_with(&r, "couldn't finish reading your account");
}

#[test]
fn test_16_t0_a_failed_reading_between_moving_runs_stops_the_chain_at_once() {
    let r = chain("t16-between-fail", vec![ok_reading(false, 5, 5), (Some(status(FRESH, false, None)), Some(cursor(false, 5, 5))), ok_reading(true, 9, 9)]);
    assert_eq!(runs(&r), 2);
    assert_eq!(first_sync(&r).state, JobState::Failed);
    assert_eq!(r.state_of("school.courses"), JobState::Failed);
}

#[test]
fn test_16_t0_a_cursor_that_moves_on_every_run_stops_at_exactly_fifty_and_try_again_starts_one_slot() {
    assert_eq!(CHAIN_BACKSTOP, 50);
    let mut readings: Vec<Reading> = (1..=50).map(|i| ok_reading(false, i, i)).collect();
    readings.push(ok_reading(true, 99, 99));
    let r = chain("t16-backstop", readings);
    assert_eq!(runs(&r), 50, "the 51st is not enqueued");
    courses_failed_with(&r, "couldn't finish reading your account");
    // Try again while unsettled resets the count and enqueues exactly one BeforeConfig slot.
    r.q.retry("school.courses");
    r.idle();
    assert_eq!(runs(&r), 51, "exactly one more");
    assert_eq!(r.log.scopes("slot.partial")[50], "BeforeConfig");
    assert!(r.q.snapshot().account_settled);
    assert_eq!(r.state_of("school.courses"), JobState::Done);
}

#[test]
fn test_36_registrar_confirm_waits_for_school_apply_as_well_as_school_courses() {
    // F8h: `school_of` reads the campus.yaml that school.apply creates, so the confirm is not ready
    // while that job is pending, and runs once it is done.
    let mut s = state_at(4, true);
    s.jobs.insert("school.courses".into(), JobRecord { state: JobState::Done, ..Default::default() });
    let r = rig("t36-waits-apply", s, true);
    r.q.enqueue("registrar.confirm");
    r.idle();
    assert!(!r.log.ran("registrar.confirm"), "started before school.apply: {:?}", r.log.all());
    assert_eq!(r.state_of("registrar.confirm"), JobState::Pending);
    r.q.enqueue("school.apply");
    r.idle();
    r.log.assert_clean();
    assert_eq!(r.state_of("registrar.confirm"), JobState::Done, "{:?}", r.log.all());
    assert!(r.log.pos("end:school.apply").unwrap() < r.log.pos("start:registrar.confirm").unwrap(), "{:?}", r.log.all());
}

#[test]
fn test_16_a_lane_job_retrying_on_a_held_sync_lock_holds_neither_vault_io_nor_the_lock_between_tries() {
    let vault_io = Mutex::new(());
    let lock_held = Arc::new(AtomicBool::new(false));
    struct Token(Arc<AtomicBool>);
    impl Drop for Token { fn drop(&mut self) { self.0.store(false, Ordering::SeqCst); } }
    let refusals = Cell::new(3usize);
    let sleeps = RefCell::new(Vec::<Duration>::new());
    let works = Cell::new(0usize);
    let held = lock_held.clone();
    let take = || -> Result<Box<dyn std::any::Any>, String> {
        if refusals.get() > 0 { refusals.set(refusals.get() - 1); return Err(knowlu::week::SYNC_RUNNING.to_string()); }
        held.store(true, Ordering::SeqCst);
        Ok(Box::new(Token(held.clone())))
    };
    let sleep = |d: Duration| {
        assert!(vault_io.try_lock().is_ok(), "a vault_io user gets it between tries");
        assert!(!lock_held.load(Ordering::SeqCst), "the sync lock is not held between tries");
        sleeps.borrow_mut().push(d);
    };
    let out = retry_with_hold(&vault_io, &take, &sleep, &mut || { works.set(works.get() + 1); Ok(7) });
    assert_eq!(out, Ok(7));
    assert_eq!(*sleeps.borrow(), vec![RETRY_INTERVAL; 3], "every 250 ms");
    assert_eq!(RETRY_INTERVAL, Duration::from_millis(250));
    assert_eq!(works.get(), 1);
    // Any other refusal is not "try again".
    let sleeps2 = RefCell::new(0usize);
    let other = retry_with_hold(&vault_io, &|| Err::<Box<dyn std::any::Any>, _>("disk error".to_string()), &|_| *sleeps2.borrow_mut() += 1, &mut || Ok(1));
    assert_eq!(other, Err("disk error".to_string()));
    assert_eq!(*sleeps2.borrow(), 0);
    // Always refused: gives up after thirty seconds of tries, never having run the work.
    let total = RefCell::new(Duration::ZERO);
    let never = retry_with_hold(&vault_io, &|| Err::<Box<dyn std::any::Any>, _>(knowlu::week::SYNC_RUNNING.to_string()), &|d| *total.borrow_mut() += d, &mut || Ok(1));
    assert_eq!(never, Err(knowlu::week::SYNC_RUNNING.to_string()), "past the budget is the busy refusal itself");
    assert_eq!(*total.borrow(), RETRY_BUDGET);
    assert_eq!(RETRY_BUDGET, Duration::from_secs(30));
}

#[test]
fn test_16_registrar_confirm_is_entered_with_vault_io_free_and_retries_only_on_the_sync_running_constant() {
    let vault_io = Mutex::new(());
    let tries = Cell::new(0usize);
    let sleeps = Cell::new(0usize);
    let fake_registrar = || -> Result<i32, String> {
        assert!(vault_io.try_lock().is_ok(), "registrar.confirm's fake is entered with vault_io free");
        tries.set(tries.get() + 1);
        if tries.get() <= 2 { Err(knowlu::week::SYNC_RUNNING.to_string()) } else { Ok(42) }
    };
    let out = retry_on_sync_running(&|_| sleeps.set(sleeps.get() + 1), &mut || fake_registrar());
    assert_eq!(out, Ok(42));
    assert_eq!((tries.get(), sleeps.get()), (3, 2));
    // A typed copy of the sentence is not the constant, and nothing else is "try again".
    let n = Cell::new(0usize);
    let other = retry_on_sync_running::<i32>(&|_| panic!("slept"), &mut || { n.set(n.get() + 1); Err("a sync is running; try again in a moment!".to_string()) });
    assert!(other.is_err());
    assert_eq!(n.get(), 1, "only week::SYNC_RUNNING itself is retried");
    // Always busy: thirty seconds of sleeps, then the busy refusal itself (review: F8h-runner maps it).
    let slept = RefCell::new(Duration::ZERO);
    let busy = retry_on_sync_running::<i32>(&|d| *slept.borrow_mut() += d, &mut || Err(knowlu::week::SYNC_RUNNING.to_string()));
    assert_eq!(busy, Err(knowlu::week::SYNC_RUNNING.to_string()));
    assert_eq!(*slept.borrow(), RETRY_BUDGET);
    // The pin: onboarding_jobs.rs compares the constant and never a copy of its words.
    let src = src_of("onboarding_jobs.rs");
    assert!(src.contains("SYNC_RUNNING"), "the retry compares week::SYNC_RUNNING");
    assert!(!src.contains("a sync is running"), "no copy of the sentence");
}

// ---- test 23: no slot outside the queue ----------------------------------------------------------------------------------

#[test]
fn test_23_a_closed_batch_tells_the_queue_in_onboarding_mode_and_runs_a_slot_in_the_normal_console() {
    use knowlu::mail::live::{on_batch_closed, BatchClosed};
    assert_eq!(on_batch_closed(LaunchMode::Onboarding), BatchClosed::TellQueue);
    assert_eq!(on_batch_closed(LaunchMode::Normal), BatchClosed::RunSlot);
}

#[test]
fn test_23_with_a_queue_a_closed_batch_enqueues_nothing_starts_no_slot_and_leaves_the_next_slots_order() {
    let r = rig("t23", state_at(4, false), true);
    let before = r.q.snapshot().jobs;
    r.q.mail_batch_closed();
    r.idle();
    assert!(r.q.batch_closed_noted(), "the queue records it for the rail's older_mail line");
    assert_eq!(r.q.snapshot().jobs, before, "no job was enqueued");
    assert!(r.log.started().is_empty(), "no slot was started: {:?}", r.log.all());
    // The next slot the queue runs is unchanged: still before-config, in the order it was asked for.
    r.q.enqueue("slot.partial");
    r.idle();
    assert_eq!(r.log.started(), ["slot.partial"]);
    assert_eq!(r.log.scopes("slot.partial"), ["BeforeConfig"]);
}

// ---- test 26: onboarding never stores a link at capture or paste -------------------------------------------------------------

/// Source text with its comment lines removed.
fn src_of(file: &str) -> String {
    let p = Path::new(env!("CARGO_MANIFEST_DIR")).join("src").join(file);
    std::fs::read_to_string(&p).unwrap_or_else(|e| panic!("{}: {e}", p.display()))
        .lines().filter(|l| !l.trim_start().starts_with("//")).collect::<Vec<_>>().join("\n")
}

/// The top-level function that contains byte `at` of `src`: from the nearest line starting `fn `,
/// `pub fn ` or `#[...]`-less signature before it to the next lone `}`.
fn enclosing_fn(src: &str, at: usize) -> &str {
    let start = ["\nfn ", "\npub fn ", "\npub(crate) fn "].iter().filter_map(|p| src[..at].rfind(p)).max().map(|i| i + 1).unwrap_or(0);
    let end = src[at..].find("\n}").map(|i| at + i + 2).unwrap_or(src.len());
    &src[start..end]
}

/// A listener nothing may connect to: counts the connections it sees.
struct Tripwire {
    base: String,
    hits: Arc<AtomicUsize>,
    stop: Arc<AtomicBool>,
    join: Option<std::thread::JoinHandle<()>>,
}
impl Tripwire {
    fn start() -> Tripwire {
        let l = std::net::TcpListener::bind("127.0.0.1:0").expect("bind loopback");
        l.set_nonblocking(true).unwrap();
        let base = format!("http://127.0.0.1:{}/functions/v1", l.local_addr().unwrap().port());
        let (hits, stop) = (Arc::new(AtomicUsize::new(0)), Arc::new(AtomicBool::new(false)));
        let (h, s) = (hits.clone(), stop.clone());
        let join = std::thread::spawn(move || {
            while !s.load(Ordering::SeqCst) {
                match l.accept() {
                    Ok(_) => { h.fetch_add(1, Ordering::SeqCst); }
                    Err(_) => std::thread::sleep(Duration::from_millis(5)),
                }
            }
        });
        Tripwire { base, hits, stop, join: Some(join) }
    }
}
impl Drop for Tripwire {
    fn drop(&mut self) {
        self.stop.store(true, Ordering::SeqCst);
        if let Some(j) = self.join.take() { let _ = j.join(); }
    }
}

/// `KNOWLU_API_BASE` is process-global; this file's only test that sets it takes this lock and
/// restores the variable on any exit path. No Credential Manager is touched: the pending credential
/// is a fake store.
static ENV_LOCK: Mutex<()> = Mutex::new(());
struct ApiBase(Option<std::ffi::OsString>);
impl ApiBase {
    fn set(v: &str) -> ApiBase {
        let prev = std::env::var_os("KNOWLU_API_BASE");
        unsafe { std::env::set_var("KNOWLU_API_BASE", v) };
        ApiBase(prev)
    }
}
impl Drop for ApiBase {
    fn drop(&mut self) {
        match self.0.take() {
            Some(v) => unsafe { std::env::set_var("KNOWLU_API_BASE", v) },
            None => unsafe { std::env::remove_var("KNOWLU_API_BASE") },
        }
    }
}

const GOOD_ICS: &str = "BEGIN:VCALENDAR\r\nBEGIN:VEVENT\r\nSUMMARY:QLT 101 Homework 4\r\nEND:VEVENT\r\nEND:VCALENDAR\r\n";
const EMPTY_ICS: &str = "BEGIN:VCALENDAR\r\nEND:VCALENDAR\r\n";

#[test]
fn test_26_a_pasted_link_is_validated_and_kept_on_this_computer_and_nothing_is_sent_to_any_server() {
    let _lock = ENV_LOCK.lock().unwrap_or_else(|e| e.into_inner());
    let wire = Tripwire::start();
    let _api = ApiBase::set(&wire.base);
    let dir = scratch("t26-paste");
    let pending = FakePending::default();
    let out = paste_link_inner(&dir, &pending, "lms_ics", LMS_LINK, &|_| Ok(GOOD_ICS.to_string()));
    assert_eq!((out["ok"].clone(), out["stored"].clone()), (true.into(), false.into()), "{out}");
    assert!(out["note"].as_str().unwrap_or_default().contains("kept on this computer until payment"), "its note says why: {out}");
    assert_eq!(pending.get(PENDING_LMS).as_deref(), Some(LMS_LINK), "the generated pending credential is written");
    assert_eq!(load_answers(&dir).unwrap().unwrap().feeds.get("lms_ics"), Some(&FeedState::Validated));
    assert_eq!(wire.hits.load(Ordering::SeqCst), 0, "the listener received no request");
}

#[test]
fn test_26_a_link_that_fails_validation_answers_not_ok_and_is_kept_as_rejected() {
    let dir = scratch("t26-rejected");
    let pending = FakePending::default();
    let out = paste_link_inner(&dir, &pending, "lms_ics", LMS_LINK, &|_| Ok(EMPTY_ICS.to_string()));
    assert_eq!(out["ok"], false, "{out}");
    assert_eq!(load_answers(&dir).unwrap().unwrap().feeds.get("lms_ics"), Some(&FeedState::Rejected));
    assert_eq!(pending.get(PENDING_LMS).as_deref(), Some(LMS_LINK), "the rejected link is kept too (§4.11)");
}

#[test]
fn test_26_neither_state_nor_jobs_name_the_storing_capture_or_paste() {
    for file in ["onboarding_jobs.rs", "onboarding_state.rs"] {
        let src = src_of(file);
        for banned in ["lms_link::finish(", "capture_calendar_link(", "paste_calendar_link("] {
            assert!(!src.contains(banned), "{file} names {banned}");
        }
    }
}

#[test]
fn test_26_store_source_appears_only_in_the_store_runner_over_the_vaults_session_never_the_pending_one() {
    let jobs = src_of("onboarding_jobs.rs");
    let state = src_of("onboarding_state.rs");
    assert!(jobs.contains("store_source("), "F8c's store runner calls store_source");
    for (file, src) in [("onboarding_jobs.rs", &jobs), ("onboarding_state.rs", &state)] {
        for (at, _) in src.match_indices("store_source(") {
            let f = enclosing_fn(src, at);
            assert!(f.contains("session_credential_target"), "{file}: store_source( outside a call that names session_credential_target:\n{f}");
            assert!(!f.contains("PENDING_TARGET"), "{file}: store_source( beside PENDING_TARGET:\n{f}");
        }
    }
    assert!(!jobs.contains("PENDING_TARGET"), "PENDING_TARGET appears in onboarding_jobs.rs nowhere");
}

#[test]
fn test_26_capture_link_only_goes_through_validate_only_and_never_hands_finish_over() {
    let src = src_of("lms_link.rs");
    assert!(src.contains("fn capture_calendar_link_with"), "F8la splits the capture body out");
    let at = src.find("pub fn capture_link_only").expect("capture_link_only exists");
    let body = enclosing_fn(&src, at + 7);
    assert!(body.contains("validate_only"), "capture_link_only names validate_only:\n{body}");
    assert!(body.contains("capture_calendar_link_with"), "{body}");
    assert!(!body.contains("finish"), "capture_link_only passes no `finish`:\n{body}");
}

#[test]
fn test_26_the_consoles_handler_list_holds_neither_storing_command() {
    let src = std::fs::read_to_string(Path::new(env!("CARGO_MANIFEST_DIR")).join("src").join("main.rs")).unwrap();
    const OPEN: &str = "generate_handler![";
    let mut rest = src.as_str();
    let mut console: Option<Vec<String>> = None;
    while let Some(at) = rest.find(OPEN) {
        let body = &rest[at + OPEN.len()..];
        let list: Vec<String> = body[..body.find(']').unwrap()].split(',').map(|n| n.trim().to_string()).collect();
        if list.iter().any(|n| n == "commands::state") { console = Some(list); }
        rest = &body[body.find(']').unwrap()..];
    }
    let console = console.expect("the console's list");
    for banned in ["lms_link::capture_calendar_link", "lms_link::paste_calendar_link"] {
        assert!(!console.iter().any(|n| n == banned), "the console's list holds {banned}");
    }
}

// ======================================================================================================
// F1 review r1: removed courses (test 12), the unconfirmed school (test 2b), the answers gate after a
// rewind (test 17), the finish write through the queue (test 19).
// ======================================================================================================

fn capture_answer() -> Value {
    serde_json::json!({"capture": {
        "courses": three_courses(),
        "registrar_rows": [RegistrarRow { code: "QLT 101".into(), days: "MWF".into(), start: "09:00".into(), end: "09:50".into() }],
        "feeds": {},
    }})
}

/// `save_pairs`' argument: each row's pairs by its key (the LMS course key).
fn pairs_of(rows: &[(&str, &[&str])]) -> BTreeMap<String, Vec<String>> {
    rows.iter().map(|(k, f)| (k.to_string(), f.iter().map(|s| s.to_string()).collect())).collect()
}

#[test]
fn test_12_a_course_removed_at_step_3_reaches_no_note_and_no_course_map_entry_and_stays_removed_after_a_quit() {
    let dir = scratch("t12-removed");
    let pending = FakePending::default();
    let give = |step: u8, a: Value| {
        let out = onboarding_answer_inner(&dir, step, &a, &mut |_| {});
        assert_eq!(out["ok"], true, "{out}");
    };
    give(3, capture_answer());
    // F8b's pairs seam (never a page answer): each course's pairs by its key (the human code, then a
    // gradebook column id in the pin form).
    save_pairs(&dir, &pairs_of(&[("_1001_1", &["QLT 101", "_71_1"]), ("_1002_1", &["KNT 220", "_72_1"]), ("_1003_1", &["WVG 105", "_73_1"])])).unwrap();
    // A label edit is display only (§14.46): it changes no pair.
    give(3, serde_json::json!({"label": {"key": "_1002_1", "label": "Knots, edited"}}));
    // `school.preview` through its core, with a fake engine that records the `--courses` it is handed
    // and answers the match-set buckets as the engine writes them.
    let handed = RefCell::new(Vec::<Value>::new());
    let out = run_school_preview(&dir, &mut |courses| {
        handed.borrow_mut().push(courses.clone());
        Ok(serde_json::json!({"by_course": {
            "_1001_1": {"codes": ["_1001_1"], "class_times": 3, "assignments_upcoming": 7, "graded": true, "turned_in": 0},
            "_1002_1": {"codes": ["_1002_1"], "class_times": 2, "assignments_upcoming": 11, "graded": true, "turned_in": 0},
            "_1003_1": {"codes": ["_1003_1"], "class_times": 1, "assignments_upcoming": 4, "graded": false, "turned_in": 0},
            "_1001_1,_1002_1": {"codes": ["_1001_1", "_1002_1"], "class_times": 0, "assignments_upcoming": 100, "turned_in": 0},
            "unmatched": {"codes": [], "class_times": 0, "assignments_upcoming": 1000, "turned_in": 0},
        }, "classes": 3, "errors": []}))
    });
    assert!(matches!(out, Outcome::Done(_)), "{out:?}");
    assert_eq!(handed.borrow().len(), 1, "one --preview call");
    give(3, serde_json::json!({"remove": "_1002_1"}));
    // A quit and a reload: nothing here but the folder.
    let reloaded = load_answers(&dir).unwrap().expect("the answers file survived");
    let totals = step3_totals(&reloaded);
    assert_eq!(totals.get("classes"), Some(&Some(2)), "{totals:?}");
    assert_eq!(totals.get("assignments_upcoming"), Some(&Some(1111)), "7 + 4 + the shared 100 + the unmatched 1000; the removed row's own 11 is out: {totals:?}");
    // §14.47: the removed course was graded, so the grades count falls by one; no class time falls.
    assert_eq!(totals.get("grades_courses"), Some(&Some(1)), "QLT; KNT removed, WVG has no graded column: {totals:?}");
    assert_eq!(totals.get("class_times"), Some(&Some(6)), "3 + 2 + 1: the removed course's class times are still imported: {totals:?}");
    assert_eq!(registrar_confirm_rows(&reloaded), reloaded.registrar_rows, "every registrar row still reaches registrar.confirm");
    let created = RefCell::new(Vec::<String>::new());
    let out = run_school_courses(&dir, &[], &mut |c| { created.borrow_mut().push(c.code.clone()); Ok(c.name.to_lowercase().replace(' ', "-")) });
    assert!(matches!(out, Outcome::Done(_)), "{out:?}");
    assert_eq!(*created.borrow(), ["QLT 101", "WVG 105"], "the removed course gets no note");
    let h = handoff(&dir, &pending);
    let map_fragments: Vec<&str> = h.course_map.iter().map(|(f, _)| f.as_str()).collect();
    assert_eq!(map_fragments, ["QLT 101", "_71_1", "WVG 105", "_73_1"], "course_map is the kept rows' pairs only");
    assert_eq!(h.course_exclude, ["KNT 220", "_72_1"], "the removed course's pairs, built as course_map's are, so ingest leaves out its items");
    // Narrowed (§14.45): the removed course's name or slug is in no course_map entry; its fragments are only under course_exclude.
    let map_text = serde_json::to_string(&h.course_map).unwrap();
    assert!(!map_text.contains("knotwork") && !map_text.contains("KNT 220") && !map_text.contains("_72_1"), "course_map: {map_text}");
    // One source of pairs (§14.46): after the label edit, the pairs in the --courses list the fake
    // --preview call was handed are byte-equal to that course's lines in the text (course_map's, or
    // course_exclude less any fragment a kept course shares; none here). The entry's `code` is the
    // row's key (the LMS course key), which is also its key in `slugs`.
    let now = load_answers(&dir).unwrap().unwrap();
    let listed = handed.borrow()[0].clone();
    for c in listed.as_array().expect("one object per course") {
        let key = c["code"].as_str().unwrap();
        let row = now.courses.iter().find(|r| r.lms_key == key).unwrap_or_else(|| panic!("--courses code {key} is no row's LMS key"));
        let in_text: Vec<&str> = if row.removed {
            h.course_exclude.iter().map(String::as_str).collect()
        } else {
            let slug = &now.slugs[key];
            h.course_map.iter().filter(|(_, s)| s == slug).map(|(f, _)| f.as_str()).collect()
        };
        assert_eq!(serde_json::to_string(&in_text).unwrap(), serde_json::to_string(&c["pairs"]).unwrap(), "{key}: the pairs in --courses and in the text");
    }
    assert!(listed.as_array().unwrap().iter().any(|c| c["code"] == "_1002_1" && c["label"] == "Knots, edited"), "the edit shows as the label only");
    // A re-capture is the student asking again: the mark is cleared, and nothing is excluded.
    give(3, capture_answer());
    assert_eq!(load_answers(&dir).unwrap().unwrap().kept_courses().len(), 3);
    assert!(handoff(&dir, &pending).course_exclude.is_empty(), "with nothing removed the key is absent");
}

#[test]
fn test_02_school_apply_is_enqueued_but_does_not_start_until_the_school_is_confirmed() {
    let mut s = state_at(3, true);
    s.school = Some(School { unitid: Some("900001".into()), label: "Example Test College".into(), path: SchoolPath::Shared, confirmed: false });
    let r = rig("t02-apply", s, true);
    r.q.enqueue("school.apply");
    r.idle();
    assert!(!r.log.ran("school.apply"), "an offered school is assumed nothing about: {:?}", r.log.all());
    assert!(matches!(r.state_of("school.apply"), JobState::Pending | JobState::Held), "{:?}", r.state_of("school.apply"));
    r.q.confirm_school();
    r.idle();
    assert!(r.log.ran("school.apply"), "confirmed, so it runs: {:?}", r.log.all());
    assert_eq!(r.state_of("school.apply"), JobState::Done);
    assert!(r.q.snapshot().school.is_some_and(|s| s.confirmed));
}

#[test]
fn test_17_after_a_rewind_with_no_answers_file_the_config_jobs_stay_pending_and_run_once_the_answers_are_saved() {
    let r = rig_full("t17-noanswers-queue", state_applied(8, true), true, vec![ok_reading(true, 10, 10)], false, |_| {});
    assert!(!answers_path(&r.dir).exists());
    for j in ["school.courses", "config.ingest"] { r.q.enqueue(j); }
    r.idle();
    for j in ["school.courses", "config.ingest"] {
        assert!(!r.log.ran(j), "{j} started with no answers file: {:?}", r.log.all());
        assert_eq!(r.state_of(j), JobState::Pending, "{j} waits");
    }
    // The student answers step 3 again: the file is back, and asking again runs them.
    let out = onboarding_answer_inner(&r.dir, 3, &capture_answer(), &mut |_| {});
    assert_eq!(out["ok"], true, "{out}");
    for j in ["school.courses", "config.ingest"] { r.q.enqueue(j); }
    r.idle();
    for j in ["school.courses", "config.ingest"] { assert_eq!(r.state_of(j), JobState::Done, "{j}: {:?}", r.log.all()); }
    // An unreadable file is the same as an absent one.
    std::fs::write(answers_path(&r.dir), "{ broken").unwrap();
    let before = r.log.started().len();
    r.q.enqueue("registrar.confirm");
    r.idle();
    assert_eq!(r.log.started().len(), before, "no config job runs on an unreadable answers file");
}

#[test]
fn test_19_the_finished_file_survives_the_next_quit_and_the_sign_out_arm_starts_no_job_afterwards() {
    let r = rig("t19-finished-survives", finishable(), true);
    let mut seams = Seams::new(&r);
    seams.relaunch_fails = true;
    let guard = ExitGuard::default();
    let out = onboarding_finish_inner(&seams, &guard, epoch(CONFIRMED) + 180);
    assert_eq!(out["restart"], true, "{out}");
    let _ = onboarding_quit_inner(&seams, &guard);
    let Loaded::Ok(after) = load_state(&r.dir) else { panic!("saved") };
    assert!(after.finished, "the quit's save did not write an unfinished copy over finish's write");
    // The sign-out arm: after a failed relaunch the stop holds, the freeze holds, nothing starts.
    let r = rig("t19-signout-quiet", state_at(8, true), true);
    let mut seams = Seams::new(&r);
    seams.relaunch_fails = true;
    let out = onboarding_sign_out_inner(&seams, &ExitGuard::default());
    assert_eq!(out["restart"], true, "{out}");
    let rewound = std::fs::read(state_path(&r.dir)).unwrap();
    let before = r.log.started().len();
    r.q.enqueue("school.capture");
    std::thread::sleep(Duration::from_millis(100));
    assert_eq!(r.log.started().len(), before, "no job starts on the sign-out arm: {:?}", r.log.all());
    assert_eq!(std::fs::read(state_path(&r.dir)).unwrap(), rewound, "and nothing is saved over the rewind");
}

#[test]
fn test_12_a_fragment_a_kept_course_also_contributes_is_never_put_in_course_exclude() {
    // A lecture and its lab share the human code "CS 100" (§14.46); the lab is removed.
    let dir = scratch("t12-shared");
    let pending = FakePending::default();
    let give = |a: Value| {
        let out = onboarding_answer_inner(&dir, 3, &a, &mut |_| {});
        assert_eq!(out["ok"], true, "{out}");
    };
    give(serde_json::json!({"capture": {"courses": [
        course("CS 100", "Intro Computing", "_3001_1"),
        course("CS 100 LAB", "Intro Computing Lab", "_3002_1"),
    ]}}));
    // The pins are gradebook column ids (`_91_1`, `_92_1`), never the course ids (`_3001_1`, `_3002_1`).
    save_pairs(&dir, &pairs_of(&[("_3001_1", &["CS 100", "_91_1"]), ("_3002_1", &["CS 100", "CS 100 LAB", "_92_1"])])).unwrap();
    give(serde_json::json!({"remove": "_3002_1"}));
    run_school_courses(&dir, &[], &mut |c| Ok(c.name.to_lowercase().replace(' ', "-")));
    let h = handoff(&dir, &pending);
    assert_eq!(h.course_exclude, ["CS 100 LAB", "_92_1"], "only the lab's own fragments: {h:?}");
    assert!(h.course_map.iter().any(|(f, _)| f == "CS 100"), "the shared fragment stays in course_map: {h:?}");
    // The lab's pairs, as the preview is given them, still list the shared fragment (they are the lab's own pairs).
    let listed = preview_courses(&load_answers(&dir).unwrap().unwrap());
    assert_eq!(listed[1]["pairs"], serde_json::json!(["CS 100", "CS 100 LAB", "_92_1"]));
}

#[test]
fn test_12_removing_a_past_terms_shell_excludes_only_its_own_pin_and_the_live_shell_keeps_its_note_and_code() {
    // "KNT 220" from spring (`_0990_1`) and from fall (`_1002_1`): one human code, two LMS keys.
    let dir = scratch("t12-shells");
    let pending = FakePending::default();
    let capture = serde_json::json!({"capture": {"courses": [
        course("KNT 220", "Knotwork Theory", "_0990_1"), course("KNT 220", "Knotwork Theory", "_1002_1"),
    ]}});
    assert_eq!(onboarding_answer_inner(&dir, 3, &capture, &mut |_| {})["ok"], true);
    save_pairs(&dir, &pairs_of(&[("_0990_1", &["KNT 220", "_81_1"]), ("_1002_1", &["KNT 220", "_82_1"])])).unwrap();
    assert_eq!(onboarding_answer_inner(&dir, 3, &serde_json::json!({"remove": "_0990_1"}), &mut |_| {})["ok"], true);
    let created = RefCell::new(Vec::<String>::new());
    run_school_courses(&dir, &[], &mut |c| { created.borrow_mut().push(c.lms_key.clone()); Ok("knotwork-theory".into()) });
    assert_eq!(*created.borrow(), ["_1002_1"], "the live shell gets its note; the removed one none");
    let h = handoff(&dir, &pending);
    assert_eq!(h.course_exclude, ["_81_1"], "the spring shell's own pin only; the shared human code is the live shell's too: {h:?}");
    let map_fragments: Vec<&str> = h.course_map.iter().map(|(f, _)| f.as_str()).collect();
    assert_eq!(map_fragments, ["KNT 220", "_82_1"], "{h:?}");
}

#[test]
fn test_03_a_runners_null_proof_count_reaches_the_job_record_and_the_saved_file_as_null() {
    let proof: BTreeMap<String, Option<i64>> = [("assignments_upcoming".to_string(), Some(3)), ("turned_in".to_string(), None)].into();
    let r = rig_with("t03-nullproof", state_at(3, false), false, vec![], |f| {
        let p = proof.clone();
        f.behave("school.preview", move || Outcome::Done(p.clone()));
    });
    r.q.enqueue("school.preview");
    r.idle();
    assert_eq!(r.state_of("school.preview"), JobState::Done);
    assert_eq!(r.job("school.preview").proof, proof, "null stays null on the live state, never 0");
    let Loaded::Ok(saved) = load_state(&r.dir) else { panic!("the queue saves as states change") };
    assert_eq!(saved.jobs["school.preview"].proof, proof, "and in onboarding.json");
}

#[test]
fn test_14_two_kept_shells_of_one_human_code_share_one_note_and_each_key_maps_to_its_slug() {
    // "KNT 220" from spring (`_0990_1`) and fall (`_1002_1`), both kept: one course, so one note
    // (F6a matches by path, `source_uid` or code; the scaffold seeds one note per slug; §14.15 forbids a
    // duplicate), made for the first kept shell in captured order, and both keys name its slug.
    let dir = scratch("t14-two-shells");
    let capture = serde_json::json!({"capture": {"courses": [
        course("KNT 220", "Knotwork Theory", "_0990_1"), course("KNT 220", "Knotwork Theory", "_1002_1"),
    ]}});
    assert_eq!(onboarding_answer_inner(&dir, 3, &capture, &mut |_| {})["ok"], true);
    let created = RefCell::new(Vec::<String>::new());
    let out = run_school_courses(&dir, &[], &mut |c| {
        created.borrow_mut().push(c.lms_key.clone());
        Ok("knotwork-theory".to_string())
    });
    assert!(matches!(out, Outcome::Done(_)), "{out:?}");
    assert_eq!(*created.borrow(), ["_0990_1"], "one create, for the first kept shell; no duplicate note");
    let slugs = load_answers(&dir).unwrap().unwrap().slugs;
    assert_eq!(slugs.get("_0990_1").map(String::as_str), Some("knotwork-theory"), "{slugs:?}");
    assert_eq!(slugs.get("_0990_1"), slugs.get("_1002_1"), "both shells name the one note: {slugs:?}");
    assert!(!slugs.contains_key("KNT 220"), "keyed by each row's key, never by the human code: {slugs:?}");
}

#[test]
fn test_12_every_registrar_row_reaches_registrar_confirm_whatever_was_removed() {
    // §14.47 (G5): the registrar holds only the current term, so a removal never removes a class time.
    let dir = scratch("t12-registrar");
    let rows = vec![
        RegistrarRow { code: "QLT 101".into(), days: "MWF".into(), start: "09:00".into(), end: "09:50".into() },
        RegistrarRow { code: "KNT 220".into(), days: "TR".into(), start: "11:00".into(), end: "12:15".into() },
        RegistrarRow { code: "WVG 105".into(), days: "W".into(), start: "14:00".into(), end: "16:50".into() },
    ];
    let capture = serde_json::json!({"capture": {"courses": three_courses(), "registrar_rows": rows}});
    assert_eq!(onboarding_answer_inner(&dir, 3, &capture, &mut |_| {})["ok"], true);
    for key in ["_1002_1", "_1001_1", "_1003_1"] {
        assert_eq!(onboarding_answer_inner(&dir, 3, &serde_json::json!({"remove": key}), &mut |_| {})["ok"], true);
        let a = load_answers(&dir).unwrap().unwrap();
        assert_eq!(registrar_confirm_rows(&a), rows, "after removing {key}, every row, in captured order");
    }
}

// ---- F1 review r5 ------------------------------------------------------------------------------------------

#[test]
fn test_14_rows_with_no_human_code_are_never_the_same_course_and_a_trimmed_code_match_is() {
    // Two typed rows (the paste fallback) with no code and different labels: two courses. Two shells
    // whose codes differ only by surrounding spaces: one course.
    let dir = scratch("t14-empty-code");
    let capture = serde_json::json!({"capture": {"courses": [
        course("", "Lab", ""), course("", "Seminar", ""),
        course("WVG 105", "Weaving Lab", "_1003_1"), course(" WVG 105 ", "Weaving Lab", "_0993_1"),
    ]}});
    assert_eq!(onboarding_answer_inner(&dir, 3, &capture, &mut |_| {})["ok"], true);
    let created = RefCell::new(Vec::<String>::new());
    let out = run_school_courses(&dir, &[], &mut |c| {
        let k = c.key();
        created.borrow_mut().push(k.clone());
        Ok(k.to_lowercase().replace(['_', ' '], "-"))
    });
    assert!(matches!(out, Outcome::Done(_)), "{out:?}");
    assert_eq!(*created.borrow(), ["Lab", "Seminar", "_1003_1"], "an empty code matches nothing; a trimmed equal code is the same course");
    let slugs = load_answers(&dir).unwrap().unwrap().slugs;
    assert_eq!(slugs.get("Lab").map(String::as_str), Some("lab"), "{slugs:?}");
    assert_eq!(slugs.get("Seminar").map(String::as_str), Some("seminar"), "{slugs:?}");
    assert_ne!(slugs.get("Lab"), slugs.get("Seminar"), "Seminar never takes Lab's note: {slugs:?}");
    assert_eq!(slugs.get("_0993_1"), slugs.get("_1003_1"), "the second WVG shell reuses the first's slug: {slugs:?}");
}

#[test]
fn test_12_a_failed_preview_is_failed_and_leaves_the_saved_buckets_as_they_were() {
    let dir = scratch("t12-preview-fails");
    assert_eq!(onboarding_answer_inner(&dir, 3, &capture_answer(), &mut |_| {})["ok"], true);
    save_preview(&dir, &serde_json::json!({
        "_1001_1": {"codes": ["_1001_1"], "class_times": 0, "assignments_upcoming": 7, "turned_in": 0},
    })).unwrap();
    let before = load_answers(&dir).unwrap().unwrap().buckets;
    let answers: [Result<Value, String>; 4] = [
        Err("knowlu-engine could not be started".into()),
        Ok(serde_json::json!({"by_course": null, "classes": null, "errors": ["courses: not given"]})),
        Ok(serde_json::json!({"classes": 3, "errors": []})),
        Ok(serde_json::json!({"by_course": [1, 2], "errors": []})),
    ];
    for answer in answers {
        let shown = format!("{answer:?}");
        let out = run_school_preview(&dir, &mut |_| answer.clone());
        assert!(matches!(out, Outcome::Failed(_)), "{shown}: {out:?}");
        assert_eq!(load_answers(&dir).unwrap().unwrap().buckets, before, "{shown} changed the saved buckets");
    }
}

// ---- F8a review r1: resume order, wake racing the start, a drop that hangs, the crash reason --------------

fn pending_job() -> JobRecord { JobRecord { state: JobState::Pending, ..Default::default() } }

#[test]
fn test_03_resumed_jobs_run_in_step_and_ask_order_not_alphabetical_order() {
    let mut state = state_at(8, true);
    for j in ["mail.canary", "mail.setup", "mail.first_pass", "coursework.discover", "school.preview", "config.ingest", "school.courses", "school.store"] {
        state.jobs.insert(j.into(), pending_job());
    }
    state.jobs.get_mut("mail.canary").unwrap().state = JobState::Running;
    state.jobs.get_mut("mail.setup").unwrap().state = JobState::Held;
    let r = rig_full("t03-resume", state, true, vec![], true, |_| {});
    r.idle();
    let started = r.log.started();
    let serial: Vec<&str> = started.iter().map(String::as_str).filter(|j| !LANE_JOBS.contains(j)).collect();
    assert_eq!(serial, ["school.preview", "mail.setup", "mail.canary", "mail.first_pass", "coursework.discover"]);
    let lane: Vec<&str> = started.iter().map(String::as_str).filter(|j| LANE_JOBS.contains(j)).collect();
    assert_eq!(lane, ["school.store", "school.courses", "config.ingest"]);
    r.log.assert_clean();
}

/// `entitled()` answers `paid`, except the first read after `refresh` returns, which is stale
/// (false): the start thread's final read, taken before the student's payment reached it. `refresh`
/// waits on a gate.
struct Racy { paid: AtomicBool, stale_next: AtomicBool, refreshing: AtomicBool, gate: Arc<Gate> }
impl Entitlement for Racy {
    fn entitled(&self) -> bool { !self.stale_next.swap(false, Ordering::SeqCst) && self.paid.load(Ordering::SeqCst) }
    fn refresh(&self) {
        self.refreshing.store(true, Ordering::SeqCst);
        self.gate.wait(&AtomicBool::new(false), false);
        self.stale_next.store(true, Ordering::SeqCst);
    }
}

#[test]
fn test_03_a_wake_during_the_start_refresh_is_not_overwritten_by_the_start_reading_stale() {
    let dir = scratch("t03-wake-race");
    let fake = Arc::new(Fake::default());
    let ent = Arc::new(Racy { paid: AtomicBool::new(false), stale_next: AtomicBool::new(false), refreshing: AtomicBool::new(false), gate: Gate::new() });
    let deps = Deps { data_dir: dir.clone(), runner: fake.clone(), entitlement: ent.clone(), sync_view: Arc::new(Syncs(Mutex::new(VecDeque::new()))) };
    let q = Queue::start(deps, state_at(4, true));
    q.enqueue("mail.setup");
    let deadline = Instant::now() + Duration::from_secs(20);
    while !ent.refreshing.load(Ordering::SeqCst) { assert!(Instant::now() < deadline, "the start never refreshed"); std::thread::sleep(Duration::from_millis(2)); }
    ent.paid.store(true, Ordering::SeqCst);
    q.wake();
    ent.gate.release();
    assert!(q.wait_idle(Duration::from_secs(20)), "{:?}", fake.log.all());
    assert_eq!(q.snapshot().jobs["mail.setup"].state, JobState::Done, "held for good by the stale start reading");
}

#[test]
fn test_09_dropping_the_queue_sets_the_stop_so_a_running_job_ends_and_the_joins_return() {
    let r = rig_full("t09-drop", state_at(5, true), true, vec![], false, |_| {});
    r.fake.block("mail.canary", true);
    r.q.enqueue("mail.canary");
    r.wait_started("mail.canary");
    let started = Instant::now();
    let Rig { q, log, .. } = r;
    drop(q);
    assert!(started.elapsed() < Duration::from_secs(10), "the drop waited for a job that honours the stop");
    assert!(log.pos("abandoned:mail.canary").is_some(), "{:?}", log.all());
}

#[test]
fn test_03_a_runner_that_panics_fails_its_job_with_the_closed_set_reason_and_the_queue_goes_on() {
    let r = rig_full("t03-panic", state_at(5, true), true, vec![], false, |f| f.behave("mail.setup", || panic!("a fake runner crash")));
    r.q.enqueue("mail.setup");
    r.q.enqueue("mail.canary");
    r.idle();
    assert_eq!(r.job("mail.setup").state, JobState::Failed);
    assert_eq!(r.job("mail.setup").reason.as_deref(), Some(REASON_JOB_CRASHED));
    assert_eq!(r.state_of("mail.canary"), JobState::Done);
}

#[test]
fn test_03_every_job_name_these_tests_use_is_in_the_resume_order() {
    // Every dotted job-name literal in this file whose first part is a job family. A job left out of
    // `JOB_ORDER` resumes after `slot.first_full` (a missing `calendar.read` once did).
    const FAMILIES: [&str; 8] = ["school", "calendar", "mail", "google", "coursework", "registrar", "config", "slot"];
    let src = std::fs::read_to_string(Path::new(env!("CARGO_MANIFEST_DIR")).join("tests").join("onboarding_jobs.rs")).unwrap();
    let mut seen = std::collections::BTreeSet::new();
    for lit in src.split('"').skip(1).step_by(2) {
        let Some((family, rest)) = lit.split_once('.') else { continue };
        let name_like = |s: &str| !s.is_empty() && s.chars().all(|c| c.is_ascii_lowercase() || c == '_');
        if FAMILIES.contains(&family) && name_like(family) && name_like(rest) && !["rs", "json", "yaml", "md"].contains(&rest) { seen.insert(lit.to_string()); }
    }
    assert!(seen.len() >= 15, "the scan found too few jobs: {seen:?}");
    let missing: Vec<&String> = seen.iter().filter(|j| !JOB_ORDER.contains(&j.as_str())).collect();
    assert!(missing.is_empty(), "not in JOB_ORDER: {missing:?}");
}

// ======================================================================================================
// F2b: one call sets both stops (test 29), the never-driven first pass (test 32's F2b half), the status
// that fails closed (test 28's second half) and the three exits' guards.
// ======================================================================================================

/// A slot runner that, like the real `run_slot_scoped`, looks only at `sch.stop` between its stages
/// (never at the queue's own flag), after waiting at a gate that ignores both.
struct SchStopRunner {
    sch: Arc<knowlu::scheduler::Scheduler>,
    gate: Arc<Gate>,
    log: Log,
}
impl Runner for SchStopRunner {
    fn run(&self, job: &str, _scope: Option<SlotScope>, _queue_stop: &AtomicBool) -> Outcome {
        self.log.push(format!("start:{job}"));
        self.gate.wait(&AtomicBool::new(false), false);
        for step in steps_of(SlotScope::Full) {
            if self.sch.stop.load(Ordering::SeqCst) {
                self.log.push(format!("stopped_before:{step}"));
                return Outcome::Stopped;
            }
            self.log.push(format!("step:{step}"));
        }
        Outcome::Slot(SlotRun { started: "2026-10-24T12:00:00.000Z".into(), steps: vec![], reason: None })
    }
}

#[test]
fn test_29_stop_both_sets_the_schedulers_stop_and_the_queues_so_a_slot_in_flight_starts_no_stage() {
    let dir = scratch("t29");
    save_answers(&dir, &Answers { courses: three_courses(), ..Default::default() }).unwrap();
    let sch = Arc::new(knowlu::scheduler::Scheduler::default());
    let (gate, log) = (Gate::new(), Log::default());
    let deps = Deps {
        data_dir: dir.clone(),
        runner: Arc::new(SchStopRunner { sch: sch.clone(), gate: gate.clone(), log: log.clone() }),
        entitlement: Arc::new(Ent(AtomicBool::new(true), AtomicUsize::new(0))),
        sync_view: Arc::new(Syncs(Mutex::new(VecDeque::new()))),
    };
    let mut state = state_at(11, true);
    state.school = Some(School { unitid: Some("900001".into()), label: "Example Test College".into(), path: SchoolPath::Shared, confirmed: true });
    let q = Queue::start(deps, state);
    q.enqueue("slot.first_full");
    let deadline = Instant::now() + Duration::from_secs(20);
    while log.pos("start:slot.first_full").is_none() {
        assert!(Instant::now() < deadline, "the slot never started");
        std::thread::sleep(Duration::from_millis(5));
    }
    assert!(!sch.stop.load(Ordering::SeqCst) && !q.stop_requested());
    stop_both(&sch, &q);
    assert!(sch.stop.load(Ordering::SeqCst), "the scheduler's stop is set");
    assert!(q.stop_requested(), "the queue's stop is set");
    gate.release();
    q.drain();
    assert!(!log.all().iter().any(|e| e.starts_with("step:")), "no stage started after the call: {:?}", log.all());
    assert!(log.pos("stopped_before:sync").is_some());
    assert_eq!(q.snapshot().jobs["slot.first_full"].state, JobState::Pending, "stopped, so pending");
}

/// The text of the `impl ExitSeams for LiveExit` block's `fn request_stop`.
fn live_request_stop(src: &str) -> String {
    let at = src.find("impl ExitSeams for LiveExit").expect("the live seams");
    let from = &src[at..];
    let s = from.find("fn request_stop").expect("request_stop");
    let body = &from[s..];
    body[..body.find("\n    }").unwrap_or(body.len())].to_string()
}

#[test]
fn test_29_the_live_request_stop_is_stop_both_and_nothing_else_calls_the_queues_request_stop_there() {
    let src = src_of("onboarding_state.rs");
    let live = live_request_stop(&src);
    assert!(live.contains("stop_both("), "{live}");
    assert!(!live.contains("request_stop()") && !live.contains(".stop.store"), "the live seam does only that call:\n{live}");
    let hits: Vec<_> = src.match_indices("queue.request_stop()").collect();
    assert_eq!(hits.len(), 1, "the queue's request_stop is called once in onboarding_state.rs, inside stop_both");
    let f = enclosing_fn(&src, hits[0].0);
    assert!(f.starts_with("pub fn stop_both"), "{f}");
}

// ---- test 30, F8f2's half: the drain never abandons a writer -----------------------------------------------------

#[test]
fn test_30_the_drain_waits_for_a_capture_that_ignores_the_stop_and_nothing_writes_after_the_sign_out() {
    let r = rig("t30-wait", state_at(3, true), true);
    let marker = r.dir.join("answers-marker.txt");
    let m = marker.clone();
    // Ignores the stop until the test releases it, then writes: it stands for the answers file and the pending link.
    let gate = r.fake.block("school.capture", false);
    r.fake.behave("school.capture", move || { std::fs::write(&m, "late").unwrap(); Outcome::Done(BTreeMap::new()) });
    r.q.enqueue("school.capture");
    r.wait_started("school.capture");
    r.q.request_stop();
    let done = AtomicBool::new(false);
    std::thread::scope(|sc| {
        let t = sc.spawn(|| { r.q.drain(); done.store(true, Ordering::SeqCst); });
        // The release comes only once the drain is waiting (it has named the running job).
        let deadline = Instant::now() + Duration::from_secs(20);
        while r.q.exiting().is_none() {
            assert!(Instant::now() < deadline, "the drain never began to wait");
            std::thread::sleep(Duration::from_millis(5));
        }
        std::thread::sleep(Duration::from_millis(200));
        let (returned, written) = (done.load(Ordering::SeqCst), marker.exists());
        gate.release();
        t.join().unwrap();
        assert!(!returned && !written, "the drain returned (or the capture wrote) while the capture ignored the stop");
    });
    assert!(marker.exists(), "the drain returned before the capture wrote");
    assert_eq!(r.state_of("school.capture"), JobState::Done, "waited for, not abandoned");
    std::fs::remove_file(&marker).unwrap(); // the fake `on_sign_out`
    std::thread::sleep(Duration::from_secs(1));
    assert!(!marker.exists(), "something wrote after the sign-out's delete");
    r.log.assert_clean();
}

#[test]
fn test_30_a_fake_canary_and_first_pass_are_abandoned_at_once_and_the_list_is_exactly_those_two() {
    assert_eq!(ABANDON_AT_ONCE, ["mail.canary", "mail.first_pass"], "the one list; a runner that writes anywhere never joins it");
    let r = rig("t30-canary", state_at(5, true), true);
    let gate = r.fake.block("mail.first_pass", false);
    r.q.enqueue("mail.first_pass");
    r.wait_started("mail.first_pass");
    // On its own thread with a deadline: a drain that waited for the first pass fails here, never hangs the suite.
    let done = AtomicBool::new(false);
    let finished = std::thread::scope(|sc| {
        let t = sc.spawn(|| { r.q.drain(); done.store(true, Ordering::SeqCst); });
        let deadline = Instant::now() + Duration::from_secs(2);
        while !done.load(Ordering::SeqCst) && Instant::now() < deadline { std::thread::sleep(Duration::from_millis(5)); }
        let finished = done.load(Ordering::SeqCst);
        gate.release(); // lets a wrongly waiting drain end before the join
        t.join().unwrap();
        finished
    });
    assert!(finished, "the drain waited for the first pass");
    assert_eq!(r.state_of("mail.first_pass"), JobState::Pending);
    let src = src_of("onboarding_jobs.rs");
    assert_eq!(src.matches("abandoned.insert").count(), 1, "one place abandons a job");
    let drain = &src[src.find("pub fn drain(").expect("drain")..];
    let drain = &drain[..drain.find("\n    }").unwrap()];
    for word in ["wait_timeout", "timeout", "Instant", "Duration", "sleep", "return", "break"] {
        assert!(!drain.contains(word), "the drain has no time limit or early way out ({word}):\n{drain}");
    }
    assert!(drain.contains("abandoned.insert") && drain.contains("ABANDON_AT_ONCE.contains"), "the one abandonment reads the list:\n{drain}");
}

/// Blocks `jobs` past the stop, drains on another thread, and returns what `exiting()` said while the
/// drain waited, with the instants around the drain's start.
fn exiting_during_drain(tag: &str, state: OnboardingState, jobs: &[&str]) -> (Option<ExitingNow>, Option<ExitingNow>, String, String) {
    let r = rig(tag, state, true);
    let gates: Vec<_> = jobs.iter().map(|j| r.fake.block(j, false)).collect();
    for j in jobs { r.q.enqueue(j); }
    for j in jobs { r.wait_started(j); }
    r.q.request_stop();
    let before_drain = r.q.exiting();
    let (seen, lo, hi) = std::thread::scope(|sc| {
        let lo = knowlu_engine::journal::now_ts(None);
        let t = sc.spawn(|| r.q.drain());
        let deadline = Instant::now() + Duration::from_secs(20);
        let seen = loop {
            if let Some(e) = r.q.exiting() { break e; }
            assert!(Instant::now() < deadline, "exiting() never named a job while the drain waited");
            std::thread::sleep(Duration::from_millis(5));
        };
        let hi = knowlu_engine::journal::now_ts(None);
        for g in &gates { g.release(); }
        t.join().unwrap();
        (seen, lo, hi)
    });
    assert!(r.q.exiting().is_none(), "nothing running once the drain has returned");
    (before_drain, Some(seen), lo, hi)
}

#[test]
fn test_30_while_the_drain_waits_exiting_names_the_job_long_for_a_slot_only_and_the_drains_start() {
    let (before, seen, lo, hi) = exiting_during_drain("t30-short", state_at(5, true), &["mail.setup"]);
    assert!(before.is_none(), "the stop alone is not a drain");
    let e = seen.unwrap();
    assert_eq!((e.job.as_str(), e.long), ("mail.setup", false), "mail.setup is short once F8d's checks land");
    assert!(e.since.as_str() >= lo.as_str() && e.since.as_str() <= hi.as_str(), "since {} is the drain's start, in [{lo}, {hi}]", e.since);
    let (_, seen, ..) = exiting_during_drain("t30-capture", state_at(3, true), &["school.capture"]);
    assert!(!seen.unwrap().long, "school.capture is not long");
    // A slot beside a running lane job: the page is told about the slot, the longer wait.
    let (_, seen, ..) = exiting_during_drain("t30-slot", state_at(5, false), &["school.store", "slot.partial"]);
    let e = seen.unwrap();
    assert_eq!((e.job.as_str(), e.long), ("slot.partial", true), "a slot is long, and it is the job named when it runs beside a lane job");
}

#[test]
fn test_30_onboarding_status_answers_the_queues_exiting() {
    let src = src_of("onboarding_state.rs");
    let at = src.find("pub fn onboarding_status(").expect("onboarding_status");
    let body = enclosing_fn(&src, at);
    assert!(body.contains("queue.exiting()"), "the status answers the drain's progress:\n{body}");
}

#[test]
fn test_24_the_three_exit_commands_are_async_wrappers_and_the_one_live_drain_is_inside_the_live_seam() {
    let src = src_of("onboarding_state.rs");
    for name in ["onboarding_quit", "onboarding_sign_out", "onboarding_finish"] {
        let at = src.find(&format!("pub fn {name}(")).unwrap_or_else(|| panic!("no command {name}"));
        assert!(src[..at].trim_end().ends_with("#[tauri::command(async)]"), "{name} carries `async`");
    }
    let drains: Vec<usize> = src.match_indices("queue.drain()").map(|(i, _)| i).collect();
    assert_eq!(drains.len(), 1, "one live drain call");
    let live = src.find("impl ExitSeams for LiveExit").expect("the live seams");
    assert!(live < drains[0] && !src[live..drains[0]].contains("\n}\n"), "the drain call is inside the live seam, which only the exits' inners reach");
}

/// `Seams`, with the answers file's recorded provider read back as the real seam reads it.
struct Provided<'a>(Seams<'a>);
impl ExitSeams for Provided<'_> {
    fn request_stop(&self) { self.0.request_stop() }
    fn drain(&self) { self.0.drain() }
    fn save(&self) -> Result<(), String> { self.0.save() }
    fn freeze_saves(&self) { self.0.freeze_saves() }
    fn sign_out(&self) -> Result<(), String> { self.0.sign_out() }
    fn relaunch(&self, args: &[&str]) -> Result<(), String> { self.0.relaunch(args) }
    fn relaunch_with(&self) -> Result<(), String> { self.0.relaunch_with() }
    fn stamp_quit_at(&self) { self.0.stamp_quit_at() }
    fn flush(&self) { self.0.flush() }
    fn exit(&self) { self.0.exit() }
    fn snapshot(&self) -> OnboardingState { self.0.snapshot() }
    fn write_finish(&self, handover: Vec<HandoverItem>) -> Result<(), String> { self.0.write_finish(handover) }
    fn mail_provider(&self) -> Option<String> { recorded_mail_provider(&self.0.dir) }
}

/// A finishable state with the three mail items open.
fn finishable_with_mail() -> OnboardingState {
    let mut s = finishable();
    for j in ["mail.setup", "mail.canary", "mail.first_pass"] { s.jobs.insert(j.into(), JobRecord { state: JobState::Pending, ..Default::default() }); }
    s
}

fn handed_over(tag: &str, provider: Option<&str>) -> Vec<String> {
    // Not entitled, so nothing runs and the three mail items stay open in the snapshot.
    let r = rig(tag, finishable_with_mail(), false);
    if let Some(p) = provider { update_answers(&r.dir, &|a| a.mail_provider = Some(p.into())).unwrap(); }
    let seams = Provided(Seams::new(&r));
    let out = onboarding_finish_inner(&seams, &ExitGuard::default(), epoch(CONFIRMED) + 180);
    assert_eq!(out["ok"], true, "{tag}: {out}");
    let Loaded::Ok(saved) = load_state(&r.dir) else { panic!("saved") };
    assert!(saved.finished);
    saved.handover.iter().filter(|h| h.item.starts_with("mail.")).map(|h| h.item.clone()).collect()
}

#[test]
fn test_32_finish_leaves_a_never_driven_providers_first_pass_out_of_the_handover() {
    assert_eq!(handed_over("t32-m365", Some("m365")), ["mail.canary", "mail.setup"], "held until C3: setup and canary kept, first pass dropped");
    assert_eq!(handed_over("t32-other", Some("other")), ["mail.canary", "mail.setup"], "Guided is never driven either");
    assert_eq!(handed_over("t32-google", Some("google")), ["mail.canary", "mail.first_pass", "mail.setup"], "Gmail's batch closes, so it is waited for");
    assert_eq!(handed_over("t32-none", None), ["mail.canary", "mail.first_pass", "mail.setup"], "no provider recorded keeps the item");
    assert_eq!(handed_over("t32-unknown", Some("something-else")), ["mail.canary", "mail.setup"], "an unknown service string reads as Guided, as from_service does");
}

#[test]
fn test_32_backfill_never_driven_is_true_for_exactly_guided_and_m365() {
    use knowlu::mail_steps::Provider;
    assert!(backfill_never_driven(Provider::Guided));
    assert!(backfill_never_driven(Provider::M365));
    assert!(!backfill_never_driven(Provider::Gmail));
}

// ---- test 28's second half: the status fails closed ---------------------------------------------------------------

#[test]
fn test_28_a_status_whose_jobs_cannot_be_read_is_not_ok_and_carries_no_gate_and_no_item_list() {
    let out = status_for(Err("the onboarding jobs could not be read".into()), None, None, 0);
    assert_eq!(out["ok"], false, "{out}");
    assert!(out["error"].as_str().is_some_and(|e| !e.is_empty()), "{out}");
    for key in ["gate", "jobs", "proposals_ready"] {
        assert!(out.get(key).is_none(), "a failed read answers no {key}: {out}");
    }
}

#[test]
fn test_28_a_status_that_read_its_jobs_carries_the_gate_the_exiting_progress_and_the_step_3_totals() {
    let mut s = finishable();
    s.jobs.insert("slot.first_full".into(), JobRecord { state: JobState::Pending, ..Default::default() });
    s.jobs.insert("first.grades".into(), JobRecord { state: JobState::Failed, reason: Some("grades_read_failed".into()), ..Default::default() });
    s.skips.push(Skip { step: 5, source: "mail".into(), cost_key: COST_MAIL.into() });
    let confirmed = epoch(CONFIRMED);
    let early = status_for(Ok(s.clone()), None, None, confirmed + 10);
    assert_eq!(early["ok"], true, "{early}");
    assert_eq!(early["gate"]["open"], false, "{early}");
    assert_eq!(early["gate"]["still_coming"], serde_json::json!(["slot.first_full"]));
    assert_eq!(early["gate"]["failed"][0]["key"], "first.grades");
    assert_eq!(early["gate"]["failed"][0]["reason"], "grades_read_failed", "a failed item is named, never hidden");
    assert_eq!(early["gate"]["not_connected"][0]["cost_key"], COST_MAIL);
    assert_eq!(early["proposals_ready"], true, "config.ingest is done");
    assert!(early["exiting"].is_null(), "not draining");
    let exiting = ExitingNow { job: "slot.first_full".into(), long: true, since: "2026-10-24T12:03:00Z".into() };
    let capped = status_for(Ok(s), None, Some(exiting), confirmed + 180);
    assert_eq!(capped["gate"]["open"], true, "three minutes after the confirm");
    assert_eq!(capped["exiting"], serde_json::json!({"job": "slot.first_full", "long": true, "since": "2026-10-24T12:03:00Z"}));
    let totals = status_for(Ok(finishable()), Some(&Answers { courses: three_courses(), ..Default::default() }), None, 0);
    assert_eq!(totals["step3"]["classes"], 3);
}

// ---- F2b review r1: the gate's class times, the lane jobs, the sign-out order, the stop's refusals ------------------

#[test]
fn test_09_finish_is_refused_before_step_10_is_confirmed_and_the_gate_lists_class_times() {
    let mut s = finishable();
    s.step10_confirmed_at = None;
    let r = rig("t09-unconfirmed", s.clone(), false);
    let seams = Seams::new(&r);
    let out = onboarding_finish_inner(&seams, &ExitGuard::default(), epoch(CONFIRMED) + 100_000);
    assert_eq!(out["ok"], false, "{out}");
    assert!(r.log.all().is_empty(), "a refusal stops nothing: {:?}", r.log.all());
    let status = status_for(Ok(s), None, None, epoch(CONFIRMED) + 100_000);
    assert_eq!(status["gate"]["open"], false, "no confirm, no cap: {status}");
    assert_eq!(status["gate"]["still_coming"], serde_json::json!(["class_times"]));
    let confirmed = finishable();
    assert_eq!(status_for(Ok(confirmed), None, None, epoch(CONFIRMED))["gate"]["open"], true, "confirmed, nothing else coming");
}

#[test]
fn test_09_finish_needs_the_four_local_jobs_present_and_done_and_only_the_stores_may_be_absent() {
    for missing in ["school.apply", "school.courses", "registrar.confirm", "config.ingest"] {
        let mut s = finishable();
        s.jobs.remove(missing);
        let r = rig(&format!("t09-absent-{missing}"), s, false);
        let seams = Seams::new(&r);
        let out = onboarding_finish_inner(&seams, &ExitGuard::default(), epoch(CONFIRMED) + 600);
        assert_eq!(out["ok"], false, "{missing} absent: {out}");
        assert!(r.log.all().is_empty(), "{missing}: {:?}", r.log.all());
    }
    let mut s = finishable();
    for store in ["school.store", "calendar.store"] { s.jobs.remove(store); }
    let r = rig("t09-no-stores", s, false);
    let out = onboarding_finish_inner(&Seams::new(&r), &ExitGuard::default(), epoch(CONFIRMED) + 600);
    assert_eq!(out["ok"], true, "no store job is fine: {out}");
}

#[test]
fn test_09_a_second_finish_after_the_first_landed_writes_nothing() {
    let r = rig("t09-twice", finishable(), false);
    let seams = Seams::new(&r);
    let guard = ExitGuard::default();
    assert_eq!(onboarding_finish_inner(&seams, &guard, epoch(CONFIRMED) + 180)["ok"], true);
    // In life the first one ended the process; with fakes the guard is still held, so release it.
    guard.0.store(false, Ordering::SeqCst);
    let before = r.log.all();
    let again = onboarding_finish_inner(&seams, &guard, epoch(CONFIRMED) + 180);
    assert_eq!((again["ok"].clone(), again["finished"].clone()), (true.into(), true.into()), "{again}");
    assert_eq!(r.log.all(), before, "no stop, drain, write, relaunch or exit the second time");
    assert!(!guard.0.load(Ordering::SeqCst), "the guard is released");
}

#[test]
fn test_17_sign_out_runs_only_after_the_drain_returns_and_a_writer_that_ignores_the_stop_is_waited_for() {
    let r = rig("t17-drain-first", state_at(8, true), true);
    // A writer that ignores the stop for a moment, standing in for school.capture writing the answers file.
    let capture = r.fake.block("school.capture", false);
    r.q.enqueue("school.capture");
    r.wait_started("school.capture");
    let seams = Seams::new(&r);
    let out = std::thread::scope(|sc| {
        let exit = sc.spawn(|| onboarding_sign_out_inner(&seams, &ExitGuard::default()));
        r.wait_for("the stop", |r| r.q.stop_requested());
        std::thread::sleep(Duration::from_millis(150));
        assert!(r.log.pos("drained").is_none() && !r.log.all().iter().any(|e| e.starts_with("sign_out")), "the sign-out waits for the writer: {:?}", r.log.all());
        capture.release();
        exit.join().unwrap()
    });
    assert_eq!(out["ok"], true, "{out}");
    let order = ["set_stop", "end:school.capture", "drained", "save", "freeze", "sign_out frozen=true file_step=8", "relaunch:--pick", "exit"];
    let at: Vec<usize> = order.iter().map(|e| seams.index(e)).collect();
    assert!(at.windows(2).all(|w| w[0] < w[1]), "stop, the writer's end, drain, save, freeze, sign-out, relaunch, exit: {:?}", r.log.all());
}

#[test]
fn test_12_the_stop_and_the_exit_guard_refuse_an_answer_and_a_skip_through_the_live_check() {
    let r = rig("t12-stop-refuses", state_at(8, true), false);
    let guard = ExitGuard::default();
    assert!(!exiting_now(&r.q, &guard));
    guard.0.store(true, Ordering::SeqCst);
    assert!(exiting_now(&r.q, &guard), "a held exit guard refuses");
    guard.0.store(false, Ordering::SeqCst);
    r.q.request_stop();
    assert!(exiting_now(&r.q, &guard), "the queue's stop refuses");
    let late = serde_json::json!({"zybooks_ignore": "ZYB-LATE"});
    let out = answer_unless_exiting(exiting_now(&r.q, &guard), &r.dir, 8, &late, &mut |_| {});
    assert_eq!(out["ok"], false, "{out}");
    let rows = load_answers(&r.dir).unwrap().map(|a| a.zybooks_ignore).unwrap_or_default();
    assert!(rows.is_empty(), "no late row after the stop: {rows:?}");
    let src = src_of("onboarding_state.rs");
    for name in ["pub fn onboarding_answer(", "pub fn onboarding_skip("] {
        let body = enclosing_fn(&src, src.find(name).unwrap());
        assert!(body.contains("exiting_now("), "{name} asks the live check:\n{body}");
    }
}

#[test]
fn test_17_update_state_rolls_back_a_change_whose_save_failed() {
    let r = rig("t17-rollback", state_at(8, true), false);
    r.q.save().unwrap();
    let path = state_path(&r.dir);
    std::fs::remove_file(&path).unwrap();
    std::fs::create_dir(&path).unwrap();
    let before = r.q.snapshot();
    let out = r.q.update_state(|s| { s.skips.push(Skip { step: 5, source: "mail".into(), cost_key: COST_MAIL.into() }); });
    assert!(out.is_err(), "the save could not land");
    assert_eq!(r.q.snapshot().skips, before.skips, "the change is not kept in memory");
    let _ = std::fs::remove_dir(&path);
}

// ======================================================================================================
// F8a2, test 31: the crash-loop guard (a job found `running` at two starts in a row is paused).
// ======================================================================================================

fn running_with(job: &str, crashes: u8) -> OnboardingState {
    let mut s = state_at(5, true);
    s.jobs.insert(job.into(), JobRecord { state: JobState::Running, crashes, ..Default::default() });
    s
}
fn on_disk(dir: &Path, job: &str) -> JobRecord {
    match load_state(dir) { Loaded::Ok(s) => s.jobs.get(job).cloned().unwrap_or_else(|| panic!("no {job} on disk")), other => panic!("the file: {other:?}") }
}

#[test]
fn test_31_a_job_found_running_is_pending_with_a_count_of_one_on_disk_before_any_job_runs() {
    // The start's entitlement read is held at its refresh, so nothing but `Queue::start` has written yet.
    let dir = scratch("t31-persist");
    let fake = Arc::new(Fake::default());
    let ent = Arc::new(Racy { paid: AtomicBool::new(false), stale_next: AtomicBool::new(false), refreshing: AtomicBool::new(false), gate: Gate::new() });
    let deps = Deps { data_dir: dir.clone(), runner: fake.clone(), entitlement: ent.clone(), sync_view: Arc::new(Syncs(Mutex::new(VecDeque::new()))) };
    let q = Queue::start(deps, running_with("mail.setup", 0));
    let rec = on_disk(&dir, "mail.setup");
    assert_eq!((rec.state, rec.crashes), (JobState::Pending, 1), "start saved the resumed job before it spawned a thread");
    assert!(!fake.log.ran("mail.setup"), "{:?}", fake.log.all());
    ent.paid.store(true, Ordering::SeqCst);
    q.wake();
    ent.gate.release();
    assert!(q.wait_idle(Duration::from_secs(20)), "{:?}", fake.log.all());
    assert_eq!(q.snapshot().jobs["mail.setup"].state, JobState::Done);
}

#[test]
fn test_31_the_second_running_start_pauses_the_job_with_job_crashed_and_never_runs_it() {
    let r = rig_full("t31-pause", running_with("school.capture", 1), true, vec![], false, |_| {});
    r.idle();
    let rec = r.job("school.capture");
    assert_eq!((rec.state, rec.reason.as_deref()), (JobState::Failed, Some(REASON_JOB_CRASHED)));
    assert!(!r.log.ran("school.capture"), "{:?}", r.log.all());
    assert_eq!(on_disk(&r.dir, "school.capture").state, JobState::Failed);
    assert!(JOB_CRASHED_FIX.contains("stopped twice") && JOB_CRASHED_FIX.contains("Try again"), "{JOB_CRASHED_FIX}");
}

#[test]
fn test_31_a_job_that_ends_normally_reads_zero_and_a_zero_is_never_written() {
    let r = rig_full("t31-normal", running_with("school.capture", 0), true, vec![], false, |_| {});
    r.idle();
    assert_eq!((r.state_of("school.capture"), r.job("school.capture").crashes), (JobState::Done, 0));
    let text = std::fs::read_to_string(state_path(&r.dir)).unwrap();
    assert!(!text.contains("crashes"), "{text}");
}

#[test]
fn test_31_an_abandoned_canary_reads_zero_and_its_late_return_keeps_zero() {
    let r = rig_full("t31-abandon", running_with("mail.canary", 0), true, vec![], false, |_| {});
    let gate = r.fake.block("mail.canary", false);
    r.wait_started("mail.canary");
    assert_eq!(r.job("mail.canary").crashes, 1, "the running insert keeps the count it is counting");
    r.q.drain();
    let rec = r.job("mail.canary");
    assert_eq!((rec.state, rec.crashes), (JobState::Pending, 0), "an abandoned job was not running when the process ended");
    gate.release();
    r.wait_log("end:mail.canary");
    r.idle();
    assert_eq!((r.state_of("mail.canary"), r.job("mail.canary").crashes), (JobState::Pending, 0));
}

#[test]
fn test_31_try_again_on_a_job_crashed_job_sets_the_count_to_one_so_one_more_kill_pauses_it() {
    let r = rig_full("t31-retry", running_with("school.capture", 1), true, vec![], false, |_| {});
    r.idle();
    assert_eq!(r.state_of("school.capture"), JobState::Failed);
    let gate = r.fake.block("school.capture", true);
    r.q.retry("school.capture");
    r.wait_started("school.capture");
    let killed = r.q.snapshot();
    assert_eq!((killed.jobs["school.capture"].state, killed.jobs["school.capture"].crashes), (JobState::Running, 1));
    // The process dies here: the next launch finds the file as it stands.
    let next = rig_full("t31-retry-next", killed, true, vec![], false, |_| {});
    next.idle();
    assert_eq!((next.state_of("school.capture"), next.job("school.capture").reason.as_deref()), (JobState::Failed, Some(REASON_JOB_CRASHED)));
    assert!(!next.log.ran("school.capture"));
    gate.release();
}

#[test]
fn test_31_a_file_written_before_the_field_loads_at_version_one_as_zero_and_keeps_its_bytes() {
    let dir = scratch("t31-bytes");
    // Written by hand, in the engine's JSON style (sorted keys), with no `crashes` key anywhere: a file from before the field.
    let old = br#"{"account_settled": true, "finished": false, "handover": [], "jobs": {"mail.setup": {"proof": {}, "reason": "setup_failed", "state": "failed"}, "school.capture": {"proof": {"courses": 3}, "reason": null, "state": "done"}}, "school": null, "skips": [], "started_at": "2026-10-24T09:00:00Z", "step": 5, "step10_confirmed_at": null, "version": 1}"#.to_vec();
    std::fs::write(state_path(&dir), &old).unwrap();
    let Loaded::Ok(loaded) = load_state(&dir) else { panic!("unreadable") };
    assert_eq!(loaded.version, 1);
    assert!(loaded.jobs.values().all(|j| j.crashes == 0));
    save_state(&dir, &loaded).unwrap();
    assert_eq!(std::fs::read(state_path(&dir)).unwrap(), old, "a load and a save change no byte");
    // A count above zero is the only thing that writes the key.
    let mut counted = loaded.clone();
    counted.jobs.get_mut("mail.setup").unwrap().crashes = 1;
    save_state(&dir, &counted).unwrap();
    assert!(String::from_utf8_lossy(&std::fs::read(state_path(&dir)).unwrap()).contains("\"crashes\": 1"));
}

#[test]
fn test_31_a_slot_found_running_twice_on_an_unsettled_account_stays_paused_and_try_again_runs_it_once() {
    let mut state = running_with("slot.partial", 1);
    state.account_settled = false;
    state.jobs.insert("school.courses".into(), pending_job());
    let r = rig_full("t31-slot", state, true, vec![ok_reading(true, 10, 10)], true, |_| {});
    r.idle();
    let rec = on_disk(&r.dir, "slot.partial");
    assert_eq!((rec.state, rec.reason.as_deref(), rec.crashes), (JobState::Failed, Some(REASON_JOB_CRASHED), 2), "start's own slot push never overwrites a paused one");
    assert!(!r.log.ran("slot.partial"), "{:?}", r.log.all());
    assert_eq!(r.state_of("school.courses"), JobState::Pending, "courses waits, with its reason");
    r.q.retry("slot.partial");
    r.idle();
    assert_eq!(r.log.started().iter().filter(|j| *j == "slot.partial").count(), 1);
    assert_eq!((r.state_of("slot.partial"), r.job("slot.partial").crashes), (JobState::Done, 0));
}

fn paused_slot() -> JobRecord { JobRecord { state: JobState::Failed, reason: Some(REASON_JOB_CRASHED.into()), crashes: 2, ..Default::default() } }
fn failed_first_sync() -> JobRecord { JobRecord { state: JobState::Failed, reason: Some(REASON_FIRST_SYNC_FAILED.into()), ..Default::default() } }

#[test]
fn test_31_try_again_on_first_sync_lifts_a_paused_slot_and_runs_it_once() {
    let mut state = state_at(8, true);
    state.jobs.insert(FIRST_SYNC.into(), failed_first_sync());
    state.jobs.insert("slot.partial".into(), paused_slot());
    let r = rig_full("t31-fs-a", state, true, vec![ok_reading(true, 10, 10)], true, |_| {});
    r.idle();
    assert!(!r.log.ran("slot.partial"), "paused until the student presses");
    r.q.retry(FIRST_SYNC);
    r.idle();
    assert_eq!(r.log.started().iter().filter(|j| *j == "slot.partial").count(), 1);
    assert_eq!((r.state_of("slot.partial"), r.job("slot.partial").crashes), (JobState::Done, 0));
    assert_eq!(r.state_of(FIRST_SYNC), JobState::Done, "the first sync was read again");
}

#[test]
fn test_31_one_more_kill_after_the_first_sync_press_pauses_the_slot_again_with_a_way_to_try_again() {
    let mut state = state_at(8, true);
    state.jobs.insert(FIRST_SYNC.into(), failed_first_sync());
    state.jobs.insert("slot.partial".into(), paused_slot());
    let r = rig_full("t31-fs-b", state, true, vec![], true, |_| {});
    r.idle();
    let gate = r.fake.block("slot.partial", true);
    r.q.retry(FIRST_SYNC);
    r.wait_started("slot.partial");
    let killed = r.q.snapshot();
    assert_eq!((killed.jobs["slot.partial"].state, killed.jobs["slot.partial"].crashes), (JobState::Running, 1));
    let next = rig_full("t31-fs-b-next", killed, true, vec![], true, |_| {});
    next.idle();
    let slot = next.job("slot.partial");
    assert_eq!((slot.state, slot.reason.as_deref()), (JobState::Failed, Some(REASON_JOB_CRASHED)), "paused again, with its fix");
    assert!(!next.log.ran("slot.partial"));
    assert_ne!(next.state_of(FIRST_SYNC), JobState::Done);
    gate.release();
}

#[test]
fn test_31_try_again_on_school_courses_of_an_unsettled_account_lifts_a_paused_slot() {
    let mut state = state_at(8, false);
    state.jobs.insert("school.courses".into(), JobRecord { state: JobState::Failed, reason: Some(REASON_ACCOUNT_UNFINISHED.into()), ..Default::default() });
    state.jobs.insert("slot.partial".into(), paused_slot());
    let r = rig_full("t31-sc", state, true, vec![ok_reading(true, 10, 10)], true, |_| {});
    r.idle();
    assert!(!r.log.ran("slot.partial"));
    r.q.retry("school.courses");
    r.idle();
    assert_eq!(r.log.started().iter().filter(|j| *j == "slot.partial").count(), 1);
    assert_eq!(r.state_of("slot.partial"), JobState::Done);
    assert_eq!(r.state_of("school.courses"), JobState::Done, "the settled account lets the courses run");
}

// ---- F8d: the mail, Google and coursework runners (test 30's F8d half) ---------------------------------------------

use knowlu::mail::setup::Progress;

/// A fake setup run: it ends on its own (`alone`), or `end_after` the stop; `progress` reads `ended` only once it has.
struct FakeRun {
    alone: bool,
    end_after: Duration,
    ended: &'static str,
    provider: &'static str,
    stops: AtomicUsize,
    stopped_at: Mutex<Option<Instant>>,
}
impl FakeRun {
    fn new(alone: bool, ended: &'static str, provider: &'static str) -> FakeRun {
        FakeRun { alone, end_after: Duration::from_millis(300), ended, provider, stops: AtomicUsize::new(0), stopped_at: Mutex::new(None) }
    }
    fn done(&self) -> bool { self.alone || self.stopped_at.lock().unwrap().is_some_and(|at| at.elapsed() >= self.end_after) }
}
impl SetupRun for FakeRun {
    fn stop(&self) { self.stops.fetch_add(1, Ordering::SeqCst); self.stopped_at.lock().unwrap().get_or_insert_with(Instant::now); }
    fn wait_done(&self, t: Duration) -> bool { std::thread::sleep(t.min(Duration::from_millis(10))); self.done() }
    fn progress(&self) -> Progress {
        Progress { status: "running", provider: self.provider, ended: self.done().then_some(self.ended), ..Default::default() }
    }
}

#[test]
fn test_30_the_mail_setup_runner_passes_the_stop_on_and_returns_only_after_the_run_has_ended() {
    let dir = scratch("f8d-setup-stop");
    let run = FakeRun::new(false, "stopped", "m365");
    let stop = AtomicBool::new(false);
    let start = Instant::now();
    let out = std::thread::scope(|sc| {
        let t = sc.spawn(|| run_mail_setup(&dir, &run, &stop));
        std::thread::sleep(Duration::from_millis(60));
        assert_eq!(run.stops.load(Ordering::SeqCst), 0, "no stop yet, so no call");
        stop.store(true, Ordering::SeqCst);
        t.join().unwrap()
    });
    assert_eq!(out, Outcome::Stopped, "a stopped setup is pending, never failed");
    assert!(start.elapsed() >= Duration::from_millis(360), "it returned before the run had ended");
    assert_eq!(run.stops.load(Ordering::SeqCst), 1, "the run's stop is called once");
    // The drain waits for this runner, so the write lands before `on_sign_out` and `write_finish`; finish needs it.
    assert_eq!(load_answers(&dir).unwrap().and_then(|a| a.mail_provider).as_deref(), Some("m365"), "the provider is recorded after a stop too");
}

#[test]
fn test_30_a_queue_stop_already_set_stops_the_run_at_once() {
    let dir = scratch("f8d-setup-prestopped");
    let run = FakeRun::new(false, "stopped", "google");
    assert_eq!(run_mail_setup(&dir, &run, &AtomicBool::new(true)), Outcome::Stopped);
    assert_eq!(run.stops.load(Ordering::SeqCst), 1);
}

#[test]
fn test_32_a_finished_setup_records_the_services_provider_string_through_the_answers_file_and_nothing_else_changes() {
    for provider in ["m365", "google", "other"] {
        let dir = scratch(&format!("f8d-provider-{provider}"));
        update_answers(&dir, &|a| a.google_calendar = Some(true)).unwrap();
        let out = run_mail_setup(&dir, &FakeRun::new(true, "finished", provider), &AtomicBool::new(false));
        assert_eq!(out, Outcome::Done(BTreeMap::new()), "{provider}");
        let a = load_answers(&dir).unwrap().unwrap();
        assert_eq!(a.mail_provider.as_deref(), Some(provider), "stored as the service gives it");
        assert_eq!(a.google_calendar, Some(true), "the other answers are kept");
    }
}

#[test]
fn test_32_a_run_that_ended_before_its_provider_was_known_records_nothing() {
    let dir = scratch("f8d-provider-none");
    let out = run_mail_setup(&dir, &FakeRun::new(true, "failed", ""), &AtomicBool::new(false));
    assert!(matches!(out, Outcome::Failed(_)), "{out:?}");
    assert_eq!(load_answers(&dir).unwrap(), None, "no file is started for an empty provider");
}

#[test]
fn test_30_the_other_ends_of_a_setup_map_to_closed_set_reasons_and_never_quote_the_runs_line() {
    let dir = scratch("f8d-setup-ends");
    let go = |ended: &'static str, stop: bool| run_mail_setup(&dir, &FakeRun::new(true, ended, "google"), &AtomicBool::new(stop));
    assert_eq!(go("stopped", false), Outcome::Stopped, "a run that stopped itself is pending");
    assert_eq!(go("cancelled", false), Outcome::Failed(REASON_MAIL_CANCELLED.into()));
    assert_eq!(go("failed", false), Outcome::Failed(REASON_MAIL_SETUP_FAILED.into()));
    assert_eq!(go("needs_you", false), Outcome::Failed(REASON_MAIL_SETUP_FAILED.into()));
    assert_eq!(go("failed", true), Outcome::Stopped, "ended by the stop, so it resumes rather than reading as a failure");
    assert_eq!(go("finished", true), Outcome::Done(BTreeMap::new()), "a setup that finished despite the stop is done");
}

fn counts(k: &str, n: i64) -> BTreeMap<String, Option<i64>> { BTreeMap::from([(k.to_string(), Some(n))]) }

#[test]
fn test_30_the_reads_write_nothing_after_the_stop_and_a_named_outcome_is_never_a_failure() {
    type Run = fn(&AtomicBool, &dyn Fn() -> Result<Read, String>, &dyn Fn(&BTreeMap<String, Option<i64>>) -> Result<(), String>) -> Outcome;
    let runners: [(&str, Run); 3] = [("gmail", run_gmail_first_read), ("calendar", run_calendar_read), ("coursework", run_coursework_discover)];
    for (name, run) in runners {
        let (reads, writes) = (AtomicUsize::new(0), AtomicUsize::new(0));
        let stop = AtomicBool::new(false);
        let read = || { reads.fetch_add(1, Ordering::SeqCst); Ok(Read::Counts(counts("n", 4))) };
        let keep = |_: &BTreeMap<String, Option<i64>>| { writes.fetch_add(1, Ordering::SeqCst); Ok(()) };
        assert_eq!(run(&stop, &read, &keep), Outcome::Done(counts("n", 4)), "{name}");
        assert_eq!((reads.load(Ordering::SeqCst), writes.load(Ordering::SeqCst)), (1, 1), "{name}");
        // The stop is set while the call is in hand: its answer is dropped, nothing is written.
        let late = || { stop.store(true, Ordering::SeqCst); Ok(Read::Counts(counts("n", 4))) };
        assert_eq!(run(&stop, &late, &keep), Outcome::Stopped, "{name}");
        assert_eq!(writes.load(Ordering::SeqCst), 1, "{name}: no write after the stop");
        // A stop already set starts no call at all.
        assert_eq!(run(&stop, &read, &keep), Outcome::Stopped, "{name}");
        assert_eq!(reads.load(Ordering::SeqCst), 1, "{name}: no second call");
        let fresh = AtomicBool::new(false);
        let named = || Ok(Read::Named("no_entitlement".into()));
        assert_eq!(run(&fresh, &named, &keep), Outcome::Waiting("no_entitlement".into()), "{name}: pending with its reason, not failed");
        let bad = || Err("the read failed".to_string());
        assert_eq!(run(&fresh, &bad, &keep), Outcome::Failed("the read failed".into()), "{name}");
        let refuse = |_: &BTreeMap<String, Option<i64>>| Err("disk".to_string());
        assert_eq!(run(&fresh, &read, &refuse), Outcome::Failed("disk".into()), "{name}: a write that fails is failed");
        assert_eq!(writes.load(Ordering::SeqCst), 1, "{name}");
    }
}

#[test]
fn test_30_the_canary_watch_writes_nothing_heed_the_stop_each_poll_and_are_bounded() {
    type Watch = fn(&dyn Fn() -> bool, &AtomicBool, &dyn Fn(Duration)) -> Outcome;
    // F8d2: the first pass no longer polls for a budget; it looks once (`MAIL_FIRST_PASS_LOOKS`, test 34 below).
    let watchers: [(&str, Watch); 1] = [("canary", run_mail_canary)];
    for (name, watch) in watchers {
        let (polls, sleeps) = (AtomicUsize::new(0), AtomicUsize::new(0));
        let sleep = |d: Duration| { assert_eq!(d, MAIL_WATCH_POLL); sleeps.fetch_add(1, Ordering::SeqCst); };
        let stop = AtomicBool::new(false);
        let third = || polls.fetch_add(1, Ordering::SeqCst) >= 2;
        assert_eq!(watch(&third, &stop, &sleep), Outcome::Done(BTreeMap::new()), "{name}");
        assert_eq!(sleeps.load(Ordering::SeqCst), 2, "{name}: waited between polls only");
        let (n, stopper) = (AtomicUsize::new(0), AtomicBool::new(false));
        let stops_on_two = |_: Duration| { if n.fetch_add(1, Ordering::SeqCst) == 1 { stopper.store(true, Ordering::SeqCst); } };
        assert_eq!(watch(&|| false, &stopper, &stops_on_two), Outcome::Stopped, "{name}: pending again, from the top");
        assert_eq!(n.load(Ordering::SeqCst), 2, "{name}: ended within one poll of the stop");
        let (ticks, never) = (AtomicUsize::new(0), AtomicBool::new(false));
        let count = |_: Duration| { ticks.fetch_add(1, Ordering::SeqCst); };
        assert_eq!(watch(&|| false, &never, &count), Outcome::Waiting(REASON_MAIL_NOT_YET.into()), "{name}: parked, not failed");
        assert_eq!(ticks.load(Ordering::SeqCst) as u64, MAIL_WATCH_BUDGET.as_secs() / MAIL_WATCH_POLL.as_secs(), "{name}: bounded by its budget");
    }
}

#[test]
fn test_32_the_setup_runners_only_answers_write_is_one_update_answers_for_mail_provider() {
    let src = src_of("onboarding_jobs.rs");
    let from = src.find("pub fn run_mail_setup(").expect("run_mail_setup");
    let body = &src[from..from + src[from..].find("
}
").expect("its end")];
    assert_eq!(body.matches("update_answers(").count(), 1, "one answers write:
{body}");
    assert!(body.contains("mail_provider = Some("), "and it is the provider:
{body}");
    for other in ["save_answers", "update_answers_unless", "save_state", "load_answers", "std::fs::write"] {
        assert!(!body.contains(other), "no other answers write in the setup runner ({other}):
{body}");
    }
}

// ---- F8d2: the mail queue additions (test 34's F8d2 parts, written first) ---------------------------------------

/// A clock the test moves by hand: `retry_at` is read against it, so no test waits 30 real seconds.
struct FakeClock { base: Instant, moved: Mutex<Duration> }
impl FakeClock {
    fn new() -> Arc<FakeClock> { Arc::new(FakeClock { base: Instant::now(), moved: Mutex::new(Duration::ZERO) }) }
    fn advance(&self, by: Duration) { *self.moved.lock().unwrap() += by; }
}
impl RetryClock for FakeClock {
    fn now(&self) -> Instant { self.base + *self.moved.lock().unwrap() }
}

/// `rig_full`'s queue (answers saved, school confirmed), started over a fake clock.
fn rig_timed(tag: &str, mut state: OnboardingState, entitled: bool, setup: impl FnOnce(&Fake)) -> (Rig, Arc<FakeClock>) {
    let dir = scratch(tag);
    save_answers(&dir, &Answers { courses: three_courses(), ..Default::default() }).unwrap();
    state.school = Some(School { unitid: Some("900001".into()), label: "Example Test College".into(), path: SchoolPath::Shared, confirmed: true });
    let fake = Arc::new(Fake::default());
    setup(&fake);
    let ent = Arc::new(Ent(AtomicBool::new(entitled), AtomicUsize::new(0)));
    let deps = Deps { data_dir: dir.clone(), runner: fake.clone(), entitlement: ent.clone(), sync_view: Arc::new(Syncs(Mutex::new(VecDeque::new()))) };
    let clock = FakeClock::new();
    let q = Queue::start_with_clock(deps, state, clock.clone());
    (Rig { log: fake.log.clone(), q, fake, ent, dir }, clock)
}

fn runs_of(r: &Rig, job: &str) -> usize { r.log.started().iter().filter(|j| *j == job).count() }
fn nap() { std::thread::sleep(Duration::from_millis(600)); }

#[test]
fn test_34_a_job_parked_on_mail_window_busy_is_picked_again_after_30_seconds_and_no_other_reason_is() {
    assert_eq!(TIMER_RETRY_AFTER, Duration::from_secs(30));
    assert_eq!(REASON_MAIL_WINDOW_BUSY, "mail_window_busy");
    let tries = Arc::new(AtomicUsize::new(0));
    let t = tries.clone();
    let (r, clock) = rig_timed("t34-retry", state_at(5, true), true, move |f| {
        f.behave("mail.setup", move || {
            if t.fetch_add(1, Ordering::SeqCst) == 0 { Outcome::Waiting(REASON_MAIL_WINDOW_BUSY.into()) } else { Outcome::Done(BTreeMap::new()) }
        });
        f.behave("mail.first_pass", || Outcome::Waiting(REASON_MAIL_NOT_YET.into()));
    });
    r.q.enqueue("mail.setup");
    r.q.enqueue("mail.first_pass");
    r.wait_for("both to park", |r| r.job("mail.setup").reason.as_deref() == Some(REASON_MAIL_WINDOW_BUSY) && r.job("mail.first_pass").reason.as_deref() == Some(REASON_MAIL_NOT_YET));
    clock.advance(Duration::from_secs(29));
    nap();
    assert_eq!(tries.load(Ordering::SeqCst), 1, "29 s is not 30");
    assert_eq!(runs_of(&r, "mail.first_pass"), 1);
    assert_eq!(r.state_of("mail.setup"), JobState::Pending);
    clock.advance(Duration::from_secs(2));
    r.wait_for("the setup to run again", |r| r.state_of("mail.setup") == JobState::Done);
    assert_eq!(tries.load(Ordering::SeqCst), 2);
    // (A job that ends done unparks the others, as it always has, so the first pass is checked alone below.)
    let (r, clock) = rig_timed("t34-retry-other", state_at(5, true), true, |f| {
        f.behave("mail.first_pass", || Outcome::Waiting(REASON_MAIL_NOT_YET.into()));
    });
    r.q.enqueue("mail.first_pass");
    r.wait_for("the first pass to park", |r| r.job("mail.first_pass").reason.as_deref() == Some(REASON_MAIL_NOT_YET));
    clock.advance(Duration::from_secs(300));
    nap();
    assert_eq!(runs_of(&r, "mail.first_pass"), 1, "a parked mail_not_yet is not on the timer: only mail_window_busy is");
}

#[test]
fn test_34_a_stop_during_the_retry_wait_starts_nothing() {
    let (r, clock) = rig_timed("t34-retry-stop", state_at(5, true), true, |f| {
        f.behave("mail.setup", || Outcome::Waiting(REASON_MAIL_WINDOW_BUSY.into()));
    });
    r.q.enqueue("mail.setup");
    r.wait_for("the setup to park", |r| r.job("mail.setup").reason.as_deref() == Some(REASON_MAIL_WINDOW_BUSY));
    r.q.request_stop();
    clock.advance(Duration::from_secs(120));
    nap();
    assert_eq!(runs_of(&r, "mail.setup"), 1, "the retry timer starts nothing once the stop is set: {:?}", r.log.all());
    assert_eq!(r.state_of("mail.setup"), JobState::Pending);
}

#[test]
fn test_34_skip_mail_with_the_setup_pending_or_held_marks_the_three_jobs_skipped_and_none_is_handed_over() {
    let mail = ["mail.setup", "mail.canary", "mail.first_pass"];
    let (r, _clock) = rig_timed("t34-skip-held", state_at(5, true), false, |_| {});
    for j in mail { r.q.enqueue(j); }
    r.idle();
    for j in mail { assert_eq!(r.state_of(j), JobState::Held, "{j}"); }
    r.q.skip_mail();
    for j in mail { assert_eq!(r.state_of(j), JobState::Skipped, "{j}"); }
    assert!(handover_for(&r.q.snapshot()).iter().all(|h| !h.item.starts_with("mail.")), "a skipped mail job is never handed over");
    r.ent.0.store(true, Ordering::SeqCst);
    r.q.wake();
    r.idle();
    nap();
    for j in mail { assert!(!r.log.ran(j), "{j} ran after the skip: {:?}", r.log.all()); }
    // A job parked on the window timer is skipped too, and the timer does not bring it back.
    let (r, clock) = rig_timed("t34-skip-parked", state_at(5, true), true, |f| {
        f.behave("mail.setup", || Outcome::Waiting(REASON_MAIL_WINDOW_BUSY.into()));
    });
    r.q.enqueue("mail.setup");
    r.wait_for("the setup to park", |r| r.job("mail.setup").reason.as_deref() == Some(REASON_MAIL_WINDOW_BUSY));
    r.q.skip_mail();
    assert_eq!(r.state_of("mail.setup"), JobState::Skipped);
    clock.advance(Duration::from_secs(60));
    nap();
    assert_eq!(runs_of(&r, "mail.setup"), 1);
    assert_eq!(r.state_of("mail.setup"), JobState::Skipped);
    // Other jobs are left alone.
    r.q.enqueue("coursework.discover");
    r.idle();
    assert_eq!(r.state_of("coursework.discover"), JobState::Done);
}

#[test]
fn test_34_skip_mail_with_the_setup_running_stops_the_run_and_the_job_lands_skipped_not_pending() {
    let run = Arc::new(FakeRun::new(false, "stopped", "google"));
    let (r, _clock) = rig_timed("t34-skip-running", state_at(5, true), true, {
        let run = run.clone();
        move |f| {
            f.behave("mail.setup", move || {
                let deadline = Instant::now() + Duration::from_secs(20);
                while run.stops.load(Ordering::SeqCst) == 0 {
                    assert!(Instant::now() < deadline, "the run was never stopped");
                    std::thread::sleep(Duration::from_millis(5));
                }
                Outcome::Stopped
            });
        }
    });
    r.q.attach_setup_run(run.clone());
    for j in ["mail.setup", "mail.canary", "mail.first_pass"] { r.q.enqueue(j); }
    r.wait_started("mail.setup");
    assert_eq!(r.state_of("mail.setup"), JobState::Running);
    assert_eq!(run.stops.load(Ordering::SeqCst), 0, "nothing stops the run before the skip");
    r.q.skip_mail();
    r.wait_for("the setup to land", |r| r.state_of("mail.setup") != JobState::Running);
    assert_eq!(r.state_of("mail.setup"), JobState::Skipped, "Stopped lands as skipped, not pending");
    assert_eq!(run.stops.load(Ordering::SeqCst), 1, "the run's stop is called once");
    r.idle();
    nap();
    for j in ["mail.canary", "mail.first_pass"] {
        assert_eq!(r.state_of(j), JobState::Skipped, "{j}");
        assert!(!r.log.ran(j), "{j} ran");
    }
    assert!(handover_for(&r.q.snapshot()).iter().all(|h| !h.item.starts_with("mail.")));
}

#[test]
fn test_34_a_run_attached_after_the_skip_is_stopped_at_once() {
    let gate = Arc::new(Mutex::new(None));
    let g2 = gate.clone();
    let (r, _clock) = rig_timed("t34-skip-late", state_at(5, true), true, move |f| { *g2.lock().unwrap() = Some(f.block("mail.setup", false)); });
    r.q.enqueue("mail.setup");
    r.wait_started("mail.setup");
    r.q.skip_mail();
    let run = Arc::new(FakeRun::new(false, "stopped", "google"));
    r.q.attach_setup_run(run.clone());
    assert_eq!(run.stops.load(Ordering::SeqCst), 1, "the skip came first, so the run is stopped as it is handed over");
    gate.lock().unwrap().as_ref().unwrap().release();
    r.wait_for("the setup to land", |r| r.state_of("mail.setup") != JobState::Running);
    assert_eq!(r.state_of("mail.setup"), JobState::Done, "a setup that ended on its own despite the skip keeps its truthful end");
}

#[test]
fn test_34_mail_first_pass_looks_once_parks_and_runs_again_after_mail_batch_closed() {
    assert_eq!(MAIL_FIRST_PASS_LOOKS, 1);
    // The runner itself: one look, no sleep.
    let looks = AtomicUsize::new(0);
    let never_sleep = |_: Duration| panic!("the first pass never waits");
    let out = run_mail_first_pass(&|| { looks.fetch_add(1, Ordering::SeqCst); false }, &AtomicBool::new(false), &never_sleep);
    assert_eq!(out, Outcome::Waiting(REASON_MAIL_NOT_YET.into()));
    assert_eq!(looks.load(Ordering::SeqCst), MAIL_FIRST_PASS_LOOKS as usize);
    assert_eq!(run_mail_first_pass(&|| true, &AtomicBool::new(false), &never_sleep), Outcome::Done(BTreeMap::new()));
    assert_eq!(run_mail_first_pass(&|| false, &AtomicBool::new(true), &never_sleep), Outcome::Stopped);
    // On the queue: it parks, does not spin, and wakes when a batch closes.
    let (closed, seen) = (Arc::new(AtomicBool::new(false)), Arc::new(AtomicUsize::new(0)));
    let (c, s) = (closed.clone(), seen.clone());
    let (r, _clock) = rig_timed("t34-first-pass", state_at(5, true), true, move |f| {
        f.behave("mail.first_pass", move || {
            run_mail_first_pass(&|| { s.fetch_add(1, Ordering::SeqCst); c.load(Ordering::SeqCst) }, &AtomicBool::new(false), &|_| panic!("no wait"))
        });
    });
    r.q.enqueue("mail.first_pass");
    r.wait_for("the first pass to park", |r| r.job("mail.first_pass").reason.as_deref() == Some(REASON_MAIL_NOT_YET));
    nap();
    assert_eq!(seen.load(Ordering::SeqCst), 1, "one look, and no spin while parked");
    closed.store(true, Ordering::SeqCst);
    r.q.mail_batch_closed();
    r.wait_for("the first pass to be done", |r| r.state_of("mail.first_pass") == JobState::Done);
    assert_eq!(seen.load(Ordering::SeqCst), 2, "it looked again after the batch closed");
}

// ---- F8d2 fix round: the skip seam, a failed end after a skip, and the retry spacing ------------------------------------

#[test]
fn test_34_a_skip_that_lands_before_any_run_is_attached_is_readable_through_mail_skipped() {
    let gate = Arc::new(Mutex::new(None));
    let g2 = gate.clone();
    let (r, _clock) = rig_timed("t34-skipped-seam", state_at(5, true), true, move |f| { *g2.lock().unwrap() = Some(f.block("mail.setup", false)); });
    assert!(!r.q.mail_skipped("mail.setup"), "not before the skip");
    r.q.enqueue("mail.setup");
    r.wait_started("mail.setup");
    assert!(!r.q.mail_skipped("mail.setup"), "running, not skipped");
    r.q.skip_mail(); // no setup run has been attached: the marker is all the glue can read
    assert!(r.q.mail_skipped("mail.setup"), "the glue checks this on each window poll and before start_setup");
    assert!(!r.q.mail_skipped("mail.canary"), "only the running job is marked");
    gate.lock().unwrap().as_ref().unwrap().release();
    r.wait_for("the setup to land", |r| r.state_of("mail.setup") != JobState::Running);
    assert!(!r.q.mail_skipped("mail.setup"), "the marker goes when the job ends");
}

#[test]
fn test_34_a_failed_end_after_a_skip_lands_skipped_and_a_done_end_stays_done() {
    for (outcome, want) in [(Outcome::Failed(REASON_MAIL_SETUP_FAILED.into()), JobState::Skipped), (Outcome::Done(BTreeMap::new()), JobState::Done)] {
        let gate = Arc::new(Mutex::new(None));
        let g2 = gate.clone();
        let (r, _clock) = rig_timed("t34-skip-failed", state_at(5, true), true, move |f| {
            *g2.lock().unwrap() = Some(f.block("mail.setup", false));
            f.behave("mail.setup", move || outcome.clone());
        });
        r.q.enqueue("mail.setup");
        r.wait_started("mail.setup");
        r.q.skip_mail();
        gate.lock().unwrap().as_ref().unwrap().release();
        r.wait_for("the setup to land", |r| r.state_of("mail.setup") != JobState::Running);
        assert_eq!(r.state_of("mail.setup"), want);
    }
}

#[test]
fn test_34_a_parked_window_busy_job_is_not_re_picked_by_other_jobs_finishing_before_its_30_seconds() {
    let (r, clock) = rig_timed("t34-spacing", state_at(5, true), true, |f| {
        f.behave("mail.setup", || Outcome::Waiting(REASON_MAIL_WINDOW_BUSY.into()));
    });
    r.q.enqueue("mail.setup");
    r.wait_for("the setup to park", |r| r.job("mail.setup").reason.as_deref() == Some(REASON_MAIL_WINDOW_BUSY));
    // A lane-1 job ends done, which unparks every parked job; the retry time still holds the setup back.
    r.q.enqueue("calendar.store");
    r.wait_for("the lane job", |r| r.state_of("calendar.store") == JobState::Done);
    // And a serial job, for the other thread.
    r.q.enqueue("coursework.discover");
    r.wait_for("the serial job", |r| r.state_of("coursework.discover") == JobState::Done);
    nap();
    assert_eq!(runs_of(&r, "mail.setup"), 1, "re-picked before its 30 s: {:?}", r.log.all());
    clock.advance(Duration::from_secs(29));
    nap();
    assert_eq!(runs_of(&r, "mail.setup"), 1);
    clock.advance(Duration::from_secs(2));
    r.wait_for("the setup to be picked again", |r| runs_of(r, "mail.setup") >= 2);
}

// ---- F2b2a: the step-5 answer, the proceeds and the Skip over the real queue and the fake runner -------------------------

/// What the `onboarding_answer` command does: the confirmed-school fact comes from the queue's live state.
fn answer_through(r: &Rig, step: u8, a: Value) -> Value {
    let out = answer_unless_exiting_with(&|| false, &r.dir, step, &a, &mut answer_enqueuer(&r.q), school_apply_may_run(&r.q.snapshot()));
    if out["ok"] == true { r.q.wake(); }
    out
}

#[test]
fn test_33_the_unlisted_paste_path_with_no_registrar_rows_ends_with_the_four_lane_jobs_done_and_finish_ready() {
    let mut s = state_at(3, true);
    s.school = Some(School { unitid: None, label: "Example Test College".into(), path: SchoolPath::Paste, confirmed: true });
    let r = rig_full("t33-e2e", s, true, vec![ok_reading(true, 10, 10)], false, |_| {});
    let typed = serde_json::json!({"capture": {"courses": three_courses(), "registrar_rows": [], "feeds": {}}});
    assert_eq!(answer_through(&r, 3, typed)["ok"], true);
    for step in [3u8, 7, 8] {
        let out = answer_through(&r, step, serde_json::json!({"proceed": true}));
        assert_eq!(out["ok"], true, "step {step}: {out}");
    }
    r.idle();
    for j in ["school.apply", "school.courses", "registrar.confirm", "config.ingest", "school.store"] {
        assert_eq!(r.state_of(j), JobState::Done, "{j}: {:?}", r.log.all());
    }
    r.q.update_state(|s| s.step10_confirmed_at = Some(CONFIRMED.into())).unwrap();
    let out = status_inner(&r.q.snapshot(), epoch(CONFIRMED) + 180);
    assert_eq!(out["finish_ready"], true, "{out}");
}

#[test]
fn test_33_an_unconfirmed_school_refuses_step_3s_proceed_and_the_queue_holds_no_lane_job() {
    let mut s = state_at(3, true);
    s.school = Some(School { unitid: Some("900001".into()), label: "Example Test College".into(), path: SchoolPath::Shared, confirmed: false });
    let r = rig("t33-unconfirmed", s, true);
    // `rig` confirms nothing for a school it was given; the queue's live state is what the wrapper reads.
    let out = answer_through(&r, 3, serde_json::json!({"proceed": true}));
    assert_eq!(out["ok"], false, "{out}");
    r.idle();
    assert!(r.q.snapshot().jobs.is_empty(), "nothing enqueued: {:?}", r.q.snapshot().jobs);
    r.q.confirm_school();
    assert_eq!(answer_through(&r, 3, serde_json::json!({"proceed": true}))["ok"], true, "refused only while unconfirmed");
}

#[test]
fn test_33_a_repeated_step_5_answer_leaves_one_copy_of_each_mail_job_and_each_runs_once() {
    let mut gate = None;
    let r = rig_with("t33-repeat", state_at(5, true), true, vec![], |f| { gate = Some(f.block("mail.setup", false)); });
    let school = serde_json::json!({"mail": {"provider": "school"}});
    assert_eq!(answer_through(&r, 5, school.clone())["ok"], true);
    r.wait_started("mail.setup");
    assert_eq!(answer_through(&r, 5, school)["ok"], true);
    gate.unwrap().release();
    r.idle();
    for j in ["mail.setup", "mail.canary", "mail.first_pass"] { assert_eq!(runs_of(&r, j), 1, "{j}: {:?}", r.log.all()); }
}

#[test]
fn test_34_the_skip_on_step_5_records_the_skip_and_skips_the_queued_mail_jobs_and_a_running_setup_lands_skipped() {
    let mut gate = None;
    let r = rig_with("t34-skip-5", state_at(5, true), true, vec![], |f| {
        gate = Some(f.block("mail.setup", false));
        f.behave("mail.setup", || Outcome::Failed(REASON_MAIL_SETUP_FAILED.into()));
    });
    assert_eq!(answer_through(&r, 5, serde_json::json!({"mail": {"provider": "school"}}))["ok"], true);
    r.wait_started("mail.setup");
    let out = skip_and_stop_mail(false, 5, "mail", &mut || r.q.update_state(|s| skip_step(s, 5, "mail")).and_then(|x| x), &mut || r.q.skip_mail());
    assert_eq!(out["ok"], true, "{out}");
    assert!(r.q.snapshot().skips.iter().any(|k| k.step == 5 && k.source == "mail"), "the skip is recorded");
    assert!(r.q.mail_skipped("mail.setup"), "the running setup is marked for the glue");
    for j in ["mail.canary", "mail.first_pass"] { assert_eq!(r.state_of(j), JobState::Skipped, "{j}"); }
    gate.unwrap().release();
    r.wait_for("the setup to land", |r| r.state_of("mail.setup") != JobState::Running);
    assert_eq!(r.state_of("mail.setup"), JobState::Skipped, "a failed end after a skip lands skipped");
    assert!(handover_for(&r.q.snapshot()).iter().all(|h| !h.item.starts_with("mail.")), "a skipped mail job is never handed over: {:?}", handover_for(&r.q.snapshot()));
}

// ---- F2b2a fix round: a repeat of an answer or a proceed changes nothing once its job has finished ----------------------

fn proceed(r: &Rig, step: u8) { assert_eq!(answer_through(r, step, serde_json::json!({"proceed": true}))["ok"], true, "step {step}"); }

/// The unlisted paste path's rig: a confirmed school without a unitid, the typed list answered.
fn unlisted_rig(tag: &str, setup: impl FnOnce(&Fake)) -> Rig {
    let mut s = state_at(3, true);
    s.school = Some(School { unitid: None, label: "Example Test College".into(), path: SchoolPath::Paste, confirmed: true });
    let r = rig_full(tag, s, true, vec![ok_reading(true, 10, 10)], false, setup);
    let typed = serde_json::json!({"capture": {"courses": three_courses(), "registrar_rows": [], "feeds": {}}});
    assert_eq!(answer_through(&r, 3, typed)["ok"], true);
    r
}

#[test]
fn test_33_a_repeated_step_5_answer_after_setup_and_canary_are_done_starts_nothing() {
    let r = rig_with("t33-repeat-done", state_at(5, true), true, vec![], |_| {});
    let school = serde_json::json!({"mail": {"provider": "school"}});
    assert_eq!(answer_through(&r, 5, school.clone())["ok"], true);
    r.idle();
    for j in ["mail.setup", "mail.canary"] { assert_eq!(r.state_of(j), JobState::Done, "{j}"); }
    assert_eq!(answer_through(&r, 5, school)["ok"], true, "the answer still saves");
    r.idle();
    for j in ["mail.setup", "mail.canary", "mail.first_pass"] { assert_eq!(runs_of(&r, j), 1, "{j} ran again: {:?}", r.log.all()); }
    for j in ["mail.setup", "mail.canary"] { assert_eq!(r.state_of(j), JobState::Done, "{j}"); }
}

#[test]
fn test_33_a_repeated_step_3_or_8_proceed_after_its_jobs_are_done_re_runs_nothing() {
    let r = unlisted_rig("t33-repeat-proceeds", |_| {});
    proceed(&r, 3);
    proceed(&r, 8);
    r.idle();
    let lane = ["school.store", "school.apply", "school.courses", "registrar.confirm", "config.ingest"];
    for j in lane { assert_eq!(r.state_of(j), JobState::Done, "{j}"); }
    proceed(&r, 3);
    proceed(&r, 8);
    r.idle();
    for j in lane { assert_eq!(runs_of(&r, j), 1, "{j} ran again: {:?}", r.log.all()); assert_eq!(r.state_of(j), JobState::Done, "{j}"); }
}

#[test]
fn test_33_a_failed_job_stays_failed_after_a_repeated_proceed() {
    let r = unlisted_rig("t33-repeat-failed", |f| f.behave("calendar.store", || Outcome::Failed("could_not_store".into())));
    proceed(&r, 7);
    r.idle();
    assert_eq!(r.state_of("calendar.store"), JobState::Failed);
    proceed(&r, 7);
    r.idle();
    assert_eq!(runs_of(&r, "calendar.store"), 1, "only Try again re-runs a failed job: {:?}", r.log.all());
    assert_eq!(r.state_of("calendar.store"), JobState::Failed);
}

#[test]
fn test_33_a_step_5_answer_after_a_step_5_skip_saves_but_leaves_the_mail_jobs_skipped() {
    let mut gate = None;
    let r = rig_with("t33-answer-after-skip", state_at(5, true), true, vec![], |f| {
        gate = Some(f.block("mail.setup", false));
        f.behave("mail.setup", || Outcome::Failed(REASON_MAIL_SETUP_FAILED.into()));
    });
    let school = serde_json::json!({"mail": {"provider": "school"}});
    assert_eq!(answer_through(&r, 5, school.clone())["ok"], true);
    r.wait_started("mail.setup");
    skip_and_stop_mail(false, 5, "mail", &mut || r.q.update_state(|s| skip_step(s, 5, "mail")).and_then(|x| x), &mut || r.q.skip_mail());
    gate.unwrap().release();
    r.wait_for("the setup to land", |r| r.state_of("mail.setup") != JobState::Running);
    assert_eq!(answer_through(&r, 5, school)["ok"], true, "the answer saves");
    r.idle();
    assert_eq!(load_answers(&r.dir).unwrap().unwrap().mail_kind.as_deref(), Some("school"));
    for j in ["mail.setup", "mail.canary", "mail.first_pass"] { assert_eq!(r.state_of(j), JobState::Skipped, "{j}"); }
    assert_eq!((runs_of(&r, "mail.setup"), runs_of(&r, "mail.canary")), (1, 0), "{:?}", r.log.all());
}

// ---- F2b2a fix round 2: the preview refreshes, an early skip holds, and the slot is asked for only on new work ----------

#[test]
fn test_33_a_second_step_3_capture_after_a_done_preview_runs_the_preview_again() {
    let r = unlisted_rig("t33-recapture", |_| {});
    r.idle();
    assert_eq!((r.state_of("school.preview"), runs_of(&r, "school.preview")), (JobState::Done, 1));
    let again = serde_json::json!({"capture": {"courses": three_courses(), "registrar_rows": [], "feeds": {}}});
    assert_eq!(answer_through(&r, 3, again)["ok"], true);
    r.idle();
    assert_eq!(runs_of(&r, "school.preview"), 2, "the new list gets its own preview: {:?}", r.log.all());
}

#[test]
fn test_34_a_skip_made_before_any_mail_job_exists_holds_against_a_later_step_5_answer() {
    let r = rig_with("t34-early-skip", state_at(5, true), true, vec![], |_| {});
    assert!(r.q.snapshot().jobs.get("mail.setup").is_none(), "no mail job yet");
    let out = skip_and_stop_mail(false, 5, "mail", &mut || r.q.update_state(|s| skip_step(s, 5, "mail")).and_then(|x| x), &mut || r.q.skip_mail());
    assert_eq!(out["ok"], true, "{out}");
    assert_eq!(answer_through(&r, 5, serde_json::json!({"mail": {"provider": "school"}}))["ok"], true, "the answer saves");
    r.idle();
    for j in ["mail.setup", "mail.canary", "mail.first_pass"] { assert_eq!((r.state_of(j), runs_of(&r, j)), (JobState::Skipped, 0), "{j}: {:?}", r.log.all()); }
    assert!(handover_for(&r.q.snapshot()).iter().all(|h| !h.item.starts_with("mail.")), "never handed over");
}

#[test]
fn test_33_a_repeated_step_3_and_8_proceed_after_everything_is_done_leaves_the_slot_run_count_unchanged() {
    let r = unlisted_rig("t33-slot-repeat", |_| {});
    proceed(&r, 3);
    proceed(&r, 8);
    r.idle();
    let slots = runs_of(&r, "slot.partial");
    assert!(slots >= 1, "the first proceeds asked for it: {:?}", r.log.all());
    proceed(&r, 3);
    proceed(&r, 8);
    r.idle();
    assert_eq!(runs_of(&r, "slot.partial"), slots, "a repeat with no new work costs no slot: {:?}", r.log.all());
}

#[test]
fn test_33_step_8s_first_proceed_still_queues_its_no_rank_slot_after_step_3s() {
    let r = unlisted_rig("t33-slot-step8", |_| {});
    proceed(&r, 3);
    r.idle();
    let after_3 = runs_of(&r, "slot.partial");
    proceed(&r, 8);
    r.idle();
    assert_eq!(runs_of(&r, "slot.partial"), after_3 + 1, "{:?}", r.log.all());
    assert_eq!(r.log.scopes("slot.partial").last().map(String::as_str), Some("NoRank"), "{:?}", r.log.all());
}

#[test]
fn test_33_a_proceed_after_a_sign_out_rewind_queues_its_slot_again() {
    let r = unlisted_rig("t33-slot-rewind", |_| {});
    proceed(&r, 3);
    proceed(&r, 8);
    r.idle();
    let before = runs_of(&r, "slot.partial");
    // What `on_sign_out_with` does to the jobs: every record back to its default.
    r.q.update_state(|s| for j in s.jobs.values_mut() { *j = JobRecord::default(); }).unwrap();
    proceed(&r, 3); // the student redoes the steps; step 8 alone would wait on the rewound school jobs
    proceed(&r, 8);
    r.idle();
    assert_eq!(runs_of(&r, "config.ingest"), 2);
    assert!(runs_of(&r, "slot.partial") > before, "{:?}", r.log.all());
}
