# Friends testers: a comp entitlement and a per-tester cost view — design

**Status: Signed by Quinn, 2026-10-19** (tester names still to come). Lane L8 of the friends-MVP push
(`.superpowers/handoffs/2026-10-18-START-HERE-session-5-mvp.md` §5). Authority: Quinn's rulings of
2026-10-18 in `.superpowers/sdd/2026-09-30-email-forwarding/progress.md` (friends get the app free,
Quinn pays their usage; "a tester list + usage view"), read under the cloud design
(`docs/specs/2026-09-09-knowlu-cloud-design.md` §5.1, §5.2, the 2026-10-16 and 2026-10-16b amendments).
Every choice that touches money is in **§7 Decisions for Quinn**, recommendation first.
Grounded in the email-forwarding branch (newest migrations; it merges to main before this lane starts).
**Revision 1 (2026-10-18), after spec review round 1:** the SQL is proved in three layers, not "against
the local database" (§9, Q10). Privacy now covers every account `route_daily` counts, plus export,
cascade and retention (§3.5, §4, Q7, Q8, Q11). The audit holds no free text. S2 is split into five
tasks with a review checkpoint (§10). The password predicate is concrete (§3.5). The ledger maps each
requirement to a task and a test (§11).
**Revision 2 (2026-10-18), after spec review round 2:** the migrations-test guards are widened to see
the `admin` schema, test first, as a recorded non-append edit (§9 layer i, S2a, §8.7). The audit no
longer outlives the account: the deletion trigger takes it by hash and the sweep holds it three years
at most (§3.3, §3.5, Q7, Q8, A3, A6). The route name parse covers unprefixed and sub-route paths (§4).
S1 runs before S2a and S4 after S2R, and the duplicate S1 row is gone (§10). The purge list, the
lane's merge gate, the contract-list reading, the sweep's minute and extending an expired grant are
stated (§3.3, §6, S4, S6).
**Revision 3 (2026-10-18), after spec review round 1 of the revised draft:** the rule-1 tripwire no
longer refuses the insert that `admin.comp_grant` must hold; it refuses any address-shaped token in a
migration and any write to the grant tables outside the admin functions (§6, §9 layer i). The prod
`route_daily` writes wait on a function secret that the controller turns on only once sentence 1 is on
the live privacy page (§4, §8.5, S7). The service role loses write rights on `comp_grants` and
`comp_audit`, so "only through the admin functions" holds by privilege (§3.5, A7). An account with no
Stripe row and no grant answers `none`, pinned as P0 (§3.2). The staging proof runs **before** the
merge (S7a). The pattern widening is its own task and commit (S2a0), and every non-append edit to
`migrations_test.ts` is listed (§8.7, §10). The model moves out of a `_test.ts` file (§9 layer ii).
`account` and `google-connect` are named among the gated routes (§2, §4). Q2 no longer says L3 edits
`enforce_budget`.
**Revision 4 (2026-10-18), after spec review round 2 of the revised draft:** every signed-up account
already has an `entitlements` row at `status = 'none'` (`handle_new_user`), so that row, not a missing
one, is now the main tester case (P0, P3), the grant branch is pinned as the complement of the paid
statuses, and the missing row is its own pair of cases (P0x, P3x) (§2, §3.1, §3.2, §9). The staging
proof lets the sign-up trigger create `accounts` and `entitlements` and updates them (§9 layer iii).
S2c and S2e are each split in two (§10). The stamp follows L3's recorded stamp, strictly later (§8.4,
L3's Q-F). The tripwire gains must-pass cases for S2a's own revoke and foreign key and pins its DML
shapes (§6, §9). The view's four column names are pinned against the lookup's query (§9). The
`ROUTE_COUNTS` switch includes a redeploy (§4). S5 also fixes `afterUpgradeSignIn`, and its static pin
goes to test-writer (§3.4, §10). The address-token ban is in the §8.7 hand-off.
**Revision 5 (2026-10-19), after the Wave A converge review (PASS, five minors).** The five
minors are folded in:
- the staging proof gains A3s, the account deletion as the service role runs it (§9 layer iii);
- Q3 and §3.4 say "while the account is comped (the view answers `tester`)", not "while a grant is
  open";
- the sweep's schedule pin compares minute fields, not whole strings (§3.1, §9);
- §8.3 names the email-change denial of service inside the password path;
- §8.2 states what is signed and what waits on Quinn's reading.

