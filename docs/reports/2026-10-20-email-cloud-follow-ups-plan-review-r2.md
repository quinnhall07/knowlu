# Email forwarding cloud follow-ups plan: plan review r2 (delta)

Plan: `docs/plans/2026-10-20-email-cloud-follow-ups-plan.md` (DRAFT r2). Reviewer, plan mode, delta against r1. Code
read on `origin/main`. Filed by the controller from the reviewer's returned text.

**Verdict: PASS (0 critical, 0 important, 2 minor).** All ten r1 findings are closed. The revision adds no critical or
important problem.

## Confirmed

- **I1 closed** (T2 :365-373, test 5 :382-383): the cited pattern matches `charge_allowance` and `count_mail_arrival`
  (`20261002000200_mail.sql`): invoker, `public, extensions`, `returning true into`, revoke/grant.
- **I2 closed** (T5 :535-545): intake deletes the object last, so test 1 (delete fails, 500, object kept) and test 3
  (second mailbox's `arrive` throws, 500) both reach `batchSend` twice and give 2n on today's code; both red as written.
- **M1** (T3 :423-436; Q1 :195-197; Q3 :218-222; §9 :728-730), **M2** (T3 :407-417; `store.wait` at
  `mail-process/index.ts:215-218`), **M3** (:177-179, :384-386), **M4**, **M5** (:111-117), **M6** (:118-123, T6
  :567-570; `mail_claim` is `openrouter` at `mail.sql:343-353`; later migrations set only `max_tokens`), **M7** (T2
  :334-340, T4 :484-489, T9 :644), **M8** (§9 :718-727): all closed.
- **Dropped grace check:** no stale-`rotated_from` path keeps the Gmail read off after the grace. Only Rotate writes
  `retired_until` (`mail/store.ts:178`), and under T4 the same PATCH writes `rotated_from`; `endRotateGrace` nulls
  `retired_until` (`mail-inbound/store.ts:205-208`); an old-code Rotate leaves the new column null (today's behaviour).
- **Q1-Q5** are accurate and each has a recommendation; the money flags hold.

## Critical

None.

## Important

None.

## Minor

1. **A missed end-of-run mark can lose mail when that run was the item's last hold before the sweep** (T3 :423-429).
   `run` has no `finally`: if `deps.store.due` throws in a later round (`mail-process/handler.ts:359`), `handle` answers
   500 (`:405-408`) after this run's holds were written, and the mark never runs. Usually harmless (holds come due
   hourly and the next run marks), but an item capped on day 0 and day 1 is due at day-2 00:00, the sweep's bound; if
   the account lapsed in the hour before, the run holds it, round 2's `due` throws, the sweep expires it, and a renewal on
   day 5 cannot reach it (reconcile reaches 3 days), against §4.6 step 3. The same loss happens with no failure if the
   sweep wins the M5 race. *Fix:* run the mark from a `finally` around the loop; add T3 test
   `a_run_that_fails_after_a_hold_still_marks`; name the sweep-before-first-hold residual in §9 and Q3.
2. **A pre-existing path keeps a rotated Gmail's OAuth read off past the grace** (outside this delta; name it in §9).
   Rotate from `active` leaves `last_canary_ok_at` set (`mail/store.ts:171-183`); a lapse mark turns the row `lapsed`;
   renewal's `unlapseAccount` writes `quiet` (`_shared/mail_unlapse.ts:19`); `quiet` is on D22's allow-list; intake's
   `confirmed` is guarded on `status=eq.pending` (`mail-inbound/store.ts:211`), so the new address's confirmation never
   lands; once `retired_until` passes the old forward's mail is deleted as unbound while the OAuth read stays off.
   Today intake's mark already does this; Q3's mark makes it reachable with no arrival during the lapse. *Fix:* a §9
   bullet for a later lane (e.g. unlapse writes `pending` while a Rotate grace is open). Not verified: whether
   `mail-canary` later moves such a row to `blocked`.
