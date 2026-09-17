-- Knowlu C3, Task 1 — the account's encrypted copy of its own vault. Spec §5.5, decided under D4.
--
-- Four properties here are structural rather than procedural, and each is pinned by
-- `cloud/supabase/migrations_sync_test.ts`:
--
--   1. **Nothing here is readable by us, and nothing here is GUESSABLE by us either.** Every row
--      holds base64 AES-256-GCM ciphertext sealed on the device with a 32-byte key that lives in
--      that machine's Windows Credential Manager and is never sent anywhere. What is NOT ciphertext
--      is exactly what the service needs in order to order, deduplicate, bound and bill: an opaque
--      16-hex device token, a 64-hex content MAC, a base64 12-byte IV, a length, one `keep` bit and
--      a receive timestamp. No path, no title, no field name, no note id, no timestamp of the
--      student's own. **The device token, the content MAC and the note ref are HMAC-SHA256 under a
--      key derived from the sync key** (`HKDF(…, "knowlu/index")`), not plain digests: a hash of a
--      guessable string is that string to whoever holds the table, and vault paths and journal
--      records are both guessable. Same column types, same uniqueness, same cursor, no oracle.
--   2. **A retried push is free.** `sync_records_once` is unique on the content MAC, so a batch
--      re-sent after a dropped connection lands on the rows it landed on the first time — the same
--      argument `telemetry_events_once` makes, one table over.
--   3. **The cursor is an identity column.** A client asks for `seq > n`. There is no clock two
--      machines have to agree about and no ordering the server has to be told.
--   4. **RLS is on and no table has a client write policy.** Every write in this system goes through
--      an edge function that verified the caller's JWT and then used the service role.
--
-- What a row is NOT: a backup of a file the service can restore for you. It is a blob the device
-- that made it can re-open. That is the whole of §5.5's "the cloud copy is the durable one" and the
-- whole of why `site/privacy.html` can keep saying what it says.

