# `cloud/supabase/` — Knowlu's backend

One codebase, two projects: **`knowlu-staging`** and **`knowlu-prod`** (spec §11 R6). Nothing here names a
project; `supabase link` decides.

**Migrations are applied to STAGING from a developer machine, and to PROD by Quinn or by CI.** That rule is
the whole of this file:

```powershell
supabase login                                  # Quinn's own login, once
supabase link --project-ref <the STAGING ref>   # never the prod ref from a laptop
supabase db push                                # applies cloud/supabase/migrations/ in order
supabase functions deploy <name> --project-ref <the STAGING ref>
```

Secrets are set by Quinn, from a value they produce, and are never printed:

```powershell
supabase secrets set STRIPE_SECRET_KEY --project-ref <ref>
```

The project URL and the **anon key are public** and are compiled into the app. The **service-role key** is
injected into every function as `SUPABASE_SERVICE_ROLE_KEY` by the platform; nothing here reads it at import
time, so `deno test` needs no environment at all.

## Testing

```powershell
deno test --allow-read --config cloud/supabase/deno.json cloud/supabase/
```

Every `index.ts` is a thin wire: it reads the environment, builds a `Deps` object and calls the handler. Every
`handler.ts` is pure over that `Deps`, so the tests need no Docker, no Supabase runtime and no network. **No
test here makes an outbound request** — the one thing `deno test` fetches is the assertion library, once, into
Deno's own cache.
