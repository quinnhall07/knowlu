# Wave B plan §14.59 (F11p1s) review (spec/plan mode): PASS

Reviewer: `reviewer` (read-only; the controller filed this report). Diff `2f8e006..b21f1e4`, plan only,
checked against the code at b21f1e4.

## Questions asked
- **Mid-shell quit, half-made profile or stale lock: none possible.**
  - `SU.making` covers the whole `create_vault_defaults` call (`console.js:3384-3390`), and Finish
    later is disabled for that span.
  - `open_profile` spawns the child with `--after-pid`, then `app.exit(0)` (`onboarding.rs:378-388`).
    With Finish later disabled during Continue's relaunch, a student can't quit while the profile is
    being reopened.
  - The shell holds no `ConsoleState`, queue, scheduler or vault lock (`main.rs:87-109`), and
    `RunEvent::Exit` wipes the sign-in session.
  - Next launch: before the folder is made, nothing changed; after it, the profile is listed "Setup
    not finished" and resumes at step 2.
- **`setup_quit` stays shell-only.** It is on `run_shell`'s list only (30 names, recounted at
  `main.rs:107`), the pin checks both lists, and the `Onboarding` guard refuses in console mode.
- **Replacing the "About 10 minutes" pin is justified** (Q-B25). The figures are still spec §1.3's
  midpoints: 540 s from step 3 is 9 min, 585 s from step 2 is 10, and 615 s from step 1 is 11.
- **§7 and the merge order are consistent** across §14.58.7, the F11b0 row and §14.59.6, with no
  cycle.
- **No task is over the size cap.**

## Minor
- **m-1:** `run_shell`'s list is one line, and F5b edits it too. Chain F11p1s after F5b, or add a
  collision note saying the second to land rebases and recounts.
- **m-2:** `SU.busy` also covers a sign-in in flight. Pin that Finish later stays enabled during a
  sign-in and is disabled only for Continue's relaunch.
- **m-3:** carry the old pin's meaning ("the spec's estimates, until S-E measures real times") into
  the new pin.
- **m-4:** §14.59's intro leaves out `o1g-google`. Make it match the row and item 5.

**PASS**
