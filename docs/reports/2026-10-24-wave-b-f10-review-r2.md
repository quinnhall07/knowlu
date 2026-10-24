# Wave B F10 review, round 2 (2026-10-24)

Reviewer, on `git diff 35109d1..8f39d31` (fixes 8f39d31 on 8292301). Filed by the controller. The diff touches only `app/static/{console.css,console.js,index.html}` and `app/tests/static_assets.rs`. Nothing on the contract list.

**Verdict: FAIL** on one copy line. The controller fixed it in the commit that files this report; see the end.

## Round-1 issues: closed
- **Syllabi row:** the rail has 8 rows and no syllabus wording; pinned.
- **Folder name after a reinstall:** `suMakeFolder` skips registered names, retries on "already exists" up to Knowlu 20, then shows a named error. Nothing is written before `dest_for`. The retry is marked INTERIM, and F3 deletes it.
- **Wizard rules moved to F11:** closed in the plan (§9 A10, F11t), apart from I2 below.
- **Counter, estimate and privacy pin:** "Step 1 of 10" and "Step 2 of 10" are pinned; there is one "About 10 minutes"; `#su-privacy` is filled from `PRIVACY`, which is pinned to `site/privacy.html`.
- **No single-user assumptions.**

## Removed tests (24), judged
Each removed test is one of three kinds:
- **Replaced** by an equal or stricter pin: the account panel, the default folders, the email doors' busy guard, the console's three Google commands, no LMS credential field, the subscribe step, the ten-panel pins, the finish panel's restore sentence.
- **Carried to F11** by §9 A10 / F11t: the blank zyBooks row, the phantom slug, discovery in flight, the mapping row, the school's rules sentence, Gmail testers only, the logins mapping.
- **Retired with their panels,** with nothing still shipped.

None of them guarded the console's views, sign-in or the updater.

## Important
1. **The welcome promises too much skipping** (`index.html:139`). "Everything after your school can be skipped" is false, because payment (step 4) and step 10 have no skip. That breaks VISION's honesty commitment.
2. **A spec rule lost its owner (plan, not F10 code).** Spec §5 says "credentials leave page memory the moment the write returns" (`clearCredentialFields`), and no task now carries it. F11c's zyBooks and VHL fields need that pin. Needs one A10/F11t line.
3. **The shots script calls a removed function.** `scripts/console-shots.py:91` calls `KNOWLU_SHOTS.startWizard`, which no longer exists, so the shots run aborts. It is not a CI gate. Give it an owner (F11a or F12).

## Minor
1. **Uncalled commands F5/F5b don't name.** The page no longer calls `open_external` or `retarget_credentials`, and never called `timezone_for_state`. F5b must name all three.
2. **A misnamed test.** `…a_nine_row_rail…` asserts eight rows.
3. **String literals with raw newlines** in `static_assets.rs:3093-3098`. They work because the file is LF, but they read as a heredoc artifact.
4. **Stale docs** (docs-keeper): `app/README.md:293` (`startWizard`, "seven panels") and the `account.rs:2100` doc comment (`#wiz-google`).

## Controller's fix (this commit)
- **Important 1:** the line now reads "Most steps after your school can be skipped, and each one says what skipping costs." The setup-shell test pins the new wording and the absence of the old.
- **Minor 2:** the test is renamed `…an_eight_row_rail…`.
- `cargo test -p knowlu --test static_assets`: 149 passed, 0 failed.
- Important 2, Important 3 and Minor 1 go to the planner, for F11c, F11a/F12 and F5b. Minor 4 goes to docs-keeper at F13.

**F10 passes** with this fix.
