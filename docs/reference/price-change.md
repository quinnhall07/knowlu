# Changing a price

The step list for raising or lowering a price. It follows `docs/specs/2026-11-19-v041-pricing-design.md`
D16, and D13 to D15 behind it. A price is changed in test mode on staging first, then live, and **every
live step is taken on Quinn's word**. A raise and a cut follow the same steps; only the amendment's rule
differs.

It changes an **amount** only. Adding or dropping a plan key (`four_weekly`, `yearly`) needs an app
release and its own lane, because the app's buttons are per key, and a build whose offer lacks one of its
keys can sell nothing ("Update Knowlu"). **A plan change needs a release.**

## Where a price lives

Every amount, every Stripe price variable name and the terms version that states the prices are written
in one file, `cloud/supabase/functions/_shared/price_catalog.ts`, and nowhere else in the billing code:

- `SELLING`: what is sold today, one entry per plan key, `{ key, cents }`, Yearly first.
- `KEPT`: a price no longer sold that a subscriber may still pay. Each entry is `{ key, cents, rule }`
  with `rule: "stay"` (they keep it until they cancel) or `rule: "move"` with `noticeDays`
  (`billing-jobs` emails them `noticeDays` before a renewal, then moves them to the key's selling price
  for that renewal). `noticeDays` runs from `MOVE_NOTICE_MIN_DAYS` (7) to `MOVE_NOTICE_MAX_DAYS` (24).
- `TERMS_VERSION`: the version of the terms page that states the selling prices.
- The variable that holds an entry's Stripe price id is derived, never written:
  `STRIPE_PRICE_<KEY>_<CENTS>`, the key in capitals. Today they are `STRIPE_PRICE_FOUR_WEEKLY_1299` and
  `STRIPE_PRICE_YEARLY_8437`. A variable names one Stripe price forever, so a price change adds a
  variable and never re-points one.

## The steps

### 0. Before anything

- `.github/workflows/site.yml` exists on `main`.
- `site/privacy.html` on `main` equals the latest Release's. Check it as `site.yml` does:
  `git diff <tag> main -- site/privacy.html` prints nothing, where `<tag>` is
  `gh release view --json tagName --jq .tagName`. A privacy bump merged before its release makes
  `site.yml` refuse, so it blocks every price change until that release ships.
- The amendment states the new amount and, for the old price, `stay` or `move` with N (the `noticeDays`
  above), and Quinn signs it. A new yearly amount is 13 times a whole-cent amount (`catalogProblems`
  refuses any other).
- Under `move`, the trial rule is ruled and the code matches the ruling. The question, its options and
  the ruling (answered 2026-11-21) are in `docs/specs/2026-11-19-v041-pricing-design.md` (D14, section 8
  item 15), and the code that implements it is `moveIfDue` in
  `cloud/supabase/functions/billing-jobs/handler.ts` (today: a `trialing` subscription is not moved to a
  dearer price until it is active; a cut moves it).

### 1. Stripe, test mode

Create a Price for the new amount, with the key's cadence and tax behaviour **exclusive**.

- **If the amount is new for this key**, Quinn creates the Price, and the main session sets
  `STRIPE_PRICE_<KEY>_<CENTS>` on staging (`supabase secrets set`, from a value Quinn produces, never
  printed).
- **If the key has been sold at this amount before** (for example 1299 to 1499 under `stay`, then back to
  1299), reuse that Price: Quinn unarchives it, its variable keeps (or is set again to) the same id, and
  step 2 moves its entry from `KEPT` back to `SELLING`. Never create a second Price at the same key and
  cents: the derived name would name two prices.

### 2. One Contract PR

- The catalog, `price_catalog.ts`: the new `SELLING` entry; the old one moved to `KEPT` with its rule;
  `TERMS_VERSION`.
