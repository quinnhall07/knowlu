# C1c — the first day: plan review

**2 blocking, 4 important, 7 minor.**

Reviewed: `docs/plans/2026-09-22-c1c-first-day-plan.md` (1,296 lines, five tasks and a hand-off)
against `docs/specs/2026-09-22-c1c-first-day-design.md` (D1–D8, §2–§6) and the current text of every
file it edits, in the worktree `.claude/worktrees/c1c-first-day` on branch `c1c-first-day`. Read-only
except for this file; no cargo, no tests, no writing git commands.

The plan is unusually close to the code: every line anchor it cites is right, every signature it
quotes is real, and the two knock-ons it named itself (the seven `runnable_vault` coursework tests
becoming first runs; the scheduler test that now reads the real Credential Manager) are handled
correctly. The two blocking findings are both cases where a test or a page condition cannot observe
the behaviour the spec asks for: one existing test that will go red and was not amended, and one
first-run condition that can never be true.

---

## BLOCKING

### B1 — Task 3 Step 7: an existing scaffold test goes red and the plan does not amend it

**Where:** Task 3, Step 5 (the course-note `front` map) and Step 7 ("Expected: PASS, including
`a_captured_course_is_exactly_what_the_wizard_plan_takes_back` and
`a_scaffolded_vault_ranks_without_the_unmigrated_warning`").

**Evidence.** `app/tests/scaffold.rs:682-686` asserts the course note's title verbatim:

```rust
for (slug, title) in [("cs-100", "CS 100 Intro to Computer Science"), ("gn-103", "GN 103 German")] {
    …
    assert!(text.contains(&format!("title: {title}")), "{text}");
```

against the seeds at `app/tests/scaffold.rs:676-678` (`code: "CS 100"`, `name: "CS 100 Intro to
Computer Science"`). Under Step 5's `let human = suggest_course(&c.code).or_else(|| course_code_in_name(&c.name));`:
`suggest_course("CS 100")` is `None` (`app/src/scaffold.rs:237-247` requires the digits to follow the
capitals with no separator), so `course_code_in_name("CS 100 Intro to Computer Science")` answers
`Some("CS 100")` and the note is titled `CS 100`, not `CS 100 Intro to Computer Science`. Both rows
of the loop fail. `every_enrolled_course_becomes_a_note_the_engine_can_find` is not named anywhere in
Task 3, so an implementer following the plan literally hits a red test with no instruction and no
stated expectation — exactly the state the plan's per-task "Expected: PASS" lines exist to prevent.

**Change.** Add a step to Task 3 (between the present Steps 5 and 6):

> **Step 5a: Amend the course-note test D4 changes.** In `app/tests/scaffold.rs`, in
> `every_enrolled_course_becomes_a_note_the_engine_can_find` (~682), the loop's expected titles
> become the human codes and the LMS name moves to its own assertion:
> ```rust
> // D4: the note is titled by the human code, and `name:` carries the school's own name.
> for (slug, title, name) in [
>     ("cs-100", "CS 100", "CS 100 Intro to Computer Science"),
>     ("gn-103", "GN 103", "GN 103 German"),
> ] {
>     …
>     assert!(text.contains(&format!("title: {title}")), "{text}");
>     assert!(text.contains(&format!("name: {name}")), "{text}");
> ```
> and the doc comment gains one sentence saying the title is the code from D4 on.

and add `every_enrolled_course_becomes_a_note_the_engine_can_find` to Step 7's "Expected: PASS" list.
(Checked and unaffected: `a_captured_course_maps_by_its_lms_id_and_by_the_code_a_summary_spells`
(`app/tests/scaffold.rs:698-751`) — `course_code_in_name` returns the same fragment as `code_in_name`
for all three of its seeds; `a_page_supplied_course_slug_is_normalised_before_it_names_a_file`
(`app/tests/onboarding.rs:916-942`) — the plan does not touch the path; every `courses_from_json`
case in `app/tests/lms_link.rs:404-424, 470-481`.)

### B2 — Task 5 Step 8: the first-run view can never paint

**Where:** Task 5, Step 8 (`poll`'s new body) and Step 5 (the static test that pins it).

**Evidence.** The plan's condition is

```js
if (env.first_run && (!env.ok || !env.state)) { renderFirstRun(env.first_run); return; }
```

and `env.ok` is false only when `build_state_value` returns `Err`. `build_state_value`
(`app/src/commands.rs:17-37`) can fail on exactly three things: an unknown view name
(`surface::View::parse`), a poisoned `head_sha`/`history`/`backup` lock, and a `serde_json::to_value`
failure on a plain struct. It does **not** fail for a vault with no read model:
`knowlu_engine::surface::build_state` returns `State` unconditionally (`engine/src/surface.rs:1769`)
and `surface::load` (`engine/src/surface.rs:103-119`) reads `tasks/`, `config/planning.yaml`,
`state/calendar.md` and `config/week_template.yaml` — never `state/today.md`. So on a wizard-made
vault the first poll answers `ok: true` with a (nearly empty) state, the condition is false, and
`renderFirstRun` is never called. D7 is not delivered, and §7's live proof ("the console shows the
first-run line, then the day") fails after all five tasks are done.

The plan already contains the argument against its own condition — Task 5 Step 3's doc comment says
the block is attached "not only when the read model could not be built: a vault that builds an EMPTY
state would otherwise paint an empty day, which is the white window by another name" — and then the
page throws that away. Note also that the spec's §0 claim ("`commands::state` answers `ok: false`
meanwhile") is not reproducible from the current code; whatever the live window really hit (an
`invoke` rejection or a throw inside `paint` both land in `poll`'s `.catch`, which the plan's new
body still bypasses), `!env.ok` is the wrong trigger.

**Change.** In Task 5:

- Step 8, `poll`'s body: `if (env.first_run) { renderFirstRun(env.first_run); return; }` — the
  envelope carries `first_run` exactly while `state/today.md` does not exist, which *is* D7's
  "until the first read model exists". Keep Step 3's envelope rewrite as the `ok: false` safety net.
- Step 5, the static test: replace the `poll.contains("env.first_run")` assertion with one that pins
  the condition itself, e.g. `assert!(poll.contains("if (env.first_run) {"), "the first-run view
  stands on is_first_run alone, not on a failed state")`, and keep the `EL("first-run").hidden = true`
  assertion.
- Step 1, the commands test: add two lines that pin what `state` really answers on a never-ranked
  vault, so the premise is tested rather than assumed —
  ```rust
  // The read model builds fine without `state/today.md` (surface::build_state has no failure
  // path), so `ok` is true and the block — not `ok: false` — is what the page keys on.
  let env = state_inner(&cs, "today").unwrap();
  assert_eq!(env["ok"], true);
  ```
- Step 8's comment: record the consequence — a vault whose `rank` keeps failing stays on the
  first-run line, with the failed step visible in the list beneath it (see M6).

---

## IMPORTANT

### I1 — Task 4 Steps 6 and 7: D6's count sentence is written into a panel that is already hidden

**Where:** Task 4, Step 6 (`noteUnmapped()` inside the `leaving === 5` branch) and Step 7 (the
headless check that "proves" it).

**Evidence.** `wizStep` sets `WIZ.step = Math.max(0, Math.min(PANELS.length - 1, n));` at
`app/static/console.js:1692` — **before** the `leaving === 5 && n > leaving` branch. On the press the
plan targets, `WIZ.discovered` is true, so the branch reaches
`if (WIZ.map.length || WIZ.discovered) { renderWizard(); return; }` (`console.js:1707`) and
`renderWizard` hides every panel but the new one (`console.js:1529`). `#wiz-map-note` lives inside
`#wiz-logins` (`app/static/index.html:138`), so the sentence lands in a `hidden` panel on the same
tick it is written. The student never reads it.

The Step 7 check does not catch this: `page.inner_text("#wiz-map-note")` is a query, not an action —
Playwright waits only for *attached*, and `innerText` on a non-rendered element falls back to
`textContent`, so the assertion passes on invisible text. Step 7 then presses Back, which is the only
reason anything in the walk ever sees the sentence.

**Change.** In Task 4:

- Step 7, make the check prove what it claims:
  ```python
  page.click("#wiz-next"); page.wait_for_timeout(300)
  if not page.is_visible("#wiz-map-note"): bad.append("the count sentence was written to a hidden panel")
  if "asked about in the app" not in page.inner_text("#wiz-map-note"): bad.append(…)
  ```
- Step 6, put the sentence where it is read. The smallest change that keeps spec §5's "Next goes on":
  store the count on `WIZ` (`WIZ.unmapped = n`) in `noteUnmapped()`, keep the `#wiz-map-note` write
  for the Back path, and have `renderWizard` append it to `#wiz-summary` on the Finish panel
  (`console.js:1562`) — the last thing the student reads before pressing Finish. Pin that in the
  `static_assets` test and in the walk (`if "asked about in the app" not in page.inner_text("#wiz-summary")`).
- Because spec §5 names `#wiz-map-note` explicitly, say in the task that this is a deviation and why,
  and flag it for the controller to confirm with Quinn rather than deciding it inside the plan.

### I2 — Task 3 Step 6: `renderCourses` shows the opposite of what spec §4 states

**Where:** Task 3, Step 6.

**Evidence.** Spec §4 says "`renderCourses` shows `BUI 100 · 202640-BUI-100-101`" — the human code
first, the LMS name beside it. The plan renders `h(c.name || c.code)` plus `" · " + h(c.slug)`, i.e.
`202640-BUI-100-101 · bui-100`. The spec is self-contradictory here: the same bullet says "the
`Course` struct gains nothing" (`app/src/lms_link.rs:461-469` carries `code`, `name`, `slug` only,
and `code` is the opaque `_404752_1`), and the plan's own rule forbids the page from reading a code
out of a name — so `BUI 100` is not available to `renderCourses` at all. The plan resolves the
contradiction silently, in a code comment, rather than recording it as a deviation.

