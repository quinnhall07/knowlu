# Deadline ordering: build plan

> **For agentic workers:** REQUIRED SUB-SKILL: use superpowers:subagent-driven-development (recommended)
> or superpowers:executing-plans to run this plan task by task. Steps use checkbox (`- [ ]`) syntax.

**Status: draft, awaiting plan review r1.** No task starts before the review passes (§7).

**Date:** 2026-10-24. **Lane:** deadline ordering (replaces the 20:00 evening step). **Base:** `main`
at or after `7b3e10c` (the evening rule's merge); T0 records the SHA.

**Spec:** `docs/specs/2026-10-24-deadline-ordering-formula.md`, **ruled by Quinn 2026-10-24**, all
ten questions as recommended. (Committed as `...-formula-draft.md`; the controller renames it in
the commit that adds this plan, since nothing links to the old name.) The rulings it builds:
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
  the timeline, steps 0-3, assembly; the 20:00 step removed), `cli.rs` (designation input; the
  hour tests retired), `surface.rs` (`Loaded` carries the plan; groups, labels, meter, rows; its
  hour tests retired). No new command or flag.
- **App:** `app/static` renders two new optional fields. No Rust change in `app/`.
- **Nothing on the contract list changes.** The new oracle file is treated as contract-list work
  (spec §8): `contract-engineer` writes it, `contract-reviewer` reviews it.

**The interface between ranking and the read model** (spec §10), settled:

```rust
pub struct WontFit {
    pub slug: String,
    pub deadline: DateTime,
    pub work_minutes: i64,
    pub available_minutes: i64,   // A(d)
    pub prefix: Vec<String>,      // every other item kept at removal, up to and including the overflower
}
pub struct DeadlinePlan { pub order: Vec<Task>, pub safe: bool, pub wont_fit: Vec<WontFit> }
pub fn plan_at(tasks: &[Task], today: Date, cal: &WeekCalendar, moment: Option<DateTime>) -> DeadlinePlan;
```

With no moment, or a moment on another day, `plan_at` returns `rank`'s order, `safe: true` and no
`wont_fit`. `rank_at(..)` becomes `plan_at(..).order`. `surface.rs` reads only this plus the
frontmatter `Loaded` already holds; it never recomputes the walk.

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
  (the one accepted `.rsrc` line). TDD: each red commit (T1a-T1e) is followed by the task that
  turns it green. **The red state** is a test that compiles and fails at run time; stubs are
  signatures with `_`-prefixed parameters and `todo!("deadline T<n>")`, `pub` where a non-test build
  would otherwise warn. **No stub sits on a path an existing test runs** (`rank`, `rank_at`, `run`,
  `run_at`, `load`, `load_at`, `build_state`, `free_blocks` keep working until their green task).
- **Contract list untouched** (`CLAUDE.md`'s list, the oracle, sync and entitlement tests,
  `engine/tests/fixtures/**`). A task that finds it needs one stops and reports.
