# Deadline ordering: build plan

> **For agentic workers:** REQUIRED SUB-SKILL: use superpowers:subagent-driven-development (recommended)
> or superpowers:executing-plans to run this plan task by task. Steps use checkbox (`- [ ]`) syntax.

**Status: r4, awaiting plan review r4.** Review r3 (2026-10-24): FAIL on one point (the
explanation's count), folded in (§9.3). Review r2: narrow FAIL, 2 important, 1 minor,
folded in (§9.2). Review r1: FAIL, 4 important, 6 minor, folded in (§9.1). Q-1 to Q-3 approved by the controller as recommended, with r1 M1's re-point (§2,
§3). No task starts before the review passes (§7).

**Date:** 2026-10-24. **Lane:** deadline ordering (replaces the 20:00 evening step). **Base:** `main`
at or after `7b3e10c` (the evening rule's merge); T0 records the SHA.

**Spec:** `docs/specs/2026-10-24-deadline-ordering-formula.md`, **ruled by Quinn 2026-10-24**, all
ten questions as recommended (renamed from `...-formula-draft.md` in `ee083cb`). The rulings it
builds:
- **Q1** overdue work leads while it costs nothing (step 3, 30-minute margin);
- **Q2** no daytime reorder when nothing is at risk (step 0);
- **Q3** the timeline is the planned day plus every evening to local midnight;
- **Q4** a 30-minute margin on step 0 and step 3, none on a "won't fit" verdict;
- **Q5** the 20:00 step is removed; its plumbing (`rank_at`, `rank --now`, `load_at`) stays;
- **Q6** Must do groups are consecutive runs of one label, rows in rank order; a due time already
  passed today reads `Overdue`;
- **Q7** "Won't fit before 11:59 pm" / "Won't fit by Wed", the meter line as written; no new line in
  `today.md`;
- **Q8** pinned items are not charged in the walk;
- **Q9** the walk removes the most work, then the later deadline, then the lower pressure;
- **Q10** a placeholder estimate anywhere in the removed item's prefix gives "may not fit".

**Goal.** When today's pressure order would make a dated Must do item late or leave it under 30
minutes to spare, Must do is reordered by deadline (Jackson), the items that cannot fit are named
(Moore–Hodgson) and moved last, and overdue work goes as early as it can without costing an
on-time item. Otherwise the order is pressure order, exactly. Concretely:
1. `WeekCalendar` gains one date-keyed method: a date's free blocks from its window's start to the
   next date's `00:00`.
2. `ranking::plan_at` computes the plan; `rank_at` returns its order; `rank` is unchanged; the
   20:00 step is deleted.
3. `cli.rs` and `surface.rs` feed the designation `rank`'s list (pressure order), never the plan's.
4. The read model shows consecutive-run groups, moment-based labels, the won't-fit row line, the
   meter's "If something has to give" line and `W`-first `spill_items`; the console renders them.
5. Both golden files and the three `surface-today-*.json` references stay byte-identical, unedited
   and never regenerated, guarded by a contract-engineer oracle file written **before** any code.

**Architecture.**
- **Engine:** `weekcal.rs` (one method, one internal refactor), `ranking.rs` (`plan_at`, its types,
  the timeline, steps 0-3, assembly; the 20:00 step removed), `cli.rs` (designation input; hour
  tests retired or re-pointed), `surface.rs` (`Loaded` carries the plan; groups, labels, meter, rows;
  hour tests retired or re-pointed). No new command or flag.
- **App:** `app/static` renders the new optional fields. No Rust change in `app/`.
- **Nothing on the contract list changes.** The new oracle file is treated as contract-list work
  (spec §8): `contract-engineer` writes it, `contract-reviewer` reviews it.

**The interface between ranking and the read model** (spec §10), settled:

```rust
#[derive(Debug, Clone, PartialEq)]
pub struct WontFit {
    pub slug: String,
    pub deadline: DateTime,
    pub work_minutes: i64,
    pub available_minutes: i64, // A(d)
    pub prefix: Vec<String>,    // every other item kept at removal, up to and including the overflower
    pub prefix_minutes: i64,    // the prefix's work at removal
    pub overflower: String,     // the item whose addition overflowed (may be this one)
    pub overflow_deadline: DateTime, // its deadline
}
#[derive(Debug, Clone, PartialEq)]
pub struct DeadlinePlan { pub order: Vec<Task>, pub safe: bool, pub wont_fit: Vec<WontFit> }
pub fn plan_at(tasks: &[Task], today: Date, cal: &WeekCalendar, moment: Option<DateTime>) -> DeadlinePlan;
```

With no moment, or a moment on another day, `plan_at` returns `rank`'s order, `safe: true` and no
`wont_fit`. `rank_at(..)` becomes `plan_at(..).order`. `surface.rs` reads only this plus the
frontmatter `Loaded` already holds; it never recomputes the walk.

**Test labels** are prefixed so they never collide with the evening lane's R, C, S and E labels:
`DO` (the oracle file), `DW` (`weekcal.rs`), `DP` (`ranking.rs`), `DC` (`cli.rs`), `DS`
(`surface.rs`). Names are suggestions; behaviour is not.

**Code read for this plan:** the evening rule's worktree (`.claude/worktrees/evening`) as it stood
before merging. The planner had no shell; T0 re-confirms every symbol and line on `main`.

---

## 1. VISION and the two overriding rules

- **The test for every feature.** "What's next?" is more honest: the order changes exactly when the
  usual order would cost a deadline, and the page names what cannot be saved before it is lost.
- **Commitment 1, deterministic engine.** Integer minutes throughout (moment truncated to the
  minute, work `round_ties_even(hours × 60)`, civil-minute timeline); total tie-breaks (spec §5).
  Same notes, day, moment and calendar give the same order.
- **`rank` never calls a model** (decision 11). Estimates are data `judge` wrote earlier.
- **Rule 1, no single-user assumptions.** The timeline is each vault's own window start, classes,
  commitments and calendar; midnight is ruled; the margin is one constant for everyone; the 20:00
  constant, a one-student guess, is deleted.
- **Rule 2, frozen references.** `oracle.rs` runs `rank --today 2026-08-28` with no `--now`, so no
  moment and no formula. Independently, every fixture vault's Must do is two past-due tasks with no
  window and nothing dated, so step 0 is safe at any moment. T1o's oracle file proves both at four
  moments, green before any code changes and at every commit after. `oracle.rs` and
  `surface_oracle.rs` run unedited and green at every commit. Nothing is regenerated.
- **Invariants kept.** No note field, no journal record, no run-record count, nothing new that syncs.
  Must do membership, Recommended, the designation and the approvals budget are unchanged.
- **Privacy.** No data moves.

## 2. Global constraints

- `cargo build --workspace` and `cargo test --workspace` from the root, dev profile, **0 warnings**
  (the one accepted `.rsrc` line). TDD: each red commit (T1a-T1g) is followed by the task that
  turns it green. **The red state** is a test that compiles and fails at run time; stubs are
  signatures with `_`-prefixed parameters and `todo!("deadline T<n>")`, `pub` where a non-test build
  would otherwise warn. **No stub sits on a path an existing test runs** (`rank`, `rank_at`, `run`,
  `run_at`, `load`, `load_at`, `build_state`, `free_blocks` keep working until their green task).
- **Guards.** A test marked **guard** is expected to pass when written and must keep passing; it
  pins behaviour an earlier task already gives. Each red task names which of its tests are red and
  which are guards.
- **Contract list untouched** (`CLAUDE.md`'s list, the oracle, sync and entitlement tests,
  `engine/tests/fixtures/**`). A task that finds it needs one stops and reports.
- **No fixture edits, no new fixture files.** Tests that need notes copy a fixture vault to a temp
  directory (as `oracle.rs`'s `copy_vault` does) and add them there.
- **Existing tests pass unedited**, with two ruled exceptions, both in T3c, whose commit message
  names Quinn's Q5 ruling of 2026-10-24:
  - **retired** (deleted): the tests T0 records as asserting the hour (§T0's list);
  - **re-pointed** (edited, **the controller's explicit allowance, plan review r1 M1**): the last
    assertion of `cli.rs`'s `now_without_today_ranks_the_day_of_now`, and the 21:00 half of
    `surface.rs`'s `build_state_after_the_hour_leads_must_do_with_due_today`, whose 09:00 half is
    kept as is (the controller confirmed that reading after review r2). Each is pointed at the
    formula's result (§T3c): C5 at Erste Reflexion first, S2's 21:00 half at `groups[0].label ==
    "Overdue"` only (DS1 covers the row order). No other existing test is edited.
  `evening_rank.rs` is not edited. `#[ignore]`d tests stay ignored.
- **Size:** every implementer task is at most about 80 changed lines; every red task about 80 lines
  of tests. A task that would exceed it stops and reports for a split.
- **Tests never leave the machine.** Line endings LF in every file this lane edits.

## 3. Questions (all answered)

Quinn's ten are ruled. The controller's three were **approved as recommended on 2026-10-24**:
- **Q-1. T3c edits test modules in `cli.rs` and `surface.rs`.** Removing the hour makes hour tests
  there fail in the same commit, so T3c deletes the retire-listed ones and re-points the two named
  in §2 (test code only), although its source edits are `ranking.rs` and `cli.rs`. Every commit
  stays green; T1e and T4a start after T3c, so nothing else edits those modules then.
- **Q-2. Field names in the read model:** `Row.fit_text` and `Row.fit_why` (the row line and its
  explanation), `SpillItem.deadline`, `Meter.give_text`, each `Option<String>` with
  `#[serde(skip_serializing_if = "Option::is_none")]`, as the Grades strip already does.
- **Q-3.** `pub const FIT_MARGIN_MINUTES: i64 = 30` in `ranking.rs`, with a test pinning it to
  Quinn's Q4 ruling (as R14 pinned the hour); `work_minutes(task)` beside it.

## 4. Conflict map

| File | This lane's change | Task | Shared with |
|---|---|---|---|
| `engine/src/weekcal.rs` | the date-keyed midnight method; `blocks_from_busy` takes a `DateTime` end; inverted spans dropped in the new path only | T1a, T2 | anything reading `free_blocks` (unchanged results, pinned by DW8) |
| `engine/src/ranking.rs` | timeline, deadlines, `plan_at` and types, steps 0-3, assembly; `EVENING_HOUR`, `closes_tonight` and the evening branch deleted; the retire-listed R tests deleted | T1b, T1c, T1d, T3a, T3b, T3c | O6 later (the late-window close in the deadline function) |
| `engine/src/cli.rs` | the designation reads `rank`'s list; DC1; hour tests retired or re-pointed (Q-1) | T1d, T3c | **L5's `cli.rs` edit** if it lands first: rebase; T0 checks |
| `engine/src/surface.rs` | `Loaded` carries the plan; designation and `moved` from `rank`'s list; groups, labels, meter, row line; hour tests retired or re-pointed (Q-1) | T3c (tests), T1e, T4a, T1f, T4b, T1g, T4c | T0 checks open lanes on `surface.rs` |
| `engine/tests/deadline_order_oracle.rs` (new) | the golden and reference guards at four moments | T1o | none |
| `app/static/console.{js,css}` | render `fit_text`, `fit_why`, `give_text`, the spill hover's deadline | T5 | T0 checks open console lanes |
| docs | §T6 | T6 | docs-keeper's queue |

## 5. Tasks at a glance

| Task | What | Agent | Why that agent | Depends on | Size |
|---|---|---|---|---|---|
| T0 | Branch from `main`; citations; the clock and retire records | main session | git and judgment | plan review | — |
| T1o | The oracle guard file, green now (DO1, DO2) | `contract-engineer` | it asserts against frozen and read-model references: contract-list work (spec §8) | T0 | S |
| T1a | Red: `weekcal.rs` DW1-DW8 | `test-writer` | the failing tests, specified | T1o | S |
| T2 | `weekcal.rs`: the midnight method | `implementer` (high) | the calendar every page reads; existing results must not move | T1a | S |
| T1b | Red: DP1-DP10 (deadlines, timeline, step 0, deadline order) | `test-writer` | as T1a | T2 | S |
| T3a | `ranking.rs`: types, timeline, steps 0-1, `plan_at` skeleton | `implementer` (high) | reorders every page; off the contract list | T1b | M (≤80) |
| T1c | Red: DP11-DP14, DP20 (the walk and its record) | `test-writer` | as T1a | T3a | S |
| T3b | `ranking.rs`: the walk, `W` last, the record | `implementer` (high) | as T3a | T1c | S |
| T1d | Red: DP15-DP19, DP21, DC1 (the merge, assembly, the run path) | `test-writer` | as T1a | T3b | S |
| T3c | `ranking.rs`: the merge; the hour deleted; `cli.rs` designation; retire and re-point | `implementer` (high) | the run path that writes `today.md` | T1d | M (≤80) |
| T1e | Red: DS1-DS4 (order, groups, labels, designation) | `test-writer` | as T1a | T3c | S |
| T4a | `surface.rs`: the plan in `Loaded`, designation input, groups, labels | `implementer` (high) | the read model; references must not move | T1e | S |
| T1f | Red: DS5, DS12, DS13 (the meter and its line; the preview) | `test-writer` | as T1a | T4a | S |
| T4b | `surface.rs`: the meter and `give_text` | `implementer` (high) | as T4a | T1f | S |
| T1g | Red: DS6-DS11, DS14 (the row line, placeholders, words) | `test-writer` | as T1a | T4b | S |
| T4c | `surface.rs`: the row line and placeholders | `implementer` (high) | as T4a | T1g | S |
| T5 | Console: the row line and the meter line | `console-ui` | `app/static` | T4c | XS |
| T6 | Docs | `docs-keeper` | references, HANDOFF, the evening spec's pointer | T4c | XS |
| T7 | Review; scratch demo | `reviewer`; `contract-reviewer` (T1o's file); main session | everything a cheaper agent changed is reviewed before a push | T5, T6 | — |

The chain is serial: each red task reads the green code before it, and T4 consumes `plan_at`.
T5 and T6 may run in parallel (disjoint files).

## T0. Branch, citations and records (main session)

- [ ] Branch `feat/deadline-ordering` from `main` at or after `7b3e10c`, with `docs/deadline-ordering`
  (the ruled spec under its new name and this plan, `ee083cb` or later) merged first or carried on
  the branch; record both SHAs in §9.
- [ ] Re-confirm on `main` every symbol this plan cites: `ranking::{pressure, rank, rank_at,
  EVENING_HOUR, closes_tonight, partition_must_do, designate_today, designate_today_explained}`,
  `cli::run_with_at` (its `rank_at` and `designate_today` calls), `surface::{Loaded, load_at,
  build_state_with, build_state_preview, with_window, meter, must_do, group_label, sort_rows, row,
  Row, SpillItem, Meter}`, `weekcal::{WeekCalendar::free_blocks, with_day_window, window,
  blocks_from_busy, class_busy, weekly_span_block, active_spans, subtract}`,
  `commitments::moved`, `models::{coerce_datetime, Task::remaining_hours}`. Record drift in §9.
- [ ] **The clock record.** `oracle.rs` runs `rank --today 2026-08-28` with no `--now`;
  `surface_oracle.rs` pins `--now 2026-08-28T09:00`; every fixture vault's Must do is
  `cs-100-hw-01` and `task-erste-reflexion`, both past due, no `late_policy` anywhere, all three
  `spill_items` empty. Write it into §9.
- [ ] **The retire record.** Check every hour-era test by name against the formula's expected
  result (by arithmetic, before code): `ranking.rs` R1-R15, `cli.rs` C1-C5, `surface.rs` S1-S5,
  `evening_rank.rs` E1-E3. Record each as **retire**, **re-point** or **keep**. The starting point
  (plan review r1, I1 and M1):
  - **retire:** R3 `at_the_hour_a_task_due_tonight_leads_an_overdue_one`, R4
    `closing_tonight_orders_by_due_time_then_pressure`, R7 `tonight_ends_at_local_midnight`, R14
    `evening_hour_is_twenty_hundred`, R15 `everything_else_keeps_the_pressure_order`, C3
    `an_evening_run_writes_the_evening_order_to_today_md`, S1
    `load_at_an_evening_moment_reorders_and_load_does_not`;
  - **re-point:** C5 `now_without_today_ranks_the_day_of_now` (its last assertion) and S2
    `build_state_after_the_hour_leads_must_do_with_due_today` (its 21:00 half);
  - **keep:** the rest, and all of `evening_rank.rs`.
  A test that would fail for any reason other than asserting the hour is a finding, not a
  retirement.
- [ ] List the app and engine tests that build a page or run `rank` with **dated** Must do work at a
  clock time (app tests pin 12:00); record which could change.
- [ ] Note open lanes touching `weekcal.rs`, `ranking.rs`, `cli.rs`, `surface.rs`, `app/static`.
- [ ] `cargo test --workspace` green with 0 warnings on the base.

## T1o. The oracle guard, green now (contract-engineer)

**Brief:** this section, §1 (rule 2), spec §8. New file `engine/tests/deadline_order_oracle.rs`,
copying `oracle.rs`'s copy-to-temp and footer strip (copied, not imported); **never writes under
`tests/fixtures/`**. Both tests are **guards**: green now (the evening rule leaves the fixtures'
order alone too) and green, unedited, at every later commit.
- **DO1** `rank_at_any_moment_renders_both_goldens`: `rank --today 2026-08-28 --now 2026-08-28T<T>`
  for `T` in 09:00, 19:30, 21:00, 23:58, on copies of `vault-s1` and `vault-s1-migrated` (against
  `golden-today-s1.md`) and `vault-full` (against `golden-today-full.md`).
- **DO2** `surface_at_nine_and_nine_pm_keeps_the_references_blocks`: `surface --view today --today
  2026-08-28 --now 2026-08-28T<T> --build-sha pinned`, `T` in 09:00 and 21:00, on each fixture in
  place (`surface` never writes): `must_do`, `recommended`, `meter` and `nav_counts` without
  `runs_warn` equal those parsed from the matching `surface-today-*.json` (the clock-driven
  regions differ for clock reasons and are not compared, as `evening_rank.rs`'s E3 does).

Acceptance: both green on the base; `contract-reviewer` reads the file before T1a (§7). Size: S.

## T1a. Red: `weekcal.rs` (test-writer)

**Brief:** this section, §2, spec §2.3 ("the timeline", "busy time at and across midnight", "the
mechanism"). Tests in `weekcal.rs`'s `mod tests`, on the vault-s1 template (window 08:00–18:00,
Friday classes 12:00–12:50 and 13:00–13:45, 45-minute minimum), events and commitment spans built
in memory.

**Stub:** `pub fn free_blocks_to_midnight(&self, _day: Date) -> Vec<Block>` as
`todo!("deadline T2")`. Nothing existing calls it. **Red:** DW1-DW7. **Guard:** DW8.
- **DW1** `midnight_blocks_run_from_window_start_to_midnight`: Friday 2026-08-28 gives 08:00–12:00
  and 13:45 to `2026-08-29T00:00` (the 10-minute gap is filtered).
- **DW2** `midnight_blocks_are_keyed_by_date_over_ten_days`: for 2026-08-24 to 2026-09-02, an event
  on Monday 08-24 19:00–21:00 subtracts there and not on Monday 08-31; every date ends at its next
  date's `00:00`.
- **DW3** `an_event_ending_at_midnight_takes_the_evening`: 20:00 to `00:00` next date: the last
  block ends at 20:00.
- **DW4** `an_event_crossing_midnight_takes_only_its_start_dates_evening`: 23:00 to 01:00: the start
  date loses 23:00–24:00; the next date's blocks equal those with no event.
- **DW5** `commitments_at_midnight`: a Friday span 19:00–23:59 ends the evening block at 19:00 (the
  minute after 23:59 is under the minimum); a span written 22:00–02:00 is dropped, so the evening
  is as with no span.
- **DW6** `an_inverted_class_counts_no_minute_twice`: a template whose Friday lists `["14:00",
  "13:00"]`: the midnight blocks do not overlap and sum to no more than the window's length.
  `free_blocks` for that template is not asserted (spec §12's separate fix).
- **DW7** `the_midnight_method_honours_the_window_start`: after `with_day_window(friday, 10:00,
  16:00)`, every Friday in a ten-day range starts at 10:00 and still ends at midnight.
- **DW8** `existing_blocks_are_unchanged` (guard): `free_blocks`, `template_blocks` and `capacity`
  for the vault-s1 and vault-full Fridays equal the literal values the base gives, which the
  test-writer reads off the base and writes in.

Acceptance: DW1-DW7 fail at run time on the stub; DW8 green; everything else green. Size: S.

## T2. `weekcal.rs`: the midnight method (implementer)

**Brief:** spec §2.3, T1a's tests.
- [ ] `blocks_from_busy` delegates to a private walk taking its end as a `DateTime`; the existing
  path passes `DateTime::from_parts(day, window_end)` exactly as now.
- [ ] `free_blocks_to_midnight(day)`: busy as `template_blocks` builds it (classes and
  `active_spans`), with every span whose end is not after its start dropped; the walk from the
  window's start to `(day + 1 day)` at `00:00`; then each event subtracted (`subtract`); then the
  minimum filter. Keyed by the date; mutates nothing.
- [ ] Doc comment: why midnight (spec Q3, evening ruling d), and that `free_blocks` keeps today's
  behaviour for an inverted class.

Acceptance: DW1-DW8 green; every existing `weekcal.rs`, `commitments.rs` and `surface.rs` test, and
`oracle.rs`, `surface_oracle.rs` and T1o's file green unedited. Size: S (about 35 lines).

## T1b. Red: `ranking.rs`, the timeline and step 0 (test-writer)

**Brief:** this section, §2, spec §2.1-§2.5 and §5. Tests in `ranking.rs`'s `mod tests`, reusing its
`D`, `at`, `week` and `task` helpers. Every setup that claims a pressure order asserts it under
`rank` first, so a green result cannot come from a weak setup.

**Stubs:** `pub const FIT_MARGIN_MINUTES: i64 = 30` (the real value); `WontFit` and `DeadlinePlan`
as in the header, with their derives; `pub fn work_minutes(_task: &Task) -> i64`, `pub struct
Timeline` with `pub fn build(_cal: &WeekCalendar, _from: DateTime, _until: DateTime) -> Timeline`
and `pub fn before(&self, _d: DateTime) -> i64`, and `pub fn plan_at(..) -> DeadlinePlan`, each
`todo!("deadline T3a")`. `rank` and `rank_at` are not touched. **Red:** DP2-DP10. **Guard:** DP1.
- **DP1** `fit_margin_is_thirty_minutes` (guard): its message says Quinn's Q4, 2026-10-24.
- **DP2** `work_is_whole_minutes`: 2.79h is 167; 0.5h is 30; progress applies (3.1h at 10%).
- **DP3** `the_timeline_runs_to_midnight_and_cuts_at_the_moment`: on Friday, `before(23:59)` from
  21:00 is 179, from 15:00 is 539, from 09:00 is 794.
- **DP4** `the_cut_comes_after_the_block_filter`: from 11:30, `before(12:00)` is 30 (a piece under
  the minimum, cut from a real block, still counts).
- **DP5** `available_time_never_shrinks_as_the_deadline_moves_later`: from Thursday 19:00,
  `before(Fri 07:59)` is 300 and `before(Fri 12:00)` is 540.
- **DP6** `the_moment_is_truncated_to_the_minute`: `plan_at` at 21:00:45 equals `plan_at` at 21:00;
  a task due 21:00 seen at 21:00:30 is overdue, not on time.
- **DP7** `no_moment_or_another_day_is_rank`: with `None`, and with a moment on `D+1` or `D-1`, the
  order equals `rank`, `safe` is true and `wont_fit` is empty.
- **DP8** `a_safe_pressure_order_is_kept_with_dated_items` (spec example (e)): an 11h essay due
  `D 23:59` and a 0.5h reflection due `D-1` (importance set so `rank` puts the essay first; setup
  asserts it); at `D 09:00` the order equals `rank` and `safe` is true.
- **DP9** `under_thirty_minutes_spare_is_unsafe`: a 3h overdue lab and a 1h task due `D 23:59`
  (setup: the lab first under `rank`): `safe` at 19:29, not at 19:30.
- **DP10** `unsafe_dated_items_go_in_deadline_order`: at `D 19:30`, a 2h task due 23:59 (importance
  4) and a 0.5h task due 21:00 (importance 2); setup: the 23:59 task first under `rank`; the plan
  puts the 21:00 task first and names nothing. A second pair due at the same time keeps `rank`'s
  order.

Acceptance: DP1 green; DP2-DP10 fail at run time; everything else green, 0 warnings. Size: S.

## T3a. `ranking.rs`: types, timeline, steps 0 and 1 (implementer)

**Brief:** spec §2.1-§2.5, §5; T1b's tests.
- [ ] `work_minutes`: `(remaining_hours() * 60.0).round_ties_even() as i64`.
- [ ] `Timeline::build`: `from` truncated to the minute; for each date from `from`'s to `until`'s
  (at most 365), `free_blocks_to_midnight`, each block clipped to start at `from`; `before(d)`
  sums whole minutes of the parts before `d`.
- [ ] The deadline function: on time when `due > m`, else overdue (no window until O6); its doc
  comment says it is **the one place O6's window close goes**, and penalty steps are out.
- [ ] `plan_at`: the moment guard (as `rank_at` has it); the scored list as `rank` builds it;
  overrides taken out; Must do (slack ≤ 0) split from the rest; step 0 over pressure order with
  `FIT_MARGIN_MINUTES`; when safe, `rank`'s order. When unsafe: dated items by `(deadline, scored
  index)`, then overdue items in scored order (the walk and the merge come in T3b and T3c),
  refilled into Must do's positions; overrides inserted as `rank` does.
- [ ] `rank_at`, `EVENING_HOUR` and `closes_tonight` are untouched: the evening rule still runs.

Acceptance: DP1-DP10 green; every existing test, T1o's file and both oracles green unedited. Size: M
(about 80 lines).

## T1c. Red: the walk and its record (test-writer)

**Brief:** this section, §2, spec §2.6, §5. Same helpers and setup rule as T1b. `plan_at` exists
from T3a, so the red tests fail on their assertions, not on a panic. **Red:** DP11, DP12, DP14.
**Guards** (T3a already gives them): DP13, DP20.
- **DP11** `the_walk_names_the_longest` (example (d)): X 1h, Y 2.5h, Z 1.5h, all due `D 23:59`, in
  that `rank` order; at `D 21:00` the order is X, Z, Y and `wont_fit` is exactly Y with
  `work_minutes` 150, `available_minutes` 179, `prefix` `[X]`, `prefix_minutes` 60, `overflower` Y
  (it overflowed itself) and `overflow_deadline` `D 23:59`.
- **DP12** `a_walk_tie_names_the_later_deadline_then_the_lower_pressure`: equal work, different
  deadlines: the later one is named; equal work and deadline: the one later in `rank`'s order.
- **DP13** `an_unsafe_order_can_name_nothing` (guard): DP9's pair at 19:30: order task, lab;
  `wont_fit` empty.
- **DP14** `the_prefix_names_a_later_overflower`: Y 2.5h and X 1h, both due `D 23:59`, at 21:00.
  With Y first under `rank`: Y is named and its `prefix` is `[X]` although X comes after it. With
  X first: Y is named, `prefix` `[X]`. And DP21's set without D (plan review r2, I1): B (2h due
  22:30, 90 minutes) is named with `prefix` `[]`, `overflower` B and `overflow_deadline` 22:30; C
  (2.5h due 23:30) fits alone (150 of 150 minutes) but is named when K (1h due 23:59) overflows,
  with `prefix` `[K]`, `prefix_minutes` 60, `overflower` K and `overflow_deadline` `D 23:59`.
- **DP20** `the_plan_is_deterministic` (guard): two calls equal; reversing an input whose keys all
  differ gives the same plan.

Acceptance: DP11, DP12, DP14 fail at run time; DP13, DP20 green; everything else green. Size: S.

## T3b. The walk, `W` last and the record (implementer)

**Brief:** spec §2.6, §2.8, §5; T1c's tests.
- [ ] The walk over the deadline order in integer minutes; removal by §5's key (most work, later
  deadline, later scored index); no margin.
- [ ] Each `WontFit` records, **at the moment of its removal** (spec §2.6): `prefix` (every other
  item kept at that point, up to and including the overflower), `prefix_minutes` (their work),
  `overflower` (the item whose addition overflowed, which may be itself) and `overflow_deadline`.
  Nothing in the record is computed after the walk ends.
- [ ] Assembly gains `W` last, by `(deadline, scored index)`. Overdue items still follow the kept
  items in scored order (the merge is T3c's).

Acceptance: DP1-DP14 and DP20 green; every existing test, T1o's file and both oracles green
unedited. Size: S (about 40 lines).

## T1d. Red: the merge, assembly and the run path (test-writer)

**Brief:** this section, §2, spec §2.7-§2.9, §4 (overrides, designation, `today.md`). Same setup
rule. **Red:** DP17, DP21, DC1. **Guards** (T3a or T3b already give them): DP15, DP16, DP18, DP19.
- **DP15** `the_lab_leads_until_1929_and_yields_at_1930` (example (b), guard): at 15:00 and 19:29,
  lab then task (equal to `rank`); at 19:30, task then lab.
- **DP16** `a_daytime_clash_puts_the_essay_first` (example (c), guard): an 11h essay due `D 23:59`,
  a 3h lab due 2026-08-20; setup: the lab first under `rank`; at `D 09:00`, essay then lab; nothing
  named.
- **DP17** `a_shorter_overdue_item_goes_ahead_of_a_longer_one`: 2.5h and 0.5h overdue, 1h due
  `D 23:59`, in that `rank` order; at 21:00: 0.5h, 1h, 2.5h.
- **DP18** `everything_outside_must_do_keeps_its_index` (guard): with a task due in ten days and an
  undated one, every non-Must-do task sits at `rank`'s index; Must do as a set equals `rank`'s.
- **DP19** `overrides_win_and_are_not_charged` (guard): a 3h overdue task with `rank_override: 1`
  and a 1h task due `D 23:59`, at 21:00: the pinned task first, the other second, nothing named.
- **DP21** `the_merge_runs_before_wont_fit_items`: at 21:00, K 1h due 23:59, B 2h due 22:30, C 2.5h
  due 23:30 and a 0.5h overdue D: `wont_fit` B then C; the order is D, K, B, C (D fits ahead of K
  with the margin: 30 + 60 ≤ 179 − 30). The walk: B overflows itself (120 > 90); C fits (150 ≤
  150); K overflows (210 > 179) and C, the longest of C and K, is removed.
- **DC1** (in `cli.rs`'s tests) `an_unsafe_run_writes_the_new_order_and_no_new_line`: a temp-dir
  copy of `vault-s1` with a 1h probe due `D 23:59` (an `id:`, `due`, `effort_hours`);
  `run_with_at` with `--today 2026-08-28 --now 2026-08-28T21:00`: the first Must do line of
  `state/today.md` is Erste Reflexion's (it fits ahead of the probe; CS 100 HW 01 does not); at
  `T09:00` it is CS 100 HW 01's; no line of either contains "fit".

Acceptance: DP17, DP21 and DC1 fail at run time on their assertions; the guards green; everything
else green. Size: S.

## T3c. The merge, the hour removed, the designation (implementer)

**Brief:** spec §2.7, §4 (designation), §6; T1d's tests; T0's retire record; §2's re-point
allowance; Q-1.
- [ ] The merge (spec §2.7): overdue items tried in scored order against
  `t + work + kept(..=i) ≤ before(d_i) − FIT_MARGIN_MINUTES` for every remaining kept item; with
  none placeable, the next kept item. Then `W`.
- [ ] `rank_at(..)` becomes `plan_at(..).order`. Delete `EVENING_HOUR`, `closes_tonight` and the
  evening branch; `rank_at`'s doc comment describes the formula.
- [ ] `cli.rs`'s run path: `designate_today` (and anything else T0 found reading `ranked` for the
  designation) is given `rank(&tasks, today, &cal)`; `render_today` and the gauge keep `ranked`.
- [ ] Delete the tests T0 recorded as **retire**. **Re-point** (§2): C5's last assertion now
  expects Erste Reflexion first at 21:00; S2's 21:00 half now asserts only `groups[0].label ==
  "Overdue"` (DS1 covers the row order), and its 09:00 half is untouched. Test code only (Q-1). The
  commit message names Quinn's Q5 ruling and the controller's re-point allowance.

Acceptance: DP1-DP21 and DC1 green; every test T0 recorded as keep green unedited, the two
re-pointed tests green; `evening_rank.rs`, T1o's file, `oracle.rs` and `surface_oracle.rs` green
unedited. Size: M (about 60 lines, not counting deleted tests).

## T1e. Red: the read model's order (test-writer)

**Brief:** this section, §2, spec §4 (designation, the Must do block). Scratch vaults are temp-dir
copies of `vault-s1`; a "probe" is a task note added to the **copy** (`id:`, `due`,
`effort_hours`). `D` is 2026-08-28. **Red:** DS1-DS3. **Guard:** DS4 (it may already pass, since
the designation's reservation is nearly order-free; T0's record notes which).
- **DS1** `must_do_groups_are_consecutive_runs_in_rank_order`: a 1h probe due `D 23:59`; at
  `D 21:00`, `must_do.groups` are `Overdue` (Erste Reflexion), `Due today` (the probe), `Overdue`
  (CS 100 HW 01); at `D 09:00`, `Overdue` (both) then `Due today`.
- **DS2** `a_due_time_passed_today_reads_overdue`: a probe due `D 10:00` is labelled `Overdue` at
  `D 15:00` and `Due today` at `D 09:00`.
- **DS3** `rows_inside_a_run_keep_rank_order`: probes of 1h due `D 21:30` and 4.5h due `D 23:59`; at
  `D 18:00` the `Due today` rows are the 21:30 probe then the 23:59 one (`sort_rows` would invert
  them: the 4.5h probe's start-by is earlier).
- **DS4** `the_designation_and_moved_read_pressure_order` (guard): on DS1's vault at 21:00, `takes`
  and `moved` equal those of `load(vault, D)`.

Acceptance: DS1-DS3 fail at run time on their assertions; DS4 recorded; everything else green.
Size: S.

## T4a. `surface.rs`: the plan, the designation and the groups (implementer)

**Brief:** spec §2.1, §4; T1e's tests.
- [ ] `Loaded` gains `plan: DeadlinePlan` and `moment: Option<DateTime>`. `load_at` computes
  `scored = rank(..)` and `plan = plan_at(.., moment)`; `ranked` is `plan.order`; `takes` and
  `moved` read `scored`. The preview path (`build_state_preview`) is unchanged: it already passes
  the page's `now`.
- [ ] `must_do` with a moment: a group per consecutive run of one label, rows in rank order (no
  `sort_rows`); with no moment, exactly as today.
- [ ] `group_label` with a moment: `due ≤ m` reads `Overdue`; otherwise the date rule.
- [ ] `note_detail` keeps `load` (a row's fields do not depend on order).

Acceptance: DS1-DS4 green; `surface_oracle.rs`, T1o's file and `build_state_never_writes` green
unedited. Size: S (about 50 lines).

## T1f. Red: the meter and its line (test-writer)

**Brief:** this section, §2, §3 Q-2, spec §3 ("when an estimate is a placeholder"), §4 (the meter)
and §7, Quinn's Q7 and Q10. Scratch vaults as T1e. Unless a test says otherwise: X 1h, Y 2.5h,
Z 1.5h, all due `D 23:59`, seen at `D 21:00` (179 minutes), in that pressure order (importance set
so; setup asserts). **A placeholder** (plan review r1, M3) is a note with `effort_confidence: unset`
or no `effort_hours` key; `needs_enrichment` alone does not make one.

**Stubs:** `SpillItem.deadline` and `Meter.give_text`, `Option<String>` with
`#[serde(skip_serializing_if = "Option::is_none")]`, always `None`; with both `None` the references
serialise as before (T1o proves it). **Red:** DS5, DS12. **Guard:** DS13.
- **DS5** `spill_items_list_wont_fit_first_and_totals_hold`: `spill_items[0]` is Y with `deadline`
  `"2026-08-28T23:59"`; `fit_blocks`, `spill_blocks`, `capped` and `spill_hours` equal the 09:00
  page's for the same vault.
- **DS12** `the_meter_line_names_what_has_to_give`: one item: "If something has to give: Y (2.5h).
  Everything else fits by its deadline."; two (DP21's set): "If something has to give: B and C
  (4.5h)."; a pinned Must do item present: the first sentence, then "Everything else fits by its
  deadline, not counting <its title>, which you pinned."; X a placeholder: "This may not all fit:
  1 item has no time estimate yet." (two: "2 items have"). No `W`: `give_text` absent.
- **DS13** `the_window_preview_ranks_at_the_page_moment` (guard after T4a): `build_state_preview` at
  `D 21:00` with a window string: its Must do rows are in `plan_at`'s order on the previewed
  calendar.

Acceptance: DS5 and DS12 fail at run time; DS13 green; everything else green, T1o's file included.
Size: S.

## T4b. `surface.rs`: the meter and its line (implementer)

**Brief:** spec §3, §4 (the meter), §7; Quinn's Q7 and Q10; T1f's tests.
- [ ] **Placeholder** (one helper, used by T4c too): `effort_confidence: unset`, or no
  `effort_hours` key, in the note's frontmatter (`metas`). Nothing else counts.
- [ ] `meter`: `W` first in `spill_items` (`deadline` set), then today's first-fit spill of the rest
  in rank order; the totals' formulas untouched.
- [ ] `give_text` per DS12: titles joined "A and B" / "A, B and C", hours summed at one decimal
  (`round1`); the pin clause when a pinned Must do item exists; the placeholder sentence when a
  placeholder is any `W` item or in any `W` item's `prefix`.

Acceptance: DS1-DS5, DS12, DS13 green; T1o's file, `surface_oracle.rs` and every existing
`surface.rs` test green unedited. Size: S (about 50 lines).

## T1g. Red: the row line and placeholders (test-writer)

**Brief:** as T1f, plus spec §7's row line and explanation (plan review r1, I3). **Stubs:**
`Row.fit_text` and `Row.fit_why`, as T1f's. **Red:** DS6-DS11. **Guard:** DS14.
- **DS6** `the_row_line_is_only_on_wont_fit_rows`: Y's `fit_text` is "Won't fit before 11:59 pm";
  X's and Z's are absent.
- **DS7** `a_later_day_names_its_weekday`: a 5h task due Saturday 08:30, seen Friday 21:00 (3h of
  Friday evening and 30 minutes of Saturday morning): "Won't fit before Sat 8:30 am".
- **DS8** `a_bare_date_due_prints_no_clock_time`: with more work than fits, `due: 2026-08-28` reads
  "Won't fit by tonight" and `due: 2026-08-29` reads "Won't fit by Sat".
- **DS9** `a_placeholder_in_the_prefix_reads_may_not_fit_in_both_orders`: X and Y only; X's note
  carries `effort_confidence: unset`; with X first, and again with Y first (spec §3's two cases), Y reads
  "May not fit before 11:59 pm · depends on 1 item with no time estimate yet". A note with no
  `effort_hours` key counts the same.
- **DS10** `a_placeholder_wont_fit_item_reads_no_time_estimate_yet`: the named item's own estimate
  is the placeholder: "May not fit before 11:59 pm · no time estimate yet".
- **DS11** `fit_why_comes_from_the_prefix_at_removal` (plan review r2, I1), on DP21's set at 21:00:
  - **C**, which fits alone (150 of 150 minutes) but was removed when K overflowed: "2.5h left;
    doing it would leave other work due by 11:59 pm late." It must **not** read "you have 2.5h
    before it's due", which would read as fitting;
  - **B**, which does not fit alone (120 minutes of work, 90 available): "2.0h left; you have 1.5h
    before it's due.";
  - example (d)'s **Y**, which overflowed itself: "2.5h left; doing it would leave other work due
    by 11:59 pm late." (doing Y would leave both X and Z late, which is why no count is printed);
  - an overflower with a bare-date due prints "due by tonight" or "due by Sat", never a clock time;
  - no explanation contains "1 other" (plan review r3).
- **DS14** `needs_enrichment_alone_is_not_a_placeholder` (guard after T4b): X with a real
  `effort_confidence: medium` and `needs_enrichment: true`: Y reads "Won't fit before 11:59 pm".

Acceptance: DS6-DS11 fail at run time; DS14 green; everything else green. Size: S.

## T4c. `surface.rs`: the row line and placeholders (implementer)

**Brief:** spec §3, §7; Quinn's Q7 and Q10; T1g's tests.
- [ ] **Bare date** (one helper): the raw `due` value in `metas` has no time part.
- [ ] `row`: `fit_text` and `fit_why` for `W` rows only, per DS6-DS11, using T4b's placeholder
  helper and the `WontFit` record. `fit_why`: when `work_minutes > available_minutes`, "<work> left;
  you have <available> before it's due."; otherwise "<work> left; doing it would leave other work
  due by <overflow_deadline> late." No count and no item name (plan review r3): doing it can leave
  more than one item late, and the late one can be due earlier than the overflower; the only true
  claim is that the work due by that time cannot all be on time. `overflower` is the item whose
  addition overflowed (this item when it overflowed itself) and supplies the deadline and whether
  it is a bare date. Times print as "11:59 pm", "8:30 am"; another day prefixes its short weekday
  (`WEEKDAYS`); a bare date (this item's or the overflower's) prints "tonight" or the weekday alone
  ("due by tonight", "due by Sat").
- [ ] No new wording outside these fields; `render.rs` is untouched (Q7: no `today.md` line).

Acceptance: DS1-DS14 green; T1o's file, `surface_oracle.rs` and every existing `surface.rs` test
green unedited. Size: S (about 60 lines).

## T5. Console: the row line and the meter line (console-ui)

**Brief:** spec §7, Q-2's names. In `app/static`:
- [ ] A row with `fit_text` shows it as a short line under the title, styled like the conflict and
  amend lines; `fit_why` shows on demand where the start-by `why` does.
- [ ] `meter.give_text`, when present, renders under the capacity sentence.
- [ ] The spill hover names an item with a `deadline` by its row's `fit_text`, any other as
  "doesn't fit in today's hours".
- [ ] Nothing renders when the fields are absent, so the fixture pages look as today.

Acceptance: the console fixture page (`?fixture=`, headless Edge) shows DS5's state with the line
and the hover, and the three reference pages unchanged; screenshots in §9. Size: XS.

## T6. Docs (docs-keeper)

Each edit by its text, never by line number.
- [ ] `docs/reference/engine-commands.md`, the `rank` entry: the evening sentence is replaced by the
  formula in two sentences (the usual order unless it would cost a deadline; then deadline order,
  won't-fit last, overdue work first while it costs nothing); `--now` unchanged.
- [ ] `docs/surface/anatomy.md`: §3.4's "After 20:00 the spill list follows the evening order"
  bullet becomes `W` first and the `give_text` line; the Must do block: consecutive runs,
  moment-based labels, `fit_text` and `fit_why`.
- [ ] The evening spec's status line gains "the 20:00 step is replaced by
  `docs/specs/2026-10-24-deadline-ordering-formula.md` (Quinn's Q5, 2026-10-24)".
- [ ] `HANDOFF.md`: the lane's row (spec, plan, branch, state) and the follow-ups: O6's hook (the
  deadline function), the two `weekcal.rs` gaps (spec §12), weighting by grade share.

Acceptance: `scripts/ci/eol-check.ps1` clean; no source file in the diff. Size: XS.

## T7. Review and scratch demo (reviewer; contract-reviewer; main session)

- [ ] **Review.** `reviewer` reviews the lane against this plan and the spec; report
  `docs/reports/2026-10-2X-deadline-ordering-review.md` (the day it runs). It checks, besides the
  code: no contract-list file and no fixture in the diff; `oracle.rs`, `surface_oracle.rs`,
  `evening_rank.rs` and T1o's file unedited after their commits and green at every commit; the
  goldens and the three references byte-identical (`git diff --stat` shows none); only T0's
  retire-listed tests deleted and only the two named tests re-pointed; 0 warnings; the CI line;
  T0's records in §9. `contract-reviewer` re-reads `deadline_order_oracle.rs` against `oracle.rs`
  and `surface_oracle.rs`.
- [ ] **Scratch demo.** On a scratch vault (`scripts\scratch-vault.ps1 -Source
  engine\tests\fixtures\vault-s1-migrated`), add a 1h task due `2026-08-28T23:59` and run
  `surface --view today --today 2026-08-28 --now 2026-08-28T19:29 --build-sha pinned`, then with
  `T19:30` and `T21:00`. **Expected** (the template's window ends 18:00, so the evening is the
  timeline's): at 19:29, safe: `Overdue` (CS 100 HW 01, Erste Reflexion), then `Due today`; at
  19:30, unsafe: CS 100 HW 01, the task, Erste Reflexion, in three runs; at 21:00: Erste
  Reflexion, the task, CS 100 HW 01. Then, on a second scratch vault, example (d)'s three tasks due
  `2026-08-28T23:59` at `T21:00`: Y carries "Won't fit before 11:59 pm" and the meter's line names
  it. Record each group's first row and the lines in §9; optionally render with `?fixture=`.
  Delete the scratch vaults afterwards. No account, no live vault, no OS input.

---

## 6. Fidelity ledger

| Spec | Requirement | Task | Proof |
|---|---|---|---|
| §2.1 | Runs only with a moment on the ranked day; otherwise `rank`'s order | T3a | DP7 |
| §2.1 | The moment truncated to the minute | T3a | DP6 |
| §2.1 | Must do permuted in place; every other index unchanged; membership unchanged | T3a, T3c | DP18 |
| §2.1, M9 | The `--window` preview ranks at the page's moment on the previewed calendar | T4a | DS13 |
| §2.2 | On time when `due > m`; due at `m` is overdue; bare date reads 23:59; one place for O6's close | T3a, T4c | DP6, DS8; the deadline function's doc comment, checked at T7 |
| §2.3, Q3 | Each date from its window's start to the next date's `00:00`, keyed by date | T2 | DW1, DW2, DW7 |
| §2.3 | Filter first, then the cut at `m`; whole civil minutes | T2, T3a | DW1, DP3, DP4 |
| §2.3 | Never shrinks as `d` moves later; independent of tasks | T3a | DP5 (`Timeline::build` takes no task) |
| §2.3, r2 M2 | Busy time at and across midnight; overnight commitment dropped; inverted class counts no minute twice | T2 | DW3, DW4, DW5, DW6 |
| §2.3 | Existing `free_blocks` results unchanged | T2 | DW8 |
| §2.4, Q2, Q4 | Step 0: pressure order kept when every dated item has 30 minutes spare | T3a | DP8, DP9, DP1 |
| §2.5 | Deadline order, ties in scored order | T3a | DP10 |
| §2.6, Q9 | Moore–Hodgson: the most work removed, then the later deadline, then the lower pressure; no margin | T3b | DP11, DP12, DP13 |
| §2.6, r2 I1 | The record: the whole prefix at removal, up to and including the overflower | T3b | DP11, DP14 |
| §2.7, Q1, Q4 | Overdue first while every kept item keeps 30 minutes; a shorter one may go first | T3c | DP15, DP16, DP17, DP21 |
| §2.8 | `W` last, in deadline order, after the merge; never hidden | T3b, T3c | DP11, DP21 |
| §2.9 | Examples (b), (c), (d), (e), (f) | T3a-T3c | DP15, DP16, DP11, DP8, DP5 |
| §3, Q10 | Placeholders (`effort_confidence: unset` or no `effort_hours`) order as stored; "may not fit" when the item or its prefix has one | T3b, T4b, T4c | DP14, DS9, DS10, DS12, DS14 |
| §4, Q8 | Overrides win and are not charged; the meter names the pin | T3c, T4b | DP19, DS12 |
| §4, M5 | The designation and `moved` read `rank`'s list | T3c, T4a | DC1 (run path), DS4 |
| §4, Q6 | Consecutive runs, rows in rank order; a passed due today reads `Overdue` | T4a | DS1, DS2, DS3 |
| §4 | The meter's totals unchanged; `W` first in `spill_items` | T4b | DS5 |
| §4, Q7 | `today.md` in the new order, no new line | T3c | DC1 |
| §5 | Integer minutes; total tie-breaks; deterministic | T3a-T3c | DP2, DP12, DP20 |
| §6, Q5 | The 20:00 step deleted; hour tests retired or re-pointed by name; plumbing kept | T0, T3c | T0's record; T7 |
| §7, Q7 | Row line, bare dates, weekday, meter line, words never used | T4b, T4c, T5 | DS6, DS7, DS8, DS12; T5 screenshots |
| §7 (edited after the ruling, r2 I1, r3) | The explanation from the prefix at removal: "you have Xh" only when the item does not fit alone, else "would leave other work due by … late", with no count | T3b, T4c | DP11, DP14, DS11 |
| §8 | Goldens and references byte-identical, never regenerated, at four moments | T1o | DO1, DO2; `oracle.rs`, `surface_oracle.rs` unedited |
| §8, M8 | The oracle file is contract-list work | T1o, T7 | `contract-engineer` wrote it; `contract-reviewer`'s read |
| §9 | `rank` model-free; no note field, no journal record | all | T7 |
| (plan) | No contract-list file, no fixture; no existing test edited except the retire list and the two re-points | all | T7 |

## 7. Checkpoints

1. **Plan review** (`reviewer`, plan mode) before T0; findings answered in §9, verdict on the
   status line.
2. **After T0:** the controller reads the clock and retire records before T1o.
3. **After T1o:** `contract-reviewer` reads `deadline_order_oracle.rs` (green on the base) before any
   code changes.
4. **After each red commit** (T1a-T1g): the controller checks which tests fail and why, that each
   named guard passes, and that `oracle.rs`, `surface_oracle.rs`, `evening_rank.rs` and T1o's file
   are green, before the matching implementer task.
5. **After T3c:** the controller checks the deleted and re-pointed tests against T0's record, name
   by name.
6. **T7** before the push; the lane PR merges on green CI and the reviewer's PASS.

## 8. Out of scope

- O6's late window (the deadline function's hook only), penalty steps, weighting by grade share.
- The meter's capacity being the whole day's window rather than what is left from now.
- The two `weekcal.rs` gaps (spec §12): an overnight commitment, and `free_blocks` with an inverted
  class. Neither is fixed here.
- Nudges and lock-in reading `W`; any `today.md` line; any app Rust change.
- `CLAUDE.md`: "same input, same order" holds with the moment as an input; no rule line changes.

## 9. Fidelity notes

*(T0's base, citation, clock and retire records, the review rounds and the controller's answers go
here, dated, newest first.)*

- **2026-10-25, T7 scratch demo (the controller), at `f79a3c3`'s engine.** Each part ran on its own scratch copy of `vault-s1-migrated`. Both copies were deleted afterwards. No account, live vault or OS input was used.
  - **The added task:** 1h, due `2026-08-28T23:59`.
    - **19:29 (safe):** Overdue [CS 100 HW 01, Erste Reflexion], then Due today [the task].
    - **19:30 (unsafe):** three runs: Overdue [CS 100 HW 01], Due today [the task], Overdue [Erste Reflexion].
    - **21:00:** Overdue [Erste Reflexion], Due today [the task], Overdue [CS 100 HW 01].
    - This matches the plan's expectation at every clock.
  - **Example (d) at 21:00:** X (1h), Y (2.5h) and Z (1.5h), all due 23:59, with the s1 tasks removed. Every line matches §4 (d) and Q8.
    - Order: X, Z, Y.
    - Y's `fit_text` is "Won't fit before 11:59 pm".
    - Y's `fit_why` is "2.5h left; doing it would leave other work due by 11:59 pm late."
    - `give_text` is "If something has to give: Task Y (2.5h). Everything else fits by its deadline."
    - `spill_items` is [Task Y, `2026-08-28T23:59`].

- **2026-10-25, T4c review (FAIL, two Important) and Quinn's rulings (signed).**
  - **I-1, rounding:** "3.0h left; you have 3.0h" printed for 180 against 179 minutes. Fix: work
    rounds **up** and available time rounds **down** to the tenth, so the reason never reads as
    fitting. A test at 179 against 180.
  - **I-2, the drawer:** `note_detail` loads with no moment, so its row has no `fit_text`/`fit_why`.
    **Controller ruling:** the console (T5) shows the row line and its reason from the list row's
    own fields, in the row and in its drawer, and never from `note_detail`. T5's brief carries it, so
    the drawer cannot disagree with the row (R22). No engine change.
  - **Quinn, "can't fit alone":** when an item's own work is more than the time before its own
    deadline (`work_minutes > available_minutes`), its row reads "Won't fit …" even if a placeholder
    is in its prefix, because no estimate could make it fit. Its own placeholder still reads
    "May not fit … · no time estimate yet". A test.
  - **Quinn, the stored 1.0h:** kept. A placeholder's reason prints "1.0h left; …" as the spec reads.
  - **Tests for T4c's rulings:** a bare date with a placeholder in its prefix reads "May not fit by
    Sat · …"; an item that is itself a placeholder with another in its prefix reads "… · no time
    estimate yet".

- **2026-10-25, T4b review (FAIL, two Important) and Quinn's ruling (signed) on the pin clause.**
  - **I-1, the pin is named in every form of the line (Q8).** Quinn approved the wording:
    several items: "If something has to give: B and C (4.5h). Everything else fits by its deadline,
    not counting Lab Pin, which you pinned."; a placeholder: "If something has to give: Y (2.5h).
    This may not all fit: 1 item has no time estimate yet, and Lab Pin, which you pinned, isn't
    counted." Two DS12 cases are added for them in the fix round.
  - **I-2, a bare-date deadline:** `SpillItem.deadline` is the date alone (`"2026-08-28"`) when the
    note's `due` has no time, so the hover never prints an 11:59 pm the source never gave (§7). A
    DS5 case pins it.
  - **Carried to T5:** the console must not hide `spill_items` when `spill_blocks == 0`.
  - **T1g's DS14** asserts "never May not fit" until T4c; after T4c a test-writer tightens it to
    the plan's exact "Won't fit before 11:59 pm".

- **2026-10-25, Quinn's ruling (signed), from T4a's review m-2: "Overdue" follows the clock
  everywhere on the page.** `group_label` reads `due <= m` as Overdue (T4a). `horizon()`, which drives
  `nav_counts.overdue` and the Overdue view, still goes by date, so a task due 10:00 read at 15:00 is
  Overdue in Must do but "This week" in the nav. Quinn ruled that the nav follows the moment too.
  **New task T4d** (after T4c, before T5): T1h first (red, a probe due `D 10:00` at `D 15:00` is in
  `nav_counts.overdue` and the Overdue view and not in This week; at `D 09:00` it is not overdue);
  then `horizon()` takes the page's moment, with no moment exactly as today. `surface_oracle.rs`
  must stay green unedited; if it moves, stop and report. `test-writer` then `implementer`; size S.
- **2026-10-25, T4a review m-1 (folded into T4b):** `Loaded.moment` is stored truncated to the
  minute, as `plan_at`'s is. **m-3 (carried to T1g):** a guard that `moved` reads `scored` needs a
  planning-day note in the scratch copy; DS4's `moved` half is `None` on both sides.

- **2026-10-24, checkpoint: contract-reviewer on T1o (`088c8f1`): PASS.**
  - **The coverage rule.** DO2 compares four blocks, by the plan's ruling. `evening_rank.rs`'s E3, which is kept unedited, still guards `the_day`, `verdict`, `ahead` and `topline` at 21:00, and `surface_oracle.rs` guards every byte at 09:00. **Any later lane that removes E3 must first widen DO2 to E3's set.**
  - **The doc comment.** DO2's doc comment calls the excluded blocks clock-driven. Only `now`, `revision`, the runs panel and `runs_warn` are; the rest are excluded by the plan's ruling. The comment is left as committed, and this note records the correction.
- **2026-10-24, T0 (the controller).**
  - **Base.** `feat/deadline-ordering` was branched from `origin/main` at `7b3e10c`, and `docs/deadline-ordering` (`6eff47d`, plan r4, PASS) was merged as `b1de78f`.
  - **Citations.** Every symbol in T0's list exists on the base. There is no drift in names. Visibility is recorded for the implementers:
    - private: `ranking::closes_tonight`; `surface::{with_window, build_state_with, sort_rows, group_label}`; `weekcal::{subtract, class_busy, weekly_span_block, active_spans, blocks_from_busy}`;
    - public: the rest.
    - Tests in each file's own module reach the private ones.
  - **Clock record.**
    - `oracle.rs:86` runs `--today 2026-08-28` with no `--now`.
    - `surface_oracle.rs:17,22,44` pins `--now 2026-08-28T09:00`.
    - No `late_policy` appears under `engine/tests/fixtures`.
    - All three `surface-today-*.json` have `"spill_items": []`.
  - **Retire record.** Line numbers are on the base.
    - **Retire:** R3 (:461), R4 (:469), R7 (:512), R14 (:615) and R15 (:624) in `ranking.rs`; C3 (`cli.rs:1663`); S1 (`surface.rs:4476`).
    - **Re-point:** C5 (`cli.rs:1707`, last assertion) and S2 (`surface.rs:4487`, the 21:00 half).
    - **Keep:** R1, R2, R5, R6 and R8-R13; C1, C2 and C4; S3 (:4501) and S4 (:4514); and E1-E3 (`evening_rank.rs`).
    - Label drift: `surface.rs`'s evening block holds four tests (S1-S4), not five. The test the reviews called "S5" (`the_meter_spills_the_overdue_lab_after_the_hour`) is S4, and is kept.
  - **Clock-dependent tests outside the evening blocks.**
    - `named_issues.rs` (surface at 09:00, issues panel only) and `surface_oracle.rs` (09:00). No app test calls `surface`, `rank` or `build_state` at a clock time; app `12:00`s are runner slot times.
  - **Open lanes.**
    - `feat/o3-long-onboarding` touches none of `weekcal.rs`, `ranking.rs`, `cli.rs` or `surface.rs`. It rewrites much of `app/static` (12 files), so T5 (`console.js`) carries a collision note: rebase onto whichever lands second.
  - **Workspace on the base:** `cargo test --workspace -j 2 --no-fail-fast` passes, with no failures and only the accepted `.rsrc` warning.
- **2026-10-24, the controller.** Q-1 to Q-3 approved as recommended. The re-point of C5's last
  assertion and of S2's 21:00 half is explicitly allowed (plan review r1 M1); §2 records it.

### 9.3 Plan review r3 (FAIL on one point; answered 2026-10-24)

1. **The count in the explanation:** "would leave 1 other (due 11:59 pm) late" could be false twice
   over: doing example (d)'s Y leaves both X and Z late, and the item left late can be due before the
   overflower. The wording is now "doing it would leave other work due by 11:59 pm late" ("due by
   tonight" / "due by Sat" for a bare date), and the justification "keeping this one makes exactly
   that overflow miss" is removed (T4c, DS11's C and Y; spec §7 and its after-ruling paragraph).
   Nothing else changed.

### 9.2 Plan review r2 (narrow FAIL; answered 2026-10-24)

1. **I1, the explanation's source:** `WontFit` drops `kept_count` and `kept_minutes` and records, at
   removal, `prefix`, `prefix_minutes`, `overflower` and `overflow_deadline` (header, T3b). "You have
   Xh before it's due" is printed only when the item does not fit alone; otherwise "doing it would
   leave 1 other (due …) late" (T4c; the count was dropped in r3, §9.3). DP14 pins DP21's B and C records; DS11 tests C (fits alone,
   removed when K overflows), B and Y. The spec's §7 and §10 follow, and its status line says the
   explanation was edited after the ruling.
2. **I2, the S2 re-point:** its 21:00 half asserts only `groups[0].label == "Overdue"`; DS1 covers
   the row order (§2, T3c). The controller confirmed the 09:00 reading.

### 9.1 Plan review r1 (FAIL; answered 2026-10-24)

1. **I1, the retire list:** R4, R7 and R15 join R3, R14, C3 and S1 (T0, §2).
2. **I2, guards:** DP13, DP15, DP16, DP18, DP19 and DP20 are marked guards; only DP11, DP12, DP14,
   DP17, DP21 and DC1 are red in their tasks (§2 defines a guard). DP21 was redesigned so the merge,
   not only `W`-last, decides it.
3. **I3, the explanation:** answered in r2 with `kept_count` and `kept_minutes`; **superseded by
   review r2's I1** (§9.2): the explanation now comes from the prefix at removal.
4. **I4, sizes:** the walk (T1c, T3b) is split from the merge (T1d, T3c); the meter (T1f, T4b) from
   the row line (T1g, T4c).
5. **M1:** C5's last assertion and S2's 21:00 half are re-pointed, not deleted (§2, T3c).
6. **M2:** `DeadlinePlan` and `WontFit` derive `Debug, Clone, PartialEq` (header, T1b).
7. **M3:** a placeholder is `effort_confidence: unset` or no `effort_hours`; `needs_enrichment`
   alone is not (T1f, T4b, DS14).
8. **M4:** T7 pins `--today 2026-08-28` and full `--now` stamps.
9. **M5:** new labels are prefixed `DO`, `DW`, `DP`, `DC`, `DS`.
10. **M6:** the rename is done (`ee083cb`); the header and T0 say so.
