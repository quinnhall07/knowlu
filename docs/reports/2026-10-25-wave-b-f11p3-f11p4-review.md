# F11p3 and F11p4 review: step 10 and its day-window question

Reviewer: `reviewer` (read-only; the controller filed this report).

## Round 1 (`2abbf7b..a2a2844`): FAIL

There are no Critical findings and three Important ones.

### Important
- **I-1. The day window uses only its first entry.**
  - **Where:** `windowFields10` and `windowArg10`.
  - **Failure:** the engine proposes a window for each group of days, e.g. weekdays and weekends.
    - Editing a time sends only the weekday entry, so the weekend falls back to the template.
    - Pressing "That's right" sends both entries, writing weekend times the student never saw.
  - **Ruling (controller):** step 10 shows and sends every entry. Pins a1-a4 (`5c8aeb9`).
- **I-2. Test 49 was missing.** Added at `5c8aeb9` and passing: the JSON spelling writes the YAML bytes, and an unknown day is refused before any write. Closed.
- **I-3. The plan has no F11p4 row.** Closed on `feat/o3-long-onboarding` (`96a8245`): the F11p4 row and §14.62 were added there.

### Minor (fixed in the same round)
- **M-1. Event rows wrap.** The title should sit on the left, with the controls on the right as one group that never wraps. Fix: `.ob-ctl` (`inline-flex`, `nowrap`, `margin-left: auto`), and the title box `flex: 1 1 auto; min-width: 0`. The `.ob-win` row keeps its own rule.
- **M-2. "That's right" toggles.** It must set the answer and never un-set it (pin b).
- **M-3. A blank window counts as answered.** It must count only when both times are set (pin c).
- **M-4. The firmness group is unnamed.** Its label should be "How firm is {title}", escaped (pin d).
- **M-5. The time inputs' names differ from their visible labels.** The visible label text should name each input (pin e).
- **M-6. Two intro sentences repeat each other.** Remove the one in `askProposals` (pin g).
- **M-7. A firmness press is not an answer.** Quinn ruled that a firmness press counts as Mine, and Not mine still overrides (pin f).

### Intended differences (not counted)
- The labels Must keep / Usually / Optional. The values sent stay `hard`, `soft` and `optional`.
- 24-hour event labels come from the engine. A 12-hour clock is offered to Quinn as a separate task.

### Checked and fine
- An untouched guess sends no `window`.
- The window sends as a string.
- The events payload and F11c1's confirm order are intact.
- No pin was loosened and no student data was used.
- No contract-list or frozen-reference file is touched.

The 13 other failing pins in `onboarding_static.rs` are the red placeholders for steps 2-6 (later tasks), not this task's.

## Round 2 (`a2a2844..da50160`): FAIL on one point

Every round-1 finding is closed on the normal path, and test 49 closes I-2.

### Important
- **R2-I1. A window typed before the calendar guess arrives is sent with the guess's days.**
  - **Failure:** `fillWindow10` skips a guess that arrives after the student has answered. `confirm10` then passes the late guess, so the weekend entry is written unseen.
  - **Fix:** remember which guess the visible pairs came from. An answer typed into the blank pair sends all seven days. A pin covers it.

### Minor
- **R2-m1.** A firmness press counts as Mine, but Mine never shows as pressed. Fix: the press sets Mine itself.
- **R2-m2.** With two entries, the Wake/Bed names are ambiguous. Fix: the days go inside each label.
- **R2-m3.** Test 49 had a raw line break inside a string. Fixed in `060375b`.
- **R2-m4.** The two pairs don't align in columns (optional).

### Implementer rulings, both accepted
- **Layout:** the title and the controls share one line, with the time on the line below. `.ob-win` has its own rule.
- **Blank times:** `answeredCount10` treats an empty string as blank. No real path brings an `undefined` time.

## Round 3 (`da50160..407c6db`): PASS

Every round-2 finding is closed:
- **R2-I1:** the window remembers which guess it was drawn from.
- **R2-m1:** a firmness press shows Mine pressed, and a second Mine press clears the row.
- **R2-m2:** each label names its days.
- **R2-m3:** fixed.
- **R2-m4:** the pairs stack.

The only Minor left was a stale paragraph in the shots index, which the controller fixed before merging. The controller also confirmed the 13 remaining failures by name: they are the red pins for steps 2-6, waiting for later tasks. Test 49 passes.
