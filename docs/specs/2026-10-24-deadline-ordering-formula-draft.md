# Deadline ordering: one formula for Must do (draft)

**Status: DRAFT for Quinn, 2026-10-24.** Nothing here is ruled. It answers Quinn's question on the
evening rule ("Why 8pm? Is there a general formula we can come up with to decide what should come
first based on how long it will take and when it is due?"). Quinn chose to ship the 20:00 rule now
(PR #65, `feat/evening-ranking`) and to spec the general formula to replace it later. This is that
spec. It needs Quinn's answers in §11 before a plan is written.

**Builds on:** `docs/specs/2026-10-23-evening-ranking-rule-draft.md` (ruled 2026-10-24: the hour,
penalty steps out, no evening rewrite of `today.md`, "tonight" ends at local midnight) and its plan
`docs/plans/2026-10-24-evening-ranking-plan.md`; the onboarding redesign's §5.2 (O6, the late
window, ruled Q6 2026-10-23); `docs/surface/anatomy.md` §3.4 (the meter). Ranking authority remains
`2026-08-11-personal-ops-system-design.md`; the cloud design wins where they disagree, and it says
nothing about order beyond decision 11 (`rank` never calls a model).

**Code read for this draft:** the `feat/evening-ranking` worktree: `engine/src/ranking.rs`
(`pressure`, `rank`, `rank_at`, `EVENING_HOUR`, `closes_tonight`, `partition_must_do`,
`designate_today_explained`), `engine/src/surface.rs` (`load_at`, `meter`, `must_do`,
`group_label`, `sort_rows`, `row`), `engine/src/scheduling.rs`, `engine/src/weekcal.rs`,
`engine/src/models.rs`, `engine/src/render.rs::capacity_breakdown`, and the three
`surface-today-*.json` references. The planner had no shell; the plan's first task re-confirms
every symbol against the branch it starts from.

---

## 0. The answer, short

**Order Must do work by when it is really due, then use the hours to decide two things: whether it
all fits, and how long overdue work may go first.**

1. **Deadline order.** Must do work that still has a deadline ahead is ordered earliest deadline
   first. A deadline is the due time, or, for a past-due item with a known late window (O6), the
   window's close. Earliest-deadline-first (Jackson's rule) minimises the worst lateness on one
   worker whatever the durations, so the order itself needs no duration.
2. **The fit walk.** Walk that order adding each item's remaining hours against the student's
   available time from now. When the running total passes an item's deadline, the longest item
   so far is moved out as **won't fit** (Moore–Hodgson: the fewest items late). Won't-fit items go
   to the end of Must do and are named on the meter.
3. **Overdue work goes first while it costs nothing.** A past-due item with no known window has no
   deadline left to meet. It goes as early as it can **without making any on-time item late**, with
   a 30-minute margin. When nothing is tight, that is first, as today.

It runs only when `rank` has a moment (the live page, timed runs). With no moment (`rank --today`,
the golden harness) the order is today's, byte for byte.

**Why 8 pm, then?** There is no fixed hour. For an overdue 3-hour lab and a 1-hour task due at
11:59 pm, the lab leads until 7:29 pm and yields after; for a 1-hour lab the change comes at
9:29 pm (§2.8). The hour is when the overdue work stops fitting in front of tonight's.

**A correction to the proposal as it reached the planner.** The proposal said "an overdue item with
no late window keeps leading". That would undo the ruled evening spec's §3, where a no-window late
item ranks **below** tonight's items after the hour ("an on-time submission tonight is a certain
grade; an unknown late rule is not"). Step 3 is the refinement that keeps both: it leads when it
fits, and yields when it would cost tonight's work. Quinn rules this in Q1.

## 1. What exists today

- `ranking::pressure` is driven by slack in whole days (`scheduling::slack_days`, from `start_by`'s
  backward walk over daily capacity, capped at the session slice, minus a one-day buffer). Overdue
  starts score `100 + |slack| × 10`, the rest `50 / (1 + slack)`, each times `0.6 + 0.1 ×
  importance`. `rank` sorts by pressure (the preserved Python quirk on ties) and inserts
  `rank_override`s by position.
