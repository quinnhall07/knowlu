# Wave B O3 F3 (step 2: the vault with defaults): review r1 (2026-10-24)

The reviewer checked `git diff bd024fd..e5e6568` (tests 5ede3b4, code e5e6568). The controller filed this report.
- Nothing on the contract list is edited; only public `account`/`credentials` functions are called.
- No token reaches an error or a log.
- `create_vault_in` is unchanged.
- The credential tests reuse `PendingSession` correctly, and the test file is unchanged between the tests and the code.

**Verdict: FAIL.**

## Important
1. **The name picker checks names, not profile ids** (`onboarding.rs:825-828`). Example: the student renames profile "Knowlu" and its folder is later moved. The picker returns "Knowlu", and `id_for(dest)` hands back the old id. Then:
   - `move_session` overwrites that profile's session.
   - Its portal logins carry over to the new vault.
   - Its `onboarding.json` and `settings.json` are overwritten.
   - A later failure's undo deletes a file this call did not create.

   **Fix:** also skip a candidate whose `id_for(parent.join(name))` is a registered id, or whose `profile_dir` exists. Add a test with a renamed profile and no folder, which must give "Knowlu 2".
2. **The failure undo is untested** (`:872-876`). The controller promotes this one: a regression would strand a refresh token at the target of a deleted vault. **Fix:** tests that drive a restore `Err`, a `save_state` failure and a `finish` failure. Each must leave the vault gone, the pending session back, and nothing at `knowlu/<id>/session`.

## Minor (fold in)
3. **The undo ignores two results** (`:873-875`). If `move_session` back or `remove_dir_all` fails, name that failure in the error.
4. **`write_atomic` leaves an empty profile folder** after an early failure. The undo should remove the folder if this call created it.
5. **`suPlan()` is dead code**, yet `static_assets.rs:3087` still pins it. Delete both, or point the assertion at the Rust defaults (`onboarding.rs:855-857`).
6. **Follow-up only, not F3:** the `America/Chicago` fallback applies when Windows reports no IANA zone (the same rule as `launch_state:155`).

## Rulings
All six are upheld, and ruling 3 (onboarding.json before `register`) is safer than the plan's "last". Ruling 4's cleanup deletes only this call's own vault folder. The exception is Important 1's case.

## Review r2 (b501448): PASS — both Importants and Minors 3-5 closed; picker can't remove a pre-existing profile folder; the seam's only production caller passes the real restore_into. New minor (else-branch onboarding.json removal without ownership) folded in at HASH.
