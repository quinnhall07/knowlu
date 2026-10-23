# Turned-in work is not a task (lane O1): build plan

> **For agentic workers:** REQUIRED SUB-SKILL: use superpowers:subagent-driven-development (recommended)
> or superpowers:executing-plans to run this plan task by task. Steps use checkbox (`- [ ]`) syntax.

**Status: REVIEWED. Round 2 (2026-10-23, `docs/reports/2026-10-23-o1-plan-review-r2.md`): PASS,
0 critical, 0 important, 4 minors, all folded in, with the controller's score-capture addendum
(§9.2).** Round 1 (plan-mode `reviewer`, 2026-10-23,
`docs/reports/2026-10-23-o1-plan-review-r1.md`): **FAIL**, 1 critical, 6 important, 5 minor; no
finding reopened a ruling. Every finding is answered in §9.1 and in the tasks it names. §3's
questions are all answered (Q-A by Quinn, the rest by the controller, 2026-10-23). Ready to run
from T0.

**Date:** 2026-10-23. **Lane:** O1, wave A of the onboarding redesign.

**Spec:** `docs/specs/2026-10-23-onboarding-redesign-draft.md` (ruled by Quinn, 2026-10-23; the
filename keeps `-draft`). This lane carries its §6 ("Turned-in work is not a task") and the slot
change §6.2 records. The rulings it builds:
- **Q3:** turned-in work is marked done automatically, with Undo; on a new vault it is archived at
  first sight.
- **Q11:** grades run before ingest. The slot becomes **sync → coursework → grades → ingest → judge →
  rank**, and the `CLAUDE.md` slot line changes in this lane.

**Goal.** A student's first look at Today holds no work they already turned in, and nothing they
must close by hand that the source already says is finished. Concretely:
1. zyBooks and VHL items at exactly 100% that `coursework` meets in a source group new to the vault
   go straight to `archive/` with `archived_reason: imported-done`, never created live.
2. Blackboard feed items whose gradebook column says submitted, graded or exempt go straight to
   `archive/` the same way on the vault's first ingest, when grade notes exist by then.
3. After first sight, a live task the source now says is finished gets `status: done`, written by
   `agent:knowlu.completion` through `write`, and shows **Undo** on "Closed this week".
4. A `status` the student set by hand is never written and never proposed, and a task with a "Mark
   done" card the student rejected, or has not yet answered, is never written either (§9.1 F1).

**Architecture.**
- **Engine.** `completion.rs` gains the one write path for an authoritative "done" (`apply_done`),
  and tier 1 calls it instead of filing amend cards. `coursework.rs` archives finished items at first
  sight. `ingest.rs` reads `grades/` (it now runs after grades) for the Blackboard half: archive at
  first sight, mark done afterwards. `surface.rs`'s "Closed this week" row gains an `undo` flag.
  `grades.rs` keeps writing only `grades/`; only its doc comment about the slot order changes.
- **App.** `scheduler::slot_argv` pushes `grades` before `ingest`. `console.js` renders Undo on a
  flagged row through the existing `set_fields` command. No new Tauri command, so `app/src/main.rs`
  and `commands.rs` (lane L6's until it merges) are not touched.
- **Nothing on the contract list changes.** `write`, `journal`, `approvals`, `sync`, `yamlemit` and
  `engine/tests/fixtures/**` are used, never edited (§2).

**Code read for this plan:** the worktree `.claude/worktrees/effort-anchors` (branch
`fix/effort-anchors`, `main` plus that fix). The planner had no shell. Every `file:line` here is from
that tree; T0 re-confirms each one on the lane branch, and an implementer re-reads before editing.

---

## 1. VISION and the two overriding rules

- **Commitment 5, who started a change.** The gradebook and the homework platform are authoritative
  on their own items, so their "finished" is applied and listed, not proposed (spec §6.2; L5's Q45
  already treats `coursework` items as authoritative). Undo is the student's control.
- **Judge once.** A `status` the journal shows the student set (`Journal::human_set`, `op: set`) is
  never overwritten, and after Undo the student's own `set` keeps the task active forever. A "Mark
  done" card the student rejected keeps the question closed, as it does today. **`apply_done`'s own
  checks are the only fence:** `status` is not in `provenance::JUDGED_FIELDS_TASK`
  (`provenance.rs:52-58`), and `write`'s judge-once guard runs only with `opts.judged`
  (`write.rs:322`), which `apply_done` leaves `false` (T3; §9.1 F5).
- **`rank` never calls a model.** Nothing here involves a model. The done rule is arithmetic on the
  source's own fields (spec §6.2's table).
- **Deterministic.** Same input, same order: figures in uid order (as `propose_vendor_completions`
  does today, `completion.rs:323-328`), feed events in feed order (ingest's mark-done pass too,
  T5), grade notes in path order.
- **Rule 1, no single-user assumptions.** Nothing names a school, a course or a student. The
  Blackboard UID prefix is Blackboard's product string, already in `ingest.rs`'s tests.
- **Rule 2, frozen references.** No fixture is edited or added. The eight Python references are
  untouched: no command they cover changes its output for their inputs (the fixture vaults have no
  `grades/` and no completion figures). The three `surface-today-*.json` references each hold an
  empty `closed_this_week` list, so the new `undo` field changes none of their bytes; T2a pins that
  `surface_oracle` passes unedited.
- **Privacy.** No new data leaves the device. A `status` change syncs like any other note field. No
  new data class, so the privacy page and `PRIVACY_VERSION` do not move.

## 2. Global constraints

- `cargo build --workspace` and `cargo test --workspace` from the root, dev profile, **0 warnings**
  (the one accepted `.rsrc` line). TDD: T2a-T2c's tests fail first, then T3 to T7 make them pass.
  **The red state** is a test that compiles and fails at run time: the test-writer adds minimal
  stubs (the signature with `_`-prefixed parameters and `todo!("O1 T<n>")`, a constant, a field
  set to `false`) so the crate builds with 0 warnings; the implementer replaces each stub (§9.1 F2).
