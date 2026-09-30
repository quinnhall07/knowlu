# M1 — Grades from Blackboard

**Date:** 2026-09-29. **Status: Signed by Quinn, 2026-09-29.** The signature covers rulings G1–G4
(§1), the two §15 defaults and the signing packet's edits 3–5, which this text carries: edit 3 (the
privacy text is drafted here and moves to the page with privacy bump #1, §11), edit 4 (manual grade
entry is out of M1, §12) and edit 5 (Connect only where the school's policy read is recorded, §2–§4,
§9, §10 and §13).
**Stream:** `m1-grades` (worktree `.claude/worktrees/m1-grades`). **Authority:** `VISION.md` ("Grades",
"The main page", "Build order": *the MVP is quinn-ops parity plus grades from Blackboard*) and the
cloud design's amendment of 2026-09-29, rulings 7, 10 and 12 (G1 is signed there as ruling 12, G4 as
ruling 13).

## 1. Rulings this spec is written from (Quinn, 2026-09-29)

| # | Ruling |
|---|---|
| G1 | **Stay signed in.** Knowlu keeps a Blackboard-only browser session on the device so grades refresh every slot without a sign-in, until the school's SSO session expires. This reverses, for this one path, the capture window's "keep nothing" rule (`app/src/lms_link.rs`'s module doc; spec §11a). The calendar-link and course-list captures keep their incognito window unchanged. Its bounds are the cloud design's ruling 12, which also promises that **Knowlu's released app never offers or uses the session at a school without a recorded university-policy read**; §4's predicate is how the app keeps that promise. |
| G2 | **A course's ring shows Blackboard's Overall Grade when the instructor publishes one, and otherwise points earned ÷ points possible across graded items, labelled "points so far".** Syllabus weights replace the second basis in a later phase. |
| G3 | **Letters come from a default 90/80/70/60 scale with ±** (A 93, A− 90, B+ 87, B 83, B− 80, C+ 77, C 73, C− 70, D+ 67, D 63, D− 60, F below) until a syllabus supplies the course's own scale. |
| G4 | The repository stays public for now (irrelevant to the design; recorded because it rules out a self-hosted CI proof). |

## 2. What the student sees

- On **Today**, a **Grades strip**: one small ring per course, filled to the course's percentage and
  coloured by its letter family (A, B, C, D, F), with the course code and the percentage beside it.
  A course with nothing graded yet shows an empty grey ring and "no grades yet".
- **Clicking a ring** opens the breakdown: every graded or pending item for that course with its
  score, points possible and status (graded, needs grading, not submitted, exempt), grouped by the
  Blackboard category when one is known, and a footer: *From Blackboard · Overall Grade* or *From
  Blackboard · points so far*, *updated 2 h ago*, and **Refresh**. When a course has both an Overall
  Grade and points so far, the ring shows the Overall Grade (G2) and the footer shows points so far
  beside it (§15, signed default 2).
- **Not available:** when §4's predicate says no (the school has no curated Blackboard row carrying a
  recorded policy read), the strip is one line, *Grades from Blackboard are not available at your
  school yet*, with no button. There is no address entry: a school gains grades only through a
  curated row with a read (§4).
- **Not connected:** where the predicate says yes, the strip is one line, *Connect Blackboard to see
  your grades*, and a button.
- **Signed out:** when the saved session has expired, the strip keeps the last grades and says
  *Sign in to Blackboard again to update grades* with a button. Never an error, never amber: a
  signed-out session is a named state, like a skipped step.
- **Hide grades:** a setting that removes the strip (VISION: "A student can hide grades").
- Settings carry **Forget Blackboard sign-in**, which deletes the saved session.

## 3. The source: Blackboard's public REST API, called with the student's own session

The student-permitted endpoints (Anthology's documentation; `docs/notes/2026-08-28-redesign-program.md`
R3b) answer to the browser session cookie — **proven on UA's tenant by Quinn's probe of 2026-08-29**
(same note, "Probe RUN"): `users/me`, the memberships, `gradebook/columns` and `gradebook/users/{id}`
all answered with the session cookie alone. The app already makes exactly this kind of call:
`lms_link::capture_courses` reads `/learn/api/public/v1/users/me/courses?expand=course` with the
sign-in window's cookies (Task 13's outcome B).

