-- L8 friends testers, the S6 review's follow-up (2026-10-20, minors 2 and 3; docs/specs/
-- 2026-10-18-friends-testers-design.md §3.3, §5, §8.3). Forward-only: 20261020000500_testers.sql is
-- applied on staging and prod and stays as it is. This file redefines two of its grant writers and
-- one of its views, each byte for byte as that file has it but for the lines named below, and
-- restates their revokes (create or replace keeps an object's rights; the restated revoke is the one
-- the corpus guards read for a definition in this file).
--
-- 1. admin.comp_grant took any end date. A past one made an open grant that was already expired and
--    held comp_grants_one_open, blocking the next grant; a mistyped year made a grant that never
--    ends. It now refuses an ends_at not after now(), or later than now() + 1 year, before it writes
--    anything, so a refusal writes no grant and no audit row. admin.comp_extend, which already
--    refused a date not in the future, takes the same ceiling.
-- 2. admin.testers' paid is true only for active or trialing, so a tester whose subscription is
--    past_due (Stripe's past_due, unpaid and incomplete, as the webhook maps them) read paid unset
--    while Stripe dunned them. The view gains, at its end, the entitlement status and whether the
--    row carries a Stripe subscription; every existing column keeps its name, place and type.

-- A new grant (000500's, with the end-date guards first).
create or replace function admin.comp_grant(p_account_id uuid, p_ends_at timestamptz, p_cohort text, p_note text)
returns uuid
language plpgsql
set search_path = ''
as $$
declare
  v_id uuid;
begin
  if p_ends_at is null or p_ends_at <= now() then
    raise exception 'the ends_at is not in the future';
  end if;
  if p_ends_at > now() + interval '1 year' then
    raise exception 'the ends_at is more than a year away';
  end if;
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

-- A new end date for the open grant (000500's, with the ceiling after its past-date check).
create or replace function admin.comp_extend(p_account_id uuid, p_ends_at timestamptz)
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
  if p_ends_at > now() + interval '1 year' then
    raise exception 'the new ends_at is more than a year away';
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

-- Closed to every role but the owner, as 000500 has it (A7).
revoke execute on function admin.comp_grant(uuid, timestamptz, text, text) from public, anon, authenticated, service_role;
revoke execute on function admin.comp_extend(uuid, timestamptz) from public, anon, authenticated, service_role;

-- The list Quinn grants and ends from (000500's, with two columns at its end). entitlement_status is
-- the entitlements row's status, none when the row is missing (paid's own coalesce); has_subscription
-- is whether that row carries a Stripe subscription id. §8.3 refuses a grant while has_subscription
-- is true and the status is not canceled.
create or replace view admin.testers as
select g.account_id,
       a.email,
       g.cohort,
       case when g.ended_at is not null then 'ended'
            when now() >= g.ends_at then 'expired'
            else 'live' end as state,
       coalesce(e.status, 'none') in ('active', 'trialing') as paid,
       g.granted_at,
       g.ends_at,
       greatest((select max(u.day) from public.usage_daily u where u.account_id = g.account_id),
                (select max(r.day) from public.route_daily r where r.account_id = g.account_id)) as last_active_day,
       coalesce(e.status, 'none') as entitlement_status,
       e.stripe_subscription_id is not null as has_subscription
  from public.comp_grants g
  left join public.accounts a on a.id = g.account_id
  left join public.entitlements e on e.account_id = g.account_id;

-- Owner rights, as 000500 has it; the admin schema is already closed to every API role.
revoke all on admin.testers from public, anon, authenticated, service_role;