- `partition_must_do`: slack ≤ 0 is Must do. Duration decides membership only through `start_by`.
- PR #65 adds `rank_at(tasks, today, cal, moment)`: from `EVENING_HOUR` (20:00) on, tasks whose due
  falls on the moment's date and after it lead, by due time; everything else keeps pressure order;
  overrides still win. `rank` is `rank_at(.., None)`.
- `surface::meter` fills the **whole day's** capacity (`capacity_breakdown`, not what is left from
  now) with Must do work in rank order, skipping what does not fit; the skipped tasks are
  `spill_items`. `spill_hours` and the block counts do not depend on order.
- `surface::must_do` groups Must do rows by `group_label` (`Overdue`, `Due today`, `Due <day>`,
  `Later`) in the order each label first appears, then re-sorts each group by start-by, then due
  (`sort_rows`).

None of this uses the time of day except PR #65's step, and none of it asks whether the Must do
work can all be finished before its deadlines.

## 2. The formula

### 2.1 When it runs and on what

- **Only with a moment `m`** whose date is the day being ranked (the guard PR #65 uses). No moment,
  or a moment on another day: the order is exactly `rank`'s.
- **Only on Must do** (slack ≤ 0, unchanged), and only on items without a `rank_override`.
- **It permutes Must do in place.** The scored list's Must do items are reordered; the positions
  they occupied are refilled in the new order; every other task keeps its exact index. Overrides are
  then inserted as today. So Recommended, the designation's input order and every non-Must-do
  position are untouched by construction, not by coincidence.

### 2.2 Deadlines

Each Must do item, at moment `m`, is one of:

| Class | Which items | Deadline |
|---|---|---|
| **On time** | `due > m` | `due` (a bare date reads as 23:59, `models::coerce_datetime`) |
| **Late, window open** (after O6) | `due ≤ m`, a confirmed `late_policy` whose window closes after `m` | the window's close; penalty steps out (evening ruling b) |
| **Late, no window** | `due ≤ m` and no known window: every past-due item until O6 ships | none |

An item due exactly at `m` has closed (as `closes_tonight` treats it). An item past a closed window
leaves Today under O6; until O6 it is "late, no window". An undated task is never Must do. **The one
place O6's close goes** is the deadline function, as `closes_tonight` is today.

In what follows, the first two classes are **dated** and the third is **overdue**.

### 2.3 Available time

The walk needs "how much of the student's time lies between now and this deadline". One timeline
serves every item, so that the scheduling theory behind steps 1 and 2 holds exactly (a timeline per
item would break it: a later deadline could end up with less time than an earlier one).

**The timeline**, from `m` forward, day by day to the last deadline (at most 365 days, `start_by`'s
bound):
- each day's free blocks as the week calendar computes them (`WeekCalendar::free_blocks`: inside
  the planning-day window, net of template classes, confirmed commitments and calendar events, the
  minimum-block filter applied), clipped to start at `m`;
- **plus the evening before a deadline.** If a dated item is due on day D after D's window ends, D's
  window is extended to the latest such due time that day. If one is due on day D before D's window
  opens, the evening of D−1 is extended to midnight (evening ruling d: tonight ends at midnight).
  Busy time in the extension still subtracts. `WeekCalendar::with_day_window` already widens one
  day's window, so this is the likely mechanism; the plan confirms it.

`A(d)` is the minutes of the timeline before `d`. It never grows as `m` advances, so with no
progress and no new estimate, a "fits" can turn into "won't fit" but never the reverse.

**Why not the window alone, or the wall clock** (Q3):
- **The planning window alone** is the student's stated day, but it ends before most homework
  deadlines. The fixture template's day ends at 18:00; at 20:00 it would leave no time before 23:59,
  so everything due tonight would read "won't fit". That is false, and it is the evening Quinn asked
  about.
- **The wall clock** (every minute to the deadline, minus busy time) never says "won't fit" wrongly,
  but it plans as if the student never sleeps, and it lets overdue work go first on time that does
  not exist.
- **The window plus the evening before a deadline** says what students do: they work up to a
  deadline on the night it falls. It is still conservative on other nights. A student who stays up
  most nights may see "won't fit" early; the planning-day window is theirs to widen.

### 2.4 Step 1: deadline order

Sort the dated items by deadline, earliest first. Ties keep their order in the scored list (pressure,
with its preserved quirk, then input order).

