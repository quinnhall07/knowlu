-- T6 — the calibration harness's query (stream J, `docs/plans/2026-09-22-judgment-quality-plan.md`
-- Task T6). Produces one row per judgment: enough to answer "does our `confidence` mean
-- anything?" per kind, grouped by the prompt that produced it.
--
-- **This file is never run by this harness.** Per the dispatch's own decision ("No database
-- access"), nothing under `cloud/eval/` opens a connection. Run this by hand (psql, or the
-- Supabase SQL editor) against a real project, save the result as NDJSON (one row per line — most
-- `psql \copy ... to ... (format text)` / Postgres `json_agg` exports, or a client library's
-- streaming cursor, give this for free) and hand that file to `calibration_cli.ts`. Column names
-- below match `CalibrationRow` in `calibration.ts` exactly, so no renaming step sits between the
-- query and the harness.
--
-- **What "wrong" means, and why it needs no comparison of `ours` against `theirs`.**
-- Field corrections reach `public.corrections` by exactly one path (the device's label rows, the
-- other path since F7, are described in the next paragraph): `app/src/telemetry.rs`'s `read_corrections`,
-- whose own comment is explicit — "a dashboard write over an `agent:` write is a correction, and
-- nothing else is" (`app/src/telemetry.rs:134`; the edge function's twin comment is
-- `cloud/supabase/functions/telemetry/handler.ts:10-13`). A correction row therefore never means
-- "the human confirmed this field" — it exists only because the human changed a value the agent
-- (here, a cloud judgment) had written. So the existence of a `corrections` row naming this
-- judgment IS the "wrong" label; there is nothing to compare `ours`/`theirs` against, and for the
-- two content fields (`course`, `title`) `ours`/`theirs` are null by construction anyway (spec §6's
-- content rule — a course name or a title is never written to the ledger, only "it changed").
-- A judgment with no matching correction is treated as correct BY ABSENCE OF EVIDENCE, not by
-- proof: nobody reviewed it and disagreed, which is the honest reading of what this table can say
-- and the one the T6 brief's own framing assumes ("a join gives pairs of claimed probability
-- against observed correctness").
--
-- **Two more row shapes, reported by the device (F7, engine follow-ups plan item (b)).** Since F7,
-- `/telemetry` also accepts label rows that carry the `judgment_id` they label, so the join below
-- finds them with no heuristic:
--   * `field = 'verdict'`, `judgment_kind = 'event'`: an `unsure` event the student answered on its
--     card, `ours = 'unsure'`, `theirs` the student's verdict. An abstention claims nothing, so an
--     `unsure` judgment is not a calibration point and is left out below; the answered label stays
--     in `corrections` for the event work that follows.
--   * `field = 'decision'`, `judgment_kind` `task` or `event`: an `amend` card from a judged write,
--     `ours = 'proposed'`, `theirs = 'rejected'` (a rejection IS "wrong"). The device never sends
--     an approval; `theirs = 'approved'` is excluded anyway, because agreeing is not a correction.
-- An `email` label row can never mark a judgment wrong here (Gmail Limited Use: email-derived
-- decisions train only that user's own model, never a cross-account aggregate). The handler
-- refuses one outright; the clause below is the second layer. And a correction marks only its own
-- account's judgment wrong (`c.account_id = j.account_id`), the second layer behind the handler's
-- ownership check on every device-supplied `judgment_id`.
--
-- **Restricted to tier-3, `outcome = 'answered'` rows.** Tier 2 (a promoted rule) writes a fixed
-- `confidence` of 1 by construction (`judge_pipeline.ts`'s rule branch: `Number(checked.verdict.
-- confidence ?? 1)`), so calibrating it is calibrating a constant — not what this query is for.
-- `outcome = 'low confidence'` rows wrote no verdict (`cause = 'below floor'`) and `'capped'` rows
-- made no model call at all, so neither carries a `confidence` a human ever acted on.
select
  j.id                                                  as judgment_id,
  j.account_id,
  j.kind,
  j.prompt_version,
  j.prompt_hash,
  j.confidence,
  -- The verdict itself, per `t6-brief.md`: "the verdict itself in `fields`" — event's decision key
  -- is `verdict` (`judge_validate.ts`'s `EVENT_VERDICTS`), email's is `tier` (`EMAIL_TIERS`); task
  -- has no single decision key (`effort_hours`/`importance`/`course` are three separate labelled
  -- fields), so this column is null for task rows and the task group is analysed on `confidence`
  -- and `wrong` alone.
  coalesce(j.fields ->> 'verdict', j.fields ->> 'tier') as verdict,
  j.judged_at,
  exists (
    select 1
      from public.corrections c
     where c.judgment_id = j.id
       and c.judgment_kind = j.kind
       and c.account_id = j.account_id
       and not (c.field in ('verdict','decision') and c.judgment_kind = 'email')
       and not (c.field = 'decision' and c.theirs = 'approved')
  )                                                      as wrong
  from public.judgments j
 where j.tier = 3
   and j.outcome = 'answered'
   and coalesce(j.fields ->> 'verdict', '') <> 'unsure'
 order by j.kind, j.prompt_version, j.prompt_hash, j.judged_at;
