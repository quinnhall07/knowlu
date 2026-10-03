# Email forwarding: Checkpoint A contract review

- Date: 2026-10-02
- Reviewer: `contract-reviewer` (Opus, xhigh), read-only; final re-check of 4194817
- Scope: T5.1 to T5.5, T8, T8b and T8c, with two fix rounds. Commits: b697cf8, 4e4c873, 968d86d (round 1),
  91f77f6, fa980a1, 5793cfd (round 2), 4194817 (final fix)
- Source of round-by-round facts: `.superpowers/sdd/2026-09-30-email-forwarding/progress.md`
- Verdict: 0 Critical, 0 Important, 3 Minor.

## Gate

- `cargo test --workspace`: 2445 pass, 0 fail, 4 ignored (the four by design).
- Contract-list diff against main: `engine/src/approvals.rs` only (T8 and T8c, which the plan allows).
  `engine/tests/fixtures/**` untouched.

## Rounds

- Round 1 (b697cf8, 4e4c873, 968d86d): F2 planned-note amendment, T8 (mail amend as the executor's actor, title
  only on `created_by: blackboard`), T8b (per-field cards for the feed). Workspace 2413 pass.
- Round 2 (91f77f6, fa980a1, 5793cfd): T8c committed, a snoozed amend card kept as the open decision on its field,
  T5.5 built whole, the PQ4 probe added. Workspace 2443 pass.
- Final (4194817): an un-archive's status is the executor's and its path the first free suffix. Workspace 2445 pass.

## Minor findings

1. and 2. Two tests for an interrupted un-archive are missing: the move done with the status unwritten, and the
   write done with the move unfinished. The write order is correct as read; the tests would pin it.
3. The plan's Status line was stale (it named an earlier HEAD). Corrected in the same docs commit.

## Quinn's confirmations, 2026-10-02

- Checkpoint A: PASSED.
- T8c arm (1) is keyed on the card: it fills any dateless task, hand-made ones too, by card. Confirmed.
- The PQ4 probe (the unknown-kind test probes `completed` and asserts `receipt` deserializes) is approved.
- Also answered the same day: PQ6 (a), PQ7 (count residual NotAmendable in U), PQ8 (ii) only. Recorded in the plan
  and spec; T5.6 follows A, as an addendum read while T6.1 runs.
