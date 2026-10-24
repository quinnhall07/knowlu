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
    /// Every required item still pending, plus every backfill in progress.
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
/// failed or skipped item is listed whether or not it is connected (a skip with its cost key in
/// `not_connected`).
pub fn gate(items: &[GateItem], step10_confirmed_at_secs: Option<i64>, now_secs: i64) -> GateOutput {
    let mut out = GateOutput { open: false, still_coming: vec![], failed: vec![], not_connected: vec![] };
    let mut required_pending = false;
    for it in items {
        match &it.state {
            ItemState::Skipped { cost_key } => {
                out.not_connected.push(NotConnected { key: it.key.clone(), cost_key: cost_key.clone() });
            }
            ItemState::Failed { reason, fix } => out.failed.push((it.key.clone(), reason.clone(), fix.clone())),
            _ if !it.connected => {}
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

/// "First grades" is marked only by a slot whose `grades` ran and succeeded (T0, 2026-10-24; review
/// r1, I1): the bare `grades` row at exit 0. A `grades (not yet: …)` or `grades (skipped: …)` row never
/// marks it, and a bare row with a non-zero exit is [`first_grades_failed`], never done.
pub fn first_grades_marked(steps: &[(String, i32)]) -> bool {
    steps.iter().any(|(name, code)| name == "grades" && *code == 0)
}

/// A slot whose `grades` ran and failed: the bare `grades` row at a non-zero exit. The caller makes
/// `first.grades` an [`ItemState::Failed`] (reason and fix are its to assign); the scoped
/// `(not yet: …)` and `(skipped: …)` rows are neither marked nor failed.
pub fn first_grades_failed(steps: &[(String, i32)]) -> bool {
    steps.iter().any(|(name, code)| name == "grades" && *code != 0)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn row(name: &str, code: i32) -> Vec<(String, i32)> {
        vec![("sync".to_string(), 0), (name.to_string(), code)]
    }

    #[test]
    fn a_bare_grades_row_marks_at_exit_zero_and_fails_at_any_other_exit() {
        assert!(first_grades_marked(&row("grades", 0)));
        assert!(!first_grades_failed(&row("grades", 0)));
        for code in [1, 2, -1] {
            assert!(!first_grades_marked(&row("grades", code)), "exit {code} never marks");
            assert!(first_grades_failed(&row("grades", code)), "exit {code} is failed");
        }
    }

    #[test]
    fn scoped_grades_rows_are_neither_marked_nor_failed() {
        for name in ["grades (not yet: your courses are not in yet)", "grades (skipped: not connected)"] {
            for code in [0, 1] {
                assert!(!first_grades_marked(&row(name, code)), "{name} at {code}");
                assert!(!first_grades_failed(&row(name, code)), "{name} at {code}");
            }
        }
        assert!(!first_grades_failed(&[("sync".to_string(), 1)]), "another step's failure is not grades'");
    }

    #[test]
    fn a_failed_item_that_is_not_connected_is_still_listed() {
        let items = [GateItem {
            key: "mail".into(),
            state: ItemState::Failed { reason: "r".into(), fix: "f".into() },
            connected: false,
            backfill: false,
        }];
        let g = gate(&items, None, 0);
        assert_eq!(g.failed, [("mail".to_string(), "r".to_string(), "f".to_string())]);
        assert!(g.open, "an unconnected item never holds the gate");
    }
}
