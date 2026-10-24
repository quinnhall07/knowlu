# Wave B O3 F8a (the queue core): review r1 (2026-10-24)

Reviewer, on `git diff 8fbaaff..25c25a3` (`feat/o3-f8a`; one file, `app/src/onboarding_jobs.rs`, +167/−11). Filed by the controller. Nothing on the contract list. No vault writes; the only write is `save_state` into `data_dir`. No single-user assumptions.

**Verdict: FAIL.** Two Important findings, plus one the controller promotes to Important. Nothing Critical.

## Checked and clean
- **Condvars.** No deadlock and no lost wakeup. Workers check `pick` and `shutdown` under the lock before `wait`, and every notify follows a change made under the lock.
- **`wait_idle`.** `finish` and the next `pick` share one guard, so it never sees a false idle.
- **Persistence order.** A job is saved `running` before its runner starts. Its result is saved under the same lock `snapshot` reads.
- **Panics.** A runner panic becomes `Failed`, and the lock is never poisoned.

## Important
1. **Resumed jobs run in alphabetical order** (`:209-211`, `BTreeMap`). §4.3 says jobs run in the order their asks were answered.
   - Example: `mail.canary` runs before `mail.setup`, so the canary waits 3 minutes for a mail nobody arranged, fails, and blocks the serial lane.
   - Fix: a const canonical job order, in step and ask order, used to sort resumed jobs.
2. **The startup entitlement read can overwrite a newer `wake()`** (`:170-172` against `:236-238`).
   - Example: the student pays during the startup refresh. `wake` sets entitled, then the stale startup read sets it back to false, and every gated job is held for good.
   - Fix: a `woken` flag (or a counter) that `wake` sets, so the start skips its write once wake has run. Combining the two results with `||` also works.
3. **`Drop` never sets `inner.stop`** (`:197-202`). This is Minor in the review; the controller promotes it because it hangs CI.
   - Example: a failing assertion unwinds while F1's fake is blocked on a stop-honoring gate. `join` then waits forever.
   - Fix: store `stop = true` before the notify. The flag is never cleared (§4.10).

## Minor (fold in)
- **`school.capture` and `school.preview` wait for the startup refresh** (`:179`, `pick`'s `ready`). §4.3 says they run at once. Let ungated jobs be picked before `ready`.
- **The panic reason is free text** (`:186`). Use a closed-set reason key with its fix (§4.3).
- **A panic in `entitlement.refresh()`** (`:170`) kills the serial thread before `ready`, and everything waits forever. Wrap it the way the runner is wrapped.
- **The doc comments are out of date** (`:148`, `:4-5`). Say the queue is the only writer of `onboarding.json` while it is live, so F2 and F4 go through it. The module doc still says every function panics.

## Rulings
1. **`confirm_step10` built early:** upheld (test 3 needs it). F8e's brief now covers only refining the step-10 gate.
2. **`slot.partial` scope:** upheld. The scope is chosen when the job is picked.
3. **The Runner holds the AppHandle:** upheld. Tell F8g.
4. **Enqueue, park and resume:** upheld, except the resume order (Important 1).
5. **167 lines, not split:** upheld. F8e's brief drops "the second, serial lane".
6. **M5 goes to F4:** upheld. `Queue::start` saves the state it is given once.

Not run: tests 3 and 5 wait on F2d's `save_answers`.