**Change.** In Task 3 Step 6, above the code block, add:

> Spec §4's `BUI 100 · 202640-BUI-100-101` cannot be rendered without either the page reading a code
> out of a name (forbidden — the rule lives in `scaffold::course_code_in_name`) or `lms_link::Course`
> gaining a fourth field (the same bullet says it gains nothing). The row therefore shows the LMS's
> own name and, beside it, the slug the vault will use — the two things the struct actually carries.
> Deviation from §4, recorded here for the controller.

If Quinn would rather have the code on screen, the alternative is one field: `Course { code, name,
slug, display }`, filled in `courses_from_json` from the same `suggest_course(..).or_else(..)` chain,
and `a_captured_course_is_exactly_what_the_wizard_plan_takes_back` (`app/tests/lms_link.rs:470-481`,
which pins `keys == ["code", "name", "slug"]`) amended with it. That is a bigger change and should be
asked, not assumed.

### I3 — Task 1 Step 5: the D3 summary line is unobservable, and the test's doc claims otherwise

**Where:** Task 1, Step 5 (the `coursework: first run — …` push) and Step 1's fourth test.

**Evidence.** Spec §3 mandates the line. The plan pushes it in `main_with_fetchers`
(`engine/src/coursework.rs:1491`), whose only return is an `i32`; the line is printed to stdout and
never reaches `state/runner-log.md`, whose summary is built separately at
`engine/src/coursework.rs:1604-1610` (`coursework (N assignments; …)`). So nothing in the plan
asserts the line exists, in any spelling. Worse, the test that is supposed to cover it says it does:
Step 1's `a_first_run_counts_and_names_what_it_archived` opens "The run record and the summary line
say it too: a student who opens `state/runner-log.md` … sees why their first day is shorter than
their vendor's list" — but its only log assertion is `run_log(&vault).contains("coursework (")`,
which every coursework run already satisfies (`engine/src/coursework.rs:2740` uses the same string
for a different purpose).

