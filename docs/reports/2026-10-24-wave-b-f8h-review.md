# Wave B F8h review (0bc9173 against c4b410f): PASS

Reviewer: `reviewer`. Filed by the controller. The diff touches only `app/src/onboarding_jobs.rs` and `app/tests/onboarding_jobs.rs`. No file on the contract list is touched.

## Rulings checked

1. **Retry budget.** The code does 120 sleeps of 250 ms, so 30 s of waiting across 121 tries. Past that it returns the `week::SYNC_RUNNING` refusal itself, which matches the plan in §4.3 and §8.
2. **`done("school.apply")`.** This is correct: "settled" would let the confirm run against a `campus.yaml` that was never written.
   - A failed apply does not leave finish stuck in a silent wait. `finish_refusal` requires `school.apply` to be Done.
   - The apply answers only Done or Failed, and Try again re-runs the lane.
3. **The test rig change** (`state_applied` in the two `test_16` rigs and one in-source test) restores the readiness the base had. It weakens nothing.
4. **Lock order.** Each try's `vault_io` and lock drop before `sleep`. Neither function takes the queue lock or `ANSWERS_LOCK`.
   - F8h-runner's reviewer should check one case: if `work()` calls `update_answers`, the order becomes `vault_io` then `ANSWERS_LOCK`.

## Minor findings (fixed by the controller)
- **`test_17_after_a_rewind...`.** Its rig never marked `school.apply` done, so its unreadable-answers block passed without testing anything. It now uses `state_applied(8, true)`.
- **Busy refusal assertions.** Test 36's case "a refusal past the budget is the busy refusal" asserted only `is_err()`. It now asserts `Err(week::SYNC_RUNNING)` for `retry_with_hold`, and a direct case for `retry_on_sync_running` was added with the same budget.
