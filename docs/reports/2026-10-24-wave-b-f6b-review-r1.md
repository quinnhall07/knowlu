# Wave B O3 F6b (config.ingest): review r1 (2026-10-24)

The reviewer checked `git diff 4346a2f..d3fa42e` (tests 2548d0b, code d3fa42e). The controller filed this report. No file on the contract list is touched.

**Verdict: FAIL.** Two Important findings.

## The questions
- **(a) The bytes are unchanged** for today's wizard. `ingest_yaml(p) == ingest_yaml_for(p, !account_id.is_empty(), [])`, and `restore_capability_url` keeps its checks and its three error texts.
- **(b) No link reaches a log, an error or the answers file.** The scratch root is deleted on every return path; Minor 1 covers the panic path.
- **(c) `course_exclude`** puts kept fragments first, so shared fragments are never excluded. Duplicates are dropped, the order is deterministic, and it is written as one flow line.
- **(d) The check reads the fresh scratch text.** The malformed-text test proves it.
- **(e) No single-user assumptions.**

## Important
1. **A kept link is silently lost** (`:509-511`, `:531`). `CredPending::get` returns `None` on a read error as well as on absence. The file is written once, F8c then deletes the pending link, and the feed never imports. That breaks §4.11 "never silently lost" and §4.6.
   - **Controller's ruling:** `Failed` with a per-feed key (`lms_link_missing` / `calendar_link_missing`). Create nothing and leave the pending credential alone.
   - **The fix the student sees:** Try again. If it fails again, they paste the link again at step 3 or step 7, which takes the feed back to `validated` and re-runs the store.
   - F8c owns that rewind; it is noted for F8c's brief. Test: state `failed` with no pending credential → `Failed`, no file, the key.
2. **A `validated` feed is guarded only by F8c.** The write cannot be undone.
   - Fix: a guard beside the slug guard (`:664`). Any `Validated` feed → `Waiting("feeds_not_settled")`. Add a test.

## Minor (fold in)
1. **A panic leaves the scratch file holding the link.** Add a `Drop` guard on the scratch root.
2. **Set the zybooks/vhl flags from the step-8 rows** ("login stored or any row for that source"). This is a note for F8c.
3. **A row with a key but a blank slug is dropped.** Step 8 should never save one. Add a debug assertion or a doc note.
4. **Missing tests:**
   - a `stored` feed whose pending credential is still present;
   - rulings 2 and 3 (the guard and the failure).
5. **`IngestHandoff` derives `Debug` while it holds the link.** Give it a redacting `Debug`.

## Rulings
- **1, 4, 5 and 6:** agreed.
- **2:** agreed, with the guard from Important 2.
- **3:** rejected (Important 1).

## Controller check of the r1 fixes (4de4184)
I-1 closed (per-feed `*_link_missing` Failed, no file, credential untouched; 4 cases tested); I-2 closed (`Waiting(feeds_not_settled)`, tested); M1 (ScratchRoot Drop guard; under release panic=abort it is moot, under unwinding it holds), M3, M4, M5 (redacting Debug, tested) closed. onboarding 81/81, scaffold 32/32. **F6b passes.** F8c notes: M2 (login flags from rows), the step 3/7 re-paste rewind for `*_link_missing`.
