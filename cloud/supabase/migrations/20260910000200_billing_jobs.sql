-- Knowlu C1, Task 5 — the state the two billing jobs need, and the daily tick that drives them.
create table public.billing_reminders (
  account_id       uuid primary key references public.accounts (id) on delete cascade,
  last_reminded_at timestamptz not null
);
alter table public.billing_reminders enable row level security;
create policy billing_reminders_select_own on public.billing_reminders
  for select to authenticated using (account_id = auth.uid());

-- Everything the job needs about one subscriber, in one read. A view rather than a join written
-- inside the function: the shape belongs with the tables, and PostgREST reads a view like a table.
create view public.billing_subscribers
with (security_invoker = true) as
select a.id                              as account_id,
       a.email                           as email,
       e.stripe_subscription_id          as subscription_id,
       e.plan                            as plan,
       coalesce(e.paused, false)         as paused,
       coalesce(e.started_at, a.created_at) as started_at,
       r.last_reminded_at                as last_reminded_at,
       -- Not read by either mail today (that's a separate call, Quinn's to make); this only makes
       -- the renewal date possible for the job to name without a second query.
       e.current_period_end              as current_period_end
from public.accounts a
join public.entitlements e on e.account_id = a.id
left join public.billing_reminders r on r.account_id = a.id
where e.status in ('active', 'trialing')
  and e.stripe_subscription_id is not null;

-- The webhook learns these from the subscription object; adding them here rather than in the first
-- migration keeps Task 1's table exactly the shape §5.1 names.
alter table public.entitlements add column stripe_subscription_id text;
alter table public.entitlements add column paused boolean not null default false;
alter table public.entitlements add column started_at timestamptz;

-- The daily tick. `app.billing_jobs_url` and `app.billing_jobs_token` are set once, by Quinn:
--   alter database postgres set app.billing_jobs_url   = 'https://<ref>.supabase.co/functions/v1/billing-jobs';
--   alter database postgres set app.billing_jobs_token = '<a random token he generates>';
-- A dedicated job token, deliberately, and not the service-role key: a database's configuration is
-- readable by anything with the database, and the worst a job token can do is run this one function.
-- pg_cron is **not relocatable** and Supabase installs it into its own fixed schema, so a
-- `with schema` clause here fails the migration outright. pg_net is relocatable and lives in
-- `extensions`, which is where Supabase puts it.
create extension if not exists pg_cron;
create extension if not exists pg_net with schema extensions;

select cron.schedule(
  'knowlu-billing-jobs',
  '17 7 * * *',
  $$
  select net.http_post(
    url     := current_setting('app.billing_jobs_url', true),
    headers := jsonb_build_object(
                 'content-type', 'application/json',
                 'x-knowlu-job-token', current_setting('app.billing_jobs_token', true)),
    body    := '{}'::jsonb
  );
  $$
);