- **Contract list untouched:** `engine/src/{write,journal,yamlemit,yaml,pystr,ledger,ids,provenance,
  approvals,sync,entitle,wincred,reconcile}.rs`, `app/src/{credentials,account,updates}.rs`, the
  oracle, sync and entitlement tests, `engine/tests/fixtures/**`. A task that finds it needs one
  stops and reports; it does not edit.
- **No new fixture files.** Tests build grade notes, figures and feed text in scratch vaults under
  the temp directory, as `completion.rs`'s and `ingest.rs`'s tests already do.
- **Existing tests keep passing unedited**, except those this plan names as changed (T2a-T2c list
  every one, with why). `#[ignore]`d tests stay ignored.
- **Tests never leave the machine.** Nothing here needs a server.
- **Files other lanes hold.** `app/src/main.rs` and `app/src/commands.rs` are L6's; this lane does not
  touch them. `app/static/console.js` is one queue (HANDOFF §2): T7b takes its turn.
- **Line endings:** LF in every file this lane edits.

## 3. Questions, all answered (2026-10-23)

- **Q-A (Quinn; the no-score half by the controller, delegated by Quinn on the same reasoning).**
  - A gradebook item `graded` with a **score of 0 is not turned in**: it stays live, and the
    late-work rule decides (spec §5, lane O6).
  - A `graded` row with **no numeric score** counts as turned in only on a column type that cannot
    hold a zero or a fail. From `grades.rs:188-213`, a graded row has no score exactly when its
    column is `Percent`, `Tabular` or `Text`, or the row has no `displayGrade` (`Score` and
    `CompleteIncomplete` rows that are graded always carry a number, `:197-203`). Each of those can
    hold a zero or a fail: `Percent` holds 0%, `Tabular` holds an F, `Text` holds any word, and a
    missing `displayGrade` says nothing. **So the set is empty: a `graded` row with no score always
    stays live.** The grade note does not record the column type anyway (`grades::desired`,
    `:419-427`), so no other rule could be applied without a new field.
  - The done rule is therefore: `needs-grading`; `exempt`; `graded` with a score above 0. Everything
    else is live (§9.1 F4).
- **Q-B (controller): as recommended.** In this lane the counts go to the run log and the run record
  only (T4, T5); the student-facing line arrives with O3's reveal (spec §1.8).
- **Q-C (controller): as recommended.** If S-C fails, T2b and T5 are not dispatched; zyBooks and VHL
  ship alone (spec §6.2 item 4), and the reorder (T6) still ships.
- **Q-D (controller): Quinn's Q11 ruling authorises the `CLAUDE.md` slot line.** It lands in the O1
  PR with no extra ask; the PR merges on green CI and the reviewer's PASS.

## 4. Conflict map

