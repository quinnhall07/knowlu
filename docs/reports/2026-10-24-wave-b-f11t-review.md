# Wave B F11t reviews (2026-10-24)

These reviews cover the failing static tests for the onboarding screens (`app/tests/onboarding_static.rs`). The reviewer checked each round; the controller filed this report.

## r1 (`94b2905..57ad517`): FAIL
Six Important findings:
- **I1.** A wrapper around `invoke` crashed `invoked()`.
- **I2.** The vm stub was not a browser-like page.
- **I3.** Arrow functions were missed, and an IIFE wrapper could satisfy pins.
- **I4.** The credential scan flagged step 8's row confirm.
- **I5.** The Open Knowlu rule was under-specified.
- **I6.** The reveal's fix text came from the server.

Controller rulings:
- `finish_ready` is computed on the server, and the page reads it. Added on O3 at b95292b, test first.
- ALLOWED lists only the commands the plan names.

## r2 (`57ad517..5cff9d3`): FAIL
I1-I6 are fixed. Three new Important findings:
- **N1.** Stripper offsets drifted on multibyte characters.
- **N2.** The removal answer's shape was wrong.
- **N3.** `mail_setup_start` is banned, because the queue's `mail.setup` owns the run, and the drain doesn't wait for a run the page starts.

Plan gap, sent to the planner as §14.52: nothing enqueues `mail.setup` yet.

## r3 (`5cff9d3..efe9e45`): PASS
N1-N3 and the minors are fixed. A stripper self-test now passes. The 38 page tests fail for the right reason:
- 36 because `static/onboarding.js` does not exist yet.
- 1 on `renderFirstRun`'s `(not yet:`.
- 1 on `console.js`'s `handover`.

Minors for F11a:
- **Credential pin.** It is text-based. Pass-throughs, copies made later, `!==`/`>=` and `;` inside strings can slip past. Harden it by splitting on the skeleton and allowing `.value` only inside the `store_credentials` invoke or a local `var`.
- **Stripper word tracking.** Words merge across whitespace when there are no semicolons.
- **Password check.** It keeps the ±200-character id window.

Step 5: the reviewer recommends an `onboarding_answer` step-5 shape over a new command (§14.52). F11b adds the step 5 entry.

Conventions the screens must follow (ids, handlers, `window.KNOWLU_ONBOARDING` helpers) are listed in the test file's header.
