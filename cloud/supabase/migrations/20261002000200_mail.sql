-- Knowlu email forwarding (spec docs/specs/2026-09-30-email-forwarding-design.md §5.1), T1.1: the
-- seven tables; T1.2: the kinds, the origin, the training and rules paths and the provisional pins;
-- T1.3, at the end: the three RPCs, the raised budget and the cron; T2g.1, before the cron: intake's
-- two RPCs. All before it is applied anywhere; nothing here edits an applied migration. Stamped after
-- 20261002000100_refund_failed_call.sql so it still sorts last.
--
-- Every table has RLS on, no policy, and Supabase's default grants to `anon` and `authenticated`
-- revoked: only the service role reads them, as `google_accounts` (§5.1). Every `account_id`
-- references accounts(id) on delete cascade; `mail_batches` and `mail_canaries` reach the account
-- through their mailbox, which cascades in turn. Where a row names an account and a mailbox, a
-- composite key makes them one account's, so a row can never name another account's mailbox.
--
-- What these rows never hold: a message body, a subject, a header value or a raw address token.
-- Message-IDs and fingerprints are HMACs under the account's own key (§4.6 step 7); the token and the
-- fingerprint key are AES-GCM ciphertext (the `sources` pattern of `_shared/crypto.ts`, a 12-byte IV
-- in base64, 16 characters); batch and canary nonces are kept only as HMACs.

-- One row per account that has set up mail. Kept until the account goes.
create table if not exists public.mail_accounts (
  account_id           uuid primary key references public.accounts (id) on delete cascade,
  -- The per-account fingerprint key `k` (§4.6 step 7), encrypted.
  fp_key_ciphertext    text not null,
  fp_key_iv            text not null check (length(fp_key_iv) = 16),
  -- D13's per-course digest, off unless the student turns it on.
  digest               boolean not null default false,
  -- D23's backfill allowance (as amended 2026-10-02, PQ6 (a)): one per account per 120-day period,
  -- keyed on the moment it opened, never on a mailbox, so Disconnect and reconnect never renew it.
  -- Zero until `charge_allowance` opens it: a closed allowance is the default, never an open one.
  allowance_opened_at  timestamptz,
  allowance_left       integer not null default 0 check (allowance_left >= 0),
  allowance_usd        numeric not null default 0 check (allowance_usd >= 0),
  allowance_until      timestamptz,
  -- D3's "six mints in 30 days": the times of this account's mints, a rolling window the minting
  -- route prunes and counts (a bare counter could not tell when a mint leaves the window).
  mints_30d            timestamptz[] not null default '{}'
);
alter table public.mail_accounts enable row level security;
revoke all on public.mail_accounts from anon, authenticated;

-- One row per connected mailbox and its secret address (D3). Deleted on Disconnect.
create table if not exists public.mail_addresses (
  id                  uuid primary key default gen_random_uuid(),
  account_id          uuid not null references public.accounts (id) on delete cascade,
  provider            text not null check (provider in ('m365', 'google', 'other')),
  -- Intake finds the mailbox by HMAC alone; the token itself is kept encrypted so Settings can show
  -- the address again.
  token_hmac          text not null unique,
  token_ciphertext    text not null,
  token_iv            text not null check (length(token_iv) = 16),
  -- Rotate's grace (D3): the old token still binds until `retired_until`.
  retired_token_hmac  text unique,
  retired_until       timestamptz,
  mailbox_hint        text,
  -- The forwarding evidence a canary carried (§4.6 step 5), e.g. the M365 tenant id.
  forward_hint        text,
  status              text not null default 'pending' check (status in ('pending', 'confirmed', 'active', 'quiet', 'blocked', 'off', 'paused_send', 'lapsed', 'revoked')),
  lapsed_since        timestamptz,
  reenabled_at        timestamptz,
  confirm_until       timestamptz,
  -- The mailbox's fixed backfill window start (BW: 120 days before setup), set once when its first
  -- backfill batch opens, so a later recover keeps it after that batch is pruned.
  backfill_since      date,
  last_received_at    timestamptz,
  last_canary_sent_at timestamptz,
  last_canary_ok_at   timestamptz,
  canaries_today      integer not null default 0 check (canaries_today >= 0),
  unbound             integer not null default 0 check (unbound >= 0),
  unauthenticated     integer not null default 0 check (unauthenticated >= 0),
  expired             integer not null default 0 check (expired >= 0),
  check ((retired_token_hmac is null) = (retired_until is null)),
  unique (id, account_id)
);
alter table public.mail_addresses enable row level security;
revoke all on public.mail_addresses from anon, authenticated;
create index if not exists mail_addresses_account on public.mail_addresses (account_id);