| File | This lane's change | Task | Shared with |
|---|---|---|---|
| `engine/src/completion.rs` | `apply_done`, `apply_vendor_completions`; tier 1 stops filing cards; its tests | T2a (new tests, stubs), T2c (rewritten tests), T3 | mail (`mail.rs` calls `propose_done`, which stays as is) |
| `engine/src/coursework.rs` | first-sight `imported-done`; the `archived` count; tier 1 call switched to apply | T2a (tests, stub const), T4 | L5 (its `collect_cloud` and syllabus work): rebase, no overlap expected |
| `engine/src/ingest.rs` | the gradebook index; first-sight `imported-done`; mark done after; a doc comment on slot order | T2b (tests, stubs), T5 | L5 (Q45's authoritative items): rebase |
| `engine/src/mailreconcile/rules.rs` | one test in its `mod tests` (no code change) | T2a | the mail lanes: rebase |
| `engine/src/grades.rs` | doc comment of `older_than_applied` (T6); `SOURCE` made `pub` (T5) | T5, T6 | none |
| `engine/src/surface.rs` | `Closed.undo` | T2a (tests, stub field), T7a | O6 later (the late window); L5 |
| `app/src/scheduler.rs` | `slot_argv` order and its doc comment | T6 | none in flight |
| `app/tests/scheduler.rs` | three order assertions in three tests: `slot_argv_places_grades_after_ingest_and_before_judge_only_with_a_bundle` (`:1508`), `an_accepted_privacy_version_leaves_the_slot_as_it_was` (`:2884`), `an_unknown_privacy_version_names_only_the_failed_check` (`:2907`) | T2a | none |
| `app/static/console.js`, `app/tests/static_assets.rs` | Undo on "Closed this week" | T7b | every console lane: one queue |
| `engine/tests/turned_in.rs` (new) | cross-module tests | T2a | none |
| docs (`CLAUDE.md`, `docs/reference/*`, `docs/surface/anatomy.md`, `app/README.md`, the completion note, the grades spec, `HANDOFF.md`) | §T8 | T8 | docs-keeper's queue |

## 5. Tasks at a glance

| Task | What | Agent | Why that agent | Depends on |
|---|---|---|---|---|
| T0 | Branch, spec commit, line re-check | main session | git and judgment about drift | none |
| T1 | Spike S-C and check P-1 | main session | S-C is live (a scratch profile at the founder's school); P-1 is a short code read | T0 |
| T2a | Failing tests and stubs: completion (new), coursework, surface, mail rules, scheduler, `turned_in` | `test-writer` | the failing tests, specified below | T0 |
| T1b | Confirm Blackboard score capture; fix it if missing | main session; `test-writer`, `implementer` only if missing | a code check and T1's live count; the fix, if needed, is specified in the 2026-10-16 note and test-checked | T1 |
| T2b | Failing tests and stubs: the ingest gradebook half | `test-writer` | as T2a; only after S-C passed (Q-C), with Q-A's rule and captured scores | T1, T1b |
| T2c | Rewrite the tier-1 card tests to the apply behaviour | `test-writer` | a long, mechanical rewrite kept apart from the new tests (review minor 8) | T2a |
| T3 | `completion.rs`: `apply_done` and tier 1 applies | `implementer` | specified, test-checked, not the contract list; a silent error writes vault bytes, so `high` | T2a, T2c |
| T4 | `coursework.rs`: first-sight `imported-done`; call the apply | `implementer` | as T3 | T3 |
| T5 | `ingest.rs`: the gradebook half | `implementer` | as T3; conditional on S-C (Q-C) | T2b, T3 |
| T6 | Slot order; two doc comments | `mechanical` | three lines of order and comment, checked by T2a's tests | T2a |
| T7a | `surface.rs`: `Closed.undo` | `implementer` | the read model; references must stay byte-identical | T2a (beside T3: `completion::ACTOR` already exists, `completion.rs:43`) |
| T7b | `console.js`: the Undo button | `console-ui` | `app/static` | T7a |
| T8 | Docs and the two spec amendments | `docs-keeper` | HANDOFF, references, ledgers | T3-T7 |
| T9 | Review | `reviewer` | everything a cheaper agent changed goes through review before a push | T3-T8 |
| T10 | Live proof on a scratch profile | main session | live, and the controller runs every live step | T9, Quinn's go |

T3, T4 and T5 touch different files but T4 and T5 call T3's function, so they run after it. T6 and
T7a are independent of T3-T5 and may run beside them.

## T0. Branch and spec (main session)

- [ ] Branch `feat/o1-turned-in-work` from `origin/main`.
- [ ] Commit the spec file as Quinn ruled it, unedited, if `main` does not already carry it.
- [ ] Re-read every `file:line` this plan cites on the branch; record drift as a dated line in §9.
- [ ] Confirm L5's state: if its PR-1 has merged, re-read `coursework.rs`'s first-sight branch and
  `ingest.rs`'s `sync_tasks`, and note any change to their signatures in §9.
- [ ] Record whether tier 1's cards ever shipped in a release (`completion.rs`'s own doc said "T8 is
  unreleased", `:157-158`). If they did, installed vaults can hold pending or rejected "Mark done"
  cards, and T3's card rule (§9.1 F1) is what protects them; T10 then checks one.

## T1. Spike S-C and check P-1 (main session)

**S-C: is the `<id>` in a feed task's `…GradableItem-<id>` UID the gradebook column id?**
- On a scratch vault made with `scripts\scratch-vault.ps1` from the founder's scratch profile, at the
  founder's school, with both the LMS feed and grades connected (a dev build; Quinn at the machine
  for the sign-in).
- Read-only: from `tasks/` and `archive/`, every `created_by: blackboard` note whose `source_uid`
  starts `_blackboard.platform.gradebook2.GradableItem-`; strip the prefix. From `grades/`, every
  `kind: item` note's `source_uid`.
- Record **counts only**: feed gradable items; grade item notes; ids found in both; ids in both whose
  feed title equals the column's name (compared locally, never written down); any id that matches
  more than one note on either side.
- **Pass:** every id in both is one-to-one, and the titles agree for all but a stated few, each
  explained by structure (e.g. a renamed column) without naming it. Anything else fails, and Q-C
  applies.
- **Score capture, live (for T1b; structure only):** on the same vault, count `kind: item` grade
  notes with `status: graded` that carry a numeric `score`, and those that do not. Before the
  capture's bundle is deleted (`app/src/grades.rs` deletes it after the engine runs), record for one
  row only the **key names** and each `displayGrade.scaleType` present across rows, with counts. No
  value, title or id is written down. This confirms what the 2026-10-16 note left unconfirmed.
- Clean up the scratch profile afterwards (profile, vault, credentials, autostart).

**P-1: does `grades` read anything `ingest` writes?** From the plan's read, no: `grades::apply` reads
`courses/` (`course_refs`, `grades.rs:405-406`), `grades/`, `state/grades.json` and the vault's zone;
`ingest` writes `tasks/`, `archive/`, `state/ingest-seen.md` and its run lines. Re-confirm on the
branch, including the run lock (`grades::run` holds `sync::RunLock`, `grades.rs:656`) and anything L5
added. Record the answer in §9. If `grades` does read an `ingest` output, stop and ask the
controller before T6.

Record both results as dated rows in §9. No file in the repo changes in T1.

## T1b. Blackboard score capture (main session to check; test-writer, implementer if needed)

**Why:** the done rule counts `graded` only with a score above 0. If the capture drops Blackboard's
scores (`docs/notes/2026-10-16-grades-score-not-captured.md`: the v2 row's number sits under
`displayGrade.score`, and the engine read only the top-level `score`), every graded row has no score
and only `needs-grading` and `exempt` would ever mark work done.

**What the plan's read found:** the worktree it read (`main` plus `fix/effort-anchors`) already
reads `displayGrade.score` for `Score` and `CompleteIncomplete` rows that are graded or have no
status, with the override first (`engine/src/grades.rs:190-203`), pinned by inline `v2_row` tests
(`a_v2_score_scale_row_takes_display_grade_score` and siblings, `:961-1050`); its comment records a
live check ("0 of 107" rows with a top-level score). Round 2's reviewer read the same rule on
`origin/main` at 9777ca6. So the note's fix appears to have landed (HANDOFF's lane L2) and the note's
status line is stale.

- [ ] **Check (main session):** on the lane branch, confirm `column_to_fields` takes
  `displayGrade.score` as above and those tests exist and pass. Record the answer in §9.
- [ ] **If it does not** (the fix is missing or reverted): `test-writer` first, then `implementer`,
  in `engine/src/grades.rs` only (off the contract list), per the note's "If `displayGrade` is
  present" fix: the failing test is an inline `json!` v2 row, never a new file under
  `engine/tests/fixtures/`. The `reviewer` sees it in T9. If T1's live check finds rows with **no**
  `displayGrade`, stop: the note's other branch (the v1 endpoint) changes the capture's shape and goes
  back to the planner.
- [ ] **Either way**, T1's live count of graded notes with and without a score is the proof that
  scores arrive. A count with no score on rows whose `scaleType` is `Score` is a T1b failure.

Acceptance: §9 records the code check and the live count. Size: XS if present; S if the fix is
needed.

## T2a. Failing tests and stubs, all but ingest (test-writer)

**Brief:** this section, §1, §2 (the red state), §3's Q-A and spec §6.2-§6.3. Write the tests below,
and the minimal stubs that let them compile: `completion::apply_done`, `Applied`,
`completion::apply_vendor_completions` (each `todo!("O1 T3")`), `ingest::IMPORTED_DONE` (the
constant itself), `Closed.undo` (always `false`). **Every stub is declared `pub`** in its module
(`completion`, `ingest` and `surface` are `pub mod`, `engine/src/lib.rs:44,63,88`), so a non-test
`cargo build` raises no "never used" warning (review r2, minor 1). Run them: each new or changed test compiles and
fails at run time for the stated reason; every other test passes; 0 warnings. Commit on the branch
as the lane's first code commit. Names are suggestions; behaviour is not.

