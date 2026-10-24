# The evening ranking rule (ruled 2026-10-24)

**Status: RULED by Quinn, 2026-10-24.** Quinn ruled the behaviour on 2026-10-23 and approved all
four of this note's recommendations on 2026-10-24 (§5). The file keeps its `-draft` name so links
hold. **Not part of wave B** (O3/O4): no task in
`docs/plans/2026-10-23-wave-b-long-onboarding-plan.md` builds it. Its plan is
`docs/plans/2026-10-24-evening-ranking-plan.md`, `test-writer` first, because it changes the order
`rank` produces.

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
- A task due today always has slack ≤ −1 (the one-day buffer puts its start-by at yesterday or
  earlier), so it is already Must do. A task more days overdue scores higher, which is why the late
  lab leads at 11 pm.

The rule adds one ordering step, **from the hour on only**:
- **"Closing tonight"** is any task `rank` receives whose last useful moment falls after the moment
  and before local midnight (§2, ruling d):
  - a task whose `due` is on the moment's date at a time still ahead (strictly later than the
    moment); a bare-date `due` already reads as 23:59 (`models::coerce_datetime`);
  - a past-due task whose **late window** closes tonight (O6's window, §3).
- **From the hour on,** closing-tonight tasks come first, by that closing time, earliest first; ties
  keep the pressure order (a stable sort over the scored list). Every other task keeps the pressure
  order below them.
- **Before the hour,** nothing changes: the order is exactly today's.
- Must do is unchanged as a set (slack ≤ 0). The rule changes order only, never membership. Since
  every closing-tonight task is already Must do, the order of everything else, Recommended included,
  is unchanged, and the designation sees the same tasks.
- `rank_override` keeps winning: overrides are inserted after the scored order, as today, so the
  evening step runs on the scored order before them.

**Its input is the moment, not just the day.** `rank` gains a moment (a wall-clock time in the
vault's zone), passed by its callers: the `rank` command at slot time and `surface`'s loader at view
time. The step runs only when the moment's date is the day being ranked and its time is at or after
the hour. A caller that pins a day without a time (`rank --today D`, as the golden harness does) has
**no moment**, so it gets today's order. A caller that pins only a moment (`rank --now T`) ranks the
day of that moment (controller's call, 2026-10-24). Beyond the day, that works exactly as `--today`:
the run's other passes date their writes by it (the info pass, the approvals budget and pass, the
proposal count), so `rank --now` is a real run, never a dry run. The engine stays deterministic: the same notes, day and
moment give the same order. No model is involved (decision 11). Nothing is written that is not
written today: no new note field, no new run-record count.

**Where students see it.**
- **The live page** re-ranks on every read (`surface::load_with` calls `rank`), so the console shows
  the evening order as soon as the hour passes (ruling c).
- **`state/today.md`** gets no evening rewrite (ruling c). The 12:00 and 18:00 slots write it before
  the hour, so it keeps the daytime order. A slot that happens to run at or after 20:00 (a catch-up
  after the machine slept) writes the evening order, the same order the live page shows. Nothing
  schedules such a run.
- **The console's Must do block** shows its groups (`Overdue`, `Due today`, `Due <day>`, …) in the
  order their first task ranks, so from the hour on `Due today` leads `Overdue`. Within a group the
  console sorts rows by start-by, then due (`surface::sort_rows`). Whether `Due today` sorts by due
  time after the hour is the plan's question Q-1.
- **The capacity meter's spill list follows the evening order too, by design** (controller's call,
  2026-10-24). `surface::meter` fills the day's capacity with Must do work in rank order and lists
  what does not fit. With 3h left, a 3h overdue lab and a 1h task due tonight, the lab fits and the
  task spills before 20:00; from 20:00 the task fits and the lab spills. The spill hours and block
  counts do not change, only which task is named.

## 2. The hour (ruled a and d)

**Ruled (a), 2026-10-24: 20:00 in the vault's zone, as one engine constant** (`EVENING_HOUR` in
`ranking.rs`). A per-vault `planning.yaml` key (for example `evening_hour`) can come later.
- **Why 20:00:** students do evening homework from about 8 pm to midnight, and an 11:59 pm deadline
  from 8 pm leaves four hours, about one session plus a buffer. Later (22:00 or 23:00) leaves too
  little time to act on the reordering; earlier starts to override slack-based planning in the
  afternoon, where pressure already handles a due-tonight item.