Jackson's rule: on one worker, earliest deadline first minimises the maximum lateness, and if any
order meets every deadline, this one does. Durations do not change it. That is the direct answer
to Quinn's question: **for the order alone, only the deadline matters; duration matters for whether
it all fits, for what gives if it does not, and for how long overdue work may go first.**

### 2.5 Step 2: the fit walk

Walk the deadline order keeping a running total `t` of work minutes. Add each item's work. If
`t > A(d)` for the item just added, remove the item with the most work among those kept so far
(ties in §5), subtract its work from `t`, and mark it **won't fit**. Continue to the end.

This is Moore–Hodgson's algorithm. In the timeline's minutes it is exact: no other choice leaves
fewer items late. The kept items, in deadline order, all meet their deadlines. The won't-fit items
are the set `W`.

- **No margin here.** The walk names something "won't fit" only when even exact estimates miss.
  Telling a student to let work go is the claim that most needs to be right (Q4).
- **Slices are not applied.** `start_by` caps each day's contribution to one task at the session
  slice, which models pacing across days. The walk asks a different question: can the work be done
  in the time there is? A 3-hour essay due tonight can be done tonight even with a 2-hour slice.
  Pacing stays where it is: Must do membership and Recommended.

### 2.6 Step 3: overdue work goes first while it costs nothing

Merge the overdue items into the kept deadline order, greedily from the front. At each step,
with `t` the work already placed:
- take the **first overdue item, in pressure order, that can go next** without making any
  remaining kept item late with a 30-minute margin: for every remaining kept item `i`,
  `t + work(overdue) + work(kept up to i) ≤ A(d_i) − 30`;
- if no overdue item can, place the next kept item.

When nothing is tight, every overdue item goes first, in pressure order, exactly as today. When an
overdue item would cost an on-time item, it waits. A shorter overdue item may go ahead of a longer
one that cannot fit yet. The margin applies only here: it guards on-time work against an overdue
item whose estimate runs long (Q4).

### 2.7 Assembling the order

The new Must do order is: the merge of §2.6, then `W` in deadline order. It refills Must do's
positions in the scored list (§2.1), and overrides are inserted as today.

`W` goes last because its deadline is lost either way; doing it first would make others late too.
Each `W` item stays Must do, stays visible and carries its label (§7). It is never hidden or
archived: VISION's "nothing missed" holds.

### 2.8 Worked examples

A Friday whose planning window ends at 18:00, nothing busy after it; estimates exact.

**(a) The 5b case, after O6.** 23:00. A (0.5h) and B (0.25h) are due 23:59; a late lab's window
closes Wednesday. Deadline order A, B, lab; `A(23:59)` = 59 min; A and B fit (45 min); the lab has
days. Order: A, B, lab. This is the ruled evening behaviour, with no hour in it.

**(b) An overdue 3h lab, no window, and a 1h task due 23:59.** The lab may go first while
`3h + 1h ≤ A(23:59) − 30 min`:

| Moment | Time to 23:59 | Lab first? | Order |
|---|---|---|---|
| 15:00 | 8h59 (3h in the window, 5h59 of evening) | yes | lab, task |
| 19:29 | 4h30 | yes, exactly | lab, task |
| 19:30 | 4h29 | no | task, lab |

The crossover is 23:59 minus the two items minus the margin: 7:29 pm for a 3h lab, 9:29 pm for 1h,
9:59 pm for 30 minutes. PR #65 puts the task first at 20:00 in all three cases; the formula
lets a short overdue item go first while tonight's task is still safe.

**(c) A daytime clash, which the 20:00 rule cannot see.** 09:00, the fixture's Friday classes
(12:00–12:50, 13:00–13:45). An 11h essay is due 23:59; a 3h lab has been overdue since the
previous Thursday, so its start-by is further past and, at equal importance, pressure puts it
first. Time to 23:59: 3h (09:00–12:00) + 4h15 (13:45–18:00) + 5h59 of evening = 13h14 (the 10-minute
gap is under the 45-minute block minimum). Lab first needs 14h + 30 min: no. Order: essay, then lab.
In pressure order the essay misses by 46 minutes, with nothing on the page saying so.

**(d) Too much tonight.** 21:00; X (1h), Y (2.5h), Z (1.5h), all due 23:59, in that pressure order;
179 minutes left. X fits (60); X+Y = 210 > 179, so Y, the longest, is removed; X+Z = 150 fits.
Order: X, Z, Y. Y is named "won't fit"; X and Z stay on time. Pressure order would start Y second
and miss two.

