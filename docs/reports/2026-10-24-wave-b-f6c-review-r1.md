# Wave B F6c review, round 1 (2026-10-24)

Reviewer, on `git diff 1d48f96..d04dfcf` (bd1dcc4 tests, d04dfcf implementation), against plan 747ccb8 (F6c, §4.9, §14.45-§14.47). Filed by the controller. The change is off the contract list. The run results are the implementer's: 3146 passed, 0 failed, 3 ignored; the oracles green and unedited; eol-check clean.

**Verdict: PASS.** No Critical or Important findings.

## What git confirms
- Only `engine/src/ingest.rs` and the new `engine/tests/ingest_exclude.rs` changed.
- `oracle.rs`, `surface_oracle.rs`, the fixtures and every existing ingest test are unedited, including `a_uid_pin_beats_everything_and_is_case_sensitive`.
- Line endings are LF, and `diff --check` is clean.

## Checks
1. **No key, or `[]`, is byte-identical.** `is_left_out` returns false before it matches anything, so the journal, the seen ledger, the run lines and the summary are unchanged. A non-sequence value fails open.
2. **The pin bound matches §14.46.** No real uid loses a claim: Blackboard `GradableItem-_N_1` / `CalendarEntry-_N_1`, a grade note's bare `source_uid`, `blackboard:_N_1`, Canvas, Google, zyBooks, VHL, Gmail and `BB-ASSIGN-…@host` all still match. Only contrived hand-written keys lose a claim.
3. **A left-out item writes nothing.** The skip comes before every `record_seen`, create, update, archive and journal write. Gradebook done marks skip it too.
4. **A kept course always wins.** This is tested by title and by pin.
5. **Deterministic.**
6. **Synthetic data, and no single-user assumptions.**

## The implementer's rulings (both upheld)
- **`mark_gradebook_done` skips left-out events:** a done mark is a write on that item's account.
- **The exclusion check runs before the noise, no-date and known-uid checks:** it has to precede the known-uid backfill.

## Minor
1. **Seen ledger not asserted.** Assert the left-out uid is absent from the seen ledger, including on the backfill path (delete `state/ingest-seen.md` before the second run).
2. **Gradebook skip untested.** Add a test where an existing task of a removed course, joined to a turned-in grade row, stays `active`.
3. **Empty fragment.** An empty `course_exclude` fragment claims every uid. Filter empty fragments when reading.
4. **Summary counts.** `ingest: N events` still counts left-out events. Harmless.
5. **Stale doc comments.** `match_course` (`:312`) should mention the pin bound. The guard list in `sync_tasks` (`:756-763`) should list the exclusion step.

The pin bound changes the shared rule for every vault (§14.46 flagged this for the controller; recorded as G4). The unedited green oracles show that no frozen reference relied on the old prefix match.
