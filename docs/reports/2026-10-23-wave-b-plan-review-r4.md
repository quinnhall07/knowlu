# Wave B plan review, round 4 (2026-10-23)

Reviewer (spec/plan mode) on `docs/plans/2026-10-23-wave-b-long-onboarding-plan.md`, o1 worktree. Filed by the controller.

**Verdict: FAIL.** 0 critical, 3 important, 6 minor. Three of round 3's findings are closed in the text: I-A for every exit except Sign out, I-C's file rules, and all six minors. I-B's "a skipped sync is not a sync" depends on a signal the code never gives. Two new holes sit in the exits and in `slot.partial`.

## How the r3 findings stand
- **I-A: closed for Open Knowlu, the window's close and a crash.** Onboarding mode never calls `scheduler::spawn` (P-3, F4), so no tick shares the never-cleared flag. A stopped slot leaves sane state. It is not given the tray handle in onboarding mode (no tray there). There is no backoff without the tick. `engine_ok` stays true. The normal console never sets the flag. Sign out is still open: I-1.
- **I-B: the notes gate is sound, the skip test is not (I-2).** The lock order is the same everywhere: gate, then `vault_io`, then `sync.lock`. The in-process takers of `sync.lock` (Sync now, grades connect) take `vault_io` first. No path holds `vault_io` while waiting for the gate. I found no deadlock. The `account_settled` ordering is coherent.
- **I-C: the file rules are closed (§4.5, F2d, F2e, F1 test 17), but the exit that triggers them is unbuilt (I-1).**
- **r3 M1-M6: closed in the text.** One gap remains in M1 (M1 below). Ledger rows exist for I-A, I-B, I-C and M6 (§10).

## Important

**I-1. Sign out and Finish later have no command, no owner and no test, and the stop flag's safety rests on them** (§4.1, §4.5, §4.10:808-813, F2b, F5, F11a).
- In the code:
  - `account::sign_out` returns JSON and ends nothing (`app/src/account.rs:710-729`, `:783`).
  - `relaunch` only spawns the new process; ending this one is the caller's job (`app/src/onboarding.rs:399-412`).
  - The console page never calls `sign_out` (no hit in `app/static/console.js`).
- So T0's check "Sign out relaunches into the shell" (plan:1081) fails today. "F2b makes so if it does not" is in no task brief.
- F2b's five commands have no sign-out or quit command. F11t lets screens call only §4.5's and F5's lists, so F11a's Sign out button would call `account::sign_out` directly.
- Scenario: sign out at step 8 while `mail.canary` waits.
  - There is no drain and no relaunch, and the process lives on.
  - `on_sign_out` rewinds `onboarding.json` to step 3.
  - The live queue then saves its in-memory state (step 8, `school.capture` done) over the rewind. I-C is back, held off only by the absent-answers backstop.
  - The student signs in again in the same process, and its queue keeps round one's in-memory `account_settled` and jobs.
- The rewind also has no route back to sign-in:
  - Sign-in is step 2, in the shell (§4.2:492).
  - `LaunchMode` sends any unfinished file to onboarding mode (F1 test 10).
  - So a relaunch, `--pick` included, opens step 3 with no session. Payment's `entitlement_now` then cannot succeed, and no screen asks for a sign-in.
- Fix:
  - Add `onboarding_sign_out` and `onboarding_quit` to F2b: drain (F8f), switch off the queue's saves, then `account::sign_out`, then `relaunch(["--pick"])`, then exit.
  - Rewind to step 2, or route an unfinished profile with no session to the shell's sign-in.
  - Add both to F5's list and F11t's allowed list. Extend F1 test 17 to show that no save lands after the rewind.

**I-2. A skipped `sync` never carries the name the plan keys on** (§4.3:613-615, F8g, F9, F1 test 16).
- `run_slot_with` names the step by `args[0]` with the child's exit code, so it is always `sync`, exit 0 (`app/src/scheduler.rs:1004-1008`, `:1060-1067`). The skip is recorded only in `state/sync-status.json` (`engine/src/sync.rs:2523-2553`, `totals.skipped`).
- F1 test 16's fake step name `sync (skipped: another sync is running)` never comes out of the real slot. Only `state.rs:245` builds such a line, and it goes into `cs.sync`, the status `refresh_sync` reads back, not into the slot's steps.
- Scenario: the slot's sync meets "no session", or another process's lock. It exits 0 named `sync`, F9 counts the first sync as done, and the reveal claims a sync that never ran.
- T0's check at plan:1084 can already be answered from the code: no.
- Fix: decide "ran" from the status record, as `account_settled` already does (`cs.sync` after `state::refresh_sync`, `scheduler.rs:1116`). Make test 16's fake a status record, not a step name.

