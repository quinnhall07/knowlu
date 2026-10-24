# Wave B F4 review r1 (2026-10-24)

The reviewer read `3bb46bc..e41fcf3` (`app/src/main.rs`, `app/tests/scheduler.rs`). The controller filed this report.

**Verdict: PASS.** Two Important follow-ups must be recorded. Neither blocks the merge into O3.

## Ruling: the Queue is not managed yet
F4 neither manages the onboarding `Queue` nor calls `Queue::start`. The reason: `Queue::start` needs `Deps` (a `Runner`, an `Entitlement` and a `SyncView`) and a live `ResumeChecks`. None of these has a live implementation until F8g.

**Accepted as a follow-up.** Without a queue, `LiveExit`'s queue seams do nothing, and a close takes the tray Quit's path: stamp `quit_at`, a flush capped at 10 s, then exit. That is no worse than today.

## Important
- **I-1. New task F4c, after F8g.**
  - Manage the `Queue` and call `Queue::start(deps, F2a's derived state)` in the onboarding arm.
  - Extend test 27's pin to `.manage(queue)`.
  - **Before** F4c persists the derived state, it must rewrite `onboarding.json` only on a parse failure, and retry a transient I/O error first. The reason: `load_state` treats any non-NotFound error as `Unreadable`, for example antivirus or a sync-tool sharing violation, or a downgrade. Once the queue persists that, a finished student would be re-onboarded.
  - Without F4c, a close in onboarding sets no stop and drains nothing, and an unreadable file is never rewritten.
- **I-2. O3 must not merge to main until F4c and F5 land.** On O3, a profile in onboarding mode has no tray, no scheduler, no queue and no finish command (F5). The window closes, but the profile is inert.

## Follow-ups
- **`install_update`.** It stays reachable in onboarding mode, and the one staged check can stage an update. Once the queue exists, an install would skip its drain. Refuse it on `LaunchMode::Onboarding`, in F4b's style, alongside F4c.

## Minor
- **The hide-to-tray branch was not pinned.** Controller fix: the F4 test now asserts `} else { let _ = w.hide(); }`.
- **The test splits setup on the first `} else {`.** If that breaks, the test fails loudly, never falsely. A stray indent was fixed.
- **A failed close thread spawn leaves the window unclosable.** The only sign is an `eprintln`. Very unlikely.

## Held up
- **Update checks:** exactly one in onboarding mode, and none from the scheduler.
- **The scheduler:** never spawned in onboarding mode.
- **The close:** no deadlock. Every close ends the process.
- **`prevent_close`:** runs in both modes, with the branch after it depending on the mode.
- **Nothing installs.**
- **No contract-list file**, and no `generate_handler!` change.
