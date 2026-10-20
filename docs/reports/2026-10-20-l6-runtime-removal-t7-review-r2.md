# L6 runtime removal: T7 whole-lane review (r2)

Reviewer, 2026-10-20, inside the wave-1 workflow. Verdict: PASS. Filed by the controller.

L6 runtime removal, T7 whole-branch re-review (after fixes)
Branch l6-runtime-removal; diff origin/main...HEAD (merge-base ba7eb0b), 9 commits 195cbf4..b143738. No contract-list file is in the diff stat.

VERDICT: PASS, with 3 minor findings and no critical or important ones.

Earlier finding: closed. b143738 commits T5b's eight paths, which had never been committed. engine/src/runtime.rs is gone, engine/src/lib.rs no longer declares `pub mod runtime`, and the ledger row records 2802 passed, 0 failed, 3 ignored, with only the .rsrc warning.

Plan T7 checks:
1. Every §3 row landed as written, and no file outside §5.2/§3.5 changed (the T6 docs and the three T6b comment files are the only additions to the list). Spot checks:
   - cloud_contract.rs: only the two `label_opts()` lines changed.
   - mail_pass.rs: only the two `opts()` fields changed.
   - app/src/grades.rs:87 and engine/src/childproc.rs:4: one doc line each.
   - `slot_argv_is_unchanged`: the `JudgeArgs` line is gone and the plan is `Cloud{log_dir:"logs"}`. Every assertion is byte for byte.
   - app/Cargo.toml: zip is gone, sha2 and ureq stay with reworded comments. Cargo.lock loses only the knowlu → zip edge.
2. Every deleted test is in the §3.4 "Deleted" table: 5 runtime.rs tests plus the smoke test, 5 judge.rs tests, 1 enrich.rs test, 1 commands.rs test, 1 scheduler.rs test, the 6 marker tests and `marker_for` in onboarding.rs, 1 static_assets.rs test, and the whole of app/tests/inference.rs.
   - Rewritten tests keep their properties. The mail-marker test loses only the independence assertion and its doc clause. no_console's floor drops from >=2 to >=1 and stays non-vacuous. The no-account test pins `engine_ok`.
   - The new tests are present: the no_local_runtime tripwire (23 tokens, files gone, no zip, a non-vacuous scanner check), the 4 tests in judge_without_runtime.rs, the handler-list test, the scheduler skip test with log_dir pinned by equality, and the static page test.
3. Ignored count: 3. git grep shows events.rs:672 and :793 and app/tests/scheduler.rs:647. The real-runtime smoke test was deleted with its module, not un-ignored.
4. `JudgePlan::Cloud` argv is unchanged: judge --vault --via local-runner --log-dir. judgments_dir moved to scheduler.rs with the same body, and the test now pins the folder by equality. No --runtime or --model reaches any argv. The engine's clap now refuses both flags with exit 2, and a test covers it.
5. commands.rs only loses lines: the settings_context marker read, the `offer_inference` key, and the 7 inference functions. The console handler list goes from 70 to 66, exactly the four inference names, counted from the diff. The wizard list is unchanged (one hunk in main.rs), and `pick_file` is in both lists.
6. Rule 2: `git diff --exit-code origin/main...HEAD -- engine/tests/fixtures` is empty. No surface-today-*.json changed.
7. No contract-list file changed (I checked every file on CLAUDE.md's list plus surface_oracle and sync_contract).
8. Warnings: the ledger shows .rsrc only on every task, including the fix. I did not run cargo myself (not permitted). The T6b diff is comment lines only. tauri-cli.ps1 is still CRLF. No other changed file contains a CR.

Invariants: `rank` is untouched, and nothing writes vault bytes or JSON in a new way. The no-account path goes through enrich_with, which writes via `write` as before. No credential, account or vault name was added.

Minor findings:
- app/README.md:117-120: says the step is left out with no session. With an account and no session the step runs, and the engine names the reason on its first line. The paragraph also omits the `cloud.yaml unreadable` skip.
- engine/src/judgelog.rs:250-251: a reworded test doc keeps "tells Quinn", against plan §2's no-names rule. The name was there before the lane.
- The base is behind origin/main by L3 (#47), which touches cloudmodel.rs and cloud_contract.rs. The hunks do not overlap and main adds no removed name, but T8 must merge main in and re-run the full gate before pushing.

On the user's question about W8: this reviewer only reviews L6 and cannot see or start other work. The only W8 mention I found is the session ledger's 2026-10-20 ruling that "a W8 fix is merged into each lane". Nothing in this branch's log carries a W8 fix. Whether W8 is being worked on is for the controller to confirm or dispatch.

Paths: C:/Users/danie/GitHub/knowlu/docs/plans/2026-10-19-l6-runtime-removal-plan.md (T7), C:/Users/danie/GitHub/knowlu/.superpowers/sdd/2026-10-18-l6-runtime-removal/progress.md, worktree C:/Users/danie/GitHub/knowlu/.claude/worktrees/l6-runtime-removal. Report to be filed by the controller at docs/reports/2026-10-19-l6-runtime-removal-review.md.
