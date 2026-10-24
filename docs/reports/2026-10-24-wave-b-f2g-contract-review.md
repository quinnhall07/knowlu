# Wave B F2g: contract review (2026-10-24)

The contract-reviewer reviewed this task because a wrong move attaches another account's session to the vault. The controller filed this report.

## r1 (`3bb46bc..262411f`): FAIL
**I-1. The session moved was not the one checked.** `move_session` read the pending target a second time. Here is the failure:
- The student starts a Google sign-in, then signs in with an emailed code.
- The forgotten Google tab finishes as account Y between the id check and the move.
- Y's session JWT lands on X's profile.

Fix: save exactly the session that was checked, then forget the pending one.

**Minor findings:**
- **m-2.** The forget skipped the delete when the credential lookup failed.
- **m-3.** A `cloud.yaml` target equal to the pending target answered `ok:true`.
- **m-4.** `write_finish`'s answers delete did not take `ANSWERS_LOCK`.
- **m-5.** The empty-target and failed-move claims had no tests.

**Held up:**
- Three agreeing ids are never empty.
- No sentence names an account or an email.
- There is no revoke.
- There is no lock-order deadlock.
- The Credential Manager tests hold `CREDMAN_LOCK` with generated targets.
- No contract-list file was touched.

## r2 (`262411f..62ad97f`): FAIL, narrow
- **I-1 and m-2 to m-5 are closed.** A swap between the load and the save is now a regression test.
- **Important.** The test 21 source pin no longer read the logic, which moved into `attach_session_with`.
- **Minor.** The pending-target compare was case-sensitive, while Credential Manager target names are not.

## Controller fix
- `attach_session_with` added to the pin's region.
- `eq_ignore_ascii_case` used for the compare.

`onboarding_state`: 56 pass and 16 fail. The 16 are F8b and F8lb in tests 01 and 12, and F2h in test 22. All nine test 21 tests pass.
