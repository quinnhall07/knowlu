-- Knowlu C1, Task 1 — the account, its subscription, its consent log and the one source we store.
-- Spec §5.1. Three rules are load-bearing and are pinned by cloud/supabase/migrations_test.ts:
--   1. There is NO date-of-birth column, in this file or any other (spec §9, minors).
--   2. Row-level security is on for every table.
--   3. No table has a client write policy. Every write in this system goes through an edge function
--      that verified the caller's JWT and then used the service role, which bypasses RLS. That is
--      what makes "the Stripe webhook is the only writer of entitlements" enforced rather than
--      promised.
create extension if not exists pgcrypto with schema extensions;

create table public.accounts (
  id                 uuid primary key references auth.users (id) on delete cascade,
  email              text not null,
  created_at         timestamptz not null default now(),
  tos_version        text,
  tos_accepted_at    timestamptz,
  privacy_version    text,
  age_attested_at    timestamptz,
  stripe_customer_id text unique,
  status             text not null default 'active' check (status in ('active', 'closed'))
);
comment on table public.accounts is
  'Spec §5.1, exactly. No date-of-birth column exists, or ever may: the 18+ gate is an attestation.';

create table public.entitlements (
  account_id         uuid primary key references public.accounts (id) on delete cascade,
  plan               text,
  status             text not null check (status in ('active', 'trialing', 'past_due', 'canceled', 'none')),
  current_period_end timestamptz,
  source             text not null default 'stripe',
  updated_at         timestamptz not null default now()
);
comment on table public.entitlements is
  'Written ONLY by the stripe-webhook function, through the service role. GET /entitlement reads it.';

create table public.consents (
  id           bigint generated always as identity primary key,
  account_id   uuid references public.accounts (id) on delete set null,
  subject_hash text not null,
  kind         text not null check (kind in ('tos', 'privacy', 'age_18', 'auto_renew')),
  version      text not null,
  accepted_at  timestamptz not null default now(),
  price_cents  integer,
  ip           inet
);
comment on table public.consents is
  'Kept three years (California ARL as amended by AB 2863; ROSCA has no term). DELETE /account nulls
   both account_id and ip and leaves the row: subject_hash keeps the record meaningful without
   identifying anyone, which is how a legal-retention duty and a deletion right are both honoured.';

create table public.sources (
  account_id     uuid not null references public.accounts (id) on delete cascade,
  -- **Three kinds, all declared here, in C1's migration, on purpose.** `lms_ics` is the school's
  -- assignment feed and `calendar_ics` the student's own busy time — the secret iCal address Google
  -- Calendar hands out; the wizard asks for both on one panel (spec §11a). `google_calendar` is
  -- **reserved and written by nobody** (R-X-9): a Google grant has no URL for the two not-null
  -- columns, so it lives in C2's `google_accounts`, and `/ingest-calendar` resolves `google` from
  -- there. The value is in this constraint from the start so that C2 never has to alter a table in a C1-owned migration:
  -- the ownership list forbids it, and a check constraint is the one thing two streams cannot both
  -- edit safely. C1 writes rows of the first two kinds only.
  kind           text not null check (kind in ('lms_ics', 'calendar_ics', 'google_calendar')),
  url_ciphertext text not null,
  url_iv         text not null,
  added_at       timestamptz not null default now(),
  primary key (account_id, kind)
);
comment on table public.sources is
  'The two URL kinds are capability URLs: anyone holding one reads that student''s schedule — the school feed
   their assignments, the personal one their life. Both are AES-GCM encrypted by the account function
   before they arrive here, and GET /account/sources never returns either. google_calendar is reserved
   and never written: a Google grant has no URL and lives in google_accounts (C2).';

alter table public.accounts enable row level security;
alter table public.entitlements enable row level security;
alter table public.consents enable row level security;
alter table public.sources enable row level security;

create policy accounts_select_own on public.accounts
  for select to authenticated using (id = auth.uid());
create policy entitlements_select_own on public.entitlements
  for select to authenticated using (account_id = auth.uid());
create policy sources_select_own on public.sources
  for select to authenticated using (account_id = auth.uid());
-- `consents` has no policy at all, deliberately: a consent log a client cannot read is still a
-- consent log, and GET /account/export returns the caller's own rows through the service role.

create index consents_subject_idx on public.consents (subject_hash, accepted_at desc);

-- Stripe retries for days and does not guarantee order. Two guards, both cheap:
--   * an event id already seen is dropped before anything is written (idempotency);
--   * a write is refused when the event is OLDER than the row it would overwrite, so a late
--     `customer.subscription.updated` cannot resurrect a subscription `deleted` already cancelled.
-- `entitlements.updated_at` therefore carries the EVENT's own `created`, not `now()`, which is what
-- makes that comparison mean anything.
create table public.webhook_events (
  event_id    text primary key,
  event_type  text not null,
  created_at  timestamptz not null,
  received_at timestamptz not null default now()
);
alter table public.webhook_events enable row level security;
-- No policy at all: nothing but the service role has any business reading this table.

-- The account row, its empty entitlement and its consent records are written the moment the auth
-- user is, from the metadata the wizard sends with the sign-up. One round trip, and the attestation
-- cannot drift away from the user it belongs to.
--
-- The `raise exception` is the 18+ gate, server side: Alabama's Ala. Code § 26-1-1(f) makes an
-- 18-year-old's contract binding, a 17-year-old's is voidable, and the cheapest compliant path is to
-- decline rather than collect a date of birth. The app refuses first, with a sentence; this refuses
-- second, so a patched client gets an account it cannot use either.
create function public.handle_new_user() returns trigger
language plpgsql security definer set search_path = public, extensions as $$
declare
  attested boolean := (new.raw_user_meta_data ->> 'age_attested') = 'true';
  tos      text    := nullif(new.raw_user_meta_data ->> 'tos_version', '');
  priv     text    := nullif(new.raw_user_meta_data ->> 'privacy_version', '');
begin
  if not attested then
    raise exception 'age attestation required: Knowlu is for people 18 or older';
  end if;
  if tos is null or priv is null then
    raise exception 'the terms and the privacy policy must be accepted at sign-up';
  end if;

  insert into public.accounts (id, email, tos_version, tos_accepted_at, privacy_version, age_attested_at)
  values (new.id, new.email, tos, now(), priv, now())
  on conflict (id) do nothing;

  insert into public.entitlements (account_id, status) values (new.id, 'none')
  on conflict (account_id) do nothing;

  insert into public.consents (account_id, subject_hash, kind, version)
  select new.id,
         encode(extensions.digest(lower(new.email), 'sha256'), 'hex'),
         k.kind,
         k.version
  from (values ('tos', tos), ('privacy', priv), ('age_18', '1')) as k(kind, version);

  return new;
end;
$$;

create trigger on_auth_user_created
  after insert on auth.users
  for each row execute function public.handle_new_user();
