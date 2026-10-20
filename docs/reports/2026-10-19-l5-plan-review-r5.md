# L5 plan review, round 5, delta (2026-10-19)

Plan: `docs/plans/2026-10-19-l5-syllabus-class-times-plan.md` (r5). Reviewer, plan mode, with the code checked
against the email branch: `approvals.rs`, `sync.rs`, `journal.rs`, `reconcile.rs`, `passes.rs`, `cli.rs`,
`Cargo.lock` and `sync_contract.rs`. Filed by the controller from the reviewer's returned text.

**Verdict: FAIL.** 0 critical, 5 important, 5 minor.
- **Closed at the cited lines:** I1, I3, I4, I5 and M1-M6.
- **Partly closed:** I2. The Q-L ruling is not written in, and G1 plus G2 can block a course for good.

## Confirmed in the code

- **`device`.** It is on every record (`journal.rs:288`). A pulled record without it is refused
  (`sync.rs:864`), and pulled records are appended unchanged (`:1825`).
- **`preserve_order` is off.** There is no `indexmap` in `Cargo.lock`. Sorting at every depth is safe for the
  three fields, and `comparable`'s other callers either never see them or benefit.
- **No byte change for existing data.** The three fields are not amendable today.
- **G3 reads the pulled record's own actor,** so a sync echo cannot hide a human write. `detect_external`
  (`cli.rs:395-407`) runs before the withdrawals (`:846`).

## Important

1. **Q-L is not in the plan** (:341-362, :1013-1015).
   - G1 still names two reasons, not four, and the inputs (:1029-1031) lack the card's `create` record and a
     `this_device: &str`. Reading the environment would break `DEVICE_ENV_MUTEX` (`journal.rs:85-94`).
   - The extension to *date moved* and *authoritative match* has no tests. It allows two cards for one item.
2. **G1 plus G2 can block a course for good.** Amend cards carry `expires: null`, read as never
   (`approvals.rs:1209`).
   - A rename or a new laptop makes every pending card foreign. G1 never withdraws them, and G2 refuses every
     later grading card for the course.
   - Fix: a finite `expires`, or an orphan rule. Test the rename.
3. **A converged card is filed again** (:978-979, :864-866, G4 :1025-1028). After a converged withdrawal the
   key holds only `superseded`, so the next `rank` files `from == to`.
   - Fix: every filing drops fields the note already holds, and files nothing if none remain.
4. **P2 (f) cannot pass** (:986-991). A re-file is a new id with a cap charge, yet the case asserts one id and
   no charge.
5. **T4d' is about 150 lines,** past its own ~120-line stop rule.
   - Fix: split it into T4d'a (`cli.rs`: hook, context, G1, G2) and T4d'b (`syllabus_cards.rs`: G3, G4, key
     block).

## Minor

- **M1.** "Human" should be `journal::is_human`, not `!is_agent`: `system:*` actors would count as answers.
  Consider ignoring a human `set` that never took effect (`sync.rs:1996`).
- **M2.** T4dt case 1 should edit the file itself, to cover `detect_external`.
- **M3.** A snoozed card that is re-based loses its snooze.
- **M4.** Two desktops that file before either syncs leave two pending cards (spec :765).
- **M5.** P2 case 1's warning must match `sync.rs:2046` with no markdown escapes.
