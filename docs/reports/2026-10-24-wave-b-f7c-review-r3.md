# Wave B F7c review, round 3 (2026-10-24)

Reviewer, on `git diff 904a92c..4c9ca48` (`feat/o3-f7a`: a753e64, 4c9ca48), against the wave B plan at 1d48f96. Filed by the controller. Nothing on the contract list is touched.

**Verdict: PASS.** No Critical or Important findings.

## Round 2 items
- **Important 1:** closed. Class times are matched against every course that carries the code (`courses_with_code`, `onboarding.rs:121`) and go in the same match-set bucket as feed items. The test with two shells and one meeting covers it.
- **Minor 1:** closed on the engine side by `pairs` in `--courses`. The byte-equal check stays with F8b and F1 test 12.
- **Minor 2:** closed (`record_with_id`).

## The implementer's calls (all accepted)
- **A pair as a string or `[fragment, slug]`, with the slug ignored:** the bucket key belongs to the course entry.
- **A present `pairs` array, even an empty one, is the whole list:** `course_fragments` gives `[]` only for a course with no LMS code, and ingest attributes nothing to such a course. Its items land in `unmatched`, which every total counts. Falling back would claim items that ingest leaves course-less. The guard belongs to the app.
- **With pairs, class times match only on those fragments:** the human code compacts to the registrar's code.
- **With pairs, the bundle adds no pin:** required, since ingest reads only `course_map`.

## Re-checks
Read-only, determinism, LF, synthetic data, no single-user assumptions, and no regression of r1 or r2: all hold.

## Minor
1. **A malformed `pairs` silently re-derives from the label** (`onboarding.rs:110-116`). A non-array `pairs` falls back to the label, and an element that is neither a string nor an array is dropped, both with no error.
   - Fix: push a named `courses: <key>: pairs unreadable` line into `errors`. Keep the fallback only for an entry with no `pairs` key. Add a test. Fold in before merge.
2. **For the planner:** with pairs, class times no longer match on the slug, but `commitments::code_table` still does (`commitments.rs:762-764`). A class time whose code only the slug carries lands in `unmatched` in the proof, while the vault ties it to the course. Rule on whether removing a course drops its class times, at F8b or F6b.
