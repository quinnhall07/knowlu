# Pre-execution review: commitment model phase 2 plan

**Reviewed:** `docs/plans/2026-09-24-commitment-model-phase2-plan.md` (4099 lines, at `3d984d7`)
against `docs/specs/2026-09-24-commitment-model-phase2-design.md` (the spec), the parent
`docs/specs/2026-09-23-commitment-model-design.md` (§2, §3.5, §5.1–§5.3, §6.4) and `CLAUDE.md`.
**Reviewer:** pre-execution reviewer, 2026-09-24. Read-only review: no cargo run, no code or plan
edited. Every "evidence" line below was read in the worktree at `3d984d7`.

## Verdict: ready with fixes

The plan is unusually well grounded. I checked every helper, type, field, line range and test
fixture builder it names (about 90 of them), and all of them exist with the signatures it uses.
Examples: `create_confirmed`/`create_marker`, `twin_keys`, `created_by_this_card`, `strict_meets`,
`file_card`, `parse_window`, `write::{to_literal, write_literals, WriteOpts}`,
`approvals::{sorted_md, as_date, withdraw_stale, Withdrawal, read_note, str_field, rel_path, stem_of, name_of}`,
`ids::{resolve_target, rel}` (with `commitments` in `NOTE_FOLDERS`), `yaml::from_json`,
`surface::build_state_preview`, `scheduler::engine_exe`, `childproc::NoConsole`,
`ConsoleState::{lock, vault_io, data_dir, seen_at, set_test_today}` and `commands::{now_in, mutate, state_inner, attach_scheduler}`.
The test code reads as compiling, apart from the one warning in finding 5. Each test asserts
the behaviour its name claims. Every new test fails before its implementation, most as a compile
error and the rest on a real assertion. No phase-1 assertion is changed. The phase-1 edits are
setup only: a seeded journal day, a moved `ts` and a decline marker.

No finding is Critical. One is Important: the first-day rule (Q1-a) has a hole east of UTC that
defeats D1/D2 for that student. Its fix is small, and it is also my ruling on flagged problem 7.
The Minor findings are each a line or two. Execute once finding 1 is applied, or once Quinn
accepts it as named, together with any Minor fixes the controller takes.

**Current counts, by script over `app/src/main.rs`:** wizard 29, console 42, 61 distinct. The
plan's 42 → 47, 29 and 66 are right. They also match the worktree's `CLAUDE.md` (42/29/61,
recounted 2026-09-17), and Q12 Step 4 edits exactly those lines.

## Findings

### 1. Important: the first day is wrong for a student east of UTC who onboards after local midnight

- **Plan:** Q1-a, lines 203–209 (ruling), 343–350 (`vault_day`), 4096–4099 (flagged problem 7).
- **Evidence:** journal day files are named by the record's UTC date (`engine/src/ledger.rs:83–85`,
  `file_for`). `vault_day` treats `today <= first` as day 1.
  - The plan analyses only the west case. There, the file date is local-tomorrow, which gives one
    extra quiet day. That cost is harmless and already named.
  - East of UTC it goes the other way. Take a student in UTC+2 who finishes the wizard at 00:30 on
    Sep 24. That is 22:30 UTC on Sep 23, so the first file is `2026-09-23.jsonl`. The console's
    local `today` is 09-24, so `vault_day` = 2 on the student's first real day. Then:
    - `overview.setup` is false, so the confirm screen never opens (D1, D3);
    - the first slot's `rank` files every proposal card (the D2 gate misses);
    - `emit_asks` starts one real day early.
  - The window is narrow: local midnight to the UTC offset, east of Greenwich only. But it defeats
    exactly what D1 and D2 exist for. CLAUDE.md's first rule forbids single-user assumptions, and
    a US-only reading of "the first day" would be one.
