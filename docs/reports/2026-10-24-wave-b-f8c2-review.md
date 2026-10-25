# Wave B F8c2 review: config.ingest's handoff translates a book row's course key

Reviewer: `reviewer` (read-only; the controller filed this report). Diff `56e3ea7..b449110`.
**Verdict: PASS**, with three Minor findings. No contract-list file and no frozen fixture is touched,
`git diff --check` is clean, and there are no single-user literals.

## Checked
- **Translation.** One path writes `answers.rows`: the `(8, "row")` arm, which accepts only a kept
  course's key. `ingest_handoff` keeps only rows for kept courses, translates each through
  `answers.slugs` (trimmed, as `course_map` does) and drops a missing or blank slug (§14.57.3 / I-4).
  No release contains `BookRow`, so no answers file in the wild holds slug-shaped values.
- **Consumers.** Only `ingest_handoff` and `ingest_base_for` read `rows`. The ingest writer reads
  `h.books`, which now holds slugs.
- **Determinism.** `books` keeps `answers.rows`' order, and the `BTreeSet` is used only for
  membership.
- **Fixtures.** Each still tests what it did before. The f6b change makes its test stronger, because
  it now goes through the translation.
- **Silent drops.** A kept course with no slug fails earlier (`REASON_COURSES_NOT_RECORDED`). A row
  for a removed course is dropped here, and `coursework` asks about the unmapped book later
  (R-OB-1).

## Minor, and what the controller did
- **m-1:** `ingest_base_for`'s comment claimed the flag mirrors `config.ingest`'s mappings, but it
  follows any stored row. *Fixed:* the comment now says on purpose that the flag follows the stored
  login, and that a dropped row is asked about by `coursework` later.
- **m-2:** test 42 had no blank-slug row. *Fixed:* added kept course `_1004_1` with slug `"  "` and
  row ZYB-C; test 42 still passes.
- **m-3:** test and code landed in one commit, so there is no red run on record. Noted for the
  ledger.
