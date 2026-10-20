-- Knowlu — email forwarding: `mail_resolve`'s `max_tokens` re-pinned from 640 to 1280 and `mail_claim`'s
-- from 1024 to 1536 (Quinn's ruling of 2026-10-20, T18b's pin check; plan P8's re-pin path, one statement).
--
-- T18b's evidence: a 15-claim `mail_resolve` batch (`mail-resolve`'s `MAX_CLAIMS`) stopped at 640 of 640
-- output tokens with none of the 15 answered, and a five-obligation `mail_claim` reply used 1008 of the
-- 1024 that 20261005000100_mail_claim_max_tokens.sql pinned. A `max_tokens` stop records the item
-- `truncated` and drops every answer (`mail-process/model.ts`), so both caps get room; the cap is not
-- what is billed: a call pays for the tokens it used.
--
-- `models` is one row per kind (primary key `kind`), so only the two mail rows move, each to its own value
-- in the one statement. Task, event and email are untouched. The model, provider, route, precision, price,
-- sampling and prompt and grammar versions stay as 20261002000200_mail.sql pinned them; `since` is
-- re-stamped, as on every re-pin. Forward-only: the mail migration and 20261005000100 are applied on
-- staging and never edited.
update public.models
   set max_tokens = case kind when 'mail_resolve' then 1280 when 'mail_claim' then 1536 end,
       since = current_date
 where kind in ('mail_resolve', 'mail_claim');
