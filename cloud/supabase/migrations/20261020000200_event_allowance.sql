-- L3, the first-day event allowance (spec docs/specs/2026-10-06-first-day-event-allowance-design.md
-- §2, §3 step 3, §5; ruled 2026-10-06). A new student's first event backlog is judged in three to
-- four days instead of about two weeks: once per account, 2,000 event judgments spent only on the
-- event calls the daily cap refuses (`judge_pipeline.ts` charges `charge_call` first and this
-- allowance only on overflow), open for 14 days from that first overflow, never renewed.
--
-- One row per `(account_id, kind)`, `kind` checked to 'event' alone (spec Q4). The row is created by
-- the first charge and is never deleted while the account lives: no migration and no function removes
-- it, only the `on delete cascade` from `accounts` when the account is purged. That is what makes the
-- allowance one-time: the opening test is "no row", never "row expired". `calls_left` is the spec's
-- `left`, renamed because LEFT is a reserved word (plan Q-D).
--
-- Usage metering of the same kind as `usage_daily`: an opening time, an end time and two counters
-- per account, and nothing else. No grades, coursework, event or note content, nothing from a mailbox.
-- RLS on with no policy, and every client grant revoked: only the service role reads or writes it.
--
-- `charge_call`, `refund_call` and `enforce_budget` are not re-issued here (spec Q5, Q6): the
-- first-two-days doubling and the $2 ceiling are unchanged, and the ceiling is checked before either
-- charge, so an account already over it never opens the allowance.
create table public.judge_allowances (
  account_id uuid not null references public.accounts (id) on delete cascade,
  kind       text not null check (kind in ('event')),
  opened_at  timestamptz not null,
  until      timestamptz not null,
  calls_left integer not null check (calls_left >= 0),
  refunds    integer not null default 0 check (refunds >= 0),
  primary key (account_id, kind)
);
alter table public.judge_allowances enable row level security;
revoke all on public.judge_allowances from anon, authenticated;

-- One event judgment charged to the account's allowance, after `charge_call` refused it. One
-- statement on the primary key, never an update followed by an insert: two concurrent first charges
-- race to insert the same key, the second waits on it and then takes the `do update` arm, and
-- Postgres re-checks that arm's `where` on the locked row. So the allowance opens exactly once and
-- `calls_left` never goes below 0. With no row, the insert opens it (now, 14 days, 2000 - 1 left) and
-- charges its first judgment. With a row, it charges one only while `until` has not passed and
-- something is left; a spent or lapsed row answers false and is never reopened, however old it is.
-- The row's own columns stay qualified in the conflict arm, where `excluded` is in scope too.
create or replace function public.charge_judge_allowance(p_account uuid, p_kind text)
returns boolean
language plpgsql
security invoker
set search_path = public, extensions
as $$
declare
  charged boolean;
begin
  insert into public.judge_allowances (account_id, kind, opened_at, until, calls_left, refunds)
  values (p_account, p_kind, now(), now() + interval '14 days', 2000 - 1, 0)
  on conflict (account_id, kind) do update
    set calls_left = judge_allowances.calls_left - 1
    where judge_allowances.until > now()
      and judge_allowances.calls_left > 0
  returning true into charged;
  return coalesce(charged, false);
end;
$$;
revoke execute on function public.charge_judge_allowance(uuid, text) from public, anon, authenticated;
grant execute on function public.charge_judge_allowance(uuid, text) to service_role;

-- One event judgment given back after an allowance-charged call the model never answered (the
-- pipeline's `model failed`, F1's rule): a refusal, a truncation or a failed validation reached the
-- model and stays charged. It gives back only while the allowance is open, never past its size in
-- `calls_left`, and never more than its size over its life in `refunds`. Each `/judge-event` request
-- is its own invocation, so F1's breaker never trips across a run; the lifetime bound is what stops a
-- device on a dead provider charging, failing and refunding forever, so the allowance reaches the
-- model at most twice its size. It never opens a missing row and never moves `opened_at` or `until`.
-- The row lock serialises it against `charge_judge_allowance`.
create or replace function public.refund_judge_allowance(p_account uuid, p_kind text)
returns boolean
language plpgsql
security invoker
set search_path = public, extensions
as $$
declare
  done boolean;
begin
  update public.judge_allowances
     set calls_left = calls_left + 1,
         refunds = refunds + 1
   where account_id = p_account
     and kind = p_kind
     and until > now()
     and calls_left < 2000
     and refunds < 2000
  returning true into done;
  return coalesce(done, false);
end;
$$;
revoke execute on function public.refund_judge_allowance(uuid, text) from public, anon, authenticated;
grant execute on function public.refund_judge_allowance(uuid, text) to service_role;
