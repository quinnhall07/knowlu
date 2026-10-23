-- Knowlu — task-3: effort anchors for the task judge, and the task kind re-pinned to Qwen3.5-35B-A3B
-- at DeepInfra (2026-10-23).
--
-- task-2's task prompt said only "the realistic time in hours to finish this one assignment, 0.25
-- to 40": no scale. Items judged from a title, a course slug and a due date (a Blackboard feed item
-- carries little else) came back at a median of 8 hours. `judge_prompts.ts`'s
-- `systemTemplate("task")` now gives one typical effort per assignment type for a typical
-- undergraduate, allows more than 8 hours only for a named large project or a paper of more than 5
-- pages, never more than 6 for an exam, answers 2 for a vague title, lets points scale effort only,
-- and says importance never follows size. The prompt hash moves, so the task row's
-- `prompt_version` moves with it, off `task-2` (`20260916000100_provider_swap.sql`) onto `task-3`.
--
-- The model (Quinn's ruling, 2026-10-23, on PR #61's eval of the 22-case task seed with this
-- prompt): Granite 4.2 8B at CoreWeave scored effort_mae 3.045 and kept answering past the
-- prompt's own ceilings (a final exam at 24 hours, a 12-page paper at 60); Qwen3.5-35B-A3B at
-- DeepInfra scored 0.273, importance_exact 0.947, course_exact 1.000. The task row takes the pin
-- the email row already carries for this model (`20260916000100_provider_swap.sql`, unmoved since):
-- provider, model id, precision, price, sampling (reasoning off) and route (one named
-- zero-retention upstream, no fallback, require_parameters). DeepInfra is already on the privacy
-- page; this file moves which company reads assignments, and `site/privacy.html` says so.
--
-- Unchanged: `grammar_version` (the JSON schema did not change), `max_tokens` (256, the value the
-- eval ran with). The event kind stays on Granite 4.2 8B at CoreWeave; email and the mail kinds are
-- not touched.
--
-- Forward-only, stamped after every existing file. Not applied by the task that wrote it: the
-- controller runs `supabase db push` on staging; production is the owner's.
update models set
  provider = 'openrouter', model_id = 'qwen/qwen3.5-35b-a3b', precision = 'fp8 (DeepInfra)',
  prompt_version = 'task-3', usd_per_m_in = 0.14, usd_per_m_out = 1.00,
  sampling = '{"temperature": 0, "reasoning": {"enabled": false}}'::jsonb,
  route = '{"order": ["DeepInfra"], "allow_fallbacks": false, "zdr": true, "require_parameters": true}'::jsonb,
  since = current_date
where kind = 'task';
