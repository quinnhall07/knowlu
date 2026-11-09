-- No client role holds EXECUTE on a SECURITY DEFINER function (Security Advisor lints 0028/0029).
--
-- `public.handle_new_user()` (20260910000100_accounts.sql, replaced by 20260917000100_oauth_consent.sql)
-- is the `on_auth_user_created` trigger's function: SECURITY DEFINER, `returns trigger`, with
-- `search_path` already pinned to `public, extensions`. It was never revoked, so `anon` and
-- `authenticated` kept the EXECUTE Supabase grants on every new function. Postgres refuses a direct
-- call to a trigger function, but EXECUTE is not left granted to client roles either way.
--
-- The trigger keeps firing: the trigger manager does not check EXECUTE on a trigger's function, so
-- GoTrue's insert into auth.users runs it as before. `supabase_auth_admin` gets EXECUTE explicitly,
-- the same belt 20260922000200 gave `trim_user_metadata`. The function, its `search_path` and the
-- trigger are unchanged.
--
-- Every other SECURITY DEFINER function in public is already revoked from public, anon and
-- authenticated: `store_google_grant`, `delete_google_grant`, `read_google_grant` and
-- `read_google_grant_any` (20260911000300), `trim_user_metadata` (20260922000100) and
-- `delete_mail_address` (20261002000200).
revoke execute on function public.handle_new_user() from public, anon, authenticated;
grant execute on function public.handle_new_user() to supabase_auth_admin;
