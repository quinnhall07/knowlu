# Wave B O3 F6a (school files and course notes): review r1 (2026-10-24)

The reviewer checked `git diff 0370c31..7d8eb2f`; the controller filed this report. No file on the contract list was touched, no frozen fixture, and no handler list. The lockfile is unchanged.

**Verdict: FAIL.** Two Important findings.

## The questions
- **(a) Notes go through `write::create` with `console_ctx`.** The journal entry comes first, the actor is the student, and nothing is parsed and re-dumped.
- **(b) The helper never replaces a file.** `CreateHardLinkW`, and `MoveFileExW` with flags 0, both refuse an existing target. The error mapping is right, and the wide strings are null-terminated.
- **(c) The seed text is byte-identical.** It is a pure move into `course_note_text`.
- **(d) Slugs are recorded through `update_answers`,** which holds `ANSWERS_LOCK`.
- **(e) No single-user assumptions.**

## Rulings
All six are upheld. Ruling 6 (the serial-lane sweep) depends on F6b staying on the local lane.

## Important
1. **A slug collision fails the first run, then merges two courses on the retry** (`onboarding_jobs.rs:401-404`).
   - Example: two rows with no code and the same name. The second `create` hits `Exists`, so the job fails. On Try again, the second row quietly shares the first row's note.
   - **Controller's ruling:** keep today's seed rule (`seeded_courses`, `scaffold.rs:431-444`): one note per slug, first wins. Keep a slug map within the run so the second row reuses the note on the first run, with no error. Record the shared slug for both rows, update the docstring, and add a test. A suffixed slug would break `lms_link`'s slug rule (`lms_link.rs:520-525`), which F6b's `course_map` and capture share.
2. **The temp file is linked into place without being flushed** (`scaffold.rs:914`). After a power loss, a zero-length `campus.yaml` or `events.yaml` survives and is never repaired. Fix: `File::create` and `write_all`, then `sync_all()`, then link.

## Minor (fold in)
1. **Clean the slug before using it in a path** (`scaffold.rs:954`). Run the row's own slug through `slugify`, or reject separators, so `../tasks/x` cannot place a note under `tasks/`.
2. **Match in a fixed order** (`onboarding_jobs.rs:396-400`): `source_uid` first, then `code`, then slug.
3. **Use the platform newline constant.** The preset-none pin should use `pystr::NEWLINE` (or the crate's equivalent), not a literal `"\r\n"`.
4. **Plan text:** the temp name is `.knowlu-tmp-<name>-<pid>`. §4.9 should say so; that goes to the planner.
