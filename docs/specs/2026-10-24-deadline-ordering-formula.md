# Deadline ordering: one formula for Must do (ruled 2026-10-24)

**Status: RULED by Quinn, 2026-10-24.** Quinn ruled all ten §11 questions as recommended, signed
directly on 2026-10-24. Where the body says "proposed" or "Q*n* asks", read "ruled as recommended".
Review r1 failed (4 important, 9 minor), review r2 failed narrowly (2 important, 5 minor), review
r3 passed (one nit, folded into §10); every finding is mapped in §13. Its plan is
`docs/plans/2026-10-24-deadline-ordering-plan.md`.

It answers Quinn's question on the evening rule ("Why 8pm? Is there a general formula we can come
up with to decide what should come first based on how long it will take and when it is due?").
Quinn chose to ship the 20:00 rule now and to spec the general formula to replace it later. The
20:00 rule (PR #65) is merged: it is on `main` as `7b3e10c`. This is the spec that replaces it.

**Builds on:** `docs/specs/2026-10-23-evening-ranking-rule-draft.md` (ruled 2026-10-24: the hour,
penalty steps out, no evening rewrite of `today.md`, "tonight" ends at local midnight) and its plan
`docs/plans/2026-10-24-evening-ranking-plan.md`; the onboarding redesign's §5.2 (O6, the late
window, ruled Q6 2026-10-23); `docs/surface/anatomy.md` §3.4 (the meter). Ranking authority remains
`2026-08-11-personal-ops-system-design.md`; the cloud design wins where they disagree, and it says
nothing about order beyond decision 11 (`rank` never calls a model).

**Code read for this draft:** the 20:00 rule's branch as it stood before merging (the merged code is
on `main` at `7b3e10c`, which the plan reads): `engine/src/ranking.rs`
(`pressure`, `rank`, `rank_at`, `EVENING_HOUR`, `closes_tonight`, `partition_must_do`,
`designate_today_explained`), `engine/src/surface.rs` (`load_at`, `build_state_with`,
`build_state_preview`, `with_window`, `meter`, `must_do`, `group_label`, `sort_rows`, `row`),
`engine/src/weekcal.rs` (`with_day_window`, `window`, `blocks_from_busy`, `free_blocks`),
`engine/src/scheduling.rs`, `engine/src/models.rs`, `engine/src/render.rs::capacity_breakdown`, and
the three `surface-today-*.json` references. The planner had no shell; the plan's first task
re-confirms every symbol against `main`, which it branches from.

"The 20:00 rule" below means the code merged as `7b3e10c`, which is what `main` ranks by today.

---

## 0. The answer, short

**Keep today's order while it is safe. When it would make something late, order Must do by when
it is really due, and use the hours to decide what fits and how long overdue work may go first.**

0. **Is today's order safe?** Walk today's Must do order, adding remaining hours against the
   student's time from now. ("Today's order" in this spec always means pressure order, `rank` with
   no moment: the order before the 20:00 step, which this spec removes.) If every item with a
   deadline still finishes at least 30 minutes before it, nothing changes (ruled Q2: no daytime
   reorder when nothing is at risk).
1. **Deadline order.** Otherwise, Must do work with a deadline ahead is ordered earliest deadline
   first. A deadline is the due time, or, for a past-due item with a known late window (O6), the
   window's close. Earliest-deadline-first (Jackson's rule) minimises the worst lateness on one
   worker whatever the durations, so this order needs no duration.
2. **The fit walk.** Walk that order adding remaining hours. When the running total passes an
   item's deadline, the longest item so far is moved out as **won't fit** (Moore–Hodgson: the
   fewest items late). Won't-fit items go to the end of Must do and are named on the page.
3. **Overdue work goes first while it costs nothing.** A past-due item with no known window has no
   deadline left to meet. It goes as early as it can without making any on-time item late, with
   the same 30-minute margin.

It runs only when `rank` has a moment (the live page, the `--window` preview, timed runs). With no
moment (`rank --today`, the golden harness) the order is today's, byte for byte.

**Why 8 pm, then?** There is no fixed hour. For an overdue 3-hour lab and a 1-hour task due at
11:59 pm, the lab leads until 7:29 pm and yields after; for a 1-hour lab the change comes at
9:29 pm (§2.9). The hour is when the overdue work stops fitting in front of tonight's.

**A correction to the proposal as it reached the planner.** The proposal said "an overdue item with
no late window keeps leading". That would undo the ruled evening spec's §3, where a no-window late
item ranks **below** tonight's items after the hour ("an on-time submission tonight is a certain
grade; an unknown late rule is not"). Step 3 keeps both: it leads when it fits and yields when it
would cost tonight's work. Quinn ruled this in Q1.

## 1. What exists today

- `ranking::pressure` is driven by slack in whole days (`scheduling::slack_days`, from `start_by`'s
  backward walk over daily capacity, capped at the session slice, minus a one-day buffer). Overdue
  starts score `100 + |slack| × 10`, the rest `50 / (1 + slack)`, each times `0.6 + 0.1 ×
  importance`. `rank` sorts by pressure (the preserved Python quirk on ties) and inserts
  `rank_override`s by position. So Must do today is in **slack order**: a big essay due tonight
  whose start-by is days past outranks a small item due yesterday.
- `partition_must_do`: slack ≤ 0 is Must do. Duration decides membership only through `start_by`.
- On `main` (`7b3e10c`), `rank_at(tasks, today, cal, moment)`: from `EVENING_HOUR` (20:00) on,
  tasks whose due falls on the moment's date and after it lead, by due time; everything else keeps
  pressure order; overrides still win. `rank` is `rank_at(.., None)`.
