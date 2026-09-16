-- Knowlu C2 — the judgment service (cloud design §5.2, §5.4).
--
-- RLS is enabled on every table and NO POLICY IS CREATED, on purpose: with RLS on and no policy,
-- the anon and authenticated roles can read and write nothing at all. Every C2 edge function runs
-- with the service role, which bypasses RLS, and scopes every statement by the account_id that
-- `requireActiveEntitlement` returned. No judgment row is ever served to a client, so there is
-- nothing a client policy could be for. If a later stream needs one, it adds it deliberately.
--
-- `pg_cron` is needed by 20260911000400 (nightly rule promotion) and is enabled here, once, so
-- that migration is a function plus a schedule and nothing else.
--
-- CORRECTION (comment only, C2 final review S-6; the statement below is applied and stays as it
-- was): two things about this line are wrong and neither is load-bearing. The cross-reference
-- above said 20260911000300 — that migration is the Google privileges; rule promotion is
-- 20260911000400. And `with schema extensions` is wrong on pg_cron, which is NOT relocatable:
-- Supabase installs it into its own fixed `cron` schema and a `with schema` clause on a fresh
-- install fails the migration outright. It did not fail here only because C1 had already created
-- the extension (20260910000200_billing_jobs.sql:46, which says exactly this), so
-- `if not exists` short-circuited and the clause was never read. C1's is the one that made it;
-- this line is a no-op that must never be copied into a new project's first migration.
create extension if not exists pg_cron with schema extensions;

-- The pinned model per kind. Changing a pin is a migration row with a date, so every historical
-- judgment names the model that made it (cloud design §5.2, §5.4 measure 4).
--
-- `sampling` and the two prices live HERE and not in code, and both for the same reason: they are
-- properties of the pinned model, and the eval is allowed to move the pin (§11 R8).
--   * `sampling` — `temperature`/`top_p`/`top_k` are removed and return a 400 on Sonnet 5, Opus 5,
--     Opus 4.8/4.7 and Fable 5/5.1, and remain valid on Haiku 4.5 and the 4.6 generation. A pin
--     change to any of the first group sets this to `{}` in the same migration row.
--   * `usd_per_m_in` / `usd_per_m_out` — Haiku 4.5's published rates.
--     CORRECTION (comment only, C2 final review S-6): this used to claim `monthly_spend` prices
--     each month with the model that actually ran it. It does not, and cannot: `usage_daily` has
--     no model column, so the view joins `models` on `kind` alone and prices EVERY month — history
--     included — at whatever the pin is today. A pin change therefore re-prices the past. That is
--     accepted (the view is a budget alarm, not an invoice); pricing history correctly would mean
--     a model column on `usage_daily`, which is a schema change nothing today needs.
create table if not exists models (
  kind            text primary key check (kind in ('task', 'event', 'email')),
  provider        text not null default 'anthropic',
  model_id        text not null,
  prompt_version  text not null,
  grammar_version text not null,
  max_tokens      integer not null default 256 check (max_tokens between 64 and 4096),
  sampling        jsonb not null default '{"temperature": 0}'::jsonb,
  usd_per_m_in    numeric not null default 1.00,
  usd_per_m_out   numeric not null default 5.00,
  since           date not null default current_date
);
alter table models enable row level security;

-- `max_tokens` per kind, not one number: the email schema has eight fields including a 200-char
-- title and a 140-char why, and a truncated reply is a `max_tokens` stop that reads as a bad
-- answer. 256 is the classification default; email gets room.
insert into models (kind, provider, model_id, prompt_version, grammar_version, max_tokens) values
  ('task',  'anthropic', 'claude-haiku-4-5', 'task-1',  'task-1',  256),
  ('event', 'anthropic', 'claude-haiku-4-5', 'event-1', 'event-1', 256),
  ('email', 'anthropic', 'claude-haiku-4-5', 'email-1', 'email-1', 640)
on conflict (kind) do nothing;

