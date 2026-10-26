# F11d shots: the reveal (step 11) and the main page's still-coming line

Headless Edge, a scratch copy of `app/static`, the status staged through
`KNOWLU_SHOTS.startOnboarding(11, status)` (reveal, 1280x900) and `?fixture=` with a stubbed
`onboarding_handover` (main page, 1440x500). Invented data only: one failed grades item, `mail.connect`
not connected, Gmail skipped, two items still coming. The prototype is `5b-departures-board/index.html?shot=1&theme={dark|light}#r1-reveal`.

| Name | Built | Prototype |
|---|---|---|
| reveal, one failed, two not connected, two coming | f11-reveal-{dark,light}-built.png | f11-reveal-{dark,light}-prototype.png |
| reveal, everything done | f11-reveal-all-done-{dark,light}-built.png | none (prototype's variant differs) |
| main page, still-coming line (dark) | f11-main-still-coming-dark-built.png | none |

## Intended differences

- The prototype's top of page (what's next, tonight, the week board, tidied, from your email) is not built here: it needs
  read-model fields that are another task's (F7c/F9). F11d builds the gate-driven part: failed items with the server's fix and a
  Try again, not-connected items (mail.connect with its Settings button), still coming, and Open Knowlu.
- The built reveal shows its sections as lists in the setup frame, not the prototype's three-paragraph band.
- A failed item whose fix is the plain "Try again" shows only the button, not the same words twice.
- The rail on the built shots reads "Not yet" because the staged status has no job records.
- The main page's line is one block under the sync line; there is no prototype screen for it.
