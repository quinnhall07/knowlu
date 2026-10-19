# L5 plan review, round 2 (2026-10-19)

Plan: `docs/plans/2026-10-19-l5-syllabus-class-times-plan.md` (DRAFT r2). Reviewer, plan mode. Filed by
the controller from the reviewer's returned text.

**Verdict: FAIL.** 0 critical, 2 important, 6 minor. Round 1's ten findings are each closed as §10 says.
The two important findings are new. Both are fixable in the plan without a spec change.

## What holds

- **T7b keeps Q14.**
  - Real seeds are read only through T7a's `--local-real` reader, which refuses inside a work tree,
    OneDrive or `%LOCALAPPDATA%\knowlu`.
  - The run is in-process. It writes no staging route and no `eval_cases`, `eval_runs` or `judgments`
    row.
  - Output is counts only, and `git status` is checked in every worktree.
  - OpenRouter's adapter refuses any model row without the zero-retention pin
    (`judge_openrouter.ts:21-30`).
- **Q-K is flagged as Quinn's, and it fails closed.**
- **P2 matches `sync.rs:2092-2110`'s card shape.**
- **`ids::NOTE_FOLDERS` is untouched.**
- **`BACKUP_FOLDERS` gaining `syllabi` needs no page change.** It is the vault mirror, spec :854; it is not
  a new data class.
- **P1's token list matches L4's §4.2**, with `PRIVACY_BUMP_1` correctly left alone.

## Important

1. **P1 cannot keep L4's page pins green as written.** Plan :1022-1030, :1035.
   - L4's T9 pins "Reading a kind of material this page says Knowlu does not read, such as syllabi, is
     never a smaller change" in `engine/tests/site.rs`.
   - Once P1 removes the parenthesis, the page contradicts itself.
   - P1 also says "the four files", and `site.rs` is not one of them.
   - Fix: either have L4 word the sentence without the syllabus example, or add the sentence and its
     `site.rs` pin to P1 (five files) and move P1's new pin into `site.rs`.
2. **T6b's nothing-kept rule needs `judge_pipeline.ts`, which no task lists.** Plan :822-838, §4.2
   :364-365.
   - `fieldsOf` (`judge_pipeline.ts:81-92`) stringifies every verdict field into `judgments.fields`, so
     syllabus text (office hours, quotes, meetings) would land in a hosted row.
   - L3 and L7 both edit that file.
   - Fix: add `judge_pipeline.ts` (a syllabus arm in `fieldsOf`/`featureMap`) and `judge_pipeline_test.ts`
     to T6b. Add a §4.2 row: "L3, L7 ahead; rebase". Model the marker test on
     `a_body_token_reaches_no_judgment_row`.

## Minor

3. **P8 does not follow M1-M8 exactly** (:887-905).
   - It says "four steps"; M7 now lists five.
   - Add a pre-merge dry run, which lists nothing once the lane has pushed.
   - Require CI green on the rebased head before the push.
   - "Lanes ahead" means every lane ahead, including email and any that overtook L5 by a recorded slip.
4. **P3 needs three clarifications** (:956-968).
   - Read the gate after L4's refresh in the same slot.
   - Add the `syllabi (skipped: …)` row only when judge's plan is `Run`.
   - Under Q-GATE's fail-closed alternative, `Unknown` acts as `Older`.
5. **T5t is missing a case P3 relies on.** Add: `--syllabus` with no `state/syllabi/` behaves exactly like
   a plain `judge`.
6. **Q-K changes more of the signed spec than "one sentence".** It also touches §4.3 :444, the §13 T5t
   and T13 rows, :1748, and decision row 5 (:1150). List them all for Quinn.
7. **The Q14 rule in §2 is narrower than the ruling.**
   - Q14 says "consented and not public" (progress.md row 122), so record consent per seed.
   - OpenRouter also sees each seed.
   - T7a's marker test needs the provider-error path.
8. **The P1 pin location** is the same issue as finding 1.
