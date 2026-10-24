# Wave B O3 F2b (the commands and the three exits): review r1 (2026-10-24)

The reviewer checked `git diff dff57f3..93a66d3`. The controller filed this report. No contract-list file is in the diff, and `account.rs` is called but never edited.

**Verdict: FAIL.** Four Important findings, plus the flaky F1 test.

## Important
1. **The gate opens, and finish is allowed, before step 10 is confirmed** (`onboarding_state.rs:882`). §4.6 (:993) requires "class times (step 10 confirmed)". Without a pending item, the empty list opens the gate.
   - Fix: add a `class_times` item that stays pending while `step10_confirmed_at` is None, and have `finish_refusal` refuse while it is unset.
2. **"An absent job never blocks" is unsafe for `school.apply` and `registrar.confirm`** (`:838`). A crash between two enqueues can leave `registrar.confirm` unrecorded. Finish then deletes the only copy of `registrar_rows`.
   - Fix: all four must be present and done. Only the two feed stores may be absent.
3. **The sign-out order is not pinned.** Moving drain after `sign_out` would pass every test.
   - Fix: add `drained` to test 17's order list or to a new pin, owned by F2b (§14.51 r1). Do it as a new test, not by editing an F1 assertion.
4. **Answers and skips are still written during an exit** (`:1076`, `:1088`). A late step-8 row can recreate the answers file after `on_sign_out`, and the next account then inherits it.
   - Fix: both commands refuse once the stop is requested.

## The flaky test
`test_09_a_job_that_noticed_the_stop…`: the bug is in the F1 helper. `Gate::wait` (`onboarding_jobs.rs` tests, :59-68) checks `open` before the stop.
- **Controller's ruling:** the F2b implementer fixes the helper by checking `honor_stop && stop` before `open`. This changes a test helper only; no assertion changes. Prove it with 30 runs in a loop.

## Minor
- **M-1 and M-2 are F5 notes.** F5 must not register `onboarding_retry` before F8e merges, or `onboarding_sign_out` before F2c merges.
- **M-3:** a second `onboarding_finish` returns early when `state.finished` is already true.
- **M-4:** `Queue::update_state` rolls back the in-memory change if the save fails.
- **M-5:** step 3's capture accepts only the `validated` or `rejected` feed states.
- **M-6:** a note for F4. The live `request_stop` needs both the `Scheduler` and the `Queue` managed.
- **M-7:** log the autostart error instead of `let _`.
- **M-8:** a merge conflict with F8e in `onboarding_jobs.rs` is expected; the controller resolves it.
- **(f):** answer-refusal errors carry the app-data path, which includes the Windows user name. Name only the file (`onboarding-answers.json`), not the path.

## Rulings
- **2, 5, 6, 7, 8:** upheld.
- **1:** upheld. Name the owner of the live `Queue::exiting()` wiring in F8f2's brief.
- **3:** upheld, apart from the class-times item (I-1).
- **4:** overturned (I-2).
