# M1 — Grades from Blackboard: the plan

**Spec:** `docs/specs/2026-09-29-grades-design.md`, **signed by Quinn, 2026-09-29** (rulings G1–G4,
the two §15 defaults, and the signing packet's edits 3–5). **Branch / worktree:** `m1-grades`,
`.claude/worktrees/m1-grades`. PR #18 (the vision) has merged into `main` (86a3431), and the branch
follows `main`. **Execution:** one task at a time, each committed on its own with its tests green,
reviewed before the next begins; a whole-branch review at the end.

## What the signature changed (2026-09-29)

- **Four commands, not five.** `grades_set_host` and the address prompt are gone (spec §4, §9). A
  school gains grades only through a curated campus row with a recorded read.
- **The policy-read gate is mechanical** (edit 5; cloud design ruling 12). `scaffold::Curated` gains
  `policy_read` (a `YYYY-MM-DD` date, absent until the school's read is done). One predicate in
  `app/src/grades.rs` decides availability, and four callers check it: `grades_status`,
  `grades_connect`, `grades_refresh` and the scheduler. The slot's new named skip is
  `grades (skipped: not available at your school yet)`. **The date-and-bump test** fails when any
  curated row carries a date while `PRIVACY_VERSION` is older than privacy bump #1.
- **UA's date lives only on a proof branch** until privacy bump #1. No task on `m1-grades` writes a
  `policy_read` date into any row. The founder's grades proof runs on a dev build from that proof
  branch (step P).
- **The privacy text stays in spec §11** (edit 3). The page, its Effective date, `PRIVACY_VERSION`
  and the sentence pins move later, in privacy bump #1's PR, which is not this stream's. Task 8 no
  longer touches `site/`, `app/src/account.rs`, `engine/tests/site.rs` or the privacy pins in
  `app/tests/static_assets.rs`.
- **Manual grade entry is out of M1** (edit 4, spec §12).
- **§15's defaults are signed:** the sign-in window closes on detection, and the breakdown's footer
  shows points so far beside an Overall Grade whenever both exist.

## The gate every task runs before it commits

```
cargo build --workspace ; cargo test --workspace      # 0 warnings other than the .rsrc line
.\scripts\ci\eol-check.ps1
# Task 1 and anything under cloud/, exactly as ci.yml runs them:
deno check --config cloud/supabase/deno.json cloud/supabase/functions/**/*.ts cloud/eval/*.ts
deno lint --config cloud/supabase/deno.json cloud/supabase/ cloud/eval/
deno test --allow-read --allow-write=cloud/eval --allow-net=127.0.0.1 --allow-env=ANTHROPIC_WEBHOOK_SIGNING_KEY,ANTHROPIC_AUTH_TOKEN,ANTHROPIC_LOG,ANTHROPIC_CUSTOM_HEADERS --config cloud/supabase/deno.json cloud/supabase/ cloud/eval/
```

## Rules every task carries

- CLAUDE.md's two rules; LF line endings (`*.ps1` CRLF); TDD: the failing test first, shown failing,
  then the code.
- Nothing under `engine/tests/fixtures/` is edited or regenerated except **new** files under
  `engine/tests/fixtures/grades/`, and **each new fixture file is its own `contract-engineer` step**
  (reviewed by `contract-reviewer`), never part of an `implementer` task. No real grades, names or
  ids in any fixture or test.
- **No task on this branch adds a `policy_read` date to any curated row.** Tests exercise dated rows
  through rows they build themselves, never by editing `scaffold::CAMPUSES`.
- **No build skips the predicate.** No `cfg(test)`, `cfg(debug_assertions)`, feature flag or
  environment variable bypasses it.
- The spec is cited by section heading, never by line number; each dispatch names the task by its
  heading here.

## Roster and order

`implementer` is Sonnet at `high`. T3 (vault writes) and T5 (the kept session) **start on Opus**:
the controller dispatches `implementer` with `model: 'opus'` from the first attempt rather than
escalating after two failures, because a silent error in either corrupts vault bytes or leaks a
student's session (CLAUDE.md, "No agent fits?", first question).

| Step | Agent | Why |
|---|---|---|
| T1 `grades/` becomes a note folder | `contract-engineer` | `ids.rs`, `backup.rs`, the sync contract test: the contract list. **Done** before signing; findings carried under "Carried findings". |
| T2F `bundle-basic.json` | `contract-engineer` | a new file under `engine/tests/fixtures/` |
| T2 the engine's grades core | `implementer` (Sonnet, high) | pure functions, fully specified, checked by tests |
| T3F `bundle-changed.json` | `contract-engineer` | a new fixture file |
| T3 the `grades` command | `implementer`, **Opus from the start** | writes the vault through `write`; judge-once filtering |
| T3C the gated-command pins | `contract-engineer` | `engine/tests/entitlement_gate.rs` and `entitle.rs` are on the contract list |
| T4 the read model | `implementer` (Sonnet, high) | pure read-model code; `surface_oracle.rs` must pass untouched |
| T5 the app's session, capture, gate and commands | `implementer`, **Opus from the start** | the kept Blackboard session (ruling 12's exception) and the gate |
| T6 the slot | `implementer` (Sonnet, high) | scheduler glue; the gate's fourth caller, decided in T5 |
| T7 the console | `console-ui` (Sonnet, medium) | `app/static` only |
| T8 docs | `docs-keeper` (Sonnet, medium); the main session for CLAUDE.md's two lines | HANDOFF and `docs/reference/`; CLAUDE.md fits on one screen |
| T9 whole-branch review and the PR | `reviewer` (Opus, high), `contract-reviewer` (Opus, xhigh) on the contract-list diffs; the main session opens the PR | pre-push review; a code push asks Quinn first |
| P the proof branch and live proof | the main session | UA's date is Quinn's to record; the proof is the controller's |

Order: T1 → T2F → T2 → T3F → T3 → T3C → T4 → T5 → T6 → T7 → T8 → T9 → P. T4 and T5 touch disjoint
files and may run in parallel only when the controller grants two build slots. Each step appends
its ledger line and is reviewed before the next begins.

## Task 1 — `grades/` becomes a note folder (`contract-engineer`; done)

**Files:** `engine/src/ids.rs`, `engine/src/backup.rs`, `engine/src/sync.rs` (comments only, if the
count is named), `engine/tests/sync_contract.rs`, `cloud/supabase/functions/_shared/sync_rows.ts`,
`cloud/supabase/functions/_shared/sync_rows_test.ts`, new
`cloud/supabase/migrations/20260929000100_sync_note_path_check_grades.sql`,
`cloud/supabase/migrations_sync_test.ts`.

1. Tests first:
   - `ids`: `NOTE_FOLDERS` contains `grades`; `is_id("grade_0123456789")`; `kind_for` maps
     `type: grade` to `grade` in any folder; `derived_id("grade", "grades/x.md")` has the `grade_` prefix.
   - `backup`: `grades` is in `BACKUP_FOLDERS` (the existing subset test covers it once the array grows).
   - `sync_contract::is_note_path_and_the_servers_regex_agree` rewritten **from `NOTE_FOLDERS`**:
     every folder in `NOTE_FOLDERS` appears in `sync_rows.ts`'s `NOTE_PATH_RE` group **and** in the
     group of the lexically latest `*sync_note_path_check*.sql`; `grades/a.md` is a note path;
     `grades/../tasks/a.md` is not. **Subset, not equality**, so the server may also carry
     `commitments` (spec §7).
   - Deno: `sync_rows_test.ts` accepts `grades/x.md` and `commitments/x.md`; `migrations_sync_test.ts`
     finds the new migration as the latest path check and that it names every folder.
2. Code: `NOTE_FOLDERS` gains `"grades"` as its seventh entry, **last** (spec §7's merge order with
   `p1-commitments`: after both merge it reads `…, "info", "commitments", "grades"`); `KINDS` gains `"grade"`; `ID_RE`
   accepts `grade`; `kind_for` returns `"grade"` for `type: grade`; `BACKUP_FOLDERS` gains index 6.
   Every `[&str; 6]` and every "six note folders" comment is updated. `NOTE_PATH_RE` becomes
   `(tasks|approvals|archive|courses|issues|info|commitments|grades)`. The migration drops and
   re-adds `sync_notes_path_check` with the same group and the same `char_length` bound as
   `20260912000400`, forward-only, and says in its header why `commitments` is there.
3. **Accept:** workspace green; Deno green; `git grep -n "\[&str; 6\]" engine/src` shows no note-folder
   array left at six.

## Task 2F — the basic bundle fixture (`contract-engineer`)

**Files:** new `engine/tests/fixtures/grades/bundle-basic.json` only.

A synthetic bundle in spec §5's shape (version 1): two courses whose `courseId`s are invented, with
columns covering every Task 2 status (a numeric score, `NeedsGrading`, `InProgress`, no row,
`exempt: true`), one `externalGrade: true` column with a score, one column with no
`score.possible`, one with `includeInCalculations: false`, one category list and one `null`
(refused) category list, and one course carrying `"error"`. No real host, name, id or grade. It is
written byte-for-byte as JSON with LF endings, and nothing reads it yet; the workspace stays green.
**Accept:** `contract-reviewer` confirms the file is new, synthetic, and the only file changed.

## Task 2 — the engine's grades core (`implementer`, Sonnet, high)

**Files:** new `engine/src/grades.rs`, `engine/src/lib.rs` (`pub mod grades;`). Reads T2F's
fixture and never writes it.

1. Tests first, in `grades.rs`:
   - `Bundle` deserializes spec §5's shape from `bundle-basic.json`; rows stay `serde_json::Value`,
     verbatim.
   - `match_course(bundle_course, course_notes)`: by `membership.courseId`, then `course.courseId`,
     then `course.id`, against the notes' `code:`; no match → `None`.
   - `column_to_fields(column, row, zone)`: every spec §6 field; `status` for a numeric score
     (`graded`), `NeedsGrading` (`needs-grading`), `InProgress` (`in-progress`), no row
     (`not-submitted`), `exempt: true` (`exempt`); `kind: overall` for `externalGrade: true`;
     `due` converted into the vault's zone; `possible` absent when the column has none.
   - `note_path(slug, column_id)`: `_4686399_1` → `grades/<slug>-4686399-1.md`; a hostile id
     (`../x`, spaces, 300 characters) is reduced to `[a-z0-9-]` and length-bounded.
   - `letter(pct)`: every G3 boundary on both sides (92.999 → A−, 93 → A, 59.999 → F, 100+ → A).
   - `course_grade(notes)`: basis `overall` / `points` / `none` exactly as spec §8; an `exempt`,
     `counts: false`, `removed` or `possible: 0` item never enters points; **a course on basis
     `overall` still carries `earned` and `possible`** (spec §8, §15 default 2).
2. Code: the pure functions above and their types. No I/O in this task.
3. **Accept:** workspace green; the module has no `std::fs` and no network.

## Task 3F — the changed bundle fixture (`contract-engineer`)

**Files:** new `engine/tests/fixtures/grades/bundle-changed.json` only.

`bundle-basic.json` with exactly three differences: one score posted on a column that had no row,
one column's `name` changed, and one column gone. Synthetic, LF, nothing else changed; the commit
message names the three columns that differ and how.
**Accept:** `contract-reviewer` confirms the three differences and nothing else.

## Task 3 — the `grades` command (`implementer`, Opus from the start; writes through `write`)

**Files:** `engine/src/grades.rs` (the apply half), `engine/src/main.rs` (`Command::Grades`, and
`gated_vault` / `name_of`, and its "gated four" comment), new `engine/tests/grades.rs`,
`docs/reference/engine-commands.md`.

1. Tests first (`engine/tests/grades.rs`, each on a temp copy of `vault-s1` with a synthetic course
   note whose `code:` matches the fixture):
   - first run creates one note per column with the deterministic id, `type: grade`, the fields of
     Task 2, actor `agent:knowlu.grades`, and a journal record per note;
   - **a second identical run writes no journal record and changes no byte**;
   - `bundle-changed.json` writes only the three changed fields, and the gone column gets
     `status: removed`;
   - every created note's `id:` equals `ids::derived_id("grade", <its path>)` (a silent re-mint by
     `write::create` fails this);
   - a student-set field, written with a `console_ctx()`-style actor (what `Journal::human_set`
     recognises), is never re-set: `apply` filters by `human_set` before writing. If ruling 11's
     neutral token (`student`) has merged by then, the test covers both `quinn` and `student`;
   - past the entitlement grace the command prints `grades (skipped: …)` at exit 0 and writes
     nothing, tested here in `engine/tests/grades.rs` with `entitlement_gate.rs`'s pattern (T3C adds
     `grades` to that file's own lists);
   - an unmatched course and a course with `"error"` are named in stdout and change nothing;
   - `state/grades.json` holds `fetched_at`, the host and the per-course counts, through
     `ledger::dumps_value`;
   - a missing or unparseable `--input` exits 1; every other outcome exits 0.
