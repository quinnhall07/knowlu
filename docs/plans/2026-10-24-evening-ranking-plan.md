# The evening ranking rule: build plan

> **For agentic workers:** REQUIRED SUB-SKILL: use superpowers:subagent-driven-development (recommended)
> or superpowers:executing-plans to run this plan task by task. Steps use checkbox (`- [ ]`) syntax.

**Status: reviewed, PASS r2; ready for T0.** Round 2 (2026-10-24,
`docs/reports/2026-10-24-evening-ranking-plan-review-r2.md`): PASS, 4 minors, all closed in §9.2 and
the tasks they name; no further round. Round 1 (`docs/reports/2026-10-24-evening-ranking-plan-review-r1.md`):
FAIL, 5 important, 7 minor, answered in §9.1. Q-2 and Q-3 are the controller's rulings (§3).
**Q-1 stays open for Quinn** and gates only S4 and T4's last item.

**Date:** 2026-10-24. **Lane:** the evening ranking rule (outside wave B). **Base:** `cb3b661`
(its engine matches `1d3c467`).

**Spec:** `docs/specs/2026-10-23-evening-ranking-rule-draft.md`, **ruled by Quinn 2026-10-24** (the
file keeps its `-draft` name). The rulings it builds:
- **(a)** the hour is 20:00 in the vault's zone, one engine constant; a `planning.yaml` key later;
- **(b)** daily late-penalty steps are out of v1; only a late window's close counts;
- **(c)** no evening rewrite of `state/today.md`; the live page re-ranks on read;
- **(d)** "tonight" ends at local midnight.

**Goal.** From 20:00 on, a task due tonight at a time still ahead outranks an overdue task, ordered
by due time; before 20:00 the order is exactly today's. Concretely:
1. `ranking::rank_at` takes an optional moment and applies the evening step; `rank` is unchanged.
2. The `rank` command ranks at the vault's clock in production and gains a strict `--now`. With
   `--today` and no `--now` there is no moment; with `--now` and no `--today` the day is `--now`'s.
3. The console's read model ranks at the page's `now`, so the live page, the Must do groups and the
   meter's spill list follow the evening order.
4. Both golden files and the three `surface-today-*.json` references stay byte-identical, unedited
   and never regenerated.

**Architecture.**
- **Engine only.** `ranking.rs` gains `EVENING_HOUR` and `rank_at` (`rank` delegates with no
  moment). `cli.rs` gains `run_at`, `run_with_at`, `rank_clock` and a `BadNow` error (`run` and
  `run_with` delegate with no `--now`, so their 28 call sites, one in the contract-listed
  `sync_replay.rs`, are not touched). The engine's `main.rs` `Rank` gains `--now`. `surface.rs`
  gains `pub fn load_at` (`load` and `load_with` delegate with no moment), and `build_state_with`
  calls it with the page's `now`.
- **App: no change.** The scheduler runs `rank` without `--today` (`app/src/scheduler.rs`, the
  `rank` argv), so production ranks at the vault's clock. The console calls `build_state` with
  `now_in(cs)` (vault zone, `app/src/commands.rs`). A pinned app test date resolves to 12:00
  (`now_in`), before the hour, so app tests are unaffected.
- **Nothing on the contract list changes,** and no fixture is edited or added (§2).

**Code read for this plan:** the worktree `.claude/worktrees/docs-1023`. The planner had no shell;
the controller gives the base as `cb3b661`. There is no `engine/src/rank.rs`; ranking lives in
`engine/src/ranking.rs`. T0 re-confirms every cited symbol on the lane branch.

---

## 1. VISION and the two overriding rules

- **The test for every feature.** At 8 pm, "what's next?" is the thing that closes tonight. The rule
  makes the answer more honest at the hour it matters, and changes nothing earlier in the day.
- **Commitment 1, deterministic engine.** The moment is an input like the day: same notes, day and
  moment, same order. The step is a stable sort over the scored list on exact `DateTime` keys; ties
  keep the pressure order, and fully equal keys keep input order, as `rank` does today.
- **`rank` never calls a model** (decision 11). The rule is arithmetic on `due` and a clock.
- **Rule 1, no single-user assumptions.** One constant for every vault, in each vault's own zone
  (`cli::vault_zone`). The spec's §2 records that a hard-coded hour must be checked against more
  than one pilot student before it is treated as settled; the `planning.yaml` key is the follow-up.
- **Rule 2, frozen references.** `oracle.rs` runs `rank --today 2026-08-28` with no time, so under
  T3's moment rule it has no moment and ranks as daytime on any wall clock. Independently, no
  fixture task is due on 2026-08-28. T0 records both, E1 and S3 prove both, and `oracle.rs` and
  `surface_oracle.rs` run **unedited and green at every commit**, T1b's included (§9.1, finding 1).
  No reference is regenerated, ever.
