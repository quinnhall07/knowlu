# Wave B plan review, round 3 (2026-10-23)

Reviewer (spec/plan mode) on `docs/plans/2026-10-23-wave-b-long-onboarding-plan.md`, o1 worktree at `49b0b43`. Filed by the controller.

**Verdict: FAIL.** 0 critical, 3 important, 6 minor. Five of the six r2 findings are closed in the text and match the code. Three new holes come from the fixes themselves.

## How the r2 findings stand
- **I5 closed.** `Restored.ok` exists (`sync.rs:938-944`). F1 test 14 and F6a/F6b test for restored notes.
- **I4 closed, except the lock (I-B).** `commitments_report_with` reads its feeds from `ingest.yaml` (`cli.rs:975`), so gating step 10 on `config.ingest` is the right gate.
- **I1 and I2 closed for quit and crash, but not for Sign out (I-C).**
- **I3 mostly closed (I-A, M1).** Slot order matches `slot_argv` (`scheduler.rs:528-552`: grades before ingest). `is_first_run` checks only whether `today.md` exists (`ingest.rs:493`).
- **I6 partly closed (M4).**
- **r2 M1-M5:** M2-M5 are closed; M1 holds, with a caveat (M3).

## Important

**I-A. The stop flag is cleared at slot start, which breaks its own test and lets a race run a whole slot** (§4.10; F8f).
- F8f makes the stop "a field on `Scheduler`, cleared at each slot's start", but F8f's own scheduler test sets the stop *before* the slot and expects no child to start. That test cannot pass as written.
- The race: the queue thread dequeues `slot.first_full`, then the student presses Open Knowlu and the stop is set, then `run_slot_with` clears it. Every step runs, up to `CHILD_TIMEOUT` (20 min) each, which brings r2's I3 back.
- Fix: never clear the flag inside the scheduler. It lives for the process, and onboarding ends in a relaunch. Pin it with "stop set before the slot".

**I-B. The local lane's note-write hold refuses rather than waits, and collides with `slot.partial`'s `sync` at payment** (§4.3, §4.2 step 4, F8e, F8c).
- The hold the plan borrows is `run_confirm`'s (`week.rs:104-117`): `vault_io` plus `state/sync.lock`. If a sync holds the lock, it refuses ("a sync is running; try again in a moment"). If the confirm holds it, the slot's sync takes `sync (skipped: another sync is running)`.
- At payment, `school.courses` → `registrar.confirm` (lane) and `slot.partial`'s `sync` (queue) start together. One of two things happens:
  - `registrar.confirm` fails and Open Knowlu is blocked until the student presses Try again; or
  - the first sync is skipped at exit 0, and the gate's "first sync" can count it as done.
- F1 test 11 ("never waits") leaves this case unspecified.
- Fix: a lane note write waits (bounded) for `sync.lock`, or the queue does not start `sync` while a lane note job is running. A skipped `sync` never satisfies "first sync". Add an F1 test with the two starting together.

**I-C. Sign out mid-onboarding deletes the answers and the pending link, but leaves `onboarding.json` claiming they exist** (§4.5, §4.11, F2c).
- F2c deletes `onboarding-answers.json` and both `pending-*_ics` credentials. `onboarding.json` (step 8, `school.capture` done, feed `validated`) stays, and the picker resumes it as "setup not finished".
- §4.5 covers an *unreadable* answers file, not an *absent* one. On resume, `config.ingest` runs with no course list: an empty `course_map` kept forever by write-once. `school.store` has no link, so the feed is lost (r2's I1 and I2 again).
- Fix: sign-out rewinds `onboarding.json` to step 3 with jobs reset, or an absent answers file past step 3 is treated as unreadable. Add an F1 test: sign out at step 8, sign in, resume.

## Minor
- **M1. The stop is only checked before engine children** (F8f, §4.10).
  - Before the loop, `run_slot_with` runs session, entitlement and privacy refreshes, a hidden grades capture and the mail pass. After it come backup and telemetry (`scheduler.rs:840-975`, `:1078-1110`). None of these honour the stop, so "one engine step at most" (Q-B16) is not what gets built.
  - F8g's "then the mail step" would run a second mail step: `run_slot_inner` already runs one.
- **M2. `hard_link` support is confirmed on one volume (T0's), not checked at runtime** (§4.9, F6a).
  - On an exFAT/FAT or network volume, `CreateHardLinkW` is unsupported. The job fails, and §4.10 blocks Open Knowlu on the local lane, for every student with such a vault.
  - Fall back to `MoveFileExW` without the replace flag at runtime, and test the fallback.
- **M3. `sign_in_hosts` comes from one live sign-in on one device** (Q-B14, S2).
  - A classmate sent to Microsoft's first-time MFA registration or a conditional-access host meets a refused navigation and a dead "needs sign-in".
  - Fall back to the mail window on its own folder (today's path) rather than stopping.
- **M4. The I6 claim is incomplete.** §14.16 says every implementer task has a line estimate. F2a, F2b, F3 ("S to M"), F4 and F9 have none. F2a (two serde files, §4.7 resume derivation, skip rules, `LaunchMode`, credential names) looks like it runs over 80 lines.
- **M5. Test order.** F8f's `app/tests/scheduler.rs` test is written by the implementer in the same task. Move it to F1 or a test-writer step, as F6 and F7 do.
- **M6. Privacy** (S7, Q-B15).
  - Two on-device data classes are new and not on the page: the pending feed link in Credential Manager, and the answers file (course names, class times) kept before payment.
  - Q-B15 says "deleted at the reveal", but an abandoned setup keeps both until sign-out or deletion. Add a sentence to S7, or record the choice.

## Checked and holding
- **Credential name.** `knowlu/<profile_id>/pending-lms_ics` fits `credentials::target_for` (`credentials.rs:17`), and `credentials.rs` is left unedited. `retarget_credentials` moves only zybooks and vhl (`onboarding.rs:1028`); no onboarding path moves a vault, so that is fine.
- **Spec §1.4 B.** The answers file is app data, not a note; spec §1.6 already puts each step's answer on the device.
- **Routing.**
  - The three `account.rs` tasks (S5, S7, F2c) go to `contract-engineer` then `contract-reviewer`. F2c and S5 use `CREDMAN_LOCK`, generated ids and `Drop` cleanup.
  - `delete_local_data`'s `remove_dir_all(data_dir)` already removes the answers file (`account.rs:1673`).
  - `sign_out_with` already takes a `clean_unfinished` closure, so the answers-file delete could live in the caller and keep F2c's `account.rs` edit to the two credentials.
- **Tests stay local.** F3 test 5 uses `127.0.0.1:0` and a closed port; F1 uses fakes for the store and the pending credential.
- **Frozen references are untouched.** F7 builds its inputs inline and runs `surface_oracle` unedited.
- **No single-user assumptions.** The founder's row is curated data, and its comment names no school.
- **Vault writes.** Course notes go through `write`, and the config files are created once, never re-dumped. `commands.rs` gains no computation.
- **Fidelity ledger.** Every r2 row maps to a task and a test.
