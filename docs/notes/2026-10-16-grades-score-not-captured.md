# Grades: Blackboard scores are not captured (diagnosed 2026-10-16, fixed)

**Status (2026-10-23):** fixed on `main` (lane L2: `column_to_fields` takes `displayGrade.score` for
`Score` and `CompleteIncomplete` rows, the top-level override first, `engine/src/grades.rs:190-203`,
pinned by inline `v2_row` tests). Confirmed live, counts only: of 69 graded item notes, 66 carry a
numeric score and 3 do not (columns that hold no score). The "unconfirmed" fact below is settled: the
v2 rows carry `displayGrade`. The original diagnosis follows unchanged.

## Symptom

On the proof profile (`proof-email-t18`) the grades capture wrote about 90 grade notes with
`status: graded` and no score, so every course's percentage was empty. Quinn confirmed in
Blackboard that those items do have scores: "The grades have scores, you're just not capturing
them."

## Cause

- `engine/src/grades.rs:190` (`column_to_fields`) reads only the row's top-level `score`.
- The app captures the row from Blackboard's **v2** endpoint,
  `v2/courses/{id}/gradebook/users/{user_id}` (`app/src/grades.rs:563`). In the v2 shape the
  top-level `score` is the instructor's override and is usually absent; the score a student sees
  is under `displayGrade.score` (with `displayGrade.scaleType`).
- With no `score`, a row whose `status` is `Graded` falls through to "graded, no score" (the arm
  meant for Complete/Incomplete and letter-only columns), which is the symptom exactly.

**Unconfirmed:** that the v2 rows really carry `displayGrade`. The capture bundle is deleted after
the engine runs (`app/src/grades.rs:723`), so there is no saved copy, and the check through Quinn's
debugging Chrome returned 401 (the Blackboard session had expired). Re-check with a live session:
fetch one `v2/courses/{id}/gradebook/users/{user_id}` and list the keys of one row.

## Proposed fix

- **If `displayGrade` is present:** in `column_to_fields`, prefer the top-level `score` (an
  override wins), else `displayGrade.score` when `displayGrade.scaleType == "Score"`. Test first:
  a regression test with an inline `json!` row in the v2 shape (no new file under
  `engine/tests/fixtures/`, which is frozen). Dispatch: `test-writer` then `implementer`, then
  `reviewer`. `grades.rs` is off the contract list.
- **If `displayGrade` is absent:** switch the grades call to the v1 endpoint
  (`v1/courses/{id}/gradebook/users/{user_id}`), whose rows carry `score` directly. That changes
  the capture's shape, so `planner` first.

## Known gap the fix will not close

UA's "Overall Grade" column is not flagged `externalGrade`, so Knowlu still won't show the
overall grade Blackboard shows. Separate issue.