**Change.** In Task 1:

- Move the push into `sync_coursework`, where the tests can see it. At the end of the function, before
  `Ok(log)`:
  ```rust
  // D3, ingest's line in coursework's words (`ingest::run_lines`, engine/src/ingest.rs:945). Pushed
  // here rather than in `main_with_fetchers` so the unit tests can read it: `main` returns an exit
  // code, and this line is not the run-log summary.
  if first_run {
      let archived = log.iter().filter(|l| l.contains("(imported-past)")).count();
      log.push(format!("coursework: first run — {archived} item(s) already past were archived"));
  }
  ```
  and delete the `if first_run { … }` block from Step 5's closure (the count filters below are
  unaffected — the line starts with `coursework:`).
- Add the assertion to Step 1's first test: `assert_eq!(log.last().unwrap(), "coursework: first run — 2 item(s) already past were archived");`
  and to the dry-run test (`… — 1 item(s) …`, proving both spellings are counted).
- Rewrite the fourth test's doc comment to claim only what it proves: the run record's `archived`
  count, and that `main` stays green.
- State the one deliberate divergence from ingest: a fetch that returned nothing runs no sync and so
  says nothing, where ingest reports `0 item(s)`.

### I4 — Task 2 Step 4: routine skips now evict real failures from `runner-log.md`

**Where:** Task 2, Step 4.

