# L5 plan review, round 4, delta (2026-10-19)

Plan: `docs/plans/2026-10-19-l5-syllabus-class-times-plan.md`, r4, reworked for Quinn's Q-B ruling: `grade_weights`,
`grade_scale` and `late_policy` stay last-write-wins across desktops. Reviewer, plan mode, with the code checked
against the email branch (`engine/src/sync.rs`, `approvals.rs`, `engine/tests/sync_contract.rs`,
`engine/Cargo.toml`). Filed by the controller from the reviewer's returned text.

**Verdict: FAIL.** 0 critical, 5 important, 6 minor. The `sync.rs` edit is sound. The *value moved* mechanism
needs another round.

## The questions asked

- **Is the `cardable` edit the only `sync.rs` change needed?** Yes. `sync.rs:2036` is the only reader of
  `AMENDABLE_FIELDS`, and its three uses (:2040, :2100, :2139) go through it. `sync::apply` produces the same
  bytes for every input. No sync test, run record, journal record or fixture changes.
- **Is the warning byte-identical?** :2046 is untouched, but no test pins it exactly (M2).
- **Is the re-filing actor right?** `agent:knowlu.syllabus` is right.
- **Is the cap at risk?** No engine path writes the three fields, so only hand edits and approvals trigger a
  re-file.
- **Ping-pong:** yes, through I2.

## Important

1. **The "student's write" claim is false.** A non-mail amend card applies under the executor's ctx,
   `agent:approvals` (`approvals.rs:1693`, :30-36). Only mail cards get the human actor (:905-918). P2 case (b)
   would be red.
2. **Two extraction-holding desktops duplicate or ping-pong cards** (spec :896 allows a second "Add
   syllabus").
   - Each withdraws the stale card and files its own under the same key, against spec :765.
   - If the extractions differ, *set changed* withdraws the other's card every slot, with a cap charge each
     time.
   - Fix: withdraw only cards this device filed. Add P2 case (f).
3. **Mapping values compare by key order.** `plain` formats a mapping with `{:?}` (`approvals.rs:120-124`,
   :519-527), while sync's `serde_json::Map` sorts keys.
   - Scenario: a phantom card that asks to change the weights to themselves.
   - Fix: compare key-sorted. Add a round-trip case.
4. **Where `amendment_is_stale` stops is unspecified.** It must be the from-loop only, with the
   `already_applied` gate staying in `apply_amendment` (:803-827). The per-field re-file rule must be stated.
   Add a two-field case.
5. **Re-proposing after the student's own hand edit was stated as a decision.** It is Quinn's to rule.

## Minor

- **M1.** P2 Done's red claim holds for cases 1 and 3 only.
- **M2.** Assert the exact :2046 warning for one field.
- **M3.** Amend the step-5 comment (`sync.rs:2023-2028`).
- **M4.** Pin B's stale-card-back-to-pending outcome.
- **M5.** `first_proposed_at` restarts on re-file. Say so.
- **M6.** Name the agent context for `withdraw_syllabus`.

## Rulings since

- **Controller on I1:** drop the claim. An approved card applies as `agent:approvals`, like every amend card.
- **Quinn on I5 (2026-10-19):** a hand edit after filing is the student's answer, so the card is withdrawn
  and not re-filed. A foreign agent write re-bases it.
- Being revised to r5.
