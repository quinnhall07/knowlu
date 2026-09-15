-- Knowlu C2, Task 12 fix round 1 (ruling R-C2-E47). 20260911000400_rule_promotion.sql is already
-- applied on staging and stays exactly as it shipped (migrations are forward-only); every
-- correction below is a `create or replace function` re-issue plus the privilege and schedule
-- statements that only a re-issue can carry.
--
-- What this corrects, and why:
--
--   C1 (CRITICAL) — `backfill_correction_judgments()`'s `UPDATE ... FROM LATERAL` correlated
--     subquery raises 42P10 ("could not identify an equality operator") on real Postgres, confirmed
--     live on staging. Because the nightly cron job is ONE statement
--     (`select backfill_correction_judgments(), promote_rules();`), that error aborted the whole
--     statement and took `promote_rules()` down with it, every night, silently (`judge`'s
--     always-exit-0 rule and pg_cron's own silence mean nothing surfaces this from the app side).
--     Fixed below with a derived table (`distinct on`), which Postgres can actually plan, and the
--     nightly job is re-scheduled as two schema-qualified statements in one transaction — schema
--     qualified because pg_cron runs jobs under its own `search_path`, not this migration's;
--     deliberately still ONE job, not two, because the contradiction guard in `promote_rules()`
--     joins on the column the backfill writes (`corrections.judgment_id`), so a backfill that
--     fails must also stop promotion rather than let promotion run against half-filled data.
--
--   I1 — `judgment_features`, `backfill_correction_judgments` and `promote_rules` are none of them
--     SECURITY DEFINER, so the original migration's silence on `revoke execute` looked safe — but
--     "safe" here was only ever a property of `judgments`/`rules`/`rule_evidence` having RLS
--     enabled with no policy, not a property of the functions themselves, and nothing stops these
--     three being called directly over PostgREST with the public anon key today. Revoked from
--     `public`, `anon` and `authenticated` and granted to `service_role` only, uniformly: no C2 SQL
--     function is reachable from a browser.
--
--   I2 — the "already proposed" guard (`not exists (... and (r.active or r.decided_at is null))`)
--     matched neither state of a REJECTED proposal (not active, and decided_at IS NOT null), so a
--     rejected rule was promoted again on every run of `promote_rules()` that still saw 3 agreeing
--     judgments and no contradiction. Fixed: a prior row for the same
--     `(account_id, kind, feature, value)` now also blocks while `decided_at` is within the last 90
--     days, so a rejection is respected for three months before the pattern can be re-proposed.
--
--   I3 — the nightly expiry delete (`delete from rules where active = false and decided_at is null
--     and expires_at < current_date`) carried no `scope` filter, so it would also sweep up
--     `scope = 'global'` candidates awaiting hand review (§11 R5) once they passed 30 days old.
--     Scoped to `scope = 'account'` — the only scope this job ever writes, and the only scope it
--     should ever delete.
--
--   M1 — `judgment_features`'s `created_by+title_prefix` row passed its own `<> '' and <> '|'`
--     filter even when `title_prefix` alone was empty (`"zybooks|"` is neither), promoting on a key
--     `_shared/judge_rules.ts`'s `features()` can never look up — that TypeScript twin only emits
--     the pair when BOTH `created_by` and the title prefix are non-empty. Fixed with the same
--     both-halves-present requirement.
--   M2 — the agreement count was `count(*)`, i.e. judgment ROWS, not distinct items; the same task
--     re-judged (a correction, a re-ingest) could pass 3 by counting one item three times. Now
--     `count(distinct item_id)`.
--   M3 — the representative answer's tie-break (`array_agg(... order by judged_at desc)`) was not a
--     total order: two judgments at the same `judged_at` left the pick non-deterministic between
--     runs. `id desc` added as the tie-break.
--   M11 — an all-empty verdict (`'{}'::jsonb`, e.g. every judged field already null) could still be
--     promoted if it agreed 3 times; excluded explicitly.
--   M4 — the original migration's own doc comment overclaimed: "two 2.5-hour answers and one 2.0
--     would never promote" is not what dropping the feature keys from `verdict_fields` fixed —
--     `distinct_answers = 1` is still exact `jsonb` equality on the verdict fields, so a 2.5/2.5/2.0
--     split still fails to agree today, correctly. What dropping the feature keys actually fixed is
--     narrower: a task whose `title_prefix` or `created_by` drifted between judgments (the feature
--     KEYS themselves, which are no longer part of the equality) can still agree on the verdict.
--     Restated correctly in the re-issued function's own doc comment below, which supersedes
--     20260911000400's now-inaccurate one; that file is not edited (forward-only).
--
-- F16's fix (promoted-rule count, not evidence-row count) and the R5 comment (no `scope = 'global'`
-- row is ever written or activated here) both carry forward unchanged in spirit, restated below.

