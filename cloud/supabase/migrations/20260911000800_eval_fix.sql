-- C2 Task 14 fix 1 (R-C2-E51): `20260911000700_eval.sql` is applied on staging and must not be
-- edited — this migration adds what fix round 1 needs instead.

-- `--load-seed` must be idempotent: re-running it after a partial load (or a re-run of the same
-- CI job) must never insert the same seed record twice. `seed_id` is the seed file's own `id`
-- (`SeedRecord.id`, always `seed-...`, Task 13's `schema.ts`), unique only among `source = 'seed'`
-- rows — a correction row (once C4 ships) carries no `seed_id` at all, so the partial index leaves
-- it alone.
alter table eval_cases add column if not exists seed_id text;
create unique index if not exists eval_cases_seed_id on eval_cases (seed_id) where source = 'seed';

-- Every `eval_runs` row from ONE run of `run_eval.ts` shares one `run_id`, minted once per process
-- (`crypto.randomUUID()`), so a later reader can group a run's per-kind, per-metric rows back
-- together without guessing from `ran_at` alone. `dry_run` records whether that run spent real
-- money.
alter table eval_runs
  add column if not exists run_id uuid not null default gen_random_uuid(),
  add column if not exists dry_run boolean not null default false;

-- `input_tokens`/`output_tokens` (added in `20260911000700_eval.sql`) are per KIND, repeated on
-- every metric row of that kind — summing them requires `sum over distinct (run_id, kind)`, never
-- a plain `sum(input_tokens)` across a run's rows, or a run with three metrics for one kind would
-- triple-count its own spend.
comment on column eval_runs.input_tokens is
  'per kind, repeated on every metric row of that kind — sum over distinct (run_id, kind)';
comment on column eval_runs.output_tokens is
  'per kind, repeated on every metric row of that kind — sum over distinct (run_id, kind)';
