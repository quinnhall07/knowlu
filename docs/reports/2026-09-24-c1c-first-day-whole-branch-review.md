# C1c (the first day) — the whole-branch reviews, the fix round and the re-review

In order: the final review of Tasks 1–5, its fix round, its scoped re-review, then the second (scoped) final review over Task 6 and the debug round that followed. Tasks 7–11 were reviewed task by task; see the ledger. Preserved verbatim from the worktree workspace (`.superpowers/sdd/2026-09-22-c1c-first-day-plan/`, git-ignored) when the stream was closed on 2026-09-25, before the worktree and its branch were deleted.

---

<!-- the review: final-review.md -->

# C1c — the first day: final whole-branch review (773f529..931fa52)

Reviewer: Claude Opus 5.5 (senior code review, read-only). Scope: the code under `engine/`, `app/`, `scripts/` against
`docs/specs/2026-09-22-c1c-first-day-design.md`, the plan's Global Constraints and rulings R-C1c-plan-1..4, and the
ledger's deferred minors. One targeted test run (build output only, no source or git change) backs Important 1.

### Strengths

- **"First run" means one thing across the branch.** Task 1 (`coursework::main_with_fetchers`), Task 5
  (`commands::first_run_value`), `ingest` and `scheduler::needs_first_run` all read `!state/today.md`.
  Coursework reads it before it writes anything, so coursework and ingest agree within one slot. `runnable_vault` now
  writes `today.md`, so every test from before D3 keeps the behaviour it was written against.
- **Task 1 matches ingest's R-OB-3 closely.** It makes one `create` into `archive/` through `write::create`, then
  one `record_seen`. The dry-run guard sits above the `create_dir_all` (M1). The cutoff is `Option<Date>` and is
  computed only on a first run (M5). "Due today" is created, not archived. Archived notes are skipped by the
  existing `archived || status != active` guard on later runs. No frozen reference is touched.
- **Task 2 respects the lock rules.** `refresh_entitlement` takes no lock. `vault_io` is held only around the
  runner-log append and is released before any child starts. Every step it adds exits 0. `est` is computed after the
  refresh, so the judge decision and the telemetry step both see the fresh cache. The in-slot refresh (at slot
  start) and the launch refresh (at +10 s) are ordered by construction rather than by winning a race.
- **Task 3 reads the code in one place.** `course_code_in_name` is hand-rolled with correct run boundaries: `BUI`
  never becomes `UI`, and `MATH-1250` never becomes `MATH 125`. `label` goes to the page and back with `#[serde(default)]`.
  The note is emitted through `safe_dump_block`, and `course_fragments` uses the same rule.
- **Task 4 is escaped and keeps R-C1b-exec-10.** Both label consumers go through `h()`, and the count is written
  with `textContent`. An empty discovery (`map.length === 0`, `discovered`) returns 0 from `noteUnmapped()` and goes
  on. Re-stored logins clear `mapWarned` along with `discovered`.
- **Task 5 is small and safe.** One `exists()` runs per poll on the main thread. `attach_scheduler` still
  short-circuits on a null state. One timer is cleared before it is set, so the 60 s interval, the focus handler and
  the 3 s chain never multiply. `#first-run` has no class, so the UA `[hidden]` rule holds (the `.app[hidden]` trap
  does not apply).
- **Repo rules hold.** Every touched blob is LF. No new child process. No secret, account id or person's name in
  code or tests (`acc-1`, loopback `127.0.0.1:9`). No digit baked into on-screen copy. Tests that touch the real
  Credential Manager take `CREDMAN_LOCK` before `ENGINE_ENV_LOCK`, the same order everywhere.

### Issues

#### Critical (Must Fix)

None.

#### Important (Should Fix)

1. **`engine/src/coursework.rs:2005` — `a_first_run_counts_and_names_what_it_archived` fails depending on the
   machine's timezone.**
   - **What:** the test builds its two items around `jiff::Zoned::now().date()`, the machine's zone. The cutoff
     it tests is `cli::local_now(vault)`, the vault's zone (`America/Chicago`). Whenever the machine's date is ahead
     of Chicago's, the "yesterday 23:59" item is due today in Chicago. It is created instead of archived, and
     `counts.archived` is 0.
   - **Evidence:** reproduced with `$env:TZ = "Asia/Tokyo"; cargo test -p knowlu-engine --lib
     a_first_run_counts_and_names_what_it_archived`. The output reads `coursework: first run — 0 item(s) already past
     were archived`, then `left: Number(0) right: 1`. It passes on this machine (Central).
   - **Why it matters:** GitHub's Windows runners run in UTC. `ci.yml` will fail this test on every push between
     00:00 UTC and 05:00 UTC (06:00 in winter), which is every Central evening. The two commits before this branch
     point exist to remove exactly this class of clock-sensitive test.
   - **Fix:** build the items from the same clock the cutoff reads: `let now = crate::cli::local_now(&vault).date();`
     after `runnable_vault` wrote `timezone:`. Alternatively, widen the margins to ±2 days.
   - **Must be fixed before merge.**

2. **`app/static/console.js:2108-2111` (with `:1849`) — the D6 sentence is false for zyBooks rows.**
   - **What:** `noteUnmapped()` counts every blank, un-ignored row and says "N of these will be asked about in the
     app". But `wizFinish` puts every zyBooks row with `!r.course` into `zybooks_ignore` (`:1849`, from C1 Task 17).
     Both `coursework::route_zybook` and the service's `routeZybook`
     (`cloud/supabase/functions/ingest-coursework/handler.ts:91-94`) skip an ignored book silently. No proposal and
     no card are ever made, so a blank zyBooks row is never asked about.
   - **Where the sentence is true:** only for VHL rows. Blank VHL sections are left out of `sections:`, the
     service proposes them, and a card follows.
   - **Why it matters:** this is a student-facing falsehood about where their work went. Spec §5's premise
     ("`wizFinish` is unchanged: such rows still reach the engine's card path") is wrong for zyBooks. R-OB-1 says
     "an unknown book on a later run is a proposal, never a silent skip".
   - **Fix, (a), recommended:** add only `r.ignore` rows to `zybooks_ignore`, so a blank book is `Unmapped`, gets
     a card, and the sentence becomes true. Add a wizard-check assertion that a blank zyBooks row is not in
     `zybooks_ignore`.
   - **Fix, (b):** count only VHL rows, and say something else about blank zyBooks rows.
   - (a) changes `wizFinish`, which the spec calls unchanged, so it needs a one-line controller ruling.
   - **Fix before the §7 proof.**

3. **`app/src/scheduler.rs:649-650` and `:656`; spec §2 and §7 — `entitlement (refreshed)` is not observable
   anywhere, so a §7 gate line cannot be checked.**
   - **What:** app-side steps live only in `RunSummary` (`Scheduler.last`). The engine's run record, which the Runs
     view reads through `runs::Runs`, never contains them. The page renders `last_slot` only as "last slot HH:MM
     ok/failed". The first-run block, the one place that lists step names, disappears once `rank` writes `today.md`,
     which happens before `Scheduler.last` is set.
   - **Result:** after a successful first slot, `entitlement (refreshed)` is in no file and on no screen. §7's
     "the run record shows `entitlement (refreshed)`" can't be verified as written.
   - **Wrong claims to correct:** spec §2 says "visible in the Runs view like every other named step". The comment
     at `:649-650` says "The step itself is still a step in the run record, which is what the Runs view reads". Both
     are wrong. The R-C1c-plan-4 ledger rationale says the same thing.
   - **Fix:** add `|| n.starts_with("entitlement (")` to the D8 filter at `:656`, which writes one `ok` line and
     keeps R-C1c-plan-4's reasoning. Correct the comment and spec §2's sentence. The alternative is to amend §7 to
     witnesses that exist: the engine's `judge` step and `usage_daily`.
   - **Decide before the live proof.**

