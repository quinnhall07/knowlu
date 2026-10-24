# Wave B F11a review r2 (495bc81 against f070dee): PASS

Reviewer: `reviewer`. Filed by the controller. No contract-list file is touched, and line endings are LF.

## r1 findings, now closed

- **C1.** `launch_state`'s `None` arm answers `"onboarding"` exactly when `LaunchMode::Onboarding` is managed. The `console.js` dispatcher returns on it before the console and picker checks. The stopgap is gone.
- **I1.** The frame now disables and re-enables only the buttons it marked with `data-ob-closed`.
- **I2.** The held and not-yet rows reset `cls`.
- **I3.** Each leave hook is caught, and the wait is capped at 3 s.
- **M2, M4, M6.** Fixed.
- **M5.** Documented: a poll never rewrites the pane, and `OB.refresh()` still does.

## Pins

- Code-stripped or behavioural, so a comment can't satisfy them: the dispatcher pin, the rail-class pin, the leave-hook pin and the z-index pin.
- The `data-ob-closed` pin only checks the string is present.
- **m1 (fixed by the controller):** the `launch_state` pin read the raw file, so a comment could satisfy it, and its scope ran from the function to the end of the file. It is now bounded to the function's body, with `//` lines dropped.

## Remaining minors (for later tasks)

- **m2.** A retry refusal's message clears on the next poll, about 2 s later. It should clear on the student's next action instead.
- **m3.** An unmapped `found` reason would read as a failure, because nothing emits `status.found` yet. Either drop the read or give it its own word map.
- **m4.** Seam rule: panels must not call `OB.refresh()` from `onStatus`, because that wipes `#ob-ask`.
- **m5.** Two cases where a past row reads wrong. A row with no job records reads "Done". A row whose jobs are mixed done and skipped reads "Not yet". F11b and F11c fixtures should cover both.
- **m6.** Carried forward:
  - `openKnowluEnabled` still reads `gate.open`, against §14.52.6. This is the planner's to settle against the F11t pin.
  - `revealModel` drops `fix`. F11d's.
  - Unmapped reason keys. F11b's.
- The `OB.rowModel` and `OB.rows` exports are harmless.