| Call | Why |
|---|---|
| `GET /learn/api/public/v1/users/me` | the user's primary id; a 401 here is "signed out" |
| `GET /learn/api/public/v1/users/me/courses?expand=course` | the enrolments; `courseId` is the primary id the vault's course notes carry as `code:` |
| `GET /learn/api/public/v2/courses/{courseId}/gradebook/columns` | the gradable items: `id`, `name`, `score.possible`, `grading.type`, `grading.due`, `gradebookCategoryId`, `includeInCalculations`, `externalGrade` |
| `GET /learn/api/public/v2/courses/{courseId}/gradebook/users/{userId}` | the student's row per item: `columnId`, `status`, `score`, `text`, `exempt`, `displayGrade`; an unattempted item has **no row** |
| `GET /learn/api/public/v1/courses/{courseId}/gradebook/categories` | category titles for grouping; **optional** — a refusal only drops the grouping |

Every list follows `paging.nextPage` to the end (a cap of 20 pages per list, so a runaway loop is
impossible). Only courses whose membership is available are read. **The Overall Grade** is the
column with `externalGrade: true` (Blackboard's own flag for the grade the institution sees); it is
used only when the student's row for it carries a numeric `score` and the column a `score.possible`.

Canvas is out of scope (§12); the module is written with the LMS kind as a parameter so Canvas is
one more arm, not a rewrite.

**Grades preferences live in `<profile app data>\grades.json`, a file beside `settings.json`, never a
field in it** — `state::Settings` has no `#[serde(default)]` and falls back to defaults on a parse
failure, so a new field would reset a student's backup folder and autostart on their first launch
after the update (the precedent `account::cache_path` and `onboarding::offer_marker` set). It holds
`hidden` only (`{ "hidden": <bool> }`), written through `ledger::dumps_value`. It holds no LMS host:
the host comes only from the curated campus row (§4), never from the student.

## 4. The session (G1)

- **The gate: one predicate decides whether grades are available** (edit 5; cloud design ruling 12).
  The curated campus row `scaffold::Curated` gains **`policy_read`**, the date (`YYYY-MM-DD`) that
  school's university-policy read was done, absent until then. Grades are available only when the
  vault's `config/campus.yaml` `unitid` has a curated row whose `lms_kind` is `blackboard` and which
  carries `policy_read`. One predicate in `app/src/grades.rs` decides this and takes the curated row
  as its input. Four callers check it and none re-derives it: `grades_status` (the strip offers
  Connect only when it says available), `grades_connect` (so the first capture cannot run),
  `grades_refresh`, and the scheduler's grades step (§10). `grades_connect` and `grades_refresh`
  refuse with the same named reason the slot records, `not available at your school yet`, and open
  no window and read no session. `grades_forget` is never gated: deleting a saved session is always
  allowed. **No build skips the predicate:** tests, development builds and releases run the same
  check.
- **The date-and-bump rule** (ruling 12). A test fails when any curated campus row carries a
  `policy_read` date while `PRIVACY_VERSION` is older than privacy bump #1, the version whose page
  discloses the kept session and the grades the account holds (§11). A school's date therefore
  reaches `main` only in the same PR as bump #1 or after it. UA's read is done at MVP exit (ruling
  10), and the founder's MVP grades proof runs on a dev build from a **proof branch** that carries
  UA's date (the existing live-proof practice: dev build, scratch profile). That branch is never
  merged ahead of bump #1. MVP exit needs UA's read recorded and the proof passed on the proof
  branch, not UA's date on `main`.
