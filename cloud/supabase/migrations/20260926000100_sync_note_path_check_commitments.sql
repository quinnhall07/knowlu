-- Knowlu commitment model, phase 1s (spec docs/specs/2026-09-23-commitment-model-design.md §10) —
-- `commitments/` becomes a note folder the account's copy can hold.
--
-- The engine's `ids::NOTE_FOLDERS` grew to seven with `commitments` (confirmed commitments, decline
-- markers and the one planning-day note). A path the server refuses wedges every push (the batch is
-- all-or-nothing, R-C3′-exec-12), so the column check has to accept the folder before any release
-- that writes one ships (R3). The commitment CARDS never reach this table: the engine keeps every
-- card of a `commitments::LOCAL_CARD_KINDS` kind, and every record about one, on the device.
--
-- **This supersedes only `sync_notes_path_check`, in place**, exactly as
-- 20260912000400_sync_note_path_check.sql declared it — the unbounded character class plus the
-- separate `char_length` check (Postgres caps a bound repetition count at 255) — with `commitments`
-- added to the folder group in `NOTE_FOLDERS` order. Forward-only: no earlier migration is edited,
-- and the two climb-out siblings (`sync_notes_path_check1`, `sync_notes_path_check2`) stay untouched.
-- `NOTE_PATH_RE` in cloud/supabase/functions/_shared/sync_rows.ts carries the same group.
alter table public.sync_notes
  drop constraint sync_notes_path_check,
  add constraint sync_notes_path_check check (
    path ~ '^(tasks|approvals|archive|courses|issues|info|commitments)/[A-Za-z0-9._ /-]+\.md$'
    and char_length(regexp_replace(path, '^[a-z]+/', '')) between 4 and 303
  );
