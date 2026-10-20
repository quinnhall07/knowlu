# L5 plan review, round 6, delta (2026-10-20)

Plan: `docs/plans/2026-10-19-l5-syllabus-class-times-plan.md` (r6, Quinn's simplification). Reviewer, plan mode, with
the code checked against the email branch (`approvals.rs`, `sync.rs`, `cli.rs`, `write.rs`, `ids.rs`, `render.rs`).
Filed by the controller from the reviewer's returned text.

**Verdict: FAIL.** 0 critical, 4 important, 4 minor. The cross-desktop design now holds. What remains is task
interfaces, plus one cost nobody had listed.

## Confirmed

- **The r5 findings are all closed, or validly dropped with Q-L, G1 and G3.**
- **P2's convergence holds for the same hash, a different hash and a hand edit.**
  - `archive` is in `ids::NOTE_FOLDERS` (`ids.rs:23-24`).
  - A pulled record for a missing path is skipped (`sync.rs:1955-1959`).
  - Tombstones archive B's copy.
  - G2 picks the same survivor on both desktops.
- **G9 is true.** `existing_source_uids` reads `approvals/` and `archive/` whatever the status
  (`approvals.rs:1341-1357`).
- **`expires: null` with G9 leaves no permanent course block.** A card whose course note is missing stays
  pending, and that is visible.
- **"No contract change beyond T4e" holds for vault bytes.** Withdrawals write only `superseded` plus
  `write::delete`, and `VIAS` is untouched. It does not hold for interfaces (I-1).
- **The random file-name part breaks no contract** (`ids::new_id`; `sync.rs:381`'s character set).

## Important

1. **T4e's `withdraw_syllabus` cannot carry what T4d'b and T4d'a need.**
   - T4d'b's reason inputs can't reach it.
   - `Withdrawal` returns only stems and an on-disk count (`approvals.rs:1948-1951`).
   - A stale card put back to `pending` is counted on disk but not by `process_approvals` (`:1702-1705`).
   - As written, an implementer would have to edit `approvals.rs` after T4e.
   - Fix: T4e fixes the signature. It takes a caller-built `reason: impl Fn(&Mapping) -> Option<_>` and the run's
     `amendments_pending` stems, and returns the counted-pending number.
2. **A withdrawn amend card still appears in today.md.** `render.rs:381` builds Must do from
   `approvals.amendments_pending`.
   - Fix: T4d'a removes the withdrawn stems from that list. Assert it in T4dt case 9 or 10.
3. **Cards the current syllabus contradicts stay pending.**
   - Grading: h2's card stays when the syllabus moves to h3 and h3 matches the note. G8 needs a differing field.
   - Items: after d1 → d2 → d1, the d2 card stays, and a due amend card whose new day is already `due` stays.
   - These share a root with the G10 edge.
   - Fix: ask Quinn together with G10. The alternative is a withdraw-only rule when the current key or day already
     has history, or already matches the note.
4. **T4d'a's Done cannot be met.** Case 9 (G7) and case 4 (G10) belong to T4d'b. Move them.

## Minor

- **M-1.** The cap on syllabus cards per day (8) can push G8's replacement to a later day. Withdraw only when the
  replacement fits today's limit, or state the one-day gap.
- **M-2.** G2 needs a tie-break for a missing or unreadable `id:`: the file name.
- **M-3.** Convergence point 4 when A approves and B withdraws leaves different archived statuses. That is benign;
  scope P2 (f) to it. Add a same-day variant of (f), since `count_proposals_created` (`approvals.rs:1144-1162`)
  counts the other desktop's card.
- **M-4.** The G10 question to Quinn should add two things:
  - the same move can trigger G8 under another key;
  - an archived empty card is another fix, but it charges a unit to the daily budget unless `proposal_weight`
    changes, which is contract.
