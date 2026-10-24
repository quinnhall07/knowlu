# Wave B O3 F8f (stop, drain, freeze, hand-over): review r1 (2026-10-24)

The reviewer checked `git diff 7f6718d..3bd817b` (`onboarding_jobs.rs` and `onboarding_state.rs`, +210/−17). Filed by the controller. Nothing on the contract list is touched, the diff has no CRLF, and it adds no single-user literals.

**Verdict: PASS.** Two Important findings go to the planner as §14.51. The Minors are folded in before the merge.

## Rulings (all five accepted)
1. **The hand-over item set** covers §4.6 plus §4.10's table.
2. **`CanaryVerified` also clears `mail.setup`.**
3. **Only the canary and the first pass are abandoned** (plan :1093).
4. **`wait_idle` ignores runnable-but-stopped jobs once the stop is set.**
5. **`finish` errors when frozen.**

## Important (to the planner, §14.51)
- **I-1: the slots never see the queue's stop.**
  - §4.10 (:1096) describes one `AtomicBool` on `Scheduler`. `request_stop` sets only `Inner.stop`, while `run_slot_scoped` checks `sch.stop`.
  - Example: the student picks Finish later during `slot.first_full`. Every stage still runs, and the drain blocks for the whole slot.
  - The owner is F2b's `ExitSeams::request_stop`, or a `Deps` field. A test should pin it.
- **I-2: `drain` has no time limit** (`:335-336`).
  - Example: a runner misses the stop. The page shows "finishing…" forever, and a second close does nothing (§4.10).
  - A time limit that marks overdue jobs abandoned is safe, because a crash already leaves them `pending`. It needs a duration and wording.

## Minor (fold in, F8f)
- **M-1:** `FirstPassDelivered` also clears `mail.canary` and `mail.setup`.
- **M-2:** a failed `finish` save restores `finished` and `handover` in memory (`:352-353`).
- **M-3:** state on `ABANDON_AT_ONCE` that its runners write nothing and hold neither the notes gate nor the school session lock. Note that `Drop` joins an abandoned runner.
- **M-4:** add a test of this order: abandon the canary, freeze, let it return late, and check the file is unchanged.
- **M-5:** `drain` calls `request_stop` itself, which is idempotent.

## Notes for F2b
- **Call `drain` off the main thread.** A hidden-read window that needs the event loop would deadlock against it.
- **No overlap with F8m.** `batch_closed` is never persisted, so it cannot meet the freeze, the stop or the abandon.
