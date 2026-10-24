# Wave B O3 F8e (order rules, account settled, step 10's gate): review r1 (2026-10-24)

The reviewer checked `git diff 1019847..37fda19`, which touches `onboarding_jobs.rs` (+148/−16) and `onboarding_state.rs` (+9/−1). The controller filed this report. No file on the contract list is touched, and the diff adds no CRLF.

**Verdict: FAIL.** Five Important findings, all small queue-rule gaps.

## Important
- **I1. The NoRank slot can be lost** (`:186-188`, `:245`). The step-8 ask merges into a running BeforeConfig chain slot, and NoRank never runs.
  - Fix: when `config.ingest` goes Done, the queue queues one follow-up `slot.partial`, unless one is already pending.
- **I2. `school.courses` can wait forever** (`:177`). This happens if it is enqueued after the chain has ended, or if an unsettled `slot.partial` fails.
  - Fix: pushing `school.courses` while unsettled, with no `slot.partial` in order, either starts a chain or fails the job with the account-unfinished reason.
- **I3. "First sync" flips from Done to Failed after the account settled** (`:229`). It is honest only while `!account_settled`. Keep the flip only in that case; test 16's between-fail case still holds.
- **I4. `slot.first_full` ignores `school.apply`** (`:180`). §4.3:819 says it must wait. Add `settled("school.apply")`.
- **I5. "No answers file means no config job" has no owner.** §4.5 (:943-946) and `test_17_after_a_rewind…` require it.
  - Ruled by the controller: F8e owns it. `inputs_ready` holds `school.courses`, `registrar.confirm` and `config.ingest` while `load_answers(deps.data_dir)` is absent or unreadable.
  - `deps.data_dir` exists already, so `Deps` gains no field and F1's literal rigs stay intact.
  - F2b's answer path calls `enqueue` or `wake` after it saves, which the controller has told F2b.

## Minor, fold in
- **M1.** `push_job` refuses `first.*` indicator names.
- **M2.** `retry` accepts Failed jobs only.
- **M3.** When the account settles, a `school.courses` that failed with the account-unfinished reason goes back to Pending.
- **M4.** Skip the chain's counting when the stop is set.
- **M5.** Controller's ruling: every reason constant is a short key, like `job_crashed` (`first_sync_failed`, `account_unfinished`), and the page maps keys to text (F11). Split "a cursor could not be saved" into its own key if it is a distinct cause.
- **M6.** The `SyncView::read` doc must match the code: the code also reads after Full.
- **M7.** Doc note for F8g's real `SyncView`: the read runs while the job is `busy`, so it takes no `vault_io` and makes no network call.
- **M8.** Size, +148 against ~75: accepted, no split.

## Rulings
All five are accepted. For ruling 3, the plan's line :2513 will read "settled" instead of "done" in the next plan pass.

## Review r2 (30b103d): FAIL
Closed: I3, I4, I5, M1–M4, M6, M7.

### Critical
- **C1. F1 test 14 is red.** I2's automatic chain uses up the test's only reading. The test also expects the reason "waiting to reach your account" (§4.3:827-828), and no code sets it; it was already red at r1.
  - Fix: start the automatic chain only after a chain has already ended in this queue (a `chain_ended` flag). The resume case stays in `Queue::start`.
  - While the account is unsettled, set the waiting reason on a pending `school.courses`.

### Important
- **I1 is only partly fixed.** The chain case is still open. The step-8 ask merges into a chain slot while `config.ingest` cannot yet be Done, so NoRank is lost. The same loss follows a `config.ingest` that finishes after the slot.
  - Fix: a `no_rank_owed` flag. Set it when a `slot.partial` ask merges, unless the existing slot is Pending and `proposals_ready` already holds.
  - When `config.ingest` goes Done, or a `slot.partial` ends, push one `slot.partial` if the flag is set, `proposals_ready` holds and none is in order. Then clear the flag. Add a test for the chain case.

### Minor
- `load_answers` runs under the lock on every pick. It is bounded; caching is optional.
- Test 14 and test 16 match reasons as prose. A test-writer follow-up will match them against the constants, which then become keys. Until then, F11 maps `first_sync_failed`.

### Questions
1. I1: see above.
2. No deadlock.
3. The `test_03_resumed` setup change is legitimate (the F1 tests contradicted each other). It is recorded for the F1 owner.
4. The prose reason is accepted as a tracked exception.

**Two failed rounds:** the controller escalates the r2 fixes to Opus (CLAUDE.md: two failed attempts means one level up).