**I-3. `slot.partial`'s own child loop leaves steps out without naming them** (§4.3:637-639, F8g; CLAUDE.md "left out and named").
- `slot_argv` does no naming of its own:
  - it adds `grades` only when there is a bundle (`scheduler.rs:530-532`);
  - it adds no `judge` step for `JudgePlan::Skip` (`:550`).
- The naming is `run_slot_with`'s:
  - the `ingest`, `grades` and `judge` skips (`:806-812`, `:955-956`, `:981-983`);
  - the runner-log lines for them (`:1033-1057`, D8).
- `run_slot_with` also applies the privacy hold (`:990-995`) and the session refresh (`:840`).
- A loop over `slot_argv` minus `rank` through `run_child` does none of this. The grades capture is a stage of `run_slot_with`, and F8b keeps no step-3 bundle.
- Scenario: `slot.partial` runs with no grades step and nothing names why. The gate's "first grades" never resolves, the reveal shows it "still coming", and nothing is written to `state/runner-log.md`.
- Fix: run `slot.partial` as `run_slot_with` with `rank` left out (one parameter). The skip naming, privacy hold and stop checks then come with it. Add an F1 case: no bundle gives a named `grades (skipped: …)` row.

## Minor
- **M1. A stop can leave a grades bundle on disk** (F8f, F1 test 15). If a stop check returns early after the grades stage, the bundle is never deleted (`scheduler.rs:1075-1079`, "whatever the engine's exit code"). Require the delete on every path, and assert it in test 15's second bullet.
- **M2. The 30-second retry should not hold `vault_io`** (§4.3:607-609).
  - `run_confirm` holds `vault_io` and then tries `sync.lock` (`app/src/week.rs:115-116`). `registrar::spawn` takes the same two (`app/src/registrar.rs:96`, `:104`).
  - Retrying with `vault_io` held blocks every other `vault_io` user in the process (the page's note writes, telemetry) for up to 30 seconds. Release both between tries.
- **M3. F2c's test cannot reach F2c's edit.**
  - The closure lives in `sign_out`, which needs an `AppHandle`. Tests call `sign_out_with` with their own closure, so they pass even with the wiring missing.
  - Fix: extract a pub `account::sign_out_clean(dir, id)` and test it, under `CREDMAN_LOCK` with generated ids and `Drop` cleanup.
- **M4. F8e is already past 80 lines.** It holds the second lane, the gate, the retry, the config-before-readers rules, `account_settled`, slot order and the proposals flag. Split out the gate now rather than on the condition.
- **M5. `onboarding_finish` has no plan for a failed relaunch.** `relaunch_with` can fail, `current_exe` or the spawn. By then `finished: true` is written and the stop is set, so the process lingers with every Try again dead. Exit anyway, or show "restart Knowlu".
- **M6. Two console exits skip the drain in onboarding mode.**
  - `switch_profile` (`app/src/commands.rs:669-684`) and `delete_my_data` stay on the console list. Neither drains, so either can cut off an engine child.
  - Either make F11t pin that no onboarding screen calls them, or make both refuse in onboarding mode.

## Checked and holding
- **`MoveFileExW` fallback.** Without `MOVEFILE_REPLACE_EXISTING`, the temp file in the same `config/` folder fails rather than replaces, and it is tested through a seam (F6a).
- **F2c.** It edits only `account.rs` (contract list), through `contract-engineer` then `contract-reviewer`. The `for source in ["zybooks", "vhl"]` loop and `remove_dir_all(data_dir)` match the code (`account.rs:1674`, `:1696`). `clean_unfinished` runs before the session credential is deleted (`:767`, `:774`).
- **Frozen references.** Untouched.
- **Single-user assumptions.** None found.
- **Test order.** F1, including test 15, is written before F2-F8.
