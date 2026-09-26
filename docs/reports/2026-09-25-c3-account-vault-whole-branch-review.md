# C3′ (the account vault) — the whole-branch review, the fix wave, and the merge with C1c

The final review of branch `c3-sync` (its re-review of the fix wave is appended inside it), the fix wave's report, and the report of the merge with main after PR #13. Preserved verbatim from the worktree workspace (`.superpowers/sdd/2026-09-17-c3-account-vault-plan/`, git-ignored) when the stream was closed on 2026-09-25, before the worktree and its branch were deleted.

---

<!-- the review and its re-review: final-review.md -->

# C3′ — final whole-branch review

Reviewer: Claude Opus 5.5, read-only, 2026-09-24. Range `main...c3-sync` (three dots), HEAD
`7574168` (H14 landed while this ran). No worktree was created; the C1c merge was simulated with
`git merge-tree --write-tree` (it writes only unreferenced objects; nothing to clean up). The gate
log is in the session scratchpad (`final-review-test.log`).

## Verdict: READY WITH FIXES

The branch is green, and its sync logic holds up task by task. The fixes needed are one fix-wave
dispatch plus two merge-time items:

- **Critical C1.** Nothing refreshes the session before the slot's `sync` child, *Sync now* or
  the quit push. In steady state, most slots will answer 401 ("signed out"). The bug predates
  C3′ (C2's cloud steps share it), but the sync line makes it load-bearing and visible.
- **Important I1**, a silent-divergence path through `build_push`'s cursor. The fix is narrow
  and belongs in the wave.
- **Important I2**, the two-desktop design gap. It needs Quinn's ruling before any student uses
  two desktops, not before merge.
- The ruled but unapplied items: R-C3′-exec-41 (N1–N5 and the nit) and R-C3′-exec-39 (N1–N4).
  Task 12's appendix omitted the second.

## The gate (run once, foreground)

| Check | Result |
|---|---|
| `RUSTFLAGS="-D warnings" CARGO_TERM_COLOR=never cargo test --workspace --no-fail-fast` | exit 0; **1403 passed, 0 failed, 4 ignored** (the four by-design); 203 s |
| ci.yml warning tally over the same log | `warnings: 1 accepted (.rsrc), 2 tallies, 0 other` |
| `oracle.rs` / `surface_oracle.rs` | 3/3 and 4/4, unchanged |
| fixtures / `backup.rs` | `git status --porcelain engine/tests/fixtures/` is empty; `git diff --quiet main -- engine/src/backup.rs` exits 0 |
| `deno check` (ci.yml's globs) | exit 0 |
| `deno lint` (ci.yml's paths) | `Checked 126 files`, clean |
| `deno test` with ci.yml's exact flags (`--allow-write=cloud/eval`, `cloud/supabase/ cloud/eval/`) | **458 passed, 0 failed** |
| `python scripts/wizard-check.py` / `settings-check.py` | `ok` / `ok` |

## Findings

### Critical

**C1: the session is never refreshed before a sync, so a steady-state sync gets 401.**
- The code:
  - `engine/src/cloudmodel.rs:110-124` reads the access token and never refreshes it ("Refresh is
    C1's job").
  - Only three things refresh it (`account::valid_access_token_at`, `app/src/account.rs:475-478`,
    and only when <120 s remain):
    - the launch and six-hourly housekeeping refresh (`app/src/scheduler.rs:799`);
    - the slot's own telemetry step, which runs *after* the children;
    - console account actions.
  - C1c's in-slot refresh (c1c `scheduler.rs`, the R-C1c-11 block) fires only when the cached
    entitlement is *not* `Entitled`. In steady state it is.
  - The token lives `jwt_expiry = 3600` seconds (`cloud/supabase/config.toml:23`, pushed to both
    projects on 2026-09-14).
  - `state::run_sync` (`app/src/state.rs:171-174`) and `quit_flush` (`:253-265`) call the engine
    directly, with no refresh.
- The scenario:
  - Slots run at 12:00 and 18:00, and the app launched at 08:00.
  - At 12:00 the newest token is from 08:00; at 18:00 it is from 14:00 or from the 12:00 slot's
    telemetry. Both are expired.
  - `/sync-pull` answers 401, and `SyncError::service` names it "signed out". The push is then
    tried and also answers 401.
  - `cs.sync.last_error` becomes "signed out", and `renderSyncLine` paints it amber. A signed-in,
    paying student reads "signed out" on most slots, and nothing syncs except within an hour of a
    refresh. *Sync now* gets the same result.
  - C2's `judge`/`coursework`/`ingest` cloud calls share the same exposure.
- The fix:
  - In the wave: `run_sync` and `quit_flush` call `account::valid_access_token_at(&auth_base(&cfg.api_base)?, &cfg.anon_key, &cfg.session_credential_target, now)`
    (all `pub`), ignoring its error, before `run_lines_with`.
  - At merge (C1c's lines): run the same call, unconditionally for a cloud vault, at the top of
    `run_slot_inner`'s entitlement block, before the children.
- Verify on staging once with a token older than an hour. This review could not do that live:
  no profile exists on this machine.

### Important

**I1: a record stamped in this desktop's future advances `pushed_through`, and this desktop's
own later writes are then never pushed.**
- The code:
  - `build_push` (`engine/src/sync.rs:585-651`) sends every non-ACTOR record from `pushed_through`
    on, including foreign records that `apply` appended verbatim with their own device's `ts`.
    It then moves `next.pushed_through` to that record's `ts` (`:645-650`).
  - `fold_confirmed` (`:1299-1310`) does the same after a restore, over records taken in `seq`
    order.
  - `ledger::read(since)` drops every record with `ts < since` (`ledger.rs:205-209`).
- The scenario:
  - Desktop A's clock runs ahead of B's by Δ. The skew must exceed the 10 s read lag: a
    dual-boot machine off by its UTC offset, or a PC whose time sync is off.
  - B pulls A's record R (`ts` = B-now + ~Δ), and the same run's push sets `pushed_through` to it.
  - For the next Δ − 10 s, every record B writes carries `ts < pushed_through`. That includes the
    student's field edits and the `coursework`/`judge` writes seconds later in the same slot.
    None of them is ever sent.
  - B's note text still goes up, but A drops it: `apply` never overwrites a note it has.
  - A never gets B's edits, no card is filed, and the divergence is permanent.
  - Journal M1 (Task 5, this device's own clock running backwards) is a different mechanism and
    stays deferred.
- The fix: never advance `pushed_through` or `boundary` past the run's own `now_ts`. Send a
  future-dated record, but leave the cursor where it is; it is re-sent (idempotently) until the
  clock passes it. Apply the same clamp in `fold_confirmed`.
- The test: pull a foreign record dated now+1h, make a local write, push twice, and assert that
  the local write was sent.

**I2: every signed-in desktop runs the machine steps, so two awake desktops create one new
portal item twice, at one path under two ids, and the id-keyed reconcile loses sight of each
desktop's own history.**
- The code:
  - `config/` never syncs (`is_note_path` takes `ids::NOTE_FOLDERS` only).
  - Each wizard writes `runners.yaml`'s `device:` as its own machine (`onboarding.rs`,
    `VaultPlan.device`), so `device_ok` passes on every desktop.
  - Both desktops run `coursework`/`ingest`/`judge` at the same wizard-default slot times.
  - The push of slot N's creates happens only at slot N+1's `sync`.
  - `coursework` names a new task `tasks/<item.slug>.md` (`coursework.rs:375`), the same path on
    both desktops.
- The scenario:
  - A new zyBooks item appears between slots. A home desktop and a laptop are both awake at
    12:00, and each creates `tasks/x.md`: A with `id: a`, B with `id: b`.
  - After the exchange, each keeps its own file, because a pulled text never overwrites. A's
    records arrive at B under `a`.
  - `apply` builds `mine = journal.records_for(id)` with the **foreign** id (`sync.rs:1948-1960`),
    so B's own records for that note (under `b`) are never in `mine`.
  - `reconcile::resolve` falls back to the file-mtime stand-in. That is Task 6's I1, reopened for
    every doubly created note. Concretely:
    - the student sets `importance` on A (3→5) and on B (3→2);
    - then an unrelated write (a `coursework` due-date update) touches A's note;
    - A keeps 5 (its mtime wins), B keeps 2 (its own mtime wins), and neither files a card.
  - Also:
    - both desktops judge the item, which charges the daily cloud cap twice and can file an
      agent-vs-agent card;
    - C1c's coursework-map proposals are filed twice, and a mapping answered on one desktop never
      reaches the other's `config/course_map`;
    - a restored desktop's `config/ingest.yaml` has no `- name: personal` unless it is re-pasted,
      so its busy time ignores the account's calendar.
- The fix is design. It needs Quinn's ruling before any student uses two desktops:
  - (a) one runner per account: a slot lease, so only one desktop runs the machine steps; or
  - (b) reconcile by path as well as id (add `records_for(<local file's id>)` to `mine`), and
    accept double judging.
- Not for this wave.

### Minor

- **M1: sync dates its cards by the UTC day** (`engine/src/sync.rs:2524`, `now_ts(None)[..10]`).
  `enrich.rs:547` and every other writer use `jiff::Zoned::now().date()`.
  - Scenario: on a Chicago evening (after 19:00 CDT), a sync card gets tomorrow's `proposed_at` and
    `first_proposed_at`. It charges tomorrow's fifteen-a-day budget and ages from −1.
  - Fix: `jiff::Zoned::now().with_time_zone(cli::vault_zone(vault)).date()`. **FIX-IN-WAVE.**
- **M2: comments that are false now.**
  - `engine/src/lib.rs:108-111` still says every record "goes up sealed with a key that lives only
    in this machine's Credential Manager". That is a privacy claim, and the opposite of what ships.
  - `judgelog.rs:5`, `childproc.rs:17`, `issues.rs:441` and `runs.rs:572,641` still name `history`.
  - `main.rs:390-392` says a gated skip "reaches the Runs view" (Task 8 N2; it reaches
    `runner-log.md` only).
  - `scheduler.rs:332`: `slot_argv`'s doc still reads `coursework → ingest → judge → rank`.
  - **FIX-IN-WAVE** (one comment sweep).
- **M3: a new test leaks a PID-keyed temp folder.** `app/tests/commands.rs:110`
  (`qo-diag-skip-data-<pid>`) and `:394` (`qo-sync-verbatim-data-<pid>`) never remove their
  app-data folder. That is the hazard `cea59a9` (on main) fixed for the sibling test: a reused PID
  on the self-hosted runner. **FIX-IN-WAVE**, two lines each.
- **M4: the picker's Backups restore (P3) and sync undo each other.**
  - The backup mirror includes `state/` and `config/` (`backup.rs:20-23`), so the restored vault
    carries the old `sync-cursor.json` and a `cloud.yaml` that names the **old** profile's
    session target.
  - Its first sync re-applies every tombstone the account got after the backup. A note the student
    restored because its loss had synced is archived again (it is in `archive/`, not lost).
  - It then syncs as a second vault on the same machine, under the same device token.
  - Remove the old profile (with its credentials), and the restored vault is "no session" for
    good, with no in-app way back: console sign-in writes the new profile's target, not
    `cloud.yaml`'s.
  - **DEFER**: needs a ruling on what a Backups restore of an account vault means.
- **M5: a delete on one desktop against an edit on the other resolves by push order.** The text
  row resurrects the note, or the tombstone archives the edit. No card, nothing lost. **DEFER**,
  with I2's ruling.
- **M6: *Sync now* during a slot's later step** (coursework, judge) runs `apply` in-process
  beside a child writing the same notes. The race class is pre-existing (console writes versus
  children); the cheap guard is for `commands::sync` to answer "a slot is syncing" while
  `sch.running`. **DEFER.**
- **M7: after the merge, the first-run view and `RunSummary` always tick `sync`.** The step is
  recorded as `("sync", 0)` whatever it did, because it always exits 0. A skipped or offline sync
  reads as done. **DEFER** (cosmetic; the sync line tells the truth).

## Lifecycle walk (what holds, what does not)

- **Onboarding → restore → first slot.**
  - `create_vault_in`: scaffold → `move_session` → H11b retry → `restore_into` → `finish_or_roll_back`.
    The order is right: a restore failure rolls the folder back, and an offline restore folds to
    `ok:false`.
  - Seed hashes are written before the pull, and E1's hold-back waits for `pulled_to_end`.
  - The restore cursor marks only account content as pushed.
  - On the merged product, the first slot's C1c refresh writes the entitlement cache before the
    engine gate reads it. On `c3-sync` alone, Task 8's C1 still gates that slot: a merge
    prerequisite, as ruled.
  - Defects on this path: I1 (via `fold_confirmed`) and M4.
  - A replaced seed's `create` still goes up and lands, harmlessly, as an orphan `create` on the
    other desktop (Task 9 residual (4)).
- **A second desktop's edits → card → approve/reject.** Sound for two desktops that share ids:
  - the ACTOR echo filter, the card-never-leaves-its-device rule and 6b's re-assert all hold;
  - the approve and reject records travel as `agent:approvals` and converge.
  - Broken when the ids differ (I2), and by clock skew (I1).
- **Lapse past grace.**
  - The gate refuses `sync`/`coursework`/`ingest`/`judge` at exit 0, `rank` runs, and
    `record_gated_skip` persists `skipped: no entitlement` under the lock. The page paints it
    amber. Correct.
  - *Sync now* and the quit push are not gated; that is the library path, per the fidelity row.
    They meet the server's 402 and show `last_error: "no entitlement"`, not the ruled skip copy
    (see R-39 N4 in the triage).
- **Sync now during a slot.** `RunLock` (`state/sync.lock`, `File::try_lock`, per handle) excludes
  the slot's child, *Sync now* and the quit push, in and across processes. A lock-held skip writes
  nothing, and `refresh_sync` repaints after the slot. Correct. M6 is the remaining gap.
- **Quit → flush.** Push only, under `vault_io` and the lock; `synced` is true only when there was
  no error and no skip; the cap is honoured. Correct, apart from C1: the push uses a stale token.

## Triage: Task 12's appendix, R-C3′-exec-41, and what the appendix left out

**The appendix left out three ruled items, so they are added here:**
- R-C3′-exec-24's **M6**, parked for Task 12 by name;
- R-C3′-exec-39's **N1–N4**, sent to "Task 12's final fix wave";
- the rest of R-C3′-exec-38's **M2** (false git comments outside Task 10's files).

It also lists F1 as parked, but R-C3′-exec-30 (I5) closed it at Task 9.

| # | Item | Ruling | Disposition, one line |
|---|---|---|---|
| 1 | N1: a per-feed `stored` flag; only a validated link reaches the account | R-41 | **FIX-IN-WAVE**: ruled; closes the vault leak, the rejected-link overwrite and the 60 s Finish stall (`onboarding.rs:676-688`) |
| 2 | N2: the 402 sentence "until it reaches your account" | R-41 | **FIX-IN-WAVE**: reuse `:627`'s "if it still can't" (`lms_link.rs:626`) |
| 3 | N3: the retry tests exercise one path; no personal-calendar retry-success test | R-41 | **FIX-IN-WAVE**: becomes real coverage once N1 lands |
| 4 | N4: privacy §6 (vi), issues carry the computer name | R-41 | **FIX-IN-WAVE**: page text; lands before 2026-09-24 is published, so no new version |
| 5 | N5: privacy §6 (vii), the retention scope | R-41 | **FIX-IN-WAVE**: same |
| 6 | Nit: `split(..).next().expect` at `sync_contract.rs:2686` | R-41 | **FIX-IN-WAVE**: `find(..).expect`, then slice |
| 7 | R-39 N1: the amber copy ("signed out — sign in to sync", "subscription inactive — changes stay on this computer") | R-39 | **FIX-IN-WAVE**: ruled. One thing it did not weigh: `no entitlement` is also what a paying student gets after >72 h offline (a stale cache the offline refresh cannot renew), so "inactive" is false for them. The controller's call: "subscription not confirmed — …" |
| 8 | R-39 N2: four comment inaccuracies (`console.js:382-383,:389`, `no_console.rs:26`, …) | R-39 | **FIX-IN-WAVE**: comment-only |
| 9 | R-39 N3: stale git comment, `app/tests/scheduler.rs:577-582` | R-39 | **FIX-IN-WAVE**: comment-only (outside C1c's conflict hunk) |
| 10 | R-39 N4: pin the page's skip words to the engine's | R-39 | **FIX-IN-WAVE**: also map the `last_error` causes *Sync now* shows ("signed out", "no entitlement", "offline: …") |
| 11 | N17: a late, older third-desktop write settles a card under a false warning | R-19 | **DEFER**: needs three desktops and late delivery; revisit with I2's ruling |
| 12 | N15: a judge-once card and a sync card live on one field | R-19 | **DEFER**: two visible cards, nothing written wrongly |
| 13 | N15b: `find_pending_amendment` (`write.rs:592`) ignores `created_by` | R-19 | **DEFER**: only delays the judge's re-proposal while a sync card is pending; lives in write.rs's judge-once path |
| 14 | Whether a decision on a sync card counts as human-set for judge-once | R-22 | **DEFER**: a Quinn question; affects only a later agent judgment on the other desktop |
| 15 | F1: `apply` refuses a live sync-card note | R-21 | **DROP**: done at Task 9 (R-30 I5; `sync.rs:2229`, test `a_pulled_live_sync_amend_card_is_refused_and_never_written`) |
| 16 | F2: `SyncCards::find` should require `kind: amend` or an `approvals/` path | R-21 | **DEFER**: latent; nothing creates a non-card `create` under `sync::ACTOR` today |
| 17 | 6b m1: contract tests for N23/N24 | 6b review | **DEFER**: coverage only; the behaviour was probed green in the 6b review |
| 18 | 6b m2: `approvals.rs` reaches into `crate::sync::ACTOR` | 6b review | **DROP**: a layering nit, confirmed harmless |
| 19 | M6 (omitted): a pulled note pushed back late overwrites a newer account body | R-24 | **DEFER**: bodies are last-writer-wins on `sync_notes` by design; the fix is a base-version protocol on the server and the device. Named in production gaps |
| 20 | R2-1: control/bidi characters in a server reason via `judge::one_line` | R-26 | **DEFER**: the strings come from our own server, and the fix belongs in C2's `judge.rs` |
| 21 | R2-2: a failed slot status save reaches only the slot log | R-26 | **DROP**: the next save replaces it, and the Runs view still shows the step |
| 22 | R2-3: a lock-file open failure saves the status without the lock | R-26 | **DEFER**: needs `state/` to be unwritable; the one-line recipe (no save in that arm) waits for the next sync.rs touch |
| 23 | Task 8 N1: `entitle.rs:98`'s registry match is looser than the app's key | R-28 | **DROP**: `Path` equality only normalises separators; it fails safe and never loosens the gate |
| 24 | Task 8 N2: `main.rs:390-392` says a gated skip reaches the Runs view | R-28 | **FIX-IN-WAVE**: make the comment true (`runner-log.md`) |
| 25 | Task 8 N3: two missing assertions in the spawned gate test | R-28 | **DEFER**: coverage only |
| 26 | Task 8 N4: a stale `state.rs` line reference; "lowercase hex" | R-28 | **FIX-IN-WAVE**: comment sweep |
| 27 | Batched fsync for the ledger (restore's append) | R-31 | **DEFER**: performance; a shared ledger; C0's list |
| 28 | E3's twin: after a failed restore, `apply` archives a seed and the next push sends it over the account's archived copy | R-37 | **DEFER**: only on the failed-restore path, and only in `archive/`; no live note is touched |
| 29 | E4c: prune stale `seed-hashes.json` entries | R-34 | **DROP**: an entry is hash-gated and can never match a changed or missing file |
| 30 | E5: a foreign `move` does not check the note's id at the old path | R-34 | **DEFER**: pre-existing; tombstones lack the same check; fold into I2's ruling |
| 31 | `judgelog.rs:5` names `history::sync` | Task 12 | **FIX-IN-WAVE**: comment sweep |
| 32 | `uievents::commit_opt_in` is dead | R-38 M3 | **DEFER**: outside C3′'s files; `anatomy.md` (H14) now calls it inert |
| 33 | **New C1**: no session refresh before a sync | this review | **FIX-IN-WAVE** for `run_sync`/`quit_flush`; the slot half is a merge-time edit in C1c's block (see the merge section) |
| 34 | **New I1**: a future-dated foreign record advances `pushed_through` | this review | **FIX-IN-WAVE**: clamp to the run's `now`, in `build_push` and `fold_confirmed`, with the test |
| 35 | **New I2**: both desktops run the machine steps; ids diverge per path | this review | **DEFER**: needs Quinn's ruling (runner lease vs reconcile by path); gate two-desktop use on it |
| 36 | **New M1**: cards dated by the UTC day | this review | **FIX-IN-WAVE**: one line |
| 37 | **New M2**: the `lib.rs:108-111` "sealed" claim; the `history` comments in `childproc`/`issues`/`runs`; `slot_argv`'s doc | this review | **FIX-IN-WAVE**: the comment sweep (with rows 24, 26 and 31) |
| 38 | **New M3**: PID temp folders leaked by two new tests | this review | **FIX-IN-WAVE**: the `cea59a9` pattern |
| 39 | **New M4**: the Backups restore and sync undo each other | this review | **DEFER**: a Quinn question |
| 40 | **New M5**: delete vs edit resolves by push order | this review | **DEFER**: nothing lost; with I2 |
| 41 | **New M6**: *Sync now* beside a slot's later child | this review | **DEFER**: pre-existing race class |
| 42 | **New M7**: the first-run view always ticks `sync` | this review | **DEFER**: cosmetic |

**Counts: FIX-IN-WAVE 18, DEFER 19, DROP 5** (42 rows).

**The wave's shape.** It is one dispatch, done in this order:
1. Correctness: rows 33, 34 and 36 (`state.rs`, `sync.rs`). Each is RED-first, with the named
   test.
2. Ruled items: rows 1–10 (`onboarding.rs`, `lms_link.rs`, `site/privacy.html`, `console.js`,
   tests).
3. The sweep: rows 24, 26, 31, 37 and 38.

Edits to the shared single-owner files (`state.rs`, `onboarding.rs`, `lms_link.rs`, `main.rs`,
`lib.rs`) land as a named hand-off commit, following R-C3′-exec-23's precedent. `console.js` and
`app/tests/scheduler.rs` edits must stay out of the C1c conflict hunks listed below. Nothing in the
wave touches `app/src/scheduler.rs`.

## Merge with C1c (PR #13)

**Where things stand.**
- **PR #9 (C1b) is already on `origin/main`** (`7477ec5`). Only the local `main` ref is stale
  (`642a749`). So the order left is C1c, then C3′, and relative to `origin/main` the 15
  C1b-inherited paths in Task 12's bucket 7 drop out of C3′'s diff.
- `c1c-first-day` (`12b46a3`) has already merged `origin/main`.
- C3′ and C1c share two merge bases, `5aacf65` and `773f529` (criss-cross); `ort` handles that.

**The trial merge.** `git merge-tree --write-tree --messages c1c-first-day c3-sync` conflicts in six
files. Everything else auto-merges.

**How it should land:**
1. The fix wave lands on `c3-sync` first. It touches no `app/src/scheduler.rs`.
2. After PR #13 merges, run `git merge --no-ff origin/main` on `c3-sync`, then resolve as below.
3. Add C1's slot half.
4. Run the whole gate, plus `wizard-check.py` (both new checks) and `settings-check.py`, on the
   merge commit.
5. Open the PR. Per R-C3′-exec-42, H14's HANDOFF block goes onto main at merge.

**H8a needs no rewrite.** The `sync` step comes from `slot_argv`, which auto-merges. C1c's loop
already runs every `slot_argv` step through the recorder (`steps.start(&args[0])`, `run_child`,
`steps.push((args[0].clone(), code))`), so `("sync", 0)` lands in `RunSummary.steps` and
`Scheduler.live` like any other step, and there is no bare `steps.push` to reconcile. C1c's
`FIRST_RUN_SAYS` already has `sync: "Syncing with your account"`. Its slot tests
(`a_slot_publishes_every_step_it_records_while_it_runs`,
`a_slot_names_the_step_in_progress_while_it_runs`) stay valid with `sync` landing first, and
`slot_argv`'s own tests already expect `["sync", "coursework", …]`. After the merge the order is
entitlement refresh → sync → coursework → ingest → judge → rank, so the engine gate reads a
refreshed cache. That closes Task 8's C1, as ruled.

### The six conflicted files

- **`app/src/scheduler.rs`, three hunks.**
  - (a) At merged ~636: C1c's `if lock(&cs.history).has_remote { steps.start("pull"); steps.push(sync_step(cs, "pull")); }`
    against C3′'s nothing. **Take C3′.** `cs.history` and `sync_step` no longer exist (C3′'s
    deletion of `fn sync_step` auto-merged).
  - (b) At ~778: **keep C3′'s F11 comment and C1c's `steps.start("backup");`**, and drop
    `steps.start("push"); steps.push(sync_step(cs, "push"));`. C1c's `match backed { … Err(_) => steps.idle() }`
    below it stays.
  - (c) At ~821: the result is `state::refresh_sync(cs);` followed by C1c's `let steps = steps.steps;`,
    in that order. `refresh_head` and `refresh_history` go.
  - Also, though not a conflict:
    - C1c's D8 comment (~724) about "a `pull (skipped: busy)`" is stale.
    - **C1's slot half goes here**, ahead of `let est = entitlement_state(cs);`. For a cloud vault,
      call `account::valid_access_token_at(&account::auth_base(&cfg.api_base)?, &cfg.anon_key, &cfg.session_credential_target, now)`
      unconditionally, and discard its error: it costs no round trip while >120 s remain.
- **`app/src/commands.rs`, one hunk (~372).** Take C1c's body (the one-line `state` command
  calling `pub fn state_envelope`, and `state_envelope` itself) with C3′'s wording of the comment
  above it ("behind a sync's own network call" and "never wait on the network", not git).
- **`app/static/console.js`, one hunk (~2031, `wizFinish`).**
  - Keep C3′'s two lines `WIZ.restoreNote = restoreSentence(r.restored); renderWizard();`.
  - Keep **C1c's** corrected R-C1-31 comment (`entitlement_now` writes no cache).
  - Drop C3′'s older R-C1-31 comment, which claims a cache write that C1c's final review showed
    is false.
- **`app/tests/scheduler.rs`, one hunk (the `use` line).** Take the union: `device_ok, engine_exe, entitlement_state, has_ics_url, ics_state, ingest_included, judge_plan, judge_state_in, lock, mode, prune_logs, run_child, run_slot_inner, should_retry, slot_argv, IcsState, JudgeArgs, JudgePlan, JudgeState, LiveSlot, Scheduler`.
- **`cloud/supabase/migrations/migrations_test.ts`, one hunk (the function-creation pin).**
  - One assertion: **28** = 18 at `5aacf65` + C1b's 3 + C1c's 1 (`charge_call`, `20260923000100`)
    + C3′'s 6.
  - One comment naming both streams' additions.
  - The view pin auto-merged to C3′'s **5**, which is correct (C1c adds no view).
  - The test's own message is the arbiter (R-C3′-exec-1).
- **`scripts/wizard-check.py`, two hunks.**
  - Take both sides: C1c's helpers and `check_first_run`, and C3′'s `check_picker_restore`. Each
    function ends at the shared `return bad`, so write `    return bad` plus two blank lines
    between them.
  - In `main`, keep both call blocks: the `console` page runs `check_first_run`, and `page2` runs
    `check_picker_restore`.

### Auto-merged files, checked by reading the merged tree

- **`app/tests/commands.rs`.** `sync_on_a_vault_with_no_account_is_calm_and_backup_needs_a_folder`
  carries both `cea59a9`'s clean-folder setup and teardown and C3′'s rename and assertions.
  Correct.
- **`site/privacy.html`.** C1c's diff there is C1b's, already on both sides; no C1c-only change
  collides with Task 11's rewrite. `PRIVACY_VERSION` stays `2026-09-24`.
- **`app/src/{account,scaffold,lms_link,onboarding}.rs`.** The hunks are disjoint: C1c's course
  `label` against C3′'s H11b, H11a and H16.
- **Git-era leftovers.** No code outside the three `scheduler.rs` hunks still names `cs.history`,
  `sync_step`, `refresh_head` or `refresh_history`. What remains is comments and assertions that
  say they are gone.
- **Build.** The merged tree was not compiled here: it cannot be built until the six hunks are
  resolved. The whole gate on the merge commit is the proof.

## Production readiness: Task 12's Step 7, checked

**Corrections to the list as written:**
- **(a) The privacy re-ask gate is CLEARED.** Quinn said so on 2026-09-24: both prod accounts are
  Quinn's own or test accounts. Close the item.
- **P5's "open half" is not open.** Quinn ruled on 2026-09-23 that "the guesses stand" (G3: `rank`
  keeps ranking past the grace). Close the item.
- **(e) The merge order.** PR #9 is already merged (`7477ec5`); what remains is C1c, then C3′
  (see the merge section).

**What the list misses:**
1. **C1's fix, verified live.** After the wave and the merge, a staging slot (and a *Sync now*)
   run more than an hour after the last session refresh must sync rather than print
   "signed out". Confirm prod's `jwt_expiry` is the repo's 3600. The same check also covers C2's
   `judge`, `coursework` and `ingest`.
2. **Production is still at C1 level**, per HANDOFF: "Production has nothing from C2 yet". C3′'s
   four migrations are not a push of their own. They go in **one ordered `db push --include-all`**
   carrying C2's ten (`20260911*`, `20260916*`), C3′'s four (`20260912*`), C1b's three
   (`20260917*`, `20260922*`) and C1c's one (`20260923*`), in timestamp order. The function
   deploys come after it: C2's eleven, C1b's changed `account`/`billing-checkout`, C1c's
   `ingest-coursework`, and `sync-push`/`sync-pull`.
3. **Deploy order matters for `account`.** H1's purge list names `sync_records`, `sync_notes` and
   `sync_usage`. If that `account` is deployed before the migrations, `DELETE /account` fails on a
   table that does not exist yet, and a student cannot delete their account.
   **Order: migrations, then `account`, then `sync-push`/`sync-pull`.** Read back
   `select public.sync_ceiling_bytes()` and the `knowlu-sync-prune` cron row.
4. **The site goes before the app.** `site/privacy.html` version 2026-09-24 (with the wave's
   §6 (vi)/(vii)) must be live on knowlu.com before any release that pushes note text to prod.
   Otherwise the live policy still says "they never hold the text of your notes" while clients
   upload it: ruling 2's one unfixable-afterwards risk.
5. **A live device proof is a release gate, not a note.** On staging, with two scratch vaults on
   one account (the controller's autonomous-proof harness), prove:
   - pull and push both ways;
   - a conflict card approved, and one rejected (6b);
   - a restore into a fresh wizard vault;
   - a lapsed-cache gate;
   - I1's clamp, using a hand-set `ts` in the future.

   The branch has only ever been one machine playing both parts. Remove the profiles afterward,
   per the standing cleanup rule.
6. **I2 needs Quinn's ruling before any student is told that two desktops work.** The one-liner
   already promises "every computer you sign in on opens on the same day". Until the ruling, a
   student with two desktops awake at slot time will diverge silently on doubly created notes.
7. **Known limitations to write down** (HANDOFF and the support notes): note **bodies** are
   last-writer-wins and never merged (M6 above, row 19); a Backups restore of an account vault
   re-applies later tombstones (M4); a delete against an edit resolves by push order (M5).
8. **`GET /account/export`** still omits the sync tables. It is correctly listed, and it is now
   the only export. It should land before the paid launch, not "someday".
9. **CI** must be green on the merge commit, on the self-hosted runner, since GitHub-hosted
   Actions minutes are stopped.

**Already right, and confirmed here:**
- Staging's `sync-push`/`sync-pull` bundles are current. No function source or `_shared` import
  they bundle has changed since the `067fe52` deploy, only tests.
- The 200 MiB ceiling, the 400-day `and not keep` prune, the `keep` generated column,
  select-own RLS with no client write policy, and the account scoping in `sync_db.ts` (Task 12's
  scan) all match the privacy text.

## Re-review of the fix wave (`7574168..7ccd784`)

Scoped re-review by the same reviewer, read-only. The package is `review-7574168..7ccd784.diff`,
the implementer's report is `final-fix-wave-report.md`, and the rulings are R-C3′-exec-43 and -44.

### Verdict: READY WITH RESIDUALS

All 18 FIX-IN-WAVE rows are ADDRESSED. None is open. Three residuals are due at merge time or
before release, and none blocks the branch:
- C1's slot half is C1c's Task 11 (R-C1c-13). It is present in C1c's worktree but not yet committed.
- Row 37's `slot_argv` doc lives in `scheduler.rs`, so it is fixed at the merge.
- C1's staging proof goes on the release checklist, per R-44 (4).

### The gate (run once, foreground, at `7ccd784`)

| Check | Result |
|---|---|
| `RUSTFLAGS="-D warnings" CARGO_TERM_COLOR=never cargo test --workspace --no-fail-fast` | exit 0; **1414 passed, 0 failed, 4 ignored** |
| ci.yml warning tally | `warnings: 1 accepted (.rsrc), 2 tallies, 0 other` |
| `oracle.rs` / `surface_oracle.rs` | 3/3 and 4/4 |
| the wave's 11 new tests (I1 ×2, M1, C1 ×3, the SYNC_SAYS pin, feed flags ×4) | all `ok` in the same run |
| `deno check` / `deno lint` (ci.yml's globs) | exit 0 / `Checked 126 files` |
| `deno test` with ci.yml's exact flags | **458 passed, 0 failed** |
| `wizard-check.py` / `settings-check.py` | `ok` / `ok` |

### The rows, one line each

| # | Row | Status |
|---|---|---|
| 1 | N1: per-feed flags; only a validated link reaches the account | **ADDRESSED** (`77c8abb`, H18). A feed that is validated and stored is skipped. One that is validated but not stored gets one retry, then goes to the vault. One that was never validated goes to the vault and never to the account. Residuals are below |
| 2 | N2: the 402 sentence | **ADDRESSED** (`lms_link.rs:630`, now "if it still can't") |
| 3 | N3: retry tests on their own paths; personal-calendar retry success | **ADDRESSED** (the new calendar test; the I1 tests are reworked onto the flags) |
| 4 | N4: privacy §6 (vi) | **ADDRESSED**: verbatim at `privacy.html:36` |
| 5 | N5: privacy §6 (vii) | **ADDRESSED**: verbatim at `:92`. `PRIVACY_VERSION` is unchanged, which is correct because 2026-09-24 is not yet published |
| 6 | Nit: `find(..).expect` | **ADDRESSED** (`sync_contract.rs:2688`) |
| 7 | R-39 N1 with R-43's copy | **ADDRESSED**: `SYNC_SAYS`; "can't confirm your subscription — changes stay on this computer"; the pin test forbids "subscription inactive" |
| 8 | R-39 N2: comment inaccuracies | **ADDRESSED** (three in code; the fourth is a pointer in an untracked report, recorded in the wave report instead, which is acceptable) |
| 9 | R-39 N3: stale git comment in `tests/scheduler.rs` | **ADDRESSED** |
| 10 | R-39 N4: page words pinned to the engine | **ADDRESSED**, with the caveat under "Regressions" |
| 24 | Task 8 N2: the `main.rs` comment | **ADDRESSED** (H19) |
| 26 | Task 8 N4: line references, "lowercase hex" | **ADDRESSED** (`entitle.rs`, `entitlement_gate.rs`; the line numbers are dropped) |
| 31 | `judgelog.rs:5` | **ADDRESSED** |
| 33 | C1: the session refresh | **ADDRESSED for the app half** (H17: `run_sync` and `quit_flush` run `refresh_session` first, under `vault_io`). The slot half is **C1c's**; see "C1's probe" below |
| 34 | I1: the push-cursor clamp | **ADDRESSED** (`build_push` and `fold_confirmed`); see "I1's probe" below |
| 36 | M1: cards dated by the vault's day | **ADDRESSED** (`cli::local_now(vault).date()`; the test picks a zone that makes it bite at any hour) |
| 37 | M2: the comment sweep | **ADDRESSED except `scheduler.rs:332`'s `slot_argv` doc**, which R-43 kept out of the wave and which is due at merge. `lib.rs`'s false "sealed" claim is gone |
| 38 | M3: leaked PID temp folders | **ADDRESSED**: both tests clean up before and after |

### I1's probe, re-run
- **The outcome.** In `a_foreign_record_from_a_clock_that_runs_ahead_never_hides_this_desktops_later_writes`,
  a foreign record dated now+1h arrives, then a local edit; on the second push the edit is sent,
  the future record is re-sent, and the saved cursor is ≤ `now`. That is exactly the probe from
  the final review, and it passes. The restore twin passes too.
- **The mechanism.** The journal is read in `ts` order, so every record newer than `now` sorts
  last. The records at or before `now` therefore always fill the page first, and a future record
  can never starve a local one.
- **Cosmetic effects.** A future record is re-upserted on every push until the clock passes it.
  That adds one row to "N record(s) up", and `merge-duplicates` rewrites the row's `device` column
  (harmless under P4(a)).
- **A residual, Minor, DEFER.** The clamp stops only records stamped in this desktop's future. A
  foreign record stamped between the wizard's own scaffold writes and the restore's `now` can
  still carry `fold_confirmed` past those scaffold records. The window is seconds wide, up to about
  60 s with H11b's retry, and needs the other desktop to push during the wizard's Finish. What it
  loses is the seed `create` records; the seeds' note text still syncs.

### C1's probe, re-run (the app half)
- `sync_now_refreshes_an_expired_session_before_it_pulls_or_pushes`: with a stored token that
  expired a minute ago, the loopback sees `POST /auth/v1/token?grant_type=refresh_token`, then the
  pull and the push, both carrying `Bearer fresh-at`, and Credential Manager ends up holding the
  fresh token. The quit push test proves the same for its one call.
- A refresh that fails leaves the sync running, and the cause is named in `lines`. R-44 deferred
  surfacing it any further.
- **The slot half, as it stands.**
  - The `c1c-first-day` ref is unchanged at `12b46a3`.
  - C1c's worktree holds uncommitted R-C1c-13 changes: a `session` pre-flight step in
    `run_slot_inner`, placed after the ingest-skip block and before the entitlement refresh, which
    calls `account::ensure_session_for_at(.., 45 * 60)`.
  - That covers every child, `sync` included.
  - It must be committed to PR #13 before C3′ merges.

### Regressions looked for
- **The `SYNC_SAYS` pin.** The test reads the engine's words out of `cloudmodel.rs` (both
  `Unavailable` labels) and `sync.rs` (`totals.skipped = Some("…`, `skipped: Some("…`, and the
  Transport, 401 and 402 arms). It requires each word to be a key, and pins five copies plus the
  "never 'subscription inactive'" rule.
  - A renamed word fails the test.
  - A moved or reformatted anchor fails loudly, because `literals_after` asserts that it found
    something.
  - **It cannot see a word built with `format!`** or taken from another label: the 403 ceiling
    reason, "the vault could not be read (…)", and CloudError's own labels. Those fall through to
    the engine's own first line in amber, which is honest, and is the design ("a new failure is
    still shown, just not yet translated").
  - No regression.
- **H18: can a validated link still be lost?** Walking `create_vault_in`'s three arms: stored
  means the account holds the link; not stored means a retry, then the vault; not validated means
  the vault. There is no path where a validated link ends up in neither the account nor the vault,
  with two narrow exceptions (Minor, DEFER):
  - (a) **The page trusts a stale `stored`.** Suppose a paste stored the link under the pending
    session, and the student then goes Back and signs in with a *different* account before Finish.
    `stored: true` skips the retry, the new account never gets the link, and the account vault's
    copy is blank.
  - (b) **Two paste replies can arrive out of order.** `WIZ.icsFeed` records whichever reply lands
    last. If the older one lands last, the current link reads as unvalidated and goes to the vault
    only, even though its own call may have stored it. It is kept, not lost.
- **H18's ruled trade-off, named explicitly because it reverses something Task 11's review
  wanted kept.**
  - `validate_for` refuses an *empty* LMS feed. `CaptureError::Empty` is an error for `lms_ics`
    and allowed only for `calendar_ics`.
  - So a school feed that has no events at onboarding, which is the norm in the first week of a
    term, never reaches the account now. Before the wave, the unconditional retry did send it.
  - It is not lost: it stays in the vault, and `ingest` falls back to it after `/ingest-ics`
    answers 404 (`ingest.rs:919-924`).
  - But the capability URL sits in the vault, and a second desktop's restore never learns it.
  - A paste that failed on a transient fetch error ends up the same way.
  - Accepted under R-44 (3). Worth a one-line production note and a later "re-save the feed" path.
- **H17: can the refresh stall *Sync now* on a dead network?** `valid_access_token_at` makes a
  network call only when the token has less than 120 s left, which after an hour of idle is
  always. It uses the account agent's `timeout_global(30 s)` (`account.rs:40`).
  - With no route, or a failed DNS lookup, the refresh and the pull both fail at once, and nothing
    changes.
  - On a black-holed network (a captive portal, dropped SYNs), *Sync now* now holds `vault_io` for
    up to 30 s + 120 s (the pull's `CALL_TIMEOUT`; the push is already skipped on transport)
    rather than 120 s. That is bounded. The console's writes are `(async)`, so the window stays
    responsive while they queue.
  - The quit push sits inside its 10 s cap, as before.
  - **Minor, DEFER:** skip the engine's network half when the refresh failed with
    `UNREACHABLE`, and record "offline" directly. That would win back the 120 s.
- **One consistency note for the merge (Minor).** C1c's slot pre-flight uses a 45-minute floor;
  H17 uses the 120 s margin. That is fine for one short *Sync now*. Aligning H17 on
  `ensure_session_for_at(.., 5 * 60)` once C1c's function lands would stop a token that has three
  minutes left from expiring between the pull and the push.

### The merge picture
- **Unchanged.** `git merge-tree --write-tree c1c-first-day c3-sync` still conflicts in the same six
  files, with the same hunk counts: `scheduler.rs` 3, `commands.rs` 1, `console.js` 1,
  `tests/scheduler.rs` 1, `migrations_test.ts` 1, `wizard-check.py` 2. The wave added no conflict.
- **C1c's uncommitted Task 11, layered on top.** I laid it over that trial merge with
  `git merge-file` on scratch copies of `account.rs`, `scheduler.rs`, `tests/account.rs` and
  `tests/scheduler.rs`. It adds **no** conflict. The test helper names do not collide either: C1c
  adds `ensure_session_for_at` and its two tests, while the wave adds `vault_with_an_expired_session`,
  `FRESH_SESSION`, `EMPTY_PULL`, `NOTHING_PUSHED` and three tests.
- **What changes in the merge recipe.** The "C1's slot half goes here" line in the merge section
  above is now C1c's own `session` block, so do not add a second one at the merge. Add instead:
  - `slot_argv`'s doc gets `sync → …` (row 37's remainder);
  - C1c's new test asserts `session_at < coursework_at`. After the merge, also assert that it
    precedes the `sync` step.
- Re-run `merge-tree` once C1c commits Task 11.

---

<!-- the fix wave: final-fix-wave-report.md -->

# C3′ final fix wave — report

Implementer: Claude Opus 5.5, 2026-09-24. FIX_BASE `7574168`, HEAD `7ccd784`, branch `c3-sync`.
Scope: the final review's 18 FIX-IN-WAVE rows under Ruling R-C3′-exec-43. C1's slot half (C1c)
and I2 are out of scope. `app/src/scheduler.rs` was not touched.

## Rows, one line each

RED means the test failed at `7574168` (or, where named, did not compile there); GREEN means it
passes at HEAD.

| Row | Item | Status | Test (RED → GREEN) |
|---|---|---|---|
| 1 | N1: per-feed `stored` flag; only validated links reach the account | **done** (`77c8abb`, H18) | `the_paste_says_whether_the_account_holds_the_link_and_never_sends_a_rejected_one`; `a_feed_the_account_already_holds_is_not_sent_again_and_never_touches_the_vault`; `a_link_that_failed_validation_never_reaches_the_account`; `the_wizard_carries_each_feeds_validated_and_stored_flags_into_the_plan`. RED: none compiles at base (no `finish_with`, no plan flags); the two onboarding cases are exactly what the unconditional retry did wrong |
| 2 | N2: the 402 sentence | **done** (`77c8abb`) | `a_402_from_sources_is_a_sentence_and_not_a_dead_end` gains the N2 assertions (RED: the file does not compile at base, for row 1's import) |
| 3 | N3: retry tests run their own paths; personal-calendar retry success | **done** (`77c8abb`) | `a_personal_calendar_that_failed_at_paste_is_saved_by_the_retry_and_stays_out_of_the_vault` (new); the two I1 retry tests reworked to the flags; the webcal/padded tests set `personal_calendar_validated` so they still run the retry-then-fallback path |
| 4 | N4: privacy §6 (vi) | **done** (`31b779a`) | `site.rs::the_review_amendments_i2_i3_and_i4_are_on_the_page`, RED → GREEN |
| 5 | N5: privacy §6 (vii) | **done** (`31b779a`) | same test, RED → GREEN |
| 6 | Nit: `sync_contract.rs` `find(..).expect` | **done** (`da312ed`) | comment/test hygiene, no test |
| 7 | R-39 N1: amber copy, as corrected by R-43 | **done** (`1726737`) | `the_sync_lines_words_are_the_engines_own_mapped_to_what_a_student_reads`, RED ("another sync is running" has no entry) → GREEN |
| 8 | R-39 N2: four comment inaccuracies | **done** (`1726737`, `da312ed`), one by record | console.js header and :389, `static_assets.rs`'s copy of the same words, `no_console.rs`'s "594". The fourth is in the Task 10 *report* (ignored `.superpowers/`, never committed); the correct pointer is recorded here instead: `state.rs`'s doc comment was at `:147` at `353deea` and `:141` at `074ca28`, not `:99` |
| 9 | R-39 N3: stale git comment, `app/tests/scheduler.rs` | **done** (`da312ed`) | comment only |
| 10 | R-39 N4: pin the page's words to the engine's; map `last_error` | **done** (`1726737`) | same test as row 7: it reads the words out of `engine/src/cloudmodel.rs` and `sync.rs` |
| 24 | Task 8 N2: `main.rs` gated-skip comment | **done** (`ec2ac89`, H19) | comment only |
| 26 | Task 8 N4: stale line refs, "lowercase hex" | **done** (`da312ed`) | comment only |
| 31 | `judgelog.rs:5` names `history::sync` | **done** (`da312ed`) | comment only |
| 33 | C1, the `run_sync` / *Sync now* / `quit_flush` half | **done** (`efba911`, H17) | `sync_now_refreshes_an_expired_session_before_it_pulls_or_pushes`, `the_quit_push_refreshes_an_expired_session_before_it_pushes`, `a_session_refresh_that_fails_is_named_in_the_sync_status`: all RED (stale token kept; refresh line absent) → GREEN |
| 34 | I1: clamp `pushed_through` to the run's `now` | **done** (`c696325`) | `a_foreign_record_from_a_clock_that_runs_ahead_never_hides_this_desktops_later_writes` (the review's test), `a_restored_record_dated_in_this_desktops_future_never_hides_the_first_local_write`: RED ("must reach the account: []") → GREEN |
| 36 | M1: cards dated by the vault's day | **done** (`c696325`) | `a_sync_card_is_dated_by_the_vaults_own_day_not_the_utc_one`: RED (`2026-09-24` vs local `2026-09-25`) → GREEN |
| 37 | M2: the comment sweep | **done except `slot_argv`'s doc** (`ec2ac89`, `da312ed`) | `lib.rs`'s "sealed" claim, `childproc`/`issues`/`runs` history mentions, `main.rs`. **Not done:** `app/src/scheduler.rs:332`'s `slot_argv` doc, because R-43 keeps that file out of the wave. Recorded for the merge below |
| 38 | M3: PID temp folders | **done** (`da312ed`) | test hygiene (`cea59a9`'s pattern), no test |

Done: 18 of 18, with one sub-item deferred by ruling (row 37's `slot_argv` doc) and one recorded
rather than edited (row 8's report pointer).

## How each fix works

- **I1** (`engine/src/sync.rs`): `build_push` computes `now = journal::now_ts(None)` once. A record
  dated after it is still sent, but `next.pushed_through`/`boundary` move only over records at or
  before it, so it is re-sent (idempotent on its hash) until the clock passes it. `fold_confirmed`
  gains a `now` argument and skips any confirmed record dated after it; `restore_all` passes the
  page's own clock.
- **M1**: `run_lines_with_client`'s `today` is `cli::local_now(vault).date()`, the vault's zone
  from `config/ingest.yaml`. The journal's day files stay UTC, which is the ledger's contract.
- **C1** (`app/src/state.rs`, H17): a private `refresh_session` calls
  `account::valid_access_token_at` (free while more than 120 s remain). It returns `None` for no
  account or no stored session, since the engine already names both, and `Some(why)` only when a
  session exists and the refresh failed. `sync_with_a_fresh_session` (used by `run_sync` and
  `quit_flush`, under `vault_io` as before) runs the engine anyway and names a failed refresh as the
  status's first line: `sync: the session could not be refreshed (<why>)`. A cloud.yaml the app
  cannot read is a fixed sentence, because its own error carries the vault path.
- **N1–N3** (H18): `lms_link::finish_with(kind, url, fetch, store)` is the old `finish` with its
  fetch and save handed in. It returns `stored`, and a link that fails validation returns before
  `store`. `capture_failed` carries `stored: false` too. The page keeps `{ url, stored }` per feed
  (`WIZ.icsFeed`, `WIZ.calFeed`), and `feedFlags` sends four `WizardPlan` flags that are true only
  for the value the field holds at Finish. In `create_vault_in`:
  - validated and stored: nothing is sent, and the vault stays blank;
  - validated, not stored: one retry, then the vault;
  - not validated: never sent to the account; kept in the vault.
  The flags are `serde(default)`, so an older page reads as "not validated".
- **Rows 7 and 10** (`app/static/console.js`): the `SYNC_SAYS` table and `syncSays(word, tone,
  prefix)`. Both halves of `renderSyncLine` and *Sync now*'s refusal (`syncRefusal`) use it. An
  unknown word keeps its own first line: amber for an error, calm after `sync skipped — ` for a
  skip. The pin test extracts the engine's words by anchor, so a moved anchor fails loudly.
- **Rows 4 and 5**: both page sentences are replaced verbatim from §6 (vi) and (vii).
  `PRIVACY_VERSION` is unchanged.

## The gate (run once, foreground, at HEAD `7ccd784`)

| Check | Result |
|---|---|
| `RUSTFLAGS="-D warnings" CARGO_TERM_COLOR=never cargo test --workspace --no-fail-fast` | exit 0; **1414 passed, 0 failed, 4 ignored** (the four by design); 112 s after a 65 s `--no-run` |
| ci.yml warning tally | **`warnings: 1 accepted (.rsrc), 2 tallies, 0 other`**. The script as run printed "0 accepted, 1 other" because PowerShell's `*>` redirect wrapped the `.rsrc` linker line across three lines (log lines 8–10). Joined, it is the accepted line, and the log holds exactly three `warning:` lines |
| `oracle.rs` / `surface_oracle.rs` | 3/3 and 4/4; `git diff --quiet 7574168` on both files |
| fixtures / `backup.rs` | `git status --porcelain --untracked-files=all engine/tests/fixtures/` empty; `backup.rs` equals `main` |
| `deno check` / `deno lint` (ci.yml's globs) | exit 0 / `Checked 126 files` |
| `deno test` with ci.yml's exact flags | **458 passed, 0 failed** |
| `python scripts/wizard-check.py` / `settings-check.py` | `ok` / `ok` |
| privacy one-liner | byte-identical, once each, in `site/privacy.html`, `site/index.html`, `console.js`, `engine/tests/site.rs` |
| CRs | 0 in all 25 touched files |
| `app/src/scheduler.rs` | untouched |
| trial merge with `c1c-first-day` (`git merge-tree`) | the same six conflicted files as the review, and the same hunk count in each (`console.js` 1, `app/tests/scheduler.rs` 1, `commands.rs` 1, `wizard-check.py` 2). The wave adds no conflict |

`target\debug\knowlu-engine.exe` was blanked once by an app build mid-wave. Rebuilding it with
`cargo build -p knowlu-engine --bin knowlu-engine` restored
`discover_coursework_against_the_real_engine_names_the_missing_credential`. It was non-zero before
the gate.

## Commits (FIX_BASE `7574168`..HEAD)

1. `c696325` sync: a future-dated record never moves the push cursor, and a card is dated by the
   vault's day (I1, M1)
2. `efba911` hand-off H17: Sync now and the quit push refresh the session first (C1)
3. `77c8abb` hand-off H18: Finish retries only a feed that validated and did not land (N1, N2, N3)
4. `1726737` page: the sync line says what to do, in words pinned to the engine's (R-39 N1/N2/N4,
   R-43)
5. `31b779a` site: privacy wording §6 (vi) and (vii) (N4, N5)
6. `da312ed` tests+comments: the sweep (rows 6, 8, 9, 26, 31, 37, 38)
7. `ec2ac89` hand-off H19: lib.rs and main.rs comments made true (M2, Task 8 N2)
8. `7ccd784` docs: anatomy.md's sync line is the mapped copy

Each message ends with the two trailers. The three edits to shared single-owner files are named
hand-offs, following R-C3′-exec-23's precedent:
- H17: `state.rs`;
- H18: `onboarding.rs` and `lms_link.rs`;
- H19: `lib.rs` and `main.rs`.

## For the merge with C1c (no scheduler.rs edits were made here)

- **C1's slot half** (R-C1c-13): at the top of `run_slot_inner`'s entitlement block, for a cloud
  vault, call `account::valid_access_token_at` unconditionally and discard its error, as the
  review's merge section says.
- **Row 37's remainder**: `app/src/scheduler.rs`'s `slot_argv` doc still reads `coursework →
  ingest → judge → rank`. It should read `sync → coursework → ingest → judge → rank`.
- C1c's D8 comment (~724) about "a `pull (skipped: busy)`" is stale, per the review.

## Concerns

1. **One unruled string.** R-39 and R-43 worded the signed-out and entitlement copy.
   `offline: the account could not be reached` also needed an entry in the mapped table, and I gave
   it `offline — changes stay on this computer` (amber, as before), following the ruled pattern. It
   is Quinn's call; it is one line in `SYNC_SAYS`, and the pin test only requires the key.
2. **C1's naming is a line, not a headline.** When a refresh fails, the sync still runs, and
   `last_error` stays the engine's word. For a revoked refresh token, and for a transport failure,
   that word is honest. For a rare auth-service 5xx or 429 on the refresh, the engine's resulting
   401 still reads "signed out — sign in to sync". The real cause is in `SyncStatus.lines`, which
   no view shows today. The diagnostics blob and the issue report show only `last_error` and
   `skipped`. On *Sync now*, the named line lives only in `cs.sync`; the engine's status file does
   not carry it.
3. **A rejected link now stays in the vault**, never in the account. R-41 ruled only that it must
   not reach the account. Keeping it in the vault follows R-40's "never lost" rule, and a vault with
   no account already does this; ingest's cloud-then-local order then uses it. The alternative
   would be to drop it.
4. **C1 needs the staging proof the review names**: a *Sync now* more than an hour after the last
   refresh. It is covered here only on loopback.
5. I2 is untouched, as ruled.

---

<!-- the merge: merge-report.md -->

# C3′ merge report: origin/main (PR #13, C1c) into c3-sync

Status: **DONE_WITH_CONCERNS** (the concerns are small and listed at the end).

- Merge commit: `825f5f4`. Its parents are `7ccd784` (c3-sync) and `f30da69` (origin/main).
- Pushed: `origin/c3-sync` = `825f5f4`. No PR was opened.
- One commit. The test extension and the comment edits went into the merge commit itself; there
  is no follow-up commit.

## The six conflicted files

`git merge --no-ff origin/main` conflicted in exactly the six files the brief names, with the hunk
counts the review predicted: scheduler.rs 3, commands.rs 1, console.js 1, tests/scheduler.rs 1,
migrations_test.ts 1, wizard-check.py 2. Everything else auto-merged. `CLAUDE.md` auto-merged
main's `CI_SELF_HOSTED` paragraph into C3′'s text.

| File | Resolution |
|---|---|
| `app/src/scheduler.rs` (a), the pull hunk | Took C3′'s side, which is empty. `if lock(&cs.history).has_remote { … sync_step(cs, "pull") }` is gone. |
| `app/src/scheduler.rs` (b), the backup hunk | Kept C3′'s F11 comment ("the same tree the slot's own `sync` step can rewrite"), then C1c's `steps.start("backup");`. Dropped `steps.start("push"); steps.push(sync_step(cs, "push"));`. C1c's `match backed { … Err(_) => steps.idle() }` is unchanged below it. |
| `app/src/scheduler.rs` (c), the end-of-slot hunk | C3′'s I4 comment and `state::refresh_sync(cs);`, then C1c's `let steps = steps.steps;`, in that order. `refresh_head` and `refresh_history` are gone. |
| `app/src/commands.rs` | Took C1c's body: the one-line `state` command calling `pub fn state_envelope`, and `state_envelope` itself. Kept C3′'s wording of the comment above it: "behind a sync's own network call", "never wait on the network". |
| `app/static/console.js` (`wizFinish`) | Kept C3′'s `WIZ.restoreNote = restoreSentence(r.restored); renderWizard();` and its M1 comment. Kept C1c's corrected R-C1-31 comment (`entitlement_now` writes NO cache). Dropped C3′'s older R-C1-31 comment. |
| `app/tests/scheduler.rs` (the `use` line) | Took the union, exactly as the recipe lists it, which adds `ingest_included`, `lock` and `LiveSlot`. |
| `cloud/supabase/migrations/migrations_test.ts` | One assertion, **28**. One comment, which keeps C3′'s three sync entries, appends C1c's `charge_call` (`20260923000100`), and closes with "C3′ adds six and C1c one, on top of the 21 both streams inherited". The view pin auto-merged at 5. |
| `scripts/wizard-check.py` | Both functions: C3′'s `check_picker_restore`, which ends in `    return bad` plus two blank lines, then C1c's `state_polls`, `first_run_block`, `listed` and `check_first_run`. `main()` keeps both call blocks: `page2` runs `check_picker_restore`, and `console` runs `check_first_run`. |

### The brief's amendments

1. **No second session refresh.** I added no `valid_access_token_at` call ahead of
   `let est = entitlement_state(cs);`. C1c's `session` block (`steps.start("session")` →
   `ensure_session_for_at(…, 45 * 60)`) is kept exactly as main has it, and there is one refresh
   per slot. I edited only the comment text inside it, to make it true now that the merge has
   landed (see item 4).
2. **The session-before-sync test.** I extended C1c's
   `a_near_expiry_session_is_refreshed_before_any_engine_step_and_a_failure_is_named` rather than
   adding a sibling. Its vault runs `sync`, because `slot_argv` puts `sync` first unconditionally.
   The test gains four assertions:
   - `session_at < sync_at`, with the message "the session is refreshed before the sync step,
     because the engine's sync spends the token";
   - `sync_at < coursework_at`;
   - exactly one step whose name starts with `session`;
   - and its doc comment now names `sync`.

   The test passes in the gate run.
3. **Row 37.** `slot_argv`'s doc now reads `sync → coursework → **ingest** → **judge** → rank`.
4. **The stale comments.**
   - D8 in `run_slot_inner`: the `pull (skipped: busy)` sentence is replaced. It now says that
     `sync` is not filtered into the runner-log lines, because the child always exits 0 and
     records its own outcome, and that git sync and that lock collision are gone.
   - The R-C1c-13 comment above the `skips` filter: dropped its closing clause ("the same reason a
     busy sync lock is named but a routine pull is not").
   - The R-C1c-13 session-block comment: "(coursework, ingest, judge; sync once C3′ lands)" now
     reads "(sync, coursework, ingest, judge)". "The C3′ merge … keeps exactly this one session
     block" is now in the past tense, and the comment names the test that pins the order. The
     code is untouched.
5. **The lock check.** See below.
6. **The migrations pin.** See below.

## Leftover git-era names

`git grep -E "cs\.history|sync_step|refresh_head|refresh_history"` outside `docs/` finds:

- `app/src/scheduler.rs:671`: this is a false match. The comment's test name contains
  `…_any_engine_step…`. It is not `sync_step`.
- `app/tests/commands.rs:529`: a comment saying that `state::refresh_head` "no longer exist[s]".
- `app/tests/no_git.rs:7,53`: the forbidden-names test, which asserts they are gone.
- `engine/src/coursework.rs:2259-2264`: a local variable named `sync_step` in a coursework unit
  test. It is unrelated to the scheduler function.

No code names them. `docs/` still names them in historical plans and reports, which are left
untouched.

## The gate (every command in the foreground, on the tree committed as `825f5f4`)

| Check | Result |
|---|---|
| `RUSTFLAGS="-D warnings" CARGO_TERM_COLOR=never cargo test --workspace --no-fail-fast` | exit 0. **1451 passed, 0 failed, 4 ignored**: the four by-design ones (`run_slot_end_to_end`, traps 4 and 5, `real_runtime…`). 164 s, after a separate `--no-run` build |
| ci.yml's warning classification | `warnings: 1 accepted (.rsrc), 2 tallies, 0 other`. The raw grep printed "0 accepted, 1 other", because Windows PowerShell 5.1's `*>` redirection wrapped the linker line after `…MSVCRT_Micro` and pushed `.rsrc merge failure: multiple non-default manifests` onto the next line. That line is the accepted one. ci.yml's `pwsh` `Tee-Object` does not wrap |
| `oracle.rs` / `surface_oracle.rs` | 3/3 and 4/4 |
| `pwsh -File scripts/ci/eol-check.ps1` | `eol contract holds over 449 files`, exit 0. The six resolved files are `i/lf w/lf` |
| `deno check` (ci.yml's globs; deno 2.9.6) | exit 0 |
| `deno lint` (ci.yml's paths) | `Checked 126 files`, exit 0 |
| `deno test` (ci.yml's exact flags) | **461 passed, 0 failed**, exit 0 |
| `python scripts/wizard-check.py` | `ok`. Both `check_picker_restore` and `check_first_run` ran |
| `python scripts/settings-check.py` | `ok` |
| Tauri commands (script over both `generate_handler!` lists) | 42 / 29 / 61 distinct, which matches CLAUDE.md |

The logs are in the session scratchpad: `merge-build.log`, `merge-test.log` and
`merge-deno-test.log`.

## The migrations pin

- `every SECURITY DEFINER or writing function in every migration has execute revoked, unless marked authenticated-by-design ... ok`
- The assertion is `assertEquals(parsed, 28, "today's corpus should parse exactly 28 function creations")`,
  and it passes, so the scan parses exactly 28.
- The count is 18 at `5aacf65`, plus C1b's 3, plus C1c's 1 (`charge_call`, `20260923000100`),
  plus C3′'s 6.
- Main gained only one migration relative to C3′ (`git diff --name-status 7ccd784 origin/main -- cloud/supabase/migrations/`
  shows one `A`: `20260923000100_first_day_cap.sql`), so the recipe's arithmetic holds.
- The view pin stays at 5.

## The lock grep (`SESSION_REFRESH_LOCK`, R-C1c-exec-14)

`grep -n "refresh_at(\|save_session(" app/src/*.rs` on the merged tree:

| Site | Refreshes a stored session? | Under the lock? |
|---|---|---|
| `account.rs:500` `refresh_at` → `:502` `save_session` in `valid_access_token_at` | yes | **yes**: `:497` takes the lock before `load_session` |
| `account.rs:522` `refresh_at` → `:524` `save_session` in `ensure_session_for_at` | yes | **yes**: `:519` |
| `account.rs:155` `save_session` in `move_session` | no. It moves the session from pending to the profile at onboarding, with no refresh | n/a |
| `account.rs:560` `save_session(PENDING_TARGET, …)` in `verify_email_code` | no. This is a fresh sign-in | n/a |
| `account.rs:1164` `save_session(PENDING_TARGET, …)` in `google_sign_in` | no. This is a fresh sign-in | n/a |

- `refresh_at` has exactly two callers, and both hold the lock across load → refresh → save.
- Every other refresh reaches the store through `valid_access_token_at` or
  `ensure_session_for_at`. That covers:
  - the telemetry send;
  - `lms_link`;
  - `report`;
  - the account commands;
  - the scheduler's `session` block;
  - **C3′'s H17**, `state.rs::refresh_session`, which `sync_with_a_fresh_session` calls for
    *Sync now* and the quit push, and which reaches the store through `valid_access_token_at`
    (`state.rs:194`).
- The engine never refreshes: `engine/src/` has no `grant_type=refresh_token`.
- Nothing needed fixing.

## Concerns

1. **An edit outside the brief's scope.** Inside C1c's session block, I edited comment text
   only, because "sync once C3′ lands" is false after the merge. The block's code is byte-identical
   to main's. If "keep that block exactly" was meant to cover its comments too, revert those
   lines.
2. **An inherited whitespace warning.** `git diff --cached --check` flags
   `docs/plans/2026-09-24-commitment-model-phase2-plan.md:4290: new blank line at EOF`. The file
   arrived from main unchanged, and I did not fix it here.
3. **Carried forward, not new.** The final review's consistency note still stands: H17's *Sync
   now* uses `valid_access_token_at`'s 120 s margin, while the slot uses a 45-minute floor.
   Aligning H17 on `ensure_session_for_at(.., 5 * 60)` is optional and was not in this brief.
   The review's other residuals are also unchanged:
   - M7: the first-run view always ticks `sync`;
   - C1's staging proof, which is on the release checklist.
