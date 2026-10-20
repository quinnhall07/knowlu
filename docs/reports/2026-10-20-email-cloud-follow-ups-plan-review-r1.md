# Email forwarding cloud follow-ups plan: plan review r1

Plan: `docs/plans/2026-10-20-email-cloud-follow-ups-plan.md` (draft). Reviewer, plan mode, code read on `ba7eb0b` and
L7's `81e4b72`/`aed129a`. Filed by the controller from the reviewer's returned text.

**Verdict: FAIL (0 critical, 2 important, 8 minor).** The design is sound and both important findings take a few lines
each to fix, so a delta re-review should be enough.

## Confirmed

- Every code citation checked says what the plan claims: `mail-process/handler.ts:256-308, 341-356, 369-373`;
  `model.ts:125, 137-150, 181-239`; `sweep.ts:45-74`; `mail-inbound/handler.ts:262-266, 361-376, 399, 437-484,
  506-512`; `mail-inbound/store.ts:95-122, 186-229`; `mail/handler.ts:363-380`; `mail/store.ts:171-183`; L7's
  `gmail-read/handler.ts:203, 210-215, 330-338` and `index.ts:199-208`; `mail-pull/handler.ts:10-13`;
  `account/mail_test.ts:167`; `ci.yml:167, 242-248`; `migrations_test.ts:333, 1003-1029, 1479, 1861-1885`;
  `mail.sql:57`.
- **Money:** `mail-process` is the only model path with no entitlement check. `mail-resolve` and `mail-pull` gate on a
  session plus entitlement (`mail-resolve/handler.ts:4, 82`), and so does `mail-canary` (`handler.ts:207-211`). A gate
  before the S3 get closes the hole.
- **Privacy:** the hold does not extend retention; the object's life is set by the bucket rule and the sweep
  (`sweep.ts:63-74`), not `next_attempt_at`, within the spec's "up to about two days" (spec :973, :982).
- **Double Rotate inside a grace:** `rotated_from` becomes `pending` and `store.rotate` retires the first rotate's token,
  so the OAuth read coming back on is correct.
- **Concurrent receipts:** `count_batch_receipt`'s guarded update re-checks its `where` after the row lock (READ
  COMMITTED), so a second concurrent delivery counts nothing.
- **Stamp:** L3 holds `20261020000200` and L8 `20261020000300`, so the floor is right; §5 matches L3's ledger.
- **Questions:** Q1-Q5 each carry a recommendation; the money ones (Q1, Q2, Q5) are flagged.

## Critical

None.

## Important

1. **T2's "existing pattern" for the two new functions does not exist.** `count_mail_arrival` is `security invoker`
   with `set search_path = public, extensions` (`20261002000200_mail.sql:539-558`), as is every function in that file
   (:441, :466, :506, :542, :569); the plan says `security definer` with `public, pg_temp`. An implementer copying the
   cited lines fails the plan's test 5; one following the text writes the first definer functions in the mail
   migrations, which bypass RLS for a `service_role` caller that does not need it. *Fix:* invoker with
   `public, extensions`, keep the revoke/grant lines, and test 5 asserts that.
2. **T5 test 1 can pass on today's code.** After a full success intake deletes the inbound object
   (`mail-inbound/handler.ts:482`), so a faithful fake S3 makes the second `handle` get a 404 and answer 500
   (`:479, 509-511`) and nothing is counted twice. *Fix:* define test 1 as a redelivery with the object still present
   (its delete failed or was lost), say so in the test, and record the red count.

## Minor

- **M1.** Entitlement is cached for the run (up to 50 s, `RUN_BUDGET_MS`): an item after a mid-run lapse can still be
  charged, and a mark from a stale `false` can re-lapse a just-renewed mailbox; name both as accepted residuals. T3 also
  calls `markLapsed` for every held item every hour; mark once per account per run, from the smallest `arrivedAt`.
- **M2.** T3's new `hold` repeats `store.wait` (`mail-process/index.ts:215`), which T6 uses for the same hold; use
  `wait` with `item.attempts` and drop `hold`.
- **M3.** The `models` test must ignore case: CI greps with `grep -qiE` (`ci.yml:247`). Use `/\bmodels\b/i`.
- **M4.** §9's residual that canaries can go to an unentitled account is wrong (`mail-canary/handler.ts:16, 207-211`
  checks first). Drop it.
- **M5.** §1.5 overstates the sweep's lead: for a day-0 arrival, attempt 3 is due at day-2 00:00, the instant
  `expiredBy` passes. A race, not "long before"; fix the wording.
- **M6.** Only OpenRouter throws `ProviderOutOfCredit` (L7 `judge_openrouter.ts:96`); state which route `mail_claim`'s
  pin uses, and if it is Anthropic direct say T6 has no effect there.
- **M7.** The `…_in_grace` check makes deploy order load-bearing (`mail-inbound` before `mail`), while D22's
  `retiredUntil > nowMs` already ignores a stale `rotated_from`. Drop the check, or name the benign rollback case.
- **M8.** §9's redelivery list misses: count, then the delete fails, then `mail-pull` closes the batch; the redelivery
  binds to no open batch (`store.ts:120`) and is counted `unbound` (`handler.ts:361-366`), feeding `UNBOUND_STREAK`.

Not verified: `app/src/mail/runner.rs:796-818`, spec :172 and :177, and the `approvals.rs`/`cli.rs` lines in §9.
