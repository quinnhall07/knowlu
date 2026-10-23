# O1 plan review, round 1

Reviewer (spec/plan mode), 2026-10-23. Filed by the controller from the reviewer's report. Plan:
`docs/plans/2026-10-23-o1-turned-in-work-plan.md`, checked against the ruled spec §6 and `origin/main` at 9777ca6.

**Verdict: FAIL.** One revision round; no finding reopens a ruling.

On main, the plan's citations hold to within a few lines. `completion::ACTOR`, `propose_done`,
`propose_vendor_completions`, `is_first_run`, `sync_tasks`, `slot_argv`, `older_than_applied`, `Closed` and
`set_fields_inner` are where the plan says. All three `surface-today-*.json` hold `"closed_this_week": []`, so
the new `undo` field changes none of their bytes. P-1 holds: `grades::apply` reads `courses/`, `grades/` and
`state/grades.json`, and `ingest.rs` writes none of them. A failed step does not stop the chain (`scheduler.rs`
slot loop). The mail arms refuse anything archived that is not `imported-past` (`approvals.rs:212`,
`mailreconcile/rules.rs:301-305`), so `imported-done` notes stay archived.

## Critical

1. **A rejected "Mark done" card stops protecting the task.** T3 keeps only `propose_done`'s first three
   rules and drops rule 4 (`already_proposed`, `completion.rs:20-28`, `:240`). Rejecting a tier-1 card writes
   nothing to the task, so `human_set(id,"status")` stays empty. Scenario: a student rejected "Mark done: Lab 3"
   (zyBooks shows 100% but the lab still needs uploading); after upgrade `apply_vendor_completions` sets
   `status: done` as `agent:knowlu.completion`, overriding the student, against Q3 and judge-once. A pending
   card has the same gap: the task is marked done while the card stays in Decisions with `from: active`, so a
   later approval fails validation. Fix: `apply_done` skips a note with a rejected done card and skips or
   settles a pending one; a T2 test for each.

## Important

2. **T2 cannot meet its own acceptance.** Tests that call `apply_done`, `apply_vendor_completions`,
   `ingest::IMPORTED_DONE` or `Closed.undo` before they exist stop the crate compiling, so nothing fails "for
   the stated reason". Fix: T2 adds minimal stubs (signature plus `todo!()` or a sentinel), or the acceptance
   accepts "does not compile" as the red state.
3. **T2 commits tests whose implementation may never ship.** Tests 14-19 (the ingest gradebook half) depend on
   S-C and Q-A, but T2 depends only on T0; if S-C fails the branch keeps red tests, and test 15 fixes the
   zero-score answer before Quinn answered. Fix: split T2 into T2a (completion, coursework, surface, scheduler,
   `turned_in`) and T2b (tests 14-19, after T1's S-C result and Q-A).
4. **Q-A as framed lets missing work vanish.** `grades.rs:188-213`: a Graded row on a Percent, Tabular or Text
   column, or one with no displayGrade, is `graded` with no score, and the plan's rule says such a row counts.
   Scenario: 0% or "F" entered for work never turned in is archived as `imported-done` on first ingest and never
   shown, though still accepted late. Fix: put the case to Quinn inside Q-A; recommend that a `graded` row with
   no score counts only on a column type that cannot hold a zero, or stays live.
5. **`write`'s guard is not a second fence** (plan §1 "Judge once"). `status` is not in `JUDGED_FIELDS_TASK`
   (`provenance.rs:52-58`), and the guard runs only with `opts.judged` (`write.rs:322`); `apply_done`'s own
   `human_set` check is the only fence. State `WriteOpts { judged: false, evidence: Some(..), .. }` outright
   (`WriteOpts.evidence` exists, `write.rs:272`); `judged: true` would take the provenance path at `write.rs:363`
   and could break test 1's "every other byte unchanged".
6. **The run-record count is unspecified.** `coursework.rs:1935` counts `archived` only from lines starting
   `archived (imported-past)`; T4 doesn't say whether `archived (imported-done)` joins it or gets a new key, and
   ingest has the same gap. A new key changes a contract's shape; leaving it out makes a first day look empty.
   Fix: count both spellings under the existing `archived` key, no new key; a T2 assertion on step counts.
7. **The conflict map undercounts the order-pinning scheduler tests.** Besides `:1508`,
   `an_accepted_privacy_version_leaves_the_slot_as_it_was` asserts the old order twice
   (`app/tests/scheduler.rs:2884`, `:2907`). Name all three in T2 item 24 and the conflict map.

## Minor

8. T2 is far over the ~80-line edit size (about 25 tests across 5 files plus a rewrite of
   `completion.rs:453-784`); split per finding 3 and consider splitting the completion test rewrite out.
9. T7a doesn't depend on T3: `completion::ACTOR` already exists (`completion.rs:43`); it can run beside T3.
10. Test 22 drives only the approvals arm; add a case for `mailreconcile::rules::amendable` (a due move on an
    `imported-done` note files no card), or note a reviewer checks it by reading.
11. Upgrade flood: on existing vaults the first slot after merge marks every submitted Blackboard or 100% zyBooks
    task done at once, each with an Undo row on "Closed this week". Within Q3, but T10 should observe it and
    HANDOFF should say it is expected.
12. Ingest's mark-done pass should state its iteration order (feed or path order) in the T5 bullet, not only §1.

## Fidelity

Q3: first-sight archiving, apply after with Undo, and the hand-set exemption are covered; finding 1 is a gap.
Q11: slot order, the `CLAUDE.md` line through T8 with Q-D, and P-1 are covered. Q-B (run log only) and Q-C
(reorder ships alone) are reflected. Nothing touches the contract list or the frozen fixtures, nothing names a
person, school account or credential, and there is no new data class.

**To pass:** findings 1-7 answered in plan §9, then round 2.
