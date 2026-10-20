-- L8 friends testers, S2a (docs/specs/2026-10-18-friends-testers-design.md §3.1, §3.3, §3.5, §6,
-- under its Amendment 2026-10-20): the comp grant table, its audit, and the admin schema the grant
-- functions will live in. A grant names an account by id, never an address, and is written after
-- sign-up. No function or view yet: the lane's later tasks (S2b to S2e2) append them to this file.

-- The admin schema. PostgREST exposes only `public` (config.toml [api] schemas), and every right on
-- this schema is revoked from the API roles, the service role included: only the owner reaches it,
-- from the SQL editor.
create schema if not exists admin;
revoke all on schema admin from public, anon, authenticated, service_role;

-- One row per grant. Every grant has an end; ending early sets ended_at and a one-word reason, and the
-- row is kept (for the record) until admin.comp_sweep removes it 30 days after it closes. The note is
-- the admin's words, shown to the tester in their export, never copied into comp_audit: no name or
-- relationship ("friends cohort, round 1", not who the person is).
create table public.comp_grants (
  id uuid primary key default gen_random_uuid(),
  account_id uuid not null references public.accounts(id) on delete cascade,
  cohort text not null check (cohort in ('friends', 'pilot')),
  granted_at timestamptz not null default now(),
  ends_at timestamptz not null,
  ended_at timestamptz,
  ended_reason text check (ended_reason in ('done', 'asked', 'misuse', 'other')),
  note text check (char_length(note) <= 200),
  check ((ended_at is null) = (ended_reason is null))
);

-- One open grant per account; an expired grant that was never ended still holds it (§3.3, P12).
create unique index comp_grants_one_open on public.comp_grants (account_id) where ended_at is null;

comment on table public.comp_grants is
  'Free tester access by account id (friends-testers spec 3.1). Written only through the admin.comp_* functions; removed by admin.comp_sweep or the cascade from accounts.';

-- The audit: append-only, written by the grant functions in the same transaction. No free-text
-- column: no note, no reason, no actor, no address or hash. db_role is a database role, never a
-- person. It goes with the account (cascade) and is held three years at most (admin.comp_sweep).
create table public.comp_audit (
  id uuid primary key default gen_random_uuid(),
  grant_id uuid references public.comp_grants(id) on delete set null,
  account_id uuid not null references public.accounts(id) on delete cascade,
  action text not null check (action in ('grant', 'extend', 'end')),
  ends_at timestamptz not null,
  cohort text not null,
  db_role text not null default session_user,
  at timestamptz not null default now()
);

create index comp_audit_account on public.comp_audit (account_id);

comment on table public.comp_audit is
  'Append-only record of comp grants, extensions and ends (friends-testers spec 3.5). No free text.';

-- No client reads or writes either table (RLS on, no policy). The service role, which every edge
-- function holds, keeps select for the caller's export and loses every write, so a mis-scoped write
-- in any function is refused by the database (A7). The owner, and the cascade from accounts that runs
-- as the owner, keep every right.
alter table public.comp_grants enable row level security;
alter table public.comp_audit enable row level security;
revoke all on public.comp_grants from public, anon, authenticated;
revoke all on public.comp_audit from public, anon, authenticated;
revoke insert, update, delete, truncate on public.comp_grants from service_role;
revoke insert, update, delete, truncate on public.comp_audit from service_role;

-- S2b (spec §3.1, §3.2): what the gate and GET /entitlement see, one row per account. It gates every
-- paying student, so it fails safe: the join starts at accounts and is left all the way, and a row
-- with no open grant is the entitlements row passed through column for column (a hand-deleted row
-- reads none / null / null, as GET /entitlement already answers for it). The grant branch is the
-- complement of the paid statuses, never a list of unpaid ones: the row every tester carries is the
-- sign-up 'none' row (handle_new_user), and a paid row always wins. A grant is matched by account id
-- alone; the view reads no auth table and no address. The partial unique index above (one open grant
-- per account) keeps it at one row per account. Exactly four columns: the names lookupFrom filters
-- and selects, and billing-checkout's plan lookup reads.
create view public.effective_entitlements as
select a.id as account_id,
       case when g.id is null then e.plan else 'tester' end as plan,
       case when g.id is null then coalesce(e.status, 'none') else 'active' end as status,
       case when g.id is null then e.current_period_end else g.ends_at end as current_period_end
  from public.accounts a
  left join public.entitlements e on e.account_id = a.id
  left join public.comp_grants g
    on g.account_id = a.id
   and g.ended_at is null
   and now() < g.ends_at
   and coalesce(e.status, 'none') not in ('active', 'trialing');

-- Owner rights, so no client may read it; the service role, which every gated function's lookup
-- uses, keeps its select (A7).
revoke all on public.effective_entitlements from public, anon, authenticated;

-- S2c1 (spec §3.3, §3.5): the three grant writers, each by account id, never an address. The owner
-- runs them from the SQL editor (the admin schema is closed to every API role), after finding the
-- account by the address the tester gives and running §8.3's checks. Each writes exactly one
-- comp_audit row on the grant's account in the same transaction, so a refused call writes none
-- (A1, A2). Security invoker, with an empty search path: they need no right the owner lacks.

-- A new grant. A second open grant for the account, even one past its ends_at that was never ended,
-- is refused by the partial unique index (P12); an id with no accounts row by the foreign key (P16).
-- The note is the admin's words, at most 200 characters, shown only to the tester in their export
-- and never copied into the audit: no name or relationship ("friends cohort, round 1").
create function admin.comp_grant(p_account_id uuid, p_ends_at timestamptz, p_cohort text, p_note text)
returns uuid
language plpgsql
set search_path = ''
as $$
declare
  v_id uuid;
