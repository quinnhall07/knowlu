# Knowlu plan 2, Part A — SDD ledger (rulings R-P2-1–R-P2-12, reviews, deferred findings, final review)

The controller ledger of the subagent-driven execution of Part A (Tasks 1–6) of `docs/superpowers/plans/2026-09-05-knowlu-runner-leaves-plan.md`, 2026-09-05. Preserved verbatim from the worktree workspace (`.superpowers/sdd/`, ignored by the VCS), which is kept in place for Part B. The ▶ RESUME HERE block at the top is the live one: it says what to check before Task 7 is dispatched. Line numbers cited by reviews refer to the files at the commits named beside them.

---

# SDD ledger — plan: docs/superpowers/plans/2026-09-05-knowlu-runner-leaves-plan.md

Spec: docs/superpowers/specs/2026-09-04-knowlu-independent-app-design.md (+ console spec 2026-09-02, S2 spec 2026-08-31). Reachable; rulings are against it.
Branch: worktree-knowlu-plan-2 in worktree .claude/worktrees/knowlu-plan-1 (same directory as plan 1). Start: main 2abfcea, clean. Session: https://claude.ai/code/session_014MQESyCz34TjYypAojCJh4
Part B GATE: Tasks 7–13 wait for cutover plan Task 9 step 2 on main (no harness call in scripts/local-run.ps1, no state/dual-run-log.md). Not checked yet — expected ~2026-09-18. Record the check here before dispatching Task 7.

## ▶ RESUME HERE (updated 2026-09-05 after Part A closed)
- **Part A (Tasks 1–6) is COMPLETE and merged to main** (see the closing lines at the bottom for SHAs). Do not re-dispatch Tasks 1–6.
- **Part B (Tasks 7–13) is GATED.** Before dispatching Task 7, check on `main`: `scripts/local-run.ps1` has no harness call (no `dual-run.ps1`, no snapshot step) and `state/dual-run-log.md` does not exist — i.e. cutover plan Task 9 step 2 landed (~2026-09-18). Record the check here (`Gate check <date>: <what you saw>`), then run `task-brief … 7` and continue the loop. Part B changes what the runner writes — the dual-run scripts become historical (Task 10) and the check is `cargo test` + both oracles.
- Part B moments that are Quinn's (ask each individually, in one or two sentences, when reached — see memory `surface-outstanding-items-one-at-a-time`): engine binary name `knowlu-engine.exe` (Task 11 dispatch); credential rotation + `cmdkey /generic:knowlu/zybooks` and `/generic:knowlu/vhl` re-store from a non-Claude terminal (Task 12 step 6); desktop `Disable-ScheduledTask quinn-ops-local-runner` (Task 9, not blocking); the both-binaries-beside-each-other release rule (Task 11 merge).
- Follow-ups carried from Part A's final review (fold into Part B tasks where they fit, else plan 3): skip the drain-thread join when the git cap fired (shared buffers; `src/history.rs::git_with` and `app/src/scheduler.rs::run_child`); `run_slot_inner` already-running → `reason: Some(...)`; one `VIEWS` map in console.js; `quit-<ts>.txt` growth; disabled-button styling; `pub(crate)` on `git_with`/`GIT_TIMEOUT`; a bare-remote `quit_flush` test and a `timed_out: true` test; `"main"` hardcoded as the sync gate (plan 3 onboarding: a friend's vault on `master`).
- Side channels: daily CS 100 P2 dry-run (main checkout, Python, Quinn-authorized) — 09-05 clean; cutover dual-run reads 5 of 14 as of 09-05 13:38 (late catch-up after standby); next 09-05 18:00.
- Track layout used for Part A (for reference): Tasks 1→2→3 sequential on the primary branch; Tasks 4→5 on an isolated agent worktree merged by the controller (R-P2-5; merges were conflict-free); Task 6 in parallel.

## Pre-flight conflict scan (2026-09-05)