**Evidence.** The plan appends a line for every named skip on **every** slot. A vault with no account
and no `ics_url` writes `ingest (skipped: no ics_url)` twice a day forever; a lapsed account adds
`judge (skipped: no entitlement)`. `cli::line_status` reads the fifth space-separated token
(`engine/src/cli.rs:648-655`), which for these lines is `skip`, so they land in the non-`ok` bucket,
and `trim_log_lines` keeps only the newest **100** non-`ok` lines (`engine/src/cli.rs:657-676`). That
rule exists for one stated reason — "A failure must not age out of the log while routine `ok` runs
keep flowing" (`engine/src/cli.rs:657`) — and four routine skips a day exhaust its budget in under a
month, after which a genuine WARN or FAIL ages out. This is the first thing ever to write a non-`ok`
line that is not a problem.

**Change.** In Task 2 Step 4, write a skip line only when it is not already the newest line for that
step — one read of the file the appender is about to read anyway:

```rust
// A skip repeats every slot; the log's retention keeps only the newest 100 non-`ok` lines
// (cli::trim_log_lines) and they are there for failures. One line per CHANGE of state: a skip
// already sitting at the end of the file is the same fact, not a new one.
let existing = knowlu_engine::pystr::read_text(&cs.vault.join("state").join("runner-log.md")).unwrap_or_default();
let tail: Vec<&str> = existing.lines().rev().take(4).collect();
for note in skips.iter().filter(|n| !tail.iter().any(|l| l.ends_with(n.as_str()))) { … }
```

and add a third scheduler test: two consecutive `run_slot_inner` calls on the same vault leave exactly
one `local skip judge (skipped: no entitlement)` line. If Quinn would rather have one line per slot,
say so in the task and record that the retention rule now trades failures for skips.

---

## MINOR

- **M1 — Task 1 Step 4: a dry run writes a directory.** `let _ = std::fs::create_dir_all(&archive_dir);`
  sits above the `if dry_run { … continue; }` guard, so `--dry-run` creates `archive/` in the vault.
  The test does not catch it (`vault_with` already makes `archive/`, `engine/src/coursework.rs:1673-1680`,
  and `snapshot` records files only, `:1694-1713`). Move the `create_dir_all` below the dry-run
  `continue`, beside the `write::create`.
- **M2 — Task 1 Step 2: the expected failure names the wrong helper.** "…and `vault_with` has no
  `today.md` to remove" — `vault_with` (`engine/src/coursework.rs:1673`) is untouched; it is
  `runnable_vault` (`:2689`) that gains one in Step 6. Replace with "…and the four `main` tests fail
  because their fixtures' due dates are already past and every `runnable_vault` is a first run."
- **M3 — Task 4 Step 1: the test comment mis-names its own slice.** "Asserted inside `wizGo`, so
  `wizFinish`'s own bookkeeping cannot stand in for it" — the slice runs from `function wizGo(` to
  `function wizRegister(`, which spans `wizStep` (`app/static/console.js:1670`), and that is where the
  call goes. Say "inside `wizGo`/`wizStep`", as the sibling test at `app/tests/static_assets.rs:523`
  does not need to.
- **M4 — Task 4 Step 4: the datalist mixes two spellings and shows the machine one.** A captured
  course offers `c.slug` (`cs-100`), a typed one offers `c.code` (`GN 103` — a typed row is pushed as
  `{code, name: code, slug: ""}`, `app/static/console.js:2011`), while the field's placeholder says
  "Course code, e.g. CS 100". Both slug to the same note, so the D5 requirement holds, but the list a
  student reads is inconsistent. Give the option a label:
  `'<option value="' + h(c.slug || c.code) + '">' + h(c.name || c.code) + "</option>"`, and pin the
  label in the static test.
- **M5 — Task 1 Step 4: the cutoff is computed on every sync.** `crate::cli::local_now(vault)` reads
  and parses `config/ingest.yaml` (`engine/src/cli.rs:115-132`) even when `first_run` is false, which
  is every run after the first. Compute it inside the `if first_run` branch, or bind it lazily.
- **M6 — Task 5 Step 8: the 3-second poll has no end.** A vault whose `rank` keeps failing never
  writes `state/today.md`, so the page polls every three seconds forever. The steps list beneath the
  line does show the non-zero step once the slot ends, which is enough to be honest; say so in the
  step's comment so the next reader knows it was considered.
- **M7 — Task 1 Step 5: the run record gains `archived: 0` on every coursework run.** The counts
  vector is built unconditionally (`engine/src/coursework.rs:1586`), so every sync step from now on
  carries a fourth key and the Runs view renders it. That is consistent with `created`/`updated`/
  `skipped`, and no frozen reference is touched (`the_run_records_match_python_byte_for_byte`,
  `engine/src/runs.rs:930`, builds its own step), but it is a visible change to a record shape and the
  plan should say it is intended.

