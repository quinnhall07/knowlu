# Wave B O3 F2a: pre-push review, r2 (2026-10-24)

Reviewer, on `git diff be1f10c..27ece1e` (`feat/o3-f2a`: `app/src/onboarding_state.rs`, `app/tests/onboarding_state.rs`, `app/tests/onboarding_jobs.rs`). Filed by the controller. Nothing on the contract list is touched. LF, and `diff --check` is clean.

**Verdict: PASS.** No Critical or Important findings.

## Round 1, closed
- **I-1:** a session present lifts a file step of 2 or below to 3 (`:472-474`). test_09's sign-out arm now uses `SignedOut`.
- **I-2:** `earliest_unproven(c, has_school)` stops at 3 with no school, and an unreadable file passes `false`. This is §14.49 (a).
- **M1:** temp file removed when the write fails.
- **M2:** rename retried after 25 ms, then 200 ms, as in `PRIVACY_RENAME_PAUSES`.
- **M3:** `STATE_VERSION` added.
- **M4:** `skip_step` matches on `(step, source)`, and the cost keys are `pub const`s.

## Test edits, line by line
- **test_09:** only `Proven` → `SignedOut`.
- **test_02's ladder:** changed exactly as §14.49 lists. The extra `Ask` on `at_least_signed_in` is allowed.
- **test_02's tail:** authorised by §14.49, plus one assertion that an unconfirmed school moves nothing.
- No other assertion changed or weakened.

## `write_atomic`
Every failure path is clean. Only the rename is retried.

## `skip_step` against §4.2
The table matches §4.2: 5 `mail`, 6 `google`, 7 `calendar`, 8 `zybooks`/`vhl`. Steps 2, 3, 4 and 10 still refuse.

## `advance_after_school`
- The plan supports it (§14.49, and F2a's brief at `:1983-1984`).
- It is safe when called at the wrong step, and a second call does nothing.
- Its shape is right: it mutates the state and the caller saves.

## Minor
- **N1.** The doc says each step comes back "with its proof", but it returns step numbers only. Reword the doc, or name the mapping in F11's brief.
- **N2.** It does not check `answers_ok`. F2b's brief should say that the confirm saves the answers before it advances.
- **N3.** Two tests were added to F1's file: `test_07_a_skip_names_a_source_that_belongs_to_its_step` and `test_01_an_unknown_version_is_unreadable_and_a_session_lifts_step_2_to_3`. They add coverage only; record them in F1's ledger rows.
- **N4.** r1 M5 (an unreadable file is never saved back) stays assigned to F8a/F4.