-- One row per judgment, whatever the outcome. `fields` is field name -> the literal that was (or
-- would have been) written, plus the five promotion features (`created_by`, `title_prefix`,
-- `organizer`, `source`, `series_uid`) that tier 2 is keyed on. (Comment only, C2 final review
-- S-6: the count said four and the list has always named five.) Those are keys, not content: a
-- three-word title prefix is already the note's filename. There is nowhere here to put a title, a
-- body, a prompt or a reply, and `migrations_test.ts` keeps it so.
create table if not exists judgments (
  id              uuid primary key default gen_random_uuid(),
  account_id      uuid not null references public.accounts(id) on delete cascade,
  kind            text not null check (kind in ('task', 'event', 'email')),
  item_id         text not null,
  tier            smallint not null check (tier between 0 and 3),
  outcome         text not null check (outcome in ('answered', 'low confidence', 'capped')),
  cause           text check (cause in ('below floor', 'incomplete', 'model failed', 'refused', 'truncated')),
  confidence      real not null default 0 check (confidence between 0 and 1),
  fields          jsonb not null default '{}'::jsonb,
  model           text,
  prompt_version  text,
  grammar_version text,
  prompt_hash     text,
  ms              integer not null default 0,
  origin          text not null default 'device' check (origin in ('device', 'gmail_api', 'events')),
  judged_at       timestamptz not null default now()
);
alter table judgments enable row level security;
create index if not exists judgments_account_day on judgments (account_id, kind, judged_at desc);
create index if not exists judgments_item on judgments (account_id, kind, item_id, judged_at desc);
create index if not exists judgments_origin on judgments (origin) where origin = 'gmail_api';

-- Ruling R-X-2: `public.corrections` is C1's table, created by C1's telemetry migration with C1's
-- columns (`ts`, `received_at`, nullable `ours`/`theirs`, and a `kind` that is the NOTE kind:
-- task|approval|course|info|issue|archive). C2 adds exactly what the eval and the promotion loop
-- need and imposes no constraint on anything C1 owns.
--   * `judgment_id`  — which judgment this correction is about. C2's reply carries the id (Task 3),
--                      the device stores it beside the field it wrote, and C1's telemetry.rs sends
--                      it back. Nullable: a correction to something no cloud judgment produced is
--                      still a correction.
--   * `judgment_kind`— the JUDGMENT kind (task|event|email), which is not C1's note kind. Both are
--                      needed and neither can be derived from the other: an event verdict and a
--                      task both live in notes of kind `task` once approved.
alter table public.corrections add column if not exists judgment_id uuid null;
alter table public.corrections add column if not exists judgment_kind text null
  check (judgment_kind in ('task', 'event', 'email'));
create index if not exists corrections_judgment_kind_ts on public.corrections (judgment_kind, ts desc);

-- Tier 2. `scope = 'account'` rows are written by the nightly promotion job; `scope = 'global'`
-- rows are inserted inactive and are activated only by a hand review (§11 R5) — there is no code
-- path in this stream that sets active = true on a global row.
create table if not exists rules (
  id          bigserial primary key,
  account_id  uuid references public.accounts(id) on delete cascade,
  scope       text not null check (scope in ('account', 'global')),
  kind        text not null check (kind in ('task', 'event', 'email')),
  feature     text not null check (feature in ('source', 'organizer', 'title_prefix', 'series', 'created_by+title_prefix')),
  value       text not null,
  verdict     jsonb not null,
  active      boolean not null default false,
  version     integer not null default 1,
  proposed_at date not null default current_date,
  expires_at  date not null default (current_date + 30),
  decided_at  timestamptz,
  constraint account_rules_name_an_account check (scope <> 'account' or account_id is not null),
  constraint global_rules_name_no_account check (scope <> 'global' or account_id is null)
);
alter table rules enable row level security;

-- **One ACTIVE rule per (account, kind, feature, value), and that is what the index says.**
-- A partial unique index on `active`: superseded versions stay for the audit trail, and two live
-- answers to one question cannot exist. (An earlier draft included `version` in the key and
-- filtered on nothing, which enforced neither.)
create unique index if not exists rules_one_active_per_feature on rules (
  coalesce(account_id, '00000000-0000-0000-0000-000000000000'::uuid), kind, feature, value
) where active;
create index if not exists rules_lookup on rules (kind, active, account_id);

-- Why a rule was promoted: the judgments that agreed. Written by `promote_rules` (Task 12) and
-- read by nothing else — it is the answer to "why does the model never get asked about this?"
create table if not exists rule_evidence (
  rule_id     bigint not null references rules(id) on delete cascade,
  judgment_id uuid not null references judgments(id) on delete cascade,
  agrees      boolean not null,
  primary key (rule_id, judgment_id)
);
alter table rule_evidence enable row level security;