---

## Spec coverage

| Spec item | Task | Lands the spec's behaviour? |
| --- | --- | --- |
| D1 (first slot refreshes the entitlement) | Task 2, Steps 3, 1 | Yes. The guard is exactly §2's (`cloud_config(..).is_ok() && !cache_path(..).exists()`), synchronous, discarded result, both arms exit 0, pushed before `judge_plan_for`. |
| D2 (work starts as onboarding gathers) | — (Task 5 for the device half) | Yes, as ruled: nothing new is built, nothing writes to disk before Finish, and the missing half (the console showing the slot run) is §6. C5's debt stays in spec §8. |
| D3 (coursework's first run archives what is past) | Task 1 | Yes for the archive, the field, the seen ledger, the count and the `--dry-run` spelling; the summary line is written but untested and unobservable (I3). |
| D4 (one course code, one slug) | Task 3 | Yes for the rule and the slug; `title:` is the human code and `name:` the LMS name; the wizard row diverges from §4's example (I2) and one existing test goes red (B1). |
| D5 (the datalist) | Task 4, Steps 3-4 | Yes on the requirement that matters — a picked value slugs to the same note as the captured course; the spelling offered is the slug, not the code (M4). |
| D6 (a blank row says what it costs) | Task 4, Steps 5-6 | The hint lands; the count sentence is written but never seen (I1). |
| D7 (the first-run view) | Task 5 | Block, page, sentence and cadence are all there; the condition that shows them can never be true (B2). |
| D8 (a skip reaches `runner-log.md`) | Task 2, Step 4 | Yes, in the engine's own format through `cli::append_run_log`; the retention interaction is unaddressed (I4). |
| §2 (the slot) | Task 2 | Both bullets and both tests, plus the amendment to `a_vault_with_an_account_and_no_entitlement_…` for `CREDMAN_LOCK`. |
| §3 (coursework's first run) | Task 1 | All four tests the section asks for; no frozen reference touched. |
| §4 (one course, one name) | Task 3 (D4) + Task 4 (D5) | See D4/D5. |
| §5 (the mapping rows) | Task 4 | See D6. |
| §6 (the first-run view) | Task 5 | See D7. The envelope shape matches §6; the page condition does not. |
| §7 (the live proof) | Hand-off, item 3 | Correctly left to the controller, with the eight checks §7 names. |
| §8 / D9 | — | Correctly out of scope and left in the spec. |

## Checks run and passed

- **Signatures and visibility.** `knowlu_engine::cli::append_run_log(&Path, &str, &str, &str, Option<DateTime>) -> io::Result<()>`
  is `pub` (`engine/src/cli.rs:687-693`) and renders through `runs::log_line` alone
  (`engine/src/runs.rs:532-555`), whose output is exactly D8's `- YYYY-MM-DD HH:MM local skip <note>`.
  `account::{cloud_config, cache_path, refresh_entitlement, save_cache, EntitlementCache, decide}` all
  `pub` with the fields and arities the plan uses (`app/src/account.rs:645-773`). `scheduler::{lock,
  Scheduler, RunSummary}` `pub` with `running`, `last`, `steps: Vec<(String, i32)>` and a `Default`
  impl (`app/src/scheduler.rs:51-133`). `ingest::{is_first_run, IMPORTED_PAST, existing_by_uid,
  slugify}` `pub`. `cli::local_now`/`vault_zone` `pub` (`engine/src/cli.rs:115-132`).
- **Anchors.** Every line number the plan cites is right: `NOTE_TEMPLATE` 183-200, `sync_coursework`
  245, the `seen.contains` guard 370 / `tasks_dir.join` 375, `let today = …` 1531, the counts vector
  1586, the test `sync` helper 1765, `runnable_vault` 2689 (`engine/src/coursework.rs`);
  `entitlement_state` 609, the judge-skip push 611-613, `match engine_exe()` 614
  (`app/src/scheduler.rs`); `code_in_name` 287, `course_fragments` 310, the course-note loop 651-666
  (`app/src/scaffold.rs`); the slug expression 501-505 (`app/src/lms_link.rs`); `attach_scheduler`
  51-72 and the `state` wrapper 338 (`app/src/commands.rs`); `poll` 579, `wizStep`'s branch 1691-1712,
  `renderCourses` 1999, `renderMapping` 2018, `setInterval(poll, 60000)` 1378 (`app/static/console.js`);
  `#delta` 37, `#wiz-logins` 131-140 (`app/static/index.html`); the VHL-row fill 240
  (`scripts/wizard-check.py`).
- **Compilation, by hand.** Task 1's archive block: every binding it uses (`item_ctx`, `new_due`,
  `stamp`, `known`, `log`, `journal`, `dry_run`) is in scope at the insertion point; `path` is borrowed
  for `ids::rel` before it is moved into `known`; the `?` arm matches the sibling create path.
  `Runs::new(&vault).read(None)` on a temporary is legal and is what
  `main_writes_a_coursework_run_record_with_per_source_counts` already does; `sync_step["counts"]["archived"]`
  indexes a `serde_json::Map` (`engine/src/ledger.rs:36`) whose `archived` key Step 5 adds.
  `Date::yesterday()/tomorrow()` return `Result` (jiff 0.2) and the plan unwraps them, as
  `app/src/scheduler.rs:717` does. `course_code_in_name` was traced by hand over all eight of its test
  inputs and answers each one as asserted, including the two negatives (`MATH-1250-001`,
  `UACS100Fall2026`).
- **Second-run behaviour.** `existing_by_uid` indexes `archive/` as well as `tasks/`
  (`engine/src/ingest.rs:397-400`), and the update branch's `archived` check
  (`engine/src/coursework.rs:314-321`) returns before logging, so Step 1's
  `assert!(again.is_empty())` holds.
- **The named knock-ons.** `runnable_vault` (`engine/src/coursework.rs:2689-2697`) writes no
  `state/today.md`, and the fixtures of `main_writes_a_coursework_run_record_with_per_source_counts`
  (due 2026-08-26/27) and its five siblings are all in the past — Step 6's one added line is exactly
  what keeps them green. `CREDMAN_LOCK` already exists in `app/tests/scheduler.rs:80`, so the two new
  tests and the amendment compile; `an_entitlement_past_the_grace_…` saves a cache
  (`app/tests/scheduler.rs:716-722`) and so needs no lock, and `an_unreadable_cloud_yaml_…` is
  `Unreadable`, not a refresh — both correctly left alone.
- **Contracts.** No frozen reference is read or written by any task: the eight in
  `engine/tests/fixtures/` and the three `surface-today-*.json` are untouched, and
  `the_run_records_match_python_byte_for_byte` builds its own step list. `stamp` keeps its own clock
  (the plan says so and the code leaves it). `journal::VIAS` is not touched. New frontmatter is
  additive (`name:` on a course note, `archived_reason`/`needs_enrichment` on a new archived note) and
  nothing rewrites an existing file; every note write goes through `write::create`.
- **Constraints.** No `std::process::Command` is added. No secret, account id, email or person's name
  appears: hosts are `http://127.0.0.1:9/functions/v1`, accounts `acc-1`, addresses `@example.invalid`.
  No digit appears in any copy string (D6's count is computed; D7's sentence says "about a minute").
  All five commit steps carry both trailers and use `git add <paths>`. Every task ends with
  `cargo test --workspace` in the foreground naming the one accepted failure
  (`app/tests/commands.rs:222`).
- **Warnings.** Nothing the plan adds is an unused import, an unused variable or an undocumented
  `pub fn`: `course_code_in_name` and `first_run_value` both carry doc comments, `IMPORTED_PAST_TEMPLATE`
  is used, and Task 5's `use knowlu::scheduler::{lock, RunSummary, Scheduler};` is fully consumed by
  its test.
- **Page invariants.** The new JS introduces no `slugify` literal, so
  `app/tests/static_assets.rs:519` still holds; `renderCourses` stays defined after the wizard click
  listener, so the slice at `app/tests/static_assets.rs:605` still finds it; the fixed function list
  at `app/tests/static_assets.rs:55` is a subset check, so `renderCourseCodes`, `noteUnmapped` and
  `renderFirstRun` do not disturb it; `.lede` is a global class (`app/static/console.css:100`).
- **Right-sizing.** The five cuts are file-disjoint and each commits green: engine only (Task 1),
  scheduler only (Task 2), the course-code rule (Task 3), the wizard rows (Task 4), the first-run view
  (Task 5). Tasks 3 and 4 both edit `app/static/console.js` and both restate `renderCourses`, which
  the plan flags in Task 3 Step 6 — that is a sequencing note, not duplication. No step is a
  placeholder; every test is written out in full.
