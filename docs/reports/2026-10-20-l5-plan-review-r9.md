# L5 plan review, round 9, delta (2026-10-20)

Plan: `docs/plans/2026-10-19-l5-syllabus-class-times-plan.md` (r9). Reviewer, plan mode, with the code checked on the
email branch worktree (`engine/src/cli.rs`, `approvals.rs`, `eventemit.rs`, `planning.rs`, `commitments.rs`, `mail/`)
and the spec at `docs/specs/2026-10-05-syllabus-class-times-design.md`. Filed by the controller from the reviewer's
returned text.

**Verdict: FAIL.** 0 critical, 1 important, 3 minor. All three r8 findings are closed. Of r9's own choices, the
eleventh place-10 change (b) and the restated reason for counting the 8 by `first_proposed_at` (c) are sound. Item
cards' `expires` (a) is sound on every day but the vault's first, where it combines with the hold to file cards that
expire before anyone sees them (I-1).

## Confirmed

- **r8 I-1 is closed** (:1336-1350, :1616-1626). The allowance is computed as `emit_checks` does at `cli.rs:873`,
  with `count_proposals_created` (`approvals.rs:1144-1162`).
  - (i): 4 mail + min(8, 11) = 8 + 3 events = 15.
  - (ii), pending: `defer_over_budget` keeps all 10 (allowance 15, `:1213-1215`); the hook gets min(8, 5) = 5 and the
    events pass 0. On a rerun the allowance is 15 against 15 candidates and the hook gets 0.
  - (ii), held: `:1203` skips snoozed cards, `:1596-1602` wakes them, the count is 10, so 5.
  - After a trim, `defer_over_budget` leaves the day at 15 or above, so the hook's room is never above 0.
- **The held variant can be written exactly** (:1190-1207). Step 1 plans min(8, 15 - 4) = 8; step 2 plans 0; step 3:
  no pending card, 18 woken, day 2 charged 18; step 4: allowance 15 - (18 - 18) = 15, sorted `(expires, stem)` with
  `Date::MAX` for null (`:1209`, `:1218`), and `.skip(15)` snoozes positions 16-18: the two highest-stem mail cards
  (`expires` day 16, `mail/text.rs:135`) and the grading card. The 8's day-1 exception still counts the grading card
  on day 2.
- **r8 M-1 is closed** (:522-536, :1351-1353; `eventemit.rs:780-793`, `write.rs:988`/`:1079`). **r8 M-2 is closed**
  (:1627-1639, :2222-2237).
- **(b) is consistent with the spec's intent.** Q21 and the risk row say "inside the 15" (:1242, :1267); §16.8's
  "nothing changes the 15 or `defer_over_budget`'s order" (:1763) still holds. A syllabus item is re-planned from the
  extraction, so leaving it unfiled loses no judgment, which is what "never dropped" protects (`approvals.rs:1167-1168`).
- **(c) is sound** (:1323-1330). Counting by `proposed_at` would move a snoozed card into the 8 of the day it wakes.
- **Done is reachable, the order holds, and nothing after T4e edits `approvals.rs`.** `read_inputs` needs only
  `count_proposals_created`, `load_planning` (`planning.rs:65`), `vault_day` (`commitments.rs:1483`), `sorted_md` and
  `as_date` (both `pub(crate)`). T4d → T4e → T4d'a is unchanged (§5 :828-830).

## Important

1. **On the vault's first day the filer plans items due that day, and they expire unseen on day 2** (T4d "The 15" and
   "`expires`" :1336-1353; T4dt (A) :1151-1158; Q-B :526-533).
   - The horizon's "never once past" is measured from today (spec :691). On day 1 a card is held to day 2, but its
     `expires` is its item's day. On day 2 `process_approvals` wakes and expires it in the same call
     (`approvals.rs:1599-1627`).
   - Q41 puts the soonest items first, so these cards take the first slots of day 2's 8 and 15, and their `expired`
     history blocks the key (M-1).
   - Scenario: a student onboards on a Monday with three assignments due Monday night. Three of day 2's 8 slots (and
     15) go to cards nobody sees, so three items due days 3-9 slip a day. This also falsifies Q-B's reason
     (:530-531), "the horizon never files a past item".
   - Fix: on vault day 1, measure "never past" and the horizon from the charge day. Add a T4dt (A) case: an item due
     on day 1 plans no card; one due on day 2 plans a held card.

## Minor

- **M-1. The expired-history edge (a) is acceptable, with a cheaper alternative (:533-536, §7 :2234-2237).** It does
  not break G8 (grading cards keep `null`); G11 acts only on pending or snoozed cards; expiry is by date, so every
  desktop agrees; an expired card is archived the next day, so no stale card stays visible. Cost: an exam rescheduled
  after its original date, on a card the student never answered, gets no card. Offer Quinn the alternative in §7:
  `expired` item history unblocks the key, as `superseded` does, under the twin's day rule (the filed day stays
  blocked, so no loop) — a twelfth place-10 change (spec :713-716).
- **M-2. `PlannedCard` does not name the fields (A) asserts (:1146 vs :1157-1158).** It names `kind`, `source_uid`,
  `proposed_at` and `snooze_until`; (A) also asserts `expires` and `first_proposed_at`. Name both.
- **M-3. Three claims overreach.** (1) "Reaches its day with no card only if every run in its 14 or 21 days is full"
  (:1638-1639, :2233): an item that enters the horizon late has fewer runs (a syllabus added mid-term with a quiz in
  2 days), and place 10's "an item that does not fit is not dropped" (:711) would put the absolute form into the
  signed spec; bound it the same way. (2) "As Q41 orders the 8" (:532-533): Q41 puts amend cards second, but
  `defer_over_budget` puts `null` last; the held variant itself snoozes the grading card while keeping the day-9 task.
  (3) "`emit_checks`' form" (:1339-1340) counts today only; the charge-day variant is new, so cite it as such.