## 3. Duration: where it comes from, and when it is missing

The walk's work for an item is `Task::remaining_hours()` (`effort_hours × (1 − progress/100)`),
converted to whole minutes (§5). It is data already in the note; `rank` reads it and calls nothing
(decision 11). The figure has five sources:

| Source | How the note shows it |
|---|---|
| A vendor's own figure (zyBooks, VHL activity counts) | `effort_source: vendor`, refreshed by `coursework` |
| `judge` in the cloud | `effort_source: inferred`, `effort_confidence: low` / `medium` / `high` |
| The student's own edit | the journal shows the user set it; judge never re-sets it |
| Not yet judged | `effort_confidence: unset`, `needs_enrichment: true`, a placeholder `1.0` |
| Absent | no `effort_hours` key; `Task::from_meta` defaults to `1.0` |

`progress` is the student's one control per task, so the walk tracks what is left, not what was
first estimated.

**When the estimate is missing** (the last two rows):
- **Step 1 needs none.** Deadline order is the same whatever the hours.
- **Steps 2 and 3 use the stored figure**, `1.0` included, so the order depends only on the
  `Task`s `rank` already receives. That keeps `rank` pure over the same input it has today, with no
  new note field and no new `Task` field.
- **The wording changes, not the order.** The read model knows each note's frontmatter (`Loaded`
  holds it). A won't-fit item whose own estimate is missing reads "may not fit" with the reason
  ("no time estimate yet"), never a flat "won't fit" (§7). When any item in the overrun has a missing
  estimate, the meter's sentence says how many (Q5).
- **Low confidence is still an estimate.** It is used as is. The 30-minute margin of §2.6 is the
  only allowance for error, and it protects on-time work.
