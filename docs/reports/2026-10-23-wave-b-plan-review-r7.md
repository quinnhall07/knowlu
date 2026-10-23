# Wave B plan review, round 7 (confirm round, 2026-10-23)

Reviewer (spec/plan mode) on `docs/plans/2026-10-23-wave-b-long-onboarding-plan.md`, o1 worktree. Filed by the controller.

**Verdict: FAIL.** 0 critical, 1 important, 4 minor. Both r6 Important findings are fixed, and the four r6 minors are closed in the text. The new Important finding comes from the I-2 fix: the task order means the main capture path is built before the seam it needs exists, and the source pins do not cover the obvious workaround. The fix is a few lines.

## r6 findings
- **I-1: closed.**
  - F2g (lines 1722-1729) now forgets with a local `credentials::delete(account::PENDING_TARGET)` only, with no `sign_out_with` and no `/logout`.
  - Test 21 (lines 1618-1620) runs the real delete over a generated target and pins the source.
  - The `sign_out_with` row (line 2249) is corrected. No `account.rs` edit.
- **I-2: closed for the command pair.**
  - `capture_calendar_link` (`lms_link.rs:397-435`) ends in the private `finish` (`:843-847`).
  - Changing that one call into a `finish` parameter is about ten lines. The shell's signature is unchanged, and so is its registration on the shell list (`main.rs:107`).
  - No existing test pins `capture_calendar_link`'s body. The `store_source` pins at `tests/lms_link.rs:511-520` and `:715` read `capture_courses` only.
  - `finish_with` (`:858-876`) with a store that returns `Err` gives `stored: false` and a note, which is what F8l and test 26 specify.
  - The console list (`main.rs:194`) holds neither command of the shell pair today, so F5's pin holds.
  - Other callers of `store_source` are `attach_in` (`account.rs:1031`, which F11t pins as never called) and `create_vault_in` (`onboarding.rs:767`, which the new flow does not use, line 1764). Neither is reachable from onboarding.
- **M1-M4: closed in the text.** See below for the remaining chain gap.

## Important

**I-1. F8b is built before F8l, but F8b needs F8l's seam, and the pins miss the workaround** (plan line 1939 against the §7 rows at 1217 and 1222).
- F8b's `school.capture` is supposed to run "the LMS capture (F8l's validate-only seam, never `lms_link::finish`)".
- But F8l depends on F8b, and it is F8l that creates `capture_calendar_link_with`. So when F8b is built, the only LMS capture available is `pub fn lms_link::capture_calendar_link(app, unitid)`, which still ends in `finish`, then `store_source(PENDING_TARGET)`.
- **Scenario:**
  - F8b's implementer calls `lms_link::capture_calendar_link(` from `onboarding_jobs.rs`. It compiles, and it is the background path every curated school takes at step 3.
  - F8l later adds its two commands but has no instruction to rewire F8b's runner.
  - Test 26 pins only `store_source` and `lms_link::finish(`, and it exercises only the paste path, so the call passes.
  - The result is r6 I-2's defect on the main path: a link stored under whatever sign-in is parked at `knowlu/pending/session`. F8b is not reviewed by contract-reviewer.
- **Fix:**
  - Either move the `lms_link.rs` split, plus a validate-only `capture_link` helper, ahead of F8b (make it F8l's first half, or make F8b depend on it), or have F8l rewire F8b's runner explicitly.
  - Add `capture_calendar_link(` and `paste_calendar_link(` to test 26's source pin over `onboarding_jobs.rs` and `onboarding_state.rs`.
  - Add F8b to the r6 I-2 ledger row (line 2153).

## Minor
- **M1. F11e (line 1227) and F11b (line 1228) don't list F8l as a dependency.**
  - Their screens call `onboarding_capture_link` and `onboarding_paste_link` (line 2018).
  - The static tests pass without F8l, but F12's walk-through and F15 would not. Add F8l to both rows.
- **M2. F2b's row (line 1198) depends on "F2a, F2d, F8f", not F2e.**
  - The stated chain "F2a, F2d, F2e, F2b, F2g, F2h" (lines 1201, 2841) is not enforced, so F2e and F2b can still run in parallel in `onboarding_state.rs`.
  - Add F2e to F2b's dependencies.
- **M3. F8l's own pin text says `finish(` (line 1986), while test 26 says `lms_link::finish(` (line 1649).**
  - A bare `finish(` match would also catch any `*_finish(` call in `onboarding_jobs.rs`, for example a call into F2b's finish.
  - Use test 26's form in both places.
- **M4. `credentials::delete` returns `Err` when there is no credential** (`credentials.rs:60-65`, where `CredDeleteW` fails with not-found).
  - F2g says "where it exists". State that a not-found result is ignored, so that test 21's no-pending case still gives the named refusal, not a delete error.

## The questions asked
- **Is the pending feed credential written before validation fails?** No. F8l writes after `finish_with` returns:
  - a validated link as `validated`;
  - a rejected paste as `rejected`, which §4.11 keeps on purpose (lines 996-1003);
  - a failed capture (`capture_failed`) writes nothing.
- **Is F8l routed and sized correctly?** Yes: `implementer` reviewed by `contract-reviewer`, about 40 lines, off the contract list (`lms_link.rs` is not on it), after S4b. The one ordering problem is I-1 above.
- **Does the `console.js` `(not yet:` change touch the normal console?**
  - `renderFirstRun` (`console.js:1531-1544`) serves only the normal console's first-run view (`env.first_run`, `:1588`).
  - The pins at `static_assets.rs:859-871` do not include line 1542's skip test, and no fixture or test contains `(not yet:`. The change only adds a case.
  - The Runs view reads the runner log, and scope rows never enter it (`scheduler.rs:1037`).
  - The change actually closes the case where a handed-over `NoRank` summary would have shown a check mark.
- **Order and ledger after the M2 reorder:**
  - TDD order holds (F1 before F2-F8, F11t before F11a-F11e).
  - F5 now waits on F8l.
  - The ledger has rows for r6 I-1, I-2 and M1, apart from the F8b gap in I-1 and the chain gap in M2.
