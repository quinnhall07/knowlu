//! The reveal gate (plan §4.6, F9): a pure function over the jobs' states, the skips,
//! `step10_confirmed_at` and now.
//!
//! The types are the contract `app/tests/onboarding_jobs.rs` (F1 test 6) writes against.

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
/// [`CAP_SECS`] after `step10_confirmed_at_secs`, whichever is first. Pure: the caller passes the
/// clock, and the output lists follow the input order.
///
/// Required means connected and not a backfill. A backfill is listed as still coming while it is
/// pending and never holds the gate. A failed item is listed in `failed`, not in `still_coming`; a
/// skipped item is listed in `not_connected` with its cost key, whether or not it is connected.
pub fn gate(items: &[GateItem], step10_confirmed_at_secs: Option<i64>, now_secs: i64) -> GateOutput {
    let mut out = GateOutput { open: false, still_coming: vec![], failed: vec![], not_connected: vec![] };
    let mut required_pending = false;
    for it in items {
        match &it.state {
            ItemState::Skipped { cost_key } => {
                out.not_connected.push(NotConnected { key: it.key.clone(), cost_key: cost_key.clone() });
            }
            _ if !it.connected => {}
            ItemState::Failed { reason, fix } => out.failed.push((it.key.clone(), reason.clone(), fix.clone())),
            ItemState::Pending => {
                out.still_coming.push(it.key.clone());
                required_pending |= !it.backfill;
            }
            ItemState::Done => {}
        }
    }
    let capped = step10_confirmed_at_secs.is_some_and(|c| now_secs >= c.saturating_add(CAP_SECS));
    out.open = !required_pending || capped;
    out
}

/// "First grades" is marked only by a slot whose `grades` ran (T0, 2026-10-24): a row named
/// `grades (not yet: …)` or `grades (skipped: …)` never marks it, so only the bare `grades` row does.
/// The exit code is not read here: a failed `grades` ran, and its failure is the caller's to name.
pub fn first_grades_marked(steps: &[(String, i32)]) -> bool {
    steps.iter().any(|(name, _)| name == "grades")
}
