# L6 runtime removal: T7 whole-lane review

Reviewer, 2026-10-20, inside the wave-1 workflow. Verdict: FAIL. Filed by the controller.

L6 runtime removal, T7 whole-branch review (reviewer). Verdict: FAIL, on one critical finding: T5b is not committed.
Scope: the 8 commits in origin/main..HEAD (merge base ba7eb0b, 195cbf4..f83ed3a), plus the uncommitted T5b worktree diff, reviewed as content. No contract-list file is in the diff.

CRITICAL
1. engine/src/runtime.rs and the T5b files: T5b never landed as a commit.
   - HEAD f83ed3a still has runtime.rs, `pub mod runtime;`, judge.rs's GRAMMAR, prompt_for and the not-installed variants, and 4 ignored tests.
   - The T5b edits are in the worktree only. The index holds only `D engine/src/runtime.rs`; the other 7 files are unstaged. The ledger shows no T5b commit row.
   - If T8 pushes HEAD, CI fails no_local_runtime (files and names). f83ed3a's docs, which say 3 ignored tests and runtime.rs gone, would then contradict the code.
   - A bare `git commit` now records the runtime.rs deletion with lib.rs still declaring `pub mod runtime;`, so the engine fails to compile at that commit.
   - Fix: commit all eight T5b paths with the plan's message, add the ledger row (expected 2802/0/3, .rsrc only), confirm a clean `git status`, and re-run T7 briefly on the committed tree.

MINOR
2. app/Cargo.toml:69: the ureq comment leaves out app/src/mail_client.rs, which also uses ureq. Add "mail" to the list.

T7 checks (the T5b content is judged from the worktree diff)
1. §3 rows and §5.2 file list: every changed file is in §5.2 or §3.5 (T6 and T6b).
   - cloud_contract.rs: exactly the two label_opts() lines.
   - mail_pass.rs: the two opts() fields.
   - app/src/grades.rs and engine/src/childproc.rs: one doc line each.
   - slot_argv_is_unchanged: only the JudgeArgs line goes and Local becomes Cloud{logs}; every assertion is byte-identical.
   - judgments_dir moved with its body unchanged (`profile_data_dir.join("judgments")`).
   - No files outside the plan changed: cli.rs, account.rs, surface.rs, events.rs, cloud/ and fixtures are untouched.
2. Deleted tests match the §3.4 table exactly:
   - runtime.rs: 5 tests plus the ignored smoke test.
   - judge.rs: 5 tests.
   - enrich.rs: run_lines_with_no_runtime_anywhere, replaced by run_lines_on_a_vault_with_no_account_names_it_first_and_exits_zero, which also asserts tier 1, the flag and the logged outcome.
   - app/tests/inference.rs (whole file); commands.rs's marshal test; judge_state_names_the_missing_half; the 6 offer-marker tests and marker_for; the Local judgment row test.
   - Rewritten tests keep their property. The mail-marker test loses only the inference-marker assertion and its doc clause. The judgelog stderr-secret property holds with a neutral literal. no_console's floor is now >= 1 and still non-vacuous.
3. The real-runtime smoke test is deleted, not un-ignored. The worktree's #[ignore = ...] attributes: events.rs traps 4 and 5, and scheduler.rs run_slot_end_to_end. That makes 3. (HEAD still has 4; see finding 1.)
4. The JudgePlan::Cloud argv arm is untouched, and the log_dir pin is tightened to equality. No --runtime or --model remains in any argv. The engine's clap refuses both flags (judge_refuses_the_runtime_and_model_flags).
5. commands.rs is deletions only (settings_context's marker read, inference_root and the four commands).
   - The main.rs diff touches only the console list's four names.
   - The wizard list is unchanged.
   - handler_lists pins pick_file in both lists.
   - README recount: console 66, wizard 29, attributes 85.
6. Rule 2: no change under engine/tests/fixtures/, and no surface-today-*.json moved.
7. The contract list is untouched, file by file. cloud_contract.rs is off the list by ruling Q9.
8. Warnings: the ledger reports .rsrc only for every task, including the T5b worktree run; I did not run cargo.
   - T6b's diff is comment lines only (3 files, +3/-3).
   - tauri-cli.ps1 is CRLF in the worktree (attr eol=crlf). The new test files and docs are LF.

Other checks
- Rule 1: nothing new names a person, vault, machine or account. Test temp names use the file's existing qo-console prefix.
- The new tests make no network calls. judge_without_runtime env_remove()s KNOWLU_RUNTIME and KNOWLU_MODEL on the child only.
- Engine invariants: rank is untouched. No note re-dump, no new JSON emission, no VIAS, run-record or frontmatter change.
- On the worktree, the tripwire's token grep over engine/src, app/src, app/static and both Cargo.toml files is empty, and so are the GRAMMAR/prompt_for and runtime-word greps.
- Behaviour: a vault with no account gives `judge: skipped (<NoConfig text>)` and tier 1 at exit 0 in the engine, and `judge (skipped: no account)` at code 0 with engine_ok in the app. This matches the Q1 ruling.
- Privacy: no new data class, no PRIVACY_VERSION change, nothing new in logs.

Out of scope: the user's question about W8 was relayed with this dispatch. This review covers only L6 T7 and does not check W8's status; the controller should answer that separately.
