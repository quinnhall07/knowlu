# Wave B F1 review, round 1 (2026-10-24)

Reviewer, pre-build, on `git diff 35109d1..ffab2ca` against the plan at cfc89ec. Filed by the controller.

**Verdict: FAIL.** The safety rules and the stubs are clean. The failures are in the tests themselves: test 12 is missing cfc89ec's additions, several assertions pass without the behaviour, and a few would fail against a correct F8. Everything can be fixed in the test files, plus small stub additions.

## 1. Coverage against the current plan
- **Test 12, removed courses (R-C1-51): missing.**
  - Stubs: add `removed: bool` (`#[serde(default)]`) to `CourseAnswer`, a per-course `by_course` counts map, `Answers::kept_courses()` and a step-3 totals function.
  - Queue test: capture 3 courses, remove 1, then quit and reload. Assert the removed code never reaches a `run_school_courses` create and is absent from `ingest_handoff().course_map`. Assert step 3's totals equal the sum of the kept rows' counts, and that a re-capture clears the removed mark.
  - Add `removed` and the counts to `full_answers()`, so test 1's round trip covers them.
- **Test 2(b).** Add a queue test: with an unconfirmed school, `school.apply` is enqueued but does not start; after `confirm_school`, it runs.
- **Test 17.** At the queue level, after a rewind with no answers file, `school.courses` and `config.ingest` stay pending and never start; once the answers are saved, they run. Also exercise the real-store wrapper `on_sign_out(data_dir, profile_id)`.
- **Test 26 against plan line 1916.** `CredPending` (`knowlu/<profile_id>/pending-*`) has no real test. Add a real put/get/delete round trip under `CREDMAN_LOCK`, with a generated id and `Drop` cleanup.
- **Test 18.** Pin the exact row `grades (not yet: your courses are not in yet)` from §4.3:845, not just the prefix.

## 2. Tests that pass without the behaviour, and races
- **Canary "abandoned at once"** (jobs.rs:665-671, 750-759).
  - Block the canary with `honor_stop=false`.
  - Assert that finish and sign-out return within about 2 s while its gate is still closed.
  - Drop `abandoned:mail.canary` from test 17's order list.
  - Check the canary's state through `resume(load_state)`.
- **Rigs with no answers file and no school** (state_at :213; tests 3, 4, 8, 11, 16, 23). Have `rig_with` save an answers file (three courses, no feeds) and set a confirmed `School`. Set a confirmed school in test_08 (:469) too.
- **Test 3.** Start the full slot with `q.confirm_step10(..)` after `wake`, or set `step10_confirmed_at`, instead of enqueuing `slot.first_full` directly.
- **FRESH stamp** (:196). Use a real `journal::now_ts` timestamp 1 ms after `started`, not `"{started}~"`.
- **Test 16, the notes gate.** Block `slot.partial`'s sync step for about 50 ms (`fake.block_step`), so an overlap is near-certain without the gate.
- **Test 12, byte equality.** Stub the answer-recording inner of `onboarding_answer(step, …)` (§4.5:948) and drive steps 3-8 through it, keeping nothing in memory across a quit.
- **Test 1, the URL scan.** Route a link through `paste_link_inner` and a fake `school.capture`, then scan both files.
- **Test 19.** Route `write_finish` through the queue (a `Queue` method that sets `finished` and `handover`, then saves). After the quit, assert that `load_state` still shows `finished: true`, and that no job starts on the sign-out arm.
- **Test 4.** Add a comment that the step order it checks is the fake's own; the real argv order is pinned by test 18.

## 3. Safety rules: pass
Real Credential Manager use is only in tests 21 and 22, under `CREDMAN_LOCK` with generated targets and `Drop` cleanup. Servers bind `127.0.0.1:0`, hosts are `.invalid`, and every environment change is serialised and restored.

## 4. Stubs: pass
No `todo!` is reachable from startup, a slot or a registered command. Plan fix: F8h says `week::SYNC_RUNNING` is `pub(crate)`, but it is `pub`.

## 5. Pinned wording: pass, except test 18's grades row (item 1).

## Minors
- Test 27: check that `prime_mode_device` comes before the onboarding window builder in `main.rs`.
- Test 7: pin §4.2's cost key for each step.
- Test 15: put a session in the rig and assert there is no refresh hit.
- Test 3: count `Ent::refresh` calls; there should be one at queue start.
- `School.unitid`: make it `Option<String>`, since the "isn't listed" school has none (controller's call).
- `first_sync()`'s "Pending" checks are weak; leave them.
