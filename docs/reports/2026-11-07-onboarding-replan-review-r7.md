# Onboarding replan draft, review r7 (2026-11-07)

**FAIL at 2ea7d85:** 0 Critical, 4 Important, 5 Minor. Reviewed d60e6dc..2ea7d85.

**What holds.**
- r6's Important 3, Important 4 and Minors 1–4 are closed. Minor 5 is closed apart from row A5.
- The changed table has 23 rows and the P count is 7. P4t sits at layers 12–13.
- These new edges hold: P1→S6a, P1→S6r, P4t→P4 and S6r→P4. Modules are declared in `app/src/lib.rs`, so P1 can stay out of `main.rs`.
- The secret's error paths look safe. The `wry::eval` span carries no script (`wry-0.55.1/src/webview2/mod.rs:1328`), no tracing subscriber is installed, and the mail runner already passes values in with `serde_json` (`app/src/mail.rs:384-385, 451-462`).
- RQ17 and RQ18 have no false premise.

## Important

1. **A sent password can go out again (§7.5 line 1023, §7.4 line 974).** The plan does not say when a submit counts as sent. The house start-then-poll pattern returns `Lost` or a timeout when the submit navigates away (`mail.rs:345-346, 466-468`). That result reads as `CannotRead`, the password is kept, and the next slot resends it. The same happens if the app closes or the machine sleeps between the submit and the mark.
   - *Fix:* write an in-flight mark to disk before the submitting script runs. Only `users/me` 200 clears it. Any other ending counts as `Rejected`. If the mark cannot be written, nothing is submitted. Add P1t cases.
2. **After setup, `CannotRead` leads to a form that cannot fix it (§7.4 line 974, P4t line 1457).**
   - *Scenario:* the school adds an account-chooser page. Every slot then ends `CannotRead`. The row opens Knowlu's password form, and saving the same password changes nothing.
   - *Fix:* for `CannotRead`, the row and Settings open the visible sign-in at the link. Pin each outcome's path in P4t.
3. **The §2.3 MFA branch rests on a false premise (lines 117-124).** The fill never answers MFA, and since r7 a `Rejected` at setup goes only back to Knowlu's form.
   - *Scenario:* under MFA, a correct password reaches the MFA page, which counts as `Rejected`. The password is deleted and the student reads "didn't accept that password", which is false. Setup can never finish, and the student's phone may get an unexplained push each time.
   - RQ5 as ruled covered this case by showing the school page. The coordinator's narrowing needs Quinn's confirmation.
4. **The form's `action` is checked once per attempt, against the form alone (§7.5 lines 1005-1009, P1t lines 1383-1385).**
   - *Scenario A:* the password step loads a new form whose `action` is off-list. It is never checked.
   - *Scenario B:* the chosen button's `formaction` or `formmethod` overrides the form's. A `GET` puts the password in the URL and the folder's history.
   - *Fix:* resolve the target from the chosen button (`formaction`, then `action`). Require https, a listed host and `POST`. Re-check before each field and again just before the submit. Add P1t cases.

## Minor

1. **Lines 995-997:** state the other costs of the conservative rule. A correct password is also deleted after the submit on an LMS 5xx, a network drop or a slow IdP. Give the post-submit wait its own window, and have P5 count how often this happens.
2. **Lines 1372-1376:** the order of steps is unspecified.
   - A rejection should set the mark before deleting.
   - A save should write the credential before clearing the mark.
   - Forget clears the mark even when the delete fails, which undoes Forget. Forget should keep the mark when the delete fails.
3. **Lines 368, 291:** S6t should depend on P1t, which defines the seam.
4. **Line 1392:** the round-trip encoding test always passes. The static pin is the real guard, so name the bare-argument splice in it (`mail.rs:456-457`).
5. **Line 296:** wave-B row A5 names no registrar, so there is nothing for "A5 loses the registrar" to remove. A5's Forget should instead gain the deletion of the saved password.

## Authority

RQ5 was ruled as "any failure shows the school page". The r7 split is the coordinator's design. List it at checkpoint 1 for Quinn's confirmation.