- `surface::meter` fills the **whole day's** capacity (`capacity_breakdown`, not what is left from
  now) with Must do work in rank order, skipping what does not fit; the skipped tasks are
  `spill_items`. `spill_hours` and the block counts do not depend on order.
- `surface::must_do` groups Must do rows by `group_label` (`Overdue`, `Due today`, `Due <day>`,
  `Later`, from the due **date** against today) in the order each label first appears, then
  re-sorts each group by start-by, then due (`sort_rows`).
- `WeekCalendar` keys its planning window by **weekday** (`window: [Option<(Time, Time)>; 7]`);
  `with_day_window(day, ..)` replaces the window for every date sharing `day`'s weekday, and ignores
  `start >= end`, so a window ending at midnight (`00:00`) cannot be expressed.

None of this uses the time of day except the 20:00 step, and none of it asks whether the Must do
work can all be finished before its deadlines.

## 2. The formula

### 2.1 When it runs and on what

- **Only with a moment `m`** whose date is the day being ranked (the guard the 20:00 rule uses). No moment,
  or a moment on another day: the order is exactly `rank`'s.
- **`m` is truncated to the minute** (seconds and below dropped) before anything else. Due times
  are minute-precise already (`coerce_datetime`), so every comparison below is on whole minutes.
- **Only on Must do** (slack ≤ 0, unchanged), and only on items without a `rank_override`.
- **It permutes Must do in place.** The scored list's Must do items are reordered; the positions
  they occupied are refilled in the new order; every other task keeps its exact index. Overrides are
  then inserted as today. Recommended and every non-Must-do position are untouched by construction.
- **The `--window` preview** (`surface::build_state_preview`) ranks at the same moment as the live
  page, on the previewed calendar, so the preview shows the order the proposed window would give.
  The previewed window is weekday-keyed, as `with_window` already makes it, so in a timeline longer
  than a week it applies to every matching weekday. The `moved` diff is unaffected (§4,
  designation).

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

### 2.3 Available time: the timeline

