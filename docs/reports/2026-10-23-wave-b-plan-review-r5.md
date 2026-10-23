# Wave B plan review, round 5 (2026-10-23)

Reviewer (spec/plan mode) on `docs/plans/2026-10-23-wave-b-long-onboarding-plan.md`, o1 worktree. Filed by the controller.

**Verdict: FAIL.** 0 critical, 5 important, 5 minor. The plan's text closes all of round 4's findings (I-1 to I-3, M1-M6). But I-1's fix depends on a re-sign-in path that does not exist, and three other holes are in code this plan reuses.

## How the r4 findings stand
- **Closed in the text:** I-1 (§4.10's table, F2b, F8f's freeze, F1 tests 17 and 19), I-2 (§4.3:639-645, F1 test 16), I-3 (F8s, F1 test 18) and M1-M6. The ledger has rows for all of them (§10).
- **Holding:**
  - The exit order is sound. Each relaunch spawns after the drain and the sign-out, and the new process waits for our pid for up to 10 s.
  - F4b's `switch_profile` refusal is a mode guard, not a computation, so `commands.rs` still computes nothing.
  - F2c stays on the contract list, with `CREDMAN_LOCK` and F2e first.
  - The vault's owner is knowable on the device: `cloud_config(vault).account_id` against the pending session's account id (`account.rs:128`, `:855`).

## Important

**I-1. Payment and the entitlement hold read a session that step 2 has already moved** (§4.2 step 4, §4.3:649-652, F3 test 1).
- `entitlement_now` and `open_checkout` read only `PENDING_TARGET` (`app/src/account.rs:1191-1199`, `:1611-1615`).
- F3 keeps today's `move_session` into the profile. That copies the session and then deletes the source (`account.rs:154-156`).
- Scenario: in onboarding mode, step 4's checkout returns "no session at knowlu/pending/session". The queue's hold never reads "entitled", so every held job stays `held` and nobody gets past payment.
- Fix: add profile-target versions outside `account.rs`, built from the public `cloud_config`, `valid_access_token_at`, `fetch_entitlement_at` and `checkout_url_at`. Add them to F5, F8a's hold and F11e, with a test on a `127.0.0.1:0` listener.

**I-2. "Step 2 again" has no Rust task, and the path T0 is told to name does not exist** (§4.2:516-522, T0:1160, F11e).
- Today's sign-in commands save only to `PENDING_TARGET` (`account.rs:614-617`).
- The only attach paths are `create_vault` (`onboarding.rs:740`) and `attach_in`. `attach_in` refuses any vault that has a `cloud.yaml` (`scaffold.rs:692-696`).
- F11e is console-ui (static files only), and F2a only derives the resume step. So no task builds F1 test 10's "another account is refused".
- `move_session` overwrites the target unconditionally (`account.rs:139-157`), so reusing it naively attaches the wrong account to the vault.
- Fix:
  - Add an implementer task for `onboarding_attach_session`. It compares the account ids and moves the session on a match.
  - On a mismatch it deletes the pending session (`sign_out_with(None, …)`) and refuses by name; otherwise another account's token is left on the machine.
  - Register it in F5. Change T0's check to "confirm none exists".

**I-3. `slot.partial` gets no grades capture: the scope sits one layer too low** (F8s:1679, F8g).
- Only `run_slot_inner` builds the `GradesSeam` and `MailSeam`, and only from `Some(AppHandle)` (`scheduler.rs:731-762`). With `None`, both the capture and the mail pass are `None`.
- The plan says there is no tray in onboarding mode, and never says which handle to pass.
- Scenario: F8g calls `run_slot_scoped(…, NoRank)` with empty seams, or calls `run_slot_inner(cs, sch, None, …)` for `slot.first_full`.
  - `grades` is always a named skip, so the gate's "first grades" never resolves.
  - The full slot runs no mail pass.
- Fix:
  - Put the scope on `run_slot_inner`, for example `run_slot_inner_scoped(cs, sch, Some(app), late, scope)`, and pass onboarding's `AppHandle`.
  - This is safe: `tray::set_state` does nothing when there is no tray (`tray.rs:150-151`).
  - F1 tests 15 and 18 stay at the seam layer.

**I-4. A closed mail-backfill batch starts a full slot outside the queue** (F4 keeps `mail::live::on_start`; F8d).
- `real_env`'s `on_closed` calls `scheduler::run_slot(&app, false)` (`mail/live.rs:115-121`).
- Scenario: forwarding is verified at step 5. The 120-day backfill's first batch closes while the student is at step 7, and a full slot runs.
  - Its `rank` runs before `config.ingest` and `slot.first_full`. It writes `state/today.md`, so `ingest::is_first_run` is false and O1's first-sight `imported-done` is lost.
  - Its `sync` takes `sync.lock` outside the notes gate.
  - Its engine child is not one of the drain's safe points.
  - If that slot holds `sch.running` when the queue starts `slot.partial`, `run_slot_with` returns the stale `sch.last` (`scheduler.rs:783-786`).
- Fix: in onboarding mode, `on_closed` notifies the queue (or does nothing, and `slot.first_full` pulls the batch). Add an F1 case.

**I-5. The window's close would run the drain on the event loop** (F4:1561, `main.rs:187-191`).
- `on_window_event` runs on the main thread. A synchronous `onboarding_quit` inner blocks it for as long as the drain takes, up to an engine child's `CHILD_TIMEOUT` of 20 minutes (`scheduler.rs:28`).
- The stages the drain waits for (grades capture, mail pass, school reads) need the event loop to open and close their windows. The likely result is a hang until each stage's own limit, or a deadlock, and "finishing <job>…" never appears on screen.
- Fix: call `api.prevent_close()`, then run the inner on a spawned thread.

## Minor
- **M1. Where the freeze lives (F8f, F2e).** If the freeze sits in F2a's file save, F2e's rewind is refused too. Put it on the queue's own saver. Test 17 would catch this, but the brief should say it.
- **M2. "Sync ran" (§4.3, F8e).**
  - `at` and `RunSummary.started` are both `now_ts`, UTC to the millisecond (`journal.rs:61-66`, `scheduler.rs:795`). Name `started` as the comparison point.
  - A lock-held skip writes no status at all (`sync.rs:2524-2528`), so only the stale-stamp rule catches it.
  - A fresh run with `ok: false` currently counts as "first sync done". Make it `failed`, with its fix.
- **M3. Runner log for the `rank` skip.** The runner-log skip list (`scheduler.rs:1036-1037`) does not include `rank (skipped:`. Say whether the new line joins it, so test 18 and the build agree.
- **M4. Call-site count.** `run_slot_inner` and `run_slot_with` have about 30 call sites, not eight; about ten are in `app/tests/scheduler.rs`. The wrappers still leave every one unchanged.
- **M5. F2c's tests could leave the machine.** `sign_out_clean(dir, id)` uses the built-in `api_base()` (`account.rs:719-722`). With a session credential present, the mail clean would call the real service. Require that no test has a session credential, or add a base parameter.

## Sizing and routing
Fine. Every task is about 80 lines or less, labels are on all of them, F1 comes before F2-F8, and F2c is on the contract list behind F2e and S5. I-2's new task would be about 30 lines, for an implementer, depending on F2a.