2. Code (Task 1 finding: `pystr::write_text` makes no parent directories, so `apply` creates
   `grades/` itself before the first create): `grades::apply(vault, &bundle, &ctx, &mut journal) ->
   Report`, using `write::create` for new notes and `write::write` with only the changed fields;
   `Command::Grades { vault, input, via, run_id }` in `main.rs`, added to `gated_vault` and
   `name_of`; the run-log line through `cli::append_run_log`.
3. **Accept:** workspace green; `docs/reference/engine-commands.md` documents the command beside
   `coursework`. (CLAUDE.md's gated list is T8's.)

## Task 3C — the gated-command pins (`contract-engineer`)

**Files:** `engine/tests/entitlement_gate.rs`, `engine/src/entitle.rs` (module doc only).

After T3 lands, so the workspace is never red between commits: the two lists of gated commands in
`entitlement_gate.rs` gain `grades` (the spawn test passes `--input` with a path under the temp
vault's parent, never the vault), and `entitle.rs`'s module doc names five gated commands. The test
passes on arrival; the task shows it would fail by running it once against a build with `grades`
removed from `gated_vault`, and says so in its report. **Accept:** workspace green; `contract-reviewer`
confirms no behavioural change to `entitle.rs`.

## Task 4 — the read model (`implementer`, Sonnet, high)

**Files:** `engine/src/surface.rs`, `docs/surface/anatomy.md`.

1. Tests first: a temp vault with synthetic `grades/` notes → `build_state(.., View::Today, ..)`
   carries `grades` with the spec §8 fields, courses in course-title order, entries graded-first by
   due; a course with an overall note carries `basis: overall` **and** its points-so-far `earned`
   and `possible`; a vault with no `grades/` folder serializes **without** a `grades` key (so the
   three surface references stay byte-identical: `surface_oracle.rs` must pass untouched);
   `fetched_at` comes from `state/grades.json`.
2. Code: `State` gains `#[serde(skip_serializing_if = "Vec::is_empty")] pub grades: Vec<CourseGrade>`,
   built by `grades::course_grades(vault)` (Task 2's pure `course_grade` over the loaded notes).
3. **Accept:** workspace green with `surface_oracle.rs` and every `surface-today-*.json` unchanged;
   anatomy.md describes the strip-and-drawer block and drops the stale "Grades page".

## Task 5 — the app's session, capture, gate and commands (`implementer`, Opus from the start)

**Files:** new `app/src/grades.rs`, `app/src/lib.rs`, `app/src/main.rs` (the console window's
`generate_handler!` list), `app/src/scaffold.rs` (`Curated.policy_read`; `build_into` gains
`grades`, the Task 1 finding), new `app/tests/grades.rs`, `docs/reference/app.md` (the command
count only).

1. Tests first (no network, no window):
   - **`Curated.policy_read`:** both `CAMPUSES` rows carry no date on this branch (the field is
     absent in the sense of `None`, not an empty string); a new vault's `build_into` creates
     `grades/`.
   - **The predicate.** It takes the curated row (or none) and the vault's `config/campus.yaml`
     `lms`, and answers one of three: *available* (with the row's `lms_host`), *not a Blackboard
     school*, or *not available yet*. Available only for a curated row whose `lms_kind` is
     `blackboard` and which carries `policy_read`. Cases, each on a row the test builds: a dated
     Blackboard row → available with its host; an undated Blackboard row → not available yet; an
     uncurated school whose `lms` is `blackboard` → not available yet; a curated Canvas row → not a
     Blackboard school; no `unitid` → not a Blackboard school.
   - **Three of the four callers**, each through a seam that takes the curated row as input (the
     Tauri command is a thin wrapper that resolves the row with `scaffold::curated` and calls the
     seam), for a dated row, an undated row and an uncurated school:
     - `grades_status` reports available or not, and with a dated row the connected, signed-out,
       host and last-fetched fields; with no date it reports *not available* whatever session
       directory exists.
     - `grades_connect` and `grades_refresh` refuse with exactly `not available at your school yet`;
       the seam takes the window-opening and session-reading actions as injected closures, and the
       test asserts **neither is called** on a refusal. On a dated row both proceed.
     - `grades_forget` is not gated: it deletes the session directory whatever the predicate says.
   - **The date-and-bump test** (spec §4). A pure check, in the test file, that takes the curated
     rows and a privacy version and fails when any row carries a `policy_read` date while the
     version is at or before `2026-09-24`, the `PRIVACY_VERSION` in force at signing (every later
     version is privacy bump #1 or after it, because `account.rs:21-23` moves the constant only with
     the page). Shown failing on a synthetic dated row with `2026-09-24`; passing with a later
     version, and with an undated row. Then pinned: the check passes on the real
     `scaffold::CAMPUSES` and `account::PRIVACY_VERSION`. Any present date must parse as
     `YYYY-MM-DD`.
   - `GradesPrefs` holds `hidden` only and round-trips `grades.json` through `ledger::dumps_value`; a
     missing or corrupt file is `hidden: false` and never touches `settings.json`.
   - `assemble_bundle(user, memberships, per_course)` from canned JSON produces spec §5's shape,
     merges pages, skips unavailable memberships, and records a failed course's `error`.
   - `next_page(host, body)` follows `paging.nextPage` only on the same host and stops at 20 pages.
   - Sign-in detection (§15 default 1): the detection check says signed in only when the window's
     URL is on the LMS host **and** `users/me` answered 200; either alone is not.
   - `session_dir(data_dir)` is `<data_dir>\lms-session` and `is_session_dir` refuses anything else;
     `forget` deletes only that directory.
   - the capability file `app/capabilities/default.json` never names `lms-grades` (static read).
2. Code: the predicate, the seams and the pure functions; then the window half (`open_visible`,
   which closes the window on detection; `refresh_hidden`, the 45-second settle, the cookie handover
   exactly as `lms_link::capture_courses`, one `ureq` agent with a 30-second timeout); the **four**
   commands (`grades_status`, `grades_connect`, `grades_refresh`, `grades_forget`), each
   `#[tauri::command(async)]` (a synchronous command that builds and waits on a window blocks the
   main thread on Windows). `grades_connect` and `grades_refresh` check the predicate **before**
   anything else. `grades_refresh` runs the capture and then the engine's `grades` step as a child
   process with `--via dashboard`, holding no vault lock across it. No address prompt and no
   `grades_set_host`.
3. **Accept:** workspace green; `docs/reference/app.md`'s command counts **recounted by script** over
   both `generate_handler!` lists (the console list gains four) and corrected in the same commit;
   `git grep -n grades_set_host -- app docs/reference` is empty; `git grep -n "policy_read: Some"
   -- app/src` is empty.

## Task 6 — the slot (`implementer`, Sonnet, high)

**Files:** `app/src/scheduler.rs`, `app/tests/scheduler.rs`.

1. Tests first:
   - `slot_argv` places `grades --vault v --input <file> --via local-runner` after `ingest` and before
     `judge` when a bundle path is given, and leaves it out otherwise;
   - **the gate's fourth caller:** the scheduler's grades decision takes the curated row as input and
     calls T5's predicate (it never re-derives availability). With a saved session present in every
     case: a dated row captures; an undated row and an uncurated school record
     `grades (skipped: not available at your school yet)` and the injected capture action is never
     called; a Canvas row records `grades (skipped: not a Blackboard school)`;
   - **the skip order** (spec §10 fixes the first three relative to each other; the rest is this
     plan's, see open question 1): `not a Blackboard school` → `not available at your school yet`
     → `no entitlement` → `not connected` → `sign-in window open`; then the capture's own outcomes,
     `signed out` and `Blackboard unreachable`. A test per adjacent pair shows the earlier wins when
     both hold;
   - each skip is recorded as a named step at exit 0 and reaches `state/runner-log.md` with status
     `ok`, through the same filter as the ingest and judge skips;
   - the bundle file is deleted after the step whatever its exit code.
2. Code: `run_slot_inner` decides the grades step after the entitlement step through the decision
   above, captures only when it says so and `tray_app` is `Some`, and otherwise names the skip.
3. **Accept:** workspace green. (CLAUDE.md's slot line and `docs/reference/app.md`'s slot text are
   T8's.)

## Task 7 — the console (`console-ui`, Sonnet, medium)

**Files:** `app/static/console.js`, `app/static/console.css`, `app/static/index.html`,
`app/tests/static_assets.rs` (grades tests only; the privacy pins there are bump #1's).

1. Tests first (static assets): the strip renders from `state.grades`; the **not-available**,
   not-connected, signed-out and hidden states exist; the not-available state is the one line
   *Grades from Blackboard are not available at your school yet* and renders **no button**; the
   console decides that state only from `grades_status` (it never reads `campus.yaml` or a campus
   list); `console.js` never names `grades_set_host` and has no address input; the ring is SVG; the
   five `--grade-*` tokens exist in both themes; the breakdown drawer is read-only (no `set_fields`
   call from it); the footer shows points so far beside *Overall Grade* when a course carries both.
2. Code: spec §2 and §9: the strip on Today, the rings, the drawer, Refresh, Connect, Sign in again,
   Forget and Hide in settings. No computation: every number comes from `state.grades` or
   `grades_status`.
3. **Accept:** workspace green; `python scripts/console-shots.py` against a scratch vault with a
   synthetic `grades/` folder produces the strip in light and dark, and the not-available line,
   attached to the task's report (window-handle capture only).

## Task 8 — docs (`docs-keeper`; the main session for CLAUDE.md)

**Files:** `docs/reference/app.md` (the grades section, the gate, the four commands and the slot
text; the count is T5's), `HANDOFF.md`; the main session edits CLAUDE.md's gated-command list and
slot line (two lines). **Not** `site/privacy.html`, `app/src/account.rs`, `engine/tests/site.rs` or
the privacy pins in `app/tests/static_assets.rs`: those move in privacy bump #1 (spec §11).

1. Content: `app.md` documents the predicate and its four callers, the named refusal and the skip;
   CLAUDE.md's gated list names `grades` and its slot line reads
   `sync → coursework → ingest → grades → judge → rank`; HANDOFF's stream table and §4 row say what
   privacy bump #1 owes from M1 (spec §11's two sentences on the page, its Effective date,
   `PRIVACY_VERSION`, the sentence pins, the lawyer-packet delta; the Pilot's re-consent screen,
   R-PS-4, still owed before a second account), that UA's `policy_read` date reaches `main` only in
   bump #1's PR or after it, and that the `20260929000100` migration is applied before any release
   that writes `grades/`.
2. **Accept:** workspace green; `git diff <branch base> -- site/ app/src/account.rs
   engine/tests/site.rs` is empty over the whole branch.

## Carried findings (from Task 1)

`app/src/scaffold.rs::build_into` lists the folders a new vault is born with; Task 5 adds `grades`
there (with its test). `app/src/report.rs::vault_shape` counts folders for the issue report and its
payload may be a server contract, so it is **left alone** in this stream and named in the PR.
**Rollout:** the `20260929000100` migration must be applied before any release that writes `grades/`
ships: the server refuses a whole push if one path is refused. **Merging with `p1-commitments`:**
besides text conflicts in `ids.rs`, `backup.rs`, `sync_rows.ts`, `sync_rows_test.ts` and the
agreement test, two of p1's tests pin facts this branch changes: its `migrations_sync_test.ts`
expects its own `20260926000100` to be the newest path check, and
`backup_folders_has_ten_entries_with_commitments_before_state` expects ten entries. The second
merger updates both: the newest check is `20260929000100`, and the backup array has eleven entries
(`…info, commitments, grades, state, config, profile`).

## Task 9 — whole-branch review and the PR (`reviewer`; `contract-reviewer`; the main session)

`reviewer` reviews the whole diff against the signed spec, CLAUDE.md's two rules and VISION;
`contract-reviewer` reviews the contract-list diffs (T1, T2F, T3F, T3C). The review checks, beyond
the tests: no `grades_set_host` and no address entry anywhere; no `policy_read` date in any row;
`site/`, `account.rs` and `site.rs` untouched; the predicate has exactly four callers and nothing
else calls `scaffold::curated` to decide grades; no build path skips it; no manual-entry surface or
write path for grades beyond the agent's `apply` (spec §12). Fix waves until it reads ready, and the
report lands in `docs/reports/`. Then the main session asks Quinn before pushing, and opens the PR
with the hand-offs named: the `NOTE_FOLDERS` merge with `p1-commitments`; whichever migration is
applied last must carry the union; and **what privacy bump #1 owes from M1** (spec §14).

## Step P — the proof branch and the founder's live proof (the main session)

Waits on **Quinn recording UA's university-policy read** (cloud design ruling 10: an MVP exit
condition). Then:

1. Cut a proof branch from `m1-grades`'s reviewed head (or `main` after the PR merges). One commit
   sets UA's row's `policy_read` to the date Quinn gives. The date-and-bump test **fails on this
   branch by design**: that failure is the tripwire that keeps it from merging ahead of bump #1.
2. Build the dev app from it (`cargo build`, never a release build) and run spec §13's live proof on
   the founder's scratch profile through the real gate: connect, rings, a silent slot refresh,
   forget, *not connected*; and a scratch profile at a school without a dated row shows *not
   available*. The existing live-proof practice holds: DOM only, no OS input, the scratch profile
   removed afterwards; the first real bundle is never committed.
3. The branch is never merged ahead of privacy bump #1. UA's date reaches `main` in bump #1's PR or
   after it. MVP exit needs the read recorded and this proof passed, not the date on `main`.

## Fidelity ledger

| Spec | Task | Proven by |
|---|---|---|
| G1 persistent session, hidden refresh, forget | 5, 6, P | `app/tests/grades.rs` (session dir, forget); live proof |
| G1 / ruling 12 gate: `policy_read`, one predicate, four callers, named refusal | 5, 6, 7 | the predicate cases and the three-case matrix per caller (`app/tests/grades.rs`, `app/tests/scheduler.rs`); injected-closure tests (no window, no session on refusal); static not-available test |
| No build skips the predicate | 5, 9 | review: no `cfg`, feature or environment bypass; step P runs through the real gate |
| Date-and-bump rule | 5, P | date-and-bump test (shown failing on a synthetic row, pinned on `CAMPUSES`); proof branch unmerged until bump #1 |
| G2 overall else points | 2, 4 | `course_grade` tests; surface test |
| G3 letter scale | 2 | `letter` boundary tests |
| §2 strip states, incl. not available with no button | 7 | static-asset tests; console shots |
| §3 endpoints, paging | 5 | `assemble_bundle`, `next_page` tests |
| §3 `grades.json` holds `hidden` only | 5 | `GradesPrefs` test |
| §4 host from the curated row only; no address entry | 5, 7, 9 | predicate tests; `grades_set_host` grep empty; static test (no address input) |
| §5 bundle verbatim, in app data, deleted | 2F, 5, 6 | bundle tests; scheduler deletion test |
| §6 gated, deterministic ids, change-only, judge-once, state file, exit codes | 3F, 3, 3C | `engine/tests/grades.rs`; `entitlement_gate.rs` |
| §7 folder, ids, backup, sync both sides | 1 | ids/backup/sync_contract tests; Deno tests |
| §8 read model, both bases' sums, references unchanged | 4 | surface test; `surface_oracle.rs` untouched |
| §9 four commands, recount | 5, 7 | the recount script; grep; static-asset tests |
| §10 slot order, named skips incl. not available, their order | 6 | scheduler tests |
| §11 edit 3: sentences drafted in the spec only; page, date and version untouched | 8, 9 | empty branch diff on `site/`, `account.rs`, `site.rs`; bump #1's debt named in HANDOFF and the PR |
| §12 edit 4: manual entry out | 9 | review: no editing surface or write path for grades |
| §13 tests; live proof through the real gate on the proof branch | 1–7, P | the tests above; step P |
| §15 default 1: window closes on detection | 5, P | detection test; live proof |
| §15 default 2: both bases shown | 2, 4, 7 | `course_grade` and surface tests; static footer test |

## Open questions for Quinn (at the first checkpoint; none block T2F–T5)

1. **The skip order beyond what §10 fixes.** The spec fixes `not a Blackboard school` → `not
   available at your school yet` → `not connected`. *Recommend* placing `no entitlement` after the
   two school checks, so a lapsed student at a school without grades sees why grades never run, and
   `sign-in window open` after `not connected`. Cost if wrong: one reordered test in T6.
2. **The headless `--run-slot-once` path has no app handle,** so it cannot capture, and §10's list
   has no name for that. *Recommend* a named skip `grades (skipped: no window on this run)`, added
   to spec §10 by a one-line amendment at T6's checkpoint. The alternative is leaving the step out
   silently, which CLAUDE.md's slot rule forbids.
3. **The proof branch stays local.** *Recommend* never pushing it: its date-and-bump test is red by
   design, and a local branch cannot be merged by accident. The alternative, pushing it for a
   record, spends a CI run on a known failure.
4. **The date-and-bump test compares versions, not page text.** It treats any `PRIVACY_VERSION`
   after `2026-09-24` as bump #1 or later, which holds because `account.rs:21-23` moves the constant
   only with the page. An unrelated page fix before bump #1 would satisfy it early. *Recommend*
   version-only, as signed; the alternative, also requiring §11's sentence on the page, ties the
   test to wording the lawyer read may change.
5. **A Canvas school sees the signed line** *Grades from Blackboard are not available at your school
   yet*. *Recommend* keeping it for M1 (it is the signed text and true); hiding the strip at
   non-Blackboard schools is a one-line console change if Quinn prefers it.
