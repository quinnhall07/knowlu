# Wave B plan §14.51: review r1 (2026-10-24)

The reviewer checked plan mode on `git diff 6448794..e64d420` against the code in o3-long-onboarding. The controller filed this report.

**Verdict: FAIL.** One Critical and five Important findings.

## Critical
**C1. The drain limit abandons jobs that keep writing** (§4.10 ~1108-1113, F8f2 ~2557-2561). `school.capture` holds the school session lock and writes the answers file and the pending link credential, but an abandoned thread keeps running.
- Example: Sign out during a slow hidden read. The capture is abandoned, `on_sign_out` deletes the answers and the links, and then the capture writes them back. They outlive sign-out.
- `mail.setup` has the same problem in a milder form.
- Fix: limit the list to runners that write nothing anywhere, or have the exit wait for every abandoned thread before `on_sign_out`. Test 30's fake `school.capture` encodes the unsafe behaviour.

## Important
**I1. A slot stopped part-way is recorded as `done`** (F8g ~2675-2679, test 29). A stopped `run_slot_scoped` sets no reason, so `Core::finish` makes the job `Done`.
- Example: Finish later during `slot.first_full`'s sync stage. `rank` never runs, and the job is never retried.
- Fix: F8g maps a slot summary whose stop was set to `Stopped`, with a test over the real summary.

**I2. The canary signal reads the wrong field** (F8s2 ~2608-2611). `confirmed` is the provider's address confirmation, which happens before the canary. The canary-back signal is `last_canary_ok_at` (`runner.rs:872`).

**I3. `last_reconcile` is not "first pass delivered".** It is written only after a normal slot's recovery pass (`mail/slot.rs:246-251`), and never for M365. A closed backfill (`open_backfills` cleared, `backfill.rs:91`) is likelier. The plan should rule on this now rather than leave it as a stop-and-report.

**I4. The disconnect signal needs the unfiltered list.** `mail_step`'s rows drop revoked and turned-off rows (`scheduler.rs:471-480`). Also, an abandoned setup deletes its row (`runner.rs:386`), so test 32 needs a "row absent" case.

**I5. The crash guard counts every kill as a crash.** Windows shutdown, End task, and the job beside the crashing one all count, and the wording blames the step.
- Suggested wording: "Knowlu stopped twice while this step was running, so it has paused it. Press Try again to run it once more."

## Minor
- **M1.** One rule for an abandoned job's count: the drain writes `default()`, and a late `Stopped` resets the count to 0.
- **M2.** Test 31 expects `Queue::start` to save, but it persists nothing today. Say that it does.
- **M3.** One slot stage can run to `CHILD_TIMEOUT` (20 min). "This can take several minutes."

## Held up
- `Scheduler.stop` is not shareable.
- The optional `crashes` field leaves existing onboarding.json bytes unchanged.
- The refused-slot paths return early.
- The 55 s worst case has no single-instance conflict.
- No slot is abandoned.
- There are no vault writes after exit.
- Each new task has an owner and a test.
