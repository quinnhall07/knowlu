# Wave B O3 F8m (the backfill hook): controller review (2026-10-24)

Controller, on `git diff 7f6718d..1042be3`: `app/src/mail/live.rs` and 4 lines of `app/src/onboarding_jobs.rs`. Nothing on the contract list.

**Verdict: PASS.**

## The change
- **The new function.** `on_batch_closed(mode)` is a pure match: onboarding mode gives `TellQueue`, the normal console gives `RunSlot`.
- **The `on_closed` hook.** It reads the managed `LaunchMode` with `try_state`; when none is managed, the mode is `Normal`, as in F4b's guard.
  - In onboarding mode it records the batch on the managed `Queue` (`mail_batch_closed`), when there is one, and starts no slot.
  - Otherwise the slot spawn is unchanged.

## Notes
- **The `onboarding_jobs.rs` hunk.** It is the minimal body for the two F8m stubs: an `AtomicBool` on `Inner`, outside `Core`. Accepted. When F8f merges, expect a one-line textual conflict on `Inner` or `Queue::start`.
- **F4's wiring.** Until F4 makes `LaunchMode` and `Queue` managed state, the hook reads `Normal`, so today's behaviour is unchanged.
- **The `mail` tests.** Two timing tests timed out under a concurrent full run and pass alone. Both are load-flaky gmail-script retries; this diff doesn't touch them.
