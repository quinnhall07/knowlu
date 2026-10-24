# F11p1 shots: the look, the rail's lines and its foot

Headless Edge, 1280x800, scale 1. Invented data only: Maya Okafor, Halverson State University,
`maya.okafor@example.test`. Built screens come from a scratch copy of `app/static` (the step-8 shot
through `?fixture=` with the new four-command file shape); the prototype screens are
`5b-departures-board/index.html?shot=1&theme={dark|light}#{id}`. Pairs, dark then light:

| Screen | Built | Prototype |
|---|---|---|
| Welcome (`o0-welcome`) | `o0-welcome-dark-built.png`, `o0-welcome-light-built.png` | `o0-welcome-dark-prototype.png`, `o0-welcome-light-prototype.png` |
| Account (`o1-account`) | `o1-account-dark-built.png`, `o1-account-light-built.png` | `o1-account-dark-prototype.png`, `o1-account-light-prototype.png` |
| Step 8 (`o7-zybooks`) | `o7-zybooks-dark-built.png`, `o7-zybooks-light-built.png` | `o7-zybooks-dark-prototype.png`, `o7-zybooks-light-prototype.png` |

## What matches

Rail, then main pane, in the same order; the logo with the red block behind "lu"; the rail's Done rows
with their proof lines ("Signed in as ...", "Trial on. First charge October 31."); the foot with
"About N minutes left" and Finish later (step 8); both themes follow the same token names.

## Differences that remain

- **Faces.** Zilla Slab and PT Serif Caption, not Big Shoulders and Public Sans (Quinn's ruling), so
  wraps and heights differ. Zilla's tabular figures are wide ("3 1" in a date); proportional figures
  are not available in the vendored files.
- **Steps 1 and 2 (the shell) keep today's foot**: "About 10 minutes", no Finish later. The shell's
  estimate is pinned elsewhere (`static_assets.rs`) and its rail has no Finish later; the prototype shows
  "About 9 minutes left" and Finish later there. Its step count reads "of 10", not "of 9" (no syllabi step).
- **Rail copy** is the shell's today: no "syllabi" in the school line, "Check your class times".
  The welcome rail is blank at the top ("Step 1 of 10", not "Before you start"). The welcome screen also
  shows the privacy paragraph and the checkbox is unticked on the account shot; the prototype mocks both.
- **Step 8** is the built page: both sites' sign-in forms stacked, not the prototype's one-site card with
  "Knowlu's form", the "Found at your school" lines or the bottom week strip. Those are F11p2.
  Its rail shows Your week as "Reading" from the fixture; the prototype's sub-rows are not built.
- **The logo's "lu"** is cream on the red block in both themes (the prototype's light shot has dark ink
  there); cream keeps the contrast on #BC4444. The picker's brand keeps its square mark.
- Top-right Sign out on step 8 is built; the prototype has none.