-- A backfill or recover send's batch (§4.2, §4.11). Kept 30 days after closing.
create table if not exists public.mail_batches (
  id         uuid primary key default gen_random_uuid(),
  address_id uuid not null references public.mail_addresses (id) on delete cascade,
  phase      text not null check (phase in ('backfill', 'recover')),
  nonce_hmac text not null unique,
  since      timestamptz not null,
  expected   integer not null check (expected >= 0),
  -- The device's count at close; null while the batch is open.
  sent       integer check (sent >= 0),
  received   integer not null default 0 check (received >= 0),
  opened_at  timestamptz not null default now(),
  sent_at    timestamptz,
  closed_at  timestamptz
);
alter table public.mail_batches enable row level security;
revoke all on public.mail_batches from anon, authenticated;
create index if not exists mail_batches_address on public.mail_batches (address_id);

-- A delivery check through the mailbox's real forwarding (D14, §4.10). Kept 30 days.
create table if not exists public.mail_canaries (
  id         uuid primary key default gen_random_uuid(),
  address_id uuid not null references public.mail_addresses (id) on delete cascade,
  nonce_hmac text not null unique,
  sent_at    timestamptz not null default now(),
  arrived_at timestamptz
);
alter table public.mail_canaries enable row level security;
revoke all on public.mail_canaries from anon, authenticated;
create index if not exists mail_canaries_address on public.mail_canaries (address_id, sent_at);

-- The per-account dedup set (§4.6 step 7). Written with its `mail_work` row, deleted if that item
-- expires; pruned at 180 days (a term and its overlap).
create table if not exists public.mail_seen (
  account_id      uuid not null references public.accounts (id) on delete cascade,
  -- HMAC_k(Message-ID) and HMAC_k(sender, normalised subject), k the account's fingerprint key.
  msg_hash        text not null,
  fp_hash         text not null,
  -- The one time field intake and reconcile share (§4.6 step 7, §4.11), truncated to the minute.
  received_minute timestamptz not null,
  address_id      uuid not null,
  phase           text not null check (phase in ('live', 'backfill', 'recover')),
  seen_at         timestamptz not null default now(),
  primary key (account_id, msg_hash),
  foreign key (address_id, account_id) references public.mail_addresses (id, account_id) on delete cascade
);
alter table public.mail_seen enable row level security;
revoke all on public.mail_seen from anon, authenticated;
create index if not exists mail_seen_fingerprint on public.mail_seen (account_id, fp_hash);
create index if not exists mail_seen_address on public.mail_seen (address_id);
create index if not exists mail_seen_age on public.mail_seen (seen_at);

