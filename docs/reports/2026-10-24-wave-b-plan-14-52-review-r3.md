# Wave B plan §14.52, review r3 (9ea69b4): FAIL

Reviewer: `reviewer`, spec/plan mode. Filed by the controller.
Read: §14.52 (~4565-5186), §4.10's mail row (~1199) and §7 (~1470-1503), checked against O3 HEAD.

## Important

### R3-1. §7 has a dependency cycle

The four rows depend on each other in a loop:

- F2b2a depends on F8d2 (`skip_mail`).
- F8d2 depends on F8b2 (the file chain F8b2, F8d2, F8g).
- F8b2 depends on F2b2b.
- F2b2b depends on F2b2a.

None of them can be dispatched first.

F8d2 needs nothing from F8b2 except its place in the file chain.

**Fix:** move F8d2 to straight after F8b, so the chain is F8b, F8d2, F8b2, F8g. F8d2 then depends only on F8d and F8b. Update the F8d2 row, the placement line in ruling 1 and the r2 note to match.

### R3-2. `TurnedOff` reads a list state that a successful Disconnect never leaves

The rule in question is "the `Ok` list has at least one row and every row is in `turned_off`".

- On the app side the rule is correct: `turn_off` (`mail_session.rs:310-322`) is Disconnect's step 0.
- But Disconnect then calls `delete_address` (`mail/commands.rs:321`), and the service purges the row (`handler.ts:446-449`).
- So after a Disconnect that succeeds, the row is gone, and `TurnedOff` fires only when the delete failed.

**Failure scenario:** a student starts email setup in Settings, then presses Disconnect. "Finish connecting your email in Settings." stays forever, against what the student explicitly did.

**Fix:** treat the off switch as an event, not a reading of the list.

- `disconnect_with`, right after step 0, applies `TurnedOff` to a finished `onboarding.json` through `update_finished`. That is about 3 lines in `mail/commands.rs`, owned by F2b2b.
- Remove the `TurnedOff` arm from `mail_signals`.
- Test 40's case becomes: "Disconnect clears `mail.connect` whether or not the service delete succeeds".

## Closed

- **R2-1:** closed for the no-row cases. The off-switch half is R3-2.
- **R2-2:** closed for ownership. The placement is R3-1.
- **The fold-ins:** I1's wording, the m4 citation, Q7 option B's three gaps and n2 are all closed. Option B can now be executed.
- **n1:** mostly closed. One gap is left as a minor finding: `Queue::start` rebuilds `order` from saved state without `push_job`, so a resumed job has no `job_context` entry. Either `start` creates the entry or the worker does when it marks the job running.

## Minor

- **m-a.** Q8's "Not now" needs a command, which changes F5's list and its count. Name the command in Q8's "if yes" branch.
- **m-b.** F2b2b now edits two files. List the second file in its §7 row and re-check its size.
- **m-c.** F8b2 converts the F8b writes that have merged. Whoever later builds F8b's held registrar half writes it through the fenced helper.

## Holding

- Q7 and Q8 are framed as Quinn's decisions.
- The rulings add no new command.
- No lock is held across a network call or a window.
- No vault write happens outside `write`.
- Every test from 33 to 40 has an owner.
