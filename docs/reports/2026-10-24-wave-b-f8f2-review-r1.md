# Wave B F8f2 review r1 (2026-10-24)

The reviewer read `3bb46bc..32cdb5c`. The controller filed this report.

**Code: passes.** The drain abandons no writer. It abandons only `ABANDON_AT_ONCE` (the canary and the first pass), has no time limit, no early return and no new job after the stop. `exiting()` stays readable while the drain waits. The rulings are agreed: the slot is named first, `since` is when the first drain began, F2b's `ExitingNow` is reused, and the abandoned-count rule is left to F8a2.

## Minor (tests), fixed in 93b1106
1. The drain source pin was easy to satisfy. It now forbids `timeout`, `sleep`, `Duration`, `return` and `break`, checks that there is exactly one `abandoned.insert`, and requires the abandonment to read `ABANDON_AT_ONCE.contains`.
2. The 600 ms fake could finish before the drain began. It now waits on a gate that is released only after `exiting()` shows the drain waiting.
3. A regression would hang the first-pass case rather than fail it. It now runs on a scoped thread with a 2 s deadline.

The controller read the test diff. Test 30 passed 25 of 25 repeat runs.

## For F11a
After the drain returns, `exiting` is null, yet the save, flush, sign-out and relaunch still run (up to 10 s). While an exit call is outstanding, the page must not read null as "not closing".

Two other Minor findings stand:
- `snapshot()` and `exiting()` take the lock separately.
- The rule that `since` is kept across later drains has no test.