- `site/terms.html`: the figures and the version, and anything the amendment adds for kept subscribers.
- `TOS_VERSION` in `app/src/account.rs`, by `contract-engineer`.
- The one test that names the selling amounts, `the_catalog_sells_two_plans_yearly_first` in
  `cloud/supabase/functions/_shared/price_catalog_test.ts` (the catalog's first pin), so the change is
  seen in a test diff too. The handlers' tests build their own catalogs and do not change.

CI ties the catalog, the page and `TOS_VERSION`, and finds any `$` figure, decimal money figure or
`STRIPE_PRICE_` name left in the billing code outside the catalog. It does not scan bare cents such as
`1299`; a hard-coded amount shows in behaviour instead, because the handler tests run on invented
catalogs (1399).

CI does not read documents, so **the same PR checks them by hand**. Each is changed, or named in the PR as
left alone with a reason:

- `VISION.md`'s One edition line;
- `CLAUDE.md`'s Direction line;
- `HANDOFF.md`'s billing lines;
- `docs/reference/app.md`'s price line;
- the Billing section of `cloud/supabase/README.md`;
- `site/index.html`'s pricing;
- the `$9.99` yardstick comments in `cloud/supabase/functions/_shared/judge_caps.ts` (lines 45 and 97).

### 3. Staging

Deploy the three billing functions from the PR head:

```powershell
supabase functions deploy billing-checkout --use-api --project-ref <the STAGING ref>
supabase functions deploy stripe-webhook   --use-api --project-ref <the STAGING ref>
supabase functions deploy billing-jobs     --use-api --project-ref <the STAGING ref>
```

`billing-jobs` imports from `stripe-webhook/handler.ts`, so a change to that file means deploying
`billing-jobs` too. **Deploy all three together, always.**

With an unchanged installed build:

- the offer (`GET /billing-checkout`), both buttons and Stripe's test page show the new figure;
- an offer from before the change is refused with the 409 "Knowlu's prices or terms have changed. Look
  them over again, then subscribe.";
- for `move`, rehearse one notice and one change on a test subscription.

### 4. Live mode

On Quinn's word, with the same two cases as step 1, in live mode:

- **If the amount is new for this key**, Quinn creates the live Price (the key's cadence, tax behaviour
  exclusive), and the main session sets the production variable `STRIPE_PRICE_<KEY>_<CENTS>`.
- **If the key has been sold at this amount before in live mode**, Quinn unarchives that live Price, and
  the production variable keeps (or is set again to) the same id. **Never create a second live Price at
  the same key and cents**: subscribers still on the original Price would match no catalog entry, and
  their next webhook event would record no plan.

### 5. Merge, then go live back to back

On Quinn's word, in this order, with no pause between:

1. **Merge the PR.**
2. **Optionally read the page first:** run the `site` workflow (`.github/workflows/site.yml`, started by
   hand from `main` in the Actions tab, `workflow_dispatch`) with `deploy_branch` = `preview`, and read
   the preview's pages, installer and `latest.json`.
3. **Publish the page:** run the same workflow with `deploy_branch` = `main`.
4. **Deploy the three billing functions** from the merge SHA, with the commands in step 3 and
   `<the PROD ref>` in place of the staging one, all three together.

Page first. In the minutes between, a checkout records the old version with the old price, a matched
pair, and never a version that is not yet published.

`site.yml` stops, as a failed run, if: it is not on `main`; a `v*` tag sorts after the latest Release's
(a release is in flight or has failed); a `release.yml` run is not completed; or `site/privacy.html`
differs from the Release tag's. It restores the Release's versioned installer, `Knowlu-setup.exe` and
`latest.json` into `site/releases/` before it deploys. It needs no `main` checkout of yours.

### 6. Check

- `knowlu.com/terms.html`'s version equals the catalog's `TERMS_VERSION`.
- The live offer shows the new figures: read it in the app's subscribe step on a signed-in account
  (`GET /billing-checkout` needs a signed-in session).
- The live endpoint's deliveries since the deploy are all 2xx: in the Stripe Dashboard, live mode,
  Webhooks, the `stripe-webhook` endpoint.
- One live checkout page opens (nothing is paid): from that same subscribe step.
- knowlu.com still serves the installer and `latest.json`: `latest.json` is at
  `https://knowlu.com/releases/latest.json` (the updater's endpoint, `app/tauri.conf.json`), and the
  installer files are in the same `releases/` folder, `Knowlu-setup.exe` and the versioned
  `*-setup.exe` (the names `site.yml` restores into `site/releases/`).

If the installer or `latest.json` is gone, recover by the rule below: `site.yml` again (the hand
restore only if `site.yml` cannot run), **never by re-running a tag's `publish` job**.

### 7. Afterwards

Quinn archives the old live Price. A `KEPT` entry is removed by a later PR, only
when Stripe's list of subscriptions on that price, **every status included**, shows none that could still
bill. The job's view leaves out `past_due` and `unpaid`
(`cloud/supabase/migrations/20260910000200_billing_jobs.sql`), and `move` skips one set to cancel at period
end, and either can recover or be un-cancelled later. (That the list filters by price and takes every
status is `unverified (Stripe)`: check it on the Dashboard before relying on it.)

When that holds, on Quinn's word: merge that PR, deploy its three billing functions as steps 3 and 5
deploy them (staging, then production, all three together), and unset the entry's variable on both
projects. A removal never deployed leaves the entry live; a variable unset before the deploy leaves a
`KEPT` entry that matches no id.

### 8. No app release

`TOS_VERSION` reaches installed apps with the next ordinary release.

## Recovery: knowlu.com has lost its installer or `latest.json`

`release.yml`'s deploy replaces the whole site, and `site/releases/` lives only in that run's artifact
(in git it holds `.gitkeep`). A hand `wrangler pages deploy site` takes the installer and `latest.json` off
knowlu.com.

- Run `site.yml` again, on Quinn's word; or
- **only if `site.yml` cannot run**, restore by hand, on Quinn's word. First compare `main`'s
  `site/privacy.html` with the latest Release's tag's (`git diff <tag> main -- site/privacy.html`) and
  **stop if they differ**. Then download the latest Release's assets into a `main` checkout's
  `site/releases/` (the versioned installer, copied to `Knowlu-setup.exe`, and `latest.json`), and run
  the deploy as `site.yml` and `release.yml` run it: `wrangler pages deploy site --project-name knowlu
  --branch main --commit-dirty=true`, which needs the Cloudflare token. The hand restore skips
  `site.yml`'s newer-tag and running-release guards, so check both by hand first.

**Never re-run a tag's `publish` job.** It deploys that tag's `site/`, whose terms page is the old one
after a price change, and its artifact is kept 7 days only (`release.yml`, `retention-days`).