-- An original waiting for `mail-process` (§4.7), by its S3 key under `work/`. Kept until processed,
-- or until its object expires (then counted `expired`).
create table if not exists public.mail_work (
  id              bigserial primary key,
  account_id      uuid not null references public.accounts (id) on delete cascade,
  address_id      uuid not null,
  batch_id        uuid references public.mail_batches (id) on delete set null,
  s3_key          text not null unique,
  phase           text not null check (phase in ('live', 'backfill', 'recover')),
  -- The word from §4.6 step 6, never the headers it was read from.
  authenticity    text not null check (authenticity in ('aligned', 'unaligned')),
  arrived_at      timestamptz not null default now(),
  attempts        integer not null default 0 check (attempts >= 0),
  next_attempt_at timestamptz not null default now(),
  foreign key (address_id, account_id) references public.mail_addresses (id, account_id) on delete cascade
);
alter table public.mail_work enable row level security;
revoke all on public.mail_work from anon, authenticated;
create index if not exists mail_work_due on public.mail_work (next_attempt_at, arrived_at);
create index if not exists mail_work_address on public.mail_work (address_id);

-- What the device pulls (§4.7 step 6): the claims and the few facts reconcile needs, never the text.
-- Delivered rows are kept 7 days, undelivered 30, and all of a mailbox's rows go on its Disconnect.
create table if not exists public.mail_claims (
  id            bigserial primary key,
  account_id    uuid not null references public.accounts (id) on delete cascade,
  address_id    uuid not null,
  batch_id      uuid references public.mail_batches (id) on delete set null,
  uid           text not null,
  message_date  timestamptz,
  ordering_date timestamptz not null,
  sender_name   text,
  sender_addr   text,
  phase         text not null check (phase in ('live', 'backfill', 'recover')),
  authenticity  text not null check (authenticity in ('aligned', 'unaligned')),
  -- The claim array of §5.1, validated server-side before it is queued.
  claims        jsonb not null check (jsonb_typeof(claims) = 'array'),
  judgment_id   uuid references public.judgments (id) on delete set null,
  queued_at     timestamptz not null default now(),
  delivered_at  timestamptz,
  foreign key (address_id, account_id) references public.mail_addresses (id, account_id) on delete cascade
);
alter table public.mail_claims enable row level security;
revoke all on public.mail_claims from anon, authenticated;
create index if not exists mail_claims_undelivered on public.mail_claims (account_id, queued_at) where delivered_at is null;
create index if not exists mail_claims_address on public.mail_claims (address_id);

