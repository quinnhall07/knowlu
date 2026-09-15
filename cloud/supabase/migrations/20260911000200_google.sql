-- Knowlu C2 — Gmail, server-side (cloud design §5.3, D12).
--
-- The refresh token is a restricted-scope credential and never reaches the device. It is held in
-- Supabase Vault, and this table keeps only the Vault secret's id — so a dump of this table is not
-- a dump of anybody's mailbox access, and Alabama's SPII definition (§9) is satisfied by the
-- Vault's encryption rather than by our care.
create table if not exists google_accounts (
  account_id      uuid primary key references public.accounts(id) on delete cascade,
  google_sub      text not null,
  secret_id       uuid not null,
  email_hint      text,
  -- What Google actually granted, not what was asked for (§11a). The calendar scope comes first
  -- and alone; `gmail.readonly` is added incrementally and only if the student takes that step, so
  -- every reader checks this rather than assuming: `/ingest-calendar` refuses without the calendar
  -- scope, `gmail-read` refuses without the Gmail one.
  scopes          text[] not null default '{}',
  -- The seam for §5.3's "user-excludable labels". No UI chooses them yet — the settings panel is
  -- C1's and the wizard's Gmail step asks only connect-or-not — so this is empty for everyone and
  -- `gmail-read` turns each entry into a `-label:<name>` term. The day the control exists it is a
  -- UI change and no schema change. Recorded as a narrowing in the fidelity ledger.
  excluded_labels text[] not null default '{}',
  connected_at    timestamptz not null default now(),
  last_read_at    timestamptz,
  -- `quiet` is §5.3's "source went quiet": connected, but yielding nothing for 14 days.
  status          text not null default 'active' check (status in ('active', 'revoked', 'quiet'))
);
alter table google_accounts enable row level security;

-- The dedup set. `gmail:<message-id>` is the same uid the device's state/ingest-seen.md holds, so
-- a message is judged once whichever side asks.
create table if not exists gmail_seen (
  account_id uuid not null references public.accounts(id) on delete cascade,
  uid        text not null,
  seen_at    timestamptz not null default now(),
  primary key (account_id, uid)
);
alter table gmail_seen enable row level security;

-- What the device has not pulled yet. `payload` is the VERDICT and the fields to write — never
-- the message text, which is discarded the moment the judgment returns (§5.3).
create table if not exists gmail_queue (
  id           bigserial primary key,
  account_id   uuid not null references public.accounts(id) on delete cascade,
  uid          text not null,
  tier         text not null check (tier in ('task', 'borderline', 'event', 'opportunity', 'information')),
  payload      jsonb not null,
  judgment_id  uuid references judgments(id) on delete set null,
  queued_at    timestamptz not null default now(),
  delivered_at timestamptz
);
alter table gmail_queue enable row level security;
create index if not exists gmail_queue_undelivered on gmail_queue (account_id, queued_at) where delivered_at is null;

-- The single-use OAuth `state` nonce. Bound to an account, consumed by the callback, and expired
-- after ten minutes: without it the callback would take any code from anyone and attach the
-- resulting mailbox to whichever account the URL happened to name.
create table if not exists google_state (
  nonce      text primary key,
  account_id uuid not null references public.accounts(id) on delete cascade,
  issued_at  timestamptz not null default now()
);
alter table google_state enable row level security;

-- The expiry is enforced, not merely commented: `take_google_state` refuses an old nonce in the
-- same statement that consumes it, so there is no window between the check and the delete.
create or replace function take_google_state(p_nonce text)
returns uuid
language sql
security invoker
set search_path = public, extensions
as $$
  delete from google_state
   where nonce = p_nonce and issued_at > now() - interval '10 minutes'
  returning account_id;
$$;

-- Housekeeping: nonces nobody came back for. Cheap, and it runs beside the nightly promotion.
select cron.schedule('knowlu-sweep-google-state', '23 7 * * *',
  $$delete from google_state where issued_at < now() - interval '1 day';$$);

-- Telemetry class (c) — raw content, behind its own opt-in — must NEVER include anything derived
-- from the Gmail API, whatever the user has opted into. Google's Workspace API User Data policy
-- (2026-07-22) allows no training "beyond that specific user's personalized model", and per-user
-- rule promotion is the only learning we do on it (§5.3, §9). One predicate, and losing it would
-- be silent, so `gmail_rows_are_excluded_from_the_training_export` pins its text.
create or replace function export_training_rows(p_since timestamptz)
returns setof judgments
language sql
stable
security invoker
set search_path = public, extensions
as $$
  select * from judgments where origin <> 'gmail_api' and judged_at >= p_since;
$$;

-- The Vault is not reachable through PostgREST, so these three are the whole interface to it.
-- `security definer` with a pinned `search_path`, and `revoke execute … from public` so only the
-- service role can call them.
create or replace function store_google_grant(
  p_account uuid, p_sub text, p_email text, p_token text, p_scopes text[]
)
returns uuid
language plpgsql
security definer
set search_path = public, vault, extensions
as $$
declare
  sid uuid;
begin
  select vault.create_secret(p_token, 'google:' || p_account::text, 'Google refresh token (read-only scopes)')
    into sid;
  -- `scopes` is a UNION on conflict, not a replacement: `gmail.readonly` is asked for
  -- incrementally and Google returns only the newly granted scope on that exchange, so
  -- overwriting would silently forget the calendar the student connected in the wizard (§11a).
  insert into google_accounts (account_id, google_sub, secret_id, email_hint, scopes, status)
       values (p_account, p_sub, sid, p_email, p_scopes, 'active')
  on conflict (account_id) do update
       set google_sub = excluded.google_sub, secret_id = excluded.secret_id,
           email_hint = excluded.email_hint, status = 'active', connected_at = now(),
           scopes = (
             select array_agg(distinct s)
               from unnest(google_accounts.scopes || excluded.scopes) as s
           );
  return sid;
end;
$$;

create or replace function delete_google_grant(p_account uuid)
returns void
language plpgsql
security definer
set search_path = public, vault, extensions
as $$
declare
  sid uuid;
begin
  select secret_id into sid from google_accounts where account_id = p_account;
  delete from google_accounts where account_id = p_account;
  if sid is not null then
    delete from vault.secrets where id = sid;
  end if;
end;
$$;

-- `p_scope` is checked here rather than by the caller: the grant may carry the calendar scope,
-- the Gmail scope, or both, and a reader that assumed would read a mailbox the student never
-- offered. `/ingest-calendar` passes the calendar scope; `gmail-read` passes the Gmail one.
create or replace function read_google_grant(p_account uuid, p_scope text)
returns text
language sql
security definer
set search_path = public, vault, extensions
as $$
  select s.decrypted_secret
    from google_accounts g
    join vault.decrypted_secrets s on s.id = g.secret_id
   where g.account_id = p_account
     and g.status <> 'revoked'
     and p_scope = any (g.scopes);
$$;

-- Disconnect needs the token whatever scopes it carries, so it gets its own reader rather than
-- passing a scope it does not care about.
create or replace function read_google_grant_any(p_account uuid)
returns text
language sql
security definer
set search_path = public, vault, extensions
as $$
  select s.decrypted_secret
    from google_accounts g
    join vault.decrypted_secrets s on s.id = g.secret_id
   where g.account_id = p_account;
$$;

revoke execute on function store_google_grant(uuid, text, text, text, text[]) from public;
revoke execute on function delete_google_grant(uuid) from public;
revoke execute on function read_google_grant(uuid, text) from public;
revoke execute on function read_google_grant_any(uuid) from public;
