# Knowlu plan 4a (the friends shell) — SDD ledger (rulings R-P4a-1–R-P4a-28, reviews, deferred findings, final review, fix wave)

The controller ledger of the subagent-driven execution of `docs/superpowers/plans/2026-09-05-knowlu-friends-shell-plan.md` (Tasks 1–11), 2026-09-05/06. Preserved verbatim from the worktree workspace (`.superpowers/sdd/`, ignored by the VCS), which is kept in place until Task 8's key-gated updater step lands. The ▶ RESUME HERE block at the top is the live one. Line numbers cited by reviews refer to the files at the commits named beside them.

---

# SDD ledger — plan: docs/superpowers/plans/2026-09-05-knowlu-friends-shell-plan.md

Spec: docs/superpowers/specs/2026-09-05-knowlu-friends-shell-design.md (on main 28c4f10). Reachable; rulings are against it, and against the Knowlu spec 2026-09-04 where it defers.
Branch: worktree-knowlu-plan-4a in worktree .claude/worktrees/knowlu-plan-1 (same directory as plans 1 and 2). Start: main 636b904 + spec commit 5c288cd. Session: https://claude.ai/code/session_014MQESyCz34TjYypAojCJh4
Quinn 2026-09-05: "I don't want to wait to keep building" → plan 4a (shell half) runs now, before plan 3; reverse week cut to 3 days (cutover plan Task 9 amended).

## ▶ RESUME HERE (updated 2026-09-06 ~03:40, plan 4a CLOSED)
- Plan 4a is COMPLETE: Tasks 1–11 on branch worktree-knowlu-plan-4a, final head 76ad1f2 (= 1837638 Task 11 + fix wave ec1c7a1/b9a64a2/7432b7c + controller fix-up 76ad1f2). Final whole-branch review: "ready to merge with fixes"; the fix wave's scoped re-review: all findings addressed. This ledger is preserved at docs/superpowers/reports/2026-09-06-knowlu-plan-4a-sdd-ledger.md; the merge to main is done from the MAIN checkout (see the last lines below for the sha).
- DONE 2026-09-06 ~05:10: Task 8's key-gated updater step landed (1737785 + ee286bc; review + re-review clean; R-P4a-30: no updater permission granted to the page) and merged to main as 9940e6e. NOTHING of plan 4a is owed any more. Historical text of the gated-step item follows for the record: Task 8's key-gated updater step — `tauri-plugin-updater` + `updater:default` capability + `plugins.updater` (pubkey + endpoint) + console-only registration + real `check_and_stage`/`install_staged` bodies + `createUpdaterArtifacts: true` (the coupling test flips with it) + R-P4a-24's three items (tray install path writes refusal/failure into `up.last_error`; the daily check spawned/timeboxed; single-flight download with temp-then-rename) + the four staged-update observables. Brief: task-8-brief.md steps 8–9 and the Task 8 review line. It needs Quinn's PUBLIC key: ask once, with the exact command, and only when nothing else is pending — Quinn generates the pair from a non-Claude terminal, stores the private half in Credential Manager `knowlu/updater-key` (release.ps1 reads it by that name), and pastes ONLY the public key here. Keep this workspace until that commit lands.
- Follow-ups ledgered for plan 4b (do not do them here): migration errors / picker-fallback reasons surfaced in the windows-subsystem build (ConsoleState::open param); picker-check.py into the repo or dropped; stale .knowlu-new-* sweep on wizard start; ingest fetch-failed test + summary-line assertion; ics_state NotFound vs locked; orphan credential on an abandoned wizard + credentialsStranded() test; Forget-this-profile; an already-empty foreign quinn-ops\logs\ removed by the fold; the failed-single-move path re-pinned by a test. Plan 3: the QUINN_OPS_DEVICE test race.
- Side channels: daily CS 100 P2 dry-run — 09-06 00:31 clean, next 09-07 afternoon (P2 posts Mon); cutover reads 6 of 14 as of 09-05 18:02, next 09-06 12:00/18:00; plan 2 Part B gated on cutover Task 9 step 2 (~09-13/14). Quinn-owned, ask individually when due: updater keypair (now due, after the merge); Cloudflare (site deploy); Trusted Signing + signtool (first signed release); first friend's name/machine.
- Live-machine note for the first launch of a merged build (from the fix-wave re-review): it creates %LOCALAPPDATA%\knowlu and folds exactly the two slot-* logs out of quinn-ops\logs\; dual\, rehearsal\, scratch\, shots\ and the other 105 logs stay. Still scratch copies only until the go-live checklist after G2.
## Rulings on the plan draft (2026-09-05)
- Ruling R-P4a-1: keep `tauri-plugin-single-instance` (identifier-keyed: one Knowlu per machine); no per-profile mutex; profile switch = spawn `knowlu.exe --vault <other> --after-pid <pid>` then exit, the child waits ≤10 s for the pid; spec §2's "two profiles open at once" withdrawn in the close task — a second simultaneous profile is not a 4a need and the plugin's focus-on-relaunch behaviour is — costs one relaunch per switch.
- Ruling R-P4a-2: profile switch relaunches the exe with `--vault` — ConsoleState/tray/scheduler/autostart are fixed at startup — costs nothing.
- Ruling R-P4a-3: `pick_folder` and `adopt_vault` land in Task 2 so the picker ships alone — costs nothing.
- Ruling R-P4a-4: settings is an overlay panel, not a `View` — a view name would be refused by `surface::View::parse` on every poll — costs nothing.
- Ruling R-P4a-5: five settings rows in Task 6; the Updates row lands with Task 7 where its command exists; the spec's §4 table count is fixed at close — costs nothing.
- Ruling R-P4a-6: the onboarding seed record is `WriteContext::new("system:migration", "cli")` — the S1 migration's exact shape; the journal's `via` vocabulary does not grow (spec decision 5 said "onboarding"; overruled for vocabulary fidelity; spec amended at close) — costs one word of honesty in the record.
- Ruling R-P4a-7: the Rust engine has NO `quinn-ops ingest` subcommand (verified: `main.rs` Command enum; `ingest.rs` holds `sync_tasks` etc. but no orchestrator; the rewrite spec promised it) — a friend's LMS feed would never be fetched. Plan 4a gains a task: port `engine/ingest.py:main` to `quinn-ops ingest` (tests first, `Fetchers`-style seam, fixture `blackboard.ics`, no network), and `slot_argv` becomes coursework → ingest → rank (effective on Quinn's vault only when plan 2 Task 9 flips `scheduler: app`; the routine's Python ingest overlaps safely via `state/ingest-seen.md`). It is the plan's one `src/` change, gated by root tests + oracles + both dual-run scripts — costs one engine task; if wrong, a friend has a console over an empty vault.
- Ruling R-P4a-8: `log_dir(&ConsoleState)` per profile; Task 7's updater-config commit is gated on Quinn's public key, asked at Task 7 dispatch — costs nothing.

