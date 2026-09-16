-- §5.4 measure 2, as narrowed by ruling R-C2-4: the suite replays cases against the pinned model
-- on every prompt or pin change, and a regression past a per-kind threshold blocks the deploy.
--
-- `source` says where a case came from, and it is the honest record of the narrowing: 'seed' is
-- Task 13's frozen corpus, 'correction' is a real user correction that arrived WITH a replayable
-- request under the (c) opt-in. Until C4 ships that toggle, every row is 'seed'.
create table if not exists eval_cases (
  id       bigserial primary key,
  kind     text not null check (kind in ('task', 'event', 'email')),
  request  jsonb not null,
  ours     jsonb,
  theirs   jsonb not null,
  source   text not null check (source in ('correction', 'seed')),
  added_at timestamptz not null default now()
);
alter table eval_cases enable row level security;
create index if not exists eval_cases_kind on eval_cases (kind, source);

-- R-C2-E9: `input_tokens`/`output_tokens` are the run's OWN total token usage, summed from every
-- `ModelReply` the run produced for this kind — never metered against any account's cap. The eval
-- suite is not a user: `cloud/eval/run_eval.ts` passes an unmetered `CapStore` (`charge` and
-- `withinBudget` always true, `recordTokens` a no-op) and records its own spend here instead, so a
-- regression run's cost is visible without attributing it to anyone's monthly ceiling.
create table if not exists eval_runs (
  id              bigserial primary key,
  ran_at          timestamptz not null default now(),
  model           text not null,
  prompt_version  text not null,
  grammar_version text not null,
  prompt_hash     text not null,
  kind            text not null check (kind in ('task', 'event', 'email')),
  metric          text not null,
  value           real not null,
  threshold       real not null,
  passed          boolean not null,
  cases           integer not null,
  input_tokens    integer not null default 0,
  output_tokens   integer not null default 0
);
alter table eval_runs enable row level security;
create index if not exists eval_runs_recent on eval_runs (kind, metric, ran_at desc);

-- `eval_cases` and `eval_runs` carry no `account_id`: they are the eval suite's own global tables
-- (one shared corpus, one shared run history), never one account's rows, so there is nothing to
-- scope by. `_shared/judge_db_test.ts`'s account-scoping guard never reaches `cloud/eval/`'s calls
-- on these tables anyway — it scans `_shared/` and each function's own `index.ts`/`handler.ts`
-- only, and `cloud/eval/run_eval.ts` is neither.
--
-- `backfill_correction_judgments()` is NOT here: it lives in `20260911000400_rule_promotion.sql`
-- (not `...000300...` — that migration is `20260911000400`, transcribed correctly here), because
-- that migration's nightly job calls it and a function must exist before the schedule that names
-- it is trustworthy. This migration only reads it, from `cloud/eval/run_eval.ts`.