- Better estimates come from the learning loop (`docs/surface/anatomy.md`'s `took:` prompt), not
  from this spec.

## 4. How it meets what exists

- **Pressure** is unchanged and still computed for every task. It orders Recommended and everything
  outside Must do, breaks deadline ties (§5), and orders the overdue items among themselves in §2.6.
- **Must do membership** is unchanged: slack ≤ 0, decided by `start_by` as today. The formula
  changes order only. Won't-fit items stay Must do.
- **`rank_override`** keeps winning: overrides are taken out before the formula and inserted after,
  as both `rank` and PR #65 do. **The walk does not count them** in v1: a pinned item's hours are
  not charged against anyone's deadline, so a pin that makes tonight's item late is not flagged
  (Q8). The student who pinned it chose that.
- **The designation** (`designate_today_explained`) reads the same Must do set and the same
  Recommended order. Must do reserves time front to back, so its order can move a `round2` by a
  cent in a rare case; the plan's test compares takes with the order changed, as PR #65's R10 does,
  and if they ever differ, the designation keeps the pressure-ordered Must do list.
- **The console's Must do block.** Today it groups by label in first-appearance order and re-sorts
  each group by start-by. Under the formula both would hide the order: in §2.6's merge an overdue
  item can sit between two items due today, and first-appearance grouping would draw the second
  one above it. **Proposed (Q6):** when the formula runs, a group is a run of consecutive rows
  with the same label, and rows keep rank order (`sort_rows` is not applied to Must do). A label
  repeats only when the order really interleaves. PR #65's open Q-1 ("does `Due today` sort by due
  time after the hour?") is answered by this: rows follow deadline order.
- **The meter.** `fit_blocks`, `spill_blocks`, `capped` and `spill_hours` keep their formulas over
  the whole day's capacity, so "how much doesn't fit" does not change. `spill_items` ("what") lists
  `W` first, in deadline order, then the existing first-fit spill of the rest in the new order.
  Each `SpillItem` gains an optional deadline for the hover's wording, omitted from the JSON when
  absent. The meter and the walk measure different things (today's capacity against all Must do
  work, and the time to each deadline); the hover says which is which (§7).
- **`state/today.md`.** Production `rank` has the vault's clock as its moment, so the slot's
  `today.md` lists Must do in the new order. **No new line in v1** (Q7): the won't-fit naming lives
  on the page. `render.rs` is not touched.
- **Later consumers,** not built here: `W` is exactly VISION's "a deadline at real risk at the
  current pace", so nudges and lock-in can read it when they are specified.

## 5. Determinism and tie-breaks

- **Integer minutes everywhere in the walk.** Work is `round_ties_even(remaining_hours × 60)`;
  the timeline is summed in whole minutes from `DateTime` differences. Every comparison in §2.5 and
  §2.6 is between integers, so no float noise decides a "won't fit" (`2.79 + 0.21` against `3.0`).
- **Deadline ties:** scored-list position (pressure, its preserved quirk, then input order, which
  is `load_task_notes`'s sorted paths).
- **Which item the walk removes:** most work; then the later deadline; then the later scored
  position (lower pressure). Q9 asks whether importance should come before hours.
- **The merge:** overdue items are tried in scored order, the first that fits is placed; with none,
  the next kept item. One pass, no search.
- **`W`'s order:** deadline, then scored position.
- **Same notes, day, moment and calendar give the same order** (VISION commitment 1). The moment is
  an input like the day, as PR #65 made it. No model, no network, no write: `rank` and `surface`
  stay pure and `surface` still never writes.

## 6. The 20:00 step: deleted

**Proposed (Q2): delete `EVENING_HOUR`, `closes_tonight` and the evening branch of `rank_at` in the
same change that adds the formula.** Keep `rank_at` itself, its moment, `rank --now` and
`surface::load_at`: the formula needs exactly that plumbing, and PR #65 built it.

- **Why not keep both:** two rules over the same rows would disagree in example (b) between 20:00
  and the crossover (the hour says the task first; the formula says the lab still fits). A
  student would see the order flip at 20:00 for no reason the page can give.
- **What it costs:** PR #65's tests that assert the hour (R14 says "changing it needs a new
  ruling"; R3 asserts the task leads at 20:00, and under the formula, with the fixture's 18:00
  window, the 2h overdue task still fits in front) are retired in that change, with this spec's
  ruling named in the commit message. The tests that state invariants the formula keeps (no moment
  equals `rank`, a moment on another day changes nothing, overrides win, Must do and the takes
  unchanged, determinism, undated tasks never move) are kept, re-pointed at the formula where their
  setup relied on the hour. The plan's first task runs each one against the formula and records
  which is which.
- **The ruled evening behaviour survives:** at 11 pm tonight's items lead (example (a)); a late
  item with a later window ranks below them (deadline order); a no-window late item yields once it
  would cost tonight's work (§2.6). Ruling (b) holds (only a window's close counts), ruling (c) holds
  (no evening rewrite of `today.md`), ruling (d) holds (tonight ends at midnight, §2.3).
- **The alternative,** keeping 20:00 as a floor under the formula, is safer against estimates that
  run short, and it is what the margin is for. Recommend against.

## 7. What the student sees

Plain, specific, never an instruction to give up. Knowlu names the item; the student decides.

- **On a won't-fit row** (a short line under the title, as the conflict and amend lines are):
  - "Won't fit before 11:59 pm" (today), "Won't fit before Wed 11:59 pm" (another day);
  - with a known late window (after O6): "Won't fit before 11:59 pm · late until Fri, −20%";
  - with no estimate of its own: "May not fit before 11:59 pm · no time estimate yet".
- **Its explanation** (shown on demand beside the start-by `why`, which is unchanged): "3.0h left
  and 1.5h of your time before it's due. Doing the others first keeps 2 on time." With the counts
  and hours from the walk.
- **On the meter,** when `W` is not empty, a line under the existing sentence: "If something has to
  give: Lab 3 (3.0h). The rest fits by its deadlines." With more than one: "If something has to
  give: Lab 3 and Essay draft (5.5h)." When an overrun includes items with no estimate: "Tonight
  may be tight: 2 items have no time estimate yet."
- **The spill hover** names a `W` item as "won't fit before Wed 11:59 pm" and any other spill item as
  today ("doesn't fit in today's hours").
- **Never** "skip", "drop" or "give up", and no "let go" in the page text: the item is still the
  student's to do. Q7 asks Quinn to rule the words.

The row line is a new optional read-model field, omitted from the JSON when absent. The console
renders it (`app/static`). Wording lives in the engine, as every other row line does.

## 8. Tests and oracles