-- T1.2: the existing objects, constraint by constraint (spec §5.1's "Changes to existing objects").
-- Each check below is the inline one of 20260911000100_judgment_service.sql, under the name Postgres
-- gave it (`<table>_<column>_check`), dropped and re-issued with the new words.
--
-- The three mail judgment kinds: `models` takes their pins, `usage_daily` lets `charge_call` and
-- `record_tokens` count them (without it `charge_call` answers 23514 and every claim reads `capped`),
-- and `judgments` records them.
alter table public.models drop constraint if exists models_kind_check;
alter table public.models add constraint models_kind_check
  check (kind in ('task', 'event', 'email', 'mail_claim', 'mail_resolve', 'mail_digest'));
alter table public.usage_daily drop constraint if exists usage_daily_kind_check;
alter table public.usage_daily add constraint usage_daily_kind_check
  check (kind in ('task', 'event', 'email', 'mail_claim', 'mail_resolve', 'mail_digest'));
alter table public.judgments drop constraint if exists judgments_kind_check;
alter table public.judgments add constraint judgments_kind_check
  check (kind in ('task', 'event', 'email', 'mail_claim', 'mail_resolve', 'mail_digest'));

-- A judgment of forwarded mail has its own origin, so the training export, the rules path and the
-- corrections back-fill can each pass over it by that word alone.
alter table public.judgments drop constraint if exists judgments_origin_check;
alter table public.judgments add constraint judgments_origin_check
  check (origin in ('device', 'gmail_api', 'events', 'forward'));

-- Not widened, deliberately: `corrections.judgment_kind`, `rules.kind`, `eval_cases.kind`,
-- `eval_runs.kind` and `telemetry/handler.ts`'s `JUDGMENT_KINDS`. A mail judgment never becomes a
-- rule, a correction or an eval row.

-- The training export: forwarded mail is excluded beside Gmail and calendar rows. Still security
-- invoker and non-writing.
create or replace function export_training_rows(p_since timestamptz)
returns setof judgments
language sql
stable
security invoker
set search_path = public, extensions
as $$
  select * from judgments where origin not in ('gmail_api', 'events', 'forward') and judged_at >= p_since;
$$;
revoke execute on function public.export_training_rows(timestamptz) from public, anon, authenticated;
grant execute on function public.export_training_rows(timestamptz) to service_role;

-- The corrections back-fill, re-issued from 20260911000500_rule_promotion_fix.sql with one change: it
-- never joins a correction to a forwarded-mail judgment. Such a join would write `mail_claim` into
-- `corrections.judgment_kind`, whose check refuses it, and abort the nightly job for every account.
create or replace function public.backfill_correction_judgments()
returns integer
language plpgsql
security invoker
set search_path = public, extensions
as $$
declare
  filled integer := 0;
begin
  update public.corrections c
     set judgment_id = j.judgment_id,
         judgment_kind = j.judgment_kind
    from (
      select distinct on (c2.id) c2.id as correction_id, j2.id as judgment_id, j2.kind as judgment_kind
        from public.corrections c2
        join public.judgments j2 on j2.account_id = c2.account_id and j2.item_id = c2.item_id and j2.judged_at < c2.ts
       where c2.judgment_id is null
         and j2.origin <> 'forward'
       order by c2.id, j2.judged_at desc, j2.id desc
    ) j
   where c.id = j.correction_id;
  get diagnostics filled = row_count;
  return filled;
end;
$$;
revoke execute on function public.backfill_correction_judgments() from public, anon, authenticated;
grant execute on function public.backfill_correction_judgments() to service_role;

-- Rule promotion, re-issued from 20260911000500_rule_promotion_fix.sql with one change: both reads of
-- `judgments` pass over forwarded mail. Otherwise `judgment_features` would offer a `mail_claim`
-- feature, the `rules` insert would refuse its kind, and the whole nightly promotion would abort for
-- every account. The nightly job (`knowlu-promote-rules`) calls it by name and is unchanged.
create or replace function public.promote_rules()
returns integer
language plpgsql
security invoker
set search_path = public, extensions
as $$
declare
  promoted integer := 0;
begin
  with judged as (
    select j.id, j.account_id, j.kind, j.item_id, j.judged_at, j.fields,
           j.fields - 'created_by' - 'title_prefix' - 'organizer' - 'source' - 'series_uid' as verdict_fields,
           f.feature, f.value
      from judgments j
      cross join lateral judgment_features(j.kind, j.fields) as f(feature, value)
     where j.tier = 3
       and j.outcome = 'answered'
       and j.origin <> 'forward'
       and j.judged_at >= now() - interval '60 days'
  ),
  agreed as (
    select account_id, kind, feature, value,
           count(distinct item_id) as items,
           count(distinct verdict_fields) as distinct_answers,
           (array_agg(verdict_fields order by judged_at desc, id desc))[1] as answer,
           array_agg(id) as judgment_ids
      from judged
     group by account_id, kind, feature, value
  ),
  contradicted as (
    select distinct j.account_id, j.kind, f.feature, f.value
      from public.corrections c
      join judgments j on j.id = c.judgment_id
      cross join lateral judgment_features(j.kind, j.fields) as f(feature, value)
     where c.ts >= now() - interval '60 days'
       and j.origin <> 'forward'
  ),
  fresh as (
    insert into rules (account_id, scope, kind, feature, value, verdict, active, proposed_at, expires_at)
    select a.account_id, 'account', a.kind, a.feature, a.value, a.answer, false,
           current_date, current_date + 30
      from agreed a
     where a.items >= 3
       and a.distinct_answers = 1
       and a.answer <> '{}'::jsonb
       and not exists (
             select 1 from contradicted x
              where x.account_id = a.account_id and x.kind = a.kind
                and x.feature = a.feature and x.value = a.value)
       and not exists (
             select 1 from rules r
              where r.account_id = a.account_id and r.kind = a.kind
                and r.feature = a.feature and r.value = a.value
                and (r.active or r.decided_at is null or r.decided_at > now() - interval '90 days'))
    returning id, account_id, kind, feature, value
  ),
  evidence as (
    insert into rule_evidence (rule_id, judgment_id, agrees)
    select f.id, unnest(a.judgment_ids), true
      from fresh f
      join agreed a
        on a.account_id = f.account_id and a.kind = f.kind
       and a.feature = f.feature and a.value = f.value
  )
  select count(*) into promoted from fresh;

  delete from rules where scope = 'account' and active = false and decided_at is null and expires_at < current_date;
  return promoted;
end;
$$;
revoke execute on function public.promote_rules() from public, anon, authenticated;
grant execute on function public.promote_rules() to service_role;

-- The pins (plan P8). PROVISIONAL: `mail_claim` and `mail_resolve` take `email`'s current model,
-- precision, price, sampling and route (one named zero-retention upstream, no fallback), read from
-- 20260916000100_provider_swap.sql's re-pin, which nothing since has moved. Test 17's live check
-- (T18a) confirms each pin, or a one-statement follow-up migration re-pins it before the proof.
-- The prompt and grammar versions are each kind's first; `max_tokens` is email's 640.
-- `mail_digest` gets a row only if T16 is built. The eval seed stays empty (R-C2-E12).
insert into public.models (kind, provider, model_id, prompt_version, grammar_version, max_tokens) values
  ('mail_claim',   'openrouter', 'qwen/qwen3.5-35b-a3b', 'mail_claim-1',   'mail_claim-1',   640),
  ('mail_resolve', 'openrouter', 'qwen/qwen3.5-35b-a3b', 'mail_resolve-1', 'mail_resolve-1', 640)
on conflict (kind) do nothing;
update public.models set
  provider = 'openrouter', model_id = 'qwen/qwen3.5-35b-a3b', precision = 'fp8 (DeepInfra)',
  usd_per_m_in = 0.14, usd_per_m_out = 1.00,
  sampling = '{"temperature": 0, "reasoning": {"enabled": false}}'::jsonb,
  route = '{"order": ["DeepInfra"], "allow_fallbacks": false, "zdr": true, "require_parameters": true}'::jsonb,
  since = current_date
where kind = 'mail_claim';
update public.models set
  provider = 'openrouter', model_id = 'qwen/qwen3.5-35b-a3b', precision = 'fp8 (DeepInfra)',
  usd_per_m_in = 0.14, usd_per_m_out = 1.00,
  sampling = '{"temperature": 0, "reasoning": {"enabled": false}}'::jsonb,
  route = '{"order": ["DeepInfra"], "allow_fallbacks": false, "zdr": true, "require_parameters": true}'::jsonb,
  since = current_date
where kind = 'mail_resolve';

-- T1.3: the three RPCs, the raised budget and the cron (spec §5.1; D17; D23 as amended 2026-10-02 by
-- PQ6 (a); plan P7).
--
-- Disconnect's purge of one mailbox (D17). Revoke first, so the row reads `revoked` for as long as the
-- purge holds it; then every row that serves the mailbox, its claims delivered or not (the Gmail
-- disconnect precedent); then the row. The account's own rows (`mail_accounts`, and with it the
-- allowance) and its judgments stay. Callers (`DELETE /mail/addresses/<id>` and the account purge)
-- have already scoped the id to the caller's account, and have read the mailbox's `work/` keys from
-- `mail_work` and deleted those objects before calling; nothing here can reach S3.
create or replace function public.delete_mail_address(p_address uuid)
returns void
language plpgsql
security definer
set search_path = public, extensions
as $$
begin
  update public.mail_addresses set status = 'revoked' where id = p_address;
  delete from public.mail_seen where address_id = p_address;
  delete from public.mail_work where address_id = p_address;
  delete from public.mail_claims where address_id = p_address;
  delete from public.mail_batches where address_id = p_address;
  delete from public.mail_canaries where address_id = p_address;
  delete from public.mail_addresses where id = p_address;