| Pair / task | Produces vs consumes | Found |
|---|---|---|
| T1 ↔ T2 ↔ T3 | all edit console.js, index.html, console.css, static_assets.rs; T2's test carries T3's assertions later | sequential on one branch; T2 adds its test without the Issues lines, T3 adds them (plan text already resolves this) — OK |
| T1 renderer ↔ console.css tokens | plan CSS uses `--mute`, `--ink2`; console.css defines `--t1..--t4`, `--acc`, `--mono` (no `--mute`/`--ink2`) | plan says "use the existing spellings verbatim" → R-P2-4 |
| T1 test ↔ digest phrase | test forbids `" events in today's digest"` and requires `=== 1 ? " event" : " events"` | renderDeck line 257 has the fixed plural; plan's Step 6 replaces it — consistent |
| T1 "15 tests" | static_assets.rs has 13 `#[test]` today; T1 adds one (14), T2 one more (15) | arithmetic off by one in T1 step 8; the requirement is "all green", count is informational — noted, no ruling needed |
| T2 test: `data-id="` count == `data-kind="` count | console.js today: template stamps at rows/cards/drawer dl carry both; the drawer's Delete button (line 446) stamps `data-id` with no `data-kind`; selectors `[data-id="` (lines 152, 548, 689) also match the needle | the assertion as written fails on pre-existing code → R-P2-2 |
| T3 ↔ not-built placeholder | T2/T3 test forbids "is not built yet"; line 402's else-branch and `#main-notbuilt` become dead once every nav view has a renderer | R-P2-3 |
| T4 self-consistency | tests name helpers `repo_pair→(remote,a,b)`, `push_a_commit_from`, `write_note`, `ctx`, `journal`, `remote_tip`, `tip_of`; module has `repo_pair→(bare, work)` 2-tuple, `edit`, `git_env`, `copy_tree` only | plan anticipates ("use real names; add beside the others") → R-P2-6 |
| T4 placement of the branch check | plan: "before `if !out.status.has_remote`"; F12 is about rebase/push, which need a remote | a no-remote vault on `master` would be refused for nothing → R-P2-1 |
| T4 ↔ dual-run scripts | history.rs is a src/ change before the gate | run diff-engines.ps1 ×3 fixtures + diff-engines-notes.ps1 (plan step 4) — controller runs them at merge if the isolated agent cannot (they refuse the working tree and need the venv in the main checkout) |
| T5 self-consistency: quit_flush | test calls `quit_flush(&cs, cap)`; prose settles on `quit_flush_inner(cs, cap)` in state.rs + `quit_flush(app, cap)` in tray.rs; `thread::scope` joins after the timeout so the 10 s cap (F10) would not be real | → R-P2-7 |
| T5 ↔ T1–T3 files | T5 touches console.js (`last slot … (attempt N)`) and static_assets.rs (literal `"attempt "`) | isolated track; merge conflicts (append regions) resolved by the controller → R-P2-5 |
| T5 tick arithmetic | `n % 360 == 0` "once an hour" | tick is 10 s (`n % 6` = 60 s mode refresh, `n % 30` = 5 min sync) — consistent |
| T5 backup under vault_io | `backup_now_inner` (commands.rs:268) takes no lock; `run_slot_inner` backup at scheduler.rs:298 after the child wait | ordering vault_io → cs.lock preserved; never held across a child wait — consistent with plan 1's ordering note |
| T5 DST test ↔ schedule.rs | `RunnerConfig { name, times, tz, grace_minutes }` (4 fields), test helper is `cfg()` not `local_cfg()`; `Zoned`/`Span` imports needed | trivial adaptation, plan already allows it |
| T6 ↔ cutover plan | registration block at cutover plan lines 253–257 (`$arg`, `$action`, `$triggers`, `$settings`); `scripts/cutover/` does not exist yet; `local-run.ps1` device regex `(?m)^\s*device:\s*(\S+)` | consistent; T6 creates the directory |
| Global: gate | Part A touches src/ only in T4 (history.rs) and T5 (schedule.rs tests) | consistent |

### Rulings (pre-flight)
- Ruling R-P2-1: the side-branch refusal in `sync` goes AFTER the `has_remote` early return and before the fetch — F12 forbids rebasing or pushing a side branch, both of which need a remote; a local-only vault on `master` must keep committing — costs one line moved; if wrong, a no-remote side-branch vault commits silently, which is today's behaviour.
- Ruling R-P2-2: the data-id/data-kind equality test counts template stamps only — the needles are `' data-id="'` and `' data-kind="'` (leading space; a CSS selector `[data-id="` never matches) — and the drawer's Delete button stops carrying `data-id`: its existing `data-del` attribute carries the note id and the delete handler reads it from there. A control is not an observed object (R-T15b); it was being observed and reporting kind null — costs one handler line; if wrong, one telemetry row is lost.
- Ruling R-P2-3: Task 3 removes the "is not built yet" else-branch and the `#main-notbuilt` element (index.html + the two `hidden` lines) once every nav view has a renderer; an unknown hash keeps whatever fallback the router already has — a dead placeholder is a lie waiting to be shown — costs nothing.
- Ruling R-P2-4: the plan's `--mute` → `--t4`, `--ink2` → `--t2`, `--acc`/`--mono` as spelled in console.css — the plan itself says use the file's spellings — costs nothing.
- Ruling R-P2-5: Task 4 then Task 5 run on an isolated agent worktree in parallel with Tasks 1–3; the controller merges each into the plan branch and resolves conflicts in console.js/static_assets.rs by keeping both sides (they append) — Task 5's two page lines are not worth serialising a 30-minute build behind three page tasks — costs one manual merge.
- Ruling R-P2-6: Task 4's test semantics bind, its helper names do not — the implementer uses the module's real `repo_pair → (bare, work)` and `edit`, adds a second-clone helper beside them if pushing a remote commit needs one, and keeps every assertion the plan's tests make — costs nothing.
- Ruling R-P2-7: `pub fn quit_flush(cs: &ConsoleState, cap: Duration, then: impl FnOnce(QuitFlush) + Send) -> QuitFlush` lives in `app/src/state.rs`; the work runs inside `std::thread::scope`, `then` is called exactly once — with the worker's result when it arrives, or with `timed_out: true` at the cap — BEFORE the scope joins; the tray's `then` writes `logs/quit-<ts>.txt` and calls `std::process::exit(0)` (settings already saved), so the 10 s cap is real (F10) and the scope join never blocks the user; the test passes `|_| {}` and asserts on the return. No `_inner`, no AppHandle wrapper — costs one closure parameter; if wrong, quit waits on the 60 s git cap instead of 10 s.

