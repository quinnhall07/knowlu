# L4 plan review, round 3 (2026-10-19)

Plan: `docs/plans/2026-10-19-l4-honest-page-plan.md` (r3). Reviewer, plan mode, with the code checked against
the email branch. Filed by the controller from the reviewer's returned text. Earlier rounds:
`2026-10-19-l4-plan-review-r1.md` and `-r2.md`.

**Verdict: FAIL.** 0 critical, 1 important, 4 minor. Every fix is a text edit to the plan, and none reopens a
design choice. All round-2 findings are closed. The one gap: the I-1 design's lock-wait bound has no test
that could fail.

## Round-2 findings: closed

- **I-1 (in design):** §4.7 :402-461, T4 :806-858, T4R :876-882. T4 is `contract-engineer` (xhigh) and T4R is
  `contract-reviewer`; §6 :621 agrees. The line references check out on the email branch.
- **I-2:** T4 :821-843.
  - `sign_out_mail_client` takes its base as a parameter and never calls `api_base()`, so the tripwire
    (tests/account.rs:992-1012) does not flag it.
  - Every `sign_out_with` test sets `ApiBase`, so the revoke is refused at once.
  - One listener serves both `/functions/v1` and `/auth/v1`.
- **M-1 to M-6:** closed.
  - `Busy` has no side effect in `grades_step`; only `SignedOut` sets the mark (scheduler.rs:455-459).
  - `WindowOpen` returns before the capture (:440-441).

## The I-1 design on the email branch

- **(a) "No other caller waits longer than today" is too strong.** std's Mutex on Windows is an unfair
  SRWLock: `try_lock` can win ahead of a blocked `.lock()`. A waiter can then wait up to about 5 s more, once
  per sign-out, but never a refresh's 30 s more. No behaviour or bytes change. (m-1)
- **(b) The rotation hazard holds.** The clean is step 2; steps 3-5 are local; step 6 reads, then deletes.
  - Another thread may reuse the old refresh token in that window. GoTrue then either answers within its
    reuse interval, or revokes the session family. The student is signing out either way.
  - "No later caller reuses the old token" is overstated. (m-1)
- **(c) `try_lock` is sound if `Poisoned(p)` means "acquired" (via `into_inner()`).** Read as "held", every
  sign-out after a panic would time out. :426 is ambiguous. (m-1)
- **(d) The worst case 60 + 5(2+n) + 30 s is right for network waits.** It leaves out a local term:
  `wipe`'s 11 × 250 ms (mail_session.rs:31-32, :301-311). (m-2)
- **(e) The tests:** the refresh half is proved; the lock-wait half is not (I-1). Credential Manager writes
  are all under `CREDMAN_LOCK`.

## Important

**I-1. The lock-wait cap has no test that can fail** (T4 :848-857, ledger :1282).
- None of the helper's tests holds `SESSION_REFRESH_LOCK`. The only lock check is a source pin ("calls
  `try_lock`, never `.lock(`").
- A loop like `while try_lock().is_err() { sleep(25ms) }`, with no deadline, passes every listed test. The
  bound would silently fall back to today's.
- **Fix:** add `valid_access_token_within_gives_up_on_a_held_lock`, under `CREDMAN_LOCK`:
  - Thread A runs `valid_access_token_at` on an expiring generated session against a silent listener; the
    first accepted connection shows A holds the lock.
  - The helper is then called with `within = 200 ms` on a second generated target, against a counting
    listener. It must answer `Err` within 1.2 s, with zero requests and the stored session unchanged.
  - Finally, drop A's stream so A ends.
  - Add the test to T4R's first bound check and to the :1282 row.

## Minor

- **m-1. §4.7 wording** (:426, :442-447).
  - Say that `Poisoned` is the lock, taken via `into_inner()`.
  - Replace "no other caller waits longer than today" with: each new hold is at most 5 s plus local
    Credential Manager work, once per sign-out; a waiter may wait that much more, never a refresh's 30 s
    more.
  - Replace "no later caller reuses the old token" with the reuse-interval reasoning in (b).
- **m-2. The bound leaves out `wipe`** (:460-461).
  - Name the local term: up to about 2.75 s per mail profile still held, in the clean's `forget` and in
    `forget_all`.
  - Fix the 401 arithmetic (:463): it is n ≥ 24, not 23, and D3's caps (three live addresses, six mints per
    30 days) mean that n cannot occur. Say so.
- **m-3. The once-per-clean memo is unpinned** (:409-411, T4 :807).
  - `TokenFn` is `Fn + Send + Sync` (mail_client.rs:151), so the memo must be a `std::sync::OnceLock`.
    Without it the bound becomes 5 + 10(1+n) under contention.
  - Add a source pin.
  - Explain in the doc why it is kept for one clean, against mail_client.rs's "never kept" module doc
    (:7-9).
  - Spell the test base `http://127.0.0.1:<port>/functions/v1`.
- **m-4. Stale fact** (:379-380). The new client reads the session once, at the clean's first request, not on
  every request. The order argument stands.

## Collisions

Neither the in-flight plans nor C3's plan touch `account.rs`, `mail_client.rs` or `clean_unfinished`. The r3
edits broke nothing elsewhere. T0's new checks match the email branch.
