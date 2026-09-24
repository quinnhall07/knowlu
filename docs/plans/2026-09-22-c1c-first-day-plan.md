# C1c — the first day: implementation plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Make the first day work — the first slot judges, past coursework is archived instead of topping Must do, one course has one name, a blank mapping row says what it costs, and the window says what is happening while the first run goes.

**Architecture:** Five small, file-disjoint changes. One is engine-only (`coursework`'s first run mirrors `ingest`'s R-OB-3 rule). One is the app's scheduler (an entitlement refresh inside the slot, and skipped steps written to `state/runner-log.md` through the engine's own appender). Three are the window and the wizard: the course-code rule that gives a course one slug, a `datalist` plus two sentences on the mapping rows, and a first-run view driven by a new `first_run` block on the `state` envelope.

**Tech Stack:** Rust 1.98 (`stable-x86_64-pc-windows-gnu`), Tauri 2, one Cargo workspace (`engine/` = `knowlu-engine`, `app/` = `knowlu`); the window is a static page (`app/static/index.html` + `console.js`), driven headlessly by `scripts/wizard-check.py` (Playwright, the system Python — there is no `.wv` venv).

**Spec:** `docs/specs/2026-09-22-c1c-first-day-design.md` (decisions D1–D8; §7 is the controller's live proof, not a task; §8 and D9 are out of scope).

**Status: REVIEWED 2026-09-22 — EXECUTE** (the scoped re-review after fix round 1 says *Execute*, none open). AMENDED 2026-09-22 (fix round 1: B1, B2, I1–I4, minors M1–M7 — all taken, none refused).** Review: `docs/reports/2026-09-22-c1c-first-day-plan-review.md`. Four controller rulings decide where the spec contradicts itself or the code; each is recorded at the step it changes as **Ruling R-C1c-plan-1…4**, and a ruling overrides the spec.

## Global Constraints

- **0 warnings is part of green.** `cargo build --workspace` and `cargo test --workspace` from the worktree root; the CI gate prints `warnings: N accepted (.rsrc), N tallies, N other` and the last number must be `0 other`. The one accepted line is the app's `.rsrc merge failure: multiple non-default manifests`.
- **LF everywhere in this repo** (`.gitattributes`: `* text=auto eol=lf`); `*.ps1` are CRLF; `engine/tests/fixtures/**` is `-text` and is never re-encoded.
- **Never regenerate a frozen reference.** The eight Python-written references in `engine/tests/fixtures/` (`golden-today-s1.md`, `golden-today-full.md`, `calendar-snapshot-gcal.md`, `vault-full/state/events.md`, `zybooks-parsed-reference.json`, `vhl-parsed-reference.json`, `run-records-reference.json`, `pyyaml-safe-dump-reference.json`) and the three Rust-generated read-model references (`surface-today-{s1,s1-migrated,full}.json`) are untouched by this plan. If the engine disagrees with one, the engine is wrong.
- **Every `std::process::Command` gets `.no_console()`.** No task in this plan adds a child process; if one is added, it carries it.
- **No secret, account id, email or person's name in code, tests, comments or test names.** Test hosts are the closed loopback `http://127.0.0.1:9/functions/v1`; test accounts are `acc-1`; test addresses are `@example.invalid`.
- **No digit count in copy.** No number is baked into a sentence on the page, in a comment or in this plan: a count on screen is computed at runtime (D6's `N of these will be asked about in the app`), and no task quotes a command or test tally.
- **The whole workspace suite runs in the foreground** — `cargo test --workspace`, never backgrounded, at the end of every task.
- **The one accepted failure on this branch** is the date-sensitive `app/tests/commands.rs::rejecting_and_snoozing_write_the_decision_fields` (fixed on another branch). Every task's test step expects it and nothing else red, until main is merged in.
- **Commits:** `git add <paths>` (never `-A`), then `git commit -F <file>`, and every message ends with the two trailers:
  ```
  Co-Authored-By: Claude Fable 5.1 <noreply@anthropic.com>
  Claude-Session: https://claude.ai/code/session_018EXZqBCHaJBKtYtkNfjj1Z
  ```
- Work only inside the worktree `C:\Users\danie\GitHub\knowlu\.claude\worktrees\c1c-first-day`, on branch `c1c-first-day`. Absolute paths; never `cd`.

## Spec map

| Spec | Task |
| --- | --- |
| §2, D1, D8 | Task 2 |
| §3, D3 | Task 1 |
| §4, D4 | Task 3 |
| §4 D5, §5 D6 | Task 4 |
| §6, D7 | Task 5 |
| D2 | Nothing new is built for it and nothing in this stream writes to disk before Finish: device-side it is already true (`scheduler::spawn` starts the first slot the instant the vault exists) and the half that was missing — the console showing it run — is §6, Task 5. The server-side pre-work is C5's, recorded in spec §8. |
| §7 | The controller's live proof — the hand-off at the end of this plan, not a task. |
| §8, D9 | Out of scope: C5's debt and two spikes, recorded in the spec and left there. |

## Fidelity notes (fix round 1, 2026-09-22)

Every finding in `docs/reports/2026-09-22-c1c-first-day-plan-review.md` is **taken**; none is refused.
Four are taken by a controller ruling that differs from the route the review proposed, and each says so:

| # | Where it landed | Note |
| --- | --- | --- |
| **B1** | Task 3, new Step 5a + Step 7 | `every_enrolled_course_becomes_a_note_the_engine_can_find` is amended, and named in the expected-PASS list. |
| **B2** | Task 5, Steps 1, 5, 8 — **Ruling R-C1c-plan-1** | `build_state` never fails, so the page keys on the presence of `first_run`, not on a failed state. |
| **I1** | Task 4, Steps 1, 6, 7 — **Ruling R-C1c-plan-3** | Taken by the stay-once latch (the shape discovery itself uses) rather than the review's `#wiz-summary` route, so the sentence is read on the panel it is about and §5's `#wiz-map-note` stands. |
| **I2** | Tasks 3 and 4 — **Ruling R-C1c-plan-2** | Taken by the review's own alternative: `Course`/`CourseSeed` gain a fourth field, `label`, so spec §4's `BUI 100 · 202640-BUI-100-101` is rendered without the page reading a code out of a name. |
| **I3** | Task 1, Steps 1, 4, 5 | The summary line moves into `sync_coursework` where the tests can read it, and both spellings are asserted. |
| **I4** | Task 2, Steps 1, 4 — **Ruling R-C1c-plan-4** | Taken by writing the skip line with status `ok` rather than by de-duplicating: a routine skip is routine, `trim_log_lines` ages it out of the 50-line `ok` bucket, and no WARN is ever evicted for one. |
| **M1** | Task 1, Step 4 | `create_dir_all` moved below the dry-run guard. |
| **M2** | Task 1, Step 2 | The expected failure names the four `main` tests, not `vault_with`. |
| **M3** | Task 4, Step 1 | The slice is `wizGo`/`wizStep`, and the comment says so. |
| **M4** | Task 4, Step 4 | The option carries a human label; its value is the ruling's `label \|\| slug`. |
| **M5** | Task 1, Step 4 | The cutoff is an `Option<Date>`, computed only on a first run. |
| **M6** | Task 5, Step 8 | The unbounded 3-second poll is recorded in the comment, with why it is honest. |
| **M7** | Task 1, Step 5 | `archived: 0` on every coursework sync step is stated as intended. |

---

### Task 1: Coursework's first run archives what is already past (spec §3, D3)

**Files:**
- Modify: `engine/src/coursework.rs` — the template block (~184–200), `sync_coursework` (~245–418), `main_with_fetchers` (~1531–1600), and its own `mod tests` (the `sync` helper ~1765, `runnable_vault` ~2689).

**Interfaces:**
- Consumes: `knowlu_engine::ingest::is_first_run(&Path) -> bool` (the absence of `state/today.md`), `crate::ingest::IMPORTED_PAST` (`"imported-past"`), `crate::cli::local_now(&Path) -> jiff::civil::DateTime`.
- Produces: `pub fn sync_coursework(assignments: &[Assignment], vault: &Path, today: Option<Date>, dry_run: bool, ctx: Option<&WriteContext>, journal: Option<&mut Journal>, first_run: bool) -> Result<Vec<String>, SourceError>` — the `first_run` flag is new and last, exactly where `ingest::sync_tasks` carries its own.

- [ ] **Step 1: Write the failing tests**

In `engine/src/coursework.rs`, inside `mod tests`, add the helper next to `fn sync(...)` (~1765) and the four tests after it:

```rust
    /// [`sync`] on a vault that has never been through a whole slot (D3).
    fn sync_first(items: &[Assignment], vault: &Path, dry_run: bool) -> Vec<String> {
        sync_coursework(items, vault, Some(date(2026, 8, 25)), dry_run, None, None, true)
            .expect("the sync completed")
    }

    fn past_and_future() -> [Assignment; 3] {
        [
            make("zybooks:p1", "cs-100-hw-01", "CS 100 HW 01", date(2026, 8, 20).at(23, 59, 0, 0), 0),
            make("zybooks:p2", "cs-100-hw-02", "CS 100 HW 02", date(2026, 8, 24).at(23, 59, 0, 0), 0),
            make("zybooks:f1", "cs-100-hw-03", "CS 100 HW 03", date(2026, 8, 26).at(23, 59, 0, 0), 0),
        ]
    }

    /// D3: a vendor's semester reaches backwards, and a vault born today has no history to
    /// reconcile against — the first live onboarding put eight already-past zyBooks tasks at the
    /// top of Must do. On a FIRST run they go straight to `archive/`, under the same field and the
    /// same word `ingest` uses (R-OB-3), and everything else is created as usual.
    #[test]
    fn a_first_run_archives_what_is_already_past_and_creates_the_rest() {
        let vault = vault_with("cwfirstpast");
        let items = past_and_future();
        let log = sync_first(&items, &vault, false);
        assert_eq!(
            log,
            vec![
                "archived (imported-past) cs-100-hw-01".to_string(),
                "archived (imported-past) cs-100-hw-02".to_string(),
                "created cs-100-hw-03".to_string(),
                // §3's summary line, ingest's in coursework's words (I3).
                "coursework: first run — 2 item(s) already past were archived".to_string(),
            ]
        );
        assert!(vault.join("tasks").join("cs-100-hw-03.md").is_file());
        assert!(!vault.join("tasks").join("cs-100-hw-01.md").exists());
        let meta = meta_of(&vault.join("archive").join("cs-100-hw-01.md"));
        assert_eq!(field(&meta, "status"), "archived");
        assert_eq!(field(&meta, "archived_reason"), crate::ingest::IMPORTED_PAST);
        assert_eq!(field(&meta, "needs_enrichment"), "false");
        // The vendor's own numbers survive the move: an archived import is still the item the
        // vendor described, and `progress` is seeded once at creation and never rewritten.
        assert_eq!(field(&meta, "effort_source"), "inferred");
        assert_eq!(field(&meta, "progress"), "0");
        // All three uids are in the seen ledger, so a second run re-creates nothing.
        let seen = load_seen(&vault);
        for item in &items {
            assert!(seen.contains(&item.uid), "{} is not in the seen ledger", item.uid);
        }
        let again = sync_first(&items, &vault, false);
        assert!(
            again.iter().all(|l| l.starts_with("coursework: first run")),
            "a second run changed something: {again:?}"
        );
        assert_eq!(std::fs::read_dir(vault.join("tasks")).unwrap().count(), 1);
        assert_eq!(std::fs::read_dir(vault.join("archive")).unwrap().count(), 2);
        let _ = std::fs::remove_dir_all(&vault);
    }

    /// …and a run that is not the first creates all three, unchanged. The rule is R-OB-3's, not a
    /// new policy about old work: a vault mid-semester keeps importing what a vendor still lists.
    #[test]
    fn a_later_run_still_creates_an_item_that_is_already_past() {
        let vault = vault_with("cwlaterpast");
        let log = sync(&past_and_future(), &vault, false);
        assert_eq!(log.iter().filter(|l| l.starts_with("created ")).count(), 3);
        assert!(log.iter().all(|l| !l.contains("imported-past")), "{log:?}");
        assert_eq!(std::fs::read_dir(vault.join("archive")).unwrap().count(), 0);
        let _ = std::fs::remove_dir_all(&vault);
    }

    /// A dry run says what it would do and writes nothing — the promise `sync_coursework`'s dry
    /// run already makes, extended to the archive path.
    #[test]
    fn a_first_run_dry_run_names_the_archive_and_writes_nothing() {
        let vault = vault_with("cwfirstdry");
        let log = sync_first(&past_and_future(), &vault, true);
        assert_eq!(
            log[0],
            "would archive (imported-past) cs-100-hw-01 (due 2026-08-20T23:59)"
        );
        assert_eq!(log[2], "would create cs-100-hw-03 (due 2026-08-26T23:59)");
        // Both spellings are counted (I3): a dry run that reported zero would be a lie about what
        // the real run is about to do.
        assert_eq!(log[3], "coursework: first run — 2 item(s) already past were archived");
        assert_eq!(std::fs::read_dir(vault.join("tasks")).unwrap().count(), 0);
        assert_eq!(std::fs::read_dir(vault.join("archive")).unwrap().count(), 0);
        assert!(load_seen(&vault).is_empty());
        let _ = std::fs::remove_dir_all(&vault);
    }

    /// The run record counts it: the Runs view's sync step says where the work went, so a first day
    /// shorter than the vendor's list has a number behind it. (The summary LINE is
    /// `sync_coursework`'s and is asserted by the three tests above — `main` returns an exit code,
    /// and that line is not the run-log summary, which is built separately at `:1604`.)
    ///
    /// `main` passes `today: None`, so the cutoff is the real clock — the two items are built
    /// around it rather than pinned to a date that is already in the past by the time anyone runs
    /// this, which would archive both and prove nothing.
    #[test]
    fn a_first_run_counts_and_names_what_it_archived() {
        let vault = runnable_vault(
            "cwfirstruns",
            "timezone: America/Chicago\ncoursework:\n  zybooks:\n    enabled: true\n",
        );
        // `runnable_vault` gives every other `main` test a vault that has already been ranked;
        // this one is about the run that has not, so the page `rank` writes is removed.
        let _ = std::fs::remove_file(vault.join("state").join("today.md"));
        let now = jiff::Zoned::now().date();
        let items = vec![
            make("zybooks:p1", "cs-100-hw-01", "CS 100 HW 01", now.yesterday().unwrap().at(23, 59, 0, 0), 0),
            make("zybooks:f1", "cs-100-hw-02", "CS 100 HW 02", now.tomorrow().unwrap().at(23, 59, 0, 0), 0),
        ];
        let both = |_: &Mapping, _: &TimeZone, _: &mut Vec<String>| -> Result<Vec<Assignment>, SourceError> {
            Ok(items.clone())
        };
        let fetchers: [(&str, Fetcher); 1] = [("zybooks", &both)];
        assert_eq!(main_with_fetchers(&vault, false, "local-runner", None, Some(&fetchers)), 0);
        let day = crate::runs::Runs::new(&vault).read(None);
        let sync_step = day
            .iter()
            .find(|r| r.get("name").and_then(|v| v.as_str()) == Some("sync"))
            .expect("a sync step");
        assert_eq!(sync_step["counts"]["archived"], 1);
        assert_eq!(sync_step["counts"]["created"], 1);
        let _ = std::fs::remove_dir_all(&vault);
    }
```

- [ ] **Step 2: Run them to verify they fail**

Run: `cargo test -p knowlu-engine --lib coursework::`
Expected: FAIL — `sync_coursework` takes six arguments, not seven, so `sync_first` does not compile. (M2: once it does, the four `main` tests over `runnable_vault` also fail, because that fixture writes no `state/today.md` and their assignments' due dates are already past — every one of them is a first run until Step 6.)

- [ ] **Step 3: Add the archived twin of the note template**

In `engine/src/coursework.rs`, immediately after `NOTE_TEMPLATE` (~200):

```rust
/// The archived twin of [`NOTE_TEMPLATE`] (D3), and `ingest::IMPORTED_PAST_TEMPLATE`'s rule in
/// coursework's own shape: the vendor's fields all stay — `progress` is still seeded once from
/// VHL's `percentage_complete` — and the three lines that say why the note is in `archive/` are
/// added. A second template rather than a substitution on the first, for the reason ingest gives:
/// the two differ in what they MEAN, and a reader should not have to diff them to see it.
///
/// `needs_enrichment: false` is stated although the active template states nothing (a vendor's
/// item arrives fully resolved): an archived import is not work the judge owes, and saying so is
/// cheaper than a reader wondering.
const IMPORTED_PAST_TEMPLATE: &str = "---\n\
title: {title}\n\
course: {course}\n\
domain: school\n\
due: {due}\n\
effort_hours: {effort_hours}\n\
effort_confidence: {effort_confidence}\n\
effort_source: {effort_source}\n\
importance: {importance}\n\
importance_reason: {importance_reason}\n\
status: archived\n\
archived_reason: imported-past\n\
progress: {progress}\n\
needs_enrichment: false\n\
created_by: {created_by}\n\
source_uid: {uid}\n\
---\n\
\n\
{body}\n";
```

- [ ] **Step 4: Take the flag, compute the cutoff, and archive on a first run**

In `sync_coursework` (~245), add the parameter after `journal`:

```rust
    journal: Option<&mut Journal>,
    // D3 / R-OB-3. `true` only on a vault that has never been through a whole slot —
    // `ingest::is_first_run`, the absence of `state/today.md`, which is the predicate `ingest`
    // already shares with `app/src/scheduler.rs::needs_first_run`. `main` computes it; every test
    // that predates D3 passes `false`, which is the behaviour it was written against.
    first_run: bool,
) -> Result<Vec<String>, SourceError> {
```

Below, beside the existing `stamp`, add the cutoff (leave `stamp` exactly as it is — the date it writes into the seen ledger is a contract with existing vaults):

```rust
    // D3 measures "already past" in the VAULT's timezone (`cli::vault_zone`, through
    // `cli::local_now`), not the machine's: a student travelling must not have a day's work
    // archived out from under them. The seen-ledger stamp above keeps its own clock — that value
    // is a contract with existing vaults and nothing in R-OB-3 asks for it to change.
    //
    // `Some` only on a first run (M5): `local_now` reads and parses `config/ingest.yaml`, and every
    // run after the first would pay for a date nothing below ever reads.
    let cutoff = if first_run {
        Some(today.unwrap_or_else(|| crate::cli::local_now(vault).date()))
    } else {
        None
    };
```

Then, in the create path, immediately after the `if seen.contains(&item.uid) { … }` guard and before `let mut path = tasks_dir.join(…)` (~368):

```rust
        // D3 / R-OB-3: a vendor's semester reaches backwards, and a vault born today has no
        // history to reconcile against. On a FIRST run an item already past is recorded as seen and
        // written straight into `archive/`. **Not skipped** — skipping leaves the uid unseen and
        // the next run creates it. **Not created-then-deleted** — that is two journal records and a
        // note that briefly ranks. One `create` into `archive/`, one `record_seen`, one line.
        //
        // Strictly before TODAY, never before *now*: an item due at 23:59 today is today's work,
        // and the one thing worse than importing a stale task is archiving a live one.
        //
        // `cutoff` is `Some` exactly when `first_run` is (M5), so this is the whole predicate.
        if cutoff.is_some_and(|c| item.due.date() < c) {
            let archive_dir = vault.join("archive");
            let mut path = archive_dir.join(format!("{}.md", item.slug));
            let mut suffix = 2;
            while path.exists() {
                path = archive_dir.join(format!("{}-{suffix}.md", item.slug));
                suffix += 1;
            }
            let stem = path
                .file_stem()
                .map(|s| s.to_string_lossy().into_owned())
                .unwrap_or_default();
            if dry_run {
                log.push(format!("would archive (imported-past) {stem} (due {new_due})"));
                continue;
            }
            // M1: below the dry-run guard, never above it — a `--dry-run` that creates a folder in
            // the vault has already broken the promise it exists to make.
            let _ = std::fs::create_dir_all(&archive_dir);
            let text = IMPORTED_PAST_TEMPLATE
                .replace("{title}", &json_quoted(&item.title))
                .replace(
                    "{course}",
                    &match item.course.as_deref().filter(|c| !c.is_empty()) {
                        Some(course) => json_quoted(course),
                        None => "null".to_string(),
                    },
                )
                .replace("{due}", &new_due)
                .replace("{effort_hours}", &py_float(item.effort_hours))
                .replace("{effort_confidence}", &item.effort_confidence)
                .replace("{effort_source}", &item.effort_source)
                .replace("{importance}", &item.importance.to_string())
                .replace("{importance_reason}", &json_quoted(&item.importance_reason))
                .replace("{progress}", &item.progress.to_string())
                .replace("{created_by}", &item.created_by)
                .replace("{uid}", &json_quoted(&item.uid))
                .replace("{body}", &item.body);
            let target = crate::ids::rel(vault, &path);
            crate::write::create(vault, &target, &text, &item_ctx, journal, None)
                .map_err(|err| SourceError::Failed(format!("{err}")))?;
            log.push(format!("archived (imported-past) {stem}"));
            known.insert(item.uid.clone(), path);
            record_seen(vault, &item.uid, &item.title, &stamp)
                .map_err(|err| SourceError::Failed(format!("{err}")))?;
            continue;
        }
```

And §3's summary line, at the very end of the function, immediately before `Ok(log)`:

```rust
    // D3, ingest's line in coursework's words (`ingest::run_lines`, engine/src/ingest.rs:945).
    // Pushed HERE rather than in `main_with_fetchers` (I3) so a test can read it: `main` returns an
    // exit code, and this line is not the run-log summary, which is built separately at `:1604`.
    //
    // Both spellings are counted, so a `--dry-run` reports the number the real run would archive
    // rather than zero. One deliberate divergence from ingest: `main` only calls this function when
    // a source returned something, so a fetch that came back empty says nothing at all, where
    // ingest reports `0 item(s)` — and an empty coursework parse is already a failure with its own
    // warning (the coursework spec §9), which is the line that matters on that run.
    if first_run {
        let archived = log.iter().filter(|l| l.contains("(imported-past)")).count();
        log.push(format!("coursework: first run — {archived} item(s) already past were archived"));
    }
    Ok(log)
```

- [ ] **Step 5: Thread the flag and the count through `main_with_fetchers`**

In `main_with_fetchers` (~1531), beside `let today = jiff::Zoned::now().date();`:

```rust
    // D3: the same predicate `ingest` uses, read BEFORE this pass writes anything — `rank` writes
    // `state/today.md` at the END of the slot, so coursework and ingest agree within one slot about
    // which run is the first one.
    let first_run = crate::ingest::is_first_run(vault);
```

In the closure, replace the sync call with:

```rust
        if !assignments.is_empty() {
            log.extend(sync_coursework(&assignments, vault, None, dry_run, Some(&ctx), None, first_run)?);
        }
```

And in the run-record block, extend the sync step's counts (~1583). **The counts vector is built on
every coursework run, so every sync step from now on carries a fourth key — `archived: 0` on a
run that archived nothing (M7). That is intended and consistent with `created`/`updated`/`skipped`;
no frozen reference is touched, because `the_run_records_match_python_byte_for_byte`
(`engine/src/runs.rs:930`) builds its own step list:**

```rust
        let created = log.iter().filter(|l| l.starts_with("created")).count() as i64;
        let updated = log.iter().filter(|l| l.starts_with("updated")).count() as i64;
        let skipped = log.iter().filter(|l| l.starts_with("skipped")).count() as i64;
        // D3: an archived import is neither a creation nor a skip, and a first day that looks empty
        // needs a number that says where the work went.
        let archived = log.iter().filter(|l| l.starts_with("archived (imported-past)")).count() as i64;
        let counts = vec![("created", created), ("updated", updated), ("skipped", skipped), ("archived", archived)];
```

- [ ] **Step 6: Fix the two callers inside the test module**

`fn sync(...)` (~1765) passes the new flag as `false`, and `runnable_vault` (~2689) gives its vaults the page a ranked vault has:

```rust
    fn sync(items: &[Assignment], vault: &Path, dry_run: bool) -> Vec<String> {
        sync_coursework(items, vault, Some(date(2026, 8, 25)), dry_run, None, None, false)
            .expect("the sync completed")
    }
```

```rust
    fn runnable_vault(name: &str, config: &str) -> PathBuf {
        let vault = scratch(name);
        for folder in ["config", "state", "tasks"] {
            std::fs::create_dir_all(vault.join(folder)).unwrap();
        }
        write_note(&vault.join("config").join("ingest.yaml"), config);
        // D3: these tests are about a vault that has already been through a slot — per-source
        // counts, the run-log summary, the via and run_id on a journal record. `rank` writes this
        // page at the end of every slot, and without it every one of them would be a FIRST run and
        // would archive its fixture's already-past assignments instead of creating them.
        write_note(&vault.join("state").join("today.md"), "# Today\n");
        vault
    }
```

- [ ] **Step 7: Run the engine tests**

Run: `cargo test -p knowlu-engine --lib coursework::`
Expected: PASS, including the seven `main`/`runnable_vault` tests that predate this task.

- [ ] **Step 8: Run the whole suite**

Run: `cargo test --workspace`
Expected: PASS at 0 warnings, with `app/tests/commands.rs::rejecting_and_snoozing_write_the_decision_fields` as the one accepted failure.

- [ ] **Step 9: Commit**

```bash
cat > /tmp/c1c-task1.txt <<'EOF'
feat(engine): coursework's first run archives what is already past

D3: an item whose due date is before today goes straight to archive/ on a
vault that has never been through a slot, under ingest's own field
(archived_reason: imported-past) and the predicate the two already share.
sync_coursework takes ingest's `first_run` flag; the run record counts
`archived`; the summary line mirrors ingest's.

Co-Authored-By: Claude Fable 5.1 <noreply@anthropic.com>
Claude-Session: https://claude.ai/code/session_018EXZqBCHaJBKtYtkNfjj1Z
EOF
git add engine/src/coursework.rs
git commit -F /tmp/c1c-task1.txt
```

---

### Task 2: The first slot refreshes the entitlement, and a skipped step reaches the vault (spec §2, D1, D8)

**Files:**
- Modify: `app/src/scheduler.rs` — `run_slot_inner`, just above `let est = entitlement_state(cs);` (~608) and just below the judge-skip push (~615).
- Test: `app/tests/scheduler.rs` — two new tests, and one amendment to `a_vault_with_an_account_and_no_entitlement_records_the_judge_skip_and_stays_green` (~607).

**Interfaces:**
- Consumes: `crate::account::cloud_config(&Path) -> Result<CloudConfig, String>`, `crate::account::cache_path(&Path) -> PathBuf`, `crate::account::refresh_entitlement(&Path, &Path) -> Result<EntitlementCache, String>` (one HTTPS round trip under `account::TIMEOUT`), and `knowlu_engine::cli::append_run_log(vault: &Path, runner: &str, status: &str, summary: &str, when: Option<jiff::civil::DateTime>) -> std::io::Result<()>` — **already `pub`**, so D8 needs no new engine entry point; it renders through `runs::log_line` and nothing else (F11).
- Produces: two new step labels in `RunSummary.steps` — `entitlement (refreshed)` and `entitlement (refresh failed: <reason>)`, both with exit code `0` — and, in the vault, one `- <YYYY-MM-DD HH:MM> local ok <note>` line per skipped step (**Ruling R-C1c-plan-4**: the status is `ok`, not the spec's implied `skip`; see Step 4).

- [ ] **Step 1: Write the failing tests**

In `app/tests/scheduler.rs`, after `an_entitlement_past_the_grace_skips_judge_by_name_and_keeps_the_slot_green`:

```rust
/// §2 / D1: the first slot of a brand-new cloud vault used to lose a race. `scheduler::spawn`
/// starts the first slot and the launch entitlement refresh on two threads, and on the first live
/// onboarding the slot reached the judge decision before the refresh had written its cache — so the
/// judge was skipped for want of an answer that was already on its way. A vault that has an account
/// and has never cached an entitlement now refreshes it INSIDE the slot, before the decision.
#[test]
fn a_first_slot_refreshes_the_entitlement_before_it_decides_about_judge() {
    // The refresh reaches `valid_access_token_at`, which reads the real Credential Manager
    // (CLAUDE.md: every test that touches the store takes this file's lock).
    let _credman_guard = CREDMAN_LOCK.lock().unwrap_or_else(|e| e.into_inner());
    let v = scratch("entrefresh");
    std::fs::write(
        v.join("config").join("runners.yaml"),
        format!("runners:\n  - name: local\n    times: [\"12:00\"]\n    tz: America/Chicago\n    grace_minutes: 20\n    device: {}\n    scheduler: app\n", knowlu_engine::journal::device_name()),
    ).unwrap();
    std::fs::write(
        v.join("config").join("cloud.yaml"),
        "api_base: 'http://127.0.0.1:9/functions/v1'\nanon_key: 'anon'\nsession_credential_target: 'knowlu/profile_x/session'\naccount_id: 'acc-1'\n",
    ).unwrap();
    let cs = open(&v, "entrefresh");
    assert!(!knowlu::account::cache_path(&cs.data_dir).exists(), "nothing has been cached yet");
    let sch = Scheduler::default();
    let fake = std::env::temp_dir().join(format!("qo-sched-entrefresh-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&fake);
    std::fs::create_dir_all(&fake).unwrap();
    let _guard = ENGINE_ENV_LOCK.lock().unwrap_or_else(|e| e.into_inner());
    let _env = EnvSeam::set(&[
        ("KNOWLU_ENGINE_EXE", std::ffi::OsStr::new("cmd")),
        ("LOCALAPPDATA", fake.as_os_str()),
        ("KNOWLU_API_BASE", std::ffi::OsStr::new("http://127.0.0.1:9/functions/v1")),
    ]);
    let s = run_slot_inner(&cs, &sch, None, false);
    let named: Vec<String> = s.steps.iter().map(|(n, _)| n.clone()).collect();
    let refresh = named.iter().position(|n| n.starts_with("entitlement (refresh failed:"))
        .unwrap_or_else(|| panic!("no entitlement step: {named:?}"));
    let judge = named.iter().position(|n| n == "judge (skipped: no entitlement)")
        .unwrap_or_else(|| panic!("no judge skip: {named:?}"));
    assert!(refresh < judge, "the refresh is attempted BEFORE the decision: {named:?}");
    assert_eq!(s.steps[refresh].1, 0, "a service that could not be reached is not a failed slot");
    assert!(s.engine_ok, "an entitlement refresh must never paint the tray amber: {:?}", s.steps);
    // D8: and the skip reaches the file a student can open, in the engine's own format — with
    // status `ok`, because a skip is routine (Ruling R-C1c-plan-4).
    let log = knowlu_engine::pystr::read_text(&v.join("state").join("runner-log.md")).unwrap();
    assert!(log.contains("local ok judge (skipped: no entitlement)"), "{log}");
    assert!(!log.contains("local skip"), "a skip must not be a non-ok line: {log}");
    let _ = std::fs::remove_dir_all(&fake);
    let _ = std::fs::remove_dir_all(&v);
}

/// …and a cache that already exists is never refreshed here, even a stale one: the six-hourly
/// housekeeping refresh and the 72-hour grace own that question, and a slot is not the place to
/// re-ask one that has an answer.
#[test]
fn a_cached_entitlement_is_never_refreshed_inside_the_slot() {
    let _credman_guard = CREDMAN_LOCK.lock().unwrap_or_else(|e| e.into_inner());
    let v = scratch("entcached");
    std::fs::write(
        v.join("config").join("runners.yaml"),
        format!("runners:\n  - name: local\n    times: [\"12:00\"]\n    tz: America/Chicago\n    grace_minutes: 20\n    device: {}\n    scheduler: app\n", knowlu_engine::journal::device_name()),
    ).unwrap();
    std::fs::write(
        v.join("config").join("cloud.yaml"),
        "api_base: 'http://127.0.0.1:9/functions/v1'\nanon_key: 'anon'\nsession_credential_target: 'knowlu/profile_x/session'\naccount_id: 'acc-1'\n",
    ).unwrap();
    let cs = open(&v, "entcached");
    knowlu::account::save_cache(&cs.data_dir, &knowlu::account::EntitlementCache {
        status: "active".into(), current_period_end: None, plan: Some("monthly".into()),
        checked_at: knowlu_engine::journal::now_ts(None),
    }).unwrap();
    let sch = Scheduler::default();
    let fake = std::env::temp_dir().join(format!("qo-sched-entcached-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&fake);
    std::fs::create_dir_all(&fake).unwrap();
    let _guard = ENGINE_ENV_LOCK.lock().unwrap_or_else(|e| e.into_inner());
    let _env = EnvSeam::set(&[
        ("KNOWLU_ENGINE_EXE", std::ffi::OsStr::new("cmd")),
        ("LOCALAPPDATA", fake.as_os_str()),
        ("KNOWLU_API_BASE", std::ffi::OsStr::new("http://127.0.0.1:9/functions/v1")),
    ]);
    let s = run_slot_inner(&cs, &sch, None, false);
    let named: Vec<String> = s.steps.iter().map(|(n, _)| n.clone()).collect();
    assert!(!named.iter().any(|n| n.starts_with("entitlement (")), "{named:?}");
    // Nothing was skipped, so nothing was written: D8 adds a line for a skip, not for every slot.
    let log = knowlu_engine::pystr::read_text(&v.join("state").join("runner-log.md")).unwrap_or_default();
    assert!(!log.contains("(skipped:"), "{log}");
    let _ = std::fs::remove_dir_all(&fake);
    let _ = std::fs::remove_dir_all(&v);
}
```

Amend the existing `a_vault_with_an_account_and_no_entitlement_records_the_judge_skip_and_stays_green` (~607) — the slot now reads the real Credential Manager on that vault, so it takes the file's lock. Add as its first line, above `let v = scratch("noentitlement");`:

```rust
    // D1: this vault has an account and no cache, so the slot now attempts one refresh, which
    // reaches `valid_access_token_at` and the real Credential Manager (CLAUDE.md).
    let _credman_guard = CREDMAN_LOCK.lock().unwrap_or_else(|e| e.into_inner());
```

- [ ] **Step 2: Run them to verify they fail**

Run: `cargo test -p knowlu --test scheduler`
Expected: FAIL — `no entitlement step: [...]` in the first test (no `entitlement (…)` step exists), and the second fails on the missing `state/runner-log.md` assertion only if the first has already landed; the amended test still passes.

- [ ] **Step 3: Refresh the entitlement inside the slot**

In `app/src/scheduler.rs`, in `run_slot_inner`, immediately above `let est = entitlement_state(cs);`:

```rust
    // §2 / D1: a cloud vault whose entitlement has NEVER been cached refreshes it here,
    // synchronously, before the judge decision. `scheduler::spawn` starts the first slot and the
    // launch refresh on two threads, and the first live onboarding proved the slot can win: the
    // judge was skipped for want of a cache that landed eleven seconds later, and the day's tasks
    // sat unenriched until the next slot. One HTTPS round trip, under `account::TIMEOUT`.
    //
    // A cache that exists is never refreshed here, even a stale one — the six-hourly housekeeping
    // refresh and the 72-hour grace own that. A failure leaves no cache and `judge_plan_for` names
    // the skip exactly as it does today: only a refusal from the service, never a missing cache,
    // is what a student reads as "no entitlement".
    if crate::account::cloud_config(&cs.vault).is_ok() && !crate::account::cache_path(&cs.data_dir).exists() {
        // Exit code **0** on both arms, like every other named step here: an account service that
        // could not be reached is not a slot that failed, and an amber tray twice a day for a
        // network is the wrong answer.
        let step = match crate::account::refresh_entitlement(&cs.vault, &cs.data_dir) {
            Ok(_) => "entitlement (refreshed)".to_string(),
            Err(e) => format!("entitlement (refresh failed: {e})"),
        };
        steps.push((step, 0));
    }
    let est = entitlement_state(cs);
```

- [ ] **Step 4: Write the skipped steps to `state/runner-log.md`**

Immediately after the `if let JudgePlan::Skip(note) = &judge { steps.push(…); }` block and before `match engine_exe() {`:

```rust
    // D8: a skipped step reaches the vault too. `RunSummary` lives in this process and the Runs
    // view reads the run record the ENGINE writes — a step this app left out appears in neither, so
    // the first live onboarding had nothing on screen and nothing on disk saying the judge never
    // ran. One line each, in the engine's own format, through the engine's own appender
    // (`cli::append_run_log` → `runs::log_line`, the single renderer, F11), so a line this app
    // wrote and a line the engine wrote are the same bytes.
    //
    // The two named skips only — a `pull (skipped: busy)` is a transient lock collision between
    // this slot and the housekeeping thread, not something a student opens a file to read.
    //
    // **Ruling R-C1c-plan-4: the status is `ok`, not `skip`.** `cli::line_status` reads the fifth
    // token and `trim_log_lines` keeps only the newest hundred NON-`ok` lines, for one stated
    // reason: a failure must not age out while routine runs keep flowing. A skip repeats every slot
    // — twice a day, forever, on a vault with no feed — and is not a problem, so filing it as
    // non-`ok` would spend a failure's budget on routine. `ok` puts it in the fifty-line routine
    // bucket, where it ages out like every other ordinary line. The step itself is still a step in
    // the run record, which is what the Runs view reads.
    //
    // Under `vault_io`, and taken here rather than around the loop below: `vault_io` is never held
    // across a child process (see `ConsoleState::vault_io`), which may run for twenty minutes.
    let skips: Vec<String> = steps
        .iter()
        .filter(|(n, _)| n.starts_with("ingest (skipped:") || n.starts_with("judge (skipped:"))
        .map(|(n, _)| n.clone())
        .collect();
    if !skips.is_empty() {
        let _io = lock(&cs.vault_io);
        for note in &skips {
            let _ = knowlu_engine::cli::append_run_log(&cs.vault, "local", "ok", note, None);
        }
    }
```

- [ ] **Step 5: Run the scheduler tests**

Run: `cargo test -p knowlu --test scheduler`
Expected: PASS, including every entitlement and telemetry test that predates this task.

- [ ] **Step 6: Run the whole suite**

Run: `cargo test --workspace`
Expected: PASS at 0 warnings, with `rejecting_and_snoozing_write_the_decision_fields` the one accepted failure.

- [ ] **Step 7: Commit**

```bash
cat > /tmp/c1c-task2.txt <<'EOF'
feat(app): the first slot refreshes the entitlement, and a skip reaches the vault

D1: a cloud vault with no cached entitlement refreshes it inside the slot,
before the judge decision, as a named step with exit code 0 — the first slot
no longer depends on winning the race against the launch refresh.
D8: ingest and judge skips are appended to state/runner-log.md through the
engine's own appender, so the file a student opens says what was left out.

Co-Authored-By: Claude Fable 5.1 <noreply@anthropic.com>
Claude-Session: https://claude.ai/code/session_018EXZqBCHaJBKtYtkNfjj1Z
EOF
git add app/src/scheduler.rs app/tests/scheduler.rs
git commit -F /tmp/c1c-task2.txt
```

---

### Task 3: One course code, one slug (spec §4, D4)

> **Ruling R-C1c-plan-2 (review I2).** Spec §4 asks for two things that cannot both be true: the
> wizard row shows `BUI 100 · 202640-BUI-100-101`, and "the `Course` struct gains nothing" — but for
> a Blackboard course the struct's `code` is the opaque `_404752_1`, and the page is forbidden from
> reading a code out of a name (that rule lives in Rust, and a second copy would drift from it in
> silence). The ruling: **`lms_link::Course` and `scaffold::CourseSeed` gain a fourth field,
> `label: String`** — the human code when one was read, empty otherwise — filled once in
> `courses_from_json` and carried through the plan. The row, the note's title and the datalist all
> read it, and nothing recomputes it.

**Files:**
- Modify: `app/src/scaffold.rs` — `code_in_name` (~287) becomes `course_code_in_name`, `CourseSeed` (~271) gains `label`, `course_fragments` (~305) calls the new function, the course-note writer (~650–665) titles by the label and carries `name:`.
- Modify: `app/src/lms_link.rs` — `Course` (~461) gains `label`, and `courses_from_json` (~502–508) fills it.
- Modify: `app/src/onboarding.rs` — the `CourseSeed` rebuild (~632–638) carries the label through.
- Modify: `app/static/console.js` — `renderCourses` (~1999).
- Modify: `scripts/wizard-check.py` — the fake `capture_courses` envelope (~69).
- Test: `app/tests/scaffold.rs` (seven `CourseSeed` literals across four tests, one amended assertion loop, one new test), `app/tests/lms_link.rs` (one new test, one amended round trip), `app/tests/onboarding.rs` (two literals).

**Interfaces:**
- Consumes: `scaffold::suggest_course(&str) -> Option<String>` (unchanged, LMS ids only), `knowlu_engine::ingest::slugify(&str) -> String`.
- Produces: `pub fn course_code_in_name(name: &str) -> Option<String>` in `app/src/scaffold.rs` — `"202640-BUI-100-101"` → `Some("BUI 100")`, `"CS 100 Intro"` → `Some("CS 100")`, `"UACS100Fall2026"` → `None`. Task 4 does **not** call it (the page never reads codes; that is Rust's job).
- Produces: `lms_link::Course { code: String, name: String, slug: String, label: String }` and `scaffold::CourseSeed { code: String, name: String, slug: String, label: String }` — one shape in two crates' worth of code, as they already are, with `#[serde(default)]` on `label` so a page or a stored plan that omits it is not a refusal.
- Produces: a course note whose frontmatter is `title` (the label when it is set, else the LMS name), `name` (always the LMS name), `slug`, `code` (still the LMS's own key — the `course_map` fragment `ingest::match_course` matches a UID against), `status`.

- [ ] **Step 1: Write the failing tests**

In `app/tests/scaffold.rs`, after `a_zybook_code_suggests_the_course_it_obviously_is`:

```rust
/// D4: one course code, one slug. Blackboard names a course by term, department, number and
/// section (`202640-BUI-100-101`), which `suggest_course` refuses because of the dash — so the
/// vault used to hold `courses/202640-bui-100-101.md` beside a typed `BUI 100` that could never
/// meet it. The name's own code is read, with no institution-prefix peel: the letters immediately
/// before the number are the subject, and there is nothing glued in front of them to peel.
#[test]
fn a_course_name_gives_up_the_code_its_school_wrote_into_it() {
    use knowlu::scaffold::course_code_in_name;
    use knowlu_engine::ingest::slugify;
    assert_eq!(course_code_in_name("202640-BUI-100-101").as_deref(), Some("BUI 100"));
    assert_eq!(course_code_in_name("MATH-125-001").as_deref(), Some("MATH 125"));
    assert_eq!(course_code_in_name("CS 100 Intro to Computer Science").as_deref(), Some("CS 100"));
    assert_eq!(course_code_in_name("PSYC 101H Honors").as_deref(), Some("PSYC 101H"));
    // No code in it at all, and never a guess: an invented fragment matches somebody else's course.
    assert_eq!(course_code_in_name("Biology"), None);
    assert_eq!(course_code_in_name(""), None);
    // A number that is not three digits is not a course number.
    assert_eq!(course_code_in_name("MATH-1250-001"), None);
    // The glued form is `suggest_course`'s, not this one's — reading it here would peel nothing
    // and answer `UACS 100`, which is nobody's course.
    assert_eq!(course_code_in_name("UACS100Fall2026"), None);
    assert_eq!(slugify("BUI 100"), "bui-100");
}

/// …and the note that course gets is titled by the code, keeps the school's own name, and keeps the
/// LMS's key: the title is what a student reads, `name:` is what their LMS calls it, and `code:` is
/// the fragment `ingest::match_course` matches a UID against.
#[test]
fn a_seeded_course_note_is_titled_by_its_code_and_keeps_the_lms_name() {
    let root = temp("coursenote");
    let v = root.join("Vault");
    let mut p = plan_for(&v);
    p.courses = vec![
        knowlu::scaffold::CourseSeed {
            code: "_404752_1".into(),
            name: "202640-BUI-100-101".into(),
            slug: "bui-100".into(),
            label: "BUI 100".into(),
        },
        // No readable code anywhere: the label is empty and the note keeps today's behaviour.
        knowlu::scaffold::CourseSeed {
            code: "Independent Study".into(),
            name: "Independent Study".into(),
            slug: "independent-study".into(),
            label: String::new(),
        },
    ];
    create_vault(&v, &p).unwrap();
    let note = std::fs::read_to_string(v.join("courses").join("bui-100.md")).unwrap().replace("\r\n", "\n");
    assert!(note.contains("title: BUI 100\n"), "{note}");
    assert!(note.contains("name: 202640-BUI-100-101\n"), "{note}");
    assert!(note.contains("code: _404752_1\n"), "{note}");
    assert!(note.contains("slug: bui-100\n"), "{note}");
    // A course whose id and name both carry no readable code keeps today's behaviour — the name.
    let plain = std::fs::read_to_string(v.join("courses").join("independent-study.md")).unwrap().replace("\r\n", "\n");
    assert!(plain.contains("title: Independent Study\n"), "{plain}");
    assert!(plain.contains("name: Independent Study\n"), "{plain}");
}
```

In `app/tests/lms_link.rs`, after `an_enrolled_course_list_is_read_from_either_lms_shape`:

```rust
/// D4, from the real enrolment: UA's Blackboard hands back an opaque `courseId` and puts the code
/// in the NAME. The slug that decides `courses/<slug>.md`, every task's `course:` field and the key
/// `judge::Heuristics::knows_course` matches now comes from either place — never from the whole
/// name, which is how one course ended up with two slugs.
#[test]
fn a_course_whose_code_is_only_in_its_name_still_slugs_to_that_code() {
    use knowlu::lms_link::courses_from_json;
    let body = r#"{"results":[{"courseId":"_404752_1","course":{"name":"202640-BUI-100-101"}},{"courseId":"_404999_1","course":{"name":"202640-MATH-125-001"}}]}"#;
    let got = courses_from_json(body);
    assert_eq!(got.len(), 2);
    assert_eq!(got[0].code, "_404752_1", "the LMS's own key is untouched");
    assert_eq!(got[0].name, "202640-BUI-100-101");
    assert_eq!(got[0].slug, "bui-100");
    // R-C1c-plan-2: the human code, read once here and carried — the page never reads one itself.
    assert_eq!(got[0].label, "BUI 100");
    assert_eq!(got[1].slug, "math-125");
    assert_eq!(got[1].label, "MATH 125");
    // The id still wins where it carries a code: `UACS100Fall2026` is `suggest_course`'s, peel and
    // all, and reading the name instead would answer the same thing the long way round.
    let glued = r#"{"results":[{"courseId":"UACS100Fall2026","course":{"name":"CS 100 Intro to Computer Science"}}]}"#;
    assert_eq!(courses_from_json(glued)[0].slug, "cs-100");
    assert_eq!(courses_from_json(glued)[0].label, "CS 100");
    // And a name with no code in it still slugs from the name, rather than being dropped — with an
    // empty label, which is what tells every reader there was nothing to read.
    let odd = courses_from_json(r#"[{"name":"Independent Study"}]"#);
    assert_eq!(odd[0].slug, "independent-study");
    assert_eq!(odd[0].label, "");
}

```

…and amend the existing round-trip test `a_captured_course_is_exactly_what_the_wizard_plan_takes_back`
(`app/tests/lms_link.rs:470-481`), whose `assert_eq!(keys, ["code", "name", "slug"])` is now a
three-field claim about a four-field struct. It is the test that pins `Course` and `CourseSeed` as
one shape, so it is where the fourth field belongs — no second test for the same fact:

```rust
    assert_eq!(keys, ["code", "label", "name", "slug"], "{v}");
```

and `assert_eq!(seed.label, "CS 100");` beside its three existing field assertions, with one sentence
added to its doc comment: *"Four fields since R-C1c-plan-2, and still nothing else riding along — a
fifth would go out to a page and come back into a vault."*

- [ ] **Step 2: Run them to verify they fail**

Run: `cargo test -p knowlu --test scaffold a_course` then `cargo test -p knowlu --test lms_link a_course`
Expected: FAIL to compile — `knowlu::scaffold::course_code_in_name` is unresolved and neither struct has a `label` field. Once they do, the Blackboard body still slugs to `202640-bui-100-101` and the note is still titled by the name.

- [ ] **Step 3: Replace `code_in_name` with `course_code_in_name`**

In `app/src/scaffold.rs`, replace `fn code_in_name` (~287) whole:

```rust
/// **D4: the course code an LMS wrote into a display name.** `202640-BUI-100-101` → `BUI 100`,
/// `MATH-125-001` → `MATH 125`, `CS 100 Intro to Computer Science` → `CS 100` (the two-word form
/// this replaces read, and still reads).
///
/// Two to four capitals, one separator — a dash or a space — then exactly three digits and an
/// optional trailing letter. **No institution-prefix peel**, unlike [`suggest_course`]: a name's
/// letters sit next to a separator rather than glued to a term, so there is nothing in front of the
/// subject to peel, and peeling `MATH` to `TH` would name a course nobody is taking.
///
/// Hand-rolled rather than a regex, like its sibling, because the two conditions a regex cannot
/// state without a lookahead are the ones that matter: the capitals must START a run (`BUI`, never
/// `UI`), and the number must END one (`MATH-1250` is not `MATH 125`).
///
/// `None` for a name with no code in it, and never a guess: an invented fragment matches somebody
/// else's course.
pub fn course_code_in_name(name: &str) -> Option<String> {
    let chars: Vec<char> = name.chars().collect();
    for start in 0..chars.len() {
        if start > 0 && chars[start - 1].is_ascii_uppercase() {
            continue;
        }
        let letters: String = chars[start..].iter().take_while(|c| c.is_ascii_uppercase()).collect();
        if letters.len() < 2 || letters.len() > 4 {
            continue;
        }
        let mut i = start + letters.len();
        // The separator is required: the glued form (`UACS100Fall2026`) is `suggest_course`'s, and
        // answering it here would skip the peel that makes it `CS 100`.
        if chars.get(i).map(|c| *c == '-' || c.is_whitespace()) != Some(true) {
            continue;
        }
        i += 1;
        // `take_while` collects the whole run, so four digits is a length of four and is refused
        // here rather than truncated into a course number nobody wrote.
        let digits: String = chars[i..].iter().take_while(|c| c.is_ascii_digit()).collect();
        if digits.len() != 3 {
            continue;
        }
        i += digits.len();
        let suffix = match chars.get(i) {
            Some(c) if c.is_ascii_alphabetic() => {
                i += 1;
                c.to_ascii_uppercase().to_string()
            }
            _ => String::new(),
        };
        if chars.get(i).map(|c| c.is_ascii_alphanumeric()) == Some(true) {
            continue;
        }
        return Some(format!("{letters} {digits}{suffix}"));
    }
    None
}
```

And in `course_fragments` (~310), the one call site:

```rust
    if let Some(human) = suggest_course(code).or_else(|| course_code_in_name(&c.name)) {
```

- [ ] **Step 3a: Give both structs the `label` field (Ruling R-C1c-plan-2)**

In `app/src/scaffold.rs`, `CourseSeed` (~271):

```rust
pub struct CourseSeed {
    pub code: String,
    // m3: `create_vault_in` explicitly invents both when they are blank
    // (`c.code.clone()` / `slug(&c.code)`), so a page that omits either is not a refusal.
    #[serde(default)]
    pub name: String,
    #[serde(default)]
    pub slug: String,
    /// R-C1c-plan-2: the human course code (`BUI 100`) when the LMS's id or its name carried one,
    /// empty when neither did. Read once, in `lms_link::courses_from_json`, and carried from there:
    /// the note's title, the wizard's row and the mapping datalist all read this rather than each
    /// deriving a code of their own.
    #[serde(default)]
    pub label: String,
}
```

In `app/src/lms_link.rs`, `Course` (~461) — the same field, same name, and its doc comment's "three
fields" becomes "four":

```rust
    /// `cs-100`. The vault's own name for it: the note's stem, and every task's `course:` field.
    pub slug: String,
    /// R-C1c-plan-2: the human course code, or empty. The one place a code is read out of an id or
    /// a name is `courses_from_json`; every reader downstream takes it from here.
    #[serde(default)]
    pub label: String,
```

In `app/src/onboarding.rs`, the `CourseSeed` rebuild (~633) carries it through — trimmed, and never
invented:

```rust
        .map(|c| crate::scaffold::CourseSeed {
            code: c.code.clone(),
            name: if c.name.trim().is_empty() { c.code.clone() } else { c.name.clone() },
            slug: slug(if c.slug.trim().is_empty() { &c.code } else { &c.slug }),
            // R-C1c-plan-2: whatever the page sent, trimmed. An empty label is a course whose id
            // and name carried no code, and the note is titled by its name — never by a guess made
            // here, two panels away from the only function that knows the rule.
            label: c.label.trim().to_string(),
        })
```

- [ ] **Step 4: Read the name in `courses_from_json`, and keep the code it read**

In `app/src/lms_link.rs` (~502), replace the slug expression:

```rust
        // The human code comes from the SUGGESTED code where the LMS's own id carries one —
        // `ua-cs-100-fall-2026` is nobody's idea of a course — and from the NAME otherwise (D4:
        // Blackboard hands back an opaque `courseId` and writes the code into the name).
        let label = crate::scaffold::suggest_course(&code)
            .or_else(|| crate::scaffold::course_code_in_name(&name));
        // The slug is that code's, and the name's when there was no code to read.
        let slug = label
            .as_deref()
            .map(knowlu_engine::ingest::slugify)
            .filter(|s| !s.is_empty())
            .unwrap_or_else(|| knowlu_engine::ingest::slugify(if name.is_empty() { &code } else { &name }));
        out.push(Course { code, name, slug, label: label.unwrap_or_default() });
```

(and delete the old `out.push(Course { code, name, slug });` line below it).

- [ ] **Step 5: Title the course note by its code and carry the LMS name**

In `app/src/scaffold.rs`, in the `for c in seeded_courses(plan)` loop (~650):

```rust
    for c in seeded_courses(plan) {
        // D4: the note is TITLED by the human code the capture read (R-C1c-plan-2's `label`), so a
        // typed `BUI 100` on the coursework panel and this note are the same course. `name:` always
        // carries the school's own name, so nothing the student recognises is lost; `code:` stays
        // the LMS's own key, which is the `course_map` fragment `ingest::match_course` matches a
        // UID against. A course whose id and name both carried no code has an empty label and keeps
        // today's behaviour — the name. Nothing is recomputed here: one reader, one rule.
        let title = if c.label.trim().is_empty() { c.name.as_str() } else { c.label.as_str() };
        let front = Node::map(vec![
            ("title", Node::text(title)),
            ("name", Node::text(&c.name)),
            ("slug", Node::text(&c.slug)),
            ("code", Node::text(&c.code)),
            ("status", Node::text("active")),
        ]);
```

- [ ] **Step 5a: Amend the existing tests the fourth field and the new title change (B1)**

`app/tests/scaffold.rs:682-686` asserts the course note's title verbatim, and under Step 5 both rows
now fail. In `every_enrolled_course_becomes_a_note_the_engine_can_find` (~676), give the seeds their
labels and assert both lines:

```rust
    p.courses = vec![
        CourseSeed { code: "CS 100".into(), name: "CS 100 Intro to Computer Science".into(), slug: "cs-100".into(), label: "CS 100".into() },
        CourseSeed { code: "GN 103".into(), name: "GN 103 German".into(), slug: "gn-103".into(), label: "GN 103".into() },
    ];
```

```rust
    // D4: the note is titled by the human code, and `name:` carries the school's own name.
    for (slug, title, name) in [
        ("cs-100", "CS 100", "CS 100 Intro to Computer Science"),
        ("gn-103", "GN 103", "GN 103 German"),
    ] {
        let note = dest.join("courses").join(format!("{slug}.md"));
        let text = knowlu_engine::pystr::read_text(&note).unwrap_or_else(|e| panic!("{}: {e}", note.display()));
        assert!(text.contains(&format!("title: {title}")), "{text}");
        assert!(text.contains(&format!("name: {name}")), "{text}");
        assert!(text.contains(&format!("slug: {slug}")), "{text}");
        assert!(text.contains("## Grade weights"), "{text}");
        // Every note has an opaque id, like every other note this app writes.
        assert!(text.contains("id: course_"), "{text}");
    }
```

and its doc comment gains one sentence: *"From D4 on the title is the human code the capture read and
`name:` carries the LMS's own name."*

Then give the remaining `CourseSeed` literals their fourth field — every one of them, or the file
does not compile:

- `app/tests/scaffold.rs:91` (`a_scaffolded_vault_ranks_without_the_unmigrated_warning`) — `label: "CS 100".into(),`
- `app/tests/scaffold.rs:710, 714, 716` (`a_captured_course_maps_by_its_lms_id_and_by_the_code_a_summary_spells`) — `label: "CS 100".into(),`, `label: "CS 100".into(),`, `label: "GN 103".into(),`. Its assertions are about `course_map` fragments, which `course_fragments` still computes from the id and the name, so they are unchanged.
- `app/tests/scaffold.rs:769` (`every_mapped_course_is_a_slug_the_vault_knows`) — `label: "CS 100".into(),`
- `app/tests/onboarding.rs:923, 924` (`a_page_supplied_course_slug_is_normalised_before_it_names_a_file`) — `label: String::new(),` on both; that test is about the slug, and an empty label is the honest value for a hand-built plan.

And the headless walk's fake, `scripts/wizard-check.py` (~69), answers what `capture_courses` now
answers:

```python
  if (cmd === 'capture_courses') {
    return Promise.resolve({ ok: true, error: null, typed: false,
      courses: [{ code: 'UACS100Fall2026', name: 'CS 100 Intro', slug: 'cs-100', label: 'CS 100' }] }); }
```

- [ ] **Step 6: Say it on the wizard's own row**

In `app/static/console.js`, `renderCourses` (~1999) — spec §4's `BUI 100 · 202640-BUI-100-101`,
rendered from the field R-C1c-plan-2 added rather than from a code the page read for itself:

```js
  function renderCourses() {
    EL("wiz-course-rows").innerHTML = WIZ.courses.map(function (c, i) {
      // R-C1c-plan-2: the human code first, the school's own name beside it — and just the name
      // when there was no code to read, or when the two are the same string (a typed course is
      // both). The page never reads a code out of a name: that rule lives in
      // `scaffold::course_code_in_name`, and a second copy here would drift from it in silence.
      var lead = (c.label && c.label !== c.name) ? h(c.label) + " &middot; " + h(c.name) : h(c.name || c.code);
      return '<div class="wiz-row" data-course="' + i + '"><span class="meta">' + lead +
             '</span><button class="b" data-drop="' + i + '">Remove</button></div>';
    }).join("");
  }
```

(Task 4 adds one call at the end of this function, for the `datalist` it introduces, and repeats the whole function so it can be read on its own.)

- [ ] **Step 7: Run the app tests that read courses**

Run: `cargo test -p knowlu --test scaffold`, `cargo test -p knowlu --test lms_link` and `cargo test -p knowlu --test onboarding`
Expected: PASS, including `every_enrolled_course_becomes_a_note_the_engine_can_find` and `a_captured_course_is_exactly_what_the_wizard_plan_takes_back` (both amended above), `a_captured_course_maps_by_its_lms_id_and_by_the_code_a_summary_spells`, `a_page_supplied_course_slug_is_normalised_before_it_names_a_file` and `a_scaffolded_vault_ranks_without_the_unmigrated_warning` (the new `name:` line is additive frontmatter; nothing warns on it).

- [ ] **Step 8: Run the headless walk and the whole suite**

Run: `python scripts/wizard-check.py` (from the worktree root)
Expected: `ok` — check 5 still finds `CS 100` in `#wiz-course-rows`, now as the captured course's label ahead of its name, and the typed `GN 103` beside it.

Run: `cargo test --workspace`
Expected: PASS at 0 warnings, with `rejecting_and_snoozing_write_the_decision_fields` the one accepted failure.

- [ ] **Step 9: Commit**

```bash
cat > /tmp/c1c-task3.txt <<'EOF'
feat(app): one course code, one slug

D4: scaffold::course_code_in_name reads the code a school writes into a
course's display name (202640-BUI-100-101 -> BUI 100), with no
institution-prefix peel; courses_from_json slugs from it when the LMS id
carries none, so a Blackboard course and a typed code meet at one note.
Course and CourseSeed carry that code as `label` (ruling R-C1c-plan-2), so
the row and the note's title read it rather than each deriving one; the note
is titled by it and carries the LMS name as `name:`.

Co-Authored-By: Claude Fable 5.1 <noreply@anthropic.com>
Claude-Session: https://claude.ai/code/session_018EXZqBCHaJBKtYtkNfjj1Z
EOF
git add app/src/scaffold.rs app/src/lms_link.rs app/src/onboarding.rs app/static/console.js app/tests/scaffold.rs app/tests/lms_link.rs app/tests/onboarding.rs scripts/wizard-check.py
git commit -F /tmp/c1c-task3.txt
```

---

### Task 4: The mapping row offers the classes, and says what a blank one costs (spec §4 D5, §5 D6)

> **Ruling R-C1c-plan-3 (review I1).** `wizStep` advances `WIZ.step` before the `leaving === 5`
> branch runs, and `renderWizard` then hides every panel but the new one — so a sentence written into
> `#wiz-map-note` on the way out lands in a hidden panel and is never read. The ruling: **the first
> Next after a completed discovery with blank, un-ignored rows writes the sentence and STAYS on the
> panel; the next Next goes on** — the same stay-once shape the discovery itself uses, latched on
> `WIZ.mapWarned` and cleared when logins are stored again. Spec §5's `#wiz-map-note` and its "Next
> goes on" both stand; what changes is that the student reads the sentence first.

**Files:**
- Modify: `app/static/index.html` — the `#wiz-logins` panel (~132), one `<datalist>`.
- Modify: `app/static/console.js` — the `WIZ` literal (~1490), `storeCredentials` (~1598), `renderCourses` (~1999, one added call), the typed-course push (~2011), a new `renderCourseCodes`, `renderMapping` (~2020), a new `noteUnmapped`, and `wizStep`'s `leaving === 5` branch (~1692–1712).
- Test: `app/tests/static_assets.rs`, `scripts/wizard-check.py` (check 6).

**Interfaces:**
- Consumes: `WIZ.courses` (`[{code, name, slug, label}]` — `label` is Task 3's, filled by `capture_courses` and, for a typed course, by the row that pushes it) and `WIZ.map` (`[{source, key, detail, suggested, course, ignore}]`).
- Produces: a page-level `<datalist id="wiz-course-codes">` that every mapping row's `<input list="wiz-course-codes">` reads, and `WIZ.mapWarned` — the stay-once latch R-C1c-plan-3 names.

- [ ] **Step 1: Write the failing test**

In `app/tests/static_assets.rs`, after `the_logins_panel_maps_what_it_finds_to_a_course`:

```rust
/// D5 and D6: the mapping row offers the classes the wizard already captured rather than asking
/// for a code from memory, a row with nothing to offer says what it needs, and leaving one blank is
/// a choice whose consequence the panel states before Next goes on. The first live onboarding left
/// the VHL row blank, wrote `sections: {}` and spent the whole next run warning about it.
#[test]
fn a_mapping_row_offers_the_captured_classes_and_says_what_a_blank_one_costs() {
    let html = read("index.html");
    let panel = html.split("id=\"wiz-logins\"").nth(1).and_then(|s| s.split("id=\"wiz-gmail\"").next()).expect("the logins panel");
    assert!(panel.contains("<datalist id=\"wiz-course-codes\">"), "the panel keeps one datalist");
    let js = read("console.js");
    assert!(js.contains("function renderCourseCodes("), "renderCourseCodes fills it");
    assert!(js.contains("list=\"wiz-course-codes\""), "every mapping row's field reads it");
    // The list offers what round-trips to the vault's own name for the course — the human code the
    // capture read, or the slug — never the LMS's opaque key, which would be slugged into a course
    // note nobody has (R-C1c-plan-2).
    assert!(js.contains("c.label || c.slug"), "the datalist offers the human code, and the slug otherwise");
    assert!(js.contains("type the course this belongs to"), "a row with no suggestion says what it needs");
    assert!(js.contains(" of these will be asked about in the app"), "…and Next says what blank rows cost");
    assert!(js.contains("function noteUnmapped("), "noteUnmapped");
    // R-C1c-plan-3: the sentence is read BEFORE the panel goes — the first Next latches and stays,
    // the second goes on. Asserted over the `wizGo`/`wizStep` slice (the split runs to
    // `wizRegister`, so it spans both), because `wizFinish`'s own bookkeeping must not stand in.
    let go = js.split("function wizGo(").nth(1).and_then(|s| s.split("function wizRegister(").next()).expect("wizGo/wizStep");
    assert!(go.contains("noteUnmapped()"), "the count is written on the way out of the panel");
    assert!(go.contains("WIZ.mapWarned"), "…and the panel stays once, so the student reads it");
    assert!(js.contains("mapWarned: false"), "the latch starts clear on a fresh wizard");
    assert!(
        js.contains("WIZ.discovered = false; WIZ.mapWarned = false"),
        "…and re-typed logins clear it with the discovery they invalidate"
    );
    // R-C1b-exec-10 still holds: the sentence is a note, never a refusal — the second Next goes on
    // whatever the rows say, and nothing writes an error for a blank one.
    assert!(!go.contains("WIZ.error = \"Map"), "a blank row must never block Next");
}
```

- [ ] **Step 2: Run it to verify it fails**

Run: `cargo test -p knowlu --test static_assets a_mapping_row_offers`
Expected: FAIL — `the panel keeps one datalist`.

- [ ] **Step 3: Add the datalist to the page**

In `app/static/index.html`, inside `#wiz-logins`, between the VHL row and `<div id="wiz-map" hidden>`:

```html
    <datalist id="wiz-course-codes"></datalist>
```

- [ ] **Step 4: Fill it from the captured classes**

In `app/static/console.js`, replace `renderCourses` (~1999) with the Task 3 body plus its new last line, and add `renderCourseCodes` beneath it:

```js
  function renderCourses() {
    EL("wiz-course-rows").innerHTML = WIZ.courses.map(function (c, i) {
      // R-C1c-plan-2: the human code first, the school's own name beside it — and just the name
      // when there was no code to read, or when the two are the same string (a typed course is
      // both). The page never reads a code out of a name: that rule lives in
      // `scaffold::course_code_in_name`, and a second copy here would drift from it in silence.
      var lead = (c.label && c.label !== c.name) ? h(c.label) + " &middot; " + h(c.name) : h(c.name || c.code);
      return '<div class="wiz-row" data-course="' + i + '"><span class="meta">' + lead +
             '</span><button class="b" data-drop="' + i + '">Remove</button></div>';
    }).join("");
    renderCourseCodes();
  }

  /// D5: the codes a mapping row offers, so a student picks a class rather than typing one from
  /// memory — the VHL row that nobody filled is why `sections: {}` reached the engine.
  ///
  /// The VALUE is the human code the capture read (R-C1c-plan-2's `label`) and the course's own
  /// slug otherwise: `create_vault_in` slugs whatever the row carries, and both of those slug to
  /// the note the course already has. The LMS's opaque key would not — it would make a second
  /// course. The LABEL is what the student recognises (M4), so the list reads as their class list
  /// rather than as identifiers.
  function renderCourseCodes() {
    EL("wiz-course-codes").innerHTML = WIZ.courses.map(function (c) {
      return '<option value="' + h(c.label || c.slug) + '">' + h(c.name || c.code) + "</option>";
    }).join("");
  }
```

…and the typed-course row gives its own push a label, so a course the student typed offers the thing
they typed rather than an empty value (`app/static/console.js:2011`):

```js
      if (code) { WIZ.courses.push({ code: code, name: code, slug: "", label: code }); EL("wiz-course-add").value = ""; renderCourses(); }
```

- [ ] **Step 5: Point the rows at it, and say what a row with no suggestion needs**

Replace the row template inside `renderMapping` (~2024):

```js
    EL("wiz-map-rows").innerHTML = WIZ.map.map(function (r, i) {
      // D6: a row nobody can guess for says so. Computed at paint, never on every keystroke —
      // re-rendering the rows under the cursor would take the focus out of the field being typed
      // into — so the hint goes on the next paint, which is what the student has already answered.
      var hint = (!r.suggested && !r.course) ? '<span class="meta">type the course this belongs to</span>' : "";
      return '<div class="wiz-row" data-map="' + i + '"><span class="meta">' + h(r.key) +
             (r.detail ? " &middot; " + h(r.detail) : "") + '</span>' +
             '<input type="text" list="wiz-course-codes" data-course-for="' + i + '" value="' + h(r.course || r.suggested || "") +
             '" placeholder="Course code, e.g. CS 100">' + hint +
             '<label><input type="checkbox" data-ignore-for="' + i + '"' + (r.ignore ? " checked" : "") + '> Ignore</label></div>';
    }).join("");
```

- [ ] **Step 6: Stay once, and say what the blank rows cost (Ruling R-C1c-plan-3)**

In `app/static/console.js`, beside `renderMapping`, add:

```js
  /// D6: leaving the logins panel with rows still blank is a choice, not a refusal
  /// (R-C1b-exec-10 already lets Next through) — but it has a consequence, and the panel says what
  /// it is: the engine files a coursework-map card for each one (R-OB-1) and the app asks about it
  /// there. Silent when nothing is blank; the singular reads correctly without a special case.
  ///
  /// Returns the count, so the caller can decide whether there is anything to stay for.
  function noteUnmapped() {
    var n = WIZ.map.filter(function (r) { return !r.ignore && !r.course; }).length;
    if (n) { EL("wiz-map-note").textContent = n + " of these will be asked about in the app"; }
    return n;
  }
```

In the `WIZ` literal (~1490), beside the other latches:

```js
              // R-C1c-plan-3: `mapWarned` is whether the blank-row sentence has been shown once.
              // `wizStep` sets `WIZ.step` BEFORE the panel branch runs and `renderWizard` hides
              // every other panel, so a sentence written on the way out is a sentence nobody reads
              // — the first Next stays on the panel to show it, the second goes on.
              lmsOpen: false, discovering: false, discovered: false, mapWarned: false, checkoutOpened: false, schoolSeq: 0,
```

In `storeCredentials` (~1598), beside the line that clears `WIZ.discovered`, so re-typed logins get
a fresh discovery *and* a fresh warning:

```js
      if (stored.length) { WIZ.discovered = false; WIZ.mapWarned = false; }
```

And in `wizStep`'s `leaving === 5 && n > leaving` branch, inside `storeCredentials().then(...)`,
replace the already-discovered early return (~1707):

```js
        if (WIZ.map.length || WIZ.discovered) {
          // R-C1c-plan-3: the first Next after a finished discovery with blank, un-ignored rows
          // writes the sentence and stays here — `WIZ.step` was advanced above, so putting it back
          // is what keeps the panel, and its note, on screen. The next Next goes on whatever the
          // rows say: this is a sentence, not a gate.
          if (!WIZ.mapWarned && noteUnmapped()) { WIZ.mapWarned = true; WIZ.step = leaving; }
          renderWizard();
          return;
        }
```

- [ ] **Step 7: Drive it headlessly**

In `scripts/wizard-check.py`, in check 6, replace the two lines that fill the VHL row and press Next (the `page.fill('[data-course-for="1"]', "GN 103")` pair, ~239) with:

```python
    # D6 / R-C1c-plan-3: the first Next after a discovery with blank rows STAYS on the panel and
    # says what they cost — the engine files a card for each (R-OB-1) and the app asks there; the
    # second Next goes on. `is_visible` is the assertion that matters: `inner_text` falls back to
    # `textContent` and passes on a hidden panel, which is the bug this check exists for.
    page.click("#wiz-next"); page.wait_for_timeout(300)
    if page.is_hidden("#wiz-logins"): bad.append("the blank-row sentence did not keep the student on the panel")
    if not page.is_visible("#wiz-map-note"): bad.append("the count sentence was written to a hidden panel")
    if "asked about in the app" not in page.inner_text("#wiz-map-note"):
        bad.append("a blank mapping row did not say it would be asked about in the app")
    if names(page).count("discover_coursework") != 2: bad.append("staying to warn re-ran discovery")
    # D5: the row offers the classes the wizard already captured, by the code the vault will use.
    if page.get_attribute('[data-course-for="1"]', "list") != "wiz-course-codes":
        bad.append("the mapping row does not offer the captured classes")
    opts = page.eval_on_selector_all("#wiz-course-codes option", "os => os.map(o => o.value)")
    if "CS 100" not in opts: bad.append(f"the datalist does not carry the captured class: {opts!r}")
    page.fill('[data-course-for="1"]', "GN 103"); page.wait_for_timeout(120)
    page.click("#wiz-next"); page.wait_for_timeout(300)
```

The rest of check 6 — the Gmail panel, the cleared password, `store_credentials`' vault — is
unchanged, and so is check 9's assertion that the plan carries the VHL mapping: the row is filled
before the Next that leaves the panel for good. The earlier empty-discovery sequence is unaffected:
`WIZ.map` is empty there, so `noteUnmapped()` returns zero, nothing is written over that panel's
"You can go on…" note and nothing stays.

- [ ] **Step 8: Run the static test and the walk**

Run: `cargo test -p knowlu --test static_assets`
Expected: PASS, including `the_logins_panel_maps_what_it_finds_to_a_course` and the empty-discovery test.

Run: `python scripts/wizard-check.py`
Expected: `ok`.

- [ ] **Step 9: Run the whole suite**

Run: `cargo test --workspace`
Expected: PASS at 0 warnings, with `rejecting_and_snoozing_write_the_decision_fields` the one accepted failure.

- [ ] **Step 10: Commit**

```bash
cat > /tmp/c1c-task4.txt <<'EOF'
feat(app): the mapping row offers the classes, and says what a blank one costs

D5: every mapping row's course field reads one page-level datalist filled
from the captured classes, by the code the vault will use, so a student
picks rather than types. D6: a row with no suggestion says it needs one, and
the first Next off the logins panel stays once to say how many blank rows
the app will ask about (ruling R-C1c-plan-3); the second goes on.

Co-Authored-By: Claude Fable 5.1 <noreply@anthropic.com>
Claude-Session: https://claude.ai/code/session_018EXZqBCHaJBKtYtkNfjj1Z
EOF
git add app/static/index.html app/static/console.js app/tests/static_assets.rs scripts/wizard-check.py
git commit -F /tmp/c1c-task4.txt
```

---

### Task 5: The first-run view (spec §6, D7)

> **Ruling R-C1c-plan-1 (review B2).** `surface::build_state` has no failure path — it returns a
> `State` unconditionally, and `surface::load` never reads `state/today.md` — so `build_state_value`
> answers `ok: true` with a nearly empty state on a wizard-made vault, and a page that keyed on
> `!env.ok` would never paint the first-run view at all. The ruling: **`commands::state` attaches
> `first_run` iff `knowlu_engine::ingest::is_first_run(&cs.vault)`, and `poll()` paints `#first-run`
> whenever the envelope carries the key and hides it when the key is absent.** The state paints
> behind it, as it is. Spec §6's `ok: true, state: null` envelope stays as the safety net for the
> three ways `build_state_value` really can fail.

**Files:**
- Modify: `app/src/commands.rs` — a new `first_run_value` beside `attach_scheduler` (~54), and the `state` command wrapper (~337).
- Modify: `app/static/index.html` — one block under `#delta`.
- Modify: `app/static/console.js` — `poll` (~579) and a new `renderFirstRun` above it.
- Test: `app/tests/commands.rs`, `app/tests/static_assets.rs`.

**Interfaces:**
- Consumes: `knowlu_engine::ingest::is_first_run(&Path) -> bool`; `crate::scheduler::{lock, Scheduler, RunSummary}` — `Scheduler.running: Mutex<bool>`, `Scheduler.last: Mutex<Option<RunSummary>>`, `RunSummary.steps: Vec<(String, i32)>`.
- Produces: `pub fn first_run_value(cs: &ConsoleState, sch: &Scheduler) -> Option<Value>` — `Some` iff `is_first_run` (R-C1c-plan-1) — and the envelope key
  `first_run: { "running": bool, "steps": [[label, code], …] }`. In the rare case where the read model could not be built at all, the envelope is `{ ok: true, error: null, state: null, first_run: {…} }`; any other failure on a ranked vault keeps today's `ok: false`.

- [ ] **Step 1: Write the failing tests**

In `app/tests/commands.rs`, extend the imports and add the test after `state_returns_the_envelope_with_the_read_model_and_never_writes`:

```rust
use knowlu::scheduler::{lock, RunSummary, Scheduler};
```

```rust
/// D7 / §6: between Finish and the first `rank` a vault has no read model, and the window painted
/// nothing for about a minute — a white page with an error line in it. The envelope now carries a
/// block that says so, with the steps the slot has finished, and the page has a sentence for it.
#[test]
fn a_vault_with_no_read_model_yet_carries_the_first_run_block() {
    let v = scratch("firstrun");
    assert!(!v.join("state").join("today.md").exists(), "this vault has never been ranked");
    let cs = ConsoleState::open(v.clone(), std::env::temp_dir().join(format!("qo-console-appdata-firstrun-{}", std::process::id())));
    // R-C1c-plan-1, the premise tested rather than assumed: the read model builds fine without
    // `state/today.md` (`surface::build_state` has no failure path), so `ok` is true and the BLOCK
    // — not a failed envelope — is what the page keys on.
    let env = state_inner(&cs, "today").unwrap();
    assert_eq!(env["ok"], true);
    let sch = Scheduler::default();
    let fr = knowlu::commands::first_run_value(&cs, &sch).expect("a vault with no today.md carries it");
    assert_eq!(fr["running"], false);
    // The slot in flight records nothing until it ends: an empty list, never an invented step.
    assert_eq!(fr["steps"], json!([]));

    *lock(&sch.last) = Some(RunSummary {
        started: "2026-09-22T19:14:37Z".into(),
        ended: "2026-09-22T19:15:38Z".into(),
        steps: vec![("coursework".into(), 0), ("judge (skipped: no entitlement)".into(), 0)],
        ok: true,
        engine_ok: true,
        late: false,
        reason: None,
        attempts: 1,
    });
    let fr = knowlu::commands::first_run_value(&cs, &sch).expect("still no read model");
    assert_eq!(fr["steps"][1][0], "judge (skipped: no entitlement)");
    assert_eq!(fr["steps"][1][1], 0);

    // …and the moment `rank` has written the day, the block is gone and the page paints normally.
    std::fs::create_dir_all(v.join("state")).unwrap();
    std::fs::write(v.join("state").join("today.md"), b"# Today\n").unwrap();
    assert!(knowlu::commands::first_run_value(&cs, &sch).is_none());
}
```

- [ ] **Step 2: Run it to verify it fails**

Run: `cargo test -p knowlu --test commands a_vault_with_no_read_model`
Expected: FAIL — `knowlu::commands::first_run_value` does not exist.

- [ ] **Step 3: Build the block, and attach it to the `state` envelope**

In `app/src/commands.rs`, after `attach_scheduler` (~72):

```rust
/// D7 / §6: what the window has to paint while the vault has no read model yet — the minute between
/// Finish and the first `rank`, which was a white page with an engine error in it.
///
/// **Ruling R-C1c-plan-1:** attached whenever the vault has never been through a whole slot
/// (`ingest::is_first_run` — the absence of `state/today.md`, the same predicate
/// `scheduler::needs_first_run` and the engine's own first-run rules read), and never on a failed
/// read model: `surface::build_state` has no failure path, so a wizard-made vault answers `ok` with
/// a nearly empty state, and a page that waited for a failure would paint that empty day and call it
/// the first look. The page paints the block while the key is there and drops it when it stops
/// coming.
///
/// `running` and `steps` come from the live `Scheduler`; a slot in flight records nothing until it
/// ends, so `steps` is empty on the first poll and the page shows its sentence alone.
pub fn first_run_value(cs: &ConsoleState, sch: &Scheduler) -> Option<Value> {
    if !knowlu_engine::ingest::is_first_run(&cs.vault) {
        return None;
    }
    let running = *crate::scheduler::lock(&sch.running);
    let steps = crate::scheduler::lock(&sch.last).as_ref().map(|s| s.steps.clone()).unwrap_or_default();
    Some(json!({ "running": running, "steps": steps }))
}
```

And replace the `state` command wrapper (~337):

```rust
#[tauri::command] pub fn state(cs: State<'_, ConsoleState>, sch: State<'_, Scheduler>, view: String) -> Value {
    let mut env = state_inner(&cs, &view).unwrap_or_else(|e| json!({ "ok": false, "error": e, "state": Value::Null }));
    if let Some(fr) = first_run_value(&cs, &sch) {
        // §6: a vault with no read model yet answers `ok: true` and no state, rather than an error
        // line a student can do nothing about — the page has a sentence for exactly this minute.
        // Any other failure, on a vault that has been ranked, keeps today's `ok: false`.
        if env["ok"] != true { env = json!({ "ok": true, "error": Value::Null, "state": Value::Null }); }
        env["first_run"] = fr;
    }
    let _ = attach_scheduler(&mut env, &sch);
    env
}
```

- [ ] **Step 4: Run the commands test**

Run: `cargo test -p knowlu --test commands a_vault_with_no_read_model`
Expected: PASS.

- [ ] **Step 5: Write the failing page test**

In `app/tests/static_assets.rs`, after the mapping-row test from Task 4:

```rust
/// D7 / §6: the minute between Finish and the first `rank` says what is happening instead of
/// painting nothing. The sentence is the page's, the steps come from the envelope, and the page
/// asks again every three seconds until the day is there — then goes back to its usual cadence.
#[test]
fn the_first_run_view_says_what_is_happening_and_polls_until_the_day_arrives() {
    let html = read("index.html");
    assert!(html.contains("id=\"first-run\""), "the first-run block");
    assert!(html.contains("id=\"first-run-steps\""), "…and the steps the slot has finished");
    assert!(
        html.contains("Knowlu is doing its first run. Your day appears here in about a minute."),
        "the sentence D7 asks for, in the page rather than in a string the engine sends"
    );
    let js = read("console.js");
    assert!(js.contains("function renderFirstRun("), "renderFirstRun");
    assert!(js.contains("function hideFirstRun("), "hideFirstRun");
    assert!(js.contains("var FIRST_RUN_MS = 3000"), "the first-run cadence is three seconds");
    assert!(js.contains("setInterval(poll, 60000)"), "…and the usual cadence is unchanged");
    assert!(js.contains("EL(\"first-run\").hidden = true"), "…and the block is hidden once the day is there");
    let poll = js.split("function poll(").nth(1).and_then(|s| s.split("function openDrawer(").next()).expect("poll");
    // R-C1c-plan-1: the view stands on the presence of the block — which is `is_first_run` — and
    // never on a failed state, because `surface::build_state` has no failure path to wait for.
    assert!(poll.contains("if (env.first_run) {"), "the first-run view stands on is_first_run alone");
    assert!(poll.contains("renderFirstRun("), "…poll paints it");
    assert!(poll.contains("hideFirstRun()"), "…and takes it away when the key stops coming");
}
```

- [ ] **Step 6: Run it to verify it fails**

Run: `cargo test -p knowlu --test static_assets the_first_run_view`
Expected: FAIL — `the first-run block`.

- [ ] **Step 7: Add the block to the page**

In `app/static/index.html`, immediately after `<div class="delta" id="delta"></div>`:

```html
    <div id="first-run" hidden>
      <p class="lede">Knowlu is doing its first run. Your day appears here in about a minute.</p>
      <div id="first-run-steps"></div>
    </div>
```

- [ ] **Step 8: Paint it, and poll faster while it is up**

In `app/static/console.js`, immediately above `function poll()` (~579):

```js
  // D7: the minute between Finish and the first `rank`. `first_run` rides on the envelope until
  // `state/today.md` exists; while it does, the page says what is happening, lists the slot's steps
  // as they land, and asks again every three seconds so the day appears as soon as it is there
  // rather than up to a minute later. The state paints behind it as it is — on a vault this new
  // that is an empty day, which is exactly what the block is covering (R-C1c-plan-1).
  //
  // M6: nothing ends the three-second cadence but the day arriving, so a vault whose `rank` keeps
  // failing polls on forever. That is honest rather than silent: the failed step shows up in the
  // list below the line as soon as the slot ends, and the Runs view has the rest. A cap would
  // replace a true "still working" with a false "gave up".
  var FIRST_RUN_MS = 3000;
  var firstRunTimer = null;
  function renderFirstRun(fr) {
    EL("first-run").hidden = false;
    EL("first-run-steps").innerHTML = (((fr && fr.steps) || []).map(function (s) {
      return '<div class="meta">' + h(s[0]) + "</div>";
    }).join("")) + (fr && fr.running ? '<div class="meta">still working&hellip;</div>' : "");
    // One timer, cleared before it is set: the 60 s interval and the window's focus handler both
    // call poll() too, and a chain per call would multiply every three seconds.
    if (firstRunTimer) { clearTimeout(firstRunTimer); }
    firstRunTimer = setTimeout(poll, FIRST_RUN_MS);
  }

  function hideFirstRun() {
    if (firstRunTimer) { clearTimeout(firstRunTimer); firstRunTimer = null; }
    EL("first-run").hidden = true;
  }
```

and replace `poll`'s body:

```js
  function poll() {
    return invoke("state", { view: current.view }).then(function (env) {
      // R-C1c-plan-1: the block is on the envelope exactly while the vault has never been ranked,
      // which IS D7's "until the first read model exists" — `surface::build_state` has no failure
      // path, so there is no failed state to wait for. The paint below still runs.
      if (env.first_run) { renderFirstRun(env.first_run); } else { hideFirstRun(); }
      if (!env.ok) { EL("delta").textContent = "engine: " + env.error; return; }
      // §6's safety net answers `ok` with no state when the read model could not be built at all;
      // the line above is what the student reads while that is true.
      if (!env.state) { return; }
      if (env.state.revision === current.revision) { return; }
      paint(env.state, false);
    }).catch(function (e) { EL("delta").textContent = (current.state ? current.state.texts.offline : "The engine did not answer.") + " (" + e.message + ")"; });
  }
```

- [ ] **Step 9: Run the page test and the walk**

Run: `cargo test -p knowlu --test static_assets`
Expected: PASS, including `no_network_reference_in_the_shipped_page` and `console_js_is_one_render_function_per_region_and_uses_invoke`.

Run: `python scripts/wizard-check.py`
Expected: `ok` — the wizard path never reaches `poll`, and `#first-run` starts hidden.

- [ ] **Step 10: Run the whole suite**

Run: `cargo test --workspace`
Expected: PASS at 0 warnings, with `rejecting_and_snoozing_write_the_decision_fields` the one accepted failure.

- [ ] **Step 11: Commit**

```bash
cat > /tmp/c1c-task5.txt <<'EOF'
feat(app): the console says what is happening during its first run

D7: `state` carries a `first_run` block (running, and the steps the slot has
finished) until `state/today.md` exists, and answers ok with no state rather
than an error nobody can act on. The page paints one sentence and the steps
beneath it, polling every three seconds until the day arrives, then hides the
block and returns to the usual cadence. The white window is gone.

Co-Authored-By: Claude Fable 5.1 <noreply@anthropic.com>
Claude-Session: https://claude.ai/code/session_018EXZqBCHaJBKtYtkNfjj1Z
EOF
git add app/src/commands.rs app/static/index.html app/static/console.js app/tests/commands.rs app/tests/static_assets.rs
git commit -F /tmp/c1c-task5.txt
```

---

## Hand-off to the controller

After Task 5's commit, the branch is code-complete. What is left is the controller's, not an implementer's:

1. **The workspace gate**, in the foreground, from the worktree root:
   - `cargo test --workspace` — 0 warnings (`… 0 other`), and `app/tests/commands.rs::rejecting_and_snoozing_write_the_decision_fields` the one accepted failure until main is merged in.
   - `python scripts/wizard-check.py` → `ok`.
   - `pwsh -File scripts/ci/eol-check.ps1` — LF everywhere but `*.ps1`; nothing under `engine/tests/fixtures/**` was touched by this plan.
2. **`HANDOFF.md` §3's sequence entry** for C1c: the stream is code-complete on `c1c-first-day` (stacked on `c1b-sign-in`, PR #9, and merging after it), with the five decisions it lands (D1/D8 the slot, D3 coursework's first run, D4 one course one slug, D5/D6 the mapping rows, D7 the first-run view) and the two spec items it does not (§8's C5 debt and D9's two spikes). Batch it with the next milestone push rather than pushing docs alone (CI minutes).
3. **The live proof of §7** — Quinn at the machine, a fresh scratch profile pointed at staging, the founder's logins, removed afterwards along with its Credential Manager entries. It checks:
   - onboarding to Finish;
   - the console shows the first-run line, then the day (D7);
   - the run record shows `entitlement (refreshed)` and a `judge` step that **ran** (D1);
   - staging's `usage_daily` has a row for that day (the judge reached the service);
   - no zyBooks task due before onboarding day is `active` (D3);
   - the course notes are `bui-100`-style with `name:` lines (D4);
   - the VHL row left blank produced the count sentence (D6) and the engine's `map-vhl-…` card (R-OB-1);
   - `state/runner-log.md` carries a `local ok <step> (skipped: …)` line for anything the slot left out (D8, as R-C1c-plan-4 spells it).
4. **Then** the finishing-a-development-branch decision: this branch merges after `c1b-sign-in`.