**`engine/src/completion.rs`, its `mod tests`** (T3 edits the code above it, after this task):
1. `apply_done` on an active task under `tasks/`: the note's `status:` line reads `done` and every
   other byte is unchanged; the journal's last record is `op: set`, `field: status`, `old: active`,
   `new: done`, `actor: agent:knowlu.completion`.
2. A task whose `status` the student set (`op: set`, actor `student`): nothing written, the outcome
   names "set by hand", bytes identical.
3. Undo sticks: apply, then the student sets `status: active`, then apply again: still active, no
   new agent record.
4. Already `done`, `archived`, a note outside `tasks/`, a note with no `status:`: each skipped,
   nothing written.
5. A dry run writes nothing and reports "would mark done".
6. Tier 1 applies: `apply_vendor_completions` with a 100% figure marks the task done and leaves
   `approvals/` empty; a 99.6% figure writes nothing; figures are processed in uid order, duplicates
   once.
6a. **A rejected card closes the question** (§9.1 F1): a task with a `status → done` amend card
    stamped `rejected` in `archive/` (made with `propose_done`, then rejected through the approvals
    path, as a student does) is skipped by `apply_done`, outcome "already proposed", bytes and
    journal unchanged; the same when the card matches by `target_id` after the note was renamed.
6b. **A pending or snoozed card is left, and the task skipped:** the card in `approvals/` keeps its
    bytes and status, the task stays active, nothing is journaled. Then the student approves the
    card through the approvals path: the task reads `done`, applied by the card. A second case
    rejects it instead: the task stays active, and a later `apply_done` still skips it.
6c. **`WriteOpts`:** the journal record from test 1 carries the evidence object (`write`'s
    `evidence` slot) and no judgment or provenance fields (`judged: false`, §9.1 F5).

**`engine/src/coursework.rs`, its `mod tests`:**
8. A source group new to the vault, an item due after today with a 100% figure: one note in
   `archive/` with `status: archived`, `archived_reason: imported-done`, `needs_enrichment: false`,
   the vendor's other fields as `IMPORTED_PAST_TEMPLATE` carries them; the uid recorded as seen; log
   line `archived (imported-done) <stem>`; the summary line
   `coursework: N item(s) already turned in were archived from a source seen for the first time`.
9. Precedence: an item already past and at 100% in a new group is `imported-past`, unchanged.
10. A group the vault already holds: a 100% item is created live, then marked done in the same run
    by `agent:knowlu.completion` (two records, by design).
11. A 99% item in a new group is created live.
12. A dry run logs `would archive (imported-done) <stem>` and writes nothing, no folder included.
12a. **The run record's count** (§9.1 F6): a run that archives two items as `imported-past` and one
     as `imported-done` records `counts.archived == 3` on its sync step, and the step's counts carry
     no other new key; `marked done:` lines count under none of `created`, `updated`, `skipped` or
     `archived`.
13. **Changed tests:** `main_with_sources` tests that assert `proposed done:` lines assert
    `marked done:` and no card.

**`engine/src/surface.rs`, its `mod tests`:**
20. `closed_this_week` marks `undo: true` only for a close written by `agent:knowlu.completion`
    whose note still reads `status: done` at that path; `false` for the student's close, a mail
    agent's close, and a completion close the student has since undone.
21. `engine/tests/surface_oracle.rs` passes **unedited** (no assertion added; the run is the proof).

**`engine/tests/turned_in.rs` (new):**
22. A mail card that only moves `due`, aimed at an `imported-done` note in `archive/`, is refused as
    for any archived note: the note stays in `archive/` (the unarchive arm matches `imported-past`
    only, `approvals.rs:200-214`). Drive it through the public approvals entry point; if none
    reaches it, report that, and the reviewer checks the arm by reading (no contract-list edit).
23. `ingest::IMPORTED_DONE` is `"imported-done"`, and coursework's template carries it (ingest's
    template is T2b's).

**`engine/src/mailreconcile/rules.rs`, its `mod tests`** (review minor 10; a test only, no code):
22a. `amendable` refuses a `due` move on a note in `archive/` with `status: archived` and
     `archived_reason: imported-done`, however far ahead the new due is; the same move on an
     `imported-past` note is still allowed (`rules.rs:294-305`).

**`app/tests/scheduler.rs`** (§9.1 F7), three changed assertions in three tests:
24. `slot_argv_places_grades_after_ingest_and_before_judge_only_with_a_bundle` (`:1491-1512`) becomes
    `…_after_coursework_and_before_ingest_…`: without a feed, `coursework, grades, judge`; with a
    feed, `coursework, grades, ingest, judge`; no bundle, no `grades`.
24a. `an_accepted_privacy_version_leaves_the_slot_as_it_was` (`:2884`): the expected argv becomes
     `["sync", "coursework", "grades", "ingest", "judge", "rank"]`; the `children_started` line
     after it follows from `want`.
24b. `an_unknown_privacy_version_names_only_the_failed_check` (`:2907`): `children_started` becomes
     the same order.
25. `slot_argv_is_unchanged` stays green unedited (it has no feed). The held-privacy tests keep their
    assertions: the held partition is `sync`, `coursework`, `ingest`, whose named skips keep their
    order. Search `app/tests/` once more for any assertion of `ingest` before `grades`; any found is
    listed as changed, with why, in the commit message.

Acceptance: as the brief says. Size: M.

## T2b. Failing tests and stubs, the ingest gradebook half (test-writer; after S-C passed)

