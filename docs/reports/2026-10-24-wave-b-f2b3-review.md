# Wave B F2b3 review r1 (971413b): PASS

Reviewer: `reviewer`. Filed by the controller.

## Minor
- **m-1:** Not now dropped `mail.connect` through `mail_signal(TurnedOff)`, which couples the guard and the removal to the item's signal. The controller fixed it to `handover.retain(|h| h.item != "mail.connect")`, which is what §14.57.9 says. onboarding_jobs 141/8 and onboarding_state 73/11 are unchanged.
- **m-2:** a step-8 row can survive a later step-3 removal of its course. F8c2's handoff must leave out a row whose key isn't a kept course's, and test 42 gains that case.
- **m-3:** F8c2 must merge before any build of the onboarding branch is tried, because rows now carry course keys.

## Rulings endorsed
- `answer_on_queue` with a source pin.
- `require_kept_course`.
- Not now does nothing without `mail.connect`.
- `error: null` in the confirm shape.
- The two `course_slug` fixture updates: they are legitimate updates under §14.57.3, not a loosening.

## Checked
- Lock order with no nesting, and no lock held across the network.
- The repeat guard sits under the queue lock.
- Absent and unfinished onboarding.json are never written.
- Refusals are byte-identical, and errors carry no path.
- No vault writes and no single-user literals.
