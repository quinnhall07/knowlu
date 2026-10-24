# Wave B F1 review, round 4 (2026-10-24)

Reviewer, pre-build, on `git diff ae504be..f7dca4c` (2ef92db, f7dca4c). Checked against the plan at a22d07b (§14.45-§14.47) and the engine contract `feat/o3-f7a` at b03d897. Filed by the controller.

**Verdict: FAIL.** All r3 items are closed. Two new Important findings. No contract-list file is in the diff.

## r3 items: closed
- **I1, the LMS-key identity:** closed.
- **I2, nullable counts:** closed in `Bucket.counts`, `JobRecord.proof` and `Outcome::Done`.
- **I3, the `save_preview` / `save_pairs` seams:** closed. `{"buckets"}` and `{"pairs"}` are refused.
- **The r3 Minors:** closed.
- **Earlier rounds:** no regression.

## Important
1. **A label edit can change a row's key** (`app/src/onboarding_state.rs:152-156`). On the paste fallback, a row has no `lms_key` and no slug, so `key()` is the label, which the student can edit (§14.46.3 says the label is display only).
   - Failure: the student renames "Lab" to "Lab, Tues". No bucket names the new key, so the proof drops 10 items, and Remove and re-capture both miss the row.
   - Fix: build the key only from fields the student can't edit. Freeze the label as it was at capture (`captured_label`).
   - Test: a row with no LMS key and no slug, with a saved bucket. Edit the label, then check that `key()`, the totals and Remove are all unchanged.
2. **Test 14 over-reaches** (`onboarding_jobs.rs:1792`). It requires a separate note and slug for each kept shell. The plan doesn't support that: F6a matches by path, `source_uid` or code; the scaffold seeds one note per slug; and §14.15/I5 forbids duplicate course notes.
   - Fix: pin one create, for the first kept shell in captured order. Assert `slugs["_0990_1"] == slugs["_1002_1"]`, and keep the check that there is no `"KNT 220"` key.

## Minor: fix before F2
- **`save_preview` replacement is unpinned.** It must replace the saved buckets, never merge with them. Test: save twice, and only the second set remains.
- **`step3_totals` with no buckets** (preview not run, or failed): pin null, never 0.
- **`save_preview` given `null` or a non-object:** pin an `Err` that changes no byte of the file.

## Minor: ride along with F2
- **The pairs check reads `preview_courses(&now)`,** not the `--courses` the fake `school.preview` runner received (plan :1774-1775). Record the fake's argument and compare that.
- **`key()` doesn't trim, but the engine does.** Say that the key is trimmed, and test it.
- **A row with no human code and no fragments:** say that `[]` is allowed there, and fix the "non-empty" assertion at `:1250`.

## Rulings
- **Page captures drop fragments:** accepted (§14.46.3).
- **A human-code fallback for `pairs`:** accepted (it equals `IngestHandoff`).
- **`Outcome::Done` with `Option<i64>`:** accepted.
- **The two-shell slug test:** over-reaches (Important 2).

## The engine contract, field by field: agrees
`--courses`, bucket names and `codes`, nullable counts, and `graded` on single-code buckets only.

## Safety: pass
