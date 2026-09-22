# Stream J — rollout note

**Date:** 2026-09-22. **Branch:** `j-judgment-quality`. **Plan:**
`docs/plans/2026-09-22-judgment-quality-plan.md`. Every ordering constraint below came out of the
whole-branch review; the first two are hard.

## What ships

| task | what it does | where |
|---|---|---|
| T1 | a fourth event verdict, `unsure`, recorded so an event is never re-asked forever; only repeatable refusals become `unsure`, a provider outage is retried | engine events/eventledger/cloudmodel, judge event region, migration `20260922120100` (event-3) |
| T2 | 26 invented event cases, the first thing the eval gate has ever scored | `cloud/eval/seed/events.jsonl` |
| T4 | the model extracts the deadline phrase; code resolves it; anything ambiguous ("next week", "next Friday") is null | `_shared/judge_due.ts`, migration `20260922120200` (email-3) |
| T6 | the calibration harness — parked until real corrections exist | `cloud/eval/calibration*` |
| T8 | zyBooks at 100% of points and VHL at 100 propose `status: done` | `engine/src/completion.rs`, coursework |
| T9 | an LMS "Submission received" email proposes `status: done`; grades never do | `_shared/lms_receipts.ts`, `gmail-read`, `enrich.rs`, email-3 migration |
| E2 | the review-triage replay, prepared; its paid Jev run waits on a key | `scripts/experiments/e2-review-replay/` |

**Not on this branch:** T3 (the event-rule decomposition, event-4) waits on branch `j-events` for
E1's measured result — it merges only if it beats event-3 by at least six net cases on the seed.

## Ordering constraints

1. **Apply migration `20260922120200` before deploying `gmail-read`.** Hard. The new `gmail-read`
   enqueues `completion` rows, and the queue's tier check rejects them until that migration widens
   it — every Gmail read would fail (23514), every slot.
2. **Apply `20260922120100` and `20260922120200`, then redeploy `judge-task`, `judge-event`,
   `judge-email` and `gmail-read` together, straight away.** They bundle the shared `judge_*` code.
   A gap between migration and deploy labels judgments `event-3` / `email-3` against the old prompt
   text (`prompt_hash` still tells them apart); a partial deploy leaves functions on mixed prompts.
3. **Migration versions against C3.** C3's branch holds `20260922000100` / `20260922000200`, which
   sort before this branch's `20260922120100` / `120200`. There is no name clash, but if this
   branch's migrations reach a remote first, C3's push needs `supabase db push --include-all`.
   Applying C3's first avoids that.
4. **Engine and server may update in either order.** Both new capabilities are negotiated:
   - An engine that does not send `accepts: ["unsure"]` never receives `unsure`; it gets the pre-T1
     reply shape (no verdict, `below floor`) and behaves exactly as it does today.
   - An engine that does not send `accepts: ["completion"]` receives `information` for a completion
     email; the receipt is acknowledged and not proposed on that device.
   - A new engine against an old server is safe: the old functions ignore `accepts`.

## Expect on the merge PR

- **The eval gate runs for real, and may go red.** The branch touches the judge prompts and adds
  migrations mentioning `models`, so `eval-gate` fires; T2's seed makes it a paid, scored run against
  thresholds that have never been measured (`cloud/eval/thresholds.json` says so). Treat that run as
  the first measurement, then set `event.weighted_exact_min` from it.
- **`.github/workflows/ci.yml:96` is stale** — it says the seed "stays empty FOREVER (R-C2-E12)". It
  is stream C0's file and was deliberately not edited here.
- **On Quinn's first vault run after deploy:** seven zyBooks `status: done` proposals (HW 01, HW 02,
  HW 05, Labs 01-03, Project 1), within the 15-a-day cap.

## Verification at `6123e71`

`cargo test --workspace`: 1,287 passed, 0 failed, 4 ignored (the four by design); the one warning is
the accepted `.rsrc` linker line. Deno: check 125 files clean, lint clean, test 578 passed, 0 failed.
Every task passed its own review; the whole-branch review's nine findings were fixed and re-reviewed.
