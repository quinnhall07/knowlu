# The evening ranking rule (DRAFT, 2026-10-23)

**Status: DRAFT. Not part of wave B** (O3/O4). Quinn ruled the behaviour on 2026-10-23; the hour
and the details below are open until Quinn signs this note. No task in
`docs/plans/2026-10-23-wave-b-long-onboarding-plan.md` builds it. It needs its own plan, with
`test-writer` first, because it changes the order `rank` produces.

**Where it came from.** The 5b Departures direction's daily page at 11 pm ranks a late lab (window
open until Wednesday) above two items due at 11:59 pm tonight
(`docs/design/2026-10-23-directions/README.md`, "Decide the 11 pm order"). Quinn's ruling: **at
11 pm, items closing tonight outrank a late item whose window runs later; after a set hour, items
due tonight rank first.**

## 1. What changes in `rank`

Today (`engine/src/ranking.rs`):
- `rank` orders by `pressure`, which is driven by slack in days, so an overdue task (negative slack)
  always scores above anything not yet due (`100 + |slack| × 10` against `50 / (1 + slack)`).
- `partition_must_do` makes every task with slack ≤ 0 a must-do.
- Neither function knows the time of day: both take `today: Date`.

The rule adds one ordering step, **after the hour only**:
- **"Closing tonight"** is any live task whose last useful moment falls before the end of today in
  the vault's zone:
  - a task not yet due, whose `due` is today at a time still ahead;
  - a past-due task whose **late window** closes tonight (O6's window, below).
- **From the hour on,** closing-tonight tasks are ordered first, by that closing time, earliest
  first; ties fall back to today's pressure order. Every other task keeps today's pressure order
  below them.
- **Before the hour,** nothing changes: the order is exactly today's.
- Must do is unchanged as a set (slack ≤ 0). The rule changes order only, never membership, so
  Today's must-do block and the designation's budget arithmetic see the same tasks as before.
- `rank_override` keeps winning: overrides are inserted after the scored order, as today.

**Its input is the moment, not just the day.** `rank` gains a `now` in the vault's zone, passed by
its callers (`cli.rs`'s `rank` command at slot time, `surface::load_with` at view time). The engine
stays deterministic: the same notes and the same `now` give the same order. No model is involved
(decision 11). Nothing is written that is not written today.

**Where students see it.** The 12:00 and 18:00 slots write `state/today.md` before any likely hour,
so the file keeps the daytime order. The live page re-ranks on every read (`surface::load_with`
calls `rank`), so the console shows the evening order as soon as the hour passes. Open item c asks
whether `today.md` needs an evening rewrite too.

## 2. The open parameter: the hour

*Recommend: 20:00 in the vault's zone, as one engine constant first.*
- **Why 20:00:** students do evening homework from about 8 pm to midnight, and an 11:59 pm deadline
  from 8 pm leaves four hours, about one session plus a buffer. Later (22:00 or 23:00) leaves too
  little time to act on the reordering; earlier starts to override slack-based planning in the
  afternoon, where pressure already handles a due-tonight item.
- **Why a constant, not a setting now:** one value, one test, no new config file key. A per-vault
  `planning.yaml` key (for example `evening_hour`) is the follow-up if the pilot shows students
  differ. A hard-coded value tuned to one student's habits would be a single-user assumption, so the
  pilot should check it against more than one student before the value is treated as settled.
- **Alternative:** a relative trigger instead of a clock hour, active when the hours left before
  tonight's deadline fall below that task's work left plus a buffer. It fits each task better, but
  it is not what Quinn ruled ("after a set hour"), so it is noted only.

## 3. How it interacts with Late and the window cost (O6)

O6 (onboarding spec §5.2, Q6) gives a past-due task a **late window** from its course's confirmed
`late_policy`. Inside the window the task stays Must do with its cost label ("−20% now; closes
Friday"); past it, the task leaves Today into "Late window closed".
- **A late task whose window closes tonight** is itself closing tonight. It joins the first group,
  ordered by its window's close against tonight's due times.
- **A late task whose window runs later** (Wednesday, in the 5b example) ranks **below** tonight's
  items after the hour. It stays Must do and keeps its label, so the student still sees its cost.
- **A late task with no known window** (no confirmed `late_policy`, or before O6 ships) has no
  close time. Recommend: treat it as **not** closing tonight. An on-time submission tonight is a
  certain grade; an unknown late rule is not. Before the hour it ranks first, as today.
- **Penalty steps.** A rule like "−10% a day" also gets worse at each day boundary after the due
  time, which can fall tonight. This draft counts only the window's **close**, not penalty steps
  (open item b).
- **Display.** The 5b page already gives a late item its own "Late · window closes" cell. The rule
  changes which row leads at night, not how a row reads.

## 4. What it must not break

- **Frozen references.** `golden-today-s1.md` and `golden-today-full.md` are Python-written and are
  never regenerated. The rule must leave them byte-identical. The plan's first check is the
  moment the golden harness ranks at: if it is at or after the hour, the rule's `now` for that path
  must stay before it, and the test-writer proves this before any code changes. The three
  `surface-today-*.json` references follow console spec §4.6: regenerated only in a commit that
  shows the change and says why.
- **The engine invariants.** `rank` stays pure and model-free, the ordering is deterministic for a
  given `now`, and no vault file gains a field.

## 5. Open items for Quinn

a. **The hour** (§2). Recommend 20:00 local, as a constant.
b. **Penalty steps tonight** (§3). Recommend not counting them in v1; only a window's close counts.
c. **`state/today.md` at night.** Recommend leaving it to the slots and letting the live page carry
   the evening order. The alternative is an evening slot at the hour, which costs a third run a day.
d. **"Tonight" past midnight.** For a student whose planning-day window runs past midnight, does
   "tonight" end at local midnight or at the window's end? Recommend local midnight for v1. Due times
   are calendar times, and 11:59 pm is the case in hand.
