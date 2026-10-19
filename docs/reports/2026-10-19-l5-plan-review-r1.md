# L5 plan review, round 1 (2026-10-19)

Plan: `docs/plans/2026-10-19-l5-syllabus-class-times-plan.md` (draft). Reviewer, plan mode. Filed by the
controller from the reviewer's returned text (the reviewer cannot write files).

**Verdict: FAIL.** 0 critical, 6 important, 4 minor. All are fixable in the plan; no spec change is needed.

## What holds

- Every spec §13 row is carried, with its id.
- The four new tasks are justified:
  - T8b creates `syllabi/` at vault creation, a real spec gap.
  - T6a widens the two `eval.sql` checks, confirmed at `20260911000700_eval.sql:9,31`. The five checks
    in `judgment_service.sql` are at :39, 70, 103, 113 and 155.
  - T4d' is the spec's convergence order.
  - T10c takes the window wipe because `app/src/main.rs` is L6's.
  - T3 lifts T1's interim `#[ignore]`.
- Q-B's premise is correct: `engine/src/sync.rs:2030-2041` withholds `AMENDABLE_FOLDERS` ×
  `AMENDABLE_FIELDS`, and `courses` is among them (`approvals.rs:55`).
- `approvals.rs` and `account.rs` tasks go to `contract-engineer` and `contract-reviewer`.
- The order against #19, L6 and L7 follows handoff §5.
- The CLAUDE.md invariants in §1 each map to a test.

## Important

1. **The early staging push can block L1 indefinitely.** Plan :235-241 (Q-I), :349 (T6a), :358 (P8).
   - M7 (first-day plan :217-219, :252-259) says a lane that pushes from its branch holds every lane
     behind it until it merges. The lane may push only once L3 and L8 are cleared, and L8 clears only
     after Quinn's S6 go.
   - PR-1 cannot merge before T14a, which waits on T15 and possibly T16/T17, which wait on L7. So the
     plan's "which follows at once" is false.
   - Scenario: T7b needs the route live, so the controller pushes from the L5 branch. L7 slips, and L1's
     C1 waits for weeks.
   - Fix: push only when L3 and L8 are cleared. Either run T7b against a local function, or push only at
     P8. Pin T16/T17 to one PR.
2. **The version bump is incomplete.** Plan :764-769 (P1).
   - L4's bump (L4 plan :204-210) moves six things in one commit: the page's Effective, version and
     Last-changed lines, `PRIVACY_VERSION`, `PRIVACY_BUMP_1` (`app/tests/grades.rs`) and the panel's
     "what changed" key in `console.js`.
   - P1 names only `account.rs`, `site/privacy.html` and "the tests that pin them". It never mentions
     `console.js`, so P1 has no console-ui queue slot.
   - Moving `PRIVACY_VERSION` in a lane PR also needs Quinn's ruling (L4's Q-AGENT, L4 plan :396-401) to
     cover a second bump.
   - Fix: list every token, give P1 a place in L5's console.js slot, and add an L5 line to Q-A asking
     Quinn to extend the ruling.
3. **Nothing stops a scheduled `judge` from sending syllabi before re-acceptance.** Plan :200 (Q-A), :701
   (T8a).
   - The consent check sits only on T8a's add path. T5's pass inside every scheduled `judge` reads any
     `state/syllabi/` file, and T8b/T9 can create one at Finish.
   - Fix: name the owning task, say how the scheduled run skips the pass, and add a test.
4. **P2's test can pass while the conflict card cannot be applied.** Plan :581-585 (P2).
   - P2 asserts only that the foreign value is withheld. For a mapping field, sync's card carries a YAML
     mapping `from`/`to` (`sync.rs:2092-2098`). T4e's flow-mapping validation and `apply_amendment`'s
     stale check must accept exactly that card, or the value is stranded (`sync_contract.rs:1134-1137`).
   - Fix: P2 also approves the card and asserts the note holds the foreign value as a single-line flow
     mapping. Add a case with a pending syllabus grading amend card on the same field.
5. **The Q14 rule is missing from S1, S3 and the staging route.** Plan :149-154, :416-426, :670-678.
   - S1 downloads a real syllabus and S3 reads real documents, with no rule on where the bytes may sit or
     that they are deleted.
   - T7b sends real seeds through staging's `/judge-syllabus`, but nothing in T6b says the route keeps no
     excerpt or output text. Function logs or result rows would put real syllabus text in a third-party
     database, which Q-E rejected.
   - T7b's "Files: none in repo" hides the env-variable seed reader, which is code.
   - Fix: add the §2 rule and a deletion step to the S1 and S3 briefs. Add a T6b test that nothing is
     kept. Give the reader to T7a.
6. **T4a's brief does not say how a hand-set field on a syllabus series lands.** Plan :333, :517-530.
   - Spec T4t: "a hand-set field follows the change path's existing rule".
   - Fix: cite that rule in T4a's brief, and have T4t pin that case.

## Minor

7. **Plan :512-513 (T4t):** the `sync::is_note_path` case is written by test-writer, but the sync tests
   are on the contract list. Move it into P2's file under contract-engineer, or record why it is exempt.
8. **Plan :705-709 (T8b):** state that `ids::NOTE_FOLDERS` (`ids.rs`, contract list) stays untouched.
   Decide whether `backup.rs`'s `BACKUP_FOLDERS` gains `syllabi`; as written, a restore drops every
   syllabus copy.
9. **Plan :470-472 (T1):** "PDF built in memory" needs a writer, and T3 owns `Cargo.toml`. Specify
   hand-built minimal PDF bytes, or a committed binary with its `-text` line.
10. **Plan :409-414 (T18), :826 (§8):** the spec already has T10c (spec :1377). Only the wipe is new (the
    spec put it in T10b). Correct §8's "spec gaps" wording.

Also: the plan's line 9 names a different report path; make the two agree.

Re-review once findings 1-5 are answered in §10.