- **Fix:** count from the *vault-local* date of the earliest journal record, not from the file
  name:
  - `JsonlLedger::first_ts()` returns the smallest `ts` in the earliest day file. It lives in
    `ledger.rs`, so the `.jsonl` seam test (`ledger.rs:806`) still holds.
  - `vault_day` parses that `ts` as a `jiff::Timestamp` and takes
    `.to_zoned(crate::cli::vault_zone(vault)).date()` (`vault_zone` is `pub`, and `commands.rs:19`
    already uses it). When no `ts` parses, it falls back to the file-name date.
  - Day 1 is then `today == first` exactly; keep `.max(1)` for a pinned `--today` before it.
  - The Q1 tests stay: `journal_on` stamps noon UTC, which is the same date in any zone the tests
    use. Add one case with `config/ingest.yaml` `timezone: Europe/Athens` and a record at
    `2026-09-23T22:30:00.000Z`: that is day 1 on 09-24 and day 2 on 09-25.
  - Amend spec D2 and parent §5.3: "the vault-local date of its earliest journal record".
  - `seed_journal_day` stamps noon UTC on 09-06, which is 07:00 CDT on 09-06. It keeps
    `P16_MONDAY` as day 2 in the P16 vaults' `America/Chicago`.
  - **But** Q1-c moves `commitment_note`'s stamp to `2026-09-06T00:00:00.000Z`. That is 19:00 CDT
    on 09-05, so under the local rule it would make `P16_MONDAY` day 3 and asks would fire. Stamp
    it `2026-09-06T12:00:00.000Z` instead. It is still before every rank.
  - A vault with no `timezone:` uses the machine's zone (`cli.rs:117–125`). Noon UTC is the same
    date anywhere from UTC−11 to UTC+11, so the tests stay stable on any development machine.

### 2. Minor: an all-blank *Your day* makes Finish write no planning-day note, against D3

- **Plan:** Q11, `finishWeekSetup`, lines 3769–3771 (`if (seq) { payload.window = seq; }`).
- **Evidence:** spec D3 (lines 55–57) says "Finish on the screen always writes the note (§3), so
  the screen never comes back after Finish". If the student clears every picker, `windowSequence`
  returns `null` and no `window` key is sent (Q4-b), so no note is written. `setup` then stays
  true, and the screen returns at the next launch that day.
- **Fix:** in `finishWeekSetup`, when `seq` is null, call
  `showWindowError(EL("ws-window"), "Set the hours for at least one day")` and return without
  invoking. Optionally, add a `check_week_setup` case that clears the pickers and expects no
  `commitments_confirm` call.

### 3. Minor: `class_note_for` ignores Q2-c's title rule, so a covered course can still be settled into a duplicate

- **Plan:** Q2-c and `uncovered_courses` (lines 614–641) versus Q7's `class_note_for`,
  `settle_ask_approved` and `ask_withdrawal_reason` (lines 2328–2410).
- **Evidence:** `uncovered_courses` counts a confirmed `class` note with no `course:` as covering
  `to_code_exempt(title)`. `class_note_for` matches only `n.course`. Suppose an ask for BUI 100 is
  pending and the student hand-writes `commitments/bui.md` (`kind: class`, title "BUI 100 Lecture",
  no `course`). Then:
  - `emit_asks` would no longer file an ask for the course;
  - `ask_withdrawal_reason` does not withdraw the pending one;
  - approving it creates a second class note (`settle_ask_approved` refuses only through
    `class_note_for`).
- **Fix:** one private helper, `covered_course_keys(set, codes) -> BTreeSet<String>`, holding
  Q2-c's two rules. `uncovered_courses`, `class_note_for` (as a membership test) and
  `ask_withdrawal_reason` all use it. Add one approvals test: a title-only note withdraws the ask.

### 4. Minor: Q6 Step 4 leaves the list of phase-1 tests it edits to the run; it is exactly one

- **Plan:** lines 2124–2148 name as "expected candidates"
  `rank_withdraws_a_card_whose_series_left_the_file`, the `age_out_cs100` users, `p16ended` and
  `p16starved`.
- **Evidence:** I traced each P16 rank past day 2 in `engine/src/cli.rs`:
  - The `age_out_cs100` tests (`cli.rs:2853–2905`) confirm `CS100_NOTE` (`course: cs-100`), so the
    course is covered and no ask is filed.
  - `no_card_is_withdrawn_and_refiled_across_three_ranks` (days 3 and 4; `cli.rs:2816–2850`) keeps
    CS 100's class proposal in `proposals`, so the course is covered.
  - Every other P16 test ranks only on `P16_MONDAY`, which is day 2.
  - Only `rank_withdraws_a_card_whose_series_left_the_file` (`cli.rs:2769–2793`) ranks on
    `p16_day(14)` with CS 100 neither confirmed nor proposed. Its
    `**Approvals: 1 pending** (oldest 2d)` and `pending == 1` assertions would read 2.
