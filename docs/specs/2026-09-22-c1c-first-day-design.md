# C1c — the first day: design

**Status: SIGNED 2026-09-22 (Quinn's rulings R-C1c-1..3, recorded in the C1b ledger the same day).** Branch `c1c-first-day`,
stacked on `c1b-sign-in` (PR #9); merges after it. Parent designs: `2026-09-09-knowlu-cloud-design.md` (§4.2 onboarding,
§4.2 step 7 "the first slot runs immediately"), `2026-09-17-c1b-sign-in-design.md`, `2026-09-05-knowlu-friends-shell-design.md`
(the wizard, R-OB-1/2/3), the coursework spec §9 ("an empty parse is a failure, never an empty semester").

## 0. Where this comes from

The first complete live onboarding on 2026-09-22 (C1b's exit gate 6, attempt 5) produced a vault and a first slot, and the
founder's first look at the console found five things (C1b ledger F10–F14) plus one naming wrinkle. Quinn's ruling, verbatim:
*"upon completion of onboarding, the agents need to be run. We must provide clients with immediate value. Ideally, as we get
information throughout onboarding, we should be running agents in the background. We shouldn't wait until they're finished to
run it all at once (unless necessary)."* and, on the course names: *"let's address those naming schemas now if possible."*

What the vault, the logs and staging showed (all verified, not inferred):

- **The judge never ran.** The first slot (`scheduler::spawn`, `needs_first_run`, its own thread) started at 19:14:37.7Z; the
  launch entitlement refresh (`n == 1`, its own thread) wrote `entitlement.json` at 19:14:48.9Z. `judge_plan_for` saw
  `NotEntitled` and answered `judge (skipped: no entitlement)`. A skipped step writes no `slot-*.txt` and no line in
  `state/runner-log.md` (the engine writes that file; the app's skip lines live only in `RunSummary`), so nothing on screen or
  on disk said so. Staging's `usage_daily` has no row for the account that day. 27 Blackboard tasks sat at
  `course: null`, `needs_enrichment: true`, `importance_reason: "pending enrichment"`.
- **Past-due coursework topped Must do.** `coursework` created 23 zyBooks tasks, eight of them due before onboarding day, all
  `status: active, progress: 0`. `ingest`, in the same slot, archived 12 past Blackboard items under `archived_reason:
  imported-past` (R-OB-3) and created 27. The zyBooks parse (`zybooks::parse_assignments`) carries due, points and sections
  and no per-student completion; the VHL parse seeds `progress` from `percentage_complete` once.
- **VHL was unmapped.** The wizard sends a VHL section only with a typed course (`vhlRows` filters on `r.course`); the row had
  no suggestion and was left blank; `ingest.yaml` got `sections: {}`; the engine filed the card `map-vhl-2102121` (R-OB-1).
  Both VHL warnings in the run record follow from the empty `sections`. Discovery itself works (2 books, 1 section).
- **The course names.** Blackboard names UA's courses `202640-BUI-100-101` (term, department, number, section); the wizard's
  `lms_link::courses_from_json` slugs the NAME when `scaffold::suggest_course(code)` finds nothing in the LMS id
  (`_404752_1`), so the vault has `courses/202640-bui-100-101.md` titled `202640-BUI-100-101`, while a zyBooks or VHL course
  the student types is slugged from the code (`CS 100` → `cs-100`). `suggest_course` refuses the name because of the dash
  between `BUI` and `100`. Two naming schemes in one vault, and a typed `BUI 100` on the coursework panel would never meet
  `202640-bui-100-101`.
- **Blackboard gradebook items name no course.** The real feed's gradable-item events carry `SUMMARY`, `UID`
  (`_blackboard.platform.gradebook2.GradableItem-_4732722_1`), `DTSTART`, `DTEND`, `DTSTAMP` and nothing else — no
  `CATEGORIES`, no `DESCRIPTION`. `ingest::match_course` cannot attribute them; the judge (tier 1 on the title, tier 3 the
  model with the course list) is the only attribution path today, which is why the skipped judge hurt twice.
- **A white window.** The console page paints nothing until `state` returns a read model; `rank` wrote `today.md` 61 seconds
  after Finish. `commands::state` answers `ok: false` meanwhile and the page writes the error into `#delta` and nothing else.

## 1. Decisions

- **D1 (R-C1c-1).** The first slot at Finish runs the judge. A cloud vault whose entitlement is not yet cached refreshes it
  inside the slot, before the judge decision; only a refusal from the service (not a missing cache) is a named skip.
- **D2 (R-C1c-2, direction).** Work starts as onboarding gathers information, wherever the friends-shell design's Decision 3
  ("nothing reaches this machine's disk before Finish") allows. Device-side that means: the first slot starts the instant the
  vault exists (already true) and the console shows it running (§6). Server-side pre-work — fetching and pre-judging the LMS
  feed as soon as the calendars panel stores it in the account — belongs to C5 (relay fetch), which moves those fetches
  server-side anyway; §8 records what C5 owes this ruling. Nothing in this stream writes to the disk before Finish.
- **D3.** `coursework`'s first run archives items already past under the same rule and the same field as `ingest`
  (`archived_reason: imported-past`, R-OB-3), for zyBooks and VHL alike. "First run" is `ingest::is_first_run` (no `today.md`),
  the predicate both already share.
- **D4.** One course code, one slug. A course's human code is `<DEPT> <NUM>` (2–4 capitals, 3 digits, optional trailing
  letter), read from the LMS's own id when it carries one (`UACS100Fall2026` → `CS 100`, the existing rule with its
  institution-prefix peel) and otherwise from the LMS's name with a dash or space between department and number
  (`202640-BUI-100-101` → `BUI 100`, `MATH-125-001` → `MATH 125`, no peel). The slug is `slugify(code)` (`bui-100`). The
  course note is titled by the code and keeps the LMS name. A course whose id and name both carry no readable code keeps
  today's behaviour (slug of the name) — never a guess.
- **D5.** The coursework mapping row offers the vault's courses. Each mapping row's course field carries a `datalist` of the
  captured courses' codes, so a student picks `GN 101` rather than types it; a typed value is still honoured.
- **D6.** A mapping row with no suggestion says a course is needed; Next off the logins panel, with rows left blank and not
  ignored, says how many will be asked about in the app and goes on (R-C1b-exec-10 already lets it).
- **D7.** The console has a first-run view: from boot until the first read model exists it paints one line —
  "Knowlu is doing its first run. Your day appears here in about a minute." — and, beneath it, the slot's steps as they
  complete, polling every 3 s instead of 60 s until the read model arrives. The white window is gone.
- **D8.** A skipped step reaches `state/runner-log.md` as one line, in the engine's own format, written by the app through
  the engine's writer (`knowlu_engine::runs` / the runner-log appender the engine already uses), so the student and the
  Runs view see `judge (skipped: no entitlement)` the same way they see a WARN.
- **D9 (later, not this stream).** zyBooks per-student completion from the zyBooks API needs a new fixture (never a
  regenerated one) and its own spike; recorded in §8. Blackboard gradebook-column pins by course (the LMS API, from the
  capture window) likewise.

## 2. The first slot runs the judge (D1, D8)

`scheduler::run_slot_inner` (app/src/scheduler.rs ~560):

- Before `let est = entitlement_state(cs);`: if `config/cloud.yaml` exists and parses (`account::cloud_config(&cs.vault)` is
  `Ok`) and `account::cache_path(&cs.data_dir)` does not exist, call `account::refresh_entitlement(&cs.vault, &cs.data_dir)`
  synchronously. It is one HTTPS round trip under `account::TIMEOUT`. Its result is discarded here; `entitlement_state(cs)`
  then reads the cache it wrote. A failure leaves no cache and `judge_plan_for` names the skip as today. A cache that exists
  — even stale — is never refreshed here: the six-hourly housekeeping refresh and the 72-hour grace are unchanged.
- The step list records what happened: `entitlement (refreshed)` / `entitlement (refresh failed: <reason>)` as a step with
  exit code 0, pushed before the judge decision — visible in the Runs view like every other named step.
- Every `JudgePlan::Skip(note)` (and the existing ingest skips) is also appended to `state/runner-log.md` as
  `- <YYYY-MM-DD HH:MM> local skip <note>` through the engine's appender, under `cs.vault_io`, never while a child runs.
  The Runs view already lists steps from the run record; this line is for the file a student can open.
- `scheduler::spawn`'s two threads stay as they are; the first slot no longer depends on winning the race.

Tests: a scheduler test with a fake `cloud.yaml`, no cache and an unreachable `api_base` asserts the `entitlement (refresh
failed: …)` step precedes `judge (skipped: no entitlement)`, and that with a cache present no refresh step appears; a test
that the skip line lands in `runner-log.md` in the engine's format.

## 3. Coursework's first run (D3)

`coursework::sync_coursework` (engine/src/coursework.rs ~245) gains the `first_run: bool` its sibling `ingest::sync_tasks`
has. On a first run, an item whose `due` is before today (the vault's timezone, `cli::vault_zone`) is created straight into
`archive/` from an `IMPORTED_PAST` twin of `NOTE_TEMPLATE` (`status: archived`, `archived_reason: imported-past`,
`needs_enrichment: false`, `progress` as parsed — VHL's `percentage_complete` still seeds it), logged as
`archived (imported-past) <stem>`, its uid recorded in the seen ledger, and counted in the run record as `archived`. The
summary line mirrors ingest's: `coursework: first run — N item(s) already past were archived`. Not a first run: unchanged.
`--dry-run`: `would archive (imported-past) <stem> (due …)`. An item due today is created, not archived (ingest's rule).

Tests (engine, `coursework.rs`): first run with two past and one future item → two archived notes with the field, one
task, the seen ledger holds all three, the log and counts match; second run → nothing re-created; not-first-run → all three
created; dry run writes nothing. No frozen reference is touched (`zybooks-parsed-reference.json` and
`vhl-parsed-reference.json` are the parses, not the sync).

## 4. One course, one name (D4, D5)

- `scaffold::suggest_course` (app/src/scaffold.rs ~235) is unchanged for LMS ids. A new `scaffold::course_code_in_name(name)
  -> Option<String>` finds the first `([A-Z]{2,4})[-\s](\d{3}[A-Za-z]?)` in an LMS name and answers `"<DEPT> <NUM>"` with no
  prefix peel; it replaces `code_in_name` (whose two-word form it also accepts). `course_fragments` uses
  `suggest_course(code).or_else(|| course_code_in_name(&c.name))`.
- `lms_link::courses_from_json`: `slug = suggest_course(&code).or_else(|| course_code_in_name(&name)).map(slugify)
  .unwrap_or_else(|| slugify(name or code))`; the `Course` struct gains nothing — the wizard's row shows
  `<code or name>` and, when a code was read, the LMS name beside it in `.meta`.
- The course note: `title` is the human code when one was read, else the LMS name; a new frontmatter line `name: "<LMS
  name>"` always carries the LMS's own name; `code:` stays the LMS id; `slug:` unchanged. `renderCourses` shows
  `BUI 100 · 202640-BUI-100-101`.
- `course_map` lines (`course_map_lines`) are unchanged in shape: `[LMS id → slug]`, `[human code → slug]`, and the
  coursework panel's typed codes first-wins as today. With D4 a typed `BUI 100` on the coursework panel slugs to `bui-100`,
  the same note.
- The mapping row: `renderMapping` renders `<input list="wiz-course-codes" …>` and the page keeps one `<datalist
  id="wiz-course-codes">` filled from `WIZ.courses` (code or name) whenever `renderCourses` runs.

Tests: `course_code_in_name` on `202640-BUI-100-101`, `MATH-125-001`, `CS 100 Intro`, `Biology` (None), `UACS100Fall2026`
(None — that is `suggest_course`'s); `courses_from_json` on a Blackboard Ultra body whose names are UA's shape → slugs
`bui-100`; the scaffold writes `title: "BUI 100"` and `name: "202640-BUI-100-101"`; `static_assets` pins the datalist
and its id; the headless walk (`scripts/wizard-check.py`) sees the datalist carry the captured course and the mapping row
reference it.

## 5. The mapping rows say what they need (D6)

`renderMapping`: a row with no `suggested` and an empty `course` renders a `.meta` hint `type the course this belongs to`
beside the field (the datalist offers the list). In `wizStep`'s `leaving === 5 && n > leaving` branch, after a completed
discovery, the count of rows with an empty course and `ignore` false becomes the note
`N of these will be asked about in the app` (singular `1 of these will be asked about in the app`) in `#wiz-map-note`, and
Next goes on. `wizFinish` is unchanged: such rows still reach the engine's card path (R-OB-1).

Tests: `static_assets` pins the hint text and the count sentence; the headless walk leaves the VHL row blank and asserts
the sentence, then reaches Gmail.

## 6. The first-run view (D7)

- `commands::state`: when `build_state_value` fails AND `knowlu_engine::ingest::is_first_run(&cs.vault)` is true, the
  envelope is `{ ok: true, error: null, state: null, first_run: { running: <bool>, steps: [[label, code], …] } }` — `running`
  from `Scheduler.running`, `steps` from `Scheduler.last` when it exists (the slot in flight records nothing until it ends;
  the page shows the line alone until then). Any other failure keeps today's `ok: false`.
- `console.js`: `poll()` on `env.first_run` paints `#first-run` (a new block in `index.html`, hidden by default): the D7
  sentence and a list of the steps that have completed; it re-polls every 3 s while `first_run` is present and returns to
  the 60 s cadence once a state arrives, whereupon `#first-run` hides and the normal paint runs.

Tests: a `commands` test on a fresh scratch vault (no `today.md`) asserts the envelope shape; `static_assets` pins the block,
the sentence and the 3 s re-poll.

## 7. Exit gate (the live proof)

On a fresh scratch profile pointed at staging, with the founder's logins: onboarding to Finish; the console shows the
first-run line, then the day; the run record shows `entitlement (refreshed)` and a `judge` step that ran; staging's
`usage_daily` has a row for the day; no zyBooks task due before onboarding day is active; the courses are `bui-100`-style
with `name:` lines; the VHL row's blank course produced the count sentence and the engine's card. Then the scratch profile
and its Credential Manager entries are removed.

## 8. Recorded for later streams

- **C5 owes R-C1c-2:** when the calendars panel stores the LMS feed in the account, the service fetches it and pre-judges
  its items keyed by account, so the first device slot's judge pass is a cache read, not 27 model calls; likewise the Gmail
  and events pulls. The wizard could show "we are already reading your feed" on the panel.
- **zyBooks completion (D9):** the zyBooks API's per-activity completion for the signed-in student; a new fixture from a
  real account; a `progress` seed like VHL's, and `status: done` at 100 %.
- **Gradebook-column pins (D9):** Blackboard Learn's `/learn/api/public/v2/courses/{id}/gradebook/columns` from the
  capture window would pin every gradable item to its course by id (`ingest::match_course`'s uid pass) — a live spike
  against a real enrolment first, exactly as `courses_from_json` had.
- **The existing staging profile** (`Test`, 2026-09-22) keeps its old slugs; it is replaced at cut day.
