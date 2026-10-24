# Wave B F2b review r2 (2026-10-24)

The reviewer read `e258582..aab9224` against the r1 list. The controller filed this report.

**Verdict: PASS**, with five Minor findings. The reviewer did not run cargo; the stub failure counts were checked by reading the tests.

## r1 items, closed
- **I-1:** a `class_times` gate item; finish refuses while step 10 is unconfirmed.
- **I-2:** `school.apply`, `school.courses`, `registrar.confirm` and `config.ingest` must be present and done.
- **I-3:** test 17 pins the sign-out order with a writer that ignores the stop.
- **I-4:** answer and skip refuse once `exiting_now` holds.
- **The `Gate::wait` helper:** the stop is checked before the gate; no assertion changed.
- **M-3, M-4, M-5, M-7 and (f).**

## Failure counts
- None of the F2b-owned tests calls a stub. The failing tests numbered 12 and 17 belong to F8b, F8c and F8e.
- **Caveat:** the refusal of the `buckets` and `pairs` shapes stays unproven until F8b lands.

## Residual race: named follow-up
An answer can pass `exiting_now` just before the stop, then write after `on_sign_out` deletes the answers file.

**Fix (about 10 lines):**
- `on_sign_out_with` takes `ANSWERS_LOCK` around its delete.
- `update_answers` and `replace_answers` check the stop inside that lock.

**Owner:** F2c, or a small F2b follow-up. It must land before F5 registers `onboarding_sign_out`.

## Minor
- **m-1.** A second finish on a finished file answers `ok:true` without exiting. It should return `{ok:false, restart:true}`.
- **m-2.** `exiting_now` also counts the exit guard, so an answer sent during finish's refusal check is told setup is closing. The stop alone is enough.
- **m-3.** F8e's `confirm_step10` stamps the confirm and enqueues `slot.first_full` under two locks, so the gate can open early. This goes to F8e: use one lock.
- **m-4.** M-1, M-2 and M-6 stand deferred as ruled. F5 waits for F8e and F2c.
- **m-5.** The live `exiting` progress waits for F8f2. The fix text waits for F8g.
