# O1 plan review, round 2

Reviewer (spec/plan mode), 2026-10-23. Filed by the controller. Plan:
`docs/plans/2026-10-23-o1-turned-in-work-plan.md`, checked against `origin/main` at 9777ca6.

**Verdict: PASS.** All of round 1's findings 1-7 are answered; four minors are folded in as T2a/T2b run.

## Round-1 findings

- **F1, rejected card: resolved.** T3's `apply_done` keeps all four of `propose_done`'s rules, so a rejected
  card in `archive/` still protects its task (T2a tests 6a and 6b, T2b test 18, T10 step 5). The planner's
  dispute holds: `apply_amendment`'s already-applied check (`approvals.rs:799-808`) makes a later approval of a
  pending card a clean retry, so round 1's "fails validation" claim was wrong. Leaving a pending card and skipping
  the task is sound: `approvals.rs` has no withdrawal for amend cards, and withdrawing from `completion.rs` would
  add a second writer of `approvals/`.
- **F2: resolved.** Red is "compiles, fails at run time"; T2a/T2b add `todo!()` stubs.
- **F3: resolved.** T2 split into T2a, T2b (waits for S-C; not dispatched if it fails) and T2c.
- **F4: resolved by Quinn's Q-A ruling and the controller's no-score rule.** `grades.rs:188-213`: a `graded`
  row has no score only for Percent, Tabular or Text columns, no `displayGrade`, or a Score row with no
  `displayGrade.score`; each can hold a zero or a fail, so it stays live. Done = `needs-grading`, `exempt`, or
  `graded` with a score above 0 (T2b test 15).
- **F5: resolved.** `WriteOpts { judged: false, evidence: Some(..) }` stated; test 6c checks the record shape.
- **F6: resolved.** `imported-done` counts under the existing `archived` key (T4, test 12a); ingest adds a
  second first-run line (test 16a).
- **F7: resolved, and corrected.** `:2907` is in `an_unknown_privacy_version_names_only_the_failed_check`;
  `:2938` has no `grades` and is unaffected.
- **Minors 8-12: folded in.**

## Minor

1. **Stubs could warn in a non-test build.** A private `todo!()` stub only tests call warns "never used" in
   `cargo build`. `ingest`, `completion` and `surface` are `pub mod` (`engine/src/lib.rs:44,63,88`), so declare
   each stub `pub`; the plan should say so.
2. **Spec §6.2's done table still lists plain `graded` as done** (lines 720-723); make the cell "`graded` with a
   score above 0".
3. **A rejected mail card blocks the gradebook's answer for good** (T3, fourth rule). Fails safe and fits "who
   started a change decides"; T8's `engine-commands.md` entry should say so.
4. **Turned-in Percent, Text and Tabular work stays live** under the no-score rule. Not a regression, but part
   of the goal stays unmet; T10 step 2 should count joined items that stay live as `graded` with no score.

## Note

Q-D lands the `CLAUDE.md` slot line with no extra ask, reading Quinn's Q11 ruling as that ask.

Checked again with no change: nothing on the contract list; frozen and surface references untouched; no
`main.rs` or `commands.rs` edit; Undo through `set_fields` and the engine's `write`; fixed order (figures by
uid, feed events in feed order, grade notes in path order); P-1 holds on main; no person or account named; no
new data class leaves the device.

## Controller addendum

Minor 4 combines with the open Blackboard score-capture bug (`docs/notes/2026-10-16-grades-score-not-captured.md`):
if `displayGrade.score` is never captured, every `graded` row has no score and stays live, so only
`needs-grading` and `exempt` would ever mark work done. The score-capture fix is folded into O1 before T2b.
