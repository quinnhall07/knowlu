# M1 — Grades from Blackboard: the plan

**Spec:** `docs/specs/2026-09-29-grades-design.md` (rulings G1–G4). **Branch / worktree:** `m1-grades`,
`.claude/worktrees/m1-grades`, based on PR #18's branch (the vision) so `VISION.md` is present; it
rebases onto `main` once PR #18 merges. **Execution:** one task at a time, each committed on its own
with its tests green, reviewed before the next begins; a whole-branch review at the end.

Every task runs the gate before it commits:

```
cargo build --workspace ; cargo test --workspace      # 0 warnings other than the .rsrc line
.\scripts\ci\eol-check.ps1
# Task 1 and anything under cloud/, exactly as ci.yml runs them:
deno check --config cloud/supabase/deno.json cloud/supabase/functions/**/*.ts cloud/eval/*.ts
deno lint --config cloud/supabase/deno.json cloud/supabase/ cloud/eval/
deno test --allow-read --allow-write=cloud/eval --allow-net=127.0.0.1 --allow-env=ANTHROPIC_WEBHOOK_SIGNING_KEY,ANTHROPIC_AUTH_TOKEN,ANTHROPIC_LOG,ANTHROPIC_CUSTOM_HEADERS --config cloud/supabase/deno.json cloud/supabase/ cloud/eval/
```

Rules every task carries: CLAUDE.md's two rules; LF line endings (`*.ps1` CRLF); nothing under
`engine/tests/fixtures/` is edited or regenerated except **new** files under
`engine/tests/fixtures/grades/`; no real grades, names or ids in any fixture or test; TDD — the failing
test first, shown failing, then the code.

## Task 1 — `grades/` becomes a note folder (contract list: Opus, xhigh)

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

## Task 2 — the engine's grades core (Opus, high)

**Files:** new `engine/src/grades.rs`, `engine/src/lib.rs` (`pub mod grades;`), new
`engine/tests/fixtures/grades/bundle-basic.json` (synthetic).

1. Tests first, in `grades.rs`:
   - `Bundle` deserializes spec §5's shape; rows stay `serde_json::Value`, verbatim.
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
     `counts: false`, `removed` or `possible: 0` item never enters points.
2. Code: the pure functions above and their types. No I/O in this task.
3. **Accept:** workspace green; the module has no `std::fs` and no network.

## Task 3 — the `grades` command (Opus, high; writes through `write`)

**Files:** `engine/src/grades.rs` (the apply half), `engine/src/main.rs` (`Command::Grades`, and
`gated_vault` / `name_of`), new `engine/tests/grades.rs`, new
`engine/tests/fixtures/grades/bundle-changed.json`, `docs/reference/engine-commands.md`.

