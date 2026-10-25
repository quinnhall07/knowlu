# T1c and T3b review: the walk (diff ba6b8af..890a177)

Reviewer: `reviewer` (read-only; the controller filed this report). **Verdict: PASS**, with no
Critical or Important findings and three Minor ones. Only `engine/src/ranking.rs` changes.

## Checked
- **Tests match the plan.** DP11, DP12, DP13, DP14 (both cases, plus DP21's set without D) and DP20
  were re-derived by hand. The records hold `prefix`, `prefix_minutes`, `overflower` and
  `overflow_deadline`.
- **`prefix`** is built from `kept` right after the removal. The overflower is the last item pushed.
  Earlier removals and the removed item itself are excluded.
- **Several removals.** Each addition is checked against its own deadline. Removing the longest item
  restores a prefix that already fit. This is Moore–Hodgson as spec §2.6 states it, with no margin.
- **Determinism.** The keys are unique `(DateTime, usize)` and `(work, deadline, index)`. No floats
  enter the walk.
- **Rounding.** Integer minutes throughout (§5).
- **Ruling (c), `wont_fit` ordered by (deadline, scored index), agrees with T4.** Spec §5 and §2.8
  give this order. `wont_fit` and `W`'s tail come from one sorted list. T4b and T4c look items up by
  slug or prefix membership.

## Minor (for the follow-up task)
1. **DP12's loop** doesn't check that the two importance pairs give different `rank` orders. Add an
   `assert_ne!` on the two `ranked` vectors.
2. **No prefix holds kept items both before and after the removed one.** A regression guard is
   needed, for example a 0.5h item due 22:00, then C, then K overflowing, giving `[a, K]`.
3. **Stale or missing doc comments** at `ranking.rs:114` and `:127`. `WontFit` still says "(filled in
   by T3b)". Add the plan's field comments and say on `DeadlinePlan.wont_fit` that it is ordered by
   (deadline, scored index).

## Controller ruling (open question from T3a m-3 and T3b)
A `progress` over 100 gives negative `work_minutes`, which can hide an overflow. **Controller ruling,
2026-10-24:** `work_minutes` clamps at 0. This affects only the deadline walk; `pressure` and
`remaining_hours` are unchanged (the frozen golden arithmetic). It goes into the follow-up task,
test first, and is reported to Quinn as reversible.
