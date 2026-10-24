# Wave B O3 F2a: pre-push review, r1 (2026-10-24)

Reviewer, on `git diff fa232ec..be1f10c` (`feat/o3-f2a`; only `app/src/onboarding_state.rs`). Filed by the controller. Nothing on the contract list; LF; `diff --check` is clean.

**Verdict: FAIL** (one Important finding, I-1).

## Important
**I-1. A rewound step-2 file with a session present resumes at step 2; the plan says step 3** (`onboarding_state.rs:442-446`).
- §4.2 step 2 and §4.5 say that after re-sign-in, resume lands on step 3. F1's `test_02_a_file_rewound_by_sign_out…` (`app/tests/onboarding_state.rs:312`) expects step 3.
- The root cause is in F1: `test_09_a_canary_that_never_notices_the_stop…`'s sign-out arm (`app/tests/onboarding_jobs.rs:727-728`) resumes the same rewound file with `Proven` (session present) and expects step 2. No resume rule can satisfy both tests.
- **Controller's ruling:** test_09's sign-out arm uses a `Checks` with no session, because sign-out deletes the session. Its step-2 expectation then comes from the no-session rule. F2a: with a session present, a file step of 2 or below resumes at 3.

**I-2. The unreadable-file ladder treats step 3 as proven, but the school never is** (`earliest_unproven`, `:420`). A restored, entitled account with a corrupt `onboarding.json` resumes past step 3 with no school, so `school.apply` never runs. §4.7 says the school is never "proven". This goes to the planner.

## Checked, no finding
- **`write_atomic`:** a sibling temp file, then `fs::rename`; the pid plus a per-process counter; the temp is removed when the rename fails; JSON through `ledger::dumps_value`.
- **No generic helper to reuse:** each writer is file-specific, so a `pub(crate)` helper is right.
- **Paths come only from `data_dir`.**
- **Absent and Unreadable:** only `NotFound` is `Absent`.
- **`launch_mode`:** matches test 10.
- **Running jobs return as pending,** and the school's `confirmed` is reset.
- **The rulings:** steps 1, 9 and 11 refused; cost keys match F9's fixtures; the answers rule deferred to F2d; the counter is sound.
- **No vacuous passes;** other stubs untouched.

## Minor
- **M1.** If `fs::write` fails, the partial `.tmp` is left behind (`:129`).
- **M2.** There is no retry on a transient lock. `privacy.json`'s writer (`account.rs:1394`) retries; the sign-out rewind must land.
- **M3.** `load_state` never checks `version`.
- **M4.** `skip_step` doesn't check `source` against the step: `(8, "")` gives `homework_due_dates_missing_`. Cost-key strings are a contract for F9 and F11, so make them named constants.
- **M5.** For an unreadable file, `resume` writes nothing, though §4.5 says the file is rewritten. F4 or F8a must save it, or every launch derives a new `started_at`.
