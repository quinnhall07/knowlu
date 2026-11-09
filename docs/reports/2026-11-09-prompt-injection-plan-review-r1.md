# Prompt-injection suite plan: review r1

**Reviewed:** `docs/plans/2026-11-09-prompt-injection-suite-plan.md` at b7750ac8, checked against `ci.yml`,
`engine/tests/workflows.rs`, `cloud/eval/run_eval.ts` and `score.ts`, `_shared/judge_validate.ts` and
`judge_prompts.ts`, `mail-process/model.ts` and `handler.ts`, and cloud design §5.4.
**Verdict: FAIL.** The plan needs an r2 before T0.

## Critical

- **C1. Per-class CI counts point to the fixture that beat the model.** About 36 attacks are spread over
  5 kinds × 13 classes, so most kind-and-class pairs hold a single fixture, and the fixtures are public.
  A count per kind and class therefore names that fixture. Fix:
  - CI and the committed T7 report print totals only: critical, drift, calls, spend.
  - The per-kind and per-class breakdown appears only under `--per-case`, run locally.
  - T5a asserts that the CI line carries no class tag.

## Important

- **I1. Red tests pushed before the fix disclose it.** T2a commits failing tests, and `cloud` is a public
  job. Rule: no push to origin until T2b is green. Say so in T2a.
- **I2. Honest file names collide with the URL rule.** Some file extensions are also real top-level
  domains (`.zip`, `.py`, `.rs`, `.sh`, `.md`, `.pl`, `.ai`, `.mov`), and the "label.label.tld" rule is
  unclear about two-label names. Fix: put a closed TLD list and a closed file-extension list in §4.3,
  where an extension wins. Add the colliding names to must-not-flag.
- **I3. Template claims are in scope, but nothing decides it.** `mailClaim()` also checks template
  claims (`source: "template"`), whose `target` is the LMS title word for word. Decide whether the
  policy applies to them, and name the receipt and template tests that must stay green unedited.
- **I4. Re-runs stop being comparable (spec §5.4 measure 4).** Validation changes, but no version moves
  with it. Quinn picks one:
  - bump `grammar_version` through a models migration (this also starts eval-gate), or
  - T8 records the policy's deploy date as the boundary.
- **I5. Layer 2's mail kinds are under-specified.** Name every step it calls: `scrubForPrompt`,
  `mailPromptText`, `buildMailResolvePrompt`, and the validation context (date, timeZone, from,
  knownCourses). Guard `mail-resolve/handler.ts` too.
- **I6. Public-repo wording.** Generalise the sentences in §3 "Rendering, today", the §4.1 H4 cell and
  the paragraph after it, §11's CSP list, and optionally the §3 table rows, so that none of them
  describes today's gaps. The wording to use is in the reviewer's report.

## Minor

- **M1. Wasted eval spend.** `mail-process/model.ts` belongs on the `redteam` path only, not on `run`.
- **M2. Make the gate settings explicit.** Write down the non-blocking mechanism and pin it in T5a. The
  weekly workflow runs on GitHub-hosted runners only, triggered by `schedule` and `workflow_dispatch`.
  T6 adds a `workflows.rs` check naming `secrets.OPENROUTER_REDTEAM_API_KEY`.
- **M3. Recompute the cost.** T0 records all five model pins and their prices, then recomputes the
  monthly cost against the $5 cap.
- **M4. Name the placeholder.** It must survive `oneLine` and the event ledger's `why` check.
  `evidence` is no longer verbatim after neutralisation, so either confirm that nothing re-checks it or
  refuse instead.
- **M5. Keep error text out of CI.** The CI path never prints exception or adapter error text.

## Holds

- The three required checks still report on docs-only PRs.
- Layer 1 runs in the `cloud` job; add the new path to the `deno check` glob.
- Fixtures stay out of the seed.
- Key handling is sound.
- Scope is clean: no engine file and no frozen reference is touched.
