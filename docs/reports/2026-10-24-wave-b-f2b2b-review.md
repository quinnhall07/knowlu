# Wave B F2b2b review (d801048 vs f4399d0): PASS on one condition

Reviewer: `reviewer`. The controller filed this report and applied the fix. No contract-list file is touched.

## Condition (fixed by the controller)
- `onboarding_state.rs:604`: the `vault_only` doc comment held two control bytes (0x0F, 0x0C) where a heredoc ate the backslashes of `\config\cloud.yaml`. It now reads `<vault>\config\cloud.yaml`, and the file has no control bytes left.

## Important (plan gap, not this diff)
- **A student can reach finish having neither answered nor skipped step 5.**
  - The backend never moves `state.step`; only the page leaves step 5.
  - `finish_refusal`, `gate_items` and `handover_for` all key on records that exist.
  - If the step-5 answer is refused and the page moves on anyway, finish opens with no mail job, no skip and no `mail.connect`. The email is silently lost.
  - "No record, no item" stays right for `advance_after_school`'s proven-mailbox pass.
  - Guard needed: a page pin, or a recorded "step 5 proven" fact that `finish_refusal` requires. The planner rules on this.

## Rulings checked
1. The three old-test edits are each forced by m-1, m-2 or §14.52.1's hand-over rule, and none hides a regression.
2. `HandoverItem` stays `{item, signal}`, so the bytes are unchanged. Whoever draws the not-connected line uses `COST_MAIL_UNFINISHED`.
3. See the plan gap above.
4. Every state except done and skipped gets the item, including held, running and failed. `test_40` covers all of them, plus a bare skipped record.
5. `TurnedOff` clears only `mail.connect`. `Disconnected` keeps it.
6. `_g` is harmless.
7. `load_state_for_launch`:
   - Existing callers see no change except the text of one error.
   - Backoff is 25 ms, 200 ms, 1 s and 2 s, then the fatal.
   - Only an Unreadable file is ever rewritten.
   - Version 0 reads as "newer". That wording is the controller's to fix.
8. `vault_only` strips every copy of the vault path. Nothing else leaks one.
9. `failed_lane_job` is deterministic.
10. `slot_rig.rs` binds only to `127.0.0.1:0`.
