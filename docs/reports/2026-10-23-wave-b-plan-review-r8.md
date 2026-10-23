# Wave B plan review, round 8 (confirm round, 2026-10-23)

Reviewer (spec/plan mode) on `docs/plans/2026-10-23-wave-b-long-onboarding-plan.md`, o1 worktree. Filed by the controller.

**Verdict: PASS with minors.** 0 critical, 0 important, 3 minor. All five r7 findings are closed. The edits created no new defect that would ship silently. M1 below is a contradiction inside the plan that test 26 would catch, so it is not Important. It should still be fixed before F8c is dispatched.

## r7 findings
- **I-1: closed.**
  - F8la (plan lines 1225 and 1987-1996) adds `capture_calendar_link_with`, `validate_only` and `capture_link_only` to `lms_link.rs`. It depends on F1 and on S4b being merged.
  - F8lb (lines 1226 and 1997-2002) depends on F8la, F8b, F8c, F8g, F8d and F2d.
  - F8b (lines 1220 and 1947-1950) depends on F8la, calls `lms_link::capture_link_only`, and is now reviewed by contract-reviewer.
  - Test 26 (lines 1653-1656) pins `onboarding_jobs.rs` and `onboarding_state.rs` against `store_source`, `lms_link::finish(`, `capture_calendar_link(` and `paste_calendar_link(`.
  - The ledger row at line 2170 names F8la, F8b and F8lb.
- **M1: closed.** F11e (line 1231) and F11b (line 1232) now depend on F8lb.
- **M2: closed.** F2b (line 1201) now depends on F2e, so the chain F2a, F2d, F2e, F2b, F2g, F2h is enforced.
- **M3: closed in part.** Line 2004 now says `lms_link::finish(`, but it also lists `PENDING_TARGET`, which test 26 does not. See M2 below.
- **M4: closed.** F2g (lines 1730-1733) ignores the not-found result and logs any other delete error by name, and the refusal still stands. Test 21's no-pending case gets the named refusal.

## The questions asked
- **Is F8la sized and routed correctly?**
  - Yes. Only the signature changes on the existing 38-line body (`lms_link.rs:397-435`): the `finish("lms_ics", &found)` call at `:427` becomes `finish(...)` through the parameter. The two new functions add a few lines each, so about 20 lines is realistic.
  - `lms_link.rs` is off the contract list. The routing is `implementer` plus contract-reviewer, and it runs after S4b, which also edits this file.
  - `_with` takes a `&dyn Fn`, so it cannot carry `#[tauri::command]`, which keeps it off every handler list.
- **Do the shell's commands keep their behaviour and tests?**
  - Yes. `capture_calendar_link` passes today's private `finish` (`:843-847`), and `paste_calendar_link` (`:883-885`) is not touched.
  - No test reads either function's body. The source pins at `tests/lms_link.rs:288`, `:501`, `:714` and `:797` read other strings or `capture_courses`.
  - `static_assets.rs:546`, the old wizard's `"capture_calendar_link"` call, is unaffected by F8la. It belongs to F10 and F5b, as before.
- **Does the §7 graph still have no cycles?**
  - None. F8la depends only on F1 and S4b. Nothing upstream of F8b depends on F8lb.
  - F8lb's dependents (F5, F11e, F11b, and F12 through them) have no edge back.
  - F4 depends on F2b, not on F5. F2b depends on F8f, which depends on F8a and F1.

## Minor
- **M1. The widened pin forbids the call F8c is pointed at** (test 26 at line 1653, against F8c at line 1955 and §4.11 at lines 999-1000).
  - F8c's `school.store` and `calendar.store` live in `onboarding_jobs.rs` (line 1157) and must send `PUT /account/sources`, "retrying once as `create_vault_in` does".
  - `create_vault_in` does this through `lms_link::store_source` (`onboarding.rs:767`), and so does `attach_in` (`account.rs:1031`). It is the only helper that takes a session target. The other, `put_source_at` (`lms_link.rs:770`), takes a raw token.
  - Scenario: F8c's implementer calls `lms_link::store_source(&api_base(), &cfg.session_credential_target, ..)`, which is the correct target. Test 26's source pin goes red, and the easy way out is to weaken a test written first.
  - Fix: name the call in F8c, either `put_source_at` with the token read from `cfg.session_credential_target`, or `store_source` with that target. Then narrow the pin to what is actually wrong: `store_source` with `PENDING_TARGET`, or `store_source` anywhere outside F8c's runner.
- **M2. Line 2004 lists `PENDING_TARGET`, which test 26's pin (line 1653) does not.**
  - F2g in `onboarding_state.rs` must name `account::PENDING_TARGET` (lines 1721 and 1728-1729) for its move and its delete.
  - If anyone builds the pin from line 2004, F2g cannot pass. Drop `PENDING_TARGET` from line 2004, or limit it to F8lb's two commands.
- **M3. Nothing tests that `capture_link_only` passes `validate_only`.**
  - Test 26 drives only the paste path, and its pin does not read `lms_link.rs`.
  - If `capture_link_only` were wired to the private `finish`, it would compile and still store under the pending session. Only contract-reviewer would see it.
  - Add a source assertion over `lms_link.rs`: `capture_link_only`'s body names `validate_only` and does not hand `finish` to `capture_calendar_link_with`.

## Outside this round's scope (seen in passing, not re-audited)
- F9 is "about 50 lines in `onboarding_jobs.rs`" (line 2010) but depends only on F1 (line 1227). So it can run alongside F8a-F8lb in the same file, which goes against "F8lb last in `onboarding_jobs.rs`'s chain". Chain F9 into that sequence, or give it its own file.
