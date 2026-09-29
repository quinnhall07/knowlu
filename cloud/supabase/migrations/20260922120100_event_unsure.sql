-- Knowlu — stream J Task T1: CHECKPOINT J-1 (ruled by Quinn, 2026-09-22, design (a)). `unsure`
-- becomes a fourth event verdict word, so the model can say honestly that whether an event obliges
-- this particular student is not in the event's own text, instead of being forced to guess among
-- obligation/opportunity/drop. That closes defect A directly, and — because a recorded verdict is
-- never re-asked (events spec §7) — it also closes most of defect B, the re-ask-forever bug, for
-- free. `judge_prompts.ts`'s EVENT_SCHEMA and its event system prompt now teach the model the new
-- word; `judge_validate.ts`'s EVENT_VERDICTS accepts it and its confidence floor no longer blocks
-- it (the floor guards a shaky guess, not an honest decline); `eventledger::VALID_VERDICTS` and
-- `cloudmodel::CloudModel::judge_event` (engine-side) close the remaining below-floor / no-verdict
-- case the same way.
--
-- Forward-only, following 20260916000100_provider_swap.sql's own pattern: only the `event` row of
-- `models` changes, and only what actually changed — the prompt text and the JSON schema's verdict
-- enum, not the pinned model, its provider, its price or its sampling. `grammar_version` moves too,
-- because the SCHEMA shape itself changed (one more enum member); `prompt_version` records the new
-- prompt text. Neither `task` nor `email` is touched: this checkpoint is event-only.
--
-- No DB check constraint enumerates the event verdict WORDS today: `judgments.cause` (`below
-- floor`/`incomplete`/`model failed`/`refused`/`truncated`) is a different, closed vocabulary — why
-- an ANSWER was rejected, never the answer itself — and a verdict like `obligation` or `unsure`
-- lives only in `judgments.fields` (jsonb, unconstrained) and in the vault's own ledger. So there is
-- no constraint here to extend.
update models set
  prompt_version = 'event-3',
  grammar_version = 'event-2',
  since = current_date
where kind = 'event';