4. **`app/static/console.js:1829` (a C1 Task 17 bug that undoes part of D4) — every captured course's LMS id reaches
   `course_map` with a phantom slug.**
   - **What:** `WIZ.courses.forEach(c => codes[c.code] = true)` sends `[<LMS id>, ""]` for every captured course.
     `create_vault_in` fills the empty slug with `slugify(<LMS id>)`, so `_404752_1` becomes `404752-1`. Because
     `scaffold::course_map_lines` is first-wins with the page's entries first, the derived
     `'_404752_1': 'bui-100'` is dropped. `ingest.yaml` ends up with `'_404752_1': '404752-1'`, a slug with no note.
     The wizard-check stub gives the same shape (`UACS100Fall2026` → `uacs100fall2026`).
   - **Why it matters:** spec §4 promises `[LMS id → slug]`. Any UID carrying the course id is attributed to a
     course that doesn't exist. Spec §8's gradebook-column pins (D9) depend on this exact line. The scaffold test
     `a_captured_course_maps_by_its_lms_id_and_by_the_code_a_summary_spells` passes only because its plan leaves out
     the page's `course_map` shape.
   - **Fix:** `if (c.code && !c.slug)`. Typed courses carry `slug: ""`; captured courses are already covered by
     `course_fragments`. Add a wizard-check assertion that `course_map` has no captured LMS id with an empty slug.
   - This predates the branch and could be deferred with a recorded follow-up, but it is one line in a file this
     branch already changes, and it is the D4 guarantee.

#### Minor (Nice to Have)

1. **`console.js:593-595` — the first-run step list drops each step's exit code.** A failed `rank` reads like one
   that finished. The M6 comment depends on "the failed step shows up in the list". The amber "last slot … failed"
   in the sync line partly covers it. Fix: add ` (failed)` when `s[1] !== 0`.
2. **The D7 copy "about a minute" was measured on a slot with no judge.** With D1 the first slot now runs the judge
   before `rank`: one service call per unenriched item, plus the events, Gmail and rules pulls. The first day will
   take longer than 61 s. Record the real time in the §7 proof; the wording is Quinn's decision.
3. **`coursework.rs:522-524` and `:1991` — stale line numbers in comments.** They cite `engine/src/ingest.rs:945`
   and "`:1604`"; the summary is now around `:1727`. Name the functions instead.
4. **`scheduler.rs:606-608` — a stranded comment.** The "Fix round 1 (M3): computed once…" comment now sits above
   the D1 refresh block rather than `let est = entitlement_state(cs);`. Move it down.
5. **`lms_link.rs` `courses_from_json` — the code rule is tried on the name only.** Canvas's `course_code`
   (`CS-100-001`) carries the code in the id with dashes, which `suggest_course` refuses. When Canvas is spiked,
   consider `suggest_course(code).or_else(|| course_code_in_name(&code)).or_else(|| course_code_in_name(&name))`.
   Canvas is unverified, so leave it for now.
6. **The coursework run-log summary doesn't mention archived items.** It reads `coursework (N assignments; X
   created, Y updated…`, so a first day's `runner-log.md` looks as if items disappeared. The run record has
   `archived`. Adding "; N archived" is optional.
7. **`commands.rs:365` — any failure during the first run becomes `ok: true, state: null`, including an unknown
   view.** This is harmless while `build_state` cannot fail. Leave it.
8. **`console.js` ~`:1863-1869` (predates the branch) — a misleading comment on `entitlement_now`.** The comment says
   the call is "the first moment the cache can be written". In fact `entitlement_now` writes no cache and reads the
   pending session that `create_vault` has just moved away. D1 is now the real fix; correct or remove the call
   later.

### Deferred minors triage

- **Task 1 — report wording nit:** leave.
- **Task 2 — `cloud_config` parsed twice on the refresh path (the plan mandated it):** leave.
- **Task 2 — the launch refresh and the in-slot refresh share `entitlement.json.tmp`:** leave for this merge.
  - The launch refresh fires at least 10 s after spawn and the in-slot refresh at slot start, so they only overlap
    if the round trip takes about 10 s.
  - The worst interleaving is worse than the ledger says. The losing writer's fallback `remove_file(&path)` deletes
    the winner's fresh cache, and the result is the F13 skip again.
  - Follow-up: a unique tmp name per call, and no remove when the tmp file itself is gone.
- **Task 3 — doc-comment paraphrase on the round-trip test:** leave.
- **Task 4 — `mapWarned` stays set after Back then Next:** leave. It matches `discovered`'s stay-once shape.
- **Task 5 — nothing open.**

### Recommendations

- **A design gap for Quinn to rule on; it doesn't block this merge.** D3's "first run" is per vault, but a source can
  be imported for the first time on a later run. Both of these bring F11 back, with past-due items created
  `active`:
  1. The first slot's coursework fetch fails. `coursework` always exits 0, so `rank` still writes `today.md` and the
     next slot is not a first run.
  2. A row left blank and answered later through its card, which the D6 sentence now points students towards.

  A per-source predicate would cover both: no uid from that book or section in the seen ledger yet. Record it in
  spec §8 beside D9.
- Run the whole workspace suite once with `TZ=UTC` set from PowerShell. Git Bash does not pass `TZ` to native
  executables. This checks for other clock-sensitive tests before CI finds them.
- For the §7 proof, also read `config/ingest.yaml`'s `course_map` (Important 4) and note the time from Finish to the
  first day (Minor 2).

### Assessment

**Ready to merge?** With fixes.

**Reasoning:** The five tasks match the spec and rulings, and I found nothing unsafe in locking, writes or escaping.
Important 1 will fail CI every UTC night and must be fixed before merge. Importants 2 and 3 should be fixed before the
§7 live proof, because they make the gate misleading or impossible to check. Important 4 is a one-line fix that makes
D4's `[LMS id → slug]` guarantee true.

---

<!-- the fix round: final-fix-report.md -->

# C1c final-review fix wave — report

Branch `c1c-first-day`, base HEAD `931fa52`. All five rulings plus the taken minors, applied as
four commits (grouped where files/functions overlap). Worktree:
`C:\Users\danie\GitHub\knowlu\.claude\worktrees\c1c-first-day`.

## R-C1c-final-1 (I1) — coursework's first-run test used the machine's clock

**File:line:** `engine/src/coursework.rs:2005` (test `a_first_run_counts_and_names_what_it_archived`,
in `mod tests`).

**Fix:** `let now = jiff::Zoned::now().date();` → `let now = crate::cli::local_now(&vault).date();`
— the test's items are now built around the same clock (the vault's `America/Chicago`) that the
cutoff under test reads, instead of the machine's local timezone.

Also corrected two stale line-number references in nearby comments (`:522-524` and `:1991`) that
cited `engine/src/ingest.rs:945` and `:1604`; both now name the function/location instead
(`ingest::run_lines`'s first-run arm; "`main_with_fetchers` builds separately, after the sync
loop"), so they cannot go stale again. The test's own doc comment was also updated to say the
cutoff is read in the vault's timezone, not the machine's (documents the fix itself).

**Test:** the existing test, corrected — no new test needed since the bug was in the test's own
setup.

**RED** (reproduced exactly as the review specified, before the fix):
```
$env:TZ = "Asia/Tokyo"; cargo test -p knowlu-engine --lib a_first_run_counts_and_names_what_it_archived
coursework: first run — 0 item(s) already past were archived
thread 'coursework::tests::a_first_run_counts_and_names_what_it_archived' (17932) panicked at
engine\src\coursework.rs:2020:9:
assertion `left == right` failed
  left: Number(0)
 right: 1
