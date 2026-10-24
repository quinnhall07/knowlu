# Wave B O3 F8s: review r1 (2026-10-24)

Reviewer, on `git diff fa232ec..d648b60` (`app/src/scheduler.rs` only). Filed by the controller. Nothing on the contract list.

**Verdict: PASS.** The implementer reports 0 warnings besides `.rsrc`, and F1 tests 15, 18 and 27 green (27's `main.rs` half belongs to F4).

## Full slots are unchanged
The reviewer read the scoped body line by line against the base. Steps, order, run-log and runner-log lines, exit codes, backoff and tray are all as before.
- The stop checks run only after each stage's original conditions.
- The re-indented grades and mail blocks make the same calls in the same order.
- `scope_off(Full)` is empty, so argv is unchanged.
- The engine-missing arm still records `engine: …` at -1.
- `spawn_update_check` is the tick's body, unchanged. `prime_mode_device` is `spawn`'s line, unchanged. The tick's own priming line is untouched.

## The questions
- **BeforeConfig dropping coursework, ingest, judge and rank:** this is the plan (§4.3 plan:846-853, F8s plan:2390-2396). Round 5 M3 replaced §14.25's `(skipped:)` with `(not yet:)`.
- **The stop:** checked before every stage, giving `(not run: stopped)` at exit 0. `engine_ok` stays untouched and the flag is never cleared (§4.10, §14.18).
- **A stop setting `privacy_held` to false:** safe.
- **The bundle:** deleted on every path.
- **Other checks:** no single-user assumptions; `run_slot_end_to_end` is still ignored; LF line endings.

## Important (tracked follow-up)
Hand-over clearing is deferred, because `slot_ended`, `load_state` and `save_state` are `todo!()` in this base. The owner is a named follow-up after F2a and F8f. It needs a test, and it must cover the refusal paths (script mode, wrong device) and the already-running path. It must also tolerate a vault with no `onboarding.json`.

## Minor
1. **Run-log lines under a stop.** A judge `Skip` row, or a late privacy hold, can still write runner-log lines (`:1100-1115`). §4.10 says a stopped slot adds none. Guard the push with `!stop`.
2. **The engine-missing arm under a stop** (`:1193-1197`) is unplanned and harmless; record it in the plan.
3. **BeforeConfig's judge `Skip`** keeps its `(skipped:)` name and its log line, and nothing tests that.
4. **Telemetry's stop check** comes after the no-account arms. That's fine.
