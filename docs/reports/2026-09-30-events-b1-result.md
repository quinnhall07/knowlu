# B1 (events plan T9): result — DO NOT SHIP, the idea is closed

**Date:** 2026-09-30. **Branch:** `j-events` at `fcad6d4` (code under test). **Harness:**
`scripts/experiments/e1-decomposition/` (unchanged). **Authority:** Q5 (c) for the run; PQ4 for the base
(run on `j-events` as it stands).

History: a first run on 2026-09-30 failed every call with HTTP 401 (stale keys) and gave no verdict; the
controller re-ran B1 the same day with a working key from Credential Manager `knowlu/dev/openrouter`.
This report records the re-run.

## Verdict

**DO NOT SHIP `event-4`.** Per the signed rule the idea is closed and the number is recorded.

| Arm | weighted_exact | 95% CI | Refused | Tokens in / out |
|---|---|---|---|---|
| event-3 (frozen control) | 0.962 | [0.910, 1.000] | 0 | 6104 / 1164 |
| event-4 (decomposed drop rules) | 0.974 | [0.936, 1.000] | 0 | 8600 / 1792 |

- Paired (event-4 minus event-3): **0.013, 95% CI [0.000, 0.038]**. The interval touches 0, so rule (1)
  fails.
- Disagreements: **1 win, 0 losses, 25 ties.** Exact two-sided sign test **p = 1.0000**, so rule (2)
  fails. With 26 cases the test needs at least 6 wins with 0 losses.
- The one win is `seed-event-0022` (label `drop`): event-3 answered `opportunity`, event-4 answered
  `drop`. Both arms missed `seed-event-0024` (`unsure`, both said `drop`) and `seed-event-0025`
  (`unsure`, both said `obligation`) identically.
- event-4's rules fired `audience` 6 times and `standing` 5 times.
- Scored the way the device records answers (refused to `unsure`), the figures are identical and the
  verdict is the same.
- Spend: about **$0.0019** at the row's rates (52 sequential calls).

**Re-scoring under `main`'s 2026-09-22 cost matrix cannot change this verdict.** The matrix changes how
much a disagreement costs, not how many there are. One disagreement cannot pass the sign test (p = 1.0
for 1-0, and the test needs 6-0), so no matrix makes event-4 win.

## 1. Pre-run check: frozen arm and pinned row against `main` (passed)

`origin/main` is `199cd1f`. `j-events`'s merge-base with it is `7c127e2`.

| What | `main` | `j-events` (what the harness uses) | Same? |
|---|---|---|---|
| Event-row migrations | `20260911000100`, `20260916000100`, `20260922120100` | the same three, byte-identical (absent from `git diff origin/main HEAD`), plus `20260922120300_event_decomposed.sql` | yes for the shared three |
| Model pin | `ibm-granite/granite-4.2-8b`, openrouter, route `{"order":["CoreWeave"],"allow_fallbacks":false,"zdr":true,"require_parameters":true}`, `bf16 (CoreWeave)`, max_tokens 256, sampling `{"temperature":0,"reasoning":{"enabled":false}}`, $0.10/$0.15 per M | identical (`event_decomposed` moves only `prompt_version`/`grammar_version`) | yes |
| Prompt / grammar | `event-3` / `event-2` | control arm frozen as `event-3`; treatment `event-4` / `event-3` | as designed |
| Event-3 prompt hash | `promptHash("event")` over `main`'s `judge_prompts.ts`: `b6324351…5e92` | `event3Hash()` over `event3_frozen.ts`, and `EVENT3_PROMPT_HASH`: `b6324351…5e92` | yes |
| Event-3 validator | `validate()`'s event branch | `validateEvent3` | same logic |
| User message, client, seed | `buildPrompt("event")`, `judge_openrouter.ts`, `cloud/eval/seed/events.jsonl` | unchanged from `main` | yes |

The frozen arm and the pinned row match `main`'s live event row, so the stop condition did not fire.

## 2. Gates

- `deno test --allow-read --allow-run=cmdkey.exe,powershell.exe` in the harness folder: **36 passed,
  0 failed.** Without `--allow-run` one test fails with `NotCapable` (it creates and deletes a
  throwaway `knowlu/test/e1-*` credential through `cmdkey`). That is a permission flag, not a defect,
  and no test credential was left behind.
- `run.ts --dry-run`: 26 cases (`obligation` 6, `opportunity` 7, `drop` 11, `unsure` 2). Arm hashes:
  event-3 `b6324351…5e92`, event-4 `cea3a45f…51d1`. Estimate: 52 calls, about $0.0029.

## 3. The run

`run.ts` resolved the key from `credential-manager:knowlu/dev/openrouter` and sent 52 sequential
requests; none was refused. The harness's gate figure is `score.ts`'s unmodified `weighted_exact` on
the branch (PQ4: run the branch as it stands). That branch matrix predates `main`'s 2026-09-22 ruling
(it names four event cells; every `unsure` cell costs the default 1, and the harness's `COST_NOTE` still
calls it "UNRULED"). The gate line was:

> B1 (the gate, on score.ts's figure): DO NOT SHIP — record the number and close the idea: difference
> 0.013, 95% CI [0.000, 0.038] (includes or sits below 0), sign test 1-0 p=1.0000 (not < 0.05 in
> event-4's favour)

Per-case table (only the rows where an answer differs from the label or between arms):

| Case | Label | event-3 | event-4 |
|---|---|---|---|
| `seed-event-0022` | drop | opportunity | drop |
| `seed-event-0024` | unsure | drop | drop |
| `seed-event-0025` | unsure | obligation | obligation |

The other 23 cases were answered correctly by both arms.

## Recommendation

**Close `j-events`.** Its branch is kept and no PR is opened. `event-4` does not ship, and
`event_decomposed` (migration `20260922120300`) never reaches `main`. This seed can rule the
decomposition out, and it has: a gain of one case in 26 is inside noise. Nothing here argues for
re-running B1 or for re-scoring under `main`'s matrix.