test coursework::tests::a_first_run_counts_and_names_what_it_archived ... FAILED
test result: FAILED. 0 passed; 1 failed; 0 ignored; 0 measured; 938 filtered out; finished in 0.15s
```

**GREEN, same command, `TZ=Asia/Tokyo`:**
```
coursework: first run — 1 item(s) already past were archived
test coursework::tests::a_first_run_counts_and_names_what_it_archived ... ok
test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 938 filtered out; finished in 0.21s
```

**GREEN, no TZ override (this machine's own, Central):**
```
coursework: first run — 1 item(s) already past were archived
test coursework::tests::a_first_run_counts_and_names_what_it_archived ... ok
test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 938 filtered out; finished in 0.16s
```
Both runs green, as required.

## R-C1c-final-2 (I2) — a blank zyBooks row was silently ignored forever, never proposed

**File:line:** `app/static/console.js`, `wizFinish`'s `zybooks_ignore:` line (was `:1849`, now
`:1858` after the added comment).

**Engine path read first, as instructed:** `engine/src/coursework.rs:571` `route_zybook(code,
courses, ignore)` returns `BookRouting::Unmapped` only when `code` is in neither `courses` nor
`ignore`; `fetch_zybooks` (line ~609) turns `Unmapped` into a WARN, `"zybook {code} not in
config; skipped"`, and takes no further action locally. The cloud mirror that actually files the
card is `cloud/supabase/functions/ingest-coursework/parse_zybooks.ts`'s `routeZybook` /
`handler.ts`'s `pickZybooks`: `routing.kind === "unmapped"` pushes a `MapProposal` (`proposals.push
({ source: "zybooks", key: code, ... })`) — this is the coursework-map card R-OB-1 promises. A code
in `ignore` (`routing.kind === "ignored"`) is skipped with `continue` and **no** proposal, silently,
forever. So a blank row that `wizFinish` put in `zybooks_ignore` never got a card; only a genuinely
unmapped one does.

**Fix:** `zybooks_ignore: WIZ.map.filter(r => r.source === "zybooks" && (r.ignore || !r.course))` →
`.filter(r => r.source === "zybooks" && r.ignore)` — only rows the student explicitly ticked.

**Tests:**
- `app/tests/static_assets.rs`, new test `a_blank_zybooks_row_is_never_sent_as_ignored`, pins the
  exact filter substring and rejects the old one.
- `scripts/wizard-check.py`: the fake `discover_coursework` now returns two extra zyBooks rows
  (`HowToUseZyBooks2`, left untouched/blank; `AnotherOldBook2020`, ticked ignored via
  `page.check('[data-ignore-for="3"]')`); the `create_vault` plan assertion checks
  `AnotherOldBook2020 in zybooks_ignore` and `HowToUseZyBooks2 not in zybooks_ignore`.

**RED** (static_assets.rs, reverted filter):
```
thread 'a_blank_zybooks_row_is_never_sent_as_ignored' (29656) panicked at app\tests\static_assets.rs:578:5:
zybooks_ignore takes only rows the student ticked as ignored
test a_blank_zybooks_row_is_never_sent_as_ignored ... FAILED
test result: FAILED. 0 passed; 1 failed; 0 ignored; 0 measured; 49 filtered out; finished in 0.00s
```
**GREEN:**
```
test a_blank_zybooks_row_is_never_sent_as_ignored ... ok
test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 49 filtered out; finished in 0.00s
```

**RED** (wizard-check.py, both I2 and I4 reverted together):
```
FAIL: a blank, un-ticked zyBooks row was sent as ignored: ['HowToUseZyBooks2', 'AnotherOldBook2020']
FAIL: a captured course's LMS id was sent in course_map with a phantom slug
2 failure(s)
```
**GREEN:** `ok`

## R-C1c-final-4 (I4) — a captured course's LMS id got a phantom empty slug

**File:line:** `app/static/console.js`, `wizFinish` (was `:1829`, now inside the `codes` block
above `zybooks_ignore`).

**Fix:** `WIZ.courses.forEach(c => { if (c.code) { codes[c.code] = true; } })` →
`if (c.code && !c.slug)`. A typed course carries `slug: ""` and this is the only place anything
derives one for it; a captured course already carries its own real slug and is covered by the
engine's `course_fragments` (`app/src/scaffold.rs:354`). Sending a captured course's code here too
gave it a second `[id, ""]` entry that `onboarding::create_vault_in` (line ~611) fills from the
code itself, and because `scaffold::course_map_lines` (line ~386) is first-wins by key with the
page's own `plan.course_map` entries inserted before the derived ones, that phantom slug
permanently shadowed the real one.

**Tests:**
- `app/tests/static_assets.rs`, new test `a_captured_courses_lms_id_never_gets_a_phantom_slug`,
  pins `if (c.code && !c.slug)`.
- `scripts/wizard-check.py`: new assertion that `course_map` never carries an entry keyed
  `UACS100Fall2026` (the mocked captured course's LMS id, which already carries slug `cs-100`).

**RED** (static_assets.rs, reverted condition):
```
thread 'a_captured_courses_lms_id_never_gets_a_phantom_slug' (30560) panicked at app\tests\static_assets.rs:601:5:
test a_captured_courses_lms_id_never_gets_a_phantom_slug ... FAILED
test result: FAILED. 0 passed; 1 failed; 0 ignored; 0 measured; 50 filtered out; finished in 0.00s
```
**GREEN:**
```
test a_captured_courses_lms_id_never_gets_a_phantom_slug ... ok
test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 50 filtered out; finished in 0.00s
```
wizard-check.py RED/GREEN: shown under I2 above (both assertions were reverted and verified
together, then restored and reverified together — `ok`).

Full `static_assets.rs` suite after both fixes: **51 passed, 0 failed.**

## R-C1c-final-3 (I3) — the entitlement step was invisible everywhere a gate could check it

**File:line:** `app/src/scheduler.rs`, the skip-line filter (was `:656`, now inside the
`filter_map` at the same location) plus the `:649-650` comment and the stranded `:606-608` comment.

**Fix:** the `skips` filter now also matches `"entitlement (refreshed)"` (verbatim) and
`"entitlement (refresh failed"` (prefix), rewriting the latter to the bare
`"entitlement (refresh failed)"` before it is appended to `state/runner-log.md` — never the `:
<reason>` tail, so no service error text reaches the vault. The in-memory `steps` vec (what
`RunSummary`/the settings page reads) is untouched and keeps the full reason.

**Comments corrected:**
- `:649-650`: "The step itself is still a step in the run record, which is what the Runs view
  reads" → now says the step is visible in `state/runner-log.md`, not the Runs view (that page
  reads the engine's own run record, `runs::Runs`, which this app-side step never enters).
- `:606-608`: the "Fix round 1 (M3): computed once and reused..." comment, which had been
  stranded above the D1 refresh block, was moved back to sit directly above
  `let est = entitlement_state(cs);`, the line it actually describes.
- `docs/specs/2026-09-22-c1c-first-day-design.md`: added two dated correction lines (2026-09-22,
  "final review I3") under §2 and §7, pointing the reader at `state/runner-log.md`'s `local ok
  entitlement (refreshed)` line instead of the (non-existent) Runs-view witness. Lines added, not
  rewritten.

**Test:** extended `a_first_slot_refreshes_the_entitlement_before_it_decides_about_judge`
(`app/tests/scheduler.rs`) with two new assertions.

**RED:**
```
thread 'a_first_slot_refreshes_the_entitlement_before_it_decides_about_judge' (27376) panicked at
app\tests\scheduler.rs:795:5:
test a_first_slot_refreshes_the_entitlement_before_it_decides_about_judge ... FAILED
test result: FAILED. 0 passed; 1 failed; 0 ignored; 0 measured; 27 filtered out; finished in 1.82s
```
**GREEN:**
```
test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 27 filtered out; finished in 1.84s
```
Full `scheduler.rs` suite after: **27 passed, 1 ignored** (the by-design `run_slot_end_to_end`).

## R-C1c-final-5 — concurrent entitlement-cache saves could delete each other's fresh cache

**File:line:** `app/src/account.rs`, `save_cache` (`:699`, now `:714`).

**Fix:** the `.tmp` filename is now unique per call — `entitlement.json.<pid>-<counter>.tmp`, via a
new file-scoped `AtomicU64` counter — instead of the single shared `entitlement.json.tmp`. Per the
review's note on the fallback path: with a shared name, the loser's `rename` can find its own
source already moved away by the winner (the winner's `rename` consumed whichever bytes were in
the shared tmp file at that instant), fall into the `remove_file(&path)` fallback, and delete the
WINNER's just-written fresh cache — both calls report `Ok(())` and the profile is left with no
cache at all (the F13 skip again). A unique name per call means no writer's source can ever be
another's to consume; the existing remove-then-retry fallback is otherwise unchanged and now only
ever contends with a genuinely locked target, never another call's in-flight write.

**Test:** new `overlapping_saves_never_clobber_each_other_into_no_cache_at_all`
(`app/tests/account.rs`) — 200 rounds of two barrier-synchronised threads calling `save_cache`
concurrently, asserting both always return `Ok` and the cache is always present and parseable
afterward. Does not take `CREDMAN_LOCK` (touches only local files, not Credential Manager), per
CLAUDE.md's rule that only credential-store tests take that lock.

**RED** (shared tmp name, unmodified):
```
thread 'overlapping_saves_never_clobber_each_other_into_no_cache_at_all' (29624) panicked at
app\tests\account.rs:441:9:
test overlapping_saves_never_clobber_each_other_into_no_cache_at_all ... FAILED
test result: FAILED. 0 passed; 1 failed; 0 ignored; 0 measured; 46 filtered out; finished in 0.21s
```
Failed on the first attempt — the race is easy to hit under a tight barrier-synchronised loop.

**GREEN** (unique tmp name), run 5 times to check for flakiness in the fix itself:
```
test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 46 filtered out; finished in 0.69s
test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 46 filtered out; finished in 0.71s
test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 46 filtered out; finished in 0.60s
test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 46 filtered out; finished in 0.58s
test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 46 filtered out; finished in 0.60s
```
Full `account.rs` suite after: **47 passed, 0 failed.**

## Minors taken

- Stale line numbers in `engine/src/coursework.rs:522-524` and `:1991` (cited
  `engine/src/ingest.rs:945` and `:1604`) — replaced with function names (folded into the R-1
  commit, same file/comments).
- Stranded comment in `app/src/scheduler.rs:606-608` — moved to sit above
  `let est = entitlement_state(cs);` (folded into the R-3 commit).
- Misleading `entitlement_now` comment in `app/static/console.js` (~`:1875-1879`, was
  `:1863-1869` pre-edit) claiming the call is "the first moment the cache can be written" —
  corrected to say the call writes no cache and typically finds nothing (the pending session was
  just moved away by `create_vault`); the real first write is D1's in-slot refresh. Folded into
  the R-2/R-4 commit since it sits inside the same `wizFinish` function those rulings touch.

No other minor was touched.

## Whole-workspace suite

One `cargo build --workspace` first (clean, only the accepted linker warning), then
`cargo test --workspace --no-fail-fast -- --test-threads=1` (foreground; `CARGO_BUILD_JOBS=2` used
after the first attempt hit a transient linker out-of-memory under full parallelism — an
environment condition, not a code issue; `knowlu-engine`'s real binary was rebuilt first per the
placeholder-sidecar hazard note).

Per-binary `test result:` lines:
```
knowlu (lib)                 ok. 10 passed; 0 failed; 0 ignored
knowlu (bin)                  ok. 0 passed; 0 failed; 0 ignored
tests/account.rs               ok. 47 passed; 0 failed; 0 ignored
tests/commands.rs              FAILED. 24 passed; 1 failed; 0 ignored   <- the one accepted failure
tests/credentials.rs           ok. 2 passed; 0 failed; 0 ignored
tests/inference.rs             ok. 19 passed; 0 failed; 0 ignored
tests/lms_link.rs              ok. 21 passed; 0 failed; 0 ignored
tests/no_console.rs (app)      ok. 1 passed; 0 failed; 0 ignored
tests/onboarding.rs            ok. 29 passed; 0 failed; 0 ignored
tests/profiles.rs              ok. 11 passed; 0 failed; 0 ignored
tests/report.rs                ok. 14 passed; 0 failed; 0 ignored
tests/scaffold.rs              ok. 24 passed; 0 failed; 0 ignored
tests/scheduler.rs             ok. 27 passed; 0 failed; 1 ignored
tests/static_assets.rs         ok. 51 passed; 0 failed; 0 ignored
tests/telemetry.rs             ok. 9 passed; 0 failed; 0 ignored
tests/updates.rs               ok. 8 passed; 0 failed; 0 ignored
knowlu_engine (lib)             ok. 936 passed; 0 failed; 3 ignored
knowlu_engine (bin)             ok. 4 passed; 0 failed; 0 ignored
tests/cloud_contract.rs         ok. 16 passed; 0 failed; 0 ignored
tests/dependency_boundary.rs    ok. 4 passed; 0 failed; 0 ignored
tests/no_console.rs (engine)    ok. 1 passed; 0 failed; 0 ignored
tests/oracle.rs                 ok. 3 passed; 0 failed; 0 ignored
tests/site.rs                   ok. 1 passed; 0 failed; 0 ignored
tests/starvation.rs             ok. 4 passed; 0 failed; 0 ignored
tests/surface_oracle.rs         ok. 4 passed; 0 failed; 0 ignored
tests/uievents_isolation.rs     ok. 1 passed; 0 failed; 0 ignored
tests/workflows.rs              ok. 9 passed; 0 failed; 0 ignored
```
Only failure: `commands::rejecting_and_snoozing_write_the_decision_fields` — the pre-existing,
named-accepted failure on this branch. Every other test across both crates passes; ignored counts
(1 in `scheduler.rs`, 3 in the engine lib) match the four by-design `#[ignore]`s.

