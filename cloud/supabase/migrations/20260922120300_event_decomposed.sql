-- Knowlu — stream J Task T3: the event prompt's two drop rules stop being prose the model must hold
-- in mind and become their own schema fields, answered first and combined in code.
-- `judge_prompts.ts`'s EVENT_SCHEMA gains `audience_excludes_student` and `standing_or_drop_in`
-- (booleans, required, ahead of `verdict`), and its event system prompt explains each one;
-- `judge_validate.ts` turns either field being true into a `drop` whose `why` is a template naming
-- the rule (EVENT_RULE_WHY), and otherwise passes the model's own verdict through. `unsure` (T1,
-- ruling J-1) stays a first-class verdict. The reply to the device is unchanged — verdict, why,
-- confidence — so nothing engine-side moves.
--
-- Shipped only if E1 (`scripts/experiments/e1-decomposition/`, the B1 gate) shows the decomposition
-- beating event-3 on T2's seed by more than the seed's noise; if it does not, this migration and
-- the prompt change are reverted together, never one without the other.
--
-- Forward-only, following 20260922120100_event_unsure.sql's own pattern: only the `event` row of
-- `models` changes, and only what actually changed. `prompt_version` records the new prompt text;
-- `grammar_version` moves because the SCHEMA shape itself changed (two new required fields). The
-- pinned model, its provider, route, precision, price, sampling and max_tokens stay as they are —
-- two booleans fit easily inside the event row's 256 tokens. Neither `task` nor `email` is touched.
update models set
  prompt_version = 'event-4',
  grammar_version = 'event-3',
  since = current_date
where kind = 'event';
