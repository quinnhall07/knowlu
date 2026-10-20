# L7 plan review, round 3b, delta check (2026-10-19)

Plan: `docs/plans/2026-10-19-l7-named-issues-plan.md`, §11 "Round 3 delta". Reviewer, plan mode, with the code
checked against the email branch (`cloud/supabase/functions/gmail-read/`). Filed by the controller from the
reviewer's returned text. C1 and C2 are the controller's and were not re-opened.

**Verdict: PASS.** 0 critical, 0 important, 2 minor. All 7 round-3 findings are closed at the cited lines.

## Round-3 findings, closed

- **I1.**
  - §3.3 :308-312 takes the answer from the rule, never from the write's effect.
  - Test 4b (:828-832) starts from a `quiet` row, an empty list and a 30-day-old grant. It expects
    `went_quiet: true` with no change to the row.
- **I2.** T0's first step (:612-617) has the controller commit the marker alone. T7 5a (:1069-1071) checks it
  is on `main`.
- **M3.** Milliseconds are compared against `7 * 86_400_000` with no rounding.
  - Test 3b at 7 d − 1 ms catches `Math.round` and `Math.ceil`.
  - Test 4 at exactly 7 d catches `>` written for `>=`.
- **M4, M5, M6, M7:** closed.
  - The write goes straight after `deps.api.list` (handler.ts:351), before the loop (:354), under the outer
    catch (:423-431).
  - The Testing-mode wording, Q4's `false` half and the citations are fixed.

## The spec marker

- `docs/specs/2026-09-09-knowlu-cloud-design.md:165` carries the marker, with the plan path. Its hunk
  (`@@ -162,7 +162,7 @@`) is alone and can be staged on its own.
- The file's other uncommitted hunks are:
  - the 2026-10-16 price markers at D2 (:14), R3 (:342) and the Pilot entry (:607);
  - the appended Amendments 2026-10-16, 2026-10-16b and 2026-10-19.
- None of them touches §5.3.

## Minor

1. **Test 5b's "the pipeline was never called" can be misread.** `deps.pipeline()` is built at
   handler.ts:340, before the list.
   - Fix: assert zero `deps.api.message` calls (the loop's first act, :364), and no `enqueue` or `markSeen`.
2. **T0's `git add -p` is interactive** (:615-616).
   - Fix: write a patch holding only the `@@ -162` hunk and stage it with `git apply --cached`. Confirm with
     `git diff --cached --stat`.