1. Tests first (`engine/tests/grades.rs`, each on a temp copy of `vault-s1` with a synthetic course
   note whose `code:` matches the fixture):
   - first run creates one note per column with the deterministic id, `type: grade`, the fields of
     Task 2, actor `agent:knowlu.grades`, and a journal record per note;
   - **a second identical run writes no journal record and changes no byte**;
   - `bundle-changed.json` (a score posted, a title changed, a column gone) writes only those fields,
     and the gone column gets `status: removed`;
   - every created note's `id:` equals `ids::derived_id("grade", <its path>)` (a silent re-mint by
     `write::create` fails this);
   - a student-set field — written with a `console_ctx()`-style actor, which is what
     `Journal::human_set` recognises — is never re-set: `apply` filters by `human_set` before writing;
   - the gate test arrays in `engine/tests/entitlement_gate.rs` (the two lists of gated commands)
     gain `grades` (the spawn test passes `--input`), and `main.rs`'s "gated four" comment and
     `entitle.rs`'s module doc name five;
   - an unmatched course and a course with `"error"` are named in stdout and change nothing;
   - `state/grades.json` holds `fetched_at`, the host and the per-course counts, through
     `ledger::dumps_value`;
   - a missing or unparseable `--input` exits 1; every other outcome exits 0;
   - past the entitlement grace the command prints `grades (skipped: …)` at exit 0 and writes nothing
     (`entitlement_gate.rs`'s pattern).
2. Code (Task 1 finding: `pystr::write_text` makes no parent directories, so `apply` creates
   `grades/` itself before the first create): `grades::apply(vault, &bundle, &ctx, &mut journal) -> Report` using `write::create` for new
   notes and `write::write` with only the changed fields; `Command::Grades { vault, input, via,
   run_id }` in `main.rs`, added to `gated_vault` and `name_of`; the run-log line through
   `cli::append_run_log`.
3. **Accept:** workspace green; `docs/reference/engine-commands.md` documents the command beside
   `coursework`, and CLAUDE.md's gated list names `grades`.

## Task 4 — the read model (Opus, high)

**Files:** `engine/src/surface.rs`, `docs/surface/anatomy.md`.

1. Tests first: a temp vault with synthetic `grades/` notes → `build_state(.., View::Today, ..)`
   carries `grades` with the spec §8 fields, courses in course-title order, entries graded-first by
   due; a vault with no `grades/` folder serializes **without** a `grades` key (so the three surface
   references stay byte-identical — `surface_oracle.rs` must pass untouched); `fetched_at` comes from
   `state/grades.json`.
2. Code: `State` gains `#[serde(skip_serializing_if = "Vec::is_empty")] pub grades: Vec<CourseGrade>`,
   built by `grades::course_grades(vault)` (Task 2's pure `course_grade` over the loaded notes).
3. **Accept:** workspace green with `surface_oracle.rs` unchanged; anatomy.md describes the block and
   drops the stale "Grades page".

## Task 5 — the app's session and capture (Opus, high)

**Files:** new `app/src/grades.rs`, `app/src/lib.rs`, `app/src/main.rs` (the console window's
`generate_handler!` list), new `app/tests/grades.rs`.

1. Tests first (no network, no window):
   - `lms_host_for(campus_yaml, grades_json)`: curated `unitid` → its `lms_host`; uncurated Blackboard
     with `lms_host` in `grades.json` → that; a Canvas school → `None`; nothing → `None`.
   - `GradesPrefs` round-trips `grades.json` through `ledger::dumps_value`; a missing or corrupt file
     is defaults (`hidden: false`, no host) and never touches `settings.json`.
   - `assemble_bundle(user, memberships, per_course)` from canned JSON produces spec §5's shape,
     merges pages, skips unavailable memberships, and records a failed course's `error`.
   - `next_page(host, body)` follows `paging.nextPage` only on the same host and stops at 20 pages.
   - `session_dir(data_dir)` is `<data_dir>\lms-session` and `is_session_dir` refuses anything else;
     `forget` deletes only that directory.
   - the capability file `app/capabilities/default.json` never names `lms-grades` (static read).
2. Code: the pure functions; then the window half (`open_visible`, `refresh_hidden`, the 45-second
   settle, the cookie handover exactly as `lms_link::capture_courses`, one `ureq` agent with a
   30-second timeout); the five commands (`grades_status`, `grades_connect`, `grades_refresh`,
   `grades_forget`, `grades_set_host`), each `#[tauri::command(async)]` — a synchronous command that
   builds and waits on a window blocks the main thread on Windows. `grades_refresh` runs the capture and then the engine's
   `grades` step as a child process with `--via dashboard`, holding no vault lock across it.
3. **Accept:** workspace green; `docs/reference/app.md`'s command counts **recounted by script** over
   both `generate_handler!` lists and corrected in the same commit.

## Task 6 — the slot (Opus, high)

**Files:** `app/src/scheduler.rs`, `app/tests/scheduler.rs`.

1. Tests first:
   - `slot_argv` places `grades --vault v --input <file> --via local-runner` after `ingest` and before
     `judge` when a bundle path is given, and leaves it out otherwise;
   - each spec §10 skip is recorded as a named step at exit 0 and reaches `state/runner-log.md` with
     status `ok`, through the same filter as the ingest and judge skips;
   - the bundle file is deleted after the step whatever its exit code.
2. Code: `run_slot_inner` captures after the entitlement step when `tray_app` is `Some`, the vault is
   Blackboard, entitlement allows, and a session directory exists; otherwise it names the skip.
3. **Accept:** workspace green; CLAUDE.md's slot line and `docs/reference/app.md` name the grades step.

## Task 7 — the console (Sonnet, medium; `console-ui`)

**Files:** `app/static/console.js`, `app/static/console.css`, `app/static/index.html`,
`app/tests/static_assets.rs`.

1. Tests first (static assets): the strip renders from `state.grades`; the not-connected, signed-out
   and hidden states exist; the ring is SVG; the five `--grade-*` tokens exist in both themes; the
   breakdown drawer is read-only (no `set_fields` call from it).
2. Code: spec §2 and §9 — the strip on Today, the rings, the drawer, Refresh, Connect, Sign in again,
   Forget and Hide in settings. No computation: every number comes from `state.grades` or
   `grades_status`.
3. **Accept:** workspace green; `python scripts/console-shots.py` against a scratch vault with a
   synthetic `grades/` folder produces the strip in light and dark, attached to the task's report
   (window-handle capture only).

## Task 8 — privacy and docs (Sonnet, medium, then reviewed by Opus)

**Files:** `site/privacy.html`, `app/src/account.rs` (`PRIVACY_VERSION`), `engine/tests/site.rs`,
`app/tests/static_assets.rs`, `docs/reference/app.md`, `HANDOFF.md`.

1. Tests first: the pinned sentences of spec §11 are present; `PRIVACY_VERSION` equals the page's
   version string.
2. Code: the two sentences, the version to `2026-09-29`, HANDOFF's stream table and §4 row (the
   lawyer-packet delta; C4's re-consent screen still owed before a second account).
3. **Accept:** workspace green.

**Task 1 findings carried here.** `app/src/scaffold.rs::build_into` lists the folders a new vault is
born with; Task 5 adds `grades` there (with its test). `app/src/report.rs::vault_shape` counts
folders for the issue report and its payload may be a server contract, so it is **left alone** in this
stream and named in the PR. **Rollout:** the `20260929000100` migration must be applied before any
release that writes `grades/` ships — the server refuses a whole push if one path is refused.
**Merging with `p1-commitments`:** besides text conflicts in `ids.rs`, `backup.rs`, `sync_rows.ts`,
`sync_rows_test.ts` and the agreement test, two of p1's tests pin facts this branch changes — its
`migrations_sync_test.ts` expects its own `20260926000100` to be the newest path check, and
`backup_folders_has_ten_entries_with_commitments_before_state` expects ten entries. The second
merger updates both: the newest check is `20260929000100`, and the backup array has eleven entries
(`…info, commitments, grades, state, config, profile`).

## Task 9 — whole-branch review and the PR

An Opus review of the whole diff against the spec, CLAUDE.md's two rules and the contract list; fix
waves until it reads ready; then the PR, with the hand-offs to the other open branches named (the
`NOTE_FOLDERS` merge with `p1-commitments`, and whichever migration is applied last must carry the
union). **Quinn's live proof** (spec §13) follows the merge of the PR into a dev build.

## Fidelity ledger

| Spec | Task | Proven by |
|---|---|---|
| G1 persistent session, hidden refresh, forget | 5, 6 | `app/tests/grades.rs` (session dir, forget), live proof |
| G2 overall else points | 2, 4 | `course_grade` tests; surface test |
| G3 letter scale | 2 | `letter` boundary tests |
| §3 endpoints, paging | 5 | `assemble_bundle`, `next_page` tests |
| §5 bundle verbatim, in app data, deleted | 5, 6 | bundle tests; scheduler deletion test |
| §6 gated, deterministic ids, change-only, judge-once, state file, exit codes | 3 | `engine/tests/grades.rs` |
| §7 folder, ids, backup, sync both sides | 1 | ids/backup/sync_contract tests; Deno tests |
| §8 read model, references unchanged | 4 | surface test; `surface_oracle.rs` untouched |
| §9 console, commands, recount | 5, 7 | static-asset tests; the recount script |
| §10 slot order, named skips | 6 | scheduler tests |
| §11 privacy sentences, version | 8 | site/static tests |
| §13 live proof | after 9 | Quinn at the machine |
