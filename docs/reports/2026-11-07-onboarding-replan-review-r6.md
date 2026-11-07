# Onboarding replan draft, review r6 (2026-11-07)

**FAIL at d60e6dc:** 0 Critical, 4 Important, 5 Minor. Reviewed a818ab7..d60e6dc. One recommendation rests on a false premise: the planner's call, at the end of this report.

**What holds.** The plan matches RQ3, RQ4, RQ5 and RQ16 (lines 81-106, 1460-1482). The withdrawn "mailbox carries the LMS" design survives only in lines that record its withdrawal (63-66, 1526), apart from Minor 5.

The saved password stays on the device. It goes through the public `credentials::target_for`, `write`, `exists`, `delete` and `wincred::read_credential`, which I checked in `app/src/credentials.rs:17-60` and `onboarding.rs:1193-1210`. P1 and P2 belong to `contract-engineer`, and S5 stays on `account.rs` with `contract-engineer`.

The task counts (178), the 20 changed rows and the layer edges for P1t to P5 are consistent.

## Important

1. **A rejected password can be sent again (§7.4 line 966, §7.5 lines 995-996, P1t line 1346).** `Rejected` needs the page to "show its own error", but nothing defines how the fill recognises that error. Any page it does not recognise counts as `CannotRead` after a submit, and that pause ends after one visible sign-in.
   - *Scenario:* the student's school password expires. The fill submits the old one and the school shows a "password expired" page. That counts as `CannotRead`, not `Rejected`. The student signs in visibly on the school page with the new password, and the pause ends. About 12 h later the fill submits the stale password again, and that repeats at every lapse. This breaks RQ16's "never retried".
   - *Fix:* after a password submit, only a new save ends the pause, or every unrecognised post-password page counts as `Rejected`. Add a P1t/P3 case.
2. **The host check covers the page, not where the password is sent (§7.5 lines 985-987, P1t lines 1351-1352).** The check is "a host in `sign_in_hosts`, checked before each field". It never checks where the form submits.
   - *Scenario:* a curated sign-in page's form `action` points off-list, after a vendor change or an IdP hand-off. The fill fills it and submits, and the password goes to an unrecorded host.
   - *Fix:* when the form submits by navigation, its resolved `action` must be https on a `sign_in_hosts` host; otherwise the result is `CannotRead` with nothing filled. Re-check the host before the submit as well, and add a P1t test for each.
3. **How the password gets into the page is unspecified (§7.5 line 991, P2 lines 1372-1375).** Neither the brief nor P1t says how the secret is placed into the page script.
   - *Scenario:* a password containing `'`, `\`, a newline or a non-BMP character is altered or breaks the script. The school rejects a correct password, the plan deletes it (line 964), and every re-save fails the same way. A script error that echoes its source would also put the secret in an error string.
   - *Fix:* pass the value JSON-encoded, never through `format!` into the script, and never forward the script's error text. Add P1t cases for those characters.
4. **No task deletes the saved password at Forget (§4.5 lines 287 and 305, layers at line 361, ledger line 1552).** S6b says "Forget deletes the folder and the saved password", but the Rust `school_forget` belongs to S6a. S6a is listed as "kept, not altered" and has no edge to P1.
   - *Scenario:* the student presses Forget. The folder goes, but `knowlu/<id>/school` stays. The next grades slot creates a new folder and the fill signs in silently, so Forget is undone.
   - *Fix:* move S6a into "changed", have it call P1's forget, add the edge S6a → P1 and add a ledger proof row.

## Minor

1. **Lines 969-970:** "one hidden read per slot" does not cover F8b's queue window and a slot's grades read running at the same time. The rejected mark and the secret must be read after `school_session`'s lock is held, and P3 has no concurrency test.
2. **Lines 1365-1366 vs line 232:** P1 adds a command to `generate_handler!` in `main.rs`, but `main.rs`'s file chain does not list P1. P1 runs at layer 8 alongside F4c, F5b and S6r. A badly merged list drops the save command, and nothing fails until runtime.
3. **Line 1349:** `retarget_credentials` moves `school` but not the rejected mark. If the delete failed (line 1345), the new target holds a rejected password with no mark, and the fill sends it again.
4. **Lines 251 and 1370-1375:** P4 has no `test-writer` half for its pins, unlike C6, C7 and C10. P2's window side, the script that picks the submit control and fires the events, has no test before P5, and that is where the 11-07 bugs were. Add static pins on that script.
5. **Line 289 vs line 1317:** S8 narrows wave-B §9 rows A1 and A5, and C9 narrows A1, A3 and A5. Wave-B row A1 (line 3321), "one kept school profile per identity", is a single-shared-folder leftover that RQ4 contradicts, and neither task rewrites it for two folders.

## Open questions and the planner's call: false premises only

- **RQ17:** no false premise. `sign_out_with` (`account.rs:753-797`) deletes only the session and the mail profiles. `zybooks`/`vhl` go only in `delete_local_data` (`account.rs:1696`). The school folder goes at sign-out (wave-B plan line 1035, S5).
- **RQ18:** no false premise. The mailbox already has its own Settings rows (`console.js:2235-2271`, `EM_PROVIDER` at 2708).
- **The planner's call (lines 974-980), deleting the password on a setup rejection and showing one "Update school password" line, rests on a false premise.**
  - It assumes the student can "finish there", on the visible school page. After a `Rejected` nothing is saved, so finishing there signs the LMS in only until the first lapse, about 12 h, and the student then gets "Sign in to update grades".
  - The line is also not "the one ask": the student is asked twice, once on the school's page and once on the Update line.
