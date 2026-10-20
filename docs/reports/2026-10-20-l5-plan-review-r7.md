# L5 plan review, round 7, delta (2026-10-20)

Plan: `docs/plans/2026-10-19-l5-syllabus-class-times-plan.md` (r7). Reviewer, plan mode, with the code checked on
the email branch at `af14914` (`approvals.rs`, `cli.rs`, `render.rs`) and the spec at
`docs/specs/2026-10-05-syllabus-class-times-design.md`. Filed by the controller from the reviewer's returned text.

**Verdict: FAIL.** 0 critical, 2 important, 4 minor. Every r6 finding is closed, and the rulings are applied
soundly except where I-2 reads one ruling two ways.

## Confirmed

- **r6 I-1 is closed (:1164-1183).** The "counted" rule matches the code:
  - an amend card counts only while pending (`approvals.rs:1633`, `:1656`);
  - an approved card the stale check puts back is pending on disk but was never counted (`:1702-1705`);
  - a woken card is pending on disk and was counted (`:1601-1602`).
  - The stem argument is needed: the decision is computed first and keyed by stem.
  - `Withdrawal`'s shape is unchanged (`:1948-1952`), and no later task edits `approvals.rs` (§4.2 :677, §5).
- **r6 I-2 is closed.** `render.rs:381` reads `amendments_pending`, T4d'a removes each withdrawn stem
  (:1395-1396), and T4dt case 10 asserts it.
- **r6 I-3 and I-4 are closed.** I-3 is now G11, per the ruling; I-4's cases moved to T4d'c's and T4d'b's Done.
- **r6 M-1 to M-4 are closed.** M-1 (G8 waits for room) is sound and the pure plan has no loop: withdrawn cards
  still count toward the day's 8 from `archive/`, so a withdrawal never frees room the plan assumed.
- **G11 converges on two desktops.** P2 (f) to (j) and the same-day variant hold: filing is bounded by G9 and the
  twin, every other rule only withdraws, and the note half reads only synced data.
- **G2 deciding only where the extraction is, is sound.** Duplicates on a desktop without the extraction are
  visible and harmless: approving both makes the second stale, and G7 withdraws it.
- **Task order and sizes hold.** T4c and T4d touch separate files; T4d'a → T4d'c → T4d'b → P2 run in sequence.

## Important

1. **T4d's Done cannot be met (:1146; §5 :741).**
   - T4d's files are `syllabus_cards.rs` and `lib.rs`, but the spec's T4d row also held the `cli.rs` hook (spec
     §13). T4dt's filer cases go through `rank`: the sum of 15 with event and mail cards needs the events pass
     (`cli.rs:439-526`), and the course notice and the read-back of written cards need the hook. T4d's Done
     defers only the withdrawal cases, so its reviewer checkpoint stalls (the r6 I-4 failure again).
   - Fix: T4dt names the pure plan's function so the filer's plan cases can be written before T4d; T4d's Done is
     those cases only; the filer's `rank`-level cases move to T4d'a's Done (:1403).
2. **G11's "the syllabus no longer gives its value" is read two ways (:439-445, :1446-1448).**
   - Grading counts "the set gives no valid value" as *unsupported*; items count only a moved day, and a dropped
     item stays (spec :744-745, :766-767, Q15's "a syllabus edit is not evidence"). Neither choice is recorded.
   - Consequence: one desktop's re-extraction fails the all-or-nothing grading check, withdraws *c*, the tombstone
     archives *c* everywhere, and when a later file gives the same set back G9 blocks its key. The course's grading
     card is lost for good to a model failure, not a syllabus change.
   - Fix: one reading for both halves. Recommended: an absent value (a dropped item, or an invalid grading part) is
     not *unsupported*, by Q15. Otherwise state why grading differs. Either way, report it to Quinn as a decision
     taken (Q-B costs, :495-503).

## Minor

- **M-1. The hook's position against the events pass is not pinned (:1377-1380).** `remaining_budget` is set at
  `cli.rs:440` and `commitment_passes` counts the budget again at `:873`. Before :440 the hook shrinks the event
  cards' budget in the same run; after the events block, syllabus cards over the 15 are snoozed next run. The spec
  says nothing changes the 15 or `defer_over_budget`'s order, and T4dt's sum-of-15 case passes under only one
  placement. Fix: name the exact line in T4d'a.
- **M-2. "Closes a latent double approval" is overstated (:2206).** `process_approvals` (`cli.rs:430`,
  `materialize` at `approvals.rs:1678`) runs before the hook, so a card approved on B before the `rank` that follows
  the sync still creates a second task or `exam` commitment. Fix: drop the claim, or have P0 item 6 decide whether
  T4e's arms skip when a note already carries the key (settled before T4e, since `approvals.rs` closes after it).
- **M-3. Two P2 cases are underspecified.** (f), different hashes (:1322-1326): if only one card carries the
  differing field the outcome depends on the random ids; state that the note holds neither value. (i), a due
  already there (:1346-1348): state that A's `rank` runs after the sync, or A's tombstone archives B's copy.
- **M-4. Two texts still describe the old count and reason.** `Withdrawal`'s doc comment
  (`approvals.rs:1944-1946`) names `withdraw_stale` and "pending on disk"; the spec's T4e row still takes the reason
  from `syllabus_cards::withdrawal_reason`. Fix: T4e amends the doc comment; place 10 records the caller-built
  reason and the counted rule.
