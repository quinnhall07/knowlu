-- Knowlu C1c Task 7 (ruling R-C1c-7, 2026-09-23). Both live proofs of the first day showed the
-- same thing: a new account's first slot judges a whole semester at once — every task, every
-- calendar event and the email backlog — so the steady-state cap cut day one's run off partway
-- through before it ever reached steady state. Quinn ruled: double the cap for an account's first
-- two judging days. `DAILY_CAP` in `_shared/judge_caps.ts` is unchanged — it stays the steady-state
-- number — and this migration alone decides the first-days allowance.
--
-- `current_date` here is the database's own day, the same UTC day `usage_daily`'s single upsert
-- keys on below, not the caller's clock. The window this doubles is that first judging day and the
-- day after it, so an evening onboarding in the Americas is not cut off at UTC midnight partway
-- through its first day. `create or replace` keeps this function's existing owner and ACL, and a
-- refused call still increments `calls`, exactly as before — only the ceiling it is compared
-- against moves.
create or replace function charge_call(p_account uuid, p_kind text, p_cap integer)
returns boolean
language plpgsql
security invoker
set search_path = public, extensions
as $$
declare
  after     integer;
  first_day date;
begin
  select min(day) into first_day from usage_daily where account_id = p_account;

  insert into usage_daily (account_id, day, kind, calls)
       values (p_account, current_date, p_kind, 1)
  on conflict (account_id, day, kind)
    do update set calls = usage_daily.calls + 1
  returning calls into after;

  if first_day is null or first_day >= current_date - 1 then
    return after <= p_cap * 2;
  end if;
  return after <= p_cap;
end;
$$;

revoke execute on function public.charge_call(uuid, text, integer) from public, anon, authenticated;
grant execute on function public.charge_call(uuid, text, integer) to service_role;
