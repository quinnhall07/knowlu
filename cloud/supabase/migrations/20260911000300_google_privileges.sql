-- Knowlu C2, Task 10 fix round 1 — R-C2-E29 and R-C2-E30. 20260911000200_google.sql is already
-- applied and stays exactly as it shipped (migrations are forward-only); this is the follow-up.
--
-- R-C2-E29 — a SECURITY DEFINER function runs with the DEFINING role's privileges, not the
-- caller's, and `revoke execute on function … from public` in 000200 removed only the PUBLIC
-- pseudo-role's entry. Supabase grants EXECUTE to `anon` and `authenticated` explicitly on every
-- new function by default, so `store_google_grant`, `delete_google_grant`, `read_google_grant` and
-- `read_google_grant_any` stayed callable over PostgREST with nothing but the public anon key —
-- verified on staging: `store_google_grant`'s `pg_proc.proacl` was
-- `{postgres=X/postgres,anon=X/postgres,authenticated=X/postgres,service_role=X/postgres}`, and
-- `read_google_grant` has no `auth.uid()` binding at all, so it returns any account's decrypted
-- refresh token to whichever `p_account` the caller names. `take_google_state` and
-- `export_training_rows` are SECURITY INVOKER, not DEFINER, so they are outside the static guard
-- this migration also satisfies (`migrations_test.ts`'s new test scans DEFINER functions only) —
-- but every one of these six is meant to have exactly one caller (this stream's own edge
-- functions, over the service role), so all six are locked down here for the same reason, not only
-- the four the guard can see.
--
-- R-C2-E30 — `vault.secrets.name` is UNIQUE (`secrets_name_idx … where name is not null`,
-- verified on staging), and `store_google_grant` always called `vault.create_secret` under the
-- name `'google:' || p_account`. A second connect for the same account — the ordinary case while
-- the Google project is in Testing and refresh tokens expire after 7 days (§9) — raised 23505 from
-- inside the RPC, `google-callback` caught it as an unnamed exchange failure, and the student saw
-- "Gmail could not be connected just now" forever. `store_google_grant` is redefined below to
-- `vault.update_secret` the existing row in place when one is already there, rather than trying to
-- create a second one under the same name.

create or replace function store_google_grant(
  p_account uuid, p_sub text, p_email text, p_token text, p_scopes text[]
)
returns uuid
language plpgsql
security definer
set search_path = public, vault, extensions
as $$
declare
  sid      uuid;
  existing uuid;
begin
  -- Update in place when the account already has a live Vault secret; only a first-ever connect
  -- creates a new one. `vault.update_secret(secret_id, new_secret, new_name, new_description,
  -- new_key_id)` leaves every parameter left at its default (null) unchanged, so this touches only
  -- the token.
  select g.secret_id into existing
    from google_accounts g
   where g.account_id = p_account
     and exists (select 1 from vault.secrets s where s.id = g.secret_id);
  if existing is not null then
    perform vault.update_secret(existing, p_token);
    sid := existing;
  else
    select vault.create_secret(p_token, 'google:' || p_account::text, 'Google refresh token (read-only scopes)')
      into sid;
  end if;
  -- `scopes` is a UNION on conflict, not a replacement: `gmail.readonly` is asked for
  -- incrementally and Google returns only the newly granted scope on that exchange, so
  -- overwriting would silently forget the calendar the student connected in the wizard (§11a).
  insert into google_accounts (account_id, google_sub, secret_id, email_hint, scopes, status)
       values (p_account, p_sub, sid, p_email, p_scopes, 'active')
  on conflict (account_id) do update
       set google_sub = excluded.google_sub, secret_id = excluded.secret_id,
           email_hint = excluded.email_hint, status = 'active', connected_at = now(),
           scopes = (
             select array_agg(distinct s)
               from unnest(google_accounts.scopes || excluded.scopes) as s
           );
  return sid;
end;
$$;

revoke execute on function store_google_grant(uuid, text, text, text, text[]) from public, anon, authenticated;
grant execute on function store_google_grant(uuid, text, text, text, text[]) to service_role;

revoke execute on function delete_google_grant(uuid) from public, anon, authenticated;
grant execute on function delete_google_grant(uuid) to service_role;

revoke execute on function read_google_grant(uuid, text) from public, anon, authenticated;
grant execute on function read_google_grant(uuid, text) to service_role;

revoke execute on function read_google_grant_any(uuid) from public, anon, authenticated;
grant execute on function read_google_grant_any(uuid) to service_role;

revoke execute on function take_google_state(text) from public, anon, authenticated;
grant execute on function take_google_state(text) to service_role;

revoke execute on function export_training_rows(timestamptz) from public, anon, authenticated;
grant execute on function export_training_rows(timestamptz) to service_role;