- **Fix:** name that one test in Step 4 and in the commit message. Keep the "stop and report if
  anything else fails" rule.

### 5. Minor: an unused import breaks the 0-warnings gate at the Q1 and Q2 commits

- **Plan:** line 228, `use crate::write::WriteContext;` in Q1's new `mod phase2_tests`.
- **Evidence:** the first use of `WriteContext` in that module is Q3's `human()` (line 846). Until
  then, `cargo test` prints `warning: unused import`, which breaks Global Constraint 2 at two
  commits.
- **Fix:** add the import in Q3 Step 1, beside `human()`.

### 6. Minor: `answer_card` would approve any approval it is pointed at

- **Plan:** Q8 `answer_card_inner`, lines 2780–2800.
- **Evidence:** nothing checks that `id` is a `commitment-ask` card. A wrong id from the page, such
  as a task proposal's, gets a stray `answer_meets:` line and is *approved and materialised*.
  Today only `askRow` calls it, so this is defence in depth.
- **Fix:** add an engine read, `commitments::check_answerable(vault, id) -> Result<(), String>`,
  which requires `type: approval` and `kind: commitment-ask` with status `pending`. Call it first
  inside the `mutate` closure; that is marshalling plus an engine check, exactly like Q8-a. Add
  one assertion to the app test: a task-proposal id is refused and the task is not created.

### 7. Minor: the confirm screen's editor has no preview (Q11-a), but the spec says one component

- **Plan:** Q11-a, lines 3530–3533.
- **Evidence:** spec §4, last bullet of the editor: "The editor is the same component inside
  `#week-setup` (§2) and in the *Schedule* view". Quinn's ruling (spec preamble) is that "the
  student sees how an edit changes the suggestions".
- **Ruling:** the plan's reason holds. On day 1 the first slot has not ranked, so there is no plan
  to diff against, and the preview would mostly print "No change". Accept Q11-a and amend spec §4:
  "…the same pickers; the preview runs only in the *Schedule* view".

### 8. Minor: two stale descriptions after Q8 and Q10

