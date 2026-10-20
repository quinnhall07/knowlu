# L5 plan review, round 8, delta (2026-10-20)

Plan: `docs/plans/2026-10-19-l5-syllabus-class-times-plan.md` (r8). Reviewer, plan mode, with the code checked on the
email branch worktree (`engine/src/cli.rs`, `approvals.rs`, `mail/`) and the spec at
`docs/specs/2026-10-05-syllabus-class-times-design.md`. Filed by the controller from the reviewer's returned text.

**Verdict: FAIL.** 0 critical, 1 important, 2 minor. All six r7 findings are closed. Of the planner's three
choices beyond its brief, counting the day's 8 by `first_proposed_at` and the G8 rule are sound. The hook's line
is right, but the reason the plan gives for it is wrong (I-1).

## Confirmed

- **r7 I-1 is closed.** T4dt names `read_inputs` and `plan` (:1104-1111). Group (A) holds only cases `plan` alone
  turns green and is T4d's whole Done (:1271-1274); group (B) is T4d'a's Done (:1562-1565). No (A) case needs T4e or
  the hook.
- **r7 I-2 is closed.** One reading of "no longer gives its value" in G11 (:454-472), T4d'b (:1617-1621) and T4d'c
  (:1571-1574). Cases 6 and 10 cover it, and it is reported to Quinn as Q-B's fourth cost (:546-557) and in §7
  (:2109-2112).
- **r7 M-1 is closed.** Only comments sit between `cli.rs:430` and `:439`.
- **r7 M-2, M-3 and M-4 are closed** at :481-485, :1456-1460, :1484-1491, :1308-1314, :678-684 and §5 :798.
- **(a) The day's 8 by `first_proposed_at` is correct.** Spec §16.8 says "first proposed a day" (:692, :1757). A
  card's charge follows `proposed_at` (`approvals.rs:1156`), which `defer_over_budget` moves forward (`:1227`), so
  each card charges exactly one day's 15. The 8 counts each card once: on its `first_proposed_at` day, or day 2 for
  a card filed on day 1. No card is uncharged or charged twice; a card withdrawn the day it was filed still charges
  from `archive/` (`:1136-1141`). The day-1 exception matches the mail hold (`mail/tests/mod.rs:47`, `cards.rs:62`).
- **(b) G8 converges and loses no field.** A card G8 can replace has every field still at its `from` (else G7
  withdraws it first), so G10 carries each field onto the replacement with the same `to`. With a partial read on A
  and a full read on B, B replaces *c*; on A the new card is neither unsupported nor replaceable and A's narrower
  key is blocked, so both desktops end on B's card in either order. Room that differs before a sync still ends with
  one card, through G9 and G2.
- **Done stays reachable and the contract list closes after T4e.** T4d'a, T4d'c and T4d'b need only public items
  (`count_proposals_created` `approvals.rs:1144`, `existing_source_uids` `:1341`, `sorted_md` `:365`,
  `amendments_pending` `:274`) plus T4e's `withdraw_syllabus` and `amendment_is_stale`.

## Important

1. **The syllabus filer is not held to the 15, so the hook line's stated reason is false (T4d'a :1526-1536; §10
   Round 8, row M-1).**
   - The plan says "one run charges no more than the 15 through sizing that already exists" and "no existing card is
     snoozed for them". Only the events pass (`cli.rs:440`, `:497`) and `commitment_passes` (`:873`, `:881`) size
     themselves to what is left; the filer is capped at the day's 8 only (T4d :1253-1263). The plan's own case (ii)
     charges 18 in one run.
   - Scenario: the mail step files 15; `defer_over_budget` (`:423`) keeps all 15; the hook files 8 more. The day is
     charged 23, the events pass gets 0, and `today.md` shows 23 new cards. If that was the day's last slot it stays
     at 23, since `defer_over_budget` reads only `proposed_at == today` (`:1206`). If a second slot comes,
     `defer_over_budget` snoozes 8 in its own order (`:1217-1223`), and mail cards (`expires` 14 days out,
     `mail/text.rs:135`) can go ahead of syllabus cards.
   - Fix: keep the line, and have `plan` file at most the smaller of what is left of the day's 8 and
     `max(0, budget - count_proposals_created(vault, charge_day))`, where `charge_day` is today, or day 2 on the
     vault's first day (`emit_checks`' sizing at `:873`). Restate case (ii) (10 mail cards and an extraction on day 2
     give 10 + 5 = 15), keep a variant for held day-1 cards that `process_approvals` wakes after
     `defer_over_budget`, and correct the two claims. The spec allows this ("8 a day, inside the 15", :1267).

## Minor

- **M-1. Case (ii) cannot be written exactly (:1137-1138).** The plan never states an item card's `expires` (:514
  says only "unchanged") and the spec has none; the grading card's `null` is snoozed first. And "soonest `expires`
  first" reads as the snooze order, but `defer_over_budget` keeps the soonest and snoozes the latest (`:1218`,
  `.skip(allowance)`). Fix: state item cards' `expires` and name the three cards snoozed.
- **M-2. Where the hook's cost is reported, and what it is.** T4d'a says "reported to the controller" (:1536), but
  §7 item 2 puts it on Quinn's list (:2113-2114); make them agree. Give the concrete cost: at the start of a term
  about 30 items in the horizon take 8 a day for four days, and an obligation event inside those days can get no
  card before it happens, because the events pass files only while the event is still ahead. Under I-1's fix the
  same cost holds, capped at 8 a day.
