-- F1, live proof 2026-10-02. A dead provider key made every one of a first day's 160 event
-- judgments fail in a few hundred milliseconds, and `charge_call` had counted every one of them, so
-- the account reached its first-day event allowance on calls the model never answered and was
-- `capped` for the rest of the day: an outage spent a student's day.
--
-- `charge_call` still charges BEFORE the call — that is the hot-loop guard, and it is unchanged.
-- `refund_call` gives one call back after a call the model never answered (transport, auth,
-- provider or timeout failure, an error envelope, an unparseable reply: the pipeline's
-- `model failed`). A refusal, a truncation or an answer that fails validation reached the model and
-- stays charged. Called only from `_shared/judge_caps.ts`'s `capStore.refund`.
--
-- Why refunds are bounded: without a bound, a device looping on a failing provider would charge,
-- fail and refund forever, and `DAILY_CAP` exists to bound exactly that request volume. So a day
-- refunds at most the same allowance `charge_call` grants (the cap, doubled on an account's first
-- two judging days — the same rule as `20260923000100`, restated here). A failing loop therefore
-- reaches the model at most twice the allowance in a day, and an outage up to one allowance long
-- costs the student nothing.
--
-- One `update`, never an insert: the row lock serialises concurrent refunds against each other and
-- against `charge_call`'s upsert, and Postgres re-checks the `where` on the locked row, so `calls`
-- never goes below 0 and `refunds` never passes the allowance. `in_tokens`/`out_tokens` are never
-- touched — they are what `monthly_spend` prices, and a failed call recorded none.
--
-- `current_date` is the database's day, as in `charge_call`. A call charged just before UTC
-- midnight and refunded just after it gives back one call of the new day (or nothing, if the new
-- day has no row yet) — one call, once, at a day boundary.
alter table usage_daily add column if not exists refunds integer not null default 0;

create or replace function refund_call(p_account uuid, p_kind text, p_cap integer)
returns boolean
language plpgsql
security invoker
set search_path = public, extensions
as $$
declare
  first_day date;
  allowance integer;
  done      boolean;
begin
  select min(day) into first_day from usage_daily where account_id = p_account;
  allowance := case when first_day is null or first_day >= current_date - 1 then p_cap * 2 else p_cap end;

  update usage_daily u
     set calls = greatest(u.calls - 1, 0),
         refunds = u.refunds + 1
   where u.account_id = p_account
     and u.day = current_date
     and u.kind = p_kind
     and u.calls > 0
     and u.refunds < allowance
  returning true into done;

  return coalesce(done, false);
end;
$$;

revoke execute on function public.refund_call(uuid, text, integer) from public, anon, authenticated;
grant execute on function public.refund_call(uuid, text, integer) to service_role;