-- The cost guards of §5.2. **The caps and the price were chosen together** — see the arithmetic
-- beside `DAILY_CAP` in `_shared/judge_caps.ts`. In one line: at Haiku 4.5's $1/$5 per MTok and
-- this plan's own prompt bounds, one call is about $0.0015, so a per-account daily cap of 1,000
-- calls would permit ~$45/month against a $9.99 subscription. The caps below are set at roughly
-- 4x plausible heavy use, which is ~$4.50/month at the ceiling, and `enforce_budget` is the real
-- stop.
create table if not exists usage_daily (
  account_id uuid not null references public.accounts(id) on delete cascade,
  day        date not null,
  kind       text not null check (kind in ('task', 'event', 'email')),
  calls      integer not null default 0,
  in_tokens  bigint not null default 0,
  out_tokens bigint not null default 0,
  primary key (account_id, day, kind)
);
alter table usage_daily enable row level security;

-- One statement, so two concurrent calls cannot both read `calls` below the cap and both write.
-- Called only from `_shared/judge_caps.ts`'s `capStore`, over the service-role client — revoked
-- from anon/authenticated in 20260911000600_caps_privileges.sql (R-C2-E48 fix 2).
create or replace function charge_call(p_account uuid, p_kind text, p_cap integer)
returns boolean
language plpgsql
security invoker
set search_path = public, extensions
as $$
declare
  after integer;
begin
  insert into usage_daily (account_id, day, kind, calls)
       values (p_account, current_date, p_kind, 1)
  on conflict (account_id, day, kind)
    do update set calls = usage_daily.calls + 1
  returning calls into after;
  return after <= p_cap;
end;
$$;

-- The tokens a call actually used, recorded after the fact. Separate from `charge_call` because
-- the count is not known until the reply arrives, and a call that failed still spent its input.
-- Called only from `_shared/judge_caps.ts`'s `capStore`, over the service-role client — revoked
-- from anon/authenticated in 20260911000600_caps_privileges.sql (R-C2-E48 fix 2).
create or replace function record_tokens(p_account uuid, p_kind text, p_in bigint, p_out bigint)
returns void
language sql
security invoker
set search_path = public, extensions
as $$
  insert into usage_daily (account_id, day, kind, calls, in_tokens, out_tokens)
       values (p_account, current_date, p_kind, 0, p_in, p_out)
  on conflict (account_id, day, kind)
    do update set in_tokens = usage_daily.in_tokens + excluded.in_tokens,
                  out_tokens = usage_daily.out_tokens + excluded.out_tokens;
$$;

-- §5.2's "monthly inference budget alert per account and global". **`usage_daily` and `models`
-- alone** — an earlier draft joined `judgments` as well, which multiplied each day's usage row by
-- that day's judgment count and reported a number several hundred times too large.
create or replace view monthly_spend as
select date_trunc('month', u.day) as month,
       u.account_id,
       sum(u.in_tokens) / 1000000.0 * m.usd_per_m_in
         + sum(u.out_tokens) / 1000000.0 * m.usd_per_m_out as usd
  from usage_daily u
  join models m on m.kind = u.kind
 group by 1, 2, m.usd_per_m_in, m.usd_per_m_out;

create table if not exists budget_alerts (
  account_id uuid not null references public.accounts(id) on delete cascade,
  month      date not null,
  usd        numeric not null,
  raised_at  timestamptz not null default now(),
  primary key (account_id, month)
);
alter table budget_alerts enable row level security;

-- The budget as an ENFORCED ceiling, not a dashboard. Called by the pipeline before every model
-- call (once per account per hour is enough, so the pipeline memoises it): over the ceiling, the
-- judgment is refused with outcome `capped` exactly as a daily cap refusal is, and one row lands
-- in `budget_alerts` so the month's overspend is visible without querying a view nobody queries.
-- Called only from `_shared/judge_caps.ts`'s `capStore`, over the service-role client — revoked
-- from anon/authenticated in 20260911000600_caps_privileges.sql (R-C2-E48 fix 2).
create or replace function enforce_budget(p_account uuid, p_ceiling numeric)
returns boolean
language plpgsql
security invoker
set search_path = public, extensions
as $$
declare
  spent numeric;
begin
  select coalesce(sum(usd), 0) into spent
    from monthly_spend
   where account_id = p_account and month = date_trunc('month', current_date);
  if spent < p_ceiling then
    return true;
  end if;
  insert into budget_alerts (account_id, month, usd)
       values (p_account, date_trunc('month', current_date)::date, spent)
  on conflict (account_id, month) do update set usd = excluded.usd;
  return false;
end;
$$;
