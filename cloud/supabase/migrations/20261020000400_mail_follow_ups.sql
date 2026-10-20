-- Knowlu — email forwarding: the cloud follow-ups after PR #42
-- (docs/plans/2026-10-20-email-cloud-follow-ups-plan.md §2 and T2; Quinn's rulings Q3 and Q4, 2026-10-20).
--
-- Three things, each read or written only by the service role:
--   * `mail_addresses.rotated_from`: the status a mailbox had when Rotate sent it back to `pending` (Q4).
--     D22's reading counts a `pending` Gmail mailbox as forwarding while its Rotate grace is unexpired and
--     this status was on the allow-list, so the old forward, still delivering through the retired token,
--     keeps the Gmail read off and no message is filed twice.
--   * `mail_batches.counted_receipts` and `count_batch_receipt`: each SES receipt counts toward a batch's
--     `received` once. The id is the SHA-256 hex of the notification's object key, so no object key is
--     stored; an SNS redelivery carries the same key and is refused by the one guarded update.
--   * `mail_mark_lapsed_from` (Q3): an item `mail-process` holds for an account that is neither `active`
--     nor `trialing` marks its mailbox `lapsed` back to the item's arrival, so the device's reconcile
--     reaches back over it.
--
-- Forward-only: 20261002000200_mail.sql, 20261005000100, 20261019000100 and 20261020000100 are applied on
-- staging and never edited. No new table, so no new row-level security: both tables keep RLS on, no
-- policy, every client role revoked. Both columns are nullable or defaulted, and nothing deployed today
-- calls either function, so the functions as deployed keep working until they are redeployed.

-- Nullable: a mailbox never rotated has none. No check ties it to the grace (review r1, M7): D22 reads it
-- only while `retired_until` is in the future, so a stale value is harmless, and such a check would make
-- intake's grace-ending write fail between this file and intake's redeploy.
alter table public.mail_addresses add column if not exists rotated_from text;

-- Exactly the statuses `mail_addresses.status` admits.
alter table public.mail_addresses add constraint mail_addresses_rotated_from_status
  check (rotated_from is null or rotated_from in ('pending', 'confirmed', 'active', 'quiet', 'blocked', 'off', 'paused_send', 'lapsed', 'revoked'));

-- The receipt ids already counted toward `received`; empty for every batch that exists today.
alter table public.mail_batches add column if not exists counted_receipts text[] not null default '{}';

-- One count per SES receipt (plan §1.3). One statement: two deliveries of the same receipt at once
-- serialise on the row lock, and the second re-checks the `where` on the locked row and finds the id
-- there. A batch of another mailbox, a negative count or an id that is not a SHA-256 hex counts nothing.
-- Answers true only when it counted.
create or replace function public.count_batch_receipt(p_batch uuid, p_address uuid, p_receipt text, p_n integer)
returns boolean
language plpgsql
security invoker
set search_path = public, extensions
as $$
declare
  counted boolean;
begin
  update public.mail_batches
     set received = received + p_n, counted_receipts = counted_receipts || p_receipt
   where id = p_batch and address_id = p_address
     and p_n >= 0 and p_receipt ~ '^[0-9a-f]{64}$'
     and not (p_receipt = any (counted_receipts))
  returning true into counted;
  return coalesce(counted, false);
end;
$$;
revoke execute on function public.count_batch_receipt(uuid, uuid, text, integer) from public, anon, authenticated;
grant execute on function public.count_batch_receipt(uuid, uuid, text, integer) to service_role;

-- A held item's mailbox marked lapsed from the item's arrival (Q3). `least` skips a null, so an
-- unstamped row takes `p_since` and an earlier stamp is kept. A row already `lapsed` is rewritten only
-- when the stamp moves earlier; its `last_lapsed_at` then moves too, which voids a `covered` already in
-- flight (spec §4.11 step 3). `off` and `revoked` are never touched, as intake's own mark leaves them,
-- and a future instant is refused. Every write sets `last_lapsed_at`, so
-- `mail_addresses_last_lapse_stamped` holds. Answers true when it wrote.
create or replace function public.mail_mark_lapsed_from(p_address uuid, p_since timestamptz)
returns boolean
language plpgsql
security invoker
set search_path = public, extensions
as $$
declare
  marked boolean;
begin
  update public.mail_addresses
     set status = 'lapsed', lapsed_since = least(lapsed_since, p_since), last_lapsed_at = now()
   where id = p_address and status not in ('revoked', 'off') and p_since <= now()
     and (status <> 'lapsed' or lapsed_since is null or lapsed_since > p_since)
  returning true into marked;
  return coalesce(marked, false);
end;
$$;
revoke execute on function public.mail_mark_lapsed_from(uuid, timestamptz) from public, anon, authenticated;
grant execute on function public.mail_mark_lapsed_from(uuid, timestamptz) to service_role;
