-- Knowlu C1, ruling R-C1-36 — the billing job's URL and token live in Vault, not in database settings.
--
-- 000200 had Quinn set `app.billing_jobs_url` and `app.billing_jobs_token` with `alter database
-- postgres set ...`. On Supabase the `postgres` role may not set a parameter on the database
-- (42501 "permission denied to set parameter", staging, 2026-09-10), so nobody can run that block.
-- Supabase's own pattern for a cron job that calls an edge function is Vault: `vault.decrypted_secrets`
-- is readable by `postgres` and `service_role` only — narrower than a role setting, which every role in
-- the database can read out of `pg_db_role_setting` — and the value is encrypted at rest.
--
-- Set once per project, in the SQL editor. The URL is public. The token is the same random string as
-- the function secret `BILLING_JOBS_TOKEN` (generated the way `SOURCES_ENC_KEY` is), because the
-- function compares the header against that secret:
--   select vault.create_secret('https://<ref>.supabase.co/functions/v1/billing-jobs', 'billing_jobs_url');
--   select vault.create_secret('<the token>', 'billing_jobs_token');
--
-- `cron.schedule` with an existing job name replaces that job's command in place, so this is the same
-- `knowlu-billing-jobs` row 000200 created, at the same minute, reading Vault instead of settings. A
-- missing secret makes the subselect null, and `net.http_post` refuses a null url — the run fails in
-- `cron.job_run_details` instead of calling nowhere, exactly as the old `current_setting(..., true)` did.
select cron.schedule(
  'knowlu-billing-jobs',
  '17 7 * * *',
  $$
  select net.http_post(
    url     := (select decrypted_secret from vault.decrypted_secrets where name = 'billing_jobs_url'),
    headers := jsonb_build_object(
                 'content-type', 'application/json',
                 'x-knowlu-job-token',
                 (select decrypted_secret from vault.decrypted_secrets where name = 'billing_jobs_token')),
    body    := '{}'::jsonb
  );
  $$
);
