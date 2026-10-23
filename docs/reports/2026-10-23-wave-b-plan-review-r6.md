# Wave B plan review, round 6 (2026-10-23)

Reviewer (spec/plan mode) on `docs/plans/2026-10-23-wave-b-long-onboarding-plan.md`, o1 worktree. Filed by the controller.

**Verdict: FAIL.** 0 critical, 2 important, 4 minor. All ten round-5 findings are closed in the text. The first Important finding comes from round 5's own suggested fix for I-2 (`sign_out_with(None, …)`); the second is a reuse the audit has no row for. Both fixes are a few lines each.

## Round-5 findings
- **Closed:** I-1 (F2h, and F8a's hold on `entitlement_state`), I-2 (F2g), I-3 (`run_slot_inner_scoped` with `Some(&app)`), I-4 (F8m), I-5 (F4's `prevent_close` and a thread, with F2b's `AtomicBool`), and M1-M5. I checked each fix against the code; they hold.
  - `refresh_entitlement` saves the same cache that `entitlement_state` reads (`account.rs:994-1000`, `scheduler.rs:291-298`).
  - `open_portal`'s shape is as described (`account.rs:1626-1636`).

## §13.1 audit spot-check: 26 rows, 26 verdicts confirmed
- **Rows checked:** `entitlement_now`, `open_checkout`, `refresh_entitlement`, `entitlement_state`, `checkout_url_at`, `open_portal`, `cloud_config` (the host check at `:870`), `move_session` (overwrites), `load_session`, `sign_out_with`, `sign_out`, `sign_out_mail_client`, `valid_access_token_at`, `run_slot_inner` (`:731-762`), `run_slot_with` (`:775-786`), `slot_argv` (`:529`, `:538`), the runner-log filter (`:1037`), `RunSummary.started` (`:793`), `real_env` and `on_closed` (`live.rs:111-121`), `start_setup`, `on_start`, the main window's close (`main.rs:187-191`), `quit_flush` (push and backup, `state.rs:330-356`), managed `Scheduler`, `Updates` and `LmsSession`, and `google_connect_url`/`connected` on the pending target (`:2120`, `:2143`).
- **Hit rate:** 100% on the verdicts. One row is incomplete, which is I-1 below.

## Important

**I-1. F2g's mismatch path signs the other account out everywhere** (plan F2g, lines 1693-1699; the `sign_out_with` row at line 2191).
- `sign_out_with` posts `{auth}/logout` with no `scope` (`account.rs:776-781`). As far as I know, GoTrue treats a missing scope as `global`, which revokes every refresh token the account holds. Worth a quick confirm, though I'm fairly confident.
- **Scenario:** a shared PC. Profile 1 is a sibling's finished install on account A. Profile 2's setup belongs to account B. At step 2 again, the browser's Google session signs in as A.
  - F2g refuses correctly, then `forget_pending` revokes A globally.
  - Profile 1, and A's other desktops, fail their next session refresh. Sync and judge stop until A signs in again.
  - The same happens on "an unreadable cloud.yaml": the student's own correct account is revoked on every device.
- F1 test 21 uses a recording fake, so nothing catches this.
- **Fix:** the forget is a local delete only, `credentials::delete(account::PENDING_TARGET)`, with no revoke. This needs no `account.rs` edit, and the token still leaves the machine. Update the row at line 2191, §14.29 and test 21.

**I-2. Steps 3 and 7 store a feed link under the pending session, which the audit missed** (F5, lines 1766-1767; §4.11).
- F5 registers `capture_calendar_link` and `paste_calendar_link`. Both end in `lms_link::finish`, which calls `store_source(&api_base(), PENDING_TARGET, …)` (`lms_link.rs:843-847`, `:883-884`).
- By step 3, step 2 has already moved the session off the pending target.
  - **Usual case:** the store fails, and the plan's pending credential and `school.store` carry the link. That works, but it breaks §4.11's rule that the link is sent only by `school.store`.
  - **Bad case:** `knowlu/pending/session` is machine-wide. If another process has a sign-in parked there (a second launch's picker making a new profile for account B), and B is entitled, the student's LMS bearer link is PUT into B's `sources`. B's `ingest` then reads the student's classes.
- Neither T0 nor any test sees this, because the audit has no row for `finish`/`store_source`.
- **Fix:** in onboarding mode, these two commands validate only. Either call `finish_with` with a store seam that does nothing (`lms_link.rs:858`), or use a variant that reads `cloud.yaml`'s target. Add the audit row, and an F11t or F1 case that onboarding's paste never calls the store.

## Minor
- **M1. A `(not yet: …)` row at exit 0 shows as "done".** `console.js:1542` classifies a row as skipped only on `(skipped:`.
  - The naming does honour "left out and named, never run-and-failed".
  - It leaves the run-record contract alone: these are app step rows, not engine run records, and they write no runner-log line.
  - The risk is a screen that reuses `renderFirstRun` or the Runs view and shows `coursework` with a check mark. Name this reader in §13.1, and either have F11 pin that no onboarding screen renders slot rows that way, or teach the reader `(not yet:`.
- **M2. F2g and F2h break file-disjointness.** Both edit `onboarding_state.rs` and depend only on F2a and F1 (§7, lines 1186-1187), so they can run alongside F2d, F2e and F2b in the same file. Chain them after F2b, or give them their own module.
- **M3. A backfill batch can close after `slot.first_full` has run.** With no queued slot left to run `judge`, F8m's "the next queued slot pulls it" waits until the first scheduled slot after the handover. That is acceptable, but §4.3 and the rail text should say so.
- **M4. The audit tally needs the I-1 correction.** Mark the `sign_out_with` row's revoke as global, and add the `lms_link::finish`/`store_source` row. That makes 53 rows, 12 "No".

## New-risk check on the new tasks
- **F2h:** sound. Every function it uses is public, the clock is `jiff`, and the listener rides the `KNOWLU_API_BASE` guard.
- **F2g's agreement path:** sound. `move_session` onto `cloud.yaml`'s target overwrites only the same account's older session.
- **BeforeConfig:** sound. It keeps `sync`, `grades` (a named skip before `campus.yaml`), `mail`, the entitlement and privacy checks, backup and telemetry. It leaves out the four steps that need `ingest.yaml`, `rank` among them, so `is_first_run` survives.
- **The exits:** sound. Quit and Finish take `quit_flush` after the drain, so its push under `vault_io` meets no lane write. Sign out skips the flush, which is consistent with F2e's rewind.

## Invariants, sizing, routing
- **Invariants:** no single-user assumption, and frozen references and byte formats are untouched. `commands.rs` gains only F4b's mode guard. Every vault write still goes through the engine.
- **Routing:** the contract-list edits are F2c only (`account.rs`, contract-engineer then contract-reviewer, with `CREDMAN_LOCK`). F2g and F2h are correctly off the list, with contract-reviewer review.
- **Sizing:** every task is about 80 lines or less. F2b has its F2f overflow.
- **Order and ledger:** TDD order holds (F1 before F2-F8, F11t before F11a-e). Ledger rows exist for every round-5 item.