-- A note's row is UPSERTED — a note is current state, not history — so `generated always as
-- identity` would not move on a re-push and a client would never see the new text. A sequence plus a
-- trigger gives every write, insert or update, a fresh cursor value.
create sequence public.sync_notes_rev as bigint;

create table public.sync_records (
  account_id  uuid        not null references public.accounts (id) on delete cascade,
  seq         bigint      generated always as identity,
  device      text        not null check (device ~ '^[0-9a-f]{16}$'),
  record_hash text        not null check (record_hash ~ '^[0-9a-f]{64}$'),
  -- base64 of exactly twelve bytes: sixteen characters, no padding.
  iv          text        not null check (length(iv) = 16),
  -- 16 KiB of plaintext is ~21,896 base64 characters; 24,576 is that with room and no more. A
  -- journal record is a handful of scalars and, for a `create`, one frontmatter mapping.
  ciphertext  text        not null check (length(ciphertext) between 1 and 24576),
  -- **The one cleartext bit about a record's content, and it buys retention correctness.** The
  -- device sets it for a record a human wrote (`actor` not starting with `agent:`, `op` one of
  -- `set`/`create`); `sync_prune` never deletes one. See the comment on `sync_prune` for what it
  -- leaks (roughly how many of a student's writes were their own) and why that is the trade.
  keep        boolean     not null default false,
  received_at timestamptz not null default now(),
  primary key (account_id, seq),
  constraint sync_records_once unique (account_id, record_hash)
);
comment on table public.sync_records is
  'Spec §5.5. One row per journal record, sealed on the device. We cannot read any of it and do not
   hold the key; the student''s own machine does. `record_hash` is a hash of unknown plaintext and is
   here so a retry is idempotent.';

create table public.sync_notes (
  account_id uuid        not null references public.accounts (id) on delete cascade,
  -- HMAC-SHA256 of the note's vault-relative path, POSIX-separated, under the device's index key.
  -- The PATH itself never leaves the device: it is inside the ciphertext, beside the text, because
  -- a restore needs it and nothing else does. **Keyed, not a digest**: `tasks/<slugified title>.md`
  -- over a known alphabet is a few million candidates, and a digest of a candidate list is the list.
  note_ref   text        not null check (note_ref ~ '^[0-9a-f]{64}$'),
  rev        bigint      not null default nextval('public.sync_notes_rev'),
  device     text        not null check (device ~ '^[0-9a-f]{16}$'),
  -- A note that has moved or been settled into `archive/` is pushed as a tombstone: the path's row
  -- stays, so a restore knows not to resurrect it, and carries no bytes.
  deleted    boolean     not null default false,
  iv         text        null check (iv is null or length(iv) = 16),
  -- 128 KiB of plaintext is ~174,768 base64 characters. A note longer than that is a pasted
  -- document, not a task, and the device refuses it by name rather than truncating it.
  ciphertext text        null check (ciphertext is null or length(ciphertext) between 1 and 196608),
  updated_at timestamptz not null default now(),
  primary key (account_id, note_ref),
  constraint sync_notes_tombstone check (
    (deleted and iv is null and ciphertext is null) or
    (not deleted and iv is not null and ciphertext is not null)
  )
);
comment on table public.sync_notes is
  'Spec §5.5, "the note text they produced". Current state, not history: one row per note path, upserted.
   The path is inside the ciphertext; `note_ref` is its hash.';

create function public.sync_notes_stamp_rev() returns trigger
language plpgsql set search_path = public as $$
begin
  -- Every write, insert or update, takes a fresh cursor value. Without this an updated note keeps
  -- its old `rev` and a second device never learns the text changed.
  new.rev := nextval('public.sync_notes_rev');
  new.updated_at := now();
  return new;
end;
$$;

create trigger sync_notes_rev_stamp
  before insert or update on public.sync_notes
  for each row execute function public.sync_notes_stamp_rev();

create index sync_notes_rev_idx on public.sync_notes (account_id, rev);

-- **Which key the account's copy is sealed under, by fingerprint.** One row per account.
--
-- This exists because a copy is only useful to a device that can open it, and a device can lose that
-- ability without knowing: the student turns the switch off and on again on their laptop, that
-- machine mints a fresh key and re-uploads the whole vault under it, and **every other device of the
-- account is now holding a key the copy is no longer sealed with**. Without this row the desktop
-- would go on pulling rows it cannot decrypt and pushing rows nobody can — for ever, quietly, and
-- counting against the ceiling both ways.
--
-- The fingerprint is **not a secret**: it is the first eight hex characters of `SHA-256(key)`, it is
-- already Credential Manager's `UserName` for that credential, and it is shown on the restore screen
-- so a student can tell two keys apart. It is not reversible and it is not the key.
--
-- Written by `sync-push` only: the first push of an account's life claims the generation, and a push
-- carrying `reset: true` claims the next one. A push or pull whose fingerprint is neither the current
-- one nor a claim is **409 and stores nothing** — which is what gives the stale device something to
-- say instead of something to fail at.
create table public.sync_generation (
  account_id      uuid        primary key references public.accounts (id) on delete cascade,
  key_fingerprint text        not null check (key_fingerprint ~ '^[0-9a-f]{8}$'),
  set_at          timestamptz not null default now()
);
comment on table public.sync_generation is
  'Which key generation the account''s copy is sealed under (spec §5.5, second device). A fingerprint,
   not a key: eight hex characters of a SHA-256, already this credential''s UserName, never reversible.';

-- **A counter, not a sum.** `select sum(length(ciphertext))` on every push is O(rows) on the hot
-- path; two triggers keep an O(1) number instead.
create table public.sync_usage (
  account_id uuid        primary key references public.accounts (id) on delete cascade,
  bytes      bigint      not null default 0,
  updated_at timestamptz not null default now()
);

create function public.sync_usage_bump() returns trigger
language plpgsql set search_path = public as $$
declare
  v_account uuid   := coalesce(new.account_id, old.account_id);
  v_delta   bigint := coalesce(length(new.ciphertext), 0) - coalesce(length(old.ciphertext), 0);
begin
  insert into public.sync_usage (account_id, bytes) values (v_account, greatest(v_delta, 0))
  on conflict (account_id) do update
    set bytes = greatest(public.sync_usage.bytes + v_delta, 0), updated_at = now();
  return null;
end;
$$;

create trigger sync_records_usage
  after insert or update or delete on public.sync_records
  for each row execute function public.sync_usage_bump();
create trigger sync_notes_usage
  after insert or update or delete on public.sync_notes
  for each row execute function public.sync_usage_bump();

-- **The ceiling (precondition P3, recommended value).** 200 MiB of ciphertext per account. A vault
-- after a year of daily use is a few megabytes of notes and a few more of journal, so this is about
-- fifty times a heavy user: it is a runaway guard — a loop that re-pushes, a pasted binary — and not
-- a quota anyone should ever meet. Changing it is this one line in a new migration.
create function public.sync_ceiling_bytes() returns bigint
language sql immutable set search_path = public as $$ select 209715200::bigint $$;

-- One row, one column, so `sync-push` reads the ceiling with the same `restSelect` shape as
-- everything else rather than a second access pattern (PostgREST's `/rpc/` route answers a scalar,
-- not an array, and one exception in a forty-line client is one too many). Not readable by a client:
-- the number is not a secret, but a view nobody queries is a view nobody has to reason about.
create view public.sync_limits with (security_invoker = false) as
  select public.sync_ceiling_bytes() as ceiling;
revoke all on public.sync_limits from anon, authenticated;

-- **Retention (precondition P3, recommended value AND recommended shape).** Journal records older
-- than 400 days go; note rows never do, because a note row is the note's current text and a note
-- that still exists still needs one. Thirteen months means a student who comes back after a summer
-- still finds last year.
--
-- **And a record a human wrote is never pruned, whatever its age.** `journal::human_set`
-- (`engine/src/journal.rs:271`) is what **judge-once** reads, and it reads records: on a machine
-- restored from this copy, a pruned human `set` is a decision the student made and the device can no
-- longer see, so an agent may quietly re-set it. Keeping those costs a rounding error — a human set
-- is the rare record in a vault full of ingest and enrichment writes — and it removes a silent
-- correctness loss from the one path the whole feature exists for.
--
-- **The server cannot read a record, so how does it know?** It does not, and it must not: the row is
-- ciphertext. The DEVICE marks it, in the one cleartext bit this design allows for the purpose —
-- `sync_records.keep` — set by `build_push` from the record it is about to seal (`actor` not
-- starting with `agent:` and `op = 'set'` or `'create'`). That leaks one bit per record: roughly how
-- many of a student's writes were their own. It is the smallest thing that makes retention correct,
-- and it is named in the migration rather than hidden in a comment.
create function public.sync_prune(p_days int default 400) returns bigint
language plpgsql set search_path = public as $$
declare
  v_gone bigint;
begin
  delete from public.sync_records
   where received_at < now() - make_interval(days => p_days)
     and not keep;
  get diagnostics v_gone = row_count;
  return v_gone;
end;
$$;

-- No client ever calls this. `cron.schedule` below runs it as `postgres`, and the service role
-- keeps `execute` by default — only PostgREST's anon and authenticated roles are named here.
revoke execute on function public.sync_prune(int) from public, anon, authenticated;

alter table public.sync_records    enable row level security;
alter table public.sync_notes      enable row level security;
alter table public.sync_usage      enable row level security;
alter table public.sync_generation enable row level security;

-- Read-your-own, and nothing else. There is deliberately no client write policy on any of the three:
-- a push goes through `sync-push`, which verified the JWT and then used the service role.
create policy sync_records_select_own on public.sync_records
  for select to authenticated using (account_id = auth.uid());
create policy sync_notes_select_own on public.sync_notes
  for select to authenticated using (account_id = auth.uid());
create policy sync_usage_select_own on public.sync_usage
  for select to authenticated using (account_id = auth.uid());
create policy sync_generation_select_own on public.sync_generation
  for select to authenticated using (account_id = auth.uid());

-- pg_cron is not relocatable and Supabase installs it into its own fixed schema (C1's
-- 20260910000200 says so and pays for it); `if not exists` records the dependency. No pg_net and no
-- Vault secret here: pruning is plain SQL and needs no HTTP call, which is why this job has no guard
-- block and no token.
create extension if not exists pg_cron;

select cron.schedule('knowlu-sync-prune', '41 4 * * *', $$ select public.sync_prune(); $$);