**Brief:** this section, §2's red state, §3's Q-A, spec §6.2. Dispatched only when T1 records S-C
as passed (Q-C) and T1b has recorded the score capture as on the branch. Stubs, each declared
`pub` as in T2a: `ingest::gradable_id` and the done predicate (`todo!("O1 T5")`), and ingest's
`IMPORTED_DONE_TEMPLATE`. **Grade notes in these tests come from captured scores:** each test
builds its `grades/` notes by running `grades::apply` on an inline v2-shape bundle (`json!`, a row
with no top-level `score` and the number under `displayGrade`, as `grades.rs`'s own `v2_row` tests
do), never by hand-writing a `score:` line, so the path a real capture takes is the one tested.
Tests in `engine/src/ingest.rs`'s `mod tests`:
14. The UID parse: `_blackboard.platform.gradebook2.GradableItem-_4686399_1` gives `_4686399_1`;
    a `CalendarEntry` UID, a `BB-ASSIGN-…@…` UID and an empty suffix give none.
15. The done rule (Q-A): `needs-grading`, `exempt`, and `graded` with a score above 0 are done;
    `graded` with a score of 0, `graded` with no score, `in-progress`, `not-submitted` and
    `removed` are live; a `kind: overall` note never joins. The scores come from the bundle: a
    `Score` row whose `displayGrade.score` is 31.5 is done; the same row at 0 is live; a `Percent`
    row at 85 is live (no score is captured for it, `grades.rs:197-203`).
15a. First ingest (`state/today.md` absent), a joined done item lands in `archive/` as
     `imported-done` with its seen record; a live one is created in `tasks/`.
16. Precedence: past and done on a first ingest is `imported-past`.
16a. The first-run lines (§9.1 F6): `ingest: first run — N item(s) already past were archived`
     keeps its text and count; a second line counts the turned-in ones, present only when that
     count is above 0.
17. A later ingest: a live feed task whose grade note now says `needs-grading` is marked done by
    `agent:knowlu.completion`; a line `marked done: <stem> (gradebook)` is logged. Two such tasks are
    marked in feed order.
18. A task the student re-opened stays active on the next ingest; so does a task with a pending or
    rejected "Mark done" card.
19. No `grades/` folder: one representative run's output is byte-identical with and without an empty
    `grades/`, and every existing `ingest` test passes unedited.

Acceptance: as T2a's. Size: S to M.

## T2c. Rewrite the tier-1 card tests (test-writer)

