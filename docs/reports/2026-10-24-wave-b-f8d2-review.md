# Wave B F8d2 review (3d5e5f0 vs 3f2b454): PASS

- **Reviewer:** `reviewer`. The controller filed this report.
- **Scope:** no contract-list file is touched. Line endings are LF, and `git diff --check` is clean.

## Important

- **I1. Nothing in the plan tells F8w2 to call `attach_setup_run`.** Without that call, `skip_mail` on a running setup only marks it. The run then finishes as `done`, and the student's Skip does nothing.
  - **Owner:** F8w2's row and test 34's F8w2 part. The planner adds this.
- **I2. A Skip can't keep a setup from starting.** If Skip lands while the glue is still polling for the mail window, the glue later opens the provider's mail window for a student who just declined email.
  - **Fix (F8d2's fix round):** add `Queue::mail_skipped(job)`. The glue checks it on each poll and once more before `start_setup`.

## Minor

- **M1. A `Failed` end after a Skip became a `mail.connect` item.** The student would see "Finish connecting your email" after choosing to skip.
  - **Controller's call (VISION commitment 5):** it lands `skipped`. Fix round.
- **M2. Two families of "retry" constants share the file.** Rename F8d2's to `TIMER_RETRY_*`. Fix round.
- **M3. Size.** F8d2 landed about 115 lines against an estimate of about 30. Each extra piece fills a gap in the plan. Correct the §7 estimate.
- **Retry spacing.** Other jobs finishing re-pick a parked `mail.setup` early, which keeps the serial thread busy while the window stays busy.
  - **Fix round:** `Core::waiting` returns false while the job still has a `retry_at` entry.

## Rulings accepted

1. **`RetryClock` and the idle wait.** There is no busy-wait when nothing is pending, a stop can't hang, and the drain doesn't wait on a parked job.
2. **`attach_setup_run`.** The lock order holds, no lock is held across a window or network call, and every order of skip, attach and finish is safe.
3. **`Stopped`, `Waiting` and `Done` after a Skip** are handled correctly. `Failed` is M1. An edge that existed before this change: a drain-abandoned canary that is then skipped may be rewritten back to `pending`.
4. **Narrowing test_30 is justified.** The first-pass assertions moved into test 34 and got stronger.
5. **The unused `_sleep` parameter** is harmless.

**Open item:** a `done` end clears `parked`, which re-picks `mail.first_pass`. That is intended.

**Interaction with F8h:** none in meaning. The only shared path is the parked-clear in `finish`, which the retry-spacing fix covers.