The walk needs "how much of the student's time lies between now and this deadline". **One timeline
serves every item and depends on no task**, so that the scheduling theory behind steps 1 and 2
holds exactly, a later deadline never has less time than an earlier one, and adding or finishing
an unrelated task never changes the time available to another item. (It can still change that
item's verdict the honest way, by adding to or taking from the work ahead of it.)

**The timeline** runs from `m` forward, date by date, to the last deadline (at most 365 days,
`start_by`'s bound). Each date contributes its free blocks **from its planning window's start to
local midnight**:
- the busy time is what `free_blocks` already subtracts: template classes, confirmed commitments
  (instances where a series has them) and calendar events;
- the end bound is the next date's `00:00` as a `DateTime`, so midnight is representable;
- the window's start, the classes and the commitments are the vault's own; only the evening bound
  is fixed, and it is evening ruling (d)'s "tonight ends at local midnight";
- **order of operations:** each date's blocks are computed whole, then the minimum-block filter is
  applied (as `free_blocks` does, after subtraction), then the date of `m` is cut to start at `m`. A
  piece left shorter than the minimum by the cut still counts: it is the rest of a real block;
- **counting:** civil minutes, as every capacity figure in the engine is (`hours_between` on civil
  `DateTime`s). On the autumn fall-back day the repeated hour is counted once; on the spring
  day the skipped hour counts as if it existed. Both fall between 01:00 and 03:00, outside any
  window that starts in the morning, so in practice neither is ever counted;
- **busy time at and across midnight** (review r2, M2):
  - a calendar event is subtracted as a `DateTime` span (`subtract`), so one ending at `00:00` takes
    the evening to midnight, and one from 23:00 to 01:00 takes 23:00–24:00 from its start date; its
    after-midnight part falls before the next date's window opens and takes nothing;
  - a commitment is a start and end time on a weekday, so it cannot cross midnight: one ending at
    23:59 takes the evening to 23:59. One written across midnight (a 22:00–02:00 shift) has its end
    before its start, and `weekly_span_block` and `active_spans` drop it, so that evening reads as
    free. That is today's behaviour everywhere, not this spec's; §12 lists it;
  - **can a span reach `blocks_from_busy` with its end before its start?** A commitment cannot (both
    readers above drop it). A template class can: `class_busy` does not check, and the cursor walk
    (`weekcal.rs`, `blocks_from_busy`) then cuts the free block at the class's start without moving
    the cursor past it, so the time before it is counted twice. **The new method drops any busy span
    whose end is not after its start before the walk**; existing callers keep today's behaviour (a
    separate fix, §12).

`A(d)` is the whole minutes of the timeline before `d`. It never grows as `m` advances and never
shrinks as `d` moves later. So with no progress and no new estimate, a "fits" can turn into "won't
fit" as time passes, never the reverse, and a later deadline never reads "won't fit" where an
earlier one with the same work fits.

**The mechanism is a new per-date method, settled here** (review r1, I1): `WeekCalendar` gains one
pure method returning a date's free blocks with the end bound replaced by that date's following
midnight, keyed by the **date**, mutating nothing. `with_day_window`, `window` and every existing
caller are untouched; `blocks_from_busy` may take its end as a `DateTime` internally so both share
one cursor walk. `weekcal.rs` is not on the contract list.

**Why every evening to midnight, and not the alternatives** (Q3):
- **The planning window alone** ends before most homework deadlines. The fixture template's day
  ends at 18:00; at 20:00 it leaves no time before 23:59, so everything due tonight would read
  "won't fit" and step 3 would put overdue work first. That is false, and it is the evening Quinn
  asked about. Wording it as "won't fit in your planned hours" does not save it: the order is still
  wrong.
- **The window plus only the evening before a deadline** (r1's choice) makes verdicts move the wrong
  way. At Thursday 19:00 with an 08:00–18:00 window, a 5h item due Friday 07:59 fits (Thursday
  evening counted) while the same item due Friday 12:00 does not (no evening counted), and an
  unrelated small task due tonight could flip it.
- **The wall clock** (every minute, minus busy time) plans as if the student never sleeps, and lets
  overdue work go first on night hours that are not there.
- **Every evening to midnight** reads the student's evenings as available for deadlines, which is
  what students do with them, and stops at midnight, which is ruled. The cost: a student who never
  works evenings sees "won't fit" later than they should. The planning window still governs
  Recommended, the designation and the meter.

### 2.4 Step 0: is today's order safe?

Take the non-override Must do items in pressure order. Walk them keeping a running total `t` of
work minutes, overdue items included. Today's order is **safe** when every dated item finishes with
the margin: `t (up to and including it) ≤ A(d) − 30`.

- **Safe:** the order is pressure order, exactly as today; nothing is named.
- **Not safe:** steps 1 to 3 produce the order.

So the order changes only when today's order would make something late or leave it under 30
minutes to spare. Quinn ruled (Q2) against reordering by deadline always (example (e) shows the
difference).

### 2.5 Step 1: deadline order

Sort the dated items by deadline, earliest first. Ties keep their order in the scored list.

Jackson's rule: on one worker, earliest deadline first minimises the maximum lateness, and if any
order meets every deadline, this one does. Durations do not change it. That is the direct answer to
Quinn's question: **for the order alone, only the deadline matters; duration matters for whether it
all fits, for what gives if it does not, and for how long overdue work may go first.**

### 2.6 Step 2: the fit walk

Walk the deadline order keeping a running total `t` of work minutes. Add each item's work. If
`t > A(d)` for the item just added, remove the item with the most work among those kept so far
(ties in §5), subtract its work from `t`, and mark it **won't fit**. Continue to the end.

This is Moore–Hodgson's algorithm. On the timeline's minutes it is exact: no other choice leaves
fewer items late. The kept items, in deadline order, all meet their deadlines. The won't-fit items
are the set `W`. For each, the walk records **its prefix at the moment of removal**: every other
item kept at that point, up to and including the item whose addition overflowed (which may be the
removed item itself, or an item after it). The wording needs it (§3): any of those items, not only
the ones ahead of it, could have made the difference.

- **No margin here.** The walk names something "won't fit" only when even exact estimates miss.
- **Slices are not applied.** `start_by` caps each day's work on one task at the session slice,
  which models pacing across days. The walk asks whether the work can be done in the time there is;
  a 3-hour essay due tonight can be done tonight even with a 2-hour slice.

### 2.7 Step 3: overdue work goes first while it costs nothing

Merge the overdue items into the kept deadline order, greedily from the front. At each step, with
`t` the work already placed:
- take the **first overdue item, in pressure order, that can go next** without making any remaining
  kept item late with the margin: for every remaining kept item `i`,
  `t + work(overdue) + work(kept up to i) ≤ A(d_i) − 30`;
- if no overdue item can, place the next kept item.

A shorter overdue item may go ahead of a longer one that cannot fit yet. The margin guards on-time
work against an overdue item whose estimate runs long (Q4).

### 2.8 Assembling the order

When step 0 finds today's order unsafe, the new Must do order is the merge of §2.7, then `W` in
deadline order. It refills Must do's positions in the scored list (§2.1), and overrides are
inserted as today.

`W` goes last because its deadline is lost either way; doing it first would make others late too.
Each `W` item stays Must do, stays visible and carries its line (§7). It is never hidden or
archived: VISION's "nothing missed" holds.

### 2.9 Worked examples

The fixture's week template (window 08:00–18:00; Friday classes 12:00–12:50 and 13:00–13:45; a
45-minute minimum block), nothing busy in the evenings, estimates exact. In each, pressure order is
the order `rank` gives with no moment; the plan's tests assert it first, as the 20:00 rule's do.

**(a) The 5b case, after O6.** Friday 23:00. A (0.5h) and B (0.25h) are due 23:59; a 2h late lab's
window closes Wednesday, and pressure puts the lab first. Step 0: 120 minutes of lab before A,
with 59 minutes to 23:59: unsafe. Deadline order A, B, lab; A and B fit (45 of 59 minutes); the lab
has days. Order: A, B, lab. This is the ruled evening behaviour, with no hour in it.

**(b) An overdue 3h lab, no window, and a 1h task due 23:59.** Pressure puts the lab first. Step 0
holds while `3h + 1h ≤ A(23:59) − 30 min`:

| Moment (Friday) | Time to 23:59 | Today's order safe? | Order |
|---|---|---|---|
| 15:00 | 8h59 (3h in the window, 5h59 of evening) | yes | lab, task (as today) |
| 19:29 | 4h30 | yes, exactly | lab, task (as today) |
| 19:30 | 4h29 | no; the lab cannot go first either | task, lab |

The crossover is 23:59 minus the two items minus the margin: 7:29 pm for a 3h lab, 9:29 pm for 1h,
9:59 pm for 30 minutes. The 20:00 rule puts the task first at 20:00 in all three cases; the formula lets a
short overdue item go first while tonight's task is still safe.

**(c) A daytime clash, which the 20:00 rule cannot see.** Friday 09:00. An 11h essay is due 23:59; a
3h lab has been overdue since the previous Thursday, and pressure puts the lab first. Time to 23:59:
3h (09:00–12:00) + 10h14 (13:45–23:59) = 13h14 (the 10-minute gap between classes is under the
minimum). Lab then essay needs 14h: unsafe. The essay fits alone (11h); the lab first would need
14h30 with the margin. Order: essay, lab. In today's order the essay misses by 46 minutes, with
nothing on the page saying so.

**(d) Too much tonight.** Friday 21:00; X (1h), Y (2.5h), Z (1.5h), all due 23:59, in that pressure
order; 179 minutes left. Unsafe at Y. X fits (60); X+Y = 210 > 179, so Y, the longest, is removed;
X+Z = 150 fits. Order: X, Z, Y. Y is named; X and Z stay on time. Today's order would start Y second
and miss two.

**(e) Daytime, nothing tight (review r1, I3).** Friday 09:00. An 11h essay is due 23:59 (start-by
days past; review r1 puts its pressure near 153) and a 0.5h reflection was due yesterday (near 108),
so today's order is essay, reflection. Step 0: the essay finishes after 11h of 13h14, more than 30
minutes spare: safe, and the order stays essay, reflection. "Always reorder" (declined, Q2) would
have put the reflection first (0.5h + 11h still leaves 1h44), changing the daytime order and the
12:00 and 18:00 `today.md` even though nothing was at risk.

**(f) Verdicts move one way (review r1, I2).** Thursday 19:00, a 5h item P. Due Friday 07:59, the
time before it is Thursday 19:00–24:00, exactly 5h: it fits (with no margin to spare, step 0 calls
the order unsafe, so the formula orders it, but nothing is named). Due Friday 12:00 instead, the
time is 5h + 4h of Friday morning: it fits more easily, never less. A small task due tonight does
not change `A` for P at all; it can only push P's running total.

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
first estimated. The last two rows are **placeholders**.

**When an estimate is a placeholder:**
- **Step 1 needs none.** Deadline order is the same whatever the hours.
- **Steps 0, 2 and 3 use the stored figure**, `1.0` included, so the order depends only on the
  `Task`s `rank` already receives: no new note field, no new `Task` field, `rank` pure over the
  same input.
- **The wording changes, not the order.** A `W` item reads **"may not fit"**, never a flat "won't
  fit", when its own estimate is a placeholder **or any other item in its prefix at removal has
  one** (§2.6: every item kept at that point, up to and including the one that overflowed; reviews
  r1 I4 and r2 I1). Two cases, both Friday 21:00 with 179 minutes left, Y a real 2.5h and X
  unestimated (1.0h stored), both due 23:59:
  - **X first** in pressure order: X fits (60), X+Y = 210 > 179, Y (the longest) is removed. If X
    really takes 15 minutes, Y fits.
  - **Y first** in pressure order: Y fits (150), Y+X = 210 > 179 when X is added, and Y is still
    the longest, so Y is removed although X comes after it. If X really takes 15 minutes, Y fits.
  In both, Y reads "may not fit · depends on 1 item with no time estimate yet". The read model
  decides this from the walk's record and the notes' frontmatter, which `Loaded` holds.
- **Low confidence is still an estimate** and is used as is. The 30-minute margin is the only
  allowance for error, and it protects on-time work.
- Better estimates come from the learning loop (`docs/surface/anatomy.md`'s `took:` prompt), not
  from this spec.

## 4. How it meets what exists

- **Pressure** is unchanged and still computed for every task. It is the order step 0 checks, orders
  Recommended and everything outside Must do, breaks deadline ties (§5) and orders the overdue items
  among themselves in step 3.
- **Must do membership** is unchanged: slack ≤ 0, decided by `start_by` as today. The formula
  changes order only. `W` items stay Must do.
- **`rank_override`** keeps winning: overrides are taken out before the formula and inserted after,
  as `rank` and the 20:00 rule both do. **The walk does not charge them** in v1: a pinned item's hours are
  not counted against anyone's deadline (Q8). Where a pinned Must do item exists, every
  "fits" sentence says so (§7).
- **The designation reads pressure order, decided** (review r1, M5). `designate_today_explained`,
  and `commitments::moved` through it, are given `rank`'s list (no moment), not the formula's.
  Because the formula permutes Must do in place, the two lists differ only in Must do's order, so
  Recommended's takes, block indexes and caps are today's exactly, and `moved` diffs the same
  designations it does today. `cli.rs` and `surface.rs` each keep both lists.
- **The console's Must do block.** First-appearance grouping and the per-group `sort_rows` would
  hide the order: in step 3's merge an overdue item can sit between two items due today, and
  first-appearance grouping would draw the second one above it. **Ruled (Q6):** when the
  formula runs, a group is a run of consecutive rows with the same label, and rows keep rank order
  (`sort_rows` is not applied to Must do). A label repeats only when the order really interleaves.
  **Labels read from the moment:** a task due today at 10:00, seen at 15:00, is past due and reads
  `Overdue`, not `Due today`; `group_label` keeps its date rule for everything not yet due. The
  20:00 rule's plan left its Q-1 open ("does `Due today` sort by due time after the hour?"); this
  answers it: rows follow the order.
- **The meter.** `fit_blocks`, `spill_blocks`, `capped` and `spill_hours` keep their formulas over
  the whole day's capacity, so "how much doesn't fit today" does not change. `spill_items` lists `W`
  first, in deadline order, then the existing first-fit spill of the rest in the new order. Each
  `SpillItem` gains an optional deadline for the hover's wording, omitted from the JSON when absent.
  The meter (today's window against all Must do work) and the walk (time to each deadline, evenings
  included) measure different things; the hover says which is which (§7).
- **`state/today.md`.** Production `rank` has the vault's clock as its moment, so a slot whose
  Must do is unsafe writes the new order. **No new line in v1** (Q7): naming lives on the page.
  `render.rs` is not touched.
- **Later consumers,** not built here: `W` is VISION's "a deadline at real risk at the current pace",
  so nudges and lock-in can read it when they are specified.

## 5. Determinism and tie-breaks

- **Integer minutes everywhere.** `m` truncated to the minute (§2.1); work is
  `round_ties_even(remaining_hours × 60)`; the timeline is whole minutes of civil `DateTime`
  differences (§2.3). Every comparison in steps 0 to 3 is between integers, so no float noise
  decides a verdict (`2.79 + 0.21` against `3.0`).
- **Deadline ties:** scored-list position (pressure, its preserved quirk, then input order, which
  is `load_task_notes`'s sorted paths).
- **Which item the walk removes:** the most work; then the later deadline; then the later scored
  position (lower pressure, which already weighs importance). Q9 uses exactly this rule.
- **The merge:** overdue items are tried in scored order, the first that fits is placed; with none,
  the next kept item. One pass, no search.
- **`W`'s order:** deadline, then scored position.
- **Same notes, day, moment and calendar give the same order** (VISION commitment 1). The moment is
  an input like the day, as the 20:00 rule made it. No model, no network, no write: `rank` and `surface` stay
  pure and `surface` still never writes.

## 6. The 20:00 step: deleted

**Ruled (Q5): delete `EVENING_HOUR`, `closes_tonight` and the evening branch of `rank_at` in the
same change that adds the formula.** Keep `rank_at` itself, its moment, `rank --now` and
`surface::load_at`: the formula needs exactly that plumbing, and the 20:00 rule (`7b3e10c`) built it.

- **Why not keep both:** two rules over the same rows would disagree in example (b) between 20:00
  and the crossover (the hour says the task first; the formula says the lab still fits). The order
  would flip at 20:00 for no reason the page can give.
- **What it costs** (review r2, M4): the tests that assert the hour are retired in that change,
  with this spec's ruling named in the commit message. By the planner's arithmetic on the
  pre-merge code, they are:

  | File | Test | Why it goes |
  |---|---|---|
  | `ranking.rs` | R3 `at_the_hour_a_task_due_tonight_leads_an_overdue_one` | the 2h overdue task still fits in front (2h + 1h plus the margin within 3h59) |
  | `ranking.rs` | R14 `evening_hour_is_twenty_hundred` | the constant goes; it says "changing it needs a new ruling" |
  | `cli.rs` | `an_evening_run_writes_the_evening_order_to_today_md` (C3) | at 21:00 on its `vault-s1` copy the 0.5h overdue reflection fits ahead of the probe and leads |
  | `cli.rs` | `now_without_today_ranks_the_day_of_now` (C5) | it asserts the probe first at 21:00, as C3 does |
  | `surface.rs` | `load_at_an_evening_moment_reorders_and_load_does_not` (S1) | the same probe-first assertion |
  | `surface.rs` | `build_state_after_the_hour_leads_must_do_with_due_today` (S2) | at 21:00 the first run is `Overdue` (the reflection) |

  `evening_rank.rs` (E1-E3) is **not** retired: its golden and reference checks and its bad-`--now`
  refusal hold under the formula. The other hour-era tests (R1, R2, R4-R13, R15, C1, C2, C4, S3,
  S5) are kept where they pass, re-pointed where their setup relied on the hour. **T0 runs every one
  of them on `main` and records each as retire or keep, by name**; this table is its starting point,
  not its result.
- **The ruled evening behaviour survives:** at 11 pm tonight's items lead (example (a)); a late
  item with a later window ranks below them; a no-window late item yields once it would cost
  tonight's work (step 3). Ruling (b) holds (only a window's close counts), (c) holds (no evening
  rewrite of `today.md`), (d) holds (the timeline stops at midnight, §2.3).
- **The alternative,** keeping 20:00 as a floor under the formula, is safer against estimates that
  run short; the margin is for that. Recommend against.

## 7. What the student sees

Plain, specific, never an instruction to give up. Knowlu names the item; the student decides.

- **On a `W` row** (a short line under the title, as the conflict and amend lines are):
  - a timed due: "Won't fit before 11:59 pm" (today), "Won't fit before Wed 11:59 pm" (later);
  - a bare-date due (no time in the note): "Won't fit by Wed", "Won't fit by tonight", never a
    precise "11:59 pm" the source never gave. The read model sees the raw `due` in `Loaded`;
  - with a known late window (after O6): "Won't fit before 11:59 pm · late until Fri, −20%";
  - a placeholder in play (§3): "May not fit before 11:59 pm · no time estimate yet" (its own), or
    "May not fit before 11:59 pm · depends on 2 items with no time estimate yet" (others in its
    prefix at removal, before or after it).
- **Its explanation** (on demand, beside the start-by `why`, which is unchanged) states the time
  left after the others (plan review r1, I3): "2.5h left; your 3.0h before it's due go to 2 others
  that then finish on time." With one other: "go to 1 other that then finishes on time." With none:
  "2.0h left; you have 1.5h before it's due." Your time counts evenings to midnight (§2.3).
- **On the meter,** when `W` is not empty, a line under the existing sentence: "If something has to
  give: Lab 3 (3.0h). Everything else fits by its deadline." With more than one: "If something has
  to give: Lab 3 and Essay draft (5.5h)." With a pinned Must do item: "Everything else fits by its
  deadline, not counting Lab 2, which you pinned." When a placeholder is in play: "This may not all
  fit: 2 items have no time estimate yet." No word in it names a day the overrun may not be on.
- **The spill hover** names a `W` item as "won't fit before Wed 11:59 pm" (or "by Wed") and any
  other spill item as today ("doesn't fit in today's hours").
- **Never** "skip", "drop", "give up" or "let go" in the page text: the item is still the
  student's to do. Quinn ruled these words (Q7).

The row line is a new optional read-model field, omitted from the JSON when absent. The console
renders it (`app/static`). Wording lives in the engine, as every other row line does.

## 8. Tests and oracles

**Frozen references (CLAUDE.md rule 2): none is regenerated, and two guards hold each one.**
- `golden-today-s1.md`, `golden-today-full.md`: `oracle.rs` runs `rank --today 2026-08-28` with no
  `--now`, so there is no moment and the formula does not run. Second guard: in all three fixture
  vaults Must do is exactly two past-due tasks with no window (`cs-100-hw-01`,
  `task-erste-reflexion`) and nothing dated, so step 0 finds today's order safe at any moment (no
  dated item to miss) and the order is the goldens'. A test drives `rank --today 2026-08-28 --now
  <T>` at 09:00, 19:30, 21:00 and 23:58 against both goldens (`evening_rank.rs`'s E1, extended in a
  new file).
- `surface-today-{s1,s1-migrated,full}.json`: **these have a moment** (`--now 2026-08-28T09:00`), so
  the formula runs on them. They stay byte-identical: step 0 is safe (same order); the two rows form
  one consecutive `Overdue` run whose rank order equals `sort_rows`'s order there; all three
  `spill_items` are empty; every new field is omitted when absent. A test proves it on the parsed
  blocks at 09:00 and 21:00 before any code changes (`evening_rank.rs`'s E2, extended in a new
  file). No §4.6 regeneration.
- `oracle.rs` and `surface_oracle.rs` run unedited and green at every commit.
- **Who writes the reference tests (review r1, M8; the controller's call, proposed here):** a test
  that reads a frozen reference or a `surface-today-*.json` and asserts against it is an oracle test
  in all but file name, so it is treated as contract-list work: `contract-engineer` writes it and
  `contract-reviewer` reviews it. The extended E1 and E2 go in a new file,
  `engine/tests/deadline_order_oracle.rs`. `evening_rank.rs` on `main` stays unedited, as the
  20:00 rule's plan ruled it (its Q-2: `test-writer`, reviewed by `contract-reviewer`); none of its
  tests is retired (§6).

**New behaviour tests** (names are the plan's; behaviour is not). In memory on the vault-s1 week
template where they can be, on temp-dir copies of a fixture vault where they need notes. **No
fixture is edited or added.**
- D1, no moment equals `rank`; a moment on another day equals `rank`; `m`'s seconds are ignored.
- D2, step 0: a safe pressure order is returned unchanged even with dated items (example (e)); an
  item finishing 29 minutes before its deadline makes it unsafe, 30 does not.
- D3, the timeline, over **ten consecutive dates** (two of each weekday from the start): each date
  runs from its window's start to the next date's `00:00`; an event on one Friday subtracts only
  there; the next Friday's evening is counted on its own; the filter applies before the cut at `m`;
  whole minutes; existing `free_blocks`, `window` and `with_day_window` results are unchanged; a
  `--window` preview applies to every matching weekday. Busy time at midnight (review r2, M2): a
  calendar event ending at the next date's `00:00` takes the evening to midnight; an event from
  23:00 to 01:00 takes 23:00–24:00 from its start date and nothing from the next; a commitment
  ending at 23:59 takes the evening to 23:59; a commitment written 22:00–02:00 is dropped, as
  today; a template class with its end before its start is dropped by the new method, so no
  minute is counted twice (while `free_blocks` keeps today's result for it).
- D4, the walk: example (d) names the longest; a tie names the later deadline, then the lower
  pressure; nothing is named when everything fits; example (f)'s two deadlines both fit, and adding
  a small task due tonight leaves `A` unchanged.
- D5, the merge: example (b) at 19:29 and 19:30; example (c); a shorter overdue item goes ahead of a
  longer one that cannot fit.
- D6, assembly: non-Must-do tasks keep their exact indexes; Must do membership unchanged; the
  designation, given `rank`'s list, gives today's takes; overrides win and are not charged.
- D7, placeholders (reviews r1 I4, r2 I1), Friday 21:00, 179 minutes left, Y a real 2.5h, X
  unestimated (1.0h stored), both due 23:59:
  - **X first** in pressure order: Y is removed and reads "may not fit · depends on 1 item";
  - **Y first** in pressure order: Y is removed when X's addition overflows, X comes after it, and
    Y still reads "may not fit · depends on 1 item";
  - both cases order exactly as the stored figures give; the walk's record for Y names X;
  - a placeholder `W` item reads "may not fit · no time estimate yet"; with no placeholder anywhere
    in the prefix the line reads "Won't fit".
- D8, determinism: two runs equal; reversed input of distinct keys gives the same order.
- D9, the read model: consecutive-run groups with rows in rank order; a task due 10:00 seen at
  15:00 is labelled `Overdue`; `spill_items` lists `W` first; totals unchanged; the row line only on
  `W` rows; a bare-date due prints no clock time; the pinned-item meter sentence.
- D10, `today.md` at a moment: an unsafe Must do is written in the new order, with no new line.

**Tests that change:** the hour tests §6 lists are retired in the replacing commit. Every other
existing test passes unedited; the plan's first task lists the engine and app tests that build a
page or run `rank` with dated Must do work at a clock time (app tests resolve a pinned date to
12:00) and confirms each.

## 9. VISION and the two rules

- **The test for every feature.** "What's next?" gets more honest: the order changes when today's
  would make something late, and the page says what cannot be saved before the evening it is lost.
- **Commitment 1.** Pure arithmetic over the notes, the calendar and the moment; integer minutes;
  total tie-breaks. **`rank` never calls a model**; estimates are data a judge wrote earlier.
- **Rule 1, no single-user assumptions.** No constant describes one student: the timeline is each
  vault's own window start, classes, commitments and calendar; midnight is ruled; the 30-minute
  margin is one value for everyone; the 20:00 constant, a single-student guess the evening spec
  flagged, goes away.
- **Rule 2.** §8: nothing regenerated, both guards proved by tests first.
- **Invariants.** No note field, no journal record, no run-record count, nothing new that syncs.
  Must do and the 15-a-day approvals budget are untouched.

## 10. Shape of the plan (for the planner who writes it)

**The interface between the ranking tasks and the read-model task**, settled now (review r1, M7):
`ranking.rs` exposes one pure function, `plan_at(tasks, today, cal, moment)`, returning the order
(what `rank_at` returns), whether step 0 found today's order safe, and for each `W` item its slug,
deadline, work minutes, the minutes `A(d)` gave it, and **the slugs of every other item in its
prefix at removal**: all items kept at that point, up to and including the one whose addition
overflowed, whether before or after it in the order (§2.6), plus the count and work of the kept
dated items due no later than it (for §7's explanation). `rank_at` becomes `plan_at(..).order`. `surface.rs` reads only that, plus the
frontmatter it already holds; it never recomputes the walk.

| Task | What | Agent | Why | After |
|---|---|---|---|---|
| T0 | Branch from `main` (at or after `7b3e10c`); re-confirm symbols; run every hour-era test in `ranking.rs`, `cli.rs` and `surface.rs` against the formula's expectations and record each by name as retire or keep (§6's table is the start) | main session | git and judgment | none |
| T1a | Red: `weekcal.rs` per-date method, D3 | `test-writer` | specified failing tests | T0 |
| T2a | `weekcal.rs`: the per-date method; `blocks_from_busy` takes a `DateTime` end internally | `implementer` (high) | a calendar every page reads; existing results must not move | T1a |
| T1b | Red: `ranking.rs` D1, D2, D4-D8 against `plan_at` | `test-writer` | as T1a | T2a |
| T2b | `ranking.rs`: deadlines, timeline, steps 0-1, `plan_at`'s type | `implementer` (high) | reorders every page | T1b |
| T2c | `ranking.rs`: steps 2-3, assembly, the 20:00 step removed; `cli.rs`: designation from `rank`'s list | `implementer` (high) | as T2b; the run path that writes `today.md` | T2b |
| T3 | Red: `surface.rs` D9, D10; `deadline_order_oracle.rs` (E1, E2 extended) | `test-writer` for D9-D10; `contract-engineer` for the oracle file | the oracle file asserts against frozen and read-model references, so it is **treated as contract-list work** (§8) | T2c |
| T4 | `surface.rs`: `plan_at`, groups, labels, meter, the row line, designation from `rank`'s list | `implementer` (high) | the read model; references must not move | T3 |
| T5 | The row line and the meter line in the console | `console-ui` | `app/static` | T4 |
| T6 | Docs: `engine-commands.md`'s `rank`, `anatomy.md` §3.4 and the Must do block, HANDOFF | `docs-keeper` | references | T4 |
| T7 | Review (`contract-reviewer` on the oracle file); scratch demo of examples (b), (c), (e) | `reviewer`; main session | pre-push; scratch vault only | T5, T6 |

T4 runs after T2c, because it consumes `plan_at`. T2a, T2b-T2c and T4 edit disjoint files
(`weekcal.rs`; `ranking.rs` and `cli.rs`; `surface.rs`); T5 only `app/static`. Each implementer
task is sized to stay under about 80 lines of change. Nothing on the contract list changes (the new
oracle file is treated as contract-list work, T3).

## 11. Quinn's answers (2026-10-24)

**All ten ruled as recommended, signed directly by Quinn on 2026-10-24:** Q1, overdue work leads
while it costs nothing; Q2, no daytime reorder when nothing is at risk; Q3, the planned day plus
evenings to midnight; Q4, a 30-minute margin; Q5, the 8 pm rule removed; Q6, a new heading each
time the label changes, and a due time already passed today shows `Overdue`; Q7, "Won't fit before
11:59 pm" / "Won't fit by Wed" and the meter line as written; Q8, pins not counted yet; Q9, the
longest item, then the later due, then the less urgent; Q10, placeholders give "may not fit".

The questions as they were asked, with the recommendations Quinn approved:

- **Q1. When should overdue work come first?** Today the list is ordered by how far behind each
  item is, so an overdue item usually leads but not always; after 8 pm, tonight's work leads. The
  8 pm rule exists because tonight's on-time work is a sure grade and late work may not be.
  **Recommend: overdue work goes first only while doing it first still lets tonight's (and every
  other) on-time work finish with half an hour to spare.** Most of the day nothing changes; when it
  would cost an on-time grade, the on-time work goes first, at whatever hour that happens.
- **Q2. Should Knowlu reorder the daytime list when nothing is at risk?** Sorting strictly by due
  time would move things around all day: at 9 am a half-hour reflection due yesterday would jump
  above an 11-hour essay due tonight, though both fit easily (example (e)), and the noon and 6 pm
  `today.md` would change too. **Recommend: no. Keep the usual order (by how far behind) unless it
  would make something late or leave it under half an hour to spare; only then switch to the
  deadline order.**
- **Q3. Which hours count as the student's when Knowlu checks whether work fits?** Only the planned
  day (8 to 6 in the fixture) says everything due tonight won't fit by 8 pm, which is plainly
  wrong. Counting only the evening before each deadline makes answers flip oddly (example (f)).
  **Recommend: the planned day plus every evening up to midnight.** Students do work evenings, and
  midnight is already the ruled end of "tonight". The cost: someone who never works evenings is
  told "won't fit" a little late. The planned day still governs recommendations and the meter.
- **Q4. How much safety margin?** **Recommend: 30 minutes, used only when deciding whether today's
  order is safe and whether overdue work may go first; none when saying "won't fit".** That leans
  careful both ways: don't risk on-time work for late work, and don't tell a student something
  won't fit unless it truly can't. One value for everyone; a setting only if the pilot asks.
- **Q5. Remove the 8 pm rule once this ships?** **Recommend: yes.** Running both would flip the
  order at 8 pm for no reason the page could give. The plumbing the 8 pm rule built stays; only the
  hour goes, and the tests that check the hour are retired with this ruling named.
- **Q6. How should Must do be grouped on the page?** Grouping all "Due today" rows together can draw
  them out of order once an overdue item sits between two of them. **Recommend: a new heading each
  time the label changes, rows in Knowlu's order, and a task whose due time already passed today
  shows as "Overdue".** A heading may repeat in the rare case the order interleaves. One plain list
  with the label on each row fits VISION better but changes the page's shape; that belongs to the
  main-page redesign.
- **Q7. What should it say?** **Recommend:** "Won't fit before 11:59 pm" on the item (or "Won't fit by
  Wed" when the source gave no time), and on the meter "If something has to give: Lab 3 (3.0h).
  Everything else fits by its deadline." Never "skip", "drop", "give up" or "let go". No new line in
  `today.md` for now; the page carries it.
- **Q8. Should pinned items count against the time?** **Recommend: not yet.** A pin is the student's
  own choice, and counting it means guessing when they'll do it. The meter says when a pin is left
  out.
- **Q9. When something must give, which one does Knowlu name?** The method names the longest item,
  which keeps the most items on time. A student might rather lose a small quiz than a big lab.
  **Recommend: the longest for now; if two are equally long, the one due later, then the less
  urgent.** Weighting by grade share needs confirmed syllabus weights and its own spec.
- **Q10. What about items with no time estimate yet?** They carry a stand-in hour. **Recommend:
  count the stand-in so the order stays stable, but never say "won't fit" because of one: say "may
  not fit" and why ("no time estimate yet").**

## 12. Out of scope

- O6's late window itself (the deadline function has one place for its close, §2.2), and penalty
  steps (evening ruling b).
- The meter's capacity being the whole day's window rather than what is left from now: an existing
  gap, shared with the 20:00 rule; noted, not changed.
- Two `weekcal.rs` gaps the timeline works around rather than fixes (§2.3): a commitment written
  across midnight (a night shift) is dropped as inverted, so that evening reads as free; and a
  template class with its end before its start makes `free_blocks` count the time before it twice.
  Each is today's behaviour on every page and wants its own small fix.
- Nudges and lock-in reading `W` (VISION's "a deadline at real risk"); each needs its own spec.
- Weighting by grade share (Q9), a per-vault margin (Q4) or evening bound (Q3), learning from actual
  durations.
- Any change to pressure, `start_by`, slack, Must do membership, Recommended or the designation.
- `CLAUDE.md`: "same input, same order" already holds with the moment as an input.

## 13. Reviews r1 and r2 (FAIL), how each finding was folded in

**Review r2** (narrow FAIL):

| Finding | Where it landed |
|---|---|
| I1, the placeholder rule must take in the whole prefix at removal, up to and including the item that overflowed | §2.6 (the walk's record), §3 (both orders), §10 (`plan_at`'s record), D7 (the Y-first case) |
| I2, Q1's premise was false | Q1 opens with how the list is ordered today and after 8 pm |
| M1, "never changes another item's verdict" | §2.3: "never changes the time available to another item" |
| M2, busy spans at and across midnight; inverted spans | §2.3 (events, commitments, template classes; the new method drops inverted spans); D3; §12 |
| M3, "treated as contract-list work" in §10 | §10, T3's row |
| M4, the hour tests to retire | §6's table: R3, R14, C3, C5, S1, S2; `evening_rank.rs` kept; T0 records each by name |
| M5, PR #65 merged as `7b3e10c` | the header, §1, §6, §8, §10 (T0 branches from `main`), §11, §12 |

**Review r1** (FAIL):

| Finding | Where it landed |
|---|---|
| I1, `with_day_window` is weekday-keyed and drops a midnight end | §2.3: a new per-date method with a `DateTime` end bound, settled; §1 states the limit; D3 over ten dates |
| I2, verdicts can move the wrong way; an unrelated task can flip one; uncounted evenings | §2.3: every evening to midnight, one timeline independent of tasks, monotone in `m` and `d`; example (f); Q3 |
| I3, "nothing tight, exactly as today" was false | §0 and §2.4: step 0 keeps pressure order when safe; example (e); Q2 asks whether to reorder anyway |
| I4, a placeholder ahead of a named item | §3: "may not fit" when a placeholder is in play (widened in r3 to the whole prefix at removal, review r2 I1); D7; Q10 |
| M1, Q9 against §5 | §5 and Q9 now state one rule: most work, later deadline, lower pressure |
| M2, "the rest fits" with pins | §4 and §7: the meter sentence names the pinned item |
| M3, rounding `m`; fall-back day | §2.1: truncated to the minute; §2.3: civil minutes, the repeated hour counted once |
| M4, cut before or after the filter | §2.3: filter first, then the cut at `m` |
| M5, the designation's input | §4: decided, `rank`'s list; `moved` with it |
| M6, wording | §7: no "tonight" in the placeholder line; bare dates print no clock time; §4: past-due today reads `Overdue` |
| M7, the T2-T4 interface; T2 split; weekcal settled | §10: `plan_at`; T2a-T2c; T4 after T2c; §2.3 settles `weekcal.rs` |
| M8, who writes the reference tests | §8: `contract-engineer`, in a new `deadline_order_oracle.rs` |
| M9, the `--window` preview | §2.1: same moment, previewed calendar, weekday-keyed; D3 |