- **No fixture edits, no new fixture files.** Tests that need notes copy a fixture vault to a temp
  directory (as `oracle.rs`'s `copy_vault` does) and add them there.
- **Existing tests pass unedited**, with one ruled exception: the hour tests listed in T0's record
  (spec §6) are deleted in T3b, the task that removes the hour, and its commit message names
  Quinn's Q5 ruling. `evening_rank.rs` is not edited. `#[ignore]`d tests stay ignored.
- **Size:** every implementer task is at most about 80 changed lines; every red task about 80 lines
  of tests. A task that would exceed it stops and reports for a split.
- **Tests never leave the machine.** Line endings LF in every file this lane edits.

## 3. Questions

Quinn's ten are ruled. These are the controller's, each with the planner's recommendation.
- **Q-1. T3b edits test modules in `cli.rs` and `surface.rs`.** Removing the hour makes the hour
  tests there fail in the same commit, so T3b deletes them (test code only) although its source
  edits are `ranking.rs` and `cli.rs`. **Recommend: yes**, so every commit is green; T1d and T4
  start after T3b, so nothing else is editing those modules then.
- **Q-2. Field names in the read model.** `Row.fit_text` and `Row.fit_why` (the row line and its
  explanation), `SpillItem.deadline`, `Meter.give_text`, each `Option`, each
  `skip_serializing_if = "Option::is_none"`, as the Grades strip already does. **Recommend: these
  names**; `console-ui` and the docs follow them.
- **Q-3. Where the margin and the minute rule live.** **Recommend:** `pub const FIT_MARGIN_MINUTES:
  i64 = 30` in `ranking.rs`, with a test pinning it to Quinn's Q4 ruling (as R14 pinned the hour);
  `work_minutes(task)` beside it.

## 4. Conflict map

| File | This lane's change | Task | Shared with |
|---|---|---|---|
| `engine/src/weekcal.rs` | the date-keyed midnight method; `blocks_from_busy` takes a `DateTime` end; inverted spans dropped in the new path only | T1a, T2 | anything reading `free_blocks` (unchanged results, pinned by T1a) |
| `engine/src/ranking.rs` | timeline, deadlines, `plan_at` and types, steps 0-3, assembly; `EVENING_HOUR`, `closes_tonight` and the evening branch deleted; R3, R14 deleted | T1b, T1c, T3a, T3b | O6 later (the late-window close in the deadline function) |
| `engine/src/cli.rs` | the designation reads `rank`'s list; the hour tests deleted (Q-1) | T1c (C1), T3b | **L5's `cli.rs` edit** if it lands first: rebase; T0 checks |
| `engine/src/surface.rs` | `Loaded` carries the plan; designation and `moved` from `rank`'s list; groups, labels, meter, row line; the hour tests deleted (Q-1) | T1d, T4a, T1e, T4b | T0 checks open lanes on `surface.rs` |
| `engine/tests/deadline_order_oracle.rs` (new) | the golden and reference guards at four moments | T1o | none |
| `app/static/console.{js,css}` | render `fit_text`, `fit_why`, `give_text`, the spill hover's deadline | T5 | T0 checks open console lanes |
| docs | §T6 | T6 | docs-keeper's queue |

## 5. Tasks at a glance

| Task | What | Agent | Why that agent | Depends on | Size |
|---|---|---|---|---|---|
| T0 | Branch from `main`; citations; the clock and retire records | main session | git and judgment | plan review | — |
| T1o | The oracle guard file, green now | `contract-engineer` | it asserts against frozen and read-model references: contract-list work (spec §8) | T0 | S |
| T1a | Red: `weekcal.rs` W1-W8 | `test-writer` | the failing tests, specified | T1o | S |
| T2 | `weekcal.rs`: the midnight method | `implementer` (high) | the calendar every page reads; existing results must not move | T1a | S |
| T1b | Red: `ranking.rs` P1-P10 (deadlines, timeline, step 0, deadline order) | `test-writer` | as T1a | T2 | S |
| T3a | `ranking.rs`: types, timeline, steps 0-1, `plan_at` skeleton | `implementer` (high) | reorders every page; off the contract list | T1b | M (≤80) |
| T1c | Red: `ranking.rs` P11-P21 (walk, merge, assembly, record); `cli.rs` C1 | `test-writer` | as T1a | T3a | S |
| T3b | `ranking.rs`: steps 2-3, assembly, the hour deleted; `cli.rs` designation; hour tests retired | `implementer` (high) | the run path that writes `today.md` | T1c | M (≤80) |
| T1d | Red: `surface.rs` S1-S4 (order, groups, labels, designation) | `test-writer` | as T1a | T3b | S |
| T4a | `surface.rs`: the plan in `Loaded`, designation input, groups, labels | `implementer` (high) | the read model; references must not move | T1d | S |
| T1e | Red: `surface.rs` S5-S13 (meter, row line, words) | `test-writer` | as T1a | T4a | S |
| T4b | `surface.rs`: the meter, the row line, the wording | `implementer` (high) | as T4a | T1e | M (≤80) |
| T5 | Console: the row line and the meter line | `console-ui` | `app/static` | T4b | XS |
| T6 | Docs | `docs-keeper` | references, HANDOFF, the evening spec's pointer | T4b | XS |
| T7 | Review; scratch demo | `reviewer`; `contract-reviewer` (T1o's file); main session | everything a cheaper agent changed is reviewed before a push | T5, T6 | — |

The chain is serial: each red task reads the green code before it, and T4 consumes `plan_at`.
T5 and T6 may run in parallel (disjoint files).

## T0. Branch, citations and records (main session)

- [ ] Branch `feat/deadline-ordering` from `main` (at or after `7b3e10c`); record the SHA in §9.
- [ ] The spec rename is committed (`git mv` of `...-formula-draft.md` to `...-formula.md`).
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
- [ ] **The retire record.** Run every hour-era test by name against the formula's expected result
  (by arithmetic, before code): `ranking.rs` R1-R15, `cli.rs` C1-C5, `surface.rs` S1-S5,
  `evening_rank.rs` E1-E3. Record each as **retire** (asserts the hour) or **keep**. The spec's
  starting point: retire R3 `at_the_hour_a_task_due_tonight_leads_an_overdue_one`, R14
  `evening_hour_is_twenty_hundred`, `an_evening_run_writes_the_evening_order_to_today_md`,
  `now_without_today_ranks_the_day_of_now`, `load_at_an_evening_moment_reorders_and_load_does_not`,
  `build_state_after_the_hour_leads_must_do_with_due_today`; keep the rest and all of
  `evening_rank.rs`. A test that would fail for any reason other than asserting the hour is a
  finding, not a retirement.
- [ ] List the app and engine tests that build a page or run `rank` with **dated** Must do work at a
  clock time (app tests pin 12:00); record which could change.
- [ ] Note open lanes touching `weekcal.rs`, `ranking.rs`, `cli.rs`, `surface.rs`, `app/static`.
- [ ] `cargo test --workspace` green with 0 warnings on the base.

## T1o. The oracle guard, green now (contract-engineer)

**Brief:** this section, §1 (rule 2), spec §8. New file `engine/tests/deadline_order_oracle.rs`,
copying `oracle.rs`'s copy-to-temp and footer strip (copied, not imported); **never writes under
`tests/fixtures/`**. Both tests are **green now** (the evening rule leaves the fixtures' order
alone too) and must stay green, unedited, at every later commit.
- **O1** `rank_at_any_moment_renders_both_goldens`: `rank --today 2026-08-28 --now 2026-08-28T<T>`
  for `T` in 09:00, 19:30, 21:00, 23:58, on copies of `vault-s1` and `vault-s1-migrated` (against
  `golden-today-s1.md`) and `vault-full` (against `golden-today-full.md`).
- **O2** `surface_at_nine_and_nine_pm_keeps_the_references_blocks`: `surface --view today --today
  2026-08-28 --now 2026-08-28T<T> --build-sha pinned`, `T` in 09:00 and 21:00, on each fixture in
  place (`surface` never writes): `must_do`, `recommended`, `meter` and `nav_counts` without
  `runs_warn` equal those parsed from the matching `surface-today-*.json` (the clock-driven
  regions differ for clock reasons and are not compared, as `evening_rank.rs`'s E2 does).

Acceptance: both green on the base; `contract-reviewer` reads the file before T1a (§7). Size: S.

## T1a. Red: `weekcal.rs` (test-writer)

**Brief:** this section, §2, spec §2.3 ("the timeline", "busy time at and across midnight", "the
mechanism"). Tests in `weekcal.rs`'s `mod tests`, on the vault-s1 template (window 08:00–18:00,
Friday classes 12:00–12:50 and 13:00–13:45, 45-minute minimum), events and commitment spans built
in memory. Names are suggestions; behaviour is not.

**Stub:** `pub fn free_blocks_to_midnight(&self, _day: Date) -> Vec<Block>` as
`todo!("deadline T2")`. Nothing existing calls it.
- **W1** `midnight_blocks_run_from_window_start_to_midnight`: Friday 2026-08-28 gives 08:00–12:00
  and 13:45 to `2026-08-29T00:00` (the 10-minute gap is filtered).
- **W2** `midnight_blocks_are_keyed_by_date_over_ten_days`: for 2026-08-24 to 2026-09-02, an event on
  Monday 08-24 19:00–21:00 subtracts there and not on Monday 08-31; every date ends at its next
  date's `00:00`.
- **W3** `an_event_ending_at_midnight_takes_the_evening`: 20:00 to `00:00` next date: the last block
  ends at 20:00.
- **W4** `an_event_crossing_midnight_takes_only_its_start_dates_evening`: 23:00 to 01:00: the start
  date loses 23:00–24:00; the next date's blocks equal those with no event.
- **W5** `commitments_at_midnight`: a Friday span 19:00–23:59 ends the evening block at 19:00 (the
  minute after 23:59 is under the minimum); a span written 22:00–02:00 is dropped, so the evening is
  as with no span.
- **W6** `an_inverted_class_counts_no_minute_twice`: a template whose Friday lists `["14:00",
  "13:00"]`: the midnight blocks do not overlap and sum to no more than the window's length.
  `free_blocks` for that template is not asserted (spec §12's separate fix).
- **W7** `the_midnight_method_honours_the_window_start`: after `with_day_window(friday, 10:00,
  16:00)`, every Friday in a ten-day range starts at 10:00 and still ends at midnight.
- **W8** `existing_blocks_are_unchanged` (**green now**): `free_blocks`, `template_blocks` and
  `capacity` for the vault-s1 and vault-full Fridays equal the literal values the base gives, which
  the test-writer reads off the base and writes in.

Acceptance: W1-W7 fail at run time on the stub; W8 green; everything else green. Size: S.

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

Acceptance: W1-W8 green; every existing `weekcal.rs`, `commitments.rs` and `surface.rs` test, and
`oracle.rs`, `surface_oracle.rs` and T1o's file green unedited. Size: S (about 35 lines).

## T1b. Red: `ranking.rs`, the timeline and step 0 (test-writer)

**Brief:** this section, §2, spec §2.1-§2.5 and §5. Tests in `ranking.rs`'s `mod tests`, reusing its
`D`, `at`, `week` and `task` helpers. Every setup that claims a pressure order asserts it under
`rank` first, so a green result cannot come from a weak setup.

**Stubs:** `pub const FIT_MARGIN_MINUTES: i64 = 30` (the real value); `WontFit` and `DeadlinePlan`
as in the header; `pub fn work_minutes(_task: &Task) -> i64`, `pub struct Timeline` with `pub fn
build(_cal: &WeekCalendar, _from: DateTime, _until: DateTime) -> Timeline` and `pub fn before(&self,
_d: DateTime) -> i64`, and `pub fn plan_at(..) -> DeadlinePlan`, each `todo!("deadline T3a")`.
`rank` and `rank_at` are not touched.
- **P1** `fit_margin_is_thirty_minutes` (**green now**): its message says Quinn's Q4, 2026-10-24.
- **P2** `work_is_whole_minutes`: 2.79h is 167; 0.5h is 30; progress applies (3.1h at 10%).
- **P3** `the_timeline_runs_to_midnight_and_cuts_at_the_moment`: on Friday, `before(23:59)` from
  21:00 is 179, from 15:00 is 539, from 09:00 is 794.
- **P4** `the_cut_comes_after_the_block_filter`: from 11:30, `before(12:00)` is 30 (a piece under
  the minimum, cut from a real block, still counts).
- **P5** `available_time_never_shrinks_as_the_deadline_moves_later`: from Thursday 19:00,
  `before(Fri 07:59)` is 300 and `before(Fri 12:00)` is 540.
- **P6** `the_moment_is_truncated_to_the_minute`: `plan_at` at 21:00:45 equals `plan_at` at 21:00;
  a task due 21:00 seen at 21:00:30 is overdue, not on time.
- **P7** `no_moment_or_another_day_is_rank`: with `None`, and with a moment on `D+1` or `D-1`, the
  order equals `rank`, `safe` is true and `wont_fit` is empty.
- **P8** `a_safe_pressure_order_is_kept_with_dated_items` (spec example (e)): an 11h essay due
  `D 23:59` and a 0.5h reflection due `D-1`; setup: `rank` puts the essay first; at `D 09:00` the
  order equals `rank` and `safe` is true.
- **P9** `under_thirty_minutes_spare_is_unsafe`: a 3h overdue lab and a 1h task due `D 23:59`
  (setup: the lab first under `rank`): `safe` at 19:29, not at 19:30.
- **P10** `unsafe_dated_items_go_in_deadline_order`: at `D 19:30`, a 2h task due 23:59 (importance
  4) and a 0.5h task due 21:00 (importance 2); setup: the 23:59 task first under `rank`; the plan
  puts the 21:00 task first and names nothing. A second pair due at the same time keeps `rank`'s
  order.

Acceptance: P1 green; P2-P10 fail at run time; everything else green, 0 warnings. Size: S.

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
  index)`, then overdue items in scored order (steps 2 and 3 are T3b's), refilled into Must do's
  positions; overrides inserted as `rank` does.
- [ ] `rank_at`, `EVENING_HOUR` and `closes_tonight` are untouched: the evening rule still runs.

Acceptance: P1-P10 green; every existing test, T1o's file and both oracles green unedited. Size: M
(about 80 lines).

## T1c. Red: `ranking.rs`, the walk, the merge and assembly (test-writer)

**Brief:** this section, §2, spec §2.6-§2.9, §4 (overrides) and §5. Same helpers and setup rule as
T1b. `plan_at` exists from T3a, so these fail on their assertions, not on a panic.
- **P11** `the_walk_names_the_longest` (example (d)): X 1h, Y 2.5h, Z 1.5h, all due `D 23:59`, in
  that `rank` order; at `D 21:00` the order is X, Z, Y and `wont_fit` is exactly Y with
  `work_minutes` 150, `available_minutes` 179 and `prefix` `[X]`.
- **P12** `a_walk_tie_names_the_later_deadline_then_the_lower_pressure`: equal work, different
  deadlines: the later one is named; equal work and deadline: the one later in `rank`'s order.
- **P13** `an_unsafe_order_can_name_nothing`: P9's pair at 19:30: order task, lab; `wont_fit` empty.
- **P14** `the_prefix_names_a_later_overflower`: Y 2.5h and X 1h, both due `D 23:59`, at 21:00.
  Y first under `rank`: Y is named and its `prefix` is `[X]` although X comes after it. X first:
  Y is named, `prefix` `[X]`.
- **P15** `the_lab_leads_until_1929_and_yields_at_1930` (example (b)): at 15:00 and 19:29, lab then
  task (equal to `rank`); at 19:30, task then lab.
- **P16** `a_daytime_clash_puts_the_essay_first` (example (c)): an 11h essay due `D 23:59`, a 3h lab
  due 2026-08-20; setup: the lab first under `rank`; at `D 09:00`, essay then lab; nothing named.
- **P17** `a_shorter_overdue_item_goes_ahead_of_a_longer_one`: 2.5h and 0.5h overdue, 1h due
  `D 23:59`, in that `rank` order; at 21:00: 0.5h, 1h, 2.5h.
- **P18** `everything_outside_must_do_keeps_its_index`: with a task due in ten days and an undated
  one, every non-Must-do task sits at `rank`'s index; Must do as a set equals `rank`'s.
- **P19** `overrides_win_and_are_not_charged`: a 3h overdue task with `rank_override: 1` and a 1h
  task due `D 23:59`, at 21:00: the pinned task first, the other second, nothing named.
- **P20** `the_plan_is_deterministic`: two calls equal; reversing an input whose keys all differ
  gives the same plan.
- **P21** `wont_fit_items_go_last_in_deadline_order`: at 21:00, A 1h due 22:00, B 2h due 22:30, C 2h
  due 23:59 and a 0.5h overdue D: order A, D, B, C; `wont_fit` B then C.
- **C1** (in `cli.rs`'s tests) `an_unsafe_run_writes_the_new_order_and_no_new_line`: a temp-dir
  copy of `vault-s1` with a 1h probe due `D 23:59` (an `id:`, `due`, `effort_hours`);
  `run_with_at` with `--today D --now D 21:00`: the first Must do line of `state/today.md` is Erste
  Reflexion's (it fits ahead of the probe; CS 100 HW 01 does not); at `D 09:00` it is CS 100 HW
  01's; no line of either contains "fit".

Acceptance: P11-P21 and C1 fail at run time on their assertions; everything else green. Size: S.

## T3b. Steps 2 and 3, assembly, and the hour removed (implementer)

**Brief:** spec §2.6-§2.8, §4 (designation), §6; T1c's tests; T0's retire record; Q-1.
- [ ] The walk (spec §2.6) over the deadline order in integer minutes; removal by §5's key (most
  work, later deadline, later scored index); each `WontFit` records `prefix` as spec §2.6 defines
  it. No margin.
- [ ] The merge (spec §2.7): overdue items tried in scored order against
  `t + work + kept(..=i) ≤ before(d_i) − FIT_MARGIN_MINUTES` for every remaining kept item.
- [ ] Assembly (spec §2.8): merge, then `W` by `(deadline, scored index)`, refilled into Must do's
  positions; overrides inserted as `rank` does, not charged.
- [ ] `rank_at(..)` becomes `plan_at(..).order`. Delete `EVENING_HOUR`, `closes_tonight` and the
  evening branch; `rank_at`'s doc comment describes the formula.
- [ ] `cli.rs`'s run path: `designate_today` (and anything else T0 found reading `ranked` for the
  designation) is given `rank(&tasks, today, &cal)`; `render_today` and the gauge keep `ranked`.
- [ ] Delete the tests T0 recorded as **retire** (in `ranking.rs`, `cli.rs`, `surface.rs`; test
  code only, Q-1). The commit message names Quinn's Q5 ruling of 2026-10-24.

Acceptance: P1-P21 and C1 green; every test T0 recorded as keep green unedited; `evening_rank.rs`, T1o's
file, `oracle.rs` and `surface_oracle.rs` green unedited. Size: M (about 80 lines, not counting
deleted tests).

## T1d. Red: the read model's order and the run's `today.md` (test-writer)

**Brief:** this section, §2, spec §4 (designation, Must do block, `today.md`). Scratch vaults are
temp-dir copies of `vault-s1`; a "probe" is a task note added to the **copy** (`id:`, `due`,
`effort_hours`). `D` is 2026-08-28.
- **S1** `must_do_groups_are_consecutive_runs_in_rank_order`: a 1h probe due `D 23:59`; at
  `D 21:00`, `must_do.groups` are `Overdue` (Erste Reflexion), `Due today` (the probe), `Overdue`
  (CS 100 HW 01); at `D 09:00`, `Overdue` (both) then `Due today`.
- **S2** `a_due_time_passed_today_reads_overdue`: a probe due `D 10:00` is labelled `Overdue` at
  `D 15:00` and `Due today` at `D 09:00`.
- **S3** `rows_inside_a_run_keep_rank_order`: probes of 1h due `D 21:30` and 4.5h due `D 23:59`; at
  `D 18:00` the `Due today` rows are the 21:30 probe then the 23:59 one (`sort_rows` would invert
  them: the 4.5h probe's start-by is earlier).
- **S4** `the_designation_and_moved_read_pressure_order` (**a guard**: it may already pass, since
  the designation's reservation is nearly order-free, and must pass after T4a): on S1's vault at
  21:00, `takes` and `moved` equal those of `load(vault, D)`.

Acceptance: S1-S3 fail at run time on their assertions; S4 passes or fails, and T0's record notes
which; everything else green. Size: S.

## T4a. `surface.rs`: the plan, the designation and the groups (implementer)

**Brief:** spec §2.1, §4; T1d's tests.
- [ ] `Loaded` gains `plan: DeadlinePlan` and `moment: Option<DateTime>`. `load_at` computes
  `scored = rank(..)` and `plan = plan_at(.., moment)`; `ranked` is `plan.order`; `takes` and
  `moved` read `scored`. The preview path (`build_state_preview`) is unchanged: it already passes
  the page's `now`.
- [ ] `must_do` with a moment: a group per consecutive run of one label, rows in rank order (no
  `sort_rows`); with no moment, exactly as today.
- [ ] `group_label` with a moment: `due ≤ m` reads `Overdue`; otherwise the date rule.
- [ ] `note_detail` keeps `load` (a row's fields do not depend on order).

Acceptance: S1-S4 green; `surface_oracle.rs`, T1o's file and `build_state_never_writes` green
unedited. Size: S (about 50 lines).

## T1e. Red: the meter, the row line and the words (test-writer)

**Brief:** this section, §2, §3 Q-2, spec §3 ("when an estimate is a placeholder") and §7, Quinn's
Q7 and Q10. Scratch vaults as T1d. Unless a test says otherwise: X 1h, Y 2.5h, Z 1.5h, all due
`D 23:59`, seen at `D 21:00` (179 minutes), in that pressure order (importance set so; setup asserts).

**Stubs:** `Row.fit_text`, `Row.fit_why`, `SpillItem.deadline` and `Meter.give_text`, each
`Option<String>` with `#[serde(skip_serializing_if = "Option::is_none")]`, always `None`. With
every one `None`, the three references serialise as before (T1o proves it).
- **S5** `spill_items_list_wont_fit_first_and_totals_hold`: `spill_items[0]` is Y with `deadline`
  `"2026-08-28T23:59"`; `fit_blocks`, `spill_blocks`, `capped` and `spill_hours` equal the 09:00
  page's for the same vault.
- **S6** `the_row_line_is_only_on_wont_fit_rows`: Y's `fit_text` is "Won't fit before 11:59 pm"; X's
  and Z's are absent.
- **S7** `a_later_day_names_its_weekday`: a 5h task due Saturday 08:30, seen Friday 21:00 (3h of
  Friday evening and 30 minutes of Saturday morning): "Won't fit before Sat 8:30 am".
- **S8** `a_bare_date_due_prints_no_clock_time`: with more work than fits, `due: 2026-08-28` reads
  "Won't fit by tonight" and `due: 2026-08-29` reads "Won't fit by Sat".
- **S9** `a_placeholder_in_the_prefix_reads_may_not_fit_in_both_orders`: X's note carries
  `effort_confidence: unset` and `needs_enrichment: true`; with X first, and again with Y first
  (spec §3's two cases), Y reads "May not fit before 11:59 pm · depends on 1 item with no time
  estimate yet". A note with no `effort_hours` key counts as a placeholder too.
- **S10** `a_placeholder_wont_fit_item_reads_no_time_estimate_yet`: when the named item's own
  estimate is the placeholder: "May not fit before 11:59 pm · no time estimate yet".
- **S11** `fit_why_explains_the_hours`: Y's `fit_why` is "2.5h left and 3.0h of your time before
  it's due, counting evenings to midnight. Doing the others first keeps 2 on time." (179 minutes
  print as 3.0h, one decimal, half to even, as `round1`).
- **S12** `the_meter_line_names_what_has_to_give`: one item: "If something has to give: Y (2.5h).
  Everything else fits by its deadline."; two (T1c's P21 set): "If something has to give: B and C
  (4.0h)."; a pinned Must do item present: the first sentence, then "Everything else fits by its
  deadline, not counting <its title>, which you pinned."; a placeholder in play: "This may not all
  fit: 1 item has no time estimate yet." (two: "2 items have"). No `W`: `give_text` absent.
- **S13** `the_window_preview_ranks_at_the_page_moment`: `build_state_preview` at `D 21:00` with a
  window string: its Must do rows are in `plan_at`'s order on the previewed calendar.

Acceptance: S5-S12 fail at run time; S13 passes or fails (T4a may already give it) and must pass
after T4b; everything else green, T1o's file included. Size: S.

## T4b. `surface.rs`: the meter, the row line and the words (implementer)

**Brief:** spec §3, §4 (the meter), §7; Quinn's Q7 and Q10; T1e's tests.
- [ ] **Placeholder** (one helper): the note's frontmatter has `effort_confidence: unset`, or
  `needs_enrichment: true`, or no `effort_hours` key. A `W` item is "may not fit" when it or any
  slug in its `prefix` is a placeholder.
- [ ] **Bare date** (one helper): the raw `due` value in `metas` has no time part.
- [ ] `meter`: `W` first in `spill_items` (deadline set), then today's first-fit spill of the rest in
  rank order; the totals' formulas untouched. `give_text` per S12.
- [ ] `row`: `fit_text` and `fit_why` for `W` rows only, per S6-S11. Times print as "11:59 pm",
  "8:30 am"; another day prefixes its short weekday (`WEEKDAYS`).
- [ ] No new wording outside these fields; `render.rs` is untouched (Q7: no `today.md` line).

Acceptance: S1-S13 green; T1o's file, `surface_oracle.rs` and every existing `surface.rs` test green
unedited. Size: M (about 80 lines; if it runs over, split the meter from the row line and stop).

## T5. Console: the row line and the meter line (console-ui)

**Brief:** spec §7, Q-2's names. In `app/static`:
- [ ] A row with `fit_text` shows it as a short line under the title, styled like the conflict and
  amend lines; `fit_why` shows on demand where the start-by `why` does.
- [ ] `meter.give_text`, when present, renders under the capacity sentence.
- [ ] The spill hover names an item with a `deadline` by the row's `fit_text`, any other as
  "doesn't fit in today's hours".
- [ ] Nothing renders when the fields are absent, so the fixture pages look as today.

Acceptance: the console fixture page (`?fixture=`, headless Edge) shows S5's state with the line
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
  retire-listed tests deleted; 0 warnings; the CI line; T0's records in §9. `contract-reviewer`
  re-reads `deadline_order_oracle.rs` against `oracle.rs` and `surface_oracle.rs`.
- [ ] **Scratch demo.** On a scratch vault (`scripts\scratch-vault.ps1 -Source
  engine\tests\fixtures\vault-s1-migrated`), add example (b)'s 1h task due today 23:59 and run
  `surface --view today --today <D> --now <D>T19:29`, then `T19:30` and `T21:00`. **Expected**
  (the template's window ends 18:00, so the evening is the timeline's): at 19:29, safe: `Overdue`
  (CS 100 HW 01, Erste Reflexion), then `Due today`; at 19:30, unsafe: CS 100 HW 01, the task, Erste
  Reflexion, in three runs; at 21:00: Erste Reflexion, the task, CS 100 HW 01. Then, on a second
  scratch vault, example (d)'s three tasks at 21:00: Y carries "Won't fit before 11:59 pm" and the
  meter's line names it. Record each group's first row and the lines in §9; optionally render with
  `?fixture=`. Delete the scratch vault afterwards. No account, no live vault, no OS input.

---

## 6. Fidelity ledger

| Spec | Requirement | Task | Proof |
|---|---|---|---|
| §2.1 | Runs only with a moment on the ranked day; otherwise `rank`'s order | T3a | P7 |
| §2.1 | The moment truncated to the minute | T3a | P6 |
| §2.1 | Must do permuted in place; every other index unchanged; membership unchanged | T3a, T3b | P18 |
| §2.1, M9 | The `--window` preview ranks at the page's moment on the previewed calendar | T4a | S13 |
| §2.2 | On time when `due > m`; due at `m` is overdue; bare date reads 23:59; one place for O6's close | T3a | P6 (due at `m` is overdue), S8 (a bare date orders as 23:59 and prints no time); the deadline function's doc comment, checked at T7 |
| §2.3, Q3 | Each date from its window's start to the next date's `00:00`, keyed by date | T2 | W1, W2, W7 |
| §2.3 | Filter first, then the cut at `m`; whole civil minutes | T2, T3a | W1, P3, P4 |
| §2.3 | Never shrinks as `d` moves later; independent of tasks | T3a | P5 (`Timeline::build` takes no task, so independence holds by its signature) |
| §2.3, r2 M2 | Busy time at and across midnight; overnight commitment dropped; inverted class counts no minute twice | T2 | W3, W4, W5, W6 |
| §2.3 | Existing `free_blocks` results unchanged | T2 | W8 |
| §2.4, Q2, Q4 | Step 0: pressure order kept when every dated item has 30 minutes spare | T3a | P8, P9, P1 |
| §2.5 | Deadline order, ties in scored order | T3a | P10 |
| §2.6, Q9 | Moore–Hodgson: the most work removed, then the later deadline, then the lower pressure; no margin | T3b | P11, P12, P13 |
| §2.6, r2 I1 | The record: the whole prefix at removal, up to and including the overflower | T3b | P11, P14 |
| §2.7, Q1, Q4 | Overdue first while every kept item keeps 30 minutes; a shorter one may go first | T3b | P15, P16, P17 |
| §2.8 | `W` last, in deadline order; never hidden | T3b | P21 |
| §2.9 | Examples (b), (c), (d), (e), (f) | T3a, T3b | P15, P16, P11, P8, P5 |
| §3, Q10 | Placeholders order as stored; "may not fit" when the item or its prefix has one | T3b, T4b | P14, S9, S10 |
| §4, Q8 | Overrides win and are not charged; the meter names the pin | T3b, T4b | P19, S12 |
| §4, M5 | The designation and `moved` read `rank`'s list | T3b, T4a | C1 (run path), S4 |
| §4, Q6 | Consecutive runs, rows in rank order; a passed due today reads `Overdue` | T4a | S1, S2, S3 |
| §4 | The meter's totals unchanged; `W` first in `spill_items` | T4b | S5 |
| §4, Q7 | `today.md` in the new order, no new line | T3b | C1 |
| §5 | Integer minutes; total tie-breaks; deterministic | T3a, T3b | P2, P20 |
| §6, Q5 | The 20:00 step deleted; the hour tests retired by name; plumbing kept | T0, T3b | T0's retire record; T7 |
| §7, Q7 | Row line, bare dates, weekday, explanation, meter line, words never used | T4b, T5 | S6, S7, S8, S11, S12; T5 screenshots |
| §8 | Goldens and references byte-identical, never regenerated, at four moments | T1o | O1, O2; `oracle.rs`, `surface_oracle.rs` unedited |
| §8, M8 | The oracle file is contract-list work | T1o, T7 | `contract-engineer` wrote it; `contract-reviewer`'s read |
| §9 | `rank` model-free; no note field, no journal record | all | T7 |
| (plan) | No contract-list file, no fixture, no existing test edited except the retire list | all | T7 |

## 7. Checkpoints

1. **Plan review** (`reviewer`, plan mode) before T0; findings answered in §9, verdict on the
   status line.
2. **After T0:** the controller reads the clock and retire records before T1o.
3. **After T1o:** `contract-reviewer` reads `deadline_order_oracle.rs` (green on the base) before any
   code changes.
4. **After each red commit** (T1a-T1e): the controller checks which tests fail and why, and that
   `oracle.rs`, `surface_oracle.rs`, `evening_rank.rs` and T1o's file are green, before the matching
   implementer task.
5. **After T3b:** the controller checks the deleted tests against T0's retire record, name by name.
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

