# Wave B plan §14.52, review r5 (fd82230): PASS

- **Reviewer:** `reviewer`, in spec/plan mode. The controller filed this report.
- **Checked against:** O3 HEAD (`onboarding_jobs.rs`, `onboarding_state.rs`, `mail/commands.rs`).

## Closed
- **R4-1:** F2b2c depends on F2b2b alone. S4c is now only a collision note, and F2b2b is about 72 lines in `onboarding_state.rs`.
- **n-1:** if Q8 is yes, F2b2b grows from about 72 to about 82 lines. The patient read and test 38's read half then move to F4c, which leaves F2b2b at about 57.
- **n-2:** F8h (about 45 lines) is not gated on Q7. F8h-runner (about 35 lines) is gated on Q7. F8g and F8w1 depend on F8h alone.

## The interim `Waiting("registrar_runner_pending")` arm: safe
- **No drain deadlock:** `Core::finish` records the job as pending and parks it.
- **No `job_crashed`:** the arm returns at once, and `finish` resets the count.
- **No spinning:** the job is picked again only when another job finishes or the queue is woken.
- **No trap:** Finish later and Sign out still work.

## §7 cycle walk
Every edge runs forward. The only outside dependency is S4d, which F8b already waited on.

## Minor
- **m-1.** Add a mechanical guard so O3 can't merge to main while `registrar_runner_pending` is still in `app/src`. Put it in the integrator's checklist, or make F15's live proof reach `finish_ready: true`. It must not be a CI-failing test.
- **m-2.** The interim arm should answer `Done` when there are no registrar rows. Answer `Waiting` only when rows exist.
- **m-3.** F8h-runner and F8w2 both edit `onboarding_live.rs`. Add F8w2 to F8h-runner's dependencies.

## Holding
- No vault writes outside `write`.
- No lock is held across the network.
- Q7 and Q8 remain Quinn's decisions.
- `generate_handler!` changes only if Q8 is yes.
- Every test from 33 to 40 has an owner.