## Cutover week reads (side channel)
- 4 of 14 as of 09-04 18:02 (from plan 1's ledger). Next reads: 09-05 12:00 / 18:00.

## Tasks
- Task 1: dispatched 2026-09-05 01:40 (sonnet, primary worktree), BASE 2abfcea.
- Task 4: dispatched 2026-09-05 01:40 (sonnet, isolated agent worktree from origin/main + merge of the plan branch), BASE 2abfcea.
- Cutover read 2026-09-05 01:42: log unchanged since 09-04 18:02 (DIFFERS runner-log.md, recheck CLEAN(drift)); still 4 of 14; next 09-05 12:00.
- Task 4: implementer DONE 69218ff on branch worktree-agent-a19082454b77b7439 (794 passed, 0 warnings). Concern: git_with returns on timeout WITHOUT joining the drain threads (claims a Windows grandchild-pipe deadlock). Review (opus) dispatched 01:55 with that deviation to adjudicate.
- Task 1: implementer DONE f87385a (14/14 static_assets). Deviations declared: pinned .flagpop preserveAcross count 2→3 in an existing test (mechanical), one doc-comment line. Note for Tasks 2/3 dispatch: each adds another preserveAcross(host, ".flagpop") occurrence — the pinned count moves 3→4→5. Review (sonnet) dispatched 02:10.
- Task 4: review (opus) — spec compliant, R-P2-1/R-P2-6 honoured, timeout deviation APPROVED as a real Windows grandchild-pipe deadlock. 2 Important: (1) flat 100 ms poll = ~100 ms floor per git call (status() ~500 ms, sync ~1.7 s) — plan-mandated code; (2) timeout leaks the grandchild + 2 reader threads per timeout. Minors: unbounded wait after failed kill; try_wait error orphans child; conflicts_pending pure alias; comment cites git status not diff-filter=U; stopped-rebase test never calls sync; nothing pins "before the fetch"; pub seam; local `status` shadows fn; state.rs "cheap" doc.
- Ruling R-P2-8: on timeout history::git_with kills the process tree (`taskkill /T /F /PID` on Windows, then kill/wait) and then joins the drain threads unconditionally — the join-skip deviation retires; no new crate dependency in the engine; mirrors Task 5 kill_tree — a leaked wedged credential helper per sync cycle is a slow resource leak in a tray app — costs one taskkill spawn per timeout.
- Task 4: fix round 1/5 dispatched 02:20 (implementer resumed): poll ramp 2→100 ms clamped to remaining; R-P2-8 tree kill + unconditional join; kill before try_wait error return; rename shadowing local; comment citation; strengthen the stopped-rebase test to call sync. Accepted as-is: alias, pub seam, app/src/state.rs doc.
- Task 1: review (sonnet) — Approved. Spec compliant; F8/F19/F20 verified; both declared deviations mechanical. Minors (all plan-mandated code): `changes` array not excluded (typeof object); no `.row.dec.gone` CSS; bindDecisionsView lacks the pre-check bindDeck has (decideCard guards anyway); `#dec-hint` `.hint` class has no CSS rule. Carry: `.hint` rule + `Array.isArray` exclusion ride with Task 3 (page cleanup moment, R-P2-3); the rest to the final review.
- Task 1: complete — f87385a (review approved, 0 fix rounds).
- Ruling R-P2-9: Task 2 defers the `!js.contains("is not built yet")` assertion to Task 3 along with the three Issues lines — Issues is still the placeholder after Task 2 and the plan already splits that test across the two tasks — costs nothing.
- Task 2: dispatched 02:30 (sonnet, primary worktree), BASE f87385a.
- Task 4: fix round 1 landed cf640ef (794 passed, 0 warnings; tree kill via taskkill /T /F then unconditional join; poll ramp; try_wait error kills tree; rename; comment; stopped-rebase test calls sync). Re-review (sonnet) dispatched 02:45.
- 03:00: Task 2 implementer and Task 4 re-reviewer both stalled (stream watchdog, 600 s); both resumed from their transcripts.
- Task 4: re-review (sonnet) — all 6 findings ADDRESSED, no new breakage, Cargo.toml untouched. Residual (ruling-inherent, noted for the final review): if taskkill cannot reach a grandchild (missing binary, privilege, PID reuse) the unconditional join can hang again; non-Windows arm is immediate-child-only. HEAD cf640ef on worktree-agent-a19082454b77b7439; merge into the plan branch + diff-engines ×3 fixtures + diff-engines-notes pending Task 2 commit (the implementer is editing this worktree).
- Task 5: dispatched 03:20 (opus, isolated agent worktree; merges the plan branch then Task 4 branch worktree-agent-a19082454b77b7439 first; BASE = its post-merge HEAD, reported).
- Task 2: implementer DONE dd769f6 (15/15 static_assets; headless PASS). Deviation: `state.empty.info` does not exist — used `state.texts.info` (rail renderer does too). Review (sonnet) dispatched 03:35.
- Task 4: merged into the plan branch as 9956038 (merge commit, src/history.rs only). diff-engines ×3 fixtures + diff-engines-notes running in the background (log: scratchpad/dual-run-task4.log).
- Task 6: dispatched 03:40 (sonnet, primary worktree — docs/scripts only, no overlap with Tasks 3/5), BASE 9956038.
- Task 4: dual-run after merge 9956038 — diff-engines vault-s1 / vault-s1-migrated / vault-full: No differences, exit 0 each; diff-engines-notes: No differences, exit 0 (13:36). Task 4: complete — 69218ff + cf640ef, merged 9956038 (1 fix round).
- Cutover read 2026-09-05 13:37: no 12:00 line yet — laptop in Modern Standby 02:20→13:31 (Kernel-Power 107/1); task quinn-ops-local-runner State=Running, LastRunTime 13:37:16, result 267009 (running) = StartWhenAvailable catch-up in progress. LATE, not missed; nothing acted on. Re-read after it ends. Cloud 08:14 WARN: "approvals: 2 expired; events: blount: not an ICS response" (routine side, not plan 2).
- Task 2: review (sonnet) — Approved; `state.texts.info` deviation confirmed right (Empty has no info field); R-P2-2 load-bearing (5 = 5 stamps). Minors: empty `kind` yields a leading " · " in the gtk meta; no disabled-button styling. Task 2: complete — dd769f6 (0 fix rounds).
- Ruling R-P2-10: Task 3 uses `p.empty_text` alone for the Issues empty state — `state.empty.issues` does not exist (Empty = active/must_do/recommended/the_day; Texts = coming_up/info/closed/offline) and the engine already sends issues_panel.empty_text — costs nothing. Riders carried into Task 3 (page-cleanup moment): R-P2-3 placeholder removal; a `.hint` CSS rule; `Array.isArray` exclusion for `changes` in renderDecisionsView; `meta.filter(Boolean)` in the gtk renderer.
- Task 3: dispatched 13:50 (sonnet, primary worktree), BASE 9956038 (Task 6 may interleave a docs commit).
- Cutover read 2026-09-05 13:40: 12:00 slot ran LATE at 13:38 (standby catch-up) — dual python-live git=35227e5 CLEAN files=252 exits 0/0; local ok coursework (30; 0/0) + today.md (149 active). Executed runs: 5 of 14 (09-03 13:05, 09-03 19:34, 09-04 12:14, 09-04 18:02, 09-05 13:38). Next: 09-05 18:00.
- Task 6: implementer DONE bd7e8ae (4 files, 59 insertions; parse check "parses"; nothing executed). Note: README says the harness scripts still live in scripts/ until Task 10. Review (sonnet) dispatched 14:00.
- Task 6: review (sonnet) — Approved; checklist byte-identical to the brief; registration block identical to the cutover plan bar the mandated $PSScriptRoot-derived $repo; guard fooling cases tested. Minor: task-exists guard fails open on a query error. Task 6: complete — bd7e8ae (0 fix rounds).
- Task 3: implementer DONE 4c2a6ad (15/15 static_assets; headless PASS incl. no #main-notbuilt; riders applied). Judgment call: e.stopPropagation() in bindIssuesView to stop the document-level row-click fallback re-opening the drawer with the issue id after "open the object". Review (sonnet) dispatched 14:15 with that to adjudicate.
- Task 5: implementer DONE — bc3aa30 (app) + ad0474f (engine DST pin) on worktree-agent-a1612b77049f55460, BASE 10a2dbc (plan branch + Task 4 merged). app 46 passed/1 ignored, only the pre-existing .rsrc line; root 808 passed, 0 warnings, oracles green. Concerns: quit_flush reads cached has_remote (quit <60 s after launch may skip the push); retry wiring unit-tested only; backoff per-process. Review (opus) dispatched 14:30.
- Task 3: review (sonnet) — Approved; stopPropagation justified (the open-target button otherwise falls through to openDrawer(issue id)); R-P2-3 removal clean, route() falls back to "today" for an unknown hash (new, reported). Minors (final review): dead `if (!applyEnvelope(...)) return` in resolveIssue; showRefusal ignores its first arg so Close/Resolve refusals get no row flash (pre-existing from Task 2); route() view-name literal is a third list of view names. Task 3: complete — 4c2a6ad (0 fix rounds).
- 14:45: Task 5 reviewer dropped (ECONNRESET) before reading; resumed from scratch. Pre-check: git merge-tree HEAD..worktree-agent-a1612b77049f55460 is conflict-free.
- Task 5: review (opus) — Needs fixes. 1 Important (plan-mandated): DST pin has no slot in the 2026-03-08 gap (02:00–03:00); slots_on drops a gap time via .ok(). Minors: LOGS_KEPT doc 2× optimistic + quit logs share the prune budget; attempts map never pruned; attempts overwritten on refusal / refusal clears the ladder; backoff delays later same-day slots (comment); kill_tree fallback unbounded without taskkill; quit test !synced cannot fail; no timed_out test; attach_scheduler poison-intolerant. Verified: vault_io one-way on all six sites, never across run_child; then-before-join real; process::exit with SyncLock held is safe (2-min stale rule).
- Task 5: fix round 1/5 dispatched 15:05 (implementer resumed): "02:30" in the DST pin (fix slots_on if it drops the slot); prune only slot-* + LOGS_KEPT 60 + true comment; prune attempts keys not today; refusal neither increments nor clears; one comment. Deferred to the final review: taskkill-missing fallback; !synced vacuity; timed_out test; commands.rs map_err.
- Task 5: fix round 1 landed cf7fee0 (app) + 8f6f79b (engine test). DST gap: "02:30" added, PASSES both days (jiff Compatible shifts 02:30→03:30-05:00) — no slots_on change. Deviation: attempts key is now the local Zoned string, not UTC (a UTC key would have put 18:00 CT on the next UTC day and the sweep would delete it at once). Re-review (sonnet) dispatched 15:25.
- Task 5: re-review (sonnet) — all 5 findings ADDRESSED, key-format deviation judged correct (due_slot only considers today; Zoned string starts YYYY-MM-DD). Task 5: complete — bc3aa30 + ad0474f + cf7fee0 + 8f6f79b, merged d660364 (1 fix round). Deferred to the final review: taskkill-missing fallback; !synced vacuity; no timed_out test; commands.rs attach_scheduler map_err; in_flight advisory race.
- Part A merged on the plan branch at d660364 (Tasks 1–6 complete). 15:40: dual-run scripts (dual-run-task5.log) and full root+app suites (tests-merged.log) running in the background; final whole-branch review package review-2abfcea..d660364.diff written.
- Dual-run at d660364 (14:30 by the tool clock; earlier ledger times after 13:40 were approximate): diff-engines vault-s1 / vault-s1-migrated / vault-full No differences exit 0; diff-engines-notes No differences exit 0.
- Merged branch d660364 green: root cargo test 808 passed / 0 failed / 2 ignored, 0 warnings; app cargo test 46 passed / 0 failed / 1 ignored, only the pre-existing .rsrc linker line. Final whole-branch review next (opus) with final-review-context.md + review-2abfcea..d660364.diff.
- Final whole-branch review dispatched (opus) at 14:45 by the tool clock: range 2abfcea..d660364, inputs final-review-context.md + the 130 KB diff file; test evidence handed in the prompt.
- Final review (opus) — Ready with fixes. 3 Important, all page: (1) .row.dec/.gtk-row/.iss inherit .row four-column grid (plan defect, faithfully implemented; never rendered — console-shots shoots only the default hash); (2) Good to know Close fires close_info twice (no stopPropagation; document handler calls closeInfoItem again); (3) clicking .dnote in a dec/iss row falls through to openDrawer(row id). Minors: chips unstyled outside .flagpop; main .empty unstyled; showRefusal ignores el (pass id); git_with normal-exit join can hang on an inherited pipe (shared-buffer refinement of R-P2-8); attach_scheduler poison-intolerant; quit-*.txt unbounded; route() third view list; alias/pub; phase-2 doc "topline build" + "60 s"; "main" hardcoded (friend on master never syncs — plan 3); LOGS_KEPT 60 not in the ruling list; state.rs "cheap" doc. Triage: fix now = gone CSS, showRefusal id, before-the-fetch assertion, attach_scheduler; accept = 2,5,7,11,12,13,18,19; follow-up = 3,6,8,10,14,15,17. Rulings: none disputed; R-P2-8 refinement suggested (skip the join when the cap fired, shared buffers).
- Ruling R-P2-11: ONE fix wave on opus in the primary worktree per final-fix-wave-brief.md: A1–A8 (page + console-shots.py shooting the three new hashes with PNG evidence), B1 (before-the-fetch assertion), C1 (attach_scheduler poison tolerance), D1–D2 (phase-2 doc wording). LOGS_KEPT = 60 is accepted as a Task 5 fix-round outcome (one log per step). Follow-ups carried to Part B / plan 3: R-P2-8 shared-buffer refinement (both crates); run_slot_inner already-running → reason:Some; single VIEWS map; quit-log growth; disabled-button styling; pub(crate) seam; bare-remote quit test; timed_out test; "main" hardcoded for a friend on master (plan 3 onboarding).
- Final fix wave dispatched (opus) at 15:00 by the tool clock; fix base d660364.
- Final fix wave DONE: fe72ff6 (page A1–A8 + console-shots.py shoots the three new hashes), 9dfb41a (B1 origin/main guard), e3cc87c (C1), f3b4361 (D1–D2). static_assets 15/15; app suite only the .rsrc line; root 795 + oracles, 0 warnings; console-shots 32 shots exit 0. PNGs in shots-final-fix-wave/. Implementer concerns: .row.iss lacks align-items flex-start; ⚑ glyph box in headless chromium (pre-existing font); tray.rs diagnostics_text two poison-intolerant locks. Scoped re-review (sonnet) dispatched 15:30 by the tool clock.
- Dual-run at f3b4361 (15:09): diff-engines ×3 fixtures and diff-engines-notes all No differences, exit 0. Main checkout: left-right 0 0, clean, main = 35a6c43 (state commits on top of 2abfcea; 2abfcea is an ancestor) — the merge into main will be a true merge of state/ vs branch files. Final review text preserved at final-review-report.md.
- Final fix wave re-review (sonnet): A1–A8, B1, C1, D1–D2 all ADDRESSED (A1 from the pixels: three tracks correct at 1512 and 820); no new breakage. Residuals: .row.iss not top-aligned; ⚑ renders as a glyph box in headless chromium only (font coverage; WebView2 has Segoe UI Symbol — not a defect); tray.rs poison-intolerant locks at 29/46/130/131 (same class as C1).
- Ruling R-P2-12: the controller closes the two mechanical residuals before merge — `.row.iss { align-items: flex-start }` and the four tray.rs locks → scheduler::lock (pause_item ×2, history, backup) — the same one-line precedent as plan 1 R-F5; a dispatch costs more than the edit and the app suite re-runs — costs one app test run. Follow-ups carried (see final-review-report.md triage) unchanged.
- Controller fix R-P2-12 committed 2fef808 (app/src/tray.rs four locks → scheduler::lock; `.row.iss { align-items: flex-start }` as its own rule — the static test pins the grid line verbatim, which caught the first attempt). App suite after it: 20 + 12 (1 ignored) + 15 passed, only the pre-existing .rsrc line. Root suite unchanged since f3b4361 (795 + oracles, 0 warnings); dual-run clean at f3b4361.
- CLOSE (Part A): plan branch head after the docs commit = the commit that adds this report; merge into main from the main checkout with --no-ff (left-right was 0 0; main = 35a6c43 + state commits, 2abfcea is an ancestor); push; verify left-right 0 0 after. Workspace kept for Part B (not deleted — the plan is half-executed).


---

## Appendix A: the final whole-branch review (preserved)

# Final whole-branch review — Knowlu plan 2, Part A (opus, 2026-09-05; range 2abfcea..d660364)

## Strengths (verified by the reviewer)
- `state::quit_flush` calls `then` on the caller's thread before the scope joins, so the tray's `std::process::exit(0)` fires at the cap even with a worker inside a 60 s `git_with`. The residual stale `state/.sync.lock` is covered by `LOCK_STALE_SECS = 120`.
- The retry-ladder key is the local `Zoned` string: `due_slot` only returns today's slots, so the daily sweep never evicts the key it is about to write; a UTC key would have swept an 18:00 CT slot at once.
- The DST pin is non-vacuous: `02:30` does not exist on 2026-03-08 and exercises `slots_on`'s `to_zoned(…).ok()` filter; the 25-hour fall-back walk would catch a double fire.
- `git_with`'s ramping poll (2 → 100 ms, clamped) is a real improvement over the plan's flat 100 ms.
- Behavioural tests throughout: real bare remote + second clone, real `cmd /c ping` for both tree kills, real mirror for quit, a decoy `quit-*.txt` that retention must keep.
- Global constraints hold: no `engine/`, fixtures, `config/`, `Cargo.toml`/`Cargo.lock` change; `uievents.rs` untouched; no `http(s)://`, `import `, `require(` under `app/static/`; `mark_seen` from `endOfLook` alone; no secret, machine or user path; merges carry only their branch's files; no line-ending flip; nothing from Part B; nothing renamed.

## Critical
None. A no-remote vault returns before the new branch check; the tick is still gated on `mode() == App`.

## Important (all fixed in the final fix wave — see final-fix-wave-brief.md)
1. All three new views inherit `.row`'s four-column grid (`5px minmax(0,1fr) 88px 50px`) — flag in the pip track, actions in the 88 px track, the Issues title in the 5 px track. A plan defect faithfully implemented; nothing on the branch had rendered these rows (console-shots shot only the default hash). Fix: per-kind `grid-template-columns`.
2. Good to know's Close fires `close_info` twice per click (no `stopPropagation`; the document handler calls `closeInfoItem` again).
3. Clicking the `.dnote` input in a Decisions/Issues row falls through to `openDrawer(row id)`.

## Minor
- Decisions verdict clicks bubble to the `.b` branch and emit `why_expanded` with the literal `"task"` after a refusal (closed by the same propagation fix).
- `.chip` styled only under `.flagpop` (Issues chips unstyled); `.empty` styled only under `.rblock`; `showRefusal` ignores its first argument (pass the id); `git_with`'s normal-exit join can block on an inherited pipe (shared-buffer refinement of R-P2-8); `attach_scheduler` poison-intolerant with swallowed errors; `quit-<ts>.txt` unbounded over time; `route()` is a third view-name list; `conflicts_pending` alias and `pub` seam; phase-2 doc "topline build" and "60 s" wordings; `"main"` hardcoded as the sync gate (a friend on `master` never syncs — plan 3 onboarding); `LOGS_KEPT = 60` deviates from the plan's 30 (accepted, one log per step); `refresh_history` doc "cheap".

## Triage of the deferred minors (reviewer's verdicts)
Fix now: `.row.dec.gone` CSS; `showRefusal` id; "before the fetch" assertion (origin/main unchanged); `attach_scheduler` poison tolerance. Accept: `bindDecisionsView` pre-check; dead `applyEnvelope` return; alias; non-Windows arm; "cheap" doc; taskkill-missing fallback; backoff delays later slots; register script fail-open guard. Follow-up: disabled-button styling; single VIEWS map; `pub(crate)` seam; unconditional join (shared buffers, both crates); bare-remote quit test; `timed_out` test; `run_slot_inner` already-running → `reason: Some`.

## Rulings questioned
None disputed. R-P2-8: agree with the tree kill; refine by skipping the join when the cap fired (shared buffers). R-P2-3: right call; the third view list is the price.

## Assessment
Ready to merge: With fixes. The engine and app hardening is the strongest work on the branch; what blocked it was the page (grid inheritance, double Close, drawer on note-field click) — one small CSS/JS commit plus a screenshot check.


---

## Appendix B: the final fix wave brief (what the wave changed)

# Knowlu plan 2 Part A — final fix wave (ONE dispatch; ruling R-P2-11)

Branch `worktree-knowlu-plan-2` at d660364. Every item below comes from the whole-branch final review. Fix exactly these; nothing else. Work in C:\Users\danie\GitHub\quinn-ops\.claude\worktrees\knowlu-plan-1.

## A. Page (`app/static/console.css`, `app/static/console.js`, `app/tests/static_assets.rs` only if an assertion must move)

A1 (Important) — the three new row kinds inherit `.row { display: grid; grid-template-columns: 5px minmax(0,1fr) 88px 50px; }` (console.css ~125) and lay out wrong (the flag button lands in the 5 px pip track; `.acts` in the 88 px track; the Issues title in the 5 px track). Add beside the existing new rules:
```css
.row.dec, .row.gtk-row { grid-template-columns: 5px minmax(0,1fr) auto; }
.row.iss { grid-template-columns: minmax(0,1fr) auto; }
```
Check the narrow breakpoint (~console.css:250) does not undo it; if it does, add the same overrides inside that media block.

A2 (Important) — `bindGoodToKnowView` (console.js ~826) handles `button[data-close-info]` without `e.stopPropagation()`, and the document-level click handler (~897) calls `closeInfoItem(id)` again: two `close_info` invokes per click. Add `e.stopPropagation();` before the call, as `bindIssuesView` already does.

A3 (Important) — clicking the `.dnote` input (or anything in `.acts` that is not a `.b`) in a `.row.dec` / `.row.iss` falls through to the document handler's `openDrawer(row.getAttribute("data-id"))` (~917). In `bindDecisionsView` and `bindIssuesView`, stop propagation for any click whose target is inside the row's `.acts` (return early after stopping if it was not a handled button). This also stops the Approve/Reject/Snooze bubble that reached the `.b` branch and emitted `why_expanded` with the literal `"task"` kind after a refusal.

A4 — add `.row.dec.gone` to the existing `.card.gone` rule set (console.css ~158/162) so a decided Decisions row gets the same fade/pointer-events treatment as the deck card.

A5 — `.chip` is styled only under `.flagpop` (console.css ~362-364); the Issues row's `<span class="chip on">` renders unstyled. Widen the selector so `.row.iss .chip` gets the same look.

A6 — `.empty` is styled only under `.rblock` (console.css ~272). Add a `main .empty` rule with the same look so the four main views' empty states match the rail.

A7 — `resolveIssue` and `closeInfoItem` pass a row element as `showRefusal`'s first argument, which it never reads. Change both calls to `showRefusal(null, m, id)` so the toast carries the title prefix and the row flashes (read `showRefusal`'s signature at ~634 first and pass the arguments it actually uses).

A8 — verify the layout: `scripts/console-shots.py` shoots only the default hash. Extend it minimally so it also shoots `#decisions`, `#good-to-know`, `#issues` (a loop over hashes; keep the existing output for the default hash unchanged; name the new files by view). Run it headless in `.wv` against `tests/fixtures/surface-today-full.json` (it has approval cards for Decisions; for Good to know and Issues, plant one row each the way Tasks 2/3's throwaway checks did, using a modified copy of the fixture fed to the same server). Put the PNGs in the SDD workspace dir (never in the repo) and, in the report, name each PNG and state in one sentence what the row layout shows (flag at the left edge, title in the wide column, actions at the right; chips styled). Headless only — no desktop input, no exe capture.

## B. Engine test (`src/history.rs`, tests only)

B1 — R-P2-1 has no regression guard: in `a_side_branch_is_committed_locally_and_neither_rebased_nor_pushed`, capture `git rev-parse origin/main` in the vault BEFORE the sync and assert it is unchanged AFTER (the remote has moved in that test, so a fetch would change it). Run `cargo test history::` and then the full root `cargo test` (0 warnings).

## C. App (`app/src/commands.rs`)

C1 — `attach_scheduler` (~58, 60) uses `.lock().map_err(|_| "lock")?` on `sch.last` / `sch.mode_device` — the mutexes Task 5 made poison-tolerant — and every caller does `let _ = attach_scheduler(...)`, so a poisoned mutex silently drops `last_slot`/`scheduler` from every poll. Use poison-tolerant locking there (`lock().unwrap_or_else(|e| e.into_inner())`, or make `scheduler::lock` `pub(crate)` and call it). Run `cd app; cargo test` (zero new warnings; the `.rsrc` linker line is pre-existing).

## D. Docs (`docs/runners/knowlu-phase-2.md`)

D1 — precondition 3 says the exe's `build` is read from the "topline"; the page does not print it — it is in the tray's Copy-diagnostics blob (`topline.console_build` in the payload). Reword that one clause to name the Copy-diagnostics blob.
D2 — step 4 says "Within 60 s"; housekeeping re-reads the key every 60 s and the page polls every 60 s, so the honest bound is about two minutes. Change to "Within two minutes".
CRLF file: check `od -c docs/runners/knowlu-phase-2.md | head -2` and preserve; `git diff --stat` must show only the changed lines.

## Commits
Up to four commits, one per section (A page incl. `scripts/console-shots.py`; B engine test; C app; D docs), `git add` specific paths only; messages via `-F <file>` in the SDD workspace dir; subjects of the form `page: … (Knowlu plan 2, final fix wave A)`; each ending with:
Co-Authored-By: Claude Fable 5.1 <noreply@anthropic.com>
Claude-Session: https://claude.ai/code/session_014MQESyCz34TjYypAojCJh4

## Gate before reporting
`cd app; cargo test --test static_assets` (15 green) and `cd app; cargo test`; root `cargo test` 0 warnings with the oracles green. Line endings per file (check each edited file with `od -c | head -2`; the page files may be LF or CRLF — keep what each has). Write the report to `final-fix-wave-report.md` in the SDD workspace dir: per item what changed, the covering test/check and its output, the PNG names with the one-sentence layout statement, files changed, concerns.
