# Wave B plan §14.53–§14.55 review, spec/plan mode (r1): FAIL

**Reviewer:** `reviewer`; the controller filed it. **Scope:** `git diff 3f2b454..67baa2f -- docs/plans`, checked against O3 HEAD.

## Important
- **I-1.** The advance marks step 7 as already done when the student granted Gmail but not Calendar. `earliest_unproven` goes straight from `google_granted` to step 8 or 10, so the student never sees the calendar paste. The rail then claims a calendar that doesn't exist. Test 33 pinned this as correct. No effect today, because `LiveChecks` answers false.
  - **Fix:** a step-7 rung when `google_granted && !google_calendar_granted`. The advance then asks only for `google.gmail_first_read` and lands at step 7.

## Checked and holding
- **§14.53.1.** `finish_ready` covers `gate.open`: `finish_refusal` and `status_inner` use the same gate call, state and time. The pin amendment is in order.
- **§14.54.**
  - The sign-out rewind is applied everywhere, and §14.20 is marked superseded.
  - `enqueue_once` matches the code.
  - A double-clicked Next is safe through `no_rank_owed`.
  - Skipped records are handled, and `mail.connect` excludes skipped.
- **§7.** No cycle. S4d is the only outside dependency. The sizes add up.
- **Other invariants.** No vault writes outside `write`, no lock held across the network, and no new command.

## Minor
- **m-1.** Name one managed-checks type alias, because `try_state` matches the exact type.
- **m-2.** F8d2's 115 lines need recording as accepted after the fact.
- **m-3.** F11b's frame fixes are unsized. Split them out as F11b0.

**r1 revision:** I-1 and m-1 to m-3 are folded into the plan. `google_calendar_granted` defaults to `google_granted`, so the existing fakes keep their meaning, and `LiveChecks` answers false.

---

## r2 (639c2da): PASS

**r1 findings: all closed.**
- **I-1:** the step-7 rung means a Gmail-only grant lands at step 7, and the advance asks for `gmail_first_read` alone.
- **m-1:** one `ManagedChecks` alias, with test 38's pin.
- **m-2:** recorded.
- **m-3:** F11b0 is split out and sized.

**The `google_calendar_granted` default (`self.google_granted()`)** is safe today:
- Every existing fake keeps its meaning.
- `LiveChecks` answers false.

**Minor:**
- **n-1:** the default could quietly bring I-1 back once `LiveChecks::google_granted` gains a local source. Fix: F8w2 gives `LiveChecks` an explicit `google_calendar_granted() -> false`, and the trait's doc says the default is for fakes only.
- **n-2:** test 38's pin must spare the alias's own line: exactly one occurrence, on the `pub type ManagedChecks` line, and none in `main.rs`.
- **n-3:** F11b, F11c and F11d can now run in parallel but share `onboarding.js` and `onboarding_static.rs`. Fix: give each its own file, or add a collision note and a merge order.

**§7:** no cycle. **Sizes:** within the cap. **Holding:** no vault writes outside `write`, no lock held across the network, no new command.
