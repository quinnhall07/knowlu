# L4 plan review, round 4, delta (2026-10-19)

Plan: `docs/plans/2026-10-19-l4-honest-page-plan.md`, r4. Reviewer, plan mode, delta against `…-r3.md`, with the
code checked against the email branch. Filed by the controller from the reviewer's returned text.

**Verdict: PASS.** 0 critical, 0 important, 2 minor, 2 nits. All five round-3 findings are closed as
specified. Nothing else broke. `account.rs` stays `contract-engineer`'s and `contract-reviewer`'s (§6 :644,
§7 :677-679, T4R :908).

## Round-3 findings: closed

- **I-1, the held-lock test.**
  - **It can fail.** A helper with no deadline, or one that calls `.lock()`, blocks behind thread A until
    A's 30 s `TIMEOUT` runs out (`account.rs:40`, `:161`). It then answers `Ok` at about 30 s, which fails
    both the `Err` check and the 1.2 s check. A helper that never takes the lock answers `Ok` at once and
    fails `Err`.
  - **The signal is reliable.** `valid_access_token_at` holds `SESSION_REFRESH_LOCK` from `load_session`
    through the network call (`account.rs:496-503`, comment `:488-490`).
  - **The 1.2 s bound is robust.** A held lock means about 200 ms of polling and nothing else.
- **m-1.** `Poisoned(p)` means acquired. The unfair-SRWLock bound reads "about 5 s, never 30 s". The reuse
  interval is argued.
- **m-2.**
  - The `wipe` term matches `mail_session.rs:31-32` and `:301-311` (about 2.75 s).
  - n ≥ 24 is right.
- **m-3.**
  - `OnceLock<Result<String, String>>` fits `TokenFn` (`mail_client.rs:151`).
  - `Cell` and `RefCell` are `!Sync`, so neither compiles.
  - The pin is checkable.
- **m-4.** "Once, at the clean's first request." The order argument stands.

## Minor

- **m-1. The held-lock test has no lower bound** (:884-887). A helper that answers `Err` at the first
  `WouldBlock` passes, yet breaks §4.7 :432's retry.
  - Fix: assert the elapsed time is at least `within`.
- **m-2. Two missing rig details** (:882-888).
  - (a) Wait for A's connection against a deadline, like `loopback`'s 10 s (tests/account.rs:19-28).
  - (b) Keep the accepted stream as a local inside the `thread::scope` closure until the helper returns.
    A failed assert then drops it while unwinding.
  - Time only the helper's call, and mark the test `#[cfg(windows)]`.

## Nits

- **:415 and :1412.** With no memo, the clean costs 10 × (1 + n) s, not 5 + 10 × (1 + n) s.
- **:486.** Deletes go only to rows the service lists as `pending` (`mail/setup.rs:415`), which are live
  addresses, at most three. So the clean makes at most 1 + 3 requests.
