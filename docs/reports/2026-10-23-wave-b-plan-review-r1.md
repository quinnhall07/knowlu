# Wave B plan review, round 1: FAIL

**Plan:** `docs/plans/2026-10-23-wave-b-long-onboarding-plan.md` (draft of 2026-10-23).
**Reviewer:** plan-mode `reviewer`, 2026-10-23. **Verdict: FAIL**: 1 critical, 7 important, 6 minor.
No finding reopens a ruling.

> **Provenance of this file.** The controller asked for the review to be filed verbatim, but the
> message that carried it held only the controller's summary of the report, not its full text. This
> file is written from that summary, finding by finding, with nothing added. If the reviewer's full
> report surfaces, it replaces this file.

## Critical

**1. §5 Q-B1 with §4.7: a whole re-render can overwrite a restored vault's config.** The recommended
re-render of `config/campus.yaml`, `events.yaml` and `ingest.yaml` (including `course_map`) would
overwrite a restored vault's config on a C3′ restore or a second device, with no journal record. Its
only guard is "refuses once finished" (plan :588). The plan is missing a third option: each file
written once, create-new. If whole rewrites are ruled in, they need a compare-and-swap against the
step-2 default bytes and a `CLAUDE.md` amendment row (A11 covers only Q-B3).

## Important

**2. §4.6, F2, F4: quitting mid-job is unspecified.** What happens when "Open Knowlu" is pressed at the
3-minute cap while `slot.first_full`, `mail.first_pass` or `coursework.apply` runs?
`relaunch_with` (`onboarding.rs:415`) ends the process. Who finishes the "still coming" jobs? No F1
test covers it.

**3. Spec §2.5's mail rows have no task or test.** Disconnecting the school mailbox turns forwarding
off, then clears the `school_mail` use; the saved-sign-in switch off stops using `school-session`.
`mail_disconnect`, `mail_keep_session` and `mail_session::forget` are absent from S4.
`forget(address_id)` may refuse the shared folder, so the profile outlives its last use, which
breaks Q9.

**4. The cookie-isolation tests are not enough.** Assert that a request's host equals the host whose
cookies were read (`cookies_for_url` returns parent-domain identity-provider cookies). The mail
window's `INIT_SCRIPT`, its `on_new_window` popups and its navigation must stay on mail hosts. Add a
popup and navigation allow-list test.

**5. A debug port on the shared folder.** It would let CDP read every school cookie. Separately, S-G
reads `document.visibilityState` on school pages, contrary to the plan's own §3.

**6. S-F's live half cannot run.** The feed is never on disk: `summarise` works in memory.

**7. Tasks too large.** F8 and F11 (L) and S4 (M to L) exceed the ~80-line edit cap.

**8. Assignment gaps.** S6's Rust (`school_status`, `school_forget`) has no agent. F10 edits
`main.rs`, against §2.

## Minor

- F4's `LaunchMode`, S6's static test and F11's static tests have no test-first task.
- Q-B11's merge order (O4 first) departs from spec §9, where O4 depends on O3.
- Spike logging should be limited to status codes, seconds and host kind. Never a URL path.
- Q-B10's `onboarding_step` telemetry against the privacy page's telemetry sentence: the page edit
  (S7) is in the other lane.
- `campus_yaml` fails on an unknown preset (`scaffold.rs` `build_into`), so step 2 needs a named "no
  school yet" preset, tested in F3.
- Spec §2.5's "needs sign-in" after Forget names grades, the registrar and school-mail repair. S10
  checks grades only.

## Code findings checked

The plan's code findings were verified true against `02e9e73`, with these drifts:
- `scheduler::spawn` at `:1129`. It runs the first slot only in App mode with `device_ok`.
- `run_slot_inner` is at `:726` and `slot_argv` at `:515`.
- The grades window carries wry's default arguments (`debug_port.rs`, `WRY_DEFAULT_ARGS`).
- The mail window's debug port is added only when `KNOWLU_MAIL_DEBUG_PORT` is set.

## Passes

- Only `account.rs` from the contract list is touched, and it is routed to `contract-engineer` and
  `contract-reviewer`.
- No fixture changes.
- Q1, Q2, Q7, Q8, Q12 and Q13 are carried faithfully.
- No single-user assumptions.
- Duo is answered by Quinn.
- Spikes run on scratch profiles.
