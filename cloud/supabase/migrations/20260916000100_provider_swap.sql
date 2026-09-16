-- Knowlu — the inference provider swap (Quinn's ruling of 2026-09-16 on the cloud design's R8:
-- option 1 of docs/notes/2026-09-16-inference-provider-and-model-scoping.md).
--
-- Forward-only: 20260911000100…000900 are applied and never edited. Two new columns on `models`
-- and the three rows re-pinned; the training export narrowed. No function here is definer or
-- writing, so nothing to revoke.
--
-- CORRECTION (comment only, 2026-09-16; the statements below are applied and stay as they are):
-- this used to end the sentence above "so nothing to revoke; the guard's counts do not move,"
-- which is wrong — the guard's aggregate parse-count pin (migrations_test.ts) DOES move, by one,
-- 17 -> 18: `create or replace function export_training_rows` below is one more function
-- definition the scan parses. "Nothing to revoke" is the only part that stands: the function is
-- `security invoker` and non-writing, so it needs no new `revoke execute … from …` to go with it.
--
-- `route`: the provider-routing object the adapter sends verbatim (OpenRouter's `provider`
-- preferences: one named upstream, no fallback, zero-retention endpoints only, and every
-- requested parameter honoured or the request refused). `precision`: what the pinned endpoint
-- declares, recorded beside the model id because a quantized build under a full-precision name
-- is the failure the scoping study warned about.
alter table models add column if not exists route jsonb not null default '{}'::jsonb;
alter table models add column if not exists precision text not null default 'unspecified';

-- The pins. Granite 4.2 8B for the two classification kinds (instruction following at 8B, an
-- explicit non-thinking mode, German supported); Qwen3.5-35B-A3B for email (the top open-weights
-- value accuracy on the Structured Output Benchmark from 3B active parameters). Prices are the
-- endpoints' list prices read 2026-09-16; `usage_daily` and `monthly_spend` price every call at
-- the row's numbers.
update models set
  provider = 'openrouter', model_id = 'ibm-granite/granite-4.2-8b', precision = 'bf16 (CoreWeave)',
  prompt_version = 'task-2', usd_per_m_in = 0.10, usd_per_m_out = 0.15,
  sampling = '{"temperature": 0, "reasoning": {"enabled": false}}'::jsonb,
  route = '{"order": ["CoreWeave"], "allow_fallbacks": false, "zdr": true, "require_parameters": true}'::jsonb,
  since = current_date
where kind = 'task';
update models set
  provider = 'openrouter', model_id = 'ibm-granite/granite-4.2-8b', precision = 'bf16 (CoreWeave)',
  prompt_version = 'event-2', usd_per_m_in = 0.10, usd_per_m_out = 0.15,
  sampling = '{"temperature": 0, "reasoning": {"enabled": false}}'::jsonb,
  route = '{"order": ["CoreWeave"], "allow_fallbacks": false, "zdr": true, "require_parameters": true}'::jsonb,
  since = current_date
where kind = 'event';
update models set
  provider = 'openrouter', model_id = 'qwen/qwen3.5-35b-a3b', precision = 'fp8 (DeepInfra)',
  prompt_version = 'email-2', usd_per_m_in = 0.14, usd_per_m_out = 1.00,
  sampling = '{"temperature": 0, "reasoning": {"enabled": false}}'::jsonb,
  route = '{"order": ["DeepInfra"], "allow_fallbacks": false, "zdr": true, "require_parameters": true}'::jsonb,
  since = current_date
where kind = 'email';

-- The training export excluded Gmail rows only. Google's Limited Use policy binds data from a
-- sensitive scope (the Calendar API) exactly as it binds a restricted one, and `origin` cannot
-- tell a Google-Calendar event from an ICS one, so every `events` row is excluded too until an
-- origin split exists (a C4 refinement).
create or replace function export_training_rows(p_since timestamptz)
returns setof judgments
language sql
stable
security invoker
set search_path = public, extensions
as $$
  select * from judgments where origin not in ('gmail_api', 'events') and judged_at >= p_since;
$$;
