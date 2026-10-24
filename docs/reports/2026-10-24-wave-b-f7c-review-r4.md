# Wave B F7c review, round 4 (2026-10-24)

Reviewer, on `git diff 4c9ca48..b03d897` (`feat/o3-f7a`: 10cb4fa, b03d897), against the wave B plan at 747ccb8 (F7a, §14.47). Filed by the controller. Nothing on the contract list is touched.

**Verdict: PASS.** No Critical or Important findings.

## Round 3 items
- **Minor 1:** closed (10cb4fa). Only a missing `pairs` key re-derives. A non-array or `null` value, or an unreadable element, is a named `pairs unreadable` line. All four cases are tested.
- **Minor 2:** ruled by §14.47 item 2 (class times are never removed).

## The implementer's calls (all accepted)
1. **`graded` follows the `grades_courses` rule** (a course whose only rows are needs-grading reads `false`).
2. **Courses are matched by `membership.courseId` against the entry's `code`.** A course with an empty code reads `false` when a bundle is present, which agrees with ingest.
3. **The top-level `grades_courses` still counts bundle courses missing from `--courses`.** Accepted for the app, but it contradicts the plan's text; see Minor 1.

## §14.47 conformance
- Every keyed course has its own bucket. `graded` is true, false, or present as null. Multi-code buckets and `unmatched` carry no `graded`.
- For class times, only the test changed, and it follows the plan.

## Re-checks
Read-only, deterministic, LF-only, synthetic data, no single-user assumptions, and no regression of r1-r3: all hold.

## Minor
1. **The top-level `grades_courses` can differ from the bucket total** (`onboarding.rs:217-221`, `:337`). The plan (F7a, :2308-2309) says they are equal.
   - Example: a graded bundle course that was left out of `--courses` makes the top level 2 while the buckets sum to 1.
   - **Controller's call:** when `--courses` is given, compute the top level as the count of `graded: true` buckets. Without `--courses`, the output is unchanged. Fold this in before merge.
