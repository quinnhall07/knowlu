-- §5.4 measure 1: a feature whose model verdicts agree at least 3 times, with NO correction
-- against THAT FEATURE in 60 days, becomes a `kind: rule` proposal. Approval writes the rule row;
-- tier 2 then answers without the model, which is the whole point — the loop retires model calls,
-- and it is also the only structural answer to the per-account cost problem.
--
-- **Only account-scoped rules are ever promoted here.** Cross-account (global) promotion is
-- hand-reviewed before activation (§11 R5), and there is deliberately no code path in this
-- function, or anywhere else in this stream, that inserts or activates a `scope = 'global'` row.
--
-- `judgment_features` is the SQL twin of `_shared/judge_rules.ts`'s `features()`; a rule promoted
-- on one key and looked up by another never fires, so `judge_rules_test.ts` pins them together.
-- It reads `judgments.fields`, which is where `featureMap` put them — the row carries no title and
-- no body to recompute them from, and that is the design.
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
     and ((p_kind = 'task' and f.feature in ('created_by+title_prefix', 'title_prefix'))
       or (p_kind <> 'task' and f.feature in ('organizer', 'source', 'series', 'title_prefix')));
$$;

-- `corrections.judgment_id` AND `judgment_kind` are both filled here, by C2, and by nothing else
-- (ruling R-X-11). C1 sends neither and needs to know nothing about them: it writes the correction
-- from the journal, and this joins it to the judgment it is about.
--
-- **The join is `(account_id, item_id)` and the time order** — deliberately NOT `judgment_kind`,
-- which is the column being filled and would make this a no-op forever. The judgment's own `kind`
-- is what `judgment_kind` becomes, which is also why it cannot be a join key: it is the answer.
-- `item_id` is the note's opaque id, the event uid, or `gmail:<message-id>` on both sides.
--
-- Run before every eval, and cheap enough to run nightly beside the promotion.
create or replace function backfill_correction_judgments()
returns integer
language plpgsql
security invoker
set search_path = public, extensions
as $$
declare
  filled integer := 0;
begin
  update public.corrections c
     set judgment_id = j.id,
         judgment_kind = j.kind
    from lateral (
      select j2.id, j2.kind
        from judgments j2
       where j2.account_id = c.account_id
         and j2.item_id = c.item_id
         and j2.judged_at < c.ts
       order by j2.judged_at desc
       limit 1
    ) j
   where c.judgment_id is null;
  get diagnostics filled = row_count;
  return filled;
end;
$$;

/*
 * Four things this function had to get right, and the earlier draft got wrong:
 *
 *  (a) `min(j.fields)` — PostgreSQL has no `min()` aggregate for `jsonb`, so that version errored
 *      on its first call and rule promotion never ran at all. The representative answer is now
 *      `(array_agg(j.fields order by j.judged_at desc))[1]`: deterministic, and the most recent.
 *  (b) The agreement test was `count(distinct j.fields) = 1`, i.e. byte-equality of the whole
 *      field map — two 2.5-hour answers and one 2.0 would never promote, and neither would any
 *      task whose title prefix drifted. §5.4 asks for agreement on the VERDICT, so the test is now
 *      over the verdict fields only (`fields - 'created_by' - 'title_prefix' - …`).
 *  (c) The contradiction guard joined on `(account_id, kind)`, so a single correction of any task
 *      in 60 days blocked EVERY task-rule promotion for that account — permanently true for any
 *      active user. §5.4 says "no disagreement" on THAT FEATURE, so it joins on the feature and
 *      value now, through the same `judgment_features` the promotion uses.
 *  (d) `rule_evidence` was created and written by nothing. It is populated here, in the same
 *      statement, so "why does the model never get asked about this?" has an answer.
 */
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
    select j.id, j.account_id, j.kind, j.judged_at, j.fields,
           j.fields - 'created_by' - 'title_prefix' - 'organizer' - 'source' - 'series_uid' as verdict_fields,
           f.feature, f.value
      from judgments j
      cross join lateral judgment_features(j.kind, j.fields) as f(feature, value)
     where j.tier = 3
       and j.outcome = 'answered'
       and j.judged_at >= now() - interval '60 days'
  ),
  agreed as (
    select account_id, kind, feature, value,
           count(*) as hits,
           count(distinct verdict_fields) as distinct_answers,
           (array_agg(verdict_fields order by judged_at desc))[1] as answer,
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
     where a.hits >= 3
       and a.distinct_answers = 1
       and not exists (
             select 1 from contradicted x
              where x.account_id = a.account_id and x.kind = a.kind
                and x.feature = a.feature and x.value = a.value)
       and not exists (
             select 1 from rules r
              where r.account_id = a.account_id and r.kind = a.kind
                and r.feature = a.feature and r.value = a.value
                and (r.active or r.decided_at is null))
    returning id, account_id, kind, feature, value
  ),
  -- Ruling F16: `promote_rules()` returns the count of FRESH rules, not evidence rows. A plain
  -- `get diagnostics … = row_count` after this statement would report the LAST data-modifying
  -- statement it ran — this `evidence` insert, one row per (rule, agreeing judgment), which is
  -- almost always more than one row per rule and would over-report "promoted" by a large factor.
  -- `evidence` is a data-modifying CTE that PostgreSQL always executes to completion even though
  -- nothing below references its output by name (that is the documented behaviour for a
  -- data-modifying WITH entry, unlike a plain SELECT one), so it still runs exactly once here; the
  -- final `select count(*) into promoted from fresh` below counts the rules themselves instead.
  evidence as (
    insert into rule_evidence (rule_id, judgment_id, agrees)
    select f.id, unnest(a.judgment_ids), true
      from fresh f
      join agreed a
        on a.account_id = f.account_id and a.kind = f.kind
       and a.feature = f.feature and a.value = f.value
  )
  select count(*) into promoted from fresh;

  -- An undecided proposal expires rather than waiting forever; a re-promotion is a new proposal.
  delete from rules where active = false and decided_at is null and expires_at < current_date;
  return promoted;
end;
$$;

-- Nightly, once, for every account at the same time: promotion is cheap and per-account scheduling
-- would be a second thing to keep true. `pg_cron` was enabled in 20260911000100.
-- The back-fill runs FIRST, in the same statement, because the contradiction guard joins on the
-- column it fills. Two schedules would be two things to keep in order.
-- One line, not the brief's own three: `judge_rules_test.ts` pins the job name and the call
-- together as one substring (`cron.schedule('knowlu-promote-rules'`), which a line break between
-- them would fail even though Postgres itself does not care where the newlines fall.
select cron.schedule('knowlu-promote-rules', '17 7 * * *', $$select backfill_correction_judgments(), promote_rules();$$);