begin
  begin
    insert into public.comp_grants (account_id, cohort, ends_at, note)
    values (p_account_id, p_cohort, p_ends_at, p_note)
    returning id into v_id;
  exception when unique_violation then
    raise exception 'an open grant exists for this account; use admin.comp_extend';
  end;
  insert into public.comp_audit (grant_id, account_id, action, ends_at, cohort)
  values (v_id, p_account_id, 'grant', p_ends_at, p_cohort);
  return v_id;
end
$$;

-- A new end date for the open grant, whether or not its ends_at has passed (so an expired grant
-- that was never ended is extended, not re-granted: P13). Refuses a date not in the future, and an
-- account with no open grant (none, or only ended ones).
create function admin.comp_extend(p_account_id uuid, p_ends_at timestamptz)
returns uuid
language plpgsql
set search_path = ''
as $$
declare
  v_id uuid;
  v_cohort text;
begin
  if p_ends_at is null or p_ends_at <= now() then
    raise exception 'the new ends_at is not in the future';
  end if;
  update public.comp_grants set ends_at = p_ends_at
   where account_id = p_account_id and ended_at is null
   returning id, cohort into v_id, v_cohort;
  if v_id is null then
    raise exception 'no open grant for this account';
  end if;
  insert into public.comp_audit (grant_id, account_id, action, ends_at, cohort)
  values (v_id, p_account_id, 'extend', p_ends_at, v_cohort);
  return v_id;
end
$$;

-- Ends the open grant early, with a one-word reason (done, asked, misuse, other; the table's
-- check). The row stays for the record until admin.comp_sweep; the audit keeps the end date the
-- grant had.
create function admin.comp_end(p_account_id uuid, p_reason text)
returns uuid
language plpgsql
set search_path = ''
as $$
declare
  v_id uuid;
  v_ends timestamptz;
  v_cohort text;
begin
  update public.comp_grants set ended_at = now(), ended_reason = p_reason
   where account_id = p_account_id and ended_at is null
   returning id, ends_at, cohort into v_id, v_ends, v_cohort;
  if v_id is null then
    raise exception 'no open grant for this account';
  end if;
  insert into public.comp_audit (grant_id, account_id, action, ends_at, cohort)
  values (v_id, p_account_id, 'end', v_ends, v_cohort);
  return v_id;
end
$$;

-- Closed to every role but the owner, on top of the schema revoke (A7).
revoke execute on function admin.comp_grant(uuid, timestamptz, text, text) from public, anon, authenticated, service_role;
revoke execute on function admin.comp_extend(uuid, timestamptz) from public, anon, authenticated, service_role;
revoke execute on function admin.comp_end(uuid, text) from public, anon, authenticated, service_role;

-- S2c2 (spec §3.1, §3.3, §3.5, Amendment 2026-10-20): deletion and retention. No trigger: both
-- comp_grants.account_id and comp_audit.account_id reference accounts on delete cascade (above), and
-- the database runs that cascade as the tables' owner, so a deletion by the service role (the
-- account function's purge), which holds no write right here, still takes them (A3, A3s).
-- The sweep only deletes. A grant goes 30 days after it closed: ended_at or ends_at, whichever is
-- first (least() skips a null ended_at), so a closed grant's note and reason are not held past its
-- life plus 30 days; its audit rows stay, their grant_id set null (A4). An audit row goes at three
-- years, a maximum (A6). Security invoker, run by pg_cron as the owner that scheduled it.
create function admin.comp_sweep()
returns void
language sql
set search_path = ''
as $$
  delete from public.comp_grants where least(ended_at, ends_at) < now() - interval '30 days';
  delete from public.comp_audit where at < now() - interval '3 years';
$$;

revoke execute on function admin.comp_sweep() from public, anon, authenticated, service_role;

-- Daily, at a minute field no other fixed-minute job in the corpus uses (§3.1).
select cron.schedule('knowlu-sweep-comp', '29 3 * * *', $c$select admin.comp_sweep();$c$);

-- S2d (spec §3.5, §4, Q5, Q11): a daily count of gated calls per account and route, for every
-- account, in usage_daily's shape. Counts only: the route is a function's name (judge-task,
-- ingest-zybooks, sync-push, ...), never what a call was about. Kept while the account lives, like
-- usage_daily; it goes with the account by the cascade and is in the caller's export. A route
-- outside §4's pattern is refused by the column's check, whoever writes it.
create table public.route_daily (
  account_id uuid not null references public.accounts(id) on delete cascade,
  day date not null,
  route text not null check (route ~ '^[a-z][a-z0-9-]{0,39}$'),
  calls integer not null default 0,
  primary key (account_id, day, route)
);

-- No client reads or writes it (RLS on, no policy). The service role keeps its rights, as on
-- usage_daily: count_route writes through it, and the account function's purge deletes from it.
-- A stray write here misstates a cost; it never grants access.
alter table public.route_daily enable row level security;
revoke all on public.route_daily from public, anon, authenticated;

-- One call counted: one statement, so two concurrent calls never lose one. Called from the gate
-- (_shared/entitlement.ts) after an entitled lookup, never awaited, its errors swallowed, and only
-- while the ROUTE_COUNTS secret reads on (§4). Security invoker, with an empty search path; execute
-- for the service role alone.
create function public.count_route(p_account uuid, p_route text)
returns void
language sql
set search_path = ''
as $$
  insert into public.route_daily (account_id, day, route, calls)
       values (p_account, current_date, p_route, 1)
  on conflict (account_id, day, route)
    do update set calls = route_daily.calls + 1;
$$;

revoke execute on function public.count_route(uuid, text) from public, anon, authenticated;
grant execute on function public.count_route(uuid, text) to service_role;
