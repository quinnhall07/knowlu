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