- **Invariants kept.** No note field, no journal record, no run-record count and no new file. `--now`
  pins the ranking moment and, **beyond the day, exactly as `--today`**: with no `--today`, its date
  is the `today` that `info_pass`, `defer_over_budget`, `process_approvals` and
  `count_proposals_created` date their writes by (`cli.rs:410-440`). `rank --now` is a real run,
  never a dry run. It never sets a timestamp the engine writes (T3).
- **Privacy.** No data moves. Nothing new syncs or leaves the device.

## 2. Global constraints

- `cargo build --workspace` and `cargo test --workspace` from the root, dev profile, **0 warnings**
  (the one accepted `.rsrc` line). TDD: each red commit (T1a, T1b, T1c) is followed by the task
  that turns it green (T2, T3, T4). **The red state** is a test that compiles and fails at run time;
  stubs are signatures with `_`-prefixed parameters and `todo!("evening T<n>")`, declared `pub`
  where a non-test build would otherwise warn. **No stub may sit on a path an existing test runs**
  (`rank`, `run`, `run_with`, `load`, `load_with`, `build_state` and the `rank` command keep working).
- **Contract list untouched:** `engine/src/{write,journal,yamlemit,yaml,pystr,ledger,ids,provenance,
  approvals,sync,entitle,wincred,reconcile}.rs`, `app/src/{credentials,account,updates}.rs`, the
  oracle, sync and entitlement tests (`oracle.rs`, `surface_oracle.rs`, `sync_*.rs` among them),
  `engine/tests/fixtures/**`. A task that finds it needs one stops and reports.
