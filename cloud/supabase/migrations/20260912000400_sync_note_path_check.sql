-- Knowlu C3′, R-C3′-exec-10 — sync_notes' path check stays under Postgres's repetition cap.
--
-- Postgres's regex engine caps a bound repetition count (`{n}`, `{m,n}`, `{m,}`) at 255 (DUPMAX),
-- and raises 2201B "invalid regular expression: invalid repetition count(s)" the first time a ROW
-- is actually checked against a pattern that exceeds it — not when the migration that declares the
-- CHECK is applied. That is exactly how 20260912000300_sync_plaintext.sql's
-- `[A-Za-z0-9._ /-]{1,300}` bound sat on staging, invisible, until the controller's live smoke
-- actually inserted a note (found 2026-09-22): every `sync_notes` insert failed with that error.
--
-- **This supersedes only `sync_notes_path_check`, in place.** 20260912000300 is already applied to
-- staging and migrations are forward-only, so it is never edited. The two siblings it also put on
-- `path` — `sync_notes_path_check1` (`path !~ '(^|/)\.\.(/|$)'`) and `sync_notes_path_check2`
-- (`path !~ '//'`) — have no bound repetition, are fine, and stay untouched.
--
-- The fix: an unbounded character class (`+`, which Postgres has always allowed) in place of the
-- bounded one, plus a SEPARATE `char_length` check for the length `{1,300}` used to enforce inline.
-- `regexp_replace(path, '^[a-z]+/', '')` strips the leading folder (e.g. `tasks/`) and leaves
-- `<middle>.md`, so the 1-300 character middle plus the 3 characters of `.md` is a length between 4
-- and 303 — exactly what `NOTE_PATH_RE` in `cloud/supabase/functions/_shared/sync_rows.ts` states,
-- left unchanged there because JavaScript's regex engine has no such cap.
alter table public.sync_notes
  drop constraint sync_notes_path_check,
  add constraint sync_notes_path_check check (
    path ~ '^(tasks|approvals|archive|courses|issues|info)/[A-Za-z0-9._ /-]+\.md$'
    and char_length(regexp_replace(path, '^[a-z]+/', '')) between 4 and 303
  );
