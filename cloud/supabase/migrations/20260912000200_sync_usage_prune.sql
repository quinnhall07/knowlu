-- Knowlu C3, Task 1 fix round 2. 20260912000100_sync.sql is already applied to staging and stays
-- exactly as it shipped (migrations are forward-only); this is the follow-up the task review asked
-- for.
--
-- 1. `sync_usage_bump` never inserts a row on DELETE. The old body always ran the same
--    insert-or-update: fine for an insert or update, but on a cascade from `accounts` the two usage
--    tables (`sync_records`, `sync_notes`) are deleted inside the SAME transaction as the account
--    row, and Postgres does not promise which cascade runs first. If `sync_usage`'s own cascade
--    (`account_id references public.accounts (id) on delete cascade`) fires before the trigger
--    finishes deleting every `sync_records`/`sync_notes` row, the old body's `insert ... on
--    conflict` re-creates a `sync_usage` row for an account that is, in this same transaction,
--    already gone — and that insert's own foreign key to `accounts` fails, aborting the entire
--    account deletion. A privacy-path failure rests on trigger ordering nothing in this schema
--    controls. On DELETE the row this trigger is about is going away regardless (or was never meant
--    to be recreated), so it only ever needs to subtract — never to insert.
create or replace function public.sync_usage_bump() returns trigger
language plpgsql set search_path = public as $$
declare
  v_account uuid   := coalesce(new.account_id, old.account_id);
  v_delta   bigint := coalesce(length(new.ciphertext), 0) - coalesce(length(old.ciphertext), 0);
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

-- 2. The nightly `sync_prune` scans `sync_records` by exactly this predicate; without a matching
--    partial index that scan is a full table scan across every account, every night, forever.
create index sync_records_prune_idx on public.sync_records (received_at) where not keep;

-- 3. `sync_limits`'s own comment already says "not readable by a client", but the function behind
--    it, `sync_ceiling_bytes`, was still callable directly over PostgREST with the anon key —
--    Supabase grants execute to anon and authenticated on every new function by default, same gap
--    as `sync_prune` had before this migration's sibling revoke. The number is not a secret, so
--    this is about the comment being true, not about hiding 209715200; the view stays readable by
--    `sync-push` because it is owned by `postgres`, not by whichever role calls it.
revoke execute on function public.sync_ceiling_bytes() from public, anon, authenticated;