- **Why a constant now:** one value, one test, no new config key. A hard-coded value tuned to one
  student's habits would be a single-user assumption, so the pilot checks it against more than one
  student before it is treated as settled; the `planning.yaml` key is the follow-up if students
  differ.
- **Not taken:** a relative trigger (active when the hours left before tonight's deadline fall below
  the task's work left plus a buffer). It fits each task better, but Quinn ruled "after a set hour".

**Ruled (d), 2026-10-24: "tonight" ends at local midnight**, in the vault's zone, even for a student
whose planning-day window runs past midnight. Due times are calendar times, and 11:59 pm is the case
in hand. At 00:30 the moment is a new day and before the hour, so the daytime order returns.

## 3. How it interacts with Late and the window cost (O6)

O6 (onboarding spec §5.2, Q6) gives a past-due task a **late window** from its course's confirmed
`late_policy`. Inside the window the task stays Must do with its cost label ("−20% now; closes
Friday"); past it, the task leaves Today into "Late window closed".
- **A late task whose window closes tonight** is itself closing tonight. It joins the first group,
  ordered by its window's close against tonight's due times.
- **A late task whose window runs later** (Wednesday, in the 5b example) ranks **below** tonight's
  items after the hour. It stays Must do and keeps its label, so the student still sees its cost.
- **A late task with no known window** (no confirmed `late_policy`) has no close time and is **not**
  closing tonight. An on-time submission tonight is a certain grade; an unknown late rule is not.
  Before the hour it ranks first, as today.
- **Penalty steps (ruled b, 2026-10-24): out of v1.** A rule like "−10% a day" also gets worse at each
  day boundary after the due time, which can fall tonight. Only the window's **close** counts.
- **O6 is not built** (wave C). Until it ships, every past-due task has no known window, so the late
  half of the rule changes nothing. This lane builds the due-tonight half and leaves one place for
  O6's close time; O6's own plan wires it, with its own test.
- **Display.** The 5b page already gives a late item its own "Late · window closes" cell. The rule
  changes which row leads at night, not how a row reads.

## 4. What it must not break

- **Frozen references.** `golden-today-s1.md` and `golden-today-full.md` are Python-written and are
  never regenerated; the rule leaves them byte-identical. The golden harness (`engine/tests/
  oracle.rs`) runs `rank --today 2026-08-28` with no time, so under this rule it has no moment and
  ranks as daytime, whatever the wall clock says. A second guard holds as well: no fixture task is
  due on 2026-08-28, so even an evening moment on that day leaves the order unchanged. The plan's
  first task proves both before any code changes.
- **Read-model references.** `surface-today-{s1,s1-migrated,full}.json` are pinned at
  `--now 2026-08-28T09:00`, before the hour, and the same fixtures have nothing due that day. They
  stay byte-identical; under console spec §4.6 they would be regenerated only in a commit whose diff
  shows the change and says why, and this rule gives no reason to.
- **The engine invariants.** `rank` stays pure and model-free, the order is deterministic for a given
  day and moment, and no vault file gains a field.

## 5. Quinn's answers (2026-10-24)

a. **The hour:** 20:00 in the vault's zone, as an engine constant first; a per-vault `planning.yaml`
   key may follow (§2).
b. **Penalty steps tonight:** out of v1; only a late window's close counts (§3).
c. **`state/today.md` at night:** no evening rewrite and no evening slot; the live page re-ranks on
   read and carries the evening order (§1).
d. **"Tonight" past midnight:** ends at local midnight (§2).
