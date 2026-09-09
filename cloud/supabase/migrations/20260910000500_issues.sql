-- Knowlu C1, Task 9 — issue reports. Legal note §9: kept 90 days, under the same access control as
-- everything else, and never called "anonymous" — every row carries an account id.
create table public.issues (
  id           uuid primary key default extensions.gen_random_uuid(),
  account_id   uuid not null references public.accounts (id) on delete cascade,
  created_at   timestamptz not null default now(),
  app_version  text,
  engine_build text,
  os_build     text,
  profile_id   text,
  body         text not null,
  payload      jsonb not null default '{}'::jsonb
);
alter table public.issues enable row level security;
create policy issues_select_own on public.issues
  for select to authenticated using (account_id = auth.uid());

select cron.schedule(
  'knowlu-issue-sweep',
  '53 7 * * *',
  $$ delete from public.issues where created_at < now() - interval '90 days'; $$
);