end;
$$;
revoke execute on function public.delete_mail_address(uuid) from public, anon, authenticated;
grant execute on function public.delete_mail_address(uuid) to service_role;

-- One backfill judgment charged to the account's allowance (D23 as amended, PQ6 (a)), in one
-- statement like `charge_call`: two concurrent charges serialise on the account's row, and the
-- second re-reads it, so neither can open the allowance twice or take it below zero. When no
-- allowance was ever opened, or the last opened 120 days or more ago, the statement opens one
-- (2,000 judgments, $1.00 above the ceiling, 14 days) and charges its first judgment. Otherwise it
-- charges only what is left, and nothing once `allowance_until` has passed. Every SET expression
-- reads the row as it was, so the opening test is the same in all four. No mailbox enters it:
-- Disconnect and reconnect inside the period renew nothing. Unlike `charge_call`, a refused charge
-- writes nothing; the caller then charges the daily cap (spec test 11's 2,001st item). An account
-- with no `mail_accounts` row answers false.
create or replace function public.charge_allowance(p_account uuid)
returns boolean
language plpgsql
security invoker
set search_path = public, extensions
as $$
declare
  charged boolean;
begin
  update public.mail_accounts set
    allowance_opened_at = case when (allowance_opened_at is null or allowance_opened_at <= now() - interval '120 days')
                               then now() else allowance_opened_at end,
    allowance_until     = case when (allowance_opened_at is null or allowance_opened_at <= now() - interval '120 days')
                               then now() + interval '14 days' else allowance_until end,
    allowance_usd       = case when (allowance_opened_at is null or allowance_opened_at <= now() - interval '120 days')
                               then 1.00 else allowance_usd end,
    allowance_left      = case when (allowance_opened_at is null or allowance_opened_at <= now() - interval '120 days')
                               then 2000 else allowance_left end - 1
   where account_id = p_account
     and ((allowance_opened_at is null or allowance_opened_at <= now() - interval '120 days')
          or (allowance_until > now() and allowance_left > 0))
  returning true into charged;
  return coalesce(charged, false);
end;
$$;
revoke execute on function public.charge_allowance(uuid) from public, anon, authenticated;
grant execute on function public.charge_allowance(uuid) to service_role;

-- One backfill judgment given back after a call the model never answered (the T1.3 review, after
-- F1's `refund_call` in 20261002000100_refund_failed_call.sql): `charge_allowance` charges before
-- the call, so without this a provider outage during a first backfill would spend the allowance on
-- failed calls and leave the backfill trickling at the daily cap for its 120 days. `mail-process`
-- (T3e) calls it on the pipeline's `model failed` outcome only; a refusal, a truncation or an answer
-- that fails validation reached the model and stays charged. One statement: it gives back only
-- while the allowance is open (`allowance_until` not yet passed) and never past 2,000, and it never
-- opens, extends or re-prices one. The row lock serialises it against `charge_allowance`, and
-- Postgres re-checks the `where` on the locked row. Request volume on a failing provider is bounded
-- by the item's wait-and-retry in `mail-process`, and dollars by `enforce_budget`'s raised ceiling.
create or replace function public.refund_allowance(p_account uuid)
returns boolean
language plpgsql
security invoker
set search_path = public, extensions
as $$
declare
  done boolean;
begin
  update public.mail_accounts set allowance_left = allowance_left + 1
   where account_id = p_account
     and allowance_until > now()
     and allowance_left < 2000
  returning true into done;
  return coalesce(done, false);
end;
$$;
revoke execute on function public.refund_allowance(uuid) from public, anon, authenticated;
grant execute on function public.refund_allowance(uuid) to service_role;

-- The budget ceiling, re-issued from 20260911000100_judgment_service.sql with one change (D23): while
-- the account's allowance is inside its 14 days, the ceiling is the caller's `MONTHLY_CEILING_USD`
-- plus the allowance's dollars, a hard per-account ceiling; from `allowance_until` on it is the bare
-- ceiling again. Allowance calls' tokens are in `usage_daily` through `record_tokens`, so this
-- month's spend includes them. Same signature, so `_shared/judge_caps.ts` does not change.
create or replace function public.enforce_budget(p_account uuid, p_ceiling numeric)
returns boolean
language plpgsql
security invoker
set search_path = public, extensions
as $$
declare
  spent  numeric;
  raised numeric;
begin
  select coalesce(sum(allowance_usd), 0) into raised
    from public.mail_accounts
   where account_id = p_account and allowance_until > now();
  select coalesce(sum(usd), 0) into spent
    from public.monthly_spend
   where account_id = p_account and month = date_trunc('month', current_date);
  if spent < p_ceiling + raised then
    return true;
  end if;
  insert into public.budget_alerts (account_id, month, usd)
       values (p_account, date_trunc('month', current_date)::date, spent)
  on conflict (account_id, month) do update set usd = excluded.usd;
  return false;
end;
$$;
revoke execute on function public.enforce_budget(uuid, numeric) from public, anon, authenticated;
grant execute on function public.enforce_budget(uuid, numeric) to service_role;

-- T2g.1: intake's two writes that PostgREST cannot make on its own (spec §4.6 steps 5, 6 and 8; D8).
--
-- The hand-off. `mail-inbound` calls it only after its `work/` put succeeded; the seen row and the work
-- row are written in this one transaction, or neither is. A Message-ID the account already holds (a
-- concurrent delivery of the same message) writes nothing and answers false, and the caller deletes its
-- put. The received minute is truncated here as well as by the caller.
create or replace function public.mail_hand_off(
  p_account uuid, p_address uuid, p_batch uuid, p_s3_key text, p_phase text,
  p_authenticity text, p_msg_hash text, p_fp_hash text, p_received_minute timestamptz
)
returns boolean
language plpgsql
security invoker
set search_path = public, extensions
as $$
declare
  fresh boolean;
begin
  insert into public.mail_seen (account_id, msg_hash, fp_hash, received_minute, address_id, phase)
       values (p_account, p_msg_hash, p_fp_hash, date_trunc('minute', p_received_minute), p_address, p_phase)
  on conflict (account_id, msg_hash) do nothing
  returning true into fresh;
  if fresh is null then
    return false;
  end if;
  insert into public.mail_work (account_id, address_id, batch_id, s3_key, phase, authenticity)
       values (p_account, p_address, p_batch, p_s3_key, p_phase, p_authenticity);
  return true;
end;
$$;
revoke execute on function public.mail_hand_off(uuid, uuid, uuid, text, text, text, text, text, timestamptz) from public, anon, authenticated;
grant execute on function public.mail_hand_off(uuid, uuid, uuid, text, text, text, text, text, timestamptz) to service_role;

-- Unbound and unauthenticated mail, counted on its address in one statement (a rising `unbound` means
-- the address leaked, and Settings offers Rotate). Any other counter name adds nothing.
create or replace function public.count_mail_arrival(p_address uuid, p_counter text)
returns void
language plpgsql
security invoker
set search_path = public, extensions
as $$
begin
  update public.mail_addresses set
    unbound         = unbound + (p_counter = 'unbound')::int,
    unauthenticated = unauthenticated + (p_counter = 'unauthenticated')::int
   where id = p_address;
end;
$$;
revoke execute on function public.count_mail_arrival(uuid, text) from public, anon, authenticated;
grant execute on function public.count_mail_arrival(uuid, text) to service_role;

-- The cron (spec §5.1, §6.3; plan P7), in 20260910000700's guarded form: the URL and the token come
-- from Vault, a missing one raises into `cron.job_run_details`, and the token travels only in
-- `x-knowlu-job-token`. `mail-process` every minute; spec D8's kick from intake after a hand-off is
-- not built yet (plan T3d adds it once `mail-process` exists), so until then this cron is the only
-- start. `mail-canary` hourly at :11; `mail-process` in sweep mode nightly at 07:47 UTC, a minute no
-- other nightly job uses.
--
-- The secrets, once per project, in the SQL editor (the controller's, T18a). The URLs are public; the
-- token is the same string as the function secret `MAIL_JOBS_TOKEN` that both functions compare the
-- header against:
--   select vault.create_secret('https://<ref>.supabase.co/functions/v1/mail-process', 'mail_process_url');
--   select vault.create_secret('https://<ref>.supabase.co/functions/v1/mail-canary', 'mail_canary_url');
--   select vault.create_secret('<the token>', 'mail_jobs_token');
create extension if not exists pg_cron;
create extension if not exists pg_net with schema extensions;
create extension if not exists supabase_vault with schema vault;

select cron.schedule(
  'knowlu-mail-process',
  '* * * * *',
  $job$
  do $guard$
  declare
    v_url   text := (select decrypted_secret from vault.decrypted_secrets
                     where name = 'mail_process_url' order by created_at desc limit 1);
    v_token text := (select decrypted_secret from vault.decrypted_secrets
                     where name = 'mail_jobs_token' order by created_at desc limit 1);
  begin
    if v_url is null or v_token is null then
      raise exception 'knowlu-mail-process: vault secret mail_process_url or mail_jobs_token is missing';
    end if;
    perform net.http_post(
      url                  := v_url,
      headers              := jsonb_build_object('content-type', 'application/json',
                                                 'x-knowlu-job-token', v_token),
      body                 := '{}'::jsonb,
      timeout_milliseconds := 60000
    );
  end
  $guard$;
  $job$
);

select cron.schedule(
  'knowlu-mail-canary',
  '11 * * * *',
  $job$
  do $guard$
  declare
    v_url   text := (select decrypted_secret from vault.decrypted_secrets
                     where name = 'mail_canary_url' order by created_at desc limit 1);
    v_token text := (select decrypted_secret from vault.decrypted_secrets
                     where name = 'mail_jobs_token' order by created_at desc limit 1);
  begin
    if v_url is null or v_token is null then
      raise exception 'knowlu-mail-canary: vault secret mail_canary_url or mail_jobs_token is missing';
    end if;
    perform net.http_post(
      url                  := v_url,
      headers              := jsonb_build_object('content-type', 'application/json',
                                                 'x-knowlu-job-token', v_token),
      body                 := '{}'::jsonb,
      timeout_milliseconds := 60000
    );
  end
  $guard$;
  $job$
);

select cron.schedule(
  'knowlu-mail-sweep',
  '47 7 * * *',
  $job$
  do $guard$
  declare
    v_url   text := (select decrypted_secret from vault.decrypted_secrets
                     where name = 'mail_process_url' order by created_at desc limit 1);
    v_token text := (select decrypted_secret from vault.decrypted_secrets
                     where name = 'mail_jobs_token' order by created_at desc limit 1);
  begin
    if v_url is null or v_token is null then
      raise exception 'knowlu-mail-sweep: vault secret mail_process_url or mail_jobs_token is missing';
    end if;
    perform net.http_post(
      url                  := v_url,
      headers              := jsonb_build_object('content-type', 'application/json',
                                                 'x-knowlu-job-token', v_token),
      body                 := '{"mode": "sweep"}'::jsonb,
      timeout_milliseconds := 60000
    );
  end
  $guard$;
  $job$
);
