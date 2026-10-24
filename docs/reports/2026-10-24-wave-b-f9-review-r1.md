# Wave B O3 F9 (the reveal gate): review r1 (2026-10-24)

Reviewer, on `git diff fa232ec..81b54a0` (`feat/o3-f9`; one file, `app/src/onboarding_gate.rs`). Filed by the controller. Nothing on the contract list; no test or fixture changed.

**Verdict: FAIL.** One Important finding.

## Checked and clean
- **Pure, deterministic, input order kept.** One pass over the list, with no clock and no I/O.
- **The 180 s cap.** `now >= c + 180` (`saturating_add`): second 179 stays closed, second 180 opens. With no confirm there is no cap.
- **Backfills never hold the gate.**
- **No single-user assumptions.**

## Important
**I1. A failed `grades` run marks first grades** (`onboarding_gate.rs:79-80`). The bare `grades` row is the engine child's step with its real exit code, and the engine exits 1 for a bad bundle.
- §4.6:972-973 says the gate is marked only by a slot whose `grades` ran. As §4.3 rules for `sync`, "ran and failed" is `failed`, never `done`.
- §4.6:982-983 says a failed item is named with its fix, never hidden.
- What the student sees today: first grades counts as done, the reveal opens with no grades and names nothing, and nothing retries.
- **Fix:** `name == "grades" && *code == 0`.
- A bare `grades` row with a non-zero exit should make `first.grades` `Failed { reason, fix }`. The plan defines no reason or fix for this case, so the planner assigns it.

## Minor
- **M1.** A failed item that is not connected disappears (`:62`): the `!it.connected` arm comes before `Failed`. Put `Failed` first, as `Skipped` already is.
- **M2.** `:40`'s doc comment should read: every required item still pending, plus every backfill in progress.
- **M3.** The engine can skip at exit 0 under the bare name `grades` (for example, the vault is busy), which marks first grades with nothing written. This is barely reachable in onboarding. The planner notes it, as §4.3 notes the same case for `sync`.

## The implementer's rulings
1. **Failed items in `failed` only:** upheld (F1 test 6a; consistent with §4.6:979).
2. **First grades ignores exit codes:** overturned (I1).
3. **A skip goes in `not_connected` whatever its `connected` flag:** upheld.
4. **An empty list opens the gate:** upheld. Caller tests (F2b) must check that a jobs read error returns an error, not an empty list.

## Controller's ruling
F9 keeps `first_grades_marked` (exit 0 only), adds `first_grades_failed(steps) -> bool` (a bare `grades` row with a non-zero exit) with tests, and folds in M1 and M2. The planner assigns the caller's `Failed { reason, fix }` mapping, plus M3's note.