-- M1: the `created_by+title_prefix` feature now requires BOTH halves non-empty, matching
-- `_shared/judge_rules.ts`'s `features()` exactly — a rule promoted on `"zybooks|"` could never be
-- looked up, because the TypeScript twin never emits that pair with an empty title prefix.
create or replace function judgment_features(p_kind text, p_fields jsonb)
returns table (feature text, value text)
language sql
immutable
set search_path = public, extensions
as $$
  select f.feature, f.value
    from (values
      ('created_by+title_prefix', (p_fields->>'created_by') || '|' || coalesce(p_fields->>'title_prefix', '')),
      ('title_prefix',            p_fields->>'title_prefix'),
      ('organizer',               p_fields->>'organizer'),
      ('source',                  p_fields->>'source'),
      ('series',                  p_fields->>'series_uid')
    ) as f(feature, value)
   where f.value is not null and f.value <> '' and f.value <> '|'
     and (f.feature <> 'created_by+title_prefix'
          or (coalesce(p_fields->>'created_by', '') <> '' and coalesce(p_fields->>'title_prefix', '') <> ''))
     and ((p_kind = 'task' and f.feature in ('created_by+title_prefix', 'title_prefix'))
       or (p_kind <> 'task' and f.feature in ('organizer', 'source', 'series', 'title_prefix')));
$$;
revoke execute on function public.judgment_features(text, jsonb) from public, anon, authenticated;
grant execute on function public.judgment_features(text, jsonb) to service_role;