Every existing test in `completion.rs`'s `mod tests` that calls `propose_vendor_completions` and
asserts a card (`completion.rs:453-784` in the plan's read) is rewritten to the apply behaviour
(through the T2a stub, so it fails red), or deleted where its rule no longer exists: "one card per
task, ever" is now T2a's tests 2-4 and 6a-6b. Tests that call `propose_done` directly stay as they
are: mail still uses it. The commit message lists each changed or deleted test with why. Acceptance:
as T2a's. Size: M.

## T3. `completion.rs`: one write path for an authoritative "done" (implementer)

**Brief:** this section, §1, §2, T2a tests 1-6c, T2c, and `completion.rs`'s module doc.

- [ ] Replace T2a's stub with `apply_done(vault, task, evidence, ctx, journal, dry_run) ->
  Result<Applied, WriteError>`. Its rules, in order, are **all four of `propose_done`'s**
  (`completion.rs:12-28`, `:215-240`): a note directly under `tasks/`; `status: active`; no `op: set`
  of `status` by the student in the journal; and **no `status → done` amend card for this note, in
  `approvals/` or `archive/`, whatever its status** (`already_proposed`, by `target_id` first, else
  by path). Then it writes the one field `status: done` through `write::write` (single-line surgery,
  journal first) under `ctx.with_actor(ACTOR)`. Outcomes: `Marked`, `WouldMark` (dry run),
  `Skipped(reason)`.
- [ ] **Why the fourth rule skips a pending card rather than withdrawing it** (§9.1 F1):
  `approvals.rs` withdraws only commitment cards (`withdraw_stale`, `withdraw_asks`, `:1845-1935`;
  `superseded` is a commitment-card status, `:1566`); an amend card has no withdrawal, so a
  withdrawal from `completion.rs` would make it a second writer of `approvals/` with a status the
  amend path does not know. Leaving the card keeps the student's open question in charge: approve
  it and `apply_amendment` sets `done` (or, if the value is already there, treats it as applied,
  `:799-808`); reject it and the rejected card closes the question for good, as it does today.
- [ ] `WriteOpts { judged: false, evidence: Some(&evidence.to_json()), ..WriteOpts::default() }`,
  stated outright (`write.rs:270-281`). `judged: true` would take the provenance path
  (`write.rs:363`) and add fields test 1 says are absent (§9.1 F5). **No change to `write.rs`.**
- [ ] Replace `propose_vendor_completions` with `apply_vendor_completions` (same input, same uid
  order and dedup, `existing_by_uid` lookup), returning `(log, warnings)` with lines
  `marked done: <stem> (<source> <percent>%)` or `would mark done: …`. Its lines start with none of
  `sync_coursework`'s counted prefixes (`completion.rs` module doc, `coursework.rs:1884-1885`).
- [ ] `propose_done`, `already_proposed` and `Evidence` stay: mail's completion claims still propose
  (`mail.rs:261-271`, `enrich.rs:879`).
- [ ] Rewrite the module doc: tier 1 now applies and lists; the rules; Undo's mechanics (the
  student's `set` is the judge-once record); the reason (spec §6.3, Q3).

Acceptance: T2a tests 1-6c and T2c's tests pass; every other test passes unedited; 0 warnings.
Size: S.

## T4. `coursework.rs`: finished items archived at first sight (implementer)

**Brief:** this section, T2a tests 8-13 and 23, and `coursework.rs`'s first-sight comment block
(R-C1c-6).

- [ ] The set of uids whose figure `is_complete()` is collected from the run's `CompletionSink`
  after `collect` and passed into the sync. Keep `sync_coursework`'s public signature for its
  existing callers (a sibling that takes the set, which `sync_coursework` calls with an empty one),
  so existing tests stay unedited.
- [ ] In the create branch, after the `imported-past` check and under the same new-group cutoff
  (`coursework.rs:485-489`): a uid in the set goes to `archive/` from `IMPORTED_DONE_TEMPLATE`,
  identical to `IMPORTED_PAST_TEMPLATE` but for `archived_reason: imported-done`; one `create`, one
  `record_seen`, one line, the dry-run guard before any folder is made (M1's rule).
- [ ] The summary line beside the "already past" one (`coursework.rs:591-596`), counting both
  spellings so a dry run reports what a real run would.
- [ ] **The run record** (§9.1 F6): the sync step's `archived` count (`coursework.rs:1933-1936`)
  counts lines starting `archived (imported-past)` **or** `archived (imported-done)`. No new key: run records are a
  contract with existing vaults (`CLAUDE.md`, engine invariants). Its D3 comment says both reasons count.
- [ ] `main_with_sources_on` calls `apply_vendor_completions` where it called the propose
  (`coursework.rs:1884-1894`), after the sync, so a note created this run in an old group can be
  marked this run.

Acceptance: T2a tests 8-13 and 23 pass; all else unedited and green. Size: S to M.

## T5. `ingest.rs`: the gradebook half (implementer; only if S-C passed)

**Brief:** this section, T2b's tests, spec §6.2 items 1-4, §3's Q-A, and `ingest.rs`'s R-OB-3
comments.

- [ ] A read-only index of `grades/`: per `kind: item` note with `source: blackboard`, its
  `source_uid` (the column id), `status` and `score`. Built once per run, in path order. A note that
  does not parse is skipped. No `grades/` folder means an empty index and today's behaviour exactly.
  The fields are the ones `grades::desired` and `create_note` write (`kind`, `source`, `status`,
  `score`, `source_uid`; `grades.rs:419-427`, `:477`). `grades::SOURCE` is private today; making it
  `pub` is the one allowed code edit to `grades.rs`; T6 changes only a doc comment there.
- [ ] `gradable_id(uid)`: the suffix after exactly `_blackboard.platform.gradebook2.GradableItem-`,
  non-empty, else `None`. No other UID shape ever joins.
- [ ] The done rule (§3 Q-A): `needs-grading`; `exempt`; `graded` with a numeric `score` above 0.
  Everything else is live, including `graded` with a score of 0 and `graded` with no score.
- [ ] **First ingest** (`is_first_run`): in `sync_tasks`'s create branch, after the `imported-past`
  check (`ingest.rs:746-781`), a joined done item goes to `archive/` from a new
  `IMPORTED_DONE_TEMPLATE` (the past template with `archived_reason: imported-done`), using T2a's
  `IMPORTED_DONE`. Keep `sync_tasks`'s signature for existing callers (a sibling taking the index,
  as T4 does).
- [ ] **The first-run lines** (§9.1 F6): ingest keeps no run-record counts, only its first-run line
  (`ingest.rs:1102-1104`), whose text and count stay as they are; a second line,
  `ingest: first run — N item(s) already turned in were archived`, follows it when N is above 0.
- [ ] **Every ingest, after the sync:** the run's feed events, **in feed order**, each joined to its
  note through `existing_by_uid`; each note in `tasks/` with `status: active` whose UID joins a done
  grade gets `completion::apply_done` (which also skips a task with a pending or rejected card),
  evidence `{source: "blackboard-gradebook", status, column}` and a summary such as "Blackboard's
  gradebook shows this submitted". Line: `marked done: <stem> (gradebook)`. A task whose event left
  the feed is not visited.
- [ ] Update `is_first_run`'s doc comment for the new slot order (`ingest.rs:473-492` names
  `coursework → ingest → judge → rank`).

Acceptance: T2b's tests pass; every existing `ingest` test passes unedited. Size: M.

## T6. The slot order (mechanical)

**Brief:** this section and T2a tests 24-25.

- [ ] `app/src/scheduler.rs` `slot_argv` (`:515-549`): push the `grades` step (when a bundle exists)
  right after `coursework` and before the `ingest` check. Update the doc comment at `:505-513` to
  `sync → coursework → grades → ingest → judge → rank`, with one sentence of why: ingest reads the
  grade notes to keep turned-in work out of `tasks/` (spec §6.2, Q11).
- [ ] `engine/src/grades.rs` `older_than_applied`'s doc (`:617-622`): the slot now applies after
  `sync` and `coursework`, before `ingest`. Doc only.
- [ ] Nothing else in `run_slot_inner` changes: the privacy-held partition still holds `sync`,
  `coursework` and `ingest` (`scheduler.rs:985-991`) and their named skips keep their order.

Acceptance: T2a tests 24-25 pass; 0 warnings. Size: XS.

## T7a. `surface.rs`: the Undo flag (implementer)

- [ ] Replace T2a's stub: `Closed.undo: bool`, serialised. True only when the row's closing record is `op: set`,
  `field: status`, `new: done`, `actor` equal to `completion::ACTOR`, and the note at `path` still
  reads `status: done` (one read per such row). Every other row is `false`.
- [ ] `docs/surface/anatomy.md`'s entry is T8's; leave a one-line doc comment on the field here.

Acceptance: T2a tests 20-21 pass; the three `surface-today-*.json` references are byte-identical
(they hold no closed rows). Size: S.

## T7b. `console.js`: Undo on "Closed this week" (console-ui)

- [ ] In `renderClosed` (`console.js:393-397`), a row with `undo` gets an **Undo** button. It calls
  the existing `set_fields` command with the row's `id` and `{status: "active"}`, the same `view`
  argument the page's other task edits pass, then re-renders from the returned state. A failure
  shows the command's error on the row; nothing is retried.
- [ ] A static test in `app/tests/static_assets.rs` pins the handler and that it calls `set_fields`.
- [ ] No new Tauri command; `main.rs` and `commands.rs` untouched.

Acceptance: the static test passes; a fixture render (the console's `?fixture=` path) shows the
button on a flagged row only. Size: S.

## T8. Docs and the two amendments (docs-keeper)

Each edit by its text, never by line number.
- [ ] `CLAUDE.md`, "Command and app contracts": the slot line reads
  `sync → coursework → grades → ingest → judge → rank`. One clause of why is enough. Authorised by
  Quinn's Q11 ruling; it lands in this PR with no extra ask (Q-D).
- [ ] `docs/reference/app.md` (the scheduler's slot line), `app/README.md` (both slot mentions, the
  second already stale), `docs/reference/engine-commands.md` (`coursework` and `ingest`: the
  `imported-done` archive, the `marked done:` lines and the new summary lines; tier 1 applies rather
  than proposes), `docs/surface/anatomy.md` (`Closed.undo`).
- [ ] The completion design note (`docs/notes/2026-09-22-completion-detection-design.md`) §2 and
  §7a: an amendment block dated 2026-10-23, citing Q3: authoritative completion is applied and
  listed with Undo; cards remain for mail's evidence.
- [ ] The grades spec (`docs/specs/2026-09-29-grades-design.md`) §10 and §12: an amendment block
  citing Q11 and Q3: the slot order; the status-only join of §6.2 leaves the out-of-scope list (no
  `course` or `due` pinning, which stays out).
- [ ] `HANDOFF.md`: lane O1's row and status, and one expected effect of the upgrade (review minor
  11): on an existing vault, the first slot after the update marks every live task the source
  already shows as finished (each submitted Blackboard item with grades connected, each zyBooks or
  VHL item at 100%) done at once, each with an Undo row on "Closed this week" for seven days. Tasks
  with a pending or rejected "Mark done" card are left alone. This is Q3 working, not a fault.
- [ ] `docs/reference/engine-commands.md` also states the done rule (§3 Q-A) and the card rule
  (T3's fourth rule), including that **a "Mark done" card the student rejected, whatever filed it
  (a mail completion claim included), blocks any later gradebook or vendor answer for that task, by
  design**: the student's answer stands (review r2, minor 3).
- [ ] `docs/notes/2026-10-16-grades-score-not-captured.md`: its status line records T1b's result
  (fixed on `main`, and T1's live count), so the note no longer reads "not fixed".

Acceptance: `scripts/ci/eol-check.ps1` clean; no source file in the diff. Size: S.

## T9. Review (reviewer)

Pre-push review of the whole lane against this plan, the spec's §6 and §1-§2 here. Report:
`docs/reports/2026-10-2X-o1-turned-in-work-review.md` (the day it runs). It checks, besides the
code: no contract-list file and no fixture in the diff; the frozen references unchanged; every
changed test listed with a reason; 0 warnings; the CI line; T1's two records in §9.

## T10. Live proof (main session, on Quinn's go)

On a scratch profile at the founder's school, a dev build against staging, structure and counts
only:
1. A new vault with zyBooks connected: after the first slot, items at 100% in `archive/` as
   `imported-done`, none live; the run log's line states the count.
2. Grades connected after the first slot (today's wizard): the next slot's ingest marks submitted
   Blackboard tasks done; they appear on "Closed this week" with Undo. Count, too, the feed tasks
   joined to a grade note that **stay live** because the note is `graded` with no score (Percent,
   Tabular or Text columns), and those left live by a score of 0 (review r2, minor 4). The first
   count is the part of the goal this rule leaves unmet; it goes to Quinn with the proof.
3. Undo one: it returns to Today and stays there after another slot.
4. The slot's steps in the Runs view read `sync, coursework, grades, ingest, judge, rank`.
5. **The upgrade flood** (review minor 11): a scratch copy of a vault that existed before this lane
   (`scripts\scratch-vault.ps1 -Source <that vault>`), run once on the new build. Record how many
   tasks were marked done in that one slot, that each has an Undo row, and, if T0 found tier-1
   cards shipped, that a task with a pending or rejected "Mark done" card was left alone.
Then remove the scratch profile (profile, vault, credentials, autostart). Record the counts as dated
rows in §9.

---

## 6. Fidelity ledger

| Spec | Requirement | Task | Proof |
|---|---|---|---|
| §6.2 table, §3 Q-A | Done means the source's own word: gradebook `needs-grading`, `exempt`, `graded` with a score above 0; vendor exactly 100% | T3, T4, T5 | T2a tests 6, 8, 11; T2b test 15 |
| §6.2 table, §3 Q-A | A graded zero and a graded row with no score stay live; the late rule decides | T5 | T2b test 15 |
| §6.2 (added) | Only item columns join, never Overall | T5 | T2b test 15 |
| §6.2 item 1 | At first sight, archived, never created; one create, one seen record, one line; never judged | T4, T5 | T2a test 8; T2b test 15a; `needs_enrichment: false` in both templates |
| §6.2 item 1 | Precedence with `imported-past` unchanged | T4, T5 | T2a test 9; T2b test 16 |
| §6.2 item 1, run records | `imported-done` counts under the existing `archived` key; no new key; ingest's first-run line kept, a second line added | T4, T5 | T2a test 12a; T2b test 16a |
| §6.2 item 2 | After first sight: `status: done` by `agent:knowlu.completion` through `write`, journal first, `judged: false`, evidence on the record | T3, T4, T5 | T2a tests 1, 6c, 10; T2b test 17 |
| §6.2 item 2 | Listed with Undo on "Closed this week"; Undo is the student's `set` | T7a, T7b | T2a test 20; T7b's static test; T10 step 3 |
| §6.2 item 3, VISION | A hand-set `status` is never written or proposed; `apply_done`'s checks are the only fence | T3 | T2a tests 2, 3; T2b test 18 |
| §6.2 item 3, Q3 | A rejected "Mark done" card closes the question; a pending one is left and the task skipped | T3 | T2a tests 6a, 6b; T2b test 18; T10 step 5 |
| §6.2 item 4 | The join is exact, by the GradableItem id; no other UID shape | T5 | T2b test 14; S-C (T1) |
| §6.2 item 4 | S-C fails: zyBooks and VHL ship alone | T1 | Q-C: T2b and T5 not dispatched |
| §6.2 "first sight" | Per source group for coursework; the first ingest for ingest | T4, T5 | T2a tests 8, 10; T2b tests 15a, 17 |
| §6.3, Q3 | Tier 1's cards retired for authoritative sources; mail keeps `propose_done` | T3 | T2a test 6; T2c; `mail.rs` untouched |
| §6.2, Q11 | Slot is sync → coursework → grades → ingest → judge → rank | T6 | T2a tests 24-25; T10 step 4 |
| §6.4, Q11 | The `CLAUDE.md` line and the references | T8 | T8's diff; Q-D |
| §6.4 risks | `imported-done` is never unarchived by a mail date change, in the approvals arm or the reconcile rule | none (by construction) | T2a tests 22, 22a |
| §6.4 risks | The upgrade marks many tasks at once on an existing vault | T8, T10 | HANDOFF's line; T10 step 5 |
| §11 rows | Completion note and grades spec amended | T8 | T8's diff |
| §1.8, §9 | The visible count | deferred to O3 | Q-B |
| §8 | Effort is not spent on finished work at first sight | T4, T5 | the templates' `needs_enrichment: false` |
| Rule 2 | No frozen reference changes | T7a | T2a test 21; T9 |
| (plan) | No contract-list file, no fixture file, no `main.rs` or `commands.rs` | all | T9 |
| (plan) | P-1: `grades` reads nothing `ingest` writes | T1 | §9's record; round 1 confirmed it on `main` |
| §3 Q-A, r2 addendum | Blackboard scores are captured, so "`graded` with a score above 0" can fire | T1b | §9's code check; T1's live count; T2b tests built from inline v2 bundles |
| r2 minor 4 | Joined items left live as graded with no score, or with 0, are counted | T10 | T10 step 2 |

## 7. Checkpoints

1. **Plan review** (`reviewer`, plan mode) before T2a. Findings are answered in §9 and the status
   line records each round's verdict.
2. **Quinn:** T10's go. (Q-A is ruled; Q-D needs no ask.)
3. **Controller:** T1's S-C verdict and T1b's score-capture record before T2b and T5.
4. **T9** before the push; the lane PR merges on green CI and the reviewer's PASS, the `CLAUDE.md`
   line included (Q-D).

## 8. Out of scope

- The visible first-run count and the reveal (O3, Q-B).
- Email receipts as completion evidence (the completion note's T9); mail's own proposals.
- Pinning feed tasks to courses or dues by gradebook column (grades spec §12 keeps it out).
- The 80-99% band (a later tier); a resubmission that reopens work (the student's Undo covers it).
- Whether `judge` skips a task marked done after it was judged (it was judged already; no new cost).
  If T9 finds `judge` would judge a `done` task, it is a follow-up, not this lane.

## 9. Fidelity notes

*(T0's drift check, T1's S-C and P-1 records, review rounds and the controller's answers go here,
dated, newest first.)*

### 9.2 Plan review, round 2 (PASS, 4 minors; folded in 2026-10-23)

Report: `docs/reports/2026-10-23-o1-plan-review-r2.md`, with the controller's addendum.
- Minor 1: every stub is declared `pub` (T2a, T2b).
- Minor 2: spec §6.2's done-table cell reads "`graded` with a score above 0".
- Minor 3: T8's `engine-commands.md` entry says a rejected card, whatever filed it, blocks later
  gradebook or vendor answers by design.
- Minor 4: T10 step 2 counts joined items left live (graded with no score, or with 0).
- Addendum: new task T1b before T2b. The plan's read shows the 2026-10-16 note's fix already in
  `grades.rs` (`displayGrade.score` for `Score` and `CompleteIncomplete` rows), so T1b is a check
  plus T1's live count, with the note's test-first fix only if the check fails; T2b builds its grade
  notes from inline v2 bundles through `grades::apply`. **This does not change the plan's shape:**
  one conditional task, one live count added to an existing spike, and test inputs; no new file,
  agent or dependency beyond T2b's wait for T1b.

### 9.1 Plan review, round 1 (FAIL; answered 2026-10-23)

Report: `docs/reports/2026-10-23-o1-plan-review-r1.md`. The reviewer checked against `origin/main` at
9777ca6 and found the citations held to within a few lines, P-1 held, and the three references hold
empty `closed_this_week` lists.

- **F1 (critical): a rejected "Mark done" card stopped protecting the task.** Answered: `apply_done`
  keeps all four of `propose_done`'s rules, the fourth being `already_proposed` (T3). A rejected card
  in `archive/` skips the task forever. A pending or snoozed card is **left, and the task skipped**,
  not withdrawn: `approvals.rs` has no withdrawal for amend cards (only commitment cards, stamped
  `superseded`), so a withdrawal from `completion.rs` would add a second writer of `approvals/` with
  a status the amend path does not know, while leaving the card keeps the student's answer in
  charge (approval applies or no-ops cleanly, `approvals.rs:799-808`; rejection closes the question).
  Tests: T2a 6a, 6b; T2b 18; T10 step 5.
- **F2 (important): T2 could not be red without compiling.** Answered: §2 defines the red state as
  compiling and failing at run time; T2a and T2b add `todo!()` stubs with `_`-prefixed parameters,
  constants and a `false` field, so the build keeps 0 warnings.
- **F3 (important): T2 committed tests whose code might never ship.** Answered: T2 split into T2a
  (no dependency on S-C), T2b (the ingest half, dispatched only after S-C passes, with Q-A's ruled
  rule) and T2c (the tier-1 rewrite, review minor 8).
- **F4 (important): Q-A let missing work vanish.** Answered by Quinn's ruling and the controller's
  delegated answer (§3): a graded zero stays live, and a `graded` row with no score stays live
  because every column type that produces one (`Percent`, `Tabular`, `Text`, no `displayGrade`) can
  hold a zero or a fail. T2b test 15 pins each case.
- **F5 (important): `write`'s guard is not a second fence.** Answered: §1 corrected; T3 states
  `WriteOpts { judged: false, evidence: Some(..), .. }`; T2a test 6c pins the record's shape.
- **F6 (important): the run-record count.** Answered: `imported-done` counts under coursework's
  existing `archived` key, no new key (T4, T2a 12a). Ingest has no run-record counts; its first-run
  line keeps its text and a second line counts the turned-in items (T5, T2b 16a).
- **F7 (important): three order-pinning scheduler assertions.** Answered: all three named in §4 and
  in T2a tests 24, 24a, 24b (`:1508`, `:2884`, `:2907`, the last in
  `an_unknown_privacy_version_names_only_the_failed_check`).
- **Minors.** 8: T2 split (F3). 9: T7a depends on T2a only and runs beside T3. 10: T2a test 22a
  covers `mailreconcile::rules::amendable` in that file's own tests. 11: HANDOFF's line (T8) and T10
  step 5. 12: T5 states feed order for the mark-done pass.
- **Controller answers folded in:** Q-A (Quinn), Q-B, Q-C as recommended, Q-D (no extra ask), §3.