**Warnings**, tallied with `grep -n "^warning" <raw log>`:
```
cargo : warning: linker stderr: ... ld.exe: .rsrc merge failure: multiple non-default manifests
warning: `knowlu` (bin "knowlu") generated 1 warning
warning: `knowlu` (bin "knowlu" test) generated 1 warning (1 duplicate)
```
Exactly one distinct warning anywhere in the log — the accepted `.rsrc merge failure` linker
message — surfaced by cargo's two summary lines (one marked "(1 duplicate)" of the other). No
other warning of any kind appears. **0 other.**

## wizard-check.py

```
Push-Location ...\c1c-first-day; python scripts\wizard-check.py; Pop-Location
ok
```
RED/GREEN for the two new assertions (I2, I4) shown above under those rulings.

## Line-ending verification

`core.autocrlf=true` makes the working tree CRLF for every `eol=lf`-attributed file (confirmed:
every touched file carries `eol: lf` via `git check-attr`), so the meaningful check is the
committed blob, not the working tree. For every file this wave touched:
```
git cat-file blob HEAD:<file> | od -An -tx1 | tr -d ' \n' | grep -o '0d0a' | wc -l   → 0 (all nine files)
```
Zero `0d 0a` byte pairs in any committed blob — all LF, as required.

## Commits

Four commits on `c1c-first-day`, newest last:
1. `64fd003` — fix(engine): the first-run archive test reads the vault's clock, not the machine's
   (R-1 + its two stale-comment minors). `engine/src/coursework.rs`.
2. `bd476f4` — fix(app): the wizard's zyBooks and captured-course mappings reach the engine
   correctly (R-2 + R-4 + the `entitlement_now` comment minor, all in the same `wizFinish`
   function). `app/static/console.js`, `app/tests/static_assets.rs`, `scripts/wizard-check.py`.
