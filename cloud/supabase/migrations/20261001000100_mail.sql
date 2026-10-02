-- Knowlu email forwarding (spec docs/specs/2026-09-30-email-forwarding-design.md §5.1), T1.1: the
-- seven tables. T1.2 and T1.3 extend this same file (kinds, origins, RPCs, budget, cron) before it
-- is applied anywhere; nothing here edits an applied migration.
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
  -- D23's backfill allowance: keyed by the start it was opened for, so Disconnect and reconnect never
  -- renew it. Zero until T1.3's RPC opens it: a closed allowance is the default, never an open one.
  allowance_term_start date,
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
  term_start          date,
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
