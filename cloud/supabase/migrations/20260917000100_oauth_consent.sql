-- Knowlu C1b, Task 1 — an OAuth sign-up can create its account row (spec §5.1, ruling R-C1b-3).
--
-- Google's ID-token claims carry no `age_attested`, `tos_version` or `privacy_version`, so today's
-- function raises on the second `if` and the whole `auth.users` insert rolls back: GoTrue answers
-- `Database error saving new user` and redirects with `error=server_error`.
--
-- The replacement reads NOTHING out of `raw_user_meta_data`, on either path. Not because Google
-- cannot send it, but because the email path's `/otp` is reachable by anyone holding the public anon
-- key and, with `enable_confirmations = false`, GoTrue creates the user before the code is ever
-- typed: a trigger that believed that request's own `data` would write an `age_18` consent row
-- asserting an attestation the address's owner never made, for any address a stranger chose. So
-- every new user starts with the four consent columns null and no `consents` rows, and
-- `POST /account/consent` — behind a session whose address has been proved — is the only writer of a
-- consent row in the system.
--
-- The 18+ gate does not leave the server with the raise: `billing-checkout` answers
-- `403 the 18+ attestation is missing` while `age_attested_at` is null, before the Stripe customer
-- and before the consent row. A patched client that skips the consent call gets an account it can
-- never subscribe with.
--
-- Nothing else changes: no table, no policy, no column. The three rules `migrations_test.ts` pins
-- are untouched.
--
-- "Never raises" is true of this body, not of the insert it makes: `accounts.email` is `not null`
-- (`20260910000100_accounts.sql`), so a sign-up with no email — anonymous or phone auth, neither
-- enabled today — would still abort at that constraint, not at a `raise` this function wrote. Worth
-- one line here so a future provider switch does not have to rediscover it (nit 9).
create or replace function public.handle_new_user() returns trigger
language plpgsql security definer set search_path = public, extensions as $$
begin
  insert into public.accounts (id, email, tos_version, tos_accepted_at, privacy_version, age_attested_at)
  values (new.id, new.email, null, null, null, null)
  on conflict (id) do nothing;

  insert into public.entitlements (account_id, status) values (new.id, 'none')
  on conflict (account_id) do nothing;

  return new;
end;
$$;