- **A persistent WebView2 profile per Knowlu profile**, at `<profile app data>\lms-session\` (under
  `state::app_data_root()`, never the vault, never synced). WebView2 encrypts its cookie store with
  the Windows user's DPAPI key; only this Windows account can read it.
- **One window label, `lms-grades`**, used both for the visible sign-in and for the hidden refresh.
  Like `lms-signin`, it has **no capability grant**: `app/capabilities/default.json` scopes IPC to
  `["main"]`, and a static test pins that `lms-grades` never appears there.
- **Connect** (only when the predicate says available): the console's button opens `lms-grades`
  **visible** at `https://<lms_host>/` on the persistent profile. The student signs in (SSO, MFA) as
  they would in a browser. The app polls the window's URL; once it is on the LMS host and `users/me`
  answers 200 with the window's cookies, the window closes itself (§15, signed default 1), the first
  capture runs, and the strip fills. The student can also close it earlier.
- **Refresh (every slot, and Refresh in the breakdown; only when the predicate says available):** the app builds `lms-grades` **hidden** at
  `https://<lms_host>/ultra/`, waits up to 45 s for it to settle on the LMS host (a live SSO session
  redirects back silently), reads the cookies, and runs §3's calls with them — the cookie handover of
  `capture_courses`, one `ureq` agent, the cookies on the stack and never written, logged or put in
  an error. `users/me` answering 401/403, or the window never leaving the SSO host, is **signed out**.
  The window is destroyed after every refresh.
- **The LMS host:** the curated row's `lms_host` for the vault's `config/campus.yaml` `unitid`, and
  nothing else. M1 has **no address entry**: an uncurated school, including one whose `lms` is
  `blackboard`, is *not available* (§2) until a curated row with a recorded read is added for it.
- **Forget:** closes any `lms-grades` window and deletes `lms-session\` (the patient delete
  `lms_link::wipe_dir` already implements, pointed only at this directory by construction).

## 5. The capture bundle (app → engine)

The app writes one JSON file per capture into the profile's app data (`<profile>\grades-capture.json`,
never the vault or `%TEMP%`), runs the engine step, and deletes the file whatever the outcome. Shape
(version 1):

```json
{ "schema": 1, "lms": "blackboard", "host": "ualearn.blackboard.com",
  "fetched_at": "2026-09-29T14:02:11Z", "user_id": "_123_1",
  "courses": [ { "membership": {…the memberships row, verbatim…},
                 "columns": [ …every column object, verbatim… ],
                 "grades": [ …every row of gradebook/users/{id}, verbatim… ],
                 "categories": [ …or null when refused… ],
                 "error": null } ] }
