# L4 plan review, round 2 (2026-10-19)

Plan: `docs/plans/2026-10-19-l4-honest-page-plan.md`, checked against the email branch. Reviewer, plan
mode. Filed by the controller from the reviewer's returned text.

**Verdict: FAIL.** 0 critical, 2 important, 6 minor.
- **Closed:** C1, I3, I4 and M1-M5.
- **Partly closed:** I2. Its time bound is wrong, and the test meant to prove it cannot work as written.

Every fix below is a text edit to the plan.

## Closed

- **C1, the queue.** No deadlock. L8's S5 waits only for L6 (L8 spec :113-119, :540-550, :1088). T13
  waits for L8's merge, which waits only on L8's own gates. Prod `ROUTE_COUNTS` stays off until the
  sentences are live (S7, :1091).
- **I3.** No committed tree is red. T7's pin goes green in T8's commit with the key `2026-09-24`, and T13
  moves the key and the constant together.
- **I4, M1, M3, M4, M5.** Closed. Mail's skip name is exact (scheduler.rs:549); `mail.pass?` (:510) keeps
  "no row" when there is no pass.
- **`account.rs`.** Its tasks go to contract-engineer and contract-reviewer.

## Important

**I-1. The clean's time bound leaves out a token refresh.** Plan :394-397; account.rs:40, :159-161,
:496-503.
- `for_session` calls `valid_access_token_at`, which takes `SESSION_REFRESH_LOCK` and refreshes near
  expiry over the account client's 30 s timeout.
- The real worst case is 60 s (`MAIL_STOP_BOUND`) + ≤30 s (the lock) + ≤30 s (the refresh) + 5 s ×
  (1 + records) + ≤30 s (the revoke).
- **Fix:** state that bound in §4.7 and in `SIGN_OUT_MAIL_CALL`'s doc line, and have T4R check it. Or
  specify a token function whose refresh is also capped by `SIGN_OUT_MAIL_CALL`: a new `account.rs`
  helper, contract-engineer's.

**I-2. T4's bound test collides with the revoke and the file's tripwire.** Plan :752-756;
tests/account.rs:286-293, :954-965.
- The unexpired session makes step 7 revoke against `api_base()`.
- If `KNOWLU_API_BASE` points at the silent listener, the revoke hangs for 30 s.
- **Fix:** the builder takes `(api_base, anon, target, timeout)`. The test sets `ApiBase` to
  `closed_loopback_base()`, so the revoke is refused at once, and passes the silent listener's base to the
  builder directly. The order test needs the same `ApiBase` line.

## Minor

**M-1. The T6 rig does read Credential Manager.** Plan :806-815, §4.4 :278-280.
- The session step (scheduler.rs:884-905) and `refresh_privacy` (like `refresh_entitlement`,
  account.rs:865-868) read `cloud.yaml`'s `session_credential_target`.
- These are reads of a never-written target, the exemption recorded at tests/scheduler.rs:630-634.
- **Fix:** say so. Write a generated target into `cloud.yaml`, and have the listener count every request,
  not only `GET /account/consent`.

**M-2. T6's expected rows leave out the session row.** Plan :817-828. The
`session (refresh failed: …)` row (scheduler.rs:901) comes first.
- **Fix:** name the session row, and say "exactly one privacy row".

**M-3. The grades skip has no stated mechanism.** §4.5 :304-306.
- `grades_step` maps errors only through `CaptureError::as_str` (scheduler.rs:455-459), and `SignedOut`
  sets `set_signed_out(true)`.
- **Fix:** a stand-in closure records "would have captured" and returns an error with no side effect,
  never `SignedOut`. `run_slot_with` then replaces the row.

**M-4. No owner for a conflict in `app/tests/static_assets.rs`.** §6 :560, §7 :591, T13 :962-964.
- L8's S5t appends tests to the same file as T7's pins (spec :1087).
- **Fix:** name who keeps both sides. Add S5t to T7's dependencies, or drop "ahead of T7".

**M-5. The rebased tree isn't checked before T13.** :965, T8 :892.
- **Fix:** with the patches reversed, run the workspace gate, both console checks and the panel
  screenshot on the rebased tree, then re-apply the patches.

**M-6. Step 1 may get no hold.** §4.7 :373-374. `close_mail_window` returns None on a timeout
(account.rs:1518-1519) or for another profile (:650-652).
- **Fix:** state that steps 2-5 still run.

## Note outside this plan

The L7 and L5 queue edits listed at §11 :1177-1194 are not applied yet. Without them, L7 waits for L4's
merge, which is slower but not a deadlock.