-- C1 (CRITICAL, 42P10 confirmed live on staging): the earlier `UPDATE ... FROM LATERAL (...) j`
-- correlated the lateral subquery's `c.account_id`/`c.item_id`/`c.ts` against the OUTER `corrections
-- c` row-by-row in a way Postgres could not plan an equality operator for. Rewritten as a derived
-- table joined by primary key instead: `distinct on (c2.id)` picks the one most-recent-then-highest-id
-- matching judgment per correction, exactly as the lateral form intended, but as an ordinary join
-- Postgres can actually execute.
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
       order by c2.id, j2.judged_at desc, j2.id desc
    ) j
   where c.id = j.correction_id;
  get diagnostics filled = row_count;
  return filled;
end;
$$;
revoke execute on function public.backfill_correction_judgments() from public, anon, authenticated;
grant execute on function public.backfill_correction_judgments() to service_role;

-- §5.4 measure 1, re-issued: I2 (a rejected proposal no longer re-promotes for 90 days), I3 (the
-- expiry sweep is scoped to account rules only), M2 (agreement counts distinct ITEMS, not judgment
-- rows), M3 (the representative answer's tie-break is now a total order), M11 (an empty verdict
-- cannot be promoted). F16's shape (the return value counts FRESH rules, never evidence rows, via
-- an unreferenced-but-always-executed data-modifying `evidence` CTE) is unchanged.
--
-- **Only account-scoped rules are ever promoted here.** Cross-account (global) promotion is
-- hand-reviewed before activation (§11 R5), and there is deliberately no code path in this
-- function, or anywhere else in this stream, that inserts or activates a `scope = 'global'` row.
--
-- M4 (correcting 20260911000400's own doc comment, which is not edited — forward-only): dropping
-- the feature keys (`created_by`, `title_prefix`, `organizer`, `source`, `series_uid`) from the
-- equality fixes ONLY prefix/organizer/source drift between judgments — a task whose title prefix
-- or attribution changed between judgments can still agree on its verdict. It does NOT relax the
-- verdict comparison itself: `distinct_answers = 1` is still exact `jsonb` equality on the verdict
-- fields, so 2.5, 2.5 and 2.0 hours still do not agree, correctly — §5.4 never asked for a fuzzy
-- match, only for the promotion key and the agreement key to be different things.
create or replace function promote_rules()
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
       and j.judged_at >= now() - interval '60 days'
  ),
  agreed as (
    -- M2: `count(distinct item_id)`, not `count(*)` — a re-judged item (a correction, a re-ingest)
    -- must not count toward the 3-agreement threshold more than once.
    select account_id, kind, feature, value,
           count(distinct item_id) as items,
           count(distinct verdict_fields) as distinct_answers,
           -- M3: `id desc` is the total-order tie-break — `judged_at desc` alone is not total, so
           -- two judgments at the same timestamp left the representative answer non-deterministic.
           (array_agg(verdict_fields order by judged_at desc, id desc))[1] as answer,
           array_agg(id) as judgment_ids
      from judged
     group by account_id, kind, feature, value
  ),
  contradicted as (
    -- A correction against THIS feature and value, not merely against this kind.
    --
    -- Joined on `c.judgment_id`, which C2's own `backfill_correction_judgments()` fills (ruling
    -- R-X-11) — never on `judgment_kind`, which that back-fill is what *writes*. A correction that
    -- has not been back-filled yet simply does not block a promotion this run; the nightly job
    -- runs the back-fill first, so in practice it always has.
    select distinct j.account_id, j.kind, f.feature, f.value
      from public.corrections c
      join judgments j on j.id = c.judgment_id
      cross join lateral judgment_features(j.kind, j.fields) as f(feature, value)
     where c.ts >= now() - interval '60 days'
  ),
  fresh as (
    insert into rules (account_id, scope, kind, feature, value, verdict, active, proposed_at, expires_at)
    select a.account_id, 'account', a.kind, a.feature, a.value, a.answer, false,
           current_date, current_date + 30
      from agreed a
     where a.items >= 3
       and a.distinct_answers = 1
       -- M11: an all-empty verdict is not a rule worth promoting.
       and a.answer <> '{}'::jsonb
       and not exists (
             select 1 from contradicted x
              where x.account_id = a.account_id and x.kind = a.kind
                and x.feature = a.feature and x.value = a.value)
       -- I2: a prior row for this exact (account, kind, feature, value) blocks promotion while it
       -- is active, OR still undecided, OR was decided (approved or REJECTED) within 90 days — not
       -- only while `active or decided_at is null`, which matched neither state of a rejection and
       -- let a rejected pattern re-promote on the very next run.
       and not exists (
             select 1 from rules r
              where r.account_id = a.account_id and r.kind = a.kind
                and r.feature = a.feature and r.value = a.value
                and (r.active or r.decided_at is null or r.decided_at > now() - interval '90 days'))
    returning id, account_id, kind, feature, value
  ),
  -- F16: `promoted` counts FRESH rules, not evidence rows. `evidence` is a data-modifying CTE
  -- PostgreSQL always executes to completion even though nothing below references its output by
  -- name (documented behaviour for a data-modifying WITH entry, unlike a plain SELECT one), so it
  -- still runs exactly once here; the final `select count(*) into promoted from fresh` counts the
  -- rules themselves instead of the (almost always larger) number of agreeing judgments.
  evidence as (
    insert into rule_evidence (rule_id, judgment_id, agrees)
    select f.id, unnest(a.judgment_ids), true
      from fresh f
      join agreed a
        on a.account_id = f.account_id and a.kind = f.kind
       and a.feature = f.feature and a.value = f.value
  )
  select count(*) into promoted from fresh;

  -- I3: scoped to `scope = 'account'` — this job never writes a global row, so it must never sweep
  -- one either. A `scope = 'global'` candidate is hand-reviewed (§11 R5) and stays past 30 days.
  -- An undecided account proposal expires rather than waiting forever; a re-promotion is a new one.
  delete from rules where scope = 'account' and active = false and decided_at is null and expires_at < current_date;
  return promoted;
end;
$$;
revoke execute on function public.promote_rules() from public, anon, authenticated;
grant execute on function public.promote_rules() to service_role;

-- C1(b): re-scheduled as two schema-qualified statements in one transaction. pg_cron runs the job
-- text under its OWN `search_path`, not this migration's `set search_path`, so an unqualified
-- `backfill_correction_judgments()`/`promote_rules()` call risks "function does not exist" outside
-- this session — schema-qualified here removes that risk entirely. Two statements, not one target
-- list (`select backfill_correction_judgments(), promote_rules();`), because a two-call target list
-- has unspecified evaluation order in Postgres; still ONE job, so both run in the same transaction
-- — the coupling is deliberate: the contradiction guard above joins on the column the backfill
-- writes, so a failed backfill must also stop promotion, not let it run against half-filled data.
select cron.unschedule('knowlu-promote-rules');
select cron.schedule('knowlu-promote-rules', '17 7 * * *', $$select public.backfill_correction_judgments(); select public.promote_rules();$$);
