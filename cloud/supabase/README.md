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
- **A Gmail that went quiet (L7).** Every `gmail-read` reply on the Gmail path carries `went_quiet`
  (a boolean). It is true when the 7-day list the handler already makes (`WINDOW`, with Knowlu's
  excluded labels kept) holds no message at all and the grant is at least 7 days old
  (`wentQuiet`, `QUIET_AFTER_MS`; milliseconds, so exactly 7 days counts). The rule reads that list
  raw, before dedup and `READ_CAP`: an already-seen message is still mail. No second Gmail call is
  made. The age is `google_accounts.connected_at`, which `store_google_grant` resets on every connect
  and reconnect. Straight after the list returns, and before the judging loop, the handler moves
  `active` to `quiet` when the rule holds and `quiet` to `active` when the list held mail (each move
  is conditional on the current status, so a `revoked` row is never touched). The reply is the rule,
  never the write's effect, so a row already `quiet` still answers true. The forwarding path moves a
  `quiet` row back to `active` and answers `went_quiet: false`. The not-connected, `revoked` and
  not-configured replies have no `went_quiet` key. The existing `quiet: true` plus `reason` keeps its
  one meaning, not connected; a went-quiet reply has `quiet: false` and no `reason`.
  - `google_accounts.last_read_at` is still never written by anything, and is not the quiet clock;
    do not read it as one.
  - The `status` comment in `20260911000200_google.sql` says "14 days". That is superseded by the
    amended cloud design §5.3 (7 days, Quinn's Q3 of 2026-10-19). The migration is not edited: its
    hash is pinned.
  - The `seen` lookback in `gmail-read/index.ts` is a different thing (the dedup window, twice
    `WINDOW`) and stays as it is.
  - Until `gmail.readonly` is verified, Google's Testing mode kills a refresh token 7 days after
    consent, which takes the `revoked` path first, so the rule cannot fire there yet.
- **Out of credit (L7).** When the model provider answers HTTP 402 (Knowlu's own balance there), the
  `/judge-*` reply gains `unavailable: "out of credit"` beside `cause: "model failed"`
  (`ProviderOutOfCredit`, told apart by class in `judge_pipeline.ts`). The reply is still HTTP 200,
  because a 402 to the device means no entitlement; the charge is refunded and the `judgments` row is
  logged as `model failed`, so no migration. `gmail-read` stops its round on a reply carrying
  `unavailable`, as it does for `capped`, and defers the item.
- The migration and both functions reach staging before a device build with the D4 gate runs a slot
  there (`db push --include-all`).

## Forwarded mail (`mail`, `mail-inbound`, `mail-process`, `mail-canary`, `mail-pull`, `mail-resolve`)

Spec `docs/specs/2026-09-30-email-forwarding-design.md`, plan `docs/plans/2026-09-30-email-forwarding-plan.md`.
Migrations `20261002000200_mail.sql` and, after it, `20261005000100_mail_claim_max_tokens.sql` and
`20261019000100_mail_last_lapsed_at.sql` (each lapse recorded apart from the gap's start); not on production, and no release is cut from a `main` carrying it
before privacy bump #1. All six functions are `verify_jwt = false` in `config.toml`; each authenticates
itself as named below.

- **`mail-inbound`**: SES stores each message in S3 and SNS posts here. Authenticated by the SNS
  signature (`_shared/sns.ts`); only `SignatureVersion` 2 is accepted, and a refusal is a bare 403.
  Finds the mailbox by the address token's HMAC, checks entitlement, fetches the object once, classifies
  it (canary, Gmail's confirmation, batch send, live mail), dedups and hands live mail off under `work/`.
  Mail for no known mailbox is deleted unread.
- **`mail-process`**: the worker over `mail_work`, called by cron every minute and kicked by intake, with
  `x-knowlu-job-token`. Parse, screen (no model), templates (no model), then the model for the rest; each
  item's object and row are deleted when it is done. `{mode: "sweep"}` runs the nightly sweep.
- **`mail-canary`**: hourly cron, same header. Judges delivery checks that did not return and sends one to
  a mailbox that has gone quiet, under the limits of spec §4.10.
- **`mail`** (device routes, session JWT): `GET`/`POST /addresses`, `POST /addresses/<id>/hint`,
  `/rotate` and `/state`, `DELETE /addresses/<id>`, `POST /canary`, `POST /batches` and
  `/batches/<id>/sent`, `POST /reconcile`, `GET /provider?domain=`. The list and the delete need a session
  only, so a lapsed account can always see and disconnect its mailboxes; every other route passes the
  entitlement gate. `DELETE` revokes, deletes the mailbox's `work/` objects, then `delete_mail_address`
  purges every row that serves it.
- **`mail-pull`** (session plus entitlement): acks, closes finished batches, withholds a batch until it is
  whole, and answers one page of queued `mail_claims` in (ordering date, uid) order.
- **`mail-resolve`** (session plus entitlement): one course's ambiguous claims against its candidates;
  the server reads each claim from the account's own row, never from the request.
- **Tables** (all with RLS on, no policy; the service role only): `mail_accounts`, `mail_addresses`,
  `mail_batches`, `mail_canaries`, `mail_seen`, `mail_work`, `mail_claims`. `models`, `usage_daily` and
  `judgments` gain the kinds `mail_claim`, `mail_resolve` and `mail_digest`, and `judgments` the origin
  `forward`. RPCs: `delete_mail_address`, `charge_allowance`, `refund_allowance`, `enforce_budget`,
  `mail_hand_off`, `count_mail_arrival`, `expire_mail_work`. `mail_digest` has a kind and no function
  yet (T16 is Quinn's call at Checkpoint D).
- **Cron** (pg_cron + pg_net, guarded: a missing Vault secret raises into `cron.job_run_details`):
  `knowlu-mail-process` every minute, `knowlu-mail-canary` at :11 each hour, `knowlu-mail-sweep` at
  07:47 UTC daily.

### The AWS runbook (spec §5.1; staging region `us-east-2`)

The receiver is SES inbound on a subdomain of the product domain (MX on it). One receipt rule set holds one
rule with one action: the S3 action, with its own `TopicArn` set and nothing else (no separate SNS action,
no bounce). The bucket blocks public access, encrypts at rest and expires `inbound/` and `work/` objects
after one day. The topic allows publish from SES only. Mail that fails DMARC is stored and judged later,
never rejected at the door (S0, `docs/reports/2026-10-01-email-forwarding-t0-gates.md`).

Secrets, by name only; each is set from the owner's own shell and never printed:

| Where | Names |
|---|---|
| Function secrets (`supabase secrets set`) | `AWS_ACCESS_KEY_ID`, `AWS_SECRET_ACCESS_KEY` (the IAM key pair, Quinn's shell), `AWS_REGION`, `MAIL_BUCKET`, `MAIL_DOMAIN`, `MAIL_SNS_TOPIC_ARN`, `MAIL_ADDRESS_KEY`, `MAIL_ENC_KEY` (generated blind), `MAIL_JOBS_TOKEN`, plus the existing `EMAIL_API_URL` and `EMAIL_API_KEY` |
| Vault (SQL editor, once per project) | `mail_process_url`, `mail_canary_url`, `mail_jobs_token` (the same string as `MAIL_JOBS_TOKEN`) |

Order, per project:

1. `supabase db push` (`--include-all` only when an older stamp is pending), then set the secrets and the
   Vault entries.
2. `supabase functions deploy <name> --use-api --project-ref <ref>` for all six.
3. Set the topic to sign with SHA-256 **before** subscribing, because a topic signs with version 1 by
   default and `verifySns` refuses it: `aws sns set-topic-attributes --topic-arn <MAIL_SNS_TOPIC_ARN>
   --attribute-name SignatureVersion --attribute-value 2`, then read it back with `aws sns
   get-topic-attributes`. Skipped, the SubscriptionConfirmation gets a bare 403 and the subscription
   stays `PendingConfirmation` with no log.
4. Point the topic's HTTPS subscription at `mail-inbound` and confirm it.
5. Smoke by a staging session (at most two one-time codes an hour): a canary round trip, one synthetic
   forward and a count-only query. Production gets its own domain, bucket, topic and rule set.

## Testing

```powershell
deno test --allow-read --config cloud/supabase/deno.json cloud/supabase/
```

Every `index.ts` is a thin wire: it reads the environment, builds a `Deps` object and calls the handler. Every
`handler.ts` is pure over that `Deps`, so the tests need no Docker, no Supabase runtime and no network. **No
test here makes an outbound request** — the one thing `deno test` fetches is the assertion library, once, into
Deno's own cache.
