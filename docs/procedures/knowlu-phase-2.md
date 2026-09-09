# Knowlu phase 2 — the runner leaves (after the harness is out)

Preconditions, all true before step 1:
- [ ] Cutover plan Task 9 step 2 is on main: no snapshot, no harness call, no `state/dual-run-log.md`; `scripts/local-run.ps1` runs pull → build → Rust coursework → Rust rank → push.
- [ ] Knowlu has been on the live vault since the go-live checklist (`docs/runners/knowlu-go-live.md`, step 7's line in `state/knowlu-log.md`) for at least three days with `synced` after every edit and no `refused:` line that was not intended.
- [ ] `cd app; cargo test` and root `cargo test` green at HEAD; the exe on the Start-menu shortcut is built from HEAD (tray → **Copy diagnostics**, whose first line reads `Knowlu <version> build <sha>` — `topline.console_build` in the payload, which the page itself never prints — and that `<sha>` == `git rev-parse --short HEAD`).
- [ ] The laptop is the machine named by `device:` in `config/runners.yaml` (it is; the key does not change).
- [ ] Nobody runs `--run-slot-once` on the live vault, before or after: the first app-run slot is the scheduled one.

Steps (Knowlu plan 2, Task 9 — the controller performs 2–3 with Quinn present; 4–6 are watching):
1. Confirm the next slot is at least 30 minutes away (`Get-Date`; slots 12:00 and 18:00 America/Chicago). Do not switch inside a slot's grace window.
2. `Unregister-ScheduledTask -TaskName quinn-ops-local-runner -Confirm:$false` on this laptop. `Get-ScheduledTask quinn-ops-local-runner` must now error. The script file stays until Task 10.
3. One commit on main: `config/runners.yaml` gains `scheduler: app` under the `local` entry (beside `device:`), plus a HANDOFF line `phase 2: scheduler: app since <date time> (commit <sha>)`. Push.
4. Within two minutes Knowlu's sync line reads `scheduler on` (the housekeeping pass re-reads the key). If it reads nothing, the app is not on HEAD's vault or the key is misspelled — fix before the slot.
5. Watch the slot fire on the clock: the tray shows running, then ok; the Runs view shows `local` and `coursework` runs with full summaries; `state/runs/<day>.jsonl` has both; `runner-log.md` gained the engine's own lines (the app writes none). **Do not act inside the 20-minute grace.** A `late` slot is on time for this purpose.
6. The next morning: two more slots seen, `synced` after each, backup `backed up` (the slot's backup tick). Then Task 10 removes the script.

Rollback (any step fails, or a slot is missed twice): revert the Task 9 commit (`scheduler: app` gone → the app is inert within 60 s), then `scripts\cutover\register-local-runner.ps1` on this laptop. Nothing else moves: the vault was written only by the engine exe either way.

What does NOT happen here: any rename (Tasks 11–12), removing the remote, touching the routine, `Disable`/`Enable` of the desktop's task (already stood down by the device guard; disable it when the desktop is next on).
