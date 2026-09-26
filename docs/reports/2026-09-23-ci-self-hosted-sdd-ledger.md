# CI on the self-hosted runner — SDD ledger

The controller ledger of the one-task change that routes `ci.yml` to a self-hosted runner behind `CI_SELF_HOSTED`, merged as PR #10 (`312d833`). Preserved verbatim from the worktree workspace (`.superpowers/sdd/ci-self-hosted/`, git-ignored) when the stream was closed on 2026-09-25, before the worktree and its branch were deleted.

---

# SDD ledger — plan: (no plan file; one-task change) .superpowers/sdd/ci-self-hosted/task-1-brief.md

- 2026-09-22 Quinn chose the self-hosted runner to unblock Actions (over a spending limit or waiting for the reset). Facts checked on this laptop: pwsh 7 absent (the test job's `shell: pwsh` needs it); `bash` on PATH is WSL's WindowsApps shim (dtolnay/rust-toolchain and eval-gate use `shell: bash` — Git's bash must lead the runner's PATH); deno 2.9.6 expands `**` itself under PowerShell (verified `deno check` exit 0). Design: one repository variable `CI_SELF_HOSTED` = `on` routes all three ci.yml jobs to `[self-hosted, windows, x64, knowlu-ci]`; unset falls back to hosted; release.yml never self-hosted (pinned). Branch `ci-self-hosted` off main `ef085fd`. Implementer dispatched (sonnet), agent a9c075b4da9ca506d. Quinn given the runner-setup steps (pwsh 7, runner into C:\actions-runner, Git bash first on PATH before config, label knowlu-ci, interactive run.cmd, not a service).
- Implementer DONE at `6445aea` (workflows.rs 10/10 after RED; eol-check under PS 5.1 holds; 0 CR; YAML parses). Controller read the diff: matches the brief. Controller finding for runner setup (not the diff): the runner runs as Quinn's user and shares ~/.cargo and ~/.rustup — dtolnay/rust-toolchain would change rustup's default to 1.98.0 and rust-cache prunes the registry; fix = runner-local `.env` with its own CARGO_HOME/RUSTUP_HOME under C:\actions-runner, written by the controller after Quinn configures and before run.cmd. Task review dispatched (sonnet).
- Task review (sonnet): Spec ✅, Quality approved, nothing open. Task 1: complete (commits ef085fd..6445aea, review clean). Pushed; PR #10 → main opened. Merge waits for the runner's first green run and Quinn's word.
- Waiting on Quinn: the runner install (steps given 2026-09-22: pwsh 7, C:\actions-runner, Git bash first on PATH before config.cmd, label knowlu-ci, NOT a service, stop before run.cmd). Then the controller writes C:\actions-runner\.env (CARGO_HOME=C:\actions-runner\_cargo, RUSTUP_HOME=C:\actions-runner\_rustup), checks .path (Git's bin before WindowsApps), Quinn starts run.cmd, the controller sets the repo variable CI_SELF_HOSTED=on (announced), re-runs PR #10's CI, then asks Quinn's word to merge PR #10.
- 2026-09-23 RUNNER SETUP (Quinn: "help me set up the self-hosted runner"):
  - Cost checked: self-hosted minutes are free today. GitHub announced a $0.002/min platform charge for private repos from 2026-03-01, then postponed it indefinitely on 2025-12-16.
  - Controller, done:
    - actions-runner-win-x64-2.337.0.zip downloaded to C:\actions-runner, its SHA-256 checked against the release notes (1150692a…85cfc), and extracted.
    - Registered as `knowlu-laptop` with the labels self-hosted, Windows, X64, knowlu-ci; not a service. The registration token was minted by `gh` under Quinn's login and piped straight into config.cmd, never displayed.
    - `.env`: CARGO_HOME=C:\actions-runner\_cargo, RUSTUP_HOME=C:\actions-runner\_rustup (both folders created).
    - `.path`: Git\bin, then PowerShell\7, then ~\.cargo\bin, then the machine and user PATH (WindowsApps' bash shim at index 16; WinLibs present).
  - Waiting on Quinn: install PowerShell 7 (`winget install --id Microsoft.PowerShell -e --source winget`, a UAC prompt).
  - Then the controller starts run.cmd in its own console window, sets the repo variable CI_SELF_HOSTED=on (announced), re-runs PR #10's CI, and asks Quinn's word to merge.
  - Open risk: whether the account's billing block ("recent account payments have failed…") also stops self-hosted jobs. They are not billed, but the first run is the witness. If they are stopped, the fix is Billing & plans.
- 2026-09-23, first runs:
  - Quinn installed PowerShell 7.6.6 via winget. It is an MSIX: `pwsh` is an app-execution alias in WindowsApps.
  - The controller started run.cmd; the runner came online; CI_SELF_HOSTED=on was set (announced); PR #10's run 35786252787 was re-run.
  - The jobs STARTED, so the billing block does NOT stop self-hosted jobs.
  - Two setup failures:
    - (1) `shell: bash` resolved to WindowsApps' WSL shim, which mangled the temp-script path: the interactive Windows runner did not honour the `.path` file.
    - (2) setup-deno's tool-cache fell back to Windows PowerShell 5.1's `Expand-Archive`, which rejects an extension-less download, because Node's `which` cannot see an app-execution alias, so no pwsh was found.
  - Fix, all under C:\actions-runner:
    - a runner-local PowerShell 7.6.6 in `_tools\pwsh` (the official zip, SHA-256 checked against hashes.sha256 = 02fe458b…c860);
    - a launcher `start-knowlu-runner.cmd` that sets PATH (Git\bin, then _tools\pwsh, then ~\.cargo\bin, then the rest), CARGO_HOME and RUSTUP_HOME, then `call C:\actions-runner\run.cmd`. The full path is needed because the environment sets NoDefaultCurrentDirectoryInExePath;
    - `.path` updated to match.
  - The runner was restarted through the launcher, minimized, with output in _diag\launcher.log: "Listening for Jobs". The run was re-run and is being watched.
- To restart the runner after a reboot: run `C:\actions-runner\start-knowlu-runner.cmd`. It is not a service; the window must stay open while CI runs.
- First complete self-hosted run (35786252787, re-run): cloud ✅ (31 s), eval-gate ✅, test ❌ after 13.5 min. The toolchain install into the runner's own RUSTUP_HOME, the GNU-host assert, rust-cache and eol-check all passed; the workspace was green except ONE test, `app/tests/commands.rs::sync_on_a_non_repo_vault_is_calm_and_backup_needs_a_folder` (the known C0 Task 6 hygiene flake: PID-keyed %TEMP% folders never removed, with 379 stale ones on this laptop, and a reused PID's settings already hold backup_dir). Warnings: 1 accepted, 2 tallies, 0 other. A ~28-min delay before pickup matched a network blip (`gh` could not reach api.github.com).
- Controller fix `cea59a9` (tests only, on PR #10's branch, because a persistent runner is what exposes it): the test removes both PID-keyed folders before opening the console state and after it finishes. commands.rs 25/25 locally; 0 CRs. Pushed; the push's CI run is being watched. It also takes this flake off C0 Task 6's hygiene list, once PR #10 merges to main and the feature branches merge main.
- Run 35880219932 (the push of cea59a9) on the self-hosted runner: ALL GREEN. test ✅ 10m44s, cloud ✅ 1m11s, eval-gate ✅ 22s. PR #10 is ready. Merge needs Quinn's word (a code push to main).
- PR #10 MERGED on Quinn's word (2026-09-23): merge commit 312d833 on main. CI_SELF_HOSTED=on stays; every later PR's CI runs on knowlu-laptop.
- 2026-09-25: STREAM CLOSED on Quinn's word. This ledger is preserved as docs/reports/2026-09-23-ci-self-hosted-sdd-ledger.md; the worktree and branch ci-self-hosted are deleted (PR #10 keeps the record).
