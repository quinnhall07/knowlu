-- Knowlu M1, grades from Blackboard (spec docs/specs/2026-09-29-grades-design.md §7) — `grades/`
-- becomes a note folder the account's copy can hold.
--
-- The engine's `ids::NOTE_FOLDERS` gains `grades` (one note per graded item). A path the server
-- refuses wedges every push (the batch is all-or-nothing, R-C3′-exec-12), so the column check has to
-- accept the folder before any release that writes one ships.
--
-- **Why `commitments` is here too.** The commitment model's phase 1 (the `p1-commitments` branch)
-- adds `commitments/` as a note folder on its own branch, with its own migration
-- (20260926000100_sync_note_path_check_commitments.sql), which merged to main before this one. Each of
-- the two migrations drops and re-adds this same constraint, so whichever is applied LAST decides
-- which folders the account accepts. Listing both here makes this constraint right whichever branch
-- merges first; the group is written in `NOTE_FOLDERS` order as it reads once both have merged
-- (`…, info, commitments, grades`). Because this file sorts after p1's, a database that applied this
-- one first and p1's later (`db push --include-all`) would end on p1's group, without `grades`: the
-- branch that merges second checks the database and, if needed, adds a newer forward-only migration
-- restating the union. Neither is applied to production before both are merged.
--
-- **This supersedes only `sync_notes_path_check`, in place**, exactly as
-- 20260912000400_sync_note_path_check.sql declared it — the unbounded character class plus the
-- separate `char_length` check (Postgres caps a bound repetition count at 255) — with the folder
-- group widened. Forward-only: no earlier migration is edited, and the two climb-out siblings
-- (`sync_notes_path_check1`, `sync_notes_path_check2`) stay untouched. `NOTE_PATH_RE` in
-- cloud/supabase/functions/_shared/sync_rows.ts carries the same group.
alter table public.sync_notes
  drop constraint sync_notes_path_check,
  add constraint sync_notes_path_check check (
    path ~ '^(tasks|approvals|archive|courses|issues|info|commitments|grades)/[A-Za-z0-9._ /-]+\.md$'
    and char_length(regexp_replace(path, '^[a-z]+/', '')) between 4 and 303
  );
