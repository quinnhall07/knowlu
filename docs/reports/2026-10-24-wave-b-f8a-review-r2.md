# Wave B O3 F8a: review r2 (2026-10-24)

Reviewer, on `git diff 25c25a3..d5ff94e`. Filed by the controller. Nothing on the contract list.

**Verdict: FAIL.** One Important blocks the merge.

## Round 1, closed
- **Resume order:** fixed (`CANON`, `:236`), apart from I-1 below.
- **Wake race:** fixed (`woken`, `:262`). The race test fails on the old code and passes on the new.
- **Drop:** fixed. It sets the stop first (`:220-221`), and test 9 proves it.
- **Minors:** closed.

## Important
- **I-1 (blocks).** `CANON` (`:101-105`) leaves out `calendar.read`, so on resume it runs after `slot.first_full`, and the first `today.md` misses the student's busy time.
  - Fix: add it after `calendar.store`.
  - Test: every job name F1 uses appears in `CANON`.
- **I-2 (planner).** The release profile has `panic = "abort"` (`Cargo.toml:24`), so `catch_unwind` (`:177`, `:207`) catches nothing in the shipped app.
  - A job that panics every time crash-loops the app at each launch, because `start` resumes it first.
  - In F8a: reword the doc comments to say they hold under unwinding only.
  - For the planner: a crash-loop guard. A job found `running` at start twice in a row becomes `failed` with `job_crashed`. It needs a field in F2a's record.

## Minor
- **M-1.** The startup read should keep a fresh `true` even after a wake: `if !g.woken || ent` (`:190`).
- **M-2.** The race test's gate ignores the stop, so a failing assertion hangs the test instead of failing it. The detach below removes this.

## Detach the start thread
`refresh_entitlement` can take about 60 s, plus any wait on `SESSION_REFRESH_LOCK`. No §4.10 exit drops the `Queue` today.

Ruling: detach. `startup` checks `shutdown` under the lock and returns without saving if it is set, so a detached starter never writes a stale `onboarding.json`.
