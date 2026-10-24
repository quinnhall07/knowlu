//! The reveal gate (plan §4.6, F9): a pure function over the jobs' states, the skips,
//! `step10_confirmed_at` and now.
//!
//! **F1 stub.** The types are the contract `app/tests/onboarding_jobs.rs` writes against; the
//! functions panic until F9 replaces them.

/// The cap: 3 minutes after step 10's confirm (Q7).
pub const CAP_SECS: i64 = 180;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ItemState {
    Pending,
    Done,
    /// A closed-set reason and its fix, never hidden.
    Failed { reason: String, fix: String },
    /// The student's skip, with the cost key the rail and the reveal name.
    Skipped { cost_key: String },
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GateItem {
    /// `first.sync`, `first.coursework`, `first.ingest`, `first.grades`, `class_times`, `mail`,
    /// `gmail`, `effort_ranking`, or a backfill (`backfill.*`).
    pub key: String,
    pub state: ItemState,
    /// False for a source the student never connected: not required, not listed.
    pub connected: bool,
    /// A backfill in progress: listed as still coming, never waited for.
    pub backfill: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NotConnected {
    pub key: String,
    pub cost_key: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GateOutput {
    pub open: bool,
    /// Every required item not yet `done`, plus every backfill in progress.
    pub still_coming: Vec<String>,
    /// `(key, reason, fix)` of each failed item.
    pub failed: Vec<(String, String, String)>,
    pub not_connected: Vec<NotConnected>,
}

/// Opens when every required item is done, failed or skipped, or when `now_secs` is
/// [`CAP_SECS`] after `step10_confirmed_at_secs`, whichever is first.
pub fn gate(_items: &[GateItem], _step10_confirmed_at_secs: Option<i64>, _now_secs: i64) -> GateOutput {
    todo!("F9: the reveal gate")
}

/// "First grades" is marked only by a slot whose `grades` ran (T0, 2026-10-24): a row named
/// `grades (not yet: …)` or `grades (skipped: …)` never marks it.
pub fn first_grades_marked(_steps: &[(String, i32)]) -> bool {
    todo!("F9: which slot marks first grades")
}
