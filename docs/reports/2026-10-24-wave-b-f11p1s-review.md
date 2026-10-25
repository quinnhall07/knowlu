# Wave B F11p1s review: the shell's minutes line and Finish later (Q-B25)

Reviewer: `reviewer` (read-only; the controller filed this report). Diff `996137d..414e894`, with the
fix round in a68ade2. **Verdict: PASS, conditional on finding 1. Finding 1 was answered by Quinn and
applied.** No contract-list file or frozen reference is touched, endings are LF, and the shots use
invented data.

## Checked
- **`setup_quit`:**
  - shell-only: it is on `run_shell`'s list, now 31 names, and not on `run_console`'s;
  - its refusal matches `open_profile`'s;
  - it writes nothing;
  - the sign-in session is still wiped on `RunEvent::Exit`.
- **The disabled expression:** `SU.making || (SU.step === "done" && SU.busy)` matches §14.59 item 2.
  It holds through the Continue relaunch and its failure, and a sign-in never carries `busy` onto the
  done screen.
- **Minutes:** 615, 585 and 540 s give 11, 10 and 9. The screens read 11, 10, 10, 10 and 9.
  414e894's fix to 7 for step 4 (420 s) was a slip in the pin, not a bent pin.
- **From step 3 on:** unchanged. A profile resumed at step 2 inside the setup frame now reads 10,
  which matches the shell.
- **Pins:** none was loosened.
- **Shots:** fonts as ruled.
- **The browser screen's two buttons** were never in wave B's scope. They are now recorded as
  unowned in §14.59.7.

## Findings and outcome
1. **Important, welcome copy "about 10 minutes" against the foot's 11.** Quinn ruled on 2026-10-24
   to drop the number. *Fixed* in a68ade2: the copy changed, a pin (`the_welcome_copy_carries_no_minutes_figure`)
   was added, and a note went into the draft spec's row 1 and S-E row.
2. **Minor, the disabled Finish later looked clickable in the shell.** *Fixed:* the rule now covers
   `.setup button[disabled].su-link`.
3. **Minor, the rail-foot pin counted font-size text file-wide.** *Fixed:* the pin finds the two
   rules by selector, and the five respellings are undone.

Final counts: onboarding_static 54 passed / 13 failed (the step-panel placeholders, listed in the
fix report), static_assets 151 passed, onboarding 83 passed.
