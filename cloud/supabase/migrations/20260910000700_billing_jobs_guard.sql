-- Knowlu C1, ruling R-C1-38 — the billing job fails loudly when a Vault secret is missing, and waits
-- long enough for the function to answer.
--
-- 000600 read the two secrets with bare subselects and claimed a missing one would fail the run. It
-- does not, reliably. `net.http_post` is not `strict` on this pg_net (0.20.4, checked on staging), so a
-- null url does raise from the queue insert — but a null token becomes a non-null header object
-- (`{"x-knowlu-job-token": null}`), the request goes out, the function answers 401 into
-- `net._http_response`, and `cron.job_run_details` records `succeeded`. Staging sat in exactly that
-- state (url created, token still Quinn's to create). On a pg_net where `http_post` is `strict`
-- (0.7–0.14), a null url is a silent NULL as well. So the job checks for itself, in a `do` block, and
-- raises: `raise exception` lands verbatim in `cron.job_run_details.return_message` with
-- `status = 'failed'`, whatever the pg_net version.
--
-- `timeout_milliseconds := 60000`: pg_net's default is 5 s, and the function makes a Stripe call — and
-- for a reminder or a resume notice an email — per subscriber. Past a handful of subscribers a 5 s
-- timeout reads as a failure in `net._http_response` while the function keeps running.
--
-- What each table proves: `cron.job_run_details` proves the guard ran and the request was queued (or
-- says why not); the HTTP outcome — status code, error — is in `net._http_response`, which is where to
-- look when a run says succeeded and nothing happened.
--
-- The secrets, once per project, in the SQL editor. The URL is public; the token is the same string as
-- the function secret `BILLING_JOBS_TOKEN`, because the function compares the header against it:
--   select vault.create_secret('https://<ref>.supabase.co/functions/v1/billing-jobs', 'billing_jobs_url');
--   select vault.create_secret('<the token>', 'billing_jobs_token');
-- To rotate, `create_secret` again is a unique violation (`vault.secrets(name)` is unique); it is
--   select vault.update_secret((select id from vault.secrets where name = 'billing_jobs_token'), '<new>');
-- with `supabase secrets set BILLING_JOBS_TOKEN=<new>` in the same sitting.
--
-- `supabase_vault` ships enabled on every Supabase project; the `if not exists` records the dependency
-- instead of assuming it. `cron.schedule` with an existing job name replaces the command in place. The
-- nested dollar tags must differ from the outer one.
create extension if not exists supabase_vault with schema vault;

select cron.schedule(
  'knowlu-billing-jobs',
  '17 7 * * *',
  $job$
  do $guard$
  declare
    v_url   text := (select decrypted_secret from vault.decrypted_secrets
                     where name = 'billing_jobs_url'   order by created_at desc limit 1);
    v_token text := (select decrypted_secret from vault.decrypted_secrets
                     where name = 'billing_jobs_token' order by created_at desc limit 1);
  begin
    if v_url is null or v_token is null then
      raise exception 'knowlu-billing-jobs: vault secret billing_jobs_url or billing_jobs_token is missing';
    end if;
    perform net.http_post(
      url                  := v_url,
      headers              := jsonb_build_object('content-type', 'application/json',
                                                 'x-knowlu-job-token', v_token),
      body                 := '{}'::jsonb,
      timeout_milliseconds := 60000
    );
  end
  $guard$;
  $job$
);