3. `1068632` — fix(app): the entitlement step's failure reason never reaches the vault (R-3 + its
   two comment minors + the spec corrections). `app/src/scheduler.rs`, `app/tests/scheduler.rs`,
   `docs/specs/2026-09-22-c1c-first-day-design.md`.
4. `927bf84` — fix(app): concurrent entitlement cache saves no longer clobber each other (R-5).
   `app/src/account.rs`, `app/tests/account.rs`.

Working tree clean after the fourth commit.

## Deviations

- Rulings 2 and 4 (both inside `wizFinish` in `console.js`) and the Minor-8 comment fix (also
  inside `wizFinish`) were committed together rather than as three separate commits — they touch
  the same function and could not be cleanly hunk-split without interactive staging, which was
  ruled out by "never `-A`, never `git stash`" plus no interactive tooling available. The commit
  message enumerates all three changes by ruling/minor number.
- `cargo test --workspace` initially hit a transient linker out-of-memory (`ld.exe: out of memory
  allocating 144898608 bytes`) under full build parallelism (~7 GB free RAM at the time); retried
  with `CARGO_BUILD_JOBS=2`, which completed cleanly. Not a code issue — no source change involved.
- The default `cargo test --workspace` invocation stops at the first failing test binary; since
  `commands.rs`'s one accepted failure is expected, `--no-fail-fast` was used for the final run so
  every other binary's results could be confirmed rather than skipped.

## Concerns