**Frozen references (CLAUDE.md rule 2): none is regenerated, and two guards hold each one.**
- `golden-today-s1.md`, `golden-today-full.md`: `oracle.rs` runs `rank --today 2026-08-28` with
  no `--now`, so there is no moment and the formula does not run. Second guard: in all three
  fixture vaults Must do is exactly two past-due tasks with no window (`cs-100-hw-01`,
  `task-erste-reflexion`) and nothing dated, so even with a moment the merge places both first in
  pressure order, the order the goldens print. A test drives `rank --today 2026-08-28 --now <T>` at
  09:00, 19:30, 21:00 and 23:58 against both goldens (PR #65's E1, extended).
- `surface-today-{s1,s1-migrated,full}.json`: **these do have a moment** (`--now
  2026-08-28T09:00`), so the formula runs on them. They stay byte-identical because of the second
  guard above (same order, the same two-row `Overdue` group, rank order equal to `sort_rows`'s
  order there), because all three `spill_items` are empty, and because every new field (the row
  line, the spill deadline) is omitted when absent. A test proves it on the parsed blocks at 09:00
  and 21:00 before any code changes (PR #65's E2, extended). No §4.6 regeneration.
- `oracle.rs` and `surface_oracle.rs` run unedited and green at every commit.

**New behaviour tests** (names are the plan's; behaviour is not). In memory on the vault-s1 week
template where they can be, on temp-dir copies of a fixture vault where they need notes. **No
fixture is edited or added.**
- D1, no moment equals `rank`; a moment on another day equals `rank`.
- D2, deadline order among dated Must do items, ties in pressure order.
- D3, the timeline: window blocks clipped at the moment; the evening before a deadline (after the
  window, and before it opens the next morning) counted; busy time in it subtracted; whole minutes.
- D4, the walk: example (d) names the longest; a tie names the later deadline, then the lower
  pressure; nothing is named when everything fits.
- D5, the merge: example (b) at 19:29 and 19:30; example (c); a shorter overdue item goes ahead of a
  longer one that cannot fit; with nothing tight, overdue items lead in pressure order.
- D6, assembly: non-Must-do tasks keep their exact indexes; Must do membership and the takes are
  unchanged; overrides still win and are not charged in the walk.
- D7, a placeholder estimate (`effort_confidence: unset`) orders like its stored figure; named in
  `W`, its row reads "may not fit".
- D8, determinism: two runs equal; reversed input of distinct keys gives the same order.
- D9, the read model: consecutive-run groups with rows in rank order; the meter's `spill_items`
  list `W` first; totals unchanged; the row line present only on `W` rows.
- D10, `today.md` at a moment lists Must do in the new order and has no new line.

**Tests that change:** PR #65's hour tests are retired in the replacing commit (§6). Every other
existing test passes unedited; the plan's first task lists engine and app tests that build a page
with Must do work at a clock time (`app` tests resolve a pinned date to 12:00) and confirms each.

## 9. VISION and the two rules

- **The test for every feature.** "What's next?" gets more honest: the order says what can still be
  saved, and the page says what cannot, before the evening it is lost.
- **Commitment 1.** Pure arithmetic over the notes, the calendar and the moment; integer minutes;
  total tie-breaks. **`rank` never calls a model**; estimates are data a judge wrote earlier.
- **Rule 1, no single-user assumptions.** No constant names a student: the timeline is each vault's
  own window, classes, commitments and calendar; the 30-minute margin is one value for everyone; the
  20:00 constant, a single-student guess the evening spec flagged for checking, goes away.
- **Rule 2.** §8: nothing regenerated, both guards proved by tests first.
- **Invariants.** No note field, no journal record, no run-record count, nothing new that syncs.
  Must do and the 15-a-day approvals budget are untouched.

## 10. Shape of the plan (for the planner who writes it)

| Task | What | Agent | Why |
|---|---|---|---|
| T0 | Branch after PR #65 merges; re-confirm symbols; run PR #65's tests against the formula's expectations and list retire/keep | main session | git and judgment |
| T1 | Red: `ranking.rs` D1-D8 | `test-writer` | the failing tests, specified |
| T2 | `ranking.rs`: deadlines, timeline, walk, merge, assembly; the 20:00 step removed | `implementer` (high) | reorders every page; off the contract list |
| T3 | Red: `surface.rs` D9 and the extended E1/E2 | `test-writer`; `contract-reviewer` reads the reference tests | they assert against frozen and read-model references |
| T4 | `surface.rs`: groups, meter, the row line, "may not fit" | `implementer` (high) | the read model; references must not move |
| T5 | The row line and the meter line in the console | `console-ui` | `app/static` |
| T6 | Docs: `engine-commands.md`'s `rank`, `anatomy.md` §3.4 and the Must do block, HANDOFF | `docs-keeper` | references |
| T7 | Review; scratch demo of examples (b) and (c) | `reviewer`; main session | pre-push; scratch vault only |

T2 and T4 touch disjoint files (`ranking.rs`, `surface.rs`); T5 touches only `app/static`. Nothing
on the contract list changes. If the plan finds the timeline needs a `weekcal.rs` change, that file
joins T2.

## 11. Questions for Quinn

Asked one at a time, in this order; each has a recommendation. Q1-Q3 decide the formula; Q4-Q9
tune it and can follow.

- **Q1. Overdue work with no known late window: always first, or first while it costs nothing?**
  The proposal kept it always first; that reverses the evening spec's ruled §3 after the hour.
  **Recommend: first while it costs nothing** (§2.6): it leads when every on-time item still makes
  its deadline with a 30-minute margin, and waits when it would cost one. It reproduces today's
  order whenever nothing is tight, and the ruled evening order when something is.
- **Q2. Delete the 20:00 step when the formula ships?** **Recommend: yes** (§6). Keep PR #65's
  moment plumbing; retire its hour tests with this ruling named. Keeping both makes the order flip
  at 20:00 for no reason the page can give.
- **Q3. Which hours count as the student's time?** **Recommend: the planning window plus the
  evening before a deadline** (§2.3), not the window alone (it says everything due tonight won't
  fit after the window closes) and not the wall clock (it plans as if no one sleeps).
- **Q4. The safety margin.** **Recommend: 30 minutes, applied only when deciding whether overdue
  work may go first; none on a "won't fit" verdict.** Be careful in both directions: don't risk
  on-time work for late work, and don't tell a student something won't fit unless it truly doesn't.
  One value for everyone; a per-vault setting only if the pilot asks for one.
- **Q5. Items with no time estimate yet.** **Recommend: order by the stored figure (`1.0`), never
  call them "won't fit", only "may not fit · no time estimate yet",** and count them in the meter
  line. The alternative, leaving them out of the walk, hides real work from the fit check.
- **Q6. The console's Must do groups.** **Recommend: a group is a run of consecutive rows with the
  same label, and rows keep rank order** (§4). The alternative, one ungrouped list with the label on
  each row, matches VISION's "one ordered list" better but changes the read-model shape and the
  three surface references; it belongs with the main-page redesign (5b Departures). This also
  answers PR #65's open Q-1.
- **Q7. The words.** **Recommend §7's set:** "Won't fit before 11:59 pm" on the row; "If something
  has to give: Lab 3 (3.0h). The rest fits by its deadlines." on the meter; never "skip", "drop",
  "give up" or "let go" in the page text. And **no new line in `today.md` in v1**: the page carries it.
- **Q8. Pinned items (`rank_override`) in the walk.** **Recommend: not charged in v1** (§4). A pin is
  the student's explicit choice; charging it means guessing when they will do the pinned item.
- **Q9. Which item the walk names.** Moore–Hodgson names the longest, which leaves the fewest items
  late. A student may rather lose a 1-point quiz than a 20-point lab. **Recommend: longest first,
  ties by lower importance, for v1;** weighting by grade weight (an exact, still-deterministic
  method exists, Lawler and Moore's) once confirmed syllabus weights are on course notes, as its own
  spec.

## 12. Out of scope

- O6's late window itself (the deadline function has one place for its close, §2.2), and penalty
  steps (evening ruling b).
- The meter's capacity being the whole day rather than what is left from now: an existing gap,
  shared with PR #65; noted, not changed.
- Nudges and lock-in reading `W` (VISION's "a deadline at real risk"); each needs its own spec.
- Weighting by grade weight (Q9), a per-vault margin (Q4), learning from actual durations.
- Any change to pressure, `start_by`, slack, Must do membership, Recommended or the designation.
- `CLAUDE.md`: "same input, same order" already holds with the moment as an input.
