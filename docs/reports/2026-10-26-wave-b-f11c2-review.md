# F11c2 review: step 10's hours Edit

Reviewer: `reviewer` (read-only; the controller filed this report). Range: `c901e17..d9e757b`. **PASS**, with four Minors, all fixed before the merge (`8b70076` pins, `b85f288` fixes).

## Checked
- **The data path.**
  - Step 10's Edit rows come from `your_week`, which builds each confirmed row's `meets` through `meets_json`. That gives zero-padded `HH:MM` times and lowercase day keys.
  - A confirmed note with no valid entry is dropped. So the prefill is real and never empty.
- **What a save sends.**
  - It sends only the changed keys, with `meets` as the whole array.
  - Step 10 makes one `set_fields` call, with `kind`, `level` and `meets` only.
  - The refusal is `env.error` as given. The page holds no copy of the engine's sentence, and there is no fallback line.
- **Accessibility.**
  - Each entry is a fieldset with a legend.
  - The day toggles are native buttons with `aria-pressed` and labels like "Time 1 Mon".
  - The time inputs have labels, and everything is reachable by keyboard.
- **The old pin at `:1135`.** It re-derived "only what changed" and is amended per the review. `editFields10` decides what is sent.

## Minor (fixed)
1. **Day order.** A note whose `days` are not in Mon-to-Sun order would have re-sent `meets` on a level-only change. That journals `meets` as the student's, which breaks judge once for the registrar's later changes. `editFields10` now compares the days sorted.
2. **The time label after a save.** The row's time label kept the old hours. A save that sent `meets` now reloads the set.
3. **Focus after Add and Remove.** Focus fell to the page body. It now goes to the new entry's first day button after Add, and to "Add another time" after Remove.
4. **The shots' size.** They are 1280x900, not 1280x800, because the open Edit is taller. The index says so.

After the fixes, `onboarding_static` gives 113 passed and 12 failed. The 12 are the step placeholders.