- None outstanding from the five rulings or the taken minors. The one design-level item the review
  flagged as "for Quinn to rule on; it doesn't block this merge" (a per-source first-run predicate
  for coursework sources imported later than the vault's own first run) was left exactly as the
  review left it — recorded, not actioned, per the review's own recommendation.

---

<!-- the re-review: final-rereview.md -->

# C1c final-review fix wave — re-review (931fa52..927bf84)

### Finding Verdicts

- **R-C1c-final-1 (I1) — coursework's first-run test now reads the vault's own clock.** — ADDRESSED,
  `engine/src/coursework.rs:2043`. `let now = crate::cli::local_now(&vault).date();` reads
  `config/ingest.yaml`'s `timezone: America/Chicago` (written by the test's own `runnable_vault` call),
  exactly the same call the production cutoff uses (`sync_coursework`, `engine/src/coursework.rs:318`:
  `today.unwrap_or_else(|| crate::cli::local_now(vault).date())`). Both reads now go through the same
  function against the same explicit config value, so the outcome no longer depends on the host's
  system timezone or `TZ` at all — not just "usually passes under Tokyo" but structurally decoupled
  from the machine's clock. Independently reran `cargo test -p knowlu-engine --lib
  a_first_run_counts_and_names_what_it_archived` — passes (`coursework: first run — 1 item(s) already
  past were archived`).

- **R-C1c-final-2 (I2) — a blank zyBooks row no longer reaches `zybooks_ignore`.** — ADDRESSED,
  `app/static/console.js:1861`. `zybooks_ignore: WIZ.map.filter(function (r) { return r.source ===
  "zybooks" && r.ignore; })` — only rows the student ticked. Named check (a) below confirms the
  consequence (a card, not silence). Tests: `app/tests/static_assets.rs`
  `a_blank_zybooks_row_is_never_sent_as_ignored` (pins the substring, rejects the old one) and
  `scripts/wizard-check.py`'s two new rows (`HowToUseZyBooks2` left blank at row index 2,
  `AnotherOldBook2020` ticked via `data-ignore-for="3"`) — index math checked against
  `renderMapping()`'s `WIZ.map.map(function (r, i) {…})` (`console.js:2103`), which is 0-based and a
  1:1 image of the `discover_coursework` rows array (`WIZ.map = ((d && d.rows) || []).map(...)`,
  `console.js:1760`); row 3 is indeed `AnotherOldBook2020`. No off-by-one.

- **R-C1c-final-4 (I4) — a captured course's LMS id no longer gets a phantom empty slug.** —
  ADDRESSED, `app/static/console.js:1837`. `if (c.code && !c.slug)`. Traced the whole chain: a typed
  course is pushed with `slug: ""` (`console.js:2091`); a captured course's slug is always non-empty
  (`lms_link.rs`'s `courses_from_json`, `app/src/lms_link.rs:481-520`, always derives a non-empty
  slug or the row is dropped outright at `:503-505`). `create_vault_in`
  (`app/src/onboarding.rs:609-616`) fills an empty second element with `slugify(code)`, and
  `scaffold::course_map_lines` (`app/src/scaffold.rs:386-403`) is first-wins by key, reading
  `plan.course_map` before the derived `course_fragments`. Before the fix, a captured course's code
  reserved its key with the phantom slug first; the real one from `course_fragments` was then
  silently dropped. The condition now excludes every captured course from `codes`, so only typed
  courses (the only ones needing a derived slug) reach `course_map` this way. Pinned by
  `static_assets.rs`'s `a_captured_courses_lms_id_never_gets_a_phantom_slug` and
  `wizard-check.py`'s new `course_map` assertion.

- **R-C1c-final-3 (I3) — the entitlement step's failure reason no longer reaches the vault; the
  Runs-view claim is corrected.** — ADDRESSED, `app/src/scheduler.rs:656-672` (the `skips` filter),
  `:649-651` (corrected comment), `:606-608` region (moved comment), plus
  `docs/specs/2026-09-22-c1c-first-day-design.md` §2/§7 correction blocks. The filter's
  `n.starts_with("entitlement (refresh failed")` arm unconditionally substitutes the literal
  `"entitlement (refresh failed)"` — it never touches `n`'s own text — so the service's `{e}` can
  never survive into what gets appended to `state/runner-log.md`. Independently reran
  `a_first_slot_refreshes_the_entitlement_before_it_decides_about_judge`
  (`app/tests/scheduler.rs:752`) — passes; its two new assertions (`log.contains("local ok
  entitlement (refresh failed)")` and `!log.contains("refresh failed:")`) match the source. The
  `:649-651` comment now correctly says the step is visible in `state/runner-log.md`, not the Runs
  view (which reads `runs::Runs`, the engine's own record — this app-side step never enters it). The
  spec's two correction blocks are additive (dated 2026-09-22, "final review I3"), nothing rewritten,
  and accurately point at the runner-log line instead of the non-existent Runs-view witness.

- **R-C1c-final-5 — concurrent `save_cache` writes can no longer clobber each other's fresh
  cache.** — ADDRESSED, `app/src/account.rs:697` (`CACHE_TMP_COUNTER`), `:714-731` (`save_cache`).
  The `.tmp` name is now `entitlement.json.<pid>-<counter>.tmp`, unique per call, so no writer's
  source can ever be another's in-flight write — the failure mode the review described (loser's
  fallback `remove_file` deleting the winner's just-written cache) is structurally impossible now.
  Independently reran `overlapping_saves_never_clobber_each_other_into_no_cache_at_all` — passes,
  and inspecting the test's own scratch directory afterward showed exactly one file
  (`entitlement.json`), no stray `.tmp` — confirms the success path is clean. See named check (b)
  below for a residual concern on the failure path.

### Named Checks

**(a) I2: does an unmapped, non-ignored zyBooks book really produce a Decisions card?** Yes, and
via the intended function. Locally, `fetch_zybooks` (`engine/src/coursework.rs:582-626`) only warns
(`"zybook {code} not in config; skipped"`) for `BookRouting::Unmapped` — no card, by design (H12
predates this branch). The production cloud path is `collect_cloud`
(`engine/src/coursework.rs:862-925`), which calls `post_coursework` to decode the service's reply
`proposals` array into `MapProposal`s, then `propose_map_cards` (`:933-984`), which calls
`coursework::write_map_card` (`:1010-1053`) for every proposal not already asked about — this
writes the `kind: coursework-map` approval note under `approvals/` that R-OB-1 promises (the
"Decisions card"). On the cloud (Deno) side, `routeZybook`
(`cloud/supabase/functions/ingest-coursework/parse_zybooks.ts:86-92`) returns `{ kind: "unmapped" }`
for a code in neither `courses` nor `ignore`, and `handler.ts:91-98` turns that into a
`proposals.push({ source: "zybooks", key: code, ... })`. A code in `zybooks_ignore` instead hits the
`ignored` arm and is skipped with no proposal — which is exactly the bug I2 fixed: before the fix, a
blank row was put in `zybooks_ignore` and so took the silent-forever path instead of this one.
**Cite:** `engine::coursework::write_map_card`, called from `propose_map_cards`, called from
`collect_cloud`.

**(b) R5: does the rename-based save leave no stray temp files on success, and clean up on failure
the way the old code did?** On success: yes — `fs::rename(&tmp, &path)` consumes `tmp`, and 400
concurrent successful calls (200 rounds × 2 threads) in the reran test left exactly one file
(`entitlement.json`) in the scratch dir, no `.tmp`. On a **total** failure (both the direct rename
and the fallback's remove-then-retry rename fail) the function still returns `Err` either way, same
as before — but the two versions differ in what happens to disk afterward. The **old** code used one
shared name (`entitlement.json.tmp`) for every call; a stranded tmp from a failed call would be
overwritten and then successfully renamed away by the very next call that happened to succeed
(self-healing). The **new** code's name is unique per call (pid + monotonic counter) and nothing
ever globs `data_dir` for orphaned `entitlement.json.*.tmp` files — so a stranded tmp from a total
failure is never reused and never cleaned up by a later call; each subsequent total failure adds
another orphan. This requires the destination rename to fail twice in a row (very rare — a
persistent lock or a vanished directory), so it does not undermine the concurrency fix that R5 was
about, but it is a genuine, narrow regression versus "cleans up on failure where the old code did."
Not something any test currently exercises. Flagged below.

**(c) Can `state/runner-log.md` ever receive a service error string from these changes?** No.
`append_run_log` (`knowlu_engine::cli`) is called from exactly one site in `scheduler.rs` — the loop
over `skips` at the end of the `filter_map` block (`app/src/scheduler.rs` ~`:678`) — and every
element that filter_map can produce is either a static literal (`"entitlement (refreshed)"`,
`"entitlement (refresh failed)"`, an `ingest (skipped: …)`/`judge (skipped: …)` name — all of which
are themselves static strings from `JudgePlan::Skip(&'static str)` or the fixed `IcsState` arms) or
that one hard-coded replacement string. The only place the service's `{e}` text is interpolated is
the in-memory `steps.push((step, 0))` at the earlier push site (`app/src/scheduler.rs:622`), which
feeds `RunSummary`/the settings page, never the vault. Confirmed by rerunning the extended
scheduler test, which asserts `!log.contains("refresh failed:")`.

### New Breakage in the Fix Diff

- **`app/src/account.rs` `save_cache` — a total-failure path (both renames fail) now leaks a
  uniquely-named orphaned `.tmp` file forever, where the pre-fix shared-name scheme self-healed on
  the next successful call.** Nothing in the app globs or prunes `entitlement.json.*.tmp` in
  `data_dir`. Narrow (needs two consecutive rename failures on the same call) and low severity
  (disk clutter in `%LOCALAPPDATA%\knowlu\profiles\<id>\`, not a correctness or security issue, and
  it doesn't affect `load_cache`, which only ever reads the fixed `entitlement.json` name) — worth a
  one-line follow-up (e.g. best-effort `remove_file(&tmp)` after the fallback also fails) but not a
  blocker.
- **`app/tests/account.rs`'s new `overlapping_saves_never_clobber_each_other_into_no_cache_at_all`
  has no panic-safe cleanup for its scratch directory**, unlike the credential-store tests this same
  file's convention otherwise uses (a generated id plus a `Drop` guard, per CLAUDE.md). Its final
  `let _ = std::fs::remove_dir_all(&dir);` only runs if the 200-round loop's `assert!`s never fire —
  exactly the case where the test is doing its job and failing on a real regression. Confirmed by
  finding a stranded `knowlu-ent-race-<pid>` directory (containing a bare `entitlement.json`, no
  `.tmp`) still sitting in `%TEMP%` on this machine — consistent with the report's own documented RED
  run, whose panic skipped the cleanup line. Harmless (temp-directory clutter on the dev/CI machine
  only, never in the repo or the vault) but worth a `Drop` guard if this test is touched again.
- No new problem found in `wizard-check.py` itself: the new rows' `data-ignore-for="3"` index was
  checked against `renderMapping`'s 0-based, order-preserving `WIZ.map` and lands on
  `AnotherOldBook2020` as intended; the reused name `HowToUseZyBooks2` for the *blank* row is a
  slightly confusing choice (that exact string is unconditionally force-ignored server-side by
  `create_vault_in`, `app/src/onboarding.rs:621`, in the real app), but since this harness mocks
  `create_vault` entirely and only inspects the JS-built plan, the two never interact and the
  assertion is testing the right thing correctly.

### Out-of-Scope Observations

- The design gap the original review flagged for Quinn to rule on later (a per-source first-run
  predicate for coursework sources onboarded after the vault's own first run) is untouched by this
  wave, as the review itself asked.
- None of the eight remaining Minor items or the "deferred minors" triage were touched beyond what
  the report claims (Minor 3, 4, 8 taken; everything else left) — confirmed against the diff; no
  scope creep.

### Verdict

**Closed.** All five rulings are correctly implemented, each pinned by a test that fails on the
pre-fix code and passes on the post-fix code (reran three of the five myself: I1's coursework test,
I3's scheduler test, R5's account test — all green with no other warnings). All nine touched blobs
are LF at HEAD (independently verified via `git cat-file blob`), matching the report's claim. The
two items under "New Breakage" are both narrow, low-severity, and test/dev-machine-only — neither
touches production correctness of the five rulings and neither blocks merge; worth a follow-up note
but not a re-fix.

---

<!-- the second review: final-review-2.md -->

# C1c final review 2 (scoped): `927bf84..39c7a22`

Reviewer: opus, read-only. Scope: Task 6 (`3644c10`, `2a4b17b`), the debug agent's test commits (`6dec7a6`,
`cd8d147`), Task 7 (`6ea27ae`), Task 8 (`1b37458`, `679f086`, `8d1d24a`) and Task 9 (`39c7a22`). The context was
the whole branch (`773f529..39c7a22`), the ledger's rulings, and read-only trial merges against `c1b-sign-in`
(`00624d9`) and `c3-sync` (`a650877`).

## Verdict: READY WITH FIXES

One Important finding. R-C1c-10 is only half delivered: the slot that files the mapping card still records
`WARN`, so the Runs nav reads "1 warn" from the hand-over through about the first day. The rest are Minor. Two
of them are cheap now and save a red test or a silent gap at the C3′ merge (M2). Nothing is Critical, and
nothing in the scope breaks a frozen reference, the eol contract, `.no_console()` or the warnings gate.

## Verification (run by the reviewer)

- `cargo test --workspace --no-fail-fast`, in the foreground: **1290 passed, 1 failed, 4 ignored**. The one
  failure is the accepted `app/tests/commands.rs::rejecting_and_snoozing_write_the_decision_fields` (index out
  of bounds at :357, the fixture's frozen `expires:`). Warnings: **1 accepted (.rsrc), 2 tallies, 0 other**. The
  engine binary under `target\debug` was real (141 MB) before the run.
- `python scripts/wizard-check.py`: **ok**.
- Deno, the touched files only (`ingest-coursework/`, `migrations_test.ts`, `judge_caps_test.ts`,
  `judge_pipeline_test.ts`, read permission only so nothing is written): **56 passed, 0 failed**.
- Hygiene: `engine/tests/fixtures/**` is untouched; every touched file has 0 CRs; each commit carries the
  session trailer; no new `Command::new`; no names, emails or account ids in the new code or tests. The
  section and book ids in the tests were already in the fixtures and the older handler tests. After every run
  the tree is clean.

## Findings

### Important

**I1: R-C1c-10 is half delivered. The slot that files the card still records `WARN`, and the Runs nav reads
"1 warn" for about a new student's first day.**
`engine/src/coursework.rs:994` (in `propose_map_cards`) pushes `"{source}: not mapped; proposed ({stem})"` into
`warnings`. `main_with_fetchers` then derives both statuses from whether `warnings` is empty:

- the per-source step (`:1725`, `let result = if source_warns.is_empty() { "ok" } else { "WARN" }`);
- the run's own status (`:1778`), which the run record and `state/runner-log.md` carry.

`surface::runs_panel` counts every non-`ok` run among the ten most recent in 48 hours (`surface.rs:1500`), and
the nav prints it (`console.js:527`, `r.warn_count + " warn"`).

Scenario: a student leaves the VHL row blank, which D6 allows. The wizard tells them "1 of these will be asked
about in the app". The first slot at Finish files `map-vhl-…` and logs the proposal line, so the coursework run
is `WARN`. From the moment the first-run view hands over, the Runs nav reads "1 warn". It stays until that run
falls out of the ten-most-recent window, about the first day at two slots a day. It comes back for one slot
each time the card's 30-day expiry (`write_map_card`) re-proposes a still-unmapped source.

The ruling asked for exactly the opposite: "It records `ok` with the proposal line, not the WARN." The brief
narrowed it ("The engine side (`collect_cloud` → `propose_map_cards`) is unchanged"), and the implementation
followed the brief. Task 9 removed the every-slot `0 assignments parsed` line, but not the first-slot `WARN`.
That first-slot `WARN` is the "warn from the first minute" defect that run 3 found.

Fix: treat the proposal line as a note, not a warning. Either `propose_map_cards` writes it to a notes list
that `main_with_fetchers` puts into the summary and the source step's message without changing `status` or
`result`, or both status computations exclude the `": not mapped; proposed ("` shape. Pin it with a test at
the status seam: a run whose only line is a proposal ends `ok`, and the line is still in the summary.

Scope notes:
- On the local no-account path (`collect`), the per-row `"section … not in config; skipped"` lines still make
  every slot `WARN`. The Task 9 comment keeps them on purpose, as the only notice. No wizard-made vault takes
  that path, so it can stay.
- Task 9's server half is not deployed to staging yet (away-log H3). Until it is, staging emits the failure
  line on every slot, so the every-slot variant is what a proof run would show today.

### Minor

**M1 (Task 8 × Task 2): a failed account check is marked done.**
`app/src/scheduler.rs:685` records `entitlement (refresh failed: …)` with code 0, which is right for the tray
and the retry ladder. `console.js:630` marks every code-0 step that is not `(skipped:` as `done`. On a first
slot whose refresh fails, the view therefore shows a check on "Checking your account". Meanwhile
`state/runner-log.md` says `entitlement (refresh failed)`, and the next row is "Working out what each task
needs — skipped". That row sits above "Fetching your coursework", because named skips land before the child
steps. More generally, "done" here means "exited 0": a coursework step whose zyBooks login failed also gets a
check, by the engine's always-exit-0 design.

Fix: map a `(refresh failed` name to the `failed` mark. The "didn't finish" line reads codes, so it stays off.
Add the case to `CONSOLE_FAKE` in `wizard-check.py`.

**M2 (merge with C3′; cheap now): two things break or go silent at the C3′ merge.**
- `app/tests/scheduler.rs:996` requires a `push` step in `a_slot_publishes_every_step_it_records_while_it_runs`.
  C3′ deletes `sync_step(cs, "push")`. The test file merges textually clean, so this surfaces only as a red
  test after the merge. Drop `"push"` from the list now; the steps that survive C3′ prove the same mirror.
- `FIRST_RUN_SAYS` (`console.js:604`) has no `sync`. C3′ makes `sync` the slot's first child step, so on a
  merged build the first seconds of a first run show no row in progress. The controller has already recorded
  this for the merge.
  - Cheaper to settle now, while no step is named `sync` and nothing can go wrong: either give it its sentence
    or rule it housekeeping.
  - Optionally, add a static test that every `slot_argv` `args[0]` is either a key of `FIRST_RUN_SAYS` or in
    an explicit housekeeping list in `console.js`. The merge then fails loudly instead of quietly dropping a
    row.
- The `LiveSlot` doc (`scheduler.rs:97`) names `pull` as an example, and `pull` goes away with C3′.

**M3 (Task 6): a section's history is read only from the uids the current fetch returns.**
`coursework.rs:306` counts a group as known only when one of this fetch's items is in `known` or `seen`. That
holds for zyBooks, whose API lists a book in full. The VHL dashboard fixture suggests VHL does not: its rows
cover only the three weeks after its capture date, and it has no row before it.

Scenario: a device off for longer than the dashboard's window, mid-semester. Every row now listed is unseen, so
the known section looks new, and any past-due row it still lists is archived as `imported-past` instead of
arriving as overdue work. That is the one outcome the rule's own comment calls worse than a stale import.

Speculative until one mid-semester dashboard is read. The ruling's own words ("no note in the vault") support
a cheap hardening: `existing_by_uid` already reads every note's frontmatter. Also count a group as known when
any note in `tasks/` or `archive/` carries that `(created_by, course)`. The failed-first-fetch case and the
mapped-later case still work, because neither has a note in the vault.

**M4 (Task 7 × the TypeScript side): nothing assumes the SQL cap equals `DAILY_CAP`; two small notes.**
Checked:
- `judge_pipeline.ts` only reads the boolean `charge_call` returns.
- `gmail-read`'s `READ_CAP` is a per-round bound, and a `capped` outcome ends the round.
- The tests pin `DAILY_CAP` against fake cap stores.
- The engine's `CAPPED_LABEL` carries no number.
- No message quotes a cap.

Two notes:
- (a) The `DAILY_CAP` docblock (`judge_caps.ts:36`) still reads as the whole cap. Add one line pointing at
  `20260923000100`.
- (b) For Quinn, not a defect: the doubled task cap cannot act in the first slot. `judge` stops at
  `enrich::DEFAULT_LIMIT` (50 tasks a run), below the steady-state task cap, so it helps only a second slot on
  day one. Events and email are where the first slot gains.

**M5 (spec drift): the spec still quotes the old summary line.**
`docs/specs/2026-09-22-c1c-first-day-design.md:113` still quotes `coursework: first run — N item(s) already past
were archived`. The code now prints `coursework: N item(s) already past were archived from a source seen for
the first time`, and the R-C1c-6 amendment below that line does not restate it.

**M6 (for Quinn; not re-litigating R-C1c-8): the first-run view stands for minutes, not one minute.**
The view now hides the whole window until `rank` writes the day, and `judge` runs before `rank`: tasks, events,
the Gmail rounds and the rule pull.
- Run 1 measured 3 minutes 40 seconds with calendars and Gmail.
- The doubled event cap lengthens day one's judge a little further.
- The upper bound is the judge's 15-minute batch budget plus the Gmail rounds.

R-C1c-8 costed this at "the day is invisible for the first minute", and the heading still says "about a
minute". That copy was left in the first review. This is recorded so the ruling's cost line matches the
measurement, not as a code change.

### Checked and sound

- **`Scheduler.live` against every writer of `sch.last`.**
  - `refuse()` and the "already running" return never touch `live`.
  - The tick's post-slot rewrite carries the same steps.
  - The install hold sets `running` without touching `live`, as the doc says.
  - Lock order is `running` then `live` at both sites where both are taken, and nothing inverts it.
    `attach_scheduler` takes `last` alone.
  - `live == RunSummary.steps` at the end of a slot, and the tests pin it.
- **Task 8 × Task 2 in progress.** `steps.start("entitlement")` maps to "Checking your account". The runner-log
  lines for the entitlement step are the sanitised pair, and service text never reaches the vault.
- **Task 6 × Task 9, as asked.** An unmapped source never reaches `sync_coursework` on either path. The server
  turns it into a proposal and `continue`s; the local fetchers skip the item. So its uids are never in `known`
  or `seen`, and when its card is approved (`apply_map_cards` runs before the fetch), the next fetch sees a new
  group and archives its past. Task 9 changed only which warning lines are emitted, not which items flow, so
  Task 6's promise still holds. The limits are M3 and the parked two-books-one-course minor. Task 9 makes cards
  the usual way a source arrives late, and a suggested course that matches an existing zyBooks course reaches
  that parked minor.
- **Task 7's migration.**
  - The one upsert is preserved.
  - `min(day)` is served by the primary key's `(account_id, day)` prefix.
  - Nothing prunes `usage_daily`, so the window cannot reopen for an old account.
  - `create or replace` keeps the ACL, and the file re-revokes anyway.
- **The debug commits.** `state_envelope` is the command's body, verbatim; the wizard-check console page is
  sound.

## Merge risk

**Against `c1b-sign-in` (PR #9, tip `00624d9`).** A trial `git merge-tree 773f529 00624d9 39c7a22` shows no
conflicts: `app/src/commands.rs` and `app/tests/commands.rs` change on both sides and merge clean. The merge
brings main's test-clock pins, which fix the accepted `rejecting_and_snoozing_…` failure. Once merged, retire
that exception: expect 0 failures.

Until then, this branch alone also meets a second date failure on **2026-10-01**:
`approving_a_task_proposal_puts_the_task_in_the_returned_state` (fixture `expires: 2026-09-30`), which the
pins fix. So either merge `00624d9` into the branch or land the merge before October.

**Against `c3-sync` (`a650877`).** A trial `git merge-tree 773f529 39c7a22 a650877` shows three conflict hunks,
all small:

1. `scheduler.rs`, the pull block that carries `steps.start("pull")`. Take C3′'s deletion.
2. `scheduler.rs`, the push/backup block. Take C3′'s deletion of the push and its comment, and keep
   `steps.start("backup")` and the backup `match`. Keep `SlotSteps::new(sch)`: it sits outside both hunks.
3. `cloud/supabase/migrations/migrations_test.ts`, the function-count pin. C1c says 22 and C3′ says 27; the
   merged corpus is **28**, with both comment additions. If either side is picked as-is, the Deno job goes red.

`commands.rs`, `state.rs` and `app/tests/scheduler.rs` merge clean. The semantic breaks they carry are M2's
`push` assertion and the missing `sync` sentence.

C3′'s migrations (`20260912*`) sort before C1b's and C1c's (`20260922*`, `20260923*`), so production needs
`db push --include-all`, as already recorded.

What C1c should do **now** to make that merge cheap:
- the M2 test edit;
- settle `sync`'s row;
- optionally, the slot-step-to-sentence static guard.

The scheduler hunks need nothing more: they resolve by deletion.

**Branch state.** `origin/c1c-first-day` is at `679f086`. `8d1d24a` (Task 8's fix round) and `39c7a22` (Task 9)
exist only locally. Before a pilot onboarding, `ingest-coursework` has to be deployed; otherwise I1's every-slot
variant is what students get.

## Re-review of the fix wave (`a749113..1f01563`, ruling R-C1c-final2)

Reviewer: opus, read-only. `a749113` is a clean merge: parents `39c7a22` and `00624d9`, and `git show --cc` is
empty, so nothing was resolved by hand.

### Verdict: READY

Every item the ruling assigned is ADDRESSED and nothing regressed. The residuals are process items already in
the merge-risk section above, none of them code:
- push the branch;
- deploy `ingest-coursework`;
- the C3′ merge's function-count pin of 28.

### Verification (run by the reviewer at `1f01563`)

- `cargo test --workspace --no-fail-fast`, in the foreground: **1295 passed, 0 failed, 4 ignored**. The
  accepted date-test exception is retired by the merge. Warnings: **1 accepted (.rsrc), 2 tallies, 0 other**.
- `python scripts/wizard-check.py`: **ok**.
- Hygiene: 0 CRs in every touched file; each of the three commits carries the session trailer; the tree is
  clean.

### Item by item

- **I1: ADDRESSED.**
  - `is_map_proposal_note` (`engine/src/coursework.rs:1644`) accepts exactly this shape:
    `<source with no whitespace>: not mapped; proposed (<stem>)`, where the stem has no parentheses and closes
    the line.
  - Every other line still WARNs, including one that merely contains "not mapped":
    - the `proposal not written (…)` error;
    - the dry-run "would file a card" line;
    - any longer prefix, because `split_once` then leaves whitespace in the source part.
  - Both status computations (the source step and the run) use `all(...)`, so an empty list is still `ok` and a
    proposal line beside any other warning still WARNs.
  - Pinned end to end on the real filed line: the step, the run record, the runner-log line and
    `runs_panel.warn_count == 0`, plus a dead sibling source and a benign note that both still WARN.
  - Out of the ruling, and left as it is: the no-account local path's `not in config; skipped` lines.
- **M1: ADDRESSED.**
  - `console.js:639` marks a name that starts with `entitlement (refresh failed` as `failed`, with a short note.
  - The "didn't finish" line still reads codes only, and wizard-check pins both halves: failed while the slot
    runs, and no end line once it has ended.
  - The note "will retry" is true: the cache is still missing, so the next slot refreshes again, and so does
    the housekeeping refresh.
- **M2: ADDRESSED.**
  - `sync` maps to "Syncing with your account", with a static pin.
  - The scheduler test no longer names `push`, and the `LiveSlot` doc no longer names `pull`.
  - The optional slot-step guard was not added. It was optional, and the C3′ merge still needs its own eye on
    this table.
- **M3: ADDRESSED, with no regression.**
  - Cost: `note_groups` (`:269`) runs once per call and only when `unheard` is non-empty (`:344`), so
    steady-state runs, where every group has history, never read it. When it does run, it is one more pass of
    the same frontmatter reads that `existing_by_uid` already makes on every call.
  - R-C1c-6's promise holds, because a truly new group has no note carrying its `(created_by, course)`:
    - a failed first fetch leaves nothing behind;
    - rows dropped as unmapped never became notes;
    - the executed map card is moved to `archive/` by `write::delete`, but it carries `created_by: coursework`,
      which is never a vendor group.
  - `a_source_first_seen_after_the_first_slot_archives_its_past` and
    `a_failed_first_fetch_still_archives_on_the_next` still pass beside the new
    `a_group_the_vault_already_holds_a_note_for_is_never_new`.
  - The widening only ever moves a row from archived to active, the safe direction. The parked
    two-books-one-course outcome is unchanged.
- **The implementer's M3 concern (unparseable frontmatter is not counted): accepted, no change.**
  - The miss needs two things at once: every note of a held group unparseable, AND none of the uids now
    listed in `known` or `seen`. `existing_by_uid`'s regex fallback still puts an unparseable note's uid into
    `known`.
  - The cost of that miss is one window of past-due rows archived; a second, lenient parser for the group
    fields would cost more than that.
- **M4: ADDRESSED.** The `DAILY_CAP` docblock names the first-day doubling, the migration that does it, and
  that `p_cap` stays this value.
- **M5: ADDRESSED.** The spec note sits under the old summary-line sentence and gives the per-source wording.
- **M6:** ruled keep; not re-reviewed.