§8.4 now states the one migration-stamp rule (M1-M8) shared with the email, L3, L5 and L1
migrations, after Quinn's C3 Q8 ruling of 2026-10-19. The same text is in L3's plan §2.1. No signed
ruling changed, and the signature line above is Quinn's.
After the convergence review of 2026-10-19 (PASS; one important, three minors; recorded in L3's plan
§9.00 item 6), M2's last sentence no longer claims the C3 phrase gives the same stamp, and the C3
plan's §5 (its r7) now follows M1-M8: picking and re-check against all three of M3's sets, M2's next
free stamp, and no `--include-all` for staging or prod.
After the stamp-rules review of 2026-10-19 (not a pass; one important, one minor; recorded in L3's
plan §9.00 item 7), M7 clears a lane ahead only once its migration is on staging or it is recorded
as slipped, never on its merge alone. S7a waits for L3's migration on staging or L3's recorded slip
(§8.4 item 4, §8.10, the S7a row and the ledger row agree). C1's wait adds "on staging" to Q8's
merges, which is stricter than Q8's words and goes to Quinn. M8 gives the one remedy for a merged,
unpushed migration (the controller's re-stamp commit on `main` before its first push), and M5 points
to it. The minor (the cloud design's amendments and this spec uncommitted) is the controller's commit.
After that review's re-review of 2026-10-19 (not a pass; one important: a merged, unpushed L3
recorded as slipped still rode into staging inside S7a's rebased `db push`; recorded in L3's plan
§9.00 item 8), M7 merges a migration only with its staging push (the controller pushes right after
the merge, so "merged but not pushed" never waits; L3's merge waits for Q-E), a recorded slip clears
only an unmerged lane, and every staging push first runs `supabase db push --dry-run` and stops if it
lists any version but the pushing lane's own. M8's re-stamp commit stays as the remedy only if the
merge-with-push rule is broken, and M5's re-check runs on the rebased lane branch before the merge.
Item 4, §8.10, the S7a row, the order paragraph and the ledger row follow.
After the second re-review of 2026-10-19 (not a pass; one important: a lane that pushed from its lane
branch but had not merged counted as cleared and could slip, though every dry run behind it then
refuses, and the CLI's `migration repair` hint would later re-run its SQL; recorded in L3's plan
§9.00 item 9), M7 clears a lane ahead only once its migration is on staging **and** on `main`; a lane
that pushed from its lane branch holds every lane behind it until it merges; a recorded slip clears
only a lane that has neither merged nor pushed; a dry-run refusal naming a remote version with no
local file waits for that lane's merge and a rebase, never `migration repair` outside M6; and no lane
merges until its dry run on the rebased lane branch lists its own version alone. M8's second bullet
now says a lane that pushed cannot be overtaken. For this lane: S7a waits for L3's merge if L3 pushed
first, and from S7a until S6's merge this lane holds L5's push and L1's C1 (§8.10). Item 4, §8.10,
the S7a row, the order paragraph and the ledger row follow. No signed ruling changed.
After the third re-review of 2026-10-19 (the rule's r7; not a pass, though r6's fix passed; one
important: a lane that pushed from its branch and then stops could never be cleared, because §8.10's
hand teardown was no clearing and its version-row delete was the barred `migration repair`; three
minors; recorded in L3's plan §9.00 item 10), M7 gains a **withdrawal** clause: only the controller
withdraws a lane that pushed from its branch and will not merge as pushed, by dropping its objects on
staging, then removing its version row (`migration repair --status reverted`, safe once the objects
are gone), redeploying from `main` any function it deployed from its branch and confirming by a dry
run on `main`, each step in the ledgers. The lane then counts as neither merged nor pushed (name
unfrozen, may be recorded as slipped, M5 again before any new push). `migration repair` now runs only
for M6's rename and in a withdrawal; a dry-run refusal waits for the lane ahead's merge or its
withdrawal, and a remote version no lane recorded goes to Quinn. At L3's T0 the controller checks
that every version on staging has a file on `main`. M8 gains a last bullet: a merge that broke M7
behind a pushed, unmerged lane is reverted on `main`, on Quinn's word. M7's C1 wait names this lane:
C1 also waits for Quinn's S6 go. For this lane: §8.10's teardown is now M7's withdrawal (a P0, P1 or
P3 failure, or Quinn declining the merge), and item 4, the S7a row, the order paragraph and the
ledger row follow. No signed ruling changed; the signature line above is Quinn's.
After the fourth re-review of 2026-10-19 (of the rule's r7: **PASS**, five minors and one M5 note;
recorded in L3's plan §9.00 item 11), the rule's final pass, r8, closes them in both copies. M7's
Withdrawal carries **Quinn's ruling of 2026-10-19**: the controller withdraws a lane on its own when
its staging proof shows a defect in its SQL, records it and tells Quinn; a merge Quinn declined is
Quinn's word; any other case asks Quinn first; a defect in the functions alone is redeployed from
the lane branch, with no withdrawal. The withdrawal now redeploys from `main` first and drops second,
runs the drop as one `begin … commit` and checks the catalogue before the repair (five steps). M5
runs again after a withdrawal; M1 places a follow-up migration after a lane's merge at the back of
the order; M8 offers Quinn two remedies for a broken merge behind a pushed lane (the revert, or a
withdrawal of the lane ahead that rewrites nothing on `main`). For this lane: §8.10 (who decides,
the one-transaction drop and the catalogue check; S7a deploys no function, so there is no window),
the S7a row and the ledger row. No signed ruling changed; the signature line above is Quinn's.
**Queue ruling (2026-10-19), Quinn's.** The L4 plan's review round 1 (`docs/reports/2026-10-19-l4-plan-review-r1.md`,
finding C1) found a deadlock. L4's bump waits for this lane's merge, so that §8.5's two sentences are
true when the page lands. This lane could not merge without S5, and §8.1 started S5 only after L4 had
merged. Quinn ruled option (a): S5 moves ahead of L4's T8 in the `console.js` queue. §8.1 and the S5
row now read L6 → L8 S5 → L4 → L7 → #19 → L1 R11 → L5: S5 starts after L6's merge and waits for no L4
task, and L4's T8 rebases over S5. Nothing else in this spec changed with this note; the signature
line above is Quinn's.

## 1. What this is, and what it is not

- **A comp:** a named tester's account is entitled with no purchase, no Stripe customer, no Stripe
  charge, and the app never shows a price or a payment step while the comp is live.
- **A cost view:** Quinn (whoever holds the prod Supabase dashboard) sees, per tester per day, judge
  calls, tokens, an estimated model cost, mail arrivals and gated-route volume, and a monthly total.
- **Not:** a free tier, a coupon, a trial, a self-serve sign-up, or a change to the engine. No tester's
  name or email lives in the repo, a migration, a fixture or a test (CLAUDE.md rule 1).
- **Unchanged contracts:** `engine/src/entitle.rs` (status must be `active`/`trialing`, the 72-hour grace
  from `checked_at`), `app/src/account.rs` (L4's; it caches `status`, `plan`, `current_period_end` as
  given), the writers of `public.entitlements` (the Stripe webhook, and `handle_new_user`, which inserts
  the `status = 'none'` row at sign-up; this lane adds none), `MONTHLY_CEILING_USD`.

## 2. What exists today

| Fact | Where |
|---|---|
| `entitlements(account_id, plan, status, current_period_end, source default 'stripe', stripe_subscription_id, paused, started_at, updated_at)`, `status in ('active','trialing','past_due','canceled','none')`. Its comment says "Written ONLY by the stripe-webhook function", but that is not the whole truth: the sign-up trigger `handle_new_user` inserts `(account_id, status) = (new.id, 'none')` for every new `auth.users` row, so **every signed-up account has a row, at `none` until Stripe writes it**; a missing row means one was deleted by hand. The current trigger (C1b) reads no user metadata, never raises and writes no `consents` row | `20260910000100_accounts.sql`, `20260910000200_billing_jobs.sql`, `20260917000100_oauth_consent.sql` (`handle_new_user`) |
| Every gated function (judge-*, ingest-*, events, sync-*, the student-token mail routes `mail`, `mail-pull` and `mail-resolve`, gmail-read, `account` and `google-connect`) takes `requireActiveEntitlement`, whose one lookup is `lookupFrom(rest)` reading `entitlements`; `GET /entitlement` uses the same `LookupEntitlement` and answers `none` with a 200. `mail-inbound` and `mail-canary` carry no student token: they call `lookupFrom` and `isActive` directly for the addressed account (email branch, `mail-inbound/index.ts`, `mail-canary/index.ts`), so they follow the lookup's table but not anything added inside `requireActiveEntitlementWith` | `functions/_shared/entitlement.ts`, `functions/entitlement/handler.ts` |
| `billing_subscribers` (the summer pause and the yearly reminder) selects only rows with a `stripe_subscription_id` | `20260910000200_billing_jobs.sql` |
| `billing-portal` answers 409 "this account has never subscribed" with no `stripe_customer_id` | `functions/billing-portal/handler.ts` |
| `usage_daily(account_id, day, kind, calls, in_tokens, out_tokens, refunds)` for all six kinds (task, event, email, mail_claim, mail_resolve, mail_digest), written by `charge_call` / `record_tokens` / `refund_call` | `20260911000100_judgment_service.sql`, `20261002000100`, `20261002000200_mail.sql` |
| `models.usd_per_m_in/out` per kind at **today's pin only**; `monthly_spend` prices all history at today's pin (accepted: "a budget alarm, not an invoice") | `20260911000100`, `20260916000100_provider_swap.sql` |
| `enforce_budget`: $2.00 a month per account (`MONTHLY_CEILING_USD`) plus the $1.00 backfill allowance while it is open; over it, `capped` and a `budget_alerts` row | `_shared/judge_caps.ts`, `20261002000200_mail.sql` |
| `judgments` (tier 0-3, outcome, per call), `mail_seen` (arrivals by phase, 180 days, gone on Disconnect), `gmail_seen` (Gmail messages judged), `sync_usage.bytes` (current only) | the same migrations, `20260911000200_google.sql`, `20260912000100_sync.sql` |
| The app's Settings account row prints `email · status · plan` raw; *Manage subscription* calls `billing-portal`; the wizard's subscribe step shows the price buttons and only skips Checkout when a button or Next re-asks `checkEntitled()` | `app/static/console.js` (`renderAccountRow`, `#set-portal`, `wizGo`, `#wiz-sub-*`) |
| Telemetry views refuse any slice under 10 accounts | `20260910000400_telemetry.sql` |
| `[api] schemas = ["public"]`: PostgREST exposes nothing else | `cloud/supabase/config.toml` |
| `[auth.email] enable_confirmations = false`; "GoTrue creates the `auth.users` row before the code is ever typed" | `cloud/supabase/config.toml` |

Missing for the cost view: the model that served each call (history is re-priced on a pin change),
the provider's own billed cost, any per-account count of ingest, events or sync calls, and any
per-account share of infrastructure (S3, SES, egress, invocations).

## 3. The comp

### 3.1 How a comp row is represented — recommended: a grant table beside `entitlements`

`public.comp_grants`, one row per grant:

| Column | Meaning |
|---|---|
| `id uuid` | the grant |
| `email_lower text not null` | the address the tester signs in with, lower-cased; a grant can exist **before** the tester signs up, so the subscribe step never shows them a price |
| `cohort text not null` | `friends` now; see Q6 for `pilot` |
| `granted_at`, `ends_at timestamptz not null` | every grant has an end (§3.3) |
| `ended_at`, `ended_reason` | set when a grant is ended early; never deleted to end it |
| `note text` | why, in the admin's words, at most 200 characters. It lives only on this row: it is shown to the tester in their export, deleted with the row and **never copied into `comp_audit`** (§3.5). Guidance in the function comment: no name or relationship ("friends cohort, round 1", not who the person is) |

A partial unique index allows one open grant (`ended_at is null`) per `email_lower`. A grant row is
not kept forever (Q11): `admin.comp_sweep()`, scheduled daily by `pg_cron` as `knowlu-sweep-comp`
(the pattern of `knowlu-sweep-google-state`), deletes a grant 30 days after it closes (`ended_at` or `ends_at`, whichever
is first), claimed or not, so an address that never signed up is not held past its grant's life plus
30 days. The audit row, which carries only a hash, survives the grant's sweep; the same sweep deletes
audit rows three years after they were written, a maximum (Q8). Its schedule is a fixed minute and
hour, once a day. **Its minute field (the first of the five cron fields) is a number that no other
fixed-minute job in the corpus uses, whatever that job's hour.** So `'23 8 * * *'` beside
`knowlu-sweep-google-state`'s `'23 7 * * *'` is refused. Jobs whose minute field is `*` or a step
(`knowlu-mail-process`'s `'* * * * *'`) are skipped: they run at every minute by design, and no choice could
avoid them. This lane's own pin compares minute fields. The existing check in
`the_mail_cron_jobs_use_the_vault_token` (`j.when !== sweep`) compares whole schedule strings, which
is weaker, so it is not reused as it stands (revision 5). A view,
`public.effective_entitlements` (one row per account), resolves what `/entitlement` and the gate see;
`lookupFrom` reads the view instead of the table — one line in `_shared/entitlement.ts`, and every gated
function and `GET /entitlement` follow, because they all share that lookup.

**This view gates every paying student, so it is built to fail safe.** Its rows for an account with no
open grant are the `entitlements` row passed through column for column (a `left join` from
`accounts`, never an inner join on grants), pinned statically and proved on staging before the merge
(§9 layer iii, case P1). **The row a tester actually carries is the `none` row** that
`handle_new_user` writes at sign-up (§2), not a missing one: every unpaid account and every real tester
has it. So the grant branch tests the complement of the paid statuses,
`coalesce(e.status, 'none') not in ('active', 'trialing')`, verbatim and pinned, and never a list of
unpaid ones: a branch written as `e.account_id is null or e.status in ('canceled', 'past_due')` would
pass every case that lacks a `none` row and answer `none` to every friend on prod (P3 is the `none`
row for that reason). Because the join starts at `accounts`, an account whose `entitlements` row was
deleted by hand gets a view row where `lookupFrom` used to get none; the view answers it
`status = 'none'` (`coalesce(e.status, 'none')`), `plan` and `current_period_end` null (case P0x). That
is what `GET /entitlement` already says for a missing row (`row?.status ?? "none"`), and `isActive`
refuses it either way, so the wire is unchanged; the coalesce exists so that one value, not two, is
pinned and proved. Order at deploy, on staging and on prod alike: the migration first (the view exists, nothing
reads it yet), then the layer-iii proof, then the functions that read it. **No function is deployed
from a `main` that carries S3 to an environment that lacks this migration** (S7 states it for L9's
prod parity pass). Rollback is the one-line revert of `lookupFrom` to `entitlements` and a function
redeploy; the view and tables can stay.

**Alternatives, briefly:**
- *A `source = 'comp'` row in `entitlements`* (the column exists). Smallest diff, but it breaks the
  load-bearing "only the Stripe webhook writes `entitlements`", needs a cron to flip status at the end,
  can only exist after sign-up (so the tester meets the price step first), and a later Stripe event's
  stale guard compares against a hand-written `updated_at`. Rejected.
- *A 100%-off Stripe coupon or free price.* Creates Stripe customers and subscriptions for friends,
  sends them billing emails and the yearly ARL reminder, enters `billing_subscribers`, and still walks
  them through Checkout. Contradicts "no purchase, no Stripe charge". Rejected.
- *A long Stripe trial.* Asks for a card, ends in a charge. Rejected.
- *Grants keyed by account id.* Safest binding, but only after sign-up; kept as the fallback if §3.5's
  sign-up check fails on staging.

### 3.2 Precedence (what the view answers)

| Stripe row | Open, unexpired grant matching `lower(accounts.email)` | `status` / `plan` / `current_period_end` |
|---|---|---|
| `active` or `trialing` | any | the Stripe row, unchanged (paid wins; see Q3) |
| anything else: `none` (the sign-up row, the usual case), `canceled`, `past_due`, or no row | yes | `active` / `tester` / the grant's `ends_at` (P3, P3x, P5, P6) |
| any row (`none` included) | no (none, ended, or `ends_at` passed) | the row, unchanged (P0 for `none`) |
| no row (deleted by hand) | no (none, ended, or `ends_at` passed) | `none` / null / null (P0x) |

The view exposes exactly four columns, `account_id`, `plan`, `status`, `current_period_end`, the names
`lookupFrom`'s PostgREST query filters and selects (`account_id=eq.…&select=plan,status,current_period_end`)
and `billing-checkout`'s `plan` lookup reads; a renamed or missing alias is a 400 that `restSelect`
throws, a 500 on every gated call, so the names are pinned on both sides (§9 layer i, S3, S4).

`status` stays in today's vocabulary, so `entitle.rs`, `account.rs`, the scheduler and the console's
`active || trialing` test need no change. `plan = 'tester'` is the only new word on the wire.

### 3.3 How a comp ends

- **By date:** `ends_at` is required at grant time and checked at read time (`now() < ends_at`), so no
  cron is needed and an expired grant can never linger as `active`. Extending writes a new `ends_at`.
- **Re-granting after expiry:** an expired grant that was never ended still has `ended_at is null`, so
  it holds the partial unique index and `comp_grant` for that address is refused (P12) until the sweep
  deletes it 30 days after `ends_at`. `admin.comp_extend` therefore **accepts an open grant whether or
  not its `ends_at` has passed** (it refuses only an ended grant, no grant, or a new `ends_at` not in
  the future), and P12's refusal message names it: "an open grant exists for this address; use
  admin.comp_extend". Extending an expired grant makes it `live` again at once (P13). After the sweep
  has deleted it, `comp_grant` works as for a new address.
- **Early:** the admin ends it (`ended_at`, `ended_reason`); the row stays for the record.
- **Effect:** the server gate refuses at once (402 on the next gated call). The app learns at its next
  `/entitlement` check (launch or 6-hourly) that the status is `none` and takes the normal
  not-entitled path: banner, cloud steps skipped as named steps, the upgrade overlay. The 72-hour grace
  covers an unreachable server only, never a fresh `none`, so it does not stretch a comp.
- **Account deletion:** an `after delete on public.accounts` trigger deletes that address's grants
  (note and reason go with them) **and its `comp_audit` rows by hash**
  (`email_hash = encode(sha256(convert_to(lower(old.email), 'UTF8')), 'hex')`, the audit's own
  expression), so the consent record stays the one record a deletion leaves behind, as the privacy
  page already says (Q8). The trigger function is `security definer` with a fixed `search_path`,
  because §3.5 takes every write right on both tables away from the service role that an account
  deletion runs as; a trigger function cannot be called over PostgREST, so the guard exempts it by kind
  as it does `handle_new_user`. `route_daily` rows go by `on delete cascade` (§4). A tester who deletes
  and re-joins needs a new grant.
- **The purge list.** `functions/account/index.ts`'s `purge` names every table holding the student's
  data "whether or not the cascade would also reach it", because the privacy page's deletion paragraph
  is written from it. S4 adds `route_daily` (by `account_id`) to that list, and a comment beside it
  naming `comp_grants` and `comp_audit` as deleted by the trigger, keyed on the address rather than the
  account id, so L4 writes the paragraph from a complete list. (`usage_daily` and `judgments` are not
  in today's list either; §8.6 hands that to L4 rather than widening this lane.)

### 3.4 How it shows in the app (it must not look like a paid plan or ask for payment)

Needs one `console-ui` change (S5; `app/static/console.js` and its markup, Quinn words the copy):
- **Wizard subscribe step:** on entering it, ask `checkEntitled()` first; an entitled account sees one
  line in place of the price buttons (suggested: "Your account is already set up. Nothing to pay.")
  and Next. A tester never sees a price, a plan choice or Checkout.
- **Settings account row:** for `plan === 'tester'`, "Free tester access through {date}. Nothing to
  pay." in place of `active · tester`, and *Manage subscription* hidden (today it would answer
  "this account has never subscribed").
- **The upgrade overlay, one fix.** It opens only when not entitled, but a friend can meet it before
  the grant is written (for example while prod grants wait on §8.5) and sign in again once it is.
  Today `afterUpgradeSignIn` (`console.js`) un-hides `#up-subscribe`, the price buttons, *before*
  `checkEntitled()` resolves, so the prices flash before the overlay closes. S5 asks `checkEntitled()`
  first and un-hides `#up-subscribe` only on a not-entitled answer (and on a failed check, so nobody is
  left with no way forward); an entitled answer goes straight to `finishUpgrade()`. No change to
  `account.rs`.
- **Belt and braces, cloud side:** `billing-checkout` refuses with 409 "this account has free tester
  access" **while the account is comped, that is, while `effective_entitlements` answers
  `plan = 'tester'` for it**, so no path, old build or stale page can start a charge (Q3). That is not
  the same as "while a grant is open" (revision 5). An open grant the view does not honour leaves
  checkout open, as for any unentitled account: one past its `ends_at` that was never ended (P8), or
  one matched to an account with a password (P7). The
  handler gains one injected dependency, a caller lookup that answers whether `effective_entitlements`
  shows `plan = 'tester'` for the caller (so the password rule and expiry are decided in one place,
  the view). **Production wires it in `functions/billing-checkout/index.ts`**, where the handler's
  other deps are built (its `getAccount` read of `accounts`); a fake in `handler_test.ts` alone would
  leave production never reading the view, so S4 also pins the wiring statically (§9).

### 3.5 Who can write it, RLS, the audit trail

- `comp_grants`, `comp_audit` and `route_daily` (§4): RLS on, **no policy**, `revoke all … from anon,
  authenticated` — the pattern of `mail_*` and `judgments`. No client reads or writes them.
- **The service role cannot write the grant tables.** RLS does not bind `service_role`, and every edge
  function holds that key, so on top of the client revokes: `revoke insert, update, delete, truncate on
  public.comp_grants from service_role` (it keeps `select`, which the export needs, §3.5 below) and
  `revoke all on public.comp_audit from service_role`. A mis-scoped `restUpsert` in any function is then
  refused by the database, not by convention (case A7). The table owner, which the migrations, the SQL
  editor, `pg_cron` and the `security definer` trigger run as, keeps every right. `route_daily` keeps
  the posture of `usage_daily` (the service role writes it, through `count_route`): it holds counts,
  and a stray write there misstates a cost, never grants access.
- **Writes only through three admin functions** in a schema `admin` that PostgREST does not expose
  (`[api] schemas` is `public` only) and whose usage is revoked from `public, anon, authenticated,
  service_role`, so no role but the owner can reach them:
  `admin.comp_grant(email, ends_at, cohort, note)`, `admin.comp_extend(email, ends_at)`,
  `admin.comp_end(email, reason)`; `admin.comp_sweep()` (§3.1) only deletes. There is no `actor`
  argument: the only writer is the project owner, and a free-text actor would invite a person's name
  (rule 1). `reason` is one of `done`, `asked`,
  `misuse`, `other` (a check constraint, no free text). They run from the dashboard's SQL editor as the
  project owner. No edge function writes a grant; no app build can.
- **Prod grants are prod writes:** the controller runs one only on Quinn's go (§1.7 of the handoff);
  the prod Supabase MCP is read-only and stays so.
- **`public.comp_audit`**, append-only, written by the three functions in the same transaction:
  `(id, grant_id → comp_grants on delete set null, email_hash, action in ('grant','extend','end'),
  ends_at, cohort, db_role, at)`. `db_role` is `session_user` (a database role such as `postgres`,
  never a person). **No free-text column**: no note, no reason text, no actor, no address. `email_hash`
  is sha256 of the lower-cased address, as `consents.subject_hash`. It is the record of what Quinn chose
  to pay for, and exactly two things delete an audit row: the account-deletion trigger, by hash (§3.3),
  and `admin.comp_sweep()`, once the row is three years old (`at < now() - interval '3 years'`, a
  maximum; Q8). No function updates one.
- **The sign-up check.** With confirmations off, anyone holding the public anon key can create an
  `auth.users` row for any address. OTP and Google sign-ins still prove the address before a session
  exists, but GoTrue's password `/signup` may hand back a session with no proof. So the view matches a
  grant only to an account whose `auth.users` row carries **no password**, and "no password" is one
  predicate, used verbatim in the view and pinned statically:
  `coalesce(u.encrypted_password, '') = ''`. GoTrue may store `''` rather than `NULL` for OTP and
  OAuth users, and both must read as "no password"; a test of `is null` alone would answer `none` to
  every OTP tester and show them the price step. The product never sets a password. S1 records, on
  staging, the literal value an OTP user, a Google user and a password sign-up carry; if the first two
  are not both `NULL` or `''`, or the third is, grants bind by account id instead (§3.1, last
  alternative); that is a spec revision before S2a, not tasks in this one (§8.9).
- **Export and deletion rights:** `GET /account/export` gains `comp_grants` (the caller's grants by
  `email_lower = lower(accounts.email)`: dates, cohort, note, `ended_reason`) and `route_daily` (the
  caller's rows, §4). Today's export also omits `usage_daily`, which the cost view reads; S4 adds it
  (Q12). Deletion is §3.3's trigger, `on delete cascade` and the purge list; retention is the sweep
  (§3.1), Q8 and Q11.

## 4. Usage and the cost estimate

**Recorded already, per account per day:** model calls, refunds and tokens for all six kinds
(`usage_daily`); each judgment's tier and outcome (`judgments`; tiers 0-2 never reach a model); mail
arrivals by phase (`mail_seen.seen_at`); Gmail messages judged (`gmail_seen.seen_at`).

**Estimate:** `est_usd = Σ_kind (in_tokens × usd_per_m_in + out_tokens × usd_per_m_out) / 10⁶`, joined
on kind at **today's pin** — the same arithmetic as `monthly_spend`, labelled an estimate in the view.
The MVP pins price a heavy day at a few cents (`judge_caps.ts`), so the per-account ceiling bounds a
tester at $2 a month plus a $1 backfill allowance once per 120 days.

**Added by this lane (Q5):** `public.route_daily(account_id uuid not null references public.accounts(id)
on delete cascade, day, route, calls)`, primary key `(account_id, day, route)`, the shape of
`usage_daily`. It is kept while the account lives, like `usage_daily` (Q11), goes with the account by
the cascade, and is in the caller's export (§3.5). A writing RPC, `count_route(account, route)`
(security invoker, one `insert … on conflict … do update`, execute for `service_role` only), refuses a
route outside `^[a-z][a-z0-9-]{0,39}$`.

- **Where it is called:** inside `requireActiveEntitlementWith`, after an entitled lookup, with the
  function name from the request path. The name is the **first path segment after an optional
  `/functions/v1` prefix**, the same tolerance as `_shared/http.ts`'s `subPath` (which keeps a request
  that never had the prefix, a direct invoke or `supabase functions serve`, routing the same): so
  `/functions/v1/judge-task` and `/judge-task` both give `judge-task`, and a function carrying several
  routes (`/functions/v1/<fn>/<sub>` or `/<fn>/<sub>`) gives `<fn>`. A path that yields no segment, or
  one outside the route pattern, is **not counted** (the gate skips the call rather than handing
  `count_route` a value it would refuse, since that refusal is swallowed by design and would leave
  `route_daily` silently empty). The parse is a small exported helper in `_shared/entitlement.ts`, so
  S3 stays within its files. A new **optional** dependency in `EntitlementDeps`,
  `count?: (accountId, route) => Promise<void>`, so every existing `{ verify, lookup }` constructor
  (tests and functions) still compiles.
- **Off until the privacy page says so.** `depsFrom` supplies `count` only when the function secret
  `ROUTE_COUNTS` reads exactly `on`; absent or any other value, it supplies none, and the gate behaves
  as today. Code and secret are separate on purpose: L9's prod parity pass deploys every function from
  `main`, so holding back the code would not hold back the writes, while a secret that is absent on
  prod by default does. The controller sets it on staging at S7, and on prod only on Quinn's go
  **after Q7's sentence 1 is on the live privacy page** (§8.5). Until then prod writes no `route_daily`
  row for anyone, and the migration that creates the empty table can ship. `depsFrom` reads the secret
  through an injected getter, defaulting to `Deno.env.get`, so the unit test needs no environment.
  **The switch includes a redeploy.** `requireActiveEntitlement` caches its deps per isolate
  (`cached ??= depsFrom(...)`, `_shared/entitlement.ts`), so `ROUTE_COUNTS` is read once per warm
  isolate: setting or unsetting it reaches only isolates started afterwards. Turning counts on or off,
  the rollback included, is therefore "set or unset the secret, then redeploy every gated function"
  (or otherwise force a cold start), and the controller checks the next call's `route_daily` row
  appears (on) or stops appearing (off) before calling the switch done. The cache stays as it is (it
  spares every gated call a deps build, and the switch is rare); §8.11 asks whether an off-switch that
  binds warm isolates at once is wanted instead.
- **Never on the critical path:** the gate does not await the count. It starts the promise, attaches a
  catch that swallows every error, and hands it to an optional `defer?: (p: Promise<unknown>) => void`
  dependency. Production's `defer` is `EdgeRuntime.waitUntil` when that global exists (the edge runtime
  then keeps the isolate alive until the write lands) and a no-op otherwise. A count that cannot land is
  lost; the call is never refused or slowed by it.
- **What it covers:** every route behind `requireActiveEntitlement`: judge-* (including
  `judge-rules`), ingest-*, events, sync-*, gmail-read, the mail routes a student's token calls
  (`mail`, `mail-pull`, `mail-resolve`), and `account` and `google-connect` on the paths where they
  take the gate (on the email branch, `account`'s `PUT` of a source and `google-connect`'s connect;
  `account`'s other routes and `google-connect`'s `?status=1` and `DELETE` skip the gate and are not
  counted). The view's comment names these routes. **Not counted:** `mail-inbound` and
  `mail-canary`, which call `lookupFrom`/`isActive` directly (§2) and are not this lane's files. Their
  volume per account already reads from `mail_seen` (arrivals by phase), and the view's comment says
  so, rather than implying `route_daily` is complete.

**Still missing, named in the view's comment:** the model per call (a pin change re-prices history),
the provider's billed cost (reconcile the month's total against the OpenRouter dashboard by hand),
and infrastructure per account (S3, SES, egress, invocations are not attributed; read the project's
own billing pages).

## 5. The admin view — recommended form: SQL views in `admin`, read in the SQL editor

- `admin.testers`: one row per grant — email, cohort, state (`waiting for sign-up` / `live` /
  `ended` / `expired`), account id once one matches, granted, ends, last active day.
- `admin.tester_usage_daily`: per tester per UTC day — calls, refunds, in and out tokens and
  `est_usd` per kind; judgments by tier (model vs rule vs deterministic); mail arrivals by phase;
  Gmail messages; `route_daily` calls per route (inbound mail volume is the `mail_seen` columns, §4).
- `admin.tester_spend_monthly`: per tester per month `est_usd`, a total row, and any `budget_alerts`.
- `admin.testers`' state is derived at read time, in this order: `ended` (`ended_at` set), `expired`
  (`now() >= ends_at`), `waiting for sign-up` (no account whose address matches **and** that passes
  the password rule), otherwise `live`. The predicates are the view's own (§3.2), so a grant is `live`
  exactly when it entitles its account (the view answers `tester`, unless an active Stripe row wins,
  which `admin.testers` shows as a separate `paid` flag).
- Counts and dollars only: no item id, title, field value, sender or message key appears in any of them.
- Views default to owner rights, so each also carries `revoke all … from anon, authenticated` (the
  `migrations_test.ts` guard demands it) on top of the unexposed schema.
- **Alternative (not recommended for the MVP):** an `admin-usage` edge function gated on an
  `ADMIN_ACCOUNT_IDS` secret, plus a small page. It puts tester data on an internet-facing route and
  adds a secret, for a reader who already has the dashboard. Revisit when someone other than the
  project owner needs to read it.

## 6. Checks

- **VISION, one edition and no free tier (D2):** a comp is a hand-granted, dated exception for named
  people Quinn pays for, never a plan anyone can choose, the same shape the signed pilot ruling
  ("the pilot's students do not pay") already needs. Confirm in §8.2.
- **VISION, telemetry read only with a minimum cohort:** the cost view reads per-account metering
  (`usage_daily` already exists as the cost guard), not interaction telemetry, and shows counts only.
  It is still a per-person view, so testers are told (Q7) and Quinn confirms the reading (§8.2).
- **Rule 1:** no address, name or id of any tester in code, migrations, fixtures or tests; grants are
  data written on prod; the admin is "the project owner", never a named account. The migrations-test
  tripwire (§9 layer i, S2a) guards what rule 1 forbids, not a statement the admin functions need:
  (a) **no address-shaped token** (`[A-Za-z0-9._%+-]+@[A-Za-z0-9.-]+\.[A-Za-z]{2,}`) appears anywhere in
  the raw text of any `.sql` migration, comments and `$$` bodies included (today's corpus has none, so
  the guard is green on arrival); and (b) every `insert`, `update` or `delete` naming `comp_grants` or
  `comp_audit` lies inside the body of a named `admin.comp_*` function or the deletion trigger,
  never at top level or in a `do` block, and an insert into `comp_grants` carries **no quoted literal**
  in its values (only parameters and expressions). "A write" means exactly the three DML shapes
  `insert into <t>`, `update [only] <t> … set` and `delete from [only] <t>`, with `<t>` bare,
  `public.`-qualified or quoted; a word such as `insert` or `delete` elsewhere in a statement is not a
  write. So S2a's own `revoke insert, update, delete, truncate on public.comp_grants from service_role`,
  comp_audit's `grant_id … references public.comp_grants on delete set null`, the partial unique index
  and a `comment on table public.comp_grants` are not writes and must pass (§9's must-pass cases).
  Both parts scan `migrations/*.sql` only; the staging proof under `proofs/` (§9 layer iii), which
  holds `@example.invalid` addresses and backdates rows as the owner, is outside them and has its own
  pin. Either part alone refuses
  `do $$ begin insert into public.comp_grants values ('friend@…') … $$`; together they let
  `admin.comp_grant`'s parameterised insert stand with no edit to the guard.
- **Rule 2:** no frozen reference or read-model oracle is touched.
- **Contract list:** untouched (`entitle.rs`, `wincred.rs`, `account.rs`, `sync.rs` stay as they are).
  The list's "entitlement tests" are the engine's, `engine/tests/entitlement_gate.rs`, which this lane
  does not edit. The cloud tests S3 and S4 edit (`functions/_shared/entitlement_test.ts`,
  `functions/entitlement/handler_test.ts`, `billing-checkout/handler_test.ts`,
  `account/handler_test.ts`) are `cloud/` files and cloud-engineer's lane, not contract-list files, so
  S6's reviewer reviews them as `reviewer` and needs no `contract-reviewer` hand-off. (If the
  controller reads the list otherwise, it says so before S3 is dispatched; §8.8.)

## 7. Decisions for Quinn (money and privacy; recommendation first)

- **Q1. When a friends grant ends.** *Recommend:* every grant carries its own `ends_at`; the default
  for `friends` is **2027-01-31** (the pilot starts in spring 2027), extended by `admin.comp_extend`.
  Alternatives: 90 days per grant; open-ended (not offered: every grant must end).
- **Q2. Testers' monthly ceiling.** *Recommend:* the same $2.00 runaway guard as everyone, plus the
  $1.00 backfill allowance, so a tester is bounded like a paying student (≈$3 worst case a month; ten
  testers ≈$30). Alternative: a raised ceiling for `cohort = friends` (a per-cohort ceiling in
  `enforce_budget`). No other lane edits that function (L3's plan, Q-F: L3 neither edits nor re-issues
  it), so the cost of the alternative is this lane re-issuing a money-path function last issued in
  `20261002000200_mail.sql`, with its own pins, not a collision.
- **Q3. Paid vs comp.** *Recommend:* while the account is comped (the view answers `tester` for it),
  `billing-checkout` refuses, so a friend can never be charged by accident. If a Stripe row is active
  anyway, it wins. A grant that no longer entitles does not refuse checkout: one past its `ends_at`
  (P8), or one matched to an account with a password (P7). That friend sees the price step, as any
  unentitled account does. *(Revision 5 aligned these words with the mechanism. The earlier "while a
  grant is open" would also have refused checkout for P7 and P8.)* Alternative: allow a friend to
  subscribe while comped (no refusal).
- **Q4. The cost basis.** *Recommend:* tokens × today's pin for the MVP, reconciled monthly by hand
  against the OpenRouter dashboard. Alternative (v1): record model and dollars per call in
  `usage_daily` at `record_tokens` time (touches `judge_caps.ts`, L3's file).
- **Q5. Counting gated routes.** *Recommend:* `route_daily` for **every** account (counts only; the
  pilot needs the same numbers to measure cost per student). That makes it a fact about paying
  students too, so Q7's first sentence (for everyone) is part of this recommendation, not optional,
  and prod counts nothing until that sentence is live: the `ROUTE_COUNTS` secret stays off on prod
  until then (§4, §8.5). Alternatives: comped accounts only (then Q7's tester sentence alone suffices, and the gate needs the
  comp state before it counts); or no route counts, leaving ingest volume out of the view.
- **Q6. Pilot reuse.** *Recommend:* the `cohort` check allows `friends` and `pilot`, so the pilot's
  ~30 non-paying students need no new mechanism. Alternative: `friends` only, widened when the pilot
  is planned.
- **Q7. What the privacy page says.** *Recommend:* two sentences on the page L4 is rewriting, both in
  L4's PR **before** its `PRIVACY_VERSION` bump (§8.5):
  1. For everyone (follows from Q5): "We keep daily counts of your AI calls and cloud requests, never
     what they were about, to keep the service's cost in bounds. They are in your export and are
     deleted with your account."
  2. For testers: "If Knowlu gives you free tester access, we keep the address you gave us until 30
     days after that access ends, and a one-way hash of it with the dates of that access for at most
     three years, or until you delete your account; we look at your daily counts and their cost to pay
     for them."
  With Q8's recommendation, the page's existing line that the consent record is "the one record a
  deletion does not take with it" stays true and needs no change. Alternative: tell friends personally
  only. That covers sentence 2 only; sentence 1 is still owed if Q5 stands.
- **Q8. Audit retention.** *Recommend (changed in revision 2):* three years is a **maximum**, held by
  `admin.comp_sweep()` (A6), and the account-deletion trigger deletes the account's audit rows by hash
  (A3), so no audit row outlives the account it describes. The row holds only the address's hash, the
  action, the dates, the cohort and the database role (§3.5). After a deletion nothing per person
  remains; the cost record that survives is the provider's and Supabase's billing totals, which name no
  one. Alternative: keep the audit through a deletion, three years as a
  minimum (the earlier draft). That needs a **third privacy sentence** ("If you had free tester access
  and delete your account, we keep a one-way hash of your address and the dates of that access for
  three years") and a change to the page's "the one record a deletion does not take with it" line,
  both in L4's page before its bump.
- **Q9. A spending stop across all testers.** *Recommend:* none for the MVP; the per-account ceiling
  bounds the total at about $3 × testers, and `admin.tester_spend_monthly` shows it. Alternative: a
  global comp budget that ends every friends grant's model calls past a dollar figure.
- **Q10. How the SQL is proved.** There is no local database: `migrations_test.ts` applies no SQL and
  nothing under `cloud/` opens one (L3's plan, Q-C, ruled the same). *Recommend:* the three layers L3
  uses, with the staging proof as a hard gate before prod (§9): (i) static pins on the exact shape of
  every predicate; (ii) a TS model of §3.2 and `admin.testers`' states, driven by one case table;
  (iii) the same case table run as real SQL on staging, in a `begin … rollback` script, before any
  function reads the view and before L9 takes the migration to prod. Alternative: a local Postgres
  harness (a new task, and RAM the session does not have, handoff §3).
- **Q11. Retention.** *Recommend:* grants (address, note, reason) are deleted 30 days after they close,
  claimed or not, by a daily sweep (§3.1); an open, never-claimed grant holds its address until
  `ends_at` (with Q1's default, at most until 2027-01-31). `route_daily` is kept while the account
  lives, like `usage_daily`. Alternatives: delete a grant the day it closes (the record is then the
  audit alone); a fixed 400-day window on `route_daily` (needs a second sweep).
- **Q12. `usage_daily` in the export.** Today's `GET /account/export` omits `usage_daily`, though it is
  per-account data. *Recommend:* S4 adds it beside `route_daily` (one line each, paged like
  `consents`). Alternative: `route_daily` only, and file the `usage_daily` gap for L4's privacy pass.

## 8. Open questions for the controller (not money)

1. **The console.js queue.** The handoff's queue (L4 → L7 → L1 R11 → L5) had no L8 slot. **Ruled by
   Quinn, 2026-10-19 (option (a) of the L4 plan's review round 1, finding C1): S5 goes ahead of L4's
   T8**, and the queue is **L6 → L8 S5 → L4 → L7 → #19 → L1 R11 → L5**, covering
   `app/static/index.html` as well as `console.js`. S5 also edits `index.html`, which **L6** touches
   first (it removes the wizard's model-download step, its "app/static wizard bits"); **L4** adds the
   re-consent view after S5, and L4's T8 rebases over S5. So S5 starts once L6 has merged, rebasing
   onto the wizard as L6 leaves it, and waits for no L4 task. L8 can then merge before L4's
   `PRIVACY_VERSION` bump, and the page goes live with both §8.5 sentences true. Before the ruling,
   this item recommended S5 after L7, and only after both L6 and L4 had merged. That deadlocked: L4's
   bump waits for L8's merge (§8.5, S8), and L8 could not merge without S5. S5 adds no Tauri command
   (`checkEntitled()` exists), so it hands nothing to L6's `main.rs`/`commands.rs`.
2. **VISION:** "No one else signs up before the privacy page … a lawyer has read it …" (Build order,
   the pilot; `VISION.md:221` in the main checkout) reads as gating friends too, while ruling 3 of
   2026-10-18 says nothing waits on the lawyer.
   - **What is signed (re-read 2026-10-19).** The cloud design's Amendment 2026-10-19, signed, makes
     L4's interim bump "privacy bump #1" without a lawyer read, and keeps the lawyer read before the
     Pilot. The rest of the 2026-10-18 amendment, ruling 10's Gate rewording, is "not written or
     signed". So VISION's sentence still stands as written.
   - **The second reading.** The telemetry rule ("read only as aggregates with a minimum cohort",
     `VISION.md:171`) can be read as forbidding §5's per-tester view. §6 argues that it does not.
   - **Consequence (revision 5).** If Quinn reads the build-order sentence as covering friends, S7's
     first prod grant waits on a signed amendment. Otherwise the lane would be built against an
     unsigned reading. S1-S6 and S7a write no prod grant and need not wait. The same holds for the
     telemetry reading and S7's prod `ROUTE_COUNTS=on`.
   - **Recommendation.** Quinn signs one amendment line in the cloud design: named friends testers on
     a comp may sign up once bump #1 is live, before the lawyer read; the per-tester cost view is
     metering read by the project owner, not telemetry. VISION's sentence then keeps its force for the
     pilot and for paying strangers.
   - Also confirm §6's reading of D2. (Rule and VISION changes are Quinn's.)
3. **The password sign-up path** (§3.5) is wider than this lane: a session for an unproved address.
   Hand it to L9's C1b settings check, **with one consequence named for this lane (revision 5)**.
   Grants bind to `accounts.email`, which is not unique, and nothing updates it when
   `auth.users.email` changes: no migration has an `after update` trigger for it. With
   `enable_confirmations = false`, GoTrue confirms a `PUT /user` email change by itself. So two
   `accounts` rows can carry one address. The scenario:
   - someone password-signs-up with a friend's address (P7: the view refuses them);
   - they change their auth email, which frees the address in GoTrue but not in `accounts`;
   - the friend signs up by OTP, and their grant goes live;
   - the first person then deletes their account.

   §3.3's deletion trigger deletes every grant and audit row for `old.email`, so the friend's live
   grant ends silently. In the same state, `admin.testers` shows two account ids for one grant. This is
   denial of service only, and only inside the password path. It grants no one access, and P7 already
   refuses the impostor. Closing that path (C1b: password sign-up off, or confirmations on) closes this
   too. Until L9 does, the controller checks `admin.testers` for a grant with two account ids before
   each new grant and after any reported loss of access. This lane adds no trigger on `auth.users`.
4. **Migration stamp: the shared rule (revision 5, 2026-10-19).** The email, L3, L8, L5 and L1
   migrations follow one rule, M1-M8, stated in full after this list ("The migration-stamp rule"). The
   same text stands in L3's plan (`2026-10-18-first-day-allowance-plan.md` §2.1). For this lane:
   - **S2a fixes the stamp (M3).** The controller passes S2a the stamp L3's T0 recorded in L3's lane
     ledger. S2a takes the next free stamp after that, after every file on `main` and after every
     version staging holds, and never one that L5's or L1's ledger already records. If L3 has not run
     T0 when S2a is dispatched, S2a waits for it rather than guessing.
   - S2a writes the stamp to this lane's ledger before it creates `<stamp>_testers.sql`. The tests find
     the file by `^\d{14}_testers\.sql$` (M4).
   - **S7a is this lane's first staging push**, so M5's re-check runs immediately before it, after the
     lane's last rebase. Then `supabase db push --dry-run`: if it refuses, or lists any version other
     than this lane's own, S7a stops and does not push (M7). The name is frozen from that push (M6).
   - S7a pushes only once L3 is cleared (M7): L3's migration on staging and on `main`, or L3's slip
     recorded by the controller, which it may do only while L3 has neither merged nor pushed. L3
     having merged is not enough, but by M7's merge-with-push rule L3 merges only once its push is
     allowed and pushes straight after, so a merged L3 never holds S7a for long. L3 having pushed from
     its lane branch (L3's Q-G proof-first) is not enough either: S7a's dry run refuses while staging
     holds L3's version and `main` lacks it, so S7a waits for L3's merge or the controller's withdrawal
     of L3 (M7), and never runs `migration repair` to get past the refusal.
   - A rename before S7a is one `git mv`, a pin recount (§8.7) and a dated ledger row. After S7a, only
     the controller renames, by M6, or withdraws the lane, by M7 (§8.10); after a withdrawal, M5 runs
     again before any new push.
5. **The privacy sentences and L4's bump.** Q7's two sentences must be in L4's page before L4 bumps
   `PRIVACY_VERSION`, or testers accept a page without them and the version moves twice. Recommend the
   controller put Q7 to Quinn at L4's page-reading checkpoint, which L4 already has. If Q7 is not
   signed by then, L4 ships without the sentences, and until a later bump carries them on the live page
   two things wait (S7): **no prod grant is written** (sentence 2), and **`ROUTE_COUNTS` stays off on
   prod** (sentence 1), so the gate every paying student passes writes no `route_daily` row there. The
   comp code, the migration and the functions can still merge and deploy, because with the secret off
   the deployed gate counts nothing. "Live" means the page the app shows at the accepted
   `PRIVACY_VERSION`, read by the controller before it sets the secret or runs a grant.
6. **`account/index.ts`** (the cloud `account` function, S4) is not on the contract list and no lane
   claims it; L4 owns `app/src/account.rs`, a different file. Confirm L4 does not also edit the cloud
   export, or order S4 after it. S4 also touches its `purge` list (§3.3): `usage_daily` and `judgments`
   are per-account tables missing from that list today, though the list's own comment says the
   privacy page's deletion paragraph is written from it. Recommend L4 add them in its privacy pass
   (or S4 adds them, one line each, if the controller prefers it in this lane).
7. **Non-append edits to the shared migrations test (collisions).** The lane appends its own test
   block at the file end, and makes exactly these in-place edits, which L3, L5 and L1 may also make
   in merge order (email → L3 → L8 → L5 → L1):
   - **`identPattern` (top of file), S2a0, its own commit.** It accepts only a bare name or
     `public.<name>`, so `FUNCTION_DEFINITION`, `VIEW_DEFINITION`, the revoke lookups and the
     `delete from` scan cannot see `admin.*`. The change is one line and stays strict: the optional
     prefix becomes `(?:"?(?:public|admin)"?\s*\.\s*)?`, never a fully optional or any-schema prefix,
     so a function in a third schema still fails the parse-count check loud.
   - **The function parse-count pin** (`assertEquals(parsed, 40, …)` and its hand-count comment, in
     the execute-revoke test, `:333` on the email branch). L3 moves it too (its plan, T1: 40 → 42).
     On this design the lane adds six function definitions (`admin.comp_grant`, `comp_extend`,
     `comp_end`, `comp_sweep`, `count_route`, the deletion trigger function); each task moves the pin
     by its own and adds its line to the comment.
   - **The view parse-count pin and the `revoked` name list** (`assertEquals(parsed, 5, …)` and
     `assertEquals(revoked, […])`, `:349` and `:353`, and their comment). The lane adds four views
     (`effective_entitlements`, `testers`, `tester_usage_daily`, `tester_spend_monthly`), each
     revoked, none `security_invoker`, so `invoker` stays `["billing_subscribers"]`.
   On rebase, L8 keeps L3's lines and recounts both pins against the rebased corpus (the sum, never
   either side's literal); L5 and L1 do the same onto L8's. Recommend the controller tell L5's and
   L1's planners three things: the widened pattern is in place (point them at S2a0's commit, so neither
   re-widens it differently); both pins and the `revoked` list will have moved; and **the rule-1
   tripwire's address-token ban (§6 (a)) scans every `.sql` migration, theirs included, comments and
   `$$` bodies too**. A comment such as `-- e.g. k7x9@in.knowlu.com` or `x@example.invalid` turns
   their CI red on L8's test. Write an example address without the `@` part (`<token> at
   in.knowlu.com`) or keep it in a `.ts` test; never loosen the ban for one file.
8. **The contract list's "entitlement tests"** (§6): this spec reads them as `engine/tests/entitlement_gate.rs`
   only. Confirm before S3 is dispatched; if the cloud entitlement tests count too, S3 and S4 go to
   `contract-engineer` and S6 to `contract-reviewer`.
9. **If S1 finds the password rule unusable** (an OTP or Google user carries a hash, or a password
   sign-up does not), email binding is wrong and this spec has no tasks for the account-id fallback.
   Then S2a does not start: the spec returns for a revision (grants keyed by `account_id`, granted
   after sign-up, so a tester meets the subscribe step once and the wizard change of §3.4 covers it
   only after the grant), and Q1 and Q7 go back to Quinn to re-read under it.
10. **The staging proof runs before the merge (S7a), so the migration reaches staging from the lane
    branch.** That way `main` never carries `lookupFrom` on an unproved view, and a P1 failure stops
    the lane with nothing on `main` to revert. S7a runs after the lane's last rebase onto `main`, so
    the stamp it pushes is the one that merges (M5, M6). Costs: from S7a until S6's merge, staging's
    `schema_migrations` holds a version `main` lacks, so every dry run on `main` refuses and L5's push
    and L1's C1 wait for S6's merge (M7); Quinn's S6 go gates them too, and this lane cannot be
    recorded as slipped while its version is on staging. L3 is cleared first: its migration on staging
    and on `main`, or L3, having neither merged nor pushed, recorded as slipped (M7; neither L3's merge
    alone nor its pre-merge push alone clears S7a), and S7a's `db push --dry-run` must list this lane's
    version alone (M7), so no push needs `--include-all` and no other lane's migration rides along.
    **If the lane stops after S7a**, the controller withdraws it by M7's withdrawal clause, recording
    each step in the session and lane ledgers. Who decides follows Quinn's ruling of 2026-10-19 (M7):
    a P0, P1 or P3 failure is a defect in the SQL, because the layer-iii script tests the migration
    alone, so the controller withdraws on its own and tells Quinn; Quinn declining S6's go is Quinn's
    word; any other stop goes to Quinn first. S7a deploys no function, so nothing is redeployed and
    the withdrawal opens no window. In one `begin … commit`, it drops on staging every object
    `<stamp>_testers.sql` creates, as the file names them (the grant, audit and `route_daily` tables,
    the four views, the `admin` functions and schema, `count_route`, the deletion trigger on
    `public.accounts` and its function, and the `knowlu-sweep-comp` job). It then checks the
    catalogue for each of them, `cron.job` included, and records that none is left (anything left
    stops it, the version row in place). Then it removes the version row with
    `supabase migration repair --status reverted <stamp>` and confirms that a dry run on `main` no
    longer names it. This lane then counts as neither merged nor pushed: it no longer holds L5 or C1,
    it may be recorded as slipped, and M5 runs again before any new push. The migration is additive (an unread view, empty
    tables, unreachable admin functions), so no other lane's staging work sees it. Confirm the
    controller accepts a pre-merge staging push for this lane. The alternative is to merge first and
    hold every staging function deploy from `main` until S7. That alternative leaves the gap the
    review found if another lane deploys from `main` in between.
11. **The `ROUTE_COUNTS` off-switch binds only new isolates (§4).** Because the gate's deps are cached
    per isolate, unsetting the secret stops counts only after a redeploy of every gated function. This
    spec accepts that and makes the redeploy part of the switch, so that S3 leaves the cache alone.
    The alternative is for S3 to read the secret on each call inside the supplied `count`, so an unset
    takes effect at once with no redeploy, at the cost of one env read per gated call. Recommend the
    redeploy. Confirm, because the off-switch is the privacy rollback.

**The migration-stamp rule (shared by the email, L3, L8, L5 and L1 migrations; written 2026-10-19;
M5, M7 and M8 revised the same day after the stamp-rules review and its re-review, and M7 and M8
again after the second and third re-reviews; M1, M5, M7 and M8 given their final pass, r8, after the
fourth re-review passed, with Quinn's withdrawal ruling of 2026-10-19 in M7).**
The same eight points appear here (§8.4) and in L3's plan (`2026-10-18-first-day-allowance-plan.md`
§2.1). If the two copies ever differ, the controller re-aligns them to the newer dated copy before
either lane dispatches a migration task. The rule rests on handoff §5's merge order and on Quinn's C3
Q8 ruling of 2026-10-19: "C1 waits until L3, L8 and L5 have merged, or until the controller records
that one slipped past C3; the prod deploy stays a plain push."

- **M1. Order.** Stamps sort in the merge order email → L3 → L8 → L5 → L1 (handoff §5; L5a, PR #19,
  adds no migration). The email branch's two migrations are fixed: `20261002000200_mail.sql` and
  `20261005000100_mail_claim_max_tokens.sql`. Any migration the email branch adds before its merge
  follows M3 and sorts after them. A migration a lane adds after its own merge (a follow-up fix to
  merged SQL, whose file M6 froze) joins this order as a new lane at the back, behind every lane then
  on record, and follows M3, M5 and M7 like any lane.
- **M2. Format and the next free stamp.** A stamp is fourteen digits (the root `migrations_test.ts:35`
  check). It is the date the stamp is fixed, followed by a step counter in the time field: `000100`,
  `000200`, `000300` and so on, never a wall-clock time. **The next free stamp after X** is today's
  date with `000100` if that sorts after X. Otherwise it is X plus `000100`. This definition replaces
  the C3 plan's earlier "one minute after the newest on `main`" (its r6 §5), which gives a different
  stamp whenever the newest is from an earlier day; the C3 plan's §5 now cites M2.
- **M3. Picking.** A lane fixes its stamp when it creates its file: L3 at T0, L8 at S2a, L5 at T6a,
  and L1 at R6 (provisional, per the C3 plan §5). The stamp is the next free stamp after the greatest
  of three sets:
  - every migration on `main`;
  - every stamp that a lane ahead of it (M1) has recorded;
  - every version staging has applied.

  The lane records the stamp in its own ledger before it creates the file, and the controller passes
  the stamp to every lane behind it. No two lanes ever record the same stamp. A stamp chosen before
  every lane ahead has recorded its own is **provisional**: L8 waits for L3's (its S2a), and L5 and L1
  may go ahead and rely on M5's re-check.
- **M4. Tests never name a stamp.** Each lane's tests find its file by its suffix, as
  `mail-process/worker_test.ts:959-967` finds `_mail.sql`: `^\d{14}_event_allowance\.sql$`,
  `^\d{14}_testers\.sql$`, `^\d{14}_mail_upload\.sql$`, and L5's own suffix. So a rename changes no test
  line. The only lines it moves are the shared parse-count pins in `migrations_test.ts`, which the lane
  recounts on its rebased tree (the sum of everything there, never either side's literal). The last
  lane to land owns the final numbers.
- **M5. The re-check before the first push.** Immediately before a lane's migration first reaches
  staging, or reaches it again after a withdrawal (M7), its stamp is checked again against M3's three
  sets, on the lane branch rebased on `main`. Under M7 that is before the lane's merge (whose push
  follows at once) or before a push from its lane branch, whichever comes first. If anything sorts at
  or after its stamp, the file is renamed (`git mv`) to the next free stamp, the pins are recounted,
  and a dated row goes in the lane ledger. That is the last free rename unless the controller
  withdraws the lane (M7). Only a migration merged against M7 (its push did not follow) reaches its
  first push with no lane branch: then the check runs on `main`, and a needed rename is M8's re-stamp
  commit, never M6's repair.
- **M6. Frozen from the first push.** Once staging's history holds the version, the name is frozen,
  because `supabase db push` refuses to run while a remote version has no local file. Any later rename
  is the controller's alone, done as one step with both stamps in the ledger:
  - on staging, `supabase migration repair --status reverted <old stamp>`;
  - the `git mv`;
  - then `supabase migration repair --status applied <new stamp>`. The SQL already ran under the old
    name, so it is not run again. The C3 plan §5's `db push` variant re-runs the file, so it suits only
    an idempotent file such as L1's (its P11).

  No agent renames a frozen file.
- **M7. Merge with push, and staging push order.**
  - **A migration merges only with its push.** A lane's migration merges to `main` only once its
    staging push is allowed, and the controller pushes it to staging from `main` right after the merge
    (unless the lane already pushed from its lane branch). "Merged but not pushed" is a transient
    state, never a waiting one. A lane whose push waits on a Quinn word does not merge before that
    word (L3's T5 waits on Q-E, so L3's merge waits for Q-E as well as Q-G).
  - **Clearing.** A lane pushes its migration to staging only after every lane ahead of it is
    cleared. A lane ahead is cleared once its migration is **on staging and on `main`**, in either
    order. Either half alone holds every lane behind it:
    - **Merged, not on staging:** its file is on `main`, so every branch rebased behind it carries the
      file and a plain `db push` would apply it.
    - **On staging, not merged** (pushed from its lane branch): staging holds a version `main` lacks,
      so a dry run on `main`, or on any branch rebased on it, refuses. The lane holds every lane behind
      it until it merges or the controller withdraws it (below).

    A lane that has **neither merged nor pushed** is also cleared once the controller has recorded it
    in the session ledger as having slipped behind. A recorded slip clears no other lane.
  - **Withdrawal.** A lane that pushed from its lane branch and will not merge as pushed leaves
    staging only by withdrawal, and only the controller withdraws a lane. When it may (Quinn's
    ruling, 2026-10-19):
    - **The lane's staging proof fails**, meaning it shows a defect in the lane's SQL: the controller
      withdraws the lane on its own, records the withdrawal in the session ledger and tells Quinn. A
      defect in the lane's functions alone is no withdrawal: they are fixed on the lane branch and
      redeployed from it, and the migration stays as pushed.
    - **Quinn declined the lane's merge:** that counts as Quinn's word, and the controller withdraws.
    - **Any other case** (for example, Quinn's word on the merge still unanswered after a pre-merge
      push, a failure the controller cannot place in the SQL or the functions, or a lane that stopped
      for another reason): the controller asks Quinn first.

    Five steps, in order, each recorded in the session ledger and the lane ledger:
    - Redeploy from `main` every function the lane deployed to staging from its branch (none, if it
      deployed none). `main`'s code ignores what the lane added, provided every column the lane added
      to an existing table is nullable or has a default. Where the lane changed an existing object
      that `main`'s functions call, the lane's own withdrawal step names the window between this
      redeploy and the next step.
    - On staging, in a single `begin … commit`, drop every object the lane's migration created, as
      its file names them (tables, views, functions, triggers, `pg_cron` jobs, schemas, and columns it
      added to existing tables), and put back `main`'s definition of any object it changed.
    - Check the catalogue for every object the file names (tables, views, functions, triggers,
      columns, `cron.job`, schemas), and record that nothing is left and that each changed object
      reads as `main` defines it. If anything is left, stop: the version row stays.
    - Then remove its version row: `supabase migration repair --status reverted <stamp>`. Once the
      objects are gone this is safe, because no later push can meet them.
    - Run `supabase db push --dry-run` on `main` and see that it no longer names the lane's version.

    The lane then counts as neither merged nor pushed: its name is no longer frozen (M6), it may be
    recorded as slipped, and M5 runs again before any new push.
  - **A dry run before every staging push and every merge.** Every staging push, from a lane branch
    or from `main`, first runs `supabase db push --dry-run`. If the dry run refuses, or its list holds
    any version other than the pushing lane's own, the controller stops, records the version, and does
    not push. So no migration ever reaches staging inside another lane's push. A refusal that names a
    remote version with no local file means a lane ahead pushed before merging; the controller
    records every such push in the session ledger when it runs, so the ledger names the lane. The
    lane behind waits for that lane's merge or its withdrawal, rebases on `main` and runs the dry run
    again. A remote version that no lane's push recorded is not waited on: the controller stops and
    asks Quinn. `supabase migration repair` is never run to get past a refusal; it runs only for M6's
    rename of a frozen file and in a withdrawal. A lane does not merge its migration until a dry run
    on its lane branch, rebased on `main`, lists its own version alone (or nothing, once it has
    pushed from that branch).

  That keeps staging's history in stamp order, so no planned push uses `--include-all`. Two waits,
  spelled out:
  - L1's C1 waits until each of L3, L8 and L5 is cleared: **merged** (C3 Q8, ruled 2026-10-19) with
    its migration on staging, or, having neither merged nor pushed, recorded as slipped. That is
    stricter than Q8's words in two ways, and Quinn hears both at the next checkpoint: Q8 names merges
    only, and Q8 lets the controller record any lane's slip, where M7 allows it only for a lane that
    has neither merged nor pushed (a withdrawn lane counts as one). Under the merge-with-push rule the
    first adds a wait only between a merge and its push; the second holds C1 behind a lane that pushed
    before merging until it merges or is withdrawn. L8 always pushes before merging (its S7a), so C1
    also waits for Quinn's go on L8's merge (its S6).
  - L8's S7a pushes from its lane branch only once L3 is cleared (L8 spec §8.10). L3 having merged is
    not enough, though under the merge-with-push rule a merged L3 is on staging moments later. L3
    having pushed from its lane branch (L3's Q-G proof-first) is not enough either: S7a then waits for
    L3's merge or L3's withdrawal.

  L3 is first after the email branch, and its T0 branches only after the email merge, so it waits
  only for the email branch's migrations to be on staging, which M3's read of staging at T0 shows. At
  T0 the controller also checks that every version staging has applied has a file on `main`. If one
  has none, it belongs to no lane on record, and the controller stops and asks Quinn before L3 fixes
  its stamp. After that check, every version on staging that `main` lacks is a lane's pre-merge push,
  recorded in the session ledger when it ran.
- **M8. A slip, and prod.** A lane recorded as slipped (only a lane that has neither merged nor
  pushed can be, M7) drops behind the lanes that overtake it.
  - M5 on its lane branch, before its merge, gives it a stamp after theirs.
  - A lane that pushed from its lane branch cannot be overtaken while its version is on staging: until
    it merges it holds every lane behind it (M7), so it is never recorded as slipped, and its frozen
    stamp keeps its place. Once the controller withdraws it (M7), it has neither merged nor pushed, and
    the bullet above applies.
  - **If M7's merge-with-push rule is ever broken** (a migration merged and its push did not follow),
    the controller's re-stamp commit on `main` is the remedy, before that migration's first staging
    push. One commit: the `git mv` to the next free stamp after M3's three sets (same suffix), the
    pins recounted, both stamps in the session ledger and the lane's ledger. It lands as a one-commit
    PR off `main` on green CI, and the push is then a plain `db push` from `main`. Staging never
    applied the old stamp, so the file is not frozen and no `migration repair` runs (M6 is only for a
    version staging holds). The same remedy answers a staging push that asks for `--include-all`
    because a merged, unpushed migration sorts too early. A re-stamp fixes the order only: the file is
    still on `main`, so M7's dry run stops every push from a branch that carries it until its own push
    is allowed and done, and the controller tells Quinn. If prod already holds the old stamp, the file
    is frozen there: the controller asks Quinn instead.
  - **If the broken merge sits behind a lane that pushed from its lane branch and has not merged**,
    the re-stamp cannot help. The lane ahead's merge dry run lists the stray file, and the stray file's
    push from `main` refuses on the lane ahead's version, so each waits on the other. The controller
    pushes nothing from `main` and offers Quinn two remedies; the choice is Quinn's:
    - **Revert the broken merge** in one commit on `main`. The reverted lane is then neither merged
      nor pushed again (M5 runs on its lane branch before its next push). It lands again by reverting
      that revert in its next merge, or with its commits re-applied on a branch cut from `main`. A
      plain rebase of the old lane branch onto `main` drops them silently, because git counts them as
      merged.
    - **Withdraw the lane ahead** (M7), which rewrites nothing on `main`. The controller records the
      withdrawn lane's slip and pushes the stray migration from `main` once its own push is allowed
      (its dry run then lists it alone). The withdrawn lane then re-stamps by M5 on its lane branch
      and pushes again, re-running its staging proof.

  Prod takes migrations only from `main`: L9's parity pass, then one push per release, each on Quinn's
  go. Each is a **plain** `supabase db push` (C3 Q8). Before each prod push, the controller checks that
  every migration on `main` not yet applied on prod sorts after prod's newest applied version. If one
  does not, the controller re-stamps it by M6 before the push, or asks Quinn. The controller never adds
  `--include-all` to a prod push on its own.

## 9. Test plan (test first; deno from `cloud/supabase`, no egress)

There is no local database (Q10): `migrations_test.ts` applies no SQL. The SQL is proved in three
layers, as L3's plan does (`2026-10-18-first-day-allowance-plan.md`, Q-C), and **one case table**
drives layers ii and iii so the cases are written once and run twice.

**The case table** (`migrations/testers_cases.ts`, data only; every address is
`…@example.invalid`, never a person):

| Id | Given | The view answers / the state |
|---|---|---|
| P0 | an account with the sign-up `entitlements` row (`status = 'none'`, `plan` and `current_period_end` null), no grant: **every unpaid account** | one view row: `none` / null / null; `GET /entitlement` answers `none` with a 200, as today; `isActive` false |
| P0x | an account whose `entitlements` row was deleted by hand, no grant | one view row: `none` / null / null (the coalesce) |
| P1 | Stripe `active`, no grant | the `entitlements` row, column for column (**the paying-student guard**) |
| P2 | Stripe `active`, open grant | the Stripe row (paid wins); `admin.testers` `live`, `paid` set |
| P3 | the sign-up `none` row, open grant, `encrypted_password = ''`: **every real tester** | `active` / `tester` / the grant's `ends_at`; `live` |
| P3x | as P3 with the `entitlements` row deleted by hand | as P3 |
| P4 | as P3 (the `none` row) with `encrypted_password` `NULL` | as P3 |
| P5, P6 | Stripe `canceled`, then `past_due`, open grant | as P3 |
| P7 | the `none` row, open grant, account carries a password | `none` / null / null; `waiting for sign-up` |
| P8 | the `none` row, grant past `ends_at` | `none` / null / null; `expired` |
| P9 | the `none` row, grant ended early | `none` / null / null; `ended` |
| P10 | the `none` row, grant `Friend@Example.invalid`, account `friend@example.invalid` | matches, as P3 |
| P11 | grant, no account | no view row; `waiting for sign-up` |
| P12 | a second open grant for one address (including one whose `ends_at` has passed but was never ended) | `comp_grant` refused by the unique index, with the message naming `admin.comp_extend` |
| P13 | an open grant past `ends_at`, then `comp_extend` to a future date | `expired`, then `live` and `active` / `tester` / the new `ends_at`; `comp_extend` to a past date, or on an ended grant, is refused |
| A1 | `comp_grant`, `comp_extend`, `comp_end` | each writes exactly one `comp_audit` row; `email_hash` is `sha256Hex` of the lower-cased address |
| A2 | a refused `comp_grant` (P12) | no audit row |
| A3 | the account is deleted | its grants, its `route_daily` rows and its `comp_audit` rows (by hash) are gone; another address's audit rows are untouched |
| A3s | a second account, set up as A3's, is deleted **the way production does it**: inside its own savepoint, `set local role service_role`, then `delete from public.accounts` for that account (the purge's `restDelete(rest, "accounts", …)` in `functions/account/index.ts`) | as A3, with no permission error. This is the only case that runs the deletion trigger under §3.5's reduced service-role rights, so a trigger that is not really `security definer` fails here and not on a student's `DELETE /account` on prod |
| A4 | the sweep, grants closed 31 and 29 days ago | the first deleted, the second kept; their audit rows stay, the first's `grant_id` now null |
| A6 | the sweep, audit rows written three years and a day ago and three years less a day ago | the first deleted, the second kept |
| A5 | a comped account | absent from `billing_subscribers` |
| A7 | as `service_role`: insert, update and delete on `comp_grants`, any statement on `comp_audit`, a call to `admin.comp_grant`; then a select on `comp_grants` and `select account_id, plan, status, current_period_end from public.effective_entitlements` | each write and the call refused (permission denied); both selects succeed |
| R1 | `count_route` twice, then a bad route, then as `authenticated` | `calls = 2`; refused; permission denied |
| V1 | seeded `usage_daily` rows at known token counts | `admin.tester_usage_daily.est_usd` equals the hand-computed sum |
| V2 | two testers, one with a `budget_alerts` row | `admin.tester_spend_monthly`'s total row is the sum of the two; the alert shows on its month |

**Layer i, first: the guards must see `admin` (S2a0, a non-append edit in its own commit, §8.7).** Today
`identPattern` (`migrations_test.ts`, top of file) allows only a bare name or `public.<name>`, so
`create function admin.comp_grant(` fails `FUNCTION_DEFINITION` (the parse-count check goes red) and
`create view admin.testers … as` parses as a view named `admin` (its revoke is never found). Before any
`admin.*` SQL exists, S2a0 adds tests over **synthetic SQL strings** (not the corpus) that the guards
must get right, sees them fail, then widens the prefix to `(public|admin)` and nothing wider:
- `create function admin.x() … as $$ … $$ language sql security definer;` parses as `x`; without a
  `revoke execute on function admin.x() from …anon, authenticated` it is reported, with one it passes.
- `create view admin.v as select 1;` parses as `v`, not `admin`; its `revoke all on admin.v from anon,
  authenticated` is found, and its absence is reported.
- Quoted forms (`"admin"."x"`) behave as the quoted `public` forms already do.
- `create function other.x() …` still fails loud (a raw keyword the definition regex cannot parse),
  so the widening admits exactly one schema.
- Every existing corpus test is still green with the widened pattern, before any new SQL lands.

**Layer i, then, static (`migrations/migrations_test.ts`, a block appended at the file end, plus the
in-place pin moves listed in §8.7 and nothing else):** RLS on
`comp_grants`, `comp_audit`, `route_daily`; every new writing function revoked from `anon` and
`authenticated`; every new view revoked; the function and view parse-count pins and the view `revoked`
list move by exactly the new definitions (§8.7); `admin` schema usage revoked from `public, anon,
authenticated, service_role`; the service role's write revokes on `comp_grants` and its `revoke all` on
`comp_audit` (§3.5, A7); `config.toml`'s `[api] schemas` still excludes `admin`; no column on any new
table can hold a title, body, sender or item id.
**The rule-1 tripwire (§6), with its own synthetic cases first** (S2a, before any admin function
exists): (a) the address-shaped-token scan over every `.sql` file's raw text; (b) the placement scan:
every `insert`/`update`/`delete` naming `comp_grants` or `comp_audit` (bare, `public.` or quoted) is
inside an `admin.comp_*` function body or the deletion trigger's, and an insert into `comp_grants` has
no quoted literal in its values. Synthetic strings it must get right: a `do $$ … insert into
public.comp_grants values ('friend@…') … $$` block is reported by both parts; a top-level
`insert into comp_grants select …` with no literal is reported by (b); a parameterised insert inside
`create function admin.comp_grant(…)` passes; an address in a `--` comment is reported by (a);
`update only public.comp_audit set at = now()` and `delete from "comp_grants"` at top level are
reported by (b). **Must pass**, because S2a's own SQL carries them: `revoke insert, update, delete,
truncate on public.comp_grants from service_role`; `revoke all on public.comp_audit from service_role`;
a `create table public.comp_audit (…, grant_id uuid references public.comp_grants on delete set null,
…)`; `create unique index … on public.comp_grants (email_lower) where ended_at is null`; and
`comment on table public.comp_grants is '… insert … delete …'`. (b) matches only the three DML shapes
of §6, never a DML word alone. On the corpus it is green at S2a (S2a's statements are the must-pass
shapes, and no write to either table exists yet) and stays green, unedited, when S2c1 and S2c2 add
the functions and the trigger.
Plus the exact shape of every predicate the cases depend on:
`effective_entitlements` reads `accounts` `left join entitlements` (P1) and yields
`coalesce(e.status, 'none')` for `status` (P0, P0x); its grant branch is gated on
`coalesce(e.status, 'none') not in ('active', 'trialing')` verbatim, and the view's text contains no
`in ('canceled'`, `in ('past_due'` or `e.account_id is null` test (P3, P3x, P5, P6); its select list,
split at top-level commas, is exactly four items whose output names are, in order, `account_id`,
`plan`, `status`, `current_period_end` (§3.2; the lookup's side of this contract is S3's pin); its grant match carries
`g.ended_at is null`, `now() < g.ends_at`, `g.email_lower = lower(a.email)` and
`coalesce(u.encrypted_password, '') = ''` verbatim (P3-P10); `admin.testers` uses the same predicates
(P7-P11); `count_route` is one `insert … on conflict (account_id, day, route) do update` with the route
pattern; `route_daily`'s foreign key is `on delete cascade`; `comp_audit` has no column named `note`,
`reason`, `actor` or `email`; each admin function's body contains its own audit insert; no function
body contains `update public.comp_audit` (or the unqualified form); `comp_extend`'s body refuses an
ended grant and a past `ends_at` but carries no `now() < ends_at` test on the grant it extends (P13);
the sweep's `cron.schedule('knowlu-sweep-comp'`, its 30-day grant interval and its
`interval '3 years'` audit delete (A6); the sweep's minute field (§3.1) is a plain number and equals
no other job's minute field where that field is a plain number, compared by a pin of this lane's own
over the existing `cron.schedule` reader in `migrations_test.ts` (not by that file's whole-string
`j.when !== sweep` compare in `the_mail_cron_jobs_use_the_vault_token`). The pin has synthetic cases
first: `'23 8 * * *'` against `'23 7 * * *'` is refused, `'29 3 * * *'` against `'* * * * *'` and
`'11 * * * *'` passes. Then the delete trigger, `security definer` with a fixed `search_path`, and in it both
the grant delete and the audit delete by the hash expression of §3.3 (A3); `est_usd`'s expression is `monthly_spend`'s; `billing_subscribers` is
unchanged.

**Layer ii, the TS model:** a small pure model of §3.2's precedence and `admin.testers`' state order in
`migrations/testers_model.ts` (no `Deno.test` in it), and its tests in
`migrations/testers_model_test.ts`, run over every P case of the table (P0, P0x and P3 included). It proves the
table is self-consistent; it does not prove the SQL. S3's handler tests import `testers_model.ts` and
`testers_cases.ts` for their fake lookup, never a `_test.ts` file, so `deno test functions/` does not
re-register the model's suite under the functions tree.

**Layer iii, real SQL on staging (`cloud/supabase/proofs/testers_staging.sql`):** every case of the
table as one `begin; … rollback;` script that raises on the first mismatch, so nothing it writes
survives. **It builds each account the way a real sign-up does.** It inserts only the `auth.users` row
(an `@example.invalid` address, `encrypted_password` set per case, and the columns GoTrue's own
inserts fill on staging, which S1 records), and lets the sign-up triggers run: `on_auth_user_metadata`
(before insert) and `handle_new_user` (after insert), which creates the `accounts` row and the
`entitlements` row at `status = 'none'`. The script never inserts `accounts` or `entitlements` itself
(both would hit their primary key). The Stripe cases then `update public.entitlements set status = …,
plan = …, current_period_end = …` for that account; P0x and P3x `delete from public.entitlements` for
it; P0, P3, P4 and P7-P10 leave the sign-up row as it is. The script's first statement checks the live
trigger is the C1b body (`pg_proc.prosrc` for `handle_new_user` contains no `raise`), and raises a
named error if not: the C1 body raised without `age_attested`, `tos_version` and `privacy_version` in
the metadata, and would make every case fail for a reason that is not the view. It then inserts its
own `usage_daily` and `budget_alerts` rows, calls the admin functions, and backdates `comp_grants` and
`comp_audit` rows as the owner where A4 and A6 need it. A3 deletes the `auth.users` row, which
cascades to `accounts` and fires the deletion trigger. A3 runs as the owner, who can do anything, so
it cannot catch a privilege mistake. **A3s repeats the deletion as production runs it** (revision 5):
in its own savepoint, under `set local role service_role`, `delete from public.accounts` for a second
account, as the account function's purge does. The trigger then runs with the service role's reduced
rights on `comp_grants` and `comp_audit` (§3.5). A function variant that the static `security definer`
pin accepts but that does not carry the owner's rights fails here, on staging, before the merge. A static test pins that the file opens with `begin;`, closes with
`rollback;`, contains no `commit`, and names only `@example.invalid` addresses. The folder is not under
`migrations/` and is never deployed. A7 runs its statements under `set local role service_role`, each
in its own savepoint so the expected refusal does not end the script. **This is the gate:** the
controller runs it on staging after the lane's migration is pushed there from the lane branch and
**before S6's merge** (S7a, §8.10), so `main` never carries a `lookupFrom` on an unproved view; it runs
again on prod after the prod migration and before any prod function deploy from a `main` that carries
S3. A failure at P0, P1 or P3 stops the lane before anything reaches `main`.

**Functions (deno unit, dependencies injected as today):**
- `lookupFrom` reads `effective_entitlements`; `GET /entitlement` for a comped account answers
  `active`, `tester`, the grant's end; for an ended grant, `none` with a 200. The fake lookups cover
  the `none`-row cases (P0, P3) first, not only the missing-row ones.
- The column contract (§3.2), a static pin in `_shared/entitlement_test.ts`: `lookupFrom`'s query reads
  `effective_entitlements` with the filter `account_id=eq.` and a `select=` naming only columns among
  `plan`, `status`, `current_period_end`; and each of those, plus `account_id`, appears as an output
  name of the view in the testers migration (found by `^\d{14}_testers\.sql$`, read as text). Either
  side renamed alone goes red before staging.
- `requireActiveEntitlementWith` passes a comped account and 402s an ended one (fakes from the case
  table).
- The route parse (§4), a table of paths: `/functions/v1/judge-task` → `judge-task`; `/judge-task`
  (no prefix) → `judge-task`; `/functions/v1/<fn>/<sub>` and `/<fn>/<sub>` → `<fn>`; a trailing slash
  and a query string change nothing; `/`, `/functions/v1` and `/functions/v1/` → no name; a segment
  outside `^[a-z][a-z0-9-]{0,39}$` → no name. With no name, `count` is not called and the gate's
  answer is unchanged.
- The count: called once, after an entitled lookup, with the parsed route name, and never after a 401
  or 402. A rejecting `count`
  neither throws nor refuses; a `count` that never resolves does not delay the gate's return; its
  promise is handed to `defer`. Deps with neither `count` nor `defer` (every existing test's shape)
  behave exactly as today.
- The secret (§4): `depsFrom` with an env getter that returns nothing, `off`, `ON` or `true` supplies
  no `count`; with `on` it supplies one. Without the secret, a gated call through `depsFrom`'s deps
  makes no `count_route` request (a fake `rest` that records calls sees none).
- `billing-checkout` answers 409 for a caller the lookup shows as `tester` and is unchanged otherwise
  (Q3). Two named cases cover that "otherwise": a caller with an open grant past `ends_at` (P8's shape)
  and a caller with an open grant whose account carries a password (P7's shape). Both proceed to
  Checkout as today. A static pin in `billing-checkout/handler_test.ts` that `index.ts` wires that lookup to
  `effective_entitlements` selecting `plan`, the precedent of `account/handler_test.ts`'s `sources`
  guard, which reads `index.ts` as text.
- The export: `account/handler.ts` receives `exportAll` injected, so `handler_test.ts` cannot reach
  `index.ts`'s assembly. A static pin there, in the same pattern, that `exportAll` reads `comp_grants`
  by `email_lower`, and `route_daily` and `usage_daily` (Q12) by `account_id` through the paging
  `all(…)` with an order; and that `purge`'s table list names `route_daily` and its comment names
  `comp_grants` and `comp_audit` (§3.3). `billing-portal` is unchanged (409, never a Stripe call).

**App (S5t, then S5):** a static pin in `app/tests/static_assets.rs`, written first and seen red, for
the two sentences, that the subscribe step asks `checkEntitled()` on entry, and that
`afterUpgradeSignIn`'s body un-hides `up-subscribe` only inside the not-entitled branch of its
`checkEntitled()` answer (no `EL("up-subscribe").hidden = false` before that call); a fixture
screenshot of Settings with `plan: tester`.

**Live, staging (the controller), after the merge, with `ROUTE_COUNTS=on` set on staging and the gated functions deployed after it (§4):** an existing Stripe-test-mode subscriber
still answers `active` from `GET /entitlement` and runs a judge call (P1 end to end); an existing
unpaid staging account still answers `none` (P0 end to end); grant a scratch account before sign-up;
onboard it (so it carries the sign-up `none` row, P3 end to end) and see no price; judge and ingest run; the views show its day and
a `route_daily` row per route it called; end the grant and see a 402 at once and the app's
not-entitled path at its next check. Prod grants, and `ROUTE_COUNTS=on` on prod, follow only on
Quinn's go and after §8.5.

## 10. Tasks

S2a0-S2f share two files (`migrations/<stamp>_testers.sql`, `migrations/migrations_test.ts`), so they
run **in order, one at a time**, each appending; each writes its layer-i pins (and its case-table rows)
first, sees them fail, then writes the SQL. **Size bound, for the whole dispatch, not the SQL alone:**
each task's SQL stays under about 80 lines and its tests (pins plus case rows) under about 80 more. A
task that would cross either line is split before dispatch, never during it: a long append across two
shared files is the documented stall pattern, and one large commit hides a wrong predicate among
routine pins. That is why S2c and S2e are each two tasks. The case table grows with the tasks: S2b
writes the P rows, S2c1 and S2c2 the A rows, S2d R1, S2e1 the state column of P7-P13, S2e2 the V rows.
The edits that are not appends are exactly those of §8.7: S2a0's `identPattern` widening (alone, so it
is one small commit L5 and L1 can be pointed at), and the in-place moves of the function and view
parse-count pins and the view `revoked` list by the task that adds each definition (S2b, S2c1, S2c2,
S2d, S2e1, S2e2).

| Task | What | Files | Agent | Why this agent |
|---|---|---|---|---|
| S1 | Staging spike: an OTP user, a Google user and a password `/signup` user. Record the literal `encrypted_password` each carries (`NULL`, `''` or a hash); it picks email vs account-id binding. Also record that each has an `entitlements` row at `none` (§2), and which `auth.users` columns GoTrue fills on insert, for S2f's script | none (ledger row) | main session | live staging reads are the controller's |
| S2a0 | The guard widening alone, test first: the five synthetic `admin.x` function and view cases of §9 layer i seen red, then `identPattern`'s optional prefix widened to `public` or `admin` and nothing wider (§8.7), the whole corpus suite still green. No SQL file; one commit | `migrations/migrations_test.ts` | cloud-engineer | a shared guard every migration lane relies on; a loose widening hides a hole in every later migration |
| S2a | First, the stamp: take L3's recorded stamp from the controller and fix this lane's by the shared rule's M3 (the next free stamp after it, after `main` and after staging), recorded in the ledger (§8.4). Tables, RLS and guards: `comp_grants` (with the partial unique index and the note length check), `comp_audit` (no free-text column), the `admin` schema, the client and service-role revokes (§3.5); the rule-1 tripwire with its synthetic cases, the must-pass ones included (§6, §9 layer i); the other static guards that need no function yet (RLS, revokes, schemas, column names) | `migrations/<stamp>_testers.sql`, `migrations/migrations_test.ts` | cloud-engineer | grants and their audit are money; `cloud/` is its lane |
| S2b | `effective_entitlements`: the pass-through `left join`, the `coalesce` to `none`, §3.2's precedence with the grant branch as the complement of `active`/`trialing`, the password predicate, the four output columns, and its revoke; the view pins moved (§8.7); the case table's P rows (P0, P0x, P1-P3, P3x, P4-P13, the `none`-row cases first) | the same, `migrations/testers_cases.ts` | cloud-engineer | this view gates every paying student |
| S2m | The TS model of §3.2 and `admin.testers`' state order, and its tests over every P row (layer ii); split from S2b by the size bound, and a file of its own | `migrations/testers_model.ts`, `migrations/testers_model_test.ts` | cloud-engineer | the fake lookup S3's gate tests stand on |
| **S2R** | **Checkpoint:** review S2a0-S2m (the widening, the tripwire, the view, its pins, the case table, the model) before anything reads it | `docs/reports/` | reviewer | a wrong join or a wrong grant branch here 402s every student or every tester; caught before S2c1-S4 build on it |
| S2c1 | The grant functions: `admin.comp_grant`, `admin.comp_extend` (accepting an expired open grant, refusing an ended one or a past `ends_at`), `admin.comp_end`, each with its refusals and its one audit insert; their revokes; pins (each body's audit insert, no `update` of `comp_audit`, P12's message, P13's refusals and no `now() < ends_at` in `comp_extend`), the function pin moved by three (§8.7); the A1, A2, A5 and A7 rows | the same | cloud-engineer | grant writes are money |
| S2c2 | Deletion and retention: the `after delete on public.accounts` trigger function (`security definer`, fixed `search_path`; grants and audit by the §3.3 hash) and its `create trigger`; `admin.comp_sweep()` (grants 30 days after closing, audit at three years) and its `cron.schedule('knowlu-sweep-comp', …)` on an unused minute field (§3.1); pins (the trigger's two deletes and hash expression, the two intervals, the minute-field pin with its synthetic cases), the function pin moved by two; the A3, A3s, A4 and A6 rows | the same | cloud-engineer | deletion rights; a wrong audit-delete predicate is reviewed on its own, not among the grant pins |
| S2d | `route_daily` (cascade) and `count_route`; their pins, the function pin moved; R1 | the same | cloud-engineer | a new per-account record |
| S2e1 | `admin.testers` (the four states in §5's order, `paid`, account id, last active day) and its revoke; pins (the same predicates as the view), the view pins moved by one; the state column of P2 and P7-P13 | the same | cloud-engineer | the list Quinn grants and ends from |
| S2e2 | `admin.tester_usage_daily` (per kind, tier, phase and route) and `admin.tester_spend_monthly` (total row, `budget_alerts`), their revokes; pins (`est_usd` is `monthly_spend`'s expression; counts and dollars only), the view pins moved by two; the V1 and V2 rows | the same | cloud-engineer | dollars Quinn reads |
| S2f | The staging proof script (layer iii) from the case table, built through the sign-up trigger (§9), and its static pin. By the size bound, two dispatches appending to the one script: the trigger precheck and the P cases first, then the A, R and V cases | `cloud/supabase/proofs/testers_staging.sql`, `migrations/migrations_test.ts` | cloud-engineer | it is the prod gate |
| S3 | The gate and the entitlement read: `lookupFrom` on the view; the route-name parse (§4) and its path table; optional `count` and `defer` deps, `depsFrom` wiring `EdgeRuntime.waitUntil` and supplying `count` only under `ROUTE_COUNTS=on` (§4); their unit tests, the fake lookup built from `testers_model.ts`; the column-contract pin against the view (§9) | `functions/_shared/entitlement.ts`, `_shared/entitlement_test.ts`, `entitlement/handler_test.ts` | cloud-engineer | every gated function goes through this file |
| S4 | `billing-checkout` refusal for a `tester` caller (Q3), wired in its `index.ts`; the export gains `comp_grants`, `route_daily`, `usage_daily`; the purge list gains `route_daily` and a comment naming `comp_grants` and `comp_audit` (§3.3); the handler tests and the two static pins of §9 | `functions/billing-checkout/handler.ts`, `billing-checkout/index.ts`, `billing-checkout/handler_test.ts`, `functions/account/index.ts`, `account/handler_test.ts` | cloud-engineer | billing path and an access right |
| S5t | The failing static pin of §9 "App" (the two sentences, the subscribe step's `checkEntitled()` on entry, `afterUpgradeSignIn`'s order), seen red; it runs in S5's queue slot, just before S5, so it rebases onto the same `console.js` | `app/tests/static_assets.rs` | test-writer | `app/tests` is outside console-ui's described lane (`app/static`, `site/`, `scripts/`); test-writer writes the failing app test first |
| S5 | The wizard step, the Settings row and the `afterUpgradeSignIn` fix (§3.4), Quinn's copy, until S5t's pin is green; fixture screenshot | `app/static/console.js`, `app/static/index.html` | console-ui | `app/static` is its lane; queued per §8.1: after L6's merge, ahead of L4's T8 (Quinn's ruling of 2026-10-19) |
| S7a | Staging, **before the merge** (§8.10), after the lane's last rebase and M5's stamp re-check: push the lane's migration to staging from the lane branch (only once M7 clears L3: L3's migration on staging and on `main`, or L3, having neither merged nor pushed, recorded as slipped, never L3's merge alone or its pre-merge push alone; and only after `supabase db push --dry-run` lists this lane's version alone, else stop, and a refusal naming a remote version with no local file waits for that lane's merge or withdrawal, never `migration repair` to get past it; the push is recorded in the session ledger; the name is frozen from here, M6, and until S6's merge or this lane's withdrawal it holds L5's push and L1's C1, M7), run the layer-iii script, and hand the result to S6. A P0, P1 or P3 failure stops the lane here, with nothing on `main`, and the controller withdraws it by M7 on its own (the script tests the SQL alone, so a failure is a defect in the SQL), recording it in the session ledger and telling Quinn; Quinn declining the merge at S6 is Quinn's word for the same withdrawal; any other stop goes to Quinn first (§8.10, Quinn's ruling of 2026-10-19) | none (ledger row, the script's output) | main session | staging pushes and live reads are the controller's |
| S6 | Review: the whole diff (S2a0-S5) and S7a's result before the PR. **The PR merge then waits for Quinn's go**, not CI green plus a reviewer PASS alone: the lane changes the gate every paying student passes and adds the `billing-checkout` refusal, which is "anything touching money" (handoff §1, ruling 7) | `docs/reports/` | reviewer | money and entitlement; S5 came from a cheaper agent |
| S7 | After the merge. Staging: set `ROUTE_COUNTS=on`, then deploy the functions (a warm isolate keeps the old value, §4), run §9's live proof. Prod, with L9 and on Quinn's go for prod pushes: the prod migration and the layer-iii script **before** any prod function deploy from a `main` that carries S3 (L9's parity pass included); the migration and functions need no wait on Q7, because the secret is off. Then, on Quinn's go and only once §8.5's sentences are on the live page: `ROUTE_COUNTS=on` on prod and a redeploy of the gated functions (sentence 1) and the first grants from Quinn's block item 7 (sentence 2) | none | main session | staging and prod pushes, secrets, tokens and prod writes are the controller's |
| S8 | Q7's two sentences to L4 **before** L4's `PRIVACY_VERSION` bump (§8.5); the friends' install note says "sign in with the address you gave Quinn" (L10) | none here | docs-keeper via L4/L10 | those files are other lanes' |

Order: **S1 before S2a** (S2a's table already commits to email binding; if S1 fails, §8.9 applies and
S2a does not start; S2a0 touches no table and may run before S1 returns); **S2a after L3's T0** has
recorded its stamp (§8.4); S2a0 → S2a → S2b → S2m → S2R → S2c1 → S2c2 → S2d → S2e1 → S2e2 → S2f; S3
after S2R and S2d (it reads the view and the RPC); **S4 after S2R** (its checkout lookup reads the
view's columns, which S2R may reshape); S5t then S5, independent of S2-S4, in S5's queue slot; **S7a
after S2f, and only once L3 is cleared: its migration on staging and on `main`, or L3, having neither
merged nor pushed, recorded as slipped** (M7; neither L3's merge alone nor its pre-merge push alone is
enough, and S7a's dry run lists this lane's version alone; from S7a until S6's merge or this lane's
withdrawal (§8.10), L5's push and L1's C1 wait); S6 after S7a, S3, S4 and S5; the merge on Quinn's go (S6); S7 after the merge; S8 as soon
as Quinn rules Q5, Q7 and Q8, and in any case before L4's bump.

## 11. Fidelity ledger

| Asked for (2026-10-18 brief) | Where | Task | Test (§9) |
|---|---|---|---|
| Comp for a named list, free, no purchase, no Stripe charge | §1, §3.1, §3.4, Q3 | S2b, S4, S5t, S5 | P3, P3x, P4-P6, A5 (P3, the sign-up `none` row, is the tester's real row); the grant-branch complement pin; the `billing-checkout` 409 and its wiring pin; S5t's static pin, `afterUpgradeSignIn`'s order included |
| Paying students unaffected by the new lookup | §3.1 (fail safe), §3.2 | S2b, S2R, S3, S7a, S7 | P0, P0x, P1, P2 (all three layers, layer iii before the merge, built through the sign-up trigger); the `coalesce` pin; the four-column pin on both sides (S2b, S3); the live P0 and P1 checks; the prod migration before any prod function deploy |
| How a comp row is represented, with alternatives | §3.1 | S2a, S2b | layer-i shape pins; P12 |
| How it expires or ends | §3.3, Q1 | S2b, S2c1, S3 | P8, P9, P12, P13; the gate's 402 for an ended grant; live end-of-grant check |
| Only a proved address is matched | §3.5 (sign-up check) | S1, S2b | P3, P4, P7; the `coalesce` pin |
| How it shows; no paid look, no payment prompt | §3.4 | S5t, S5 | S5t's static pin; the Settings fixture screenshot |
| Who writes it; RLS; audit trail | §3.5, Q8 | S2a, S2c1 | layer-i RLS and revoke pins, the service-role revokes included; A1, A2, A7; the `comp_audit` column pin |
| No tester's address in the repo (rule 1) | §6, §8.7 | S2a | the address-token and placement scans, their synthetic cases and the must-pass ones (S2a's revoke, the FK, the index, the comment); the proof script's `@example.invalid` pin; the ban named in the L5 and L1 hand-off |
| Export, deletion, retention | §3.3, §3.5, §4, Q8, Q11, Q12 | S2c2, S2d, S4 | A3, A3s (the deletion as the service role runs it), A4, A6; the trigger's audit-delete pin; the sweep's three-year and unused-minute pins; the export and purge-list static pins |
| Cost estimate: what is recorded, what is missing | §2, §4, Q4, Q5 | S2d, S2e2, S3 | R1, V1; the route-parse path table; the count tests (never awaited, never refusing) |
| Admin view's form | §5 | S2e1, S2e2 | P7-P11 and P13 states, V1, V2 |
| Test plan (deno, migration) | §9, Q10 | S2a0-S2f, S3, S4 | the three layers; the synthetic `admin.x` guard cases before the widening (S2a0); each task within the §10 size bound |
| Money changes merge only on Quinn's go | §10 S6, handoff §1 ruling 7 | S6 | none (a gate, not a test) |
| `entitle.rs` contract unchanged | §1, §3.2 | none (no engine edit) | P3's output uses today's `status` vocabulary |
| Testers and students told | Q7, §8.5, §4, §8.11 | S3, S8, S7 | Quinn reads L4's page before the bump; the `ROUTE_COUNTS` tests (off unless exactly `on`); the controller reads the live page before setting the prod secret or writing a grant, and redeploys and checks `route_daily` after any switch |
| Money choices for Quinn, recommendation first | §7 | none | none |
| Stamp strictly after L3's recorded stamp (L3's Q-F), under the one rule shared by the email, L3, L8, L5 and L1 migrations (C3 Q8, ruled 2026-10-19) | §8.4 and the migration-stamp rule (M1-M8) after §8 | S2a, S7a | the stamp in the ledger before the file (M3); the file found by pattern, never by stamp (M4); M5's re-check before S7a; S7a only once L3 is cleared, its migration on staging and on `main` or L3, having neither merged nor pushed, recorded as slipped (M7: a slip never clears a lane that has merged or pushed, a lane that pushed before merging holds every lane behind it until it merges or the controller withdraws it, and a migration merges only with its push); S7a's `db push --dry-run` listing this lane's version alone, a refusal naming a remote version with no local file waiting for that lane's merge or withdrawal, never `migration repair` to get past it (M7); no rename after S7a except the controller's M6 repair; if the lane stops after S7a, M7's withdrawal (§8.10: the controller's own call on a P0, P1 or P3 failure, Quinn told; Quinn's no is Quinn's word; any other stop asked of Quinn first; the drop in one transaction and a catalogue check before the repair) |

**Deviations recorded here (revision 1):** the admin functions take no `actor` and the audit holds no
note (§3.5, which narrows the brief's "audit trail" to what may be kept three years); `route_daily` does
not count `mail-inbound` or `mail-canary` (§4), whose volume reads from `mail_seen`.
**Revision 2:** the audit trail is narrowed again, to the account's life and three years at most
(Q8's recommendation changed; the earlier "kept through a deletion" is now the alternative). The lane
makes one non-append edit to `migrations_test.ts`, the `identPattern` widening (§8.7).
**Revision 3:** that count was wrong. The non-append edits are the widening (its own task, S2a0) and the
in-place moves of the function and view parse-count pins and the view `revoked` list (§8.7). Prod's
`route_daily` writes wait on the `ROUTE_COUNTS` secret and the live privacy page, not on the merge
(§4, §8.5). The staging proof moves before the merge (S7a). None of these narrows the brief.
**Revision 4:** no deviation from the brief. The `entitlements` writer list in §1 and §2 is corrected
(it was wrong, not narrowed). S2b's model half becomes S2m, and S2f is written in two dispatches,
both by the same size bound that splits S2c and S2e.