- **Evidence:**
  - `app/static/console.js:11–13` says its `EDITABLE` is "the twelve fields `set_fields` accepts
    (app/src/commands.rs's EDITABLE)". After Q8 that is false (14 against 12, by design, Q8-a).
  - `docs/surface/anatomy.md` §2's page table (lines 41–51) and its "Three panels that are not
    pages" paragraph (58–61) explain *why* panels have no hash. The *Schedule* view is the first
    nav view that is not a read-model view.
- **Fix:**
  - Q8 rewrites the `console.js` comment: "the drawer's twelve; `set_fields` also takes `kind` and
    `level` on a commitment (Schedule view)".
  - Q12 Step 2 adds a *Schedule* row to §2's page table. It also adds one sentence to the "not
    pages" paragraph: a nav view may be a page view as long as it polls `state` as `today`
    (`stateView()`).

### 9. Minor: repaint side effects in the new views (no fix required; name them)

- **Q10:**
  - `saveCommitment` runs `applyEnvelope`, which paints and so already calls
    `renderScheduleView`; the plan then calls it again. That is two `your_week` round trips and two
    previews per click. Drop the explicit `return renderScheduleView()`, or keep it and accept the
    cost.
  - A poll repaint while the kind `<select>` is open rebuilds the list and closes the select.
    Paint happens only on a new `revision`, so this is rare.
- **Q11:** a `revision` change repaints `#dec-list` and drops any half-entered day toggles and
  times in an ask form. The deck already preserves notes per id (`console.js:448–450`). Preserve
  the ask rows' toggles the same way, or accept.
- **Q9:** `commitment_proposals` spawns `commitments --json` with no timeout of its own. If the
  fetch hangs, the screen stays on "Reading your calendar…". The rest of the screen, Finish
  included, still works, so this is acceptable. Name it in `app/README.md`.

### 10. Minor: line references that have drifted (harmless; the plan says find by name)

- `decide_inner` is at `app/src/commands.rs:267–283`, not 247–263.
- `pub mod updates;` is at `app/src/lib.rs:17`.
- `withdrawal_reason` ends before line 3906, not at 3912.
- `to_code` is at line 671.

None of these misleads, because each step also names the item.

## Invariants, checked

| Invariant | Holds? | Where |
|---|---|---|
| Journal-first writes through `write` | Yes | Every note goes through `write::create` (`create_confirmed_as`, `create_marker_as`, `file_card`) or `write_literals` (the window line, the ask stamps, `answer_meets`, `status`). |
| No note parsed and re-dumped | Yes | The window line is the *input* re-emitted by `to_literal` (Q4-c). `check_console_edit` and `asked_courses` only read. |
| Frozen fixtures and surface-oracle JSONs untouched | Yes | No fixture vault has `courses/`, a series file or a planning-day note. `rank`'s oracle vault never reaches the day gate's effect, because it has no series. Q12 Step 5 byte-checks. |
| `commands.rs` computes nothing | Yes | Kind and level are checked by `check_console_edit` (engine). `answer_card` filters warnings by file-name prefix, which is marshalling. Finding 6 adds an engine check, not app logic. |
| `rank` never reaches a model or the network beyond transport | Yes | `emit_asks`, `withdraw_asks` and `vault_day` read the vault only. The `rank_cannot_reach_a_judgment_endpoint` scan covers `commitments.rs`, `approvals.rs`, `cli.rs` and the `commitments` arm, and no new string trips it. |
| `--confirm` makes no network call | Yes | No fetcher is built and `cloud.yaml` is not read. `confirm_makes_no_network_call` fails if an ICS fetch is added. |
| Judge-once | Yes | No agent writes `kind`, `level` or `window` on an existing note. The only agent write after a student's is Q7-a's `status: pending` on a returned card, the same kind of status transition the snooze wake already makes (`approvals.rs:1279–1282`). It is not a judged field. |
| 15-a-day cap | Yes | `emit_asks` takes `daily_approval_budget − count_proposals_created` after the checks are filed (Q6 Step 3), and never exceeds 2 a day. `--confirm` files no card. |
| `LOCAL_CARD_KINDS` | Yes | The constant is unchanged, and every ask goes through `file_card`. |
| Tauri counts 42 → 47 and 29 | Yes | Q8 +1 and Q9 +4. Q9 Step 4 and Q12 Step 1 recount by script. |
| No typed text on the confirm screen | Yes | Pinned statically (section markup and `setupRow`) and behaviourally (the DOM query in `check_week_setup`). The editor inputs are `type="time"`. |
| The D2 day-1 gate | Mostly | Right in `cli::commitment_passes` (Q1-b), so change cards pass and the 30-odd direct `emit_checks` tests are untouched. Wrong day east of UTC: finding 1. |

## Cross-task table (shared files and interfaces)

| From → to | What the first produces | What the second consumes | Match? |
|---|---|---|---|
| Q1 → Q5, Q6 | `commitments::vault_day(&Path, Date) -> i64` | `overview.setup` (`== 1`); `emit_asks` (`< ASK_FROM_DAY`) | Yes |
| Q1 → Q6 (cli tests) | `seed_journal_day`, `p16_vault` on 09-06, `commitment_note` stamp | `decline_cs100_ask` placed beside it; `rank_files_an_ask…` relies on `p16_day(1)` = day 3 | Yes (see finding 1 for the stamp) |
| Q1 → Q6, Q7 (cli) | `asked` beside `proposals` in `commitment_passes` | Q6 passes `&proposals` to `emit_asks`; Q7 inserts `withdraw_asks` after the `proposals` line (`withdrawn_pending` is already `let mut`, `cli.rs:824`) | Yes |
| Q2 → Q3–Q8 (tests) | `course`, `t`, `meet`, `note_of`, `proposal_of`, `three_courses`, `slugs` | `week_vault` (course, t), Q6 (three_courses) | Yes |
| Q2 → Q5, Q6 | `uncovered_courses(vault, &Commitments, &[Proposal], &Codes) -> Vec<UncoveredCourse>` | `overview` (stored.set, proposals, codes); `emit_asks` (load, proposals, Codes::load) | Yes |
| Q2 → Q7 | `active_courses(vault) -> Vec<(slug, title)>` | `settle_ask_approved`'s title | Yes |
| Q2 → Q5, Q9, Q11 | `proposal_value` keys `{kind, level, title, course, meets, when, where, from, until, source_uid, window}` | `overview.office_hours`; `commitment_proposals_inner` passes them through; JS reads `p.window`, `p.kind`, `p.when`, `p.where`, `p.source_uid`, `p.level`, `win.meets[].days/start/end` | Yes |
| Q2 → Q9, Q11 | `--json` top-level `uncovered_courses` (`CommitmentsReport.uncovered`) | `v["uncovered_courses"]`, then `r.uncovered_courses` | Yes |
| Q3 → Q4, Q7 | `create_confirmed_as` (body moved) | Q4's planning-day create; Q7 adds the `card:` body branch *inside* `create_confirmed_as` | Yes |
| Q3 → Q5 | `Stored { file, set, codes, template, proposals, unanswered, warnings }` | `overview` uses `set`, `codes`, `template.day_start/day_end` (pub, `weekcal.rs:104–105`), `proposals` and `warnings` | Yes |
| Q3 → Q4 | `ConfirmInput`, `parse_confirm`, `ConfirmReport::to_json`, `confirm(…) -> Result<ConfirmReport, String>` | `cli::commitments_confirm`; `main.rs` prints `to_json` through `dumps_value`, `Err` → exit 2 | Yes |
| Q3 → Q4, Q5, Q8 (tests) | `week_vault`, `run`, `input`, `files`, `journal_records`, `human` | used unchanged | Yes |
| Q4 → Q9 | argv `commitments --vault --today --confirm --via dashboard`; stdout keys `{created, declined, warnings, window}`; stderr `knowlu-engine: <why>`, exit 2 | `confirm_argv`; `commitments_confirm_inner` strips the prefix into `error` | Yes |
| Q4 → Q10 | the error text `planning day <day>: …` (`parse_window`, `commitments.rs:4141–4143`) | `showWindowError`'s `/planning day (\w+)/` | Yes |
| Q5 → Q9 → Q10, Q11 | `Overview::to_json`: `commitments[{id, path, kind, level, title, course, meets, when, where, from, until}]`, `office_hours`, `window[{day, start, end, source}]`, `uncovered_courses`, `setup`, `warnings` | `your_week` `{ok, error, week}`; `renderScheduleView` and `checkWeekSetup` read exactly these | Yes |
| Q6 → Q7 | `COMMITMENT_ASK`, `ask_key`, the card's `course:` and `source_uid: card:<slug>` | the settlement arms, `withdraw_asks`, the approvals tests' fixture card | Yes |
| Q7 → Q8 | `Returned(why)` → `result.warnings` as `"<file name>: <why>"` (`name_of`) | `answer_card_inner` prefix from `resolve_target(…).file_name()` | Yes (same file name) |
| Q8 → Q9 | `commands::now_in` made `pub(crate)`; `decide_in` | `week.rs` "today"; `answer_card_inner` | Yes |
| Q8 → Q11 | `answer_card(view, id, meets)` | `invoke("answer_card", { view: stateView(), id, meets })` | Yes |
| Q9 → Q10, Q11 | `your_week {}`, `preview_window {window}`, `commitments_confirm {view, confirm}`, `commitment_proposals {}` | the JS invokes, argument for argument | Yes |
| Q9 → Q10 | `preview_window` → `{ok, error, state}` with `state.moved.text` and `state.the_day.blocks[].takes[].title` (`surface.rs:874–888, 1843`) | `runPreview` | Yes |
| Q10 → Q11 | `DAYS`, `DAY_NAMES`, `levelButtons`, `windowEditorHtml`, `windowSequence`, `bindWindowEditor`, `showWindowError`, `confirmWeek`, `stateView` | `setupRow`, `openWeekSetup`, `finishWeekSetup`, `askTimeRow`, `answerAsk` | Yes |
| Q10 ↔ Q11 (static files) | each appends a CSS block and one `static_assets` test | the `data-id`/`data-kind` count equality (`static_assets.rs` settings and GTK tests): sched rows and `askRow` each stamp both; setup rows use `data-key` | Yes |
| Q10 → existing tests | `route` whitelist, twelve `view:` swaps, `renderNav` guard | no existing static test pins the old whitelist or `view: current.view` (checked `static_assets.rs`) | Yes |
| Q9, Q12 → CLAUDE.md, README | 47 / 29 / 66 and nine mutating commands | Q12 writes what the script prints | Yes |

## The plan's rulings, judged against the spec

All are sound, with these exceptions:

- **Q1-a** has the east-of-UTC hole (finding 1).
- **Q2-c** is applied in one place but not in its Q7 twin (finding 3).
- **Q11-a** departs from spec §4's "same component". It is acceptable with a spec line (finding 7).

Three more are worth a word:

- **Q4-a** (`--actor` beside `--via`) follows `write`'s house pattern (`main.rs:175–188`). It adds
  no new trust: `write --actor` already exists.
- **Q9-a** (holding `vault_io` for the child's run) is right. The child writes notes, and a sync
  must not rewrite the tree under it. It is also safe: `--confirm` fetches nothing, so the hold is
  short.
- **Q10-a** (`stateView()`) is the only way a nav view can exist without a read-model view name.
  It departs from anatomy §2's "panels not pages" rule on purpose (finding 8 records it).

## Rulings on the seven problems the plan flagged

1. **`emit_asks` lacks the proposals in the spec's signature.** *Amend the spec.*
   - Q6-a is right: "no pending class proposal" (parent §5.3) cannot be tested without them, and
     `rank` already holds them.
   - Change spec §5's line to `emit_asks(vault, proposals, today, budget, ctx, journal)`.
2. **An ask-created note has no title.** *Amend the parent spec (§5.3) and accept Q7-b.*
   - The title is the course note's title (`active_courses`: `title`, else `name`, else the slug).
   - The file is `commitments/<slugify(title)>.md`. For a wizard vault that is `<slug>.md`, because
     the scaffold titles a course by its code label (`app/src/scaffold.rs:699–705`).
   - §2.2's "a file name is never an identity" covers the rest.
3. **An answer's warning has no read-model carrier.** *Accept Q7-a/Q8-c and amend spec §5.*
   - New wording: "…the console shows the engine's warning on the card, from `answer_card`'s
     envelope, until the next repaint".
   - One correction to the plan's reasoning: adding an `Option` field to `Card` with
     `skip_serializing_if` would *not* change the surface-oracle bytes, since none of their vaults
     has an ask card.
   - Even so, a persistent carrier would need a second write onto the card (for example
     `answer_error:`) for a message the student has just seen. The envelope is enough.
4. **`schedule` is not a read-model view.** *Accept Q10-a and amend spec D6.*
   - Add: "the view polls the read model as `today`; `schedule` is a page view and never reaches
     `surface::View::parse`".
   - Q12 must also update anatomy §2 (finding 8).
5. **The deck cannot answer an ask.** *Accept Q11-b and amend spec §5 ("The form").*
   - Add: "In the rail's deck an ask shows **Answer…**, which opens Decisions; Reject and Snooze
     stay".
   - This is exactly parent §5.3's warning: "a card the console cannot answer would teach the
     student to reject".
6. **The P16 rank tests gain a decline marker.** *Accept, narrowed (finding 4).*
   - The edits are setup, never assertions: the seeded journal day (Q1-c), the moved
     `commitment_note` stamp (Q1-c; noon under finding 1), and the `card:cs-100` marker.
   - The marker is needed in exactly one test, `rank_withdraws_a_card_whose_series_left_the_file`.
   - The plan's stop-and-report rule stays.
7. **The UTC first day gives evening onboarders one extra day.** *Change the plan (finding 1) and
   amend spec D2 and parent §5.3.*
   - The west case is harmless, as the plan says. The east case is not: after local midnight east
     of UTC, the first local day is already day 2, so the screen never opens and day-1 cards are
     filed.
   - Counting from the vault-local date of the earliest journal record fixes both cases for about
     ten lines. The ledger seam and every existing test survive, provided `commitment_note` is
     stamped at noon.

## Before execution

- **Apply:** finding 1 (with its `commitment_note` stamp), finding 4 (name the one test) and
  finding 5 (move the import).
- **Take or leave:** findings 2, 3, 6 and 8. Each is a few lines and belongs in the task already
  named.
- **Name only:** findings 7, 9 and 10 need no plan change beyond the spec lines above.
- **Spec lines:** land them with Quinn's go at the checkpoint (problems 1–5 and 7), so the spec
  still binds the plan. They cover D2, D6, §4 (the editor), §5 (the signature, the warning, the
  deck) and parent §5.3 (the title and the first day).
