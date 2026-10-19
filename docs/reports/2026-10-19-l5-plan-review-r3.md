# L5 plan review, round 3 (2026-10-19)

Plan: `docs/plans/2026-10-19-l5-syllabus-class-times-plan.md` (DRAFT r3). Reviewer, plan mode, with the code
checked against the email branch. Filed by the controller from the reviewer's returned text. Earlier rounds:
`2026-10-19-l5-plan-review-r1.md` and `-r2.md`.

**Verdict: PASS.** 0 critical, 0 important, 5 minor. All eight round-2 findings are closed as §10's Round 2
table says. No real syllabus content appears anywhere in the plan: a grep for course codes, names and
addresses found none.

## Round-2 findings, checked against the plan and the code

- **1 and 8 (P1's five files).** P1 items 1-5 name `privacy.html`, `account.rs`, `console.js`, `site.rs` and
  `static_assets.rs`. P0 item 14 records L4's Q3 ruling and the pin's name. L4's pin list pins the Q3 (ii)
  sentence but not the "does not read syllabi" parenthesis, so the change-rule pin is the only existing pin
  P1 edits.
- **2 (`fieldsOf`).**
  - `fieldsOf` (`judge_pipeline.ts:81-91`) is the only caller of `featureMap`.
  - Every row write that carries fields goes through `fields()`: :153 (tier 2) and :309 (tier 3 answered).
    The other five writes put `fields: {}`.
  - So the T6b arm covers every path. Its marker test is modelled on `judge_pipeline_test.ts:122`.
- **3 (P8 against M1-M8 r8).** Clearing matches step 2. The five withdrawal steps match step 6. The pre-push
  and pre-merge dry runs match steps 5 and 7. Step 4 requires CI green on the rebased head. P8 asking Quinn
  where M7 would let the controller withdraw alone is stricter than M7, not a conflict.
- **4, 5, 6.**
  - P3's three rules and six cases are in.
  - T5t has the no-folder case.
  - Q-K's eight places are at the cited spec lines; minor 1 adds a ninth.
- **7 (Q14 consent).**
  - §2 and Q-E carry the manifest.
  - T7a refuses without one and skips unmarked seeds by count.
  - The marker test covers three paths.
  - §2 names OpenRouter and the pinned upstream as the only third parties.

## The planner's decisions

- **Skipping `featureMap` is enough.** `features()` is otherwise reached only through tier 2's read-only
  `deps.rules.lookup` (`judge_pipeline.ts:143`). For a request item of only `id`, `date` and `parts`, the
  non-task branch (`judge_rules.ts:52-59`) returns `[]`. This holds while the request shape stays closed
  (minor 4).
- **The V2 pin in `static_assets.rs` is sound** (minor 3 for a later bump).
- **T6b waiting for L3 is sound.** P8 needs L3 cleared anyway, so the critical path does not grow (minor 2).

## Minor

1. **Q-K misses a ninth place, and T8a's status line can be untrue.**
   - Spec §10 row 17 (:1162) says "if a slot holds it, the slot reads the course". Under Q-K that is true
     only when the gate is `Accepted`.
   - T8a (:1033-1035) shows "Reading with this run" while a slot is running, without checking the gate.
   - Scenario: the gate reads `Unknown`, the refresh fails, and the student sees "Reading with this run".
     Then `judge` runs without `--syllabus` and nothing is read.
   - Fix:
     - Add row 17 to Q-K as place 9.
     - Show the line only under `Accepted`, and the waiting outcome otherwise.
     - Add a T8a test for a slot-held run under `Unknown`.
2. **Stale dependency lines after r3.**
   - :518 should name T6b as waiting on L3.
   - Q-C (:295) lets T7a start before the cut "once T6b's schema is fixed", which can no longer happen.
   - The T6a row (§5 :487) and §4.2's `judge_caps.ts` row (:427) disagree on whether L3 must have merged.
     T6a also edits `judge_pipeline_test.ts:402-404`, which L3 edits.
   - Fix: name T6b at :518; drop or condition Q-C's pre-cut clause; pick one rule for T6a.
3. **The V2 pin may break at the next privacy bump.**
   - If L4's "one key per bump" (L4 plan :371) means `PRIVACY_CHANGES` keeps only the current key, the
     lawyer-read bump removes `V2`.
   - Fix: word the pin against `PRIVACY_VERSION` while it is `V2`, or note that the next bump retires it.
4. **A syllabus row's `confidence` is set outside `fieldsOf`.**
   - `judge_pipeline.ts:152` and `:308` compute `Number(checked.verdict.confidence ?? 0)`, and the column is
     `real not null check (0..1)` (`20260911000100_judgment_service.sql:75`).
   - If the syllabus schema has a per-part `confidence` map, `Number(map)` is NaN and the audit row is lost.
   - Fix:
     - T6b names how the row's `confidence` is set.
     - Test 1 asserts it lies in 0..1.
     - Test 1 asserts `features("syllabus", item)` is `[]` for the request shape.
5. **§10's Round 2 references into §9 are one row low** (a row was inserted at :1297). Correct them, or cite
   by the Source column.
