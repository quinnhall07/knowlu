# Onboarding replan draft, review r3 (2026-11-04)

**FAIL at 8f45e60:** 0 Critical, 3 Important, 10 Minor; all text fixes. All 13 r2 findings closed, one residual on M5 (Minor 7). I2's route through `emit_checks_locked` is confirmed: cards filtered, asks not, on every production path.

## Important

1. **`CredError::NotFound` does not mean signed out** (§6.12 714-716, C5d 984-985). `wincred.rs:158-163` returns `NotFound { code }` for every `CredReadW` failure. Code 1312 for a signed-in student at step 10 runs `All` plus the bundle delete; `enqueue_once` never re-runs the jobs. Fix: signed out only on `NotFound { code: 1168, .. }`, a three-outcome seam, and a C5d case for code 1312.
2. **The sweep checks a different credential from the app's** (§6.12 714). The app reads the target in `config/cloud.yaml` (`account.rs:863`, `:997`), written once (`scaffold.rs:701`); registry ids derive from the path (`profiles.rs:24`, `:69`), and `retarget_credentials` moves only `zybooks` and `vhl` (`onboarding.rs:1194`). A moved plan folder looks signed out on every launch, and the no-time block goes. Fix: read `session_credential_target` from the vault's `cloud.yaml`, `session_target(id)` only with no such file; test with a moved vault.
3. **§4.7's completeness claim is false: F4c is missing** (282-285, 247). F4c's dependencies are F4, F5, F2b2b, F8w2 (plan 1552) and `main.rs` holds neither `Queue::start` nor `load_state_for_launch`. T3b would read as ready with no queue started. Fix: F4c in layer 11 after F8w2, T3b depends on it, a `main.rs` row in §4.2 for F4c and C5d.

## Minor

1. No sweep row for `Loaded::Unreadable` (`onboarding_state.rs:195`). Signed in and unreadable should run nothing; add a C5d case.
2. `Settings` has no vault field (`state.rs`): use `profiles::load`. `change_school` (`onboarding_jobs.rs:1035`) takes no vault, and C5d's row omits that file.
3. `lock_series_file` blocks without a bound and creates `state/` (`commitments.rs:2214-2216`), before the single-instance plugin runs. Read first, lock only if dirty, `try_lock` in the sweep.
4. `series_for_backup`: exact bytes hold only if the raw bytes are returned; `parse_file` (`:2121`) drops unknown keys. C2e's test needs a non-canonical file.
5. `same_bytes` (`backup.rs:43`) compares source to destination, so a filtered file recopies every tick. Compare filtered bytes; add a second-tick test.
6. `write_state_file` leaves `calendar-series.json.tmp<pid>` (`commitments.rs:2233`) and `TMP_MARK` is `.tmp-`, so the mirror can copy and archive the unfiltered file. Filter by name prefix.
7. `:5156` also prints the key; §6.12 and C2d name four warnings.
8. `KeepAlive::PerCourse`: C2a's "a row past its `until` goes" holds only on the `Some(_)` branch. Harmless; fix the test wording.
9. A3 and B3 sit in layer 9 but live inside F11e1 and F11b, and F11e1 needs F8lb (layer 10). Move them to layer 12.
10. "While a row exists it covers its course" (576) fails for a lab row (`:892`, `:3631` count `class` only). "So no ask fires" (442) holds only for new asks: `ask_withdrawal_reason` (`:3623`) has no no-time input, and fixing that edits `approvals.rs`. State both as accepted, or rule.

## Held up

§4.7's other edges, no cycle; the session is never deleted by a failed refresh; sign-out rewinds the jobs; `create_marker_as` has no bypass; a refused Finish purges nothing; §4's counts; no contract-list test edits for the new `SeriesFile` field.
