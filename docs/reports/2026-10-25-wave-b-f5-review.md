# F5 review: the console's handler list

Reviewer: `reviewer` (read-only; the controller filed this report). It reviewed `84797b5..9f4dd75` and passed it, with three Minors. The fix commit came after the review.

## Checked
- **The list.** The console list gains exactly the plan's 14 names, taking it from 75 to 89.
  - None of these are added: `timezone_for_state`, `store_credentials`, `discover_coursework`, `capture_calendar_link`, `paste_calendar_link`, or F8l's two commands (not written yet).
  - The wizard list is unchanged at 31.
- **Managed state, in both modes.** Without a `Queue` (F4c manages it), every queue-backed command returns its named no-setup envelope and never panics. `attach_session`, `handover` and `mail_not_now` take `ConsoleState`, which is managed in both modes.
- **Normal console mode.** No path writes under the wrong session or credential target.
  - `attach_session` checks both account ids and the vault's own target.
  - The `lms_link` commands write no credential.
- **IPC scope.** IPC is scoped to the `main` window, so the campus window gains nothing.
- **Other rules.** No contract-list file is touched, and there is no single-user assumption.

## Minor (all fixed by the controller)
1. The new pin checked only `onboarding_status` against the wizard list. It now loops over the ten `onboarding_state` names.
2. `capture_calendar_link` and `paste_calendar_link` were not pinned as console-absent. They are now. `handler_lists` passes: 9 passed, 0 failed.
3. Plan §14.63 said Open Knowlu works once F5 lands, but it waits for F4c's `Queue`. The plan is corrected.
