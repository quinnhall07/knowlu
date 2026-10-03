-- Gmail connect in the app, D14 (docs/specs/2026-09-29-gmail-connect-design.md §4.1; Quinn's Q9,
-- decided 2026-09-29: (a) with (ii)). Disconnect deletes what the server kept only to serve the
-- connection.
--
-- Until now `delete_google_grant` removed the `google_accounts` row and its Vault secret and nothing
-- else. `gmail_queue` and `gmail_seen` cascade only from `accounts`, so after a Disconnect the
-- account's undelivered queue rows (each a model-written title and why about one message) stayed
-- until a later reconnect delivered them stale, and its read ledger of message ids stayed until the
-- nightly 30-day prune. This redefinition also deletes, for `p_account` only:
--   - its `gmail_queue` rows, delivered or not;
--   - its `gmail_seen` rows.
-- Q9 (a)(ii): the account's `judgments` rows, `gmail_api` ones included, are KEPT. They carry no
-- message text and are what per-user rule promotion learns from; they go with the account.
--
-- The DELETE handler in `functions/google-connect` is unchanged: it revokes at Google first and calls
-- this second, so a failed revoke purges nothing. Account deletion (`functions/account/
-- google_delete.ts`) calls this too, so it now clears the same rows a moment before the account
-- row's cascade would.
--
-- Same signature, `security definer`, pinned `search_path`. `create or replace` keeps the existing
-- grants; the revoke and grant of 20260911000300_google_privileges.sql are re-issued below so this
-- file says so. 20260911000200_google.sql is applied and is never edited.

create or replace function delete_google_grant(p_account uuid)
returns void
language plpgsql
security definer
set search_path = public, vault, extensions
as $$
declare
  sid uuid;
begin
  delete from gmail_queue where account_id = p_account;
  delete from gmail_seen where account_id = p_account;
  select secret_id into sid from google_accounts where account_id = p_account;
  delete from google_accounts where account_id = p_account;
  if sid is not null then
    delete from vault.secrets where id = sid;
  end if;
end;
$$;

revoke execute on function delete_google_grant(uuid) from public, anon, authenticated;
grant execute on function delete_google_grant(uuid) to service_role;