- **No fixture edits, no new fixture files.** Tests that need a due-tonight task copy a fixture vault
  to the temp directory (as `oracle.rs`'s `copy_vault` does) and add the note there.
- **Existing tests pass unedited.** Every new entry point is a sibling, so no existing call changes.
  `#[ignore]`d tests stay ignored.
- **Tests never leave the machine.** `Fetchers::default()` and the fixtures' offline fallbacks cover
  `rank`.
- **Line endings:** LF in every file this lane edits; fixtures are read, never written.

## 3. Questions

- **Q-1 (open, Quinn): does `Due today` sort by due time after the hour?** The console's Must do
  block groups rows (`surface::group_label`) and sorts each group by start-by, then due
  (`surface::sort_rows`). After the hour the groups follow the evening order, so `Due today` leads
  `Overdue`, but inside `Due today` a 5-hour task due 23:59 (start-by days earlier) sorts above a
  1-hour task due 21:30. `today.md` lists Must do in rank order and has no such split. **Recommend:
  from the hour on, sort the `Due today` group by due time, then the existing key; before the hour,
  nothing changes.** If declined, T1c drops S4, T4 skips its last item, and T5 records in the spec
  that the console keeps its in-group sort.
- **Q-2 (ruled by the controller, 2026-10-24, as recommended):** `test-writer` writes
  `engine/tests/evening_rank.rs`; `contract-reviewer` reviews that one file, because it asserts
  against the golden files and the surface references.
- **Q-3 (ruled by the controller, 2026-10-24, as recommended): the zone split stays.** Without
  `--today`, `cli::run_with` takes the day in the machine's zone (`TimeZone::system()`) but
  `local_now` in the vault's. Where they differ, the step's guard (moment's date equals the day)
  keeps the rule off rather than misfiring. This lane leaves `today`'s zone alone; a follow-up takes
  the day from `vault_zone`, as `surface` already does.
- **Decided by the controller, 2026-10-24:** `--now` without `--today` takes the day from `--now`
  (C5); the meter's spill swap after the hour is intended (spec §1, S5).
- **Decided by the planner:** `rank` gains `--now` rather than a test-only seam. It mirrors
  `surface --now` (console spec §4.6), lets E1 drive the real binary at an evening moment, and
  changes no timestamp the engine writes.

## 4. Conflict map

| File | This lane's change | Task | Shared with |
|---|---|---|---|
| `engine/src/ranking.rs` | `EVENING_HOUR`, `rank_at`, the closing-time helper; R1-R15 | T1a, T2 | O6 later (adds the late window's close in the helper) |
| `engine/src/cli.rs` | `run_at`, `run_with_at`, `rank_clock`, `RunError::BadNow`; C1-C5 | T1b, T3 | **L5's `cli.rs` edit** (not built yet): whichever lands second rebases. O1 merged (#64) |
| `engine/src/main.rs` (the engine's) | `Rank { now }` and its arm | T1b, T3 | **wave B's F7a** (a new `Command` variant and arm: a separate hunk, rebase only). L6 owns `app/src/main.rs`, not this file |
| `engine/src/surface.rs` | `pub fn load_at`; `build_state_with` passes `now`; S4 per Q-1; S1-S5 | T1c, T4 | L7 merged. L5 is not built: T4 runs when ready, and L5 rebases over about ten lines if it starts later (controller, 2026-10-24) |
| `engine/tests/evening_rank.rs` (new) | E1, E3 (T1b); E2 (T1c) | T1b, T1c | none |
| docs (`docs/reference/engine-commands.md`, `docs/surface/anatomy.md`, `HANDOFF.md`, the spec) | §T5 | T5 | docs-keeper's queue |

## 5. Tasks at a glance

| Task | What | Agent | Why that agent | Depends on |
|---|---|---|---|---|
| T0 | Branch from `cb3b661`, spec commit, citation check, the clock record | main session | git, and judgment about drift | none |
| T1a | Red: `ranking.rs` R1-R15 and stubs | `test-writer` | the failing tests, specified below | T0, plan review |
| T2 | `ranking.rs`: `EVENING_HOUR`, `rank_at` | `implementer` | specified, test-checked, off the contract list; it reorders every page, so `high` | T1a |
| T1b | Red: `cli.rs` C1-C5, the `--now` flag, E1 and E3 | `test-writer` | as T1a | T2 |
| T3 | `cli.rs` and the engine's `main.rs`: the moment, the day, `--now` | `implementer` | as T2; it touches the run path that writes `today.md` | T1b |
| T1c | Red: `surface.rs` S1-S5 (S4 per Q-1) and E2 | `test-writer` | as T1a | T2 |
| T4 | `surface.rs`: `load_at`, the page's `now`, S4 per Q-1 | `implementer` | the read model; references must stay byte-identical | T1c |
| T5 | Docs | `docs-keeper` | references, HANDOFF, the spec | T3, T4 |
| T6 | Review | `reviewer`; `contract-reviewer` for `evening_rank.rs` (Q-2) | everything a cheaper agent changed is reviewed before a push | T2-T5 |
| T7 | Scratch demo at 19:59 and 20:00 | main session | a scratch vault; no account, no live vault | T6 |

After T2, the T1b-T3 pair and the T1c-T4 pair edit disjoint source files and may run in parallel;
both append to `evening_rank.rs`, so the second to commit rebases over the first's tests.

## T0. Branch, spec and the clock record (main session)

- [ ] Branch `feat/evening-ranking` from `cb3b661` (or `origin/main` if it has moved; record the SHA
  in §9).
- [ ] Commit the ruled spec, unedited, if `main` does not already carry it.
- [ ] Re-confirm every symbol this plan cites: `ranking::{rank, partition_must_do,
  designate_today_explained}`, `cli::{run, run_with, local_now, vault_zone, RunError}`, the engine
  `main.rs`'s `Command::Rank` and `Command::Surface`, `surface::{load, load_with,
  build_state_with, meter, must_do, group_label, sort_rows, note_detail}`,
  `models::coerce_datetime`. Record drift in §9.
- [ ] **The clock record.** Confirm and write into §9: `engine/tests/oracle.rs`'s `render` runs
  `rank --vault <copy> --today 2026-08-28 --runner manual`, with no time of day, so today `now` is
  the wall clock (`cli::local_now`); `strip_footer` drops only the `*Generated ` line.
  `engine/tests/surface_oracle.rs` pins `--today 2026-08-28 --now 2026-08-28T09:00`. No task note
  under `engine/tests/fixtures/{vault-s1,vault-s1-migrated,vault-full}/tasks/` has a `due` on
  2026-08-28 (the planner's read: 08-26, 08-27, 08-31, 09-01, 09-25, and a `done` note on 08-20).
- [ ] Note in §9 whether wave B's F7a or L5 has touched the engine's `main.rs` or `cli.rs` yet.
- [ ] `cargo test --workspace` is green with 0 warnings on the base, so each red is its own.

## T1a. Red: `ranking.rs` (test-writer)

**Brief:** this section, §1, §2 (the red state) and the spec's §1-§3. Write R1-R15 and the stubs.
Run: each new test compiles and fails at run time for its stated reason, except R14 (**green
now**); every other test passes unedited; 0 warnings. Commit. Names are suggestions; behaviour is
not.

**Stubs:** `pub const EVENING_HOUR: jiff::civil::Time = Time::constant(20, 0, 0, 0)` (the real
value) and `pub fn rank_at(tasks, today, cal, _moment: Option<DateTime>) -> Vec<Task>` as
`todo!("evening T2")`. `rank` is not touched, so `oracle.rs` and every existing caller stay green.

**Helper (in `ranking.rs`'s `mod tests`).** Tasks are built in memory with `Task { .. }` (all fields
are `pub`) on the vault-s1 week template, read-only, as the existing `fixture()` does. The day is a
`D` constant (Friday 2026-08-28, so the template has capacity). Effort and importance are chosen so
the overdue task's pressure is strictly higher than the due-tonight task's under `rank`; each test
first asserts that, so a green result cannot come from a weak setup.

- **R1** `rank_at_with_no_moment_is_rank`: equal outputs for the vault-s1 fixture and the in-memory
  set.
- **R2** `before_the_hour_nothing_changes`: moment `D 19:59`, a task due `D 23:59` and one due
  `D-2 23:59`: `rank_at == rank`.
- **R3** `at_the_hour_a_task_due_tonight_leads_an_overdue_one`: moment exactly `D 20:00`; the
  due-tonight task is first, the overdue one second.
- **R4** `closing_tonight_orders_by_due_time_then_pressure`: at moment `D 20:30`, a task due 21:00
  and two due 23:59 whose pressures differ (different effort, so different slack): the 21:00 task is
  first, then the two 23:59 tasks with the higher-pressure one first, as `rank` orders them.
- **R5** `a_due_time_already_passed_is_not_closing_tonight`: due `D 19:00` at moment `D 21:00`, and
  a due exactly equal to the moment: neither moves; both keep `rank`'s position.
- **R6** `a_past_due_task_with_no_window_is_not_closing_tonight`: v1 has no late window (O6), so an
  overdue task never joins the first group; it ranks below tonight's items after the hour.
- **R7** `tonight_ends_at_local_midnight`: at `D 23:00` a task due `D+1 00:30` does not move; at
  moment `D+1 00:30` with `today = D+1` the order is `rank`'s.
- **R8** `a_moment_on_another_day_changes_nothing`: `today = D`, moment `D+1 21:00` (and `D-1
  21:00`): `rank_at == rank`.
- **R9** `a_rank_override_still_wins_after_the_hour`: an overdue task with `rank_override: 1` stays
  first at 21:00; the due-tonight task is second.
- **R10** `the_evening_order_keeps_must_do_and_the_takes`: at 21:00, `partition_must_do` over
  `rank_at` and over `rank` give the same set (compared sorted by slug), and
  `designate_today_explained` gives equal takes.
- **R11** `the_evening_order_is_deterministic`: two calls are equal, and reversing the input order of
  tasks whose keys all differ gives the same output.
- **R12** `undated_tasks_never_close_tonight`: a task with `due: None` keeps `rank`'s position.
- **R13** `a_bare_date_due_today_closes_at_2359`: a task built by `Task::from_meta` from `due:
  2026-08-28` leads an overdue one at 21:00 (it reads as 23:59, `coerce_datetime`).
- **R14** `evening_hour_is_twenty_hundred` (**green now**): `EVENING_HOUR == 20:00`. Its message says
  the value is Quinn's ruling (a), 2026-10-24, and changing it needs a new one.
- **R15** `everything_else_keeps_the_pressure_order`: at 21:00, the tasks not closing tonight appear
  in `rank_at`'s output in exactly `rank`'s relative order.

Size: S.

## T2. `ranking.rs`: the evening step (implementer)

**Brief:** spec §1-§3, this plan's §1, T1a's tests.
- [ ] `rank_at(tasks, today, cal, moment)`: compute the scored list exactly as `rank` does; then, if
  `moment` is `Some(m)` with `m.date() == today` and `m.time() >= EVENING_HOUR`, stable-partition the
  non-override list into closing-tonight tasks (sorted, stably, by closing time) followed by the
  rest in their scored order; then insert overrides exactly as `rank` does.
- [ ] One private helper decides a task's closing time: `Some(due)` when `due.date() == m.date()` and
  `due > m`, otherwise `None`. **It is the one place O6's late-window close goes**; its doc comment
  says so and that penalty steps are out (ruling b).
- [ ] `rank` becomes `rank_at(tasks, today, cal, None)`. The decorate-sort-undecorate and its
  preserved quirk are untouched; the doc comment gains one paragraph on the moment.
- [ ] No `f64` key enters the evening sort: closing times are `DateTime`s, compared exactly.

Acceptance: R1-R15 green; `ranking.rs`'s existing tests and `oracle.rs` green unedited. Size: S.

## T1b. Red: `cli.rs`, the flag, E1 and E3 (test-writer)

**Brief:** this section, §1, §2, §3 and the spec's §1 ("Its input is the moment"). Same run and
commit rules as T1a.

**Stubs:**
- `cli.rs`: `pub fn rank_clock(today_iso: Option<&str>, now_iso: Option<&str>, system_today: Date,
  wall: DateTime) -> Result<(Date, Option<DateTime>), RunError>`, `pub fn run_at(vault, today_iso,
  now_iso, runner, run_id)` and `pub fn run_with_at(.., now_iso, .., fetchers)`, each
  `todo!("evening T3")`. `RunError::BadNow(String)` with its arms filled in (`exit_code` 2,
  `variant_name` `"BadNow"`, the message `--now must look like YYYY-MM-DDTHH:MM, got {v:?}`).
- The engine's `main.rs`: `Command::Rank` gains `#[arg(long)] now: Option<String>` (doc: "Pin the
  ranking moment, YYYY-MM-DDTHH:MM in vault time. Without it, a run with --today has no moment and
  ranks as daytime"). **The arm keeps calling `cli::run` and ignores `now`** (`now: _`), so
  `oracle.rs` and every `rank` test stay green through T1b (§9.1, finding 1).

**Scratch vaults** are temp-dir copies of `vault-s1`; a test that needs a due-tonight task writes
`tasks/evening-probe.md` (`due: 2026-08-28T23:59`, an `id:`, effort 1.0) into the **copy**.
- **C1** `rank_clock_resolves_the_day_and_the_moment`: `--today 2026-08-28` alone gives
  `(2026-08-28, None)`; neither flag gives `(system_today, Some(wall))`; `--today` and `--now` give
  `(today, Some(now))`. Strictness: `2026-08-28T21:00` parses, and so does `2026-8-28T9:00` (single
  digits are accepted, as `2026-08-28T09:00`); `2026-08-28` (a bare date),
  `21:00`, `2026-08-28T21:00:00` (seconds), `2026-08-28 21:00` and `tonight` each give `BadNow`
  with `exit_code() == 2`. A bad `--today` still gives `BadDate`.
- **C2** `a_bad_now_writes_nothing`: `run_at` with a bad `--now` returns `BadNow`, and the copy has
  no `state/runs/` entry, no journal file and no `state/today.md` it did not have before.
- **C3** `an_evening_run_writes_the_evening_order_to_today_md`: `run_with_at(.., Some("2026-08-28"),
  Some("2026-08-28T21:00"), "manual", None, Fetchers::default())`: the first Must do line in
  `state/today.md` names the probe. At `T19:59` the first line is `cs-100-hw-01`'s, as the golden.
- **C4** `now_pins_only_the_ranking_moment`: after C3's run at `T21:00`, neither the `*Generated `
  footer nor the run record's `ts` carries the pinned date 2026-08-28 or the pinned stamp (21:00 on
  that date). It does not compare against the wall clock's own date: UTC and local time can
  disagree near midnight and flake (§9.1, finding 7).
- **C5** `now_without_today_ranks_the_day_of_now` (controller's call): `rank_clock(None,
  Some("2026-08-28T21:00"), <any other date>, ..)` gives `(2026-08-28, Some(21:00))`, and
  `run_with_at` with only that `--now` renders `today.md` for Friday, August 28 with the probe first.

**`engine/tests/evening_rank.rs`, new** (Q-2: `contract-reviewer` reviews it). It copies fixtures to
the temp directory exactly as `oracle.rs` does, **never writes under `tests/fixtures/`**, and
compares with `oracle.rs`'s footer strip, copied, not imported.
- **E1** `rank_at_an_evening_moment_still_renders_both_goldens` (**green now**, since the stub ignores
  `--now`, and it must stay green after T3): `rank --today 2026-08-28 --now 2026-08-28T21:00`, and
  again at `T23:58`, on copies of `vault-s1` and `vault-s1-migrated` match `golden-today-s1.md`, and
  on `vault-full` match `golden-today-full.md`.
- **E3** `rank_refuses_a_bad_now_with_exit_2`: on a scratch copy (with the stub, `rank` runs and
  writes, so never on a fixture), `rank --today 2026-08-28 --now tonight` exits 2 and names `--now`.

Acceptance: C1-C5 and E3 fail at run time; E1 passes; everything else green, 0 warnings. Size: S.

## T3. `cli.rs` and the engine's `main.rs`: the moment (implementer)

**Brief:** spec §1 ("Its input is the moment"), §3 here, T1b's tests.
- [ ] `rank_clock`: `--now` is parsed **strictly** with `DateTime::strptime("%Y-%m-%dT%H:%M", ..)`,
  the call `models::coerce_datetime`'s `parse_exact` makes, never jiff's `DateTime` `FromStr`
  (which reads a bare date as midnight). **Seconds are not accepted.** Single-digit fields are
  accepted (jiff's `%m`, `%d` and `%H` take one digit), on purpose: they are harmless, and C1 pins
  it (controller's call, r2). The day is `--today` when
  given, else `--now`'s date, else `system_today`; the moment is `--now`, else `None` when `--today`
  is given, else `wall`.
- [ ] `run_with_at` is `run_with`'s body with `rank_clock` in place of the `today` match, called
  **before `start_run`**, so a bad value writes nothing (as `BadDate` does today). `system_today`
  is computed exactly as now (`TimeZone::system()`, Q-3). `wall` is the existing `now` binding
  (`let now = local_now(vault)`, at the top of `run_with`, `cli.rs:370`), never a second
  `local_now` call, so the moment and the footer read one clock. The `rank`
  call becomes `rank_at(&tasks, today, &cal, moment)`. The wall clock still feeds the footer, the
  run record and `crash`. `run_with` becomes `run_with_at(.., None, ..)`.
- [ ] `run_at` builds the fetchers as `run` does and calls `run_with_at`; `run` calls `run_at` with
  `None`. No other line of the run path moves.
- [ ] The engine's `main.rs`: the `Rank` arm calls `cli::run_at(&vault, today, now, ..)`.
- [ ] `surface --now` keeps its own parse; aligning it is out of scope (§8).

Acceptance: C1-C5, E1 and E3 green; every `cli.rs` test and `oracle.rs` green unedited. Size: S.

## T1c. Red: `surface.rs` and E2, in T4's turn (test-writer)

**Brief:** this section, §1, §2, §3 Q-1 and the spec's §1 ("Where students see it"). Same rules.

**Stub:** `pub fn load_at(vault, today, preview: Option<&Window>, _moment: Option<DateTime>) ->
Loaded` that calls `load_with` and ignores the moment, so the reorder tests fail on their
assertions, not on a panic. `build_state_with` is not touched. Scratch copies and the probe as T1b.
- **S1** `load_at_an_evening_moment_reorders_and_load_does_not`: `load_at(.., Some(D 21:00))` ranks
  the probe first; `load(vault, D)` does not.
- **S2** `build_state_after_the_hour_leads_must_do_with_due_today`: `build_state` at `now = D 21:00`
  (vault zone) has `must_do.groups[0].label == "Due today"`; at `D 09:00`, `"Overdue"`.
- **S3** `the_fixture_vaults_rank_the_same_at_nine_and_nine_pm` (**green now, and it must stay green
  after T4**): for each of the three fixture vaults, read in place (`load_at` writes nothing), the
  ranked slugs at `D 21:00` equal those of `load(vault, D)`.
- **S4** (only if Q-1 is yes) `after_the_hour_due_today_rows_sort_by_due_time`: two probes due
  today, 5.0h due 23:59 and 1.0h due 21:30; at `now = D 20:30` the `Due today` rows are the 21:30
  task, then the 23:59 task; at `D 09:00` they keep today's start-by order.
- **S5** `the_meter_spills_the_overdue_lab_after_the_hour` (controller's call): a scratch copy whose
  `tasks/` holds only a 3.0h lab due `D-2 23:59` and a 1.0h task due `D 23:59`, and whose week
  template gives `D` exactly 3.0h of capacity. `build_state` at `D 09:00`: the meter's
  `spill_items` name only the task; at `D 21:00`, only the lab. `spill_hours` (1.0) and
  `spill_blocks` are equal in both.
- **E2** `surface_at_an_evening_moment_keeps_the_references_ranked_blocks` (**green now**): `surface
  --view today --today 2026-08-28 --now 2026-08-28T21:00 --build-sha pinned` on each fixture, in
  place (`surface` never writes): its `must_do`, `recommended` and `meter`, and its `nav_counts`
  with the `runs_warn` field removed, equal those parsed from the matching `surface-today-*.json`.
  Only those: `now`, `revision`, the runs panel, "closed this week" and `nav_counts.runs_warn`
  (built from `runs::expected_status`) read the clock, so they differ at 21:00 for clock reasons,
  not ranking ones.

Acceptance: S1, S2, S4 and S5 fail at run time on their order assertions; S3 and E2 pass; everything
else green, 0 warnings. Size: S.

## T4. `surface.rs`: the page's moment (implementer)

**Brief:** spec §1 ("Where students see it"), §3 Q-1 here, T1c's tests.
- [ ] `load_at` holds `load_with`'s body with `rank_at(.., moment)`; `load_with` calls it with
  `None`; `load` is unchanged.
- [ ] `build_state_with` calls `load_at(.., Some(now.datetime()))`: the same civil `now` the page
  displays, which both production callers give in the vault's zone. `meter` and `must_do` then
  follow the evening order with no change of their own.
- [ ] `note_detail` keeps `load`: a row's fields do not depend on rank position (`surface::row`), so
  the drawer cannot disagree with the list.
- [ ] **Only if Q-1 is yes:** `Loaded` carries the moment, and `must_do` sorts the `Due today`
  group by due time, then the existing key, when the step is active; every other group, and every
  group before the hour, keeps `sort_rows`.

Acceptance: S1-S5 and E2 green; `surface_oracle.rs` and `build_state_never_writes` green, unedited.
Size: S.

## T5. Docs (docs-keeper)

Each edit by its text, never by line number.
- [ ] `docs/reference/engine-commands.md`, the `rank` entry: `[--now YYYY-MM-DDTHH:MM]` (strict, no
  seconds); one sentence each on the evening order (from 20:00 vault time, due-tonight tasks first
  by due time, before the hour unchanged), on the day and moment rule (§3, T3), and that `--now`
  pins no written timestamp. It says that beyond the day, exactly as `--today`, a `--now` with no
  `--today` dates the run's writes (the info pass, the approvals budget and pass, the proposal
  count) by `--now`'s day, so `rank --now` is a real run, never a dry run. The `surface` entry: its `--now` is also the ranking moment.
- [ ] `docs/surface/anatomy.md`: the ranked list, Must do's group order and the meter's spill list
  follow the evening order after the hour; Q-1's answer for the `Due today` group.
- [ ] `HANDOFF.md`: the lane's row (spec, plan, branch, state), and the follow-ups: Q-3's zone split,
  the `planning.yaml` key, aligning `surface --now`'s parse, and O6's hook (`ranking.rs`'s
  closing-time helper).
- [ ] The spec: if Q-1 is declined, §1's Must do bullet records that the console keeps its in-group
  sort.

Acceptance: `scripts/ci/eol-check.ps1` clean; no source file in the diff. Size: XS.

## T6. Review (reviewer; contract-reviewer per Q-2)

Pre-push review of the lane against this plan and the spec. Report:
`docs/reports/2026-10-2X-evening-ranking-review.md` (the day it runs). It checks, besides the code:
no contract-list file and no fixture in the diff; `oracle.rs` and `surface_oracle.rs` unedited and
green at every commit of the lane; the goldens and the three surface references byte-identical
(`git diff --stat` shows none of them); every existing test unedited; 0 warnings; the CI line; T0's
clock record in §9. `contract-reviewer` reads `engine/tests/evening_rank.rs` against `oracle.rs`
and `surface_oracle.rs`: same copy-to-temp, same footer strip, no write under `tests/fixtures/`.

## T7. Scratch demo (main session)

On a scratch vault (`scripts\scratch-vault.ps1 -Source engine\tests\fixtures\vault-s1-migrated`),
with one task added due today at 23:59, run `surface --view today --now <today>T19:59`, then
`T20:00`. **Expected:** at 19:59 Must do's first group is `Overdue`; at 20:00 it is `Due today`,
led by the added task. Record both, with each group's first row, in §9. Optionally render both
states with the console fixture page (`?fixture=`, headless Edge) for Quinn. Delete the scratch
vault afterwards. No account, no live vault, no OS input.

---

## 6. Fidelity ledger

| Spec | Requirement | Task | Proof |
|---|---|---|---|
| §1, ruling a | The step runs at or after 20:00 vault time; `EVENING_HOUR` is one constant | T2 | R3 (exactly 20:00), R2 (19:59), R14 |
| §1 | Before the hour the order is exactly today's | T2 | R1, R2, R8 |
| §1 | Closing tonight: due on the moment's date, strictly after the moment; bare dates read as 23:59 | T2 | R3, R5, R13 |
| §1 | Closing-tonight first, earliest first; ties keep the pressure order | T2 | R4 |
| §1 | Everything else keeps the pressure order below them | T2 | R15 |
| §1 | Must do membership and the designation unchanged | T2 | R10 |
| §1 | `rank_override` keeps winning | T2 | R9 |
| §1, ruling d | "Tonight" ends at local midnight | T2 | R7 |
| §3 | A past-due task with no known window is not closing tonight; v1 has no window | T2 | R6 |
| §3, ruling b | Penalty steps not counted; one place for O6's close | T2 | the helper's doc comment; T6 |
| §1 | The moment: `--now`, else none with `--today`, else the vault's clock | T3 | C1, E1 |
| §1 (controller) | `--now` without `--today` ranks `--now`'s day | T3 | C1, C5 |
| §1 | `--now` is strict `YYYY-MM-DDTHH:MM` (single digits accepted); a bare date, seconds or a space are refused, exit 2 | T3 | C1, C2, E3 |
| §1 | Beyond the day, `--now` alone works exactly as `--today`: a real run, never a dry run | T3, T5 | C5; T5's `engine-commands.md` line |
| §1 | Deterministic for the same notes, day and moment | T2, T3 | R11 |
| §1, §4 | Nothing new written; `--now` pins no written timestamp | T3 | C2, C4 |
| §1, ruling c | Live page re-ranks at its `now`; no evening slot | T4 | S1, S2; the app is unchanged |
| §1, ruling c | A slot that runs after the hour writes the evening order | T3 | C3 |
| §1 | Must do groups follow the evening order on the console | T4 | S2; T7 |
| §1 (controller) | The meter's spill list names the overdue task after the hour; spill hours and blocks unchanged | T4 | S5 |
| §1, Q-1 | `Due today` by due time after the hour (if ruled) | T4 | S4 |
| §4 | Golden files byte-identical, never regenerated, on any wall clock | T1a-T3 | `oracle.rs` unedited and green at every commit; E1; T0's record |
| §4 | Surface references byte-identical; no §4.6 regeneration | T4 | `surface_oracle.rs` unedited; S3; E2 |
| §4 | `rank` model-free; no vault field | all | T6 |
| (plan) | No contract-list file, no fixture, no existing test edited, no app file | all | T6 |

## 7. Checkpoints

1. **Plan review, round 2** (`reviewer`, plan mode) before T1a; findings answered in §9, verdict on
   the status line.
2. **Q-1** answered by Quinn before T1c (it decides S4 and T4's last item).
3. **After each red commit** (T1a, T1b, T1c): the controller checks which tests fail and why, and
   that `oracle.rs` and `surface_oracle.rs` are green, before the matching implementer task.
4. **T6** before the push; the lane PR merges on green CI and the reviewer's PASS.

## 8. Out of scope

- The per-vault `planning.yaml` key (ruling a's follow-up), after the pilot shows whether students
  differ.
- O6's late window, and therefore the late half of the rule (spec §3); O6's plan adds the close time
  in T2's helper, with its own test.
- Penalty steps (ruling b). An evening slot or any `today.md` rewrite (ruling c).
- Q-3's zone split in `rank`'s day. Aligning `surface --now`'s looser parse with `rank`'s.
- Any app or `console.js` change: the page already renders the order, the groups and the meter it
  is given.
- `CLAUDE.md`: "same input, same order" already holds with the moment as an input; no rule line
  changes.

## 9. Fidelity notes

*(T0's base, citation and clock records, the review rounds and the controller's answers go here,
dated, newest first.)*

- **2026-10-24, T7 scratch demo (main session).** The engine was built at `209e768` (code as at
  `123ee02`). It ran on a scratch copy of `vault-s1-migrated` (`scripts\scratch-vault.ps1`), with one
  added task, "Demo essay due tonight" (1.0h, due `2026-08-28T23:59`). The command was
  `surface --view today --today 2026-08-28 --now <T> --build-sha pinned`. At `T19:59`, Must do's
  groups are `Overdue` (2 rows, first "CS 100 HW 01 Introduction to C"), then `Due today` (1 row,
  first the demo essay). At `T20:00`, they are `Due today` (1 row, the demo essay), then `Overdue`
  (2 rows, the same first row). Both exits were 0, matching the expectation. The scratch vault was
  deleted afterwards. No account, no live vault, no OS input.
- **2026-10-24, T0 (main session).** Branch `feat/evening-ranking` from `docs/evening-ranking`
  (`2023817`, on `cb3b661`; engine unchanged since `1d3c467`, whose CI was green). The clock record
  holds: `oracle.rs:86` runs `--today 2026-08-28 --runner manual` with no time; `surface_oracle.rs:22`
  and `:44` pin `--now 2026-08-28T09:00`. Every task `due:` across the three fixture vaults is
  2026-08-20, 08-26T23:59, 08-27T10:59, 08-31T23:59, 09-01 or 09-25T09:00, none on 08-28 or 08-29.
  Symbols: both reviews checked them against `cb3b661` with no drift. Wave B's F7a (`onboarding`
  arm in the engine's `main.rs`) is on `feat/o3-f7a` and not on `main`; L5 is unbuilt. Neither
  touches `Command::Rank`'s arm or `cli.rs`'s rank path.
- **2026-10-24, controller's rulings.** Q-2: `test-writer` writes `evening_rank.rs`,
  `contract-reviewer` reviews it. Q-3: the zone split stays; a follow-up. `--now` without `--today`
  takes `--now`'s day. The meter's spill swap is intended. `surface.rs`'s turn: L7 has merged and L5
  is not built, so T4 runs when ready. Base `cb3b661` (its engine matches `1d3c467`). Q-1 stays
  with Quinn.
- **2026-10-24, the planner's read (T0 confirms).** Harness clock: `oracle.rs` pins the day only
  (`--today 2026-08-28`), so its `now` is the wall clock; `surface_oracle.rs` pins
  `--now 2026-08-28T09:00`. No fixture task is due on 2026-08-28. Every due-today task has slack ≤ −1
  (the one-day buffer, `scheduling::DEFAULT_BUFFER_DAYS`), so it is already Must do, and the step
  never changes Recommended's order. `render::runway_days` is a minimum over slacks, so the topline
  and the run record's gauge counts do not depend on order. `surface::meter` fills capacity in rank
  order, so its spill list does (S5).

### 9.2 Plan review, round 2 (PASS, 4 minors; closed 2026-10-24)

Report: `docs/reports/2026-10-24-evening-ranking-plan-review-r2.md`.
1. E2 compares `nav_counts` without `runs_warn` (clock-driven, from `runs::expected_status`).
2. §1, the spec's §1 and T5 say `--now` alone sets the day exactly as `--today`, so the run's dated
   writes follow it: a real run, never a dry run.
3. T3's `wall` is the existing `now` binding (`cli.rs:370`), with no second `local_now` call.
4. Single-digit fields are accepted (controller's call); T3 says so and C1 pins `2026-8-28T9:00`.

### 9.1 Plan review, round 1 (FAIL; answered 2026-10-24)

Report: `docs/reports/2026-10-24-evening-ranking-plan-review-r1.md`.
1. **`Rank` stub.** T1b's arm keeps calling `cli::run` and ignores `--now`; §2 bars any stub on a
   path an existing test runs. E1 is green from T1b on.
2. **Strict `--now`.** T3 parses with `strptime("%Y-%m-%dT%H:%M")`; seconds, a bare date and a space
   are refused (C1).
3. **`surface.rs` turn.** The stub and S tests moved to T1c, T4's own red commit; the wait is gone
   (§4, controller's call).
4. **T1 split** into T1a, T1b and T1c (§5).
5. **Meter.** Spec §1 records the swap; S5 pins it; a ledger row.
6. `load_at` is `pub` (T1c). 7. C4 asserts the pinned date and stamp are absent. 8. R4 uses two
   23:59 tasks of different pressure, with no quirk claim. 9. §4 names F7a, L5's `cli.rs` edit and
   O1's merge. 10. C5 and `rank_clock`. 11. T7 states the expected groups. 12. §3, §9 and the
   status line record the rulings and the base.
