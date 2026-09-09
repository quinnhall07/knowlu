# Knowlu go-live (after G2) — the switch to the live vault

Preconditions, all true before step 1:
- [ ] G2 given and cutover Task 8's commit is on main. **Amended 2026-09-09:** this used to say
      *check `$mode = "rust-live"` in scripts/local-run.ps1*. That variable no longer exists
      — cutover Task 9 step 2 (`c4e5707`) took the whole dual-run harness out of the script.
      The check is now stronger and simpler: `scripts/local-run.ps1` runs `& $rust coursework`
      and `& $rust rank`, and `-DryRun` prints `pull --rebase, cargo build --release (engine),
      Rust coursework, Rust rank --runner local, add, commit, push, cargo build --release
      (console)`.
- [ ] `cd app; cargo test` green; root `cargo test` green.
- [ ] `scripts\diff-engines-notes.ps1` clean at HEAD. **Amended 2026-09-09: `diff-engines.ps1`
      is NO LONGER a precondition, and running it here will report a difference that is not a
      failure.** It compares run records, and plan 2 Task 7 added `runway_days`,
      `deficit_hours_x10` and `must_count` to the `tasks` step; `engine/cli.py:235` writes only
      `active` and `unreadable` and never will. Past the cutover gate Python is no longer the
      reference for what `rank` *records* — the standing check is `cargo test` with
      `tests/oracle.rs` and `tests/surface_oracle.rs`, which is what plan 2's Global Constraints
      already say. `diff-engines-notes.ps1` is unaffected: it never runs `rank`.
- [ ] The last three scratch-vault sessions of Knowlu show no `refused:` lines that were not intended and the journal shows every edit as quinn/dashboard.
- [ ] A backup folder exists on the laptop (OneDrive or an external drive) and `back up now` against a scratch vault filled it.
- [ ] That folder is **private storage of your own**. The backup is a whole-vault mirror, so it carries `config/ingest.yaml` — a live Blackboard token and a secret Google Calendar capability URL — until the profiles milestone moves secrets out of the vault. Never a shared drive, a shared folder, or anyone else's machine.

Steps:
1. Build: `cd app; cargo build --release` in the MAIN checkout (not a worktree). Confirm `app\target\release\knowlu.exe` and `target\release\quinn-ops.exe` are from the same HEAD (`knowlu.exe` topline `build` == `git rev-parse --short HEAD`).
2. Update the Start-menu shortcut `quinn-ops.lnk` → rename to `Knowlu.lnk`; target `app\target\release\knowlu.exe`, args `--vault "C:\Users\danie\GitHub\quinn-ops"`, icon `app\icons\icon.ico`.
3. Launch from the shortcut, then quit from the tray. **Set the backup folder in the settings panel** (the topline gear, or *Settings* in the tray → *Backup folder* → *Choose…*) — since Knowlu plan 4a there is a UI for it, and no file to hand-edit. It is written to `%LOCALAPPDATA%\knowlu\profiles\<profile_id>\settings.json`, which is that profile's own app data (`settings.json`, `seen.txt`, `logs\`); the flat `%LOCALAPPDATA%\quinn-ops\settings.json` an older build wrote is folded into it on the first launch.

   Then press `back up now` in the sync line and confirm `<folder>\<profile>\vault\tasks` exists. If the folder did not take, `back up now` answers with a refusal toast — `no backup folder set — pick one in settings` — and nothing is written; re-open the panel and check what the row shows.
4. Make one edit (an importance) and wait 30 s: the sync line goes `1 commit pending push` → `synced`. Confirm on GitHub that a `surface: 1 edit` commit landed on main.
5. Close the window (it hides). Confirm the tray icon. Quit from the tray. Relaunch — no missed-slot line (the script runner still owns slots).
6. Stop Obsidian Git: in Obsidian, disable the plugin's auto-commit and auto-push. Do NOT uninstall Obsidian yet — it is the rollback viewer for the reverse week.
7. Record the switch in docs/HANDOFF.md and state/dual-run-log.md's neighbour, `state/knowlu-log.md`: `- <date> knowlu live vault=<path> build=<sha>`.

Rollback (any step fails): point the shortcut back at the previous exe, re-enable Obsidian Git. The vault has only ever been written through `write`; nothing needs undoing.

What does NOT happen here: `scheduler: app` (phase 2, after the harness comes out ~09-18); renaming anything the engine, the runner or the routine reads; removing the remote.
