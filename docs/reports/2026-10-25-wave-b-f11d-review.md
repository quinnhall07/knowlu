# F11d review: the reveal and the main page's still-coming line

Reviewer: `reviewer` (read-only; the controller filed this report). No contract-list file is touched.

## Round 1 (`f09c9eb..4142c89`): FAIL
- **I-1.** The main page drew `mail.connect` under "Still coming". §14.52.1 says it is a not-connected line with its Settings button, never "still coming", and its cost line was missing.
- **I-2.** The reveal's Settings button never renders with real data, because `mail.connect` is only in `handover`, after finish. If it did render, it would open behind `.setup`, and `mail_setup_start` refuses in onboarding mode. The pin and the fixture staged a status the server can't produce.
- **Minor:**
  - m3: the reveal's finish promise covered mail items.
  - m4: the read cadence didn't match §14.63.
  - m5: the leave hook cleared the double-press guard.
  - m6: the "no argument" pin pinned nothing.
  - m7: the handover tests left temp folders.
  - m8: spacing.
- **Rulings 1, 2, 3, 5 and 6 were accepted:**
  - `txt` escaping is safe;
  - the gate-only reveal;
  - Open Knowlu's exit path;
  - the poll;
  - the read without a lock.
- **Controller rulings (`d198edd`):**
  - no Settings button on the reveal;
  - `mail.connect` under its own "Not connected" heading on the main page;
  - the promise only for items that finish alone;
  - §14.63's cadence amended.

## Round 2 (`4142c89..3ea155d`): PASS
Every round-1 finding is closed. The pins were amended in `3bde0a3` and the fixes are in `3ea155d`. Two Minors are carried in the plan:
1. **The mixed case.** With `["mail.setup", "backfill.older_mail"]`, the sentence "Knowlu finishes these" still sits over the email row. The fix is to list the mail rows separately or word the sentence per group, then amend the pin's mixed case. This goes to F11e2 (the reveal's later pass).
2. **After Not now, the main page shows no skip cost.** `onboarding_handover` sends only handover names. §14.57.9 wants the not-connected cost line a skip shows. This goes to F7c/F9 (the main page's skip lines).

One note: the main page's cost sentence is hard-coded apart from `onboarding.js`'s `COST_WORDS`, so a wording change touches both files.

The controller found that `test_29_stop_both…` was failing on the branch before F11d. A debugger traced it to the F8e merge (`3bb46bc`), which left the test's setup stale. It is fixed on `fix/o3-test29` (`3dd140a`), with setup only and no assertion changed.
