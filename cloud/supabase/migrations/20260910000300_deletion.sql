-- Knowlu C1, Task 6 — the tombstone a deleted account leaves behind.
-- Spec §5.1: "a tombstone (email_hash, deleted_at) kept 90 days for abuse control, then gone."
-- A hash, not an address: it answers "has this address deleted an account recently" and nothing else.
create table public.deleted_accounts (
  email_hash text primary key,
  deleted_at timestamptz not null default now()
);
alter table public.deleted_accounts enable row level security;
-- No policy at all: nothing but the service role has any business reading this table.

-- 90 days, swept daily by the same cron the billing jobs use.
select cron.schedule(
  'knowlu-tombstone-sweep',
  '41 7 * * *',
  $$ delete from public.deleted_accounts where deleted_at < now() - interval '90 days'; $$
);