## Tasks
- Ruling R-P4a-9: Python `ingest` exits 1 on an empty `ics_url`, and the slot treats non-zero as engine failure (backoff, tray Warn) — so `slot_argv` includes the ingest step only when `config/ingest.yaml` has a non-empty `ics_url`, recording `("ingest (skipped: no ics_url)", 0)` otherwise; the engine command stays byte-faithful to Python — costs one yaml read per slot; if wrong, a friend who skipped the LMS panel would see a permanent Warn.
- Facts for the new ingest task: engine/ingest.py:main = config load, ics_url, timezone default America/Chicago, fetch (urllib 60 s), parse_ics, sync_tasks(actor agent:ingest.blackboard, via, run_id), print log + "ingest: N events, M action(s)"; exit 1 on unreadable/missing config, empty url, fetch failure; NO run record, NO runner-log line. Python main tests: tests/test_ingest.py:302,312,341,359 (39 tests in the file; the other 35 cover the already-ported core).
- Plan revised (3173 lines, 11 tasks; Task 5 = engine ingest). Controller read Task 5 whole: faithful to engine/ingest.py:main; every name it uses exists (calfeed::fetch_ics, journal::VIAS as clap value_parser, pystr::read_text/write_text, journal::device_name, run_slot_inner `steps`). Open points from the writer accepted: no run record/runner-log for ingest (a Part B Task 8 decision); two documented deviations (unknown tz / empty file → 1 with a message instead of a traceback). Independent plan review (opus) dispatched.
- Plan review 1 (opus): READY AFTER FIXES — 6 blocking (wizard unreachable via main.rs restructure; create/restore refuse an existing dest and panel 2 has no name/default path; three static tests use the wrong data-id needle; block_on inside #[command(async)] panics; updater plugin registered without its required pubkey config panics at startup; release.ps1 never supplies TAURI_SIGNING_PRIVATE_KEY), 13 should-fix, 8 minor. All rulings honoured; ingest port faithful; every signature verified against the code and the plugin sources (dialog 2.7.3, updater 2.11.0, windows 0.62.2).
- Rulings R-P4a-10…17 written to plan-review-1-and-rulings.md (main written out in full; parent+name for the vault with the spec default path; async commands; Task 8 split so the plugin lands with its config in one key-gated step; release.ps1 reads knowlu/updater-key from Credential Manager via CredReadW into env for the build only; Add another / Switch profile in scope; sign via bundle.windows.signCommand + scripts/sign.ps1; has_ics_url distinguishes config-unreadable). Revision 2 dispatched to the plan writer.
- 22:18: tauri-cli 2.11.4 installed on the laptop (cargo install --locked) ahead of Task 1; the spike still decides whether the NSIS bundler works on the GNU target.
- Plan revision 2 received (3820 lines, CRLF): B1–B6, S1–S13, M1–M8 applied; verified by grep (space-prefixed needles ×3, async commands, signCommand + sign.ps1, CredReadW helper in release.ps1, fn main written out, no ellipses). Open points accepted: default vault folder name "Vault" under Documents\Knowlu; the skip-step test drives run_slot_inner end to end (watch its runtime).
- Ruling R-P4a-18: the independent plan review (opus, 97 checks against the code and the plugin sources) stands as the pre-flight conflict scan — its check list is the scan table; the controller added the Task 5 name verification — a second full read of 3820 lines would repeat it — costs nothing; the review loop remains the net.
- Plan committed on worktree-knowlu-plan-4a; merge to main next; then Task 1 (spike) dispatch.
- Plan revision 2 received (3820 lines, CRLF): B1–B6, S1–S13, M1–M8 applied; verified by grep (space-prefixed needles ×3, async commands, signCommand + sign.ps1, CredReadW helper in release.ps1, fn main written out, no ellipses). Open points accepted: default vault folder name "Vault" under Documents\Knowlu; the skip-step test drives run_slot_inner end to end (watch its runtime).
- Ruling R-P4a-18: the independent plan review (opus, 97 checks against the code and the plugin sources) stands as the pre-flight conflict scan — its check list is the scan table; the controller added the Task 5 name verification — a second full read of 3820 lines would repeat it — costs nothing; the review loop remains the net.
- Plan committed 80f92b6 on worktree-knowlu-plan-4a. Task 1 (spike): dispatched 22:30 (opus, primary worktree), BASE 80f92b6.

## Tasks (execution)
- Task 1: implementer DONE 90d7110 (report-only commit; outcome A: cargo tauri build works on GNU — NSIS 3.11 fetched by the CLI, WebView2 bootstrapper embedded, sidecar suffix stripped to quinn-ops.exe; installer 4.42 MiB; app/binaries/ and tauri.conf.json reverted). Review (sonnet) dispatched 22:45.
- Track layout for plan 4a: primary (this worktree) Task 2 → 4 → 6 → 7 → 8 → 9; isolated A: Task 3 (credentials, sonnet); isolated B: Task 5 (engine ingest, opus; controller runs the dual-run scripts at merge); isolated C: Task 10 (site, sonnet). Dispatched 22:45: Task 2 (opus, primary, BASE 90d7110), Task 3, Task 5, Task 10 (each merges the plan branch first; BASE reported).
- Task 1: review (sonnet) — Approved; four binding checks verified directly (clean tree, single-file commit, bundle.active false restored, no app/binaries). Identifier: settled by spec decision 8 (com.knowlu.desktop), Task 9 applies it. Task 1: complete — 90d7110 (0 fix rounds).
- Task 3: implementer DONE aa1f436 on worktree-agent-a059a3facfe75eeaa (BASE d6f77ad = its merge of the plan branch); credentials tests 2/2, app suite green, cmdkey shows no knowlu/test entry left. Review (sonnet) dispatched 23:05.
- Task 3: review (sonnet) — Needs fixes. Verified live: UTF-16LE blob round-trips through the engine reader; CredFree only on success; no secret in any string; four files, CRLF/LF preserved. 1 Important: test cleanup not panic-safe (a failing assert leaks knowlu/test/<id>). Minor: exists() collapses all read errors to false (brief-mandated; accepted). Fix round 1/5 dispatched 23:15 (Drop guard).
- Task 5: implementer DONE — 6be7492 (engine: quinn-ops ingest, 5 tests) + 3675205 (app: slot_argv coursework→ingest→rank, skip steps) on worktree-agent-a8e7c85b51d9088ab, BASE 4b0e2da. Root 800+2+3+3+4+1 passed, 0 warnings, oracles green; app 20/13(1 ign)/15; end-to-end skip test 0.09 s. Dual-run scripts NOT run there (no venv) — controller runs them at merge, after Task 2 commits (the merge touches app/src/scheduler.rs, which Task 2 is editing). Concern: two stale doc comments in scheduler.rs left for the merge. Review (opus) dispatched 23:25.
- Task 3: fix round 1 landed dcdaf29 (Drop guard `Cleanup(target)` at line 24, before the first write at 29; explicit delete + double-delete assertion kept; panicking-assert run and clean run both leave no knowlu/test entry per cmdkey).
- Ruling R-P4a-19: the Task 3 fix is verified by the controller from the diff and the file instead of a re-review dispatch — a 13-line test-only change whose one property (guard before write) is a line-number check — costs nothing; if wrong, the final review sees the file.
- Task 3: complete — aa1f436 + dcdaf29 on worktree-agent-a059a3facfe75eeaa (1 fix round); merge into the plan branch after Task 2 commits (both touch app/Cargo.toml and app/src/lib.rs).
- Task 2: implementer DONE 42f3110 (app 53 passed/1 ignored, 0 new warnings; root 808, 0 warnings; console-shots ok; +146 KiB). Forced deviation: tauri-plugin-dialog 2.7.3 forces rfd/common-controls-v6 → STATUS_ENTRYPOINT_NOT_FOUND on the GNU binary (the documented comctl32 trap); used rfd 0.16 directly with default-features=false — same IFileOpenDialog picker, no plugin, no dialog:default capability. Dev-habit change: no --vault from inside a vault dir → the wizard shell, not the cwd vault.
- Ruling R-P4a-20: the rfd-direct folder picker stands in for tauri-plugin-dialog — the plugin was a means, the constraint (native picker, no network capability) holds, and the plugin cannot load on this toolchain; Task 6 uses MessageBoxW for any message box (never rfd message dialogs); Task 11 amends spec §4 — costs nothing; if wrong, a future Tauri upgrade re-opens it.
- Task 10: NEEDS_CONTEXT — tests/site.rs asserted app/static/console.js contains the wizard PRIVACY constant (Task 6, not yet built). Ruling R-P4a-21: that cross-check moves to Task 6 (static_assets.rs reads ../site/privacy.html); Task 10 drops the assertion and commits. Message sent 23:50.
- Task 5: review (opus) — Approved. Fidelity table all ✓ (M counts the malformed line; course_map order via indexmap-backed Mapping); both authorised deviations only; reviewer re-ran engine tests, both oracles and the app scheduler suite green. Minors → final wave: assert the summary line in the happy-path test ("ingest: 5 events, 6 action(s)" from the fixture); a fetch-failed test via the seam; three stale scheduler.rs comments (LOGS_KEPT is ~10 days with a feed: six logs/day); skip step recorded before coursework in the step list; ics_state computed twice; unreadable-but-present config reads as no_url; Task 17 end-to-end never runs ingest (vault-full has no url). Task 5: complete — 6be7492 + 3675205 (0 fix rounds); merge + dual-run next.
- Merged into the plan branch: Task 3 (da84449; conflicts in app/Cargo.toml windows features and app/src/lib.rs mod lines resolved by union) and Task 5 (175804b, clean). Dual-run scripts and both suites running on 175804b in the background; Task 4 dispatch after they report green.
- Dual-run at 175804b (Task 5 merged): diff-engines vault-s1 / vault-s1-migrated / vault-full No differences exit 0; diff-engines-notes No differences exit 0 — rank and write did not move under the ingest addition. Task 10: implementer DONE 105280d (R-P4a-21 applied; site test 1/1; root 795, 0 warnings); review (sonnet) dispatched.
- Merged tree 175804b green: root 800+2+3+3+4+1 = 813 passed, 0 warnings, oracles green; app 20+2+5+13(1 ign)+16 = 56 passed, only the pre-existing .rsrc line. Task 4: dispatched 23:20 by the tool clock (opus, primary worktree), BASE 175804b.
- Task 10: review (sonnet) — Approved (privacy sentence byte-identical on both pages; no http/script/url(); CSS tokens are the console's; CRLF verified). Minor: test name/doc still mention the wizard cross-check (moved to Task 6 by R-P4a-21); tests/site.rs is LF. Task 10: complete — 105280d, merged into the plan branch (0 fix rounds).
- Task 2: review (opus) — Needs fixes. R-P4a-20 rfd deviation verified against both manifests and rfd cfg gates (accepted). 3 Important: (1) `.app` stays display:grid under [hidden] so the picker renders below an empty console (needs `.app[hidden]{display:none}`); (2) migrate_flat_layout `.ok()?` on a failed rename strands settings/seen/logs forever, silently (plan-mandated); (3) profiles.json save not atomic and load maps unreadable→empty so register overwrites everyone (plan-mandated). Minors: non-UTF-8 vault path falls to cwd; id is the raw path string; app_data_dir fallback gone but state.rs doc says otherwise; README lines stale; profile_legacy comment overclaims; picker row layout; single-instance second --vault silently focuses A (note for Task 7: Switch profile goes through --pick). Never run: the picker has zero runtime evidence.
- Ruling R-P4a-22: the picker button reads "Use an existing vault…" (the brief's string) — it adopts; the true "Add another → wizard" button arrives with Task 6 as "Create a new vault…" — costs nothing.
- Task 2 fix round 1 queued behind Task 4 (same worktree; concurrent edits would give Task 4 transient compile failures). Task 7 dispatched now on an isolated worktree from the plan branch at 057dfc4.
- Task 4: implementer DONE f2cb2a2 (app 61 passed/1 ignored, zero new warnings; scaffold 5/5; rank on a scaffolded vault: six stub-fetch warnings, no unmigrated warning; journal = one system:migration/cli create + one quinn/dashboard create). Deviation: unused Path import dropped from the brief's test (warning). Trap noted: `sed -i` in Git Bash strips CR from CRLF files and git diff hides it (index LF) — caught by od -c. Review (opus) dispatched. Task 2 fix round 1/5 dispatched (implementer resumed) 23:40 by the tool clock.
- Task 4: review (opus) — Needs fixes. Rulings honoured (seed via cli satisfies the real guard; preset byte-faithful, no secret; keys match the engine readers; CRLF via write_text). 2 Important (plan-mandated): (1) timezone/tz/device/slots/ics_url interpolated into YAML unescaped — an unparsable runners.yaml is SILENT (load_runners_config → Ok(empty) → scheduler: script, runner never fires); (2) atomicity: the refused-create cleanup test is vacuous (refusal returns before staging) and seed_writes runs after the rename, so a seed failure leaves an unadoptable half-vault — seed inside the staging folder before the rename (journal paths are vault-relative) + a real mid-materialisation failure test. Minors: fs errors without paths; stale .knowlu-new-* after a kill; preset has no timezone (Central-only trap); no test guards app/assets for secrets; scaffold not cfg(windows); the six-warning claim unpinned. Fix round 1/5 queued behind the Task 2 fix (same worktree).
- Task 7: implementer DONE 76d102f on worktree-agent-a02725118c9e50b1a (BASE 057dfc4; app 58 passed/1 ignored; headless settings-check.py ok; shot settings-1512.png). Deviations: separate settings-check.py until Task 6 lands the KNOWLU_SHOTS seam; panel after #drawer; two extra CSS lines so the 1180 px aside grid does not keep it visible; panel covers + task. Review (sonnet) dispatched.
- Task 2: fix round 1 landed fdb4088 (.app[hidden]; rename-failure fold from old + non-empty gate; atomic registry save + fallible load + Launch::Broken fatal; non-UTF-8 vault fatal; docs; picker layout). app 65 passed/1 ignored; profiles tests 5→9; headless picker check 13/13; picker-1512.png read: picker alone, two rows, Open flush right. Concerns: migration errors only eprintln (invisible in the windows-subsystem build) — plumbing needs a ConsoleState::open param → follow-up/final wave; picker-check.py lives in the SDD workspace; Launch::Broken is fatal (recovery = hand-rename profiles.json). Scoped re-review (sonnet) next. Task 4 fix round 1/5 dispatched (implementer resumed).
- Task 2: re-review (sonnet) — all 3 Important + 3 minors ADDRESSED; picker-1512.png confirms the picker alone at the top; rename-failure test deterministic (open handle). Nits: empty old `quinn-ops` dir left behind after a per-file fold; one error message wording; settings.json content not byte-compared (existence only). Task 2: complete — 42f3110 + fdb4088 (1 fix round). MERGE NOTE for Task 7: profiles::load is now Result<Vec<Profile>, String>; Task 7's branch (BASE 057dfc4) calls the old Vec signature — expect a compile break at merge; handle in its fix round or a controller fix-up.
- Task 7: review (sonnet) — Needs fixes. Rulings R-P4a-4/5/15/20, S1, S4 verified; screenshot shows six rows, placeholder disabled, panel covers + task. 1 Important (plan-mandated): switch_profile exits without quit_at stamp / settings save / quit_flush. Merge break: settings_context iterates profiles::load, which is now Result. Minors: CSS comment wrong about .drawer (which itself has only the [hidden] half — latent bug at 1180 px, note for the final review); switch duplicates relaunch_with; overlay covers + task. Fix round 1/5 dispatched (implementer resumed; merges the plan branch first).
- Task 4: fix round 1 landed 6001f3e (yaml_scalar on every field; create_vault seeds in staging before the one rename, seed_writes private; real failure tests; path-bearing fs errors; warning pin; asset secret scan; preset tz note). app 69 passed/1 ignored, zero new warnings. Task 6 brief updated by the implementer for the new contract. Scoped re-review (sonnet) dispatched; Task 6 (opus, primary) dispatched, BASE 6001f3e.
- Task 7: fix round 1 landed — b6a52d8 (merge of the plan branch, clean) + 365eb57 (switch_profile = the tray quit arm with a spawn in the middle; relaunch_args/relaunch shared; settings_context carries registry_error; set_profile_name refuses on a corrupt registry, bytes unchanged; CSS comment corrected). app 69 passed/1 ignored, zero new warnings; settings-check.py ok incl. the registry-error case. Concerns: two 10 s caps can race on a switch (documented); .drawer latent 1180 px issue untouched (final review). Scoped re-review (sonnet) dispatched; merge into the plan branch after Task 6 commits (page overlap).
- Task 4: re-review (sonnet) — both Important + minors ADDRESSED; device quoting confirmed necessary (runner_settings uses as_str). Gaps noted: the `none` campus not exercised through rank; credential_target not in the control-char loop; stale .knowlu-new-* sweep and cfg(windows) carried. Task 4: complete — f2cb2a2 + 6001f3e (1 fix round).
- Task 9: dispatched 23:46 (opus, isolated worktree from the plan branch at 6001f3e; Task 1 outcome A; sign via signCommand; release.ps1 with CredReadW helper; no signing, no upload, no installer run).
- Task 7: re-review (sonnet) — both Important + minor ADDRESSED; argv test verified; no new warnings. Task 7: complete — 76d102f + b6a52d8 (merge) + 365eb57 (1 fix round); merge into the plan branch after Task 6 commits. Carry to the final review: settings panel covers + task; .drawer exposed to the 1180 px aside grid rule when open.
- Task 9 agent stalled (stream watchdog) mid-read; resumed from its transcript.
- Task 6: implementer DONE 4edbb15 (app 73 passed/1 ignored; root 814; wizard-check.py ok; console-shots ok; nine shots). Declared: picker button "Create a new vault…" (R-P4a-22); brief defect fixed — storeCredentials sent WIZ.vault (empty in the create flow) → now dest(); three extra CSS rules incl .wiz-row[hidden]; wizard→relaunch never run in a window. Review (opus) dispatched with the credential-target id consistency as the named Critical risk. Task 7 merge in progress (conflicts in console.css and static_assets.rs).
- Task 7 merged into the plan branch as 3852d86 (conflicts: console.css and static_assets.rs — both end-of-file appends from Tasks 6/7, resolved keeping both; CRLF preserved 465/465 and 391/391). App suite on 3852d86 running. Task 8 (pre-key half; R-P4a-13) dispatched 00:10 on an isolated worktree (opus), BASE = its merge of the plan branch.
- Merged tree 3852d86 (Tasks 6+7): app 23+2+2+9+9+13(1 ign)+19 = 77 passed, only the .rsrc line.
- Task 6: review (opus) — Needs fixes. 1 CRITICAL: credential keyed to dest() at panel 5; a rename after (which the refused-Finish panel instructs) leaves the password under the old id while ingest.yaml names the new one with enabled:true. 3 Important: failed credential write advances anyway with flags set; Finish summary reads WIZ.slots/tz never refreshed from the DOM (stale); finish_profile `let _ = save` and adopt path ignore settings errors (plan-mandated). Minors: no rollback when register fails after create; dest_for and Windows names (reserved devices, trailing dots, *?"<>|); backup folder may be inside/equal the vault; wizard-check does not assert decision 3; half-filled pair silently skipped; find_mirror first-match; copy_tree duplication; restore skips panels 4–6.
- Ruling R-P4a-23: at Finish, if the credential vault differs from dest(), a Rust `retarget_credentials(from, to)` re-keys both sources in Credential Manager (secret never reaches the page) or Finish is refused back to panel 5; `dest_for` refuses Windows-invalid names (*?"<>|, reserved device names, trailing dot/space) — a credential must never key to a path Windows would rewrite — costs one command; if wrong, a re-entered password. Fix round 1/5 dispatched 00:20 (implementer resumed).
- Quinn 2026-09-06: Gmail OAuth client created; personal address is a test user; Crimson/UA address cannot be (Workspace) and need not be.
- Task 8 (pre-key half): implementer DONE 1cee6b5 on worktree-agent-ae12e2bcf2c66fa15 (BASE 3852d86; app 81 passed/1 ignored; settings-check.py ok with the not-configured row, a staged offer, a refused install; release build 5.7 MB). Declared: scheduler::lock now pub; "channel" wording removed; settings-check.py updated; interactive launch NOT run (autostart Run key + registry side effects; desktop-safety) — correct refusal. Gated step needs Quinn's public key. Review (opus) dispatched.
- Task 9: implementer DONE — f3e98ef + 1e1ff3a + 6511512 on worktree-agent-a3fbf291dfb9e2e92 (BASE 6001f3e). app 71 passed/1 ignored, zero new warnings; release.ps1 ran end to end unsigned: installer 4.47 MiB, sign.ps1 invoked 9× before packing (UNSIGNED:), BUNDLEID com.knowlu.desktop, sidecar quinn-ops.exe; nothing signed/uploaded/run. Deviations to rule at review dispatch: (1) createUpdaterArtifacts=false until Task 8 adds the plugin (tauri-cli hard-fails otherwise; test asserts flag⇔plugin coupling) — ACCEPT; (2) the bundler requires signtool.exe even with a custom signCommand (sidecar verify probe) — TAURI_SKIP_SIDECAR_SIGNATURE_CHECK set only when signtool is absent; Windows SDK becomes a prerequisite for the first signed release — ACCEPT provisionally, reviewer to examine; (3) two brief defects in release.ps1 fixed (Add-Type -UsingNamespace; native stderr under Stop). REVIEW NOT YET DISPATCHED (compaction stop) — next session: package `6001f3e..6511512`, review (opus), fix loop, merge.
- Task 8 (pre-key half): review (opus) — Approved. Gated step verifiably absent; mid-run gate one pure function on all three surfaces; install hold atomic check-and-set, poison-tolerant, released on every path; failed check quiet in memory; async commands the correct Tauri shape; staged update never paints Warn; refusal of the interactive launch judged correct. 3 Important, all to land WITH the key-gated commit (R-P4a-24, below): tray install path swallows refusal/failure (write into up.last_error); the daily check runs inline on the housekeeping thread with no timeout (spawn it or give the plugin a timeout); two launch checks race one bundle path (single-flight + temp-then-rename). Minors: "up to date" said while staged during a slot; updates_dir() duplicated; updates.rs open-codes the poison lock; InstallHold duplicates RunGuard; renderUpdateOffer hides without clearing; no .upd CSS; stale CSS comment; .refusal toast z-index over the settings panel (settings-check removes the node); memory-only Staged / no updates_dir pruning (gated step). Task 8 pre-key half: complete — 1cee6b5 on worktree-agent-ae12e2bcf2c66fa15 (0 fix rounds); merge next session.
- Ruling R-P4a-24: the three Important findings on Task 8 are folded into the key-gated commit's brief (they become real only when the plugin lands) — one dispatch instead of two — costs nothing now; if wrong, the gated step grows by three small items.
- Task 6: fix round 1 landed 143f4a5 (retarget_credentials + credVault tracking; dest_for Windows-name refusals (11 tests); failed write stays on panel 5 with fields kept; panel-6 values reach the summary; settings save errors surface; rollback removes the vault when register fails; check_backup_dir; wizard-check asserts decision 3, the rename→retarget order, 09:00 propagation, both failure paths). app 80 passed/1 ignored, no new warnings. Concerns: retarget not atomic across two sources (loud); check_backup_dir refuses the CONTAINS direction so Documents\Knowlu (the default parent) is rejected as a backup folder; retarget_credentials registered in run_shell only; onboarding.rs became CRLF via the Task 7 merge checkout. SCOPED RE-REVIEW NOT DISPATCHED (compaction stop) — next session: package `3852d86..143f4a5`, re-review (sonnet), then `Task 6: complete`.
- Ruling R-P4a-25 (apply in Task 6's re-review round or the final fix wave): `check_backup_dir` refuses only (a) backup == vault and (b) backup INSIDE the vault (the mirror would copy itself); a vault INSIDE the backup folder is allowed — the mirror writes under <backup>\<profile>\vault, never into the vault, so no recursion, and the default parent Documents\Knowlu must remain a valid backup folder — costs one condition; if wrong, a friend picks a different folder.
- 2026-09-06 00:55 (after compaction): resumed at 143f4a5, clean. Task 6 scoped re-review (sonnet) dispatched on review-3852d86..143f4a5.diff; Task 9 review (opus) dispatched on review-6001f3e..6511512.diff with the three deviations to adjudicate. Merge dry-runs: Task 8 branch (1cee6b5) merges into 143f4a5 CLEAN; Task 9 branch (6511512) conflicts only in app/tests/static_assets.rs (append). R-P4a-25 goes to the final fix wave (the re-reviewer confirms the current behaviour). Cutover: no 09-06 slot yet (00:54).
- Task 6: re-review (sonnet) — Critical 1 + Importants 2–4 + 5 minors ADDRESSED (retarget_credentials read→write-new→delete-old; dest_for refuses 22 device names, *?"<>|, trailing dot/space, mirrored in wizValid; readSlotsPanel wired; finish_profile_in writes settings before register; finish_or_roll_back). Left as ledgered: find_mirror first-match, copy_tree duplication, restore skips panels 4–6 (spec §3). R-P4a-25 confirmed NOT YET APPLIED: check_backup_dir onboarding.rs:309-310 and console.js:1200 refuse both directions → final fix wave. New minors → final review: partial credential-pair success leaves an orphan entry if the wizard is abandoned; retarget failure branches and credentialsStranded() untested (wizard-check fakes ok:true). Task 6: complete — 4edbb15 + 143f4a5 (1 fix round).
- Task 8 branch merged into the plan branch as 578412a (clean auto-merge: main.rs, console.js, static_assets.rs; message amended with both trailers). App suite running on 578412a in the background. Task 9 review still out.
- Merged tree 578412a (Task 8 in): app 23+2+5+9+9+13(1 ign)+20+3 = 84 passed, only the pre-existing .rsrc warning line.
- Task 9: review (opus) — Needs fixes. All three deviations CONCUR (tauri-cli rust.rs:855 hard-fails createUpdaterArtifacts without the plugin; the coupling test is sound only because of its !is_null guard, which is present; the sidecar verify probe bundle.rs:327 needs signtool and TAURI_SKIP_SIDECAR_SIGNATURE_CHECK also skips SIGNING the sidecar; the two PS 5.1 fixes correct). Spec: identifier, bundle, signCommand-before-pack (verified in tauri-bundler), sign.ps1 verbatim, key read/cleared in finally, no endpoint assertion — all ✓. 2 Important: (1) externalBin makes plain cargo build/test in app/ FAIL on any checkout without the staged sidecar (tauri-build lib.rs:545 copy_binaries → ResourcePathNotFound; app/binaries/ is git-ignored) — the README's dev flow, the plan's green gate and the local runner's app build all break; (2) Invoke-Native leaves a stale $LASTEXITCODE when the exe cannot launch (cargo off PATH) → a stale engine or a stale installer ships silently. Minors: comment overstates the skip var; hard-coded triple; untracked files pass the clean gate; sign.ps1 still uses $? under Stop; static_assets.rs no trailing newline; targets length unasserted.
- Ruling R-P4a-26: app/build.rs writes a ZERO-BYTE placeholder at binaries/quinn-ops-<TARGET>.exe when the file is missing, in every profile — plain cargo never bundles, so the placeholder can never ship through cargo alone; release.ps1 stages the real engine before cargo tauri build and REFUSES to build if the staged sidecar is under 1 MiB; a hand-run cargo tauri build is not a supported release path and the README says so — costs one build-script branch; if wrong, an unsupported manual bundle carries an empty sidecar and the console's first slot reports engine failure loudly. Fix round 1/5 dispatched (implementer resumed) with Importants 1–2 and minors 1, 2, 5, 6; minors 3–4 → final review.
- Task 9: fix round 1 landed 4b8eaf8 on worktree-agent-a3fbf291dfb9e2e92 (build.rs zero-byte placeholder + rerun-if-changed; release.ps1 1 MiB sidecar guard proved to fire, -StageOnly (skips the clean-tree gate by design), LASTEXITCODE reset + null-guarded at all four callers, cargo/git/rustc resolved up front, triple from rustc -vV; README; guard test cross-checks build.rs stem vs externalBin; m1/m5/m6). app 72 passed/1 ignored in BOTH states (placeholder-only and staged). Report copied. Scoped re-review (sonnet) dispatched on review-6511512..4b8eaf8.diff. Merge dry-run: still only app/tests/static_assets.rs conflicts (append).
- Task 9: re-review (sonnet) — Importants 1–2 and m1/m2/m5/m6 all ADDRESSED (placeholder before tauri_build::build via TARGET; six null-guarded LASTEXITCODE call sites; cargo/git resolved up front; triple from rustc -vV with a loud throw). Nits → final fix wave: app/build.rs lost its trailing newline; Invoke-Native's 'returns NOTHING' comment now contradicted by the rustc -vV capture. Task 9: complete — f3e98ef + 1e1ff3a + 6511512 + 4b8eaf8 on worktree-agent-a3fbf291dfb9e2e92 (1 fix round); merging into the plan branch now.
- Task 9 merged into the plan branch as c1d8f30 (conflict: app/tests/static_assets.rs, two EOF appends kept both, CRLF 508/508, no duplicate test names). Tasks 1–10 are now all on the plan branch. App suite (placeholder-sidecar path: this worktree has no app/binaries) and root suite running on c1d8f30 in the background; Task 11 dispatches after both are green (supplement written: task-11-supplement.md).
- Root suite on c1d8f30: 800(2 ign)+2+3+3+1+4+1 = 814 passed, 0 warnings, oracles + site green.
- c1d8f30 app suite FAILED to compile: static_assets.rs 'unclosed delimiter' — git placed the shared final } after the conflict hunk, so keep-both left the last plan-branch test unclosed. Fixed as d95bab7 (one line, CRLF 509/509; cargo test --test static_assets 23 passed). The build.rs placeholder path proved live here: a 0-byte app/binaries/quinn-ops-x86_64-pc-windows-gnu.exe appeared on the first build in this worktree (no staged sidecar). Lesson for keep_both.py: after resolving, check that every side's block ends in its own closing line — a shared suffix line is context, not part of either block. Full app suite running on d95bab7.
- Merged tree d95bab7 (Tasks 1–10 all in): app 23+2+5+9+9+13(1 ign)+23+3 = 87 passed, only the pre-existing .rsrc line; root 814 passed, 0 warnings (c1d8f30, unchanged by the fix-up). Task 11 (close) dispatched 01:45 by the tool clock (opus, primary worktree), BASE d95bab7, with task-11-supplement.md.
- Task 11: implementer DONE_WITH_CONCERNS 1837638 (8 docs files, +440/−69, all CRLF verified; root 814/0 warnings; app 87/1 ign). Measured: 32 distinct tauri commands (22 commands.rs + 10 onboarding.rs; console window registers 24, vault-less shell 10). Concerns: KNOWLU_SHOTS lacks openSettings so console-shots.py cannot shoot settings (settings-check.py used) → fix wave; the Updates row shipped LIVE with Check now (not disabled) — docs say what ships; R-P4a-25 absent from code (spec says 'code catches up in the fix wave'); Knowlu spec identifier item was in §7 stage 1; a one-off root flake runs::tests::start_step_end_records_full_text_and_fold_steps (QUINN_OPS_DEVICE race, pre-existing). Review (sonnet) on review-d95bab7..1837638.diff and the whole-branch FINAL REVIEW (opus) on e068846..1837638 dispatched together at 02:05 by the tool clock; both feed ONE fix wave.
- Task 11: review (sonnet) — Needs fixes. Every spot-checked claim TRUE against the code (32 commands 22+10, console 24 / shell 10; module list; slot order + skip texts; build.rs placeholder; release.ps1 -StageOnly/1 MiB/key clear; tauri.conf.json keys; target_for; seed context; picker strings; KNOWLU_SHOTS lacks openSettings; Updates row live; tray seven items; ACTIONS unchanged at 11; 814/87 reproduced; three screenshot sentences match the images). 1 CRITICAL: CLAUDE.md:211-212 says every command other than state/note/get_settings/settings_context touches 'never the vault' — false for ui_event, which writes <vault>/state/events-ui/ (src/uievents.rs:58); README scopes the same sentence to 'the vault's notes' and is correct. Minors: 'Releasing' is a retitle; rule-1 amendment reads mid-sentence.
- Ruling R-P4a-27: the Task 11 Critical (one CLAUDE.md sentence) is folded into the single final fix wave instead of its own fix round — the wave touches docs anyway and the final review is still out; Task 11 is marked complete when the wave's scoped re-review confirms the sentence — costs nothing; if wrong, one extra re-read of CLAUDE.md.
- FINAL REVIEW (opus, 9 passes, e068846..1837638) — Ready to merge WITH FIXES. Every ruling HONOURED with evidence except R-P4a-25 (known). 1 CRITICAL C1: migrate_flat_layout (profiles.rs:129-136) renames the WHOLE %LOCALAPPDATA%\quinn-ops → knowlu when knowlu is absent; on this laptop quinn-ops\ also holds dual\ (live cutover snapshots), logs\ (107 files from the LIVE local-run.ps1 and dual-run.ps1), rehearsal\, scratch\ (5 scratch vaults), shots\ — the first launch of a merged build would move them mid-cutover-week, the scripts would recreate the old root and split the evidence trail, and a --vault <scratch> launch would register a dead profile and fatal exit 2 forever; no test plants a sibling. Importants: I1 an unopenable registered profile is a launch dead end (fall back to the picker; tighten adopt_vault_in to planning.yaml); I2 no test runs the read model over a wizard-born vault; I3 commands.rs updates_dir() doc says per-profile (it is per install) + a duplicate in scheduler.rs; I4 profiles are append-only, every --vault launch registers one (README now, Forget control → 4b); I5 the spec body still says via onboarding. Minors: privacy.html 'last sixty runs' false; no .upd CSS + stale CSS comment; build.rs newline; tests/site.rs doc; §0.0 panel number; set-updates row unasserted; ics_state locked-file case; scheduler LOGS_KEPT comments; America/Chicago fallback in launch_state (consistent with events config; noted); go-live checklist still names quinn-ops\settings.json.
- Ruling R-P4a-28: the flat-layout migration NEVER renames the old root — always the per-file fold (settings.json, seen.txt, and only the app's own slot-*/quit-* logs), everything else under quinn-ops\ untouched, the old root removed only when the fold empties it; a test plants dual/, logs/dual-*, scratch/v/ siblings and asserts they stay — the live harness's evidence must survive the first launch — costs one deletion and one test; if wrong, an old install keeps an empty folder.
- Fix-wave triage per the final review: FIX NOW = C1, I1 (both halves), I2, I3, I5, minors (.upd CSS + stale comment, set-updates row, scheduler LOGS_KEPT comments, site.rs redoc, privacy.html sentence, §0.0 panel number + KNOWLU_SHOTS shape, go-live checklist line, build.rs newline, Invoke-Native comment) + the already-queued R-P4a-25, KNOWLU_SHOTS.openSettings, CLAUDE.md ui_event sentence. FOLLOW-UP 4b = migration errors surfacing, picker-check.py, .knowlu-new-* sweep, ingest fetch-failed + summary tests, ics_state NotFound, orphan credential + credentialsStranded test, Forget-this-profile. Key-gated step = staged-update observables + R-P4a-24. Plan 3 = QUINN_OPS_DEVICE test race. ACCEPT = the rest as the review reasoned. Brief: final-fix-wave-brief.md. ONE fix-wave dispatch (opus, primary worktree) BASE 1837638 at 02:40 by the tool clock.
- Final fix wave DONE: ec1c7a1 (A4.1 migration fold, alone) + b9a64a2 (A1 R-P4a-25, A2 openSettings shot, A3, A4.2 picker fallback + adopt predicate, A4.3 read-model test, A4.4, A4.6) + 7432b7c (B1–B6, A4.5). app 90 passed/1 ign; root 814/0 warnings; wizard-check + settings-check ok; console-shots settings view added; real %LOCALAPPDATA%\quinn-ops unchanged (dual 2 / logs 107 / scratch 5), no knowlu root created. Deviations declared: two (not three) LOGS_KEPT comments; fatal kept for zero-profile --vault; A1 rollback test rebuilt; B5 JSON template dropped; emptied logs\ removed before the empty-root check; privacy.html in the docs commit. Scoped re-review (opus — C1 is the live-machine change) dispatched on review-1837638..7432b7c.diff at 03:20 by the tool clock.
- Fix wave re-review (opus) — ALL findings ADDRESSED (A4.1: only two renames remain, both files; no path outside settings.json/seen.txt/slot-*/quit-* is moved, removed or created; has_flat_layout short-circuits when nothing of ours is present; a locked slot log costs the fold one error line and the file stays; remove_dir only, never remove_dir_all, behind is_empty_dir; the sibling test plants dual/, logs/dual-*, slot-*, quit-*, scratch/v/ and asserts original bytes; temp roots only; migration precedes register; real quinn-ops\ unchanged 107 logs, no knowlu root). All six deviations CONCUR. Four minors: main.rs:48-49 comment still described the rename (FIXED 76ad1f2); an already-empty foreign quinn-ops\logs\ is removed outside the fold block (4b); superseded-branch remove_dir_all is unreachable in practice (accept); the failed-single-move path is no longer pinned by a test (4b). Out of scope: plan 2 line 1022 named the renamed test (FIXED 76ad1f2); README slot-*.log → .txt (FIXED 76ad1f2); the A4.2 fallback's eprintln is invisible in the windows build (4b, already ledgered). Live-machine note: the first launch of a merged build WILL create %LOCALAPPDATA%\knowlu and fold exactly the two slot-* logs out of quinn-ops\logs\; everything else stays — the ruling working as intended.
- Ruling R-P4a-29: the three one-line residuals are fixed by the controller in 76ad1f2 (comment/doc lines; R-P4a-19 precedent) rather than a second wave; the two behavioural minors go to plan 4b — costs nothing; if wrong, 4b's first task reads two ledger lines.
- Task 11: complete — 1837638 + the B-section of 7432b7c (CLAUDE.md ui_event sentence confirmed true by the re-review) (1 fix round, folded into the wave per R-P4a-27).
- Plan 4a: ALL TASKS COMPLETE at 76ad1f2. Final head gates: root 814 passed/0 warnings (7432b7c; nothing under src/ changed since), app suite re-run on 76ad1f2 (comment-only change) below. Next: preserve this ledger to docs/superpowers/reports/2026-09-06-knowlu-plan-4a-sdd-ledger.md (commit on the plan branch), then merge to main from the MAIN checkout (left-right 0 0 verified at 03:25, main clean at e068846), keep this workspace until the key-gated updater step lands, then ask Quinn for the updater public key (one ask, exact command).
- Final head 76ad1f2 app suite: 23+2+5+11+10+13(1 ign)+23+3 = 90 passed, only the pre-existing .rsrc line. Ledger preserved as a82fee6 (678 lines CRLF, four appendices). MERGED TO MAIN from the MAIN checkout as 60ccaf4 (pre-check immediately before: main clean, left-right 0 0; 56 files, +6055/−157 vs e068846). Obsidian Git / the 12:00 local runner will push. This worktree fast-forwarded to main. Plan 4a CLOSED 2026-09-06 ~03:50 except the key-gated updater step — the updater public key is now the item to ask Quinn for.
- Quinn 2026-09-06 ~04:00: updater PUBLIC key received (minisign key id C2EC981122E1D2DF; saved to updater-pubkey.txt in this workspace — public, safe in the repo). Whether the private half is already in Credential Manager knowlu/updater-key is unconfirmed — release.ps1 refuses at build time if it is missing, so it is not a blocker for the commit. Task 8 key-gated step dispatched (opus, primary worktree, BASE 60ccaf4 = main) with the brief's step 9 + R-P4a-24's three items + the two cheap staged-update observables; Staged persistence and updates_dir pruning stay 4b.
- Task 8 key-gated step: implementer DONE_WITH_CONCERNS 1737785 (14 files, +735/−59; app 95 passed/1 ign, root 814, zero new warnings; settings-check ok; dev build ok). Declared: timeout lives on UpdaterBuilder (app.updater_builder().timeout(...)), not the plugin Builder; Update::download verifies minisign before returning bytes, so only verified bytes are staged; the signature is NOT re-verified across the disk hop (verify_signature private in 2.11.0) — mitigated by temp-then-rename, %LOCALAPPDATA%-only storage, version match against a re-read manifest; updater:default grants the page allow-install/allow-download-and-install (the page never calls plugin:updater|* — grep 0); tray item enablement still refreshed only at check time (pre-existing); release.ps1 now demands the key for every release (README sentence).
- Ruling R-P4a-30: the console capability does NOT grant `updater:default` — the page never invokes `plugin:updater|*` (verified: 0 matches in console.js), the app's own `check_for_updates`/`install_update` commands are Rust-side and carry the mid-run gate, and the grant would let page code bypass that gate; capabilities gate IPC only, so `app.updater_builder()` works without it (reviewer to confirm against the plugin) — costs one line; if wrong, the review says so and the line comes back. Review (opus) dispatched on review-60ccaf4..1737785.diff.
- Task 8 key-gated step: review (opus) — Needs fixes. All five edits + config test + single-flight (compare_exchange latch inside check_and_stage, held across check AND download, atomic so unpoisonable) + temp-then-rename (.part<pid>) + spawned/timeboxed housekeeping (30 s check, 20 min download; check() resets Update.timeout to None so setting it is required) + both observables verified; version comparison in install_staged enforces one-bundle-one-version; zero new warnings confirmed by a build. R-P4a-30 CONFIRMED from the plugin source (lib.rs:79-125 updater_builder is plain Manager::state; only invoke_handler is capability-gated; default.toml grants allow-install + allow-download-and-install → the grant re-opens the mid-run gate). Disk hop: staged bytes are already minisign-verified by Update::download; install() does not re-verify; acceptable while NSIS installMode is CurrentUser (the unpinned tauri-utils default). 3 Important: (1) remove updater:default + fix the description + pin it with a static test; (2) tray note_error is unobservable — record_check overwrites last_error before any reader; needs its own field not cleared by a check, rendered in the row; the test encodes the gap; (3) check_and_stage re-downloads the whole bundle every check even when that version is already staged. Minors: no re-offer when a held slot ends (4b); a single-flight refusal stamps last_check; installMode default unpinned; temp_dir fallback can be world-writable; console.css:340 stale comment. Fix round 1/5 dispatched (implementer resumed): Importants 1–3 + minors 2–5; minor 1 → 4b.
- Task 8 key-gated step: fix round 1 landed ee286bc (updater:default removed + the_page_is_granted_no_updater_permission asserts the whole set; Updates.last_action_error untouched by record_check, shown as "· last action: …", cleared by the next explicit action (chosen over success-only because Update::install exits the process); already_staged(version match AND file present) skips the download, four cases tested; refusals no longer stamp last_check; bundle.windows.nsis.installMode currentUser pinned + asserted + named in the disk-hop comment; updates_dir() -> Result, refuses without LOCALAPPDATA; CSS comment). app 97 passed/1 ign, root 814, zero new warnings, settings-check ok (six states). Concerns: a persistently failing install leaves a sticky action line until the next action (by design); already_staged trusts the file's presence, not its bytes (same residual as the disk hop); no-LOCALAPPDATA machines get a permanently failed check. Scoped re-review (sonnet) dispatched on review-1737785..ee286bc.diff.
- Task 8 key-gated step: re-review (sonnet) — I1/I2/I3/m2–m5 all ADDRESSED (permission array asserted whole; last_action_error never touched by record_check, tray/command/tick routing verified; already_staged needs version AND file; refusals stamp nothing; installMode pinned + asserted; updates_dir Result with both callers handling Err; CSS comment). Three deviations CONCUR. cargo build --lib clean. Task 8: complete — 1cee6b5 (pre-key) + 1737785 + ee286bc (1 fix round on the gated step). MERGED TO MAIN from the MAIN checkout as 9940e6e (pre-check: main clean at 60ccaf4, left-right 0 0; 15 files, +963/−74). Plan 4a is now COMPLETE with nothing owed. Remaining Quinn items (ask when due): confirm knowlu/updater-key is in Credential Manager before the first release build; Cloudflare account for the site; Trusted Signing + signtool for the first signed release; the first friend's name and machine. 4b follow-ups are in the RESUME HERE block. This worktree fast-forwarded to main; the ledger re-preserved into the report next.


---

## Appendix A: the final whole-branch review (preserved)

# Final whole-branch review — Knowlu plan 4a (opus, 2026-09-06, range e068846..1837638)

### Passes made
1. Topology and merges — first-parent order, the six merges, the fix-up d95bab7; duplicate-definition scans across app/src/*.rs, console.js, console.css, index.html (only the pre-existing `.row.iss`/`.track` media-query pairs and the `cfg(windows)`/`cfg(not(windows))` `wait_for_pid_gone` pair).
2. Scope and global constraints — tests/fixtures/ diff empty; src/ limited to ingest.rs + main.rs and untouched after 175804b; app/Cargo.lock gains only rfd 0.16; page rules; uievents::ACTIONS unchanged (11); every WriteContext::new and every invoke( inventoried against both generate_handler! lists; per-file line endings.
3. Engine — src/ingest.rs::run_lines read line-by-line against engine/ingest.py:main (388–431), including the len(log) ordering; journal::VIAS.
4. App runtime — main.rs launch paths traced end to end into profiles.rs, onboarding.rs, scaffold.rs, credentials.rs, updates.rs, scheduler.rs, tray.rs, commands.rs, state.rs.
5. Page — index.html / console.js / console.css against the Rust they call (argument names, camel-case trap, path-spelling agreement between dest() and dest_for).
6. Release and security — release.ps1 on every exit path, sign.ps1, build.rs, tauri.conf.json, capabilities/default.json, .gitignore, plus a secret/name grep over the whole diff.
7. Tests — every new/changed test file read; coverage gaps identified.
8. Docs — CLAUDE.md, HANDOFF, app/README.md, docs/surface/anatomy.md, the two spec amendments and the plan-2 amendment, checked only against code read.
9. Live-machine read-only — %LOCALAPPDATA% layout and the scripts/*.ps1 that write into it, to size the migration's blast radius.

### Strengths
- scaffold::create_vault (app/src/scaffold.rs:117–127) builds AND seeds inside the staging folder, so the destination is either absent or a complete, already-migrated vault; the_move_does_not_leak_the_staging_path_into_the_journal proves it (two vaults at different depths → byte-identical journal records once id/ts/seq are stripped).
- yaml_scalar (scaffold.rs:57–62) refuses control characters by field name and names the real stake: runs::load_runners_config degrades an unparsable file to Ok(vec![]) and runner_settings to scheduler: Script, so a bad timezone would switch a friend's only runner off in silence.
- profiles::load (profiles.rs:35–43) separates absent from unreadable; Launch::Broken refuses to launch rather than rewrite; save goes through a sibling temp file; two tests assert the bytes are unchanged.
- retarget_credentials (onboarding.rs:380–408) writes the new target before deleting the old, returns `moved` on the failure path, never lets the secret cross back to the page; its test derives the targets with the same target_for(id_for(path)) the command uses.
- The page and the backend agree on path spelling by construction (dest() vs dest_for).
- the_bundle_config_is_the_one_the_installer_and_the_updater_need pins createUpdaterArtifacts ⇔ plugins.updater as a relationship.
- Invoke-Native (release.ps1:32–43) clears $LASTEXITCODE first, with the reasoning spelled out.
- build.rs's silent placeholder + the 1 MiB guard + the three-spelling cross-check.
- ics_state distinguishes the two "no" cases and records the skip as a step with code 0 before the engine is resolved.
- the_wizards_privacy_sentence_is_the_sites_privacy_sentence compares two files to each other rather than each to a literal.

### Issues

#### Critical (Must Fix)
C1 — migrate_flat_layout renames the whole %LOCALAPPDATA%\quinn-ops directory, and on this machine that directory is the live cutover harness's output. app/src/profiles.rs:129–136. The spec (§2), CLAUDE.md and app/README.md describe moving "the flat files" (settings.json, seen.txt, logs\); the code moves the entire directory. Read-only check of the live machine: %LOCALAPPDATA%\quinn-ops\ → dual\ logs\ rehearsal\ scratch\ shots\; %LOCALAPPDATA%\knowlu does not exist; dual\ = two cutover-week dual-run snapshots; logs\ = 107 files (dual-*, rehearsal-*, build-*, local-run); scratch\ = 5 vaults. Writers: scripts/local-run.ps1:57,115 (the LIVE Task Scheduler job), scripts/dual-run.ps1:71,72 (the LIVE dual-run harness, mid-week), scripts/scratch-vault.ps1:7,17 (prints the --vault launch line). On the first launch of a merged build the rename fires and takes everything; the scripts recreate the old root and the evidence trail G2 is judged on is split at a fixed point in the week; a --vault <scratch> launch then registers a profile at a path the rename invalidated and run_console fatals with exit 2 forever. Nothing is deleted, but it is a live-system consequence during the cutover week and no test can see it. Fix: delete the rename half; always the per-file fold; for logs\ move only the app's own files (slot-*, quit-*); test with planted siblings; is_empty_dir repurposed to remove quinn-ops only when the fold empties it.

#### Important (Should Fix)
I1 — A registered profile that cannot be opened is a launch dead end (main.rs:103–113, onboarding.rs:64–74): run_console fatals when resolve_vault fails; with one registered profile every launch hits the same dialog; adopt_vault_in's predicate (config/ dir and tasks/ dir) is weaker than resolve_vault's (config/planning.yaml file). Fix: fall through to run_shell(root, "picker", ps) when profiles exist; tighten adopt_vault_in.
I2 — Nothing exercises the read model over a wizard-born vault (no test calls state_inner/build_state_value against a scaffolded vault — the first screen a friend sees). Fix: create_vault → ConsoleState::open → state_inner ok for each view.
I3 — commands.rs's private updates_dir() doc comment says "this profile's updates/"; profiles::updates_dir's doc (the truth) says one folder for the whole install. Fix the comment; collapse the duplicate in scheduler.rs's housekeeping tick.
I4 — Profiles can be added but never removed, and every --vault launch adds one (main.rs:113, profiles.rs:189–201); scratch-vault.ps1 mints a new vault per demo. Fix (follow-up-sized): a Forget-this-profile affordance, or drop unresolvable entries when the picker renders; a README line meanwhile.
I5 — The spec body still says via: "onboarding" (friends-shell design line 232 and the §10 decision-5 row, line 414) although D5 said Task 11 amends the wording. Strike both.

#### Minor (Nice to Have)
- site/privacy.html:16 "the logs of the last sixty runs" is false (LOGS_KEPT = 60 counts files; a slot with a feed writes three; quit-*.txt is never pruned). Say "the last sixty run logs".
- app/static/console.css — no .upd rule; the update banner renders unstyled.
- console.css:462–464 — the "Updates row is a placeholder … disabled button" comment is stale; the button is live.
- app/build.rs:80 — no trailing newline.
- tests/site.rs:1–3, 9 — doc and test name say the privacy paragraph is checked against the wizard's; the real cross-check is app/tests/static_assets.rs (R-P4a-21). The file IS CRLF (the deferred "LF" finding no longer holds).
- Spec §0.0 — the R-P4a-25 bullet says "§3 panel 4"; the backup folder is panel 3.
- scheduler::ics_state — a present-but-unreadable file takes the NoUrl branch; only a parse failure reaches Unreadable. Match NotFound specifically.
- run_slot_inner — the ingest skip step is pushed before coursework; ics_state computed twice per slot.
- static_assets.rs::the_settings_panel_has_its_rows_and_one_way_in — the row loop omits set-updates.
- onboarding::launch_state — .unwrap_or("America/Chicago") as the fallback zone (consistent with events::load_events_config's default; noted).
- profiles.rs and scaffold.rs are bare LF while onboarding.rs, credentials.rs, updates.rs are CRLF (harmless, documented).
- scaffold::seed_writes writes created_by: quinn / effort_source: quinn into a friend's first task — engine vocabulary matching console_ctx(); a known consequence.

### Rulings and constraints — verification table
R-P4a-1 HONOURED (main.rs:29–31; profiles.rs:213–236; onboarding.rs:107–131; single-instance in both builders) · R-P4a-2 HONOURED · R-P4a-3 HONOURED · R-P4a-4 HONOURED · R-P4a-5 HONOURED (six .set-row ids; no "channel") · R-P4a-6 HONOURED in code (scaffold.rs:179), spec body pending (I5) · R-P4a-7/9/17 HONOURED (run_lines vs engine/ingest.py:388–431; slot_argv/ics_state; the skip test) · R-P4a-8 HONOURED · R-P4a-10 HONOURED · R-P4a-11 HONOURED · R-P4a-12 HONOURED · R-P4a-13 HONOURED (no plugin in Cargo.toml/lock; no plugins key) · R-P4a-14/15 HONOURED (release.ps1:78–99, 184–191, 214–233) · R-P4a-15 HONOURED · R-P4a-16 HONOURED · R-P4a-19 HONOURED · R-P4a-20 HONOURED (rfd only in the lockfile; MessageBoxW; no dialog permission) · R-P4a-21 HONOURED · R-P4a-22 HONOURED · R-P4a-23 HONOURED (dest_for:173–195) · R-P4a-24 HONOURED (correctly absent) · R-P4a-25 VIOLATED (known; fix wave) · R-P4a-26 HONOURED · one src/ change + 0 warnings HONOURED, dual-runs NOT RERUN (src/ unchanged since 175804b) · fixtures untouched HONOURED · page rules HONOURED (0/0/0 network; 6 == 6; one mark_seen; ACTIONS 11) · every write via console_ctx + the one seed record HONOURED (three WriteContext constructors in app/: console_ctx, the pre-existing executor_ctx, scaffold.rs:179) · scheduler inert / scheduler: app + device from birth HONOURED · credentials HONOURED (retarget test targets are knowlu/profile_<hash of temp>/… — runtime-minted, Drop-deleted) · no network in tests HONOURED · no single-user assumption HONOURED · no telemetry HONOURED · no updater plugin HONOURED · no signing secret HONOURED · line endings HONOURED · never-rename list HONOURED · desktop safety HONOURED.

### Deferred-findings triage
FIX NOW: empty-old-root sweep (with C1); scheduler LOGS_KEPT comments; updates_dir duplicate + I3 comment; .upd CSS + stale CSS comment; build.rs newline; Invoke-Native comment; tests/site.rs redoc; KNOWLU_SHOTS.openSettings (+ its four dependents: console.js seam, static_assets literal, console-shots panel list, spec §0.0 last bullet).
FOLLOW-UP plan 4b: migration errors surfacing (ConsoleState::open param); picker-check.py; stale .knowlu-new-* sweep; ingest happy-path summary assertion; ingest fetch-failed test (worth pulling forward); ics_state NotFound; orphan credential on abandoned wizard; retarget failure branches / credentialsStranded() test; Forget-this-profile.
FOLLOW-UP key-gated step: "up to date" while staged; memory-only Staged; no updates_dir pruning; renderUpdateOffer hides without clearing; the three R-P4a-24 items.
FOLLOW-UP plan 3: the QUINN_OPS_DEVICE test race.
ACCEPT: Launch::Broken fatal; settings.json not byte-compared; none campus; credential_target loop; scaffold cfg(windows); preset tz note; skip-step order; ics_state twice; end-to-end never runs ingest; find_mirror; copy_tree; restore skips 4–6; retarget non-atomic (loud); retarget in run_shell only; settings panel covers + task; .drawer 1180 px; two 10 s caps; poison lock / InstallHold duplication; .refusal z-index; untracked files + -StageOnly gate; sign.ps1 $?; placeholder silence (1 MiB guard); Updates row live; ledger report path created at preservation; tests/site.rs LF (no longer true).

### Recommendations
1. Land C1 first; do not merge without it. 2. Keep the wave small: C1, R-P4a-25, KNOWLU_SHOTS seam, I3 + Invoke-Native comment, the byte-level cosmetics; I1's adopt predicate in the same wave, I1's picker fallback worth it. 3. Add I2's console-over-a-fresh-vault test. 4. Before the first friend install re-read app/README.md's scratch section and docs/runners/knowlu-go-live.md:13 (still says hand-edit %LOCALAPPDATA%\quinn-ops\settings.json). 5. Fix the privacy page's "sixty runs" sentence before publishing.

### Assessment
Ready to merge? With fixes. The branch is unusually disciplined — the scaffold's atomicity, the registry's absent-vs-unreadable distinction, the credential retarget and the flag ⇔ plugin coupling test are better than the spec asked for, and every ruling except the known R-P4a-25 is honoured with evidence. But migrate_flat_layout renames the whole %LOCALAPPDATA%\quinn-ops directory, which on this machine holds the live cutover harness's snapshots, the local runner's logs and five scratch vaults, and no test can see it; that plus the resulting unopenable-profile dead end must land before this reaches main.


---

## Appendix B: the final fix wave brief (what the wave changed)

# Knowlu plan 4a — the single final fix wave (brief)

Branch `worktree-knowlu-plan-4a`, base for this wave: 1837638. One implementer, one commit per
area is fine (or one commit total); the controller runs a scoped re-review afterwards. Nothing
here changes behaviour outside the items listed. Docs-only items are in section B.

## A. Code

### A1. R-P4a-25 — backup-folder rule (Task 6 leftover)
Ruling: `check_backup_dir` refuses only (a) backup == vault and (b) backup INSIDE the vault (the
mirror would copy itself). A vault INSIDE the backup folder is ALLOWED — the mirror writes under
`<backup>\<profile>\vault`, never into the vault, and the default parent
`%USERPROFILE%\Documents\Knowlu` must be a valid backup folder for a vault at
`Documents\Knowlu\<name>`.
- `app/src/onboarding.rs` `check_backup_dir` (~304–313): drop the `v.starts_with(b\)` disjunct;
  keep equality and `b.starts_with(v\)`. Comparison stays case-insensitive and
  trailing-separator-tolerant as it is now.
- `app/static/console.js` (~1200): the wizard's client-side check mirrors it — refuse
  `within(WIZ.bdir, dest())` (backup inside vault) and equality only; delete the
  `within(dest(), WIZ.bdir)` half. Update the refusal text if it names the dropped case.
- Tests: in `app/tests/onboarding.rs` add three cases through `check_backup_dir` (or the command
  that calls it): equal → refused; backup inside vault → refused; vault inside backup → ALLOWED.
  Add the third case to `scripts/wizard-check.py` too (a backup folder that is the vault's parent
  must pass Next on panel 3).

### A2. `KNOWLU_SHOTS.openSettings` (Task 7/11 gap)
`window.KNOWLU_SHOTS` in `app/static/console.js` (~1342) exports only `startWizard` and
`renderPicker`; the plan said Task 7 adds `openSettings` and `scripts/console-shots.py` shoots the
settings panel. Add `openSettings` to the export (the existing `openSettings` function; no new
behaviour), and give `scripts/console-shots.py` a `settings` view that calls it and shoots
`<w>-settings.png`, matching how it shoots the wizard and the picker. Pin the export in
`app/tests/static_assets.rs` (the existing KNOWLU_SHOTS assertion gains `openSettings`). Run
`console-shots.py` once on `tests/fixtures/surface-today-full.json` into the SDD workspace's
`shots-final-fix-wave/` and LOOK at `1280-settings.png` (Read tool) — report one sentence on what
it shows. `docs/surface/anatomy.md` §8 / the friends-shell spec §8 line that says
`settings-check.py` is the only settings shot: amend to say console-shots.py shoots it too.

### A3. Nits from the Task 9 re-review
- `app/build.rs`: restore the trailing newline at EOF (the file's own line-ending style; check
  with `od -c | tail -2`).
- `scripts/release.ps1` `Invoke-Native` doc comment: it says the function "returns NOTHING"; the
  `rustc -vV` call site captures its output. Reword: it returns the native command's stdout
  (PowerShell's implicit output) and never a status; callers read `$LASTEXITCODE`.

### A4. Items the final review marks FIX NOW

**A4.1 — CRITICAL C1 (ruling R-P4a-28): `migrate_flat_layout` must never rename the whole
`%LOCALAPPDATA%\quinn-ops` directory.** `app/src/profiles.rs:129–136` renames `quinn-ops` →
`knowlu` wholesale when `knowlu` does not exist. On this laptop `quinn-ops\` also holds `dual\`
(the live cutover harness's snapshots), `logs\` (107 files written by the LIVE Task Scheduler job
`scripts/local-run.ps1` and by `scripts/dual-run.ps1`), `rehearsal\`, `scratch\` (five scratch
vaults, the ones `scripts/scratch-vault.ps1` prints `--vault` lines for) and `shots\`. The first
launch of a merged build would move all of it mid-cutover-week, the scripts would recreate the old
root and split the evidence trail, and a `--vault <scratch path>` launch would then register a
profile at a path the rename just invalidated and fatal with exit 2 forever.
Fix, exactly: delete the rename half; ALWAYS take the per-file fold (it already reads from both
roots and is already the fallback path): move `settings.json` and `seen.txt` into
`knowlu\profiles\<legacy id>\`; for `logs\`, move ONLY the app's own files — `slot-*` and
`quit-*` — file by file, never the directory; touch nothing else under `quinn-ops\`. Remove the
old `quinn-ops` directory only if the fold has left it completely empty (repurpose `is_empty_dir`
for that, or drop it). The docs' description ("the flat files move") becomes true as written.
Tests in `app/tests/profiles.rs`: plant, alongside the flat files, `quinn-ops/dual/keep.txt`,
`quinn-ops/logs/dual-20260903.txt`, `quinn-ops/logs/slot-20260903-1200.txt` and
`quinn-ops/scratch/v/config/planning.yaml`; after `migrate_flat_layout`, assert `dual/keep.txt`,
`logs/dual-20260903.txt` and `scratch/v/config/planning.yaml` are still at their ORIGINAL paths
byte for byte, `logs/slot-…` moved to the profile's `logs\`, `settings.json`/`seen.txt` moved,
and the old root still exists (not empty). Keep the existing move/fold tests green; adjust any
that asserted the old root disappears. Update the doc comments in `profiles.rs`, `state.rs` and
the `app/README.md` data-root paragraph and CLAUDE.md sentence if either says "moved whole".

**A4.2 — I1: an unopenable registered profile must fall back to the picker, not fatal.**
`app/src/main.rs:103–113` `run_console`: when `resolve_vault` fails AND `profiles::load` returns a
non-empty list, call `run_shell(root, "picker", ps)` instead of `fatal` (keep the fatal for the
`--vault` case with zero profiles, where the wizard is the right shell — choose the shell the
existing `resolve_launch` would choose). And tighten `onboarding::adopt_vault_in` to the predicate
the console enforces: `vault.join("config").join("planning.yaml").is_file() &&
vault.join("tasks").is_dir()`. Test: register a profile whose vault folder has been deleted, run
the launch resolution, assert the picker shell (or whatever pure function decides it) is chosen;
and an adopt of a folder with `config/` and `tasks/` but no `planning.yaml` is refused.

**A4.3 — I2: the read model over a wizard-born vault.** In `app/tests/scaffold.rs`:
`create_vault` → `ConsoleState::open` → `state_inner(&cs, view)` for every view name the console
serves (get the list from `surface::View` or the existing tests) and assert `["ok"] == true` on
each — no `state/runs/`, no `today.md`, no `runner-log.md`, no git repo exists yet.

**A4.4 — I3 + duplicate:** `app/src/commands.rs` private `updates_dir()` doc comment says "this
profile's `updates/`"; the folder is one per install (`profiles::updates_dir`'s doc is the true
one). Correct the comment, and make `scheduler.rs`'s inline copy in the housekeeping tick call
the same one function.

**A4.5 — I4 (README only now):** one sentence in `app/README.md`'s scratch section: every
`--vault` launch registers a profile and nothing removes one yet; with two or more the plain
launch shows the picker; a *Forget this profile* control is a plan-4b item.

**A4.6 — minors marked FIX NOW:** (a) `app/static/console.css`: add a `.upd` rule for the update
banner (`<div class="upd" id="upd">`) in the console's tokens — one line of layout, no colour
invention beyond existing tokens — and fix the stale comment at ~462–464 that says the Updates
row is a disabled placeholder; (b) `app/tests/static_assets.rs::the_settings_panel_has_its_rows_and_one_way_in`:
add `set-updates` to the row loop so D9's "five plus Updates" is asserted; (c) the three stale
`scheduler.rs` comments about `LOGS_KEPT` (with a feed a slot writes three logs, so 60 files is
~20 runs / ~10 days); (d) `tests/site.rs`: rename/redoc — it compares the two site pages to its
own literal; the wizard↔site cross-check is `app/tests/static_assets.rs` (R-P4a-21).

## B. Docs (continued)

### B3. `site/privacy.html:16` — "the logs of the last sixty runs" is false (`LOGS_KEPT = 60`
counts files; a slot with a feed writes three; `quit-*` is never pruned). Say "the last sixty run
logs". Keep the privacy SENTENCE byte-identical to the wizard's (a test pins it) — this is a
different sentence.
### B4. Friends-shell spec body: `docs/superpowers/specs/2026-09-05-knowlu-friends-shell-design.md`
~line 232 and the §10 decision-5 row (~414) still say `via: "onboarding"`; strike both to
`("system:migration", "cli")` (R-P4a-6, D5 said Task 11 amends the body). Also §0.0's R-P4a-25
bullet says "§3 panel 4" — the backup folder is panel 3 (panel 4 is the LMS calendar). And §0.0's
last bullet states the two-export `KNOWLU_SHOTS` shape — update for A2.
### B5. `docs/runners/knowlu-go-live.md:13` tells the reader to hand-edit
`%LOCALAPPDATA%\quinn-ops\settings.json`; after this branch it is
`%LOCALAPPDATA%\knowlu\profiles\<id>\settings.json` and is edited from the settings panel. Fix
that line only.
### B6. Wherever the docs say the old root is "moved" or "renamed" whole (README data-root
paragraph, CLAUDE.md Knowlu section, HANDOFF plan-4a block, spec §2), make them say the flat
files are folded into the profile folder and the old root's other contents are left alone
(A4.1).

## Explicitly NOT in this wave (follow-ups, already ledgered)
Migration errors surfaced in the windows-subsystem build (4b); `picker-check.py` (4b); stale
`.knowlu-new-*` sweep (4b); ingest fetch-failed test and summary-line assertion (4b);
`ics_state` NotFound vs locked (4b); orphan credential on an abandoned wizard and the
`credentialsStranded()` test (4b); the four staged-update observables and the three R-P4a-24
items (key-gated step); *Forget this profile* (4b); the `QUINN_OPS_DEVICE` test race (plan 3).

## B. Docs

### B1. CLAUDE.md (Task 11 review Critical)
Lines ~211–212 say that apart from `state`, `note`, `get_settings` and `settings_context`,
"everything else touches app data, `profiles.json`, the clipboard, the process or the updater —
never the vault." False for `ui_event`, which writes `<vault>/state/events-ui/` via
`JsonlLedger` (`src/uievents.rs:58`). Fix by scoping the sentence the way `app/README.md` does
("never the vault's notes") and naming `ui_event`'s ledger write explicitly. Keep CRLF; change
only that sentence.

### B2. Rule-1 wording (Task 11 review minor)
The rule-1 amendment about `quinn-ops ingest` reads mid-sentence; make it its own sentence. Same
meaning, no new claims.

## Gates
`cd app; cargo test` zero new warnings (the `.rsrc merge failure` linker line is pre-existing);
root `cargo test` 0 warnings (unchanged code under `src/`, but run it once: `tests/oracle.rs`,
`tests/surface_oracle.rs`, `tests/site.rs` green); `scripts/wizard-check.py` and
`scripts/settings-check.py` pass headless; line endings per file preserved (`tr -cd '\r' | wc -c`
vs `wc -l`; never `sed -i`, never `grep -c $'\r'`, never `Get-Content` to check);
`git diff --stat` shows no whole-file flip. Scratch copies only; never the live vault; never an
interactive launch of `knowlu.exe`; no synthetic input; no subagents. Commit with specific
`git add`, `-F <file>` message, both trailers.


---

## Appendix C: the final fix wave report

# Knowlu plan 4a — the final fix wave (report)

Branch `worktree-knowlu-plan-4a`, base `1837638`. Three commits, working tree clean.

| commit | what |
|---|---|
| `ec1c7a1` | A4.1 alone — `migrate_flat_layout` folds our own files and never renames `%LOCALAPPDATA%\quinn-ops` |
| `b9a64a2` | A1, A2, A3, A4.2, A4.3, A4.4, A4.6 — code |
| `7432b7c` | B1–B6 + A4.5 — docs (and the one `site/privacy.html` line) |

## Gates

- **`cd app; cargo test`** — 90 passed, 0 failed, 1 ignored, across `commands` 23, `credentials` 2,
  `onboarding` 5, `profiles` 11, `scaffold` 10, `scheduler` 13 (+1 ignored), `static_assets` 23,
  `updates` 3. Warnings: only the pre-existing `.rsrc merge failure: multiple non-default manifests`
  linker line (reported twice, once as a duplicate). Was 87 + 1 ignored at `1837638`; the three new
  tests are the +3.
- **Root `cargo test`** — 814 passed, 0 failed, 2 ignored, **0 warnings**: lib 800 (+2 ignored),
  bin 2, `dependency_boundary` 3, **`oracle` 3**, **`site` 1**, **`surface_oracle` 4**,
  `uievents_isolation` 1. Run twice (after the code commit and after the docs commit); identical
  both times. **The known `runs::tests::start_step_end_records_full_text_and_fold_steps` flake did
  not fire in either run.**
- **`scripts/wizard-check.py`** — `ok` (exit 0), headless Chromium, with the new panel-3 case in it.
- **`scripts/settings-check.py`** — `ok` (exit 0), headless.
- **`scripts/console-shots.py`** — exit 0 on `tests/fixtures/surface-today-full.json` into
  `.superpowers/sdd/2026-09-05-knowlu-friends-shell-plan/shots-final-fix-wave/`; every viewport ≥ 820
  `ok` across Today and the three plan-2 views, the 390px phone shot informational. 32 view shots +
  8 wizard + 8 picker + **8 settings**.
- **`git diff --stat 1837638..HEAD`** — 24 files, 477 insertions, 163 deletions; no file shows a
  whole-file line-ending flip (every count matches the real edit).
- **Working tree** — clean (`git status --short` empty). Nothing under `.superpowers/` was committed.

### `%LOCALAPPDATA%\quinn-ops` — before and after, unchanged

|  | before (session start) | after (all work done) |
|---|---|---|
| `dual\` | 2 entries | 2 entries |
| `logs\` | 107 entries | 107 entries |
| `rehearsal\` | 0 | 0 |
| `scratch\` | 5 | 5 |
| `shots\` | 5 | 5 |
| `%LOCALAPPDATA%\knowlu` | does not exist | does not exist |

No test, script or command in this wave resolved the real app-data root: every profiles test builds
its own temp base and calls `migrate_flat_layout(&base, …)` / `app_data_root_in(temp)`. No
credential was touched (the two credential tests that run are the pre-existing derived-temp-path
ones with their `Drop` guard). No vault outside `tests/fixtures/` and the tests' own temp dirs was
opened; `knowlu.exe` was never launched.

## Per item

### A1 — R-P4a-25, the backup-folder rule
- `app/src/onboarding.rs:304-321` — `check_backup_dir` drops the `v.starts_with(b\)` disjunct;
  equality and `b.starts_with(v\)` remain, comparison unchanged (case-insensitive,
  separator-normalised, only at a separator). Refusal text is now *"the backup folder cannot be the
  vault, or inside it"*; the doc comment above it states the ruling and why the allowed direction has
  to be allowed.
- `app/static/console.js:1228-1235` — the wizard's mirror keeps `within(WIZ.bdir, dest())` and drops
  `within(dest(), WIZ.bdir)`; text matched to the engine's.
- Covering test: `app/tests/onboarding.rs::a_vault_that_cannot_be_finished_is_removed_and_nothing_is_registered`
  now drives four cases through `create_vault_in` — backup inside the vault → refused **and rolled
  back** ("was removed", nothing registered); backup == vault → refused; **vault inside backup (the
  vault's own parent) → allowed**; backup elsewhere → allowed. **PASS.**
- `scripts/wizard-check.py:84-94` — new step 3a: on panel 3 it clicks *Choose folder…* (the fake
  picker answers `C:\Docs\Knowlu`, which is the vault's parent), asserts the panel shows it, asserts
  **Next advances to panel 4**, then goes Back and Skips so the rest of the run is the no-backup path
  it always was. **PASS (`ok`).**
- The old test's premise was the case the ruling flips (it used the vault's parent as the refused
  example), so it was rebuilt rather than extended — the rollback assertion it existed for is kept,
  now on the genuinely-refused case.

### A2 — `KNOWLU_SHOTS.openSettings`
- `app/static/console.js:1340-1345` — the seam exports three: `startWizard`, `renderPicker`,
  `openSettings` (the console's own function, unchanged, no new behaviour).
- `scripts/console-shots.py:84-97` — a third `(panel, js)` pair, `("settings",
  "KNOWLU_SHOTS.openSettings()")`, writing `<w>-settings.png` exactly as the other two do.
- `app/tests/static_assets.rs:339-341` — the pinned literal is the three-export line. **PASS.**
- `docs/surface/anatomy.md` §8 and the spec's §0.0 §8 bullet now say `console-shots.py` shoots all
  three and `settings-check.py` is the one with the rows filled in.
- **`1280-settings.png`** (looked at): the settings overlay sits pinned to the top-right over a fully
  rendered console (the Issues view, which is the hash the shot loop leaves behind), one lift above
  the page and covering the right rail — `SETTINGS` with *Close*, then all six rows as a
  label/value/control grid: *Profile name* (empty field + *Save*), *Vault* (*Copy*, *Switch
  profile…*), *Backup folder* (*Choose…*, *Back up now*), *Start with Windows* (unchecked box),
  *Updates* (*Check now*) and *Diagnostics* (*Copy diagnostics*). The values are blank because there
  is no backend behind `settings_context`/`get_settings` in this script — which is the point of the
  shot being about layout — and there is no horizontal overflow.
- **`1280-wizard.png`** (looked at): unchanged from the Task 11 description — a single centred column
  on the bare canvas, **Knowlu** top-left with `step 1 of 7` right-aligned, *Welcome*, the privacy
  paragraph over the "what should I work on today" line, *Back* / *Next* bottom-right.

### A3 — nits
- `app/build.rs` — trailing `\r\n` restored (the file's own style; `od -c | tail -2` now ends
  `} \r \n`). CR 80 → 81, LF 80 → 81.
- `scripts/release.ps1:30-34` — the `Invoke-Native` comment says it hands back the native command's
  stdout (PowerShell's implicit output, which the `rustc -vV` call site captures and reads `host:`
  out of), adds no status of its own, and that every caller judges `$LASTEXITCODE`.
  Covered by `app/tests/static_assets.rs::the_release_scripts_parse_under_powershell_5_1`. **PASS.**

### A4.1 — CRITICAL C1 / R-P4a-28 (committed alone, `ec1c7a1`)
- `app/src/profiles.rs:95-131` — `FLAT` is the two files only; new `is_app_log` (`slot-*`, `quit-*`),
  `app_logs_in`, `has_flat_layout`, `fold_one`; `is_empty_dir` repurposed.
- `app/src/profiles.rs:133-205` — the rename half is **gone**. The fold reads both roots, moves
  `settings.json` / `seen.txt`, then moves only the app's own files out of `logs\` one at a time,
  removes `logs\` only if that emptied it, and removes `quinn-ops` only if the fold emptied it.
- `app/src/state.rs:184-189` — `app_data_root_in`'s doc no longer says the migration "refuses once
  `knowlu` exists"; it says why the order still matters and that the fold renames nothing.
- Covering tests, all **PASS**:
  - `nothing_but_the_apps_own_files_is_ever_moved_out_of_the_old_root` (new) — plants
    `dual/keep.txt`, `logs/dual-20260903.txt`, `logs/slot-20260903-1200.txt`,
    `logs/quit-20260903T1201.txt`, `scratch/v/config/planning.yaml` beside the flat files; asserts the
    three foreign paths are byte-identical **at their original paths** afterwards, `logs\` still
    exists, ours moved, nothing foreign reached `knowlu\`, the old root survives, and a second call
    returns `None`.
  - `the_flat_files_fold_into_the_profile_folder_and_nothing_is_lost` (renamed from
    `…moves_whole…`) — keeps every assertion including `!old.exists()`, now explained: our three
    files were all it held, so the fold emptied it.
  - `a_file_someone_else_holds_open_costs_the_fold_nothing` (was
    `a_rename_that_cannot_happen_still_moves_every_flat_file_and_says_why`) — the held handle now
    costs nothing: errors empty, `busy.txt` neither moved nor removed, every flat file still arrives.
  - `an_empty_knowlu_left_by_an_earlier_launch_does_not_block_the_fold` — same assertions, rename
    premise removed.
  - `a_half_moved_layout_is_folded_on_the_next_launch_and_never_stranded` — unchanged, re-worded.
- Docs updated for the same fact: `CLAUDE.md` Knowlu section, `app/README.md` data-root paragraph,
  `docs/HANDOFF.md` plan-4a block, spec §2 and a new §0.0 bullet (all in `7432b7c`, item B6).

### A4.2 — I1
- `app/src/profiles.rs:245-258` — new pure `fallback_shell(root) -> Option<Vec<Profile>>`: `Some` only
  for a registry that loads and is non-empty; `None` for absent, empty **and unreadable** (an
  unreadable registry must not become an empty picker).
- `app/src/main.rs:100-125` — `run_console` calls it when `resolve_vault` fails: picker if `Some`
  (the reason is still printed as a `Knowlu: …` line), the existing `fatal` if `None`.
- `app/src/onboarding.rs:59-77` — `adopt_vault_in` now requires `config/planning.yaml` to be a file
  (plus `tasks/`), the console's own predicate; message says `config/planning.yaml and tasks/`.
- Covering tests, both **PASS**:
  `app/tests/profiles.rs::a_profile_whose_vault_is_gone_falls_back_to_the_picker` (registers a real
  vault, deletes the folder, asserts `resolve_vault` errs and `fallback_shell` offers that profile;
  plus empty and corrupt registries → `None`) and
  `app/tests/onboarding.rs::adopting_a_vault_writes_nothing_into_it` (a folder with `config/` and
  `tasks/` but no `planning.yaml` is refused, and `resolve_vault` agrees).
- Judgement call on the brief's parenthetical: it says both "keep the fatal for the `--vault` case
  with zero profiles" and "where the wizard is the right shell". I kept the **fatal** — it is the
  explicit instruction, and a wizard opening because a `--vault` the user typed does not exist would
  hide the mistake. `app/README.md`'s launch table gains the row.

### A4.3 — I2
- `app/tests/scaffold.rs:269-295` — `every_view_answers_over_a_vault_the_wizard_has_just_made`:
  `create_vault` → asserts no `state/today.md`, no `state/runs/`, no `state/runner-log.md`, no
  `.git` → `ConsoleState::open` → `state_inner` for all nine names `surface::View::parse` accepts
  (`today, overdue, week, later, all, decisions, good-to-know, issues, runs`), each `ok == true` and
  `schema == 1`. **PASS.**

### A4.4 — I3 + the duplicate
- `app/src/commands.rs:482-491` — `updates_dir` is `pub(crate)` and its doc says one folder per
  install, not per profile (citing `profiles::updates_dir` and spec §6).
- `app/src/scheduler.rs:585-590` — the housekeeping tick calls `crate::commands::updates_dir()`
  instead of its own copy. Covered by the existing `app/tests/updates.rs` trio and the app build.
  **PASS.**

### A4.5 — I4 (README)
- `app/README.md`, *Never point this at the live vault* — a new paragraph: every `--vault` launch
  registers a profile, nothing removes one yet, with two or more a plain launch shows the picker
  (that is the rules working, not a fault), pass `--vault` to go straight in, and *Forget this
  profile* is a plan-4b item — with the interim manual step named.

### A4.6 — minors
- (a) `app/static/console.css:336-342` — `.upd { display:flex; align-items:baseline; gap:var(--s2);
  font-family:var(--mono); font-size:11px; margin:0 0 var(--s2); }` plus **`.upd[hidden] { display:
  none; }`**. The `[hidden]` companion is not optional: `renderUpdateOffer` hides the element on
  every poll with nothing staged, and a class `display` beats the UA rule — the `.app[hidden]` /
  `.wiz-row[hidden]` lesson. No new colour; existing tokens only. The stale comment at (now) 470-472
  no longer calls the Updates row a disabled placeholder.
- (b) `app/tests/static_assets.rs::the_settings_panel_has_its_rows_and_one_way_in` — `set-updates`
  joins the row loop (six rows: D9's "five plus Updates"). **PASS.**
- (c) `app/src/scheduler.rs:29-34` and `:564-566` — both `LOGS_KEPT` comments corrected: one file per
  step, three steps on a vault with a feed, six files a day, sixty ≈ 20 runs ≈ 10 days; `quit-*` is
  neither counted nor pruned. **Found two such comments, not three** — I grepped `scheduler.rs`,
  `app/tests/scheduler.rs` and the rest of `app/` for `fortnight`, `four files`, `four slot`,
  `sixty`, `LOGS_KEPT` and `60`; the only count claims were those two. `prune_logs`'s own doc makes
  no arithmetic claim and was left alone.
- (d) `tests/site.rs` — test renamed to
  `the_site_is_plain_html_and_carries_the_privacy_sentence_on_both_pages`, and the module doc now
  says it pins the sentence against its own literal and that the wizard↔site cross-check is
  `app/tests/static_assets.rs::the_wizards_privacy_sentence_is_the_sites_privacy_sentence`
  (R-P4a-21). **PASS.**

### B1 — CLAUDE.md's "never the vault"
`CLAUDE.md` Knowlu section: the sentence is scoped to **the vault's notes**, and names `ui_event`'s
write into `<vault>\state\events-ui\` through `JsonlLedger` (`src/uievents.rs`) and `mark_seen`'s
`seen.txt` in app data. CRLF kept; nothing else in the paragraph changed.

### B2 — rule-1 wording
The `quinn-ops ingest` amendment is its own sentence after the Tasks 16/17 list: *"That `clap` claim
was false until Knowlu plan 4a (2026-09-06), which added the one command that was missing:
`quinn-ops ingest`."* Same meaning, no new claims.

### B3 — `site/privacy.html`
Line 16: "the logs of the last sixty runs" → "the last sixty run logs". The privacy **sentence**
(line 12), which `app/tests/static_assets.rs` pins against `console.js`, is byte-identical — both
that test and `tests/site.rs` pass.

### B4 — spec body
- §3.1 seed record → `WriteContext::new("system:migration", "cli")`, with R-P4a-6 named and the
  struck `via: "onboarding"` said out loud.
- §10 decision-5 row → `system:migration`/`cli` (amended by R-P4a-6).
- §0.0's R-P4a-25 bullet → **panel 3**, "(Panel 4 is the LMS calendar.)", and the "code catches up in
  the fix wave" note becomes "done in the final fix wave".
- §0.0's §8 bullet → the three-export `KNOWLU_SHOTS` and `console-shots.py`'s settings shot.

### B5 — `docs/runners/knowlu-go-live.md`
Step 3 now sets the backup folder **in the settings panel** and names
`%LOCALAPPDATA%\knowlu\profiles\<profile_id>\settings.json` as where it lands. **Beyond the letter of
"fix that line only":** the four-line JSON template underneath existed only to be hand-typed into the
old path, so it went with the instruction; leaving a JSON example pointing at a file nobody should
edit would have been worse than the line itself. The `back up now` verification and its refusal-toast
sentence are kept.

### B6 — "moved"/"renamed" whole
`CLAUDE.md` (Knowlu section), `app/README.md` (data-root paragraph), `docs/HANDOFF.md` (plan-4a
block), spec §2, plus a new §0.0 bullet for R-P4a-28. All four now say the flat files fold in file by
file and the old root's other contents are left alone, and all four name what is in that directory on
this machine, since that is why the ruling exists.

## Line endings

`tr -cd '\r' | wc -c` against `wc -l`, before → after, for every file touched. LF files stayed LF;
CRLF files kept CR == LF throughout. No `sed -i`, no `Get-Content`; all CRLF edits went through a
byte-preserving helper in the git-ignored `target/` (removed afterwards).

| file | before (CR/LF) | after (CR/LF) |
|---|---|---|
| `app/src/profiles.rs` | 0 / 238 | 0 / 297 |
| `app/tests/profiles.rs` | 0 / 214 | 0 / 296 |
| `app/tests/onboarding.rs` | 0 / 196 | 0 / 226 |
| `app/tests/scaffold.rs` | 0 / 341 | 0 / 371 |
| `scripts/wizard-check.py` | 0 / 187 | 0 / 198 |
| `app/src/state.rs` | 264 / 264 | 265 / 265 |
| `app/src/onboarding.rs` | 412 / 412 | 421 / 421 |
| `app/src/main.rs` | 159 / 159 | 171 / 171 |
| `app/src/commands.rs` | 531 / 531 | 536 / 536 |
| `app/src/scheduler.rs` | 595 / 595 | 601 / 601 |
| `app/build.rs` | 80 / 80 | 81 / 81 |
| `app/static/console.js` | 1349 / 1349 | 1355 / 1355 |
| `app/static/console.css` | 465 / 465 | 474 / 474 |
| `app/tests/static_assets.rs` | 509 / 509 | 513 / 513 |
| `scripts/console-shots.py` | 100 / 100 | 105 / 105 |
| `scripts/release.ps1` | 275 / 275 | 278 / 278 |
| `tests/site.rs` | 27 / 27 | 33 / 33 |
| `CLAUDE.md` | 372 / 372 | 379 / 379 |
| `app/README.md` | 269 / 269 | 281 / 281 |
| `docs/HANDOFF.md` | 1154 / 1154 | 1158 / 1158 |
| `docs/surface/anatomy.md` | 593 / 593 | 595 / 595 |
| `docs/superpowers/specs/2026-09-05-knowlu-friends-shell-design.md` | 443 / 443 | 458 / 458 |
| `docs/runners/knowlu-go-live.md` | 27 / 27 | 23 / 23 |
| `site/privacy.html` | 30 / 30 | 30 / 30 |

(`knowlu-go-live.md` loses four lines because the JSON block went — see B5.)

## Where the brief and the code disagreed

1. **A4.6(c) says "the three stale `scheduler.rs` comments"; there are two.** Both fixed; the search
   that found them is recorded above.
2. **A1's test target.** `check_backup_dir` is private and reached only through `create_vault_in` /
   `apply_profile_settings` (the latter needs a Tauri handle), so the three cases are driven through
   `create_vault_in`, as the brief's "(or the command that calls it)" allows. Doing so meant
   rewriting the existing rollback test, whose refused example *was* the case the ruling now allows.
3. **A4.2's parenthetical is self-contradictory** ("keep the fatal … where the wizard is the right
   shell"). Resolved in favour of the explicit "keep the fatal"; reasoning under A4.2.
4. **A4.1's "remove the old root if the fold left it empty"** also needs the emptied `logs\` removed,
   or a root holding one empty directory would never qualify — and the pre-existing
   `!old.exists()` assertions would have failed. `logs\` is removed only when it is empty after the
   fold, never otherwise.
5. **B5 removed the JSON block** attached to the line it fixed (see B5).
6. **The `site/privacy.html` line rides in the `docs:` commit** rather than a separate `site:` one:
   it is one line and belongs with B1–B6.

## Not done (correctly out of scope)

Everything under the brief's *Explicitly NOT in this wave* — the windows-subsystem migration errors,
`picker-check.py`, the `.knowlu-new-*` sweep, the ingest fetch-failed test, `ics_state` NotFound vs
locked, the orphan-credential/`credentialsStranded()` case, the four staged-update observables and
the three R-P4a-24 items, *Forget this profile*, and the `QUINN_OPS_DEVICE` test race.


---

## Appendix D: the plan review and rulings (pre-execution)

# Plan 4a — independent plan review 1 (opus) and the controller's rulings (2026-09-05)

Apply every item below to `docs/superpowers/plans/2026-09-05-knowlu-friends-shell-plan.md`. Line numbers refer to the 3173-line revision. Rulings are marked **R-P4a-N**.

## Blocking

B1 (line 651) — `main.rs` restructure leaves the wizard unreachable: `resolve_vault(...) → fatal(...)` runs before the `--run-slot-once` block, so a first launch with no `--vault` dies in a MessageBox. `run_console`'s body is an ellipsis. **R-P4a-10:** write `fn main` out in full — parse args → `--after-pid` wait → `--run-slot-once` (resolving its own vault; fatal there is right) → `migrate_flat_layout` → `resolve_launch` → `run_shell` / `run_console` — and write `run_console`'s builder out verbatim (cwd, the leaked `--vault` autostart argument, the `.setup()` capture of `data_dir`). No ellipses anywhere in the plan.

B2 (lines 1297, 1883, 1959, 2082) — `pick_folder` returns an existing folder; `create_vault`/`restore_vault` refuse an existing `dest`; panel 2 has no name field or default path. **R-P4a-11:** panel 2 picks a PARENT folder and has a name field; default parent `%USERPROFILE%\Documents\Knowlu`, default name from the profile name; `dest = <parent>\<name>` and must not exist (spec §3 panel 2 restored); same for restore. The wizard shows the resulting path before Finish.

B3 (lines 797, 1825, 2214) — the three new count-equality assertions use `data-id="` / `data-kind="`; the repo's invariant (static_assets.rs:272) is the space-prefixed `" data-id=\""` vs `" data-kind=\""`. **Fix:** copy the space-prefixed spelling verbatim in all three.

B4 (lines 2532, 2534, 2548) — `tauri::async_runtime::block_on` inside `#[tauri::command(async)]` panics (the body already runs on the tokio runtime). **R-P4a-12:** `check_for_updates` and `install_update` become `async fn` commands that `.await` async helpers in `updates.rs`; the tray arm and the housekeeping tick (plain std threads) keep `block_on`. Say so in a comment at each.

B5 (lines 2647, 2707) — registering `tauri_plugin_updater` with no `plugins.updater` config panics at startup (`pubkey` is a required field), so a commit exists in which the app cannot start. **R-P4a-13:** Task 8 is split in two halves inside the task: steps that need no plugin land first (`updates.rs` model, `update_offer`, the mid-run deferral, the commands returning `updater not configured`, the topline/tray offer rendering, all tests); the plugin registration + `plugins.updater` config block + the real `check`/`install` paths land in ONE final step that is gated on Quinn's public key and is committed together or not at all. No intermediate commit registers the plugin without its config. Task 10's (site) assertion on `plugins.updater.endpoints` moves to that same gated step (the site task must not depend on it).

B6 (line 2859) — `release.ps1` never provides `TAURI_SIGNING_PRIVATE_KEY`, so updater artefacts cannot be produced. **R-P4a-14:** the script reads `knowlu/updater-key` (and, if set, `knowlu/updater-key-password`) from Credential Manager via a small `Add-Type` C# `CredReadW` helper into `$env:TAURI_SIGNING_PRIVATE_KEY` / `$env:TAURI_SIGNING_PRIVATE_KEY_PASSWORD` immediately before `cargo tauri build`, and clears both variables in a `finally`. The key never touches disk and never appears in output; the script refuses to run with `createUpdaterArtifacts: true` if the credential is absent, with one line saying which credential to create.

## Should fix before execution (all accepted; rulings where a choice was needed)

S1 (2272 vs 651/699) — `onboarding::pick_folder`/`launch_state` already registered in Task 2's `generate_handler!`; Task 7 must add only its four `commands::*` arms (a duplicate arm is a warning and breaks the zero-warnings gate).
S2 (670) — `Launch::Open` registers the profile only AFTER `resolve_vault` succeeds.
S3 (2084) — the adopt path gets the `plan` too (or calls `finish_profile`) so panel 3's backup folder and autostart are kept.
S4 (2320) — `settings_context` returns the profile name as well; `renderSettings` reads it from there.
S5 (409) — `migrate_flat_layout`: if no id can be derived, fold the flat files under a fallback id `profile_legacy` (never strand them at the new root); idempotent on a half-moved state (test it: run twice, and once from a state where the folder is renamed but not folded).
S6 (2887) — see R-P4a-13: the site test does not assert the updater endpoint; the gated step does.
S7 (spec §8) — add the two Rust tests: adopting a vault writes nothing into it (hash the tree before/after); restoring never writes into the backup folder (hash the mirror before/after). Add the minimal Playwright behaviour checks for the wizard (panel order, Back/Next, ICS shape validation, credential fields cleared after `store_credentials` resolves) and the settings overlay (Save invokes `set_settings` with the payload) to the tasks that build them — checks, not only screenshots.
S8 (spec §2) — the picker gets *Add another…* → the wizard; the settings overlay gets *Switch profile…* → the picker (relaunch per R-P4a-1). **R-P4a-15:** both are in scope (spec §2 names the first; the second is the same relaunch path and costs one button).
S9 (2843/2858) — signing must happen before the bundler packs. **R-P4a-16:** use Tauri 2's `bundle.windows.signCommand` (the bundler invokes it per binary before packing) pointing at a `scripts/sign.ps1` wrapper around `signtool` + the Azure Trusted Signing dlib; the wrapper exits 0 with one loud `UNSIGNED:` line when the profile is not configured, so unsigned dev builds still bundle. `release.ps1` no longer signs after the fact.
S10 (2873) — `latest.json` written UTF-8 without BOM via `[System.IO.File]::WriteAllText($p, $json, (New-Object System.Text.UTF8Encoding($false)))`.
S11 (2568–2626) — `install_update` takes the `Scheduler.running` flag for the duration of the install (through `RunGuard` or an equivalent), so a slot cannot start mid-install; test it with a fake staged update.
S12 (3135) — Task 11 says plan 2 Task 12's step-1 test (`the_app_data_root_is_knowlu_and_an_old_root_is_moved_once`, root-level `settings.json`) is superseded by `app/tests/profiles.rs` and must not be written as the runner-leaves plan spells it.
S13 (2137/2143) — fix the `console-shots.py` snippet to the `KNOWLU_SHOTS.startWizard(...)` seam and add the static assertion that names the seam.

## Minor (apply; each is one line)

M1 (57) ledger R2 test name → `one_migration_record_and_the_rest_are_dashboard_writes`. M2 (1795/2165) Interfaces: `restore_vault(backup, dest, name, plan)`; add `copy_text`, `settings_context` to Task 7's lists. M3 (703–706) "three edits"/"four edits" → one number. M4 `generate_context!()` twice: accept, add one sentence to Task 2 noting the size cost and that plan 4b may factor it. M5 site: the release script also copies the installer to the stable name `Knowlu-setup.exe` and `index.html` links that; the versioned file stays beside it. M6 the Azure dlib path becomes a `-SignDlib` parameter with that default (used by `sign.ps1`). M7 **R-P4a-17:** `has_ics_url` distinguishes the parse-error branch: the skip step reads `ingest (skipped: config unreadable)` when the YAML does not parse and `ingest (skipped: no ics_url)` when it parses without a url; a missing file is `no ics_url`. M8 the Updates row shows version, last check time, staged/none, error — "channel" is dropped (there is one channel; say so in the spec amendment at Task 11).

## Rulings/spec reconciliation for Task 11 (the close)
- R-P4a-5's wording: the Updates row's markup may land with Task 7 as a disabled row labelled *updates arrive with the next build* and is wired in Task 8 — record this in the spec's §4 amendment.
- Spec §3 panel 2 default path restored (R-P4a-11); §2 "two profiles open" withdrawn (R-P4a-1); §2 *Add another* and *Switch profile* (R-P4a-15); decision 5 via `cli` (R-P4a-6); §4 no "channel" (M8).

Report as before: path, line count, CR == LF, the eleven task titles, and any new open point. Do not commit.
