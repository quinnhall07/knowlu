# Wave B F8s2 reviews (2026-10-24)

The reviewer checked each round. The controller filed this report.

## r1 (`3bb46bc..bc4d52a`): FAIL
**Rulings.**
- The rewritten `stop_tests` assertion is a legitimate change, because the spec reversed the old rule. The new test is stricter.
- A stopped slot needs no exclusion. Only onboarding's `stop_both` sets the stop, and an unfinished file ignores `update_finished`.
- A run with no app handle applies no signals, which is sound.

**Important.** `mail_rows` turned a client it could not build into `Ok(vec![])`. That covers a `cloud.yaml` that is missing, unparseable or names another host. The empty list then read as Disconnected and cleared all three mail items for good.

**Minor.** `pending_setups` was read after the list call.

## r2 (`bc4d52a..ff1ad7e`): PASS
- `Err(MAIL_NO_ACCOUNT)` leads to no signal. The normal slot is unchanged: still `None`, idle, no row and no amber.
- `pending_before` is read before the list and re-read after it.
- `MAIL_JSON_LOCK` and `ONBOARDING_JSON_LOCK` are never held together.

**Minor.** A comment could satisfy the source pin. Controller fix: comment lines are stripped, and the pin matches the arm `Some(Err(_)) => Err(MAIL_NO_ACCOUNT)`. Test 32: 18 of 18 pass.

**Known limitation (n1).** Hand-over items carry no address id, so one verified row clears every address's items.
