# Wave B plan §14.52, review r4 (b094edb): FAIL

Reviewer: `reviewer`, in spec/plan mode. The controller filed this report.

## Important

### R4-1. F2b2b depends on S4c, which is not built

S4c has no commits on any branch, and F2b2b sits upstream of F8b2, F8g, F8w1, F8w2 and F4c. So the whole onboarding queue would wait on another stream's mail task.

The dependency is only a file collision: both tasks edit `disconnect_with`. Neither needs the other's code.

**Fix:** move Disconnect's 3-line `TurnedOff` event into its own task, F2b2c (`mail/commands.rs`, `implementer`), with a collision note against S4c. F2b2b then edits only `onboarding_state.rs`.

## Closed

- **R3-1:** no cycle when §7 is walked again.
- **R3-2:** the code matches.
  - `turn_off` returns early (`mail/commands.rs:302-305`) before the event fires.
  - The event fires before `list_addresses` and `delete_address`.
  - `apply_mail_signal` takes only `ONBOARDING_JSON_LOCK`, so no locks nest.
- **n1, m-a, m-b and m-c:** closed.

## The planner's two calls

Both are safe.

- **Early Disconnect leaves the item.** The early returns happen before `turn_off`, so nothing has been turned off yet and the item should stay.
- **Any Disconnect clears the item.** Items carry no address id, and `CanaryVerified` already clears from any row.

## Minor

- **n-1.** If Q8 = yes, F2b2b grows to about 88 lines. By the plan's own rule, the patient read and test 38's read half then move to F4c. Q8 should say so.
- **n-2.** Split F8h into two rows:
  - **F8h**: the retry and the input rule. F8g depends on it.
  - **F8h-runner**: gated on Q7. F8w1's `registrar.confirm` arm lands with it.

  State F4c's dependency on Q7 explicitly.

## Holding

- No vault writes outside `write`.
- No lock held across the network.
- Q7 and Q8 stay Quinn's.
- `generate_handler!` changes only under Q8 = yes.
- Every test has an owner.