```

Rows are **verbatim** so the engine owns every interpretation (the app computes nothing — CLAUDE.md),
and a fixture recorded from a real account can be replayed through the same code. A course whose
calls failed carries `"error": "<status or reason>"` and empty arrays; the engine names it and moves on.

## 6. The engine: `knowlu-engine grades --vault <v> --input <bundle> [--via <via>] [--run-id <id>]`

- **Gated** like `coursework` (`main.rs::gated_vault`): past the entitlement grace it prints the named
  refusal and exits 0.
- **Course matching:** a bundle course matches the vault course note whose `code:` equals the
  membership's `courseId`, else `course.courseId`, else `course.id`. Unmatched courses are counted
  and named, never guessed. Nothing is created in `courses/`.
- **One note per gradebook column**, in a new note folder **`grades/`**, named
  `grades/<course-slug>-<column-id-slug>.md` (`_4686399_1` → `4686399-1`), with a deterministic id
  `ids::derived_id("grade", path)` → `grade_<10 hex>` — the same file and id on every desktop, so two
  desktops that capture the same course converge through sync instead of duplicating.
- **Frontmatter** (every value single-line, through `yamlemit`):

  | field | value |
  |---|---|
  | `id`, `type` | `grade_…`, `grade` |
  | `title` | the column's `name` |
  | `course` | the vault course slug |
  | `source`, `source_uid` | `blackboard`, the column id |
  | `kind` | `item`, or `overall` for the `externalGrade` column |
  | `possible` | `score.possible`, or absent |
  | `score` | the row's numeric `score`, absent until graded |
  | `status` | `graded` (a numeric score), `needs-grading`, `in-progress`, `not-submitted` (no row), `exempt`, or `removed` (the column no longer comes back) |
  | `counts` | `includeInCalculations` |
  | `category` | the category title when known, else absent |
  | `due` | `grading.due` in the vault's zone, else absent |

- **Idempotent, and quiet when nothing changed.** A new column is `write::create`d; an existing note
  gets `write::write` with **only the fields whose values differ**, so an unchanged gradebook writes
  no journal record and syncs nothing. Actor `agent:knowlu.grades` (an agent actor, so
  `provenance::is_agent` holds); via as given (`local-runner` from the slot, `dashboard` from Refresh).
  **Judge-once is explicit here, not inherited:** `write`'s own guard covers only the kinds
  `provenance::judged_fields_for` names (`task`, `appr`), so `grades::apply` drops, before every
  write, each field for which `Journal::human_set(id, field)` answers — the agent never re-sets a
  field the student set. (Known, inherited: `human_set` and `console_ctx()` still match the literal
  actor `"quinn"`, a rule-1 bug that predates this stream; the fix changes both together and is not
  this stream's.) `write::create` keeps a caller-chosen `id:` only when `ids::is_id` accepts it, so
  the `grade_` id pattern (Task 1) must land before any grade note is created.
- **`state/grades.json`** (generated, device-local): `fetched_at`, the host, and per course the counts
  matched / skipped / failed. It is the strip's "updated 2 h ago". It is rewritten on every run and
  never synced.
- **Output:** one line per outcome (`grades: 6 courses, 3 changed items`; `grades: GN 103 failed
  (403)`), exit 0 for every per-course outcome; exit 1 only for a missing or unparseable bundle,
  because that is a bug in the capture, not a school being slow.

## 7. The vault: `grades/` becomes a note folder

- `ids::NOTE_FOLDERS` gains `grades`; `ids::ID_RE` accepts `grade_`; `ids::kind_for` maps
  `type: grade` to `grade`; `backup::BACKUP_FOLDERS` follows.
- **Sync:** `sync::is_note_path` follows `NOTE_FOLDERS`; the server's `sync_rows.ts::NOTE_PATH_RE` and
  the `sync_notes.path` check constraint gain `grades` through a new migration. **The migration lists
  `commitments` too** (the commitment model's phase 1 adds that folder on its own branch), so the
  constraint is right whichever branch merges first; `migrations_sync_test.ts`'s agreement check pins
  the server and the engine together.
- **The merge with `p1-commitments`.** Its agreement test asserts the server regex contains the exact
  group built from `NOTE_FOLDERS` in order. So the order is fixed now: `grades` goes **after** where
  `commitments` will sit — on this branch `[…, "info", "grades"]`, after both merge
  `[…, "info", "commitments", "grades"]` — and the server group is written in that merged order,
  `(tasks|approvals|archive|courses|issues|info|commitments|grades)`. This branch's own agreement
  test checks each engine folder appears in the server group (subset), so it passes alone; after the
  merge p1's exact-group test passes too. **Migration order:** this branch's migration
  (`20260929000100`) sorts after p1's (`20260926000100`). If `m1-grades` is applied to a database
  first, p1's older migration applied later by `db push --include-all` would re-add the constraint
  without `grades`. So whichever of the two merges second checks the database and, if needed, adds a
  newer forward-only migration that restates the union. Neither is applied to production before both
  are merged.
- Grades sync with the account like every other note (VISION, "Grades"). The privacy page says so
  from privacy bump #1 (§11).

## 8. The read model

`surface::build_state` gains **`grades: Vec<CourseGrade>`**, serialized only when non-empty, so every
existing surface reference (`surface-today-*.json`) is byte-for-byte unchanged — the fixtures have no
`grades/` folder. Pure and deterministic, computed from the `grades/` notes and `state/grades.json`:

- `CourseGrade { course, title, pct, basis, letter, family, earned, possible, graded, items, pending,
  fetched_at, entries: Vec<GradeEntry> }`, courses in the order of their course notes' titles.
- **basis `overall`** when the course's `kind: overall` note has `score` and `possible > 0`;
  **basis `points`** otherwise — Σ`score` ÷ Σ`possible` over `kind: item` notes with a score,
  `counts: true`, status `graded`, `possible > 0`; **basis `none`** (pct null) when nothing is graded.
- `earned` and `possible` are the points-so-far sums **whatever the basis**, so a course on basis
  `overall` still carries them and the footer can show points so far beside it (§15, default 2).
- `letter` from G3's scale; `family` is its first letter, for the ring's colour.
- `GradeEntry { title, category, score, possible, pct, status, due, counts }`, graded first by due,
  then pending by due.

A grade never enters `rank` in this phase (VISION: grade-based weighting is a later, bounded nudge).

## 9. The console

- The strip renders on Today from `state.grades`; the rings are inline SVG, coloured from five CSS
  tokens (`--grade-a` … `--grade-f`) defined for light and dark.
- The breakdown is the existing drawer pattern, read-only.
- New app commands (console window only), **four**: `grades_status` (available or not, connected,
  signed-out, host, last fetched), `grades_connect`, `grades_refresh`, `grades_forget`. Each lives in
  `app/src/grades.rs`; the console window's `generate_handler!` list gains four, and
  `docs/reference/app.md`'s command count is recounted by script, not by hand. There is no
  `grades_set_host` (§4: no address entry). `grades_status`, `grades_connect` and `grades_refresh`
  check §4's predicate; `grades_forget` does not.
- The console never decides availability itself: it shows the *not available* line (§2) exactly
  when `grades_status` says so.
- *Hide grades* is the per-profile `hidden` flag in `grades.json` (§3), toggled from the settings
  panel like the existing rows.

## 10. The slot

`sync → coursework → ingest → grades → judge → rank`. Before the engine steps run, the scheduler
captures (§4) when the vault is a Blackboard school, **§4's predicate says available**, entitlement
allows cloud steps, a session exists, and an app handle exists (the `--run-slot-once` path has none).
The predicate is checked whatever session exists: a saved session at a school without a recorded
read is never used. The engine step runs only when a bundle was written. Otherwise the slot records
one named step, exit 0, in the same way as the ingest and judge skips (and appended to
`state/runner-log.md` the same way):

- `grades (skipped: not available at your school yet)` — the predicate says no; checked after
  `not a Blackboard school` and before `not connected`;
- `grades (skipped: not connected)` — no saved session;
- `grades (skipped: signed out)` — the session expired; the strip asks the student to sign in;
- `grades (skipped: no entitlement)`;
- `grades (skipped: not a Blackboard school)`;
- `grades (skipped: Blackboard unreachable)`;
- `grades (skipped: sign-in window open)` — the student is signing in right now.

A capture never paints the tray amber; only the engine step's own exit 1 (a bad bundle) does.

## 11. Privacy

**M1 drafts the privacy sentences here and does not edit `site/privacy.html`** (edit 3; cloud design
ruling 12). The draft, for the page's section on what stays on the device: *If you connect
Blackboard for grades, Knowlu keeps your Blackboard sign-in on this computer, in a browser profile
Windows encrypts for your Windows account, so it can read your grades without asking you to sign in
every time. It never leaves this computer; Forget Blackboard sign-in deletes it.* And for what the
account holds: *your grades, with your tasks, to keep your desktops in step.*

**Privacy bump #1 carries them.** The page's text, its Effective date, `PRIVACY_VERSION` and the
tests that pin the new sentences (`engine/tests/site.rs`, `app/tests/static_assets.rs`) move
together in one PR, privacy bump #1 (M1's sentences, the registrar's and the re-consent screen
together, one lawyer read), as `app/src/account.rs:21-23` requires: bump the constant and the page's
date in the same commit, or the consent log points at text nobody can find. That PR ships no later
than the first release that records a `policy_read` date (§4). A release can carry M1's code before
then, because without a date it offers grades nowhere; §4's date-and-bump rule keeps every
`policy_read` date off `main` until bump #1. M1 does not move `PRIVACY_VERSION`.
**Consent:** the only accounts that accepted an earlier version are Quinn's (HANDOFF §1), so no
re-consent screen is needed for them; the Pilot's re-consent screen (R-PS-4, amendment ruling 10's
gate) is still required before any second account is onboarded. The lawyer packet (P5) gets this
delta.

Grades never enter telemetry and never an issue report: the report's vault counts may say how many
grade notes exist, never their names or values.

## 12. Out of scope (later phases, VISION's build order)

Canvas; syllabus weights and scales; the uncertainty band; term and cumulative GPA; grade-based
priority weighting; grade-posted emails as a source; zyBooks/VHL/Pearson scores as grades; pinning
ICS tasks to courses by gradebook column (C1c's D9 — the bundle makes it cheap, but it changes
`ingest`, which is not this stream's); **manual grade entry** (VISION's gap-filler, "manual entry
fills any gap"; its own phase, edit 4 — it adds an editing surface and a new write path, so M1 does
not build it). Also outside M1, and not promised to a later phase: an address entry for an
uncurated Blackboard school (§4); a school gains grades only through a curated row with a read.

## 13. Tests and proofs

- **Engine, TDD:** the bundle reader (verbatim rows, pagination-merged, errors), course matching,
  note creation with the deterministic id, change-only writes (a second identical run writes no
  journal record), `removed`, `exempt`, the overall/points/none bases, every letter boundary, and
  the CLI end to end on a temp copy of `vault-s1` with a **synthetic** bundle fixture under
  `engine/tests/fixtures/grades/` (a new fixture — never a frozen reference, never real grades).
- **Surface:** the grades block from synthetic notes; the three surface references unchanged.
- **Sync:** `is_note_path` accepts `grades/…`; the server regex and constraint agree (Deno test).
- **App:** the bundle assembly from canned responses (no network); the slot's named skips and step
  order; the capability file never grants `lms-grades`; the console renders the strip, the empty,
  not-available, signed-out and hidden states (static-asset tests, `scripts/console-shots.py` against
  a scratch vault).
- **The gate (§4):** the predicate takes the curated row as its input, so tests cover a dated row,
  an undated row and an uncurated school for **each** of the four callers (`grades_status`,
  `grades_connect`, `grades_refresh`, the scheduler's grades step); a refusal opens no window and
  reads no session. **The date-and-bump test** fails when any curated campus row carries a
  `policy_read` date while `PRIVACY_VERSION` is older than privacy bump #1.
- **Live proof** (desktop safety: never synthetic input): on the founder's scratch profile, on a dev
  build from the **proof branch** that carries UA's `policy_read` date, through the real gate (§4) —
  connect Blackboard, see the rings, run a slot and see a silent refresh, forget the sign-in and see
  *not connected*. The same build on a profile at a school without a dated row shows *not
  available*. The proof branch is never merged ahead of privacy bump #1. The first real capture's
  bundle is **not** committed (real grades).

## 14. Coordination

- `ids.rs`, `sync.rs`, `backup.rs` are on the contract list: those edits go through
  `contract-engineer`-level care and review. The commitment model adds `commitments` to the same
  array on `p1-commitments`; the second branch to merge adds its entry beside the first's.
- `main.rs` (engine and app), `lib.rs`, `scheduler.rs`, `scaffold.rs` (`Curated` gains
  `policy_read`) and `console.js` are shared files; the changes are additive and small, and the PR
  lists them as hand-offs.
- **Privacy bump #1** is not this stream's PR (§11). The PR names what bump #1 owes from M1: §11's
  sentences on the page, its Effective date, `PRIVACY_VERSION`, the sentence pins in
  `engine/tests/site.rs` and `app/tests/static_assets.rs`; and UA's `policy_read` date reaches
  `main` in that PR or after it.
- `docs/surface/anatomy.md` already names a Grades page; it is corrected to the strip-and-drawer this
  spec builds.

## 15. Defaults (signed by Quinn, 2026-09-29)

1. **The sign-in window closes on detection.** The visible Blackboard window closes itself once
   sign-in is detected (the window is on the LMS host and `users/me` answers 200); the student can
   close it earlier. It does not stay open like a browser tab (§4).
2. **Both bases are shown when both exist.** When a course has an Overall Grade and points so far,
   the ring shows the Overall Grade (G2), and the breakdown's footer shows points so far beside it
   (§2, §8).
