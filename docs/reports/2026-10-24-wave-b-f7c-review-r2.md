# Wave B F7c review, round 2 (2026-10-24)

Reviewer, on `git diff 6a64eea..904a92c` (`feat/o3-f7a`) against the wave B plan at d69e79c (F7a, F7c, §14.45). Filed by the controller. Nothing on the contract list is touched, and no frozen reference changed.

**Verdict: FAIL.** One Important finding, no Critical.

## Round 1 items
- **I1, I3, I4:** closed.
- **I2:** half closed. Feed items are keyed by code, then slug, then label; class times are not (see below).
- **Minors 1-4:** closed.

## Important
1. **`class_times` still go to the first course in the list** (`onboarding.rs:208`, `:239`). `course_with_code` takes the first course whose codes contain the registrar code.
   - Failure: an old shell and the live shell both carry "SYN 110", and the old shell is listed first. The meeting lands in the old shell's bucket. The student removes that shell, and the kept total shows 0 class times, though the vault still has the class.
   - Fix: bucket class times by match set, like feed items, and add a test with two shells and one meeting.

## The implementer's calls
- **Bucket name and `codes`:** accepted. The app totals by `codes`, never by parsing the name, and `codes` can hold a slug or label.
- **The `(code, key)` and `(label, key)` pairs:** faithful for captured lists, because `lms_link` derives `label` by the same chain as `course_fragments` (see Minor 1).
- **Gradebook uid pins first:** accepted.
- **`SYN-110-003` no longer matches `SYN 110`:** faithful. Ingest leaves that item course-less too, so the proof agrees with the vault.
- **The `inferred` create rule counting coursework:** right. zyBooks' `inferred` effort is Knowlu's own estimate; VHL's `vendor` effort is not counted.
- **The test helper's journal ids:** correct, and the id join is now really tested.

## Re-checks
Read-only, determinism, line endings, synthetic data only, no single-user assumptions: all hold.

## Minor
1. A step-3 label that was typed or edited differs from the fragment `course_fragments` writes. Have F8b pass each course's saved pairs, or test that `lms_link`'s label equals that fragment.
2. The `note_id_of` fallback gives every unknown path the same id. Pass the id explicitly in `reused`'s record.

## Outside this diff (for the planner)
- **A fragment two courses share, when one is removed.** F6b must exclude only fragments that no kept course also contributes. Otherwise F6c drops an item the proof counts under a kept course.
- **The uid-pin form is undefined.** F6b and F8b need to define it. Ingest's uid pass is a substring test in list order, so `-_71_1` also matches `-_71_10`.
