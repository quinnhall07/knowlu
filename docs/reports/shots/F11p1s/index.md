# F11p1s shots: the shell's minutes line and Finish later (Q-B25)

Headless Edge (Playwright, `channel=msedge`), 1280x800, scale 1, dark then light. Invented data only:
`maya.okafor@example.test`, a made-up folder. Built screens come from a scratch copy of `app/static`
through `KNOWLU_SHOTS.startSetup` (the copy adds one shot-only setter; the page itself is unchanged).
Prototype: `5b-departures-board/index.html?shot=1&theme={dark|light}#{id}`.

| Screen | Built | Prototype |
|---|---|---|
| Welcome, "About 11 minutes left" | `o0-welcome-{dark,light}-built.png` | `o0-welcome-{dark,light}-prototype.png` |
| Account, 10 | `o1-account-*-built.png` | `o1-account-*-prototype.png` |
| Browser (Google), 10 | `o1g-google-*-built.png` | `o1g-google-*-prototype.png` |
| Code, 10 | `o1c-code-*-built.png` | `o1c-code-*-prototype.png` |
| Signed in, 9 | `o1d-signed-in-*-built.png` | `o1d-signed-in-*-prototype.png` |

## Intended differences

- **Finish later on every screen.** The prototype hides it on some of these screens; Quinn's ruling Q-B25
  overrides that. The built foot reads "About N minutes left" then Finish later on all five.
- **Fonts.** Zilla Slab for text and PT Serif Caption for the logo (checked in the shots), not Big
  Shoulders and Public Sans, so wraps and heights differ.
- **Counts.** The built shell reads "Step N of 10" and 11, 10, 10, 10, 9 minutes; the prototype reads
  "of 9" (it has a syllabi step) and its own mock minutes.
- **Rail copy and screen furniture** are the shell's today (no "syllabi"; no "Open the browser again" or
  "Use an emailed code instead" buttons on the browser screen, as built before this task). Not in scope.
- **Disabled states** (making the folder; Continue relaunching) are not shown; the pins cover them.
