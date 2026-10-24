# Wave B F1 review, round 2 (2026-10-24)

Reviewer, pre-build, on `git diff 4784021..16b02bd` (99a2a86, 6dcaecf, 16b02bd) against r1 and the plan at 47672d4. Filed by the controller.

**Verdict: FAIL.** The r1 items are addressed, but two new tests pass without their behaviour. No contract-list file is in the diff.

## Important
1. **Test 16 lost the half of the gate where a note job runs first** (`app/tests/onboarding_jobs.rs:1191`). Neither test 16 now has a note job holding the gate while a slot is asked for.
   - Failure: F8h makes the lane wait for slots without taking the gate itself. `school.courses` is mid-write when `slot.partial` starts, and its `sync` collides (§14.19 I-B). F1 stays green.
   - Fix: keep the new test, and add one where `school.courses` is held open with `fake.block` while `slot.partial` is asked for. Assert no `start:slot.partial` before `end:school.courses`, then `assert_clean`.
2. **Nothing tests which jobs `onboarding_answer_inner` enqueues** (`app/tests/onboarding_state.rs:907-930`). `enqueued` is never asserted, and `.all()` runs over a possibly empty `saw`.
   - Failure: an F2b that saves and never enqueues passes. Step 3 never starts `school.preview`, and step 6 never starts `google.calendar_read`.
   - Fix: assert `!saw.is_empty()`, and pin step 6's `google.calendar_read` and step 3's capture job.

## Minor
- `onboarding_jobs.rs:700-731`: the sign-out arm's canary check is vacuous, since the rewind resets every job to pending and `unwrap_or(Pending)` hides a missing entry. Drop the check or say so.
- `scheduler.rs:3236-3238`: the anchor is the last `.build(..)`, which is `run_console`'s, so a prime placed in `run_shell` also passes. Scope the search to `run_console`'s body.
- `onboarding_state.rs:906`: the capture has only `courses`, but the stub's doc lists `registrar_rows` and `feeds` with no default. Say "default empty" in the doc, or pass both fields.

## r1 items: closed
All of §1, §2 and the Minors (test 16 only partly; see Important 1).

## The test-writer's rulings
- **Test 15 on the real Credential Manager:** acceptable (generated target, lock order, cleanup).
- **Tests 17 and 26 on the real Credential Manager:** acceptable.
- **Test 7 pinning properties rather than strings:** acceptable.
- **Answer shapes:** acceptable as of the plan before §14.45; see the delta below.

## Safety: pass
Credential Manager use is only under the lock with generated ids and `Drop` cleanup. Servers bind `127.0.0.1:0` and use `.invalid` hosts. All data is synthetic. No new `todo!` is reachable from `app/src` or a `generate_handler!`.

## New from §14.45 (not counted toward the verdict)
1. `IngestHandoff` has no `course_exclude`. Test 12 must assert the removed course's pairs are under `course_exclude` and in no `course_map` entry (plan :1767-1769).
2. `!contains("knotwork")` (~`onboarding_jobs.rs:1600`) now contradicts §14.45. Narrow it to `course_map`.
3. `CourseAnswer.counts` can't hold match-set buckets. Use buckets keyed by the sorted code set, with the empty set as `unmatched` (plan :2254-2260, :2359).
4. `step3_totals` should total every bucket whose set is empty or names a kept code. Cover a two-code bucket with one code removed, then both, and `unmatched` counted.
5. `CourseAnswer` needs a field for the pairs F8b saves: code fragments and gradebook uid pins (:2362).
6. `{"remove": <code>}` and recapture's identity need `by_course`'s key: code, then slug, then label (:2246).
