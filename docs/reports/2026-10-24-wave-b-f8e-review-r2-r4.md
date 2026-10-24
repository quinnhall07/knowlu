# Wave B F8e reviews r2 to r4 (2026-10-24)

The reviewer read each round. The controller filed this report. r1 is in `2026-10-24-wave-b-f8e-review-r1.md`.

## r2: FAIL
- **C1.** A `school.courses` push onto an unsettled account always started a chain, and test 14 could not hold.
- **I1.** A step-8 ask merged into a running chain slot never got its NoRank pass.

After two failed rounds, the fix went to Opus.

## r3 (`30b103d..48b9c74`): FAIL
r3 checked the Opus fixes, which are commit `48b9c74`:
- **C1 fix:** a `chain_ended` flag, plus `REASON_WAITING_FOR_ACCOUNT` on a pending `school.courses`.
- **I1 fix:** a `no_rank_owed` flag.
- **m-3 (from F2b r2):** `confirm_step10` now runs under one lock.

All three passed. The setup edits to F8e's own `order_tests` `i2` and `m3` are legitimate, since no assertion changed.

**Important finding, I-r3.** `Queue::retry` deleted the record before `push_job`, which refuses `first.*` names. So "Try again" on a failed `first.sync` hid the failure, against §4.6.

**Minor findings:**
- An ask merged into a slot already running as NoRank owed a second NoRank.
- The two new flags are not saved. This is acceptable, as documented.

**Ruling on the fresh, unmerged step-8 ask:** it owes no NoRank. `slot.first_full` covers it, and this is recorded as a known limitation.

## r4 (`48b9c74..e1f927b`): PASS
- **I-r3 closed.**
  - A failed `first.sync` goes back to Pending, and one `slot.partial` is pushed.
  - The chain resets while the account is unsettled, and a second retry does nothing.
  - No `first.*` record is ever deleted.
- **Minor closed.** `partial_no_rank` records the scope of the running slot.
- **Confirmed.** A failed `first.grades` stays Failed on retry until F8g. Nothing re-reads it before then, and the failure stays named.

## Carried to other briefs
- **F8g:**
  - Replace the `first.grades` retry branch with plan :2506.
  - Push the re-read slot behind the running one, not through `enqueue`'s merge.
  - Until F8g, `onboarding_retry` answers `ok:true` for other `first.*` items, though nothing happens. This cannot be reached.
- **Payment and step-8 enqueues:** with C1, a fresh queue starts no chain itself. Payment must ask for `slot.partial` along with `school.courses`.
- **F2b follow-up:** `finish_refusal` should name a `school.courses` that failed after the chain ended, instead of "still saving".
- **`Queue::update_state`:** it neither settles nor wakes the queue. A later edit that changes a job's state through it must do both.
- **F11:** map `first_sync_failed`.
