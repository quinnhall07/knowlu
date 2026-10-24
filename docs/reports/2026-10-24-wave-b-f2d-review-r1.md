# Wave B O3 F2d (the answers file): review r1 (2026-10-24)

Reviewer, on `git diff 7191751..0fa8c43` (`app/src/onboarding_state.rs` only, +77/−12). Filed by the controller. Nothing on the contract list. No single-user assumptions. LF line endings, and `diff --check` is clean.

**Verdict: FAIL.** One Important finding.

## The questions, all upheld
- **(a) The totals.**
  - `class_times` counts every bucket, removed ones included (§14.47.2, G5).
  - Feed counts are kept-first (§14.45).
  - `grades_courses` follows §14.47.1.
  - The result is deterministic.
- **(b) The credentials.**
  - The names are per-profile: `knowlu/<profile_id>/pending-*`.
  - No secret reaches an error.
  - The read goes through `wincred::read_credential`, as `account.rs:129` does.
- **(c) The resume rule** composes with §14.49 and test_02's ladder.
- **(d) A recapture keeps the saved pairs** for a row with the same key (R-C1-51, §14.46.3).

## Important
1. **`update_answers` can lose an update** (`:397-402`). Two overlapping load, change and save calls can overwrite each other.
   - Example: Remove on step 3 races F8b's `save_preview`, and the removal is lost. KNT's items are then imported.
   - Fix: a file-scoped `ANSWERS_LOCK` held across the whole update, as `mail_session.rs:40` does. F8b's `save_preview` and `save_pairs` go through `update_answers`.
2. **`advance_after_school` skips the re-ask** (`:509-515`, F2a's function, with `:532`). When the answers file is unreadable, a confirmed school jumps past step 3 with no capture.
   - The file then stays unreadable, and every later save fails.
   - Ruled by the controller, in F2d (same file, a few lines): `advance_after_school` does not advance past 3 while `answers_ok()` is false.
   - F2b's step-3 capture replaces an unreadable answers file instead of updating it. That note goes to F2b's brief.

## Minor
1. **A serde error can quote file content** (`:389`). Map it to its category, line and column only, so no course label reaches a log.
2. **A failed pending delete is dropped** (`:428`). The `PendingStore::delete` signature is F1/F2a's. This goes to F2e as a follow-up: `on_sign_out` must report it.
3. **Untested case** (`:533`): no session and `answers_ok = false`. Add one case.
4. **Counts are null, not 0, when every course is removed and there is no `unmatched` bucket** (`:330`). This is a note only. Today's engine always writes `unmatched`.

## Controller check of the r1 fixes (ff19cd2)
- **I-1 closed.** `ANSWERS_LOCK` is held across load, change and save. The new 16-thread test fails when the lock is removed.
- **I-2 closed.** `advance_after_school` returns early while `!answers_ok()`, and a new test covers it.
- **M1 closed.** The error now gives the category, line and column only, and a test checks it never quotes the file.
- **M3 closed.** The new case is tested.
- **M2** goes to F2e.
- Verified from the diff. **F2d passes.**
