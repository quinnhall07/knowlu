-- Knowlu — stream J Task T4: the email `due` reference date.
--
-- The old email prompt asked the model to resolve a relative deadline ("Friday", "next week")
-- into an absolute date itself — the 2026-09-16 scoping note's "single most dangerous line": a
-- mis-resolved phrase comes back as a well-formed, silently wrong date. The corroborated fix is to
-- keep the model on EXTRACTION (it now returns the deadline phrase as written, or an absolute date
-- only when the email states one explicitly) and let a new deterministic TypeScript resolver
-- (`cloud/supabase/functions/_shared/judge_due.ts`) do the arithmetic, between the model's answer
-- and `judge_validate.ts`'s `validate()`. The wire contract to the device does not change: `due`
-- is still `YYYY-MM-DD`, `YYYY-MM-DDTHH:MM` or null.
--
-- Only the email prompt's due bullet changes (`judge_prompts.ts`'s `systemTemplate("email")`), so
-- only the email row's `prompt_version` moves — off `email-2`
-- (`20260916000100_provider_swap.sql`'s re-pin) onto `email-3`. Task and event are untouched: this
-- lane never touches the EVENT region of `judge_prompts.ts`/`judge_validate.ts`, and the task
-- prompt never had a `due` field to begin with.
--
-- Forward-only, like every migration here: 20260911000100…20260916000100 are applied and never
-- edited. Not applied anywhere by this task (stream J lane rule 7): no `supabase` CLI call against
-- a remote, staging or production.
update models set prompt_version = 'email-3', since = current_date where kind = 'email';

-- Stream J Task T9, folded into this same `email-3` (nothing deployed between T4 and T9): the
-- email prompt gains a sixth tier, `completion` ("this email confirms the student already submitted
-- or finished a specific piece of work", `title` = the work's name), and `gmail-read` recognises a
-- templated LMS submission receipt deterministically before the model. Same route, provider, pin
-- and budget — the one database change is the queue's own tier check, which named the five tiers
-- and would otherwise refuse a `completion` row with a 23514 and fail the whole read. The inline
-- check in `20260911000200_google.sql` took Postgres's default name, `gmail_queue_tier_check`.
alter table gmail_queue drop constraint if exists gmail_queue_tier_check;
alter table gmail_queue add constraint gmail_queue_tier_check
  check (tier in ('task', 'borderline', 'event', 'opportunity', 'information', 'completion'));
