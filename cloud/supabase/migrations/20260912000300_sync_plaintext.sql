-- Knowlu C3′, Task 1 — the account's copy of its own vault, in plain text.
--
-- **This supersedes the shape 20260912000100 created, and does not edit it.** That migration is
-- applied to staging and migrations are forward-only; Quinn's amendment of 2026-09-17 (ruling 2)
-- reversed §11 R4, so the rows that were AES-256-GCM ciphertext under a device-held key become the
-- record's canonical JSON and the note's own text, readable by this service, encrypted at rest by
-- the platform, and deleted with the account. The tables held nothing — `/sync-push` was never
-- built — so they are dropped and recreated rather than altered column by column into a shape with
-- a different primary key.
--
-- What ruling 2 buys, and it is worth naming because it is the whole reason the reversal is not
-- only a loss: the student's desktops can actually stay in step (a promise the product could not
-- keep across their own machines is not trust); `keep` stops being a bit the client asserts and
-- becomes a fact the server computes; a pushed row can be checked against its own hash instead of
-- taken on faith; and nothing is lost when a laptop and a printed code are lost together.
--
-- What is NOT here, because 20260912000100/000200 already put it there and it is unchanged:
-- `public.sync_usage` and its O(1) counter, `public.sync_ceiling_bytes()` (200 MiB, precondition P3
-- as Quinn answered it on 2026-09-17), `public.sync_limits`, `public.sync_prune(int)` (400 days,
-- `and not keep`) and the `knowlu-sync-prune` cron job. `sync_prune` is plpgsql and binds its table
-- late, so it goes on working over the tables recreated below.

-- The device key is gone (ruling 2), and so is the generation its fingerprint named.
drop table if exists public.sync_generation;

-- `drop table` fires no row triggers, so the byte counter is not decremented by the two drops
-- below. On staging both tables are empty — the endpoints did not exist — and on any project this
-- keeps the counter honest rather than leaving the ceiling guarding bytes that are not there.
drop table if exists public.sync_records;
drop table if exists public.sync_notes;
update public.sync_usage set bytes = 0, updated_at = now();

-- The sequence survives both drops (it is standalone, never `owned by`), so a second desktop's
-- cursor cannot be handed a `rev` it has already seen.

create table public.sync_records (
  account_id  uuid        not null references public.accounts (id) on delete cascade,
  seq         bigint      generated always as identity,
  -- Opaque, and it is the only thing here that is. `sha256(account_id + "\n" + hostname)[..16]`,
  -- computed on the device: the server needs to know that two rows came from the same machine and
  -- has no reason ever to learn which machine that is.
  device      text        not null check (device ~ '^[0-9a-f]{16}$'),
  -- `sha256` of `body`, hex. The SERVER re-derives it and refuses a mismatch (`sync-push`), so this
  -- column is a fact about the row rather than a claim about it — which is what makes
  -- `sync_records_once` an honest idempotence key for a retried batch.
  record_hash text        not null check (record_hash ~ '^[0-9a-f]{64}$'),
  -- **The journal record, exactly as `ledger::dumps_value` wrote it**: sorted keys, Python's
  -- separators, no trailing newline. `text` and not `jsonb` on purpose — jsonb renormalises numbers
  -- and would hand back bytes that no longer hash to `record_hash`, and the canonical bytes are the
  -- contract two machines agree on. 16 KiB is far past any record that exists (a record is a handful
  -- of scalars and, for a `create`, one frontmatter mapping); `octet_length`, not `length`, because
  -- the device's cap is in bytes and Postgres's `length` counts characters.
  body        text        not null check (octet_length(body) between 2 and 16384),
  -- **Decided here, from the record, never sent.** `sync_prune` keeps a record a human wrote for
  -- ever (precondition P3, Quinn 2026-09-17): `journal::human_set` is what judge-once reads and it
  -- reads records, so a pruned human `set` on a restored machine is a decision the student made and
  -- the device can no longer see. `not like 'agent:%'` is `provenance::is_agent`'s own test.
  -- `coalesce` on the whole expression, not only on `actor` (review M5): `->>` answers NULL for an
  -- absent key, `NULL in (...)` is NULL, and a NOT NULL generated column over a NULL expression is a
  -- 23502 from PostgREST rather than the 400 `sync_rows.ts` would have given. The validator refuses a
  -- body with no `op` or no `actor` before it ever gets here, so this is the backstop being total
  -- rather than a second guard. **What is deliberately NOT guarded here is JSON validity**: a
  -- non-JSON `body` raises 22P02 on the cast, and the validator is the only thing standing between a
  -- client and that 5xx — said out loud because it is the one place this schema trusts the function.
  keep        boolean     not null generated always as (
                            coalesce(
                              (body::jsonb ->> 'op') in ('set', 'create')
                              and coalesce(body::jsonb ->> 'actor', '') not like 'agent:%',
                              false)
                          ) stored,
  received_at timestamptz not null default now(),
  primary key (account_id, seq),
  constraint sync_records_once unique (account_id, record_hash)
);
comment on table public.sync_records is
  'Spec §5.5 as amended 2026-09-17. One row per journal record, in the canonical JSON the device
   wrote. Readable by this service, encrypted at rest, deleted with the account.';

