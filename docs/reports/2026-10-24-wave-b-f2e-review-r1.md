# Wave B O3 F2e (the sign-out rewind): controller review (2026-10-24)

The controller checked `git diff 7f6718d..995a7ac` (`app/src/onboarding_state.rs`, plus the signature change in two test fakes). Nothing on the contract list is edited: `credentials::delete` is called, not edited.

**Verdict: PASS.**

## What it does
- Deletes the answers file (NotFound is fine) and both pending links. Each failure is collected and joined, so one stuck file never stops the rest.
- Rewinds an unfinished file to step 2. Every job goes back to `JobRecord::default()`. `account_settled`, `school`, `step10_confirmed_at` and `handover` are cleared, and skips are kept.
  - Clearing `step10_confirmed_at` and `handover` is the controller's ruling: a step-2 file is a fresh install's.
- Leaves a finished file alone.
- Reports an unreadable file as an error, never as absent.

## Checks
- **F2d review M2 is closed.** `PendingStore::delete` returns a `Result`, so a link that is there and stays is reported.
- **Error strings name paths and credential names only.** No link value reaches one.
- **Sign-out order.** The queue may still be live while the state is written. F2b's sign-out stops, drains and freezes the queue before it calls this (§4.10), so the queue never overwrites the rewind. F2b's brief carries that order.
- **The feed states** in the brief live in the job records, which the rewind resets. `OnboardingState` has no separate feed field.
