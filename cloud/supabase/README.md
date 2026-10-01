# `cloud/supabase/` — Knowlu's backend

One codebase, two projects: **`knowlu-staging`** and **`knowlu-prod`** (spec §11 R6). Nothing here names a
project; `supabase link` decides.

**Migrations are applied to STAGING from a developer machine, and to PROD by Quinn or by CI.** That rule is
the whole of this file:

```powershell
supabase login                                  # Quinn's own login, once
supabase link --project-ref <the STAGING ref>   # never the prod ref from a laptop
supabase db push                                # applies cloud/supabase/migrations/ in order
supabase db push --include-all                  # when a new file sorts before one already applied
                                                # (20260922120100/120200 on staging after 20260923000100)
supabase functions deploy <name> --project-ref <the STAGING ref>
```

Secrets are set by Quinn, from a value they produce, and are never printed:

```powershell
supabase secrets set STRIPE_SECRET_KEY --project-ref <ref>
```

The project URL and the **anon key are public** and are compiled into the app. The **service-role key** is
injected into every function as `SUPABASE_SERVICE_ROLE_KEY` by the platform; nothing here reads it at import
time, so `deno test` needs no environment at all.

## Google connection (`google-connect`, `gmail-read`)

- `GET google-connect?status=1` answers four keys: `connected`, `scopes`, `status` and `email`.
  `status` is `none` (no row), or the row's `active`, `quiet` or `revoked`; `scopes` is `[]` for a
  revoked row; `email` is the stored `email_hint` or `null`.
- `GET google-connect?scope=calendar|gmail|reconnect` returns the consent URL. `reconnect` asks for the
  identity scopes plus every API scope the row records, in one consent; with no row or no recorded
  scopes it answers 400 `nothing to reconnect`.
- `DELETE google-connect` revokes at Google first (a failed revoke answers 502 and purges nothing),
  then calls `delete_google_grant`, which since `20260929000200_gmail_disconnect_purge.sql` deletes the
  account's `gmail_queue` rows, its `gmail_seen` rows, and the `google_accounts` row with its Vault
  secret. The account's `judgments` rows, `origin = 'gmail_api'` included, stay until the account is
  deleted (Q9 (a)(ii)). Account deletion calls it too.
- Only the consent URL is entitlement-gated (402 for a lapsed subscription). `?status=1` and `DELETE`
  need a valid session and nothing more (401 without one), so a canceled or past-due account can still
  see its connection and disconnect it, as it can delete its account.
- `gmail-read` reads the grant row without a status filter: no row is the silent `no_gmail_scope`, a
  `revoked` row answers `revoked` (and marks it), any other row without the Gmail scope is
  `no_gmail_scope`.
- The migration and both functions reach staging before a device build with the D4 gate runs a slot
  there (`db push --include-all`).

## Testing

```powershell
deno test --allow-read --config cloud/supabase/deno.json cloud/supabase/
```

Every `index.ts` is a thin wire: it reads the environment, builds a `Deps` object and calls the handler. Every
`handler.ts` is pure over that `Deps`, so the tests need no Docker, no Supabase runtime and no network. **No
test here makes an outbound request** — the one thing `deno test` fetches is the assertion library, once, into
Deno's own cache.