create table public.sync_notes (
  account_id uuid        not null references public.accounts (id) on delete cascade,
  -- **The note''s vault-relative path, POSIX-separated, and it is the primary key.** A path a client
  -- sends is a path a restore would write, so it is checked rather than trusted: one of the six
  -- note folders, markdown, and no segment that can climb out of the vault. `ids::NOTE_FOLDERS` is
  -- the list, and `engine/src/sync.rs::is_note_path` is the same rule on the device.
  path       text        not null
                         check (path ~ '^(tasks|approvals|archive|courses|issues|info)/[A-Za-z0-9._ /-]{1,300}\.md$')
                         check (path !~ '(^|/)\.\.(/|$)')
                         check (path !~ '//'),
  rev        bigint      not null default nextval('public.sync_notes_rev'),
  device     text        not null check (device ~ '^[0-9a-f]{16}$'),
  -- A note that has been settled into `archive/` is pushed as a tombstone at its old path: the row
  -- stays, so a restore knows not to resurrect it, and carries no bytes.
  deleted    boolean     not null default false,
  -- The note''s whole text, frontmatter and body, as the file holds it. 128 KiB in bytes: a note
  -- longer than that is a pasted document, not a task, and the device refuses it by name rather
  -- than truncating it — a truncated note is a lie.
  body       text        null check (body is null or octet_length(body) between 1 and 131072),
  updated_at timestamptz not null default now(),
  primary key (account_id, path),
  constraint sync_notes_tombstone check ((deleted and body is null) or (not deleted and body is not null))
);
comment on table public.sync_notes is
  'Spec §5.5 as amended 2026-09-17, "the note text they produced". Current state, not history: one
   row per note path, upserted.';

-- A note''s row is UPSERTED — a note is current state, not history — so `generated always as
-- identity` would not move on a re-push and a second desktop would never learn the text changed.
-- The sequence plus this trigger gives every write, insert or update, a fresh cursor value.
create trigger sync_notes_rev_stamp
  before insert or update on public.sync_notes
  for each row execute function public.sync_notes_stamp_rev();

create index sync_notes_rev_idx on public.sync_notes (account_id, rev);
-- The nightly prune scans by exactly this predicate; without a matching partial index that scan is
-- a full table scan across every account, every night, forever (20260912000200's item 2, re-made
-- because the table it was on is gone).
create index sync_records_prune_idx on public.sync_records (received_at) where not keep;

-- `sync_usage_bump` is 20260912000200's fixed body and reads `new.ciphertext`/`old.ciphertext`,
-- which no longer exist. Same shape, same DELETE rule (it must never INSERT on a delete, or an
-- `accounts` cascade can abort on trigger ordering), new column name — and the name is `body` on
-- BOTH tables precisely so one trigger function serves both.
create or replace function public.sync_usage_bump() returns trigger
language plpgsql set search_path = public as $$
declare
  v_account uuid   := coalesce(new.account_id, old.account_id);
  v_delta   bigint := coalesce(octet_length(new.body), 0) - coalesce(octet_length(old.body), 0);
begin
  if (tg_op = 'DELETE') then
    update public.sync_usage set bytes = greatest(bytes + v_delta, 0), updated_at = now()
      where account_id = v_account;
  else
    insert into public.sync_usage (account_id, bytes) values (v_account, greatest(v_delta, 0))
    on conflict (account_id) do update
      set bytes = greatest(public.sync_usage.bytes + v_delta, 0), updated_at = now();
  end if;
  return null;
end;
$$;

create trigger sync_records_usage
  after insert or update or delete on public.sync_records
  for each row execute function public.sync_usage_bump();
create trigger sync_notes_usage
  after insert or update or delete on public.sync_notes
  for each row execute function public.sync_usage_bump();

alter table public.sync_records enable row level security;
alter table public.sync_notes   enable row level security;

-- Read-your-own, and nothing else. There is deliberately no client write policy on either: a push
-- goes through `sync-push`, which verified the JWT and then used the service role.
create policy sync_records_select_own on public.sync_records
  for select to authenticated using (account_id = auth.uid());
create policy sync_notes_select_own on public.sync_notes
  for select to authenticated using (account_id = auth.uid());
