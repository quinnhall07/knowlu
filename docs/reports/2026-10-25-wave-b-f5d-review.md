# F5d review: `set_fields` writes `meets`

Reviewer: `reviewer` (read-only; the controller filed this report). Range: `dd79f7a..2c0875a`. **PASS**, with three Minors. No contract-list file is touched.

## Checked
- **The routing.** `EDITABLE` has 15 names. A `meets` edit goes through `check_console_edit` and then `answer_literal`, never `literal_for`. The write is `write_literals` with `console_ctx`.
- **Test 46.** It sends `"9:00"` and asserts the exact line `meets: [{days: [mon], end: '10:00', start: '09:00'}]`, which meets F7e's advisory. The journal record carries actor `student` and via `dashboard`.
- **A bad value writes nothing.** Every literal is built before the write, and the test compares the note bytes and the journal before and after.
- **The other fields are unchanged.**
- **`console.js`'s drawer `EDITABLE` rightly leaves out `meets`.** Step 10's Edit belongs to F11c2, which goes through the onboarding panel.

## Minor
1. **The fixed message is duplicated.** The sentence appears in both the app and the engine; the app's branch can't be reached today. Not fixed. A later edit can make the engine's const public.
2. **A stale doc comment.** It said "a confirmed class's time", but any confirmed commitment except the planning day takes `meets`. Fixed by the controller.
3. **No planning-day case at the app level.** Test 45 covers it in the engine. Not needed.
